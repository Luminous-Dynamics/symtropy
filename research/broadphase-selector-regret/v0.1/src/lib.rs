#![forbid(unsafe_code)]

//! Sequence-level evaluator for workload-adaptive broadphase policies.
//!
//! This crate consumes frozen benchmark receipts. It does not run a broadphase,
//! generate the pair oracle, train a selector, or authorize production switching.
//! Its offline comparator is a dynamic-programming optimum over qualified
//! backends, with explicit directed transition costs.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

const SHA256_HEX_LEN: usize = 64;
const BASIS_POINT_SCALE: u128 = 10_000;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BackendId(String);

impl BackendId {
    pub fn new(value: impl Into<String>) -> Result<Self, SelectorError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SelectorError::InvalidBackendId);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceSplit {
    Training,
    HeldOut,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionTiming {
    BeforeSimulationFrame,
    AfterSimulationFrame,
}

/// Counts and oracle metadata copied from a separately validated pair-set receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairEvidence {
    pub oracle_result_sha256: String,
    pub eligible_pair_count: u64,
    pub oracle_overlap_pairs: u64,
    pub emitted_candidate_pairs: u64,
    pub true_positive_pairs: u64,
    pub false_positive_pairs: u64,
    pub false_negative_pairs: u64,
    pub oracle_coverage_complete: bool,
    pub receipt_valid: bool,
    pub duplicate_pair_rows: u64,
    pub unknown_pair_endpoints: u64,
    pub unsupported_case_count: u64,
    pub invalid_prune_certificates: u64,
}

impl PairEvidence {
    pub fn validate(&self) -> Result<(), SelectorError> {
        validate_sha256(&self.oracle_result_sha256)?;
        if self.oracle_overlap_pairs > self.eligible_pair_count
            || self.emitted_candidate_pairs > self.eligible_pair_count
            || self.true_positive_pairs > self.oracle_overlap_pairs
            || self.true_positive_pairs > self.emitted_candidate_pairs
        {
            return Err(SelectorError::InvalidPairEvidence);
        }
        let oracle_accounted = self
            .true_positive_pairs
            .checked_add(self.false_negative_pairs)
            .ok_or(SelectorError::PairEvidenceCountOverflow)?;
        let candidates_accounted = self
            .true_positive_pairs
            .checked_add(self.false_positive_pairs)
            .ok_or(SelectorError::PairEvidenceCountOverflow)?;
        if oracle_accounted != self.oracle_overlap_pairs
            || candidates_accounted != self.emitted_candidate_pairs
        {
            return Err(SelectorError::InvalidPairEvidence);
        }
        Ok(())
    }

    /// Eligibility is non-compensatory: speed cannot offset an incomplete oracle,
    /// a missed pair, invalid receipt, duplicate row or unknown endpoint.
    #[must_use]
    pub fn qualifies(&self) -> bool {
        self.receipt_valid
            && self.oracle_coverage_complete
            && self.false_negative_pairs == 0
            && self.duplicate_pair_rows == 0
            && self.unknown_pair_endpoints == 0
            && self.unsupported_case_count == 0
            && self.invalid_prune_certificates == 0
    }
}

/// One backend's observation of one frozen simulation frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendFrameObservation {
    pub frame_index: u64,
    pub backend: BackendId,
    pub aabb_snapshot_sha256: String,
    pub collision_filter_sha256: String,
    pub environment_sha256: String,
    pub implementation_sha256: String,
    pub config_sha256: String,
    pub pair_evidence: PairEvidence,
    pub broadphase_ns: u64,
    pub full_step_ns: u64,
    pub peak_memory_bytes: u64,
}

impl BackendFrameObservation {
    pub fn validate(&self) -> Result<(), SelectorError> {
        if self.backend.as_str().trim().is_empty() {
            return Err(SelectorError::InvalidBackendId);
        }
        validate_sha256(&self.aabb_snapshot_sha256)?;
        validate_sha256(&self.collision_filter_sha256)?;
        validate_sha256(&self.environment_sha256)?;
        validate_sha256(&self.implementation_sha256)?;
        validate_sha256(&self.config_sha256)?;
        self.pair_evidence.validate()?;
        if self.broadphase_ns > self.full_step_ns {
            return Err(SelectorError::BroadphaseExceedsFullStep);
        }
        Ok(())
    }

    #[must_use]
    pub fn qualifies(&self) -> bool {
        self.pair_evidence.qualifies()
    }
}

/// Directed transition cost, including rebuild, migration, warm-up and
/// synchronization needed to switch from one backend to another.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionCost {
    pub from: BackendId,
    pub to: BackendId,
    pub cost_ns: u64,
}

/// Provenance for the selector's pre-frame feature vector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyDecision {
    pub frame_index: u64,
    pub selected_backend: BackendId,
    pub timing: DecisionTiming,
    /// Latest completed frame whose outcomes influenced this decision.
    /// The current frame's outcome must never be used before its simulation.
    pub latest_completed_frame_used: Option<u64>,
    pub feature_snapshot_sha256: String,
    pub selection_overhead_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceInput {
    pub sequence_id: String,
    pub workload_family: String,
    /// Digest of the frozen policy implementation/configuration used for all
    /// decisions in this sequence.
    pub policy_sha256: String,
    pub split: SequenceSplit,
    pub initial_backend: Option<BackendId>,
    pub observations: Vec<BackendFrameObservation>,
    pub transition_costs: Vec<TransitionCost>,
    pub decisions: Vec<PolicyDecision>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceScore {
    pub sequence_id: String,
    pub workload_family: String,
    pub policy_sha256: String,
    pub frame_count: u64,
    pub online_total_cost_ns: u128,
    pub offline_oracle_cost_ns: u128,
    pub best_fixed_backend: Option<BackendId>,
    pub best_fixed_cost_ns: Option<u128>,
    pub regret_basis_points: Option<u128>,
    /// Signed: negative means the adaptive policy beat every fixed backend.
    pub regression_vs_best_fixed_basis_points: Option<i128>,
    pub selected_backend_path: Vec<BackendId>,
    pub offline_oracle_path: Vec<BackendId>,
    pub selected_switches: u64,
    pub selector_overhead_ns: u128,
    pub transition_overhead_ns: u128,
    pub selected_broadphase_ns: u128,
    pub selected_full_step_p50_ns: Option<u64>,
    pub selected_full_step_p95_ns: Option<u64>,
    pub selected_full_step_p99_ns: Option<u64>,
    pub selected_broadphase_p50_ns: Option<u64>,
    pub selected_broadphase_p95_ns: Option<u64>,
    pub selected_broadphase_p99_ns: Option<u64>,
    pub selected_peak_memory_bytes: u64,
    pub selector_and_switch_overhead_basis_points: Option<u128>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilySummary {
    pub workload_family: String,
    pub sequence_count: u64,
    pub median_regret_basis_points: Option<u128>,
    pub p95_regret_basis_points: Option<u128>,
    pub p95_regression_vs_best_fixed_basis_points: Option<i128>,
    pub fixed_baseline_qualified_sequence_count: u64,
    pub fixed_baseline_unavailable_sequence_count: u64,
    pub regret_available_sequence_count: u64,
    pub regret_unavailable_sequence_count: u64,
    pub overhead_ratio_available_sequence_count: u64,
    pub overhead_ratio_unavailable_sequence_count: u64,
    pub median_selector_and_switch_overhead_basis_points: Option<u128>,
    pub p95_selector_and_switch_overhead_basis_points: Option<u128>,
    /// Ratio of summed selector and transition costs to summed selected
    /// broadphase time for this workload family.
    pub aggregate_selector_and_switch_overhead_basis_points: Option<u128>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignSummary {
    pub sequences: Vec<SequenceScore>,
    pub families: Vec<FamilySummary>,
    pub median_regret_basis_points: Option<u128>,
    pub p95_regret_basis_points: Option<u128>,
    pub p95_regression_vs_best_fixed_basis_points: Option<i128>,
    pub fixed_baseline_qualified_sequence_count: u64,
    pub fixed_baseline_unavailable_sequence_count: u64,
    pub regret_available_sequence_count: u64,
    pub regret_unavailable_sequence_count: u64,
    pub overhead_ratio_available_sequence_count: u64,
    pub overhead_ratio_unavailable_sequence_count: u64,
    pub median_selector_and_switch_overhead_basis_points: Option<u128>,
    pub p95_selector_and_switch_overhead_basis_points: Option<u128>,
    /// Ratio of summed selector and transition costs to summed selected
    /// broadphase time across the campaign (the primary overhead gate).
    pub aggregate_selector_and_switch_overhead_basis_points: Option<u128>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectorError {
    InvalidBackendId,
    InvalidDigest,
    InvalidSequenceMetadata,
    TrainingSequenceNotEligible,
    EmptySequence,
    EmptyCampaign,
    DuplicateSequenceId,
    InvalidPairEvidence,
    PairEvidenceCountOverflow,
    BroadphaseExceedsFullStep,
    DuplicateBackendObservation,
    NonContiguousFrameIndices,
    InconsistentFrameInputs,
    InconsistentOracleDenominator,
    InconsistentEnvironment,
    InconsistentCollisionFilter,
    InconsistentBackendBuild,
    InconsistentPolicy,
    NoQualifiedBackendForFrame(u64),
    EmptyOraclePositivePopulation,
    DuplicateTransitionCost,
    InvalidSelfTransitionCost,
    UnknownTransitionEndpoint,
    MissingPolicyDecision,
    DuplicatePolicyDecision,
    DecisionAfterSimulation,
    FutureInformationLeak,
    MissingSelectionObservation,
    SelectedBackendUnqualified,
    MissingPolicySwitchCost,
    NoFeasibleOraclePath,
    CostOverflow,
    OfflineOracleNotOptimal,
    RatioOverflow,
}

impl fmt::Display for SelectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for SelectorError {}

#[derive(Clone, Debug)]
struct DpCell {
    total_cost_ns: u128,
    previous_backend: Option<BackendId>,
}

fn validate_sha256(value: &str) -> Result<(), SelectorError> {
    if value.len() != SHA256_HEX_LEN
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(SelectorError::InvalidDigest);
    }
    Ok(())
}

fn add_cost(left: u128, right: u64) -> Result<u128, SelectorError> {
    left.checked_add(u128::from(right))
        .ok_or(SelectorError::CostOverflow)
}

fn ratio_basis_points(numerator: u128, denominator: u128) -> Result<Option<u128>, SelectorError> {
    if denominator == 0 {
        return Ok(None);
    }
    let whole = numerator / denominator;
    let remainder = numerator % denominator;
    let whole_scaled = whole
        .checked_mul(BASIS_POINT_SCALE)
        .ok_or(SelectorError::RatioOverflow)?;
    let remainder_scaled = remainder
        .checked_mul(BASIS_POINT_SCALE)
        .ok_or(SelectorError::RatioOverflow)?
        / denominator;
    Ok(Some(
        whole_scaled
            .checked_add(remainder_scaled)
            .ok_or(SelectorError::RatioOverflow)?,
    ))
}

fn signed_ratio_basis_points(actual: u128, baseline: u128) -> Result<Option<i128>, SelectorError> {
    if baseline == 0 {
        return Ok(None);
    }
    let (negative, difference) = if actual >= baseline {
        (false, actual - baseline)
    } else {
        (true, baseline - actual)
    };
    let Some(value) = ratio_basis_points(difference, baseline)? else {
        return Ok(None);
    };
    let signed = i128::try_from(value).map_err(|_| SelectorError::RatioOverflow)?;
    Ok(Some(if negative { -signed } else { signed }))
}

fn nearest_rank_u64(values: &mut [u64], percentile: u8) -> Option<u64> {
    if values.is_empty() || !(1..=100).contains(&percentile) {
        return None;
    }
    values.sort_unstable();
    let rank = (values.len() * usize::from(percentile))
        .div_ceil(100)
        .max(1);
    values.get(rank - 1).copied()
}

fn nearest_rank_u128(values: &mut [u128], percentile: u8) -> Option<u128> {
    if values.is_empty() || !(1..=100).contains(&percentile) {
        return None;
    }
    values.sort_unstable();
    let rank = (values.len() * usize::from(percentile))
        .div_ceil(100)
        .max(1);
    values.get(rank - 1).copied()
}

fn nearest_rank_i128(values: &mut [i128], percentile: u8) -> Option<i128> {
    if values.is_empty() || !(1..=100).contains(&percentile) {
        return None;
    }
    values.sort_unstable();
    let rank = (values.len() * usize::from(percentile))
        .div_ceil(100)
        .max(1);
    values.get(rank - 1).copied()
}

fn transition_lookup(
    transitions: &BTreeMap<(BackendId, BackendId), u64>,
    from: &BackendId,
    to: &BackendId,
) -> Option<u64> {
    if from == to {
        Some(0)
    } else {
        transitions.get(&(from.clone(), to.clone())).copied()
    }
}

fn best_fixed_backend(
    frames: &[BTreeMap<BackendId, &BackendFrameObservation>],
    initial_backend: Option<&BackendId>,
    transitions: &BTreeMap<(BackendId, BackendId), u64>,
) -> Result<Option<(BackendId, u128)>, SelectorError> {
    let mut backend_ids = BTreeSet::new();
    for frame in frames {
        backend_ids.extend(frame.keys().cloned());
    }

    let mut best: Option<(BackendId, u128)> = None;
    for backend in backend_ids {
        let mut total = 0_u128;
        let mut complete = true;
        for frame in frames {
            let Some(observation) = frame.get(&backend) else {
                complete = false;
                break;
            };
            if !observation.qualifies() {
                complete = false;
                break;
            }
            total = add_cost(total, observation.full_step_ns)?;
        }
        if !complete {
            continue;
        }

        if let Some(initial) = initial_backend {
            if let Some(cost) = transition_lookup(transitions, initial, &backend) {
                total = add_cost(total, cost)?;
            } else {
                continue;
            }
        }

        let replace = best.as_ref().is_none_or(|(best_id, best_cost)| {
            total < *best_cost || (total == *best_cost && backend < *best_id)
        });
        if replace {
            best = Some((backend, total));
        }
    }
    Ok(best)
}

fn offline_oracle_path(
    frames: &[BTreeMap<BackendId, &BackendFrameObservation>],
    initial_backend: Option<&BackendId>,
    transitions: &BTreeMap<(BackendId, BackendId), u64>,
) -> Result<(Vec<BackendId>, u128), SelectorError> {
    let mut layers: Vec<BTreeMap<BackendId, DpCell>> = Vec::with_capacity(frames.len());

    for (frame_index, frame) in frames.iter().enumerate() {
        let mut current = BTreeMap::new();
        for (backend, observation) in frame {
            if !observation.qualifies() {
                continue;
            }

            let observation_cost = u128::from(observation.full_step_ns);
            if frame_index == 0 {
                let initial_cost = match initial_backend {
                    Some(initial) => match transition_lookup(transitions, initial, backend) {
                        Some(cost) => u128::from(cost),
                        None => continue,
                    },
                    None => 0,
                };
                let total = observation_cost
                    .checked_add(initial_cost)
                    .ok_or(SelectorError::CostOverflow)?;
                current.insert(
                    backend.clone(),
                    DpCell {
                        total_cost_ns: total,
                        previous_backend: None,
                    },
                );
                continue;
            }

            let previous_layer = layers.last().ok_or(SelectorError::NoFeasibleOraclePath)?;
            let mut best: Option<(u128, BackendId)> = None;
            for (previous_backend, previous_cell) in previous_layer {
                let Some(switch_cost) = transition_lookup(transitions, previous_backend, backend)
                else {
                    continue;
                };
                let total = add_cost(previous_cell.total_cost_ns, switch_cost)?;
                let total = total
                    .checked_add(observation_cost)
                    .ok_or(SelectorError::CostOverflow)?;
                let replace = best.as_ref().is_none_or(|(best_cost, best_previous)| {
                    total < *best_cost || (total == *best_cost && previous_backend < best_previous)
                });
                if replace {
                    best = Some((total, previous_backend.clone()));
                }
            }

            if let Some((total_cost_ns, previous_backend)) = best {
                current.insert(
                    backend.clone(),
                    DpCell {
                        total_cost_ns,
                        previous_backend: Some(previous_backend),
                    },
                );
            }
        }

        if current.is_empty() {
            return Err(SelectorError::NoFeasibleOraclePath);
        }
        layers.push(current);
    }

    let last = layers.last().ok_or(SelectorError::EmptySequence)?;
    let mut best_final: Option<(BackendId, u128)> = None;
    for (backend, cell) in last {
        let replace = best_final.as_ref().is_none_or(|(best_id, best_cost)| {
            cell.total_cost_ns < *best_cost
                || (cell.total_cost_ns == *best_cost && backend < best_id)
        });
        if replace {
            best_final = Some((backend.clone(), cell.total_cost_ns));
        }
    }
    let (mut current_backend, total_cost_ns) =
        best_final.ok_or(SelectorError::NoFeasibleOraclePath)?;

    let mut reverse_path = Vec::with_capacity(layers.len());
    for layer_index in (0..layers.len()).rev() {
        reverse_path.push(current_backend.clone());
        if layer_index > 0 {
            current_backend = layers[layer_index]
                .get(&current_backend)
                .and_then(|cell| cell.previous_backend.clone())
                .ok_or(SelectorError::NoFeasibleOraclePath)?;
        }
    }
    reverse_path.reverse();
    Ok((reverse_path, total_cost_ns))
}

/// Evaluate one complete, held-out sequence against a transition-cost-aware
/// sequence-level offline oracle. Frame indices must be exactly 0..N-1.
pub fn evaluate_sequence(input: &SequenceInput) -> Result<SequenceScore, SelectorError> {
    if input.sequence_id.trim().is_empty() || input.workload_family.trim().is_empty() {
        return Err(SelectorError::InvalidSequenceMetadata);
    }
    validate_sha256(&input.policy_sha256)?;
    if input.split != SequenceSplit::HeldOut {
        return Err(SelectorError::TrainingSequenceNotEligible);
    }
    if input.observations.is_empty() {
        return Err(SelectorError::EmptySequence);
    }

    let mut grouped: BTreeMap<u64, BTreeMap<BackendId, &BackendFrameObservation>> = BTreeMap::new();
    let mut known_backends = BTreeSet::new();
    let mut implementation_by_backend: BTreeMap<BackendId, (String, String)> = BTreeMap::new();
    let mut environment_digest: Option<String> = None;

    for observation in &input.observations {
        observation.validate()?;
        known_backends.insert(observation.backend.clone());

        match &environment_digest {
            Some(expected) if expected != &observation.environment_sha256 => {
                return Err(SelectorError::InconsistentEnvironment);
            }
            None => environment_digest = Some(observation.environment_sha256.clone()),
            _ => {}
        }
        let build = (
            observation.implementation_sha256.clone(),
            observation.config_sha256.clone(),
        );
        match implementation_by_backend.get(&observation.backend) {
            Some(expected) if expected != &build => {
                return Err(SelectorError::InconsistentBackendBuild);
            }
            None => {
                implementation_by_backend.insert(observation.backend.clone(), build);
            }
            _ => {}
        }

        let frame = grouped.entry(observation.frame_index).or_default();
        if frame
            .insert(observation.backend.clone(), observation)
            .is_some()
        {
            return Err(SelectorError::DuplicateBackendObservation);
        }
    }

    if input
        .initial_backend
        .as_ref()
        .is_some_and(|initial| initial.as_str().trim().is_empty())
    {
        return Err(SelectorError::InvalidBackendId);
    }

    let mut frames: Vec<BTreeMap<BackendId, &BackendFrameObservation>> =
        Vec::with_capacity(grouped.len());
    let mut positive_oracle_population = false;
    for (expected_index, (frame_index, frame)) in grouped.iter().enumerate() {
        if *frame_index != expected_index as u64 {
            return Err(SelectorError::NonContiguousFrameIndices);
        }
        let first = frame.values().next().ok_or(SelectorError::EmptySequence)?;
        for observation in frame.values() {
            if observation.aabb_snapshot_sha256 != first.aabb_snapshot_sha256 {
                return Err(SelectorError::InconsistentFrameInputs);
            }
            if observation.collision_filter_sha256 != first.collision_filter_sha256 {
                return Err(SelectorError::InconsistentCollisionFilter);
            }
            let left = &observation.pair_evidence;
            let right = &first.pair_evidence;
            if left.oracle_result_sha256 != right.oracle_result_sha256
                || left.eligible_pair_count != right.eligible_pair_count
                || left.oracle_overlap_pairs != right.oracle_overlap_pairs
                || left.oracle_coverage_complete != right.oracle_coverage_complete
            {
                return Err(SelectorError::InconsistentOracleDenominator);
            }
        }
        positive_oracle_population |= first.pair_evidence.oracle_overlap_pairs > 0;
        if !frame.values().any(|observation| observation.qualifies()) {
            return Err(SelectorError::NoQualifiedBackendForFrame(*frame_index));
        }
        frames.push((*frame).clone());
    }
    if !positive_oracle_population {
        return Err(SelectorError::EmptyOraclePositivePopulation);
    }

    let mut transition_map = BTreeMap::new();
    for transition in &input.transition_costs {
        if transition.from == transition.to {
            return Err(SelectorError::InvalidSelfTransitionCost);
        }
        if !known_backends.contains(&transition.to) {
            return Err(SelectorError::UnknownTransitionEndpoint);
        }
        if !known_backends.contains(&transition.from)
            && input.initial_backend.as_ref() != Some(&transition.from)
        {
            return Err(SelectorError::UnknownTransitionEndpoint);
        }
        if transition_map
            .insert(
                (transition.from.clone(), transition.to.clone()),
                transition.cost_ns,
            )
            .is_some()
        {
            return Err(SelectorError::DuplicateTransitionCost);
        }
    }

    // Establish that a complete qualified comparator path exists before
    // attempting to score the submitted policy decisions.
    let (offline_oracle_path, offline_oracle_cost_ns) =
        offline_oracle_path(&frames, input.initial_backend.as_ref(), &transition_map)?;

    let mut decisions = BTreeMap::new();
    for decision in &input.decisions {
        validate_sha256(&decision.feature_snapshot_sha256)?;
        if decision.timing == DecisionTiming::AfterSimulationFrame {
            return Err(SelectorError::DecisionAfterSimulation);
        }
        if decision
            .latest_completed_frame_used
            .is_some_and(|observed| observed >= decision.frame_index)
        {
            return Err(SelectorError::FutureInformationLeak);
        }
        if decisions.insert(decision.frame_index, decision).is_some() {
            return Err(SelectorError::DuplicatePolicyDecision);
        }
    }

    if decisions.len() != frames.len() {
        return Err(SelectorError::MissingPolicyDecision);
    }

    let mut selected_path = Vec::with_capacity(frames.len());
    let mut online_total_cost_ns = 0_u128;
    let mut selector_overhead_ns = 0_u128;
    let mut transition_overhead_ns = 0_u128;
    let mut selected_broadphase_ns = 0_u128;
    let mut selected_switches = 0_u64;
    let mut selected_full_step_samples = Vec::with_capacity(frames.len());
    let mut selected_broadphase_samples = Vec::with_capacity(frames.len());
    let mut selected_peak_memory_bytes = 0_u64;
    let mut previous_backend = input.initial_backend.clone();

    for (frame_index, frame) in frames.iter().enumerate() {
        let frame_index = frame_index as u64;
        let decision = decisions
            .get(&frame_index)
            .ok_or(SelectorError::MissingPolicyDecision)?;
        let observation = frame
            .get(&decision.selected_backend)
            .ok_or(SelectorError::MissingSelectionObservation)?;
        if !observation.qualifies() {
            return Err(SelectorError::SelectedBackendUnqualified);
        }

        online_total_cost_ns = add_cost(online_total_cost_ns, observation.full_step_ns)?;
        selector_overhead_ns = add_cost(selector_overhead_ns, decision.selection_overhead_ns)?;
        online_total_cost_ns = add_cost(online_total_cost_ns, decision.selection_overhead_ns)?;
        selected_broadphase_ns = add_cost(selected_broadphase_ns, observation.broadphase_ns)?;
        selected_full_step_samples.push(observation.full_step_ns);
        selected_broadphase_samples.push(observation.broadphase_ns);
        selected_peak_memory_bytes = selected_peak_memory_bytes.max(observation.peak_memory_bytes);

        if let Some(previous) = &previous_backend
            && previous != &decision.selected_backend
        {
            let transition =
                transition_lookup(&transition_map, previous, &decision.selected_backend)
                    .ok_or(SelectorError::MissingPolicySwitchCost)?;
            transition_overhead_ns = add_cost(transition_overhead_ns, transition)?;
            online_total_cost_ns = add_cost(online_total_cost_ns, transition)?;
            selected_switches = selected_switches.saturating_add(1);
        }

        previous_backend = Some(decision.selected_backend.clone());
        selected_path.push(decision.selected_backend.clone());
    }

    if online_total_cost_ns < offline_oracle_cost_ns {
        return Err(SelectorError::OfflineOracleNotOptimal);
    }

    let regret_basis_points = ratio_basis_points(
        online_total_cost_ns - offline_oracle_cost_ns,
        offline_oracle_cost_ns,
    )?;
    let fixed = best_fixed_backend(&frames, input.initial_backend.as_ref(), &transition_map)?;
    let (best_fixed_backend, best_fixed_cost_ns, regression_vs_best_fixed_basis_points) =
        match fixed {
            Some((backend, cost)) => (
                Some(backend),
                Some(cost),
                signed_ratio_basis_points(online_total_cost_ns, cost)?,
            ),
            None => (None, None, None),
        };
    let selector_and_switch_overhead_basis_points = ratio_basis_points(
        selector_overhead_ns
            .checked_add(transition_overhead_ns)
            .ok_or(SelectorError::CostOverflow)?,
        selected_broadphase_ns,
    )?;

    Ok(SequenceScore {
        sequence_id: input.sequence_id.clone(),
        workload_family: input.workload_family.clone(),
        policy_sha256: input.policy_sha256.clone(),
        frame_count: frames.len() as u64,
        online_total_cost_ns,
        offline_oracle_cost_ns,
        best_fixed_backend,
        best_fixed_cost_ns,
        regret_basis_points,
        regression_vs_best_fixed_basis_points,
        selected_backend_path: selected_path,
        offline_oracle_path,
        selected_switches,
        selector_overhead_ns,
        transition_overhead_ns,
        selected_broadphase_ns,
        selected_full_step_p50_ns: nearest_rank_u64(&mut selected_full_step_samples, 50),
        selected_full_step_p95_ns: nearest_rank_u64(&mut selected_full_step_samples, 95),
        selected_full_step_p99_ns: nearest_rank_u64(&mut selected_full_step_samples, 99),
        selected_broadphase_p50_ns: nearest_rank_u64(&mut selected_broadphase_samples, 50),
        selected_broadphase_p95_ns: nearest_rank_u64(&mut selected_broadphase_samples, 95),
        selected_broadphase_p99_ns: nearest_rank_u64(&mut selected_broadphase_samples, 99),
        selected_peak_memory_bytes,
        selector_and_switch_overhead_basis_points,
    })
}

/// Evaluate only held-out sequences and summarize regret/overhead by workload
/// family. The caller must freeze train/held-out sequence assignment upstream.
pub fn evaluate_campaign(inputs: &[SequenceInput]) -> Result<CampaignSummary, SelectorError> {
    if inputs.is_empty() {
        return Err(SelectorError::EmptyCampaign);
    }
    let expected_policy = &inputs[0].policy_sha256;
    validate_sha256(expected_policy)?;
    let mut ids = BTreeSet::new();
    let mut sequences = Vec::with_capacity(inputs.len());
    for input in inputs {
        if &input.policy_sha256 != expected_policy {
            return Err(SelectorError::InconsistentPolicy);
        }
        if !ids.insert(input.sequence_id.clone()) {
            return Err(SelectorError::DuplicateSequenceId);
        }
        sequences.push(evaluate_sequence(input)?);
    }
    sequences.sort_by(|left, right| left.sequence_id.cmp(&right.sequence_id));

    let mut regrets = sequences
        .iter()
        .filter_map(|score| score.regret_basis_points)
        .collect::<Vec<_>>();
    let mut fixed_regressions = sequences
        .iter()
        .filter_map(|score| score.regression_vs_best_fixed_basis_points)
        .collect::<Vec<_>>();
    let mut overheads = sequences
        .iter()
        .filter_map(|score| score.selector_and_switch_overhead_basis_points)
        .collect::<Vec<_>>();

    let mut grouped: BTreeMap<String, Vec<&SequenceScore>> = BTreeMap::new();
    for score in &sequences {
        grouped
            .entry(score.workload_family.clone())
            .or_default()
            .push(score);
    }

    let mut families = Vec::with_capacity(grouped.len());
    for (workload_family, group) in grouped {
        let mut family_regrets = group
            .iter()
            .filter_map(|score| score.regret_basis_points)
            .collect::<Vec<_>>();
        let mut family_fixed_regressions = group
            .iter()
            .filter_map(|score| score.regression_vs_best_fixed_basis_points)
            .collect::<Vec<_>>();
        let mut family_overheads = group
            .iter()
            .filter_map(|score| score.selector_and_switch_overhead_basis_points)
            .collect::<Vec<_>>();
        let fixed_baseline_qualified_sequence_count = group
            .iter()
            .filter(|score| score.best_fixed_backend.is_some())
            .count() as u64;
        let fixed_baseline_unavailable_sequence_count =
            group.len() as u64 - fixed_baseline_qualified_sequence_count;
        let regret_available_sequence_count = group
            .iter()
            .filter(|score| score.regret_basis_points.is_some())
            .count() as u64;
        let overhead_ratio_available_sequence_count = group
            .iter()
            .filter(|score| score.selector_and_switch_overhead_basis_points.is_some())
            .count() as u64;
        let family_overhead_total = group.iter().try_fold(0_u128, |sum, score| {
            sum.checked_add(score.selector_overhead_ns)
                .and_then(|value| value.checked_add(score.transition_overhead_ns))
                .ok_or(SelectorError::CostOverflow)
        })?;
        let family_broadphase_total = group.iter().try_fold(0_u128, |sum, score| {
            sum.checked_add(score.selected_broadphase_ns)
                .ok_or(SelectorError::CostOverflow)
        })?;
        families.push(FamilySummary {
            workload_family,
            sequence_count: group.len() as u64,
            median_regret_basis_points: nearest_rank_u128(&mut family_regrets, 50),
            p95_regret_basis_points: nearest_rank_u128(&mut family_regrets, 95),
            p95_regression_vs_best_fixed_basis_points: nearest_rank_i128(
                &mut family_fixed_regressions,
                95,
            ),
            fixed_baseline_qualified_sequence_count,
            fixed_baseline_unavailable_sequence_count,
            regret_available_sequence_count,
            regret_unavailable_sequence_count: group.len() as u64 - regret_available_sequence_count,
            overhead_ratio_available_sequence_count,
            overhead_ratio_unavailable_sequence_count: group.len() as u64
                - overhead_ratio_available_sequence_count,
            median_selector_and_switch_overhead_basis_points: nearest_rank_u128(
                &mut family_overheads,
                50,
            ),
            p95_selector_and_switch_overhead_basis_points: nearest_rank_u128(
                &mut family_overheads,
                95,
            ),
            aggregate_selector_and_switch_overhead_basis_points: ratio_basis_points(
                family_overhead_total,
                family_broadphase_total,
            )?,
        });
    }

    let fixed_baseline_qualified_sequence_count = sequences
        .iter()
        .filter(|score| score.best_fixed_backend.is_some())
        .count() as u64;
    let fixed_baseline_unavailable_sequence_count =
        sequences.len() as u64 - fixed_baseline_qualified_sequence_count;
    let regret_available_sequence_count = sequences
        .iter()
        .filter(|score| score.regret_basis_points.is_some())
        .count() as u64;
    let overhead_ratio_available_sequence_count = sequences
        .iter()
        .filter(|score| score.selector_and_switch_overhead_basis_points.is_some())
        .count() as u64;

    let campaign_overhead_total = sequences.iter().try_fold(0_u128, |sum, score| {
        sum.checked_add(score.selector_overhead_ns)
            .and_then(|value| value.checked_add(score.transition_overhead_ns))
            .ok_or(SelectorError::CostOverflow)
    })?;
    let campaign_broadphase_total = sequences.iter().try_fold(0_u128, |sum, score| {
        sum.checked_add(score.selected_broadphase_ns)
            .ok_or(SelectorError::CostOverflow)
    })?;

    let sequence_count = sequences.len() as u64;

    Ok(CampaignSummary {
        sequences,
        families,
        median_regret_basis_points: nearest_rank_u128(&mut regrets, 50),
        p95_regret_basis_points: nearest_rank_u128(&mut regrets, 95),
        p95_regression_vs_best_fixed_basis_points: nearest_rank_i128(&mut fixed_regressions, 95),
        fixed_baseline_qualified_sequence_count,
        fixed_baseline_unavailable_sequence_count,
        regret_available_sequence_count,
        regret_unavailable_sequence_count: sequence_count - regret_available_sequence_count,
        overhead_ratio_available_sequence_count,
        overhead_ratio_unavailable_sequence_count: sequence_count
            - overhead_ratio_available_sequence_count,
        median_selector_and_switch_overhead_basis_points: nearest_rank_u128(&mut overheads, 50),
        p95_selector_and_switch_overhead_basis_points: nearest_rank_u128(&mut overheads, 95),
        aggregate_selector_and_switch_overhead_basis_points: ratio_basis_points(
            campaign_overhead_total,
            campaign_broadphase_total,
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend(value: &str) -> BackendId {
        BackendId::new(value).unwrap()
    }

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn evidence(oracle_pairs: u64, false_negatives: u64) -> PairEvidence {
        let true_positive = oracle_pairs - false_negatives;
        PairEvidence {
            oracle_result_sha256: digest('a'),
            eligible_pair_count: 10,
            oracle_overlap_pairs: oracle_pairs,
            emitted_candidate_pairs: true_positive,
            true_positive_pairs: true_positive,
            false_positive_pairs: 0,
            false_negative_pairs: false_negatives,
            oracle_coverage_complete: true,
            receipt_valid: true,
            duplicate_pair_rows: 0,
            unknown_pair_endpoints: 0,
            unsupported_case_count: 0,
            invalid_prune_certificates: 0,
        }
    }

    fn observation(
        frame_index: u64,
        backend_id: &str,
        broadphase_ns: u64,
        full_step_ns: u64,
        false_negatives: u64,
    ) -> BackendFrameObservation {
        let mut pair_evidence = evidence(2, false_negatives);
        pair_evidence.oracle_result_sha256 =
            digest(char::from_u32(97 + frame_index as u32).unwrap());
        BackendFrameObservation {
            frame_index,
            backend: backend(backend_id),
            aabb_snapshot_sha256: digest(char::from_u32(48 + frame_index as u32).unwrap()),
            collision_filter_sha256: digest('b'),
            environment_sha256: digest('c'),
            implementation_sha256: digest(if backend_id == "A" { 'd' } else { 'e' }),
            config_sha256: digest(if backend_id == "A" { 'f' } else { '1' }),
            pair_evidence,
            broadphase_ns,
            full_step_ns,
            peak_memory_bytes: 4096,
        }
    }

    fn input(choices: &[&str], costs: &[(&str, &str, u64)]) -> SequenceInput {
        let mut observations = Vec::new();
        for (frame, (a_cost, b_cost)) in [(100_u64, 50_u64), (100, 50), (100, 100)]
            .into_iter()
            .enumerate()
        {
            for backend_id in ["A", "B"] {
                let full = if backend_id == "A" { a_cost } else { b_cost };
                observations.push(observation(frame as u64, backend_id, full / 2, full, 0));
            }
        }
        let decisions = choices
            .iter()
            .enumerate()
            .map(|(frame, selected)| PolicyDecision {
                frame_index: frame as u64,
                selected_backend: backend(selected),
                timing: DecisionTiming::BeforeSimulationFrame,
                latest_completed_frame_used: frame.checked_sub(1).map(|index| index as u64),
                feature_snapshot_sha256: digest('9'),
                selection_overhead_ns: 0,
            })
            .collect();
        SequenceInput {
            sequence_id: "heldout-01".to_owned(),
            workload_family: "clustered".to_owned(),
            policy_sha256: digest('8'),
            split: SequenceSplit::HeldOut,
            initial_backend: None,
            observations,
            transition_costs: costs
                .iter()
                .map(|(from, to, cost_ns)| TransitionCost {
                    from: backend(from),
                    to: backend(to),
                    cost_ns: *cost_ns,
                })
                .collect(),
            decisions,
        }
    }

    #[test]
    fn sequence_oracle_prices_switching_and_beats_framewise_minimum() {
        let mut value = input(&["A", "A", "A"], &[("A", "B", 40), ("B", "A", 40)]);
        // B is the cheaper backend on the first two frames; A wins the final
        // frame. The oracle must price the transition rather than switch for free.
        value.observations.iter_mut().for_each(|row| {
            if row.backend.as_str() == "B" && row.frame_index == 2 {
                row.full_step_ns = 150;
                row.broadphase_ns = 75;
            }
        });
        let score = evaluate_sequence(&value).unwrap();
        assert_eq!(
            score.offline_oracle_path,
            vec![backend("B"), backend("B"), backend("A")]
        );
        assert_eq!(score.offline_oracle_cost_ns, 240);
        assert_eq!(score.online_total_cost_ns, 300);
        assert_eq!(score.regret_basis_points, Some(2_500));
        assert_eq!(score.best_fixed_backend, Some(backend("B")));
        assert_eq!(score.best_fixed_cost_ns, Some(250));
        assert_eq!(score.selected_full_step_p50_ns, Some(100));
        assert_eq!(score.selected_full_step_p95_ns, Some(100));
        assert_eq!(score.selected_full_step_p99_ns, Some(100));
        assert_eq!(score.selected_broadphase_p50_ns, Some(50));
        assert_eq!(score.selected_peak_memory_bytes, 4096);
    }

    #[test]
    fn missed_pair_backend_is_never_an_oracle_candidate_or_policy_selection() {
        let mut value = input(&["A", "A", "A"], &[]);
        for row in &mut value.observations {
            if row.backend.as_str() == "B" {
                let oracle_digest = row.pair_evidence.oracle_result_sha256.clone();
                row.pair_evidence = evidence(2, 1);
                row.pair_evidence.oracle_result_sha256 = oracle_digest;
                row.full_step_ns = 1;
                row.broadphase_ns = 1;
            }
        }
        let score = evaluate_sequence(&value).unwrap();
        assert!(
            score
                .offline_oracle_path
                .iter()
                .all(|backend| backend.as_str() == "A")
        );
        value.decisions[0].selected_backend = backend("B");
        assert_eq!(
            evaluate_sequence(&value),
            Err(SelectorError::SelectedBackendUnqualified)
        );
    }

    #[test]
    fn unsupported_or_invalid_prune_receipts_disqualify_a_backend() {
        let mut value = input(&["A", "A", "A"], &[]);
        for row in &mut value.observations {
            if row.backend.as_str() == "B" {
                row.pair_evidence.invalid_prune_certificates = 1;
                row.full_step_ns = 1;
                row.broadphase_ns = 1;
            }
        }
        let score = evaluate_sequence(&value).unwrap();
        assert!(
            score
                .offline_oracle_path
                .iter()
                .all(|backend| backend.as_str() == "A")
        );

        for row in &mut value.observations {
            if row.backend.as_str() == "B" {
                row.pair_evidence.invalid_prune_certificates = 0;
                row.pair_evidence.unsupported_case_count = 1;
            }
        }
        assert!(
            evaluate_sequence(&value)
                .unwrap()
                .offline_oracle_path
                .iter()
                .all(|backend| backend.as_str() == "A")
        );
    }

    #[test]
    fn future_frame_feature_provenance_is_rejected() {
        let mut value = input(&["A", "A", "A"], &[]);
        value.decisions[1].latest_completed_frame_used = Some(1);
        assert_eq!(
            evaluate_sequence(&value),
            Err(SelectorError::FutureInformationLeak)
        );
    }

    #[test]
    fn after_step_decisions_are_rejected() {
        let mut value = input(&["A", "A", "A"], &[]);
        value.decisions[0].timing = DecisionTiming::AfterSimulationFrame;
        assert_eq!(
            evaluate_sequence(&value),
            Err(SelectorError::DecisionAfterSimulation)
        );
    }

    #[test]
    fn mismatched_frozen_aabb_snapshots_are_rejected() {
        let mut value = input(&["A", "A", "A"], &[]);
        value.observations[1].aabb_snapshot_sha256 = digest('8');
        assert_eq!(
            evaluate_sequence(&value),
            Err(SelectorError::InconsistentFrameInputs)
        );
    }

    #[test]
    fn filter_digest_may_change_between_frames_but_must_match_backends() {
        let mut valid = input(&["A", "A", "A"], &[]);
        for row in &mut valid.observations {
            if row.frame_index == 1 {
                row.collision_filter_sha256 = digest('7');
            }
        }
        assert!(evaluate_sequence(&valid).is_ok());

        let mut invalid = input(&["A", "A", "A"], &[]);
        let row = invalid
            .observations
            .iter_mut()
            .find(|row| row.frame_index == 1 && row.backend.as_str() == "B")
            .unwrap();
        row.collision_filter_sha256 = digest('7');
        assert_eq!(
            evaluate_sequence(&invalid),
            Err(SelectorError::InconsistentCollisionFilter)
        );
    }

    #[test]
    fn empty_positive_oracle_population_is_unavailable_not_a_perfect_score() {
        let mut value = input(&["A", "A", "A"], &[]);
        for row in &mut value.observations {
            let oracle_digest = row.pair_evidence.oracle_result_sha256.clone();
            row.pair_evidence = evidence(0, 0);
            row.pair_evidence.oracle_result_sha256 = oracle_digest;
        }
        assert_eq!(
            evaluate_sequence(&value),
            Err(SelectorError::EmptyOraclePositivePopulation)
        );
    }

    #[test]
    fn incomplete_oracle_cannot_qualify_any_backend() {
        let mut value = input(&["A", "A", "A"], &[]);
        for row in &mut value.observations {
            row.pair_evidence.oracle_coverage_complete = false;
        }
        assert_eq!(
            evaluate_sequence(&value),
            Err(SelectorError::NoQualifiedBackendForFrame(0))
        );
    }

    #[test]
    fn missing_directed_switch_cost_blocks_a_switching_oracle_path() {
        let mut value = input(&["A", "A", "A"], &[]);
        // Make A qualify only on frame 0 and B only on frames 1 and 2.
        for row in &mut value.observations {
            if (row.frame_index == 0 && row.backend.as_str() == "B")
                || (row.frame_index > 0 && row.backend.as_str() == "A")
            {
                row.pair_evidence.false_negative_pairs = 1;
                row.pair_evidence.true_positive_pairs = 1;
                row.pair_evidence.emitted_candidate_pairs = 1;
            }
        }
        assert_eq!(
            evaluate_sequence(&value),
            Err(SelectorError::NoFeasibleOraclePath)
        );
    }

    #[test]
    fn policy_switch_cost_is_included_in_online_total() {
        let value = input(&["A", "B", "B"], &[("A", "B", 40)]);
        let score = evaluate_sequence(&value).unwrap();
        assert_eq!(score.transition_overhead_ns, 40);
        assert_eq!(score.selected_switches, 1);
        assert_eq!(score.online_total_cost_ns, 100 + 50 + 100 + 40);
    }

    #[test]
    fn held_out_campaign_summarizes_families_deterministically() {
        let first = input(&["A", "A", "A"], &[("A", "B", 40), ("B", "A", 40)]);
        let mut second = first.clone();
        second.sequence_id = "heldout-02".to_owned();
        let report = evaluate_campaign(&[second, first]).unwrap();
        assert_eq!(report.sequences.len(), 2);
        assert_eq!(report.families.len(), 1);
        assert_eq!(report.families[0].workload_family, "clustered");
        assert_eq!(report.fixed_baseline_qualified_sequence_count, 2);
        assert_eq!(report.fixed_baseline_unavailable_sequence_count, 0);
        assert_eq!(
            report.families[0].fixed_baseline_qualified_sequence_count,
            2
        );
        assert_eq!(
            report.families[0].fixed_baseline_unavailable_sequence_count,
            0
        );
        assert!(
            report
                .aggregate_selector_and_switch_overhead_basis_points
                .is_some()
        );
        assert!(
            report.families[0]
                .aggregate_selector_and_switch_overhead_basis_points
                .is_some()
        );
        assert_eq!(report.sequences[0].sequence_id, "heldout-01");
    }

    #[test]
    fn campaign_preserves_zero_denominator_metric_counts() {
        let mut value = input(&["A", "A", "A"], &[]);
        for observation in &mut value.observations {
            observation.full_step_ns = 0;
            observation.broadphase_ns = 0;
        }
        let report = evaluate_campaign(&[value]).unwrap();
        assert_eq!(report.regret_available_sequence_count, 0);
        assert_eq!(report.regret_unavailable_sequence_count, 1);
        assert_eq!(report.overhead_ratio_available_sequence_count, 0);
        assert_eq!(report.overhead_ratio_unavailable_sequence_count, 1);
        assert_eq!(report.families[0].regret_available_sequence_count, 0);
        assert_eq!(report.families[0].regret_unavailable_sequence_count, 1);
    }

    #[test]
    fn campaign_rejects_mixed_selector_policy_versions() {
        let first = input(&["A", "A", "A"], &[]);
        let mut second = first.clone();
        second.sequence_id = "heldout-02".to_owned();
        second.policy_sha256 = digest('7');
        assert_eq!(
            evaluate_campaign(&[first, second]),
            Err(SelectorError::InconsistentPolicy)
        );
    }

    #[test]
    fn invalid_policy_digest_is_rejected() {
        let mut value = input(&["A", "A", "A"], &[]);
        value.policy_sha256 = "not-a-sha256".to_owned();
        assert_eq!(evaluate_sequence(&value), Err(SelectorError::InvalidDigest));
    }

    #[test]
    fn training_sequences_cannot_be_scored_as_held_out() {
        let mut value = input(&["A", "A", "A"], &[]);
        value.split = SequenceSplit::Training;
        assert_eq!(
            evaluate_sequence(&value),
            Err(SelectorError::TrainingSequenceNotEligible)
        );
    }

    #[test]
    fn transition_costs_are_directional() {
        let mut value = input(&["A", "A", "A"], &[("B", "A", 60)]);
        value.initial_backend = Some(backend("B"));
        // The B -> A transition exists and is charged to the selected policy.
        // The reverse A -> B transition is absent and is not inferred.
        let score = evaluate_sequence(&value).unwrap();
        assert_eq!(score.transition_overhead_ns, 60);
        assert!(
            score
                .offline_oracle_path
                .iter()
                .all(|id| id == &backend("B"))
        );
    }
}
