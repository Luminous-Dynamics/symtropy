// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked 3D kinetic-energy interpretation for live [`RigidBody`] state.
//!
//! This is the #54 reference/evidence implementation relocated out of `body.rs`
//! so the newer thermodynamic `RigidBody` representation from #850 remains
//! untouched. The inherent methods and public error surface are unchanged.

use crate::body::RigidBody;

/// Failures produced while interpreting live `RigidBody<3>` state through the
/// checked principal-inertia reference model.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RigidBodyEnergyError {
    InvalidMass,
    NonFiniteLinearVelocity,
    UnrepresentableLinearEnergy,
    Angular(crate::angular_dynamics::AngularDynamicsError),
    UnrepresentableTotalEnergy,
}

impl From<crate::angular_dynamics::AngularDynamicsError> for RigidBodyEnergyError {
    fn from(value: crate::angular_dynamics::AngularDynamicsError) -> Self {
        Self::Angular(value)
    }
}

impl RigidBody<3> {
    /// Interpret the body's three stored inertia components as body-space
    /// principal moments under the checked #35 asymmetric-top convention.
    pub fn principal_inertia_3_checked(
        &self,
    ) -> Result<crate::angular_dynamics::PrincipalInertia3, RigidBodyEnergyError> {
        Ok(crate::angular_dynamics::PrincipalInertia3::new([
            self.inertia[0],
            self.inertia[1],
            self.inertia[2],
        ])?)
    }

    /// Exact checked 3D kinetic energy for the body's current represented state.
    ///
    /// This is a validation/reference path, not yet the generic production hot
    /// path. Dynamic bodies use exact linear kinetic energy plus the #35
    /// anisotropic body-space principal-inertia rotational energy. Static and
    /// kinematic bodies return zero, matching the existing compatibility API.
    pub fn kinetic_energy_3d_checked(&self) -> Result<f64, RigidBodyEnergyError> {
        if !self.is_dynamic() {
            return Ok(0.0);
        }
        if !self.mass.is_finite() || self.mass <= 0.0 {
            return Err(RigidBodyEnergyError::InvalidMass);
        }
        if !self.linear_velocity.iter().all(|value| value.is_finite()) {
            return Err(RigidBodyEnergyError::NonFiniteLinearVelocity);
        }

        let speed_squared = self.linear_velocity.norm_squared();
        let linear = 0.5 * self.mass * speed_squared;
        if !speed_squared.is_finite() || !linear.is_finite() || linear < 0.0 {
            return Err(RigidBodyEnergyError::UnrepresentableLinearEnergy);
        }

        let inertia = self.principal_inertia_3_checked()?;
        let angular = crate::angular_dynamics::rotational_kinetic_energy(
            &self.transform.rotation,
            &self.angular_velocity,
            inertia,
        )?;
        let total = linear + angular;
        if !total.is_finite() || total < 0.0 {
            return Err(RigidBodyEnergyError::UnrepresentableTotalEnergy);
        }
        Ok(total)
    }
}
