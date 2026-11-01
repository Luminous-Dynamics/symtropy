//! Exact geometry-derived rank-deficient Delassus fixture.
//!
//! Two point contacts on one rigid body against a static plane have a
//! one-dimensional unobservable relative twist: rotation about the line joining
//! the contacts. The corresponding Delassus matrix is PSD, not SPD. This test
//! certifies the physical Jacobian, nullspace, impulse equivalence family and
//! a near-rank-deficient regularization entirely over exact rationals.

#[path = "../src/coupled_friction_sturm_support.rs"]
mod exact_support;

use exact_support::Rational;
use num_bigint::BigInt;
use num_traits::Zero;

type Matrix = Vec<Vec<Rational>>;
type Vector = Vec<Rational>;

fn r(numerator: i64, denominator: i64) -> Rational {
    Rational::new(BigInt::from(numerator), BigInt::from(denominator))
}

fn z() -> Rational {
    Rational::zero()
}

fn i(value: i64) -> Rational {
    Rational::from_integer(BigInt::from(value))
}

fn transpose(matrix: &Matrix) -> Matrix {
    (0..matrix[0].len())
        .map(|column| matrix.iter().map(|row| row[column].clone()).collect())
        .collect()
}

fn multiply(lhs: &Matrix, rhs: &Matrix) -> Matrix {
    assert_eq!(lhs[0].len(), rhs.len());
    (0..lhs.len())
        .map(|row| {
            (0..rhs[0].len())
                .map(|column| {
                    (0..rhs.len()).fold(z(), |sum, inner| {
                        sum + lhs[row][inner].clone() * rhs[inner][column].clone()
                    })
                })
                .collect()
        })
        .collect()
}

fn mat_vec(matrix: &Matrix, vector: &Vector) -> Vector {
    matrix
        .iter()
        .map(|row| {
            row.iter()
                .zip(vector)
                .fold(z(), |sum, (left, right)| sum + left.clone() * right.clone())
        })
        .collect()
}

fn dot(lhs: &Vector, rhs: &Vector) -> Rational {
    lhs.iter()
        .zip(rhs)
        .fold(z(), |sum, (left, right)| sum + left.clone() * right.clone())
}

fn rank(matrix: &Matrix) -> usize {
    let mut reduced = matrix.clone();
    let rows = reduced.len();
    let columns = reduced[0].len();
    let mut pivot_row = 0;

    for column in 0..columns {
        let Some(pivot) = (pivot_row..rows).find(|&row| !reduced[row][column].is_zero()) else {
            continue;
        };
        reduced.swap(pivot_row, pivot);

        let pivot_value = reduced[pivot_row][column].clone();
        for entry in column..columns {
            reduced[pivot_row][entry] = reduced[pivot_row][entry].clone() / pivot_value.clone();
        }

        for row in 0..rows {
            if row == pivot_row {
                continue;
            }
            let factor = reduced[row][column].clone();
            if factor.is_zero() {
                continue;
            }
            for entry in column..columns {
                reduced[row][entry] = reduced[row][entry].clone()
                    - factor.clone() * reduced[pivot_row][entry].clone();
            }
        }
        pivot_row += 1;
        if pivot_row == rows {
            break;
        }
    }
    pivot_row
}

fn determinant(matrix: &Matrix) -> Rational {
    assert_eq!(matrix.len(), matrix[0].len());
    let mut work = matrix.clone();
    let size = work.len();
    let mut result = i(1);

    for column in 0..size {
        let Some(pivot) = (column..size).find(|&row| !work[row][column].is_zero()) else {
            return z();
        };
        if pivot != column {
            work.swap(pivot, column);
            result = -result;
        }
        let pivot_value = work[column][column].clone();
        result *= pivot_value.clone();

        for row in (column + 1)..size {
            let factor = work[row][column].clone() / pivot_value.clone();
            for entry in (column + 1)..size {
                work[row][entry] =
                    work[row][entry].clone() - factor.clone() * work[column][entry].clone();
            }
        }
    }
    result
}

fn leading_principal_minor(matrix: &Matrix, size: usize) -> Rational {
    determinant(
        &matrix
            .iter()
            .take(size)
            .map(|row| row.iter().take(size).cloned().collect())
            .collect(),
    )
}

fn geometry_fixture() -> (Matrix, Matrix, Matrix, Vector, Vector, Vector) {
    // Generalized coordinates: (v_x, v_y, v_z, omega_x, omega_y, omega_z).
    // At r_2=(0,1,0), omega x r_2=(-omega_z, 0, omega_x).
    let jacobian = vec![
        vec![i(1), z(), z(), z(), z(), z()],
        vec![z(), i(1), z(), z(), z(), z()],
        vec![z(), z(), i(1), z(), z(), z()],
        vec![i(1), z(), z(), z(), z(), i(-1)],
        vec![z(), i(1), z(), z(), z(), z()],
        vec![z(), z(), i(1), i(1), z(), z()],
    ];

    let inverse_mass = vec![
        vec![i(1), z(), z(), z(), z(), z()],
        vec![z(), i(1), z(), z(), z(), z()],
        vec![z(), z(), i(1), z(), z(), z()],
        vec![z(), z(), z(), i(1), z(), z()],
        vec![z(), z(), z(), z(), r(1, 2), z()],
        vec![z(), z(), z(), z(), z(), r(1, 3)],
    ];

    let expected_delassus = vec![
        vec![i(1), z(), z(), i(1), z(), z()],
        vec![z(), i(1), z(), z(), i(1), z()],
        vec![z(), z(), i(1), z(), z(), i(1)],
        vec![i(1), z(), z(), r(4, 3), z(), z()],
        vec![z(), i(1), z(), z(), i(1), z()],
        vec![z(), z(), i(1), z(), z(), i(2)],
    ];

    let delassus = multiply(&multiply(&jacobian, &inverse_mass), &transpose(&jacobian));
    assert_eq!(delassus, expected_delassus);

    let null_impulse = vec![z(), i(1), z(), z(), i(-1), z()];
    let base_impulse = vec![i(2), z(), z(), i(2), z(), z()];
    let free_velocity = mat_vec(&delassus, &base_impulse)
        .into_iter()
        .map(std::ops::Neg::neg)
        .collect();

    (
        jacobian,
        inverse_mass,
        delassus,
        null_impulse,
        base_impulse,
        free_velocity,
    )
}

#[test]
fn physical_two_point_delassus_is_psd_rank_five_with_a_one_dimensional_kernel() {
    let (jacobian, inverse_mass, delassus, null_impulse, _, _) = geometry_fixture();

    assert_eq!(rank(&jacobian), 5);
    assert_eq!(rank(&delassus), 5);
    assert_eq!(
        mat_vec(&delassus, &null_impulse),
        vec![z(), z(), z(), z(), z(), z()]
    );

    // Exact Sylvester tests are not applicable to a singular PSD matrix.
    // Instead, verify symmetry and reconstruct W=J M^{-1} J^T. The quadratic
    // form is x^T J M^{-1} J^T x = ||M^{-1/2} J^T x||^2 >= 0 because M^{-1}
    // is an exactly positive diagonal matrix.
    assert_eq!(delassus, transpose(&delassus));
    assert!(inverse_mass.iter().enumerate().all(|(index, row)| {
        row[index] > Rational::zero()
            && row
                .iter()
                .enumerate()
                .all(|(column, value)| index == column || value.is_zero())
    }));
}

#[test]
fn rank_deficiency_creates_a_continuum_of_admissible_equivalent_impulses() {
    let (_, _, delassus, null_impulse, base_impulse, free_velocity) = geometry_fixture();
    let mu = r(1, 2);
    let base_response = mat_vec(&delassus, &base_impulse);
    let expected_energy = r(-26, 3);

    assert_eq!(free_velocity, vec![i(-4), z(), z(), r(-14, 3), z(), z()]);
    assert_eq!(dot(&free_velocity, &null_impulse), z());

    // For every rational alpha in (-1,1), p(alpha)=p0+alpha*n has the same
    // body response and is strictly inside both Coulomb disks.
    let samples = [r(-9, 10), r(-1, 2), z(), r(1, 3), r(9, 10)];
    let mut allocations = Vec::new();

    for alpha in samples {
        assert!(alpha > i(-1) && alpha < i(1));
        let impulse = base_impulse
            .iter()
            .zip(&null_impulse)
            .map(|(base, null)| base.clone() + alpha.clone() * null.clone())
            .collect::<Vector>();

        let response = mat_vec(&delassus, &impulse);
        assert_eq!(response, base_response);
        assert_eq!(
            response
                .iter()
                .zip(&free_velocity)
                .map(|(left, right)| left.clone() + right.clone())
                .collect::<Vector>(),
            vec![z(), z(), z(), z(), z(), z()]
        );

        let normal_1 = impulse[0].clone();
        let normal_2 = impulse[3].clone();
        let radius_1 = mu.clone() * normal_1;
        let radius_2 = mu.clone() * normal_2;
        let tangent_1_squared =
            impulse[1].clone() * impulse[1].clone() + impulse[2].clone() * impulse[2].clone();
        let tangent_2_squared =
            impulse[4].clone() * impulse[4].clone() + impulse[5].clone() * impulse[5].clone();
        assert!(normal_1 > Rational::zero() && normal_2 > Rational::zero());
        assert!(radius_1.clone() * radius_1 > tangent_1_squared);
        assert!(radius_2.clone() * radius_2 > tangent_2_squared);

        let energy = dot(&impulse, &free_velocity) + r(1, 2) * dot(&impulse, &response);
        assert_eq!(energy, expected_energy);
        allocations.push(impulse);
    }

    for pair in allocations.windows(2) {
        assert_ne!(pair[0], pair[1]);
    }
}

#[test]
fn arbitrarily_small_exact_nullspace_lift_removes_the_impulse_continuum() {
    let (_, _, delassus, null_impulse, base_impulse, free_velocity) = geometry_fixture();
    let epsilon = r(1, 10_000);
    let mut lifted = delassus.clone();

    for row in 0..6 {
        for column in 0..6 {
            lifted[row][column] +=
                epsilon.clone() * null_impulse[row].clone() * null_impulse[column].clone();
        }
    }

    assert_eq!(rank(&delassus), 5);
    assert_eq!(rank(&lifted), 6);
    for size in 1..=6 {
        assert!(
            leading_principal_minor(&lifted, size) > Rational::zero(),
            "the exact nullspace lift must be SPD by Sylvester's criterion"
        );
    }

    // Since n^T p0 = 0 and n^T n = 2, W_epsilon p(alpha) differs
    // from W_epsilon p0 by 2*epsilon*alpha*n. Only alpha=0 retains
    // the same post-contact response throughout this interior-disk family.
    let alpha = r(1, 3);
    let impulse = base_impulse
        .iter()
        .zip(&null_impulse)
        .map(|(base, null)| base.clone() + alpha.clone() * null.clone())
        .collect::<Vector>();
    let base_response = mat_vec(&lifted, &base_impulse);
    let perturbed_response = mat_vec(&lifted, &impulse);
    let difference = perturbed_response
        .iter()
        .zip(&base_response)
        .map(|(left, right)| left.clone() - right.clone())
        .collect::<Vector>();
    let expected_difference = null_impulse
        .iter()
        .map(|value| r(2, 1) * epsilon.clone() * alpha.clone() * value.clone())
        .collect::<Vector>();
    assert_eq!(difference, expected_difference);
    assert_ne!(
        mat_vec(&lifted, &impulse)
            .iter()
            .zip(&free_velocity)
            .map(|(left, right)| left.clone() + right.clone())
            .collect::<Vector>(),
        vec![z(), z(), z(), z(), z(), z()]
    );
}

#[test]
fn one_null_direction_means_unique_response_but_nonunique_force_allocation() {
    let (_, _, delassus, null_impulse, base_impulse, _) = geometry_fixture();
    assert_eq!(rank(&delassus), 5);

    // The quotient response map [p] -> Wp is injective on R^6 / ker(W),
    // even though the representative impulse p itself is not unique.
    let alternate = base_impulse
        .iter()
        .zip(&null_impulse)
        .map(|(base, null)| base.clone() + r(2, 5) * null.clone())
        .collect::<Vector>();
    assert_ne!(alternate, base_impulse);
    assert_eq!(
        mat_vec(&delassus, &alternate),
        mat_vec(&delassus, &base_impulse)
    );
}


#[test]
fn three_non_collinear_point_contacts_have_a_three_dimensional_impulse_fiber() {
    // Three points on the plane z=0, at r1=(0,0,0), r2=(1,0,0),
    // r3=(0,1,0); contact frames use normal ez and tangents ex,ey.
    let jacobian = vec![
        vec![z(), z(), i(1), z(), z(), z()],
        vec![i(1), z(), z(), z(), z(), z()],
        vec![z(), i(1), z(), z(), z(), z()],
        vec![z(), z(), i(1), z(), i(-1), z()],
        vec![i(1), z(), z(), z(), z(), z()],
        vec![z(), i(1), z(), z(), z(), i(1)],
        vec![z(), z(), i(1), i(1), z(), z()],
        vec![i(1), z(), z(), z(), z(), i(-1)],
        vec![z(), i(1), z(), z(), z(), z()],
    ];
    let inverse_mass = vec![
        vec![i(1), z(), z(), z(), z(), z()],
        vec![z(), i(1), z(), z(), z(), z()],
        vec![z(), z(), i(1), z(), z(), z()],
        vec![z(), z(), z(), i(1), z(), z()],
        vec![z(), z(), z(), z(), r(1, 2), z()],
        vec![z(), z(), z(), z(), z(), r(1, 3)],
    ];
    let delassus = multiply(
        &multiply(&jacobian, &inverse_mass),
        &transpose(&jacobian),
    );

    assert_eq!(jacobian.len(), 9);
    assert_eq!(jacobian[0].len(), 6);
    assert_eq!(delassus.len(), 9);
    assert_eq!(rank(&jacobian), 6);
    assert_eq!(rank(&delassus), 6);
    assert_eq!(delassus, transpose(&delassus));

    // Columns are a rational basis for ker(J^T)=ker(W).
    let null_basis = vec![
        vec![z(), z(), z()],
        vec![i(-1), i(-1), z()],
        vec![z(), i(-1), i(-1)],
        vec![z(), z(), z()],
        vec![i(1), z(), z()],
        vec![z(), i(1), z()],
        vec![z(), z(), z()],
        vec![z(), i(1), z()],
        vec![z(), z(), i(1)],
    ];
    assert_eq!(rank(&null_basis), 3);
    let basis_columns = transpose(&null_basis);
    for basis_vector in &basis_columns {
        assert_eq!(
            mat_vec(&delassus, basis_vector),
            vec![z(), z(), z(), z(), z(), z(), z(), z(), z()]
        );
    }

    let base_impulse = vec![
        i(2), z(), z(),
        i(2), z(), z(),
        i(2), z(), z(),
    ];
    let response = mat_vec(&delassus, &base_impulse);
    assert_eq!(
        response,
        vec![i(6), z(), z(), i(7), z(), z(), i(8), z(), z()]
    );
    let free_velocity = response
        .iter()
        .cloned()
        .map(std::ops::Neg::neg)
        .collect::<Vector>();
    let expected_energy = i(-21);
    let mu = r(1, 2);

    // The parameter cube is wholly inside every local Coulomb disk:
    // |alpha|,|beta|,|gamma| <= 1/10 gives tangent components bounded by 1/5,
    // while each disk radius is mu*lambda=1.
    let parameters = [
        (r(-1, 10), r(-1, 10), r(-1, 10)),
        (r(-1, 10), r(1, 10), r(-1, 10)),
        (r(1, 10), r(-1, 10), r(1, 10)),
        (r(1, 10), r(1, 10), r(1, 10)),
        (z(), z(), z()),
    ];
    let mut allocations = Vec::new();

    for (alpha, beta, gamma) in parameters {
        let impulse = (0..9)
            .map(|row| {
                base_impulse[row].clone()
                    + alpha.clone() * null_basis[row][0].clone()
                    + beta.clone() * null_basis[row][1].clone()
                    + gamma.clone() * null_basis[row][2].clone()
            })
            .collect::<Vector>();
        let actual_response = mat_vec(&delassus, &impulse);
        assert_eq!(actual_response, response);
        assert_eq!(
            actual_response
                .iter()
                .zip(&free_velocity)
                .map(|(left, right)| left.clone() + right.clone())
                .collect::<Vector>(),
            vec![z(), z(), z(), z(), z(), z(), z(), z(), z()]
        );

        for (normal_index, tangent_indices) in [
            (0_usize, [1_usize, 2_usize]),
            (3_usize, [4_usize, 5_usize]),
            (6_usize, [7_usize, 8_usize]),
        ] {
            let normal = impulse[normal_index].clone();
            let radius = mu.clone() * normal.clone();
            let tangent_norm_squared = impulse[tangent_indices[0]].clone()
                * impulse[tangent_indices[0]].clone()
                + impulse[tangent_indices[1]].clone()
                    * impulse[tangent_indices[1]].clone();
            assert!(normal > Rational::zero());
            assert!(radius.clone() * radius > tangent_norm_squared);
        }

        let energy = dot(&impulse, &free_velocity)
            + r(1, 2) * dot(&impulse, &actual_response);
        assert_eq!(energy, expected_energy);
        allocations.push(impulse);
    }

    assert!(allocations.windows(2).all(|pair| pair[0] != pair[1]));
}
