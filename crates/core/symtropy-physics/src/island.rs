// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Island detection: groups connected bodies into independent clusters.
//!
//! Bodies connected by contacts or constraints form an "island." Each island
//! can be solved independently. If ALL bodies in an island are sleeping,
//! the entire island is skipped — no broadphase, narrowphase, or solver cost.
//!
//! Uses Union-Find (disjoint set) for O(n * alpha(n)) grouping, effectively linear.

use crate::body::{BodyHandle, RigidBody};
use crate::constraint::Constraint;
use crate::contact::ContactManifold;
use std::collections::HashMap;

/// A group of connected bodies that can be solved independently.
#[derive(Debug)]
pub struct Island {
    /// Indices into the world's body array.
    pub body_indices: Vec<usize>,
    /// Indices into the world's contact array.
    pub contact_indices: Vec<usize>,
    /// Indices into the world's constraint array.
    pub constraint_indices: Vec<usize>,
    /// Whether all bodies in this island are sleeping.
    pub sleeping: bool,
}

/// Union-Find (disjoint set) data structure for island detection.
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]]; // path compression
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }
        // Union by rank
        if self.rank[ra] < self.rank[rb] {
            self.parent[ra] = rb;
        } else if self.rank[ra] > self.rank[rb] {
            self.parent[rb] = ra;
        } else {
            self.parent[rb] = ra;
            self.rank[ra] += 1;
        }
    }
}

/// Build islands from the current contact and constraint graph.
///
/// Returns a list of islands, each containing indices into the world's
/// body, contact, and constraint arrays. Islands where all bodies are
/// sleeping are marked as `sleeping = true`.
pub fn build_islands<const D: usize>(
    bodies: &[RigidBody<D>],
    contacts: &[ContactManifold<D>],
    constraints: &[Box<dyn Constraint<D>>],
    handle_to_index: &HashMap<BodyHandle, usize>,
) -> Vec<Island> {
    let n = bodies.len();
    if n == 0 {
        return Vec::new();
    }

    let mut uf = UnionFind::new(n);

    // Union bodies connected by contacts
    for contact in contacts {
        if let (Some(&ia), Some(&ib)) = (
            handle_to_index.get(&contact.body_a),
            handle_to_index.get(&contact.body_b),
        ) {
            uf.union(ia, ib);
        }
    }

    // Union bodies connected by constraints
    for constraint in constraints {
        let (ha, hb) = constraint.bodies();
        if let (Some(&ia), Some(&ib)) = (handle_to_index.get(&ha), handle_to_index.get(&hb)) {
            uf.union(ia, ib);
        }
    }

    // Group body indices by root. Body iteration is ascending, so every
    // membership vector is canonical without a separate per-island sort.
    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> =
        std::collections::BTreeMap::new();
    let mut body_roots = Vec::with_capacity(n);
    for i in 0..n {
        let root = uf.find(i);
        body_roots.push(root);
        groups.entry(root).or_default().push(i);
    }

    // Build each island once, then retain a root -> output-index map so every
    // contact and constraint is assigned in a single pass rather than
    // rescanning the full edge arrays once per island.
    let mut islands = Vec::with_capacity(groups.len());
    let mut root_to_island = HashMap::with_capacity(groups.len());
    for (root, body_indices) in groups {
        let sleeping = body_indices.iter().all(|&index| bodies[index].sleeping);
        let island_index = islands.len();
        root_to_island.insert(root, island_index);
        islands.push(Island {
            body_indices,
            contact_indices: Vec::new(),
            constraint_indices: Vec::new(),
            sleeping,
        });
    }

    // A valid contact whose two endpoints are present must have endpoints in
    // the same union-find component. If only one endpoint is present, preserve
    // the prior behavior and associate it with that known endpoint's island.
    for (contact_index, contact) in contacts.iter().enumerate() {
        let body_a = handle_to_index.get(&contact.body_a).copied();
        let body_b = handle_to_index.get(&contact.body_b).copied();
        let root = body_a
            .or(body_b)
            .map(|body_index| body_roots[body_index]);
        if let Some(root) = root {
            if let Some(&island_index) = root_to_island.get(&root) {
                islands[island_index].contact_indices.push(contact_index);
            }
        }
    }

    // Constraints follow the same single-owner rule as contact manifolds.
    for (constraint_index, constraint) in constraints.iter().enumerate() {
        let (handle_a, handle_b) = constraint.bodies();
        let body_a = handle_to_index.get(&handle_a).copied();
        let body_b = handle_to_index.get(&handle_b).copied();
        let root = body_a
            .or(body_b)
            .map(|body_index| body_roots[body_index]);
        if let Some(root) = root {
            if let Some(&island_index) = root_to_island.get(&root) {
                islands[island_index]
                    .constraint_indices
                    .push(constraint_index);
            }
        }
    }

    // Hash-derived roots are not stable semantic identities. Order by the
    // lowest stable BodyHandle in each component for deterministic scheduling.
    islands.sort_by_key(|island| {
        island
            .body_indices
            .iter()
            .map(|&index| bodies[index].handle)
            .min()
            .unwrap_or(BodyHandle(usize::MAX))
    });

    islands
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;
    use symtropy_math::Point;

    fn make_bodies(n: usize) -> (Vec<RigidBody<3>>, HashMap<BodyHandle, usize>) {
        let mut bodies = Vec::new();
        let mut map = HashMap::new();
        for i in 0..n {
            let mut b = RigidBody::<3>::dynamic_sphere(
                BodyHandle(i),
                Point::new([i as f64 * 10.0, 0.0, 0.0]),
                0.5,
                1.0,
            );
            // Make every other body sleeping for testing
            if i >= n / 2 {
                b.sleeping = true;
            }
            map.insert(BodyHandle(i), i);
            bodies.push(b);
        }
        (bodies, map)
    }

    #[test]
    fn no_contacts_each_body_is_own_island() {
        let (bodies, map) = make_bodies(4);
        let islands = build_islands(&bodies, &[], &[], &map);
        assert_eq!(islands.len(), 4, "4 disconnected bodies = 4 islands");
    }

    #[test]
    fn connected_bodies_form_one_island() {
        let (bodies, map) = make_bodies(4);
        let contacts = vec![
            ContactManifold::single(
                BodyHandle(0),
                BodyHandle(1),
                nalgebra::SVector::from([1.0, 0.0, 0.0]),
                nalgebra::SVector::zeros(),
                0.1,
            ),
            ContactManifold::single(
                BodyHandle(1),
                BodyHandle(2),
                nalgebra::SVector::from([1.0, 0.0, 0.0]),
                nalgebra::SVector::zeros(),
                0.1,
            ),
        ];
        let islands = build_islands(&bodies, &contacts, &[], &map);

        // Bodies 0,1,2 connected; body 3 separate
        assert_eq!(islands.len(), 2);
        let big = islands.iter().max_by_key(|i| i.body_indices.len()).unwrap();
        assert_eq!(big.body_indices.len(), 3);
    }

    #[test]
    fn sleeping_island_detected() {
        let (mut bodies, map) = make_bodies(4);
        // Make bodies 2,3 sleeping
        bodies[2].sleeping = true;
        bodies[3].sleeping = true;
        bodies[0].sleeping = false;
        bodies[1].sleeping = false;

        let contacts = vec![
            ContactManifold::single(
                BodyHandle(0),
                BodyHandle(1),
                nalgebra::SVector::from([1.0, 0.0, 0.0]),
                nalgebra::SVector::zeros(),
                0.1,
            ),
            ContactManifold::single(
                BodyHandle(2),
                BodyHandle(3),
                nalgebra::SVector::from([1.0, 0.0, 0.0]),
                nalgebra::SVector::zeros(),
                0.1,
            ),
        ];
        let islands = build_islands(&bodies, &contacts, &[], &map);
        assert_eq!(islands.len(), 2);

        let awake_count = islands.iter().filter(|i| !i.sleeping).count();
        let sleeping_count = islands.iter().filter(|i| i.sleeping).count();
        assert_eq!(awake_count, 1, "one awake island");
        assert_eq!(sleeping_count, 1, "one sleeping island");
    }

    #[test]
    fn constraint_connects_bodies() {
        use crate::constraint::DistanceConstraint;

        let (bodies, map) = make_bodies(3);
        let constraints: Vec<Box<dyn Constraint<3>>> = vec![Box::new(DistanceConstraint {
            body_a: BodyHandle(0),
            body_b: BodyHandle(2),
            rest_length: 5.0,
            stiffness: 1.0,
        })];
        let islands = build_islands(&bodies, &[], &constraints, &map);

        // Bodies 0,2 connected by constraint; body 1 separate
        assert_eq!(islands.len(), 2);
        let connected = islands.iter().find(|i| i.body_indices.len() == 2).unwrap();
        assert_eq!(connected.constraint_indices.len(), 1);
    }

    #[test]
    fn islands_are_canonically_ordered_by_lowest_body_index() {
        let (bodies, map) = make_bodies(8);
        let contacts = vec![
            ContactManifold::single(
                BodyHandle(6),
                BodyHandle(7),
                nalgebra::SVector::from([1.0, 0.0, 0.0]),
                nalgebra::SVector::zeros(),
                0.1,
            ),
            ContactManifold::single(
                BodyHandle(2),
                BodyHandle(3),
                nalgebra::SVector::from([1.0, 0.0, 0.0]),
                nalgebra::SVector::zeros(),
                0.1,
            ),
            ContactManifold::single(
                BodyHandle(0),
                BodyHandle(1),
                nalgebra::SVector::from([1.0, 0.0, 0.0]),
                nalgebra::SVector::zeros(),
                0.1,
            ),
        ];

        for _ in 0..16 {
            let islands = build_islands(&bodies, &contacts, &[], &map);
            let first_indices: Vec<_> = islands
                .iter()
                .map(|island| island.body_indices[0])
                .collect();
            assert_eq!(first_indices, vec![0, 2, 4, 5, 6]);
            assert!(
                islands
                    .windows(2)
                    .all(|pair| { pair[0].body_indices[0] < pair[1].body_indices[0] })
            );
        }
    }

    #[test]
    fn contacts_and_constraints_are_assigned_exactly_once() {
        use crate::constraint::DistanceConstraint;

        let (bodies, map) = make_bodies(6);
        let contacts = vec![
            ContactManifold::single(
                BodyHandle(1),
                BodyHandle(2),
                nalgebra::SVector::from([1.0, 0.0, 0.0]),
                nalgebra::SVector::zeros(),
                0.1,
            ),
            ContactManifold::single(
                BodyHandle(2),
                BodyHandle(3),
                nalgebra::SVector::from([1.0, 0.0, 0.0]),
                nalgebra::SVector::zeros(),
                0.1,
            ),
            // Preserve the known endpoint in an island when the other handle
            // is absent from the supplied body map.
            ContactManifold::single(
                BodyHandle(4),
                BodyHandle(999),
                nalgebra::SVector::from([1.0, 0.0, 0.0]),
                nalgebra::SVector::zeros(),
                0.1,
            ),
            // Both endpoints absent: this edge belongs to no island.
            ContactManifold::single(
                BodyHandle(998),
                BodyHandle(999),
                nalgebra::SVector::from([1.0, 0.0, 0.0]),
                nalgebra::SVector::zeros(),
                0.1,
            ),
        ];
        let constraints: Vec<Box<dyn Constraint<3>>> = vec![
            Box::new(DistanceConstraint {
                body_a: BodyHandle(0),
                body_b: BodyHandle(4),
                rest_length: 1.0,
                stiffness: 1.0,
            }),
            Box::new(DistanceConstraint {
                body_a: BodyHandle(5),
                body_b: BodyHandle(997),
                rest_length: 1.0,
                stiffness: 1.0,
            }),
            Box::new(DistanceConstraint {
                body_a: BodyHandle(996),
                body_b: BodyHandle(997),
                rest_length: 1.0,
                stiffness: 1.0,
            }),
        ];

        let islands = build_islands(&bodies, &contacts, &constraints, &map);
        let mut contact_indices: Vec<_> = islands
            .iter()
            .flat_map(|island| island.contact_indices.iter().copied())
            .collect();
        let mut constraint_indices: Vec<_> = islands
            .iter()
            .flat_map(|island| island.constraint_indices.iter().copied())
            .collect();
        contact_indices.sort_unstable();
        constraint_indices.sort_unstable();

        assert_eq!(contact_indices, vec![0, 1, 2]);
        assert_eq!(constraint_indices, vec![0, 1]);
    }

    #[test]
    fn empty_world() {
        let islands = build_islands::<3>(&[], &[], &[], &HashMap::new());
        assert!(islands.is_empty());
    }
}
