#![deny(clippy::exit)]

use std::path::{Path, PathBuf};

mod openssl;

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

    // get this out of the way real quick...
    if !(cfg!(feature = "sample_platform") || cfg!(feature = "dll_platform")) {
        cc::Build::new()
            .file("./src/callback_plat/RunCommand.c")
            .compile("run_command");
    }

    // build tpm

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

    // NOTE: calling ossl::main() first won't work, as it will println!
    // `cargo:rustc-link-lib` calls before tpm, which will mess up the link order.
    //
    // Aren't linkers just great?

    let mut builder = cc::Build::new();
    builder.include(openssl::get_include_dir());

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
        // suppress warning
        .flag_if_supported("-Wno-cast-function-type")
        .flag_if_supported("-Wno-implicit-fallthrough")
        .flag_if_supported("-Wno-missing-field-initializers")
        .flag_if_supported("-Wno-parentheses")
        .flag_if_supported("-Wno-ignored-qualifiers")
        .flag_if_supported("-Wno-deprecated-declarations")

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

        .compile("tpm");

    // build openssl
    openssl::main();

    Ok(())
}
