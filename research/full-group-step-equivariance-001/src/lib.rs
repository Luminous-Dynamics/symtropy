// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! One-step physical equivariance for an O(2)-symmetric constrained reference.
//!
//! The integrator is a bounded projected velocity-Verlet / SHAKE-RATTLE-shaped
//! map on a circle. It is a test reference, not a production multibody solver.

use metric_induced_error_envelope_reference::{
    ConstraintMultiplierV1, ConstraintResidualV1, ConstraintSpace, CoordinateFrame, Cotangent,
    DelassusMetricV1, Metric, MetricErrorV1, MetricProfileV1, Tangent, pairing,
};

pub type Matrix2 = [[f64; 2]; 2];
pub type Vector2<F> = Tangent<F, 2>;
pub type Covector2<F> = Cotangent<F, 2>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircleStepProfileV1 {
    pub mass: f64,
    pub spring: f64,
    pub timestep: f64,
    pub radius: f64,
    pub metric_profile: MetricProfileV1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepOutcome<F: CoordinateFrame> {
    pub position: Tangent<F, 2>,
    pub velocity: Tangent<F, 2>,
    pub position_multiplier: f64,
    pub velocity_multiplier: f64,
    pub position_constraint_defect: f64,
    pub velocity_constraint_defect: f64,
    pub delassus: f64,
    pub multiplier_metric_norm: f64,
    pub residual_dual_norm: f64,
    pub external_work: f64,
    pub kinetic_energy: f64,
    pub potential_energy: f64,
}

fn add_vectors<F: CoordinateFrame>(
    left: &Vector2<F>,
    right: &Vector2<F>,
) -> Result<Vector2<F>, MetricErrorV1> {
    Tangent::new([
        left.components()[0] + right.components()[0],
        left.components()[1] + right.components()[1],
    ])
}

fn scale_vector<F: CoordinateFrame>(
    vector: &Vector2<F>,
    scale: f64,
) -> Result<Vector2<F>, MetricErrorV1> {
    Tangent::new([
        vector.components()[0] * scale,
        vector.components()[1] * scale,
    ])
}

fn add_covectors<F: CoordinateFrame>(
    left: &Covector2<F>,
    right: &Covector2<F>,
) -> Result<Covector2<F>, MetricErrorV1> {
    Cotangent::new([
        left.components()[0] + right.components()[0],
        left.components()[1] + right.components()[1],
    ])
}

fn scale_covector<F: CoordinateFrame>(
    covector: &Covector2<F>,
    scale: f64,
) -> Result<Covector2<F>, MetricErrorV1> {
    Cotangent::new([
        covector.components()[0] * scale,
        covector.components()[1] * scale,
    ])
}

fn apply_matrix(matrix: &Matrix2, vector: &[f64; 2]) -> [f64; 2] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1],
    ]
}

fn transpose(matrix: &Matrix2) -> Matrix2 {
    [[matrix[0][0], matrix[1][0]], [matrix[0][1], matrix[1][1]]]
}

fn multiply(left: &Matrix2, right: &Matrix2) -> Matrix2 {
    [
        [
            left[0][0] * right[0][0] + left[0][1] * right[1][0],
            left[0][0] * right[0][1] + left[0][1] * right[1][1],
        ],
        [
            left[1][0] * right[0][0] + left[1][1] * right[1][0],
            left[1][0] * right[0][1] + left[1][1] * right[1][1],
        ],
    ]
}

fn inverse(matrix: &Matrix2) -> Result<Matrix2, MetricErrorV1> {
    let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
    if !determinant.is_finite() || determinant.abs() <= 1.0e-14 {
        return Err(MetricErrorV1::SingularCoordinateTransform { column: 0 });
    }
    let inverse = [
        [matrix[1][1] / determinant, -matrix[0][1] / determinant],
        [-matrix[1][0] / determinant, matrix[0][0] / determinant],
    ];
    if inverse.iter().flatten().all(|value| value.is_finite()) {
        Ok(inverse)
    } else {
        Err(MetricErrorV1::NonFiniteIntermediate)
    }
}

pub fn transform_tangent<F: CoordinateFrame>(
    action: &Matrix2,
    vector: &Tangent<F, 2>,
) -> Result<Tangent<F, 2>, MetricErrorV1> {
    Tangent::new(apply_matrix(action, vector.components()))
}

pub fn transform_covector<F: CoordinateFrame>(
    action: &Matrix2,
    covector: &Cotangent<F, 2>,
) -> Result<Cotangent<F, 2>, MetricErrorV1> {
    let inverse_transpose = transpose(&inverse(action)?);
    Cotangent::new(apply_matrix(&inverse_transpose, covector.components()))
}

pub fn conjugate_action(
    rebasis: &Matrix2,
    action_from: &Matrix2,
) -> Result<Matrix2, MetricErrorV1> {
    let inverse_rebasis = inverse(rebasis)?;
    Ok(multiply(&multiply(&inverse_rebasis, action_from), rebasis))
}

fn force_at<F: CoordinateFrame>(
    mass_metric: &Metric<F, 2>,
    position: &Tangent<F, 2>,
    external_force: &Cotangent<F, 2>,
    mass: f64,
    spring: f64,
) -> Result<Cotangent<F, 2>, MetricErrorV1> {
    let spring_covector = scale_covector(&mass_metric.lower(position)?, -spring / mass)?;
    add_covectors(external_force, &spring_covector)
}

fn acceleration<F: CoordinateFrame>(
    mass_metric: &Metric<F, 2>,
    force: &Cotangent<F, 2>,
) -> Result<Tangent<F, 2>, MetricErrorV1> {
    mass_metric.raise(force)
}

/// One constrained projected velocity-Verlet step on q^T G q = radius^2,
/// where M = mass * G is the admitted SPD mass metric.
///
/// The two scalar multipliers witness the position and velocity projections.
/// Constraint Jacobian is J(q) = (2/mass) q^T M.
pub fn projected_circle_step<F: CoordinateFrame>(
    mass_metric: &Metric<F, 2>,
    position: &Tangent<F, 2>,
    velocity: &Tangent<F, 2>,
    external_force: &Cotangent<F, 2>,
    profile: CircleStepProfileV1,
) -> Result<StepOutcome<F>, MetricErrorV1> {
    let CircleStepProfileV1 {
        mass,
        spring,
        timestep,
        radius,
        metric_profile,
    } = profile;
    if !mass.is_finite()
        || mass <= 0.0
        || !spring.is_finite()
        || !timestep.is_finite()
        || timestep <= 0.0
        || !radius.is_finite()
        || radius <= 0.0
    {
        return Err(MetricErrorV1::InvalidProfile);
    }

    let force_0 = force_at(mass_metric, position, external_force, mass, spring)?;
    let acceleration_0 = acceleration(mass_metric, &force_0)?;
    let drift = add_vectors(
        position,
        &add_vectors(
            &scale_vector(velocity, timestep)?,
            &scale_vector(&acceleration_0, 0.5 * timestep * timestep)?,
        )?,
    )?;

    let drift_norm = mass_metric.tangent_norm(&drift)?;
    if drift_norm <= 1.0e-14 {
        return Err(MetricErrorV1::NegativeMetricQuadraticForm);
    }
    // ||q||_M = sqrt(mass) * ||q||_G.
    let scale = radius * mass.sqrt() / drift_norm;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }
    let position_next = scale_vector(&drift, scale)?;

    let force_1 = force_at(mass_metric, &position_next, external_force, mass, spring)?;
    let acceleration_1 = acceleration(mass_metric, &force_1)?;
    let velocity_star = add_vectors(
        velocity,
        &scale_vector(
            &add_vectors(&acceleration_0, &acceleration_1)?,
            0.5 * timestep,
        )?,
    )?;

    let lowered_position = mass_metric.lower(&position_next)?;
    let numerator = pairing(&lowered_position, &velocity_star);
    let denominator = pairing(&lowered_position, &position_next);
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(MetricErrorV1::NegativeMetricQuadraticForm);
    }
    let radial_velocity = numerator / denominator;
    let velocity_next = add_vectors(
        &velocity_star,
        &scale_vector(&position_next, -radial_velocity)?,
    )?;

    let position_multiplier = 0.5 * mass * (1.0 - scale.recip());
    let velocity_multiplier = -0.5 * mass * radial_velocity;
    let position_norm_sq = mass_metric.tangent_norm(&position_next)?.powi(2) / mass;
    let position_constraint_defect = position_norm_sq - radius * radius;
    let velocity_constraint_defect = 2.0 * pairing(&lowered_position, &velocity_next) / mass;

    let jacobian_components = [
        2.0 * lowered_position.components()[0] / mass,
        2.0 * lowered_position.components()[1] / mass,
    ];
    let delassus = DelassusMetricV1::<F, ConstraintSpace, 2, 1>::from_mass_jacobian(
        mass_metric,
        [jacobian_components],
        metric_profile,
    )?;
    let multiplier = ConstraintMultiplierV1::<ConstraintSpace, 1>::new([velocity_multiplier])?;
    let residual = ConstraintResidualV1::<ConstraintSpace, 1>::new([2.0 * numerator / mass])?;

    let delta_position = add_vectors(&position_next, &scale_vector(position, -1.0)?)?;
    let external_work = pairing(external_force, &delta_position);
    let kinetic_energy = 0.5 * pairing(&mass_metric.lower(&velocity_next)?, &velocity_next);
    let potential_energy = 0.5 * spring * radius * radius;
    let multiplier_metric_norm = delassus.multiplier_norm(&multiplier)?;
    let residual_dual_norm = delassus.residual_norm(&residual)?;

    let values = [
        position_multiplier,
        velocity_multiplier,
        position_constraint_defect,
        velocity_constraint_defect,
        external_work,
        kinetic_energy,
        potential_energy,
        multiplier_metric_norm,
        residual_dual_norm,
        delassus.matrix()[0][0],
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }

    Ok(StepOutcome {
        position: position_next,
        velocity: velocity_next,
        position_multiplier,
        velocity_multiplier,
        position_constraint_defect,
        velocity_constraint_defect,
        delassus: delassus.matrix()[0][0],
        multiplier_metric_norm,
        residual_dual_norm,
        external_work,
        kinetic_energy,
        potential_energy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use metric_induced_error_envelope_reference::{CartesianQ, DenseY, Rebasis};

    const PROFILE: MetricProfileV1 = MetricProfileV1 {
        relative_pivot_tolerance: 64.0 * f64::EPSILON,
        relative_symmetry_tolerance: 256.0 * f64::EPSILON,
    };
    const MASS: f64 = 2.0;
    const SPRING: f64 = 0.7;
    const TIMESTEP: f64 = 0.05;
    const RADIUS: f64 = 1.2;
    const STEP_PROFILE: CircleStepProfileV1 = CircleStepProfileV1 {
        mass: MASS,
        spring: SPRING,
        timestep: TIMESTEP,
        radius: RADIUS,
        metric_profile: PROFILE,
    };

    fn cartesian_mass() -> Metric<CartesianQ, 2> {
        Metric::admit([[MASS, 0.0], [0.0, MASS]], PROFILE).unwrap()
    }

    fn fixture_q() -> (
        Tangent<CartesianQ, 2>,
        Tangent<CartesianQ, 2>,
        Cotangent<CartesianQ, 2>,
    ) {
        (
            Tangent::new([1.0, 0.7]).unwrap(),
            Tangent::new([0.2, -0.35]).unwrap(),
            Cotangent::new([0.6, -0.4]).unwrap(),
        )
    }

    fn rotation(theta: f64) -> Matrix2 {
        [[theta.cos(), -theta.sin()], [theta.sin(), theta.cos()]]
    }

    fn reflection() -> Matrix2 {
        [[1.0, 0.0], [0.0, -1.0]]
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance * left.abs().max(right.abs()).max(1.0),
            "left={left:.16e}, right={right:.16e}, tolerance={tolerance:.3e}"
        );
    }

    fn close_vector<F: CoordinateFrame>(
        left: &Tangent<F, 2>,
        right: &Tangent<F, 2>,
        tolerance: f64,
    ) {
        close(left.components()[0], right.components()[0], tolerance);
        close(left.components()[1], right.components()[1], tolerance);
    }

    fn assert_step_action<F: CoordinateFrame>(
        mass_metric: &Metric<F, 2>,
        action: &Matrix2,
        position: &Tangent<F, 2>,
        velocity: &Tangent<F, 2>,
        force: &Cotangent<F, 2>,
    ) {
        let source =
            projected_circle_step(mass_metric, position, velocity, force, STEP_PROFILE).unwrap();
        let action_position = transform_tangent(action, position).unwrap();
        let action_velocity = transform_tangent(action, velocity).unwrap();
        let action_force = transform_covector(action, force).unwrap();
        let acted = projected_circle_step(
            mass_metric,
            &action_position,
            &action_velocity,
            &action_force,
            STEP_PROFILE,
        )
        .unwrap();

        close_vector(
            &acted.position,
            &transform_tangent(action, &source.position).unwrap(),
            4.0e-12,
        );
        close_vector(
            &acted.velocity,
            &transform_tangent(action, &source.velocity).unwrap(),
            4.0e-12,
        );
        close(
            acted.position_multiplier,
            source.position_multiplier,
            4.0e-12,
        );
        close(
            acted.velocity_multiplier,
            source.velocity_multiplier,
            4.0e-12,
        );
        close(
            acted.position_constraint_defect,
            source.position_constraint_defect,
            4.0e-12,
        );
        close(
            acted.velocity_constraint_defect,
            source.velocity_constraint_defect,
            4.0e-12,
        );
        close(acted.delassus, source.delassus, 4.0e-12);
        close(
            acted.multiplier_metric_norm,
            source.multiplier_metric_norm,
            4.0e-12,
        );
        close(acted.residual_dual_norm, source.residual_dual_norm, 4.0e-12);
        close(acted.external_work, source.external_work, 4.0e-12);
        close(acted.kinetic_energy, source.kinetic_energy, 4.0e-12);
        close(acted.potential_energy, source.potential_energy, 4.0e-12);
    }

    #[test]
    fn connected_rotation_commutes_with_the_actual_discrete_step() {
        let mass = cartesian_mass();
        let (position, velocity, force) = fixture_q();
        let action = rotation(0.61);
        assert_step_action(&mass, &action, &position, &velocity, &force);
    }

    #[test]
    fn disconnected_reflection_commutes_with_the_actual_discrete_step() {
        let mass = cartesian_mass();
        let (position, velocity, force) = fixture_q();
        let action = reflection();
        assert_step_action(&mass, &action, &position, &velocity, &force);
    }

    #[test]
    fn dense_rebasing_preserves_step_and_full_group_action() {
        let mass_q = cartesian_mass();
        let (position_q, velocity_q, force_q) = fixture_q();
        let rebasis_matrix = [[1.0, 0.0], [2.0, 1.0]];
        let rebasis = Rebasis::<CartesianQ, DenseY, 2>::new(rebasis_matrix, PROFILE).unwrap();
        let mass_y = rebasis.metric_to_to(&mass_q, PROFILE).unwrap();
        let position_y = rebasis.tangent_to_to(&position_q).unwrap();
        let velocity_y = rebasis.tangent_to_to(&velocity_q).unwrap();
        let force_y = rebasis.cotangent_to_to(&force_q).unwrap();

        let step_q =
            projected_circle_step(&mass_q, &position_q, &velocity_q, &force_q, STEP_PROFILE)
                .unwrap();
        let step_y =
            projected_circle_step(&mass_y, &position_y, &velocity_y, &force_y, STEP_PROFILE)
                .unwrap();

        close_vector(
            &step_q.position,
            &rebasis.tangent_to_from(&step_y.position).unwrap(),
            5.0e-12,
        );
        close_vector(
            &step_q.velocity,
            &rebasis.tangent_to_from(&step_y.velocity).unwrap(),
            5.0e-12,
        );
        close(
            step_q.position_multiplier,
            step_y.position_multiplier,
            5.0e-12,
        );
        close(
            step_q.velocity_multiplier,
            step_y.velocity_multiplier,
            5.0e-12,
        );
        close(step_q.delassus, step_y.delassus, 5.0e-12);
        close(step_q.external_work, step_y.external_work, 5.0e-12);

        for action_q in [rotation(0.61), reflection()] {
            let action_y = conjugate_action(&rebasis_matrix, &action_q).unwrap();
            let isometry = multiply(&multiply(&transpose(&action_y), mass_y.matrix()), &action_y);
            for (actual_row, expected_row) in isometry.iter().zip(mass_y.matrix().iter()) {
                for (actual, expected) in actual_row.iter().zip(expected_row.iter()) {
                    close(*actual, *expected, 5.0e-12);
                }
            }
            assert_step_action(&mass_y, &action_y, &position_y, &velocity_y, &force_y);
        }

        // The dense representation is not Euclidean-orthogonal; the transformed
        // mass metric, rather than the Euclidean component norm, is the invariant.
        let reflection_y = conjugate_action(&rebasis_matrix, &reflection()).unwrap();
        let euclidean_defect = multiply(&transpose(&reflection_y), &reflection_y);
        assert!(
            (euclidean_defect[0][1]).abs() > 1.0e-3
                || (euclidean_defect[1][0]).abs() > 1.0e-3
                || (euclidean_defect[0][0] - 1.0).abs() > 1.0e-3
        );
    }

    #[test]
    fn mass_metric_rejects_nonpositive_and_nonfinite_geometry() {
        assert!(Metric::<CartesianQ, 2>::admit([[1.0, 0.0], [0.0, 0.0]], PROFILE).is_err());
        assert!(Metric::<CartesianQ, 2>::admit([[f64::NAN, 0.0], [0.0, 1.0]], PROFILE).is_err());
    }
}
