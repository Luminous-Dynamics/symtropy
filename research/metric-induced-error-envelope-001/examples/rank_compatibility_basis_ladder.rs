use metric_induced_error_envelope_reference::{
    CartesianQ, ConstraintQuotientProfileV1, ConstraintResidualStatusV1, ConstraintResidualV1,
    CoordinateFrame, Metric, MetricProfileV1, RankDeficientDelassusQuotientV1, Rebasis, Tangent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OldConstraintSpace;
impl CoordinateFrame for OldConstraintSpace {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NewConstraintSpace;
impl CoordinateFrame for NewConstraintSpace {}

const T: [[f64; 3]; 3] = [[1.0, 8.0, 0.0], [0.0, 1.0, 4.0], [0.0, 0.0, 1.0]];
const S: [[f64; 3]; 3] = [[1.0, -8.0, 32.0], [0.0, 1.0, -4.0], [0.0, 0.0, 1.0]];

fn metric_profile() -> MetricProfileV1 {
    MetricProfileV1::default()
}

fn quotient_profile() -> ConstraintQuotientProfileV1 {
    ConstraintQuotientProfileV1 {
        relative_rank_tolerance: 1.0e-10,
        rank_uncertainty_factor: 10.0,
        relative_compatibility_tolerance: 1.0e-10,
        relative_reduced_pivot_tolerance: 1.0e-12,
    }
}

fn multiply(left: &[[f64; 3]; 3], right: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..3)
                .map(|inner| left[row][inner] * right[inner][column])
                .sum()
        })
    })
}

fn transform_jacobian(jacobian: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    multiply(&T, jacobian)
}

fn apply(jacobian: &[[f64; 3]; 3], vector: &[f64; 3]) -> [f64; 3] {
    std::array::from_fn(|row| {
        (0..3)
            .map(|column| jacobian[row][column] * vector[column])
            .sum()
    })
}

fn close(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance * left.abs().max(right.abs()).max(1.0)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mp = metric_profile();
    let qp = quotient_profile();
    let mass = Metric::<CartesianQ, 3>::admit(
        [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
        mp,
    )?;
    let h_old = Metric::<OldConstraintSpace, 3>::admit(
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        mp,
    )?;
    let row_rebasis = Rebasis::<OldConstraintSpace, NewConstraintSpace, 3>::new(S, mp)?;
    let h_new = row_rebasis.metric_to_to(&h_old, mp)?;

    // A row-space shear must be paired with H_new = T^-T H_old T^-1.
    // Since S = T^-1, metric_to_to computes S^T H_old S.
    let mass_base = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
    let x = [0.1, -0.2, 0.3];

    println!("epsilon,old_rank,new_rank,old_status,new_status,old_compatibility_defect,new_compatibility_defect,dual_norm_relative_drift,correction_relative_drift");
    for epsilon in [1.0e-8, 1.0e-9, 1.0e-10, 1.0e-11, 1.0e-12, 0.0] {
        let j_old = [
            mass_base[0],
            mass_base[1],
            [1.0, epsilon, 1.0],
        ];
        let j_new = transform_jacobian(&j_old);
        let residual_old = apply(&j_old, &x);
        let residual_new = row_rebasis
            .tangent_to_to(&Tangent::<OldConstraintSpace, 3>::new(residual_old)?)?;

        let old_quotient = RankDeficientDelassusQuotientV1::<
            CartesianQ,
            OldConstraintSpace,
            3,
            3,
        >::new(&mass, j_old, &h_old, qp)?;
        let new_quotient = RankDeficientDelassusQuotientV1::<
            CartesianQ,
            NewConstraintSpace,
            3,
            3,
        >::new(&mass, j_new, &h_new, qp)?;
        let old_result = old_quotient.solve_residual(
            &ConstraintResidualV1::<OldConstraintSpace, 3>::new(residual_old)?,
        )?;
        let new_result = new_quotient.solve_residual(
            &ConstraintResidualV1::<NewConstraintSpace, 3>::new(*residual_new.components())?,
        )?;

        if old_result.status() != new_result.status()
            || old_result.numerical_rank() != new_result.numerical_rank()
            || old_result.multiplier_nullity() != new_result.multiplier_nullity()
        {
            return Err(format!("rank/status changed under row re-basing at epsilon={epsilon:e}").into());
        }

        let compatibility_drift = (old_result.compatibility_relative_defect()
            - new_result.compatibility_relative_defect())
        .abs();
        let (dual_norm_drift, correction_drift) = match (
            old_result.residual_dual_norm(),
            new_result.residual_dual_norm(),
            old_result.minimum_kinetic_correction(),
            new_result.minimum_kinetic_correction(),
        ) {
            (Some(old_norm), Some(new_norm), Some(old_correction), Some(new_correction)) => {
                let dual_drift = (old_norm - new_norm).abs() / old_norm.abs().max(1.0);
                let correction_drift = (0..3)
                    .map(|i| {
                        (old_correction.components()[i] - new_correction.components()[i]).abs()
                    })
                    .fold(0.0_f64, f64::max);
                (dual_drift, correction_drift)
            }
            (None, None, None, None) => (f64::NAN, f64::NAN),
            _ => {
                return Err(format!("authority mismatch under row re-basing at epsilon={epsilon:e}").into());
            }
        };

        println!(
            "{epsilon:.1e},{},{},{:?},{:?},{compatibility_old:.8e},{compatibility_new:.8e},{dual_norm_drift:.8e},{correction_drift:.8e}",
            old_result.numerical_rank(),
            new_result.numerical_rank(),
            old_result.status(),
            new_result.status(),
            compatibility_old = old_result.compatibility_relative_defect(),
            compatibility_new = new_result.compatibility_relative_defect(),
        );

        if !close(
            old_result.compatibility_relative_defect(),
            new_result.compatibility_relative_defect(),
            2.0e-8,
        ) || compatibility_drift > 2.0e-8
            || (dual_norm_drift.is_finite() && dual_norm_drift > 2.0e-8)
            || (correction_drift.is_finite() && correction_drift > 2.0e-8)
        {
            return Err(format!("invariant drift outside the declared envelope at epsilon={epsilon:e}").into());
        }
    }

    // Exact duplicate row: z=(1,0,-1) lies in ker(J^T), so adding it to a
    // realizable residual creates an incompatible right-hand side.
    let duplicate_j = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0], [1.0, 0.0, 1.0]];
    let base = apply(&duplicate_j, &x);
    let incompatible_old = [base[0] + 0.1, base[1], base[2] - 0.1];
    let incompatible_new = row_rebasis
        .tangent_to_to(&Tangent::<OldConstraintSpace, 3>::new(incompatible_old)?)?;
    let old_quotient = RankDeficientDelassusQuotientV1::<
        CartesianQ,
        OldConstraintSpace,
        3,
        3,
    >::new(&mass, duplicate_j, &h_old, qp)?;
    let new_quotient = RankDeficientDelassusQuotientV1::<
        CartesianQ,
        NewConstraintSpace,
        3,
        3,
    >::new(&mass, transform_jacobian(&duplicate_j), &h_new, qp)?;
    let old_result = old_quotient.solve_residual(
        &ConstraintResidualV1::<OldConstraintSpace, 3>::new(incompatible_old)?,
    )?;
    let new_result = new_quotient.solve_residual(
        &ConstraintResidualV1::<NewConstraintSpace, 3>::new(*incompatible_new.components())?,
    )?;
    if old_result.status() != ConstraintResidualStatusV1::Incompatible
        || new_result.status() != ConstraintResidualStatusV1::Incompatible
        || old_result.minimum_kinetic_correction().is_some()
        || new_result.minimum_kinetic_correction().is_some()
    {
        return Err("incompatible residual was promoted under a row-basis change".into());
    }
    println!("exact_incompatible_control,old={:?},new={:?},correction_authorized=false", old_result.status(), new_result.status());
    Ok(())
}
