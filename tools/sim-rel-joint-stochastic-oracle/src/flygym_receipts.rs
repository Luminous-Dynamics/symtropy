//! Source-pinned observation receipts for FlyGym / NeuroMechFly 2.1.1.
//!
//! This module is an ingestion contract, not a simulator binding. It keeps
//! native streams, units, coordinate frames and clocks distinct until an
//! exact capture join has been verified. Raw IEEE-754 values are preserved
//! as bits; finite checks are performed before they can enter derived bins.

use std::collections::BTreeSet;

use super::locomotion::LegId;

/// Exact finite IEEE-754 value. Storing bits preserves signed zero and avoids
/// lossy decimal round-trips in a raw observation receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FiniteF64(u64);

impl FiniteF64 {
    pub fn new(value: f64) -> Result<Self, &'static str> {
        if value.is_finite() {
            Ok(Self(value.to_bits()))
        } else {
            Err("raw observation contains NaN or infinity")
        }
    }

    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    pub fn value(self) -> f64 {
        f64::from_bits(self.0)
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub fn is_finite(self) -> bool {
        self.value().is_finite()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlyGymBackend {
    Cpu,
    WarpGpu,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NeuroMechFlyMorphology {
    Standard,
    SeparateTrochanterFemur,
}

/// Run-level identity. Digest fields are syntax checked here; the upstream
/// custody/export process remains responsible for hashing the actual bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlyGymRunIdentity {
    pub package_version: String,
    pub source_commit: String,
    pub model_xml_sha256: String,
    pub assets_manifest_sha256: String,
    pub compiled_qpos_layout_sha256: String,
    pub anatomy_order_sha256: String,
    pub morphology: NeuroMechFlyMorphology,
    pub mujoco_version: String,
    pub mujoco_warp_version: Option<String>,
    pub backend: FlyGymBackend,
    /// Preserve the configured decimal spelling (for example "1e-5").
    pub solver_tolerance_decimal: String,
    pub timestep_ns: u64,
    pub simulation_time_origin_ns: u64,
    pub world_count: u32,
    pub control_rate_millihz: Option<u64>,
    pub physics_substeps: u32,
    /// Units/sign conventions are explicit model/export metadata, not inferred.
    pub ground_contact_force_unit: String,
    pub ground_contact_torque_unit: String,
    pub body_segment_force_unit: String,
    pub world_position_unit: String,
    pub world_frame_id: String,
    pub qpos_width: u32,
    pub mocap_body_count: u32,
}

impl FlyGymRunIdentity {
    pub fn validate(&self) -> Result<(), String> {
        if self.package_version != "2.1.1" {
            return Err("only the pinned FlyGym 2.1.1 API is accepted by this adapter".into());
        }
        if !is_hex_digest(&self.source_commit, 40) {
            return Err("source_commit must be a full 40-character hexadecimal commit".into());
        }
        for digest in [
            &self.model_xml_sha256,
            &self.assets_manifest_sha256,
            &self.compiled_qpos_layout_sha256,
            &self.anatomy_order_sha256,
        ] {
            if !is_hex_digest(digest, 64) {
                return Err("model and asset digests must be 64-character SHA-256 hex".into());
            }
        }
        let tolerance = self.solver_tolerance_decimal.parse::<f64>().ok();
        if self.mujoco_version.trim().is_empty()
            || tolerance.is_none_or(|value| !value.is_finite() || value <= 0.0)
            || self.timestep_ns == 0
            || self.world_count == 0
            || self.physics_substeps == 0
            || self.qpos_width == 0
            || self.control_rate_millihz.is_some_and(|rate| rate == 0)
            || self.ground_contact_force_unit.trim().is_empty()
            || self.ground_contact_torque_unit.trim().is_empty()
            || self.body_segment_force_unit.trim().is_empty()
            || self.world_position_unit.trim().is_empty()
            || self.world_frame_id.trim().is_empty()
        {
            return Err("simulator version, solver identity, timestep, substeps and qpos width are required".into());
        }
        if self.backend == FlyGymBackend::WarpGpu
            && self.mujoco_warp_version.as_deref().is_none_or(str::is_empty)
        {
            return Err("GPU runs must pin the MuJoCo Warp version".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegOrderEntry {
    pub flygym_index: u8,
    pub flygym_name: String,
    pub symtropy_leg: LegId,
}

/// Empty leg membership is allowed only with an explicit reason (e.g. a
/// central thorax segment or an unresolved anatomical association). Multiple
/// legs are allowed for genuinely shared structures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnatomicalMembership {
    pub legs: Vec<LegId>,
    pub unassigned_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedAnatomicalMapping {
    pub native_name: String,
    pub membership: AnatomicalMembership,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActuatorOrder {
    pub actuator_type: String,
    /// Exact order returned by fly.get_actuated_jointdofs_order(actuator_type).
    pub native_dof_names: Vec<String>,
    /// Explicit model/export units and sign convention; never inferred here.
    pub force_unit: String,
    pub sign_convention: String,
}

/// Exact order manifests are captured from the configured FlyGym model; they
/// must never be recreated from a presumed default leg/joint order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlyGymAnatomyOrder {
    pub legs: Vec<LegOrderEntry>,
    pub joint_dof_order: Vec<String>,
    pub body_segment_order: Vec<String>,
    pub actuator_orders: Vec<ActuatorOrder>,
    pub joint_dof_membership: Vec<NamedAnatomicalMapping>,
    pub body_segment_membership: Vec<NamedAnatomicalMapping>,
}

impl FlyGymAnatomyOrder {
    pub fn validate(&self) -> Result<(), String> {
        if self.legs.len() != 6 {
            return Err("FlyGym leg order must contain exactly six legs".into());
        }
        let mut leg_names = BTreeSet::new();
        let mut leg_indices = BTreeSet::new();
        let mut symtropy_legs = BTreeSet::new();
        for (expected_index, leg) in self.legs.iter().enumerate() {
            if leg.flygym_index as usize != expected_index
                || leg.flygym_name.trim().is_empty()
                || !leg_names.insert(leg.flygym_name.clone())
                || !leg_indices.insert(leg.flygym_index)
                || !symtropy_legs.insert(leg.symtropy_leg)
            {
                return Err("leg order must be contiguous, unique and explicitly mapped".into());
            }
        }
        validate_unique_names(&self.joint_dof_order, "joint DoF")?;
        validate_unique_names(&self.body_segment_order, "body segment")?;
        validate_named_mappings(
            &self.joint_dof_order,
            &self.joint_dof_membership,
            &symtropy_legs,
            "joint DoF",
        )?;
        validate_named_mappings(
            &self.body_segment_order,
            &self.body_segment_membership,
            &symtropy_legs,
            "body segment",
        )?;
        let mut actuator_types = BTreeSet::new();
        for actuator in &self.actuator_orders {
            if actuator.actuator_type.trim().is_empty()
                || actuator.force_unit.trim().is_empty()
                || actuator.sign_convention.trim().is_empty()
                || !actuator_types.insert(actuator.actuator_type.clone())
            {
                return Err("actuator types must be non-empty and unique".into());
            }
            validate_unique_names(&actuator.native_dof_names, "actuator DoF")?;
            if actuator.native_dof_names.is_empty() {
                return Err("each declared actuator type needs an explicit order".into());
            }
            for name in &actuator.native_dof_names {
                if !self.joint_dof_order.contains(name) {
                    return Err(format!("actuator DoF {name} is absent from the joint order"));
                }
            }
        }
        Ok(())
    }

    pub fn actuator_width(&self, actuator_type: &str) -> Option<usize> {
        self.actuator_orders
            .iter()
            .find(|entry| entry.actuator_type == actuator_type)
            .map(|entry| entry.native_dof_names.len())
    }
}

fn validate_unique_names(names: &[String], kind: &str) -> Result<(), String> {
    if names.iter().any(|name| name.trim().is_empty()) {
        return Err(format!("{kind} names cannot be empty"));
    }
    let unique: BTreeSet<_> = names.iter().collect();
    if unique.len() != names.len() {
        return Err(format!("{kind} names must be unique"));
    }
    Ok(())
}

fn validate_named_mappings(
    exact_order: &[String],
    mappings: &[NamedAnatomicalMapping],
    valid_legs: &BTreeSet<LegId>,
    kind: &str,
) -> Result<(), String> {
    if exact_order.len() != mappings.len() {
        return Err(format!("every {kind} in the native order needs exactly one membership record"));
    }
    let mut mapped = BTreeSet::new();
    for mapping in mappings {
        if !exact_order.contains(&mapping.native_name) || !mapped.insert(mapping.native_name.clone()) {
            return Err(format!("unknown or duplicate {kind} mapping: {}", mapping.native_name));
        }
        let mut legs = BTreeSet::new();
        if mapping.membership.legs.iter().any(|leg| {
            !valid_legs.contains(leg) || !legs.insert(*leg)
        }) {
            return Err(format!("{kind} membership contains a duplicate or undeclared leg"));
        }
        match (mapping.membership.legs.is_empty(), &mapping.membership.unassigned_reason) {
            (true, Some(reason)) if !reason.trim().is_empty() => {}
            (true, _) => return Err(format!("unassigned {kind} requires an explicit missingness reason")),
            (false, None) => {}
            (false, Some(_)) => return Err(format!("mapped {kind} cannot also declare an unassigned reason")),
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationStream {
    TrajectoryQpos {
        output_fps: u32,
        playback_speed_millis: u32,
    },
    TrajectoryMocapPosition,
    TrajectoryMocapQuaternion,
    JointAnglesRadians,
    JointVelocitiesRadiansPerSecond,
    GroundContactFound,
    GroundContactForceContactFrame,
    GroundContactTorqueContactFrame,
    GroundContactPositionWorld,
    GroundContactNormalWorld,
    GroundContactTangentWorld,
    ActuatorForces {
        actuator_type: String,
    },
    BodySegmentContactForcesWorld {
        requested_segments: Vec<String>,
    },
}

impl ObservationStream {
    fn is_trajectory(&self) -> bool {
        matches!(
            self,
            Self::TrajectoryQpos { .. }
                | Self::TrajectoryMocapPosition
                | Self::TrajectoryMocapQuaternion
        )
    }

    fn expected_shape(
        &self,
        identity: &FlyGymRunIdentity,
        anatomy: &FlyGymAnatomyOrder,
    ) -> Result<Vec<u32>, String> {
        let width = match self {
            Self::TrajectoryQpos {
                output_fps,
                playback_speed_millis,
            } if *output_fps > 0 && *playback_speed_millis > 0 => {
                return Ok(vec![identity.qpos_width]);
            }
            Self::TrajectoryQpos { .. } => {
                return Err("trajectory output_fps and playback speed must be positive".into());
            }
            Self::TrajectoryMocapPosition => {
                if identity.mocap_body_count == 0 {
                    return Err("mocap position stream declared for a model with no mocap bodies".into());
                }
                return Ok(vec![identity.mocap_body_count, 3]);
            }
            Self::TrajectoryMocapQuaternion => {
                if identity.mocap_body_count == 0 {
                    return Err("mocap quaternion stream declared for a model with no mocap bodies".into());
                }
                return Ok(vec![identity.mocap_body_count, 4]);
            }
            Self::JointAnglesRadians | Self::JointVelocitiesRadiansPerSecond => {
                return Ok(vec![anatomy.joint_dof_order.len() as u32]);
            }
            Self::GroundContactFound => return Ok(vec![6]),
            Self::GroundContactForceContactFrame
            | Self::GroundContactTorqueContactFrame
            | Self::GroundContactPositionWorld
            | Self::GroundContactNormalWorld
            | Self::GroundContactTangentWorld => return Ok(vec![6, 3]),
            Self::ActuatorForces { actuator_type } => anatomy
                .actuator_width(actuator_type)
                .ok_or_else(|| format!("actuator type {actuator_type} is absent from the pinned order"))?,
            Self::BodySegmentContactForcesWorld { requested_segments } => {
                if requested_segments.is_empty() {
                    return Err("body-segment force query must declare its requested order".into());
                }
                validate_unique_names(requested_segments, "requested body segment")?;
                if requested_segments
                    .iter()
                    .any(|name| !anatomy.body_segment_order.contains(name))
                {
                    return Err("body-segment force query contains an unknown segment".into());
                }
                return Ok(vec![requested_segments.len() as u32, 3]);
            }
        };
        Ok(vec![width as u32])
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleClock {
    /// A physics-step aligned getter/callback sample.
    PhysicsStep {
        step_index: u64,
    },
    /// A video-frame trajectory sample. It can only join a physics stream if
    /// the exporter also supplies the exact corresponding physics step.
    TrajectoryFrame {
        frame_index: u64,
        output_fps: u32,
        matched_physics_step: Option<u64>,
    },
    /// An instantaneous API getter sample. Without a step identity it remains
    /// a valid raw observation but cannot be joined to another stream.
    DirectGetter {
        capture_sequence: u64,
        matched_physics_step: Option<u64>,
    },
}

impl SampleClock {
    pub fn matched_physics_step(self) -> Option<u64> {
        match self {
            Self::PhysicsStep { step_index } => Some(step_index),
            Self::TrajectoryFrame {
                matched_physics_step,
                ..
            }
            | Self::DirectGetter {
                matched_physics_step,
                ..
            } => matched_physics_step,
        }
    }

    pub fn validate_for(&self, stream: &ObservationStream) -> Result<(), String> {
        match self {
            Self::TrajectoryFrame { output_fps, .. } if *output_fps > 0 => match stream {
                ObservationStream::TrajectoryQpos {
                    output_fps: stream_fps,
                    playback_speed_millis,
                } if stream_fps == output_fps && *playback_speed_millis > 0 => Ok(()),
                ObservationStream::TrajectoryMocapPosition
                | ObservationStream::TrajectoryMocapQuaternion => Ok(()),
                _ => Err("trajectory frame clock does not match the declared trajectory stream".into()),
            },
            Self::TrajectoryFrame { .. } => {
                Err("trajectory frame clock requires positive output_fps".into())
            }
            Self::PhysicsStep { .. } | Self::DirectGetter { .. } if stream.is_trajectory() => {
                Err("trajectory streams require a trajectory-frame clock".into())
            }
            Self::PhysicsStep { .. } | Self::DirectGetter { .. } => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SamplePayload {
    Values {
        shape: Vec<u32>,
        values: Vec<FiniteF64>,
        raw_payload_sha256: String,
    },
    Missing {
        expected_shape: Vec<u32>,
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlyGymObservationSample {
    pub sample_id: String,
    pub capture_id: String,
    pub world_id: u32,
    /// Simulation time in integer nanoseconds from the simulator time origin.
    pub simulation_time_ns: u64,
    pub clock: SampleClock,
    pub stream: ObservationStream,
    pub payload: SamplePayload,
}

impl FlyGymObservationSample {
    pub fn validate(
        &self,
        identity: &FlyGymRunIdentity,
        anatomy: &FlyGymAnatomyOrder,
    ) -> Result<(), String> {
        if self.sample_id.trim().is_empty() || self.capture_id.trim().is_empty() {
            return Err("sample_id and capture_id are required".into());
        }
        if self.world_id >= identity.world_count {
            return Err("sample world_id is outside the pinned run world count".into());
        }
        self.clock.validate_for(&self.stream)?;
        if let Some(step) = self.clock.matched_physics_step() {
            let elapsed = step
                .checked_mul(identity.timestep_ns)
                .ok_or_else(|| "physics-step time calculation overflow".to_string())?;
            let expected_time = identity
                .simulation_time_origin_ns
                .checked_add(elapsed)
                .ok_or_else(|| "simulation-time calculation overflow".to_string())?;
            if self.simulation_time_ns != expected_time {
                return Err("sample time does not match its declared physics step and pinned timestep".into());
            }
        }
        let expected = self.stream.expected_shape(identity, anatomy)?;
        match &self.payload {
            SamplePayload::Values {
                shape,
                values,
                raw_payload_sha256,
            } => {
                if shape != &expected {
                    return Err(format!("shape mismatch: expected {expected:?}, got {shape:?}"));
                }
                if !is_hex_digest(raw_payload_sha256, 64) {
                    return Err("raw payload digest must be SHA-256 hex".into());
                }
                let count = shape.iter().try_fold(1_u64, |product, dim| {
                    product.checked_mul(*dim as u64)
                }).ok_or_else(|| "payload shape element count overflow".to_string())?;
                if count != values.len() as u64 {
                    return Err("raw payload value count does not match its declared shape".into());
                }
                if values.iter().any(|value| !value.is_finite()) {
                    return Err("raw payload contains a non-finite value".into());
                }
            }
            SamplePayload::Missing {
                expected_shape,
                reason,
            } => {
                if expected_shape != &expected || reason.trim().is_empty() {
                    return Err("missing payload must declare the expected shape and a reason".into());
                }
            }
        }
        Ok(())
    }
}

/// One run envelope plus its immutable order manifests and synchronized raw
/// samples. This is intentionally separate from the finite JointKernel: raw
/// continuous values are never silently quantized during ingestion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlyGymObservationBundle {
    pub run_id: String,
    pub identity: FlyGymRunIdentity,
    pub anatomy: FlyGymAnatomyOrder,
    pub samples: Vec<FlyGymObservationSample>,
}

impl FlyGymObservationBundle {
    pub fn validate(&self) -> Result<(), String> {
        if self.run_id.trim().is_empty() {
            return Err("run_id is required".into());
        }
        self.identity.validate()?;
        self.anatomy.validate()?;
        let mut sample_ids = BTreeSet::new();
        for sample in &self.samples {
            if !sample_ids.insert(sample.sample_id.clone()) {
                return Err(format!("duplicate sample ID: {}", sample.sample_id));
            }
            sample.validate(&self.identity, &self.anatomy)?;
        }
        Ok(())
    }

    /// Validate a synchronized join across multiple streams. A timestamp
    /// alone is insufficient: all streams must also share a non-null physics
    /// step, capture ID, world ID and run ID.
    pub fn validate_capture_join<'a>(
        &'a self,
        sample_ids: &[&str],
    ) -> Result<Vec<&'a FlyGymObservationSample>, String> {
        if sample_ids.len() < 2 {
            return Err("a cross-stream join requires at least two samples".into());
        }
        let mut selected = Vec::with_capacity(sample_ids.len());
        let mut seen_streams = BTreeSet::new();
        for sample_id in sample_ids {
            let sample = self.samples
                .iter()
                .find(|sample| sample.sample_id.as_str() == *sample_id)
                .ok_or_else(|| format!("unknown sample ID: {sample_id}"))?;
            if !seen_streams.insert(stream_discriminator(&sample.stream)) {
                return Err("a capture join cannot include the same stream twice".into());
            }
            selected.push(sample);
        }
        let first = selected[0];
        let first_step = first.clock.matched_physics_step()
            .ok_or_else(|| "sample has no verified physics-step mapping".to_string())?;
        for sample in selected.iter().skip(1) {
            if sample.world_id != first.world_id
                || sample.capture_id != first.capture_id
                || sample.simulation_time_ns != first.simulation_time_ns
                || sample.clock.matched_physics_step() != Some(first_step)
            {
                return Err("cross-stream samples are not synchronized to the same world/time/physics step".into());
            }
        }
        Ok(selected)
    }
}

fn stream_discriminator(stream: &ObservationStream) -> String {
    match stream {
        ObservationStream::TrajectoryQpos { .. } => "trajectory_qpos".into(),
        ObservationStream::TrajectoryMocapPosition => "trajectory_mocap_position".into(),
        ObservationStream::TrajectoryMocapQuaternion => "trajectory_mocap_quaternion".into(),
        ObservationStream::JointAnglesRadians => "joint_angles".into(),
        ObservationStream::JointVelocitiesRadiansPerSecond => "joint_velocities".into(),
        ObservationStream::GroundContactFound => "ground_contact_found".into(),
        ObservationStream::GroundContactForceContactFrame => "ground_contact_force_contact_frame".into(),
        ObservationStream::GroundContactTorqueContactFrame => "ground_contact_torque_contact_frame".into(),
        ObservationStream::GroundContactPositionWorld => "ground_contact_position_world".into(),
        ObservationStream::GroundContactNormalWorld => "ground_contact_normal_world".into(),
        ObservationStream::GroundContactTangentWorld => "ground_contact_tangent_world".into(),
        ObservationStream::ActuatorForces { actuator_type } => format!("actuator_forces:{actuator_type}"),
        ObservationStream::BodySegmentContactForcesWorld { requested_segments } => {
            format!("body_segment_contact_forces:{}", requested_segments.join("|"))
        }
    }
}

fn is_hex_digest(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> FlyGymRunIdentity {
        FlyGymRunIdentity {
            package_version: "2.1.1".into(),
            source_commit: "a".repeat(40),
            model_xml_sha256: "b".repeat(64),
            assets_manifest_sha256: "c".repeat(64),
            compiled_qpos_layout_sha256: "d".repeat(64),
            anatomy_order_sha256: "f".repeat(64),
            morphology: NeuroMechFlyMorphology::SeparateTrochanterFemur,
            mujoco_version: "3.16.0".into(),
            mujoco_warp_version: Some("3.16.0".into()),
            backend: FlyGymBackend::WarpGpu,
            solver_tolerance_decimal: "1e-5".into(),
            timestep_ns: 100_000,
            simulation_time_origin_ns: 0,
            world_count: 1,
            control_rate_millihz: Some(50_000),
            physics_substeps: 1,
            ground_contact_force_unit: "declared-model-force-unit".into(),
            ground_contact_torque_unit: "declared-model-torque-unit".into(),
            body_segment_force_unit: "declared-model-force-unit".into(),
            world_position_unit: "declared-model-length-unit".into(),
            world_frame_id: "mujoco-world".into(),
            qpos_width: 73,
            mocap_body_count: 0,
        }
    }

    fn anatomy() -> FlyGymAnatomyOrder {
        let names = [
            "LF".to_string(), "LM".to_string(), "LH".to_string(),
            "RF".to_string(), "RM".to_string(), "RH".to_string(),
        ];
        let legs = names.iter().enumerate().map(|(index, name)| LegOrderEntry {
            flygym_index: index as u8,
            flygym_name: name.clone(),
            symtropy_leg: LegId::ALL[index],
        }).collect();
        let joint_dof_order = vec!["lf_coxa".into(), "rf_coxa".into()];
        let body_segment_order = vec!["lf_tibia".into(), "rf_tibia".into(), "c_thorax".into()];
        let actuator_orders = vec![ActuatorOrder {
            actuator_type: "motor".into(),
            native_dof_names: joint_dof_order.clone(),
            force_unit: "declared-model-force-unit".into(),
            sign_convention: "positive follows native actuator coordinate".into(),
        }];
        let joint_dof_membership = vec![
            NamedAnatomicalMapping {
                native_name: "lf_coxa".into(),
                membership: AnatomicalMembership { legs: vec![LegId::FrontLeft], unassigned_reason: None },
            },
            NamedAnatomicalMapping {
                native_name: "rf_coxa".into(),
                membership: AnatomicalMembership { legs: vec![LegId::FrontRight], unassigned_reason: None },
            },
        ];
        let body_segment_membership = vec![
            NamedAnatomicalMapping {
                native_name: "lf_tibia".into(),
                membership: AnatomicalMembership { legs: vec![LegId::FrontLeft], unassigned_reason: None },
            },
            NamedAnatomicalMapping {
                native_name: "rf_tibia".into(),
                membership: AnatomicalMembership { legs: vec![LegId::FrontRight], unassigned_reason: None },
            },
            NamedAnatomicalMapping {
                native_name: "c_thorax".into(),
                membership: AnatomicalMembership { legs: vec![], unassigned_reason: Some("central non-leg segment".into()) },
            },
        ];
        FlyGymAnatomyOrder {
            legs,
            joint_dof_order,
            body_segment_order,
            actuator_orders,
            joint_dof_membership,
            body_segment_membership,
        }
    }

    fn sample(stream: ObservationStream, clock: SampleClock, shape: Vec<u32>, len: usize, id: &str) -> FlyGymObservationSample {
        FlyGymObservationSample {
            sample_id: id.into(),
            capture_id: "capture-1".into(),
            world_id: 0,
            simulation_time_ns: 1_000_000,
            clock,
            stream,
            payload: SamplePayload::Values {
                shape,
                values: vec![FiniteF64::new(0.25).unwrap(); len],
                raw_payload_sha256: "e".repeat(64),
            },
        }
    }

    #[test]
    fn accepts_pinned_six_leg_ground_contact_and_joint_streams() {
        let bundle = FlyGymObservationBundle {
            run_id: "run-1".into(),
            identity: identity(),
            anatomy: anatomy(),
            samples: vec![
                sample(
                    ObservationStream::GroundContactForceContactFrame,
                    SampleClock::PhysicsStep { step_index: 10 },
                    vec![6, 3],
                    18,
                    "contact-force-10",
                ),
                sample(
                    ObservationStream::JointAnglesRadians,
                    SampleClock::PhysicsStep { step_index: 10 },
                    vec![2],
                    2,
                    "joint-angle-10",
                ),
            ],
        };
        assert!(bundle.validate().is_ok());
        assert_eq!(
            bundle.validate_capture_join(&["contact-force-10", "joint-angle-10"]).unwrap().len(),
            2
        );
    }

    #[test]
    fn rejects_video_frame_join_without_verified_physics_step() {
        let bundle = FlyGymObservationBundle {
            run_id: "run-1".into(),
            identity: identity(),
            anatomy: anatomy(),
            samples: vec![
                sample(
                    ObservationStream::TrajectoryQpos { output_fps: 25, playback_speed_millis: 200 },
                    SampleClock::TrajectoryFrame { frame_index: 5, output_fps: 25, matched_physics_step: None },
                    vec![73],
                    73,
                    "qpos-frame-5",
                ),
                sample(
                    ObservationStream::GroundContactFound,
                    SampleClock::PhysicsStep { step_index: 10 },
                    vec![6],
                    6,
                    "contact-step-10",
                ),
            ],
        };
        assert!(bundle.validate().is_ok());
        assert!(bundle.validate_capture_join(&["qpos-frame-5", "contact-step-10"]).is_err());
    }

    #[test]
    fn accepts_qpos_join_only_when_exporter_pins_exact_physics_step() {
        let bundle = FlyGymObservationBundle {
            run_id: "run-1".into(),
            identity: identity(),
            anatomy: anatomy(),
            samples: vec![
                sample(
                    ObservationStream::TrajectoryQpos { output_fps: 25, playback_speed_millis: 200 },
                    SampleClock::TrajectoryFrame { frame_index: 5, output_fps: 25, matched_physics_step: Some(10) },
                    vec![73],
                    73,
                    "qpos-frame-5",
                ),
                sample(
                    ObservationStream::GroundContactFound,
                    SampleClock::PhysicsStep { step_index: 10 },
                    vec![6],
                    6,
                    "contact-step-10",
                ),
            ],
        };
        assert!(bundle.validate_capture_join(&["qpos-frame-5", "contact-step-10"]).is_ok());
    }

    #[test]
    fn trajectory_clock_must_match_recorded_output_fps() {
        let mut sample = sample(
            ObservationStream::TrajectoryQpos {
                output_fps: 25,
                playback_speed_millis: 200,
            },
            SampleClock::TrajectoryFrame {
                frame_index: 5,
                output_fps: 30,
                matched_physics_step: None,
            },
            vec![73],
            73,
            "qpos-fps-mismatch",
        );
        assert!(sample.validate(&identity(), &anatomy()).is_err());
        sample.clock = SampleClock::TrajectoryFrame {
            frame_index: 5,
            output_fps: 25,
            matched_physics_step: None,
        };
        assert!(sample.validate(&identity(), &anatomy()).is_ok());
    }

    #[test]
    fn rejects_inconsistent_step_time_and_out_of_range_world() {
        let mut sample = sample(
            ObservationStream::JointAnglesRadians,
            SampleClock::PhysicsStep { step_index: 10 },
            vec![2],
            2,
            "joint-step-time",
        );
        sample.simulation_time_ns += 1;
        assert!(sample.validate(&identity(), &anatomy()).is_err());
        sample.simulation_time_ns -= 1;
        sample.world_id = 1;
        assert!(sample.validate(&identity(), &anatomy()).is_err());
    }

    #[test]
    fn actuator_force_receipts_require_a_pinned_type_unit_and_sign_order() {
        let mut map = anatomy();
        map.actuator_orders[0].force_unit.clear();
        assert!(map.validate().is_err());
        map.actuator_orders[0].force_unit = "declared-model-force-unit".into();
        map.actuator_orders[0].sign_convention.clear();
        assert!(map.validate().is_err());
    }

    #[test]
    fn shared_and_unassigned_anatomy_are_explicit_not_forced_into_one_leg() {
        let mut map = anatomy();
        map.body_segment_membership[2].membership = AnatomicalMembership {
            legs: vec![LegId::FrontLeft, LegId::FrontRight],
            unassigned_reason: None,
        };
        assert!(map.validate().is_ok());
        map.body_segment_membership[2].membership = AnatomicalMembership {
            legs: vec![],
            unassigned_reason: Some("central shared thorax".into()),
        };
        assert!(map.validate().is_ok());
        map.body_segment_membership[2].membership.unassigned_reason = None;
        assert!(map.validate().is_err());
    }

    #[test]
    fn rejects_changed_leg_order_or_duplicate_anatomical_names() {
        let mut map = anatomy();
        map.legs.swap(0, 1);
        assert!(map.validate().is_err());
        let mut map = anatomy();
        map.joint_dof_order.push("lf_coxa".into());
        assert!(map.validate().is_err());
    }

    #[test]
    fn rejects_nonfinite_raw_values_and_shape_mismatch() {
        assert!(FiniteF64::new(f64::NAN).is_err());
        assert!(FiniteF64::new(f64::INFINITY).is_err());
        let mut sample = sample(
            ObservationStream::GroundContactFound,
            SampleClock::PhysicsStep { step_index: 1 },
            vec![6],
            5,
            "contact-found-1",
        );
        assert!(sample.validate(&identity(), &anatomy()).is_err());
        sample.payload = SamplePayload::Missing {
            expected_shape: vec![6],
            reason: "contact sensor unavailable in exported stream".into(),
        };
        assert!(sample.validate(&identity(), &anatomy()).is_ok());
    }

    #[test]
    fn gpu_run_requires_pinned_warp_version_and_model_digests() {
        let mut run = identity();
        run.mujoco_warp_version = None;
        assert!(run.validate().is_err());
        run.mujoco_warp_version = Some("3.16.0".into());
        run.model_xml_sha256 = "bad".into();
        assert!(run.validate().is_err());
    }
}
