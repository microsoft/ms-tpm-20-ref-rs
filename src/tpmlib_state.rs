// Copyright (C) Microsoft Corporation. All rights reserved.

//! Bindings to injected `runtime_state.c`, which allows doing hot save/restores
//! of TPM C library state.

use crate::error::Error;
use serde::Deserialize;
use serde::Serialize;

#[link(name = "tpm")]
extern "C" {
    // Returns:
    // - 0 on success
    // - 1 for invalid arg
    // - 2 for insufficient size (setting pBufferSize to required size)
    pub fn INJECTED_GetRuntimeState(pBuffer: *mut u8, pBufferSize: *mut u32) -> i32;

    // Returns:
    // - 0 on success
    // - 1 for invalid arg
    // - 2 for size mismatch
    // - 3 for format validation error
    pub fn INJECTED_ApplyRuntimeState(pBuffer: *const u8, pBufferSize: u32) -> i32;
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MsTpm20RefLibraryState {
    opaque: Vec<u8>,
}

pub fn get_runtime_state() -> MsTpm20RefLibraryState {
    let mut size: u32 = 0;
    // SAFETY: passing a nullptr returns the required size
    let ret = unsafe { INJECTED_GetRuntimeState(std::ptr::null_mut(), &mut size) };

    assert_eq!(ret, 2);
    assert_ne!(size, 0);

    let mut state = MsTpm20RefLibraryState {
        opaque: vec![0; size as usize],
    };

    // SAFETY: passing in pointer + size corresponding to perfectly-sized buffer
    // (as per previous call)
    let ret = unsafe { INJECTED_GetRuntimeState(state.opaque.as_mut_ptr(), &mut size) };

    assert_eq!(ret, 0);

    state
}

pub fn restore_runtime_state(state: MsTpm20RefLibraryState) -> Result<(), Error> {
    // SAFETY: passing valid pointer + size pair from a Rust Vec<u8>
    let ret =
        unsafe { INJECTED_ApplyRuntimeState(state.opaque.as_ptr(), state.opaque.len() as u32) };

    match ret {
        0 => Ok(()),
        1 => unreachable!(), // API is being used correctly
        2 => Err(Error::InvalidRestoreSize),
        3 => Err(Error::InvalidRestoreFormat),
        _ => unreachable!(),
    }
}
