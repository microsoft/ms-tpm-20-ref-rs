use std::fmt;

/// ms-tpm-20-ref errors
#[derive(Debug)]
pub enum Error {
    /// Platform is already initialized
    AlreadyInitialized,
    /// Error when calling platform callback
    PlatformCallback(Box<dyn std::error::Error + Send + Sync>),
    /// Error calling specified C API
    Ffi {
        /// The C function being called
        function: &'static str,
        /// Returned error code
        error: i32,
    },
    /// Mismatch between request buffer size and command header size
    InvalidRequestSize,
    /// Mismatch between response buffer size and reply header size
    InvalidResponseSize,
    /// Error calling nvmem platform API
    NvMem(crate::plat::api::nvmem::NvError),
    /// Error restoring platform state
    FailedPlatformRestore(postcard::Error),
    /// Invalid saved state size
    InvalidRestoreSize,
    /// Invalid saved state format
    InvalidRestoreFormat,
}

/// Alias for `Result<T, Box<dyn std::error::Error + Send + Sync>>`
pub type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Error::*;
        match self {
            AlreadyInitialized => write!(f, "platform is already initialized"),
            PlatformCallback(e) => write!(f, "error when calling platform callback: {}", e),
            Ffi { function, error } => {
                write!(
                    f,
                    "error calling C API: {} returned {:#x?}",
                    function, error
                )
            }

            InvalidRequestSize => write!(
                f,
                "mismatch between request buffer size and command header size"
            ),
            InvalidResponseSize => write!(
                f,
                "mismatch between response buffer size and reply header size"
            ),
            NvMem(e) => write!(f, "nvmem error: {:?}", e),
            FailedPlatformRestore(e) => write!(f, "failed restore: {}", e),
            InvalidRestoreSize => write!(f, "invalid saved state size"),
            InvalidRestoreFormat => write!(f, "invalid saved state format"),
        }
    }
}

impl std::error::Error for Error {}
