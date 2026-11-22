//! Orientation-aware principal-axis inertia reference on N-D bivectors.
//!
//! This M1 oracle assumes the input vector contains physical principal moments
//! about the local body axes. It reconstructs the diagonal mass second-moment
//! tensor and the induced inertia operator on antisymmetric generators. It is
//! not a production rigid-body solver.

use std::error::Error;
use std::fmt::{Display, Formatter};

const MIN_DIMENSION: usize = 2;
const MAX_DIMENSION: usize = 9;
const RELATIVE_TOLERANCE: f64 = 1.0e-12;

pub type Matrix = Vec<Vec<f64>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InertiaError {
    InvalidDimension,
    DimensionMismatch,
    NonFiniteInput,
    NonPositivePrincipalMoment,
    NonRealizablePrincipalMoments,
    SingularPlaneInertia,
    InvalidRotation,
    NonFiniteResult,
}

impl Display for InertiaError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDimension => write!(formatter, "dimension must be in 2..=9"),
            Self::DimensionMismatch => {
                write!(formatter, "matrix or bivector dimensions do not match")
            }
            Self::NonFiniteInput => write!(formatter, "input values must be finite"),
            Self::NonPositivePrincipalMoment => {
                write!(formatter, "principal moments must be positive")
            }
            Self::NonRealizablePrincipalMoments => write!(
                formatter,
                "principal moments do not define a nonnegative second-moment tensor"
            ),
            Self::SingularPlaneInertia => {
                write!(formatter, "one or more plane inertias are non-positive")
            }
            Self::InvalidRotation => write!(
                formatter,
                "rotation matrix must be a proper orthogonal matrix"
            ),
            Self::NonFiniteResult => {
                write!(formatter, "inertia operator produced a non-finite result")
            }
        }
    }
}

impl Error for InertiaError {}

/// Bivector coefficients in lexicographic plane order (0,1), (0,2), ...
#[derive(Clone, Debug, PartialEq)]
pub struct Bivector {
    dimension: usize,
    components: Vec<f64>,
}

impl Bivector {
    pub fn new(dimension: usize, components: Vec<f64>) -> Result<Self, InertiaError> {
        validate_dimension(dimension)?;
        if components.len() != dimension * (dimension - 1) / 2 {
            return Err(InertiaError::DimensionMismatch);
        }
        if components.iter().any(|value| !value.is_finite()) {
            return Err(InertiaError::NonFiniteInput);
        }
        Ok(Self {
            dimension,
            components,
        })
    }

    pub fn zero(dimension: usize) -> Result<Self, InertiaError> {
        validate_dimension(dimension)?;
        Ok(Self {
            dimension,
            components: vec![0.0; dimension * (dimension - 1) / 2],
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

    pub fn set(&mut self, i: usize, j: usize, value: f64) -> Result<(), InertiaError> {
        if !value.is_finite() {
            return Err(InertiaError::NonFiniteInput);
        }
        let index = plane_index(self.dimension, i, j);
        self.components[index] = value;
        Ok(())
    }

    /// Convert to the antisymmetric matrix used by the rotor Lie algebra.
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

    /// Recover the antisymmetric part of a square matrix as a bivector.
    pub fn from_matrix(matrix: &Matrix) -> Result<Self, InertiaError> {
        validate_square_matrix(matrix)?;
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

    pub fn inner_product(&self, other: &Self) -> Result<f64, InertiaError> {
        if self.dimension != other.dimension {
            return Err(InertiaError::DimensionMismatch);
        }
        finite_result(
            self.components
                .iter()
                .zip(&other.components)
                .map(|(left, right)| left * right)
                .sum::<f64>(),
        )
    }
}

/// Principal-axis inertia operator for a declared local body frame.
///
/// If C is the diagonal mass second-moment tensor, principal moments satisfy
/// I_i = trace(C) - C_i. The operator on an angular-velocity generator Omega
/// is I_C(Omega) = C Omega + Omega C. On plane (i,j), its eigenvalue is
/// J_ij = C_i + C_j.
#[derive(Clone, Debug, PartialEq)]
pub struct PrincipalInertia {
    dimension: usize,
    principal_moments: Vec<f64>,
    second_moments: Vec<f64>,
    plane_moments: Vec<f64>,
}

impl PrincipalInertia {
    /// Construct from principal moments about local body-frame axes.
    ///
    /// Requires C_i = sum(I)/(D-1) - I_i >= 0 and positive inertia on every
    /// represented bivector plane.
    pub fn new(principal_moments: Vec<f64>) -> Result<Self, InertiaError> {
        let dimension = principal_moments.len();
        validate_dimension(dimension)?;
        if principal_moments.iter().any(|value| !value.is_finite()) {
            return Err(InertiaError::NonFiniteInput);
        }
        if principal_moments.iter().any(|value| *value <= 0.0) {
            return Err(InertiaError::NonPositivePrincipalMoment);
        }

        let total = principal_moments.iter().sum::<f64>();
        if !total.is_finite() {
            return Err(InertiaError::NonFiniteResult);
        }
        let trace_second_moment = total / (dimension - 1) as f64;
        let scale = trace_second_moment.abs().max(f64::MIN_POSITIVE);
        let tolerance = RELATIVE_TOLERANCE * scale;
        let mut second_moments = Vec::with_capacity(dimension);

        for principal in &principal_moments {
            let value = trace_second_moment - principal;
            if value < -tolerance {
                return Err(InertiaError::NonRealizablePrincipalMoments);
            }
            second_moments.push(value.max(0.0));
        }

        let mut plane_moments = Vec::with_capacity(dimension * (dimension - 1) / 2);
        for i in 0..dimension {
            for j in (i + 1)..dimension {
                let plane = second_moments[i] + second_moments[j];
                if !plane.is_finite() {
                    return Err(InertiaError::NonFiniteResult);
                }
                if plane <= tolerance {
                    return Err(InertiaError::SingularPlaneInertia);
                }
                plane_moments.push(plane);
            }
        }

        Ok(Self {
            dimension,
            principal_moments,
            second_moments,
            plane_moments,
        })
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn principal_moments(&self) -> &[f64] {
        &self.principal_moments
    }

    pub fn second_moments(&self) -> &[f64] {
        &self.second_moments
    }

    pub fn plane_inertia(&self, i: usize, j: usize) -> f64 {
        self.plane_moments[plane_index(self.dimension, i, j)]
    }

    /// Apply the inverse inertia operator in the body principal frame.
    pub fn apply_inverse_body(&self, impulse: &Bivector) -> Result<Bivector, InertiaError> {
        if impulse.dimension != self.dimension {
            return Err(InertiaError::DimensionMismatch);
        }
        let mut response = Bivector::zero(self.dimension)?;
        for i in 0..self.dimension {
            for j in (i + 1)..self.dimension {
                response.set(i, j, impulse.get(i, j) / self.plane_inertia(i, j))?;
            }
        }
        Ok(response)
    }

    /// Transform a world-frame impulse into the principal frame, apply the
    /// diagonal plane operator, then transform the angular response to world.
    pub fn apply_inverse_world(
        &self,
        body_to_world_rotation: &Matrix,
        impulse_world: &Bivector,
    ) -> Result<Bivector, InertiaError> {
        validate_rotation(body_to_world_rotation, self.dimension)?;
        if impulse_world.dimension != self.dimension {
            return Err(InertiaError::DimensionMismatch);
        }

        let rotation_transpose = transpose(body_to_world_rotation);
        let impulse_body_matrix = multiply(
            &multiply(&rotation_transpose, &impulse_world.to_matrix())?,
            body_to_world_rotation,
        )?;
        let impulse_body = Bivector::from_matrix(&impulse_body_matrix)?;
        let response_body = self.apply_inverse_body(&impulse_body)?;
        let response_world_matrix = multiply(
            &multiply(body_to_world_rotation, &response_body.to_matrix())?,
            &rotation_transpose,
        )?;
        Bivector::from_matrix(&response_world_matrix)
    }

    /// Rotational kinetic energy for a world-frame angular-velocity generator.
    ///
    /// The body-frame formula is 1/2 sum_(i<j) J_ij * Omega_ij^2.
    pub fn kinetic_energy_world(
        &self,
        body_to_world_rotation: &Matrix,
        angular_velocity_world: &Bivector,
    ) -> Result<f64, InertiaError> {
        validate_rotation(body_to_world_rotation, self.dimension)?;
        if angular_velocity_world.dimension != self.dimension {
            return Err(InertiaError::DimensionMismatch);
        }

        let rotation_transpose = transpose(body_to_world_rotation);
        let omega_body_matrix = multiply(
            &multiply(&rotation_transpose, &angular_velocity_world.to_matrix())?,
            body_to_world_rotation,
        )?;
        let omega_body = Bivector::from_matrix(&omega_body_matrix)?;

        let mut energy = 0.0;
        for i in 0..self.dimension {
            for j in (i + 1)..self.dimension {
                energy += 0.5 * self.plane_inertia(i, j) * omega_body.get(i, j).powi(2);
            }
        }
        finite_result(energy)
    }

    /// Discrete mechanical work of a world-frame angular impulse.
    pub fn midpoint_mechanical_work(
        &self,
        body_to_world_rotation: &Matrix,
        impulse_world: &Bivector,
        omega_before_world: &Bivector,
    ) -> Result<f64, InertiaError> {
        if omega_before_world.dimension != self.dimension {
            return Err(InertiaError::DimensionMismatch);
        }
        let delta_omega = self.apply_inverse_world(body_to_world_rotation, impulse_world)?;
        let mut omega_after = omega_before_world.clone();
        for i in 0..self.dimension {
            for j in (i + 1)..self.dimension {
                omega_after.set(i, j, omega_before_world.get(i, j) + delta_omega.get(i, j))?;
            }
        }
        impulse_world.inner_product(&midpoint(&omega_after, omega_before_world)?)
    }
}

/// Apply a proper basis rotation to a bivector using R B R^T.
pub fn rotate_bivector(rotation: &Matrix, bivector: &Bivector) -> Result<Bivector, InertiaError> {
    validate_rotation(rotation, bivector.dimension)?;
    let rotated = multiply(
        &multiply(rotation, &bivector.to_matrix())?,
        &transpose(rotation),
    )?;
    Bivector::from_matrix(&rotated)
}

fn midpoint(left: &Bivector, right: &Bivector) -> Result<Bivector, InertiaError> {
    if left.dimension != right.dimension {
        return Err(InertiaError::DimensionMismatch);
    }
    let mut result = Bivector::zero(left.dimension)?;
    for i in 0..left.dimension {
        for j in (i + 1)..left.dimension {
            result.set(i, j, 0.5 * (left.get(i, j) + right.get(i, j)))?;
        }
    }
    Ok(result)
}

fn plane_index(dimension: usize, i: usize, j: usize) -> usize {
    assert!(i < j && j < dimension, "requires i < j < dimension");
    i * dimension - i * (i + 1) / 2 + j - i - 1
}

fn validate_dimension(dimension: usize) -> Result<(), InertiaError> {
    if !(MIN_DIMENSION..=MAX_DIMENSION).contains(&dimension) {
        Err(InertiaError::InvalidDimension)
    } else {
        Ok(())
    }
}

fn validate_square_matrix(matrix: &Matrix) -> Result<(), InertiaError> {
    let dimension = matrix.len();
    if matrix.iter().any(|row| row.len() != dimension) {
        return Err(InertiaError::DimensionMismatch);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(InertiaError::NonFiniteInput);
    }
    Ok(())
}

fn validate_rotation(rotation: &Matrix, dimension: usize) -> Result<(), InertiaError> {
    validate_square_matrix(rotation)?;
    if rotation.len() != dimension {
        return Err(InertiaError::DimensionMismatch);
    }

    let product = multiply(&transpose(rotation), rotation)?;
    for (i, row) in product.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            let expected = if i == j { 1.0 } else { 0.0 };
            if (value - expected).abs() > 1.0e-10 {
                return Err(InertiaError::InvalidRotation);
            }
        }
    }
    if (determinant(rotation)? - 1.0).abs() > 1.0e-10 {
        return Err(InertiaError::InvalidRotation);
    }
    Ok(())
}

fn zero_matrix(dimension: usize) -> Matrix {
    vec![vec![0.0; dimension]; dimension]
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

fn multiply(left: &Matrix, right: &Matrix) -> Result<Matrix, InertiaError> {
    if left.is_empty() || right.is_empty() || left[0].len() != right.len() {
        return Err(InertiaError::DimensionMismatch);
    }
    let rows = left.len();
    let inner = right.len();
    let columns = right[0].len();
    if left.iter().any(|row| row.len() != inner) || right.iter().any(|row| row.len() != columns) {
        return Err(InertiaError::DimensionMismatch);
    }
    let mut result = vec![vec![0.0; columns]; rows];
    for i in 0..rows {
        for k in 0..inner {
            for j in 0..columns {
                result[i][j] += left[i][k] * right[k][j];
            }
        }
    }
    if result.iter().flatten().all(|value| value.is_finite()) {
        Ok(result)
    } else {
        Err(InertiaError::NonFiniteResult)
    }
}

fn determinant(matrix: &Matrix) -> Result<f64, InertiaError> {
    validate_square_matrix(matrix)?;
    let dimension = matrix.len();
    let mut work = matrix.clone();
    let mut result = 1.0;
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
            result = -result;
        }
        let diagonal = work[column][column];
        result *= diagonal;
        let pivot_row = work[column].clone();
        for row in work.iter_mut().skip(column + 1) {
            let factor = row[column] / diagonal;
            for (value, pivot_value) in row.iter_mut().zip(&pivot_row).skip(column + 1) {
                *value -= factor * pivot_value;
            }
        }
    }
    finite_result(result)
}

fn finite_result(value: f64) -> Result<f64, InertiaError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(InertiaError::NonFiniteResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_matrix(dimension: usize) -> Result<Matrix, InertiaError> {
        validate_dimension(dimension)?;
        let mut matrix = zero_matrix(dimension);
        for (index, row) in matrix.iter_mut().enumerate() {
            row[index] = 1.0;
        }
        Ok(matrix)
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.15e}, expected {expected:.15e}, tolerance {tolerance:.3e}"
        );
    }

    fn direct_transpose(matrix: &Matrix) -> Matrix {
        let n = matrix.len();
        let mut output = vec![vec![0.0; n]; n];
        for row in 0..n {
            for column in 0..n {
                output[column][row] = matrix[row][column];
            }
        }
        output
    }

    fn direct_product(left: &Matrix, right: &Matrix) -> Matrix {
        let rows = left.len();
        let inner = right.len();
        let columns = right[0].len();
        let mut output = vec![vec![0.0; columns]; rows];
        for row in 0..rows {
            for column in 0..columns {
                let mut value = 0.0;
                for k in 0..inner {
                    value += left[row][k] * right[k][column];
                }
                output[row][column] = value;
            }
        }
        output
    }

    fn rotate_3d_xz_quarter_turn() -> Matrix {
        vec![
            vec![0.0, 0.0, -1.0],
            vec![0.0, 1.0, 0.0],
            vec![1.0, 0.0, 0.0],
        ]
    }

    #[test]
    fn isotropic_three_dimensional_body_is_isotropic_on_every_plane() {
        let inertia = PrincipalInertia::new(vec![2.0, 2.0, 2.0]).expect("sphere");
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            close(inertia.plane_inertia(i, j), 2.0, 1.0e-12);
        }

        let impulse = Bivector::new(3, vec![4.0, -2.0, 6.0]).expect("impulse");
        let response = inertia
            .apply_inverse_world(&identity_matrix(3).expect("identity"), &impulse)
            .expect("response");
        for (actual, expected) in response.components().iter().zip([2.0, -1.0, 3.0]) {
            close(*actual, expected, 1.0e-12);
        }
    }

    #[test]
    fn asymmetric_three_dimensional_principal_moments_use_complementary_axis() {
        let inertia = PrincipalInertia::new(vec![3.0, 4.0, 5.0]).expect("realizable");
        for (i, j, expected) in [(0, 1, 5.0), (0, 2, 4.0), (1, 2, 3.0)] {
            close(inertia.plane_inertia(i, j), expected, 1.0e-12);
        }

        let impulse = Bivector::new(3, vec![10.0, 8.0, 6.0]).expect("impulse");
        let response = inertia
            .apply_inverse_world(&identity_matrix(3).expect("identity"), &impulse)
            .expect("response");
        for (actual, expected) in response.components().iter().zip([2.0, 2.0, 2.0]) {
            close(*actual, expected, 1.0e-12);
        }
    }

    #[test]
    fn two_dimensional_single_plane_known_answer() {
        let inertia = PrincipalInertia::new(vec![2.0, 3.0]).expect("2D inertia");
        close(inertia.second_moments()[0], 3.0, 1.0e-12);
        close(inertia.second_moments()[1], 2.0, 1.0e-12);
        close(inertia.plane_inertia(0, 1), 5.0, 1.0e-12);

        let impulse = Bivector::new(2, vec![15.0]).expect("impulse");
        let response = inertia
            .apply_inverse_world(&identity_matrix(2).expect("identity"), &impulse)
            .expect("response");
        close(response.get(0, 1), 3.0, 1.0e-12);
    }

    #[test]
    fn four_dimensional_multi_plane_operator_has_known_answers() {
        let inertia = PrincipalInertia::new(vec![4.0, 5.0, 6.0, 7.0]).expect("4D inertia");
        close(inertia.plane_inertia(0, 1), 17.0 / 3.0, 1.0e-12);
        close(inertia.plane_inertia(0, 2), 14.0 / 3.0, 1.0e-12);
        close(inertia.plane_inertia(2, 3), 5.0 / 3.0, 1.0e-12);

        let impulse = Bivector::new(4, vec![17.0 / 3.0, 14.0 / 3.0, 0.0, 0.0, 0.0, 5.0 / 3.0])
            .expect("multi-plane impulse");
        let response = inertia
            .apply_inverse_world(&identity_matrix(4).expect("identity"), &impulse)
            .expect("response");
        for (actual, expected) in response
            .components()
            .iter()
            .zip([1.0, 1.0, 0.0, 0.0, 0.0, 1.0])
        {
            close(*actual, expected, 1.0e-12);
        }
    }

    #[test]
    fn rotated_anisotropic_response_matches_independent_direct_matrix_reference() {
        let inertia = PrincipalInertia::new(vec![3.0, 4.0, 5.0]).expect("realizable");
        let rotation = rotate_3d_xz_quarter_turn();
        let impulse = Bivector::new(3, vec![1.0, 2.0, -3.0]).expect("impulse");
        let actual = inertia
            .apply_inverse_world(&rotation, &impulse)
            .expect("rotated response");

        // Independent direct matrix reference: R^T B R, solve each plane
        // using the derived eigenvalues (5,4,3), then R response R^T.
        let rotation_transpose = direct_transpose(&rotation);
        let body_impulse = direct_product(
            &direct_product(&rotation_transpose, &impulse.to_matrix()),
            &rotation,
        );
        let mut body_response = zero_matrix(3);
        for (i, j, plane) in [(0, 1, 5.0), (0, 2, 4.0), (1, 2, 3.0)] {
            let value = body_impulse[i][j] / plane;
            body_response[i][j] = value;
            body_response[j][i] = -value;
        }
        let direct_world = direct_product(
            &direct_product(&rotation, &body_response),
            &rotation_transpose,
        );
        let expected = Bivector::from_matrix(&direct_world).expect("direct bivector");

        for (actual, expected) in actual.components().iter().zip(expected.components()) {
            close(*actual, *expected, 1.0e-12);
        }
        // The quarter-turn rotation produces an explicit world-frame response
        // [1/3, 1/2, -3/5] for this impulse and principal-moment fixture.
        for (actual, expected) in actual.components().iter().zip([1.0 / 3.0, 0.5, -0.6]) {
            close(*actual, expected, 1.0e-12);
        }
    }

    #[test]
    fn bivector_rotation_adjoint_round_trip_preserves_plane_coordinates() {
        let rotation = rotate_3d_xz_quarter_turn();
        let original = Bivector::new(3, vec![0.25, -0.75, 1.5]).expect("original");
        let rotated = rotate_bivector(&rotation, &original).expect("rotated");
        let recovered = rotate_bivector(&direct_transpose(&rotation), &rotated).expect("recovered");
        for (actual, expected) in recovered.components().iter().zip(original.components()) {
            close(*actual, *expected, 1.0e-12);
        }
    }

    #[test]
    fn midpoint_work_equals_exact_rotational_kinetic_energy_change() {
        let inertia = PrincipalInertia::new(vec![3.0, 4.0, 5.0]).expect("realizable");
        let rotation = rotate_3d_xz_quarter_turn();
        let impulse = Bivector::new(3, vec![1.5, -2.0, 0.75]).expect("impulse");
        let omega_before = Bivector::new(3, vec![0.2, -0.4, 0.8]).expect("omega");
        let response = inertia
            .apply_inverse_world(&rotation, &impulse)
            .expect("response");

        let mut omega_after = omega_before.clone();
        for i in 0..3 {
            for j in (i + 1)..3 {
                omega_after
                    .set(i, j, omega_before.get(i, j) + response.get(i, j))
                    .expect("set");
            }
        }

        let work = inertia
            .midpoint_mechanical_work(&rotation, &impulse, &omega_before)
            .expect("work");
        let energy_before = inertia
            .kinetic_energy_world(&rotation, &omega_before)
            .expect("energy before");
        let energy_after = inertia
            .kinetic_energy_world(&rotation, &omega_after)
            .expect("energy after");
        close(work, energy_after - energy_before, 1.0e-12);
    }

    #[test]
    fn kinetic_energy_is_invariant_under_body_frame_rotation() {
        let inertia = PrincipalInertia::new(vec![3.0, 4.0, 5.0]).expect("realizable");
        let rotation = rotate_3d_xz_quarter_turn();
        let omega_body = Bivector::new(3, vec![0.2, -0.4, 0.8]).expect("omega");
        let omega_world = rotate_bivector(&rotation, &omega_body).expect("rotated omega");

        let energy_body_frame = inertia
            .kinetic_energy_world(&identity_matrix(3).expect("identity"), &omega_body)
            .expect("body-frame energy");
        let energy_world_frame = inertia
            .kinetic_energy_world(&rotation, &omega_world)
            .expect("world-frame energy");
        close(energy_body_frame, energy_world_frame, 1.0e-12);
    }

    #[test]
    fn reflected_basis_is_not_accepted_as_a_physical_rotation() {
        let inertia = PrincipalInertia::new(vec![3.0, 4.0, 5.0]).expect("inertia");
        let reflection = vec![
            vec![-1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let impulse = Bivector::new(3, vec![1.0, 0.0, 0.0]).expect("impulse");
        assert_eq!(
            inertia.apply_inverse_world(&reflection, &impulse),
            Err(InertiaError::InvalidRotation)
        );
    }

    #[test]
    fn invalid_and_non_realizable_principal_moments_are_rejected() {
        assert_eq!(
            PrincipalInertia::new(vec![1.0]),
            Err(InertiaError::InvalidDimension)
        );
        assert_eq!(
            PrincipalInertia::new(vec![1.0, f64::NAN]),
            Err(InertiaError::NonFiniteInput)
        );
        assert_eq!(
            PrincipalInertia::new(vec![0.0, 1.0, 1.0]),
            Err(InertiaError::NonPositivePrincipalMoment)
        );
        assert_eq!(
            PrincipalInertia::new(vec![1.0, 1.0, 10.0]),
            Err(InertiaError::NonRealizablePrincipalMoments)
        );
    }

    #[test]
    fn world_kinetic_energy_matches_direct_rotated_second_moment_trace() {
        let inertia = PrincipalInertia::new(vec![3.0, 4.0, 5.0]).expect("realizable");
        let rotation = rotate_3d_xz_quarter_turn();
        let omega_world = Bivector::new(3, vec![0.3, -0.7, 1.1]).expect("omega");
        let energy = inertia
            .kinetic_energy_world(&rotation, &omega_world)
            .expect("operator energy");

        let c_body = vec![
            vec![3.0, 0.0, 0.0],
            vec![0.0, 2.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let c_world = direct_product(
            &direct_product(&rotation, &c_body),
            &direct_transpose(&rotation),
        );
        let omega_matrix = omega_world.to_matrix();
        let omega_t_omega = direct_product(&direct_transpose(&omega_matrix), &omega_matrix);
        let mut direct_trace = 0.0;
        for i in 0..3 {
            for j in 0..3 {
                direct_trace += omega_t_omega[i][j] * c_world[j][i];
            }
        }
        let direct_energy = 0.5 * direct_trace;
        close(energy, direct_energy, 1.0e-12);
    }

    #[test]
    fn midpoint_work_rejects_a_mismatched_pre_impulse_velocity_dimension() {
        let inertia = PrincipalInertia::new(vec![3.0, 4.0, 5.0]).expect("inertia");
        let identity = identity_matrix(3).expect("identity");
        let impulse = Bivector::new(3, vec![1.0, 0.0, 0.0]).expect("impulse");
        let wrong_omega = Bivector::new(2, vec![0.0]).expect("wrong dimension");
        assert_eq!(
            inertia.midpoint_mechanical_work(&identity, &impulse, &wrong_omega),
            Err(InertiaError::DimensionMismatch)
        );
    }

    #[test]
    fn invalid_rotations_and_bivector_dimensions_are_rejected() {
        let inertia = PrincipalInertia::new(vec![3.0, 4.0, 5.0]).expect("inertia");
        let impulse = Bivector::new(3, vec![1.0, 0.0, 0.0]).expect("impulse");
        let invalid_rotation = vec![
            vec![2.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        assert_eq!(
            inertia.apply_inverse_world(&invalid_rotation, &impulse),
            Err(InertiaError::InvalidRotation)
        );
        let wrong_dimension = Bivector::new(2, vec![1.0]).expect("2D impulse");
        assert_eq!(
            inertia.apply_inverse_world(&identity_matrix(3).expect("identity"), &wrong_dimension),
            Err(InertiaError::DimensionMismatch)
        );
    }
}
