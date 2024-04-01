# ms-tpm-20-ref-rs

Rust bindings to the
[microsoft/ms-tpm-20-ref](https://github.com/microsoft/ms-tpm-20-ref) C library.

## Features

All features are disabled by default.

- `vendored` - Compile OpenSSL from source (corresponds to `openssl/vendored`)

## Building

The core C code in `microsoft/ms-tpm-20-ref` has no external dependencies, and
can be compiled with just about any C compiler.

See the `openssl` crate documentation for instructions on how to build + link
against OpenSSL: <https://docs.rs/openssl/latest/openssl/#building>

## Relationship to `tpm-rs`

This crate is NOT associated with the <https://github.com/tpm-rs> project.

This crate wraps the existing C-based TPM codebase, only rewriting the generic
"platform" layer in Rust, without porting the underlying "engine" to Rust.

For a pure Rust implementation of the TPM 2.0 specification, see (and support!)
the effort over at <https://github.com/tpm-rs/tpm-rs>.

## Versioning

### Supported `ms-tpm-20-ref` versions

At this time, the only supported version of `microsoft/ms-tpm-20-ref` that this
crate can compile + link against is revision 1.38.

This particular revision was selected in order to maintain compatibility with
the vTPM device used in Hyper-V.

In the future, this crate may be updated to support compiling + linking against
alternate versions of `microsoft/ms-tpm-20-ref`, though at this time, there is
no concrete roadmap as to when that is going to happen.

If you are interested in extending `ms-tpm-20-ref-rs` to work with multiple
alternate versions of `microsoft/ms-tpm-20-ref`, please feel free to reach out
by opening a GitHub Issue.

### Supported crypto backends

While the underlying `microsoft/ms-tpm-20-ref` library does support multiple
different crypto backends, at this time, the only supported crypto backend is
OpenSSL 3.x.

This particular backend was selected in order to seamlessly integrate
`ms-tpm-20-ref-rs` into a larger codebase that was already using OpenSSL 3.x.

In the future, this crate may be updated to support linking against alternate
crypto backends, though at this time, there is no concrete roadmap as to when
that is going to happen.

If you are interested in extending `ms-tpm-20-ref-rs` to work with
alternate crypto backends, please feel free to reach out by opening a
GitHub Issue.

### Saved-state compatibility

At this time, `microsoft/ms-tpm-20-ref` makes no guarantees as to the stability
of its saved state across major revisions. This applies to both volatile
(in-memory), and non-volatile (nvram) state.

As such, `ms-tpm-20-ref-rs` makes the exact same guarantees wrt. saved state.

If you are interested as to why this is the case, and why it is not trivial to
support inter-revision migration, see
[docs/upgrade_138_to_162.md](docs/upgrade_138_to_162.md).
