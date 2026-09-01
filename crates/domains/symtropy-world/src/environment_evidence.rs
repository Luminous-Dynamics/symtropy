// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Non-authoritative, exact-time bundles of environmental observation evidence.
//!
//! The bundle intentionally contains provenance only. It does not copy terrain,
//! hydrology, climate, or ecology values and therefore cannot become a second
//! environmental state store. Exact time/frame coherence is strict: interpolation,
//! extrapolation, or stale-data tolerance must be an explicit domain policy.

use std::{error::Error, fmt};

use symtropy_sim_contracts::{
    ObservationEvidence, ReferenceFrameId, ScopeId, SimInstant,
};

use crate::authority_view::PlanetCellAuthorityView;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentalEvidenceBundle {
    pub scope: ScopeId,
    pub reference_frame: ReferenceFrameId,
    pub observed_at: SimInstant,
    pub terrain: Option<ObservationEvidence>,
    pub hydrology: Option<ObservationEvidence>,
    pub climate: Option<ObservationEvidence>,
    pub ecology: Option<ObservationEvidence>,
}

impl EnvironmentalEvidenceBundle {
    /// Extract a provenance-only snapshot from the views currently attached to
    /// one planetary cell. All present observations must share exact scope,
    /// reference frame, and simulation instant.
    pub fn exact_from_cell(
        cell: &PlanetCellAuthorityView,
    ) -> Result<Self, EnvironmentalEvidenceError> {
        let scope = cell
            .identity
            .scope_id()
            .map_err(|error| EnvironmentalEvidenceError::Identity(error.to_string()))?;

        let terrain = cell
            .terrain
            .as_ref()
            .map(|view| view.observation_evidence())
            .transpose()
            .map_err(|error| EnvironmentalEvidenceError::Observation(error.to_string()))?;
        let hydrology = cell
            .hydrology
            .as_ref()
            .map(|view| view.observation_evidence())
            .transpose()
            .map_err(|error| EnvironmentalEvidenceError::Observation(error.to_string()))?;
        let climate = cell
            .climate
            .as_ref()
            .map(|view| view.observation_evidence())
            .transpose()
            .map_err(|error| EnvironmentalEvidenceError::Observation(error.to_string()))?;
        let ecology = cell
            .ecology
            .as_ref()
            .map(|view| view.observation_evidence())
            .transpose()
            .map_err(|error| EnvironmentalEvidenceError::Observation(error.to_string()))?;

        let observations = [
            terrain.as_ref(),
            hydrology.as_ref(),
            climate.as_ref(),
            ecology.as_ref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let Some(first) = observations.first() else {
            return Err(EnvironmentalEvidenceError::NoObservations);
        };

        if first.scope != scope {
            return Err(EnvironmentalEvidenceError::ScopeMismatch {
                expected: scope,
                actual: first.scope.clone(),
            });
        }

        for observation in observations.iter().skip(1) {
            if observation.scope != scope {
                return Err(EnvironmentalEvidenceError::ScopeMismatch {
                    expected: scope,
                    actual: observation.scope.clone(),
                });
            }
            if observation.reference_frame != first.reference_frame {
                return Err(EnvironmentalEvidenceError::ReferenceFrameMismatch {
                    expected: first.reference_frame.clone(),
                    actual: observation.reference_frame.clone(),
                });
            }
            if observation.observed_at != first.observed_at {
                return Err(EnvironmentalEvidenceError::ObservationTimeMismatch {
                    expected: first.observed_at,
                    actual: observation.observed_at,
                });
            }
        }

        Ok(Self {
            scope,
            reference_frame: first.reference_frame.clone(),
            observed_at: first.observed_at,
            terrain,
            hydrology,
            climate,
            ecology,
        })
    }

    pub fn observation_count(&self) -> usize {
        [
            self.terrain.is_some(),
            self.hydrology.is_some(),
            self.climate.is_some(),
            self.ecology.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count()
    }

    /// Deterministic source-evidence digests in semantic domain order.
    pub fn source_digests(&self) -> Vec<symtropy_sim_contracts::TypedDigest32> {
        [
            self.terrain.as_ref(),
            self.hydrology.as_ref(),
            self.climate.as_ref(),
            self.ecology.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(|observation| observation.state_digest.clone())
        .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvironmentalEvidenceError {
    NoObservations,
    Identity(String),
    Observation(String),
    ScopeMismatch {
        expected: ScopeId,
        actual: ScopeId,
    },
    ReferenceFrameMismatch {
        expected: ReferenceFrameId,
        actual: ReferenceFrameId,
    },
    ObservationTimeMismatch {
        expected: SimInstant,
        actual: SimInstant,
    },
}

impl fmt::Display for EnvironmentalEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoObservations => write!(formatter, "environmental evidence bundle has no observations"),
            Self::Identity(error) => write!(formatter, "invalid cell identity: {error}"),
            Self::Observation(error) => write!(formatter, "invalid observation evidence: {error}"),
            Self::ScopeMismatch { expected, actual } => write!(
                formatter,
                "environment observation scope {actual} does not match cell scope {expected}"
            ),
            Self::ReferenceFrameMismatch { expected, actual } => write!(
                formatter,
                "environment observations use different reference frames: {expected} vs {actual}"
            ),
            Self::ObservationTimeMismatch { expected, actual } => write!(
                formatter,
                "environment observations are asynchronous: {expected:?} vs {actual:?}"
            ),
        }
    }
}

impl Error for EnvironmentalEvidenceError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BodyCellIdentity, BodyId, ClimateCellSummary, DerivedDomainView, GridSystem, HexCellId,
        HydrologyCellSummary, PlanetCellAuthorityView, TerrainCellSummary,
    };
    use symtropy_sim_contracts::{
        AuthorityId, ReferenceFrameId, RepresentationId, TypedDigest32,
    };

    fn cell() -> PlanetCellAuthorityView {
        PlanetCellAuthorityView {
            identity: BodyCellIdentity {
                id: HexCellId::new(
                    BodyId::earth(),
                    GridSystem::BodyIcosahedral,
                    7,
                    "cell-7",
                ),
                center_lat_deg: 0.0,
                center_lon_deg: 0.0,
                area_m2: 100.0,
            },
            terrain: None,
            hydrology: None,
            climate: None,
            ecology: None,
        }
    }

    fn terrain_view(at: SimInstant) -> DerivedDomainView<TerrainCellSummary> {
        let scope = cell().identity.scope_id().unwrap();
        DerivedDomainView::new(
            AuthorityId::parse("terrain.authority.v1").unwrap(),
            scope,
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            RepresentationId::parse("terrain.voxel.v2").unwrap(),
            at,
            TypedDigest32::sha256("terrain.state.v2", 2, b"terrain").unwrap(),
            TerrainCellSummary {
                elevation_m: 42.0,
                slope: 0.1,
            },
        )
        .unwrap()
    }

    fn hydrology_view(at: SimInstant) -> DerivedDomainView<HydrologyCellSummary> {
        let scope = cell().identity.scope_id().unwrap();
        DerivedDomainView::new(
            AuthorityId::parse("hydrology.authority.v1").unwrap(),
            scope,
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            RepresentationId::parse("hydrology.local-flow.v1").unwrap(),
            at,
            TypedDigest32::sha256("hydrology.state.v1", 1, b"hydrology").unwrap(),
            HydrologyCellSummary {
                surface_water_m: 0.2,
                groundwater_m: 1.0,
                flow_accumulation: 3.0,
                salinity: 0.01,
            },
        )
        .unwrap()
    }

    #[test]
    fn exact_bundle_contains_evidence_not_cached_values() {
        let at = SimInstant::new(100, 0).unwrap();
        let cell = cell()
            .with_terrain(terrain_view(at))
            .unwrap()
            .with_hydrology(hydrology_view(at))
            .unwrap();
        let bundle = EnvironmentalEvidenceBundle::exact_from_cell(&cell).unwrap();

        assert_eq!(bundle.observation_count(), 2);
        assert!(bundle.terrain.is_some());
        assert!(bundle.hydrology.is_some());
        assert_eq!(bundle.source_digests().len(), 2);
    }

    #[test]
    fn asynchronous_observations_are_not_silently_coherent() {
        let cell = cell()
            .with_terrain(terrain_view(SimInstant::new(100, 0).unwrap()))
            .unwrap()
            .with_hydrology(hydrology_view(SimInstant::new(103, 0).unwrap()))
            .unwrap();

        assert!(matches!(
            EnvironmentalEvidenceBundle::exact_from_cell(&cell),
            Err(EnvironmentalEvidenceError::ObservationTimeMismatch { .. })
        ));
    }

    #[test]
    fn identity_only_cell_cannot_mint_environmental_evidence() {
        assert_eq!(
            EnvironmentalEvidenceBundle::exact_from_cell(&cell()),
            Err(EnvironmentalEvidenceError::NoObservations)
        );
    }

    #[test]
    fn climate_can_join_same_exact_epoch() {
        let at = SimInstant::new(100, 0).unwrap();
        let scope = cell().identity.scope_id().unwrap();
        let climate = DerivedDomainView::new(
            AuthorityId::parse("climate.authority.v1").unwrap(),
            scope,
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            RepresentationId::parse("climate.local.v1").unwrap(),
            at,
            TypedDigest32::sha256("climate.state.v1", 1, b"climate").unwrap(),
            ClimateCellSummary {
                temperature_k: 290.0,
                atmosphere_pressure_pa: 100_000.0,
            },
        )
        .unwrap();
        let cell = cell()
            .with_terrain(terrain_view(at))
            .unwrap()
            .with_climate(climate)
            .unwrap();

        let bundle = EnvironmentalEvidenceBundle::exact_from_cell(&cell).unwrap();
        assert_eq!(bundle.observation_count(), 2);
        assert!(bundle.climate.is_some());
    }
}
