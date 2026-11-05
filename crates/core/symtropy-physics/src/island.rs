// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Island detection: groups connected bodies into independent clusters.
//!
//! Dynamic bodies connected by dynamic-dynamic contacts or constraints form
//! a solver island. Static and kinematic bodies are fixed endpoints: they do
//! not merge otherwise independent dynamic islands. Kinematics remain wake
//! sources, so an awake kinematic contact can reactivate a sleeping island.
//! If all dynamic bodies sleep and no awake kinematic touches the island, the
//! entire island is skipped by the solver.
//!
//! Uses Union-Find (disjoint set) for O(n * alpha(n)) grouping, effectively linear.

use crate::body::{BodyHandle, BodyType, RigidBody};
use crate::constraint::Constraint;
use crate::contact::ContactManifold;
use std::collections::{BTreeMap, HashMap};

/// A group of connected bodies that can be solved independently.
#[derive(Debug)]
pub struct Island {
    /// Indices of dynamic bodies owned by this solver island. Static and
    /// kinematic bodies are read-only endpoints and are not island owners.
    pub body_indices: Vec<usize>,
    /// Indices into the world's contact array.
    pub contact_indices: Vec<usize>,
    /// Indices into the world's constraint array.
    pub constraint_indices: Vec<usize>,
    /// Whether all dynamic bodies sleep and no awake kinematic wake source
    /// touches this island.
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
        ) && bodies[ia].is_dynamic()
            && bodies[ib].is_dynamic()
        {
            // Fixed bodies are read-only solver endpoints. Sharing a floor or
            // kinematic platform must not merge otherwise independent dynamic
            // solver islands.
            uf.union(ia, ib);
        }
    }

    // Union bodies connected by constraints
    for constraint in constraints {
        let (ha, hb) = constraint.bodies();
        if let (Some(&ia), Some(&ib)) = (handle_to_index.get(&ha), handle_to_index.get(&hb))
            && bodies[ia].is_dynamic()
            && bodies[ib].is_dynamic()
        {
            uf.union(ia, ib);
        }
    }

    // Only dynamic bodies own solver islands. Static and kinematic bodies
    // remain read-only endpoints: contacts/constraints to them are assigned
    // to the dynamic island, but they cannot connect distinct dynamic islands.
    let mut island_map: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        if !bodies[i].is_dynamic() {
            continue;
        }
        let root = uf.find(i);
        island_map.entry(root).or_default().push(i);
    }

    // Build islands with contact and constraint indices
    let mut islands = Vec::new();
    for (_, mut body_indices) in island_map {
        body_indices.sort_by_key(|&index| bodies[index].handle);

        // Collect contacts that belong to this island
        let body_set: std::collections::HashSet<usize> = body_indices.iter().copied().collect();
        let contact_indices: Vec<usize> = contacts
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                let ia = handle_to_index.get(&c.body_a).copied();
                let ib = handle_to_index.get(&c.body_b).copied();
                ia.is_some_and(|i| body_set.contains(&i))
                    || ib.is_some_and(|i| body_set.contains(&i))
            })
            .map(|(i, _)| i)
            .collect();

        // Collect constraints that belong to this island
        let constraint_indices: Vec<usize> = constraints
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                let (ha, hb) = c.bodies();
                let ia = handle_to_index.get(&ha).copied();
                let ib = handle_to_index.get(&hb).copied();
                ia.is_some_and(|i| body_set.contains(&i))
                    || ib.is_some_and(|i| body_set.contains(&i))
            })
            .map(|(i, _)| i)
            .collect();

        // Static endpoints do not wake dynamic bodies. An awake kinematic
        // endpoint does: it is externally prescribed but can collide with or
        // constrain a sleeping dynamic body. Keep this wake dependency separate
        // from solver-island connectivity so one platform does not merge stacks.
        let has_awake_kinematic_wake_source =
            contact_indices.iter().any(|&contact_index| {
                let contact = &contacts[contact_index];
                [contact.body_a, contact.body_b].into_iter().any(|handle| {
                    handle_to_index.get(&handle).is_some_and(|&index| {
                        bodies[index].body_type == BodyType::Kinematic && !bodies[index].sleeping
                    })
                })
            }) || constraint_indices.iter().any(|&constraint_index| {
                let (body_a, body_b) = constraints[constraint_index].bodies();
                [body_a, body_b].into_iter().any(|handle| {
                    handle_to_index.get(&handle).is_some_and(|&index| {
                        bodies[index].body_type == BodyType::Kinematic && !bodies[index].sleeping
                    })
                })
            });

        let sleeping =
            body_indices.iter().all(|&i| bodies[i].sleeping) && !has_awake_kinematic_wake_source;

        islands.push(Island {
            body_indices,
            contact_indices,
            constraint_indices,
            sleeping,
        });
    }

    // Hash-map roots and insertion history are not part of simulation
    // semantics. Canonicalize the emitted island list by its smallest stable
    // dynamic BodyHandle before callers flatten work or serialize evidence.
    islands.sort_by_key(|island| {
        island
            .body_indices
            .iter()
            .map(|&index| bodies[index].handle)
            .min()
            .expect("every solver island contains a dynamic body")
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
    fn empty_world() {
        let islands = build_islands::<3>(&[], &[], &[], &HashMap::new());
        assert!(islands.is_empty());
    }
}

#[cfg(test)]
mod fixed_body_island_tests {
    use super::*;
    use crate::body::BodyType;
    use crate::contact::ContactManifold;
    use nalgebra::SVector;
    use symtropy_math::{Point, Shape, Sphere, Transform};

    fn handles_for(bodies: &[RigidBody<3>]) -> HashMap<BodyHandle, usize> {
        bodies
            .iter()
            .enumerate()
            .map(|(index, body)| (body.handle, index))
            .collect()
    }

    fn contact(a: usize, b: usize) -> ContactManifold<3> {
        ContactManifold::single(
            BodyHandle(a),
            BodyHandle(b),
            SVector::from([0.0, 1.0, 0.0]),
            SVector::zeros(),
            0.01,
        )
    }

    fn kinematic_body(handle: usize) -> RigidBody<3> {
        let collider: Box<dyn Shape<3>> = Box::new(Sphere::<3>::unit());
        RigidBody::new(
            BodyHandle(handle),
            BodyType::Kinematic,
            Transform::from_translation(Point::origin()),
            collider,
            0.0,
            SVector::zeros(),
        )
    }

    #[test]
    fn shared_static_floor_does_not_merge_independent_dynamic_islands() {
        let bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([-2.0, 1.0, 0.0]), 0.5, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([2.0, 1.0, 0.0]), 0.5, 1.0),
            RigidBody::<3>::static_body(
                BodyHandle(2),
                Point::origin(),
                Box::new(Sphere::<3>::unit()),
            ),
        ];
        let contacts = vec![contact(0, 2), contact(1, 2)];
        let islands = build_islands(&bodies, &contacts, &[], &handles_for(&bodies));

        assert_eq!(islands.len(), 2);
        assert!(islands.iter().all(|island| island.body_indices.len() == 1));
        assert!(islands.iter().all(|island| {
            island
                .body_indices
                .iter()
                .all(|&index| bodies[index].is_dynamic())
        }));
        let assigned: Vec<_> = islands
            .iter()
            .flat_map(|island| island.contact_indices.iter().copied())
            .collect();
        assert_eq!(assigned, vec![0, 1]);
    }

    #[test]
    fn static_floor_does_not_wake_sleeping_dynamic_islands() {
        let mut bodies = vec![
            RigidBody::<3>::dynamic_sphere(
                BodyHandle(0),
                Point::new([-2.0, 1.0, 0.0]),
                0.5,
                1.0,
            ),
            RigidBody::<3>::dynamic_sphere(
                BodyHandle(1),
                Point::new([2.0, 1.0, 0.0]),
                0.5,
                1.0,
            ),
            RigidBody::<3>::static_body(
                BodyHandle(2),
                Point::origin(),
                Box::new(Sphere::<3>::unit()),
            ),
        ];
        bodies[0].sleeping = true;
        bodies[1].sleeping = true;
        let contacts = vec![contact(0, 2), contact(1, 2)];
        let islands = build_islands(&bodies, &contacts, &[], &handles_for(&bodies));

        assert_eq!(islands.len(), 2);
        assert!(islands.iter().all(|island| island.sleeping));
    }

    #[test]
    fn shared_kinematic_platform_does_not_merge_dynamic_solver_islands() {
        let mut bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([-1.0, 1.0, 0.0]), 0.5, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.0, 1.0, 0.0]), 0.5, 1.0),
            kinematic_body(2),
        ];
        bodies[0].sleeping = true;
        bodies[1].sleeping = true;
        let contacts = vec![contact(0, 2), contact(1, 2)];
        let islands = build_islands(&bodies, &contacts, &[], &handles_for(&bodies));

        assert_eq!(islands.len(), 2);
        assert!(islands.iter().all(|island| !island.sleeping));
        assert_eq!(
            islands
                .iter()
                .flat_map(|island| island.contact_indices.iter().copied())
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    #[test]
    fn static_only_world_has_no_solver_islands() {
        let bodies = vec![
            RigidBody::<3>::static_body(
                BodyHandle(0),
                Point::origin(),
                Box::new(Sphere::<3>::unit()),
            ),
            RigidBody::<3>::static_body(
                BodyHandle(1),
                Point::new([2.0, 0.0, 0.0]),
                Box::new(Sphere::<3>::unit()),
            ),
        ];
        let islands = build_islands(&bodies, &[contact(0, 1)], &[], &handles_for(&bodies));
        assert!(islands.is_empty());
    }

    #[test]
    fn dynamic_contact_chains_remain_one_island_with_fixed_endpoints() {
        let bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([0.0, 1.0, 0.0]), 0.5, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.0, 1.0, 0.0]), 0.5, 1.0),
            RigidBody::<3>::static_body(
                BodyHandle(2),
                Point::origin(),
                Box::new(Sphere::<3>::unit()),
            ),
        ];
        let contacts = vec![contact(0, 1), contact(1, 2)];
        let islands = build_islands(&bodies, &contacts, &[], &handles_for(&bodies));

        assert_eq!(islands.len(), 1);
        assert_eq!(islands[0].body_indices.len(), 2);
        assert_eq!(islands[0].contact_indices, vec![0, 1]);
    }

    #[test]
    fn islands_are_canonicalized_by_smallest_dynamic_handle() {
        let bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(10), Point::new([0.0, 0.0, 0.0]), 0.5, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(2), Point::new([10.0, 0.0, 0.0]), 0.5, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(6), Point::new([20.0, 0.0, 0.0]), 0.5, 1.0),
        ];
        let islands = build_islands(&bodies, &[], &[], &handles_for(&bodies));
        let first_handles: Vec<_> = islands
            .iter()
            .map(|island| {
                island
                    .body_indices
                    .iter()
                    .map(|&index| bodies[index].handle)
                    .min()
                    .unwrap()
            })
            .collect();

        assert_eq!(
            first_handles,
            vec![BodyHandle(2), BodyHandle(6), BodyHandle(10)]
        );
    }

    #[test]
    fn static_anchor_constraints_are_assigned_once_to_dynamic_islands() {
        use crate::constraint::DistanceConstraint;

        let bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([-2.0, 0.0, 0.0]), 0.5, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([2.0, 0.0, 0.0]), 0.5, 1.0),
            RigidBody::<3>::static_body(
                BodyHandle(2),
                Point::origin(),
                Box::new(Sphere::<3>::unit()),
            ),
        ];
        let constraints: Vec<Box<dyn Constraint<3>>> = vec![
            Box::new(DistanceConstraint {
                body_a: BodyHandle(0),
                body_b: BodyHandle(2),
                rest_length: 2.0,
                stiffness: 1.0,
            }),
            Box::new(DistanceConstraint {
                body_a: BodyHandle(1),
                body_b: BodyHandle(2),
                rest_length: 2.0,
                stiffness: 1.0,
            }),
        ];
        let islands = build_islands(&bodies, &[], &constraints, &handles_for(&bodies));

        assert_eq!(islands.len(), 2);
        let assigned: Vec<_> = islands
            .iter()
            .flat_map(|island| island.constraint_indices.iter().copied())
            .collect();
        assert_eq!(assigned, vec![0, 1]);
    }
}
