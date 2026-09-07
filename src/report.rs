//! Stable JSON, CSV and synthetic artifact writers.
use crate::{model::*, svg, synthetic::SyntheticData, AuditError};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

pub fn prepare_output(out: &Path) -> Result<(), AuditError> {
    if out.exists() {
        if !out.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "output path is not a directory",
            )
            .into());
        }
        if fs::read_dir(out)?.next().transpose()?.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "output directory must be new or empty",
            )
            .into());
        }
    } else {
        fs::create_dir_all(out)?
    }
    Ok(())
}
fn opt(v: Option<f64>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}
fn opt_u(v: Option<u32>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}
pub fn write_report(report: &AuditReport, out: &Path) -> Result<(), AuditError> {
    prepare_output(out)?;
    let mut json = BufWriter::new(File::create(out.join("summary.json"))?);
    serde_json::to_writer_pretty(&mut json, report)?;
    json.write_all(b"\n")?;
    let mut th = vec![
        "k",
        "status",
        "lower_v",
        "upper_v",
        "lower_record",
        "upper_record",
        "estimate_v",
        "half_bracket_v",
        "nominal_error_lower_lsb",
        "nominal_error_upper_lsb",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    for r in &report.references {
        for suffix in ["inl_lsb", "cumulative_inl_lsb", "run_id", "run_start"] {
            th.push(format!("{}_{}", r.name, suffix))
        }
    }
    let mut tw = csv::WriterBuilder::new()
        .terminator(csv::Terminator::Any(b'\n'))
        .from_path(out.join("transitions.csv"))?;
    tw.write_record(&th)?;
    for (i, t) in report.transitions.iter().enumerate() {
        let status = serde_json::to_value(t.status)?
            .as_str()
            .unwrap_or("")
            .to_string();
        let b = t.bracket;
        let mut row = vec![
            t.k.to_string(),
            status,
            opt(b.map(|x| x.lower_v)),
            opt(b.map(|x| x.upper_v)),
            b.map(|x| x.lower_record.to_string()).unwrap_or_default(),
            b.map(|x| x.upper_record.to_string()).unwrap_or_default(),
            opt(t.estimate_v),
            opt(t.half_bracket_v),
            opt(t.nominal_error_lower_lsb),
            opt(t.nominal_error_upper_lsb),
        ];
        for r in &report.references {
            let m = &r.transition_metrics[i];
            row.extend([
                opt(m.inl_lsb),
                opt(m.cumulative_inl_lsb),
                opt_u(m.cumulative_run_id),
                m.cumulative_inl_lsb
                    .map(|_| m.cumulative_run_start.to_string())
                    .unwrap_or_default(),
            ])
        }
        tw.write_record(row)?;
    }
    tw.flush()?;
    let mut ch = vec![
        "code",
        "samples",
        "observation_status",
        "width_status",
        "width_v",
        "width_lower_v",
        "width_upper_v",
        "nominal_dnl_lsb",
        "nominal_dnl_lower_lsb",
        "nominal_dnl_upper_lsb",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    for r in &report.references {
        ch.push(format!("{}_dnl_lsb", r.name))
    }
    let mut cw = csv::WriterBuilder::new()
        .terminator(csv::Terminator::Any(b'\n'))
        .from_path(out.join("codes.csv"))?;
    cw.write_record(&ch)?;
    for c in &report.codes {
        let mut row = vec![
            c.code.to_string(),
            c.samples.to_string(),
            c.observation_status.clone(),
            c.width_status.clone(),
            opt(c.width_v),
            opt(c.width_lower_v),
            opt(c.width_upper_v),
            opt(c.nominal_dnl_lsb),
            opt(c.nominal_dnl_lower_lsb),
            opt(c.nominal_dnl_upper_lsb),
        ];
        for r in &report.references {
            row.push(if c.code == 0 || c.code + 1 == report.config.levels {
                String::new()
            } else {
                opt(r.code_metrics[c.code - 1].dnl_lsb)
            })
        }
        cw.write_record(row)?;
    }
    cw.flush()?;
    fs::write(out.join("transfer.svg"), svg::render_transfer(report)?)?;
    fs::write(out.join("dnl.svg"), svg::render_dnl(report)?)?;
    fs::write(out.join("inl.svg"), svg::render_inl(report)?)?;
    Ok(())
}
pub fn write_synth(
    data: &SyntheticData,
    out: &Path,
    decimation: Option<u64>,
) -> Result<(), AuditError> {
    prepare_output(out)?;
    let mut w = csv::WriterBuilder::new()
        .terminator(csv::Terminator::Any(b'\n'))
        .from_path(out.join("sweep.csv"))?;
    w.write_record(["input_v", "code"])?;
    let d = decimation.unwrap_or(1);
    let mut i = 0;
    loop {
        let s = data.sample(i)?;
        w.write_record([s.input_v.to_string(), s.code.to_string()])?;
        if i == data.intervals {
            break;
        }
        i = (i + d).min(data.intervals)
    }
    w.flush()?;
    let truth = data.truth(decimation);
    let mut f = BufWriter::new(File::create(out.join("truth.json"))?);
    serde_json::to_writer_pretty(&mut f, &truth)?;
    f.write_all(b"\n")?;
    Ok(())
}
