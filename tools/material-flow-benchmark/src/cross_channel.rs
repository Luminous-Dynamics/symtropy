// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Cross-channel L1 evaluation for a physically blocked receiver and a
//! synthetic false-positive device observation.
//!
//! Simulator ground truth is produced by the physics evaluator and retained
//! only in the evaluator report. It is never passed to the receiver-presence
//! adjudicator; that function receives only declared device observations and
//! an observation policy.

use serde::Serialize;

use crate::ScenarioManifest;
use crate::observation::{
    PresenceAdjudicationPolicy, PresenceAdjudicationReport, PresenceReading, PresenceVerdict,
    ReceiverPresenceObservation, adjudicate_receiver_presence,
};
use crate::simulation::simulate_closed_gate_obstruction;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CrossChannelFaultResult {
    pub schema_version: u16,
    pub scenario_id: String,
    pub seed: u64,
    /// Physics-derived ground truth is evaluator-only and is not an input to
    /// the observation adjudicator or a controller-visible record.
    pub physical_ground_truth_for_evaluator: PhysicalGroundTruthSummary,
    /// Device observations are independent inputs. They are not rewritten
    /// using the simulator result.
    pub device_observations: Vec<ReceiverPresenceObservation>,
    pub observation_adjudication: PresenceAdjudicationReport,
    pub physical_and_observation_disagreement_detected: bool,
    pub reconciliation_required: bool,
    pub blind_retry_blocked: bool,
    pub custody_completion_blocked: bool,
    pub false_completion_blocked: bool,
    pub fault_oracle_passed: bool,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PhysicalGroundTruthSummary {
    pub visibility: String,
    pub package_inside_tote: bool,
    pub package_center_m: [f64; 3],
    pub gate_contact_event_count: usize,
    pub gate_contact_max_impulse_ns: f64,
    pub transaction_state: crate::transaction::PhysicalTransactionState,
    pub delivery_accepted: bool,
}

fn observations_for_scenario(
    scenario: &ScenarioManifest,
) -> (PresenceAdjudicationPolicy, Vec<ReceiverPresenceObservation>) {
    let uncertainty_ms = u64::from(scenario.clock_uncertainty_ms);
    let earliest_decision = scenario.started_at_unix_ms.saturating_sub(uncertainty_ms);
    let latest_decision = scenario.started_at_unix_ms.saturating_add(uncertainty_ms);

    // Synthetic fault schedule: the receiver's presence sensor falsely says
    // "present" while a distinct intake-plane channel says "absent". The
    // observation values are fixed independently of simulator ground truth.
    let observations = vec![
        ReceiverPresenceObservation {
            event_id: "evt-false-positive-receiver-presence".into(),
            source_id: "nestport-presence-sensor".into(),
            common_cause_group: Some("nestport-presence-controller".into()),
            scenario_id: scenario.scenario_id.clone(),
            subject_id: scenario.package.package_id.clone(),
            reading: PresenceReading::Present,
            observed_at_unix_ms: earliest_decision.saturating_sub(1_000),
            expires_at_unix_ms: latest_decision.saturating_add(1_000),
        },
        ReceiverPresenceObservation {
            event_id: "evt-independent-intake-plane-absence".into(),
            source_id: "intake-plane-optical-sensor".into(),
            common_cause_group: Some("intake-plane-optical-controller".into()),
            scenario_id: scenario.scenario_id.clone(),
            subject_id: scenario.package.package_id.clone(),
            reading: PresenceReading::Absent,
            observed_at_unix_ms: earliest_decision.saturating_sub(900),
            expires_at_unix_ms: latest_decision.saturating_add(1_100),
        },
    ];

    let policy = PresenceAdjudicationPolicy {
        scenario_id: scenario.scenario_id.clone(),
        subject_id: scenario.package.package_id.clone(),
        earliest_decision_time_unix_ms: earliest_decision,
        latest_decision_time_unix_ms: latest_decision,
        maximum_observation_age_ms: 2_000,
        minimum_independent_groups: 2,
    };
    (policy, observations)
}

/// Execute the static-gate L1 physical scene, then independently adjudicate a
/// conflicting receiver-presence signal. No ground-truth value is passed to
/// the observation evaluator.
pub fn run_gate_fault_observation_cross_check(
    scenario: &ScenarioManifest,
) -> Result<CrossChannelFaultResult, String> {
    let physical = simulate_closed_gate_obstruction(scenario)?;
    let (policy, device_observations) = observations_for_scenario(scenario);
    let observation_adjudication = adjudicate_receiver_presence(&device_observations, &policy)
        .map_err(|errors| format!("observation adjudication rejected: {}", errors.join("; ")))?;

    let physical_and_observation_disagreement_detected = !physical.simulation.safe_inside_tote
        && device_observations
            .iter()
            .any(|observation| observation.reading == PresenceReading::Present)
        && observation_adjudication.verdict == PresenceVerdict::Contradictory
        && !observation_adjudication.positive_presence_confirmed;

    let reconciliation_required = physical.reconciliation_required;
    let blind_retry_blocked = physical.blind_retry_blocked;
    let custody_completion_blocked = !observation_adjudication.custody_completion_permitted;
    let false_completion_blocked = physical.false_completion_blocked
        && !physical.simulation.accepted
        && custody_completion_blocked;

    let fault_oracle_passed = physical.fault_oracle_passed
        && physical_and_observation_disagreement_detected
        && reconciliation_required
        && blind_retry_blocked
        && custody_completion_blocked
        && false_completion_blocked;

    Ok(CrossChannelFaultResult {
        schema_version: scenario.schema_version,
        scenario_id: scenario.scenario_id.clone(),
        seed: scenario.seed,
        physical_ground_truth_for_evaluator: PhysicalGroundTruthSummary {
            visibility: "independent_evaluator_only".into(),
            package_inside_tote: physical.simulation.safe_inside_tote,
            package_center_m: physical.simulation.package_center_m,
            gate_contact_event_count: physical.gate_contact_event_count,
            gate_contact_max_impulse_ns: physical.gate_contact_max_impulse_ns,
            transaction_state: physical.simulation.transaction_state,
            delivery_accepted: physical.simulation.accepted,
        },
        device_observations,
        observation_adjudication,
        physical_and_observation_disagreement_detected,
        reconciliation_required,
        blind_retry_blocked,
        custody_completion_blocked,
        false_completion_blocked,
        fault_oracle_passed,
        limitations: vec![
            "The false-positive and independent-absence observations are synthetic typed inputs, not live sensor outputs".into(),
            "The observation adjudicator receives no simulator ground-truth object or package pose".into(),
            "Common-cause groups are declared in the fixture and are not authenticated or physically proven independent".into(),
            "The static gate is collision geometry, not a moving gate, actuator, sensor or interlock controller".into(),
            "This is L1 same-host simulation plus synthetic observation adjudication; it is not HIL, guarded-rig or field evidence".into(),
            "No cryptographic authentication, trusted chronology or sensor truth is established".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIO: &str = include_str!("../fixtures/nominal-stationary-carrier.json");

    fn scenario() -> ScenarioManifest {
        serde_json::from_str(SCENARIO).expect("MFB scenario fixture")
    }

    #[test]
    fn false_positive_presence_cannot_override_physical_gate_obstruction() {
        let result = run_gate_fault_observation_cross_check(&scenario())
            .expect("cross-channel fault episode should execute");
        assert_eq!(
            result.physical_ground_truth_for_evaluator.visibility,
            "independent_evaluator_only"
        );
        assert!(
            !result
                .physical_ground_truth_for_evaluator
                .package_inside_tote
        );
        assert_eq!(
            result.physical_ground_truth_for_evaluator.transaction_state,
            crate::transaction::PhysicalTransactionState::ReconciliationRequired
        );
        assert_eq!(
            result.observation_adjudication.verdict,
            PresenceVerdict::Contradictory
        );
        assert_eq!(result.device_observations.len(), 2);
        assert!(
            result
                .device_observations
                .iter()
                .any(|observation| observation.reading == PresenceReading::Present)
        );
        assert!(result.physical_and_observation_disagreement_detected);
        assert!(result.reconciliation_required);
        assert!(result.blind_retry_blocked);
        assert!(result.custody_completion_blocked);
        assert!(result.false_completion_blocked);
        assert!(!result.physical_ground_truth_for_evaluator.delivery_accepted);
        assert!(result.fault_oracle_passed, "{result:?}");
    }

    #[test]
    fn observation_adjudicator_reports_no_authority_even_when_physics_is_known_to_evaluator() {
        let result = run_gate_fault_observation_cross_check(&scenario())
            .expect("cross-channel fault episode should execute");
        assert!(!result.observation_adjudication.sensor_truth_established);
        assert!(
            !result
                .observation_adjudication
                .cryptographic_authentication_verified
        );
        assert!(!result.observation_adjudication.trusted_time_verified);
        assert!(!result.observation_adjudication.motion_authority_granted);
        assert!(!result.observation_adjudication.physical_command_emitted);
        assert!(!result.observation_adjudication.custody_completion_permitted);
    }

    #[test]
    fn scenario_identity_and_package_subject_are_bound_into_both_channels() {
        let scenario = scenario();
        let result = run_gate_fault_observation_cross_check(&scenario)
            .expect("cross-channel fault episode should execute");
        assert_eq!(result.scenario_id, scenario.scenario_id);
        assert!(result.device_observations.iter().all(|observation| {
            observation.scenario_id == scenario.scenario_id
                && observation.subject_id == scenario.package.package_id
        }));
    }
}
