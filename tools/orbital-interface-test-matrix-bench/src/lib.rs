use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-interface-test-matrix.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-test-matrix-coverage-semantics-only";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Criticality {
    Mandatory,
    Critical,
    Informational,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestOutcome {
    Passed,
    Failed,
    Aborted,
    Invalid,
    Planned,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    CoverageComplete,
    CoverageCompleteWithFailures,
    CoverageGap,
    Indeterminate,
    Rejected,
    Disputed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Obligation {
    pub id: String,
    pub criticality: Criticality,
    pub configuration_id: String,
    pub procedure_revision: String,
    pub service_kind: String,
    pub lifecycle_phase: String,
    pub conditions: BTreeMap<String, String>,
    pub evidence_key: String,
    pub minimum_run_groups: usize,
    pub not_applicable_reason: Option<String>,
    pub not_applicable_review_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TestRun {
    pub run_id: String,
    pub configuration_id: String,
    pub procedure_revision: String,
    pub service_kind: String,
    pub lifecycle_phase: String,
    pub conditions: BTreeMap<String, String>,
    pub outcome: TestOutcome,
    pub raw_artifact_sha256: String,
    pub calibration_valid: bool,
    pub instrument_in_range: bool,
    pub uncertainty_complete: bool,
    pub clock_aligned: bool,
    pub evidence_keys: Vec<String>,
    pub run_group_id: String,
    pub cost_units: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition,
    pub covered_obligations: usize,
    pub missing_obligation_ids: Vec<String>,
    pub eligible_runs: usize,
    pub failed_runs: usize,
    pub aborted_runs: usize,
    pub invalid_runs: usize,
    pub planned_runs: usize,
    pub duplicate_runs: usize,
    pub budget_exceeded: bool,
    pub cycle_count_baseline_pass: bool,
    pub ofat_false_coverage_count: usize,
    pub pairwise_false_coverage_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignCase {
    pub case_id: String,
    pub preregistration_ref: Option<String>,
    pub cost_budget: u64,
    pub cycle_count_threshold: usize,
    pub obligations: Vec<Obligation>,
    pub runs: Vec<TestRun>,
    pub expected: Expected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<CampaignCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub covered_obligations: usize,
    pub missing_obligation_ids: Vec<String>,
    pub eligible_runs: usize,
    pub failed_runs: usize,
    pub aborted_runs: usize,
    pub invalid_runs: usize,
    pub planned_runs: usize,
    pub duplicate_runs: usize,
    pub budget_exceeded: bool,
    pub cycle_count_baseline_pass: bool,
    pub ofat_false_coverage_count: usize,
    pub pairwise_false_coverage_count: usize,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub coverage_complete: usize,
    pub coverage_complete_with_failures: usize,
    pub coverage_gaps: usize,
    pub indeterminate: usize,
    pub rejected: usize,
    pub disputed: usize,
    pub eligible_runs: usize,
    pub failed_runs: usize,
    pub aborted_runs: usize,
    pub invalid_runs: usize,
    pub planned_runs: usize,
    pub duplicate_runs: usize,
    pub budget_exceeded_campaigns: usize,
    pub cycle_count_false_pass_campaigns: usize,
    pub ofat_false_coverage_obligations: usize,
    pub pairwise_false_coverage_obligations: usize,
    pub expected_result_mismatches: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, Clone, Copy)]
struct Decision {
    disposition: Disposition,
    covered: usize,
    missing: usize,
    eligible_runs: usize,
    failed_runs: usize,
    aborted_runs: usize,
    invalid_runs: usize,
    planned_runs: usize,
    duplicate_runs: usize,
    budget_exceeded: bool,
    cycle_count_baseline_pass: bool,
    ofat_false_coverage_count: usize,
    pairwise_false_coverage_count: usize,
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn has_not_applicable_review_reference(obligation: &Obligation) -> bool {
    obligation
        .not_applicable_reason
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && obligation
            .not_applicable_review_ref
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn is_completed(outcome: TestOutcome) -> bool {
    matches!(outcome, TestOutcome::Passed | TestOutcome::Failed)
}

fn run_has_valid_evidence(run: &TestRun) -> bool {
    is_completed(run.outcome)
        && valid_digest(&run.raw_artifact_sha256)
        && run.calibration_valid
        && run.instrument_in_range
        && run.uncertainty_complete
        && run.clock_aligned
        && !run.run_group_id.is_empty()
        && !run.evidence_keys.is_empty()
}

fn run_matches_scope(run: &TestRun, obligation: &Obligation) -> bool {
    run.configuration_id == obligation.configuration_id
        && run.procedure_revision == obligation.procedure_revision
        && run.service_kind == obligation.service_kind
        && run.lifecycle_phase == obligation.lifecycle_phase
        && run.evidence_keys.iter().any(|key| key == &obligation.evidence_key)
}

fn exact_conditions_match(run: &TestRun, obligation: &Obligation) -> bool {
    obligation
        .conditions
        .iter()
        .all(|(factor, level)| run.conditions.get(factor) == Some(level))
}

fn strict_run_groups(obligation: &Obligation, runs: &[TestRun]) -> BTreeSet<String> {
    runs.iter()
        .filter(|run| {
            run_has_valid_evidence(run)
                && run_matches_scope(run, obligation)
                && exact_conditions_match(run, obligation)
        })
        .map(|run| run.run_group_id.clone())
        .collect()
}

fn ofat_baseline_covers(obligation: &Obligation, runs: &[TestRun]) -> bool {
    if obligation.conditions.is_empty() {
        return !strict_run_groups(obligation, runs).is_empty();
    }

    obligation.conditions.iter().all(|(factor, level)| {
        runs.iter().any(|run| {
            run_has_valid_evidence(run)
                && run_matches_scope(run, obligation)
                && run.conditions.get(factor) == Some(level)
        })
    })
}

fn pairwise_baseline_covers(obligation: &Obligation, runs: &[TestRun]) -> bool {
    let factors: Vec<_> = obligation.conditions.iter().collect();
    if factors.len() <= 2 {
        return ofat_baseline_covers(obligation, runs);
    }

    for left_index in 0..factors.len() {
        for right_index in (left_index + 1)..factors.len() {
            let (left_factor, left_level) = factors[left_index];
            let (right_factor, right_level) = factors[right_index];
            let pair_seen = runs.iter().any(|run| {
                run_has_valid_evidence(run)
                    && run_matches_scope(run, obligation)
                    && run.conditions.get(left_factor) == Some(left_level)
                    && run.conditions.get(right_factor) == Some(right_level)
            });
            if !pair_seen {
                return false;
            }
        }
    }
    true
}

fn evaluate_case(case: &CampaignCase) -> Result<Decision, String> {
    if case.case_id.is_empty() || case.obligations.is_empty() {
        return Err("case ID and obligation set must be non-empty".to_owned());
    }
    if case
        .preregistration_ref
        .as_deref()
        .is_none_or(|reference| reference.trim().is_empty())
    {
        return Ok(Decision {
            disposition: Disposition::Rejected,
            covered: 0,
            missing: case.obligations.len(),
            eligible_runs: 0,
            failed_runs: 0,
            aborted_runs: 0,
            invalid_runs: 0,
            planned_runs: 0,
            duplicate_runs: 0,
            budget_exceeded: false,
            cycle_count_baseline_pass: false,
            ofat_false_coverage_count: 0,
            pairwise_false_coverage_count: 0,
        });
    }

    let mut obligation_ids = HashSet::new();
    for obligation in &case.obligations {
        if obligation.id.is_empty() || !obligation_ids.insert(obligation.id.as_str()) {
            return Err("obligation IDs must be non-empty and unique".to_owned());
        }
        if obligation.configuration_id.is_empty()
            || obligation.procedure_revision.is_empty()
            || obligation.service_kind.is_empty()
            || obligation.lifecycle_phase.is_empty()
            || obligation.evidence_key.is_empty()
            || obligation.minimum_run_groups == 0
        {
            return Err(format!("obligation {} has incomplete scope", obligation.id));
        }
        if obligation.not_applicable_reason.is_some()
            || obligation.not_applicable_review_ref.is_some()
        {
            if !has_not_applicable_review_reference(obligation) {
                return Ok(Decision {
                disposition: Disposition::Rejected,
                covered: 0,
                missing: case.obligations.len(),
                eligible_runs: 0,
                failed_runs: 0,
                aborted_runs: 0,
                invalid_runs: 0,
                planned_runs: 0,
                duplicate_runs: 0,
                budget_exceeded: false,
                cycle_count_baseline_pass: false,
                ofat_false_coverage_count: 0,
                    pairwise_false_coverage_count: 0,
                });
            }
        }
    }

    let mut by_id: HashMap<&str, &TestRun> = HashMap::new();
    let mut unique_runs = Vec::with_capacity(case.runs.len());
    let mut duplicate_runs = 0usize;
    for run in &case.runs {
        if run.run_id.is_empty() {
            return Err("test run IDs must be non-empty".to_owned());
        }
        if let Some(previous) = by_id.get(run.run_id.as_str()) {
            if *previous == run {
                duplicate_runs += 1;
                continue;
            }
            return Ok(Decision {
                disposition: Disposition::Disputed,
                covered: 0,
                missing: case.obligations.len(),
                eligible_runs: 0,
                failed_runs: 0,
                aborted_runs: 0,
                invalid_runs: 0,
                planned_runs: 0,
                duplicate_runs,
                budget_exceeded: false,
                cycle_count_baseline_pass: false,
                ofat_false_coverage_count: 0,
                pairwise_false_coverage_count: 0,
            });
        }
        by_id.insert(&run.run_id, run);
        unique_runs.push(run.clone());
    }

    let eligible_runs = unique_runs
        .iter()
        .filter(|run| run_has_valid_evidence(run))
        .count();
    let failed_runs = unique_runs
        .iter()
        .filter(|run| run.outcome == TestOutcome::Failed && run_has_valid_evidence(run))
        .count();
    let aborted_runs = unique_runs
        .iter()
        .filter(|run| run.outcome == TestOutcome::Aborted)
        .count();
    let invalid_runs = unique_runs
        .iter()
        .filter(|run| {
            run.outcome == TestOutcome::Invalid
                || (is_completed(run.outcome) && !run_has_valid_evidence(run))
        })
        .count();
    let planned_runs = unique_runs
        .iter()
        .filter(|run| run.outcome == TestOutcome::Planned)
        .count();
    let executed_cost = unique_runs
        .iter()
        .filter(|run| run.outcome != TestOutcome::Planned)
        .try_fold(0u64, |total, run| total.checked_add(run.cost_units));
    let budget_exceeded = executed_cost.is_none_or(|cost| cost > case.cost_budget);

    let passed_cycle_count = unique_runs
        .iter()
        .filter(|run| run.outcome == TestOutcome::Passed)
        .count();
    let cycle_count_baseline_pass = passed_cycle_count >= case.cycle_count_threshold;

    let mut covered = 0usize;
    let mut missing = Vec::new();
    let mut ofat_false_coverage_count = 0usize;
    let mut pairwise_false_coverage_count = 0usize;
    let mut failure_in_covered = false;

    for obligation in &case.obligations {
        if has_not_applicable_review_reference(obligation) {
            covered += 1;
            continue;
        }
        let groups = strict_run_groups(obligation, &unique_runs);
        let strict_covered = groups.len() >= obligation.minimum_run_groups;
        let required = obligation.criticality != Criticality::Informational;

        if strict_covered {
            covered += 1;
            failure_in_covered |= unique_runs.iter().any(|run| {
                run.outcome == TestOutcome::Failed
                    && run_has_valid_evidence(run)
                    && run_matches_scope(run, obligation)
                    && exact_conditions_match(run, obligation)
            });
        } else if required {
            missing.push(obligation.id.clone());
        }

        if required && !strict_covered && ofat_baseline_covers(obligation, &unique_runs) {
            ofat_false_coverage_count += 1;
        }
        if required && !strict_covered && pairwise_baseline_covers(obligation, &unique_runs) {
            pairwise_false_coverage_count += 1;
        }
    }

    let disposition = if !missing.is_empty() {
        if eligible_runs == 0 {
            Disposition::Indeterminate
        } else {
            Disposition::CoverageGap
        }
    } else if failure_in_covered {
        Disposition::CoverageCompleteWithFailures
    } else {
        Disposition::CoverageComplete
    };

    Ok(Decision {
        disposition,
        covered,
        missing: missing.len(),
        eligible_runs,
        failed_runs,
        aborted_runs,
        invalid_runs,
        planned_runs,
        duplicate_runs,
        budget_exceeded,
        cycle_count_baseline_pass,
        ofat_false_coverage_count,
        pairwise_false_coverage_count,
    })
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != CORPUS_SCHEMA {
        return Err("unsupported test-matrix corpus schema".to_owned());
    }
    if corpus.evidence_class != "synthetic" {
        return Err("v0.1 accepts synthetic campaign fixtures only".to_owned());
    }
    if corpus.claim_ceiling != CLAIM_CEILING {
        return Err("unsupported test-matrix claim ceiling".to_owned());
    }
    if corpus.cases.is_empty() {
        return Err("test-matrix corpus must contain cases".to_owned());
    }

    let mut case_ids = HashSet::with_capacity(corpus.cases.len());
    let mut summary = Summary {
        case_count: corpus.cases.len(),
        coverage_complete: 0,
        coverage_complete_with_failures: 0,
        coverage_gaps: 0,
        indeterminate: 0,
        rejected: 0,
        disputed: 0,
        eligible_runs: 0,
        failed_runs: 0,
        aborted_runs: 0,
        invalid_runs: 0,
        planned_runs: 0,
        duplicate_runs: 0,
        budget_exceeded_campaigns: 0,
        cycle_count_false_pass_campaigns: 0,
        ofat_false_coverage_obligations: 0,
        pairwise_false_coverage_obligations: 0,
        expected_result_mismatches: 0,
    };
    let mut results = Vec::with_capacity(corpus.cases.len());

    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("campaign case IDs must be unique".to_owned());
        }
        let decision = evaluate_case(case)?;
        match decision.disposition {
            Disposition::CoverageComplete => summary.coverage_complete += 1,
            Disposition::CoverageCompleteWithFailures => {
                summary.coverage_complete_with_failures += 1
            }
            Disposition::CoverageGap => summary.coverage_gaps += 1,
            Disposition::Indeterminate => summary.indeterminate += 1,
            Disposition::Rejected => summary.rejected += 1,
            Disposition::Disputed => summary.disputed += 1,
        }
        summary.eligible_runs += decision.eligible_runs;
        summary.failed_runs += decision.failed_runs;
        summary.aborted_runs += decision.aborted_runs;
        summary.invalid_runs += decision.invalid_runs;
        summary.planned_runs += decision.planned_runs;
        summary.duplicate_runs += decision.duplicate_runs;
        summary.budget_exceeded_campaigns += usize::from(decision.budget_exceeded);
        summary.cycle_count_false_pass_campaigns += usize::from(
            decision.cycle_count_baseline_pass
                && !matches!(
                    decision.disposition,
                    Disposition::CoverageComplete | Disposition::CoverageCompleteWithFailures
                ),
        );
        summary.ofat_false_coverage_obligations += decision.ofat_false_coverage_count;
        summary.pairwise_false_coverage_obligations += decision.pairwise_false_coverage_count;

        let mut missing_ids = Vec::new();
        for obligation in &case.obligations {
            if obligation.criticality == Criticality::Informational
                || (obligation.not_applicable_approved
                    && obligation.not_applicable_reason.is_some())
            {
                continue;
            }
            let fail_closed = matches!(
                decision.disposition,
                Disposition::Rejected | Disposition::Disputed
            );
            let groups = strict_run_groups(obligation, &case.runs);
            if fail_closed || groups.len() < obligation.minimum_run_groups {
                missing_ids.push(obligation.id.clone());
            }
        }
        missing_ids.sort();

        let expected = &case.expected;
        let mut expected_missing = expected.missing_obligation_ids.clone();
        expected_missing.sort();
        let matched = decision.disposition == expected.disposition
            && decision.covered == expected.covered_obligations
            && decision.eligible_runs == expected.eligible_runs
            && decision.failed_runs == expected.failed_runs
            && decision.aborted_runs == expected.aborted_runs
            && decision.invalid_runs == expected.invalid_runs
            && decision.planned_runs == expected.planned_runs
            && decision.duplicate_runs == expected.duplicate_runs
            && decision.budget_exceeded == expected.budget_exceeded
            && decision.cycle_count_baseline_pass == expected.cycle_count_baseline_pass
            && decision.ofat_false_coverage_count == expected.ofat_false_coverage_count
            && decision.pairwise_false_coverage_count == expected.pairwise_false_coverage_count
            && decision.missing == expected_missing.len()
            && missing_ids == expected_missing;
        if !matched {
            summary.expected_result_mismatches += 1;
        }

        results.push(CaseResult {
            case_id: case.case_id.clone(),
            disposition: decision.disposition,
            covered_obligations: decision.covered,
            missing_obligation_ids: missing_ids,
            eligible_runs: decision.eligible_runs,
            failed_runs: decision.failed_runs,
            aborted_runs: decision.aborted_runs,
            invalid_runs: decision.invalid_runs,
            planned_runs: decision.planned_runs,
            duplicate_runs: decision.duplicate_runs,
            budget_exceeded: decision.budget_exceeded,
            cycle_count_baseline_pass: decision.cycle_count_baseline_pass,
            ofat_false_coverage_count: decision.ofat_false_coverage_count,
            pairwise_false_coverage_count: decision.pairwise_false_coverage_count,
            expected_result_matched: matched,
        });
    }

    Ok(Report {
        result: if summary.expected_result_mismatches == 0 {
            "pass".to_owned()
        } else {
            "fail".to_owned()
        },
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        summary,
        cases: results,
    })
}

pub fn corpus_schema() -> &'static str {
    CORPUS_SCHEMA
}
