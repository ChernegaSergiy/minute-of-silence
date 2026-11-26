//! Configuration and validation errors.

/// Configuration import, export, and validation errors.
#[derive(Debug)]
pub enum ConfigError {
    /// JSON serialization or deserialization error.
    #[cfg(feature = "std")]
    Json(serde_json::Error),
    /// Schema version is newer than supported.
    UnsupportedVersion { found: u32, max_supported: u32 },
    /// Schema version is zero or invalid.
    InvalidVersion(u32),
    /// A configuration field value is outside its valid range.
    InvalidField {
        field: &'static str,
        message: &'static str,
    },
}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            #[cfg(feature = "std")]
            Self::Json(e) => write!(f, "JSON error: {e}"),
            Self::UnsupportedVersion {
                found,
                max_supported,
            } => {
                write!(
                    f,
                    "Unsupported configuration schema version: {found} (maximum supported is {max_supported})"
                )
            }
            Self::InvalidVersion(v) => write!(f, "Invalid schema version: {v} (must be >= 1)"),
            Self::InvalidField { field, message } => {
                write!(f, "Invalid field '{field}': {message}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(feature = "std")]
impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}
