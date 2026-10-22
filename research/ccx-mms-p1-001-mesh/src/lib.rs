//! Deterministic structured P1 triangular mesh and consistent MMS load assembly.
//!
//! This crate generates the unit-square CPE3-style topology and integrates the
//! analytical manufactured body force into consistent nodal loads. It does not
//! write a CalculiX deck, execute a solver, parse solver output, or qualify FEM.

use std::fmt::Write as _;

use ccx_mms_p1_oracle::{ManufacturedProblem, Vec2};

const MAX_RESOLUTION: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    /// Stable one-based node identifier.
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub boundary: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    /// Stable one-based element identifier.
    pub id: usize,
    /// One-based node identifiers in counter-clockwise order.
    pub nodes: [usize; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub cells_per_side: usize,
    pub nodes: Vec<Node>,
    pub triangles: Vec<Triangle>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshError {
    ZeroResolution,
    ResolutionTooLarge,
    InvalidQuadratureOrder,
    InvalidMeshTopology,
    InvalidLoadVectorLength,
}

impl Mesh {
    /// Serialize the mesh canonically. Coordinates are encoded using IEEE-754
    /// bit patterns; the result is deterministic, but is not itself a hash.
    pub fn canonical_text(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "ccx-mms-p1-mesh-v1").expect("String writes cannot fail");
        writeln!(&mut output, "resolution {}", self.cells_per_side)
            .expect("String writes cannot fail");
        writeln!(&mut output, "nodes {}", self.nodes.len())
            .expect("String writes cannot fail");
        for node in &self.nodes {
            writeln!(
                &mut output,
                "N {} {:016x} {:016x} {}",
                node.id,
                node.x.to_bits(),
                node.y.to_bits(),
                u8::from(node.boundary)
            )
            .expect("String writes cannot fail");
        }
        writeln!(&mut output, "triangles {}", self.triangles.len())
            .expect("String writes cannot fail");
        for triangle in &self.triangles {
            writeln!(
                &mut output,
                "T {} {} {} {}",
                triangle.id, triangle.nodes[0], triangle.nodes[1], triangle.nodes[2]
            )
            .expect("String writes cannot fail");
        }
        output
    }

    /// Assemble consistent nodal loads F_i = integral(N_i * f dOmega).
    ///
    /// A tensor-product Gauss-Legendre rule on [0,1]^2 is mapped to the
    /// reference triangle with r=u, s=(1-u)*v. The Duffy Jacobian (1-u) and
    /// the physical triangle determinant are both included.
    pub fn assemble_body_force(
        &self,
        problem: ManufacturedProblem,
        quadrature_order: usize,
    ) -> Result<Vec<Vec2>, MeshError> {
        validate_mesh(self)?;
        let quadrature = gauss_legendre_unit_interval(quadrature_order)?;
        let mut loads = vec![Vec2 { x: 0.0, y: 0.0 }; self.nodes.len()];

        for triangle in &self.triangles {
            let a = self.nodes[triangle.nodes[0] - 1];
            let b = self.nodes[triangle.nodes[1] - 1];
            let c = self.nodes[triangle.nodes[2] - 1];
            let determinant = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            if !determinant.is_finite() || determinant <= 0.0 {
                return Err(MeshError::InvalidMeshTopology);
            }

            for &(u, wu) in &quadrature {
                for &(v, wv) in &quadrature {
                    let r = u;
                    let s = (1.0 - u) * v;
                    let shape = [1.0 - r - s, r, s];
                    let x = a.x + r * (b.x - a.x) + s * (c.x - a.x);
                    let y = a.y + r * (b.y - a.y) + s * (c.y - a.y);
                    let force = problem.body_force_closed_form(x, y);
                    let weight = determinant * (1.0 - u) * wu * wv;

                    for (node_id, shape_value) in triangle.nodes.iter().zip(shape) {
                        let contribution = shape_value * weight;
                        let load = &mut loads[*node_id - 1];
                        load.x += contribution * force.x;
                        load.y += contribution * force.y;
                    }
                }
            }
        }

        if loads
            .iter()
            .any(|load| !load.x.is_finite() || !load.y.is_finite())
        {
            return Err(MeshError::InvalidMeshTopology);
        }
        Ok(loads)
    }

    /// Sum the nodal loads to recover the global body-force resultant.
    pub fn resultant(loads: &[Vec2]) -> Result<Vec2, MeshError> {
        if loads.is_empty() {
            return Err(MeshError::InvalidLoadVectorLength);
        }
        let mut total = Vec2 { x: 0.0, y: 0.0 };
        for load in loads {
            total.x += load.x;
            total.y += load.y;
        }
        if !total.x.is_finite() || !total.y.is_finite() {
            return Err(MeshError::InvalidLoadVectorLength);
        }
        Ok(total)
    }
}

/// Create a uniform n-by-n unit-square mesh, each cell split along its
/// lower-left to upper-right diagonal. Node/element IDs are one-based.
pub fn generate_mesh(cells_per_side: usize) -> Result<Mesh, MeshError> {
    if cells_per_side == 0 {
        return Err(MeshError::ZeroResolution);
    }
    if cells_per_side > MAX_RESOLUTION {
        return Err(MeshError::ResolutionTooLarge);
    }

    let n = cells_per_side;
    let side = n + 1;
    let node_count = side
        .checked_mul(side)
        .ok_or(MeshError::ResolutionTooLarge)?;
    let element_count = n
        .checked_mul(n)
        .and_then(|cells| cells.checked_mul(2))
        .ok_or(MeshError::ResolutionTooLarge)?;

    let mut nodes = Vec::with_capacity(node_count);
    for j in 0..=n {
        for i in 0..=n {
            let id = j * side + i + 1;
            nodes.push(Node {
                id,
                x: i as f64 / n as f64,
                y: j as f64 / n as f64,
                boundary: i == 0 || i == n || j == 0 || j == n,
            });
        }
    }

    let mut triangles = Vec::with_capacity(element_count);
    for j in 0..n {
        for i in 0..n {
            let lower_left = j * side + i + 1;
            let lower_right = lower_left + 1;
            let upper_left = (j + 1) * side + i + 1;
            let upper_right = upper_left + 1;

            let first_id = triangles.len() + 1;
            triangles.push(Triangle {
                id: first_id,
                nodes: [lower_left, lower_right, upper_right],
            });
            triangles.push(Triangle {
                id: first_id + 1,
                nodes: [lower_left, upper_right, upper_left],
            });
        }
    }

    let mesh = Mesh {
        cells_per_side,
        nodes,
        triangles,
    };
    validate_mesh(&mesh)?;
    Ok(mesh)
}

fn validate_mesh(mesh: &Mesh) -> Result<(), MeshError> {
    let n = mesh.cells_per_side;
    if n == 0 || n > MAX_RESOLUTION {
        return Err(MeshError::InvalidMeshTopology);
    }
    let side = n + 1;
    if mesh.nodes.len() != side * side || mesh.triangles.len() != 2 * n * n {
        return Err(MeshError::InvalidMeshTopology);
    }

    for (index, node) in mesh.nodes.iter().enumerate() {
        if node.id != index + 1 || !node.x.is_finite() || !node.y.is_finite() {
            return Err(MeshError::InvalidMeshTopology);
        }
    }

    let mut seen_elements = vec![false; mesh.triangles.len()];
    let mut used_nodes = vec![false; mesh.nodes.len()];
    for element in &mesh.triangles {
        if element.id == 0
            || element.id > seen_elements.len()
            || seen_elements[element.id - 1]
        {
            return Err(MeshError::InvalidMeshTopology);
        }
        seen_elements[element.id - 1] = true;
        if element
            .nodes
            .iter()
            .any(|id| *id == 0 || *id > mesh.nodes.len())
        {
            return Err(MeshError::InvalidMeshTopology);
        }
        let a = mesh.nodes[element.nodes[0] - 1];
        let b = mesh.nodes[element.nodes[1] - 1];
        let c = mesh.nodes[element.nodes[2] - 1];
        let determinant = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        if !determinant.is_finite() || determinant <= 0.0 {
            return Err(MeshError::InvalidMeshTopology);
        }
        for id in element.nodes {
            used_nodes[id - 1] = true;
        }
    }
    if seen_elements.iter().any(|seen| !seen) || used_nodes.iter().any(|used| !used) {
        return Err(MeshError::InvalidMeshTopology);
    }

    for j in 0..=n {
        for i in 0..=n {
            let node = mesh.nodes[j * side + i];
            let expected_boundary = i == 0 || i == n || j == 0 || j == n;
            if node.boundary != expected_boundary {
                return Err(MeshError::InvalidMeshTopology);
            }
        }
    }
    Ok(())
}

/// Gauss-Legendre points and weights mapped to [0,1], sorted by point.
pub fn gauss_legendre_unit_interval(order: usize) -> Result<Vec<(f64, f64)>, MeshError> {
    if order == 0 || order > 64 {
        return Err(MeshError::InvalidQuadratureOrder);
    }
    let mut points = vec![(0.0, 0.0); order];
    let half = order.div_ceil(2);

    for i in 0..half {
        let mut z = (std::f64::consts::PI * (i as f64 + 0.75) / (order as f64 + 0.5)).cos();
        for _ in 0..64 {
            let (polynomial, previous) = legendre_pair(order, z);
            let derivative = order as f64 * (z * polynomial - previous) / (z * z - 1.0);
            let next = z - polynomial / derivative;
            if (next - z).abs() <= 2.0 * f64::EPSILON * next.abs().max(1.0) {
                z = next;
                break;
            }
            z = next;
        }

        let (polynomial, previous) = legendre_pair(order, z);
        let derivative = order as f64 * (z * polynomial - previous) / (z * z - 1.0);
        let weight = 1.0 / ((1.0 - z * z) * derivative * derivative);
        points[i] = ((1.0 - z) * 0.5, weight);
        points[order - 1 - i] = ((1.0 + z) * 0.5, weight);
    }
    Ok(points)
}

fn legendre_pair(order: usize, z: f64) -> (f64, f64) {
    if order == 1 {
        return (z, 1.0);
    }
    let mut previous = 1.0;
    let mut current = z;
    for degree in 2..=order {
        let next = ((2 * degree - 1) as f64 * z * current
            - (degree - 1) as f64 * previous)
            / degree as f64;
        previous = current;
        current = next;
    }
    (current, previous)
}

/// Relative Euclidean difference of two equally sized load vectors.
pub fn relative_load_difference(left: &[Vec2], right: &[Vec2]) -> Result<f64, MeshError> {
    if left.len() != right.len() || left.is_empty() {
        return Err(MeshError::InvalidLoadVectorLength);
    }
    let mut difference_squared = 0.0;
    let mut reference_squared = 0.0;
    for (a, b) in left.iter().zip(right) {
        difference_squared += (a.x - b.x).powi(2) + (a.y - b.y).powi(2);
        reference_squared += b.x.powi(2) + b.y.powi(2);
    }
    if !difference_squared.is_finite() || !reference_squared.is_finite() || reference_squared == 0.0
    {
        return Err(MeshError::InvalidLoadVectorLength);
    }
    Ok(difference_squared.sqrt() / reference_squared.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_family_has_expected_inventory_and_canonical_output() {
        for n in [8, 16, 32, 64] {
            let first = generate_mesh(n).unwrap();
            let second = generate_mesh(n).unwrap();
            assert_eq!(first.nodes.len(), (n + 1) * (n + 1));
            assert_eq!(first.triangles.len(), 2 * n * n);
            assert_eq!(first.canonical_text(), second.canonical_text());
            assert_eq!(first.nodes[0].id, 1);
            assert_eq!(first.nodes.last().unwrap().id, first.nodes.len());
            assert_eq!(first.nodes.iter().filter(|node| node.boundary).count(), 4 * n);
        }
    }

    #[test]
    fn rejects_invalid_mesh_resolutions_and_quadrature_orders() {
        assert_eq!(generate_mesh(0).unwrap_err(), MeshError::ZeroResolution);
        assert_eq!(
            generate_mesh(MAX_RESOLUTION + 1).unwrap_err(),
            MeshError::ResolutionTooLarge
        );
        assert_eq!(
            gauss_legendre_unit_interval(0).unwrap_err(),
            MeshError::InvalidQuadratureOrder
        );
        assert_eq!(
            gauss_legendre_unit_interval(65).unwrap_err(),
            MeshError::InvalidQuadratureOrder
        );
    }

    #[test]
    fn gauss_rule_integrates_polynomials_on_unit_interval() {
        for order in [1, 2, 4, 8, 16, 32, 48] {
            let rule = gauss_legendre_unit_interval(order).unwrap();
            let total_weight: f64 = rule.iter().map(|(_, weight)| weight).sum();
            assert!((total_weight - 1.0).abs() < 2.0e-14, "order={order}");
            for degree in 0..(2 * order).min(12) {
                let estimate: f64 = rule
                    .iter()
                    .map(|(point, weight)| weight * point.powi(degree as i32))
                    .sum();
                let exact = 1.0 / (degree + 1) as f64;
                assert!(
                    (estimate - exact).abs() < 2.0e-13,
                    "order={order}, degree={degree}, estimate={estimate:.17e}"
                );
            }
        }
    }

    #[test]
    fn consistent_loads_match_independent_quadrature_and_resultant() {
        let mesh = generate_mesh(8).unwrap();
        let problem = ManufacturedProblem::reference();
        let loads_32 = mesh.assemble_body_force(problem, 32).unwrap();
        let loads_48 = mesh.assemble_body_force(problem, 48).unwrap();
        let difference = relative_load_difference(&loads_32, &loads_48).unwrap();
        assert!(difference <= 1.0e-12, "relative load difference={difference:e}");

        let total = Mesh::resultant(&loads_48).unwrap();
        let exact = problem.integrated_body_force();
        let normalized_x = (total.x - exact.x).abs() / exact.x.abs().max(1.0);
        let normalized_y =
            total.y.abs() / loads_48.iter().map(|load| load.y.abs()).sum::<f64>().max(1.0);
        assert!(normalized_x <= 1.0e-10, "x resultant residual={normalized_x:e}");
        assert!(normalized_y <= 1.0e-10, "y normalized residual={normalized_y:e}");
    }

    #[test]
    fn one_percent_load_perturbation_is_measurable() {
        let mesh = generate_mesh(8).unwrap();
        let problem = ManufacturedProblem::reference();
        let baseline = mesh.assemble_body_force(problem, 16).unwrap();
        let mut tampered = baseline.clone();
        let first_interior = mesh.nodes.iter().position(|node| !node.boundary).unwrap();
        tampered[first_interior].x *= 1.01;
        let measured = relative_load_difference(&baseline, &tampered).unwrap();
        assert!(measured > 1.0e-5, "tamper signal={measured:e}");
    }

    #[test]
    fn every_boundary_node_lies_on_the_unit_square_boundary() {
        let mesh = generate_mesh(16).unwrap();
        for node in mesh.nodes {
            let on_boundary = node.x == 0.0 || node.x == 1.0 || node.y == 0.0 || node.y == 1.0;
            assert_eq!(node.boundary, on_boundary);
        }
    }
}
