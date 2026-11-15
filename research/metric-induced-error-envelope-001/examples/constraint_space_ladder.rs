use metric_induced_error_envelope_reference::{
    CoordinateFrame, Cotangent, Metric, MetricProfileV1, Rebasis, Tangent,
    constraint_space_receipt::{
        ConstraintMapRankStatusV1, ConstraintSpaceProfileV1, ConstraintSpaceReceiptV1,
        rebase_constraint_jacobian_both,
    },
    operator::OperatorConditionProfileV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CartesianQ;
impl CoordinateFrame for CartesianQ {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DenseY;
impl CoordinateFrame for DenseY {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConstraintSpace;
impl CoordinateFrame for ConstraintSpace {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DenseConstraint;
impl CoordinateFrame for DenseConstraint {}

fn metric_profile() -> MetricProfileV1 {
    MetricProfileV1 {
        relative_pivot_tolerance: 1.0e-13,
        relative_symmetry_tolerance: 1.0e-12,
    }
}

fn constraint_profile() -> ConstraintSpaceProfileV1 {
    ConstraintSpaceProfileV1 {
        operator: OperatorConditionProfileV1 {
            relative_jacobi_tolerance: 1.0e-13,
            relative_rank_tolerance: 1.0e-12,
            maximum_condition_estimate: 1.0e10,
            maximum_jacobi_sweeps: 64,
        },
        maximum_matrix_entries: 128,
        maximum_work_units: 100_000,
    }
}

fn max_abs_matrix_difference<const R: usize, const C: usize>(
    left: &[[f64; C]; R],
    right: &[[f64; C]; R],
) -> f64 {
    let mut maximum = 0.0_f64;
    for row in 0..R {
        for column in 0..C {
            maximum = maximum.max((left[row][column] - right[row][column]).abs());
        }
    }
    maximum
}

fn transpose<const N: usize>(matrix: &[[f64; N]; N]) -> [[f64; N]; N] {
    let mut result = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..N {
            result[row][column] = matrix[column][row];
        }
    }
    result
}

fn multiply<const R: usize, const K: usize, const C: usize>(
    left: &[[f64; K]; R],
    right: &[[f64; C]; K],
) -> [[f64; C]; R] {
    let mut result = [[0.0; C]; R];
    for row in 0..R {
        for column in 0..C {
            for inner in 0..K {
                result[row][column] += left[row][inner] * right[inner][column];
            }
        }
    }
    result
}

fn inverse_from_rebasis<From: CoordinateFrame, To: CoordinateFrame, const N: usize>(
    rebasis: &Rebasis<From, To, N>,
) -> Result<[[f64; N]; N], Box<dyn std::error::Error>> {
    let mut inverse = [[0.0; N]; N];
    for column in 0..N {
        let mut unit = [0.0; N];
        unit[column] = 1.0;
        let vector = Tangent::<From, N>::new(unit)?;
        let transformed = rebasis.tangent_to_to(&vector)?;
        for row in 0..N {
            inverse[row][column] = transformed.components()[row];
        }
    }
    Ok(inverse)
}

fn assert_close(
    left: f64,
    right: f64,
    tolerance: f64,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let scale = 1.0_f64.max(left.abs()).max(right.abs());
    let defect = (left - right).abs();
    if defect > tolerance * scale {
        return Err(
            format!("{label}: left={left:.17e}, right={right:.17e}, defect={defect:.3e}").into(),
        );
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = metric_profile();
    let solver_profile = constraint_profile();
    let mass_q = Metric::<CartesianQ, 3>::admit(
        [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
        profile,
    )?;
    let constraint_c = Metric::<ConstraintSpace, 2>::admit([[2.0, 0.25], [0.25, 1.5]], profile)?;
    let jacobian_q = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
    let receipt_q = ConstraintSpaceReceiptV1::<CartesianQ, ConstraintSpace, 3, 2>::new(
        mass_q,
        constraint_c,
        jacobian_q,
        profile,
        solver_profile,
    )?;
    assert_eq!(
        receipt_q.rank_status(),
        ConstraintMapRankStatusV1::FullRowRank
    );

    let velocity_q = Tangent::<CartesianQ, 3>::new([0.2, -0.3, 0.8])?;
    let multiplier_q = Cotangent::<ConstraintSpace, 2>::new([0.75, -0.2])?;
    let target_rate_q = Tangent::<ConstraintSpace, 2>::new([0.4, -0.2])?;
    let (solved_multiplier_q, solved_correction_q) =
        receipt_q.solve_minimum_kinetic_correction(&target_rate_q)?;

    println!(
        "shear,domain_condition_inf,constraint_condition_inf,singular_spectrum_max_delta,whitened_delassus_eigen_max_delta,delassus_congruence_max_defect,constraint_rate_norm_delta,constraint_covector_dual_norm_delta,residual_dual_norm_delta,minimum_kinetic_correction_max_delta"
    );

    for shear in [0.0, 1.0, 2.0, 4.0, 8.0] {
        let domain_rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
            [
                [1.0, shear, 0.5 * shear * shear],
                [0.0, 1.0, shear],
                [0.0, 0.0, 1.0],
            ],
            profile,
        )?;
        let constraint_shear = 0.5 * shear;
        let constraint_rebasis = Rebasis::<ConstraintSpace, DenseConstraint, 2>::new(
            [[1.0, constraint_shear], [0.0, 1.0]],
            profile,
        )?;

        let mass_y = domain_rebasis.metric_to_to(&mass_q, profile)?;
        let constraint_d = constraint_rebasis.metric_to_to(&constraint_c, profile)?;
        let jacobian_d =
            rebase_constraint_jacobian_both(&jacobian_q, &domain_rebasis, &constraint_rebasis)?;
        let receipt_d = ConstraintSpaceReceiptV1::<DenseY, DenseConstraint, 3, 2>::new(
            mass_y,
            constraint_d,
            jacobian_d,
            profile,
            solver_profile,
        )?;
        if receipt_d.rank_status() != ConstraintMapRankStatusV1::FullRowRank {
            return Err(format!("rank changed at shear {shear}").into());
        }

        let mut singular_delta = 0.0_f64;
        for (left, right) in receipt_q
            .singular_values()
            .iter()
            .zip(receipt_d.singular_values().iter())
        {
            singular_delta = singular_delta.max((left - right).abs());
            assert_close(*left, *right, 2.0e-9, "rectangular singular spectrum")?;
        }

        let mut eigen_delta = 0.0_f64;
        for (left, right) in receipt_q
            .delassus_whitened_eigenvalue_estimates()
            .iter()
            .zip(receipt_d.delassus_whitened_eigenvalue_estimates().iter())
        {
            eigen_delta = eigen_delta.max((left - right).abs());
            assert_close(*left, *right, 5.0e-9, "whitened Delassus eigen spectrum")?;
        }

        let delassus_q = receipt_q.delassus().ok_or("canonical Delassus missing")?;
        let delassus_d = receipt_d.delassus().ok_or("rebased Delassus missing")?;
        let inverse_t = inverse_from_rebasis(&constraint_rebasis)?;
        let expected_w_d = multiply(
            &multiply(&inverse_t, delassus_q.matrix()),
            &transpose(&inverse_t),
        );
        let delassus_defect = max_abs_matrix_difference(delassus_d.matrix(), &expected_w_d);
        if delassus_defect > 5.0e-9 {
            return Err(format!(
                "Delassus congruence failed at shear {shear}: {delassus_defect:e}"
            )
            .into());
        }

        let velocity_y = domain_rebasis.tangent_to_to(&velocity_q)?;
        let rate_q = receipt_q.apply_jacobian(&velocity_q)?;
        let rate_d = receipt_d.apply_jacobian(&velocity_y)?;
        let expected_rate_d = constraint_rebasis.tangent_to_to(&rate_q)?;
        for (actual, expected) in rate_d
            .components()
            .iter()
            .zip(expected_rate_d.components().iter())
        {
            assert_close(
                *actual,
                *expected,
                2.0e-10,
                "constraint-rate coordinate map",
            )?;
        }
        let rate_norm_q = receipt_q.constraint_rate_norm(&rate_q)?;
        let rate_norm_d = receipt_d.constraint_rate_norm(&rate_d)?;
        assert_close(
            rate_norm_q,
            rate_norm_d,
            2.0e-9,
            "constraint-rate metric norm",
        )?;

        let multiplier_d = constraint_rebasis.cotangent_to_to(&multiplier_q)?;
        let covector_norm_q = receipt_q.constraint_covector_dual_norm(&multiplier_q)?;
        let covector_norm_d = receipt_d.constraint_covector_dual_norm(&multiplier_d)?;
        assert_close(
            covector_norm_q,
            covector_norm_d,
            2.0e-9,
            "constraint covector dual norm",
        )?;
        assert_close(
            metric_induced_error_envelope_reference::pairing(&multiplier_q, &rate_q),
            metric_induced_error_envelope_reference::pairing(&multiplier_d, &rate_d),
            2.0e-10,
            "constraint virtual-work pairing",
        )?;

        let target_rate_d = constraint_rebasis.tangent_to_to(&target_rate_q)?;
        let residual_norm_q = receipt_q.residual_delassus_dual_norm(&target_rate_q)?;
        let residual_norm_d = receipt_d.residual_delassus_dual_norm(&target_rate_d)?;
        assert_close(
            residual_norm_q,
            residual_norm_d,
            2.0e-9,
            "Delassus residual dual norm",
        )?;
        let (solved_multiplier_d, solved_correction_d) =
            receipt_d.solve_minimum_kinetic_correction(&target_rate_d)?;
        let expected_multiplier_d = constraint_rebasis.cotangent_to_to(&solved_multiplier_q)?;
        let correction_q_from_d = domain_rebasis.tangent_to_from(&solved_correction_d)?;

        for (actual, expected) in solved_multiplier_d
            .components()
            .iter()
            .zip(expected_multiplier_d.components().iter())
        {
            assert_close(*actual, *expected, 2.0e-9, "minimum-kinetic multiplier")?;
        }
        let mut correction_delta = 0.0_f64;
        for (actual, expected) in solved_correction_q
            .components()
            .iter()
            .zip(correction_q_from_d.components().iter())
        {
            correction_delta = correction_delta.max((actual - expected).abs());
            assert_close(*actual, *expected, 2.0e-9, "lifted physical correction")?;
        }

        println!(
            "{shear:.1},{:.8e},{:.8e},{singular_delta:.8e},{eigen_delta:.8e},{delassus_defect:.8e},{:.8e},{:.8e},{:.8e},{correction_delta:.8e}",
            domain_rebasis.condition_inf_estimate(),
            constraint_rebasis.condition_inf_estimate(),
            (rate_norm_q - rate_norm_d).abs(),
            (covector_norm_q - covector_norm_d).abs(),
            (residual_norm_q - residual_norm_d).abs()
        );
    }
    Ok(())
}
