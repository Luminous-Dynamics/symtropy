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

    /// Release every body claim held by a peer and return the number of claims removed.
    ///
    /// Call when a peer disconnects or loses its authority lease. Removing a
    /// claim does not transfer authority; a subsequent election/lease must assign
    /// the body to another peer before that peer simulates it.
    pub fn release_peer(&mut self, peer: PeerId) -> usize {
        let owned_bodies: Vec<BodyHandle> = self
            .body_authority
            .iter()
            .filter_map(|(body, owner)| (*owner == peer).then_some(*body))
            .collect();

        for body in &owned_bodies {
            self.release(*body);
        }

        owned_bodies.len()
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
}
