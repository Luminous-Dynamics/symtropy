// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Scope-bounded source inventory closure for Power Fabric.
//!
//! This module distinguishes inventory closure from semantic mapping quality.
//! It validates the declared scope, snapshot/traversal evidence, page/cursor chain,
//! record identities and one-to-one semantic dispositions. Source authenticity,
//! snapshot guarantees and evidence admission are external trust boundaries.
//!
//! A complete result means complete within the exact declared scope only. It does not
//! establish universal completeness, physical truth, engineering correctness or safety.

use crate::{EvidenceAdmission, EvidenceAdmissionVerifier, EvidenceRecord};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryError {
    EmptyField(&'static str),
    InvalidSha256 { field: &'static str, value: String },
    InvalidTimeRange { entity_id: String },
    DuplicatePageId { page_id: String },
    DuplicatePageIndex { page_index: u32 },
    DuplicateRecordIdentity { namespace: String, source_id: String },
    DuplicateDisposition { namespace: String, source_id: String },
    DuplicateExclusion { exclusion_id: String },
    InvalidPage { page_id: String, reason: &'static str },
    InvalidRecordDisposition { reason: &'static str },
    ExclusionScopeMismatch { exclusion_id: String },
    Serialization(String),
}

impl std::fmt::Display for InventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for InventoryError {}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), InventoryError> {
    if value.trim().is_empty() {
        return Err(InventoryError::EmptyField(field));
    }
    Ok(())
}

fn validate_sha256(field: &'static str, value: &str) -> Result<(), InventoryError> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !valid {
        return Err(InventoryError::InvalidSha256 {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

/// Declared query boundary. It intentionally stores digests of filters rather than
/// credentials or potentially sensitive query contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryScope {
    source_system_id: String,
    source_namespace: String,
    adapter_profile_id: String,
    adapter_profile_version: String,
    query_scope_sha256: String,
    filter_manifest_sha256: String,
    included_record_classes: BTreeSet<String>,
    permission_scope_receipt_ref: String,
    explicit_exclusion_ids: BTreeSet<String>,
    scope_start_ms: Option<i64>,
    scope_end_ms: Option<i64>,
}

impl InventoryScope {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_system_id: impl Into<String>,
        source_namespace: impl Into<String>,
        adapter_profile_id: impl Into<String>,
        adapter_profile_version: impl Into<String>,
        query_scope_sha256: impl Into<String>,
        filter_manifest_sha256: impl Into<String>,
        included_record_classes: BTreeSet<String>,
        permission_scope_receipt_ref: impl Into<String>,
        explicit_exclusion_ids: BTreeSet<String>,
        scope_start_ms: Option<i64>,
        scope_end_ms: Option<i64>,
    ) -> Result<Self, InventoryError> {
        let value = Self {
            source_system_id: source_system_id.into(),
            source_namespace: source_namespace.into(),
            adapter_profile_id: adapter_profile_id.into(),
            adapter_profile_version: adapter_profile_version.into(),
            query_scope_sha256: query_scope_sha256.into(),
            filter_manifest_sha256: filter_manifest_sha256.into(),
            included_record_classes,
            permission_scope_receipt_ref: permission_scope_receipt_ref.into(),
            explicit_exclusion_ids,
            scope_start_ms,
            scope_end_ms,
        };
        require_non_empty("source_system_id", &value.source_system_id)?;
        require_non_empty("source_namespace", &value.source_namespace)?;
        require_non_empty("adapter_profile_id", &value.adapter_profile_id)?;
        require_non_empty("adapter_profile_version", &value.adapter_profile_version)?;
        require_non_empty(
            "permission_scope_receipt_ref",
            &value.permission_scope_receipt_ref,
        )?;
        validate_sha256("query_scope_sha256", &value.query_scope_sha256)?;
        validate_sha256("filter_manifest_sha256", &value.filter_manifest_sha256)?;
        if value.included_record_classes.is_empty()
            || value.included_record_classes.iter().any(|item| item.trim().is_empty())
        {
            return Err(InventoryError::EmptyField("included_record_classes"));
        }
        if value.explicit_exclusion_ids.iter().any(|item| item.trim().is_empty()) {
            return Err(InventoryError::EmptyField("explicit_exclusion_ids"));
        }
        if matches!((scope_start_ms, scope_end_ms), (Some(start), Some(end)) if start > end) {
            return Err(InventoryError::InvalidTimeRange {
                entity_id: value.source_system_id.clone(),
            });
        }
        Ok(value)
    }
}

/// Source consistency capability, not a caller-assigned completeness status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotConsistencyModel {
    ImmutableExport,
    PointInTimeSnapshot,
    SnapshotPaginatedApi,
    BestEffortMutable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventorySnapshot {
    snapshot_id: String,
    source_revision: String,
    consistency_model: SnapshotConsistencyModel,
    snapshot_token_sha256: String,
    captured_from_ms: i64,
    captured_until_ms: i64,
    source_snapshot_evidence: EvidenceRecord,
}

impl InventorySnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        snapshot_id: impl Into<String>,
        source_revision: impl Into<String>,
        consistency_model: SnapshotConsistencyModel,
        snapshot_token_sha256: impl Into<String>,
        captured_from_ms: i64,
        captured_until_ms: i64,
        source_snapshot_evidence: EvidenceRecord,
    ) -> Result<Self, InventoryError> {
        let value = Self {
            snapshot_id: snapshot_id.into(),
            source_revision: source_revision.into(),
            consistency_model,
            snapshot_token_sha256: snapshot_token_sha256.into(),
            captured_from_ms,
            captured_until_ms,
            source_snapshot_evidence,
        };
        require_non_empty("snapshot_id", &value.snapshot_id)?;
        require_non_empty("source_revision", &value.source_revision)?;
        validate_sha256("snapshot_token_sha256", &value.snapshot_token_sha256)?;
        if captured_from_ms > captured_until_ms {
            return Err(InventoryError::InvalidTimeRange {
                entity_id: value.snapshot_id.clone(),
            });
        }
        Ok(value)
    }
}

/// Traversal mechanics are pinned separately from the provider snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraversalKind {
    ImmutableManifest,
    CursorPaginatedApi,
    OffsetPaginatedApi,
    ContinuationLink,
    BestEffort,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryTraversalContract {
    kind: TraversalKind,
    contract_version: String,
    expected_page_count: Option<u32>,
    expected_record_count: Option<u64>,
    terminal_boundary_required: bool,
    terminal_boundary_sha256: Option<String>,
    closure_evidence: EvidenceRecord,
}

impl InventoryTraversalContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: TraversalKind,
        contract_version: impl Into<String>,
        expected_page_count: Option<u32>,
        expected_record_count: Option<u64>,
        terminal_boundary_required: bool,
        terminal_boundary_sha256: Option<String>,
        closure_evidence: EvidenceRecord,
    ) -> Result<Self, InventoryError> {
        let value = Self {
            kind,
            contract_version: contract_version.into(),
            expected_page_count,
            expected_record_count,
            terminal_boundary_required,
            terminal_boundary_sha256,
            closure_evidence,
        };
        require_non_empty("traversal_contract_version", &value.contract_version)?;
        if value.expected_page_count == Some(0) {
            return Err(InventoryError::InvalidPage {
                page_id: "traversal".to_owned(),
                reason: "expected page count must be positive when supplied",
            });
        }
        if let Some(digest) = &value.terminal_boundary_sha256 {
            validate_sha256("terminal_boundary_sha256", digest)?;
        }
        if value.terminal_boundary_required && value.terminal_boundary_sha256.is_none() {
            return Err(InventoryError::InvalidPage {
                page_id: "traversal".to_owned(),
                reason: "terminal boundary evidence digest is required by this contract",
            });
        }
        Ok(value)
    }
}

/// Namespaced identity: equal local IDs in different namespaces remain distinct.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SourceRecordIdentity {
    namespace: String,
    source_id: String,
}

impl SourceRecordIdentity {
    pub fn new(
        namespace: impl Into<String>,
        source_id: impl Into<String>,
    ) -> Result<Self, InventoryError> {
        let value = Self {
            namespace: namespace.into(),
            source_id: source_id.into(),
        };
        require_non_empty("source_record_namespace", &value.namespace)?;
        require_non_empty("source_record_id", &value.source_id)?;
        Ok(value)
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryPage {
    page_id: String,
    page_index: u32,
    request_cursor_sha256: Option<String>,
    next_cursor_sha256: Option<String>,
    terminal: bool,
    source_revision: String,
    snapshot_token_sha256: String,
    page_payload_sha256: String,
    received_at_ms: i64,
    evidence: EvidenceRecord,
    record_identities: Vec<SourceRecordIdentity>,
}

impl InventoryPage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        page_id: impl Into<String>,
        page_index: u32,
        request_cursor_sha256: Option<String>,
        next_cursor_sha256: Option<String>,
        terminal: bool,
        source_revision: impl Into<String>,
        snapshot_token_sha256: impl Into<String>,
        page_payload_sha256: impl Into<String>,
        received_at_ms: i64,
        evidence: EvidenceRecord,
        mut record_identities: Vec<SourceRecordIdentity>,
    ) -> Result<Self, InventoryError> {
        let value = Self {
            page_id: page_id.into(),
            page_index,
            request_cursor_sha256,
            next_cursor_sha256,
            terminal,
            source_revision: source_revision.into(),
            snapshot_token_sha256: snapshot_token_sha256.into(),
            page_payload_sha256: page_payload_sha256.into(),
            received_at_ms,
            evidence,
            record_identities: {
                record_identities.sort();
                record_identities
            },
        };
        require_non_empty("page_id", &value.page_id)?;
        require_non_empty("page_source_revision", &value.source_revision)?;
        validate_sha256("page_snapshot_token_sha256", &value.snapshot_token_sha256)?;
        validate_sha256("page_payload_sha256", &value.page_payload_sha256)?;
        if let Some(digest) = &value.request_cursor_sha256 {
            validate_sha256("request_cursor_sha256", digest)?;
        }
        if let Some(digest) = &value.next_cursor_sha256 {
            validate_sha256("next_cursor_sha256", digest)?;
        }
        if value.terminal != value.next_cursor_sha256.is_none() {
            return Err(InventoryError::InvalidPage {
                page_id: value.page_id.clone(),
                reason: "terminal marker and next cursor must agree",
            });
        }
        if value
            .record_identities
            .windows(2)
            .any(|pair| pair[0] == pair[1])
        {
            return Err(InventoryError::InvalidPage {
                page_id: value.page_id.clone(),
                reason: "duplicate namespaced record identity within page",
            });
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryRecord {
    identity: SourceRecordIdentity,
    source_record_class: String,
    page_id: String,
    source_revision: String,
    payload_sha256: String,
}

impl InventoryRecord {
    pub fn new(
        identity: SourceRecordIdentity,
        source_record_class: impl Into<String>,
        page_id: impl Into<String>,
        source_revision: impl Into<String>,
        payload_sha256: impl Into<String>,
    ) -> Result<Self, InventoryError> {
        let value = Self {
            identity,
            source_record_class: source_record_class.into(),
            page_id: page_id.into(),
            source_revision: source_revision.into(),
            payload_sha256: payload_sha256.into(),
        };
        require_non_empty("source_record_class", &value.source_record_class)?;
        require_non_empty("record_page_id", &value.page_id)?;
        require_non_empty("record_source_revision", &value.source_revision)?;
        validate_sha256("record_payload_sha256", &value.payload_sha256)?;
        Ok(value)
    }

    pub fn identity(&self) -> &SourceRecordIdentity {
        &self.identity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticDispositionKind {
    Mapped,
    LossExplicit,
    Unsupported,
    Ambiguous,
    ExcludedWithReason,
    UnavailableWithReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecordDisposition {
    identity: SourceRecordIdentity,
    kind: SemanticDispositionKind,
    mapping_record_ref: Option<String>,
    exclusion_ref: Option<String>,
    explanation: String,
}

impl RecordDisposition {
    pub fn new(
        identity: SourceRecordIdentity,
        kind: SemanticDispositionKind,
        mapping_record_ref: Option<String>,
        exclusion_ref: Option<String>,
        explanation: impl Into<String>,
    ) -> Result<Self, InventoryError> {
        let value = Self {
            identity,
            kind,
            mapping_record_ref,
            exclusion_ref,
            explanation: explanation.into(),
        };
        require_non_empty("disposition_explanation", &value.explanation)?;
        match value.kind {
            SemanticDispositionKind::Mapped | SemanticDispositionKind::LossExplicit => {
                if value.mapping_record_ref.as_deref().is_none_or(str::is_empty)
                    || value.exclusion_ref.is_some()
                {
                    return Err(InventoryError::InvalidRecordDisposition {
                        reason: "mapped/loss-explicit disposition requires mapping ref only",
                    });
                }
            }
            SemanticDispositionKind::ExcludedWithReason => {
                if value.exclusion_ref.as_deref().is_none_or(str::is_empty)
                    || value.mapping_record_ref.is_some()
                {
                    return Err(InventoryError::InvalidRecordDisposition {
                        reason: "excluded disposition requires exclusion ref only",
                    });
                }
            }
            SemanticDispositionKind::Unsupported
            | SemanticDispositionKind::Ambiguous
            | SemanticDispositionKind::UnavailableWithReason => {
                if value.mapping_record_ref.is_some() || value.exclusion_ref.is_some() {
                    return Err(InventoryError::InvalidRecordDisposition {
                        reason: "unsupported/ambiguous/unavailable cannot claim a mapping or exclusion ref",
                    });
                }
            }
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryExclusion {
    exclusion_id: String,
    boundary_kind: String,
    boundary_ref: String,
    reason: String,
    evidence: EvidenceRecord,
}

impl InventoryExclusion {
    pub fn new(
        exclusion_id: impl Into<String>,
        boundary_kind: impl Into<String>,
        boundary_ref: impl Into<String>,
        reason: impl Into<String>,
        evidence: EvidenceRecord,
    ) -> Result<Self, InventoryError> {
        let value = Self {
            exclusion_id: exclusion_id.into(),
            boundary_kind: boundary_kind.into(),
            boundary_ref: boundary_ref.into(),
            reason: reason.into(),
            evidence,
        };
        require_non_empty("exclusion_id", &value.exclusion_id)?;
        require_non_empty("exclusion_boundary_kind", &value.boundary_kind)?;
        require_non_empty("exclusion_boundary_ref", &value.boundary_ref)?;
        require_non_empty("exclusion_reason", &value.reason)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotVerification {
    Stable,
    Changed,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeVerification {
    Verified,
    VerifiedWithDeclaredLimitations,
    Unknown,
    Rejected,
}

/// External provider/source verifier. It must verify that the provider's snapshot token,
/// source revision, query/filter scope and traversal semantics mean what the adapter claims.
/// It is not allowed to infer completeness from caller counts alone.
pub trait SourceInventoryVerifier {
    fn verify_snapshot(
        &self,
        scope: &InventoryScope,
        snapshot: &InventorySnapshot,
        traversal: &InventoryTraversalContract,
        as_known_at_ms: i64,
    ) -> SnapshotVerification;

    fn verify_scope(
        &self,
        scope: &InventoryScope,
        exclusions: &[InventoryExclusion],
        as_known_at_ms: i64,
    ) -> ScopeVerification;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryClosureState {
    CompleteWithinDeclaredScope,
    Incomplete,
    Unknown,
    Conflicted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InventorySemanticState {
    Complete,
    CompleteWithDeclaredLoss,
    IncompleteUnsupportedOrAmbiguous,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryDiagnosticCode {
    SnapshotConsistencyBestEffort,
    SnapshotChanged,
    SnapshotVerificationUnknown,
    SnapshotVerificationRejected,
    ScopeVerificationUnknown,
    ScopeVerificationRejected,
    EvidenceUnavailableAtCutoff,
    EvidenceAdmissionUnknown,
    EvidenceAdmissionRejected,
    PageUnavailableAtCutoff,
    PageArtifactDigestMismatch,
    PageReceiptTimeMismatch,
    MissingPage,
    PageCountMismatch,
    RecordCountMismatch,
    TerminalBoundaryMissing,
    TerminalPageNotLast,
    FirstPageCursorNotEmpty,
    CursorChainDiscontinuity,
    CursorCycle,
    SourceRevisionChanged,
    SnapshotTokenMismatch,
    SnapshotTokenNotPinned,
    PageRecordInventoryMismatch,
    UnknownPageReference,
    RecordPageUnavailableAtCutoff,
    RecordRevisionMismatch,
    RecordOutsideIncludedClass,
    DuplicateSourceRecordIdentity,
    MissingDisposition,
    DuplicateDisposition,
    UnknownDispositionRecord,
    UnknownExclusionReference,
    ExclusionScopeMismatch,
    SemanticInventoryContainsUnsupported,
    SemanticInventoryContainsAmbiguous,
    SemanticInventoryContainsDeclaredLoss,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct InventoryDiagnostic {
    pub code: InventoryDiagnosticCode,
    pub entity_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryClosureReport {
    pub source_system_id: String,
    pub snapshot_id: String,
    pub as_known_at_ms: i64,
    pub manifest_sha256: String,
    pub closure_state: InventoryClosureState,
    pub semantic_state: InventorySemanticState,
    pub scope_limited: bool,
    pub declared_exclusion_ids: Vec<String>,
    pub enumerated_record_count: usize,
    pub accounted_record_count: usize,
    pub page_count: usize,
    pub diagnostics: Vec<InventoryDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceInventoryManifest {
    scope: InventoryScope,
    snapshot: InventorySnapshot,
    traversal: InventoryTraversalContract,
    pages: Vec<InventoryPage>,
    records: Vec<InventoryRecord>,
    dispositions: Vec<RecordDisposition>,
    exclusions: Vec<InventoryExclusion>,
}

impl SourceInventoryManifest {
    pub fn new(
        scope: InventoryScope,
        snapshot: InventorySnapshot,
        traversal: InventoryTraversalContract,
        mut pages: Vec<InventoryPage>,
        mut records: Vec<InventoryRecord>,
        mut dispositions: Vec<RecordDisposition>,
        mut exclusions: Vec<InventoryExclusion>,
    ) -> Result<Self, InventoryError> {
        pages.sort_by_key(|page| (page.page_index, page.page_id.clone()));
        records.sort_by(|left, right| left.identity.cmp(&right.identity));
        dispositions.sort_by(|left, right| left.identity.cmp(&right.identity));
        exclusions.sort_by(|left, right| left.exclusion_id.cmp(&right.exclusion_id));

        let mut page_ids = BTreeSet::new();
        let mut page_indexes = BTreeSet::new();
        for page in &pages {
            if !page_ids.insert(page.page_id.clone()) {
                return Err(InventoryError::DuplicatePageId {
                    page_id: page.page_id.clone(),
                });
            }
            if !page_indexes.insert(page.page_index) {
                return Err(InventoryError::DuplicatePageIndex {
                    page_index: page.page_index,
                });
            }
        }

        let mut record_ids = BTreeSet::new();
        for record in &records {
            let identity = record.identity.clone();
            if !record_ids.insert(identity.clone()) {
                return Err(InventoryError::DuplicateRecordIdentity {
                    namespace: identity.namespace,
                    source_id: identity.source_id,
                });
            }
        }

        let mut disposition_ids = BTreeSet::new();
        for disposition in &dispositions {
            let identity = disposition.identity.clone();
            if !disposition_ids.insert(identity.clone()) {
                return Err(InventoryError::DuplicateDisposition {
                    namespace: identity.namespace,
                    source_id: identity.source_id,
                });
            }
        }

        let mut exclusion_ids = BTreeSet::new();
        for exclusion in &exclusions {
            if !exclusion_ids.insert(exclusion.exclusion_id.clone()) {
                return Err(InventoryError::DuplicateExclusion {
                    exclusion_id: exclusion.exclusion_id.clone(),
                });
            }
        }

        Ok(Self {
            scope,
            snapshot,
            traversal,
            pages,
            records,
            dispositions,
            exclusions,
        })
    }

    pub fn scope(&self) -> &InventoryScope {
        &self.scope
    }

    pub fn pages(&self) -> &[InventoryPage] {
        &self.pages
    }

    pub fn records(&self) -> &[InventoryRecord] {
        &self.records
    }

    /// Recompute the manifest commitment under serde_json's declared struct field order
    /// and ordered-vector contract. This is deterministic serialization, not RFC 8785.
    pub fn manifest_sha256(&self) -> Result<String, InventoryError> {
        let bytes = serde_json::to_vec(self)
            .map_err(|error| InventoryError::Serialization(error.to_string()))?;
        let digest = Sha256::digest(bytes);
        Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
    }

    pub fn project(
        &self,
        as_known_at_ms: i64,
        evidence_verifier: &dyn EvidenceAdmissionVerifier,
        source_verifier: &dyn SourceInventoryVerifier,
    ) -> InventoryClosureReport {
        let mut diagnostics = BTreeSet::new();
        let mut closure = InventoryClosureState::CompleteWithinDeclaredScope;
        let mut semantic = InventorySemanticState::Complete;
        let mut scope_limited = !self.scope.explicit_exclusion_ids.is_empty();

        let mut push = |code: InventoryDiagnosticCode, entity_ref: String, state: InventoryClosureState| {
            diagnostics.insert(InventoryDiagnostic { code, entity_ref });
            closure = combine_closure(closure, state);
        };

        if self.snapshot.consistency_model == SnapshotConsistencyModel::BestEffortMutable
            || self.traversal.kind == TraversalKind::BestEffort
        {
            push(
                InventoryDiagnosticCode::SnapshotConsistencyBestEffort,
                self.snapshot.snapshot_id.clone(),
                InventoryClosureState::Unknown,
            );
        }

        if self.snapshot.snapshot_token_sha256.trim().is_empty() {
            push(
                InventoryDiagnosticCode::SnapshotTokenNotPinned,
                self.snapshot.snapshot_id.clone(),
                InventoryClosureState::Unknown,
            );
        }

        let evidence_records = std::iter::once(&self.snapshot.source_snapshot_evidence)
            .chain(std::iter::once(&self.traversal.closure_evidence))
            .chain(self.pages.iter().map(|page| &page.evidence))
            .chain(self.exclusions.iter().map(|exclusion| &exclusion.evidence));

        for evidence in evidence_records {
            if evidence.received_at_ms() > as_known_at_ms {
                push(
                    InventoryDiagnosticCode::EvidenceUnavailableAtCutoff,
                    evidence.source_receipt_id().to_owned(),
                    InventoryClosureState::Unknown,
                );
                continue;
            }
            match evidence_verifier.verify(evidence, as_known_at_ms) {
                EvidenceAdmission::Accepted => {}
                EvidenceAdmission::Rejected => push(
                    InventoryDiagnosticCode::EvidenceAdmissionRejected,
                    evidence.source_receipt_id().to_owned(),
                    InventoryClosureState::Conflicted,
                ),
                EvidenceAdmission::Unknown => push(
                    InventoryDiagnosticCode::EvidenceAdmissionUnknown,
                    evidence.source_receipt_id().to_owned(),
                    InventoryClosureState::Unknown,
                ),
            }
        }

        match source_verifier.verify_snapshot(
            &self.scope,
            &self.snapshot,
            &self.traversal,
            as_known_at_ms,
        ) {
            SnapshotVerification::Stable => {}
            SnapshotVerification::Changed => push(
                InventoryDiagnosticCode::SnapshotChanged,
                self.snapshot.snapshot_id.clone(),
                InventoryClosureState::Conflicted,
            ),
            SnapshotVerification::Unknown => push(
                InventoryDiagnosticCode::SnapshotVerificationUnknown,
                self.snapshot.snapshot_id.clone(),
                InventoryClosureState::Unknown,
            ),
            SnapshotVerification::Invalid => push(
                InventoryDiagnosticCode::SnapshotVerificationRejected,
                self.snapshot.snapshot_id.clone(),
                InventoryClosureState::Conflicted,
            ),
        }

        match source_verifier.verify_scope(&self.scope, &self.exclusions, as_known_at_ms) {
            ScopeVerification::Verified => {}
            ScopeVerification::VerifiedWithDeclaredLimitations => {
                scope_limited = true;
            }
            ScopeVerification::Unknown => push(
                InventoryDiagnosticCode::ScopeVerificationUnknown,
                self.scope.source_system_id.clone(),
                InventoryClosureState::Unknown,
            ),
            ScopeVerification::Rejected => push(
                InventoryDiagnosticCode::ScopeVerificationRejected,
                self.scope.source_system_id.clone(),
                InventoryClosureState::Conflicted,
            ),
        }

        let mut available_pages = self
            .pages
            .iter()
            .filter(|page| page.received_at_ms <= as_known_at_ms)
            .collect::<Vec<_>>();
        if available_pages.len() != self.pages.len() {
            push(
                InventoryDiagnosticCode::PageUnavailableAtCutoff,
                self.snapshot.snapshot_id.clone(),
                InventoryClosureState::Incomplete,
            );
        }
        available_pages.sort_by_key(|page| (page.page_index, page.page_id.as_str()));

        let expected_page_count = self.traversal.expected_page_count;
        if let Some(expected) = expected_page_count {
            if available_pages.len() != expected as usize {
                push(
                    InventoryDiagnosticCode::PageCountMismatch,
                    self.snapshot.snapshot_id.clone(),
                    InventoryClosureState::Incomplete,
                );
            }
        }

        let mut seen_cursors = BTreeSet::new();
        for (position, page) in available_pages.iter().enumerate() {
            if page.evidence.digest_sha256() != page.page_payload_sha256 {
                push(
                    InventoryDiagnosticCode::PageArtifactDigestMismatch,
                    page.page_id.clone(),
                    InventoryClosureState::Conflicted,
                );
            }
            if page.evidence.received_at_ms() != page.received_at_ms {
                push(
                    InventoryDiagnosticCode::PageReceiptTimeMismatch,
                    page.page_id.clone(),
                    InventoryClosureState::Conflicted,
                );
            }
            if page.source_revision != self.snapshot.source_revision {
                push(
                    InventoryDiagnosticCode::SourceRevisionChanged,
                    page.page_id.clone(),
                    InventoryClosureState::Conflicted,
                );
            }
            if page.snapshot_token_sha256 != self.snapshot.snapshot_token_sha256 {
                push(
                    InventoryDiagnosticCode::SnapshotTokenMismatch,
                    page.page_id.clone(),
                    InventoryClosureState::Conflicted,
                );
            }
            if page.page_index as usize != position {
                push(
                    InventoryDiagnosticCode::MissingPage,
                    page.page_id.clone(),
                    InventoryClosureState::Incomplete,
                );
            }
            if position == 0 && page.request_cursor_sha256.is_some() {
                push(
                    InventoryDiagnosticCode::FirstPageCursorNotEmpty,
                    page.page_id.clone(),
                    InventoryClosureState::Incomplete,
                );
            }
            if position > 0 {
                let previous = available_pages[position - 1];
                if previous.next_cursor_sha256 != page.request_cursor_sha256 {
                    push(
                        InventoryDiagnosticCode::CursorChainDiscontinuity,
                        page.page_id.clone(),
                        InventoryClosureState::Incomplete,
                    );
                }
            }
            if let Some(cursor) = &page.request_cursor_sha256 {
                if !seen_cursors.insert(cursor.clone()) {
                    push(
                        InventoryDiagnosticCode::CursorCycle,
                        page.page_id.clone(),
                        InventoryClosureState::Conflicted,
                    );
                }
            }
            if page.terminal && position + 1 != available_pages.len() {
                push(
                    InventoryDiagnosticCode::TerminalPageNotLast,
                    page.page_id.clone(),
                    InventoryClosureState::Conflicted,
                );
            }
        }

        let terminal_found = available_pages.last().is_some_and(|page| page.terminal);
        if self.traversal.terminal_boundary_required && !terminal_found {
            push(
                InventoryDiagnosticCode::TerminalBoundaryMissing,
                self.snapshot.snapshot_id.clone(),
                InventoryClosureState::Incomplete,
            );
        }

        let page_id_map = available_pages
            .iter()
            .map(|page| (page.page_id.as_str(), *page))
            .collect::<BTreeMap<_, _>>();
        let mut page_identities = BTreeSet::new();
        for page in &available_pages {
            for identity in &page.record_identities {
                if !page_identities.insert(identity.clone()) {
                    push(
                        InventoryDiagnosticCode::DuplicateSourceRecordIdentity,
                        format!("{}:{}", identity.namespace, identity.source_id),
                        InventoryClosureState::Conflicted,
                    );
                }
            }
        }

        let mut record_map = BTreeMap::new();
        for record in &self.records {
            let Some(page) = page_id_map.get(record.page_id.as_str()) else {
                push(
                    InventoryDiagnosticCode::RecordPageUnavailableAtCutoff,
                    record.identity.source_id.clone(),
                    InventoryClosureState::Incomplete,
                );
                continue;
            };
            if record.source_revision != self.snapshot.source_revision
                || record.source_revision != page.source_revision
            {
                push(
                    InventoryDiagnosticCode::RecordRevisionMismatch,
                    record.identity.source_id.clone(),
                    InventoryClosureState::Conflicted,
                );
            }
            if !self
                .scope
                .included_record_classes
                .contains(&record.source_record_class)
            {
                push(
                    InventoryDiagnosticCode::RecordOutsideIncludedClass,
                    record.identity.source_id.clone(),
                    InventoryClosureState::Conflicted,
                );
            }
            if !page.record_identities.contains(&record.identity) {
                push(
                    InventoryDiagnosticCode::PageRecordInventoryMismatch,
                    record.identity.source_id.clone(),
                    InventoryClosureState::Conflicted,
                );
            }
            record_map.insert(record.identity.clone(), record);
        }
        for identity in &page_identities {
            if !record_map.contains_key(identity) {
                push(
                    InventoryDiagnosticCode::PageRecordInventoryMismatch,
                    identity.source_id.clone(),
                    InventoryClosureState::Incomplete,
                );
            }
        }

        let mut record_disposition_counts: BTreeMap<SourceRecordIdentity, usize> = BTreeMap::new();
        let mut unsupported = false;
        let mut ambiguous = false;
        let mut losses = false;
        for disposition in &self.dispositions {
            if !record_map.contains_key(&disposition.identity) {
                push(
                    InventoryDiagnosticCode::UnknownDispositionRecord,
                    disposition.identity.source_id.clone(),
                    InventoryClosureState::Conflicted,
                );
            }
            *record_disposition_counts
                .entry(disposition.identity.clone())
                .or_default() += 1;
            match disposition.kind {
                SemanticDispositionKind::Unsupported => unsupported = true,
                SemanticDispositionKind::Ambiguous => ambiguous = true,
                SemanticDispositionKind::LossExplicit => losses = true,
                SemanticDispositionKind::UnavailableWithReason => unsupported = true,
                SemanticDispositionKind::Mapped | SemanticDispositionKind::ExcludedWithReason => {}
            }
            if let Some(exclusion_ref) = &disposition.exclusion_ref {
                if !self.scope.explicit_exclusion_ids.contains(exclusion_ref) {
                    push(
                        InventoryDiagnosticCode::UnknownExclusionReference,
                        exclusion_ref.clone(),
                        InventoryClosureState::Conflicted,
                    );
                }
            }
        }

        let mut accounted = 0usize;
        for identity in record_map.keys() {
            match record_disposition_counts.get(identity).copied().unwrap_or(0) {
                0 => push(
                    InventoryDiagnosticCode::MissingDisposition,
                    identity.source_id.clone(),
                    InventoryClosureState::Incomplete,
                ),
                1 => accounted += 1,
                _ => push(
                    InventoryDiagnosticCode::DuplicateDisposition,
                    identity.source_id.clone(),
                    InventoryClosureState::Conflicted,
                ),
            }
        }

        let declared_exclusion_ids = &self.scope.explicit_exclusion_ids;
        let observed_exclusion_ids = self
            .exclusions
            .iter()
            .map(|exclusion| exclusion.exclusion_id.clone())
            .collect::<BTreeSet<_>>();
        if declared_exclusion_ids != &observed_exclusion_ids {
            push(
                InventoryDiagnosticCode::ExclusionScopeMismatch,
                self.scope.source_system_id.clone(),
                InventoryClosureState::Conflicted,
            );
        }

        if let Some(expected) = self.traversal.expected_record_count {
            if record_map.len() as u64 != expected {
                push(
                    InventoryDiagnosticCode::RecordCountMismatch,
                    self.snapshot.snapshot_id.clone(),
                    InventoryClosureState::Incomplete,
                );
            }
        }

        if ambiguous {
            semantic = InventorySemanticState::IncompleteUnsupportedOrAmbiguous;
            diagnostics.insert(InventoryDiagnostic {
                code: InventoryDiagnosticCode::SemanticInventoryContainsAmbiguous,
                entity_ref: self.snapshot.snapshot_id.clone(),
            });
        } else if unsupported {
            semantic = InventorySemanticState::IncompleteUnsupportedOrAmbiguous;
            diagnostics.insert(InventoryDiagnostic {
                code: InventoryDiagnosticCode::SemanticInventoryContainsUnsupported,
                entity_ref: self.snapshot.snapshot_id.clone(),
            });
        } else if losses {
            semantic = InventorySemanticState::CompleteWithDeclaredLoss;
            diagnostics.insert(InventoryDiagnostic {
                code: InventoryDiagnosticCode::SemanticInventoryContainsDeclaredLoss,
                entity_ref: self.snapshot.snapshot_id.clone(),
            });
        }

        if closure != InventoryClosureState::CompleteWithinDeclaredScope {
            semantic = InventorySemanticState::Unknown;
        }

        let diagnostics = diagnostics.into_iter().collect::<Vec<_>>();
        let manifest_sha256 = self
            .manifest_sha256()
            .unwrap_or_else(|_| String::new());
        InventoryClosureReport {
            source_system_id: self.scope.source_system_id.clone(),
            snapshot_id: self.snapshot.snapshot_id.clone(),
            as_known_at_ms,
            manifest_sha256,
            closure_state: closure,
            semantic_state: semantic,
            scope_limited,
            declared_exclusion_ids: self.scope.explicit_exclusion_ids.iter().cloned().collect(),
            enumerated_record_count: record_map.len(),
            accounted_record_count: accounted,
            page_count: available_pages.len(),
            diagnostics,
        }
    }
}

fn combine_closure(
    current: InventoryClosureState,
    next: InventoryClosureState,
) -> InventoryClosureState {
    use InventoryClosureState::{CompleteWithinDeclaredScope, Conflicted, Incomplete, Unknown};
    match (current, next) {
        (Conflicted, _) | (_, Conflicted) => Conflicted,
        (Incomplete, _) | (_, Incomplete) => Incomplete,
        (Unknown, _) | (_, Unknown) => Unknown,
        (CompleteWithinDeclaredScope, CompleteWithinDeclaredScope) => CompleteWithinDeclaredScope,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const D1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const D2: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    const D3: &str = "3333333333333333333333333333333333333333333333333333333333333333";
    const D4: &str = "4444444444444444444444444444444444444444444444444444444444444444";

    struct TestEvidenceVerifier(EvidenceAdmission);

    impl EvidenceAdmissionVerifier for TestEvidenceVerifier {
        fn verify(&self, _evidence: &EvidenceRecord, _as_known_at_ms: i64) -> EvidenceAdmission {
            self.0
        }
    }

    struct TestSourceVerifier(SnapshotVerification, ScopeVerification);

    impl SourceInventoryVerifier for TestSourceVerifier {
        fn verify_snapshot(
            &self,
            _scope: &InventoryScope,
            _snapshot: &InventorySnapshot,
            _traversal: &InventoryTraversalContract,
            _as_known_at_ms: i64,
        ) -> SnapshotVerification {
            self.0
        }

        fn verify_scope(
            &self,
            _scope: &InventoryScope,
            _exclusions: &[InventoryExclusion],
            _as_known_at_ms: i64,
        ) -> ScopeVerification {
            self.1
        }
    }

    fn evidence(id: &str, digest: &str, captured: i64, received: i64) -> EvidenceRecord {
        EvidenceRecord::new(id, digest, format!("receipt-{id}"), captured, received).unwrap()
    }

    fn identity(namespace: &str, id: &str) -> SourceRecordIdentity {
        SourceRecordIdentity::new(namespace, id).unwrap()
    }

    fn make_manifest(
        pages: Vec<InventoryPage>,
        records: Vec<InventoryRecord>,
        dispositions: Vec<RecordDisposition>,
        kind: TraversalKind,
    ) -> SourceInventoryManifest {
        let scope = InventoryScope::new(
            "source-a",
            "urn:source:a",
            "adapter-profile-a",
            "0.1",
            D1,
            D2,
            ["device", "equipment"].into_iter().map(str::to_owned).collect(),
            "permission-receipt",
            ["archive-exclusion"].into_iter().map(str::to_owned).collect(),
            None,
            None,
        )
        .unwrap();
        let snapshot = InventorySnapshot::new(
            "snapshot-r1",
            "source-r1",
            if kind == TraversalKind::BestEffort {
                SnapshotConsistencyModel::BestEffortMutable
            } else {
                SnapshotConsistencyModel::SnapshotPaginatedApi
            },
            D3,
            10,
            40,
            evidence("snapshot", D1, 10, 10),
        )
        .unwrap();
        let traversal = InventoryTraversalContract::new(
            kind,
            "contract-v1",
            Some(3),
            Some(3),
            true,
            Some(D4.to_owned()),
            evidence("closure", D2, 10, 10),
        )
        .unwrap();
        let exclusion = InventoryExclusion::new(
            "archive-exclusion",
            "partition",
            "archive-prior-to-cutoff",
            "Explicitly excluded by the inventory scope",
            evidence("exclusion", D3, 10, 10),
        )
        .unwrap();
        SourceInventoryManifest::new(
            scope,
            snapshot,
            traversal,
            pages,
            records,
            dispositions,
            vec![exclusion],
        )
        .unwrap()
    }

    fn complete_manifest() -> SourceInventoryManifest {
        let i1 = identity("urn:source:a", "r1");
        let i2 = identity("urn:source:a", "r2");
        let i3 = identity("urn:source:b", "r1");
        let pages = vec![
            InventoryPage::new(
                "p0",
                0,
                None,
                Some(D1.to_owned()),
                false,
                "source-r1",
                D3,
                D1,
                11,
                evidence("p0", D1, 10, 11),
                vec![i1.clone()],
            )
            .unwrap(),
            InventoryPage::new(
                "p1",
                1,
                Some(D1.to_owned()),
                Some(D2.to_owned()),
                false,
                "source-r1",
                D3,
                D2,
                20,
                evidence("p1", D2, 10, 20),
                vec![i2.clone()],
            )
            .unwrap(),
            InventoryPage::new(
                "p2",
                2,
                Some(D2.to_owned()),
                None,
                true,
                "source-r1",
                D3,
                D3,
                30,
                evidence("p2", D3, 10, 30),
                vec![i3.clone()],
            )
            .unwrap(),
        ];
        let records = vec![
            InventoryRecord::new(i1.clone(), "device", "p0", "source-r1", D1).unwrap(),
            InventoryRecord::new(i2.clone(), "equipment", "p1", "source-r1", D2).unwrap(),
            InventoryRecord::new(i3.clone(), "device", "p2", "source-r1", D3).unwrap(),
        ];
        let dispositions = vec![
            RecordDisposition::new(
                i1,
                SemanticDispositionKind::Mapped,
                Some("mapping-1".to_owned()),
                None,
                "Mapped",
            )
            .unwrap(),
            RecordDisposition::new(
                i2,
                SemanticDispositionKind::LossExplicit,
                Some("mapping-2".to_owned()),
                None,
                "One qualifier loss is explicitly recorded",
            )
            .unwrap(),
            RecordDisposition::new(
                i3,
                SemanticDispositionKind::Mapped,
                Some("mapping-3".to_owned()),
                None,
                "Mapped",
            )
            .unwrap(),
        ];
        make_manifest(pages, records, dispositions, TraversalKind::CursorPaginatedApi)
    }

    #[test]
    fn stable_snapshot_with_complete_cursor_chain_closes_with_declared_loss() {
        let report = complete_manifest().project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::CompleteWithinDeclaredScope);
        assert_eq!(report.semantic_state, InventorySemanticState::CompleteWithDeclaredLoss);
        assert_eq!(report.enumerated_record_count, 3);
        assert_eq!(report.accounted_record_count, 3);
    }

    #[test]
    fn best_effort_mutable_snapshot_is_never_complete() {
        let mut manifest = complete_manifest();
        manifest.snapshot.consistency_model = SnapshotConsistencyModel::BestEffortMutable;
        manifest.traversal.kind = TraversalKind::BestEffort;
        let report = manifest.project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::Unknown);
        assert!(report.diagnostics.iter().any(|item| {
            item.code == InventoryDiagnosticCode::SnapshotConsistencyBestEffort
        }));
    }

    #[test]
    fn missing_page_breaks_cursor_closure() {
        let manifest = complete_manifest();
        let mut pages = manifest.pages.clone();
        pages.remove(1);
        let rebuilt = SourceInventoryManifest::new(
            manifest.scope,
            manifest.snapshot,
            manifest.traversal,
            pages,
            manifest.records,
            manifest.dispositions,
            manifest.exclusions,
        )
        .unwrap();
        let report = rebuilt.project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::Incomplete);
        assert!(report
            .diagnostics
            .iter()
            .any(|item| item.code == InventoryDiagnosticCode::PageCountMismatch));
    }

    #[test]
    fn unknown_evidence_admission_blocks_closure() {
        let report = complete_manifest().project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Unknown),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::Unknown);
    }

    #[test]
    fn source_revision_drift_conflicts_with_snapshot() {
        let mut manifest = complete_manifest();
        manifest.pages[1].source_revision = "source-r2".to_owned();
        let report = manifest.project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::Conflicted);
        assert!(report
            .diagnostics
            .iter()
            .any(|item| item.code == InventoryDiagnosticCode::SourceRevisionChanged));
    }

    #[test]
    fn late_page_is_not_visible_to_historical_cutoff() {
        let mut manifest = complete_manifest();
        manifest.pages[2].received_at_ms = 120;
        let report = manifest.project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::Incomplete);
        assert!(report
            .diagnostics
            .iter()
            .any(|item| item.code == InventoryDiagnosticCode::PageUnavailableAtCutoff));
    }

    #[test]
    fn page_snapshot_token_mismatch_is_conflicted() {
        let mut manifest = complete_manifest();
        manifest.pages[1].snapshot_token_sha256 = D4.to_owned();
        let report = manifest.project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::Conflicted);
        assert!(report.diagnostics.iter().any(|item| {
            item.code == InventoryDiagnosticCode::SnapshotTokenMismatch
        }));
    }

    #[test]
    fn cursor_discontinuity_is_incomplete() {
        let mut manifest = complete_manifest();
        manifest.pages[1].request_cursor_sha256 = Some(D4.to_owned());
        let report = manifest.project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::Incomplete);
        assert!(report.diagnostics.iter().any(|item| {
            item.code == InventoryDiagnosticCode::CursorChainDiscontinuity
        }));
    }

    #[test]
    fn unsupported_record_is_still_counted_in_complete_inventory() {
        let mut manifest = complete_manifest();
        manifest.dispositions[0].kind = SemanticDispositionKind::Unsupported;
        manifest.dispositions[0].mapping_record_ref = None;
        let report = manifest.project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::CompleteWithinDeclaredScope);
        assert_eq!(
            report.semantic_state,
            InventorySemanticState::IncompleteUnsupportedOrAmbiguous
        );
    }

    #[test]
    fn missing_record_disposition_makes_semantic_inventory_incomplete() {
        let mut manifest = complete_manifest();
        manifest.dispositions.pop();
        let report = manifest.project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::Incomplete);
        assert_eq!(report.accounted_record_count, 2);
        assert!(report.diagnostics.iter().any(|item| {
            item.code == InventoryDiagnosticCode::MissingDisposition
        }));
    }

    #[test]
    fn changed_provider_snapshot_is_a_conflict() {
        let report = complete_manifest().project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Changed, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::Conflicted);
        assert!(report.diagnostics.iter().any(|item| {
            item.code == InventoryDiagnosticCode::SnapshotChanged
        }));
    }

    #[test]
    fn unknown_permission_or_scope_verification_never_closes() {
        let report = complete_manifest().project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Unknown),
        );
        assert_eq!(report.closure_state, InventoryClosureState::Unknown);
    }

    #[test]
    fn explicit_scope_limitations_are_disclosed_in_report() {
        let report = complete_manifest().project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(
                SnapshotVerification::Stable,
                ScopeVerification::VerifiedWithDeclaredLimitations,
            ),
        );
        assert!(report.scope_limited);
        assert_eq!(report.declared_exclusion_ids, vec!["archive-exclusion".to_owned()]);
    }

    #[test]
    fn page_payload_digest_must_match_admitted_page_evidence() {
        let mut manifest = complete_manifest();
        manifest.pages[0].evidence = evidence("p0", D4, 10, 11);
        let report = manifest.project(
            100,
            &TestEvidenceVerifier(EvidenceAdmission::Accepted),
            &TestSourceVerifier(SnapshotVerification::Stable, ScopeVerification::Verified),
        );
        assert_eq!(report.closure_state, InventoryClosureState::Conflicted);
        assert!(report.diagnostics.iter().any(|item| {
            item.code == InventoryDiagnosticCode::PageArtifactDigestMismatch
        }));
    }

    #[test]
    fn record_identity_is_namespaced() {
        let first = identity("urn:source:a", "same-id");
        let second = identity("urn:source:b", "same-id");
        assert_ne!(first, second);
    }

    #[test]
    fn duplicate_identity_in_same_namespace_is_rejected_at_manifest_boundary() {
        let manifest = complete_manifest();
        let mut records = manifest.records.clone();
        records.push(records[0].clone());
        let result = SourceInventoryManifest::new(
            manifest.scope,
            manifest.snapshot,
            manifest.traversal,
            manifest.pages,
            records,
            manifest.dispositions,
            manifest.exclusions,
        );
        assert!(matches!(
            result,
            Err(InventoryError::DuplicateRecordIdentity { .. })
        ));
    }

    #[test]
    fn unordered_pages_are_normalized_before_manifest_commitment() {
        let manifest = complete_manifest();
        let mut reversed_pages = manifest.pages.clone();
        reversed_pages.reverse();
        let rebuilt = SourceInventoryManifest::new(
            manifest.scope,
            manifest.snapshot,
            manifest.traversal,
            reversed_pages,
            manifest.records,
            manifest.dispositions,
            manifest.exclusions,
        )
        .unwrap();
        assert_eq!(manifest.manifest_sha256().unwrap(), rebuilt.manifest_sha256().unwrap());
    }
}
