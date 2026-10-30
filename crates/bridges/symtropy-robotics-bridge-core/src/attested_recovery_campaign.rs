// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fail-closed recovery campaign scorer requiring signed evidence for every row.
//!
//! This layer joins preregistered scenarios, paired recovery records, command
//! traces, deterministic receipts, Ed25519 attestations, and an independently
//! supplied trust policy. It does not prove physical truth: that still requires
//! authenticated native event capture and an independent actuator witness.

use crate::audit_receipt::CommandEvidenceReceiptContext;
use crate::command_evidence::{CommandEvidenceEvent, ExpectedCommandIdentity};
use crate::receipt_attestation::{
    CommandEvidenceAttestationError, EvidenceTrustPolicy, SignedCommandEvidenceReceipt,
    verify_signed_command_evidence_receipt,
};
use crate::recovery_benchmark::{
    ExpectedRecoveryScenario, RecoveryBenchmarkArm, RecoveryBenchmarkError,
    RecoveryBenchmarkSummary, RecoveryTrialRecord, score_recovery_campaign,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Independently supplied campaign identity and per-arm policy bindings.
///
/// This object must come from the benchmark orchestrator or a preregistered
/// campaign manifest, not from the trial record being evaluated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryCampaignEvidencePolicy {
    pub campaign_id: String,
    pub campaign_revision: String,
    pub scenario_manifest_digest: String,
    pub trace_source_id: String,
    pub evaluator_version: String,
    pub policy_digest_by_arm: BTreeMap<RecoveryBenchmarkArm, String>,
}

/// One recovery trial plus the raw command evidence and its signed receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestedRecoveryTrialRecord {
    pub trial: RecoveryTrialRecord,
    pub expected_commands: Vec<ExpectedCommandIdentity>,
    /// Raw globally ordered event stream used to create and verify the receipt.
    pub events: Vec<CommandEvidenceEvent>,
    pub signed_receipt: SignedCommandEvidenceReceipt,
}

/// A campaign result is available only after every expected row has passed
/// evidence verification and the paired recovery scorer has accepted the data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttestedRecoveryCampaignSummary {
    pub recovery: RecoveryBenchmarkSummary,
    pub verified_trial_count: usize,
    /// Root commitment over campaign policy, trust policy, and every signed receipt.
    pub evidence_root_digest: String,
}

/// Failure modes for campaign evidence coverage, binding, and verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestedRecoveryCampaignError {
    EmptyExpectedCampaign,
    EmptyAttestedTrials,
    EmptyPolicyField { field: &'static str },
    MissingArmPolicyDigest { arm: RecoveryBenchmarkArm },
    EmptyArmPolicyDigest { arm: RecoveryBenchmarkArm },
    DuplicateExpectedScenario,
    MissingTrialEvidence {
        scenario_id: String,
        seed: u64,
        arm: RecoveryBenchmarkArm,
    },
    DuplicateTrialEvidence {
        scenario_id: String,
        seed: u64,
        arm: RecoveryBenchmarkArm,
    },
    UnexpectedTrialEvidence {
        scenario_id: String,
        seed: u64,
        arm: RecoveryBenchmarkArm,
    },
    ReceiptContextMismatch {
        scenario_id: String,
        seed: u64,
        arm: RecoveryBenchmarkArm,
        field: &'static str,
    },
    AttestationVerificationFailed {
        scenario_id: String,
        seed: u64,
        arm: RecoveryBenchmarkArm,
        source: CommandEvidenceAttestationError,
    },
    RecoveryBenchmark(RecoveryBenchmarkError),
}

impl From<RecoveryBenchmarkError> for AttestedRecoveryCampaignError {
    fn from(value: RecoveryBenchmarkError) -> Self {
        Self::RecoveryBenchmark(value)
    }
}

type ScenarioKey = (String, u64);
type TrialEvidenceKey = (String, u64, RecoveryBenchmarkArm);

/// Score a paired recovery campaign only when every expected scenario/arm row
/// has a matching, trusted, signed command-evidence receipt.
///
/// The context checked for each receipt is reconstructed from the independent
/// campaign policy, preregistered scenario manifest, and recovery trial record.
/// The verifier then replays the auditor over the original expected identities
/// and ordered events. Missing, duplicate, unexpected, untrusted, or mismatched
/// evidence makes the campaign incomplete; it is never silently omitted.
pub fn score_attested_recovery_campaign(
    expected: &[ExpectedRecoveryScenario],
    trials: &[AttestedRecoveryTrialRecord],
    campaign_policy: &RecoveryCampaignEvidencePolicy,
    trust_policy: &EvidenceTrustPolicy,
) -> Result<AttestedRecoveryCampaignSummary, AttestedRecoveryCampaignError> {
    if expected.is_empty() {
        return Err(AttestedRecoveryCampaignError::EmptyExpectedCampaign);
    }
    if trials.is_empty() {
        return Err(AttestedRecoveryCampaignError::EmptyAttestedTrials);
    }
    validate_campaign_policy(campaign_policy)?;

    let mut expected_by_key: BTreeMap<ScenarioKey, &ExpectedRecoveryScenario> = BTreeMap::new();
    for scenario in expected {
        let key = (scenario.scenario_id.clone(), scenario.seed);
        if expected_by_key.insert(key, scenario).is_some() {
            return Err(AttestedRecoveryCampaignError::DuplicateExpectedScenario);
        }
    }

    let mut evidence_by_key: BTreeMap<TrialEvidenceKey, &AttestedRecoveryTrialRecord> =
        BTreeMap::new();
    for attested in trials {
        let trial = &attested.trial;
        let scenario_key = (trial.scenario_id.clone(), trial.seed);
        let Some(scenario) = expected_by_key.get(&scenario_key) else {
            return Err(AttestedRecoveryCampaignError::UnexpectedTrialEvidence {
                scenario_id: trial.scenario_id.clone(),
                seed: trial.seed,
                arm: trial.arm,
            });
        };

        let key = (trial.scenario_id.clone(), trial.seed, trial.arm);
        if evidence_by_key.insert(key, attested).is_some() {
            return Err(AttestedRecoveryCampaignError::DuplicateTrialEvidence {
                scenario_id: trial.scenario_id.clone(),
                seed: trial.seed,
                arm: trial.arm,
            });
        }

        let expected_context = expected_receipt_context(
            campaign_policy,
            scenario,
            trial,
        );
        if let Some(field) = context_mismatch_field(
            &expected_context,
            &attested.signed_receipt.receipt.context,
        ) {
            return Err(AttestedRecoveryCampaignError::ReceiptContextMismatch {
                scenario_id: trial.scenario_id.clone(),
                seed: trial.seed,
                arm: trial.arm,
                field,
            });
        }

        verify_signed_command_evidence_receipt(
            &attested.signed_receipt,
            &expected_context,
            &attested.expected_commands,
            &attested.events,
            trust_policy,
        )
        .map_err(|source| {
            AttestedRecoveryCampaignError::AttestationVerificationFailed {
                scenario_id: trial.scenario_id.clone(),
                seed: trial.seed,
                arm: trial.arm,
                source,
            }
        })?;
    }

    for scenario in expected {
        for arm in RecoveryBenchmarkArm::ALL {
            let key = (scenario.scenario_id.clone(), scenario.seed, arm);
            if !evidence_by_key.contains_key(&key) {
                return Err(AttestedRecoveryCampaignError::MissingTrialEvidence {
                    scenario_id: scenario.scenario_id.clone(),
                    seed: scenario.seed,
                    arm,
                });
            }
        }
    }

    // Use the scorer's existing complete-pairing and hard-gate logic only after
    // evidence coverage and authenticity checks have passed for every row.
    let ordered_trials = evidence_by_key
        .values()
        .map(|record| record.trial.clone())
        .collect::<Vec<_>>();
    let recovery = score_recovery_campaign(expected, &ordered_trials)?;
    let evidence_root_digest = digest_campaign_evidence(
        campaign_policy,
        trust_policy,
        &evidence_by_key,
    );

    Ok(AttestedRecoveryCampaignSummary {
        recovery,
        verified_trial_count: evidence_by_key.len(),
        evidence_root_digest,
    })
}

fn validate_campaign_policy(
    policy: &RecoveryCampaignEvidencePolicy,
) -> Result<(), AttestedRecoveryCampaignError> {
    for (field, value) in [
        ("campaign_id", policy.campaign_id.as_str()),
        ("campaign_revision", policy.campaign_revision.as_str()),
        (
            "scenario_manifest_digest",
            policy.scenario_manifest_digest.as_str(),
        ),
        ("trace_source_id", policy.trace_source_id.as_str()),
        ("evaluator_version", policy.evaluator_version.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AttestedRecoveryCampaignError::EmptyPolicyField { field });
        }
    }
    if trust_policy_revision_is_empty_placeholder(policy) {
        // Kept as a separate helper so the policy validation contract remains
        // explicit without conflating campaign policy with attestor key policy.
    }
    for arm in RecoveryBenchmarkArm::ALL {
        let digest = policy
            .policy_digest_by_arm
            .get(&arm)
            .ok_or(AttestedRecoveryCampaignError::MissingArmPolicyDigest { arm })?;
        if digest.trim().is_empty() {
            return Err(AttestedRecoveryCampaignError::EmptyArmPolicyDigest { arm });
        }
    }
    Ok(())
}

fn trust_policy_revision_is_empty_placeholder(_policy: &RecoveryCampaignEvidencePolicy) -> bool {
    false
}

fn expected_receipt_context(
    policy: &RecoveryCampaignEvidencePolicy,
    scenario: &ExpectedRecoveryScenario,
    trial: &RecoveryTrialRecord,
) -> CommandEvidenceReceiptContext {
    CommandEvidenceReceiptContext {
        campaign_id: policy.campaign_id.clone(),
        campaign_revision: policy.campaign_revision.clone(),
        scenario_manifest_digest: policy.scenario_manifest_digest.clone(),
        scenario_id: scenario.scenario_id.clone(),
        scenario_digest: scenario.scenario_digest.clone(),
        arm_id: trial.arm.id().to_owned(),
        arm_source_revision: trial.arm_source_revision.clone(),
        policy_digest: policy
            .policy_digest_by_arm
            .get(&trial.arm)
            .cloned()
            .unwrap_or_default(),
        task_profile_digest: trial.profile_digest.clone(),
        plant_digest: trial.plant_digest.clone(),
        controller_config_digest: trial.native_controller_digest.clone(),
        trace_source_id: policy.trace_source_id.clone(),
        evaluator_version: policy.evaluator_version.clone(),
    }
}

fn context_mismatch_field(
    expected: &CommandEvidenceReceiptContext,
    actual: &CommandEvidenceReceiptContext,
) -> Option<&'static str> {
    if expected.campaign_id != actual.campaign_id {
        Some("campaign_id")
    } else if expected.campaign_revision != actual.campaign_revision {
        Some("campaign_revision")
    } else if expected.scenario_manifest_digest != actual.scenario_manifest_digest {
        Some("scenario_manifest_digest")
    } else if expected.scenario_id != actual.scenario_id {
        Some("scenario_id")
    } else if expected.scenario_digest != actual.scenario_digest {
        Some("scenario_digest")
    } else if expected.arm_id != actual.arm_id {
        Some("arm_id")
    } else if expected.arm_source_revision != actual.arm_source_revision {
        Some("arm_source_revision")
    } else if expected.policy_digest != actual.policy_digest {
        Some("policy_digest")
    } else if expected.task_profile_digest != actual.task_profile_digest {
        Some("task_profile_digest")
    } else if expected.plant_digest != actual.plant_digest {
        Some("plant_digest")
    } else if expected.controller_config_digest != actual.controller_config_digest {
        Some("controller_config_digest")
    } else if expected.trace_source_id != actual.trace_source_id {
        Some("trace_source_id")
    } else if expected.evaluator_version != actual.evaluator_version {
        Some("evaluator_version")
    } else {
        None
    }
}

fn digest_campaign_evidence(
    policy: &RecoveryCampaignEvidencePolicy,
    trust_policy: &EvidenceTrustPolicy,
    evidence: &BTreeMap<TrialEvidenceKey, &AttestedRecoveryTrialRecord>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"symtropy.attested-recovery-campaign.root.v1");
    hasher.update([0]);
    hash_string(&mut hasher, &policy.campaign_id);
    hash_string(&mut hasher, &policy.campaign_revision);
    hash_string(&mut hasher, &policy.scenario_manifest_digest);
    hash_string(&mut hasher, &policy.trace_source_id);
    hash_string(&mut hasher, &policy.evaluator_version);
    hash_string(&mut hasher, &trust_policy.policy_revision);

    hasher.update((policy.policy_digest_by_arm.len() as u64).to_le_bytes());
    for (arm, digest) in &policy.policy_digest_by_arm {
        hash_string(&mut hasher, arm.id());
        hash_string(&mut hasher, digest);
    }

    let mut signers = trust_policy.trusted_signers.iter().collect::<Vec<_>>();
    signers.sort_by(|left, right| left.key_id.cmp(&right.key_id));
    hasher.update((signers.len() as u64).to_le_bytes());
    for signer in signers {
        hash_string(&mut hasher, &signer.key_id);
        hash_bytes(&mut hasher, &signer.public_key);
        hasher.update([match signer.role {
            crate::receipt_attestation::EvidenceSignerRole::IndependentReceiptAttestor => 0,
            crate::receipt_attestation::EvidenceSignerRole::TraceCollector => 1,
            crate::receipt_attestation::EvidenceSignerRole::HardwareApplicationWitness => 2,
        }]);
        hasher.update([u8::from(signer.revoked)]);
    }

    hasher.update((evidence.len() as u64).to_le_bytes());
    for ((scenario_id, seed, arm), record) in evidence {
        hash_string(&mut hasher, scenario_id);
        hasher.update(seed.to_le_bytes());
        hash_string(&mut hasher, arm.id());
        hash_string(&mut hasher, &record.signed_receipt.receipt.receipt_digest);
        hash_string(&mut hasher, &record.signed_receipt.trust_policy_revision);
        hash_string(&mut hasher, &record.signed_receipt.signer_key_id);
        hash_bytes(&mut hasher, &record.signed_receipt.signer_public_key);
        hash_bytes(&mut hasher, &record.signed_receipt.signature);
    }

    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(7 + digest.len() * 2);
    encoded.push_str("sha256:");
    use std::fmt::Write as _;
    for byte in digest {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    hash_bytes(hasher, value.as_bytes());
}

fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit_receipt::create_command_evidence_receipt;
    use crate::command_evidence::{CommandEvidenceKind, CommandTerminalOutcome};
    use crate::receipt_attestation::{
        EvidenceSignerRole, TrustedEvidenceSigner, sign_command_evidence_receipt,
    };
    use ring::signature::{Ed25519KeyPair, KeyPair};

    fn nominal_class() -> crate::recovery_benchmark::RecoveryScenarioClass {
        crate::recovery_benchmark::RecoveryScenarioClass::Nominal
    }

    fn faulted_class() -> crate::recovery_benchmark::RecoveryScenarioClass {
        crate::recovery_benchmark::RecoveryScenarioClass::Faulted {
            family_id: "outcome_stream_disorder".into(),
            severity_id: "single_duplicate_or_delay".into(),
        }
    }

    fn scenario(id: &str, seed: u64, class: crate::recovery_benchmark::RecoveryScenarioClass)
        -> ExpectedRecoveryScenario
    {
        ExpectedRecoveryScenario {
            scenario_id: id.into(),
            seed,
            class,
            scenario_digest: format!("scenario-digest-{id}-{seed}"),
        }
    }

    fn trial(
        scenario: &ExpectedRecoveryScenario,
        arm: RecoveryBenchmarkArm,
    ) -> RecoveryTrialRecord {
        let faulted = scenario.class == faulted_class();
        let is_b3 = arm == RecoveryBenchmarkArm::GuardedOutcomeFeedback;
        let is_b2 = arm == RecoveryBenchmarkArm::GuardedAdapter;
        let recovery_success = faulted.then_some(is_b3 || !is_b2);
        let latency = recovery_success.map(|success| if success { 100 } else { 0 });
        RecoveryTrialRecord {
            scenario_id: scenario.scenario_id.clone(),
            seed: scenario.seed,
            class: scenario.class.clone(),
            scenario_digest: scenario.scenario_digest.clone(),
            profile_digest: "profile-digest".into(),
            plant_digest: "plant-digest".into(),
            native_controller_digest: "native-controller-digest".into(),
            arm,
            arm_source_revision: format!("revision-{}", arm.id()),
            task_success: true,
            recovery_success,
            time_to_safe_recovery_ns: latency,
            hard_boundary_violations: 0,
            unsafe_dispatches: 0,
            outcome_integrity_violations: 0,
            stale_epoch_event_acceptances: 0,
            stale_epoch_command_admission_acceptances: 0,
            stale_epoch_command_application_acceptances: 0,
            expired_command_application_acceptances: 0,
            native_controller_epoch_mismatch_acceptances: 0,
            native_expired_command_acceptances: 0,
            epoch_authority_fail_open_acceptances: 0,
            unresolved_lifecycles: 0,
            controller_deadline_misses: 0,
            admissible_requests: if faulted { 0 } else { 10 },
            false_vetoes: 0,
        }
    }

    fn campaign_policy() -> RecoveryCampaignEvidencePolicy {
        let policy_digest_by_arm = RecoveryBenchmarkArm::ALL
            .into_iter()
            .map(|arm| (arm, format!("sha256:policy-{}", arm.id())))
            .collect();
        RecoveryCampaignEvidencePolicy {
            campaign_id: "campaign-robotics-1".into(),
            campaign_revision: "campaign-rev-1".into(),
            scenario_manifest_digest: "sha256:manifest".into(),
            trace_source_id: "trace-collector-v1".into(),
            evaluator_version: "command-evidence-auditor-v0.1".into(),
            policy_digest_by_arm,
        }
    }

    fn key_pair() -> Ed25519KeyPair {
        Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap()
    }

    fn trust_policy(pair: &Ed25519KeyPair) -> EvidenceTrustPolicy {
        EvidenceTrustPolicy {
            policy_revision: "trust-policy-v1".into(),
            trusted_signers: vec![TrustedEvidenceSigner {
                key_id: "independent-auditor-1".into(),
                public_key: pair.public_key().as_ref().to_vec(),
                role: EvidenceSignerRole::IndependentReceiptAttestor,
                revoked: false,
            }],
        }
    }

    fn attested_trial(
        scenario: &ExpectedRecoveryScenario,
        trial: RecoveryTrialRecord,
        policy: &RecoveryCampaignEvidencePolicy,
        pair: &Ed25519KeyPair,
    ) -> AttestedRecoveryTrialRecord {
        let identity = ExpectedCommandIdentity {
            request_id: 1,
            session_id: 1,
            sequence: 1,
            controller_id: "mobile-base-01".into(),
            controller_epoch: 1,
            command_payload_digest: format!("sha256:command-{}", trial.arm.id()),
            controller_config_digest: trial.native_controller_digest.clone(),
            deadline_ns: 1_000,
            clock_domain_id: "robot-monotonic-0".into(),
        };
        let expected_commands = vec![identity.clone()];
        let events = vec![
            CommandEvidenceEvent {
                identity: identity.clone(),
                observed_at_ns: 10,
                kind: CommandEvidenceKind::Proposed,
            },
            CommandEvidenceEvent {
                identity: identity.clone(),
                observed_at_ns: 11,
                kind: CommandEvidenceKind::RuntimeAdmitted,
            },
            CommandEvidenceEvent {
                identity: identity.clone(),
                observed_at_ns: 12,
                kind: CommandEvidenceKind::NativeAccepted { accepted_epoch: 1 },
            },
            CommandEvidenceEvent {
                identity: identity.clone(),
                observed_at_ns: 13,
                kind: CommandEvidenceKind::Applied { applied_epoch: 1 },
            },
            CommandEvidenceEvent {
                identity,
                observed_at_ns: 14,
                kind: CommandEvidenceKind::OutcomeRecorded {
                    outcome: CommandTerminalOutcome::Succeeded,
                },
            },
        ];
        let context = expected_receipt_context(policy, scenario, &trial);
        let receipt =
            create_command_evidence_receipt(context, &expected_commands, &events).unwrap();
        let signed_receipt = sign_command_evidence_receipt(
            &receipt,
            "independent-auditor-1",
            "trust-policy-v1",
            pair,
        )
        .unwrap();
        AttestedRecoveryTrialRecord {
            trial,
            expected_commands,
            events,
            signed_receipt,
        }
    }

    fn complete_campaign() -> (
        Vec<ExpectedRecoveryScenario>,
        Vec<AttestedRecoveryTrialRecord>,
        RecoveryCampaignEvidencePolicy,
        EvidenceTrustPolicy,
    ) {
        let expected = vec![
            scenario("nominal-a", 11, nominal_class()),
            scenario("fault-a", 11, faulted_class()),
        ];
        let policy = campaign_policy();
        let pair = key_pair();
        let trust = trust_policy(&pair);
        let trials = expected
            .iter()
            .flat_map(|scenario| {
                RecoveryBenchmarkArm::ALL
                    .into_iter()
                    .map(|arm| {
                        let trial = trial(scenario, arm);
                        attested_trial(scenario, trial, &policy, &pair)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        (expected, trials, policy, trust)
    }

    #[test]
    fn complete_campaign_requires_and_verifies_every_signed_row() {
        let (expected, trials, policy, trust) = complete_campaign();
        let summary =
            score_attested_recovery_campaign(&expected, &trials, &policy, &trust).unwrap();
        assert_eq!(summary.verified_trial_count, 8);
        assert_eq!(summary.recovery.expected_scenario_count, 2);
        assert_eq!(summary.recovery.arm_episode_count, 8);
        assert!(summary.evidence_root_digest.starts_with("sha256:"));
    }

    #[test]
    fn evidence_root_is_stable_under_trial_input_permutation() {
        let (expected, mut trials, policy, trust) = complete_campaign();
        let first =
            score_attested_recovery_campaign(&expected, &trials, &policy, &trust).unwrap();
        trials.reverse();
        let second =
            score_attested_recovery_campaign(&expected, &trials, &policy, &trust).unwrap();
        assert_eq!(first.evidence_root_digest, second.evidence_root_digest);
        assert_eq!(first.recovery, second.recovery);
    }

    #[test]
    fn missing_trial_evidence_fails_closed() {
        let (expected, mut trials, policy, trust) = complete_campaign();
        trials.pop();
        assert!(matches!(
            score_attested_recovery_campaign(&expected, &trials, &policy, &trust),
            Err(AttestedRecoveryCampaignError::MissingTrialEvidence { .. })
        ));
    }

    #[test]
    fn duplicate_trial_evidence_fails_closed() {
        let (expected, mut trials, policy, trust) = complete_campaign();
        trials.push(trials[0].clone());
        assert!(matches!(
            score_attested_recovery_campaign(&expected, &trials, &policy, &trust),
            Err(AttestedRecoveryCampaignError::DuplicateTrialEvidence { .. })
        ));
    }

    #[test]
    fn unexpected_trial_evidence_fails_closed() {
        let (expected, mut trials, policy, trust) = complete_campaign();
        trials[0].trial.scenario_id = "not-in-manifest".into();
        assert!(matches!(
            score_attested_recovery_campaign(&expected, &trials, &policy, &trust),
            Err(AttestedRecoveryCampaignError::UnexpectedTrialEvidence { .. })
        ));
    }

    #[test]
    fn mismatched_scenario_digest_context_fails_closed() {
        let (expected, mut trials, policy, trust) = complete_campaign();
        trials[0].signed_receipt.receipt.context.scenario_digest = "sha256:other-scenario".into();
        assert!(matches!(
            score_attested_recovery_campaign(&expected, &trials, &policy, &trust),
            Err(AttestedRecoveryCampaignError::ReceiptContextMismatch {
                field: "scenario_digest",
                ..
            })
        ));
    }

    #[test]
    fn corrupted_signature_fails_closed() {
        let (expected, mut trials, policy, trust) = complete_campaign();
        trials[0].signed_receipt.signature[0] ^= 0x01;
        assert!(matches!(
            score_attested_recovery_campaign(&expected, &trials, &policy, &trust),
            Err(AttestedRecoveryCampaignError::AttestationVerificationFailed { .. })
        ));
    }

    #[test]
    fn revoked_attestor_fails_closed_for_whole_campaign() {
        let (expected, trials, policy, mut trust) = complete_campaign();
        trust.trusted_signers[0].revoked = true;
        assert!(matches!(
            score_attested_recovery_campaign(&expected, &trials, &policy, &trust),
            Err(AttestedRecoveryCampaignError::AttestationVerificationFailed { .. })
        ));
    }

    #[test]
    fn every_arm_must_have_a_preregistered_policy_digest() {
        let (expected, trials, mut policy, trust) = complete_campaign();
        policy
            .policy_digest_by_arm
            .remove(&RecoveryBenchmarkArm::GuardedAdapter);
        assert_eq!(
            score_attested_recovery_campaign(&expected, &trials, &policy, &trust),
            Err(AttestedRecoveryCampaignError::MissingArmPolicyDigest {
                arm: RecoveryBenchmarkArm::GuardedAdapter,
            })
        );
    }
}
