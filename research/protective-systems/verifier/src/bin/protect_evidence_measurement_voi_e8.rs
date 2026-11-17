//! E8 deterministic measurement-selection oracle.
//!
//! The planner uses a finite, explicitly declared hypothesis set and complete
//! outcome partitions. It emits no priors, likelihoods, confidence values or
//! expected information gain. It enumerates small test sets, computes
//! worst-case residual hypothesis classes and reports a Pareto frontier over
//! ambiguity, cost and sequential duration. All fixtures are synthetic and
//! every plan is advisory only.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::process;

const FIXTURE_BYTES: &[u8] = include_bytes!(
    "../../../fixtures/protect-evidence-measurement-voi-e8-v0.1/measurement-selection-corpus.json"
);
const SCHEMA_TEXT: &str = include_str!(
    "../../../schemas/protect-evidence-measurement-voi-e8-v0.1.schema.json"
);
const RULE_VERSION: &str = "protect-evidence-measurement-voi-e8/0.1";
const CLAIM_CEILING: &str = "DETERMINISTIC_MEASUREMENT_DISCRIMINATION_ONLY";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct Hypothesis {
    hypothesis_id: String,
    description: String,
    known_truth_label: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct HypothesisSet {
    set_id: String,
    closed_world: bool,
    hypotheses: Vec<Hypothesis>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct ClaimScope {
    claim_id: String,
    subject_id: String,
    configuration_digest: String,
    quantity: String,
    unit: String,
    valid_from: String,
    valid_to: String,
    conflict_evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct Constraints {
    maximum_tests_per_plan: usize,
    cost_unit: String,
    duration_unit: String,
    mandatory_safety_floor: String,
    required_independence_state: String,
    unknown_status_policy: String,
    open_world_policy: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct OutcomePartition {
    outcome_id: String,
    hypothesis_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct CandidateMeasurement {
    candidate_id: String,
    method_id: String,
    description: String,
    configuration_digest: String,
    subject_id: String,
    quantity: String,
    unit: String,
    calibration_status: String,
    authority_status: String,
    safety_status: String,
    independence_status: String,
    independence_group_id: String,
    dependency_evidence_ids: Vec<String>,
    cost: u64,
    duration_minutes: u64,
    outcomes: Vec<OutcomePartition>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct Fixture {
    schema_version: String,
    evidence_class: String,
    claim_ceiling: String,
    hypothesis_set: HypothesisSet,
    claim_scope: ClaimScope,
    constraints: Constraints,
    candidates: Vec<CandidateMeasurement>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct CandidateRejection {
    candidate_id: String,
    reason_code: String,
    detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct OutcomeClass {
    outcome_signature: Vec<String>,
    remaining_hypothesis_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct MeasurementPlan {
    plan_id: String,
    candidate_ids: Vec<String>,
    cost: u64,
    duration_minutes: u64,
    worst_case_remaining_hypothesis_count: usize,
    residual_outcome_classes: Vec<OutcomeClass>,
    fully_identifies_declared_hypothesis_set: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct SourceReceipt {
    fixture_sha256: String,
    fixture_byte_length: usize,
    evidence_class: &'static str,
    claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PlannerReport {
    schema_version: &'static str,
    rule_version: &'static str,
    claim_ceiling: &'static str,
    status: &'static str,
    objective_rule: &'static str,
    independence_validation_scope: &'static str,
    probabilities_emitted: bool,
    priors_assumed: bool,
    hypothesis_set_id: String,
    declared_hypothesis_count: usize,
    closed_world_hypothesis_set: bool,
    admissible_candidate_ids: Vec<String>,
    rejected_candidates: Vec<CandidateRejection>,
    enumerated_plan_count: usize,
    pareto_frontier: Vec<MeasurementPlan>,
    recommended_plan: Option<MeasurementPlan>,
    source_receipt: SourceReceipt,
    advisory_only: bool,
    physical_execution_authorized: bool,
}

fn parse_fixture() -> Result<Fixture, String> {
    let value: Value = serde_json::from_slice(FIXTURE_BYTES)
        .map_err(|error| format!("E8 fixture JSON invalid: {error}"))?;
    let schema: Value = serde_json::from_str(SCHEMA_TEXT)
        .map_err(|error| format!("E8 fixture schema JSON invalid: {error}"))?;
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("E8 schema build failed: {error}"))?;
    validator
        .validate(&value)
        .map_err(|error| format!("E8 fixture schema validation failed: {error}"))?;
    serde_json::from_value(value).map_err(|error| format!("E8 fixture model invalid: {error}"))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn unique_strings(values: &[String], label: &str) -> Result<BTreeSet<String>, String> {
    let mut result = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(format!("{label} contains an empty identity"));
        }
        if !result.insert(value.clone()) {
            return Err(format!("{label} contains duplicate identity: {value}"));
        }
    }
    Ok(result)
}

fn validate_fixture(fixture: &Fixture) -> Result<BTreeSet<String>, String> {
    if fixture.schema_version != RULE_VERSION
        || fixture.evidence_class != "synthetic"
        || fixture.claim_ceiling != CLAIM_CEILING
    {
        return Err("E8 fixture must remain synthetic and discrimination-only".to_owned());
    }
    if fixture.claim_scope.valid_from >= fixture.claim_scope.valid_to {
        return Err("E8 claim valid-time interval is invalid".to_owned());
    }
    if fixture.constraints.maximum_tests_per_plan == 0
        || fixture.constraints.maximum_tests_per_plan > 8
    {
        return Err("E8 maximum_tests_per_plan must be in 1..=8".to_owned());
    }
    if fixture.hypothesis_set.hypotheses.is_empty() {
        return Err("E8 hypothesis set must not be empty".to_owned());
    }
    let hypothesis_ids = fixture
        .hypothesis_set
        .hypotheses
        .iter()
        .map(|item| item.hypothesis_id.clone())
        .collect::<Vec<_>>();
    let hypothesis_set = unique_strings(&hypothesis_ids, "hypothesis set")?;
    if hypothesis_set.len() > 64 {
        return Err("E8 finite hypothesis set exceeds the 64-hypothesis safety bound".to_owned());
    }
    if fixture.candidates.len() > 16 {
        return Err("E8 candidate catalog exceeds the 16-candidate exhaustive-search bound".to_owned());
    }
    let candidate_ids = fixture
        .candidates
        .iter()
        .map(|item| item.candidate_id.clone())
        .collect::<Vec<_>>();
    unique_strings(&candidate_ids, "candidate catalog")?;
    unique_strings(
        &fixture.claim_scope.conflict_evidence_ids,
        "conflict evidence set",
    )?;
    for candidate in &fixture.candidates {
        if candidate.independence_group_id.trim().is_empty() {
            return Err(format!("{}.independence_group_id is required", candidate.candidate_id));
        }
        unique_strings(
            &candidate.dependency_evidence_ids,
            &format!("{}.dependency_evidence_ids", candidate.candidate_id),
        )?;
        if !candidate.cost.checked_add(candidate.duration_minutes).is_some() {
            return Err(format!("{}.cost/duration overflow", candidate.candidate_id));
        }
    }
    Ok(hypothesis_set)
}

fn admissibility_reason(
    candidate: &CandidateMeasurement,
    scope: &ClaimScope,
    constraints: &Constraints,
) -> Option<(&'static str, String)> {
    if candidate.subject_id != scope.subject_id
        || candidate.configuration_digest != scope.configuration_digest
    {
        return Some((
            "TARGET_SCOPE_MISMATCH",
            "measurement candidate targets a different subject or configuration digest".to_owned(),
        ));
    }
    if candidate.calibration_status != "VALID" {
        return Some((
            "CALIBRATION_NOT_VALID",
            format!("calibration_status={}", candidate.calibration_status),
        ));
    }
    if candidate.authority_status != "QUALIFIED" {
        return Some((
            "METHOD_AUTHORITY_NOT_QUALIFIED",
            format!("authority_status={}", candidate.authority_status),
        ));
    }
    if candidate.safety_status != constraints.mandatory_safety_floor {
        return Some((
            "MANDATORY_SAFETY_FLOOR_NOT_MET",
            format!("safety_status={}", candidate.safety_status),
        ));
    }
    if candidate.independence_status != constraints.required_independence_state {
        return Some((
            "INDEPENDENCE_NOT_ADMITTED",
            format!("independence_status={}", candidate.independence_status),
        ));
    }
    let conflict_ids = scope
        .conflict_evidence_ids
        .iter()
        .collect::<BTreeSet<_>>();
    if candidate
        .dependency_evidence_ids
        .iter()
        .any(|dependency| conflict_ids.contains(dependency))
    {
        return Some((
            "SHARES_CONFLICTING_EVIDENCE",
            "candidate depends on an evidence source in the unresolved conflict set".to_owned(),
        ));
    }
    None
}

fn validate_partition(
    candidate: &CandidateMeasurement,
    hypothesis_ids: &BTreeSet<String>,
) -> Result<(), (&'static str, String)> {
    let mut seen = BTreeSet::new();
    let mut outcome_ids = BTreeSet::new();
    for outcome in &candidate.outcomes {
        if outcome.outcome_id.trim().is_empty() || !outcome_ids.insert(outcome.outcome_id.clone()) {
            return Err((
                "OUTCOME_ID_EMPTY_OR_DUPLICATE",
                "outcome identifiers must be non-empty and unique".to_owned(),
            ));
        }
        if outcome.hypothesis_ids.is_empty() {
            return Err((
                "OUTCOME_PARTITION_EMPTY",
                format!("outcome {} has no compatible hypotheses", outcome.outcome_id),
            ));
        }
        for hypothesis_id in &outcome.hypothesis_ids {
            if !hypothesis_ids.contains(hypothesis_id) {
                return Err((
                    "OUTCOME_REFERENCES_UNKNOWN_HYPOTHESIS",
                    format!("outcome references unknown hypothesis {hypothesis_id}"),
                ));
            }
            if !seen.insert(hypothesis_id.clone()) {
                return Err((
                    "OUTCOME_PARTITION_OVERLAPS",
                    format!("hypothesis {hypothesis_id} occurs in multiple outcomes"),
                ));
            }
        }
    }
    if &seen != hypothesis_ids {
        let missing = hypothesis_ids.difference(&seen).cloned().collect::<Vec<_>>();
        return Err((
            "OUTCOME_PARTITION_INCOMPLETE",
            format!("outcome map omits hypotheses: {}", missing.join(", ")),
        ));
    }
    Ok(())
}

fn outcome_for_hypothesis<'a>(
    candidate: &'a CandidateMeasurement,
    hypothesis_id: &str,
) -> &'a str {
    candidate
        .outcomes
        .iter()
        .find(|outcome| outcome.hypothesis_ids.iter().any(|id| id == hypothesis_id))
        .map(|outcome| outcome.outcome_id.as_str())
        .expect("partition is validated before plan enumeration")
}

fn evaluate_plan(
    candidates: &[CandidateMeasurement],
    hypothesis_ids: &[String],
) -> Result<MeasurementPlan, String> {
    if candidates.is_empty() {
        return Err("cannot evaluate an empty test plan".to_owned());
    }
    let mut group_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for hypothesis_id in hypothesis_ids {
        let signature = candidates
            .iter()
            .map(|candidate| outcome_for_hypothesis(candidate, hypothesis_id).to_owned())
            .collect::<Vec<_>>();
        let key = serde_json::to_string(&signature)
            .map_err(|error| format!("outcome signature serialization failed: {error}"))?;
        group_map.entry(key).or_default().push(hypothesis_id.clone());
    }
    let mut residual_outcome_classes = group_map
        .into_iter()
        .map(|(key, mut ids)| {
            ids.sort();
            let signature: Vec<String> = serde_json::from_str(&key)
                .expect("signature was serialized from a vector of strings");
            OutcomeClass {
                outcome_signature: signature,
                remaining_hypothesis_ids: ids,
            }
        })
        .collect::<Vec<_>>();
    residual_outcome_classes.sort_by(|left, right| {
        left.outcome_signature.cmp(&right.outcome_signature)
    });
    let worst_case = residual_outcome_classes
        .iter()
        .map(|class| class.remaining_hypothesis_ids.len())
        .max()
        .unwrap_or(hypothesis_ids.len());
    let mut candidate_ids = candidates
        .iter()
        .map(|candidate| candidate.candidate_id.clone())
        .collect::<Vec<_>>();
    candidate_ids.sort();
    let cost = candidates.iter().try_fold(0_u64, |sum, candidate| {
        sum.checked_add(candidate.cost)
            .ok_or_else(|| "plan cost overflow".to_owned())
    })?;
    let duration = candidates.iter().try_fold(0_u64, |sum, candidate| {
        sum.checked_add(candidate.duration_minutes)
            .ok_or_else(|| "plan duration overflow".to_owned())
    })?;
    let plan_bytes = serde_json::to_vec(&candidate_ids)
        .map_err(|error| format!("plan identity serialization failed: {error}"))?;
    Ok(MeasurementPlan {
        plan_id: format!("{:x}", Sha256::digest(plan_bytes)),
        candidate_ids,
        cost,
        duration_minutes: duration,
        worst_case_remaining_hypothesis_count: worst_case,
        residual_outcome_classes,
        fully_identifies_declared_hypothesis_set: worst_case == 1,
    })
}

fn enumerate_subsets(
    candidates: &[CandidateMeasurement],
    hypothesis_ids: &[String],
    maximum_tests: usize,
    start: usize,
    current: &mut Vec<CandidateMeasurement>,
    plans: &mut Vec<MeasurementPlan>,
) -> Result<(), String> {
    if !current.is_empty() {
        plans.push(evaluate_plan(current, hypothesis_ids)?);
    }
    if current.len() >= maximum_tests {
        return Ok(());
    }
    for index in start..candidates.len() {
        let candidate = &candidates[index];
        if current.iter().any(|selected| {
            selected.independence_group_id == candidate.independence_group_id
                || selected
                    .dependency_evidence_ids
                    .iter()
                    .any(|dependency| candidate.dependency_evidence_ids.contains(dependency))
        }) {
            continue;
        }
        current.push(candidate.clone());
        enumerate_subsets(
            candidates,
            hypothesis_ids,
            maximum_tests,
            index + 1,
            current,
            plans,
        )?;
        current.pop();
    }
    Ok(())
}

fn dominates(left: &MeasurementPlan, right: &MeasurementPlan) -> bool {
    let no_worse = left.worst_case_remaining_hypothesis_count
        <= right.worst_case_remaining_hypothesis_count
        && left.cost <= right.cost
        && left.duration_minutes <= right.duration_minutes;
    let strictly_better = left.worst_case_remaining_hypothesis_count
        < right.worst_case_remaining_hypothesis_count
        || left.cost < right.cost
        || left.duration_minutes < right.duration_minutes;
    no_worse && strictly_better
}

fn pareto_frontier(plans: &[MeasurementPlan]) -> Vec<MeasurementPlan> {
    let mut frontier = plans
        .iter()
        .filter(|candidate| {
            !plans
                .iter()
                .any(|other| other.plan_id != candidate.plan_id && dominates(other, candidate))
        })
        .cloned()
        .collect::<Vec<_>>();
    frontier.sort_by(|left, right| {
        (
            left.worst_case_remaining_hypothesis_count,
            left.duration_minutes,
            left.cost,
            &left.candidate_ids,
        )
            .cmp(&(
                right.worst_case_remaining_hypothesis_count,
                right.duration_minutes,
                right.cost,
                &right.candidate_ids,
            ))
    });
    frontier
}

fn build_report(fixture: &Fixture) -> Result<PlannerReport, String> {
    let hypothesis_set = validate_fixture(fixture)?;
    let mut rejected_candidates = Vec::new();
    let mut admissible_candidates = Vec::new();
    let mut sorted_candidates = fixture.candidates.clone();
    sorted_candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    for candidate in sorted_candidates {
        if let Some((reason_code, detail)) =
            admissibility_reason(&candidate, &fixture.claim_scope, &fixture.constraints)
        {
            rejected_candidates.push(CandidateRejection {
                candidate_id: candidate.candidate_id,
                reason_code: reason_code.to_owned(),
                detail,
            });
            continue;
        }
        if let Err((reason_code, detail)) = validate_partition(&candidate, &hypothesis_set) {
            rejected_candidates.push(CandidateRejection {
                candidate_id: candidate.candidate_id,
                reason_code: reason_code.to_owned(),
                detail,
            });
            continue;
        }
        admissible_candidates.push(candidate);
    }
    rejected_candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    let mut all_plans = Vec::new();
    if fixture.hypothesis_set.closed_world && !admissible_candidates.is_empty() {
        enumerate_subsets(
            &admissible_candidates,
            &hypothesis_set.iter().cloned().collect::<Vec<_>>(),
            fixture.constraints.maximum_tests_per_plan,
            0,
            &mut Vec::new(),
            &mut all_plans,
        )?;
    }
    all_plans.sort_by(|left, right| left.plan_id.cmp(&right.plan_id));
    let frontier = if fixture.hypothesis_set.closed_world {
        pareto_frontier(&all_plans)
    } else {
        Vec::new()
    };
    let recommended_plan = frontier
        .iter()
        .filter(|plan| {
            plan.worst_case_remaining_hypothesis_count
                < fixture.hypothesis_set.hypotheses.len()
        })
        .min_by(|left, right| {
            (
                left.worst_case_remaining_hypothesis_count,
                left.duration_minutes,
                left.cost,
                &left.candidate_ids,
            )
                .cmp(&(
                    right.worst_case_remaining_hypothesis_count,
                    right.duration_minutes,
                    right.cost,
                    &right.candidate_ids,
                ))
        })
        .cloned();
    let status = if !fixture.hypothesis_set.closed_world {
        "ABSTAIN_OPEN_WORLD_HYPOTHESES"
    } else if admissible_candidates.is_empty() {
        "NO_ADMISSIBLE_DISCRIMINATING_TEST"
    } else if recommended_plan.is_none() {
        "NO_ADMISSIBLE_DISCRIMINATING_TEST"
    } else {
        "PLAN_AVAILABLE"
    };
    Ok(PlannerReport {
        schema_version: RULE_VERSION,
        rule_version: RULE_VERSION,
        claim_ceiling: CLAIM_CEILING,
        status,
        objective_rule: "MINIMAX_RESIDUAL_THEN_DURATION_THEN_COST",
        independence_validation_scope: "SYNTHETIC_ATTESTATION_MARKER_ONLY; signer identity and physical measurement independence are not authenticated",
        probabilities_emitted: false,
        priors_assumed: false,
        hypothesis_set_id: fixture.hypothesis_set.set_id.clone(),
        declared_hypothesis_count: fixture.hypothesis_set.hypotheses.len(),
        closed_world_hypothesis_set: fixture.hypothesis_set.closed_world,
        admissible_candidate_ids: admissible_candidates
            .iter()
            .map(|candidate| candidate.candidate_id.clone())
            .collect(),
        rejected_candidates,
        enumerated_plan_count: all_plans.len(),
        pareto_frontier: frontier,
        recommended_plan,
        source_receipt: SourceReceipt {
            fixture_sha256: sha256(FIXTURE_BYTES),
            fixture_byte_length: FIXTURE_BYTES.len(),
            evidence_class: "synthetic",
            claim_ceiling: CLAIM_CEILING,
        },
        advisory_only: true,
        physical_execution_authorized: false,
    })
}

fn run() -> Result<PlannerReport, String> {
    let fixture = parse_fixture()?;
    build_report(&fixture)
}

fn main() {
    match run() {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("E8 report serialization")
        ),
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        parse_fixture().expect("valid E8 fixture")
    }

    fn known_hypotheses() -> Vec<String> {
        fixture()
            .hypothesis_set
            .hypotheses
            .iter()
            .map(|hypothesis| hypothesis.hypothesis_id.clone())
            .collect()
    }

    fn frontier_oracle(fixture: &Fixture) -> Vec<MeasurementPlan> {
        // Independent test oracle: enumerate bit masks instead of using the
        // production recursive subset enumerator.
        let hypothesis_ids = known_hypotheses();
        let mut candidates = fixture
            .candidates
            .iter()
            .filter(|candidate| {
                admissibility_reason(candidate, &fixture.claim_scope, &fixture.constraints)
                    .is_none()
                    && validate_partition(
                        candidate,
                        &hypothesis_ids.iter().cloned().collect(),
                    )
                    .is_ok()
            })
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

        let mut plans = Vec::new();
        for mask in 1_u64..(1_u64 << candidates.len()) {
            let count = mask.count_ones() as usize;
            if count > fixture.constraints.maximum_tests_per_plan {
                continue;
            }
            let selected = candidates
                .iter()
                .enumerate()
                .filter(|(index, _)| (mask & (1_u64 << index)) != 0)
                .map(|(_, candidate)| candidate)
                .collect::<Vec<_>>();
            let mut groups = BTreeSet::new();
            let mut dependencies = BTreeSet::new();
            let mut compatible = true;
            for candidate in &selected {
                if !groups.insert(candidate.independence_group_id.clone()) {
                    compatible = false;
                    break;
                }
                for dependency in &candidate.dependency_evidence_ids {
                    if !dependencies.insert(dependency.clone()) {
                        compatible = false;
                        break;
                    }
                }
                if !compatible {
                    break;
                }
            }
            if !compatible {
                continue;
            }

            let mut classes: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
            for hypothesis in &hypothesis_ids {
                let signature = selected
                    .iter()
                    .map(|candidate| {
                        candidate
                            .outcomes
                            .iter()
                            .find(|outcome| outcome.hypothesis_ids.contains(hypothesis))
                            .expect("complete partition")
                            .outcome_id
                            .clone()
                    })
                    .collect::<Vec<_>>();
                classes.entry(signature).or_default().push(hypothesis.clone());
            }
            let mut residual = classes
                .into_iter()
                .map(|(signature, mut hypotheses)| {
                    hypotheses.sort();
                    OutcomeClass {
                        outcome_signature: signature,
                        remaining_hypothesis_ids: hypotheses,
                    }
                })
                .collect::<Vec<_>>();
            residual.sort_by(|left, right| left.outcome_signature.cmp(&right.outcome_signature));
            let worst = residual
                .iter()
                .map(|class| class.remaining_hypothesis_ids.len())
                .max()
                .unwrap_or(hypothesis_ids.len());
            let cost = selected.iter().map(|candidate| candidate.cost).sum::<u64>();
            let duration = selected
                .iter()
                .map(|candidate| candidate.duration_minutes)
                .sum::<u64>();
            let mut ids = selected
                .iter()
                .map(|candidate| candidate.candidate_id.clone())
                .collect::<Vec<_>>();
            ids.sort();
            let bytes = serde_json::to_vec(&ids).expect("plan IDs serialize");
            plans.push(MeasurementPlan {
                plan_id: format!("{:x}", Sha256::digest(bytes)),
                candidate_ids: ids,
                cost,
                duration_minutes: duration,
                worst_case_remaining_hypothesis_count: worst,
                residual_outcome_classes: residual,
                fully_identifies_declared_hypothesis_set: worst == 1,
            });
        }

        let mut frontier = plans
            .iter()
            .filter(|candidate| {
                !plans.iter().any(|other| {
                    let no_worse = other.worst_case_remaining_hypothesis_count
                        <= candidate.worst_case_remaining_hypothesis_count
                        && other.cost <= candidate.cost
                        && other.duration_minutes <= candidate.duration_minutes;
                    let strictly = other.worst_case_remaining_hypothesis_count
                        < candidate.worst_case_remaining_hypothesis_count
                        || other.cost < candidate.cost
                        || other.duration_minutes < candidate.duration_minutes;
                    other.plan_id != candidate.plan_id && no_worse && strictly
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        frontier.sort_by(|left, right| {
            (
                left.worst_case_remaining_hypothesis_count,
                left.duration_minutes,
                left.cost,
                &left.candidate_ids,
            )
                .cmp(&(
                    right.worst_case_remaining_hypothesis_count,
                    right.duration_minutes,
                    right.cost,
                    &right.candidate_ids,
                ))
        });
        frontier
    }

    #[test]
    fn baseline_emits_probability_free_pareto_frontier() {
        let report = run().expect("E8 report");
        assert_eq!(report.status, "PLAN_AVAILABLE");
        assert!(!report.probabilities_emitted);
        assert!(!report.priors_assumed);
        assert!(report.advisory_only);
        assert!(!report.physical_execution_authorized);
        assert!(!report.pareto_frontier.is_empty());
    }

    #[test]
    fn shared_sensor_vendor_repeat_is_rejected() {
        let report = run().expect("report");
        let rejection = report
            .rejected_candidates
            .iter()
            .find(|item| item.candidate_id == "M5_VENDOR_REPEAT_OF_SENSOR_A")
            .expect("shared-source rejection");
        assert_eq!(rejection.reason_code, "INDEPENDENCE_NOT_ADMITTED");
    }

    #[test]
    fn unknown_calibration_candidate_is_rejected() {
        let report = run().expect("report");
        let rejection = report
            .rejected_candidates
            .iter()
            .find(|item| item.candidate_id == "M6_UNCALIBRATED_PRESSURE_TEST")
            .expect("calibration rejection");
        assert_eq!(rejection.reason_code, "CALIBRATION_NOT_VALID");
    }

    #[test]
    fn incomplete_outcome_partition_is_rejected() {
        let report = run().expect("report");
        let rejection = report
            .rejected_candidates
            .iter()
            .find(|item| item.candidate_id == "M7_INCOMPLETE_PARTITION")
            .expect("partition rejection");
        assert_eq!(rejection.reason_code, "OUTCOME_PARTITION_INCOMPLETE");
    }

    #[test]
    fn independent_pressure_profile_fully_identifies_declared_hypotheses() {
        let report = run().expect("report");
        let pressure = report
            .pareto_frontier
            .iter()
            .find(|plan| plan.candidate_ids == vec!["M3_PRESSURE_PROFILE"])
            .expect("pressure plan");
        assert_eq!(pressure.worst_case_remaining_hypothesis_count, 1);
        assert!(pressure.fully_identifies_declared_hypothesis_set);
    }

    #[test]
    fn complementary_inlet_and_endpoint_measurements_identify_all_hypotheses() {
        let report = run().expect("report");
        let plan = report
            .pareto_frontier
            .iter()
            .find(|plan| {
                plan.candidate_ids
                    == vec!["M1_INLET_FLOWMETER", "M2_ENDPOINT_FLOWLOGGER"]
            })
            .expect("complementary plan");
        assert_eq!(plan.worst_case_remaining_hypothesis_count, 1);
        assert!(plan.fully_identifies_declared_hypothesis_set);
    }

    #[test]
    fn pressure_profile_and_complementary_pair_expose_a_cost_time_pareto_tradeoff() {
        let report = run().expect("report");
        let pressure = report.pareto_frontier.iter().find(|plan| {
            plan.candidate_ids == vec!["M3_PRESSURE_PROFILE"]
        }).expect("pressure plan");
        let pair = report.pareto_frontier.iter().find(|plan| {
            plan.candidate_ids == vec!["M1_INLET_FLOWMETER", "M2_ENDPOINT_FLOWLOGGER"]
        }).expect("paired plan");
        assert!(pressure.cost > pair.cost);
        assert!(pressure.duration_minutes < pair.duration_minutes);
        assert_eq!(pressure.worst_case_remaining_hypothesis_count, 1);
        assert_eq!(pair.worst_case_remaining_hypothesis_count, 1);
    }

    #[test]
    fn recommended_plan_uses_declared_lexicographic_objective_not_a_weighted_score() {
        let report = run().expect("report");
        assert_eq!(report.objective_rule, "MINIMAX_RESIDUAL_THEN_DURATION_THEN_COST");
        assert!(report
            .independence_validation_scope
            .contains("not authenticated"));
        let recommended = report.recommended_plan.expect("recommended");
        assert_eq!(
            recommended.candidate_ids,
            vec!["M3_PRESSURE_PROFILE"]
        );
        assert!(recommended.fully_identifies_declared_hypothesis_set);
    }

    #[test]
    fn candidate_with_wrong_configuration_is_rejected() {
        let mut data = fixture();
        data.candidates[0].configuration_digest = "sha256:config-B".to_owned();
        let result = build_report(&data).expect("report");
        assert!(result.rejected_candidates.iter().any(|item| {
            item.candidate_id == "M1_INLET_FLOWMETER"
                && item.reason_code == "TARGET_SCOPE_MISMATCH"
        }));
    }

    #[test]
    fn candidate_that_fails_safety_floor_is_rejected() {
        let mut data = fixture();
        data.candidates[0].safety_status = "UNKNOWN".to_owned();
        let report = build_report(&data).expect("report");
        assert!(report.rejected_candidates.iter().any(|item| {
            item.candidate_id == "M1_INLET_FLOWMETER"
                && item.reason_code == "MANDATORY_SAFETY_FLOOR_NOT_MET"
        }));
    }

    #[test]
    fn candidate_with_unqualified_authority_is_rejected() {
        let mut data = fixture();
        data.candidates[0].authority_status = "UNKNOWN".to_owned();
        let report = build_report(&data).expect("report");
        assert!(report.rejected_candidates.iter().any(|item| {
            item.candidate_id == "M1_INLET_FLOWMETER"
                && item.reason_code == "METHOD_AUTHORITY_NOT_QUALIFIED"
        }));
    }

    #[test]
    fn test_reusing_conflicting_evidence_is_rejected_even_if_marked_admissible() {
        let mut data = fixture();
        data.candidates[0].dependency_evidence_ids.push("ev-sensor-a".to_owned());
        let report = build_report(&data).expect("report");
        assert!(report.rejected_candidates.iter().any(|item| {
            item.candidate_id == "M1_INLET_FLOWMETER"
                && item.reason_code == "SHARES_CONFLICTING_EVIDENCE"
        }));
    }

    #[test]
    fn pair_of_candidates_sharing_independence_group_cannot_form_a_plan() {
        let mut data = fixture();
        data.candidates[1].independence_group_id =
            data.candidates[0].independence_group_id.clone();
        let report = build_report(&data).expect("report");
        assert!(!report.pareto_frontier.iter().any(|plan| {
            plan.candidate_ids
                == vec!["M1_INLET_FLOWMETER", "M2_ENDPOINT_FLOWLOGGER"]
        }));
    }

    #[test]
    fn candidate_outcome_partition_overlap_is_rejected() {
        let mut data = fixture();
        data.candidates[0].outcomes[1]
            .hypothesis_ids
            .push("H1_VALVE_RESTRICTION".to_owned());
        let result = build_report(&data).expect("report");
        assert!(result.rejected_candidates.iter().any(|item| {
            item.candidate_id == "M1_INLET_FLOWMETER"
                && item.reason_code == "OUTCOME_PARTITION_OVERLAPS"
        }));
    }

    #[test]
    fn candidate_with_unknown_hypothesis_is_rejected() {
        let mut data = fixture();
        data.candidates[0].outcomes[0]
            .hypothesis_ids
            .push("H5_UNKNOWN".to_owned());
        let result = build_report(&data).expect("report");
        assert!(result.rejected_candidates.iter().any(|item| {
            item.candidate_id == "M1_INLET_FLOWMETER"
                && item.reason_code == "OUTCOME_REFERENCES_UNKNOWN_HYPOTHESIS"
        }));
    }

    #[test]
    fn open_world_hypotheses_force_abstention() {
        let mut data = fixture();
        data.hypothesis_set.closed_world = false;
        let report = build_report(&data).expect("report");
        assert_eq!(report.status, "ABSTAIN_OPEN_WORLD_HYPOTHESES");
        assert!(report.pareto_frontier.is_empty());
        assert!(report.recommended_plan.is_none());
    }

    #[test]
    fn no_admissible_candidates_returns_abstention() {
        let mut data = fixture();
        for candidate in &mut data.candidates {
            candidate.calibration_status = "UNKNOWN".to_owned();
        }
        let report = build_report(&data).expect("report");
        assert_eq!(report.status, "NO_ADMISSIBLE_DISCRIMINATING_TEST");
        assert!(report.pareto_frontier.is_empty());
        assert!(report.recommended_plan.is_none());
    }

    #[test]
    fn if_no_candidate_reduces_hypothesis_set_planner_abstains() {
        let mut data = fixture();
        for candidate in &mut data.candidates {
            candidate.outcomes = vec![OutcomePartition {
                outcome_id: "NO_INFORMATION".to_owned(),
                hypothesis_ids: known_hypotheses(),
            }];
        }
        let report = build_report(&data).expect("report");
        assert_eq!(report.status, "NO_ADMISSIBLE_DISCRIMINATING_TEST");
        assert!(report.recommended_plan.is_none());
    }

    #[test]
    fn pareto_frontier_matches_exhaustive_oracle() {
        let data = fixture();
        let report = build_report(&data).expect("report");
        assert_eq!(report.pareto_frontier, frontier_oracle(&data));
    }

    #[test]
    fn candidate_and_hypothesis_input_order_does_not_change_result() {
        let mut data = fixture();
        let baseline = build_report(&data).expect("baseline");
        data.candidates.reverse();
        data.hypothesis_set.hypotheses.reverse();
        for candidate in &mut data.candidates {
            candidate.outcomes.reverse();
            for outcome in &mut candidate.outcomes {
                outcome.hypothesis_ids.reverse();
            }
        }
        let reordered = build_report(&data).expect("reordered");
        assert_eq!(baseline, reordered);
    }

    #[test]
    fn same_frozen_input_produces_byte_identical_report() {
        let data = fixture();
        let first = build_report(&data).expect("first");
        let second = build_report(&data).expect("second");
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).expect("first bytes"),
            serde_json::to_vec(&second).expect("second bytes")
        );
    }

    #[test]
    fn report_carries_source_digest_and_no_physical_authorization() {
        let report = run().expect("report");
        assert_eq!(report.source_receipt.fixture_byte_length, FIXTURE_BYTES.len());
        assert_eq!(report.source_receipt.fixture_sha256, sha256(FIXTURE_BYTES));
        assert!(report.advisory_only);
        assert!(!report.physical_execution_authorized);
    }

    #[test]
    fn zero_cost_tests_are_supported_without_division_or_probability_logic() {
        let mut data = fixture();
        data.candidates[0].cost = 0;
        let report = build_report(&data).expect("zero cost report");
        assert!(report.pareto_frontier.iter().any(|plan| plan.cost == 0));
        assert!(!report.probabilities_emitted);
    }
}
