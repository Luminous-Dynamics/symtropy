use serde::Deserialize;
use shipyard_scan_metrology::{
    evaluate_profile_pair, MeasurementReport, ProfilePairInput, BENCHMARK_ID,
};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const DEFAULT_CORPUS: &str =
    "benchmarks/shipyard/scan-metrology/paired-profile-v0.1.json";

#[derive(Debug, Deserialize)]
struct NegativeControl {
    control_id: String,
    mutation: String,
    expected_disposition: String,
}

#[derive(Debug, Deserialize)]
struct BenchmarkCorpus {
    schema_version: String,
    benchmark_id: String,
    profile_pair: ProfilePairInput,
    expected_report: MeasurementReport,
    negative_controls: Vec<NegativeControl>,
}

fn verify_corpus(corpus: &BenchmarkCorpus) -> Result<MeasurementReport, String> {
    if corpus.schema_version != "0.1.0" {
        return Err("corpus schema_version must be 0.1.0".to_owned());
    }
    if corpus.benchmark_id != BENCHMARK_ID {
        return Err("corpus benchmark_id mismatch".to_owned());
    }
    if corpus.negative_controls.len() != 32 {
        return Err(format!(
            "expected 32 negative controls, found {}",
            corpus.negative_controls.len()
        ));
    }

    let mut ids = HashSet::new();
    for (index, control) in corpus.negative_controls.iter().enumerate() {
        let expected_id = format!("SMET-{:03}", index + 1);
        if control.control_id != expected_id || !ids.insert(control.control_id.as_str()) {
            return Err(format!(
                "negative control {} has duplicate or non-sequential identity",
                index + 1
            ));
        }
        if control.mutation.trim().is_empty() || control.expected_disposition.trim().is_empty() {
            return Err(format!("{} has an empty mutation contract", control.control_id));
        }
    }

    let report = evaluate_profile_pair(&corpus.profile_pair)
        .map_err(|reason| format!("profile-pair oracle rejected positive fixture: {reason:?}"))?;
    if report != corpus.expected_report {
        return Err("computed profile metrics disagree with frozen known-answer report".to_owned());
    }
    Ok(report)
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let corpus_path = args.next().unwrap_or_else(|| DEFAULT_CORPUS.to_owned());
    if args.next().is_some() {
        return Err("usage: shipyard-scan-metrology [CORPUS_PATH]".to_owned());
    }
    let text = fs::read_to_string(&corpus_path)
        .map_err(|error| format!("cannot read corpus {corpus_path}: {error}"))?;
    let corpus: BenchmarkCorpus = serde_json::from_str(&text)
        .map_err(|error| format!("cannot parse corpus {corpus_path}: {error}"))?;
    let report = verify_corpus(&corpus)?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cannot serialize measurement report: {error}"))?;
    println!("{rendered}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => {
            println!("PASS: 1 synthetic scan-pair known answer and 32 negative controls");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_corpus_known_answer_is_reproduced() {
        let corpus: BenchmarkCorpus = serde_json::from_str(include_str!(
            "../../../../benchmarks/shipyard/scan-metrology/paired-profile-v0.1.json"
        ))
        .expect("frozen corpus must parse");
        let report = verify_corpus(&corpus).expect("known-answer corpus must verify");
        assert_eq!(report.paired_sample_count, 4);
        assert_eq!(report.coverage_permille, 1_000);
        assert_eq!(report.signed_area_change_um2, 300_000);
    }

    #[test]
    fn duplicate_control_id_is_rejected() {
        let mut corpus: BenchmarkCorpus = serde_json::from_str(include_str!(
            "../../../../benchmarks/shipyard/scan-metrology/paired-profile-v0.1.json"
        ))
        .expect("frozen corpus must parse");
        corpus.negative_controls[1].control_id = corpus.negative_controls[0].control_id.clone();
        assert!(verify_corpus(&corpus).is_err());
    }

    #[test]
    fn unknown_extra_control_does_not_expand_frozen_corpus() {
        let mut corpus: BenchmarkCorpus = serde_json::from_str(include_str!(
            "../../../../benchmarks/shipyard/scan-metrology/paired-profile-v0.1.json"
        ))
        .expect("frozen corpus must parse");
        corpus.negative_controls.push(NegativeControl {
            control_id: "SMET-033".to_owned(),
            mutation: "extra case".to_owned(),
            expected_disposition: "BLOCK".to_owned(),
        });
        assert!(verify_corpus(&corpus).is_err());
    }
}
