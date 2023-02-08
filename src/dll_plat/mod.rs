use core::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::error::*;
use crate::InitKind;
use crate::PlatformCallbacks;

/// Serde de/serializable representation of the ms-tpm-20-ref library's runtime
/// state (both core C library runtime, and Rust platform runtime)
#[derive(Clone, Serialize, Deserialize)]
pub struct MsTpm20RefRuntimeState {}

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
    lib: libloading::Library,
    _not_sync: PhantomData<*const ()>,
}

// SAFETY: the underlying C library is single threaded, and doesn't use TLS
unsafe impl Send for MsTpm20RefPlatform {}

#[allow(non_camel_case_types, unused)]
mod tpmengum_types {
    pub type FTPM_STATE_PERSIST_CALLBACK = extern "C" fn(
        /* ctx: */ *const core::ffi::c_void,
        /* blob: */ *const u8,
        /* blob_size: */ u32,
    ) -> u32;

    // VTpmColdInitWithPersistentState
    pub type VTpmColdInitWithPersistentStateFn = unsafe extern "C" fn(
        /* blob: */ *const u8,
        /* blob_size: */ u32,
        /* cb: */ FTPM_STATE_PERSIST_CALLBACK,
        /* ctx: */ *const core::ffi::c_void,
    ) -> u32;

    // VTpmWarmInit
    // TODO

    // VTpmShutdown
    pub type VTpmShutdownFn = unsafe extern "C" fn() -> u32;

    // VTpmExecuteCommand
    pub type VTpmExecuteCommandFn = unsafe extern "C" fn(
        /* req_size: */ u32,
        /* req: */ *const u8,
        /* res_size: */ *mut u32,
        /* res: */ *mut u8,
    ) -> u32;

    // VTpmSetCancelFlag
    pub type VTpmSetCancelFlagFn = unsafe extern "C" fn(/* flagValue: */ i32);

    // VTpmGetRuntimeState
    // TODO

    // VTpmSetTargetVersion
    // TODO
}

extern "C" fn state_persist_callback(
    ctx: *const core::ffi::c_void,
    blob: *const u8,
    blob_size: u32,
) -> u32 {
    log::debug!("called state persist callback");

    if blob_size != 0x8000 {
        eprintln!(
            "expected blob size of 0x8000, got one of size {:#x?}",
            blob_size
        );
        eprintln!("are you using an up to date version of TpmEngUM138.dll?");
        eprintln!("you might accidentally be loading a stale dll from System32");
        panic!("unexpected nvmem blob size");
    }

    if blob.is_null() {
        log::debug!("passed null blob to state persist callback!");
        return 0;
    }

    let mut callbacks: Box<Box<dyn PlatformCallbacks + Send>> = unsafe { Box::from_raw(ctx as _) };

    let blob = unsafe { core::slice::from_raw_parts(blob, blob_size as usize) };

    let ret: i32 = match callbacks.commit_nv_state(blob) {
        Ok(()) => 0,
        Err(e) => {
            log::error!("error committing nv state: {}", e);
            -1
        }
    };

    std::mem::forget(callbacks);

    ret as u32
}

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
        let lib = unsafe { libloading::Library::new("TpmEngUM138.dll").unwrap() };

        let vtpm_cold_init_with_persistent_state = unsafe {
            lib.get::<tpmengum_types::VTpmColdInitWithPersistentStateFn>(
                b"VTpmColdInitWithPersistentState",
            )
            .unwrap()
        };

        match init_kind {
            InitKind::ColdInit => unsafe {
                let ret = vtpm_cold_init_with_persistent_state(
                    std::ptr::null(),
                    0,
                    state_persist_callback,
                    Box::into_raw(Box::new(callbacks)) as _,
                );

                if ret != 0 {
                    return Err(Error::Ffi {
                        function: "VTpmColdInitWithPersistentState",
                        error: ret as i32,
                    });
                }
            },
            InitKind::ColdInitWithPersistentState { nvmem_blob } => unsafe {
                let ret = vtpm_cold_init_with_persistent_state(
                    nvmem_blob.as_ptr(),
                    nvmem_blob.len() as u32,
                    state_persist_callback,
                    Box::into_raw(Box::new(callbacks)) as _,
                );

                if ret != 0 {
                    return Err(Error::Ffi {
                        function: "VTpmColdInitWithPersistentState",
                        error: ret as i32,
                    });
                }
            },
        }

        Ok(MsTpm20RefPlatform {
            lib,
            _not_sync: PhantomData,
        })
    }

    fn shutdown(&mut self) {
        let vtpm_cold_init_with_persistent_state = unsafe {
            self.lib
                .get::<tpmengum_types::VTpmShutdownFn>(b"VTpmShutdown")
                .unwrap()
        };

        let ret = unsafe { vtpm_cold_init_with_persistent_state() };
        if ret != 0 {
            log::error!("TPM dll failed to shutdown: {}", ret);
        }
    }

    /// Reset the TPM device (i.e: simulate power off + power on)
    pub fn reset(&mut self) -> Result<(), Error> {
        unimplemented!();
    }

    unsafe fn execute_command_unchecked_inner(
        &mut self,
        request: &mut [u8],
        response: &mut [u8],
    ) -> Result<usize, Error> {
        let vtpm_execute_command = unsafe {
            self.lib
                .get::<tpmengum_types::VTpmExecuteCommandFn>(b"VTpmExecuteCommand")
                .unwrap()
        };

        let request_size = request.len() as u32;
        let request_ptr = request.as_mut_ptr();
        let mut response_size = response.len() as u32;
        let response_ptr = response.as_mut_ptr();

        // SAFETY: The request / response buffers point to valid memory locations
        let ret = unsafe {
            vtpm_execute_command(
                request_size,
                request_ptr,
                (&mut response_size) as *mut _,
                response_ptr,
            )
        };

        if ret != 0 {
            return Err(Error::Ffi {
                function: "VTpmExecuteCommand",
                error: ret as i32,
            });
        }

        Ok(response_size as usize)
    }

    /// Execute a command on the TPM without checking / truncating request /
    /// response buffers to the size specified by the contained command.
    ///
    /// # Safety
    ///
    /// Callers must ensure that the request buffer is appropriately sized for
    /// the contained command.
    pub unsafe fn execute_command_unchecked(
        &mut self,
        request: &mut [u8],
        response: &mut [u8],
    ) -> usize {
        unsafe {
            self.execute_command_unchecked_inner(request, response)
                .unwrap()
        }
    }

    /// Execute a command on the vTPM.
    ///
    /// Corresponds to `VTpmExecuteCommand`
    pub fn execute_command(
        &mut self,
        request: &mut [u8],
        response: &mut [u8],
    ) -> Result<usize, Error> {
        // SAFETY: TPM dll performs buffer truncation
        unsafe { self.execute_command_unchecked_inner(request, response) }
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
    pub fn set_cancel_flag(&mut self, enabled: bool) {
        let vtpm_set_cancel_flag = unsafe {
            self.lib
                .get::<tpmengum_types::VTpmSetCancelFlagFn>(b"VTpmSetCancelFlag")
                .unwrap()
        };

        unsafe { vtpm_set_cancel_flag(enabled as i32) };
    }

    // `VTpmSetTargetVersion` omitted for now (never used)
}

impl Drop for MsTpm20RefPlatform {
    /// Corresponds to `VTpmShutdown`
    fn drop(&mut self) {
        self.shutdown()
    }
}
