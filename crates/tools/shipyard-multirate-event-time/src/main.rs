use serde_json::Value;
use std::{env, fs, process::ExitCode};
const DEFAULT_CORPUS: &str = "benchmarks/shipyard/multirate-event-time/corpus-v0.1.json";
const BENCHMARK_ID: &str = "SHIPYARD-MULTIRATE-EVENT-TIME-QUAL-001";
fn validate_corpus(text: &str) -> Result<(usize, usize, usize), Vec<String>> {
    let v: Value =
        serde_json::from_str(text).map_err(|e| vec![format!("corpus JSON parse failed: {e}")])?;
    let mut e = Vec::new();
    if v["schema_version"].as_str() != Some("0.1.0") {
        e.push("schema version mismatch".into());
    }
    if v["benchmark_id"].as_str() != Some(BENCHMARK_ID) {
        e.push("benchmark ID mismatch".into());
    }
    if v["source_scope"].as_str() != Some("synthetic_only") {
        e.push("scope must be synthetic only".into());
    }
    if v["raw_bam_files_downloaded"].as_bool() != Some(false) {
        e.push("raw BAM files are forbidden in V0".into());
    }
    let expected = [
        ("process_current_voltage", 100_000u64),
        ("robot_tcp_coordinates", 8_000_000),
        ("power_source_opcua", 100_000_000),
        ("control_power_gas_thermocouple", 1_000_000),
        ("laser_scan_profile_trigger", 20_000_000),
        ("video_frame_trigger", 25_000_000),
    ];
    let Some(p) = v["channel_profiles"].as_array() else {
        return Err(vec!["channel_profiles must be an array".into()]);
    };
    if p.len() != 6 {
        e.push(format!("expected six channel profiles, found {}", p.len()));
    }
    for (id, period) in expected {
        if !p.iter().any(|x| {
            x["channel_id"].as_str() == Some(id) && x["nominal_period_ns"].as_u64() == Some(period)
        }) {
            e.push(format!("{id} cadence must remain {period} ns"));
        }
    }
    let Some(c) = v["controls"].as_array() else {
        return Err(vec!["controls must be an array".into()]);
    };
    if c.len() != 31 {
        e.push(format!("expected 31 controls, found {}", c.len()));
    }
    let (mut pos, mut neg) = (0usize, 0usize);
    for (i, x) in c.iter().enumerate() {
        let id = format!("MTIME-{:03}", i + 1);
        if x["control_id"].as_str() != Some(id.as_str()) {
            e.push(format!("control {i} expected {id}"));
        }
        match x["class"].as_str() {
            Some("positive_contract_control") => pos += 1,
            Some("negative_control") => neg += 1,
            _ => e.push(format!("{id} invalid class")),
        }
        if x["expected_disposition"].as_str().is_none_or(str::is_empty) {
            e.push(format!("{id} missing disposition"));
        }
    }
    if pos != 1 || neg != 30 {
        e.push(format!(
            "expected 1 positive / 30 negative; got {pos}/{neg}"
        ));
    }
    if e.is_empty() {
        Ok((c.len(), pos, neg))
    } else {
        Err(e)
    }
}
fn main() -> ExitCode {
    let mut a = env::args().skip(1);
    let path = a.next().unwrap_or_else(|| DEFAULT_CORPUS.into());
    if a.next().is_some() {
        eprintln!("usage: shipyard-multirate-event-time [CORPUS]");
        return ExitCode::from(2);
    }
    let text = match fs::read_to_string(&path) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    match validate_corpus(&text) {
        Ok((n, p, q)) => {
            println!(
                "PASS: {n} synthetic controls ({p} positive / {q} negative); six modality cadences preserved; no raw BAM files"
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("FAIL: {} corpus contract error(s)", e.len());
            for x in e {
                eprintln!("- {x}");
            }
            ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const C: &str =
        include_str!("../../../../benchmarks/shipyard/multirate-event-time/corpus-v0.1.json");
    #[test]
    fn frozen_corpus_has_31_controls_and_six_rates() {
        assert_eq!(validate_corpus(C), Ok((31, 1, 30)));
    }
    #[test]
    fn raw_bam_promotion_is_rejected() {
        let mut v: Value = serde_json::from_str(C).unwrap();
        v["raw_bam_files_downloaded"] = Value::Bool(true);
        assert!(validate_corpus(&serde_json::to_string(&v).unwrap()).is_err());
    }
    #[test]
    fn common_cadence_fabrication_is_rejected() {
        let mut v: Value = serde_json::from_str(C).unwrap();
        v["channel_profiles"][2]["nominal_period_ns"] = Value::from(100_000);
        assert!(validate_corpus(&serde_json::to_string(&v).unwrap()).is_err());
    }
    #[test]
    fn control_count_drift_is_rejected() {
        let mut v: Value = serde_json::from_str(C).unwrap();
        v["controls"].as_array_mut().unwrap().pop();
        assert!(validate_corpus(&serde_json::to_string(&v).unwrap()).is_err());
    }
}
