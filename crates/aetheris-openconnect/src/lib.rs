mod error;
pub mod events;
mod session;

#[cfg(feature = "system-libopenconnect")]
mod ffi;

pub use error::{OpenConnectError, Result};
pub use events::{OpenConnectEvent, ProgressLevel};
pub use session::{LogLevel, OpenConnectConfig, OpenConnectSession};

pub fn is_linked() -> bool {
    cfg!(feature = "system-libopenconnect")
}

pub fn library_version() -> Result<Option<String>> {
    session::library_version()
}
