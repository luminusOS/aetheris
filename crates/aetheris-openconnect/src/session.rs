#[cfg(any(feature = "system-libopenconnect", test))]
use std::ffi::CString;

use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenConnectConfig {
    pub server_url: String,
    pub protocol: Option<String>,
    pub ca_file: Option<String>,
    pub user_agent: Option<String>,
    pub log_level: LogLevel,
    pub use_system_trust: bool,
    pub disable_ipv6: bool,
    pub disable_dtls: bool,
}

impl OpenConnectConfig {
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            protocol: None,
            ca_file: None,
            user_agent: Some(default_user_agent()),
            log_level: LogLevel::Info,
            use_system_trust: true,
            disable_ipv6: false,
            disable_dtls: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    #[cfg(feature = "system-libopenconnect")]
    fn as_openconnect_level(self) -> i32 {
        match self {
            Self::Error => 0,
            Self::Info => 1,
            Self::Debug => 2,
            Self::Trace => 3,
        }
    }
}

pub fn default_user_agent() -> String {
    format!("Aetheris/{}", env!("CARGO_PKG_VERSION"))
}

#[cfg(any(feature = "system-libopenconnect", test))]
fn c_string(field: &'static str, value: impl Into<Vec<u8>>) -> Result<CString> {
    CString::new(value).map_err(|source| crate::OpenConnectError::InteriorNul { field, source })
}

#[cfg(feature = "system-libopenconnect")]
mod linked {
    use std::{
        ffi::{CStr, CString},
        ptr,
        sync::OnceLock,
    };

    use super::{OpenConnectConfig, c_string};
    use crate::{OpenConnectError, Result, ffi};

    static INIT_RESULT: OnceLock<i32> = OnceLock::new();

    pub struct OpenConnectSession {
        vpninfo: *mut ffi::OpenconnectInfo,
        strings: Vec<CString>,
    }

    impl OpenConnectSession {
        pub fn new(config: OpenConnectConfig) -> Result<Self> {
            initialize_ssl()?;

            let mut strings = Vec::new();
            let user_agent = c_string(
                "user_agent",
                config
                    .user_agent
                    .clone()
                    .unwrap_or_else(super::default_user_agent),
            )?;

            let vpninfo = unsafe {
                // SAFETY: libopenconnect receives a valid, NUL-terminated user-agent string.
                // All callbacks are NULL for this initial non-interactive session wrapper.
                ffi::openconnect_vpninfo_new(
                    user_agent.as_ptr(),
                    None,
                    None,
                    None,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            if vpninfo.is_null() {
                return Err(OpenConnectError::AllocationFailed);
            }
            strings.push(user_agent);

            let mut session = Self { vpninfo, strings };
            session.configure(config)?;
            Ok(session)
        }

        pub fn protocol(&self) -> Option<String> {
            unsafe {
                // SAFETY: vpninfo is owned by self and valid until Drop.
                optional_string(ffi::openconnect_get_protocol(self.vpninfo))
            }
        }

        pub fn connect_url(&self) -> Option<String> {
            unsafe {
                // SAFETY: vpninfo is owned by self and valid until Drop.
                optional_string(ffi::openconnect_get_connect_url(self.vpninfo))
            }
        }

        fn configure(&mut self, config: OpenConnectConfig) -> Result<()> {
            self.set_log_level(config.log_level);
            self.set_system_trust(config.use_system_trust);

            let server_url = c_string("server_url", config.server_url)?;
            let code = unsafe {
                // SAFETY: vpninfo is valid and server_url is a valid C string.
                ffi::openconnect_parse_url(self.vpninfo, server_url.as_ptr())
            };
            self.strings.push(server_url);
            check_code("parse OpenConnect URL", code)?;

            if let Some(protocol) = config.protocol {
                let protocol = c_string("protocol", protocol)?;
                let code = unsafe {
                    // SAFETY: vpninfo is valid and protocol is a valid C string.
                    ffi::openconnect_set_protocol(self.vpninfo, protocol.as_ptr())
                };
                self.strings.push(protocol);
                check_code("set OpenConnect protocol", code)?;
            }

            if let Some(ca_file) = config.ca_file {
                let ca_file = c_string("ca_file", ca_file)?;
                let code = unsafe {
                    // SAFETY: vpninfo is valid and ca_file is a valid C string.
                    ffi::openconnect_set_cafile(self.vpninfo, ca_file.as_ptr())
                };
                self.strings.push(ca_file);
                check_code("set OpenConnect CA file", code)?;
            }

            if config.disable_ipv6 {
                let code = unsafe {
                    // SAFETY: vpninfo is valid for the lifetime of self.
                    ffi::openconnect_disable_ipv6(self.vpninfo)
                };
                check_code("disable OpenConnect IPv6", code)?;
            }

            if config.disable_dtls {
                let code = unsafe {
                    // SAFETY: vpninfo is valid for the lifetime of self.
                    ffi::openconnect_disable_dtls(self.vpninfo)
                };
                check_code("disable OpenConnect DTLS", code)?;
            }

            Ok(())
        }

        fn set_log_level(&self, log_level: super::LogLevel) {
            unsafe {
                // SAFETY: vpninfo is valid for the lifetime of self.
                ffi::openconnect_set_loglevel(self.vpninfo, log_level.as_openconnect_level());
            }
        }

        fn set_system_trust(&self, enabled: bool) {
            unsafe {
                // SAFETY: vpninfo is valid for the lifetime of self.
                ffi::openconnect_set_system_trust(self.vpninfo, u32::from(enabled));
            }
        }
    }

    impl Drop for OpenConnectSession {
        fn drop(&mut self) {
            unsafe {
                // SAFETY: vpninfo was returned by openconnect_vpninfo_new and is freed once here.
                ffi::openconnect_vpninfo_free(self.vpninfo);
            }
        }
    }

    pub fn library_version() -> Result<Option<String>> {
        initialize_ssl()?;
        unsafe {
            // SAFETY: openconnect_get_version returns a static string or NULL.
            Ok(optional_string(ffi::openconnect_get_version()))
        }
    }

    fn initialize_ssl() -> Result<()> {
        let code = *INIT_RESULT.get_or_init(|| unsafe {
            // SAFETY: libopenconnect documents openconnect_init_ssl as process-global init.
            ffi::openconnect_init_ssl()
        });
        if code == 0 {
            Ok(())
        } else {
            Err(OpenConnectError::InitFailed(code))
        }
    }

    fn check_code(operation: &'static str, code: i32) -> Result<()> {
        if code == 0 {
            Ok(())
        } else {
            Err(OpenConnectError::OperationFailed { operation, code })
        }
    }

    unsafe fn optional_string(value: *const std::ffi::c_char) -> Option<String> {
        if value.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(value) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }
}

#[cfg(not(feature = "system-libopenconnect"))]
mod linked {
    use super::OpenConnectConfig;
    use crate::{OpenConnectError, Result};

    #[derive(Debug)]
    pub struct OpenConnectSession;

    impl OpenConnectSession {
        pub fn new(_config: OpenConnectConfig) -> Result<Self> {
            Err(OpenConnectError::NotLinked)
        }

        pub fn protocol(&self) -> Option<String> {
            None
        }

        pub fn connect_url(&self) -> Option<String> {
            None
        }
    }

    pub fn library_version() -> Result<Option<String>> {
        Err(OpenConnectError::NotLinked)
    }
}

pub use linked::OpenConnectSession;

pub fn library_version() -> Result<Option<String>> {
    linked::library_version()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OpenConnectError;

    #[test]
    fn default_config_sets_aetheris_user_agent() {
        let config = OpenConnectConfig::new("https://vpn.example.com");

        assert_eq!(config.server_url, "https://vpn.example.com");
        assert_eq!(
            config.user_agent.as_deref(),
            Some(default_user_agent().as_str())
        );
        assert!(config.use_system_trust);
    }

    #[test]
    fn interior_nul_is_reported_with_field_name() {
        let error = c_string("server_url", b"https://vpn.example.com\0bad".to_vec())
            .expect_err("interior NUL should fail");

        assert!(matches!(
            error,
            OpenConnectError::InteriorNul {
                field: "server_url",
                ..
            }
        ));
    }

    #[cfg(not(feature = "system-libopenconnect"))]
    #[test]
    fn session_reports_not_linked_without_system_feature() {
        let error = OpenConnectSession::new(OpenConnectConfig::new("https://vpn.example.com"))
            .expect_err("session creation should require libopenconnect feature");

        assert!(matches!(error, OpenConnectError::NotLinked));
    }
}
