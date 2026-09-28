//! Deterministic, simulation-scoped economic events.
//!
//! This module is deliberately narrower than a ledger. It records what a
//! declared Symtropy simulation says happened, with enough replay identity
//! and provenance to prevent adapters from silently widening that claim.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimulationQuantity {
    pub value: i128,
    pub unit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EconomicEventKind {
    ResourceProduced,
    ResourceConsumed,
    ServicePerformed,
    AssetTransferred,
    OfferCreated,
    OfferAccepted,
    DeliveryCompleted,
    MarketCleared,
    RepairPerformed,
    OutcomeObserved,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SimulationValidity {
    Valid,
    Superseded,
    Corrected,
    Conflicting,
    Indeterminate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayFingerprint {
    pub algorithm: String,
    pub digest_hex: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EconomicEventEnvelopeV1 {
    pub schema_version: String,
    pub envelope_id: String,
    pub world_id: String,
    pub episode_id: String,
    pub simulation_build_id: String,
    pub deterministic_replay_id: String,
    pub tick: u64,
    pub source_event_id: String,
    pub source_event_type: EconomicEventKind,
    pub actor_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub quantities_and_units: Vec<SimulationQuantity>,
    pub causal_parent_refs: Vec<String>,
    pub observed_at_simulation_time: u64,
    pub emitted_at_wall_time: Option<String>,
    pub origin: String,
    pub evidence_refs: Vec<String>,
    pub validity: SimulationValidity,
    pub correction_lineage: Vec<String>,
    pub replay_fingerprint: ReplayFingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomicEventValidationError {
    Empty(&'static str),
    FingerprintMismatch,
    DuplicateIdentityMutation,
    InvalidCorrectionLineage,
}

impl fmt::Display for EconomicEventValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty(field) => write!(f, "required field is empty: {field}"),
            Self::FingerprintMismatch => write!(f, "replay fingerprint does not match event"),
            Self::DuplicateIdentityMutation => write!(f, "event identity fields changed under the same source identity"),
            Self::InvalidCorrectionLineage => write!(f, "corrected events must reference their predecessor"),
        }
    }
}

impl std::error::Error for EconomicEventValidationError {}

impl EconomicEventEnvelopeV1 {
    pub fn new(
        world_id: impl Into<String>,
        episode_id: impl Into<String>,
        simulation_build_id: impl Into<String>,
        deterministic_replay_id: impl Into<String>,
        tick: u64,
        source_event_id: impl Into<String>,
        source_event_type: EconomicEventKind,
        origin: impl Into<String>,
    ) -> Self {
        let mut event = Self {
            schema_version: "EconomicEventEnvelopeV1".into(),
            envelope_id: String::new(),
            world_id: world_id.into(),
            episode_id: episode_id.into(),
            simulation_build_id: simulation_build_id.into(),
            deterministic_replay_id: deterministic_replay_id.into(),
            tick,
            source_event_id: source_event_id.into(),
            source_event_type,
            actor_refs: Vec::new(),
            resource_refs: Vec::new(),
            quantities_and_units: Vec::new(),
            causal_parent_refs: Vec::new(),
            observed_at_simulation_time: tick,
            emitted_at_wall_time: None,
            origin: origin.into(),
            evidence_refs: Vec::new(),
            validity: SimulationValidity::Valid,
            correction_lineage: Vec::new(),
            replay_fingerprint: ReplayFingerprint {
                algorithm: "SHA-256".into(),
                digest_hex: String::new(),
            },
        };
        event.envelope_id = event.stable_identity();
        event.replay_fingerprint.digest_hex = event.compute_replay_digest();
        event
    }

    /// Identity excludes wall-clock emission time and derived fingerprints.
    /// Thus the same deterministic replay produces the same identity even if
    /// serialized at different wall-clock times.
    pub fn stable_identity(&self) -> String {
        let canonical = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.schema_version,
            self.world_id,
            self.episode_id,
            self.simulation_build_id,
            self.deterministic_replay_id,
            self.tick,
            self.source_event_id,
        );
        sha256_hex(canonical.as_bytes())
    }

    pub fn compute_replay_digest(&self) -> String {
        let mut canonical = String::new();
        canonical.push_str(&self.stable_identity());
        canonical.push('|');
        canonical.push_str(&format!("{:?}", self.source_event_type));
        canonical.push('|');
        canonical.push_str(&self.actor_refs.join(","));
        canonical.push('|');
        canonical.push_str(&self.resource_refs.join(","));
        canonical.push('|');
        for q in &self.quantities_and_units {
            canonical.push_str(&q.value.to_string());
            canonical.push(':');
            canonical.push_str(&q.unit);
            canonical.push(';');
        }
        canonical.push('|');
        canonical.push_str(&self.causal_parent_refs.join(","));
        sha256_hex(canonical.as_bytes())
    }

    pub fn refresh_fingerprint(&mut self) {
        self.replay_fingerprint.digest_hex = self.compute_replay_digest();
    }

    pub fn validate(&self) -> Result<(), EconomicEventValidationError> {
        for (name, value) in [
            ("schema_version", self.schema_version.as_str()),
            ("world_id", self.world_id.as_str()),
            ("episode_id", self.episode_id.as_str()),
            ("simulation_build_id", self.simulation_build_id.as_str()),
            ("deterministic_replay_id", self.deterministic_replay_id.as_str()),
            ("source_event_id", self.source_event_id.as_str()),
            ("origin", self.origin.as_str()),
        ] {
            if value.is_empty() {
                return Err(EconomicEventValidationError::Empty(name));
            }
        }
        if self.envelope_id != self.stable_identity() {
            return Err(EconomicEventValidationError::DuplicateIdentityMutation);
        }
        if self.replay_fingerprint.digest_hex != self.compute_replay_digest() {
            return Err(EconomicEventValidationError::FingerprintMismatch);
        }
        if matches!(self.validity, SimulationValidity::Corrected)
            && self.correction_lineage.is_empty()
        {
            return Err(EconomicEventValidationError::InvalidCorrectionLineage);
        }
        Ok(())
    }

    /// A projection may recognize this event, but it must not replace the
    /// simulation origin or turn the event into physical/legal evidence.
    pub fn simulation_claim_ceiling(&self) -> &'static str {
        "simulation-scoped event within the declared world/episode/build/replay"
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> EconomicEventEnvelopeV1 {
        let mut e = EconomicEventEnvelopeV1::new(
            "world-a", "episode-1", "build-7", "replay-7", 42,
            "evt-1", EconomicEventKind::ResourceProduced, "symtropy:world-a",
        );
        e.actor_refs.push("agent-1".into());
        e.resource_refs.push("ore".into());
        e.quantities_and_units.push(SimulationQuantity { value: 5, unit: "kg".into() });
        e.refresh_fingerprint();
        e
    }

    #[test]
    fn deterministic_replay_has_stable_identity() {
        let a = event();
        let b = event();
        assert_eq!(a.envelope_id, b.envelope_id);
        assert_eq!(a.replay_fingerprint, b.replay_fingerprint);
        assert!(a.validate().is_ok());
    }

    #[test]
    fn wall_clock_does_not_change_replay_identity() {
        let mut a = event();
        let original = a.envelope_id.clone();
        a.emitted_at_wall_time = Some("2026-09-28T00:00:00Z".into());
        a.refresh_fingerprint();
        assert_eq!(a.envelope_id, original);
        assert!(a.validate().is_ok());
    }

    #[test]
    fn resource_quantity_mutation_is_detected() {
        let mut a = event();
        a.quantities_and_units[0].value = 6;
        assert_eq!(a.validate(), Err(EconomicEventValidationError::FingerprintMismatch));
    }

    #[test]
    fn source_identity_mutation_is_detected() {
        let mut a = event();
        a.source_event_id = "evt-2".into();
        assert_eq!(a.validate(), Err(EconomicEventValidationError::DuplicateIdentityMutation));
    }

    #[test]
    fn corrected_event_requires_lineage() {
        let mut a = event();
        a.validity = SimulationValidity::Corrected;
        a.refresh_fingerprint();
        assert_eq!(a.validate(), Err(EconomicEventValidationError::InvalidCorrectionLineage));
        a.correction_lineage.push("evt-0".into());
        assert!(a.validate().is_ok());
    }

    #[test]
    fn simulation_claim_does_not_widen() {
        assert_eq!(
            event().simulation_claim_ceiling(),
            "simulation-scoped event within the declared world/episode/build/replay"
        );
    }
}
