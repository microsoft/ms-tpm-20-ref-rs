#!/bin/bash

if ! [ -x "$(command -v bindgen)" ]; then
  echo 'Error: bindgen is not installed.' >&2
  echo 'Please run "cargo install bindgen"' >&2
  echo 'You may also need to install libclang. See https://rust-lang.github.io/rust-bindgen/requirements.html' >&2

  exit 1
fi

bindgen \
    ./bindgen.h \
    -o src/bindgen.rs \
    -- \
    -I./ms-tpm-20-ref/TPMCmd/tpm/include/ \
    -I./ms-tpm-20-ref/TPMCmd/tpm/include/Ossl/ \
    -I./ms-tpm-20-ref/TPMCmd/tpm/include/prototypes \
    -I./ms-tpm-20-ref/TPMCmd/Platform/include/ \
    -I./ms-tpm-20-ref/TPMCmd/Platform/include/prototypes \
    -DHASH_LIB=Ossl \
    -DSYM_LIB=Ossl \
    -DMATH_LIB=Ossl
