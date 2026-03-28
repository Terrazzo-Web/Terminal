#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LogEvent {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "i"))]
    pub id: u64,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "l"))]
    pub level: LogLevel,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "m"))]
    pub message: String,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "t"))]
    pub timestamp_ms: u64,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "f"))]
    pub file: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LogLevel {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "I"))]
    Info,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "W"))]
    Warn,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "E"))]
    Error,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "D"))]
    Debug,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Debug => "DEBUG",
        }
        .fmt(f)
    }
}
