//! Width, reference-line and calibration analysis.
use std::io::Read;
use crate::{config::AuditConfig, input::read_samples, model::*, transitions::ObservationBuilder, AuditError};

#[derive(Clone, Copy, Debug)]
pub enum ReferenceSelection { Nominal, Endpoint, BestFit, All }
impl ReferenceSelection {
    pub fn kinds(self)->Vec<ReferenceKind>{match self{Self::Nominal=>vec![ReferenceKind::Nominal],Self::Endpoint=>vec![ReferenceKind::Endpoint],Self::BestFit=>vec![ReferenceKind::BestFit],Self::All=>vec![ReferenceKind::Nominal,ReferenceKind::Endpoint,ReferenceKind::BestFit]}}
}

pub fn analyze<R:Read>(reader:R, config:&AuditConfig, selection:ReferenceSelection, source:String)->Result<AuditReport,AuditError>{
    let cfg=config.validate()?; let mut builder=ObservationBuilder::new(&cfg);
    read_samples(reader,cfg.levels,|s|builder.push(s))?;
    let obs=builder.finish()?;
    let mut codes=Vec::with_capacity(cfg.levels);
    let min=obs.coverage.min_observed_code; let max=obs.coverage.max_observed_code;
    for code in 0..cfg.levels {
        let observation_status=if obs.seen[code]>0{"observed"}else if min.is_some_and(|x|code>=x)&&max.is_some_and(|x|code<=x){"missing_candidate"}else{"untested"}.to_string();
        if code==0||code==cfg.levels-1 {
            codes.push(CodeRow{code,samples:obs.seen[code],observation_status,width_status:"saturation".into(),width_v:None,width_lower_v:None,width_upper_v:None,nominal_dnl_lsb:None,nominal_dnl_lower_lsb:None,nominal_dnl_upper_lsb:None}); continue;
        }
        let left=&obs.transitions[code-1]; let right=&obs.transitions[code];
        let (width_status,width_v,lower,upper)=if obs.status==SweepStatus::NonMonotonic {
            ("invalidated",None,None,None)
        } else {
            let point=left.estimate_v.zip(right.estimate_v).map(|(a,b)|b-a);
            let bounds=left.bracket.zip(right.bracket).map(|(a,b)|((b.lower_v-a.upper_v).max(0.0),b.upper_v-a.lower_v));
            let status=if point.is_some(){"resolved"}else if left.bracket.is_some()&&right.bracket.is_some(){"unresolved"}else{"unobserved"};
            (status,point,bounds.map(|x|x.0),bounds.map(|x|x.1))
        };
        if upper.is_some_and(|u|u<0.0) {return Err(AuditError::validation("internally inconsistent width bound"));}
        codes.push(CodeRow{code,samples:obs.seen[code],observation_status,width_status:width_status.into(),width_v,
            width_lower_v:lower,width_upper_v:upper,nominal_dnl_lsb:width_v.map(|w|w/cfg.nominal_lsb_v-1.0),
            nominal_dnl_lower_lsb:lower.map(|w|w/cfg.nominal_lsb_v-1.0),nominal_dnl_upper_lsb:upper.map(|w|w/cfg.nominal_lsb_v-1.0)});
    }
    let calibration=calibration(&obs.transitions,&cfg,obs.status);
    let references=selection.kinds().into_iter().map(|k|reference(k,&obs.transitions,&codes,&cfg,obs.status)).collect();
    Ok(AuditReport{schema_version:1,source,config:ReportConfig{bits:config.bits,vmin_v:config.vmin_v,vmax_v:config.vmax_v,levels:cfg.levels,nominal_lsb_v:cfg.nominal_lsb_v,quantizer:"floor".into()},
        status:obs.status,sample_count:obs.sample_count,coverage:obs.coverage,quality:obs.quality,calibration,references,
        transitions:obs.transitions,codes,diagnostics:obs.diagnostics,assumptions:vec![
            "increasing deterministic sweep".into(),"floor transition convention".into(),
            "sampling bounds exclude input-source error and noise".into(),"fitted-line bounds are not estimated".into(),
            "unobserved codes are not proven missing".into(),"saturation widths are excluded".into()],plot_points:obs.plot_points})
}

fn unavailable_cal(reason:&str)->Calibration{Calibration{availability:"unavailable".into(),reason:Some(reason.into()),basis:"first_and_last_internal_transition".into(),gain_convention:"input_span".into(),offset_v:None,offset_lsb:None,gain_span_error_v:None,gain_span_error_lsb:None,gain_span_error_percent:None}}
fn calibration(t:&[Transition],cfg:&crate::config::DerivedConfig,status:SweepStatus)->Calibration{
    if status==SweepStatus::NonMonotonic{return unavailable_cal("non_monotonic_sweep")}
    let Some(first)=t.first().and_then(|x|x.estimate_v) else{return unavailable_cal("endpoint_not_resolved")};
    let Some(last)=t.last().and_then(|x|x.estimate_v) else{return unavailable_cal("endpoint_not_resolved")};
    let ideal=(cfg.levels-2) as f64*cfg.nominal_lsb_v; let span=last-first;
    let offset=first-(cfg.source.vmin_v+cfg.nominal_lsb_v); let gain=span-ideal;
    let values=[offset,offset/cfg.nominal_lsb_v,gain,gain/cfg.nominal_lsb_v,100.0*(span/ideal-1.0)];
    if values.iter().any(|v|!v.is_finite()){return unavailable_cal("numerical_failure")}
    Calibration{availability:"available".into(),reason:None,basis:"first_and_last_internal_transition".into(),gain_convention:"input_span".into(),offset_v:Some(values[0]),offset_lsb:Some(values[1]),gain_span_error_v:Some(values[2]),gain_span_error_lsb:Some(values[3]),gain_span_error_percent:Some(values[4])}
}
fn line(kind:ReferenceKind,t:&[Transition],cfg:&crate::config::DerivedConfig,status:SweepStatus)->Result<ReferenceLine,&'static str>{
    if status==SweepStatus::NonMonotonic{return Err("non_monotonic_sweep")}
    let resolved:Vec<(f64,f64)>=t.iter().filter_map(|x|x.estimate_v.map(|v|(x.k as f64,v))).collect();
    match kind {
        ReferenceKind::Nominal=>{
            if resolved.is_empty(){return Err("no_resolved_transitions")}
            Ok(ReferenceLine{a_v:cfg.source.vmin_v,b_v_per_code:cfg.nominal_lsb_v,anchor_k:0.0,anchor_v:cfg.source.vmin_v,intercept_shift_v:0.0})
        }
        ReferenceKind::Endpoint=>{
            let Some(first)=t.first().and_then(|x|x.estimate_v)else{return Err("endpoint_not_resolved")};
            let Some(last)=t.last().and_then(|x|x.estimate_v)else{return Err("endpoint_not_resolved")};
            let b=(last-first)/(cfg.levels-2) as f64;let a=first-b;
            if !a.is_finite()||!b.is_finite()||b<=0.0{return Err("numerical_failure")}
            Ok(ReferenceLine{a_v:a,b_v_per_code:b,anchor_k:1.0,anchor_v:first,intercept_shift_v:a-cfg.source.vmin_v})
        }
        ReferenceKind::BestFit=>{
            if resolved.len()<3{return Err("insufficient_resolved_transitions")}
            let n=resolved.len() as f64;let km=resolved.iter().map(|x|x.0).sum::<f64>()/n;let tm=resolved.iter().map(|x|x.1).sum::<f64>()/n;
            let skk=resolved.iter().map(|x|(x.0-km)*(x.0-km)).sum::<f64>();
            let skt=resolved.iter().map(|x|(x.0-km)*(x.1-tm)).sum::<f64>();
            let b=skt/skk;let a=tm-b*km;
            if !a.is_finite()||!b.is_finite()||b<=0.0{return Err("numerical_failure")}
            Ok(ReferenceLine{a_v:a,b_v_per_code:b,anchor_k:km,anchor_v:tm,intercept_shift_v:a-cfg.source.vmin_v})
        }
    }
}
fn reference(kind:ReferenceKind,t:&[Transition],codes:&[CodeRow],cfg:&crate::config::DerivedConfig,status:SweepStatus)->ReferenceResult{
    let expected_t=cfg.levels-1;let expected_w=cfg.levels-2;
    let line_result=line(kind,t,cfg,status);
    let Ok(ref_line)=line_result else {
        return ReferenceResult{name:kind.name().into(),availability:"unavailable".into(),reason:Some(line_result.err().unwrap_or("numerical_failure").into()),line:None,
            lsb_basis:if kind==ReferenceKind::Nominal{"nominal"}else{"reference_slope"}.into(),display_quantity:if kind==ReferenceKind::Nominal{"total_transition_error"}else{"inl"}.into(),
            coverage:RefCoverage{scope:"partial".into(),evaluated_transition_count:0,evaluated_width_count:0,expected_transition_count:expected_t,expected_width_count:expected_w},
            dnl_summary:None,inl_summary:None,transition_metrics:(1..=expected_t).map(|k|TransitionMetric{k,inl_lsb:None,cumulative_inl_lsb:None,cumulative_run_id:None,cumulative_run_start:false}).collect(),
            code_metrics:(1..=expected_w).map(|code|CodeMetric{code,dnl_lsb:None}).collect()};
    };
    let mut tm=Vec::with_capacity(expected_t);let mut run=0u32;let mut prev_k=None;let mut cumulative=0.0;
    for tr in t {
        let direct=tr.estimate_v.map(|v|(v-(ref_line.anchor_v+ref_line.b_v_per_code*(tr.k as f64-ref_line.anchor_k)))/ref_line.b_v_per_code);
        let start=direct.is_some()&&prev_k!=Some(tr.k-1);
        if let Some(d)=direct { if start {if prev_k.is_some(){run+=1} cumulative=d;} else {let dnl=codes[tr.k-1].width_v.map(|w|w/ref_line.b_v_per_code-1.0).unwrap_or(0.0);cumulative+=dnl;} prev_k=Some(tr.k); }
        else {prev_k=None;}
        tm.push(TransitionMetric{k:tr.k,inl_lsb:direct,cumulative_inl_lsb:direct.map(|_|cumulative),cumulative_run_id:direct.map(|_|run),cumulative_run_start:start});
    }
    let cm:Vec<_>=(1..=expected_w).map(|code|CodeMetric{code,dnl_lsb:codes[code].width_v.map(|w|w/ref_line.b_v_per_code-1.0)}).collect();
    let et=tm.iter().filter(|x|x.inl_lsb.is_some()).count();let ew=cm.iter().filter(|x|x.dnl_lsb.is_some()).count();
    ReferenceResult{name:kind.name().into(),availability:"available".into(),reason:None,line:Some(ref_line),
        lsb_basis:if kind==ReferenceKind::Nominal{"nominal"}else{"reference_slope"}.into(),display_quantity:if kind==ReferenceKind::Nominal{"total_transition_error"}else{"inl"}.into(),
        coverage:RefCoverage{scope:if et==expected_t&&ew==expected_w{"full_range"}else{"partial"}.into(),evaluated_transition_count:et,evaluated_width_count:ew,expected_transition_count:expected_t,expected_width_count:expected_w},
        dnl_summary:dnl_summary(&cm),inl_summary:inl_summary(&tm),transition_metrics:tm,code_metrics:cm}
}
fn dnl_summary(v:&[CodeMetric])->Option<DnlSummary>{
    let vals:Vec<_>=v.iter().filter_map(|x|x.dnl_lsb.map(|n|(x.code,n))).collect();let &(c0,x0)=vals.first()?;
    let(mut min,mut minc,mut max,mut maxc,mut ma,mut mac)=(x0,c0,x0,c0,x0.abs(),c0);
    for &(c,x) in &vals[1..]{if x<min{min=x;minc=c}if x>max{max=x;maxc=c}if x.abs()>ma{ma=x.abs();mac=c}}
    Some(DnlSummary{count:vals.len(),min_lsb:min,min_code:minc,max_lsb:max,max_code:maxc,max_abs_lsb:ma,max_abs_code:mac})
}
fn inl_summary(v:&[TransitionMetric])->Option<InlSummary>{
    let vals:Vec<_>=v.iter().filter_map(|x|x.inl_lsb.map(|n|(x.k,n))).collect();let &(k0,x0)=vals.first()?;
    let(mut min,mut mink,mut max,mut maxk,mut ma,mut mak,mut sum)=(x0,k0,x0,k0,x0.abs(),k0,0.0);
    for &(k,x) in &vals{if x<min{min=x;mink=k}if x>max{max=x;maxk=k}if x.abs()>ma{ma=x.abs();mak=k}sum+=x*x}
    Some(InlSummary{count:vals.len(),min_lsb:min,min_k:mink,max_lsb:max,max_k:maxk,max_abs_lsb:ma,max_abs_k:mak,rms_lsb:(sum/vals.len() as f64).sqrt()})
}
