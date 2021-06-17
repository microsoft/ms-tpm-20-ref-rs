# ms-tpm-20-ref

Rust wrapper around [microsoft/ms-tpm-20-ref](https://github.com/microsoft/ms-tpm-20-ref).

Ideally, we'd be able to have a separate `*-sys` crate that encapsulates the bindings to the underlying ms-tpm-20-ref lib, but unfortunately, due to the library's bi-directional communication with the platform layer, both the platform layer implementation and the C library bindings need to be performed within a single Rust crate (i.e: a single translation unit).

## Features

- `vendored` - if enabled, `openssl` will be compiled from source
- `sample_platfom` - if enabled, the `microsoft/ms-tpm-20-ref` sample platform implementation will be compiled + linked in instead of the Rust, callback based platform. The API will remain the same, but all provided callbacks / state blobs will simply be ignored. **This should only be used for testing and cross-validation.**

## Build Dependencies

### Linux

On Debian-based systems (such as Ubuntu):

```bash
sudo apt install pkg-config build-essential libssl-dev
```

### Windows

TODO

## Updating `ms-tpm-20-ref`

Since this crate also provides an _implementation_ of the platform API, any changes in the underlying `ms-tpm-20-ref` platform API will require updating this crate's implementation as well. This cannot be automated, and will require a human to audit / validate that all platform API signatures line up correctly.

## Attribution

The code under `build/openssl/` was extracted and lightly modified from the [openssl-sys](https://github.com/sfackler/rust-openssl/tree/master/openssl-sys) crate, used with permission under the MIT license. See `build/openssl/LICENSE` for a copy of the original license.
