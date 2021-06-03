//! Callback-based Platform implementation for `ms-tpm-20-ref`

#![deny(future_incompatible, nonstandard_style, rust_2018_idioms)]
#![warn(
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::filter_map_next,
    clippy::fn_params_excessive_bools,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::map_flatten,
    clippy::match_on_vec_items,
    clippy::mismatched_target_os,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::option_option,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::string_to_string,
    clippy::suboptimal_flops,
    clippy::verbose_file_reads
)]
#![allow(
    clippy::collapsible_else_if,
    clippy::collapsible_if,
    clippy::len_without_is_empty,
    clippy::mutex_atomic,
    clippy::new_without_default,
    clippy::too_many_arguments,
    clippy::transmute_ptr_to_ptr,
    clippy::transmutes_expressible_as_ptr_casts,
    clippy::type_complexity
)]
// crate-specific warnings
#![warn(unsafe_op_in_unsafe_fn)]

#[allow(
    clippy::all,
    dead_code,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    improper_ctypes, // u128
)]
mod bindgen;

mod error;
mod tpmlib_state;

cfg_if::cfg_if! {
    if #[cfg(feature = "sample_platform")] {
        mod sample_plat;
        use sample_plat as plat;
    } else {
        mod callback_plat;
        use callback_plat as plat;
    }
}

pub use plat::{MsTpm20RefPlatform, MsTpm20RefRuntimeState};

pub use error::*;

pub enum InitKind<'a> {
    ColdInit,
    ColdInitWithPersistentState {
        nvmem_blob: &'a [u8],
    },
    WarmInit {
        nvmem_blob: &'a [u8],
        runtime_state: MsTpm20RefRuntimeState,
    },
}

/// Implementation-specific platform callbacks.
pub trait PlatformCallbacks {
    /// Persist the provided non volative state.
    fn commit_nv_state(&mut self, state: &[u8]) -> DynResult<()>;

    /// Write cryptographically secure random bytes into `buf`.
    ///
    /// Returns the number of bytes written into `buf`.
    fn get_crypt_random(&mut self, buf: &mut [u8]) -> DynResult<usize>;

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
        log::info!("committing nv state with len {}", state.len());
        Ok(())
    }

    fn get_crypt_random(&mut self, buf: &mut [u8]) -> DynResult<usize> {
        log::info!("returning dummy entropy into buf of len {}", buf.len());
        if let Some(b) = buf.get_mut(0) {
            *b = 1;
        }

        Ok(buf.len())
    }

    fn get_unique_value(&self) -> &'static [u8] {
        log::info!("fetching unique value from platform");
        b"somebody once told me the world was gonna roll me, I ain't the sharpest tool in the shed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        pretty_env_logger::init();

        let mut platform =
            MsTpm20RefPlatform::initialize(Box::new(DummyPlatformCallbacks), InitKind::ColdInit)
                .unwrap();

        // use raw variant to make sure the tpm fail setjmp/longjmp code works correctly
        unsafe {
            platform.execute_command_unchecked(&mut [0; 4096], &mut [0; 4096]);
        }

        drop(platform)
    }
}
