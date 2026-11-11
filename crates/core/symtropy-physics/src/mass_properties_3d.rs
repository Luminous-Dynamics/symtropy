// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked analytical mass properties for centered uniform-density 3D primitives.
//!
//! The production solver still evolves generic N-D inertia approximately. This
//! module only establishes trustworthy 3D principal moments for primitive body
//! construction and validation. It deliberately keeps compound/tensor mass
//! properties out of scope until the full parallel-axis migration is reviewed.

use nalgebra::SVector;

use crate::angular_dynamics::PrincipalInertia3;

/// Mass and body-space principal inertia about the center of mass.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MassProperties3 {
    pub mass: f64,
    pub center_of_mass: SVector<f64, 3>,
    pub principal_inertia: PrincipalInertia3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MassProperties3Error {
    InvalidMass,
    InvalidRadius,
    InvalidHalfExtent { axis: usize },
    InvalidHalfHeight,
    InvalidAxis,
    UnrepresentableGeometry,
    UnrepresentableInertia,
}

impl MassProperties3 {
    /// Uniform solid sphere centered at the body origin.
    ///
    /// `I = 2/5 m r^2` about every principal axis.
    pub fn solid_sphere(mass: f64, radius: f64) -> Result<Self, MassProperties3Error> {
        validate_mass(mass)?;
        if !radius.is_finite() || radius <= 0.0 {
            return Err(MassProperties3Error::InvalidRadius);
        }

        let radius_squared = checked_square(radius)?;
        let moment = checked_mul(0.4 * mass, radius_squared)?;
        Self::from_moments(mass, [moment; 3])
    }

    /// Uniform solid cuboid centered at the body origin.
    ///
    /// `half_extents = [hx, hy, hz]`, so full side lengths are `2h`.
    /// The principal moments are:
    ///
    /// - `Ixx = m/3 (hy^2 + hz^2)`
    /// - `Iyy = m/3 (hx^2 + hz^2)`
    /// - `Izz = m/3 (hx^2 + hy^2)`
    pub fn solid_cuboid(
        mass: f64,
        half_extents: [f64; 3],
    ) -> Result<Self, MassProperties3Error> {
        validate_mass(mass)?;
        for (axis, half_extent) in half_extents.iter().copied().enumerate() {
            if !half_extent.is_finite() || half_extent <= 0.0 {
                return Err(MassProperties3Error::InvalidHalfExtent { axis });
            }
        }

        let hx2 = checked_square(half_extents[0])?;
        let hy2 = checked_square(half_extents[1])?;
        let hz2 = checked_square(half_extents[2])?;
        let scale = mass / 3.0;
        if !scale.is_finite() || scale <= 0.0 {
            return Err(MassProperties3Error::UnrepresentableInertia);
        }

        let ixx = checked_mul(scale, checked_add(hy2, hz2)?)?;
        let iyy = checked_mul(scale, checked_add(hx2, hz2)?)?;
        let izz = checked_mul(scale, checked_add(hx2, hy2)?)?;
        Self::from_moments(mass, [ixx, iyy, izz])
    }

    /// Uniform solid capsule centered at the body origin.
    ///
    /// Geometry matches `symtropy_math::Capsule<3>`: a cylinder whose
    /// hemisphere centers are separated by `2 * half_height`, swept by a sphere
    /// of `radius`, aligned to `axis`.
    ///
    /// Mass is partitioned by exact uniform-density cylinder/spherical-cap
    /// volume ratio. The pair of hemispheres is treated analytically, including
    /// each hemisphere COM offset (`3r/8`) in the perpendicular-axis inertia.
    pub fn solid_capsule(
        mass: f64,
        half_height: f64,
        radius: f64,
        axis: usize,
    ) -> Result<Self, MassProperties3Error> {
        validate_mass(mass)?;
        if !half_height.is_finite() || half_height < 0.0 {
            return Err(MassProperties3Error::InvalidHalfHeight);
        }
        if !radius.is_finite() || radius <= 0.0 {
            return Err(MassProperties3Error::InvalidRadius);
        }
        if axis >= 3 {
            return Err(MassProperties3Error::InvalidAxis);
        }

        // V_cylinder : V_two_hemispheres = 2 h : 4 r / 3 after the common
        // factor pi*r^2 is canceled. Normalize first so extreme finite lengths
        // cannot overflow merely while computing the mass fractions.
        let length_scale = half_height.max(radius);
        if !length_scale.is_finite() || length_scale <= 0.0 {
            return Err(MassProperties3Error::UnrepresentableGeometry);
        }
        let cylinder_weight = 2.0 * (half_height / length_scale);
        let caps_weight = (4.0 / 3.0) * (radius / length_scale);
        let total_weight = checked_add(cylinder_weight, caps_weight)?;
        if total_weight <= 0.0 {
            return Err(MassProperties3Error::UnrepresentableGeometry);
        }

        let cylinder_mass = checked_mul(mass, cylinder_weight / total_weight)?;
        let caps_mass = checked_mul(mass, caps_weight / total_weight)?;
        let r2 = checked_square(radius)?;
        let h2 = checked_square(half_height)?;

        // Symmetry-axis moment: cylinder + the two hemispheres (whose axial
        // displacement does not change inertia about the same axis).
        let cylinder_axis = checked_mul(0.5 * cylinder_mass, r2)?;
        let caps_axis = checked_mul(0.4 * caps_mass, r2)?;
        let i_axis = checked_add(cylinder_axis, caps_axis)?;

        // Cylinder perpendicular moment about the capsule center.
        let cylinder_bracket = checked_add(checked_mul(3.0, r2)?, checked_mul(4.0, h2)?)?;
        let cylinder_perp = checked_mul(cylinder_mass / 12.0, cylinder_bracket)?;

        // A solid hemisphere has I_perp about its own COM = 83/320 m_h r^2.
        // Its COM lies 3r/8 beyond the hemisphere-center plane. For both caps,
        // 2*m_h == caps_mass, so the combined parallel-axis term is written
        // directly with caps_mass.
        let hemisphere_intrinsic = checked_mul(83.0 / 320.0, r2)?;
        let cap_com_offset = checked_add(half_height, checked_mul(3.0 / 8.0, radius)?)?;
        let cap_offset_squared = checked_square(cap_com_offset)?;
        let caps_bracket = checked_add(hemisphere_intrinsic, cap_offset_squared)?;
        let caps_perp = checked_mul(caps_mass, caps_bracket)?;
        let i_perp = checked_add(cylinder_perp, caps_perp)?;

        let mut moments = [i_perp; 3];
        moments[axis] = i_axis;
        Self::from_moments(mass, moments)
    }

    fn from_moments(mass: f64, moments: [f64; 3]) -> Result<Self, MassProperties3Error> {
        if moments.iter().any(|moment| !moment.is_finite() || *moment <= 0.0) {
            return Err(MassProperties3Error::UnrepresentableInertia);
        }
        let principal_inertia = PrincipalInertia3::new(moments)
            .map_err(|_| MassProperties3Error::UnrepresentableInertia)?;
        Ok(Self {
            mass,
            center_of_mass: SVector::zeros(),
            principal_inertia,
        })
    }
}

fn validate_mass(mass: f64) -> Result<(), MassProperties3Error> {
    if mass.is_finite() && mass > 0.0 {
        Ok(())
    } else {
        Err(MassProperties3Error::InvalidMass)
    }
}

fn checked_square(value: f64) -> Result<f64, MassProperties3Error> {
    checked_mul(value, value).map_err(|_| MassProperties3Error::UnrepresentableGeometry)
}

fn checked_add(lhs: f64, rhs: f64) -> Result<f64, MassProperties3Error> {
    let result = lhs + rhs;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(MassProperties3Error::UnrepresentableGeometry)
    }
}

fn checked_mul(lhs: f64, rhs: f64) -> Result<f64, MassProperties3Error> {
    let result = lhs * rhs;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(MassProperties3Error::UnrepresentableInertia)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_matches_two_fifths_mr_squared() {
        let properties = MassProperties3::solid_sphere(10.0, 2.0).unwrap();
        assert_eq!(properties.principal_inertia.moments(), [16.0, 16.0, 16.0]);
    }

    #[test]
    fn cuboid_uses_half_extent_formula() {
        let properties = MassProperties3::solid_cuboid(12.0, [1.0, 2.0, 3.0]).unwrap();
        let [ixx, iyy, izz] = properties.principal_inertia.moments();
        assert!((ixx - 52.0).abs() < 1.0e-12);
        assert!((iyy - 40.0).abs() < 1.0e-12);
        assert!((izz - 20.0).abs() < 1.0e-12);
    }

    #[test]
    fn zero_half_height_capsule_reduces_to_sphere() {
        let capsule = MassProperties3::solid_capsule(10.0, 0.0, 2.0, 1).unwrap();
        let sphere = MassProperties3::solid_sphere(10.0, 2.0).unwrap();
        let a = capsule.principal_inertia.moments();
        let b = sphere.principal_inertia.moments();
        for axis in 0..3 {
            assert!((a[axis] - b[axis]).abs() < 1.0e-12);
        }
    }

    #[test]
    fn capsule_matches_independent_analytical_case() {
        let properties = MassProperties3::solid_capsule(10.0, 2.0, 0.5, 1).unwrap();
        let [ixx, iyy, izz] = properties.principal_inertia.moments();
        assert!((iyy - 1.214_285_714_285_714_2).abs() < 1.0e-12);
        assert!((ixx - 18.892_857_142_857_142).abs() < 1.0e-12);
        assert!((izz - ixx).abs() < 1.0e-12);
    }

    #[test]
    fn capsule_axis_permutation_only_permutes_principal_moments() {
        let x = MassProperties3::solid_capsule(5.0, 1.5, 0.4, 0)
            .unwrap()
            .principal_inertia
            .moments();
        let z = MassProperties3::solid_capsule(5.0, 1.5, 0.4, 2)
            .unwrap()
            .principal_inertia
            .moments();
        assert!((x[0] - z[2]).abs() < 1.0e-12);
        assert!((x[1] - z[1]).abs() < 1.0e-12);
        assert!((x[2] - z[0]).abs() < 1.0e-12);
    }

    #[test]
    fn invalid_geometry_and_mass_fail_closed() {
        assert_eq!(
            MassProperties3::solid_sphere(f64::NAN, 1.0),
            Err(MassProperties3Error::InvalidMass)
        );
        assert_eq!(
            MassProperties3::solid_sphere(1.0, 0.0),
            Err(MassProperties3Error::InvalidRadius)
        );
        assert_eq!(
            MassProperties3::solid_cuboid(1.0, [1.0, -1.0, 1.0]),
            Err(MassProperties3Error::InvalidHalfExtent { axis: 1 })
        );
        assert_eq!(
            MassProperties3::solid_capsule(1.0, -1.0, 1.0, 1),
            Err(MassProperties3Error::InvalidHalfHeight)
        );
        assert_eq!(
            MassProperties3::solid_capsule(1.0, 1.0, 1.0, 3),
            Err(MassProperties3Error::InvalidAxis)
        );
    }

    #[test]
    fn finite_extreme_geometry_never_emits_infinite_inertia() {
        assert!(matches!(
            MassProperties3::solid_sphere(1.0, f64::MAX),
            Err(MassProperties3Error::UnrepresentableGeometry)
                | Err(MassProperties3Error::UnrepresentableInertia)
        ));
        assert!(matches!(
            MassProperties3::solid_cuboid(1.0, [f64::MAX, 1.0, 1.0]),
            Err(MassProperties3Error::UnrepresentableGeometry)
                | Err(MassProperties3Error::UnrepresentableInertia)
        ));
        assert!(matches!(
            MassProperties3::solid_capsule(1.0, f64::MAX, 1.0, 1),
            Err(MassProperties3Error::UnrepresentableGeometry)
                | Err(MassProperties3Error::UnrepresentableInertia)
        ));
    }
}
