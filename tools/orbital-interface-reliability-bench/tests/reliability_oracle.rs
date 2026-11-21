use orbital_interface_reliability_bench::{
    Corpus, Disposition, evaluate, minimum_zero_failure_trials,
    mission_reliability_lower_bound,
};
use serde_json::Value;
use std::collections::BTreeMap;

fn fixture() -> &'static str {
    include_str!("../fixtures/v0.1/reliability-cases.json")
}

#[test]
fn exact_zero_failure_sample_sizes_match_known_answers() {
    assert_eq!(minimum_zero_failure_trials(0.90, 0.90).unwrap(), 22);
    assert_eq!(minimum_zero_failure_trials(0.95, 0.90).unwrap(), 45);
    assert_eq!(minimum_zero_failure_trials(0.99, 0.90).unwrap(), 230);
    assert!(mission_reliability_lower_bound(22, 0, 0.90).unwrap() >= 0.90);
    assert!(mission_reliability_lower_bound(21, 0, 0.90).unwrap() < 0.90);
}

#[test]
fn failures_reduce_the_exact_one_sided_lower_bound() {
    let no_failure = mission_reliability_lower_bound(22, 0, 0.90).unwrap();
    let one_failure = mission_reliability_lower_bound(22, 1, 0.90).unwrap();
    assert!(one_failure < no_failure);
    assert_eq!(mission_reliability_lower_bound(4, 4, 0.90).unwrap(), 0.0);
}

#[test]
fn production_evaluator_matches_all_frozen_reliability_cases() {
    let corpus: Corpus = serde_json::from_str(fixture()).expect("reliability corpus parses");
    let report = evaluate(&corpus).expect("reliability corpus evaluates");
    assert_eq!(report.result, "pass");
    assert_eq!(report.summary.case_count, 36);
    assert_eq!(report.summary.expected_result_mismatches, 0);
    assert!(!report.authority_granted);
}

#[test]
fn repeated_cycles_on_one_article_do_not_establish_mission_reliability() {
    let corpus: Corpus = serde_json::from_str(fixture()).expect("reliability corpus parses");
    let report = evaluate(&corpus).expect("reliability corpus evaluates");
    let case = report
        .cases
        .iter()
        .find(|item| item.case_id == "case-34-cycle-baseline-false-assurance-on-one-article")
        .expect("cycle pseudoreplication case exists");
    assert_eq!(case.unique_trials, 1);
    assert_eq!(case.disposition, Disposition::ReliabilityNotDemonstrated);
    assert!(case.naive_cycle_false_assurance);
    assert!(case.naive_cycle_lower_bound.unwrap() >= 0.90);
    assert!(case.lower_confidence_bound.unwrap() < 0.90);
}

#[test]
fn pseudoreplicated_article_rows_cannot_pass_the_cycle_baseline() {
    let corpus: Corpus = serde_json::from_str(fixture()).expect("reliability corpus parses");
    let report = evaluate(&corpus).expect("reliability corpus evaluates");
    let case = report
        .cases
        .iter()
        .find(|item| item.case_id == "case-11-repeated-cycles-on-one-article-are-pseudoreplication")
        .expect("repeated article case exists");
    assert_eq!(case.disposition, Disposition::Indeterminate);
    assert_eq!(case.unique_trials, 0);
    assert!(case.naive_cycle_false_assurance);
    assert!(case.naive_cycle_lower_bound.unwrap() >= 0.90);
}

#[test]
fn repeated_daq_samples_do_not_create_additional_trials() {
    let corpus: Corpus = serde_json::from_str(fixture()).expect("reliability corpus parses");
    let report = evaluate(&corpus).expect("reliability corpus evaluates");
    let case = report
        .cases
        .iter()
        .find(|item| item.case_id == "case-12-daq-samples-within-one-mission-are-one-trial")
        .expect("DAQ sample case exists");
    assert_eq!(case.unique_trials, 1);
}

#[test]
fn article_selection_and_lifecycle_scope_are_not_silently_pooled() {
    let corpus: Corpus = serde_json::from_str(fixture()).expect("reliability corpus parses");
    let report = evaluate(&corpus).expect("reliability corpus evaluates");
    for id in [
        "case-18-configuration-mismatch-rejected",
        "case-19-mission-profile-mismatch-rejected",
        "case-20-failure-definition-mismatch-rejected",
        "case-31-multi-lot-pooling-not-supported",
        "case-32-population-mismatch-rejected",
    ] {
        let case = report.cases.iter().find(|item| item.case_id == id).unwrap();
        assert_eq!(case.disposition, Disposition::Rejected, "{id}");
    }
}

#[test]
fn result_receipt_is_invariant_to_case_order() {
    let mut corpus: Corpus = serde_json::from_str(fixture()).expect("reliability corpus parses");
    let first = evaluate(&corpus).expect("original corpus evaluates");
    corpus.cases.reverse();
    let second = evaluate(&corpus).expect("permuted corpus evaluates");
    let index = |report: &orbital_interface_reliability_bench::Report| {
        report.cases.iter().map(|item| {
            (
                item.case_id.clone(),
                serde_json::to_value(item).expect("case serializes"),
            )
        }).collect::<BTreeMap<String, Value>>()
    };
    assert_eq!(index(&first), index(&second));
}
