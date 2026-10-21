// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dependency-free, linear-elastic 2D pin-jointed truss reference solver.
//!
//! This is a small-displacement, axial-member model. It excludes member bending,
//! geometric/material nonlinearity, buckling, contact, fracture and dynamics.
//! A successful result verifies this mathematical model only; it does not
//! validate a real structure or authorize construction/demolition work.

use crate::Check;
use std::collections::{BTreeMap, BTreeSet};

const MAX_NODES: usize = 128;
const MAX_BARS: usize = 512;
const PIVOT_EPSILON_FACTOR: f64 = 100.0;

#[derive(Clone, Debug, PartialEq)]
pub struct TrussNode {
    pub id: String,
    pub x_m: f64,
    pub y_m: f64,
    pub restrained_x: bool,
    pub restrained_y: bool,
    pub force_x_n: f64,
    pub force_y_n: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrussBar {
    pub id: String,
    pub node_i: String,
    pub node_j: String,
    pub area_m2: f64,
    pub youngs_modulus_pa: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlanarTrussInput {
    pub nodes: Vec<TrussNode>,
    pub bars: Vec<TrussBar>,
}

/// Canonical analytical known-answer input used by the reference receipt.
///
/// Two fixed base nodes and one free apex form a symmetric V-truss. The downward
/// 1000 N apex load produces compression in both equal diagonal axial members.
pub fn symmetric_two_bar_reference_fixture() -> PlanarTrussInput {
    PlanarTrussInput {
        nodes: vec![
            TrussNode {
                id: "left".to_owned(),
                x_m: 0.0,
                y_m: 0.0,
                restrained_x: true,
                restrained_y: true,
                force_x_n: 0.0,
                force_y_n: 0.0,
            },
            TrussNode {
                id: "right".to_owned(),
                x_m: 2.0,
                y_m: 0.0,
                restrained_x: true,
                restrained_y: true,
                force_x_n: 0.0,
                force_y_n: 0.0,
            },
            TrussNode {
                id: "apex".to_owned(),
                x_m: 1.0,
                y_m: 1.0,
                restrained_x: false,
                restrained_y: false,
                force_x_n: 0.0,
                force_y_n: -1_000.0,
            },
        ],
        bars: vec![
            TrussBar {
                id: "left-diagonal".to_owned(),
                node_i: "left".to_owned(),
                node_j: "apex".to_owned(),
                area_m2: 0.01,
                youngs_modulus_pa: 200.0e9,
            },
            TrussBar {
                id: "right-diagonal".to_owned(),
                node_i: "right".to_owned(),
                node_j: "apex".to_owned(),
                area_m2: 0.01,
                youngs_modulus_pa: 200.0e9,
            },
        ],
    }
}

/// Canonical nonsymmetric two-bar truss with mixed horizontal/vertical loading.
///
/// Supports A=(0,0), B=(4,0); free apex C=(1,3). Bars have E=200 GPa
/// and areas chosen for approximately EA/L = 1e9 and 2e9 N/m. The asymmetric
/// geometry makes the off-diagonal direction-cosine terms observable.
pub fn asymmetric_two_bar_reference_fixture() -> PlanarTrussInput {
    PlanarTrussInput {
        nodes: vec![
            TrussNode {
                id: "A".to_owned(),
                x_m: 0.0,
                y_m: 0.0,
                restrained_x: true,
                restrained_y: true,
                force_x_n: 0.0,
                force_y_n: 0.0,
            },
            TrussNode {
                id: "B".to_owned(),
                x_m: 4.0,
                y_m: 0.0,
                restrained_x: true,
                restrained_y: true,
                force_x_n: 0.0,
                force_y_n: 0.0,
            },
            TrussNode {
                id: "C".to_owned(),
                x_m: 1.0,
                y_m: 3.0,
                restrained_x: false,
                restrained_y: false,
                force_x_n: 600.0,
                force_y_n: -1_200.0,
            },
        ],
        bars: vec![
            TrussBar {
                id: "AC".to_owned(),
                node_i: "A".to_owned(),
                node_j: "C".to_owned(),
                area_m2: 0.015_811_388_300_841_896,
                youngs_modulus_pa: 200.0e9,
            },
            TrussBar {
                id: "BC".to_owned(),
                node_i: "B".to_owned(),
                node_j: "C".to_owned(),
                area_m2: 0.042_426_406_871_192_854,
                youngs_modulus_pa: 200.0e9,
            },
        ],
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrussNodeResult {
    pub node_id: String,
    pub displacement_x_m: f64,
    pub displacement_y_m: f64,
    /// Present only on a restrained degree of freedom.
    pub reaction_x_n: Option<f64>,
    /// Present only on a restrained degree of freedom.
    pub reaction_y_n: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrussBarResult {
    pub bar_id: String,
    pub length_m: f64,
    pub axial_extension_m: f64,
    pub axial_strain: f64,
    /// Tension is positive; compression is negative.
    pub axial_force_n: f64,
    /// Tension is positive; compression is negative.
    pub axial_stress_pa: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EquilibriumSummary {
    pub applied_force_x_n: f64,
    pub applied_force_y_n: f64,
    pub support_reaction_x_n: f64,
    pub support_reaction_y_n: f64,
    /// Applied loads plus support reactions; ideal static equilibrium is zero.
    pub net_force_residual_x_n: f64,
    pub net_force_residual_y_n: f64,
    /// Moment about the global origin, in N m.
    pub net_moment_residual_nm: f64,
    pub normalized_force_residual: f64,
    pub normalized_moment_residual: f64,
    /// Largest |K u - F| at a free degree of freedom.
    pub max_free_dof_residual_n: f64,
    pub normalized_free_dof_residual: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnergySummary {
    /// Element strain energy, joules, reconstructed from axial force and extension.
    pub strain_energy_j: f64,
    /// One-half applied load dot displacement, joules, for proportional linear loading.
    pub external_work_j: f64,
    /// Strain energy minus external work, joules.
    pub residual_j: f64,
    /// Absolute energy residual divided by the larger absolute energy (or MIN_POSITIVE).
    pub normalized_residual: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlanarTrussResult {
    pub node_results: Vec<TrussNodeResult>,
    pub bar_results: Vec<TrussBarResult>,
    pub equilibrium: EquilibriumSummary,
    pub energy: EnergySummary,
    pub claim_ceiling: &'static str,
}

/// Canonical, order-independent textual representation used to bind a
/// fixture request to a receipt. This is not itself a cryptographic hash.
/// Call `solve` first to validate the model; IDs may not contain the pipe
/// delimiter because `solve` rejects characters outside its ID grammar.
pub fn canonical_request(fixture_id: &str, input: &PlanarTrussInput) -> Check<String> {
    validate_id(fixture_id, "fixture ID")?;
    let mut nodes = input.nodes.clone();
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut bars = input.bars.clone();
    bars.sort_by(|left, right| left.id.cmp(&right.id));

    let mut fields = vec![
        "schema=planar-truss-request-v0.1".to_owned(),
        format!("fixture={fixture_id}"),
    ];
    for node in nodes {
        fields.push(format!(
            "node|{}|{}|{}|{}|{}|{}|{}",
            node.id,
            node.x_m,
            node.y_m,
            node.restrained_x,
            node.restrained_y,
            node.force_x_n,
            node.force_y_n
        ));
    }
    for bar in bars {
        fields.push(format!(
            "bar|{}|{}|{}|{}|{}",
            bar.id, bar.node_i, bar.node_j, bar.area_m2, bar.youngs_modulus_pa
        ));
    }
    Ok(fields.join("|"))
}

/// Assemble and solve a finite, small-displacement, pin-jointed 2D truss.
///
/// Coordinates are metres, forces newtons, area square metres, and Young's
/// modulus pascals. Nodes and bars are sorted by ID before assembly so identical
/// input data yields stable result ordering independent of caller input order.
pub fn solve(input: &PlanarTrussInput) -> Check<PlanarTrussResult> {
    if input.nodes.len() < 2 || input.nodes.len() > MAX_NODES {
        return Err(format!(
            "planar truss must contain between 2 and {MAX_NODES} nodes"
        ));
    }
    if input.bars.is_empty() || input.bars.len() > MAX_BARS {
        return Err(format!(
            "planar truss must contain between 1 and {MAX_BARS} bars"
        ));
    }

    let mut nodes = input.nodes.clone();
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut bars = input.bars.clone();
    bars.sort_by(|left, right| left.id.cmp(&right.id));

    let mut node_ids = BTreeSet::new();
    let mut node_index = BTreeMap::new();
    for (index, node) in nodes.iter().enumerate() {
        validate_id(&node.id, "node ID")?;
        if !node_ids.insert(node.id.clone()) {
            return Err(format!("duplicate planar truss node ID: {}", node.id));
        }
        require_finite(node.x_m, "node x coordinate")?;
        require_finite(node.y_m, "node y coordinate")?;
        require_finite(node.force_x_n, "node x load")?;
        require_finite(node.force_y_n, "node y load")?;
        node_index.insert(node.id.clone(), index);
    }

    let mut bar_ids = BTreeSet::new();
    let mut connected_nodes = BTreeSet::new();
    let mut bar_geometry = BTreeMap::new();
    for bar in &bars {
        validate_id(&bar.id, "bar ID")?;
        if !bar_ids.insert(bar.id.clone()) {
            return Err(format!("duplicate planar truss bar ID: {}", bar.id));
        }
        let i = *node_index
            .get(&bar.node_i)
            .ok_or_else(|| format!("bar {} references unknown node {}", bar.id, bar.node_i))?;
        let j = *node_index
            .get(&bar.node_j)
            .ok_or_else(|| format!("bar {} references unknown node {}", bar.id, bar.node_j))?;
        if i == j {
            return Err(format!("bar {} cannot connect a node to itself", bar.id));
        }
        require_positive_finite(bar.area_m2, "bar area")?;
        require_positive_finite(bar.youngs_modulus_pa, "bar Young's modulus")?;

        let dx = nodes[j].x_m - nodes[i].x_m;
        let dy = nodes[j].y_m - nodes[i].y_m;
        require_finite(dx, "bar delta-x")?;
        require_finite(dy, "bar delta-y")?;
        let length = dx.hypot(dy);
        require_positive_finite(length, "bar length")?;
        let cosine = dx / length;
        let sine = dy / length;
        let axial_stiffness = bar.youngs_modulus_pa * bar.area_m2 / length;
        require_positive_finite(axial_stiffness, "bar axial stiffness")?;
        let geometry = BarGeometry {
            i,
            j,
            length,
            cosine,
            sine,
            axial_stiffness,
        };
        bar_geometry.insert(bar.id.clone(), geometry);
        connected_nodes.insert(bar.node_i.clone());
        connected_nodes.insert(bar.node_j.clone());
    }

    for node in &nodes {
        if !connected_nodes.contains(&node.id) {
            return Err(format!(
                "isolated node is not a valid truss node: {}",
                node.id
            ));
        }
    }

    let dof_count = nodes
        .len()
        .checked_mul(2)
        .ok_or_else(|| "planar truss degree-of-freedom count overflow".to_owned())?;
    let mut stiffness = vec![vec![0.0_f64; dof_count]; dof_count];
    let mut loads = vec![0.0_f64; dof_count];

    for (index, node) in nodes.iter().enumerate() {
        loads[2 * index] = node.force_x_n;
        loads[2 * index + 1] = node.force_y_n;
    }

    for bar in &bars {
        let geometry = bar_geometry
            .get(&bar.id)
            .ok_or_else(|| format!("bar geometry absent for {}", bar.id))?;
        assemble_bar_stiffness(&mut stiffness, geometry)?;
    }

    let mut free_dofs = Vec::new();
    let mut restrained_dofs = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if node.restrained_x {
            restrained_dofs.push(2 * index);
        } else {
            free_dofs.push(2 * index);
        }
        if node.restrained_y {
            restrained_dofs.push(2 * index + 1);
        } else {
            free_dofs.push(2 * index + 1);
        }
    }

    let mut displacement = vec![0.0_f64; dof_count];
    if !free_dofs.is_empty() {
        let reduced_stiffness: Vec<Vec<f64>> = free_dofs
            .iter()
            .map(|row| {
                free_dofs
                    .iter()
                    .map(|column| stiffness[*row][*column])
                    .collect()
            })
            .collect();
        let reduced_loads: Vec<f64> = free_dofs.iter().map(|dof| loads[*dof]).collect();
        let reduced_displacement = solve_dense(reduced_stiffness, reduced_loads)?;
        for (dof, value) in free_dofs.iter().zip(reduced_displacement) {
            require_finite(value, "solved degree-of-freedom displacement")?;
            displacement[*dof] = value;
        }
    }

    let mut internal = vec![0.0_f64; dof_count];
    let mut max_free_dof_residual_n = 0.0_f64;
    let mut max_abs_applied_load_n = 0.0_f64;
    let mut max_abs_internal_force_n = 0.0_f64;
    for row in 0..dof_count {
        let mut total = 0.0_f64;
        for column in 0..dof_count {
            total += stiffness[row][column] * displacement[column];
        }
        require_finite(total, "global internal nodal force")?;
        internal[row] = total;
        max_abs_internal_force_n = max_abs_internal_force_n.max(total.abs());
        max_abs_applied_load_n = max_abs_applied_load_n.max(loads[row].abs());
    }
    for dof in &free_dofs {
        max_free_dof_residual_n = max_free_dof_residual_n.max((internal[*dof] - loads[*dof]).abs());
    }

    let mut node_results = Vec::with_capacity(nodes.len());
    let mut reaction_sum_x = 0.0_f64;
    let mut reaction_sum_y = 0.0_f64;
    let mut reaction_moment = 0.0_f64;
    let mut max_abs_reaction_n = 0.0_f64;
    let mut max_abs_applied_moment_nm = 0.0_f64;
    let mut applied_sum_x = 0.0_f64;
    let mut applied_sum_y = 0.0_f64;
    let mut applied_moment = 0.0_f64;
    let mut characteristic_length_m = 1.0_f64;

    for (index, node) in nodes.iter().enumerate() {
        characteristic_length_m = characteristic_length_m
            .max(node.x_m.abs())
            .max(node.y_m.abs());
        applied_sum_x += node.force_x_n;
        applied_sum_y += node.force_y_n;
        let applied_node_moment = node.x_m * node.force_y_n - node.y_m * node.force_x_n;
        require_finite(applied_node_moment, "applied nodal moment")?;
        applied_moment += applied_node_moment;
        max_abs_applied_moment_nm = max_abs_applied_moment_nm.max(applied_node_moment.abs());

        let reaction_x = if node.restrained_x {
            let value = internal[2 * index] - loads[2 * index];
            require_finite(value, "support reaction x")?;
            reaction_sum_x += value;
            max_abs_reaction_n = max_abs_reaction_n.max(value.abs());
            Some(value)
        } else {
            None
        };
        let reaction_y = if node.restrained_y {
            let value = internal[2 * index + 1] - loads[2 * index + 1];
            require_finite(value, "support reaction y")?;
            reaction_sum_y += value;
            max_abs_reaction_n = max_abs_reaction_n.max(value.abs());
            Some(value)
        } else {
            None
        };

        let reaction_x_value = reaction_x.unwrap_or(0.0);
        let reaction_y_value = reaction_y.unwrap_or(0.0);
        let reaction_node_moment = node.x_m * reaction_y_value - node.y_m * reaction_x_value;
        require_finite(reaction_node_moment, "support reaction moment")?;
        reaction_moment += reaction_node_moment;

        node_results.push(TrussNodeResult {
            node_id: node.id.clone(),
            displacement_x_m: displacement[2 * index],
            displacement_y_m: displacement[2 * index + 1],
            reaction_x_n: reaction_x,
            reaction_y_n: reaction_y,
        });
    }

    require_finite(applied_sum_x, "sum of applied x loads")?;
    require_finite(applied_sum_y, "sum of applied y loads")?;
    require_finite(reaction_sum_x, "sum of support x reactions")?;
    require_finite(reaction_sum_y, "sum of support y reactions")?;
    require_finite(applied_moment, "sum of applied moments")?;
    require_finite(reaction_moment, "sum of support moments")?;
    require_finite(max_abs_reaction_n, "maximum support reaction")?;

    let net_force_x = applied_sum_x + reaction_sum_x;
    let net_force_y = applied_sum_y + reaction_sum_y;
    let net_moment = applied_moment + reaction_moment;
    require_finite(net_force_x, "net x-force residual")?;
    require_finite(net_force_y, "net y-force residual")?;
    require_finite(net_moment, "net moment residual")?;
    let force_scale = max_abs_applied_load_n
        .max(max_abs_internal_force_n)
        .max(max_abs_reaction_n)
        .max(1.0);
    let moment_scale = (force_scale * characteristic_length_m)
        .max(max_abs_applied_moment_nm)
        .max(reaction_moment.abs())
        .max(1.0);
    require_positive_finite(force_scale, "force normalization scale")?;
    require_positive_finite(moment_scale, "moment normalization scale")?;

    let equilibrium = EquilibriumSummary {
        applied_force_x_n: applied_sum_x,
        applied_force_y_n: applied_sum_y,
        support_reaction_x_n: reaction_sum_x,
        support_reaction_y_n: reaction_sum_y,
        net_force_residual_x_n: net_force_x,
        net_force_residual_y_n: net_force_y,
        net_moment_residual_nm: net_moment,
        normalized_force_residual: net_force_x.hypot(net_force_y) / force_scale,
        normalized_moment_residual: net_moment.abs() / moment_scale,
        max_free_dof_residual_n,
        normalized_free_dof_residual: max_free_dof_residual_n / force_scale,
    };
    for value in [
        equilibrium.normalized_force_residual,
        equilibrium.normalized_moment_residual,
        equilibrium.normalized_free_dof_residual,
    ] {
        require_finite(value, "normalized equilibrium residual")?;
    }

    let mut bar_results = Vec::with_capacity(bars.len());
    for bar in &bars {
        let geometry = bar_geometry
            .get(&bar.id)
            .ok_or_else(|| format!("bar geometry absent for {}", bar.id))?;
        let ux_i = displacement[2 * geometry.i];
        let uy_i = displacement[2 * geometry.i + 1];
        let ux_j = displacement[2 * geometry.j];
        let uy_j = displacement[2 * geometry.j + 1];
        let extension = geometry.cosine * (ux_j - ux_i) + geometry.sine * (uy_j - uy_i);
        let strain = extension / geometry.length;
        let axial_force = bar.youngs_modulus_pa * bar.area_m2 * strain;
        let axial_stress = bar.youngs_modulus_pa * strain;
        require_finite(extension, "bar axial extension")?;
        require_finite(strain, "bar axial strain")?;
        require_finite(axial_force, "bar axial force")?;
        require_finite(axial_stress, "bar axial stress")?;
        bar_results.push(TrussBarResult {
            bar_id: bar.id.clone(),
            length_m: geometry.length,
            axial_extension_m: extension,
            axial_strain: strain,
            axial_force_n: axial_force,
            axial_stress_pa: axial_stress,
        });
    }

    // Energy balance is computed independently from global equilibrium
    // diagnostics: member axial force × extension versus applied load ×
    // displacement. Supports have zero prescribed displacement, so reactions
    // perform no work in this formulation.
    let mut strain_energy_j = 0.0_f64;
    for bar in &bar_results {
        let contribution = 0.5 * bar.axial_force_n * bar.axial_extension_m;
        require_finite(contribution, "member strain-energy contribution")?;
        strain_energy_j += contribution;
        require_finite(strain_energy_j, "total strain energy")?;
    }

    let mut full_load_dot_displacement = 0.0_f64;
    for (&applied_load, &node_displacement) in loads.iter().zip(displacement.iter()) {
        let contribution = applied_load * node_displacement;
        require_finite(contribution, "applied-load work contribution")?;
        full_load_dot_displacement += contribution;
        require_finite(full_load_dot_displacement, "applied load-displacement sum")?;
    }
    let external_work_j = 0.5 * full_load_dot_displacement;
    let residual_j = strain_energy_j - external_work_j;
    let energy_scale = strain_energy_j
        .abs()
        .max(external_work_j.abs())
        .max(f64::MIN_POSITIVE);
    let normalized_residual = residual_j.abs() / energy_scale;
    for (label, value) in [
        ("external work", external_work_j),
        ("energy residual", residual_j),
        ("normalized energy residual", normalized_residual),
    ] {
        require_finite(value, label)?;
    }

    Ok(PlanarTrussResult {
        node_results,
        bar_results,
        equilibrium,
        energy: EnergySummary {
            strain_energy_j,
            external_work_j,
            residual_j,
            normalized_residual,
        },
        claim_ceiling: "linear-elastic small-displacement 2D pin-jointed truss only; not member bending/buckling, nonlinear response, physical validation, structural safety, code compliance, or construction/demolition authorization",
    })
}

#[derive(Clone, Copy, Debug)]
struct BarGeometry {
    i: usize,
    j: usize,
    length: f64,
    cosine: f64,
    sine: f64,
    axial_stiffness: f64,
}

fn assemble_bar_stiffness(stiffness: &mut [Vec<f64>], bar: &BarGeometry) -> Check<()> {
    let c = bar.cosine;
    let s = bar.sine;
    let cc = c * c;
    let cs = c * s;
    let ss = s * s;
    let local = [
        [cc, cs, -cc, -cs],
        [cs, ss, -cs, -ss],
        [-cc, -cs, cc, cs],
        [-cs, -ss, cs, ss],
    ];
    let dofs = [2 * bar.i, 2 * bar.i + 1, 2 * bar.j, 2 * bar.j + 1];
    for row in 0..4 {
        for column in 0..4 {
            let contribution = bar.axial_stiffness * local[row][column];
            require_finite(contribution, "bar stiffness contribution")?;
            let global_row = dofs[row];
            let global_column = dofs[column];
            stiffness[global_row][global_column] += contribution;
            require_finite(
                stiffness[global_row][global_column],
                "assembled global stiffness",
            )?;
        }
    }
    Ok(())
}

fn solve_dense(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Check<Vec<f64>> {
    let n = rhs.len();
    if matrix.len() != n || matrix.iter().any(|row| row.len() != n) {
        return Err("reduced stiffness matrix dimensions are inconsistent".to_owned());
    }
    if n == 0 {
        return Ok(Vec::new());
    }
    let mut matrix_scale = 0.0_f64;
    for row in &matrix {
        for value in row {
            require_finite(*value, "reduced stiffness matrix")?;
            matrix_scale = matrix_scale.max(value.abs());
        }
    }
    for value in &rhs {
        require_finite(*value, "reduced load vector")?;
    }
    if matrix_scale == 0.0 {
        return Err("singular truss: reduced stiffness matrix is zero".to_owned());
    }
    // The threshold must scale with the matrix itself; max(1.0) would make
    // the singularity decision depend on the chosen force/length units.
    let pivot_tolerance = f64::EPSILON * PIVOT_EPSILON_FACTOR * matrix_scale * n as f64;

    for pivot in 0..n {
        let mut pivot_row = pivot;
        let mut pivot_magnitude = matrix[pivot][pivot].abs();
        for (candidate, row_values) in matrix.iter().enumerate().skip(pivot + 1) {
            let magnitude = row_values[pivot].abs();
            if magnitude > pivot_magnitude {
                pivot_magnitude = magnitude;
                pivot_row = candidate;
            }
        }
        if !pivot_magnitude.is_finite() || pivot_magnitude <= pivot_tolerance {
            return Err(format!(
                "singular or numerically unresolved truss stiffness at reduced DOF {pivot}; no displacement result issued"
            ));
        }
        if pivot_row != pivot {
            matrix.swap(pivot, pivot_row);
            rhs.swap(pivot, pivot_row);
        }
        let diagonal = matrix[pivot][pivot];
        let pivot_values = matrix[pivot].clone();
        let pivot_rhs = rhs[pivot];
        for (row_index, row_values) in matrix.iter_mut().enumerate().skip(pivot + 1) {
            let factor = row_values[pivot] / diagonal;
            require_finite(factor, "Gaussian-elimination factor")?;
            row_values[pivot] = 0.0;
            for (column, entry) in row_values.iter_mut().enumerate().skip(pivot + 1) {
                *entry -= factor * pivot_values[column];
                require_finite(*entry, "Gaussian-elimination matrix")?;
            }
            rhs[row_index] -= factor * pivot_rhs;
            require_finite(rhs[row_index], "Gaussian-elimination load vector")?;
        }
    }

    let mut solution = vec![0.0_f64; n];
    for row in (0..n).rev() {
        let mut remainder = rhs[row];
        for (column, coefficient) in matrix[row].iter().enumerate().skip(row + 1) {
            remainder -= coefficient * solution[column];
        }
        require_finite(remainder, "back-substitution remainder")?;
        let diagonal = matrix[row][row];
        if diagonal.abs() <= pivot_tolerance {
            return Err(format!(
                "singular truss stiffness during back-substitution at reduced DOF {row}"
            ));
        }
        solution[row] = remainder / diagonal;
        require_finite(solution[row], "back-substituted displacement")?;
    }
    Ok(solution)
}

fn validate_id(value: &str, label: &str) -> Check<()> {
    if value.is_empty()
        || value.len() > 128
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
        })
    {
        return Err(format!("invalid {label}"));
    }
    Ok(())
}

fn require_finite(value: f64, label: &str) -> Check<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("{label} must be finite"))
    }
}

fn require_positive_finite(value: f64, label: &str) -> Check<()> {
    require_finite(value, label)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(format!("{label} must be positive and finite"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symmetric_two_bar_truss() -> PlanarTrussInput {
        symmetric_two_bar_reference_fixture()
    }

    fn asymmetric_two_bar_truss() -> PlanarTrussInput {
        asymmetric_two_bar_reference_fixture()
    }

    fn close(actual: f64, expected: f64, tolerance: f64, label: &str) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{label}: actual={actual:.17e}, expected={expected:.17e}, tolerance={tolerance:.17e}"
        );
    }

    fn node<'a>(result: &'a PlanarTrussResult, id: &str) -> &'a TrussNodeResult {
        result
            .node_results
            .iter()
            .find(|node| node.node_id == id)
            .expect("expected node result")
    }

    #[test]
    fn canonical_request_matches_frozen_bytes_and_ignores_input_order() {
        let original = symmetric_two_bar_truss();
        let expected =
            include_str!("../fixtures/planar_truss_request_v0_1.txt").trim_end_matches('\n');
        assert_eq!(
            canonical_request("PHYS-TRUSS-V-001", &original).expect("canonical request"),
            expected
        );

        let mut permuted = original;
        permuted.nodes.reverse();
        permuted.bars.reverse();
        assert_eq!(
            canonical_request("PHYS-TRUSS-V-001", &permuted).expect("permuted request"),
            expected
        );
        assert!(canonical_request("bad|fixture", &permuted).is_err());
    }

    #[test]
    fn symmetric_two_bar_truss_matches_independent_closed_form_oracle() {
        // For L=sqrt(2) m, theta=45 deg and P=1000 N:
        // ux(apex)=0; uy(apex)=-P*L/(EA) because 2*sin^2(theta)=1;
        // N_bar=-P/(2*sin(theta)); vertical reactions are P/2 each.
        let result = solve(&symmetric_two_bar_truss()).expect("stable symmetric truss");
        let expected_displacement = -1_000.0 * 2.0_f64.sqrt() / (0.01 * 200.0e9);
        let expected_bar_force = -1_000.0 / 2.0_f64.sqrt();
        close(
            node(&result, "apex").displacement_x_m,
            0.0,
            1.0e-15,
            "apex x",
        );
        close(
            node(&result, "apex").displacement_y_m,
            expected_displacement,
            1.0e-15,
            "apex y",
        );
        close(
            result.bar_results[0].axial_force_n,
            expected_bar_force,
            1.0e-9,
            "left diagonal force",
        );
        close(
            result.bar_results[1].axial_force_n,
            expected_bar_force,
            1.0e-9,
            "right diagonal force",
        );
        close(
            result.bar_results[0].axial_stress_pa,
            expected_bar_force / 0.01,
            1.0e-6,
            "bar stress",
        );
        close(
            node(&result, "left")
                .reaction_x_n
                .expect("fixed x reaction"),
            500.0,
            1.0e-8,
            "left support x reaction",
        );
        close(
            node(&result, "right")
                .reaction_x_n
                .expect("fixed x reaction"),
            -500.0,
            1.0e-8,
            "right support x reaction",
        );
        close(
            node(&result, "left")
                .reaction_y_n
                .expect("fixed y reaction"),
            500.0,
            1.0e-8,
            "left support y reaction",
        );
        close(
            node(&result, "right")
                .reaction_y_n
                .expect("fixed y reaction"),
            500.0,
            1.0e-8,
            "right support y reaction",
        );
        assert!(result.equilibrium.normalized_force_residual < 1.0e-12);
        assert!(result.equilibrium.normalized_moment_residual < 1.0e-12);
        assert!(result.equilibrium.normalized_free_dof_residual < 1.0e-12);
        assert!(result.claim_ceiling.contains("not member bending/buckling"));

        let expected_strain_energy = 0.5 * 1_000.0 * expected_displacement.abs();
        close(
            result.energy.strain_energy_j,
            expected_strain_energy,
            expected_strain_energy.abs() * 1.0e-10,
            "closed-form strain energy",
        );
        close(
            result.energy.external_work_j,
            expected_strain_energy,
            expected_strain_energy.abs() * 1.0e-10,
            "closed-form external work",
        );
        close(
            result.energy.residual_j,
            0.0,
            expected_strain_energy.abs() * 1.0e-10,
            "closed-form energy residual",
        );
        assert!(
            result.energy.normalized_residual < 1.0e-12,
            "normalized energy residual={:.17e}",
            result.energy.normalized_residual
        );
    }

    #[test]
    fn asymmetric_request_matches_frozen_bytes_and_ignores_input_order() {
        let original = asymmetric_two_bar_truss();
        let expected = include_str!("../fixtures/planar_truss_asymmetric_request_v0_1.txt")
            .trim_end_matches('\n');
        assert_eq!(
            canonical_request("PHYS-TRUSS-ASYM-001", &original).expect("canonical asymmetric request"),
            expected
        );

        let mut permuted = original;
        permuted.nodes.reverse();
        permuted.bars.reverse();
        assert_eq!(
            canonical_request("PHYS-TRUSS-ASYM-001", &permuted)
                .expect("permuted asymmetric request"),
            expected
        );
    }

    #[test]
    fn asymmetric_two_bar_truss_matches_closed_form_oracle() {
        // Geometry A=(0,0), B=(4,0), C=(1,3), with EA/L=1e9 and 2e9 N/m.
        // For the free apex, K = 1e9 [[1.1,-0.7],[-0.7,1.9]] and F=(600,-1200) N.
        // This exercises off-diagonal coupling that cancels in the symmetric oracle.
        let result = solve(&asymmetric_two_bar_truss()).expect("stable asymmetric truss");
        let apex = node(&result, "C");
        close(
            apex.displacement_x_m,
            1.875e-7,
            1.0e-15,
            "asymmetric apex x",
        );
        close(
            apex.displacement_y_m,
            -5.625e-7,
            1.0e-15,
            "asymmetric apex y",
        );

        let left = node(&result, "A");
        let right = node(&result, "B");
        close(
            left.reaction_x_n.expect("A x reaction"),
            150.0,
            1.0e-6,
            "A x reaction",
        );
        close(
            left.reaction_y_n.expect("A y reaction"),
            450.0,
            1.0e-6,
            "A y reaction",
        );
        close(
            right.reaction_x_n.expect("B x reaction"),
            -750.0,
            1.0e-6,
            "B x reaction",
        );
        close(
            right.reaction_y_n.expect("B y reaction"),
            750.0,
            1.0e-6,
            "B y reaction",
        );

        let ac = result
            .bar_results
            .iter()
            .find(|bar| bar.bar_id == "AC")
            .expect("AC member");
        let bc = result
            .bar_results
            .iter()
            .find(|bar| bar.bar_id == "BC")
            .expect("BC member");
        let expected_ac_force = -1_500.0 / 10.0_f64.sqrt();
        let expected_bc_force = -1_500.0 / 2.0_f64.sqrt();
        close(ac.axial_force_n, expected_ac_force, 1.0e-6, "AC force");
        close(bc.axial_force_n, expected_bc_force, 1.0e-6, "BC force");
        close(ac.axial_stress_pa, -30_000.0, 1.0e-6, "AC stress");
        close(bc.axial_stress_pa, -25_000.0, 1.0e-6, "BC stress");
        close(ac.axial_strain, -1.5e-7, 1.0e-15, "AC strain");
        close(bc.axial_strain, -1.25e-7, 1.0e-15, "BC strain");
        close(
            ac.axial_extension_m,
            -1.5e-7 * 10.0_f64.sqrt(),
            1.0e-15,
            "AC extension",
        );
        close(
            bc.axial_extension_m,
            -1.25e-7 * 18.0_f64.sqrt(),
            1.0e-15,
            "BC extension",
        );

        for (label, residual) in [
            ("force", result.equilibrium.normalized_force_residual),
            ("moment", result.equilibrium.normalized_moment_residual),
            ("free-DOF", result.equilibrium.normalized_free_dof_residual),
            ("energy", result.energy.normalized_residual),
        ] {
            assert!(
                residual < 1.0e-12,
                "asymmetric normalized {label} residual={residual:.17e}"
            );
        }
        let expected_energy_j = 3.9375e-4;
        close(
            result.energy.strain_energy_j,
            expected_energy_j,
            1.0e-12,
            "asymmetric strain energy",
        );
        close(
            result.energy.external_work_j,
            expected_energy_j,
            1.0e-12,
            "asymmetric external work",
        );
        close(
            result.energy.residual_j,
            0.0,
            1.0e-12,
            "asymmetric energy residual",
        );
    }

    fn next_unit_interval(seed: &mut u64) -> f64 {
        // Frozen LCG for reproducibility; not a statistical or cryptographic RNG.
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*seed >> 11) as f64 / 9_007_199_254_740_992.0
    }

    fn sample_between(seed: &mut u64, minimum: f64, maximum: f64) -> f64 {
        minimum + (maximum - minimum) * next_unit_interval(seed)
    }

    #[test]
    fn fixed_seed_parameter_sweep_matches_symmetric_truss_closed_form() {
        // Fixed seed and bounded ranges make every case reproducible. These are
        // numerical coverage ranges for the declared linear-elastic model, not
        // a statistical sample of materials or a real-world validation claim.
        let mut seed = 0x5452_5553_535F_5631_u64;
        const CASES: usize = 128;

        for case_index in 0..CASES {
            let half_span_m = sample_between(&mut seed, 0.25, 12.0);
            let area_m2 = sample_between(&mut seed, 1.0e-4, 5.0e-2);
            let youngs_modulus_pa = sample_between(&mut seed, 30.0e9, 210.0e9);
            let load_n = sample_between(&mut seed, 10.0, 10_000.0);

            let mut input = symmetric_two_bar_truss();
            for node in &mut input.nodes {
                match node.id.as_str() {
                    "left" => {
                        node.x_m = 0.0;
                        node.y_m = 0.0;
                    }
                    "right" => {
                        node.x_m = 2.0 * half_span_m;
                        node.y_m = 0.0;
                    }
                    "apex" => {
                        node.x_m = half_span_m;
                        node.y_m = half_span_m;
                        node.force_y_n = -load_n;
                    }
                    other => panic!("unexpected frozen node ID in case {case_index}: {other}"),
                }
            }
            for bar in &mut input.bars {
                bar.area_m2 = area_m2;
                bar.youngs_modulus_pa = youngs_modulus_pa;
            }

            // Reverse storage order on alternating cases to couple numeric
            // coverage with the solver's promised input-order determinism.
            if case_index % 2 == 1 {
                input.nodes.reverse();
                input.bars.reverse();
            }

            let result = solve(&input)
                .unwrap_or_else(|error| panic!("truss sweep case {case_index} rejected: {error}"));
            let apex = node(&result, "apex");
            let left = node(&result, "left");
            let right = node(&result, "right");
            let expected_apex_y =
                -load_n * (2.0_f64.sqrt() * half_span_m) / (area_m2 * youngs_modulus_pa);
            let expected_bar_force = -load_n / 2.0_f64.sqrt();
            let expected_bar_stress = expected_bar_force / area_m2;
            let expected_bar_strain = expected_bar_stress / youngs_modulus_pa;
            let expected_bar_extension = expected_bar_strain * (2.0_f64.sqrt() * half_span_m);
            let scale_tolerance =
                |expected: f64, relative: f64, floor: f64| (expected.abs() * relative).max(floor);

            close(
                apex.displacement_x_m,
                0.0,
                1.0e-13,
                &format!("case {case_index} apex horizontal symmetry"),
            );
            close(
                apex.displacement_y_m,
                expected_apex_y,
                scale_tolerance(expected_apex_y, 1.0e-10, 1.0e-15),
                &format!("case {case_index} apex vertical displacement"),
            );

            for bar in &result.bar_results {
                close(
                    bar.axial_force_n,
                    expected_bar_force,
                    scale_tolerance(expected_bar_force, 1.0e-10, 1.0e-9),
                    &format!("case {case_index} {} axial force", bar.bar_id),
                );
                close(
                    bar.axial_stress_pa,
                    expected_bar_stress,
                    scale_tolerance(expected_bar_stress, 1.0e-10, 1.0e-6),
                    &format!("case {case_index} {} axial stress", bar.bar_id),
                );
                close(
                    bar.axial_strain,
                    expected_bar_strain,
                    scale_tolerance(expected_bar_strain, 1.0e-10, 1.0e-18),
                    &format!("case {case_index} {} axial strain", bar.bar_id),
                );
                close(
                    bar.axial_extension_m,
                    expected_bar_extension,
                    scale_tolerance(expected_bar_extension, 1.0e-10, 1.0e-15),
                    &format!("case {case_index} {} axial extension", bar.bar_id),
                );
            }

            close(
                left.reaction_x_n.expect("left x support reaction"),
                load_n / 2.0,
                scale_tolerance(load_n / 2.0, 1.0e-10, 1.0e-8),
                &format!("case {case_index} left horizontal reaction"),
            );
            close(
                right.reaction_x_n.expect("right x support reaction"),
                -load_n / 2.0,
                scale_tolerance(load_n / 2.0, 1.0e-10, 1.0e-8),
                &format!("case {case_index} right horizontal reaction"),
            );
            close(
                left.reaction_y_n.expect("left y support reaction"),
                load_n / 2.0,
                scale_tolerance(load_n / 2.0, 1.0e-10, 1.0e-8),
                &format!("case {case_index} left vertical reaction"),
            );
            close(
                right.reaction_y_n.expect("right y support reaction"),
                load_n / 2.0,
                scale_tolerance(load_n / 2.0, 1.0e-10, 1.0e-8),
                &format!("case {case_index} right vertical reaction"),
            );

            assert!(
                result.equilibrium.normalized_force_residual < 1.0e-12,
                "case {case_index} normalized force residual={:.17e}",
                result.equilibrium.normalized_force_residual
            );
            assert!(
                result.equilibrium.normalized_moment_residual < 1.0e-12,
                "case {case_index} normalized moment residual={:.17e}",
                result.equilibrium.normalized_moment_residual
            );
            assert!(
                result.equilibrium.normalized_free_dof_residual < 1.0e-12,
                "case {case_index} normalized free-DOF residual={:.17e}",
                result.equilibrium.normalized_free_dof_residual
            );
            let expected_energy = 0.5 * load_n * expected_apex_y.abs();
            close(
                result.energy.strain_energy_j,
                expected_energy,
                scale_tolerance(expected_energy, 1.0e-10, 1.0e-15),
                &format!("case {case_index} strain energy"),
            );
            close(
                result.energy.external_work_j,
                expected_energy,
                scale_tolerance(expected_energy, 1.0e-10, 1.0e-15),
                &format!("case {case_index} external work"),
            );
            close(
                result.energy.residual_j,
                0.0,
                scale_tolerance(expected_energy, 1.0e-10, 1.0e-15),
                &format!("case {case_index} energy residual"),
            );
            assert!(
                result.energy.normalized_residual < 1.0e-12,
                "case {case_index} normalized energy residual={:.17e}",
                result.energy.normalized_residual
            );
        }
    }

    #[test]
    fn input_order_does_not_change_result_order_or_values() {
        let original = symmetric_two_bar_truss();
        let mut reversed = original.clone();
        reversed.nodes.reverse();
        reversed.bars.reverse();
        let left = solve(&original).expect("original input");
        let right = solve(&reversed).expect("reversed input");
        assert_eq!(left, right);
    }

    #[test]
    fn load_and_stiffness_scaling_obey_metamorphic_relations() {
        let base_input = symmetric_two_bar_truss();
        let base = solve(&base_input).expect("base truss");

        let mut doubled_load = base_input.clone();
        doubled_load
            .nodes
            .iter_mut()
            .find(|n| n.id == "apex")
            .unwrap()
            .force_y_n *= 2.0;
        let result = solve(&doubled_load).expect("doubled load");
        close(
            node(&result, "apex").displacement_y_m / node(&base, "apex").displacement_y_m,
            2.0,
            1.0e-12,
            "load displacement scale",
        );
        close(
            result.bar_results[0].axial_force_n / base.bar_results[0].axial_force_n,
            2.0,
            1.0e-12,
            "load axial force scale",
        );

        for scale in [2.0, 4.0] {
            let mut stiffer = base_input.clone();
            for bar in &mut stiffer.bars {
                bar.youngs_modulus_pa *= scale;
            }
            let result = solve(&stiffer).expect("modulus-scaled truss");
            close(
                node(&result, "apex").displacement_y_m / node(&base, "apex").displacement_y_m,
                1.0 / scale,
                1.0e-12,
                "modulus displacement inverse scale",
            );

            let mut larger_area = base_input.clone();
            for bar in &mut larger_area.bars {
                bar.area_m2 *= scale;
            }
            let result = solve(&larger_area).expect("area-scaled truss");
            close(
                node(&result, "apex").displacement_y_m / node(&base, "apex").displacement_y_m,
                1.0 / scale,
                1.0e-12,
                "area displacement inverse scale",
            );
        }
    }

    #[test]
    fn mechanisms_invalid_geometry_and_duplicate_ids_fail_closed() {
        let mechanism = PlanarTrussInput {
            nodes: vec![
                TrussNode {
                    id: "a".to_owned(),
                    x_m: 0.0,
                    y_m: 0.0,
                    restrained_x: true,
                    restrained_y: true,
                    force_x_n: 0.0,
                    force_y_n: 0.0,
                },
                TrussNode {
                    id: "b".to_owned(),
                    x_m: 1.0,
                    y_m: 0.0,
                    restrained_x: false,
                    restrained_y: false,
                    force_x_n: 0.0,
                    force_y_n: -1.0,
                },
            ],
            bars: vec![TrussBar {
                id: "ab".to_owned(),
                node_i: "a".to_owned(),
                node_j: "b".to_owned(),
                area_m2: 1.0,
                youngs_modulus_pa: 1.0e9,
            }],
        };
        assert!(
            solve(&mechanism)
                .unwrap_err()
                .contains("singular or numerically unresolved")
        );

        let mut invalid = symmetric_two_bar_truss();
        invalid.bars[0].area_m2 = 0.0;
        assert!(solve(&invalid).unwrap_err().contains("bar area"));

        let mut duplicate_node = symmetric_two_bar_truss();
        duplicate_node.nodes[1].id = "left".to_owned();
        assert!(
            solve(&duplicate_node)
                .unwrap_err()
                .contains("duplicate planar truss node ID")
        );

        let mut duplicate_bar = symmetric_two_bar_truss();
        duplicate_bar.bars[1].id = duplicate_bar.bars[0].id.clone();
        assert!(
            solve(&duplicate_bar)
                .unwrap_err()
                .contains("duplicate planar truss bar ID")
        );

        let mut coincident = symmetric_two_bar_truss();
        coincident.nodes[2].x_m = coincident.nodes[0].x_m;
        coincident.nodes[2].y_m = coincident.nodes[0].y_m;
        assert!(solve(&coincident).unwrap_err().contains("bar length"));
    }

    #[test]
    fn disconnected_nodes_and_invalid_ids_fail_closed() {
        let mut input = symmetric_two_bar_truss();
        input.nodes.push(TrussNode {
            id: "isolated".to_owned(),
            x_m: 3.0,
            y_m: 3.0,
            restrained_x: true,
            restrained_y: true,
            force_x_n: 0.0,
            force_y_n: 0.0,
        });
        assert!(solve(&input).unwrap_err().contains("isolated node"));

        let mut invalid_id = symmetric_two_bar_truss();
        invalid_id.nodes[0].id = "bad node".to_owned();
        assert!(solve(&invalid_id).unwrap_err().contains("invalid node ID"));
    }

    #[test]
    fn excessive_or_non_finite_parameters_fail_closed() {
        let mut non_finite = symmetric_two_bar_truss();
        non_finite.nodes[2].force_y_n = f64::INFINITY;
        assert!(solve(&non_finite).unwrap_err().contains("node y load"));

        let mut overflow = symmetric_two_bar_truss();
        for bar in &mut overflow.bars {
            bar.area_m2 = 1.0e308;
            bar.youngs_modulus_pa = 1.0e308;
        }
        assert!(
            solve(&overflow)
                .unwrap_err()
                .contains("bar axial stiffness")
        );
    }

    #[test]
    fn uniform_stiffness_and_load_rescaling_does_not_change_displacement() {
        let base_input = symmetric_two_bar_truss();
        let base = solve(&base_input).expect("base truss");
        let scale = 1.0e-23;
        let mut scaled = base_input;
        scaled
            .nodes
            .iter_mut()
            .find(|n| n.id == "apex")
            .expect("apex node")
            .force_y_n *= scale;
        for bar in &mut scaled.bars {
            bar.youngs_modulus_pa *= scale;
        }
        let result = solve(&scaled).expect("uniformly scaled stiffness and load");
        close(
            node(&result, "apex").displacement_y_m,
            node(&base, "apex").displacement_y_m,
            1.0e-15,
            "unit/scale invariant displacement",
        );
        close(
            result.bar_results[0].axial_force_n / base.bar_results[0].axial_force_n,
            scale,
            scale * 1.0e-12,
            "scaled axial force",
        );
    }
}
