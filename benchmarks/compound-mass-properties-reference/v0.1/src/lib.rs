//! Compound mass-property composition and dense N-D bivector inertia reference.
//!
//! This M1 oracle stores the full symmetric mass second-moment tensor about a
//! component's center of mass. It composes rotated components with the tensor
//! parallel-axis theorem and builds the induced inertia operator on bivectors.
//! It is a research/reference implementation, not a production physics engine.

use std::error::Error;
use std::fmt::{Display, Formatter};

const MIN_DIMENSION: usize = 2;
const MAX_DIMENSION: usize = 9;
const RELATIVE_TOLERANCE: f64 = 1.0e-10;

pub type Matrix = Vec<Vec<f64>>;
pub type Vector = Vec<f64>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MassPropertiesError {
    InvalidDimension,
    DimensionMismatch,
    NonFiniteInput,
    NonPositiveMass,
    NonSymmetricSecondMoment,
    NonPositiveSemidefiniteSecondMoment,
    InvalidRotation,
    SingularInertiaOperator,
    NonFiniteResult,
}

impl Display for MassPropertiesError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDimension => write!(formatter, "dimension must be in 2..=9"),
            Self::DimensionMismatch => {
                write!(formatter, "vector or matrix dimensions do not match")
            }
            Self::NonFiniteInput => write!(formatter, "input values must be finite"),
            Self::NonPositiveMass => write!(formatter, "component mass must be positive"),
            Self::NonSymmetricSecondMoment => {
                write!(formatter, "second-moment tensor must be symmetric")
            }
            Self::NonPositiveSemidefiniteSecondMoment => write!(
                formatter,
                "second-moment tensor must be positive semidefinite"
            ),
            Self::InvalidRotation => write!(
                formatter,
                "component rotation must be proper and orthogonal"
            ),
            Self::SingularInertiaOperator => {
                write!(formatter, "bivector inertia operator is singular")
            }
            Self::NonFiniteResult => write!(
                formatter,
                "mass-property calculation produced a non-finite result"
            ),
        }
    }
}

impl Error for MassPropertiesError {}

#[derive(Clone, Debug, PartialEq)]
pub struct Bivector {
    dimension: usize,
    components: Vec<f64>,
}

impl Bivector {
    pub fn new(dimension: usize, components: Vec<f64>) -> Result<Self, MassPropertiesError> {
        validate_dimension(dimension)?;
        if components.len() != plane_count(dimension) {
            return Err(MassPropertiesError::DimensionMismatch);
        }
        if components.iter().any(|x| !x.is_finite()) {
            return Err(MassPropertiesError::NonFiniteInput);
        }
        Ok(Self {
            dimension,
            components,
        })
    }

    pub fn zero(dimension: usize) -> Result<Self, MassPropertiesError> {
        validate_dimension(dimension)?;
        Ok(Self {
            dimension,
            components: vec![0.0; plane_count(dimension)],
        })
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn components(&self) -> &[f64] {
        &self.components
    }

    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.components[plane_index(self.dimension, i, j)]
    }

    pub fn set(&mut self, i: usize, j: usize, value: f64) -> Result<(), MassPropertiesError> {
        if !value.is_finite() {
            return Err(MassPropertiesError::NonFiniteInput);
        }
        let index = plane_index(self.dimension, i, j);
        self.components[index] = value;
        Ok(())
    }

    pub fn to_matrix(&self) -> Matrix {
        (0..self.dimension)
            .map(|i| {
                (0..self.dimension)
                    .map(|j| {
                        if i < j {
                            self.get(i, j)
                        } else if i > j {
                            -self.get(j, i)
                        } else {
                            0.0
                        }
                    })
                    .collect()
            })
            .collect()
    }

    pub fn from_matrix(matrix: &Matrix) -> Result<Self, MassPropertiesError> {
        validate_square(matrix, matrix.len())?;
        let dimension = matrix.len();
        validate_dimension(dimension)?;
        let mut result = Self::zero(dimension)?;
        for (i, row) in matrix.iter().enumerate() {
            for (j, value) in row.iter().enumerate().skip(i + 1) {
                result.set(i, j, 0.5 * (value - matrix[j][i]))?;
            }
        }
        Ok(result)
    }
}

/// A rigid component's mass and second-moment tensor about its own center of mass.
///
/// local_second_moment is C_local = integral(r r^T dm), in kg m^2. The rotation
/// maps local component coordinates into the assembly frame; centroid is the
/// component center-of-mass position in the assembly frame.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentMassProperties {
    id: String,
    mass_kg: f64,
    centroid_in_assembly_m: Vector,
    local_to_assembly_rotation: Matrix,
    local_second_moment_kg_m2: Matrix,
}

impl ComponentMassProperties {
    pub fn new(
        id: impl Into<String>,
        mass_kg: f64,
        centroid_in_assembly_m: Vector,
        local_to_assembly_rotation: Matrix,
        local_second_moment_kg_m2: Matrix,
    ) -> Result<Self, MassPropertiesError> {
        let dimension = centroid_in_assembly_m.len();
        validate_dimension(dimension)?;
        if !mass_kg.is_finite() || centroid_in_assembly_m.iter().any(|x| !x.is_finite()) {
            return Err(MassPropertiesError::NonFiniteInput);
        }
        if mass_kg <= 0.0 {
            return Err(MassPropertiesError::NonPositiveMass);
        }
        validate_rotation(&local_to_assembly_rotation, dimension)?;
        validate_second_moment(&local_second_moment_kg_m2, dimension)?;

        Ok(Self {
            id: id.into(),
            mass_kg,
            centroid_in_assembly_m,
            local_to_assembly_rotation,
            local_second_moment_kg_m2,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn mass_kg(&self) -> f64 {
        self.mass_kg
    }

    pub fn centroid_in_assembly_m(&self) -> &[f64] {
        &self.centroid_in_assembly_m
    }

    pub fn local_second_moment_kg_m2(&self) -> &Matrix {
        &self.local_second_moment_kg_m2
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompoundMassProperties {
    dimension: usize,
    total_mass_kg: f64,
    center_of_mass_m: Vector,
    second_moment_at_com_kg_m2: Matrix,
}

impl CompoundMassProperties {
    /// Compose components using the full tensor parallel-axis theorem.
    ///
    /// C_total = sum(R_k C_k R_k^T + m_k d_k d_k^T), with d_k = r_k - c.
    pub fn compose(components: &[ComponentMassProperties]) -> Result<Self, MassPropertiesError> {
        if components.is_empty() {
            return Err(MassPropertiesError::InvalidDimension);
        }
        let dimension = components[0].centroid_in_assembly_m.len();
        validate_dimension(dimension)?;
        for component in components {
            if component.centroid_in_assembly_m.len() != dimension {
                return Err(MassPropertiesError::DimensionMismatch);
            }
        }

        let total_mass_kg = components.iter().map(|x| x.mass_kg).sum::<f64>();
        if !total_mass_kg.is_finite() || total_mass_kg <= 0.0 {
            return Err(MassPropertiesError::NonFiniteResult);
        }

        let mut center_of_mass_m = vec![0.0; dimension];
        for (axis, coordinate) in center_of_mass_m.iter_mut().enumerate() {
            for component in components {
                *coordinate += component.mass_kg * component.centroid_in_assembly_m[axis];
            }
            *coordinate /= total_mass_kg;
        }

        let mut second_moment_at_com_kg_m2 = zero_matrix(dimension);
        for component in components {
            let rotation = &component.local_to_assembly_rotation;
            let rotated_component_moment = multiply(
                &multiply(rotation, &component.local_second_moment_kg_m2)?,
                &transpose(rotation),
            )?;
            let displacement: Vector = (0..dimension)
                .map(|axis| component.centroid_in_assembly_m[axis] - center_of_mass_m[axis])
                .collect();

            for (row_index, row_values) in second_moment_at_com_kg_m2.iter_mut().enumerate() {
                for (column_index, value) in row_values.iter_mut().enumerate() {
                    *value += rotated_component_moment[row_index][column_index]
                        + component.mass_kg * displacement[row_index] * displacement[column_index];
                }
            }
        }

        validate_second_moment(&second_moment_at_com_kg_m2, dimension)?;
        Ok(Self {
            dimension,
            total_mass_kg,
            center_of_mass_m,
            second_moment_at_com_kg_m2,
        })
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn total_mass_kg(&self) -> f64 {
        self.total_mass_kg
    }

    pub fn center_of_mass_m(&self) -> &[f64] {
        &self.center_of_mass_m
    }

    pub fn second_moment_at_com_kg_m2(&self) -> &Matrix {
        &self.second_moment_at_com_kg_m2
    }

    /// Dense inertia operator on the lexicographic bivector basis.
    ///
    /// Each column applies I_C(Omega) = C Omega + Omega C to one unit plane
    /// generator. Off-diagonal C terms generate cross-plane coupling.
    pub fn bivector_inertia_operator(&self) -> Result<Matrix, MassPropertiesError> {
        let count = plane_count(self.dimension);
        let mut operator = vec![vec![0.0; count]; count];

        for column in 0..count {
            let mut basis = Bivector::zero(self.dimension)?;
            let (i, j) = plane_from_index(self.dimension, column);
            basis.set(i, j, 1.0)?;
            let omega = basis.to_matrix();
            let image = add(
                &multiply(&self.second_moment_at_com_kg_m2, &omega)?,
                &multiply(&omega, &self.second_moment_at_com_kg_m2)?,
            )?;
            let image_bivector = Bivector::from_matrix(&image)?;
            for (row_index, row_values) in operator.iter_mut().enumerate() {
                row_values[column] = image_bivector.components[row_index];
            }
        }
        Ok(operator)
    }

    pub fn apply_inverse_inertia(
        &self,
        angular_impulse: &Bivector,
    ) -> Result<Bivector, MassPropertiesError> {
        if angular_impulse.dimension != self.dimension {
            return Err(MassPropertiesError::DimensionMismatch);
        }
        let operator = self.bivector_inertia_operator()?;
        let solution = solve_linear(&operator, angular_impulse.components())?;
        Bivector::new(self.dimension, solution)
    }

    pub fn kinetic_energy(&self, angular_velocity: &Bivector) -> Result<f64, MassPropertiesError> {
        if angular_velocity.dimension != self.dimension {
            return Err(MassPropertiesError::DimensionMismatch);
        }
        let operator = self.bivector_inertia_operator()?;
        let applied = matrix_vector(&operator, angular_velocity.components())?;
        finite_result(
            0.5 * angular_velocity
                .components()
                .iter()
                .zip(applied)
                .map(|(w, momentum)| w * momentum)
                .sum::<f64>(),
        )
    }

    pub fn midpoint_mechanical_work(
        &self,
        angular_impulse: &Bivector,
        omega_before: &Bivector,
    ) -> Result<f64, MassPropertiesError> {
        if angular_impulse.dimension != self.dimension || omega_before.dimension != self.dimension {
            return Err(MassPropertiesError::DimensionMismatch);
        }
        let delta_omega = self.apply_inverse_inertia(angular_impulse)?;
        let mut midpoint = Vec::with_capacity(plane_count(self.dimension));
        for index in 0..plane_count(self.dimension) {
            midpoint.push(omega_before.components[index] + 0.5 * delta_omega.components[index]);
        }
        finite_result(
            angular_impulse
                .components
                .iter()
                .zip(midpoint)
                .map(|(impulse, omega_mid)| impulse * omega_mid)
                .sum(),
        )
    }
}

fn validate_dimension(dimension: usize) -> Result<(), MassPropertiesError> {
    if !(MIN_DIMENSION..=MAX_DIMENSION).contains(&dimension) {
        Err(MassPropertiesError::InvalidDimension)
    } else {
        Ok(())
    }
}

fn plane_count(dimension: usize) -> usize {
    dimension * (dimension - 1) / 2
}

fn plane_index(dimension: usize, i: usize, j: usize) -> usize {
    assert!(i < j && j < dimension, "requires i < j < dimension");
    i * dimension - i * (i + 1) / 2 + j - i - 1
}

fn plane_from_index(dimension: usize, mut target: usize) -> (usize, usize) {
    for i in 0..dimension {
        for j in (i + 1)..dimension {
            if target == 0 {
                return (i, j);
            }
            target -= 1;
        }
    }
    unreachable!("plane index is within the checked bivector dimension")
}

fn zero_matrix(dimension: usize) -> Matrix {
    vec![vec![0.0; dimension]; dimension]
}

fn validate_square(matrix: &Matrix, dimension: usize) -> Result<(), MassPropertiesError> {
    if matrix.len() != dimension || matrix.iter().any(|row| row.len() != dimension) {
        return Err(MassPropertiesError::DimensionMismatch);
    }
    if matrix.iter().flatten().any(|x| !x.is_finite()) {
        return Err(MassPropertiesError::NonFiniteInput);
    }
    Ok(())
}

fn validate_second_moment(matrix: &Matrix, dimension: usize) -> Result<(), MassPropertiesError> {
    validate_square(matrix, dimension)?;
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |a, b| a.max(b.abs()))
        .max(f64::MIN_POSITIVE);
    let tolerance = RELATIVE_TOLERANCE * scale;
    for (i, row) in matrix.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            if (value - matrix[j][i]).abs() > tolerance {
                return Err(MassPropertiesError::NonSymmetricSecondMoment);
            }
        }
    }

    let eigenvalues = symmetric_eigenvalues_jacobi(matrix)?;
    if eigenvalues.iter().any(|value| *value < -tolerance) {
        return Err(MassPropertiesError::NonPositiveSemidefiniteSecondMoment);
    }
    Ok(())
}

/// Symmetric Jacobi eigenvalue iteration, used only for M1 validation.
fn symmetric_eigenvalues_jacobi(matrix: &Matrix) -> Result<Vector, MassPropertiesError> {
    let dimension = matrix.len();
    let mut work = matrix.clone();
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |a, b| a.max(b.abs()))
        .max(f64::MIN_POSITIVE);
    let tolerance = 1.0e-14 * scale;

    for _ in 0..(100 * dimension * dimension) {
        let mut p = 0;
        let mut q = 1;
        let mut maximum = 0.0;
        for (i, row) in work.iter().enumerate() {
            for (j, value) in row.iter().enumerate().skip(i + 1) {
                if value.abs() > maximum {
                    maximum = value.abs();
                    p = i;
                    q = j;
                }
            }
        }
        if maximum <= tolerance {
            break;
        }

        let app = work[p][p];
        let aqq = work[q][q];
        let apq = work[p][q];
        let theta = 0.5 * (2.0 * apq).atan2(aqq - app);
        let cosine = theta.cos();
        let sine = theta.sin();

        let mut rotated_p_column = vec![0.0; dimension];
        let mut rotated_q_column = vec![0.0; dimension];
        for (k, row) in work.iter_mut().enumerate() {
            if k == p || k == q {
                continue;
            }
            let akp = row[p];
            let akq = row[q];
            let new_p = cosine * akp - sine * akq;
            let new_q = sine * akp + cosine * akq;
            row[p] = new_p;
            row[q] = new_q;
            rotated_p_column[k] = new_p;
            rotated_q_column[k] = new_q;
        }
        for (k, (new_p, new_q)) in rotated_p_column.iter().zip(&rotated_q_column).enumerate() {
            work[p][k] = *new_p;
            work[q][k] = *new_q;
        }

        work[p][p] = cosine * cosine * app - 2.0 * sine * cosine * apq + sine * sine * aqq;
        work[q][q] = sine * sine * app + 2.0 * sine * cosine * apq + cosine * cosine * aqq;
        work[p][q] = 0.0;
        work[q][p] = 0.0;
    }

    let values: Vector = (0..dimension).map(|i| work[i][i]).collect();
    if values.iter().all(|x| x.is_finite()) {
        Ok(values)
    } else {
        Err(MassPropertiesError::NonFiniteResult)
    }
}

fn validate_rotation(rotation: &Matrix, dimension: usize) -> Result<(), MassPropertiesError> {
    validate_square(rotation, dimension)?;
    let product = multiply(&transpose(rotation), rotation)?;
    for (i, row) in product.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            let expected = if i == j { 1.0 } else { 0.0 };
            if (value - expected).abs() > 1.0e-10 {
                return Err(MassPropertiesError::InvalidRotation);
            }
        }
    }
    if (determinant(rotation)? - 1.0).abs() > 1.0e-10 {
        return Err(MassPropertiesError::InvalidRotation);
    }
    Ok(())
}

fn transpose(matrix: &Matrix) -> Matrix {
    let dimension = matrix.len();
    let mut result = zero_matrix(dimension);
    for i in 0..dimension {
        for j in 0..dimension {
            result[i][j] = matrix[j][i];
        }
    }
    result
}

fn multiply(left: &Matrix, right: &Matrix) -> Result<Matrix, MassPropertiesError> {
    if left.is_empty() || right.is_empty() || left[0].len() != right.len() {
        return Err(MassPropertiesError::DimensionMismatch);
    }
    let rows = left.len();
    let inner = right.len();
    let columns = right[0].len();
    if left.iter().any(|row| row.len() != inner) || right.iter().any(|row| row.len() != columns) {
        return Err(MassPropertiesError::DimensionMismatch);
    }
    let mut result = vec![vec![0.0; columns]; rows];
    for i in 0..rows {
        for k in 0..inner {
            for j in 0..columns {
                result[i][j] += left[i][k] * right[k][j];
            }
        }
    }
    if result.iter().flatten().all(|x| x.is_finite()) {
        Ok(result)
    } else {
        Err(MassPropertiesError::NonFiniteResult)
    }
}

fn add(left: &Matrix, right: &Matrix) -> Result<Matrix, MassPropertiesError> {
    if left.len() != right.len() || left.iter().zip(right).any(|(a, b)| a.len() != b.len()) {
        return Err(MassPropertiesError::DimensionMismatch);
    }
    let mut result = left.clone();
    for i in 0..left.len() {
        for j in 0..left.len() {
            result[i][j] += right[i][j];
        }
    }
    if result.iter().flatten().all(|x| x.is_finite()) {
        Ok(result)
    } else {
        Err(MassPropertiesError::NonFiniteResult)
    }
}

fn determinant(matrix: &Matrix) -> Result<f64, MassPropertiesError> {
    let dimension = matrix.len();
    let mut work = matrix.clone();
    let mut determinant = 1.0;
    for column in 0..dimension {
        let mut pivot = column;
        for row in column + 1..dimension {
            if work[row][column].abs() > work[pivot][column].abs() {
                pivot = row;
            }
        }
        if work[pivot][column].abs() < 1.0e-15 {
            return Ok(0.0);
        }
        if pivot != column {
            work.swap(pivot, column);
            determinant = -determinant;
        }
        let diagonal = work[column][column];
        determinant *= diagonal;
        let pivot_row = work[column].clone();
        for row in work.iter_mut().skip(column + 1) {
            let factor = row[column] / diagonal;
            for (value, pivot_value) in row.iter_mut().zip(&pivot_row).skip(column + 1) {
                *value -= factor * pivot_value;
            }
        }
    }
    finite_result(determinant)
}

fn solve_linear(matrix: &Matrix, rhs: &[f64]) -> Result<Vector, MassPropertiesError> {
    let dimension = matrix.len();
    if rhs.len() != dimension || matrix.iter().any(|row| row.len() != dimension) {
        return Err(MassPropertiesError::DimensionMismatch);
    }
    let mut augmented = vec![vec![0.0; dimension + 1]; dimension];
    let scale = matrix.iter().flatten().fold(0.0_f64, |a, b| a.max(b.abs()));
    if !scale.is_finite() || scale == 0.0 {
        return Err(MassPropertiesError::SingularInertiaOperator);
    }
    for i in 0..dimension {
        for j in 0..dimension {
            augmented[i][j] = matrix[i][j];
        }
        augmented[i][dimension] = rhs[i];
    }

    for column in 0..dimension {
        let mut pivot = column;
        for row in column + 1..dimension {
            if augmented[row][column].abs() > augmented[pivot][column].abs() {
                pivot = row;
            }
        }
        if augmented[pivot][column].abs() <= 1.0e-14 * scale {
            return Err(MassPropertiesError::SingularInertiaOperator);
        }
        augmented.swap(pivot, column);
        let divisor = augmented[column][column];
        for value in augmented[column].iter_mut().skip(column) {
            *value /= divisor;
        }
        let pivot_row = augmented[column].clone();
        for (row_index, row_values) in augmented.iter_mut().enumerate() {
            if row_index == column {
                continue;
            }
            let factor = row_values[column];
            for (value, pivot_value) in row_values.iter_mut().zip(&pivot_row).skip(column) {
                *value -= factor * pivot_value;
            }
        }
    }
    let solution: Vector = (0..dimension).map(|i| augmented[i][dimension]).collect();
    if solution.iter().all(|x| x.is_finite()) {
        Ok(solution)
    } else {
        Err(MassPropertiesError::NonFiniteResult)
    }
}

fn matrix_vector(matrix: &Matrix, vector: &[f64]) -> Result<Vector, MassPropertiesError> {
    if matrix.iter().any(|row| row.len() != vector.len()) {
        return Err(MassPropertiesError::DimensionMismatch);
    }
    let output: Vector = matrix
        .iter()
        .map(|row| row.iter().zip(vector).map(|(a, b)| a * b).sum())
        .collect();
    if output.iter().all(|x| x.is_finite()) {
        Ok(output)
    } else {
        Err(MassPropertiesError::NonFiniteResult)
    }
}

fn finite_result(value: f64) -> Result<f64, MassPropertiesError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(MassPropertiesError::NonFiniteResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.15e}, expected {expected:.15e}"
        );
    }

    fn identity(dimension: usize) -> Matrix {
        let mut matrix = zero_matrix(dimension);
        for (index, row) in matrix.iter_mut().enumerate() {
            row[index] = 1.0;
        }
        matrix
    }

    fn sphere_component(id: &str, position: Vector) -> ComponentMassProperties {
        // A unit solid sphere of mass 1 kg has C = (1/5) I kg m^2.
        ComponentMassProperties::new(
            id,
            1.0,
            position,
            identity(3),
            vec![
                vec![0.2, 0.0, 0.0],
                vec![0.0, 0.2, 0.0],
                vec![0.0, 0.0, 0.2],
            ],
        )
        .expect("valid sphere")
    }

    #[test]
    fn single_component_preserves_mass_com_and_second_moment() {
        let component = sphere_component("one", vec![1.0, -2.0, 0.5]);
        let compound =
            CompoundMassProperties::compose(std::slice::from_ref(&component)).expect("compound");
        close(compound.total_mass_kg(), 1.0, 1.0e-12);
        for (actual, expected) in compound.center_of_mass_m().iter().zip([1.0, -2.0, 0.5]) {
            close(*actual, expected, 1.0e-12);
        }
        for i in 0..3 {
            for j in 0..3 {
                close(
                    compound.second_moment_at_com_kg_m2()[i][j],
                    component.local_second_moment_kg_m2()[i][j],
                    1.0e-12,
                );
            }
        }
    }

    #[test]
    fn two_offset_spheres_reproduce_parallel_axis_tensor() {
        let compound = CompoundMassProperties::compose(&[
            sphere_component("left", vec![0.0, 0.0, 0.0]),
            sphere_component("right", vec![2.0, 0.0, 0.0]),
        ])
        .expect("compound");
        close(compound.total_mass_kg(), 2.0, 1.0e-12);
        close(compound.center_of_mass_m()[0], 1.0, 1.0e-12);
        close(compound.center_of_mass_m()[1], 0.0, 1.0e-12);
        close(compound.center_of_mass_m()[2], 0.0, 1.0e-12);

        let c = compound.second_moment_at_com_kg_m2();
        close(c[0][0], 2.4, 1.0e-12);
        close(c[1][1], 0.4, 1.0e-12);
        close(c[2][2], 0.4, 1.0e-12);
        close(c[0][1], 0.0, 1.0e-12);

        let operator = compound.bivector_inertia_operator().expect("operator");
        close(operator[0][0], 2.8, 1.0e-12);
        close(operator[1][1], 2.8, 1.0e-12);
        close(operator[2][2], 0.8, 1.0e-12);
    }

    #[test]
    fn component_rotation_preserves_products_of_inertia() {
        let rotation = vec![
            vec![
                std::f64::consts::FRAC_1_SQRT_2,
                -std::f64::consts::FRAC_1_SQRT_2,
                0.0,
            ],
            vec![
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
                0.0,
            ],
            vec![0.0, 0.0, 1.0],
        ];
        let local = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 2.0, 0.0],
            vec![0.0, 0.0, 3.0],
        ];
        let component = ComponentMassProperties::new(
            "rotated-anisotropic",
            2.0,
            vec![0.0, 0.0, 0.0],
            rotation,
            local,
        )
        .expect("component");
        let compound = CompoundMassProperties::compose(&[component]).expect("compound");
        close(compound.second_moment_at_com_kg_m2()[0][0], 1.5, 1.0e-12);
        close(compound.second_moment_at_com_kg_m2()[1][1], 1.5, 1.0e-12);
        close(compound.second_moment_at_com_kg_m2()[0][1], -0.5, 1.0e-12);
    }

    #[test]
    fn component_order_does_not_change_compound_mass_properties() {
        let a = sphere_component("a", vec![-1.0, 0.2, 0.0]);
        let b = sphere_component("b", vec![2.0, -0.1, 0.5]);
        let first = CompoundMassProperties::compose(&[a.clone(), b.clone()]).expect("first");
        let second = CompoundMassProperties::compose(&[b, a]).expect("second");
        close(first.total_mass_kg(), second.total_mass_kg(), 1.0e-12);
        for axis in 0..3 {
            close(
                first.center_of_mass_m()[axis],
                second.center_of_mass_m()[axis],
                1.0e-12,
            );
            for column in 0..3 {
                close(
                    first.second_moment_at_com_kg_m2()[axis][column],
                    second.second_moment_at_com_kg_m2()[axis][column],
                    1.0e-12,
                );
            }
        }
    }

    #[test]
    fn dense_bivector_operator_matches_direct_tensor_action() {
        let rotation = vec![
            vec![
                std::f64::consts::FRAC_1_SQRT_2,
                -std::f64::consts::FRAC_1_SQRT_2,
                0.0,
            ],
            vec![
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
                0.0,
            ],
            vec![0.0, 0.0, 1.0],
        ];
        let component = ComponentMassProperties::new(
            "anisotropic",
            1.0,
            vec![0.0, 0.0, 0.0],
            rotation,
            vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 2.0, 0.0],
                vec![0.0, 0.0, 3.0],
            ],
        )
        .expect("component");
        let compound = CompoundMassProperties::compose(&[component]).expect("compound");
        let omega = Bivector::new(3, vec![0.3, -0.7, 1.1]).expect("omega");
        let operator = compound.bivector_inertia_operator().expect("operator");
        let generalized = matrix_vector(&operator, omega.components()).expect("matrix action");

        let omega_matrix = omega.to_matrix();
        let c = compound.second_moment_at_com_kg_m2();
        let direct = add(
            &multiply(c, &omega_matrix).expect("C Omega"),
            &multiply(&omega_matrix, c).expect("Omega C"),
        )
        .expect("sum");
        let direct_bivector = Bivector::from_matrix(&direct).expect("direct");
        for (actual, expected) in generalized.iter().zip(direct_bivector.components()) {
            close(*actual, *expected, 1.0e-12);
        }
    }

    #[test]
    fn midpoint_work_matches_exact_dense_operator_energy_change() {
        let compound = CompoundMassProperties::compose(&[
            sphere_component("a", vec![-0.5, 0.0, 0.0]),
            sphere_component("b", vec![0.5, 0.0, 0.0]),
        ])
        .expect("compound");
        let omega_before = Bivector::new(3, vec![0.2, -0.4, 0.8]).expect("omega");
        let impulse = Bivector::new(3, vec![1.0, -2.0, 0.5]).expect("impulse");
        let delta = compound.apply_inverse_inertia(&impulse).expect("delta");
        let mut omega_after = omega_before.clone();
        for i in 0..3 {
            for j in i + 1..3 {
                omega_after
                    .set(i, j, omega_before.get(i, j) + delta.get(i, j))
                    .expect("set");
            }
        }
        let energy_delta = compound.kinetic_energy(&omega_after).expect("after")
            - compound.kinetic_energy(&omega_before).expect("before");
        let work = compound
            .midpoint_mechanical_work(&impulse, &omega_before)
            .expect("work");
        close(work, energy_delta, 1.0e-11);
    }

    #[test]
    fn rejects_invalid_mass_tensor_and_transform_inputs() {
        assert_eq!(
            ComponentMassProperties::new("negative", -1.0, vec![0.0; 3], identity(3), identity(3)),
            Err(MassPropertiesError::NonPositiveMass)
        );
        assert_eq!(
            ComponentMassProperties::new(
                "bad-symmetry",
                1.0,
                vec![0.0; 3],
                identity(3),
                vec![
                    vec![1.0, 0.3, 0.0],
                    vec![0.0, 1.0, 0.0],
                    vec![0.0, 0.0, 1.0]
                ]
            ),
            Err(MassPropertiesError::NonSymmetricSecondMoment)
        );
        assert_eq!(
            ComponentMassProperties::new(
                "bad-psd",
                1.0,
                vec![0.0; 3],
                identity(3),
                vec![
                    vec![1.0, 0.0, 0.0],
                    vec![0.0, -0.1, 0.0],
                    vec![0.0, 0.0, 1.0]
                ]
            ),
            Err(MassPropertiesError::NonPositiveSemidefiniteSecondMoment)
        );
        let reflection = vec![
            vec![-1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        assert_eq!(
            ComponentMassProperties::new("reflection", 1.0, vec![0.0; 3], reflection, identity(3)),
            Err(MassPropertiesError::InvalidRotation)
        );
    }
}
