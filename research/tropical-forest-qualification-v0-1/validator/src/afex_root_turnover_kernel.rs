// SPDX-License-Identifier: Apache-2.0 OR MIT
//! AFEX fine-root stock/productivity/turnover support and process diagnostics.
//!
//! This is a bounded benchmark verifier, not a root dynamics model or causal estimator.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as FmtWrite;

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}
fn number_at(value: &Value, key: &str) -> Result<f64, String> {
    let n = value.get(key).and_then(Value::as_f64)
        .ok_or_else(|| format!("{key} must be a number"))?;
    if !n.is_finite() { return Err(format!("{key} must be finite")); }
    Ok(n)
}
fn same_number(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}
fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut out = String::with_capacity(64);
    for byte in digest { write!(&mut out, "{byte:02x}").expect("String write cannot fail"); }
    out
}
fn expected_support(period: &str) -> Result<(&'static str, &'static str, &'static str), String> {
    match period {
        "year1" => Ok(("2017-08", "2017-08", "2018-09")),
        "year2" => Ok(("2019-09", "2018-09", "2019-09")),
        _ => Err(format!("unsupported AFEX root period: {period}")),
    }
}
#[derive(Clone, Debug)]
struct Estimate {
    stock: f64,
    productivity: f64,
    turnover: f64,
    lifespan_days: Option<f64>,
}
fn parse_estimate(record: &Value, expected_plot: &str, expected_depth: (u64,u64), label: &str) -> Result<Estimate, String> {
    let plot_id = string_at(record, "plot_id")?;
    if plot_id != expected_plot { return Err(format!("{label} plot identity mismatch")); }
    let depth = record.get("depth_cm").and_then(Value::as_array)
        .ok_or_else(|| format!("{label}.depth_cm must be a two-item array"))?;
    if depth.len()!=2 { return Err(format!("{label}.depth_cm must have two entries")); }
    let d0=depth[0].as_u64().ok_or_else(||format!("{label} depth start must be integer"))?;
    let d1=depth[1].as_u64().ok_or_else(||format!("{label} depth end must be integer"))?;
    if (d0,d1)!=expected_depth { return Err(format!("{label} depth support mismatch")); }
    let stock_record=record.get("stock").ok_or_else(||format!("{label}.stock is required"))?;
    let productivity_record=record.get("productivity").ok_or_else(||format!("{label}.productivity is required"))?;
    let stock=parse_metric(stock_record,label,"stock_value","unit")?;
    let productivity=parse_metric(productivity_record,label,"productivity_value","unit")?;
    let turnover=number_at(record,"turnover_value")?;
    if turnover<0.0 { return Err(format!("{label} turnover must be non-negative")); }
    let expected_turnover=productivity/stock;
    if !same_number(turnover,expected_turnover,1e-9) {
        return Err(format!("{label} turnover does not equal productivity / stock"));
    }
    let lifespan=if turnover>0.0 {Some(365.25/turnover)} else {None};
    Ok(Estimate{stock,productivity,turnover,lifespan_days:lifespan})
}
pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value,String> {
    if string_at(manifest,"schema_version")?!="0.1.0"
        || string_at(manifest,"benchmark_id")?!="LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || string_at(manifest,"corpus_id")?!="AFEX-ROOT-TURNOVER-001" {
        return Err("AFEX root-turnover corpus identity mismatch".to_string());
    }
    let process=manifest.get("process_definition").ok_or_else(||"process_definition required".to_string())?;
    if string_at(process,"turnover_definition")?!="annual productivity divided by fine-root stock"
        || string_at(process,"turnover_unit")?!="yr-1"
        || string_at(process,"stock_quantity")?!="fine-root standing biomass"
        || string_at(process,"productivity_quantity")?!="annual fine-root biomass production from in-growth cores"
        || number_at(process,"root_diameter_max_mm")? != 2.0 {
        return Err("AFEX root process semantics invalid".to_string());
    }
    let process_depth=process.get("root_depth_cm").and_then(Value::as_array)
        .ok_or_else(||"process_definition.root_depth_cm must be an array".to_string())?;
    if process_depth.len()!=2 || process_depth[0].as_u64()!=Some(0)
        || process_depth[1].as_u64()!=Some(30) {
        return Err("AFEX root process depth must be 0-30 cm".to_string());
    }
    let source=manifest.get("source").ok_or_else(||"source metadata required".to_string())?;
    if string_at(source,"doi")?!="10.1007/s11104-026-08303-2"
        || string_at(source,"dataset_doi")?!="10.5285/b3a55011-bf46-40f5-8850-86dc8bc4c85d"
        || string_at(source,"root_stock_data_access")?!="AVAILABLE_FROM_CORRESPONDING_AUTHOR_ON_REASONABLE_REQUEST"
        || source.get("raw_data_ingested").and_then(Value::as_bool)!=Some(false)
        || string_at(manifest,"published_summary_status")?!="TRANSCRIBED_FROM_PAPER_NOT_RECOMPUTED_FROM_RAW_DATA" {
        return Err("AFEX root-turnover source or claim-ceiling metadata invalid".to_string());
    }
    let tol=manifest.get("tolerances").ok_or_else(||"tolerances required".to_string())?;
    let stock_tol=number_at(tol,"stock_absolute")?;
    let prod_tol=number_at(tol,"productivity_absolute")?;
    let turnover_tol=number_at(tol,"turnover_absolute")?;
    if stock_tol<0.0||prod_tol<0.0||turnover_tol<0.0 {
        return Err("comparison tolerances must be non-negative".to_string());
    }
    let rows=manifest.get("cases").and_then(Value::as_array).ok_or_else(||"cases must be array".to_string())?;
    let expected_count=manifest.get("expected_case_count").and_then(Value::as_u64).ok_or_else(||"expected_case_count must be integer".to_string())?;
    if rows.len()!=expected_count as usize || rows.is_empty() {
        return Err("case count does not match expected_case_count".to_string());
    }
    let mut ids=BTreeSet::new();
    let mut results=Vec::with_capacity(rows.len());
    for (index,case) in rows.iter().enumerate() {
        let id=string_at(case,"case_id")?;
        if id.trim().is_empty()||!ids.insert(id.clone()) { return Err("case IDs must be non-empty and unique".to_string()); }
        let period=string_at(case,"period")?;
        if number_at(case,"root_diameter_max_mm")? != 2.0 {
            return Err(format!("case {id} root diameter threshold mismatch"));
        }
        let plot_id=string_at(case.get("reference").ok_or_else(||"reference required".to_string())?,"plot_id")?;
        let depthv=case.get("depth_cm").and_then(Value::as_array).ok_or_else(||"case depth_cm must be array".to_string())?;
        if depthv.len()!=2 { return Err("case depth_cm must have two entries".to_string()); }
        let depth=(depthv[0].as_u64().ok_or_else(||"depth start must be integer".to_string())?,depthv[1].as_u64().ok_or_else(||"depth end must be integer".to_string())?);
        if depth.0>=depth.1 || depth.1>200 { return Err("invalid depth interval".to_string()); }
        let reference=parse_series_wrapper(case.get("reference").unwrap(),&period,&plot_id,depth,"reference")?;
        let candidate_value=case.get("candidate").ok_or_else(||"candidate required".to_string())?;
        let candidate_plot=string_at(candidate_value,"plot_id")?;
        let candidate_depth=case.get("candidate").and_then(|v|v.get("depth_cm")).and_then(Value::as_array);
        let candidate_depth_matches=candidate_depth.map(|v|v.len()==2&&v[0].as_u64()==Some(depth.0)&&v[1].as_u64()==Some(depth.1)).unwrap_or(true);
        let candidate=if candidate_plot==plot_id && candidate_depth_matches {
            match parse_series_wrapper(candidate_value,&period,&plot_id,depth,"candidate") {
                Ok(v)=>Some(v),
                Err(e) if e.contains("date does not match")||e.contains("interval does not match")||e.contains("depth support mismatch")||e.contains("plot identity mismatch")=>None,
                Err(e)=>return Err(e)
            }
        } else {None};
        let disposition=if candidate.is_none() {
            "SUPPORT_MISMATCH_REFUSED"
        } else {
            let candidate=candidate.as_ref().unwrap();
            if (reference.stock-candidate.stock).abs()>stock_tol {
                "STATE_MISMATCH"
            } else if (reference.productivity-candidate.productivity).abs()>prod_tol
                || (reference.turnover-candidate.turnover).abs()>turnover_tol {
                "STOCK_MATCH_WITH_PROCESS_MISMATCH"
            } else {
                "PROCESS_AND_STOCK_MATCH"
            }
        };
        let expected=string_at(case,"expected_disposition")?;
        if expected!=disposition { return Err(format!("case {id} disposition {disposition} does not match expected {expected}")); }
        let turnover_error=candidate.as_ref().map(|c|(c.turnover-reference.turnover).abs());
        let candidate_stock=candidate.as_ref().map(|c|c.stock);
        let candidate_productivity=candidate.as_ref().map(|c|c.productivity);
        let lifespan_error=candidate.as_ref().and_then(|c|match (reference.lifespan_days,c.lifespan_days){(Some(a),Some(b))=>Some((b-a).abs()),_=>None});
        results.push(json!({
            "case_id":id,
            "disposition":disposition,
            "reference":{"stock":reference.stock,"productivity":reference.productivity,"turnover_per_year":reference.turnover,"nominal_lifespan_days":reference.lifespan_days},
            "candidate":{"stock":candidate_stock,"productivity":candidate_productivity},
            "stock_error":candidate.as_ref().map(|c|(c.stock-reference.stock).abs()),
            "productivity_error":candidate.as_ref().map(|c|(c.productivity-reference.productivity).abs()),
            "turnover_error":turnover_error,
            "lifespan_error_days":lifespan_error
        }));
        let _=index;
    }
    let summary=manifest.get("published_summary_effects").and_then(Value::as_array).ok_or_else(||"published_summary_effects must be array".to_string())?;
    let pct_tol=number_at(tol,"published_percentage_points")?;
    let mut published_checks=Vec::new();
    for item in summary {
        let id=string_at(item,"effect_id")?;
        let minus=number_at(item,"minus_mean")?;
        let plus=number_at(item,"plus_mean")?;
        let minus_se=number_at(item,"minus_se")?;
        let plus_se=number_at(item,"plus_se")?;
        if minus<=0.0 || plus<=0.0 || minus_se<=0.0 || plus_se<=0.0 {
            return Err(format!("published summary {id} requires positive means and standard errors"));
        }
        for p_key in ["p_value","reported_p_value","p_value_less_than"] {
            if let Some(p)=item.get(p_key).and_then(Value::as_f64) {
                if !(0.0..=1.0).contains(&p) { return Err(format!("published summary {id} has invalid {p_key}")); }
            }
        }
        let reported=if let Some(p)=item.get("reported_change_percent").and_then(Value::as_f64) {Some(p)} else {None};
        let interpretation=string_at(item,"interpretation")?;
        let direction=string_at(item,"direction")?;
        if direction=="NO_SIGNIFICANT_DIFFERENCE"
            && interpretation!="NON_SIGNIFICANCE_IS_NOT_A_ZERO_EFFECT" {
            return Err(format!("published summary {id} must preserve non-significance without claiming zero effect"));
        }
        if interpretation=="NON_SIGNIFICANCE_IS_NOT_A_ZERO_EFFECT" && direction!="NO_SIGNIFICANT_DIFFERENCE" {
            return Err(format!("published summary {id} mislabels non-significance"));
        }
        if let Some(reported)=reported {
            if minus<=0.0 { return Err(format!("published summary {id} denominator must be positive")); }
            let actual=(plus/minus-1.0)*100.0;
            if (actual-reported).abs()>pct_tol {
                return Err(format!("published summary {id} percentage arithmetic mismatch"));
            }
            published_checks.push(json!({"effect_id":id,"computed_change_percent":actual,"reported_rounded_change_percent":reported,"arithmetic_match":true,"p_value_recomputed":false}));
        } else {
            published_checks.push(json!({"effect_id":id,"computed_change_percent":null,"reported_p_value":item.get("reported_p_value").cloned().unwrap_or(Value::Null),"effect_interpretation":"NON_SIGNIFICANT_DOES_NOT_ESTABLISH_ZERO","p_value_recomputed":false}));
        }
    }
    let passed=results.len()==expected_count as usize && published_checks.len()==summary.len();
    Ok(json!({
        "schema_version":"0.1.0",
        "benchmark_id":"LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "corpus_id":"AFEX-ROOT-TURNOVER-001",
        "result":if passed {"PASS_SYNTHETIC_ROOT_TURNOVER"} else {"FAIL"},
        "source_manifest_sha256":sha256_hex(raw),
        "case_count":results.len(),
        "published_summary_effect_count":published_checks.len(),
        "case_results":results,
        "published_summary_arithmetic_checks":published_checks,
        "published_summary_data_status":"TRANSCRIBED_NOT_RECOMPUTED_FROM_RAW",
        "root_stock_data_access":"CORRESPONDING_AUTHOR_REQUEST_STATED_BY_2026_PAPER",
        "raw_data_ingested":false,
        "empirical_root_turnover_reanalysis":"NOT_ESTABLISHED",
        "claim_ceiling":string_at(manifest,"claim_ceiling")?
    }))
}

fn parse_series_wrapper(series:&Value,period:&str,plot:&str,depth:(u64,u64),label:&str)->Result<Estimate,String>{
    let (stock_date,start,end)=expected_support(period)?;
    let stock=series.get("stock").ok_or_else(||format!("{label}.stock required"))?;
    let prod=series.get("productivity").ok_or_else(||format!("{label}.productivity required"))?;
    if string_at(stock,"date")?!=stock_date { return Err(format!("{label} stock date does not match expected period")); }
    if string_at(prod,"start")?!=start||string_at(prod,"end")?!=end { return Err(format!("{label} productivity interval does not match expected period")); }
    if string_at(stock,"unit")?!="Mg C ha-1"||string_at(prod,"unit")?!="Mg C ha-1 yr-1" { return Err(format!("{label} unit mismatch")); }
    let stock_value=number_at(stock,"value")?;
    let productivity_value=number_at(prod,"value")?;
    if stock_value<=0.0||productivity_value<0.0 { return Err(format!("{label} invalid positive stock / non-negative productivity")); }
    let turnover=productivity_value/stock_value;
    let declared_depth=series.get("depth_cm").and_then(Value::as_array)
        .ok_or_else(||format!("{label}.depth_cm is required"))?;
    if declared_depth.len()!=2 || declared_depth[0].as_u64()!=Some(depth.0)
        || declared_depth[1].as_u64()!=Some(depth.1) {
        return Err(format!("{label} depth support mismatch"));
    }
    let record=json!({
        "plot_id":series.get("plot_id").and_then(Value::as_str).unwrap_or(""),
        "depth_cm":series.get("depth_cm").cloned().unwrap_or(Value::Null),
        "stock":{"stock_value":stock_value,"unit":"Mg C ha-1"},
        "productivity":{"productivity_value":productivity_value,"unit":"Mg C ha-1 yr-1"},
        "turnover_value":turnover
    });
    parse_estimate(&record,plot,depth,label)
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE:&str=include_str!("../../cases/afex-root-turnover-synthetic-v0.1.json");
    #[test]
    fn root_turnover_corpus_checks_four_cases_and_published_arithmetic(){
        let m:Value=serde_json::from_str(FIXTURE).expect("fixture parses");
        let r=validate_corpus(FIXTURE.as_bytes(),&m).expect("receipt");
        assert_eq!(r["result"],"PASS_SYNTHETIC_ROOT_TURNOVER");
        assert_eq!(r["case_count"],4);
        assert_eq!(r["published_summary_effect_count"],6);
        assert_eq!(r["case_results"][1]["disposition"],"STOCK_MATCH_WITH_PROCESS_MISMATCH");
    }
    #[test]
    fn same_stock_different_turnover_is_detected(){
        let m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let r=validate_corpus(FIXTURE.as_bytes(),&m).expect("receipt");
        assert_eq!(r["case_results"][1]["reference"]["stock"],r["case_results"][1]["candidate"]["stock"]);
        assert_ne!(r["case_results"][1]["reference"]["turnover_per_year"],r["case_results"][1]["candidate"]["turnover_per_year"]);
    }
    #[test]
    fn wrong_depth_is_refused(){
        let mut m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        m["cases"][3]["candidate"]["depth_cm"]=json!([0,10]);
        let raw=serde_json::to_vec(&m).expect("serialize");
        let r=validate_corpus(&raw,&m).expect("refusal receipt");
        assert_eq!(r["case_results"][3]["disposition"],"SUPPORT_MISMATCH_REFUSED");
    }
    #[test]
    fn negative_productivity_is_rejected(){
        let mut m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        m["cases"][0]["candidate"]["productivity"]["value"]=json!(-1.0);
        let raw=serde_json::to_vec(&m).expect("serialize");
        assert!(validate_corpus(&raw,&m).is_err());
    }
    #[test]
    fn non_positive_stock_is_rejected(){
        let mut m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        m["cases"][0]["candidate"]["stock"]["value"]=json!(0.0);
        let raw=serde_json::to_vec(&m).expect("serialize");
        assert!(validate_corpus(&raw,&m).is_err());
    }
    #[test]
    fn unsupported_stock_date_is_rejected(){
        let mut m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        m["cases"][0]["candidate"]["stock"]["date"]=json!("2018-08");
        let raw=serde_json::to_vec(&m).expect("serialize");
        assert!(validate_corpus(&raw,&m).is_err());
    }
    #[test]
    fn root_stock_depth_cannot_be_silently_relabelled(){
        let mut m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        m["cases"][0]["candidate"]["depth_cm"]=json!([0,10]);
        let raw=serde_json::to_vec(&m).expect("serialize");
        assert!(validate_corpus(&raw,&m).is_err());
    }
    #[test]
    fn false_raw_data_ingestion_claim_is_rejected(){
        let mut m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        m["source"]["raw_data_ingested"]=json!(true);
        let raw=serde_json::to_vec(&m).expect("serialize");
        assert!(validate_corpus(&raw,&m).is_err());
    }
    #[test]
    fn non_significance_cannot_be_relabelled_as_zero_effect(){
        let mut m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        m["published_summary_effects"][4]["interpretation"]=json!("ZERO_EFFECT_ESTABLISHED");
        let raw=serde_json::to_vec(&m).expect("serialize");
        assert!(validate_corpus(&raw,&m).is_err());
    }
}
