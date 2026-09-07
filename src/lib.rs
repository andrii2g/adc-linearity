//! Static ADC linearity audit library.
#![forbid(unsafe_code)]

use std::fmt;

#[derive(Debug)]
pub enum AuditError {
    Validation(String),
    Io(std::io::Error),
    Csv(csv::Error),
    Json(serde_json::Error),
}
impl AuditError {
    pub fn validation(message: impl Into<String>) -> Self { Self::Validation(message.into()) }
    pub fn exit_code(&self) -> i32 { if matches!(self, Self::Validation(_)) { 2 } else { 1 } }
}
impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(s) => f.write_str(s),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Csv(e) => write!(f, "CSV error: {e}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
        }
    }
}
impl std::error::Error for AuditError {}
impl From<std::io::Error> for AuditError { fn from(e: std::io::Error) -> Self { Self::Io(e) } }
impl From<csv::Error> for AuditError { fn from(e: csv::Error) -> Self { Self::Csv(e) } }
impl From<serde_json::Error> for AuditError { fn from(e: serde_json::Error) -> Self { Self::Json(e) } }

pub mod analysis;
pub mod cli;
pub mod config;
pub mod input;
pub mod model;
pub mod report;
pub mod svg;
pub mod synthetic;
pub mod transitions;
