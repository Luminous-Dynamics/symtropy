#![forbid(unsafe_code)]

//! Analytic free-space field of ideal point magnetic dipoles.
//!
//! This is a coordinate, sign, unit and superposition reference only. It is
//! not a finite-magnet solver and omits nonlinear B-H laws, finite-body
//! demagnetization, hysteresis, eddy currents, temperature, mechanics and
//! manufacturing.
//!
//! B(r) = (mu0 / 4pi) * (3 * (m dot r_hat) * r_hat - m) / |r|^3
//! mu0 / 4pi = 1e-7 T m / A.

use std::fmt;

/// Right-handed Cartesian position in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionMeters(pub [f64; 3]);

/// Ideal point-dipole moment in ampere-square-metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DipoleMomentAm2(pub [f64; 3]);

/// Magnetic flux density in tesla.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MagneticFluxDensityTesla(pub [f64; 3]);

impl MagneticFluxDensityTesla {
    #[must_use]
    pub fn magnitude(self) -> f64 {
        self.0[0].hypot(self.0[1]).hypot(self.0[2])
    }
}

/// Ideal point dipole in free space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointDipole {
    pub position: PositionMeters,
    pub moment: DipoleMomentAm2,
}

impl PointDipole {
    #[must_use]
    pub const fn new(position_m: [f64; 3], moment_am2: [f64; 3]) -> Self {
        Self {
            position: PositionMeters(position_m),
            moment: DipoleMomentAm2(moment_am2),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldError {
    NonFinitePosition,
    NonFiniteMoment,
    CoincidentQuery,
    NonFiniteField,
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePosition => f.write_str("position contains a non-finite value"),
            Self::NonFiniteMoment => f.write_str("dipole moment contains a non-finite value"),
            Self::CoincidentQuery => f.write_str("field is undefined at a point-dipole location"),
            Self::NonFiniteField => f.write_str("field arithmetic produced a non-finite value"),
        }
    }
}

impl std::error::Error for FieldError {}

const MU0_OVER_4PI: f64 = 1.0e-7;

/// Compute the linear superposition of ideal point-dipole fields in free space.
///
/// Empty input returns exactly zero. Inputs and outputs use SI units.
///
/// # Errors
/// Returns an error for non-finite input, a query at a dipole singularity, or
/// an overflow/non-finite intermediate result.
pub fn field_at(
    dipoles: &[PointDipole],
    query: PositionMeters,
) -> Result<MagneticFluxDensityTesla, FieldError> {
    if !finite(query.0) {
        return Err(FieldError::NonFinitePosition);
    }

    let mut total = [0.0; 3];
    for dipole in dipoles {
        if !finite(dipole.position.0) {
            return Err(FieldError::NonFinitePosition);
        }
        if !finite(dipole.moment.0) {
            return Err(FieldError::NonFiniteMoment);
        }

        let r = sub(query.0, dipole.position.0);
        if !finite(r) {
            return Err(FieldError::NonFiniteField);
        }
        if r == [0.0; 3] {
            return Err(FieldError::CoincidentQuery);
        }

        // hypot avoids squaring small or large coordinates before finding
        // their norm; radius^3 is checked separately for representability.
        let radius = r[0].hypot(r[1]).hypot(r[2]);
        if !radius.is_finite() || radius == 0.0 {
            return Err(FieldError::NonFiniteField);
        }
        let r3 = radius * radius * radius;
        if !r3.is_finite() || r3 == 0.0 {
            return Err(FieldError::NonFiniteField);
        }

        let r_hat = [r[0] / radius, r[1] / radius, r[2] / radius];
        let m_dot_rhat = dot(dipole.moment.0, r_hat);
        if !m_dot_rhat.is_finite() {
            return Err(FieldError::NonFiniteField);
        }

        let angular = sub(mul(r_hat, 3.0 * m_dot_rhat), dipole.moment.0);
        let contribution = mul(angular, MU0_OVER_4PI / r3);
        if !finite(contribution) {
            return Err(FieldError::NonFiniteField);
        }

        total = add(total, contribution);
        if !finite(total) {
            return Err(FieldError::NonFiniteField);
        }
    }

    Ok(MagneticFluxDensityTesla(total))
}

fn finite(v: [f64; 3]) -> bool {
    v.iter().all(|x| x.is_finite())
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn mul(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f64 = 1.0e-12;

    fn close(actual: [f64; 3], expected: [f64; 3]) {
        for (axis, (actual_value, expected_value)) in
            actual.iter().zip(expected.iter()).enumerate()
        {
            assert!(
                (actual_value - expected_value).abs() <= TOL,
                "axis {axis}: actual {:?}, expected {:?}",
                actual,
                expected
            );
        }
    }

    fn rz(v: [f64; 3]) -> [f64; 3] {
        [-v[1], v[0], v[2]]
    }

    #[test]
    fn axial_and_equatorial_known_answers() {
        let d = PointDipole::new([0.0; 3], [0.0, 0.0, 1.0]);
        let axial = field_at(&[d], PositionMeters([0.0, 0.0, 0.1])).unwrap();
        let equatorial = field_at(&[d], PositionMeters([0.1, 0.0, 0.0])).unwrap();
        close(axial.0, [0.0, 0.0, 2.0e-4]);
        close(equatorial.0, [0.0, 0.0, -1.0e-4]);
    }

    #[test]
    fn symmetric_pair_known_answer() {
        let ds = [
            PointDipole::new([-0.05, 0.0, 0.0], [0.0, 0.0, 1.0]),
            PointDipole::new([0.05, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ];
        let f = field_at(&ds, PositionMeters([0.0, 0.0, 0.1])).unwrap();
        close(f.0, [0.0, 0.0, 2.003_516_907_839_811e-4]);
    }

    #[test]
    fn off_axis_pair_known_answer_and_magnitude() {
        let ds = [
            PointDipole::new([-0.05, 0.0, 0.0], [0.0, 0.0, 1.0]),
            PointDipole::new([0.05, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ];
        let f = field_at(&ds, PositionMeters([0.0, 0.05, 0.1])).unwrap();
        close(f.0, [0.0, 1.088_662_107_903_634_5e-4, 1.088_662_107_903_634_5e-4]);
        assert!((f.magnitude() - 1.539_600_717_839_001_6e-4).abs() <= TOL);
    }

    #[test]
    fn superposition_and_moment_scaling() {
        let a = PointDipole::new([-0.05, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let b = PointDipole::new([0.05, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let q = PositionMeters([0.0, 0.05, 0.1]);
        let both = field_at(&[a, b], q).unwrap();
        let fa = field_at(&[a], q).unwrap();
        let fb = field_at(&[b], q).unwrap();
        close(both.0, add(fa.0, fb.0));

        let triple = PointDipole::new([0.0; 3], [0.0, 0.0, 3.0]);
        let single = PointDipole::new([0.0; 3], [0.0, 0.0, 1.0]);
        let f1 = field_at(&[single], q).unwrap();
        let f3 = field_at(&[triple], q).unwrap();
        close(f3.0, mul(f1.0, 3.0));
    }

    #[test]
    fn inverse_cube_scaling() {
        let d = PointDipole::new([0.0; 3], [0.0, 0.0, 1.0]);
        let near = field_at(&[d], PositionMeters([0.0, 0.0, 0.1])).unwrap();
        let far = field_at(&[d], PositionMeters([0.0, 0.0, 0.2])).unwrap();
        close(far.0, mul(near.0, 1.0 / 8.0));
    }

    #[test]
    fn moment_reversal_reverses_field() {
        let pos = PointDipole::new([0.0; 3], [0.2, -0.4, 1.0]);
        let neg = PointDipole::new([0.0; 3], [-0.2, 0.4, -1.0]);
        let q = PositionMeters([0.03, 0.02, 0.07]);
        let fp = field_at(&[pos], q).unwrap();
        let fn_ = field_at(&[neg], q).unwrap();
        close(fn_.0, mul(fp.0, -1.0));
    }

    #[test]
    fn rotation_covariance_about_z() {
        let d = PointDipole::new([0.02, -0.01, 0.03], [0.2, 0.5, -0.7]);
        let q = PositionMeters([0.08, 0.04, 0.11]);
        let f = field_at(&[d], q).unwrap();
        let dr = PointDipole::new(rz(d.position.0), rz(d.moment.0));
        let qr = PositionMeters(rz(q.0));
        let fr = field_at(&[dr], qr).unwrap();
        close(fr.0, rz(f.0));
    }

    #[test]
    fn singularity_and_non_finite_inputs_are_rejected() {
        let d = PointDipole::new([1.0, 2.0, 3.0], [0.0, 0.0, 1.0]);
        assert_eq!(field_at(&[d], PositionMeters([1.0, 2.0, 3.0])), Err(FieldError::CoincidentQuery));

        // A nonzero distance whose cube is not representable is not a
        // geometric coincidence; its field is outside this f64 oracle's range.
        let tiny = PointDipole::new([0.0; 3], [0.0, 0.0, 1.0]);
        assert_eq!(
            field_at(&[tiny], PositionMeters([1.0e-200, 0.0, 0.0])),
            Err(FieldError::NonFiniteField)
        );
        assert_eq!(field_at(&[], PositionMeters([f64::NAN, 0.0, 0.0])), Err(FieldError::NonFinitePosition));

        let bad_pos = PointDipole::new([f64::INFINITY, 0.0, 0.0], [0.0, 0.0, 1.0]);
        assert_eq!(field_at(&[bad_pos], PositionMeters([0.0, 0.0, 1.0])), Err(FieldError::NonFinitePosition));

        let bad_moment = PointDipole {
            position: PositionMeters([0.0; 3]),
            moment: DipoleMomentAm2([0.0, f64::NAN, 0.0]),
        };
        assert_eq!(field_at(&[bad_moment], PositionMeters([0.0, 0.0, 1.0])), Err(FieldError::NonFiniteMoment));
    }

    #[test]
    fn empty_set_returns_zero() {
        assert_eq!(
            field_at(&[], PositionMeters([1.0, 2.0, 3.0])).unwrap(),
            MagneticFluxDensityTesla([0.0, 0.0, 0.0])
        );
    }
}
