// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Digest-bound, non-authoritative views over domain-owned world state.
//!
//! `symtropy-world` coordinates scales and presentation. It must not become a
//! second terrain, hydrology, climate, or ecology authority. These types let a
//! cell cache derived domain observations only when their source authority,
//! scope, frame, representation, time, and state digest are explicit.

use std::{error::Error, fmt};

use symtropy_sim_contracts::{
    AuthorityId, ContractError, ReferenceFrameId, RepresentationId, ScopeId, SimInstant,
    TypedDigest32,
};

use crate::grid::{BiomeKind, HexCellId, PlanetCell};

/// Stable geometric identity for one body-local cell.
///
/// This deliberately excludes biome, hydrology, climate, resources, and other
/// mutable domain claims.
#[derive(Clone, Debug, PartialEq)]
pub struct BodyCellIdentity {
    pub id: HexCellId,
    pub center_lat_deg: f64,
    pub center_lon_deg: f64,
    pub area_m2: f64,
}

impl BodyCellIdentity {
    /// Extract only the identity/geometric portion of a legacy `PlanetCell`.
    ///
    /// None of the legacy physical/ecological fields are promoted here because
    /// they do not carry domain-authority provenance or typed state digests.
    pub fn from_legacy(cell: &PlanetCell) -> Self {
        Self {
            id: cell.id.clone(),
            center_lat_deg: cell.center_lat_deg,
            center_lon_deg: cell.center_lon_deg,
            area_m2: cell.area_m2,
        }
    }

    /// Canonical causal scope for this body cell.
    pub fn scope_id(&self) -> Result<ScopeId, AuthorityViewError> {
        ScopeId::parse(format!(
            "body-cell:{}/r{}/{}",
            self.id.body.as_str(),
            self.id.resolution,
            self.id.index
        ))
        .map_err(AuthorityViewError::Contract)
    }
}

/// A derived value whose provenance is explicit enough to be safely cached by
/// the world orchestrator. The owning domain remains authoritative.
#[derive(Clone, Debug, PartialEq)]
pub struct DerivedDomainView<T> {
    pub authority: AuthorityId,
    pub scope: ScopeId,
    pub reference_frame: ReferenceFrameId,
    pub representation: RepresentationId,
    pub observed_at: SimInstant,
    pub state_digest: TypedDigest32,
    pub value: T,
}

impl<T> DerivedDomainView<T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        authority: AuthorityId,
        scope: ScopeId,
        reference_frame: ReferenceFrameId,
        representation: RepresentationId,
        observed_at: SimInstant,
        state_digest: TypedDigest32,
        value: T,
    ) -> Result<Self, AuthorityViewError> {
        state_digest.validate().map_err(AuthorityViewError::Contract)?;
        Ok(Self {
            authority,
            scope,
            reference_frame,
            representation,
            observed_at,
            state_digest,
            value,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainCellSummary {
    pub elevation_m: f32,
    pub slope: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HydrologyCellSummary {
    pub surface_water_m: f32,
    pub groundwater_m: f32,
    pub flow_accumulation: f32,
    pub salinity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClimateCellSummary {
    pub temperature_k: f32,
    pub atmosphere_pressure_pa: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EcologyCellSummary {
    /// Transitional derived classification. This value is meaningful only
    /// because the containing view binds it to ecology authority evidence.
    pub biome: BiomeKind,
}

/// Non-authoritative orchestration/cache view for one planetary cell.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanetCellAuthorityView {
    pub identity: BodyCellIdentity,
    pub terrain: Option<DerivedDomainView<TerrainCellSummary>>,
    pub hydrology: Option<DerivedDomainView<HydrologyCellSummary>>,
    pub climate: Option<DerivedDomainView<ClimateCellSummary>>,
    pub ecology: Option<DerivedDomainView<EcologyCellSummary>>,
}

impl PlanetCellAuthorityView {
    /// Safe compatibility boundary for existing `PlanetCell` values.
    ///
    /// Legacy biome/hydrology/climate/terrain values are intentionally ignored:
    /// without authority provenance and a typed state digest they are migration
    /// hints, not authoritative claims.
    pub fn identity_only_from_legacy(cell: &PlanetCell) -> Self {
        Self {
            identity: BodyCellIdentity::from_legacy(cell),
            terrain: None,
            hydrology: None,
            climate: None,
            ecology: None,
        }
    }

    pub fn with_terrain(
        mut self,
        view: DerivedDomainView<TerrainCellSummary>,
    ) -> Result<Self, AuthorityViewError> {
        self.require_scope(&view)?;
        self.terrain = Some(view);
        Ok(self)
    }

    pub fn with_hydrology(
        mut self,
        view: DerivedDomainView<HydrologyCellSummary>,
    ) -> Result<Self, AuthorityViewError> {
        self.require_scope(&view)?;
        self.hydrology = Some(view);
        Ok(self)
    }

    pub fn with_climate(
        mut self,
        view: DerivedDomainView<ClimateCellSummary>,
    ) -> Result<Self, AuthorityViewError> {
        self.require_scope(&view)?;
        self.climate = Some(view);
        Ok(self)
    }

    pub fn with_ecology(
        mut self,
        view: DerivedDomainView<EcologyCellSummary>,
    ) -> Result<Self, AuthorityViewError> {
        self.require_scope(&view)?;
        self.ecology = Some(view);
        Ok(self)
    }

    fn require_scope<T>(&self, view: &DerivedDomainView<T>) -> Result<(), AuthorityViewError> {
        let expected = self.identity.scope_id()?;
        if view.scope == expected {
            Ok(())
        } else {
            Err(AuthorityViewError::ScopeMismatch {
                expected,
                actual: view.scope.clone(),
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityViewError {
    Contract(ContractError),
    ScopeMismatch { expected: ScopeId, actual: ScopeId },
}

impl From<ContractError> for AuthorityViewError {
    fn from(value: ContractError) -> Self {
        Self::Contract(value)
    }
}

impl fmt::Display for AuthorityViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract(error) => write!(formatter, "simulation contract rejected view: {error}"),
            Self::ScopeMismatch { expected, actual } => write!(
                formatter,
                "domain view scope {actual} does not match cell scope {expected}"
            ),
        }
    }
}

impl Error for AuthorityViewError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{BodyId, GridSystem, HydrologyState};

    fn legacy_cell() -> PlanetCell {
        let mut cell = PlanetCell::new(
            HexCellId::new(
                BodyId::earth(),
                GridSystem::EarthH3,
                7,
                "872830828ffffff",
            ),
            37.7749,
            -122.4194,
        );
        cell.area_m2 = 5_000_000.0;
        cell.elevation_m = 42.0;
        cell.slope = 0.18;
        cell.biome = BiomeKind::Forest;
        cell.hydrology = HydrologyState {
            surface_water_m: 1.2,
            groundwater_m: 4.5,
            flow_accumulation: 0.8,
            salinity: 0.02,
        };
        cell.temperature_k = 286.4;
        cell.atmosphere_pressure_pa = 100_800.0;
        cell
    }

    fn digest(domain: &str) -> TypedDigest32 {
        TypedDigest32::sha256(domain, 1, b"authoritative-state").unwrap()
    }

    fn hydrology_view(scope: ScopeId) -> DerivedDomainView<HydrologyCellSummary> {
        DerivedDomainView::new(
            AuthorityId::parse("hydrology.authority.v1").unwrap(),
            scope,
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            RepresentationId::parse("hydrology.cell-summary.v1").unwrap(),
            SimInstant::new(100, 0).unwrap(),
            digest("symtropy.hydrology.state.v1"),
            HydrologyCellSummary {
                surface_water_m: 1.2,
                groundwater_m: 4.5,
                flow_accumulation: 0.8,
                salinity: 0.02,
            },
        )
        .unwrap()
    }

    #[test]
    fn legacy_conversion_mints_no_domain_claims() {
        let legacy = legacy_cell();
        let view = PlanetCellAuthorityView::identity_only_from_legacy(&legacy);

        assert_eq!(view.identity.id, legacy.id);
        assert!(view.terrain.is_none());
        assert!(view.hydrology.is_none());
        assert!(view.climate.is_none());
        assert!(view.ecology.is_none());
    }

    #[test]
    fn matching_scope_can_attach_digest_bound_hydrology() {
        let legacy = legacy_cell();
        let view = PlanetCellAuthorityView::identity_only_from_legacy(&legacy);
        let scope = view.identity.scope_id().unwrap();
        let view = view.with_hydrology(hydrology_view(scope)).unwrap();

        assert_eq!(
            view.hydrology.unwrap().state_digest.domain,
            "symtropy.hydrology.state.v1"
        );
    }

    #[test]
    fn wrong_scope_is_rejected() {
        let legacy = legacy_cell();
        let view = PlanetCellAuthorityView::identity_only_from_legacy(&legacy);
        let wrong_scope = ScopeId::parse("body-cell:sol:mars/r7/872830828ffffff").unwrap();
        let error = view.with_hydrology(hydrology_view(wrong_scope)).unwrap_err();

        assert!(matches!(error, AuthorityViewError::ScopeMismatch { .. }));
    }

    #[test]
    fn biome_becomes_a_claim_only_when_digest_bound() {
        let legacy = legacy_cell();
        let view = PlanetCellAuthorityView::identity_only_from_legacy(&legacy);
        assert!(view.ecology.is_none());

        let scope = view.identity.scope_id().unwrap();
        let ecology = DerivedDomainView::new(
            AuthorityId::parse("ecology.authority.v1").unwrap(),
            scope,
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            RepresentationId::parse("ecology.biome-summary.v1").unwrap(),
            SimInstant::new(100, 0).unwrap(),
            digest("symtropy.ecology.state.v1"),
            EcologyCellSummary {
                biome: BiomeKind::Forest,
            },
        )
        .unwrap();
        let view = view.with_ecology(ecology).unwrap();

        assert_eq!(view.ecology.unwrap().value.biome, BiomeKind::Forest);
    }
}
