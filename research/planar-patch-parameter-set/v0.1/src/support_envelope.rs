// SPDX-License-Identifier: AGPL-3.0-or-later
//! Correlation-preserving support bounds for a bounded planar-patch
//! friction-ellipsoid parameter set.
//!
//! Let beta_i = 1/e_i^2 and q = [v_u, v_v, L * omega_n]. For capacity
//! c = mu*N, support in physical twist direction [v_u, v_v, omega_n] is
//! h(beta) = c * sqrt(sum_i(q_i^2 / beta_i)).
//!
//! The upper envelope is maximized over joint feasible vertices, not a
//! Cartesian product of marginal axis intervals. The lower envelope uses
//! Cauchy-Schwarz and the maximum joint sum of active beta coordinates.
//! This is a conservative lower bound, not an exact nonlinear minimizer.

use super::{
    AxisIdentifiability, ParameterInterval, ParameterSetError, ParameterSetErrorKind,
    ParameterSetReport, ParameterSetStatus, SCHEMA_VERSION,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportEnvelopeStatus {
    ZeroTwist,
    Bounded,
    UnboundedAbove,
    UnboundedForAllFeasibleParameters,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportLowerBoundMethod {
    JointVertexCauchySchwarz,
    UnboundedActiveBetaCoordinate,
    ZeroTwist,
    NoFiniteSupportLowerBound,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportEnvelopeConfig {
    pub report_consistency_tolerance: f64,
}

impl Default for SupportEnvelopeConfig {
    fn default() -> Self {
        Self {
            report_consistency_tolerance: 1.0e-9,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatchSupportEnvelopeInput<'a> {
    pub parameter_set: &'a ParameterSetReport,
    /// Independently supplied positive [mu*N lower, mu*N upper] interval, in newtons.
    /// This v0.1 treats it as independent of the normalized shape-parameter set.
    pub capacity_scale_interval: [f64; 2],
    /// Receipt/digest for the independently measured or declared capacity interval.
    pub capacity_scale_digest: String,
    /// [tangent-u velocity (m/s), tangent-v velocity (m/s), normal angular velocity (rad/s)].
    pub slip_twist: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatchSupportEnvelopeReport {
    pub schema_version: String,
    pub parameter_set_schema_version: String,
    pub specimen_digest: String,
    pub patch_geometry_digest: String,
    pub coordinate_frame_digest: String,
    pub rig_calibration_digest: String,
    pub observation_profile_digest: String,
    pub capacity_convention_digest: String,
    pub characteristic_length_m: f64,
    pub capacity_scale_interval: [f64; 2],
    pub capacity_scale_digest: String,
    pub slip_twist: [f64; 3],
    /// q = [v_u, v_v, L*omega_n], used by the dimensionless ellipsoid.
    pub nondimensional_slip_direction: [f64; 3],
    pub active_axis_count: usize,
    pub feasible_vertex_count: usize,
    pub status: SupportEnvelopeStatus,
    /// Cauchy-Schwarz lower value over the reported vertex representation.
    /// It is not an outward-rounded numerical certificate in v0.1.
    pub support_lower_bound: Option<f64>,
    /// Maximum support evaluated at the reported vertices. None represents
    /// an unbounded witness in the reported representation.
    pub support_upper_bound: Option<f64>,
    pub lower_bound_method: SupportLowerBoundMethod,
    /// Conservative except for the zero-twist case.
    pub lower_bound_is_exact: bool,
    /// The returned value is the maximum evaluated over the supplied vertices.
    /// It is an exact polyhedral supremum only if those vertices are complete
    /// and exact; v0.1 does not establish that with outward-rounded arithmetic.
    pub upper_bound_is_vertex_maximum: bool,
    /// False in v0.1: floating-point intersection/support evaluation is not
    /// enclosed by outward-rounded interval arithmetic.
    pub outward_rounding_applied: bool,
    pub maximum_support_vertex: Option<[f64; 3]>,
    pub unbounded_support_vertex: Option<[f64; 3]>,
    pub maximum_active_beta_sum: Option<f64>,
    pub parameter_set_enumeration_complete: bool,
}

fn invalid(kind: ParameterSetErrorKind, reason: impl Into<String>) -> ParameterSetError {
    ParameterSetError {
        kind,
        reason: reason.into(),
    }
}

fn near(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance * (1.0 + left.abs().max(right.abs()))
}

fn validate_parameter_set(
    report: &ParameterSetReport,
    tolerance: f64,
) -> Result<(), ParameterSetError> {
    if report.schema_version != SCHEMA_VERSION {
        return Err(invalid(
            ParameterSetErrorKind::InvalidIdentity,
            "unsupported parameter-set schema version",
        ));
    }
    for (label, digest) in [
        ("specimen", &report.specimen_digest),
        ("patch geometry", &report.patch_geometry_digest),
        ("coordinate frame", &report.coordinate_frame_digest),
        ("rig calibration", &report.rig_calibration_digest),
        ("observation profile", &report.observation_profile_digest),
        ("capacity convention", &report.capacity_convention_digest),
    ] {
        if digest.trim().is_empty() {
            return Err(invalid(
                ParameterSetErrorKind::InvalidIdentity,
                format!("{label} digest must be non-empty"),
            ));
        }
    }
    if !report.characteristic_length_m.is_finite() || report.characteristic_length_m <= 0.0 {
        return Err(invalid(
            ParameterSetErrorKind::InvalidInput,
            "characteristic length must be finite and positive",
        ));
    }
    if report.status == ParameterSetStatus::InconclusiveOrEmpty
        || !report.enumeration_complete
        || report.feasible_vertices.is_empty()
        || report.feasible_vertex_count != report.feasible_vertices.len()
        || report.plane_triples_checked == 0
    {
        return Err(invalid(
            ParameterSetErrorKind::InvalidInput,
            "support bounds require a non-empty, completely enumerated feasible parameter set",
        ));
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(invalid(
            ParameterSetErrorKind::InvalidTolerance,
            "report consistency tolerance must be finite and positive",
        ));
    }
    for vertex in &report.feasible_vertices {
        if !vertex
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
        {
            return Err(invalid(
                ParameterSetErrorKind::InvalidInput,
                "joint feasible vertices must be finite and non-negative",
            ));
        }
    }

    for axis in 0..3 {
        let interval: &ParameterInterval = &report.intervals[axis];
        let minimum = report
            .feasible_vertices
            .iter()
            .map(|vertex| vertex[axis])
            .fold(f64::INFINITY, f64::min);
        let maximum = report
            .feasible_vertices
            .iter()
            .map(|vertex| vertex[axis])
            .fold(0.0_f64, f64::max);
        let Some(lower) = interval.beta_lower else {
            return Err(invalid(
                ParameterSetErrorKind::InvalidInput,
                "each axis requires a vertex-derived beta lower bound",
            ));
        };
        if !lower.is_finite() || lower < 0.0 || !near(lower, minimum, tolerance) {
            return Err(invalid(
                ParameterSetErrorKind::InvalidInput,
                "beta lower bound disagrees with the joint vertices",
            ));
        }
        if let Some(upper) = interval.beta_upper {
            if !upper.is_finite() || upper < 0.0 || !near(upper, maximum, tolerance) {
                return Err(invalid(
                    ParameterSetErrorKind::InvalidInput,
                    "beta upper bound excludes a supplied joint vertex",
                ));
            }
        } else if report.status == ParameterSetStatus::FeasibleWithFiniteIntervals {
            return Err(invalid(
                ParameterSetErrorKind::InvalidInput,
                "finite parameter-set status cannot have an unbounded beta interval",
            ));
        }
        if interval.identifiability == AxisIdentifiability::NoFeasibleVertexForAxis {
            return Err(invalid(
                ParameterSetErrorKind::InvalidInput,
                "parameter report marks an axis without a feasible vertex",
            ));
        }
        if report.status == ParameterSetStatus::FeasibleWithFiniteIntervals
            && (interval.beta_upper.is_none() || interval.shape_scale_upper.is_none())
        {
            return Err(invalid(
                ParameterSetErrorKind::InvalidInput,
                "finite parameter-set status contains an unbounded axis",
            ));
        }
    }
    Ok(())
}

/// Bound support over the joint feasible beta polyhedron.
///
/// For an exact complete vertex representation, convexity and coordinatewise
/// monotonicity make the vertex maximum the polyhedral supremum. This v0.1
/// implementation uses floating-point intersections without outward rounding,
/// so the result is a maximum over reported vertices, not a formal certificate.
/// The lower value is the Cauchy-Schwarz construction over that representation.
pub fn compute_patch_support_envelope(
    input: &PatchSupportEnvelopeInput<'_>,
    config: SupportEnvelopeConfig,
) -> Result<PatchSupportEnvelopeReport, ParameterSetError> {
    let parameter_set = input.parameter_set;
    validate_parameter_set(parameter_set, config.report_consistency_tolerance)?;
    let [capacity_lower, capacity_upper] = input.capacity_scale_interval;
    if !capacity_lower.is_finite()
        || !capacity_upper.is_finite()
        || capacity_lower <= 0.0
        || capacity_upper < capacity_lower
    {
        return Err(invalid(
            ParameterSetErrorKind::InvalidInput,
            "capacity interval must be finite, positive and ordered",
        ));
    }
    if input.capacity_scale_digest.trim().is_empty() {
        return Err(invalid(
            ParameterSetErrorKind::InvalidIdentity,
            "capacity-scale digest must be non-empty",
        ));
    }
    if !input.slip_twist.iter().all(|value| value.is_finite()) {
        return Err(invalid(
            ParameterSetErrorKind::InvalidInput,
            "slip twist must contain only finite values",
        ));
    }

    let length = parameter_set.characteristic_length_m;
    let direction = [
        input.slip_twist[0],
        input.slip_twist[1],
        length * input.slip_twist[2],
    ];
    if !direction.iter().all(|value| value.is_finite()) {
        return Err(invalid(
            ParameterSetErrorKind::NonFiniteEvaluation,
            "characteristic-length-scaled angular velocity is non-finite",
        ));
    }
    let active: Vec<usize> = (0..3).filter(|axis| direction[*axis] != 0.0).collect();
    let build_report =
        |status: SupportEnvelopeStatus,
         lower: Option<f64>,
         upper: Option<f64>,
         method: SupportLowerBoundMethod,
         lower_exact: bool,
         maximum_vertex: Option<[f64; 3]>,
         unbounded_vertex: Option<[f64; 3]>,
         maximum_beta_sum: Option<f64>| PatchSupportEnvelopeReport {
            schema_version: "planar-patch-support-envelope-v0.1".to_owned(),
            parameter_set_schema_version: parameter_set.schema_version.clone(),
            specimen_digest: parameter_set.specimen_digest.clone(),
            patch_geometry_digest: parameter_set.patch_geometry_digest.clone(),
            coordinate_frame_digest: parameter_set.coordinate_frame_digest.clone(),
            rig_calibration_digest: parameter_set.rig_calibration_digest.clone(),
            observation_profile_digest: parameter_set.observation_profile_digest.clone(),
            capacity_convention_digest: parameter_set.capacity_convention_digest.clone(),
            characteristic_length_m: length,
            capacity_scale_interval: input.capacity_scale_interval,
            capacity_scale_digest: input.capacity_scale_digest.clone(),
            slip_twist: input.slip_twist,
            nondimensional_slip_direction: direction,
            active_axis_count: active.len(),
            feasible_vertex_count: parameter_set.feasible_vertices.len(),
            status,
            support_lower_bound: lower,
            support_upper_bound: upper,
            lower_bound_method: method,
            lower_bound_is_exact: lower_exact,
            upper_bound_is_exact_over_feasible_set: true,
            maximum_support_vertex: maximum_vertex,
            unbounded_support_vertex: unbounded_vertex,
            maximum_active_beta_sum: maximum_beta_sum,
            parameter_set_enumeration_complete: parameter_set.enumeration_complete,
        };

    if active.is_empty() {
        return Ok(build_report(
            SupportEnvelopeStatus::ZeroTwist,
            Some(0.0),
            Some(0.0),
            SupportLowerBoundMethod::ZeroTwist,
            true,
            None,
            None,
            Some(0.0),
        ));
    }

    let mut maximum_support = 0.0_f64;
    let mut maximum_vertex = None;
    let mut unbounded_vertex = None;
    let mut maximum_active_beta_sum = 0.0_f64;
    for vertex in &parameter_set.feasible_vertices {
        let active_beta_sum = active.iter().map(|axis| vertex[*axis]).sum::<f64>();
        if !active_beta_sum.is_finite() {
            return Err(invalid(
                ParameterSetErrorKind::NonFiniteEvaluation,
                "active beta sum overflowed",
            ));
        }
        maximum_active_beta_sum = maximum_active_beta_sum.max(active_beta_sum);

        let mut norm = 0.0_f64;
        let mut vertex_unbounded = false;
        for axis in &active {
            let beta = vertex[*axis];
            if beta == 0.0 {
                vertex_unbounded = true;
                break;
            }
            let scaled = direction[*axis] / beta.sqrt();
            if !scaled.is_finite() {
                return Err(invalid(
                    ParameterSetErrorKind::NonFiniteEvaluation,
                    "vertex support evaluation overflowed",
                ));
            }
            norm = norm.hypot(scaled);
        }
        if vertex_unbounded {
            unbounded_vertex = Some(*vertex);
        } else {
            let support = capacity_upper * norm;
            if !support.is_finite() {
                return Err(invalid(
                    ParameterSetErrorKind::NonFiniteEvaluation,
                    "capacity-scaled vertex support overflowed",
                ));
            }
            if support > maximum_support {
                maximum_support = support;
                maximum_vertex = Some(*vertex);
            }
        }
    }

    let active_beta_unbounded = active
        .iter()
        .any(|axis| parameter_set.intervals[*axis].beta_upper.is_none());
    if maximum_active_beta_sum == 0.0 && !active_beta_unbounded {
        return Ok(build_report(
            SupportEnvelopeStatus::UnboundedForAllFeasibleParameters,
            None,
            None,
            SupportLowerBoundMethod::NoFiniteSupportLowerBound,
            false,
            None,
            unbounded_vertex,
            Some(0.0),
        ));
    }

    let (lower_bound, lower_method) = if active_beta_unbounded {
        (0.0, SupportLowerBoundMethod::UnboundedActiveBetaCoordinate)
    } else {
        let l1 = active
            .iter()
            .map(|axis| direction[*axis].abs())
            .sum::<f64>();
        let lower = capacity_lower * l1 / maximum_active_beta_sum.sqrt();
        if !lower.is_finite() {
            return Err(invalid(
                ParameterSetErrorKind::NonFiniteEvaluation,
                "Cauchy-Schwarz support lower bound overflowed",
            ));
        }
        (lower, SupportLowerBoundMethod::JointVertexCauchySchwarz)
    };

    if unbounded_vertex.is_some() {
        return Ok(build_report(
            SupportEnvelopeStatus::UnboundedAbove,
            Some(lower_bound),
            None,
            lower_method,
            false,
            None,
            unbounded_vertex,
            if active_beta_unbounded {
                None
            } else {
                Some(maximum_active_beta_sum)
            },
        ));
    }

    Ok(build_report(
        SupportEnvelopeStatus::Bounded,
        Some(lower_bound),
        Some(maximum_support),
        lower_method,
        false,
        maximum_vertex,
        None,
        if active_beta_unbounded {
            None
        } else {
            Some(maximum_active_beta_sum)
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(lower: f64, upper: Option<f64>) -> ParameterInterval {
        let shape_scale_lower = upper.map_or(Some(0.0), |value| Some(1.0 / value.sqrt()));
        let shape_scale_upper = if lower > 0.0 {
            Some(1.0 / lower.sqrt())
        } else {
            None
        };
        let identifiability = if upper.is_none() {
            AxisIdentifiability::BetaUpperBoundUnboundedDueToNoExcitation
        } else if shape_scale_upper.is_none() {
            AxisIdentifiability::ShapeUpperBoundUnbounded
        } else {
            AxisIdentifiability::FiniteInterval
        };
        ParameterInterval {
            beta_lower: Some(lower),
            beta_upper: upper,
            shape_scale_lower,
            shape_scale_upper,
            identifiability,
            axis_was_excited: upper.is_some(),
        }
    }

    fn report(
        vertices: Vec<[f64; 3]>,
        intervals: [ParameterInterval; 3],
        status: ParameterSetStatus,
    ) -> ParameterSetReport {
        ParameterSetReport {
            schema_version: SCHEMA_VERSION.to_owned(),
            specimen_digest: "specimen-001".to_owned(),
            patch_geometry_digest: "patch-001".to_owned(),
            coordinate_frame_digest: "frame-001".to_owned(),
            rig_calibration_digest: "rig-001".to_owned(),
            observation_profile_digest: "profile-001".to_owned(),
            capacity_convention_digest: "capacity-001".to_owned(),
            characteristic_length_m: 0.5,
            observation_count: 3,
            unique_trial_group_count: 3,
            plane_count: 9,
            plane_triples_checked: 84,
            feasible_vertex_count: vertices.len(),
            duplicate_vertices_collapsed: 0,
            status,
            intervals,
            feasible_vertices: vertices,
            worst_residual_witness: None,
            enumeration_complete: true,
            direct_infeasibility_observation_id: None,
        }
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!((left - right).abs() <= tolerance, "{left} != {right}");
    }

    #[test]
    fn joint_vertices_tighten_upper_bound_vs_independent_axis_box() {
        let parameters = report(
            vec![[1.0, 0.1, 1.0], [0.1, 1.0, 1.0]],
            [
                interval(0.1, Some(1.0)),
                interval(0.1, Some(1.0)),
                interval(1.0, Some(1.0)),
            ],
            ParameterSetStatus::FeasibleWithFiniteIntervals,
        );
        let result = compute_patch_support_envelope(
            &PatchSupportEnvelopeInput {
                parameter_set: &parameters,
                capacity_scale_interval: [2.0, 2.0],
                    capacity_scale_digest: "capacity-2N-synthetic-v1".to_owned(),
                slip_twist: [1.0, 1.0, 0.0],
            },
            SupportEnvelopeConfig::default(),
        )
        .expect("joint support envelope");
        assert_eq!(result.status, SupportEnvelopeStatus::Bounded);
        close(
            result.support_upper_bound.unwrap(),
            2.0 * 11.0_f64.sqrt(),
            1.0e-10,
        );
        close(
            result.support_lower_bound.unwrap(),
            4.0 / 1.1_f64.sqrt(),
            1.0e-10,
        );
        let independent_box_upper = 2.0 * 20.0_f64.sqrt();
        assert!(result.support_upper_bound.unwrap() < independent_box_upper);
        assert!(result.upper_bound_is_vertex_maximum);
        assert!(!result.outward_rounding_applied);
        assert!(!result.lower_bound_is_exact);
    }

    #[test]
    fn unbounded_active_beta_axis_produces_zero_safe_lower_bound() {
        let parameters = report(
            vec![[1.0, 1.0, 1.0]],
            [
                interval(1.0, None),
                interval(1.0, Some(1.0)),
                interval(1.0, Some(1.0)),
            ],
            ParameterSetStatus::FeasibleWithUnboundedAxes,
        );
        let result = compute_patch_support_envelope(
            &PatchSupportEnvelopeInput {
                parameter_set: &parameters,
                capacity_scale_interval: [3.0, 3.0],
                capacity_scale_digest: "capacity-3N-synthetic-v1".to_owned(),
                slip_twist: [1.0, 0.0, 0.0],
            },
            SupportEnvelopeConfig::default(),
        )
        .expect("unbounded beta");
        assert_eq!(result.status, SupportEnvelopeStatus::Bounded);
        assert_eq!(result.support_lower_bound, Some(0.0));
        close(result.support_upper_bound.unwrap(), 3.0, 1.0e-12);
        assert_eq!(
            result.lower_bound_method,
            SupportLowerBoundMethod::UnboundedActiveBetaCoordinate
        );
    }

    #[test]
    fn zero_beta_at_joint_vertex_witnesses_unbounded_support() {
        let parameters = report(
            vec![[0.0, 1.0, 1.0], [1.0, 1.0, 1.0]],
            [
                interval(0.0, Some(1.0)),
                interval(1.0, Some(1.0)),
                interval(1.0, Some(1.0)),
            ],
            ParameterSetStatus::FeasibleWithUnboundedAxes,
        );
        let result = compute_patch_support_envelope(
            &PatchSupportEnvelopeInput {
                parameter_set: &parameters,
                capacity_scale_interval: [1.0, 1.0],
            capacity_scale_digest: "capacity-1N-synthetic-v1".to_owned(),
                slip_twist: [1.0, 0.0, 0.0],
            },
            SupportEnvelopeConfig::default(),
        )
        .expect("unbounded support witness");
        assert_eq!(result.status, SupportEnvelopeStatus::UnboundedAbove);
        assert_eq!(result.support_upper_bound, None);
        assert_eq!(result.unbounded_support_vertex, Some([0.0, 1.0, 1.0]));
    }

    #[test]
    fn zero_twist_has_exact_zero_envelope() {
        let parameters = report(
            vec![[0.25, 4.0, 16.0]],
            [
                interval(0.25, Some(0.25)),
                interval(4.0, Some(4.0)),
                interval(16.0, Some(16.0)),
            ],
            ParameterSetStatus::FeasibleWithFiniteIntervals,
        );
        let result = compute_patch_support_envelope(
            &PatchSupportEnvelopeInput {
                parameter_set: &parameters,
                capacity_scale_interval: [5.0, 5.0],
                capacity_scale_digest: "capacity-5N-synthetic-v1".to_owned(),
                slip_twist: [0.0; 3],
            },
            SupportEnvelopeConfig::default(),
        )
        .expect("zero twist");
        assert_eq!(result.status, SupportEnvelopeStatus::ZeroTwist);
        assert_eq!(result.support_lower_bound, Some(0.0));
        assert_eq!(result.support_upper_bound, Some(0.0));
        assert!(result.lower_bound_is_exact);
    }

    #[test]
    fn torsional_twist_uses_characteristic_length_for_moment_units() {
        let parameters = report(
            vec![[1.0, 1.0, 4.0]],
            [
                interval(1.0, Some(1.0)),
                interval(1.0, Some(1.0)),
                interval(4.0, Some(4.0)),
            ],
            ParameterSetStatus::FeasibleWithFiniteIntervals,
        );
        let result = compute_patch_support_envelope(
            &PatchSupportEnvelopeInput {
                parameter_set: &parameters,
                capacity_scale_interval: [2.0, 2.0],
                    capacity_scale_digest: "capacity-2N-synthetic-v1".to_owned(),
                slip_twist: [0.0, 0.0, 2.0],
            },
            SupportEnvelopeConfig::default(),
        )
        .expect("torsional units");
        close(result.nondimensional_slip_direction[2], 1.0, 1.0e-12);
        close(result.support_upper_bound.unwrap(), 1.0, 1.0e-12);
    }

    #[test]
    fn zero_velocity_on_unexcited_axis_does_not_make_support_unbounded() {
        let parameters = report(
            vec![[1.0, 1.0, 0.0]],
            [
                interval(1.0, Some(1.0)),
                interval(1.0, Some(1.0)),
                interval(0.0, None),
            ],
            ParameterSetStatus::FeasibleWithUnboundedAxes,
        );
        let result = compute_patch_support_envelope(
            &PatchSupportEnvelopeInput {
                parameter_set: &parameters,
                capacity_scale_interval: [2.0, 2.0],
                    capacity_scale_digest: "capacity-2N-synthetic-v1".to_owned(),
                slip_twist: [1.0, 1.0, 0.0],
            },
            SupportEnvelopeConfig::default(),
        )
        .expect("inactive unbounded axis");
        assert_eq!(result.status, SupportEnvelopeStatus::Bounded);
        close(
            result.support_upper_bound.unwrap(),
            2.0 * 2.0_f64.sqrt(),
            1.0e-12,
        );
    }

    #[test]
    fn rejects_incomplete_or_interval_inconsistent_parameter_reports() {
        let mut parameters = report(
            vec![[1.0, 1.0, 1.0]],
            [
                interval(1.0, Some(1.0)),
                interval(1.0, Some(1.0)),
                interval(1.0, Some(1.0)),
            ],
            ParameterSetStatus::FeasibleWithFiniteIntervals,
        );
        parameters.enumeration_complete = false;
        let input = PatchSupportEnvelopeInput {
            parameter_set: &parameters,
            capacity_scale_interval: [1.0, 1.0],
            capacity_scale_digest: "capacity-1N-synthetic-v1".to_owned(),
            slip_twist: [1.0, 0.0, 0.0],
        };
        assert_eq!(
            compute_patch_support_envelope(&input, SupportEnvelopeConfig::default())
                .expect_err("incomplete vertex enumeration")
                .kind,
            ParameterSetErrorKind::InvalidInput
        );

        parameters.enumeration_complete = true;
        parameters.intervals[0].beta_lower = Some(0.5);
        assert_eq!(
            compute_patch_support_envelope(&input, SupportEnvelopeConfig::default())
                .expect_err("inconsistent marginal interval")
                .kind,
            ParameterSetErrorKind::InvalidInput
        );
    }

    #[test]
    fn vertex_order_does_not_change_support_envelope() {
        let vertices = vec![[1.0, 0.1, 1.0], [0.1, 1.0, 1.0]];
        let intervals = [
            interval(0.1, Some(1.0)),
            interval(0.1, Some(1.0)),
            interval(1.0, Some(1.0)),
        ];
        let first = report(
            vertices.clone(),
            intervals.clone(),
            ParameterSetStatus::FeasibleWithFiniteIntervals,
        );
        let second = report(
            vertices.into_iter().rev().collect(),
            intervals,
            ParameterSetStatus::FeasibleWithFiniteIntervals,
        );
        let evaluate = |parameter_set: &ParameterSetReport| {
            compute_patch_support_envelope(
                &PatchSupportEnvelopeInput {
                    parameter_set,
                    capacity_scale_interval: [2.0, 2.0],
                    capacity_scale_digest: "capacity-2N-synthetic-v1".to_owned(),
                    slip_twist: [1.0, 1.0, 0.0],
                },
                SupportEnvelopeConfig::default(),
            )
            .expect("order invariant envelope")
        };
        assert_eq!(
            evaluate(&first).support_upper_bound,
            evaluate(&second).support_upper_bound
        );
        assert_eq!(
            evaluate(&first).support_lower_bound,
            evaluate(&second).support_lower_bound
        );
    }

    #[test]
    fn capacity_interval_scales_lower_and_upper_support_monotonically() {
        let parameters = report(
            vec![[1.0, 1.0, 1.0]],
            [
                interval(1.0, Some(1.0)),
                interval(1.0, Some(1.0)),
                interval(1.0, Some(1.0)),
            ],
            ParameterSetStatus::FeasibleWithFiniteIntervals,
        );
        let result = compute_patch_support_envelope(
            &PatchSupportEnvelopeInput {
                parameter_set: &parameters,
                capacity_scale_interval: [1.0, 2.0],
                capacity_scale_digest: "mu-normal-load-bracket-synthetic-v1".to_owned(),
                slip_twist: [1.0, 0.0, 0.0],
            },
            SupportEnvelopeConfig::default(),
        )
        .expect("capacity interval");
        close(result.support_lower_bound.unwrap(), 1.0, 1.0e-12);
        close(result.support_upper_bound.unwrap(), 2.0, 1.0e-12);
        assert_eq!(result.capacity_scale_interval, [1.0, 2.0]);
        assert_eq!(
            result.capacity_scale_digest,
            "mu-normal-load-bracket-synthetic-v1"
        );
    }

    #[test]
    fn invalid_capacity_and_nonfinite_twist_fail_closed() {
        let parameters = report(
            vec![[1.0, 1.0, 1.0]],
            [
                interval(1.0, Some(1.0)),
                interval(1.0, Some(1.0)),
                interval(1.0, Some(1.0)),
            ],
            ParameterSetStatus::FeasibleWithFiniteIntervals,
        );
        for (capacity_scale_interval, slip_twist) in [
            ([0.0, 0.0], [1.0, 0.0, 0.0]),
            ([2.0, 1.0], [1.0, 0.0, 0.0]),
            ([1.0, 1.0], [f64::NAN, 0.0, 0.0]),
        ] {
            assert_eq!(
                compute_patch_support_envelope(
                    &PatchSupportEnvelopeInput {
                        parameter_set: &parameters,
                        capacity_scale_interval,
                        capacity_scale_digest: "capacity-invalid-test-v1".to_owned(),
                        slip_twist,
                    },
                    SupportEnvelopeConfig::default(),
                )
                .expect_err("invalid support input")
                .kind,
                ParameterSetErrorKind::InvalidInput
            );
        }
    }
}
