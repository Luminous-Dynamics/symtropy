// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Ed25519 attestation and explicit trust-policy verification for audit receipts.
//!
//! A valid signature authenticates the configured receipt attestor's key. It
//! does not prove that the attestor observed the physical robot, that the trace
//! collector was honest, or that an Applied event corresponds to motor motion.
//! Key custody, role assignment, revocation, and policy distribution remain
//! deployment responsibilities.

use crate::audit_receipt::{
    CommandEvidenceReceipt, CommandEvidenceReceiptContext, CommandEvidenceReceiptError,
    verify_command_evidence_receipt,
};
use crate::command_evidence::{CommandEvidenceEvent, ExpectedCommandIdentity};
use ring::signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey};
use serde::{Deserialize, Serialize};

/// Roles in the external evidence trust policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSignerRole {
    /// May attest that an independent verifier processed a receipt and its inputs.
    IndependentReceiptAttestor,
    /// Identifies the component that collected events; it is not an auditor authority.
    TraceCollector,
    /// May attest to a separate hardware-witness statement, not an audit receipt.
    HardwareApplicationWitness,
}

/// One key entry distributed through an out-of-band trust policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedEvidenceSigner {
    pub key_id: String,
    /// Raw 32-byte Ed25519 public key.
    pub public_key: Vec<u8>,
    pub role: EvidenceSignerRole,
    pub revoked: bool,
}

/// Versioned trust-policy snapshot used to authorize receipt attestors.
///
/// The caller must obtain this policy from a trusted administrative channel;
/// a policy bundled beside an untrusted receipt is not an independent trust root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceTrustPolicy {
    pub policy_revision: String,
    pub trusted_signers: Vec<TrustedEvidenceSigner>,
}

/// Receipt plus an Ed25519 signature over its schema, digest, signer ID, and
/// the trust-policy revision under which the attestation was issued.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedCommandEvidenceReceipt {
    pub receipt: CommandEvidenceReceipt,
    pub trust_policy_revision: String,
    pub signer_key_id: String,
    pub signer_public_key: Vec<u8>,
    /// Raw 64-byte Ed25519 signature.
    pub signature: Vec<u8>,
}

/// Fail-closed errors for signing and trust-policy verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEvidenceAttestationError {
    EmptySignerKeyId,
    EmptyTrustPolicyRevision,
    InvalidReceiptDigest,
    UnsupportedReceiptSchemaVersion { found: u16, supported: u16 },
    DuplicateTrustedKeyId { key_id: String },
    UnknownSigner { key_id: String },
    RevokedSigner { key_id: String },
    WrongSignerRole { key_id: String, role: EvidenceSignerRole },
    InvalidTrustedPublicKeyLength { key_id: String, length: usize },
    InvalidSignaturePublicKeyLength { length: usize },
    InvalidSignatureLength { length: usize },
    TrustedKeyMismatch { key_id: String },
    SignatureInvalid,
    TrustPolicyRevisionMismatch { receipt: String, trusted: String },
    ReceiptVerificationFailed(CommandEvidenceReceiptError),
}

impl From<CommandEvidenceReceiptError> for CommandEvidenceAttestationError {
    fn from(value: CommandEvidenceReceiptError) -> Self {
        Self::ReceiptVerificationFailed(value)
    }
}

/// Sign a previously constructed receipt with a deployment-managed Ed25519 key.
///
/// This function intentionally does not load keys from disk, environment
/// variables, or a network service. The caller owns secure key custody and
/// passes an already initialized key pair from an appropriate secure boundary.
pub fn sign_command_evidence_receipt(
    receipt: &CommandEvidenceReceipt,
    signer_key_id: &str,
    trust_policy_revision: &str,
    signing_key: &Ed25519KeyPair,
) -> Result<SignedCommandEvidenceReceipt, CommandEvidenceAttestationError> {
    validate_signing_fields(receipt, signer_key_id, trust_policy_revision)?;
    let payload = signature_payload(
        receipt.schema_version,
        signer_key_id,
        trust_policy_revision,
        &receipt.receipt_digest,
    );
    let signature = signing_key.sign(&payload);

    Ok(SignedCommandEvidenceReceipt {
        receipt: receipt.clone(),
        trust_policy_revision: trust_policy_revision.to_owned(),
        signer_key_id: signer_key_id.to_owned(),
        signer_public_key: signing_key.public_key().as_ref().to_vec(),
        signature: signature.as_ref().to_vec(),
    })
}

/// Verify signer authorization, signature, and the original evidence receipt.
///
/// The caller supplies the trusted policy independently. A matching key ID is
/// insufficient: the key bytes must match, the key must be unrevoked, and its
/// role must be IndependentReceiptAttestor. The receipt is then recomputed
/// against the original context, expected identities, and raw ordered events.
pub fn verify_signed_command_evidence_receipt(
    signed: &SignedCommandEvidenceReceipt,
    context: &CommandEvidenceReceiptContext,
    expected: &[ExpectedCommandIdentity],
    events: &[CommandEvidenceEvent],
    trust_policy: &EvidenceTrustPolicy,
) -> Result<(), CommandEvidenceAttestationError> {
    if signed.signer_key_id.trim().is_empty() {
        return Err(CommandEvidenceAttestationError::EmptySignerKeyId);
    }
    if trust_policy.policy_revision.trim().is_empty() {
        return Err(CommandEvidenceAttestationError::EmptyTrustPolicyRevision);
    }
    if signed.trust_policy_revision != trust_policy.policy_revision {
        return Err(CommandEvidenceAttestationError::TrustPolicyRevisionMismatch {
            receipt: signed.trust_policy_revision.clone(),
            trusted: trust_policy.policy_revision.clone(),
        });
    }
    if signed.receipt.schema_version
        != crate::audit_receipt::COMMAND_EVIDENCE_RECEIPT_SCHEMA_VERSION
    {
        return Err(
            CommandEvidenceAttestationError::UnsupportedReceiptSchemaVersion {
                found: signed.receipt.schema_version,
                supported: crate::audit_receipt::COMMAND_EVIDENCE_RECEIPT_SCHEMA_VERSION,
            },
        );
    }
    validate_receipt_digest(&signed.receipt.receipt_digest)?;

    let mut matching = trust_policy
        .trusted_signers
        .iter()
        .filter(|entry| entry.key_id == signed.signer_key_id);
    let trusted = matching
        .next()
        .ok_or_else(|| CommandEvidenceAttestationError::UnknownSigner {
            key_id: signed.signer_key_id.clone(),
        })?;
    if matching.next().is_some() {
        return Err(CommandEvidenceAttestationError::DuplicateTrustedKeyId {
            key_id: signed.signer_key_id.clone(),
        });
    }
    if trusted.revoked {
        return Err(CommandEvidenceAttestationError::RevokedSigner {
            key_id: signed.signer_key_id.clone(),
        });
    }
    if trusted.role != EvidenceSignerRole::IndependentReceiptAttestor {
        return Err(CommandEvidenceAttestationError::WrongSignerRole {
            key_id: signed.signer_key_id.clone(),
            role: trusted.role,
        });
    }
    if trusted.public_key.len() != 32 {
        return Err(
            CommandEvidenceAttestationError::InvalidTrustedPublicKeyLength {
                key_id: trusted.key_id.clone(),
                length: trusted.public_key.len(),
            },
        );
    }
    if signed.signer_public_key.len() != 32 {
        return Err(
            CommandEvidenceAttestationError::InvalidSignaturePublicKeyLength {
                length: signed.signer_public_key.len(),
            },
        );
    }
    if signed.signature.len() != 64 {
        return Err(CommandEvidenceAttestationError::InvalidSignatureLength {
            length: signed.signature.len(),
        });
    }
    if signed.signer_public_key != trusted.public_key {
        return Err(CommandEvidenceAttestationError::TrustedKeyMismatch {
            key_id: trusted.key_id.clone(),
        });
    }

    let payload = signature_payload(
        signed.receipt.schema_version,
        &signed.signer_key_id,
        &signed.trust_policy_revision,
        &signed.receipt.receipt_digest,
    );
    UnparsedPublicKey::new(&ED25519, &trusted.public_key)
        .verify(&payload, &signed.signature)
        .map_err(|_| CommandEvidenceAttestationError::SignatureInvalid)?;

    verify_command_evidence_receipt(&signed.receipt, context, expected, events)?;
    Ok(())
}

fn validate_signing_fields(
    receipt: &CommandEvidenceReceipt,
    signer_key_id: &str,
    trust_policy_revision: &str,
) -> Result<(), CommandEvidenceAttestationError> {
    if signer_key_id.trim().is_empty() {
        return Err(CommandEvidenceAttestationError::EmptySignerKeyId);
    }
    if trust_policy_revision.trim().is_empty() {
        return Err(CommandEvidenceAttestationError::EmptyTrustPolicyRevision);
    }
    if receipt.schema_version != crate::audit_receipt::COMMAND_EVIDENCE_RECEIPT_SCHEMA_VERSION {
        return Err(
            CommandEvidenceAttestationError::UnsupportedReceiptSchemaVersion {
                found: receipt.schema_version,
                supported: crate::audit_receipt::COMMAND_EVIDENCE_RECEIPT_SCHEMA_VERSION,
            },
        );
    }
    validate_receipt_digest(&receipt.receipt_digest)
}

fn validate_receipt_digest(digest: &str) -> Result<(), CommandEvidenceAttestationError> {
    let bytes = digest.as_bytes();
    if bytes.len() != 71
        || !bytes.starts_with(b"sha256:")
        || !bytes[7..].iter().all(u8::is_ascii_hexdigit)
    {
        return Err(CommandEvidenceAttestationError::InvalidReceiptDigest);
    }
    Ok(())
}

fn signature_payload(
    schema_version: u16,
    signer_key_id: &str,
    trust_policy_revision: &str,
    receipt_digest: &str,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(
        42 + signer_key_id.len() + trust_policy_revision.len() + receipt_digest.len(),
    );
    payload.extend_from_slice(b"symtropy.command-evidence.attestation.ed25519.v1");
    payload.push(0);
    payload.extend_from_slice(&schema_version.to_le_bytes());
    append_len_prefixed(&mut payload, signer_key_id.as_bytes());
    append_len_prefixed(&mut payload, trust_policy_revision.as_bytes());
    append_len_prefixed(&mut payload, receipt_digest.as_bytes());
    payload
}

fn append_len_prefixed(target: &mut Vec<u8>, bytes: &[u8]) {
    target.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    target.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit_receipt::{
        CommandEvidenceReceiptContext, create_command_evidence_receipt,
    };
    use crate::command_evidence::{
        CommandEvidenceEvent, CommandEvidenceKind, CommandTerminalOutcome,
    };

    fn key_pair(seed: u8) -> Ed25519KeyPair {
        Ed25519KeyPair::from_seed_unchecked(&[seed; 32]).unwrap()
    }

    fn fixture() -> (
        CommandEvidenceReceiptContext,
        Vec<ExpectedCommandIdentity>,
        Vec<CommandEvidenceEvent>,
        CommandEvidenceReceipt,
    ) {
        let context = CommandEvidenceReceiptContext {
            campaign_id: "attestation-fixture".into(),
            campaign_revision: "rev-1".into(),
            scenario_manifest_digest: "sha256:manifest".into(),
            scenario_id: "scenario-1".into(),
            scenario_digest: "sha256:scenario".into(),
            scenario_digest: "sha256:scenario".into(),
            arm_id: "B3_guarded_outcome_feedback".into(),
            arm_source_revision: "git:fixture".into(),
            policy_digest: "sha256:policy".into(),
            task_profile_digest: "sha256:profile".into(),
            task_profile_digest: "sha256:profile".into(),
            plant_digest: "sha256:plant".into(),
            controller_config_digest: "sha256:controller".into(),
            trace_source_id: "collector-1".into(),
            evaluator_version: "auditor-1".into(),
        };
        let identity = ExpectedCommandIdentity {
            request_id: 1,
            session_id: 1,
            sequence: 1,
            controller_id: "base-1".into(),
            controller_epoch: 1,
            command_payload_digest: "sha256:command".into(),
            controller_config_digest: "sha256:controller".into(),
            deadline_ns: 1_000,
            clock_domain_id: "mono-1".into(),
        };
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
                identity: identity.clone(),
                observed_at_ns: 14,
                kind: CommandEvidenceKind::OutcomeRecorded {
                    outcome: CommandTerminalOutcome::Succeeded,
                },
            },
        ];
        let expected = vec![identity];
        let receipt =
            create_command_evidence_receipt(context.clone(), &expected, &events).unwrap();
        (context, expected, events, receipt)
    }

    fn policy(pair: &Ed25519KeyPair) -> EvidenceTrustPolicy {
        EvidenceTrustPolicy {
            policy_revision: "trust-policy-v1".into(),
            trusted_signers: vec![TrustedEvidenceSigner {
                key_id: "independent-auditor-2026".into(),
                public_key: pair.public_key().as_ref().to_vec(),
                role: EvidenceSignerRole::IndependentReceiptAttestor,
                revoked: false,
            }],
        }
    }

    fn sign(
        receipt: &CommandEvidenceReceipt,
        pair: &Ed25519KeyPair,
    ) -> SignedCommandEvidenceReceipt {
        sign_command_evidence_receipt(
            receipt,
            "independent-auditor-2026",
            "trust-policy-v1",
            pair,
        )
        .unwrap()
    }

    #[test]
    fn trusted_independent_attestor_signature_and_receipt_verify() {
        let (context, expected, events, receipt) = fixture();
        let pair = key_pair(7);
        let signed = sign(&receipt, &pair);

        assert_eq!(
            verify_signed_command_evidence_receipt(
                &signed,
                &context,
                &expected,
                &events,
                &policy(&pair),
            ),
            Ok(())
        );
    }

    #[test]
    fn unknown_signer_is_rejected() {
        let (context, expected, events, receipt) = fixture();
        let pair = key_pair(7);
        let signed = sign(&receipt, &pair);
        let mut trust = policy(&pair);
        trust.trusted_signers.clear();

        assert_eq!(
            verify_signed_command_evidence_receipt(
                &signed,
                &context,
                &expected,
                &events,
                &trust,
            ),
            Err(CommandEvidenceAttestationError::UnknownSigner {
                key_id: "independent-auditor-2026".into(),
            })
        );
    }

    #[test]
    fn revoked_attestor_is_rejected() {
        let (context, expected, events, receipt) = fixture();
        let pair = key_pair(7);
        let signed = sign(&receipt, &pair);
        let mut trust = policy(&pair);
        trust.trusted_signers[0].revoked = true;

        assert_eq!(
            verify_signed_command_evidence_receipt(&signed, &context, &expected, &events, &trust),
            Err(CommandEvidenceAttestationError::RevokedSigner {
                key_id: "independent-auditor-2026".into(),
            })
        );
    }

    #[test]
    fn collector_key_cannot_be_used_as_an_independent_attestor() {
        let (context, expected, events, receipt) = fixture();
        let pair = key_pair(7);
        let signed = sign(&receipt, &pair);
        let mut trust = policy(&pair);
        trust.trusted_signers[0].role = EvidenceSignerRole::TraceCollector;

        assert_eq!(
            verify_signed_command_evidence_receipt(&signed, &context, &expected, &events, &trust),
            Err(CommandEvidenceAttestationError::WrongSignerRole {
                key_id: "independent-auditor-2026".into(),
                role: EvidenceSignerRole::TraceCollector,
            })
        );
    }

    #[test]
    fn mismatched_public_key_for_trusted_key_id_is_rejected() {
        let (context, expected, events, receipt) = fixture();
        let signer = key_pair(7);
        let different = key_pair(8);
        let signed = sign(&receipt, &signer);
        let trust = policy(&different);

        assert_eq!(
            verify_signed_command_evidence_receipt(&signed, &context, &expected, &events, &trust),
            Err(CommandEvidenceAttestationError::TrustedKeyMismatch {
                key_id: "independent-auditor-2026".into(),
            })
        );
    }

    #[test]
    fn signature_cannot_be_replayed_under_another_trust_policy_revision() {
        let (context, expected, events, receipt) = fixture();
        let pair = key_pair(7);
        let signed = sign(&receipt, &pair);
        let mut trust = policy(&pair);
        trust.policy_revision = "trust-policy-v2".into();

        assert_eq!(
            verify_signed_command_evidence_receipt(&signed, &context, &expected, &events, &trust),
            Err(CommandEvidenceAttestationError::TrustPolicyRevisionMismatch {
                receipt: "trust-policy-v1".into(),
                trusted: "trust-policy-v2".into(),
            })
        );
    }

    #[test]
    fn signature_binds_receipt_digest_and_signer_identity() {
        let (context, expected, events, receipt) = fixture();
        let pair = key_pair(7);
        let mut signed = sign(&receipt, &pair);
        signed.receipt.receipt_digest = format!("sha256:{}", "0".repeat(64));

        assert_eq!(
            verify_signed_command_evidence_receipt(
                &signed,
                &context,
                &expected,
                &events,
                &policy(&pair),
            ),
            Err(CommandEvidenceAttestationError::SignatureInvalid)
        );
    }

    #[test]
    fn signed_receipt_still_recomputes_original_evidence() {
        let (context, expected, events, receipt) = fixture();
        let pair = key_pair(7);
        let signed = sign(&receipt, &pair);
        let mut changed_events = events.clone();
        changed_events[0].observed_at_ns += 1;

        assert_eq!(
            verify_signed_command_evidence_receipt(
                &signed,
                &context,
                &expected,
                &changed_events,
                &policy(&pair),
            ),
            Err(CommandEvidenceAttestationError::ReceiptVerificationFailed(
                CommandEvidenceReceiptError::ReceiptMismatch,
            ))
        );
    }

    #[test]
    fn duplicate_trusted_key_id_fails_closed() {
        let (context, expected, events, receipt) = fixture();
        let pair = key_pair(7);
        let signed = sign(&receipt, &pair);
        let mut trust = policy(&pair);
        trust.trusted_signers.push(trust.trusted_signers[0].clone());

        assert_eq!(
            verify_signed_command_evidence_receipt(&signed, &context, &expected, &events, &trust),
            Err(CommandEvidenceAttestationError::DuplicateTrustedKeyId {
                key_id: "independent-auditor-2026".into(),
            })
        );
    }

    #[test]
    fn malformed_signature_length_fails_closed() {
        let (context, expected, events, receipt) = fixture();
        let pair = key_pair(7);
        let mut signed = sign(&receipt, &pair);
        signed.signature.pop();

        assert_eq!(
            verify_signed_command_evidence_receipt(
                &signed,
                &context,
                &expected,
                &events,
                &policy(&pair),
            ),
            Err(CommandEvidenceAttestationError::InvalidSignatureLength { length: 63 })
        );
    }
}
