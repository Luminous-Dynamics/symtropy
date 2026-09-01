// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Narrow deterministic Living Watershed reference policy.
//!
//! The policy proposes existing Basin interventions from exact digest-bound
//! world observations. It does not mutate Basin state. An owning executor may
//! apply the proposal and then mint a v0.7 ingest receipt from before/after state.

use std::{error::Error, fmt};

use sha2::{Digest, Sha256};
use symtropy_basin::BasinIntervention;
use symtropy_sim_contracts::{
    AuthorityId, ContractError, DigestAlgorithm, TypedDigest32,
};

use crate::{
    BasinEnvironmentalIngestError, BasinEnvironmentalIngestReceipt, EnvironmentalEvidenceBundle,
    EnvironmentalEvidenceError, PlanetCellAuthorityView,
};

pub const LIVING_WATERSHED_POLICY_DOMAIN: &str =
    "symtropy.basin.environment-policy.living-watershed.v1";
pub const LIVING_WATERSHED_POLICY_SCHEMA_VERSION: u32 = 1;

pub const FLOOD_SURFACE_WATER_M: f32 = 0.75;
pub const FLOOD_MAX_SLOPE: f32 = 0.20;
pub const FLOOD_MIN_FLOW_ACCUMULATION: f32 = 1.0;

pub const RIPARIAN_MIN_SURFACE_WATER_M: f32 = 0.10;
pub const RIPARIAN_MAX_SURFACE_WATER_M: f32 = 0.50;
pub const RIPARIAN_MAX_SLOPE: f32 = 0.20;
pub const RIPARIAN_MAX_SALINITY: f32 = 0.10;
pub const RIPARIAN_MIN_TEMPERATURE_K: f32 = 278.0;
pub const RIPARIAN_MAX_TEMPERATURE_K: f32 = 303.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LivingWatershedReason {
    FloodplainPonding,
    RiparianRestorationWindow,
    MissingClimateForRiparianDecision,
    Observe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LivingWatershedProposal {
    pub intervention: Option<BasinIntervention>,
    pub reason: LivingWatershedReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LivingWatershedEvaluation {
    pub evidence: EnvironmentalEvidenceBundle,
    pub proposal: LivingWatershedProposal,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LivingWatershedPolicyV1;

impl LivingWatershedPolicyV1 {
    /// Stable identity of the v1 policy rules and thresholds.
    pub fn digest(&self) -> Result<TypedDigest32, ContractError> {
        let mut hasher = Sha256::new();
        hasher.update(b"symtropy.basin.environment-policy.living-watershed.v1\0");
        hasher.update(b"rule-order:floodplain-reroute,riparian-planting,observe\0");
        for threshold in [
            FLOOD_SURFACE_WATER_M,
            FLOOD_MAX_SLOPE,
            FLOOD_MIN_FLOW_ACCUMULATION,
            RIPARIAN_MIN_SURFACE_WATER_M,
            RIPARIAN_MAX_SURFACE_WATER_M,
            RIPARIAN_MAX_SLOPE,
            RIPARIAN_MAX_SALINITY,
            RIPARIAN_MIN_TEMPERATURE_K,
            RIPARIAN_MAX_TEMPERATURE_K,
        ] {
            hasher.update(threshold.to_bits().to_le_bytes());
        }
        TypedDigest32::new(
            LIVING_WATERSHED_POLICY_DOMAIN,
            DigestAlgorithm::Sha256,
            LIVING_WATERSHED_POLICY_SCHEMA_VERSION,
            hasher.finalize().into(),
        )
    }

    pub fn evaluate(
        &self,
        cell: &PlanetCellAuthorityView,
    ) -> Result<LivingWatershedEvaluation, LivingWatershedPolicyError> {
        let evidence = EnvironmentalEvidenceBundle::exact_from_cell(cell)
            .map_err(LivingWatershedPolicyError::Evidence)?;
        let terrain = cell
            .terrain
            .as_ref()
            .ok_or(LivingWatershedPolicyError::MissingTerrain)?;
        let hydrology = cell
            .hydrology
            .as_ref()
            .ok_or(LivingWatershedPolicyError::MissingHydrology)?;

        validate_finite_nonnegative("terrain.slope", terrain.value.slope)?;
        validate_finite("terrain.elevation_m", terrain.value.elevation_m)?;
        validate_finite_nonnegative(
            "hydrology.surface_water_m",
            hydrology.value.surface_water_m,
        )?;
        validate_finite("hydrology.groundwater_m", hydrology.value.groundwater_m)?;
        validate_finite_nonnegative(
            "hydrology.flow_accumulation",
            hydrology.value.flow_accumulation,
        )?;
        validate_unit("hydrology.salinity", hydrology.value.salinity)?;

        if let Some(climate) = &cell.climate {
            validate_positive("climate.temperature_k", climate.value.temperature_k)?;
            validate_positive(
                "climate.atmosphere_pressure_pa",
                climate.value.atmosphere_pressure_pa,
            )?;
        }

        let floodplain = hydrology.value.surface_water_m >= FLOOD_SURFACE_WATER_M
            && terrain.value.slope <= FLOOD_MAX_SLOPE
            && hydrology.value.flow_accumulation >= FLOOD_MIN_FLOW_ACCUMULATION;
        if floodplain {
            return Ok(LivingWatershedEvaluation {
                evidence,
                proposal: LivingWatershedProposal {
                    intervention: Some(BasinIntervention::EcologicalReroute),
                    reason: LivingWatershedReason::FloodplainPonding,
                },
            });
        }

        let hydrologically_riparian = hydrology.value.surface_water_m
            >= RIPARIAN_MIN_SURFACE_WATER_M
            && hydrology.value.surface_water_m <= RIPARIAN_MAX_SURFACE_WATER_M
            && terrain.value.slope <= RIPARIAN_MAX_SLOPE
            && hydrology.value.salinity <= RIPARIAN_MAX_SALINITY;

        if hydrologically_riparian {
            let Some(climate) = &cell.climate else {
                return Ok(LivingWatershedEvaluation {
                    evidence,
                    proposal: LivingWatershedProposal {
                        intervention: None,
                        reason: LivingWatershedReason::MissingClimateForRiparianDecision,
                    },
                });
            };
            let temperature = climate.value.temperature_k;
            if (RIPARIAN_MIN_TEMPERATURE_K..=RIPARIAN_MAX_TEMPERATURE_K)
                .contains(&temperature)
            {
                return Ok(LivingWatershedEvaluation {
                    evidence,
                    proposal: LivingWatershedProposal {
                        intervention: Some(BasinIntervention::WillowPlanting),
                        reason: LivingWatershedReason::RiparianRestorationWindow,
                    },
                });
            }
        }

        Ok(LivingWatershedEvaluation {
            evidence,
            proposal: LivingWatershedProposal {
                intervention: None,
                reason: LivingWatershedReason::Observe,
            },
        })
    }

    /// Mint evidence after an owning executor has evaluated/applied the proposal.
    /// This method receives already-produced before/after Basin identities and
    /// therefore cannot mutate Basin itself.
    pub fn receipt_after_execution(
        &self,
        basin_authority: AuthorityId,
        evaluation: &LivingWatershedEvaluation,
        prior_basin_state: TypedDigest32,
        resulting_basin_state: TypedDigest32,
        causal_parents: Vec<TypedDigest32>,
    ) -> Result<BasinEnvironmentalIngestReceipt, LivingWatershedPolicyError> {
        let policy = self.digest().map_err(LivingWatershedPolicyError::Contract)?;
        BasinEnvironmentalIngestReceipt::new(
            basin_authority,
            &evaluation.evidence,
            prior_basin_state,
            policy,
            resulting_basin_state,
            causal_parents,
        )
        .map_err(LivingWatershedPolicyError::Receipt)
    }
}

fn validate_finite(name: &'static str, value: f32) -> Result<(), LivingWatershedPolicyError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(LivingWatershedPolicyError::NonFinite { name, value_bits: value.to_bits() })
    }
}

fn validate_finite_nonnegative(
    name: &'static str,
    value: f32,
) -> Result<(), LivingWatershedPolicyError> {
    validate_finite(name, value)?;
    if value >= 0.0 {
        Ok(())
    } else {
        Err(LivingWatershedPolicyError::Negative { name, value })
    }
}

fn validate_positive(name: &'static str, value: f32) -> Result<(), LivingWatershedPolicyError> {
    validate_finite(name, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(LivingWatershedPolicyError::NonPositive { name, value })
    }
}

fn validate_unit(name: &'static str, value: f32) -> Result<(), LivingWatershedPolicyError> {
    validate_finite(name, value)?;
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(LivingWatershedPolicyError::OutsideUnitInterval { name, value })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LivingWatershedPolicyError {
    Evidence(EnvironmentalEvidenceError),
    Receipt(BasinEnvironmentalIngestError),
    Contract(ContractError),
    MissingTerrain,
    MissingHydrology,
    NonFinite { name: &'static str, value_bits: u32 },
    Negative { name: &'static str, value: f32 },
    NonPositive { name: &'static str, value: f32 },
    OutsideUnitInterval { name: &'static str, value: f32 },
}

impl fmt::Display for LivingWatershedPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evidence(error) => write!(formatter, "living watershed evidence error: {error}"),
            Self::Receipt(error) => write!(formatter, "living watershed receipt error: {error}"),
            Self::Contract(error) => write!(formatter, "living watershed contract error: {error}"),
            Self::MissingTerrain => write!(formatter, "living watershed v1 requires terrain evidence"),
            Self::MissingHydrology => write!(formatter, "living watershed v1 requires hydrology evidence"),
            Self::NonFinite { name, value_bits } => write!(
                formatter,
                "living watershed input {name} is non-finite (bits=0x{value_bits:08x})"
            ),
            Self::Negative { name, value } => write!(formatter, "living watershed input {name} is negative: {value}"),
            Self::NonPositive { name, value } => write!(formatter, "living watershed input {name} must be positive: {value}"),
            Self::OutsideUnitInterval { name, value } => write!(
                formatter,
                "living watershed input {name} must be in [0,1]: {value}"
            ),
        }
    }
}

impl Error for LivingWatershedPolicyError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BasinCausalStateIdentity, BasinIngestEffect, BodyCellIdentity, BodyId, ClimateCellSummary,
        DerivedDomainView, GridSystem, HexCellId, HydrologyCellSummary, TerrainCellSummary,
    };
    use symtropy_basin::BasinWorld;
    use symtropy_sim_contracts::{ReferenceFrameId, RepresentationId, SimInstant};

    fn identity() -> BodyCellIdentity {
        BodyCellIdentity {
            id: HexCellId::new(
                BodyId::earth(),
                GridSystem::BodyIcosahedral,
                7,
                "living-watershed-7",
            ),
            center_lat_deg: -26.2,
            center_lon_deg: 28.0,
            area_m2: 100.0,
        }
    }

    fn view(
        slope: f32,
        surface_water_m: f32,
        flow_accumulation: f32,
        salinity: f32,
        temperature_k: Option<f32>,
    ) -> PlanetCellAuthorityView {
        let identity = identity();
        let scope = identity.scope_id().unwrap();
        let at = SimInstant::new(1_000, 0).unwrap();
        let frame = ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap();
        let terrain = crate::DerivedDomainView::new(
            AuthorityId::parse("terrain.authority.v1").unwrap(),
            scope.clone(),
            frame.clone(),
            RepresentationId::parse("terrain.local.v1").unwrap(),
            at,
            TypedDigest32::sha256("terrain.state.v1", 1, b"terrain").unwrap(),
            TerrainCellSummary {
                elevation_m: 100.0,
                slope,
            },
        )
        .unwrap();
        let hydrology = DerivedDomainView::new(
            AuthorityId::parse("hydrology.authority.v1").unwrap(),
            scope.clone(),
            frame.clone(),
            RepresentationId::parse("hydrology.local.v1").unwrap(),
            at,
            TypedDigest32::sha256("hydrology.state.v1", 1, b"hydrology").unwrap(),
            HydrologyCellSummary {
                surface_water_m,
                groundwater_m: 1.0,
                flow_accumulation,
                salinity,
            },
        )
        .unwrap();

        let mut cell = PlanetCellAuthorityView {
            identity,
            terrain: None,
            hydrology: None,
            climate: None,
            ecology: None,
        }
        .with_terrain(terrain)
        .unwrap()
        .with_hydrology(hydrology)
        .unwrap();

        if let Some(temperature_k) = temperature_k {
            let climate = DerivedDomainView::new(
                AuthorityId::parse("climate.authority.v1").unwrap(),
                scope,
                frame,
                RepresentationId::parse("climate.local.v1").unwrap(),
                at,
                TypedDigest32::sha256("climate.state.v1", 1, b"climate").unwrap(),
                ClimateCellSummary {
                    temperature_k,
                    atmosphere_pressure_pa: 100_000.0,
                },
            )
            .unwrap();
            cell = cell.with_climate(climate).unwrap();
        }
        cell
    }

    #[test]
    fn floodplain_conditions_propose_ecological_reroute() {
        let evaluation = LivingWatershedPolicyV1
            .evaluate(&view(0.05, 1.0, 3.0, 0.02, None))
            .unwrap();
        assert_eq!(
            evaluation.proposal,
            LivingWatershedProposal {
                intervention: Some(BasinIntervention::EcologicalReroute),
                reason: LivingWatershedReason::FloodplainPonding,
            }
        );
    }

    #[test]
    fn temperate_low_salinity_riparian_window_proposes_planting() {
        let evaluation = LivingWatershedPolicyV1
            .evaluate(&view(0.10, 0.25, 0.5, 0.03, Some(291.0)))
            .unwrap();
        assert_eq!(evaluation.proposal.intervention, Some(BasinIntervention::WillowPlanting));
        assert_eq!(evaluation.proposal.reason, LivingWatershedReason::RiparianRestorationWindow);
    }

    #[test]
    fn riparian_candidate_without_climate_is_observation_only() {
        let evaluation = LivingWatershedPolicyV1
            .evaluate(&view(0.10, 0.25, 0.5, 0.03, None))
            .unwrap();
        assert_eq!(evaluation.proposal.intervention, None);
        assert_eq!(
            evaluation.proposal.reason,
            LivingWatershedReason::MissingClimateForRiparianDecision
        );
    }

    #[test]
    fn out_of_policy_conditions_do_not_invent_an_intervention() {
        let evaluation = LivingWatershedPolicyV1
            .evaluate(&view(0.50, 0.02, 0.1, 0.20, Some(310.0)))
            .unwrap();
        assert_eq!(evaluation.proposal.intervention, None);
        assert_eq!(evaluation.proposal.reason, LivingWatershedReason::Observe);
    }

    #[test]
    fn invalid_physical_inputs_fail_closed() {
        let error = LivingWatershedPolicyV1
            .evaluate(&view(0.10, f32::NAN, 0.5, 0.03, Some(291.0)))
            .unwrap_err();
        assert!(matches!(error, LivingWatershedPolicyError::NonFinite { .. }));
    }

    #[test]
    fn policy_digest_is_stable_and_namespaced_for_v07_receipts() {
        let policy = LivingWatershedPolicyV1;
        let digest = policy.digest().unwrap();
        assert_eq!(digest, policy.digest().unwrap());
        assert_eq!(digest.domain, LIVING_WATERSHED_POLICY_DOMAIN);
        assert_eq!(digest.schema_version, LIVING_WATERSHED_POLICY_SCHEMA_VERSION);
    }

    #[test]
    fn flood_proposal_can_be_executed_by_owner_and_proven_afterward() {
        let policy = LivingWatershedPolicyV1;
        let evaluation = policy
            .evaluate(&view(0.05, 1.0, 3.0, 0.02, None))
            .unwrap();
        let mut basin = BasinWorld::old_waterworks(8, 5);
        let prior = basin.causal_state_digest().unwrap();

        let intervention = evaluation.proposal.intervention.unwrap();
        basin.apply(intervention);
        let resulting = basin.causal_state_digest().unwrap();

        let receipt = policy
            .receipt_after_execution(
                AuthorityId::parse("basin.authority.v1").unwrap(),
                &evaluation,
                prior,
                resulting,
                vec![],
            )
            .unwrap();
        assert_eq!(receipt.effect(), BasinIngestEffect::StateChanged);
        assert_eq!(
            receipt.transformation_policy.domain,
            LIVING_WATERSHED_POLICY_DOMAIN
        );
    }

    #[test]
    fn no_action_evaluation_can_be_receipted_without_fake_mutation() {
        let policy = LivingWatershedPolicyV1;
        let evaluation = policy
            .evaluate(&view(0.50, 0.02, 0.1, 0.20, Some(310.0)))
            .unwrap();
        let basin = BasinWorld::old_waterworks(8, 5);
        let state = basin.causal_state_digest().unwrap();
        let receipt = policy
            .receipt_after_execution(
                AuthorityId::parse("basin.authority.v1").unwrap(),
                &evaluation,
                state.clone(),
                state,
                vec![],
            )
            .unwrap();
        assert_eq!(receipt.effect(), BasinIngestEffect::StateUnchanged);
    }
}
