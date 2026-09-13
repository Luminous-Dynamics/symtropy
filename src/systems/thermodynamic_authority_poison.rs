// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! First-fault latch for fail-stop physics-authority scheduler evidence.
//!
//! A poisoned fixed tick must preserve the first causal authority fault that made
//! normal continuation inadmissible. Re-entering an error path after poison must
//! not overwrite that evidence or manufacture a second independent incident.

use super::thermodynamic_authority_fault::ThermodynamicPhysicsAuthorityFault;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ThermodynamicPhysicsAuthorityPoisonRecord {
    pub tick_id: Option<u64>,
    pub fault: ThermodynamicPhysicsAuthorityFault,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PhysicsAuthorityPoisonAdmission {
    /// This call established the fail-stop authority record.
    First(ThermodynamicPhysicsAuthorityPoisonRecord),
    /// The latch was already poisoned. The original record remains authoritative
    /// and this later observation is not admitted as a replacement incident.
    AlreadyPoisoned(ThermodynamicPhysicsAuthorityPoisonRecord),
}

/// Sticky first-fault authority latch.
///
/// There is intentionally no clear/reset method. Recovery from a poisoned fixed
/// tick requires an explicit future recovery protocol rather than an ordinary
/// scheduler caller mutating the evidence back to healthy.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ThermodynamicPhysicsAuthorityPoisonLatch {
    first: Option<ThermodynamicPhysicsAuthorityPoisonRecord>,
}

impl ThermodynamicPhysicsAuthorityPoisonLatch {
    pub const fn new() -> Self {
        Self { first: None }
    }

    pub const fn is_poisoned(&self) -> bool {
        self.first.is_some()
    }

    pub const fn first_fault(&self) -> Option<ThermodynamicPhysicsAuthorityPoisonRecord> {
        self.first
    }

    /// Admit the first fail-stop authority fault exactly once.
    ///
    /// Later calls are observational only: they return the already-authoritative
    /// first record without changing the latch.
    pub fn poison_once(
        &mut self,
        tick_id: Option<u64>,
        fault: ThermodynamicPhysicsAuthorityFault,
    ) -> PhysicsAuthorityPoisonAdmission {
        if let Some(first) = self.first {
            return PhysicsAuthorityPoisonAdmission::AlreadyPoisoned(first);
        }

        let record = ThermodynamicPhysicsAuthorityPoisonRecord { tick_id, fault };
        self.first = Some(record);
        PhysicsAuthorityPoisonAdmission::First(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_physics::{
        BodyHandle, FrictionPromotionError, FrictionSolverCoordinateComponent,
        FrictionSolverCoordinates,
    };

    use super::super::thermodynamic_authority_fault::{
        RuntimeFrictionFaultSummary, ThermodynamicFrictionAuthorityFaultSummary,
    };

    fn coordinate_fault(value: usize) -> ThermodynamicPhysicsAuthorityFault {
        ThermodynamicPhysicsAuthorityFault::CoordinateOverflow {
            component: FrictionSolverCoordinateComponent::ContactSequence,
            value,
        }
    }

    fn promotion_fault() -> ThermodynamicPhysicsAuthorityFault {
        ThermodynamicPhysicsAuthorityFault::FrictionAuthority {
            body_a: BodyHandle(7),
            body_b: BodyHandle(11),
            coordinates: FrictionSolverCoordinates::new(3, 5, 1),
            reason: ThermodynamicFrictionAuthorityFaultSummary::Runtime(
                RuntimeFrictionFaultSummary::Promotion(
                    FrictionPromotionError::MissingThermalState,
                ),
            ),
        }
    }

    #[test]
    fn first_fault_is_latched_exactly() {
        let mut latch = ThermodynamicPhysicsAuthorityPoisonLatch::new();
        let fault = coordinate_fault(99);
        let record = ThermodynamicPhysicsAuthorityPoisonRecord {
            tick_id: Some(23),
            fault,
        };

        assert_eq!(
            latch.poison_once(Some(23), fault),
            PhysicsAuthorityPoisonAdmission::First(record)
        );
        assert!(latch.is_poisoned());
        assert_eq!(latch.first_fault(), Some(record));
    }

    #[test]
    fn repeated_same_fault_does_not_create_a_second_incident() {
        let mut latch = ThermodynamicPhysicsAuthorityPoisonLatch::new();
        let fault = coordinate_fault(101);
        let original = ThermodynamicPhysicsAuthorityPoisonRecord {
            tick_id: Some(4),
            fault,
        };
        assert_eq!(
            latch.poison_once(Some(4), fault),
            PhysicsAuthorityPoisonAdmission::First(original)
        );

        assert_eq!(
            latch.poison_once(Some(4), fault),
            PhysicsAuthorityPoisonAdmission::AlreadyPoisoned(original)
        );
        assert_eq!(latch.first_fault(), Some(original));
    }

    #[test]
    fn later_different_fault_cannot_overwrite_first_causal_evidence() {
        let mut latch = ThermodynamicPhysicsAuthorityPoisonLatch::new();
        let first_fault = coordinate_fault(123);
        let first = ThermodynamicPhysicsAuthorityPoisonRecord {
            tick_id: Some(8),
            fault: first_fault,
        };
        latch.poison_once(Some(8), first_fault);

        assert_eq!(
            latch.poison_once(Some(9), promotion_fault()),
            PhysicsAuthorityPoisonAdmission::AlreadyPoisoned(first)
        );
        assert_eq!(latch.first_fault(), Some(first));
    }

    #[test]
    fn default_is_healthy_and_has_no_clear_authority() {
        let latch = ThermodynamicPhysicsAuthorityPoisonLatch::default();
        assert!(!latch.is_poisoned());
        assert_eq!(latch.first_fault(), None);
    }
}
