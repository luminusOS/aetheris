use std::ffi::NulError;

#[derive(Debug, thiserror::Error)]
pub enum OpenConnectError {
    #[error("OpenConnect support was built without libopenconnect")]
    NotLinked,
    #[error("{field} contains an interior NUL byte")]
    InteriorNul {
        field: &'static str,
        #[source]
        source: NulError,
    },
    #[error("libopenconnect initialization failed with code {0}")]
    InitFailed(i32),
    #[error("unable to allocate OpenConnect session")]
    AllocationFailed,
    #[error("{operation} failed with code {code}")]
    OperationFailed { operation: &'static str, code: i32 },
}

pub type Result<T> = std::result::Result<T, OpenConnectError>;
