// SPDX-License-Identifier: MIT
#![allow(clippy::float_cmp)] // Exact domain boundary comparisons are intentional.

//! Branch-cut-free interval phase acceptance using normalized phasor similarity.
//!
//! The acceptance expression does not evaluate a wrapped angle. It compares
//! the normalized dot product of two nonzero phase vectors with a supplied,
//! provenance-bound cosine threshold.

use magnetic_interval_ad::{
    Interval, IntervalAdError, IntervalDual, ParameterDomain, SpatialDerivativeSupremumEvidence,
    SpatialMappingError, ValidatedDerivativeEnclosure, certify_expression_centered,
    map_spatial_derivative_suprema,
};

pub const PHASE_MARGIN_EXPRESSION_ID: &str = "normalized-phasor-cosine-margin-v0.1";

#[derive(Clone, Debug, PartialEq)]
pub struct PhasorPhaseMarginCertificate {
    pub derivative_enclosure: ValidatedDerivativeEnclosure<5>,
    pub measured_norm_squared_enclosure: Interval,
    pub reference_norm_squared_enclosure: Interval,
    pub metric_id: String,
    pub shortest_arc_contract: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhasorComponents<const N: usize> {
    pub measured_x: IntervalDual<N>,
    pub measured_y: IntervalDual<N>,
    pub reference_x: IntervalDual<N>,
    pub reference_y: IntervalDual<N>,
    pub minimum_cosine: IntervalDual<N>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialPhasorPhaseMarginCertificate {
    pub derivative_enclosure: ValidatedDerivativeEnclosure<3>,
    pub spatial_derivative_evidence: SpatialDerivativeSupremumEvidence,
    pub measured_norm_squared_enclosure: Interval,
    pub reference_norm_squared_enclosure: Interval,
    pub minimum_cosine_enclosure: Interval,
    pub metric_id: String,
    pub shortest_arc_contract: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseMarginError {
    WrongExpressionIdentity,
    WrongParameterOrder { index: usize },
    WrongParameterUnit { index: usize },
    InvalidThresholdCosineDomain,
    MeasuredMagnitudeMayBeZero,
    ReferenceMagnitudeMayBeZero,
    InvalidSpatialParameterOrder { axis: usize },
    InvalidSpatialParameterUnit { axis: usize },
    InvalidSpatialOutputUnit,
    SpatialMapping(SpatialMappingError),
    IntervalAd(IntervalAdError),
}

impl From<IntervalAdError> for PhaseMarginError {
    fn from(error: IntervalAdError) -> Self {
        Self::IntervalAd(error)
    }
}

fn expected_parameter_ids() -> [&'static str; 5] {
    [
        "measured_x",
        "measured_y",
        "reference_x",
        "reference_y",
        "maximum_cosine",
    ]
}

fn validate_domain(domain: &ParameterDomain<5>) -> Result<(), PhaseMarginError> {
    if domain.expression_id != PHASE_MARGIN_EXPRESSION_ID {
        return Err(PhaseMarginError::WrongExpressionIdentity);
    }

    let expected_ids = expected_parameter_ids();
    for (index, ((actual_id, expected_id), unit)) in domain
        .parameter_ids
        .iter()
        .zip(expected_ids.iter())
        .zip(domain.parameter_units.iter())
        .enumerate()
    {
        if actual_id != *expected_id {
            return Err(PhaseMarginError::WrongParameterOrder { index });
        }
        if unit != "dimensionless" {
            return Err(PhaseMarginError::WrongParameterUnit { index });
        }
    }

    let threshold = domain.parameter_bounds[4];
    if !threshold.is_finite() || threshold.lower() <= -1.0 || threshold.upper() > 1.0 {
        return Err(PhaseMarginError::InvalidThresholdCosineDomain);
    }

    let measured_norm_squared =
        domain.parameter_bounds[0].square() + domain.parameter_bounds[1].square();
    if !measured_norm_squared.is_finite() || measured_norm_squared.lower() <= 0.0 {
        return Err(PhaseMarginError::MeasuredMagnitudeMayBeZero);
    }

    let reference_norm_squared =
        domain.parameter_bounds[2].square() + domain.parameter_bounds[3].square();
    if !reference_norm_squared.is_finite() || reference_norm_squared.lower() <= 0.0 {
        return Err(PhaseMarginError::ReferenceMagnitudeMayBeZero);
    }

    Ok(())
}

fn normalized_phasor_margin<const N: usize>(
    components: PhasorComponents<N>,
) -> Result<IntervalDual<N>, IntervalAdError> {
    let measured_norm_squared = components.measured_x.square() + components.measured_y.square();
    let reference_norm_squared = components.reference_x.square() + components.reference_y.square();
    let magnitude_product = measured_norm_squared * reference_norm_squared;
    let magnitude = magnitude_product.sqrt()?;
    let dot_product = components.measured_x * components.reference_x
        + components.measured_y * components.reference_y;
    let normalized_similarity = dot_product / magnitude;
    Ok(normalized_similarity - components.minimum_cosine)
}

fn phase_similarity_expression(
    [
        measured_x,
        measured_y,
        reference_x,
        reference_y,
        minimum_cosine,
    ]: [IntervalDual<5>; 5],
) -> Result<IntervalDual<5>, IntervalAdError> {
    normalized_phasor_margin(PhasorComponents {
        measured_x,
        measured_y,
        reference_x,
        reference_y,
        minimum_cosine,
    })
}

/// Certify the shortest-arc phase tolerance using a branch-cut-free scalar
/// functional margin.
///
/// The parameter order is fixed:
/// [measured_x, measured_y, reference_x, reference_y, maximum_cosine].
///
/// A nonzero lower magnitude bound is mandatory for both vectors. The
/// maximum-cosine threshold is supplied as an interval; this function does not
/// call atan2, sin or cos.
pub fn certify_phasor_phase_margin(
    domain: &ParameterDomain<5>,
) -> Result<PhasorPhaseMarginCertificate, PhaseMarginError> {
    validate_domain(domain)?;

    let measured_norm_squared =
        domain.parameter_bounds[0].square() + domain.parameter_bounds[1].square();
    let reference_norm_squared =
        domain.parameter_bounds[2].square() + domain.parameter_bounds[3].square();

    let derivative_enclosure = certify_expression_centered(domain, phase_similarity_expression)?;

    Ok(PhasorPhaseMarginCertificate {
        derivative_enclosure,
        measured_norm_squared_enclosure: measured_norm_squared,
        reference_norm_squared_enclosure: reference_norm_squared,
        metric_id: "normalized-phasor-dot-product".to_owned(),
        shortest_arc_contract:
            "normalized_dot(p,q) >= cos(delta_max), with 0 <= delta_max < pi".to_owned(),
    })
}

/// Certify a spatially varying branch-free phase margin and map its
/// whole-cell XYZ derivative suprema to the spatial-certification contract.
///
/// The evaluator maps spatial coordinate duals [x, y, z] to measured/reference
/// phasor components and a supplied cosine threshold. The returned derivative
/// record can populate the spatial certifier's derivative bounds, but it does
/// not create the independent centre-sample margin observation.
pub fn certify_spatial_phasor_phase_margin<F>(
    domain: &ParameterDomain<3>,
    evaluate: F,
) -> Result<SpatialPhasorPhaseMarginCertificate, PhaseMarginError>
where
    F: Fn([IntervalDual<3>; 3]) -> Result<PhasorComponents<3>, IntervalAdError>,
{
    if domain.expression_id != PHASE_MARGIN_EXPRESSION_ID {
        return Err(PhaseMarginError::WrongExpressionIdentity);
    }
    if domain.output_unit != "dimensionless" {
        return Err(PhaseMarginError::InvalidSpatialOutputUnit);
    }

    let expected_axes = ["x", "y", "z"];
    for (axis, ((actual_axis, expected_axis), unit)) in domain
        .parameter_ids
        .iter()
        .zip(expected_axes.iter())
        .zip(domain.parameter_units.iter())
        .enumerate()
    {
        if actual_axis != expected_axis {
            return Err(PhaseMarginError::InvalidSpatialParameterOrder { axis });
        }
        if unit != "metre" {
            return Err(PhaseMarginError::InvalidSpatialParameterUnit { axis });
        }
    }

    let measured_norm = certify_expression_centered(domain, |variables| {
        let components = evaluate(variables)?;
        Ok(components.measured_x.square() + components.measured_y.square())
    })?;
    if measured_norm.value_enclosure.lower() <= 0.0 {
        return Err(PhaseMarginError::MeasuredMagnitudeMayBeZero);
    }

    let reference_norm = certify_expression_centered(domain, |variables| {
        let components = evaluate(variables)?;
        Ok(components.reference_x.square() + components.reference_y.square())
    })?;
    if reference_norm.value_enclosure.lower() <= 0.0 {
        return Err(PhaseMarginError::ReferenceMagnitudeMayBeZero);
    }

    let threshold = certify_expression_centered(domain, |variables| {
        let components = evaluate(variables)?;
        Ok(components.minimum_cosine)
    })?;
    if threshold.value_enclosure.lower() <= -1.0 || threshold.value_enclosure.upper() > 1.0 {
        return Err(PhaseMarginError::InvalidThresholdCosineDomain);
    }

    let derivative_enclosure = certify_expression_centered(domain, |variables| {
        normalized_phasor_margin(evaluate(variables)?)
    })?;

    let spatial_derivative_evidence = map_spatial_derivative_suprema(
        &derivative_enclosure,
        &domain.cell_id,
        &domain.requirement_id,
        ["x", "y", "z"],
        "metre",
    )
    .map_err(PhaseMarginError::SpatialMapping)?;

    Ok(SpatialPhasorPhaseMarginCertificate {
        derivative_enclosure,
        spatial_derivative_evidence,
        measured_norm_squared_enclosure: measured_norm.value_enclosure,
        reference_norm_squared_enclosure: reference_norm.value_enclosure,
        minimum_cosine_enclosure: threshold.value_enclosure,
        metric_id: "normalized-phasor-dot-product".to_owned(),
        shortest_arc_contract:
            "normalized_dot(p,q) >= cos(delta_max), with 0 <= delta_max < pi".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn interval(lower: f64, upper: f64) -> Interval {
        Interval::new(lower, upper).expect("valid phase interval")
    }

    fn domain(bounds: [Interval; 5]) -> ParameterDomain<5> {
        ParameterDomain {
            record_id: "phasor-margin-record-001".to_owned(),
            expression_id: PHASE_MARGIN_EXPRESSION_ID.to_owned(),
            cell_id: "encoder-phase-cell-001".to_owned(),
            input_sha256: DIGEST_A.to_owned(),
            source_revision: "phasor-model-v1".to_owned(),
            implementation_id: "magnetic-phasor-margin".to_owned(),
            artifact_sha256: DIGEST_B.to_owned(),
            environment_sha256: DIGEST_C.to_owned(),
            parameter_ids: [
                "measured_x".to_owned(),
                "measured_y".to_owned(),
                "reference_x".to_owned(),
                "reference_y".to_owned(),
                "maximum_cosine".to_owned(),
            ],
            parameter_units: [
                "dimensionless".to_owned(),
                "dimensionless".to_owned(),
                "dimensionless".to_owned(),
                "dimensionless".to_owned(),
                "dimensionless".to_owned(),
            ],
            parameter_bounds: bounds,
            output_unit: "dimensionless".to_owned(),
            coordinate_frame_id: "encoder-phase-frame".to_owned(),
            operating_condition_id: "nominal-speed-temperature".to_owned(),
            model_set_id: "encoder-response-model-v1".to_owned(),
            solver_run_id: "response-run-001".to_owned(),
            geometry_sha256: DIGEST_A.to_owned(),
            material_sha256: DIGEST_B.to_owned(),
            requirement_id: "maximum-shortest-arc-phase-error".to_owned(),
        }
    }

    fn point(value: f64) -> Interval {
        interval(value, value)
    }

    fn spatial_domain() -> ParameterDomain<3> {
        ParameterDomain {
            record_id: "spatial-phase-record-001".to_owned(),
            expression_id: PHASE_MARGIN_EXPRESSION_ID.to_owned(),
            cell_id: "phase-cell-xyz-001".to_owned(),
            input_sha256: DIGEST_A.to_owned(),
            source_revision: "spatial-phasor-model-v1".to_owned(),
            implementation_id: "magnetic-phasor-margin".to_owned(),
            artifact_sha256: DIGEST_B.to_owned(),
            environment_sha256: DIGEST_C.to_owned(),
            parameter_ids: ["x".to_owned(), "y".to_owned(), "z".to_owned()],
            parameter_units: ["metre".to_owned(), "metre".to_owned(), "metre".to_owned()],
            parameter_bounds: [
                interval(-0.1, 0.1),
                interval(-0.1, 0.1),
                interval(-0.05, 0.05),
            ],
            output_unit: "dimensionless".to_owned(),
            coordinate_frame_id: "encoder-phase-spatial-frame".to_owned(),
            operating_condition_id: "nominal-speed-temperature".to_owned(),
            model_set_id: "encoder-response-model-v1".to_owned(),
            solver_run_id: "spatial-response-run-001".to_owned(),
            geometry_sha256: DIGEST_A.to_owned(),
            material_sha256: DIGEST_B.to_owned(),
            requirement_id: "maximum-shortest-arc-phase-error".to_owned(),
        }
    }

    fn spatial_phasors(
        [x, y, _z]: [IntervalDual<3>; 3],
    ) -> Result<PhasorComponents<3>, IntervalAdError> {
        let one = IntervalDual::constant(Interval::ONE);
        let zero = IntervalDual::constant(Interval::ZERO);
        let point_one = IntervalDual::constant(point(0.1));
        let point_two = IntervalDual::constant(point(0.2));
        let cosine_limit = IntervalDual::constant(point(0.95));
        Ok(PhasorComponents {
            measured_x: one + point_one * x,
            measured_y: point_two * y,
            reference_x: one,
            reference_y: zero,
            minimum_cosine: cosine_limit,
        })
    }

    fn enclosure_contains(enclosure: Interval, value: f64) -> bool {
        enclosure.lower() <= value && enclosure.upper() >= value
    }

    #[test]
    fn phase_pairs_straddling_the_negative_axis_remain_compliant() {
        // The conventional atan2 angles lie on opposite sides of ±pi,
        // but the shortest physical separation is approximately 0.02 rad.
        let scope = domain([
            point(-1.0),
            interval(-0.01, -0.009),
            point(-1.0),
            interval(0.009, 0.01),
            point(0.95),
        ]);
        let result = certify_phasor_phase_margin(&scope)
            .expect("nonzero phasors across the angle branch cut");
        assert!(result.derivative_enclosure.value_enclosure.lower() > 0.0);
        assert!(
            result
                .derivative_enclosure
                .derivative_abs_supremum
                .iter()
                .all(|bound| bound.is_finite())
        );
        assert_eq!(result.metric_id, "normalized-phasor-dot-product");
    }

    #[test]
    fn phase_pair_outside_the_tolerance_has_a_negative_margin() {
        // Opposite vectors are separated by pi; cos(pi) = -1 < 0.9.
        let scope = domain([point(1.0), point(0.0), point(-1.0), point(0.0), point(0.9)]);
        let result = certify_phasor_phase_margin(&scope).expect("nonzero opposite phasors");
        assert!(result.derivative_enclosure.value_enclosure.upper() < 0.0);
    }

    #[test]
    fn common_quarter_turn_rotation_preserves_the_margin() {
        let original = domain([point(1.0), point(0.0), point(0.8), point(0.6), point(0.7)]);
        // Apply the exact rotation (x,y) -> (-y,x) to both vectors.
        let rotated = domain([point(0.0), point(1.0), point(-0.6), point(0.8), point(0.7)]);
        let a = certify_phasor_phase_margin(&original).expect("original frame");
        let b = certify_phasor_phase_margin(&rotated).expect("rotated frame");
        assert!(enclosure_contains(
            a.derivative_enclosure.value_enclosure,
            0.1
        ));
        assert!(enclosure_contains(
            b.derivative_enclosure.value_enclosure,
            0.1
        ));
    }

    #[test]
    fn positive_phasor_scaling_preserves_the_margin() {
        let original = domain([point(1.0), point(0.0), point(0.8), point(0.6), point(0.7)]);
        let scaled = domain([point(2.0), point(0.0), point(2.4), point(1.8), point(0.7)]);
        let a = certify_phasor_phase_margin(&original).expect("original amplitudes");
        let b = certify_phasor_phase_margin(&scaled).expect("positive scaled amplitudes");
        assert!(enclosure_contains(
            a.derivative_enclosure.value_enclosure,
            0.1
        ));
        assert!(enclosure_contains(
            b.derivative_enclosure.value_enclosure,
            0.1
        ));
    }

    #[test]
    fn measured_phasor_box_that_contains_zero_fails_closed() {
        let scope = domain([
            interval(-0.1, 0.1),
            interval(-0.1, 0.1),
            point(1.0),
            point(0.0),
            point(0.9),
        ]);
        assert_eq!(
            certify_phasor_phase_margin(&scope)
                .expect_err("zero measured phasor makes phase undefined"),
            PhaseMarginError::MeasuredMagnitudeMayBeZero
        );
    }

    #[test]
    fn reference_phasor_box_that_contains_zero_fails_closed() {
        let scope = domain([
            point(1.0),
            point(0.0),
            interval(-0.1, 0.1),
            interval(-0.1, 0.1),
            point(0.9),
        ]);
        assert_eq!(
            certify_phasor_phase_margin(&scope)
                .expect_err("zero reference phasor makes phase undefined"),
            PhaseMarginError::ReferenceMagnitudeMayBeZero
        );
    }

    #[test]
    fn threshold_cosine_must_represent_a_shortest_arc_below_pi() {
        let high = domain([
            point(1.0),
            point(0.0),
            point(1.0),
            point(0.0),
            interval(0.99, 1.01),
        ]);
        assert_eq!(
            certify_phasor_phase_margin(&high).expect_err("cosine threshold cannot exceed one"),
            PhaseMarginError::InvalidThresholdCosineDomain
        );

        let low = domain([
            point(1.0),
            point(0.0),
            point(1.0),
            point(0.0),
            interval(-1.0, -0.9),
        ]);
        assert_eq!(
            certify_phasor_phase_margin(&low)
                .expect_err("threshold at -1 admits the degenerate pi bound"),
            PhaseMarginError::InvalidThresholdCosineDomain
        );
    }

    #[test]
    fn phase_parameter_order_and_dimensionless_units_are_enforced() {
        let mut wrong_order = domain([point(1.0), point(0.0), point(0.8), point(0.6), point(0.7)]);
        wrong_order.parameter_ids.swap(0, 1);
        assert_eq!(
            certify_phasor_phase_margin(&wrong_order)
                .expect_err("expression parameter order is explicit"),
            PhaseMarginError::WrongParameterOrder { index: 0 }
        );

        let mut wrong_unit = domain([point(1.0), point(0.0), point(0.8), point(0.6), point(0.7)]);
        wrong_unit.parameter_units[0] = "tesla".to_owned();
        assert_eq!(
            certify_phasor_phase_margin(&wrong_unit)
                .expect_err("phasor components are dimensionless"),
            PhaseMarginError::WrongParameterUnit { index: 0 }
        );
    }

    #[test]
    fn spatial_phase_margin_exports_xyz_derivative_suprema() {
        let scope = spatial_domain();
        let certificate = certify_spatial_phasor_phase_margin(&scope, spatial_phasors)
            .expect("nonzero phasors throughout the spatial cell");
        let mapped = &certificate.spatial_derivative_evidence;

        assert_eq!(mapped.cell_id, "phase-cell-xyz-001");
        assert_eq!(
            mapped.axis_parameter_ids,
            ["x".to_owned(), "y".to_owned(), "z".to_owned()]
        );
        assert_eq!(mapped.axis_bounds_m, scope.parameter_bounds);
        assert_eq!(mapped.output_unit, "dimensionless");
        assert_eq!(mapped.derivative_unit, "dimensionless/metre");
        assert_eq!(mapped.requirement_id, scope.requirement_id);
        assert!(
            mapped
                .derivative_abs_supremum_by_axis
                .iter()
                .all(|value| value.is_finite())
        );
        assert!(certificate.measured_norm_squared_enclosure.lower() > 0.0);
        assert!(certificate.reference_norm_squared_enclosure.lower() > 0.0);
        assert!(certificate.minimum_cosine_enclosure.lower() > -1.0);
        assert!(certificate.minimum_cosine_enclosure.upper() <= 1.0);
    }

    #[test]
    fn spatial_phase_margin_does_not_create_a_centre_measurement() {
        let scope = spatial_domain();
        let certificate = certify_spatial_phasor_phase_margin(&scope, spatial_phasors)
            .expect("valid spatial phase envelope");
        assert_eq!(certificate.derivative_enclosure.cell_id, scope.cell_id);
        // The spatial centre sample and uncertainty are still required by
        // CellMarginEvidence; only derivative evidence is provided here.
        assert_eq!(
            certificate.spatial_derivative_evidence.source_record_id,
            certificate.derivative_enclosure.record_id
        );
    }

    #[test]
    fn spatial_phase_margin_rejects_wrong_xyz_parameter_identity() {
        let mut scope = spatial_domain();
        scope.parameter_ids.swap(0, 1);
        assert_eq!(
            certify_spatial_phasor_phase_margin(&scope, spatial_phasors)
                .expect_err("XYZ axis identities are explicit"),
            PhaseMarginError::InvalidSpatialParameterOrder { axis: 0 }
        );
    }

    #[test]
    fn spatial_phase_margin_rejects_non_metre_coordinates() {
        let mut scope = spatial_domain();
        scope.parameter_units[1] = "millimetre".to_owned();
        assert_eq!(
            certify_spatial_phasor_phase_margin(&scope, spatial_phasors)
                .expect_err("spatial bounds are in metres"),
            PhaseMarginError::InvalidSpatialParameterUnit { axis: 1 }
        );
    }

    #[test]
    fn spatial_phase_margin_binds_the_declared_requirement_identity() {
        let mut scope = spatial_domain();
        scope.requirement_id = "other-phase-limit".to_owned();
        // Identity itself is still valid and will be carried through; the
        // adapter binds this record to the explicitly supplied requirement.
        let certificate = certify_spatial_phasor_phase_margin(&scope, spatial_phasors)
            .expect("valid alternative requirement identity");
        assert_eq!(
            certificate.spatial_derivative_evidence.requirement_id,
            "other-phase-limit"
        );
    }

    #[test]
    fn non_singular_phase_domain_has_finite_whole_box_derivative_bounds() {
        let scope = domain([
            interval(0.99, 1.01),
            interval(-0.01, 0.01),
            interval(0.79, 0.81),
            interval(0.59, 0.61),
            interval(0.69, 0.71),
        ]);
        let result = certify_phasor_phase_margin(&scope)
            .expect("all phasor norms are bounded away from zero");
        assert!(
            result
                .derivative_enclosure
                .derivative_abs_supremum
                .iter()
                .all(|bound| bound.is_finite())
        );
        assert!(result.derivative_enclosure.value_enclosure.is_finite());
        assert!(result.measured_norm_squared_enclosure.lower() > 0.0);
        assert!(result.reference_norm_squared_enclosure.lower() > 0.0);
    }
}
