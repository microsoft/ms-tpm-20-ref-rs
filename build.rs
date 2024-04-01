// Copyright (C) Microsoft Corporation. All rights reserved.

#![deny(clippy::exit)]

use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

// corresponds to path within git submodule.
const MS_TPM_20_REF_SRC_PATH: &str = "./ms-tpm-20-ref/TPMCmd/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `RunCommand.c` contains setjmp/longjmp code, and must be compiled in
    // separately
    cc::Build::new()
        .file("./src/plat/RunCommand.c")
        .compile("run_command");

    // users can link against a pre-built `libtpm.a` if they don't want to use
    // the version of `ms-tpm-20-ref` included in-tree
    match env("TPM_LIB_DIR") {
        Some(var) => {
            println!("cargo:rustc-link-search=native={}", var.to_string_lossy());
            println!("cargo:rustc-link-lib=static=tpm");
            return Ok(());
        }
        None => compile_ms_tpm_20_ref()?,
    }

    Ok(())
}

/// Compile the `ms-tpm-20-ref` C codebase to a statically linked `tpmlib.a`.
///
/// See `README.md` for additional info regarding supported TPM library versions
/// and crypto backends.
fn compile_ms_tpm_20_ref() -> Result<(), Box<dyn std::error::Error>> {
    // DEVNOTE: While there are undoubtedly better ways one could've structured
    // this code... this approach has worked _well enough_, so

    let tpm_src_path = PathBuf::from(MS_TPM_20_REF_SRC_PATH);

    let overrides = [
        "tpm/src/crypt/ossl/TpmToOsslSupport.c",
        "tpm/src/crypt/ossl/TpmToOsslMath.c",
        "tpm/src/crypt/ossl/TpmToOsslDesSupport.c",
        "tpm/include/prototypes/TpmToOsslMath_fp.h",
        "tpm/include/prototypes/TpmToOsslDesSupport_fp.h",
        "tpm/include/prototypes/TpmToOsslSupport_fp.h",
        "tpm/include/ossl/TpmToOsslSym.h",
        "tpm/include/ossl/TpmToOsslHash.h",
        "tpm/include/ossl/TpmToOsslMath.h",
        "tpm/include/CompilerDependencies.h",
        "tpm/include/TpmBuildSwitches.h",
        "tpm/include/Implementation.h",
    ]
    .iter()
    .map(|p| tpm_src_path.join(p))
    .collect::<Vec<_>>();

    for path in overrides {
        if let Err(err) = std::fs::remove_file(&path) {
            if !matches!(err.kind(), std::io::ErrorKind::NotFound) {
                eprintln!("error deleting {:?}", path);
                return Err(err.into());
            }
        }
    }

    // Get the openssl include path from the openssl-sys crate.
    let ossl_include = if let Ok(include) = std::env::var("DEP_OPENSSL_INCLUDE") {
        PathBuf::from(include)
    } else {
        return Err("openssl not found".into());
    };

    let mut builder = cc::Build::new();
    builder.include(&ossl_include);

    let includes = [
        "./overrides/include".into(),
        "./overrides/include/ossl".into(),
        "./overrides/include/prototypes".into(),
        tpm_src_path.join("tpm/include"),
        tpm_src_path.join("tpm/include/prototypes"),
        tpm_src_path.join("Platform/include"),
        tpm_src_path.join("Platform/include/prototypes"),
    ];

    for path in includes.iter() {
        builder.include(path);
    }

    // we have a custom openssl 3.0 based crypto implementation, so don't build
    // the in-tree openssl 1.0 based crypto implementation.
    let excludes = [
        tpm_src_path.join("tpm/src/crypt/ossl/TpmToOsslDesSupport.c"),
        tpm_src_path.join("tpm/src/crypt/ossl/TpmToOsslMath.c"),
        tpm_src_path.join("tpm/src/crypt/ossl/TpmToOsslSupport.c"),
    ];

    add_deps(&mut builder, &tpm_src_path.join("tpm"), &excludes)?;
    add_deps(&mut builder, "./overrides/src/", &[])?;

    #[rustfmt::skip]
    builder
        // suppress warnings that fire _everywhere_ in the TPM codebase
        .flag_if_supported("-Wno-cast-function-type")
        .flag_if_supported("-Wno-ignored-qualifiers")
        // warnings specific to ossl 3.0 stuff
        .flag_if_supported("-Wno-deprecated-declarations")
        // crank up this warning to catch issues in custom override code
        .flag_if_supported("-Werror=implicit-function-declaration")
        .flag_if_supported("-Werror=pointer-arith")

        // disable debug / unused code
        .define("CERTIFYX509_DEBUG", "NO")
        .define("SIMULATION", "NO")

        .define("_X86_", "")

        .define("MANUFACTURER", r#""MSFT""#)
        .define("VENDOR_STRING_1",       r#""TPM ""#)
        .define("VENDOR_STRING_2",       r#""Simu""#)
        .define("VENDOR_STRING_3",       r#""lato""#)
        .define("VENDOR_STRING_4",       r#""r   ""#)
        .define("FIRMWARE_V1", "0x20200312")
        .define("FIRMWARE_V2", "0x00120003")

        .define("NV_MEMORY_SIZE", "0x8000")

        // avoid throwing libtpm.a directly into OUT_DIR for insidious linker
        // order reasons.
        //
        // Without this fix, if you build without `TPM_LIB_DIR`, and then set
        // `TPM_LIB_DIR`, you'll actually end up linking the _old_ libtpm.a from
        // OUT_DIR.
        //
        // This is because `ms-tpm-20-ref-rs` will add OUT_DIR to the linker
        // search path in order to pick up some of those other C dependencies
        // (e.g: RunCommand.c), and since the linker will pick the _first_
        // libfoo.a it encounters, it'll end up using the non-custom one.
        .out_dir(Path::new(&std::env::var("OUT_DIR").unwrap()).join("ms-tpm-20-ref"))
        .compile("tpm");

    Ok(())
}

fn add_deps(
    builder: &mut cc::Build,
    sources: impl AsRef<Path>,
    exclude: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in walkdir::WalkDir::new(sources) {
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }

        if exclude.iter().any(|p| p.as_os_str() == entry.file_name()) {
            continue;
        }

        if entry.file_name().to_string_lossy().ends_with(".c") {
            builder.file(entry.path());
        }
    }

    Ok(())
}

/// Read a environment variable that may / may-not have a target-specific
/// prefix. e.g: `env("FOO")` would first try and read from
/// `X86_64_UNKNOWN_LINUX_GNU_FOO`,  and then fall back to just `FOO`.
// yoinked from openssl-sys/build/main.rs
fn env(name: &str) -> Option<OsString> {
    fn env_inner(name: &str) -> Option<OsString> {
        let var = std::env::var_os(name);
        println!("cargo:rerun-if-env-changed={}", name);

        match var {
            Some(ref v) => println!("{} = {}", name, v.to_string_lossy()),
            None => println!("{} unset", name),
        }

        var
    }

    let prefix = std::env::var("TARGET")
        .unwrap()
        .to_uppercase()
        .replace('-', "_");
    let prefixed = format!("{}_{}", prefix, name);
    env_inner(&prefixed).or_else(|| env_inner(name))
}
