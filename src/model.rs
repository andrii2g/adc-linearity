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
impl ReferenceKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::Endpoint => "endpoint",
            Self::BestFit => "best_fit",
        }
    }
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
    pub nominal_error_lower_lsb: Option<f64>,
    pub nominal_error_upper_lsb: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeRow {
    pub code: usize,
    pub samples: u64,
    pub observation_status: String,
    pub width_status: String,
    pub width_v: Option<f64>,
    pub width_lower_v: Option<f64>,
    pub width_upper_v: Option<f64>,
    pub nominal_dnl_lsb: Option<f64>,
    pub nominal_dnl_lower_lsb: Option<f64>,
    pub nominal_dnl_upper_lsb: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiagnosticEvent {
    pub previous_record: u64,
    pub current_record: u64,
    pub previous_v: f64,
    pub current_v: f64,
    pub previous_code: usize,
    pub current_code: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostics {
    pub jump_count: u64,
    pub reversal_count: u64,
    pub jumps_omitted: u64,
    pub reversals_omitted: u64,
    pub jumps: Vec<DiagnosticEvent>,
    pub reversals: Vec<DiagnosticEvent>,
    pub missing_code_candidates: Vec<usize>,
    pub untested_codes: Vec<usize>,
    pub candidates_interpretable: bool,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coverage {
    pub observed_code_count: usize,
    pub min_observed_code: Option<usize>,
    pub max_observed_code: Option<usize>,
    pub resolved_transition_count: usize,
    pub covered_transition_count: usize,
    pub expected_transition_count: usize,
    pub resolved_width_count: usize,
    pub expected_width_count: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quality {
    pub step_count: u64,
    pub step_min_v: f64,
    pub step_mean_v: f64,
    pub step_max_v: f64,
    pub step_min_lsb: f64,
    pub step_mean_lsb: f64,
    pub step_max_lsb: f64,
    pub max_bracket_v: Option<f64>,
    pub max_bracket_lsb: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportConfig {
    pub bits: u8,
    pub vmin_v: f64,
    pub vmax_v: f64,
    pub levels: usize,
    pub nominal_lsb_v: f64,
    pub quantizer: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Calibration {
    pub availability: String,
    pub reason: Option<String>,
    pub basis: String,
    pub gain_convention: String,
    pub offset_v: Option<f64>,
    pub offset_lsb: Option<f64>,
    pub gain_span_error_v: Option<f64>,
    pub gain_span_error_lsb: Option<f64>,
    pub gain_span_error_percent: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReferenceLine {
    pub a_v: f64,
    pub b_v_per_code: f64,
    pub anchor_k: f64,
    pub anchor_v: f64,
    pub intercept_shift_v: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefCoverage {
    pub scope: String,
    pub evaluated_transition_count: usize,
    pub evaluated_width_count: usize,
    pub expected_transition_count: usize,
    pub expected_width_count: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionMetric {
    pub k: usize,
    pub inl_lsb: Option<f64>,
    pub cumulative_inl_lsb: Option<f64>,
    pub cumulative_run_id: Option<u32>,
    pub cumulative_run_start: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeMetric {
    pub code: usize,
    pub dnl_lsb: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnlSummary {
    pub count: usize,
    pub min_lsb: f64,
    pub min_code: usize,
    pub max_lsb: f64,
    pub max_code: usize,
    pub max_abs_lsb: f64,
    pub max_abs_code: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InlSummary {
    pub count: usize,
    pub min_lsb: f64,
    pub min_k: usize,
    pub max_lsb: f64,
    pub max_k: usize,
    pub max_abs_lsb: f64,
    pub max_abs_k: usize,
    pub rms_lsb: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReferenceResult {
    pub name: String,
    pub availability: String,
    pub reason: Option<String>,
    pub line: Option<ReferenceLine>,
    pub lsb_basis: String,
    pub display_quantity: String,
    pub coverage: RefCoverage,
    pub dnl_summary: Option<DnlSummary>,
    pub inl_summary: Option<InlSummary>,
    pub transition_metrics: Vec<TransitionMetric>,
    pub code_metrics: Vec<CodeMetric>,
}
#[derive(Clone, Debug)]
pub struct PlotPoint {
    pub record: u64,
    pub input_v: f64,
    pub code: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditReport {
    pub schema_version: u32,
    pub source: String,
    pub config: ReportConfig,
    pub status: SweepStatus,
    pub sample_count: u64,
    pub coverage: Coverage,
    pub quality: Quality,
    pub calibration: Calibration,
    pub references: Vec<ReferenceResult>,
    pub transitions: Vec<Transition>,
    pub codes: Vec<CodeRow>,
    pub diagnostics: Diagnostics,
    pub assumptions: Vec<String>,
    #[serde(skip)]
    pub plot_points: Vec<PlotPoint>,
}
