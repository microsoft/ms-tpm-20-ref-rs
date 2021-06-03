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
    // get this out of the way real quick...
    if !cfg!(feature = "sample_platform") {
        cc::Build::new()
            .file("./src/callback_plat/RunCommand.c")
            .compile("run_command");
    }

    // build tpm
    //
    // NOTE: calling ossl::main() first won't work, as it will println!
    // `cargo:rustc-link-lib` calls before tpm, which will mess up the link order.
    //
    // Aren't linkers just great?

    let mut builder = cc::Build::new();
    builder.include(openssl::get_include_dir());

    let tpm_src_path = PathBuf::from(MS_TPM_20_REF_SRC_PATH);
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

    // build openssl
    openssl::main();

    Ok(())
}
