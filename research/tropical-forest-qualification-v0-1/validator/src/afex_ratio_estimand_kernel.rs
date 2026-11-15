// SPDX-License-Identifier: Apache-2.0 OR MIT
//! AFEX plot-paired ratio estimand and covariance-aware uncertainty benchmark.
//!
//! The delta method is first-order and approximate; this module is not a Fieller solver.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;

#[derive(Clone, Debug)]
struct PlotPair {
    plot_id: String,
    block_id: String,
    treatment_id: String,
    stock: f64,
    productivity: f64,
    turnover: f64,
    lifespan_days: Option<f64>,
}

#[derive(Clone, Debug)]
struct Summary {
    n: usize,
    mean_stock: f64,
    mean_productivity: f64,
    stock_sample_variance: f64,
    productivity_sample_variance: f64,
    sample_covariance: f64,
    mean_plot_turnover: f64,
    pooled_ratio_of_sums: f64,
    mean_plot_lifespan_days: Option<f64>,
    inverse_mean_turnover_days: Option<f64>,
    mean_plot_turnover_standard_error: f64,
    mean_plot_lifespan_standard_error: Option<f64>,
    ratio_of_means_delta_variance: f64,
    ratio_of_means_delta_standard_error: f64,
}

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}
fn number_at(value: &Value, key: &str) -> Result<f64, String> {
    let n = value.get(key).and_then(Value::as_f64)
        .ok_or_else(|| format!("{key} must be a finite number"))?;
    if !n.is_finite() { return Err(format!("{key} must be finite")); }
    Ok(n)
}
fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut out = String::with_capacity(64);
    for byte in digest { write!(&mut out, "{byte:02x}").expect("String write cannot fail"); }
    out
}
fn mean(values: &[f64]) -> Result<f64, String> {
    if values.is_empty() { return Err("cannot calculate mean of empty values".to_string()); }
    Ok(values.iter().sum::<f64>() / values.len() as f64)
}
fn sample_variance(values: &[f64], avg: f64) -> Result<f64, String> {
    if values.len() < 2 { return Err("at least two plot pairs are required".to_string()); }
    Ok(values.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / (values.len() - 1) as f64)
}
fn sample_covariance(x: &[f64], y: &[f64], mean_x: f64, mean_y: f64) -> Result<f64, String> {
    if x.len() != y.len() || x.len() < 2 {
        return Err("paired covariance requires at least two equally sized vectors".to_string());
    }
    Ok(x.iter().zip(y.iter()).map(|(a,b)| (a-mean_x)*(b-mean_y)).sum::<f64>() / (x.len()-1) as f64)
}
fn compare_optional_number(expected: &Value, key: &str, actual: Option<f64>, tolerance: f64) -> Result<(), String> {
    match (actual, expected.get(key)) {
        (Some(value), Some(expected_value)) => {
            let expected_number=expected_value.as_f64()
                .ok_or_else(||format!("expected_summary.{key} must be numeric"))?;
            if !value.is_finite() || (value-expected_number).abs()>tolerance {
                return Err(format!("expected_summary.{key} mismatch: actual={value}, expected={expected_number}"));
            }
        }
        (None, Some(Value::Null)) => {}
        (None, _) => return Err(format!("expected_summary.{key} must be null when the estimand is undefined")),
        (Some(_), None) => return Err(format!("expected_summary.{key} is required")),
    }
    Ok(())
}
fn expected_support(period: &str) -> Result<(&'static str, &'static str, &'static str), String> {
    match period {
        "year1" => Ok(("2017-08", "2017-08", "2018-09")),
        "year2" => Ok(("2019-09", "2018-09", "2019-09")),
        _ => Err(format!("unsupported AFEX period {period}")),
    }
}
fn read_depth(value: &Value, label: &str) -> Result<(u64,u64), String> {
    let d = value.get("depth_cm").and_then(Value::as_array)
        .ok_or_else(|| format!("{label}.depth_cm must be an array"))?;
    if d.len()!=2 { return Err(format!("{label}.depth_cm must contain two values")); }
    let start=d[0].as_u64().ok_or_else(||format!("{label} depth start must be integer"))?;
    let end=d[1].as_u64().ok_or_else(||format!("{label} depth end must be integer"))?;
    if start>=end { return Err(format!("{label} depth interval is invalid")); }
    Ok((start,end))
}
fn validate_pair(obs: &Value, period: &str, expected_depth:(u64,u64)) -> Result<Option<PlotPair>,String> {
    let plot_id=string_at(obs,"plot_id")?;
    let block_id=string_at(obs,"block_id")?;
    let treatment_id=string_at(obs,"treatment_id")?;
    let stock=obs.get("stock").ok_or_else(||"stock observation missing".to_string())?;
    let productivity=obs.get("productivity").ok_or_else(||"productivity observation missing".to_string())?;
    if string_at(stock,"plot_id")?!=plot_id || string_at(productivity,"plot_id")?!=plot_id {
        return Ok(None);
    }
    let stock_depth=read_depth(stock,"stock")?;
    let productivity_depth=read_depth(productivity,"productivity")?;
    if stock_depth!=expected_depth || productivity_depth!=expected_depth {
        return Ok(None);
    }
    let (stock_date,prod_start,prod_end)=expected_support(period)?;
    if string_at(stock,"date")?!=stock_date
        || string_at(productivity,"start")?!=prod_start
        || string_at(productivity,"end")?!=prod_end {
        return Ok(None);
    }
    if string_at(stock,"unit")?!="Mg C ha-1"
        || string_at(productivity,"unit")?!="Mg C ha-1 yr-1" {
        return Err("stock/productivity unit mismatch".to_string());
    }
    let stock_value=number_at(stock,"value")?;
    let productivity_value=number_at(productivity,"value")?;
    if stock_value<=0.0 || productivity_value<0.0 {
        return Err("stock must be positive and productivity non-negative".to_string());
    }
    let turnover=productivity_value/stock_value;
    let lifespan=if turnover>0.0 {Some(365.25/turnover)} else {None};
    Ok(Some(PlotPair{plot_id,block_id,treatment_id,stock:stock_value,productivity:productivity_value,turnover,lifespan_days:lifespan}))
}
fn summarize(pairs:&[PlotPair]) -> Result<Summary,String> {
    if pairs.len()<2 { return Err("at least two valid plot pairs are required".to_string()); }
    let stocks:Vec<f64>=pairs.iter().map(|p|p.stock).collect();
    let productivity:Vec<f64>=pairs.iter().map(|p|p.productivity).collect();
    let turnover:Vec<f64>=pairs.iter().map(|p|p.turnover).collect();
    let life:Vec<f64>=pairs.iter().filter_map(|p|p.lifespan_days).collect();
    let n=pairs.len();
    let mean_stock=mean(&stocks)?;
    let mean_productivity=mean(&productivity)?;
    let mean_turnover=mean(&turnover)?;
    let stock_var=sample_variance(&stocks,mean_stock)?;
    let prod_var=sample_variance(&productivity,mean_productivity)?;
    let covariance=sample_covariance(&stocks,&productivity,mean_stock,mean_productivity)?;
    let pooled=productivity.iter().sum::<f64>()/stocks.iter().sum::<f64>();
    let ratio_delta=(
        prod_var/(mean_stock*mean_stock)
        + mean_productivity.powi(2)*stock_var/mean_stock.powi(4)
        - 2.0*mean_productivity*covariance/mean_stock.powi(3)
    )/n as f64;
    if ratio_delta < -1e-9 {
        return Err("delta-method ratio variance is materially negative; inspect pairing and numeric conditioning".to_string());
    }
    let ratio_delta=if ratio_delta.abs()<1e-12 {0.0} else {ratio_delta.max(0.0)};
    let turnover_se=(sample_variance(&turnover,mean_turnover)?/n as f64).sqrt();
    let mean_life=if life.len()==n {Some(mean(&life)?)} else {None};
    let life_se=if life.len()==n {Some((sample_variance(&life,mean_life.unwrap())?/n as f64).sqrt())} else {None};
    let inverse_mean=if mean_turnover>0.0 {Some(365.25/mean_turnover)} else {None};
    Ok(Summary{
        n,mean_stock,mean_productivity,
        stock_sample_variance:stock_var,productivity_sample_variance:prod_var,
        sample_covariance:covariance,mean_plot_turnover:mean_turnover,
        pooled_ratio_of_sums:pooled,mean_plot_lifespan_days:mean_life,
        inverse_mean_turnover_days:inverse_mean,
        mean_plot_turnover_standard_error:turnover_se,
        mean_plot_lifespan_standard_error:life_se,
        ratio_of_means_delta_variance:ratio_delta,
        ratio_of_means_delta_standard_error:ratio_delta.sqrt(),
    })
}
fn summary_json(s:&Summary)->Value {
    json!({
        "plot_count":s.n,
        "mean_stock":s.mean_stock,
        "mean_productivity":s.mean_productivity,
        "stock_sample_variance":s.stock_sample_variance,
        "productivity_sample_variance":s.productivity_sample_variance,
        "stock_productivity_sample_covariance":s.sample_covariance,
        "mean_of_plot_level_turnover":s.mean_plot_turnover,
        "ratio_of_group_means":s.pooled_ratio_of_sums,
        "mean_plot_lifespan_days":s.mean_plot_lifespan_days,
        "inverse_mean_turnover_days":s.inverse_mean_turnover_days,
        "mean_plot_turnover_standard_error":s.mean_plot_turnover_standard_error,
        "mean_plot_lifespan_standard_error":s.mean_plot_lifespan_standard_error,
        "ratio_of_means_delta_method_variance_approx":s.ratio_of_means_delta_variance,
        "ratio_of_means_delta_method_standard_error_approx":s.ratio_of_means_delta_standard_error,
        "delta_method_status":"FIRST_ORDER_APPROXIMATION_NOT_FIELLER_INTERVAL"
    })
}

pub fn validate_corpus(raw:&[u8],manifest:&Value)->Result<Value,String>{
    if string_at(manifest,"schema_version")?!="0.1.0"
        || string_at(manifest,"benchmark_id")?!="LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || string_at(manifest,"corpus_id")?!="AFEX-RATIO-ESTIMAND-001" {
        return Err("AFEX ratio-estimand corpus identity mismatch".to_string());
    }
    let source=manifest.get("source").ok_or_else(||"source metadata required".to_string())?;
    if string_at(source,"paper_doi")?!="10.1007/s11104-026-08303-2"
        || string_at(source,"stock_data_access")?!="AVAILABLE_FROM_CORRESPONDING_AUTHOR_ON_REASONABLE_REQUEST"
        || source.get("raw_plot_data_ingested").and_then(Value::as_bool)!=Some(false) {
        return Err("AFEX root data claim ceiling invalid".to_string());
    }
    let contract=manifest.get("estimand_contract").ok_or_else(||"estimand_contract required".to_string())?;
    let period=string_at(contract,"period")?;
    let expected_depth=read_depth(contract,"estimand_contract")?;
    let (stock_date,prod_start,prod_end)=expected_support(&period)?;
    if string_at(contract,"stock_date")?!=stock_date
        || string_at(contract,"productivity_start")?!=prod_start
        || string_at(contract,"productivity_end")?!=prod_end
        || string_at(contract,"stock_unit")?!="Mg C ha-1"
        || string_at(contract,"productivity_unit")?!="Mg C ha-1 yr-1"
        || string_at(contract,"treatment_assignment_unit")?!="plot"
        || string_at(contract,"independent_sample_unit")?!="plot" {
        return Err("AFEX estimand contract support or unit mismatch".to_string());
    }
    let cases=manifest.get("cases").and_then(Value::as_array).ok_or_else(||"cases must be array".to_string())?;
    let expected_count=manifest.get("expected_case_count").and_then(Value::as_u64).ok_or_else(||"expected_case_count required".to_string())? as usize;
    if cases.len()!=expected_count { return Err("ratio case count mismatch".to_string()); }
    let mut case_ids=BTreeSet::new();
    let mut reports:BTreeMap<String,Summary>=BTreeMap::new();
    let mut rows=Vec::new();
    for case in cases {
        let case_id=string_at(case,"case_id")?;
        if !case_ids.insert(case_id.clone()) { return Err("duplicate ratio case ID".to_string()); }
        if string_at(case,"period")?!=period || read_depth(case,"case")?!=expected_depth {
            return Err(format!("case {case_id} period/depth does not match estimand contract"));
        }
        let observations=case.get("observations").and_then(Value::as_array).ok_or_else(||"observations must be an array".to_string())?;
        let mut pairs=Vec::new();
        let mut unmatched=false;
        let mut plot_ids=BTreeSet::new();
        let mut blocks=BTreeSet::new();
        let mut treatment_ids=BTreeSet::new();
        for obs in observations {
            let pair=validate_pair(obs,&period,expected_depth)?;
            match pair {
                Some(p)=>{
                    if !plot_ids.insert(p.plot_id.clone()) { return Err(format!("case {case_id} contains duplicate plot ID")); }
                    blocks.insert(p.block_id.clone());
                    treatment_ids.insert(p.treatment_id.clone());
                    pairs.push(p);
                },
                None=>unmatched=true,
            }
        }
        if unmatched {
            if string_at(case,"expected_disposition")?!="SUPPORT_MISMATCH_REFUSED" {
                return Err(format!("case {case_id} has unmatched support but does not expect refusal"));
            }
            rows.push(json!({"case_id":case_id,"disposition":"SUPPORT_MISMATCH_REFUSED","summary":null}));
            continue;
        }
        let declared_treatment=string_at(case,"treatment_id")?;
        if pairs.len()!=4 || blocks.len()!=4 || treatment_ids.len()!=1
            || treatment_ids.iter().next()!=Some(&declared_treatment) {
            return Err(format!("case {case_id} must contain four plot units across four blocks in the declared treatment cell"));
        }
        if string_at(case,"expected_disposition")?!="ESTIMAND_SUMMARY_VALID" {
            return Err(format!("case {case_id} valid support has an incompatible expected disposition"));
        }
        let summary=summarize(&pairs)?;
        let expected=case.get("expected_summary").ok_or_else(||format!("case {case_id} expected_summary required"))?;
        let tol=number_at(manifest.get("tolerances").ok_or_else(||"tolerances required".to_string())?,"absolute")?;
        let var_tol=number_at(manifest.get("tolerances").unwrap(),"variance_absolute")?;
        let checks=[
            ("mean_stock",summary.mean_stock),
            ("mean_productivity",summary.mean_productivity),
            ("stock_sample_variance",summary.stock_sample_variance),
            ("productivity_sample_variance",summary.productivity_sample_variance),
            ("sample_covariance",summary.sample_covariance),
            ("mean_plot_turnover",summary.mean_plot_turnover),
            ("pooled_ratio_of_sums",summary.pooled_ratio_of_sums),
            ("mean_plot_turnover_standard_error",summary.mean_plot_turnover_standard_error),
            ("ratio_of_means_delta_variance",summary.ratio_of_means_delta_variance),
            ("ratio_of_means_delta_standard_error",summary.ratio_of_means_delta_standard_error)
        ];
        for (key,actual) in checks {
            let expected_value=number_at(expected,key)?;
            let tolerance=if key.contains("variance") {var_tol} else {tol};
            if (actual-expected_value).abs()>tolerance {
                return Err(format!("case {case_id} oracle mismatch at {key}: actual={actual}, expected={expected_value}"));
            }
        }
        compare_optional_number(expected,"mean_plot_lifespan_days",summary.mean_plot_lifespan_days,tol)?;
        compare_optional_number(expected,"inverse_mean_turnover_days",summary.inverse_mean_turnover_days,tol)?;
        compare_optional_number(expected,"mean_plot_lifespan_standard_error",summary.mean_plot_lifespan_standard_error,tol)?;
        reports.insert(case_id.clone(),summary.clone());
        rows.push(json!({"case_id":case_id,"disposition":"ESTIMAND_SUMMARY_VALID","summary":summary_json(&summary)}));
    }
    let comparison=manifest.get("comparison_oracle").ok_or_else(||"comparison_oracle required".to_string())?;
    let case_a=string_at(comparison,"case_a")?;
    let case_b=string_at(comparison,"case_b")?;
    let a=reports.get(&case_a).ok_or_else(||"comparison case A is missing/invalid".to_string())?;
    let b=reports.get(&case_b).ok_or_else(||"comparison case B is missing/invalid".to_string())?;
    let same_marginals=(a.mean_stock-b.mean_stock).abs()<=1e-9
        && (a.mean_productivity-b.mean_productivity).abs()<=1e-9
        && (a.stock_sample_variance-b.stock_sample_variance).abs()<=1e-9
        && (a.productivity_sample_variance-b.productivity_sample_variance).abs()<=1e-9;
    let opposite_covariance=a.sample_covariance*b.sample_covariance<0.0;
    let different_plot_ratio=(a.mean_plot_turnover-b.mean_plot_turnover).abs()>1e-9;
    let same_pooled_ratio=(a.pooled_ratio_of_sums-b.pooled_ratio_of_sums).abs()<=1e-9;
    let different_delta=(a.ratio_of_means_delta_variance-b.ratio_of_means_delta_variance).abs()>1e-9;
    let different_lifespan_estimands=b.mean_plot_lifespan_days.zip(b.inverse_mean_turnover_days)
        .map(|(mean_life,inverse_mean)| (mean_life-inverse_mean).abs()>1e-9).unwrap_or(false);
    if !same_marginals||!opposite_covariance||!different_plot_ratio||!same_pooled_ratio
        ||!different_delta||!different_lifespan_estimands {
        return Err("paired-marginal/covariance/ratio-inversion comparison oracle failed".to_string());
    }
    Ok(json!({
        "schema_version":"0.1.0",
        "benchmark_id":"LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "corpus_id":"AFEX-RATIO-ESTIMAND-001",
        "result":"PASS_SYNTHETIC_RATIO_ESTIMANDS",
        "source_manifest_sha256":sha256_hex(raw),
        "case_count":rows.len(),
        "valid_summary_count":reports.len(),
        "support_refusal_count":rows.iter().filter(|r|r["disposition"]=="SUPPORT_MISMATCH_REFUSED").count(),
        "comparison_oracle":{
            "same_stock_and_productivity_marginals":same_marginals,
            "opposite_stock_productivity_covariance":opposite_covariance,
            "mean_plot_turnover_changes_with_pairing":different_plot_ratio,
            "pooled_ratio_stays_equal":same_pooled_ratio,
            "delta_method_uncertainty_changes_with_covariance":different_delta,
            "mean_plot_lifespan_differs_from_inverse_mean_turnover":different_lifespan_estimands
        },
        "case_results":rows,
        "delta_method_claim_ceiling":"FIRST_ORDER_APPROXIMATION_ONLY; NOT FIELLER; NOT A COVERAGE GUARANTEE",
        "raw_plot_data_ingested":false,
        "empirical_ratio_reanalysis":"NOT_ESTABLISHED"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE:&str=include_str!("../../cases/afex-ratio-estimand-synthetic-v0.1.json");

    #[test]
    fn ratio_estimands_distinguish_pairing_and_covariance(){
        let m:Value=serde_json::from_str(FIXTURE).expect("fixture parses");
        let r=validate_corpus(FIXTURE.as_bytes(),&m).expect("receipt");
        assert_eq!(r["result"],"PASS_SYNTHETIC_RATIO_ESTIMANDS");
        assert_eq!(r["valid_summary_count"],2);
        assert_eq!(r["support_refusal_count"],1);
        assert_eq!(r["comparison_oracle"]["same_stock_and_productivity_marginals"],true);
        assert_eq!(r["comparison_oracle"]["opposite_stock_productivity_covariance"],true);
        assert_eq!(r["comparison_oracle"]["mean_plot_turnover_changes_with_pairing"],true);
        assert_eq!(r["comparison_oracle"]["mean_plot_lifespan_differs_from_inverse_mean_turnover"],true);
        let reverse=&r["case_results"][1]["summary"];
        assert!((reverse["mean_of_plot_level_turnover"].as_f64().unwrap()-3.2083333333333335).abs()<1e-9);
        assert!((reverse["mean_plot_lifespan_days"].as_f64().unwrap()-292.9609375).abs()<1e-9);
        assert!((reverse["inverse_mean_turnover_days"].as_f64().unwrap()-113.84415584415584).abs()<1e-9);
    }
    #[test]
    fn unmatched_plot_pair_is_refused(){
        let m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let r=validate_corpus(FIXTURE.as_bytes(),&m).expect("refusal receipt");
        assert_eq!(r["case_results"][2]["disposition"],"SUPPORT_MISMATCH_REFUSED");
    }
    #[test]
    fn zero_stock_is_rejected(){
        let mut m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        m["cases"][0]["observations"][0]["stock"]["value"]=json!(0.0);
        let raw=serde_json::to_vec(&m).expect("serialize");
        assert!(validate_corpus(&raw,&m).is_err());
    }
    #[test]
    fn mismatched_productivity_interval_is_refused(){
        let mut m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        m["cases"][0]["observations"][0]["productivity"]["start"]=json!("2018-08");
        let raw=serde_json::to_vec(&m).expect("serialize");
        assert!(validate_corpus(&raw,&m).is_err());
    }
    #[test]
    fn unknown_units_are_rejected(){
        let mut m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        m["cases"][0]["observations"][0]["stock"]["unit"]=json!("kg m-2");
        let raw=serde_json::to_vec(&m).expect("serialize");
        assert!(validate_corpus(&raw,&m).is_err());
    }
    #[test]
    fn undeclared_treatment_cell_is_rejected(){
        let mut m:Value=serde_json::from_str(FIXTURE).expect("fixture");
        m["cases"][0]["treatment_id"]=json!("N1-P1-C1");
        let raw=serde_json::to_vec(&m).expect("serialize");
        assert!(validate_corpus(&raw,&m).is_err());
    }
    #[test]
    fn zero_productivity_keeps_lifespan_undefined_instead_of_infinite(){
        let pairs=vec![
            PlotPair{plot_id:"p1".to_string(),block_id:"b1".to_string(),treatment_id:"t".to_string(),stock:1.0,productivity:0.0,turnover:0.0,lifespan_days:None},
            PlotPair{plot_id:"p2".to_string(),block_id:"b2".to_string(),treatment_id:"t".to_string(),stock:1.0,productivity:1.0,turnover:1.0,lifespan_days:Some(365.25)},
        ];
        let summary=summarize(&pairs).expect("zero productivity is a valid zero-turnover observation");
        assert_eq!(summary.mean_plot_lifespan_days,None);
        assert_eq!(summary.mean_plot_lifespan_standard_error,None);
        assert_eq!(summary.inverse_mean_turnover_days,Some(730.5));
    }
}
