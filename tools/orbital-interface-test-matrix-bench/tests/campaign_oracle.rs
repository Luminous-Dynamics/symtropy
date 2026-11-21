use orbital_interface_test_matrix_bench::{Corpus, Disposition, evaluate};
use serde_json::Value;
use std::collections::BTreeMap;

fn fixture() -> &'static str {
    include_str!("../fixtures/v0.1/campaigns.json")
}

#[test]
fn production_evaluator_matches_all_frozen_campaign_expectations() {
    let corpus: Corpus = serde_json::from_str(fixture()).expect("campaign corpus parses");
    let report = evaluate(&corpus).expect("campaign corpus evaluates");
    assert_eq!(report.result, "pass");
    assert_eq!(report.summary.case_count, 35);
    assert_eq!(report.summary.expected_result_mismatches, 0);
    assert!(!report.authority_granted);
}

#[test]
fn naive_baselines_expose_nominal_cycle_and_interaction_failures() {
    let corpus: Corpus = serde_json::from_str(fixture()).expect("campaign corpus parses");
    let report = evaluate(&corpus).expect("campaign corpus evaluates");

    let cycle = report
        .cases
        .iter()
        .find(|item| item.case_id == "case-02-many-nominal-cycles-no-low-energy")
        .expect("cycle-count counterexample exists");
    assert!(cycle.cycle_count_baseline_pass);
    assert_eq!(cycle.disposition, Disposition::CoverageGap);

    let ofat = report
        .cases
        .iter()
        .find(|item| item.case_id == "case-04-ofat-does-not-cover-required-pair")
        .expect("OFAT counterexample exists");
    assert_eq!(ofat.ofat_false_coverage_count, 1);
    assert_eq!(ofat.pairwise_false_coverage_count, 0);
    assert_eq!(ofat.disposition, Disposition::CoverageGap);

    let pairwise = report
        .cases
        .iter()
        .find(|item| item.case_id == "case-05-pairwise-does-not-cover-critical-triple")
        .expect("pairwise counterexample exists");
    assert_eq!(pairwise.pairwise_false_coverage_count, 1);
    assert_eq!(pairwise.disposition, Disposition::CoverageGap);
}

#[test]
fn completed_failure_is_coverage_but_never_a_pass() {
    let corpus: Corpus = serde_json::from_str(fixture()).expect("campaign corpus parses");
    let report = evaluate(&corpus).expect("campaign corpus evaluates");
    let failed = report
        .cases
        .iter()
        .find(|item| item.case_id == "case-11-completed-failure-still-covers-test-condition")
        .expect("completed failure case exists");
    assert_eq!(
        failed.disposition,
        Disposition::CoverageCompleteWithFailures
    );
    assert_eq!(failed.failed_runs, 1);
    assert_eq!(failed.covered_obligations, 1);
}

#[test]
fn duplicates_and_run_group_ids_are_not_double_counted() {
    let corpus: Corpus = serde_json::from_str(fixture()).expect("campaign corpus parses");
    let report = evaluate(&corpus).expect("campaign corpus evaluates");

    let duplicate = report
        .cases
        .iter()
        .find(|item| {
            item.case_id == "case-13-identical-duplicate-run-is-idempotent-not-independent"
        })
        .expect("duplicate case exists");
    assert_eq!(duplicate.duplicate_runs, 1);
    assert_eq!(duplicate.eligible_runs, 1);
    assert_eq!(duplicate.disposition, Disposition::CoverageGap);

    let common_cause = report
        .cases
        .iter()
        .find(|item| item.case_id == "case-29-common-fixture-is-not-independent-replication")
        .expect("common-cause case exists");
    assert_eq!(common_cause.disposition, Disposition::CoverageGap);
    assert_eq!(common_cause.pairwise_false_coverage_count, 1);
}

#[test]
fn input_order_permutation_does_not_change_campaign_dispositions() {
    let mut corpus: Corpus = serde_json::from_str(fixture()).expect("campaign corpus parses");
    let original = evaluate(&corpus).expect("original corpus evaluates");
    for case in &mut corpus.cases {
        case.obligations.reverse();
        case.runs.reverse();
    }
    corpus.cases.reverse();
    let permuted = evaluate(&corpus).expect("permuted corpus evaluates");

    let index = |report: &orbital_interface_test_matrix_bench::Report| {
        report
            .cases
            .iter()
            .map(|item| {
                (
                    item.case_id.clone(),
                    serde_json::to_value(item).expect("case result serializes"),
                )
            })
            .collect::<BTreeMap<String, Value>>()
    };
    assert_eq!(index(&original), index(&permuted));
}
