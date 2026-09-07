//! Streaming transition extraction and bounded diagnostics.
use crate::{config::DerivedConfig, model::*, AuditError};

const EVENT_CAP: usize = 1000;
const BUCKETS: usize = 1024;

#[derive(Clone, Debug)]
pub struct Observation {
    pub transitions: Vec<Transition>,
    pub seen: Vec<u64>,
    pub diagnostics: Diagnostics,
    pub coverage: Coverage,
    pub quality: Quality,
    pub status: SweepStatus,
    pub sample_count: u64,
    pub plot_points: Vec<PlotPoint>,
}

pub struct ObservationBuilder<'a> {
    cfg: &'a DerivedConfig,
    previous: Option<Sample>,
    seen: Vec<u64>,
    slots: Vec<Option<(Bracket, bool)>>,
    jumps: Vec<DiagnosticEvent>,
    reversals: Vec<DiagnosticEvent>,
    jump_count: u64,
    reversal_count: u64,
    sample_count: u64,
    step_count: u64,
    step_min: f64,
    step_max: f64,
    step_mean: f64,
    buckets: Vec<Vec<PlotPoint>>,
}
impl<'a> ObservationBuilder<'a> {
    pub fn new(cfg: &'a DerivedConfig) -> Self {
        Self {
            cfg,
            previous: None,
            seen: vec![0; cfg.levels],
            slots: vec![None; cfg.levels - 1],
            jumps: vec![],
            reversals: vec![],
            jump_count: 0,
            reversal_count: 0,
            sample_count: 0,
            step_count: 0,
            step_min: f64::INFINITY,
            step_max: 0.0,
            step_mean: 0.0,
            buckets: (0..BUCKETS).map(|_| Vec::new()).collect(),
        }
    }
    fn event(a: Sample, b: Sample) -> DiagnosticEvent {
        DiagnosticEvent {
            previous_record: a.record,
            current_record: b.record,
            previous_v: a.input_v,
            current_v: b.input_v,
            previous_code: a.code,
            current_code: b.code,
        }
    }
    fn plot(&mut self, s: Sample) {
        let span = self.cfg.source.vmax_v - self.cfg.source.vmin_v;
        let pos = ((s.input_v - self.cfg.source.vmin_v) / span).clamp(0.0, 1.0);
        let idx = ((pos * (BUCKETS - 1) as f64).floor() as usize).min(BUCKETS - 1);
        let p = PlotPoint {
            record: s.record,
            input_v: s.input_v,
            code: s.code,
        };
        let b = &mut self.buckets[idx];
        if b.len() < 4 {
            b.push(p);
            return;
        }
        b[1] = p.clone();
        if p.code < b[2].code {
            b[2] = p.clone();
        }
        if p.code > b[3].code {
            b[3] = p;
        }
    }
    pub fn push(&mut self, s: Sample) -> Result<(), AuditError> {
        self.sample_count = self
            .sample_count
            .checked_add(1)
            .ok_or_else(|| AuditError::validation("sample count overflow"))?;
        self.seen[s.code] = self.seen[s.code]
            .checked_add(1)
            .ok_or_else(|| AuditError::validation("code sample count overflow"))?;
        self.plot(s);
        if let Some(p) = self.previous {
            let step = s.input_v - p.input_v;
            if !step.is_finite() || step <= 0.0 {
                return Err(AuditError::validation(format!(
                    "CSV record {}: input_v must increase strictly",
                    s.record
                )));
            }
            self.step_count += 1;
            self.step_min = self.step_min.min(step);
            self.step_max = self.step_max.max(step);
            self.step_mean += (step - self.step_mean) / (self.step_count as f64);
            if s.code > p.code {
                let delta = s.code - p.code;
                if delta > 1 {
                    self.jump_count += 1;
                    if self.jumps.len() < EVENT_CAP {
                        self.jumps.push(Self::event(p, s));
                    }
                }
                if self.reversal_count == 0 {
                    let bracket = Bracket {
                        lower_v: p.input_v,
                        upper_v: s.input_v,
                        lower_record: p.record,
                        upper_record: s.record,
                    };
                    for k in (p.code + 1)..=s.code {
                        if self.slots[k - 1].is_none() {
                            self.slots[k - 1] = Some((bracket, delta == 1));
                        }
                    }
                }
            } else if s.code < p.code {
                self.reversal_count += 1;
                if self.reversals.len() < EVENT_CAP {
                    self.reversals.push(Self::event(p, s));
                }
            }
        }
        self.previous = Some(s);
        Ok(())
    }
    pub fn finish(self) -> Result<Observation, AuditError> {
        if self.sample_count < 2 {
            return Err(AuditError::validation(
                "CSV requires at least two data records",
            ));
        }
        let nonmono = self.reversal_count > 0;
        let mut transitions = Vec::with_capacity(self.cfg.levels - 1);
        for (i, slot) in self.slots.iter().enumerate() {
            let k = i + 1;
            let ideal = self.cfg.source.vmin_v + k as f64 * self.cfg.nominal_lsb_v;
            let (status, bracket, estimate, half, lo, hi) = if nonmono {
                (
                    TransitionStatus::Invalidated,
                    slot.map(|x| x.0),
                    None,
                    None,
                    None,
                    None,
                )
            } else if let Some((b, resolved)) = slot {
                let lo = Some((b.lower_v - ideal) / self.cfg.nominal_lsb_v);
                let hi = Some((b.upper_v - ideal) / self.cfg.nominal_lsb_v);
                if *resolved {
                    let h = (b.upper_v - b.lower_v) / 2.0;
                    (
                        TransitionStatus::Resolved,
                        Some(*b),
                        Some(b.lower_v + h),
                        Some(h),
                        lo,
                        hi,
                    )
                } else {
                    (TransitionStatus::Unresolved, Some(*b), None, None, lo, hi)
                }
            } else {
                (TransitionStatus::Unobserved, None, None, None, None, None)
            };
            transitions.push(Transition {
                k,
                status,
                bracket,
                estimate_v: estimate,
                half_bracket_v: half,
                nominal_error_lower_lsb: lo,
                nominal_error_upper_lsb: hi,
            });
        }
        let observed = self.seen.iter().filter(|&&n| n > 0).count();
        let min = self.seen.iter().position(|&n| n > 0);
        let max = self.seen.iter().rposition(|&n| n > 0);
        let mut missing = vec![];
        let mut untested = vec![];
        for (c, &n) in self.seen.iter().enumerate() {
            if n == 0 {
                if min.is_some_and(|x| c >= x) && max.is_some_and(|x| c <= x) {
                    missing.push(c)
                } else {
                    untested.push(c)
                }
            }
        }
        let covered = transitions.iter().filter(|t| t.bracket.is_some()).count();
        let resolved = if nonmono {
            0
        } else {
            transitions
                .iter()
                .filter(|t| t.status == TransitionStatus::Resolved)
                .count()
        };
        let status = if nonmono {
            SweepStatus::NonMonotonic
        } else if resolved == self.cfg.levels - 1 {
            SweepStatus::Valid
        } else {
            SweepStatus::Partial
        };
        let max_bracket = transitions
            .iter()
            .filter_map(|t| t.bracket.map(|b| b.upper_v - b.lower_v))
            .reduce(f64::max);
        let resolved_widths = if nonmono {
            0
        } else {
            transitions
                .windows(2)
                .filter(|w| w[0].estimate_v.is_some() && w[1].estimate_v.is_some())
                .count()
        };
        let mut warnings = vec![];
        if self.step_max / self.cfg.nominal_lsb_v >= 1.0 {
            warnings.push("coarse_sampling".into());
        }
        if status == SweepStatus::Partial {
            warnings.push("partial_coverage".into());
        }
        if nonmono {
            warnings.push("code_reversal".into());
            warnings.push("diagnostic_brackets_incomplete".into());
        }
        let diagnostics = Diagnostics {
            jump_count: self.jump_count,
            reversal_count: self.reversal_count,
            jumps_omitted: self.jump_count.saturating_sub(self.jumps.len() as u64),
            reversals_omitted: self
                .reversal_count
                .saturating_sub(self.reversals.len() as u64),
            jumps: self.jumps,
            reversals: self.reversals,
            missing_code_candidates: missing,
            untested_codes: untested,
            candidates_interpretable: !nonmono,
            warnings,
        };
        let quality = Quality {
            step_count: self.step_count,
            step_min_v: self.step_min,
            step_mean_v: self.step_mean,
            step_max_v: self.step_max,
            step_min_lsb: self.step_min / self.cfg.nominal_lsb_v,
            step_mean_lsb: self.step_mean / self.cfg.nominal_lsb_v,
            step_max_lsb: self.step_max / self.cfg.nominal_lsb_v,
            max_bracket_v: max_bracket,
            max_bracket_lsb: max_bracket.map(|x| x / self.cfg.nominal_lsb_v),
        };
        let coverage = Coverage {
            observed_code_count: observed,
            min_observed_code: min,
            max_observed_code: max,
            resolved_transition_count: resolved,
            covered_transition_count: covered,
            expected_transition_count: self.cfg.levels - 1,
            resolved_width_count: resolved_widths,
            expected_width_count: self.cfg.levels - 2,
        };
        let mut points: Vec<_> = self.buckets.into_iter().flatten().collect();
        points.sort_by_key(|p| p.record);
        points.dedup_by_key(|p| p.record);
        Ok(Observation {
            transitions,
            seen: self.seen,
            diagnostics,
            coverage,
            quality,
            status,
            sample_count: self.sample_count,
            plot_points: points,
        })
    }
}
