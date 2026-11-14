use metric_induced_error_envelope_reference::{
    CartesianQ, ConstraintQuotientProfileV1, ConstraintResidualV1, CoordinateFrame, Metric,
    MetricProfileV1, RankDeficientDelassusQuotientV1, Tangent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConstraintRows;
impl CoordinateFrame for ConstraintRows {}

fn solve3(mut matrix: [[f64; 3]; 3], mut rhs: [f64; 3]) -> [f64; 3] {
    for column in 0..3 {
        let pivot = (column..3)
            .max_by(|left, right| {
                matrix[*left][column]
                    .abs()
                    .total_cmp(&matrix[*right][column].abs())
            })
            .unwrap();
        matrix.swap(column, pivot);
        rhs.swap(column, pivot);
        let diagonal = matrix[column][column];
        for entry in column..3 {
            matrix[column][entry] /= diagonal;
        }
        rhs[column] /= diagonal;
        for row in 0..3 {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            for entry in column..3 {
                matrix[row][entry] -= factor * matrix[column][entry];
            }
            rhs[row] -= factor * rhs[column];
        }
    }
    rhs
}

fn symmetric_eigenvalues_3(mut matrix: [[f64; 3]; 3]) -> [f64; 3] {
    for _ in 0..64 {
        let mut p = 0usize;
        let mut q = 1usize;
        let mut largest = matrix[0][1].abs();
        for (row, column) in [(0, 2), (1, 2)] {
            if matrix[row][column].abs() > largest {
                p = row;
                q = column;
                largest = matrix[row][column].abs();
            }
        }
        if largest
            <= 32.0 * f64::EPSILON * matrix.iter().flatten().map(|x| x.abs()).fold(0.0, f64::max)
        {
            break;
        }

        let apq = matrix[p][q];
        let tau = (matrix[q][q] - matrix[p][p]) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        let app = matrix[p][p];
        let aqq = matrix[q][q];
        matrix[p][p] = app - t * apq;
        matrix[q][q] = aqq + t * apq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;
        for k in 0..3 {
            if k == p || k == q {
                continue;
            }
            let akp = matrix[k][p];
            let akq = matrix[k][q];
            matrix[k][p] = c * akp - s * akq;
            matrix[p][k] = matrix[k][p];
            matrix[k][q] = s * akp + c * akq;
            matrix[q][k] = matrix[k][q];
        }
    }
    let mut eigenvalues = [matrix[0][0], matrix[1][1], matrix[2][2]];
    eigenvalues.sort_by(f64::total_cmp);
    eigenvalues
}

fn mat_vec(matrix: &[[f64; 3]; 3], vector: &[f64; 3]) -> [f64; 3] {
    std::array::from_fn(|row| {
        (0..3)
            .map(|column| matrix[row][column] * vector[column])
            .sum()
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metric_profile = MetricProfileV1::default();
    let quotient_profile = ConstraintQuotientProfileV1 {
        relative_rank_tolerance: 1.0e-10,
        relative_compatibility_tolerance: 1.0e-9,
        relative_reduced_pivot_tolerance: 1.0e-12,
    };
    let mass = Metric::<CartesianQ, 3>::admit(
        [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
        metric_profile,
    )?;
    let residual_metric = Metric::<ConstraintRows, 3>::admit(
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        metric_profile,
    )?;
    let jacobian = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0], [1.0, 0.0, 1.0]];
    let residual = [0.4, -0.7, 0.4];
    let quotient = RankDeficientDelassusQuotientV1::<CartesianQ, ConstraintRows, 3, 3>::new(
        &mass,
        jacobian,
        &residual_metric,
        quotient_profile,
    )?;
    let hard =
        quotient.solve_residual(&ConstraintResidualV1::<ConstraintRows, 3>::new(residual)?)?;
    let hard_correction = hard
        .minimum_kinetic_correction()
        .ok_or("hard quotient solve did not return a correction")?;
    let hard_norm = mass.tangent_norm(hard_correction)?;

    let eigenvalues = symmetric_eigenvalues_3(*quotient.matrix());
    let largest = eigenvalues[2];
    let smallest = eigenvalues[0].max(0.0);
    let reduced_condition = eigenvalues[2] / eigenvalues[1];

    println!(
        "method,epsilon,rank,condition_scope,condition_estimate,physical_residual_norm,relative_kinetic_correction_error"
    );
    println!(
        "rank_quotient,0,{},quotient_restriction,{:.8e},{:.8e},{:.8e}",
        quotient.rank(),
        reduced_condition,
        0.0,
        0.0
    );

    for epsilon in [1.0e-2, 1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 1.0e-8, 1.0e-10] {
        let regularized = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                quotient.matrix()[row][column] + if row == column { epsilon } else { 0.0 }
            })
        });
        let condition = (largest + epsilon) / (smallest + epsilon);
        let multiplier = solve3(regularized, residual);
        let generalized_force: [f64; 3] =
            std::array::from_fn(|dof| (0..3).map(|row| jacobian[row][dof] * multiplier[row]).sum());
        let correction = mass.raise(&metric_induced_error_envelope_reference::Cotangent::<
            CartesianQ,
            3,
        >::new(generalized_force)?)?;
        let reconstructed = mat_vec(&jacobian, correction.components());
        let residual_defect: [f64; 3] =
            std::array::from_fn(|row| reconstructed[row] - residual[row]);
        let residual_norm = residual_defect
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        let correction_error = Tangent::<CartesianQ, 3>::new(std::array::from_fn(|index| {
            correction.components()[index] - hard_correction.components()[index]
        }))?;
        let relative_kinetic_error =
            mass.tangent_norm(&correction_error)? / hard_norm.max(1.0e-300);
        println!(
            "diagonal_regularization,{epsilon:.1e},{},full_regularized_matrix,{condition:.8e},{residual_norm:.8e},{relative_kinetic_error:.8e}",
            quotient.rank()
        );
    }
    Ok(())
}
