// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_net_core::{PeerId, SpatialAuthority};
use symtropy_physics::body::BodyHandle;

#[test]
fn release_peer_clears_only_that_peers_claims() {
    let mut authority = SpatialAuthority::new(PeerId(0), 100.0);
    let local_body = BodyHandle(1);
    let disconnected_body = BodyHandle(2);
    let other_remote_body = BodyHandle(3);

    authority.claim(local_body, PeerId(0));
    authority.claim(disconnected_body, PeerId(1));
    authority.claim(other_remote_body, PeerId(2));

    assert_eq!(authority.release_peer(PeerId(1)), 1);
    assert_eq!(authority.authority_of(disconnected_body), None);
    assert_eq!(authority.authority_of(local_body), Some(PeerId(0)));
    assert_eq!(authority.authority_of(other_remote_body), Some(PeerId(2)));
    assert!(authority.is_local(local_body));
    assert_eq!(authority.local_body_count(), 1);
    assert_eq!(authority.total_claimed(), 2);
}

#[test]
fn release_peer_is_idempotent() {
    let mut authority = SpatialAuthority::new(PeerId(0), 100.0);
    authority.claim(BodyHandle(2), PeerId(1));

    assert_eq!(authority.release_peer(PeerId(1)), 1);
    assert_eq!(authority.release_peer(PeerId(1)), 0);
    assert_eq!(authority.total_claimed(), 0);
}
