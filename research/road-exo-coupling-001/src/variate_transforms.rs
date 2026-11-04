//! Address-scoped unbiased bounded-integer sampling.
//!
//! Every rejection attempt gets a distinct semantic child address. A rejected
//! attempt therefore cannot shift any sibling event's random stream.

use crate::philox_adapter::{resolve_draw, DrawReceipt};
use crate::{DrawAddress, StableId};

pub const TRANSFORM_SCHEMA_ID: &str = "symtropy.road-variate-transforms.v0.1";
pub const BOUNDED_INTEGER_TRANSFORM: &str = "lemire-multiply-high-rejection";
pub const BOUNDED_INTEGER_VERSION: &str = "1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttemptDisposition {
    RejectedBelowThreshold,
    Accepted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SamplingDisposition {
    Sampled,
    DeterministicSingleton,
    BudgetExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedAttempt {
    pub attempt_index: u32,
    pub child_address_digest: [u8; 32],
    pub draw: DrawReceipt,
    pub product_low: u32,
    pub candidate: u32,
    pub rejection_threshold: u32,
    pub disposition: AttemptDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedIntegerReceipt {
    pub schema_id: &'static str,
    pub transform_id: StableId,
    pub transform_version: StableId,
    pub root_address_digest: [u8; 32],
    pub bound_exclusive: u32,
    pub max_attempts: u32,
    pub disposition: SamplingDisposition,
    pub result: Option<u32>,
    pub attempts: Vec<BoundedAttempt>,
    pub receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SamplingError {
    ZeroBound,
    InvalidTransformId,
    ArithmeticOverflow,
    ReceiptMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WordMapping {
    product_low: u32,
    candidate: u32,
    rejection_threshold: u32,
    accepted: bool,
}

/// Sample one unbiased integer in [0, bound_exclusive).
///
/// The experiment seed and root DrawAddress are reproducibility inputs.
/// Their causal identity/coupling semantics must already be qualified upstream.
/// Attempt-budget exhaustion returns a receipt with no sample and no biased
/// fallback.
pub fn sample_bounded_u32(
    experiment_seed: &[u8; 32],
    root_address: &DrawAddress,
    transform_id: &StableId,
    transform_version: &StableId,
    bound_exclusive: u32,
    max_attempts: u32,
) -> Result<BoundedIntegerReceipt, SamplingError> {
    if bound_exclusive == 0 {
        return Err(SamplingError::ZeroBound);
    }
    if transform_id.as_str().len() > 64 || transform_version.as_str().len() > 32 {
        return Err(SamplingError::InvalidTransformId);
    }

    let root_address_digest = *blake3::hash(&root_address.canonical_bytes()).as_bytes();

    if bound_exclusive == 1 {
        return Ok(finish_receipt(
            transform_id.clone(),
            transform_version.clone(),
            root_address_digest,
            bound_exclusive,
            max_attempts,
            SamplingDisposition::DeterministicSingleton,
            Some(0),
            Vec::new(),
        ));
    }

    let mut attempts = Vec::new();
    for attempt_index in 0..max_attempts {
        let child = attempt_child_address(
            root_address,
            transform_id,
            transform_version,
            attempt_index,
        )?;
        let child_address_digest = *blake3::hash(&child.canonical_bytes()).as_bytes();
        let draw = resolve_draw(experiment_seed, &child);
        let mapping = map_word(draw.words[0], bound_exclusive);
        let disposition = if mapping.accepted {
            AttemptDisposition::Accepted
        } else {
            AttemptDisposition::RejectedBelowThreshold
        };

        attempts.push(BoundedAttempt {
            attempt_index,
            child_address_digest,
            draw,
            product_low: mapping.product_low,
            candidate: mapping.candidate,
            rejection_threshold: mapping.rejection_threshold,
            disposition,
        });

        if mapping.accepted {
            return Ok(finish_receipt(
                transform_id.clone(),
                transform_version.clone(),
                root_address_digest,
                bound_exclusive,
                max_attempts,
                SamplingDisposition::Sampled,
                Some(mapping.candidate),
                attempts,
            ));
        }
    }

    Ok(finish_receipt(
        transform_id.clone(),
        transform_version.clone(),
        root_address_digest,
        bound_exclusive,
        max_attempts,
        SamplingDisposition::BudgetExceeded,
        None,
        attempts,
    ))
}

fn map_word(word: u32, bound: u32) -> WordMapping {
    // Lemire multiply-high/rejection: no modulo reduction of the output.
    let product = u64::from(word) * u64::from(bound);
    let product_low = product as u32;
    let candidate = (product >> 32) as u32;
    let rejection_threshold = bound.wrapping_neg() % bound;
    WordMapping {
        product_low,
        candidate,
        rejection_threshold,
        accepted: product_low >= rejection_threshold,
    }
}

fn attempt_child_address(
    root_address: &DrawAddress,
    transform_id: &StableId,
    transform_version: &StableId,
    attempt_index: u32,
) -> Result<DrawAddress, SamplingError> {
    let root_bytes = root_address.canonical_bytes();
    let transform_bytes = transform_id.as_str().as_bytes();
    let version_bytes = transform_version.as_str().as_bytes();

    let mut preimage = Vec::new();
    preimage.extend_from_slice(b"symtropy.road-variate-child.v0.1\0");
    push_bytes(&mut preimage, &root_bytes)?;
    push_bytes(&mut preimage, transform_bytes)?;
    push_bytes(&mut preimage, version_bytes)?;
    preimage.extend_from_slice(&attempt_index.to_le_bytes());

    let digest = blake3::hash(&preimage);
    let component = StableId::new(format!(
        "sampler:{}:{}:{}",
        transform_id.as_str(),
        transform_version.as_str(),
        encode_hex(digest.as_bytes())
    ))
    .map_err(|_| SamplingError::InvalidTransformId)?;

    let mut child = root_address.clone();
    child.component = component;
    Ok(child)
}

fn finish_receipt(
    transform_id: StableId,
    transform_version: StableId,
    root_address_digest: [u8; 32],
    bound_exclusive: u32,
    max_attempts: u32,
    disposition: SamplingDisposition,
    result: Option<u32>,
    attempts: Vec<BoundedAttempt>,
) -> BoundedIntegerReceipt {
    let mut receipt = BoundedIntegerReceipt {
        schema_id: TRANSFORM_SCHEMA_ID,
        transform_id,
        transform_version,
        root_address_digest,
        bound_exclusive,
        max_attempts,
        disposition,
        result,
        attempts,
        receipt_digest: [0; 32],
    };
    receipt.receipt_digest = *blake3::hash(&receipt_bytes(&receipt)).as_bytes();
    receipt
}

fn receipt_bytes(receipt: &BoundedIntegerReceipt) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"symtropy.road-bounded-integer.receipt.v0.1\0");
    push_bytes(&mut out, receipt.schema_id.as_bytes())
        .expect("static schema ID length fits u64");
    push_id(&mut out, &receipt.transform_id);
    push_id(&mut out, &receipt.transform_version);
    out.extend_from_slice(&receipt.root_address_digest);
    out.extend_from_slice(&receipt.bound_exclusive.to_le_bytes());
    out.extend_from_slice(&receipt.max_attempts.to_le_bytes());
    out.push(match receipt.disposition {
        SamplingDisposition::Sampled => 1,
        SamplingDisposition::DeterministicSingleton => 2,
        SamplingDisposition::BudgetExceeded => 3,
    });
    match receipt.result {
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
        None => out.push(0),
    }
    push_len(&mut out, receipt.attempts.len());
    for attempt in &receipt.attempts {
        out.extend_from_slice(&attempt.attempt_index.to_le_bytes());
        out.extend_from_slice(&attempt.child_address_digest);
        out.extend_from_slice(&attempt.draw.derivation_digest);
        out.extend_from_slice(&attempt.draw.key[0].to_le_bytes());
        out.extend_from_slice(&attempt.draw.key[1].to_le_bytes());
        out.extend_from_slice(&attempt.draw.counter.to_le_bytes());
        for word in attempt.draw.words {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(&attempt.product_low.to_le_bytes());
        out.extend_from_slice(&attempt.candidate.to_le_bytes());
        out.extend_from_slice(&attempt.rejection_threshold.to_le_bytes());
        out.push(match attempt.disposition {
            AttemptDisposition::RejectedBelowThreshold => 1,
            AttemptDisposition::Accepted => 2,
        });
    }
    out
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let length = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), SamplingError> {
    let length = u64::try_from(bytes.len()).map_err(|_| SamplingError::ArithmeticOverflow)?;
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

fn push_len(out: &mut Vec<u8>, length: usize) {
    let length = u64::try_from(length).expect("fixture length must fit u64");
    out.extend_from_slice(&length.to_le_bytes());
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    result
}

impl BoundedIntegerReceipt {
    /// An unkeyed checksum only; this is not authentication.
    pub fn integrity_check(&self) -> bool {
        *blake3::hash(&receipt_bytes(self)).as_bytes() == self.receipt_digest
    }

    /// Recompute the sample and complete attempt trace from the frozen root.
    pub fn verify_against(
        &self,
        experiment_seed: &[u8; 32],
        root_address: &DrawAddress,
    ) -> Result<(), SamplingError> {
        let recomputed = sample_bounded_u32(
            experiment_seed,
            root_address,
            &self.transform_id,
            &self.transform_version,
            self.bound_exclusive,
            self.max_attempts,
        )?;
        if recomputed == *self {
            Ok(())
        } else {
            Err(SamplingError::ReceiptMismatch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Coupling, EventIdentity};

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("fixture ID")
    }

    fn root_address(branch: &str, shared: bool) -> DrawAddress {
        let coupling = if shared {
            Coupling::SharedInvariant {
                equivalence_group: id("crossing-weather"),
                justification: id("scm:weather-v1"),
            }
        } else {
            Coupling::BranchSpecific {
                branch: id(branch),
            }
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
            id("uniform-root"),
            coupling,
        )
        .expect("valid root address")
    }

    fn transform_id() -> StableId {
        id(BOUNDED_INTEGER_TRANSFORM)
    }

    fn transform_version() -> StableId {
        id(BOUNDED_INTEGER_VERSION)
    }

    #[test]
    fn multiply_high_rejection_mapping_has_exact_boundary_behavior() {
        let rejected = map_word(0, 3);
        assert_eq!(rejected.product_low, 0);
        assert_eq!(rejected.rejection_threshold, 1);
        assert!(!rejected.accepted);

        let accepted = map_word(1, 3);
        assert_eq!(accepted.product_low, 3);
        assert_eq!(accepted.candidate, 0);
        assert_eq!(accepted.rejection_threshold, 1);
        assert!(accepted.accepted);
    }

    #[test]
    fn zero_bound_rejects_and_singleton_consumes_no_draw() {
        let seed = [1; 32];
        let root = root_address("continue", true);
        assert_eq!(
            sample_bounded_u32(&seed, &root, &transform_id(), &transform_version(), 0, 8),
            Err(SamplingError::ZeroBound)
        );

        let receipt =
            sample_bounded_u32(&seed, &root, &transform_id(), &transform_version(), 1, 0)
                .unwrap();
        assert_eq!(
            receipt.disposition,
            SamplingDisposition::DeterministicSingleton
        );
        assert_eq!(receipt.result, Some(0));
        assert!(receipt.attempts.is_empty());
        assert!(receipt.integrity_check());
    }

    #[test]
    fn power_of_two_bound_never_rejects_and_stays_in_range() {
        let receipt = sample_bounded_u32(
            &[2; 32],
            &root_address("continue", true),
            &transform_id(),
            &transform_version(),
            8,
            8,
        )
        .unwrap();
        assert_eq!(receipt.disposition, SamplingDisposition::Sampled);
        assert_eq!(receipt.attempts.len(), 1);
        assert!(receipt.result.unwrap() < 8);
        assert_eq!(receipt.attempts[0].rejection_threshold, 0);
    }

    fn seed_with_first_attempt_rejection(root: &DrawAddress) -> [u8; 32] {
        for candidate in 0u16..512 {
            let mut seed = [0u8; 32];
            seed[30..32].copy_from_slice(&candidate.to_le_bytes());
            let child = attempt_child_address(
                root,
                &transform_id(),
                &transform_version(),
                0,
            )
            .unwrap();
            let draw = resolve_draw(&seed, &child);
            if !map_word(draw.words[0], 2_147_483_649).accepted {
                return seed;
            }
        }
        panic!("fixed bounded seed search finds a rejection fixture");
    }

    #[test]
    fn rejected_attempt_gets_a_distinct_semantic_child_address() {
        let root = root_address("continue", true);
        let seed = seed_with_first_attempt_rejection(&root);
        let receipt = sample_bounded_u32(
            &seed,
            &root,
            &transform_id(),
            &transform_version(),
            2_147_483_649,
            8,
        )
        .unwrap();
        assert_eq!(
            receipt.attempts[0].disposition,
            AttemptDisposition::RejectedBelowThreshold
        );
        assert!(receipt.attempts.len() >= 2);
        for adjacent in receipt.attempts.windows(2) {
            assert_eq!(adjacent[1].attempt_index, adjacent[0].attempt_index + 1);
            assert_ne!(
                adjacent[0].child_address_digest,
                adjacent[1].child_address_digest
            );
            assert_ne!(
                adjacent[0].draw.derivation_digest,
                adjacent[1].draw.derivation_digest
            );
        }
        assert_eq!(receipt.disposition, SamplingDisposition::Sampled);
        assert!(receipt.result.unwrap() < receipt.bound_exclusive);
    }

    #[test]
    fn attempt_budget_exhaustion_is_typed_without_biased_fallback() {
        let root = root_address("continue", true);
        let seed = seed_with_first_attempt_rejection(&root);
        let receipt = sample_bounded_u32(
            &seed,
            &root,
            &transform_id(),
            &transform_version(),
            2_147_483_649,
            1,
        )
        .unwrap();
        assert_eq!(receipt.disposition, SamplingDisposition::BudgetExceeded);
        assert_eq!(receipt.result, None);
        assert_eq!(receipt.attempts.len(), 1);
        assert_eq!(
            receipt.attempts[0].disposition,
            AttemptDisposition::RejectedBelowThreshold
        );
    }

    #[test]
    fn unrelated_draws_do_not_shift_bounded_sample() {
        let seed = [19; 32];
        let root = root_address("continue", true);
        let expected = sample_bounded_u32(
            &seed,
            &root,
            &transform_id(),
            &transform_version(),
            137,
            16,
        )
        .unwrap();
        let _other = resolve_draw(&seed, &root_address("yield", false));
        let _more = resolve_draw(&seed, &root_address("continue", false));
        assert_eq!(
            sample_bounded_u32(
                &seed,
                &root,
                &transform_id(),
                &transform_version(),
                137,
                16
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn shared_root_samples_match_across_branches_but_branch_local_roots_do_not() {
        let seed = [31; 32];
        let continue_shared = root_address("continue", true);
        let yield_shared = root_address("yield", true);
        assert_eq!(
            sample_bounded_u32(
                &seed,
                &continue_shared,
                &transform_id(),
                &transform_version(),
                101,
                16
            )
            .unwrap(),
            sample_bounded_u32(
                &seed,
                &yield_shared,
                &transform_id(),
                &transform_version(),
                101,
                16
            )
            .unwrap()
        );

        let continue_local = sample_bounded_u32(
            &seed,
            &root_address("continue", false),
            &transform_id(),
            &transform_version(),
            101,
            16,
        )
        .unwrap();
        let yield_local = sample_bounded_u32(
            &seed,
            &root_address("yield", false),
            &transform_id(),
            &transform_version(),
            101,
            16,
        )
        .unwrap();
        assert_ne!(
            continue_local.root_address_digest,
            yield_local.root_address_digest
        );
        assert_ne!(
            continue_local.attempts[0].draw.derivation_digest,
            yield_local.attempts[0].draw.derivation_digest
        );
    }

    #[test]
    fn transform_version_and_receipt_parameters_are_bound() {
        let seed = [39; 32];
        let root = root_address("continue", true);
        let receipt = sample_bounded_u32(
            &seed,
            &root,
            &transform_id(),
            &transform_version(),
            17,
            8,
        )
        .unwrap();
        assert_eq!(receipt.verify_against(&seed, &root), Ok(()));
        assert!(receipt.integrity_check());

        let mut tampered = receipt.clone();
        tampered.bound_exclusive += 1;
        assert!(!tampered.integrity_check());
        assert_eq!(
            tampered.verify_against(&seed, &root),
            Err(SamplingError::ReceiptMismatch)
        );
    }

    #[test]
    fn transform_identity_changes_child_address() {
        let root = root_address("continue", true);
        let first = attempt_child_address(
            &root,
            &id("lemire-multiply-high-rejection"),
            &id("1"),
            0,
        )
        .unwrap();
        let second = attempt_child_address(
            &root,
            &id("another-versioned-transform"),
            &id("1"),
            0,
        )
        .unwrap();
        let third = attempt_child_address(
            &root,
            &id("lemire-multiply-high-rejection"),
            &id("1"),
            1,
        )
        .unwrap();
        assert_ne!(first.canonical_bytes(), second.canonical_bytes());
        assert_ne!(first.canonical_bytes(), third.canonical_bytes());
    }
}
