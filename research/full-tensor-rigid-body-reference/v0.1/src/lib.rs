//! Independent 3D rigid-body reference for full-tensor rotational dynamics.
//!
//! This crate is a research oracle, not the production Symtropy integrator.
//! It uses body-frame angular velocity and inertia, a body-to-world unit
//! quaternion, Euler's rigid-body equation, and a classical RK4 reference step.
//! The quaternion is normalized after each accepted step; RK4 is not claimed
//! to be a geometric or symplectic integrator.

#![forbid(unsafe_code)]
#![allow(clippy::needless_range_loop)]

use std::error::Error;
use std::fmt;

const TENSOR_REL_TOL: f64 = 1.0e-12;

/// A row-major, symmetric positive-definite inertia tensor in body coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InertiaTensor {
    matrix: [[f64; 3]; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TensorError {
    NonFinite,
    NotSymmetric,
    NotPositiveDefinite,
    TriangleInequality,
}

impl fmt::Display for TensorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => write!(f, "inertia tensor contains a non-finite value"),
            Self::NotSymmetric => write!(f, "inertia tensor is not symmetric"),
            Self::NotPositiveDefinite => write!(f, "inertia tensor is not positive definite"),
            Self::TriangleInequality => {
                write!(f, "principal moments violate the rigid-body triangle inequality")
            }
        }
    }
}

impl Error for TensorError {}

impl InertiaTensor {
    /// Construct a physical rigid-body inertia tensor.
    ///
    /// Positive definiteness alone is insufficient: a realizable mass
    /// distribution also requires the principal moments to obey the triangle
    /// inequalities. Invalid values are rejected, never clipped.
    pub fn new(matrix: [[f64; 3]; 3]) -> Result<Self, TensorError> {
        if matrix.iter().flatten().any(|value| !value.is_finite()) {
            return Err(TensorError::NonFinite);
        }

        let scale = matrix
            .iter()
            .flatten()
            .fold(0.0_f64, |current, value| current.max(value.abs()));
        let symmetry_tolerance = TENSOR_REL_TOL * (1.0 + scale);
        let mut matrix = matrix;
        for i in 0..3 {
            for j in (i + 1)..3 {
                if (matrix[i][j] - matrix[j][i]).abs() > symmetry_tolerance {
                    return Err(TensorError::NotSymmetric);
                }
                // Canonicalize tolerated roundoff asymmetry so the SPD solve
                // and eigenvalue calculation operate on the same symmetric
                // matrix rather than opposite triangles.
                let average = 0.5 * (matrix[i][j] + matrix[j][i]);
                matrix[i][j] = average;
                matrix[j][i] = average;
            }
        }

        let tensor = Self { matrix };
        tensor.cholesky()?;

        let mut moments = tensor.principal_moments();
        moments.sort_by(f64::total_cmp);
        let moment_scale = 1.0_f64.max(moments[2]);
        if moments[0] <= 0.0
            || moments[1] <= 0.0
            || moments[2] > moments[0] + moments[1] + TENSOR_REL_TOL * moment_scale
        {
            return Err(TensorError::TriangleInequality);
        }

        Ok(tensor)
    }

    /// Construct a tensor from principal moments aligned with body axes.
    pub fn diagonal(principal_moments: [f64; 3]) -> Result<Self, TensorError> {
        Self::new([
            [principal_moments[0], 0.0, 0.0],
            [0.0, principal_moments[1], 0.0],
            [0.0, 0.0, principal_moments[2]],
        ])
    }

    /// Return the body-frame matrix.
    pub fn matrix(&self) -> [[f64; 3]; 3] {
        self.matrix
    }

    /// Return principal moments, computed by a symmetric Jacobi eigensolver.
    pub fn principal_moments(&self) -> [f64; 3] {
        let mut matrix = self.matrix;
        for _ in 0..64 {
            let mut p = 0;
            let mut q = 1;
            let mut largest = matrix[0][1].abs();
            for (i, j) in [(0, 2), (1, 2)] {
                if matrix[i][j].abs() > largest {
                    largest = matrix[i][j].abs();
                    p = i;
                    q = j;
                }
            }

            let diagonal_scale = 1.0_f64
                .max(matrix[0][0].abs())
                .max(matrix[1][1].abs())
                .max(matrix[2][2].abs());
            if largest <= f64::EPSILON * diagonal_scale * 8.0 {
                break;
            }

            let app = matrix[p][p];
            let aqq = matrix[q][q];
            let apq = matrix[p][q];
            let tau = (aqq - app) / (2.0 * apq);
            let t = if tau >= 0.0 {
                1.0 / (tau + (1.0 + tau * tau).sqrt())
            } else {
                -1.0 / (-tau + (1.0 + tau * tau).sqrt())
            };
            let cosine = 1.0 / (1.0 + t * t).sqrt();
            let sine = t * cosine;

            matrix[p][p] = app - t * apq;
            matrix[q][q] = aqq + t * apq;
            matrix[p][q] = 0.0;
            matrix[q][p] = 0.0;

            for k in 0..3 {
                if k != p && k != q {
                    let mkp = matrix[k][p];
                    let mkq = matrix[k][q];
                    matrix[k][p] = cosine * mkp - sine * mkq;
                    matrix[p][k] = matrix[k][p];
                    matrix[k][q] = sine * mkp + cosine * mkq;
                    matrix[q][k] = matrix[k][q];
                }
            }
        }

        [matrix[0][0], matrix[1][1], matrix[2][2]]
    }

    /// Apply the tensor to a body-frame vector.
    pub fn mul_vec(&self, vector: Vec3) -> Vec3 {
        [
            dot(self.matrix[0], vector),
            dot(self.matrix[1], vector),
            dot(self.matrix[2], vector),
        ]
    }

    /// Compute angular acceleration from Euler's body-frame equation:
    /// I * omega_dot = torque - omega cross (I * omega).
    pub fn angular_acceleration(
        &self,
        angular_velocity_body: Vec3,
        torque_body: Vec3,
    ) -> Result<Vec3, TensorError> {
        if !is_finite_vec(angular_velocity_body) || !is_finite_vec(torque_body) {
            return Err(TensorError::NonFinite);
        }
        let angular_momentum = self.mul_vec(angular_velocity_body);
        let right_hand_side = sub(torque_body, cross(angular_velocity_body, angular_momentum));
        self.solve(right_hand_side)
    }

    fn cholesky(&self) -> Result<[[f64; 3]; 3], TensorError> {
        let a = self.matrix;
        let mut l = [[0.0; 3]; 3];

        if a[0][0] <= 0.0 {
            return Err(TensorError::NotPositiveDefinite);
        }
        l[0][0] = a[0][0].sqrt();
        l[1][0] = a[1][0] / l[0][0];
        l[2][0] = a[2][0] / l[0][0];

        let pivot_1 = a[1][1] - l[1][0] * l[1][0];
        if !pivot_1.is_finite() || pivot_1 <= 0.0 {
            return Err(TensorError::NotPositiveDefinite);
        }
        l[1][1] = pivot_1.sqrt();
        l[2][1] = (a[2][1] - l[2][0] * l[1][0]) / l[1][1];

        let pivot_2 = a[2][2] - l[2][0] * l[2][0] - l[2][1] * l[2][1];
        if !pivot_2.is_finite() || pivot_2 <= 0.0 {
            return Err(TensorError::NotPositiveDefinite);
        }
        l[2][2] = pivot_2.sqrt();
        Ok(l)
    }

    fn solve(&self, right_hand_side: Vec3) -> Result<Vec3, TensorError> {
        let l = self.cholesky()?;
        let y0 = right_hand_side[0] / l[0][0];
        let y1 = (right_hand_side[1] - l[1][0] * y0) / l[1][1];
        let y2 = (right_hand_side[2] - l[2][0] * y0 - l[2][1] * y1) / l[2][2];

        let x2 = y2 / l[2][2];
        let x1 = (y1 - l[2][1] * x2) / l[1][1];
        let x0 = (y0 - l[1][0] * x1 - l[2][0] * x2) / l[0][0];
        Ok([x0, x1, x2])
    }
}

/// Right-handed vector convention with components [x, y, z].
pub type Vec3 = [f64; 3];

/// Unit quaternion mapping body-frame coordinates into world-frame coordinates.
/// Components are stored internally in scalar-first order (w, x, y, z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quaternion {
    w: f64,
    x: f64,
    y: f64,
    z: f64,
}

impl Quaternion {
    pub const IDENTITY: Self = Self {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// Construct and normalize from scalar-first (w, x, y, z) components.
    pub fn from_components(components: [f64; 4]) -> Result<Self, IntegrationError> {
        let q = Self {
            w: components[0],
            x: components[1],
            y: components[2],
            z: components[3],
        };
        q.normalized()
    }

    /// Exponential-map quaternion for a rotation vector.
    pub fn from_rotation_vector(rotation_vector: Vec3) -> Result<Self, IntegrationError> {
        if !is_finite_vec(rotation_vector) {
            return Err(IntegrationError::NonFiniteState);
        }
        let angle = norm(rotation_vector);
        let half_angle = 0.5 * angle;
        let scale = if angle < 1.0e-12 {
            0.5 - angle * angle / 48.0
        } else {
            half_angle.sin() / angle
        };
        Self {
            w: half_angle.cos(),
            x: rotation_vector[0] * scale,
            y: rotation_vector[1] * scale,
            z: rotation_vector[2] * scale,
        }
        .normalized()
    }

    pub fn components(self) -> [f64; 4] {
        [self.w, self.x, self.y, self.z]
    }

    pub fn norm(self) -> f64 {
        (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    fn normalized(self) -> Result<Self, IntegrationError> {
        let n = self.norm();
        if !n.is_finite() {
            return Err(IntegrationError::NonFiniteState);
        }
        if n <= 1.0e-15 {
            return Err(IntegrationError::InvalidOrientation);
        }
        Ok(self.scale(1.0 / n))
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            w: self.w * factor,
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    fn add(self, other: Self) -> Self {
        Self {
            w: self.w + other.w,
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    fn multiply(self, rhs: Self) -> Self {
        Self {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
    }

    fn conjugate(self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    /// Rotate a body-frame vector into world coordinates.
    pub fn rotate(self, vector: Vec3) -> Vec3 {
        let pure = Self {
            w: 0.0,
            x: vector[0],
            y: vector[1],
            z: vector[2],
        };
        let rotated = self.multiply(pure).multiply(self.conjugate());
        [rotated.x, rotated.y, rotated.z]
    }
}

/// Rigid state with body-frame angular velocity and body-to-world orientation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidState {
    pub orientation_body_to_world: Quaternion,
    pub angular_velocity_body: Vec3,
}

impl RigidState {
    pub fn new(
        orientation_body_to_world: Quaternion,
        angular_velocity_body: Vec3,
    ) -> Result<Self, IntegrationError> {
        if !is_finite_vec(angular_velocity_body) {
            return Err(IntegrationError::NonFiniteState);
        }
        Ok(Self {
            orientation_body_to_world: orientation_body_to_world.normalized()?,
            angular_velocity_body,
        })
    }

    /// World angular momentum: L_W = R(q) * I_B * omega_B.
    pub fn angular_momentum_world(self, inertia: &InertiaTensor) -> Vec3 {
        self.orientation_body_to_world
            .rotate(inertia.mul_vec(self.angular_velocity_body))
    }

    /// Rotational kinetic energy: T = 1/2 * omega_B dot I_B * omega_B.
    pub fn kinetic_energy(self, inertia: &InertiaTensor) -> f64 {
        0.5 * dot(
            self.angular_velocity_body,
            inertia.mul_vec(self.angular_velocity_body),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationError {
    InvalidTimestep,
    NonFiniteState,
    InvalidOrientation,
    InvalidInertia,
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimestep => write!(f, "timestep must be finite and positive"),
            Self::NonFiniteState => write!(f, "state or applied torque contains a non-finite value"),
            Self::InvalidOrientation => write!(f, "orientation quaternion has zero norm"),
            Self::InvalidInertia => write!(f, "inertia tensor solve failed"),
        }
    }
}

impl Error for IntegrationError {}

#[derive(Clone, Copy)]
struct Derivative {
    angular_velocity: Vec3,
    orientation: Quaternion,
}

fn derivative(
    state: RigidState,
    inertia: &InertiaTensor,
    torque_body: Vec3,
) -> Result<Derivative, IntegrationError> {
    let angular_momentum_body = inertia.mul_vec(state.angular_velocity_body);
    let rhs = sub(
        torque_body,
        cross(state.angular_velocity_body, angular_momentum_body),
    );
    let angular_acceleration = inertia
        .solve(rhs)
        .map_err(|_| IntegrationError::InvalidInertia)?;
    let spin = Quaternion {
        w: 0.0,
        x: state.angular_velocity_body[0],
        y: state.angular_velocity_body[1],
        z: state.angular_velocity_body[2],
    };
    Ok(Derivative {
        angular_velocity: angular_acceleration,
        orientation: state.orientation_body_to_world.multiply(spin).scale(0.5),
    })
}

fn add_scaled(state: RigidState, derivative: Derivative, scale: f64) -> RigidState {
    RigidState {
        orientation_body_to_world: state
            .orientation_body_to_world
            .add(derivative.orientation.scale(scale)),
        angular_velocity_body: add(
            state.angular_velocity_body,
            scale_vec(derivative.angular_velocity, scale),
        ),
    }
}

/// Advance one torque-driven rigid-body step using classical fourth-order RK.
///
/// torque_body is held constant in body coordinates during this step. The
/// returned quaternion is normalized, but no claim of symplecticity or exact
/// energy conservation is made. Use this as an independent high-order
/// reference against legacy, midpoint, and geometric integrators.
pub fn step_rk4(
    state: RigidState,
    inertia: &InertiaTensor,
    torque_body: Vec3,
    dt: f64,
) -> Result<RigidState, IntegrationError> {
    if !dt.is_finite() || dt <= 0.0 {
        return Err(IntegrationError::InvalidTimestep);
    }
    if !is_finite_vec(torque_body) || !is_finite_vec(state.angular_velocity_body) {
        return Err(IntegrationError::NonFiniteState);
    }

    let state = RigidState::new(state.orientation_body_to_world, state.angular_velocity_body)?;
    let k1 = derivative(state, inertia, torque_body)?;
    let k2 = derivative(add_scaled(state, k1, 0.5 * dt), inertia, torque_body)?;
    let k3 = derivative(add_scaled(state, k2, 0.5 * dt), inertia, torque_body)?;
    let k4 = derivative(add_scaled(state, k3, dt), inertia, torque_body)?;

    let weighted_omega = add(
        add(k1.angular_velocity, scale_vec(k2.angular_velocity, 2.0)),
        add(
            scale_vec(k3.angular_velocity, 2.0),
            k4.angular_velocity,
        ),
    );
    let omega = add(state.angular_velocity_body, scale_vec(weighted_omega, dt / 6.0));

    let weighted_orientation = k1
        .orientation
        .add(k2.orientation.scale(2.0))
        .add(k3.orientation.scale(2.0))
        .add(k4.orientation);
    let orientation = state
        .orientation_body_to_world
        .add(weighted_orientation.scale(dt / 6.0))
        .normalized()?;

    RigidState::new(orientation, omega)
}

/// Sign-invariant geodesic distance between unit quaternions, in radians.
pub fn orientation_error_radians(a: Quaternion, b: Quaternion) -> f64 {
    let ac = a.components();
    let bc = b.components();
    let product = ac[0] * bc[0] + ac[1] * bc[1] + ac[2] * bc[2] + ac[3] * bc[3];
    2.0 * product.abs().clamp(0.0, 1.0).acos()
}

/// Largest orthogonality residual among the three rotated basis axes.
pub fn rotation_orthogonality_residual(q: Quaternion) -> f64 {
    let axes = [
        q.rotate([1.0, 0.0, 0.0]),
        q.rotate([0.0, 1.0, 0.0]),
        q.rotate([0.0, 0.0, 1.0]),
    ];
    let mut residual = 0.0_f64;
    for i in 0..3 {
        for j in 0..3 {
            let expected = if i == j { 1.0 } else { 0.0 };
            residual = residual.max((dot(axes[i], axes[j]) - expected).abs());
        }
    }
    residual
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale_vec(vector: Vec3, factor: f64) -> Vec3 {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
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

fn norm(vector: Vec3) -> f64 {
    dot(vector, vector).sqrt()
}

fn is_finite_vec(vector: Vec3) -> bool {
    vector.iter().all(|value| value.is_finite())
}

fn mul_matrix_vec(matrix: [[f64; 3]; 3], vector: Vec3) -> Vec3 {
    [
        dot(matrix[0], vector),
        dot(matrix[1], vector),
        dot(matrix[2], vector),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagonal_inertia() -> InertiaTensor {
        InertiaTensor::diagonal([1.0, 2.0, 3.0]).expect("valid physical tensor")
    }

    fn integrate(
        mut state: RigidState,
        inertia: &InertiaTensor,
        torque: Vec3,
        dt: f64,
        steps: usize,
    ) -> RigidState {
        for _ in 0..steps {
            state = step_rk4(state, inertia, torque, dt).expect("valid reference step");
        }
        state
    }

    fn relative_error(a: Vec3, b: Vec3) -> f64 {
        norm(sub(a, b)) / norm(a).max(1.0e-15)
    }

    #[test]
    fn rejects_non_finite_nonsymmetric_non_spd_and_unphysical_tensors() {
        assert_eq!(
            InertiaTensor::new([[f64::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
            Err(TensorError::NonFinite)
        );
        assert_eq!(
            InertiaTensor::new([[1.0, 0.1, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
            Err(TensorError::NotSymmetric)
        );
        assert_eq!(
            InertiaTensor::new([[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]]),
            Err(TensorError::NotPositiveDefinite)
        );
        assert_eq!(
            InertiaTensor::diagonal([1.0, 1.0, 3.0]),
            Err(TensorError::TriangleInequality)
        );
    }

    #[test]
    fn accepts_spherical_symmetric_and_asymmetric_tops() {
        assert!(InertiaTensor::diagonal([2.0, 2.0, 2.0]).is_ok());
        assert!(InertiaTensor::diagonal([2.0, 2.0, 3.0]).is_ok());
        assert!(diagonal_inertia().is_ok());
    }

    #[test]
    fn spherical_top_matches_closed_form_so3_rotation() {
        let inertia = InertiaTensor::diagonal([2.0, 2.0, 2.0]).expect("sphere");
        let omega = [0.3, -0.5, 0.7];
        let initial = RigidState::new(Quaternion::IDENTITY, omega).expect("valid state");
        let final_state = integrate(initial, &inertia, [0.0; 3], 1.0e-4, 10_000);
        let exact = Quaternion::from_rotation_vector(scale_vec(omega, 1.0)).expect("exact q");
        assert!(orientation_error_radians(final_state.orientation_body_to_world, exact) < 1.0e-10);
        assert!(norm(sub(final_state.angular_velocity_body, omega)) < 1.0e-12);
    }

    #[test]
    fn asymmetric_top_preserves_energy_and_world_angular_momentum() {
        let inertia = diagonal_inertia();
        let initial = RigidState::new(Quaternion::IDENTITY, [0.7, 1.1, 1.3])
            .expect("valid state");
        let initial_energy = initial.kinetic_energy(&inertia);
        let initial_momentum = initial.angular_momentum_world(&inertia);
        let final_state = integrate(initial, &inertia, [0.0; 3], 1.0e-3, 10_000);

        assert!(relative_error(initial_momentum, final_state.angular_momentum_world(&inertia)) < 1.0e-6);
        assert!(
            ((final_state.kinetic_energy(&inertia) - initial_energy) / initial_energy).abs() < 1.0e-6
        );
        assert!((final_state.orientation_body_to_world.norm() - 1.0).abs() < 1.0e-12);
        assert!(rotation_orthogonality_residual(final_state.orientation_body_to_world) < 1.0e-12);
    }

    #[test]
    fn intermediate_axis_rotation_is_unstable() {
        let inertia = diagonal_inertia();
        let initial = RigidState::new(Quaternion::IDENTITY, [1.0e-4, 1.0, 0.0])
            .expect("valid state");
        let initial_transverse = norm([initial.angular_velocity_body[0], 0.0, 0.0]);
        let final_state = integrate(initial, &inertia, [0.0; 3], 1.0e-3, 10_000);
        let final_transverse = norm([
            final_state.angular_velocity_body[0],
            0.0,
            final_state.angular_velocity_body[2],
        ]);
        assert!(final_transverse > initial_transverse * 100.0);
    }

    #[test]
    fn spherical_top_constant_body_torque_matches_angular_momentum_impulse() {
        // For a spherical inertia, I*omega is parallel to omega and the
        // gyroscopic cross term vanishes. Therefore body-frame angular
        // momentum changes by the constant body torque integral exactly.
        let inertia = InertiaTensor::diagonal([2.0, 2.0, 2.0]).expect("sphere");
        let omega = [0.4, -0.2, 0.7];
        let torque = [0.03, 0.04, -0.02];
        let initial = RigidState::new(Quaternion::IDENTITY, omega).expect("valid state");
        let initial_momentum = inertia.mul_vec(omega);
        let final_state = integrate(initial, &inertia, torque, 1.0e-3, 1_000);
        let expected = add(initial_momentum, torque);
        assert!(norm(sub(inertia.mul_vec(final_state.angular_velocity_body), expected)) < 1.0e-10);
    }

    #[test]
    fn symmetric_top_body_precession_matches_euler_closed_form() {
        let inertia = InertiaTensor::diagonal([2.0, 2.0, 3.0]).expect("symmetric top");
        let omega = [0.7, 1.1, 1.3];
        let initial = RigidState::new(Quaternion::IDENTITY, omega).expect("valid state");
        let duration = 2.0;
        let precession_rate = (3.0 - 2.0) / 2.0 * omega[2];
        let phase = precession_rate * duration;
        let expected = [
            omega[0] * phase.cos() - omega[1] * phase.sin(),
            omega[0] * phase.sin() + omega[1] * phase.cos(),
            omega[2],
        ];
        let final_state = integrate(initial, &inertia, [0.0; 3], 1.0e-3, 2_000);
        assert!(
            norm(sub(final_state.angular_velocity_body, expected)) < 1.0e-10,
            "actual={:?}, expected={expected:?}",
            final_state.angular_velocity_body
        );
    }

    #[test]
    fn euler_equation_is_covariant_under_body_basis_rotation() {
        let inertia = InertiaTensor::new([
            [2.0, 0.2, 0.1],
            [0.2, 2.5, 0.3],
            [0.1, 0.3, 3.0],
        ])
        .expect("valid off-diagonal tensor");
        let basis = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let mut rotated = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    for l in 0..3 {
                        rotated[i][j] += basis[i][k] * inertia.matrix[k][l] * basis[j][l];
                    }
                }
            }
        }
        let rotated_inertia = InertiaTensor::new(rotated).expect("rotated physical tensor");
        let omega = [0.7, -0.4, 0.9];
        let torque = [-0.1, 0.2, 0.3];
        let alpha = inertia
            .angular_acceleration(omega, torque)
            .expect("original acceleration");
        let alpha_rotated = rotated_inertia
            .angular_acceleration(mul_matrix_vec(basis, omega), mul_matrix_vec(basis, torque))
            .expect("rotated acceleration");
        let expected = mul_matrix_vec(basis, alpha);
        assert!(norm(sub(alpha_rotated, expected)) < 1.0e-12);
    }

    #[test]
    fn rk4_has_fourth_order_timestep_convergence_on_asymmetric_top() {
        let inertia = diagonal_inertia();
        let initial = RigidState::new(Quaternion::IDENTITY, [0.7, 1.1, 1.3])
            .expect("valid state");
        let reference = integrate(initial, &inertia, [0.0; 3], 1.25e-4, 16_000);
        let coarse = integrate(initial, &inertia, [0.0; 3], 1.0e-2, 200);
        let medium = integrate(initial, &inertia, [0.0; 3], 5.0e-3, 400);
        let fine = integrate(initial, &inertia, [0.0; 3], 2.5e-3, 800);
        let error = |state: RigidState| {
            norm(sub(
                state.angular_velocity_body,
                reference.angular_velocity_body,
            ))
        };
        let coarse_error = error(coarse);
        let medium_error = error(medium);
        let fine_error = error(fine);
        let ratio_1 = coarse_error / medium_error;
        let ratio_2 = medium_error / fine_error;
        assert!((14.0..18.0).contains(&ratio_1), "first refinement ratio {ratio_1}");
        assert!((14.0..18.0).contains(&ratio_2), "second refinement ratio {ratio_2}");
    }

    #[test]
    fn rejects_bad_time_step_torque_and_zero_quaternion() {
        let inertia = diagonal_inertia();
        let state = RigidState::new(Quaternion::IDENTITY, [0.0; 3]).expect("valid state");
        assert_eq!(
            step_rk4(state, &inertia, [0.0; 3], 0.0),
            Err(IntegrationError::InvalidTimestep)
        );
        assert_eq!(
            step_rk4(state, &inertia, [f64::INFINITY, 0.0, 0.0], 0.1),
            Err(IntegrationError::NonFiniteState)
        );
        assert_eq!(
            Quaternion::from_components([0.0; 4]),
            Err(IntegrationError::InvalidOrientation)
        );
    }
}
