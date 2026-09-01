// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Read-only canonical identity for complete Basin causal state.
//!
//! This module deliberately depends on the Basin authority rather than making
//! Basin depend on world orchestration. It hashes the complete public state
//! surface needed to determine future Basin evolution and never mutates it.

use std::{error::Error, fmt};

use sha2::{Digest, Sha256};
use symtropy_basin::{
    AgencyStructure, BasinCell, BasinWorld, ClaimJustification, ClaimType, ChronicleEvidence,
    EcoCivicClaim, MetabolicFlux, SignalField, SignalKind, SignalSource, TrophicMemory,
    ViabilityProfile,
};
use symtropy_lifesim_core::FieldLayer;
use symtropy_sim_contracts::{ContractError, DigestAlgorithm, TypedDigest32};

pub const BASIN_STATE_SCHEMA_VERSION: u32 = 1;
pub const BASIN_STATE_DIGEST_DOMAIN: &str = "symtropy.basin.state.v1";
const CANONICAL_NAN_F32_BITS: u32 = 0x7fc0_0000;

const FIELD_LAYER_ORDER: [FieldLayer; FieldLayer::COUNT] = [
    FieldLayer::FoodPheromone,
    FieldLayer::HomePheromone,
    FieldLayer::DangerPheromone,
    FieldLayer::Moisture,
    FieldLayer::Obstacle,
    FieldLayer::Nutrient,
    FieldLayer::Toxin,
    FieldLayer::Biomass,
    FieldLayer::Heat,
    FieldLayer::Light,
    FieldLayer::Oxygen,
    FieldLayer::Disease,
    FieldLayer::SignalNoise,
    FieldLayer::NullContamination,
];

/// Read-only extension implemented for the Basin authority.
pub trait BasinCausalStateIdentity {
    fn causal_state_digest(&self) -> Result<TypedDigest32, BasinStateDigestError>;
}

impl BasinCausalStateIdentity for BasinWorld {
    fn causal_state_digest(&self) -> Result<TypedDigest32, BasinStateDigestError> {
        let width = canonical_len("width", self.width())?;
        let height = canonical_len("height", self.height())?;
        let cell_count = self
            .width()
            .checked_mul(self.height())
            .ok_or(BasinStateDigestError::CellCountOverflow)?;

        if self.fields.width() != self.width() || self.fields.height() != self.height() {
            return Err(BasinStateDigestError::FieldGridShapeMismatch {
                basin_width: self.width(),
                basin_height: self.height(),
                field_width: self.fields.width(),
                field_height: self.fields.height(),
            });
        }

        let mut hasher = Sha256::new();
        hasher.update(b"symtropy.basin.causal-state.v1\0");
        hash_u32(&mut hasher, BASIN_STATE_SCHEMA_VERSION);
        hash_u64(&mut hasher, width);
        hash_u64(&mut hasher, height);
        hash_u64(&mut hasher, self.tick());

        hasher.update(b"cells\0");
        hash_u64(&mut hasher, canonical_len("cells", cell_count)?);
        for y in 0..self.height() {
            for x in 0..self.width() {
                hash_cell(&mut hasher, self.cell(x, y));
            }
        }

        hasher.update(b"fields\0");
        hash_u64(
            &mut hasher,
            canonical_len("field-layers", FIELD_LAYER_ORDER.len())?,
        );
        for layer in FIELD_LAYER_ORDER {
            hash_u8(&mut hasher, field_layer_code(layer));
            for y in 0..self.height() {
                for x in 0..self.width() {
                    hash_f32(&mut hasher, self.fields.get(layer, x, y));
                }
            }
        }

        hasher.update(b"flux\0");
        hash_flux(&mut hasher, self.flux);

        hasher.update(b"memory\0");
        hash_memory(&mut hasher, self.memory);

        hasher.update(b"viability\0");
        hash_viability(&mut hasher, self.viability);

        hasher.update(b"signals\0");
        hash_u64(
            &mut hasher,
            canonical_len("signals", self.signals.len())?,
        );
        for signal in &self.signals {
            hash_signal(&mut hasher, signal)?;
        }

        hasher.update(b"civic-claims\0");
        hash_u64(
            &mut hasher,
            canonical_len("civic-claims", self.civic_claims.len())?,
        );
        for claim in &self.civic_claims {
            hash_claim(&mut hasher, claim)?;
        }

        TypedDigest32::new(
            BASIN_STATE_DIGEST_DOMAIN,
            DigestAlgorithm::Sha256,
            BASIN_STATE_SCHEMA_VERSION,
            hasher.finalize().into(),
        )
        .map_err(BasinStateDigestError::Contract)
    }
}

fn canonical_len(kind: &'static str, value: usize) -> Result<u64, BasinStateDigestError> {
    u64::try_from(value).map_err(|_| BasinStateDigestError::LengthOverflow { kind, value })
}

fn canonical_f32_bits(value: f32) -> u32 {
    if value == 0.0 {
        0
    } else if value.is_nan() {
        CANONICAL_NAN_F32_BITS
    } else {
        value.to_bits()
    }
}

fn hash_u8(hasher: &mut Sha256, value: u8) {
    hasher.update([value]);
}

fn hash_u32(hasher: &mut Sha256, value: u32) {
    hasher.update(value.to_le_bytes());
}

fn hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_le_bytes());
}

fn hash_f32(hasher: &mut Sha256, value: f32) {
    hash_u32(hasher, canonical_f32_bits(value));
}

fn hash_cell(hasher: &mut Sha256, cell: BasinCell) {
    hash_f32(hasher, cell.water.standing);
    hash_f32(hasher, cell.water.flow);
    hash_f32(hasher, cell.water.pressure);
    hash_f32(hasher, cell.water.trust);

    hash_f32(hasher, cell.soil.moisture);
    hash_f32(hasher, cell.soil.carbon);
    hash_f32(hasher, cell.soil.compaction);
    hash_f32(hasher, cell.soil.decomposer_capacity);

    hash_f32(hasher, cell.atmosphere.humidity);
    hash_f32(hasher, cell.atmosphere.particulates);

    hash_f32(hasher, cell.heat.temperature_c);
    hash_f32(hasher, cell.heat.heat_stress);

    hash_f32(hasher, cell.toxin_load.dissolved_metals);
    hash_f32(hasher, cell.toxin_load.hydrocarbons);
    hash_f32(hasher, cell.toxin_load.bioavailable);

    hash_f32(hasher, cell.radiation.background);
    hash_f32(hasher, cell.radiation.anomaly);

    hash_f32(hasher, cell.salinity);
    hash_f32(hasher, cell.erosion_risk);
    hash_f32(hasher, cell.root_intrusion);
    hash_f32(hasher, cell.infrastructure_integrity);
}

fn hash_flux(hasher: &mut Sha256, flux: MetabolicFlux) {
    hash_f32(hasher, flux.water);
    hash_f32(hasher, flux.nutrient);
    hash_f32(hasher, flux.carbon);
    hash_f32(hasher, flux.toxin);
    hash_f32(hasher, flux.heat);
    hash_f32(hasher, flux.biomass);
    hash_f32(hasher, flux.waste);
    hash_f32(hasher, flux.disease);
    hash_f32(hasher, flux.signal);
    hash_f32(hasher, flux.labor);
}

fn hash_memory(hasher: &mut Sha256, memory: TrophicMemory) {
    hash_f32(hasher, memory.producer_health);
    hash_f32(hasher, memory.decomposer_capacity);
    hash_f32(hasher, memory.pollination_reliability);
    hash_f32(hasher, memory.grazer_pressure);
    hash_f32(hasher, memory.predator_regulation);
    hash_f32(hasher, memory.scavenger_efficiency);
    hash_f32(hasher, memory.disease_suppression);
    hash_f32(hasher, memory.soil_engineering);
    hash_f32(hasher, memory.extinction_debt);
    hash_f32(hasher, memory.invasive_pressure);
    hash_f32(hasher, memory.recovery_momentum);
}

fn hash_viability(hasher: &mut Sha256, viability: ViabilityProfile) {
    hash_u32(hasher, viability.entity_id.0);
    hash_u8(hasher, agency_structure_code(viability.agency_structure));
    hash_f32(hasher, viability.viability);
    hash_f32(hasher, viability.boundary_integrity);
    hash_f32(hasher, viability.metabolic_stability);
    hash_f32(hasher, viability.signal_coherence);
    hash_f32(hasher, viability.reproductive_continuity);
    hash_f32(hasher, viability.habitat_continuity);
    hash_f32(hasher, viability.harm_perception);
    hash_f32(hasher, viability.reciprocity_trust);
    hash_f32(hasher, viability.cohabitation_possibility);
}

fn hash_signal(
    hasher: &mut Sha256,
    signal: &SignalField,
) -> Result<(), BasinStateDigestError> {
    hash_u8(hasher, signal_kind_code(signal.kind));
    hash_f32(hasher, signal.intensity);
    hash_f32(hasher, signal.decay_rate);
    hash_f32(hasher, signal.diffusion_rate);
    hash_u8(hasher, signal_source_code(signal.source));
    hash_u64(
        hasher,
        canonical_len("signal.readable-by", signal.readable_by.len())?,
    );
    for system in &signal.readable_by {
        hash_u32(hasher, system.0);
    }
    hash_f32(hasher, signal.corruption);
    Ok(())
}

fn hash_claim(
    hasher: &mut Sha256,
    claim: &EcoCivicClaim,
) -> Result<(), BasinStateDigestError> {
    hash_u32(hasher, claim.claimant.0);
    hash_u32(hasher, claim.target.0);
    hash_u8(hasher, claim_type_code(claim.claim_type));
    hash_u8(hasher, claim_justification_code(claim.justification));
    hash_u64(
        hasher,
        canonical_len("claim.evidence", claim.evidence.len())?,
    );
    for evidence in &claim.evidence {
        hash_u8(hasher, chronicle_evidence_code(*evidence));
    }
    hash_f32(hasher, claim.legitimacy);
    hash_u64(
        hasher,
        canonical_len("claim.opposition", claim.opposition.len())?,
    );
    for faction in &claim.opposition {
        hash_u32(hasher, faction.0);
    }
    Ok(())
}

const fn field_layer_code(layer: FieldLayer) -> u8 {
    match layer {
        FieldLayer::FoodPheromone => 0,
        FieldLayer::HomePheromone => 1,
        FieldLayer::DangerPheromone => 2,
        FieldLayer::Moisture => 3,
        FieldLayer::Obstacle => 4,
        FieldLayer::Nutrient => 5,
        FieldLayer::Toxin => 6,
        FieldLayer::Biomass => 7,
        FieldLayer::Heat => 8,
        FieldLayer::Light => 9,
        FieldLayer::Oxygen => 10,
        FieldLayer::Disease => 11,
        FieldLayer::SignalNoise => 12,
        FieldLayer::NullContamination => 13,
    }
}

const fn agency_structure_code(value: AgencyStructure) -> u8 {
    match value {
        AgencyStructure::Colony => 0,
        AgencyStructure::MycelialNetwork => 1,
        AgencyStructure::Wetland => 2,
        AgencyStructure::Settlement => 3,
        AgencyStructure::MachineEcology => 4,
    }
}

const fn signal_kind_code(value: SignalKind) -> u8 {
    match value {
        SignalKind::Pheromone => 0,
        SignalKind::RootExudate => 1,
        SignalKind::FungalPulse => 2,
        SignalKind::BirdAlarm => 3,
        SignalKind::WaterTurbidity => 4,
        SignalKind::MachineDiagnostic => 5,
        SignalKind::Vibration => 6,
        SignalKind::ChemicalGradient => 7,
        SignalKind::FieldDeckScan => 8,
        SignalKind::NullChatter => 9,
    }
}

const fn signal_source_code(value: SignalSource) -> u8 {
    match value {
        SignalSource::Basin => 0,
        SignalSource::Colony => 1,
        SignalSource::Mycelium => 2,
        SignalSource::Settlement => 3,
        SignalSource::DeviceBus => 4,
        SignalSource::NullSystem => 5,
    }
}

const fn claim_type_code(value: ClaimType) -> u8 {
    match value {
        ClaimType::ProtectAsLivingInfrastructure => 0,
        ClaimType::AuthorizeMechanicalRepair => 1,
        ClaimType::RequireEvidenceReview => 2,
        ClaimType::QuarantineRisk => 3,
    }
}

const fn claim_justification_code(value: ClaimJustification) -> u8 {
    match value {
        ClaimJustification::FloodBuffering => 0,
        ClaimJustification::WaterSecurity => 1,
        ClaimJustification::ToxinCapture => 2,
        ClaimJustification::Biosecurity => 3,
        ClaimJustification::BoundaryIntegrity => 4,
    }
}

const fn chronicle_evidence_code(value: ChronicleEvidence) -> u8 {
    match value {
        ChronicleEvidence::PipeLeakDetected => 0,
        ChronicleEvidence::ToxinTrend => 1,
        ChronicleEvidence::RootIntrusion => 2,
        ChronicleEvidence::SignalCorruption => 3,
        ChronicleEvidence::RecoveryTrajectory => 4,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BasinStateDigestError {
    Contract(ContractError),
    LengthOverflow {
        kind: &'static str,
        value: usize,
    },
    CellCountOverflow,
    FieldGridShapeMismatch {
        basin_width: usize,
        basin_height: usize,
        field_width: usize,
        field_height: usize,
    },
}

impl fmt::Display for BasinStateDigestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract(error) => write!(formatter, "basin state digest contract error: {error}"),
            Self::LengthOverflow { kind, value } => {
                write!(formatter, "{kind} length {value} does not fit canonical u64 encoding")
            }
            Self::CellCountOverflow => write!(formatter, "basin width * height overflowed usize"),
            Self::FieldGridShapeMismatch {
                basin_width,
                basin_height,
                field_width,
                field_height,
            } => write!(
                formatter,
                "basin/field shape mismatch: basin {basin_width}x{basin_height}, field {field_width}x{field_height}"
            ),
        }
    }
}

impl Error for BasinStateDigestError {}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_basin::{BasinIntervention, FactionId, SystemId};

    fn digest(world: &BasinWorld) -> TypedDigest32 {
        world.causal_state_digest().unwrap()
    }

    #[test]
    fn identical_worlds_have_identical_digest() {
        let a = BasinWorld::old_waterworks(8, 5);
        let b = a.clone();
        assert_eq!(digest(&a), digest(&b));
    }

    #[test]
    fn intervention_changes_cell_backed_identity() {
        let a = BasinWorld::old_waterworks(8, 5);
        let mut b = a.clone();
        b.apply(BasinIntervention::WillowPlanting);
        assert_ne!(digest(&a), digest(&b));
    }

    #[test]
    fn field_change_changes_identity() {
        let a = BasinWorld::old_waterworks(8, 5);
        let mut b = a.clone();
        b.fields.add(FieldLayer::Oxygen, 0, 0, 0.125);
        assert_ne!(digest(&a), digest(&b));
    }

    #[test]
    fn memory_flux_and_viability_are_authoritative_state() {
        let a = BasinWorld::old_waterworks(8, 5);

        let mut flux = a.clone();
        flux.flux.labor += 0.25;
        assert_ne!(digest(&a), digest(&flux));

        let mut memory = a.clone();
        memory.memory.extinction_debt += 0.01;
        assert_ne!(digest(&a), digest(&memory));

        let mut viability = a.clone();
        viability.viability.reciprocity_trust += 0.01;
        assert_ne!(digest(&a), digest(&viability));
    }

    #[test]
    fn signal_and_civic_sequence_are_part_of_stored_identity() {
        let mut a = BasinWorld::old_waterworks(8, 5);
        a.signals[0].readable_by = vec![SystemId(7), SystemId(9)];
        let mut b = a.clone();
        b.signals[0].readable_by.swap(0, 1);
        assert_ne!(digest(&a), digest(&b));

        let mut c = a.clone();
        c.civic_claims[0].opposition.push(FactionId(99));
        assert_ne!(digest(&a), digest(&c));
    }

    #[test]
    fn step_changes_causal_state_identity() {
        let mut world = BasinWorld::old_waterworks(8, 5);
        let initial = digest(&world);
        world.step();
        assert_ne!(initial, digest(&world));
    }

    #[test]
    fn floating_point_zero_and_nan_payloads_are_canonical() {
        assert_eq!(canonical_f32_bits(0.0), canonical_f32_bits(-0.0));
        let nan_a = f32::from_bits(0x7fc0_0001);
        let nan_b = f32::from_bits(0xffd2_3456);
        assert!(nan_a.is_nan());
        assert!(nan_b.is_nan());
        assert_eq!(canonical_f32_bits(nan_a), canonical_f32_bits(nan_b));
        assert_eq!(canonical_f32_bits(nan_a), CANONICAL_NAN_F32_BITS);
    }

    #[test]
    fn digest_domain_and_schema_are_explicit() {
        let world = BasinWorld::old_waterworks(8, 5);
        let digest = digest(&world);
        assert_eq!(digest.domain, BASIN_STATE_DIGEST_DOMAIN);
        assert_eq!(digest.schema_version, BASIN_STATE_SCHEMA_VERSION);
        assert_eq!(digest.algorithm, DigestAlgorithm::Sha256);
    }
}
