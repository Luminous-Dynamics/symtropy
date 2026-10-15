// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Engine-neutral rigid-body validation vocabulary and native reference adapter.
//!
//! This module deliberately sits above the hot solver path: it defines small,
//! serializable scenario/trace types and pure comparison logic used by research
//! harnesses. External engines may adapt to the same types, but agreement with
//! any external engine is evidence, not canonical Symtropy physical truth.
//!
//! V0.1 intentionally covers a narrow shared validity domain: 3D, axis-aligned
//! sphere/cuboid bodies, static/dynamic body types, explicit SI gravity and
//! linear velocity, and position/linear-velocity observations. Initial
//! orientation, angular-velocity equivalence, joints, per-body CCD and applied
//! forces remain outside this schema until their cross-engine conventions are
//! explicitly reconciled.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use nalgebra::SVector;
use serde::{Deserialize, Serialize};
use symtropy_math::{HyperBox, Point, Shape, Sphere, Transform};

use crate::body::{BodyHandle, BodyType, NetId, RigidBody};
use crate::world::PhysicsWorld;

/// Current schema version for [`PhysicsScenario3d`].
pub const PHYSICS_SCENARIO_SCHEMA_VERSION: u32 = 1;
/// Current schema version for [`PhysicsTrace3d`].
pub const PHYSICS_TRACE_SCHEMA_VERSION: u32 = 1;

/// Stable, engine-neutral body identity within one validation scenario.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ValidationBodyId(pub u64);

/// Body authority represented by the v0.1 validation schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioBodyType {
    Static,
    Dynamic,
}

/// Collider primitives shared by the initial native/Rapier corpus.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScenarioShape3d {
    Sphere { radius_m: f64 },
    Cuboid { half_extents_m: [f64; 3] },
}

/// One rigid body in a canonical differential-validation scenario.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScenarioBody3d {
    pub id: ValidationBodyId,
    pub body_type: ScenarioBodyType,
    pub shape: ScenarioShape3d,
    /// World-space center/translation in metres. V0.1 orientation is identity.
    pub position_m: [f64; 3],
    /// World-space linear velocity in metres/second.
    pub linear_velocity_mps: [f64; 3],
    /// Dynamic body mass in kilograms. Static bodies must declare `0.0`.
    pub mass_kg: f64,
    pub friction: f64,
    pub restitution: f64,
}

/// Canonical 3D rigid-body fixture shared by independent solver backends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsScenario3d {
    pub schema_version: u32,
    pub scenario_id: String,
    /// Uniform acceleration in metres/second².
    pub gravity_mps2: [f64; 3],
    pub dt_s: f64,
    pub steps: u64,
    /// Record step zero, every Nth step, and always the final step.
    pub sample_every_steps: u64,
    pub bodies: Vec<ScenarioBody3d>,
}

impl PhysicsScenario3d {
    /// Validate all v0.1 invariants without changing caller ordering.
    pub fn validate(&self) -> Result<(), ScenarioValidationError> {
        if self.schema_version != PHYSICS_SCENARIO_SCHEMA_VERSION {
            return Err(ScenarioValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.scenario_id.trim().is_empty() {
            return Err(ScenarioValidationError::EmptyScenarioId);
        }
        if !self.dt_s.is_finite() || self.dt_s <= 0.0 {
            return Err(ScenarioValidationError::InvalidTimeStep);
        }
        if self.steps == 0 {
            return Err(ScenarioValidationError::ZeroSteps);
        }
        let total_duration_s = self.dt_s * self.steps as f64;
        if !total_duration_s.is_finite() || total_duration_s <= 0.0 {
            return Err(ScenarioValidationError::InvalidSimulationDuration);
        }
        if self.sample_every_steps == 0 {
            return Err(ScenarioValidationError::ZeroSampleStride);
        }
        if !all_finite(&self.gravity_mps2) {
            return Err(ScenarioValidationError::NonFiniteGravity);
        }
        if self.bodies.is_empty() {
            return Err(ScenarioValidationError::NoBodies);
        }

        let mut ids = BTreeSet::new();
        for body in &self.bodies {
            if !ids.insert(body.id) {
                return Err(ScenarioValidationError::DuplicateBodyId(body.id));
            }
            if !all_finite(&body.position_m) || !all_finite(&body.linear_velocity_mps) {
                return Err(ScenarioValidationError::NonFiniteBodyState(body.id));
            }
            if !body.friction.is_finite() || body.friction < 0.0 {
                return Err(ScenarioValidationError::InvalidFriction(body.id));
            }
            if !body.restitution.is_finite() || !(0.0..=1.0).contains(&body.restitution) {
                return Err(ScenarioValidationError::InvalidRestitution(body.id));
            }

            match body.body_type {
                ScenarioBodyType::Static => {
                    if !body.mass_kg.is_finite() || body.mass_kg != 0.0 {
                        return Err(ScenarioValidationError::InvalidStaticMass(body.id));
                    }
                    if body.linear_velocity_mps.iter().any(|value| *value != 0.0) {
                        return Err(ScenarioValidationError::StaticBodyHasVelocity(body.id));
                    }
                }
                ScenarioBodyType::Dynamic => {
                    if !body.mass_kg.is_finite() || body.mass_kg <= 0.0 {
                        return Err(ScenarioValidationError::InvalidDynamicMass(body.id));
                    }
                }
            }

            match body.shape {
                ScenarioShape3d::Sphere { radius_m } => {
                    if !radius_m.is_finite() || radius_m <= 0.0 {
                        return Err(ScenarioValidationError::InvalidShape(body.id));
                    }
                }
                ScenarioShape3d::Cuboid { half_extents_m } => {
                    if !all_finite(&half_extents_m)
                        || half_extents_m.iter().any(|extent| *extent <= 0.0)
                    {
                        return Err(ScenarioValidationError::InvalidShape(body.id));
                    }
                }
            }
        }

        Ok(())
    }

    /// Return a validated copy in deterministic body-ID order.
    pub fn normalized(&self) -> Result<Self, ScenarioValidationError> {
        self.validate()?;
        let mut normalized = self.clone();
        normalized.bodies.sort_by_key(|body| body.id);
        Ok(normalized)
    }
}

/// Invalid or ambiguous canonical scenario input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScenarioValidationError {
    UnsupportedSchemaVersion(u32),
    EmptyScenarioId,
    InvalidTimeStep,
    InvalidSimulationDuration,
    ZeroSteps,
    ZeroSampleStride,
    NonFiniteGravity,
    NoBodies,
    DuplicateBodyId(ValidationBodyId),
    NonFiniteBodyState(ValidationBodyId),
    InvalidStaticMass(ValidationBodyId),
    StaticBodyHasVelocity(ValidationBodyId),
    InvalidDynamicMass(ValidationBodyId),
    InvalidFriction(ValidationBodyId),
    InvalidRestitution(ValidationBodyId),
    InvalidShape(ValidationBodyId),
}

impl fmt::Display for ScenarioValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported physics scenario schema version {version}")
            }
            Self::EmptyScenarioId => write!(f, "scenario_id must not be empty"),
            Self::InvalidTimeStep => write!(f, "dt_s must be finite and greater than zero"),
            Self::InvalidSimulationDuration => write!(
                f,
                "dt_s multiplied by steps must produce a finite positive duration"
            ),
            Self::ZeroSteps => write!(f, "steps must be greater than zero"),
            Self::ZeroSampleStride => write!(f, "sample_every_steps must be greater than zero"),
            Self::NonFiniteGravity => write!(f, "gravity_mps2 must be finite"),
            Self::NoBodies => write!(f, "scenario must contain at least one body"),
            Self::DuplicateBodyId(id) => write!(f, "duplicate validation body id {}", id.0),
            Self::NonFiniteBodyState(id) => {
                write!(f, "body {} has non-finite position or velocity", id.0)
            }
            Self::InvalidStaticMass(id) => {
                write!(f, "static body {} must declare mass_kg = 0", id.0)
            }
            Self::StaticBodyHasVelocity(id) => write!(
                f,
                "static body {} must start with zero linear velocity",
                id.0
            ),
            Self::InvalidDynamicMass(id) => {
                write!(f, "dynamic body {} must have finite positive mass", id.0)
            }
            Self::InvalidFriction(id) => {
                write!(f, "body {} must have finite non-negative friction", id.0)
            }
            Self::InvalidRestitution(id) => {
                write!(f, "body {} restitution must be finite and in [0, 1]", id.0)
            }
            Self::InvalidShape(id) => write!(
                f,
                "body {} has invalid or non-finite shape dimensions",
                id.0
            ),
        }
    }
}

impl std::error::Error for ScenarioValidationError {}

/// Declares which independent implementation produced a trace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicsBackendDescriptor {
    pub engine: String,
    pub engine_version: String,
    pub profile: String,
}

/// Engine-neutral post-step state for one validation body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyObservation3d {
    pub body_id: ValidationBodyId,
    pub position_m: [f64; 3],
    pub linear_velocity_mps: [f64; 3],
    pub sleeping: bool,
}

impl BodyObservation3d {
    pub fn is_finite(&self) -> bool {
        all_finite(&self.position_m) && all_finite(&self.linear_velocity_mps)
    }
}

/// One deterministic observation point in a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsSample3d {
    pub step: u64,
    pub time_s: f64,
    /// Must be sorted by [`ValidationBodyId`] in produced traces.
    pub bodies: Vec<BodyObservation3d>,
}

/// Complete sampled trace for one backend executing one scenario.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsTrace3d {
    pub schema_version: u32,
    pub scenario_id: String,
    pub backend: PhysicsBackendDescriptor,
    pub samples: Vec<PhysicsSample3d>,
}

impl PhysicsTrace3d {
    pub fn final_sample(&self) -> Option<&PhysicsSample3d> {
        self.samples.last()
    }
}

/// Aggregate differential metrics between two traces of the same scenario.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DifferentialMetrics3d {
    pub scenario_id: String,
    pub left_backend: PhysicsBackendDescriptor,
    pub right_backend: PhysicsBackendDescriptor,
    pub compared_samples: u64,
    pub compared_bodies: u64,
    /// Absent when no body pair had finite position and velocity on both sides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_position_error_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rms_position_error_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_linear_velocity_error_mps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rms_linear_velocity_error_mps: Option<f64>,
    /// Number of (sample, body) pairs included in the numerical error metrics.
    #[serde(default)]
    pub finite_comparison_pairs: u64,
    pub sleeping_mismatches: u64,
    pub left_non_finite_observations: u64,
    pub right_non_finite_observations: u64,
}

impl DifferentialMetrics3d {
    /// Whether every compared body sample pair had finite position and velocity.
    ///
    /// A caller making a qualification decision should normally require complete
    /// coverage. Partial metrics are useful diagnostically but do not prove that
    /// the entire trace met its tolerances.
    pub fn has_complete_finite_coverage(&self) -> bool {
        self.compared_bodies > 0
            && self.finite_comparison_pairs > 0
            && self.finite_comparison_pairs == self.compared_bodies
            && self.left_non_finite_observations == 0
            && self.right_non_finite_observations == 0
    }
}

/// Structural mismatch that makes two traces unsafe to compare numerically.
#[derive(Clone, Debug, PartialEq)]
pub enum TraceComparisonError {
    UnsupportedTraceSchemaVersion {
        left: u32,
        right: u32,
        supported: u32,
    },
    ScenarioMismatch,
    SampleCountMismatch,
    EmptyTrace,
    EmptyBodySample {
        step: u64,
    },
    InvalidSampleSequence {
        backend: String,
        sample_index: usize,
    },
    SampleStepMismatch {
        left: u64,
        right: u64,
    },
    NonFiniteSampleTime {
        step: u64,
    },
    NonFiniteComputedMetric {
        field: &'static str,
    },
    SampleTimeMismatch {
        step: u64,
        left: f64,
        right: f64,
    },
    DuplicateBodyObservation {
        step: u64,
        body_id: ValidationBodyId,
    },
    BodySetMismatch {
        step: u64,
    },
}

impl fmt::Display for TraceComparisonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTraceSchemaVersion {
                left,
                right,
                supported,
            } => write!(
                f,
                "unsupported trace schema version: left={left}, right={right}, supported={supported}"
            ),
            Self::ScenarioMismatch => write!(f, "trace scenario_id values differ"),
            Self::SampleCountMismatch => write!(f, "trace sample counts differ"),
            Self::EmptyTrace => write!(f, "trace must contain at least one sample"),
            Self::EmptyBodySample { step } => {
                write!(f, "trace sample at step {step} has no body observations")
            }
            Self::InvalidSampleSequence {
                backend,
                sample_index,
            } => write!(
                f,
                "trace from backend {backend} has invalid sample ordering at index {sample_index}"
            ),
            Self::SampleStepMismatch { left, right } => {
                write!(f, "trace sample steps differ: left={left}, right={right}")
            }
            Self::NonFiniteSampleTime { step } => {
                write!(f, "trace sample time is non-finite at step {step}")
            }
            Self::NonFiniteComputedMetric { field } => {
                write!(f, "computed differential metric {field} is non-finite")
            }
            Self::SampleTimeMismatch { step, left, right } => write!(
                f,
                "trace sample times differ at step {step}: left={left}, right={right}"
            ),
            Self::DuplicateBodyObservation { step, body_id } => {
                write!(f, "duplicate body observation {} at step {step}", body_id.0)
            }
            Self::BodySetMismatch { step } => {
                write!(f, "trace body identity sets differ at step {step}")
            }
        }
    }
}

impl std::error::Error for TraceComparisonError {}

/// Compare two traces strictly by schema, scenario, sample grid, and stable body ID.
///
/// Non-finite observations are counted and excluded from numeric maxima/RMS so
/// the report itself stays finite; their presence is an explicit hard signal
/// for qualification policy rather than being hidden inside NaN aggregates.
pub fn compare_traces_3d(
    left: &PhysicsTrace3d,
    right: &PhysicsTrace3d,
) -> Result<DifferentialMetrics3d, TraceComparisonError> {
    if left.schema_version != PHYSICS_TRACE_SCHEMA_VERSION
        || right.schema_version != PHYSICS_TRACE_SCHEMA_VERSION
    {
        return Err(TraceComparisonError::UnsupportedTraceSchemaVersion {
            left: left.schema_version,
            right: right.schema_version,
            supported: PHYSICS_TRACE_SCHEMA_VERSION,
        });
    }
    if left.scenario_id != right.scenario_id {
        return Err(TraceComparisonError::ScenarioMismatch);
    }
    if left.samples.len() != right.samples.len() {
        return Err(TraceComparisonError::SampleCountMismatch);
    }
    validate_sample_sequence(left)?;
    validate_sample_sequence(right)?;

    let mut metrics = DifferentialMetrics3d {
        scenario_id: left.scenario_id.clone(),
        left_backend: left.backend.clone(),
        right_backend: right.backend.clone(),
        compared_samples: 0,
        compared_bodies: 0,
        max_position_error_m: None,
        rms_position_error_m: None,
        max_linear_velocity_error_mps: None,
        rms_linear_velocity_error_mps: None,
        finite_comparison_pairs: 0,
        sleeping_mismatches: 0,
        left_non_finite_observations: 0,
        right_non_finite_observations: 0,
    };
    let mut position_error_squares = StableSumSquares::default();
    let mut velocity_error_squares = StableSumSquares::default();
    let mut max_position_error_m = 0.0_f64;
    let mut max_linear_velocity_error_mps = 0.0_f64;
    let mut finite_pairs = 0_u64;

    for (left_sample, right_sample) in left.samples.iter().zip(&right.samples) {
        if left_sample.bodies.is_empty() || right_sample.bodies.is_empty() {
            return Err(TraceComparisonError::EmptyBodySample {
                step: left_sample.step,
            });
        }
        if left_sample.step != right_sample.step {
            return Err(TraceComparisonError::SampleStepMismatch {
                left: left_sample.step,
                right: right_sample.step,
            });
        }
        if !left_sample.time_s.is_finite() || !right_sample.time_s.is_finite() {
            return Err(TraceComparisonError::NonFiniteSampleTime {
                step: left_sample.step,
            });
        }
        if left_sample.time_s != right_sample.time_s {
            return Err(TraceComparisonError::SampleTimeMismatch {
                step: left_sample.step,
                left: left_sample.time_s,
                right: right_sample.time_s,
            });
        }

        let left_bodies = body_map(left_sample)?;
        let right_bodies = body_map(right_sample)?;
        if left_bodies.keys().copied().collect::<Vec<_>>()
            != right_bodies.keys().copied().collect::<Vec<_>>()
        {
            return Err(TraceComparisonError::BodySetMismatch {
                step: left_sample.step,
            });
        }

        metrics.compared_samples += 1;
        for (body_id, left_body) in left_bodies {
            let right_body = right_bodies[&body_id];
            metrics.compared_bodies += 1;

            if left_body.sleeping != right_body.sleeping {
                metrics.sleeping_mismatches += 1;
            }

            let left_finite = left_body.is_finite();
            let right_finite = right_body.is_finite();
            if !left_finite {
                metrics.left_non_finite_observations += 1;
            }
            if !right_finite {
                metrics.right_non_finite_observations += 1;
            }
            if !left_finite || !right_finite {
                continue;
            }

            let position_error = euclidean_error(&left_body.position_m, &right_body.position_m);
            let velocity_error = euclidean_error(
                &left_body.linear_velocity_mps,
                &right_body.linear_velocity_mps,
            );
            if !position_error.is_finite() {
                return Err(TraceComparisonError::NonFiniteComputedMetric {
                    field: "position_error_m",
                });
            }
            if !velocity_error.is_finite() {
                return Err(TraceComparisonError::NonFiniteComputedMetric {
                    field: "linear_velocity_error_mps",
                });
            }
            max_position_error_m = max_position_error_m.max(position_error);
            max_linear_velocity_error_mps =
                max_linear_velocity_error_mps.max(velocity_error);
            position_error_squares.add(position_error);
            velocity_error_squares.add(velocity_error);
            finite_pairs += 1;
        }
    }

    metrics.finite_comparison_pairs = finite_pairs;
    if finite_pairs > 0 {
        metrics.max_position_error_m = Some(max_position_error_m);
        metrics.rms_position_error_m = Some(position_error_squares.rms(finite_pairs));
        metrics.max_linear_velocity_error_mps = Some(max_linear_velocity_error_mps);
        metrics.rms_linear_velocity_error_mps = Some(velocity_error_squares.rms(finite_pairs));
    }

    Ok(metrics)
}


/// Explicit acceptance thresholds for a differential comparison.
///
/// Thresholds are caller-owned and unit-specific; no universal epsilon is
/// implied. At least one metric must be selected. Unselected metrics are not
/// part of this profile's acceptance claim.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DifferentialAcceptanceProfile3d {
    pub profile_id: String,
    pub max_position_error_m: Option<f64>,
    pub rms_position_error_m: Option<f64>,
    pub max_linear_velocity_error_mps: Option<f64>,
    pub rms_linear_velocity_error_mps: Option<f64>,
}

impl DifferentialAcceptanceProfile3d {
    fn validate(&self) -> Result<(), AcceptanceProfileError> {
        if self.profile_id.trim().is_empty() {
            return Err(AcceptanceProfileError::EmptyProfileId);
        }
        let criteria = [
            ("max_position_error_m", self.max_position_error_m),
            ("rms_position_error_m", self.rms_position_error_m),
            (
                "max_linear_velocity_error_mps",
                self.max_linear_velocity_error_mps,
            ),
            (
                "rms_linear_velocity_error_mps",
                self.rms_linear_velocity_error_mps,
            ),
        ];
        let mut selected = 0;
        for (field, tolerance) in criteria {
            if let Some(tolerance) = tolerance {
                selected += 1;
                if !tolerance.is_finite() || tolerance < 0.0 {
                    return Err(AcceptanceProfileError::InvalidTolerance { field });
                }
            }
        }
        if selected == 0 {
            return Err(AcceptanceProfileError::NoCriteria);
        }
        Ok(())
    }
}

/// Invalid acceptance-profile configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AcceptanceProfileError {
    EmptyProfileId,
    NoCriteria,
    InvalidTolerance { field: &'static str },
}

impl fmt::Display for AcceptanceProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProfileId => write!(f, "acceptance profile ID must not be empty"),
            Self::NoCriteria => write!(f, "acceptance profile must select at least one metric"),
            Self::InvalidTolerance { field } => {
                write!(f, "acceptance tolerance {field} must be finite and nonnegative")
            }
        }
    }
}

impl std::error::Error for AcceptanceProfileError {}

/// Disposition of a differential physics comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationDecision3d {
    /// All selected thresholds were met with complete finite observation coverage.
    Pass,
    /// At least one measured selected metric exceeded its declared threshold.
    Fail,
    /// Some numeric evidence exists, but coverage or a required metric is incomplete.
    Inconclusive,
    /// The traces cannot be compared under the shared structural contract.
    NotComparable,
}

/// Decision record keeping the acceptance policy separate from raw metrics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QualificationReport3d {
    pub left_scenario_id: String,
    pub right_scenario_id: String,
    pub profile_id: String,
    pub decision: QualificationDecision3d,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<DifferentialMetrics3d>,
    pub reasons: Vec<String>,
}

/// Compare traces and evaluate them against explicit, unit-specific thresholds.
///
/// A structural comparison error becomes `not_comparable`. A threshold violation
/// is a failure even when other observations are missing; otherwise incomplete
/// finite coverage yields `inconclusive`, never a pass.
pub fn qualify_traces_3d(
    left: &PhysicsTrace3d,
    right: &PhysicsTrace3d,
    profile: &DifferentialAcceptanceProfile3d,
) -> Result<QualificationReport3d, AcceptanceProfileError> {
    profile.validate()?;
    let mut report = QualificationReport3d {
        left_scenario_id: left.scenario_id.clone(),
        right_scenario_id: right.scenario_id.clone(),
        profile_id: profile.profile_id.clone(),
        decision: QualificationDecision3d::NotComparable,
        metrics: None,
        reasons: Vec::new(),
    };

    let metrics = match compare_traces_3d(left, right) {
        Ok(metrics) => metrics,
        Err(error) => {
            report.reasons.push(error.to_string());
            return Ok(report);
        }
    };

    let criteria = [
        (
            "max_position_error_m",
            profile.max_position_error_m,
            metrics.max_position_error_m,
        ),
        (
            "rms_position_error_m",
            profile.rms_position_error_m,
            metrics.rms_position_error_m,
        ),
        (
            "max_linear_velocity_error_mps",
            profile.max_linear_velocity_error_mps,
            metrics.max_linear_velocity_error_mps,
        ),
        (
            "rms_linear_velocity_error_mps",
            profile.rms_linear_velocity_error_mps,
            metrics.rms_linear_velocity_error_mps,
        ),
    ];
    let mut failed = false;
    let mut missing_required_metric = false;
    for (field, tolerance, measured) in criteria {
        let Some(tolerance) = tolerance else {
            continue;
        };
        match measured {
            Some(measured) if measured > tolerance => {
                failed = true;
                report.reasons.push(format!(
                    "{field} exceeds tolerance: measured {measured}, allowed {tolerance}"
                ));
            }
            Some(_) => {}
            None => {
                missing_required_metric = true;
                report
                    .reasons
                    .push(format!("required metric {field} is unavailable"));
            }
        }
    }

    if failed {
        report.decision = QualificationDecision3d::Fail;
    } else if missing_required_metric || !metrics.has_complete_finite_coverage() {
        report.decision = QualificationDecision3d::Inconclusive;
        if !metrics.has_complete_finite_coverage() {
            report
                .reasons
                .push("observation coverage is not completely finite".to_owned());
        }
    } else {
        report.decision = QualificationDecision3d::Pass;
        report
            .reasons
            .push("all selected thresholds met with complete finite coverage".to_owned());
    }
    report.metrics = Some(metrics);
    Ok(report)
}

/// Execute a validated v0.1 scenario with the native Symtropy 3D solver.
pub fn run_native_scenario_3d(
    scenario: &PhysicsScenario3d,
) -> Result<PhysicsTrace3d, ScenarioValidationError> {
    let scenario = scenario.normalized()?;
    let mut world = PhysicsWorld::<3>::new(SVector::from(scenario.gravity_mps2));

    for spec in &scenario.bodies {
        let body = build_native_body(spec);
        let handle = world.add_body(body);
        world.set_net_id(handle, NetId(spec.id.0));
    }

    let mut samples = vec![capture_native_sample(&world, 0, 0.0)];
    for step in 1..=scenario.steps {
        world.step(scenario.dt_s);
        if should_sample(step, scenario.steps, scenario.sample_every_steps) {
            samples.push(capture_native_sample(
                &world,
                step,
                scenario.dt_s * step as f64,
            ));
        }
    }

    Ok(PhysicsTrace3d {
        schema_version: PHYSICS_TRACE_SCHEMA_VERSION,
        scenario_id: scenario.scenario_id,
        backend: PhysicsBackendDescriptor {
            engine: "symtropy-physics".to_owned(),
            engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            profile: "native-default-v0.1".to_owned(),
        },
        samples,
    })
}

fn validate_sample_sequence(trace: &PhysicsTrace3d) -> Result<(), TraceComparisonError> {
    let Some(first) = trace.samples.first() else {
        return Err(TraceComparisonError::EmptyTrace);
    };
    if !first.time_s.is_finite() {
        return Err(TraceComparisonError::NonFiniteSampleTime { step: first.step });
    }
    if first.step != 0 || first.time_s != 0.0 {
        return Err(TraceComparisonError::InvalidSampleSequence {
            backend: trace.backend.engine.clone(),
            sample_index: 0,
        });
    }

    for (index, pair) in trace.samples.windows(2).enumerate() {
        let previous = &pair[0];
        let current = &pair[1];
        if !current.time_s.is_finite() {
            return Err(TraceComparisonError::NonFiniteSampleTime { step: current.step });
        }
        if current.step <= previous.step || current.time_s <= previous.time_s {
            return Err(TraceComparisonError::InvalidSampleSequence {
                backend: trace.backend.engine.clone(),
                sample_index: index + 1,
            });
        }
    }
    Ok(())
}

fn body_map(
    sample: &PhysicsSample3d,
) -> Result<BTreeMap<ValidationBodyId, &BodyObservation3d>, TraceComparisonError> {
    let mut map = BTreeMap::new();
    for body in &sample.bodies {
        if map.insert(body.body_id, body).is_some() {
            return Err(TraceComparisonError::DuplicateBodyObservation {
                step: sample.step,
                body_id: body.body_id,
            });
        }
    }
    Ok(map)
}

fn build_native_body(spec: &ScenarioBody3d) -> RigidBody<3> {
    let position = Point::<3>::new(spec.position_m);
    let collider: Box<dyn Shape<3>> = match spec.shape {
        ScenarioShape3d::Sphere { radius_m } => Box::new(Sphere::new(Point::origin(), radius_m)),
        ScenarioShape3d::Cuboid { half_extents_m } => Box::new(HyperBox::new(half_extents_m)),
    };

    let mut body = match spec.body_type {
        ScenarioBodyType::Static => RigidBody::static_body(BodyHandle(0), position, collider),
        ScenarioBodyType::Dynamic => {
            let inertia = native_principal_inertia(spec);
            RigidBody::new(
                BodyHandle(0),
                BodyType::Dynamic,
                Transform::from_translation(position),
                collider,
                spec.mass_kg,
                inertia,
            )
        }
    };

    body.linear_velocity = SVector::from(spec.linear_velocity_mps);
    body.friction = spec.friction;
    body.restitution = spec.restitution;
    body
}

fn native_principal_inertia(spec: &ScenarioBody3d) -> SVector<f64, 3> {
    match spec.shape {
        ScenarioShape3d::Sphere { radius_m } => {
            SVector::from_element(0.4 * spec.mass_kg * radius_m * radius_m)
        }
        ScenarioShape3d::Cuboid { half_extents_m } => {
            let [hx, hy, hz] = half_extents_m;
            // I = m/12 * (full_side_a² + full_side_b²)
            //   = m/3  * (half_extent_a² + half_extent_b²).
            SVector::from([
                spec.mass_kg / 3.0 * (hy * hy + hz * hz),
                spec.mass_kg / 3.0 * (hx * hx + hz * hz),
                spec.mass_kg / 3.0 * (hx * hx + hy * hy),
            ])
        }
    }
}

fn capture_native_sample(world: &PhysicsWorld<3>, step: u64, time_s: f64) -> PhysicsSample3d {
    let mut bodies: Vec<_> = world
        .bodies
        .iter()
        .map(|body| {
            let net_id = body
                .net_id
                .expect("validation runner assigns every body a stable NetId");
            let position = body.position();
            BodyObservation3d {
                body_id: ValidationBodyId(net_id.0),
                position_m: [position[0], position[1], position[2]],
                linear_velocity_mps: [
                    body.linear_velocity[0],
                    body.linear_velocity[1],
                    body.linear_velocity[2],
                ],
                sleeping: body.sleeping,
            }
        })
        .collect();
    bodies.sort_by_key(|body| body.body_id);

    PhysicsSample3d {
        step,
        time_s,
        bodies,
    }
}

fn should_sample(step: u64, final_step: u64, stride: u64) -> bool {
    step == final_step || step.is_multiple_of(stride)
}

fn all_finite<const N: usize>(values: &[f64; N]) -> bool {
    values.iter().all(|value| value.is_finite())
}

/// Scaled sum-of-squares accumulator for an RMS without avoidable intermediate
/// overflow. This is the same numerical strategy used by stable norm algorithms:
/// keep the largest magnitude separate and accumulate only ratios in [0, 1].
#[derive(Clone, Copy, Debug, Default)]
struct StableSumSquares {
    scale: f64,
    scaled_sum_squares: f64,
}

impl StableSumSquares {
    fn add(&mut self, value: f64) {
        let value = value.abs();
        if value == 0.0 {
            return;
        }

        if self.scale < value {
            let ratio = if self.scale == 0.0 {
                0.0
            } else {
                self.scale / value
            };
            self.scaled_sum_squares = 1.0 + self.scaled_sum_squares * ratio * ratio;
            self.scale = value;
        } else {
            let ratio = value / self.scale;
            self.scaled_sum_squares += ratio * ratio;
        }
    }

    fn rms(self, count: u64) -> f64 {
        if self.scale == 0.0 {
            0.0
        } else {
            self.scale * (self.scaled_sum_squares / count as f64).sqrt()
        }
    }
}

/// Use hypot to avoid overflowing while squaring otherwise representable deltas.
fn euclidean_error<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter()
        .zip(right)
        .fold(0.0_f64, |norm, (left, right)| norm.hypot(left - right))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dynamic_sphere(id: u64, y: f64) -> ScenarioBody3d {
        ScenarioBody3d {
            id: ValidationBodyId(id),
            body_type: ScenarioBodyType::Dynamic,
            shape: ScenarioShape3d::Sphere { radius_m: 0.5 },
            position_m: [0.0, y, 0.0],
            linear_velocity_mps: [0.0, 0.0, 0.0],
            mass_kg: 1.0,
            friction: 0.5,
            restitution: 0.0,
        }
    }

    fn free_fall_scenario() -> PhysicsScenario3d {
        PhysicsScenario3d {
            schema_version: PHYSICS_SCENARIO_SCHEMA_VERSION,
            scenario_id: "free-fall-v0".to_owned(),
            gravity_mps2: [0.0, -9.81, 0.0],
            dt_s: 0.01,
            steps: 10,
            sample_every_steps: 5,
            bodies: vec![dynamic_sphere(7, 10.0)],
        }
    }

    fn one_sample_trace() -> PhysicsTrace3d {
        PhysicsTrace3d {
            schema_version: PHYSICS_TRACE_SCHEMA_VERSION,
            scenario_id: "identity".to_owned(),
            backend: PhysicsBackendDescriptor {
                engine: "test".to_owned(),
                engine_version: "1".to_owned(),
                profile: "default".to_owned(),
            },
            samples: vec![PhysicsSample3d {
                step: 0,
                time_s: 0.0,
                bodies: vec![BodyObservation3d {
                    body_id: ValidationBodyId(1),
                    position_m: [1.0, 0.0, 0.0],
                    linear_velocity_mps: [0.0, 0.0, 0.0],
                    sleeping: false,
                }],
            }],
        }
    }

    #[test]
    fn validation_rejects_duplicate_body_ids() {
        let mut scenario = free_fall_scenario();
        scenario.bodies.push(dynamic_sphere(7, 12.0));
        assert_eq!(
            scenario.validate(),
            Err(ScenarioValidationError::DuplicateBodyId(ValidationBodyId(
                7
            )))
        );
    }

    #[test]
    fn validation_rejects_total_duration_overflow() {
        let mut scenario = free_fall_scenario();
        scenario.dt_s = f64::MAX;
        scenario.steps = 2;
        assert_eq!(
            scenario.validate(),
            Err(ScenarioValidationError::InvalidSimulationDuration)
        );
    }

    #[test]
    fn validation_rejects_non_finite_and_ambiguous_inputs() {
        let mut scenario = free_fall_scenario();
        scenario.gravity_mps2[0] = f64::NAN;
        assert_eq!(
            scenario.validate(),
            Err(ScenarioValidationError::NonFiniteGravity)
        );

        let mut scenario = free_fall_scenario();
        scenario.bodies[0].restitution = 1.01;
        assert_eq!(
            scenario.validate(),
            Err(ScenarioValidationError::InvalidRestitution(
                ValidationBodyId(7)
            ))
        );
    }

    #[test]
    fn normalization_orders_bodies_by_stable_identity() {
        let mut scenario = free_fall_scenario();
        scenario.bodies = vec![dynamic_sphere(9, 9.0), dynamic_sphere(2, 2.0)];
        let normalized = scenario.normalized().unwrap();
        assert_eq!(normalized.bodies[0].id, ValidationBodyId(2));
        assert_eq!(normalized.bodies[1].id, ValidationBodyId(9));
    }

    #[test]
    fn native_runner_advances_free_fall_and_always_samples_final_step() {
        let trace = run_native_scenario_3d(&free_fall_scenario()).unwrap();
        assert_eq!(trace.samples.len(), 3);
        assert_eq!(trace.samples[0].step, 0);
        assert_eq!(trace.samples[1].step, 5);
        assert_eq!(trace.samples[2].step, 10);
        assert!(trace.samples[2].bodies[0].position_m[1] < 10.0);
        assert!(trace.samples[2].bodies[0].linear_velocity_mps[1] < 0.0);
    }

    #[test]
    fn native_static_body_does_not_move_under_gravity() {
        let mut scenario = free_fall_scenario();
        scenario.bodies = vec![ScenarioBody3d {
            id: ValidationBodyId(1),
            body_type: ScenarioBodyType::Static,
            shape: ScenarioShape3d::Cuboid {
                half_extents_m: [1.0, 0.5, 1.0],
            },
            position_m: [0.0, 3.0, 0.0],
            linear_velocity_mps: [0.0, 0.0, 0.0],
            mass_kg: 0.0,
            friction: 0.5,
            restitution: 0.0,
        }];

        let trace = run_native_scenario_3d(&scenario).unwrap();
        assert_eq!(
            trace.final_sample().unwrap().bodies[0].position_m,
            [0.0, 3.0, 0.0]
        );
    }

    #[test]
    fn trace_comparison_matches_by_body_id_not_vector_order() {
        let body_a = BodyObservation3d {
            body_id: ValidationBodyId(1),
            position_m: [1.0, 0.0, 0.0],
            linear_velocity_mps: [0.0, 0.0, 0.0],
            sleeping: false,
        };
        let body_b = BodyObservation3d {
            body_id: ValidationBodyId(2),
            position_m: [2.0, 0.0, 0.0],
            linear_velocity_mps: [0.0, 0.0, 0.0],
            sleeping: true,
        };
        let backend = PhysicsBackendDescriptor {
            engine: "test".to_owned(),
            engine_version: "1".to_owned(),
            profile: "default".to_owned(),
        };
        let left = PhysicsTrace3d {
            schema_version: PHYSICS_TRACE_SCHEMA_VERSION,
            scenario_id: "identity".to_owned(),
            backend: backend.clone(),
            samples: vec![PhysicsSample3d {
                step: 0,
                time_s: 0.0,
                bodies: vec![body_a.clone(), body_b.clone()],
            }],
        };
        let right = PhysicsTrace3d {
            backend,
            samples: vec![PhysicsSample3d {
                step: 0,
                time_s: 0.0,
                bodies: vec![body_b, body_a],
            }],
            ..left.clone()
        };

        let metrics = compare_traces_3d(&left, &right).unwrap();
        assert_eq!(metrics.compared_bodies, 2);
        assert_eq!(metrics.finite_comparison_pairs, 2);
        assert!(metrics.has_complete_finite_coverage());
        assert_eq!(metrics.max_position_error_m, Some(0.0));
        assert_eq!(metrics.max_linear_velocity_error_mps, Some(0.0));
        assert_eq!(metrics.sleeping_mismatches, 0);
    }

    #[test]
    fn trace_comparison_rejects_samples_without_body_observations() {
        let mut left = one_sample_trace();
        left.samples[0].bodies.clear();
        let right = left.clone();

        assert_eq!(
            compare_traces_3d(&left, &right),
            Err(TraceComparisonError::EmptyBodySample { step: 0 })
        );
    }

    #[test]
    fn trace_comparison_rejects_empty_and_nonmonotonic_traces() {
        let mut empty = one_sample_trace();
        empty.samples.clear();
        assert_eq!(
            compare_traces_3d(&empty, &empty.clone()),
            Err(TraceComparisonError::EmptyTrace)
        );

        let mut repeated_step = one_sample_trace();
        repeated_step.samples.push(repeated_step.samples[0].clone());
        let expected = TraceComparisonError::InvalidSampleSequence {
            backend: "test".to_owned(),
            sample_index: 1,
        };
        assert_eq!(
            compare_traces_3d(&repeated_step, &repeated_step.clone()),
            Err(expected)
        );
    }

    #[test]
    fn trace_comparison_rejects_unknown_schema_version() {
        let left = one_sample_trace();
        let mut right = left.clone();
        right.schema_version = PHYSICS_TRACE_SCHEMA_VERSION + 1;
        assert_eq!(
            compare_traces_3d(&left, &right),
            Err(TraceComparisonError::UnsupportedTraceSchemaVersion {
                left: PHYSICS_TRACE_SCHEMA_VERSION,
                right: PHYSICS_TRACE_SCHEMA_VERSION + 1,
                supported: PHYSICS_TRACE_SCHEMA_VERSION,
            })
        );
    }

    #[test]
    fn trace_comparison_rejects_mismatched_sample_time() {
        let mut left = one_sample_trace();
        let mut later_sample = left.samples[0].clone();
        later_sample.step = 1;
        later_sample.time_s = 1.0;
        left.samples.push(later_sample);

        let mut right = left.clone();
        right.samples[1].time_s = 0.5;
        assert_eq!(
            compare_traces_3d(&left, &right),
            Err(TraceComparisonError::SampleTimeMismatch {
                step: 1,
                left: 1.0,
                right: 0.5,
            })
        );
    }

    #[test]
    fn trace_comparison_rejects_duplicate_body_observations() {
        let left = one_sample_trace();
        let mut right = left.clone();
        right.samples[0]
            .bodies
            .push(right.samples[0].bodies[0].clone());
        assert_eq!(
            compare_traces_3d(&left, &right),
            Err(TraceComparisonError::DuplicateBodyObservation {
                step: 0,
                body_id: ValidationBodyId(1),
            })
        );
    }

    #[test]
    fn partial_finite_coverage_reports_metrics_but_cannot_claim_complete_coverage() {
        let mut left = one_sample_trace();
        left.samples[0].bodies[0].position_m = [f64::NAN, 0.0, 0.0];
        let mut second = left.samples[0].bodies[0].clone();
        second.body_id = ValidationBodyId(2);
        second.position_m = [2.0, 0.0, 0.0];
        left.samples[0].bodies.push(second);

        let mut right = left.clone();
        right.samples[0].bodies[0].position_m = [0.0, 0.0, 0.0];

        let metrics = compare_traces_3d(&left, &right).unwrap();
        assert_eq!(metrics.compared_bodies, 2);
        assert_eq!(metrics.finite_comparison_pairs, 1);
        assert_eq!(metrics.left_non_finite_observations, 1);
        assert_eq!(metrics.right_non_finite_observations, 0);
        assert!(!metrics.has_complete_finite_coverage());
        assert_eq!(metrics.max_position_error_m, Some(0.0));
        assert_eq!(metrics.rms_position_error_m, Some(0.0));
    }

    #[test]
    fn finite_components_with_overflowing_error_are_rejected() {
        let mut left = one_sample_trace();
        left.samples[0].bodies[0].position_m = [f64::MAX, 0.0, 0.0];
        let mut right = left.clone();
        right.samples[0].bodies[0].position_m = [-f64::MAX, 0.0, 0.0];

        assert_eq!(
            compare_traces_3d(&left, &right),
            Err(TraceComparisonError::NonFiniteComputedMetric {
                field: "position_error_m",
            })
        );
    }

    #[test]
    fn rms_avoids_intermediate_overflow_when_the_aggregate_is_representable() {
        let mut left = one_sample_trace();
        left.samples[0].bodies[0].position_m = [1.0e154, 0.0, 0.0];
        let mut second = left.samples[0].bodies[0].clone();
        second.body_id = ValidationBodyId(2);
        left.samples[0].bodies.push(second);

        let mut right = left.clone();
        for body in &mut right.samples[0].bodies {
            body.position_m = [0.0, 0.0, 0.0];
        }

        let metrics = compare_traces_3d(&left, &right).unwrap();
        assert_eq!(metrics.max_position_error_m, Some(1.0e154));
        assert_eq!(metrics.rms_position_error_m, Some(1.0e154));
        assert_eq!(metrics.finite_comparison_pairs, 2);
        assert!(metrics.has_complete_finite_coverage());
    }

    fn test_acceptance_profile() -> DifferentialAcceptanceProfile3d {
        DifferentialAcceptanceProfile3d {
            profile_id: "test-profile-v1".to_owned(),
            max_position_error_m: Some(0.1),
            rms_position_error_m: Some(0.1),
            max_linear_velocity_error_mps: Some(0.1),
            rms_linear_velocity_error_mps: Some(0.1),
        }
    }

    #[test]
    fn qualification_passes_only_with_selected_thresholds_and_full_coverage() {
        let trace = one_sample_trace();
        let report = qualify_traces_3d(&trace, &trace, &test_acceptance_profile()).unwrap();

        assert_eq!(report.decision, QualificationDecision3d::Pass);
        assert!(report.metrics.as_ref().unwrap().has_complete_finite_coverage());
    }

    #[test]
    fn qualification_fails_when_a_measured_metric_exceeds_its_threshold() {
        let left = one_sample_trace();
        let mut right = left.clone();
        right.samples[0].bodies[0].position_m = [2.0, 0.0, 0.0];

        let report = qualify_traces_3d(&left, &right, &test_acceptance_profile()).unwrap();
        assert_eq!(report.decision, QualificationDecision3d::Fail);
        assert!(report.reasons.iter().any(|reason| reason.contains("max_position_error_m")));
    }

    #[test]
    fn qualification_is_inconclusive_when_finite_coverage_is_partial() {
        let mut left = one_sample_trace();
        left.samples[0].bodies[0].position_m = [f64::NAN, 0.0, 0.0];
        let right = one_sample_trace();

        let report = qualify_traces_3d(&left, &right, &test_acceptance_profile()).unwrap();
        assert_eq!(report.decision, QualificationDecision3d::Inconclusive);
        assert!(
            !report
                .metrics
                .as_ref()
                .unwrap()
                .has_complete_finite_coverage()
        );
    }

    #[test]
    fn qualification_reports_structurally_invalid_traces_as_not_comparable() {
        let mut left = one_sample_trace();
        left.samples[0].bodies.clear();
        let right = left.clone();

        let report = qualify_traces_3d(&left, &right, &test_acceptance_profile()).unwrap();
        assert_eq!(report.decision, QualificationDecision3d::NotComparable);
        assert!(report.metrics.is_none());
        assert!(!report.reasons.is_empty());
    }

    #[test]
    fn acceptance_profiles_reject_empty_or_invalid_thresholds() {
        let mut profile = test_acceptance_profile();
        profile.profile_id = " ".to_owned();
        assert_eq!(
            qualify_traces_3d(&one_sample_trace(), &one_sample_trace(), &profile),
            Err(AcceptanceProfileError::EmptyProfileId)
        );

        let mut profile = test_acceptance_profile();
        profile.max_position_error_m = Some(f64::INFINITY);
        assert_eq!(
            qualify_traces_3d(&one_sample_trace(), &one_sample_trace(), &profile),
            Err(AcceptanceProfileError::InvalidTolerance {
                field: "max_position_error_m",
            })
        );
    }

    #[test]
    fn non_finite_observation_is_explicit_instead_of_poisoning_metrics() {
        let mut left = one_sample_trace();
        left.scenario_id = "nan".to_owned();
        left.samples[0].bodies[0].position_m = [f64::NAN, 0.0, 0.0];
        let mut right = left.clone();
        right.samples[0].bodies[0].position_m = [0.0, 0.0, 0.0];

        let metrics = compare_traces_3d(&left, &right).unwrap();
        assert_eq!(metrics.left_non_finite_observations, 1);
        assert_eq!(metrics.right_non_finite_observations, 0);
        assert_eq!(metrics.finite_comparison_pairs, 0);
        assert!(!metrics.has_complete_finite_coverage());
        assert_eq!(metrics.max_position_error_m, None);
        assert_eq!(metrics.rms_position_error_m, None);
        assert_eq!(metrics.max_linear_velocity_error_mps, None);
        assert_eq!(metrics.rms_linear_velocity_error_mps, None);
    }
}
