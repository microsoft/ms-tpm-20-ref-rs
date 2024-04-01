// Copyright (C) Microsoft Corporation. All rights reserved.

//! Callback-based Platform implementation for `ms-tpm-20-ref`

#![warn(missing_docs)]

mod error;
mod plat;
mod tpmlib_state;

pub use error::DynResult;
pub use error::Error;
pub use plat::MsTpm20RefPlatform;
pub use plat::MsTpm20RefRuntimeState;

use std::borrow::Cow;

/// Various library initialization modes
pub enum InitKind<'a> {
    /// Initialize the TPM entirely from scratch, having it manufacture an
    /// initial nvmem blob.
    ColdInit,
    /// Initialize the TPM from an existing saved nvmem blob.
    ColdInitWithPersistentState {
        /// Opaque nvmem blob
        nvmem_blob: Cow<'a, [u8]>,
    },
}

impl core::fmt::Debug for InitKind<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitKind::ColdInit => write!(f, "ColdInit"),
            InitKind::ColdInitWithPersistentState { .. } => {
                write!(f, "ColdInitWithPersistentState {{ .. }}")
            }
        }
    }
}

/// Implementation-specific platform callbacks.
pub trait PlatformCallbacks {
    /// Persist the provided non volatile state.
    fn commit_nv_state(&mut self, state: &[u8]) -> DynResult<()>;

    /// Write cryptographically secure random bytes into `buf`.
    ///
    /// Returns the number of bytes written into `buf`.
    fn get_crypt_random(&mut self, buf: &mut [u8]) -> DynResult<usize>;

    /// Return a monotonically increasing duration.
    ///
    /// A simple implementation can simply initialize a [`std::time::Instant`],
    /// and then call `.elapsed()` on it.
    fn monotonic_timer(&mut self) -> std::time::Duration;

    /// Return a platform specific unique number that is used as
    /// VENDOR_PERMANENT authorization value.
    ///
    /// This function MUST return the same value each time it is called.
    fn get_unique_value(&self) -> &'static [u8];
}

/// Sample platform callback implementation that simply logs invocations +
/// returns dummy data.
pub struct DummyPlatformCallbacks;

impl PlatformCallbacks for DummyPlatformCallbacks {
    fn commit_nv_state(&mut self, state: &[u8]) -> DynResult<()> {
        tracing::info!("committing nv state with len {}", state.len());
        Ok(())
    }

    fn get_crypt_random(&mut self, buf: &mut [u8]) -> DynResult<usize> {
        tracing::info!("returning dummy entropy into buf of len {}", buf.len());
        if let Some(b) = buf.get_mut(0) {
            *b = 1;
        }

        Ok(buf.len())
    }

    fn monotonic_timer(&mut self) -> std::time::Duration {
        tracing::info!("checking time from the platform");
        std::time::Duration::ZERO
    }

    fn get_unique_value(&self) -> &'static [u8] {
        tracing::info!("fetching unique value from platform");
        b"somebody once told me the world was gonna roll me, I ain't the sharpest tool in the shed"
    }
}
