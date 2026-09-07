//! Self-contained accessible SVG rendering.
use crate::{model::*,AuditError};
const W:f64=1280.0;const H:f64=800.0;const L:f64=100.0;const R:f64=40.0;const T:f64=100.0;const B:f64=140.0;
fn esc(s:&str)->String{s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;").replace(''',"&apos;")}
fn status(s:SweepStatus)->&'static str{match s{SweepStatus::Valid=>"valid",SweepStatus::Partial=>"partial",SweepStatus::NonMonotonic=>"non_monotonic"}}
fn start(report:&AuditReport,title:&str,desc:&str)->String{format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="800" viewBox="0 0 1280 800"><title>{}</title><desc>{}</desc><rect width="1280" height="800" fill="#ffffff"/><style>text{{font-family:system-ui,sans-serif;fill:#17202a;font-size:16px}}.grid{{stroke:#d9e1e8;stroke-width:1}}.axis{{stroke:#17202a;stroke-width:2;fill:none}}.data{{fill:none;stroke-width:2}}</style><text x="100" y="42" font-size="26">{}</text><text x="100" y="72">{} · {}-bit · {} · {}/{} transitions resolved</text>"#,esc(title),esc(desc),esc(title),esc(&report.source),report.config.bits,status(report.status),report.coverage.resolved_transition_count,report.coverage.expected_transition_count)}
fn axes(mut s:String,xlabel:&str,ylabel:&str)->String{let pw=W-L-R;let ph=H-T-B;for i in 0..=5{let x=L+pw*i as f64/5.0;let y=T+ph*i as f64/5.0;s.push_str(&format!(r#"<line class="grid" x1="{x}" y1="{T}" x2="{x}" y2="{}"/><line class="grid" x1="{L}" y1="{y}" x2="{}" y2="{y}"/>"#,H-B,W-R));}s.push_str(&format!(r#"<rect class="axis" x="{L}" y="{T}" width="{pw}" height="{ph}"/><text x="{}" y="720" text-anchor="middle">{}</text><text x="28" y="380" text-anchor="middle" transform="rotate(-90 28 380)">{}</text>"#,L+pw/2.0,esc(xlabel),esc(ylabel)));s}
fn finish(mut s:String)->String{s.push_str(r#"<text x="100" y="770">Static sweep estimate; sampling bounds exclude source error.</text></svg>"#);s}
fn color(name:&str)->(&'static str,&'static str){match name{"nominal"=>("#0072b2",""),"endpoint"=>("#d55e00","8 5"),_=>("#009e73","3 5")}}
fn polyline(s:&mut String,points:&[(f64,f64)],stroke:&str,dash:&str){if points.is_empty(){return}let p=points.iter().map(|(x,y)|format!("{x:.3},{y:.3}")).collect::<Vec<_>>().join(" ");s.push_str(&format!(r#"<polyline class="data" points="{p}" stroke="{stroke}" stroke-dasharray="{dash}"/>"#))}
pub fn render_transfer(report:&AuditReport)->Result<String,AuditError>{
    let mut s=axes(start(report,"ADC transfer","Observed samples and transition reference lines."),"analogue input (V)","ADC code");
    let xmin=report.plot_points.first().map(|p|p.input_v).unwrap_or(report.config.vmin_v);let xmax=report.plot_points.last().map(|p|p.input_v).unwrap_or(report.config.vmax_v);let xr=(xmax-xmin).max(f64::EPSILON);let yr=(report.config.levels-1)as f64;
    let map=|x:f64,y:f64|(L+(x-xmin)/xr*(W-L-R),T+(1.0-y/yr)*(H-T-B));
    let pts:Vec<_>=report.plot_points.iter().map(|p|map(p.input_v,p.code as f64)).collect();polyline(&mut s,&pts,"#17202a","");
    for r in &report.references{if let Some(line)=&r.line{let(x1,y1)=map(xmin,((xmin-line.a_v)/line.b_v_per_code).clamp(0.0,yr));let(x2,y2)=map(xmax,((xmax-line.a_v)/line.b_v_per_code).clamp(0.0,yr));let(c,d)=color(&r.name);polyline(&mut s,&[(x1,y1),(x2,y2)],c,d)}}
    s.push_str(r#"<text x="105" y="625">sampled transfer (black); colored lines are transition references</text>"#);
    if report.status==SweepStatus::NonMonotonic{s.push_str(&format!(r#"<text x="105" y="650" fill="#b00020">Warning: {} observed code reversal(s); metrics invalidated.</text>"#,report.diagnostics.reversal_count))}
    Ok(finish(s))
}
fn range(values:&[f64])->(f64,f64){if values.is_empty(){return(-0.1,0.1)}let mut lo=values.iter().copied().fold(0.0,f64::min);let mut hi=values.iter().copied().fold(0.0,f64::max);if lo==hi{lo-=0.1;hi+=0.1}let pad=(hi-lo)*0.08;(lo-pad,hi+pad)}
pub fn render_dnl(report:&AuditReport)->Result<String,AuditError>{
    let mut s=axes(start(report,"Differential nonlinearity","Point DNL only; unresolved audit bins are gaps, not -1 values."),"interior code","DNL (labeled LSB basis)");
    let vals:Vec<f64>=report.references.iter().flat_map(|r|r.code_metrics.iter().filter_map(|m|m.dnl_lsb)).collect();let(lo,hi)=range(&vals);let n=(report.config.levels-2).max(1)as f64;
    for r in &report.references{let(c,d)=color(&r.name);let mut seg=vec![];for m in &r.code_metrics{if let Some(v)=m.dnl_lsb{seg.push((L+(m.code-1)as f64/n*(W-L-R),T+(hi-v)/(hi-lo)*(H-T-B)))}else{polyline(&mut s,&seg,c,d);seg.clear()}}polyline(&mut s,&seg,c,d)}
    if lo<=-1.0&&hi>=-1.0{let y=T+(hi+1.0)/(hi-lo)*(H-T-B);s.push_str(&format!(r#"<line x1="{L}" y1="{y}" x2="{}" y2="{y}" stroke="#666" stroke-dasharray="2 6"/>"#,W-R))}
    s.push_str(r#"<text x="105" y="625">−1 is the zero-width truth boundary; blank bins are unresolved or untested.</text>"#);Ok(finish(s))
}
pub fn render_inl(report:&AuditReport)->Result<String,AuditError>{
    let mut s=axes(start(report,"Integral nonlinearity","Direct transition residuals; gaps split lines and cumulative reconstruction restarts locally."),"transition index","error / INL (LSB)");
    let vals:Vec<f64>=report.references.iter().flat_map(|r|r.transition_metrics.iter().filter_map(|m|m.inl_lsb)).collect();let(lo,hi)=range(&vals);let n=(report.config.levels-2).max(1)as f64;
    for r in &report.references{let(c,d)=color(&r.name);let mut seg=vec![];for m in &r.transition_metrics{if let Some(v)=m.inl_lsb{seg.push((L+(m.k-1)as f64/n*(W-L-R),T+(hi-v)/(hi-lo)*(H-T-B)))}else{polyline(&mut s,&seg,c,d);seg.clear()}}polyline(&mut s,&seg,c,d)}
    s.push_str(r#"<text x="105" y="625">Nominal: total transition error; endpoint/best-fit: INL.</text>"#);Ok(finish(s))
}
