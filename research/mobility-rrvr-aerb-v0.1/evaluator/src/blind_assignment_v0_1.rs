// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Evaluator-side CSPRNG assignment primitive for AERB blind campaigns.
//!
//! This module is deliberately separate from the public deterministic
//! conformance generator. It generates opaque episode identities, independently
//! sampled hidden-world assignments, a randomized candidate order and a
//! whole-manifest pre-run commitment. The candidate-visible manifest contains
//! no hidden labels or per-episode commitments.
//!
//! This is an assignment/commitment primitive, not a confidential execution
//! service or process sandbox. The caller must keep EvaluatorBlindManifest in
//! evaluator-only custody and must not expose the reveal before evaluation.

use std::collections::BTreeSet;

#[cfg(test)]
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use super::sha256_hex;

pub const BLIND_SCHEMA_VERSION: &str = "aerb-blind-campaign-v0.1";
pub const BLIND_CORPUS_CLASS: &str = "blind_evaluation";
const WORLD_IDS: [&str; 8] = [
    "not_dispatched",
    "dispatched_no_effect",
    "applied_once_ack_lost",
    "partial_effect",
    "duplicate_effect",
    "stale_authority_effect",
    "unresolved_command_holder",
    "open_world_residual",
];
const OPAQUE_ID_BYTES: usize = 16;
const MANIFEST_SALT_BYTES: usize = 32;
const MAX_EPISODES: usize = 100_000;
const MAX_ID_ATTEMPTS: usize = 32;
const COMMITMENT_DOMAIN: &[u8] = b"AERB-BLIND-REVEAL-V0.1\0";

/// Candidate-visible metadata. It contains no evaluator labels, salt or
/// per-episode commitments. Episode ordering is randomized before publication.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateBlindManifest {
    pub schema_version: String,
    pub corpus_class: String,
    pub campaign_id: String,
    pub campaign_commitment_sha256: String,
    pub episode_ids: Vec<String>,
}

/// Evaluator-only reveal. Do not mount or serialize this into the candidate
/// process. The salt is committed together with all assignments before a run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorBlindManifest {
    pub schema_version: String,
    pub corpus_class: String,
    pub campaign_id: String,
    pub manifest_salt_hex: String,
    pub assignments: Vec<BlindAssignment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlindAssignment {
    pub episode_id: String,
    pub hidden_truth_world_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedBlindCampaign {
    pub candidate_manifest: CandidateBlindManifest,
    pub evaluator_manifest: EvaluatorBlindManifest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlindGenerationError {
    InvalidEpisodeCount,
    EntropyUnavailable(String),
    DuplicateOpaqueIdSpaceExhausted,
    SerializationFailed(String),
    RevealMismatch(&'static str),
}

impl Display for BlindGenerationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEpisodeCount => {
                formatter.write_str("blind episode count must be within 1..=100000")
            }
            Self::EntropyUnavailable(reason) => {
                write!(formatter, "operating-system entropy unavailable: {reason}")
            }
            Self::DuplicateOpaqueIdSpaceExhausted => {
                formatter.write_str("opaque episode ID collision retry limit exhausted")
            }
            Self::SerializationFailed(reason) => {
                write!(formatter, "blind manifest serialization failed: {reason}")
            }
            Self::RevealMismatch(reason) => {
                write!(formatter, "blind campaign reveal mismatch: {reason}")
            }
        }
    }
}

impl Error for BlindGenerationError {}

trait EntropySource {
    fn fill_bytes(&mut self, destination: &mut [u8]) -> Result<(), BlindGenerationError>;
}

struct OperatingSystemEntropy;

impl EntropySource for OperatingSystemEntropy {
    fn fill_bytes(&mut self, destination: &mut [u8]) -> Result<(), BlindGenerationError> {
        getrandom::fill(destination)
            .map_err(|error| BlindGenerationError::EntropyUnavailable(error.to_string()))
    }
}

/// Generate an evaluator/candidate manifest pair using the operating system's
/// cryptographically secure random source. There is no seed parameter and no
/// deterministic fallback.
pub fn generate_blind_campaign(
    episode_count: usize,
) -> Result<GeneratedBlindCampaign, BlindGenerationError> {
    let mut entropy = OperatingSystemEntropy;
    generate_blind_campaign_with_entropy(episode_count, &mut entropy)
}

fn generate_blind_campaign_with_entropy<E: EntropySource>(
    episode_count: usize,
    entropy: &mut E,
) -> Result<GeneratedBlindCampaign, BlindGenerationError> {
    if !(1..=MAX_EPISODES).contains(&episode_count) {
        return Err(BlindGenerationError::InvalidEpisodeCount);
    }

    let campaign_id = format!("camp_{}", random_hex(entropy, OPAQUE_ID_BYTES)?);
    let manifest_salt_hex = random_hex(entropy, MANIFEST_SALT_BYTES)?;

    let mut assignments = Vec::with_capacity(episode_count);
    let mut unique_ids = BTreeSet::new();
    for _ in 0..episode_count {
        let episode_id = unique_opaque_episode_id(entropy, &mut unique_ids)?;
        let world_index = random_below(entropy, WORLD_IDS.len())?;
        assignments.push(BlindAssignment {
            episode_id,
            hidden_truth_world_id: WORLD_IDS[world_index].to_owned(),
        });
    }

    // Independent random shuffle. The hidden-world assignments are sampled
    // independently; no exact-quota or predictable-cycle balancing is applied.
    fisher_yates_shuffle(entropy, &mut assignments)?;

    let evaluator_manifest = EvaluatorBlindManifest {
        schema_version: BLIND_SCHEMA_VERSION.to_owned(),
        corpus_class: BLIND_CORPUS_CLASS.to_owned(),
        campaign_id: campaign_id.clone(),
        manifest_salt_hex,
        assignments,
    };
    let candidate_manifest = CandidateBlindManifest {
        schema_version: BLIND_SCHEMA_VERSION.to_owned(),
        corpus_class: BLIND_CORPUS_CLASS.to_owned(),
        campaign_id,
        campaign_commitment_sha256: commitment_for(&evaluator_manifest)?,
        episode_ids: evaluator_manifest
            .assignments
            .iter()
            .map(|assignment| assignment.episode_id.clone())
            .collect(),
    };

    Ok(GeneratedBlindCampaign {
        candidate_manifest,
        evaluator_manifest,
    })
}

/// Verify the post-run evaluator reveal against the candidate-visible
/// pre-run commitment. This checks byte/profile identity and assignment-ID
/// correspondence; it does not authenticate the custodian or establish
/// representativeness of the sampled campaign.
pub fn verify_post_run_reveal(
    candidate: &CandidateBlindManifest,
    evaluator: &EvaluatorBlindManifest,
) -> Result<(), BlindGenerationError> {
    if candidate.schema_version != BLIND_SCHEMA_VERSION
        || evaluator.schema_version != BLIND_SCHEMA_VERSION
    {
        return Err(BlindGenerationError::RevealMismatch("schema_version"));
    }
    if candidate.corpus_class != BLIND_CORPUS_CLASS
        || evaluator.corpus_class != BLIND_CORPUS_CLASS
    {
        return Err(BlindGenerationError::RevealMismatch("corpus_class"));
    }
    if candidate.campaign_id != evaluator.campaign_id
        || !is_opaque_campaign_id(&candidate.campaign_id)
    {
        return Err(BlindGenerationError::RevealMismatch("campaign_id"));
    }
    if !valid_sha256(&candidate.campaign_commitment_sha256) {
        return Err(BlindGenerationError::RevealMismatch("commitment_digest"));
    }
    if evaluator.manifest_salt_hex.len() != MANIFEST_SALT_BYTES * 2
        || !is_lower_hex(&evaluator.manifest_salt_hex)
    {
        return Err(BlindGenerationError::RevealMismatch("manifest_salt"));
    }
    if candidate.episode_ids.len() != evaluator.assignments.len() {
        return Err(BlindGenerationError::RevealMismatch("episode_count"));
    }

    let mut unique_ids = BTreeSet::new();
    for (candidate_id, assignment) in candidate
        .episode_ids
        .iter()
        .zip(evaluator.assignments.iter())
    {
        if candidate_id != &assignment.episode_id {
            return Err(BlindGenerationError::RevealMismatch(
                "candidate_episode_order_or_identity",
            ));
        }
        if !is_opaque_episode_id(candidate_id) || !unique_ids.insert(candidate_id.as_str()) {
            return Err(BlindGenerationError::RevealMismatch(
                "opaque_episode_id_uniqueness",
            ));
        }
        if !WORLD_IDS.contains(&assignment.hidden_truth_world_id.as_str()) {
            return Err(BlindGenerationError::RevealMismatch(
                "hidden_world_vocabulary",
            ));
        }
    }

    if commitment_for(evaluator)? != candidate.campaign_commitment_sha256 {
        return Err(BlindGenerationError::RevealMismatch("campaign_commitment"));
    }
    Ok(())
}

fn commitment_for(
    evaluator: &EvaluatorBlindManifest,
) -> Result<String, BlindGenerationError> {
    // Scoped profile: deterministic Serde struct/array serialization for this
    // exact Rust schema revision. This is not a claim of RFC 8785/JCS support.
    let bytes = serde_json::to_vec(evaluator)
        .map_err(|error| BlindGenerationError::SerializationFailed(error.to_string()))?;
    let mut preimage = Vec::with_capacity(COMMITMENT_DOMAIN.len() + bytes.len());
    preimage.extend_from_slice(COMMITMENT_DOMAIN);
    preimage.extend_from_slice(&bytes);
    Ok(sha256_hex(&preimage))
}

fn unique_opaque_episode_id<E: EntropySource>(
    entropy: &mut E,
    unique_ids: &mut BTreeSet<String>,
) -> Result<String, BlindGenerationError> {
    for _ in 0..MAX_ID_ATTEMPTS {
        let candidate = format!("ep_{}", random_hex(entropy, OPAQUE_ID_BYTES)?);
        if unique_ids.insert(candidate.clone()) {
            return Ok(candidate);
        }
    }
    Err(BlindGenerationError::DuplicateOpaqueIdSpaceExhausted)
}

fn random_hex<E: EntropySource>(
    entropy: &mut E,
    byte_count: usize,
) -> Result<String, BlindGenerationError> {
    let mut bytes = vec![0u8; byte_count];
    entropy.fill_bytes(&mut bytes)?;
    Ok(encode_hex(&bytes))
}

/// Uniform rejection sampling over [0, upper), avoiding modulo bias.
fn random_below<E: EntropySource>(
    entropy: &mut E,
    upper: usize,
) -> Result<usize, BlindGenerationError> {
    let upper = u64::try_from(upper)
        .map_err(|_| BlindGenerationError::InvalidEpisodeCount)?;
    if upper == 0 {
        return Err(BlindGenerationError::InvalidEpisodeCount);
    }
    let rejection_floor = upper.wrapping_neg() % upper;
    loop {
        let mut bytes = [0u8; 8];
        entropy.fill_bytes(&mut bytes)?;
        let value = u64::from_le_bytes(bytes);
        if value >= rejection_floor {
            return usize::try_from(value % upper)
                .map_err(|_| BlindGenerationError::InvalidEpisodeCount);
        }
    }
}

fn fisher_yates_shuffle<E: EntropySource>(
    entropy: &mut E,
    assignments: &mut [BlindAssignment],
) -> Result<(), BlindGenerationError> {
    for index in (1..assignments.len()).rev() {
        let other = random_below(entropy, index + 1)?;
        assignments.swap(index, other);
    }
    Ok(())
}

fn is_opaque_episode_id(value: &str) -> bool {
    value.len() == 3 + OPAQUE_ID_BYTES * 2
        && value.starts_with("ep_")
        && is_lower_hex(&value[3..])
}

fn is_opaque_campaign_id(value: &str) -> bool {
    value.len() == 5 + OPAQUE_ID_BYTES * 2
        && value.starts_with("camp_")
        && is_lower_hex(&value[5..])
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && is_lower_hex(value)
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CounterEntropy {
        calls: u8,
    }

    impl EntropySource for CounterEntropy {
        fn fill_bytes(
            &mut self,
            destination: &mut [u8],
        ) -> Result<(), BlindGenerationError> {
            let value = self.calls;
            self.calls = self.calls.wrapping_add(1);
            destination.fill(value);
            Ok(())
        }
    }

    struct FailingEntropy;

    impl EntropySource for FailingEntropy {
        fn fill_bytes(
            &mut self,
            _destination: &mut [u8],
        ) -> Result<(), BlindGenerationError> {
            Err(BlindGenerationError::EntropyUnavailable(
                "injected test failure".to_owned(),
            ))
        }
    }

    #[test]
    fn candidate_manifest_never_contains_hidden_labels_or_evaluator_fields() {
        let mut entropy = CounterEntropy { calls: 0 };
        let campaign = generate_blind_campaign_with_entropy(16, &mut entropy)
            .expect("test campaign generation");
        let candidate_json = serde_json::to_string(&campaign.candidate_manifest)
            .expect("candidate manifest serialization");

        assert!(!candidate_json.contains("hidden_truth_world_id"));
        assert!(!candidate_json.contains("manifest_salt_hex"));
        assert!(!candidate_json.contains("assignments"));
        for world_id in WORLD_IDS {
            assert!(!candidate_json.contains(world_id));
        }
    }

    #[test]
    fn pre_run_commitment_verifies_against_post_run_reveal() {
        let campaign = generate_blind_campaign(32).expect("OS-random campaign");
        verify_post_run_reveal(
            &campaign.candidate_manifest,
            &campaign.evaluator_manifest,
        )
        .expect("reveal matches pre-run commitment");
    }

    #[test]
    fn changing_hidden_assignment_invalidates_the_campaign_commitment() {
        let mut entropy = CounterEntropy { calls: 0 };
        let mut campaign = generate_blind_campaign_with_entropy(8, &mut entropy)
            .expect("test campaign generation");
        let original = &campaign.evaluator_manifest.assignments[0].hidden_truth_world_id;
        let replacement = WORLD_IDS
            .iter()
            .find(|world_id| **world_id != original.as_str())
            .expect("different hidden world");
        campaign.evaluator_manifest.assignments[0].hidden_truth_world_id =
            (*replacement).to_owned();
        assert_eq!(
            verify_post_run_reveal(
                &campaign.candidate_manifest,
                &campaign.evaluator_manifest,
            ),
            Err(BlindGenerationError::RevealMismatch("campaign_commitment"))
        );
    }

    #[test]
    fn nonsemantic_manifest_order_change_invalidates_pre_run_commitment() {
        let mut entropy = CounterEntropy { calls: 0 };
        let mut campaign = generate_blind_campaign_with_entropy(8, &mut entropy)
            .expect("test campaign generation");
        campaign.evaluator_manifest.assignments.swap(0, 1);
        assert_eq!(
            verify_post_run_reveal(
                &campaign.candidate_manifest,
                &campaign.evaluator_manifest,
            ),
            Err(BlindGenerationError::RevealMismatch(
                "candidate_episode_order_or_identity"
            ))
        );
    }

    #[test]
    fn invalid_episode_count_is_rejected_without_using_entropy() {
        let mut entropy = FailingEntropy;
        assert_eq!(
            generate_blind_campaign_with_entropy(0, &mut entropy),
            Err(BlindGenerationError::InvalidEpisodeCount)
        );
    }

    #[test]
    fn entropy_failure_fails_closed_without_pseudorandom_fallback() {
        let mut entropy = FailingEntropy;
        assert_eq!(
            generate_blind_campaign_with_entropy(1, &mut entropy),
            Err(BlindGenerationError::EntropyUnavailable(
                "injected test failure".to_owned()
            ))
        );
    }

    #[test]
    fn generated_episode_ids_are_opaque_unique_and_not_sequential() {
        let mut entropy = CounterEntropy { calls: 0 };
        let campaign = generate_blind_campaign_with_entropy(32, &mut entropy)
            .expect("test campaign generation");
        let ids = &campaign.candidate_manifest.episode_ids;
        let unique = ids.iter().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), ids.len());
        assert!(ids.iter().all(|id| is_opaque_episode_id(id)));
        assert!(!ids.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn assignment_accepts_unbalanced_world_frequencies_without_exact_quotas() {
        let mut entropy = CounterEntropy { calls: 0 };
        let campaign = generate_blind_campaign_with_entropy(16, &mut entropy)
            .expect("test campaign generation");
        let mut counts = BTreeMap::<&str, usize>::new();
        for assignment in &campaign.evaluator_manifest.assignments {
            *counts
                .entry(assignment.hidden_truth_world_id.as_str())
                .or_default() += 1;
        }
        assert!(counts.values().any(|count| *count != 2));
    }


    #[test]
    fn campaign_identity_tamper_invalidates_the_reveal() {
        let mut entropy = CounterEntropy { calls: 0 };
        let mut campaign = generate_blind_campaign_with_entropy(4, &mut entropy)
            .expect("test campaign generation");
        campaign.candidate_manifest.campaign_id = format!("camp_{}", "a".repeat(32));
        assert_eq!(
            verify_post_run_reveal(
                &campaign.candidate_manifest,
                &campaign.evaluator_manifest,
            ),
            Err(BlindGenerationError::RevealMismatch("campaign_id"))
        );
    }

    #[test]
    fn reveal_salt_tamper_invalidates_the_campaign_commitment() {
        let mut entropy = CounterEntropy { calls: 0 };
        let mut campaign = generate_blind_campaign_with_entropy(4, &mut entropy)
            .expect("test campaign generation");
        campaign.evaluator_manifest.manifest_salt_hex = "f".repeat(64);
        assert_eq!(
            verify_post_run_reveal(
                &campaign.candidate_manifest,
                &campaign.evaluator_manifest,
            ),
            Err(BlindGenerationError::RevealMismatch("campaign_commitment"))
        );
    }
}
