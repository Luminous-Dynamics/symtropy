// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic, digest-bound receipts for independently audited command traces.
//!
//! A receipt commits to the campaign context, independently expected command
//! identities, the ordered evidence stream, and the audit summary. It is an
//! integrity commitment, not a signature, trusted timestamp, durable log, or
//! proof that an event was emitted by physical hardware.

use crate::command_evidence::{
    CommandEvidenceAudit, CommandEvidenceAuditError, CommandEvidenceEvent, CommandEvidenceKind,
    CommandRejectionStage, CommandTerminalOutcome, ExpectedCommandIdentity, audit_command_evidence,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Current version of the canonical receipt encoding.
pub const COMMAND_EVIDENCE_RECEIPT_SCHEMA_VERSION: u16 = 1;

/// Immutable run context supplied by the campaign orchestrator, independently
/// of the command event stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEvidenceReceiptContext {
    pub campaign_id: String,
    pub campaign_revision: String,
    /// Digest of the exact preregistered scenario-manifest bytes.
    pub scenario_manifest_digest: String,
    pub scenario_id: String,
    pub arm_id: String,
    pub arm_source_revision: String,
    /// Digest identifying the policy/configuration evaluated in this arm.
    pub policy_digest: String,
    /// Digest identifying the plant/model configuration.
    pub plant_digest: String,
    /// Digest identifying the native controller and safety-relevant settings.
    pub controller_config_digest: String,
    /// Stable identifier for the collector that produced this trace.
    pub trace_source_id: String,
    /// Version of the independent evaluator and canonical encoding contract.
    pub evaluator_version: String,
}

/// A deterministic commitment to one successfully audited command-evidence set.
///
/// Digests use the `sha256:<lowercase-hex>` form. The receipt deliberately has
/// no wall-clock creation time: identical inputs must produce identical output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEvidenceReceipt {
    pub schema_version: u16,
    pub context: CommandEvidenceReceiptContext,
    pub audit: CommandEvidenceAudit,
    pub context_digest: String,
    pub expected_identity_digest: String,
    pub event_stream_digest: String,
    pub audit_summary_digest: String,
    pub receipt_digest: String,
}

/// Receipt construction and verification failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEvidenceReceiptError {
    EmptyContextField { field: &'static str },
    ControllerConfigDigestMismatch { request_id: u64 },
    AuditFailed(CommandEvidenceAuditError),
    UnsupportedSchemaVersion { found: u16, supported: u16 },
    ReceiptMismatch,
}

impl From<CommandEvidenceAuditError> for CommandEvidenceReceiptError {
    fn from(value: CommandEvidenceAuditError) -> Self {
        Self::AuditFailed(value)
    }
}

/// Audit the trace and create a reproducible receipt bound to its campaign
/// context and exact ordered evidence input.
///
/// Expected identities are canonicalized by request ID because their input
/// slice order is not semantically meaningful. Evidence events retain their
/// original global order because interleaving is part of the captured trace.
/// Each digest uses a versioned domain separator and length-prefixed fields.
pub fn create_command_evidence_receipt(
    context: CommandEvidenceReceiptContext,
    expected: &[ExpectedCommandIdentity],
    events: &[CommandEvidenceEvent],
) -> Result<CommandEvidenceReceipt, CommandEvidenceReceiptError> {
    validate_context(&context)?;
    let audit = audit_command_evidence(expected, events)?;
    for identity in expected {
        if identity.controller_config_digest != context.controller_config_digest {
            return Err(
                CommandEvidenceReceiptError::ControllerConfigDigestMismatch {
                    request_id: identity.request_id,
                },
            );
        }
    }

    let context_digest = digest_context(&context);
    let expected_identity_digest = digest_expected_identities(expected);
    let event_stream_digest = digest_event_stream(events);
    let audit_summary_digest = digest_audit_summary(&audit);
    let receipt_digest = digest_receipt(
        COMMAND_EVIDENCE_RECEIPT_SCHEMA_VERSION,
        &context_digest,
        &expected_identity_digest,
        &event_stream_digest,
        &audit_summary_digest,
    );

    Ok(CommandEvidenceReceipt {
        schema_version: COMMAND_EVIDENCE_RECEIPT_SCHEMA_VERSION,
        context,
        audit,
        context_digest,
        expected_identity_digest,
        event_stream_digest,
        audit_summary_digest,
        receipt_digest,
    })
}

/// Re-run the auditor and canonical digest construction, then compare every
/// receipt field. A mismatch is fail-closed; callers must not trust the stored
/// summary without successful verification against the supplied original inputs.
pub fn verify_command_evidence_receipt(
    receipt: &CommandEvidenceReceipt,
    context: &CommandEvidenceReceiptContext,
    expected: &[ExpectedCommandIdentity],
    events: &[CommandEvidenceEvent],
) -> Result<(), CommandEvidenceReceiptError> {
    if receipt.schema_version != COMMAND_EVIDENCE_RECEIPT_SCHEMA_VERSION {
        return Err(CommandEvidenceReceiptError::UnsupportedSchemaVersion {
            found: receipt.schema_version,
            supported: COMMAND_EVIDENCE_RECEIPT_SCHEMA_VERSION,
        });
    }

    let rebuilt = create_command_evidence_receipt(context.clone(), expected, events)?;
    if &rebuilt != receipt {
        return Err(CommandEvidenceReceiptError::ReceiptMismatch);
    }
    Ok(())
}

fn validate_context(
    context: &CommandEvidenceReceiptContext,
) -> Result<(), CommandEvidenceReceiptError> {
    let fields = [
        ("campaign_id", context.campaign_id.as_str()),
        ("campaign_revision", context.campaign_revision.as_str()),
        (
            "scenario_manifest_digest",
            context.scenario_manifest_digest.as_str(),
        ),
        ("scenario_id", context.scenario_id.as_str()),
        ("arm_id", context.arm_id.as_str()),
        ("arm_source_revision", context.arm_source_revision.as_str()),
        ("policy_digest", context.policy_digest.as_str()),
        ("plant_digest", context.plant_digest.as_str()),
        (
            "controller_config_digest",
            context.controller_config_digest.as_str(),
        ),
        ("trace_source_id", context.trace_source_id.as_str()),
        ("evaluator_version", context.evaluator_version.as_str()),
    ];

    for (field, value) in fields {
        if value.trim().is_empty() {
            return Err(CommandEvidenceReceiptError::EmptyContextField { field });
        }
    }
    Ok(())
}

struct CanonicalEncoder {
    hasher: Sha256,
}

impl CanonicalEncoder {
    fn new(domain: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(domain);
        hasher.update([0]);
        Self { hasher }
    }

    fn tag(&mut self, value: u8) {
        self.hasher.update([value]);
    }

    fn u16(&mut self, value: u16) {
        self.hasher.update(value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.hasher.update(value.to_le_bytes());
    }

    fn string(&mut self, value: &str) {
        self.u64(value.len() as u64);
        self.hasher.update(value.as_bytes());
    }

    fn finish(self) -> String {
        let digest = self.hasher.finalize();
        let mut encoded = String::with_capacity(7 + digest.len() * 2);
        encoded.push_str("sha256:");
        use std::fmt::Write as _;
        for byte in digest {
            let _ = write!(encoded, "{byte:02x}");
        }
        encoded
    }
}

fn digest_encoded(encoder: CanonicalEncoder) -> String {
    encoder.finish()
}

fn encode_context(encoder: &mut CanonicalEncoder, context: &CommandEvidenceReceiptContext) {
    encoder.string(&context.campaign_id);
    encoder.string(&context.campaign_revision);
    encoder.string(&context.scenario_manifest_digest);
    encoder.string(&context.scenario_id);
    encoder.string(&context.arm_id);
    encoder.string(&context.arm_source_revision);
    encoder.string(&context.policy_digest);
    encoder.string(&context.plant_digest);
    encoder.string(&context.controller_config_digest);
    encoder.string(&context.trace_source_id);
    encoder.string(&context.evaluator_version);
}

fn digest_context(context: &CommandEvidenceReceiptContext) -> String {
    let mut encoder = CanonicalEncoder::new(b"symtropy.command-evidence.receipt.context.v1");
    encode_context(&mut encoder, context);
    digest_encoded(encoder)
}

fn encode_identity(encoder: &mut CanonicalEncoder, identity: &ExpectedCommandIdentity) {
    encoder.u64(identity.request_id);
    encoder.u64(identity.session_id);
    encoder.u64(identity.sequence);
    encoder.string(&identity.controller_id);
    encoder.u64(identity.controller_epoch);
    encoder.string(&identity.command_payload_digest);
    encoder.string(&identity.controller_config_digest);
    encoder.u64(identity.deadline_ns);
    encoder.string(&identity.clock_domain_id);
}

fn digest_expected_identities(expected: &[ExpectedCommandIdentity]) -> String {
    let mut sorted = expected.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|identity| identity.request_id);

    let mut encoder = CanonicalEncoder::new(b"symtropy.command-evidence.receipt.expected.v1");
    encoder.u64(sorted.len() as u64);
    for identity in sorted {
        encode_identity(&mut encoder, identity);
    }
    digest_encoded(encoder)
}

fn encode_event_kind(encoder: &mut CanonicalEncoder, kind: &CommandEvidenceKind) {
    match kind {
        CommandEvidenceKind::Proposed => encoder.tag(0),
        CommandEvidenceKind::RuntimeAdmitted => encoder.tag(1),
        CommandEvidenceKind::NativeAccepted { accepted_epoch } => {
            encoder.tag(2);
            encoder.u64(*accepted_epoch);
        }
        CommandEvidenceKind::Applied { applied_epoch } => {
            encoder.tag(3);
            encoder.u64(*applied_epoch);
        }
        CommandEvidenceKind::Rejected { stage } => {
            encoder.tag(4);
            encoder.tag(match stage {
                CommandRejectionStage::RuntimeAdmission => 0,
                CommandRejectionStage::NativeAdmission => 1,
                CommandRejectionStage::Application => 2,
            });
        }
        CommandEvidenceKind::Cancelled => encoder.tag(5),
        CommandEvidenceKind::OutcomeRecorded { outcome } => {
            encoder.tag(6);
            encoder.tag(match outcome {
                CommandTerminalOutcome::Succeeded => 0,
                CommandTerminalOutcome::Failed => 1,
                CommandTerminalOutcome::Rejected => 2,
                CommandTerminalOutcome::Cancelled => 3,
                CommandTerminalOutcome::TimedOut => 4,
            });
        }
    }
}

fn digest_event_stream(events: &[CommandEvidenceEvent]) -> String {
    let mut encoder = CanonicalEncoder::new(b"symtropy.command-evidence.receipt.events.v1");
    encoder.u64(events.len() as u64);
    for event in events {
        encode_identity(&mut encoder, &event.identity);
        encoder.u64(event.observed_at_ns);
        encode_event_kind(&mut encoder, &event.kind);
    }
    digest_encoded(encoder)
}

fn digest_audit_summary(audit: &CommandEvidenceAudit) -> String {
    let mut encoder = CanonicalEncoder::new(b"symtropy.command-evidence.receipt.audit.v1");
    encoder.u64(audit.command_count as u64);
    encoder.u64(audit.runtime_admissions as u64);
    encoder.u64(audit.native_acceptances as u64);
    encoder.u64(audit.applications as u64);
    encoder.u64(audit.rejected_commands as u64);
    encoder.u64(audit.cancellations as u64);
    encoder.u64(audit.successful_outcomes as u64);
    encoder.u64(audit.failed_outcomes as u64);
    encoder.u64(audit.timed_out_outcomes as u64);
    digest_encoded(encoder)
}

fn digest_receipt(
    schema_version: u16,
    context_digest: &str,
    expected_identity_digest: &str,
    event_stream_digest: &str,
    audit_summary_digest: &str,
) -> String {
    let mut encoder = CanonicalEncoder::new(b"symtropy.command-evidence.receipt.root.v1");
    encoder.u16(schema_version);
    encoder.string(context_digest);
    encoder.string(expected_identity_digest);
    encoder.string(event_stream_digest);
    encoder.string(audit_summary_digest);
    digest_encoded(encoder)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> CommandEvidenceReceiptContext {
        CommandEvidenceReceiptContext {
            campaign_id: "recovery-campaign-2026-10".into(),
            campaign_revision: "campaign-rev-7".into(),
            scenario_manifest_digest: "sha256:manifest".into(),
            scenario_id: "fault-network-delay-sev2-seed19".into(),
            arm_id: "B3_guarded_outcome_feedback".into(),
            arm_source_revision: "git:0123456789abcdef".into(),
            policy_digest: "sha256:policy".into(),
            plant_digest: "sha256:plant".into(),
            controller_config_digest: "sha256:controller".into(),
            trace_source_id: "sim-event-collector-v1".into(),
            evaluator_version: "command-auditor-v0.1".into(),
        }
    }

    fn identity(request_id: u64) -> ExpectedCommandIdentity {
        ExpectedCommandIdentity {
            request_id,
            session_id: 7,
            sequence: request_id,
            controller_id: "mobile-base-01".into(),
            controller_epoch: 4,
            command_payload_digest: format!("sha256:command-{request_id}"),
            controller_config_digest: "sha256:controller".into(),
            deadline_ns: 1_000,
            clock_domain_id: "robot-monotonic-0".into(),
        }
    }

    fn trace(identity: &ExpectedCommandIdentity, offset: u64) -> Vec<CommandEvidenceEvent> {
        vec![
            CommandEvidenceEvent {
                identity: identity.clone(),
                observed_at_ns: offset + 1,
                kind: CommandEvidenceKind::Proposed,
            },
            CommandEvidenceEvent {
                identity: identity.clone(),
                observed_at_ns: offset + 2,
                kind: CommandEvidenceKind::RuntimeAdmitted,
            },
            CommandEvidenceEvent {
                identity: identity.clone(),
                observed_at_ns: offset + 3,
                kind: CommandEvidenceKind::NativeAccepted {
                    accepted_epoch: identity.controller_epoch,
                },
            },
            CommandEvidenceEvent {
                identity: identity.clone(),
                observed_at_ns: offset + 4,
                kind: CommandEvidenceKind::Applied {
                    applied_epoch: identity.controller_epoch,
                },
            },
            CommandEvidenceEvent {
                identity: identity.clone(),
                observed_at_ns: offset + 5,
                kind: CommandEvidenceKind::OutcomeRecorded {
                    outcome: CommandTerminalOutcome::Succeeded,
                },
            },
        ]
    }

    #[test]
    fn receipt_is_deterministic_when_expected_identity_order_changes() {
        let first = identity(11);
        let second = identity(12);
        let expected = [first.clone(), second.clone()];
        let events = [trace(&first, 10), trace(&second, 20)].concat();

        let receipt_a = create_command_evidence_receipt(context(), &expected, &events).unwrap();
        let receipt_b =
            create_command_evidence_receipt(context(), &[second, first], &events).unwrap();

        assert_eq!(receipt_a, receipt_b);
        assert!(receipt_a.receipt_digest.starts_with("sha256:"));
        assert_eq!(receipt_a.audit.command_count, 2);
    }

    #[test]
    fn receipt_commits_to_global_event_interleaving() {
        let first = identity(11);
        let second = identity(12);
        let first_trace = trace(&first, 10);
        let second_trace = trace(&second, 20);
        let expected = [first, second];

        let grouped = [first_trace.clone(), second_trace.clone()].concat();
        let interleaved = (0..first_trace.len())
            .flat_map(|index| [first_trace[index].clone(), second_trace[index].clone()])
            .collect::<Vec<_>>();

        let grouped_receipt =
            create_command_evidence_receipt(context(), &expected, &grouped).unwrap();
        let interleaved_receipt =
            create_command_evidence_receipt(context(), &expected, &interleaved).unwrap();

        assert_ne!(
            grouped_receipt.event_stream_digest,
            interleaved_receipt.event_stream_digest
        );
        assert_ne!(
            grouped_receipt.receipt_digest,
            interleaved_receipt.receipt_digest
        );
    }

    #[test]
    fn receipt_is_bound_to_campaign_scenario_and_policy_context() {
        let identity = identity(11);
        let expected = [identity.clone()];
        let events = trace(&identity, 10);
        let original = create_command_evidence_receipt(context(), &expected, &events).unwrap();

        let mut changed_context = context();
        changed_context.policy_digest = "sha256:different-policy".into();
        let changed = create_command_evidence_receipt(changed_context, &expected, &events).unwrap();

        assert_ne!(original.context_digest, changed.context_digest);
        assert_ne!(original.receipt_digest, changed.receipt_digest);
    }

    #[test]
    fn verifier_recomputes_the_receipt_from_original_inputs() {
        let identity = identity(11);
        let expected = [identity.clone()];
        let events = trace(&identity, 10);
        let receipt = create_command_evidence_receipt(context(), &expected, &events).unwrap();

        assert_eq!(
            verify_command_evidence_receipt(&receipt, &context(), &expected, &events),
            Ok(())
        );

        let mut changed_events = events.clone();
        changed_events[0].observed_at_ns += 1;
        assert_eq!(
            verify_command_evidence_receipt(&receipt, &context(), &expected, &changed_events),
            Err(CommandEvidenceReceiptError::ReceiptMismatch)
        );
    }

    #[test]
    fn unsupported_receipt_schema_fails_closed() {
        let identity = identity(11);
        let expected = [identity.clone()];
        let events = trace(&identity, 10);
        let mut receipt = create_command_evidence_receipt(context(), &expected, &events).unwrap();
        receipt.schema_version = 99;

        assert_eq!(
            verify_command_evidence_receipt(&receipt, &context(), &expected, &events),
            Err(CommandEvidenceReceiptError::UnsupportedSchemaVersion {
                found: 99,
                supported: COMMAND_EVIDENCE_RECEIPT_SCHEMA_VERSION,
            })
        );
    }

    #[test]
    fn empty_context_field_is_rejected() {
        let identity = identity(11);
        let expected = [identity.clone()];
        let events = trace(&identity, 10);
        let mut invalid_context = context();
        invalid_context.trace_source_id = " ".into();

        assert_eq!(
            create_command_evidence_receipt(invalid_context, &expected, &events),
            Err(CommandEvidenceReceiptError::EmptyContextField {
                field: "trace_source_id",
            })
        );
    }

    #[test]
    fn outer_controller_configuration_must_match_each_expected_identity() {
        let identity = identity(11);
        let expected = [identity.clone()];
        let events = trace(&identity, 10);
        let mut mismatched_context = context();
        mismatched_context.controller_config_digest = "sha256:another-controller".into();

        assert_eq!(
            create_command_evidence_receipt(mismatched_context, &expected, &events),
            Err(
                CommandEvidenceReceiptError::ControllerConfigDigestMismatch {
                    request_id: identity.request_id,
                }
            )
        );
    }

    #[test]
    fn invalid_command_evidence_never_produces_a_receipt() {
        let identity = identity(11);
        let expected = [identity.clone()];
        let mut events = trace(&identity, 10);
        events[2].kind = CommandEvidenceKind::NativeAccepted { accepted_epoch: 3 };

        assert!(matches!(
            create_command_evidence_receipt(context(), &expected, &events),
            Err(CommandEvidenceReceiptError::AuditFailed(
                CommandEvidenceAuditError::StaleNativeEpochAccepted { .. }
            ))
        ));
    }
}
