use serde::Deserialize;
use shipyard_reference_drift::{
    BENCHMARK_ID, Decision, DriftReport, DriftSequenceInput, evaluate_drift_sequence,
};
use std::env;
use std::fs;
use std::process::ExitCode;

const DEFAULT_CORPUS: &str = "benchmarks/shipyard/reference-drift/corpus-v0.1.json";

#[derive(Debug, Deserialize)]
struct Scenario {
    scenario_id: String,
    input: DriftSequenceInput,
    expected_decision: Decision,
    expected_hold_reason: Option<shipyard_reference_drift::HoldReason>,
}

#[derive(Debug, Deserialize)]
struct NegativeControl {
    control_id: String,
    mutation: String,
    expected_disposition: String,
}

#[derive(Debug, Deserialize)]
struct Corpus {
    schema_version: String,
    benchmark_id: String,
    scenarios: Vec<Scenario>,
    negative_controls: Vec<NegativeControl>,
}

fn verify_corpus(corpus: &Corpus) -> Result<Vec<DriftReport>, String> {
    if corpus.schema_version != "0.1.0" || corpus.benchmark_id != BENCHMARK_ID {
        return Err("corpus schema version or benchmark ID mismatch".to_owned());
    }
    if corpus.scenarios.len() != 2 || corpus.negative_controls.len() != 18 {
        return Err("frozen corpus must contain two scenarios and 18 negative controls".to_owned());
    }
    let mut reports = Vec::new();
    for scenario in &corpus.scenarios {
        let report = evaluate_drift_sequence(&scenario.input)
            .map_err(|reason| format!("{} rejected: {reason:?}", scenario.scenario_id))?;
        if report.decision != scenario.expected_decision
            || report.hold_reason != scenario.expected_hold_reason
        {
            return Err(format!("{} decision differs from known answer", scenario.scenario_id));
        }
        reports.push(report);
    }
    for (index, control) in corpus.negative_controls.iter().enumerate() {
        let expected_id = format!("SRD-{:03}", index + 1);
        if control.control_id != expected_id
            || control.mutation.trim().is_empty()
            || control.expected_disposition.trim().is_empty()
        {
            return Err(format!("negative control {} is invalid", index + 1));
        }
    }
    Ok(reports)
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap_or_else(|| DEFAULT_CORPUS.to_owned());
    if args.next().is_some() {
        return Err("usage: shipyard-reference-drift [CORPUS_PATH]".to_owned());
    }
    let text = fs::read_to_string(&path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let corpus: Corpus =
        serde_json::from_str(&text).map_err(|error| format!("cannot parse {path}: {error}"))?;
    let reports = verify_corpus(&corpus)?;
    let rendered = serde_json::to_string_pretty(&reports)
        .map_err(|error| format!("cannot serialize reports: {error}"))?;
    println!("{rendered}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => {
            println!("PASS: 2 reference-drift known answers and 18 negative controls");
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
    fn frozen_corpus_reproduces_stable_and_drift_hold_answers() {
        let corpus: Corpus = serde_json::from_str(include_str!(
            "../../../../benchmarks/shipyard/reference-drift/corpus-v0.1.json"
        ))
        .expect("corpus must parse");
        let reports = verify_corpus(&corpus).expect("corpus must replay");
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].decision, Decision::ContinueWithinDeclaredCheckLimits);
        assert_eq!(reports[1].decision, Decision::HoldForMetrologyReview);
    }

    #[test]
    fn extra_negative_control_does_not_expand_frozen_corpus() {
        let mut corpus: Corpus = serde_json::from_str(include_str!(
            "../../../../benchmarks/shipyard/reference-drift/corpus-v0.1.json"
        ))
        .expect("corpus must parse");
        corpus.negative_controls.push(NegativeControl {
            control_id: "SRD-019".to_owned(),
            mutation: "extra".to_owned(),
            expected_disposition: "HOLD".to_owned(),
        });
        assert!(verify_corpus(&corpus).is_err());
    }
}
