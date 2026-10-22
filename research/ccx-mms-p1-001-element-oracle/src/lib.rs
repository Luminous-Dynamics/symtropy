//! Independent constant-strain triangle (CST) plane-strain element oracle.
//!
//! This crate checks element algebra and patch-test invariants independently of
//! CalculiX. It is a verification oracle, not a production FEM solver.

use std::error::Error;
use std::fmt;

use ccx_mms_p1_mesh::generate_mesh;
use ccx_mms_p1_oracle::{PlaneStrainMaterial, Stress3, Strain2, Vec2};

const DEGENERACY_FACTOR: f64 = 64.0 * f64::EPSILON;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ElementError {
    NonFiniteGeometry,
    DegenerateTriangle,
    InvalidThickness,
    NonFiniteDisplacement,
    NonFiniteStiffness,
    NonFiniteInternalForce,
    NonFiniteStrain,
    NonFiniteStress,
    MeshConstruction,
}

impl fmt::Display for ElementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteGeometry => {
                f.write_str("triangle coordinates are non-finite or overflow")
            }
            Self::DegenerateTriangle => {
                f.write_str("triangle area is zero or numerically degenerate")
            }
            Self::InvalidThickness => f.write_str("thickness must be finite and positive"),
            Self::NonFiniteDisplacement => f.write_str("nodal displacement is non-finite"),
            Self::NonFiniteStiffness => f.write_str("element stiffness contains non-finite values"),
            Self::NonFiniteInternalForce => f.write_str("element internal force is non-finite"),
            Self::NonFiniteStrain => f.write_str("element strain is non-finite"),
            Self::NonFiniteStress => f.write_str("element stress is non-finite"),
            Self::MeshConstruction => f.write_str("patch probe could not construct a valid mesh"),
        }
    }
}

impl Error for ElementError {}

/// Plane-strain linear triangle. DOF order is [ux1, uy1, ux2, uy2, ux3, uy3].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriangleElement {
    pub area: f64,
    /// Per-node shape-function gradients [(dN/dx), (dN/dy)].
    pub shape_gradients: [[f64; 2]; 3],
    /// Small-strain element stiffness matrix (6 × 6).
    pub stiffness: [[f64; 6]; 6],
    material: PlaneStrainMaterial,
    thickness: f64,
}

impl TriangleElement {
    /// Construct the CST stiffness using engineering shear strain gamma_xy.
    ///
    /// Either vertex winding is accepted. The signed determinant is retained in
    /// the shape gradients, while area is positive. Near-degenerate geometry is
    /// rejected using a scale-relative floating-point threshold.
    pub fn new(
        vertices: [Point2; 3],
        material: PlaneStrainMaterial,
        thickness: f64,
    ) -> Result<Self, ElementError> {
        if !thickness.is_finite() || thickness <= 0.0 {
            return Err(ElementError::InvalidThickness);
        }
        if vertices
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
        {
            return Err(ElementError::NonFiniteGeometry);
        }

        let [a, b, c] = vertices;
        let abx = b.x - a.x;
        let aby = b.y - a.y;
        let acx = c.x - a.x;
        let acy = c.y - a.y;
        let bcx = c.x - b.x;
        let bcy = c.y - b.y;
        let max_edge_squared = [
            abx * abx + aby * aby,
            acx * acx + acy * acy,
            bcx * bcx + bcy * bcy,
        ]
        .into_iter()
        .fold(0.0_f64, f64::max);
        let determinant = abx * acy - aby * acx;
        if !max_edge_squared.is_finite() || !determinant.is_finite() {
            return Err(ElementError::NonFiniteGeometry);
        }
        if max_edge_squared <= 0.0
            || determinant.abs() <= DEGENERACY_FACTOR * max_edge_squared
        {
            return Err(ElementError::DegenerateTriangle);
        }

        let inverse_determinant = determinant.recip();
        let shape_gradients = [
            [(b.y - c.y) * inverse_determinant, (c.x - b.x) * inverse_determinant],
            [(c.y - a.y) * inverse_determinant, (a.x - c.x) * inverse_determinant],
            [(a.y - b.y) * inverse_determinant, (b.x - a.x) * inverse_determinant],
        ];
        if shape_gradients
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(ElementError::NonFiniteGeometry);
        }

        // Engineering strain vector [epsilon_xx, epsilon_yy, gamma_xy].
        let mut b_matrix = [[0.0_f64; 6]; 3];
        for (node, gradient) in shape_gradients.iter().enumerate() {
            let dof_x = 2 * node;
            let dof_y = dof_x + 1;
            b_matrix[0][dof_x] = gradient[0];
            b_matrix[1][dof_y] = gradient[1];
            b_matrix[2][dof_x] = gradient[1];
            b_matrix[2][dof_y] = gradient[0];
        }

        let (lambda, mu) = material.lame_parameters();
        // Plane-strain constitutive matrix for [epsilon_xx, epsilon_yy, gamma_xy].
        let constitutive = [
            [lambda + 2.0 * mu, lambda, 0.0],
            [lambda, lambda + 2.0 * mu, 0.0],
            [0.0, 0.0, mu],
        ];
        let area = determinant.abs() * 0.5;
        let scale = area * thickness;
        let mut constitutive_times_b = [[0.0_f64; 6]; 3];
        for (row_index, result_row) in constitutive_times_b.iter_mut().enumerate() {
            for (constitutive_value, b_row) in
                constitutive[row_index].iter().zip(b_matrix.iter())
            {
                for (result, b_value) in result_row.iter_mut().zip(b_row.iter()) {
                    *result += *constitutive_value * *b_value;
                }
            }
        }

        let mut stiffness = [[0.0_f64; 6]; 6];
        for (i, stiffness_row) in stiffness.iter_mut().enumerate() {
            for (j, stiffness_value) in stiffness_row.iter_mut().enumerate() {
                let value = b_matrix
                    .iter()
                    .zip(constitutive_times_b.iter())
                    .map(|(b_row, db_row)| b_row[i] * db_row[j])
                    .sum::<f64>();
                *stiffness_value = scale * value;
            }
        }
        if stiffness.iter().flatten().any(|value| !value.is_finite()) {
            return Err(ElementError::NonFiniteStiffness);
        }

        Ok(Self {
            area,
            shape_gradients,
            stiffness,
            material,
            thickness,
        })
    }

    pub fn thickness(self) -> f64 {
        self.thickness
    }

    /// Recover the constant small-strain tensor from the three nodal values.
    pub fn strain(self, displacements: [Vec2; 3]) -> Result<Strain2, ElementError> {
        if displacements
            .iter()
            .any(|u| !u.x.is_finite() || !u.y.is_finite())
        {
            return Err(ElementError::NonFiniteDisplacement);
        }
        let mut dux_dx = 0.0;
        let mut dux_dy = 0.0;
        let mut duy_dx = 0.0;
        let mut duy_dy = 0.0;
        for (u, gradient) in displacements.iter().zip(self.shape_gradients) {
            dux_dx += u.x * gradient[0];
            dux_dy += u.x * gradient[1];
            duy_dx += u.y * gradient[0];
            duy_dy += u.y * gradient[1];
        }
        let strain = Strain2 {
            xx: dux_dx,
            yy: duy_dy,
            xy: 0.5 * (dux_dy + duy_dx),
        };
        if !(strain.xx.is_finite() && strain.yy.is_finite() && strain.xy.is_finite()) {
            return Err(ElementError::NonFiniteStrain);
        }
        Ok(strain)
    }

    pub fn stress(self, displacements: [Vec2; 3]) -> Result<Stress3, ElementError> {
        let strain = self.strain(displacements)?;
        let (lambda, mu) = self.material.lame_parameters();
        let trace = strain.xx + strain.yy;
        let stress = Stress3 {
            xx: lambda * trace + 2.0 * mu * strain.xx,
            yy: lambda * trace + 2.0 * mu * strain.yy,
            zz: lambda * trace,
            xy: 2.0 * mu * strain.xy,
        };
        if !(stress.xx.is_finite()
            && stress.yy.is_finite()
            && stress.zz.is_finite()
            && stress.xy.is_finite())
        {
            return Err(ElementError::NonFiniteStress);
        }
        Ok(stress)
    }

    pub fn internal_force(self, displacements: [Vec2; 3]) -> Result<[f64; 6], ElementError> {
        if displacements
            .iter()
            .any(|u| !u.x.is_finite() || !u.y.is_finite())
        {
            return Err(ElementError::NonFiniteDisplacement);
        }
        let dofs = [
            displacements[0].x,
            displacements[0].y,
            displacements[1].x,
            displacements[1].y,
            displacements[2].x,
            displacements[2].y,
        ];
        let mut force = [0.0; 6];
        for (stiffness_row, total) in self.stiffness.iter().zip(force.iter_mut()) {
            *total = stiffness_row
                .iter()
                .zip(dofs.iter())
                .map(|(stiffness, displacement)| stiffness * displacement)
                .sum();
        }
        if force.iter().any(|value| !value.is_finite()) {
            return Err(ElementError::NonFiniteInternalForce);
        }
        Ok(force)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatchProbe {
    pub elements: usize,
    pub interior_nodes: usize,
    pub max_strain_error: f64,
    pub max_stiffness_symmetry_defect: f64,
    pub max_interior_force_linf: f64,
    pub max_rigid_mode_force_linf: f64,
}

impl PatchProbe {
    pub fn passed(self) -> bool {
        self.max_strain_error <= 1.0e-12
            && self.max_stiffness_symmetry_defect <= 1.0e-12
            && self.max_interior_force_linf <= 1.0e-12
            && self.max_rigid_mode_force_linf <= 1.0e-12
    }
}

fn affine_displacement(point: Point2) -> Vec2 {
    Vec2 {
        x: 0.13 + 0.021 * point.x - 0.037 * point.y,
        y: -0.17 + 0.043 * point.x + 0.052 * point.y,
    }
}

/// Assemble an affine-strain patch using the independent element kernel.
/// The report tests element strain, stiffness symmetry, rigid modes, and
/// cancellation of internal nodal forces at interior mesh nodes.
pub fn run_patch_probe() -> Result<PatchProbe, ElementError> {
    let mesh = generate_mesh(4).map_err(|_| ElementError::MeshConstruction)?;
    let material =
        PlaneStrainMaterial::new(1.0, 0.3).expect("reference material is admissible");
    let expected_strain = Strain2 {
        xx: 0.021,
        yy: 0.052,
        xy: 0.003,
    };
    let nodal_displacements: Vec<Vec2> = mesh
        .nodes
        .iter()
        .map(|node| affine_displacement(Point2 { x: node.x, y: node.y }))
        .collect();
    let mut global_internal_forces = vec![Vec2 { x: 0.0, y: 0.0 }; mesh.nodes.len()];
    let mut max_strain_error = 0.0_f64;
    let mut max_stiffness_symmetry_defect = 0.0_f64;

    for triangle in &mesh.triangles {
        let vertices = triangle.nodes.map(|id| {
            let node = mesh.nodes[id - 1];
            Point2 { x: node.x, y: node.y }
        });
        let displacements = triangle.nodes.map(|id| nodal_displacements[id - 1]);
        let element = TriangleElement::new(vertices, material, 1.0)?;
        let strain = element.strain(displacements)?;
        max_strain_error = max_strain_error
            .max((strain.xx - expected_strain.xx).abs())
            .max((strain.yy - expected_strain.yy).abs())
            .max((strain.xy - expected_strain.xy).abs());

        for (i, row) in element.stiffness.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                max_stiffness_symmetry_defect = max_stiffness_symmetry_defect
                    .max((*value - element.stiffness[j][i]).abs());
            }
        }

        let internal_force = element.internal_force(displacements)?;
        for (local_node, node_id) in triangle.nodes.iter().enumerate() {
            let global = &mut global_internal_forces[node_id - 1];
            global.x += internal_force[2 * local_node];
            global.y += internal_force[2 * local_node + 1];
        }
    }

    let mut interior_nodes = 0;
    let mut max_interior_force_linf = 0.0_f64;
    for (index, node) in mesh.nodes.iter().enumerate() {
        if !node.boundary {
            interior_nodes += 1;
            max_interior_force_linf = max_interior_force_linf
                .max(global_internal_forces[index].x.abs())
                .max(global_internal_forces[index].y.abs());
        }
    }

    let first_triangle = mesh.triangles[0];
    let vertices = first_triangle.nodes.map(|id| {
        let node = mesh.nodes[id - 1];
        Point2 { x: node.x, y: node.y }
    });
    let element = TriangleElement::new(vertices, material, 1.0)?;
    let translation = [Vec2 { x: 0.31, y: -0.27 }; 3];
    let rotation = vertices.map(|point| Vec2 {
        x: -0.37 * point.y,
        y: 0.37 * point.x,
    });
    let translation_force = element.internal_force(translation)?;
    let rotation_force = element.internal_force(rotation)?;
    let max_rigid_mode_force_linf = translation_force
        .into_iter()
        .chain(rotation_force)
        .map(f64::abs)
        .fold(0.0_f64, f64::max);

    Ok(PatchProbe {
        elements: mesh.triangles.len(),
        interior_nodes,
        max_strain_error,
        max_stiffness_symmetry_defect,
        max_interior_force_linf,
        max_rigid_mode_force_linf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material() -> PlaneStrainMaterial {
        PlaneStrainMaterial::new(1.0, 0.3).unwrap()
    }

    fn points() -> [Point2; 3] {
        [
            Point2 { x: 0.2, y: -0.4 },
            Point2 { x: 2.3, y: 0.1 },
            Point2 { x: 0.4, y: 1.7 },
        ]
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.17e}, expected={expected:.17e}, tol={tolerance:.3e}"
        );
    }

    #[test]
    fn affine_patch_recovers_exact_constant_strain_and_stress() {
        let vertices = points();
        let displacements = vertices.map(affine_displacement);
        let element = TriangleElement::new(vertices, material(), 1.0).unwrap();
        let strain = element.strain(displacements).unwrap();
        close(strain.xx, 0.021, 1.0e-14);
        close(strain.yy, 0.052, 1.0e-14);
        close(strain.xy, 0.003, 1.0e-14);

        let stress = element.stress(displacements).unwrap();
        let (lambda, mu) = material().lame_parameters();
        close(stress.xx, lambda * 0.073 + 2.0 * mu * 0.021, 1.0e-14);
        close(stress.yy, lambda * 0.073 + 2.0 * mu * 0.052, 1.0e-14);
        close(stress.zz, lambda * 0.073, 1.0e-14);
        close(stress.xy, mu * 0.006, 1.0e-14);
    }

    #[test]
    fn stiffness_is_symmetric_and_has_rigid_body_null_modes() {
        let vertices = points();
        let element = TriangleElement::new(vertices, material(), 1.7).unwrap();
        for (i, row) in element.stiffness.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                close(*value, element.stiffness[j][i], 1.0e-14);
            }
        }
        for mode in [
            [Vec2 { x: 1.0, y: 0.0 }; 3],
            [Vec2 { x: 0.0, y: 1.0 }; 3],
            vertices.map(|point| Vec2 { x: -point.y, y: point.x }),
        ] {
            let force = element.internal_force(mode).unwrap();
            for value in force {
                close(value, 0.0, 1.0e-13);
            }
        }
    }

    #[test]
    fn reversing_vertex_order_only_permutes_matrix_dofs() {
        let vertices = points();
        let original = TriangleElement::new(vertices, material(), 0.8).unwrap();
        let reversed =
            TriangleElement::new([vertices[0], vertices[2], vertices[1]], material(), 0.8).unwrap();
        let permutation = [0, 1, 4, 5, 2, 3];
        for (i, row) in reversed.stiffness.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                close(
                    *value,
                    original.stiffness[permutation[i]][permutation[j]],
                    1.0e-14,
                );
            }
        }
    }

    #[test]
    fn stiffness_energy_is_nonnegative_for_deterministic_displacements() {
        let element = TriangleElement::new(points(), material(), 1.0).unwrap();
        let displacements = [
            Vec2 { x: 0.21, y: -0.08 },
            Vec2 { x: -0.13, y: 0.43 },
            Vec2 { x: 0.37, y: 0.11 },
        ];
        let force = element.internal_force(displacements).unwrap();
        let values = [
            displacements[0].x,
            displacements[0].y,
            displacements[1].x,
            displacements[1].y,
            displacements[2].x,
            displacements[2].y,
        ];
        let energy = 0.5 * force.iter().zip(values).map(|(f, u)| f * u).sum::<f64>();
        assert!(energy.is_finite() && energy >= 0.0, "energy={energy:e}");
    }

    #[test]
    fn invalid_geometry_thickness_and_displacements_fail_closed() {
        assert_eq!(
            TriangleElement::new(
                [
                    Point2 { x: 0.0, y: 0.0 },
                    Point2 { x: 1.0, y: 1.0 },
                    Point2 { x: 2.0, y: 2.0 },
                ],
                material(),
                1.0,
            )
            .unwrap_err(),
            ElementError::DegenerateTriangle
        );
        assert_eq!(
            TriangleElement::new(points(), material(), f64::NAN).unwrap_err(),
            ElementError::InvalidThickness
        );
        assert_eq!(
            TriangleElement::new(
                [
                    Point2 { x: f64::INFINITY, y: 0.0 },
                    points()[1],
                    points()[2],
                ],
                material(),
                1.0,
            )
            .unwrap_err(),
            ElementError::NonFiniteGeometry
        );

        let element = TriangleElement::new(points(), material(), 1.0).unwrap();
        assert_eq!(
            element
                .strain(
                    [
                        Vec2 { x: f64::NAN, y: 0.0 },
                        Vec2 { x: 0.0, y: 0.0 },
                        Vec2 { x: 0.0, y: 0.0 },
                    ]
                )
                .unwrap_err(),
            ElementError::NonFiniteDisplacement
        );
    }

    #[test]
    fn assembled_global_patch_has_zero_interior_force_and_exact_strain() {
        let probe = run_patch_probe().unwrap();
        assert_eq!(probe.elements, 32);
        assert_eq!(probe.interior_nodes, 9);
        assert!(probe.passed(), "{probe:?}");
    }
}
