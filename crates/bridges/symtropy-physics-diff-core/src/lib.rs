// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Dependency-light records shared by independent physics backends.
//!
//! The schema deliberately has no engine dependencies. Semantic body IDs must be
//! supplied by a declared scenario mapping; adapters must never infer cross-engine
//! identity from insertion order or a physics engine's arena handle. Measurements
//! that are unavailable or not comparable are represented as `None`, not as zero.

use serde::{Deserialize, Serialize};

/// Current version of the common observation record schema.
pub const OBSERVATION_SCHEMA_VERSION: u16 = 1;

/// Immutable provenance for one physics-backend execution profile.
///
/// The scenario and configuration digests should be computed from canonicalized
/// scenario/configuration inputs, not from runtime handles, renderer state, or timing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunIdentity {
    pub schema_version: u16,
    pub scenario_id: String,
    pub scenario_digest: String,
    pub backend_id: String,
    pub backend_version: String,
    pub solver_profile: String,
    pub configuration_digest: String,
    /// Fixed simulation timestep in seconds.
    pub timestep_seconds: f64,
}

impl RunIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scenario_id: impl Into<String>,
        scenario_digest: impl Into<String>,
        backend_id: impl Into<String>,
        backend_version: impl Into<String>,
        solver_profile: impl Into<String>,
        configuration_digest: impl Into<String>,
        timestep_seconds: f64,
    ) -> Self {
        Self {
            schema_version: OBSERVATION_SCHEMA_VERSION,
            scenario_id: scenario_id.into(),
            scenario_digest: scenario_digest.into(),
            backend_id: backend_id.into(),
            backend_version: backend_version.into(),
            solver_profile: solver_profile.into(),
            configuration_digest: configuration_digest.into(),
            timestep_seconds,
        }
    }

    pub fn validate(&self) -> Result<(), ObservationError> {
        if self.schema_version != OBSERVATION_SCHEMA_VERSION {
            return Err(ObservationError::UnsupportedSchemaVersion {
                found: self.schema_version,
            });
        }
        for (field, value) in [
            ("scenario_id", self.scenario_id.as_str()),
            ("scenario_digest", self.scenario_digest.as_str()),
            ("backend_id", self.backend_id.as_str()),
            ("backend_version", self.backend_version.as_str()),
            ("solver_profile", self.solver_profile.as_str()),
            ("configuration_digest", self.configuration_digest.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ObservationError::EmptyIdentityField(field));
            }
        }
        if !self.timestep_seconds.is_finite() || self.timestep_seconds <= 0.0 {
            return Err(ObservationError::InvalidTimestep);
        }
        Ok(())
    }
}

/// Per-body observations in SI units wherever a physical unit applies.
///
/// Orientation uses a scalar-first quaternion `[w, x, y, z]`. Values are named
/// with units to discourage accidental comparison of incompatible conventions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyObservation {
    /// Stable scenario-level identity supplied by the caller, not an engine handle.
    pub semantic_body_id: String,
    pub position_m: [f64; 3],
    pub orientation_wxyz: [f64; 4],
    pub linear_velocity_mps: [f64; 3],
    pub angular_velocity_radps: [f64; 3],
    /// Engine-local sleep/active information when semantics are comparable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sleeping: Option<bool>,
    /// Per-body contact count; `None` means the adapter does not report it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_count: Option<u64>,
    /// Maximum observed penetration depth for this body, if derivable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_penetration_m: Option<f64>,
}

impl BodyObservation {
    pub fn new(
        semantic_body_id: impl Into<String>,
        position_m: [f64; 3],
        orientation_wxyz: [f64; 4],
        linear_velocity_mps: [f64; 3],
        angular_velocity_radps: [f64; 3],
    ) -> Self {
        Self {
            semantic_body_id: semantic_body_id.into(),
            position_m,
            orientation_wxyz,
            linear_velocity_mps,
            angular_velocity_radps,
            sleeping: None,
            contact_count: None,
            max_penetration_m: None,
        }
    }

    fn validate(&self) -> Result<(), ObservationError> {
        if self.semantic_body_id.trim().is_empty() {
            return Err(ObservationError::EmptySemanticBodyId);
        }
        for (field, values) in [
            ("position_m", self.position_m.as_slice()),
            ("orientation_wxyz", self.orientation_wxyz.as_slice()),
            ("linear_velocity_mps", self.linear_velocity_mps.as_slice()),
            ("angular_velocity_radps", self.angular_velocity_radps.as_slice()),
        ] {
            if !values.iter().all(|value| value.is_finite()) {
                return Err(ObservationError::NonFiniteBodyValue {
                    semantic_body_id: self.semantic_body_id.clone(),
                    field,
                });
            }
        }
        let quaternion_norm_squared = self
            .orientation_wxyz
            .iter()
            .map(|value| value * value)
            .sum::<f64>();
        if !quaternion_norm_squared.is_finite()
            || (quaternion_norm_squared - 1.0).abs() > 1.0e-3
        {
            return Err(ObservationError::InvalidQuaternion {
                semantic_body_id: self.semantic_body_id.clone(),
            });
        }
        validate_nonnegative(
            self.max_penetration_m,
            "body",
            "max_penetration_m",
        )?;
        Ok(())
    }
}

/// Contact metrics reported only when the source backend can measure them reliably.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifold_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_point_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_penetration_m: Option<f64>,
}

/// Energy diagnostics for which the scenario and backend profile define a meaningful
/// accounting convention. Potential energy may be negative because its zero is chosen.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EnergySummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinetic_energy_j: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub potential_energy_j: Option<f64>,
}

/// One timestamped observation from a single backend run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationFrame {
    pub identity: RunIdentity,
    pub step_index: u64,
    pub simulation_time_seconds: f64,
    pub bodies: Vec<BodyObservation>,
    pub contacts: ContactSummary,
    pub energy: EnergySummary,
}

impl ObservationFrame {
    pub fn new(
        identity: RunIdentity,
        step_index: u64,
        simulation_time_seconds: f64,
    ) -> Self {
        Self {
            identity,
            step_index,
            simulation_time_seconds,
            bodies: Vec::new(),
            contacts: ContactSummary::default(),
            energy: EnergySummary::default(),
        }
    }

    /// Validate provenance, units-bearing physical values, body identity uniqueness,
    /// and optional metrics without substituting zero for unavailable measurements.
    pub fn validate(&self) -> Result<(), ObservationError> {
        self.identity.validate()?;
        if !self.simulation_time_seconds.is_finite() || self.simulation_time_seconds < 0.0 {
            return Err(ObservationError::InvalidSimulationTime);
        }

        for (index, body) in self.bodies.iter().enumerate() {
            body.validate()?;
            if self.bodies[..index]
                .iter()
                .any(|prior| prior.semantic_body_id == body.semantic_body_id)
            {
                return Err(ObservationError::DuplicateSemanticBodyId(
                    body.semantic_body_id.clone(),
                ));
            }
        }

        validate_nonnegative(
            self.contacts.max_penetration_m,
            "contact_summary",
            "max_penetration_m",
        )?;
        validate_nonnegative(
            self.energy.kinetic_energy_j,
            "energy",
            "kinetic_energy_j",
        )?;
        validate_finite_optional(
            self.energy.potential_energy_j,
            "energy",
            "potential_energy_j",
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationError {
    UnsupportedSchemaVersion { found: u16 },
    EmptyIdentityField(&'static str),
    InvalidTimestep,
    InvalidSimulationTime,
    EmptySemanticBodyId,
    DuplicateSemanticBodyId(String),
    NonFiniteBodyValue {
        semantic_body_id: String,
        field: &'static str,
    },
    InvalidQuaternion { semantic_body_id: String },
    NonFiniteOptionalValue {
        subject: &'static str,
        field: &'static str,
    },
    NegativeOptionalValue {
        subject: &'static str,
        field: &'static str,
    },
}

fn validate_finite_optional(
    value: Option<f64>,
    subject: &'static str,
    field: &'static str,
) -> Result<(), ObservationError> {
    if value.is_some_and(|number| !number.is_finite()) {
        return Err(ObservationError::NonFiniteOptionalValue { subject, field });
    }
    Ok(())
}

fn validate_nonnegative(
    value: Option<f64>,
    subject: &'static str,
    field: &'static str,
) -> Result<(), ObservationError> {
    validate_finite_optional(value, subject, field)?;
    if value.is_some_and(|number| number < 0.0) {
        return Err(ObservationError::NegativeOptionalValue { subject, field });
    }
    Ok(())
}

impl std::fmt::Display for ObservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion { found } => {
                write!(f, "unsupported observation schema version {found}")
            }
            Self::EmptyIdentityField(field) => write!(f, "run identity field {field} is empty"),
            Self::InvalidTimestep => write!(f, "timestep must be finite and positive"),
            Self::InvalidSimulationTime => {
                write!(f, "simulation time must be finite and nonnegative")
            }
            Self::EmptySemanticBodyId => write!(f, "semantic body ID must not be empty"),
            Self::DuplicateSemanticBodyId(body_id) => {
                write!(f, "duplicate semantic body ID {body_id:?}")
            }
            Self::NonFiniteBodyValue {
                semantic_body_id,
                field,
            } => write!(f, "body {semantic_body_id:?} has non-finite {field}"),
            Self::InvalidQuaternion { semantic_body_id } => {
                write!(f, "body {semantic_body_id:?} has a non-unit quaternion")
            }
            Self::NonFiniteOptionalValue { subject, field } => {
                write!(f, "{subject}.{field} must be finite when present")
            }
            Self::NegativeOptionalValue { subject, field } => {
                write!(f, "{subject}.{field} must be nonnegative when present")
            }
        }
    }
}

impl std::error::Error for ObservationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> RunIdentity {
        RunIdentity::new(
            "free-fall",
            "sha256:scenario-fixture",
            "symtropy-native",
            "test-build",
            "deterministic-profile",
            "sha256:configuration-fixture",
            1.0 / 60.0,
        )
    }

    fn body(id: &str) -> BodyObservation {
        BodyObservation::new(
            id,
            [0.0, 10.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, -0.1, 0.0],
            [0.0, 0.0, 0.0],
        )
    }

    fn frame() -> ObservationFrame {
        let mut frame = ObservationFrame::new(identity(), 1, 1.0 / 60.0);
        frame.bodies.push(body("ball/main"));
        frame
    }

    #[test]
    fn valid_frame_accepts_missing_engine_specific_measurements() {
        let mut frame = frame();
        frame.bodies[0].sleeping = Some(false);
        frame.bodies[0].contact_count = None;
        frame.energy.potential_energy_j = Some(-2.5);

        assert_eq!(frame.validate(), Ok(()));
    }

    #[test]
    fn observation_frame_json_round_trip_preserves_unknown_measurements_as_absent() {
        let frame = frame();
        let encoded = serde_json::to_string(&frame).expect("serialize validated frame");
        assert!(
            !encoded.contains("\\"contact_count\\":null"),
            "unavailable metrics should be omitted rather than encoded as fake zero/null values",
        );

        let decoded: ObservationFrame =
            serde_json::from_str(&encoded).expect("deserialize observation frame");
        assert_eq!(decoded, frame);
        assert_eq!(decoded.validate(), Ok(()));
    }

    #[test]
    fn duplicate_semantic_body_ids_are_rejected() {
        let mut frame = frame();
        frame.bodies.push(body("ball/main"));

        assert_eq!(
            frame.validate(),
            Err(ObservationError::DuplicateSemanticBodyId(
                "ball/main".to_owned()
            )),
        );
    }

    #[test]
    fn invalid_pose_and_motion_values_are_rejected() {
        let mut non_finite_position = frame();
        non_finite_position.bodies[0].position_m[0] = f64::NAN;
        assert!(matches!(
            non_finite_position.validate(),
            Err(ObservationError::NonFiniteBodyValue {
                field: "position_m",
                ..
            })
        ));

        let mut invalid_quaternion = frame();
        invalid_quaternion.bodies[0].orientation_wxyz = [2.0, 0.0, 0.0, 0.0];
        assert!(matches!(
            invalid_quaternion.validate(),
            Err(ObservationError::InvalidQuaternion { .. })
        ));
    }

    #[test]
    fn invalid_provenance_and_physical_summaries_are_rejected() {
        let mut invalid_dt = frame();
        invalid_dt.identity.timestep_seconds = 0.0;
        assert_eq!(invalid_dt.validate(), Err(ObservationError::InvalidTimestep));

        let mut invalid_penetration = frame();
        invalid_penetration.contacts.max_penetration_m = Some(-0.01);
        assert_eq!(
            invalid_penetration.validate(),
            Err(ObservationError::NegativeOptionalValue {
                subject: "contact_summary",
                field: "max_penetration_m",
            }),
        );

        let mut invalid_kinetic_energy = frame();
        invalid_kinetic_energy.energy.kinetic_energy_j = Some(f64::INFINITY);
        assert_eq!(
            invalid_kinetic_energy.validate(),
            Err(ObservationError::NonFiniteOptionalValue {
                subject: "energy",
                field: "kinetic_energy_j",
            }),
        );
    }
}
