#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenConnectEvent {
    Progress {
        level: ProgressLevel,
        message: String,
    },
    CertificateValidationRequired {
        reason: String,
    },
    AuthenticationFormRequired,
    Connected,
    Disconnected,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressLevel {
    Error,
    Info,
    Debug,
    Trace,
    Unknown(i32),
}

impl From<i32> for ProgressLevel {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Error,
            1 => Self::Info,
            2 => Self::Debug,
            3 => Self::Trace,
            other => Self::Unknown(other),
        }
    }
}
