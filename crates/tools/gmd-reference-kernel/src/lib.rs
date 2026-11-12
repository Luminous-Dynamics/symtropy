//! Small, deterministic analytical reference kernel for geomagnetically
//! induced voltage and quasi-DC resistive-network current.
//!
//! This crate intentionally has no external dependencies and no renderer,
//! network client, control interface, or transformer-damage model.
//!
//! Coordinates are local horizontal East/North Cartesian metres. Electric
//! field is in volts per metre, resistance in ohms, voltage in volts, current
//! in amperes. The branch convention is documented on Branch.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

/// One locally constant horizontal electric-field segment along an oriented
/// path. delta_east_north_m points from the segment's start to its end.
///
/// For a sufficiently small segment,
/// delta_v = E_east * delta_east + E_north * delta_north.
/// Curved paths or spatially varying fields should be split into segments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldPathSegment {
    pub electric_field_v_per_m: [f64; 2],
    pub delta_east_north_m: [f64; 2],
}

/// Compute the induced voltage along a piecewise-constant local E-field path.
///
/// The path orientation is the order supplied by the caller. Reversing every
/// displacement reverses the voltage sign. This function performs no
/// geographic projection, interpolation, conductivity inversion, or unit
/// conversion: callers must supply an East/North local tangent frame in SI.
pub fn integrate_induced_voltage_v(
    segments: &[FieldPathSegment],
) -> Result<f64, GmdError> {
    if segments.is_empty() {
        return Err(GmdError::EmptyPath);
    }

    let mut voltage_v = 0.0;
    for segment in segments {
        let [east_v_per_m, north_v_per_m] = segment.electric_field_v_per_m;
        let [east_m, north_m] = segment.delta_east_north_m;
        if !east_v_per_m.is_finite()
            || !north_v_per_m.is_finite()
            || !east_m.is_finite()
            || !north_m.is_finite()
        {
            return Err(GmdError::NonFiniteInput);
        }

        let increment_v = east_v_per_m * east_m + north_v_per_m * north_m;
        if !increment_v.is_finite() {
            return Err(GmdError::NonFiniteInput);
        }

        voltage_v += increment_v;
        if !voltage_v.is_finite() {
            return Err(GmdError::NonFiniteInput);
        }
    }

    Ok(voltage_v)
}

/// An oriented resistive branch with an impressed geoelectric voltage.
///
/// Positive branch current is defined from from to to:
/// I_from_to = (phi_from - phi_to + induced_voltage_v) / resistance_ohm.
///
/// Thus a positive induced_voltage_v drives positive current when both
/// endpoint potentials are equal. Reversing the path requires swapping the
/// endpoints and negating the induced voltage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Branch {
    pub from: usize,
    pub to: usize,
    pub resistance_ohm: f64,
    pub induced_voltage_v: f64,
}

/// A resistive connection from one network node to the reference ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ground {
    pub node: usize,
    pub resistance_ohm: f64,
}

/// A finite, quasi-DC resistive network.
///
/// Each connected component must contain at least one explicit ground path.
/// Parallel branches and multiple ground paths are supported. Mutual
/// coupling, capacitive effects, AC power flow, transformer saturation,
/// harmonics, thermal response and protection behaviour are out of scope.
#[derive(Debug, Clone, PartialEq)]
pub struct ResistiveNetwork {
    pub node_count: usize,
    pub branches: Vec<Branch>,
    pub grounds: Vec<Ground>,
}

/// Result of the nodal solve. Branch-current order matches input branch order;
/// ground-current order matches input ground order.
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkSolution {
    pub node_potentials_v: Vec<f64>,
    pub branch_currents_a: Vec<f64>,
    pub ground_currents_a: Vec<f64>,
    pub max_abs_kcl_residual_a: f64,
}

/// Input and numerical errors. Invalid or unsupported configurations fail
/// closed instead of silently omitting a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmdError {
    EmptyPath,
    ZeroNodes,
    NonFiniteInput,
    InvalidBranchNode,
    SelfLoop,
    InvalidBranchResistance,
    InvalidGroundNode,
    InvalidGroundResistance,
    UngroundedComponent,
    SingularOrIllConditionedNetwork,
}

impl fmt::Display for GmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyPath => "the field path contains no segments",
            Self::ZeroNodes => "the network must contain at least one node",
            Self::NonFiniteInput => "an input or intermediate value is not finite",
            Self::InvalidBranchNode => "a branch endpoint is outside the network",
            Self::SelfLoop => "a branch cannot connect a node to itself",
            Self::InvalidBranchResistance => {
                "branch resistance must be positive, finite, and numerically invertible"
            }
            Self::InvalidGroundNode => "a ground connection references an unknown node",
            Self::InvalidGroundResistance => {
                "ground resistance must be positive, finite, and numerically invertible"
            }
            Self::UngroundedComponent => {
                "every connected network component requires an explicit ground path"
            }
            Self::SingularOrIllConditionedNetwork => {
                "the nodal system is singular or numerically ill-conditioned"
            }
        };
        f.write_str(message)
    }
}

impl Error for GmdError {}

/// Solve the linear quasi-DC nodal equations.
///
/// For each branch oriented i -> j, current is
/// (phi_i - phi_j + V_ij) / R_ij. Positive ground current is from a node to
/// reference ground: phi_i / R_ground. Kirchhoff current balance is imposed
/// at every node.
///
/// The solver uses dense Gaussian elimination with partial pivoting. It is a
/// small-network reference oracle, not a large-grid production solver.
pub fn solve_quasi_dc(network: &ResistiveNetwork) -> Result<NetworkSolution, GmdError> {
    let n = network.node_count;
    if n == 0 {
        return Err(GmdError::ZeroNodes);
    }

    let mut adjacency = vec![Vec::<usize>::new(); n];
    let mut node_has_ground = vec![false; n];

    for branch in &network.branches {
        if branch.from >= n || branch.to >= n {
            return Err(GmdError::InvalidBranchNode);
        }
        if branch.from == branch.to {
            return Err(GmdError::SelfLoop);
        }
        validate_positive_resistance(branch.resistance_ohm)
            .map_err(|_| GmdError::InvalidBranchResistance)?;
        if !branch.induced_voltage_v.is_finite() {
            return Err(GmdError::NonFiniteInput);
        }

        adjacency[branch.from].push(branch.to);
        adjacency[branch.to].push(branch.from);
    }

    for ground in &network.grounds {
        if ground.node >= n {
            return Err(GmdError::InvalidGroundNode);
        }
        validate_positive_resistance(ground.resistance_ohm)
            .map_err(|_| GmdError::InvalidGroundResistance)?;
        node_has_ground[ground.node] = true;
    }

    ensure_every_component_is_grounded(&adjacency, &node_has_ground)?;

    let mut matrix = vec![vec![0.0_f64; n]; n];
    let mut rhs = vec![0.0_f64; n];

    for branch in &network.branches {
        let conductance = 1.0 / branch.resistance_ohm;
        let i = branch.from;
        let j = branch.to;

        matrix[i][i] += conductance;
        matrix[j][j] += conductance;
        matrix[i][j] -= conductance;
        matrix[j][i] -= conductance;

        // Nodal balance with I_i->j = (phi_i - phi_j + V_ij) / R.
        rhs[i] -= conductance * branch.induced_voltage_v;
        rhs[j] += conductance * branch.induced_voltage_v;
    }

    let ground_conductances: Vec<f64> = network
        .grounds
        .iter()
        .map(|ground| 1.0 / ground.resistance_ohm)
        .collect();

    for (ground, conductance) in network.grounds.iter().zip(ground_conductances.iter()) {
        matrix[ground.node][ground.node] += conductance;
    }

    if matrix.iter().flatten().any(|value| !value.is_finite())
        || rhs.iter().any(|value| !value.is_finite())
    {
        return Err(GmdError::NonFiniteInput);
    }

    let node_potentials_v = solve_dense(matrix, rhs)?;
    if node_potentials_v.iter().any(|value| !value.is_finite()) {
        return Err(GmdError::NonFiniteInput);
    }

    let mut branch_currents_a = Vec::with_capacity(network.branches.len());
    for branch in &network.branches {
        let current_a = (node_potentials_v[branch.from] - node_potentials_v[branch.to]
            + branch.induced_voltage_v)
            / branch.resistance_ohm;
        if !current_a.is_finite() {
            return Err(GmdError::NonFiniteInput);
        }
        branch_currents_a.push(current_a);
    }

    let mut ground_currents_a = Vec::with_capacity(network.grounds.len());
    for (ground, conductance) in network.grounds.iter().zip(ground_conductances.iter()) {
        let current_a = node_potentials_v[ground.node] * conductance;
        if !current_a.is_finite() {
            return Err(GmdError::NonFiniteInput);
        }
        ground_currents_a.push(current_a);
    }

    let mut kcl_residuals_a = vec![0.0_f64; n];
    for (branch, current_a) in network.branches.iter().zip(branch_currents_a.iter()) {
        kcl_residuals_a[branch.from] += current_a;
        kcl_residuals_a[branch.to] -= current_a;
    }
    for (ground, current_a) in network.grounds.iter().zip(ground_currents_a.iter()) {
        kcl_residuals_a[ground.node] += current_a;
    }

    let max_abs_kcl_residual_a = kcl_residuals_a
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if !max_abs_kcl_residual_a.is_finite() {
        return Err(GmdError::NonFiniteInput);
    }

    Ok(NetworkSolution {
        node_potentials_v,
        branch_currents_a,
        ground_currents_a,
        max_abs_kcl_residual_a,
    })
}

fn validate_positive_resistance(resistance_ohm: f64) -> Result<(), GmdError> {
    if !resistance_ohm.is_finite() {
        return Err(GmdError::NonFiniteInput);
    }
    if resistance_ohm <= 0.0 || !(1.0 / resistance_ohm).is_finite() {
        return Err(GmdError::InvalidBranchResistance);
    }
    Ok(())
}

fn ensure_every_component_is_grounded(
    adjacency: &[Vec<usize>],
    node_has_ground: &[bool],
) -> Result<(), GmdError> {
    let n = adjacency.len();
    let mut visited = vec![false; n];

    for root in 0..n {
        if visited[root] {
            continue;
        }

        let mut queue = VecDeque::new();
        queue.push_back(root);
        visited[root] = true;
        let mut component_has_ground = false;

        while let Some(node) = queue.pop_front() {
            component_has_ground |= node_has_ground[node];
            for &neighbour in &adjacency[node] {
                if !visited[neighbour] {
                    visited[neighbour] = true;
                    queue.push_back(neighbour);
                }
            }
        }

        if !component_has_ground {
            return Err(GmdError::UngroundedComponent);
        }
    }

    Ok(())
}

fn solve_dense(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Result<Vec<f64>, GmdError> {
    let n = rhs.len();
    let matrix_scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);

    if !matrix_scale.is_finite() || matrix_scale == 0.0 {
        return Err(GmdError::SingularOrIllConditionedNetwork);
    }

    let pivot_floor = f64::EPSILON * matrix_scale * n as f64 * 16.0;

    for column in 0..n {
        let pivot_row = (column..n)
            .max_by(|&left, &right| {
                matrix[left][column]
                    .abs()
                    .total_cmp(&matrix[right][column].abs())
            })
            .ok_or(GmdError::SingularOrIllConditionedNetwork)?;

        if matrix[pivot_row][column].abs() <= pivot_floor {
            return Err(GmdError::SingularOrIllConditionedNetwork);
        }

        if pivot_row != column {
            matrix.swap(pivot_row, column);
            rhs.swap(pivot_row, column);
        }

        let pivot = matrix[column][column];
        for row in (column + 1)..n {
            let factor = matrix[row][column] / pivot;
            matrix[row][column] = 0.0;
            for next_column in (column + 1)..n {
                matrix[row][next_column] -= factor * matrix[column][next_column];
            }
            rhs[row] -= factor * rhs[column];
        }
    }

    let mut solution = vec![0.0_f64; n];
    for row in (0..n).rev() {
        let known_sum = ((row + 1)..n)
            .map(|column| matrix[row][column] * solution[column])
            .sum::<f64>();
        let pivot = matrix[row][row];
        if !pivot.is_finite() || pivot.abs() <= pivot_floor {
            return Err(GmdError::SingularOrIllConditionedNetwork);
        }
        solution[row] = (rhs[row] - known_sum) / pivot;
    }

    Ok(solution)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "expected {left} ~= {right} (tolerance {tolerance})"
        );
    }

    #[test]
    fn line_integral_uses_oriented_east_north_path() {
        let path = [FieldPathSegment {
            electric_field_v_per_m: [1.0e-3, 2.0e-3],
            delta_east_north_m: [5_000.0, 2_000.0],
        }];
        approx_eq(integrate_induced_voltage_v(&path).unwrap(), 9.0, 1.0e-12);

        let reversed = [FieldPathSegment {
            electric_field_v_per_m: [1.0e-3, 2.0e-3],
            delta_east_north_m: [-5_000.0, -2_000.0],
        }];
        approx_eq(
            integrate_induced_voltage_v(&reversed).unwrap(),
            -9.0,
            1.0e-12,
        );
    }

    #[test]
    fn piecewise_field_integral_is_additive() {
        let path = [
            FieldPathSegment {
                electric_field_v_per_m: [1.0e-3, 0.0],
                delta_east_north_m: [4_000.0, 0.0],
            },
            FieldPathSegment {
                electric_field_v_per_m: [0.0, 2.0e-3],
                delta_east_north_m: [0.0, 3_000.0],
            },
        ];
        approx_eq(integrate_induced_voltage_v(&path).unwrap(), 10.0, 1.0e-12);
    }

    #[test]
    fn two_ground_single_branch_matches_hand_solution() {
        let network = ResistiveNetwork {
            node_count: 2,
            branches: vec![Branch {
                from: 0,
                to: 1,
                resistance_ohm: 2.0,
                induced_voltage_v: 10.0,
            }],
            grounds: vec![
                Ground {
                    node: 0,
                    resistance_ohm: 1.0,
                },
                Ground {
                    node: 1,
                    resistance_ohm: 1.0,
                },
            ],
        };

        let solution = solve_quasi_dc(&network).unwrap();
        approx_eq(solution.branch_currents_a[0], 2.5, 1.0e-12);
        approx_eq(solution.node_potentials_v[0], -2.5, 1.0e-12);
        approx_eq(solution.node_potentials_v[1], 2.5, 1.0e-12);
        approx_eq(solution.ground_currents_a[0], -2.5, 1.0e-12);
        approx_eq(solution.ground_currents_a[1], 2.5, 1.0e-12);
        assert!(solution.max_abs_kcl_residual_a < 1.0e-12);
    }

    #[test]
    fn reversing_branch_orientation_reverses_signed_current() {
        let forward = ResistiveNetwork {
            node_count: 2,
            branches: vec![Branch {
                from: 0,
                to: 1,
                resistance_ohm: 2.0,
                induced_voltage_v: 10.0,
            }],
            grounds: vec![
                Ground {
                    node: 0,
                    resistance_ohm: 1.0,
                },
                Ground {
                    node: 1,
                    resistance_ohm: 1.0,
                },
            ],
        };
        let reverse = ResistiveNetwork {
            branches: vec![Branch {
                from: 1,
                to: 0,
                resistance_ohm: 2.0,
                induced_voltage_v: -10.0,
            }],
            ..forward.clone()
        };

        let forward_solution = solve_quasi_dc(&forward).unwrap();
        let reverse_solution = solve_quasi_dc(&reverse).unwrap();
        approx_eq(
            forward_solution.branch_currents_a[0],
            -reverse_solution.branch_currents_a[0],
            1.0e-12,
        );
        approx_eq(
            forward_solution.node_potentials_v[0],
            reverse_solution.node_potentials_v[0],
            1.0e-12,
        );
    }

    #[test]
    fn parallel_branches_share_current_and_conserve_kcl() {
        let network = ResistiveNetwork {
            node_count: 2,
            branches: vec![
                Branch {
                    from: 0,
                    to: 1,
                    resistance_ohm: 2.0,
                    induced_voltage_v: 10.0,
                },
                Branch {
                    from: 0,
                    to: 1,
                    resistance_ohm: 2.0,
                    induced_voltage_v: 10.0,
                },
            ],
            grounds: vec![
                Ground {
                    node: 0,
                    resistance_ohm: 1.0,
                },
                Ground {
                    node: 1,
                    resistance_ohm: 1.0,
                },
            ],
        };

        let solution = solve_quasi_dc(&network).unwrap();
        approx_eq(solution.branch_currents_a[0], 5.0 / 3.0, 1.0e-12);
        approx_eq(solution.branch_currents_a[1], 5.0 / 3.0, 1.0e-12);
        assert!(solution.max_abs_kcl_residual_a < 1.0e-12);
    }

    #[test]
    fn disconnected_ungrounded_component_is_rejected() {
        let network = ResistiveNetwork {
            node_count: 3,
            branches: vec![Branch {
                from: 0,
                to: 1,
                resistance_ohm: 2.0,
                induced_voltage_v: 10.0,
            }],
            grounds: vec![Ground {
                node: 0,
                resistance_ohm: 1.0,
            }],
        };

        assert_eq!(
            solve_quasi_dc(&network),
            Err(GmdError::UngroundedComponent)
        );
    }

    #[test]
    fn non_finite_field_is_rejected() {
        let path = [FieldPathSegment {
            electric_field_v_per_m: [f64::NAN, 0.0],
            delta_east_north_m: [10.0, 0.0],
        }];
        assert_eq!(
            integrate_induced_voltage_v(&path),
            Err(GmdError::NonFiniteInput)
        );
    }

    #[test]
    fn non_positive_branch_resistance_is_rejected() {
        let network = ResistiveNetwork {
            node_count: 2,
            branches: vec![Branch {
                from: 0,
                to: 1,
                resistance_ohm: 0.0,
                induced_voltage_v: 1.0,
            }],
            grounds: vec![
                Ground {
                    node: 0,
                    resistance_ohm: 1.0,
                },
                Ground {
                    node: 1,
                    resistance_ohm: 1.0,
                },
            ],
        };

        assert_eq!(
            solve_quasi_dc(&network),
            Err(GmdError::InvalidBranchResistance)
        );
    }

    #[test]
    fn ungrounded_empty_network_is_not_silently_accepted() {
        let network = ResistiveNetwork {
            node_count: 1,
            branches: Vec::new(),
            grounds: Vec::new(),
        };
        assert_eq!(
            solve_quasi_dc(&network),
            Err(GmdError::UngroundedComponent)
        );
    }
}
