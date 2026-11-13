// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Rank-aware PSD Delassus analysis for small dense linear holonomic systems.
//!
//! The solver distinguishes a multiplier fibre from the unique generalized
//! response. The minimum dual-metric-norm multiplier is a declared gauge,
//! never a claim of physical multiplier uniqueness.

use std::fmt;

pub type Matrix<const R: usize, const C: usize> = [[f64; C]; R];
pub type Vector<const N: usize> = [f64; N];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RankProfileV1 {
    pub relative_rank_tolerance: f64,
    pub absolute_rank_tolerance: f64,
    pub near_rank_band_factor: f64,
    pub relative_psd_tolerance: f64,
    pub absolute_psd_tolerance: f64,
    pub relative_range_tolerance: f64,
    pub absolute_range_tolerance: f64,
    pub relative_pivot_tolerance: f64,
    pub jacobi_max_sweeps: usize,
}

impl Default for RankProfileV1 {
    fn default() -> Self {
        Self {
            relative_rank_tolerance: 1.0e-10,
            absolute_rank_tolerance: 1.0e-14,
            near_rank_band_factor: 4.0,
            relative_psd_tolerance: 1.0e-11,
            absolute_psd_tolerance: 1.0e-14,
            relative_range_tolerance: 1.0e-10,
            absolute_range_tolerance: 1.0e-13,
            relative_pivot_tolerance: 64.0 * f64::EPSILON,
            jacobi_max_sweeps: 128,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RankErrorV1 {
    ZeroDimension,
    InvalidProfile,
    NonFiniteInput,
    NonFiniteIntermediate,
    NonSymmetric {
        row: usize,
        column: usize,
        defect: f64,
    },
    NotPositiveDefinite {
        pivot: usize,
        value: f64,
    },
    SingularPivot {
        column: usize,
        magnitude: f64,
        threshold: f64,
    },
    NonPositiveSemidefinite {
        eigenvalue: f64,
        tolerance: f64,
    },
    JacobiDidNotConverge {
        residual_off_diagonal: f64,
    },
    InconclusiveRank {
        eigenvalue: f64,
        cutoff: f64,
    },
    InvalidExactRankWitness,
    ExactWitnessArithmeticOverflow,
}

impl fmt::Display for RankErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RankErrorV1 {}

fn validate_profile(profile: RankProfileV1) -> Result<(), RankErrorV1> {
    if !profile.relative_rank_tolerance.is_finite()
        || profile.relative_rank_tolerance <= 0.0
        || !profile.absolute_rank_tolerance.is_finite()
        || profile.absolute_rank_tolerance < 0.0
        || !profile.near_rank_band_factor.is_finite()
        || profile.near_rank_band_factor <= 1.0
        || !profile.relative_psd_tolerance.is_finite()
        || profile.relative_psd_tolerance <= 0.0
        || !profile.absolute_psd_tolerance.is_finite()
        || profile.absolute_psd_tolerance < 0.0
        || !profile.relative_range_tolerance.is_finite()
        || profile.relative_range_tolerance <= 0.0
        || !profile.absolute_range_tolerance.is_finite()
        || profile.absolute_range_tolerance < 0.0
        || !profile.relative_pivot_tolerance.is_finite()
        || profile.relative_pivot_tolerance <= 0.0
        || profile.jacobi_max_sweeps == 0
    {
        return Err(RankErrorV1::InvalidProfile);
    }
    Ok(())
}

fn finite_matrix<const R: usize, const C: usize>(a: &Matrix<R, C>) -> bool {
    a.iter().flatten().all(|x| x.is_finite())
}
fn transpose<const R: usize, const C: usize>(a: &Matrix<R, C>) -> Matrix<C, R> {
    let mut out = [[0.0; R]; C];
    for (i, row) in a.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            out[j][i] = *value;
        }
    }
    out
}
fn multiply<const R: usize, const K: usize, const C: usize>(
    a: &Matrix<R, K>,
    b: &Matrix<K, C>,
) -> Matrix<R, C> {
    let mut out = [[0.0; C]; R];
    for (i, row) in a.iter().enumerate() {
        for (k, left) in row.iter().enumerate() {
            for (j, right) in b[k].iter().enumerate() {
                out[i][j] += left * right;
            }
        }
    }
    out
}
fn mat_vec<const R: usize, const C: usize>(a: &Matrix<R, C>, x: &Vector<C>) -> Vector<R> {
    let mut out = [0.0; R];
    for (i, row) in a.iter().enumerate() {
        out[i] = row
            .iter()
            .zip(x.iter())
            .map(|(left, right)| left * right)
            .sum();
    }
    out
}
fn dot<const N: usize>(a: &Vector<N>, b: &Vector<N>) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}
fn norm<const N: usize>(a: &Vector<N>) -> f64 {
    a.iter().fold(0.0_f64, |accumulator, value| accumulator.hypot(*value))
}
fn matrix_max_abs<const R: usize, const C: usize>(a: &Matrix<R, C>) -> f64 {
    a.iter().flatten().map(|x| x.abs()).fold(0.0_f64, f64::max)
}

fn cholesky<const N: usize>(a: &Matrix<N, N>) -> Result<Matrix<N, N>, RankErrorV1> {
    if N == 0 {
        return Err(RankErrorV1::ZeroDimension);
    }
    if !finite_matrix(a) {
        return Err(RankErrorV1::NonFiniteInput);
    }
    let mut lower = [[0.0; N]; N];
    for i in 0..N {
        for j in 0..=i {
            let correction = (0..j).map(|k| lower[i][k] * lower[j][k]).sum::<f64>();
            if i == j {
                let pivot = a[i][i] - correction;
                if !pivot.is_finite() {
                    return Err(RankErrorV1::NonFiniteIntermediate);
                }
                if pivot <= 0.0 {
                    return Err(RankErrorV1::NotPositiveDefinite {
                        pivot: i,
                        value: pivot,
                    });
                }
                lower[i][j] = pivot.sqrt();
            } else {
                lower[i][j] = (a[i][j] - correction) / lower[j][j];
                if !lower[i][j].is_finite() {
                    return Err(RankErrorV1::NonFiniteIntermediate);
                }
            }
        }
    }
    Ok(lower)
}

fn solve<const N: usize>(
    a: &Matrix<N, N>,
    rhs: &Vector<N>,
    profile: RankProfileV1,
) -> Result<Vector<N>, RankErrorV1> {
    if N == 0 {
        return Err(RankErrorV1::ZeroDimension);
    }
    if !finite_matrix(a) || rhs.iter().any(|x| !x.is_finite()) {
        return Err(RankErrorV1::NonFiniteInput);
    }
    let scale = matrix_max_abs(a);
    if scale == 0.0 {
        return Err(RankErrorV1::SingularPivot {
            column: 0,
            magnitude: 0.0,
            threshold: 0.0,
        });
    }
    let threshold = scale * profile.relative_pivot_tolerance;
    let mut lu = *a;
    let mut b = *rhs;
    for col in 0..N {
        let pivot_row = (col..N)
            .max_by(|r, s| lu[*r][col].abs().total_cmp(&lu[*s][col].abs()))
            .ok_or(RankErrorV1::ZeroDimension)?;
        let pivot = lu[pivot_row][col].abs();
        if !pivot.is_finite() || pivot <= threshold {
            return Err(RankErrorV1::SingularPivot {
                column: col,
                magnitude: pivot,
                threshold,
            });
        }
        if pivot_row != col {
            lu.swap(col, pivot_row);
            b.swap(col, pivot_row);
        }
        for row in (col + 1)..N {
            lu[row][col] /= lu[col][col];
            for k in (col + 1)..N {
                lu[row][k] -= lu[row][col] * lu[col][k];
            }
            b[row] -= lu[row][col] * b[col];
        }
    }
    let mut x = [0.0; N];
    for row in (0..N).rev() {
        let tail = ((row + 1)..N).map(|col| lu[row][col] * x[col]).sum::<f64>();
        x[row] = (b[row] - tail) / lu[row][row];
    }
    if x.iter().any(|v| !v.is_finite()) {
        return Err(RankErrorV1::NonFiniteIntermediate);
    }
    Ok(x)
}

#[cfg(test)]
fn inverse<const N: usize>(
    a: &Matrix<N, N>,
    profile: RankProfileV1,
) -> Result<Matrix<N, N>, RankErrorV1> {
    let mut out = [[0.0; N]; N];
    for col in 0..N {
        let mut unit = [0.0; N];
        unit[col] = 1.0;
        let solution = solve(a, &unit, profile)?;
        for row in 0..N {
            out[row][col] = solution[row];
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpdMetricV1<const N: usize> {
    matrix: Matrix<N, N>,
    lower: Matrix<N, N>,
}

impl<const N: usize> SpdMetricV1<N> {
    pub fn new(matrix: Matrix<N, N>, symmetry_tolerance: f64) -> Result<Self, RankErrorV1> {
        if N == 0 {
            return Err(RankErrorV1::ZeroDimension);
        }
        if !symmetry_tolerance.is_finite() || symmetry_tolerance < 0.0 {
            return Err(RankErrorV1::InvalidProfile);
        }
        if !finite_matrix(&matrix) {
            return Err(RankErrorV1::NonFiniteInput);
        }
        let symmetry_scale = matrix_max_abs(&matrix);
        for (i, row) in matrix.iter().enumerate() {
            for (j, value) in row.iter().enumerate().skip(i + 1) {
                let defect = (*value - matrix[j][i]).abs();
                if defect > symmetry_tolerance * symmetry_scale {
                    return Err(RankErrorV1::NonSymmetric {
                        row: i,
                        column: j,
                        defect,
                    });
                }
            }
        }
        let lower = cholesky(&matrix)?;
        Ok(Self { matrix, lower })
    }

    pub fn matrix(&self) -> &Matrix<N, N> {
        &self.matrix
    }

    pub fn lower(&self) -> &Matrix<N, N> {
        &self.lower
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EigenSystemV1<const K: usize> {
    values: Vector<K>,
    vectors: Matrix<K, K>,
    residual_off_diagonal: f64,
}

fn jacobi_eigen<const K: usize>(
    matrix: &Matrix<K, K>,
    profile: RankProfileV1,
) -> Result<EigenSystemV1<K>, RankErrorV1> {
    if K == 0 {
        return Err(RankErrorV1::ZeroDimension);
    }
    if !finite_matrix(matrix) {
        return Err(RankErrorV1::NonFiniteInput);
    }
    let mut a = *matrix;
    let mut vectors = [[0.0; K]; K];
    for (i, row) in vectors.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    // Use the operator's actual scale. A unit floor makes the Jacobi stopping
    // rule representation-dependent for small but otherwise well-scaled PSD
    // operators, and can falsely declare their off-diagonal terms converged.
    let scale = matrix_max_abs(matrix);
    let convergence_threshold = 32.0 * f64::EPSILON * scale;
    let mut residual = f64::INFINITY;
    for _ in 0..profile.jacobi_max_sweeps.saturating_mul(K.max(1)) {
        let mut p = 0usize;
        let mut q = 0usize;
        let mut largest = 0.0_f64;
        for (i, row) in a.iter().enumerate() {
            for (j, value) in row.iter().enumerate().skip(i + 1) {
                if value.abs() > largest {
                    largest = value.abs();
                    p = i;
                    q = j;
                }
            }
        }
        residual = largest;
        if largest <= convergence_threshold {
            break;
        }
        let apq = a[p][q];
        if apq == 0.0 {
            continue;
        }
        let tau = (a[q][q] - a[p][p]) / (2.0 * apq);
        let tau_hypotenuse = tau.hypot(1.0);
        let t = if tau >= 0.0 {
            1.0 / (tau + tau_hypotenuse)
        } else {
            -1.0 / (-tau + tau_hypotenuse)
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        let app = a[p][p];
        let aqq = a[q][q];
        a[p][p] = app - t * apq;
        a[q][q] = aqq + t * apq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for k in 0..K {
            if k != p && k != q {
                let akp = a[k][p];
                let akq = a[k][q];
                a[k][p] = c * akp - s * akq;
                a[p][k] = a[k][p];
                a[k][q] = s * akp + c * akq;
                a[q][k] = a[k][q];
            }
            let vkp = vectors[k][p];
            let vkq = vectors[k][q];
            vectors[k][p] = c * vkp - s * vkq;
            vectors[k][q] = s * vkp + c * vkq;
        }
    }
    if residual > convergence_threshold {
        return Err(RankErrorV1::JacobiDidNotConverge {
            residual_off_diagonal: residual,
        });
    }

    let values_unsorted: Vector<K> = std::array::from_fn(|i| a[i][i]);
    let mut order: Vec<usize> = (0..K).collect();
    order.sort_by(|left, right| values_unsorted[*left].total_cmp(&values_unsorted[*right]));
    let values = std::array::from_fn(|i| values_unsorted[order[i]]);
    let sorted_vectors =
        std::array::from_fn(|row| std::array::from_fn(|col| vectors[row][order[col]]));
    if !values.iter().all(|x| x.is_finite()) || !finite_matrix(&sorted_vectors) {
        return Err(RankErrorV1::NonFiniteIntermediate);
    }
    Ok(EigenSystemV1 {
        values,
        vectors: sorted_vectors,
        residual_off_diagonal: residual,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankStatusV1 {
    FullRowRank,
    RankDeficient,
    InconclusiveNearThreshold,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RankReceiptV1<const K: usize> {
    pub row_count: usize,
    pub dof_count: usize,
    pub rank: Option<usize>,
    pub nullity: Option<usize>,
    pub eigenvalues: Vector<K>,
    pub rank_cutoff: f64,
    pub psd_tolerance: f64,
    pub delassus_symmetry_defect: f64,
    pub jacobi_residual_off_diagonal: f64,
    pub condition_nonzero_spectrum: Option<f64>,
    pub condition_full_operator: Option<f64>,
    pub status: RankStatusV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeStatusV1 {
    Consistent,
    Inconsistent,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintSolveReceiptV1<const N: usize, const K: usize> {
    pub rank: RankReceiptV1<K>,
    pub range_status: RangeStatusV1,
    pub relative_range_defect: f64,
    /// Minimum norm in the declared dual constraint metric. This is a gauge.
    pub canonical_multiplier: Vector<K>,
    /// Columns 0..multiplier_nullity are a basis of ker(W).
    pub multiplier_nullspace: Matrix<K, K>,
    pub multiplier_nullity: usize,
    pub generalized_constraint_force: Vector<N>,
    pub minimum_kinetic_velocity_correction: Vector<N>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstraintSolveOutcomeV1<const N: usize, const K: usize> {
    Compatible(ConstraintSolveReceiptV1<N, K>),
    Inconsistent {
        rank: RankReceiptV1<K>,
        relative_range_defect: f64,
    },
    Inconclusive {
        rank: RankReceiptV1<K>,
        relative_range_defect: Option<f64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RankAwareDelassusV1<const N: usize, const K: usize> {
    mass: SpdMetricV1<N>,
    jacobian: Matrix<K, N>,
    constraint_metric: SpdMetricV1<K>,
    delassus: Matrix<K, K>,
    normalized_delassus: Matrix<K, K>,
    eigen: EigenSystemV1<K>,
    rank_receipt: RankReceiptV1<K>,
    active: [bool; K],
    profile: RankProfileV1,
}

impl<const N: usize, const K: usize> RankAwareDelassusV1<N, K> {
    pub fn new(
        mass: SpdMetricV1<N>,
        jacobian: Matrix<K, N>,
        constraint_metric: SpdMetricV1<K>,
        profile: RankProfileV1,
    ) -> Result<Self, RankErrorV1> {
        validate_profile(profile)?;
        if N == 0 || K == 0 {
            return Err(RankErrorV1::ZeroDimension);
        }
        if !finite_matrix(&jacobian) {
            return Err(RankErrorV1::NonFiniteInput);
        }

        let mut delassus = [[0.0; K]; K];
        // Column j of J^T is row j of J, with N generalized-velocity
        // components. Solve M x_j = J_j^T before forming J_i x_j.
        for (column, constraint_row) in jacobian.iter().enumerate() {
            let mass_inverse_jt_column = solve(&mass.matrix, constraint_row, profile)?;
            for (row, row_values) in jacobian.iter().enumerate() {
                delassus[row][column] = dot(row_values, &mass_inverse_jt_column);
            }
        }
        // Measure symmetry defect before canonicalization; never silently
        // turn a materially asymmetric assembly into a symmetric operator.
        let delassus_scale = matrix_max_abs(&delassus);
        let mut delassus_symmetry_defect = 0.0_f64;
        let mut defect_pair = (0usize, 0usize);
        for (i, row) in delassus.iter().enumerate() {
            for (j, value) in row.iter().enumerate().skip(i + 1) {
                let defect = (*value - delassus[j][i]).abs();
                if defect > delassus_symmetry_defect {
                    delassus_symmetry_defect = defect;
                    defect_pair = (i, j);
                }
            }
        }
        let symmetry_limit =
            profile.absolute_psd_tolerance + profile.relative_psd_tolerance * delassus_scale;
        if !delassus_symmetry_defect.is_finite() || delassus_symmetry_defect > symmetry_limit {
            return Err(RankErrorV1::NonSymmetric {
                row: defect_pair.0,
                column: defect_pair.1,
                defect: delassus_symmetry_defect,
            });
        }
        let symmetric_delassus = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                if i == j {
                    delassus[i][i]
                } else {
                    0.5 * (delassus[i][j] + delassus[j][i])
                }
            })
        });
        delassus = symmetric_delassus;
        if !finite_matrix(&delassus) {
            return Err(RankErrorV1::NonFiniteIntermediate);
        }

        // G = L L^T; W_hat = L^T W L. Canonicalize only final roundoff.
        let lower = constraint_metric.lower;
        let mut normalized_delassus = multiply(&multiply(&transpose(&lower), &delassus), &lower);
        if !finite_matrix(&normalized_delassus) {
            return Err(RankErrorV1::NonFiniteIntermediate);
        }
        let symmetric_normalized = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                if i == j {
                    normalized_delassus[i][i]
                } else {
                    0.5 * (normalized_delassus[i][j] + normalized_delassus[j][i])
                }
            })
        });
        normalized_delassus = symmetric_normalized;
        let eigen = jacobi_eigen(&normalized_delassus, profile)?;
        let max_eigen = eigen.values.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
        let rank_cutoff =
            profile.absolute_rank_tolerance + profile.relative_rank_tolerance * max_eigen;
        let psd_tolerance =
            profile.absolute_psd_tolerance + profile.relative_psd_tolerance * max_eigen;
        if !rank_cutoff.is_finite() || !psd_tolerance.is_finite() {
            return Err(RankErrorV1::NonFiniteIntermediate);
        }

        let mut active = [false; K];
        let mut rank = 0usize;
        let mut near_threshold = false;
        for value in eigen.values {
            if value < -psd_tolerance {
                return Err(RankErrorV1::NonPositiveSemidefinite {
                    eigenvalue: value,
                    tolerance: psd_tolerance,
                });
            }
            if value.abs() <= rank_cutoff / profile.near_rank_band_factor {
                continue;
            }
            if value <= rank_cutoff * profile.near_rank_band_factor {
                near_threshold = true;
            } else {
                rank += 1;
            }
        }
        if !near_threshold {
            // Reconstruct active flags in sorted eigenvalue order.
            for (i, value) in eigen.values.iter().enumerate() {
                active[i] = *value > rank_cutoff * profile.near_rank_band_factor;
            }
        }

        let status = if near_threshold {
            RankStatusV1::InconclusiveNearThreshold
        } else if rank == K {
            RankStatusV1::FullRowRank
        } else {
            RankStatusV1::RankDeficient
        };
        let nullity = if near_threshold { None } else { Some(K - rank) };
        let min_nonzero = if near_threshold || rank == 0 {
            None
        } else {
            Some(
                eigen
                    .values
                    .iter()
                    .filter(|v| **v > rank_cutoff * profile.near_rank_band_factor)
                    .copied()
                    .fold(f64::INFINITY, f64::min),
            )
        };
        let max_nonzero = if near_threshold || rank == 0 {
            None
        } else {
            Some(
                eigen
                    .values
                    .iter()
                    .filter(|v| **v > rank_cutoff * profile.near_rank_band_factor)
                    .copied()
                    .fold(0.0_f64, f64::max),
            )
        };
        let condition_nonzero_spectrum = match (min_nonzero, max_nonzero) {
            (Some(min), Some(max)) if min > 0.0 => Some(max / min),
            _ => None,
        };
        let condition_full_operator = if status == RankStatusV1::FullRowRank {
            condition_nonzero_spectrum
        } else {
            None
        };
        let rank_receipt = RankReceiptV1 {
            row_count: K,
            dof_count: N,
            rank: if near_threshold { None } else { Some(rank) },
            nullity,
            eigenvalues: eigen.values,
            rank_cutoff,
            psd_tolerance,
            delassus_symmetry_defect,
            jacobi_residual_off_diagonal: eigen.residual_off_diagonal,
            condition_nonzero_spectrum,
            condition_full_operator,
            status,
        };

        Ok(Self {
            mass,
            jacobian,
            constraint_metric,
            delassus,
            normalized_delassus,
            eigen,
            rank_receipt,
            active,
            profile,
        })
    }

    pub fn jacobian(&self) -> &Matrix<K, N> {
        &self.jacobian
    }

    pub fn delassus(&self) -> &Matrix<K, K> {
        &self.delassus
    }

    pub fn normalized_delassus(&self) -> &Matrix<K, K> {
        &self.normalized_delassus
    }

    pub fn rank_receipt(&self) -> RankReceiptV1<K> {
        self.rank_receipt
    }

    pub fn analyze_rhs(
        &self,
        rhs: Vector<K>,
    ) -> Result<ConstraintSolveOutcomeV1<N, K>, RankErrorV1> {
        let profile = self.profile;
        if self.rank_receipt.status == RankStatusV1::InconclusiveNearThreshold {
            return Ok(ConstraintSolveOutcomeV1::Inconclusive {
                rank: self.rank_receipt,
                relative_range_defect: None,
            });
        }
        if rhs.iter().any(|x| !x.is_finite()) {
            return Err(RankErrorV1::NonFiniteInput);
        }
        // b_hat = L^T b, because G = L L^T is the residual-space metric.
        let lower = self.constraint_metric.lower;
        let rhs_hat = mat_vec(&transpose(&lower), &rhs);
        let rhs_norm = norm(&rhs_hat);
        // ||r|| <= atol + rtol ||b||, expressed as a relative defect.
        // Dividing atol by ||b|| makes the decision invariant to common scaling.
        let range_tolerance = profile.relative_range_tolerance
            + profile.absolute_range_tolerance / rhs_norm.max(1.0e-300);
        if !range_tolerance.is_finite() {
            return Ok(ConstraintSolveOutcomeV1::Inconclusive {
                rank: self.rank_receipt,
                relative_range_defect: None,
            });
        }

        let mut coefficients = [0.0; K];
        let mut range_residual_hat = [0.0; K];
        for (eig_index, coefficient_out) in coefficients.iter_mut().enumerate() {
            let coefficient = self
                .eigen
                .vectors
                .iter()
                .zip(rhs_hat.iter())
                .map(|(basis_row, rhs_component)| basis_row[eig_index] * rhs_component)
                .sum::<f64>();
            *coefficient_out = coefficient;
            if !self.active[eig_index] {
                for (residual_component, basis_row) in
                    range_residual_hat.iter_mut().zip(self.eigen.vectors.iter())
                {
                    *residual_component += coefficient * basis_row[eig_index];
                }
            }
        }
        let range_defect = norm(&range_residual_hat) / rhs_norm.max(1.0e-300);
        if !range_defect.is_finite() {
            return Err(RankErrorV1::NonFiniteIntermediate);
        }
        if range_defect > range_tolerance * profile.near_rank_band_factor {
            return Ok(ConstraintSolveOutcomeV1::Inconsistent {
                rank: self.rank_receipt,
                relative_range_defect: range_defect,
            });
        }
        if range_defect > range_tolerance / profile.near_rank_band_factor {
            return Ok(ConstraintSolveOutcomeV1::Inconclusive {
                rank: self.rank_receipt,
                relative_range_defect: Some(range_defect),
            });
        }

        // z = lambda_hat is the minimum Euclidean-norm solution of
        // W_hat z = b_hat. lambda = L z is the min dual-G-norm gauge.
        let mut lambda_hat = [0.0; K];
        let mut nullspace_hat = [[0.0; K]; K];
        let mut nullity = 0usize;
        for (eig_index, coefficient) in coefficients.iter().enumerate() {
            if self.active[eig_index] {
                let scale = *coefficient / self.eigen.values[eig_index];
                for (lambda_component, basis_row) in
                    lambda_hat.iter_mut().zip(self.eigen.vectors.iter())
                {
                    *lambda_component += scale * basis_row[eig_index];
                }
            } else {
                for (nullspace_row, basis_row) in
                    nullspace_hat.iter_mut().zip(self.eigen.vectors.iter())
                {
                    nullspace_row[nullity] = basis_row[eig_index];
                }
                nullity += 1;
            }
        }
        let canonical_multiplier = mat_vec(&lower, &lambda_hat);
        let multiplier_nullspace = multiply(&lower, &nullspace_hat);
        let jt = transpose(&self.jacobian);
        let generalized_constraint_force = mat_vec(&jt, &canonical_multiplier);
        let minimum_kinetic_velocity_correction =
            solve(&self.mass.matrix, &generalized_constraint_force, profile)?;
        if !canonical_multiplier.iter().all(|x| x.is_finite())
            || !generalized_constraint_force.iter().all(|x| x.is_finite())
            || !minimum_kinetic_velocity_correction
                .iter()
                .all(|x| x.is_finite())
        {
            return Err(RankErrorV1::NonFiniteIntermediate);
        }

        Ok(ConstraintSolveOutcomeV1::Compatible(
            ConstraintSolveReceiptV1 {
                rank: self.rank_receipt,
                range_status: RangeStatusV1::Consistent,
                relative_range_defect: range_defect,
                canonical_multiplier,
                multiplier_nullspace,
                multiplier_nullity: nullity,
                generalized_constraint_force,
                minimum_kinetic_velocity_correction,
            },
        ))
    }
}

/// Verify a rank-two exact-rational witness for an integer 3x3 Jacobian:
/// rows 0 and 1 have a nonzero 2x2 minor and row 2 = row 0 + row 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactRankTwoWitnessV1 {
    pub jacobian: [[i64; 3]; 3],
    pub left_null_vector: [i64; 3],
    pub independent_minor_determinant: i64,
}

impl ExactRankTwoWitnessV1 {
    pub fn verify(self) -> Result<(), RankErrorV1> {
        let j = self.jacobian;
        let n = self.left_null_vector;
        if n.iter().all(|value| *value == 0) {
            return Err(RankErrorV1::InvalidExactRankWitness);
        }
        for (col, _) in j[0].iter().enumerate() {
            let mut value = 0_i64;
            for (row, coefficient) in j.iter().zip(n.iter()) {
                let term = coefficient
                    .checked_mul(row[col])
                    .ok_or(RankErrorV1::ExactWitnessArithmeticOverflow)?;
                value = value
                    .checked_add(term)
                    .ok_or(RankErrorV1::ExactWitnessArithmeticOverflow)?;
            }
            if value != 0 {
                return Err(RankErrorV1::InvalidExactRankWitness);
            }
        }
        let left = j[0][0]
            .checked_mul(j[1][1])
            .ok_or(RankErrorV1::ExactWitnessArithmeticOverflow)?;
        let right = j[0][1]
            .checked_mul(j[1][0])
            .ok_or(RankErrorV1::ExactWitnessArithmeticOverflow)?;
        let determinant = left
            .checked_sub(right)
            .ok_or(RankErrorV1::ExactWitnessArithmeticOverflow)?;
        if determinant == 0 || determinant != self.independent_minor_determinant {
            return Err(RankErrorV1::InvalidExactRankWitness);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> RankProfileV1 {
        RankProfileV1::default()
    }

    fn mass() -> SpdMetricV1<3> {
        SpdMetricV1::new([[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 4.0]], 1.0e-14).unwrap()
    }

    fn identity_constraint_metric() -> SpdMetricV1<3> {
        SpdMetricV1::new([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], 1.0e-14).unwrap()
    }

    fn redundant_j() -> Matrix<3, 3> {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]]
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance * expected.abs().max(1.0),
            "actual={actual:.17e}, expected={expected:.17e}, tolerance={tolerance:.3e}"
        );
    }

    fn mat3_mul(a: &Matrix<3, 3>, b: &Matrix<3, 3>) -> Matrix<3, 3> {
        multiply(a, b)
    }

    #[test]
    fn metric_symmetry_admission_is_relative_to_metric_scale() {
        let small_asymmetric = [
            [2.0e-20, 1.0e-20],
            [1.0e-20 + 1.0e-26, 3.0e-20],
        ];
        assert!(matches!(
            SpdMetricV1::new(small_asymmetric, 1.0e-10),
            Err(RankErrorV1::NonSymmetric { .. })
        ));
    }

    #[test]
    fn exact_rational_jacobian_rank_two_witness_is_valid() {
        let witness = ExactRankTwoWitnessV1 {
            jacobian: [[1, 0, 0], [0, 1, 0], [1, 1, 0]],
            left_null_vector: [-1, -1, 1],
            independent_minor_determinant: 1,
        };
        witness.verify().unwrap();
    }

    #[test]
    fn redundant_rows_produce_psd_rank_two_delassus_not_false_spd() {
        let operator = RankAwareDelassusV1::new(
            mass(),
            redundant_j(),
            identity_constraint_metric(),
            profile(),
        )
        .unwrap();
        let receipt = operator.rank_receipt();
        assert_eq!(receipt.status, RankStatusV1::RankDeficient);
        assert_eq!(receipt.rank, Some(2));
        assert_eq!(receipt.nullity, Some(1));
        assert!(receipt.condition_full_operator.is_none());
        assert!(receipt.condition_nonzero_spectrum.unwrap().is_finite());

        let expected = [[1.0, 0.0, 1.0], [0.0, 0.5, 0.5], [1.0, 0.5, 1.5]];
        for (i, row) in expected.iter().enumerate() {
            for (j, expected_value) in row.iter().enumerate() {
                close(operator.delassus()[i][j], *expected_value, 1.0e-14);
            }
        }
    }

    #[test]
    fn multiplier_fibre_has_one_unique_generalized_response() {
        let operator = RankAwareDelassusV1::new(
            mass(),
            redundant_j(),
            identity_constraint_metric(),
            profile(),
        )
        .unwrap();
        let lambda0 = [2.0, -1.0, 0.5];
        let rhs = mat_vec(operator.delassus(), &lambda0);
        let result = operator.analyze_rhs(rhs).unwrap();
        let ConstraintSolveOutcomeV1::Compatible(receipt) = result else {
            panic!("exact compatible rhs must be admitted");
        };
        assert_eq!(receipt.multiplier_nullity, 1);

        let null = [
            receipt.multiplier_nullspace[0][0],
            receipt.multiplier_nullspace[1][0],
            receipt.multiplier_nullspace[2][0],
        ];
        let j_t = transpose(operator.jacobian());
        let force_null = mat_vec(&j_t, &null);
        for value in force_null {
            close(value, 0.0, 2.0e-13);
        }

        for alpha in [-3.0, -1.0, 0.0, 0.25, 2.0] {
            let representative =
                std::array::from_fn(|i| receipt.canonical_multiplier[i] + alpha * null[i]);
            let reconstructed_rhs = mat_vec(operator.delassus(), &representative);
            for i in 0..3 {
                close(reconstructed_rhs[i], rhs[i], 2.0e-12);
            }
            let force = mat_vec(&j_t, &representative);
            let correction = solve(&mass().matrix, &force, profile()).unwrap();
            for i in 0..3 {
                close(force[i], receipt.generalized_constraint_force[i], 2.0e-12);
                close(
                    correction[i],
                    receipt.minimum_kinetic_velocity_correction[i],
                    2.0e-12,
                );
            }
        }
    }

    #[test]
    fn incompatible_rhs_is_not_silently_projected_into_range() {
        let operator = RankAwareDelassusV1::new(
            mass(),
            redundant_j(),
            identity_constraint_metric(),
            profile(),
        )
        .unwrap();
        let result = operator.analyze_rhs([1.0, 0.0, 0.0]).unwrap();
        assert!(matches!(
            result,
            ConstraintSolveOutcomeV1::Inconsistent { .. }
        ));
    }

    #[test]
    fn full_row_rank_baseline_keeps_finite_full_condition() {
        let jacobian = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let operator =
            RankAwareDelassusV1::new(mass(), jacobian, identity_constraint_metric(), profile())
                .unwrap();
        assert_eq!(operator.rank_receipt().status, RankStatusV1::FullRowRank);
        assert_eq!(operator.rank_receipt().rank, Some(3));
        assert_eq!(operator.rank_receipt().nullity, Some(0));
        assert!(
            operator
                .rank_receipt()
                .condition_full_operator
                .unwrap()
                .is_finite()
        );
    }

    fn inverse3(a: &Matrix<3, 3>) -> Matrix<3, 3> {
        inverse(a, profile()).unwrap()
    }

    #[test]
    fn coherent_row_rebasis_preserves_normalized_spectrum_and_physical_response() {
        let mass = mass();
        let j_old = redundant_j();
        let g_old = identity_constraint_metric();
        let op_old = RankAwareDelassusV1::new(mass, j_old, g_old, profile()).unwrap();
        let lambda0 = [2.0, -1.0, 0.5];
        let rhs_old = mat_vec(op_old.delassus(), &lambda0);
        let solve_old = op_old.analyze_rhs(rhs_old).unwrap();
        let ConstraintSolveOutcomeV1::Compatible(old_receipt) = solve_old else {
            panic!("old system should be compatible");
        };

        let transform = [[1.0, 2.0, 0.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]];
        let transform_inv = inverse3(&transform);
        let j_new = mat3_mul(&transform, &j_old);
        let w_new_expected = mat3_mul(
            &mat3_mul(&transform, op_old.delassus()),
            &transpose(&transform),
        );
        let g_new_matrix = mat3_mul(
            &mat3_mul(&transpose(&transform_inv), g_old.matrix()),
            &transform_inv,
        );
        let g_new = SpdMetricV1::new(g_new_matrix, 1.0e-10).unwrap();
        let op_new = RankAwareDelassusV1::new(mass, j_new, g_new, profile()).unwrap();

        for (i, expected_row) in w_new_expected.iter().enumerate() {
            for (j, expected_value) in expected_row.iter().enumerate() {
                close(op_new.delassus()[i][j], *expected_value, 2.0e-12);
            }
            close(
                op_old.rank_receipt().eigenvalues[i],
                op_new.rank_receipt().eigenvalues[i],
                3.0e-11,
            );
        }
        assert_eq!(op_old.rank_receipt().rank, op_new.rank_receipt().rank);
        assert_eq!(op_old.rank_receipt().nullity, op_new.rank_receipt().nullity);

        let rhs_new = mat_vec(&transform, &rhs_old);
        let solve_new = op_new.analyze_rhs(rhs_new).unwrap();
        let ConstraintSolveOutcomeV1::Compatible(new_receipt) = solve_new else {
            panic!("new system should be compatible");
        };

        let expected_lambda_new = mat_vec(
            &transpose(&transform_inv),
            &old_receipt.canonical_multiplier,
        );
        for (i, expected_multiplier) in expected_lambda_new.iter().enumerate() {
            close(
                new_receipt.canonical_multiplier[i],
                *expected_multiplier,
                5.0e-10,
            );
            close(
                new_receipt.generalized_constraint_force[i],
                old_receipt.generalized_constraint_force[i],
                5.0e-10,
            );
            close(
                new_receipt.minimum_kinetic_velocity_correction[i],
                old_receipt.minimum_kinetic_velocity_correction[i],
                5.0e-10,
            );
        }
    }

    #[test]
    fn near_rank_threshold_band_abstains() {
        let small = (0.8e-8_f64).sqrt();
        let jacobian = [[1.0, 0.0], [0.0, small]];
        let mass = SpdMetricV1::new([[1.0, 0.0], [0.0, 1.0]], 1.0e-14).unwrap();
        let metric = SpdMetricV1::new([[1.0, 0.0], [0.0, 1.0]], 1.0e-14).unwrap();
        let tuned = RankProfileV1 {
            relative_rank_tolerance: 1.0e-8,
            absolute_rank_tolerance: 0.0,
            ..profile()
        };
        let operator = RankAwareDelassusV1::new(mass, jacobian, metric, tuned).unwrap();
        assert_eq!(
            operator.rank_receipt().status,
            RankStatusV1::InconclusiveNearThreshold
        );
        let outcome = operator.analyze_rhs([1.0, 1.0]).unwrap();
        assert!(matches!(
            outcome,
            ConstraintSolveOutcomeV1::Inconclusive { .. }
        ));
    }

    #[test]
    fn jacobi_rank_classification_is_invariant_under_small_operator_scaling() {
        let mut exact_relative_profile = profile();
        exact_relative_profile.absolute_rank_tolerance = 0.0;
        exact_relative_profile.absolute_psd_tolerance = 0.0;
        let base = RankAwareDelassusV1::new(
            mass(),
            redundant_j(),
            identity_constraint_metric(),
            exact_relative_profile,
        )
        .unwrap();
        let scale = 1.0e-9;
        let scaled_jacobian = redundant_j().map(|row| row.map(|value| value * scale));
        let scaled = RankAwareDelassusV1::new(
            mass(),
            scaled_jacobian,
            identity_constraint_metric(),
            exact_relative_profile,
        )
        .unwrap();

        assert_eq!(base.rank_receipt().status, RankStatusV1::RankDeficient);
        assert_eq!(scaled.rank_receipt().status, RankStatusV1::RankDeficient);
        assert_eq!(base.rank_receipt().rank, scaled.rank_receipt().rank);
        assert_eq!(base.rank_receipt().nullity, scaled.rank_receipt().nullity);
        close(
            base.rank_receipt().condition_nonzero_spectrum.unwrap(),
            scaled.rank_receipt().condition_nonzero_spectrum.unwrap(),
            2.0e-10,
        );
    }

    #[test]
    fn range_compatibility_is_invariant_under_rhs_scaling() {
        let operator = RankAwareDelassusV1::new(
            mass(),
            redundant_j(),
            identity_constraint_metric(),
            profile(),
        )
        .unwrap();
        let rhs = mat_vec(operator.delassus(), &[0.5, -1.0, 2.0]);
        let base = operator.analyze_rhs(rhs).unwrap();
        let scaled_rhs = rhs.map(|value| value * 1.0e8);
        let scaled = operator.analyze_rhs(scaled_rhs).unwrap();
        let ConstraintSolveOutcomeV1::Compatible(base_receipt) = base else {
            panic!("base rhs should be compatible");
        };
        let ConstraintSolveOutcomeV1::Compatible(scaled_receipt) = scaled else {
            panic!("scaled rhs should remain compatible");
        };
        close(
            base_receipt.relative_range_defect,
            scaled_receipt.relative_range_defect,
            2.0e-12,
        );
    }

    #[test]
    fn zero_exact_null_vector_is_rejected() {
        let witness = ExactRankTwoWitnessV1 {
            jacobian: [[1, 0, 0], [0, 1, 0], [1, 1, 0]],
            left_null_vector: [0, 0, 0],
            independent_minor_determinant: 1,
        };
        assert_eq!(witness.verify(), Err(RankErrorV1::InvalidExactRankWitness));
    }

    #[test]
    fn exact_rank_witness_rejects_a_forged_null_vector() {
        let witness = ExactRankTwoWitnessV1 {
            jacobian: [[1, 0, 0], [0, 1, 0], [1, 1, 0]],
            left_null_vector: [1, 1, 1],
            independent_minor_determinant: 1,
        };
        assert_eq!(witness.verify(), Err(RankErrorV1::InvalidExactRankWitness));
    }

    #[test]
    fn singular_mass_and_nonfinite_jacobian_fail_closed() {
        let singular =
            SpdMetricV1::new([[1.0, 2.0, 0.0], [2.0, 1.0, 0.0], [0.0, 0.0, 1.0]], 1.0e-14);
        assert!(matches!(
            singular,
            Err(RankErrorV1::NotPositiveDefinite { .. })
        ));

        let invalid_j = [[f64::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]];
        let result =
            RankAwareDelassusV1::new(mass(), invalid_j, identity_constraint_metric(), profile());
        assert_eq!(result, Err(RankErrorV1::NonFiniteInput));
    }

    #[test]
    fn rank_deficient_full_condition_is_never_reported() {
        let operator = RankAwareDelassusV1::new(
            mass(),
            redundant_j(),
            identity_constraint_metric(),
            profile(),
        )
        .unwrap();
        let receipt = operator.rank_receipt();
        assert_eq!(receipt.status, RankStatusV1::RankDeficient);
        assert!(receipt.condition_full_operator.is_none());
        assert!(receipt.condition_nonzero_spectrum.is_some());
    }
}
