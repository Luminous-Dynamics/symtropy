//! Source-pinned FlyGym/NeuroMechFly telemetry and intervention evidence gate.
//!
//! This crate is the Rust-owned evidence boundary for a version-pinned upstream
//! simulator adapter. An upstream sidecar may collect measurements, but a trial
//! is not eligible for scientific scoring until this validator accepts its
//! source, asset, schema, clock, chronology and intervention-delivery receipts.
//!
//! All digests are caller-supplied SHA-256 values. This crate checks presence
//! and binds values together; it does not hash files or authenticate a source.

use std::collections::BTreeSet;

pub const REQUIRED_FLYGYM_VERSION: FlyGymVersion = FlyGymVersion {
    major: 2,
    minor: 1,
    patch: 1,
};

pub const LEG_COUNT: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlyGymVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationBackend {
    CpuMuJoCo,
    GpuWarp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LegId {
    LeftFront = 0,
    LeftMiddle = 1,
    LeftRear = 2,
    RightFront = 3,
    RightMiddle = 4,
    RightRear = 5,
}

impl LegId {
    pub const ALL: [Self; LEG_COUNT] = [
        Self::LeftFront,
        Self::LeftMiddle,
        Self::LeftRear,
        Self::RightFront,
        Self::RightMiddle,
        Self::RightRear,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Digest(pub [u8; 32]);

impl Digest {
    pub fn is_present(self) -> bool {
        self.0.iter().any(|byte| *byte != 0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlyGymRunManifest {
    pub flygym_version: FlyGymVersion,
    /// Full 40-character Git commit SHA for the exact source tree.
    pub source_commit: String,
    pub mujoco_version: String,
    pub warp_version: Option<String>,
    pub backend: SimulationBackend,
    pub physics_timestep_seconds: f64,
    pub control_period_seconds: f64,
    pub timestamp_tolerance_seconds: f64,
    pub maximum_intervention_latency_seconds: f64,
    pub solver_name: String,
    pub solver_tolerance: f64,
    /// Explicit units are required; never infer them from asset names.
    pub length_unit: String,
    pub force_unit: String,
    pub torque_unit: String,
    pub model_asset_digest: Digest,
    pub world_asset_digest: Digest,
    pub controller_digest: Digest,
    pub configuration_digest: Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointChannelDescriptor {
    pub channel_id: String,
    pub leg: LegId,
    pub axis: String,
    pub position_unit: String,
    pub velocity_unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActuatorChannelDescriptor {
    pub actuator_id: String,
    pub joint_channel_id: String,
    pub control_mode: String,
    pub input_unit: String,
    pub force_unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetrySchema {
    /// Order used by the source-pinned FlyGym adapter for all six-leg arrays.
    /// It may differ from Symtropy's canonical order but must be a permutation.
    pub leg_order: [LegId; LEG_COUNT],
    /// Position in the vector is the source's canonical index for that channel.
    pub joint_channels: Vec<JointChannelDescriptor>,
    pub actuator_channels: Vec<ActuatorChannelDescriptor>,
    pub schema_digest: Digest,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundContactTelemetry {
    pub contact_found: bool,
    /// Raw contact-frame force, in manifest.force_unit.
    pub force: [f64; 3],
    /// Raw contact-frame torque, in manifest.torque_unit.
    pub torque: [f64; 3],
    /// Raw global contact point, in manifest.length_unit.
    pub position: [f64; 3],
    /// Raw contact normal and tangent vectors in the source coordinate frame.
    pub normal: [f64; 3],
    pub tangent: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryFrame {
    pub timestamp_seconds: f64,
    pub joint_positions: Vec<f64>,
    pub joint_velocities: Vec<f64>,
    pub actuator_inputs: Vec<f64>,
    pub actuator_forces: Vec<f64>,
    /// Array order is exactly TelemetrySchema.leg_order.
    pub ground_contacts: [GroundContactTelemetry; LEG_COUNT],
    pub body_position: [f64; 3],
    /// Quaternion in explicitly declared x/y/z/w order.
    pub body_rotation_xyzw: [f64; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterventionAssignment {
    pub action_id: String,
    pub assigned_at_seconds: f64,
    pub target_legs: Vec<LegId>,
    /// IDs must refer to actuator IDs or joint-channel IDs in the schema.
    pub target_channels: Vec<String>,
    pub requested_value: f64,
    pub value_tolerance: f64,
    pub requested_duration_seconds: f64,
    pub duration_tolerance_seconds: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterventionDelivery {
    pub action_id: String,
    pub delivered_at_seconds: f64,
    pub actual_value: f64,
    pub actual_duration_seconds: f64,
    pub verified: bool,
    pub verification_receipt_digest: Digest,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterventionRecord {
    pub assignment: InterventionAssignment,
    /// None is not treated as successful delivery.
    pub delivery: Option<InterventionDelivery>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrialRecord {
    pub run_id: String,
    pub trial_id: String,
    pub random_seed: u64,
    pub initial_state_digest: Digest,
    pub manifest: FlyGymRunManifest,
    pub schema: TelemetrySchema,
    pub frames: Vec<TelemetryFrame>,
    pub interventions: Vec<InterventionRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValidatedTrialSummary {
    pub frame_count: usize,
    pub duration_seconds: f64,
    pub intervention_count: usize,
    pub contact_positive_samples: usize,
    pub source_version: FlyGymVersion,
    pub backend: SimulationBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceError {
    EmptyRunId,
    EmptyTrialId,
    MissingInitialStateDigest,
    UnsupportedFlyGymVersion,
    InvalidSourceCommit,
    MissingMujocoVersion,
    MissingWarpVersion,
    UnexpectedWarpVersion,
    InvalidPhysicsTimestep,
    InvalidControlPeriod,
    ControlPeriodNotPhysicsMultiple,
    InvalidTimestampTolerance,
    InvalidInterventionLatency,
    InvalidSolverName,
    InvalidSolverTolerance,
    MissingLengthUnit,
    MissingForceUnit,
    MissingTorqueUnit,
    MissingAssetDigest,
    MissingWorldDigest,
    MissingControllerDigest,
    MissingConfigurationDigest,
    MissingSchemaDigest,
    InvalidLegOrder,
    EmptyJointSchema,
    DuplicateJointChannel,
    InvalidJointDescriptor,
    MissingLegJointChannel,
    EmptyActuatorSchema,
    DuplicateActuatorChannel,
    InvalidActuatorDescriptor,
    UnknownActuatorJoint,
    EmptyTelemetry,
    InvalidTimestamp,
    NonMonotonicTimestamp,
    TimestampNotPhysicsAligned,
    JointPositionDimensionMismatch,
    JointVelocityDimensionMismatch,
    ActuatorInputDimensionMismatch,
    ActuatorForceDimensionMismatch,
    NonFiniteTelemetryValue,
    ZeroBodyQuaternion,
    DuplicateActionId,
    EmptyActionId,
    InvalidActionAssignment,
    DuplicateTargetLeg,
    DuplicateTargetChannel,
    UnknownTargetChannel,
    MissingDeliveryReceipt,
    DeliveryActionIdMismatch,
    UnverifiedDelivery,
    MissingDeliveryVerificationDigest,
    InvalidDeliveryTiming,
    InterventionLatencyExceeded,
    InvalidDeliveredValue,
    DeliveredValueOutsideTolerance,
    InvalidDeliveredDuration,
    DeliveredDurationOutsideTolerance,
}

impl FlyGymRunManifest {
    pub fn validate(&self) -> Result<(), EvidenceError> {
        if self.flygym_version != REQUIRED_FLYGYM_VERSION {
            return Err(EvidenceError::UnsupportedFlyGymVersion);
        }
        if !is_git_sha(&self.source_commit) {
            return Err(EvidenceError::InvalidSourceCommit);
        }
        if self.mujoco_version.trim().is_empty() {
            return Err(EvidenceError::MissingMujocoVersion);
        }
        match (self.backend, self.warp_version.as_deref()) {
            (SimulationBackend::GpuWarp, None) => return Err(EvidenceError::MissingWarpVersion),
            (SimulationBackend::GpuWarp, Some(version)) if version.trim().is_empty() => {
                return Err(EvidenceError::MissingWarpVersion);
            }
            (SimulationBackend::CpuMuJoCo, Some(_)) => {
                return Err(EvidenceError::UnexpectedWarpVersion);
            }
            _ => {}
        }
        if !self.physics_timestep_seconds.is_finite() || self.physics_timestep_seconds <= 0.0 {
            return Err(EvidenceError::InvalidPhysicsTimestep);
        }
        if !self.control_period_seconds.is_finite()
            || self.control_period_seconds < self.physics_timestep_seconds
        {
            return Err(EvidenceError::InvalidControlPeriod);
        }
        let ratio = self.control_period_seconds / self.physics_timestep_seconds;
        if !ratio.is_finite() || (ratio - ratio.round()).abs() > 1.0e-9 {
            return Err(EvidenceError::ControlPeriodNotPhysicsMultiple);
        }
        if !self.timestamp_tolerance_seconds.is_finite()
            || self.timestamp_tolerance_seconds < 0.0
            || self.timestamp_tolerance_seconds >= self.physics_timestep_seconds * 0.5
        {
            return Err(EvidenceError::InvalidTimestampTolerance);
        }
        if !self.maximum_intervention_latency_seconds.is_finite()
            || self.maximum_intervention_latency_seconds < 0.0
        {
            return Err(EvidenceError::InvalidInterventionLatency);
        }
        if self.solver_name.trim().is_empty() {
            return Err(EvidenceError::InvalidSolverName);
        }
        if !self.solver_tolerance.is_finite() || self.solver_tolerance <= 0.0 {
            return Err(EvidenceError::InvalidSolverTolerance);
        }
        if self.length_unit.trim().is_empty() {
            return Err(EvidenceError::MissingLengthUnit);
        }
        if self.force_unit.trim().is_empty() {
            return Err(EvidenceError::MissingForceUnit);
        }
        if self.torque_unit.trim().is_empty() {
            return Err(EvidenceError::MissingTorqueUnit);
        }
        if !self.model_asset_digest.is_present() {
            return Err(EvidenceError::MissingAssetDigest);
        }
        if !self.world_asset_digest.is_present() {
            return Err(EvidenceError::MissingWorldDigest);
        }
        if !self.controller_digest.is_present() {
            return Err(EvidenceError::MissingControllerDigest);
        }
        if !self.configuration_digest.is_present() {
            return Err(EvidenceError::MissingConfigurationDigest);
        }
        Ok(())
    }
}

impl TelemetrySchema {
    pub fn validate(&self) -> Result<(), EvidenceError> {
        if !self.schema_digest.is_present() {
            return Err(EvidenceError::MissingSchemaDigest);
        }
        let mut seen_legs = BTreeSet::new();
        for leg in self.leg_order {
            if !seen_legs.insert(leg) {
                return Err(EvidenceError::InvalidLegOrder);
            }
        }
        if self.joint_channels.is_empty() {
            return Err(EvidenceError::EmptyJointSchema);
        }
        let mut joint_ids = BTreeSet::new();
        let mut legs_with_joint_channels = BTreeSet::new();
        for channel in &self.joint_channels {
            if channel.channel_id.trim().is_empty()
                || channel.axis.trim().is_empty()
                || channel.position_unit.trim().is_empty()
                || channel.velocity_unit.trim().is_empty()
            {
                return Err(EvidenceError::InvalidJointDescriptor);
            }
            if !joint_ids.insert(channel.channel_id.as_str()) {
                return Err(EvidenceError::DuplicateJointChannel);
            }
            legs_with_joint_channels.insert(channel.leg);
        }
        if legs_with_joint_channels.len() != LEG_COUNT {
            return Err(EvidenceError::MissingLegJointChannel);
        }

        if self.actuator_channels.is_empty() {
            return Err(EvidenceError::EmptyActuatorSchema);
        }
        let mut actuator_ids = BTreeSet::new();
        for actuator in &self.actuator_channels {
            if actuator.actuator_id.trim().is_empty()
                || actuator.control_mode.trim().is_empty()
                || actuator.input_unit.trim().is_empty()
                || actuator.force_unit.trim().is_empty()
            {
                return Err(EvidenceError::InvalidActuatorDescriptor);
            }
            if !actuator_ids.insert(actuator.actuator_id.as_str()) {
                return Err(EvidenceError::DuplicateActuatorChannel);
            }
            if !joint_ids.contains(actuator.joint_channel_id.as_str()) {
                return Err(EvidenceError::UnknownActuatorJoint);
            }
        }
        Ok(())
    }

    fn accepts_target_channel(&self, channel_id: &str) -> bool {
        self.joint_channels
            .iter()
            .any(|channel| channel.channel_id == channel_id)
            || self
                .actuator_channels
                .iter()
                .any(|channel| channel.actuator_id == channel_id)
    }
}

impl TrialRecord {
    pub fn validate(&self) -> Result<ValidatedTrialSummary, EvidenceError> {
        if self.run_id.trim().is_empty() {
            return Err(EvidenceError::EmptyRunId);
        }
        if self.trial_id.trim().is_empty() {
            return Err(EvidenceError::EmptyTrialId);
        }
        if !self.initial_state_digest.is_present() {
            return Err(EvidenceError::MissingInitialStateDigest);
        }
        self.manifest.validate()?;
        self.schema.validate()?;
        if self.frames.is_empty() {
            return Err(EvidenceError::EmptyTelemetry);
        }

        let mut previous_timestamp = None;
        let mut contact_positive_samples = 0_usize;
        let joint_count = self.schema.joint_channels.len();
        let actuator_count = self.schema.actuator_channels.len();
        for frame in &self.frames {
            if !frame.timestamp_seconds.is_finite() || frame.timestamp_seconds < 0.0 {
                return Err(EvidenceError::InvalidTimestamp);
            }
            if previous_timestamp.is_some_and(|previous| frame.timestamp_seconds <= previous) {
                return Err(EvidenceError::NonMonotonicTimestamp);
            }
            let aligned_step =
                (frame.timestamp_seconds / self.manifest.physics_timestep_seconds).round();
            let aligned_time = aligned_step * self.manifest.physics_timestep_seconds;
            if (aligned_time - frame.timestamp_seconds).abs()
                > self.manifest.timestamp_tolerance_seconds
            {
                return Err(EvidenceError::TimestampNotPhysicsAligned);
            }
            previous_timestamp = Some(frame.timestamp_seconds);

            if frame.joint_positions.len() != joint_count {
                return Err(EvidenceError::JointPositionDimensionMismatch);
            }
            if frame.joint_velocities.len() != joint_count {
                return Err(EvidenceError::JointVelocityDimensionMismatch);
            }
            if frame.actuator_inputs.len() != actuator_count {
                return Err(EvidenceError::ActuatorInputDimensionMismatch);
            }
            if frame.actuator_forces.len() != actuator_count {
                return Err(EvidenceError::ActuatorForceDimensionMismatch);
            }

            if !frame.joint_positions.iter().all(|value| value.is_finite())
                || !frame.joint_velocities.iter().all(|value| value.is_finite())
                || !frame.actuator_inputs.iter().all(|value| value.is_finite())
                || !frame.actuator_forces.iter().all(|value| value.is_finite())
                || !frame.body_position.iter().all(|value| value.is_finite())
                || !frame.body_rotation_xyzw.iter().all(|value| value.is_finite())
            {
                return Err(EvidenceError::NonFiniteTelemetryValue);
            }
            if frame
                .body_rotation_xyzw
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                <= f64::EPSILON
            {
                return Err(EvidenceError::ZeroBodyQuaternion);
            }

            for contact in &frame.ground_contacts {
                if !contact.force.iter().all(|value| value.is_finite())
                    || !contact.torque.iter().all(|value| value.is_finite())
                    || !contact.position.iter().all(|value| value.is_finite())
                    || !contact.normal.iter().all(|value| value.is_finite())
                    || !contact.tangent.iter().all(|value| value.is_finite())
                {
                    return Err(EvidenceError::NonFiniteTelemetryValue);
                }
                if contact.contact_found {
                    contact_positive_samples += 1;
                }
            }
        }

        self.validate_interventions()?;

        let first = self.frames.first().ok_or(EvidenceError::EmptyTelemetry)?;
        let last = self.frames.last().ok_or(EvidenceError::EmptyTelemetry)?;
        let duration_seconds = last.timestamp_seconds - first.timestamp_seconds;
        if !duration_seconds.is_finite() || duration_seconds < 0.0 {
            return Err(EvidenceError::InvalidTimestamp);
        }

        Ok(ValidatedTrialSummary {
            frame_count: self.frames.len(),
            duration_seconds,
            intervention_count: self.interventions.len(),
            contact_positive_samples,
            source_version: self.manifest.flygym_version,
            backend: self.manifest.backend,
        })
    }

    fn validate_interventions(&self) -> Result<(), EvidenceError> {
        let mut action_ids = BTreeSet::new();
        let trial_end = self
            .frames
            .last()
            .ok_or(EvidenceError::EmptyTelemetry)?
            .timestamp_seconds;

        for intervention in &self.interventions {
            let assignment = &intervention.assignment;
            if assignment.action_id.trim().is_empty() {
                return Err(EvidenceError::EmptyActionId);
            }
            if !action_ids.insert(assignment.action_id.as_str()) {
                return Err(EvidenceError::DuplicateActionId);
            }
            if !assignment.assigned_at_seconds.is_finite()
                || assignment.assigned_at_seconds < 0.0
                || assignment.assigned_at_seconds > trial_end
                || !assignment.requested_value.is_finite()
                || !assignment.value_tolerance.is_finite()
                || assignment.value_tolerance < 0.0
                || !assignment.requested_duration_seconds.is_finite()
                || assignment.requested_duration_seconds <= 0.0
                || !assignment.duration_tolerance_seconds.is_finite()
                || assignment.duration_tolerance_seconds < 0.0
            {
                return Err(EvidenceError::InvalidActionAssignment);
            }
            if assignment.target_legs.is_empty() {
                return Err(EvidenceError::InvalidActionAssignment);
            }
            let mut target_legs = BTreeSet::new();
            for leg in &assignment.target_legs {
                if !target_legs.insert(*leg) {
                    return Err(EvidenceError::DuplicateTargetLeg);
                }
                if !self.schema.leg_order.contains(leg) {
                    return Err(EvidenceError::InvalidActionAssignment);
                }
            }

            let mut target_channels = BTreeSet::new();
            for channel_id in &assignment.target_channels {
                if channel_id.trim().is_empty() {
                    return Err(EvidenceError::InvalidActionAssignment);
                }
                if !target_channels.insert(channel_id.as_str()) {
                    return Err(EvidenceError::DuplicateTargetChannel);
                }
                if !self.schema.accepts_target_channel(channel_id) {
                    return Err(EvidenceError::UnknownTargetChannel);
                }
            }

            let delivery = intervention
                .delivery
                .as_ref()
                .ok_or(EvidenceError::MissingDeliveryReceipt)?;
            if delivery.action_id != assignment.action_id {
                return Err(EvidenceError::DeliveryActionIdMismatch);
            }
            if !delivery.verified {
                return Err(EvidenceError::UnverifiedDelivery);
            }
            if !delivery.verification_receipt_digest.is_present() {
                return Err(EvidenceError::MissingDeliveryVerificationDigest);
            }
            if !delivery.delivered_at_seconds.is_finite()
                || delivery.delivered_at_seconds < assignment.assigned_at_seconds
                || delivery.delivered_at_seconds > trial_end
                || !delivery.actual_value.is_finite()
                || !delivery.actual_duration_seconds.is_finite()
                || delivery.actual_duration_seconds <= 0.0
            {
                return Err(EvidenceError::InvalidDeliveryTiming);
            }
            let latency =
                delivery.delivered_at_seconds - assignment.assigned_at_seconds;
            if latency > self.manifest.maximum_intervention_latency_seconds {
                return Err(EvidenceError::InterventionLatencyExceeded);
            }
            if (delivery.actual_value - assignment.requested_value).abs()
                > assignment.value_tolerance
            {
                return Err(EvidenceError::DeliveredValueOutsideTolerance);
            }
            if (delivery.actual_duration_seconds - assignment.requested_duration_seconds).abs()
                > assignment.duration_tolerance_seconds
            {
                return Err(EvidenceError::DeliveredDurationOutsideTolerance);
            }
        }
        Ok(())
    }
}

fn is_git_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(seed: u8) -> Digest {
        Digest([seed; 32])
    }

    fn manifest() -> FlyGymRunManifest {
        FlyGymRunManifest {
            flygym_version: REQUIRED_FLYGYM_VERSION,
            source_commit: "a".repeat(40),
            mujoco_version: "3.3.6".to_owned(),
            warp_version: None,
            backend: SimulationBackend::CpuMuJoCo,
            physics_timestep_seconds: 0.001,
            control_period_seconds: 0.01,
            timestamp_tolerance_seconds: 1.0e-7,
            maximum_intervention_latency_seconds: 0.002,
            solver_name: "mujoco-default".to_owned(),
            solver_tolerance: 1.0e-8,
            length_unit: "mm".to_owned(),
            force_unit: "N".to_owned(),
            torque_unit: "N*m".to_owned(),
            model_asset_digest: digest(1),
            world_asset_digest: digest(2),
            controller_digest: digest(3),
            configuration_digest: digest(4),
        }
    }

    fn schema() -> TelemetrySchema {
        let joint_channels = LegId::ALL
            .iter()
            .map(|leg| JointChannelDescriptor {
                channel_id: format!("{leg:?}-coxa-pitch"),
                leg: *leg,
                axis: "pitch".to_owned(),
                position_unit: "rad".to_owned(),
                velocity_unit: "rad/s".to_owned(),
            })
            .collect::<Vec<_>>();
        let actuator_channels = LegId::ALL
            .iter()
            .map(|leg| {
                let channel_id = format!("{leg:?}-coxa-pitch");
                ActuatorChannelDescriptor {
                    actuator_id: format!("{leg:?}-coxa-motor"),
                    joint_channel_id: channel_id,
                    control_mode: "position".to_owned(),
                    input_unit: "rad".to_owned(),
                    force_unit: "N*m".to_owned(),
                }
            })
            .collect::<Vec<_>>();
        TelemetrySchema {
            leg_order: [
                LegId::LeftFront,
                LegId::RightFront,
                LegId::LeftMiddle,
                LegId::RightMiddle,
                LegId::LeftRear,
                LegId::RightRear,
            ],
            joint_channels,
            actuator_channels,
            schema_digest: digest(5),
        }
    }

    fn frame(timestamp: f64) -> TelemetryFrame {
        TelemetryFrame {
            timestamp_seconds: timestamp,
            joint_positions: vec![0.1; LEG_COUNT],
            joint_velocities: vec![0.2; LEG_COUNT],
            actuator_inputs: vec![0.3; LEG_COUNT],
            actuator_forces: vec![0.4; LEG_COUNT],
            ground_contacts: std::array::from_fn(|index| GroundContactTelemetry {
                contact_found: index % 2 == 0,
                force: [1.0, 0.0, 0.0],
                torque: [0.0, 0.1, 0.0],
                position: [index as f64, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
            }),
            body_position: [0.0, 0.0, 1.0],
            body_rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }

    fn valid_trial() -> TrialRecord {
        TrialRecord {
            run_id: "run-001".to_owned(),
            trial_id: "trial-001".to_owned(),
            random_seed: 42,
            initial_state_digest: digest(6),
            manifest: manifest(),
            schema: schema(),
            frames: vec![frame(0.0), frame(0.01), frame(0.02)],
            interventions: Vec::new(),
        }
    }

    fn valid_intervention() -> InterventionRecord {
        InterventionRecord {
            assignment: InterventionAssignment {
                action_id: "local-pulse-left-front".to_owned(),
                assigned_at_seconds: 0.01,
                target_legs: vec![LegId::LeftFront],
                target_channels: vec!["LeftFront-coxa-motor".to_owned()],
                requested_value: 0.5,
                value_tolerance: 0.02,
                requested_duration_seconds: 0.005,
                duration_tolerance_seconds: 0.001,
            },
            delivery: Some(InterventionDelivery {
                action_id: "local-pulse-left-front".to_owned(),
                delivered_at_seconds: 0.0105,
                actual_value: 0.49,
                actual_duration_seconds: 0.0055,
                verified: true,
                verification_receipt_digest: digest(7),
            }),
        }
    }

    #[test]
    fn accepts_complete_cpu_flygym_211_baseline_trial() {
        let trial = valid_trial();
        let report = trial.validate().unwrap();
        assert_eq!(report.frame_count, 3);
        assert!((report.duration_seconds - 0.02).abs() < 1.0e-12);
        assert_eq!(report.intervention_count, 0);
        assert_eq!(report.contact_positive_samples, 9);
        assert_eq!(report.source_version, REQUIRED_FLYGYM_VERSION);
        assert_eq!(report.backend, SimulationBackend::CpuMuJoCo);
    }

    #[test]
    fn accepts_gpu_trial_only_with_explicit_warp_version() {
        let mut trial = valid_trial();
        trial.manifest.backend = SimulationBackend::GpuWarp;
        trial.manifest.warp_version = Some("3.16.0".to_owned());
        assert!(trial.validate().is_ok());
        trial.manifest.warp_version = None;
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::MissingWarpVersion)
        );
    }

    #[test]
    fn accepts_verified_intervention_assignment_and_delivery_receipt() {
        let mut trial = valid_trial();
        trial.interventions.push(valid_intervention());
        let report = trial.validate().unwrap();
        assert_eq!(report.intervention_count, 1);
    }

    #[test]
    fn rejects_flygym_version_drift() {
        let mut trial = valid_trial();
        trial.manifest.flygym_version.patch = 0;
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::UnsupportedFlyGymVersion)
        );
    }

    #[test]
    fn rejects_non_sha_source_commit_and_missing_artifact_digest() {
        let mut trial = valid_trial();
        trial.manifest.source_commit = "main".to_owned();
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::InvalidSourceCommit)
        );

        trial = valid_trial();
        trial.manifest.model_asset_digest = Digest([0; 32]);
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::MissingAssetDigest)
        );
    }

    #[test]
    fn rejects_control_period_not_integral_multiple_of_physics_timestep() {
        let mut trial = valid_trial();
        trial.manifest.control_period_seconds = 0.0105;
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::ControlPeriodNotPhysicsMultiple)
        );
    }

    #[test]
    fn rejects_duplicate_leg_order_or_joint_channel() {
        let mut trial = valid_trial();
        trial.schema.leg_order[5] = trial.schema.leg_order[4];
        assert_eq!(trial.validate(), Err(EvidenceError::InvalidLegOrder));

        trial = valid_trial();
        trial.schema.joint_channels[1].channel_id =
            trial.schema.joint_channels[0].channel_id.clone();
        assert_eq!(trial.validate(), Err(EvidenceError::DuplicateJointChannel));
    }


    #[test]
    fn rejects_joint_channel_without_separate_velocity_unit() {
        let mut trial = valid_trial();
        trial.schema.joint_channels[0].velocity_unit.clear();
        assert_eq!(trial.validate(), Err(EvidenceError::InvalidJointDescriptor));
    }

    #[test]
    fn rejects_actuator_referencing_unknown_joint_channel() {
        let mut trial = valid_trial();
        trial.schema.actuator_channels[0].joint_channel_id = "not-a-joint".to_owned();
        assert_eq!(trial.validate(), Err(EvidenceError::UnknownActuatorJoint));
    }

    #[test]
    fn rejects_telemetry_vector_dimension_mismatch() {
        let mut trial = valid_trial();
        trial.frames[0].joint_positions.pop();
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::JointPositionDimensionMismatch)
        );

        trial = valid_trial();
        trial.frames[0].actuator_forces.pop();
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::ActuatorForceDimensionMismatch)
        );
    }

    #[test]
    fn rejects_nonmonotonic_or_non_physics_aligned_timestamps() {
        let mut trial = valid_trial();
        trial.frames[2].timestamp_seconds = trial.frames[1].timestamp_seconds;
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::NonMonotonicTimestamp)
        );

        trial = valid_trial();
        trial.frames[1].timestamp_seconds = 0.0105;
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::TimestampNotPhysicsAligned)
        );
    }

    #[test]
    fn rejects_nonfinite_sensors_and_zero_body_quaternion() {
        let mut trial = valid_trial();
        trial.frames[1].ground_contacts[0].force[1] = f64::NAN;
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::NonFiniteTelemetryValue)
        );

        trial = valid_trial();
        trial.frames[0].body_rotation_xyzw = [0.0; 4];
        assert_eq!(trial.validate(), Err(EvidenceError::ZeroBodyQuaternion));
    }

    #[test]
    fn rejects_intervention_without_delivery_or_with_unverified_delivery() {
        let mut trial = valid_trial();
        let mut intervention = valid_intervention();
        intervention.delivery = None;
        trial.interventions.push(intervention);
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::MissingDeliveryReceipt)
        );

        trial = valid_trial();
        let mut intervention = valid_intervention();
        intervention.delivery.as_mut().unwrap().verified = false;
        trial.interventions.push(intervention);
        assert_eq!(trial.validate(), Err(EvidenceError::UnverifiedDelivery));
    }

    #[test]
    fn rejects_delivery_latency_value_and_duration_mismatch() {
        let mut trial = valid_trial();
        let mut intervention = valid_intervention();
        intervention.delivery.as_mut().unwrap().delivered_at_seconds = 0.019;
        trial.interventions.push(intervention);
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::InterventionLatencyExceeded)
        );

        trial = valid_trial();
        let mut intervention = valid_intervention();
        intervention.delivery.as_mut().unwrap().actual_value = 0.9;
        trial.interventions.push(intervention);
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::DeliveredValueOutsideTolerance)
        );

        trial = valid_trial();
        let mut intervention = valid_intervention();
        intervention.delivery.as_mut().unwrap().actual_duration_seconds = 0.02;
        trial.interventions.push(intervention);
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::DeliveredDurationOutsideTolerance)
        );
    }

    #[test]
    fn rejects_duplicate_actions_unknown_channels_and_wrong_delivery_identity() {
        let mut trial = valid_trial();
        let intervention = valid_intervention();
        trial.interventions = vec![intervention.clone(), intervention];
        assert_eq!(trial.validate(), Err(EvidenceError::DuplicateActionId));

        trial = valid_trial();
        let mut intervention = valid_intervention();
        intervention.assignment.target_channels[0] = "unknown actuator".to_owned();
        trial.interventions.push(intervention);
        assert_eq!(trial.validate(), Err(EvidenceError::UnknownTargetChannel));

        trial = valid_trial();
        let mut intervention = valid_intervention();
        intervention.delivery.as_mut().unwrap().action_id = "different-action".to_owned();
        trial.interventions.push(intervention);
        assert_eq!(
            trial.validate(),
            Err(EvidenceError::DeliveryActionIdMismatch)
        );
    }
}
