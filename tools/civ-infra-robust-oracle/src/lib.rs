// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const FIXTURE_SCHEMA: &str = "luminous.civ_infra.robust_fixture.v0.1";
pub const REPORT_SCHEMA: &str = "luminous.civ_infra.robust_report.v0.1";
const CLAIM_CEILING: &str = "declared_finite_synthetic_states_only";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RobustFixture {
    pub schema_version: String,
    pub fixture_id: String,
    pub exhaustive_joint_state_set: bool,
    pub uncertainties: Vec<Uncertainty>,
    pub states: Vec<WorldState>,
    pub required_constraints: Vec<String>,
    pub objectives: Vec<ObjectiveSpec>,
    pub candidates: Vec<Candidate>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Uncertainty {
    pub id: String,
    pub unit: String,
    pub support: Support,
    pub evidence_state: EvidenceState,
    pub evidence_ref: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Support {
    FiniteValues { values: Vec<f64> },
    BoundedInterval { lower: f64, upper: f64 },
    UnboundedUnknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    SyntheticFixture,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldState {
    pub id: String,
    pub values: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectiveSpec {
    pub id: String,
    pub unit: String,
    pub direction: ObjectiveDirection,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveDirection {
    Minimize,
    Maximize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub state_results: Vec<CandidateStateResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CandidateStateResult {
    pub state_id: String,
    pub constraints: BTreeMap<String, bool>,
    pub objectives: BTreeMap<String, f64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateDisposition {
    RobustFeasibleForDeclaredSet,
    RobustInfeasibleForDeclaredSet,
    ConditionallyFeasible,
    UnknownReviewRequired,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct StateEvaluation {
    pub state_id: String,
    pub evidence_complete: bool,
    pub feasible: Option<bool>,
    pub violated_constraints: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ObjectiveRange {
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub observed_states: usize,
    pub expected_states: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CandidateEvaluation {
    pub candidate_id: String,
    pub disposition: CandidateDisposition,
    pub review_reasons: Vec<String>,
    pub state_evaluations: Vec<StateEvaluation>,
    pub objective_ranges: BTreeMap<String, ObjectiveRange>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct RobustReport {
    pub schema_version: String,
    pub fixture_id: String,
    pub input_sha256: String,
    pub claim_ceiling: String,
    pub global_review_gates: Vec<String>,
    pub candidates: Vec<CandidateEvaluation>,
    pub robust_pareto_frontier_candidate_ids: Vec<String>,
}

pub fn evaluate_json(bytes: &[u8]) -> Result<RobustReport, Vec<String>> {
    let fixture: RobustFixture = serde_json::from_slice(bytes)
        .map_err(|error| vec![format!("fixture JSON rejected: {error}")])?;
    let errors = validate_fixture(&fixture);
    if !errors.is_empty() {
        return Err(errors);
    }
    let input_sha256 = format!("{:x}", Sha256::digest(bytes));
    Ok(evaluate_fixture(&fixture, input_sha256))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn unique_ids<'a>(
    values: impl Iterator<Item = &'a str>,
    label: &str,
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    for value in values {
        if !nonempty(value) {
            errors.push(format!("{label}: empty ID"));
        }
        if !result.insert(value.to_string()) {
            errors.push(format!("{label}: duplicate ID {value}"));
        }
    }
    result
}

fn same_keys<T>(values: &BTreeMap<String, T>, expected: &BTreeSet<String>) -> bool {
    let observed = values.keys().cloned().collect::<BTreeSet<_>>();
    &observed == expected
}

fn subset_keys<T>(values: &BTreeMap<String, T>, allowed: &BTreeSet<String>) -> bool {
    values.keys().all(|key| allowed.contains(key))
}

const MAX_EXHAUSTIVE_STATES: usize = 100_000;

fn canonical_float_key(value: f64) -> u64 {
    if value == 0.0 {
        0.0f64.to_bits()
    } else {
        value.to_bits()
    }
}

// R0 only certifies exhaustiveness when every uncertainty has finite discrete
// support and every Cartesian tuple appears exactly once. Correlated/joint
// supports need a future explicit dependency contract, not a trusted boolean.
fn validate_cartesian_state_coverage(fixture: &RobustFixture, errors: &mut Vec<String>) {
    if !fixture.exhaustive_joint_state_set
        || fixture
            .uncertainties
            .iter()
            .any(|item| !matches!(&item.support, Support::FiniteValues { .. }))
    {
        return;
    }

    let mut uncertainties = fixture.uncertainties.iter().collect::<Vec<_>>();
    uncertainties.sort_by(|left, right| left.id.cmp(&right.id));

    let mut expected_tuples: BTreeSet<Vec<u64>> = BTreeSet::from([Vec::new()]);
    for uncertainty in &uncertainties {
        let Support::FiniteValues { values } = &uncertainty.support else {
            return;
        };
        let Some(next_len) = expected_tuples.len().checked_mul(values.len()) else {
            errors.push("state coverage: Cartesian product size overflow".to_string());
            return;
        };
        if next_len > MAX_EXHAUSTIVE_STATES {
            errors.push(format!(
                "state coverage: Cartesian product has {next_len} states, exceeding R0 cap \
                 {MAX_EXHAUSTIVE_STATES}"
            ));
            return;
        }

        let mut next = BTreeSet::new();
        for prefix in &expected_tuples {
            for value in values {
                let mut tuple = prefix.clone();
                tuple.push(canonical_float_key(*value));
                next.insert(tuple);
            }
        }
        expected_tuples = next;
    }

    let mut observed_tuples = BTreeSet::new();
    for state in &fixture.states {
        let Some(tuple) = uncertainties
            .iter()
            .map(|uncertainty| state.values.get(&uncertainty.id).copied())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        observed_tuples.insert(
            tuple
                .into_iter()
                .map(canonical_float_key)
                .collect::<Vec<_>>(),
        );
    }

    if observed_tuples != expected_tuples || observed_tuples.len() != fixture.states.len() {
        errors.push(format!(
            "state coverage: exhaustive Cartesian support requires {} unique joint states, \
             but fixture provides {} states and {} unique joint tuples",
            expected_tuples.len(),
            fixture.states.len(),
            observed_tuples.len()
        ));
    }
}

fn validate_fixture(fixture: &RobustFixture) -> Vec<String> {
    let mut errors = Vec::new();
    if fixture.schema_version != FIXTURE_SCHEMA {
        errors.push(format!(
            "unsupported schema_version: {}",
            fixture.schema_version
        ));
    }
    if !nonempty(&fixture.fixture_id) {
        errors.push("fixture_id must not be empty".to_string());
    }
    if fixture.uncertainties.is_empty() {
        errors.push("uncertainty register must not be empty".to_string());
    }
    if fixture.states.is_empty() {
        errors.push("finite joint state set must not be empty".to_string());
    }
    if fixture.required_constraints.is_empty() {
        errors.push("required_constraints must not be empty".to_string());
    }
    if fixture.objectives.is_empty() {
        errors.push("objectives must not be empty".to_string());
    }
    if fixture.candidates.is_empty() {
        errors.push("candidates must not be empty".to_string());
    }

    let uncertainty_ids = unique_ids(
        fixture.uncertainties.iter().map(|item| item.id.as_str()),
        "uncertainty",
        &mut errors,
    );
    let state_ids = unique_ids(
        fixture.states.iter().map(|item| item.id.as_str()),
        "state",
        &mut errors,
    );
    let constraint_ids = unique_ids(
        fixture.required_constraints.iter().map(String::as_str),
        "constraint",
        &mut errors,
    );
    let objective_ids = unique_ids(
        fixture.objectives.iter().map(|item| item.id.as_str()),
        "objective",
        &mut errors,
    );
    let candidate_ids = unique_ids(
        fixture.candidates.iter().map(|item| item.id.as_str()),
        "candidate",
        &mut errors,
    );

    for uncertainty in &fixture.uncertainties {
        if !nonempty(&uncertainty.unit) {
            errors.push(format!("uncertainty {}: unit must not be empty", uncertainty.id));
        }
        if !nonempty(&uncertainty.evidence_ref) {
            errors.push(format!(
                "uncertainty {}: evidence_ref must not be empty",
                uncertainty.id
            ));
        }
        match &uncertainty.support {
            Support::FiniteValues { values } => {
                if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
                    errors.push(format!(
                        "uncertainty {}: finite support must contain finite values",
                        uncertainty.id
                    ));
                }
                let mut seen_values = BTreeSet::new();
                for value in values {
                    if !seen_values.insert(canonical_float_key(*value)) {
                        errors.push(format!(
                            "uncertainty {}: duplicate finite support value {value}",
                            uncertainty.id
                        ));
                    }
                }
            }
            Support::BoundedInterval { lower, upper } => {
                if !lower.is_finite() || !upper.is_finite() || lower >= upper {
                    errors.push(format!(
                        "uncertainty {}: invalid bounded interval",
                        uncertainty.id
                    ));
                }
            }
            Support::UnboundedUnknown => {}
        }
    }

    for objective in &fixture.objectives {
        if !nonempty(&objective.unit) {
            errors.push(format!("objective {}: unit must not be empty", objective.id));
        }
    }

    validate_cartesian_state_coverage(fixture, &mut errors);

    for state in &fixture.states {
        if !same_keys(&state.values, &uncertainty_ids) {
            errors.push(format!(
                "state {}: values must name exactly the registered uncertainties",
                state.id
            ));
        }
        for uncertainty in &fixture.uncertainties {
            let Some(value) = state.values.get(&uncertainty.id) else {
                continue;
            };
            if !value.is_finite() {
                errors.push(format!(
                    "state {}: uncertainty {} is non-finite",
                    state.id, uncertainty.id
                ));
            }
            if let Support::FiniteValues { values } = &uncertainty.support {
                if !values.iter().any(|allowed| allowed == value) {
                    errors.push(format!(
                        "state {}: value {value} is outside finite support for {}",
                        state.id, uncertainty.id
                    ));
                }
            }
        }
    }

    for candidate in &fixture.candidates {
        let observed_state_ids = unique_ids(
            candidate.state_results.iter().map(|item| item.state_id.as_str()),
            &format!("candidate {} state result", candidate.id),
            &mut errors,
        );
        if observed_state_ids.iter().any(|id| !state_ids.contains(id)) {
            errors.push(format!(
                "candidate {}: state result references an undeclared state",
                candidate.id
            ));
        }
        for result in &candidate.state_results {
            if !subset_keys(&result.constraints, &constraint_ids) {
                errors.push(format!(
                    "candidate {} state {}: undeclared constraint result",
                    candidate.id, result.state_id
                ));
            }
            if !subset_keys(&result.objectives, &objective_ids) {
                errors.push(format!(
                    "candidate {} state {}: undeclared objective",
                    candidate.id, result.state_id
                ));
            }
            if result.objectives.values().any(|value| !value.is_finite()) {
                errors.push(format!(
                    "candidate {} state {}: objective must be finite",
                    candidate.id, result.state_id
                ));
            }
        }
    }

    if candidate_ids.is_empty() {
        errors.push("no usable candidate IDs".to_string());
    }
    errors
}

fn global_review_gates(fixture: &RobustFixture) -> Vec<String> {
    let mut gates = Vec::new();
    if !fixture.exhaustive_joint_state_set {
        gates.push("joint_state_coverage_not_exhaustive".to_string());
    }
    for uncertainty in &fixture.uncertainties {
        if uncertainty.evidence_state == EvidenceState::Unknown {
            gates.push(format!("uncertainty_evidence_unknown:{}", uncertainty.id));
        }
        match &uncertainty.support {
            Support::FiniteValues { .. } => {}
            Support::BoundedInterval { .. } => gates.push(format!(
                "interval_not_soundly_evaluated_by_r0:{}",
                uncertainty.id
            )),
            Support::UnboundedUnknown => {
                gates.push(format!("unbounded_unknown:{}", uncertainty.id))
            }
        }
    }
    gates.sort();
    gates.dedup();
    gates
}

fn state_evaluations(
    fixture: &RobustFixture,
    candidate: &Candidate,
) -> Vec<StateEvaluation> {
    let mut states = fixture.states.iter().collect::<Vec<_>>();
    states.sort_by(|left, right| left.id.cmp(&right.id));

    states
        .into_iter()
        .map(|state| {
            let result = candidate
                .state_results
                .iter()
                .find(|item| item.state_id == state.id);
            let Some(result) = result else {
                return StateEvaluation {
                    state_id: state.id.clone(),
                    evidence_complete: false,
                    feasible: None,
                    violated_constraints: Vec::new(),
                };
            };

            let violated_constraints = result
                .constraints
                .iter()
                .filter_map(|(id, passed)| (!passed).then_some(id.clone()))
                .collect::<Vec<_>>();
            let constraints_complete =
                result.constraints.len() == fixture.required_constraints.len()
                    && fixture
                        .required_constraints
                        .iter()
                        .all(|id| result.constraints.contains_key(id));
            let objectives_complete = result.objectives.len() == fixture.objectives.len()
                && fixture
                    .objectives
                    .iter()
                    .all(|item| result.objectives.contains_key(&item.id));
            let feasible = constraints_complete
                .then(|| result.constraints.values().all(|passed| *passed));
            StateEvaluation {
                state_id: state.id.clone(),
                evidence_complete: constraints_complete && objectives_complete,
                feasible,
                violated_constraints,
            }
        })
        .collect()
}

fn candidate_objective_ranges(
    fixture: &RobustFixture,
    candidate: &Candidate,
) -> BTreeMap<String, ObjectiveRange> {
    let mut state_ids = fixture
        .states
        .iter()
        .map(|state| state.id.as_str())
        .collect::<Vec<_>>();
    state_ids.sort_unstable();

    fixture
        .objectives
        .iter()
        .map(|objective| {
            let mut observations = Vec::new();
            for state_id in &state_ids {
                if let Some(value) = candidate
                    .state_results
                    .iter()
                    .find(|result| result.state_id == *state_id)
                    .and_then(|result| result.objectives.get(&objective.id))
                {
                    observations.push(*value);
                }
            }
            let minimum = observations.iter().copied().reduce(f64::min);
            let maximum = observations.iter().copied().reduce(f64::max);
            (
                objective.id.clone(),
                ObjectiveRange {
                    minimum,
                    maximum,
                    observed_states: observations.len(),
                    expected_states: fixture.states.len(),
                },
            )
        })
        .collect()
}

fn candidate_is_complete(state_evaluations: &[StateEvaluation]) -> bool {
    state_evaluations.len() > 0
        && state_evaluations
            .iter()
            .all(|state| state.evidence_complete && state.feasible.is_some())
}

fn disposition(
    state_evaluations: &[StateEvaluation],
    global_gates: &[String],
) -> CandidateDisposition {
    if !global_gates.is_empty() || !candidate_is_complete(state_evaluations) {
        return CandidateDisposition::UnknownReviewRequired;
    }
    let feasible_count = state_evaluations
        .iter()
        .filter(|state| state.feasible == Some(true))
        .count();
    if feasible_count == state_evaluations.len() {
        CandidateDisposition::RobustFeasibleForDeclaredSet
    } else if feasible_count == 0 {
        CandidateDisposition::RobustInfeasibleForDeclaredSet
    } else {
        CandidateDisposition::ConditionallyFeasible
    }
}

fn candidate_dominates(
    fixture: &RobustFixture,
    left: &Candidate,
    right: &Candidate,
) -> bool {
    let mut states = fixture.states.iter().map(|state| state.id.as_str()).collect::<Vec<_>>();
    states.sort_unstable();
    let mut strictly_better = false;

    for state_id in states {
        let Some(left_state) = left.state_results.iter().find(|item| item.state_id == state_id)
        else {
            return false;
        };
        let Some(right_state) = right.state_results.iter().find(|item| item.state_id == state_id)
        else {
            return false;
        };

        for objective in &fixture.objectives {
            let (Some(left_value), Some(right_value)) = (
                left_state.objectives.get(&objective.id),
                right_state.objectives.get(&objective.id),
            ) else {
                return false;
            };
            let no_worse = match objective.direction {
                ObjectiveDirection::Minimize => left_value <= right_value,
                ObjectiveDirection::Maximize => left_value >= right_value,
            };
            let strictly_better_here = match objective.direction {
                ObjectiveDirection::Minimize => left_value < right_value,
                ObjectiveDirection::Maximize => left_value > right_value,
            };
            if !no_worse {
                return false;
            }
            strictly_better |= strictly_better_here;
        }
    }
    strictly_better
}

fn evaluate_fixture(fixture: &RobustFixture, input_sha256: String) -> RobustReport {
    let global_gates = global_review_gates(fixture);
    let mut candidates = fixture.candidates.iter().collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.id.cmp(&right.id));

    let candidate_evaluations = candidates
        .iter()
        .map(|candidate| {
            let state_evaluations = state_evaluations(fixture, candidate);
            let mut review_reasons = global_gates.clone();
            for state in &state_evaluations {
                if !state.evidence_complete {
                    review_reasons.push(format!(
                        "candidate_state_evidence_incomplete:{}:{}",
                        candidate.id, state.state_id
                    ));
                }
            }
            review_reasons.sort();
            review_reasons.dedup();
            CandidateEvaluation {
                candidate_id: candidate.id.clone(),
                disposition: disposition(&state_evaluations, &global_gates),
                review_reasons,
                state_evaluations,
                objective_ranges: candidate_objective_ranges(fixture, candidate),
            }
        })
        .collect::<Vec<_>>();

    let robust_candidate_ids = candidate_evaluations
        .iter()
        .filter(|item| item.disposition == CandidateDisposition::RobustFeasibleForDeclaredSet)
        .map(|item| item.candidate_id.as_str())
        .collect::<BTreeSet<_>>();

    let robust_candidates = candidates
        .iter()
        .copied()
        .filter(|candidate| robust_candidate_ids.contains(candidate.id.as_str()))
        .collect::<Vec<_>>();
    let mut frontier = Vec::new();
    for candidate in &robust_candidates {
        let dominated = robust_candidates.iter().any(|other| {
            other.id != candidate.id && candidate_dominates(fixture, other, candidate)
        });
        if !dominated {
            frontier.push(candidate.id.clone());
        }
    }
    frontier.sort();

    RobustReport {
        schema_version: REPORT_SCHEMA.to_string(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256,
        claim_ceiling: CLAIM_CEILING.to_string(),
        global_review_gates: global_gates,
        candidates: candidate_evaluations,
        robust_pareto_frontier_candidate_ids: frontier,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use sha2::{Digest, Sha256};

    #[derive(Serialize)]
    struct GeneratedCaseEvidence {
        fixture_id: String,
        input_sha256: String,
        candidate_state_evaluations: usize,
        oracle_dispositions: BTreeMap<String, CandidateDisposition>,
        evaluated_dispositions: BTreeMap<String, CandidateDisposition>,
        oracle_frontier: Vec<String>,
        evaluated_frontier: Vec<String>,
    }

    #[derive(Serialize)]
    struct GeneratedCorpusEvidence {
        schema_version: String,
        base_fixture_sha256: String,
        generator: String,
        initial_seed: String,
        generated_fixture_count: usize,
        total_candidate_state_evaluations: usize,
        cases: Vec<GeneratedCaseEvidence>,
    }

    fn fixture_bytes() -> &'static [u8] {
        include_bytes!("../fixtures/robust-v0.1.json")
    }

    fn parse_fixture() -> RobustFixture {
        serde_json::from_slice(fixture_bytes()).expect("fixture parses")
    }

    fn disposition_for<'a>(
        report: &'a RobustReport,
        candidate_id: &str,
    ) -> &'a CandidateEvaluation {
        report
            .candidates
            .iter()
            .find(|item| item.candidate_id == candidate_id)
            .expect("candidate result exists")
    }

    // Test-only oracle: recompute each state's constraint conjunction directly,
    // without calling the production disposition/state-evaluation functions.
    fn exhaustive_oracle(fixture: &RobustFixture) -> BTreeMap<String, CandidateDisposition> {
        let unsupported = !fixture.exhaustive_joint_state_set
            || fixture.uncertainties.iter().any(|uncertainty| {
                uncertainty.evidence_state != EvidenceState::SyntheticFixture
                    || !matches!(&uncertainty.support, Support::FiniteValues { .. })
            });
        let mut expected = BTreeMap::new();
        for candidate in &fixture.candidates {
            let mut observed_state_count = 0usize;
            let mut pass_state_count = 0usize;
            let mut all_results_complete = true;
            for state in &fixture.states {
                let Some(result) = candidate
                    .state_results
                    .iter()
                    .find(|item| item.state_id == state.id)
                else {
                    all_results_complete = false;
                    continue;
                };
                observed_state_count += 1;
                if fixture
                    .required_constraints
                    .iter()
                    .any(|id| !result.constraints.contains_key(id))
                    || fixture
                        .objectives
                        .iter()
                        .any(|item| !result.objectives.contains_key(&item.id))
                {
                    all_results_complete = false;
                    continue;
                }
                if fixture.required_constraints.iter().all(|id| result.constraints[id]) {
                    pass_state_count += 1;
                }
            }
            let result = if unsupported
                || !all_results_complete
                || observed_state_count != fixture.states.len()
            {
                CandidateDisposition::UnknownReviewRequired
            } else if pass_state_count == fixture.states.len() {
                CandidateDisposition::RobustFeasibleForDeclaredSet
            } else if pass_state_count == 0 {
                CandidateDisposition::RobustInfeasibleForDeclaredSet
            } else {
                CandidateDisposition::ConditionallyFeasible
            };
            expected.insert(candidate.id.clone(), result);
        }
        expected
    }

    // Separate test-only frontier oracle. It uses direct pairwise comparisons
    // rather than calling the production candidate_dominates helper.
    fn exhaustive_frontier_oracle(
        fixture: &RobustFixture,
        dispositions: &BTreeMap<String, CandidateDisposition>,
    ) -> Vec<String> {
        let eligible = fixture
            .candidates
            .iter()
            .filter(|candidate| {
                dispositions.get(&candidate.id)
                    == Some(&CandidateDisposition::RobustFeasibleForDeclaredSet)
            })
            .collect::<Vec<_>>();
        let mut frontier = Vec::new();

        for candidate in &eligible {
            let mut dominated = false;
            for other in &eligible {
                if candidate.id == other.id {
                    continue;
                }
                let mut no_worse_everywhere = true;
                let mut strictly_better_somewhere = false;

                for state in &fixture.states {
                    let left = other
                        .state_results
                        .iter()
                        .find(|result| result.state_id == state.id);
                    let right = candidate
                        .state_results
                        .iter()
                        .find(|result| result.state_id == state.id);
                    let (Some(left), Some(right)) = (left, right) else {
                        no_worse_everywhere = false;
                        break;
                    };

                    for objective in &fixture.objectives {
                        let (Some(a), Some(b)) = (
                            left.objectives.get(&objective.id),
                            right.objectives.get(&objective.id),
                        ) else {
                            no_worse_everywhere = false;
                            break;
                        };
                        let no_worse = match objective.direction {
                            ObjectiveDirection::Minimize => a <= b,
                            ObjectiveDirection::Maximize => a >= b,
                        };
                        if !no_worse {
                            no_worse_everywhere = false;
                            break;
                        }
                        strictly_better_somewhere |= match objective.direction {
                            ObjectiveDirection::Minimize => a < b,
                            ObjectiveDirection::Maximize => a > b,
                        };
                    }
                    if !no_worse_everywhere {
                        break;
                    }
                }

                if no_worse_everywhere && strictly_better_somewhere {
                    dominated = true;
                    break;
                }
            }
            if !dominated {
                frontier.push(candidate.id.clone());
            }
        }
        frontier.sort();
        frontier
    }

    fn next_deterministic_value(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *seed
    }

    #[test]
    fn generated_finite_worlds_match_independent_oracle_for_over_ten_thousand_states() {
        let mut seed = 0xC1_71_1F_A5_2026_u64;
        let mut state_evaluations = 0usize;
        let mut case_evidence = Vec::new();
        const GENERATED_FIXTURES: usize = 625;

        for case_index in 0..GENERATED_FIXTURES {
            let mut fixture = parse_fixture();
            fixture.fixture_id = format!("generated-world-{case_index}");

            for candidate in &mut fixture.candidates {
                for result in &mut candidate.state_results {
                    for passed in result.constraints.values_mut() {
                        *passed = next_deterministic_value(&mut seed) & 1 == 1;
                    }
                    for value in result.objectives.values_mut() {
                        let perturbation =
                            (next_deterministic_value(&mut seed) % 1_000) as f64 / 100.0;
                        *value += perturbation;
                    }
                }
            }

            state_evaluations += fixture.candidates.len() * fixture.states.len();
            let expected = exhaustive_oracle(&fixture);
            let expected_frontier = exhaustive_frontier_oracle(&fixture, &expected);
            let bytes = serde_json::to_vec(&fixture).expect("generated fixture serializes");
            let report = evaluate_json(&bytes).expect("generated finite fixture evaluates");

            for candidate in &report.candidates {
                assert_eq!(
                    expected[&candidate.candidate_id],
                    candidate.disposition,
                    "disposition mismatch for generated fixture {case_index}"
                );
            }
            assert_eq!(
                expected_frontier,
                report.robust_pareto_frontier_candidate_ids,
                "Pareto mismatch for generated fixture {case_index}"
            );
            let evaluated_dispositions = report
                .candidates
                .iter()
                .map(|candidate| (candidate.candidate_id.clone(), candidate.disposition))
                .collect::<BTreeMap<_, _>>();
            case_evidence.push(GeneratedCaseEvidence {
                fixture_id: fixture.fixture_id.clone(),
                input_sha256: format!("{:x}", Sha256::digest(&bytes)),
                candidate_state_evaluations: fixture.candidates.len() * fixture.states.len(),
                oracle_dispositions: expected,
                evaluated_dispositions,
                oracle_frontier: expected_frontier,
                evaluated_frontier: report.robust_pareto_frontier_candidate_ids,
            });
        }

        assert_eq!(state_evaluations, 12_500);
        if let Ok(path) = std::env::var("CIV_INFRA_CORPUS_OUT") {
            let evidence = GeneratedCorpusEvidence {
                schema_version: "luminous.civ_infra.robust_generated_corpus.v0.1".to_string(),
                base_fixture_sha256: format!("{:x}", Sha256::digest(fixture_bytes())),
                generator: "lcg64-6364136223846793005-1442695040888963407".to_string(),
                initial_seed: "0xC1711FA52026".to_string(),
                generated_fixture_count: case_evidence.len(),
                total_candidate_state_evaluations: state_evaluations,
                cases: case_evidence,
            };
            let json =
                serde_json::to_vec_pretty(&evidence).expect("corpus evidence serializes");
            std::fs::write(path, json).expect("generated corpus evidence writes");
        }
    }

    #[test]
    fn frozen_fixture_matches_authored_outcomes_and_exhaustive_oracle() {
        let report = evaluate_json(fixture_bytes()).expect("fixture evaluates");
        assert_eq!(report.claim_ceiling, "declared_finite_synthetic_states_only");
        assert_eq!(
            disposition_for(&report, "A-shortest").disposition,
            CandidateDisposition::ConditionallyFeasible
        );
        assert_eq!(
            disposition_for(&report, "B-resilient").disposition,
            CandidateDisposition::RobustFeasibleForDeclaredSet
        );
        assert_eq!(
            disposition_for(&report, "C-fragile-clearance").disposition,
            CandidateDisposition::ConditionallyFeasible
        );
        assert_eq!(
            disposition_for(&report, "D-high-margin").disposition,
            CandidateDisposition::RobustFeasibleForDeclaredSet
        );
        assert_eq!(
            disposition_for(&report, "E-overcapacity").disposition,
            CandidateDisposition::RobustInfeasibleForDeclaredSet
        );
        assert_eq!(
            report.robust_pareto_frontier_candidate_ids,
            vec!["B-resilient".to_string(), "D-high-margin".to_string()]
        );

        let fixture = parse_fixture();
        let expected = exhaustive_oracle(&fixture);
        for candidate in &report.candidates {
            assert_eq!(expected[&candidate.candidate_id], candidate.disposition);
        }

        let range =
            &disposition_for(&report, "B-resilient").objective_ranges["hydraulic_margin_lps"];
        assert_eq!(range.minimum, Some(8.0));
        assert_eq!(range.maximum, Some(20.0));
        assert_eq!(range.observed_states, 4);
        assert_eq!(range.expected_states, 4);
    }

    #[test]
    fn exhaustive_flag_cannot_hide_an_omitted_cartesian_state() {
        let mut fixture = parse_fixture();
        fixture.states.retain(|state| state.id != "soft-storm");
        for candidate in &mut fixture.candidates {
            candidate
                .state_results
                .retain(|result| result.state_id != "soft-storm");
        }
        // All supplied records remain internally complete. Only recomputing
        // the joint support reveals that one of the four possible worlds is absent.
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err(
            "an exhaustive boolean must not certify an incomplete Cartesian state set",
        );
        assert!(errors.iter().any(|error| {
            error.contains("Cartesian support requires 4 unique joint states")
        }));
    }

    #[test]
    fn cartesian_state_space_cap_fails_closed_without_exploding() {
        let mut fixture = parse_fixture();
        fixture.uncertainties[0].support = Support::FiniteValues {
            values: (0..50_001).map(f64::from).collect(),
        };
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err(
            "a Cartesian state space above the configured cap must not be sampled or certified",
        );
        assert!(errors.iter().any(|error| error.contains("exceeding R0 cap 100000")));
    }

    #[test]
    fn duplicate_joint_tuple_cannot_count_as_full_state_coverage() {
        let mut fixture = parse_fixture();
        fixture.states[3].values = fixture.states[2].values.clone();
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let errors =
            evaluate_json(&bytes).expect_err(
                "duplicate joint tuple leaves one support tuple absent",
            );
        assert!(errors.iter().any(|error| error.contains("state coverage")));
    }

    #[test]
    fn bounded_interval_is_not_misrepresented_as_robustly_evaluated() {
        let mut fixture = parse_fixture();
        fixture.uncertainties[0].support = Support::BoundedInterval { lower: 0.0, upper: 1.0 };
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let report = evaluate_json(&bytes).expect("unsupported support remains reportable");
        assert!(!report.global_review_gates.is_empty());
        assert!(report.candidates.iter().all(|candidate| {
            candidate.disposition == CandidateDisposition::UnknownReviewRequired
        }));
        assert!(report.robust_pareto_frontier_candidate_ids.is_empty());
    }

    #[test]
    fn unbounded_unknown_blocks_every_robust_feasibility_claim() {
        let mut fixture = parse_fixture();
        fixture.uncertainties[0].support = Support::UnboundedUnknown;
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let report = evaluate_json(&bytes).expect("unknown support remains reportable");
        assert!(report
            .global_review_gates
            .iter()
            .any(|gate| gate.starts_with("unbounded_unknown:")));
        assert!(report.candidates.iter().all(|candidate| {
            candidate.disposition == CandidateDisposition::UnknownReviewRequired
        }));
    }

    #[test]
    fn incomplete_joint_state_coverage_fails_closed() {
        let mut fixture = parse_fixture();
        fixture.exhaustive_joint_state_set = false;
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let report = evaluate_json(&bytes).expect("coverage limitation remains reportable");
        assert!(report
            .global_review_gates
            .contains(&"joint_state_coverage_not_exhaustive".to_string()));
        assert!(report.candidates.iter().all(|candidate| {
            candidate.disposition == CandidateDisposition::UnknownReviewRequired
        }));
    }

    #[test]
    fn missing_candidate_state_result_is_unknown_not_a_pass() {
        let mut fixture = parse_fixture();
        let candidate = fixture.candidates.iter_mut()
            .find(|candidate| candidate.id == "B-resilient")
            .expect("candidate exists");
        candidate.state_results.retain(|result| result.state_id != "soft-storm");
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let report = evaluate_json(&bytes).expect("partial evidence remains reportable");
        let result = disposition_for(&report, "B-resilient");
        assert_eq!(result.disposition, CandidateDisposition::UnknownReviewRequired);
        assert!(result.review_reasons.iter().any(|reason| reason.ends_with(":soft-storm")));
        assert_eq!(result.objective_ranges["route_length_m"].observed_states, 3);
        assert_eq!(result.objective_ranges["route_length_m"].expected_states, 4);
    }

    #[test]
    fn input_permutation_preserves_semantics_but_changes_exact_digest() {
        let original = evaluate_json(fixture_bytes()).expect("original evaluates");
        let mut fixture = parse_fixture();
        fixture.uncertainties.reverse();
        fixture.states.reverse();
        fixture.candidates.reverse();
        for candidate in &mut fixture.candidates {
            candidate.state_results.reverse();
        }
        let reordered_bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let reordered = evaluate_json(&reordered_bytes).expect("reordered fixture evaluates");
        assert_eq!(original.candidates, reordered.candidates);
        assert_eq!(
            original.robust_pareto_frontier_candidate_ids,
            reordered.robust_pareto_frontier_candidate_ids
        );
        assert_ne!(original.input_sha256, reordered.input_sha256);
    }

    #[test]
    fn exact_input_replays_to_identical_report_bytes() {
        let first = evaluate_json(fixture_bytes()).expect("first run evaluates");
        let second = evaluate_json(fixture_bytes()).expect("second run evaluates");
        assert_eq!(
            serde_json::to_vec(&first).expect("first report serializes"),
            serde_json::to_vec(&second).expect("second report serializes")
        );
    }

    #[test]
    fn undeclared_probability_fields_are_rejected() {
        let mut fixture: serde_json::Value =
            serde_json::from_slice(fixture_bytes()).expect("fixture parses as JSON");
        fixture["uncertainties"][0]["probability"] = serde_json::json!(0.5);
        let bytes = serde_json::to_vec(&fixture).expect("mutated fixture serializes");
        assert!(evaluate_json(&bytes).is_err());
    }

    #[test]
    fn out_of_support_world_values_are_rejected() {
        let mut fixture = parse_fixture();
        fixture.states[0].values.insert("ground".to_string(), 2.0);
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err("out-of-support state must fail");
        assert!(errors.iter().any(|error| error.contains("outside finite support")));
    }

    #[test]
    fn duplicated_state_ids_are_rejected() {
        let mut fixture = parse_fixture();
        fixture.states[1].id = fixture.states[0].id.clone();
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err("duplicate state ID must fail");
        assert!(errors.iter().any(|error| error.contains("duplicate ID")));
    }

    #[test]
    fn missing_objective_value_cannot_preserve_robust_feasibility() {
        let mut fixture = parse_fixture();
        let candidate = fixture.candidates.iter_mut()
            .find(|candidate| candidate.id == "D-high-margin")
            .expect("candidate exists");
        candidate.state_results[0].objectives.remove("earthworks_m3");
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let report = evaluate_json(&bytes).expect("missing result stays reportable");
        assert_eq!(
            disposition_for(&report, "D-high-margin").disposition,
            CandidateDisposition::UnknownReviewRequired
        );
        assert!(!report
            .robust_pareto_frontier_candidate_ids
            .contains(&"D-high-margin".to_string()));
    }

    #[test]
    fn forged_unknown_objective_key_is_rejected() {
        let mut fixture = parse_fixture();
        fixture.candidates[0].state_results[0]
            .objectives.insert("magic_score".to_string(), 999.0);
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err("undeclared objective must fail");
        assert!(errors.iter().any(|error| error.contains("undeclared objective")));
    }
}
