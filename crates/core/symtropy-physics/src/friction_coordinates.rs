// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Solver-local identity for one friction impulse.
//!
//! These coordinates deliberately exclude fixed-tick identity. The contact solver
//! may identify where an impulse occurred *within* one physics step, while the
//! enclosing fixed-tick authority remains solely responsible for constructing the
//! global friction transaction identity.

use serde::{Deserialize, Serialize};

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_contain_only_solver_local_identity() {
        let coordinates = FrictionSolverCoordinates::new(3, 7, 2);
        assert_eq!(coordinates.solver_iteration, 3);
        assert_eq!(coordinates.contact_sequence, 7);
        assert_eq!(coordinates.point_sequence, 2);
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
