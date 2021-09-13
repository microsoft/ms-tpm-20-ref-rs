use ms_tpm_20_ref::{DynResult, InitKind, MsTpm20RefPlatform, PlatformCallbacks};
use std::convert::TryInto;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};

/// Sample platform callback implementation that simply logs invocations +
/// returns dummy data.
pub struct TestPlatformCallbacks {
    file: fs::File,
}

impl PlatformCallbacks for TestPlatformCallbacks {
    fn commit_nv_state(&mut self, state: &[u8]) -> DynResult<()> {
        log::info!("committing nv state with len {}", state.len());
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(state)?;
        Ok(())
    }

    fn get_crypt_random(&mut self, buf: &mut [u8]) -> DynResult<usize> {
        log::info!("returning dummy entropy into buf of len {}", buf.len());

        if let Some(b) = buf.get_mut(0) {
            *b = 1;
        }

        Ok(buf.len())
    }

    fn get_unique_value(&self) -> &'static [u8] {
        log::info!("fetching unique value from platform");
        b"This is not really a unique value. A real unique value should be generated by the platform.\0"
    }
}

const USAGE: &str = r#"
usage: test-harness <.nvmem file>
"#;

fn extract_res(res: &[u8]) -> (u16, u32, String) {
    let tag = u16::from_be_bytes(res[0..2].try_into().unwrap());
    let size = u32::from_be_bytes(res[2..6].try_into().unwrap());
    let code = u32::from_be_bytes(res[6..10].try_into().unwrap());
    (
        tag,
        code,
        // format for use with Ron Aigner's TPM 2.0 Parser app on the Windows Store
        res[..size as usize]
            .iter()
            .map(|b| format!("{:02x?}", b))
            .collect::<String>(),
    )
}

fn main() -> DynResult<()> {
    pretty_env_logger::init();

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

    let mut platform =
        MsTpm20RefPlatform::initialize(Box::new(TestPlatformCallbacks { file }), init_kind)
            .unwrap();

    let mut res = vec![0; 4096];

    if false {
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
    }

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