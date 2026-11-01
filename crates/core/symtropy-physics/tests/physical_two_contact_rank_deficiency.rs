//! Exact physical geometry fixture for a rank-deficient two-contact Delassus island.
//!
//! Two point contacts on one rigid body cannot, in general, be assumed to
//! produce an SPD 6x6 Delassus matrix. This test constructs W = J M^-1 J^T
//! from rational contact geometry and certifies a continuum of admissible
//! impulse allocations that all produce the same generalized response.

#[path = "../src/coupled_friction_sturm_support.rs"]
mod exact_support;

use exact_support::Rational;
use num_bigint::BigInt;
use num_traits::{One, Zero};

fn rational(value: i64) -> Rational {
    Rational::from_integer(BigInt::from(value))
}

fn rational_matrix(values: &[[i64; 6]; 6]) -> Vec<Vec<Rational>> {
    values
        .iter()
        .map(|row| row.iter().map(|value| rational(*value)).collect())
        .collect()
}

fn transpose(matrix: &[Vec<Rational>]) -> Vec<Vec<Rational>> {
    let rows = matrix.len();
    let columns = matrix.first().map_or(0, Vec::len);
    (0..columns)
        .map(|column| {
            (0..rows)
                .map(|row| matrix[row][column].clone())
                .collect()
        })
        .collect()
}

fn matrix_multiply(
    lhs: &[Vec<Rational>],
    rhs: &[Vec<Rational>],
) -> Vec<Vec<Rational>> {
    assert!(!lhs.is_empty() && !rhs.is_empty());
    let inner = lhs[0].len();
    assert_eq!(inner, rhs.len());
    assert!(lhs.iter().all(|row| row.len() == inner));
    let columns = rhs[0].len();
    assert!(rhs.iter().all(|row| row.len() == columns));
    lhs.iter()
        .map(|row| {
            (0..columns)
                .map(|column| {
                    (0..inner).fold(Rational::zero(), |sum, index| {
                        sum + row[index].clone() * rhs[index][column].clone()
                    })
                })
                .collect()
        })
        .collect()
}

fn matrix_vector(
    matrix: &[Vec<Rational>],
    vector: &[Rational],
) -> Vec<Rational> {
    matrix
        .iter()
        .map(|row| {
            row.iter()
                .zip(vector)
                .fold(Rational::zero(), |sum, (a, b)| {
                    sum + a.clone() * b.clone()
                })
        })
        .collect()
}

fn dot(lhs: &[Rational], rhs: &[Rational]) -> Rational {
    lhs.iter()
        .zip(rhs)
        .fold(Rational::zero(), |sum, (a, b)| {
            sum + a.clone() * b.clone()
        })
}

fn exact_rank(matrix: &[Vec<Rational>]) -> usize {
    if matrix.is_empty() {
        return 0;
    }
    let mut work = matrix.to_vec();
    let row_count = work.len();
    let column_count = work[0].len();
    assert!(work.iter().all(|row| row.len() == column_count));
    let mut pivot_row = 0;

    for column in 0..column_count {
        let Some(found) = (pivot_row..row_count)
            .find(|&row| !work[row][column].is_zero())
        else {
            continue;
        };
        work.swap(pivot_row, found);

        let pivot = work[pivot_row][column].clone();
        for entry in &mut work[pivot_row][column..] {
            *entry = entry.clone() / pivot.clone();
        }
        for row in 0..row_count {
            if row == pivot_row {
                continue;
            }
            let factor = work[row][column].clone();
            for inner_column in column..column_count {
                work[row][inner_column] = work[row][inner_column].clone()
                    - factor.clone() * work[pivot_row][inner_column].clone();
            }
        }

        pivot_row += 1;
        if pivot_row == row_count {
            break;
        }
    }
    pivot_row
}

#[test]
fn physical_two_point_delassus_is_rank_five_and_has_a_continuum_of_sticking_impulses() {
    // One unit-mass rigid body; principal inertia I = diag(1, 2, 3).
    // Contact points are r1=(0,0,0) and r2=(0,1,0), so they lie on the same
    // plane with normal e_x. Both contact frames are (e_x,e_y,e_z).
    //
    // For a twist (v, omega), point velocity is v + omega x r. Therefore:
    // J = [ I  0 ]
    //     [ I -[r2]x ]
    // in contact-coordinate order (n1,t11,t12,n2,t21,t22).
    let jacobian = rational_matrix(&[
        [1, 0, 0, 0, 0, 0],
        [0, 1, 0, 0, 0, 0],
        [0, 0, 1, 0, 0, 0],
        [1, 0, 0, 0, 0, -1],
        [0, 1, 0, 0, 0, 0],
        [0, 0, 1, 1, 0, 0],
    ]);
    assert_eq!(exact_rank(&jacobian), 5);

    // M^-1 = diag(1,1,1,1,1/2,1/3), represented exactly.
    let inverse_mass_diagonal = [
        Rational::one(),
        Rational::one(),
        Rational::one(),
        Rational::one(),
        Rational::new(BigInt::one(), BigInt::from(2)),
        Rational::new(BigInt::one(), BigInt::from(3)),
    ];
    let mut jacobian_inverse_mass = jacobian.clone();
    for row in &mut jacobian_inverse_mass {
        for (entry, inverse_mass) in row.iter_mut().zip(&inverse_mass_diagonal) {
            *entry = entry.clone() * inverse_mass.clone();
        }
    }
    let delassus = matrix_multiply(&jacobian_inverse_mass, &transpose(&jacobian));

    // Exact W = J M^-1 J^T. Since M^-1 is positive definite, W is PSD;
    // equality of ranks establishes rank(W)=rank(J)=5.
    let expected = vec![
        vec![rational(1), rational(0), rational(0), rational(1), rational(0), rational(0)],
        vec![rational(0), rational(1), rational(0), rational(0), rational(1), rational(0)],
        vec![rational(0), rational(0), rational(1), rational(0), rational(0), rational(1)],
        vec![rational(1), rational(0), rational(0), Rational::new(BigInt::from(4), BigInt::from(3)), rational(0), rational(0)],
        vec![rational(0), rational(1), rational(0), rational(0), rational(1), rational(0)],
        vec![rational(0), rational(0), rational(1), rational(0), rational(0), rational(2)],
    ];
    assert_eq!(delassus, expected);
    assert_eq!(exact_rank(&delassus), 5);

    // Contact-impulse null vector: opposite tangent-1 impulses at the two
    // points are a self-equilibrated force pair because their separation is
    // parallel to that tangent direction.
    let null_impulse = vec![
        rational(0),
        rational(1),
        rational(0),
        rational(0),
        rational(-1),
        rational(0),
    ];
    let w_null = matrix_vector(&delassus, &null_impulse);
    assert!(w_null.iter().all(Rational::is_zero));

    // The chosen q makes p0 an all-sticking solution. Every p(alpha)=p0+alpha*n
    // has the same Wp and therefore the same post-contact velocity.
    let p0 = vec![
        rational(2),
        rational(0),
        rational(0),
        rational(2),
        rational(0),
        rational(0),
    ];
    let wp0 = matrix_vector(&delassus, &p0);
    let q = wp0.iter().map(|value| -value.clone()).collect::<Vec<_>>();
    assert_eq!(
        q,
        vec![
            rational(-4),
            rational(0),
            rational(0),
            Rational::new(BigInt::from(-14), BigInt::from(3)),
            rational(0),
            rational(0),
        ]
    );
    assert!(dot(&q, &null_impulse).is_zero());

    // mu_i=1/2 and lambda_i=2, hence each disk radius is exactly one.
    // For every alpha in (-1,1), both tangential impulses have magnitude
    // |alpha|<1, so this is a continuum of strictly sticking solutions.
    let half = Rational::new(BigInt::one(), BigInt::from(2));
    for alpha in [
        Rational::new(BigInt::from(-3), BigInt::from(4)),
        Rational::new(BigInt::from(-1), BigInt::from(2)),
        Rational::zero(),
        Rational::new(BigInt::one(), BigInt::from(2)),
        Rational::new(BigInt::from(3), BigInt::from(4)),
    ] {
        let impulse = p0
            .iter()
            .zip(&null_impulse)
            .map(|(base, direction)| base.clone() + alpha.clone() * direction.clone())
            .collect::<Vec<_>>();
        assert!(impulse[0] > Rational::zero());
        assert!(impulse[3] > Rational::zero());

        let post_velocity = matrix_vector(&delassus, &impulse)
            .into_iter()
            .zip(&q)
            .map(|(response, free)| response + free.clone())
            .collect::<Vec<_>>();
        assert!(post_velocity.iter().all(Rational::is_zero));

        let disk_1_margin =
            (&half * &impulse[0]).pow(2) - impulse[1].clone().pow(2) - impulse[2].clone().pow(2);
        let disk_2_margin =
            (&half * &impulse[3]).pow(2) - impulse[4].clone().pow(2) - impulse[5].clone().pow(2);
        assert!(disk_1_margin > Rational::zero());
        assert!(disk_2_margin > Rational::zero());

        let wp = matrix_vector(&delassus, &impulse);
        let energy_delta = dot(&impulse, &q) + half.clone() * dot(&impulse, &wp);
        assert_eq!(
            energy_delta,
            Rational::new(BigInt::from(-26), BigInt::from(3))
        );
    }

    // For arbitrary alpha, W(p0+alpha*n)=Wp0 because Wn=0; q^T n=0 and
    // n^T W=0 also make the kinetic-energy change invariant. The exact
    // inequalities above reduce to 1-alpha^2>0, proving the open interval
    // (-1,1) consists entirely of admissible impulse allocations.
    assert_eq!(matrix_vector(&delassus, &p0), wp0);
}
