// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Portable provenance extraction from world-layer derived views.

use symtropy_sim_contracts::{ContractError, ObservationEvidence};

use crate::authority_view::DerivedDomainView;

impl<T> DerivedDomainView<T> {
    /// Drop the cached value and retain only portable source provenance.
    ///
    /// This allows Basin, LifeSim, Terrain bridges, persistence, networking, or
    /// replay code to consume evidence without depending on the cached value's
    /// Rust type and without treating `symtropy-world` as an authority.
    pub fn observation_evidence(&self) -> Result<ObservationEvidence, ContractError> {
        ObservationEvidence::new(
            self.authority.clone(),
            self.scope.clone(),
            self.reference_frame.clone(),
            self.representation.clone(),
            self.observed_at,
            self.state_digest.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_sim_contracts::{
        AuthorityId, ReferenceFrameId, RepresentationId, ScopeId, SimInstant, TypedDigest32,
    };

    #[test]
    fn derived_view_exports_exact_source_provenance() {
        let view = DerivedDomainView::new(
            AuthorityId::parse("hydrology.authority.v1").unwrap(),
            ScopeId::parse("sol:earth:firstlight/cell-7").unwrap(),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            RepresentationId::parse("hydrology.local-flow.v1").unwrap(),
            SimInstant::new(77, 3).unwrap(),
            TypedDigest32::sha256("hydrology.state.v1", 1, b"state").unwrap(),
            42_u32,
        )
        .unwrap();

        let evidence = view.observation_evidence().unwrap();
        assert_eq!(evidence.authority, view.authority);
        assert_eq!(evidence.scope, view.scope);
        assert_eq!(evidence.reference_frame, view.reference_frame);
        assert_eq!(evidence.representation, view.representation);
        assert_eq!(evidence.observed_at, view.observed_at);
        assert_eq!(evidence.state_digest, view.state_digest);
    }
}
