//! ADC configuration validation.

use crate::AuditError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AuditConfig {
    pub bits: u8,
    pub vmin_v: f64,
    pub vmax_v: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct DerivedConfig {
    pub source: AuditConfig,
    pub levels: usize,
    pub nominal_lsb_v: f64,
}

impl AuditConfig {
    pub fn validate(self) -> Result<DerivedConfig, AuditError> {
        if !(2..=16).contains(&self.bits) {
            return Err(AuditError::validation("bits must be in 2..=16"));
        }
        if !self.vmin_v.is_finite() || !self.vmax_v.is_finite() {
            return Err(AuditError::validation("range endpoints must be finite"));
        }
        let span = self.vmax_v - self.vmin_v;
        if !span.is_finite() || span <= 0.0 {
            return Err(AuditError::validation(
                "vmax-vmin must be finite and positive",
            ));
        }
        let levels = 1usize
            .checked_shl(u32::from(self.bits))
            .ok_or_else(|| AuditError::validation("code-space size overflow"))?;
        let nominal_lsb_v = span / levels as f64;
        if !nominal_lsb_v.is_finite() || nominal_lsb_v <= 0.0 {
            return Err(AuditError::validation(
                "nominal LSB must be representable and positive",
            ));
        }
        let mut previous = self.vmin_v;
        for k in 1..levels {
            let current = self.vmin_v + k as f64 * nominal_lsb_v;
            if !current.is_finite() || current <= previous {
                return Err(AuditError::validation(
                    "nominal transitions are not distinguishable in f64",
                ));
            }
            previous = current;
        }
        if previous >= self.vmax_v {
            return Err(AuditError::validation(
                "last nominal bin is not distinguishable in f64",
            ));
        }
        Ok(DerivedConfig {
            source: self,
            levels,
            nominal_lsb_v,
        })
    }
}
