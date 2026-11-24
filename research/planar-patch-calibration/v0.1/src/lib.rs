// SPDX-License-Identifier: AGPL-3.0-or-later
//! Weighted calibration and identifiability reference for aggregate planar
//! friction-wrench ellipsoids.
//!
//! For each declared sliding/boundary observation:
//!
//!   (f_u / (c e_u))^2 + (f_v / (c e_v))^2
//!       + (m_n / (c L (e_n/L)))^2 = 1
//!
//! where c = mu * N and characteristic length L are supplied per observation
//! profile. Define beta = [1/e_u^2, 1/e_v^2, 1/(e_n/L)^2] and
//! x = [ (f_u/c)^2, (f_v/c)^2, (m_n/(c L))^2 ]. Then x beta = 1.
//!
//! This crate uses weighted, pivoted, twice-reorthogonalized MGS to diagnose
//! rank and solve the three-parameter least-squares fit. It deliberately does
//! not regularize missing excitation into point estimates.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const SCHEMA_VERSION: &str = "planar-patch-calibration-v0.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalibrationErrorKind {
    EmptyObservations,
    InvalidIdentity,
    DuplicateObservationId,
    InvalidParameter,
    NonFiniteInput,
    InvalidTolerance,
    NonFiniteEvaluation,
    RankDeficient,
    NonPositiveScaleEstimate,
    LinearSolveFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationError {
    pub kind: CalibrationErrorKind,
    pub reason: String,
}

impl CalibrationError {
    fn new(kind: CalibrationErrorKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            reason: reason.into(),
        }
    }
}

impl Display for CalibrationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl Error for CalibrationError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeasuredPatchWrench {
    pub tangent_u_force: f64,
    pub tangent_v_force: f64,
    pub normal_torsional_moment: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryObservation {
    /// Unique observation identity, not merely a trial-group label.
    pub observation_id: String,
    /// Repeated observations may share a trial group; preserve that grouping.
    pub trial_group_id: String,
    pub measured_wrench: MeasuredPatchWrench,
    /// mu * N for this observation. Must be independently measured/supplied.
    pub capacity_scale: f64,
    /// Positive inverse-variance or preregistered relative observation weight.
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatchCalibrationInput {
    pub specimen_digest: String,
    pub patch_geometry_digest: String,
    pub coordinate_frame_digest: String,
    pub rig_calibration_digest: String,
    pub observation_profile_digest: String,
    /// Characteristic patch length in metres, used to nondimensionalize torsion.
    pub characteristic_length_m: f64,
    pub observations: Vec<BoundaryObservation>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalibrationConfig {
    /// Relative threshold for pivoted design-matrix rank detection.
    pub rank_relative_tolerance: f64,
    /// Maximum absolute boundary-equation residual for the good-fit disposition.
    pub maximum_boundary_residual: f64,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            rank_relative_tolerance: 1.0e-10,
            maximum_boundary_residual: 1.0e-3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalibrationStatus {
    IdentifiedWithinResidualTolerance,
    IdentifiedPoorBoundaryFit,
    Unidentifiable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PatchAxis {
    TangentialU,
    TangentialV,
    TorsionalNormal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AxisExcitation {
    pub axis: PatchAxis,
    pub weighted_column_norm: f64,
    pub final_projection_residual: f64,
    pub relative_projection_residual: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatchCalibrationReport {
    pub schema_version: String,
    pub specimen_digest: String,
    pub patch_geometry_digest: String,
    pub coordinate_frame_digest: String,
    pub rig_calibration_digest: String,
    pub observation_profile_digest: String,
    pub characteristic_length_m: f64,
    pub observation_count: usize,
    pub unique_trial_group_count: usize,
    pub design_rank: usize,
    pub rank_threshold: f64,
    /// Greedy pivot order of [u-force, v-force, normal-torsion] design columns.
    pub pivot_order: Vec<PatchAxis>,
    pub pivot_residual_norms: Vec<f64>,
    pub axis_excitation: Vec<AxisExcitation>,
    pub status: CalibrationStatus,
    /// Dimensionless beta = [1/e_u^2, 1/e_v^2, 1/(e_n/L)^2].
    /// None for a rank-deficient or invalid fit.
    pub inverse_squared_dimensionless_shape_scales: Option<[f64; 3]>,
    /// e = [e_u, e_v, e_n]. e_n is in metres when input moments are N·m.
    pub ellipsoid_shape_scales: Option<[f64; 3]>,
    /// Maximum absolute residual in the normalized boundary equation.
    pub maximum_absolute_boundary_residual: Option<f64>,
    /// Weighted RMS residual in the normalized boundary equation.
    pub weighted_rms_boundary_residual: Option<f64>,
    pub rank_relative_tolerance: f64,
    pub maximum_boundary_residual_tolerance: f64,
    /// A fitted result is not physical validation. No arbitrary robust model is inferred.
    pub physical_model_validated: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaximumDissipationObservation {
    pub observation_id: String,
    pub trial_group_id: String,
    /// Local [tangent-u velocity, tangent-v velocity, normal angular velocity].
    pub slip_twist: [f64; 3],
    pub measured_wrench: MeasuredPatchWrench,
    /// Independently measured mu * N for this observation.
    pub capacity_scale: f64,
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaximumDissipationValidationInput {
    pub specimen_digest: String,
    pub patch_geometry_digest: String,
    pub coordinate_frame_digest: String,
    pub rig_calibration_digest: String,
    pub observation_profile_digest: String,
    pub calibration_parameter_digest: String,
    pub held_out_split_digest: String,
    pub characteristic_length_m: f64,
    /// [e_u, e_v, e_n] where e_n is in metres.
    pub ellipsoid_shape_scales: [f64; 3],
    pub observations: Vec<MaximumDissipationObservation>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaximumDissipationValidationConfig {
    /// Treat ||diag(a) v|| at or below this value as zero slip (watts).
    pub slip_power_tolerance: f64,
    /// Maximum tolerated positive measured power (watts).
    pub positive_power_tolerance: f64,
    /// Maximum weighted RMS normalized wrench error for a consistent result.
    pub maximum_weighted_rms_wrench_error: f64,
}

impl Default for MaximumDissipationValidationConfig {
    fn default() -> Self {
        Self {
            slip_power_tolerance: 1.0e-12,
            positive_power_tolerance: 1.0e-8,
            maximum_weighted_rms_wrench_error: 1.0e-3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectionObservationDisposition {
    SlidingDirectionEvaluated,
    ZeroSlipStickingSetNotDirectional,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DirectionObservationDiagnostic {
    pub observation_id: String,
    pub trial_group_id: String,
    pub disposition: DirectionObservationDisposition,
    pub predicted_wrench: Option<MeasuredPatchWrench>,
    pub observed_power: Option<f64>,
    pub predicted_power: Option<f64>,
    /// Dimensionless L2 error after dividing each channel by its ellipsoid axis.
    pub normalized_wrench_error: Option<f64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaximumDissipationValidationStatus {
    ConsistentWithinTolerance,
    Inconsistent,
    NoInformativeSlidingObservations,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaximumDissipationValidationReport {
    pub schema_version: String,
    pub specimen_digest: String,
    pub patch_geometry_digest: String,
    pub coordinate_frame_digest: String,
    pub rig_calibration_digest: String,
    pub observation_profile_digest: String,
    pub calibration_parameter_digest: String,
    pub held_out_split_digest: String,
    pub characteristic_length_m: f64,
    pub ellipsoid_shape_scales: [f64; 3],
    pub observation_count: usize,
    pub unique_trial_group_count: usize,
    pub sliding_observation_count: usize,
    pub zero_slip_observation_count: usize,
    pub non_dissipative_observation_count: usize,
    pub maximum_positive_observed_power: Option<f64>,
    pub maximum_normalized_wrench_error: Option<f64>,
    pub weighted_rms_normalized_wrench_error: Option<f64>,
    pub slip_power_tolerance: f64,
    pub positive_power_tolerance: f64,
    pub maximum_weighted_rms_wrench_error: f64,
    pub status: MaximumDissipationValidationStatus,
    pub diagnostics: Vec<DirectionObservationDiagnostic>,
}

/// Validate calibrated ellipsoid shape scales against held-out sliding
/// force-torque / slip-twist observations. Zero-slip data are retained as
/// set-valued sticking diagnostics and excluded from the directional score.
pub fn validate_maximum_dissipation(
    input: &MaximumDissipationValidationInput,
    config: MaximumDissipationValidationConfig,
) -> Result<MaximumDissipationValidationReport, CalibrationError> {
    for (label, digest) in [
        ("specimen", &input.specimen_digest),
        ("patch geometry", &input.patch_geometry_digest),
        ("coordinate frame", &input.coordinate_frame_digest),
        ("rig calibration", &input.rig_calibration_digest),
        ("observation profile", &input.observation_profile_digest),
        (
            "calibration parameters",
            &input.calibration_parameter_digest,
        ),
        ("held-out split", &input.held_out_split_digest),
    ] {
        if digest.trim().is_empty() {
            return Err(CalibrationError::new(
                CalibrationErrorKind::InvalidIdentity,
                format!("{label} digest must be non-empty"),
            ));
        }
    }
    if !input.characteristic_length_m.is_finite()
        || input.characteristic_length_m <= 0.0
        || !input
            .ellipsoid_shape_scales
            .iter()
            .all(|value| value.is_finite() && *value > 0.0)
    {
        return Err(CalibrationError::new(
            CalibrationErrorKind::InvalidParameter,
            "characteristic length and calibrated ellipsoid shape scales must be finite and positive",
        ));
    }
    if !config.slip_power_tolerance.is_finite()
        || config.slip_power_tolerance < 0.0
        || !config.positive_power_tolerance.is_finite()
        || config.positive_power_tolerance < 0.0
        || !config.maximum_weighted_rms_wrench_error.is_finite()
        || config.maximum_weighted_rms_wrench_error <= 0.0
    {
        return Err(CalibrationError::new(
            CalibrationErrorKind::InvalidTolerance,
            "directional validation tolerances must be finite and non-negative, with positive RMS tolerance",
        ));
    }
    if input.observations.is_empty() {
        return Err(CalibrationError::new(
            CalibrationErrorKind::EmptyObservations,
            "at least one held-out direction observation is required",
        ));
    }

    let mut ids = BTreeSet::new();
    let mut trial_groups = BTreeSet::new();
    let mut diagnostics = Vec::with_capacity(input.observations.len());
    let mut sliding_count = 0_usize;
    let mut zero_slip_count = 0_usize;
    let mut non_dissipative_count = 0_usize;
    let mut maximum_positive_power = 0.0_f64;
    let mut maximum_error = 0.0_f64;
    let mut weighted_error_sum = 0.0_f64;
    let mut total_sliding_weight = 0.0_f64;

    for observation in &input.observations {
        if observation.observation_id.trim().is_empty()
            || observation.trial_group_id.trim().is_empty()
        {
            return Err(CalibrationError::new(
                CalibrationErrorKind::InvalidIdentity,
                "direction observation and trial-group IDs must be non-empty",
            ));
        }
        if !ids.insert(observation.observation_id.as_str()) {
            return Err(CalibrationError::new(
                CalibrationErrorKind::DuplicateObservationId,
                "direction observation IDs must be unique",
            ));
        }
        trial_groups.insert(observation.trial_group_id.as_str());
        if !observation.slip_twist.iter().all(|value| value.is_finite())
            || !finite_wrench(observation.measured_wrench)
        {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteInput,
                "slip twist and measured wrench must be finite",
            ));
        }
        if !observation.capacity_scale.is_finite()
            || observation.capacity_scale <= 0.0
            || !observation.weight.is_finite()
            || observation.weight <= 0.0
        {
            return Err(CalibrationError::new(
                CalibrationErrorKind::InvalidParameter,
                "capacity scale and observation weight must be finite and positive",
            ));
        }

        let axes = input
            .ellipsoid_shape_scales
            .map(|shape| shape * observation.capacity_scale);
        if !axes.iter().all(|value| value.is_finite() && *value > 0.0) {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "directional validation ellipsoid axes are invalid",
            ));
        }
        let scaled_slip =
            std::array::from_fn::<_, 3, _>(|index| axes[index] * observation.slip_twist[index]);
        if !scaled_slip.iter().all(|value| value.is_finite()) {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "capacity-scaled slip twist is non-finite",
            ));
        }
        let slip_power_norm = stable_norm(&scaled_slip);
        if !slip_power_norm.is_finite() {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "slip-power norm is non-finite",
            ));
        }

        if slip_power_norm <= config.slip_power_tolerance {
            zero_slip_count += 1;
            diagnostics.push(DirectionObservationDiagnostic {
                observation_id: observation.observation_id.clone(),
                trial_group_id: observation.trial_group_id.clone(),
                disposition: DirectionObservationDisposition::ZeroSlipStickingSetNotDirectional,
                predicted_wrench: None,
                observed_power: None,
                predicted_power: None,
                normalized_wrench_error: None,
            });
            continue;
        }

        let predicted_values = std::array::from_fn::<_, 3, _>(|index| {
            -(axes[index] * scaled_slip[index]) / slip_power_norm
        });
        if !predicted_values.iter().all(|value| value.is_finite()) {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "predicted maximum-dissipation wrench is non-finite",
            ));
        }
        let measured_values = [
            observation.measured_wrench.tangent_u_force,
            observation.measured_wrench.tangent_v_force,
            observation.measured_wrench.normal_torsional_moment,
        ];
        let difference = std::array::from_fn::<_, 3, _>(|index| {
            (measured_values[index] - predicted_values[index]) / axes[index]
        });
        if !difference.iter().all(|value| value.is_finite()) {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "normalized maximum-dissipation residual is non-finite",
            ));
        }
        let normalized_error = stable_norm(&difference);
        let observed_power = dot(&observation.slip_twist, &measured_values)?;
        let predicted_power = dot(&observation.slip_twist, &predicted_values)?;
        if !normalized_error.is_finite()
            || !observed_power.is_finite()
            || !predicted_power.is_finite()
        {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "maximum-dissipation residual or power is non-finite",
            ));
        }
        if observed_power > config.positive_power_tolerance {
            non_dissipative_count += 1;
            maximum_positive_power = maximum_positive_power.max(observed_power);
        }
        maximum_error = maximum_error.max(normalized_error);
        weighted_error_sum += observation.weight * normalized_error * normalized_error;
        total_sliding_weight += observation.weight;
        if !weighted_error_sum.is_finite() || !total_sliding_weight.is_finite() {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "weighted maximum-dissipation residual accumulation overflowed",
            ));
        }
        sliding_count += 1;
        diagnostics.push(DirectionObservationDiagnostic {
            observation_id: observation.observation_id.clone(),
            trial_group_id: observation.trial_group_id.clone(),
            disposition: DirectionObservationDisposition::SlidingDirectionEvaluated,
            predicted_wrench: Some(MeasuredPatchWrench {
                tangent_u_force: predicted_values[0],
                tangent_v_force: predicted_values[1],
                normal_torsional_moment: predicted_values[2],
            }),
            observed_power: Some(observed_power),
            predicted_power: Some(predicted_power),
            normalized_wrench_error: Some(normalized_error),
        });
    }

    let weighted_rms = if sliding_count > 0 {
        Some((weighted_error_sum / total_sliding_weight).sqrt())
    } else {
        None
    };
    if weighted_rms.is_some_and(|value| !value.is_finite()) {
        return Err(CalibrationError::new(
            CalibrationErrorKind::NonFiniteEvaluation,
            "weighted maximum-dissipation RMS is non-finite",
        ));
    }
    let status = match weighted_rms {
        None => MaximumDissipationValidationStatus::NoInformativeSlidingObservations,
        Some(value)
            if value <= config.maximum_weighted_rms_wrench_error && non_dissipative_count == 0 =>
        {
            MaximumDissipationValidationStatus::ConsistentWithinTolerance
        }
        Some(_) => MaximumDissipationValidationStatus::Inconsistent,
    };

    Ok(MaximumDissipationValidationReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        specimen_digest: input.specimen_digest.clone(),
        patch_geometry_digest: input.patch_geometry_digest.clone(),
        coordinate_frame_digest: input.coordinate_frame_digest.clone(),
        rig_calibration_digest: input.rig_calibration_digest.clone(),
        observation_profile_digest: input.observation_profile_digest.clone(),
        calibration_parameter_digest: input.calibration_parameter_digest.clone(),
        held_out_split_digest: input.held_out_split_digest.clone(),
        characteristic_length_m: input.characteristic_length_m,
        ellipsoid_shape_scales: input.ellipsoid_shape_scales,
        observation_count: input.observations.len(),
        unique_trial_group_count: trial_groups.len(),
        sliding_observation_count: sliding_count,
        zero_slip_observation_count: zero_slip_count,
        non_dissipative_observation_count: non_dissipative_count,
        maximum_positive_observed_power: if non_dissipative_count > 0 {
            Some(maximum_positive_power)
        } else {
            None
        },
        maximum_normalized_wrench_error: if sliding_count > 0 {
            Some(maximum_error)
        } else {
            None
        },
        weighted_rms_normalized_wrench_error: weighted_rms,
        slip_power_tolerance: config.slip_power_tolerance,
        positive_power_tolerance: config.positive_power_tolerance,
        maximum_weighted_rms_wrench_error: config.maximum_weighted_rms_wrench_error,
        status,
        diagnostics,
    })
}

fn axis_from_index(index: usize) -> PatchAxis {
    match index {
        0 => PatchAxis::TangentialU,
        1 => PatchAxis::TangentialV,
        _ => PatchAxis::TorsionalNormal,
    }
}

fn stable_norm(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |norm, value| norm.hypot(*value))
}

fn dot(left: &[f64], right: &[f64]) -> Result<f64, CalibrationError> {
    let mut sum = 0.0_f64;
    for (left_value, right_value) in left.iter().zip(right) {
        sum += left_value * right_value;
        if !sum.is_finite() {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "weighted calibration dot product is non-finite",
            ));
        }
    }
    Ok(sum)
}

fn leave_one_axis_out_residual(
    target: &[f64],
    other_columns: &[&[f64]],
    relative_tolerance: f64,
) -> Result<f64, CalibrationError> {
    let reference_scale = other_columns
        .iter()
        .map(|column| stable_norm(column))
        .fold(0.0_f64, f64::max);
    let threshold = relative_tolerance * reference_scale;
    let mut basis = Vec::<Vec<f64>>::new();

    for column in other_columns {
        let mut residual = column.to_vec();
        for _ in 0..2 {
            for direction in &basis {
                let coefficient = dot(direction, &residual)?;
                for (value, basis_value) in residual.iter_mut().zip(direction) {
                    *value -= coefficient * basis_value;
                }
            }
        }
        let residual_norm = stable_norm(&residual);
        if !residual_norm.is_finite() {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "leave-one-axis-out basis residual is non-finite",
            ));
        }
        if residual_norm > threshold {
            for value in &mut residual {
                *value /= residual_norm;
            }
            basis.push(residual);
        }
    }

    let mut residual = target.to_vec();
    for _ in 0..2 {
        for direction in &basis {
            let coefficient = dot(direction, &residual)?;
            for (value, basis_value) in residual.iter_mut().zip(direction) {
                *value -= coefficient * basis_value;
            }
        }
    }
    let residual_norm = stable_norm(&residual);
    if !residual_norm.is_finite() {
        return Err(CalibrationError::new(
            CalibrationErrorKind::NonFiniteEvaluation,
            "leave-one-axis-out excitation residual is non-finite",
        ));
    }
    Ok(residual_norm)
}

fn finite_wrench(wrench: MeasuredPatchWrench) -> bool {
    wrench.tangent_u_force.is_finite()
        && wrench.tangent_v_force.is_finite()
        && wrench.normal_torsional_moment.is_finite()
}

fn validate(
    input: &PatchCalibrationInput,
    config: CalibrationConfig,
) -> Result<(), CalibrationError> {
    for (label, digest) in [
        ("specimen", &input.specimen_digest),
        ("patch geometry", &input.patch_geometry_digest),
        ("coordinate frame", &input.coordinate_frame_digest),
        ("rig calibration", &input.rig_calibration_digest),
        ("observation profile", &input.observation_profile_digest),
    ] {
        if digest.trim().is_empty() {
            return Err(CalibrationError::new(
                CalibrationErrorKind::InvalidIdentity,
                format!("{label} digest must be non-empty"),
            ));
        }
    }
    if !input.characteristic_length_m.is_finite() || input.characteristic_length_m <= 0.0 {
        return Err(CalibrationError::new(
            CalibrationErrorKind::InvalidParameter,
            "characteristic patch length must be finite and positive",
        ));
    }
    if input.observations.is_empty() {
        return Err(CalibrationError::new(
            CalibrationErrorKind::EmptyObservations,
            "at least one boundary observation is required",
        ));
    }
    if !config.rank_relative_tolerance.is_finite()
        || config.rank_relative_tolerance <= 0.0
        || config.rank_relative_tolerance >= 1.0
        || !config.maximum_boundary_residual.is_finite()
        || config.maximum_boundary_residual <= 0.0
    {
        return Err(CalibrationError::new(
            CalibrationErrorKind::InvalidTolerance,
            "rank tolerance must be in (0,1); boundary residual tolerance must be positive",
        ));
    }

    let mut observation_ids = BTreeSet::new();
    for observation in &input.observations {
        if observation.observation_id.trim().is_empty()
            || observation.trial_group_id.trim().is_empty()
        {
            return Err(CalibrationError::new(
                CalibrationErrorKind::InvalidIdentity,
                "observation and trial-group IDs must be non-empty",
            ));
        }
        if !observation_ids.insert(observation.observation_id.as_str()) {
            return Err(CalibrationError::new(
                CalibrationErrorKind::DuplicateObservationId,
                "observation IDs must be unique",
            ));
        }
        if !finite_wrench(observation.measured_wrench) {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteInput,
                "measured wrench components must be finite",
            ));
        }
        if !observation.capacity_scale.is_finite()
            || observation.capacity_scale <= 0.0
            || !observation.weight.is_finite()
            || observation.weight <= 0.0
        {
            return Err(CalibrationError::new(
                CalibrationErrorKind::InvalidParameter,
                "capacity scale and observation weight must be finite and positive",
            ));
        }
    }
    Ok(())
}

/// Solve a weighted least-squares system with exactly three unknown columns.
///
/// The returned solution is in original column order. Pivoted twice-reorthogonalized
/// MGS diagnoses rank before back-substitution; no normal equations are formed.
#[derive(Clone, Debug)]
struct LeastSquaresResult {
    rank: usize,
    pivot_order: Vec<usize>,
    pivot_norms: Vec<f64>,
    basis: Vec<Vec<f64>>,
    coefficients: Option<[f64; 3]>,
}

fn pivoted_weighted_least_squares(
    columns: &[Vec<f64>; 3],
    target: &[f64],
    rank_relative_tolerance: f64,
) -> Result<LeastSquaresResult, CalibrationError> {
    let rows = target.len();
    if rows == 0 || columns.iter().any(|column| column.len() != rows) {
        return Err(CalibrationError::new(
            CalibrationErrorKind::EmptyObservations,
            "weighted calibration design must have matching non-empty columns",
        ));
    }

    let column_norms = std::array::from_fn::<_, 3, _>(|index| stable_norm(&columns[index]));
    if !column_norms.iter().all(|value| value.is_finite()) {
        return Err(CalibrationError::new(
            CalibrationErrorKind::NonFiniteEvaluation,
            "weighted design-column norm is non-finite",
        ));
    }
    let first_pivot_norm = column_norms.iter().copied().fold(0.0_f64, f64::max);
    let rank_threshold = rank_relative_tolerance * first_pivot_norm;
    if !rank_threshold.is_finite() {
        return Err(CalibrationError::new(
            CalibrationErrorKind::NonFiniteEvaluation,
            "calibration rank threshold is non-finite",
        ));
    }

    let mut selected = [false; 3];
    let mut selected_order = Vec::new();
    let mut basis: Vec<Vec<f64>> = Vec::new();
    let mut pivot_norms = Vec::new();
    let mut r = vec![vec![0.0_f64; 3]; 3];

    for step in 0..3 {
        let mut best_index = None;
        let mut best_residual = Vec::new();
        let mut best_norm = 0.0_f64;
        let mut residual_by_index: [Option<Vec<f64>>; 3] = [None, None, None];

        for index in 0..3 {
            if selected[index] {
                continue;
            }
            let mut residual = columns[index].clone();
            for row in r.iter_mut().take(basis.len()) {
                row[index] = 0.0;
            }
            for _ in 0..2 {
                for (basis_index, direction) in basis.iter().enumerate() {
                    let coefficient = dot(direction, &residual)?;
                    r[basis_index][index] += coefficient;
                    for (value, basis_value) in residual.iter_mut().zip(direction) {
                        *value -= coefficient * basis_value;
                        if !value.is_finite() {
                            return Err(CalibrationError::new(
                                CalibrationErrorKind::NonFiniteEvaluation,
                                "calibration orthogonal residual is non-finite",
                            ));
                        }
                    }
                }
            }
            let residual_norm = stable_norm(&residual);
            if !residual_norm.is_finite() {
                return Err(CalibrationError::new(
                    CalibrationErrorKind::NonFiniteEvaluation,
                    "calibration residual norm is non-finite",
                ));
            }
            residual_by_index[index] = Some(residual);
            let replace = match best_index {
                None => true,
                Some(current) => {
                    residual_norm > best_norm || (residual_norm == best_norm && index < current)
                }
            };
            if replace {
                best_index = Some(index);
                best_norm = residual_norm;
                best_residual = residual_by_index[index].clone().unwrap_or_default();
            }
        }

        let Some(index) = best_index else {
            break;
        };
        if best_norm <= rank_threshold {
            break;
        }
        for value in &mut best_residual {
            *value /= best_norm;
        }
        if !best_residual.iter().all(|value| value.is_finite()) {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "calibration basis normalization is non-finite",
            ));
        }

        r[step][index] = best_norm;
        selected[index] = true;
        selected_order.push(index);
        pivot_norms.push(best_norm);
        basis.push(best_residual);
    }

    let rank = basis.len();
    if rank < 3 {
        return Ok(LeastSquaresResult {
            rank,
            pivot_order: selected_order,
            pivot_norms,
            basis,
            coefficients: None,
        });
    }

    let mut triangular = vec![vec![0.0_f64; 3]; 3];
    for row in 0..3 {
        for column in row..3 {
            triangular[row][column] = r[row][selected_order[column]];
        }
    }
    let mut rhs = Vec::with_capacity(3);
    for direction in &basis {
        rhs.push(dot(direction, target)?);
    }

    let mut permuted_solution = [0.0_f64; 3];
    for row in (0..3).rev() {
        let mut value = rhs[row];
        for column in (row + 1)..3 {
            value -= triangular[row][column] * permuted_solution[column];
        }
        let pivot = triangular[row][row];
        if !pivot.is_finite() || pivot.abs() <= rank_threshold {
            return Err(CalibrationError::new(
                CalibrationErrorKind::LinearSolveFailure,
                "calibration QR triangular pivot is not safely invertible",
            ));
        }
        permuted_solution[row] = value / pivot;
        if !permuted_solution[row].is_finite() {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "calibration shape coefficient is non-finite",
            ));
        }
    }

    let mut solution = [0.0_f64; 3];
    for index in 0..3 {
        solution[selected_order[index]] = permuted_solution[index];
    }
    Ok(LeastSquaresResult {
        rank,
        pivot_order: selected_order,
        pivot_norms,
        basis,
        coefficients: Some(solution),
    })
}

pub fn calibrate_planar_patch(
    input: &PatchCalibrationInput,
    config: CalibrationConfig,
) -> Result<PatchCalibrationReport, CalibrationError> {
    validate(input, config)?;

    let mut columns = [
        Vec::with_capacity(input.observations.len()),
        Vec::with_capacity(input.observations.len()),
        Vec::with_capacity(input.observations.len()),
    ];
    let mut target = Vec::with_capacity(input.observations.len());
    let mut total_weight = 0.0_f64;
    let mut trial_groups = BTreeSet::new();

    for observation in &input.observations {
        trial_groups.insert(observation.trial_group_id.as_str());
        let scale = observation.capacity_scale;
        let wrench = observation.measured_wrench;
        let normalized = [
            wrench.tangent_u_force / scale,
            wrench.tangent_v_force / scale,
            wrench.normal_torsional_moment / scale / input.characteristic_length_m,
        ];
        if !normalized.iter().all(|value| value.is_finite()) {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "capacity-normalized wrench is non-finite",
            ));
        }
        let root_weight = observation.weight.sqrt();
        let weighted = [
            normalized[0] * normalized[0] * root_weight,
            normalized[1] * normalized[1] * root_weight,
            normalized[2] * normalized[2] * root_weight,
        ];
        if !weighted.iter().all(|value| value.is_finite()) {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "weighted boundary design value is non-finite",
            ));
        }
        for axis in 0..3 {
            columns[axis].push(weighted[axis]);
        }
        target.push(root_weight);
        total_weight += observation.weight;
        if !total_weight.is_finite() {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "total calibration weight overflowed",
            ));
        }
    }

    let LeastSquaresResult {
        rank,
        pivot_order,
        pivot_norms,
        coefficients,
        ..
    } = pivoted_weighted_least_squares(&columns, &target, config.rank_relative_tolerance)?;

    let mut axis_excitation = Vec::with_capacity(3);
    for (axis, column) in columns.iter().enumerate() {
        let other_columns = columns
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                if index == axis {
                    None
                } else {
                    Some(candidate.as_slice())
                }
            })
            .collect::<Vec<_>>();
        let norm = stable_norm(column);
        let residual = leave_one_axis_out_residual(
            column,
            &other_columns,
            config.rank_relative_tolerance,
        )?;
        axis_excitation.push(AxisExcitation {
            axis: axis_from_index(axis),
            weighted_column_norm: norm,
            // Independent excitation after projecting onto the other two channels.
            final_projection_residual: residual,
            relative_projection_residual: if norm > 0.0 {
                residual / norm
            } else {
                0.0
            },
        });
    }

    let Some(beta) = coefficients else {
        return Ok(PatchCalibrationReport {
            schema_version: SCHEMA_VERSION.to_owned(),
            specimen_digest: input.specimen_digest.clone(),
            patch_geometry_digest: input.patch_geometry_digest.clone(),
            coordinate_frame_digest: input.coordinate_frame_digest.clone(),
            rig_calibration_digest: input.rig_calibration_digest.clone(),
            observation_profile_digest: input.observation_profile_digest.clone(),
            characteristic_length_m: input.characteristic_length_m,
            observation_count: input.observations.len(),
            unique_trial_group_count: trial_groups.len(),
            design_rank: rank,
            rank_threshold: config.rank_relative_tolerance
                * columns
                    .iter()
                    .map(|column| stable_norm(column))
                    .fold(0.0_f64, f64::max),
            pivot_order: pivot_order
                .iter()
                .map(|index| axis_from_index(*index))
                .collect(),
            pivot_residual_norms: pivot_norms,
            axis_excitation,
            status: CalibrationStatus::Unidentifiable,
            inverse_squared_dimensionless_shape_scales: None,
            ellipsoid_shape_scales: None,
            maximum_absolute_boundary_residual: None,
            weighted_rms_boundary_residual: None,
            rank_relative_tolerance: config.rank_relative_tolerance,
            maximum_boundary_residual_tolerance: config.maximum_boundary_residual,
            physical_model_validated: false,
        });
    };

    if beta.iter().any(|value| !value.is_finite() || *value <= 0.0) {
        return Err(CalibrationError::new(
            CalibrationErrorKind::NonPositiveScaleEstimate,
            "boundary fit produced a non-positive inverse-squared shape scale",
        ));
    }

    let dimensionless_shape_scales = beta.map(|value| 1.0 / value.sqrt());
    let shape_scales = [
        dimensionless_shape_scales[0],
        dimensionless_shape_scales[1],
        dimensionless_shape_scales[2] * input.characteristic_length_m,
    ];
    if !shape_scales
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
    {
        return Err(CalibrationError::new(
            CalibrationErrorKind::NonFiniteEvaluation,
            "calibrated ellipsoid shape scale is non-finite or non-positive",
        ));
    }

    let mut max_residual = 0.0_f64;
    let mut weighted_residual_sum = 0.0_f64;
    for observation in &input.observations {
        let scale = observation.capacity_scale;
        let wrench = observation.measured_wrench;
        let normalized = [
            wrench.tangent_u_force / scale,
            wrench.tangent_v_force / scale,
            wrench.normal_torsional_moment / scale / input.characteristic_length_m,
        ];
        let predicted = beta[0] * normalized[0] * normalized[0]
            + beta[1] * normalized[1] * normalized[1]
            + beta[2] * normalized[2] * normalized[2];
        let residual = predicted - 1.0;
        if !residual.is_finite() {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "boundary calibration residual is non-finite",
            ));
        }
        max_residual = max_residual.max(residual.abs());
        weighted_residual_sum += observation.weight * residual * residual;
        if !weighted_residual_sum.is_finite() {
            return Err(CalibrationError::new(
                CalibrationErrorKind::NonFiniteEvaluation,
                "weighted residual sum is non-finite",
            ));
        }
    }
    let weighted_rms = (weighted_residual_sum / total_weight).sqrt();
    if !weighted_rms.is_finite() {
        return Err(CalibrationError::new(
            CalibrationErrorKind::NonFiniteEvaluation,
            "weighted RMS residual is non-finite",
        ));
    }

    Ok(PatchCalibrationReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        specimen_digest: input.specimen_digest.clone(),
        patch_geometry_digest: input.patch_geometry_digest.clone(),
        coordinate_frame_digest: input.coordinate_frame_digest.clone(),
        rig_calibration_digest: input.rig_calibration_digest.clone(),
        observation_profile_digest: input.observation_profile_digest.clone(),
        characteristic_length_m: input.characteristic_length_m,
        observation_count: input.observations.len(),
        unique_trial_group_count: trial_groups.len(),
        design_rank: rank,
        rank_threshold: config.rank_relative_tolerance
            * columns
                .iter()
                .map(|column| stable_norm(column))
                .fold(0.0_f64, f64::max),
        pivot_order: pivot_order
            .iter()
            .map(|index| axis_from_index(*index))
            .collect(),
        pivot_residual_norms: pivot_norms,
        axis_excitation,
        status: if max_residual <= config.maximum_boundary_residual {
            CalibrationStatus::IdentifiedWithinResidualTolerance
        } else {
            CalibrationStatus::IdentifiedPoorBoundaryFit
        },
        inverse_squared_dimensionless_shape_scales: Some(beta),
        ellipsoid_shape_scales: Some(shape_scales),
        maximum_absolute_boundary_residual: Some(max_residual),
        weighted_rms_boundary_residual: Some(weighted_rms),
        rank_relative_tolerance: config.rank_relative_tolerance,
        maximum_boundary_residual_tolerance: config.maximum_boundary_residual,
        physical_model_validated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        id: &str,
        group: &str,
        wrench: [f64; 3],
        capacity: f64,
        weight: f64,
    ) -> BoundaryObservation {
        BoundaryObservation {
            observation_id: id.to_owned(),
            trial_group_id: group.to_owned(),
            measured_wrench: MeasuredPatchWrench {
                tangent_u_force: wrench[0],
                tangent_v_force: wrench[1],
                normal_torsional_moment: wrench[2],
            },
            capacity_scale: capacity,
            weight,
        }
    }

    fn input(observations: Vec<BoundaryObservation>) -> PatchCalibrationInput {
        PatchCalibrationInput {
            specimen_digest: "specimen-001".to_owned(),
            patch_geometry_digest: "patch-geometry-001".to_owned(),
            coordinate_frame_digest: "patch-frame-001".to_owned(),
            rig_calibration_digest: "rig-calibration-001".to_owned(),
            observation_profile_digest: "boundary-sliding-profile-001".to_owned(),
            characteristic_length_m: 1.0,
            observations,
        }
    }

    fn direction_observation(
        id: &str,
        group: &str,
        slip: [f64; 3],
        wrench: [f64; 3],
        capacity: f64,
        weight: f64,
    ) -> MaximumDissipationObservation {
        MaximumDissipationObservation {
            observation_id: id.to_owned(),
            trial_group_id: group.to_owned(),
            slip_twist: slip,
            measured_wrench: MeasuredPatchWrench {
                tangent_u_force: wrench[0],
                tangent_v_force: wrench[1],
                normal_torsional_moment: wrench[2],
            },
            capacity_scale: capacity,
            weight,
        }
    }

    fn direction_input(
        observations: Vec<MaximumDissipationObservation>,
    ) -> MaximumDissipationValidationInput {
        MaximumDissipationValidationInput {
            specimen_digest: "specimen-001".to_owned(),
            patch_geometry_digest: "patch-geometry-001".to_owned(),
            coordinate_frame_digest: "patch-frame-001".to_owned(),
            rig_calibration_digest: "rig-calibration-001".to_owned(),
            observation_profile_digest: "held-out-sliding-profile-001".to_owned(),
            calibration_parameter_digest: "calibration-parameters-001".to_owned(),
            held_out_split_digest: "held-out-trials-001".to_owned(),
            characteristic_length_m: 0.5,
            ellipsoid_shape_scales: [2.0, 0.5, 0.25],
            observations,
        }
    }

    fn direction_config() -> MaximumDissipationValidationConfig {
        MaximumDissipationValidationConfig::default()
    }

    fn config() -> CalibrationConfig {
        CalibrationConfig::default()
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!((left - right).abs() <= tolerance, "{left} != {right}");
    }

    #[test]
    fn three_independent_axis_trials_recover_known_shape_scales() {
        let problem = input(vec![
            observation("u", "u-trial", [2.0, 0.0, 0.0], 1.0, 1.0),
            observation("v", "v-trial", [0.0, 0.5, 0.0], 1.0, 1.0),
            observation("torsion", "spin-trial", [0.0, 0.0, 0.25], 1.0, 1.0),
        ]);
        let report = calibrate_planar_patch(&problem, config()).expect("calibration");
        assert_eq!(
            report.status,
            CalibrationStatus::IdentifiedWithinResidualTolerance
        );
        assert_eq!(report.design_rank, 3);
        let scales = report.ellipsoid_shape_scales.expect("identified scales");
        close(scales[0], 2.0, 1.0e-12);
        close(scales[1], 0.5, 1.0e-12);
        close(scales[2], 0.25, 1.0e-12);
        assert!(!report.physical_model_validated);
    }

    #[test]
    fn capacity_scale_is_removed_before_fitting_shape() {
        let problem = input(vec![
            observation("u-low-load", "u-trial", [1.0, 0.0, 0.0], 0.5, 1.0),
            observation("u-high-load", "u-trial", [4.0, 0.0, 0.0], 2.0, 1.0),
            observation("v", "v-trial", [0.0, 1.5, 0.0], 3.0, 1.0),
            observation("spin", "spin-trial", [0.0, 0.0, 0.2], 0.8, 1.0),
        ]);
        let report = calibrate_planar_patch(&problem, config()).expect("calibration");
        let scales = report.ellipsoid_shape_scales.expect("identified scales");
        close(scales[0], 2.0, 1.0e-12);
        close(scales[1], 0.5, 1.0e-12);
        close(scales[2], 0.25, 1.0e-12);
    }

    #[test]
    fn torsional_shape_scale_is_invariant_to_characteristic_length_choice() {
        let observations = vec![
            observation("u", "u", [2.0, 0.0, 0.0], 1.0, 1.0),
            observation("v", "v", [0.0, 0.5, 0.0], 1.0, 1.0),
            observation("spin", "spin", [0.0, 0.0, 0.25], 1.0, 1.0),
        ];
        let mut short_length = input(observations.clone());
        short_length.characteristic_length_m = 0.5;
        let mut long_length = input(observations);
        long_length.characteristic_length_m = 2.0;
        let short_report = calibrate_planar_patch(&short_length, config()).expect("short scale");
        let long_report = calibrate_planar_patch(&long_length, config()).expect("long scale");
        let short_axes = short_report.ellipsoid_shape_scales.expect("short axes");
        let long_axes = long_report.ellipsoid_shape_scales.expect("long axes");
        close(short_axes[0], long_axes[0], 1.0e-12);
        close(short_axes[1], long_axes[1], 1.0e-12);
        close(short_axes[2], 0.25, 1.0e-12);
        close(long_axes[2], 0.25, 1.0e-12);
    }

    #[test]
    fn invalid_characteristic_length_is_rejected() {
        let mut problem = input(vec![observation("u", "u", [2.0, 0.0, 0.0], 1.0, 1.0)]);
        problem.characteristic_length_m = f64::NAN;
        assert_eq!(
            calibrate_planar_patch(&problem, config())
                .expect_err("non-finite characteristic length")
                .kind,
            CalibrationErrorKind::InvalidParameter
        );
        problem.characteristic_length_m = 0.0;
        assert_eq!(
            calibrate_planar_patch(&problem, config())
                .expect_err("zero characteristic length")
                .kind,
            CalibrationErrorKind::InvalidParameter
        );
    }

    #[test]
    fn independent_axis_excitation_residuals_are_one_for_orthogonal_trials() {
        let problem = input(vec![
            observation("u", "u", [2.0, 0.0, 0.0], 1.0, 1.0),
            observation("v", "v", [0.0, 0.5, 0.0], 1.0, 1.0),
            observation("spin", "spin", [0.0, 0.0, 0.25], 1.0, 1.0),
        ]);
        let report = calibrate_planar_patch(&problem, config()).expect("orthogonal axes");
        assert_eq!(report.design_rank, 3);
        for axis in &report.axis_excitation {
            close(axis.relative_projection_residual, 1.0, 1.0e-12);
        }
    }

    #[test]
    fn pure_tangential_u_trials_are_unidentifiable() {
        let problem = input(vec![
            observation("u1", "trial-a", [2.0, 0.0, 0.0], 1.0, 1.0),
            observation("u2", "trial-b", [2.0, 0.0, 0.0], 1.0, 1.0),
        ]);
        let report = calibrate_planar_patch(&problem, config()).expect("rank report");
        assert_eq!(report.status, CalibrationStatus::Unidentifiable);
        assert_eq!(report.design_rank, 1);
        assert!(report.ellipsoid_shape_scales.is_none());
        assert!(report.inverse_squared_dimensionless_shape_scales.is_none());
    }

    #[test]
    fn repeated_identical_groups_are_reported_as_one_trial_group() {
        let problem = input(vec![
            observation("u-a", "same-trial", [2.0, 0.0, 0.0], 1.0, 1.0),
            observation("u-b", "same-trial", [2.0, 0.0, 0.0], 1.0, 1.0),
            observation("v", "v-trial", [0.0, 0.5, 0.0], 1.0, 1.0),
            observation("spin", "spin-trial", [0.0, 0.0, 0.25], 1.0, 1.0),
        ]);
        let report = calibrate_planar_patch(&problem, config()).expect("calibration");
        assert_eq!(report.observation_count, 4);
        assert_eq!(report.unique_trial_group_count, 3);
    }

    #[test]
    fn weighted_fit_reports_poor_boundary_model_fit() {
        let problem = input(vec![
            observation("u-1", "u", [2.0, 0.0, 0.0], 1.0, 1.0),
            observation("u-2", "u", [2.2, 0.0, 0.0], 1.0, 4.0),
            observation("v", "v", [0.0, 0.5, 0.0], 1.0, 1.0),
            observation("spin", "spin", [0.0, 0.0, 0.25], 1.0, 1.0),
        ]);
        let report = calibrate_planar_patch(&problem, config()).expect("weighted least squares");
        assert_eq!(report.design_rank, 3);
        assert_eq!(report.status, CalibrationStatus::IdentifiedPoorBoundaryFit);
        assert!(report.maximum_absolute_boundary_residual.unwrap() > 0.0);
    }

    #[test]
    fn observation_order_does_not_change_estimated_axes() {
        let observations = vec![
            observation("u", "u", [2.0, 0.0, 0.0], 1.0, 1.0),
            observation("v", "v", [0.0, 0.5, 0.0], 1.0, 1.0),
            observation("spin", "spin", [0.0, 0.0, 0.25], 1.0, 1.0),
            observation(
                "mixed",
                "mixed",
                [1.1547005383792515, 0.2886751345948129, 0.14433756729740646],
                1.0,
                1.0,
            ),
        ];
        let first = calibrate_planar_patch(&input(observations.clone()), config()).expect("first");
        let second =
            calibrate_planar_patch(&input(observations.into_iter().rev().collect()), config())
                .expect("permuted");
        let a = first.ellipsoid_shape_scales.expect("first scales");
        let b = second.ellipsoid_shape_scales.expect("second scales");
        close(a[0], 2.0, 1.0e-9);
        close(a[1], 0.5, 1.0e-9);
        close(a[2], 0.25, 1.0e-9);
        for index in 0..3 {
            close(a[index], b[index], 1.0e-10);
        }
    }

    #[test]
    fn maximum_dissipation_matches_mixed_slide_and_is_velocity_scale_invariant() {
        let inverse_root_six = 1.0 / 6.0_f64.sqrt();
        let predicted = [
            -4.0 * inverse_root_six,
            -0.5 * inverse_root_six,
            -0.25 * inverse_root_six,
        ];
        let problem = direction_input(vec![
            direction_observation("mixed", "mixed-trial", [1.0, 2.0, 4.0], predicted, 1.0, 1.0),
            direction_observation(
                "mixed-fast",
                "mixed-fast-trial",
                [5.0, 10.0, 20.0],
                predicted,
                1.0,
                2.0,
            ),
            direction_observation(
                "pure-u",
                "u-trial",
                [1.0, 0.0, 0.0],
                [-2.0, 0.0, 0.0],
                1.0,
                1.0,
            ),
            direction_observation(
                "pure-v",
                "v-trial",
                [0.0, 1.0, 0.0],
                [0.0, -0.5, 0.0],
                1.0,
                1.0,
            ),
            direction_observation(
                "pure-spin",
                "spin-trial",
                [0.0, 0.0, 1.0],
                [0.0, 0.0, -0.25],
                1.0,
                1.0,
            ),
        ]);
        let report =
            validate_maximum_dissipation(&problem, direction_config()).expect("direction fit");
        assert_eq!(
            report.status,
            MaximumDissipationValidationStatus::ConsistentWithinTolerance
        );
        assert_eq!(report.sliding_observation_count, 5);
        assert_eq!(report.zero_slip_observation_count, 0);
        assert_eq!(report.unique_trial_group_count, 5);
        close(
            report.weighted_rms_normalized_wrench_error.unwrap(),
            0.0,
            1.0e-12,
        );
        close(
            report.maximum_normalized_wrench_error.unwrap(),
            0.0,
            1.0e-12,
        );
        assert_eq!(report.non_dissipative_observation_count, 0);
        close(
            report.diagnostics[0].predicted_power.unwrap(),
            -6.0_f64.sqrt(),
            1.0e-12,
        );
    }

    #[test]
    fn zero_slip_sticking_samples_are_preserved_but_not_directionally_scored() {
        let problem = direction_input(vec![
            direction_observation("stick", "stick-trial", [0.0; 3], [0.2, 0.1, 0.05], 1.0, 1.0),
            direction_observation(
                "slide",
                "slide-trial",
                [1.0, 0.0, 0.0],
                [-2.0, 0.0, 0.0],
                1.0,
                1.0,
            ),
        ]);
        let report =
            validate_maximum_dissipation(&problem, direction_config()).expect("sticking/sliding");
        assert_eq!(report.sliding_observation_count, 1);
        assert_eq!(report.zero_slip_observation_count, 1);
        assert_eq!(
            report.diagnostics[0].disposition,
            DirectionObservationDisposition::ZeroSlipStickingSetNotDirectional
        );
        assert!(report.diagnostics[0].normalized_wrench_error.is_none());
        assert_eq!(
            report.status,
            MaximumDissipationValidationStatus::ConsistentWithinTolerance
        );
    }

    #[test]
    fn all_zero_slip_trials_return_no_informative_directional_observations() {
        let problem = direction_input(vec![
            direction_observation("stick-a", "trial-a", [0.0; 3], [1.0, 0.0, 0.0], 1.0, 1.0),
            direction_observation("stick-b", "trial-b", [0.0; 3], [0.0, 0.5, 0.0], 1.0, 1.0),
        ]);
        let report =
            validate_maximum_dissipation(&problem, direction_config()).expect("no sliding");
        assert_eq!(report.sliding_observation_count, 0);
        assert_eq!(report.zero_slip_observation_count, 2);
        assert_eq!(
            report.status,
            MaximumDissipationValidationStatus::NoInformativeSlidingObservations
        );
        assert!(report.weighted_rms_normalized_wrench_error.is_none());
    }

    #[test]
    fn wrong_direction_and_positive_measured_power_are_reported_as_inconsistent() {
        let problem = direction_input(vec![direction_observation(
            "wrong-way",
            "trial-a",
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            1.0,
            1.0,
        )]);
        let report =
            validate_maximum_dissipation(&problem, direction_config()).expect("wrong direction");
        assert_eq!(
            report.status,
            MaximumDissipationValidationStatus::Inconsistent
        );
        assert_eq!(report.non_dissipative_observation_count, 1);
        close(
            report.maximum_positive_observed_power.unwrap(),
            2.0,
            1.0e-12,
        );
        close(
            report.maximum_normalized_wrench_error.unwrap(),
            2.0,
            1.0e-12,
        );
    }

    #[test]
    fn maximum_dissipation_validation_rejects_duplicate_ids_and_missing_holdout_identity() {
        let duplicate = direction_input(vec![
            direction_observation(
                "same",
                "trial-a",
                [1.0, 0.0, 0.0],
                [-2.0, 0.0, 0.0],
                1.0,
                1.0,
            ),
            direction_observation(
                "same",
                "trial-b",
                [0.0, 1.0, 0.0],
                [0.0, -0.5, 0.0],
                1.0,
                1.0,
            ),
        ]);
        assert_eq!(
            validate_maximum_dissipation(&duplicate, direction_config())
                .expect_err("duplicate direction IDs")
                .kind,
            CalibrationErrorKind::DuplicateObservationId
        );

        let mut missing = direction_input(vec![direction_observation(
            "slide",
            "trial-a",
            [1.0, 0.0, 0.0],
            [-2.0, 0.0, 0.0],
            1.0,
            1.0,
        )]);
        missing.held_out_split_digest.clear();
        assert_eq!(
            validate_maximum_dissipation(&missing, direction_config())
                .expect_err("missing held-out split")
                .kind,
            CalibrationErrorKind::InvalidIdentity
        );
    }

    #[test]
    fn duplicate_ids_missing_digests_and_invalid_weights_are_rejected() {
        let duplicate = input(vec![
            observation("same", "u", [2.0, 0.0, 0.0], 1.0, 1.0),
            observation("same", "v", [0.0, 0.5, 0.0], 1.0, 1.0),
        ]);
        assert_eq!(
            calibrate_planar_patch(&duplicate, config())
                .expect_err("duplicate observation")
                .kind,
            CalibrationErrorKind::DuplicateObservationId
        );

        let mut missing = input(vec![observation("u", "u", [2.0, 0.0, 0.0], 1.0, 1.0)]);
        missing.rig_calibration_digest.clear();
        assert_eq!(
            calibrate_planar_patch(&missing, config())
                .expect_err("missing rig")
                .kind,
            CalibrationErrorKind::InvalidIdentity
        );

        let bad_weight = input(vec![observation("u", "u", [2.0, 0.0, 0.0], 1.0, 0.0)]);
        assert_eq!(
            calibrate_planar_patch(&bad_weight, config())
                .expect_err("zero weight")
                .kind,
            CalibrationErrorKind::InvalidParameter
        );
    }

    #[test]
    fn nonfinite_wrench_and_invalid_tolerances_are_rejected() {
        let bad = input(vec![observation("u", "u", [f64::NAN, 0.0, 0.0], 1.0, 1.0)]);
        assert_eq!(
            calibrate_planar_patch(&bad, config())
                .expect_err("nonfinite wrench")
                .kind,
            CalibrationErrorKind::NonFiniteInput
        );

        let invalid_config = CalibrationConfig {
            rank_relative_tolerance: 0.0,
            ..config()
        };
        assert_eq!(
            calibrate_planar_patch(
                &input(vec![observation("u", "u", [2.0, 0.0, 0.0], 1.0, 1.0)]),
                invalid_config,
            )
            .expect_err("invalid tolerance")
            .kind,
            CalibrationErrorKind::InvalidTolerance
        );
    }
}
