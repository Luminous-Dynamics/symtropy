//! Composition of SIM-REL-039 finite thermal challenges and SIM-REL-040
//! causal action-realization evidence.
//!
//! The bridge verifies that the physical episode, actuator readback, state
//! commitments, sensor sample and downstream decision all refer to one
//! intervention. It does not independently validate a real furnace model or
//! authenticate raw instrument records. State/sample commitments are checked
//! through an explicit provider so test fixtures cannot masquerade as
//! production cryptographic verification.

use super::causal_execution::{
    AcknowledgementState, CausalExecutionAuditReceipt, CausalExecutionBundle,
    DecisionEvidenceStatus, RealizationAssessment, audit_causal_execution_bundle,
};
use super::counterexample_ledger::EvidenceSplit;
use super::finite_physical_challenge::{
    FiniteThermalChallengeRecipe, GeneratedFiniteThermalChallenge,
    verify_finite_thermal_challenge_from_recipe,
};
use super::scientific_challenge::ChallengeSplitRegistry;
use crate::Rational;

const MILLWATTS_PER_WATT: u64 = 1_000;
const SUPPORTED_COMMAND_UNIT: &str = "milliwatt";
const SUPPORTED_MODEL_VERSION: &str = "one-node-thermal-euler-v1";
const MAX_ID_BYTES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThermalPhysicalReadbackBinding {
    pub task_id: String,
    pub command_id: String,
    pub actuator_readback_unit: String,
    pub model_version: String,
    pub transition_event_id: String,
    pub sensor_event_id: String,
    pub instrument_id: String,
    pub sensor_calibration_receipt_id: String,
    pub raw_sample_digest: String,
    pub pre_state_temperature: Rational,
    pub post_state_temperature: Rational,
    pub measured_post_state_temperature: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThermalCausalBridgeReceipt {
    pub task_id: String,
    pub command_id: String,
    pub recipe_id: String,
    pub recipe_version: String,
    pub family_group_id: String,
    pub evidence_split: EvidenceSplit,
    pub causal_audit: CausalExecutionAuditReceipt,
    pub commanded_power_watts: Rational,
    pub realized_power_watts: Rational,
    pub actuator_measurement_interval_contains_realized_power: bool,
    pub pre_state_commitment_verified: bool,
    pub post_state_commitment_verified: bool,
    pub sensor_sample_commitment_verified: bool,
    pub physical_hazard_observed: bool,
    pub decision_status: DecisionEvidenceStatus,
    pub cross_module_binding_complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThermalBridgeStage {
    ChallengeReplay,
    CausalExecution,
    IdentityBinding,
    UnitConversion,
    ActuatorReadback,
    PhysicalTransition,
    SensorAcquisition,
    DecisionSafety,
    CommitmentVerification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThermalBridgeError {
    pub stage: ThermalBridgeStage,
    pub reason: String,
}

impl std::fmt::Display for ThermalBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.stage, self.reason)
    }
}

impl std::error::Error for ThermalBridgeError {}

/// Provider that binds exact rational thermal states and sensor samples to
/// their declared content digests. A production implementation should use a
/// canonical, versioned encoding and a cryptographic digest implementation.
/// Unit-test providers must not be used in production.
pub trait ThermalEvidenceCommitmentVerifier {
    fn verify_state_commitment(
        &self,
        digest: &str,
        model_version: &str,
        temperature: Rational,
    ) -> Result<bool, String>;

    fn verify_sensor_sample_commitment(
        &self,
        digest: &str,
        instrument_id: &str,
        calibration_receipt_id: &str,
        temperature_readback: Rational,
    ) -> Result<bool, String>;
}

fn fail(stage: ThermalBridgeStage, reason: impl Into<String>) -> ThermalBridgeError {
    ThermalBridgeError {
        stage,
        reason: reason.into(),
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn nonnegative_integer(value: i64, label: &str) -> Result<u64, ThermalBridgeError> {
    u64::try_from(value).map_err(|_| {
        fail(
            ThermalBridgeStage::UnitConversion,
            format!("{label} must be non-negative"),
        )
    })
}

fn milli_units_interval_contains(
    measured_value: i64,
    uncertainty: u64,
    expected: Rational,
) -> Result<bool, ThermalBridgeError> {
    let measured = nonnegative_integer(measured_value, "actuator measurement")?;
    let lower = measured.saturating_sub(uncertainty);
    let upper = measured.checked_add(uncertainty).ok_or_else(|| {
        fail(
            ThermalBridgeStage::ActuatorReadback,
            "actuator measurement upper uncertainty bound overflows u64",
        )
    })?;
    Ok(Rational::from_u64(lower) <= expected && expected <= Rational::from_u64(upper))
}

/// Join a deterministic finite thermal episode with its causal execution
/// bundle. Command power is expressed as integer milliwatts in the causal
/// ledger and rational watts in the thermal model. Actuator readback uncertainty
/// is expressed in milliwatts as well.
///
/// The thermal challenge must be regenerated from the exact recipe, registered
/// split and seed. The causal audit must prove event order and a device
/// acknowledgement. The measured actuator interval must contain the exact
/// simulated realized power. Physical state and sensor sample digests must be
/// verified against the exact thermal values by the supplied commitment
/// provider.
///
/// A successful receipt establishes cross-module agreement only. It is not a
/// hardware validation or physical safety certificate.
pub fn verify_thermal_causal_execution_bridge<C>(
    challenge: &GeneratedFiniteThermalChallenge,
    recipe: &FiniteThermalChallengeRecipe,
    split_registry: &ChallengeSplitRegistry,
    causal: &CausalExecutionBundle,
    physical_binding: &ThermalPhysicalReadbackBinding,
    commitments: &C,
) -> Result<ThermalCausalBridgeReceipt, ThermalBridgeError>
where
    C: ThermalEvidenceCommitmentVerifier,
{
    verify_finite_thermal_challenge_from_recipe(challenge, recipe, split_registry).map_err(
        |reason| fail(ThermalBridgeStage::ChallengeReplay, reason),
    )?;
    let causal_audit = audit_causal_execution_bundle(causal).map_err(|error| {
        fail(
            ThermalBridgeStage::CausalExecution,
            format!("{:?}: {}", error.stage, error.reason),
        )
    })?;

    if causal_audit.resolved_delivery != AcknowledgementState::Accepted
        || causal_audit.realization_assessment == RealizationAssessment::Unattributed
    {
        return Err(fail(
            ThermalBridgeStage::CausalExecution,
            "thermal challenge cannot be attributed to a command without device acceptance",
        ));
    }

    let task_id = &challenge.public_task.task_id;
    let command_id = &causal.command.command_id;
    let generation = &challenge.generation_receipt;
    if !valid_id(task_id)
        || !valid_id(command_id)
        || physical_binding.task_id != *task_id
        || physical_binding.command_id != *command_id
        || generation.recipe_id != recipe.recipe_id
        || generation.recipe_version != recipe.recipe_version
        || generation.family_group_id != recipe.family_group_id
        || generation.split != recipe.split
    {
        return Err(fail(
            ThermalBridgeStage::IdentityBinding,
            "task, command, recipe or registered evidence-split identities do not agree",
        ));
    }
    if causal.command.unit != SUPPORTED_COMMAND_UNIT {
        return Err(fail(
            ThermalBridgeStage::UnitConversion,
            format!(
                "unsupported command unit {}; expected {}",
                causal.command.unit, SUPPORTED_COMMAND_UNIT
            ),
        ));
    }
    if physical_binding.actuator_readback_unit != SUPPORTED_COMMAND_UNIT {
        return Err(fail(
            ThermalBridgeStage::UnitConversion,
            "actuator readback unit must use the same declared milliwatt unit as the command",
        ));
    }
    if physical_binding.model_version != SUPPORTED_MODEL_VERSION
        || causal.physical_transition.model_version != SUPPORTED_MODEL_VERSION
    {
        return Err(fail(
            ThermalBridgeStage::IdentityBinding,
            "causal and thermal model versions differ from the supported discrete model profile",
        ));
    }

    let requested_milliwatts = Rational::from_u64(nonnegative_integer(
        causal.command.requested_value,
        "commanded power",
    )?);
    let commanded_power_milliwatts = challenge
        .public_task
        .commanded_power
        .checked_mul(Rational::from_u64(MILLWATTS_PER_WATT))
        .map_err(|reason| fail(ThermalBridgeStage::UnitConversion, reason))?;
    if requested_milliwatts != commanded_power_milliwatts {
        return Err(fail(
            ThermalBridgeStage::UnitConversion,
            "causal command milliwatts do not exactly equal the thermal model command in watts",
        ));
    }

    let episode = &challenge.sealed_oracle.episode;
    let first = episode.steps.first().ok_or_else(|| {
        fail(
            ThermalBridgeStage::PhysicalTransition,
            "thermal episode has no physical transition steps",
        )
    })?;
    let last = episode.steps.last().ok_or_else(|| {
        fail(
            ThermalBridgeStage::PhysicalTransition,
            "thermal episode has no final physical transition step",
        )
    })?;
    if episode
        .steps
        .iter()
        .any(|step| step.realized_power != first.realized_power)
    {
        return Err(fail(
            ThermalBridgeStage::ActuatorReadback,
            "one-shot thermal challenge contains inconsistent realized power across steps",
        ));
    }
    let realized_power_milliwatts = first
        .realized_power
        .checked_mul(Rational::from_u64(MILLWATTS_PER_WATT))
        .map_err(|reason| fail(ThermalBridgeStage::ActuatorReadback, reason))?;
    if !milli_units_interval_contains(
        causal.actuator_readback.measured_value,
        causal.actuator_readback.measurement_uncertainty,
        realized_power_milliwatts,
    )? {
        return Err(fail(
            ThermalBridgeStage::ActuatorReadback,
            "actuator measurement interval does not contain the thermal model's exact realized power",
        ));
    }

    if physical_binding.transition_event_id != causal.physical_transition.event_id
        || causal.physical_transition.parent_actuator_event_id
            != causal.actuator_readback.event_id
        || physical_binding.pre_state_temperature != first.temperature_before
        || physical_binding.post_state_temperature != last.temperature_after
    {
        return Err(fail(
            ThermalBridgeStage::PhysicalTransition,
            "physical transition identity or exact pre/post temperatures differ from the thermal episode",
        ));
    }
    let pre_state_commitment_verified = commitments
        .verify_state_commitment(
            &causal.physical_transition.pre_state_digest,
            &physical_binding.model_version,
            physical_binding.pre_state_temperature,
        )
        .map_err(|reason| fail(ThermalBridgeStage::CommitmentVerification, reason))?;
    let post_state_commitment_verified = commitments
        .verify_state_commitment(
            &causal.physical_transition.post_state_digest,
            &physical_binding.model_version,
            physical_binding.post_state_temperature,
        )
        .map_err(|reason| fail(ThermalBridgeStage::CommitmentVerification, reason))?;
    if !pre_state_commitment_verified || !post_state_commitment_verified {
        return Err(fail(
            ThermalBridgeStage::CommitmentVerification,
            "physical pre/post state digest does not commit to the exact thermal state value",
        ));
    }

    if physical_binding.sensor_event_id != causal.sensor_acquisition.event_id
        || causal.sensor_acquisition.parent_transition_event_id
            != causal.physical_transition.event_id
        || physical_binding.instrument_id != causal.sensor_acquisition.instrument_id
        || physical_binding.sensor_calibration_receipt_id
            != causal.sensor_acquisition.calibration_receipt_id
        || physical_binding.raw_sample_digest != causal.sensor_acquisition.raw_sample_digest
        || physical_binding.measured_post_state_temperature
            != last.measured_temperature_after
    {
        return Err(fail(
            ThermalBridgeStage::SensorAcquisition,
            "sensor event, calibration, raw-sample digest or measured temperature differs from the thermal episode",
        ));
    }
    let sensor_sample_commitment_verified = commitments
        .verify_sensor_sample_commitment(
            &physical_binding.raw_sample_digest,
            &physical_binding.instrument_id,
            &physical_binding.sensor_calibration_receipt_id,
            physical_binding.measured_post_state_temperature,
        )
        .map_err(|reason| fail(ThermalBridgeStage::CommitmentVerification, reason))?;
    if !sensor_sample_commitment_verified {
        return Err(fail(
            ThermalBridgeStage::CommitmentVerification,
            "sensor sample digest does not commit to the exact instrument/calibration/readback tuple",
        ));
    }

    if !episode.remained_within_safe_envelope && causal.decision.risk_budget_satisfied {
        return Err(fail(
            ThermalBridgeStage::DecisionSafety,
            "thermal hazard was observed but the downstream decision claims its risk budget is satisfied",
        ));
    }

    Ok(ThermalCausalBridgeReceipt {
        task_id: task_id.clone(),
        command_id: command_id.clone(),
        recipe_id: recipe.recipe_id.clone(),
        recipe_version: recipe.recipe_version.clone(),
        family_group_id: recipe.family_group_id.clone(),
        evidence_split: recipe.split,
        causal_audit,
        commanded_power_watts: challenge.public_task.commanded_power,
        realized_power_watts: first.realized_power,
        actuator_measurement_interval_contains_realized_power: true,
        pre_state_commitment_verified,
        post_state_commitment_verified,
        sensor_sample_commitment_verified,
        physical_hazard_observed: !episode.remained_within_safe_envelope,
        decision_status: if causal.decision.abstained {
            DecisionEvidenceStatus::Abstained
        } else if causal.decision.risk_budget_satisfied {
            DecisionEvidenceStatus::RiskBudgetSatisfied
        } else {
            DecisionEvidenceStatus::RiskBudgetViolation
        },
        cross_module_binding_complete: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        SUPPORTED_MODEL_VERSION, ThermalEvidenceCommitmentVerifier,
        ThermalPhysicalReadbackBinding, verify_thermal_causal_execution_bridge,
    };
    use super::super::causal_execution::{
        AcknowledgementState, ActuatorReadback, CausalExecutionBundle, CommandIntent,
        DecisionEvidence, DispatchReceipt, DispatchState, DeviceAcknowledgement,
        InferenceEvidence, PhysicalTransitionEvidence, SensorAcquisition,
        UncertainTimestamp,
    };
    use super::super::counterexample_ledger::EvidenceSplit;
    use super::super::finite_physical_challenge::{
        FiniteThermalChallengeRecipe, GeneratedFiniteThermalChallenge,
        generate_finite_thermal_challenge,
    };
    use super::super::scientific_challenge::ChallengeSplitRegistry;
    use crate::Rational;

    struct TestCommitments;

    impl ThermalEvidenceCommitmentVerifier for TestCommitments {
        fn verify_state_commitment(
            &self,
            digest: &str,
            model_version: &str,
            temperature: Rational,
        ) -> Result<bool, String> {
            Ok(digest == format!(
                "test-state:{model_version}:{}:{}",
                temperature.numerator(),
                temperature.denominator()
            ))
        }

        fn verify_sensor_sample_commitment(
            &self,
            digest: &str,
            instrument_id: &str,
            calibration_receipt_id: &str,
            temperature_readback: Rational,
        ) -> Result<bool, String> {
            Ok(digest == format!(
                "test-sample:{instrument_id}:{calibration_receipt_id}:{}:{}",
                temperature_readback.numerator(),
                temperature_readback.denominator()
            ))
        }
    }

    fn q(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn registry() -> ChallengeSplitRegistry {
        ChallengeSplitRegistry {
            development_group_ids: vec!["thermal-dev".into()],
            calibration_group_ids: vec!["thermal-cal".into()],
            sealed_holdout_group_ids: vec!["thermal-holdout".into()],
        }
    }

    fn recipe(safe_limit: u64) -> FiniteThermalChallengeRecipe {
        FiniteThermalChallengeRecipe {
            task_id: "thermal-task-bridge".into(),
            recipe_id: "one-node-thermal".into(),
            recipe_version: "v1".into(),
            family_group_id: "thermal-dev".into(),
            split: EvidenceSplit::Development,
            initial_temperature_grid: vec![q(80, 1)],
            ambient_temperature_grid: vec![q(20, 1)],
            heat_capacity_grid: vec![q(10, 1)],
            conductance_grid: vec![q(1, 1)],
            time_step_grid: vec![q(1, 1)],
            commanded_power_grid: vec![q(100, 1)],
            actuator_delivery_fraction_grid: vec![q(1, 2)],
            sensor_bias_grid: vec![q(1, 2)],
            safe_temperature_grid: vec![q(u128::from(safe_limit), 1)],
            horizon: 2,
        }
    }

    fn make_challenge(recipe: &FiniteThermalChallengeRecipe) -> GeneratedFiniteThermalChallenge {
        generate_finite_thermal_challenge(recipe, &registry(), 0x5eed).unwrap()
    }

    fn clock(sequence: u64) -> UncertainTimestamp {
        UncertainTimestamp {
            timestamp_ns: sequence * 1_000,
            uncertainty_ns: 10,
        }
    }

    fn make_binding(challenge: &GeneratedFiniteThermalChallenge) -> ThermalPhysicalReadbackBinding {
        let first = challenge.sealed_oracle.episode.steps.first().unwrap();
        let last = challenge.sealed_oracle.episode.steps.last().unwrap();
        let pre = first.temperature_before;
        let post = last.temperature_after;
        let measured = last.measured_temperature_after;
        ThermalPhysicalReadbackBinding {
            task_id: challenge.public_task.task_id.clone(),
            command_id: "heater-command-1".into(),
            actuator_readback_unit: "milliwatt".into(),
            model_version: SUPPORTED_MODEL_VERSION.into(),
            transition_event_id: "transition-event".into(),
            sensor_event_id: "sensor-event".into(),
            instrument_id: "thermometer-1".into(),
            sensor_calibration_receipt_id: "sensor-cal-1".into(),
            raw_sample_digest: format!(
                "test-sample:thermometer-1:sensor-cal-1:{}:{}",
                measured.numerator(),
                measured.denominator()
            ),
            pre_state_temperature: pre,
            post_state_temperature: post,
            measured_post_state_temperature: measured,
        }
    }

    fn make_causal(challenge: &GeneratedFiniteThermalChallenge) -> CausalExecutionBundle {
        let binding = make_binding(challenge);
        let pre = binding.pre_state_temperature;
        let post = binding.post_state_temperature;
        let first = challenge.sealed_oracle.episode.steps.first().unwrap();
        let commanded_milliwatts = challenge.public_task.commanded_power
            .checked_mul(Rational::from_u64(1_000))
            .unwrap();
        let realized_milliwatts = first.realized_power
            .checked_mul(Rational::from_u64(1_000))
            .unwrap();
        assert_eq!(commanded_milliwatts.denominator(), 1);
        assert_eq!(realized_milliwatts.denominator(), 1);
        let commanded_milliwatts = i64::try_from(commanded_milliwatts.numerator()).unwrap();
        let realized_milliwatts = i64::try_from(realized_milliwatts.numerator()).unwrap();
        CausalExecutionBundle {
            command: CommandIntent {
                event_id: "command-event".into(),
                sequence: 0,
                command_id: "heater-command-1".into(),
                device_id: "heater-1".into(),
                requested_value: commanded_milliwatts,
                unit: "milliwatt".into(),
                tolerance: 1_000,
                operating_minimum: 0,
                operating_maximum: commanded_milliwatts.checked_mul(2).unwrap(),
                clock: clock(1),
            },
            dispatch: DispatchReceipt {
                event_id: "dispatch-event".into(),
                sequence: 1,
                command_id: "heater-command-1".into(),
                device_id: "heater-1".into(),
                transport_id: "transport-1".into(),
                payload_digest: "payload-sha256".into(),
                state: DispatchState::Accepted,
                clock: clock(2),
            },
            acknowledgement: DeviceAcknowledgement {
                event_id: "ack-event".into(),
                sequence: 2,
                command_id: "heater-command-1".into(),
                device_id: "heater-1".into(),
                firmware_identity: "heater-fw-1".into(),
                state: AcknowledgementState::Accepted,
                echoed_value: Some(commanded_milliwatts),
                clock: clock(3),
            },
            actuator_readback: ActuatorReadback {
                event_id: "actuator-event".into(),
                sequence: 3,
                command_id: "heater-command-1".into(),
                device_id: "heater-1".into(),
                measured_value: realized_milliwatts,
                measurement_uncertainty: 500,
                calibration_receipt_id: "actuator-cal-1".into(),
                clock: clock(4),
            },
            physical_transition: PhysicalTransitionEvidence {
                event_id: "transition-event".into(),
                sequence: 4,
                command_id: "heater-command-1".into(),
                parent_actuator_event_id: "actuator-event".into(),
                pre_state_digest: format!(
                    "test-state:{SUPPORTED_MODEL_VERSION}:{}:{}",
                    pre.numerator(),
                    pre.denominator()
                ),
                post_state_digest: format!(
                    "test-state:{SUPPORTED_MODEL_VERSION}:{}:{}",
                    post.numerator(),
                    post.denominator()
                ),
                model_version: SUPPORTED_MODEL_VERSION.into(),
                clock: clock(5),
            },
            sensor_acquisition: SensorAcquisition {
                event_id: "sensor-event".into(),
                sequence: 5,
                command_id: "heater-command-1".into(),
                parent_transition_event_id: "transition-event".into(),
                instrument_id: "thermometer-1".into(),
                raw_sample_digest: binding.raw_sample_digest.clone(),
                calibration_receipt_id: "sensor-cal-1".into(),
                quality_flags: Vec::new(),
                clock: clock(6),
            },
            inference: InferenceEvidence {
                event_id: "inference-event".into(),
                sequence: 6,
                command_id: "heater-command-1".into(),
                sensor_event_ids: vec!["sensor-event".into()],
                inference_version: "inference-v1".into(),
                conclusion_digest: "conclusion-digest".into(),
                uncertainty_digest: "uncertainty-digest".into(),
                clock: clock(7),
            },
            decision: DecisionEvidence {
                event_id: "decision-event".into(),
                sequence: 7,
                command_id: "heater-command-1".into(),
                inference_event_id: "inference-event".into(),
                policy_identity: "policy-v1".into(),
                decision_digest: "decision-digest".into(),
                abstained: false,
                risk_budget_satisfied: true,
                clock: clock(8),
            },
        }
    }

    #[test]
    fn joins_partial_actuation_to_exact_thermal_episode_and_sensor_state() {
        let recipe = recipe(120);
        let challenge = make_challenge(&recipe);
        let causal = make_causal(&challenge);
        let receipt = verify_thermal_causal_execution_bridge(
            &challenge,
            &recipe,
            &registry(),
            &causal,
            &make_binding(&challenge),
            &TestCommitments,
        )
        .unwrap();
        assert_eq!(receipt.commanded_power_watts, q(100, 1));
        assert_eq!(receipt.realized_power_watts, q(50, 1));
        assert!(receipt.actuator_measurement_interval_contains_realized_power);
        assert!(receipt.pre_state_commitment_verified);
        assert!(receipt.post_state_commitment_verified);
        assert!(receipt.sensor_sample_commitment_verified);
        assert!(receipt.cross_module_binding_complete);
    }

    #[test]
    fn rejects_command_mismatch_and_actuator_interval_that_excludes_realized_power() {
        let recipe = recipe(120);
        let challenge = make_challenge(&recipe);
        let mut causal = make_causal(&challenge);
        causal.command.requested_value = 101_000;
        assert!(verify_thermal_causal_execution_bridge(
            &challenge,
            &recipe,
            &registry(),
            &causal,
            &make_binding(&challenge),
            &TestCommitments,
        )
        .is_err());

        causal = make_causal(&challenge);
        causal.actuator_readback.measured_value = 80_000;
        causal.actuator_readback.measurement_uncertainty = 100;
        assert!(verify_thermal_causal_execution_bridge(
            &challenge,
            &recipe,
            &registry(),
            &causal,
            &make_binding(&challenge),
            &TestCommitments,
        )
        .is_err());
    }

    #[test]
    fn rejects_state_and_sensor_commitment_substitution() {
        let recipe = recipe(120);
        let challenge = make_challenge(&recipe);
        let causal = make_causal(&challenge);
        let mut binding = make_binding(&challenge);
        binding.post_state_temperature = binding.post_state_temperature.checked_add(q(1, 1)).unwrap();
        assert!(verify_thermal_causal_execution_bridge(
            &challenge,
            &recipe,
            &registry(),
            &causal,
            &binding,
            &TestCommitments,
        )
        .is_err());

        let mut binding = make_binding(&challenge);
        binding.measured_post_state_temperature =
            binding.measured_post_state_temperature.checked_add(q(1, 1)).unwrap();
        assert!(verify_thermal_causal_execution_bridge(
            &challenge,
            &recipe,
            &registry(),
            &causal,
            &binding,
            &TestCommitments,
        )
        .is_err());
    }

    #[test]
    fn missing_device_acceptance_remains_unattributed_even_when_readback_matches() {
        let recipe = recipe(120);
        let challenge = make_challenge(&recipe);
        let mut causal = make_causal(&challenge);
        causal.acknowledgement.state = AcknowledgementState::Absent;
        causal.acknowledgement.echoed_value = None;
        causal.dispatch.state = DispatchState::Unknown;
        assert!(verify_thermal_causal_execution_bridge(
            &challenge,
            &recipe,
            &registry(),
            &causal,
            &make_binding(&challenge),
            &TestCommitments,
        )
        .is_err());
    }

    #[test]
    fn thermal_hazard_cannot_be_promoted_to_a_risk_budget_pass() {
        let mut recipe = recipe(120);
        recipe.initial_temperature_grid = vec![q(74, 1)];
        recipe.commanded_power_grid = vec![q(200, 1)];
        recipe.actuator_delivery_fraction_grid = vec![q(1, 1)];
        recipe.safe_temperature_grid = vec![q(75, 1)];
        let challenge = make_challenge(&recipe);
        assert!(!challenge.sealed_oracle.episode.remained_within_safe_envelope);
        let causal = make_causal(&challenge);
        assert!(verify_thermal_causal_execution_bridge(
            &challenge,
            &recipe,
            &registry(),
            &causal,
            &make_binding(&challenge),
            &TestCommitments,
        )
        .is_err());
    }

    #[test]
    fn rejects_wrong_command_identity_model_revision_or_sensor_event() {
        let recipe = recipe(120);
        let challenge = make_challenge(&recipe);
        let mut causal = make_causal(&challenge);
        causal.command.command_id = "other-command".into();
        assert!(verify_thermal_causal_execution_bridge(
            &challenge,
            &recipe,
            &registry(),
            &causal,
            &make_binding(&challenge),
            &TestCommitments,
        )
        .is_err());

        causal = make_causal(&challenge);
        let mut binding = make_binding(&challenge);
        binding.model_version = "other-model".into();
        assert!(verify_thermal_causal_execution_bridge(
            &challenge,
            &recipe,
            &registry(),
            &causal,
            &binding,
            &TestCommitments,
        )
        .is_err());

        let causal = make_causal(&challenge);
        let mut binding = make_binding(&challenge);
        binding.sensor_event_id = "other-sensor-event".into();
        assert!(verify_thermal_causal_execution_bridge(
            &challenge,
            &recipe,
            &registry(),
            &causal,
            &binding,
            &TestCommitments,
        )
        .is_err());
    }
}
