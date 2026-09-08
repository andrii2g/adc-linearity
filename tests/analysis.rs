use adc_linearity_audit::{
    analysis::{analyze, ReferenceSelection},
    config::AuditConfig,
    model::{SweepStatus, TransitionStatus},
    svg,
    synthetic::{make, SynthConfig, SynthModel},
};
use std::{fs::File, io::Cursor, path::PathBuf};

fn cfg() -> AuditConfig {
    AuditConfig {
        bits: 3,
        vmin_v: 0.0,
        vmax_v: 8.0,
    }
}
fn fixture(name: &str) -> File {
    File::open(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(name),
    )
    .unwrap()
}
fn audit(name: &str) -> adc_linearity_audit::model::AuditReport {
    analyze(fixture(name), &cfg(), ReferenceSelection::All, name.into()).unwrap()
}
fn near(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-12 * f64::max(1.0, b.abs()),
        "{a} != {b}"
    )
}

#[test]
fn perfect_fixture_has_exact_midpoints_widths_and_bounds() {
    let r = audit("perfect-3bit.csv");
    assert_eq!(r.status, SweepStatus::Valid);
    assert_eq!(r.sample_count, 64);
    for (t, k) in r.transitions.iter().zip(1..=7) {
        assert_eq!(t.status, TransitionStatus::Resolved);
        assert_eq!(t.estimate_v, Some(k as f64));
        assert_eq!(t.half_bracket_v, Some(0.0625))
    }
    for c in &r.codes[1..7] {
        assert_eq!(c.width_v, Some(1.0));
        assert_eq!(c.width_lower_v, Some(0.875));
        assert_eq!(c.width_upper_v, Some(1.125));
        assert_eq!(c.nominal_dnl_lsb, Some(0.0))
    }
}
#[test]
fn bow_fixture_catches_indexing_and_fit_anchor() {
    let r = audit("bow-hand-3bit.csv");
    let widths = [1.0, 1.25, 1.25, 0.75, 0.75, 1.0];
    for (c, e) in r.codes[1..7].iter().zip(widths) {
        near(c.width_v.unwrap(), e)
    }
    let endpoint = r.references.iter().find(|x| x.name == "endpoint").unwrap();
    let expected = [0.0, 0.0, 0.25, 0.5, 0.25, 0.0, 0.0];
    for (m, e) in endpoint.transition_metrics.iter().zip(expected) {
        near(m.inl_lsb.unwrap(), e);
        near(m.cumulative_inl_lsb.unwrap(), e)
    }
    let best = r.references.iter().find(|x| x.name == "best_fit").unwrap();
    near(best.line.as_ref().unwrap().a_v, 1.0 / 7.0);
    near(best.transition_metrics[0].inl_lsb.unwrap(), -1.0 / 7.0);
    near(best.inl_summary.as_ref().unwrap().max_abs_lsb, 5.0 / 14.0);
}
#[test]
fn affine_calibration_separates_offset_intercept_and_normalization() {
    let r = audit("affine-3bit.csv");
    near(r.calibration.offset_v.unwrap(), 0.375);
    near(r.calibration.gain_span_error_v.unwrap(), 0.75);
    near(r.calibration.gain_span_error_percent.unwrap(), 12.5);
    let endpoint = r.references.iter().find(|x| x.name == "endpoint").unwrap();
    near(endpoint.line.as_ref().unwrap().a_v, 0.25);
    for m in &endpoint.code_metrics {
        near(m.dnl_lsb.unwrap(), 0.0)
    }
    for c in &r.codes[1..7] {
        near(c.nominal_dnl_lsb.unwrap(), 0.125)
    }
}
#[test]
fn missing_code_remains_unresolved_and_reanchors_runs() {
    let r = audit("missing-3bit.csv");
    assert_eq!(r.status, SweepStatus::Partial);
    assert_eq!(r.diagnostics.missing_code_candidates, vec![3]);
    assert_eq!(r.transitions[2].status, TransitionStatus::Unresolved);
    assert_eq!(r.transitions[3].status, TransitionStatus::Unresolved);
    for code in [2usize, 3, 4] {
        assert!(r.codes[code].width_v.is_none())
    }
    assert!(r.codes[3].width_lower_v.is_some());
    let endpoint = r.references.iter().find(|x| x.name == "endpoint").unwrap();
    let ids: Vec<_> = endpoint
        .transition_metrics
        .iter()
        .map(|m| m.cumulative_run_id)
        .collect();
    assert_eq!(
        ids,
        vec![Some(0), Some(0), None, None, Some(1), Some(1), Some(1)]
    );
    assert!(endpoint.transition_metrics[4].cumulative_run_start);
}
#[test]
fn coarse_partial_constant_and_reversal_gates_match_contract() {
    let coarse = audit("coarse-perfect-3bit.csv");
    assert_eq!(coarse.diagnostics.missing_code_candidates, vec![1, 3, 5]);
    assert_eq!(coarse.coverage.resolved_transition_count, 1);
    assert!(coarse
        .references
        .iter()
        .find(|x| x.name == "endpoint")
        .unwrap()
        .line
        .is_none());
    let partial = audit("partial-3bit.csv");
    assert_eq!(partial.diagnostics.untested_codes, vec![0, 1, 6, 7]);
    assert!(partial
        .references
        .iter()
        .find(|x| x.name == "best_fit")
        .unwrap()
        .line
        .is_some());
    let constant = audit("constant-3bit.csv");
    assert!(constant
        .references
        .iter()
        .all(|x| x.availability == "unavailable"));
    let reversal = audit("reversal-3bit.csv");
    assert_eq!(reversal.status, SweepStatus::NonMonotonic);
    assert_eq!(reversal.diagnostics.reversal_count, 1);
    assert!(reversal
        .transitions
        .iter()
        .all(|t| t.status == TransitionStatus::Invalidated));
    assert!(reversal.references.iter().all(|x| x.line.is_none()));
}
#[test]
fn strict_csv_rules_reject_frozen_invalid_inputs() {
    for name in [
        "bad-header.csv",
        "fractional-code.csv",
        "negative-code.csv",
        "non-finite.csv",
        "non-increasing.csv",
        "out-of-range.csv",
        "ragged.csv",
        "single-row.csv",
    ] {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/invalid")
            .join(name);
        assert!(
            analyze(
                File::open(p).unwrap(),
                &cfg(),
                ReferenceSelection::All,
                name.into()
            )
            .is_err(),
            "{name}"
        )
    }
}
#[test]
fn csv_accepts_bom_reversed_headers_whitespace_scientific_and_crlf() {
    let csv = "﻿ code , input_v \r\n 0 , 0e0 \r\n 1 , 1.0 \r\n";
    let r = analyze(
        Cursor::new(csv),
        &cfg(),
        ReferenceSelection::Nominal,
        "memory.csv".into(),
    )
    .unwrap();
    assert_eq!(r.sample_count, 2);
    assert_eq!(r.transitions[0].estimate_v, Some(0.5));
}
#[test]
fn synthetic_missing_truth_is_independent_and_threshold_equality_uses_higher_code() {
    let d = make(SynthConfig {
        audit: cfg(),
        model: SynthModel::MissingCode,
        samples_per_lsb: 64,
        amplitude_lsb: None,
        periods: None,
        missing_code: Some(3),
        offset_lsb: 0.0,
        span_error_percent: 0.0,
    })
    .unwrap();
    assert_eq!(d.thresholds[2], d.thresholds[3]);
    assert_eq!(d.true_missing_codes, vec![3]);
    assert_eq!(d.truth(None).nominal_dnl_lsb[2], -1.0);
    let x = d.thresholds[2];
    let code = d.thresholds.partition_point(|&t| t <= x);
    assert_eq!(code, 4);
}
#[test]
fn diagnostic_event_storage_is_bounded_but_counts_are_exact() {
    let mut csv = String::from("input_v,code\n");
    for i in 0..2202 {
        csv.push_str(&format!("{},{}\n", i, if i % 2 == 0 { 0 } else { 7 }))
    }
    let r = analyze(
        Cursor::new(csv),
        &cfg(),
        ReferenceSelection::All,
        "events.csv".into(),
    )
    .unwrap();
    assert_eq!(r.diagnostics.jump_count, 1101);
    assert_eq!(r.diagnostics.reversal_count, 1100);
    assert_eq!(r.diagnostics.jumps.len(), 1000);
    assert_eq!(r.diagnostics.reversals.len(), 1000);
}

#[test]
fn extrema_ties_saturation_and_absent_summaries_are_explicit() {
    let perfect = audit("perfect-3bit.csv");
    let nominal = perfect
        .references
        .iter()
        .find(|x| x.name == "nominal")
        .unwrap();
    let d = nominal.dnl_summary.as_ref().unwrap();
    assert_eq!((d.min_code, d.max_code, d.max_abs_code), (1, 1, 1));
    let i = nominal.inl_summary.as_ref().unwrap();
    assert_eq!((i.min_k, i.max_k, i.max_abs_k), (1, 1, 1));
    for c in [&perfect.codes[0], &perfect.codes[7]] {
        assert_eq!(c.width_status, "saturation");
        assert!(c.width_v.is_none() && c.nominal_dnl_lsb.is_none());
    }
    let constant = audit("constant-3bit.csv");
    assert!(constant.references.iter().all(|r| r.dnl_summary.is_none()));
}

#[test]
fn calibration_does_not_depend_on_requested_reference() {
    let bytes =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/affine-3bit.csv"))
            .unwrap();
    let mut values = vec![];
    for selection in [
        ReferenceSelection::Nominal,
        ReferenceSelection::Endpoint,
        ReferenceSelection::BestFit,
        ReferenceSelection::All,
    ] {
        let r = analyze(
            Cursor::new(bytes.clone()),
            &cfg(),
            selection,
            "affine.csv".into(),
        )
        .unwrap();
        values.push((
            r.calibration.offset_v,
            r.calibration.gain_span_error_percent,
        ));
    }
    assert!(values.windows(2).all(|w| w[0] == w[1]));
}

#[test]
fn centered_fit_is_stable_with_a_large_voltage_origin() {
    let origin = 1_000_000_000.0;
    let mut csv = String::from("input_v,code\n");
    for i in 0..64 {
        let x = origin + 0.0625 + i as f64 * 0.125;
        let code = ((x - origin).floor() as usize).min(7);
        csv.push_str(&format!("{x},{code}\n"))
    }
    let c = AuditConfig {
        bits: 3,
        vmin_v: origin,
        vmax_v: origin + 8.0,
    };
    let r = analyze(
        Cursor::new(csv),
        &c,
        ReferenceSelection::BestFit,
        "large.csv".into(),
    )
    .unwrap();
    near(r.references[0].line.as_ref().unwrap().b_v_per_code, 1.0);
    assert!(r.references[0]
        .transition_metrics
        .iter()
        .all(|m| m.inl_lsb.unwrap().abs() < 1e-12));
}

#[test]
fn every_resolved_width_truth_lies_inside_sampling_bounds() {
    for name in ["perfect-3bit.csv", "bow-hand-3bit.csv", "affine-3bit.csv"] {
        let r = audit(name);
        for c in &r.codes[1..7] {
            let w = c.width_v.unwrap();
            assert!(c.width_lower_v.unwrap() <= w && w <= c.width_upper_v.unwrap());
        }
        for rf in &r.references {
            for m in &rf.transition_metrics {
                if let (Some(a), Some(b)) = (m.inl_lsb, m.cumulative_inl_lsb) {
                    near(a, b)
                }
            }
        }
    }
}

#[test]
fn synthesis_rejects_excessive_sample_counts_and_nonmonotone_model() {
    let too_many = SynthConfig {
        audit: AuditConfig {
            bits: 16,
            vmin_v: 0.0,
            vmax_v: 1.0,
        },
        model: SynthModel::Perfect,
        samples_per_lsb: 1024,
        amplitude_lsb: None,
        periods: None,
        missing_code: None,
        offset_lsb: 0.0,
        span_error_percent: 0.0,
    };
    assert!(make(too_many).is_err());
    let invalid = SynthConfig {
        audit: cfg(),
        model: SynthModel::Periodic,
        samples_per_lsb: 64,
        amplitude_lsb: Some(100.0),
        periods: Some(2),
        missing_code: None,
        offset_lsb: 0.0,
        span_error_percent: 0.0,
    };
    assert!(make(invalid).is_err());
}

#[test]
fn all_documented_csv_validation_failures_are_rejected_with_context() {
    let cases = [
        ("input_v,other\n0,0\n1,1\n", "header"),
        ("input_v,input_v\n0,0\n1,1\n", "header"),
        ("input_v,code,extra\n0,0,x\n1,1,x\n", "header"),
        ("input_v,code\n0,\n1,1\n", "record 2"),
        ("input_v,code\n0,-1\n1,1\n", "record 2"),
        ("input_v,code\n0,1.5\n1,1\n", "record 2"),
        ("input_v,code\n0,8\n1,1\n", "record 2"),
        ("input_v,code\n0,18446744073709551616\n1,1\n", "record 2"),
        ("input_v,code\nNaN,0\n1,1\n", "record 2"),
        ("input_v,code\nInfinity,0\n1,1\n", "record 2"),
        ("input_v,code\n0,0\n0,1\n", "record 3"),
        ("input_v,code\n1,0\n0,1\n", "record 3"),
        ("input_v,code\n0,0\n", "at least two"),
        ("input_v,code\n\"0,0\n", "record 2"),
    ];
    for (csv, context) in cases {
        let error = analyze(
            Cursor::new(csv),
            &cfg(),
            ReferenceSelection::All,
            "bad.csv".into(),
        )
        .unwrap_err()
        .to_string()
        .to_lowercase();
        assert!(
            error.contains(context),
            "{error:?} did not contain {context:?}"
        );
    }
    let overflow = AuditConfig {
        bits: 3,
        vmin_v: -f64::MAX,
        vmax_v: f64::MAX,
    };
    assert!(analyze(
        Cursor::new("input_v,code\n0,0\n1,1\n"),
        &overflow,
        ReferenceSelection::All,
        "bad.csv".into()
    )
    .is_err());
}

#[test]
fn plot_buckets_retain_exact_first_last_min_and_max_samples() {
    let csv = "input_v,code\n0.000,3\n0.001,7\n0.002,4\n0.003,5\n0.004,0\n0.005,6\n";
    let r = analyze(
        Cursor::new(csv),
        &cfg(),
        ReferenceSelection::All,
        "plot.csv".into(),
    )
    .unwrap();
    let records: Vec<_> = r.plot_points.iter().map(|p| p.record).collect();
    assert_eq!(records, vec![2, 3, 6, 7]);
}

#[test]
fn svg_uses_small_code_bars_brackets_and_bounded_large_series() {
    let perfect = audit("perfect-3bit.csv");
    let dnl = svg::render_dnl(&perfect).unwrap();
    assert!(dnl.contains("<rect x="));
    let transfer = svg::render_transfer(&perfect).unwrap();
    assert!(transfer.contains("#7b3294"));

    let data = make(SynthConfig {
        audit: AuditConfig {
            bits: 12,
            vmin_v: 0.0,
            vmax_v: 4.096,
        },
        model: SynthModel::Bow,
        samples_per_lsb: 1,
        amplitude_lsb: Some(3.0),
        periods: None,
        missing_code: None,
        offset_lsb: 0.0,
        span_error_percent: 0.0,
    })
    .unwrap();
    let mut csv = String::from("input_v,code\n");
    for i in 0..=data.intervals {
        let sample = data.sample(i).unwrap();
        csv.push_str(&format!("{},{}\n", sample.input_v, sample.code))
    }
    let large = analyze(
        Cursor::new(csv),
        &data.request.audit,
        ReferenceSelection::All,
        "large.csv".into(),
    )
    .unwrap();
    for image in [
        svg::render_dnl(&large).unwrap(),
        svg::render_inl(&large).unwrap(),
    ] {
        let mut rest = image.as_str();
        while let Some(start) = rest.find("points=\"") {
            rest = &rest[start + 8..];
            let end = rest.find('"').unwrap();
            let count = rest[..end].split_whitespace().count();
            assert!(count <= 2048, "series had {count} points");
            rest = &rest[end + 1..];
        }
    }
}

#[test]
fn frozen_json_oracle_matches_all_fixture_metrics() {
    let expected:serde_json::Value=serde_json::from_reader(fixture("expected.json")).unwrap();
    for (name,case) in expected["cases"].as_object().unwrap(){
        let report=audit(&format!("{name}.csv"));
        let status=match report.status{SweepStatus::Valid=>"valid",SweepStatus::Partial=>"partial",SweepStatus::NonMonotonic=>"non_monotonic"};
        assert_eq!(status,case["status"].as_str().unwrap(),"{name}: status");
        assert_eq!(report.sample_count,case["samples"].as_u64().unwrap(),"{name}: samples");
        let resolved:Vec<_>=report.transitions.iter().filter(|t|t.status==TransitionStatus::Resolved).map(|t|t.k as u64).collect();
        let unresolved:Vec<_>=report.transitions.iter().filter(|t|t.status==TransitionStatus::Unresolved).map(|t|t.k as u64).collect();
        assert_eq!(resolved,case["resolved_k"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()).collect::<Vec<_>>(),"{name}: resolved");
        assert_eq!(unresolved,case["unresolved_k"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()).collect::<Vec<_>>(),"{name}: unresolved");
        assert_eq!(report.diagnostics.missing_code_candidates.iter().map(|&x|x as u64).collect::<Vec<_>>(),case["missing_candidates"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()).collect::<Vec<_>>(),"{name}: candidates");
        if let Some(values)=case.get("estimate_v").and_then(|v|v.as_array()){for(t,e)in report.transitions.iter().zip(values){match e.as_f64(){Some(v)=>near(t.estimate_v.unwrap(),v),None=>assert!(t.estimate_v.is_none(),"{name}: estimate k{}",t.k)}}}
        if let Some(values)=case.get("widths_v").and_then(|v|v.as_array()){for(c,e)in report.codes[1..report.config.levels-1].iter().zip(values){match e.as_f64(){Some(v)=>near(c.width_v.unwrap(),v),None=>assert!(c.width_v.is_none(),"{name}: width code {}",c.code)}}}
        for(key,reference)in[("endpoint_inl_lsb","endpoint"),("best_fit_inl_lsb","best_fit")]{
            if let Some(values)=case.get(key).and_then(|v|v.as_array()){let r=report.references.iter().find(|r|r.name==reference).unwrap();for(m,e)in r.transition_metrics.iter().zip(values){match e.as_f64(){Some(v)=>near(m.inl_lsb.unwrap(),v),None=>assert!(m.inl_lsb.is_none())}}}
        }
        if let Some(values)=case.get("nominal_dnl_lsb").and_then(|v|v.as_array()){for(c,e)in report.codes[1..report.config.levels-1].iter().zip(values){match e.as_f64(){Some(v)=>near(c.nominal_dnl_lsb.unwrap(),v),None=>assert!(c.nominal_dnl_lsb.is_none())}}}
        for(key,actual)in[("offset_v",report.calibration.offset_v),("gain_span_error_percent",report.calibration.gain_span_error_percent)]{if let Some(expected)=case.get(key){match expected.as_f64(){Some(v)=>near(actual.unwrap(),v),None=>assert!(actual.is_none(),"{name}: {key}")}}}
    }
}

#[test]
fn maximum_code_space_is_bounded_and_sized_correctly() {
    let csv="input_v,code\n0,0\n1,65535\n";
    let config=AuditConfig{bits:16,vmin_v:0.0,vmax_v:1.0};
    let report=analyze(Cursor::new(csv),&config,ReferenceSelection::All,"16bit.csv".into()).unwrap();
    assert_eq!(report.transitions.len(),65_535);
    assert_eq!(report.codes.len(),65_536);
    assert_eq!(report.diagnostics.jumps.len(),1);
    assert!(report.plot_points.len()<=4_096);
}

#[test]
fn svg_escapes_source_and_never_bridges_inl_gaps() {
    let mut report=audit("missing-3bit.csv");report.source="a&b.csv".into();
    let transfer=svg::render_transfer(&report).unwrap();assert!(transfer.contains("a&amp;b.csv"));assert!(!transfer.contains("a&b.csv"));
    let inl=svg::render_inl(&report).unwrap();
    assert!(inl.matches("<polyline class=\"data\"").count()>=6);
    let empty=svg::render_dnl(&audit("reversal-3bit.csv")).unwrap();
    assert!(empty.contains("No DNL values are available"));
}
