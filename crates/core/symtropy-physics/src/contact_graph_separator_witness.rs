// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Test-only first witness for set-valued Schur reduction on a sparse contact graph.
//!
//! The fixture is a three-contact star: one separator contact coupled to two
//! mutually independent leaves. The leaves are conditionally eliminated on
//! strict sticking branches; the separator retains the full Schur-reduced
//! Signorini-Coulomb problem. Every reconstructed candidate is checked against
//! the original 9x9 contact law and full-island energy.

use crate::coupled_friction::{CoupledCoulombRegime, solve_coupled_signorini_coulomb};
use nalgebra::{Matrix3, SMatrix, SVector};

type Matrix9 = SMatrix<f64, 9, 9>;
type Matrix12 = SMatrix<f64, 12, 12>;
type Jacobian9x12 = SMatrix<f64, 9, 12>;
type Vector9 = SVector<f64, 9>;
type Vector12 = SVector<f64, 12>;

const FRICTION: f64 = 0.5;
const COUPLING: f64 = -0.5;
const TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Debug)]
struct StickingLeafMessage {
    self_operator: Matrix3<f64>,
    separator_coupling: Matrix3<f64>,
    free_velocity: SVector<f64, 3>,
    friction_coefficient: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StickingBoxClassification {
    StrictlySticking,
    OutsideStrictStickingDomain,
    BoundaryOrInconclusive,
}

#[derive(Clone, Debug)]
struct StickingBoxResponse {
    impulse_bounds: [OutwardInterval; 3],
    local_residual_bounds: [OutwardInterval; 3],
    disk_margin_bounds: OutwardInterval,
    classification: StickingBoxClassification,
}

#[derive(Clone, Debug)]
struct StickingLeafResponse {
    impulse: SVector<f64, 3>,
    post_velocity: SVector<f64, 3>,
    strict_disk_margin: f64,
}

impl StickingLeafMessage {
    /// Return the affine impulse response p_leaf = offset + gain * p_separator.
    ///
    /// The fixed-size SPD Cholesky factorization solves A offset = -q_leaf
    /// and A gain = -B^T. Rank-aware fibres are a separate response-message lane.
    fn affine_response_coefficients(
        &self,
    ) -> Result<(SVector<f64, 3>, Matrix3<f64>), &'static str> {
        if !self.self_operator.iter().all(|value| value.is_finite())
            || !self
                .separator_coupling
                .iter()
                .all(|value| value.is_finite())
            || !self.free_velocity.iter().all(|value| value.is_finite())
            || !self.friction_coefficient.is_finite()
            || self.friction_coefficient < 0.0
        {
            return Err("sticking leaf message contains invalid or non-finite data");
        }
        let factor = self
            .self_operator
            .cholesky()
            .ok_or("sticking leaf self-block is not positive definite")?;
        let offset = -factor.solve(&self.free_velocity);
        let separator_gain = -factor.solve(&self.separator_coupling.transpose());
        Ok((offset, separator_gain))
    }

    /// Enclose the exact affine response over a whole separator box.
    ///
    /// The offset and gain are obtained by interval Gaussian elimination on
    /// the represented binary64 A/B/q data. This encloses the exact solution
    /// of those inputs; it does not merely widen a rounded Cholesky point solve.
    // The indexed loops implement fixed-size interval Gaussian elimination.
    #[allow(clippy::needless_range_loop)]
    fn response_over_box(
        &self,
        separator_box: &SeparatorImpulseBox,
    ) -> Result<StickingBoxResponse, &'static str> {
        if !self.self_operator.iter().all(|value| value.is_finite())
            || !self
                .separator_coupling
                .iter()
                .all(|value| value.is_finite())
            || !self.free_velocity.iter().all(|value| value.is_finite())
            || !self.friction_coefficient.is_finite()
            || self.friction_coefficient < 0.0
        {
            return Err("sticking leaf message contains invalid or non-finite data");
        }
        for row in 0..3 {
            for column in 0..3 {
                if self.self_operator[(row, column)] != self.self_operator[(column, row)] {
                    return Err("sticking leaf self-block must be exactly symmetric");
                }
            }
        }

        let interval_solve =
            |right_hand_side: SVector<f64, 3>| -> Result<[OutwardInterval; 3], &'static str> {
                let mut matrix = [[OutwardInterval::point(0.0)?; 3]; 3];
                let mut rhs = [OutwardInterval::point(0.0)?; 3];
                for row in 0..3 {
                    rhs[row] = OutwardInterval::point(right_hand_side[row])?;
                    for column in 0..3 {
                        matrix[row][column] =
                            OutwardInterval::point(self.self_operator[(row, column)])?;
                    }
                }

                // No pivoting is needed for an SPD matrix. Requiring every
                // outward-enclosed leading pivot to be positive certifies the
                // represented symmetric A along this fixed elimination order.
                for pivot_index in 0..3 {
                    let pivot = matrix[pivot_index][pivot_index];
                    if pivot.lower <= 0.0 {
                        return Err(
                            "interval Gaussian elimination cannot certify a positive SPD pivot",
                        );
                    }
                    for row in (pivot_index + 1)..3 {
                        let factor = matrix[row][pivot_index].outward_div(pivot)?;
                        for column in (pivot_index + 1)..3 {
                            matrix[row][column] = matrix[row][column]
                                .outward_sub(factor.outward_mul(matrix[pivot_index][column])?)?;
                        }
                        rhs[row] = rhs[row].outward_sub(factor.outward_mul(rhs[pivot_index])?)?;
                        matrix[row][pivot_index] = OutwardInterval::point(0.0)?;
                    }
                }

                let mut solution = [OutwardInterval::point(0.0)?; 3];
                for row in (0..3).rev() {
                    let mut residual = rhs[row];
                    for column in (row + 1)..3 {
                        residual = residual
                            .outward_sub(matrix[row][column].outward_mul(solution[column])?)?;
                    }
                    solution[row] = residual.outward_div(matrix[row][row])?;
                }
                Ok(solution)
            };

        let offset = interval_solve(-self.free_velocity)?;
        let mut gain = [[OutwardInterval::point(0.0)?; 3]; 3];
        for separator_axis in 0..3 {
            let right_hand_side = SVector::<f64, 3>::from_fn(|row, _| {
                -self.separator_coupling[(separator_axis, row)]
            });
            let column = interval_solve(right_hand_side)?;
            for impulse_axis in 0..3 {
                gain[impulse_axis][separator_axis] = column[impulse_axis];
            }
        }

        let mut impulse_bounds = [OutwardInterval::point(0.0)?; 3];
        for impulse_axis in 0..3 {
            let mut impulse = offset[impulse_axis];
            for separator_axis in 0..3 {
                let separator_coordinate = OutwardInterval::from_bounds(
                    separator_box.lower[separator_axis],
                    separator_box.upper[separator_axis],
                )?;
                impulse = impulse.outward_add(
                    gain[impulse_axis][separator_axis].outward_mul(separator_coordinate)?,
                )?;
            }
            impulse_bounds[impulse_axis] = impulse;
        }

        // Replay the original local equation over the same separator box.
        // Its enclosure must contain zero in every component for the declared
        // affine solve to be consistent with the unreduced contact law.
        let mut local_residual_bounds = [OutwardInterval::point(0.0)?; 3];
        for row in 0..3 {
            let mut residual = OutwardInterval::point(self.free_velocity[row])?;
            for separator_axis in 0..3 {
                let separator_coordinate = OutwardInterval::from_bounds(
                    separator_box.lower[separator_axis],
                    separator_box.upper[separator_axis],
                )?;
                let coupling =
                    OutwardInterval::point(self.separator_coupling[(separator_axis, row)])?;
                residual = residual.outward_add(coupling.outward_mul(separator_coordinate)?)?;
            }
            for impulse_axis in 0..3 {
                let coefficient = OutwardInterval::point(self.self_operator[(row, impulse_axis)])?;
                residual =
                    residual.outward_add(coefficient.outward_mul(impulse_bounds[impulse_axis])?)?;
            }
            if !residual.contains_zero() {
                return Err(
                    "interval sticking response fails the original local velocity equation",
                );
            }
            local_residual_bounds[row] = residual;
        }

        let normal_load = impulse_bounds[0];
        let friction_radius =
            OutwardInterval::point(self.friction_coefficient)?.outward_mul(normal_load)?;
        let tangent_norm = impulse_bounds[1]
            .outward_square()?
            .outward_add(impulse_bounds[2].outward_square()?)?
            .outward_sqrt_nonnegative()?;
        let disk_margin_bounds = friction_radius.outward_sub(tangent_norm)?;

        let classification = if normal_load.upper <= 0.0 || disk_margin_bounds.upper < 0.0 {
            StickingBoxClassification::OutsideStrictStickingDomain
        } else if normal_load.lower > 0.0 && disk_margin_bounds.lower > 0.0 {
            StickingBoxClassification::StrictlySticking
        } else {
            StickingBoxClassification::BoundaryOrInconclusive
        };

        Ok(StickingBoxResponse {
            impulse_bounds,
            local_residual_bounds,
            disk_margin_bounds,
            classification,
        })
    }

    /// Return the block-Schur terms for this regime-conditioned leaf.
    ///
    /// A p_leaf + B^T p_separator + q_leaf = 0 gives
    /// W_eff = C - B A^-1 B^T and q_eff = q_separator - B A^-1 q_leaf.
    /// Using offset=-A^-1 q_leaf and gain=-A^-1 B^T:
    /// B A^-1 B^T = -B gain and B A^-1 q_leaf = -B offset.
    fn schur_terms(&self) -> Result<(Matrix3<f64>, SVector<f64, 3>), &'static str> {
        let (offset, separator_gain) = self.affine_response_coefficients()?;
        Ok((
            -(self.separator_coupling * separator_gain),
            -(self.separator_coupling * offset),
        ))
    }

    /// Evaluate the leaf's set-valued response relation at one separator impulse.
    ///
    /// The affine response is retained only where the leaf is strictly loaded,
    /// strictly inside its Coulomb disk, and zero post-contact velocity holds.
    fn response_at(
        &self,
        separator_impulse: &SVector<f64, 3>,
        tolerance: f64,
    ) -> Result<Option<StickingLeafResponse>, &'static str> {
        if !tolerance.is_finite()
            || tolerance <= 0.0
            || !separator_impulse.iter().all(|value| value.is_finite())
        {
            return Err("sticking leaf response received invalid or non-finite input");
        }
        let (offset, separator_gain) = self.affine_response_coefficients()?;
        let impulse = offset + separator_gain * separator_impulse;
        let post_velocity = self.free_velocity
            + self.separator_coupling.transpose() * separator_impulse
            + self.self_operator * impulse;
        let strict_disk_margin =
            self.friction_coefficient * impulse[0] - impulse.fixed_rows::<2>(1).norm();
        let impulse_tolerance = tolerance * impulse.norm().max(f64::MIN_POSITIVE) * 500.0;
        let velocity_scale = (self.free_velocity.norm()
            + self.separator_coupling.norm() * separator_impulse.norm()
            + self.self_operator.norm() * impulse.norm())
        .max(f64::MIN_POSITIVE);
        let velocity_tolerance = tolerance * velocity_scale * 500.0;

        // None means this valid message has no strict-sticking response at the
        // supplied separator impulse. Invalid data and unsupported self-blocks
        // are errors, so callers must not conflate domain exclusion with failure.
        if impulse[0] <= impulse_tolerance
            || strict_disk_margin <= impulse_tolerance
            || post_velocity.norm() > velocity_tolerance
        {
            return Ok(None);
        }

        Ok(Some(StickingLeafResponse {
            impulse,
            post_velocity,
            strict_disk_margin,
        }))
    }
}

/// A leaf restricted to the strict separating regime p_leaf = 0.
#[derive(Clone, Debug)]
struct SeparatingLeafMessage {
    /// B in the block partition W = [[C, B], [B^T, A]].
    separator_coupling: Matrix3<f64>,
    free_velocity: SVector<f64, 3>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SeparatingLeafClassification {
    /// p_n = 0 and u_n is strictly positive beyond the declared tolerance.
    StrictlySeparating,
    /// p_n = 0 and u_n is zero or indistinguishable from zero at this tolerance.
    /// This is a boundary stratum, not a strict separating branch.
    BoundaryWithinTolerance,
    /// p_n = 0 but u_n is strictly negative: this separating regime is invalid.
    SignoriniViolation,
}

/// A conservative interval whose endpoints enclose the exact arithmetic
/// result for the represented binary64 inputs.
#[derive(Clone, Copy, Debug, PartialEq)]
struct OutwardInterval {
    lower: f64,
    upper: f64,
}

impl OutwardInterval {
    fn point(value: f64) -> Result<Self, &'static str> {
        if !value.is_finite() {
            return Err("interval point must be finite");
        }
        Ok(Self {
            lower: value,
            upper: value,
        })
    }

    fn from_bounds(lower: f64, upper: f64) -> Result<Self, &'static str> {
        if !lower.is_finite() || !upper.is_finite() || lower > upper {
            return Err("interval bounds must be finite and ordered");
        }
        Ok(Self { lower, upper })
    }

    fn outward_add(self, other: Self) -> Result<Self, &'static str> {
        let lower = self.lower + other.lower;
        let upper = self.upper + other.upper;
        if !lower.is_finite() || !upper.is_finite() {
            return Err("interval addition overflowed");
        }
        let enclosure = Self {
            lower: lower.next_down(),
            upper: upper.next_up(),
        };
        if !enclosure.lower.is_finite() || !enclosure.upper.is_finite() {
            return Err("outward interval addition reached a non-finite endpoint");
        }
        Ok(enclosure)
    }

    fn outward_mul(self, other: Self) -> Result<Self, &'static str> {
        let products = [
            self.lower * other.lower,
            self.lower * other.upper,
            self.upper * other.lower,
            self.upper * other.upper,
        ];
        if !products.iter().all(|value| value.is_finite()) {
            return Err("interval multiplication overflowed or became non-finite");
        }
        let minimum = products.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let enclosure = Self {
            lower: minimum.next_down(),
            upper: maximum.next_up(),
        };
        if !enclosure.lower.is_finite() || !enclosure.upper.is_finite() {
            return Err("outward interval multiplication reached a non-finite endpoint");
        }
        Ok(enclosure)
    }

    fn outward_sub(self, other: Self) -> Result<Self, &'static str> {
        self.outward_add(Self {
            lower: -other.upper,
            upper: -other.lower,
        })
    }

    fn outward_div(self, other: Self) -> Result<Self, &'static str> {
        if other.lower <= 0.0 && other.upper >= 0.0 {
            return Err("interval division denominator contains zero");
        }
        let reciprocal_a = 1.0 / other.lower;
        let reciprocal_b = 1.0 / other.upper;
        if !reciprocal_a.is_finite() || !reciprocal_b.is_finite() {
            return Err("interval reciprocal overflowed");
        }
        let reciprocal = Self {
            lower: reciprocal_a.min(reciprocal_b).next_down(),
            upper: reciprocal_a.max(reciprocal_b).next_up(),
        };
        if !reciprocal.lower.is_finite() || !reciprocal.upper.is_finite() {
            return Err("outward interval reciprocal reached a non-finite endpoint");
        }
        self.outward_mul(reciprocal)
    }

    fn contains_zero(self) -> bool {
        self.lower <= 0.0 && self.upper >= 0.0
    }

    fn min_abs(self) -> f64 {
        if self.contains_zero() {
            0.0
        } else {
            self.lower.abs().min(self.upper.abs())
        }
    }

    fn max_abs(self) -> f64 {
        self.lower.abs().max(self.upper.abs())
    }

    fn outward_square(self) -> Result<Self, &'static str> {
        let minimum = self.min_abs();
        let maximum = self.max_abs();
        let lower_product = minimum * minimum;
        let upper_product = maximum * maximum;
        if !lower_product.is_finite() || !upper_product.is_finite() {
            return Err("interval square overflowed");
        }
        let lower = lower_product.next_down().max(0.0);
        let upper = upper_product.next_up();
        if !upper.is_finite() {
            return Err("outward interval square reached a non-finite endpoint");
        }
        Ok(Self { lower, upper })
    }

    /// Square-root an exact nonnegative quantity whose outward interval
    /// enclosure may have a tiny negative lower endpoint from rounding.
    /// A wholly negative enclosure is invalid; otherwise clamp only the lower
    /// bound to the known mathematical domain [0, +inf).
    fn outward_sqrt_nonnegative(self) -> Result<Self, &'static str> {
        if !self.lower.is_finite() || !self.upper.is_finite() || self.upper < 0.0 {
            return Err(
                "interval square root requires a finite enclosure intersecting the nonnegative domain",
            );
        }
        let lower = self.lower.max(0.0).sqrt().next_down().max(0.0);
        let upper = self.upper.sqrt().next_up();
        if !lower.is_finite() || !upper.is_finite() {
            return Err("outward interval square root reached a non-finite endpoint");
        }
        Ok(Self { lower, upper })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SeparatorImpulseBox {
    lower: SVector<f64, 3>,
    upper: SVector<f64, 3>,
}

impl SeparatorImpulseBox {
    fn new(lower: SVector<f64, 3>, upper: SVector<f64, 3>) -> Result<Self, &'static str> {
        for axis in 0..3 {
            OutwardInterval::from_bounds(lower[axis], upper[axis])?;
        }
        Ok(Self { lower, upper })
    }

    fn intersection(&self, other: &Self) -> Option<Self> {
        let lower = SVector::<f64, 3>::from_fn(|axis, _| self.lower[axis].max(other.lower[axis]));
        let upper = SVector::<f64, 3>::from_fn(|axis, _| self.upper[axis].min(other.upper[axis]));
        (0..3)
            .all(|axis| lower[axis] <= upper[axis])
            .then_some(Self { lower, upper })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SeparatingBoxClassification {
    StrictlySeparating,
    SignoriniViolation,
    BoundaryOrInconclusive,
}

#[derive(Clone, Debug)]
struct SeparatingBoxResponse {
    impulse: SVector<f64, 3>,
    post_velocity_bounds: [OutwardInterval; 3],
    classification: SeparatingBoxClassification,
}

#[derive(Clone, Debug, PartialEq)]
struct SeparatorBoxCell {
    separator_box: SeparatorImpulseBox,
    classification: SeparatingBoxClassification,
    depth: usize,
    lineage: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
struct SeparatorBoxPartition {
    root_box: SeparatorImpulseBox,
    cells: Vec<SeparatorBoxCell>,
    processed_nodes: usize,
    max_depth_reached: usize,
    node_budget_exhausted: bool,
    coverage_preserved: bool,
    classification_complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComposedSeparatorClassification {
    AllLocalHypothesesStrict,
    LocalHypothesisExcluded,
    BoundaryOrInconclusive,
}

#[derive(Clone, Debug, PartialEq)]
struct ComposedSeparatorBoxCell {
    separator_box: SeparatorImpulseBox,
    left_classification: SeparatingBoxClassification,
    right_classification: SeparatingBoxClassification,
    left_lineage: Vec<u8>,
    right_lineage: Vec<u8>,
    classification: ComposedSeparatorClassification,
}

#[derive(Clone, Debug, PartialEq)]
struct ComposedSeparatorBoxPartition {
    root_box: SeparatorImpulseBox,
    cells: Vec<ComposedSeparatorBoxCell>,
    candidate_pairs: usize,
    empty_intersections: usize,
    coverage_preserved: bool,
    classification_complete: bool,
}

#[derive(Clone, Debug)]
struct SeparatingLeafResponse {
    impulse: SVector<f64, 3>,
    post_velocity: SVector<f64, 3>,
    classification: SeparatingLeafClassification,
    normal_velocity_margin: f64,
}

impl SeparatingLeafMessage {
    /// A strict separating leaf has p_leaf = 0, so it contributes no
    /// separator impulse, no Schur fill, and no free-velocity correction.
    fn schur_terms(&self) -> Result<(Matrix3<f64>, SVector<f64, 3>), &'static str> {
        if !self
            .separator_coupling
            .iter()
            .all(|value| value.is_finite())
            || !self.free_velocity.iter().all(|value| value.is_finite())
        {
            return Err("separating leaf message contains invalid or non-finite data");
        }
        Ok((Matrix3::zeros(), SVector::<f64, 3>::zeros()))
    }

    /// Select a deterministic split axis that actually changes the affine
    /// normal response. The largest absolute normal coefficient wins; ties
    /// resolve to the lowest axis index.
    fn split_box(
        &self,
        parent: &SeparatorImpulseBox,
    ) -> Option<(usize, f64, SeparatorImpulseBox, SeparatorImpulseBox)> {
        let mut selected_axis = None;
        let mut largest_coefficient = 0.0_f64;
        for axis in 0..3 {
            let coefficient = self.separator_coupling[(axis, 0)].abs();
            if parent.lower[axis] < parent.upper[axis] && coefficient > largest_coefficient {
                selected_axis = Some(axis);
                largest_coefficient = coefficient;
            }
        }
        let axis = selected_axis?;
        let lower = parent.lower[axis];
        let upper = parent.upper[axis];
        // Halving each endpoint avoids overflowing an intermediate upper-lower
        // difference for finite values near the binary64 range limit.
        let midpoint = lower * 0.5 + upper * 0.5;
        if !midpoint.is_finite() || midpoint <= lower || midpoint >= upper {
            return None;
        }

        let mut left_upper = parent.upper;
        left_upper[axis] = midpoint;
        let left = SeparatorImpulseBox {
            lower: parent.lower,
            upper: left_upper,
        };
        let mut right_lower = parent.lower;
        right_lower[axis] = midpoint;
        let right = SeparatorImpulseBox {
            lower: right_lower,
            upper: parent.upper,
        };
        Some((axis, midpoint, left, right))
    }

    /// Verify that retained leaf lineages form a complete binary partition of
    /// the declared root box, and that every lineage reconstructs its exact
    /// recorded axis-aligned box under the deterministic split policy.
    fn verify_box_cover(&self, root: &SeparatorImpulseBox, cells: &[SeparatorBoxCell]) -> bool {
        let mut leaves = std::collections::BTreeMap::<Vec<u8>, &SeparatorBoxCell>::new();
        for cell in cells {
            if cell.depth != cell.lineage.len()
                || cell.lineage.iter().any(|branch| *branch > 1)
                || leaves.insert(cell.lineage.clone(), cell).is_some()
            {
                return false;
            }

            // Independently replay the interval classification as well as
            // reconstructing the geometry of every retained leaf. A valid
            // cover may not relabel a boundary cell as a strict regime.
            let replayed_classification = self
                .response_over_box(&cell.separator_box)
                .map(|response| response.classification)
                .unwrap_or(SeparatingBoxClassification::BoundaryOrInconclusive);
            if replayed_classification != cell.classification {
                return false;
            }
        }
        let paths = leaves.keys().cloned().collect::<Vec<_>>();
        for (index, path) in paths.iter().enumerate() {
            for (other_index, other) in paths.iter().enumerate() {
                if index != other_index && path.len() < other.len() && other.starts_with(path) {
                    return false;
                }
            }
        }

        fn walk(
            message: &SeparatingLeafMessage,
            current_box: &SeparatorImpulseBox,
            path: &mut Vec<u8>,
            leaves: &std::collections::BTreeMap<Vec<u8>, &SeparatorBoxCell>,
        ) -> bool {
            if let Some(cell) = leaves.get(path) {
                return cell.separator_box == *current_box && cell.depth == path.len();
            }
            let Some((_, _, left, right)) = message.split_box(current_box) else {
                return false;
            };
            let mut left_path = path.clone();
            left_path.push(0);
            let mut right_path = path.clone();
            right_path.push(1);
            let has_left_descendant = leaves
                .keys()
                .any(|candidate| candidate.starts_with(&left_path));
            let has_right_descendant = leaves
                .keys()
                .any(|candidate| candidate.starts_with(&right_path));
            if !has_left_descendant || !has_right_descendant {
                return false;
            }
            path.push(0);
            let left_valid = walk(message, &left, path, leaves);
            path.pop();
            if !left_valid {
                return false;
            }
            path.push(1);
            let right_valid = walk(message, &right, path, leaves);
            path.pop();
            right_valid
        }

        walk(self, root, &mut Vec::new(), &leaves)
    }

    /// Subdivide a separator box while preserving every strict and unresolved
    /// region. Budget/depth limits retain frontier boxes as inconclusive cells.
    fn partition_separator_box(
        &self,
        root: &SeparatorImpulseBox,
        max_depth: usize,
        node_budget: usize,
    ) -> Result<SeparatorBoxPartition, &'static str> {
        if node_budget == 0 {
            return Err("separator box partition requires a positive node budget");
        }
        self.schur_terms()?;

        let mut stack = vec![(root.clone(), 0_usize, Vec::<u8>::new())];
        let mut cells = Vec::new();
        let mut processed_nodes = 0_usize;
        let mut node_budget_exhausted = false;

        while let Some((current_box, depth, lineage)) = stack.pop() {
            if processed_nodes >= node_budget {
                node_budget_exhausted = true;
                cells.push(SeparatorBoxCell {
                    separator_box: current_box,
                    classification: SeparatingBoxClassification::BoundaryOrInconclusive,
                    depth,
                    lineage,
                });
                for (remaining_box, remaining_depth, remaining_lineage) in stack.drain(..) {
                    cells.push(SeparatorBoxCell {
                        separator_box: remaining_box,
                        classification: SeparatingBoxClassification::BoundaryOrInconclusive,
                        depth: remaining_depth,
                        lineage: remaining_lineage,
                    });
                }
                break;
            }
            processed_nodes += 1;

            let classification = self
                .response_over_box(&current_box)
                .map(|response| response.classification)
                .unwrap_or(SeparatingBoxClassification::BoundaryOrInconclusive);

            if classification != SeparatingBoxClassification::BoundaryOrInconclusive {
                cells.push(SeparatorBoxCell {
                    separator_box: current_box,
                    classification,
                    depth,
                    lineage,
                });
                continue;
            }

            if depth < max_depth {
                if let Some((_, _, left, right)) = self.split_box(&current_box) {
                    let mut left_lineage = lineage.clone();
                    left_lineage.push(0);
                    let mut right_lineage = lineage;
                    right_lineage.push(1);
                    // Stack is LIFO: right is pushed first so the left branch
                    // is processed first deterministically.
                    stack.push((right, depth + 1, right_lineage));
                    stack.push((left, depth + 1, left_lineage));
                    continue;
                }
            }

            cells.push(SeparatorBoxCell {
                separator_box: current_box,
                classification: SeparatingBoxClassification::BoundaryOrInconclusive,
                depth,
                lineage,
            });
        }

        cells.sort_by(|left, right| left.lineage.cmp(&right.lineage));
        let coverage_preserved = self.verify_box_cover(root, &cells);
        if !coverage_preserved {
            return Err("separator box partition failed its independent lineage coverage check");
        }
        let max_depth_reached = cells.iter().map(|cell| cell.depth).max().unwrap_or(0);
        let classification_complete = cells
            .iter()
            .all(|cell| cell.classification != SeparatingBoxClassification::BoundaryOrInconclusive);

        Ok(SeparatorBoxPartition {
            root_box: root.clone(),
            cells,
            processed_nodes,
            max_depth_reached,
            node_budget_exhausted,
            coverage_preserved,
            classification_complete,
        })
    }

    fn composed_classification(
        left: SeparatingBoxClassification,
        right: SeparatingBoxClassification,
    ) -> ComposedSeparatorClassification {
        if left == SeparatingBoxClassification::SignoriniViolation
            || right == SeparatingBoxClassification::SignoriniViolation
        {
            ComposedSeparatorClassification::LocalHypothesisExcluded
        } else if left == SeparatingBoxClassification::StrictlySeparating
            && right == SeparatingBoxClassification::StrictlySeparating
        {
            ComposedSeparatorClassification::AllLocalHypothesesStrict
        } else {
            ComposedSeparatorClassification::BoundaryOrInconclusive
        }
    }

    /// Independently verify a Cartesian intersection of two local regime
    /// partitions. The lineage pair is identity: equal geometric boxes with
    /// distinct source branches must remain distinct composed cells.
    fn verify_composed_partition(
        &self,
        left: &SeparatorBoxPartition,
        right_message: &SeparatingLeafMessage,
        right: &SeparatorBoxPartition,
        composed: &ComposedSeparatorBoxPartition,
    ) -> bool {
        if left.root_box != right.root_box
            || composed.root_box != left.root_box
            || !left.coverage_preserved
            || !right.coverage_preserved
            || !self.verify_box_cover(&left.root_box, &left.cells)
            || !right_message.verify_box_cover(&right.root_box, &right.cells)
        {
            return false;
        }

        let candidate_pairs = match left.cells.len().checked_mul(right.cells.len()) {
            Some(count) => count,
            None => return false,
        };
        if composed.candidate_pairs != candidate_pairs {
            return false;
        }

        let mut expected = std::collections::BTreeMap::<
            (Vec<u8>, Vec<u8>),
            ComposedSeparatorBoxCell,
        >::new();
        let mut empty_intersections = 0_usize;
        for left_cell in &left.cells {
            for right_cell in &right.cells {
                let key = (
                    left_cell.lineage.clone(),
                    right_cell.lineage.clone(),
                );
                if let Some(intersection) = left_cell
                    .separator_box
                    .intersection(&right_cell.separator_box)
                {
                    let classification = Self::composed_classification(
                        left_cell.classification,
                        right_cell.classification,
                    );
                    expected.insert(
                        key.clone(),
                        ComposedSeparatorBoxCell {
                            separator_box: intersection,
                            left_classification: left_cell.classification,
                            right_classification: right_cell.classification,
                            left_lineage: key.0,
                            right_lineage: key.1,
                            classification,
                        },
                    );
                } else {
                    empty_intersections += 1;
                }
            }
        }
        if composed.empty_intersections != empty_intersections
            || composed.cells.len() != expected.len()
        {
            return false;
        }

        let mut observed = std::collections::BTreeSet::<(Vec<u8>, Vec<u8>)>::new();
        for cell in &composed.cells {
            let key = (cell.left_lineage.clone(), cell.right_lineage.clone());
            if !observed.insert(key.clone()) {
                return false;
            }
            let Some(expected_cell) = expected.get(&key) else {
                return false;
            };
            if cell != expected_cell {
                return false;
            }
        }
        observed.len() == expected.len()
    }

    /// Intersect two coverage-preserving local regime partitions over one
    /// common separator domain. Empty pair intersections are counted, while
    /// every nonempty branch-pair cell keeps both labels and lineages.
    fn compose_separator_partitions(
        &self,
        left: &SeparatorBoxPartition,
        right_message: &SeparatingLeafMessage,
        right: &SeparatorBoxPartition,
    ) -> Result<ComposedSeparatorBoxPartition, &'static str> {
        if left.root_box != right.root_box {
            return Err("local separator partitions do not share an identical root domain");
        }
        if !left.coverage_preserved
            || !right.coverage_preserved
            || !self.verify_box_cover(&left.root_box, &left.cells)
            || !right_message.verify_box_cover(&right.root_box, &right.cells)
        {
            return Err("local separator partition failed independent source coverage verification");
        }

        let candidate_pairs = left
            .cells
            .len()
            .checked_mul(right.cells.len())
            .ok_or("separator partition Cartesian product size overflowed")?;
        let mut cells = Vec::new();
        let mut empty_intersections = 0_usize;
        for left_cell in &left.cells {
            for right_cell in &right.cells {
                let Some(separator_box) = left_cell
                    .separator_box
                    .intersection(&right_cell.separator_box)
                else {
                    empty_intersections += 1;
                    continue;
                };
                cells.push(ComposedSeparatorBoxCell {
                    separator_box,
                    left_classification: left_cell.classification,
                    right_classification: right_cell.classification,
                    left_lineage: left_cell.lineage.clone(),
                    right_lineage: right_cell.lineage.clone(),
                    classification: Self::composed_classification(
                        left_cell.classification,
                        right_cell.classification,
                    ),
                });
            }
        }
        cells.sort_by(|left, right| {
            left.left_lineage
                .cmp(&right.left_lineage)
                .then_with(|| left.right_lineage.cmp(&right.right_lineage))
        });

        let mut composed = ComposedSeparatorBoxPartition {
            root_box: left.root_box.clone(),
            cells,
            candidate_pairs,
            empty_intersections,
            coverage_preserved: false,
            classification_complete: false,
        };
        composed.coverage_preserved =
            self.verify_composed_partition(left, right_message, right, &composed);
        if !composed.coverage_preserved {
            return Err("composed separator partition failed its independent intersection-cover check");
        }
        composed.classification_complete = composed.cells.iter().all(|cell| {
            cell.classification != ComposedSeparatorClassification::BoundaryOrInconclusive
        });
        Ok(composed)
    }

    /// Evaluate the separating response at one separator impulse.
    ///
    /// The local post-contact velocity is u_L = q_L + B^T p_S because
    /// p_L=0. Its normal component determines whether this regime is strict,
    /// boundary-only, or violates Signorini. Tangential velocity is retained
    /// for the original-system replay, but the zero-radius Coulomb disk forces
    /// the tangential impulse to zero.
    /// Enclose the entire affine response over an axis-aligned separator box.
    ///
    /// Each multiply and add widens its endpoints by one adjacent binary64
    /// value. A strict regime is reported only when the complete normal
    /// velocity interval has a strict sign. If zero is included, the result
    /// remains boundary/inconclusive; midpoint signs are never used.
    fn response_over_box(
        &self,
        separator_box: &SeparatorImpulseBox,
    ) -> Result<SeparatingBoxResponse, &'static str> {
        self.schur_terms()?;
        let mut post_velocity_bounds = [OutwardInterval::point(0.0)?; 3];
        for (component, post_velocity_bound) in post_velocity_bounds.iter_mut().enumerate() {
            let mut velocity = OutwardInterval::point(self.free_velocity[component])?;
            for separator_axis in 0..3 {
                let coefficient =
                    OutwardInterval::point(self.separator_coupling[(separator_axis, component)])?;
                let separator_coordinate = OutwardInterval::from_bounds(
                    separator_box.lower[separator_axis],
                    separator_box.upper[separator_axis],
                )?;
                velocity = velocity.outward_add(coefficient.outward_mul(separator_coordinate)?)?;
            }
            *post_velocity_bound = velocity;
        }

        let normal = post_velocity_bounds[0];
        let classification = if normal.lower > 0.0 {
            SeparatingBoxClassification::StrictlySeparating
        } else if normal.upper < 0.0 {
            SeparatingBoxClassification::SignoriniViolation
        } else {
            SeparatingBoxClassification::BoundaryOrInconclusive
        };

        Ok(SeparatingBoxResponse {
            impulse: SVector::<f64, 3>::zeros(),
            post_velocity_bounds,
            classification,
        })
    }

    fn response_at(
        &self,
        separator_impulse: &SVector<f64, 3>,
        tolerance: f64,
    ) -> Result<SeparatingLeafResponse, &'static str> {
        if !tolerance.is_finite()
            || tolerance <= 0.0
            || !separator_impulse.iter().all(|value| value.is_finite())
        {
            return Err("separating leaf response received invalid or non-finite input");
        }
        let (operator_contribution, free_velocity_contribution) = self.schur_terms()?;
        debug_assert_eq!(operator_contribution, Matrix3::zeros());
        debug_assert_eq!(free_velocity_contribution, SVector::<f64, 3>::zeros());

        let post_velocity =
            self.free_velocity + self.separator_coupling.transpose() * separator_impulse;
        let velocity_scale = (self.free_velocity.norm()
            + self.separator_coupling.norm() * separator_impulse.norm())
        .max(f64::MIN_POSITIVE);
        let normal_tolerance = tolerance * velocity_scale * 500.0;
        let normal_velocity = post_velocity[0];
        let classification = if normal_velocity > normal_tolerance {
            SeparatingLeafClassification::StrictlySeparating
        } else if normal_velocity < -normal_tolerance {
            SeparatingLeafClassification::SignoriniViolation
        } else {
            SeparatingLeafClassification::BoundaryWithinTolerance
        };

        Ok(SeparatingLeafResponse {
            impulse: SVector::<f64, 3>::zeros(),
            post_velocity,
            classification,
            normal_velocity_margin: normal_velocity,
        })
    }
}

#[derive(Clone, Debug)]
struct StarCandidate {
    impulse: Vector9,
    post_velocity: Vector9,
    separator_regime: CoupledCoulombRegime,
    separator_tau: f64,
    energy_delta: f64,
    effective_operator: Matrix3<f64>,
    effective_free_velocity: SVector<f64, 3>,
    leaf_disk_margins: [f64; 2],
}

fn star_contact_jacobian() -> Jacobian9x12 {
    let mut jacobian = Jacobian9x12::zeros();
    for axis in 0..3 {
        // Separator contact 0: v_B - v_C.
        jacobian[(axis, axis + 3)] = 1.0;
        jacobian[(axis, axis + 6)] = -1.0;
        // Leaf contact 1: v_A - v_B.
        jacobian[(axis + 3, axis)] = 1.0;
        jacobian[(axis + 3, axis + 3)] = -1.0;
        // Leaf contact 2: v_C - v_D.
        jacobian[(axis + 6, axis + 6)] = 1.0;
        jacobian[(axis + 6, axis + 9)] = -1.0;
    }
    jacobian
}

fn star_body_generated_fixture(first_leaf_tangent: f64) -> (Matrix9, Vector9) {
    // Four translational bodies A/B/C/D, each with mass 2 I. The three
    // contacts are (B,C), (A,B), and (C,D), so contact 0 is the separator
    // and contacts 1/2 are leaves. Shared-body Jacobian support creates the
    // two separator/leaf blocks; the leaf-to-leaf block is structurally zero.
    let jacobian = star_contact_jacobian();
    let inverse_mass = Matrix12::identity() * 0.5;
    let w = jacobian * inverse_mass * jacobian.transpose();
    let free_body_velocity = Vector12::from_row_slice(&[
        -0.5,
        first_leaf_tangent,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        -1.0,
        0.0,
        0.5,
        -0.75,
        0.0,
    ]);
    let q = jacobian * free_body_velocity;
    (w, q)
}

fn solve_star_with_strict_sticking_leaves(
    w: &Matrix9,
    q: &Vector9,
) -> Result<Vec<StarCandidate>, &'static str> {
    if !w.iter().all(|value| value.is_finite()) || !q.iter().all(|value| value.is_finite()) {
        return Err("non-finite star contact input");
    }
    if (w - w.transpose()).norm() > TOLERANCE || w.cholesky().is_none() {
        return Err("star Delassus operator must be symmetric positive definite");
    }

    // Validate the body-support-derived star sparsity rather than inferring
    // zeros by thresholding: leaf self-blocks are I and their cross block is 0.
    for row in 0..3 {
        for column in 0..3 {
            if w[(row + 3, column + 6)] != 0.0
                || w[(row + 6, column + 3)] != 0.0
                || w[(row + 3, column + 3)] != if row == column { 1.0 } else { 0.0 }
                || w[(row + 6, column + 6)] != if row == column { 1.0 } else { 0.0 }
                || w[(row, column + 3)] != if row == column { COUPLING } else { 0.0 }
                || w[(row, column + 6)] != if row == column { COUPLING } else { 0.0 }
            {
                return Err("input does not match the declared three-contact star structure");
            }
        }
    }

    let block = |row_offset: usize, column_offset: usize| {
        Matrix3::from_fn(|row, column| w[(row + row_offset, column + column_offset)])
    };
    let vector = |offset: usize| SVector::<f64, 3>::new(q[offset], q[offset + 1], q[offset + 2]);
    let separator_block = block(0, 0);
    let mut effective_operator = separator_block;
    let mut effective_free_velocity = vector(0);
    let messages = [
        StickingLeafMessage {
            self_operator: block(3, 3),
            separator_coupling: block(0, 3),
            free_velocity: vector(3),
            friction_coefficient: FRICTION,
        },
        StickingLeafMessage {
            self_operator: block(6, 6),
            separator_coupling: block(0, 6),
            free_velocity: vector(6),
            friction_coefficient: FRICTION,
        },
    ];

    // Each leaf contributes a regime-conditioned response message over the
    // same separator impulse. Their Schur terms update the reduced interface.
    for message in &messages {
        let (operator_contribution, free_velocity_contribution) = message.schur_terms()?;
        effective_operator -= operator_contribution;
        effective_free_velocity -= free_velocity_contribution;
    }

    let reduced = solve_coupled_signorini_coulomb(
        &effective_operator,
        &effective_free_velocity,
        FRICTION,
        TOLERANCE,
    )
    .map_err(|_| "reduced separator problem was rejected")?;

    let mut candidates = Vec::new();
    for separator in &reduced.candidates {
        let p_0 = separator.impulse;
        let Some(leaf_1) = messages[0].response_at(&p_0, TOLERANCE)? else {
            continue;
        };
        let Some(leaf_2) = messages[1].response_at(&p_0, TOLERANCE)? else {
            continue;
        };
        let impulse = Vector9::from_row_slice(&[
            p_0[0],
            p_0[1],
            p_0[2],
            leaf_1.impulse[0],
            leaf_1.impulse[1],
            leaf_1.impulse[2],
            leaf_2.impulse[0],
            leaf_2.impulse[1],
            leaf_2.impulse[2],
        ]);
        let post_velocity = q + w * impulse;

        let impulse_scale = impulse.norm().max(f64::MIN_POSITIVE);
        let velocity_scale = (w.norm() * impulse.norm())
            .max(q.norm())
            .max(f64::MIN_POSITIVE);
        let impulse_tolerance = TOLERANCE * impulse_scale * 500.0;
        let velocity_tolerance = TOLERANCE * velocity_scale * 500.0;
        let leaf_1_velocity =
            SVector::<f64, 3>::new(post_velocity[3], post_velocity[4], post_velocity[5]);
        let leaf_2_velocity =
            SVector::<f64, 3>::new(post_velocity[6], post_velocity[7], post_velocity[8]);

        if (leaf_1_velocity - leaf_1.post_velocity).norm() > velocity_tolerance
            || (leaf_2_velocity - leaf_2.post_velocity).norm() > velocity_tolerance
            || leaf_1_velocity.norm() > velocity_tolerance
            || leaf_2_velocity.norm() > velocity_tolerance
        {
            continue;
        }

        let separator_velocity =
            SVector::<f64, 3>::new(post_velocity[0], post_velocity[1], post_velocity[2]);
        let separator_tau = separator.disk_multiplier;
        let separator_kkt =
            separator_velocity.fixed_rows::<2>(1) + separator_tau * p_0.fixed_rows::<2>(1);
        if separator.regime != CoupledCoulombRegime::Sliding
            || p_0[0] <= impulse_tolerance
            || (FRICTION * p_0[0] - p_0.fixed_rows::<2>(1).norm()).abs() > impulse_tolerance
            || separator_velocity[0].abs() > velocity_tolerance
            || separator_tau <= 0.0
            || separator_kkt.norm() > velocity_tolerance
        {
            continue;
        }

        let energy_delta = impulse.dot(q) + 0.5 * impulse.dot(&(w * impulse));
        let energy_tolerance = TOLERANCE * (1.0 + q.norm() * impulse.norm()) * 500.0;
        if energy_delta > energy_tolerance {
            continue;
        }

        candidates.push(StarCandidate {
            impulse,
            post_velocity,
            separator_regime: separator.regime,
            separator_tau,
            energy_delta,
            effective_operator,
            effective_free_velocity,
            leaf_disk_margins: [leaf_1.strict_disk_margin, leaf_2.strict_disk_margin],
        });
    }
    Ok(candidates)
}

fn swap_star_leaves(w: &Matrix9, q: &Vector9) -> (Matrix9, Vector9) {
    let permutation = [0, 1, 2, 6, 7, 8, 3, 4, 5];
    let permuted_w = Matrix9::from_fn(|row, column| w[(permutation[row], permutation[column])]);
    let permuted_q = Vector9::from_fn(|row, _| q[permutation[row]]);
    (permuted_w, permuted_q)
}

#[test]
fn two_sticking_leaves_reduce_to_one_sliding_separator_and_reconstruct_full_island() {
    let (w, q) = star_body_generated_fixture(-0.25);
    assert!(w.cholesky().is_some());
    assert_eq!(w[(3, 6)], 0.0);
    assert_eq!(w[(6, 3)], 0.0);
    assert_eq!(w[(0, 3)], COUPLING);
    assert_eq!(w[(0, 6)], COUPLING);

    let candidates = solve_star_with_strict_sticking_leaves(&w, &q).unwrap();
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];

    assert_eq!(candidate.separator_regime, CoupledCoulombRegime::Sliding);
    assert!((candidate.impulse[0] - 1.0).abs() < 1.0e-9);
    assert!((candidate.impulse[1] + 0.5).abs() < 1.0e-9);
    assert!((candidate.impulse[3] - 1.0).abs() < 1.0e-9);
    assert!(candidate.impulse[4].abs() < 1.0e-9);
    assert!((candidate.impulse[6] - 1.0).abs() < 1.0e-9);
    assert!(candidate.impulse[7].abs() < 1.0e-9);

    assert!((candidate.effective_operator[(0, 0)] - 0.5).abs() < 1.0e-12);
    assert!((candidate.effective_free_velocity[0] + 0.5).abs() < 1.0e-12);
    assert!((candidate.effective_free_velocity[1] - 0.75).abs() < 1.0e-12);

    assert!(candidate.post_velocity.fixed_rows::<3>(0).norm() > 0.49);
    assert!(candidate.post_velocity[0].abs() < 1.0e-9);
    assert!(candidate.post_velocity[1] > 0.49);
    assert!(candidate.post_velocity.fixed_rows::<3>(3).norm() < 1.0e-9);
    assert!(candidate.post_velocity.fixed_rows::<3>(6).norm() < 1.0e-9);
    assert!((candidate.separator_tau - 1.0).abs() < 1.0e-8);
    assert!(
        candidate
            .leaf_disk_margins
            .iter()
            .all(|margin| *margin > 0.49)
    );
    assert!((candidate.energy_delta + 0.875).abs() < 1.0e-9);
    assert!(candidate.energy_delta < 0.0);

    // Re-evaluate every contact law and the energy from the original 9x9 system.
    let independently_reconstructed_velocity = q + w * candidate.impulse;
    assert!((independently_reconstructed_velocity - candidate.post_velocity).norm() < 1.0e-12);
    for offset in [3_usize, 6_usize] {
        let leaf_impulse = candidate.impulse.fixed_rows::<3>(offset);
        let leaf_velocity = candidate.post_velocity.fixed_rows::<3>(offset);
        assert!(leaf_impulse[0] > 0.0);
        assert!(FRICTION * leaf_impulse[0] - leaf_impulse.fixed_rows::<2>(1).norm() > 0.49);
        assert!(leaf_velocity.norm() < 1.0e-9);
    }
}

#[test]
fn leaf_sticking_reduction_is_invariant_to_leaf_permutation() {
    let (w, q) = star_body_generated_fixture(-0.25);
    let original = solve_star_with_strict_sticking_leaves(&w, &q).unwrap();
    let (permuted_w, permuted_q) = swap_star_leaves(&w, &q);
    let permuted = solve_star_with_strict_sticking_leaves(&permuted_w, &permuted_q).unwrap();

    assert_eq!(original.len(), 1);
    assert_eq!(permuted.len(), 1);
    assert!(
        (original[0].impulse.fixed_rows::<3>(0) - permuted[0].impulse.fixed_rows::<3>(0)).norm()
            < 1.0e-10
    );
    assert!(
        (original[0].impulse.fixed_rows::<3>(3) - permuted[0].impulse.fixed_rows::<3>(6)).norm()
            < 1.0e-10
    );
    assert!(
        (original[0].impulse.fixed_rows::<3>(6) - permuted[0].impulse.fixed_rows::<3>(3)).norm()
            < 1.0e-10
    );
}

#[test]
fn leaf_with_noninterior_friction_impulse_is_not_promoted_as_sticking() {
    let (w, q) = star_body_generated_fixture(-0.8);
    let candidates = solve_star_with_strict_sticking_leaves(&w, &q).unwrap();
    assert!(candidates.is_empty());
}

#[test]
fn sticking_leaf_message_returns_affine_schur_terms_and_a_local_admissibility_response() {
    let message = StickingLeafMessage {
        self_operator: Matrix3::from_diagonal(&SVector::<f64, 3>::new(2.0, 1.0, 1.0)),
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.25, 0.25)),
        free_velocity: SVector::<f64, 3>::new(-1.5, 0.125, 0.0),
        friction_coefficient: FRICTION,
    };
    let separator_impulse = SVector::<f64, 3>::new(1.0, -0.5, 0.0);

    let (response_offset, separator_gain) = message.affine_response_coefficients().unwrap();
    assert!((response_offset[0] - 0.75).abs() < 1.0e-12);
    assert!((response_offset[1] + 0.125).abs() < 1.0e-12);
    assert!((separator_gain[(0, 0)] - 0.25).abs() < 1.0e-12);
    assert!((separator_gain[(1, 1)] + 0.25).abs() < 1.0e-12);

    let (schur_operator, schur_free_velocity) = message.schur_terms().unwrap();
    assert!((schur_operator[(0, 0)] - 0.125).abs() < 1.0e-12);
    assert!((schur_operator[(1, 1)] - 0.0625).abs() < 1.0e-12);
    assert!((schur_free_velocity[0] - 0.375).abs() < 1.0e-12);
    assert!((schur_free_velocity[1] - 0.03125).abs() < 1.0e-12);

    let response = message
        .response_at(&separator_impulse, TOLERANCE)
        .unwrap()
        .expect("strictly loaded, sticking leaf response");
    assert!((response.impulse[0] - 1.0).abs() < 1.0e-12);
    assert!(response.impulse.fixed_rows::<2>(1).norm() < 1.0e-12);
    assert!(response.post_velocity.norm() < 1.0e-12);
    assert!((response.strict_disk_margin - 0.5).abs() < 1.0e-12);

    // Nearby separator impulses remain inside the same local strict-sticking
    // response region, and the message returns the corresponding affine impulse.
    let nearby_separator_impulse = SVector::<f64, 3>::new(1.1, -0.4, 0.0);
    let nearby_response = message
        .response_at(&nearby_separator_impulse, TOLERANCE)
        .unwrap()
        .expect("nearby separator impulse remains in the leaf sticking region");
    assert!((nearby_response.impulse[0] - 1.025).abs() < 1.0e-12);
    assert!((nearby_response.impulse[1] + 0.025).abs() < 1.0e-12);
    assert!(nearby_response.strict_disk_margin > 0.48);

    let reduced_c = Matrix3::identity() - schur_operator;
    let reduced_q = -SVector::<f64, 3>::new(0.0, 0.0, 0.0) - schur_free_velocity;
    assert!(
        (reduced_c - Matrix3::from_diagonal(&SVector::<f64, 3>::new(0.875, 0.9375, 0.9375))).norm()
            < 1.0e-12
    );
    assert!((reduced_q[0] + 0.375).abs() < 1.0e-12);
    assert!((reduced_q[1] + 0.03125).abs() < 1.0e-12);
}

#[test]
fn separating_leaf_message_preserves_strict_boundary_and_signorini_strata() {
    let message = SeparatingLeafMessage {
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.25, 0.25)),
        free_velocity: SVector::<f64, 3>::new(1.0, 0.125, -0.25),
    };
    let separator_inside = SVector::<f64, 3>::new(1.0, -0.5, 0.0);
    let strict = message.response_at(&separator_inside, TOLERANCE).unwrap();
    assert_eq!(
        strict.classification,
        SeparatingLeafClassification::StrictlySeparating
    );
    assert!((strict.normal_velocity_margin - 0.5).abs() < 1.0e-12);
    assert_eq!(strict.impulse, SVector::<f64, 3>::zeros());
    assert!((strict.post_velocity[1] - 0.0).abs() < 1.0e-12);
    assert!((strict.post_velocity[2] + 0.25).abs() < 1.0e-12);

    // Moving the separator normal impulse to 2 reaches u_n=0 exactly.
    // The contact remains complementarity-admissible, but is not a strict
    // separating branch and must be retained as a boundary stratum.
    let separator_boundary = SVector::<f64, 3>::new(2.0, -0.5, 0.0);
    let boundary = message.response_at(&separator_boundary, TOLERANCE).unwrap();
    assert_eq!(
        boundary.classification,
        SeparatingLeafClassification::BoundaryWithinTolerance
    );
    assert!(boundary.normal_velocity_margin.abs() <= 1.0e-12);
    assert_eq!(boundary.impulse, SVector::<f64, 3>::zeros());

    // Beyond the boundary, p_n=0 yields a strict Signorini violation.
    let separator_outside = SVector::<f64, 3>::new(3.0, -0.5, 0.0);
    let outside = message.response_at(&separator_outside, TOLERANCE).unwrap();
    assert_eq!(
        outside.classification,
        SeparatingLeafClassification::SignoriniViolation
    );
    assert!(outside.normal_velocity_margin < -0.49);
    assert_eq!(outside.impulse, SVector::<f64, 3>::zeros());

    let (schur_operator, schur_free_velocity) = message.schur_terms().unwrap();
    assert_eq!(schur_operator, Matrix3::zeros());
    assert_eq!(schur_free_velocity, SVector::<f64, 3>::zeros());

    assert!(message.response_at(&separator_inside, 0.0).is_err());
    assert!(
        message
            .response_at(&SVector::<f64, 3>::new(f64::NAN, 0.0, 0.0), TOLERANCE)
            .is_err()
    );
}

#[test]
fn separating_leaf_box_partition_preserves_cover_and_retains_boundary_cells() {
    let message = SeparatingLeafMessage {
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.0, 0.0)),
        free_velocity: SVector::<f64, 3>::new(1.0, 0.0, 0.0),
    };
    let root = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0, -0.5, -0.5),
        SVector::<f64, 3>::new(3.0, 0.5, 0.5),
    )
    .unwrap();

    let partition = message.partition_separator_box(&root, 3, 64).unwrap();
    assert!(partition.coverage_preserved);
    assert!(!partition.classification_complete);
    assert!(!partition.node_budget_exhausted);
    assert_eq!(partition.cells.len(), 6);
    assert_eq!(
        partition
            .cells
            .iter()
            .filter(|cell| {
                cell.classification == SeparatingBoxClassification::StrictlySeparating
            })
            .count(),
        2
    );
    assert_eq!(
        partition
            .cells
            .iter()
            .filter(|cell| {
                cell.classification == SeparatingBoxClassification::SignoriniViolation
            })
            .count(),
        2
    );
    assert_eq!(
        partition
            .cells
            .iter()
            .filter(
                |cell| cell.classification == SeparatingBoxClassification::BoundaryOrInconclusive
            )
            .count(),
        2
    );
    assert!(message.verify_box_cover(&root, &partition.cells));

    let mut sorted_intervals = partition
        .cells
        .iter()
        .map(|cell| (cell.separator_box.lower[0], cell.separator_box.upper[0]))
        .collect::<Vec<_>>();
    sorted_intervals.sort_by(|left, right| left.0.total_cmp(&right.0));
    assert_eq!(sorted_intervals.first().unwrap().0, 1.0);
    assert_eq!(sorted_intervals.last().unwrap().1, 3.0);
    for adjacent in sorted_intervals.windows(2) {
        assert_eq!(adjacent[0].1, adjacent[1].0);
    }

    let mut dropped_child = partition.cells.clone();
    dropped_child.pop();
    assert!(!message.verify_box_cover(&root, &dropped_child));

    // Same input bits and policy produce a byte-for-byte equivalent lineage,
    // class and box sequence.
    assert_eq!(
        partition,
        message.partition_separator_box(&root, 3, 64).unwrap()
    );
}

#[test]
fn separating_leaf_box_partition_retains_unresolved_budget_and_unsplittable_frontiers() {
    let message = SeparatingLeafMessage {
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.0, 0.0)),
        free_velocity: SVector::<f64, 3>::new(1.0, 0.0, 0.0),
    };
    let crossing = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0, 0.0, 0.0),
        SVector::<f64, 3>::new(3.0, 0.0, 0.0),
    )
    .unwrap();

    let budgeted = message.partition_separator_box(&crossing, 16, 1).unwrap();
    assert!(budgeted.coverage_preserved);
    assert!(budgeted.node_budget_exhausted);
    assert!(!budgeted.classification_complete);
    assert_eq!(budgeted.cells.len(), 2);
    assert!(budgeted.cells.iter().all(|cell| {
        cell.classification == SeparatingBoxClassification::BoundaryOrInconclusive
    }));
    assert!(message.verify_box_cover(&crossing, &budgeted.cells));

    let boundary = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(2.0, 0.0, 0.0),
        SVector::<f64, 3>::new(2.0, 0.0, 0.0),
    )
    .unwrap();
    let unsplittable = message.partition_separator_box(&boundary, 8, 16).unwrap();
    assert!(unsplittable.coverage_preserved);
    assert!(!unsplittable.node_budget_exhausted);
    assert_eq!(unsplittable.cells.len(), 1);
    assert_eq!(
        unsplittable.cells[0].classification,
        SeparatingBoxClassification::BoundaryOrInconclusive
    );

    let adjacent = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0, 0.0, 0.0),
        SVector::<f64, 3>::new(f64::from_bits(1.0_f64.to_bits() + 1), 0.0, 0.0),
    )
    .unwrap();
    let no_midpoint = message.partition_separator_box(&adjacent, 8, 16).unwrap();
    assert!(no_midpoint.coverage_preserved);
    assert_eq!(no_midpoint.cells.len(), 1);
    assert_eq!(
        no_midpoint.cells[0].classification,
        SeparatingBoxClassification::BoundaryOrInconclusive
    );

    assert!(message.partition_separator_box(&crossing, 2, 0).is_err());
}

#[test]
fn separator_message_composition_preserves_strict_boundary_and_excluded_cells() {
    let left_message = SeparatingLeafMessage {
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.0, 0.0)),
        free_velocity: SVector::<f64, 3>::new(1.0, 0.0, 0.0),
    };
    let right_message = SeparatingLeafMessage {
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.0, 0.0)),
        free_velocity: SVector::<f64, 3>::new(2.0, 0.0, 0.0),
    };
    let root = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0, -0.5, -0.5),
        SVector::<f64, 3>::new(5.0, 0.5, 0.5),
    )
    .unwrap();

    let left_partition = left_message.partition_separator_box(&root, 3, 128).unwrap();
    let right_partition = right_message.partition_separator_box(&root, 3, 128).unwrap();
    let composed = left_message
        .compose_separator_partitions(&left_partition, &right_message, &right_partition)
        .unwrap();

    assert!(composed.coverage_preserved);
    assert_eq!(
        composed.candidate_pairs,
        left_partition.cells.len() * right_partition.cells.len()
    );
    assert!(composed.empty_intersections > 0);
    assert!(!composed.cells.is_empty());
    assert!(left_message.verify_composed_partition(
        &left_partition,
        &right_message,
        &right_partition,
        &composed
    ));
    assert!(composed.cells.iter().any(|cell| {
        cell.classification == ComposedSeparatorClassification::AllLocalHypothesesStrict
    }));
    assert!(composed.cells.iter().any(|cell| {
        cell.classification == ComposedSeparatorClassification::LocalHypothesisExcluded
    }));
    assert!(composed.cells.iter().any(|cell| {
        cell.classification == ComposedSeparatorClassification::BoundaryOrInconclusive
    }));

    let mut dropped = composed.clone();
    dropped.cells.pop();
    assert!(!left_message.verify_composed_partition(
        &left_partition,
        &right_message,
        &right_partition,
        &dropped
    ));

    let mismatched_root = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0, -0.5, -0.5),
        SVector::<f64, 3>::new(4.0, 0.5, 0.5),
    )
    .unwrap();
    let mismatched = right_message.partition_separator_box(&mismatched_root, 2, 64).unwrap();
    assert!(
        left_message
            .compose_separator_partitions(&left_partition, &right_message, &mismatched)
            .is_err()
    );
}

#[test]
fn separator_message_composition_retains_equal_geometry_with_distinct_branch_lineage() {
    let message = SeparatingLeafMessage {
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.0, 0.0)),
        free_velocity: SVector::<f64, 3>::new(1.0, 0.0, 0.0),
    };
    let root = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0, -0.5, -0.5),
        SVector::<f64, 3>::new(3.0, 0.5, 0.5),
    )
    .unwrap();
    let partition = message.partition_separator_box(&root, 1, 16).unwrap();
    assert_eq!(partition.cells.len(), 2);

    let composed = message
        .compose_separator_partitions(&partition, &message, &partition)
        .unwrap();
    let same_geometry = composed
        .cells
        .iter()
        .filter(|cell| cell.separator_box.lower == SVector::<f64, 3>::new(2.0, -0.5, -0.5)
            && cell.separator_box.upper == SVector::<f64, 3>::new(2.0, 0.5, 0.5))
        .collect::<Vec<_>>();
    assert_eq!(same_geometry.len(), 2);
    assert_ne!(same_geometry[0].left_lineage, same_geometry[1].left_lineage);
    assert_ne!(same_geometry[0].right_lineage, same_geometry[1].right_lineage);
    assert!(message.verify_composed_partition(
        &partition,
        &message,
        &partition,
        &composed
    ));
}

#[test]
fn sticking_leaf_interval_message_certifies_strict_boxes_and_abstains_at_boundaries() {
    let message = StickingLeafMessage {
        self_operator: Matrix3::from_diagonal(&SVector::<f64, 3>::new(2.0, 1.0, 1.0)),
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.25, 0.25)),
        free_velocity: SVector::<f64, 3>::new(-1.5, 0.125, 0.0),
        friction_coefficient: 0.5,
    };

    // The pointwise affine map is a=(3/4,-1/8,0),
    // K=diag(1/4,-1/4,-1/4). This whole box has positive normal load
    // and remains strictly inside the Coulomb disk.
    let strict_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(0.875, -0.5, -0.125),
        SVector::<f64, 3>::new(1.125, -0.375, 0.125),
    )
    .unwrap();
    let strict = message.response_over_box(&strict_box).unwrap();
    assert_eq!(
        strict.classification,
        StickingBoxClassification::StrictlySticking
    );
    assert!(strict.impulse_bounds[0].lower > 0.96);
    assert!(strict.impulse_bounds[0].upper < 1.04);
    assert!(strict.disk_margin_bounds.lower > 0.43);
    assert!(
        strict
            .local_residual_bounds
            .iter()
            .all(|bound| bound.contains_zero())
    );

    // Exact boundary: lambda=1, j_t=(-1/2,0), mu=1/2.
    // A zero-radius separator box must not become a strict certificate.
    let boundary_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0, 1.5, 0.0),
        SVector::<f64, 3>::new(1.0, 1.5, 0.0),
    )
    .unwrap();
    let boundary = message.response_over_box(&boundary_box).unwrap();
    assert_eq!(
        boundary.classification,
        StickingBoxClassification::BoundaryOrInconclusive
    );
    assert!(boundary.disk_margin_bounds.contains_zero());

    // The entire box is beyond the disk boundary while the normal load is 1.
    let outside_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0, 1.625, 0.0),
        SVector::<f64, 3>::new(1.0, 1.625, 0.0),
    )
    .unwrap();
    let outside = message.response_over_box(&outside_box).unwrap();
    assert_eq!(
        outside.classification,
        StickingBoxClassification::OutsideStrictStickingDomain
    );
    assert!(outside.disk_margin_bounds.upper < 0.0);

    // The normal-load interval crosses zero; it cannot be certified strict.
    let normal_boundary_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(-3.125, -0.5, 0.0),
        SVector::<f64, 3>::new(-2.875, -0.5, 0.0),
    )
    .unwrap();
    let normal_boundary = message.response_over_box(&normal_boundary_box).unwrap();
    assert_eq!(
        normal_boundary.classification,
        StickingBoxClassification::BoundaryOrInconclusive
    );
}

#[test]
fn sticking_leaf_interval_solve_fails_closed_on_unproved_spd_pivots() {
    for self_operator in [
        Matrix3::from_diagonal(&SVector::<f64, 3>::new(1.0, -1.0, 1.0)),
        Matrix3::from_diagonal(&SVector::<f64, 3>::new(1.0, 0.0, 1.0)),
        Matrix3::new(1.0, 0.1, 0.0, 0.2, 1.0, 0.0, 0.0, 0.0, 1.0),
    ] {
        let message = StickingLeafMessage {
            self_operator,
            separator_coupling: Matrix3::identity(),
            free_velocity: SVector::<f64, 3>::new(-1.0, 0.0, 0.0),
            friction_coefficient: 0.5,
        };
        let separator_box =
            SeparatorImpulseBox::new(SVector::<f64, 3>::zeros(), SVector::<f64, 3>::zeros())
                .unwrap();
        assert!(message.response_over_box(&separator_box).is_err());
    }
}

#[test]
fn separating_leaf_interval_messages_certify_only_boxes_with_a_strict_sign() {
    let message = SeparatingLeafMessage {
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.25, 0.25)),
        free_velocity: SVector::<f64, 3>::new(1.0, 0.125, -0.25),
    };

    // Exact dyadic reference: u_n = 1 - (1/2) p_S,n.
    // For p_S,n in [1,5/4], its exact range is [3/8,1/2].
    let strict_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0, -0.5, -0.5),
        SVector::<f64, 3>::new(1.25, 0.5, 0.5),
    )
    .unwrap();
    let strict = message.response_over_box(&strict_box).unwrap();
    assert_eq!(
        strict.classification,
        SeparatingBoxClassification::StrictlySeparating
    );
    assert!(strict.post_velocity_bounds[0].lower <= 0.375);
    assert!(strict.post_velocity_bounds[0].upper >= 0.5);
    assert_eq!(strict.impulse, SVector::<f64, 3>::zeros());
    assert!(strict.post_velocity_bounds[1].lower <= 0.0);
    assert!(strict.post_velocity_bounds[1].upper >= 0.25);
    assert!(strict.post_velocity_bounds[2].lower <= -0.375);
    assert!(strict.post_velocity_bounds[2].upper >= -0.125);

    // Exact dyadic reference: for p_S,n in [3,13/4], u_n is [-5/8,-1/2].
    let violating_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(3.0, -0.5, -0.5),
        SVector::<f64, 3>::new(3.25, 0.5, 0.5),
    )
    .unwrap();
    let violating = message.response_over_box(&violating_box).unwrap();
    assert_eq!(
        violating.classification,
        SeparatingBoxClassification::SignoriniViolation
    );
    assert!(violating.post_velocity_bounds[0].lower <= -0.625);
    assert!(violating.post_velocity_bounds[0].upper >= -0.5);

    // This box straddles the exact transition hyperplane p_S,n=2.
    let crossing_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.875, -0.5, -0.5),
        SVector::<f64, 3>::new(2.125, 0.5, 0.5),
    )
    .unwrap();
    let crossing = message.response_over_box(&crossing_box).unwrap();
    assert_eq!(
        crossing.classification,
        SeparatingBoxClassification::BoundaryOrInconclusive
    );
    assert!(crossing.post_velocity_bounds[0].lower <= 0.0);
    assert!(crossing.post_velocity_bounds[0].upper >= 0.0);

    // A zero-radius box exactly on the transition is still not strict:
    // outward rounding encloses the exact zero from both sides.
    let boundary_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(2.0, 0.0, 0.0),
        SVector::<f64, 3>::new(2.0, 0.0, 0.0),
    )
    .unwrap();
    let boundary = message.response_over_box(&boundary_box).unwrap();
    assert_eq!(
        boundary.classification,
        SeparatingBoxClassification::BoundaryOrInconclusive
    );
    assert!(boundary.post_velocity_bounds[0].lower <= 0.0);
    assert!(boundary.post_velocity_bounds[0].upper >= 0.0);
}

#[test]
fn separating_leaf_interval_messages_reject_malformed_and_overflowing_boxes() {
    assert!(
        SeparatorImpulseBox::new(
            SVector::<f64, 3>::new(1.0, 0.0, 0.0),
            SVector::<f64, 3>::new(0.0, 0.0, 0.0),
        )
        .is_err()
    );
    assert!(
        SeparatorImpulseBox::new(
            SVector::<f64, 3>::new(f64::NAN, 0.0, 0.0),
            SVector::<f64, 3>::new(1.0, 0.0, 0.0),
        )
        .is_err()
    );

    let overflowing = SeparatingLeafMessage {
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(1.0e308, 0.0, 0.0)),
        free_velocity: SVector::<f64, 3>::zeros(),
    };
    let box_with_large_coordinate = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0e308, 0.0, 0.0),
        SVector::<f64, 3>::new(1.0e308, 0.0, 0.0),
    )
    .unwrap();
    assert!(
        overflowing
            .response_over_box(&box_with_large_coordinate)
            .is_err()
    );
}

#[test]
fn one_sticking_and_one_separating_leaf_reduce_to_sliding_separator() {
    // Four-body chain A-B-C-D, with v_A=(-1/2,-1/4,0),
    // v_B=(0,0,0), v_C=(0,-1,0), v_D=(-1,-1,0).
    // Derive both W and q from the same body-level J/M/v_free construction.
    let jacobian = star_contact_jacobian();
    let inverse_mass = Matrix12::identity() * 0.5;
    let w = jacobian * inverse_mass * jacobian.transpose();
    let free_body_velocity = Vector12::from_row_slice(&[
        -0.5, -0.25, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, -1.0, -1.0, 0.0,
    ]);
    let q = jacobian * free_body_velocity;

    let block = |row_offset: usize, column_offset: usize| {
        Matrix3::from_fn(|row, column| w[(row + row_offset, column + column_offset)])
    };
    let vector = |offset: usize| SVector::<f64, 3>::new(q[offset], q[offset + 1], q[offset + 2]);
    let separator_block = block(0, 0);

    let sticking = StickingLeafMessage {
        self_operator: block(3, 3),
        separator_coupling: block(0, 3),
        free_velocity: vector(3),
        friction_coefficient: FRICTION,
    };
    let separating = SeparatingLeafMessage {
        separator_coupling: block(0, 6),
        free_velocity: vector(6),
    };

    // The sticking leaf contributes its Schur terms. The separating leaf
    // contributes exactly zero because its impulse is identically zero.
    let (sticking_w, sticking_q) = sticking.schur_terms().unwrap();
    let (separating_w, separating_q) = separating.schur_terms().unwrap();
    let effective_operator = separator_block - sticking_w - separating_w;
    let effective_free_velocity = vector(0) - sticking_q - separating_q;
    assert!((effective_operator - Matrix3::identity() * 0.75).norm() < 1.0e-12);
    assert!((effective_free_velocity - SVector::<f64, 3>::new(-0.25, 0.875, 0.0)).norm() < 1.0e-12);

    let reduced = solve_coupled_signorini_coulomb(
        &effective_operator,
        &effective_free_velocity,
        FRICTION,
        TOLERANCE,
    )
    .unwrap();
    assert_eq!(reduced.candidates.len(), 1);
    let separator = &reduced.candidates[0];
    assert_eq!(separator.regime, CoupledCoulombRegime::Sliding);
    assert!((separator.impulse[0] - (1.0 / 3.0)).abs() < 1.0e-9);
    assert!((separator.impulse[1] + (1.0 / 6.0)).abs() < 1.0e-9);

    let sticking_response = sticking
        .response_at(&separator.impulse, TOLERANCE)
        .unwrap()
        .expect("leaf 1 remains strictly sticking");
    let separating_response = separating
        .response_at(&separator.impulse, TOLERANCE)
        .unwrap();
    assert_eq!(
        separating_response.classification,
        SeparatingLeafClassification::StrictlySeparating
    );
    assert!((separating_response.normal_velocity_margin - (5.0 / 6.0)).abs() < 1.0e-9);

    let impulse = Vector9::from_row_slice(&[
        separator.impulse[0],
        separator.impulse[1],
        separator.impulse[2],
        sticking_response.impulse[0],
        sticking_response.impulse[1],
        sticking_response.impulse[2],
        separating_response.impulse[0],
        separating_response.impulse[1],
        separating_response.impulse[2],
    ]);
    let post_velocity = q + w * impulse;

    // Replay the complete original 9D contact equations and every local law.
    assert!(post_velocity.fixed_rows::<3>(3).norm() < 1.0e-9);
    let body_impulse = jacobian.transpose() * impulse;
    let post_body_velocity = free_body_velocity + inverse_mass * body_impulse;
    let body_reconstructed_velocity = jacobian * post_body_velocity;
    assert!((body_reconstructed_velocity - post_velocity).norm() < 1.0e-12);
    assert!(impulse[3] > 0.0);
    assert!(FRICTION * impulse[3] - impulse.fixed_rows::<2>(4).norm() > 0.15);
    assert!(impulse.fixed_rows::<3>(6).norm() < 1.0e-12);
    assert!(post_velocity[6] > 0.8);
    assert!((post_velocity[7] - (1.0 / 12.0)).abs() < 1.0e-9);
    assert!(post_velocity[8].abs() < 1.0e-9);
    assert!(impulse[7].abs() < 1.0e-12);
    assert!(impulse[8].abs() < 1.0e-12);

    let separator_tau = separator.disk_multiplier;
    assert!(separator_tau > 0.0);
    assert!(post_velocity[0].abs() < 1.0e-9);
    let separator_kkt =
        post_velocity.fixed_rows::<2>(1) + separator_tau * separator.impulse.fixed_rows::<2>(1);
    assert!(separator_kkt.norm() < 1.0e-9);
    assert!(
        (FRICTION * separator.impulse[0] - separator.impulse.fixed_rows::<2>(1).norm()).abs()
            < 1.0e-9
    );

    let energy = impulse.dot(&q) + 0.5 * impulse.dot(&(w * impulse));
    assert!((energy + (1.0 / 3.0)).abs() < 1.0e-9);
    assert!(energy < 0.0);
}

#[test]
fn sticking_leaf_message_refuses_disk_boundary_and_invalid_self_blocks() {
    let boundary = StickingLeafMessage {
        self_operator: Matrix3::identity(),
        separator_coupling: Matrix3::zeros(),
        free_velocity: SVector::<f64, 3>::new(-1.0, 0.5, 0.0),
        friction_coefficient: FRICTION,
    };
    assert!(
        boundary
            .response_at(&SVector::<f64, 3>::zeros(), TOLERANCE)
            .unwrap()
            .is_none()
    );

    let indefinite = StickingLeafMessage {
        self_operator: Matrix3::from_diagonal(&SVector::<f64, 3>::new(1.0, -1.0, 1.0)),
        separator_coupling: Matrix3::identity(),
        free_velocity: SVector::<f64, 3>::new(-1.0, 0.0, 0.0),
        friction_coefficient: FRICTION,
    };
    assert!(indefinite.schur_terms().is_err());
    assert!(
        indefinite
            .response_at(&SVector::<f64, 3>::zeros(), TOLERANCE)
            .is_err()
    );
}

#[derive(Clone, Copy)]
struct NarySeparatorPartitionSource<'a> {
    source_id: u64,
    message: &'a SeparatingLeafMessage,
    partition: &'a SeparatorBoxPartition,
}

#[derive(Clone, Debug, PartialEq)]
struct NaryComposedSeparatorBoxCell {
    separator_box: SeparatorImpulseBox,
    source_classifications: Vec<SeparatingBoxClassification>,
    source_lineages: Vec<Vec<u8>>,
    classification: ComposedSeparatorClassification,
}

#[derive(Clone, Debug, PartialEq)]
struct NaryComposedSeparatorBoxPartition {
    root_box: SeparatorImpulseBox,
    source_ids: Vec<u64>,
    cells: Vec<NaryComposedSeparatorBoxCell>,
    candidate_combinations: usize,
    empty_intersections: usize,
    coverage_preserved: bool,
    classification_complete: bool,
}

#[derive(Clone)]
struct NaryPartialSeparatorCell {
    separator_box: SeparatorImpulseBox,
    source_classifications: Vec<SeparatingBoxClassification>,
    source_lineages: Vec<Vec<u8>>,
}

fn nary_cartesian_product_size(sizes: &[usize]) -> Option<usize> {
    sizes
        .iter()
        .try_fold(1_usize, |product, size| product.checked_mul(*size))
}

fn nary_composed_classification(
    classifications: &[SeparatingBoxClassification],
) -> ComposedSeparatorClassification {
    if classifications.contains(&SeparatingBoxClassification::SignoriniViolation) {
        ComposedSeparatorClassification::LocalHypothesisExcluded
    } else if classifications
        .iter()
        .all(|classification| *classification == SeparatingBoxClassification::StrictlySeparating)
    {
        ComposedSeparatorClassification::AllLocalHypothesesStrict
    } else {
        ComposedSeparatorClassification::BoundaryOrInconclusive
    }
}

fn canonical_nary_sources<'a>(
    sources: &'a [NarySeparatorPartitionSource<'a>],
) -> Result<Vec<NarySeparatorPartitionSource<'a>>, &'static str> {
    if sources.len() < 2 {
        return Err("n-ary separator composition requires at least two sources");
    }
    let mut ordered = sources.to_vec();
    ordered.sort_by_key(|source| source.source_id);
    if ordered
        .windows(2)
        .any(|pair| pair[0].source_id == pair[1].source_id)
    {
        return Err("n-ary separator sources must have unique stable identities");
    }

    let root = &ordered[0].partition.root_box;
    for source in &ordered {
        if source.partition.root_box != *root {
            return Err("n-ary separator sources do not share an identical root domain");
        }
        if !source.partition.coverage_preserved
            || !source
                .message
                .verify_box_cover(&source.partition.root_box, &source.partition.cells)
        {
            return Err("n-ary source partition failed independent coverage verification");
        }
    }
    Ok(ordered)
}

/// Incremental n-ary join. Every partial cell retains the complete source
/// lineage tuple; geometrically equal intersections are never deduplicated.
fn compose_nary_separator_partitions(
    sources: &[NarySeparatorPartitionSource<'_>],
) -> Result<NaryComposedSeparatorBoxPartition, &'static str> {
    let ordered = canonical_nary_sources(sources)?;
    let root_box = ordered[0].partition.root_box.clone();
    let source_ids = ordered
        .iter()
        .map(|source| source.source_id)
        .collect::<Vec<_>>();
    let source_sizes = ordered
        .iter()
        .map(|source| source.partition.cells.len())
        .collect::<Vec<_>>();
    let candidate_combinations = nary_cartesian_product_size(&source_sizes)
        .ok_or("n-ary separator Cartesian product size overflowed")?;
    if candidate_combinations == 0 {
        return Err("n-ary separator source partition is empty");
    }

    let mut partials = vec![NaryPartialSeparatorCell {
        separator_box: root_box.clone(),
        source_classifications: Vec::with_capacity(ordered.len()),
        source_lineages: Vec::with_capacity(ordered.len()),
    }];

    for source in &ordered {
        let mut next = Vec::new();
        for partial in &partials {
            for source_cell in &source.partition.cells {
                let Some(intersection) = partial
                    .separator_box
                    .intersection(&source_cell.separator_box)
                else {
                    continue;
                };
                let mut source_classifications = partial.source_classifications.clone();
                source_classifications.push(source_cell.classification);
                let mut source_lineages = partial.source_lineages.clone();
                source_lineages.push(source_cell.lineage.clone());
                next.push(NaryPartialSeparatorCell {
                    separator_box: intersection,
                    source_classifications,
                    source_lineages,
                });
            }
        }
        partials = next;
        if partials.is_empty() {
            break;
        }
    }

    let mut cells = partials
        .into_iter()
        .map(|partial| NaryComposedSeparatorBoxCell {
            classification: nary_composed_classification(&partial.source_classifications),
            separator_box: partial.separator_box,
            source_classifications: partial.source_classifications,
            source_lineages: partial.source_lineages,
        })
        .collect::<Vec<_>>();
    cells.sort_by(|left, right| left.source_lineages.cmp(&right.source_lineages));
    let empty_intersections = candidate_combinations
        .checked_sub(cells.len())
        .ok_or("n-ary output has more cells than Cartesian source tuples")?;

    let mut result = NaryComposedSeparatorBoxPartition {
        root_box,
        source_ids,
        cells,
        candidate_combinations,
        empty_intersections,
        coverage_preserved: false,
        classification_complete: false,
    };
    result.coverage_preserved = verify_nary_separator_composition(&ordered, &result);
    if !result.coverage_preserved {
        return Err(
            "n-ary separator composition failed independent tuple/intersection verification",
        );
    }
    result.classification_complete = result
        .cells
        .iter()
        .all(|cell| cell.classification != ComposedSeparatorClassification::BoundaryOrInconclusive);
    Ok(result)
}

/// Independent verifier: enumerate the full Cartesian source tuple with an
/// explicit stack, intersect each tuple directly from the source cells, and
/// compare the complete lineage-keyed map with the producer output.
fn verify_nary_separator_composition(
    ordered_sources: &[NarySeparatorPartitionSource<'_>],
    composed: &NaryComposedSeparatorBoxPartition,
) -> bool {
    let Ok(canonical) = canonical_nary_sources(ordered_sources) else {
        return false;
    };
    if canonical.len() != ordered_sources.len() {
        return false;
    }
    let root_box = &canonical[0].partition.root_box;
    if composed.root_box != *root_box {
        return false;
    }
    let expected_source_ids = canonical
        .iter()
        .map(|source| source.source_id)
        .collect::<Vec<_>>();
    if composed.source_ids != expected_source_ids {
        return false;
    }

    let sizes = canonical
        .iter()
        .map(|source| source.partition.cells.len())
        .collect::<Vec<_>>();
    let Some(candidate_combinations) = nary_cartesian_product_size(&sizes) else {
        return false;
    };
    if composed.candidate_combinations != candidate_combinations {
        return false;
    }

    let mut expected =
        std::collections::BTreeMap::<Vec<Vec<u8>>, NaryComposedSeparatorBoxCell>::new();
    let mut empty_intersections = 0_usize;
    let mut stack = vec![(
        0_usize,
        Some(root_box.clone()),
        Vec::<SeparatingBoxClassification>::with_capacity(canonical.len()),
        Vec::<Vec<u8>>::with_capacity(canonical.len()),
    )];

    while let Some((source_index, geometry, classifications, lineages)) = stack.pop() {
        if source_index == canonical.len() {
            if let Some(separator_box) = geometry {
                let key = lineages.clone();
                let cell = NaryComposedSeparatorBoxCell {
                    separator_box,
                    classification: nary_composed_classification(&classifications),
                    source_classifications: classifications,
                    source_lineages: lineages,
                };
                if expected.insert(key, cell).is_some() {
                    return false;
                }
            } else {
                let Some(next) = empty_intersections.checked_add(1) else {
                    return false;
                };
                empty_intersections = next;
            }
            continue;
        }

        for source_cell in &canonical[source_index].partition.cells {
            let next_geometry = match &geometry {
                Some(current) => current.intersection(&source_cell.separator_box),
                None => None,
            };
            let mut next_classifications = classifications.clone();
            next_classifications.push(source_cell.classification);
            let mut next_lineages = lineages.clone();
            next_lineages.push(source_cell.lineage.clone());
            stack.push((
                source_index + 1,
                next_geometry,
                next_classifications,
                next_lineages,
            ));
        }
    }

    let Some(accounted_combinations) = expected.len().checked_add(empty_intersections) else {
        return false;
    };
    if empty_intersections != composed.empty_intersections
        || expected.len() != composed.cells.len()
        || accounted_combinations != candidate_combinations
    {
        return false;
    }

    let mut observed = std::collections::BTreeSet::<Vec<Vec<u8>>>::new();
    for cell in &composed.cells {
        if cell.source_lineages.len() != canonical.len()
            || cell.source_classifications.len() != canonical.len()
            || !observed.insert(cell.source_lineages.clone())
        {
            return false;
        }
        let Some(expected_cell) = expected.get(&cell.source_lineages) else {
            return false;
        };
        if cell != expected_cell {
            return false;
        }
    }
    observed.len() == expected.len()
}

fn make_nary_separating_source(
    source_id: u64,
    axis: usize,
    free_normal_velocity: f64,
    coupling: f64,
    root_box: &SeparatorImpulseBox,
    max_depth: usize,
) -> Result<(u64, SeparatingLeafMessage, SeparatorBoxPartition), &'static str> {
    if axis >= 3 {
        return Err("separator source axis is out of range");
    }
    let mut separator_coupling = Matrix3::zeros();
    separator_coupling[(axis, 0)] = coupling;
    let message = SeparatingLeafMessage {
        separator_coupling,
        free_velocity: SVector::<f64, 3>::new(free_normal_velocity, 0.0, 0.0),
    };
    let partition = message.partition_separator_box(root_box, max_depth, 256)?;
    Ok((source_id, message, partition))
}

#[test]
fn nary_separator_join_preserves_all_three_and_six_source_lineage_tuples() {
    let root = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(-1.0, -1.0, -1.0),
        SVector::<f64, 3>::new(3.0, 3.0, 3.0),
    )
    .unwrap();
    let owned = [
        make_nary_separating_source(37, 0, 0.5, -0.5, &root, 4).unwrap(),
        make_nary_separating_source(11, 0, 1.0, -0.5, &root, 4).unwrap(),
        make_nary_separating_source(23, 1, 0.5, -0.5, &root, 4).unwrap(),
        make_nary_separating_source(41, 2, -0.5, 1.0, &root, 4).unwrap(),
        make_nary_separating_source(53, 1, 1.5, -0.5, &root, 4).unwrap(),
        make_nary_separating_source(61, 2, 0.5, -0.5, &root, 4).unwrap(),
    ];

    let three_sources = [37_u64, 11, 23]
        .into_iter()
        .map(|id| {
            let source = owned.iter().find(|source| source.0 == id).unwrap();
            NarySeparatorPartitionSource {
                source_id: source.0,
                message: &source.1,
                partition: &source.2,
            }
        })
        .collect::<Vec<_>>();
    let three = compose_nary_separator_partitions(&three_sources).unwrap();
    assert_eq!(three.source_ids, vec![11, 23, 37]);
    let three_product = nary_cartesian_product_size(
        &three_sources
            .iter()
            .map(|source| source.partition.cells.len())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(three.candidate_combinations, three_product);
    assert!(three.empty_intersections > 0);
    assert_eq!(
        three.empty_intersections + three.cells.len(),
        three.candidate_combinations
    );
    assert!(three.coverage_preserved);
    assert!(verify_nary_separator_composition(&three_sources, &three));

    let six_sources = owned
        .iter()
        .map(|source| NarySeparatorPartitionSource {
            source_id: source.0,
            message: &source.1,
            partition: &source.2,
        })
        .collect::<Vec<_>>();
    let six = compose_nary_separator_partitions(&six_sources).unwrap();
    assert_eq!(six.source_ids, vec![11, 23, 37, 41, 53, 61]);
    let six_product = nary_cartesian_product_size(
        &six_sources
            .iter()
            .map(|source| source.partition.cells.len())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(six.candidate_combinations, six_product);
    assert!(six.empty_intersections > 0);
    assert_eq!(
        six.empty_intersections + six.cells.len(),
        six.candidate_combinations
    );
    assert!(six.coverage_preserved);
    assert!(verify_nary_separator_composition(&six_sources, &six));

    assert!(six.cells.iter().any(|cell| {
        cell.classification == ComposedSeparatorClassification::LocalHypothesisExcluded
    }));
    assert!(six.cells.iter().any(|cell| {
        cell.classification == ComposedSeparatorClassification::AllLocalHypothesesStrict
    }));
    assert!(six.cells.iter().any(|cell| {
        cell.classification == ComposedSeparatorClassification::BoundaryOrInconclusive
    }));
}

#[test]
fn nary_separator_join_is_invariant_to_source_enumeration_order() {
    let root = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(-1.0, -1.0, -1.0),
        SVector::<f64, 3>::new(3.0, 3.0, 3.0),
    )
    .unwrap();
    let owned = [
        make_nary_separating_source(3, 0, 0.5, -0.5, &root, 2).unwrap(),
        make_nary_separating_source(1, 1, -0.5, 1.0, &root, 2).unwrap(),
        make_nary_separating_source(2, 2, 1.0, -0.5, &root, 2).unwrap(),
    ];
    let order_a = [0_usize, 1, 2];
    let order_b = [2_usize, 0, 1];
    let make_sources = |order: &[usize]| {
        order
            .iter()
            .map(|index| {
                let source = &owned[*index];
                NarySeparatorPartitionSource {
                    source_id: source.0,
                    message: &source.1,
                    partition: &source.2,
                }
            })
            .collect::<Vec<_>>()
    };

    let a_sources = make_sources(&order_a);
    let b_sources = make_sources(&order_b);
    let a = compose_nary_separator_partitions(&a_sources).unwrap();
    let b = compose_nary_separator_partitions(&b_sources).unwrap();
    assert_eq!(a, b);
}

#[test]
fn nary_separator_join_rejects_missing_duplicate_and_relabelled_tuple_cells() {
    let root = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(-1.0, -1.0, -1.0),
        SVector::<f64, 3>::new(3.0, 3.0, 3.0),
    )
    .unwrap();
    let owned = [
        make_nary_separating_source(1, 0, 0.5, -0.5, &root, 2).unwrap(),
        make_nary_separating_source(2, 1, 0.5, -0.5, &root, 2).unwrap(),
        make_nary_separating_source(3, 2, 0.5, -0.5, &root, 2).unwrap(),
    ];
    let sources = owned
        .iter()
        .map(|source| NarySeparatorPartitionSource {
            source_id: source.0,
            message: &source.1,
            partition: &source.2,
        })
        .collect::<Vec<_>>();
    let joined = compose_nary_separator_partitions(&sources).unwrap();
    assert!(verify_nary_separator_composition(&sources, &joined));

    let duplicate_identity = [
        sources[0],
        NarySeparatorPartitionSource {
            source_id: sources[0].source_id,
            message: sources[1].message,
            partition: sources[1].partition,
        },
    ];
    assert!(compose_nary_separator_partitions(&duplicate_identity).is_err());

    let mut missing = joined.clone();
    missing.cells.pop();
    assert!(!verify_nary_separator_composition(&sources, &missing));

    let mut duplicated = joined.clone();
    duplicated.cells.push(duplicated.cells[0].clone());
    assert!(!verify_nary_separator_composition(&sources, &duplicated));

    let mut relabelled = joined.clone();
    relabelled.cells[0].source_lineages[0].push(1);
    assert!(!verify_nary_separator_composition(&sources, &relabelled));

    let mut changed_label = joined.clone();
    changed_label.cells[0].source_classifications[0] =
        SeparatingBoxClassification::SignoriniViolation;
    assert!(!verify_nary_separator_composition(&sources, &changed_label));

    let mut changed_geometry = joined;
    changed_geometry.cells[0].separator_box.lower[0] += 0.125;
    assert!(!verify_nary_separator_composition(
        &sources,
        &changed_geometry
    ));
}

#[test]
fn nary_separator_join_keeps_lineage_distinct_cells_with_equal_geometry() {
    let root = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(-1.0, -1.0, -1.0),
        SVector::<f64, 3>::new(3.0, 3.0, 3.0),
    )
    .unwrap();
    let owned = [
        make_nary_separating_source(1, 0, 0.5, -0.5, &root, 1).unwrap(),
        make_nary_separating_source(2, 0, 0.5, -0.5, &root, 1).unwrap(),
        make_nary_separating_source(3, 1, 0.5, -0.5, &root, 1).unwrap(),
    ];
    let sources = owned
        .iter()
        .map(|source| NarySeparatorPartitionSource {
            source_id: source.0,
            message: &source.1,
            partition: &source.2,
        })
        .collect::<Vec<_>>();
    let joined = compose_nary_separator_partitions(&sources).unwrap();

    let duplicate_geometry = joined
        .cells
        .iter()
        .filter(|cell| cell.separator_box.lower[0] == 1.0 && cell.separator_box.upper[0] == 1.0)
        .collect::<Vec<_>>();
    assert!(duplicate_geometry.len() >= 2);
    let distinct_lineages = duplicate_geometry
        .iter()
        .map(|cell| cell.source_lineages.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(distinct_lineages.len(), duplicate_geometry.len());
    assert!(verify_nary_separator_composition(&sources, &joined));
}

#[test]
fn nary_separator_cartesian_count_uses_checked_multiplication() {
    assert_eq!(nary_cartesian_product_size(&[2, 3, 5, 7]), Some(210));
    assert_eq!(nary_cartesian_product_size(&[usize::MAX, 2]), None);
    assert_eq!(nary_cartesian_product_size(&[]), Some(1));
}
