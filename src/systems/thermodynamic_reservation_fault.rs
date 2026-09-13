// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Read-only operator fault view for unresolved friction reservations.
//!
//! This module deliberately contains no recovery authority. It turns the private
//! runtime reservation census into a typed diagnostic payload that schedulers,
//! HUDs, logs, and future checkpoint tooling can inspect without receiving any
//! capability to clear, cancel, forge, or relabel a reservation.

use symtropy_physics::FrictionTransactionId;

use super::thermodynamic_runtime::ThermodynamicTransactionRuntime;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingFrictionReservationFault {
    pub open_tick_id: Option<u64>,
    pub reservation_ids: Vec<FrictionTransactionId>,
}

impl PendingFrictionReservationFault {
    pub fn count(&self) -> usize {
        self.reservation_ids.len()
    }
}

/// Snapshot the unresolved-reservation fault, if one exists.
///
/// IDs are already canonically ordered by the runtime's private `BTreeSet`.
/// Returning `None` means only that no runtime reservation is currently in
/// flight; it does not make any statement about core `Applied` friction entries,
/// which remain governed by the separate friction journal lifecycle.
pub fn pending_friction_reservation_fault(
    runtime: &ThermodynamicTransactionRuntime,
) -> Option<PendingFrictionReservationFault> {
    let reservation_ids = runtime.pending_friction_reservation_ids();
    if reservation_ids.is_empty() {
        return None;
    }

    Some(PendingFrictionReservationFault {
        open_tick_id: runtime.open_tick_id(),
        reservation_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::SVector;
    use symtropy_physics::{BodyHandle, FrictionSolverCoordinates, FrictionTransactionId};

    #[test]
    fn operator_fault_is_typed_sorted_and_non_authoritative() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        assert_eq!(pending_friction_reservation_fault(&runtime), None);
        runtime.begin_next_tick().unwrap();

        let later = runtime
            .reserve_friction_impulse_at(
                BodyHandle(1),
                BodyHandle(2),
                &SVector::<f64, 2>::zeros(),
                &SVector::<f64, 2>::from([0.1, 0.0]),
                FrictionSolverCoordinates::new(4, 2, 9),
            )
            .unwrap();
        let earlier = runtime
            .reserve_friction_impulse_at(
                BodyHandle(3),
                BodyHandle(4),
                &SVector::<f64, 2>::zeros(),
                &SVector::<f64, 2>::from([0.2, 0.0]),
                FrictionSolverCoordinates::new(1, 7, 3),
            )
            .unwrap();

        let fault = pending_friction_reservation_fault(&runtime).unwrap();
        assert_eq!(fault.open_tick_id, Some(0));
        assert_eq!(fault.count(), 2);
        assert_eq!(
            fault.reservation_ids,
            vec![
                FrictionTransactionId::new(0, 1, 7, 3),
                FrictionTransactionId::new(0, 4, 2, 9),
            ]
        );

        // Dropping or mutating the diagnostic payload cannot mutate runtime
        // authority; only the original reservation tokens can resolve it.
        let mut local_copy = fault.reservation_ids;
        local_copy.clear();
        assert_eq!(runtime.pending_friction_reservation_count(), 2);

        runtime.cancel_friction_reservation(earlier).unwrap();
        assert_eq!(pending_friction_reservation_fault(&runtime).unwrap().count(), 1);
        runtime.cancel_friction_reservation(later).unwrap();
        assert_eq!(pending_friction_reservation_fault(&runtime), None);
    }
}
