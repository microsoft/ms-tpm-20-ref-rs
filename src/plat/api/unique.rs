// Copyright (C) Microsoft Corporation. All rights reserved.

//! Unique.c

use super::super::MsTpm20RefPlatformImpl;

impl MsTpm20RefPlatformImpl {
    fn get_unique(&mut self, _which: u32, buf: &mut [u8]) -> usize {
        // TODO: how to handle `which`?

        tracing::debug!("fetching first {} unique value bytes", buf.len());

        let unique = self.callbacks.get_unique_value();

        let n = buf.len().min(unique.len());
        buf[..n].copy_from_slice(&unique[..n]);
        n
    }
}

mod c_api {
    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__GetUnique(which: u32, b_size: u32, b: *mut u8) -> u32 {
        assert!(!b.is_null());

        // SAFETY: Caller guarantees `b` and `b_size` are valid.
        let buf = unsafe { core::slice::from_raw_parts_mut(b, b_size as usize) };
        platform!().get_unique(which, buf) as u32
    }
}
