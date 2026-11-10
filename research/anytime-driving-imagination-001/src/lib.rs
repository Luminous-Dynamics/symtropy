//! Deadline-aware allocation of inference-time world-model rollouts.
//!
//! The allocator consumes pre-qualified marginal decision-value and latency
//! evidence. It does not execute a world model, estimate true value of
//! information, authenticate receipts, validate SAM, or certify safety.

use std::cmp::Ordering;
use std::collections::BTreeSet;

pub const MAX_OPPORTUNITIES: usize = 64;
pub const MAX_MARGINAL_ROLLOUTS: usize = 128;
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
pub struct SamEvidence {
    pub disposition: EvidenceDisposition,
    pub evidence_digest: String,
    pub checkpoint_digest: String,
    pub context_digest: String,
    pub observation_digest: String,
    pub model_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionOpportunity {
    pub opportunity_id: String,
    pub opportunity_digest: String,
    pub context_digest: String,
    pub checkpoint_digest: String,
    pub observation_digest: String,
    pub model_digest: String,
    pub model_evidence: EvidenceDisposition,
    pub observation_evidence: EvidenceDisposition,
    pub observation_source_role: SourceRole,
    pub observation_available_tick: u64,
    pub selection_context_digest: String,
    pub current_rollout_count: u32,
    pub maximum_rollout_count: u32,
    pub decision_deadline_tick: u64,
    pub actuation_recovery_reserve_ticks: u64,
    pub latency_bound_ticks_per_rollout: u64,
    pub latency_bound_digest: String,
    pub compute_units_per_rollout: u64,
    pub compute_cost_digest: String,
    /// Marginal expected decision-regret reduction for rollout 1, 2, ... .
    /// Values are in caller-defined micro-loss units and must be non-increasing.
    pub marginal_regret_reduction_microunits: Vec<u64>,
    pub sam: Option<SamEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationConfig {
    pub planning_tick: u64,
    pub maximum_observation_age_ticks: u64,
    pub total_compute_budget_units: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RejectionReason {
    EmptyIdentity,
    IdentityTooLong,
    MissingSourceDigest,
    ModelEvidenceNotAdmitted,
    ObservationEvidenceNotAdmitted,
    ObservationNotPolicyVisible,
    ObservationFromFuture,
    ObservationTooOld,
    SamEvidenceNotAdmitted,
    SamContextMismatch,
    SelectionContextMismatch,
    ZeroRolloutLatency,
    ZeroComputeCost,
    InvalidRolloutRange,
    MissingMarginalSchedule,
    TooManyMarginals,
    MarginalValueNotDiminishing,
    MarginalValueOverflow,
    DeadlineAlreadyPassed,
    NoTimeAfterReserve,
    DuplicateOpportunityId,
    TooManyOpportunities,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AllocationStopReason {
    MaxRolloutsReached,
    MarginalScheduleExhausted,
    NoPositiveMarginalValue,
    DeadlineReservePreserved,
    SharedComputeBudgetExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpportunityAllocation {
    pub opportunity_id: String,
    pub opportunity_digest: String,
    pub context_digest: String,
    pub checkpoint_digest: String,
    pub observation_digest: String,
    pub model_digest: String,
    pub latency_bound_digest: String,
    pub compute_cost_digest: String,
    pub sam_evidence_digest: String,
    pub initial_rollout_count: u32,
    pub additional_rollout_count: u32,
    pub final_rollout_count: u32,
    pub expected_regret_reduction_microunits: u64,
    pub compute_units_allocated: u64,
    pub completion_bound_tick: u64,
    pub decision_deadline_tick: u64,
    pub actuation_recovery_reserve_ticks: u64,
    pub stop_reason: AllocationStopReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedOpportunity {
    pub opportunity_id: String,
    pub reason: RejectionReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationReceipt {
    pub planning_tick: u64,
    pub total_compute_budget_units: u64,
    pub compute_units_allocated: u64,
    pub opportunities_considered: Vec<String>,
    pub allocations: Vec<OpportunityAllocation>,
    pub rejected_opportunities: Vec<RejectedOpportunity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocationError {
    DuplicateOpportunityId(String),
    TooManyOpportunities { actual: usize, limit: usize },
}

#[derive(Debug, Clone)]
struct ValidatedOpportunity {
    source: DecisionOpportunity,
    max_additional_rollouts: u32,
}

#[derive(Debug, Clone, Copy)]
struct Choice {
    opportunity_index: usize,
    marginal_gain: u64,
    compute_cost: u64,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_id(value: &str) -> bool {
    nonempty(value) && value.len() <= MAX_ID_BYTES
}

fn valid_digest(value: &str) -> bool {
    nonempty(value) && value.len() <= MAX_DIGEST_BYTES
}

fn validate_opportunity(
    opportunity: &DecisionOpportunity,
    config: AllocationConfig,
) -> Result<u32, RejectionReason> {
    if !valid_id(&opportunity.opportunity_id) {
        return Err(if opportunity.opportunity_id.len() > MAX_ID_BYTES {
            RejectionReason::IdentityTooLong
        } else {
            RejectionReason::EmptyIdentity
        });
    }
    if !valid_digest(&opportunity.opportunity_digest)
        || !valid_digest(&opportunity.context_digest)
        || !valid_digest(&opportunity.checkpoint_digest)
        || !valid_digest(&opportunity.observation_digest)
        || !valid_digest(&opportunity.model_digest)
        || !valid_digest(&opportunity.latency_bound_digest)
        || !valid_digest(&opportunity.compute_cost_digest)
        || !valid_digest(&opportunity.selection_context_digest)
    {
        return Err(RejectionReason::MissingSourceDigest);
    }
    if opportunity.selection_context_digest != opportunity.context_digest {
        return Err(RejectionReason::SelectionContextMismatch);
    }
    if opportunity.model_evidence != EvidenceDisposition::Admitted {
        return Err(RejectionReason::ModelEvidenceNotAdmitted);
    }
    if opportunity.observation_evidence != EvidenceDisposition::Admitted {
        return Err(RejectionReason::ObservationEvidenceNotAdmitted);
    }
    if opportunity.observation_source_role != SourceRole::PolicyVisible {
        return Err(RejectionReason::ObservationNotPolicyVisible);
    }
    if opportunity.observation_available_tick > config.planning_tick {
        return Err(RejectionReason::ObservationFromFuture);
    }
    if config.planning_tick - opportunity.observation_available_tick
        > config.maximum_observation_age_ticks
    {
        return Err(RejectionReason::ObservationTooOld);
    }

    let sam = opportunity
        .sam
        .as_ref()
        .ok_or(RejectionReason::SamEvidenceNotAdmitted)?;
    if sam.disposition != EvidenceDisposition::Admitted {
        return Err(RejectionReason::SamEvidenceNotAdmitted);
    }
    if !valid_digest(&sam.evidence_digest)
        || !valid_digest(&sam.checkpoint_digest)
        || !valid_digest(&sam.context_digest)
        || !valid_digest(&sam.observation_digest)
        || !valid_digest(&sam.model_digest)
        || sam.checkpoint_digest != opportunity.checkpoint_digest
        || sam.context_digest != opportunity.context_digest
        || sam.observation_digest != opportunity.observation_digest
        || sam.model_digest != opportunity.model_digest
    {
        return Err(RejectionReason::SamContextMismatch);
    }

    if opportunity.latency_bound_ticks_per_rollout == 0 {
        return Err(RejectionReason::ZeroRolloutLatency);
    }
    if opportunity.compute_units_per_rollout == 0 {
        return Err(RejectionReason::ZeroComputeCost);
    }
    if opportunity.current_rollout_count > opportunity.maximum_rollout_count {
        return Err(RejectionReason::InvalidRolloutRange);
    }
    if opportunity.maximum_rollout_count > MAX_MARGINAL_ROLLOUTS as u32 {
        return Err(RejectionReason::TooManyMarginals);
    }
    let marginal_len = opportunity.marginal_regret_reduction_microunits.len();
    if marginal_len == 0
        && opportunity.current_rollout_count < opportunity.maximum_rollout_count
    {
        return Err(RejectionReason::MissingMarginalSchedule);
    }
    if marginal_len > MAX_MARGINAL_ROLLOUTS {
        return Err(RejectionReason::TooManyMarginals);
    }
    if opportunity
        .marginal_regret_reduction_microunits
        .windows(2)
        .any(|pair| pair[1] > pair[0])
    {
        return Err(RejectionReason::MarginalValueNotDiminishing);
    }
    opportunity
        .marginal_regret_reduction_microunits
        .iter()
        .try_fold(0_u64, |sum, value| sum.checked_add(*value))
        .ok_or(RejectionReason::MarginalValueOverflow)?;

    let ticks_to_deadline = opportunity
        .decision_deadline_tick
        .checked_sub(config.planning_tick)
        .ok_or(RejectionReason::DeadlineAlreadyPassed)?;
    let available_for_rollouts = ticks_to_deadline
        .checked_sub(opportunity.actuation_recovery_reserve_ticks)
        .ok_or(RejectionReason::NoTimeAfterReserve)?;
    let max_by_time = available_for_rollouts / opportunity.latency_bound_ticks_per_rollout;
    let max_by_count = opportunity.maximum_rollout_count - opportunity.current_rollout_count;
    let max_by_schedule = u32::try_from(marginal_len).unwrap_or(u32::MAX);
    Ok((max_by_time.min(u64::from(max_by_count)) as u32).min(max_by_schedule))
}

fn compare_choices(
    left: &Choice,
    right: &Choice,
    opportunities: &[ValidatedOpportunity],
) -> Ordering {
    let left_product = u128::from(left.marginal_gain) * u128::from(right.compute_cost);
    let right_product = u128::from(right.marginal_gain) * u128::from(left.compute_cost);
    right_product
        .cmp(&left_product)
        .then_with(|| {
            opportunities[left.opportunity_index]
                .source
                .opportunity_id
                .cmp(&opportunities[right.opportunity_index].source.opportunity_id)
        })
}

fn stop_reason(
    opportunity: &ValidatedOpportunity,
    allocated: u32,
    planning_tick: u64,
    compute_budget_exhausted: bool,
) -> AllocationStopReason {
    if allocated >= opportunity.max_additional_rollouts {
        if opportunity.source.current_rollout_count + allocated
            >= opportunity.source.maximum_rollout_count
        {
            AllocationStopReason::MaxRolloutsReached
        } else if opportunity.source.marginal_regret_reduction_microunits.len() as u32
            <= allocated
        {
            AllocationStopReason::MarginalScheduleExhausted
        } else {
            AllocationStopReason::DeadlineReservePreserved
        }
    } else if compute_budget_exhausted {
        AllocationStopReason::SharedComputeBudgetExhausted
    } else if opportunity
        .source
        .marginal_regret_reduction_microunits
        .get(allocated as usize)
        .copied()
        .unwrap_or(0)
        == 0
    {
        AllocationStopReason::NoPositiveMarginalValue
    } else if planning_tick
        .checked_add(
            u64::from(allocated + 1)
                .saturating_mul(opportunity.source.latency_bound_ticks_per_rollout),
        )
        .and_then(|arrival| arrival.checked_add(opportunity.source.actuation_recovery_reserve_ticks))
        .is_none_or(|arrival| arrival > opportunity.source.decision_deadline_tick)
    {
        AllocationStopReason::DeadlineReservePreserved
    } else {
        AllocationStopReason::SharedComputeBudgetExhausted
    }
}

/// Greedily allocate each next rollout by exact marginal decision value per
/// compute unit, subject to each opportunity's rollout-count and time bounds.
///
/// This is a deterministic priority heuristic, not a globally optimal knapsack
/// solver. It preserves every declared actuation/recovery reserve.
pub fn allocate(
    opportunities: &[DecisionOpportunity],
    config: AllocationConfig,
) -> Result<AllocationReceipt, AllocationError> {
    if opportunities.len() > MAX_OPPORTUNITIES {
        return Err(AllocationError::TooManyOpportunities {
            actual: opportunities.len(),
            limit: MAX_OPPORTUNITIES,
        });
    }
    let mut seen = BTreeSet::new();
    for opportunity in opportunities {
        if !seen.insert(opportunity.opportunity_id.clone()) {
            return Err(AllocationError::DuplicateOpportunityId(
                opportunity.opportunity_id.clone(),
            ));
        }
    }

    let mut ordered = opportunities.iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| a.opportunity_id.cmp(&b.opportunity_id));

    let mut validated = Vec::new();
    let mut rejected = Vec::new();
    for opportunity in &ordered {
        match validate_opportunity(opportunity, config) {
            Ok(max_additional_rollouts) => validated.push(ValidatedOpportunity {
                source: (*opportunity).clone(),
                max_additional_rollouts,
            }),
            Err(reason) => rejected.push(RejectedOpportunity {
                opportunity_id: opportunity.opportunity_id.clone(),
                reason,
            }),
        }
    }

    let mut allocated = vec![0_u32; validated.len()];
    let mut compute_used = 0_u64;
    let mut gain_by_opportunity = vec![0_u64; validated.len()];

    loop {
        let remaining_budget = config.total_compute_budget_units.checked_sub(compute_used).unwrap_or(0);
        if remaining_budget == 0 {
            break;
        }
        let mut choices = Vec::new();
        for (index, item) in validated.iter().enumerate() {
            if allocated[index] >= item.max_additional_rollouts {
                continue;
            }
            let marginal_index = allocated[index] as usize;
            let marginal_gain = item.source.marginal_regret_reduction_microunits[marginal_index];
            if marginal_gain == 0 || item.source.compute_units_per_rollout > remaining_budget {
                continue;
            }
            choices.push(Choice {
                opportunity_index: index,
                marginal_gain,
                compute_cost: item.source.compute_units_per_rollout,
            });
        }
        if choices.is_empty() {
            break;
        }
        let Some(choice) = choices
            .iter()
            .min_by(|a, b| compare_choices(a, b, &validated))
            .copied()
        else {
            break;
        };
        let index = choice.opportunity_index;
        let item = &validated[index];
        allocated[index] += 1;
        compute_used = compute_used
            .checked_add(choice.compute_cost)
            .expect("cost is bounded by remaining budget");
        gain_by_opportunity[index] = gain_by_opportunity[index]
            .checked_add(choice.marginal_gain)
            .expect("marginal value sum is validated before allocation");
    }

    let mut allocations = Vec::new();
    for (index, item) in validated.iter().enumerate() {
        let source = &item.source;
        let additional = allocated[index];
        let elapsed = u64::from(additional)
            .checked_mul(source.latency_bound_ticks_per_rollout)
            .expect("validated rollout count and latency bound");
        let completion = config
            .planning_tick
            .checked_add(elapsed)
            .expect("deadline gate prevents overflow");
        let cost = u64::from(additional)
            .checked_mul(source.compute_units_per_rollout)
            .expect("compute budget gate prevents overflow");
        let schedule_or_time_capped = additional >= item.max_additional_rollouts;
        let remaining_budget = config.total_compute_budget_units
            .checked_sub(compute_used)
            .unwrap_or(0);
        let budget_exhausted = remaining_budget < source.compute_units_per_rollout;
        allocations.push(OpportunityAllocation {
            opportunity_id: source.opportunity_id.clone(),
            opportunity_digest: source.opportunity_digest.clone(),
            context_digest: source.context_digest.clone(),
            checkpoint_digest: source.checkpoint_digest.clone(),
            observation_digest: source.observation_digest.clone(),
            model_digest: source.model_digest.clone(),
            latency_bound_digest: source.latency_bound_digest.clone(),
            compute_cost_digest: source.compute_cost_digest.clone(),
            sam_evidence_digest: source.sam.as_ref().expect("validated SAM evidence").evidence_digest.clone(),
            initial_rollout_count: source.current_rollout_count,
            additional_rollout_count: additional,
            final_rollout_count: source.current_rollout_count + additional,
            expected_regret_reduction_microunits: gain_by_opportunity[index],
            compute_units_allocated: cost,
            completion_bound_tick: completion,
            decision_deadline_tick: source.decision_deadline_tick,
            actuation_recovery_reserve_ticks: source.actuation_recovery_reserve_ticks,
            stop_reason: stop_reason(
                item,
                additional,
                config.planning_tick,
                budget_exhausted && !schedule_or_time_capped,
            ),
        });
    }

    allocations.sort_by(|a, b| a.opportunity_id.cmp(&b.opportunity_id));
    rejected.sort_by(|a, b| a.opportunity_id.cmp(&b.opportunity_id));
    Ok(AllocationReceipt {
        planning_tick: config.planning_tick,
        total_compute_budget_units: config.total_compute_budget_units,
        compute_units_allocated: compute_used,
        opportunities_considered: ordered.iter().map(|item| item.opportunity_id.clone()).collect(),
        allocations,
        rejected_opportunities: rejected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sam() -> SamEvidence {
        SamEvidence {
            disposition: EvidenceDisposition::Admitted,
            evidence_digest: "sam-evidence".to_string(),
            checkpoint_digest: "checkpoint-1".to_string(),
            context_digest: "context-1".to_string(),
            observation_digest: "observation-1".to_string(),
            model_digest: "model-1".to_string(),
        }
    }

    fn opportunity(id: &str, gains: Vec<u64>) -> DecisionOpportunity {
        DecisionOpportunity {
            opportunity_id: id.to_string(),
            opportunity_digest: format!("opportunity-{id}"),
            context_digest: "context-1".to_string(),
            checkpoint_digest: "checkpoint-1".to_string(),
            observation_digest: "observation-1".to_string(),
            model_digest: "model-1".to_string(),
            model_evidence: EvidenceDisposition::Admitted,
            observation_evidence: EvidenceDisposition::Admitted,
            observation_source_role: SourceRole::PolicyVisible,
            observation_available_tick: 100,
            selection_context_digest: "context-1".to_string(),
            current_rollout_count: 1,
            maximum_rollout_count: 5,
            decision_deadline_tick: 120,
            actuation_recovery_reserve_ticks: 4,
            latency_bound_ticks_per_rollout: 2,
            latency_bound_digest: "latency-bound-v1".to_string(),
            compute_units_per_rollout: 3,
            compute_cost_digest: "compute-cost-v1".to_string(),
            marginal_regret_reduction_microunits: gains,
            sam: Some(sam()),
        }
    }

    fn config(budget: u64) -> AllocationConfig {
        AllocationConfig {
            planning_tick: 100,
            maximum_observation_age_ticks: 5,
            total_compute_budget_units: budget,
        }
    }

    fn allocation<'a>(receipt: &'a AllocationReceipt, id: &str) -> &'a OpportunityAllocation {
        receipt.allocations.iter().find(|item| item.opportunity_id == id).unwrap()
    }

    #[test]
    fn allocates_compute_to_positive_marginal_value() {
        let receipt = allocate(&[opportunity("car-a", vec![100, 80, 60, 0])], config(6)).unwrap();
        let item = allocation(&receipt, "car-a");
        assert_eq!(item.additional_rollout_count, 2);
        assert_eq!(item.expected_regret_reduction_microunits, 180);
        assert_eq!(item.compute_units_allocated, 6);
        assert_eq!(item.completion_bound_tick, 104);
    }

    #[test]
    fn shared_budget_prefers_highest_marginal_value_per_compute() {
        let mut expensive = opportunity("expensive", vec![100, 80]);
        expensive.compute_units_per_rollout = 5;
        let cheap = opportunity("cheap", vec![60, 50]);
        let receipt = allocate(&[expensive, cheap], config(5)).unwrap();
        assert_eq!(allocation(&receipt, "cheap").additional_rollout_count, 1);
        assert_eq!(allocation(&receipt, "expensive").additional_rollout_count, 0);
    }

    #[test]
    fn deadline_gate_preserves_actuation_reserve() {
        let mut item = opportunity("tight", vec![100, 80, 60]);
        item.decision_deadline_tick = 108;
        item.actuation_recovery_reserve_ticks = 4;
        let receipt = allocate(&[item], config(100)).unwrap();
        let result = allocation(&receipt, "tight");
        assert_eq!(result.additional_rollout_count, 2);
        assert_eq!(result.completion_bound_tick, 104);
        assert_eq!(result.completion_bound_tick + result.actuation_recovery_reserve_ticks, 108);
    }

    #[test]
    fn time_too_short_allocates_no_rollouts() {
        let mut item = opportunity("no-time", vec![100]);
        item.decision_deadline_tick = 105;
        let receipt = allocate(&[item], config(10)).unwrap();
        assert_eq!(allocation(&receipt, "no-time").additional_rollout_count, 0);
    }

    #[test]
    fn stops_at_zero_marginal_gain() {
        let receipt = allocate(&[opportunity("zero", vec![10, 0, 0])], config(100)).unwrap();
        assert_eq!(allocation(&receipt, "zero").additional_rollout_count, 1);
        assert_eq!(allocation(&receipt, "zero").stop_reason, AllocationStopReason::NoPositiveMarginalValue);
    }

    #[test]
    fn rejects_non_diminishing_marginal_schedule() {
        let item = opportunity("bad-schedule", vec![10, 20]);
        let receipt = allocate(&[item], config(10)).unwrap();
        assert_eq!(receipt.rejected_opportunities[0].reason, RejectionReason::MarginalValueNotDiminishing);
    }

    #[test]
    fn rejects_marginal_value_sum_overflow() {
        let item = opportunity("overflow", vec![u64::MAX, u64::MAX]);
        let receipt = allocate(&[item], config(10)).unwrap();
        assert_eq!(receipt.rejected_opportunities[0].reason, RejectionReason::MarginalValueOverflow);
    }

    #[test]
    fn rejects_missing_or_stale_sam_evidence() {
        for disposition in [EvidenceDisposition::Missing, EvidenceDisposition::Stale, EvidenceDisposition::Rejected] {
            let mut item = opportunity("bad-sam", vec![10]);
            item.sam.as_mut().unwrap().disposition = disposition;
            let receipt = allocate(&[item], config(10)).unwrap();
            assert_eq!(receipt.rejected_opportunities[0].reason, RejectionReason::SamEvidenceNotAdmitted);
        }
    }

    #[test]
    fn rejects_sam_context_mismatch() {
        let mut item = opportunity("bad-context", vec![10]);
        item.sam.as_mut().unwrap().observation_digest = "other-observation".to_string();
        let receipt = allocate(&[item], config(10)).unwrap();
        assert_eq!(receipt.rejected_opportunities[0].reason, RejectionReason::SamContextMismatch);
    }

    #[test]
    fn rejects_model_evidence_not_admitted() {
        let mut item = opportunity("bad-model", vec![10]);
        item.model_evidence = EvidenceDisposition::Stale;
        let receipt = allocate(&[item], config(10)).unwrap();
        assert_eq!(receipt.rejected_opportunities[0].reason, RejectionReason::ModelEvidenceNotAdmitted);
    }

    #[test]
    fn rejects_non_policy_visible_observation() {
        let mut item = opportunity("hidden", vec![10]);
        item.observation_source_role = SourceRole::EvaluatorOnly;
        let receipt = allocate(&[item], config(10)).unwrap();
        assert_eq!(receipt.rejected_opportunities[0].reason, RejectionReason::ObservationNotPolicyVisible);
    }

    #[test]
    fn rejects_future_observation() {
        let mut item = opportunity("future", vec![10]);
        item.observation_available_tick = 101;
        let receipt = allocate(&[item], config(10)).unwrap();
        assert_eq!(receipt.rejected_opportunities[0].reason, RejectionReason::ObservationFromFuture);
    }

    #[test]
    fn rejects_stale_observation() {
        let mut item = opportunity("stale", vec![10]);
        item.observation_available_tick = 90;
        let receipt = allocate(&[item], config(10)).unwrap();
        assert_eq!(receipt.rejected_opportunities[0].reason, RejectionReason::ObservationTooOld);
    }

    #[test]
    fn rejects_missing_resource_measurement_identity() {
        let mut latency = opportunity("missing-latency-proof", vec![10]);
        latency.latency_bound_digest.clear();
        let mut compute = opportunity("missing-compute-proof", vec![10]);
        compute.compute_cost_digest.clear();
        let receipt = allocate(&[latency, compute], config(10)).unwrap();
        assert_eq!(receipt.rejected_opportunities.len(), 2);
        assert!(receipt.rejected_opportunities.iter().all(|item| item.reason == RejectionReason::MissingSourceDigest));
    }

    #[test]
    fn rejects_zero_rollout_latency_or_compute_cost() {
        let mut latency = opportunity("latency", vec![10]);
        latency.latency_bound_ticks_per_rollout = 0;
        let mut compute = opportunity("compute", vec![10]);
        compute.compute_units_per_rollout = 0;
        let receipt = allocate(&[latency, compute], config(10)).unwrap();
        assert!(receipt.rejected_opportunities.iter().any(|item| item.reason == RejectionReason::ZeroRolloutLatency));
        assert!(receipt.rejected_opportunities.iter().any(|item| item.reason == RejectionReason::ZeroComputeCost));
    }

    #[test]
    fn rejects_invalid_rollout_range() {
        let mut item = opportunity("bad-range", vec![10]);
        item.current_rollout_count = 6;
        let receipt = allocate(&[item], config(10)).unwrap();
        assert_eq!(receipt.rejected_opportunities[0].reason, RejectionReason::InvalidRolloutRange);
    }

    #[test]
    fn rejects_empty_marginal_schedule_when_more_rollouts_are_allowed() {
        let item = opportunity("empty", vec![]);
        let receipt = allocate(&[item], config(10)).unwrap();
        assert_eq!(receipt.rejected_opportunities[0].reason, RejectionReason::MissingMarginalSchedule);
    }

    #[test]
    fn stable_output_under_input_permutation() {
        let a = opportunity("a", vec![100, 50]);
        let b = opportunity("b", vec![100, 50]);
        let first = allocate(&[a.clone(), b.clone()], config(6)).unwrap();
        let second = allocate(&[b, a], config(6)).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_ids_reject_campaign() {
        let a = opportunity("same", vec![100]);
        let b = opportunity("same", vec![100]);
        assert_eq!(
            allocate(&[a, b], config(10)),
            Err(AllocationError::DuplicateOpportunityId("same".to_string()))
        );
    }

    #[test]
    fn exact_rational_comparison_handles_large_integers() {
        let mut a = opportunity("a", vec![u64::MAX]);
        a.compute_units_per_rollout = u64::MAX;
        let mut b = opportunity("b", vec![u64::MAX - 1]);
        b.compute_units_per_rollout = u64::MAX;
        let receipt = allocate(&[a, b], config(u64::MAX)).unwrap();
        assert_eq!(allocation(&receipt, "a").additional_rollout_count, 1);
        assert_eq!(allocation(&receipt, "b").additional_rollout_count, 0);
    }

    #[test]
    fn rejects_deadline_before_planning_tick() {
        let mut item = opportunity("past", vec![10]);
        item.decision_deadline_tick = 99;
        let receipt = allocate(&[item], config(10)).unwrap();
        assert_eq!(receipt.rejected_opportunities[0].reason, RejectionReason::DeadlineAlreadyPassed);
    }

    #[test]
    fn zero_budget_preserves_all_rollouts_for_fallback() {
        let receipt = allocate(&[opportunity("zero-budget", vec![100])], config(0)).unwrap();
        assert_eq!(allocation(&receipt, "zero-budget").additional_rollout_count, 0);
        assert_eq!(receipt.compute_units_allocated, 0);
    }

    #[test]
    fn caps_at_maximum_rollout_count() {
        let mut item = opportunity("max", vec![100, 90, 80]);
        item.maximum_rollout_count = 2;
        let receipt = allocate(&[item], config(100)).unwrap();
        assert_eq!(allocation(&receipt, "max").additional_rollout_count, 1);
        assert_eq!(allocation(&receipt, "max").final_rollout_count, 2);
    }
}
