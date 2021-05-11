use openssl_src;

use super::FoundUsing;

pub fn get_openssl(_target: &str) -> FoundUsing {
    let artifacts = openssl_src::Build::new().build();
    println!("cargo:vendored=1");
    println!(
        "cargo:root={}",
        artifacts.lib_dir().parent().unwrap().display()
    );

    FoundUsing::Paths {
        lib_dir: artifacts.lib_dir().to_path_buf(),
        include_dir: artifacts.include_dir().to_path_buf(),
    }
}
