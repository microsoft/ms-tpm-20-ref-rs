#![deny(clippy::exit)]

use std::path::PathBuf;

mod openssl;

const MS_TPM_20_REF_SRC_PATH: &str = "./ms-tpm-20-ref/TPMCmd/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");

    // locate / build openssl
    let ossl_include_dir = openssl::main();

    // build tpm
    let mut builder = cc::Build::new();
    builder.include(ossl_include_dir);

    let tpm_src_path = PathBuf::from(MS_TPM_20_REF_SRC_PATH);

    builder.include(tpm_src_path.join("tpm/include"));
    builder.include(tpm_src_path.join("tpm/include/prototypes"));
    builder.include(tpm_src_path.join("tpm/include/ossl"));
    builder.include(tpm_src_path.join("Platform/include"));
    builder.include(tpm_src_path.join("Platform/include/prototypes"));
    for entry in walkdir::WalkDir::new(tpm_src_path.join("tpm"))
        .into_iter()
        .chain(walkdir::WalkDir::new(tpm_src_path.join("Platform")))
    {
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }

        if entry.file_name().to_string_lossy().ends_with(".c") {
            builder.file(entry.path());
        }
    }

    builder
        .flag_if_supported("-Wno-cast-function-type")
        .flag_if_supported("-Wno-implicit-fallthrough")
        .flag_if_supported("-Wno-missing-field-initializers")
        .compile("tpm");

    Ok(())
}
