// Copyright (C) Microsoft Corporation. All rights reserved.

//! Sample binary that uses `ms-tpm-20-ref-rs` to initialize a TPM engine, send
//! a few commands to it, and persist state to an on-disk `.nvram` blob.

use ms_tpm_20_ref::DynResult;
use ms_tpm_20_ref::InitKind;
use ms_tpm_20_ref::MsTpm20RefPlatform;
use ms_tpm_20_ref::PlatformCallbacks;
use std::convert::TryInto;
use std::fs;
use std::io::Read;
use std::io::Seek;
use std::io::Write;
use std::time::Instant;

/// Minimal callback implementation, returning fake enropy,
pub struct TestPlatformCallbacks {
    file: fs::File,
    time: Instant,
}

impl PlatformCallbacks for TestPlatformCallbacks {
    fn commit_nv_state(&mut self, state: &[u8]) -> DynResult<()> {
        tracing::info!("committing nv state with len {}", state.len());
        self.file.rewind()?;
        self.file.write_all(state)?;
        Ok(())
    }

    fn get_crypt_random(&mut self, buf: &mut [u8]) -> DynResult<usize> {
        tracing::info!("returning dummy entropy into buf of len {}", buf.len());

        if let Some(b) = buf.get_mut(0) {
            *b = 0xff;
        }

        Ok(buf.len())
    }

    fn monotonic_timer(&mut self) -> std::time::Duration {
        self.time.elapsed()
    }

    fn get_unique_value(&self) -> &'static [u8] {
        tracing::info!("fetching unique value from platform");
        b"somebody once told me the world was gonna roll me, I ain't the sharpest tool in the shed"
    }
}

const USAGE: &str = r#"
usage: test-harness <.nvmem file>
"#;

fn main() -> DynResult<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let file_path = match std::env::args().nth(1) {
        None => {
            eprintln!("{}", USAGE.trim());
            return Ok(());
        }
        Some(file_name) => std::path::PathBuf::from(file_name),
    };

    let is_cold_init = !file_path.exists();

    let mut file = if is_cold_init {
        fs::File::create(file_path)?
    } else {
        fs::OpenOptions::new()
            .write(true)
            .read(true)
            .open(file_path)?
    };

    let init_kind = if is_cold_init {
        InitKind::ColdInit
    } else {
        let mut blob = Vec::new();
        file.read_to_end(&mut blob)?;
        InitKind::ColdInitWithPersistentState {
            nvmem_blob: blob.into(),
        }
    };

    let mut platform = MsTpm20RefPlatform::initialize(
        Box::new(TestPlatformCallbacks {
            file,
            time: Instant::now(),
        }),
        init_kind,
    )?;

    smoke_test_tpm(&mut platform)?;

    Ok(())
}

fn extract_res(res: &[u8]) -> (u16, u32, String) {
    let tag = u16::from_be_bytes(res[0..2].try_into().unwrap());
    let size = u32::from_be_bytes(res[2..6].try_into().unwrap());
    let code = u32::from_be_bytes(res[6..10].try_into().unwrap());

    let mut res_str = String::new();
    for b in &res[..size as usize] {
        res_str.push_str(&format!("{:02x?}", b));
    }

    (tag, code, res_str)
}

/// Sends a few basic commands to ensure basic TPM engine functionality works.
fn smoke_test_tpm(platform: &mut MsTpm20RefPlatform) -> DynResult<()> {
    let mut res = vec![0; 4096];

    // send startup command
    platform.execute_command(
        &mut [
            0x80, 0x01, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x01, 0x44, 0x00, 0x00,
        ],
        &mut res,
    )?;

    eprintln!("startup cmd response: {:x?}", extract_res(&res));

    // send self test command
    platform.execute_command(
        &mut [
            0x80, 0x01, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x01, 0x43, 0x01,
        ],
        &mut res,
    )?;

    eprintln!("self test cmd response: {:x?}", extract_res(&res));

    // quick sanity check
    let state = platform.save_state();
    platform.restore_state(state).unwrap();

    // clear tpm hierarchy control
    platform.execute_command(
        &mut [
            0x80, 0x02, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x01, 0x21, 0x40, 0x00, 0x00, 0x0c,
            0x00, 0x00, 0x00, 0x09, 0x40, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40,
            0x00, 0x00, 0x0c, 0x00,
        ],
        &mut res,
    )?;

    eprintln!(
        "clear tpm hierarchy control cmd response: {:x?}",
        extract_res(&res)
    );
    Ok(())
}
