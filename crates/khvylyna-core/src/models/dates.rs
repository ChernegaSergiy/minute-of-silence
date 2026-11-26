//! Remembrance date representation and validation.

use alloc::string::String;
use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// A personal remembrance date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "camelCase")]
pub struct PersonalDate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub month: u8,
    pub day: u8,
    pub year: i32,
    pub label: String,
}

impl PersonalDate {
    /// Validates month (1-12), day (1-31), and non-empty label.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.month < 1 || self.month > 12 {
            return Err(ConfigError::InvalidField {
                field: "month",
                message: "month must be between 1 and 12",
            });
        }
        let max_day = match self.month {
            2 => 29,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        if self.day < 1 || self.day > max_day {
            return Err(ConfigError::InvalidField {
                field: "day",
                message: "day must be between 1 and 31",
            });
        }
        if self.label.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                field: "label",
                message: "label cannot be empty",
            });
        }
        Ok(())
    }
}
