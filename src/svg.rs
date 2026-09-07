//! Self-contained accessible SVG rendering.
use crate::{model::*, AuditError};
const W: f64 = 1280.0;
const H: f64 = 800.0;
const L: f64 = 100.0;
const R: f64 = 40.0;
const T: f64 = 100.0;
const B: f64 = 140.0;
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace(char::from(39), "&apos;")
}
fn status(s: SweepStatus) -> &'static str {
    match s {
        SweepStatus::Valid => "valid",
        SweepStatus::Partial => "partial",
        SweepStatus::NonMonotonic => "non_monotonic",
    }
}
fn start(report: &AuditReport, title: &str, desc: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="800" viewBox="0 0 1280 800"><title>{}</title><desc>{}</desc><rect width="1280" height="800" fill="#ffffff"/><style>text{{font-family:system-ui,sans-serif;fill:#17202a;font-size:16px}}.grid{{stroke:#d9e1e8;stroke-width:1}}.axis{{stroke:#17202a;stroke-width:2;fill:none}}.data{{fill:none;stroke-width:2}}</style><text x="100" y="42" font-size="26">{}</text><text x="100" y="72">{} · {}-bit · {} · {}/{} transitions resolved</text>"##,
        esc(title),
        esc(desc),
        esc(title),
        esc(&report.source),
        report.config.bits,
        status(report.status),
        report.coverage.resolved_transition_count,
        report.coverage.expected_transition_count
    )
}
fn axes(mut s: String, xlabel: &str, ylabel: &str) -> String {
    let pw = W - L - R;
    let ph = H - T - B;
    for i in 0..=5 {
        let x = L + pw * i as f64 / 5.0;
        let y = T + ph * i as f64 / 5.0;
        s.push_str(&format!(r#"<line class="grid" x1="{x}" y1="{T}" x2="{x}" y2="{}"/><line class="grid" x1="{L}" y1="{y}" x2="{}" y2="{y}"/>"#,H-B,W-R));
    }
    s.push_str(&format!(r#"<rect class="axis" x="{L}" y="{T}" width="{pw}" height="{ph}"/><text x="{}" y="720" text-anchor="middle">{}</text><text x="28" y="380" text-anchor="middle" transform="rotate(-90 28 380)">{}</text>"#,L+pw/2.0,esc(xlabel),esc(ylabel)));
    s
}
fn tick_labels(s: &mut String, xmin: f64, xmax: f64, ymin: f64, ymax: f64) {
    let pw = W - L - R;
    let ph = H - T - B;
    for i in 0..=5 {
        let x = L + pw * i as f64 / 5.0;
        let y = T + ph * i as f64 / 5.0;
        let xv = xmin + (xmax - xmin) * i as f64 / 5.0;
        let yv = ymax - (ymax - ymin) * i as f64 / 5.0;
        s.push_str(&format!(
            r#"<text x="{x}" y="684" text-anchor="middle">{xv:.4}</text><text x="90" y="{}" text-anchor="end">{yv:.4}</text>"#,
            y + 6.0
        ));
    }
}
fn legend(s: &mut String, refs: &[ReferenceResult], y: f64) {
    let mut x = 720.0;
    for r in refs {
        let (c, d) = color(&r.name);
        s.push_str(&format!(
            r#"<line x1="{x}" y1="{y}" x2="{}" y2="{y}" stroke="{c}" stroke-width="3" stroke-dasharray="{d}"/><text x="{}" y="{}">{}</text>"#,
            x + 36.0,
            x + 43.0,
            y + 6.0,
            esc(&r.name)
        ));
        x += 165.0;
    }
}
fn finish(mut s: String) -> String {
    s.push_str(r#"<text x="100" y="780">Static sweep estimate; sampling bounds exclude source error.</text></svg>"#);
    s
}
fn color(name: &str) -> (&'static str, &'static str) {
    match name {
        "nominal" => ("#0072b2", ""),
        "endpoint" => ("#d55e00", "8 5"),
        _ => ("#009e73", "3 5"),
    }
}
fn polyline(s: &mut String, points: &[(f64, f64)], stroke: &str, dash: &str) {
    if points.is_empty() {
        return;
    }
    let p = points
        .iter()
        .map(|(x, y)| format!("{x:.3},{y:.3}"))
        .collect::<Vec<_>>()
        .join(" ");
    s.push_str(&format!(
        r#"<polyline class="data" points="{p}" stroke="{stroke}" stroke-dasharray="{dash}"/>"#
    ))
}
pub fn render_transfer(report: &AuditReport) -> Result<String, AuditError> {
    let mut s = axes(
        start(
            report,
            "ADC transfer",
            "Observed samples and transition reference lines.",
        ),
        "analogue input (V)",
        "ADC code",
    );
    let xmin = report
        .plot_points
        .first()
        .map(|p| p.input_v)
        .unwrap_or(report.config.vmin_v);
    let xmax = report
        .plot_points
        .last()
        .map(|p| p.input_v)
        .unwrap_or(report.config.vmax_v);
    let xr = (xmax - xmin).max(f64::EPSILON);
    let yr = (report.config.levels - 1) as f64;
    let map = |x: f64, y: f64| {
        (
            L + (x - xmin) / xr * (W - L - R),
            T + (1.0 - y / yr) * (H - T - B),
        )
    };
    let pts: Vec<_> = report
        .plot_points
        .iter()
        .map(|p| map(p.input_v, p.code as f64))
        .collect();
    polyline(&mut s, &pts, "#17202a", "");
    for r in &report.references {
        if let Some(line) = &r.line {
            let (x1, y1) = map(xmin, ((xmin - line.a_v) / line.b_v_per_code).clamp(0.0, yr));
            let (x2, y2) = map(xmax, ((xmax - line.a_v) / line.b_v_per_code).clamp(0.0, yr));
            let (c, d) = color(&r.name);
            polyline(&mut s, &[(x1, y1), (x2, y2)], c, d)
        }
    }
    tick_labels(&mut s, xmin, xmax, 0.0, yr);
    legend(&mut s, &report.references, 120.0);
    s.push_str(r#"<text x="105" y="745">sampled transfer (black); colored lines are transition references</text>"#);
    if report.status == SweepStatus::NonMonotonic {
        s.push_str(&format!(r##"<text x="105" y="735" fill="#b00020">Warning: {} observed code reversal(s); metrics invalidated.</text>"##,report.diagnostics.reversal_count))
    }
    Ok(finish(s))
}
fn range(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (-0.1, 0.1);
    }
    let mut lo = values.iter().copied().fold(0.0, f64::min);
    let mut hi = values.iter().copied().fold(0.0, f64::max);
    if lo == hi {
        lo -= 0.1;
        hi += 0.1
    }
    let pad = (hi - lo) * 0.08;
    (lo - pad, hi + pad)
}
pub fn render_dnl(report: &AuditReport) -> Result<String, AuditError> {
    let mut s = axes(
        start(
            report,
            "Differential nonlinearity",
            "Point DNL only; unresolved audit bins are gaps, not -1 values.",
        ),
        "interior code",
        "DNL (labeled LSB basis)",
    );
    let vals: Vec<f64> = report
        .references
        .iter()
        .flat_map(|r| r.code_metrics.iter().filter_map(|m| m.dnl_lsb))
        .collect();
    let (lo, hi) = range(&vals);
    let n = (report.config.levels - 2).max(1) as f64;
    for code in 1..report.config.levels - 1 {
        if report.codes[code].width_status != "resolved" {
            let x = L + (code - 1) as f64 / n * (W - L - R);
            s.push_str(&format!(
                r##"<line x1="{x}" y1="{T}" x2="{x}" y2="{}" stroke="#f4b6b6" stroke-width="3"/>"##,
                H - B
            ));
        }
    }
    for r in &report.references {
        let (c, d) = color(&r.name);
        let mut seg = vec![];
        for m in &r.code_metrics {
            if let Some(v) = m.dnl_lsb {
                seg.push((
                    L + (m.code - 1) as f64 / n * (W - L - R),
                    T + (hi - v) / (hi - lo) * (H - T - B),
                ))
            } else {
                polyline(&mut s, &seg, c, d);
                seg.clear()
            }
        }
        polyline(&mut s, &seg, c, d)
    }
    tick_labels(&mut s, 1.0, (report.config.levels - 2) as f64, lo, hi);
    legend(&mut s, &report.references, 120.0);
    if lo <= -1.0 && hi >= -1.0 {
        let y = T + (hi + 1.0) / (hi - lo) * (H - T - B);
        s.push_str(&format!(
            r##"<line x1="{L}" y1="{y}" x2="{}" y2="{y}" stroke="#666" stroke-dasharray="2 6"/>"##,
            W - R
        ))
    }
    s.push_str(r#"<text x="105" y="745">−1 is the zero-width truth boundary; pale markers are unresolved or untested.</text>"#);
    Ok(finish(s))
}
pub fn render_inl(report: &AuditReport) -> Result<String, AuditError> {
    if report.references.len() > 1
        && report.references.iter().any(|r| r.name == "nominal")
        && report.references.iter().any(|r| r.name != "nominal")
    {
        let mut s = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="1100" viewBox="0 0 1280 1100"><title>Integral nonlinearity</title><desc>Nominal total transition error is separated from corrected INL. Gaps split lines and cumulative reconstruction restarts locally.</desc><rect width="1280" height="1100" fill="#ffffff"/><style>text{{font-family:system-ui,sans-serif;fill:#17202a;font-size:16px}}.grid{{stroke:#d9e1e8;stroke-width:1}}.axis{{stroke:#17202a;stroke-width:2;fill:none}}.data{{fill:none;stroke-width:2}}</style><text x="100" y="42" font-size="26">Integral nonlinearity</text><text x="100" y="72">{} · {}-bit · {} · {}/{} transitions resolved</text>"##,
            esc(&report.source),
            report.config.bits,
            status(report.status),
            report.coverage.resolved_transition_count,
            report.coverage.expected_transition_count
        );
        let nominal: Vec<&ReferenceResult> = report
            .references
            .iter()
            .filter(|r| r.name == "nominal")
            .collect();
        let corrected: Vec<&ReferenceResult> = report
            .references
            .iter()
            .filter(|r| r.name != "nominal")
            .collect();
        inl_panel(
            &mut s,
            &nominal,
            110.0,
            350.0,
            "total transition error (nominal LSB)",
        );
        inl_panel(
            &mut s,
            &corrected,
            585.0,
            350.0,
            "corrected INL (reference-slope LSB)",
        );
        s.push_str(r#"<text x="570" y="980" text-anchor="middle">transition index</text><text x="100" y="1030">Gaps are unresolved or unobserved transitions; no line crosses them.</text><text x="100" y="1080">Static sweep estimate; sampling bounds exclude source error.</text></svg>"#);
        return Ok(s);
    }
    let mut s=axes(start(report,"Integral nonlinearity","Direct transition residuals; gaps split lines and cumulative reconstruction restarts locally."),"transition index","error / INL (LSB)");
    let vals: Vec<f64> = report
        .references
        .iter()
        .flat_map(|r| r.transition_metrics.iter().filter_map(|m| m.inl_lsb))
        .collect();
    let (lo, hi) = range(&vals);
    let n = (report.config.levels - 2).max(1) as f64;
    for r in &report.references {
        let (c, d) = color(&r.name);
        let mut seg = vec![];
        for m in &r.transition_metrics {
            if let Some(v) = m.inl_lsb {
                seg.push((
                    L + (m.k - 1) as f64 / n * (W - L - R),
                    T + (hi - v) / (hi - lo) * (H - T - B),
                ))
            } else {
                polyline(&mut s, &seg, c, d);
                seg.clear()
            }
        }
        polyline(&mut s, &seg, c, d)
    }
    tick_labels(&mut s, 1.0, (report.config.levels - 1) as f64, lo, hi);
    legend(&mut s, &report.references, 120.0);
    s.push_str(
        r#"<text x="105" y="745">Nominal: total transition error; endpoint/best-fit: INL.</text>"#,
    );
    Ok(finish(s))
}
fn inl_panel(s: &mut String, refs: &[&ReferenceResult], y0: f64, ph: f64, label: &str) {
    let pw = W - L - R;
    let vals: Vec<f64> = refs
        .iter()
        .flat_map(|r| r.transition_metrics.iter().filter_map(|m| m.inl_lsb))
        .collect();
    let (lo, hi) = range(&vals);
    for i in 0..=5 {
        let x = L + pw * i as f64 / 5.0;
        let y = y0 + ph * i as f64 / 5.0;
        let k = 1.0
            + (refs
                .first()
                .map(|r| r.transition_metrics.len())
                .unwrap_or(1) as f64
                - 1.0)
                * i as f64
                / 5.0;
        let v = hi - (hi - lo) * i as f64 / 5.0;
        s.push_str(&format!(r#"<line class="grid" x1="{x}" y1="{y0}" x2="{x}" y2="{}"/><line class="grid" x1="{L}" y1="{y}" x2="{}" y2="{y}"/><text x="{x}" y="{}" text-anchor="middle">{k:.2}</text><text x="90" y="{}" text-anchor="end">{v:.4}</text>"#,y0+ph,W-R,y0+ph+24.0,y+6.0));
    }
    s.push_str(&format!(r#"<rect class="axis" x="{L}" y="{y0}" width="{pw}" height="{ph}"/><text x="28" y="{}" text-anchor="middle" transform="rotate(-90 28 {})">{}</text>"#,y0+ph/2.0,y0+ph/2.0,esc(label)));
    let n = refs
        .first()
        .map(|r| r.transition_metrics.len().saturating_sub(1))
        .unwrap_or(1)
        .max(1) as f64;
    for r in refs {
        let (c, d) = color(&r.name);
        let mut seg = vec![];
        for m in &r.transition_metrics {
            if let Some(v) = m.inl_lsb {
                seg.push((
                    L + (m.k - 1) as f64 / n * pw,
                    y0 + (hi - v) / (hi - lo) * ph,
                ))
            } else {
                polyline(s, &seg, c, d);
                seg.clear()
            }
        }
        polyline(s, &seg, c, d)
    }
    let mut lx = 820.0;
    for r in refs {
        let (c, d) = color(&r.name);
        s.push_str(&format!(r#"<line x1="{lx}" y1="{}" x2="{}" y2="{}" stroke="{c}" stroke-width="3" stroke-dasharray="{d}"/><text x="{}" y="{}">{}</text>"#,y0+20.0,lx+36.0,y0+20.0,lx+43.0,y0+26.0,esc(&r.name)));
        lx += 175.0
    }
}
