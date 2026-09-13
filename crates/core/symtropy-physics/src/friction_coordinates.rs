// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Solver-local identity for one friction impulse.
//!
//! These coordinates deliberately exclude fixed-tick identity. The contact solver
//! may identify where an impulse occurred *within* one physics step, but only the
//! enclosing fixed-tick authority may bind those coordinates to a global
//! [`FrictionTransactionId`]. This prevents a solver or callback from relabeling
//! mechanical evidence across thermodynamic ticks.

use serde::{Deserialize, Serialize};

use crate::friction_evidence::FrictionTransactionId;

/// Deterministic coordinates of one friction impulse within a single fixed tick.
///
/// Ordering follows the production solver's nested traversal:
/// solver iteration -> active contact sequence -> contact-point sequence.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct FrictionSolverCoordinates {
    pub solver_iteration: u32,
    pub contact_sequence: u32,
    pub point_sequence: u32,
}

impl FrictionSolverCoordinates {
    pub const fn new(
        solver_iteration: u32,
        contact_sequence: u32,
        point_sequence: u32,
    ) -> Self {
        Self {
            solver_iteration,
            contact_sequence,
            point_sequence,
        }
    }

    /// Bind solver-local coordinates to a fixed tick.
    ///
    /// This primitive is intentionally pure; authority is established by *who owns
    /// the fixed tick*. Production callers should obtain `fixed_tick` only from the
    /// enclosing transaction runtime rather than user/gameplay input.
    pub const fn bind_fixed_tick(self, fixed_tick: u64) -> FrictionTransactionId {
        FrictionTransactionId::new(
            fixed_tick,
            self.solver_iteration,
            self.contact_sequence,
            self.point_sequence,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solver_coordinates_bind_all_fields_without_owning_tick_identity() {
        let coordinates = FrictionSolverCoordinates::new(3, 7, 2);
        let id = coordinates.bind_fixed_tick(41);

        assert_eq!(id.fixed_tick, 41);
        assert_eq!(id.solver_iteration, 3);
        assert_eq!(id.contact_sequence, 7);
        assert_eq!(id.point_sequence, 2);
    }

    #[test]
    fn canonical_order_matches_nested_solver_traversal() {
        let mut coordinates = vec![
            FrictionSolverCoordinates::new(1, 0, 0),
            FrictionSolverCoordinates::new(0, 2, 0),
            FrictionSolverCoordinates::new(0, 1, 3),
            FrictionSolverCoordinates::new(0, 1, 1),
        ];
        coordinates.sort_unstable();

        assert_eq!(
            coordinates,
            vec![
                FrictionSolverCoordinates::new(0, 1, 1),
                FrictionSolverCoordinates::new(0, 1, 3),
                FrictionSolverCoordinates::new(0, 2, 0),
                FrictionSolverCoordinates::new(1, 0, 0),
            ]
        );
    }
}
