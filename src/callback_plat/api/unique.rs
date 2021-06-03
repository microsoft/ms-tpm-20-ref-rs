//! Unique.c

use super::super::MsTpm20RefPlatformImpl;

impl MsTpm20RefPlatformImpl {
    fn get_unique(&mut self, _which: u32, buf: &mut [u8]) -> usize {
        // TODO: how to handle `which`?

        let unique = self.callbacks.get_unique_value();

        let n = buf.len().min(unique.len());
        buf[..n].copy_from_slice(&unique[..n]);
        n
    }
}

mod c_api {
    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__GetUnique(which: u32, b_size: u32, b: *mut u8) -> u32 {
        let buf = unsafe { core::slice::from_raw_parts_mut(b, b_size as usize) };
        platform!().get_unique(which, buf) as u32
    }
}
