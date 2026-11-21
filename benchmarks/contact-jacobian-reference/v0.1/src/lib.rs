//! Independent 3D point-contact Jacobian and rotational effective-mass oracle.
//! This M1 reference does not depend on the production contact solver or Bivector.

use std::error::Error;
use std::fmt::{Display, Formatter};

pub type Vec3 = [f64; 3];
pub type Mat3 = [[f64; 3]; 3];
const TOL: f64 = 1.0e-10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactError {
    NonFiniteInput,
    NonPositiveMass,
    NonSymmetricTensor,
    NonPositiveSemidefiniteTensor,
    InvalidRotation,
    SingularInertia,
    InvalidContactDirection,
    NonPositiveEffectiveMass,
    NonFiniteResult,
}

impl Display for ContactError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "input values must be finite"),
            Self::NonPositiveMass => write!(f, "dynamic body mass must be positive"),
            Self::NonSymmetricTensor => write!(f, "second-moment tensor must be symmetric"),
            Self::NonPositiveSemidefiniteTensor => {
                write!(f, "second-moment tensor must be positive semidefinite")
            }
            Self::InvalidRotation => write!(f, "body rotation must be proper and orthogonal"),
            Self::SingularInertia => {
                write!(f, "rotational inertia tensor must be positive definite")
            }
            Self::InvalidContactDirection => {
                write!(f, "contact direction must be a finite unit vector")
            }
            Self::NonPositiveEffectiveMass => write!(f, "effective inverse mass must be positive"),
            Self::NonFiniteResult => write!(f, "calculation produced a non-finite result"),
        }
    }
}
impl Error for ContactError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointImpulseResponse {
    pub linear_velocity_delta_m_s: Vec3,
    pub angular_velocity_delta_rad_s: Vec3,
    pub contact_point_velocity_delta_m_s: Vec3,
    pub kinetic_energy_delta_j: f64,
}

/// Dynamic 3D body with a full local-frame mass second-moment tensor.
/// Fixed/kinematic endpoints contribute zero inverse mass and inverse inertia.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyMassProperties3 {
    dynamic: bool,
    mass_kg: f64,
    local_second_moment_kg_m2: Mat3,
    body_to_world: Mat3,
}

impl BodyMassProperties3 {
    pub fn dynamic(
        mass_kg: f64,
        local_second_moment_kg_m2: Mat3,
        body_to_world: Mat3,
    ) -> Result<Self, ContactError> {
        if !mass_kg.is_finite() {
            return Err(ContactError::NonFiniteInput);
        }
        if mass_kg <= 0.0 {
            return Err(ContactError::NonPositiveMass);
        }
        validate_second_moment(local_second_moment_kg_m2)?;
        validate_rotation(body_to_world)?;
        let candidate = Self {
            dynamic: true,
            mass_kg,
            local_second_moment_kg_m2,
            body_to_world,
        };
        validate_spd(candidate.world_inertia_kg_m2())?;
        Ok(candidate)
    }

    pub fn fixed() -> Self {
        Self {
            dynamic: false,
            mass_kg: 0.0,
            local_second_moment_kg_m2: zero_matrix(),
            body_to_world: identity(),
        }
    }

    pub fn is_dynamic(&self) -> bool {
        self.dynamic
    }
    pub fn mass_kg(&self) -> f64 {
        self.mass_kg
    }
    pub fn local_second_moment_kg_m2(&self) -> Mat3 {
        self.local_second_moment_kg_m2
    }
    pub fn body_to_world(&self) -> Mat3 {
        self.body_to_world
    }

    pub fn world_inertia_kg_m2(&self) -> Mat3 {
        if !self.dynamic {
            return zero_matrix();
        }
        let local_i = inertia_from_second_moment(self.local_second_moment_kg_m2);
        multiply(
            &multiply(&self.body_to_world, &local_i),
            &transpose(self.body_to_world),
        )
    }

    /// Apply a world-space impulse at a world-frame lever arm and return M^-1 J^T.
    pub fn response_to_point_impulse(
        &self,
        lever_arm_world_m: Vec3,
        impulse_world_ns: Vec3,
    ) -> Result<PointImpulseResponse, ContactError> {
        validate_vector(lever_arm_world_m)?;
        validate_vector(impulse_world_ns)?;
        if !self.dynamic {
            return Ok(PointImpulseResponse {
                linear_velocity_delta_m_s: [0.0; 3],
                angular_velocity_delta_rad_s: [0.0; 3],
                contact_point_velocity_delta_m_s: [0.0; 3],
                kinetic_energy_delta_j: 0.0,
            });
        }
        let linear_delta = scale(impulse_world_ns, 1.0 / self.mass_kg);
        let torque_world = cross(lever_arm_world_m, impulse_world_ns);
        let angular_delta = solve3(self.world_inertia_kg_m2(), torque_world)?;
        let point_delta = add_vec(linear_delta, cross(angular_delta, lever_arm_world_m));
        let energy_delta = 0.5 * dot(impulse_world_ns, point_delta);
        if !energy_delta.is_finite() || energy_delta < -1.0e-10 {
            return Err(ContactError::NonFiniteResult);
        }
        Ok(PointImpulseResponse {
            linear_velocity_delta_m_s: linear_delta,
            angular_velocity_delta_rad_s: angular_delta,
            contact_point_velocity_delta_m_s: point_delta,
            kinetic_energy_delta_j: energy_delta.max(0.0),
        })
    }

    /// Compute n^T J M^-1 J^T n for a unit contact normal n at lever arm r.
    pub fn contact_inverse_effective_mass(
        &self,
        lever_arm_world_m: Vec3,
        contact_normal_world: Vec3,
    ) -> Result<f64, ContactError> {
        validate_unit_vector(contact_normal_world)?;
        let response = self.response_to_point_impulse(lever_arm_world_m, contact_normal_world)?;
        let value = dot(
            contact_normal_world,
            response.contact_point_velocity_delta_m_s,
        );
        if !value.is_finite() {
            return Err(ContactError::NonFiniteResult);
        }
        if self.dynamic && value <= 0.0 {
            return Err(ContactError::NonPositiveEffectiveMass);
        }
        Ok(value.max(0.0))
    }

    /// Exact kinetic-energy increase for this impulse from a zero-velocity state.
    pub fn kinetic_energy_after_point_impulse(
        &self,
        lever_arm_world_m: Vec3,
        impulse_world_ns: Vec3,
    ) -> Result<f64, ContactError> {
        Ok(self
            .response_to_point_impulse(lever_arm_world_m, impulse_world_ns)?
            .kinetic_energy_delta_j)
    }
}

pub fn pair_contact_inverse_effective_mass(
    body_a: &BodyMassProperties3,
    lever_arm_a_world_m: Vec3,
    body_b: &BodyMassProperties3,
    lever_arm_b_world_m: Vec3,
    contact_normal_world: Vec3,
) -> Result<f64, ContactError> {
    validate_unit_vector(contact_normal_world)?;
    let a = body_a.contact_inverse_effective_mass(lever_arm_a_world_m, contact_normal_world)?;
    let b = body_b.contact_inverse_effective_mass(lever_arm_b_world_m, contact_normal_world)?;
    let total = a + b;
    if !total.is_finite() {
        return Err(ContactError::NonFiniteResult);
    }
    if total <= 0.0 {
        return Err(ContactError::NonPositiveEffectiveMass);
    }
    Ok(total)
}

/// Discrete work at a frictionless contact port: W = λ(v_n + 1/2 λ k_pair).
pub fn midpoint_contact_work_j(
    lambda_ns: f64,
    pre_impulse_relative_normal_velocity_m_s: f64,
    pair_inverse_effective_mass_per_kg: f64,
) -> Result<f64, ContactError> {
    if !lambda_ns.is_finite()
        || !pre_impulse_relative_normal_velocity_m_s.is_finite()
        || !pair_inverse_effective_mass_per_kg.is_finite()
        || pair_inverse_effective_mass_per_kg <= 0.0
    {
        return Err(ContactError::NonFiniteInput);
    }
    let work = lambda_ns
        * (pre_impulse_relative_normal_velocity_m_s
            + 0.5 * lambda_ns * pair_inverse_effective_mass_per_kg);
    if work.is_finite() {
        Ok(work)
    } else {
        Err(ContactError::NonFiniteResult)
    }
}

fn zero_matrix() -> Mat3 {
    [[0.0; 3]; 3]
}
fn identity() -> Mat3 {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
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
fn add_vec(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: Vec3, s: f64) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn transpose(a: Mat3) -> Mat3 {
    [
        [a[0][0], a[1][0], a[2][0]],
        [a[0][1], a[1][1], a[2][1]],
        [a[0][2], a[1][2], a[2][2]],
    ]
}
fn multiply(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut out = zero_matrix();
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                out[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    out
}
#[cfg(test)]
#[cfg(test)]
fn multiply_vec(a: Mat3, b: Vec3) -> Vec3 {
    [dot(a[0], b), dot(a[1], b), dot(a[2], b)]
}
fn determinant(a: Mat3) -> f64 {
    a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
}
fn inertia_from_second_moment(c: Mat3) -> Mat3 {
    let tr = c[0][0] + c[1][1] + c[2][2];
    let mut i = zero_matrix();
    for row in 0..3 {
        for col in 0..3 {
            i[row][col] = (if row == col { tr } else { 0.0 }) - c[row][col];
        }
    }
    i
}
fn validate_vector(v: Vec3) -> Result<(), ContactError> {
    if v.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(ContactError::NonFiniteInput)
    }
}
fn validate_unit_vector(v: Vec3) -> Result<(), ContactError> {
    validate_vector(v)?;
    if (dot(v, v) - 1.0).abs() <= 1.0e-9 {
        Ok(())
    } else {
        Err(ContactError::InvalidContactDirection)
    }
}
fn validate_second_moment(c: Mat3) -> Result<(), ContactError> {
    if c.iter().flatten().any(|x| !x.is_finite()) {
        return Err(ContactError::NonFiniteInput);
    }
    let scale = c.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()));
    if scale == 0.0 {
        return Err(ContactError::NonPositiveSemidefiniteTensor);
    }
    let tol = TOL * scale;
    for (i, row) in c.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            if (value - c[j][i]).abs() > tol {
                return Err(ContactError::NonSymmetricTensor);
            }
        }
    }
    for (i, row) in c.iter().enumerate() {
        if row[i] < -tol {
            return Err(ContactError::NonPositiveSemidefiniteTensor);
        }
    }
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        if c[i][i] * c[j][j] - c[i][j] * c[i][j] < -tol * scale {
            return Err(ContactError::NonPositiveSemidefiniteTensor);
        }
    }
    if determinant(c) < -tol * scale * scale {
        return Err(ContactError::NonPositiveSemidefiniteTensor);
    }
    Ok(())
}
fn validate_rotation(r: Mat3) -> Result<(), ContactError> {
    if r.iter().flatten().any(|x| !x.is_finite()) {
        return Err(ContactError::NonFiniteInput);
    }
    let p = multiply(&transpose(r), &r);
    for (i, row) in p.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            let target = if i == j { 1.0 } else { 0.0 };
            if (value - target).abs() > 1.0e-9 {
                return Err(ContactError::InvalidRotation);
            }
        }
    }
    if (determinant(r) - 1.0).abs() > 1.0e-9 {
        return Err(ContactError::InvalidRotation);
    }
    Ok(())
}
fn validate_spd(a: Mat3) -> Result<(), ContactError> {
    let scale = a.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()));
    if !scale.is_finite() || scale == 0.0 {
        return Err(ContactError::SingularInertia);
    }
    let tol = 1.0e-12 * scale;
    let minor2 = a[0][0] * a[1][1] - a[0][1] * a[1][0];
    if a[0][0] <= tol || minor2 <= tol * scale || determinant(a) <= tol * scale * scale {
        return Err(ContactError::SingularInertia);
    }
    Ok(())
}
fn solve3(a: Mat3, b: Vec3) -> Result<Vec3, ContactError> {
    let scale = a.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()));
    if !scale.is_finite() || scale == 0.0 {
        return Err(ContactError::SingularInertia);
    }
    let mut aug = [[0.0; 4]; 3];
    for (row_index, row_values) in aug.iter_mut().enumerate() {
        for (column_index, value) in row_values.iter_mut().take(3).enumerate() {
            *value = a[row_index][column_index];
        }
        row_values[3] = b[row_index];
    }
    for col in 0..3 {
        let mut pivot = col;
        for row in (col + 1)..3 {
            if aug[row][col].abs() > aug[pivot][col].abs() {
                pivot = row;
            }
        }
        if aug[pivot][col].abs() <= 1.0e-13 * scale {
            return Err(ContactError::SingularInertia);
        }
        aug.swap(pivot, col);
        let divisor = aug[col][col];
        for value in aug[col].iter_mut().skip(col) {
            *value /= divisor;
        }
        let pivot_values = aug[col];
        for (row_index, row_values) in aug.iter_mut().enumerate() {
            if row_index != col {
                let factor = row_values[col];
                for (value, pivot_value) in row_values.iter_mut().zip(pivot_values).skip(col) {
                    *value -= factor * pivot_value;
                }
            }
        }
    }
    let out = [aug[0][3], aug[1][3], aug[2][3]];
    if out.iter().all(|x| x.is_finite()) {
        Ok(out)
    } else {
        Err(ContactError::NonFiniteResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn close(a: f64, b: f64, t: f64) {
        assert!((a - b).abs() <= t, "actual {a:.15e}, expected {b:.15e}");
    }
    fn sphere(m: f64, r: f64) -> BodyMassProperties3 {
        let c = 0.2 * m * r * r;
        BodyMassProperties3::dynamic(m, [[c, 0.0, 0.0], [0.0, c, 0.0], [0.0, 0.0, c]], identity())
            .expect("sphere")
    }
    fn cuboid() -> BodyMassProperties3 {
        BodyMassProperties3::dynamic(
            2.0,
            [
                [2.0 / 3.0, 0.0, 0.0],
                [0.0, 1.0 / 6.0, 0.0],
                [0.0, 0.0, 1.0 / 24.0],
            ],
            identity(),
        )
        .expect("cuboid")
    }

    #[test]
    fn sphere_matches_isotropic_point_contact_closed_form() {
        let body = sphere(2.0, 0.5);
        let k = body
            .contact_inverse_effective_mass([0.0, 0.5, 0.0], [1.0, 0.0, 0.0])
            .expect("k");
        close(k, 0.5 + 0.25 / 0.2, 1.0e-12);
    }

    #[test]
    fn asymmetric_cuboid_has_directional_rotational_effective_mass() {
        let body = cuboid();
        let k = body
            .contact_inverse_effective_mass([0.0, 0.5, 0.0], [1.0, 0.0, 0.0])
            .expect("k");
        // Izz=Cxx+Cyy=5/6; rotational term=(0.5)^2/(5/6)=0.3.
        close(k, 0.8, 1.0e-12);
        // Conditional comparison: if the three principal moments above are fed
        // to the production mean-of-reciprocals approximation, its result is
        // 0.5 + 0.25 * mean(1/Ix, 1/Iy, 1/Iz) = 1.117647... kg^-1.
        // The exact tensor-Jacobian oracle is 0.8 kg^-1 for this pose.
        let legacy_mean_inverse = 0.5 + 0.25 * ((24.0 / 5.0 + 24.0 / 17.0 + 6.0 / 5.0) / 3.0);
        close(legacy_mean_inverse, 1.1176470588235294, 1.0e-12);
        assert!(
            (k - legacy_mean_inverse).abs() > 0.3,
            "full tensor response must expose the isotropic mean-inertia shortcut"
        );
    }

    #[test]
    fn effective_mass_is_covariant_under_body_world_rotation() {
        let original = cuboid();
        let angle = std::f64::consts::FRAC_PI_4;
        let r = [
            [angle.cos(), -angle.sin(), 0.0],
            [angle.sin(), angle.cos(), 0.0],
            [0.0, 0.0, 1.0],
        ];
        let rotated = BodyMassProperties3::dynamic(2.0, original.local_second_moment_kg_m2(), r)
            .expect("rotated");
        let lever = [0.0, 0.5, 0.0];
        let normal = [1.0, 0.0, 0.0];
        let k = original
            .contact_inverse_effective_mass(lever, normal)
            .expect("k");
        let kr = rotated
            .contact_inverse_effective_mass(multiply_vec(r, lever), multiply_vec(r, normal))
            .expect("kr");
        close(k, kr, 1.0e-12);
    }

    #[test]
    fn fixed_endpoint_contributes_zero_dynamic_endpoints_add() {
        let body = sphere(2.0, 0.5);
        let fixed = BodyMassProperties3::fixed();
        let ra = [0.0, 0.5, 0.0];
        let rb = [0.0, -0.5, 0.0];
        let n = [1.0, 0.0, 0.0];
        let one = body.contact_inverse_effective_mass(ra, n).expect("one");
        let fixed_pair =
            pair_contact_inverse_effective_mass(&body, ra, &fixed, rb, n).expect("fixed pair");
        let dynamic_pair =
            pair_contact_inverse_effective_mass(&body, ra, &body, rb, n).expect("dynamic pair");
        close(fixed_pair, one, 1.0e-12);
        close(dynamic_pair, 2.0 * one, 1.0e-12);
    }

    #[test]
    fn midpoint_contact_work_matches_pair_energy_change_from_rest() {
        let a = sphere(2.0, 0.5);
        let b = cuboid();
        let ra = [0.0, 0.5, 0.0];
        let rb = [0.0, -0.5, 0.0];
        let n = [1.0, 0.0, 0.0];
        let lambda = 0.7;
        let k = pair_contact_inverse_effective_mass(&a, ra, &b, rb, n).expect("pair k");
        let work = midpoint_contact_work_j(lambda, 0.0, k).expect("work");
        let j = scale(n, lambda);
        let ea = a
            .kinetic_energy_after_point_impulse(ra, scale(j, -1.0))
            .expect("energy a");
        let eb = b
            .kinetic_energy_after_point_impulse(rb, j)
            .expect("energy b");
        close(work, ea + eb, 1.0e-11);
    }

    #[test]
    fn rejects_nonunit_contact_direction() {
        let body = sphere(1.0, 1.0);
        assert_eq!(
            body.contact_inverse_effective_mass([0.0; 3], [2.0, 0.0, 0.0]),
            Err(ContactError::InvalidContactDirection)
        );
    }

    #[test]
    fn rejects_non_psd_second_moment_and_reflection() {
        assert_eq!(
            BodyMassProperties3::dynamic(
                1.0,
                [[1.0, 0.0, 0.0], [0.0, -0.1, 0.0], [0.0, 0.0, 1.0]],
                identity()
            ),
            Err(ContactError::NonPositiveSemidefiniteTensor)
        );
        let reflection = [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(
            BodyMassProperties3::dynamic(1.0, identity(), reflection),
            Err(ContactError::InvalidRotation)
        );
    }
}
