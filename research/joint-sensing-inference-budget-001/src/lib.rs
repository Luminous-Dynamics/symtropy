//! Joint scheduling of camera refresh and inference-time reasoning.
//!
//! This crate coordinates two resource types but delegates rollout allocation
//! to the existing anytime-driving-imagination owner. It never executes a
//! camera, world model, policy action, actuator or safety fallback.

use anytime_driving_imagination::{
    AllocationConfig, AllocationStopReason, DecisionOpportunity, EvidenceDisposition,
    SamEvidence, SourceRole, allocate,
};
use serde::{Deserialize, Serialize};

pub const DECISION_SCHEMA_VERSION: u32 = 1;
pub const SCHEDULER_REVISION: &str = "joint-sensing-inference-budget-v1";
pub const ALLOCATOR_BASE_REVISION: &str = "265c2a1dc82ff94df1a4ff53e97ac4bc42cc4cf5";
pub const MAX_ID_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointBudget {
    pub planning_tick: u64,
    pub decision_deadline_tick: u64,
    pub actuation_recovery_reserve_ticks: u64,
    pub maximum_observation_age_ticks: u64,
    pub remaining_resource_units: u64,
    /// Must be identical for sensor acquisition and inference rollout costs.
    /// Prefer a measured target-hardware unit such as "microjoule-v1".
    pub resource_unit_id: String,
    pub decision_value_unit_id: String,
    pub clock_id: String,
    pub fallback_policy_id: String,
    pub context_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorRefreshOption {
    pub request_id: String,
    pub sensor_profile_id: String,
    pub sensor_profile_digest: String,
    pub source_observation_digest: String,
    pub context_digest: String,
    pub checkpoint_digest: String,
    pub model_digest: String,
    pub sensor_value_evidence_digest: String,
    pub latency_bound_digest: String,
    pub resource_cost_digest: String,
    pub resource_unit_id: String,
    pub decision_value_unit_id: String,
    pub clock_id: String,
    pub expected_regret_reduction_microunits: u64,
    pub resource_cost_units: u64,
    pub acquisition_to_delivery_bound_ticks: u64,
    pub source_role: SourceRole,
    pub evidence_disposition: EvidenceDisposition,
    pub sam: SamEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionAction {
    RequestObservation,
    AllocateInference,
    CommitNow,
    RequireFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionReason {
    StaleObservationRequiresRefresh,
    SensorValuePerResourceUnitHigher,
    InferenceValuePerResourceUnitHigher,
    SensorOnlyEligible,
    InferenceOnlyEligible,
    EqualRatioPrefersInference,
    NoEligiblePositiveResource,
    NoFreshObservationCanArriveBeforeDeadline,
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorGrant {
    pub request_id: String,
    pub sensor_profile_id: String,
    pub sensor_profile_digest: String,
    pub source_observation_digest: String,
    pub sensor_value_evidence_digest: String,
    pub latency_bound_digest: String,
    pub resource_cost_digest: String,
    pub expected_regret_reduction_microunits: u64,
    pub resource_cost_units: u64,
    pub acquisition_to_delivery_bound_ticks: u64,
    pub completion_bound_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceGrant {
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
    pub resource_cost_units: u64,
    pub completion_bound_tick: u64,
    pub stop_reason: AllocatorStopReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorRejection {
    EmptyIdentity,
    IdentityTooLong,
    MissingSourceDigest,
    ObservationContextMismatch,
    SensorEvidenceNotAdmitted,
    SensorNotPolicyVisible,
    SamEvidenceNotAdmitted,
    SamContextMismatch,
    ResourceUnitMismatch,
    ZeroDecisionValue,
    ZeroResourceCost,
    ZeroLatencyBound,
    DeadlineInfeasible,
    ResourceBudgetInsufficient,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JointDecisionReceipt {
    pub schema_version: u32,
    pub scheduler_revision: String,
    pub allocator_base_revision: String,
    pub action: DecisionAction,
    pub reason: DecisionReason,
    pub planning_tick: u64,
    pub decision_deadline_tick: u64,
    pub actuation_recovery_reserve_ticks: u64,
    pub current_observation_age_ticks: u64,
    pub current_observation_digest: String,
    pub context_digest: String,
    pub resource_unit_id: String,
    pub resource_budget_units: u64,
    pub resource_units_committed: u64,
    pub resource_units_remaining: u64,
    pub decision_value_unit_id: String,
    pub clock_id: String,
    pub completion_bound_tick: u64,
    pub deadline_slack_after_reserve_ticks: u64,
    pub sensor_rejection: Option<SensorRejection>,
    pub sensor_grant: Option<SensorGrant>,
    pub inference_grant: Option<InferenceGrant>,
    pub fallback_policy_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    InvalidBudget(&'static str),
    InvalidInferenceContext(&'static str),
    ResourceUnitMismatch,
    DecisionValueUnitMismatch,
    ClockDomainMismatch,
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SchedulerError {}

#[derive(Debug, Clone)]
struct ValidatedSensorOption {
    option: SensorRefreshOption,
    completion_bound_tick: u64,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_id(value: &str) -> bool {
    nonempty(value) && value.len() <= MAX_ID_BYTES
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_budget(budget: &JointBudget) -> Result<(), SchedulerError> {
    if !valid_id(&budget.resource_unit_id) {
        return Err(SchedulerError::InvalidBudget(
            "resource unit identity is missing or too long",
        ));
    }
    if !valid_id(&budget.decision_value_unit_id) {
        return Err(SchedulerError::InvalidBudget(
            "decision-value unit identity is missing or too long",
        ));
    }
    if !valid_id(&budget.clock_id) {
        return Err(SchedulerError::InvalidBudget(
            "clock-domain identity is missing or too long",
        ));
    }
    if !valid_id(&budget.fallback_policy_id) {
        return Err(SchedulerError::InvalidBudget(
            "fallback policy identity is missing or too long",
        ));
    }
    if !valid_sha256(&budget.context_digest) {
        return Err(SchedulerError::InvalidBudget(
            "context digest must be canonical lowercase SHA-256",
        ));
    }
    let time_to_deadline = budget
        .decision_deadline_tick
        .checked_sub(budget.planning_tick)
        .ok_or(SchedulerError::InvalidBudget(
            "decision deadline has already passed",
        ))?;
    if budget.actuation_recovery_reserve_ticks > time_to_deadline {
        return Err(SchedulerError::InvalidBudget(
            "actuation/recovery reserve exceeds remaining decision window",
        ));
    }
    Ok(())
}

fn validate_inference_context(
    budget: &JointBudget,
    opportunity: &DecisionOpportunity,
    resource_unit_id: &str,
    decision_value_unit_id: &str,
    clock_id: &str,
) -> Result<(), SchedulerError> {
    if resource_unit_id != budget.resource_unit_id {
        return Err(SchedulerError::ResourceUnitMismatch);
    }
    if decision_value_unit_id != budget.decision_value_unit_id {
        return Err(SchedulerError::DecisionValueUnitMismatch);
    }
    if clock_id != budget.clock_id {
        return Err(SchedulerError::ClockDomainMismatch);
    }
    if opportunity.context_digest != budget.context_digest {
        return Err(SchedulerError::InvalidInferenceContext(
            "inference and joint scheduler context digests differ",
        ));
    }
    if opportunity.decision_deadline_tick != budget.decision_deadline_tick
        || opportunity.actuation_recovery_reserve_ticks != budget.actuation_recovery_reserve_ticks
    {
        return Err(SchedulerError::InvalidInferenceContext(
            "inference allocator deadline/reserve differs from joint budget",
        ));
    }
    Ok(())
}

fn validate_sensor_option(
    budget: &JointBudget,
    opportunity: &DecisionOpportunity,
    option: &SensorRefreshOption,
) -> Result<ValidatedSensorOption, SensorRejection> {
    for id in [&option.request_id, &option.sensor_profile_id] {
        if !valid_id(id) {
            return Err(if id.len() > MAX_ID_BYTES {
                SensorRejection::IdentityTooLong
            } else {
                SensorRejection::EmptyIdentity
            });
        }
    }
    for digest in [
        option.sensor_profile_digest.as_str(),
        option.source_observation_digest.as_str(),
        option.context_digest.as_str(),
        option.checkpoint_digest.as_str(),
        option.model_digest.as_str(),
        option.sensor_value_evidence_digest.as_str(),
        option.latency_bound_digest.as_str(),
        option.resource_cost_digest.as_str(),
        option.sam.evidence_digest.as_str(),
        option.sam.checkpoint_digest.as_str(),
        option.sam.context_digest.as_str(),
        option.sam.observation_digest.as_str(),
        option.sam.model_digest.as_str(),
    ] {
        if !valid_sha256(digest) {
            return Err(SensorRejection::MissingSourceDigest);
        }
    }
    if opportunity.observation_available_tick > budget.planning_tick
        || opportunity.observation_evidence == EvidenceDisposition::Rejected
        || opportunity.observation_source_role != SourceRole::PolicyVisible
        || option.source_observation_digest != opportunity.observation_digest
    {
        return Err(SensorRejection::ObservationContextMismatch);
    }
    if option.evidence_disposition != EvidenceDisposition::Admitted {
        return Err(SensorRejection::SensorEvidenceNotAdmitted);
    }
    if option.source_role != SourceRole::PolicyVisible {
        return Err(SensorRejection::SensorNotPolicyVisible);
    }
    if option.sam.disposition != EvidenceDisposition::Admitted {
        return Err(SensorRejection::SamEvidenceNotAdmitted);
    }
    if option.sam.context_digest != opportunity.context_digest
        || option.sam.checkpoint_digest != opportunity.checkpoint_digest
        || option.sam.observation_digest != opportunity.observation_digest
        || option.sam.model_digest != opportunity.model_digest
        || option.context_digest != opportunity.context_digest
        || option.checkpoint_digest != opportunity.checkpoint_digest
        || option.model_digest != opportunity.model_digest
    {
        return Err(SensorRejection::SamContextMismatch);
    }
    if option.resource_unit_id != budget.resource_unit_id {
        return Err(SensorRejection::ResourceUnitMismatch);
    }
    if option.decision_value_unit_id != budget.decision_value_unit_id {
        return Err(SensorRejection::DecisionValueUnitMismatch);
    }
    if option.clock_id != budget.clock_id {
        return Err(SensorRejection::ClockDomainMismatch);
    }
    if option.expected_regret_reduction_microunits == 0 {
        return Err(SensorRejection::ZeroDecisionValue);
    }
    if option.resource_cost_units == 0 {
        return Err(SensorRejection::ZeroResourceCost);
    }
    if option.acquisition_to_delivery_bound_ticks == 0 {
        return Err(SensorRejection::ZeroLatencyBound);
    }
    let completion_bound_tick = budget
        .planning_tick
        .checked_add(option.acquisition_to_delivery_bound_ticks)
        .ok_or(SensorRejection::ArithmeticOverflow)?;
    let reserve_end = completion_bound_tick
        .checked_add(budget.actuation_recovery_reserve_ticks)
        .ok_or(SensorRejection::ArithmeticOverflow)?;
    if reserve_end > budget.decision_deadline_tick {
        return Err(SensorRejection::DeadlineInfeasible);
    }
    if option.resource_cost_units > budget.remaining_resource_units {
        return Err(SensorRejection::ResourceBudgetInsufficient);
    }
    Ok(ValidatedSensorOption {
        option: option.clone(),
        completion_bound_tick,
    })
}

fn next_inference_grant(
    budget: &JointBudget,
    opportunity: &DecisionOpportunity,
) -> Option<InferenceGrant> {
    if opportunity.current_rollout_count >= opportunity.maximum_rollout_count {
        return None;
    }
    let first_gain = opportunity
        .marginal_regret_reduction_microunits
        .first()
        .copied()?;
    if first_gain == 0 {
        return None;
    }
    let mut one_step = opportunity.clone();
    one_step.maximum_rollout_count = one_step.current_rollout_count.checked_add(1)?;
    one_step.marginal_regret_reduction_microunits = vec![first_gain];

    let allocation = allocate(
        &[one_step],
        AllocationConfig {
            planning_tick: budget.planning_tick,
            maximum_observation_age_ticks: budget.maximum_observation_age_ticks,
            total_compute_budget_units: budget.remaining_resource_units,
        },
    )
    .ok()?;
    if !allocation.rejected_opportunities.is_empty() {
        return None;
    }
    let item = allocation.allocations.first()?;
    if item.additional_rollout_count != 1 || item.compute_units_allocated == 0 {
        return None;
    }
    Some(InferenceGrant {
        opportunity_id: item.opportunity_id.clone(),
        opportunity_digest: item.opportunity_digest.clone(),
        context_digest: item.context_digest.clone(),
        checkpoint_digest: item.checkpoint_digest.clone(),
        observation_digest: item.observation_digest.clone(),
        model_digest: item.model_digest.clone(),
        latency_bound_digest: item.latency_bound_digest.clone(),
        compute_cost_digest: item.compute_cost_digest.clone(),
        sam_evidence_digest: item.sam_evidence_digest.clone(),
        initial_rollout_count: item.initial_rollout_count,
        additional_rollout_count: item.additional_rollout_count,
        final_rollout_count: item.final_rollout_count,
        expected_regret_reduction_microunits: item.expected_regret_reduction_microunits,
        resource_cost_units: item.compute_units_allocated,
        completion_bound_tick: item.completion_bound_tick,
    })
}

fn sensor_has_higher_ratio(sensor: &SensorGrant, inference: &InferenceGrant) -> bool {
    u128::from(sensor.expected_regret_reduction_microunits)
        * u128::from(inference.resource_cost_units)
        > u128::from(inference.expected_regret_reduction_microunits)
            * u128::from(sensor.resource_cost_units)
}

fn make_sensor_grant(valid: ValidatedSensorOption) -> SensorGrant {
    SensorGrant {
        request_id: valid.option.request_id,
        sensor_profile_id: valid.option.sensor_profile_id,
        sensor_profile_digest: valid.option.sensor_profile_digest,
        source_observation_digest: valid.option.source_observation_digest,
        sensor_value_evidence_digest: valid.option.sensor_value_evidence_digest,
        latency_bound_digest: valid.option.latency_bound_digest,
        resource_cost_digest: valid.option.resource_cost_digest,
        expected_regret_reduction_microunits: valid.option.expected_regret_reduction_microunits,
        resource_cost_units: valid.option.resource_cost_units,
        acquisition_to_delivery_bound_ticks: valid.option.acquisition_to_delivery_bound_ticks,
        completion_bound_tick: valid.completion_bound_tick,
    }
}

fn deadline_slack(budget: &JointBudget, completion_bound_tick: u64) -> u64 {
    budget
        .decision_deadline_tick
        .saturating_sub(completion_bound_tick)
        .saturating_sub(budget.actuation_recovery_reserve_ticks)
}

/// Decide whether the next shared resource unit should buy a fresh/richer
/// sensor observation or one more inference rollout.
///
/// This function does not invoke the sensor, world model, policy or actuator.
/// A CommitNow result ends this scheduler's resource allocation; the policy
/// and independent runtime-assurance owners still decide the action.
pub fn schedule(
    budget: &JointBudget,
    inference: &DecisionOpportunity,
    inference_resource_unit_id: &str,
    inference_decision_value_unit_id: &str,
    inference_clock_id: &str,
    sensor: Option<&SensorRefreshOption>,
) -> Result<JointDecisionReceipt, SchedulerError> {
    validate_budget(budget)?;
    validate_inference_context(
        budget,
        inference,
        inference_resource_unit_id,
        inference_decision_value_unit_id,
        inference_clock_id,
    )?;

    let observation_available = inference.observation_available_tick <= budget.planning_tick;
    let observation_age = budget
        .planning_tick
        .checked_sub(inference.observation_available_tick)
        .unwrap_or(u64::MAX);
    let observation_stale = !observation_available
        || inference.observation_evidence != EvidenceDisposition::Admitted
        || inference.observation_source_role != SourceRole::PolicyVisible
        || observation_age > budget.maximum_observation_age_ticks;

    let (validated_sensor, sensor_rejection) = match sensor {
        Some(option) => match validate_sensor_option(budget, inference, option) {
            Ok(valid) => (Some(valid), None),
            Err(reason) => (None, Some(reason)),
        },
        None => (None, None),
    };
    let inference_grant = next_inference_grant(budget, inference);

    let (action, reason, sensor_grant, chosen_inference, resource_units_committed, completion_bound_tick, fallback) =
        if observation_stale {
            if let Some(valid) = validated_sensor {
                let grant = make_sensor_grant(valid);
                (
                    DecisionAction::RequestObservation,
                    DecisionReason::StaleObservationRequiresRefresh,
                    Some(grant.clone()),
                    None,
                    grant.resource_cost_units,
                    grant.completion_bound_tick,
                    None,
                )
            } else {
                (
                    DecisionAction::RequireFallback,
                    DecisionReason::NoFreshObservationCanArriveBeforeDeadline,
                    None,
                    None,
                    0,
                    budget.planning_tick,
                    Some(budget.fallback_policy_id.clone()),
                )
            }
        } else {
            match (validated_sensor, inference_grant) {
                (Some(valid), Some(inference)) => {
                    let grant = make_sensor_grant(valid);
                    if sensor_has_higher_ratio(&grant, &inference) {
                        (
                            DecisionAction::RequestObservation,
                            DecisionReason::SensorValuePerResourceUnitHigher,
                            Some(grant.clone()),
                            None,
                            grant.resource_cost_units,
                            grant.completion_bound_tick,
                            None,
                        )
                    } else {
                        let tied =
                            u128::from(grant.expected_regret_reduction_microunits)
                                * u128::from(inference.resource_cost_units)
                                == u128::from(inference.expected_regret_reduction_microunits)
                                    * u128::from(grant.resource_cost_units);
                        (
                            DecisionAction::AllocateInference,
                            if tied {
                                DecisionReason::EqualRatioPrefersInference
                            } else {
                                DecisionReason::InferenceValuePerResourceUnitHigher
                            },
                            None,
                            Some(inference.clone()),
                            inference.resource_cost_units,
                            inference.completion_bound_tick,
                            None,
                        )
                    }
                }
                (Some(valid), None) => {
                    let grant = make_sensor_grant(valid);
                    (
                        DecisionAction::RequestObservation,
                        DecisionReason::SensorOnlyEligible,
                        Some(grant.clone()),
                        None,
                        grant.resource_cost_units,
                        grant.completion_bound_tick,
                        None,
                    )
                }
                (None, Some(inference)) => (
                    DecisionAction::AllocateInference,
                    DecisionReason::InferenceOnlyEligible,
                    None,
                    Some(inference.clone()),
                    inference.resource_cost_units,
                    inference.completion_bound_tick,
                    None,
                ),
                (None, None) => (
                    DecisionAction::CommitNow,
                    DecisionReason::NoEligiblePositiveResource,
                    None,
                    None,
                    0,
                    budget.planning_tick,
                    None,
                ),
            }
        };

    Ok(JointDecisionReceipt {
        schema_version: DECISION_SCHEMA_VERSION,
        scheduler_revision: SCHEDULER_REVISION.to_string(),
        allocator_base_revision: ALLOCATOR_BASE_REVISION.to_string(),
        action,
        reason,
        planning_tick: budget.planning_tick,
        decision_deadline_tick: budget.decision_deadline_tick,
        actuation_recovery_reserve_ticks: budget.actuation_recovery_reserve_ticks,
        current_observation_age_ticks: observation_age,
        current_observation_digest: inference.observation_digest.clone(),
        context_digest: budget.context_digest.clone(),
        resource_unit_id: budget.resource_unit_id.clone(),
        resource_budget_units: budget.remaining_resource_units,
        decision_value_unit_id: budget.decision_value_unit_id.clone(),
        clock_id: budget.clock_id.clone(),
        resource_units_committed,
        resource_units_remaining: budget.remaining_resource_units - resource_units_committed,
        completion_bound_tick,
        deadline_slack_after_reserve_ticks: deadline_slack(budget, completion_bound_tick),
        sensor_rejection,
        sensor_grant,
        inference_grant: chosen_inference,
        fallback_policy_id: fallback,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> String {
        std::iter::repeat_n(format!("{byte:02x}"), 32).collect()
    }

    fn budget() -> JointBudget {
        JointBudget {
            planning_tick: 100,
            decision_deadline_tick: 130,
            actuation_recovery_reserve_ticks: 5,
            maximum_observation_age_ticks: 3,
            remaining_resource_units: 100,
            resource_unit_id: "measured_uj_v1".into(),
            decision_value_unit_id: "micro_loss_v1".into(),
            clock_id: "sim_tick_200hz_v1".into(),
            fallback_policy_id: "fallback-v1".into(),
            context_digest: digest(1),
        }
    }

    fn inference() -> DecisionOpportunity {
        DecisionOpportunity {
            opportunity_id: "decision-1".into(),
            opportunity_digest: digest(2),
            context_digest: digest(1),
            checkpoint_digest: digest(3),
            observation_digest: digest(4),
            model_digest: digest(5),
            model_evidence: EvidenceDisposition::Admitted,
            observation_evidence: EvidenceDisposition::Admitted,
            observation_source_role: SourceRole::PolicyVisible,
            observation_available_tick: 99,
            selection_context_digest: digest(1),
            current_rollout_count: 0,
            maximum_rollout_count: 3,
            decision_deadline_tick: 130,
            actuation_recovery_reserve_ticks: 5,
            latency_bound_ticks_per_rollout: 4,
            latency_bound_digest: digest(6),
            compute_units_per_rollout: 20,
            compute_cost_digest: digest(7),
            marginal_regret_reduction_microunits: vec![100, 50, 25],
            sam: Some(SamEvidence {
                disposition: EvidenceDisposition::Admitted,
                evidence_digest: digest(8),
                checkpoint_digest: digest(3),
                context_digest: digest(1),
                observation_digest: digest(4),
                model_digest: digest(5),
            }),
        }
    }

    fn sensor() -> SensorRefreshOption {
        SensorRefreshOption {
            request_id: "camera-refresh-1".into(),
            sensor_profile_id: "front-1080p".into(),
            sensor_profile_digest: digest(9),
            source_observation_digest: digest(4),
            context_digest: digest(1),
            checkpoint_digest: digest(3),
            model_digest: digest(5),
            sensor_value_evidence_digest: digest(10),
            latency_bound_digest: digest(11),
            resource_cost_digest: digest(12),
            resource_unit_id: "measured_uj_v1".into(),
            decision_value_unit_id: "micro_loss_v1".into(),
            clock_id: "sim_tick_200hz_v1".into(),
            expected_regret_reduction_microunits: 150,
            resource_cost_units: 15,
            acquisition_to_delivery_bound_ticks: 5,
            source_role: SourceRole::PolicyVisible,
            evidence_disposition: EvidenceDisposition::Admitted,
            sam: SamEvidence {
                disposition: EvidenceDisposition::Admitted,
                evidence_digest: digest(13),
                checkpoint_digest: digest(3),
                context_digest: digest(1),
                observation_digest: digest(4),
                model_digest: digest(5),
            },
        }
    }

    #[test]
    fn sensor_wins_when_its_exact_ratio_is_higher() {
        let result = schedule(&budget(), &inference(), "measured_uj_v1", Some(&sensor())).unwrap();
        assert_eq!(result.action, DecisionAction::RequestObservation);
        assert_eq!(result.reason, DecisionReason::SensorValuePerResourceUnitHigher);
        assert_eq!(result.sensor_grant.unwrap().resource_cost_units, 15);
        assert_eq!(result.resource_units_remaining, 85);
    }

    #[test]
    fn inference_wins_when_its_exact_ratio_is_higher() {
        let mut sensor = sensor();
        sensor.expected_regret_reduction_microunits = 20;
        let result = schedule(&budget(), &inference(), "measured_uj_v1", Some(&sensor)).unwrap();
        assert_eq!(result.action, DecisionAction::AllocateInference);
        assert_eq!(result.reason, DecisionReason::InferenceValuePerResourceUnitHigher);
        assert_eq!(result.inference_grant.unwrap().additional_rollout_count, 1);
        assert_eq!(result.resource_units_remaining, 80);
    }

    #[test]
    fn exact_ratio_tie_prefers_existing_inference_allocator() {
        let mut sensor = sensor();
        sensor.expected_regret_reduction_microunits = 75;
        let result = schedule(&budget(), &inference(), "measured_uj_v1", Some(&sensor)).unwrap();
        assert_eq!(result.action, DecisionAction::AllocateInference);
        assert_eq!(result.reason, DecisionReason::EqualRatioPrefersInference);
    }

    #[test]
    fn stale_observation_requires_refresh_before_more_inference() {
        let mut inference = inference();
        inference.observation_available_tick = 90;
        let result = schedule(&budget(), &inference, "measured_uj_v1", Some(&sensor())).unwrap();
        assert_eq!(result.action, DecisionAction::RequestObservation);
        assert_eq!(result.reason, DecisionReason::StaleObservationRequiresRefresh);
        assert!(result.inference_grant.is_none());
    }

    #[test]
    fn policy_visible_missing_frame_event_can_request_a_replacement() {
        let mut inference = inference();
        inference.observation_evidence = EvidenceDisposition::Missing;
        let result = schedule(&budget(), &inference, "measured_uj_v1", Some(&sensor())).unwrap();
        assert_eq!(result.action, DecisionAction::RequestObservation);
        assert_eq!(result.reason, DecisionReason::StaleObservationRequiresRefresh);
    }

    #[test]
    fn stale_observation_with_no_admissible_refresh_requires_fallback() {
        let mut inference = inference();
        inference.observation_available_tick = 90;
        let mut sensor = sensor();
        sensor.sam.disposition = EvidenceDisposition::Rejected;
        let result = schedule(&budget(), &inference, "measured_uj_v1", Some(&sensor)).unwrap();
        assert_eq!(result.action, DecisionAction::RequireFallback);
        assert_eq!(result.fallback_policy_id.as_deref(), Some("fallback-v1"));
    }

    #[test]
    fn sensor_request_that_misses_deadline_is_rejected() {
        let mut sensor = sensor();
        sensor.acquisition_to_delivery_bound_ticks = 26;
        let result = schedule(&budget(), &inference(), "measured_uj_v1", Some(&sensor)).unwrap();
        assert_eq!(result.action, DecisionAction::AllocateInference);
        assert_eq!(result.sensor_rejection, Some(SensorRejection::DeadlineInfeasible));
    }

    #[test]
    fn stale_observation_with_unavailable_sensor_cannot_commit() {
        let mut inference = inference();
        inference.observation_available_tick = 90;
        let result = schedule(&budget(), &inference, "measured_uj_v1", None).unwrap();
        assert_eq!(result.action, DecisionAction::RequireFallback);
        assert_eq!(result.reason, DecisionReason::NoFreshObservationCanArriveBeforeDeadline);
    }

    #[test]
    fn no_positive_resource_value_commits_without_consuming_budget() {
        let mut inference = inference();
        inference.marginal_regret_reduction_microunits = vec![0];
        let result = schedule(&budget(), &inference, "measured_uj_v1", None).unwrap();
        assert_eq!(result.action, DecisionAction::CommitNow);
        assert_eq!(result.resource_units_committed, 0);
        assert_eq!(result.resource_units_remaining, 100);
    }

    #[test]
    fn insufficient_sensor_resource_budget_rejects_sensor_option() {
        let mut budget = budget();
        budget.remaining_resource_units = 10;
        let result = schedule(&budget, &inference(), "measured_uj_v1", Some(&sensor())).unwrap();
        assert_eq!(result.action, DecisionAction::AllocateInference);
        assert_eq!(
            result.sensor_rejection,
            Some(SensorRejection::ResourceBudgetInsufficient)
        );
    }

    #[test]
    fn sensor_resource_unit_mismatch_is_rejected() {
        let mut sensor = sensor();
        sensor.resource_unit_id = "image_bytes_v1".into();
        let result = schedule(&budget(), &inference(), "measured_uj_v1", Some(&sensor)).unwrap();
        assert_eq!(result.action, DecisionAction::AllocateInference);
        assert_eq!(result.sensor_rejection, Some(SensorRejection::ResourceUnitMismatch));
    }

    #[test]
    fn inference_resource_unit_mismatch_is_a_hard_error() {
        assert_eq!(
            schedule(&budget(), &inference(), "flops_v1", Some(&sensor())).err(),
            Some(SchedulerError::ResourceUnitMismatch)
        );
    }

    #[test]
    fn sensor_sam_context_mismatch_is_rejected() {
        let mut sensor = sensor();
        sensor.sam.context_digest = digest(20);
        let result = schedule(&budget(), &inference(), "measured_uj_v1", Some(&sensor)).unwrap();
        assert_eq!(result.action, DecisionAction::AllocateInference);
        assert_eq!(result.sensor_rejection, Some(SensorRejection::SamContextMismatch));
    }

    #[test]
    fn future_sensor_evidence_is_not_admitted_by_role() {
        let mut sensor = sensor();
        sensor.source_role = SourceRole::EvaluatorOnly;
        let result = schedule(&budget(), &inference(), "measured_uj_v1", Some(&sensor)).unwrap();
        assert_eq!(result.action, DecisionAction::AllocateInference);
        assert_eq!(result.sensor_rejection, Some(SensorRejection::SensorNotPolicyVisible));
    }

    #[test]
    fn inference_allocator_rejects_non_policy_observation() {
        let mut inference = inference();
        inference.observation_source_role = SourceRole::EvaluatorOnly;
        let result = schedule(&budget(), &inference, "measured_uj_v1", None).unwrap();
        assert_eq!(result.action, DecisionAction::RequireFallback);
    }

    #[test]
    fn inference_allocator_rejects_missing_sam_evidence() {
        let mut inference = inference();
        inference.sam = None;
        let result = schedule(&budget(), &inference, "measured_uj_v1", None).unwrap();
        assert_eq!(result.action, DecisionAction::CommitNow);
        assert_eq!(result.inference_grant, None);
    }

    #[test]
    fn sensor_refresh_preserves_actuation_reserve() {
        let mut sensor = sensor();
        sensor.acquisition_to_delivery_bound_ticks = 25;
        let result = schedule(&budget(), &inference(), "measured_uj_v1", Some(&sensor)).unwrap();
        assert_eq!(result.action, DecisionAction::RequestObservation);
        assert_eq!(result.deadline_slack_after_reserve_ticks, 0);
        assert_eq!(result.completion_bound_tick + 5, 130);
    }

    #[test]
    fn budget_deadline_inversion_is_rejected() {
        let mut budget = budget();
        budget.decision_deadline_tick = 99;
        assert!(matches!(
            schedule(&budget, &inference(), "measured_uj_v1", None),
            Err(SchedulerError::InvalidBudget(_))
        ));
    }

    #[test]
    fn sensor_only_eligible_is_not_reported_as_a_ratio_win() {
        let mut inference = inference();
        inference.maximum_rollout_count = 0;
        let result = schedule(&budget(), &inference, "measured_uj_v1", Some(&sensor())).unwrap();
        assert_eq!(result.action, DecisionAction::RequestObservation);
        assert_eq!(result.reason, DecisionReason::SensorOnlyEligible);
    }

    #[test]
    fn inference_only_eligible_is_not_reported_as_a_ratio_win() {
        let mut sensor = sensor();
        sensor.evidence_disposition = EvidenceDisposition::Rejected;
        let result = schedule(&budget(), &inference(), "measured_uj_v1", Some(&sensor)).unwrap();
        assert_eq!(result.action, DecisionAction::AllocateInference);
        assert_eq!(result.reason, DecisionReason::InferenceOnlyEligible);
    }

    #[test]
    fn decision_schema_is_valid_json_and_lists_privilege_safe_fields() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/joint-sensing-inference-decision-v1.schema.json"
        ))
        .unwrap();
        assert_eq!(schema["properties"]["schema_version"]["const"], 1);
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|field| field == "resource_unit_id"));
        assert!(required.iter().any(|field| field == "fallback_policy_id"));
        assert!(schema["properties"].get("world_snapshot_sha256").is_none());
    }

    #[test]
    fn decision_receipt_round_trips_as_json() {
        let result = schedule(&budget(), &inference(), "measured_uj_v1", Some(&sensor())).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let decoded: JointDecisionReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, result);
    }

    #[test]
    fn decision_receipt_binds_parent_allocator_revision() {
        let result = schedule(&budget(), &inference(), "measured_uj_v1", None).unwrap();
        assert_eq!(result.allocator_base_revision, ALLOCATOR_BASE_REVISION);
    }

    #[test]
    fn sensor_option_must_reference_the_current_policy_observation() {
        let mut sensor = sensor();
        sensor.source_observation_digest = digest(21);
        let result = schedule(&budget(), &inference(), "measured_uj_v1", Some(&sensor)).unwrap();
        assert_eq!(result.action, DecisionAction::AllocateInference);
        assert_eq!(
            result.sensor_rejection,
            Some(SensorRejection::ObservationContextMismatch)
        );
    }

    #[test]
    fn future_observation_cannot_be_refreshed_from_future_evidence() {
        let mut inference = inference();
        inference.observation_available_tick = 101;
        let result = schedule(&budget(), &inference, "measured_uj_v1", Some(&sensor())).unwrap();
        assert_eq!(result.action, DecisionAction::RequireFallback);
        assert_eq!(
            result.sensor_rejection,
            Some(SensorRejection::ObservationContextMismatch)
        );
    }
}
