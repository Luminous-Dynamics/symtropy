// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_physics::{
    LocalNamespacePhysicsAuthorityWorld, LocalQualifiedPhysicalAuthority, PhysicsWorld,
};

#[test]
fn fresh_local_authority_roots_are_distinct() {
    let first = LocalQualifiedPhysicalAuthority::mint().expect("first root should mint");
    let second = LocalQualifiedPhysicalAuthority::mint().expect("second root should mint");

    assert_ne!(first.physical_authority_id(), second.physical_authority_id());
    assert_ne!(first.physical_authority_id().get(), 0);
    assert_ne!(second.physical_authority_id().get(), 0);
}

#[test]
fn one_root_mints_distinct_generation_namespaces() {
    let mut root = LocalQualifiedPhysicalAuthority::mint().expect("root should mint");
    let first = root.mint_generation().expect("first generation should mint");
    let second = root
        .mint_generation()
        .expect("second generation should mint");

    assert_eq!(first.physical_authority_id(), root.physical_authority_id());
    assert_eq!(second.physical_authority_id(), root.physical_authority_id());
    assert_ne!(first.world_generation_id(), second.world_generation_id());
}

#[test]
fn generation_namespace_pairs_remain_distinct_across_roots() {
    let mut first_root = LocalQualifiedPhysicalAuthority::mint().expect("first root should mint");
    let mut second_root =
        LocalQualifiedPhysicalAuthority::mint().expect("second root should mint");

    let first_generation = first_root
        .mint_generation()
        .expect("first generation should mint");
    let second_generation = second_root
        .mint_generation()
        .expect("second generation should mint");

    assert_ne!(
        (
            first_generation.physical_authority_id(),
            first_generation.world_generation_id(),
        ),
        (
            second_generation.physical_authority_id(),
            second_generation.world_generation_id(),
        )
    );
}

#[test]
fn binding_consumes_generation_capability_and_preserves_exact_namespace() {
    let mut root = LocalQualifiedPhysicalAuthority::mint().expect("root should mint");
    let generation = root.mint_generation().expect("generation should mint");
    let expected_authority = generation.physical_authority_id();
    let expected_generation = generation.world_generation_id();

    let qualified =
        LocalNamespacePhysicsAuthorityWorld::bind(generation, PhysicsWorld::<2>::default());

    assert_eq!(qualified.physical_authority_id(), expected_authority);
    assert_eq!(qualified.world_generation_id(), expected_generation);
    assert_eq!(
        qualified.authority_world().physical_authority_id(),
        expected_authority
    );
    assert_eq!(
        qualified.authority_world().world_generation_id(),
        expected_generation
    );
}

#[test]
fn explicit_mutable_projection_keeps_namespace_binding() {
    let mut root = LocalQualifiedPhysicalAuthority::mint().expect("root should mint");
    let generation = root.mint_generation().expect("generation should mint");
    let expected_authority = generation.physical_authority_id();
    let expected_generation = generation.world_generation_id();

    let mut qualified =
        LocalNamespacePhysicsAuthorityWorld::bind(generation, PhysicsWorld::<2>::default());

    assert_eq!(
        qualified.authority_world_mut().physical_authority_id(),
        expected_authority
    );
    assert_eq!(
        qualified.authority_world_mut().world_generation_id(),
        expected_generation
    );
    assert_eq!(qualified.physical_authority_id(), expected_authority);
    assert_eq!(qualified.world_generation_id(), expected_generation);
}

#[test]
fn into_unqualified_preserves_ids_but_drops_capability_wrapper() {
    let mut root = LocalQualifiedPhysicalAuthority::mint().expect("root should mint");
    let generation = root.mint_generation().expect("generation should mint");
    let expected_authority = generation.physical_authority_id();
    let expected_generation = generation.world_generation_id();

    let qualified =
        LocalNamespacePhysicsAuthorityWorld::bind(generation, PhysicsWorld::<2>::default());
    let unqualified = qualified.into_unqualified();

    assert_eq!(unqualified.physical_authority_id(), expected_authority);
    assert_eq!(unqualified.world_generation_id(), expected_generation);
}
