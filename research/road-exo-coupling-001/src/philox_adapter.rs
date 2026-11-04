//! Deterministic semantic-address to Philox4x32-10 resolver.
//!
//! The caller must qualify event identity/coupling before entering this module.
//! This module derives one Philox block per canonical DrawAddress; no sequential
//! stream position or variable-consumption sampling loop is involved.

use std::collections::BTreeMap;

use rand_philox::Philox;

use crate::DrawAddress;

pub const ADDRESS_SCHEMA_ID: &str = "symtropy.road-exo-coupling.draw-address.v0.1";
pub const DERIVATION_CONTEXT: &str =
    "Symtropy ROAD-EXO-COUPLING-001 Philox4x32-10 draw-address v0.1";
pub const DERIVATION_ID: &str = "BLAKE3-derive-key-256/road-exo-v0.1";
pub const PRNG_ID: &str = "Philox4x32-10/rand_philox-0.2.1";
pub const PHILOX_OUTPUT_WORDS: usize = 4;

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
    let derivation_digest = derive_address_digest(experiment_seed, &canonical_address);

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
        address_schema: ADDRESS_SCHEMA_ID,
        derivation_id: DERIVATION_ID,
        prng_id: PRNG_ID,
        derivation_digest,
        collision_tag,
        key,
        counter,
        words,
    }
}

fn derive_address_digest(experiment_seed: &[u8; 32], address: &[u8]) -> [u8; 32] {
    let mut key_material = Vec::with_capacity(8 + experiment_seed.len() + 8 + address.len());
    push_len(&mut key_material, experiment_seed.len());
    key_material.extend_from_slice(experiment_seed);
    push_len(&mut key_material, address.len());
    key_material.extend_from_slice(address);
    blake3::derive_key(DERIVATION_CONTEXT, &key_material)
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

#[cfg(test)]
mod tests {
    use super::*;
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
            derive_address_digest(&seed, &address),
            blake3::derive_key(DERIVATION_CONTEXT, &material)
        );
    }

    #[test]
    fn kdf_context_is_domain_separated() {
        let seed = [0x31; 32];
        let address = [0x42, 0x43, 0x44];
        let digest = derive_address_digest(&seed, &address);
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
