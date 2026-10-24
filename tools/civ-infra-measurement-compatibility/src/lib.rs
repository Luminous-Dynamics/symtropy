#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: &str = "civ-infra-measurement-compatibility/v0.1";
pub const EVALUATOR_VERSION: &str = "0.1.0";
pub const FIXED_SCALE: i128 = 1_000_000;
pub const MAX_STATES: usize = 100_000;
pub const MAX_OUTCOMES: usize = 100_000;
pub const MAX_COMPATIBILITY_EDGES: usize = 100_000;
pub const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityInput {
    pub schema_version: String,
    pub world_id: String,
    pub measurement_model: MeasurementModel,
    pub states: Vec<StateEnvelope>,
    pub outcomes: Vec<OutcomeBin>,
    #[serde(default)]
    pub observed_value: Option<Quantity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MeasurementModel {
    pub model_id: String,
    pub model_version: String,
    pub model_artifact_ref: String,
    #[serde(default)]
    pub model_artifact_sha256: Option<String>,
    pub input_unit: Unit,
    pub input_domain: Interval,
    pub transform: Transform,
    pub model_discrepancy: ModelDiscrepancy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModelDiscrepancy {
    pub unit: Unit,
    pub interval: Interval,
    pub basis: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Meter,
    Millimeter,
    Dimensionless,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Transform {
    Identity,
    SquareNonNegativeDimensionless,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Interval {
    /// Endpoints are signed integers in fixed-point units: 1 unit = 1,000,000 ticks.
    pub lower_ticks: i64,
    pub upper_ticks: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Quantity {
    pub unit: Unit,
    pub value_ticks: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StateEnvelope {
    pub state_id: String,
    pub input_unit: Unit,
    pub input_interval: Interval,
    pub decision_signature_id: String,
    pub upstream_case_ref: String,
    #[serde(default)]
    pub upstream_report_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutcomeBin {
    pub outcome_id: String,
    pub unit: Unit,
    pub interval: Interval,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompatibilityReport {
    pub schema_version: String,
    pub evaluator_version: String,
    pub world_id: String,
    pub input_sha256: String,
    pub measurement_model_id: String,
    pub measurement_model_version: String,
    pub measurement_model_artifact_ref: String,
    pub measurement_model_artifact_sha256: Option<String>,
    pub transform: Transform,
    pub model_discrepancy: ModelDiscrepancy,
    pub output_dimension: String,
    pub output_tick_representation: String,
    pub claim_ceiling: String,
    pub state_count: usize,
    pub outcome_count: usize,
    pub compatibility_edge_count: usize,
    pub state_results: Vec<StateCompatibility>,
    pub outcome_results: Vec<OutcomeCompatibility>,
    pub decision_ambiguity: DecisionAmbiguity,
    pub observed_update: ObservedUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateCompatibility {
    pub state_id: String,
    pub decision_signature_id: String,
    pub upstream_case_ref: String,
    pub upstream_report_sha256: Option<String>,
    pub domain_status: DomainStatus,
    pub predicted_output_interval: Option<Interval>,
    pub possible_outcome_ids: Vec<String>,
    pub output_range_has_uncovered_regions: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DomainStatus {
    WithinModelDomain,
    PartlyOrWhollyOutsideModelDomain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeCompatibility {
    pub outcome_id: String,
    pub compatible_state_ids: Vec<String>,
    pub distinct_decision_signature_count: usize,
    pub decision_discordant_pairs_remaining: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionAmbiguity {
    pub decision_signature_classes_before_observation: usize,
    pub worst_case_remaining_states_within_declared_outcome_bins: usize,
    pub worst_case_decision_signature_classes_within_declared_outcome_bins: usize,
    pub worst_case_decision_discordant_pairs_within_declared_outcome_bins: u64,
    pub outcome_worst_case_is_conditional_on_declared_support: bool,
    pub support_failure_preserves_prior: bool,
    pub support_failure_preserved_state_count: usize,
    pub support_failure_preserved_decision_signature_classes: usize,
    pub support_failure_preserved_decision_discordant_pairs: u64,
    pub outcomes_with_no_compatible_states: usize,
    pub states_outside_model_domain: usize,
    pub states_with_uncovered_output_regions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservedUpdate {
    pub status: ObservedStatus,
    pub observed_value: Option<Quantity>,
    pub matching_outcome_ids: Vec<String>,
    pub prior_state_count: usize,
    pub compatible_state_ids: Vec<String>,
    pub decision_signature_class_count: usize,
    pub decision_discordant_pairs_remaining: u64,
    pub update_note: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservedStatus {
    NotObserved,
    SupportedByModel,
    OutOfModelSupport,
}

#[derive(Debug, Clone, Copy)]
struct CanonicalInterval {
    lower: i64,
    upper: i64,
}

#[derive(Debug, Clone)]
struct NormalizedState {
    state_id: String,
    signature_id: String,
    source_ref: String,
    source_digest: Option<String>,
    domain_status: DomainStatus,
    predicted: Option<CanonicalInterval>,
    compatible_outcomes: Vec<String>,
    uncovered: bool,
}

#[derive(Debug, Clone)]
struct NormalizedBin {
    outcome_id: String,
    interval: CanonicalInterval,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn evaluate_bytes(input_bytes: &[u8]) -> Result<CompatibilityReport, String> {
    if input_bytes.len() > MAX_INPUT_BYTES {
        return Err(format!("input byte count exceeds limit {MAX_INPUT_BYTES}"));
    }
    let input_sha256 = sha256_hex(input_bytes);
    let input: CompatibilityInput =
        serde_json::from_slice(input_bytes).map_err(|err| format!("invalid compatibility input JSON: {err}"))?;
    evaluate(&input, &input_sha256)
}

fn evaluate(input: &CompatibilityInput, input_sha256: &str) -> Result<CompatibilityReport, String> {
    if input.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema_version {:?}; expected {:?}",
            input.schema_version, SCHEMA_VERSION
        ));
    }
    validate_name(&input.world_id, "world_id")?;
    validate_digest(input_sha256, "input_sha256")?;

    let model = &input.measurement_model;
    validate_name(&model.model_id, "model_id")?;
    validate_name(&model.model_version, "model_version")?;
    validate_name(&model.model_artifact_ref, "model_artifact_ref")?;
    if let Some(digest) = &model.model_artifact_sha256 {
        validate_digest(digest, "model_artifact_sha256")?;
    }
    validate_interval(model.input_domain, "measurement_model.input_domain")?;
    if model.input_domain.lower_ticks < 0 && model.transform == Transform::SquareNonNegativeDimensionless {
        return Err("square_non_negative_dimensionless requires a non-negative model domain".to_owned());
    }
    if model.transform == Transform::SquareNonNegativeDimensionless && model.input_unit != Unit::Dimensionless {
        return Err("square_non_negative_dimensionless requires dimensionless input".to_owned());
    }
    validate_same_dimension(output_unit(model), model.model_discrepancy.unit, "model discrepancy")?;
    validate_interval(model.model_discrepancy.interval, "model_discrepancy.interval")?;
    validate_name(&model.model_discrepancy.basis, "model_discrepancy.basis")?;
    let discrepancy = canonical_interval(model.model_discrepancy.unit, model.model_discrepancy.interval)?;

    if input.states.is_empty() {
        return Err("states must not be empty".to_owned());
    }
    if input.states.len() > MAX_STATES {
        return Err(format!("state count exceeds limit {MAX_STATES}"));
    }
    if input.outcomes.is_empty() {
        return Err("outcomes must not be empty".to_owned());
    }
    if input.outcomes.len() > MAX_OUTCOMES {
        return Err(format!("outcome count exceeds limit {MAX_OUTCOMES}"));
    }
    validate_work_bound(input.states.len(), input.outcomes.len())?;

    let domain = canonical_interval(model.input_unit, model.input_domain)?;
    let mut bin_ids = BTreeSet::new();
    let mut normalized_bins = Vec::with_capacity(input.outcomes.len());
    for bin in &input.outcomes {
        validate_name(&bin.outcome_id, "outcome_id")?;
        if !bin_ids.insert(bin.outcome_id.clone()) {
            return Err(format!("duplicate outcome_id {:?}", bin.outcome_id));
        }
        validate_interval(bin.interval, "outcome interval")?;
        validate_same_dimension(output_unit(model), bin.unit, "outcome bin")?;
        normalized_bins.push(NormalizedBin {
            outcome_id: bin.outcome_id.clone(),
            interval: canonical_interval(bin.unit, bin.interval)?,
        });
    }
    normalized_bins.sort_by(|left, right| {
        left.interval
            .lower
            .cmp(&right.interval.lower)
            .then_with(|| left.interval.upper.cmp(&right.interval.upper))
            .then_with(|| left.outcome_id.cmp(&right.outcome_id))
    });

    let all_outcome_ids: Vec<String> = normalized_bins.iter().map(|bin| bin.outcome_id.clone()).collect();
    let mut state_ids = BTreeSet::new();
    let mut normalized_states = Vec::with_capacity(input.states.len());
    for state in &input.states {
        validate_name(&state.state_id, "state_id")?;
        validate_name(&state.decision_signature_id, "decision_signature_id")?;
        validate_name(&state.upstream_case_ref, "upstream_case_ref")?;
        if let Some(digest) = &state.upstream_report_sha256 {
            validate_digest(digest, "upstream_report_sha256")?;
        }
        if !state_ids.insert(state.state_id.clone()) {
            return Err(format!("duplicate state_id {:?}", state.state_id));
        }
        validate_same_dimension(model.input_unit, state.input_unit, "state input")?;
        validate_interval(state.input_interval, "state input interval")?;
        let state_interval = canonical_interval(state.input_unit, state.input_interval)?;

        let inside_domain = state_interval.lower >= domain.lower && state_interval.upper <= domain.upper;
        let domain_status = if inside_domain {
            DomainStatus::WithinModelDomain
        } else {
            DomainStatus::PartlyOrWhollyOutsideModelDomain
        };

        let (predicted, compatible_outcomes, uncovered) = if !inside_domain {
            // Out-of-domain predictions cannot safely rule out any outcome.
            (None, all_outcome_ids.clone(), false)
        } else {
            let transformed = transform_interval(state_interval, model.transform)?;
            let predicted = apply_model_discrepancy(transformed, discrepancy)?;
            let compatible: Vec<String> = normalized_bins
                .iter()
                .filter(|bin| overlaps(predicted, bin.interval))
                .map(|bin| bin.outcome_id.clone())
                .collect();
            let uncovered = !interval_fully_covered(predicted, &normalized_bins);
            (Some(predicted), compatible, uncovered)
        };

        normalized_states.push(NormalizedState {
            state_id: state.state_id.clone(),
            signature_id: state.decision_signature_id.clone(),
            source_ref: state.upstream_case_ref.clone(),
            source_digest: state.upstream_report_sha256.clone(),
            domain_status,
            predicted,
            compatible_outcomes,
            uncovered,
        });
    }

    normalized_states.sort_by(|left, right| left.state_id.cmp(&right.state_id));
    let mut state_results = Vec::with_capacity(normalized_states.len());
    let mut compatibility_edge_count = 0usize;
    for state in &normalized_states {
        compatibility_edge_count = compatibility_edge_count
            .checked_add(state.compatible_outcomes.len())
            .ok_or_else(|| "compatibility edge count overflow".to_owned())?;
        state_results.push(StateCompatibility {
            state_id: state.state_id.clone(),
            decision_signature_id: state.signature_id.clone(),
            upstream_case_ref: state.source_ref.clone(),
            upstream_report_sha256: state.source_digest.clone(),
            domain_status: state.domain_status,
            predicted_output_interval: state.predicted.map(interval_from_canonical),
            possible_outcome_ids: state.compatible_outcomes.clone(),
            output_range_has_uncovered_regions: state.uncovered,
        });
    }

    let distinct_signatures: BTreeSet<String> = normalized_states
        .iter()
        .map(|state| state.signature_id.clone())
        .collect();
    let mut outcome_results = Vec::with_capacity(normalized_bins.len());
    for bin in &normalized_bins {
        let mut compatible_states = Vec::new();
        let mut signature_counts = BTreeMap::<String, u64>::new();
        for state in &normalized_states {
            if state.compatible_outcomes.contains(&bin.outcome_id) {
                compatible_states.push(state.state_id.clone());
                *signature_counts.entry(state.signature_id.clone()).or_insert(0) += 1;
            }
        }
        outcome_results.push(OutcomeCompatibility {
            outcome_id: bin.outcome_id.clone(),
            decision_discordant_pairs_remaining: discordant_pairs(&signature_counts),
            distinct_decision_signature_count: signature_counts.len(),
            compatible_state_ids: compatible_states,
        });
    }
    outcome_results.sort_by(|left, right| left.outcome_id.cmp(&right.outcome_id));

    let worst_case_remaining_states = outcome_results
        .iter()
        .map(|outcome| outcome.compatible_state_ids.len())
        .max()
        .unwrap_or(0);
    let worst_case_decision_signature_classes = outcome_results
        .iter()
        .map(|outcome| outcome.distinct_decision_signature_count)
        .max()
        .unwrap_or(0);
    let worst_case_decision_discordant_pairs_remaining = outcome_results
        .iter()
        .map(|outcome| outcome.decision_discordant_pairs_remaining)
        .max()
        .unwrap_or(0);
    let outcomes_with_no_compatible_states = outcome_results
        .iter()
        .filter(|outcome| outcome.compatible_state_ids.is_empty())
        .count();
    let states_outside_model_domain = normalized_states
        .iter()
        .filter(|state| state.domain_status != DomainStatus::WithinModelDomain)
        .count();
    let states_with_uncovered_output_regions = normalized_states.iter().filter(|state| state.uncovered).count();

    let observed_update = build_observed_update(input.observed_value, model, &normalized_bins, &normalized_states)?;

    Ok(CompatibilityReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        world_id: input.world_id.clone(),
        input_sha256: input_sha256.to_owned(),
        measurement_model_id: model.model_id.clone(),
        measurement_model_version: model.model_version.clone(),
        measurement_model_artifact_ref: model.model_artifact_ref.clone(),
        measurement_model_artifact_sha256: model.model_artifact_sha256.clone(),
        transform: model.transform,
        model_discrepancy: model.model_discrepancy.clone(),
        output_dimension: if output_unit(model) == Unit::Dimensionless {
            "dimensionless".to_owned()
        } else {
            "length".to_owned()
        },
        output_tick_representation: if output_unit(model) == Unit::Dimensionless {
            "1e-6 of a dimensionless output unit".to_owned()
        } else {
            "nanometre ticks; interval endpoints are normalized, not displayed-unit values".to_owned()
        },
        claim_ceiling: concat!(
            "finite_typed_intervals_declared_transform_and_discrepancy_only; ",
            "model_artifacts_not_resolved; ",
            "no_probability_or_physical_validity_claim"
        )
        .to_owned(),
        state_count: normalized_states.len(),
        outcome_count: normalized_bins.len(),
        compatibility_edge_count,
        state_results,
        outcome_results,
        decision_ambiguity: DecisionAmbiguity {
            decision_signature_classes_before_observation: distinct_signatures.len(),
            worst_case_remaining_states_within_declared_outcome_bins: worst_case_remaining_states,
            worst_case_decision_signature_classes_within_declared_outcome_bins: worst_case_decision_signature_classes,
            worst_case_decision_discordant_pairs_within_declared_outcome_bins:
                worst_case_decision_discordant_pairs_remaining,
            outcome_worst_case_is_conditional_on_declared_support: true,
            support_failure_preserves_prior: true,
            support_failure_preserved_state_count: normalized_states.len(),
            support_failure_preserved_decision_signature_classes: distinct_signatures.len(),
            support_failure_preserved_decision_discordant_pairs: discordant_pairs(&signature_counts(
                &normalized_states,
                None,
            )),
            outcomes_with_no_compatible_states,
            states_outside_model_domain,
            states_with_uncovered_output_regions,
        },
        observed_update,
    })
}

fn validate_work_bound(state_count: usize, outcome_count: usize) -> Result<usize, String> {
    let potential_edges = state_count
        .checked_mul(outcome_count)
        .ok_or_else(|| "state-outcome compatibility count overflow".to_owned())?;
    if potential_edges > MAX_COMPATIBILITY_EDGES {
        return Err(format!(
            "state-outcome compatibility count {potential_edges} exceeds limit {MAX_COMPATIBILITY_EDGES}"
        ));
    }
    Ok(potential_edges)
}

fn build_observed_update(
    observation: Option<Quantity>,
    model: &MeasurementModel,
    bins: &[NormalizedBin],
    states: &[NormalizedState],
) -> Result<ObservedUpdate, String> {
    let all_states: Vec<String> = states.iter().map(|state| state.state_id.clone()).collect();
    let prior_state_count = all_states.len();
    let Some(observation) = observation else {
        return Ok(ObservedUpdate {
            status: ObservedStatus::NotObserved,
            observed_value: None,
            matching_outcome_ids: Vec::new(),
            prior_state_count,
            compatible_state_ids: all_states,
            decision_signature_class_count: distinct_signature_count(states, None),
            decision_discordant_pairs_remaining: discordant_pairs(&signature_counts(states, None)),
            update_note: "no observation supplied; prior state set is unchanged".to_owned(),
        });
    };
    validate_same_dimension(output_unit(model), observation.unit, "observed value")?;
    let observed_interval = canonical_point(observation)?;
    let matching_bins: Vec<&NormalizedBin> = bins
        .iter()
        .filter(|bin| observed_interval.lower >= bin.interval.lower && observed_interval.lower <= bin.interval.upper)
        .collect();

    if matching_bins.is_empty() {
        return Ok(ObservedUpdate {
            status: ObservedStatus::OutOfModelSupport,
            observed_value: Some(observation),
            matching_outcome_ids: Vec::new(),
            prior_state_count,
            compatible_state_ids: all_states,
            decision_signature_class_count: distinct_signature_count(states, None),
            decision_discordant_pairs_remaining: discordant_pairs(&signature_counts(states, None)),
            update_note: concat!(
                "observed value lies outside every declared outcome bin; ",
                "preserve the entire prior state set and investigate ",
                "model/domain/support mismatch"
            )
            .to_owned(),
        });
    }

    let matching_bin_ids: BTreeSet<&str> = matching_bins.iter().map(|bin| bin.outcome_id.as_str()).collect();
    let mut compatible_states = Vec::new();
    let mut compatible_signature_counts = BTreeMap::<String, u64>::new();
    for state in states {
        let compatible = if state.domain_status != DomainStatus::WithinModelDomain {
            true
        } else {
            state
                .predicted
                .map(|interval| observed_interval.lower >= interval.lower && observed_interval.lower <= interval.upper)
                .unwrap_or(false)
                && state
                    .compatible_outcomes
                    .iter()
                    .any(|id| matching_bin_ids.contains(id.as_str()))
        };
        if compatible {
            compatible_states.push(state.state_id.clone());
            *compatible_signature_counts
                .entry(state.signature_id.clone())
                .or_insert(0) += 1;
        }
    }
    compatible_states.sort();

    if compatible_states.is_empty() {
        return Ok(ObservedUpdate {
            status: ObservedStatus::OutOfModelSupport,
            observed_value: Some(observation),
            matching_outcome_ids: matching_bins.iter().map(|bin| bin.outcome_id.clone()).collect(),
            prior_state_count,
            compatible_state_ids: all_states,
            decision_signature_class_count: distinct_signature_count(states, None),
            decision_discordant_pairs_remaining: discordant_pairs(&signature_counts(states, None)),
            update_note: concat!(
                "observed value matches an outcome bin but no declared state output ",
                "envelope supports it; preserve the entire prior state set and investigate ",
                "model/support mismatch"
            )
            .to_owned(),
        });
    }

    Ok(ObservedUpdate {
        status: ObservedStatus::SupportedByModel,
        observed_value: Some(observation),
        matching_outcome_ids: matching_bins.iter().map(|bin| bin.outcome_id.clone()).collect(),
        prior_state_count,
        decision_signature_class_count: compatible_signature_counts.len(),
        decision_discordant_pairs_remaining: discordant_pairs(&compatible_signature_counts),
        compatible_state_ids: compatible_states,
        update_note: concat!(
            "only states whose declared output envelope contains the observation remain; ",
            "out-of-domain states remain possible. No probabilities are assigned"
        )
        .to_owned(),
    })
}

fn distinct_signature_count(states: &[NormalizedState], ids: Option<&BTreeSet<String>>) -> usize {
    states
        .iter()
        .filter(|state| ids.map(|set| set.contains(&state.state_id)).unwrap_or(true))
        .map(|state| state.signature_id.clone())
        .collect::<BTreeSet<_>>()
        .len()
}

fn signature_counts(states: &[NormalizedState], ids: Option<&BTreeSet<String>>) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for state in states {
        if ids.map(|set| set.contains(&state.state_id)).unwrap_or(true) {
            *counts.entry(state.signature_id.clone()).or_insert(0) += 1;
        }
    }
    counts
}

fn apply_model_discrepancy(
    predicted: CanonicalInterval,
    discrepancy: CanonicalInterval,
) -> Result<CanonicalInterval, String> {
    let lower = predicted
        .lower
        .checked_add(discrepancy.lower)
        .ok_or_else(|| "model discrepancy lower-bound addition overflow".to_owned())?;
    let upper = predicted
        .upper
        .checked_add(discrepancy.upper)
        .ok_or_else(|| "model discrepancy upper-bound addition overflow".to_owned())?;
    if lower > upper {
        return Err("model discrepancy produced an invalid output interval".to_owned());
    }
    Ok(CanonicalInterval { lower, upper })
}

fn transform_interval(input: CanonicalInterval, transform: Transform) -> Result<CanonicalInterval, String> {
    match transform {
        Transform::Identity => Ok(input),
        Transform::SquareNonNegativeDimensionless => {
            if input.lower < 0 {
                return Err("square transform received negative input despite validated model domain".to_owned());
            }
            let lower = square_floor(input.lower)?;
            let upper = square_ceil(input.upper)?;
            Ok(CanonicalInterval { lower, upper })
        }
    }
}

fn square_floor(value: i64) -> Result<i64, String> {
    let n = i128::from(value);
    let squared = n.checked_mul(n).ok_or_else(|| "square transform overflow".to_owned())?;
    let result = squared / FIXED_SCALE;
    i64::try_from(result).map_err(|_| "square transform result exceeds i64 range".to_owned())
}

fn square_ceil(value: i64) -> Result<i64, String> {
    let n = i128::from(value);
    let squared = n.checked_mul(n).ok_or_else(|| "square transform overflow".to_owned())?;
    let result = squared
        .checked_add(FIXED_SCALE - 1)
        .ok_or_else(|| "square transform rounding overflow".to_owned())?
        / FIXED_SCALE;
    i64::try_from(result).map_err(|_| "square transform result exceeds i64 range".to_owned())
}

fn interval_fully_covered(interval: CanonicalInterval, bins: &[NormalizedBin]) -> bool {
    let mut cursor = interval.lower;
    for bin in bins {
        if bin.interval.upper < cursor {
            continue;
        }
        if bin.interval.lower > cursor {
            return false;
        }
        if bin.interval.upper >= interval.upper {
            return true;
        }
        cursor = bin.interval.upper;
    }
    false
}

fn overlaps(left: CanonicalInterval, right: CanonicalInterval) -> bool {
    left.lower <= right.upper && right.lower <= left.upper
}

fn canonical_interval(unit: Unit, interval: Interval) -> Result<CanonicalInterval, String> {
    validate_interval(interval, "interval")?;
    let lower = canonical_value(unit, interval.lower_ticks, Rounding::Floor)?;
    let upper = canonical_value(unit, interval.upper_ticks, Rounding::Ceil)?;
    if lower > upper {
        return Err("unit conversion inverted interval bounds".to_owned());
    }
    Ok(CanonicalInterval { lower, upper })
}

fn canonical_point(quantity: Quantity) -> Result<CanonicalInterval, String> {
    let value = canonical_value(quantity.unit, quantity.value_ticks, Rounding::Exact)?;
    Ok(CanonicalInterval {
        lower: value,
        upper: value,
    })
}

#[derive(Debug, Clone, Copy)]
enum Rounding {
    Exact,
    Floor,
    Ceil,
}

fn canonical_value(unit: Unit, value: i64, rounding: Rounding) -> Result<i64, String> {
    // Values are signed fixed-point numbers with 1,000,000 ticks per declared unit.
    // Length intervals are mapped to nanometre ticks to make m/mm comparisons exact.
    match unit {
        Unit::Meter => i64::try_from(
            i128::from(value)
                .checked_mul(1000)
                .ok_or_else(|| "meter to canonical conversion overflow".to_owned())?,
        )
        .map_err(|_| "meter to canonical conversion exceeds i64 range".to_owned()),
        Unit::Millimeter => Ok(value),
        Unit::Dimensionless => {
            let _ = rounding;
            Ok(value)
        }
    }
}

fn interval_from_canonical(interval: CanonicalInterval) -> Interval {
    Interval {
        lower_ticks: interval.lower,
        upper_ticks: interval.upper,
    }
}

fn output_unit(model: &MeasurementModel) -> Unit {
    match model.transform {
        Transform::Identity => model.input_unit,
        Transform::SquareNonNegativeDimensionless => Unit::Dimensionless,
    }
}

fn validate_same_dimension(expected: Unit, actual: Unit, label: &str) -> Result<(), String> {
    let expected_is_length = matches!(expected, Unit::Meter | Unit::Millimeter);
    let actual_is_length = matches!(actual, Unit::Meter | Unit::Millimeter);
    if expected_is_length != actual_is_length {
        return Err(format!(
            "{label} unit {:?} has incompatible dimension; expected {:?}",
            actual, expected
        ));
    }
    Ok(())
}

fn validate_interval(interval: Interval, label: &str) -> Result<(), String> {
    if interval.lower_ticks > interval.upper_ticks {
        return Err(format!("{label} lower bound exceeds upper bound"));
    }
    Ok(())
}

fn validate_name(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.len() > 1024 {
        return Err(format!("{label} exceeds 1024 UTF-8 bytes"));
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{label} must be a lowercase 64-character SHA-256 digest"));
    }
    Ok(())
}

fn choose2(value: u64) -> u64 {
    if value < 2 { 0 } else { value * (value - 1) / 2 }
}

fn discordant_pairs(counts: &BTreeMap<String, u64>) -> u64 {
    let total: u64 = counts.values().sum();
    choose2(total) - counts.values().map(|count| choose2(*count)).sum::<u64>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn interval(lower: i64, upper: i64) -> Interval {
        Interval {
            lower_ticks: lower,
            upper_ticks: upper,
        }
    }

    fn state(id: &str, unit: Unit, lower: i64, upper: i64, signature: &str) -> StateEnvelope {
        StateEnvelope {
            state_id: id.to_owned(),
            input_unit: unit,
            input_interval: interval(lower, upper),
            decision_signature_id: signature.to_owned(),
            upstream_case_ref: format!("synthetic-case#{id}"),
            upstream_report_sha256: None,
        }
    }

    fn bin(id: &str, unit: Unit, lower: i64, upper: i64) -> OutcomeBin {
        OutcomeBin {
            outcome_id: id.to_owned(),
            unit,
            interval: interval(lower, upper),
        }
    }

    fn base_input() -> CompatibilityInput {
        CompatibilityInput {
            schema_version: SCHEMA_VERSION.to_owned(),
            world_id: "synthetic-compatibility-v0.1".to_owned(),
            measurement_model: MeasurementModel {
                model_id: "synthetic-length-probe".to_owned(),
                model_version: "1".to_owned(),
                model_artifact_ref: "synthetic-models/length-probe-v1".to_owned(),
                model_artifact_sha256: None,
                input_unit: Unit::Meter,
                input_domain: interval(0, 2_000_000),
                transform: Transform::Identity,
                model_discrepancy: ModelDiscrepancy {
                    unit: Unit::Meter,
                    interval: interval(0, 0),
                    basis: "synthetic zero-discrepancy assumption; not calibrated".to_owned(),
                },
            },
            states: vec![
                state("short", Unit::Millimeter, 500_000_000, 900_000_000, "decision-A"),
                state("medium", Unit::Meter, 800_000, 1_200_000, "decision-B"),
                state("long", Unit::Meter, 1_500_000, 1_800_000, "decision-C"),
            ],
            outcomes: vec![
                bin("low", Unit::Meter, 0, 800_000),
                bin("middle", Unit::Meter, 800_000, 1_400_000),
                bin("high", Unit::Meter, 1_400_000, 2_000_000),
            ],
            observed_value: None,
        }
    }

    fn digest(input: &CompatibilityInput) -> String {
        sha256_hex(&serde_json::to_vec(input).expect("input serializes"))
    }

    #[test]
    fn fixture_evaluates_with_overlapping_compatibility() {
        let input: CompatibilityInput =
            serde_json::from_str(include_str!("../fixtures/compatibility-v0.1.json")).expect("fixture parses");
        let report = evaluate(&input, &digest(&input)).expect("fixture evaluates");
        assert_eq!(report.state_count, 4);
        assert_eq!(report.outcome_count, 3);
        assert!(report.compatibility_edge_count >= report.state_count);
        assert!(
            report
                .state_results
                .iter()
                .any(|state| state.possible_outcome_ids.len() > 1)
        );
    }

    #[test]
    fn converts_meter_and_millimeter_intervals_to_same_canonical_range() {
        let a = canonical_interval(Unit::Meter, interval(1_000_000, 1_200_000)).expect("meters");
        let b = canonical_interval(Unit::Millimeter, interval(1_000_000_000, 1_200_000_000)).expect("millimeters");
        assert_eq!(a.lower, b.lower);
        assert_eq!(a.upper, b.upper);
    }

    #[test]
    fn model_discrepancy_envelope_widens_possible_outcomes() {
        let mut input = base_input();
        input.states = vec![state(
            "between-bins",
            Unit::Meter,
            1_100_000,
            1_100_000,
            "decision-between-bins",
        )];
        input.outcomes = vec![
            bin("left", Unit::Meter, 0, 1_050_000),
            bin("right", Unit::Meter, 1_150_000, 2_000_000),
        ];
        input.measurement_model.model_discrepancy.interval = interval(-200_000, 200_000);
        input.measurement_model.model_discrepancy.basis =
            "synthetic bounded discrepancy for test only; not calibrated".to_owned();
        let report = evaluate(&input, &digest(&input)).expect("discrepancy widens output");
        assert_eq!(report.state_results[0].possible_outcome_ids, vec!["left", "right"]);
        assert_eq!(
            report.state_results[0].predicted_output_interval,
            Some(interval(900_000_000, 1_300_000_000))
        );
        assert!(report.state_results[0].output_range_has_uncovered_regions);
        assert_eq!(report.model_discrepancy, input.measurement_model.model_discrepancy);
    }

    #[test]
    fn model_discrepancy_overflow_fails_closed() {
        assert!(
            apply_model_discrepancy(
                CanonicalInterval {
                    lower: i64::MAX - 1,
                    upper: i64::MAX
                },
                CanonicalInterval { lower: 0, upper: 1 },
            )
            .unwrap_err()
            .contains("overflow")
        );
    }

    #[test]
    fn nonlinear_square_transform_rounds_outward() {
        let result = transform_interval(
            CanonicalInterval {
                lower: 1_000_001,
                upper: 2_000_001,
            },
            Transform::SquareNonNegativeDimensionless,
        )
        .expect("square transform");
        assert_eq!(result.lower, 1_000_002);
        assert_eq!(result.upper, 4_000_005);
    }

    #[test]
    fn overlap_at_closed_bin_boundary_keeps_both_outcomes() {
        let mut input = base_input();
        input.states = vec![state("edge", Unit::Meter, 800_000, 800_000, "decision-edge")];
        input.outcomes = vec![
            bin("left", Unit::Meter, 0, 800_000),
            bin("right", Unit::Meter, 800_000, 2_000_000),
        ];
        let report = evaluate(&input, &digest(&input)).expect("edge evaluates");
        assert_eq!(report.state_results[0].possible_outcome_ids.len(), 2);
    }

    #[test]
    fn reports_support_failure_bound_separately_from_bin_conditioned_metrics() {
        let mut input = base_input();
        input.states = vec![
            state("sig-a-1", Unit::Meter, 100_000, 200_000, "sig-a"),
            state("sig-b", Unit::Meter, 800_000, 900_000, "sig-b"),
            state("sig-a-2", Unit::Meter, 300_000, 400_000, "sig-a"),
        ];
        input.outcomes = vec![
            bin("low", Unit::Meter, 0, 450_000),
            bin("high", Unit::Meter, 750_000, 1_000_000),
        ];
        let report =
            evaluate_bytes(serde_json::to_vec(&input).expect("input serializes").as_slice()).expect("input evaluates");
        let ambiguity = report.decision_ambiguity;
        assert!(ambiguity.outcome_worst_case_is_conditional_on_declared_support);
        assert!(ambiguity.support_failure_preserves_prior);
        assert_eq!(ambiguity.support_failure_preserved_state_count, 3);
        assert_eq!(ambiguity.support_failure_preserved_decision_signature_classes, 2);
        assert_eq!(ambiguity.support_failure_preserved_decision_discordant_pairs, 2);
        assert!(ambiguity.worst_case_remaining_states_within_declared_outcome_bins < 3);
    }

    #[test]
    fn out_of_model_domain_keeps_every_outcome_possible() {
        let mut input = base_input();
        input.states = vec![state("outside", Unit::Meter, 2_100_000, 2_200_000, "decision-outside")];
        let report = evaluate(&input, &digest(&input)).expect("out of domain is represented");
        assert_eq!(
            report.state_results[0].domain_status,
            DomainStatus::PartlyOrWhollyOutsideModelDomain
        );
        assert_eq!(report.state_results[0].possible_outcome_ids.len(), input.outcomes.len());
        assert_eq!(report.decision_ambiguity.states_outside_model_domain, 1);
    }

    #[test]
    fn partially_out_of_domain_state_is_not_clipped_or_narrowed() {
        let mut input = base_input();
        input.states = vec![state(
            "crossing",
            Unit::Meter,
            1_900_000,
            2_100_000,
            "decision-crossing",
        )];
        let report = evaluate(&input, &digest(&input)).expect("partly out-of-domain state is retained");
        assert_eq!(
            report.state_results[0].domain_status,
            DomainStatus::PartlyOrWhollyOutsideModelDomain
        );
        assert_eq!(report.state_results[0].possible_outcome_ids.len(), input.outcomes.len());
        assert_eq!(report.state_results[0].predicted_output_interval, None);
    }

    #[test]
    fn uncovered_output_range_is_explicit() {
        let mut input = base_input();
        input.states = vec![state("gap", Unit::Meter, 1_000_000, 1_800_000, "decision-gap")];
        input.outcomes = vec![
            bin("low", Unit::Meter, 0, 1_200_000),
            bin("high", Unit::Meter, 1_500_000, 2_000_000),
        ];
        let report = evaluate(&input, &digest(&input)).expect("gap is reported");
        assert!(report.state_results[0].output_range_has_uncovered_regions);
    }

    #[test]
    fn observation_outside_all_bins_preserves_prior_state_set() {
        let mut input = base_input();
        input.observed_value = Some(Quantity {
            unit: Unit::Meter,
            value_ticks: 3_000_000,
        });
        let report = evaluate(&input, &digest(&input)).expect("out of support is a report");
        assert_eq!(report.observed_update.status, ObservedStatus::OutOfModelSupport);
        assert_eq!(report.observed_update.compatible_state_ids.len(), input.states.len());
    }

    #[test]
    fn supported_observation_keeps_all_overlapping_states() {
        let mut input = base_input();
        input.states = vec![
            state("left", Unit::Meter, 700_000, 900_000, "signature-left"),
            state("right", Unit::Meter, 850_000, 1_100_000, "signature-right"),
            state("far", Unit::Meter, 1_500_000, 1_700_000, "signature-far"),
        ];
        input.observed_value = Some(Quantity {
            unit: Unit::Meter,
            value_ticks: 875_000,
        });
        let report = evaluate(&input, &digest(&input)).expect("supported observation");
        assert_eq!(report.observed_update.status, ObservedStatus::SupportedByModel);
        assert_eq!(report.observed_update.compatible_state_ids, vec!["left", "right"]);
    }

    #[test]
    fn observation_inside_a_bin_but_outside_all_state_predictions_preserves_prior() {
        let mut input = base_input();
        input.observed_value = Some(Quantity {
            unit: Unit::Meter,
            value_ticks: 1_900_000,
        });
        let report = evaluate(&input, &digest(&input)).expect("unsupported state prediction is represented");
        assert_eq!(report.observed_update.status, ObservedStatus::OutOfModelSupport);
        assert_eq!(report.observed_update.compatible_state_ids.len(), input.states.len());
        assert!(
            report
                .observed_update
                .update_note
                .contains("no declared state output envelope supports it")
        );
    }

    #[test]
    fn dimensionless_square_rejects_length_units() {
        let mut input = base_input();
        input.measurement_model.transform = Transform::SquareNonNegativeDimensionless;
        assert!(
            evaluate(&input, &digest(&input))
                .unwrap_err()
                .contains("requires dimensionless input")
        );
    }

    #[test]
    fn rejects_state_from_incompatible_dimension() {
        let mut input = base_input();
        input.states[0].input_unit = Unit::Dimensionless;
        assert!(
            evaluate(&input, &digest(&input))
                .unwrap_err()
                .contains("incompatible dimension")
        );
    }

    #[test]
    fn rejects_reversed_intervals_and_unknown_model_version_fields() {
        let mut input = base_input();
        input.states[0].input_interval = interval(4, 3);
        assert!(
            evaluate(&input, &digest(&input))
                .unwrap_err()
                .contains("lower bound exceeds upper bound")
        );
        let mut value = serde_json::to_value(base_input()).expect("serialize");
        value["measurement_model"]["magic_prediction"] = serde_json::json!(0.999);
        assert!(serde_json::from_value::<CompatibilityInput>(value).is_err());
    }

    #[test]
    fn rejects_bad_model_and_upstream_digests() {
        let mut input = base_input();
        input.measurement_model.model_artifact_sha256 = Some("bad".to_owned());
        assert!(
            evaluate(&input, &digest(&input))
                .unwrap_err()
                .contains("model_artifact_sha256")
        );
        let mut input = base_input();
        input.states[0].upstream_report_sha256 = Some("bad".to_owned());
        assert!(
            evaluate(&input, &digest(&input))
                .unwrap_err()
                .contains("upstream_report_sha256")
        );
    }

    #[test]
    fn rejects_empty_duplicate_state_and_outcome_ids() {
        let mut input = base_input();
        input.states[1].state_id = input.states[0].state_id.clone();
        assert!(
            evaluate(&input, &digest(&input))
                .unwrap_err()
                .contains("duplicate state_id")
        );
        let mut input = base_input();
        input.outcomes[1].outcome_id = input.outcomes[0].outcome_id.clone();
        assert!(
            evaluate(&input, &digest(&input))
                .unwrap_err()
                .contains("duplicate outcome_id")
        );
        let mut input = base_input();
        input.measurement_model.model_artifact_ref = " ".to_owned();
        assert!(
            evaluate(&input, &digest(&input))
                .unwrap_err()
                .contains("model_artifact_ref")
        );
    }

    #[test]
    fn observation_at_shared_boundary_matches_both_outcome_bins() {
        let mut input = base_input();
        input.observed_value = Some(Quantity {
            unit: Unit::Millimeter,
            value_ticks: 800_000_000,
        });
        let report = evaluate(&input, &digest(&input)).expect("boundary observation");
        assert!(report.observed_update.matching_outcome_ids.contains(&"low".to_owned()));
        assert!(
            report
                .observed_update
                .matching_outcome_ids
                .contains(&"middle".to_owned())
        );
    }

    #[test]
    fn input_order_permutation_preserves_semantic_results() {
        let mut first = base_input();
        let mut second = first.clone();
        second.states.reverse();
        second.outcomes.reverse();
        let left = evaluate(&first, &digest(&first)).expect("left report");
        let right = evaluate(&second, &digest(&second)).expect("right report");
        assert_eq!(left.state_results, right.state_results);
        assert_eq!(left.outcome_results, right.outcome_results);
        assert_eq!(left.decision_ambiguity, right.decision_ambiguity);
    }

    #[test]
    fn deterministic_replay_and_generated_interval_oracle() {
        let mut case_count = 0usize;
        let mut generated_edges = 0usize;
        let mut worlds_with_nonzero_discrepancy = 0usize;
        let mut corpus = Vec::new();
        for world in 0..100i64 {
            let mut input = base_input();
            input.world_id = format!("generated-{world:03}");
            let square_world = world % 2 != 0;
            input.measurement_model.model_id = if square_world {
                "synthetic-dimensionless-square".to_owned()
            } else {
                "synthetic-length-identity".to_owned()
            };
            input.measurement_model.model_artifact_ref = if square_world {
                "synthetic-models/dimensionless-square-v1".to_owned()
            } else {
                "synthetic-models/length-identity-v1".to_owned()
            };
            input.measurement_model.input_unit = if square_world { Unit::Dimensionless } else { Unit::Meter };
            input.measurement_model.transform = if square_world {
                Transform::SquareNonNegativeDimensionless
            } else {
                Transform::Identity
            };
            input.measurement_model.input_domain = interval(0, 2_000_000);
            let discrepancy_radius = if world % 4 == 0 || world % 4 == 3 { 25_000 } else { 0 };
            input.measurement_model.model_discrepancy = ModelDiscrepancy {
                unit: if square_world { Unit::Dimensionless } else { Unit::Meter },
                interval: interval(-discrepancy_radius, discrepancy_radius),
                basis: if discrepancy_radius == 0 {
                    "synthetic zero-discrepancy assumption; not calibrated".to_owned()
                } else {
                    "synthetic nonzero discrepancy perturbation; not calibrated".to_owned()
                },
            };
            input.states.clear();
            for state_index in 0..20i64 {
                let low = (state_index * 95_000 + world * 1_000) % 1_700_000;
                let high = (low + 300_000).min(2_000_000);
                input.states.push(state(
                    &format!("w{world:03}-s{state_index:02}"),
                    if square_world { Unit::Dimensionless } else { Unit::Meter },
                    low,
                    high,
                    &format!("decision-{}", (state_index + world) % 4),
                ));
            }
            input.outcomes = if square_world {
                vec![
                    bin("o0", Unit::Dimensionless, 0, 500_000),
                    bin("o1", Unit::Dimensionless, 450_000, 1_500_000),
                    bin("o2", Unit::Dimensionless, 1_450_000, 2_500_000),
                    bin("o3", Unit::Dimensionless, 2_450_000, 3_500_000),
                    bin("o4", Unit::Dimensionless, 3_450_000, 4_000_000),
                ]
            } else {
                vec![
                    bin("o0", Unit::Meter, 0, 500_000),
                    bin("o1", Unit::Meter, 450_000, 900_000),
                    bin("o2", Unit::Meter, 850_000, 1_300_000),
                    bin("o3", Unit::Meter, 1_250_000, 1_700_000),
                    bin("o4", Unit::Meter, 1_650_000, 2_000_000),
                ]
            };
            if discrepancy_radius != 0 {
                worlds_with_nonzero_discrepancy += 1;
            }
            let serialized = serde_json::to_vec(&input).expect("generated input serializes");
            let hash = sha256_hex(&serialized);
            let report = evaluate(&input, &hash).expect("generated input evaluates");
            assert_eq!(evaluate(&input, &hash).expect("replay"), report);

            // Independent direct interval-overlap oracle (does not use evaluator helpers).
            let mut oracle_pairs = BTreeSet::<(String, String)>::new();
            for state in &input.states {
                let (base_lo, base_hi) = if square_world {
                    let lower = i128::from(state.input_interval.lower_ticks);
                    let upper = i128::from(state.input_interval.upper_ticks);
                    let lower_squared = lower * lower;
                    let upper_squared = upper * upper;
                    (
                        lower_squared / FIXED_SCALE,
                        (upper_squared + FIXED_SCALE - 1) / FIXED_SCALE,
                    )
                } else {
                    (
                        i128::from(state.input_interval.lower_ticks) * 1000,
                        i128::from(state.input_interval.upper_ticks) * 1000,
                    )
                };
                let discrepancy_factor = if square_world { 1 } else { 1000 };
                let discrepancy_lo =
                    i128::from(input.measurement_model.model_discrepancy.interval.lower_ticks) * discrepancy_factor;
                let discrepancy_hi =
                    i128::from(input.measurement_model.model_discrepancy.interval.upper_ticks) * discrepancy_factor;
                let lo = base_lo + discrepancy_lo;
                let hi = base_hi + discrepancy_hi;
                for bin in &input.outcomes {
                    let (blo, bhi) = if square_world {
                        (
                            i128::from(bin.interval.lower_ticks),
                            i128::from(bin.interval.upper_ticks),
                        )
                    } else {
                        (
                            i128::from(bin.interval.lower_ticks) * 1000,
                            i128::from(bin.interval.upper_ticks) * 1000,
                        )
                    };
                    if lo <= bhi && blo <= hi {
                        oracle_pairs.insert((state.state_id.clone(), bin.outcome_id.clone()));
                    }
                }
            }
            let evaluated_pairs: BTreeSet<(String, String)> = report
                .state_results
                .iter()
                .flat_map(|state| {
                    state
                        .possible_outcome_ids
                        .iter()
                        .map(move |outcome_id| (state.state_id.clone(), outcome_id.clone()))
                })
                .collect();
            assert_eq!(evaluated_pairs, oracle_pairs);
            let oracle_edges = oracle_pairs.len();
            let oracle_pairs_json: Vec<serde_json::Value> = oracle_pairs
                .iter()
                .map(|(state_id, outcome_id)| serde_json::json!({ "state_id": state_id, "outcome_id": outcome_id }))
                .collect();
            generated_edges += oracle_edges;
            case_count += input.states.len() * input.outcomes.len();
            corpus.push(serde_json::json!({
                "world_index": world,
                "seed": 20261017i64 + world,
                "input_sha256": hash,
                "input": input,
                "oracle_compatibility_edges": oracle_edges,
                "oracle_compatibility_pairs": oracle_pairs_json,
                "report": report
            }));
        }
        assert_eq!(case_count, 100 * 20 * 5);
        assert_eq!(worlds_with_nonzero_discrepancy, 50);
        assert!(generated_edges > 0);

        if let Ok(path) = env::var("CIV_INFRA_COMPATIBILITY_CORPUS_OUT") {
            let artifact = serde_json::json!({
                "corpus_version": "civ-infra-measurement-compatibility-generated/v0.1",
                "generator": "seeded-interval-mutator-v1",
                "seed_base": 20261017,
                "world_count": 100,
                "states_per_world": 20,
                "outcomes_per_world": 5,
                "state_outcome_evaluations": case_count,
                "generated_compatibility_edges": generated_edges,
                "worlds_with_nonzero_discrepancy": worlds_with_nonzero_discrepancy,
                "claim_ceiling": "synthetic interval compatibility software oracle only",
                "cases": corpus
            });
            fs::write(path, serde_json::to_vec_pretty(&artifact).expect("corpus serializes"))
                .expect("write compatibility corpus");
        }
    }

    #[test]
    fn rejects_unsupported_schema_and_wrong_input_digest() {
        let mut input = base_input();
        input.schema_version = "v9".to_owned();
        assert!(
            evaluate(&input, &digest(&input))
                .unwrap_err()
                .contains("unsupported schema_version")
        );
        let input = base_input();
        assert!(evaluate(&input, "not-a-digest").unwrap_err().contains("input_sha256"));
    }

    #[test]
    fn public_byte_entrypoint_binds_exact_raw_bytes() {
        let raw = include_bytes!("../fixtures/compatibility-v0.1.json");
        let report = evaluate_bytes(raw).expect("raw fixture evaluates");
        assert_eq!(report.input_sha256, sha256_hex(raw));

        let mut whitespace_variant = Vec::with_capacity(raw.len() + 1);
        whitespace_variant.push(b' ');
        whitespace_variant.extend_from_slice(raw);
        let variant_report = evaluate_bytes(&whitespace_variant).expect("whitespace variant evaluates");
        assert_ne!(report.input_sha256, variant_report.input_sha256);
        assert_eq!(report.state_results, variant_report.state_results);
    }

    #[test]
    fn public_byte_entrypoint_rejects_oversized_input_before_json_parse() {
        let oversized = vec![b' '; MAX_INPUT_BYTES + 1];
        assert!(
            evaluate_bytes(&oversized)
                .unwrap_err()
                .contains("input byte count exceeds limit")
        );
    }

    #[test]
    fn compatibility_work_bound_accepts_limit_and_rejects_over_limit() {
        assert_eq!(
            validate_work_bound(100, 1_000).expect("limit accepted"),
            MAX_COMPATIBILITY_EDGES
        );
        assert!(validate_work_bound(101, 1_000).unwrap_err().contains("exceeds limit"));
        assert!(validate_work_bound(usize::MAX, 2).unwrap_err().contains("overflow"));
    }
}
