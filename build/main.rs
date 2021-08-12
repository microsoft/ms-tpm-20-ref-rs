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

    #[rustfmt::skip]
    builder
        // suppress warning
        .flag_if_supported("-Wno-cast-function-type")
        .flag_if_supported("-Wno-implicit-fallthrough")
        .flag_if_supported("-Wno-missing-field-initializers")
        .flag_if_supported("-Wno-parentheses")
        // disable debug / unused code
        .define("CERTIFYX509_DEBUG", "NO")
        .define("SIMULATION", "NO")
        // Table 0:2 - Defines for Implemented Algorithms (ImplementedDefines)
        .define("ALG_RSA",               "ALG_YES")
        .define("ALG_SHA1",              "ALG_YES")
        .define("ALG_HMAC",              "ALG_YES")
        .define("ALG_TDES",              "ALG_NO")
        .define("ALG_AES",               "ALG_YES")
        .define("ALG_MGF1",              "ALG_YES")
        .define("ALG_XOR",               "ALG_YES")
        .define("ALG_KEYEDHASH",         "ALG_YES")
        .define("ALG_SHA256",            "ALG_YES")
        .define("ALG_SHA384",            "ALG_YES")
        .define("ALG_SHA512",            "ALG_NO")
        .define("ALG_SM3_256",           "ALG_NO")
        .define("ALG_SM4",               "ALG_NO")
        .define("ALG_RSASSA",            "ALG_YES")
        .define("ALG_RSAES",             "ALG_YES")
        .define("ALG_RSAPSS",            "ALG_YES")
        .define("ALG_OAEP",              "ALG_YES")
        .define("ALG_ECC",               "ALG_YES")
        .define("ALG_ECDH",              "ALG_YES")
        .define("ALG_ECDSA",             "ALG_YES")
        .define("ALG_ECDAA",             "ALG_YES")
        .define("ALG_SM2",               "ALG_NO")
        .define("ALG_ECSCHNORR",         "ALG_YES")
        .define("ALG_ECMQV",             "ALG_NO")
        .define("ALG_SYMCIPHER",         "ALG_YES")
        .define("ALG_KDF1_SP800_56A",    "ALG_YES")
        .define("ALG_KDF2",              "ALG_NO")
        .define("ALG_KDF1_SP800_108",    "ALG_YES")
        .define("ALG_CTR",               "ALG_YES")
        .define("ALG_OFB",               "ALG_YES")
        .define("ALG_CBC",               "ALG_YES")
        .define("ALG_CFB",               "ALG_YES")
        .define("ALG_ECB",               "ALG_YES")
        .define("ALG_CAMELLIA",          "ALG_NO")
        // Table 0:4 - Defines for Implemented Curves (CurveTableProcessing)
        .define("ECC_NIST_P192",         "NO")
        .define("ECC_NIST_P224",         "YES")
        .define("ECC_NIST_P256",         "YES")
        .define("ECC_NIST_P384",         "YES")
        .define("ECC_NIST_P521",         "NO")
        .define("ECC_BN_P256",           "YES")
        .define("ECC_BN_P638",           "NO")
        .define("ECC_SM2_P256",          "NO")
        // disable unsupported
        .define("CC_ACT_SetTimeout",                   "CC_NO")
        .define("CC_AC_GetCapability",                 "CC_NO")
        .define("CC_AC_Send",                          "CC_NO")
        // Table 0:7 - Defines for Implementation Values (DefinesTable)
        .define("FIELD_UPGRADE_IMPLEMENTED",      "NO")
        // TODO: figure out how to override RADIX_BITS
        // .define("RADIX_BITS",                     "32")
        // #if defined(_X86_)
        // .define("HASH_ALIGNMENT",                 "4")
        // .define("SYMMETRIC_ALIGNMENT",            "4")
        // #elif defined(_ARM_)
        // #define  HASH_ALIGNMENT                 8
        // #define  SYMMETRIC_ALIGNMENT            8
        // #elif defined(_AMD64_) || defined(_ARM64_)
        // #undef RADIX_BITS
        // #define  RADIX_BITS                     64
        // #define  HASH_ALIGNMENT                 16
        // #define  SYMMETRIC_ALIGNMENT            16
        // #else
        // #error "Unexpected architecture"
        // #endif
        .define("HASH_LIB", "Ossl")
        .define("SYM_LIB", "Ossl")
        .define("MATH_LIB", "Ossl")

        .define("NV_MEMORY_SIZE", "0x8000")

        .define("BSIZE",                          "UINT16")
        .define("IMPLEMENTATION_PCR",             "24")
        .define("PLATFORM_PCR",                   "24")
        .define("DRTM_PCR",                       "17")
        .define("HCRTM_PCR",                      "0")
        .define("NUM_LOCALITIES",                 "5")
        .define("MAX_HANDLE_NUM",                 "3")
        .define("MAX_ACTIVE_SESSIONS",            "64")
        .define("CONTEXT_SLOT",                   "UINT8") // is this right?
        .define("CONTEXT_COUNTER",                "UINT64")
        .define("MAX_LOADED_SESSIONS",            "3")
        .define("MAX_SESSION_NUM",                "3")
        .define("MAX_LOADED_OBJECTS",             "3")
        .define("MIN_EVICT_OBJECTS",              "2")
        .define("NUM_POLICY_PCR_GROUP",           "1")
        .define("NUM_AUTHVALUE_PCR_GROUP",        "1")
        .define("MAX_CONTEXT_SIZE",               "2474")
        .define("MAX_DIGEST_BUFFER",              "1024")
        .define("MAX_NV_INDEX_SIZE",              "4096")
        .define("MAX_NV_BUFFER_SIZE",             "1024")
        .define("MAX_CAP_BUFFER",                 "1024")
        .define("MIN_COUNTER_INDICES",            "8")
        .define("NUM_STATIC_PCR",                 "16")
        .define("MAX_ALG_LIST_SIZE",              "64")
        .define("PRIMARY_SEED_SIZE",              "32")
        .define("CONTEXT_ENCRYPT_ALGORITHM",      "AES")
        .define("NV_CLOCK_UPDATE_INTERVAL",       "12")
        .define("NUM_POLICY_PCR",                 "1")
        .define("MAX_COMMAND_SIZE",               "4096")
        .define("MAX_RESPONSE_SIZE",              "4096")
        .define("ORDERLY_BITS",                   "8")
        .define("MAX_SYM_DATA",                   "128")
        .define("MAX_RNG_ENTROPY_SIZE",           "64")
        .define("RAM_INDEX_SPACE",                "512")
        .define("RSA_DEFAULT_PUBLIC_EXPONENT",    "0x00010001")
        .define("ENABLE_PCR_NO_INCREMENT",        "YES")
        .define("CRT_FORMAT_RSA",                 "YES")
        .define("VENDOR_COMMAND_COUNT",           "0")
        .define("MAX_VENDOR_BUFFER_SIZE",         "1024")

        .define("USE_SPEC_COMPLIANT_PROOFS", "NO")
        .define("SKIP_PROOF_ERRORS", "YES")

        .define("RH_ACT_0", "NO")
        .define("RH_ACT_A", "NO")
        .compile("tpm");

    // build openssl
    openssl::main();

    Ok(())
}
