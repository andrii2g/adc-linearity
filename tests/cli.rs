use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
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
fn collect_tree(root:&Path)->BTreeMap<PathBuf,Vec<u8>>{
    fn visit(root:&Path,dir:&Path,files:&mut BTreeMap<PathBuf,Vec<u8>>){
        for entry in fs::read_dir(dir).unwrap(){let entry=entry.unwrap();let path=entry.path();if path.is_dir(){visit(root,&path,files)}else{files.insert(path.strip_prefix(root).unwrap().to_owned(),fs::read(path).unwrap());}}
    }
    let mut files=BTreeMap::new();visit(root,root,&mut files);files
}

#[test]
fn help_and_version_succeed() {
    assert!(bin().arg("--help").status().unwrap().success());
    assert!(bin().arg("--version").status().unwrap().success())
}
#[test]
fn strict_partial_writes_reports_then_returns_three() {
    let out = temp("strict");
    let output = bin()
        .args(["analyze"])
        .arg(fixture("missing-3bit.csv"))
        .args(["--bits", "3", "--vmin", "0", "--vmax", "8", "--out"])
        .arg(&out)
        .arg("--strict")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("warning: partial_coverage"));
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
    let mut csv = csv::Reader::from_path(out.join("transitions.csv")).unwrap();
    let headers = csv.headers().unwrap().clone();
    let boolean_columns: Vec<_> = headers
        .iter()
        .enumerate()
        .filter(|(_, h)| h.ends_with("_run_start"))
        .map(|(i, _)| i)
        .collect();
    for row in csv.records() {
        let row = row.unwrap();
        for &column in &boolean_columns {
            assert!(matches!(row.get(column), Some("true" | "false")));
        }
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

#[test]
fn synth_help_explains_model_defaults_and_inapplicable_flags_fail() {
    let help = bin().args(["synth", "--help"]).output().unwrap();
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(text.contains("defaults to 3 for bow and 0.4 for periodic"));
    assert!(text.contains("default 8"));
    assert!(text.contains("default M/2"));
    let out = temp("bad-flags");
    let status = bin()
        .args(["synth", "--model", "perfect", "--periods", "8", "--out"])
        .arg(&out)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
    assert!(!out.exists());
}

#[test]
fn invalid_output_parent_returns_one() {
    let parent = temp("file-parent");
    fs::write(&parent, b"file").unwrap();
    let out = parent.join("child");
    let status = bin()
        .args(["analyze"])
        .arg(fixture("perfect-3bit.csv"))
        .args(["--bits", "3", "--vmin", "0", "--vmax", "8", "--out"])
        .arg(&out)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
    fs::remove_file(parent).unwrap();
}

#[test]
fn demo_artifacts_are_complete_semantic_and_deterministic(){
    let first=temp("demo-first");let second=temp("demo-second");
    assert!(bin().args(["demo","--out"]).arg(&first).status().unwrap().success());
    assert!(bin().args(["demo","--out"]).arg(&second).status().unwrap().success());
    let scenarios=["perfect","bow","periodic","missing-code","affine","coarse-perfect"];
    for name in scenarios{
        let root=first.join(name);let audit=root.join("audit");
        let report:serde_json::Value=serde_json::from_reader(fs::File::open(audit.join("summary.json")).unwrap()).unwrap();
        let truth:serde_json::Value=serde_json::from_reader(fs::File::open(root.join("input/truth.json")).unwrap()).unwrap();
        assert_eq!(report["schema_version"],1);let levels=1usize<<report["config"]["bits"].as_u64().unwrap();
        assert_eq!(report["transitions"].as_array().unwrap().len(),levels-1);assert_eq!(report["codes"].as_array().unwrap().len(),levels);
        let refs:Vec<_>=report["references"].as_array().unwrap().iter().map(|r|r["name"].as_str().unwrap()).collect();assert_eq!(refs,vec!["nominal","endpoint","best_fit"]);
        for(file,count)in[("transitions.csv",levels-1),("codes.csv",levels)]{let mut reader=csv::Reader::from_path(audit.join(file)).unwrap();assert_eq!(reader.records().count(),count);}
        for file in ["transfer.svg","dnl.svg","inl.svg"]{let svg=fs::read_to_string(audit.join(file)).unwrap();assert!(svg.starts_with("<svg"));assert!(svg.contains("<title>"));assert!(svg.ends_with("</svg>"));assert!(!svg.contains("<script"));assert!(!svg.contains("<foreignObject"));}
        if matches!(name,"perfect"|"bow"|"periodic"|"affine"){assert_eq!(report["status"],"valid");assert_eq!(report["diagnostics"]["missing_code_candidates"].as_array().unwrap().len(),0);}
        if name=="missing-code"{assert_eq!(report["status"],"partial");assert_eq!(truth["true_missing_codes"],serde_json::json!([128]));assert_eq!(report["diagnostics"]["missing_code_candidates"],serde_json::json!([128]));assert!(report["codes"][128]["width_v"].is_null());}
        if name=="coarse-perfect"{assert_eq!(report["status"],"partial");assert_eq!(truth["true_missing_codes"],serde_json::json!([]));assert!(!report["diagnostics"]["missing_code_candidates"].as_array().unwrap().is_empty());}
    }
    let mut summary=csv::Reader::from_path(first.join("demo-summary.csv")).unwrap();assert_eq!(summary.records().count(),6);
    assert_eq!(collect_tree(&first),collect_tree(&second));
    fs::remove_dir_all(first).unwrap();fs::remove_dir_all(second).unwrap();
}
