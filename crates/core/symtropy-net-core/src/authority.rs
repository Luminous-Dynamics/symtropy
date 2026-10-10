// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Spatial authority partitioning for P2P physics.

use crate::peer::PeerId;
use std::collections::{HashMap, HashSet};
use symtropy_physics::body::BodyHandle;

/// Spatial authority system: determines which peer computes physics for which bodies.
pub struct SpatialAuthority {
    /// Which peer has authority over each body.
    body_authority: HashMap<BodyHandle, PeerId>,
    /// Bodies owned by the local peer.
    local_bodies: HashSet<BodyHandle>,
    /// Authority radius (bodies within this distance of the peer's player are owned).
    pub authority_radius: f64,
    /// Local peer ID.
    pub local_peer: PeerId,
}

impl SpatialAuthority {
    /// Create a new spatial authority system.
    pub fn new(local_peer: PeerId, authority_radius: f64) -> Self {
        Self {
            body_authority: HashMap::new(),
            local_bodies: HashSet::new(),
            authority_radius,
            local_peer,
        }
    }

    /// Replace the provisional local peer ID with the identity assigned by the
    /// transport handshake, preserving the local peer's existing body claims.
    pub fn reidentify_local_peer(&mut self, new_local_peer: PeerId) {
        let old_local_peer = self.local_peer;
        if old_local_peer == new_local_peer {
            return;
        }

        for owner in self.body_authority.values_mut() {
            if *owner == old_local_peer {
                *owner = new_local_peer;
            }
        }

        // local_bodies is keyed by body, so it remains valid across ID remapping.
        self.local_peer = new_local_peer;
    }

    /// Claim authority over a body.
    pub fn claim(&mut self, body: BodyHandle, peer: PeerId) {
        self.body_authority.insert(body, peer);
        if peer == self.local_peer {
            self.local_bodies.insert(body);
        } else {
            self.local_bodies.remove(&body);
        }
    }

    /// Release authority over a body.
    pub fn release(&mut self, body: BodyHandle) {
        if let Some(peer) = self.body_authority.remove(&body)
            && peer == self.local_peer
        {
            self.local_bodies.remove(&body);
        }
    }

    /// Whether the local peer has authority over a body.
    pub fn is_local(&self, body: BodyHandle) -> bool {
        self.local_bodies.contains(&body)
    }

    /// Which peer has authority over a body.
    pub fn authority_of(&self, body: BodyHandle) -> Option<PeerId> {
        self.body_authority.get(&body).copied()
    }

    /// All bodies the local peer has authority over.
    pub fn local_body_count(&self) -> usize {
        self.local_bodies.len()
    }

    /// All bodies with assigned authority.
    pub fn total_claimed(&self) -> usize {
        self.body_authority.len()
    }

    /// Update authority based on distances from peers' players.
    pub fn update_from_distances(
        &mut self,
        bodies: &[BodyHandle],
        body_distances_to_local: &HashMap<BodyHandle, f64>,
        remote_peer_claims: &HashMap<BodyHandle, PeerId>,
    ) {
        for &body in bodies {
            let local_dist = body_distances_to_local
                .get(&body)
                .copied()
                .unwrap_or(f64::MAX);

            if local_dist < self.authority_radius {
                if let Some(&remote_peer) = remote_peer_claims.get(&body) {
                    self.claim(body, remote_peer);
                } else {
                    self.claim(body, self.local_peer);
                }
            } else if let Some(&remote_peer) = remote_peer_claims.get(&body) {
                self.claim(body, remote_peer);
            } else {
                self.release(body);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_physics::body::BodyHandle;

    #[test]
    fn reidentifying_local_peer_preserves_and_remaps_local_claims() {
        let provisional = PeerId(999);
        let assigned = PeerId(1);
        let remote = PeerId(2);
        let local_body = BodyHandle(10);
        let remote_body = BodyHandle(20);
        let mut authority = SpatialAuthority::new(provisional, 100.0);

        authority.claim(local_body, provisional);
        authority.claim(remote_body, remote);
        authority.reidentify_local_peer(assigned);

        assert_eq!(authority.local_peer, assigned);
        assert_eq!(authority.authority_of(local_body), Some(assigned));
        assert_eq!(authority.authority_of(remote_body), Some(remote));
        assert!(authority.is_local(local_body));
        assert!(!authority.is_local(remote_body));
        assert_eq!(authority.local_body_count(), 1);
    }

    #[test]
    fn reidentifying_local_peer_to_same_id_is_idempotent() {
        let local = PeerId(1);
        let body = BodyHandle(10);
        let mut authority = SpatialAuthority::new(local, 100.0);
        authority.claim(body, local);

        authority.reidentify_local_peer(local);

        assert_eq!(authority.authority_of(body), Some(local));
        assert!(authority.is_local(body));
    }
}

