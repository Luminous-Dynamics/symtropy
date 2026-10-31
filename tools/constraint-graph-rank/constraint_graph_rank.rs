// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Rank-aware geometric constraint-graph diagnostics for three point masses
// in R^3 with two-edge open chains or three-edge closed triangles.
//
// This module is independent of the time integrator. It deliberately
// separates geometric constraint rank from the timestep-dependent
// nonlinear multiplier Jacobian.

use std::f64;

pub type Vec3 = [f64; 3];
const BODY_COUNT: usize = 3;
const MAX_EDGES: usize = 3;
const MAX_JACOBI_SWEEPS: usize = 48;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GraphError {
    InvalidBodyMass { body_index: usize },
    NonFiniteBodyState { body_index: usize },
    DuplicateBodyId { id: u64 },
    UnsupportedEdgeCount,
    DuplicateEdgeId { id: u64 },
    SelfEdge { edge_id: u64 },
    DuplicateUndirectedEdge { edge_id: u64, other_edge_id: u64 },
    MissingEndpoint { edge_id: u64, body_id: u64 },
    InvalidEdgeLength { edge_id: u64 },
    InfeasibleTriangleLengths,
    PositionDoesNotMatchEdge { edge_id: u64 },
    NonFiniteMatrix,
    InvalidRankPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeometryStatus {
    FullRank,
    IllConditioned,
    RankDeficient,
    InfeasibleGeometry,
    NonFinite,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RankPolicy {
    /// Below this reciprocal condition indicator, preserve a warning status.
    pub ill_conditioned_rcond: f64,
    /// At or below this reciprocal condition indicator, do not treat the
    /// constraint multipliers as uniquely determined.
    pub rank_deficient_rcond: f64,
    /// Relative tolerance used for geometric consistency and triangle tests.
    pub geometry_tolerance: f64,
}

impl Default for RankPolicy {
    fn default() -> Self {
        Self {
            ill_conditioned_rcond: 1.0e-6,
            rank_deficient_rcond: 1.0e-12,
            geometry_tolerance: 1.0e-10,
        }
    }
}

impl RankPolicy {
    pub fn validate(self) -> Result<Self, GraphError> {
        if !self.ill_conditioned_rcond.is_finite()
            || !self.rank_deficient_rcond.is_finite()
            || !self.geometry_tolerance.is_finite()
            || self.ill_conditioned_rcond <= 0.0
            || self.rank_deficient_rcond <= 0.0
            || self.geometry_tolerance <= 0.0
            || self.rank_deficient_rcond >= self.ill_conditioned_rcond
            || self.ill_conditioned_rcond >= 1.0
        {
            return Err(GraphError::InvalidRankPolicy);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphBody {
    pub id: u64,
    pub mass: f64,
    pub position: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistanceEdge {
    pub id: u64,
    pub from_body_id: u64,
    pub to_body_id: u64,
    pub length: f64,
}

#[derive(Clone, Debug)]
pub struct ConstraintGraph3 {
    revision: u64,
    bodies: [GraphBody; BODY_COUNT],
    edges: Vec<DistanceEdge>,
    policy: RankPolicy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeometryRankReceipt {
    pub graph_revision: u64,
    pub body_ids: [u64; BODY_COUNT],
    /// Stable edge IDs, in deterministic ascending-ID order.
    pub edge_ids: Vec<u64>,
    pub numerical_rank: usize,
    pub edge_count: usize,
    pub smallest_eigenvalue: f64,
    pub largest_eigenvalue: f64,
    /// λ_min(C) / λ_max(C), with C = G M^{-1} G^T.
    pub reciprocal_condition_indicator: f64,
    pub status: GeometryStatus,
    /// Signed position residual for each stable edge ID:
    /// ||q_j-q_i|| - L_e.
    pub signed_length_residuals: Vec<f64>,
    /// Triangle area for a closed 3-edge graph; zero for a two-edge chain.
    pub triangle_area: f64,
    /// The mass-weighted Gram matrix in stable edge-ID order, padded to 3x3.
    pub gram_matrix: [[f64; MAX_EDGES]; MAX_EDGES],
    /// Ascending eigenvalues, padded with +infinity for unused rows.
    pub eigenvalues: [f64; MAX_EDGES],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmallMatrixCondition {
    pub dimension: usize,
    pub numerical_rank: usize,
    pub smallest_singular_value: f64,
    pub largest_singular_value: f64,
    pub reciprocal_condition_indicator: f64,
    pub status: GeometryStatus,
    /// Eigenvalues of A^T A, ascending; square roots produce singular values.
    pub squared_singular_values: [f64; MAX_EDGES],
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: Vec3, factor: f64) -> Vec3 {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

fn rotate(value: Vec3, rotation_vector: Vec3) -> Vec3 {
    let angle = norm(rotation_vector);
    if angle < 1.0e-8 {
        let first = cross(rotation_vector, value);
        return add(add(value, first), scale(cross(rotation_vector, first), 0.5));
    }
    let axis = scale(rotation_vector, 1.0 / angle);
    let (sin, cos) = angle.sin_cos();
    add(
        add(scale(value, cos), scale(cross(axis, value), sin)),
        scale(axis, dot(axis, value) * (1.0 - cos)),
    )
}

/// Deterministic Jacobi eigenvalue iteration for a real symmetric matrix of
/// dimension 2 or 3. Returns eigenvalues in ascending order.
fn symmetric_eigenvalues(
    mut matrix: [[f64; MAX_EDGES]; MAX_EDGES],
    dimension: usize,
) -> Result<[f64; MAX_EDGES], GraphError> {
    if !(2..=MAX_EDGES).contains(&dimension)
        || !matrix.iter().flatten().all(|value| value.is_finite())
    {
        return Err(GraphError::NonFiniteMatrix);
    }

    for _ in 0..MAX_JACOBI_SWEEPS {
        let mut p = 0usize;
        let mut q = 1usize;
        let mut largest_off_diagonal = 0.0_f64;
        let mut diagonal_scale = 0.0_f64;
        for i in 0..dimension {
            diagonal_scale = diagonal_scale.max(matrix[i][i].abs());
            for j in (i + 1)..dimension {
                if matrix[i][j].abs() > largest_off_diagonal {
                    largest_off_diagonal = matrix[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }

        if largest_off_diagonal
            <= 8.0 * f64::EPSILON * diagonal_scale.max(f64::MIN_POSITIVE)
        {
            break;
        }

        let off_diagonal = matrix[p][q];
        if off_diagonal == 0.0 {
            continue;
        }
        let tau = (matrix[q][q] - matrix[p][p]) / (2.0 * off_diagonal);
        let sign = if tau < 0.0 { -1.0 } else { 1.0 };
        let tangent = sign / (tau.abs() + (1.0 + tau * tau).sqrt());
        let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
        let sine = tangent * cosine;

        let pp = matrix[p][p];
        let qq = matrix[q][q];
        matrix[p][p] = pp - tangent * off_diagonal;
        matrix[q][q] = qq + tangent * off_diagonal;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;

        for k in 0..dimension {
            if k == p || k == q {
                continue;
            }
            let kp = matrix[k][p];
            let kq = matrix[k][q];
            let new_kp = cosine * kp - sine * kq;
            let new_kq = sine * kp + cosine * kq;
            matrix[k][p] = new_kp;
            matrix[p][k] = new_kp;
            matrix[k][q] = new_kq;
            matrix[q][k] = new_kq;
        }
    }

    let mut values = [f64::INFINITY; MAX_EDGES];
    for i in 0..dimension {
        values[i] = matrix[i][i];
        if !values[i].is_finite() {
            return Err(GraphError::NonFiniteMatrix);
        }
    }
    for i in 0..dimension {
        for j in (i + 1)..dimension {
            if values[j] < values[i] {
                values.swap(i, j);
            }
        }
    }
    Ok(values)
}

fn assess_symmetric_psd(
    matrix: [[f64; MAX_EDGES]; MAX_EDGES],
    dimension: usize,
    policy: RankPolicy,
) -> Result<SmallMatrixCondition, GraphError> {
    let eigenvalues = symmetric_eigenvalues(matrix, dimension)?;
    let largest = eigenvalues[dimension - 1];
    if !largest.is_finite() || largest <= f64::MIN_POSITIVE {
        return Ok(SmallMatrixCondition {
            dimension,
            numerical_rank: 0,
            smallest_singular_value: 0.0,
            largest_singular_value: 0.0,
            reciprocal_condition_indicator: 0.0,
            status: GeometryStatus::RankDeficient,
            squared_singular_values: eigenvalues,
        });
    }

    let mut cleaned = eigenvalues;
    for value in cleaned.iter_mut().take(dimension) {
        if *value < 0.0 {
            if value.abs() <= largest * 128.0 * f64::EPSILON {
                *value = 0.0;
            } else {
                return Err(GraphError::NonFiniteMatrix);
            }
        }
    }

    let smallest = cleaned[0];
    let rcond = (smallest / largest).clamp(0.0, 1.0);
    let cutoff = largest * policy.rank_deficient_rcond;
    let rank = cleaned
        .iter()
        .take(dimension)
        .filter(|value| **value > cutoff)
        .count();
    let status = if rank < dimension {
        GeometryStatus::RankDeficient
    } else if rcond <= policy.ill_conditioned_rcond {
        GeometryStatus::IllConditioned
    } else {
        GeometryStatus::FullRank
    };

    Ok(SmallMatrixCondition {
        dimension,
        numerical_rank: rank,
        smallest_singular_value: smallest.sqrt(),
        largest_singular_value: largest.sqrt(),
        reciprocal_condition_indicator: rcond,
        status,
        squared_singular_values: cleaned,
    })
}

/// Assess a generally non-symmetric E×E multiplier Jacobian through the
/// singular values of J (eigenvalues of J^T J). This is distinct from the
/// geometric Gram-matrix rank of G M^{-1} G^T.
pub fn assess_multiplier_jacobian(
    jacobian: [[f64; MAX_EDGES]; MAX_EDGES],
    dimension: usize,
    policy: RankPolicy,
) -> Result<SmallMatrixCondition, GraphError> {
    let policy = policy.validate()?;
    if !(2..=MAX_EDGES).contains(&dimension) {
        return Err(GraphError::UnsupportedEdgeCount);
    }

    let mut normal = [[0.0_f64; MAX_EDGES]; MAX_EDGES];
    for row in 0..dimension {
        for column in 0..dimension {
            let mut value = 0.0;
            for k in 0..dimension {
                value += jacobian[k][row] * jacobian[k][column];
            }
            if !value.is_finite() {
                return Err(GraphError::NonFiniteMatrix);
            }
            normal[row][column] = value;
        }
    }

    // J^T J squares the condition number. Use squared cutoffs during the
    // eigenvalue/rank analysis, then report sigma_min/sigma_max for J itself.
    let squared_policy = RankPolicy {
        ill_conditioned_rcond: policy.ill_conditioned_rcond.powi(2),
        rank_deficient_rcond: policy.rank_deficient_rcond.powi(2),
        geometry_tolerance: policy.geometry_tolerance,
    };
    let mut result = assess_symmetric_psd(normal, dimension, squared_policy)?;
    result.reciprocal_condition_indicator =
        result.reciprocal_condition_indicator.sqrt();
    Ok(result)
}

impl ConstraintGraph3 {
    pub fn new(
        revision: u64,
        bodies: [GraphBody; BODY_COUNT],
        mut edges: Vec<DistanceEdge>,
        policy: RankPolicy,
    ) -> Result<Self, GraphError> {
        let policy = policy.validate()?;
        if edges.len() != 2 && edges.len() != 3 {
            return Err(GraphError::UnsupportedEdgeCount);
        }

        for (index, body) in bodies.iter().enumerate() {
            if !body.mass.is_finite() || body.mass <= 0.0 {
                return Err(GraphError::InvalidBodyMass { body_index: index });
            }
            if !body.position.iter().all(|value| value.is_finite()) {
                return Err(GraphError::NonFiniteBodyState { body_index: index });
            }
            if bodies[..index].iter().any(|prior| prior.id == body.id) {
                return Err(GraphError::DuplicateBodyId { id: body.id });
            }
        }

        edges.sort_by_key(|edge| edge.id);
        for (index, edge) in edges.iter().enumerate() {
            if edges[..index].iter().any(|prior| prior.id == edge.id) {
                return Err(GraphError::DuplicateEdgeId { id: edge.id });
            }
            if edge.from_body_id == edge.to_body_id {
                return Err(GraphError::SelfEdge { edge_id: edge.id });
            }
            if !edge.length.is_finite() || edge.length <= 0.0 {
                return Err(GraphError::InvalidEdgeLength { edge_id: edge.id });
            }
            for id in [edge.from_body_id, edge.to_body_id] {
                if !bodies.iter().any(|body| body.id == id) {
                    return Err(GraphError::MissingEndpoint {
                        edge_id: edge.id,
                        body_id: id,
                    });
                }
            }
            if let Some(prior) = edges[..index].iter().find(|prior| {
                (prior.from_body_id == edge.from_body_id
                    && prior.to_body_id == edge.to_body_id)
                    || (prior.from_body_id == edge.to_body_id
                        && prior.to_body_id == edge.from_body_id)
            }) {
                return Err(GraphError::DuplicateUndirectedEdge {
                    edge_id: edge.id,
                    other_edge_id: prior.id,
                });
            }
        }

        if edges.len() == 3 {
            let mut lengths = [edges[0].length, edges[1].length, edges[2].length];
            lengths.sort_by(f64::total_cmp);
            let largest = lengths[2];
            let other_sum = lengths[0] + lengths[1];
            let tolerance = policy.geometry_tolerance * (1.0 + largest + other_sum);
            if largest > other_sum + tolerance {
                return Err(GraphError::InfeasibleTriangleLengths);
            }
        }

        let graph = Self {
            revision,
            bodies,
            edges,
            policy,
        };
        graph.validate_positions()?;
        Ok(graph)
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn bodies(&self) -> &[GraphBody; BODY_COUNT] {
        &self.bodies
    }

    pub fn edges(&self) -> &[DistanceEdge] {
        &self.edges
    }

    fn body_index(&self, id: u64) -> Option<usize> {
        self.bodies.iter().position(|body| body.id == id)
    }

    fn validate_positions(&self) -> Result<(), GraphError> {
        for edge in &self.edges {
            let from_index = self
                .body_index(edge.from_body_id)
                .ok_or(GraphError::MissingEndpoint {
                    edge_id: edge.id,
                    body_id: edge.from_body_id,
                })?;
            let to_index = self
                .body_index(edge.to_body_id)
                .ok_or(GraphError::MissingEndpoint {
                    edge_id: edge.id,
                    body_id: edge.to_body_id,
                })?;
            let distance = norm(sub(
                self.bodies[to_index].position,
                self.bodies[from_index].position,
            ));
            let residual = distance - edge.length;
            let scale = 1.0 + distance + edge.length;
            if !residual.is_finite() {
                return Err(GraphError::NonFiniteMatrix);
            }
            if residual.abs() > self.policy.geometry_tolerance * scale {
                return Err(GraphError::PositionDoesNotMatchEdge { edge_id: edge.id });
            }
        }
        Ok(())
    }

    pub fn assess_geometry(&self) -> Result<GeometryRankReceipt, GraphError> {
        let edge_count = self.edges.len();
        let mut gram = [[0.0_f64; MAX_EDGES]; MAX_EDGES];
        let mut signed_residuals = Vec::with_capacity(edge_count);
        let mut edge_ids = Vec::with_capacity(edge_count);

        for edge in &self.edges {
            edge_ids.push(edge.id);
            let from = self.body_index(edge.from_body_id).ok_or(
                GraphError::MissingEndpoint {
                    edge_id: edge.id,
                    body_id: edge.from_body_id,
                },
            )?;
            let to = self.body_index(edge.to_body_id).ok_or(
                GraphError::MissingEndpoint {
                    edge_id: edge.id,
                    body_id: edge.to_body_id,
                },
            )?;
            let relative_position =
                sub(self.bodies[to].position, self.bodies[from].position);
            signed_residuals.push(norm(relative_position) - edge.length);
        }

        for row in 0..edge_count {
            let edge_row = self.edges[row];
            let row_from = self.body_index(edge_row.from_body_id).ok_or(
                GraphError::MissingEndpoint {
                    edge_id: edge_row.id,
                    body_id: edge_row.from_body_id,
                },
            )?;
            let row_to = self.body_index(edge_row.to_body_id).ok_or(
                GraphError::MissingEndpoint {
                    edge_id: edge_row.id,
                    body_id: edge_row.to_body_id,
                },
            )?;
            let row_vector = sub(
                self.bodies[row_to].position,
                self.bodies[row_from].position,
            );

            for column in 0..edge_count {
                let edge_column = self.edges[column];
                let column_from = self.body_index(edge_column.from_body_id).ok_or(
                    GraphError::MissingEndpoint {
                        edge_id: edge_column.id,
                        body_id: edge_column.from_body_id,
                    },
                )?;
                let column_to = self.body_index(edge_column.to_body_id).ok_or(
                    GraphError::MissingEndpoint {
                        edge_id: edge_column.id,
                        body_id: edge_column.to_body_id,
                    },
                )?;
                let column_vector = sub(
                    self.bodies[column_to].position,
                    self.bodies[column_from].position,
                );

                let mut value = 0.0;
                for body in 0..BODY_COUNT {
                    let row_gradient = if body == row_from {
                        scale(row_vector, -1.0)
                    } else if body == row_to {
                        row_vector
                    } else {
                        [0.0; 3]
                    };
                    let column_gradient = if body == column_from {
                        scale(column_vector, -1.0)
                    } else if body == column_to {
                        column_vector
                    } else {
                        [0.0; 3]
                    };
                    value += dot(row_gradient, column_gradient) / self.bodies[body].mass;
                }
                if !value.is_finite() {
                    return Err(GraphError::NonFiniteMatrix);
                }
                gram[row][column] = value;
            }
        }

        let mut triangle_area = 0.0;
        if edge_count == 3 {
            let q0 = self.bodies[0].position;
            let q1 = self.bodies[1].position;
            let q2 = self.bodies[2].position;
            triangle_area = 0.5 * norm(cross(sub(q1, q0), sub(q2, q0)));
        }

        let spectral = assess_symmetric_psd(gram, edge_count, self.policy)?;
        Ok(GeometryRankReceipt {
            graph_revision: self.revision,
            body_ids: self.bodies.map(|body| body.id),
            edge_ids,
            numerical_rank: spectral.numerical_rank,
            edge_count,
            smallest_eigenvalue: spectral.squared_singular_values[0],
            largest_eigenvalue: spectral.squared_singular_values[edge_count - 1],
            reciprocal_condition_indicator: spectral.reciprocal_condition_indicator,
            status: spectral.status,
            signed_length_residuals: signed_residuals,
            triangle_area,
            gram_matrix: gram,
            eigenvalues: spectral.squared_singular_values,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_chain(theta: f64, revision: u64) -> ConstraintGraph3 {
        let q0 = [0.0, 0.0, 0.0];
        let q1 = [1.0, 0.0, 0.0];
        let q2 = [1.0 + theta.cos(), theta.sin(), 0.0];
        let bodies = [
            GraphBody { id: 10, mass: 1.3, position: q0 },
            GraphBody { id: 20, mass: 2.1, position: q1 },
            GraphBody { id: 30, mass: 0.8, position: q2 },
        ];
        let edges = vec![
            DistanceEdge {
                id: 101,
                from_body_id: 10,
                to_body_id: 20,
                length: 1.0,
            },
            DistanceEdge {
                id: 202,
                from_body_id: 20,
                to_body_id: 30,
                length: 1.0,
            },
        ];
        ConstraintGraph3::new(revision, bodies, edges, RankPolicy::default()).unwrap()
    }

    fn closed_triangle(theta: f64, revision: u64) -> ConstraintGraph3 {
        let q0 = [0.0, 0.0, 0.0];
        let q1 = [1.0, 0.0, 0.0];
        let q2 = [1.0 + theta.cos(), theta.sin(), 0.0];
        let bodies = [
            GraphBody { id: 10, mass: 1.3, position: q0 },
            GraphBody { id: 20, mass: 2.1, position: q1 },
            GraphBody { id: 30, mass: 0.8, position: q2 },
        ];
        let edges = vec![
            DistanceEdge {
                id: 303,
                from_body_id: 30,
                to_body_id: 10,
                length: norm(q2),
            },
            DistanceEdge {
                id: 101,
                from_body_id: 10,
                to_body_id: 20,
                length: 1.0,
            },
            DistanceEdge {
                id: 202,
                from_body_id: 20,
                to_body_id: 30,
                length: 1.0,
            },
        ];
        ConstraintGraph3::new(revision, bodies, edges, RankPolicy::default()).unwrap()
    }

    #[test]
    fn open_chain_has_full_constraint_rank_even_when_links_are_collinear() {
        for theta in [0.0, 0.1, 0.5, 1.2] {
            let receipt = open_chain(theta, 7).assess_geometry().unwrap();
            assert_eq!(receipt.numerical_rank, 2);
            assert_eq!(receipt.status, GeometryStatus::FullRank);
            assert!(receipt.reciprocal_condition_indicator > 1.0e-6);
            assert_eq!(receipt.edge_ids, vec![101, 202]);
        }
    }

    #[test]
    fn regular_closed_triangle_is_full_rank() {
        let receipt = closed_triangle(0.5, 9).assess_geometry().unwrap();
        assert_eq!(receipt.edge_count, 3);
        assert_eq!(receipt.numerical_rank, 3);
        assert_eq!(receipt.status, GeometryStatus::FullRank);
        assert!(receipt.triangle_area > 0.0);
    }

    #[test]
    fn near_collinear_triangle_condition_degrades_monotonically() {
        let angles = [0.5, 0.1, 0.05, 0.01, 0.005, 0.001, 1.0e-4, 1.0e-5];
        let indicators: Vec<f64> = angles
            .iter()
            .enumerate()
            .map(|(index, angle)| {
                closed_triangle(*angle, 100 + index as u64)
                    .assess_geometry()
                    .unwrap()
                    .reciprocal_condition_indicator
            })
            .collect();
        for pair in indicators.windows(2) {
            assert!(
                pair[1] < pair[0],
                "near-collinear Gram rcond should decrease monotonically: {indicators:?}"
            );
        }
        assert!(indicators[0] > 1.0e-3);
        assert!(indicators[6] < 1.0e-6);
        assert!(indicators[6] > 1.0e-12);
        assert!(indicators[7] > 1.0e-12);
        assert_eq!(
            closed_triangle(1.0e-4, 222).assess_geometry().unwrap().status,
            GeometryStatus::IllConditioned
        );
        assert_eq!(
            closed_triangle(1.0e-6, 223).assess_geometry().unwrap().status,
            GeometryStatus::RankDeficient
        );
    }

    #[test]
    fn exactly_collinear_triangle_is_rank_two_not_a_unique_three_multiplier_solve() {
        let receipt = closed_triangle(0.0, 11).assess_geometry().unwrap();
        assert_eq!(receipt.numerical_rank, 2);
        assert_eq!(receipt.status, GeometryStatus::RankDeficient);
        assert!(receipt.reciprocal_condition_indicator <= 1.0e-12);
    }

    #[test]
    fn infeasible_triangle_lengths_are_rejected_before_geometry_solve() {
        let bodies = [
            GraphBody { id: 10, mass: 1.0, position: [0.0, 0.0, 0.0] },
            GraphBody { id: 20, mass: 1.0, position: [1.0, 0.0, 0.0] },
            GraphBody { id: 30, mass: 1.0, position: [2.0, 0.0, 0.0] },
        ];
        let edges = vec![
            DistanceEdge { id: 1, from_body_id: 10, to_body_id: 20, length: 1.0 },
            DistanceEdge { id: 2, from_body_id: 20, to_body_id: 30, length: 1.0 },
            DistanceEdge { id: 3, from_body_id: 30, to_body_id: 10, length: 2.1 },
        ];
        assert_eq!(
            ConstraintGraph3::new(1, bodies, edges, RankPolicy::default()).unwrap_err(),
            GraphError::InfeasibleTriangleLengths
        );
    }

    #[test]
    fn input_edge_permutation_preserves_stable_order_and_spectrum() {
        let base = closed_triangle(0.37, 13);
        let mut edges = base.edges().to_vec();
        edges.reverse();
        let permuted = ConstraintGraph3::new(
            14,
            *base.bodies(),
            edges,
            RankPolicy::default(),
        )
        .unwrap();
        let a = base.assess_geometry().unwrap();
        let b = permuted.assess_geometry().unwrap();
        assert_eq!(a.edge_ids, b.edge_ids);
        for index in 0..3 {
            assert!(
                (a.eigenvalues[index] - b.eigenvalues[index]).abs() < 1.0e-13
            );
        }
    }

    #[test]
    fn edge_orientation_reversal_preserves_geometric_spectrum() {
        let base = closed_triangle(0.42, 15);
        let mut edges = base.edges().to_vec();
        edges[0].from_body_id = base.edges()[0].to_body_id;
        edges[0].to_body_id = base.edges()[0].from_body_id;
        let reversed = ConstraintGraph3::new(
            16,
            *base.bodies(),
            edges,
            RankPolicy::default(),
        )
        .unwrap();
        let a = base.assess_geometry().unwrap();
        let b = reversed.assess_geometry().unwrap();
        assert_eq!(a.numerical_rank, b.numerical_rank);
        for index in 0..3 {
            assert!(
                (a.eigenvalues[index] - b.eigenvalues[index]).abs() < 2.0e-13
            );
        }
    }

    #[test]
    fn so3_rotation_preserves_gram_spectrum_and_rank() {
        let base = closed_triangle(0.23, 17);
        let baseline = base.assess_geometry().unwrap();
        let rotation = [0.731, -0.419, 0.283];
        let mut bodies = *base.bodies();
        for body in &mut bodies {
            body.position = rotate(body.position, rotation);
        }
        let rotated =
            ConstraintGraph3::new(18, bodies, base.edges().to_vec(), RankPolicy::default())
                .unwrap()
                .assess_geometry()
                .unwrap();
        assert_eq!(baseline.numerical_rank, rotated.numerical_rank);
        assert!(
            (baseline.reciprocal_condition_indicator
                - rotated.reciprocal_condition_indicator)
                .abs()
                < 2.0e-13
        );
        for index in 0..3 {
            assert!(
                (baseline.eigenvalues[index] - rotated.eigenvalues[index]).abs()
                    < 2.0e-12
            );
        }
    }

    #[test]
    fn multiplier_jacobian_condition_is_distinct_from_geometric_rank() {
        let policy = RankPolicy::default();
        let well_conditioned = assess_multiplier_jacobian(
            [[1.0, 0.1, 0.0], [-0.2, 0.9, 0.0], [0.0; 3]],
            2,
            policy,
        )
        .unwrap();
        let nearly_singular = assess_multiplier_jacobian(
            [[1.0, 1.0, 0.0], [1.0, 1.0 + 1.0e-10, 0.0], [0.0; 3]],
            2,
            policy,
        )
        .unwrap();
        assert_eq!(well_conditioned.status, GeometryStatus::FullRank);
        assert_ne!(nearly_singular.status, GeometryStatus::FullRank);
        assert!(nearly_singular.reciprocal_condition_indicator < 1.0e-6);
    }

    #[test]
    fn invalid_ids_edges_masses_and_lengths_fail_closed() {
        let body = [
            GraphBody { id: 10, mass: 1.0, position: [0.0, 0.0, 0.0] },
            GraphBody { id: 20, mass: 1.0, position: [1.0, 0.0, 0.0] },
            GraphBody { id: 30, mass: 1.0, position: [2.0, 0.0, 0.0] },
        ];
        let duplicate_edges = vec![
            DistanceEdge { id: 1, from_body_id: 10, to_body_id: 20, length: 1.0 },
            DistanceEdge { id: 1, from_body_id: 20, to_body_id: 30, length: 1.0 },
        ];
        assert_eq!(
            ConstraintGraph3::new(1, body, duplicate_edges, RankPolicy::default()).unwrap_err(),
            GraphError::DuplicateEdgeId { id: 1 }
        );

        let self_edge = vec![
            DistanceEdge { id: 1, from_body_id: 10, to_body_id: 10, length: 1.0 },
            DistanceEdge { id: 2, from_body_id: 20, to_body_id: 30, length: 1.0 },
        ];
        assert_eq!(
            ConstraintGraph3::new(1, body, self_edge, RankPolicy::default()).unwrap_err(),
            GraphError::SelfEdge { edge_id: 1 }
        );

        let invalid_mass = [
            GraphBody { mass: 0.0, ..body[0] },
            body[1],
            body[2],
        ];
        assert_eq!(
            ConstraintGraph3::new(
                1,
                invalid_mass,
                vec![
                    DistanceEdge { id: 1, from_body_id: 10, to_body_id: 20, length: 1.0 },
                    DistanceEdge { id: 2, from_body_id: 20, to_body_id: 30, length: 1.0 },
                ],
                RankPolicy::default(),
            )
            .unwrap_err(),
            GraphError::InvalidBodyMass { body_index: 0 }
        );
    }
}

#[cfg(not(test))]
fn main() {
    println!("theta,status,rank,rcond,lambda_min,lambda_max,area");
    for (index, angle) in [0.5, 0.1, 0.05, 0.01, 0.005, 0.001, 1.0e-4, 1.0e-5, 0.0]
        .iter()
        .enumerate()
    {
        let q0 = [0.0, 0.0, 0.0];
        let q1 = [1.0, 0.0, 0.0];
        let q2 = [1.0 + angle.cos(), angle.sin(), 0.0];
        let bodies = [
            GraphBody { id: 10, mass: 1.3, position: q0 },
            GraphBody { id: 20, mass: 2.1, position: q1 },
            GraphBody { id: 30, mass: 0.8, position: q2 },
        ];
        let edges = vec![
            DistanceEdge { id: 101, from_body_id: 10, to_body_id: 20, length: 1.0 },
            DistanceEdge { id: 202, from_body_id: 20, to_body_id: 30, length: 1.0 },
            DistanceEdge { id: 303, from_body_id: 30, to_body_id: 10, length: norm(q2) },
        ];
        let graph = ConstraintGraph3::new(
            500 + index as u64,
            bodies,
            edges,
            RankPolicy::default(),
        )
        .expect("rank-stress triangle must be geometrically feasible");
        let receipt = graph.assess_geometry().expect("Gram matrix must be finite");
        println!(
            "{:.17e},{:?},{},{:.17e},{:.17e},{:.17e},{:.17e}",
            angle,
            receipt.status,
            receipt.numerical_rank,
            receipt.reciprocal_condition_indicator,
            receipt.smallest_eigenvalue,
            receipt.largest_eigenvalue,
            receipt.triangle_area
        );
    }
}
