// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic multi-tick policy decision traces.
//!
//! The trace runner receives an observation schedule and a policy definition,
//! never a latent timeline or outcome report. Repair requests remain requests;
//! this module does not accept, execute, or authorize interventions.

use crate::{
    ComponentId, SettlementError, sha256_hex,
    observation::{
        KnownComponentCondition, ObservationDisposition, ObservedValue, ObservationSchedule,
        PolicyInformationFrame, build_policy_information_frame,
    },
    policy::{
        BaselineStrategy, DecisionDisposition, PolicyDecision, PolicyDefinition,
        evaluate_baseline_policy, verify_policy_decision_with_policy_and_frame,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const POLICY_TRACE_SCHEMA_VERSION: u32 = 1;
pub const POLICY_TRACE_CLAIM_CEILING: &str =
    "synthetic_observation_bounded_policy_trace_no_intervention_execution";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyStepReceipt {
    pub decision_tick: u64,
    pub input_frame_sha256: String,
    pub decision: PolicyDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyTracePayload {
    pub schema_version: u32,
    pub policy_id: String,
    pub policy_version: u32,
    pub strategy: BaselineStrategy,
    pub horizon_ticks: u64,
    pub claim_ceiling: String,
    pub canonical_policy_sha256: String,
    pub canonical_observation_schedule_sha256: String,
    pub steps: Vec<PolicyStepReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyTrace {
    pub payload: PolicyTracePayload,
    /// SHA-256 over canonical compact JSON of the trace payload. Not a signature.
    pub payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingRepair {
    requested_at_tick: u64,
    action_id: String,
}

pub fn run_policy_trace(
    definition: &PolicyDefinition,
    schedule: &ObservationSchedule,
    horizon_ticks: u64,
) -> Result<PolicyTrace, SettlementError> {
    if horizon_ticks > 10_000 {
        return Err(SettlementError(
            "policy trace horizon exceeds bounded maximum 10000".into(),
        ));
    }
    let canonical_policy = definition.canonicalized();
    let policy_bytes = serde_json::to_vec(&canonical_policy)
        .map_err(|error| SettlementError(format!("serialize policy definition: {error}")))?;
    let canonical_schedule = schedule.canonicalized();
    let schedule_bytes = serde_json::to_vec(&canonical_schedule)
        .map_err(|error| SettlementError(format!("serialize observation schedule: {error}")))?;
    let canonical_policy_sha256 = sha256_hex(&policy_bytes);
    let canonical_observation_schedule_sha256 = sha256_hex(&schedule_bytes);

    let mut pending = BTreeMap::<(ComponentId, u64), PendingRepair>::new();
    let mut steps = Vec::with_capacity((horizon_ticks + 1) as usize);

    for decision_tick in 0..=horizon_ticks {
        let frame = build_policy_information_frame(schedule, decision_tick)?;

        // A newer, current Available reading is the only observation-based
        // signal that clears a pending request. No hidden action completion is
        // assumed. Contradictory current values do not authorize new requests.
        pending.retain(|(component_id, generation), repair| {
            !frame.payload.observations.iter().any(|observation| {
                observation.target_component == *component_id
                    && observation.target_generation == *generation
                    && observation.sample_tick > repair.requested_at_tick
                    && matches!(
                        &observation.disposition,
                        ObservationDisposition::Current {
                            value: ObservedValue::ComponentCondition {
                                condition: KnownComponentCondition::Available
                            }
                        }
                    )
            })
        });

        let mut decision = evaluate_baseline_policy(&canonical_policy, &frame)?;
        let request = match &decision.payload.disposition {
            DecisionDisposition::RepairRequested { request } => Some(request.clone()),
            _ => None,
        };
        if let Some(request) = request {
            let key = (request.target_component.clone(), request.target_generation);
            if let Some(existing) = pending.get(&key) {
                decision = rewrite_as_pending_suppression(
                    decision,
                    request.target_component,
                    request.target_generation,
                    existing.action_id.clone(),
                )?;
            } else {
                pending.insert(
                    key,
                    PendingRepair {
                        requested_at_tick: decision_tick,
                        action_id: request.action_id,
                    },
                );
            }
        }

        steps.push(PolicyStepReceipt {
            decision_tick,
            input_frame_sha256: frame.payload_sha256.clone(),
            decision,
        });
    }

    let payload = PolicyTracePayload {
        schema_version: POLICY_TRACE_SCHEMA_VERSION,
        policy_id: canonical_policy.policy_id,
        policy_version: canonical_policy.policy_version,
        strategy: canonical_policy.strategy,
        horizon_ticks,
        claim_ceiling: POLICY_TRACE_CLAIM_CEILING.into(),
        canonical_policy_sha256,
        canonical_observation_schedule_sha256,
        steps,
    };
    let encoded = serde_json::to_vec(&payload)
        .map_err(|error| SettlementError(format!("serialize policy trace: {error}")))?;
    let trace = PolicyTrace {
        payload,
        payload_sha256: sha256_hex(&encoded),
    };
    verify_policy_trace(&trace, &canonical_policy, &canonical_schedule)?;
    Ok(trace)
}

pub fn verify_policy_trace(
    trace: &PolicyTrace,
    definition: &PolicyDefinition,
    schedule: &ObservationSchedule,
) -> Result<(), SettlementError> {
    let payload = &trace.payload;
    if payload.schema_version != POLICY_TRACE_SCHEMA_VERSION {
        return Err(SettlementError(
            "unsupported policy trace schema version".into(),
        ));
    }
    if payload.claim_ceiling != POLICY_TRACE_CLAIM_CEILING {
        return Err(SettlementError(
            "policy trace claim ceiling is missing or widened".into(),
        ));
    }
    if payload.horizon_ticks > 10_000 {
        return Err(SettlementError(
            "policy trace horizon exceeds bounded maximum 10000".into(),
        ));
    }
    let policy = definition.canonicalized();
    let policy_bytes = serde_json::to_vec(&policy)
        .map_err(|error| SettlementError(format!("serialize policy definition: {error}")))?;
    let canonical_schedule = schedule.canonicalized();
    let schedule_bytes = serde_json::to_vec(&canonical_schedule)
        .map_err(|error| SettlementError(format!("serialize observation schedule: {error}")))?;
    if sha256_hex(&policy_bytes) != payload.canonical_policy_sha256
        || sha256_hex(&schedule_bytes) != payload.canonical_observation_schedule_sha256
    {
        return Err(SettlementError(
            "policy trace does not bind to the supplied policy and observation schedule".into(),
        ));
    }
    if payload.policy_id != policy.policy_id
        || payload.policy_version != policy.policy_version
        || payload.strategy != policy.strategy
    {
        return Err(SettlementError(
            "policy trace identity mismatches the supplied policy".into(),
        ));
    }
    let expected_count = payload
        .horizon_ticks
        .checked_add(1)
        .ok_or_else(|| SettlementError("policy trace horizon overflow".into()))?;
    if u64::try_from(payload.steps.len()).ok() != Some(expected_count) {
        return Err(SettlementError(
            "policy trace does not contain every declared decision tick".into(),
        ));
    }
    let encoded = serde_json::to_vec(payload)
        .map_err(|error| SettlementError(format!("serialize policy trace: {error}")))?;
    if sha256_hex(&encoded) != trace.payload_sha256 {
        return Err(SettlementError("policy trace payload digest mismatch".into()));
    }

    let mut pending = BTreeMap::<(ComponentId, u64), PendingRepair>::new();
    for (index, step) in payload.steps.iter().enumerate() {
        let expected_tick = u64::try_from(index)
            .map_err(|_| SettlementError("policy step index does not fit u64".into()))?;
        if step.decision_tick != expected_tick
            || step.decision.payload.decision_tick != expected_tick
        {
            return Err(SettlementError(format!(
                "policy trace tick mismatch at index {expected_tick}"
            )));
        }
        let frame = build_policy_information_frame(schedule, expected_tick)?;
        if frame.payload_sha256 != step.input_frame_sha256 {
            return Err(SettlementError(format!(
                "policy step {expected_tick} has a mismatched frame digest"
            )));
        }
        let decision = &step.decision;
        if decision.payload.claim_ceiling.is_empty()
            || decision.payload.input_frame_sha256 != frame.payload_sha256
        {
            return Err(SettlementError(format!(
                "policy decision at tick {expected_tick} has invalid frame binding"
            )));
        }

        pending.retain(|(component_id, generation), repair| {
            !frame.payload.observations.iter().any(|observation| {
                observation.target_component == *component_id
                    && observation.target_generation == *generation
                    && observation.sample_tick > repair.requested_at_tick
                    && matches!(
                        &observation.disposition,
                        ObservationDisposition::Current {
                            value: ObservedValue::ComponentCondition {
                                condition: KnownComponentCondition::Available
                            }
                        }
                    )
            })
        });

        let mut expected_decision = evaluate_baseline_policy(&policy, &frame)?;
        let expected_request = match &expected_decision.payload.disposition {
            DecisionDisposition::RepairRequested { request } => Some(request.clone()),
            _ => None,
        };
        if let Some(request) = expected_request {
            let key = (request.target_component.clone(), request.target_generation);
            if let Some(existing) = pending.get(&key) {
                expected_decision = rewrite_as_pending_suppression(
                    expected_decision,
                    request.target_component,
                    request.target_generation,
                    existing.action_id.clone(),
                )?;
            } else {
                pending.insert(
                    key,
                    PendingRepair {
                        requested_at_tick: expected_tick,
                        action_id: request.action_id,
                    },
                );
            }
        }
        if &expected_decision != decision {
            return Err(SettlementError(format!(
                "policy decision at tick {expected_tick} differs from the declared baseline rule"
            )));
        }
        verify_policy_decision_with_policy_and_frame(decision, &frame, &policy)?;
    }
    Ok(())
}

fn rewrite_as_pending_suppression(
    mut decision: PolicyDecision,
    target_component: ComponentId,
    target_generation: u64,
    pending_action_id: String,
) -> Result<PolicyDecision, SettlementError> {
    decision.payload.disposition = DecisionDisposition::RepairSuppressedPending {
        target_component,
        target_generation,
        pending_action_id,
    };
    let encoded = serde_json::to_vec(&decision.payload)
        .map_err(|error| SettlementError(format!("serialize suppressed decision: {error}")))?;
    decision.payload_sha256 = sha256_hex(&encoded);
    Ok(decision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ComponentId,
        observation::{
            OBSERVATION_SCHEMA_VERSION, ObservationRecord, ObservationSchedule,
        },
        policy::{RepairRule, POLICY_SCHEMA_VERSION},
    };

    fn observation(
        id: &str,
        component: &str,
        source: &str,
        sample_tick: u64,
        available_tick: u64,
        condition: KnownComponentCondition,
    ) -> ObservationRecord {
        ObservationRecord {
            observation_id: id.into(),
            source_id: source.into(),
            source_generation: 1,
            target_component: ComponentId::parse(component).unwrap(),
            target_generation: 1,
            sample_tick,
            available_tick,
            freshness_window_ticks: 1,
            disposition: ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition { condition },
            },
        }
    }

    fn schedule(records: Vec<ObservationRecord>) -> ObservationSchedule {
        ObservationSchedule {
            schema_version: OBSERVATION_SCHEMA_VERSION,
            records,
        }
    }

    fn policy() -> PolicyDefinition {
        PolicyDefinition {
            schema_version: POLICY_SCHEMA_VERSION,
            policy_id: "reactive-trace".into(),
            policy_version: 1,
            strategy: BaselineStrategy::ReactiveRepair,
            repair_rules: vec![RepairRule {
                target_component: ComponentId::parse("grid").unwrap(),
                target_generation: 1,
                priority: 10,
                allowed_source_ids: vec!["grid-sensor".into()],
                required_parts_supply_id: "parts-store".into(),
                required_parts_units: 1,
                required_crew_units: 1,
                dispatch_latency_ticks: 1,
                repair_duration_ticks: 2,
            }],
        }
    }

    #[test]
    fn future_event_values_cannot_change_decisions_before_they_arrive() {
        let prefix = observation(
            "grid-available",
            "grid",
            "grid-sensor",
            0,
            0,
            KnownComponentCondition::Available,
        );
        let failed_future = observation(
            "future-grid",
            "grid",
            "grid-sensor",
            4,
            4,
            KnownComponentCondition::Failed,
        );
        let available_future = observation(
            "future-grid",
            "grid",
            "grid-sensor",
            4,
            4,
            KnownComponentCondition::Available,
        );
        let first = run_policy_trace(&policy(), &schedule(vec![prefix.clone(), failed_future]), 2)
            .unwrap();
        let second =
            run_policy_trace(&policy(), &schedule(vec![prefix, available_future]), 2).unwrap();
        for tick in 0..=2 {
            assert_eq!(
                first.payload.steps[tick].decision,
                second.payload.steps[tick].decision
            );
        }
    }

    #[test]
    fn repeated_failure_observation_does_not_spam_duplicate_repair_requests() {
        let failure = observation(
            "grid-failed",
            "grid",
            "grid-sensor",
            1,
            1,
            KnownComponentCondition::Failed,
        );
        let trace = run_policy_trace(&policy(), &schedule(vec![failure]), 3).unwrap();
        assert!(matches!(
            &trace.payload.steps[1].decision.payload.disposition,
            DecisionDisposition::RepairRequested { .. }
        ));
        assert!(matches!(
            &trace.payload.steps[2].decision.payload.disposition,
            DecisionDisposition::RepairSuppressedPending { .. }
        ));
        assert!(matches!(
            &trace.payload.steps[3].decision.payload.disposition,
            DecisionDisposition::NoAction { .. }
        ));
    }

    #[test]
    fn newer_current_available_evidence_clears_pending_request() {
        let failure = observation(
            "grid-failed",
            "grid",
            "grid-sensor",
            1,
            1,
            KnownComponentCondition::Failed,
        );
        let restored = observation(
            "grid-restored",
            "grid",
            "grid-sensor",
            2,
            2,
            KnownComponentCondition::Available,
        );
        let later_failure = observation(
            "grid-failed-again",
            "grid",
            "grid-sensor",
            4,
            4,
            KnownComponentCondition::Failed,
        );
        let trace = run_policy_trace(
            &policy(),
            &schedule(vec![failure, restored, later_failure]),
            5,
        )
        .unwrap();
        assert!(matches!(
            &trace.payload.steps[2].decision.payload.disposition,
            DecisionDisposition::NoAction { .. }
        ));
        assert!(matches!(
            &trace.payload.steps[4].decision.payload.disposition,
            DecisionDisposition::RepairRequested { .. }
        ));
    }

    #[test]
    fn independent_trace_verifier_detects_decision_tampering() {
        let failure = observation(
            "grid-failed",
            "grid",
            "grid-sensor",
            1,
            1,
            KnownComponentCondition::Failed,
        );
        let schedule = schedule(vec![failure]);
        let definition = policy();
        let mut trace = run_policy_trace(&definition, &schedule, 2).unwrap();
        trace.payload.steps[1].decision.payload.decision_tick = 2;
        let encoded = serde_json::to_vec(&trace.payload).unwrap();
        trace.payload_sha256 = sha256_hex(&encoded);
        assert!(verify_policy_trace(&trace, &definition, &schedule).is_err());
    }

    #[test]
    fn policy_trace_verifier_rejects_rehashed_decision_semantic_tampering() {
        let failure = observation(
            "grid-failed",
            "grid",
            "grid-sensor",
            1,
            1,
            KnownComponentCondition::Failed,
        );
        let schedule = schedule(vec![failure]);
        let definition = policy();
        let mut trace = run_policy_trace(&definition, &schedule, 3).unwrap();
        trace.payload.steps[2].decision.payload.disposition =
            DecisionDisposition::NoAction {
                reason_code: "policy_no_intervention".into(),
            };
        let encoded_decision =
            serde_json::to_vec(&trace.payload.steps[2].decision.payload).unwrap();
        trace.payload.steps[2].decision.payload_sha256 = sha256_hex(&encoded_decision);
        let encoded_trace = serde_json::to_vec(&trace.payload).unwrap();
        trace.payload_sha256 = sha256_hex(&encoded_trace);
        assert!(verify_policy_trace(&trace, &definition, &schedule).is_err());
    }

    #[test]
    fn observation_schedule_record_order_does_not_change_canonical_trace() {
        let failed = observation(
            "grid-failed",
            "grid",
            "grid-sensor",
            1,
            1,
            KnownComponentCondition::Failed,
        );
        let available = observation(
            "grid-available",
            "grid",
            "grid-sensor",
            0,
            0,
            KnownComponentCondition::Available,
        );
        let first = run_policy_trace(
            &policy(),
            &schedule(vec![failed.clone(), available.clone()]),
            3,
        )
        .unwrap();
        let second =
            run_policy_trace(&policy(), &schedule(vec![available, failed]), 3).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn altered_observation_schedule_cannot_verify_against_existing_trace() {
        let failure = observation(
            "grid-failed",
            "grid",
            "grid-sensor",
            1,
            1,
            KnownComponentCondition::Failed,
        );
        let original = schedule(vec![failure.clone()]);
        let trace = run_policy_trace(&policy(), &original, 2).unwrap();
        let changed = schedule(vec![observation(
            "grid-failed",
            "grid",
            "grid-sensor",
            1,
            1,
            KnownComponentCondition::Available,
        )]);
        assert!(verify_policy_trace(&trace, &policy(), &changed).is_err());
    }
}
