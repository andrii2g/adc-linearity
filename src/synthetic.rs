//! Deterministic threshold-model synthesis and independent truth.
use crate::{
    config::{AuditConfig, DerivedConfig},
    model::{ReferenceLine, ReportConfig, Sample},
    AuditError,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SynthModel {
    Perfect,
    Bow,
    Periodic,
    MissingCode,
}
impl SynthModel {
    pub fn name(self) -> &'static str {
        match self {
            Self::Perfect => "perfect",
            Self::Bow => "bow",
            Self::Periodic => "periodic",
            Self::MissingCode => "missing-code",
        }
    }
}
#[derive(Clone, Debug)]
pub struct SynthConfig {
    pub audit: AuditConfig,
    pub model: SynthModel,
    pub samples_per_lsb: u32,
    pub amplitude_lsb: Option<f64>,
    pub periods: Option<u32>,
    pub missing_code: Option<usize>,
    pub offset_lsb: f64,
    pub span_error_percent: f64,
}
#[derive(Clone, Debug)]
pub struct SyntheticData {
    pub config: DerivedConfig,
    pub request: SynthConfig,
    pub thresholds: Vec<f64>,
    pub start_v: f64,
    pub end_v: f64,
    pub intervals: u64,
    pub true_missing_codes: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Truth {
    pub schema_version: u32,
    pub kind: String,
    pub generation: String,
    pub config: ReportConfig,
    pub model: String,
    pub parameters: serde_json::Value,
    pub sample_count: u64,
    pub sample_start_v: f64,
    pub sample_end_v: f64,
    pub max_nominal_step_v: f64,
    pub thresholds_v: Vec<f64>,
    pub interior_widths_v: Vec<f64>,
    pub true_missing_codes: Vec<usize>,
    pub nominal_dnl_lsb: Vec<f64>,
    pub endpoint_inl_lsb: Vec<f64>,
    pub endpoint_line: ReferenceLine,
    pub expected_calibration: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<String>,
}
pub fn make(config: SynthConfig) -> Result<SyntheticData, AuditError> {
    let cfg = config.audit.validate()?;
    if config.samples_per_lsb == 0 || config.samples_per_lsb > 1024 {
        return Err(AuditError::validation(
            "samples-per-lsb must be in 1..=1024",
        ));
    }
    if !config.offset_lsb.is_finite() || !config.span_error_percent.is_finite() {
        return Err(AuditError::validation("affine parameters must be finite"));
    }
    let scale = 1.0 + config.span_error_percent / 100.0;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(AuditError::validation(
            "span-error-percent must leave a positive finite scale",
        ));
    }
    let amp = config.amplitude_lsb.unwrap_or(match config.model {
        SynthModel::Bow => 3.0,
        SynthModel::Periodic => 0.4,
        _ => 0.0,
    });
    if !amp.is_finite() || amp < 0.0 {
        return Err(AuditError::validation(
            "amplitude-lsb must be finite and nonnegative",
        ));
    }
    let periods = config.periods.unwrap_or(8);
    if periods == 0 {
        return Err(AuditError::validation("periods must be positive"));
    }
    let missing = config.missing_code.unwrap_or(cfg.levels / 2);
    if config.model == SynthModel::MissingCode && !(1..=cfg.levels - 2).contains(&missing) {
        return Err(AuditError::validation(
            "missing-code must select an interior code",
        ));
    }
    let mut thresholds = Vec::with_capacity(cfg.levels - 1);
    for k in 1..cfg.levels {
        let u = (k - 1) as f64 / (cfg.levels - 2) as f64;
        let displacement = match config.model {
            SynthModel::Perfect | SynthModel::MissingCode => 0.0,
            SynthModel::Bow => amp * cfg.nominal_lsb_v * 4.0 * u * (1.0 - u),
            SynthModel::Periodic => {
                if k == 1 || k == cfg.levels - 1 {
                    0.0
                } else {
                    amp * cfg.nominal_lsb_v
                        * (2.0 * std::f64::consts::PI * periods as f64 * u).sin()
                }
            }
        };
        let base = cfg.source.vmin_v + k as f64 * cfg.nominal_lsb_v + displacement;
        thresholds.push(
            cfg.source.vmin_v
                + config.offset_lsb * cfg.nominal_lsb_v
                + scale * (base - cfg.source.vmin_v),
        );
    }
    if config.model == SynthModel::MissingCode {
        thresholds[missing] = thresholds[missing - 1]
    }
    for (i, &t) in thresholds.iter().enumerate() {
        if !t.is_finite() {
            return Err(AuditError::validation("synthetic threshold is non-finite"));
        }
        if i > 0 {
            let equal = config.model == SynthModel::MissingCode && i == missing;
            if t < thresholds[i - 1] || (!equal && t == thresholds[i - 1]) {
                return Err(AuditError::validation(
                    "synthetic thresholds are not ordered; reduce amplitude",
                ));
            }
        }
    }
    let start_v = cfg.source.vmin_v.min(thresholds[0] - cfg.nominal_lsb_v);
    let end_v = cfg
        .source
        .vmax_v
        .max(thresholds[cfg.levels - 2] + cfg.nominal_lsb_v);
    let span = end_v - start_v;
    let h = cfg.nominal_lsb_v / config.samples_per_lsb as f64;
    let intervals = (span / h).ceil() as u64;
    if intervals < 1 || intervals.checked_add(1).is_none_or(|n| n > 5_000_000) {
        return Err(AuditError::validation(
            "synthetic sample count exceeds 5,000,000",
        ));
    }
    let at = |i: u64| {
        if i == 0 {
            start_v
        } else if i == intervals {
            end_v
        } else {
            start_v + (end_v - start_v) * (i as f64 / intervals as f64)
        }
    };
    let mut previous = at(0);
    for i in 1..=intervals {
        let current = at(i);
        if !current.is_finite() || current <= previous {
            return Err(AuditError::validation(
                "synthetic sample grid is not strictly representable in f64",
            ));
        }
        previous = current;
    }
    let true_missing_codes = if config.model == SynthModel::MissingCode {
        vec![missing]
    } else {
        vec![]
    };
    Ok(SyntheticData {
        config: cfg,
        request: config,
        thresholds,
        start_v,
        end_v,
        intervals,
        true_missing_codes,
    })
}
impl SyntheticData {
    pub fn sample_count(&self) -> u64 {
        self.intervals + 1
    }
    pub fn sample(&self, i: u64) -> Result<Sample, AuditError> {
        let x = if i == 0 {
            self.start_v
        } else if i == self.intervals {
            self.end_v
        } else {
            self.start_v + (self.end_v - self.start_v) * (i as f64 / self.intervals as f64)
        };
        if !x.is_finite() {
            return Err(AuditError::validation("synthetic sample is non-finite"));
        }
        let code = self.thresholds.partition_point(|&t| t <= x);
        Ok(Sample {
            record: i + 2,
            input_v: x,
            code,
        })
    }
    pub fn truth(&self, decimation: Option<u64>) -> Truth {
        let q = self.config.nominal_lsb_v;
        let widths: Vec<_> = self.thresholds.windows(2).map(|w| w[1] - w[0]).collect();
        let first = self.thresholds[0];
        let last = self.thresholds[self.thresholds.len() - 1];
        let b = (last - first) / (self.config.levels - 2) as f64;
        let a = first - b;
        let endpoint_line = ReferenceLine {
            a_v: a,
            b_v_per_code: b,
            anchor_k: 1.0,
            anchor_v: first,
            intercept_shift_v: a - self.config.source.vmin_v,
        };
        let endpoint_inl_lsb = self
            .thresholds
            .iter()
            .enumerate()
            .map(|(i, &t)| (t - (first + b * i as f64)) / b)
            .collect();
        let ideal = (self.config.levels - 2) as f64 * q;
        let measured = last - first;
        let offset = first - (self.config.source.vmin_v + q);
        let retained = decimation
            .map(|d| {
                let mut n = self.intervals / d + 1;
                if !self.intervals.is_multiple_of(d) {
                    n += 1
                }
                n
            })
            .unwrap_or_else(|| self.sample_count());
        let effective_amplitude = match self.request.model {
            SynthModel::Bow => Some(self.request.amplitude_lsb.unwrap_or(3.0)),
            SynthModel::Periodic => Some(self.request.amplitude_lsb.unwrap_or(0.4)),
            _ => None,
        };
        let effective_periods =
            (self.request.model == SynthModel::Periodic).then(|| self.request.periods.unwrap_or(8));
        let effective_missing = (self.request.model == SynthModel::MissingCode)
            .then(|| self.request.missing_code.unwrap_or(self.config.levels / 2));
        Truth {
            schema_version: 1,
            kind: "synthetic_truth".into(),
            generation: "threshold_model".into(),
            config: ReportConfig {
                bits: self.config.source.bits,
                vmin_v: self.config.source.vmin_v,
                vmax_v: self.config.source.vmax_v,
                levels: self.config.levels,
                nominal_lsb_v: q,
                quantizer: "floor".into(),
            },
            model: self.request.model.name().into(),
            parameters: json!({"amplitude_lsb":effective_amplitude,"periods":effective_periods,"missing_code":effective_missing,"offset_lsb":self.request.offset_lsb,"span_error_percent":self.request.span_error_percent}),
            sample_count: retained,
            sample_start_v: self.start_v,
            sample_end_v: self.end_v,
            max_nominal_step_v: if let Some(d) = decimation {
                d.min(self.intervals) as f64 * (self.end_v - self.start_v) / self.intervals as f64
            } else {
                (self.end_v - self.start_v) / self.intervals as f64
            },
            thresholds_v: self.thresholds.clone(),
            interior_widths_v: widths.clone(),
            true_missing_codes: self.true_missing_codes.clone(),
            nominal_dnl_lsb: widths.into_iter().map(|w| w / q - 1.0).collect(),
            endpoint_inl_lsb,
            endpoint_line,
            expected_calibration: json!({"offset_v":offset,"offset_lsb":offset/q,"gain_span_error_v":measured-ideal,"gain_span_error_lsb":(measured-ideal)/q,"gain_span_error_percent":100.0*(measured/ideal-1.0)}),
            sampling: decimation.map(|d| format!("every_{d}th_plus_final")),
        }
    }
}
