//! Exact two-contact slide/stick stratum certificate.
//!
//! This child module reuses the exact binary64 decoder, polynomial arithmetic,
//! and rational Sturm isolator from its parent single-contact verifier.

use crate::coupled_friction_sturm_support::{
    Polynomial, Rational, derivative, exact_f64, is_zero, normalize_positive, polynomial_add,
    polynomial_multiply, polynomial_scale, remainder, sturm_isolate_rational, trim,
};
use nalgebra::{SMatrix, SVector};
use num_bigint::BigInt;
use num_traits::{One, Zero};

type RationalInterval = (Rational, Rational);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlidingBranchClassification {
    Admissible,
    NegativeMultiplier,
    EnergyIncreasing,
}

fn polynomial_divide_exact(
    dividend: &[Rational],
    divisor: &[Rational],
) -> Result<Polynomial, &'static str> {
    if is_zero(divisor) {
        return Err("zero divisor in exact polynomial division");
    }
    let mut rest = dividend.to_vec();
    trim(&mut rest);
    let mut quotient =
        vec![Rational::zero(); rest.len().saturating_sub(divisor.len()).saturating_add(1)];
    while !is_zero(&rest) && rest.len() >= divisor.len() {
        let shift = rest.len() - divisor.len();
        let factor = rest.last().ok_or("empty division remainder")?.clone()
            / divisor.last().ok_or("empty division divisor")?.clone();
        quotient[shift] += factor.clone();
        for (index, coefficient) in divisor.iter().enumerate() {
            let slot = index + shift;
            rest[slot] = rest[slot].clone() - factor.clone() * coefficient.clone();
        }
        trim(&mut rest);
    }
    if !is_zero(&rest) {
        return Err("polynomial division left a nonzero exact remainder");
    }
    trim(&mut quotient);
    Ok(quotient)
}

fn polynomial_gcd(lhs: &[Rational], rhs: &[Rational]) -> Result<Polynomial, &'static str> {
    let mut a = lhs.to_vec();
    let mut b = rhs.to_vec();
    trim(&mut a);
    trim(&mut b);
    while !is_zero(&b) {
        let next = remainder(&a, &b)?;
        a = b;
        b = next;
    }
    normalize_positive(&mut a);
    Ok(a)
}

fn polynomial_determinant(matrix: &[Vec<Polynomial>]) -> Polynomial {
    if matrix.is_empty() {
        return vec![Rational::one()];
    }
    let mut determinant = Vec::new();
    for column in 0..matrix.len() {
        let minor = matrix
            .iter()
            .skip(1)
            .map(|row| {
                row.iter()
                    .enumerate()
                    .filter_map(|(index, entry)| (index != column).then_some(entry.clone()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let term = polynomial_multiply(&matrix[0][column], &polynomial_determinant(&minor));
        let signed = if column % 2 == 0 {
            term
        } else {
            polynomial_scale(&term, &-Rational::one())
        };
        determinant = polynomial_add(&determinant, &signed);
    }
    determinant
}

fn exact_two_contact_slide_stick_polynomials(
    w_input: &SMatrix<f64, 6, 6>,
    q_input: &SVector<f64, 6>,
    mu_input: f64,
) -> Result<(Polynomial, Polynomial, Vec<Polynomial>), &'static str> {
    let mut w = vec![vec![Rational::zero(); 6]; 6];
    let mut q = vec![Rational::zero(); 6];
    for row in 0..6 {
        q[row] = exact_f64(q_input[row])?;
        for column in 0..6 {
            w[row][column] = exact_f64(w_input[(row, column)])?;
        }
    }
    let mu = exact_f64(mu_input)?;
    let zero = Rational::zero();
    let one = Rational::one();
    let two = Rational::from_integer(BigInt::from(2));
    let denominator = vec![one.clone(), zero.clone(), one.clone()];
    let one_minus_t_squared = vec![one.clone(), zero.clone(), -one.clone()];
    let twice_t = vec![zero.clone(), two];

    // The active equations are u_n1 = u_n2 = u_t21 = u_t22 = 0.
    // At a fixed angle, p is parameterized by z=(lambda_1, lambda_2, j_21, j_22):
    // p=(lambda_1, -mu*lambda_1*cos(theta), -mu*lambda_1*sin(theta),
    //    lambda_2, j_21, j_22).
    // Multiplying by 1+t^2 makes the 4x4 system polynomial.
    let active_rows = [0_usize, 3, 4, 5];
    let mut system = Vec::with_capacity(4);
    let mut rhs = Vec::with_capacity(4);
    for row in active_rows {
        let mut first_column = polynomial_scale(&denominator, &w[row][0]);
        first_column = polynomial_add(
            &first_column,
            &polynomial_scale(&one_minus_t_squared, &(-mu.clone() * w[row][1].clone())),
        );
        first_column = polynomial_add(
            &first_column,
            &polynomial_scale(&twice_t, &(-mu.clone() * w[row][2].clone())),
        );
        system.push(vec![
            first_column,
            polynomial_scale(&denominator, &w[row][3]),
            polynomial_scale(&denominator, &w[row][4]),
            polynomial_scale(&denominator, &w[row][5]),
        ]);
        rhs.push(polynomial_scale(&denominator, &(-q[row].clone())));
    }

    // Cramer's numerators preserve exact rational dependence on t.
    let determinant = polynomial_determinant(&system);
    let mut numerators = Vec::with_capacity(4);
    for column in 0..4 {
        let mut replaced = system.clone();
        for row in 0..4 {
            replaced[row][column] = rhs[row].clone();
        }
        numerators.push(polynomial_determinant(&replaced));
    }

    // Express u_t1 over the common denominator (1+t^2)*det(system).
    let mut impulse_numerators = Vec::with_capacity(6);
    impulse_numerators.push(polynomial_multiply(&denominator, &numerators[0]));
    impulse_numerators.push(polynomial_scale(
        &polynomial_multiply(&one_minus_t_squared, &numerators[0]),
        &(-mu.clone()),
    ));
    impulse_numerators.push(polynomial_scale(
        &polynomial_multiply(&twice_t, &numerators[0]),
        &(-mu),
    ));
    for numerator in numerators.iter().skip(1) {
        impulse_numerators.push(polynomial_multiply(&denominator, numerator));
    }
    let common_denominator = polynomial_multiply(&denominator, &determinant);
    let mut post_velocity_numerators = Vec::with_capacity(6);
    for row in 0..6 {
        let mut value = polynomial_scale(&common_denominator, &q[row]);
        for column in 0..6 {
            value = polynomial_add(
                &value,
                &polynomial_scale(&impulse_numerators[column], &w[row][column]),
            );
        }
        post_velocity_numerators.push(value);
    }

    // Sliding KKT requires s_perp dot u_t1 = 0. After clearing both the
    // active-system and unit-direction denominators, the numerator contains
    // (1+t^2)^4, which has no real zeros; remove it exactly to retain the
    // quartic whose distinct real roots are the finite angular candidates.
    let angular_numerator = polynomial_add(
        &polynomial_scale(
            &polynomial_multiply(&twice_t, &post_velocity_numerators[1]),
            &-Rational::one(),
        ),
        &polynomial_multiply(&one_minus_t_squared, &post_velocity_numerators[2]),
    );
    let mut quartic = angular_numerator;
    for _ in 0..4 {
        quartic = polynomial_divide_exact(&quartic, &denominator)?;
    }
    if quartic.len() != 5
        || quartic
            .last()
            .is_none_or(|coefficient| coefficient.is_zero())
    {
        return Err("slide/stick angular polynomial did not reduce to a proper quartic");
    }
    Ok((quartic, determinant, numerators))
}

fn interval_add(lhs: &RationalInterval, rhs: &RationalInterval) -> RationalInterval {
    (lhs.0.clone() + rhs.0.clone(), lhs.1.clone() + rhs.1.clone())
}

fn interval_neg(value: &RationalInterval) -> RationalInterval {
    (-value.1.clone(), -value.0.clone())
}

fn interval_sub(lhs: &RationalInterval, rhs: &RationalInterval) -> RationalInterval {
    interval_add(lhs, &interval_neg(rhs))
}

fn interval_mul(lhs: &RationalInterval, rhs: &RationalInterval) -> RationalInterval {
    let products = [
        lhs.0.clone() * rhs.0.clone(),
        lhs.0.clone() * rhs.1.clone(),
        lhs.1.clone() * rhs.0.clone(),
        lhs.1.clone() * rhs.1.clone(),
    ];
    let mut lower = products[0].clone();
    let mut upper = products[0].clone();
    for value in products.iter().skip(1) {
        if value < &lower {
            lower = value.clone();
        }
        if value > &upper {
            upper = value.clone();
        }
    }
    (lower, upper)
}

fn interval_div(
    numerator: &RationalInterval,
    denominator: &RationalInterval,
) -> Result<RationalInterval, &'static str> {
    if denominator.0 <= Rational::zero() && denominator.1 >= Rational::zero() {
        return Err("interval denominator contains zero");
    }
    let reciprocal = (
        Rational::one() / denominator.1.clone(),
        Rational::one() / denominator.0.clone(),
    );
    Ok(interval_mul(numerator, &reciprocal))
}

fn interval_scale(value: &RationalInterval, scale: &Rational) -> RationalInterval {
    interval_mul(value, &(scale.clone(), scale.clone()))
}

fn interval_evaluate(polynomial: &[Rational], x: &RationalInterval) -> RationalInterval {
    polynomial.iter().rev().fold(
        (Rational::zero(), Rational::zero()),
        |value, coefficient| {
            interval_add(
                &interval_mul(&value, x),
                &(coefficient.clone(), coefficient.clone()),
            )
        },
    )
}

fn exact_symmetric_matrix_is_spd(matrix: &[Vec<Rational>]) -> bool {
    if matrix.is_empty() || matrix.iter().any(|row| row.len() != matrix.len()) {
        return false;
    }
    for row in 0..matrix.len() {
        for column in 0..matrix.len() {
            if matrix[row][column] != matrix[column][row] {
                return false;
            }
        }
    }
    for size in 1..=matrix.len() {
        let leading = matrix
            .iter()
            .take(size)
            .map(|row| {
                row.iter()
                    .take(size)
                    .map(|value| vec![value.clone()])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let determinant = polynomial_determinant(&leading);
        if determinant.len() != 1 || determinant[0] <= Rational::zero() {
            return false;
        }
    }
    true
}

fn exact_two_contact_slide_stick_branch_intervals(
    w: &SMatrix<f64, 6, 6>,
    q: &SVector<f64, 6>,
    mu_sliding: f64,
    mu_sticking: f64,
    determinant: &[Rational],
    numerators: &[Polynomial],
    roots: &[(Rational, Rational)],
) -> Result<Vec<SlidingBranchClassification>, &'static str> {
    let mut exact_w = vec![vec![Rational::zero(); 6]; 6];
    let mut exact_q = vec![Rational::zero(); 6];
    for row in 0..6 {
        exact_q[row] = exact_f64(q[row])?;
        for column in 0..6 {
            exact_w[row][column] = exact_f64(w[(row, column)])?;
        }
    }
    let mu = exact_f64(mu_sliding)?;
    let mu_stick = exact_f64(mu_sticking)?;
    let half = Rational::new(BigInt::one(), BigInt::from(2));
    let one = Rational::one();
    let mut classifications = Vec::with_capacity(roots.len());

    for (lower, upper) in roots {
        let t = (lower.clone(), upper.clone());
        let det = interval_evaluate(determinant, &t);
        if det.0 <= Rational::zero() && det.1 >= Rational::zero() {
            return Err("an angular root interval overlaps a singular active-system determinant");
        }
        let z = numerators
            .iter()
            .map(|polynomial| interval_div(&interval_evaluate(polynomial, &t), &det))
            .collect::<Result<Vec<_>, _>>()?;
        let denominator = interval_add(&(one.clone(), one.clone()), &interval_mul(&t, &t));
        let cosine = interval_div(
            &interval_sub(&(one.clone(), one.clone()), &interval_mul(&t, &t)),
            &denominator,
        )?;
        let sine = interval_div(
            &interval_scale(&t, &Rational::from_integer(BigInt::from(2))),
            &denominator,
        )?;

        let impulse = vec![
            z[0].clone(),
            interval_scale(&interval_mul(&cosine, &z[0]), &(-mu.clone())),
            interval_scale(&interval_mul(&sine, &z[0]), &(-mu.clone())),
            z[1].clone(),
            z[2].clone(),
            z[3].clone(),
        ];
        let mut velocity = Vec::with_capacity(6);
        for row in 0..6 {
            let mut value = (exact_q[row].clone(), exact_q[row].clone());
            for column in 0..6 {
                value = interval_add(
                    &value,
                    &interval_scale(&impulse[column], &exact_w[row][column]),
                );
            }
            velocity.push(value);
        }

        // These four equations define the Cramer system exactly. Their
        // outward interval evaluations must enclose zero.
        for row in [0_usize, 3, 4, 5] {
            if velocity[row].0 > Rational::zero() || velocity[row].1 < Rational::zero() {
                return Err("active normal/sticking equation is not enclosed by its root interval");
            }
        }

        let tangent_norm_squared = interval_add(
            &interval_mul(&impulse[1], &impulse[1]),
            &interval_mul(&impulse[2], &impulse[2]),
        );
        if tangent_norm_squared.0 <= Rational::zero() {
            return Err("sliding tangent impulse norm interval is not strictly positive");
        }
        let tangent_work = interval_add(
            &interval_mul(&velocity[1], &impulse[1]),
            &interval_mul(&velocity[2], &impulse[2]),
        );
        let tau = interval_div(&interval_neg(&tangent_work), &tangent_norm_squared)?;

        let stick_radius = interval_scale(&impulse[3], &mu_stick);
        let disk_2_margin = interval_sub(
            &interval_mul(&stick_radius, &stick_radius),
            &interval_add(
                &interval_mul(&impulse[4], &impulse[4]),
                &interval_mul(&impulse[5], &impulse[5]),
            ),
        );
        let mut energy = (Rational::zero(), Rational::zero());
        for row in 0..6 {
            energy = interval_add(
                &energy,
                &interval_mul(&impulse[row], &(exact_q[row].clone(), exact_q[row].clone())),
            );
            for column in 0..6 {
                energy = interval_add(
                    &energy,
                    &interval_scale(
                        &interval_mul(&impulse[row], &impulse[column]),
                        &(&exact_w[row][column] * half.clone()),
                    ),
                );
            }
        }

        if impulse[0].0 <= Rational::zero()
            || impulse[3].0 <= Rational::zero()
            || disk_2_margin.0 <= Rational::zero()
        {
            return Err(
                "a root does not have positive normal loads and a strict sticking-disk margin",
            );
        }
        if tau.0 > Rational::zero() {
            if energy.1 <= Rational::zero() {
                classifications.push(SlidingBranchClassification::Admissible);
            } else if energy.0 > Rational::zero() {
                classifications.push(SlidingBranchClassification::EnergyIncreasing);
            } else {
                return Err("the island energy interval overlaps the admissibility boundary");
            }
        } else if tau.1 < Rational::zero() {
            classifications.push(SlidingBranchClassification::NegativeMultiplier);
        } else {
            return Err("the sliding multiplier interval overlaps zero");
        }
    }
    Ok(classifications)
}

fn two_contact_witness() -> (SMatrix<f64, 6, 6>, SVector<f64, 6>) {
    let w = SMatrix::<f64, 6, 6>::from_row_slice(&[
        0.59497432,
        -0.39054620,
        -0.63542426,
        -0.0015969863578854011,
        -0.05030946883687662,
        0.1029609275985498,
        -0.39054620,
        1.65052340,
        0.80910089,
        0.019271919047677163,
        0.025741864981422207,
        -0.0083170671965776075,
        -0.63542426,
        0.80910089,
        0.94522735,
        0.002331483384856203,
        0.013469705454618652,
        0.0078042835950851083,
        -0.0015969863578854011,
        0.019271919047677163,
        0.002331483384856203,
        1.0,
        0.0,
        0.0,
        -0.05030946883687662,
        0.025741864981422207,
        0.013469705454618652,
        0.0,
        1.0,
        0.0,
        0.1029609275985498,
        -0.0083170671965776075,
        0.0078042835950851083,
        0.0,
        0.0,
        1.0,
    ]);
    let q = SVector::<f64, 6>::new(-0.10770302, 1.49149027, -0.95561905, -1.0, 0.1, -0.1);
    (w, q)
}

#[test]
fn two_contact_slide_stick_stratum_has_exactly_two_admissible_roots() {
    let (w, q) = two_contact_witness();
    let mu_1 = 1.383_289_774_658_335;

    let mut exact_w = vec![vec![Rational::zero(); 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            exact_w[row][column] = exact_f64(w[(row, column)]).unwrap();
        }
    }
    assert!(
        exact_symmetric_matrix_is_spd(&exact_w),
        "the exact binary64 Delassus matrix must be symmetric positive definite"
    );

    let (quartic, determinant, numerators) =
        exact_two_contact_slide_stick_polynomials(&w, &q, mu_1)
            .expect("exact dyadic slide/stick polynomial derivation");
    assert_eq!(quartic.len(), 5);
    assert!(
        !quartic
            .last()
            .expect("quartic leading coefficient")
            .is_zero()
    );
    assert_eq!(
        polynomial_gcd(&quartic, &derivative(&quartic))
            .expect("square-free polynomial gcd")
            .len(),
        1,
        "the reduced angular quartic must have four simple roots"
    );

    let roots = sturm_isolate_rational(quartic.clone())
        .expect("exact Sturm isolation of the slide/stick angular quartic");
    assert_eq!(roots.len(), 4, "all finite real angular roots are isolated");

    // det(M_d) contains the known (1+t^2)^3 scaling factor. Remove it
    // before the Cramer gcd check; the remaining factor has exactly the real
    // zeros of the actual active-system determinant.
    let half_angle_denominator = vec![Rational::one(), Rational::zero(), Rational::one()];
    let mut singular_determinant = determinant.clone();
    for _ in 0..3 {
        singular_determinant =
            polynomial_divide_exact(&singular_determinant, &half_angle_denominator)
                .expect("remove the exact projective denominator factor");
    }
    let singular_angles = sturm_isolate_rational(singular_determinant.clone())
        .expect("exact Sturm isolation of singular active-system angles");
    assert_eq!(singular_angles.len(), 2);
    assert_eq!(
        polynomial_gcd(&singular_determinant, &numerators[0])
            .expect("Cramer/determinant gcd")
            .len(),
        1,
        "singular angular systems have no compatible right-hand side"
    );

    // The nonzero quartic leading coefficient excludes an additional
    // projective root at theta=pi (t=infinity).
    let admissible = exact_two_contact_slide_stick_branch_intervals(
        &w,
        &q,
        mu_1,
        1.0,
        &determinant,
        &numerators,
        &roots,
    )
    .expect("exact interval classification of every finite angular root");
    assert_eq!(
        admissible,
        vec![
            SlidingBranchClassification::Admissible,
            SlidingBranchClassification::Admissible,
            SlidingBranchClassification::NegativeMultiplier,
            SlidingBranchClassification::NegativeMultiplier,
        ]
    );
}

#[test]
fn contact_permutation_slide_stick_stratum_has_no_admissible_root() {
    // Swapping contact blocks maps original contact 2 to the sliding contact
    // and original contact 1 to the sticking contact. The permutation is an
    // exact coordinate relabeling, not a new physical problem.
    let (w, q) = two_contact_witness();
    let permutation = [3_usize, 4, 5, 0, 1, 2];
    let permuted_w =
        SMatrix::<f64, 6, 6>::from_fn(|row, column| w[(permutation[row], permutation[column])]);
    let permuted_q = SVector::<f64, 6>::from_fn(|row, _| q[permutation[row]]);
    let mu_sliding = 1.0;
    let mu_sticking = 1.383_289_774_658_335;

    let mut exact_w = vec![vec![Rational::zero(); 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            exact_w[row][column] = exact_f64(permuted_w[(row, column)]).unwrap();
        }
    }
    assert!(
        exact_symmetric_matrix_is_spd(&exact_w),
        "a contact permutation must preserve exact SPD"
    );

    let (quartic, determinant, numerators) =
        exact_two_contact_slide_stick_polynomials(&permuted_w, &permuted_q, mu_sliding)
            .expect("exact dyadic permuted slide/stick polynomial derivation");
    assert_eq!(quartic.len(), 5);
    assert_eq!(
        polynomial_gcd(&quartic, &derivative(&quartic))
            .expect("square-free polynomial gcd")
            .len(),
        1,
        "the permuted angular quartic must have four simple complex roots"
    );
    let roots = sturm_isolate_rational(quartic.clone())
        .expect("exact Sturm isolation of the permuted slide/stick quartic");
    assert!(
        quartic.last().is_some_and(|coefficient| !coefficient.is_zero()),
        "a nonzero leading coefficient excludes the projective theta=pi root"
    );
    assert_eq!(
        roots.len(),
        0,
        "the reverse slide/stick quartic has no finite real angular roots"
    );

    let half_angle_denominator = vec![Rational::one(), Rational::zero(), Rational::one()];
    let mut singular_determinant = determinant.clone();
    for _ in 0..3 {
        singular_determinant =
            polynomial_divide_exact(&singular_determinant, &half_angle_denominator)
                .expect("remove the exact projective denominator factor");
    }
    let singular_angles = sturm_isolate_rational(singular_determinant.clone())
        .expect("exact Sturm isolation of permuted singular active-system angles");
    assert_eq!(singular_angles.len(), 2);
    assert_eq!(
        polynomial_gcd(&singular_determinant, &numerators[0])
            .expect("permuted Cramer/determinant gcd")
            .len(),
        1,
        "permuted singular angular systems have no compatible right-hand side"
    );

    assert!(
        roots.is_empty(),
        "there are no finite angular candidates to classify after exact Sturm isolation"
    );
    // The active linear system's two singular directions were independently
    // excluded above by the exact determinant/Cramer gcd. Combined with the
    // no-real-root quartic and nonzero leading coefficient, this proves that
    // this reverse slide/stick stratum has no admissible branch.
}

#[test]
fn tangent_frame_quarter_turn_preserves_slide_stick_root_classification() {
    let (w, q) = two_contact_witness();
    let mut frame = SMatrix::<f64, 6, 6>::identity();
    for (first, second) in [(1_usize, 2_usize), (4_usize, 5_usize)] {
        frame[(first, first)] = 0.0;
        frame[(first, second)] = -1.0;
        frame[(second, first)] = 1.0;
        frame[(second, second)] = 0.0;
    }
    assert_eq!(frame.transpose() * frame, SMatrix::<f64, 6, 6>::identity());
    let rotated_w = frame * w * frame.transpose();
    let rotated_q = frame * q;
    let mu_sliding = 1.383_289_774_658_335;
    let mu_sticking = 1.0;

    let mut exact_w = vec![vec![Rational::zero(); 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            exact_w[row][column] = exact_f64(rotated_w[(row, column)]).unwrap();
        }
    }
    assert!(exact_symmetric_matrix_is_spd(&exact_w));

    let (quartic, determinant, numerators) =
        exact_two_contact_slide_stick_polynomials(&rotated_w, &rotated_q, mu_sliding)
            .expect("exact quarter-turned slide/stick polynomial derivation");
    assert_eq!(quartic.len(), 5);
    assert_eq!(
        polynomial_gcd(&quartic, &derivative(&quartic))
            .expect("quarter-turned square-free polynomial gcd")
            .len(),
        1
    );
    let roots = sturm_isolate_rational(quartic.clone())
        .expect("exact Sturm isolation after tangent-frame rotation");
    assert_eq!(roots.len(), 4);

    let half_angle_denominator = vec![Rational::one(), Rational::zero(), Rational::one()];
    let mut singular_determinant = determinant.clone();
    for _ in 0..3 {
        singular_determinant =
            polynomial_divide_exact(&singular_determinant, &half_angle_denominator)
                .expect("remove quarter-turned projective denominator factor");
    }
    let singular_angles = sturm_isolate_rational(singular_determinant.clone())
        .expect("exact singular-angle count after tangent-frame rotation");
    assert_eq!(singular_angles.len(), 2);
    assert_eq!(
        polynomial_gcd(&singular_determinant, &numerators[0])
            .expect("quarter-turned Cramer/determinant gcd")
            .len(),
        1
    );

    let classifications = exact_two_contact_slide_stick_branch_intervals(
        &rotated_w,
        &rotated_q,
        mu_sliding,
        mu_sticking,
        &determinant,
        &numerators,
        &roots,
    )
    .expect("exact KKT classification after tangent-frame rotation");
    assert_eq!(
        classifications
            .iter()
            .filter(|&&value| value == SlidingBranchClassification::Admissible)
            .count(),
        2
    );
    assert_eq!(
        classifications
            .iter()
            .filter(|&&value| value == SlidingBranchClassification::NegativeMultiplier)
            .count(),
        2
    );
}

#[test]
fn coherent_delassus_and_velocity_scaling_preserves_contact_solution_set() {
    let (w, q) = two_contact_witness();
    let scaled_w = w * 2.0;
    let scaled_q = q * 2.0;
    let mu_sliding = 1.383_289_774_658_335;
    let mu_sticking = 1.0;

    let mut exact_w = vec![vec![Rational::zero(); 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            exact_w[row][column] = exact_f64(scaled_w[(row, column)]).unwrap();
        }
    }
    assert!(exact_symmetric_matrix_is_spd(&exact_w));

    let (quartic, determinant, numerators) =
        exact_two_contact_slide_stick_polynomials(&scaled_w, &scaled_q, mu_sliding)
            .expect("exact scaled slide/stick polynomial derivation");
    assert_eq!(quartic.len(), 5);
    assert_eq!(
        polynomial_gcd(&quartic, &derivative(&quartic))
            .expect("scaled square-free polynomial gcd")
            .len(),
        1
    );
    let roots = sturm_isolate_rational(quartic.clone())
        .expect("exact Sturm isolation under coherent scaling");
    assert_eq!(roots.len(), 4);

    let half_angle_denominator = vec![Rational::one(), Rational::zero(), Rational::one()];
    let mut singular_determinant = determinant.clone();
    for _ in 0..3 {
        singular_determinant =
            polynomial_divide_exact(&singular_determinant, &half_angle_denominator)
                .expect("remove scaled projective denominator factor");
    }
    let singular_angles = sturm_isolate_rational(singular_determinant.clone())
        .expect("exact singular-angle count under coherent scaling");
    assert_eq!(singular_angles.len(), 2);
    assert_eq!(
        polynomial_gcd(&singular_determinant, &numerators[0])
            .expect("scaled Cramer/determinant gcd")
            .len(),
        1
    );

    let classifications = exact_two_contact_slide_stick_branch_intervals(
        &scaled_w,
        &scaled_q,
        mu_sliding,
        mu_sticking,
        &determinant,
        &numerators,
        &roots,
    )
    .expect("exact KKT classification under coherent scaling");
    assert_eq!(
        classifications,
        vec![
            SlidingBranchClassification::Admissible,
            SlidingBranchClassification::Admissible,
            SlidingBranchClassification::NegativeMultiplier,
            SlidingBranchClassification::NegativeMultiplier,
        ]
    );
}
