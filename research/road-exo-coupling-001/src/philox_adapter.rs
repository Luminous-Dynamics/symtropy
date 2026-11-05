//! Deterministic semantic-address to Philox4x32-10 resolver.
//!
//! The caller must qualify event identity/coupling before entering this module.
//! This module derives one Philox block per canonical DrawAddress; no sequential
//! stream position or variable-consumption sampling loop is involved.

use std::collections::BTreeMap;

use rand_philox::Philox;

use crate::DrawAddress;
use crate::coupling_projection::{
    CorrespondenceAddressPlan, HypothesisAddressPlan, PlanCompleteness, PlannedAddress,
    PlannedPresence, SharingStatus,
};

pub const ADDRESS_SCHEMA_ID: &str = "symtropy.road-exo-coupling.draw-address.v0.1";
pub const DERIVATION_CONTEXT: &str =
    "Symtropy ROAD-EXO-COUPLING-001 Philox4x32-10 draw-address v0.1";
pub const DERIVATION_ID: &str = "BLAKE3-derive-key-256/road-exo-v0.1";
pub const PRNG_ID: &str = "Philox4x32-10/rand_philox-0.2.1";
pub const PHILOX_OUTPUT_WORDS: usize = 4;

pub const CORRESPONDENCE_ADDRESS_SCHEMA_ID: &str =
    "symtropy.road-correspondence-coupling.address.v0.2";
pub const CORRESPONDENCE_DERIVATION_CONTEXT: &str =
    "Symtropy ROAD-CORRESPONDENCE-COUPLING-001 Philox4x32-10 address v0.2";
pub const CORRESPONDENCE_DERIVATION_ID: &str =
    "BLAKE3-derive-key-256/road-correspondence-coupling-v0.2";

/// Reproducibility receipt for exactly one semantic random draw.
///
/// The 256-bit BLAKE3 derivation digest is retained in full. Its first 192 bits
/// are mapped to the Philox key and counter; the last 64 bits are a collision
/// audit tag and are not fed into Philox.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrawReceipt {
    pub address_schema: &'static str,
    pub derivation_id: &'static str,
    pub prng_id: &'static str,
    pub derivation_digest: [u8; 32],
    pub collision_tag: [u8; 8],
    pub key: [u32; 2],
    pub counter: u128,
    /// Philox block output, in the algorithm's canonical word order.
    pub words: [u32; PHILOX_OUTPUT_WORDS],
}

/// Resolve exactly one Philox block from an experiment seed and a qualified
/// semantic event address.
///
/// The experiment seed is reproducibility material, not a CSPRNG secret.
/// DrawAddress::canonical_bytes carries the world/source/mechanism/event and
/// coupling identity, including branch identity for branch-specific draws.
#[must_use]
pub fn resolve_draw(experiment_seed: &[u8; 32], address: &DrawAddress) -> DrawReceipt {
    let canonical_address = address.canonical_bytes();
    resolve_canonical_address(
        experiment_seed,
        ADDRESS_SCHEMA_ID,
        DERIVATION_ID,
        DERIVATION_CONTEXT,
        &canonical_address,
    )
}

/// Resolve exactly one Philox block from a canonical v0.2 correspondence address.
///
/// The correspondence projection and causal disposition must already have
/// been qualified. This function consumes the unchanged canonical bytes and
/// uses the same pinned BLAKE3 + Philox versions as the v0.1 resolver, but a
/// distinct KDF context/version prevents accidental cross-schema reuse.
#[must_use]
pub fn resolve_correspondence_address(
    experiment_seed: &[u8; 32],
    canonical_address: &[u8],
) -> DrawReceipt {
    resolve_canonical_address(
        experiment_seed,
        CORRESPONDENCE_ADDRESS_SCHEMA_ID,
        CORRESPONDENCE_DERIVATION_ID,
        CORRESPONDENCE_DERIVATION_CONTEXT,
        canonical_address,
    )
}

fn resolve_canonical_address(
    experiment_seed: &[u8; 32],
    address_schema: &'static str,
    derivation_id: &'static str,
    derivation_context: &str,
    canonical_address: &[u8],
) -> DrawReceipt {
    let derivation_digest =
        derive_address_digest_with_context(experiment_seed, canonical_address, derivation_context);

    let key = [
        u32::from_le_bytes(
            derivation_digest[0..4]
                .try_into()
                .expect("fixed four-byte Philox key word"),
        ),
        u32::from_le_bytes(
            derivation_digest[4..8]
                .try_into()
                .expect("fixed four-byte Philox key word"),
        ),
    ];
    let counter = u128::from_le_bytes(
        derivation_digest[8..24]
            .try_into()
            .expect("fixed sixteen-byte Philox counter"),
    );
    let collision_tag = derivation_digest[24..32]
        .try_into()
        .expect("fixed eight-byte collision tag");

    // Exactly one block per semantic address. No carried stream position.
    let mut generator = Philox::new(key, counter);
    let words = std::array::from_fn(|_| generator.next_u32());

    DrawReceipt {
        address_schema,
        derivation_id,
        prng_id: PRNG_ID,
        derivation_digest,
        collision_tag,
        key,
        counter,
        words,
    }
}

fn derive_address_digest_with_context(
    experiment_seed: &[u8; 32],
    address: &[u8],
    context: &str,
) -> [u8; 32] {
    let mut key_material = Vec::with_capacity(8 + experiment_seed.len() + 8 + address.len());
    push_len(&mut key_material, experiment_seed.len());
    key_material.extend_from_slice(experiment_seed);
    push_len(&mut key_material, address.len());
    key_material.extend_from_slice(address);
    blake3::derive_key(context, &key_material)
}

fn push_len(out: &mut Vec<u8>, length: usize) {
    let length = u64::try_from(length).expect("address material length must fit u64");
    out.extend_from_slice(&length.to_le_bytes());
}

/// Receipt validation error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptMismatch;

/// Recompute the entire receipt from the frozen seed and semantic address.
/// This verifies the KDF mapping and all four PRNG output words.
pub fn verify_receipt(
    experiment_seed: &[u8; 32],
    address: &DrawAddress,
    receipt: &DrawReceipt,
) -> Result<(), ReceiptMismatch> {
    if resolve_draw(experiment_seed, address) == *receipt {
        Ok(())
    } else {
        Err(ReceiptMismatch)
    }
}

/// Detects a collision in the 192-bit Philox (key,counter) mapping within one
/// retained run. Reusing one mapping for the same full derivation digest is
/// valid; the same mapping with a different 256-bit digest is rejected.
///
/// This is a run-local detector, not a global collision-free guarantee.
#[derive(Clone, Debug, Default)]
pub struct AddressCollisionRegistry {
    seen: BTreeMap<([u32; 2], u128), [u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressCollision {
    pub key: [u32; 2],
    pub counter: u128,
}

impl AddressCollisionRegistry {
    pub fn register(&mut self, receipt: &DrawReceipt) -> Result<(), AddressCollision> {
        let block_address = (receipt.key, receipt.counter);
        match self.seen.get(&block_address) {
            Some(existing) if existing != &receipt.derivation_digest => Err(AddressCollision {
                key: receipt.key,
                counter: receipt.counter,
            }),
            Some(_) => Ok(()),
            None => {
                self.seen.insert(block_address, receipt.derivation_digest);
                Ok(())
            }
        }
    }

    pub fn len(&self) -> usize {
        self.seen.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

/// One resolved address with its correspondence and presence metadata retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrespondenceDrawReceipt {
    pub hypothesis_id: crate::StableId,
    pub branch_id: crate::StableId,
    pub event_id: crate::StableId,
    pub sharing: SharingStatus,
    pub presence: PlannedPresence,
    /// Exact canonical bytes used for derivation; retained for self-contained replay.
    pub canonical_address: Vec<u8>,
    pub draw: DrawReceipt,
}

/// Resolved draws for every retained correspondence hypothesis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrespondenceHypothesisDraws {
    pub hypothesis_id: crate::StableId,
    pub completeness: PlanCompleteness,
    /// Original exact event map, pairing bases, and correspondence evidence references.
    pub correspondence: crate::correspondence::CorrespondenceHypothesis,
    /// Original causal invariance / branch-locality rules for each mapped pair.
    pub causal_rules: Vec<crate::coupling_projection::PairCausalRule>,
    pub draws: Vec<CorrespondenceDrawReceipt>,
}

/// A complete or explicitly incomplete bundle of all hypothesis-conditioned draws.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrespondenceDrawBundle {
    pub bundle_schema: &'static str,
    pub bundle_digest: [u8; 32],
    pub left_branch: crate::StableId,
    pub right_branch: crate::StableId,
    pub completeness: PlanCompleteness,
    /// Original candidate-universe evidence reference/status from correspondence input.
    pub universe_status: crate::correspondence::CandidateUniverseStatus,
    /// Original hypothesis-search evidence reference/status from correspondence input.
    pub hypothesis_set_status: crate::correspondence::HypothesisSetStatus,
    pub hypotheses: Vec<CorrespondenceHypothesisDraws>,
}

pub const CORRESPONDENCE_DRAW_BUNDLE_SCHEMA_ID: &str =
    "symtropy.road-correspondence-coupling.draw-bundle.v0.1";
const CORRESPONDENCE_DRAW_BUNDLE_DERIVATION_CONTEXT: &str =
    "Symtropy ROAD-CORRESPONDENCE-COUPLING-001 draw bundle commitment v0.1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CorrespondenceBundleVerificationError {
    BundleDigestMismatch,
    DrawReceiptMismatch,
}

/// Compute the versioned content digest over the complete draw bundle.
///
/// The digest commits to scope/search evidence references, correspondence maps,
/// justifications, address bytes, presence/sharing labels and raw draw receipts.
/// It is a content commitment, not a signature or proof that source evidence is true.
pub fn correspondence_draw_bundle_digest(bundle: &CorrespondenceDrawBundle) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(CORRESPONDENCE_DRAW_BUNDLE_DERIVATION_CONTEXT);
    hash_string(&mut hasher, bundle.bundle_schema);
    hash_stable_id(&mut hasher, &bundle.left_branch);
    hash_stable_id(&mut hasher, &bundle.right_branch);
    hash_completeness(&mut hasher, bundle.completeness);
    hash_universe_status(&mut hasher, &bundle.universe_status);
    hash_hypothesis_set_status(&mut hasher, &bundle.hypothesis_set_status);
    hash_len(&mut hasher, bundle.hypotheses.len());

    for hypothesis in &bundle.hypotheses {
        hash_stable_id(&mut hasher, &hypothesis.hypothesis_id);
        hash_completeness(&mut hasher, hypothesis.completeness);
        hash_correspondence_hypothesis(&mut hasher, &hypothesis.correspondence);
        hash_len(&mut hasher, hypothesis.causal_rules.len());
        for rule in &hypothesis.causal_rules {
            hash_pair_causal_rule(&mut hasher, rule);
        }
        hash_len(&mut hasher, hypothesis.draws.len());
        for receipt in &hypothesis.draws {
            hash_stable_id(&mut hasher, &receipt.hypothesis_id);
            hash_stable_id(&mut hasher, &receipt.branch_id);
            hash_stable_id(&mut hasher, &receipt.event_id);
            hash_sharing_status(&mut hasher, receipt.sharing);
            hash_presence(&mut hasher, &receipt.presence);
            hash_bytes(&mut hasher, &receipt.canonical_address);
            hash_draw_receipt(&mut hasher, &receipt.draw);
        }
    }

    *hasher.finalize().as_bytes()
}

/// Verify the bundle commitment and independently recompute every Philox receipt.
///
/// A caller that stores the digest in an independently controlled evidence record
/// can detect later changes to either the bundle metadata or its draw outputs.
pub fn verify_correspondence_draw_bundle(
    experiment_seed: &[u8; 32],
    bundle: &CorrespondenceDrawBundle,
) -> Result<(), CorrespondenceBundleVerificationError> {
    if bundle.bundle_schema != CORRESPONDENCE_DRAW_BUNDLE_SCHEMA_ID
        || correspondence_draw_bundle_digest(bundle) != bundle.bundle_digest
    {
        return Err(CorrespondenceBundleVerificationError::BundleDigestMismatch);
    }

    for hypothesis in &bundle.hypotheses {
        for receipt in &hypothesis.draws {
            verify_correspondence_receipt(
                experiment_seed,
                &receipt.canonical_address,
                &receipt.draw,
            )
            .map_err(|_| CorrespondenceBundleVerificationError::DrawReceiptMismatch)?;
        }
    }
    Ok(())
}

fn hash_len(hasher: &mut blake3::Hasher, length: usize) {
    let length = u64::try_from(length).expect("bundle field length must fit u64");
    hasher.update(&length.to_le_bytes());
}

fn hash_bytes(hasher: &mut blake3::Hasher, value: &[u8]) {
    hash_len(hasher, value.len());
    hasher.update(value);
}

fn hash_string(hasher: &mut blake3::Hasher, value: &str) {
    hash_bytes(hasher, value.as_bytes());
}

fn hash_stable_id(hasher: &mut blake3::Hasher, value: &crate::StableId) {
    hash_string(hasher, value.as_str());
}

fn hash_completeness(hasher: &mut blake3::Hasher, value: PlanCompleteness) {
    hasher.update(&[match value {
        PlanCompleteness::CompleteWithinDeclaredSets => 0,
        PlanCompleteness::Incomplete => 1,
    }]);
}

fn hash_universe_status(
    hasher: &mut blake3::Hasher,
    value: &crate::correspondence::CandidateUniverseStatus,
) {
    match value {
        crate::correspondence::CandidateUniverseStatus::ExhaustiveWithinDeclaredScope {
            evidence_ref,
        } => {
            hasher.update(&[0]);
            hash_stable_id(hasher, evidence_ref);
        }
        crate::correspondence::CandidateUniverseStatus::Incomplete { evidence_ref } => {
            hasher.update(&[1]);
            hash_stable_id(hasher, evidence_ref);
        }
    }
}

fn hash_hypothesis_set_status(
    hasher: &mut blake3::Hasher,
    value: &crate::correspondence::HypothesisSetStatus,
) {
    match value {
        crate::correspondence::HypothesisSetStatus::ExhaustiveWithinDeclaredModelFamily {
            evidence_ref,
        } => {
            hasher.update(&[0]);
            hash_stable_id(hasher, evidence_ref);
        }
        crate::correspondence::HypothesisSetStatus::IncompleteOrBudgetLimited { evidence_ref } => {
            hasher.update(&[1]);
            hash_stable_id(hasher, evidence_ref);
        }
    }
}

fn hash_correspondence_hypothesis(
    hasher: &mut blake3::Hasher,
    value: &crate::correspondence::CorrespondenceHypothesis,
) {
    hash_stable_id(hasher, &value.hypothesis_id);
    hash_stable_id(hasher, &value.justification_ref);
    hash_len(hasher, value.pairs.len());
    for pair in &value.pairs {
        hash_stable_id(hasher, &pair.left_event);
        hash_stable_id(hasher, &pair.right_event);
        hasher.update(&[match pair.basis {
            crate::correspondence::IdentityBasis::Actor => 0,
            crate::correspondence::IdentityBasis::DirectedDyad => 1,
            crate::correspondence::IdentityBasis::EncounterSlot => 2,
        }]);
        hash_stable_id(hasher, &pair.justification_ref);
    }

    for unmatched in [&value.unmatched_left, &value.unmatched_right] {
        hash_len(hasher, unmatched.len());
        for event in unmatched {
            hash_stable_id(hasher, &event.event_id);
            match &event.reason {
                crate::correspondence::UnmatchedReason::NoCounterpartUnderHypothesis {
                    rationale_ref,
                } => {
                    hasher.update(&[0]);
                    hash_stable_id(hasher, rationale_ref);
                }
                crate::correspondence::UnmatchedReason::CorrespondenceUnknown { evidence_ref } => {
                    hasher.update(&[1]);
                    hash_stable_id(hasher, evidence_ref);
                }
            }
        }
    }
}

fn hash_pair_causal_rule(
    hasher: &mut blake3::Hasher,
    value: &crate::coupling_projection::PairCausalRule,
) {
    hash_stable_id(hasher, &value.hypothesis_id);
    hash_stable_id(hasher, &value.left_event);
    hash_stable_id(hasher, &value.right_event);
    hasher.update(&[match value.basis {
        crate::correspondence::IdentityBasis::Actor => 0,
        crate::correspondence::IdentityBasis::DirectedDyad => 1,
        crate::correspondence::IdentityBasis::EncounterSlot => 2,
    }]);
    match &value.disposition {
        crate::coupling_projection::CausalDisposition::SharedInvariant {
            equivalence_group,
            justification,
        } => {
            hasher.update(&[0]);
            hash_stable_id(hasher, equivalence_group);
            hash_stable_id(hasher, justification);
        }
        crate::coupling_projection::CausalDisposition::BranchSpecific => {
            hasher.update(&[1]);
        }
        crate::coupling_projection::CausalDisposition::SensitivityOnly {
            sensitivity_id,
            justification,
        } => {
            hasher.update(&[2]);
            hash_stable_id(hasher, sensitivity_id);
            hash_stable_id(hasher, justification);
        }
        crate::coupling_projection::CausalDisposition::NotIdentified => {
            hasher.update(&[3]);
        }
        crate::coupling_projection::CausalDisposition::Incomplete => {
            hasher.update(&[4]);
        }
    }
}

fn hash_sharing_status(hasher: &mut blake3::Hasher, value: SharingStatus) {
    hasher.update(&[match value {
        SharingStatus::SharedInvariant => 0,
        SharingStatus::SensitivityOnly => 1,
        SharingStatus::BranchSpecific => 2,
        SharingStatus::NotIdentified => 3,
        SharingStatus::Incomplete => 4,
        SharingStatus::NoCounterpartUnderHypothesis => 5,
    }]);
}

fn hash_presence(hasher: &mut blake3::Hasher, value: &PlannedPresence) {
    match value {
        PlannedPresence::Observed { evidence_ref } => {
            hasher.update(&[0]);
            hash_stable_id(hasher, evidence_ref);
        }
        PlannedPresence::NotRealized { condition_witness } => {
            hasher.update(&[1]);
            hash_stable_id(hasher, condition_witness);
        }
        PlannedPresence::Unknown { evidence_ref } => {
            hasher.update(&[2]);
            hash_stable_id(hasher, evidence_ref);
        }
    }
}

fn hash_draw_receipt(hasher: &mut blake3::Hasher, value: &DrawReceipt) {
    hash_string(hasher, value.address_schema);
    hash_string(hasher, value.derivation_id);
    hash_string(hasher, value.prng_id);
    hash_bytes(hasher, &value.derivation_digest);
    hash_bytes(hasher, &value.collision_tag);
    for word in value.key {
        hasher.update(&word.to_le_bytes());
    }
    hasher.update(&value.counter.to_le_bytes());
    hash_len(hasher, value.words.len());
    for word in value.words {
        hasher.update(&word.to_le_bytes());
    }
}

/// Resolve every address in every retained
/// collision registry over the whole bundle. Incomplete plans remain incomplete.
pub fn resolve_correspondence_plan(
    experiment_seed: &[u8; 32],
    plan: &CorrespondenceAddressPlan,
) -> Result<CorrespondenceDrawBundle, AddressCollision> {
    let mut registry = AddressCollisionRegistry::default();
    let mut hypotheses = Vec::with_capacity(plan.hypotheses.len());

    for hypothesis in &plan.hypotheses {
        let draws = resolve_hypothesis_addresses(experiment_seed, hypothesis, &mut registry)?;
        hypotheses.push(CorrespondenceHypothesisDraws {
            hypothesis_id: hypothesis.hypothesis_id.clone(),
            completeness: hypothesis.completeness,
            correspondence: hypothesis.correspondence.clone(),
            causal_rules: hypothesis.causal_rules.clone(),
            draws,
        });
    }

    let mut bundle = CorrespondenceDrawBundle {
        bundle_schema: CORRESPONDENCE_DRAW_BUNDLE_SCHEMA_ID,
        bundle_digest: [0; 32],
        left_branch: plan.left_branch.clone(),
        right_branch: plan.right_branch.clone(),
        completeness: plan.completeness,
        universe_status: plan.universe_status.clone(),
        hypothesis_set_status: plan.hypothesis_set_status.clone(),
        hypotheses,
    };
    bundle.bundle_digest = correspondence_draw_bundle_digest(&bundle);
    Ok(bundle)
}

fn resolve_hypothesis_addresses(
    experiment_seed: &[u8; 32],
    hypothesis: &HypothesisAddressPlan,
    registry: &mut AddressCollisionRegistry,
) -> Result<Vec<CorrespondenceDrawReceipt>, AddressCollision> {
    let mut draws = Vec::with_capacity(hypothesis.addresses.len());
    for address in &hypothesis.addresses {
        let receipt = resolve_planned_address(experiment_seed, &hypothesis.hypothesis_id, address);
        registry.register(&receipt.draw)?;
        draws.push(receipt);
    }
    Ok(draws)
}

fn resolve_planned_address(
    experiment_seed: &[u8; 32],
    hypothesis_id: &crate::StableId,
    address: &PlannedAddress,
) -> CorrespondenceDrawReceipt {
    CorrespondenceDrawReceipt {
        hypothesis_id: hypothesis_id.clone(),
        branch_id: address.branch_id.clone(),
        event_id: address.event_id.clone(),
        sharing: address.sharing,
        presence: address.presence.clone(),
        canonical_address: address.canonical_bytes.clone(),
        draw: resolve_correspondence_address(experiment_seed, &address.canonical_bytes),
    }
}

/// Recompute a v0.2 address receipt against the exact canonical bytes and seed.
pub fn verify_correspondence_receipt(
    experiment_seed: &[u8; 32],
    canonical_address: &[u8],
    receipt: &DrawReceipt,
) -> Result<(), ReceiptMismatch> {
    if resolve_correspondence_address(experiment_seed, canonical_address) == *receipt {
        Ok(())
    } else {
        Err(ReceiptMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correspondence::{
        CandidateUniverseStatus, CorrespondenceHypothesis, EventPair, HypothesisSetStatus,
        IdentityBasis, UnmatchedEvent, UnmatchedReason,
    };
    use crate::coupling_projection::{
        CausalDisposition, CorrespondenceAddressPlan, HypothesisAddressPlan, PairCausalRule,
        PlanCompleteness, PlannedAddress, PlannedPresence, SharingStatus,
    };
    use crate::{Coupling, EventIdentity, StableId};

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("fixture ID must be canonical")
    }

    fn actor_address(branch: &str, shared: bool) -> DrawAddress {
        let coupling = if shared {
            Coupling::SharedInvariant {
                equivalence_group: id("crossing-weather"),
                justification: id("scm:weather-v1"),
            }
        } else {
            Coupling::BranchSpecific { branch: id(branch) }
        };
        DrawAddress::new(
            id("world-17"),
            id("weather-source"),
            id("gust-mechanism-v1"),
            EventIdentity::ActorEvent {
                actor: id("ped-004"),
                tick: 120,
                event_kind: id("wind-gust"),
            },
            id("uniform-0"),
            coupling,
        )
        .expect("fixture address is valid")
    }

    fn pair_causal_rule(
        hypothesis: &str,
        right_event: &str,
        basis: IdentityBasis,
        disposition: CausalDisposition,
    ) -> PairCausalRule {
        PairCausalRule {
            hypothesis_id: id(hypothesis),
            left_event: id("L1"),
            right_event: id(right_event),
            basis,
            disposition,
        }
    }

    fn correspondence_hypothesis(
        name: &str,
        right_event: &str,
        basis: IdentityBasis,
        unmatched: &str,
    ) -> CorrespondenceHypothesis {
        CorrespondenceHypothesis {
            hypothesis_id: id(name),
            justification_ref: id(&format!("justification-{name}")),
            pairs: vec![EventPair {
                left_event: id("L1"),
                right_event: id(right_event),
                basis,
                justification_ref: id(&format!("pairing-{name}")),
            }],
            unmatched_left: Vec::new(),
            unmatched_right: vec![UnmatchedEvent {
                event_id: id(unmatched),
                reason: UnmatchedReason::NoCounterpartUnderHypothesis {
                    rationale_ref: id(&format!("unmatched-{name}")),
                },
            }],
        }
    }

    fn planned_address(
        branch: &str,
        event: &str,
        bytes: &[u8],
        sharing: SharingStatus,
        presence: PlannedPresence,
    ) -> PlannedAddress {
        PlannedAddress {
            branch_id: id(branch),
            event_id: id(event),
            canonical_bytes: bytes.to_vec(),
            presence,
            sharing,
        }
    }

    fn correspondence_plan() -> CorrespondenceAddressPlan {
        let shared_pair = b"v0.2:world-17:H-actor:pair-L1-R1".to_vec();
        let alternative_pair = b"v0.2:world-17:H-slot:pair-L1-R2".to_vec();
        CorrespondenceAddressPlan {
            left_branch: id("continue"),
            right_branch: id("yield"),
            completeness: PlanCompleteness::CompleteWithinDeclaredSets,
            universe_status: CandidateUniverseStatus::ExhaustiveWithinDeclaredScope {
                evidence_ref: id("fixture-event-universe"),
            },
            hypothesis_set_status: HypothesisSetStatus::ExhaustiveWithinDeclaredModelFamily {
                evidence_ref: id("fixture-hypothesis-search"),
            },
            hypotheses: vec![
                HypothesisAddressPlan {
                    hypothesis_id: id("H-actor"),
                    completeness: PlanCompleteness::CompleteWithinDeclaredSets,
                    correspondence: correspondence_hypothesis(
                        "H-actor",
                        "R1",
                        IdentityBasis::Actor,
                        "R2",
                    ),
                    causal_rules: vec![pair_causal_rule(
                        "H-actor",
                        "R1",
                        IdentityBasis::Actor,
                        CausalDisposition::SharedInvariant {
                            equivalence_group: id("crossing-weather"),
                            justification: id("scm:weather-v1"),
                        },
                    )],
                    addresses: vec![
                        planned_address(
                            "continue",
                            "L1",
                            &shared_pair,
                            SharingStatus::SharedInvariant,
                            PlannedPresence::Observed {
                                evidence_ref: id("left-evidence"),
                            },
                        ),
                        planned_address(
                            "yield",
                            "R1",
                            &shared_pair,
                            SharingStatus::SharedInvariant,
                            PlannedPresence::NotRealized {
                                condition_witness: id("not-observed-after-yield"),
                            },
                        ),
                    ],
                },
                HypothesisAddressPlan {
                    hypothesis_id: id("H-slot"),
                    completeness: PlanCompleteness::CompleteWithinDeclaredSets,
                    correspondence: correspondence_hypothesis(
                        "H-slot",
                        "R2",
                        IdentityBasis::EncounterSlot,
                        "R1",
                    ),
                    causal_rules: vec![pair_causal_rule(
                        "H-slot",
                        "R2",
                        IdentityBasis::EncounterSlot,
                        CausalDisposition::BranchSpecific,
                    )],
                    addresses: vec![
                        planned_address(
                            "continue",
                            "L1",
                            &alternative_pair,
                            SharingStatus::SharedInvariant,
                            PlannedPresence::Observed {
                                evidence_ref: id("left-evidence"),
                            },
                        ),
                        planned_address(
                            "yield",
                            "R2",
                            &alternative_pair,
                            SharingStatus::SharedInvariant,
                            PlannedPresence::Observed {
                                evidence_ref: id("right-slot-evidence"),
                            },
                        ),
                    ],
                },
            ],
        }
    }

    #[test]
    fn correspondence_plan_resolves_each_admissible_map_without_collapsing_hypotheses() {
        let seed = [0x42; 32];
        let plan = correspondence_plan();
        let resolved = resolve_correspondence_plan(&seed, &plan).expect("complete draw bundle");

        assert_eq!(
            resolved.completeness,
            PlanCompleteness::CompleteWithinDeclaredSets
        );
        assert_eq!(resolved.hypotheses.len(), 2);

        let actor = &resolved.hypotheses[0];
        let slot = &resolved.hypotheses[1];
        assert_eq!(actor.hypothesis_id, id("H-actor"));
        assert_eq!(slot.hypothesis_id, id("H-slot"));
        assert_eq!(actor.draws.len(), 2);
        assert_eq!(slot.draws.len(), 2);

        assert_eq!(
            actor.draws[0].draw.derivation_digest,
            actor.draws[1].draw.derivation_digest
        );
        assert_eq!(actor.draws[0].draw.words, actor.draws[1].draw.words);
        assert_eq!(
            slot.draws[0].draw.derivation_digest,
            slot.draws[1].draw.derivation_digest
        );

        // Different counterpart hypotheses create different cross-world event identities.
        assert_ne!(
            actor.draws[0].draw.derivation_digest,
            slot.draws[0].draw.derivation_digest
        );

        // Presence is preserved separately from the potential event draw identity.
        assert!(matches!(
            &actor.draws[1].presence,
            PlannedPresence::NotRealized { .. }
        ));
        assert_eq!(actor.draws[0].sharing, SharingStatus::SharedInvariant);
    }

    #[test]
    fn resolved_bundle_retains_scope_evidence_and_every_canonical_address() {
        let seed = [0x6b; 32];
        let plan = correspondence_plan();
        let resolved = resolve_correspondence_plan(&seed, &plan).expect("draw bundle");

        assert_eq!(resolved.universe_status, plan.universe_status);
        assert_eq!(resolved.hypothesis_set_status, plan.hypothesis_set_status);
        assert_eq!(resolved.hypotheses.len(), plan.hypotheses.len());

        for (resolved_hypothesis, planned_hypothesis) in
            resolved.hypotheses.iter().zip(&plan.hypotheses)
        {
            assert_eq!(
                resolved_hypothesis.hypothesis_id,
                planned_hypothesis.hypothesis_id
            );
            assert_eq!(
                resolved_hypothesis.correspondence,
                planned_hypothesis.correspondence
            );
            assert_eq!(
                resolved_hypothesis.causal_rules,
                planned_hypothesis.causal_rules
            );
            assert_eq!(
                resolved_hypothesis.draws.len(),
                planned_hypothesis.addresses.len()
            );
            for (receipt, address) in resolved_hypothesis
                .draws
                .iter()
                .zip(&planned_hypothesis.addresses)
            {
                assert_eq!(receipt.canonical_address, address.canonical_bytes);
                assert_eq!(
                    verify_correspondence_receipt(&seed, &receipt.canonical_address, &receipt.draw),
                    Ok(())
                );
            }
        }
    }

    #[test]
    fn retained_canonical_address_detects_receipt_or_address_tampering() {
        let seed = [0x3d; 32];
        let plan = correspondence_plan();
        let resolved = resolve_correspondence_plan(&seed, &plan).expect("draw bundle");
        let receipt = &resolved.hypotheses[0].draws[0];

        let mut altered_address = receipt.canonical_address.clone();
        altered_address.push(0xff);
        assert_eq!(
            verify_correspondence_receipt(&seed, &altered_address, &receipt.draw),
            Err(ReceiptMismatch)
        );

        let mut altered_draw = receipt.draw.clone();
        altered_draw.words[0] ^= 1;
        assert_eq!(
            verify_correspondence_receipt(&seed, &receipt.canonical_address, &altered_draw),
            Err(ReceiptMismatch)
        );
    }

    #[test]
    fn bundle_digest_binds_correspondence_provenance_and_each_draw() {
        let seed = [0x71; 32];
        let plan = correspondence_plan();
        let bundle = resolve_correspondence_plan(&seed, &plan).expect("draw bundle");
        assert_eq!(verify_correspondence_draw_bundle(&seed, &bundle), Ok(()));

        let mut changed_scope = bundle.clone();
        changed_scope.universe_status = CandidateUniverseStatus::Incomplete {
            evidence_ref: id("replacement-universe-reference"),
        };
        assert_eq!(
            verify_correspondence_draw_bundle(&seed, &changed_scope),
            Err(CorrespondenceBundleVerificationError::BundleDigestMismatch)
        );

        let mut changed_map = bundle.clone();
        changed_map.hypotheses[0].correspondence.justification_ref =
            id("replacement-correspondence-justification");
        assert_eq!(
            verify_correspondence_draw_bundle(&seed, &changed_map),
            Err(CorrespondenceBundleVerificationError::BundleDigestMismatch)
        );

        // Even if an outer digest is recomputed, the inner Philox receipt must
        // still replay from its retained canonical address and frozen seed.
        let mut changed_draw = bundle.clone();
        changed_draw.hypotheses[0].draws[0].draw.words[0] ^= 1;
        changed_draw.bundle_digest = correspondence_draw_bundle_digest(&changed_draw);
        assert_eq!(
            verify_correspondence_draw_bundle(&seed, &changed_draw),
            Err(CorrespondenceBundleVerificationError::DrawReceiptMismatch)
        );
    }

    #[test]
    fn incomplete_scope_evidence_survives_address_resolution() {
        let seed = [0x92; 32];
        let mut plan = correspondence_plan();
        plan.completeness = PlanCompleteness::Incomplete;
        for hypothesis in &mut plan.hypotheses {
            hypothesis.completeness = PlanCompleteness::Incomplete;
        }
        plan.universe_status = CandidateUniverseStatus::Incomplete {
            evidence_ref: id("truncated-event-candidate-universe"),
        };
        plan.hypothesis_set_status = HypothesisSetStatus::IncompleteOrBudgetLimited {
            evidence_ref: id("bounded-hypothesis-search"),
        };
        let resolved = resolve_correspondence_plan(&seed, &plan).expect("known draws can resolve");

        assert_eq!(resolved.completeness, PlanCompleteness::Incomplete);
        assert_eq!(resolved.universe_status, plan.universe_status);
        assert_eq!(resolved.hypothesis_set_status, plan.hypothesis_set_status);
        assert_eq!(resolved.hypotheses.len(), plan.hypotheses.len());
        assert!(
            resolved
                .hypotheses
                .iter()
                .all(|hypothesis| hypothesis.completeness == PlanCompleteness::Incomplete)
        );
    }

    #[test]
    fn correspondence_receipt_replays_against_v0_2_address_bytes() {
        let seed = [0x27; 32];
        let plan = correspondence_plan();
        let left = &plan.hypotheses[0].addresses[0];
        let receipt = resolve_correspondence_address(&seed, &left.canonical_bytes);
        assert_eq!(receipt.address_schema, CORRESPONDENCE_ADDRESS_SCHEMA_ID);
        assert_eq!(receipt.derivation_id, CORRESPONDENCE_DERIVATION_ID);
        assert_eq!(
            verify_correspondence_receipt(&seed, &left.canonical_bytes, &receipt),
            Ok(())
        );

        let mut altered = left.canonical_bytes.clone();
        altered.push(0xff);
        assert_eq!(
            verify_correspondence_receipt(&seed, &altered, &receipt),
            Err(ReceiptMismatch)
        );
    }

    #[test]
    fn branch_local_noise_is_stable_across_alternative_correspondence_maps() {
        let seed = [0x19; 32];
        let local = b"v0.2:world-17:continue:L1:branch-local".to_vec();
        let first = planned_address(
            "continue",
            "L1",
            &local,
            SharingStatus::BranchSpecific,
            PlannedPresence::Observed {
                evidence_ref: id("evidence"),
            },
        );
        let second = planned_address(
            "continue",
            "L1",
            &local,
            SharingStatus::BranchSpecific,
            PlannedPresence::Observed {
                evidence_ref: id("evidence"),
            },
        );
        let first_receipt = resolve_planned_address(&seed, &id("H-actor"), &first);
        let second_receipt = resolve_planned_address(&seed, &id("H-slot"), &second);
        assert_eq!(
            first_receipt.draw.derivation_digest,
            second_receipt.draw.derivation_digest
        );
        assert_eq!(first_receipt.branch_id, second_receipt.branch_id);
        assert_ne!(first_receipt.hypothesis_id, second_receipt.hypothesis_id);
    }

    #[test]
    fn incomplete_correspondence_keeps_an_incomplete_draw_bundle() {
        let seed = [0x88; 32];
        let mut plan = correspondence_plan();
        plan.completeness = PlanCompleteness::Incomplete;
        plan.hypotheses[0].completeness = PlanCompleteness::Incomplete;

        let resolved = resolve_correspondence_plan(&seed, &plan).expect("known plans can resolve");
        assert_eq!(resolved.completeness, PlanCompleteness::Incomplete);
        assert_eq!(
            resolved.hypotheses[0].completeness,
            PlanCompleteness::Incomplete
        );
        assert_eq!(resolved.hypotheses[0].draws.len(), 2);
    }

    #[test]
    fn v0_1_and_v0_2_address_schemas_have_separate_kdf_domains() {
        let seed = [0x55; 32];
        let v0_1 = resolve_draw(&seed, &actor_address("continue", true));
        let v0_2 = resolve_correspondence_address(&seed, b"v0.2:otherwise-identical-event");
        assert_ne!(v0_1.address_schema, v0_2.address_schema);
        assert_ne!(v0_1.derivation_id, v0_2.derivation_id);
        assert_ne!(v0_1.derivation_digest, v0_2.derivation_digest);
    }

    #[test]
    fn raw_philox_matches_random123_known_answer_vectors() {
        assert_eq!(
            rand_philox::philox4x32_10([0, 0, 0, 0], [0, 0]),
            [0x6627_e8d5, 0xe169_c58d, 0xbc57_ac4c, 0x9b00_dbd8]
        );
        assert_eq!(
            rand_philox::philox4x32_10([u32::MAX; 4], [u32::MAX, u32::MAX]),
            [0x408f_276d, 0x41c8_3b0e, 0xa20b_c7c6, 0x6d54_51fd]
        );
        assert_eq!(
            rand_philox::philox4x32_10(
                [0x243f_6a88, 0x85a3_08d3, 0x1319_8a2e, 0x0370_7344],
                [0xa409_3822, 0x299f_31d0]
            ),
            [0xd16c_fe09, 0x94fd_cceb, 0x5001_e420, 0x2412_6ea1]
        );
    }

    #[test]
    fn explicit_counter_and_key_each_change_the_raw_block() {
        let base = rand_philox::philox4x32_10([0, 0, 0, 0], [0, 0]);
        let changed_counter = rand_philox::philox4x32_10([1, 0, 0, 0], [0, 0]);
        let changed_key = rand_philox::philox4x32_10([0, 0, 0, 0], [1, 0]);
        assert_ne!(base, changed_counter);
        assert_ne!(base, changed_key);
    }

    #[test]
    fn blake3_derive_key_matches_official_zero_length_vector() {
        let expected = [
            0x2c, 0xc3, 0x97, 0x83, 0xc2, 0x23, 0x15, 0x4f, 0xea, 0x8d, 0xfb, 0x7c, 0x1b, 0x16,
            0x60, 0xf2, 0xac, 0x2d, 0xcb, 0xd1, 0xc1, 0xde, 0x82, 0x77, 0xb0, 0xb0, 0xdd, 0x39,
            0xb7, 0xe5, 0x0d, 0x7d,
        ];
        assert_eq!(
            blake3::derive_key("BLAKE3 2019-12-27 16:29:52 test vectors context", b""),
            expected
        );
    }

    #[test]
    fn address_kdf_length_prefixes_seed_and_address_separately() {
        let seed = [0x5a; 32];
        let address = [0xa5, 0x5a, 0x11];
        let mut material = Vec::new();
        push_len(&mut material, seed.len());
        material.extend_from_slice(&seed);
        push_len(&mut material, address.len());
        material.extend_from_slice(&address);

        assert_eq!(&material[..8], &[32, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&material[40..48], &[3, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&material[48..], &address);
        assert_eq!(
            derive_address_digest_with_context(&seed, &address, DERIVATION_CONTEXT),
            blake3::derive_key(DERIVATION_CONTEXT, &material)
        );
    }

    #[test]
    fn kdf_context_is_domain_separated() {
        let seed = [0x31; 32];
        let address = [0x42, 0x43, 0x44];
        let digest = derive_address_digest_with_context(&seed, &address, DERIVATION_CONTEXT);
        let mut material = Vec::new();
        push_len(&mut material, seed.len());
        material.extend_from_slice(&seed);
        push_len(&mut material, address.len());
        material.extend_from_slice(&address);
        let other_context = blake3::derive_key("different application context", &material);
        assert_ne!(digest, other_context);
    }

    #[test]
    fn one_semantic_address_repeats_identically() {
        let seed = [42; 32];
        let address = actor_address("continue", true);
        assert_eq!(resolve_draw(&seed, &address), resolve_draw(&seed, &address));
    }

    #[test]
    fn shared_exogenous_address_is_branch_invariant() {
        let seed = [7; 32];
        let continue_address = actor_address("continue", true);
        let yield_address = actor_address("yield", true);
        assert_eq!(
            continue_address.canonical_bytes(),
            yield_address.canonical_bytes()
        );
        assert_eq!(
            resolve_draw(&seed, &continue_address),
            resolve_draw(&seed, &yield_address)
        );
    }

    #[test]
    fn branch_specific_address_is_not_shared() {
        let seed = [7; 32];
        let continue_receipt = resolve_draw(&seed, &actor_address("continue", false));
        let yield_receipt = resolve_draw(&seed, &actor_address("yield", false));
        assert_ne!(
            continue_receipt.derivation_digest,
            yield_receipt.derivation_digest
        );
        assert_ne!(
            (continue_receipt.key, continue_receipt.counter),
            (yield_receipt.key, yield_receipt.counter)
        );
    }

    #[test]
    fn draw_is_independent_of_call_order_and_unrelated_consumers() {
        let seed = [9; 32];
        let target = actor_address("continue", true);
        let expected = resolve_draw(&seed, &target);
        let _extra_policy_draw = resolve_draw(&seed, &actor_address("yield", false));
        let _another_unrelated_draw = resolve_draw(&seed, &actor_address("continue", false));
        assert_eq!(resolve_draw(&seed, &target), expected);
    }

    #[test]
    fn changing_experiment_seed_changes_the_derivation() {
        let address = actor_address("continue", true);
        let first = resolve_draw(&[1; 32], &address);
        let second = resolve_draw(&[2; 32], &address);
        assert_ne!(first.derivation_digest, second.derivation_digest);
        assert_ne!((first.key, first.counter), (second.key, second.counter));
    }

    #[test]
    fn changing_event_source_or_mechanism_changes_the_derivation() {
        let seed = [3; 32];
        let base = actor_address("continue", true);
        let source_changed = DrawAddress::new(
            id("world-17"),
            id("different-weather-source"),
            id("gust-mechanism-v1"),
            EventIdentity::ActorEvent {
                actor: id("ped-004"),
                tick: 120,
                event_kind: id("wind-gust"),
            },
            id("uniform-0"),
            Coupling::SharedInvariant {
                equivalence_group: id("crossing-weather"),
                justification: id("scm:weather-v1"),
            },
        )
        .unwrap();
        assert_ne!(
            resolve_draw(&seed, &base).derivation_digest,
            resolve_draw(&seed, &source_changed).derivation_digest
        );
    }

    #[test]
    fn receipt_is_bound_to_address_seed_mapping_and_output() {
        let seed = [5; 32];
        let address = actor_address("continue", true);
        let receipt = resolve_draw(&seed, &address);
        assert_eq!(verify_receipt(&seed, &address, &receipt), Ok(()));

        let mut altered = receipt.clone();
        altered.words[0] ^= 1;
        assert_eq!(
            verify_receipt(&seed, &address, &altered),
            Err(ReceiptMismatch)
        );
        assert_eq!(
            verify_receipt(&[6; 32], &address, &receipt),
            Err(ReceiptMismatch)
        );
    }

    #[test]
    fn collision_registry_accepts_same_full_mapping() {
        let seed = [8; 32];
        let receipt = resolve_draw(&seed, &actor_address("continue", true));
        let mut registry = AddressCollisionRegistry::default();
        assert!(registry.is_empty());
        assert_eq!(registry.register(&receipt), Ok(()));
        assert_eq!(registry.register(&receipt), Ok(()));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn collision_registry_rejects_same_block_with_different_full_digest() {
        let seed = [8; 32];
        let receipt = resolve_draw(&seed, &actor_address("continue", true));
        let mut conflicting = receipt.clone();
        conflicting.derivation_digest[31] ^= 1;

        let mut registry = AddressCollisionRegistry::default();
        assert_eq!(registry.register(&receipt), Ok(()));
        assert_eq!(
            registry.register(&conflicting),
            Err(AddressCollision {
                key: receipt.key,
                counter: receipt.counter,
            })
        );
    }

    #[test]
    fn collision_tag_is_the_retained_digest_suffix() {
        let receipt = resolve_draw(&[11; 32], &actor_address("continue", true));
        assert_eq!(receipt.collision_tag, receipt.derivation_digest[24..32]);
    }

    #[test]
    fn derivation_and_prng_versions_are_explicit_in_receipt() {
        let receipt = resolve_draw(&[0; 32], &actor_address("continue", true));
        assert_eq!(receipt.address_schema, ADDRESS_SCHEMA_ID);
        assert_eq!(receipt.derivation_id, DERIVATION_ID);
        assert_eq!(receipt.prng_id, PRNG_ID);
        assert_eq!(receipt.words.len(), PHILOX_OUTPUT_WORDS);
    }

    #[test]
    fn changing_draw_component_changes_the_address() {
        let seed = [13; 32];
        let base = actor_address("continue", true);
        let other_component = DrawAddress::new(
            id("world-17"),
            id("weather-source"),
            id("gust-mechanism-v1"),
            EventIdentity::ActorEvent {
                actor: id("ped-004"),
                tick: 120,
                event_kind: id("wind-gust"),
            },
            id("normal-quantile-0"),
            Coupling::SharedInvariant {
                equivalence_group: id("crossing-weather"),
                justification: id("scm:weather-v1"),
            },
        )
        .unwrap();
        assert_ne!(
            resolve_draw(&seed, &base).derivation_digest,
            resolve_draw(&seed, &other_component).derivation_digest
        );
    }
}
