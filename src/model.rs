//! Initial domain types. Extend using docs/ARCHITECTURE.md and REPORT_FORMAT.md.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SweepStatus {
    Valid,
    Partial,
    NonMonotonic,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionStatus {
    Resolved,
    Unresolved,
    Unobserved,
    Invalidated,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    Nominal,
    Endpoint,
    BestFit,
}

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub record: u64,
    pub input_v: f64,
    pub code: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Bracket {
    pub lower_v: f64,
    pub upper_v: f64,
    pub lower_record: u64,
    pub upper_record: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transition {
    pub k: usize,
    pub status: TransitionStatus,
    pub bracket: Option<Bracket>,
    pub estimate_v: Option<f64>,
    pub half_bracket_v: Option<f64>,
}
