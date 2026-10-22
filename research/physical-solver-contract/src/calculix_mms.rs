// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic manufactured-solution fixture for 2D plane-stress mesh verification.
//!
//! This module defines a smooth exact displacement field, derives its body force
//! analytically, integrates consistent element-equivalent nodal loads, and measures
//! field errors against the exact solution. It does not execute or authenticate CCX.

use crate::Check;
use std::fmt::Write as _;

const PI: f64 = std::f64::consts::PI;
const DEFAULT_QUADRATURE_ORDER: usize = 12;
const MAX_QUADRATURE_ORDER: usize = 32;
const MIN_CELLS: usize = 2;
const MAX_CELLS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MmsParameters {
    pub youngs_modulus_pa: f64,
    pub poisson_ratio: f64,
    pub thickness_m: f64,
    pub amplitude_u_m: f64,
    pub amplitude_v_m: f64,
}

impl Default for MmsParameters {
    fn default() -> Self {
        Self {
            youngs_modulus_pa: 210.0e9,
            poisson_ratio: 0.3,
            thickness_m: 1.0,
            amplitude_u_m: 1.0e-6,
            amplitude_v_m: 0.7e-6,
        }
    }
}

impl MmsParameters {
    pub fn validate(self) -> Check<()> {
        if !self.youngs_modulus_pa.is_finite() || self.youngs_modulus_pa <= 0.0 {
            return Err("youngs_modulus_pa must be finite and positive".to_owned());
        }
        if !self.poisson_ratio.is_finite()
            || self.poisson_ratio <= -1.0
            || self.poisson_ratio >= 0.5
        {
            return Err("poisson_ratio must be finite and in (-1, 0.5)".to_owned());
        }
        if !self.thickness_m.is_finite() || self.thickness_m <= 0.0 {
            return Err("thickness_m must be finite and positive".to_owned());
        }
        if !self.amplitude_u_m.is_finite() || self.amplitude_u_m == 0.0 {
            return Err("amplitude_u_m must be finite and non-zero".to_owned());
        }
        if !self.amplitude_v_m.is_finite() || self.amplitude_v_m == 0.0 {
            return Err("amplitude_v_m must be finite and non-zero".to_owned());
        }
        Ok(())
    }

    fn plane_stress_constants(self) -> (f64, f64) {
        (
            self.youngs_modulus_pa / (1.0 - self.poisson_ratio * self.poisson_ratio),
            self.youngs_modulus_pa / (2.0 * (1.0 + self.poisson_ratio)),
        )
    }
}

/// Exact field values at one point in the unit-square domain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MmsField {
    pub u_m: f64,
    pub v_m: f64,
    pub du_dx: f64,
    pub du_dy: f64,
    pub dv_dx: f64,
    pub dv_dy: f64,
    pub sigma_xx_pa: f64,
    pub sigma_yy_pa: f64,
    pub tau_xy_pa: f64,
    pub body_force_x_n_m3: f64,
    pub body_force_y_n_m3: f64,
}

/// Exact smooth solution:
///
/// u = A sin(pi x) sin(pi y)
/// v = B sin(2 pi x) sin(pi y)
///
/// For plane stress, sigma = D epsilon, with
/// D = E/(1-nu^2) [[1,nu,0],[nu,1,0],[0,0,(1-nu)/2]].
/// The returned body force is -div(sigma), so div(sigma)+b=0.
pub fn manufactured_field(parameters: MmsParameters, x: f64, y: f64) -> Check<MmsField> {
    parameters.validate()?;
    if !x.is_finite() || !y.is_finite() || !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
        return Err("manufactured field coordinates must be finite and in [0,1]".to_owned());
    }

    let a = parameters.amplitude_u_m;
    let b = parameters.amplitude_v_m;
    let p = PI;
    let sx = (p * x).sin();
    let cx = (p * x).cos();
    let s2x = (2.0 * p * x).sin();
    let c2x = (2.0 * p * x).cos();
    let sy = (p * y).sin();
    let cy = (p * y).cos();

    let u_m = a * sx * sy;
    let v_m = b * s2x * sy;
    let du_dx = a * p * cx * sy;
    let du_dy = a * p * sx * cy;
    let dv_dx = 2.0 * b * p * c2x * sy;
    let dv_dy = b * p * s2x * cy;

    let (d, g) = parameters.plane_stress_constants();
    let sigma_xx_pa = d * (du_dx + parameters.poisson_ratio * dv_dy);
    let sigma_yy_pa = d * (parameters.poisson_ratio * du_dx + dv_dy);
    let tau_xy_pa = g * (du_dy + dv_dx);

    let body_force_x_n_m3 = (d + g) * a * p * p * sx * sy
        - 2.0 * b * p * p * (d * parameters.poisson_ratio + g) * c2x * cy;
    let body_force_y_n_m3 = -a * p * p * (g + d * parameters.poisson_ratio) * cx * cy
        + b * p * p * (4.0 * g + d) * s2x * sy;

    let values = [
        u_m,
        v_m,
        du_dx,
        du_dy,
        dv_dx,
        dv_dy,
        sigma_xx_pa,
        sigma_yy_pa,
        tau_xy_pa,
        body_force_x_n_m3,
        body_force_y_n_m3,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err("manufactured field contains a non-finite value".to_owned());
    }

    Ok(MmsField {
        u_m,
        v_m,
        du_dx,
        du_dy,
        dv_dx,
        dv_dy,
        sigma_xx_pa,
        sigma_yy_pa,
        tau_xy_pa,
        body_force_x_n_m3,
        body_force_y_n_m3,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MmsNode {
    pub id: u64,
    pub x_m: f64,
    pub y_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MmsTriangle {
    /// Zero-based indices into MmsMesh.nodes. Deck element node IDs are 1-based.
    pub nodes: [usize; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct MmsMesh {
    pub cells_per_axis: usize,
    pub nodes: Vec<MmsNode>,
    pub elements: Vec<MmsTriangle>,
    /// Consistent body-force loads, summed at nodes, in newtons.
    pub equivalent_nodal_loads_n: Vec<[f64; 2]>,
}

impl MmsMesh {
    pub fn generate(cells_per_axis: usize, parameters: MmsParameters) -> Check<Self> {
        parameters.validate()?;
        if !(MIN_CELLS..=MAX_CELLS).contains(&cells_per_axis) {
            return Err(format!(
                "cells_per_axis must be in [{MIN_CELLS},{MAX_CELLS}]"
            ));
        }

        let width = cells_per_axis
            .checked_add(1)
            .ok_or_else(|| "mesh node dimension overflow".to_owned())?;
        let node_count = width
            .checked_mul(width)
            .ok_or_else(|| "mesh node count overflow".to_owned())?;
        let element_count = cells_per_axis
            .checked_mul(cells_per_axis)
            .and_then(|count| count.checked_mul(2))
            .ok_or_else(|| "mesh element count overflow".to_owned())?;

        let mut nodes = Vec::with_capacity(node_count);
        for j in 0..=cells_per_axis {
            for i in 0..=cells_per_axis {
                let index = j * width + i;
                nodes.push(MmsNode {
                    id: u64::try_from(index + 1)
                        .map_err(|_| "mesh node ID overflow".to_owned())?,
                    x_m: i as f64 / cells_per_axis as f64,
                    y_m: j as f64 / cells_per_axis as f64,
                });
            }
        }

        let mut elements = Vec::with_capacity(element_count);
        for j in 0..cells_per_axis {
            for i in 0..cells_per_axis {
                let bl = j * width + i;
                let br = bl + 1;
                let tl = bl + width;
                let tr = tl + 1;
                // Consistent diagonal and counter-clockwise element orientation.
                elements.push(MmsTriangle { nodes: [bl, br, tr] });
                elements.push(MmsTriangle { nodes: [bl, tr, tl] });
            }
        }

        let quadrature = gauss_legendre_unit(DEFAULT_QUADRATURE_ORDER)?;
        let mut equivalent_nodal_loads_n = vec![[0.0_f64; 2]; node_count];
        for element in &elements {
            let points = element.nodes.map(|index| {
                let node = nodes[index];
                [node.x_m, node.y_m]
            });
            integrate_body_force_on_triangle(
                points,
                parameters,
                &quadrature,
                &mut equivalent_nodal_loads_n,
                element.nodes,
            )?;
        }
        if equivalent_nodal_loads_n.iter().flatten().any(|force| !force.is_finite()) {
            return Err("equivalent nodal load generation produced non-finite forces".to_owned());
        }

        Ok(Self {
            cells_per_axis,
            nodes,
            elements,
            equivalent_nodal_loads_n,
        })
    }

    pub fn generate_input_deck(&self, parameters: MmsParameters) -> Check<Vec<u8>> {
        parameters.validate()?;
        let mut deck = String::new();
        deck.push_str("*HEADING\n");
        deck.push_str("PHYS-MMS-001 smooth plane-stress manufactured solution\n");
        deck.push_str("*NODE\n");
        for node in &self.nodes {
            writeln!(&mut deck, "{},{:.17e},{:.17e},0.0", node.id, node.x_m, node.y_m)
                .map_err(|error| format!("format node failed: {error}"))?;
        }

        deck.push_str("*ELEMENT,TYPE=CPS3,ELSET=SOLID\n");
        for (index, element) in self.elements.iter().enumerate() {
            writeln!(
                &mut deck,
                "{},{},{},{}",
                index + 1,
                self.nodes[element.nodes[0]].id,
                self.nodes[element.nodes[1]].id,
                self.nodes[element.nodes[2]].id
            )
            .map_err(|error| format!("format element failed: {error}"))?;
        }

        append_nset(&mut deck, "ALLNODES", self.nodes.iter().map(|node| node.id))?;
        append_nset(
            &mut deck,
            "BOUNDARY",
            self.nodes
                .iter()
                .filter(|node| {
                    node.x_m == 0.0 || node.x_m == 1.0 || node.y_m == 0.0 || node.y_m == 1.0
                })
                .map(|node| node.id),
        )?;

        deck.push_str("*MATERIAL,NAME=MMSMAT\n");
        deck.push_str("*ELASTIC\n");
        writeln!(
            &mut deck,
            "{:.17e},{:.17e}",
            parameters.youngs_modulus_pa, parameters.poisson_ratio
        )
        .map_err(|error| format!("format elastic parameters failed: {error}"))?;
        deck.push_str("*SOLID SECTION,ELSET=SOLID,MATERIAL=MMSMAT\n");
        writeln!(&mut deck, "{:.17e}", parameters.thickness_m)
            .map_err(|error| format!("format section thickness failed: {error}"))?;

        deck.push_str("*STEP\n*STATIC\n1.,1.,1.E-08,1.\n");
        deck.push_str("*BOUNDARY\nBOUNDARY,1,2,0.\n");
        deck.push_str("*CLOAD\n");
        for (index, force) in self.equivalent_nodal_loads_n.iter().enumerate() {
            let node_id = self.nodes[index].id;
            writeln!(&mut deck, "{node_id},1,{:.17e}", force[0])
                .map_err(|error| format!("format x load failed: {error}"))?;
            writeln!(&mut deck, "{node_id},2,{:.17e}", force[1])
                .map_err(|error| format!("format y load failed: {error}"))?;
        }
        deck.push_str("*NODE PRINT,NSET=ALLNODES\nU\n*END STEP\n");
        Ok(deck.into_bytes())
    }
}

fn append_nset<I>(deck: &mut String, name: &str, ids: I) -> Check<()>
where
    I: IntoIterator<Item = u64>,
{
    writeln!(deck, "*NSET,NSET={name}")
        .map_err(|error| format!("format node set header failed: {error}"))?;
    let mut line_count = 0usize;
    for id in ids {
        if line_count > 0 {
            deck.push(',');
        }
        write!(deck, "{id}").map_err(|error| format!("format node set ID failed: {error}"))?;
        line_count += 1;
        if line_count == 16 {
            deck.push('\n');
            line_count = 0;
        }
    }
    if line_count > 0 {
        deck.push('\n');
    }
    Ok(())
}

fn integrate_body_force_on_triangle(
    points: [[f64; 2]; 3],
    parameters: MmsParameters,
    quadrature: &[(f64, f64)],
    global_loads: &mut [[f64; 2]],
    node_indices: [usize; 3],
) -> Check<()> {
    let determinant = (points[1][0] - points[0][0]) * (points[2][1] - points[0][1])
        - (points[2][0] - points[0][0]) * (points[1][1] - points[0][1]);
    if !determinant.is_finite() || determinant <= 0.0 {
        return Err("MMS triangle must have positive finite orientation".to_owned());
    }

    for &(r, wr) in quadrature {
        for &(s, ws) in quadrature {
            let shape = [1.0 - r, r * (1.0 - s), r * s];
            let mut x = 0.0;
            let mut y = 0.0;
            for local in 0..3 {
                x += shape[local] * points[local][0];
                y += shape[local] * points[local][1];
            }
            let field = manufactured_field(parameters, x, y)?;
            let weight = determinant * r * wr * ws * parameters.thickness_m;
            for local in 0..3 {
                let force = &mut global_loads[node_indices[local]];
                force[0] += weight * shape[local] * field.body_force_x_n_m3;
                force[1] += weight * shape[local] * field.body_force_y_n_m3;
            }
        }
    }
    Ok(())
}

/// Gauss-Legendre nodes and weights on [0,1]. Roots are computed in f64 without
/// external numerical dependencies; the returned weights sum to one.
pub fn gauss_legendre_unit(order: usize) -> Check<Vec<(f64, f64)>> {
    if order == 0 || order > MAX_QUADRATURE_ORDER {
        return Err(format!("quadrature order must be in [1,{MAX_QUADRATURE_ORDER}]"));
    }
    let mut points = Vec::with_capacity(order);
    for i in 0..((order + 1) / 2) {
        let mut z = (PI * (i as f64 + 0.75) / (order as f64 + 0.5)).cos();
        let mut derivative = 0.0;
        for _ in 0..100 {
            let (pn, pnm1) = legendre_pair(order, z);
            derivative = order as f64 * (z * pn - pnm1) / (z * z - 1.0);
            let next = z - pn / derivative;
            if (next - z).abs() <= 4.0 * f64::EPSILON {
                z = next;
                break;
            }
            z = next;
        }
        let (_, pnm1) = legendre_pair(order, z);
        derivative = order as f64 * (z * legendre_pair(order, z).0 - pnm1) / (z * z - 1.0);
        let weight = 1.0 / ((1.0 - z * z) * derivative * derivative);
        let left = (1.0 - z) * 0.5;
        let right = (1.0 + z) * 0.5;
        points.push((left, weight));
        if order - 1 - i != i {
            points.push((right, weight));
        }
    }
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    if points.iter().any(|(x, w)| !x.is_finite() || !w.is_finite() || *w <= 0.0) {
        return Err("Gauss-Legendre quadrature produced invalid nodes or weights".to_owned());
    }
    Ok(points)
}

fn legendre_pair(order: usize, x: f64) -> (f64, f64) {
    let mut pnm1 = 1.0;
    if order == 0 {
        return (1.0, 0.0);
    }
    let mut pn = x;
    if order == 1 {
        return (pn, pnm1);
    }
    for n in 2..=order {
        let pnext = ((2 * n - 1) as f64 * x * pn - (n - 1) as f64 * pnm1) / n as f64;
        pnm1 = pn;
        pn = pnext;
    }
    (pn, pnm1)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MmsMetrics {
    pub cells_per_axis: usize,
    pub node_count: usize,
    pub element_count: usize,
    pub displacement_l1_error_m: f64,
    pub displacement_l2_error_m: f64,
    /// Quadrature-sampled maximum, not a proof of the true continuous L-infinity norm.
    pub displacement_quadrature_linf_error_m: f64,
    pub displacement_max_nodal_error_m: f64,
    pub displacement_h1_seminorm_error_m: f64,
    /// L2 norm of the stress-tensor error with both shear entries counted.
    pub stress_l2_error_pa_m: f64,
}

pub fn audit_nodal_displacements(
    mesh: &MmsMesh,
    parameters: MmsParameters,
    displacements_m: &[[f64; 2]],
) -> Check<MmsMetrics> {
    parameters.validate()?;
    if displacements_m.len() != mesh.nodes.len() {
        return Err(format!(
            "displacement node count {} does not match mesh node count {}",
            displacements_m.len(),
            mesh.nodes.len()
        ));
    }
    if displacements_m.iter().flatten().any(|value| !value.is_finite()) {
        return Err("displacement field contains non-finite values".to_owned());
    }

    let mut displacement_max_nodal_error_m = 0.0_f64;
    for (node, observed) in mesh.nodes.iter().zip(displacements_m) {
        let exact = manufactured_field(parameters, node.x_m, node.y_m)?;
        let error = ((observed[0] - exact.u_m).powi(2) + (observed[1] - exact.v_m).powi(2)).sqrt();
        displacement_max_nodal_error_m = displacement_max_nodal_error_m.max(error);
    }

    let (d, g) = parameters.plane_stress_constants();
    let quadrature = gauss_legendre_unit(DEFAULT_QUADRATURE_ORDER)?;
    let mut l1 = 0.0_f64;
    let mut l2_squared = 0.0_f64;
    let mut linf = displacement_max_nodal_error_m;
    let mut h1_squared = 0.0_f64;
    let mut stress_squared = 0.0_f64;

    for element in &mesh.elements {
        let nodes = element.nodes.map(|index| mesh.nodes[index]);
        let disp = element.nodes.map(|index| displacements_m[index]);
        let det = (nodes[1].x_m - nodes[0].x_m) * (nodes[2].y_m - nodes[0].y_m)
            - (nodes[2].x_m - nodes[0].x_m) * (nodes[1].y_m - nodes[0].y_m);
        if !det.is_finite() || det <= 0.0 {
            return Err("MMS audit encountered an invalid element orientation".to_owned());
        }

        let dndx = [
            (nodes[1].y_m - nodes[2].y_m) / det,
            (nodes[2].y_m - nodes[0].y_m) / det,
            (nodes[0].y_m - nodes[1].y_m) / det,
        ];
        let dndy = [
            (nodes[2].x_m - nodes[1].x_m) / det,
            (nodes[0].x_m - nodes[2].x_m) / det,
            (nodes[1].x_m - nodes[0].x_m) / det,
        ];
        let mut uh_dx = 0.0;
        let mut uh_dy = 0.0;
        let mut vh_dx = 0.0;
        let mut vh_dy = 0.0;
        for local in 0..3 {
            uh_dx += dndx[local] * disp[local][0];
            uh_dy += dndy[local] * disp[local][0];
            vh_dx += dndx[local] * disp[local][1];
            vh_dy += dndy[local] * disp[local][1];
        }
        let sigma_h_xx = d * (uh_dx + parameters.poisson_ratio * vh_dy);
        let sigma_h_yy = d * (parameters.poisson_ratio * uh_dx + vh_dy);
        let tau_h_xy = g * (uh_dy + vh_dx);

        for &(r, wr) in &quadrature {
            for &(s, ws) in &quadrature {
                let shape = [1.0 - r, r * (1.0 - s), r * s];
                let mut x = 0.0;
                let mut y = 0.0;
                let mut uh = [0.0, 0.0];
                for local in 0..3 {
                    x += shape[local] * nodes[local].x_m;
                    y += shape[local] * nodes[local].y_m;
                    uh[0] += shape[local] * disp[local][0];
                    uh[1] += shape[local] * disp[local][1];
                }
                let exact = manufactured_field(parameters, x, y)?;
                let eu = uh[0] - exact.u_m;
                let ev = uh[1] - exact.v_m;
                let displacement_error = (eu * eu + ev * ev).sqrt();
                let weight = det * r * wr * ws;
                l1 += weight * displacement_error;
                l2_squared += weight * displacement_error * displacement_error;
                linf = linf.max(displacement_error);

                let e_ux = uh_dx - exact.du_dx;
                let e_uy = uh_dy - exact.du_dy;
                let e_vx = vh_dx - exact.dv_dx;
                let e_vy = vh_dy - exact.dv_dy;
                h1_squared += weight * (e_ux * e_ux + e_uy * e_uy + e_vx * e_vx + e_vy * e_vy);

                let e_sxx = sigma_h_xx - exact.sigma_xx_pa;
                let e_syy = sigma_h_yy - exact.sigma_yy_pa;
                let e_txy = tau_h_xy - exact.tau_xy_pa;
                stress_squared += weight * (e_sxx * e_sxx + e_syy * e_syy + 2.0 * e_txy * e_txy);
            }
        }
    }

    let result = MmsMetrics {
        cells_per_axis: mesh.cells_per_axis,
        node_count: mesh.nodes.len(),
        element_count: mesh.elements.len(),
        displacement_l1_error_m: l1,
        displacement_l2_error_m: l2_squared.sqrt(),
        displacement_quadrature_linf_error_m: linf,
        displacement_max_nodal_error_m,
        displacement_h1_seminorm_error_m: h1_squared.sqrt(),
        stress_l2_error_pa_m: stress_squared.sqrt(),
    };
    let values = [
        result.displacement_l1_error_m,
        result.displacement_l2_error_m,
        result.displacement_quadrature_linf_error_m,
        result.displacement_max_nodal_error_m,
        result.displacement_h1_seminorm_error_m,
        result.stress_l2_error_pa_m,
    ];
    if values.iter().any(|value| !value.is_finite() || *value < 0.0) {
        return Err("MMS error metrics contain invalid values".to_owned());
    }
    Ok(result)
}

#[derive(Clone, Debug, PartialEq)]
pub struct MmsDisplacementOutput {
    pub terminal_time_s: f64,
    pub displacements_m: Vec<[f64; 2]>,
}

/// Parse one exact *NODE PRINT U section produced for NSET=ALLNODES at time 1.
/// Duplicate/missing node IDs, non-finite values, wrong sets, or wrong time fail closed.
pub fn parse_mms_output(dat: &str, expected_node_count: usize) -> Check<MmsDisplacementOutput> {
    if expected_node_count == 0 || expected_node_count > 100_000 {
        return Err("expected node count must be in [1,100000]".to_owned());
    }
    let lines: Vec<&str> = dat.lines().collect();
    let headers: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            line.trim()
                .to_ascii_lowercase()
                .starts_with("displacements (")
                .then_some(index)
        })
        .collect();
    if headers.len() != 1 {
        return Err(format!("expected exactly one U output section, found {}", headers.len()));
    }
    let header_index = headers[0];
    let header = lines[header_index].trim().to_ascii_lowercase();
    let marker = "for set ";
    let marker_index = header
        .find(marker)
        .ok_or_else(|| "U output header is missing its node set".to_owned())?;
    let rest = &header[marker_index + marker.len()..];
    let time_marker = " and time ";
    let time_index = rest
        .find(time_marker)
        .ok_or_else(|| "U output header is missing its output time".to_owned())?;
    if rest[..time_index].trim() != "allnodes" {
        return Err("U output set does not match ALLNODES".to_owned());
    }
    let observed_time = parse_finite_number(rest[time_index + time_marker.len()..].trim())?;
    if (observed_time - 1.0).abs() > 1.0e-8 {
        return Err(format!("U output time {observed_time} does not match 1.0"));
    }

    let mut values = vec![None::<[f64; 2]>; expected_node_count];
    let mut saw_data = false;
    for (index, line) in lines.iter().enumerate().skip(header_index + 1) {
        let text = line.trim();
        if text.is_empty() {
            if saw_data {
                break;
            }
            continue;
        }
        let fields: Vec<&str> = text.split_whitespace().collect();
        if fields.is_empty() {
            continue;
        }
        let node_id = match fields[0].parse::<usize>() {
            Ok(value) => value,
            Err(_) => {
                if saw_data {
                    break;
                }
                if text.chars().all(|ch| ch == '-' || ch == '=' || ch.is_whitespace()) {
                    continue;
                }
                return Err(format!("unexpected line in U section at dat line {}", index + 1));
            }
        };
        if node_id == 0 || node_id > expected_node_count {
            return Err(format!("U section has out-of-range node ID {node_id}"));
        }
        if fields.len() != 3 && fields.len() != 4 {
            return Err(format!("U row for node {node_id} must contain 2 or 3 displacement components"));
        }
        let ux = parse_finite_number(fields[1])?;
        let uy = parse_finite_number(fields[2])?;
        if fields.len() == 4 {
            let uz = parse_finite_number(fields[3])?;
            let z_tolerance = 1.0e-10
                * parameters_amplitude_scale_from_expected_node_count(expected_node_count);
            if uz.abs() > z_tolerance {
                return Err(format!("plane-stress output has nonzero out-of-plane displacement at node {node_id}"));
            }
        }
        if values[node_id - 1].replace([ux, uy]).is_some() {
            return Err(format!("duplicate U row for node {node_id}"));
        }
        saw_data = true;
    }
    if values.iter().any(Option::is_none) {
        return Err("U output section is missing one or more mesh nodes".to_owned());
    }
    Ok(MmsDisplacementOutput {
        terminal_time_s: observed_time,
        displacements_m: values.into_iter().map(Option::unwrap).collect(),
    })
}

pub fn parse_mms_displacements(dat: &str, expected_node_count: usize) -> Check<Vec<[f64; 2]>> {
    Ok(parse_mms_output(dat, expected_node_count)?.displacements_m)
}

fn parameters_amplitude_scale_from_expected_node_count(_node_count: usize) -> f64 {
    // The exact amplitudes are O(1e-6 m) in this pinned fixture. Kept local to the
    // parser to avoid accepting arbitrary out-of-plane motion in a plane-stress run.
    1.0e-6
}

fn parse_finite_number(text: &str) -> Check<f64> {
    let normalized = text.trim().replace('D', "E").replace('d', "E");
    let value = normalized
        .parse::<f64>()
        .map_err(|_| format!("invalid numeric value '{text}'"))?;
    if !value.is_finite() {
        return Err("non-finite numeric value in output".to_owned());
    }
    Ok(value)
}

pub fn observed_order(coarse_error: f64, fine_error: f64) -> Check<f64> {
    if !coarse_error.is_finite()
        || !fine_error.is_finite()
        || coarse_error <= 0.0
        || fine_error <= 0.0
    {
        return Err("observed-order errors must be finite and positive".to_owned());
    }
    let order = (coarse_error / fine_error).ln() / 2.0_f64.ln();
    if !order.is_finite() {
        return Err("observed order is non-finite".to_owned());
    }
    Ok(order)
}

fn gauss_legendre_weight_sum(order: usize) -> Check<f64> {
    Ok(gauss_legendre_unit(order)?.iter().map(|(_, weight)| weight).sum())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact_nodal_solution(mesh: &MmsMesh, parameters: MmsParameters) -> Vec<[f64; 2]> {
        mesh.nodes
            .iter()
            .map(|node| {
                let field = manufactured_field(parameters, node.x_m, node.y_m).unwrap();
                [field.u_m, field.v_m]
            })
            .collect()
    }

    #[test]
    fn gauss_legendre_weights_integrate_polynomials_and_sum_to_one() {
        for order in [1, 2, 3, 4, 8, 12, 16] {
            assert!((gauss_legendre_weight_sum(order).unwrap() - 1.0).abs() < 2e-14);
            let points = gauss_legendre_unit(order).unwrap();
            let degree = (2 * order - 1).min(20);
            let exact = 1.0 / (degree as f64 + 1.0);
            let numerical = points
                .iter()
                .map(|(x, weight)| x.powi(degree as i32) * weight)
                .sum::<f64>();
            assert!((numerical - exact).abs() < 3e-13, "order={order}, degree={degree}");
        }
    }

    #[test]
    fn exact_field_is_zero_on_all_edges_and_finite_inside() {
        let parameters = MmsParameters::default();
        for index in 0..=16 {
            let coordinate = index as f64 / 16.0;
            for (x, y) in [(0.0, coordinate), (1.0, coordinate), (coordinate, 0.0), (coordinate, 1.0)] {
                let field = manufactured_field(parameters, x, y).unwrap();
                assert!(field.u_m.abs() < 1.0e-20);
                assert!(field.v_m.abs() < 1.0e-20);
            }
        }
        let center = manufactured_field(parameters, 0.371, 0.419).unwrap();
        assert!(center.sigma_xx_pa.is_finite());
        assert!(center.body_force_x_n_m3.is_finite());
        assert!(center.body_force_y_n_m3.is_finite());
    }

    #[test]
    fn body_force_is_negative_divergence_of_analytic_stress() {
        let parameters = MmsParameters::default();
        let h = 1.0e-5;
        for &(x, y) in &[(0.13, 0.29), (0.37, 0.61), (0.73, 0.43)] {
            let xp = manufactured_field(parameters, x + h, y).unwrap();
            let xm = manufactured_field(parameters, x - h, y).unwrap();
            let yp = manufactured_field(parameters, x, y + h).unwrap();
            let ym = manufactured_field(parameters, x, y - h).unwrap();
            let center = manufactured_field(parameters, x, y).unwrap();
            let div_x =
                (xp.sigma_xx_pa - xm.sigma_xx_pa) / (2.0 * h)
                    + (yp.tau_xy_pa - ym.tau_xy_pa) / (2.0 * h);
            let div_y =
                (xp.tau_xy_pa - xm.tau_xy_pa) / (2.0 * h)
                    + (yp.sigma_yy_pa - ym.sigma_yy_pa) / (2.0 * h);
            let force_scale = parameters.youngs_modulus_pa
                * parameters.amplitude_u_m.max(parameters.amplitude_v_m)
                * PI
                * PI;
            assert!(
                (div_x + center.body_force_x_n_m3).abs() < 2.0e-7 * force_scale,
                "x residual={}",
                div_x + center.body_force_x_n_m3
            );
            assert!(
                (div_y + center.body_force_y_n_m3).abs() < 2.0e-7 * force_scale,
                "y residual={}",
                div_y + center.body_force_y_n_m3
            );
        }
    }

    #[test]
    fn mesh_loads_and_deck_are_deterministic_and_topologically_exact() {
        let parameters = MmsParameters::default();
        let a = MmsMesh::generate(4, parameters).unwrap();
        let b = MmsMesh::generate(4, parameters).unwrap();
        assert_eq!(a.nodes.len(), 25);
        assert_eq!(a.elements.len(), 32);
        assert_eq!(a.equivalent_nodal_loads_n, b.equivalent_nodal_loads_n);
        assert_eq!(
            a.generate_input_deck(parameters).unwrap(),
            b.generate_input_deck(parameters).unwrap()
        );
        let deck = String::from_utf8(a.generate_input_deck(parameters).unwrap()).unwrap();
        assert!(deck.contains("*ELEMENT,TYPE=CPS3,ELSET=SOLID"));
        assert!(deck.contains("*NODE PRINT,NSET=ALLNODES\nU\n*END STEP"));
        assert!(a.equivalent_nodal_loads_n.iter().flatten().all(|v| v.is_finite()));
    }

    #[test]
    fn linear_interpolation_error_exhibits_expected_refinement_trend() {
        let parameters = MmsParameters::default();
        let mut metrics = Vec::new();
        for n in [4, 8, 16] {
            let mesh = MmsMesh::generate(n, parameters).unwrap();
            let solution = exact_nodal_solution(&mesh, parameters);
            metrics.push(audit_nodal_displacements(&mesh, parameters, &solution).unwrap());
        }
        assert!(metrics[1].displacement_l2_error_m < metrics[0].displacement_l2_error_m);
        assert!(metrics[2].displacement_l2_error_m < metrics[1].displacement_l2_error_m);
        let order = observed_order(metrics[1].displacement_l2_error_m, metrics[2].displacement_l2_error_m).unwrap();
        assert!((1.7..=2.4).contains(&order), "observed interpolation order={order}");
        assert!(metrics[2].displacement_h1_seminorm_error_m < metrics[1].displacement_h1_seminorm_error_m);
    }

    #[test]
    fn malformed_duplicate_missing_or_wrong_time_outputs_fail_closed() {
        assert!(parse_mms_displacements("", 1).is_err());
        let duplicate = "displacements (vx,vy,vz) for set ALLNODES and time 1.0\n1 0 0 0\n1 0 0 0\n\n";
        assert!(parse_mms_displacements(duplicate, 1).is_err());
        let missing = "displacements (vx,vy,vz) for set ALLNODES and time 1.0\n1 0 0 0\n\n";
        assert!(parse_mms_displacements(missing, 2).is_err());
        let wrong_time = "displacements (vx,vy,vz) for set ALLNODES and time 0.5\n1 0 0 0\n\n";
        assert!(parse_mms_displacements(wrong_time, 1).is_err());
        let nonfinite = "displacements (vx,vy,vz) for set ALLNODES and time 1.0\n1 NaN 0 0\n\n";
        assert!(parse_mms_displacements(nonfinite, 1).is_err());
    }
}
