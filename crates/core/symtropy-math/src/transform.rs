// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use crate::point::Point;
use crate::rotor::Rotor;
use nalgebra::SVector;

/// N-dimensional rigid body transform: rotation + translation.
/// Applied as: p' = R(p) + t
#[derive(Clone, Debug)]
pub struct Transform<const D: usize> {
    pub translation: Point<D>,
    pub rotation: Rotor<D>,
}

impl<const D: usize> Transform<D> {
    pub fn identity() -> Self {
        Self {
            translation: Point::origin(),
            rotation: Rotor::identity(),
        }
    }

    pub fn from_translation(t: Point<D>) -> Self {
        Self {
            translation: t,
            rotation: Rotor::identity(),
        }
    }

    pub fn from_rotation(r: Rotor<D>) -> Self {
        Self {
            translation: Point::origin(),
            rotation: r,
        }
    }

    #[inline]
    pub fn transform_point(&self, point: &Point<D>) -> Point<D> {
        let rotated = self.rotation.rotate_point(point);
        Point(rotated.0 + self.translation.0)
    }

    #[inline]
    pub fn transform_vector(&self, v: &SVector<f64, D>) -> SVector<f64, D> {
        self.rotation.rotate_vector(v)
    }

    pub fn compose(&self, other: &Self) -> Self {
        let rotation = self.rotation.compose(&other.rotation);
        let t = self.rotation.rotate_vector(&other.translation.0);
        Self {
            translation: Point(self.translation.0 + t),
            rotation,
        }
    }

    pub fn inverse(&self) -> Self {
        let inv_rot = self.rotation.reverse();
        let neg_t = inv_rot.rotate_vector(&self.translation.0) * -1.0;
        Self {
            translation: Point(neg_t),
            rotation: inv_rot,
        }
    }

    pub fn interpolate(&self, other: &Self, t: f64) -> Self {
        let translation = self.translation.lerp(&other.translation, t);
        let relative = other.rotation.compose(&self.rotation.reverse());
        let rotation = relative.slerp(t).compose(&self.rotation);
        Self {
            translation,
            rotation,
        }
    }
}

impl Transform<3> {
    /// Interpolate a 3D rigid transform using linear translation and a checked
    /// principal SO(3) geodesic for rotation.
    ///
    /// Unlike the legacy dimension-generic `interpolate` method, this path
    /// returns a typed error for invalid rotations, non-finite parameters, or
    /// an unresolved logarithm branch instead of silently substituting a
    /// fallback orientation.
    pub fn try_interpolate(
        &self,
        other: &Self,
        t: f64,
    ) -> Result<Self, crate::lie::So3LogError> {
        if !self.translation.0.iter().all(|value| value.is_finite())
            || !other.translation.0.iter().all(|value| value.is_finite())
        {
            return Err(crate::lie::So3LogError::NonFiniteTranslation);
        }
        let rotation = crate::lie::so3_interpolate(&self.rotation, &other.rotation, t)?;
        let translation = self.translation.lerp(&other.translation, t);
        if !translation.0.iter().all(|value| value.is_finite()) {
            return Err(crate::lie::So3LogError::NonFiniteTranslation);
        }
        Ok(Self {
            translation,
            rotation,
        })
    }
}

impl Transform<4> {
    /// Interpolate a 4D rigid transform with a checked multi-plane SO(4)
    /// principal geodesic. Unlike Rotor::slerp, this preserves two independent
    /// rotation planes and returns typed errors for invalid inputs/branches.
    pub fn try_interpolate(
        &self,
        other: &Self,
        t: f64,
    ) -> Result<Self, crate::so4::So4LogError> {
        if !self.translation.0.iter().all(|value| value.is_finite())
            || !other.translation.0.iter().all(|value| value.is_finite())
        {
            return Err(crate::so4::So4LogError::NonFiniteTranslation);
        }
        let rotation = crate::so4::so4_interpolate(&self.rotation, &other.rotation, t)?;
        let translation = self.translation.lerp(&other.translation, t);
        if !translation.0.iter().all(|value| value.is_finite()) {
            return Err(crate::so4::So4LogError::NonFiniteTranslation);
        }
        Ok(Self { translation, rotation })
    }
}

impl<const D: usize> Default for Transform<D> {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bivector::Bivector;
    use std::f64::consts::FRAC_PI_2;

    fn approx_vec<const N: usize>(a: &SVector<f64, N>, b: &SVector<f64, N>) -> bool {
        a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1e-10)
    }

    #[test]
    fn identity_preserves() {
        let t = Transform::<3>::identity();
        let p = Point::new([1.0, 2.0, 3.0]);
        assert!(approx_vec(&p.0, &t.transform_point(&p).0));
    }

    #[test]
    fn pure_translation() {
        let t = Transform::from_translation(Point::new([10.0, 20.0]));
        let p = Point::new([1.0, 2.0]);
        let q = t.transform_point(&p);
        assert!((q.coord(0) - 11.0).abs() < 1e-10);
        assert!((q.coord(1) - 22.0).abs() < 1e-10);
    }

    #[test]
    fn inverse_roundtrip() {
        let plane = Bivector::<4>::unit_plane(0, 2);
        let t = Transform {
            translation: Point::new([1.0, 2.0, 3.0, 4.0]),
            rotation: Rotor::from_plane_angle(&plane, 1.0),
        };
        let p = Point::new([5.0, 6.0, 7.0, 8.0]);
        let back = t.inverse().transform_point(&t.transform_point(&p));
        assert!(approx_vec(&p.0, &back.0));
    }

    #[test]
    fn compose_equals_sequential() {
        let t1 = Transform {
            translation: Point::new([1.0, 0.0, 0.0]),
            rotation: Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 1), 0.5),
        };
        let t2 = Transform {
            translation: Point::new([0.0, 2.0, 0.0]),
            rotation: Rotor::from_plane_angle(&Bivector::<3>::unit_plane(1, 2), 0.3),
        };
        let p = Point::new([1.0, 1.0, 1.0]);
        let seq = t2.transform_point(&t1.transform_point(&p));
        let composed = t2.compose(&t1).transform_point(&p);
        assert!(approx_vec(&seq.0, &composed.0));
    }

    #[test]
    fn rotate_then_translate() {
        let t = Transform {
            translation: Point::new([5.0, 0.0, 0.0]),
            rotation: Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 1), FRAC_PI_2),
        };
        let p = Point::new([1.0, 0.0, 0.0]);
        let q = t.transform_point(&p);
        assert!((q.coord(0) - 5.0).abs() < 1e-10);
        assert!((q.coord(1) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn checked_so3_interpolation_handles_exact_pi_rotation() {
        use std::f64::consts::PI;

        let start = Transform::<3>::identity();
        let end = Transform {
            translation: Point::new([2.0, 0.0, 0.0]),
            rotation: Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 1), PI),
        };

        let midpoint = start.try_interpolate(&end, 0.5).expect("valid SO(3) interpolation");
        assert!((midpoint.translation.coord(0) - 1.0).abs() < 1.0e-12);
        assert!(midpoint.translation.coord(1).abs() < 1.0e-12);

        let rotated = midpoint.transform_vector(&SVector::from([1.0, 0.0, 0.0]));
        assert!(rotated[0].abs() < 1.0e-10);
        assert!((rotated[1] - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn checked_so3_interpolation_rejects_non_finite_translation_input_and_result() {
        let identity = Transform::<3>::identity();
        let invalid = Transform {
            translation: Point::new([f64::NAN, 0.0, 0.0]),
            rotation: Rotor::<3>::identity(),
        };
        assert!(matches!(
            invalid.try_interpolate(&identity, 0.5),
            Err(crate::lie::So3LogError::NonFiniteTranslation)
        ));

        let huge = Transform {
            translation: Point::new([1.0e308, 0.0, 0.0]),
            rotation: Rotor::<3>::identity(),
        };
        assert!(matches!(
            identity.try_interpolate(&huge, 2.0),
            Err(crate::lie::So3LogError::NonFiniteTranslation)
        ));
    }

    #[test]
    fn checked_so3_interpolation_rejects_invalid_endpoint_rotation() {
        let start = Transform::<3>::identity();
        let invalid_rotation = Rotor::from_matrix(
            nalgebra::SMatrix::<f64, 3, 3>::identity() * 2.0,
        );
        let end = Transform {
            translation: Point::origin(),
            rotation: invalid_rotation,
        };

        assert!(matches!(
            start.try_interpolate(&end, 0.5),
            Err(crate::lie::So3LogError::InvalidRotation { .. })
        ));
    }

    #[test]
    fn checked_so4_interpolation_preserves_unequal_double_rotation() {
        let start = Transform::<4>::identity();
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, std::f64::consts::FRAC_PI_2);
        generator.set(2, 3, -0.8);
        let end = Transform {
            translation: Point::new([1.0, 2.0, 3.0, 4.0]),
            rotation: Rotor::from_bivector(&generator),
        };

        let midpoint = start.try_interpolate(&end, 0.5).expect("valid SO(4) interpolation");
        assert!((midpoint.translation.coord(0) - 0.5).abs() < 1.0e-12);
        assert!(midpoint.rotation.is_proper_rotation(1.0e-10));

        let x = SVector::from([1.0, 0.0, 0.0, 0.0]);
        let rotated = midpoint.transform_vector(&x);
        assert!((rotated[0] - std::f64::consts::FRAC_PI_4.cos()).abs() < 1.0e-10);
        assert!((rotated[1] - std::f64::consts::FRAC_PI_4.sin()).abs() < 1.0e-10);
    }

    #[test]
    fn checked_so4_interpolation_rejects_non_finite_translation_input_and_result() {
        let identity = Transform::<4>::identity();
        let invalid = Transform {
            translation: Point::new([0.0, f64::INFINITY, 0.0, 0.0]),
            rotation: Rotor::<4>::identity(),
        };
        assert!(matches!(
            invalid.try_interpolate(&identity, 0.5),
            Err(crate::so4::So4LogError::NonFiniteTranslation)
        ));

        let huge = Transform {
            translation: Point::new([1.0e308, 0.0, 0.0, 0.0]),
            rotation: Rotor::<4>::identity(),
        };
        assert!(matches!(
            identity.try_interpolate(&huge, 2.0),
            Err(crate::so4::So4LogError::NonFiniteTranslation)
        ));
    }

    #[test]
    fn checked_so4_interpolation_handles_exact_pi_and_rejects_invalid_pose() {
        let start = Transform::<4>::identity();
        let mut half_turn = Bivector::<4>::zero();
        half_turn.set(0, 1, std::f64::consts::PI);
        let end = Transform::from_rotation(Rotor::from_bivector(&half_turn));

        let midpoint = start.try_interpolate(&end, 0.5).expect("valid SO(4) half-turn");
        assert!(midpoint.rotation.is_proper_rotation(1.0e-10));
        let rotated = midpoint.transform_vector(&SVector::from([1.0, 0.0, 0.0, 0.0]));
        assert!(rotated[0].abs() < 1.0e-10);
        assert!((rotated[1] - 1.0).abs() < 1.0e-10);

        let invalid = Transform {
            translation: Point::origin(),
            rotation: Rotor::from_matrix(nalgebra::SMatrix::<f64, 4, 4>::identity() * 2.0),
        };
        assert!(matches!(
            start.try_interpolate(&invalid, 0.5),
            Err(crate::so4::So4LogError::InvalidRotation { .. })
        ));
    }

}
