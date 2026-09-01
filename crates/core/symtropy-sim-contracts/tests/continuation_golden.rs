// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Golden vectors for serializer-independent continuation identity.

use symtropy_sim_contracts::{
    FixedTimebase, LifecycleMode, ReferenceFrameId, SimInstant, TimebaseId, TypedDigest32,
    WorldContinuationManifest, WorldInstanceId,
};

fn digest(domain: &str, bytes: &[u8]) -> TypedDigest32 {
    TypedDigest32::sha256(domain, 1, bytes).expect("valid test digest")
}

fn golden_timebase() -> FixedTimebase {
    FixedTimebase::new(
        TimebaseId::parse("gameplay.fixed.test.v1").expect("valid timebase id"),
        digest("symtropy.test.genesis.v1", b"world-genesis"),
        0,
        SimInstant::GENESIS,
        50_000_000,
    )
    .expect("valid fixed timebase")
}

#[test]
fn fixed_timebase_v1_golden_vector() {
    let actual = golden_timebase().digest().expect("timebase digest");
    let expected = [
        0x43, 0x60, 0x8b, 0xcf, 0x13, 0x9d, 0x92, 0x22, 0xc3, 0x56, 0xa0, 0x35, 0x30,
        0x99, 0x1e, 0x35, 0x81, 0xc7, 0x2b, 0x85, 0x3f, 0x90, 0xb4, 0x20, 0x5f, 0x92,
        0x3b, 0xc5, 0x87, 0x4a, 0x0a, 0x30,
    ];
    assert_eq!(actual.value, expected);
}

#[test]
fn minimal_world_continuation_manifest_v1_golden_vector() {
    let manifest = WorldContinuationManifest::new(
        WorldInstanceId::parse("world:golden").expect("valid world id"),
        0,
        LifecycleMode::Genesis,
        None,
        SimInstant::new(20, 0).expect("valid instant"),
        golden_timebase().digest().expect("timebase digest"),
        ReferenceFrameId::parse("sol:earth:surface-fixed").expect("valid frame"),
        digest("symtropy.inactive-time-policy.v1", b"paused"),
        None,
        None,
        None,
        vec![],
        vec![],
    )
    .expect("valid minimal manifest");

    let actual = manifest.digest().expect("manifest digest");
    let expected = [
        0x40, 0xd8, 0xdd, 0x14, 0xec, 0x5e, 0x07, 0x86, 0xcd, 0xba, 0x70, 0x29, 0x37,
        0xb0, 0x27, 0x22, 0x4d, 0x53, 0x82, 0x7d, 0xd9, 0x75, 0xfc, 0xc0, 0x02, 0xac,
        0x0c, 0x2b, 0xac, 0xad, 0x01, 0x86,
    ];
    assert_eq!(actual.value, expected);
}
