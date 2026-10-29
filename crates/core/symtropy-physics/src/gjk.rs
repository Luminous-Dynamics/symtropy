// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! GJK (Gilbert–Johnson–Keerthi) intersection test generalized to N dimensions.
//!
//! The key insight: two convex shapes intersect if and only if their
//! Minkowski difference contains the origin. GJK iteratively builds a
//! simplex in the Minkowski difference, checking if the origin is enclosed.
//!
//! Works for any dimension because:
//! - The support function is dimension-agnostic
//! - The simplex grows from point → line → triangle → ... → (D+1)-simplex
//! - Each iteration uses dot products and vector arithmetic (dimension-independent)

use arrayvec::ArrayVec;
use nalgebra::SVector;
use symtropy_math::{Point, Shape, Transform};

use crate::support_map::{TransformedShape, WorldSupportMap};

/// Maximum GJK iterations before giving up.
const MAX_ITERATIONS: usize = 64;

/// Maximum simplex size: D+1 vertices. Cap at 5 (D=4).
const MAX_SIMPLEX: usize = 5;

/// Stack-allocated simplex type. Zero heap allocation.
type Simplex<const D: usize> = ArrayVec<SVector<f64, D>, MAX_SIMPLEX>;

/// Why a GJK query did not produce a definitive geometric classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GjkFailureReason {
    /// The support-map center or another authored query input was non-finite.
    NonFiniteInput,
    /// A support map returned a non-finite support point.
    NonFiniteSupport,
    /// Finite inputs produced non-finite intermediate arithmetic.
    NonFiniteArithmetic,
    /// The solver repeated a support point before establishing convergence.
    DuplicateSupport,
    /// No usable closest point could be computed for the current simplex.
    DegenerateSimplex,
    /// The simplex exceeded the algorithm's fixed storage capacity.
    SimplexCapacityExceeded,
    /// The iteration budget was exhausted without a definitive result.
    IterationLimit,
}

/// Termination outcome for a GJK query.
///
/// Separated is reserved for a converged separation conclusion. A failed or
/// exhausted numerical path must return Indeterminate, never a false negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GjkTermination {
    Intersecting,
    Separated,
    Indeterminate(GjkFailureReason),
}

/// Result of a GJK intersection test.
///
/// \`intersecting\` is retained for source compatibility. New code should branch
/// on \`termination\`; \`false\` alone is not proof that the shapes are separated.
#[derive(Clone, Debug)]
pub struct GjkResult<const D: usize> {
    pub intersecting: bool,
    /// Categorical solver result; the authoritative status field.
    pub termination: GjkTermination,
    /// The final simplex (safe as an EPA seed only when status is Intersecting).
    pub simplex: Simplex<D>,
    /// Number of iterations used.
    pub iterations: usize,
}

fn indeterminate<const D: usize>(
    simplex: Simplex<D>,
    iterations: usize,
    reason: GjkFailureReason,
) -> GjkResult<D> {
    GjkResult {
        intersecting: false,
        termination: GjkTermination::Indeterminate(reason),
        simplex,
        iterations,
    }
}

#[inline]
fn vector_is_finite<const D: usize>(vector: &SVector<f64, D>) -> bool {
    vector.iter().all(|value| value.is_finite())
}

fn simplex_is_finite<const D: usize>(simplex: &Simplex<D>) -> bool {
    simplex.iter().all(|point| vector_is_finite(point))
}

/// Test if two translated shapes intersect using the GJK algorithm.
///
/// This compatibility entry point preserves the original API. New code should
/// prefer [`intersects_transformed`] so orientation is included.
pub fn intersects<const D: usize>(
    shape_a: &dyn Shape<D>,
    pos_a: &SVector<f64, D>,
    shape_b: &dyn Shape<D>,
    pos_b: &SVector<f64, D>,
) -> GjkResult<D> {
    let transform_a = Transform::from_translation(Point(*pos_a));
    let transform_b = Transform::from_translation(Point(*pos_b));
    intersects_transformed(shape_a, &transform_a, shape_b, &transform_b)
}

/// Test two local-space shapes after their complete rigid transforms have been
/// applied. This is the canonical GJK entry point used by `PhysicsWorld`.
pub fn intersects_transformed<const D: usize>(
    shape_a: &dyn Shape<D>,
    transform_a: &Transform<D>,
    shape_b: &dyn Shape<D>,
    transform_b: &Transform<D>,
) -> GjkResult<D> {
    let map_a = TransformedShape::new(shape_a, transform_a);
    let map_b = TransformedShape::new(shape_b, transform_b);
    intersects_support_maps(&map_a, &map_b)
}

/// GJK over arbitrary world-space support maps.
pub fn intersects_support_maps<const D: usize>(
    shape_a: &dyn WorldSupportMap<D>,
    shape_b: &dyn WorldSupportMap<D>,
) -> GjkResult<D> {
    // Four dimensions need a five-vertex simplex. The 3D tetrahedron reducer
    // is not a valid facet test for a 4D simplex.
    if D == 4 {
        return intersects_support_maps_with_closest_simplex(shape_a, shape_b);
    }

    let center_a = shape_a.center_world();
    let center_b = shape_b.center_world();
    let mut simplex = Simplex::<D>::new();
    if !vector_is_finite(&center_a) || !vector_is_finite(&center_b) {
        return indeterminate(simplex, 0, GjkFailureReason::NonFiniteInput);
    }

    // Initial direction: from A to B.
    let mut direction = center_b - center_a;
    if !vector_is_finite(&direction) || !direction.norm_squared().is_finite() {
        return indeterminate(simplex, 0, GjkFailureReason::NonFiniteArithmetic);
    }
    if direction.norm_squared() < 1e-20 {
        direction = SVector::zeros();
        direction[0] = 1.0;
    }

    let first = minkowski_support(shape_a, shape_b, &direction);
    if !vector_is_finite(&first) {
        return indeterminate(simplex, 0, GjkFailureReason::NonFiniteSupport);
    }
    simplex.push(first);
    direction = -first;

    for iteration in 0..MAX_ITERATIONS {
        let direction_squared = direction.norm_squared();
        if !direction_squared.is_finite() || !simplex_is_finite(&simplex) {
            return indeterminate(
                simplex,
                iteration,
                GjkFailureReason::NonFiniteArithmetic,
            );
        }
        if direction_squared < 1e-20 {
            return GjkResult {
                intersecting: true,
                termination: GjkTermination::Intersecting,
                simplex,
                iterations: iteration,
            };
        }

        let new_point = minkowski_support(shape_a, shape_b, &direction);
        if !vector_is_finite(&new_point) {
            return indeterminate(simplex, iteration, GjkFailureReason::NonFiniteSupport);
        }
        let support_projection = new_point.dot(&direction);
        if !support_projection.is_finite() {
            return indeterminate(
                simplex,
                iteration,
                GjkFailureReason::NonFiniteArithmetic,
            );
        }
        if support_projection < -1e-10 {
            return GjkResult {
                intersecting: false,
                termination: GjkTermination::Separated,
                simplex,
                iterations: iteration,
            };
        }
        if simplex.len() >= MAX_SIMPLEX {
            return indeterminate(
                simplex,
                iteration,
                GjkFailureReason::SimplexCapacityExceeded,
            );
        }

        simplex.push(new_point);
        if do_simplex(&mut simplex, &mut direction) {
            if !simplex_is_finite(&simplex) {
                return indeterminate(
                    simplex,
                    iteration,
                    GjkFailureReason::NonFiniteArithmetic,
                );
            }
            return GjkResult {
                intersecting: true,
                termination: GjkTermination::Intersecting,
                simplex,
                iterations: iteration,
            };
        }
        if !vector_is_finite(&direction) || !simplex_is_finite(&simplex) {
            return indeterminate(
                simplex,
                iteration,
                GjkFailureReason::NonFiniteArithmetic,
            );
        }
    }

    indeterminate(
        simplex,
        MAX_ITERATIONS,
        GjkFailureReason::IterationLimit,
    )
}

/// GJK path for 4D using an exhaustive closest-point reduction over simplex faces.
///
/// A 4D simplex has at most five vertices, so enumerating its non-empty subsets
/// costs at most 31 small systems. This is intentionally bounded and stack-only.
/// It avoids applying the 3D tetrahedron facet test to a 4D simplex.
fn intersects_support_maps_with_closest_simplex<const D: usize>(
    shape_a: &dyn WorldSupportMap<D>,
    shape_b: &dyn WorldSupportMap<D>,
) -> GjkResult<D> {
    let center_a = shape_a.center_world();
    let center_b = shape_b.center_world();
    let mut simplex = Simplex::<D>::new();
    if !vector_is_finite(&center_a) || !vector_is_finite(&center_b) {
        return indeterminate(simplex, 0, GjkFailureReason::NonFiniteInput);
    }

    let mut direction = center_b - center_a;
    if !vector_is_finite(&direction) || !direction.norm_squared().is_finite() {
        return indeterminate(simplex, 0, GjkFailureReason::NonFiniteArithmetic);
    }
    if direction.norm_squared() < 1e-20 {
        direction = SVector::zeros();
        direction[0] = 1.0;
    }

    let first = minkowski_support(shape_a, shape_b, &direction);
    if !vector_is_finite(&first) {
        return indeterminate(simplex, 0, GjkFailureReason::NonFiniteSupport);
    }
    simplex.push(first);
    let mut closest = first;

    for iteration in 0..MAX_ITERATIONS {
        let closest_squared = closest.norm_squared();
        if !closest_squared.is_finite() {
            return indeterminate(
                simplex,
                iteration,
                GjkFailureReason::NonFiniteArithmetic,
            );
        }
        if closest_squared < 1e-20 {
            return GjkResult {
                intersecting: true,
                termination: GjkTermination::Intersecting,
                simplex,
                iterations: iteration,
            };
        }

        direction = -closest;
        let new_point = minkowski_support(shape_a, shape_b, &direction);
        if !vector_is_finite(&new_point) {
            return indeterminate(simplex, iteration, GjkFailureReason::NonFiniteSupport);
        }
        let improvement = (new_point - closest).dot(&direction);
        if !improvement.is_finite() {
            return indeterminate(
                simplex,
                iteration,
                GjkFailureReason::NonFiniteArithmetic,
            );
        }
        let progress_tolerance = 64.0 * f64::EPSILON * closest_squared;
        if improvement <= progress_tolerance {
            return GjkResult {
                intersecting: false,
                termination: GjkTermination::Separated,
                simplex,
                iterations: iteration,
            };
        }

        for point in &simplex {
            let delta_squared = (*point - new_point).norm_squared();
            if !delta_squared.is_finite() {
                return indeterminate(
                    simplex,
                    iteration,
                    GjkFailureReason::NonFiniteArithmetic,
                );
            }
            if delta_squared < 1e-24 {
                return indeterminate(simplex, iteration, GjkFailureReason::DuplicateSupport);
            }
        }

        // A full 4D simplex contains five points. Reaching this guard means
        // reduction failed to leave capacity for the next support point.
        if simplex.len() >= MAX_SIMPLEX {
            return indeterminate(
                simplex,
                iteration,
                GjkFailureReason::SimplexCapacityExceeded,
            );
        }
        simplex.push(new_point);

        let Some((reduced, next_closest)) = closest_point_on_simplex(&simplex) else {
            return indeterminate(simplex, iteration, GjkFailureReason::DegenerateSimplex);
        };
        if !vector_is_finite(&next_closest) || !simplex_is_finite(&reduced) {
            return indeterminate(
                reduced,
                iteration,
                GjkFailureReason::NonFiniteArithmetic,
            );
        }
        simplex = reduced;
        closest = next_closest;

        if closest.norm_squared() < 1e-20 {
            return GjkResult {
                intersecting: true,
                termination: GjkTermination::Intersecting,
                simplex,
                iterations: iteration,
            };
        }
    }

    indeterminate(
        simplex,
        MAX_ITERATIONS,
        GjkFailureReason::IterationLimit,
    )
}

/// Return the closest point to the origin on a simplex and the active face.
///
/// Every closest point on a convex simplex belongs to the relative interior of
/// one of its faces. Enumerating at most 2^5 - 1 subsets lets us solve each
/// candidate face with a small Gram system and retain only non-negative
/// barycentric coordinates. Degenerate faces are skipped; their lower-dimensional
/// subsets are considered independently.
fn closest_point_on_simplex<const D: usize>(
    simplex: &Simplex<D>,
) -> Option<(Simplex<D>, SVector<f64, D>)> {
    let count = simplex.len();
    if count == 0 || count > MAX_SIMPLEX {
        return None;
    }

    let mut best: Option<(usize, SVector<f64, D>, f64)> = None;
    for mask in 1usize..(1usize << count) {
        let mut indices = [0usize; MAX_SIMPLEX];
        let mut face_count = 0usize;
        for index in 0..count {
            if mask & (1usize << index) != 0 {
                indices[face_count] = index;
                face_count += 1;
            }
        }

        let origin = simplex[indices[0]];
        let edge_count = face_count - 1;
        let mut weights = [0.0f64; MAX_SIMPLEX];
        weights[0] = 1.0;
        let candidate = if edge_count == 0 {
            origin
        } else {
            let mut edges: ArrayVec<SVector<f64, D>, MAX_SIMPLEX> = ArrayVec::new();
            for i in 1..face_count {
                edges.push(simplex[indices[i]] - origin);
            }

            let mut gram = [[0.0f64; MAX_SIMPLEX]; MAX_SIMPLEX];
            let mut rhs = [0.0f64; MAX_SIMPLEX];
            for i in 0..edge_count {
                rhs[i] = -edges[i].dot(&origin);
                for j in 0..edge_count {
                    gram[i][j] = edges[i].dot(&edges[j]);
                }
            }

            let Some(solution) = solve_small_system(gram, rhs, edge_count) else {
                continue;
            };
            let mut sum = 0.0;
            for i in 0..edge_count {
                weights[i + 1] = solution[i];
                sum += solution[i];
            }
            weights[0] = 1.0 - sum;

            if weights[..face_count]
                .iter()
                .any(|weight| !weight.is_finite() || *weight < -1e-9)
            {
                continue;
            }

            // Project tiny negative round-off back onto the simplex face.
            let weight_sum: f64 = weights[..face_count]
                .iter()
                .map(|weight| (*weight).max(0.0))
                .sum();
            if weight_sum <= f64::MIN_POSITIVE {
                continue;
            }
            let mut point = SVector::<f64, D>::zeros();
            for i in 0..face_count {
                point += simplex[indices[i]] * (weights[i].max(0.0) / weight_sum);
            }
            point
        };

        let distance_squared = candidate.norm_squared();
        let replace = if !distance_squared.is_finite() {
            false
        } else {
            match best.as_ref() {
                // If the origin is on the simplex, prefer the largest available
                // containing face. EPA benefits from a fuller seed simplex.
                Some((best_mask, _, best_distance))
                    if distance_squared < 1e-20 && *best_distance < 1e-20 =>
                {
                    mask.count_ones() > best_mask.count_ones()
                }
                Some((_, _, best_distance)) => distance_squared < *best_distance,
                None => true,
            }
        };
        if replace {
            best = Some((mask, candidate, distance_squared));
        }
    }

    let (mask, point, _) = best?;
    let mut reduced = Simplex::<D>::new();
    for index in 0..count {
        if mask & (1usize << index) != 0 {
            reduced.push(simplex[index]);
        }
    }
    Some((reduced, point))
}

/// Solve a dense system of at most four equations with partial pivoting.
fn solve_small_system(
    mut matrix: [[f64; MAX_SIMPLEX]; MAX_SIMPLEX],
    mut rhs: [f64; MAX_SIMPLEX],
    size: usize,
) -> Option<[f64; MAX_SIMPLEX]> {
    if size == 0 || size >= MAX_SIMPLEX {
        return None;
    }
    let scale = matrix[..size]
        .iter()
        .flat_map(|row| row[..size].iter())
        .fold(0.0_f64, |largest, value| largest.max(value.abs()));
    if !scale.is_finite() || scale == 0.0 {
        return None;
    }

    for column in 0..size {
        let mut pivot_row = column;
        for row in (column + 1)..size {
            if matrix[row][column].abs() > matrix[pivot_row][column].abs() {
                pivot_row = row;
            }
        }
        if matrix[pivot_row][column].abs() <= scale * 64.0 * f64::EPSILON {
            return None;
        }
        matrix.swap(column, pivot_row);
        rhs.swap(column, pivot_row);

        let diagonal = matrix[column][column];
        for entry in 0..size {
            matrix[column][entry] /= diagonal;
        }
        rhs[column] /= diagonal;

        for row in 0..size {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            for entry in 0..size {
                matrix[row][entry] -= factor * matrix[column][entry];
            }
            rhs[row] -= factor * rhs[column];
        }
    }

    if rhs[..size].iter().all(|value| value.is_finite()) {
        Some(rhs)
    } else {
        None
    }
}

/// Support point on the Minkowski difference A - B.
#[inline]
fn minkowski_support<const D: usize>(
    shape_a: &dyn WorldSupportMap<D>,
    shape_b: &dyn WorldSupportMap<D>,
    direction: &SVector<f64, D>,
) -> SVector<f64, D> {
    shape_a.support_world(direction) - shape_b.support_world(&(-*direction))
}

/// Process the simplex and update the search direction.
/// Returns true if the origin is enclosed by the simplex.
///
/// This is the dimension-agnostic simplex handler. For D dimensions,
/// we need at most D+1 points to enclose the origin.
fn do_simplex<const D: usize>(simplex: &mut Simplex<D>, direction: &mut SVector<f64, D>) -> bool {
    match simplex.len() {
        2 => do_simplex_line(simplex, direction),
        3 => do_simplex_triangle(simplex, direction),
        4 if D >= 3 => do_simplex_tetrahedron(simplex, direction),
        n if n == D + 1 => {
            // Full simplex for this dimension — origin is enclosed
            true
        }
        _ => {
            // For D > 3, we handle intermediate simplices by checking
            // if the origin is past the newest point in the search direction.
            // This is a conservative approximation that works for all dimensions.
            do_simplex_general(simplex, direction)
        }
    }
}

/// Line simplex: 2 points [B, A] where A is the newest.
fn do_simplex_line<const D: usize>(
    simplex: &mut Simplex<D>,
    direction: &mut SVector<f64, D>,
) -> bool {
    let a = simplex[1]; // newest
    let b = simplex[0];
    let ab = b - a;
    let ao = -a; // origin - a

    if ab.dot(&ao) > 0.0 {
        // Origin is between A and B (in the Voronoi region of edge AB)
        // Direction perpendicular to AB toward origin
        *direction = triple_cross_product(&ab, &ao, &ab);
        if direction.norm_squared() < 1e-20 {
            // Origin is on the line segment
            return true;
        }
    } else {
        // Origin is past A — discard B
        simplex.clear();
        simplex.push(a);
        *direction = ao;
    }
    false
}

/// Triangle simplex: 3 points [C, B, A] where A is the newest.
fn do_simplex_triangle<const D: usize>(
    simplex: &mut Simplex<D>,
    direction: &mut SVector<f64, D>,
) -> bool {
    let a = simplex[2]; // newest
    let b = simplex[1];
    let c = simplex[0];
    let ab = b - a;
    let ac = c - a;
    let ao = -a;

    // Edge AB perpendicular: perpendicular to AB in the triangle plane, pointing away from C.
    // Computed as: component of AC perpendicular to AB, then negated.
    // proj_AB(AC) = AB * (AC·AB)/(AB·AB)
    // perp_part = AC - proj_AB(AC) points toward C
    // negate → points away from C
    let ab_sq = ab.dot(&ab);
    let ab_perp = if ab_sq > 1e-20 {
        let proj = ab * (ac.dot(&ab) / ab_sq);
        -(ac - proj)
    } else {
        SVector::zeros()
    };

    if ab_perp.dot(&ao) > 0.0 {
        simplex.clear();
        simplex.push(b);
        simplex.push(a);
        return do_simplex_line(simplex, direction);
    }

    // Edge AC perpendicular: perpendicular to AC, pointing away from B
    let ac_sq = ac.dot(&ac);
    let ac_perp = if ac_sq > 1e-20 {
        let proj = ac * (ab.dot(&ac) / ac_sq);
        -(ab - proj)
    } else {
        SVector::zeros()
    };

    if ac_perp.dot(&ao) > 0.0 {
        simplex.clear();
        simplex.push(c);
        simplex.push(a);
        return do_simplex_line(simplex, direction);
    }

    if D == 2 {
        return true; // In 2D, triangle encloses origin
    }

    // Origin projects inside the triangle edges.
    // Need to determine which side of the triangle plane the origin is on.
    // Compute face normal via Gram-Schmidt orthogonal complement.
    let e1e1 = ab.norm_squared();
    let e2e2 = ac.norm_squared();
    let e1e2 = ab.dot(&ac);
    let det = e1e1 * e2e2 - e1e2 * e1e2;

    if det.abs() < 1e-20 {
        simplex.clear();
        simplex.push(b);
        simplex.push(a);
        return do_simplex_line(simplex, direction);
    }

    // Project AO onto the triangle plane, then subtract to get normal component
    let ao_e1 = ao.dot(&ab);
    let ao_e2 = ao.dot(&ac);
    let proj =
        ab * ((ao_e1 * e2e2 - ao_e2 * e1e2) / det) + ac * ((ao_e2 * e1e1 - ao_e1 * e1e2) / det);
    let face_normal = ao - proj;

    if face_normal.norm_squared() < 1e-20 {
        return true; // Origin is on the triangle plane
    }

    *direction = face_normal;
    false
}

/// Tetrahedron simplex (3D): 4 points [D, C, B, A] where A is the newest.
///
/// We check each of the 3 faces adjacent to A. If the origin is outside any face,
/// we reduce to that face (triangle). If inside all faces, origin is enclosed.
fn do_simplex_tetrahedron<const D: usize>(
    simplex: &mut Simplex<D>,
    direction: &mut SVector<f64, D>,
) -> bool {
    let a = simplex[3]; // newest
    let b = simplex[2];
    let c = simplex[1];
    let d = simplex[0];
    let ao = -a;
    let ab = b - a;
    let ac = c - a;
    let ad = d - a;

    // Face ABC normal: perpendicular to AB and AC, pointing away from D
    let abc = face_normal(&ab, &ac, &ad);

    // Face ACD normal: perpendicular to AC and AD, pointing away from B
    let acd = face_normal(&ac, &ad, &ab);

    // Face ADB normal: perpendicular to AD and AB, pointing away from C
    let adb = face_normal(&ad, &ab, &ac);

    if abc.dot(&ao) > 0.0 {
        simplex.clear();
        simplex.push(c);
        simplex.push(b);
        simplex.push(a);
        return do_simplex_triangle(simplex, direction);
    }

    if acd.dot(&ao) > 0.0 {
        simplex.clear();
        simplex.push(d);
        simplex.push(c);
        simplex.push(a);
        return do_simplex_triangle(simplex, direction);
    }

    if adb.dot(&ao) > 0.0 {
        simplex.clear();
        simplex.push(b);
        simplex.push(d);
        simplex.push(a);
        return do_simplex_triangle(simplex, direction);
    }

    true
}

/// Compute face normal for the face defined by edge1 and edge2,
/// oriented to point away from the opposite vertex direction.
fn face_normal<const D: usize>(
    edge1: &SVector<f64, D>,
    edge2: &SVector<f64, D>,
    opposite: &SVector<f64, D>,
) -> SVector<f64, D> {
    // The normal to the plane of edge1 and edge2:
    // n = edge1 × edge2 = triple_cross(edge1, edge2, edge1)... no.
    // For ND: compute the component of `opposite` orthogonal to both edges,
    // then negate it. But simpler: use Gram-Schmidt.
    //
    // Orthogonal complement of {edge1, edge2} applied to any test vector:
    // Project `opposite` onto span(edge1, edge2), subtract → normal direction.
    // Then negate so it points away from opposite.

    let e1_norm = edge1.norm_squared();
    let e2_norm = edge2.norm_squared();
    let e1_e2 = edge1.dot(edge2);
    let denom = e1_norm * e2_norm - e1_e2 * e1_e2;

    if denom.abs() < 1e-20 {
        return SVector::zeros(); // Degenerate
    }

    let opp_e1 = opposite.dot(edge1);
    let opp_e2 = opposite.dot(edge2);

    // Projection of opposite onto the plane
    let proj = edge1 * ((opp_e1 * e2_norm - opp_e2 * e1_e2) / denom)
        + edge2 * ((opp_e2 * e1_norm - opp_e1 * e1_e2) / denom);

    // Normal component (perpendicular to face, in direction of opposite vertex)
    let normal_toward_opp = opposite - proj;

    // We want normal pointing AWAY from opposite
    -normal_toward_opp
}

/// General simplex handler for D > 3.
/// Checks faces of the simplex to see if origin is outside any.
fn do_simplex_general<const D: usize>(
    simplex: &mut Simplex<D>,
    direction: &mut SVector<f64, D>,
) -> bool {
    let n = simplex.len();
    let a = simplex[n - 1]; // newest point
    let ao = -a;

    // Check each face (formed by removing one non-newest vertex)
    for i in 0..(n - 1) {
        // Build face normal by taking the vectors from A to each other vertex
        // except vertex i, then computing the perpendicular
        let mut face_vecs: ArrayVec<SVector<f64, D>, MAX_SIMPLEX> = ArrayVec::new();
        for j in 0..(n - 1) {
            if j != i {
                face_vecs.push(simplex[j] - a);
            }
        }

        // Gram-Schmidt normal: project toward-removed onto face span, take perpendicular.
        // Exact for any D (Fix 9: replaces approximate triple cross product).
        if !face_vecs.is_empty() {
            let to_removed = simplex[i] - a;
            let mut normal = to_removed;
            for edge in &face_vecs {
                let edge_nsq = edge.norm_squared();
                if edge_nsq > 1e-20 {
                    let proj = edge * (normal.dot(edge) / edge_nsq);
                    normal -= proj;
                }
            }
            // Normal now perpendicular to face, pointing toward removed vertex.
            // Negate to point OUTWARD (away from removed).
            let outward = -normal;

            if outward.dot(&ao) > 0.0 {
                // Origin is outside this face — remove vertex i and continue
                simplex.remove(i);
                *direction = outward;
                return false;
            }
        }
    }

    // Origin is inside all faces — enclosed
    true
}

/// Triple cross product: (a × b) × c
/// Generalized to ND using the BAC-CAB rule:
/// (a × b) × c = b(a·c) - a(b·c)
#[inline]
fn triple_cross_product<const D: usize>(
    a: &SVector<f64, D>,
    b: &SVector<f64, D>,
    c: &SVector<f64, D>,
) -> SVector<f64, D> {
    b * a.dot(c) - a * b.dot(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::{ConvexHull, Point, Sphere};

    #[test]
    fn spheres_overlapping_3d() {
        let a = Sphere::<3>::new(Point::new([0.0, 0.0, 0.0]), 1.0);
        let b = Sphere::<3>::new(Point::origin(), 1.0);
        let pa = SVector::from([0.0, 0.0, 0.0]);
        let pb = SVector::from([1.0, 0.0, 0.0]);
        let result = intersects(&a, &pa, &b, &pb);
        assert!(result.intersecting);
    }

    #[test]
    fn spheres_separated_3d() {
        let a = Sphere::<3>::new(Point::origin(), 1.0);
        let b = Sphere::<3>::new(Point::origin(), 1.0);
        let pa = SVector::from([0.0, 0.0, 0.0]);
        let pb = SVector::from([3.0, 0.0, 0.0]);
        let result = intersects(&a, &pa, &b, &pb);
        assert!(!result.intersecting);
    }

    #[test]
    fn spheres_touching_3d() {
        let a = Sphere::<3>::new(Point::origin(), 1.0);
        let b = Sphere::<3>::new(Point::origin(), 1.0);
        let pa = SVector::from([0.0, 0.0, 0.0]);
        let pb = SVector::from([2.0, 0.0, 0.0]);
        let result = intersects(&a, &pa, &b, &pb);
        // At exact touching, GJK may report either way — both are acceptable
        // The important thing is it doesn't crash or infinite loop
        assert!(result.iterations < MAX_ITERATIONS);
    }

    #[test]
    fn boxes_overlapping_2d() {
        let a = ConvexHull::<2>::unit_cube();
        let b = ConvexHull::<2>::unit_cube();
        let pa = SVector::from([0.0, 0.0]);
        let pb = SVector::from([1.0, 0.0]);
        let result = intersects(&a, &pa, &b, &pb);
        assert!(result.intersecting);
    }

    #[test]
    fn boxes_separated_2d() {
        let a = ConvexHull::<2>::unit_cube();
        let b = ConvexHull::<2>::unit_cube();
        let pa = SVector::from([0.0, 0.0]);
        let pb = SVector::from([5.0, 0.0]);
        let result = intersects(&a, &pa, &b, &pb);
        assert!(!result.intersecting);
    }

    #[test]
    fn boxes_overlapping_4d() {
        let a = ConvexHull::<4>::unit_cube(); // tesseract
        let b = ConvexHull::<4>::unit_cube();
        let pa = SVector::from([0.0, 0.0, 0.0, 0.0]);
        let pb = SVector::from([1.0, 0.0, 0.0, 0.0]);
        let result = intersects(&a, &pa, &b, &pb);
        assert!(result.intersecting);
    }

    #[test]
    fn boxes_separated_4d() {
        let a = ConvexHull::<4>::unit_cube();
        let b = ConvexHull::<4>::unit_cube();
        let pa = SVector::from([0.0, 0.0, 0.0, 0.0]);
        let pb = SVector::from([5.0, 0.0, 0.0, 0.0]);
        let result = intersects(&a, &pa, &b, &pb);
        assert!(!result.intersecting);
    }

    #[test]
    fn sphere_vs_box_3d() {
        // Use Sphere with origin center — pos_a provides world offset
        let sphere = Sphere::<3>::new(Point::origin(), 1.0);
        let cube = ConvexHull::<3>::unit_cube();
        let ps = SVector::from([0.0, 0.0, 0.0]);
        let pc = SVector::from([1.5, 0.0, 0.0]);
        let result = intersects(&sphere, &ps, &cube, &pc);
        // sphere(r=1) at 0 + cube(half=1) at 1.5: overlap = 1+1 - 1.5 = 0.5
        assert!(result.intersecting, "sphere+box should overlap at dist 1.5");
    }

    #[test]
    fn sphere_vs_box_separated_3d() {
        let sphere = Sphere::<3>::new(Point::origin(), 1.0);
        let cube = ConvexHull::<3>::unit_cube();
        let ps = SVector::from([0.0, 0.0, 0.0]);
        let pc = SVector::from([3.0, 0.0, 0.0]);
        let result = intersects(&sphere, &ps, &cube, &pc);
        // sphere(r=1) at 0 + cube(half=1) at 3: gap = 3 - 1 - 1 = 1
        assert!(
            !result.intersecting,
            "sphere+box should be separated at dist 3"
        );
    }

    #[test]
    fn identical_position_intersects() {
        let a = Sphere::<3>::unit();
        let pos = SVector::from([5.0, 5.0, 5.0]);
        let result = intersects(&a, &pos, &a, &pos);
        assert!(result.intersecting);
    }

    #[test]
    fn convergence_sphere_box() {
        let a = Sphere::<3>::unit();
        let b = ConvexHull::<3>::unit_cube();
        let pa = SVector::zeros();
        let pb = SVector::from([0.5, 0.3, 0.1]);
        let result = intersects(&a, &pa, &b, &pb);
        // Sphere r=1 at origin + cube half=1 at (0.5,0.3,0.1): clearly overlapping
        assert!(
            result.intersecting,
            "sphere+cube should intersect, iters={}",
            result.iterations
        );
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use symtropy_math::Sphere;

    proptest! {
        /// Two spheres at the same position always intersect.
        #[test]
        fn coincident_spheres_always_intersect(
            x in -100.0f64..100.0,
            y in -100.0f64..100.0,
            z in -100.0f64..100.0,
            r in 0.1f64..10.0,
        ) {
            let a = Sphere::<3>::new(Point::origin(), r);
            let b = Sphere::<3>::new(Point::origin(), r);
            let pos = SVector::from([x, y, z]);
            let result = intersects(&a, &pos, &b, &pos);
            prop_assert!(result.intersecting, "coincident spheres must intersect");
        }

        /// Two spheres far apart never intersect.
        #[test]
        fn distant_spheres_never_intersect(
            x in 0.0f64..10.0,
            y in 0.0f64..10.0,
            z in 0.0f64..10.0,
            r in 0.1f64..1.0,
        ) {
            let a = Sphere::<3>::new(Point::origin(), r);
            let b = Sphere::<3>::new(Point::origin(), r);
            let pa = SVector::from([0.0, 0.0, 0.0]);
            // Distance > 2*r + 10 (always separated)
            let pb = SVector::from([x + 2.0 * r + 10.0, y + 2.0 * r + 10.0, z + 2.0 * r + 10.0]);
            let result = intersects(&a, &pa, &b, &pb);
            prop_assert!(!result.intersecting, "distant spheres must not intersect");
        }

        /// GJK is symmetric: intersects(A,B) == intersects(B,A).
        #[test]
        fn gjk_is_symmetric(
            ax in -10.0f64..10.0, ay in -10.0f64..10.0,
            bx in -10.0f64..10.0, by in -10.0f64..10.0,
        ) {
            let a = Sphere::<2>::unit();
            let b = Sphere::<2>::unit();
            let pa = SVector::from([ax, ay]);
            let pb = SVector::from([bx, by]);
            let ab = intersects(&a, &pa, &b, &pb);
            let ba = intersects(&b, &pb, &a, &pa);
            prop_assert!(ab.intersecting == ba.intersecting, "GJK must be symmetric: ab={} ba={}", ab.intersecting, ba.intersecting);
        }

        /// GJK never exceeds MAX_ITERATIONS.
        #[test]
        fn gjk_always_terminates(
            ax in -50.0f64..50.0, ay in -50.0f64..50.0, az in -50.0f64..50.0,
            bx in -50.0f64..50.0, by in -50.0f64..50.0, bz in -50.0f64..50.0,
            r in 0.01f64..5.0,
        ) {
            let a = Sphere::<3>::new(Point::origin(), r);
            let b = Sphere::<3>::new(Point::origin(), r);
            let pa = SVector::from([ax, ay, az]);
            let pb = SVector::from([bx, by, bz]);
            let result = intersects(&a, &pa, &b, &pb);
            prop_assert!(result.iterations <= MAX_ITERATIONS, "GJK must terminate");
        }

        /// Sphere intersection agrees with analytical distance check.
        #[test]
        fn gjk_matches_analytical_sphere_check(
            dist in 0.0f64..5.0,
            r in 0.1f64..2.0,
        ) {
            let a = Sphere::<3>::new(Point::origin(), r);
            let b = Sphere::<3>::new(Point::origin(), r);
            let pa = SVector::from([0.0, 0.0, 0.0]);
            let pb = SVector::from([dist, 0.0, 0.0]);
            let result = intersects(&a, &pa, &b, &pb);

            let analytical = dist < 2.0 * r;
            // Allow tolerance at the boundary (dist ≈ 2r)
            if (dist - 2.0 * r).abs() > 0.1 {
                prop_assert!(
                    result.intersecting == analytical,
                    "GJK disagrees with analytical at dist={dist}, r={r}: gjk={}, analytical={analytical}",
                    result.intersecting,
                );
            }
        }
    }

    #[test]
    fn rotated_thin_box_uses_full_transform() {
        use std::f64::consts::FRAC_PI_2;
        use symtropy_math::{Bivector, HyperBox, Rotor, Transform};

        // Local long axis is X. After a 90-degree XY rotation it is vertical,
        // so a sphere at y=1.5 must overlap the box. A position-only query
        // would incorrectly see the box as only 0.25 units tall.
        let box_shape = HyperBox::<3>::new([2.0, 0.25, 0.25]);
        let sphere = Sphere::<3>::unit();
        let box_transform = Transform {
            translation: Point::origin(),
            rotation: Rotor::from_plane_angle(&Bivector::unit_plane(0, 1), FRAC_PI_2),
        };
        let sphere_transform = Transform::from_translation(Point::new([0.0, 1.5, 0.0]));

        let result = intersects_transformed(&box_shape, &box_transform, &sphere, &sphere_transform);
        assert!(result.intersecting);
    }

    #[test]
    fn rotated_thin_box_separation_is_not_axis_aligned_false_positive() {
        use std::f64::consts::FRAC_PI_2;
        use symtropy_math::{Bivector, HyperBox, Rotor, Transform};

        let box_shape = HyperBox::<3>::new([2.0, 0.25, 0.25]);
        let sphere = Sphere::<3>::new(Point::origin(), 0.2);
        let box_transform = Transform {
            translation: Point::origin(),
            rotation: Rotor::from_plane_angle(&Bivector::unit_plane(0, 1), FRAC_PI_2),
        };
        let sphere_transform = Transform::from_translation(Point::new([1.0, 0.0, 0.0]));

        let result = intersects_transformed(&box_shape, &box_transform, &sphere, &sphere_transform);
        assert!(!result.intersecting);
    }
}

#[cfg(test)]
mod termination_status_tests {
    use super::*;
    use crate::support_map::WorldSupportMap;
    use symtropy_math::Sphere;

    struct NonFiniteSupportMap;

    impl WorldSupportMap<3> for NonFiniteSupportMap {
        fn support_world(&self, _direction: &SVector<f64, 3>) -> SVector<f64, 3> {
            SVector::from([f64::NAN, 0.0, 0.0])
        }

        fn center_world(&self) -> SVector<f64, 3> {
            SVector::zeros()
        }

        fn bounding_sphere_world(&self) -> (SVector<f64, 3>, f64) {
            (SVector::zeros(), 1.0)
        }
    }

    #[test]
    fn converged_separation_has_explicit_separated_status() {
        let a = Sphere::<3>::unit();
        let b = Sphere::<3>::unit();
        let result = intersects(
            &a,
            &SVector::zeros(),
            &b,
            &SVector::from([4.0, 0.0, 0.0]),
        );

        assert!(!result.intersecting);
        assert_eq!(result.termination, GjkTermination::Separated);
    }

    #[test]
    fn overlapping_pair_has_explicit_intersecting_status() {
        let a = Sphere::<3>::unit();
        let b = Sphere::<3>::unit();
        let result = intersects(
            &a,
            &SVector::zeros(),
            &b,
            &SVector::from([1.0, 0.0, 0.0]),
        );

        assert!(result.intersecting);
        assert_eq!(result.termination, GjkTermination::Intersecting);
    }

    #[test]
    fn non_finite_support_is_indeterminate_not_separated() {
        let invalid = NonFiniteSupportMap;
        let ordinary = Sphere::<3>::unit();
        let result = intersects_support_maps(&invalid, &ordinary);

        assert!(!result.intersecting);
        assert_eq!(
            result.termination,
            GjkTermination::Indeterminate(GjkFailureReason::NonFiniteSupport)
        );
    }

    #[test]
    fn non_finite_query_center_is_indeterminate() {
        struct NonFiniteCenterMap;
        impl WorldSupportMap<3> for NonFiniteCenterMap {
            fn support_world(&self, direction: &SVector<f64, 3>) -> SVector<f64, 3> {
                *direction
            }

            fn center_world(&self) -> SVector<f64, 3> {
                SVector::from([0.0, f64::INFINITY, 0.0])
            }

            fn bounding_sphere_world(&self) -> (SVector<f64, 3>, f64) {
                (self.center_world(), 1.0)
            }
        }

        let invalid = NonFiniteCenterMap;
        let ordinary = Sphere::<3>::unit();
        let result = intersects_support_maps(&invalid, &ordinary);

        assert_eq!(
            result.termination,
            GjkTermination::Indeterminate(GjkFailureReason::NonFiniteInput)
        );
    }
}
