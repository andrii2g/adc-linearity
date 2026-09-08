//! Command-line orchestration and centralized exit mapping.
use crate::{
    analysis::{analyze, ReferenceSelection},
    config::AuditConfig,
    model::SweepStatus,
    report,
    synthetic::{self, SynthConfig, SynthModel},
    AuditError,
};
use clap::{error::ErrorKind, Parser, Subcommand, ValueEnum};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(
    name = "adc-linearity-audit",
    version,
    about = "Audit static ADC linearity from a known-input ramp"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Analyze a strictly increasing input-voltage/code CSV sweep.
    Analyze {
        /// CSV containing exactly input_v and code columns.
        csv: PathBuf,
        /// ADC resolution in bits (2..=16).
        #[arg(long)]
        bits: u8,
        /// Nominal minimum input voltage.
        #[arg(long, allow_hyphen_values = true)]
        vmin: f64,
        /// Nominal maximum input voltage.
        #[arg(long, allow_hyphen_values = true)]
        vmax: f64,
        /// Straight-line reference(s) to evaluate.
        #[arg(long, value_enum, default_value = "all")]
        reference: RefArg,
        /// New or empty report directory.
        #[arg(long)]
        out: PathBuf,
        /// Return 3 after writing reports unless the sweep is fully valid.
        #[arg(long)]
        strict: bool,
    },
    /// Generate a deterministic threshold-model sweep and independent truth.
    Synth {
        /// Threshold model to generate.
        #[arg(long, value_enum)]
        model: ModelArg,
        #[arg(long, default_value_t = 12)]
        bits: u8,
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        vmin: f64,
        #[arg(long, default_value_t = 3.3, allow_hyphen_values = true)]
        vmax: f64,
        #[arg(long, default_value_t = 64)]
        samples_per_lsb: u32,
        /// Peak threshold displacement in nominal LSB; defaults to 3 for bow and 0.4 for periodic.
        #[arg(long)]
        amplitude_lsb: Option<f64>,
        /// Number of sinusoidal periods; periodic only, default 8.
        #[arg(long)]
        periods: Option<u32>,
        /// Interior code to delete; missing-code only, default M/2.
        #[arg(long)]
        missing_code: Option<usize>,
        /// Affine threshold shift at vmin in nominal LSB. The reported first-transition offset also includes span scaling.
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        offset_lsb: f64,
        /// Input-span scale error in percent; must leave a positive scale.
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        span_error_percent: f64,
        /// New or empty synthetic-output directory.
        #[arg(long)]
        out: PathBuf,
    },
    /// Generate and audit the six fixed educational scenarios.
    Demo {
        /// New or empty demo root.
        #[arg(long)]
        out: PathBuf,
    },
}
#[derive(Clone, Copy, ValueEnum)]
enum RefArg {
    Nominal,
    Endpoint,
    BestFit,
    All,
}
impl From<RefArg> for ReferenceSelection {
    fn from(v: RefArg) -> Self {
        match v {
            RefArg::Nominal => Self::Nominal,
            RefArg::Endpoint => Self::Endpoint,
            RefArg::BestFit => Self::BestFit,
            RefArg::All => Self::All,
        }
    }
}
#[derive(Clone, Copy, ValueEnum)]
enum ModelArg {
    Perfect,
    Bow,
    Periodic,
    MissingCode,
}
impl From<ModelArg> for SynthModel {
    fn from(v: ModelArg) -> Self {
        match v {
            ModelArg::Perfect => Self::Perfect,
            ModelArg::Bow => Self::Bow,
            ModelArg::Periodic => Self::Periodic,
            ModelArg::MissingCode => Self::MissingCode,
        }
    }
}

pub fn run() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            let ok = matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion);
            let _ = e.print();
            return if ok { 0 } else { 2 };
        }
    };
    match execute(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            e.exit_code()
        }
    }
}
fn execute(cli: Cli) -> Result<i32, AuditError> {
    match cli.command {
        Command::Analyze {
            csv,
            bits,
            vmin,
            vmax,
            reference,
            out,
            strict,
        } => {
            let source = csv
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("input.csv")
                .to_owned();
            let report = analyze(
                File::open(&csv)?,
                &AuditConfig {
                    bits,
                    vmin_v: vmin,
                    vmax_v: vmax,
                },
                reference.into(),
                source,
            )?;
            report::write_report(&report, &out)?;
            for warning in &report.diagnostics.warnings {
                eprintln!("warning: {warning}");
            }
            print_summary(&report, &out);
            Ok(if strict && report.status != SweepStatus::Valid {
                3
            } else {
                0
            })
        }
        Command::Synth {
            model,
            bits,
            vmin,
            vmax,
            samples_per_lsb,
            amplitude_lsb,
            periods,
            missing_code,
            offset_lsb,
            span_error_percent,
            out,
        } => {
            validate_model_flags(model, amplitude_lsb, periods, missing_code)?;
            let data = synthetic::make(SynthConfig {
                audit: AuditConfig {
                    bits,
                    vmin_v: vmin,
                    vmax_v: vmax,
                },
                model: model.into(),
                samples_per_lsb,
                amplitude_lsb,
                periods,
                missing_code,
                offset_lsb,
                span_error_percent,
            })?;
            report::write_synth(&data, &out, None)?;
            println!(
                "generated {} samples for {} in {}",
                data.sample_count(),
                data.request.model.name(),
                out.display()
            );
            Ok(0)
        }
        Command::Demo { out } => demo(&out),
    }
}
fn validate_model_flags(
    model: ModelArg,
    a: Option<f64>,
    p: Option<u32>,
    m: Option<usize>,
) -> Result<(), AuditError> {
    if a.is_some() && !matches!(model, ModelArg::Bow | ModelArg::Periodic) {
        return Err(AuditError::validation(
            "--amplitude-lsb applies only to bow or periodic",
        ));
    }
    if p.is_some() && !matches!(model, ModelArg::Periodic) {
        return Err(AuditError::validation("--periods applies only to periodic"));
    }
    if m.is_some() && !matches!(model, ModelArg::MissingCode) {
        return Err(AuditError::validation(
            "--missing-code applies only to missing-code",
        ));
    }
    Ok(())
}
fn print_summary(r: &crate::model::AuditReport, out: &Path) {
    println!(
        "{}: {} rows, {} bits, [{}, {}] V, q={} V",
        r.source,
        r.sample_count,
        r.config.bits,
        r.config.vmin_v,
        r.config.vmax_v,
        r.config.nominal_lsb_v
    );
    println!(
        "status {:?}; transitions {}/{}; widths {}/{}; mean/max step {:.6}/{:.6} LSB",
        r.status,
        r.coverage.resolved_transition_count,
        r.coverage.expected_transition_count,
        r.coverage.resolved_width_count,
        r.coverage.expected_width_count,
        r.quality.step_mean_lsb,
        r.quality.step_max_lsb
    );
    println!(
        "jumps {}; reversals {}; missing candidates {}; untested codes {}",
        r.diagnostics.jump_count,
        r.diagnostics.reversal_count,
        r.diagnostics.missing_code_candidates.len(),
        r.diagnostics.untested_codes.len()
    );
    if r.calibration.availability == "available" {
        println!(
            "endpoint offset {} LSB; input-span gain error {}%",
            r.calibration.offset_lsb.unwrap_or(0.0),
            r.calibration.gain_span_error_percent.unwrap_or(0.0)
        )
    } else {
        println!(
            "endpoint calibration unavailable: {}",
            r.calibration.reason.as_deref().unwrap_or("unknown")
        )
    }
    for rf in &r.references {
        if rf.availability == "available" {
            let scope = if rf.coverage.scope == "partial" {
                "partial "
            } else {
                ""
            };
            let dnl = rf
                .dnl_summary
                .as_ref()
                .map(|x| x.max_abs_lsb.to_string())
                .unwrap_or_else(|| "unavailable".into());
            let inl = rf
                .inl_summary
                .as_ref()
                .map(|x| x.max_abs_lsb.to_string())
                .unwrap_or_else(|| "unavailable".into());
            println!(
                "{}: {} transitions/{} widths; {}max |DNL| = {} {}, {}max |{}| = {} {}",
                rf.name,
                rf.coverage.evaluated_transition_count,
                rf.coverage.evaluated_width_count,
                scope,
                dnl,
                rf.lsb_basis,
                scope,
                rf.display_quantity,
                inl,
                rf.lsb_basis
            )
        } else {
            println!(
                "{} unavailable: {}",
                rf.name,
                rf.reason.as_deref().unwrap_or("unknown")
            )
        }
    }
    println!("reports: {}", out.display());
}
fn demo(out: &Path) -> Result<i32, AuditError> {
    report::prepare_output(out)?;
    let base = AuditConfig {
        bits: 8,
        vmin_v: 0.0,
        vmax_v: 2.56,
    };
    let cases = [
        (
            "perfect",
            SynthModel::Perfect,
            None,
            None,
            None,
            0.0,
            0.0,
            None,
        ),
        (
            "bow",
            SynthModel::Bow,
            Some(3.0),
            None,
            None,
            0.0,
            0.0,
            None,
        ),
        (
            "periodic",
            SynthModel::Periodic,
            Some(0.4),
            Some(8),
            None,
            0.0,
            0.0,
            None,
        ),
        (
            "missing-code",
            SynthModel::MissingCode,
            None,
            None,
            Some(128),
            0.0,
            0.0,
            None,
        ),
        (
            "affine",
            SynthModel::Perfect,
            None,
            None,
            None,
            0.25,
            1.0,
            None,
        ),
        (
            "coarse-perfect",
            SynthModel::Perfect,
            None,
            None,
            None,
            0.0,
            0.0,
            Some(256u64),
        ),
    ];
    let mut summary = csv::WriterBuilder::new()
        .terminator(csv::Terminator::Any(b'\n'))
        .from_path(out.join("demo-summary.csv"))?;
    summary.write_record([
        "scenario",
        "status",
        "resolved_transitions",
        "expected_transitions",
        "missing_candidate_count",
        "truth_missing_code_count",
        "endpoint_max_abs_inl_lsb",
        "best_fit_max_abs_inl_lsb",
        "nominal_max_abs_dnl_lsb",
        "offset_lsb",
        "gain_span_error_percent",
    ])?;
    for (name, model, amp, periods, missing, offset, span, decimation) in cases {
        let root = out.join(name);
        std::fs::create_dir(&root)?;
        let input = root.join("input");
        let audit = root.join("audit");
        let data = synthetic::make(SynthConfig {
            audit: base,
            model,
            samples_per_lsb: 64,
            amplitude_lsb: amp,
            periods,
            missing_code: missing,
            offset_lsb: offset,
            span_error_percent: span,
        })?;
        report::write_synth(&data, &input, decimation)?;
        let r = analyze(
            File::open(input.join("sweep.csv"))?,
            &base,
            ReferenceSelection::All,
            "sweep.csv".into(),
        )?;
        report::write_report(&r, &audit)?;
        let metric = |n: &str, dnl: bool| {
            r.references
                .iter()
                .find(|x| x.name == n)
                .and_then(|x| {
                    if dnl {
                        x.dnl_summary.as_ref().map(|s| s.max_abs_lsb)
                    } else {
                        x.inl_summary.as_ref().map(|s| s.max_abs_lsb)
                    }
                })
                .map(|x| x.to_string())
                .unwrap_or_default()
        };
        summary.write_record([
            name,
            match r.status {
                SweepStatus::Valid => "valid",
                SweepStatus::Partial => "partial",
                SweepStatus::NonMonotonic => "non_monotonic",
            },
            &r.coverage.resolved_transition_count.to_string(),
            &r.coverage.expected_transition_count.to_string(),
            &r.diagnostics.missing_code_candidates.len().to_string(),
            &data.true_missing_codes.len().to_string(),
            &metric("endpoint", false),
            &metric("best_fit", false),
            &metric("nominal", true),
            &r.calibration
                .offset_lsb
                .map(|x| x.to_string())
                .unwrap_or_default(),
            &r.calibration
                .gain_span_error_percent
                .map(|x| x.to_string())
                .unwrap_or_default(),
        ])?;
    }
    summary.flush()?;
    println!("generated six-scenario demo in {}", out.display());
    Ok(0)
}
