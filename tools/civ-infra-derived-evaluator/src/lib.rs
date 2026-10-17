// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const FIXTURE_SCHEMA: &str = "luminous.civ_infra.derived_fixture.v0.1";
pub const REPORT_SCHEMA: &str = "luminous.civ_infra.derived_report.v0.1";
pub const RULESET_VERSION: &str = "0.1";
pub const EVALUATOR_VERSION: &str = "0.1.0";
pub const MAX_EXHAUSTIVE_STATES: usize = 100_000;
const CLAIM_CEILING: &str = "synthetic_rules_and_declared_finite_states_only";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Meter,
    CubicMeters,
    Percent,
    LitersPerSecond,
    Dimensionless,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Quantity {
    pub value: f64,
    pub unit: Unit,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SyntheticRuleset {
    pub id: String,
    pub version: String,
    pub evidence_ref: String,
    pub max_grade_percent: Quantity,
    pub max_earthworks_m3: Quantity,
    pub min_vertical_clearance_m: Quantity,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldState {
    pub id: String,
    pub evidence_ref: String,
    pub storm_multiplier: Quantity,
    pub utility_encroachment_m: Quantity,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveCandidate {
    pub id: String,
    pub evidence_ref: String,
    pub route_length_m: Quantity,
    pub grade_percent: Quantity,
    pub earthworks_m3: Quantity,
    pub drainage_capacity_lps: Quantity,
    pub base_drainage_demand_lps: Quantity,
    pub vertical_clearance_m: Quantity,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DerivedFixture {
    pub schema_version: String,
    pub fixture_id: String,
    pub ruleset: SyntheticRuleset,
    pub storm_multiplier_support: Vec<Quantity>,
    pub utility_encroachment_support_m: Vec<Quantity>,
    pub states: Vec<WorldState>,
    pub candidates: Vec<PrimitiveCandidate>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Comparator {
    LessThanOrEqual,
    GreaterThanOrEqual,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Pass,
    Violation,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ConstraintFinding {
    pub rule_id: String,
    pub status: FindingStatus,
    pub observed: Quantity,
    pub comparator: Comparator,
    pub threshold: Quantity,
    pub input_refs: Vec<String>,
    pub equation: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DerivedObjective {
    pub quantity: Quantity,
    pub input_refs: Vec<String>,
    pub equation: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct StateResult {
    pub state_id: String,
    pub required_flow_lps: Quantity,
    pub effective_clearance_m: Quantity,
    pub findings: Vec<ConstraintFinding>,
    pub objectives: BTreeMap<String, DerivedObjective>,
    pub feasible: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    RobustFeasibleForDeclaredSet,
    RobustInfeasibleForDeclaredSet,
    ConditionallyFeasible,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CandidateResult {
    pub candidate_id: String,
    pub disposition: Disposition,
    pub state_results: Vec<StateResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DerivedReport {
    pub schema_version: String,
    pub evaluator_version: String,
    pub fixture_id: String,
    pub input_sha256: String,
    pub ruleset_sha256: String,
    pub claim_ceiling: String,
    pub candidates: Vec<CandidateResult>,
    pub robust_pareto_frontier_candidate_ids: Vec<String>,
}

#[derive(Clone, Copy)]
enum ObjectiveDirection {
    Minimize,
    Maximize,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn float_key(value: f64) -> u64 {
    if value == 0.0 {
        0.0f64.to_bits()
    } else {
        value.to_bits()
    }
}

fn quantity(value: f64, unit: Unit) -> Quantity {
    Quantity { value, unit }
}

fn valid_quantity(
    value: &Quantity,
    expected_unit: Unit,
    field: &str,
    errors: &mut Vec<String>,
) {
    if value.unit != expected_unit {
        errors.push(format!(
            "{field}: expected unit {expected_unit:?}, received {:?}",
            value.unit
        ));
    }
    if !value.value.is_finite() {
        errors.push(format!("{field}: value must be finite"));
    }
}

fn validate_nonnegative(value: &Quantity, field: &str, errors: &mut Vec<String>) {
    if value.value.is_finite() && value.value < 0.0 {
        errors.push(format!("{field}: value must be non-negative"));
    }
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

fn validate_quantity_support(
    values: &[Quantity],
    expected_unit: Unit,
    label: &str,
    strictly_positive: bool,
    errors: &mut Vec<String>,
) {
    if values.is_empty() {
        errors.push(format!("{label}: support must not be empty"));
    }
    let mut seen = BTreeSet::new();
    for value in values {
        valid_quantity(value, expected_unit, label, errors);
        if value.value.is_finite() {
            if strictly_positive && value.value <= 0.0 {
                errors.push(format!("{label}: values must be positive"));
            } else if !strictly_positive && value.value < 0.0 {
                errors.push(format!("{label}: values must be non-negative"));
            }
            if !seen.insert(float_key(value.value)) {
                errors.push(format!("{label}: duplicate support value"));
            }
        }
    }
}

fn validate_fixture(fixture: &DerivedFixture) -> Vec<String> {
    let mut errors = Vec::new();
    if fixture.schema_version != FIXTURE_SCHEMA {
        errors.push(format!(
            "unsupported schema version {}",
            fixture.schema_version
        ));
    }
    if !nonempty(&fixture.fixture_id) {
        errors.push("fixture_id must not be empty".to_string());
    }
    if !nonempty(&fixture.ruleset.id) {
        errors.push("ruleset id must not be empty".to_string());
    }
    if !nonempty(&fixture.ruleset.evidence_ref) {
        errors.push("ruleset evidence_ref must not be empty".to_string());
    }
    if fixture.ruleset.version != RULESET_VERSION {
        errors.push(format!(
            "unsupported ruleset version {}",
            fixture.ruleset.version
        ));
    }

    valid_quantity(
        &fixture.ruleset.max_grade_percent,
        Unit::Percent,
        "ruleset.max_grade_percent",
        &mut errors,
    );
    valid_quantity(
        &fixture.ruleset.max_earthworks_m3,
        Unit::CubicMeters,
        "ruleset.max_earthworks_m3",
        &mut errors,
    );
    valid_quantity(
        &fixture.ruleset.min_vertical_clearance_m,
        Unit::Meter,
        "ruleset.min_vertical_clearance_m",
        &mut errors,
    );
    validate_nonnegative(
        &fixture.ruleset.max_grade_percent,
        "ruleset.max_grade_percent",
        &mut errors,
    );
    validate_nonnegative(
        &fixture.ruleset.max_earthworks_m3,
        "ruleset.max_earthworks_m3",
        &mut errors,
    );
    validate_nonnegative(
        &fixture.ruleset.min_vertical_clearance_m,
        "ruleset.min_vertical_clearance_m",
        &mut errors,
    );

    validate_quantity_support(
        &fixture.storm_multiplier_support,
        Unit::Dimensionless,
        "storm_multiplier_support",
        true,
        &mut errors,
    );
    validate_quantity_support(
        &fixture.utility_encroachment_support_m,
        Unit::Meter,
        "utility_encroachment_support_m",
        false,
        &mut errors,
    );

    let state_ids = unique_ids(
        fixture.states.iter().map(|state| state.id.as_str()),
        "state",
        &mut errors,
    );
    let _candidate_ids = unique_ids(
        fixture.candidates.iter().map(|candidate| candidate.id.as_str()),
        "candidate",
        &mut errors,
    );
    if fixture.states.is_empty() {
        errors.push("states must not be empty".to_string());
    }
    if fixture.candidates.is_empty() {
        errors.push("candidates must not be empty".to_string());
    }

    let mut expected_states = BTreeSet::new();
    let mut product_count = fixture.storm_multiplier_support.len();
    let Some(next_count) = product_count
        .checked_mul(fixture.utility_encroachment_support_m.len())
    else {
        errors.push("state product count overflow".to_string());
        return errors;
    };
    product_count = next_count;
    if product_count > MAX_EXHAUSTIVE_STATES {
        errors.push(format!(
            "state product has {product_count} tuples, exceeding cap {MAX_EXHAUSTIVE_STATES}"
        ));
        return errors;
    }
    for storm in &fixture.storm_multiplier_support {
        for encroachment in &fixture.utility_encroachment_support_m {
            expected_states.insert((
                float_key(storm.value),
                float_key(encroachment.value),
            ));
        }
    }

    let mut observed_states = BTreeSet::new();
    for state in &fixture.states {
        if !nonempty(&state.evidence_ref) {
            errors.push(format!(
                "state {}: evidence_ref must not be empty",
                state.id
            ));
        }
        valid_quantity(
            &state.storm_multiplier,
            Unit::Dimensionless,
            "state.storm_multiplier",
            &mut errors,
        );
        valid_quantity(
            &state.utility_encroachment_m,
            Unit::Meter,
            "state.utility_encroachment_m",
            &mut errors,
        );
        if !fixture
            .storm_multiplier_support
            .iter()
            .any(|item| item.value == state.storm_multiplier.value)
        {
            errors.push(format!(
                "state {}: storm multiplier lies outside support",
                state.id
            ));
        }
        if !fixture
            .utility_encroachment_support_m
            .iter()
            .any(|item| item.value == state.utility_encroachment_m.value)
        {
            errors.push(format!(
                "state {}: utility encroachment lies outside support",
                state.id
            ));
        }
        observed_states.insert((
            float_key(state.storm_multiplier.value),
            float_key(state.utility_encroachment_m.value),
        ));
    }
    if observed_states != expected_states || observed_states.len() != fixture.states.len() {
        errors.push(format!(
            "state coverage mismatch: expected {} combinations, got {} states \
             and {} unique combinations",
            expected_states.len(),
            fixture.states.len(),
            observed_states.len()
        ));
    }

    for candidate in &fixture.candidates {
        if !nonempty(&candidate.evidence_ref) {
            errors.push(format!(
                "candidate {}: evidence_ref must not be empty",
                candidate.id
            ));
        }
        valid_quantity(
            &candidate.route_length_m,
            Unit::Meter,
            "candidate.route_length_m",
            &mut errors,
        );
        valid_quantity(
            &candidate.grade_percent,
            Unit::Percent,
            "candidate.grade_percent",
            &mut errors,
        );
        valid_quantity(
            &candidate.earthworks_m3,
            Unit::CubicMeters,
            "candidate.earthworks_m3",
            &mut errors,
        );
        valid_quantity(
            &candidate.drainage_capacity_lps,
            Unit::LitersPerSecond,
            "candidate.drainage_capacity_lps",
            &mut errors,
        );
        valid_quantity(
            &candidate.base_drainage_demand_lps,
            Unit::LitersPerSecond,
            "candidate.base_drainage_demand_lps",
            &mut errors,
        );
        valid_quantity(
            &candidate.vertical_clearance_m,
            Unit::Meter,
            "candidate.vertical_clearance_m",
            &mut errors,
        );
        validate_nonnegative(
            &candidate.route_length_m,
            "candidate.route_length_m",
            &mut errors,
        );
        validate_nonnegative(
            &candidate.grade_percent,
            "candidate.grade_percent",
            &mut errors,
        );
        validate_nonnegative(
            &candidate.earthworks_m3,
            "candidate.earthworks_m3",
            &mut errors,
        );
        validate_nonnegative(
            &candidate.drainage_capacity_lps,
            "candidate.drainage_capacity_lps",
            &mut errors,
        );
        validate_nonnegative(
            &candidate.base_drainage_demand_lps,
            "candidate.base_drainage_demand_lps",
            &mut errors,
        );
        validate_nonnegative(
            &candidate.vertical_clearance_m,
            "candidate.vertical_clearance_m",
            &mut errors,
        );
    }

    let _ = state_ids;
    errors
}

fn compare(
    observed: &Quantity,
    threshold: &Quantity,
    comparator: Comparator,
) -> bool {
    match comparator {
        Comparator::LessThanOrEqual => observed.value <= threshold.value,
        Comparator::GreaterThanOrEqual => observed.value >= threshold.value,
    }
}

fn finding(
    rule_id: &str,
    observed: Quantity,
    comparator: Comparator,
    threshold: Quantity,
    input_refs: &[&str],
    equation: String,
) -> ConstraintFinding {
    ConstraintFinding {
        rule_id: rule_id.to_string(),
        status: if compare(&observed, &threshold, comparator) {
            FindingStatus::Pass
        } else {
            FindingStatus::Violation
        },
        observed,
        comparator,
        threshold,
        input_refs: input_refs.iter().map(|item| (*item).to_string()).collect(),
        equation,
    }
}

fn derive_state(
    fixture: &DerivedFixture,
    candidate: &PrimitiveCandidate,
    state: &WorldState,
) -> Result<StateResult, String> {
    let required_flow = candidate.base_drainage_demand_lps.value
        * state.storm_multiplier.value;
    let effective_clearance =
        candidate.vertical_clearance_m.value - state.utility_encroachment_m.value;
    let hydraulic_margin =
        candidate.drainage_capacity_lps.value - required_flow;
    let clearance_margin =
        effective_clearance - fixture.ruleset.min_vertical_clearance_m.value;
    if !required_flow.is_finite()
        || !effective_clearance.is_finite()
        || !hydraulic_margin.is_finite()
        || !clearance_margin.is_finite()
    {
        return Err(format!(
            "candidate {} state {}: derived arithmetic is non-finite",
            candidate.id, state.id
        ));
    }

    let mut findings = vec![
        finding(
            "grade_limit",
            candidate.grade_percent.clone(),
            Comparator::LessThanOrEqual,
            fixture.ruleset.max_grade_percent.clone(),
            &[
                &candidate.evidence_ref,
                "candidate.grade_percent",
                &fixture.ruleset.evidence_ref,
                "ruleset.max_grade_percent",
            ],
            "grade_percent <= max_grade_percent".to_string(),
        ),
        finding(
            "earthworks_budget",
            candidate.earthworks_m3.clone(),
            Comparator::LessThanOrEqual,
            fixture.ruleset.max_earthworks_m3.clone(),
            &[
                &candidate.evidence_ref,
                "candidate.earthworks_m3",
                &fixture.ruleset.evidence_ref,
                "ruleset.max_earthworks_m3",
            ],
            "earthworks_m3 <= max_earthworks_m3".to_string(),
        ),
        finding(
            "hydraulic_capacity",
            candidate.drainage_capacity_lps.clone(),
            Comparator::GreaterThanOrEqual,
            quantity(required_flow, Unit::LitersPerSecond),
            &[
                &candidate.evidence_ref,
                "candidate.drainage_capacity_lps",
                "candidate.base_drainage_demand_lps",
                &state.evidence_ref,
                "state.storm_multiplier",
            ],
            "capacity_lps >= base_demand_lps * storm_multiplier".to_string(),
        ),
        finding(
            "utility_clearance",
            quantity(effective_clearance, Unit::Meter),
            Comparator::GreaterThanOrEqual,
            fixture.ruleset.min_vertical_clearance_m.clone(),
            &[
                &candidate.evidence_ref,
                "candidate.vertical_clearance_m",
                &state.evidence_ref,
                "state.utility_encroachment_m",
                &fixture.ruleset.evidence_ref,
                "ruleset.min_vertical_clearance_m",
            ],
            "clearance_m - utility_encroachment_m >= minimum_clearance_m".to_string(),
        ),
    ];
    findings.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    let feasible = findings
        .iter()
        .all(|item| item.status == FindingStatus::Pass);

    let mut objectives = BTreeMap::new();
    objectives.insert(
        "route_length_m".to_string(),
        DerivedObjective {
            quantity: candidate.route_length_m.clone(),
            input_refs: vec![
                candidate.evidence_ref.clone(),
                "candidate.route_length_m".to_string(),
            ],
            equation: "route_length_m (primitive input)".to_string(),
        },
    );
    objectives.insert(
        "earthworks_m3".to_string(),
        DerivedObjective {
            quantity: candidate.earthworks_m3.clone(),
            input_refs: vec![
                candidate.evidence_ref.clone(),
                "candidate.earthworks_m3".to_string(),
            ],
            equation: "earthworks_m3 (primitive input)".to_string(),
        },
    );
    objectives.insert(
        "hydraulic_margin_lps".to_string(),
        DerivedObjective {
            quantity: quantity(hydraulic_margin, Unit::LitersPerSecond),
            input_refs: vec![
                candidate.evidence_ref.clone(),
                "candidate.drainage_capacity_lps".to_string(),
                "candidate.base_drainage_demand_lps".to_string(),
                state.evidence_ref.clone(),
                "state.storm_multiplier".to_string(),
            ],
            equation: "capacity_lps - base_demand_lps * storm_multiplier".to_string(),
        },
    );
    objectives.insert(
        "clearance_margin_m".to_string(),
        DerivedObjective {
            quantity: quantity(clearance_margin, Unit::Meter),
            input_refs: vec![
                candidate.evidence_ref.clone(),
                "candidate.vertical_clearance_m".to_string(),
                state.evidence_ref.clone(),
                "state.utility_encroachment_m".to_string(),
                fixture.ruleset.evidence_ref.clone(),
                "ruleset.min_vertical_clearance_m".to_string(),
            ],
            equation: "clearance_m - encroachment_m - minimum_clearance_m".to_string(),
        },
    );

    Ok(StateResult {
        state_id: state.id.clone(),
        required_flow_lps: quantity(required_flow, Unit::LitersPerSecond),
        effective_clearance_m: quantity(effective_clearance, Unit::Meter),
        findings,
        objectives,
        feasible,
    })
}

fn state_disposition(results: &[StateResult]) -> Disposition {
    let feasible_count = results.iter().filter(|result| result.feasible).count();
    if feasible_count == results.len() {
        Disposition::RobustFeasibleForDeclaredSet
    } else if feasible_count == 0 {
        Disposition::RobustInfeasibleForDeclaredSet
    } else {
        Disposition::ConditionallyFeasible
    }
}

fn objectives_dominate(
    left: &StateResult,
    right: &StateResult,
    directions: &[(&str, ObjectiveDirection)],
) -> (bool, bool) {
    let mut no_worse = true;
    let mut strictly_better = false;
    for (id, direction) in directions {
        let (Some(a), Some(b)) = (left.objectives.get(*id), right.objectives.get(*id)) else {
            return (false, false);
        };
        let a = a.quantity.value;
        let b = b.quantity.value;
        let no_worse_this = match direction {
            ObjectiveDirection::Minimize => a <= b,
            ObjectiveDirection::Maximize => a >= b,
        };
        let strictly_better_this = match direction {
            ObjectiveDirection::Minimize => a < b,
            ObjectiveDirection::Maximize => a > b,
        };
        if !no_worse_this {
            no_worse = false;
            break;
        }
        strictly_better |= strictly_better_this;
    }
    (no_worse, strictly_better)
}

fn robust_pareto_frontier(results: &[CandidateResult]) -> Vec<String> {
    let directions = [
        ("route_length_m", ObjectiveDirection::Minimize),
        ("earthworks_m3", ObjectiveDirection::Minimize),
        ("hydraulic_margin_lps", ObjectiveDirection::Maximize),
        ("clearance_margin_m", ObjectiveDirection::Maximize),
    ];
    let eligible = results
        .iter()
        .filter(|result| {
            result.disposition == Disposition::RobustFeasibleForDeclaredSet
        })
        .collect::<Vec<_>>();
    let mut frontier = Vec::new();
    for candidate in &eligible {
        let dominated = eligible.iter().any(|other| {
            if other.candidate_id == candidate.candidate_id {
                return false;
            }
            let mut strict = false;
            for state in &candidate.state_results {
                let Some(other_state) = other
                    .state_results
                    .iter()
                    .find(|item| item.state_id == state.state_id)
                else {
                    return false;
                };
                let (no_worse, better) =
                    objectives_dominate(other_state, state, &directions);
                if !no_worse {
                    return false;
                }
                strict |= better;
            }
            strict
        });
        if !dominated {
            frontier.push(candidate.candidate_id.clone());
        }
    }
    frontier.sort();
    frontier
}

pub fn evaluate_json(bytes: &[u8]) -> Result<DerivedReport, Vec<String>> {
    let fixture: DerivedFixture = serde_json::from_slice(bytes)
        .map_err(|error| vec![format!("fixture JSON rejected: {error}")])?;
    let errors = validate_fixture(&fixture);
    if !errors.is_empty() {
        return Err(errors);
    }
    let ruleset_bytes = serde_json::to_vec(&fixture.ruleset)
        .map_err(|error| vec![format!("ruleset serialization failed: {error}")])?;
    let ruleset_sha256 = sha256_hex(&ruleset_bytes);
    let mut states = fixture.states.iter().collect::<Vec<_>>();
    states.sort_by(|left, right| left.id.cmp(&right.id));
    let mut candidates = fixture.candidates.iter().collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    let mut results = Vec::new();
    let mut errors = Vec::new();

    for candidate in candidates {
        let mut state_results = Vec::new();
        for state in &states {
            match derive_state(&fixture, candidate, state) {
                Ok(result) => state_results.push(result),
                Err(error) => errors.push(error),
            }
        }
        results.push(CandidateResult {
            candidate_id: candidate.id.clone(),
            disposition: state_disposition(&state_results),
            state_results,
        });
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let pareto = robust_pareto_frontier(&results);
    Ok(DerivedReport {
        schema_version: REPORT_SCHEMA.to_string(),
        evaluator_version: EVALUATOR_VERSION.to_string(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256: sha256_hex(bytes),
        ruleset_sha256,
        claim_ceiling: CLAIM_CEILING.to_string(),
        candidates: results,
        robust_pareto_frontier_candidate_ids: pareto,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct GeneratedCaseEvidence {
        fixture_id: String,
        input_sha256: String,
        input_json: String,
        candidate_state_evaluations: usize,
        oracle_dispositions: BTreeMap<String, Disposition>,
        evaluated_dispositions: BTreeMap<String, Disposition>,
        oracle_frontier: Vec<String>,
        evaluated_frontier: Vec<String>,
    }

    #[derive(Serialize)]
    struct GeneratedCorpusEvidence {
        schema_version: String,
        evaluator_version: String,
        base_fixture_sha256: String,
        generator: String,
        mutator: String,
        initial_seed: String,
        generated_fixture_count: usize,
        total_candidate_state_evaluations: usize,
        cases: Vec<GeneratedCaseEvidence>,
    }

    fn fixture_bytes() -> &'static [u8] {
        include_bytes!("../fixtures/derived-v0.1.json")
    }

    fn fixture() -> DerivedFixture {
        serde_json::from_slice(fixture_bytes()).expect("fixture parses")
    }

    fn result<'a>(report: &'a DerivedReport, id: &str) -> &'a CandidateResult {
        report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == id)
            .expect("candidate result exists")
    }

    fn reference_state(
        fixture: &DerivedFixture,
        candidate: &PrimitiveCandidate,
        state: &WorldState,
    ) -> (bool, BTreeMap<String, f64>, BTreeMap<String, bool>) {
        // Independent test oracle: compute each result directly from primitive
        // inputs; do not call derive_state, finding, or compare.
        let flow = candidate.base_drainage_demand_lps.value
            * state.storm_multiplier.value;
        let clearance = candidate.vertical_clearance_m.value
            - state.utility_encroachment_m.value;
        let hydraulic_margin = candidate.drainage_capacity_lps.value - flow;
        let clearance_margin = clearance
            - fixture.ruleset.min_vertical_clearance_m.value;
        let constraints = BTreeMap::from([
            (
                "grade_limit".to_string(),
                candidate.grade_percent.value
                    <= fixture.ruleset.max_grade_percent.value,
            ),
            (
                "earthworks_budget".to_string(),
                candidate.earthworks_m3.value
                    <= fixture.ruleset.max_earthworks_m3.value,
            ),
            (
                "hydraulic_capacity".to_string(),
                candidate.drainage_capacity_lps.value >= flow,
            ),
            (
                "utility_clearance".to_string(),
                clearance >= fixture.ruleset.min_vertical_clearance_m.value,
            ),
        ]);
        let feasible = constraints.values().all(|passed| *passed);
        let objectives = BTreeMap::from([
            ("route_length_m".to_string(), candidate.route_length_m.value),
            ("earthworks_m3".to_string(), candidate.earthworks_m3.value),
            ("hydraulic_margin_lps".to_string(), hydraulic_margin),
            ("clearance_margin_m".to_string(), clearance_margin),
        ]);
        (feasible, objectives, constraints)
    }

    fn reference_disposition(
        fixture: &DerivedFixture,
        candidate: &PrimitiveCandidate,
    ) -> Disposition {
        let passes = fixture
            .states
            .iter()
            .map(|state| reference_state(fixture, candidate, state).0)
            .collect::<Vec<_>>();
        if passes.iter().all(|passed| *passed) {
            Disposition::RobustFeasibleForDeclaredSet
        } else if passes.iter().all(|passed| !*passed) {
            Disposition::RobustInfeasibleForDeclaredSet
        } else {
            Disposition::ConditionallyFeasible
        }
    }

    fn reference_frontier(fixture: &DerivedFixture) -> Vec<String> {
        let eligible = fixture
            .candidates
            .iter()
            .filter(|candidate| {
                reference_disposition(fixture, candidate)
                    == Disposition::RobustFeasibleForDeclaredSet
            })
            .collect::<Vec<_>>();
        let directions = [
            ("route_length_m", false),
            ("earthworks_m3", false),
            ("hydraulic_margin_lps", true),
            ("clearance_margin_m", true),
        ];
        let mut frontier = Vec::new();
        for candidate in &eligible {
            let mut dominated = false;
            for other in &eligible {
                if other.id == candidate.id {
                    continue;
                }
                let mut strict = false;
                let mut no_worse_all = true;
                for state in &fixture.states {
                    let left = reference_state(fixture, other, state).1;
                    let right = reference_state(fixture, candidate, state).1;
                    for (objective, maximize) in directions {
                        let a = left[objective];
                        let b = right[objective];
                        let no_worse = if maximize { a >= b } else { a <= b };
                        let better = if maximize { a > b } else { a < b };
                        if !no_worse {
                            no_worse_all = false;
                            break;
                        }
                        strict |= better;
                    }
                    if !no_worse_all {
                        break;
                    }
                }
                if no_worse_all && strict {
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

    fn next_value(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *seed
    }

    #[test]
    fn primitive_inputs_derive_expected_findings_without_assertion_fields() {
        let report = evaluate_json(fixture_bytes()).expect("fixture evaluates");
        assert_eq!(report.claim_ceiling, CLAIM_CEILING);
        assert_eq!(
            result(&report, "A-shortest").disposition,
            Disposition::RobustInfeasibleForDeclaredSet
        );
        assert_eq!(
            result(&report, "B-balanced").disposition,
            Disposition::RobustFeasibleForDeclaredSet
        );
        assert_eq!(
            result(&report, "C-fragile-clearance").disposition,
            Disposition::ConditionallyFeasible
        );
        assert_eq!(
            result(&report, "D-resilient").disposition,
            Disposition::RobustFeasibleForDeclaredSet
        );
        assert_eq!(
            result(&report, "E-under-capacity").disposition,
            Disposition::RobustInfeasibleForDeclaredSet
        );
        assert_eq!(
            report.robust_pareto_frontier_candidate_ids,
            vec!["B-balanced".to_string(), "D-resilient".to_string()]
        );
        assert_eq!(
            report.input_sha256,
            sha256_hex(fixture_bytes())
        );
        let state = result(&report, "B-balanced")
            .state_results
            .iter()
            .find(|state| state.state_id == "storm-high__encroach-high")
            .expect("high stress state exists");
        assert_eq!(state.required_flow_lps.value, 105.0);
        assert_eq!(state.effective_clearance_m.value, 0.2);
        assert!(state.feasible);
        assert!(state.findings.iter().all(|finding| !finding.input_refs.is_empty()));
    }

    #[test]
    fn missing_ruleset_state_or_candidate_evidence_reference_fails_closed() {
        let mut input = fixture();
        input.ruleset.evidence_ref.clear();
        let bytes = serde_json::to_vec(&input).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err("missing ruleset evidence ref must fail");
        assert!(errors.iter().any(|error| error.contains("ruleset evidence_ref")));

        let mut input = fixture();
        input.states[0].evidence_ref.clear();
        let bytes = serde_json::to_vec(&input).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err("missing state evidence ref must fail");
        assert!(errors.iter().any(|error| error.contains("state evidence_ref")));

        let mut input = fixture();
        input.candidates[0].evidence_ref.clear();
        let bytes = serde_json::to_vec(&input).expect("fixture serializes");
        let errors =
            evaluate_json(&bytes).expect_err("missing candidate evidence ref must fail");
        assert!(errors.iter().any(|error| error.contains("candidate")));
    }

    #[test]
    fn derivation_traces_retain_candidate_state_and_ruleset_evidence_refs() {
        let input = fixture();
        let report = evaluate_json(fixture_bytes()).expect("fixture evaluates");
        let candidate = input
            .candidates
            .iter()
            .find(|candidate| candidate.id == "B-balanced")
            .expect("candidate exists");
        let state = input
            .states
            .iter()
            .find(|state| state.id == "storm-high__encroach-high")
            .expect("state exists");
        let output_state = result(&report, "B-balanced")
            .state_results
            .iter()
            .find(|item| item.state_id == state.id)
            .expect("output state exists");

        let grade = output_state.findings
            .iter()
            .find(|finding| finding.rule_id == "grade_limit")
            .expect("grade finding exists");
        assert!(grade.input_refs.contains(&candidate.evidence_ref));
        assert!(grade.input_refs.contains(&input.ruleset.evidence_ref));

        let drainage = output_state.findings
            .iter()
            .find(|finding| finding.rule_id == "hydraulic_capacity")
            .expect("drainage finding exists");
        assert!(drainage.input_refs.contains(&candidate.evidence_ref));
        assert!(drainage.input_refs.contains(&state.evidence_ref));

        let clearance = output_state.findings
            .iter()
            .find(|finding| finding.rule_id == "utility_clearance")
            .expect("clearance finding exists");
        assert!(clearance.input_refs.contains(&candidate.evidence_ref));
        assert!(clearance.input_refs.contains(&state.evidence_ref));
        assert!(clearance.input_refs.contains(&input.ruleset.evidence_ref));

        let margin = &output_state.objectives["hydraulic_margin_lps"];
        assert!(margin.input_refs.contains(&candidate.evidence_ref));
        assert!(margin.input_refs.contains(&state.evidence_ref));
    }

    #[test]
    fn independent_oracle_matches_all_fixture_state_findings_and_objectives() {
        let input = fixture();
        let report = evaluate_json(fixture_bytes()).expect("fixture evaluates");
        for candidate in &input.candidates {
            assert_eq!(
                reference_disposition(&input, candidate),
                result(&report, &candidate.id).disposition
            );
            for state in &input.states {
                let (feasible, objectives, constraints) =
                    reference_state(&input, candidate, state);
                let actual = result(&report, &candidate.id)
                    .state_results
                    .iter()
                    .find(|item| item.state_id == state.id)
                    .expect("derived state result exists");
                assert_eq!(actual.feasible, feasible);
                for finding in &actual.findings {
                    assert_eq!(
                        finding.status == FindingStatus::Pass,
                        constraints[&finding.rule_id]
                    );
                }
                for (id, expected) in objectives {
                    assert_eq!(actual.objectives[&id].quantity.value, expected);
                }
            }
        }
        assert_eq!(report.robust_pareto_frontier_candidate_ids, reference_frontier(&input));
    }

    #[test]
    fn equality_at_rule_threshold_passes_but_epsilon_exceedance_fails() {
        let mut input = fixture();
        input.ruleset.max_grade_percent.value = 5.0;
        let bytes = serde_json::to_vec(&input).expect("fixture serializes");
        let report = evaluate_json(&bytes).expect("exact threshold is valid");
        let balanced = result(&report, "B-balanced");
        assert!(balanced.state_results.iter().all(|state| {
            state.findings.iter().any(|finding| {
                finding.rule_id == "grade_limit"
                    && finding.status == FindingStatus::Pass
            })
        }));

        input
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == "B-balanced")
            .expect("candidate exists")
            .grade_percent
            .value = 5.000_001;
        let bytes = serde_json::to_vec(&input).expect("fixture serializes");
        let report = evaluate_json(&bytes).expect("finite exceedance is valid input");
        assert!(result(&report, "B-balanced").state_results.iter().all(|state| {
            state.findings.iter().any(|finding| {
                finding.rule_id == "grade_limit"
                    && finding.status == FindingStatus::Violation
            })
        }));
    }

    #[test]
    fn wrong_units_are_rejected() {
        let mut input = fixture();
        input.candidates[0].grade_percent.unit = Unit::Meter;
        let bytes = serde_json::to_vec(&input).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err("incompatible unit must fail");
        assert!(errors.iter().any(|error| error.contains("expected unit Percent")));
    }

    #[test]
    fn unsupported_ruleset_version_is_rejected() {
        let mut input = fixture();
        input.ruleset.version = "0.2".to_string();
        let bytes = serde_json::to_vec(&input).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err("unknown ruleset must fail");
        assert!(errors.iter().any(|error| error.contains("unsupported ruleset version")));
    }

    #[test]
    fn unknown_assertion_fields_cannot_override_derived_results() {
        let mut input: serde_json::Value =
            serde_json::from_slice(fixture_bytes()).expect("fixture parses as JSON");
        input["candidates"][0]["state_results"] = serde_json::json!([]);
        input["candidates"][0]["constraints"] = serde_json::json!({
            "grade_limit": true,
            "earthworks_budget": true,
            "hydraulic_capacity": true,
            "utility_clearance": true
        });
        let bytes = serde_json::to_vec(&input).expect("mutated JSON serializes");
        assert!(evaluate_json(&bytes).is_err());
    }

    #[test]
    fn incomplete_or_duplicate_joint_state_coverage_is_rejected() {
        let mut input = fixture();
        input.states.pop();
        let bytes = serde_json::to_vec(&input).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err("omitted state must fail");
        assert!(errors.iter().any(|error| error.contains("state coverage mismatch")));

        let mut input = fixture();
        input.states[3].storm_multiplier = input.states[2].storm_multiplier.clone();
        input.states[3].utility_encroachment_m =
            input.states[2].utility_encroachment_m.clone();
        let bytes = serde_json::to_vec(&input).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err("duplicate combination must fail");
        assert!(errors.iter().any(|error| error.contains("state coverage mismatch")));
    }

    #[test]
    fn oversized_cartesian_support_fails_closed_before_enumeration() {
        let mut input = fixture();
        input.storm_multiplier_support = (1..=100_001)
            .map(|value| quantity(value as f64, Unit::Dimensionless))
            .collect();
        let errors = validate_fixture(&input);
        assert!(errors.iter().any(|error| error.contains("exceeding cap 100000")));
    }

    #[test]
    fn derived_numeric_overflow_fails_closed() {
        let mut input = fixture();
        let candidate = input
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == "B-balanced")
            .expect("candidate exists");
        candidate.base_drainage_demand_lps.value = 1.0e308;
        let state = input
            .states
            .iter_mut()
            .find(|state| state.id == "storm-high__encroach-high")
            .expect("state exists");
        state.storm_multiplier.value = 1.5;
        input.storm_multiplier_support = vec![
            quantity(1.0, Unit::Dimensionless),
            quantity(1.5, Unit::Dimensionless),
        ];
        let bytes = serde_json::to_vec(&input).expect("fixture serializes");
        let errors = evaluate_json(&bytes).expect_err("overflow must not yield a report");
        assert!(errors.iter().any(|error| error.contains("non-finite")));
    }

    #[test]
    fn unknown_json_fields_are_rejected() {
        let mut input: serde_json::Value =
            serde_json::from_slice(fixture_bytes()).expect("fixture parses as JSON");
        input["ruleset"]["probability"] = serde_json::json!(0.95);
        let bytes = serde_json::to_vec(&input).expect("mutated JSON serializes");
        assert!(evaluate_json(&bytes).is_err());
    }

    #[test]
    fn identical_inputs_replay_identical_report_bytes() {
        let first = evaluate_json(fixture_bytes()).expect("first evaluation succeeds");
        let second = evaluate_json(fixture_bytes()).expect("second evaluation succeeds");
        assert_eq!(
            serde_json::to_vec(&first).expect("first report serializes"),
            serde_json::to_vec(&second).expect("second report serializes")
        );
    }

    #[test]
    fn permutation_preserves_semantics_but_changes_exact_input_digest() {
        let original = evaluate_json(fixture_bytes()).expect("original evaluates");
        let mut input = fixture();
        input.states.reverse();
        input.candidates.reverse();
        input.storm_multiplier_support.reverse();
        input.utility_encroachment_support_m.reverse();
        let bytes = serde_json::to_vec(&input).expect("permuted fixture serializes");
        let report = evaluate_json(&bytes).expect("permuted fixture evaluates");
        let normalize = |report: &DerivedReport| {
            report
                .candidates
                .iter()
                .map(|candidate| {
                    (
                        candidate.candidate_id.clone(),
                        candidate.disposition,
                        candidate
                            .state_results
                            .iter()
                            .map(|state| (state.state_id.clone(), state.feasible))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(normalize(&original), normalize(&report));
        assert_eq!(
            original.robust_pareto_frontier_candidate_ids,
            report.robust_pareto_frontier_candidate_ids
        );
        assert_ne!(original.input_sha256, report.input_sha256);
        assert_eq!(original.ruleset_sha256, report.ruleset_sha256);
    }

    #[test]
    fn generated_primitive_designs_match_reference_and_emit_replay_evidence() {
        let mut seed = 0xD3_21_1A_2026_u64;
        let mut evaluations = 0usize;
        let mut case_records = Vec::new();
        for case_index in 0..625 {
            let mut input = fixture();
            input.fixture_id = format!("generated-derived-world-{case_index}");
            for candidate in &mut input.candidates {
                candidate.grade_percent.value +=
                    (next_value(&mut seed) % 300) as f64 / 100.0;
                candidate.drainage_capacity_lps.value +=
                    (next_value(&mut seed) % 150) as f64 / 10.0;
                candidate.vertical_clearance_m.value +=
                    (next_value(&mut seed) % 100) as f64 / 1_000.0;
                candidate.base_drainage_demand_lps.value +=
                    (next_value(&mut seed) % 200) as f64 / 10.0;
            }

            let bytes = serde_json::to_vec(&input).expect("generated fixture serializes");
            let report = evaluate_json(&bytes).expect("generated fixture evaluates");
            let mut expected_dispositions = BTreeMap::new();
            let mut actual_dispositions = BTreeMap::new();

            for candidate in &input.candidates {
                let expected_disposition = reference_disposition(&input, candidate);
                let actual = result(&report, &candidate.id);
                assert_eq!(
                    expected_disposition,
                    actual.disposition,
                    "disposition mismatch in generated fixture {case_index}"
                );
                expected_dispositions.insert(candidate.id.clone(), expected_disposition);
                actual_dispositions.insert(candidate.id.clone(), actual.disposition);

                for state in &input.states {
                    let (expected_feasible, expected_objectives, expected_constraints) =
                        reference_state(&input, candidate, state);
                    let actual_state = actual
                        .state_results
                        .iter()
                        .find(|item| item.state_id == state.id)
                        .expect("derived result exists");
                    assert_eq!(actual_state.feasible, expected_feasible);
                    for (id, expected) in expected_constraints {
                        let finding = actual_state
                            .findings
                            .iter()
                            .find(|finding| finding.rule_id == id)
                            .expect("finding exists");
                        assert_eq!(
                            finding.status == FindingStatus::Pass,
                            expected,
                            "constraint mismatch in generated fixture {case_index}"
                        );
                    }
                    for (id, expected) in expected_objectives {
                        assert_eq!(
                            actual_state.objectives[&id].quantity.value,
                            expected,
                            "objective mismatch in generated fixture {case_index}"
                        );
                    }
                    evaluations += 1;
                }
            }

            let expected_frontier = reference_frontier(&input);
            assert_eq!(
                expected_frontier,
                report.robust_pareto_frontier_candidate_ids,
                "Pareto mismatch in generated fixture {case_index}"
            );
            case_records.push(GeneratedCaseEvidence {
                fixture_id: input.fixture_id.clone(),
                input_sha256: sha256_hex(&bytes),
                input_json: String::from_utf8(bytes.clone())
                    .expect("generated fixture is valid UTF-8 JSON"),
                candidate_state_evaluations: input.candidates.len() * input.states.len(),
                oracle_dispositions: expected_dispositions,
                evaluated_dispositions: actual_dispositions,
                oracle_frontier: expected_frontier,
                evaluated_frontier: report.robust_pareto_frontier_candidate_ids,
            });
        }
        assert_eq!(evaluations, 12_500);

        if let Ok(path) = std::env::var("CIV_INFRA_DERIVED_CORPUS_OUT") {
            let evidence = GeneratedCorpusEvidence {
                schema_version: "luminous.civ_infra.derived_corpus.v0.1".to_string(),
                evaluator_version: EVALUATOR_VERSION.to_string(),
                base_fixture_sha256: sha256_hex(fixture_bytes()),
                generator: "lcg64-6364136223846793005-1442695040888963407".to_string(),
                mutator: concat!(
                    "per-candidate grade+=lcg%300/100; capacity+=lcg%150/10; ",
                    "clearance+=lcg%100/1000; base-demand+=lcg%200/10"
                )
                .to_string(),
                initial_seed: "0xD3211A2026".to_string(),
                generated_fixture_count: case_records.len(),
                total_candidate_state_evaluations: evaluations,
                cases: case_records,
            };
            let json =
                serde_json::to_vec_pretty(&evidence).expect("corpus evidence serializes");
            std::fs::write(path, json).expect("generated corpus artifact writes");
        }
    }


}
