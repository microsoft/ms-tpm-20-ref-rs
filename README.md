# ms-tpm-20-ref

**HvLite is currently using the `138-compat` branch!**

Rust wrapper around
[microsoft/ms-tpm-20-ref](https://github.com/microsoft/ms-tpm-20-ref).

Ideally, we'd be able to have a separate `*-sys` crate that encapsulates the
bindings to the underlying ms-tpm-20-ref lib, but unfortunately, due to the
library's bi-directional communication with the platform layer, both the
platform layer implementation and the C library bindings need to be performed
within a single Rust crate (i.e: a single translation unit).

## Features

- `vendored` - if enabled, `openssl` will be compiled from source. **WARNING: This will
  substantially bump compile-from-clean times!**

## Build Dependencies

The `ms-tpm-20-ref` library technically supports several different crypto
backends: openSSL, wolfSSL, and SymCrypt.

At the moment, only the openSSL backend is supported.

### Linux

On Debian-based systems (such as Ubuntu):

```bash
sudo apt install pkg-config build-essential libssl-dev
```

### Linux MUSL

At the moment, compiling on Linux MUSL targets requires using the `vendored`
feature, as the builds system doesn't have any logic for ingesting pre-built
MUSL openSSL static libs.

### Windows

_Theoretically_, it is possible to use a pre-compiled openSSL binary via vcpkg,
but this isn't something that's been tested working.

TODO: actually figure this out.

For now, the `vendored` feature must be enabled, which will build openSSL from
source.

## Upgrading `ms-tpm-20-ref`

See the `UPGRADE_PATH.md` document for information on how to update the
underlying `ms-tpm-20-ref` version, along with a brief discussion around
maintaining backwards-compatibility with earlier library versions.
