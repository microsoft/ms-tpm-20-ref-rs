use std::fmt;

#[derive(Debug)]
pub enum Error {
    AlreadyInitialized,
    PlatformCallback(Box<dyn std::error::Error + Send + Sync>),
    Ffi {
        function: &'static str,
        error: i32,
    },
    InvalidRequestSize,
    InvalidResponseSize,
    #[cfg(not(feature = "sample_platform"))]
    NvMem(crate::callback_plat::api::nvmem::NvError),
}

pub type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Error::*;
        match self {
            AlreadyInitialized => write!(f, "platform is already initialized"),
            PlatformCallback(e) => write!(f, "error when calling platform callback: {}", e),
            Ffi { function, error } => {
                write!(f, "error calling C API: {} returned {}", function, error)
            }

            InvalidRequestSize => write!(
                f,
                "mismatch between request buffer size and command header size"
            ),
            InvalidResponseSize => write!(
                f,
                "mismatch between response buffer size and reply header size"
            ),
            #[cfg(not(feature = "sample_platform"))]
            NvMem(e) => write!(f, "nvmem error: {:?}", e),
        }
    }
}

impl std::error::Error for Error {}
