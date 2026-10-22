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

#[derive(Clone, Debug, PartialEq)]
pub struct BodyForceAssembly {
    pub nodal_loads: Vec<Vec2>,
    pub integrated_absolute_force: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErrorNorms {
    pub displacement_l2_absolute: f64,
    pub displacement_l2_relative: f64,
    pub energy_error_absolute: f64,
    pub energy_error_relative: f64,
    pub stress_l2_relative: f64,
    /// L2 norm of the FE volumetric strain div(u_h).
    pub fem_volumetric_strain_l2_absolute: f64,
    /// L2 norm of div(u_h)-div(u_exact), useful when the exact field is divergence-free.
    pub volumetric_strain_error_l2_absolute: f64,
    pub exact_volumetric_strain_l2_norm: f64,
    pub exact_displacement_l2_norm: f64,
    pub exact_energy_norm: f64,
    pub exact_stress_l2_norm: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshError {
    ZeroResolution,
    ResolutionTooLarge,
    InvalidQuadratureOrder,
    InvalidMeshTopology,
    InvalidLoadVectorLength,
    InvalidDisplacementVectorLength,
    NonFiniteDisplacement,
    InvalidReferenceNorm,
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

    /// Assemble nodal loads and quadrature-scaled absolute force integrals.
    ///
    /// A tensor-product Gauss-Legendre rule on [0,1]^2 is mapped to the
    /// reference triangle with r=u, s=(1-u)*v. The Duffy Jacobian (1-u) and
    /// the physical triangle determinant are both included.
    pub fn assemble_body_force_with_diagnostics(
        &self,
        problem: ManufacturedProblem,
        quadrature_order: usize,
    ) -> Result<BodyForceAssembly, MeshError> {
        validate_mesh(self)?;
        let quadrature = gauss_legendre_unit_interval(quadrature_order)?;
        let mut loads = vec![Vec2 { x: 0.0, y: 0.0 }; self.nodes.len()];
        let mut integrated_absolute_force = Vec2 { x: 0.0, y: 0.0 };

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
                    integrated_absolute_force.x += weight * force.x.abs();
                    integrated_absolute_force.y += weight * force.y.abs();

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
        if !integrated_absolute_force.x.is_finite()
            || !integrated_absolute_force.y.is_finite()
            || integrated_absolute_force.x < 0.0
            || integrated_absolute_force.y < 0.0
        {
            return Err(MeshError::InvalidLoadVectorLength);
        }
        Ok(BodyForceAssembly {
            nodal_loads: loads,
            integrated_absolute_force,
        })
    }

    /// Assemble only the complete nodal load vector.
    pub fn assemble_body_force(
        &self,
        problem: ManufacturedProblem,
        quadrature_order: usize,
    ) -> Result<Vec<Vec2>, MeshError> {
        self.assemble_body_force_with_diagnostics(problem, quadrature_order)
            .map(|assembly| assembly.nodal_loads)
    }

    /// Evaluate exact-continuum errors of a P1 finite-element field.
    ///
    /// This is a postprocessor, not a solver. The nodal displacement slice must
    /// use the same node order as this mesh. FE strain is constant per triangle;
    /// exact displacement, strain, and stress are integrated by Duffy/Gauss.
    /// Plane-strain energy density is lambda*trace(epsilon)^2 plus
    /// 2*mu*(epsilon:epsilon).
    pub fn compute_error_norms(
        &self,
        problem: ManufacturedProblem,
        nodal_displacements: &[Vec2],
        quadrature_order: usize,
    ) -> Result<ErrorNorms, MeshError> {
        validate_mesh(self)?;
        if nodal_displacements.len() != self.nodes.len() {
            return Err(MeshError::InvalidDisplacementVectorLength);
        }
        if nodal_displacements
            .iter()
            .any(|u| !u.x.is_finite() || !u.y.is_finite())
        {
            return Err(MeshError::NonFiniteDisplacement);
        }

        let quadrature = gauss_legendre_unit_interval(quadrature_order)?;
        let (lambda, mu) = problem.material().lame_parameters();
        let mut displacement_error_squared = 0.0;
        let mut exact_displacement_squared = 0.0;
        let mut energy_error_squared = 0.0;
        let mut exact_energy_squared = 0.0;
        let mut stress_error_squared = 0.0;
        let mut exact_stress_squared = 0.0;
        let mut fem_volumetric_strain_squared = 0.0;
        let mut volumetric_strain_error_squared = 0.0;
        let mut exact_volumetric_strain_squared = 0.0;

        for triangle in &self.triangles {
            let a = self.nodes[triangle.nodes[0] - 1];
            let b = self.nodes[triangle.nodes[1] - 1];
            let c = self.nodes[triangle.nodes[2] - 1];
            let nodal = [
                nodal_displacements[triangle.nodes[0] - 1],
                nodal_displacements[triangle.nodes[1] - 1],
                nodal_displacements[triangle.nodes[2] - 1],
            ];
            let determinant = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            if !determinant.is_finite() || determinant <= 0.0 {
                return Err(MeshError::InvalidMeshTopology);
            }

            let gradients = [
                [(b.y - c.y) / determinant, (c.x - b.x) / determinant],
                [(c.y - a.y) / determinant, (a.x - c.x) / determinant],
                [(a.y - b.y) / determinant, (b.x - a.x) / determinant],
            ];
            let mut dux_dx = 0.0;
            let mut dux_dy = 0.0;
            let mut duy_dx = 0.0;
            let mut duy_dy = 0.0;
            for (displacement, gradient) in nodal.iter().zip(gradients) {
                dux_dx += displacement.x * gradient[0];
                dux_dy += displacement.x * gradient[1];
                duy_dx += displacement.y * gradient[0];
                duy_dy += displacement.y * gradient[1];
            }
            let fem_strain = ccx_mms_p1_oracle::Strain2 {
                xx: dux_dx,
                yy: duy_dy,
                xy: 0.5 * (dux_dy + duy_dx),
            };

            for &(u, wu) in &quadrature {
                for &(v, wv) in &quadrature {
                    let r = u;
                    let s = (1.0 - u) * v;
                    let shape = [1.0 - r - s, r, s];
                    let x = a.x + r * (b.x - a.x) + s * (c.x - a.x);
                    let y = a.y + r * (b.y - a.y) + s * (c.y - a.y);
                    let fem_displacement = Vec2 {
                        x: shape[0] * nodal[0].x
                            + shape[1] * nodal[1].x
                            + shape[2] * nodal[2].x,
                        y: shape[0] * nodal[0].y
                            + shape[1] * nodal[1].y
                            + shape[2] * nodal[2].y,
                    };
                    let exact_displacement = problem.displacement(x, y);
                    let exact_strain = problem.strain(x, y);
                    let strain_error = ccx_mms_p1_oracle::Strain2 {
                        xx: fem_strain.xx - exact_strain.xx,
                        yy: fem_strain.yy - exact_strain.yy,
                        xy: fem_strain.xy - exact_strain.xy,
                    };
                    let weight = determinant * (1.0 - u) * wu * wv;

                    displacement_error_squared += weight
                        * ((fem_displacement.x - exact_displacement.x).powi(2)
                            + (fem_displacement.y - exact_displacement.y).powi(2));
                    exact_displacement_squared += weight
                        * (exact_displacement.x.powi(2) + exact_displacement.y.powi(2));
                    energy_error_squared += weight * energy_density(strain_error, lambda, mu);
                    exact_energy_squared += weight * energy_density(exact_strain, lambda, mu);

                    let fem_trace = fem_strain.xx + fem_strain.yy;
                    let exact_trace = exact_strain.xx + exact_strain.yy;
                    fem_volumetric_strain_squared += weight * fem_trace.powi(2);
                    volumetric_strain_error_squared +=
                        weight * (fem_trace - exact_trace).powi(2);
                    exact_volumetric_strain_squared += weight * exact_trace.powi(2);
                    let fem_stress = [
                        lambda * fem_trace + 2.0 * mu * fem_strain.xx,
                        lambda * fem_trace + 2.0 * mu * fem_strain.yy,
                        lambda * fem_trace,
                        2.0 * mu * fem_strain.xy,
                    ];
                    let exact_stress = problem.stress(x, y);
                    let exact_stress_components = [
                        exact_stress.xx,
                        exact_stress.yy,
                        exact_stress.zz,
                        exact_stress.xy,
                    ];
                    stress_error_squared += weight
                        * ((fem_stress[0] - exact_stress_components[0]).powi(2)
                            + (fem_stress[1] - exact_stress_components[1]).powi(2)
                            + (fem_stress[2] - exact_stress_components[2]).powi(2)
                            + 2.0 * (fem_stress[3] - exact_stress_components[3]).powi(2));
                    exact_stress_squared += weight
                        * (exact_stress_components[0].powi(2)
                            + exact_stress_components[1].powi(2)
                            + exact_stress_components[2].powi(2)
                            + 2.0 * exact_stress_components[3].powi(2));
                }
            }
        }

        let exact_displacement_norm = exact_displacement_squared.sqrt();
        let exact_energy_norm = exact_energy_squared.sqrt();
        let exact_stress_norm = exact_stress_squared.sqrt();
        if !exact_displacement_norm.is_finite()
            || !exact_energy_norm.is_finite()
            || !exact_stress_norm.is_finite()
            || exact_displacement_norm <= 0.0
            || exact_energy_norm <= 0.0
            || exact_stress_norm <= 0.0
            || !displacement_error_squared.is_finite()
            || !energy_error_squared.is_finite()
            || !stress_error_squared.is_finite()
            || !fem_volumetric_strain_squared.is_finite()
            || !volumetric_strain_error_squared.is_finite()
            || !exact_volumetric_strain_squared.is_finite()
            || energy_error_squared < 0.0
            || stress_error_squared < 0.0
            || fem_volumetric_strain_squared < 0.0
            || volumetric_strain_error_squared < 0.0
            || exact_volumetric_strain_squared < 0.0
        {
            return Err(MeshError::InvalidReferenceNorm);
        }

        let displacement_l2_absolute = displacement_error_squared.sqrt();
        let energy_error_absolute = energy_error_squared.sqrt();
        Ok(ErrorNorms {
            displacement_l2_absolute,
            displacement_l2_relative: displacement_l2_absolute / exact_displacement_norm,
            energy_error_absolute,
            energy_error_relative: energy_error_absolute / exact_energy_norm,
            stress_l2_relative: stress_error_squared.sqrt() / exact_stress_norm,
            fem_volumetric_strain_l2_absolute: fem_volumetric_strain_squared.sqrt(),
            volumetric_strain_error_l2_absolute: volumetric_strain_error_squared.sqrt(),
            exact_volumetric_strain_l2_norm: exact_volumetric_strain_squared.sqrt(),
            exact_displacement_l2_norm: exact_displacement_norm,
            exact_energy_norm,
            exact_stress_l2_norm: exact_stress_norm,
        })
    }


    /// Render a deterministic CalculiX input deck for the frozen MMS problem.
    ///
    /// The complete consistent nodal load vector is computed from the chosen
    /// quadrature. Loads on boundary nodes are preserved in explicit deck
    /// comments but excluded from the CLOAD section because those DOFs are
    /// constrained. The postprocessor must restore those omitted loads when
    /// correcting the reported support reactions; this convention does not
    /// change free-DOF equilibrium or the computed displacement field.
    pub fn to_calculix_input_deck(
        &self,
        problem: ManufacturedProblem,
        quadrature_order: usize,
    ) -> Result<String, MeshError> {
        validate_mesh(self)?;
        let loads = self.assemble_body_force(problem, quadrature_order)?;
        if loads.len() != self.nodes.len() {
            return Err(MeshError::InvalidLoadVectorLength);
        }
        let material = problem.material();
        let mut deck = String::new();

        writeln!(
            &mut deck,
            "** PHYS-CCX-MMS-P1-001; protocol=0.3.0; mesh_n={}",
            self.cells_per_side
        )
        .expect("String writes cannot fail");
        writeln!(&mut deck, "*HEADING").expect("String writes cannot fail");
        writeln!(
            &mut deck,
            "PHYS-CCX-MMS-P1-001 manufactured plane-strain field"
        )
        .expect("String writes cannot fail");

        writeln!(&mut deck, "*NODE, NSET=NALL").expect("String writes cannot fail");
        for node in &self.nodes {
            writeln!(
                &mut deck,
                "{}, {:.17e}, {:.17e}, 0.00000000000000000e+00",
                node.id, node.x, node.y
            )
            .expect("String writes cannot fail");
        }

        writeln!(&mut deck, "*ELEMENT, TYPE=CPE3, ELSET=EALL")
            .expect("String writes cannot fail");
        for triangle in &self.triangles {
            writeln!(
                &mut deck,
                "{}, {}, {}, {}",
                triangle.id, triangle.nodes[0], triangle.nodes[1], triangle.nodes[2]
            )
            .expect("String writes cannot fail");
        }

        writeln!(&mut deck, "*NSET, NSET=BOUNDARY").expect("String writes cannot fail");
        write_node_set(
            &mut deck,
            self.nodes.iter().filter(|node| node.boundary).map(|node| node.id),
        );

        writeln!(&mut deck, "*MATERIAL, NAME=MMSMAT").expect("String writes cannot fail");
        writeln!(&mut deck, "*ELASTIC").expect("String writes cannot fail");
        writeln!(
            &mut deck,
            "{:.17e}, {:.17e}",
            material.youngs_modulus(),
            material.poisson_ratio()
        )
        .expect("String writes cannot fail");
        writeln!(&mut deck, "*SOLID SECTION, ELSET=EALL, MATERIAL=MMSMAT")
            .expect("String writes cannot fail");
        writeln!(&mut deck, "1.00000000000000000e+00")
            .expect("String writes cannot fail");

        writeln!(&mut deck, "** Boundary-equivalent loads omitted from CLOAD on constrained DOFs.")
            .expect("String writes cannot fail");
        writeln!(
            &mut deck,
            "** Recover full support reaction by subtracting these loads from solver RF."
        )
        .expect("String writes cannot fail");
        writeln!(&mut deck, "** BOUNDARY_LOAD columns: node_id, fx, fy")
            .expect("String writes cannot fail");
        for node in self.nodes.iter().filter(|node| node.boundary) {
            let load = loads[node.id - 1];
            writeln!(
                &mut deck,
                "** BOUNDARY_LOAD, {}, {:.17e}, {:.17e}",
                node.id, load.x, load.y
            )
            .expect("String writes cannot fail");
        }

        writeln!(&mut deck, "*STEP").expect("String writes cannot fail");
        writeln!(&mut deck, "*STATIC").expect("String writes cannot fail");
        writeln!(&mut deck, "1.00000000000000000e+00, 1.00000000000000000e+00")
            .expect("String writes cannot fail");
        writeln!(&mut deck, "*BOUNDARY").expect("String writes cannot fail");
        writeln!(&mut deck, "BOUNDARY, 1, 2, 0.00000000000000000e+00")
            .expect("String writes cannot fail");

        writeln!(&mut deck, "*CLOAD").expect("String writes cannot fail");
        for node in self.nodes.iter().filter(|node| !node.boundary) {
            let load = loads[node.id - 1];
            if load.x != 0.0 {
                writeln!(&mut deck, "{}, 1, {:.17e}", node.id, load.x)
                    .expect("String writes cannot fail");
            }
            if load.y != 0.0 {
                writeln!(&mut deck, "{}, 2, {:.17e}", node.id, load.y)
                    .expect("String writes cannot fail");
            }
        }

        writeln!(&mut deck, "*NODE PRINT, NSET=NALL").expect("String writes cannot fail");
        writeln!(&mut deck, "U, RF").expect("String writes cannot fail");
        writeln!(&mut deck, "*NODE FILE").expect("String writes cannot fail");
        writeln!(&mut deck, "U").expect("String writes cannot fail");
        writeln!(&mut deck, "*EL FILE").expect("String writes cannot fail");
        writeln!(&mut deck, "S").expect("String writes cannot fail");
        writeln!(&mut deck, "*END STEP").expect("String writes cannot fail");

        Ok(deck)
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
    if !(1..=MAX_RESOLUTION).contains(&n) {
        return Err(MeshError::InvalidMeshTopology);
    }
    let side = n + 1;
    if mesh.nodes.len() != side * side || mesh.triangles.len() != 2 * n * n {
        return Err(MeshError::InvalidMeshTopology);
    }

    for (index, node) in mesh.nodes.iter().enumerate() {
        let i = index % side;
        let j = index / side;
        let expected_x = i as f64 / n as f64;
        let expected_y = j as f64 / n as f64;
        let expected_boundary = i == 0 || i == n || j == 0 || j == n;
        if node.id != index + 1
            || !node.x.is_finite()
            || !node.y.is_finite()
            || node.x.to_bits() != expected_x.to_bits()
            || node.y.to_bits() != expected_y.to_bits()
            || node.boundary != expected_boundary
        {
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

    for (cell, pair) in mesh.triangles.chunks_exact(2).enumerate() {
        let i = cell % n;
        let j = cell / n;
        let lower_left = j * side + i + 1;
        let lower_right = lower_left + 1;
        let upper_left = (j + 1) * side + i + 1;
        let upper_right = upper_left + 1;
        let first = pair[0];
        let second = pair[1];
        if first.id != 2 * cell + 1
            || first.nodes != [lower_left, lower_right, upper_right]
            || second.id != 2 * cell + 2
            || second.nodes != [lower_left, upper_right, upper_left]
        {
            return Err(MeshError::InvalidMeshTopology);
        }
    }

    Ok(())
}


fn write_node_set(output: &mut String, node_ids: impl Iterator<Item = usize>) {
    let mut on_line = 0_usize;
    for node_id in node_ids {
        if on_line > 0 {
            output.push(',');
            output.push(' ');
        }
        write!(output, "{node_id}").expect("String writes cannot fail");
        on_line += 1;
        if on_line == 16 {
            output.push('\n');
            on_line = 0;
        }
    }
    if on_line > 0 {
        output.push('\n');
    }
}

fn energy_density(strain: ccx_mms_p1_oracle::Strain2, lambda: f64, mu: f64) -> f64 {
    lambda * (strain.xx + strain.yy).powi(2)
        + 2.0 * mu * (strain.xx.powi(2) + strain.yy.powi(2) + 2.0 * strain.xy.powi(2))
}

/// Gauss-Legendre points and weights mapped to [0,1], sorted by point.
pub fn gauss_legendre_unit_interval(order: usize) -> Result<Vec<(f64, f64)>, MeshError> {
    if !(1..=64).contains(&order) {
        return Err(MeshError::InvalidQuadratureOrder);
    }
    let mut points = vec![(0.0, 0.0); order];
    let half = order.div_ceil(2);

    for i in 0..half {
        let mut z = (std::f64::consts::PI * (i as f64 + 0.75) / (order as f64 + 0.5)).cos();
        let mut converged = false;
        for _ in 0..64 {
            let (polynomial, previous) = legendre_pair(order, z);
            let derivative = order as f64 * (z * polynomial - previous) / (z * z - 1.0);
            let next = z - polynomial / derivative;
            if !next.is_finite() {
                return Err(MeshError::InvalidQuadratureOrder);
            }
            if (next - z).abs() <= 2.0 * f64::EPSILON * next.abs().max(1.0) {
                z = next;
                converged = true;
                break;
            }
            z = next;
        }
        if !converged || !z.is_finite() || z.abs() >= 1.0 {
            return Err(MeshError::InvalidQuadratureOrder);
        }

        let (polynomial, previous) = legendre_pair(order, z);
        let derivative = order as f64 * (z * polynomial - previous) / (z * z - 1.0);
        let weight = 1.0 / ((1.0 - z * z) * derivative * derivative);
        points[i] = ((1.0 - z) * 0.5, weight);
        points[order - 1 - i] = ((1.0 + z) * 0.5, weight);
    }
    if points.iter().any(|(point, weight)| {
        !point.is_finite()
            || !weight.is_finite()
            || !(0.0..=1.0).contains(point)
            || *weight <= 0.0
    }) {
        return Err(MeshError::InvalidQuadratureOrder);
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

    fn deck_rows<'a>(deck: &'a str, section: &str) -> Vec<&'a str> {
        let mut in_section = false;
        let mut rows = Vec::new();
        for line in deck.lines() {
            let trimmed = line.trim();
            if trimmed.eq_ignore_ascii_case(section) {
                in_section = true;
                continue;
            }
            if in_section && trimmed.starts_with('*') {
                break;
            }
            if in_section && !trimmed.is_empty() {
                rows.push(trimmed);
            }
        }
        rows
    }

    #[test]
    fn calculix_deck_is_deterministic_and_request_shaped() {
        let mesh = generate_mesh(4).unwrap();
        let problem = ManufacturedProblem::reference();
        let first = mesh.to_calculix_input_deck(problem, 16).unwrap();
        let second = mesh.to_calculix_input_deck(problem, 16).unwrap();
        assert_eq!(first, second);

        assert!(first.contains("*ELEMENT, TYPE=CPE3, ELSET=EALL"));
        assert!(first.contains("*SOLID SECTION, ELSET=EALL, MATERIAL=MMSMAT"));
        assert!(first.contains("*BOUNDARY\nBOUNDARY, 1, 2, 0.00000000000000000e+00"));
        assert!(first.contains("*NODE PRINT, NSET=NALL\nU, RF"));
        assert!(first.contains("*NODE FILE\nU"));
        assert!(first.contains("*EL FILE\nS"));

        let node_rows = deck_rows(&first, "*NODE, NSET=NALL");
        let element_rows = deck_rows(&first, "*ELEMENT, TYPE=CPE3, ELSET=EALL");
        let boundary_rows = deck_rows(&first, "*NSET, NSET=BOUNDARY");
        let cload_rows = deck_rows(&first, "*CLOAD");
        assert_eq!(node_rows.len(), 25);
        assert_eq!(element_rows.len(), 32);
        assert_eq!(boundary_rows.iter().map(|row| row.split(',').count()).sum::<usize>(), 16);
        assert!(!cload_rows.is_empty());

        let boundary_ids: std::collections::HashSet<usize> = mesh
            .nodes
            .iter()
            .filter(|node| node.boundary)
            .map(|node| node.id)
            .collect();
        for row in &cload_rows {
            let mut fields = row.split(',');
            let node_id = fields.next().unwrap().trim().parse::<usize>().unwrap();
            let degree = fields.next().unwrap().trim().parse::<u8>().unwrap();
            let value = fields.next().unwrap().trim().parse::<f64>().unwrap();
            assert!(!boundary_ids.contains(&node_id), "constrained node in CLOAD: {row}");
            assert!(degree == 1 || degree == 2, "unexpected CLOAD degree: {row}");
            assert!(value.is_finite(), "non-finite load: {row}");
            assert!(fields.next().is_none(), "unexpected extra CLOAD field: {row}");
        }

        let boundary_load_comments = first
            .lines()
            .filter(|line| line.starts_with("** BOUNDARY_LOAD,"))
            .count();
        assert_eq!(boundary_load_comments, 16);
    }

    #[test]
    fn calculix_deck_reconstructs_the_complete_consistent_load_vector() {
        let mesh = generate_mesh(4).unwrap();
        let problem = ManufacturedProblem::reference();
        let deck = mesh.to_calculix_input_deck(problem, 16).unwrap();
        let mut total = Vec2 { x: 0.0, y: 0.0 };

        for row in deck_rows(&deck, "*CLOAD") {
            let fields: Vec<&str> = row.split(',').map(str::trim).collect();
            assert_eq!(fields.len(), 3, "malformed CLOAD row: {row}");
            let degree = fields[1].parse::<u8>().unwrap();
            let value = fields[2].parse::<f64>().unwrap();
            match degree {
                1 => total.x += value,
                2 => {
                    total.y += value;
                }
                _ => panic!("unexpected CLOAD degree in {row}"),
            }
        }

        let mut boundary_records = 0_usize;
        for line in deck.lines().filter(|line| line.starts_with("** BOUNDARY_LOAD,")) {
            let payload = line.trim_start_matches("** BOUNDARY_LOAD,");
            let fields: Vec<&str> = payload.split(',').map(str::trim).collect();
            assert_eq!(fields.len(), 3, "malformed boundary-load comment: {line}");
            let node_id = fields[0].parse::<usize>().unwrap();
            assert!(mesh.nodes[node_id - 1].boundary);
            let fx = fields[1].parse::<f64>().unwrap();
            let fy = fields[2].parse::<f64>().unwrap();
            assert!(fx.is_finite() && fy.is_finite());
            total.x += fx;
            total.y += fy;
            boundary_records += 1;
        }
        assert_eq!(boundary_records, 16);

        let exact = problem.integrated_body_force();
        let full_load_x_residual = (total.x - exact.x).abs() / exact.x.abs().max(1.0);
        let absolute_force_y_scale = mesh
            .assemble_body_force_with_diagnostics(problem, 16)
            .unwrap()
            .integrated_absolute_force
            .y
            .max(1.0);
        let full_load_y_residual = total.y.abs() / absolute_force_y_scale;
        assert!(
            full_load_x_residual <= 1.0e-10,
            "reconstructed x-load residual={full_load_x_residual:e}"
        );
        assert!(
            full_load_y_residual <= 1.0e-10,
            "reconstructed y-load residual={full_load_y_residual:e}"
        );
    }

    #[test]
    fn calculix_deck_writer_rejects_mutated_mesh_and_invalid_quadrature() {
        let problem = ManufacturedProblem::reference();
        let mut changed = generate_mesh(4).unwrap();
        changed.nodes[5].y += 1.0e-4;
        assert_eq!(
            changed.to_calculix_input_deck(problem, 8).unwrap_err(),
            MeshError::InvalidMeshTopology
        );
        let mesh = generate_mesh(4).unwrap();
        assert_eq!(
            mesh.to_calculix_input_deck(problem, 0).unwrap_err(),
            MeshError::InvalidQuadratureOrder
        );
    }

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
    fn mutated_mesh_coordinates_or_connectivity_are_rejected() {
        let problem = ManufacturedProblem::reference();

        let mut changed_coordinate = generate_mesh(4).unwrap();
        changed_coordinate.nodes[0].x = 0.01;
        assert_eq!(
            changed_coordinate.assemble_body_force(problem, 4).unwrap_err(),
            MeshError::InvalidMeshTopology
        );

        let mut changed_connectivity = generate_mesh(4).unwrap();
        changed_connectivity.triangles.swap(0, 1);
        assert_eq!(
            changed_connectivity.assemble_body_force(problem, 4).unwrap_err(),
            MeshError::InvalidMeshTopology
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
        let assembly_32 = mesh
            .assemble_body_force_with_diagnostics(problem, 32)
            .unwrap();
        let assembly_48 = mesh
            .assemble_body_force_with_diagnostics(problem, 48)
            .unwrap();
        let difference = relative_load_difference(
            &assembly_32.nodal_loads,
            &assembly_48.nodal_loads,
        )
        .unwrap();
        assert!(difference <= 1.0e-12, "relative load difference={difference:e}");

        let total = Mesh::resultant(&assembly_48.nodal_loads).unwrap();
        let exact = problem.integrated_body_force();
        let normalized_x = (total.x - exact.x).abs() / exact.x.abs().max(1.0);
        let normalized_y =
            total.y.abs() / assembly_48.integrated_absolute_force.y.max(1.0);
        assert!(normalized_x <= 1.0e-10, "x resultant residual={normalized_x:e}");
        assert!(normalized_y <= 1.0e-10, "y normalized residual={normalized_y:e}");
        assert!(assembly_48.integrated_absolute_force.x > 0.0);
        assert!(assembly_48.integrated_absolute_force.y > 0.0);
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
    fn exact_nodal_interpolation_exhibits_expected_l2_and_energy_orders() {
        let problem = ManufacturedProblem::reference();
        let mut errors = Vec::new();
        for n in [8, 16, 32, 64] {
            let mesh = generate_mesh(n).unwrap();
            let exact_nodal: Vec<Vec2> = mesh
                .nodes
                .iter()
                .map(|node| problem.displacement(node.x, node.y))
                .collect();
            errors.push(mesh.compute_error_norms(problem, &exact_nodal, 12).unwrap());
        }

        for pair in errors.windows(2) {
            assert!(
                pair[1].displacement_l2_relative < pair[0].displacement_l2_relative,
                "displacement L2 error did not decrease"
            );
            assert!(
                pair[1].energy_error_relative < pair[0].energy_error_relative,
                "energy error did not decrease"
            );
        }

        for index in [1, 2] {
            let l2_order = (errors[index].displacement_l2_relative
                / errors[index + 1].displacement_l2_relative)
                .ln()
                / 2.0_f64.ln();
            let energy_order = (errors[index].energy_error_relative
                / errors[index + 1].energy_error_relative)
                .ln()
                / 2.0_f64.ln();
            assert!(
                (1.5..=2.5).contains(&l2_order),
                "interpolation L2 order={l2_order:.6}"
            );
            assert!(
                (0.75..=1.25).contains(&energy_order),
                "interpolation energy order={energy_order:.6}"
            );
        }
    }

    #[test]
    fn divergence_free_interpolation_reports_volumetric_strain_defect() {
        let material = ccx_mms_p1_oracle::PlaneStrainMaterial::new(1.0, 0.49999).unwrap();
        let problem = ManufacturedProblem::divergence_free(material);
        let mut measures = Vec::new();
        for resolution in [8, 16, 32] {
            let mesh = generate_mesh(resolution).unwrap();
            let exact_nodal: Vec<Vec2> = mesh
                .nodes
                .iter()
                .map(|node| problem.displacement(node.x, node.y))
                .collect();
            let norms = mesh.compute_error_norms(problem, &exact_nodal, 16).unwrap();
            assert_eq!(norms.exact_volumetric_strain_l2_norm, 0.0);
            assert_eq!(
                norms.volumetric_strain_error_l2_absolute,
                norms.fem_volumetric_strain_l2_absolute
            );
            assert!(norms.fem_volumetric_strain_l2_absolute.is_finite());
            measures.push(norms.fem_volumetric_strain_l2_absolute);
        }
        assert!(measures.iter().all(|value| *value > 0.0));
        assert!(measures.windows(2).all(|pair| pair[1] < pair[0]));
    }

    #[test]
    fn interpolation_error_norms_are_quadrature_independent() {
        let mesh = generate_mesh(8).unwrap();
        let problem = ManufacturedProblem::reference();
        let exact_nodal: Vec<Vec2> = mesh
            .nodes
            .iter()
            .map(|node| problem.displacement(node.x, node.y))
            .collect();
        let low = mesh.compute_error_norms(problem, &exact_nodal, 32).unwrap();
        let high = mesh.compute_error_norms(problem, &exact_nodal, 48).unwrap();
        let relative_l2 = (low.displacement_l2_relative - high.displacement_l2_relative).abs()
            / high.displacement_l2_relative.abs();
        let relative_energy = (low.energy_error_relative - high.energy_error_relative).abs()
            / high.energy_error_relative.abs();
        assert!(relative_l2 <= 1.0e-10, "L2 quadrature difference={relative_l2:e}");
        assert!(
            relative_energy <= 1.0e-10,
            "energy quadrature difference={relative_energy:e}"
        );
    }

    #[test]
    fn error_postprocessor_fails_closed_on_invalid_nodal_data() {
        let mesh = generate_mesh(4).unwrap();
        let problem = ManufacturedProblem::reference();
        assert_eq!(
            mesh.compute_error_norms(problem, &[], 8).unwrap_err(),
            MeshError::InvalidDisplacementVectorLength
        );
        let mut nodal: Vec<Vec2> = mesh
            .nodes
            .iter()
            .map(|node| problem.displacement(node.x, node.y))
            .collect();
        nodal[3].x = f64::NAN;
        assert_eq!(
            mesh.compute_error_norms(problem, &nodal, 8).unwrap_err(),
            MeshError::NonFiniteDisplacement
        );
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
