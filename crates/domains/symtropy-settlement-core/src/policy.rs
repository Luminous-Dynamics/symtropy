// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Transparent synthetic policy baselines.
//!
//! These policies receive only a verified PolicyInformationFrame. They cannot
//! access latent scenario state, future events, timelines or outcome reports.
//! The result is a request receipt, never execution authority.

use super::{
    ComponentId, SettlementError, sha256_hex, valid_identifier,
    observation::{
        KnownComponentCondition, ObservationDisposition, ObservedValue, PolicyInformationFrame,
        verify_policy_information_frame,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const POLICY_SCHEMA_VERSION: u32 = 1;
pub const POLICY_DECISION_CLAIM_CEILING: &str =
    "synthetic_policy_request_only_no_execution_or_actuation_authority";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineStrategy {
    NoIntervention,
    ReactiveRepair,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairRule {
    pub target_component: ComponentId,
    pub target_generation: u64,
    /// Higher values are selected first; component ID and observation ID break ties.
    pub priority: u16,
    /// Only evidence from these exact source IDs may trigger this rule.
    pub allowed_source_ids: Vec<String>,
    pub required_parts_supply_id: String,
    pub required_parts_units: u64,
    pub required_crew_units: u64,
    pub dispatch_latency_ticks: u64,
    pub repair_duration_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDefinition {
    pub schema_version: u32,
    pub policy_id: String,
    pub policy_version: u32,
    pub strategy: BaselineStrategy,
    pub repair_rules: Vec<RepairRule>,
}

impl PolicyDefinition {
    pub(crate) fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        canonical.repair_rules.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.target_component.cmp(&b.target_component))
                .then_with(|| a.target_generation.cmp(&b.target_generation))
        });
        for rule in &mut canonical.repair_rules {
            rule.allowed_source_ids.sort();
        }
        canonical
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairRequest {
    pub action_id: String,
    pub target_component: ComponentId,
    pub target_generation: u64,
    pub trigger_observation_id: String,
    pub trigger_observation_sha256: String,
    pub decision_tick: u64,
    pub dispatch_latency_ticks: u64,
    pub repair_duration_ticks: u64,
    pub not_before_tick: u64,
    pub expected_completion_tick: u64,
    pub required_parts_supply_id: String,
    pub required_parts_units: u64,
    pub required_crew_units: u64,
    pub priority: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionDisposition {
    NoAction {
        reason_code: String,
    },
    RepairRequested {
        request: RepairRequest,
    },
    /// A duplicate request is suppressed while an earlier request for the same
    /// component generation remains unresolved. This is still not actuation.
    RepairSuppressedPending {
        target_component: ComponentId,
        target_generation: u64,
        pending_action_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDecisionPayload {
    pub schema_version: u32,
    pub policy_id: String,
    pub policy_version: u32,
    pub strategy: BaselineStrategy,
    pub decision_tick: u64,
    pub input_frame_sha256: String,
    pub claim_ceiling: String,
    pub disposition: DecisionDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDecision {
    pub payload: PolicyDecisionPayload,
    pub payload_sha256: String,
}

fn has_conflicting_current_component_observation(
    frame: &PolicyInformationFrame,
    candidate: &super::observation::ObservationRecord,
    proposed: KnownComponentCondition,
) -> bool {
    frame.payload.observations.iter().any(|other| {
        if other.observation_id == candidate.observation_id
            || other.target_component != candidate.target_component
            || other.target_generation != candidate.target_generation
        {
            return false;
        }
        match &other.disposition {
            ObservationDisposition::Conflicting { .. } => true,
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition { condition },
            } => *condition != proposed,
            _ => false,
        }
    })
}

pub fn evaluate_baseline_policy(
    definition: &PolicyDefinition,
    frame: &PolicyInformationFrame,
) -> Result<PolicyDecision, SettlementError> {
    verify_policy_information_frame(frame)?;
    let canonical = definition.canonicalized();
    validate_policy_definition(&canonical)?;

    let disposition = match canonical.strategy {
        BaselineStrategy::NoIntervention => DecisionDisposition::NoAction {
            reason_code: "policy_no_intervention".into(),
        },
        BaselineStrategy::ReactiveRepair => {
            let mut candidates = Vec::<(
                &RepairRule,
                &super::observation::ObservationRecord,
            )>::new();
            for observation in &frame.payload.observations {
                let ObservationDisposition::Current {
                    value: ObservedValue::ComponentCondition {
                        condition: KnownComponentCondition::Failed,
                    },
                } = &observation.disposition
                else {
                    continue;
                };
                if has_conflicting_current_component_observation(
                    frame,
                    observation,
                    KnownComponentCondition::Failed,
                ) {
                    continue;
                }

                for rule in &canonical.repair_rules {
                    if rule.target_component == observation.target_component
                        && rule.target_generation == observation.target_generation
                        && rule
                            .allowed_source_ids
                            .iter()
                            .any(|source| source == &observation.source_id)
                    {
                        candidates.push((rule, observation));
                    }
                }
            }
            candidates.sort_by(|(rule_a, observation_a), (rule_b, observation_b)| {
                rule_b
                    .priority
                    .cmp(&rule_a.priority)
                    .then_with(|| rule_a.target_component.cmp(&rule_b.target_component))
                    .then_with(|| observation_a.observation_id.cmp(&observation_b.observation_id))
            });
            // Emit at most one repair request per decision tick. This v1 rule
            // keeps labor/cost arbitration explicit instead of assuming infinite crews.
            if let Some((rule, observation)) = candidates.first() {
                let not_before_tick = frame
                    .payload
                    .decision_tick
                    .checked_add(rule.dispatch_latency_ticks)
                    .ok_or_else(|| SettlementError("repair dispatch tick overflow".into()))?;
                let expected_completion_tick = not_before_tick
                    .checked_add(rule.repair_duration_ticks)
                    .ok_or_else(|| SettlementError("repair completion tick overflow".into()))?;
                let encoded_observation = serde_json::to_vec(observation).map_err(|error| {
                    SettlementError(format!("serialize triggering observation: {error}"))
                })?;
                let trigger_observation_sha256 = sha256_hex(&encoded_observation);
                let action_material = format!(
                    "{}\0{}\0{}\0{}\0{}\0{}",
                    canonical.policy_id,
                    canonical.policy_version,
                    frame.payload.decision_tick,
                    rule.target_component.as_str(),
                    rule.target_generation,
                    observation.observation_id
                );
                let action_digest = sha256_hex(action_material.as_bytes());
                let action_id = format!("action:{}", &action_digest[..32]);
                DecisionDisposition::RepairRequested {
                    request: RepairRequest {
                        action_id,
                        target_component: rule.target_component.clone(),
                        target_generation: rule.target_generation,
                        trigger_observation_id: observation.observation_id.clone(),
                        trigger_observation_sha256,
                        decision_tick: frame.payload.decision_tick,
                        dispatch_latency_ticks: rule.dispatch_latency_ticks,
                        repair_duration_ticks: rule.repair_duration_ticks,
                        not_before_tick,
                        expected_completion_tick,
                        required_parts_supply_id: rule.required_parts_supply_id.clone(),
                        required_parts_units: rule.required_parts_units,
                        required_crew_units: rule.required_crew_units,
                        priority: rule.priority,
                    },
                }
            } else {
                DecisionDisposition::NoAction {
                    reason_code: "no_current_authorized_failure_observation".into(),
                }
            }
        }
    };

    let payload = PolicyDecisionPayload {
        schema_version: POLICY_SCHEMA_VERSION,
        policy_id: canonical.policy_id.clone(),
        policy_version: canonical.policy_version,
        strategy: canonical.strategy,
        decision_tick: frame.payload.decision_tick,
        input_frame_sha256: frame.payload_sha256.clone(),
        claim_ceiling: POLICY_DECISION_CLAIM_CEILING.into(),
        disposition,
    };
    let encoded = serde_json::to_vec(&payload)
        .map_err(|error| SettlementError(format!("serialize policy decision: {error}")))?;
    let decision = PolicyDecision {
        payload,
        payload_sha256: sha256_hex(&encoded),
    };
    verify_policy_decision_with_policy_and_frame(&decision, frame, &canonical)?;
    Ok(decision)
}

pub fn verify_policy_decision_with_policy_and_frame(
    decision: &PolicyDecision,
    frame: &PolicyInformationFrame,
    definition: &PolicyDefinition,
) -> Result<(), SettlementError> {
    verify_policy_decision_with_frame(decision, frame)?;
    let canonical = definition.canonicalized();
    validate_policy_definition(&canonical)?;
    if decision.payload.policy_id != canonical.policy_id
        || decision.payload.policy_version != canonical.policy_version
        || decision.payload.strategy != canonical.strategy
    {
        return Err(SettlementError(
            "policy decision identity does not match its policy definition".into(),
        ));
    }
    if let DecisionDisposition::RepairRequested { request } = &decision.payload.disposition {
        let rule = canonical
            .repair_rules
            .iter()
            .find(|rule| {
                rule.target_component == request.target_component
                    && rule.target_generation == request.target_generation
            })
            .ok_or_else(|| SettlementError("repair request has no matching policy rule".into()))?;
        let trigger = frame
            .payload
            .observations
            .iter()
            .find(|observation| observation.observation_id == request.trigger_observation_id)
            .ok_or_else(|| SettlementError("repair trigger is missing".into()))?;
        if !rule.allowed_source_ids.iter().any(|source| source == &trigger.source_id)
            || request.required_parts_supply_id != rule.required_parts_supply_id
            || request.required_parts_units != rule.required_parts_units
            || request.required_crew_units != rule.required_crew_units
            || request.dispatch_latency_ticks != rule.dispatch_latency_ticks
            || request.repair_duration_ticks != rule.repair_duration_ticks
            || request.priority != rule.priority
        {
            return Err(SettlementError(
                "repair request does not match its authorized policy rule".into(),
            ));
        }
    }
    Ok(())
}

pub fn verify_policy_decision_with_frame(
    decision: &PolicyDecision,
    frame: &PolicyInformationFrame,
) -> Result<(), SettlementError> {
    verify_policy_information_frame(frame)?;
    verify_policy_decision(decision)?;
    if decision.payload.decision_tick != frame.payload.decision_tick
        || decision.payload.input_frame_sha256 != frame.payload_sha256
    {
        return Err(SettlementError(
            "policy decision is not bound to the supplied information frame".into(),
        ));
    }
    if let DecisionDisposition::RepairRequested { request } = &decision.payload.disposition {
        let observation = frame
            .payload
            .observations
            .iter()
            .find(|observation| observation.observation_id == request.trigger_observation_id)
            .ok_or_else(|| {
                SettlementError("repair trigger observation is absent from input frame".into())
            })?;
        let encoded = serde_json::to_vec(observation).map_err(|error| {
            SettlementError(format!("serialize trigger observation: {error}"))
        })?;
        if sha256_hex(&encoded) != request.trigger_observation_sha256 {
            return Err(SettlementError(
                "repair trigger observation digest mismatch".into(),
            ));
        }
        let is_current_failure = matches!(
            &observation.disposition,
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed
                }
            }
        );
        if !is_current_failure
            || has_conflicting_current_component_observation(
                frame,
                observation,
                KnownComponentCondition::Failed,
            )
            || observation.target_component != request.target_component
            || observation.target_generation != request.target_generation
        {
            return Err(SettlementError(
                "repair request is not supported by its triggering observation".into(),
            ));
        }
    }
    Ok(())
}

pub fn verify_policy_decision(decision: &PolicyDecision) -> Result<(), SettlementError> {
    let payload = &decision.payload;
    if payload.schema_version != POLICY_SCHEMA_VERSION {
        return Err(SettlementError(
            "unsupported policy decision schema version".into(),
        ));
    }
    valid_identifier(&payload.policy_id, "policy ID")?;
    if payload.policy_version == 0 {
        return Err(SettlementError("policy version zero is invalid".into()));
    }
    if payload.claim_ceiling != POLICY_DECISION_CLAIM_CEILING {
        return Err(SettlementError(
            "policy decision claim ceiling is missing or widened".into(),
        ));
    }
    if !is_sha256_hex(&payload.input_frame_sha256) {
        return Err(SettlementError(
            "policy decision input frame digest is malformed".into(),
        ));
    }
    let encoded = serde_json::to_vec(payload)
        .map_err(|error| SettlementError(format!("serialize policy decision: {error}")))?;
    if sha256_hex(&encoded) != decision.payload_sha256 {
        return Err(SettlementError("policy decision digest mismatch".into()));
    }

    match &payload.disposition {
        DecisionDisposition::NoAction { reason_code } => {
            valid_identifier(reason_code, "no-action reason code")?;
            if payload.strategy == BaselineStrategy::ReactiveRepair
                && reason_code == "policy_no_intervention"
            {
                return Err(SettlementError(
                    "reactive policy has a no-intervention disposition code".into(),
                ));
            }
        }
        DecisionDisposition::RepairRequested { request } => {
            valid_identifier(&request.action_id, "repair action ID")?;
            valid_identifier(request.target_component.as_str(), "repair target component ID")?;
            valid_identifier(
                &request.trigger_observation_id,
                "repair trigger observation ID",
            )?;
            valid_identifier(
                &request.required_parts_supply_id,
                "repair parts supply ID",
            )?;
            if request.target_generation == 0
                || request.required_parts_units == 0
                || request.required_crew_units == 0
                || request.repair_duration_ticks == 0
            {
                return Err(SettlementError(
                    "repair request has a zero generation, cost, crew or duration".into(),
                ));
            }
            if !is_sha256_hex(&request.trigger_observation_sha256) {
                return Err(SettlementError(
                    "repair trigger observation digest is malformed".into(),
                ));
            }
            if request.decision_tick != payload.decision_tick {
                return Err(SettlementError(
                    "repair request decision tick mismatches its decision receipt".into(),
                ));
            }
            let expected_dispatch = request
                .decision_tick
                .checked_add(request.dispatch_latency_ticks)
                .ok_or_else(|| SettlementError("repair dispatch tick overflow".into()))?;
            let expected_completion = request
                .not_before_tick
                .checked_add(request.repair_duration_ticks)
                .ok_or_else(|| SettlementError("repair completion tick overflow".into()))?;
            if request.not_before_tick != expected_dispatch
                || request.expected_completion_tick != expected_completion
            {
                return Err(SettlementError(
                    "repair request ticks disagree with declared latency".into(),
                ));
            }
            let action_material = format!(
                "{}\0{}\0{}\0{}\0{}\0{}",
                payload.policy_id,
                payload.policy_version,
                payload.decision_tick,
                request.target_component.as_str(),
                request.target_generation,
                request.trigger_observation_id
            );
            let action_digest = sha256_hex(action_material.as_bytes());
            let expected_action_id = format!("action:{}", &action_digest[..32]);
            if request.action_id != expected_action_id {
                return Err(SettlementError(
                    "repair action ID does not match its causal identity".into(),
                ));
            }
            if payload.strategy != BaselineStrategy::ReactiveRepair {
                return Err(SettlementError(
                    "non-repair policy emitted a repair request".into(),
                ));
            }
        }
        DecisionDisposition::RepairSuppressedPending {
            target_component,
            target_generation,
            pending_action_id,
        } => {
            valid_identifier(target_component.as_str(), "suppressed repair component ID")?;
            valid_identifier(pending_action_id, "pending repair action ID")?;
            if *target_generation == 0 || payload.strategy != BaselineStrategy::ReactiveRepair {
                return Err(SettlementError(
                    "invalid pending repair suppression disposition".into(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_policy_definition(definition: &PolicyDefinition) -> Result<(), SettlementError> {
    if definition.schema_version != POLICY_SCHEMA_VERSION {
        return Err(SettlementError(format!(
            "unsupported policy schema version {}",
            definition.schema_version
        )));
    }
    valid_identifier(&definition.policy_id, "policy ID")?;
    if definition.policy_version == 0 {
        return Err(SettlementError("policy version zero is invalid".into()));
    }
    if definition.strategy == BaselineStrategy::NoIntervention
        && !definition.repair_rules.is_empty()
    {
        return Err(SettlementError(
            "no-intervention policy must not carry repair rules".into(),
        ));
    }

    let mut targets = BTreeSet::new();
    for rule in &definition.repair_rules {
        valid_identifier(rule.target_component.as_str(), "repair rule component ID")?;
        valid_identifier(&rule.required_parts_supply_id, "repair parts supply ID")?;
        if rule.target_generation == 0
            || rule.required_parts_units == 0
            || rule.required_crew_units == 0
            || rule.repair_duration_ticks == 0
        {
            return Err(SettlementError(
                "repair rule has zero generation, parts, crew or duration".into(),
            ));
        }
        if !targets.insert((rule.target_component.clone(), rule.target_generation)) {
            return Err(SettlementError(format!(
                "duplicate repair rule target {} generation {}",
                rule.target_component, rule.target_generation
            )));
        }
        if rule.allowed_source_ids.is_empty() {
            return Err(SettlementError(format!(
                "repair rule for {} has no authorized observation source",
                rule.target_component
            )));
        }
        let mut source_ids = BTreeSet::new();
        for source_id in &rule.allowed_source_ids {
            valid_identifier(source_id, "allowed observation source ID")?;
            if !source_ids.insert(source_id) {
                return Err(SettlementError(format!(
                    "repair rule for {} repeats source ID {}",
                    rule.target_component, source_id
                )));
            }
        }
        rule.dispatch_latency_ticks
            .checked_add(rule.repair_duration_ticks)
            .ok_or_else(|| SettlementError("repair latency overflow".into()))?;
    }
    Ok(())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::{build_policy_information_frame, ObservationSchedule};

    fn failed_observation(
        disposition: ObservationDisposition,
        sample_tick: u64,
        available_tick: u64,
    ) -> super::super::observation::ObservationRecord {
        super::super::observation::ObservationRecord {
            observation_id: "grid-failed-reading".into(),
            source_id: "grid-sensor".into(),
            source_generation: 1,
            target_component: ComponentId::parse("grid").unwrap(),
            target_generation: 1,
            sample_tick,
            available_tick,
            freshness_window_ticks: 1,
            disposition,
        }
    }

    fn policy(strategy: BaselineStrategy) -> PolicyDefinition {
        PolicyDefinition {
            schema_version: POLICY_SCHEMA_VERSION,
            policy_id: "test-policy".into(),
            policy_version: 1,
            strategy,
            repair_rules: if strategy == BaselineStrategy::ReactiveRepair {
                vec![RepairRule {
                    target_component: ComponentId::parse("grid").unwrap(),
                    target_generation: 1,
                    priority: 100,
                    allowed_source_ids: vec!["grid-sensor".into()],
                    required_parts_supply_id: "repair-store".into(),
                    required_parts_units: 2,
                    required_crew_units: 1,
                    dispatch_latency_ticks: 2,
                    repair_duration_ticks: 3,
                }]
            } else {
                Vec::new()
            },
        }
    }

    fn frame_with_records(
        records: Vec<super::super::observation::ObservationRecord>,
        decision_tick: u64,
    ) -> PolicyInformationFrame {
        build_policy_information_frame(
            &ObservationSchedule {
                schema_version: crate::observation::OBSERVATION_SCHEMA_VERSION,
                records,
            },
            decision_tick,
        )
        .unwrap()
    }

    fn frame(
        observation: super::super::observation::ObservationRecord,
        decision_tick: u64,
    ) -> PolicyInformationFrame {
        frame_with_records(vec![observation], decision_tick)
    }

    #[test]
    fn no_intervention_policy_never_requests_repair() {
        let observation = failed_observation(
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed,
                },
            },
            1,
            1,
        );
        let decision = evaluate_baseline_policy(
            &policy(BaselineStrategy::NoIntervention),
            &frame(observation, 1),
        )
        .unwrap();
        assert!(matches!(
            &decision.payload.disposition,
            DecisionDisposition::NoAction { .. }
        ));
        assert!(verify_policy_decision(&decision).is_ok());
    }

    #[test]
    fn reactive_policy_emits_a_delayed_request_from_current_failed_observation() {
        let observation = failed_observation(
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed,
                },
            },
            1,
            1,
        );
        let decision = evaluate_baseline_policy(
            &policy(BaselineStrategy::ReactiveRepair),
            &frame(observation, 1),
        )
        .unwrap();
        let DecisionDisposition::RepairRequested { request } = &decision.payload.disposition
        else {
            panic!("current authorized failure should request a repair");
        };
        assert_eq!(request.target_component.as_str(), "grid");
        assert_eq!(request.not_before_tick, 3);
        assert_eq!(request.expected_completion_tick, 6);
        assert_eq!(request.required_parts_units, 2);
        assert!(verify_policy_decision(&decision).is_ok());
    }

    #[test]
    fn decision_verifier_binds_trigger_to_the_exact_input_frame() {
        let observation = failed_observation(
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed,
                },
            },
            1,
            1,
        );
        let frame = frame(observation, 1);
        let definition = policy(BaselineStrategy::ReactiveRepair);
        let decision = evaluate_baseline_policy(&definition, &frame).unwrap();
        assert!(
            verify_policy_decision_with_policy_and_frame(&decision, &frame, &definition).is_ok()
        );

        let changed_frame = frame(
            failed_observation(
                ObservationDisposition::Current {
                    value: ObservedValue::ComponentCondition {
                        condition: KnownComponentCondition::Available,
                    },
                },
                1,
                1,
            ),
            1,
        );
        assert!(
            verify_policy_decision_with_policy_and_frame(&decision, &changed_frame, &definition)
                .is_err()
        );
    }

    #[test]
    fn stale_missing_conflicting_or_unvalidated_evidence_does_not_trigger() {
        let cases = [
            ObservationDisposition::Stale {
                last_value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed,
                },
            },
            ObservationDisposition::Missing {
                reason_ref: "sensor-dropout".into(),
            },
            ObservationDisposition::Conflicting {
                candidate_observation_ids: vec!["reading-a".into(), "reading-b".into()],
            },
            ObservationDisposition::Unvalidated {
                value: Some(ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed,
                }),
            },
        ];
        for disposition in cases {
            let is_stale = matches!(&disposition, ObservationDisposition::Stale { .. });
            let sample_tick = if is_stale { 0 } else { 1 };
            let mut observation = failed_observation(disposition, sample_tick, 1);
            if is_stale {
                observation.freshness_window_ticks = 0;
            }
            let decision = evaluate_baseline_policy(
                &policy(BaselineStrategy::ReactiveRepair),
                &frame(observation, 1),
            )
            .unwrap();
            assert!(matches!(
                &decision.payload.disposition,
                DecisionDisposition::NoAction { .. }
            ));
        }
    }

    #[test]
    fn contradictory_current_observations_block_repair() {
        let failed = failed_observation(
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed,
                },
            },
            1,
            1,
        );
        let mut available = failed.clone();
        available.observation_id = "grid-available-reading".into();
        available.source_id = "grid-backup-sensor".into();
        available.disposition = ObservationDisposition::Current {
            value: ObservedValue::ComponentCondition {
                condition: KnownComponentCondition::Available,
            },
        };
        let frame = frame_with_records(vec![failed, available], 1);
        let decision = evaluate_baseline_policy(
            &policy(BaselineStrategy::ReactiveRepair),
            &frame,
        )
        .unwrap();
        assert!(matches!(
            &decision.payload.disposition,
            DecisionDisposition::NoAction { .. }
        ));
    }

    #[test]
    fn unauthorized_observation_source_cannot_trigger_repair() {
        let mut definition = policy(BaselineStrategy::ReactiveRepair);
        definition.repair_rules[0].allowed_source_ids = vec!["authorized-sensor".into()];
        let observation = failed_observation(
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed,
                },
            },
            1,
            1,
        );
        let decision =
            evaluate_baseline_policy(&definition, &frame(observation, 1)).unwrap();
        assert!(matches!(
            &decision.payload.disposition,
            DecisionDisposition::NoAction { .. }
        ));
    }

    #[test]
    fn highest_priority_then_stable_id_breaks_multi_failure_ties() {
        let mut definition = policy(BaselineStrategy::ReactiveRepair);
        definition.repair_rules.push(RepairRule {
            target_component: ComponentId::parse("pump").unwrap(),
            target_generation: 1,
            priority: 200,
            allowed_source_ids: vec!["grid-sensor".into()],
            required_parts_supply_id: "pump-store".into(),
            required_parts_units: 1,
            required_crew_units: 1,
            dispatch_latency_ticks: 1,
            repair_duration_ticks: 2,
        });
        let mut grid = failed_observation(
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed,
                },
            },
            1,
            1,
        );
        grid.observation_id = "grid-reading".into();
        let mut pump = grid.clone();
        pump.observation_id = "pump-reading".into();
        pump.target_component = ComponentId::parse("pump").unwrap();

        let decision = evaluate_baseline_policy(
            &definition,
            &frame_with_records(vec![grid.clone(), pump.clone()], 1),
        )
        .unwrap();
        assert!(matches!(
            &decision.payload.disposition,
            DecisionDisposition::RepairRequested { request }
                if request.target_component.as_str() == "pump"
        ));

        // Equal priority uses stable component ID ordering, not observation arrival order.
        definition.repair_rules[1].priority = 100;
        let tied = evaluate_baseline_policy(
            &definition,
            &frame_with_records(vec![pump, grid], 1),
        )
        .unwrap();
        assert!(matches!(
            &tied.payload.disposition,
            DecisionDisposition::RepairRequested { request }
                if request.target_component.as_str() == "grid"
        ));
    }

    #[test]
    fn duplicate_rules_overflow_and_unknown_fields_fail_closed() {
        let mut duplicate = policy(BaselineStrategy::ReactiveRepair);
        duplicate.repair_rules.push(duplicate.repair_rules[0].clone());
        let failed_frame = frame(
            failed_observation(
                ObservationDisposition::Current {
                    value: ObservedValue::ComponentCondition {
                        condition: KnownComponentCondition::Failed,
                    },
                },
                1,
                1,
            ),
            1,
        );
        assert!(evaluate_baseline_policy(&duplicate, &failed_frame).is_err());

        let mut overflow = policy(BaselineStrategy::ReactiveRepair);
        overflow.repair_rules[0].dispatch_latency_ticks = u64::MAX;
        assert!(evaluate_baseline_policy(&overflow, &failed_frame).is_err());

        let mut json = serde_json::to_value(policy(BaselineStrategy::NoIntervention)).unwrap();
        json["unreviewed_action"] = serde_json::json!("unsafe");
        assert!(serde_json::from_value::<PolicyDefinition>(json).is_err());
    }

    #[test]
    fn repair_request_must_match_the_rule_and_triggering_frame() {
        let definition = policy(BaselineStrategy::ReactiveRepair);
        let observation = failed_observation(
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed,
                },
            },
            1,
            1,
        );
        let frame = frame(observation, 1);
        let mut decision =
            evaluate_baseline_policy(&definition, &frame).unwrap();
        let DecisionDisposition::RepairRequested { request } = &mut decision.payload.disposition
        else {
            panic!("expected repair request");
        };
        request.required_parts_units += 1;
        let encoded = serde_json::to_vec(&decision.payload).unwrap();
        decision.payload_sha256 = sha256_hex(&encoded);

        assert!(verify_policy_decision(&decision).is_ok());
        assert!(
            verify_policy_decision_with_policy_and_frame(&decision, &frame, &definition).is_err()
        );
    }

    #[test]
    fn policy_decision_tampering_is_detected_even_if_outer_digest_is_recomputed() {
        let observation = failed_observation(
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed,
                },
            },
            1,
            1,
        );
        let mut decision = evaluate_baseline_policy(
            &policy(BaselineStrategy::ReactiveRepair),
            &frame(observation, 1),
        )
        .unwrap();
        let DecisionDisposition::RepairRequested { request } = &mut decision.payload.disposition
        else {
            panic!("expected repair request");
        };
        request.expected_completion_tick = request.not_before_tick;
        let encoded = serde_json::to_vec(&decision.payload).unwrap();
        decision.payload_sha256 = sha256_hex(&encoded);
        assert!(verify_policy_decision(&decision).is_err());
    }
}
