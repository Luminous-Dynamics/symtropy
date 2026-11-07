//! Terrain-profile receipt validation for fair locomotion comparisons.
//!
//! This crate is benchmark-side only. It references terrain/material/solver
//! authorities owned elsewhere; it does not become a second terrain or
//! contact-material authority.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TerrainClass {
    Prepared,
    StructuredUneven,
    Unprepared,
    DeformableSoft,
    LunarSimulant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TerrainMetric {
    /// Maximum absolute ground grade, radians.
    MaximumAbsoluteGradeRadians,
    /// RMS surface-height roughness, metres.
    HeightRoughnessRmsMetres,
    /// Maximum obstacle height, metres.
    MaximumObstacleHeightMetres,
    /// Maximum declared gap width, metres.
    MaximumGapWidthMetres,
    /// Heightfield spatial resolution, metres per cell.
    HeightfieldResolutionMetres,
    /// Declared characteristic wavelength for the terrain profile, metres.
    CharacteristicWavelengthMetres,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileValue {
    /// Interval measured with a declared evidence artifact and calibration.
    Measured {
        lower: f64,
        upper: f64,
        evidence_ref: String,
        calibration_ref: String,
    },
    /// Interval used as an explicit assumption, never a measurement.
    Assumed {
        lower: f64,
        upper: f64,
        assumption_ref: String,
        rationale: String,
    },
    /// Explicitly unknown. This is not equivalent to zero or a default.
    Unknown { reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerrainProfile {
    pub schema_version: u16,
    pub profile_id: String,
    pub profile_version: u32,
    pub terrain_class: TerrainClass,
    /// SHA-256 identity of the canonical terrain asset; validation checks
    /// syntax only. The producer must verify the digest against the bytes.
    pub terrain_asset_sha256: String,
    pub coordinate_frame_ref: String,
    /// Qualified contact/material profile owned by the material authority
    /// (e.g. PHYS-MATERIAL-09). This crate stores the reference, not values.
    pub contact_material_profile_ref: String,
    /// Properties required for the benchmark claim in this scenario.
    pub required_metrics: BTreeSet<TerrainMetric>,
    /// Versioned descriptors with typed units encoded by TerrainMetric.
    pub geometry: BTreeMap<TerrainMetric, ProfileValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileQualification {
    FullyMeasured,
    AssumptionDependent,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileCoverage {
    pub required_measured: usize,
    pub required_assumed: usize,
    pub required_unknown: usize,
    pub optional_measured: usize,
    pub optional_assumed: usize,
    pub optional_unknown: usize,
    pub qualification: ProfileQualification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptError {
    EmptyIdentifier,
    UnsupportedSchemaVersion,
    InvalidProfileVersion,
    InvalidAssetDigest,
    EmptyRequiredMetrics,
    MissingRequiredMetric,
    InvalidMetricInterval,
    EmptyEvidenceReference,
    EmptyCalibrationReference,
    EmptyAssumptionReference,
    EmptyAssumptionRationale,
    EmptyUnknownReason,
    EmptyRunReference,
    InvalidPhysicsTimestep,
    InvalidControllerPeriod,
    InvalidInitialStateDigest,
    TerrainProfileMismatch,
    BodyProfileMismatch,
    ContactMaterialProfileMismatch,
    SolverProfileMismatch,
    TaskProtocolMismatch,
    ObservationSchemaMismatch,
    ExecutionProfileMismatch,
    ComputeBudgetMismatch,
    PhysicsTimestepMismatch,
    ControllerPeriodMismatch,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunDescriptor {
    pub run_id: String,
    pub terrain_profile: TerrainProfile,
    /// Morphology, actuator map, contact geometry and mass-profile authority.
    pub body_profile_ref: String,
    /// Contact solver / numerical profile from physics qualification.
    pub solver_profile_ref: String,
    pub task_protocol_ref: String,
    pub initial_state_sha256: String,
    pub observation_schema_ref: String,
    pub execution_profile_ref: String,
    pub compute_budget_ref: String,
    pub physics_timestep_seconds: f64,
    pub controller_period_seconds: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchedPairQualification {
    pub terrain_profile: ProfileCoverage,
    pub pairs_same_terrain_profile: bool,
    pub pairs_same_body_profile: bool,
    pub pairs_same_solver_profile: bool,
    pub pairs_same_task_protocol: bool,
    pub pairs_same_observation_schema: bool,
    pub pairs_same_execution_profile: bool,
    pub pairs_same_compute_budget: bool,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_interval(lower: f64, upper: f64) -> Result<(), ReceiptError> {
    if !lower.is_finite() || !upper.is_finite() || lower < 0.0 || lower > upper {
        return Err(ReceiptError::InvalidMetricInterval);
    }
    Ok(())
}

impl ProfileValue {
    fn validate(&self) -> Result<(), ReceiptError> {
        match self {
            Self::Measured {
                lower,
                upper,
                evidence_ref,
                calibration_ref,
            } => {
                validate_interval(*lower, *upper)?;
                if evidence_ref.trim().is_empty() {
                    return Err(ReceiptError::EmptyEvidenceReference);
                }
                if calibration_ref.trim().is_empty() {
                    return Err(ReceiptError::EmptyCalibrationReference);
                }
            }
            Self::Assumed {
                lower,
                upper,
                assumption_ref,
                rationale,
            } => {
                validate_interval(*lower, *upper)?;
                if assumption_ref.trim().is_empty() {
                    return Err(ReceiptError::EmptyAssumptionReference);
                }
                if rationale.trim().is_empty() {
                    return Err(ReceiptError::EmptyAssumptionRationale);
                }
            }
            Self::Unknown { reason } => {
                if reason.trim().is_empty() {
                    return Err(ReceiptError::EmptyUnknownReason);
                }
            }
        }
        Ok(())
    }

    fn kind(&self) -> ProfileValueKind {
        match self {
            Self::Measured { .. } => ProfileValueKind::Measured,
            Self::Assumed { .. } => ProfileValueKind::Assumed,
            Self::Unknown { .. } => ProfileValueKind::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileValueKind {
    Measured,
    Assumed,
    Unknown,
}

impl TerrainProfile {
    pub fn validate(&self) -> Result<(), ReceiptError> {
        if self.schema_version != 1 {
            return Err(ReceiptError::UnsupportedSchemaVersion);
        }
        if self.profile_id.trim().is_empty()
            || self.coordinate_frame_ref.trim().is_empty()
            || self.contact_material_profile_ref.trim().is_empty()
        {
            return Err(ReceiptError::EmptyIdentifier);
        }
        if self.profile_version == 0 {
            return Err(ReceiptError::InvalidProfileVersion);
        }
        if !valid_sha256(&self.terrain_asset_sha256) {
            return Err(ReceiptError::InvalidAssetDigest);
        }
        if self.required_metrics.is_empty() {
            return Err(ReceiptError::EmptyRequiredMetrics);
        }
        for metric in &self.required_metrics {
            if !self.geometry.contains_key(metric) {
                return Err(ReceiptError::MissingRequiredMetric);
            }
        }
        for value in self.geometry.values() {
            value.validate()?;
        }
        Ok(())
    }

    /// Count evidence states without converting assumptions or unknowns into
    /// measurements. Required metrics determine the qualification label.
    pub fn coverage(&self) -> Result<ProfileCoverage, ReceiptError> {
        self.validate()?;
        let mut required_measured = 0;
        let mut required_assumed = 0;
        let mut required_unknown = 0;
        let mut optional_measured = 0;
        let mut optional_assumed = 0;
        let mut optional_unknown = 0;

        for (metric, value) in &self.geometry {
            let required = self.required_metrics.contains(metric);
            match (required, value.kind()) {
                (true, ProfileValueKind::Measured) => required_measured += 1,
                (true, ProfileValueKind::Assumed) => required_assumed += 1,
                (true, ProfileValueKind::Unknown) => required_unknown += 1,
                (false, ProfileValueKind::Measured) => optional_measured += 1,
                (false, ProfileValueKind::Assumed) => optional_assumed += 1,
                (false, ProfileValueKind::Unknown) => optional_unknown += 1,
            }
        }

        let qualification = if required_unknown > 0 {
            ProfileQualification::Incomplete
        } else if required_assumed > 0 {
            ProfileQualification::AssumptionDependent
        } else {
            ProfileQualification::FullyMeasured
        };

        Ok(ProfileCoverage {
            required_measured,
            required_assumed,
            required_unknown,
            optional_measured,
            optional_assumed,
            optional_unknown,
            qualification,
        })
    }
}

impl RunDescriptor {
    pub fn validate(&self) -> Result<(), ReceiptError> {
        if self.run_id.trim().is_empty()
            || self.body_profile_ref.trim().is_empty()
            || self.solver_profile_ref.trim().is_empty()
            || self.task_protocol_ref.trim().is_empty()
            || self.observation_schema_ref.trim().is_empty()
            || self.execution_profile_ref.trim().is_empty()
            || self.compute_budget_ref.trim().is_empty()
        {
            return Err(ReceiptError::EmptyRunReference);
        }
        if !valid_sha256(&self.initial_state_sha256) {
            return Err(ReceiptError::InvalidInitialStateDigest);
        }
        if !self.physics_timestep_seconds.is_finite() || self.physics_timestep_seconds <= 0.0 {
            return Err(ReceiptError::InvalidPhysicsTimestep);
        }
        if !self.controller_period_seconds.is_finite() || self.controller_period_seconds <= 0.0 {
            return Err(ReceiptError::InvalidControllerPeriod);
        }
        self.terrain_profile.validate()
    }
}

/// Validate whether two controller runs are a fair matched pair under the
/// declared benchmark identity. Controller identity is intentionally omitted:
/// it is the candidate factor being compared. This function does not prove
/// that the external profile references are physically correct.
pub fn qualify_matched_pair(
    left: &RunDescriptor,
    right: &RunDescriptor,
) -> Result<MatchedPairQualification, ReceiptError> {
    left.validate()?;
    right.validate()?;

    if left.terrain_profile != right.terrain_profile {
        return Err(ReceiptError::TerrainProfileMismatch);
    }
    if left.body_profile_ref != right.body_profile_ref {
        return Err(ReceiptError::BodyProfileMismatch);
    }
    if left.terrain_profile.contact_material_profile_ref
        != right.terrain_profile.contact_material_profile_ref
    {
        return Err(ReceiptError::ContactMaterialProfileMismatch);
    }
    if left.solver_profile_ref != right.solver_profile_ref {
        return Err(ReceiptError::SolverProfileMismatch);
    }
    if left.task_protocol_ref != right.task_protocol_ref
        || left.initial_state_sha256 != right.initial_state_sha256
    {
        return Err(ReceiptError::TaskProtocolMismatch);
    }
    if left.observation_schema_ref != right.observation_schema_ref {
        return Err(ReceiptError::ObservationSchemaMismatch);
    }
    if left.execution_profile_ref != right.execution_profile_ref {
        return Err(ReceiptError::ExecutionProfileMismatch);
    }
    if left.compute_budget_ref != right.compute_budget_ref {
        return Err(ReceiptError::ComputeBudgetMismatch);
    }
    if left.physics_timestep_seconds != right.physics_timestep_seconds {
        return Err(ReceiptError::PhysicsTimestepMismatch);
    }
    if left.controller_period_seconds != right.controller_period_seconds {
        return Err(ReceiptError::ControllerPeriodMismatch);
    }

    Ok(MatchedPairQualification {
        terrain_profile: left.terrain_profile.coverage()?,
        pairs_same_terrain_profile: true,
        pairs_same_body_profile: true,
        pairs_same_solver_profile: true,
        pairs_same_task_protocol: true,
        pairs_same_observation_schema: true,
        pairs_same_execution_profile: true,
        pairs_same_compute_budget: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured(lower: f64, upper: f64) -> ProfileValue {
        ProfileValue::Measured {
            lower,
            upper,
            evidence_ref: "evidence://terrain/calibration-run-01".to_owned(),
            calibration_ref: "calibration://terrain/profile-01".to_owned(),
        }
    }

    fn base_profile() -> TerrainProfile {
        let required_metrics = BTreeSet::from([
            TerrainMetric::MaximumAbsoluteGradeRadians,
            TerrainMetric::HeightRoughnessRmsMetres,
            TerrainMetric::MaximumObstacleHeightMetres,
        ]);
        let geometry = BTreeMap::from([
            (TerrainMetric::MaximumAbsoluteGradeRadians, measured(0.0, 0.08)),
            (TerrainMetric::HeightRoughnessRmsMetres, measured(0.002, 0.004)),
            (TerrainMetric::MaximumObstacleHeightMetres, measured(0.0, 0.015)),
        ]);
        TerrainProfile {
            schema_version: 1,
            profile_id: "terrain/t1/fixture-01".to_owned(),
            profile_version: 1,
            terrain_class: TerrainClass::StructuredUneven,
            terrain_asset_sha256: "a".repeat(64),
            coordinate_frame_ref: "frame/world-z-up/v1".to_owned(),
            contact_material_profile_ref: "material-profile/mat-09/v3".to_owned(),
            required_metrics,
            geometry,
        }
    }

    fn base_run(run_id: &str) -> RunDescriptor {
        RunDescriptor {
            run_id: run_id.to_owned(),
            terrain_profile: base_profile(),
            body_profile_ref: "body/morphology/hexapod-18dof/v1".to_owned(),
            solver_profile_ref: "physics/solver/mujoco-3.16/tol-1e-5".to_owned(),
            task_protocol_ref: "task/flat-start-stop/v2".to_owned(),
            initial_state_sha256: "b".repeat(64),
            observation_schema_ref: "sensors/joint-contact-imu/v1".to_owned(),
            execution_profile_ref: "hardware/gpu-off/cpu-x86_64/v1".to_owned(),
            compute_budget_ref: "budget/100hz-5ms/1".to_owned(),
            physics_timestep_seconds: 0.001,
            controller_period_seconds: 0.01,
        }
    }

    #[test]
    fn measured_profile_produces_fully_measured_qualification() {
        let coverage = base_profile().coverage().unwrap();
        assert_eq!(coverage.required_measured, 3);
        assert_eq!(coverage.required_assumed, 0);
        assert_eq!(coverage.required_unknown, 0);
        assert_eq!(coverage.qualification, ProfileQualification::FullyMeasured);
    }

    #[test]
    fn assumptions_are_not_mislabeled_as_measurements() {
        let mut profile = base_profile();
        profile.geometry.insert(
            TerrainMetric::HeightRoughnessRmsMetres,
            ProfileValue::Assumed {
                lower: 0.002,
                upper: 0.004,
                assumption_ref: "assumption/roughness-estimate-01".to_owned(),
                rationale: "No direct profilometer sample for this asset".to_owned(),
            },
        );
        let coverage = profile.coverage().unwrap();
        assert_eq!(coverage.required_measured, 2);
        assert_eq!(coverage.required_assumed, 1);
        assert_eq!(coverage.qualification, ProfileQualification::AssumptionDependent);
    }

    #[test]
    fn unknown_required_metric_remains_unknown_and_incomplete() {
        let mut profile = base_profile();
        profile.geometry.insert(
            TerrainMetric::MaximumObstacleHeightMetres,
            ProfileValue::Unknown {
                reason: "Occluded section; no height observation".to_owned(),
            },
        );
        let coverage = profile.coverage().unwrap();
        assert_eq!(coverage.required_unknown, 1);
        assert_eq!(coverage.qualification, ProfileQualification::Incomplete);
    }

    #[test]
    fn missing_required_metric_is_rejected() {
        let mut profile = base_profile();
        profile.geometry.remove(&TerrainMetric::HeightRoughnessRmsMetres);
        assert_eq!(profile.validate(), Err(ReceiptError::MissingRequiredMetric));
    }

    #[test]
    fn measured_interval_rejects_nonfinite_negative_or_reversed_bounds() {
        for value in [
            measured(-0.1, 0.2),
            measured(0.3, 0.2),
            measured(0.0, f64::INFINITY),
        ] {
            let mut profile = base_profile();
            profile.geometry.insert(TerrainMetric::HeightRoughnessRmsMetres, value);
            assert_eq!(profile.validate(), Err(ReceiptError::InvalidMetricInterval));
        }
    }

    #[test]
    fn missing_calibration_or_evidence_reference_is_rejected() {
        let mut profile = base_profile();
        profile.geometry.insert(
            TerrainMetric::MaximumAbsoluteGradeRadians,
            ProfileValue::Measured {
                lower: 0.0,
                upper: 0.1,
                evidence_ref: "evidence/grade".to_owned(),
                calibration_ref: " ".to_owned(),
            },
        );
        assert_eq!(profile.validate(), Err(ReceiptError::EmptyCalibrationReference));
    }

    #[test]
    fn invalid_asset_digest_is_rejected() {
        let mut profile = base_profile();
        profile.terrain_asset_sha256 = "not-a-digest".to_owned();
        assert_eq!(profile.validate(), Err(ReceiptError::InvalidAssetDigest));
    }

    #[test]
    fn profiles_without_contact_material_authority_are_rejected() {
        let mut profile = base_profile();
        profile.contact_material_profile_ref.clear();
        assert_eq!(profile.validate(), Err(ReceiptError::EmptyIdentifier));
    }

    #[test]
    fn identical_profile_and_run_identity_are_a_matched_pair() {
        let result = qualify_matched_pair(&base_run("run-a"), &base_run("run-b")).unwrap();
        assert!(result.pairs_same_terrain_profile);
        assert!(result.pairs_same_body_profile);
        assert!(result.pairs_same_solver_profile);
        assert_eq!(result.terrain_profile.qualification, ProfileQualification::FullyMeasured);
    }

    #[test]
    fn different_terrain_asset_is_not_a_matched_pair() {
        let left = base_run("run-a");
        let mut right = base_run("run-b");
        right.terrain_profile.terrain_asset_sha256 = "c".repeat(64);
        assert_eq!(
            qualify_matched_pair(&left, &right),
            Err(ReceiptError::TerrainProfileMismatch)
        );
    }

    #[test]
    fn different_contact_profile_is_not_a_matched_pair() {
        let left = base_run("run-a");
        let mut right = base_run("run-b");
        right.terrain_profile.contact_material_profile_ref = "material-profile/other/v1".to_owned();
        assert_eq!(
            qualify_matched_pair(&left, &right),
            Err(ReceiptError::TerrainProfileMismatch)
        );
    }

    #[test]
    fn different_body_solver_or_observation_schema_is_not_a_matched_pair() {
        let left = base_run("run-a");
        let mut right = base_run("run-b");
        right.body_profile_ref = "body/other".to_owned();
        assert_eq!(
            qualify_matched_pair(&left, &right),
            Err(ReceiptError::BodyProfileMismatch)
        );

        let mut right = base_run("run-b");
        right.solver_profile_ref = "physics/solver/other".to_owned();
        assert_eq!(
            qualify_matched_pair(&left, &right),
            Err(ReceiptError::SolverProfileMismatch)
        );

        let mut right = base_run("run-b");
        right.observation_schema_ref = "sensors/other".to_owned();
        assert_eq!(
            qualify_matched_pair(&left, &right),
            Err(ReceiptError::ObservationSchemaMismatch)
        );
    }

    #[test]
    fn different_task_state_compute_or_execution_is_not_a_matched_pair() {
        let left = base_run("run-a");
        let mut right = base_run("run-b");
        right.task_protocol_ref = "task/other".to_owned();
        assert_eq!(
            qualify_matched_pair(&left, &right),
            Err(ReceiptError::TaskProtocolMismatch)
        );

        let mut right = base_run("run-b");
        right.initial_state_sha256 = "c".repeat(64);
        assert_eq!(
            qualify_matched_pair(&left, &right),
            Err(ReceiptError::TaskProtocolMismatch)
        );

        let mut right = base_run("run-b");
        right.compute_budget_ref = "budget/other".to_owned();
        assert_eq!(
            qualify_matched_pair(&left, &right),
            Err(ReceiptError::ComputeBudgetMismatch)
        );

        let mut right = base_run("run-b");
        right.execution_profile_ref = "hardware/other".to_owned();
        assert_eq!(
            qualify_matched_pair(&left, &right),
            Err(ReceiptError::ExecutionProfileMismatch)
        );
    }

    #[test]
    fn different_time_steps_or_controller_periods_are_not_a_matched_pair() {
        let left = base_run("run-a");
        let mut right = base_run("run-b");
        right.physics_timestep_seconds = 0.002;
        assert_eq!(
            qualify_matched_pair(&left, &right),
            Err(ReceiptError::PhysicsTimestepMismatch)
        );

        let mut right = base_run("run-b");
        right.controller_period_seconds = 0.02;
        assert_eq!(
            qualify_matched_pair(&left, &right),
            Err(ReceiptError::ControllerPeriodMismatch)
        );
    }

    #[test]
    fn profile_identity_and_coverage_are_independent_of_map_insertion_order() {
        let mut a = base_profile();
        let mut b = base_profile();
        b.geometry = BTreeMap::from([
            (TerrainMetric::MaximumObstacleHeightMetres, measured(0.0, 0.015)),
            (TerrainMetric::MaximumAbsoluteGradeRadians, measured(0.0, 0.08)),
            (TerrainMetric::HeightRoughnessRmsMetres, measured(0.002, 0.004)),
        ]);
        assert_eq!(a, b);
        assert_eq!(a.coverage(), b.coverage());
        a.required_metrics = BTreeSet::from([TerrainMetric::MaximumAbsoluteGradeRadians]);
        assert_ne!(a, b);
    }
}
