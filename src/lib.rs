//! Raw FFI bindings to the microsoft/ms-tpm-20-ref C library.

#[allow(
    clippy::all,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    improper_ctypes, // u128
)]
mod bindgen;
pub use bindgen::*;
