use core::marker::PhantomData;
use std::convert::TryInto;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde::Deserialize;
use serde::Serialize;

use crate::error::*;
use crate::tpmlib_state;
use crate::InitKind;
use crate::PlatformCallbacks;

pub(crate) mod api;

// NOTE: Stashing the platform implementation behind a global Mutex is *not*
// done to enforce serialized access to the platform's various methods. The
// underlying C library is single-threaded, and will never call multiple
// platform methods at the same time. In addition, by marking
// `MsTpm20RefPlatform` as `!Sync`, we can leverage the Rust type system to
// statically guarantee that Rust code will only ever invoke platform methods on
// a single thread.
//
// Indeed, if you read through this wrapper code, you'll find that the
// potentially-deadlocking `.lock()` method is never called on the platform
// mutex, with `.try_lock()` being used instead.
//
// So, why use a mutex at all?
//
// 1. It serves as a good "assert" mechanism to ensure that the underlying C
// library is indeed single-threaded, and isn't calling platform methods at the
// same time. The current platform implementation is _not_ reentrant, and if the
// underlying TPM library ever switches to a multithreaded model, we'd want to
// fail-fast.
//
// 2. It's nicer than using a `static mut PLATFORM` + copious `unsafe` blocks to
// access the global platform. Moreover, this is not supposed to be "high
// performance" code, so the minor overhead of going through a mutex isn't
// important.
static PLATFORM: Lazy<Mutex<Option<MsTpm20RefPlatformImpl>>> = Lazy::new(|| Mutex::new(None));

// Defined in `RunCommand.c`
#[link(name = "run_command")]
extern "C" {
    fn RunCommand(
        requestSize: u32,
        request: *mut u8,
        responseSize: *mut u32,
        response: *mut *mut u8,
    );
}

// methods defined within ms-tpm-20-ref
mod ffi {
    extern "C" {
        pub fn _TPM_Init();
        pub fn TPM_Manufacture(firstTime: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
    }
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
pub struct MsTpm20RefPlatform {
    _not_sync: PhantomData<*const ()>,
}

// SAFETY: the underlying C library is single threaded, and doesn't use TLS
unsafe impl Send for MsTpm20RefPlatform {}

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
        tracing::trace!("Initializing TPM platform...");

        let mut maybe_platform = PLATFORM.try_lock().unwrap();

        match &mut *maybe_platform {
            Some(_platform) => return Err(Error::AlreadyInitialized),
            None => {
                let mut platform = MsTpm20RefPlatformImpl::new(callbacks);
                match &init_kind {
                    InitKind::ColdInit => platform.nv_enable()?,
                    InitKind::ColdInitWithPersistentState { nvmem_blob } => {
                        platform.nv_enable_from_blob(nvmem_blob)?
                    }
                };
                *maybe_platform = Some(platform);
            }
        }

        tracing::trace!("TPM platform initialized");

        // now that the platform layer has been set up, we can call into the TPM lib
        // itself to prep the TPM.
        tracing::trace!("Initializing TPM library...");

        maybe_platform.as_mut().unwrap().signal_power_on()?;

        // Make sure to drop the mutex guard, as the TPM library will call back into the
        // platform, and Rust's std mutex is not reentrant!
        drop(maybe_platform);

        if matches!(&init_kind, InitKind::ColdInit) {
            // SAFETY: TPM_Manufacture doesn't have any preconditions
            let ret = unsafe { ffi::TPM_Manufacture(true as i32) };
            if ret != 0 {
                return Err(Error::Ffi {
                    function: "TPM_Manufacture",
                    error: ret,
                });
            }
        }

        // SAFETY: the nvram state has been manufactured (either by loading an existing
        // nvram blob, or through TPM_Manufacture), and has been powered on.
        unsafe { ffi::_TPM_Init() }
        tracing::trace!("_TPM_Init Completed");

        tracing::info!("TPM library initialized");

        Ok(MsTpm20RefPlatform {
            _not_sync: PhantomData,
        })
    }

    fn shutdown(&mut self) {
        let mut platform = PLATFORM.try_lock().unwrap();
        platform.as_mut().unwrap().signal_power_off();
        *platform = None;
    }

    /// Reset the TPM device (i.e: simulate power off + power on)
    pub fn reset(&mut self, with_new_nvmem_blob: Option<&[u8]>) -> Result<(), Error> {
        tracing::trace!("Resetting TPM library...");
        // open new scope to drop the mutex before calling _TPM_Init
        {
            let mut platform = PLATFORM.try_lock().unwrap();
            let platform = platform.as_mut().unwrap();
            platform.signal_power_off();

            if let Some(nvmem_blob) = with_new_nvmem_blob {
                platform.nv_enable_from_blob(nvmem_blob)?;
            } else {
                // instead of requiring the caller to do a full roundtrip
                // through their backing nvmem storage as part of the reset, we
                // cheat and set this flag to true (after it was cleared as part
                // of signal_power_off), which lets us re-use the current nvmem
                // state in memory.
                platform.state.nvmem.is_init = true;
            }

            platform.signal_power_on()?;
        }
        // SAFETY: nvram is in a valid state, and the device is powered on.
        unsafe {
            ffi::_TPM_Init();
        }
        tracing::trace!("TPM Reset");
        Ok(())
    }

    /// Execute a command on the TPM, without parsing the request header to
    /// validate an appropriately sized request / response buffer.
    ///
    /// # Safety
    ///
    /// Callers must ensure that the request and response buffers are
    /// appropriately sized for the respective command.
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
        // SAFETY: The request / response buffers point to valid Rust slices
        unsafe {
            RunCommand(
                request_size,
                request_ptr,
                &mut response_size,
                &mut response_ptr,
            );
        }

        // NOTE: the API of the underlying C library makes it possible for the
        // underlying C library to modify the response pointer to point to a
        // different buffer than the one passed in.
        //
        // The common use case is to return a pointer to a global static buffer
        // when the TPM enters a failure mode. This is pretty easy to handle, as
        // we can simply copy data from said buffer into the response buffer
        // prior to returning from the function.
        //
        // That said, we do need to be careful against the possible case of the
        // C library returning a response pointer that points into the provided
        // request buffer. In that case, naively using
        // `slice::from_raw_parts_mut` would result in UB, as it would result in
        // two mutable Rust slices which alias the same memory location.
        //
        // This doesn't happen in the current version of the library, but we
        // double-check and handle this edge-case regardless.
        if prev_response_ptr != response_ptr {
            if response_ptr == request_ptr {
                panic!("TPM library unexpectedly returned a response in request buffer");
            }

            if response_ptr.is_null() {
                panic!("TPM library set response pointer to null");
            }

            tracing::warn!("TPM library returned a response ptr that doesn't match the provided response buffer: {:#x?} != {:#x?}", prev_response_ptr, response_ptr);

            // copy response from library provided response buffer into user response buffer
            //
            // SAFETY: C library is returning a valid, albeit different, pointer.
            let c_response =
                unsafe { core::slice::from_raw_parts_mut(response_ptr, response_size as usize) };
            response[..response_size as usize].copy_from_slice(c_response);
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
        let request_len = request.len();
        let request_header_size = request
            .get(2..6)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
            .ok_or(Error::InvalidRequestSize)?;

        if request_header_size > request_len as u32 {
            return Err(Error::InvalidRequestSize);
        }

        // SAFETY: the request buffer has been truncated to the size specified
        // in the request header
        Ok(unsafe {
            self.execute_command_unchecked(
                &mut request[..request_len.min(request_header_size as usize)],
                response,
            )
        })
    }

    /// Save the current vTPM's current state into an opaque saved-state blob.
    ///
    /// Corresponds to `VTpmGetRuntimeState`
    pub fn save_state(&self) -> Vec<u8> {
        let state = MsTpm20RefRuntimeState {
            tpmlib_state: tpmlib_state::get_runtime_state(),
            platform_state: PLATFORM
                .try_lock()
                .unwrap()
                .as_mut()
                .expect("platform is initialized")
                .get_runtime_state(),
        };

        postcard::to_stdvec(&state).expect("failed to serialize state")
    }

    /// Restore the vTPM from a previously-saved blob.
    pub fn restore_state(&mut self, state: Vec<u8>) -> Result<(), Error> {
        let state: MsTpm20RefRuntimeState =
            postcard::from_bytes(&state).map_err(Error::FailedPlatformRestore)?;

        PLATFORM
            .try_lock()
            .unwrap()
            .as_mut()
            .expect("platform is initialized")
            .restore_runtime_state(state.platform_state);

        tpmlib_state::restore_runtime_state(state.tpmlib_state)?;

        Ok(())
    }

    /// Sets or resets the Cancel flag.
    ///
    /// When set the TPM library will opportunistically abort the command being
    /// executed.
    ///
    /// Corresponds to `VTpmSetCancelFlag`
    pub fn set_cancel_flag(&mut self, enabled: bool) {
        let mut platform = PLATFORM.try_lock().unwrap();
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

/// This function is never called but is present to ensure openssl-sys is linked
/// in, which ensures that libcrypto is linked in, which ensures that the C code
/// in `overrides` can reference the crypto primitives.
///
/// This is the least bad way we could find to ensure this. If we find a better
/// way, then this should be removed.
#[allow(dead_code)]
unsafe fn ensure_openssl_is_linked() {
    // SAFETY: SHA256_Init has no preconditions, and the `SHA256_CTX` structure
    // is a POD C type.
    unsafe {
        let mut ctx = std::mem::zeroed();
        openssl_sys::SHA256_Init(&mut ctx);
    }
}
