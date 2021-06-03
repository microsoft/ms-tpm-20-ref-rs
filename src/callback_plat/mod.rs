use std::convert::TryInto;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::error::*;
use crate::tpmlib_state;
use crate::InitKind;
use crate::PlatformCallbacks;

pub(crate) mod api;

static PLATFORM: Lazy<Mutex<Option<MsTpm20RefPlatformImpl>>> = Lazy::new(|| Mutex::new(None));

#[link(name = "run_command")]
extern "C" {
    fn RunCommand(
        requestSize: u32,
        request: *mut u8,
        responseSize: *mut u32,
        response: *mut *mut u8,
    );
}

/// Serde de/serializable representation of the ms-tpm-20-ref library's runtime
/// state (both core C library runtime, and Rust platform runtime)
#[derive(Clone, Serialize, Deserialize)]
pub struct MsTpm20RefRuntimeState {
    tpmlib_state: tpmlib_state::MsTpm20RefLibraryState,
    platform_state: MsTpm20PlatformState,
}

/// A handle which encapsulates the logical ownership of the global platform
/// singleton.
///
/// Only a single instance of `MsTpm20Platform` can be live at any given time.
/// If [`MsTpm20Platform::initialize`] is called while a instance of
/// `MsTpm20Platform` is still live, it will return an
/// [`Error::AlreadyInitialized`].
///
/// When `MsTpm20Platform` is dropped, it will uninitialize the platform,
/// allowing a subsequent call to [`MsTpm20Platform::initialize`] to succeed.
#[non_exhaustive]
#[derive(Debug)]
pub struct MsTpm20RefPlatform {}

impl MsTpm20RefPlatform {
    /// Initialize the TPM library with the given implementation-specific
    /// callbacks.
    ///
    /// Corresponds to both `VTpmColdInitWithPersistentState` and `VTpmWarmInit`
    ///
    /// NOTE: This method will NOT send the TPM startup or selftest commands.
    pub fn initialize(
        callbacks: Box<dyn PlatformCallbacks + Send>,
        init_kind: InitKind<'_>,
    ) -> Result<MsTpm20RefPlatform, Error> {
        log::trace!("Initializing TPM platform...");

        let mut maybe_platform = PLATFORM.lock().unwrap();

        match &mut *maybe_platform {
            Some(_platform) => return Err(Error::AlreadyInitialized),
            None => {
                let mut platform = MsTpm20RefPlatformImpl::new(callbacks);
                match init_kind {
                    InitKind::ColdInit => platform.nv_enable()?,
                    InitKind::ColdInitWithPersistentState { nvmem_blob }
                    | InitKind::WarmInit { nvmem_blob, .. } => {
                        platform.nv_enable_from_blob(nvmem_blob)?
                    }
                };
                *maybe_platform = Some(platform);
            }
        }

        log::trace!("TPM platform initialized");

        // now that the platform layer has been set up, we can call into the TPM lib
        // itself to prep the TPM.
        log::trace!("Initializing TPM library...");

        unsafe {
            maybe_platform.as_mut().unwrap().signal_power_on()?;

            // Make sure to drop the mutex guard, as the TPM library will call back into the
            // platform, and Rust's std mutex is not reentrant!
            drop(maybe_platform);

            let ret = crate::bindgen::TPM_Manufacture(true as i32);
            if ret != 0 {
                return Err(Error::Ffi {
                    function: "TPM_Manufacture",
                    error: ret,
                });
            }

            crate::bindgen::_TPM_Init();
            log::trace!("_TPM_Init Completed");
        }

        log::info!("TPM library initialized");

        // apply any warm init state, if available
        if let InitKind::WarmInit { runtime_state, .. } = init_kind {
            PLATFORM
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .restore_runtime_state(runtime_state.platform_state);

            tpmlib_state::restore_runtime_state(runtime_state.tpmlib_state);
        }

        Ok(MsTpm20RefPlatform {})
    }

    fn shutdown(&mut self) {
        let mut platform = PLATFORM.lock().unwrap();
        platform.as_mut().unwrap().signal_power_off();
        *platform = None;
    }

    /// Execute a command on the TPM without checking / truncating request /
    /// response buffers to the size specified by the contained command.
    ///
    /// # Safety
    ///
    /// Callers must ensure that the request buffer is appropriately sized for
    /// the contained command.
    ///
    /// # Panics
    ///
    /// If the TPM library returns a response using a internally allocated
    /// buffer larger than the provided user-allocated response buffer, this
    /// function will panic instead of truncating the output.
    pub unsafe fn execute_command_unchecked(
        &mut self,
        request: &mut [u8],
        response: &mut [u8],
    ) -> usize {
        let request_size = request.len() as u32;
        let request_ptr = request.as_mut_ptr();
        let mut response_size = response.len() as u32;
        let mut response_ptr = response.as_mut_ptr();

        let prev_response_ptr = response_ptr;
        unsafe {
            RunCommand(
                request_size,
                request_ptr,
                &mut response_size,
                &mut response_ptr,
            );
        }

        if prev_response_ptr != response_ptr {
            let tmp_buf =
                unsafe { core::slice::from_raw_parts_mut(response_ptr, response_size as usize) };
            response[..response_size as usize].copy_from_slice(tmp_buf);
        }

        response_size as usize
    }

    /// Execute a command on the vTPM.
    ///
    /// Corresponds to `VTpmExecuteCommand`
    pub fn execute_command(
        &mut self,
        request: &mut [u8],
        response: &mut [u8],
    ) -> Result<usize, Error> {
        let request_header_size = request
            .get(2..6)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
            .ok_or(Error::InvalidRequestSize)?;

        if request_header_size as usize > request.len() {
            return Err(Error::InvalidRequestSize);
        }

        let request_size = (request.len() as u32).min(request_header_size);
        let request_ptr = request.as_mut_ptr();
        let mut response_size = response.len() as u32;
        let mut response_ptr = response.as_mut_ptr();

        let prev_response_ptr = response_ptr;
        unsafe {
            RunCommand(
                request_size,
                request_ptr,
                &mut response_size,
                &mut response_ptr,
            );
        }

        if prev_response_ptr != response_ptr {
            let tmp_buf =
                unsafe { core::slice::from_raw_parts_mut(response_ptr, response_size as usize) };
            response
                .get_mut(..response_size as usize)
                .ok_or(Error::InvalidResponseSize)?
                .copy_from_slice(tmp_buf);
        }

        Ok(response_size as usize)
    }

    /// Return a serde de/serializable structure containing the vTPM's current
    /// runtime state.
    ///
    /// Corresponds to `VTpmGetRuntimeState`
    pub fn get_runtime_state(&self) -> MsTpm20RefRuntimeState {
        MsTpm20RefRuntimeState {
            tpmlib_state: tpmlib_state::get_runtime_state(),
            platform_state: PLATFORM
                .lock()
                .unwrap()
                .as_mut()
                .expect("platform is initialized")
                .get_runtime_state(),
        }
    }

    /// Sets or resets the Cancel flag.
    ///
    /// When set the TPM library will opportunistically abort the command being
    /// executed.
    ///
    /// Corresponds to `VTpmSetCancelFlag`
    pub fn set_cancel_flag(&mut self, enabled: bool) {
        let mut platform = PLATFORM.lock().unwrap();
        let platform = platform.as_mut().expect("platform is initialized");
        if enabled {
            platform.set_cancel()
        } else {
            platform.clear_cancel()
        }
    }

    // `VTpmSetTargetVersion` omitted for now (never used)
}

impl Drop for MsTpm20RefPlatform {
    /// Corresponds to `VTpmShutdown`
    fn drop(&mut self) {
        self.shutdown()
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct MsTpm20PlatformState {
    cancel: api::cancel::CancelState,
    locality: api::locality_plat::LocalityState,
    clock: api::clock::ClockState,
    power_plat: api::power_plat::PowerPlatState,
    nvmem: api::nvmem::NvState,
}

impl MsTpm20PlatformState {
    fn new() -> MsTpm20PlatformState {
        MsTpm20PlatformState {
            cancel: api::cancel::CancelState::new(),
            locality: api::locality_plat::LocalityState::new(),
            clock: api::clock::ClockState::new(),
            power_plat: api::power_plat::PowerPlatState::new(),
            nvmem: api::nvmem::NvState::new(),
        }
    }
}

struct MsTpm20RefPlatformImpl {
    callbacks: Box<dyn PlatformCallbacks + Send>,
    state: MsTpm20PlatformState,
}

impl MsTpm20RefPlatformImpl {
    fn new(callbacks: Box<dyn PlatformCallbacks + Send>) -> MsTpm20RefPlatformImpl {
        MsTpm20RefPlatformImpl {
            callbacks,
            state: MsTpm20PlatformState::new(),
        }
    }

    fn restore_runtime_state(&mut self, state: MsTpm20PlatformState) {
        self.state = state;
    }

    fn get_runtime_state(&self) -> MsTpm20PlatformState {
        self.state.clone()
    }
}
