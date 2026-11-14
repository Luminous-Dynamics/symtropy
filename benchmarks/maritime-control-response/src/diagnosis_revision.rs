#![forbid(unsafe_code)]

//! Append-only diagnostic claim revision with evidence-driven current-state updates.
//!
//! Historical diagnosis receipts are immutable. New, time-available evidence may
//! create a new current disposition, including an explicit withdrawal, without
//! rewriting the historical prediction. This module validates structure,
//! chronology, declared transition reasons and receipt replay; it does not
//! authenticate evidence issuers or establish physical truth.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const DIAGNOSIS_REVISION_SCHEMA: &str = "symtropy-diagnostic-revision-v0.1";
pub const DIAGNOSIS_REVISION_RECEIPT_SCHEMA: &str = "symtropy-diagnostic-revision-receipt-v0.1";
const TIME_TOLERANCE_SECONDS: f64 = 1e-9;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RevisionEvidenceRole {
    SensorMeasurement,
    ActuatorFeedback,
    ControlIntent,
    DisturbanceContext,
    DerivedResidual,
    OperatorObservation,
    MaintenanceRecord,
    CalibrationRecord,
    ConfigurationRecord,
    IndependentAssessment,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RevisionEvidenceEffect {
    SupportsCurrent,
    ContradictsCurrent,
    NarrowsHypothesisSet,
    ExpandsAmbiguity,
    ContextShift,
    InvalidatesPriorEvidence,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RevisionEvidenceReference {
    /// SHA-256 of the canonical evidence bytes, not a hash of this wrapper.
    pub evidence_sha256: String,
    pub source_id: String,
    pub role: RevisionEvidenceRole,
    /// Time the physical/recorded observation occurred.
    pub observed_at_seconds: f64,
    /// Time the evidence actually became available to the diagnoser.
    pub available_at_seconds: f64,
    /// Upstream semantic appraisal; this module checks consistency, not truth.
    pub declared_effect: RevisionEvidenceEffect,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RevisionEvidenceInvalidation {
    /// An evidence digest that was active in an earlier revision.
    pub invalidated_evidence_sha256: String,
    /// Digest of newly added evidence explaining why the prior item is invalid.
    pub basis_evidence_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiagnosticDisposition {
    NoFault,
    KnownHypotheses { hypothesis_ids: Vec<String> },
    UnknownOrOutOfSupport,
    NotEvaluable,
    Withdrawn,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticRevisionKind {
    InitialAssessment,
    Corroboration,
    Reassessment,
    EvidenceRetraction,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRevisionBinding {
    pub run_id: String,
    pub recording_id: String,
    pub asset_id: String,
    pub configuration_sha256: String,
    pub model_sha256: String,
    pub hypothesis_catalogue_sha256: String,
    /// Canonically sorted, unique, versioned hypothesis IDs.
    pub hypothesis_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRevisionInput {
    pub revision_kind: DiagnosticRevisionKind,
    pub issued_at_seconds: f64,
    pub new_evidence: Vec<RevisionEvidenceReference>,
    pub invalidations: Vec<RevisionEvidenceInvalidation>,
    pub disposition: DiagnosticDisposition,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRevisionReceipt {
    pub schema_version: String,
    pub binding: DiagnosticRevisionBinding,
    pub revision_index: u64,
    pub issued_at_seconds: f64,
    pub revision_kind: DiagnosticRevisionKind,
    pub new_evidence: Vec<RevisionEvidenceReference>,
    pub invalidations: Vec<RevisionEvidenceInvalidation>,
    pub disposition: DiagnosticDisposition,
    /// Digest over the sorted currently active evidence references.
    pub active_evidence_sha256: String,
    /// Sorted tombstones; invalidated evidence cannot be resurrected.
    pub invalidated_evidence_sha256: Vec<String>,
    pub previous_receipt_sha256: Option<String>,
    /// Hash over the complete receipt with this field set to the empty string.
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticRevisionError {
    InvalidBinding,
    InvalidIdentity,
    InvalidDigest,
    InvalidDisposition,
    InvalidTimestamp,
    EvidenceNotYetAvailable,
    NoNewEvidence,
    DuplicateEvidence(String),
    EvidenceAlreadyInvalidated(String),
    UnknownInvalidatedEvidence(String),
    DuplicateInvalidation(String),
    InvalidInvalidationBasis(String),
    InconsistentRevisionKind,
    TransitionNotEvidenceJustified,
    RevisionTimeNotMonotonic,
    SessionClosed,
    ReceiptChainBreak,
    ReceiptIntegrityFailure,
    Serialization,
}

impl fmt::Display for DiagnosticRevisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBinding => write!(formatter, "diagnostic revision binding is invalid"),
            Self::InvalidIdentity => write!(formatter, "diagnostic revision identity is empty"),
            Self::InvalidDigest => write!(
                formatter,
                "diagnostic revision digest is not lowercase SHA-256"
            ),
            Self::InvalidDisposition => write!(
                formatter,
                "diagnostic disposition is invalid for the frozen hypothesis catalogue"
            ),
            Self::InvalidTimestamp => write!(
                formatter,
                "diagnostic evidence or revision timestamp is invalid"
            ),
            Self::EvidenceNotYetAvailable => write!(
                formatter,
                "evidence was not available when the revision was issued"
            ),
            Self::NoNewEvidence => write!(
                formatter,
                "a diagnostic revision needs new evidence or an explicit invalidation basis"
            ),
            Self::DuplicateEvidence(id) => write!(formatter, "evidence digest was repeated: {id}"),
            Self::EvidenceAlreadyInvalidated(id) => write!(
                formatter,
                "invalidated evidence cannot be resurrected: {id}"
            ),
            Self::UnknownInvalidatedEvidence(id) => {
                write!(formatter, "invalidation target was not active: {id}")
            }
            Self::DuplicateInvalidation(id) => {
                write!(formatter, "evidence was invalidated more than once: {id}")
            }
            Self::InvalidInvalidationBasis(id) => write!(
                formatter,
                "invalidation basis is missing or has the wrong declared effect: {id}"
            ),
            Self::InconsistentRevisionKind => write!(
                formatter,
                "revision kind does not match the prior disposition or evidence delta"
            ),
            Self::TransitionNotEvidenceJustified => write!(
                formatter,
                "changed disposition is not justified by the declared evidence effects"
            ),
            Self::RevisionTimeNotMonotonic => {
                write!(formatter, "revision issue time did not increase")
            }
            Self::SessionClosed => {
                write!(formatter, "diagnostic lineage is closed after withdrawal")
            }
            Self::ReceiptChainBreak => write!(
                formatter,
                "diagnostic revision receipt chain is incomplete or out of order"
            ),
            Self::ReceiptIntegrityFailure => write!(
                formatter,
                "diagnostic revision receipt digest or replay failed"
            ),
            Self::Serialization => write!(
                formatter,
                "failed to serialize diagnostic revision evidence"
            ),
        }
    }
}

impl std::error::Error for DiagnosticRevisionError {}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_serializable<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<String, DiagnosticRevisionError> {
    let bytes = serde_json::to_vec(value).map_err(|_| DiagnosticRevisionError::Serialization)?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn revision_hypothesis_catalogue_sha256(
    hypothesis_ids: &[String],
) -> Result<String, DiagnosticRevisionError> {
    if hypothesis_ids.is_empty()
        || hypothesis_ids.iter().any(|id| id.trim().is_empty())
        || hypothesis_ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(DiagnosticRevisionError::InvalidBinding);
    }
    sha256_serializable(
        b"symtropy-diagnostic-hypothesis-catalogue-v1\0",
        hypothesis_ids,
    )
}

impl DiagnosticRevisionBinding {
    pub fn validate(&self) -> Result<(), DiagnosticRevisionError> {
        if self.run_id.trim().is_empty()
            || self.recording_id.trim().is_empty()
            || self.asset_id.trim().is_empty()
        {
            return Err(DiagnosticRevisionError::InvalidIdentity);
        }
        if !is_sha256_hex(&self.configuration_sha256)
            || !is_sha256_hex(&self.model_sha256)
            || !is_sha256_hex(&self.hypothesis_catalogue_sha256)
        {
            return Err(DiagnosticRevisionError::InvalidDigest);
        }
        if revision_hypothesis_catalogue_sha256(&self.hypothesis_ids)?
            != self.hypothesis_catalogue_sha256
        {
            return Err(DiagnosticRevisionError::InvalidBinding);
        }
        Ok(())
    }
}

fn validate_disposition(
    disposition: &DiagnosticDisposition,
    binding: &DiagnosticRevisionBinding,
) -> Result<(), DiagnosticRevisionError> {
    match disposition {
        DiagnosticDisposition::NoFault
        | DiagnosticDisposition::UnknownOrOutOfSupport
        | DiagnosticDisposition::NotEvaluable
        | DiagnosticDisposition::Withdrawn => Ok(()),
        DiagnosticDisposition::KnownHypotheses { hypothesis_ids } => {
            if hypothesis_ids.is_empty()
                || hypothesis_ids.iter().any(|id| id.trim().is_empty())
                || hypothesis_ids.windows(2).any(|pair| pair[0] >= pair[1])
                || hypothesis_ids
                    .iter()
                    .any(|id| !binding.hypothesis_ids.contains(id))
            {
                return Err(DiagnosticRevisionError::InvalidDisposition);
            }
            Ok(())
        }
    }
}

fn validate_evidence(
    evidence: &RevisionEvidenceReference,
    issued_at_seconds: f64,
) -> Result<(), DiagnosticRevisionError> {
    if evidence.source_id.trim().is_empty() {
        return Err(DiagnosticRevisionError::InvalidIdentity);
    }
    if !is_sha256_hex(&evidence.evidence_sha256) {
        return Err(DiagnosticRevisionError::InvalidDigest);
    }
    if !evidence.observed_at_seconds.is_finite()
        || !evidence.available_at_seconds.is_finite()
        || evidence.observed_at_seconds < 0.0
        || evidence.available_at_seconds < evidence.observed_at_seconds - TIME_TOLERANCE_SECONDS
        || evidence.available_at_seconds > issued_at_seconds + TIME_TOLERANCE_SECONDS
    {
        if evidence.available_at_seconds > issued_at_seconds + TIME_TOLERANCE_SECONDS {
            return Err(DiagnosticRevisionError::EvidenceNotYetAvailable);
        }
        return Err(DiagnosticRevisionError::InvalidTimestamp);
    }
    Ok(())
}

fn has_effect(evidence: &[RevisionEvidenceReference], expected: &[RevisionEvidenceEffect]) -> bool {
    evidence
        .iter()
        .any(|item| expected.contains(&item.declared_effect))
}

fn transition_is_justified(
    previous: &DiagnosticDisposition,
    next: &DiagnosticDisposition,
    evidence: &[RevisionEvidenceReference],
) -> bool {
    use DiagnosticDisposition as D;
    use RevisionEvidenceEffect as E;

    match (previous, next) {
        (
            D::KnownHypotheses {
                hypothesis_ids: before,
            },
            D::KnownHypotheses {
                hypothesis_ids: after,
            },
        ) => {
            let before_set = before.iter().collect::<BTreeSet<_>>();
            let after_set = after.iter().collect::<BTreeSet<_>>();
            let removed = before_set.difference(&after_set).count();
            let added = after_set.difference(&before_set).count();
            if removed > 0 && added == 0 {
                has_effect(evidence, &[E::NarrowsHypothesisSet, E::ContradictsCurrent])
            } else if added > 0 && removed == 0 {
                has_effect(evidence, &[E::ExpandsAmbiguity, E::ContradictsCurrent])
            } else {
                has_effect(
                    evidence,
                    &[
                        E::ContradictsCurrent,
                        E::NarrowsHypothesisSet,
                        E::ExpandsAmbiguity,
                    ],
                )
            }
        }
        (D::NoFault, D::KnownHypotheses { .. }) | (D::KnownHypotheses { .. }, D::NoFault) => {
            has_effect(evidence, &[E::ContradictsCurrent])
        }
        (_, D::UnknownOrOutOfSupport) => has_effect(
            evidence,
            &[E::ContradictsCurrent, E::ExpandsAmbiguity, E::ContextShift],
        ),
        (_, D::NotEvaluable) => has_effect(evidence, &[E::ContextShift]),
        (D::UnknownOrOutOfSupport, D::KnownHypotheses { .. })
        | (D::NotEvaluable, D::KnownHypotheses { .. }) => has_effect(
            evidence,
            &[
                E::ContradictsCurrent,
                E::NarrowsHypothesisSet,
                E::ContextShift,
            ],
        ),
        (D::UnknownOrOutOfSupport, D::NoFault) | (D::NotEvaluable, D::NoFault) => {
            has_effect(evidence, &[E::ContradictsCurrent, E::ContextShift])
        }
        (D::Withdrawn, _) => false,
        (_, D::Withdrawn) => false,
        _ => has_effect(evidence, &[E::ContradictsCurrent, E::ContextShift]),
    }
}

fn active_evidence_sha256(
    active: &BTreeMap<String, RevisionEvidenceReference>,
) -> Result<String, DiagnosticRevisionError> {
    sha256_serializable(b"symtropy-diagnostic-active-evidence-set-v1\0", active)
}

fn revision_receipt_sha256(
    receipt: &DiagnosticRevisionReceipt,
) -> Result<String, DiagnosticRevisionError> {
    let mut payload = receipt.clone();
    payload.receipt_sha256.clear();
    sha256_serializable(b"symtropy-diagnostic-revision-receipt-v1\0", &payload)
}

pub struct DiagnosticRevisionSession {
    binding: DiagnosticRevisionBinding,
    active_evidence: BTreeMap<String, RevisionEvidenceReference>,
    invalidated_evidence: BTreeSet<String>,
    previous_issued_at_seconds: Option<f64>,
    previous_receipt_sha256: Option<String>,
    current_disposition: Option<DiagnosticDisposition>,
    receipts: Vec<DiagnosticRevisionReceipt>,
    closed: bool,
}

impl DiagnosticRevisionSession {
    pub fn new(binding: DiagnosticRevisionBinding) -> Result<Self, DiagnosticRevisionError> {
        binding.validate()?;
        Ok(Self {
            binding,
            active_evidence: BTreeMap::new(),
            invalidated_evidence: BTreeSet::new(),
            previous_issued_at_seconds: None,
            previous_receipt_sha256: None,
            current_disposition: None,
            receipts: Vec::new(),
            closed: false,
        })
    }

    pub fn append(
        &mut self,
        input: &DiagnosticRevisionInput,
    ) -> Result<DiagnosticRevisionReceipt, DiagnosticRevisionError> {
        if self.closed {
            return Err(DiagnosticRevisionError::SessionClosed);
        }
        if !input.issued_at_seconds.is_finite() || input.issued_at_seconds < 0.0 {
            return Err(DiagnosticRevisionError::InvalidTimestamp);
        }
        if self
            .previous_issued_at_seconds
            .is_some_and(|previous| input.issued_at_seconds <= previous)
        {
            return Err(DiagnosticRevisionError::RevisionTimeNotMonotonic);
        }
        if input.new_evidence.is_empty() {
            return Err(DiagnosticRevisionError::NoNewEvidence);
        }
        validate_disposition(&input.disposition, &self.binding)?;

        let mut added = BTreeMap::<String, RevisionEvidenceReference>::new();
        for evidence in &input.new_evidence {
            validate_evidence(evidence, input.issued_at_seconds)?;
            let id = evidence.evidence_sha256.clone();
            if self.invalidated_evidence.contains(&id) {
                return Err(DiagnosticRevisionError::EvidenceAlreadyInvalidated(id));
            }
            if self.active_evidence.contains_key(&id) || added.contains_key(&id) {
                return Err(DiagnosticRevisionError::DuplicateEvidence(id));
            }
            added.insert(id, evidence.clone());
        }

        let mut invalidation_targets = BTreeSet::new();
        for invalidation in &input.invalidations {
            if !invalidation_targets.insert(invalidation.invalidated_evidence_sha256.clone()) {
                return Err(DiagnosticRevisionError::DuplicateInvalidation(
                    invalidation.invalidated_evidence_sha256.clone(),
                ));
            }
            if !self
                .active_evidence
                .contains_key(&invalidation.invalidated_evidence_sha256)
            {
                return Err(DiagnosticRevisionError::UnknownInvalidatedEvidence(
                    invalidation.invalidated_evidence_sha256.clone(),
                ));
            }
            let Some(basis) = added.get(&invalidation.basis_evidence_sha256) else {
                return Err(DiagnosticRevisionError::InvalidInvalidationBasis(
                    invalidation.basis_evidence_sha256.clone(),
                ));
            };
            if basis.declared_effect != RevisionEvidenceEffect::InvalidatesPriorEvidence
                || invalidation.invalidated_evidence_sha256 == invalidation.basis_evidence_sha256
            {
                return Err(DiagnosticRevisionError::InvalidInvalidationBasis(
                    invalidation.basis_evidence_sha256.clone(),
                ));
            }
        }

        let invalidation_basis_ids = input
            .invalidations
            .iter()
            .map(|item| item.basis_evidence_sha256.as_str())
            .collect::<BTreeSet<_>>();
        for evidence in &input.new_evidence {
            if evidence.declared_effect == RevisionEvidenceEffect::InvalidatesPriorEvidence
                && !invalidation_basis_ids.contains(evidence.evidence_sha256.as_str())
            {
                return Err(DiagnosticRevisionError::InvalidInvalidationBasis(
                    evidence.evidence_sha256.clone(),
                ));
            }
        }

        let previous = self.current_disposition.as_ref();
        match (input.revision_kind, previous) {
            (DiagnosticRevisionKind::InitialAssessment, None) => {
                if !input.invalidations.is_empty()
                    || matches!(&input.disposition, DiagnosticDisposition::Withdrawn)
                    || input.new_evidence.iter().any(|item| {
                        item.declared_effect == RevisionEvidenceEffect::InvalidatesPriorEvidence
                    })
                {
                    return Err(DiagnosticRevisionError::InconsistentRevisionKind);
                }
            }
            (DiagnosticRevisionKind::InitialAssessment, Some(_)) => {
                return Err(DiagnosticRevisionError::InconsistentRevisionKind);
            }
            (_, None) => return Err(DiagnosticRevisionError::InconsistentRevisionKind),
            (DiagnosticRevisionKind::Corroboration, Some(previous)) => {
                if !input.invalidations.is_empty()
                    || &input.disposition != previous
                    || input
                        .new_evidence
                        .iter()
                        .any(|item| item.declared_effect != RevisionEvidenceEffect::SupportsCurrent)
                {
                    return Err(DiagnosticRevisionError::InconsistentRevisionKind);
                }
            }
            (DiagnosticRevisionKind::Reassessment, Some(previous)) => {
                if !input.invalidations.is_empty()
                    || &input.disposition == previous
                    || !transition_is_justified(previous, &input.disposition, &input.new_evidence)
                {
                    return Err(DiagnosticRevisionError::TransitionNotEvidenceJustified);
                }
            }
            (DiagnosticRevisionKind::EvidenceRetraction, Some(previous)) => {
                if input.invalidations.is_empty() || &input.disposition == previous {
                    return Err(DiagnosticRevisionError::InconsistentRevisionKind);
                }
                if !matches!(&input.disposition, DiagnosticDisposition::Withdrawn)
                    && !transition_is_justified(previous, &input.disposition, &input.new_evidence)
                {
                    return Err(DiagnosticRevisionError::TransitionNotEvidenceJustified);
                }
            }
        }

        let mut next_active = self.active_evidence.clone();
        let mut next_invalidated = self.invalidated_evidence.clone();
        for invalidation in &input.invalidations {
            next_active.remove(&invalidation.invalidated_evidence_sha256);
            next_invalidated.insert(invalidation.invalidated_evidence_sha256.clone());
        }
        for (id, evidence) in added {
            next_active.insert(id, evidence);
        }

        let mut invalidated_ids = next_invalidated.iter().cloned().collect::<Vec<_>>();
        invalidated_ids.sort();
        let mut receipt = DiagnosticRevisionReceipt {
            schema_version: DIAGNOSIS_REVISION_RECEIPT_SCHEMA.to_owned(),
            binding: self.binding.clone(),
            revision_index: self.receipts.len() as u64,
            issued_at_seconds: input.issued_at_seconds,
            revision_kind: input.revision_kind,
            new_evidence: input.new_evidence.clone(),
            invalidations: input.invalidations.clone(),
            disposition: input.disposition.clone(),
            active_evidence_sha256: active_evidence_sha256(&next_active)?,
            invalidated_evidence_sha256: invalidated_ids,
            previous_receipt_sha256: self.previous_receipt_sha256.clone(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = revision_receipt_sha256(&receipt)?;

        self.active_evidence = next_active;
        self.invalidated_evidence = next_invalidated;
        self.previous_issued_at_seconds = Some(input.issued_at_seconds);
        self.previous_receipt_sha256 = Some(receipt.receipt_sha256.clone());
        self.current_disposition = Some(input.disposition.clone());
        self.closed = matches!(&input.disposition, DiagnosticDisposition::Withdrawn);
        self.receipts.push(receipt.clone());
        Ok(receipt)
    }

    pub fn current_disposition(&self) -> Option<&DiagnosticDisposition> {
        self.current_disposition.as_ref()
    }

    pub fn active_evidence_count(&self) -> usize {
        self.active_evidence.len()
    }

    pub fn receipts(&self) -> &[DiagnosticRevisionReceipt] {
        &self.receipts
    }

    pub fn binding(&self) -> &DiagnosticRevisionBinding {
        &self.binding
    }
}

/// Rebuild a lineage from its receipts. A receipt hash provides integrity and
/// tamper evidence, not issuer authentication or proof of evidence truth.
pub fn replay_diagnostic_revision_chain(
    binding: DiagnosticRevisionBinding,
    receipts: &[DiagnosticRevisionReceipt],
) -> Result<DiagnosticRevisionSession, DiagnosticRevisionError> {
    let mut session = DiagnosticRevisionSession::new(binding)?;
    for (expected_index, expected) in receipts.iter().enumerate() {
        if expected.revision_index != expected_index as u64
            || expected.schema_version != DIAGNOSIS_REVISION_RECEIPT_SCHEMA
            || expected.binding != *session.binding()
        {
            return Err(DiagnosticRevisionError::ReceiptChainBreak);
        }
        let input = DiagnosticRevisionInput {
            revision_kind: expected.revision_kind,
            issued_at_seconds: expected.issued_at_seconds,
            new_evidence: expected.new_evidence.clone(),
            invalidations: expected.invalidations.clone(),
            disposition: expected.disposition.clone(),
        };
        let replayed = session.append(&input)?;
        if replayed != *expected {
            return Err(DiagnosticRevisionError::ReceiptIntegrityFailure);
        }
    }
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn binding() -> DiagnosticRevisionBinding {
        let hypothesis_ids = vec!["fault_a".to_owned(), "fault_b".to_owned()];
        DiagnosticRevisionBinding {
            run_id: "run-001".into(),
            recording_id: "recording-001".into(),
            asset_id: "engine-001".into(),
            configuration_sha256: digest('a'),
            model_sha256: digest('b'),
            hypothesis_catalogue_sha256: revision_hypothesis_catalogue_sha256(&hypothesis_ids)
                .unwrap(),
            hypothesis_ids,
        }
    }

    fn evidence(
        id: char,
        effect: RevisionEvidenceEffect,
        observed_at_seconds: f64,
        available_at_seconds: f64,
    ) -> RevisionEvidenceReference {
        RevisionEvidenceReference {
            evidence_sha256: digest(id),
            source_id: format!("source-{id}"),
            role: RevisionEvidenceRole::SensorMeasurement,
            observed_at_seconds,
            available_at_seconds,
            declared_effect: effect,
        }
    }

    fn initial_fault_a(session: &mut DiagnosticRevisionSession) -> DiagnosticRevisionReceipt {
        session
            .append(&DiagnosticRevisionInput {
                revision_kind: DiagnosticRevisionKind::InitialAssessment,
                issued_at_seconds: 2.0,
                new_evidence: vec![evidence(
                    'c',
                    RevisionEvidenceEffect::SupportsCurrent,
                    1.0,
                    1.5,
                )],
                invalidations: vec![],
                disposition: DiagnosticDisposition::KnownHypotheses {
                    hypothesis_ids: vec!["fault_a".into()],
                },
            })
            .unwrap()
    }

    #[test]
    fn contradictory_new_evidence_revises_current_disposition_without_mutating_history() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        let first = initial_fault_a(&mut session);
        let second = session
            .append(&DiagnosticRevisionInput {
                revision_kind: DiagnosticRevisionKind::Reassessment,
                issued_at_seconds: 4.0,
                new_evidence: vec![evidence(
                    'd',
                    RevisionEvidenceEffect::ContradictsCurrent,
                    3.0,
                    3.5,
                )],
                invalidations: vec![],
                disposition: DiagnosticDisposition::KnownHypotheses {
                    hypothesis_ids: vec!["fault_b".into()],
                },
            })
            .unwrap();

        assert_eq!(first.revision_index, 0);
        assert_eq!(second.revision_index, 1);
        assert_eq!(session.receipts()[0], first);
        assert_eq!(
            session.current_disposition(),
            Some(&DiagnosticDisposition::KnownHypotheses {
                hypothesis_ids: vec!["fault_b".into()]
            })
        );
        let replayed = replay_diagnostic_revision_chain(binding(), session.receipts()).unwrap();
        assert_eq!(replayed.receipts(), session.receipts());
    }

    #[test]
    fn corroboration_adds_evidence_without_fabricating_a_new_diagnosis() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        let first = initial_fault_a(&mut session);
        let second = session
            .append(&DiagnosticRevisionInput {
                revision_kind: DiagnosticRevisionKind::Corroboration,
                issued_at_seconds: 3.0,
                new_evidence: vec![evidence(
                    'd',
                    RevisionEvidenceEffect::SupportsCurrent,
                    2.5,
                    2.8,
                )],
                invalidations: vec![],
                disposition: first.disposition.clone(),
            })
            .unwrap();
        assert_eq!(second.disposition, first.disposition);
        assert_eq!(second.revision_index, 1);
        assert_eq!(session.active_evidence_count(), 2);
    }

    #[test]
    fn evidence_unavailable_at_decision_time_is_rejected() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        let result = session.append(&DiagnosticRevisionInput {
            revision_kind: DiagnosticRevisionKind::InitialAssessment,
            issued_at_seconds: 2.0,
            new_evidence: vec![evidence(
                'c',
                RevisionEvidenceEffect::SupportsCurrent,
                1.0,
                2.1,
            )],
            invalidations: vec![],
            disposition: DiagnosticDisposition::NoFault,
        });
        assert_eq!(
            result,
            Err(DiagnosticRevisionError::EvidenceNotYetAvailable)
        );
    }

    #[test]
    fn missingness_or_time_passing_alone_cannot_change_current_disposition() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        initial_fault_a(&mut session);
        let result = session.append(&DiagnosticRevisionInput {
            revision_kind: DiagnosticRevisionKind::Reassessment,
            issued_at_seconds: 10.0,
            new_evidence: vec![],
            invalidations: vec![],
            disposition: DiagnosticDisposition::NoFault,
        });
        assert_eq!(result, Err(DiagnosticRevisionError::NoNewEvidence));
        assert_eq!(session.receipts().len(), 1);
    }

    #[test]
    fn invalidation_requires_new_basis_and_target_must_be_active() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        initial_fault_a(&mut session);
        let missing_basis = session.append(&DiagnosticRevisionInput {
            revision_kind: DiagnosticRevisionKind::EvidenceRetraction,
            issued_at_seconds: 3.0,
            new_evidence: vec![evidence(
                'd',
                RevisionEvidenceEffect::InvalidatesPriorEvidence,
                2.5,
                2.8,
            )],
            invalidations: vec![RevisionEvidenceInvalidation {
                invalidated_evidence_sha256: digest('c'),
                basis_evidence_sha256: digest('e'),
            }],
            disposition: DiagnosticDisposition::Withdrawn,
        });
        assert_eq!(
            missing_basis,
            Err(DiagnosticRevisionError::InvalidInvalidationBasis(digest(
                'e'
            )))
        );

        let unknown_target = session.append(&DiagnosticRevisionInput {
            revision_kind: DiagnosticRevisionKind::EvidenceRetraction,
            issued_at_seconds: 3.0,
            new_evidence: vec![evidence(
                'd',
                RevisionEvidenceEffect::InvalidatesPriorEvidence,
                2.5,
                2.8,
            )],
            invalidations: vec![RevisionEvidenceInvalidation {
                invalidated_evidence_sha256: digest('f'),
                basis_evidence_sha256: digest('d'),
            }],
            disposition: DiagnosticDisposition::Withdrawn,
        });
        assert_eq!(
            unknown_target,
            Err(DiagnosticRevisionError::UnknownInvalidatedEvidence(digest(
                'f'
            )))
        );
    }

    #[test]
    fn explicit_evidence_retraction_withdraws_current_claim_and_preserves_old_receipt() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        let first = initial_fault_a(&mut session);
        let second = session
            .append(&DiagnosticRevisionInput {
                revision_kind: DiagnosticRevisionKind::EvidenceRetraction,
                issued_at_seconds: 3.0,
                new_evidence: vec![evidence(
                    'd',
                    RevisionEvidenceEffect::InvalidatesPriorEvidence,
                    2.5,
                    2.8,
                )],
                invalidations: vec![RevisionEvidenceInvalidation {
                    invalidated_evidence_sha256: digest('c'),
                    basis_evidence_sha256: digest('d'),
                }],
                disposition: DiagnosticDisposition::Withdrawn,
            })
            .unwrap();
        assert_eq!(first.revision_index, 0);
        assert_eq!(
            second.revision_kind,
            DiagnosticRevisionKind::EvidenceRetraction
        );
        assert_eq!(session.receipts()[0], first);
        assert_eq!(session.active_evidence_count(), 1);
        assert_eq!(
            session.current_disposition(),
            Some(&DiagnosticDisposition::Withdrawn)
        );
        assert_eq!(
            session.append(&DiagnosticRevisionInput {
                revision_kind: DiagnosticRevisionKind::InitialAssessment,
                issued_at_seconds: 4.0,
                new_evidence: vec![evidence(
                    'e',
                    RevisionEvidenceEffect::SupportsCurrent,
                    3.5,
                    3.8,
                )],
                invalidations: vec![],
                disposition: DiagnosticDisposition::NoFault,
            }),
            Err(DiagnosticRevisionError::SessionClosed)
        );
    }

    #[test]
    fn invalidated_evidence_cannot_be_resurrected() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        initial_fault_a(&mut session);
        session
            .append(&DiagnosticRevisionInput {
                revision_kind: DiagnosticRevisionKind::EvidenceRetraction,
                issued_at_seconds: 3.0,
                new_evidence: vec![
                    evidence(
                        'd',
                        RevisionEvidenceEffect::InvalidatesPriorEvidence,
                        2.5,
                        2.8,
                    ),
                    evidence('e', RevisionEvidenceEffect::ContextShift, 2.6, 2.9),
                ],
                invalidations: vec![RevisionEvidenceInvalidation {
                    invalidated_evidence_sha256: digest('c'),
                    basis_evidence_sha256: digest('d'),
                }],
                disposition: DiagnosticDisposition::NotEvaluable,
            })
            .unwrap();
        let result = session.append(&DiagnosticRevisionInput {
            revision_kind: DiagnosticRevisionKind::Corroboration,
            issued_at_seconds: 4.0,
            new_evidence: vec![evidence(
                'c',
                RevisionEvidenceEffect::SupportsCurrent,
                1.0,
                3.9,
            )],
            invalidations: vec![],
            disposition: DiagnosticDisposition::NotEvaluable,
        });
        assert_eq!(
            result,
            Err(DiagnosticRevisionError::EvidenceAlreadyInvalidated(digest(
                'c'
            )))
        );
    }

    #[test]
    fn ambiguous_to_singleton_transition_requires_a_narrowing_or_contradiction_effect() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        session
            .append(&DiagnosticRevisionInput {
                revision_kind: DiagnosticRevisionKind::InitialAssessment,
                issued_at_seconds: 2.0,
                new_evidence: vec![evidence(
                    'c',
                    RevisionEvidenceEffect::SupportsCurrent,
                    1.0,
                    1.5,
                )],
                invalidations: vec![],
                disposition: DiagnosticDisposition::KnownHypotheses {
                    hypothesis_ids: vec!["fault_a".into(), "fault_b".into()],
                },
            })
            .unwrap();
        let result = session.append(&DiagnosticRevisionInput {
            revision_kind: DiagnosticRevisionKind::Reassessment,
            issued_at_seconds: 3.0,
            new_evidence: vec![evidence(
                'd',
                RevisionEvidenceEffect::SupportsCurrent,
                2.5,
                2.8,
            )],
            invalidations: vec![],
            disposition: DiagnosticDisposition::KnownHypotheses {
                hypothesis_ids: vec!["fault_a".into()],
            },
        });
        assert_eq!(
            result,
            Err(DiagnosticRevisionError::TransitionNotEvidenceJustified)
        );
    }

    #[test]
    fn duplicate_evidence_is_rejected_even_when_source_wrapper_changes() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        initial_fault_a(&mut session);
        let mut replay = evidence('c', RevisionEvidenceEffect::SupportsCurrent, 1.0, 1.5);
        replay.source_id = "different-wrapper".into();
        let result = session.append(&DiagnosticRevisionInput {
            revision_kind: DiagnosticRevisionKind::Corroboration,
            issued_at_seconds: 3.0,
            new_evidence: vec![replay],
            invalidations: vec![],
            disposition: DiagnosticDisposition::KnownHypotheses {
                hypothesis_ids: vec!["fault_a".into()],
            },
        });
        assert_eq!(
            result,
            Err(DiagnosticRevisionError::DuplicateEvidence(digest('c')))
        );
    }

    #[test]
    fn late_arriving_evidence_can_create_a_new_revision_but_not_rewrite_the_past() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        let first = initial_fault_a(&mut session);
        let second = session
            .append(&DiagnosticRevisionInput {
                revision_kind: DiagnosticRevisionKind::Reassessment,
                issued_at_seconds: 6.0,
                new_evidence: vec![evidence(
                    'd',
                    RevisionEvidenceEffect::ContradictsCurrent,
                    2.5,
                    5.0,
                )],
                invalidations: vec![],
                disposition: DiagnosticDisposition::UnknownOrOutOfSupport,
            })
            .unwrap();
        assert_eq!(first.issued_at_seconds, 2.0);
        assert_eq!(
            first.disposition,
            DiagnosticDisposition::KnownHypotheses {
                hypothesis_ids: vec!["fault_a".into()]
            }
        );
        assert_eq!(second.issued_at_seconds, 6.0);
        assert_eq!(session.receipts()[0], first);
    }

    #[test]
    fn changed_model_or_configuration_requires_a_new_binding_lineage() {
        let mut changed = binding();
        changed.configuration_sha256 = digest('e');
        let session = DiagnosticRevisionSession::new(binding()).unwrap();
        assert_ne!(
            session.binding().configuration_sha256,
            changed.configuration_sha256
        );
        assert_eq!(
            DiagnosticRevisionSession::new(changed)
                .unwrap()
                .receipts()
                .len(),
            0
        );
    }

    #[test]
    fn reordered_or_edited_receipts_fail_replay() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        let first = initial_fault_a(&mut session);
        session
            .append(&DiagnosticRevisionInput {
                revision_kind: DiagnosticRevisionKind::Corroboration,
                issued_at_seconds: 3.0,
                new_evidence: vec![evidence(
                    'd',
                    RevisionEvidenceEffect::SupportsCurrent,
                    2.5,
                    2.8,
                )],
                invalidations: vec![],
                disposition: first.disposition.clone(),
            })
            .unwrap();
        let original = session.receipts().to_vec();
        let replayed = replay_diagnostic_revision_chain(binding(), &original).unwrap();
        assert_eq!(replayed.receipts(), original.as_slice());

        let mut tampered = original;
        tampered[0].disposition = DiagnosticDisposition::NoFault;
        assert!(matches!(
            replay_diagnostic_revision_chain(binding(), &tampered),
            Err(DiagnosticRevisionError::ReceiptIntegrityFailure)
        ));
    }

    #[test]
    fn revision_times_must_be_strictly_increasing() {
        let mut session = DiagnosticRevisionSession::new(binding()).unwrap();
        initial_fault_a(&mut session);
        let result = session.append(&DiagnosticRevisionInput {
            revision_kind: DiagnosticRevisionKind::Corroboration,
            issued_at_seconds: 2.0,
            new_evidence: vec![evidence(
                'd',
                RevisionEvidenceEffect::SupportsCurrent,
                1.8,
                1.9,
            )],
            invalidations: vec![],
            disposition: DiagnosticDisposition::KnownHypotheses {
                hypothesis_ids: vec!["fault_a".into()],
            },
        });
        assert_eq!(
            result,
            Err(DiagnosticRevisionError::RevisionTimeNotMonotonic)
        );
    }

    #[test]
    fn protocol_and_fixture_keep_the_hard_gates_machine_readable() {
        let protocol: serde_json::Value =
            serde_json::from_str(include_str!("../diagnostic-claim-revision-v0.1.json")).unwrap();
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../diagnostic-claim-revision-cases-v0.1.json"))
                .unwrap();
        assert_eq!(protocol["schema_version"], DIAGNOSIS_REVISION_SCHEMA);
        assert_eq!(protocol["status"], "protocol_only_not_executed");
        assert!(protocol["hard_gates"].as_array().unwrap().len() >= 12);
        assert_eq!(corpus["cases"].as_array().unwrap().len(), 14);
        assert!(
            corpus["cases"]
                .as_array()
                .unwrap()
                .iter()
                .all(|case| case["expected_contract_result"].is_string())
        );
    }
}
