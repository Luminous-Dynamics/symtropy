//! Deterministic experience ranking over finite, pre-qualified evidence records.
//!
//! The selector consumes upstream expected-loss, timing and SAM evidence. It
//! does not estimate true value of information, authenticate digests, validate
//! SAM, train a policy, run a simulator, or certify safety.

use std::cmp::Ordering;
use std::collections::BTreeSet;

pub const PPM_SCALE: u64 = 1_000_000;
pub const MAX_CANDIDATES: usize = 100_000;
pub const MAX_ID_BYTES: usize = 256;
pub const MAX_DIGEST_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceDisposition {
    Admitted,
    Missing,
    Stale,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceRole {
    PolicyVisible,
    TeacherOnly,
    EvaluatorOnly,
    Future,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Strategy {
    UniformCyclic { cursor_after_id: Option<String> },
    FamilyRarity,
    Entropy,
    PredictionError,
    DecisionMargin,
    SamDecisionValue,
    SamRecoverability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RejectionReason {
    EmptyIdentity,
    IdentityTooLong,
    MissingSourceDigest,
    ScenarioEvidenceNotAdmitted,
    ObservationEvidenceNotAdmitted,
    ObservationNotPolicyVisible,
    ObservationFromFuture,
    ObservationTooOld,
    SamEvidenceNotAdmitted,
    SamContextMismatch,
    MissingStrategyMetric,
    MetricOutOfRange,
    QueryLatencyMissing,
    DecisionDeadlineMissing,
    QueryMissesDeadline,
    TimingOverflow,
    CurrentLossMissing,
    PostQueryLossMissing,
    NoPositiveDecisionLossReduction,
    AcquisitionCostMissing,
    AcquisitionCostZero,
    AcquisitionCostOverLimit,
    DuplicateCandidateId,
    MixedSelectionTicks,
    CampaignTooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamEvidence {
    pub disposition: EvidenceDisposition,
    pub evidence_digest: String,
    pub checkpoint_digest: String,
    pub context_digest: String,
    pub observation_digest: String,
    pub current_expected_loss_microunits: Option<u64>,
    pub expected_post_query_loss_microunits: Option<u64>,
    pub acquisition_cost_micro_units: Option<u64>,
    pub option_fragility_ppm: Option<u32>,
    pub recoverability_depth: Option<u32>,
    pub residual_decision_ticks: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub candidate_id: String,
    pub candidate_digest: String,
    pub scenario_digest: String,
    pub observation_digest: String,
    pub checkpoint_digest: String,
    pub context_digest: String,
    pub scenario_evidence: EvidenceDisposition,
    pub observation_evidence: EvidenceDisposition,
    pub observation_source_role: SourceRole,
    pub observation_available_tick: u64,
    pub selection_tick: u64,
    pub family_id: String,
    pub family_seen_count: Option<u64>,
    pub entropy_microbits: Option<u64>,
    pub prediction_error_ppm: Option<u32>,
    pub decision_margin_ppm: Option<u32>,
    pub expected_query_latency_ticks: Option<u64>,
    pub decision_deadline_tick: Option<u64>,
    pub sam: Option<SamEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionConfig {
    pub maximum_observation_age_ticks: u64,
    pub actuation_reserve_ticks: u64,
    pub maximum_acquisition_cost_micro_units: u64,
    pub selection_limit: usize,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            maximum_observation_age_ticks: 0,
            actuation_reserve_ticks: 0,
            maximum_acquisition_cost_micro_units: u64::MAX,
            selection_limit: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RankEvidence {
    CyclicDistance(usize),
    FamilyExposureCount(u64),
    EntropyMicrobits(u64),
    PredictionErrorPpm(u32),
    DecisionMarginPpm(u32),
    SamDecisionValue {
        regret_reduction_microunits: u64,
        acquisition_cost_micro_units: u64,
        query_arrival_tick: u64,
        decision_deadline_tick: u64,
        actuation_reserve_ticks: u64,
    },
    SamRecoverability {
        option_fragility_ppm: u32,
        recoverability_depth: u32,
        residual_decision_ticks: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedCandidate {
    pub candidate_id: String,
    pub candidate_digest: String,
    pub scenario_digest: String,
    pub observation_digest: String,
    pub checkpoint_digest: String,
    pub context_digest: String,
    pub evidence_digest: String,
    pub rank_evidence: RankEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedCandidate {
    pub candidate_id: String,
    pub reason: RejectionReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionReceipt {
    pub strategy: Strategy,
    pub selection_tick: u64,
    pub candidates_considered: Vec<String>,
    pub ranked_candidates: Vec<RankedCandidate>,
    pub selected_candidate_ids: Vec<String>,
    pub rejected_candidates: Vec<RejectedCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionError {
    TooManyCandidates { actual: usize, limit: usize },
    DuplicateCandidateId(String),
    MixedSelectionTicks,
    EmptyCursor,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn valid_digest(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_DIGEST_BYTES
}

fn base_rejection(candidate: &Candidate, config: SelectionConfig) -> Option<RejectionReason> {
    if !valid_id(&candidate.candidate_id) || !valid_id(&candidate.family_id) {
        return Some(if candidate.candidate_id.len() > MAX_ID_BYTES
            || candidate.family_id.len() > MAX_ID_BYTES
        {
            RejectionReason::IdentityTooLong
        } else {
            RejectionReason::EmptyIdentity
        });
    }
    if !valid_digest(&candidate.candidate_digest)
        || !valid_digest(&candidate.scenario_digest)
        || !valid_digest(&candidate.observation_digest)
        || !valid_digest(&candidate.checkpoint_digest)
        || !valid_digest(&candidate.context_digest)
    {
        return Some(RejectionReason::MissingSourceDigest);
    }
    if candidate.scenario_evidence != EvidenceDisposition::Admitted {
        return Some(RejectionReason::ScenarioEvidenceNotAdmitted);
    }
    if candidate.observation_evidence != EvidenceDisposition::Admitted {
        return Some(RejectionReason::ObservationEvidenceNotAdmitted);
    }
    if candidate.observation_source_role != SourceRole::PolicyVisible {
        return Some(RejectionReason::ObservationNotPolicyVisible);
    }
    if candidate.observation_available_tick > candidate.selection_tick {
        return Some(RejectionReason::ObservationFromFuture);
    }
    let age = candidate.selection_tick - candidate.observation_available_tick;
    if age > config.maximum_observation_age_ticks {
        return Some(RejectionReason::ObservationTooOld);
    }
    None
}

fn sam_base_rejection(candidate: &Candidate) -> Option<RejectionReason> {
    let Some(sam) = &candidate.sam else {
        return Some(RejectionReason::SamEvidenceNotAdmitted);
    };
    if sam.disposition != EvidenceDisposition::Admitted {
        return Some(RejectionReason::SamEvidenceNotAdmitted);
    }
    if !valid_digest(&sam.evidence_digest)
        || !valid_digest(&sam.checkpoint_digest)
        || !valid_digest(&sam.context_digest)
        || !valid_digest(&sam.observation_digest)
        || sam.checkpoint_digest != candidate.checkpoint_digest
        || sam.context_digest != candidate.context_digest
        || sam.observation_digest != candidate.observation_digest
    {
        return Some(RejectionReason::SamContextMismatch);
    }
    None
}

fn rank_for(
    candidate: &Candidate,
    strategy: &Strategy,
    config: SelectionConfig,
) -> Result<RankEvidence, RejectionReason> {
    match strategy {
        Strategy::UniformCyclic { .. } => Ok(RankEvidence::CyclicDistance(0)),
        Strategy::FamilyRarity => candidate
            .family_seen_count
            .map(RankEvidence::FamilyExposureCount)
            .ok_or(RejectionReason::MissingStrategyMetric),
        Strategy::Entropy => candidate
            .entropy_microbits
            .map(RankEvidence::EntropyMicrobits)
            .ok_or(RejectionReason::MissingStrategyMetric),
        Strategy::PredictionError => {
            let error = candidate.prediction_error_ppm
                .ok_or(RejectionReason::MissingStrategyMetric)?;
            if u64::from(error) > PPM_SCALE {
                return Err(RejectionReason::MetricOutOfRange);
            }
            Ok(RankEvidence::PredictionErrorPpm(error))
        }
        Strategy::DecisionMargin => {
            let margin = candidate
                .decision_margin_ppm
                .ok_or(RejectionReason::MissingStrategyMetric)?;
            if u64::from(margin) > PPM_SCALE {
                return Err(RejectionReason::MetricOutOfRange);
            }
            Ok(RankEvidence::DecisionMarginPpm(margin))
        }
        Strategy::SamDecisionValue => {
            if let Some(reason) = sam_base_rejection(candidate) {
                return Err(reason);
            }
            let sam = candidate.sam.as_ref().expect("checked above");
            let current = sam.current_expected_loss_microunits
                .ok_or(RejectionReason::CurrentLossMissing)?;
            let after = sam.expected_post_query_loss_microunits
                .ok_or(RejectionReason::PostQueryLossMissing)?;
            let gain = current.checked_sub(after)
                .filter(|gain| *gain > 0)
                .ok_or(RejectionReason::NoPositiveDecisionLossReduction)?;
            let cost = sam.acquisition_cost_micro_units
                .ok_or(RejectionReason::AcquisitionCostMissing)?;
            if cost == 0 {
                return Err(RejectionReason::AcquisitionCostZero);
            }
            if cost > config.maximum_acquisition_cost_micro_units {
                return Err(RejectionReason::AcquisitionCostOverLimit);
            }
            let latency = candidate.expected_query_latency_ticks
                .ok_or(RejectionReason::QueryLatencyMissing)?;
            let deadline = candidate.decision_deadline_tick
                .ok_or(RejectionReason::DecisionDeadlineMissing)?;
            let arrival = candidate.selection_tick.checked_add(latency)
                .ok_or(RejectionReason::TimingOverflow)?;
            let latest_arrival = deadline.checked_sub(config.actuation_reserve_ticks)
                .ok_or(RejectionReason::QueryMissesDeadline)?;
            if arrival > latest_arrival {
                return Err(RejectionReason::QueryMissesDeadline);
            }
            Ok(RankEvidence::SamDecisionValue {
                regret_reduction_microunits: gain,
                acquisition_cost_micro_units: cost,
                query_arrival_tick: arrival,
                decision_deadline_tick: deadline,
                actuation_reserve_ticks: config.actuation_reserve_ticks,
            })
        }
        Strategy::SamRecoverability => {
            if let Some(reason) = sam_base_rejection(candidate) {
                return Err(reason);
            }
            let sam = candidate.sam.as_ref().expect("checked above");
            let fragility = sam.option_fragility_ppm
                .ok_or(RejectionReason::MissingStrategyMetric)?;
            if u64::from(fragility) > PPM_SCALE {
                return Err(RejectionReason::MetricOutOfRange);
            }
            let depth = sam.recoverability_depth
                .ok_or(RejectionReason::MissingStrategyMetric)?;
            let residual = sam.residual_decision_ticks
                .ok_or(RejectionReason::MissingStrategyMetric)?;
            Ok(RankEvidence::SamRecoverability {
                option_fragility_ppm: fragility,
                recoverability_depth: depth,
                residual_decision_ticks: residual,
            })
        }
    }
}

fn compare_rank(left: &RankEvidence, right: &RankEvidence, strategy: &Strategy) -> Ordering {
    match (strategy, left, right) {
        (
            Strategy::UniformCyclic { .. },
            RankEvidence::CyclicDistance(a),
            RankEvidence::CyclicDistance(b),
        ) => a.cmp(b),
        (
            Strategy::FamilyRarity,
            RankEvidence::FamilyExposureCount(a),
            RankEvidence::FamilyExposureCount(b),
        ) => a.cmp(b),
        (
            Strategy::Entropy,
            RankEvidence::EntropyMicrobits(a),
            RankEvidence::EntropyMicrobits(b),
        ) => b.cmp(a),
        (
            Strategy::PredictionError,
            RankEvidence::PredictionErrorPpm(a),
            RankEvidence::PredictionErrorPpm(b),
        ) => b.cmp(a),
        (
            Strategy::DecisionMargin,
            RankEvidence::DecisionMarginPpm(a),
            RankEvidence::DecisionMarginPpm(b),
        ) => a.cmp(b),
        (
            Strategy::SamDecisionValue,
            RankEvidence::SamDecisionValue {
                regret_reduction_microunits: gain_a,
                acquisition_cost_micro_units: cost_a,
                ..
            },
            RankEvidence::SamDecisionValue {
                regret_reduction_microunits: gain_b,
                acquisition_cost_micro_units: cost_b,
                ..
            },
        ) => {
            let left_product = u128::from(*gain_a) * u128::from(*cost_b);
            let right_product = u128::from(*gain_b) * u128::from(*cost_a);
            right_product.cmp(&left_product)
        }
        (
            Strategy::SamRecoverability,
            RankEvidence::SamRecoverability {
                option_fragility_ppm: fragility_a,
                recoverability_depth: depth_a,
                residual_decision_ticks: residual_a,
            },
            RankEvidence::SamRecoverability {
                option_fragility_ppm: fragility_b,
                recoverability_depth: depth_b,
                residual_decision_ticks: residual_b,
            },
        ) => fragility_b
            .cmp(fragility_a)
            .then_with(|| depth_a.cmp(depth_b))
            .then_with(|| residual_a.cmp(residual_b)),
        _ => Ordering::Equal,
    }
}

/// Build a deterministic selection receipt over finite candidates.
///
/// Invalid evidence and ineligible queries are returned as typed rejections.
/// The function does not authenticate digests or calibrate supplied SAM values.
pub fn select(
    candidates: &[Candidate],
    strategy: Strategy,
    config: SelectionConfig,
) -> Result<SelectionReceipt, SelectionError> {
    if candidates.len() > MAX_CANDIDATES {
        return Err(SelectionError::TooManyCandidates {
            actual: candidates.len(),
            limit: MAX_CANDIDATES,
        });
    }
    if let Strategy::UniformCyclic {
        cursor_after_id: Some(cursor),
    } = &strategy
    {
        if !valid_id(cursor) {
            return Err(SelectionError::EmptyCursor);
        }
    }
    let mut seen = BTreeSet::new();
    for candidate in candidates {
        if !seen.insert(candidate.candidate_id.clone()) {
            return Err(SelectionError::DuplicateCandidateId(
                candidate.candidate_id.clone(),
            ));
        }
    }
    let selection_tick = candidates
        .first()
        .map(|candidate| candidate.selection_tick)
        .unwrap_or(0);
    if candidates
        .iter()
        .any(|candidate| candidate.selection_tick != selection_tick)
    {
        return Err(SelectionError::MixedSelectionTicks);
    }

    let mut candidates_sorted = candidates.iter().collect::<Vec<_>>();
    candidates_sorted.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));

    let mut ranked_candidates = Vec::new();
    let mut rejected_candidates = Vec::new();
    for candidate in &candidates_sorted {
        if let Some(reason) = base_rejection(candidate, config) {
            rejected_candidates.push(RejectedCandidate {
                candidate_id: candidate.candidate_id.clone(),
                reason,
            });
            continue;
        }
        match rank_for(candidate, &strategy, config) {
            Ok(rank_evidence) => {
                ranked_candidates.push(RankedCandidate {
                    candidate_id: candidate.candidate_id.clone(),
                    candidate_digest: candidate.candidate_digest.clone(),
                    scenario_digest: candidate.scenario_digest.clone(),
                    observation_digest: candidate.observation_digest.clone(),
                    checkpoint_digest: candidate.checkpoint_digest.clone(),
                    context_digest: candidate.context_digest.clone(),
                    evidence_digest: candidate
                        .sam
                        .as_ref()
                        .filter(|_| matches!(&strategy, Strategy::SamDecisionValue | Strategy::SamRecoverability))
                        .map(|sam| sam.evidence_digest.clone())
                        .unwrap_or_else(|| candidate.candidate_digest.clone()),
                    rank_evidence,
                });
            }
            Err(reason) => rejected_candidates.push(RejectedCandidate {
                candidate_id: candidate.candidate_id.clone(),
                reason,
            }),
        }
    }

    if let Strategy::UniformCyclic { cursor_after_id } = &strategy {
        let cursor = cursor_after_id.as_deref().unwrap_or("");
        let cursor_position = ranked_candidates
            .iter()
            .position(|candidate| candidate.candidate_id == cursor);
        let split = match cursor_position {
            Some(position) => position + 1,
            None => ranked_candidates
                .iter()
                .position(|candidate| candidate.candidate_id.as_str() > cursor)
                .unwrap_or(0),
        };
        ranked_candidates.rotate_left(split);
        for (distance, candidate) in ranked_candidates.iter_mut().enumerate() {
            candidate.rank_evidence = RankEvidence::CyclicDistance(distance);
        }
    } else {
        ranked_candidates.sort_by(|a, b| {
            compare_rank(&a.rank_evidence, &b.rank_evidence, &strategy)
                .then_with(|| a.candidate_id.cmp(&b.candidate_id))
        });
    }

    let selected_candidate_ids = ranked_candidates
        .iter()
        .take(config.selection_limit)
        .map(|candidate| candidate.candidate_id.clone())
        .collect::<Vec<_>>();
    let candidates_considered = candidates_sorted
        .iter()
        .map(|candidate| candidate.candidate_id.clone())
        .collect::<Vec<_>>();

    Ok(SelectionReceipt {
        strategy: strategy.clone(),
        selection_tick,
        candidates_considered,
        ranked_candidates,
        selected_candidate_ids,
        rejected_candidates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sam(
        before: Option<u64>,
        after: Option<u64>,
        cost: Option<u64>,
    ) -> SamEvidence {
        SamEvidence {
            disposition: EvidenceDisposition::Admitted,
            evidence_digest: "sam-evidence-a".to_string(),
            checkpoint_digest: "checkpoint-a".to_string(),
            context_digest: "context-a".to_string(),
            observation_digest: "observation-a".to_string(),
            current_expected_loss_microunits: before,
            expected_post_query_loss_microunits: after,
            acquisition_cost_micro_units: cost,
            option_fragility_ppm: Some(500_000),
            recoverability_depth: Some(3),
            residual_decision_ticks: Some(10),
        }
    }

    fn candidate(id: &str) -> Candidate {
        Candidate {
            candidate_id: id.to_string(),
            candidate_digest: format!("candidate-{id}"),
            scenario_digest: format!("scenario-{id}"),
            observation_digest: "observation-a".to_string(),
            checkpoint_digest: "checkpoint-a".to_string(),
            context_digest: "context-a".to_string(),
            scenario_evidence: EvidenceDisposition::Admitted,
            observation_evidence: EvidenceDisposition::Admitted,
            observation_source_role: SourceRole::PolicyVisible,
            observation_available_tick: 10,
            selection_tick: 10,
            family_id: "urban-interaction".to_string(),
            family_seen_count: Some(10),
            entropy_microbits: Some(100),
            prediction_error_ppm: Some(100_000),
            decision_margin_ppm: Some(500_000),
            expected_query_latency_ticks: Some(2),
            decision_deadline_tick: Some(20),
            sam: Some(sam(Some(100), Some(50), Some(10))),
        }
    }

    fn config() -> SelectionConfig {
        SelectionConfig {
            maximum_observation_age_ticks: 5,
            actuation_reserve_ticks: 3,
            maximum_acquisition_cost_micro_units: 100,
            selection_limit: 2,
        }
    }

    fn ids(receipt: &SelectionReceipt) -> Vec<&str> {
        receipt
            .ranked_candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect()
    }

    #[test]
    fn sam_value_ranks_larger_loss_reduction_per_cost_first() {
        let mut low = candidate("low");
        low.sam = Some(sam(Some(100), Some(80), Some(10)));
        let mut high = candidate("high");
        high.sam = Some(sam(Some(100), Some(50), Some(10)));
        let receipt = select(&[low, high], Strategy::SamDecisionValue, config()).unwrap();
        assert_eq!(ids(&receipt), vec!["high", "low"]);
    }

    #[test]
    fn high_entropy_with_zero_decision_value_is_rejected() {
        let mut noisy = candidate("noisy");
        noisy.entropy_microbits = Some(9_000);
        noisy.sam = Some(sam(Some(100), Some(100), Some(1)));
        let receipt = select(&[noisy], Strategy::SamDecisionValue, config()).unwrap();
        assert!(receipt.ranked_candidates.is_empty());
        assert_eq!(
            receipt.rejected_candidates[0].reason,
            RejectionReason::NoPositiveDecisionLossReduction
        );
    }

    #[test]
    fn lower_entropy_with_higher_decision_value_can_rank_first() {
        let mut high_entropy = candidate("entropy");
        high_entropy.entropy_microbits = Some(9_000);
        high_entropy.sam = Some(sam(Some(100), Some(95), Some(10)));
        let mut decision = candidate("decision");
        decision.entropy_microbits = Some(10);
        decision.sam = Some(sam(Some(100), Some(40), Some(10)));
        let receipt = select(&[high_entropy, decision], Strategy::SamDecisionValue, config()).unwrap();
        assert_eq!(ids(&receipt), vec!["decision", "entropy"]);
    }

    #[test]
    fn rejects_query_that_misses_deadline() {
        let mut late = candidate("late");
        late.expected_query_latency_ticks = Some(8);
        late.decision_deadline_tick = Some(20);
        let receipt = select(&[late], Strategy::SamDecisionValue, config()).unwrap();
        assert_eq!(receipt.rejected_candidates[0].reason, RejectionReason::QueryMissesDeadline);
    }

    #[test]
    fn preserves_actuation_reserve_at_exact_boundary() {
        let mut exact = candidate("exact");
        exact.expected_query_latency_ticks = Some(7);
        exact.decision_deadline_tick = Some(20);
        let receipt = select(&[exact], Strategy::SamDecisionValue, config()).unwrap();
        assert_eq!(receipt.ranked_candidates.len(), 1);
        assert_eq!(
            receipt.ranked_candidates[0].rank_evidence,
            RankEvidence::SamDecisionValue {
                regret_reduction_microunits: 50,
                acquisition_cost_micro_units: 10,
                query_arrival_tick: 17,
                decision_deadline_tick: 20,
                actuation_reserve_ticks: 3,
            }
        );
    }

    #[test]
    fn rejects_query_that_arrives_on_deadline_without_reserve() {
        let mut late = candidate("late");
        late.expected_query_latency_ticks = Some(10);
        late.decision_deadline_tick = Some(20);
        let receipt = select(&[late], Strategy::SamDecisionValue, config()).unwrap();
        assert_eq!(receipt.rejected_candidates[0].reason, RejectionReason::QueryMissesDeadline);
    }

    #[test]
    fn rejects_missing_stale_or_rejected_sam_evidence() {
        for disposition in [EvidenceDisposition::Missing, EvidenceDisposition::Stale, EvidenceDisposition::Rejected] {
            let mut item = candidate("candidate");
            item.sam.as_mut().unwrap().disposition = disposition;
            let receipt = select(&[item], Strategy::SamDecisionValue, config()).unwrap();
            assert_eq!(receipt.rejected_candidates[0].reason, RejectionReason::SamEvidenceNotAdmitted);
        }
    }

    #[test]
    fn rejects_sam_context_mismatch() {
        let mut item = candidate("candidate");
        item.sam.as_mut().unwrap().context_digest = "different-context".to_string();
        let receipt = select(&[item], Strategy::SamDecisionValue, config()).unwrap();
        assert_eq!(receipt.rejected_candidates[0].reason, RejectionReason::SamContextMismatch);
    }

    #[test]
    fn rejects_post_query_loss_not_lower_than_current_loss() {
        for after in [100, 120] {
            let mut item = candidate("candidate");
            item.sam = Some(sam(Some(100), Some(after), Some(10)));
            let receipt = select(&[item], Strategy::SamDecisionValue, config()).unwrap();
            assert_eq!(receipt.rejected_candidates[0].reason, RejectionReason::NoPositiveDecisionLossReduction);
        }
    }

    #[test]
    fn rejects_zero_or_missing_acquisition_cost() {
        for cost in [Some(0), None] {
            let mut item = candidate("candidate");
            item.sam = Some(sam(Some(100), Some(50), cost));
            let receipt = select(&[item], Strategy::SamDecisionValue, config()).unwrap();
            assert_eq!(
                receipt.rejected_candidates[0].reason,
                if cost == Some(0) {
                    RejectionReason::AcquisitionCostZero
                } else {
                    RejectionReason::AcquisitionCostMissing
                }
            );
        }
    }

    #[test]
    fn rejects_acquisition_cost_over_bound() {
        let mut item = candidate("candidate");
        item.sam = Some(sam(Some(100), Some(50), Some(101)));
        let receipt = select(&[item], Strategy::SamDecisionValue, config()).unwrap();
        assert_eq!(receipt.rejected_candidates[0].reason, RejectionReason::AcquisitionCostOverLimit);
    }

    #[test]
    fn rejects_timing_overflow() {
        let mut item = candidate("overflow");
        item.selection_tick = u64::MAX;
        item.observation_available_tick = u64::MAX;
        item.expected_query_latency_ticks = Some(1);
        item.decision_deadline_tick = Some(u64::MAX);
        let receipt = select(&[item], Strategy::SamDecisionValue, config()).unwrap();
        assert_eq!(receipt.rejected_candidates[0].reason, RejectionReason::TimingOverflow);
    }

    #[test]
    fn rejects_evaluator_only_and_future_observations() {
        let mut evaluator = candidate("evaluator");
        evaluator.observation_source_role = SourceRole::EvaluatorOnly;
        let mut future = candidate("future");
        future.observation_available_tick = 11;
        let receipt = select(&[evaluator, future], Strategy::Entropy, config()).unwrap();
        assert_eq!(receipt.rejected_candidates.len(), 2);
        assert!(receipt.rejected_candidates.iter().any(|item| item.reason == RejectionReason::ObservationNotPolicyVisible));
        assert!(receipt.rejected_candidates.iter().any(|item| item.reason == RejectionReason::ObservationFromFuture));
    }

    #[test]
    fn rejects_stale_observation() {
        let mut stale = candidate("stale");
        stale.observation_available_tick = 1;
        let receipt = select(&[stale], Strategy::Entropy, config()).unwrap();
        assert_eq!(receipt.rejected_candidates[0].reason, RejectionReason::ObservationTooOld);
    }

    #[test]
    fn baseline_rankings_match_declared_order() {
        let mut a = candidate("a");
        let mut b = candidate("b");
        let mut c = candidate("c");
        a.family_seen_count = Some(10);
        b.family_seen_count = Some(1);
        c.family_seen_count = Some(5);
        assert_eq!(
            ids(&select(&[a.clone(), b.clone(), c.clone()], Strategy::FamilyRarity, config()).unwrap()),
            vec!["b", "c", "a"]
        );

        a.entropy_microbits = Some(10);
        b.entropy_microbits = Some(100);
        c.entropy_microbits = Some(50);
        assert_eq!(
            ids(&select(&[a.clone(), b.clone(), c.clone()], Strategy::Entropy, config()).unwrap()),
            vec!["b", "c", "a"]
        );

        a.prediction_error_ppm = Some(10);
        b.prediction_error_ppm = Some(100);
        c.prediction_error_ppm = Some(50);
        assert_eq!(
            ids(&select(&[a.clone(), b.clone(), c.clone()], Strategy::PredictionError, config()).unwrap()),
            vec!["b", "c", "a"]
        );

        a.decision_margin_ppm = Some(900_000);
        b.decision_margin_ppm = Some(100_000);
        c.decision_margin_ppm = Some(500_000);
        assert_eq!(
            ids(&select(&[a, b, c], Strategy::DecisionMargin, config()).unwrap()),
            vec!["b", "c", "a"]
        );
    }

    #[test]
    fn recoverability_is_lexicographic_not_a_weighted_sum() {
        let mut low = candidate("low");
        low.sam.as_mut().unwrap().option_fragility_ppm = Some(500_000);
        low.sam.as_mut().unwrap().recoverability_depth = Some(1);
        low.sam.as_mut().unwrap().residual_decision_ticks = Some(1);

        let mut high = candidate("high");
        high.sam.as_mut().unwrap().option_fragility_ppm = Some(900_000);
        high.sam.as_mut().unwrap().recoverability_depth = Some(10);
        high.sam.as_mut().unwrap().residual_decision_ticks = Some(100);

        let receipt = select(&[low, high], Strategy::SamRecoverability, config()).unwrap();
        assert_eq!(ids(&receipt), vec!["high", "low"]);
    }

    #[test]
    fn uniform_is_a_deterministic_cyclic_order() {
        let a = candidate("a");
        let b = candidate("b");
        let c = candidate("c");
        let receipt = select(
            &[a, b, c],
            Strategy::UniformCyclic { cursor_after_id: Some("a".to_string()) },
            config(),
        )
        .unwrap();
        assert_eq!(ids(&receipt), vec!["b", "c", "a"]);
    }

    #[test]
    fn ties_and_input_permutation_do_not_change_selection_receipt() {
        let mut a = candidate("a");
        let mut b = candidate("b");
        a.entropy_microbits = Some(100);
        b.entropy_microbits = Some(100);
        let first = select(&[a.clone(), b.clone()], Strategy::Entropy, config()).unwrap();
        let second = select(&[b, a], Strategy::Entropy, config()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_candidate_ids_reject_campaign() {
        let item = candidate("same");
        let error = select(&[item.clone(), item], Strategy::Entropy, config()).unwrap_err();
        assert_eq!(error, SelectionError::DuplicateCandidateId("same".to_string()));
    }

    #[test]
    fn no_metric_is_not_silently_interpreted_as_zero() {
        let mut item = candidate("missing");
        item.entropy_microbits = None;
        let receipt = select(&[item], Strategy::Entropy, config()).unwrap();
        assert_eq!(receipt.rejected_candidates[0].reason, RejectionReason::MissingStrategyMetric);
    }

    #[test]
    fn rank_uses_exact_rational_comparison_without_float_rounding() {
        let mut a = candidate("a");
        a.sam = Some(sam(Some(u64::MAX), Some(0), Some(u64::MAX)));
        let mut b = candidate("b");
        b.sam = Some(sam(Some(u64::MAX), Some(1), Some(u64::MAX)));
        let receipt = select(&[a, b], Strategy::SamDecisionValue, config()).unwrap();
        assert_eq!(ids(&receipt), vec!["a", "b"]);
    }

    #[test]
    fn mixed_selection_ticks_reject_campaign() {
        let a = candidate("a");
        let mut b = candidate("b");
        b.selection_tick = 11;
        assert_eq!(select(&[a, b], Strategy::Entropy, config()), Err(SelectionError::MixedSelectionTicks));
    }

    #[test]
    fn rejects_metric_outside_declared_ppm_range() {
        let mut item = candidate("bad-metric");
        item.prediction_error_ppm = Some(1_000_001);
        let receipt = select(&[item], Strategy::PredictionError, config()).unwrap();
        assert_eq!(
            receipt.rejected_candidates[0].reason,
            RejectionReason::MetricOutOfRange
        );
    }
}
