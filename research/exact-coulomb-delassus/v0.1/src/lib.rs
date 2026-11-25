#![forbid(unsafe_code)]

//! Dependency-free 3D matrix-free Delassus reference operator.
//! Evaluates J M^-1 J^T lambda without materializing the dense matrix.
//! This is a linear response operator, not a Coulomb contact solver.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

const AXES_PER_CONTACT: usize = 3;
const FRAME_TOLERANCE: f64 = 1.0e-10;
const PSD_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BodyId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat3 {
    pub rows: [[f64; 3]; 3],
}

impl Mat3 {
    pub const ZERO: Self = Self { rows: [[0.0; 3]; 3] };
    pub const IDENTITY: Self = Self {
        rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    #[must_use]
    pub fn mul_vec(self, vector: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|index| dot(self.rows[index], vector))
    }

    fn validate_symmetric_psd(self) -> Result<(), DelassusError> {
        if self.rows.iter().flatten().any(|value| !value.is_finite()) {
            return Err(DelassusError::InvalidInverseInertia);
        }
        let scale = self.rows.iter().flatten()
            .fold(1.0_f64, |largest, value| largest.max(value.abs()));
        let tolerance = PSD_TOLERANCE * scale;
        let [a, b, c] = self.rows[0];
        let [d, e, f] = self.rows[1];
        let [g, h, i] = self.rows[2];

        if (b - d).abs() > tolerance
            || (c - g).abs() > tolerance
            || (f - h).abs() > tolerance
        {
            return Err(DelassusError::InvalidInverseInertia);
        }

        let diagonal_ok = [a, e, i].iter().all(|value| *value >= -tolerance);
        let minors = [a * e - b * d, a * i - c * g, e * i - f * h];
        let minor_tolerance = PSD_TOLERANCE * scale * scale;
        let determinant = a * (e * i - f * h) - b * (d * i - f * g)
            + c * (d * h - e * g);
        let determinant_tolerance = PSD_TOLERANCE * scale * scale * scale;
        if minors.iter().any(|value| !value.is_finite())
            || !minor_tolerance.is_finite()
            || !determinant.is_finite()
            || !determinant_tolerance.is_finite()
        {
            return Err(DelassusError::InvalidInverseInertia);
        }
        let minors_ok = minors.iter().all(|value| *value >= -minor_tolerance);
        let determinant_ok = determinant >= -determinant_tolerance;

        if diagonal_ok && minors_ok && determinant_ok {
            Ok(())
        } else {
            Err(DelassusError::InvalidInverseInertia)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyMassProperties {
    pub id: BodyId,
    pub inverse_mass: f64,
    /// World-space inverse inertia tensor about the center of mass.
    pub inverse_inertia_world: Mat3,
}

impl BodyMassProperties {
    pub fn new(
        id: BodyId,
        inverse_mass: f64,
        inverse_inertia_world: Mat3,
    ) -> Result<Self, DelassusError> {
        if !inverse_mass.is_finite() || inverse_mass < 0.0 {
            return Err(DelassusError::InvalidInverseMass);
        }
        inverse_inertia_world.validate_symmetric_psd()?;
        Ok(Self { id, inverse_mass, inverse_inertia_world })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactFrame {
    pub normal: [f64; 3],
    pub tangent_1: [f64; 3],
    pub tangent_2: [f64; 3],
}

impl ContactFrame {
    pub fn new(
        normal: [f64; 3],
        tangent_1: [f64; 3],
        tangent_2: [f64; 3],
    ) -> Result<Self, DelassusError> {
        let frame = Self { normal, tangent_1, tangent_2 };
        frame.validate()?;
        Ok(frame)
    }

    fn validate(self) -> Result<(), DelassusError> {
        let axes = [self.normal, self.tangent_1, self.tangent_2];
        if axes.iter().flatten().any(|value| !value.is_finite()) {
            return Err(DelassusError::InvalidContactFrame);
        }
        if axes.iter().any(|axis| (dot(*axis, *axis) - 1.0).abs() > FRAME_TOLERANCE)
            || dot(self.normal, self.tangent_1).abs() > FRAME_TOLERANCE
            || dot(self.normal, self.tangent_2).abs() > FRAME_TOLERANCE
            || dot(self.tangent_1, self.tangent_2).abs() > FRAME_TOLERANCE
        {
            return Err(DelassusError::InvalidContactFrame);
        }
        if (dot(cross(self.normal, self.tangent_1), self.tangent_2) - 1.0).abs()
            > FRAME_TOLERANCE
        {
            return Err(DelassusError::InvalidContactFrame);
        }
        Ok(())
    }

    #[must_use]
    pub fn axes(self) -> [[f64; 3]; 3] {
        [self.normal, self.tangent_1, self.tangent_2]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactBlock {
    pub body_a: BodyId,
    pub body_b: BodyId,
    pub lever_arm_a: [f64; 3],
    pub lever_arm_b: [f64; 3],
    pub frame: ContactFrame,
}

impl ContactBlock {
    pub fn new(
        body_a: BodyId,
        body_b: BodyId,
        lever_arm_a: [f64; 3],
        lever_arm_b: [f64; 3],
        frame: ContactFrame,
    ) -> Result<Self, DelassusError> {
        if body_a == body_b {
            return Err(DelassusError::SameBodyContact);
        }
        if lever_arm_a.iter().chain(lever_arm_b.iter()).any(|value| !value.is_finite()) {
            return Err(DelassusError::InvalidLeverArm);
        }
        frame.validate()?;
        Ok(Self { body_a, body_b, lever_arm_a, lever_arm_b, frame })
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct GeneralizedImpulse {
    linear: [f64; 3],
    angular: [f64; 3],
}

impl GeneralizedImpulse {
    fn add_scaled(&mut self, linear: [f64; 3], angular: [f64; 3], scale: f64) {
        self.linear = add(self.linear, scaled(linear, scale));
        self.angular = add(self.angular, scaled(angular, scale));
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct GeneralizedVelocity {
    linear: [f64; 3],
    angular: [f64; 3],
}

#[derive(Clone, Copy, Debug, Default)]
struct JacobianBlock {
    linear: [f64; 3],
    angular: [f64; 3],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DelassusError {
    DuplicateBodyId,
    UnknownBodyId(BodyId),
    SameBodyContact,
    InvalidInverseMass,
    InvalidInverseInertia,
    InvalidContactFrame,
    InvalidLeverArm,
    WrongImpulseDimension { expected: usize, actual: usize },
    NonFiniteImpulse,
    NonFiniteResult,
    DimensionOverflow,
}

impl fmt::Display for DelassusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl Error for DelassusError {}

/// Stable contact-coordinate layout: [normal, tangent_1, tangent_2] per contact.
/// Bodies are keyed by BodyId, and accumulation follows input contact order.
#[derive(Clone, Debug)]
pub struct DelassusOperator {
    bodies: BTreeMap<BodyId, BodyMassProperties>,
    contacts: Vec<ContactBlock>,
    dimension: usize,
}

impl DelassusOperator {
    pub fn new(
        bodies: impl IntoIterator<Item = BodyMassProperties>,
        contacts: Vec<ContactBlock>,
    ) -> Result<Self, DelassusError> {
        let mut body_map = BTreeMap::new();
        for body in bodies {
            if !body.inverse_mass.is_finite() || body.inverse_mass < 0.0 {
                return Err(DelassusError::InvalidInverseMass);
            }
            body.inverse_inertia_world.validate_symmetric_psd()?;
            if body_map.insert(body.id, body).is_some() {
                return Err(DelassusError::DuplicateBodyId);
            }
        }

        let dimension = contacts.len().checked_mul(AXES_PER_CONTACT)
            .ok_or(DelassusError::DimensionOverflow)?;
        for contact in &contacts {
            if contact.body_a == contact.body_b {
                return Err(DelassusError::SameBodyContact);
            }
            if !body_map.contains_key(&contact.body_a) {
                return Err(DelassusError::UnknownBodyId(contact.body_a));
            }
            if !body_map.contains_key(&contact.body_b) {
                return Err(DelassusError::UnknownBodyId(contact.body_b));
            }
            if contact.lever_arm_a.iter().chain(contact.lever_arm_b.iter())
                .any(|value| !value.is_finite())
            {
                return Err(DelassusError::InvalidLeverArm);
            }
            contact.frame.validate()?;
        }

        Ok(Self { bodies: body_map, contacts, dimension })
    }

    #[must_use]
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    #[must_use]
    pub fn contact_count(&self) -> usize {
        self.contacts.len()
    }

    /// Apply W lambda by scattering J^T lambda, applying M^-1, then gathering J.
    pub fn apply(&self, lambda: &[f64]) -> Result<Vec<f64>, DelassusError> {
        if lambda.len() != self.dimension {
            return Err(DelassusError::WrongImpulseDimension {
                expected: self.dimension,
                actual: lambda.len(),
            });
        }
        if lambda.iter().any(|value| !value.is_finite()) {
            return Err(DelassusError::NonFiniteImpulse);
        }

        let mut body_impulses = self.bodies.keys()
            .map(|id| (*id, GeneralizedImpulse::default()))
            .collect::<BTreeMap<_, _>>();

        for (contact_index, contact) in self.contacts.iter().enumerate() {
            for (axis_index, axis) in contact.frame.axes().iter().enumerate() {
                let value = lambda[contact_index * AXES_PER_CONTACT + axis_index];
                let torque_a = cross(contact.lever_arm_a, *axis);
                let torque_b = cross(contact.lever_arm_b, *axis);
                body_impulses.get_mut(&contact.body_a).expect("validated body A")
                    .add_scaled(*axis, torque_a, -value);
                body_impulses.get_mut(&contact.body_b).expect("validated body B")
                    .add_scaled(*axis, torque_b, value);
            }
        }

        if body_impulses.values().any(|impulse| {
            !is_finite_vector(impulse.linear) || !is_finite_vector(impulse.angular)
        }) {
            return Err(DelassusError::NonFiniteResult);
        }

        let mut delta_velocity = BTreeMap::new();
        for (id, body) in &self.bodies {
            let impulse = body_impulses.get(id).expect("impulse initialized");
            delta_velocity.insert(*id, GeneralizedVelocity {
                linear: scaled(impulse.linear, body.inverse_mass),
                angular: body.inverse_inertia_world.mul_vec(impulse.angular),
            });
        }

        if delta_velocity.values().any(|velocity| {
            !is_finite_vector(velocity.linear) || !is_finite_vector(velocity.angular)
        }) {
            return Err(DelassusError::NonFiniteResult);
        }

        let mut result = vec![0.0; self.dimension];
        for (contact_index, contact) in self.contacts.iter().enumerate() {
            let velocity_a = delta_velocity.get(&contact.body_a).expect("validated body A");
            let velocity_b = delta_velocity.get(&contact.body_b).expect("validated body B");
            let point_a = add(velocity_a.linear, cross(velocity_a.angular, contact.lever_arm_a));
            let point_b = add(velocity_b.linear, cross(velocity_b.angular, contact.lever_arm_b));
            let relative = subtract(point_b, point_a);
            for (axis_index, axis) in contact.frame.axes().iter().enumerate() {
                result[contact_index * AXES_PER_CONTACT + axis_index] = dot(*axis, relative);
            }
        }
        if result.iter().any(|value| !value.is_finite()) {
            return Err(DelassusError::NonFiniteResult);
        }
        Ok(result)
    }

    /// Materialize a dense reference by explicitly assembling sparse Jacobian
    /// rows and contracting each shared body's inverse-mass/inertia block.
    /// This path is O(m^2 B) in the worst case and intended only for tiny tests.
    pub fn materialize_dense(&self) -> Result<Vec<Vec<f64>>, DelassusError> {
        let mut jacobian_rows = Vec::with_capacity(self.dimension);
        for contact in &self.contacts {
            for axis in contact.frame.axes() {
                let mut row = BTreeMap::new();
                row.insert(
                    contact.body_a,
                    JacobianBlock {
                        linear: scaled(axis, -1.0),
                        angular: scaled(cross(contact.lever_arm_a, axis), -1.0),
                    },
                );
                row.insert(
                    contact.body_b,
                    JacobianBlock {
                        linear: axis,
                        angular: cross(contact.lever_arm_b, axis),
                    },
                );
                jacobian_rows.push(row);
            }
        }

        let mut matrix = vec![vec![0.0; self.dimension]; self.dimension];
        for (row_index, left_row) in jacobian_rows.iter().enumerate() {
            for (column_index, right_row) in jacobian_rows.iter().enumerate() {
                let mut value = 0.0;
                for (body_id, left_block) in left_row {
                    if let Some(right_block) = right_row.get(body_id) {
                        let body = self.bodies.get(body_id).expect("validated body");
                        value += body.inverse_mass
                            * dot(left_block.linear, right_block.linear);
                        value += dot(
                            left_block.angular,
                            body.inverse_inertia_world.mul_vec(right_block.angular),
                        );
                    }
                }
                if !value.is_finite() {
                    return Err(DelassusError::NonFiniteResult);
                }
                matrix[row_index][column_index] = value;
            }
        }
        Ok(matrix)
    }
}

fn is_finite_vector(vector: [f64; 3]) -> bool {
    vector.iter().all(|value| value.is_finite())
}

fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}
fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}
fn scaled(vector: [f64; 3], scalar: f64) -> [f64; 3] {
    [vector[0] * scalar, vector[1] * scalar, vector[2] * scalar]
}
fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}
fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(id: u64, inverse_mass: f64, inverse_inertia_world: Mat3) -> BodyMassProperties {
        BodyMassProperties::new(BodyId(id), inverse_mass, inverse_inertia_world).unwrap()
    }

    fn frame() -> ContactFrame {
        ContactFrame::new(
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        )
        .unwrap()
    }

    fn contact(a: u64, b: u64, ra: [f64; 3], rb: [f64; 3]) -> ContactBlock {
        ContactBlock::new(BodyId(a), BodyId(b), ra, rb, frame()).unwrap()
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!((actual - expected).abs() <= tolerance,
            "actual={actual:.16e} expected={expected:.16e} tolerance={tolerance:.3e}");
    }

    #[test]
    fn centered_contact_matches_analytic_inverse_mass_sum() {
        let op = DelassusOperator::new(
            [body(1, 0.5, Mat3::ZERO), body(2, 0.25, Mat3::ZERO)],
            vec![contact(1, 2, [0.0; 3], [0.0; 3])],
        ).unwrap();
        let lambda = [3.0, -4.0, 5.0];
        let result = op.apply(&lambda).unwrap();
        for (actual, input) in result.iter().zip(lambda) {
            close(*actual, 0.75 * input, 1.0e-14);
        }
    }

    #[test]
    fn static_body_contributes_zero_inverse_mass_and_inertia() {
        let op = DelassusOperator::new(
            [body(1, 0.0, Mat3::ZERO), body(2, 0.5, Mat3::IDENTITY)],
            vec![contact(1, 2, [0.0; 3], [0.0; 3])],
        ).unwrap();
        assert_eq!(op.apply(&[2.0, 4.0, 6.0]).unwrap(), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn matrix_free_action_matches_dense_reference_for_coupled_contacts() {
        let ia = Mat3 { rows: [[0.5, 0.0, 0.0], [0.0, 0.25, 0.0], [0.0, 0.0, 0.125]] };
        let ib = Mat3 { rows: [[0.2, 0.01, 0.0], [0.01, 0.3, 0.0], [0.0, 0.0, 0.4]] };
        let op = DelassusOperator::new(
            [body(10, 0.25, ia), body(20, 0.5, ib), body(30, 0.75, Mat3::IDENTITY)],
            vec![
                contact(10, 20, [0.1, 0.0, 0.0], [-0.1, 0.0, 0.0]),
                contact(10, 30, [0.0, 0.2, 0.0], [0.0, -0.2, 0.0]),
            ],
        ).unwrap();
        let lambda = [0.3, -0.2, 0.4, -0.1, 0.8, -0.6];
        let matrix = op.materialize_dense().unwrap();
        let dense = matrix.iter()
            .map(|row| row.iter().zip(&lambda).map(|(a, b)| a * b).sum::<f64>())
            .collect::<Vec<_>>();
        let matrix_free = op.apply(&lambda).unwrap();
        for (actual, expected) in matrix_free.iter().zip(dense) {
            close(*actual, expected, 1.0e-12);
        }
        for (i, row) in matrix.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                close(*value, matrix[j][i], 1.0e-12);
            }
        }
    }

    #[test]
    fn quadratic_form_is_nonnegative_for_psd_mass_properties() {
        let op = DelassusOperator::new(
            [
                body(1, 0.4, Mat3::IDENTITY),
                body(2, 0.7, Mat3::IDENTITY),
                body(3, 0.2, Mat3::ZERO),
            ],
            vec![
                contact(1, 2, [0.3, -0.1, 0.0], [-0.2, 0.0, 0.1]),
                contact(2, 3, [0.1, 0.2, 0.0], [0.0, -0.2, 0.1]),
                contact(3, 1, [-0.1, 0.0, 0.2], [0.2, 0.1, 0.0]),
            ],
        ).unwrap();
        let lambda = [0.1, -0.2, 0.3, -0.4, 0.5, -0.6, 0.7, -0.8, 0.9];
        let product = op.apply(&lambda).unwrap();
        let quadratic = lambda.iter().zip(&product)
            .map(|(a, b)| a * b).sum::<f64>();
        assert!(quadratic >= -1.0e-12);
    }

    #[test]
    fn body_input_order_does_not_change_operator_action() {
        let contacts = vec![contact(1, 2, [0.2, 0.0, 0.0], [-0.2, 0.0, 0.0])];
        let first = DelassusOperator::new(
            [body(1, 0.5, Mat3::IDENTITY), body(2, 0.25, Mat3::IDENTITY)],
            contacts.clone(),
        ).unwrap();
        let second = DelassusOperator::new(
            [body(2, 0.25, Mat3::IDENTITY), body(1, 0.5, Mat3::IDENTITY)],
            contacts,
        ).unwrap();
        assert_eq!(first.apply(&[0.5, 1.5, -2.0]).unwrap(),
            second.apply(&[0.5, 1.5, -2.0]).unwrap());
    }

    #[test]
    fn rejects_duplicate_and_unknown_bodies_and_self_contacts() {
        assert_eq!(
            DelassusOperator::new(
                [body(1, 1.0, Mat3::IDENTITY), body(1, 1.0, Mat3::IDENTITY)],
                Vec::new(),
            ).err(),
            Some(DelassusError::DuplicateBodyId)
        );
        assert_eq!(
            DelassusOperator::new(
                [body(1, 1.0, Mat3::IDENTITY)],
                vec![contact(1, 2, [0.0; 3], [0.0; 3])],
            ).err(),
            Some(DelassusError::UnknownBodyId(BodyId(2)))
        );
        assert_eq!(
            ContactBlock::new(BodyId(1), BodyId(1), [0.0; 3], [0.0; 3], frame()).err(),
            Some(DelassusError::SameBodyContact)
        );
    }

    #[test]
    fn rejects_invalid_mass_and_inertia() {
        assert_eq!(
            BodyMassProperties::new(BodyId(1), f64::NAN, Mat3::ZERO).err(),
            Some(DelassusError::InvalidInverseMass)
        );
        assert_eq!(
            BodyMassProperties::new(BodyId(1), -0.1, Mat3::ZERO).err(),
            Some(DelassusError::InvalidInverseMass)
        );
        assert_eq!(
            BodyMassProperties::new(
                BodyId(1), 1.0,
                Mat3 { rows: [[1.0, 0.5, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] },
            ).err(),
            Some(DelassusError::InvalidInverseInertia)
        );
        assert_eq!(
            BodyMassProperties::new(
                BodyId(1), 1.0,
                Mat3 { rows: [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]] },
            ).err(),
            Some(DelassusError::InvalidInverseInertia)
        );
    }

    #[test]
    fn rejects_invalid_contact_frames_and_lever_arms() {
        assert_eq!(
            ContactFrame::new([2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]).err(),
            Some(DelassusError::InvalidContactFrame)
        );
        assert_eq!(
            ContactFrame::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]).err(),
            Some(DelassusError::InvalidContactFrame)
        );
        assert_eq!(
            ContactBlock::new(
                BodyId(1), BodyId(2), [f64::INFINITY, 0.0, 0.0], [0.0; 3], frame(),
            ).err(),
            Some(DelassusError::InvalidLeverArm)
        );
    }

    #[test]
    fn rejects_wrong_impulse_dimension_and_nonfinite_impulses() {
        let op = DelassusOperator::new(
            [body(1, 0.5, Mat3::IDENTITY), body(2, 0.5, Mat3::IDENTITY)],
            vec![contact(1, 2, [0.0; 3], [0.0; 3])],
        ).unwrap();
        assert_eq!(
            op.apply(&[1.0, 2.0]).err(),
            Some(DelassusError::WrongImpulseDimension { expected: 3, actual: 2 })
        );
        assert_eq!(
            op.apply(&[1.0, f64::NAN, 3.0]).err(),
            Some(DelassusError::NonFiniteImpulse)
        );
    }

    #[test]
    fn rejects_nonfinite_delassus_output_after_arithmetic_overflow() {
        let op = DelassusOperator::new(
            [
                body(1, f64::MAX, Mat3::ZERO),
                body(2, f64::MAX, Mat3::ZERO),
            ],
            vec![contact(1, 2, [0.0; 3], [0.0; 3])],
        ).unwrap();
        assert_eq!(
            op.apply(&[f64::MAX, 0.0, 0.0]).err(),
            Some(DelassusError::NonFiniteResult)
        );
    }

    #[test]
    fn empty_contact_set_is_zero_dimensional() {
        let op = DelassusOperator::new([body(1, 1.0, Mat3::IDENTITY)], Vec::new()).unwrap();
        assert_eq!(op.dimension(), 0);
        assert_eq!(op.apply(&[]).unwrap(), Vec::<f64>::new());
        assert_eq!(op.materialize_dense().unwrap(), Vec::<Vec<f64>>::new());
    }
}
