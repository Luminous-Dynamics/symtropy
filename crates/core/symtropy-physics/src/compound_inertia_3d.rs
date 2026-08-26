// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked 3D compound mass-property composition using a full body-frame
//! inertia tensor and the parallel-axis theorem.
//!
//! This module is a reference/evidence layer. `RigidBody<3>` still stores only
//! three diagonal inertia values in its body frame, so a general non-diagonal
//! compound tensor cannot yet be represented losslessly by production state.

use nalgebra::{SMatrix, SVector};
use symtropy_math::Transform;

use crate::mass_properties_3d::MassProperties3;

const ROTATION_VALIDITY_TOLERANCE: f64 = 1.0e-8;

/// One child contribution to a compound body.
///
/// `local_transform.rotation` maps the child's principal-inertia frame into the
/// compound body frame. `local_transform.translation` locates the child's local
/// origin in the compound body frame.
#[derive(Clone, Debug)]
pub struct CompoundMassPart3 {
    /// Stable identity used to canonicalize floating-point accumulation order.
    pub part_id: u64,
    pub local_transform: Transform<3>,
    pub properties: MassProperties3,
}

/// Checked compound mass properties expressed in the chosen compound body frame.
#[derive(Clone, Debug, PartialEq)]
pub struct CompoundMassProperties3 {
    pub total_mass: f64,
    pub center_of_mass: SVector<f64, 3>,
    /// Full symmetric inertia tensor about `center_of_mass`, expressed in the
    /// compound body frame.
    pub inertia_tensor_body: SMatrix<f64, 3, 3>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CompoundInertia3Error {
    EmptyCompound,
    DuplicatePartId(u64),
    InvalidPartMass { part_id: u64 },
    NonFinitePartCenterOfMass { part_id: u64 },
    NonFinitePartTranslation { part_id: u64 },
    InvalidPartRotation { part_id: u64 },
    InvalidPartInertia { part_id: u64 },
    UnrepresentableTotalMass,
    UnrepresentableCenterOfMass,
    UnrepresentableTensor,
    NonPositiveDefiniteTensor,
    InvalidTolerance,
}

impl CompoundMassProperties3 {
    /// Compose a set of child mass properties in a deterministic `part_id` order.
    pub fn compose(parts: &[CompoundMassPart3]) -> Result<Self, CompoundInertia3Error> {
        if parts.is_empty() {
            return Err(CompoundInertia3Error::EmptyCompound);
        }

        let mut ordered: Vec<&CompoundMassPart3> = parts.iter().collect();
        ordered.sort_by_key(|part| part.part_id);
        for pair in ordered.windows(2) {
            if pair[0].part_id == pair[1].part_id {
                return Err(CompoundInertia3Error::DuplicatePartId(pair[0].part_id));
            }
        }

        for part in &ordered {
            validate_part(part)?;
        }

        // Incremental center-of-mass accumulation avoids forming m*r directly,
        // which can overflow even when the final weighted mean is representable.
        let mut total_mass = 0.0_f64;
        let mut center_of_mass = SVector::<f64, 3>::zeros();
        let mut child_centers = Vec::with_capacity(ordered.len());

        for part in &ordered {
            let child_center = child_center_in_compound(part)?;
            child_centers.push(child_center);

            let next_total = total_mass + part.properties.mass;
            if !next_total.is_finite() || next_total <= 0.0 {
                return Err(CompoundInertia3Error::UnrepresentableTotalMass);
            }

            if total_mass == 0.0 {
                center_of_mass = child_center;
            } else {
                let delta = child_center - center_of_mass;
                if !vector_is_finite(&delta) {
                    return Err(CompoundInertia3Error::UnrepresentableCenterOfMass);
                }
                let ratio = part.properties.mass / next_total;
                if !ratio.is_finite() || ratio <= 0.0 || ratio > 1.0 {
                    return Err(CompoundInertia3Error::UnrepresentableCenterOfMass);
                }
                let increment = delta * ratio;
                let next_center = center_of_mass + increment;
                if !vector_is_finite(&increment) || !vector_is_finite(&next_center) {
                    return Err(CompoundInertia3Error::UnrepresentableCenterOfMass);
                }
                center_of_mass = next_center;
            }
            total_mass = next_total;
        }

        let identity = SMatrix::<f64, 3, 3>::identity();
        let mut tensor = SMatrix::<f64, 3, 3>::zeros();

        for (part, child_center) in ordered.iter().zip(child_centers.iter()) {
            let moments = part.properties.principal_inertia.moments();
            let local_tensor = SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from(moments));
            let rotation = part.local_transform.rotation.to_matrix();
            let rotated_tensor = rotation * local_tensor * rotation.transpose();
            if !matrix_is_finite(&rotated_tensor) {
                return Err(CompoundInertia3Error::UnrepresentableTensor);
            }

            let d = child_center - center_of_mass;
            if !vector_is_finite(&d) {
                return Err(CompoundInertia3Error::UnrepresentableCenterOfMass);
            }
            let d2 = d.dot(&d);
            if !d2.is_finite() || d2 < 0.0 {
                return Err(CompoundInertia3Error::UnrepresentableTensor);
            }

            let parallel_axis = (identity * d2 - d * d.transpose()) * part.properties.mass;
            if !matrix_is_finite(&parallel_axis) {
                return Err(CompoundInertia3Error::UnrepresentableTensor);
            }

            let contribution = rotated_tensor + parallel_axis;
            let next_tensor = tensor + contribution;
            if !matrix_is_finite(&contribution) || !matrix_is_finite(&next_tensor) {
                return Err(CompoundInertia3Error::UnrepresentableTensor);
            }
            tensor = next_tensor;
        }

        // Remove only roundoff-scale antisymmetry introduced by matrix products.
        for row in 0..3 {
            for col in (row + 1)..3 {
                let average = 0.5 * (tensor[(row, col)] + tensor[(col, row)]);
                if !average.is_finite() {
                    return Err(CompoundInertia3Error::UnrepresentableTensor);
                }
                tensor[(row, col)] = average;
                tensor[(col, row)] = average;
            }
        }

        validate_positive_definite(&tensor)?;

        Ok(Self {
            total_mass,
            center_of_mass,
            inertia_tensor_body: tensor,
        })
    }

    /// Checked rotational kinetic energy in the compound body frame.
    pub fn rotational_energy_body_checked(
        &self,
        omega_body: &SVector<f64, 3>,
    ) -> Result<f64, CompoundInertia3Error> {
        if !vector_is_finite(omega_body) || !matrix_is_finite(&self.inertia_tensor_body) {
            return Err(CompoundInertia3Error::UnrepresentableTensor);
        }
        let inertia_omega = self.inertia_tensor_body * omega_body;
        if !vector_is_finite(&inertia_omega) {
            return Err(CompoundInertia3Error::UnrepresentableTensor);
        }
        let energy = 0.5 * omega_body.dot(&inertia_omega);
        if !energy.is_finite() || energy < 0.0 {
            return Err(CompoundInertia3Error::UnrepresentableTensor);
        }
        Ok(energy)
    }

    /// Whether the selected compound body frame is already a principal-inertia
    /// frame within an absolute inertia tolerance.
    pub fn body_frame_is_principal_within(
        &self,
        tolerance: f64,
    ) -> Result<bool, CompoundInertia3Error> {
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(CompoundInertia3Error::InvalidTolerance);
        }
        if !matrix_is_finite(&self.inertia_tensor_body) {
            return Err(CompoundInertia3Error::UnrepresentableTensor);
        }
        let max_off_diagonal = [
            self.inertia_tensor_body[(0, 1)].abs(),
            self.inertia_tensor_body[(0, 2)].abs(),
            self.inertia_tensor_body[(1, 2)].abs(),
        ]
        .into_iter()
        .fold(0.0_f64, f64::max);
        Ok(max_off_diagonal <= tolerance)
    }
}

fn validate_part(part: &CompoundMassPart3) -> Result<(), CompoundInertia3Error> {
    if !part.properties.mass.is_finite() || part.properties.mass <= 0.0 {
        return Err(CompoundInertia3Error::InvalidPartMass {
            part_id: part.part_id,
        });
    }
    if !vector_is_finite(&part.properties.center_of_mass) {
        return Err(CompoundInertia3Error::NonFinitePartCenterOfMass {
            part_id: part.part_id,
        });
    }
    if !vector_is_finite(&part.local_transform.translation.0) {
        return Err(CompoundInertia3Error::NonFinitePartTranslation {
            part_id: part.part_id,
        });
    }

    let rotation_matrix = part.local_transform.rotation.to_matrix();
    if !matrix_is_finite(&rotation_matrix)
        || !part
            .local_transform
            .rotation
            .is_proper_rotation(ROTATION_VALIDITY_TOLERANCE)
    {
        return Err(CompoundInertia3Error::InvalidPartRotation {
            part_id: part.part_id,
        });
    }

    if part
        .properties
        .principal_inertia
        .moments()
        .iter()
        .any(|moment| !moment.is_finite() || *moment <= 0.0)
    {
        return Err(CompoundInertia3Error::InvalidPartInertia {
            part_id: part.part_id,
        });
    }

    Ok(())
}

fn child_center_in_compound(
    part: &CompoundMassPart3,
) -> Result<SVector<f64, 3>, CompoundInertia3Error> {
    let rotated_local_com = part
        .local_transform
        .rotation
        .rotate_vector(&part.properties.center_of_mass);
    let center = rotated_local_com + part.local_transform.translation.0;
    if vector_is_finite(&rotated_local_com) && vector_is_finite(&center) {
        Ok(center)
    } else {
        Err(CompoundInertia3Error::UnrepresentableCenterOfMass)
    }
}

fn vector_is_finite(vector: &SVector<f64, 3>) -> bool {
    vector.iter().all(|value| value.is_finite())
}

fn matrix_is_finite(matrix: &SMatrix<f64, 3, 3>) -> bool {
    matrix.iter().all(|value| value.is_finite())
}

/// Sylvester's criterion on a scale-normalized symmetric matrix.
///
/// Scaling by a positive scalar preserves positive-definiteness while avoiding
/// overflow in determinant/minor calculations. Extremely ill-conditioned cases
/// that underflow a required positive minor fail closed.
fn validate_positive_definite(
    tensor: &SMatrix<f64, 3, 3>,
) -> Result<(), CompoundInertia3Error> {
    if !matrix_is_finite(tensor) {
        return Err(CompoundInertia3Error::UnrepresentableTensor);
    }

    let scale = tensor
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if !scale.is_finite() || scale <= 0.0 {
        return Err(CompoundInertia3Error::NonPositiveDefiniteTensor);
    }
    let a = tensor / scale;

    let minor1 = a[(0, 0)];
    let minor2 = a[(0, 0)] * a[(1, 1)] - a[(0, 1)] * a[(1, 0)];
    let determinant = a.determinant();
    if !minor1.is_finite()
        || !minor2.is_finite()
        || !determinant.is_finite()
        || minor1 <= 0.0
        || minor2 <= 0.0
        || determinant <= 0.0
    {
        return Err(CompoundInertia3Error::NonPositiveDefiniteTensor);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::{Bivector, Point, Rotor};

    fn part(
        part_id: u64,
        translation: [f64; 3],
        rotation: Rotor<3>,
        properties: MassProperties3,
    ) -> CompoundMassPart3 {
        CompoundMassPart3 {
            part_id,
            local_transform: Transform {
                translation: Point::new(translation),
                rotation,
            },
            properties,
        }
    }

    #[test]
    fn symmetric_sphere_dumbbell_matches_parallel_axis_theorem() {
        let sphere = MassProperties3::solid_sphere(1.0, 1.0).unwrap();
        let parts = vec![
            part(10, [-2.0, 0.0, 0.0], Rotor::identity(), sphere),
            part(20, [2.0, 0.0, 0.0], Rotor::identity(), sphere),
        ];
        let compound = CompoundMassProperties3::compose(&parts).unwrap();

        assert_eq!(compound.total_mass, 2.0);
        assert_eq!(compound.center_of_mass, SVector::zeros());
        assert!((compound.inertia_tensor_body[(0, 0)] - 0.8).abs() < 1.0e-12);
        assert!((compound.inertia_tensor_body[(1, 1)] - 8.8).abs() < 1.0e-12);
        assert!((compound.inertia_tensor_body[(2, 2)] - 8.8).abs() < 1.0e-12);
        assert!(compound.body_frame_is_principal_within(1.0e-12).unwrap());
    }

    #[test]
    fn part_order_does_not_define_evidence() {
        let sphere = MassProperties3::solid_sphere(1.0, 1.0).unwrap();
        let a = part(1, [-2.0, 0.0, 0.0], Rotor::identity(), sphere);
        let b = part(2, [2.0, 0.0, 0.0], Rotor::identity(), sphere);
        let forward = CompoundMassProperties3::compose(&[a.clone(), b.clone()]).unwrap();
        let reverse = CompoundMassProperties3::compose(&[b, a]).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn rotated_anisotropic_child_produces_products_of_inertia() {
        let cuboid = MassProperties3::solid_cuboid(12.0, [1.0, 2.0, 3.0]).unwrap();
        let rotation = Rotor::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 1),
            std::f64::consts::FRAC_PI_4,
        );
        let compound = CompoundMassProperties3::compose(&[part(
            1,
            [0.0, 0.0, 0.0],
            rotation,
            cuboid,
        )])
        .unwrap();

        assert!((compound.inertia_tensor_body[(0, 0)] - 46.0).abs() < 1.0e-10);
        assert!((compound.inertia_tensor_body[(1, 1)] - 46.0).abs() < 1.0e-10);
        assert!((compound.inertia_tensor_body[(2, 2)] - 20.0).abs() < 1.0e-10);
        assert!((compound.inertia_tensor_body[(0, 1)].abs() - 6.0).abs() < 1.0e-10);
        assert!(!compound.body_frame_is_principal_within(1.0e-12).unwrap());
    }

    #[test]
    fn duplicate_part_identity_is_rejected() {
        let sphere = MassProperties3::solid_sphere(1.0, 1.0).unwrap();
        let parts = vec![
            part(7, [-1.0, 0.0, 0.0], Rotor::identity(), sphere),
            part(7, [1.0, 0.0, 0.0], Rotor::identity(), sphere),
        ];
        assert!(matches!(
            CompoundMassProperties3::compose(&parts),
            Err(CompoundInertia3Error::DuplicatePartId(7))
        ));
    }

    #[test]
    fn improper_child_rotation_is_rejected() {
        let sphere = MassProperties3::solid_sphere(1.0, 1.0).unwrap();
        let mut reflection = SMatrix::<f64, 3, 3>::identity();
        reflection[(0, 0)] = -1.0;
        let bad_rotation = Rotor::from_matrix(reflection);
        let parts = [part(3, [0.0, 0.0, 0.0], bad_rotation, sphere)];
        assert!(matches!(
            CompoundMassProperties3::compose(&parts),
            Err(CompoundInertia3Error::InvalidPartRotation { part_id: 3 })
        ));
    }

    #[test]
    fn corrupted_public_child_state_is_revalidated() {
        let mut properties = MassProperties3::solid_sphere(1.0, 1.0).unwrap();
        properties.mass = f64::NAN;
        let parts = [part(4, [0.0, 0.0, 0.0], Rotor::identity(), properties)];
        assert!(matches!(
            CompoundMassProperties3::compose(&parts),
            Err(CompoundInertia3Error::InvalidPartMass { part_id: 4 })
        ));
    }

    #[test]
    fn finite_total_mass_overflow_is_rejected() {
        let mut a_properties = MassProperties3::solid_sphere(1.0, 1.0).unwrap();
        let mut b_properties = a_properties;
        a_properties.mass = f64::MAX;
        b_properties.mass = f64::MAX;
        let parts = [
            part(1, [0.0, 0.0, 0.0], Rotor::identity(), a_properties),
            part(2, [0.0, 0.0, 0.0], Rotor::identity(), b_properties),
        ];
        assert!(matches!(
            CompoundMassProperties3::compose(&parts),
            Err(CompoundInertia3Error::UnrepresentableTotalMass)
        ));
    }

    #[test]
    fn opposite_extreme_centers_fail_before_partial_commit() {
        let sphere = MassProperties3::solid_sphere(1.0, 1.0).unwrap();
        let parts = [
            part(1, [f64::MAX, 0.0, 0.0], Rotor::identity(), sphere),
            part(2, [-f64::MAX, 0.0, 0.0], Rotor::identity(), sphere),
        ];
        assert!(matches!(
            CompoundMassProperties3::compose(&parts),
            Err(CompoundInertia3Error::UnrepresentableCenterOfMass)
        ));
    }

    #[test]
    fn full_tensor_energy_includes_cross_terms() {
        let cuboid = MassProperties3::solid_cuboid(12.0, [1.0, 2.0, 3.0]).unwrap();
        let rotation = Rotor::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 1),
            std::f64::consts::FRAC_PI_4,
        );
        let compound = CompoundMassProperties3::compose(&[part(
            1,
            [0.0, 0.0, 0.0],
            rotation,
            cuboid,
        )])
        .unwrap();
        let omega = SVector::from([1.0, 1.0, 0.0]);
        let energy = compound.rotational_energy_body_checked(&omega).unwrap();
        let diagonal_only = 0.5
            * (compound.inertia_tensor_body[(0, 0)] + compound.inertia_tensor_body[(1, 1)]);
        assert!((energy - diagonal_only).abs() > 1.0);
    }
}
