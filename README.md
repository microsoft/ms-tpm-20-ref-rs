# ms-tpm-20-ref-sys

Rust FFI bindings to [microsoft/ms-tpm-20-ref](https://github.com/microsoft/ms-tpm-20-ref).

## Dependencies

### Windows

TBD

### Linux

On Debian-based systems (such as Ubuntu):

```bash
sudo apt install autoconf-archive pkg-config build-essential automake libssl-dev
```

## Updating `ms-tpm-20-ref`

After bumping the git submodule, make sure to regenerate `bindgen.rs`!

This can be done by running `./bindgen.sh` from the root of the repo.

## Attribution

The code under `build/openssl/` was extracted and lightly modified from the [openssl-sys](https://github.com/sfackler/rust-openssl/tree/master/openssl-sys) crate, used with permission under the MIT license. See `build/openssl/LICENSE` for a copy of the original license.
