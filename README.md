# ms-tpm-20-ref-sys

Rust FFI bindings to [microsoft/ms-tpm-20-ref](https://github.com/microsoft/ms-tpm-20-ref).

**NOTE:** Unless the `sample_platform` cargo feature is enabled, this crate will _not_ compile a platform implementation!

Users are expected to provide their own platform implementation by ensuring the functions defined by [TPMCmd/Platform/include/prototypes/Platform_fp.h](https://github.com/microsoft/ms-tpm-20-ref/blob/master/TPMCmd/Platform/include/prototypes/Platform_fp.h) and [TPMCmd/Platform/include/Platform.h](https://github.com/microsoft/ms-tpm-20-ref/blob/master/TPMCmd/Platform/include/prototypes/Platform_fp.h) are available at link time.

Reminder: if implementing platform layer in Rust, make sure to mark the functions as `#[no_mangle] pub extern "C"`.

## Features

- `vendored` - if enabled, `openssl` will be compiled from source
- `sample_platfom` - if enabled, the microsoft/ms-tpm-20-ref sample platform implementation will be compiled + linked in

## Build Dependencies

### Linux

On Debian-based systems (such as Ubuntu):

```bash
sudo apt install pkg-config build-essential libssl-dev
```

### Windows

TODO

## Updating `ms-tpm-20-ref`

After bumping the git submodule, make sure to regenerate `src/bindgen.rs` using the `./bindgen.sh` script!

## Attribution

The code under `build/openssl/` was extracted and lightly modified from the [openssl-sys](https://github.com/sfackler/rust-openssl/tree/master/openssl-sys) crate, used with permission under the MIT license. See `build/openssl/LICENSE` for a copy of the original license.
