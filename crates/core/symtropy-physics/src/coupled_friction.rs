// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Coupled Signorini-Coulomb reference for one 3D point contact.
//!
//! Unlike the fixed-load tangent disk subproblem, this module solves the
//! normal impulse and the two-dimensional tangential impulse together. It
//! retains the full symmetric positive-definite 3x3 Delassus matrix, including
//! normal/tangent coupling.
//!
//! The sliding branch is not solved by a black-box multidimensional Newton
//! iteration. Writing j_t = -mu * lambda_n * s(theta), eliminating lambda_n
//! with the normal equation, and eliminating the nonnegative KKT multiplier
//! leaves a trigonometric polynomial of degree at most two. Under
//! t = tan(theta/2), that is a quartic. We enumerate its real roots, map them
//! back to the original equations, and classify all admissible candidates.
//!
//! This is a single-contact impulse-law reference, not a multi-contact island
//! solver, not a compliant/acceleration-level contact law, and not an engine
//! integrator.

use nalgebra::{Matrix2, Matrix3, SVector};

/// Invalid input or unsupported numerical conditioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoupledCoulombError {
    NonFinite,
    InvalidFrictionCoefficient,
    InvalidTolerance,
    NotSymmetric,
    NotPositiveDefinite,
    IllConditioned,
}

/// Mathematical classification of the bounded single-contact problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoupledCoulombStatus {
    /// Exactly one admissible impulse was isolated by the floating-point
    /// quartic enumerator at the requested tolerance and passed all original
    /// equation residual gates. This is not a formal interval proof of root
    /// completeness; promotion requires an independent high-precision count.
    UniqueWithinTolerance,
    /// Two or more distinct admissible impulses satisfy the same contact law.
    MultipleSolutions,
    /// Floating-point root isolation or a degenerate branch prevented a
    /// complete classification. No impulse is promoted as authoritative.
    Inconclusive,
}

/// Regime associated with one admissible impulse candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoupledCoulombRegime {
    Separating,
    Frictionless,
    Sticking,
    Sliding,
}

/// One candidate checked against the original coupled contact equations.
#[derive(Clone, Debug)]
pub struct CoupledCoulombCandidate {
    /// Impulse vector (normal, tangent-1, tangent-2).
    pub impulse: SVector<f64, 3>,
    /// Post-impulse relative contact velocity q + W p.
    pub post_velocity: SVector<f64, 3>,
    pub regime: CoupledCoulombRegime,
    /// Nonnegative disk KKT multiplier on a sliding branch; zero otherwise.
    pub disk_multiplier: f64,
    /// Sliding direction angle in the caller's tangent frame, when applicable.
    pub sliding_angle: Option<f64>,
    /// Maximum normalized Signorini residual.
    pub normal_complementarity_residual: f64,
    /// Normalized tangential stationarity residual.
    pub tangential_kkt_residual: f64,
    /// Absolute excess beyond ||j_t|| <= mu * lambda_n.
    pub coulomb_disk_violation: f64,
    /// Full 3D kinetic-energy change pᵀq + 1/2 pᵀWp.
    pub kinetic_energy_delta: f64,
}

/// Complete bounded outcome, including the polynomial and every admissible
/// candidate found. Inconclusive deliberately carries no chosen solution.
#[derive(Clone, Debug)]
pub struct CoupledCoulombOutcome {
    pub status: CoupledCoulombStatus,
    pub candidates: Vec<CoupledCoulombCandidate>,
    /// Quartic coefficients in ascending powers of t = tan(theta / 2).
    pub sliding_quartic: [f64; 5],
    /// Real-root angles considered by the sliding branch.
    pub sliding_angles: Vec<f64>,
    /// Sufficient branch guard: W_nn - mu * ||W_nt||. If positive, the
    /// normal coefficient D(theta) stays positive for every tangent direction.
    pub normal_coupling_margin: f64,
}

impl CoupledCoulombOutcome {
    /// Return the sole candidate only when uniqueness was established.
    pub fn unique_solution(&self) -> Option<&CoupledCoulombCandidate> {
        if self.status == CoupledCoulombStatus::UniqueWithinTolerance {
            self.candidates.first()
        } else {
            None
        }
    }
}

/// Solve one 3D point contact with the impulse-level Signorini-Coulomb law.
///
/// Coordinates are (normal, tangent_1, tangent_2). effective_mass is the full
/// symmetric positive-definite Delassus matrix W, and free_velocity is q before
/// the contact impulse. The post-impulse velocity is q + W p.
///
/// Normal law:
/// - lambda_n >= 0
/// - u_n >= 0
/// - lambda_n * u_n = 0
///
/// Tangential law, for fixed lambda_n:
/// - minimize 1/2 j_tᵀ W_tt j_t + (q_t + W_tn lambda_n)ᵀ j_t
/// - subject to ||j_t|| <= mu * lambda_n
///
/// The normal equation is not frozen while solving the tangent plane: the
/// complete W is used by the sliding angular elimination and every final
/// residual check.
pub fn solve_coupled_signorini_coulomb(
    effective_mass: &Matrix3<f64>,
    free_velocity: &SVector<f64, 3>,
    friction_coefficient: f64,
    relative_tolerance: f64,
) -> Result<CoupledCoulombOutcome, CoupledCoulombError> {
    if !effective_mass.iter().all(|x| x.is_finite())
        || !free_velocity.iter().all(|x| x.is_finite())
        || !friction_coefficient.is_finite()
    {
        return Err(CoupledCoulombError::NonFinite);
    }
    if friction_coefficient < 0.0 {
        return Err(CoupledCoulombError::InvalidFrictionCoefficient);
    }
    if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 || relative_tolerance >= 0.1 {
        return Err(CoupledCoulombError::InvalidTolerance);
    }

    // Scale by the largest entry before symmetry/eigenvalue checks. This
    // avoids avoidable overflow in a Frobenius norm for finite large matrices.
    let matrix_scale = effective_mass
        .iter()
        .fold(0.0_f64, |scale, value| scale.max(value.abs()));
    if matrix_scale == 0.0 {
        return Err(CoupledCoulombError::NotPositiveDefinite);
    }
    let scaled_input = effective_mass / matrix_scale;
    let scaled_asymmetry = (&scaled_input - scaled_input.transpose()).norm();
    if !scaled_asymmetry.is_finite() || scaled_asymmetry > relative_tolerance {
        return Err(CoupledCoulombError::NotSymmetric);
    }
    let scaled_w = (&scaled_input + scaled_input.transpose()) * 0.5;
    let eigenvalues = nalgebra::linalg::SymmetricEigen::new(scaled_w).eigenvalues;
    let minimum_eigenvalue = eigenvalues.iter().fold(f64::INFINITY, |a, b| a.min(*b));
    if minimum_eigenvalue <= 0.0 {
        return Err(CoupledCoulombError::NotPositiveDefinite);
    }
    if minimum_eigenvalue <= relative_tolerance {
        return Err(CoupledCoulombError::IllConditioned);
    }
    let w = (effective_mass + effective_mass.transpose()) * 0.5;

    let mu = friction_coefficient;
    let qn = free_velocity[0];
    let a = w[(0, 0)];
    let d = SVector::<f64, 2>::new(w[(0, 1)], w[(0, 2)]);
    let normal_coupling_margin = a - mu * d.norm();

    // Frictionless normal complementarity has a closed-form unique solution.
    if mu == 0.0 {
        let lambda = if qn < 0.0 { -qn / a } else { 0.0 };
        let impulse = SVector::<f64, 3>::new(lambda, 0.0, 0.0);
        let regime = if lambda == 0.0 {
            CoupledCoulombRegime::Separating
        } else {
            CoupledCoulombRegime::Frictionless
        };
        let candidate = validate_candidate(
            &w, free_velocity, mu, impulse, regime, 0.0, None, relative_tolerance,
        );
        let candidates = candidate.into_iter().collect::<Vec<_>>();
        return Ok(CoupledCoulombOutcome {
            status: if candidates.len() == 1 {
                CoupledCoulombStatus::UniqueWithinTolerance
            } else {
                CoupledCoulombStatus::Inconclusive
            },
            candidates,
            sliding_quartic: [0.0; 5],
            sliding_angles: Vec::new(),
            normal_coupling_margin,
        });
    }

    // At grazing, lambda=0 is always admissible. If D(theta) stays strictly
    // positive for every tangent direction, it is the only possible branch.
    // Otherwise D(theta)=0 can support a singular positive-load limit; do not
    // guess through that degenerate branch.
    let velocity_scale = free_velocity.norm().max(f64::MIN_POSITIVE);
    if qn.abs() <= relative_tolerance * velocity_scale {
        if normal_coupling_margin > relative_tolerance * matrix_scale {
            let candidate = validate_candidate(
                &w,
                free_velocity,
                mu,
                SVector::zeros(),
                CoupledCoulombRegime::Separating,
                0.0,
                None,
                relative_tolerance,
            );
            let candidates = candidate.into_iter().collect::<Vec<_>>();
            return Ok(CoupledCoulombOutcome {
                status: if candidates.len() == 1 {
                    CoupledCoulombStatus::UniqueWithinTolerance
                } else {
                    CoupledCoulombStatus::Inconclusive
                },
                candidates,
                sliding_quartic: [0.0; 5],
                sliding_angles: Vec::new(),
                normal_coupling_margin,
            });
        }
        return Ok(CoupledCoulombOutcome {
            status: CoupledCoulombStatus::Inconclusive,
            candidates: Vec::new(),
            sliding_quartic: [0.0; 5],
            sliding_angles: Vec::new(),
            normal_coupling_margin,
        });
    }

    // If q_n > 0 and D(theta) = W_nn - mu W_nt.s stays positive for
    // every unit tangent direction, lambda_n > 0 would require
    // lambda_n = -q_n / D(theta) < 0. The separating solution is then unique.
    if qn > 0.0 && normal_coupling_margin > relative_tolerance * matrix_scale {
        let candidate = validate_candidate(
            &w,
            free_velocity,
            mu,
            SVector::zeros(),
            CoupledCoulombRegime::Separating,
            0.0,
            None,
            relative_tolerance,
        );
        let candidates = candidate.into_iter().collect::<Vec<_>>();
        return Ok(CoupledCoulombOutcome {
            status: if candidates.len() == 1 {
                CoupledCoulombStatus::UniqueWithinTolerance
            } else {
                CoupledCoulombStatus::Inconclusive
            },
            candidates,
            sliding_quartic: [0.0; 5],
            sliding_angles: Vec::new(),
            normal_coupling_margin,
        });
    }

    let quartic = sliding_quartic(&w, free_velocity, mu);
    let mut candidates = Vec::new();
    let mut unresolved = false;
    let mut angles = Vec::new();

    // Separating is an exact candidate whenever the incoming normal velocity
    // is positive. It may coexist with positive-load branches in the
    // non-associated Coulomb law when normal/tangent coupling is strong.
    if qn > 0.0 {
        if let Some(candidate) = validate_candidate(
            &w,
            free_velocity,
            mu,
            SVector::zeros(),
            CoupledCoulombRegime::Separating,
            0.0,
            None,
            relative_tolerance,
        ) {
            push_distinct(&mut candidates, candidate, relative_tolerance);
        }
    }

    // Sticking is the full 3x3 unconstrained minimizer. It is admissible only
    // when its normal impulse is nonnegative and its tangent impulse is in
    // the Coulomb disk.
    if let Some(impulse) = w.lu().solve(&(-free_velocity)) {
        if impulse.iter().all(|x| x.is_finite()) && impulse[0] >= 0.0 {
            if let Some(candidate) = validate_candidate(
                &w,
                free_velocity,
                mu,
                impulse,
                CoupledCoulombRegime::Sticking,
                0.0,
                None,
                relative_tolerance,
            ) {
                push_distinct(&mut candidates, candidate, relative_tolerance);
            }
        }
    } else {
        unresolved = true;
    }

    // Sliding solutions correspond to admissible roots of the reduced angular
    // quartic, except at D(theta)=0, which is a singular branch.
    match real_polynomial_roots(&quartic, relative_tolerance) {
        Ok(roots) => {
            for t in roots {
                let theta = 2.0 * t.atan();
                let theta = if theta < 0.0 {
                    theta + 2.0 * std::f64::consts::PI
                } else {
                    theta
                };
                angles.push(theta);
            }
            let (pi_value, pi_scale) =
                angular_residual(&w, free_velocity, mu, std::f64::consts::PI);
            if pi_value.abs() <= 64.0 * relative_tolerance * pi_scale.max(f64::MIN_POSITIVE) {
                angles.push(std::f64::consts::PI);
            }
        }
        Err(()) => unresolved = true,
    }

    deduplicate_angles(&mut angles, relative_tolerance);
    for theta in &angles {
        let (s, _) = tangent_direction(*theta);
        let denominator = a - mu * d.dot(&s);
        if denominator.abs() <= relative_tolerance * matrix_scale {
            unresolved = true;
            continue;
        }
        let lambda = -qn / denominator;
        if !lambda.is_finite() {
            unresolved = true;
            continue;
        }
        if lambda <= 0.0 {
            continue;
        }
        let jt = -mu * lambda * s;
        let impulse = SVector::<f64, 3>::new(lambda, jt[0], jt[1]);
        let post = free_velocity + w * impulse;
        let jt_norm_sq = jt.norm_squared();
        if jt_norm_sq <= f64::MIN_POSITIVE {
            unresolved = true;
            continue;
        }
        // Sliding KKT: u_t + tau j_t = 0, tau >= 0.
        let tau = -jt.dot(&SVector::<f64, 2>::new(post[1], post[2])) / jt_norm_sq;
        if !tau.is_finite() {
            unresolved = true;
            continue;
        }
        if tau < -100.0 * relative_tolerance * matrix_scale {
            continue;
        }
        if let Some(candidate) = validate_candidate(
            &w,
            free_velocity,
            mu,
            impulse,
            CoupledCoulombRegime::Sliding,
            tau.max(0.0),
            Some(*theta),
            relative_tolerance,
        ) {
            push_distinct(&mut candidates, candidate, relative_tolerance);
        }
    }

    // For SPD W and finite mu, this single-contact impulse law has an
    // existence argument: q_n < 0 starts below zero at lambda=0, while the
    // large-load normal residual has positive Schur-complement slope. Thus an
    // empty numerical candidate set is an isolation failure, not evidence of
    // a physical no-solution paradox.
    let status = if unresolved || candidates.is_empty() {
        CoupledCoulombStatus::Inconclusive
    } else if candidates.len() == 1 {
        CoupledCoulombStatus::UniqueWithinTolerance
    } else {
        CoupledCoulombStatus::MultipleSolutions
    };

    Ok(CoupledCoulombOutcome {
        status,
        candidates,
        sliding_quartic: quartic,
        sliding_angles: angles,
        normal_coupling_margin,
    })
}

fn validate_candidate(
    w: &Matrix3<f64>,
    q: &SVector<f64, 3>,
    mu: f64,
    impulse: SVector<f64, 3>,
    regime: CoupledCoulombRegime,
    disk_multiplier: f64,
    sliding_angle: Option<f64>,
    tolerance: f64,
) -> Option<CoupledCoulombCandidate> {
    if !impulse.iter().all(|x| x.is_finite()) || !disk_multiplier.is_finite() {
        return None;
    }
    let post_velocity = q + w * impulse;
    if !post_velocity.iter().all(|x| x.is_finite()) {
        return None;
    }

    let impulse_scale = impulse.norm().max(f64::MIN_POSITIVE);
    let velocity_scale = q.norm().max((w * impulse).norm()).max(f64::MIN_POSITIVE);
    let lambda = impulse[0];
    let jt = SVector::<f64, 2>::new(impulse[1], impulse[2]);
    let radius = mu * lambda.max(0.0);
    let jt_norm = jt.norm();
    let coulomb_disk_violation = (jt_norm - radius).max(0.0);
    let normal_residual = (-lambda).max(0.0) / impulse_scale;
    let normal_velocity_residual = (-post_velocity[0]).max(0.0) / velocity_scale;
    let complementarity_residual =
        (lambda * post_velocity[0]).abs() / (impulse_scale * velocity_scale);
    let normal_complementarity_residual = normal_residual
        .max(normal_velocity_residual)
        .max(complementarity_residual);

    let tangent_velocity = SVector::<f64, 2>::new(post_velocity[1], post_velocity[2]);
    let tangential_kkt_residual = match regime {
        CoupledCoulombRegime::Sticking => tangent_velocity.norm() / velocity_scale,
        CoupledCoulombRegime::Sliding => {
            let stationarity = tangent_velocity + jt * disk_multiplier;
            let boundary = (jt_norm - radius).abs() / impulse_scale;
            (stationarity.norm() / velocity_scale).max(boundary)
        }
        CoupledCoulombRegime::Separating | CoupledCoulombRegime::Frictionless => 0.0,
    };

    let kinetic_energy_delta = impulse.dot(&(q + 0.5 * (w * impulse)));
    let work_scale = impulse.norm() * velocity_scale;
    let work_residual =
        kinetic_energy_delta.max(0.0) / work_scale.max(f64::MIN_POSITIVE);

    let limit = 200.0 * tolerance;
    if normal_complementarity_residual > limit
        || tangential_kkt_residual > limit
        || coulomb_disk_violation > limit * impulse_scale
        || work_residual > limit
        || lambda < -limit * impulse_scale
        || (regime == CoupledCoulombRegime::Sliding
            && disk_multiplier < -limit * w.norm())
    {
        return None;
    }

    Some(CoupledCoulombCandidate {
        impulse,
        post_velocity,
        regime,
        disk_multiplier,
        sliding_angle,
        normal_complementarity_residual,
        tangential_kkt_residual,
        coulomb_disk_violation,
        kinetic_energy_delta,
    })
}

fn push_distinct(
    candidates: &mut Vec<CoupledCoulombCandidate>,
    candidate: CoupledCoulombCandidate,
    tolerance: f64,
) {
    let scale = candidate.impulse.norm().max(f64::MIN_POSITIVE);
    if let Some(existing) = candidates.iter_mut().find(|existing| {
        (existing.impulse - candidate.impulse).norm()
            <= 256.0 * tolerance * scale.max(existing.impulse.norm())
    }) {
        // At the stick/slip boundary, tau=0 is the same physical impulse.
        if candidate.regime == CoupledCoulombRegime::Sticking {
            *existing = candidate;
        }
    } else {
        candidates.push(candidate);
    }
}

fn tangent_direction(theta: f64) -> (SVector<f64, 2>, SVector<f64, 2>) {
    let (sin, cos) = theta.sin_cos();
    (
        SVector::<f64, 2>::new(cos, sin),
        SVector::<f64, 2>::new(-sin, cos),
    )
}

fn angular_residual(
    w: &Matrix3<f64>,
    q: &SVector<f64, 3>,
    mu: f64,
    theta: f64,
) -> (f64, f64) {
    let (s, perpendicular) = tangent_direction(theta);
    let d = SVector::<f64, 2>::new(w[(0, 1)], w[(0, 2)]);
    let c = SVector::<f64, 2>::new(w[(1, 0)], w[(2, 0)]);
    let tangent_metric =
        Matrix2::<f64>::new(w[(1, 1)], w[(1, 2)], w[(2, 1)], w[(2, 2)]);
    let tangent_free = SVector::<f64, 2>::new(q[1], q[2]);
    let denominator = w[(0, 0)] - mu * d.dot(&s);
    let tangent_quadratic = tangent_metric * s;
    let value = denominator * perpendicular.dot(&tangent_free)
        - q[0] * perpendicular.dot(&c)
        + mu * q[0] * perpendicular.dot(&tangent_quadratic);
    let scale = denominator.abs() * tangent_free.norm()
        + q[0].abs() * c.norm()
        + mu * q[0].abs() * tangent_quadratic.norm();
    (value, scale)
}

/// Quartic for the sliding angular equation, coefficients in ascending order.
///
/// Starting from D(theta) s_perp·(q_t + lambda c - mu lambda A s) = 0,
/// lambda = -q_n / D(theta), and multiplying by (1+t²)² gives the polynomial.
/// The full W_nt and W_tn coupling appears in d/c.
fn sliding_quartic(w: &Matrix3<f64>, q: &SVector<f64, 3>, mu: f64) -> [f64; 5] {
    let a = w[(0, 0)];
    let d1 = w[(0, 1)];
    let d2 = w[(0, 2)];
    let qn = q[0];
    let q1 = q[1];
    let q2 = q[2];
    let a11 = w[(1, 1)];
    let a12 = w[(1, 2)];
    let a22 = w[(2, 2)];
    [
        mu * qn * a12 + a * q2 - d2 * qn - d1 * mu * q2,
        -2.0 * a11 * mu * qn + 2.0 * a22 * mu * qn - 2.0 * a * q1
            + 2.0 * d1 * qn + 2.0 * d1 * mu * q1 - 2.0 * d2 * mu * q2,
        -6.0 * a12 * mu * qn + 2.0 * d1 * mu * q2 + 4.0 * d2 * mu * q1,
        2.0 * a11 * mu * qn - 2.0 * a22 * mu * qn - 2.0 * a * q1
            + 2.0 * d1 * qn - 2.0 * d1 * mu * q1 + 2.0 * d2 * mu * q2,
        a12 * mu * qn - a * q2 + d2 * qn - d1 * mu * q2,
    ]
}

fn polynomial_value_and_scale(coefficients: &[f64], x: f64) -> Option<(f64, f64)> {
    let mut value = 0.0;
    let mut scale = 0.0;
    for coefficient in coefficients.iter().rev() {
        value = value * x + coefficient;
        scale = scale * x.abs() + coefficient.abs();
        if !value.is_finite() || !scale.is_finite() {
            return None;
        }
    }
    Some((value, scale.max(f64::MIN_POSITIVE)))
}

fn add_root(roots: &mut Vec<f64>, root: f64, tolerance: f64) {
    if !root.is_finite() {
        return;
    }
    if !roots.iter().any(|existing| {
        (existing - root).abs() <= 128.0 * tolerance * (1.0 + root.abs().max(existing.abs()))
    }) {
        roots.push(root);
    }
}

/// Enumerate real roots by recursively partitioning the real line at roots of
/// the derivative. Each open partition is monotone; sign-changing intervals
/// contain one simple root, while stationary points capture even multiplicity.
/// Floating-point degeneracy and unbounded root ranges fail closed.
fn real_polynomial_roots(input: &[f64], tolerance: f64) -> Result<Vec<f64>, ()> {
    if input.is_empty() || !input.iter().all(|x| x.is_finite()) {
        return Err(());
    }
    let scale = input.iter().fold(0.0_f64, |a, x| a.max(x.abs()));
    if scale == 0.0 || !scale.is_finite() {
        return Err(());
    }
    let mut coefficients = input.iter().map(|x| x / scale).collect::<Vec<_>>();
    while coefficients.len() > 1 && *coefficients.last().unwrap_or(&0.0) == 0.0 {
        coefficients.pop();
    }

    let degree = coefficients.len() - 1;
    if degree == 0 {
        return Ok(Vec::new());
    }
    if degree == 1 {
        let root = -coefficients[0] / coefficients[1];
        return if root.is_finite() { Ok(vec![root]) } else { Err(()) };
    }

    let leading = coefficients[degree].abs();
    let lower_max = coefficients[..degree]
        .iter()
        .fold(0.0_f64, |a, x| a.max(x.abs()));
    let bound = 1.0 + lower_max / leading;
    if !bound.is_finite() || bound > 1.0e12 {
        return Err(());
    }

    let derivative = (1..coefficients.len())
        .map(|i| coefficients[i] * i as f64)
        .collect::<Vec<_>>();
    let critical = real_polynomial_roots(&derivative, tolerance)?;
    let mut points = Vec::with_capacity(critical.len() + 2);
    points.push(-bound);
    points.extend(critical.into_iter().filter(|x| *x > -bound && *x < bound));
    points.push(bound);
    points.sort_by(f64::total_cmp);
    points.dedup_by(|a, b| {
        (*a - *b).abs() <= 64.0 * tolerance * (1.0 + a.abs().max(b.abs()))
    });

    let mut roots = Vec::new();
    for point in &points {
        let (value, eval_scale) =
            polynomial_value_and_scale(&coefficients, *point).ok_or(())?;
        if value == 0.0 {
            add_root(&mut roots, *point, tolerance);
        } else if value.abs() <= 64.0 * tolerance * eval_scale {
            // A small nonzero value at a stationary point cannot distinguish
            // an even-multiplicity root from two very close simple roots.
            // Do not collapse that uncertainty into one fabricated root.
            return Err(());
        }
    }

    for pair in points.windows(2) {
        let mut lower = pair[0];
        let mut upper = pair[1];
        let (mut f_lower, _) =
            polynomial_value_and_scale(&coefficients, lower).ok_or(())?;
        let (f_upper, _) =
            polynomial_value_and_scale(&coefficients, upper).ok_or(())?;
        let lower_is_root = f_lower == 0.0;
        let upper_is_root = f_upper == 0.0;
        if lower_is_root
            || upper_is_root
            || f_lower.is_sign_positive() == f_upper.is_sign_positive()
        {
            continue;
        }

        for _ in 0..192 {
            let midpoint = lower + 0.5 * (upper - lower);
            if midpoint == lower || midpoint == upper {
                break;
            }
            let (f_mid, _) =
                polynomial_value_and_scale(&coefficients, midpoint).ok_or(())?;
            if f_mid == 0.0 {
                lower = midpoint;
                upper = midpoint;
                break;
            }
            if f_mid.is_sign_positive() == f_lower.is_sign_positive() {
                lower = midpoint;
                f_lower = f_mid;
            } else {
                upper = midpoint;
            }
            if upper - lower <= 8.0 * tolerance * (1.0 + midpoint.abs()) {
                break;
            }
        }
        add_root(&mut roots, lower + 0.5 * (upper - lower), tolerance);
    }

    roots.sort_by(f64::total_cmp);
    Ok(roots)
}

fn deduplicate_angles(angles: &mut Vec<f64>, tolerance: f64) {
    angles.sort_by(f64::total_cmp);
    angles.dedup_by(|a, b| {
        let direct = (a - b).abs();
        let wrapped = (2.0 * std::f64::consts::PI - direct).abs();
        direct.min(wrapped) <= 256.0 * tolerance
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_candidate(candidate: &CoupledCoulombCandidate, tolerance: f64) {
        assert!(candidate.normal_complementarity_residual <= tolerance);
        assert!(candidate.tangential_kkt_residual <= tolerance);
        assert!(candidate.coulomb_disk_violation <= tolerance);
        assert!(candidate.kinetic_energy_delta <= tolerance);
    }

    #[test]
    fn diagonal_sticking_matches_full_unconstrained_solution() {
        let w = Matrix3::new(
            2.0, 0.0, 0.0,
            0.0, 1.5, 0.0,
            0.0, 0.0, 0.8,
        );
        let q = SVector::new(-1.0, 0.2, -0.1);
        let outcome = solve_coupled_signorini_coulomb(&w, &q, 0.6, 1.0e-12).unwrap();
        assert_eq!(outcome.status, CoupledCoulombStatus::UniqueWithinTolerance);
        let candidate = outcome.unique_solution().unwrap();
        assert_eq!(candidate.regime, CoupledCoulombRegime::Sticking);
        assert!(
            (candidate.impulse - SVector::new(0.5, -0.2 / 1.5, 0.1 / 0.8)).norm() < 1.0e-10
        );
        assert_candidate(candidate, 1.0e-9);
    }

    #[test]
    fn diagonal_sliding_matches_circular_maximum_dissipation() {
        let w = Matrix3::identity();
        let q = SVector::new(-1.0, -3.0, 1.0);
        let mu = 0.25;
        let outcome = solve_coupled_signorini_coulomb(&w, &q, mu, 1.0e-12).unwrap();
        assert_eq!(outcome.status, CoupledCoulombStatus::UniqueWithinTolerance);
        let candidate = outcome.unique_solution().unwrap();
        assert_eq!(candidate.regime, CoupledCoulombRegime::Sliding);
        let expected = SVector::new(1.0, 3.0 * mu / 10.0_f64.sqrt(), -mu / 10.0_f64.sqrt());
        assert!((candidate.impulse - expected).norm() < 1.0e-9);
        assert_candidate(candidate, 1.0e-8);
    }

    #[test]
    fn off_diagonal_normal_tangent_coupling_is_solved_in_one_system() {
        let w = Matrix3::new(
            3.0, 0.4, -0.2,
            0.4, 2.0, 0.5,
            -0.2, 0.5, 1.5,
        );
        let q = SVector::new(-0.8, -2.5, 0.9);
        let outcome = solve_coupled_signorini_coulomb(&w, &q, 0.22, 1.0e-12).unwrap();
        assert_ne!(outcome.status, CoupledCoulombStatus::Inconclusive);
        assert!(!outcome.candidates.is_empty());
        for candidate in &outcome.candidates {
            assert_candidate(candidate, 1.0e-8);
        }
    }

    #[test]
    fn separating_contact_returns_zero_impulse() {
        let w = Matrix3::new(
            2.0, 0.1, 0.0,
            0.1, 1.0, 0.2,
            0.0, 0.2, 1.4,
        );
        let q = SVector::new(0.5, 2.0, -1.0);
        let outcome = solve_coupled_signorini_coulomb(&w, &q, 0.5, 1.0e-12).unwrap();
        assert_eq!(outcome.status, CoupledCoulombStatus::UniqueWithinTolerance);
        let candidate = outcome.unique_solution().unwrap();
        assert_eq!(candidate.regime, CoupledCoulombRegime::Separating);
        assert_eq!(candidate.impulse, SVector::zeros());
    }

    #[test]
    fn multiple_admissible_solutions_are_reported_not_arbitrarily_selected() {
        let w = Matrix3::new(
            0.59497432, -0.39054620, -0.63542426,
            -0.39054620, 1.65052340, 0.80910089,
            -0.63542426, 0.80910089, 0.94522735,
        );
        let q = SVector::new(-0.10770302, 1.49149027, -0.95561905);
        let outcome = solve_coupled_signorini_coulomb(&w, &q, 1.383289774658335, 1.0e-11).unwrap();
        assert_eq!(outcome.status, CoupledCoulombStatus::MultipleSolutions);
        assert!(outcome.candidates.len() >= 2);
        for candidate in &outcome.candidates {
            assert_candidate(candidate, 1.0e-7);
        }
    }

    #[test]
    fn strong_coupling_can_create_multiple_solutions_from_initial_separation() {
        let w = Matrix3::new(
            3.38215268, 1.97328063, 2.48164591,
            1.97328063, 5.28809392, -1.19836457,
            2.48164591, -1.19836457, 4.53646300,
        );
        let q = SVector::new(0.93301773, 0.94422924, 2.42911533);
        let mu = 2.609311810828915;
        let outcome = solve_coupled_signorini_coulomb(&w, &q, mu, 1.0e-11).unwrap();
        assert_eq!(outcome.status, CoupledCoulombStatus::MultipleSolutions);
        assert!(outcome.candidates.iter().any(|candidate| {
            candidate.regime == CoupledCoulombRegime::Separating
                && candidate.impulse.norm() == 0.0
        }));
        assert!(outcome.candidates.iter().any(|candidate| {
            candidate.regime == CoupledCoulombRegime::Sticking
        }));
        assert!(outcome.candidates.iter().any(|candidate| {
            candidate.regime == CoupledCoulombRegime::Sliding
        }));
        for candidate in &outcome.candidates {
            assert_candidate(candidate, 1.0e-7);
        }
    }

    #[test]
    fn distinguishes_indefinite_from_positive_but_ill_conditioned_metrics() {
        let q = SVector::new(-1.0, 0.2, 0.1);
        let ill_conditioned = Matrix3::new(
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0e-14,
        );
        assert_eq!(
            solve_coupled_signorini_coulomb(&ill_conditioned, &q, 0.4, 1.0e-12)
                .unwrap_err(),
            CoupledCoulombError::IllConditioned
        );
    }

    #[test]
    fn rejects_nonfinite_velocity_and_invalid_friction_or_tolerance() {
        let w = Matrix3::identity();
        let finite_q = SVector::new(-1.0, 0.2, 0.1);
        let nonfinite_q = SVector::new(f64::NAN, 0.2, 0.1);
        assert_eq!(
            solve_coupled_signorini_coulomb(&w, &nonfinite_q, 0.4, 1.0e-12).unwrap_err(),
            CoupledCoulombError::NonFinite
        );
        assert_eq!(
            solve_coupled_signorini_coulomb(&w, &finite_q, -0.1, 1.0e-12).unwrap_err(),
            CoupledCoulombError::InvalidFrictionCoefficient
        );
        assert_eq!(
            solve_coupled_signorini_coulomb(&w, &finite_q, 0.4, f64::NAN).unwrap_err(),
            CoupledCoulombError::InvalidTolerance
        );
    }

    #[test]
    fn contact_frame_rotation_preserves_candidate_impulses_and_classification() {
        let w = Matrix3::new(
            3.0, 0.4, -0.2,
            0.4, 2.0, 0.5,
            -0.2, 0.5, 1.5,
        );
        let q = SVector::new(-0.8, -2.5, 0.9);
        let (sin, cos) = 0.73_f64.sin_cos();
        let tangent_rotation = Matrix2::new(cos, -sin, sin, cos);
        let transform = Matrix3::new(
            1.0, 0.0, 0.0,
            0.0, tangent_rotation[(0, 0)], tangent_rotation[(0, 1)],
            0.0, tangent_rotation[(1, 0)], tangent_rotation[(1, 1)],
        );
        let rotated_w = transform * w * transform.transpose();
        let rotated_q = transform * q;
        let direct = solve_coupled_signorini_coulomb(&w, &q, 0.22, 1.0e-12).unwrap();
        let rotated =
            solve_coupled_signorini_coulomb(&rotated_w, &rotated_q, 0.22, 1.0e-12).unwrap();
        assert_eq!(direct.status, rotated.status);
        assert_eq!(direct.candidates.len(), rotated.candidates.len());
        for candidate in &direct.candidates {
            let matched = rotated.candidates.iter().find(|other| {
                (other.impulse - transform * candidate.impulse).norm() < 1.0e-7
            });
            assert!(matched.is_some());
        }
    }

    #[test]
    fn quartic_coefficients_match_the_unreduced_angular_equation() {
        let w = Matrix3::new(
            3.0, 0.4, -0.2,
            0.4, 2.0, 0.5,
            -0.2, 0.5, 1.5,
        );
        let q = SVector::new(-0.8, -2.5, 0.9);
        let mu = 0.37;
        let coefficients = sliding_quartic(&w, &q, mu);
        for index in 0..64 {
            let theta = -std::f64::consts::PI
                + (index as f64 + 0.37) * (2.0 * std::f64::consts::PI / 64.0);
            let t = (0.5 * theta).tan();
            let polynomial = coefficients.iter().rev().fold(0.0, |value, coefficient| {
                value * t + coefficient
            });
            let (angular, _) = angular_residual(&w, &q, mu, theta);
            let factor = (1.0 + t * t).powi(2);
            let scale = polynomial.abs().max((angular * factor).abs()).max(1.0);
            assert!((polynomial - angular * factor).abs() <= 1.0e-11 * scale);
        }
    }

    #[test]
    fn grazing_with_singular_normal_coupling_is_not_guessed_through() {
        let w = Matrix3::new(
            1.0, 0.8, 0.0,
            0.8, 1.0, 0.0,
            0.0, 0.0, 1.0,
        );
        let q = SVector::new(0.0, -1.0, 0.5);
        let outcome = solve_coupled_signorini_coulomb(&w, &q, 2.0, 1.0e-12).unwrap();
        assert_eq!(outcome.status, CoupledCoulombStatus::Inconclusive);
        assert!(outcome.unique_solution().is_none());
    }

    #[test]
    fn grazing_with_positive_global_coupling_margin_is_separating() {
        let w = Matrix3::new(
            2.0, 0.1, 0.0,
            0.1, 1.0, 0.2,
            0.0, 0.2, 1.4,
        );
        let q = SVector::new(0.0, -2.0, 1.0);
        let outcome = solve_coupled_signorini_coulomb(&w, &q, 0.5, 1.0e-12).unwrap();
        assert_eq!(outcome.status, CoupledCoulombStatus::UniqueWithinTolerance);
        assert_eq!(
            outcome.unique_solution().unwrap().regime,
            CoupledCoulombRegime::Separating
        );
    }

    #[test]
    fn rejects_asymmetric_and_indefinite_effective_mass() {
        let q = SVector::new(-1.0, 0.2, 0.1);
        let asymmetric = Matrix3::new(
            1.0, 0.3, 0.0,
            0.1, 1.0, 0.0,
            0.0, 0.0, 1.0,
        );
        assert_eq!(
            solve_coupled_signorini_coulomb(&asymmetric, &q, 0.4, 1.0e-12).unwrap_err(),
            CoupledCoulombError::NotSymmetric
        );
        let indefinite = Matrix3::new(
            1.0, 2.0, 0.0,
            2.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        );
        assert_eq!(
            solve_coupled_signorini_coulomb(&indefinite, &q, 0.4, 1.0e-12).unwrap_err(),
            CoupledCoulombError::NotPositiveDefinite
        );
    }

    #[test]
    fn deterministic_spd_approach_panel_never_fabricates_a_solution() {
        for seed in 1..=128_u64 {
            let mut x = [[0.0_f64; 3]; 3];
            for i in 0..3 {
                for j in 0..3 {
                    let k = (seed * (7 + (i * 3 + j) as u64 * 11)) % 101;
                    x[i][j] = (k as f64 / 37.0) - 1.2;
                }
            }
            let m = Matrix3::new(
                x[0][0], x[0][1], x[0][2],
                x[1][0], x[1][1], x[1][2],
                x[2][0], x[2][1], x[2][2],
            );
            let w = m.transpose() * m + Matrix3::identity() * 0.3;
            let q = SVector::new(
                -0.2 - (seed % 13) as f64 / 7.0,
                (seed % 17) as f64 / 5.0 - 1.3,
                (seed % 19) as f64 / 6.0 - 1.1,
            );
            let mu = 0.05 + (seed % 11) as f64 * 0.21;
            let outcome = solve_coupled_signorini_coulomb(&w, &q, mu, 1.0e-10)
                .unwrap_or_else(|error| panic!("seed={seed}: {error:?}"));
            assert_ne!(
                outcome.status,
                CoupledCoulombStatus::Inconclusive,
                "seed={seed}, quartic={:?}, angles={:?}",
                outcome.sliding_quartic,
                outcome.sliding_angles
            );
            assert!(!outcome.candidates.is_empty(), "seed={seed}");
            for candidate in &outcome.candidates {
                assert_candidate(candidate, 2.0e-7);
            }
        }
    }
}


#[cfg(test)]
#[path = "coupled_friction_sturm.rs"]
mod sturm;
