//! Evidence-bound synthetic safety envelope for construction robotics.
//!
//! This crate emits review/mitigation/stop dispositions only. It has no
//! actuator, PLC, emergency-stop, reset, dispatch or safety-rated interface.

use std::collections::BTreeSet;

const CONFIDENCE_SCALE_PPM: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperatingMode {
    Autonomous,
    Supervised,
    Manual,
    Maintenance,
    Recovery,
    EmergencyStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyStopObservation {
    Active,
    Clear,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanPresence {
    Clear,
    Detected,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorState {
    PresentAndAuthorized,
    PresentButNotAuthorized,
    Absent,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyPolicyProfile {
    pub policy_id: String,
    pub policy_revision: String,
    pub issuer_id: String,
    pub standards_profile_id: String,
    pub standards_profile_revision: String,
    pub machine_family: String,
    pub machine_id: String,
    pub as_built_revision: String,
    pub tool_id: String,
    pub task_id: String,
    pub task_revision: String,
    pub site_id: String,
    pub site_revision: String,
    pub coordinate_frame: String,
    pub supported_domains: Vec<String>,
    pub allowed_modes: Vec<OperatingMode>,
    pub required_sensor_modalities: Vec<String>,
    pub minimum_independent_sensor_groups: usize,
    pub maximum_sensor_age_ticks: u64,
    pub minimum_sensor_confidence_ppm: u32,
    pub mitigation_sensor_confidence_ppm: u32,
    pub minimum_separation_mm: u64,
    pub mitigation_separation_mm: u64,
    pub mitigation_motion_limit: u64,
    pub hard_motion_limit: u64,
    pub motion_unit_id: String,
    pub operator_required: bool,
    pub valid_through_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorObservation {
    pub sensor_id: String,
    pub modality: String,
    /// Sensors sharing a power, clock, network or physical dependency may share a group.
    pub independence_group_id: String,
    pub observed_at_tick: u64,
    pub confidence_ppm: u32,
    pub healthy: bool,
    pub calibrated: bool,
    pub calibration_valid_through_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanProximityObservation {
    pub state: HumanPresence,
    pub sensor_id: String,
    pub distance_mm: Option<u64>,
    pub distance_uncertainty_mm: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSafetyObservation {
    pub machine_family: String,
    pub machine_id: String,
    pub as_built_revision: String,
    pub tool_id: String,
    pub task_id: String,
    pub task_revision: String,
    pub site_id: String,
    pub site_revision: String,
    pub coordinate_frame: String,
    pub operational_domain: String,
    pub policy_id: String,
    pub policy_revision: String,
    pub current_tick: u64,
    pub mode: OperatingMode,
    pub emergency_stop: EmergencyStopObservation,
    pub operator_state: OperatorState,
    pub motion_intensity: u64,
    pub motion_unit_id: String,
    pub human_proximity: HumanProximityObservation,
    pub sensors: Vec<SensorObservation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyDisposition {
    WithinSyntheticEnvelope,
    RequireProgressiveMitigation,
    HoldForHumanReview,
    StopRequiredBySyntheticPolicy,
    EvidenceInconsistent,
    UnsupportedODD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyReason {
    InvalidPolicyIdentity,
    InvalidThresholdOrdering,
    InvalidPolicySensorSet,
    PolicyExpired,
    PolicyBindingMismatch,
    MissingStandardsProfile,
    UnsupportedOperationalDomain,
    ModeNotAllowed,
    MaintenanceModeWithMotion,
    EmergencyStopActive,
    EmergencyStopUnknown,
    OperatorNotAuthorized,
    OperatorPresenceUnknown,
    DuplicateSensorIdentity,
    InvalidSensorIdentity,
    MissingRequiredModality,
    SensorUnhealthy,
    SensorUncalibrated,
    SensorCalibrationExpired,
    SensorStale,
    SensorFutureDated,
    SensorConfidenceInvalid,
    SensorConfidenceBelowMinimum,
    SensorConfidenceNeedsMitigation,
    InsufficientIndependentSensorGroups,
    HumanPresenceUnknown,
    HumanSensorMissing,
    HumanDistanceMissing,
    HumanDistanceUncertaintyMissing,
    HumanSeparationHardLimit,
    HumanSeparationNeedsMitigation,
    MotionUnitMismatch,
    MotionHardLimit,
    MotionNeedsMitigation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyEvaluation {
    pub disposition: SafetyDisposition,
    pub reasons: Vec<SafetyReason>,
    /// Indicates the observation reports an active external E-stop.
    /// This evaluator cannot clear or reset it.
    pub emergency_stop_observed_active: bool,
    /// Always false: this evaluator never produces actuator/control output.
    pub control_output_emitted: bool,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_policy_identity(policy: &SafetyPolicyProfile) -> bool {
    [
        policy.policy_id.as_str(),
        policy.policy_revision.as_str(),
        policy.issuer_id.as_str(),
        policy.machine_family.as_str(),
        policy.machine_id.as_str(),
        policy.as_built_revision.as_str(),
        policy.tool_id.as_str(),
        policy.task_id.as_str(),
        policy.task_revision.as_str(),
        policy.site_id.as_str(),
        policy.site_revision.as_str(),
        policy.coordinate_frame.as_str(),
        policy.motion_unit_id.as_str(),
    ]
    .iter()
    .all(|field| nonempty(field))
}

fn policy_binding_matches(
    policy: &SafetyPolicyProfile,
    observation: &RuntimeSafetyObservation,
) -> bool {
    policy.machine_family == observation.machine_family
        && policy.machine_id == observation.machine_id
        && policy.as_built_revision == observation.as_built_revision
        && policy.tool_id == observation.tool_id
        && policy.task_id == observation.task_id
        && policy.task_revision == observation.task_revision
        && policy.site_id == observation.site_id
        && policy.site_revision == observation.site_revision
        && policy.coordinate_frame == observation.coordinate_frame
        && policy.policy_id == observation.policy_id
        && policy.policy_revision == observation.policy_revision
}

fn validate_profile(policy: &SafetyPolicyProfile, reasons: &mut Vec<SafetyReason>) -> bool {
    let mut inconsistent = false;
    if !valid_policy_identity(policy) {
        reasons.push(SafetyReason::InvalidPolicyIdentity);
        inconsistent = true;
    }
    if policy.minimum_separation_mm >= policy.mitigation_separation_mm
        || policy.mitigation_motion_limit >= policy.hard_motion_limit
        || policy.minimum_sensor_confidence_ppm > policy.mitigation_sensor_confidence_ppm
        || policy.mitigation_sensor_confidence_ppm > CONFIDENCE_SCALE_PPM
        || policy.minimum_independent_sensor_groups == 0
        || policy.supported_domains.is_empty()
        || policy.allowed_modes.is_empty()
        || policy.required_sensor_modalities.is_empty()
    {
        reasons.push(SafetyReason::InvalidThresholdOrdering);
        inconsistent = true;
    }
    if !nonempty(&policy.standards_profile_id)
        || !nonempty(&policy.standards_profile_revision)
    {
        reasons.push(SafetyReason::MissingStandardsProfile);
        inconsistent = true;
    }
    let mut modalities = BTreeSet::new();
    if policy.required_sensor_modalities.iter().any(|value| {
        !nonempty(value) || !modalities.insert(value.as_str())
    }) {
        reasons.push(SafetyReason::InvalidPolicySensorSet);
        inconsistent = true;
    }
    inconsistent
}

/// Deterministic synthetic envelope evaluation. It does not issue a command.
#[must_use]
pub fn evaluate_safety_envelope(
    policy: &SafetyPolicyProfile,
    observation: &RuntimeSafetyObservation,
) -> SafetyEvaluation {
    let mut reasons = Vec::new();
    let mut stop = false;
    let mut inconsistent = validate_profile(policy, &mut reasons);
    let mut unsupported = false;
    let mut hold = false;
    let mut mitigate = false;

    if observation.emergency_stop == EmergencyStopObservation::Active
        || observation.mode == OperatingMode::EmergencyStop
    {
        reasons.push(SafetyReason::EmergencyStopActive);
        stop = true;
    } else if observation.emergency_stop == EmergencyStopObservation::Unknown {
        reasons.push(SafetyReason::EmergencyStopUnknown);
        hold = true;
    }

    if !policy_binding_matches(policy, observation) {
        reasons.push(SafetyReason::PolicyBindingMismatch);
        inconsistent = true;
    }
    if policy.valid_through_tick < observation.current_tick {
        reasons.push(SafetyReason::PolicyExpired);
        hold = true;
    }
    if !policy
        .supported_domains
        .iter()
        .any(|domain| domain == &observation.operational_domain)
    {
        reasons.push(SafetyReason::UnsupportedOperationalDomain);
        unsupported = true;
    }

    if !inconsistent && !unsupported {
        if !policy.allowed_modes.contains(&observation.mode) {
            reasons.push(SafetyReason::ModeNotAllowed);
            stop = true;
        }
        if observation.mode == OperatingMode::Maintenance && observation.motion_intensity > 0 {
            reasons.push(SafetyReason::MaintenanceModeWithMotion);
            stop = true;
        }
        if policy.operator_required {
            match observation.operator_state {
                OperatorState::PresentAndAuthorized => {}
                OperatorState::PresentButNotAuthorized | OperatorState::Absent => {
                    reasons.push(SafetyReason::OperatorNotAuthorized);
                    hold = true;
                }
                OperatorState::Unknown => {
                    reasons.push(SafetyReason::OperatorPresenceUnknown);
                    hold = true;
                }
            }
        }
        if observation.motion_unit_id != policy.motion_unit_id {
            reasons.push(SafetyReason::MotionUnitMismatch);
            inconsistent = true;
        } else if observation.motion_intensity > policy.hard_motion_limit {
            reasons.push(SafetyReason::MotionHardLimit);
            stop = true;
        } else if observation.motion_intensity > policy.mitigation_motion_limit {
            reasons.push(SafetyReason::MotionNeedsMitigation);
            mitigate = true;
        }

        let mut sensor_ids = BTreeSet::new();
        let mut modalities = BTreeSet::new();
        let mut independent_groups = BTreeSet::new();
        let mut usable_sensor_ids = BTreeSet::new();
        let mut confidence_below_minimum = false;
        let mut confidence_needs_mitigation = false;

        for sensor in &observation.sensors {
            if !nonempty(&sensor.sensor_id)
                || !nonempty(&sensor.modality)
                || !nonempty(&sensor.independence_group_id)
            {
                reasons.push(SafetyReason::InvalidSensorIdentity);
                inconsistent = true;
                continue;
            }
            if !sensor_ids.insert(sensor.sensor_id.as_str()) {
                reasons.push(SafetyReason::DuplicateSensorIdentity);
                inconsistent = true;
                continue;
            }
            if sensor.observed_at_tick > observation.current_tick {
                reasons.push(SafetyReason::SensorFutureDated);
                hold = true;
                continue;
            }
            if observation.current_tick - sensor.observed_at_tick
                > policy.maximum_sensor_age_ticks
            {
                reasons.push(SafetyReason::SensorStale);
                hold = true;
                continue;
            }
            if !sensor.healthy {
                reasons.push(SafetyReason::SensorUnhealthy);
                hold = true;
                continue;
            }
            if !sensor.calibrated {
                reasons.push(SafetyReason::SensorUncalibrated);
                hold = true;
                continue;
            }
            if sensor.calibration_valid_through_tick < observation.current_tick {
                reasons.push(SafetyReason::SensorCalibrationExpired);
                hold = true;
                continue;
            }
            if sensor.confidence_ppm > CONFIDENCE_SCALE_PPM {
                reasons.push(SafetyReason::SensorConfidenceInvalid);
                inconsistent = true;
                continue;
            }
            if sensor.confidence_ppm < policy.minimum_sensor_confidence_ppm {
                reasons.push(SafetyReason::SensorConfidenceBelowMinimum);
                confidence_below_minimum = true;
                hold = true;
                continue;
            }
            if sensor.confidence_ppm < policy.mitigation_sensor_confidence_ppm {
                reasons.push(SafetyReason::SensorConfidenceNeedsMitigation);
                confidence_needs_mitigation = true;
                mitigate = true;
            }
            usable_sensor_ids.insert(sensor.sensor_id.as_str());
            modalities.insert(sensor.modality.as_str());
            independent_groups.insert(sensor.independence_group_id.as_str());
        }

        for required in &policy.required_sensor_modalities {
            if !modalities.contains(required.as_str()) {
                reasons.push(SafetyReason::MissingRequiredModality);
                hold = true;
            }
        }
        if independent_groups.len() < policy.minimum_independent_sensor_groups {
            reasons.push(SafetyReason::InsufficientIndependentSensorGroups);
            hold = true;
        }

        match observation.human_proximity.state {
            HumanPresence::Unknown => {
                reasons.push(SafetyReason::HumanPresenceUnknown);
                hold = true;
            }
            HumanPresence::Clear => {
                if !usable_sensor_ids.contains(observation.human_proximity.sensor_id.as_str()) {
                    reasons.push(SafetyReason::HumanSensorMissing);
                    hold = true;
                }
            }
            HumanPresence::Detected => {
                if !usable_sensor_ids.contains(observation.human_proximity.sensor_id.as_str()) {
                    reasons.push(SafetyReason::HumanSensorMissing);
                    hold = true;
                }
                let (Some(distance), Some(uncertainty)) = (
                    observation.human_proximity.distance_mm,
                    observation.human_proximity.distance_uncertainty_mm,
                ) else {
                    if observation.human_proximity.distance_mm.is_none() {
                        reasons.push(SafetyReason::HumanDistanceMissing);
                    }
                    if observation.human_proximity.distance_uncertainty_mm.is_none() {
                        reasons.push(SafetyReason::HumanDistanceUncertaintyMissing);
                    }
                    hold = true;
                    // No inferred safe distance when either term is absent.
                    let _ = (confidence_below_minimum, confidence_needs_mitigation);
                    continue_evaluation();
                    // This helper is intentionally a no-op; the disposition is held below.
                    fn continue_evaluation() {}
                    // Continue is replaced by no control effect; remaining checks still execute.
                };
                if let (Some(distance), Some(uncertainty)) = (
                    observation.human_proximity.distance_mm,
                    observation.human_proximity.distance_uncertainty_mm,
                ) {
                    let lower_bound = distance.saturating_sub(uncertainty);
                    if lower_bound <= policy.minimum_separation_mm {
                        reasons.push(SafetyReason::HumanSeparationHardLimit);
                        stop = true;
                    } else if lower_bound <= policy.mitigation_separation_mm {
                        reasons.push(SafetyReason::HumanSeparationNeedsMitigation);
                        mitigate = true;
                    }
                }
            }
        }
    }

    let disposition = if stop {
        SafetyDisposition::StopRequiredBySyntheticPolicy
    } else if inconsistent {
        SafetyDisposition::EvidenceInconsistent
    } else if unsupported {
        SafetyDisposition::UnsupportedODD
    } else if hold {
        SafetyDisposition::HoldForHumanReview
    } else if mitigate {
        SafetyDisposition::RequireProgressiveMitigation
    } else {
        SafetyDisposition::WithinSyntheticEnvelope
    };

    SafetyEvaluation {
        disposition,
        reasons,
        emergency_stop_observed_active: observation.emergency_stop == EmergencyStopObservation::Active,
        control_output_emitted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> SafetyPolicyProfile {
        SafetyPolicyProfile {
            policy_id: "policy-robot-01".into(),
            policy_revision: "p3".into(),
            issuer_id: "qualified-safety-owner".into(),
            standards_profile_id: "synthetic-robot-profile".into(),
            standards_profile_revision: "2026-a".into(),
            machine_family: "mobile-manipulator".into(),
            machine_id: "robot-01".into(),
            as_built_revision: "asbuilt-7".into(),
            tool_id: "gripper-2".into(),
            task_id: "place-segment".into(),
            task_revision: "task-r5".into(),
            site_id: "tunnel-site-4".into(),
            site_revision: "survey-12".into(),
            coordinate_frame: "site-local-mm".into(),
            supported_domains: vec!["underground".into()],
            allowed_modes: vec![OperatingMode::Autonomous, OperatingMode::Supervised],
            required_sensor_modalities: vec!["lidar".into(), "vision".into()],
            minimum_independent_sensor_groups: 2,
            maximum_sensor_age_ticks: 5,
            minimum_sensor_confidence_ppm: 700_000,
            mitigation_sensor_confidence_ppm: 900_000,
            minimum_separation_mm: 500,
            mitigation_separation_mm: 1_000,
            mitigation_motion_limit: 500,
            hard_motion_limit: 1_000,
            motion_unit_id: "mm-per-second".into(),
            operator_required: true,
            valid_through_tick: 1_000,
        }
    }

    fn sensors() -> Vec<SensorObservation> {
        vec![
            SensorObservation {
                sensor_id: "lidar-1".into(),
                modality: "lidar".into(),
                independence_group_id: "sensor-power-a".into(),
                observed_at_tick: 98,
                confidence_ppm: 950_000,
                healthy: true,
                calibrated: true,
                calibration_valid_through_tick: 2_000,
            },
            SensorObservation {
                sensor_id: "camera-1".into(),
                modality: "vision".into(),
                independence_group_id: "sensor-power-b".into(),
                observed_at_tick: 99,
                confidence_ppm: 950_000,
                healthy: true,
                calibrated: true,
                calibration_valid_through_tick: 2_000,
            },
        ]
    }

    fn observation() -> RuntimeSafetyObservation {
        RuntimeSafetyObservation {
            machine_family: "mobile-manipulator".into(),
            machine_id: "robot-01".into(),
            as_built_revision: "asbuilt-7".into(),
            tool_id: "gripper-2".into(),
            task_id: "place-segment".into(),
            task_revision: "task-r5".into(),
            site_id: "tunnel-site-4".into(),
            site_revision: "survey-12".into(),
            coordinate_frame: "site-local-mm".into(),
            operational_domain: "underground".into(),
            policy_id: "policy-robot-01".into(),
            policy_revision: "p3".into(),
            current_tick: 100,
            mode: OperatingMode::Supervised,
            emergency_stop: EmergencyStopObservation::Clear,
            operator_state: OperatorState::PresentAndAuthorized,
            motion_intensity: 300,
            motion_unit_id: "mm-per-second".into(),
            human_proximity: HumanProximityObservation {
                state: HumanPresence::Clear,
                sensor_id: "lidar-1".into(),
                distance_mm: None,
                distance_uncertainty_mm: None,
            },
            sensors: sensors(),
        }
    }

    #[test]
    fn qualified_clear_observation_is_within_synthetic_envelope() {
        let result = evaluate_safety_envelope(&policy(), &observation());
        assert_eq!(result.disposition, SafetyDisposition::WithinSyntheticEnvelope);
        assert!(!result.control_output_emitted);
    }

    #[test]
    fn active_emergency_stop_always_requires_stop_and_is_never_reset() {
        let mut obs = observation();
        obs.emergency_stop = EmergencyStopObservation::Active;
        let result = evaluate_safety_envelope(&policy(), &obs);
        assert_eq!(result.disposition, SafetyDisposition::StopRequiredBySyntheticPolicy);
        assert!(result.emergency_stop_observed_active);
        assert!(!result.control_output_emitted);
    }

    #[test]
    fn unknown_emergency_stop_state_requires_human_review() {
        let mut obs = observation();
        obs.emergency_stop = EmergencyStopObservation::Unknown;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn hard_separation_limit_uses_uncertainty_lower_bound() {
        let mut obs = observation();
        obs.human_proximity = HumanProximityObservation {
            state: HumanPresence::Detected,
            sensor_id: "lidar-1".into(),
            distance_mm: Some(700),
            distance_uncertainty_mm: Some(250),
        };
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::StopRequiredBySyntheticPolicy
        );
    }

    #[test]
    fn soft_separation_limit_requests_progressive_mitigation() {
        let mut obs = observation();
        obs.human_proximity = HumanProximityObservation {
            state: HumanPresence::Detected,
            sensor_id: "lidar-1".into(),
            distance_mm: Some(1_100),
            distance_uncertainty_mm: Some(200),
        };
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::RequireProgressiveMitigation
        );
    }

    #[test]
    fn missing_distance_cannot_be_treated_as_clear() {
        let mut obs = observation();
        obs.human_proximity.state = HumanPresence::Detected;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn sensor_common_mode_groups_must_be_independent() {
        let mut obs = observation();
        obs.sensors[1].independence_group_id = obs.sensors[0].independence_group_id.clone();
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn missing_required_modality_requires_hold() {
        let mut obs = observation();
        obs.sensors.retain(|s| s.modality != "vision");
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn stale_sensor_requires_hold() {
        let mut obs = observation();
        obs.sensors[0].observed_at_tick = 90;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn future_sensor_timestamp_requires_hold() {
        let mut obs = observation();
        obs.sensors[0].observed_at_tick = 101;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn expired_calibration_requires_hold() {
        let mut obs = observation();
        obs.sensors[0].calibration_valid_through_tick = 99;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn uncalibrated_or_unhealthy_sensor_requires_hold() {
        let mut obs = observation();
        obs.sensors[0].healthy = false;
        obs.sensors[1].calibrated = false;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn insufficient_confidence_requests_hold() {
        let mut obs = observation();
        obs.sensors[0].confidence_ppm = 600_000;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn intermediate_confidence_requests_progressive_mitigation() {
        let mut obs = observation();
        obs.sensors[0].confidence_ppm = 800_000;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::RequireProgressiveMitigation
        );
    }

    #[test]
    fn confidence_above_scale_is_inconsistent() {
        let mut obs = observation();
        obs.sensors[0].confidence_ppm = 1_000_001;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn unsupported_domain_is_not_interpreted_as_safe() {
        let mut obs = observation();
        obs.operational_domain = "open-water".into();
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::UnsupportedODD
        );
    }

    #[test]
    fn machine_tool_task_site_or_policy_revision_mismatch_is_inconsistent() {
        let mut obs = observation();
        obs.tool_id = "breaker-head".into();
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn motion_soft_limit_requires_mitigation() {
        let mut obs = observation();
        obs.motion_intensity = 700;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::RequireProgressiveMitigation
        );
    }

    #[test]
    fn motion_hard_limit_requires_stop() {
        let mut obs = observation();
        obs.motion_intensity = 1_001;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::StopRequiredBySyntheticPolicy
        );
    }

    #[test]
    fn maintenance_mode_with_motion_requires_stop() {
        let mut obs = observation();
        obs.mode = OperatingMode::Maintenance;
        obs.motion_intensity = 1;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::StopRequiredBySyntheticPolicy
        );
    }

    #[test]
    fn mode_not_allowed_requires_stop() {
        let mut obs = observation();
        obs.mode = OperatingMode::Manual;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::StopRequiredBySyntheticPolicy
        );
    }

    #[test]
    fn operator_absence_or_non_authorization_requires_hold() {
        let mut obs = observation();
        obs.operator_state = OperatorState::Absent;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn expired_policy_requires_hold() {
        let mut p = policy();
        p.valid_through_tick = 99;
        assert_eq!(
            evaluate_safety_envelope(&p, &observation()).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn policy_threshold_ordering_must_be_valid() {
        let mut p = policy();
        p.minimum_separation_mm = 1_000;
        assert_eq!(
            evaluate_safety_envelope(&p, &observation()).disposition,
            SafetyDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn identical_inputs_replay_deterministically() {
        let p = policy();
        let o = observation();
        assert_eq!(
            evaluate_safety_envelope(&p, &o),
            evaluate_safety_envelope(&p, &o)
        );
    }

    #[test]
    fn definitive_stop_dominates_other_uncertainty() {
        let mut obs = observation();
        obs.emergency_stop = EmergencyStopObservation::Active;
        obs.sensors[0].healthy = false;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::StopRequiredBySyntheticPolicy
        );
    }

    #[test]
    fn unknown_human_presence_requires_hold() {
        let mut obs = observation();
        obs.human_proximity.state = HumanPresence::Unknown;
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::HoldForHumanReview
        );
    }

    #[test]
    fn motion_unit_mismatch_is_inconsistent() {
        let mut obs = observation();
        obs.motion_unit_id = "km-per-hour".into();
        assert_eq!(
            evaluate_safety_envelope(&policy(), &obs).disposition,
            SafetyDisposition::EvidenceInconsistent
        );
    }
}
