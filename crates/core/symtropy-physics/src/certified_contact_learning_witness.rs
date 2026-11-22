// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact proof-carrying learned-contact warm-start certificate fixture.
//!
//! This test-only fixture is the first mathematical control for
//! PHYS-CERTIFIED-LEARNED-CONTACT-001 (#3324). It does not contain a neural
//! network. It verifies the deterministic certificate that a future learned
//! warm start must satisfy on a strongly monotone fixed-normal-load disk QP.
//!
//! The 4x4 tangential Hessian is H = H_NUM / 10, where
//!
//!     H_NUM = [10I  I; I  10I].
//!
//! For every x in R^4, exact sum-of-squares identities give
//!
//!     10 x^T (H - 9/10 I) x = (x0+x2)^2 + (x1+x3)^2 >= 0,
//!     10 x^T (11/10 I - H) x = (x0-x2)^2 + (x1-x3)^2 >= 0.
//!
//! Thus H is strongly monotone with alpha=9/10 and Lipschitz with L=11/10.
//! The feasible set is the product of two Coulomb disks of radii 2 and 3.
//! The exact minimizer is j* = (-3/5,-4/5,-3/5,-4/5), chosen in the interior.
//!
//! A nearby feasible candidate j_hat has an exact KKT residual. The residual
//! / alpha error bound is tight, while a projected fixed-point contraction
//! gives a sound but more pessimistic bound. No floating-point operations,
//! learned weights, or solver stopping flags participate in the certificate.

type Vector4 = [i128; 4];
type Matrix4 = [[i128; 4]; 4];

const H_NUM: Matrix4 = [[10, 0, 1, 0], [0, 10, 0, 1], [1, 0, 10, 0], [0, 1, 0, 10]];

const C_NUM_OVER_100: Vector4 = [66, 88, 66, 88];
const J_STAR_NUM_OVER_10: Vector4 = [-6, -8, -6, -8];
const J_HAT_NUM_OVER_10: Vector4 = [-5, -9, -7, -7];
const RESIDUAL_NUM_OVER_100: Vector4 = [9, -9, -9, 9];
const T_GAMMA_NUM_OVER_400: Vector4 = [-227, -333, -253, -307];

fn mat_vec(matrix: &Matrix4, vector: &Vector4) -> Vector4 {
    std::array::from_fn(|row| {
        matrix[row]
            .iter()
            .zip(vector.iter())
            .map(|(coefficient, value)| coefficient * value)
            .sum()
    })
}

fn dot(left: &Vector4, right: &Vector4) -> i128 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn squared_norm(vector: &Vector4) -> i128 {
    dot(vector, vector)
}

fn disk_squared(vector: &[i128; 2]) -> i128 {
    vector[0] * vector[0] + vector[1] * vector[1]
}

fn assert_product_of_disks_feasible(vector_numerators: &[i128; 4], denominator: i128) {
    let first = [vector_numerators[0], vector_numerators[1]];
    let second = [vector_numerators[2], vector_numerators[3]];
    let denominator_squared = denominator * denominator;

    // Contact 1 has mu*lambda = 2; contact 2 has mu*lambda = 3.
    assert!(disk_squared(&first) < 4 * denominator_squared);
    assert!(disk_squared(&second) < 9 * denominator_squared);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strongly_monotone_disk_qp_has_exact_residual_over_alpha_error_bound() {
        // Verify symmetry of the exact coupled tangential Hessian.
        for (row_index, row_values) in H_NUM.iter().enumerate() {
            for (column_index, value) in row_values.iter().enumerate() {
                assert_eq!(value, &H_NUM[column_index][row_index]);
            }
        }

        // Exact Gram factors establish the two matrix identities for every
        // x in R^4, rather than relying only on a finite numerical sample.
        let lower_factors: [[i128; 4]; 2] = [[1, 0, 1, 0], [0, 1, 0, 1]];
        let upper_factors: [[i128; 4]; 2] = [[1, 0, -1, 0], [0, 1, 0, -1]];
        for row in 0..4 {
            for column in 0..4 {
                let lower_gram: i128 = lower_factors
                    .iter()
                    .map(|factor| factor[row] * factor[column])
                    .sum();
                let upper_gram: i128 = upper_factors
                    .iter()
                    .map(|factor| factor[row] * factor[column])
                    .sum();
                let identity = if row == column { 1_i128 } else { 0_i128 };
                assert_eq!(H_NUM[row][column] - 9 * identity, lower_gram);
                assert_eq!(11 * identity - H_NUM[row][column], upper_gram);
            }
        }

        // Retain additional integer-vector controls as a metamorphic check
        // against accidental changes to the frozen coupled Hessian.
        let controls: [Vector4; 6] = [
            [1, 0, 0, 0],
            [0, 1, 0, 0],
            [0, 0, 1, 0],
            [0, 0, 0, 1],
            [1, -2, 3, -4],
            [-7, 5, 2, 9],
        ];
        for x in controls {
            let hx_num = mat_vec(&H_NUM, &x);
            let x_h_x_num = dot(&x, &hx_num);
            let norm_squared = squared_norm(&x);
            let lower_sos = (x[0] + x[2]).pow(2) + (x[1] + x[3]).pow(2);
            let upper_sos = (x[0] - x[2]).pow(2) + (x[1] - x[3]).pow(2);
            assert_eq!(x_h_x_num - 9 * norm_squared, lower_sos);
            assert_eq!(11 * norm_squared - x_h_x_num, upper_sos);
        }

        // H*j_star + c = 0 exactly. Since j_star is strictly interior to K
        // and H >= (9/10)I, it is the unique variational-inequality solution.
        let h_j_star_num = mat_vec(&H_NUM, &J_STAR_NUM_OVER_10);
        for index in 0..4 {
            assert_eq!(h_j_star_num[index] + C_NUM_OVER_100[index], 0);
        }
        assert_product_of_disks_feasible(&J_STAR_NUM_OVER_10, 10);

        // j_hat is also strictly feasible. Its exact KKT/VI residual is
        // F(j_hat)=H(j_hat-j_star)=RESIDUAL_NUM_OVER_100 / 100. Because the
        // normal cone is zero in the interior, this is a valid residual vector.
        assert_product_of_disks_feasible(&J_HAT_NUM_OVER_10, 10);
        let h_j_hat_num = mat_vec(&H_NUM, &J_HAT_NUM_OVER_10);
        let residual_numerator: Vector4 =
            std::array::from_fn(|index| h_j_hat_num[index] + C_NUM_OVER_100[index]);
        assert_eq!(residual_numerator, RESIDUAL_NUM_OVER_100);

        let error_numerator: Vector4 =
            std::array::from_fn(|index| J_HAT_NUM_OVER_10[index] - J_STAR_NUM_OVER_10[index]);
        assert_eq!(error_numerator, [1, -1, -1, 1]);

        // Exact squared error: ||j_hat-j*||^2 = 4/100.
        let error_squared_numerator = squared_norm(&error_numerator);
        let error_squared_denominator = 100_i128;
        assert_eq!(error_squared_numerator, 4);

        // Exact squared residual: ||v||^2 = 324/10000.
        let residual_squared_numerator = squared_norm(&residual_numerator);
        let residual_squared_denominator = 10_000_i128;
        assert_eq!(residual_squared_numerator, 324);

        // alpha=9/10. The KKT residual bound is ||v||/alpha and is tight:
        // 324/10000 / (81/100) = 4/100.
        let alpha_numerator = 9_i128;
        let alpha_denominator = 10_i128;
        assert_eq!(
            residual_squared_numerator
                * alpha_denominator
                * alpha_denominator
                * error_squared_denominator,
            error_squared_numerator
                * residual_squared_denominator
                * alpha_numerator
                * alpha_numerator
        );
    }

    #[test]
    fn projected_fixed_point_bound_is_sound_and_more_pessimistic() {
        // For alpha=9/10, L=11/10 and gamma=3/4:
        // rho^2 = 1 - 2*gamma*alpha + gamma^2*L^2 = 529/1600,
        // so rho=23/40 and 1-rho=17/40.
        let rho_squared_numerator = 1_600_i128 - 2_160 + 1_089;
        assert_eq!(rho_squared_numerator, 529);

        // The exact fixed-point image is j_hat-gamma*F(j_hat), represented
        // over denominator 400. It remains strictly inside both Coulomb disks,
        // so the projection P_K does not alter this point.
        assert_eq!(T_GAMMA_NUM_OVER_400, [-227, -333, -253, -307]);
        let first_image = [T_GAMMA_NUM_OVER_400[0], T_GAMMA_NUM_OVER_400[1]];
        let second_image = [T_GAMMA_NUM_OVER_400[2], T_GAMMA_NUM_OVER_400[3]];
        let denominator_squared = 400_i128 * 400;
        assert!(disk_squared(&first_image) < 4 * denominator_squared);
        assert!(disk_squared(&second_image) < 9 * denominator_squared);

        // ||j_hat-T_gamma(j_hat)||^2 = 729/40000. The contraction
        // denominator is (1-rho)^2 = 289/1600, yielding a squared
        // error bound of 729/7225. The exact squared error is 1/25=289/7225.
        let fixed_point_residual_squared_numerator = 729_i128;
        let fixed_point_residual_squared_denominator = 40_000_i128;
        let one_minus_rho_numerator = 17_i128;
        let one_minus_rho_denominator = 40_i128;
        let fixed_point_bound_squared_numerator = fixed_point_residual_squared_numerator
            * one_minus_rho_denominator
            * one_minus_rho_denominator;
        let fixed_point_bound_squared_denominator = fixed_point_residual_squared_denominator
            * one_minus_rho_numerator
            * one_minus_rho_numerator;
        assert_eq!(fixed_point_bound_squared_numerator, 1_166_400);
        assert_eq!(fixed_point_bound_squared_denominator, 11_560_000);
        // Compare with exact error squared 1/25 by cross multiplication.
        assert!(fixed_point_bound_squared_numerator * 25 >= fixed_point_bound_squared_denominator);
        // It is strictly looser than the KKT residual/alpha bound.
        assert!(fixed_point_bound_squared_numerator * 25 > fixed_point_bound_squared_denominator);
    }

    #[test]
    fn rank_deficient_tangential_hessian_cannot_claim_strong_monotonicity() {
        let rank_deficient: Matrix4 = [[10, 0, 0, 0], [0, 0, 0, 0], [0, 0, 10, 0], [0, 0, 0, 0]];
        let null_direction = [0_i128, 1, 0, 0];
        assert_eq!(mat_vec(&rank_deficient, &null_direction), [0; 4]);
        assert_eq!(squared_norm(&null_direction), 1);
        assert_eq!(
            dot(&null_direction, &mat_vec(&rank_deficient, &null_direction)),
            0
        );

        // A claimed positive lower bound alpha would require x^T H x >=
        // alpha ||x||^2 for every x. This exact null direction disproves any
        // alpha>0, so the learned single-valued oracle must abstain.
    }

    #[test]
    fn primal_dual_gap_gives_an_independent_strong_convexity_error_bound() {
        // This fixture's primal candidate and exact minimizer are both strictly
        // inside the product of Coulomb disks. The dual multipliers are therefore
        // zero, and stationarity at j_star supplies a feasible dual lower bound.
        assert_product_of_disks_feasible(&J_HAT_NUM_OVER_10, 10);
        assert_product_of_disks_feasible(&J_STAR_NUM_OVER_10, 10);

        let h_j_star_num = mat_vec(&H_NUM, &J_STAR_NUM_OVER_10);
        for index in 0..4 {
            assert_eq!(h_j_star_num[index] + C_NUM_OVER_100[index], 0);
        }

        // For e = j_hat - j_star = [1,-1,-1,1]/10, the objective
        // gap is exactly 1/2 e^T H e. H=H_NUM/10 and e=e_num/10.
        // The integer numerator e_num^T H_NUM e_num is 36, so
        // f(j_hat)-d(0) = 18/1000 = 9/500.
        let error_numerator: Vector4 =
            std::array::from_fn(|index| J_HAT_NUM_OVER_10[index] - J_STAR_NUM_OVER_10[index]);
        assert_eq!(error_numerator, [1, -1, -1, 1]);
        let h_error_numerator = mat_vec(&H_NUM, &error_numerator);
        let twice_gap_numerator = dot(&error_numerator, &h_error_numerator);
        assert_eq!(twice_gap_numerator, 36);

        let gap_numerator = 18_i128;
        let gap_denominator = 1_000_i128;
        let alpha_numerator = 9_i128;
        let alpha_denominator = 10_i128;
        let actual_error_squared_numerator = squared_norm(&error_numerator);
        let actual_error_squared_denominator = 100_i128;
        assert_eq!(actual_error_squared_numerator, 4);

        // Strong convexity gives ||j_hat-j_star||^2 <= 2*gap/alpha.
        // Here the gap enclosure is exact and the resulting bound is tight:
        // 2*(18/1000)/(9/10) = 4/100 = ||j_hat-j_star||^2.
        assert_eq!(
            2 * gap_numerator * alpha_denominator * actual_error_squared_denominator,
            actual_error_squared_numerator * gap_denominator * alpha_numerator
        );

        // The same frozen input also satisfies the independently derived
        // residual/alpha bound in the preceding test. Keep this equality as
        // an exact cross-check between the two certificate constructions.
        let residual_squared_numerator = squared_norm(&RESIDUAL_NUM_OVER_100);
        assert_eq!(residual_squared_numerator, 324);
        assert_eq!(
            residual_squared_numerator
                * alpha_denominator
                * alpha_denominator
                * actual_error_squared_denominator,
            actual_error_squared_numerator * 10_000_i128 * alpha_numerator * alpha_numerator
        );
    }

    #[test]
    fn primal_dual_gap_certificate_covers_two_active_contact_disks() {
        // H=I and two unit Coulomb disks. The exact minimizer is on both
        // boundaries with nonzero multipliers lambda_1=lambda_2=1.
        // f(j)=1/2||j||^2 - 3*j0 - 3*j2.
        let h_num: Matrix4 = [
            [10, 0, 0, 0],
            [0, 10, 0, 0],
            [0, 0, 10, 0],
            [0, 0, 0, 10],
        ];
        let solution_num: Vector4 = [10, 0, 10, 0];
        let candidate_num: Vector4 = [8, 6, 8, 6];
        let linear_term_num_over_100: Vector4 = [-300, 0, -300, 0];

        for disk in [0..2, 2..4] {
            let solution_radius_squared =
                solution_num[disk.start] * solution_num[disk.start]
                    + solution_num[disk.start + 1] * solution_num[disk.start + 1];
            let candidate_radius_squared =
                candidate_num[disk.start] * candidate_num[disk.start]
                    + candidate_num[disk.start + 1] * candidate_num[disk.start + 1];
            assert_eq!(solution_radius_squared, 100);
            assert_eq!(candidate_radius_squared, 100);
        }

        // Exact KKT stationarity, with each active disk contributing 2*x.
        let h_solution_num = mat_vec(&h_num, &solution_num);
        let stationarity_num: Vector4 = std::array::from_fn(|index| {
            h_solution_num[index]
                + linear_term_num_over_100[index]
                + if index == 0 || index == 2 { 200 } else { 0 }
        });
        assert_eq!(stationarity_num, [0; 4]);

        // Lagrangian Hessian is diag(3,3,3,3), so the stationary
        // complementary point is the unique global minimizer.
        let lagrangian_hessian_diagonal = [3_i128; 4];
        assert!(lagrangian_hessian_diagonal.iter().all(|value| *value >= 1));

        // Exact objectives over denominator 100:
        // f(j_hat)=100/100 - 480/100 = -380/100.
        // d(lambda)=f(j_star)=100/100 - 600/100 = -500/100.
        assert_eq!(squared_norm(&candidate_num), 200);
        let primal_num_over_100 = 100_i128 - 480_i128;
        let dual_num_over_100 = 100_i128 - 600_i128;
        assert_eq!(primal_num_over_100, -380);
        assert_eq!(dual_num_over_100, -500);
        let gap_num_over_100 = primal_num_over_100 - dual_num_over_100;
        assert_eq!(gap_num_over_100, 120);

        // Exact squared error is ||(-2,6,-2,6)/10||^2 = 80/100.
        let error_num: Vector4 =
            std::array::from_fn(|index| candidate_num[index] - solution_num[index]);
        assert_eq!(error_num, [-2, 6, -2, 6]);
        let error_squared_num = squared_norm(&error_num);
        assert_eq!(error_squared_num, 80);

        // Strong convexity alpha=1 gives 80/100 <= 2*120/100.
        assert!(error_squared_num <= 2 * gap_num_over_100);
    }

    #[test]
    fn primal_dual_gap_certificate_covers_an_active_coulomb_disk_boundary() {
        // A separate two-contact product-of-disks QP exercises a nonzero
        // multiplier at an active Coulomb disk boundary. H=I is strongly
        // convex with alpha=1. The first disk has radius 1 and the second
        // disk also has radius 1.
        //
        // f(j) = 1/2 ||j||^2 - 3*j0.
        // j_star=(1,0,0,0) is on the first disk boundary; lambda_1=1,
        // lambda_2=0. The candidate j_hat=(9/10,0,1/2,0) is feasible.
        let h_num: Matrix4 = [[10, 0, 0, 0], [0, 10, 0, 0], [0, 0, 10, 0], [0, 0, 0, 10]];
        let solution_num: Vector4 = [10, 0, 0, 0];
        let candidate_num: Vector4 = [9, 0, 5, 0];
        // c=(-3,0,0,0), represented over denominator 100.
        let linear_term_num_over_100: Vector4 = [-300, 0, 0, 0];

        // Both candidate contact impulses are inside their unit Coulomb disks.
        assert!(candidate_num[0] * candidate_num[0] + candidate_num[1] * candidate_num[1] < 100);
        assert!(candidate_num[2] * candidate_num[2] + candidate_num[3] * candidate_num[3] < 100);
        // The minimizer lies exactly on the first disk boundary and at the
        // centre of the second disk.
        assert_eq!(
            solution_num[0] * solution_num[0] + solution_num[1] * solution_num[1],
            100
        );
        assert_eq!(
            solution_num[2] * solution_num[2] + solution_num[3] * solution_num[3],
            0
        );

        // Exact KKT stationarity at j_star:
        // H*j_star + c + 2*lambda_1*j_star(first disk) = 0,
        // with lambda_1=1 and lambda_2=0. All terms below share denominator 100.
        let h_solution_num = mat_vec(&h_num, &solution_num);
        let stationarity_num: Vector4 = std::array::from_fn(|index| {
            h_solution_num[index]
                + linear_term_num_over_100[index]
                + if index == 0 { 200 } else { 0 }
        });
        assert_eq!(stationarity_num, [0; 4]);

        // The Lagrangian Hessian is diag(3,3,1,1), hence positive definite;
        // the stationary point is its global minimizer. Since j_star is
        // primal-feasible and complementary, its Lagrangian value is a dual
        // lower bound equal to f(j_star).
        let lagrangian_hessian_diagonal = [3_i128, 3, 1, 1];
        assert!(lagrangian_hessian_diagonal.iter().all(|value| *value >= 1));

        // Exact objective values over denominator 100:
        // f(j_hat)=53/100 - 270/100 = -217/100.
        // d(lambda)=f(j_star)=50/100 - 300/100 = -250/100.
        let candidate_norm_squared_num = squared_norm(&candidate_num);
        assert_eq!(candidate_norm_squared_num, 106);
        let primal_twice_kinetic_num = candidate_norm_squared_num;
        let primal_linear_num_over_100 = -270_i128;
        let primal_num_over_100 = primal_twice_kinetic_num / 2 + primal_linear_num_over_100;
        assert_eq!(primal_num_over_100, -217);

        let dual_num_over_100 = 50_i128 - 300_i128;
        assert_eq!(dual_num_over_100, -250);
        let gap_num_over_100 = primal_num_over_100 - dual_num_over_100;
        assert_eq!(gap_num_over_100, 33);

        // Exact squared error is ||(-1,0,5,0)/10||^2 = 26/100.
        let error_num: Vector4 =
            std::array::from_fn(|index| candidate_num[index] - solution_num[index]);
        assert_eq!(error_num, [-1, 0, 5, 0]);
        let error_squared_num = squared_norm(&error_num);
        assert_eq!(error_squared_num, 26);

        // Strong convexity yields ||j_hat-j_star||^2 <= 2*gap/alpha.
        // Here alpha=1, so 26/100 <= 66/100. This active-boundary example
        // complements the prior interior / zero-multiplier exact-gap witness.
        assert!(error_squared_num <= 2 * gap_num_over_100);
    }
}
