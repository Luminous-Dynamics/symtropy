//! Independent constant-strain triangle (CST) plane-strain element oracle.
//!
//! This crate checks element algebra and patch-test invariants independently of
//! CalculiX. It is a verification oracle, not a production FEM solver.

use std::error::Error;
use std::fmt;

use ccx_mms_p1_mesh::generate_mesh;
use ccx_mms_p1_oracle::{PlaneStrainMaterial, Strain2, Stress3, Vec2};

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
    NonFiniteEnergyAudit,
    InvalidStiffnessSpectrum,
    EigenvalueIterationLimit,
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
            Self::NonFiniteEnergyAudit => {
                f.write_str("element energy identity is non-finite or invalid")
            }
            Self::InvalidStiffnessSpectrum => {
                f.write_str("element stiffness spectrum is not three rigid plus three positive modes")
            }
            Self::EigenvalueIterationLimit => {
                f.write_str("symmetric Jacobi eigensolver did not converge")
            }
            Self::MeshConstruction => f.write_str("patch probe could not construct a valid mesh"),
        }
    }
}

impl Error for ElementError {}

/// Plane-strain linear triangle. DOF order is [ux1, uy1, ux2, uy2, ux3, uy3].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriangleElement {
    pub area: f64,
    /// Scale-invariant triangle mean-ratio quality in (0, 1], where 1 is equilateral.
    pub mean_ratio_quality: f64,
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
        let edge_squared = [
            abx * abx + aby * aby,
            acx * acx + acy * acy,
            bcx * bcx + bcy * bcy,
        ];
        let max_edge_squared = edge_squared.into_iter().fold(0.0_f64, f64::max);
        let edge_squared_sum = edge_squared.into_iter().sum::<f64>();
        let determinant = abx * acy - aby * acx;
        if !max_edge_squared.is_finite()
            || !edge_squared_sum.is_finite()
            || !determinant.is_finite()
        {
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
        let raw_mean_ratio_quality =
            2.0 * 3.0_f64.sqrt() * determinant.abs() / edge_squared_sum;
        if !raw_mean_ratio_quality.is_finite()
            || raw_mean_ratio_quality <= 0.0
            || raw_mean_ratio_quality > 1.0 + 64.0 * f64::EPSILON
        {
            return Err(ElementError::NonFiniteGeometry);
        }
        // Roundoff may put an equilateral triangle a few ulps above one.
        let mean_ratio_quality = raw_mean_ratio_quality.min(1.0);
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
            mean_ratio_quality,
            shape_gradients,
            stiffness,
            material,
            thickness,
        })
    }

    pub fn thickness(self) -> f64 {
        self.thickness
    }

    /// Estimate the element stiffness spectrum, separating its three rigid-body
    /// modes from its three deformational modes. This is a local element metric,
    /// not the condition number of a constrained global FEM system.
    pub fn spectrum(self) -> Result<ElementSpectrum, ElementError> {
        let mut eigenvalues = symmetric_eigenvalues(self.stiffness)?;
        eigenvalues.sort_by(f64::total_cmp);
        let spectral_scale = eigenvalues
            .iter()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        if !spectral_scale.is_finite() || spectral_scale <= 0.0 {
            return Err(ElementError::InvalidStiffnessSpectrum);
        }
        let rigid_mode_threshold = 4096.0 * f64::EPSILON * spectral_scale;
        let rigid_mode_count = eigenvalues
            .iter()
            .filter(|value| value.abs() <= rigid_mode_threshold)
            .count();
        let positive: Vec<f64> = eigenvalues
            .iter()
            .copied()
            .filter(|value| *value > rigid_mode_threshold)
            .collect();
        if rigid_mode_count != 3
            || positive.len() != 3
            || eigenvalues.iter().any(|value| *value < -rigid_mode_threshold)
        {
            return Err(ElementError::InvalidStiffnessSpectrum);
        }
        let smallest_positive_eigenvalue = positive[0];
        let largest_positive_eigenvalue = positive[2];
        let nonrigid_condition_number =
            largest_positive_eigenvalue / smallest_positive_eigenvalue;
        if !nonrigid_condition_number.is_finite() || nonrigid_condition_number < 1.0 {
            return Err(ElementError::InvalidStiffnessSpectrum);
        }
        Ok(ElementSpectrum {
            eigenvalues,
            rigid_mode_count,
            positive_mode_count: positive.len(),
            smallest_positive_eigenvalue,
            largest_positive_eigenvalue,
            nonrigid_condition_number,
            rigid_mode_threshold,
        })
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

    /// Compare matrix-based strain energy with an independently evaluated
    /// constitutive energy density. The returned defect is relative to the
    /// constitutive energy; this metric is intended for non-rigid test fields.
    pub fn normalized_energy_identity_defect(
        self,
        displacements: [Vec2; 3],
    ) -> Result<f64, ElementError> {
        let internal_force = self.internal_force(displacements)?;
        let dofs = [
            displacements[0].x,
            displacements[0].y,
            displacements[1].x,
            displacements[1].y,
            displacements[2].x,
            displacements[2].y,
        ];
        let matrix_energy = 0.5
            * internal_force
                .iter()
                .zip(dofs)
                .map(|(force, displacement)| *force * *displacement)
                .sum::<f64>();
        let strain = self.strain(displacements)?;
        let (lambda, mu) = self.material.lame_parameters();
        let trace = strain.xx + strain.yy;
        let constitutive_energy = 0.5
            * self.area
            * self.thickness
            * (lambda * trace * trace
                + 2.0
                    * mu
                    * (strain.xx * strain.xx
                        + strain.yy * strain.yy
                        + 2.0 * strain.xy * strain.xy));
        if !(matrix_energy.is_finite()
            && constitutive_energy.is_finite()
            && constitutive_energy > 0.0)
        {
            return Err(ElementError::NonFiniteEnergyAudit);
        }
        let defect = (matrix_energy - constitutive_energy).abs() / constitutive_energy;
        if !defect.is_finite() {
            return Err(ElementError::NonFiniteEnergyAudit);
        }
        Ok(defect)
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
                .map(|(stiffness, displacement)| *stiffness * *displacement)
                .sum();
        }
        if force.iter().any(|value| !value.is_finite()) {
            return Err(ElementError::NonFiniteInternalForce);
        }
        Ok(force)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElementSpectrum {
    /// Symmetric Jacobi eigenvalues sorted in nondecreasing order.
    pub eigenvalues: [f64; 6],
    /// Number of eigenvalues classified as rigid-body zero modes.
    pub rigid_mode_count: usize,
    /// Number of positive deformational modes.
    pub positive_mode_count: usize,
    pub smallest_positive_eigenvalue: f64,
    pub largest_positive_eigenvalue: f64,
    /// Spectral condition number on the three-dimensional deformational subspace.
    pub nonrigid_condition_number: f64,
    /// Absolute spectral threshold used to classify the three zero modes.
    pub rigid_mode_threshold: f64,
}

/// Symmetric Jacobi eigensolver for a small dense 6-by-6 element matrix.
/// The input is symmetrized at the level of roundoff before rotations; callers
/// separately test the unsymmetrized stiffness symmetry defect.
fn symmetric_eigenvalues(
    mut matrix: [[f64; 6]; 6],
) -> Result<[f64; 6], ElementError> {
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ElementError::InvalidStiffnessSpectrum);
    }
    for i in 0..6 {
        for j in (i + 1)..6 {
            let average = 0.5 * (matrix[i][j] + matrix[j][i]);
            if !average.is_finite() {
                return Err(ElementError::InvalidStiffnessSpectrum);
            }
            matrix[i][j] = average;
            matrix[j][i] = average;
        }
    }

    for _ in 0..(64 * 6 * 6) {
        let mut pivot = (0_usize, 1_usize);
        let mut largest_off_diagonal = 0.0_f64;
        let mut scale = 0.0_f64;
        for (i, row) in matrix.iter().enumerate() {
            scale = scale.max(row[i].abs());
            for (j, value) in row.iter().enumerate().skip(i + 1) {
                if value.abs() > largest_off_diagonal {
                    largest_off_diagonal = value.abs();
                    pivot = (i, j);
                }
            }
        }
        if !scale.is_finite() || !largest_off_diagonal.is_finite() {
            return Err(ElementError::InvalidStiffnessSpectrum);
        }
        if largest_off_diagonal <= 32.0 * f64::EPSILON * scale.max(f64::MIN_POSITIVE) {
            let mut values = [0.0_f64; 6];
            for (i, row) in matrix.iter().enumerate() {
                values[i] = row[i];
            }
            return if values.iter().all(|value| value.is_finite()) {
                Ok(values)
            } else {
                Err(ElementError::InvalidStiffnessSpectrum)
            };
        }

        let (p, q) = pivot;
        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        if apq == 0.0 {
            continue;
        }
        let tau = (aqq - app) / (2.0 * apq);
        if !tau.is_finite() {
            return Err(ElementError::InvalidStiffnessSpectrum);
        }
        let t = if tau >= 0.0 {
            1.0 / (tau + tau.hypot(1.0))
        } else {
            -1.0 / (-tau + tau.hypot(1.0))
        };
        let cosine = 1.0 / (1.0 + t * t).sqrt();
        let sine = t * cosine;
        matrix[p][p] = app - t * apq;
        matrix[q][q] = aqq + t * apq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;
        for k in 0..6 {
            if k == p || k == q {
                continue;
            }
            let akp = matrix[k][p];
            let akq = matrix[k][q];
            let new_kp = cosine * akp - sine * akq;
            let new_kq = sine * akp + cosine * akq;
            matrix[k][p] = new_kp;
            matrix[p][k] = new_kp;
            matrix[k][q] = new_kq;
            matrix[q][k] = new_kq;
        }
    }
    Err(ElementError::EigenvalueIterationLimit)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometrySweepPoint {
    pub height: f64,
    pub mean_ratio_quality: f64,
    /// Longest edge length times the maximum shape-gradient norm. This is a
    /// dimensionless geometry-amplification proxy, not a matrix condition number.
    pub dimensionless_gradient_amplification: f64,
    pub element_nonrigid_condition_number: f64,
}

pub fn run_geometry_conditioning_sweep() -> Result<Vec<GeometrySweepPoint>, ElementError> {
    let heights = [1.0, 0.1, 0.01, 0.001, 0.0001];
    let mut points = Vec::with_capacity(heights.len());
    for height in heights {
        let vertices = [
            Point2 { x: 0.0, y: 0.0 },
            Point2 { x: 1.0, y: 0.0 },
            Point2 { x: 0.5, y: height },
        ];
        let element = TriangleElement::new(vertices, material_for_sweep(), 1.0)?;
        let [a, b, c] = vertices;
        let longest_edge = [
            (b.x - a.x).hypot(b.y - a.y),
            (c.x - a.x).hypot(c.y - a.y),
            (c.x - b.x).hypot(c.y - b.y),
        ]
        .into_iter()
        .fold(0.0_f64, f64::max);
        let max_gradient_norm = element
            .shape_gradients
            .iter()
            .map(|gradient| gradient[0].hypot(gradient[1]))
            .fold(0.0_f64, f64::max);
        let spectrum = element.spectrum()?;
        let point = GeometrySweepPoint {
            height,
            mean_ratio_quality: element.mean_ratio_quality,
            dimensionless_gradient_amplification: longest_edge * max_gradient_norm,
            element_nonrigid_condition_number: spectrum.nonrigid_condition_number,
        };
        if !(point.mean_ratio_quality.is_finite()
            && point.dimensionless_gradient_amplification.is_finite()
            && point.element_nonrigid_condition_number.is_finite()
            && point.mean_ratio_quality > 0.0
            && point.dimensionless_gradient_amplification > 0.0
            && point.element_nonrigid_condition_number >= 1.0)
        {
            return Err(ElementError::NonFiniteGeometry);
        }
        points.push(point);
    }
    Ok(points)
}

fn material_for_sweep() -> PlaneStrainMaterial {
    PlaneStrainMaterial::new(1.0, 0.3).expect("reference material is admissible")
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatchProbe {
    pub elements: usize,
    pub interior_nodes: usize,
    pub max_strain_error: f64,
    pub max_stiffness_symmetry_defect: f64,
    pub max_energy_identity_defect: f64,
    pub max_interior_force_linf: f64,
    pub max_rigid_mode_force_linf: f64,
}

impl PatchProbe {
    pub fn passed(self) -> bool {
        self.max_strain_error <= 1.0e-12
            && self.max_stiffness_symmetry_defect <= 1.0e-12
            && self.max_energy_identity_defect <= 1.0e-12
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
    let mut max_energy_identity_defect = 0.0_f64;

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

        let energy_defect = element.normalized_energy_identity_defect(displacements)?;
        max_energy_identity_defect = max_energy_identity_defect.max(energy_defect);

        let internal_force = element.internal_force(displacements)?;
        for (local_node, node_id) in triangle.nodes.iter().enumerate() {
            let global = &mut global_internal_forces[*node_id - 1];
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
        max_energy_identity_defect,
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
        let energy = 0.5
            * force
                .iter()
                .zip(values)
                .map(|(internal, displacement)| *internal * displacement)
                .sum::<f64>();
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

    #[test]
    fn stiffness_matrix_corruption_is_detected_by_energy_identity_control() {
        let vertices = points();
        let displacements = vertices.map(affine_displacement);
        let mut corrupted = TriangleElement::new(vertices, material(), 1.0).unwrap();
        corrupted.stiffness[0][0] += 0.5;

        let defect = corrupted
            .normalized_energy_identity_defect(displacements)
            .unwrap();
        assert!(
            defect > 1.0e-6,
            "mutated stiffness passed the energy identity control: {defect:e}"
        );
    }

    #[test]
    fn irregular_star_patch_preserves_affine_strain_and_center_equilibrium() {
        let vertices = [
            Point2 { x: 0.0, y: 0.0 },
            Point2 { x: 2.1, y: 0.2 },
            Point2 { x: 1.7, y: 1.8 },
            Point2 { x: -0.2, y: 1.4 },
            Point2 { x: 0.72, y: 0.63 },
        ];
        let triangles = [[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];
        let displacements = vertices.map(affine_displacement);
        let mut assembled = [Vec2 { x: 0.0, y: 0.0 }; 5];
        for triangle in triangles {
            let element_vertices = triangle.map(|index| vertices[index]);
            let element_displacements = triangle.map(|index| displacements[index]);
            let element =
                TriangleElement::new(element_vertices, material(), 1.0).unwrap();
            let strain = element.strain(element_displacements).unwrap();
            close(strain.xx, 0.021, 1.0e-14);
            close(strain.yy, 0.052, 1.0e-14);
            close(strain.xy, 0.003, 1.0e-14);

            let internal = element.internal_force(element_displacements).unwrap();
            for (local_node, global_node) in triangle.iter().enumerate() {
                assembled[*global_node].x += internal[2 * local_node];
                assembled[*global_node].y += internal[2 * local_node + 1];
            }
        }
        close(assembled[4].x, 0.0, 1.0e-12);
        close(assembled[4].y, 0.0, 1.0e-12);
    }

    #[test]
    fn uniform_geometry_scaling_preserves_cst_stiffness() {
        let original = TriangleElement::new(points(), material(), 1.0).unwrap();
        for scale in [1.0e-6, 1.0e6] {
            let scaled_points = points().map(|point| Point2 {
                x: point.x * scale,
                y: point.y * scale,
            });
            let scaled = TriangleElement::new(scaled_points, material(), 1.0).unwrap();
            for (i, row) in scaled.stiffness.iter().enumerate() {
                for (j, value) in row.iter().enumerate() {
                    close(*value, original.stiffness[i][j], 1.0e-12);
                }
            }
        }
    }

    #[test]
    fn stiffness_is_linear_in_element_thickness() {
        let thin = TriangleElement::new(points(), material(), 1.0).unwrap();
        let thick = TriangleElement::new(points(), material(), 2.5).unwrap();
        for (i, row) in thick.stiffness.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                close(*value, 2.5 * thin.stiffness[i][j], 1.0e-14);
            }
        }
    }

    #[test]
    fn strain_is_covariant_under_rigid_coordinate_rotation() {
        let vertices = points();
        let displacements = vertices.map(affine_displacement);
        let element = TriangleElement::new(vertices, material(), 1.0).unwrap();
        let strain = element.strain(displacements).unwrap();

        let angle: f64 = 0.37;
        let cosine = angle.cos();
        let sine = angle.sin();
        let rotate = |x: f64, y: f64| Vec2 {
            x: cosine * x - sine * y,
            y: sine * x + cosine * y,
        };
        let rotated_vertices = vertices.map(|point| {
            let rotated = rotate(point.x, point.y);
            Point2 {
                x: rotated.x,
                y: rotated.y,
            }
        });
        let rotated_displacements = displacements.map(|value| rotate(value.x, value.y));
        let rotated =
            TriangleElement::new(rotated_vertices, material(), 1.0).unwrap();
        let observed = rotated.strain(rotated_displacements).unwrap();

        let expected_xx = cosine * cosine * strain.xx
            + sine * sine * strain.yy
            - 2.0 * sine * cosine * strain.xy;
        let expected_yy = sine * sine * strain.xx
            + cosine * cosine * strain.yy
            + 2.0 * sine * cosine * strain.xy;
        let expected_xy = sine * cosine * (strain.xx - strain.yy)
            + (cosine * cosine - sine * sine) * strain.xy;
        close(observed.xx, expected_xx, 2.0e-14);
        close(observed.yy, expected_yy, 2.0e-14);
        close(observed.xy, expected_xy, 2.0e-14);

        let stress = element.stress(displacements).unwrap();
        let rotated_stress = rotated.stress(rotated_displacements).unwrap();
        let expected_sxx = cosine * cosine * stress.xx
            + sine * sine * stress.yy
            - 2.0 * sine * cosine * stress.xy;
        let expected_syy = sine * sine * stress.xx
            + cosine * cosine * stress.yy
            + 2.0 * sine * cosine * stress.xy;
        let expected_sxy = sine * cosine * (stress.xx - stress.yy)
            + (cosine * cosine - sine * sine) * stress.xy;
        close(rotated_stress.xx, expected_sxx, 4.0e-14);
        close(rotated_stress.yy, expected_syy, 4.0e-14);
        close(rotated_stress.xy, expected_sxy, 4.0e-14);
        close(rotated_stress.zz, stress.zz, 4.0e-14);
    }

    #[test]
    fn mean_ratio_quality_is_scale_and_rotation_invariant() {
        let vertices = points();
        let original = TriangleElement::new(vertices, material(), 1.0).unwrap();
        close(
            TriangleElement::new(vertices.map(|point| Point2 {
                x: point.x * 1.0e-6,
                y: point.y * 1.0e-6,
            }), material(), 1.0)
            .unwrap()
            .mean_ratio_quality,
            original.mean_ratio_quality,
            1.0e-14,
        );

        let angle: f64 = -0.61;
        let cosine = angle.cos();
        let sine = angle.sin();
        let rotated_vertices = vertices.map(|point| Point2 {
            x: cosine * point.x - sine * point.y,
            y: sine * point.x + cosine * point.y,
        });
        let rotated =
            TriangleElement::new(rotated_vertices, material(), 1.0).unwrap();
        close(
            rotated.mean_ratio_quality,
            original.mean_ratio_quality,
            1.0e-14,
        );
        assert!(original.mean_ratio_quality > 0.0);
        assert!(original.mean_ratio_quality <= 1.0);
    }

    #[test]
    fn equilateral_triangle_quality_is_one_with_roundoff_clamped() {
        let equilateral = TriangleElement::new(
            [
                Point2 { x: 0.0, y: 0.0 },
                Point2 { x: 1.0, y: 0.0 },
                Point2 {
                    x: 0.5,
                    y: 3.0_f64.sqrt() * 0.5,
                },
            ],
            material(),
            1.0,
        )
        .unwrap();
        close(equilateral.mean_ratio_quality, 1.0, 1.0e-15);
        assert!(equilateral.mean_ratio_quality <= 1.0);
    }

    #[test]
    fn near_degenerate_geometry_is_rejected_at_relative_floating_point_floor() {
        let epsilon = f64::EPSILON;
        let rejected = TriangleElement::new(
            [
                Point2 { x: 0.0, y: 0.0 },
                Point2 { x: 1.0, y: 0.0 },
                Point2 { x: 0.5, y: 16.0 * epsilon },
            ],
            material(),
            1.0,
        );
        assert_eq!(rejected.unwrap_err(), ElementError::DegenerateTriangle);

        let accepted = TriangleElement::new(
            [
                Point2 { x: 0.0, y: 0.0 },
                Point2 { x: 1.0, y: 0.0 },
                Point2 { x: 0.5, y: 256.0 * epsilon },
            ],
            material(),
            1.0,
        )
        .unwrap();
        assert!(accepted.mean_ratio_quality > 0.0);
        assert!(accepted.mean_ratio_quality < 1.0e-12);
    }

    #[test]
    fn mean_ratio_quality_degrades_monotonically_for_sliver_triangles() {
        let heights = [1.0, 0.1, 0.01, 0.001];
        let qualities = heights.map(|height| {
            TriangleElement::new(
                [
                    Point2 { x: 0.0, y: 0.0 },
                    Point2 { x: 1.0, y: 0.0 },
                    Point2 { x: 0.5, y: height },
                ],
                material(),
                1.0,
            )
            .unwrap()
            .mean_ratio_quality
        });
        assert!(qualities.windows(2).all(|pair| pair[1] < pair[0]));
    }


    #[test]
    fn conditioning_sweep_separates_quality_from_gradient_amplification() {
        let sweep = run_geometry_conditioning_sweep().unwrap();
        assert_eq!(sweep.len(), 5);
        assert!(sweep.windows(2).all(|pair| {
            pair[1].mean_ratio_quality < pair[0].mean_ratio_quality
                && pair[1].dimensionless_gradient_amplification
                    > pair[0].dimensionless_gradient_amplification
                && pair[1].element_nonrigid_condition_number
                    > pair[0].element_nonrigid_condition_number
        }));
        assert!(sweep.iter().all(|point| {
            point.mean_ratio_quality > 0.0
                && point.mean_ratio_quality <= 1.0
                && point.dimensionless_gradient_amplification.is_finite()
        }));
    }

    #[test]
    fn element_spectrum_resolves_three_rigid_and_three_positive_modes() {
        let element = TriangleElement::new(points(), material(), 1.0).unwrap();
        let spectrum = element.spectrum().unwrap();
        assert_eq!(spectrum.rigid_mode_count, 3, "{spectrum:?}");
        assert_eq!(spectrum.positive_mode_count, 3, "{spectrum:?}");
        assert!(spectrum.smallest_positive_eigenvalue > 0.0);
        assert!(spectrum.largest_positive_eigenvalue >= spectrum.smallest_positive_eigenvalue);
        assert!(spectrum.nonrigid_condition_number >= 1.0);
        assert!(spectrum.eigenvalues.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn element_spectral_conditioning_degrades_for_slivers() {
        let sweep = run_geometry_conditioning_sweep().unwrap();
        assert!(sweep.windows(2).all(|pair| {
            pair[1].element_nonrigid_condition_number
                > pair[0].element_nonrigid_condition_number
        }));
        assert!(
            sweep.last().unwrap().element_nonrigid_condition_number > 1.0e7,
            "{sweep:?}"
        );
    }


}
