// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! External/public-surface adversarial corpus for ECON-05C.

use symtropy_core::prelude::*;

fn actor(value: &str) -> ActorId {
    ActorId::new(value).unwrap()
}

fn asset(value: &str) -> AssetId {
    AssetId::new(value).unwrap()
}

fn cause(value: &str) -> CausalId {
    CausalId::new(value).unwrap()
}

fn location(value: &str) -> LocationId {
    LocationId::new(value).unwrap()
}

fn cargo() -> CarrierCargoBinding {
    CarrierCargoBinding::new(
        asset("truck-7"),
        location("truck-7:cargo-hold"),
        actor("carrier"),
        cause("physics-session:alpha"),
    )
}

fn physics_ref(net_id: u64) -> CarrierPhysicsRef {
    CarrierPhysicsRef::new(
        asset("truck-7"),
        cause("physics-session:alpha"),
        NetId(net_id),
    )
}

fn world_with_body(net_id: u64) -> (PhysicsWorld<3>, BodyHandle) {
    let mut world = PhysicsWorld::<3>::default();
    let handle = world.add_sphere(Point::origin(), 1.0, 1.0);
    world.set_net_id(handle, NetId(net_id));
    (world, handle)
}

#[test]
fn unique_live_net_id_resolves_to_runtime_handle() {
    let (world, handle) = world_with_body(7);

    assert_eq!(
        resolve_live_carrier(&world, &cargo(), &physics_ref(7)).unwrap(),
        handle
    );
    assert_eq!(world.handle_for_net_id(NetId(7)), Some(handle));
    assert_eq!(world.net_id_for_handle(handle), Some(NetId(7)));
}

#[test]
fn carrier_asset_mismatch_fails_before_resolution() {
    let (world, _) = world_with_body(7);
    let reference = CarrierPhysicsRef::new(
        asset("different-truck"),
        cause("physics-session:alpha"),
        NetId(7),
    );

    assert_eq!(
        resolve_live_carrier(&world, &cargo(), &reference),
        Err(CarrierPhysicsError::CarrierAssetMismatch {
            economic: asset("truck-7"),
            physical: asset("different-truck"),
        })
    );
}

#[test]
fn physical_authority_mismatch_fails_before_resolution() {
    let (world, _) = world_with_body(7);
    let reference = CarrierPhysicsRef::new(
        asset("truck-7"),
        cause("physics-session:beta"),
        NetId(7),
    );

    assert_eq!(
        resolve_live_carrier(&world, &cargo(), &reference),
        Err(CarrierPhysicsError::PhysicalAuthorityMismatch {
            economic: cause("physics-session:alpha"),
            physical: cause("physics-session:beta"),
        })
    );
}

#[test]
fn unknown_net_id_fails_closed() {
    let (world, _) = world_with_body(7);

    assert_eq!(
        resolve_live_carrier(&world, &cargo(), &physics_ref(8)),
        Err(CarrierPhysicsError::UnknownNetId { net_id: NetId(8) })
    );
}

#[test]
fn duplicate_net_id_state_is_rejected_even_when_index_points_to_one_body() {
    let mut world = PhysicsWorld::<3>::default();
    let first = world.add_sphere(Point::origin(), 1.0, 1.0);
    let second = world.add_sphere(Point::origin(), 1.0, 1.0);
    world.set_net_id(first, NetId(7));

    // Legacy PhysicsWorld::set_net_id currently permits this and overwrites
    // net_id_map while leaving the first body's field unchanged. ECON-05C must
    // not trust that index as unique physical identity.
    world.set_net_id(second, NetId(7));

    assert_eq!(world.net_id_for_handle(first), Some(NetId(7)));
    assert_eq!(world.net_id_for_handle(second), Some(NetId(7)));
    assert_eq!(
        resolve_live_carrier(&world, &cargo(), &physics_ref(7)),
        Err(CarrierPhysicsError::AmbiguousNetId { net_id: NetId(7) })
    );
}

#[test]
fn direct_body_identity_mutation_with_stale_index_is_rejected() {
    let (mut world, handle) = world_with_body(7);

    // RigidBody::net_id is currently public. Simulate downstream code mutating
    // it without rebuilding PhysicsWorld::net_id_map.
    world.body_mut(handle).unwrap().net_id = Some(NetId(8));

    assert_eq!(world.handle_for_net_id(NetId(8)), None);
    assert_eq!(
        resolve_live_carrier(&world, &cargo(), &physics_ref(8)),
        Err(CarrierPhysicsError::IdentityIndexMissing {
            net_id: NetId(8),
            handle,
        })
    );
}
