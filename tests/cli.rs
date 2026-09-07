use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn temp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "adc-linearity-audit-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        name
    ))
}
fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_adc-linearity-audit"))
}
fn fixture(path: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(path)
}

#[test]
fn help_and_version_succeed() {
    assert!(bin().arg("--help").status().unwrap().success());
    assert!(bin().arg("--version").status().unwrap().success())
}
#[test]
fn strict_partial_writes_reports_then_returns_three() {
    let out = temp("strict");
    let status = bin()
        .args(["analyze"])
        .arg(fixture("missing-3bit.csv"))
        .args(["--bits", "3", "--vmin", "0", "--vmax", "8", "--out"])
        .arg(&out)
        .arg("--strict")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(3));
    for f in [
        "summary.json",
        "transitions.csv",
        "codes.csv",
        "transfer.svg",
        "dnl.svg",
        "inl.svg",
    ] {
        assert!(out.join(f).is_file())
    }
    fs::remove_dir_all(out).unwrap();
}
#[test]
fn invalid_csv_returns_two_without_creating_output() {
    let out = temp("invalid");
    let status = bin()
        .args(["analyze"])
        .arg(fixture("invalid/non-increasing.csv"))
        .args(["--bits", "3", "--vmin", "0", "--vmax", "8", "--out"])
        .arg(&out)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
    assert!(!out.exists());
}
#[test]
fn nonempty_output_is_unchanged_and_returns_one() {
    let out = temp("occupied");
    fs::create_dir(&out).unwrap();
    let sentinel = out.join("sentinel");
    fs::write(&sentinel, b"keep").unwrap();
    let status = bin()
        .args(["analyze"])
        .arg(fixture("perfect-3bit.csv"))
        .args(["--bits", "3", "--vmin", "0", "--vmax", "8", "--out"])
        .arg(&out)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
    assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
    fs::remove_dir_all(out).unwrap();
}
#[test]
fn synth_accepts_negative_range_and_writes_truth() {
    let out = temp("synth");
    let status = bin()
        .args([
            "synth", "--model", "perfect", "--bits", "3", "--vmin", "-4", "--vmax", "4", "--out",
        ])
        .arg(&out)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
    assert!(out.join("sweep.csv").is_file());
    assert!(out.join("truth.json").is_file());
    fs::remove_dir_all(out).unwrap();
}
