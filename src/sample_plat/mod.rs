use std::convert::TryInto;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};

use crate::error::*;
use crate::InitKind;
use crate::PlatformCallbacks;

const NV_STATE_FILE: &str = "NVChip";

static INITIALIZED: AtomicBool = AtomicBool::new(false);

fn cerr(val: std::os::raw::c_int) -> Result<i32, Error> {
    if val >= 0 {
        Ok(val)
    } else {
        Err(Error::Ffi {
            function: "<not given>",
            error: val,
        })
    }
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
    /// NOTE: Unlike the C++ platform implementation, this method will NOT send
    /// the TPM startup command or selftest commands.
    pub fn initialize(
        _callbacks: Box<dyn PlatformCallbacks + Send>,
        _init_kind: InitKind<'_>,
    ) -> Result<MsTpm20RefPlatform, Error> {
        log::warn!("Using sample platform implementation!");
        log::warn!("Ignoring the provided callbacks...");
        log::warn!("Ignoring both the runtime and persistent state blobs...");
        log::warn!("Reading/Writing from/to '{}' file directly", NV_STATE_FILE);

        if !INITIALIZED.fetch_or(true, std::sync::atomic::Ordering::SeqCst) {
            log::trace!("Initializing TPM...");

            unsafe {
                crate::bindgen::_plat__SetNvAvail();
                log::trace!("TPM _plat__SetNvAvail Completed");

                cerr(crate::bindgen::_plat__NVEnable(std::ptr::null_mut()))?;
                log::trace!("TPM _plat__NVEnable Completed");

                cerr(crate::bindgen::_plat__Signal_PowerOn())?;
                log::trace!("TPM _plat__Signal_PowerOn Completed");

                let needs_manufacture = crate::bindgen::_plat__NVNeedsManufacture() == 1;
                log::trace!("TPM _plat__NVNeedsManufacture Completed");

                if needs_manufacture {
                    cerr(crate::bindgen::TPM_Manufacture(1))?;
                    log::trace!("TPM TPM_Manufacture Completed");
                }

                crate::bindgen::_TPM_Init();
                log::trace!("TPM _TPM_Init Completed");
            }

            log::info!("TPM Initialized");
            Ok(MsTpm20RefPlatform {})
        } else {
            Err(Error::AlreadyInitialized)
        }
    }

    fn shutdown(&mut self) {
        unsafe {
            crate::bindgen::_plat__Signal_PowerOff();
        }
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
            crate::bindgen::_plat__RunCommand(
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
            crate::bindgen::_plat__RunCommand(
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
        MsTpm20RefRuntimeState {}
    }

    /// Sets or resets the Cancel flag.
    ///
    /// When set the TPM library will opportunistically abort the command being
    /// executed.
    ///
    /// Corresponds to `VTpmSetCancelFlag`
    pub fn set_cancel_flag(&mut self, _enabled: bool) {
        unimplemented!()
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
pub struct MsTpm20RefRuntimeState {}
