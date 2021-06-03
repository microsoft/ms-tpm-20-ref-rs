//! Entropy.c

use crate::error::Error;

use super::super::MsTpm20RefPlatformImpl;

impl MsTpm20RefPlatformImpl {
    fn get_entropy(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.callbacks
            .get_crypt_random(buf)
            .map_err(Error::PlatformCallback)
    }
}

mod c_api {
    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__GetEntropy(entropy: *mut u8, amount: u32) -> i32 {
        let buf = unsafe { core::slice::from_raw_parts_mut(entropy, amount as usize) };

        match platform!().get_entropy(buf) {
            Ok(len) => len as i32,
            Err(e) => {
                log::error!(
                    "error calling _plat__GetEntropy(entropy: {:?}, amount: {:#x?}): {}",
                    entropy,
                    amount,
                    e
                );
                -1
            }
        }
    }
}
