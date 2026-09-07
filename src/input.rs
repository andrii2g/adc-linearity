//! Strict streaming CSV input.
use crate::{model::Sample, AuditError};
use std::io::Read;

pub fn read_samples<R, F>(reader: R, levels: usize, mut emit: F) -> Result<u64, AuditError>
where
    R: Read,
    F: FnMut(Sample) -> Result<(), AuditError>,
{
    let mut csv = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(false)
        .from_reader(reader);
    let headers = csv
        .headers()
        .map_err(|e| AuditError::validation(format!("CSV header: {e}")))?
        .clone();
    let names: Vec<String> = headers
        .iter()
        .map(|s| s.trim_start_matches('﻿').trim().to_owned())
        .collect();
    if names.len() != 2
        || names.iter().filter(|s| s.as_str() == "input_v").count() != 1
        || names.iter().filter(|s| s.as_str() == "code").count() != 1
    {
        return Err(AuditError::validation(
            "CSV header must contain exactly input_v and code",
        ));
    }
    let vi = names.iter().position(|s| s == "input_v").unwrap_or(0);
    let ci = names.iter().position(|s| s == "code").unwrap_or(1);
    let mut count = 0u64;
    for (index, row) in csv.records().enumerate() {
        let record = (index as u64) + 2;
        let row = row.map_err(|e| AuditError::validation(format!("CSV record {record}: {e}")))?;
        if row.len() != 2 {
            return Err(AuditError::validation(format!(
                "CSV record {record}: expected 2 fields"
            )));
        }
        let vs = row.get(vi).unwrap_or("").trim();
        let cs = row.get(ci).unwrap_or("").trim();
        if vs.is_empty() || cs.is_empty() {
            return Err(AuditError::validation(format!(
                "CSV record {record}: blank field"
            )));
        }
        let input_v: f64 = vs
            .parse()
            .map_err(|_| AuditError::validation(format!("CSV record {record}: invalid input_v")))?;
        if !input_v.is_finite() {
            return Err(AuditError::validation(format!(
                "CSV record {record}: input_v must be finite"
            )));
        }
        let code_u: u64 = cs.parse().map_err(|_| {
            AuditError::validation(format!(
                "CSV record {record}: code must be an unsigned integer"
            ))
        })?;
        let code = usize::try_from(code_u).map_err(|_| {
            AuditError::validation(format!("CSV record {record}: code is too large"))
        })?;
        if code >= levels {
            return Err(AuditError::validation(format!(
                "CSV record {record}: code {code} outside 0..{}",
                levels - 1
            )));
        }
        emit(Sample {
            record,
            input_v,
            code,
        })?;
        count = count
            .checked_add(1)
            .ok_or_else(|| AuditError::validation("sample count overflow"))?;
    }
    if count < 2 {
        return Err(AuditError::validation(
            "CSV requires at least two data records",
        ));
    }
    Ok(count)
}
