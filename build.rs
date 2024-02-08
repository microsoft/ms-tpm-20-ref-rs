#![deny(clippy::exit)]

use std::path::{Path, PathBuf};

const MS_TPM_20_REF_SRC_PATH: &str = "./ms-tpm-20-ref/TPMCmd/";

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(feature = "dll_platform") {
        return Ok(());
    }

    // `RunCommand.c` contains setjmp/longjmp code, and must be compiled in
    // separately
    if !(cfg!(feature = "sample_platform") || cfg!(feature = "dll_platform")) {
        cc::Build::new()
            .file("./src/callback_plat/RunCommand.c")
            .compile("run_command");
    }

    println!("cargo:rerun-if-env-changed=TPM_LIB_DIR");

    // users can link against a pre-built `libtpm.a` if they don't want to use
    // the version of `ms-tpm-20-ref` included in-tree
    match std::env::var("TPM_LIB_DIR").ok() {
        Some(var) => {
            println!("cargo:rustc-link-search=native={var}");
            println!("cargo:rustc-link-lib=static=tpm");
            return Ok(());
        }
        None => compile_ms_tpm_20_ref()?,
    }

    Ok(())
}

fn compile_ms_tpm_20_ref() -> Result<(), Box<dyn std::error::Error>> {
    // a little sketchy, but we want to override some of C headers with our own
    // tweaked versions.
    //
    // as such, we have to do a bit of "intrusive modification" to the
    // submodule, and delete the headers we are overriding...

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

    if cfg!(feature = "sample_platform") {
        add_deps(&mut builder, &tpm_src_path.join("Platform"), &[])?;
    }

    #[rustfmt::skip]
    builder
        // suppress warnings that fire _everywhere_ in the TPM codebase
        .flag_if_supported("-Wno-cast-function-type")
        .flag_if_supported("-Wno-ignored-qualifiers")
        // warnings specific to ossl 3.0 stuff
        .flag_if_supported("-Wno-deprecated-declarations")
        // crank up this warning (to catch issues in custom override code)
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
        // the less significant 32-bits of a vendor-specific value
        // indicating the version of the firmware
        // 0x00115400 - original snapshot for rev 1.38
        // 0x00120000 - fix padding check for RSAES_Decode() function (errata 1.11)
        // 0x00120001 - fix missing size parameters when copying out parameters for ECC_Parameters (errata 1.3)
        // 0x00120002 - NV size is determined by platform
        // 0x00120003 - 4k NV Index max size
        .define("FIRMWARE_V2", "0x00120003")

        .define("NV_MEMORY_SIZE", "0x8000")

        // .define("USE_SPEC_COMPLIANT_PROOFS", "NO")
        // .define("SKIP_PROOF_ERRORS", "YES")

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
