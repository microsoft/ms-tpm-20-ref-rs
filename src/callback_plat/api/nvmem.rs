//! NVMem.c

use serde::{Deserialize, Serialize};

use crate::error::Error;

use super::super::MsTpm20RefPlatformImpl;

const NV_MEMORY_SIZE: usize = 0x4000;

#[derive(Clone, Serialize, Deserialize)]
pub struct NvState {
    pub region: Vec<u8>,
    pub is_init: bool,
}

impl NvState {
    pub fn new() -> NvState {
        NvState {
            region: Vec::new(),
            is_init: false,
        }
    }
}

#[derive(Debug)]
pub enum NvError {
    AlreadyInitialized,
    MismatchedBlobSize,
    InvalidAccess { start_offset: usize, len: usize },
}

impl From<NvError> for Error {
    fn from(e: NvError) -> Error {
        Error::NvMem(e)
    }
}

#[allow(dead_code)]
enum NvAvailability {
    Available = 0,
    WriteFailure = 1,
    RateLimit = 2,
}

impl MsTpm20RefPlatformImpl {
    pub fn nv_enable_from_blob(&mut self, blob: &[u8]) -> Result<(), Error> {
        if self.state.nvmem.is_init {
            return Err(NvError::AlreadyInitialized.into());
        }

        if blob.len() != NV_MEMORY_SIZE {
            return Err(NvError::MismatchedBlobSize.into());
        }

        self.state.nvmem.region = blob.to_vec();
        self.state.nvmem.is_init = true;

        Ok(())
    }
}

impl MsTpm20RefPlatformImpl {
    pub fn nv_enable(&mut self) -> Result<(), Error> {
        if !self.state.nvmem.is_init {
            log::warn!("calling __plat_NvEnable before `nv_enable_from_blob` was called");
            self.state.nvmem.region = vec![0; NV_MEMORY_SIZE];
            self.state.nvmem.is_init = true;
        }

        Ok(())
    }

    pub fn nv_disable(&mut self, delete: bool) {
        // `delete` is only ever used by the simulator code.
        assert_eq!(delete, false);
        self.state.nvmem.is_init = false;
    }

    fn is_nv_available(&mut self) -> NvAvailability {
        NvAvailability::Available
    }

    fn nv_memory_read(&mut self, start_offset: usize, buf: &mut [u8]) -> Result<(), Error> {
        match self
            .state
            .nvmem
            .region
            .get(start_offset..(start_offset + buf.len()))
        {
            Some(region) => buf.copy_from_slice(region),
            None => {
                return Err(NvError::InvalidAccess {
                    start_offset,
                    len: buf.len(),
                }
                .into())
            }
        }

        Ok(())
    }

    fn nv_is_different(&mut self, start_offset: usize, buf: &[u8]) -> Result<bool, Error> {
        let is_different = match self
            .state
            .nvmem
            .region
            .get_mut(start_offset..(start_offset + buf.len()))
        {
            Some(region) => region != buf,
            None => {
                return Err(NvError::InvalidAccess {
                    start_offset,
                    len: buf.len(),
                }
                .into())
            }
        };

        Ok(is_different)
    }

    fn nv_memory_write(&mut self, start_offset: usize, buf: &[u8]) -> Result<(), Error> {
        match self
            .state
            .nvmem
            .region
            .get_mut(start_offset..(start_offset + buf.len()))
        {
            Some(region) => region.copy_from_slice(buf),
            None => {
                return Err(NvError::InvalidAccess {
                    start_offset,
                    len: buf.len(),
                }
                .into())
            }
        }

        Ok(())
    }

    fn nv_memory_clear(&mut self, start: usize, size: usize) -> Result<(), Error> {
        match self.state.nvmem.region.get_mut(start..(start + size)) {
            Some(region) => region.fill(0),
            None => {
                return Err(NvError::InvalidAccess {
                    start_offset: start,
                    len: size,
                }
                .into())
            }
        }

        Ok(())
    }

    fn nv_memory_move(
        &mut self,
        source_offset: usize,
        dest_offset: usize,
        size: usize,
    ) -> Result<(), Error> {
        if source_offset + size > self.state.nvmem.region.len() {
            return Err(NvError::InvalidAccess {
                start_offset: source_offset,
                len: size,
            }
            .into());
        }

        self.state
            .nvmem
            .region
            .copy_within(source_offset..(source_offset + size), dest_offset);

        Ok(())
    }

    fn nv_commit(&mut self) -> Result<(), Error> {
        self.callbacks
            .commit_nv_state(&self.state.nvmem.region)
            .map_err(Error::PlatformCallback)
    }
}

mod c_api {
    use core::ffi::c_void;

    // NOTE: The commented out functions are only ever called from the simulator,
    // and as such, they really shouldn't have been specified as part of the the
    // platform interface...

    // #[no_mangle]
    // pub unsafe extern "C" fn _plat__NvErrors(
    //     recoverable: i32,
    //     unrecoverable: i32
    // ) {
    //      platform!().nv_errors(recoverable != 0, unrecoverable != 0)
    // }

    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__NVEnable(plat_parameter: *mut c_void) -> i32 {
        match platform!().nv_enable() {
            Ok(()) => 0,
            Err(e) => {
                log::error!("error calling _plat__NVEnable({:?}): {}", plat_parameter, e);
                -1 // TODO: assign different error IDs to each error variant?
            }
        }
    }

    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__NVDisable(delete: i32) {
        platform!().nv_disable(delete != 0)
    }

    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__IsNvAvailable() -> i32 {
        platform!().is_nv_available() as i32
    }

    // NOTE: Why doesn't NvMemoryRead return a bool like NvMemoryWrite??
    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__NvMemoryRead(start_offset: u32, size: u32, data: *mut c_void) {
        let buf = unsafe { core::slice::from_raw_parts_mut(data as *mut u8, size as usize) };

        match platform!().nv_memory_read(start_offset as usize, buf) {
            Ok(()) => {}
            Err(e) => {
                log::error!(
                    "error calling _plat__NvMemoryRead(start_offset: {:#x?}, size: {:#x?}, data: {:?}): {}",
                    start_offset,
                    size,
                    data,
                    e
                );
            }
        }
    }

    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__NvIsDifferent(
        start_offset: u32,
        size: u32,
        data: *mut c_void,
    ) -> i32 {
        let buf = unsafe { core::slice::from_raw_parts(data as *const u8, size as usize) };

        match platform!().nv_is_different(start_offset as usize, buf) {
            Ok(is_diff) => is_diff as i32,
            Err(e) => {
                log::error!(
                    "error calling _plat__NvIsDifferent(start_offset: {:#x?}, size: {:#x?}, data: {:?}): {}",
                    start_offset,
                    size,
                    data,
                    e
                );
                // need to return something... might as well say the memory is different?
                true as i32
            }
        }
    }

    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__NvMemoryWrite(
        start_offset: u32,
        size: u32,
        data: *mut c_void,
    ) -> i32 {
        let buf = unsafe { core::slice::from_raw_parts(data as *const u8, size as usize) };

        match platform!().nv_memory_write(start_offset as usize, buf) {
            Ok(()) => true as i32,
            Err(e) => {
                log::error!(
                    "error calling _plat__NvMemoryWrite(start_offset: {:#x?}, size: {:#x?}, data: {:?}): {}",
                    start_offset,
                    size,
                    data,
                    e
                );
                false as i32
            }
        }
    }

    // NOTE: Why doesn't NvMemoryClear return a bool??
    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__NvMemoryClear(start: u32, size: u32) {
        match platform!().nv_memory_clear(start as usize, size as usize) {
            Ok(()) => {}
            Err(e) => {
                log::error!(
                    "error calling _plat__NvMemoryClear(start: {:#x?}, size: {:#x?}): {}",
                    start,
                    size,
                    e
                );
            }
        }
    }

    // NOTE: Why doesn't NvMemoryClear return a bool??
    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__NvMemoryMove(source_offset: u32, dest_offset: u32, size: u32) {
        match platform!().nv_memory_move(
            source_offset as usize,
            dest_offset as usize,
            size as usize,
        ) {
            Ok(()) => {}
            Err(e) => {
                log::error!(
                    "error calling _plat__NvMemoryMove(source_offset: {:#x?}, dest_offset: {:#x?}, size: {:#x?}): {}",
                    source_offset,
                    dest_offset,
                    size,
                    e
                );
            }
        }
    }

    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__NvCommit() -> i32 {
        match platform!().nv_commit() {
            Ok(()) => 0,
            Err(e) => {
                log::error!("error calling _plat__NvCommit(): {}", e);
                1
            }
        }
    }
}
