#![deny(clippy::exit)]

use std::path::{Path, PathBuf};

mod openssl;

const MS_TPM_20_REF_SRC_PATH: &str = "./ms-tpm-20-ref/TPMCmd/";

fn add_deps(builder: &mut cc::Build, sources: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in walkdir::WalkDir::new(sources) {
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }

        if entry.file_name().to_string_lossy().ends_with(".c") {
            builder.file(entry.path());
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tpm_src_path = PathBuf::from(MS_TPM_20_REF_SRC_PATH);

    // println!("cargo:rerun-if-changed=build.rs");

    // locate / build openssl
    let ossl_include_dir = openssl::main();

    // build tpm
    let mut builder = cc::Build::new();
    builder.include(ossl_include_dir);

    let includes = [
        tpm_src_path.join("tpm/include"),
        tpm_src_path.join("tpm/include/prototypes"),
        tpm_src_path.join("tpm/include/ossl"),
        tpm_src_path.join("Platform/include"),
        tpm_src_path.join("Platform/include/prototypes"),
    ];

    for path in includes.iter() {
        builder.include(path);
    }

    add_deps(&mut builder, &tpm_src_path.join("tpm"))?;

    if cfg!(feature = "sample_platform") {
        add_deps(&mut builder, &tpm_src_path.join("Platform"))?;
    }

    builder
        .flag_if_supported("-Wno-cast-function-type")
        .flag_if_supported("-Wno-implicit-fallthrough")
        .flag_if_supported("-Wno-missing-field-initializers")
        .define("CERTIFYX509_DEBUG", "NO")
        .define("SIMULATION", "NO")
        .compile("tpm");

    Ok(())
}
