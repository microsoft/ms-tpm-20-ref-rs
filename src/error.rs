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
    /// Nvmem callback error
    #[cfg(not(feature = "sample_platform"))]
    NvMem(crate::callback_plat::api::nvmem::NvError),
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
