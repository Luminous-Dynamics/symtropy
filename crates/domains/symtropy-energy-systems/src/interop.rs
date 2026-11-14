// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Loss-aware standards mapping ledger and semantic round-trip comparator.
//!
//! This module defines a conformance harness. It does not implement IEC, CIM,
//! SCL, IEEE, or SANS adapters and does not claim standards conformity.

use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StandardPublicationStatus {
    Published,
    WorkingDraft,
    TechnicalReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StandardReference {
    designation: String,
    part: Option<String>,
    edition: String,
    amendment: Option<String>,
    publication_status: StandardPublicationStatus,
    jurisdiction: Option<String>,
}

impl StandardReference {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        designation: impl Into<String>,
        part: Option<String>,
        edition: impl Into<String>,
        amendment: Option<String>,
        publication_status: StandardPublicationStatus,
        jurisdiction: Option<String>,
    ) -> Result<Self, InteropError> {
        let designation = designation.into();
        let edition = edition.into();
        require_non_empty("standard_designation", &designation)?;
        require_non_empty("standard_edition", &edition)?;
        if let Some(value) = &part { require_non_empty("standard_part", value)?; }
        if let Some(value) = &amendment { require_non_empty("standard_amendment", value)?; }
        if let Some(value) = &jurisdiction { require_non_empty("standard_jurisdiction", value)?; }
        Ok(Self { designation, part, edition, amendment, publication_status, jurisdiction })
    }

    pub fn designation(&self) -> &str { &self.designation }
    pub fn part(&self) -> Option<&str> { self.part.as_deref() }
    pub fn edition(&self) -> &str { &self.edition }
    pub fn amendment(&self) -> Option<&str> { self.amendment.as_deref() }
    pub const fn publication_status(&self) -> StandardPublicationStatus { self.publication_status }
    pub fn jurisdiction(&self) -> Option<&str> { self.jurisdiction.as_deref() }

    /// A working draft or technical report is not silently promoted to a
    /// published normative edition by the adapter profile.
    pub const fn is_published_edition(&self) -> bool {
        matches!(self.publication_status, StandardPublicationStatus::Published)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingDirection {
    Import,
    Export,
    RoundTrip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingProfile {
    profile_id: String,
    profile_version: String,
    adapter_version: String,
    direction: MappingDirection,
    standard: StandardReference,
}

impl MappingProfile {
    pub fn new(
        profile_id: impl Into<String>,
        profile_version: impl Into<String>,
        adapter_version: impl Into<String>,
        direction: MappingDirection,
        standard: StandardReference,
    ) -> Result<Self, InteropError> {
        let profile_id = profile_id.into();
        let profile_version = profile_version.into();
        let adapter_version = adapter_version.into();
        require_non_empty("mapping_profile_id", &profile_id)?;
        require_non_empty("mapping_profile_version", &profile_version)?;
        require_non_empty("adapter_version", &adapter_version)?;
        Ok(Self { profile_id, profile_version, adapter_version, direction, standard })
    }

    pub fn profile_id(&self) -> &str { &self.profile_id }
    pub fn profile_version(&self) -> &str { &self.profile_version }
    pub fn adapter_version(&self) -> &str { &self.adapter_version }
    pub const fn direction(&self) -> MappingDirection { self.direction }
    pub fn standard(&self) -> &StandardReference { &self.standard }
    pub const fn normatively_eligible(&self) -> bool { self.standard.is_published_edition() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticItemKind {
    Object,
    Relationship,
    Property,
    Extension,
    Namespace,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SemanticItem {
    item_id: String,
    kind: SemanticItemKind,
    namespace: String,
    semantic_digest: String,
    in_scope: bool,
}

impl SemanticItem {
    pub fn new(
        item_id: impl Into<String>,
        kind: SemanticItemKind,
        namespace: impl Into<String>,
        semantic_digest: impl Into<String>,
        in_scope: bool,
    ) -> Result<Self, InteropError> {
        let item_id = item_id.into();
        let namespace = namespace.into();
        let semantic_digest = semantic_digest.into();
        require_non_empty("semantic_item_id", &item_id)?;
        require_non_empty("semantic_namespace", &namespace)?;
        require_non_empty("semantic_digest", &semantic_digest)?;
        Ok(Self { item_id, kind, namespace, semantic_digest, in_scope })
    }

    pub fn item_id(&self) -> &str { &self.item_id }
    pub const fn kind(&self) -> SemanticItemKind { self.kind }
    pub fn namespace(&self) -> &str { &self.namespace }
    pub fn semantic_digest(&self) -> &str { &self.semantic_digest }
    pub const fn in_scope(&self) -> bool { self.in_scope }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SemanticInventory {
    document_id: String,
    document_digest: String,
    #[serde(serialize_with = "serialize_map_values")]
    items: BTreeMap<String, SemanticItem>,
}

impl SemanticInventory {
    pub fn new(
        document_id: impl Into<String>,
        document_digest: impl Into<String>,
        items: Vec<SemanticItem>,
    ) -> Result<Self, InteropError> {
        let document_id = document_id.into();
        let document_digest = document_digest.into();
        require_non_empty("semantic_document_id", &document_id)?;
        require_non_empty("semantic_document_digest", &document_digest)?;
        let mut by_id = BTreeMap::new();
        for item in items {
            let id = item.item_id.clone();
            if by_id.insert(id.clone(), item).is_some() {
                return Err(InteropError::DuplicateSemanticItem { item_id: id });
            }
        }
        Ok(Self { document_id, document_digest, items: by_id })
    }

    pub fn document_id(&self) -> &str { &self.document_id }
    pub fn document_digest(&self) -> &str { &self.document_digest }
    pub fn items(&self) -> &BTreeMap<String, SemanticItem> { &self.items }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformationOutcome {
    ExactPreservation,
    SemanticallyEquivalent,
    ExplicitlyLossy,
    Unsupported,
    Ambiguous,
    NotInScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InformationLossCategory {
    Identity,
    Relationship,
    Property,
    Unit,
    Precision,
    Namespace,
    Extension,
    Ordering,
    Signature,
    ExternalReference,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InformationLoss {
    loss_id: String,
    category: InformationLossCategory,
    description: String,
}

impl InformationLoss {
    pub fn new(
        loss_id: impl Into<String>,
        category: InformationLossCategory,
        description: impl Into<String>,
    ) -> Result<Self, InteropError> {
        let loss_id = loss_id.into();
        let description = description.into();
        require_non_empty("information_loss_id", &loss_id)?;
        require_non_empty("information_loss_description", &description)?;
        Ok(Self { loss_id, category, description })
    }
    pub fn loss_id(&self) -> &str { &self.loss_id }
    pub const fn category(&self) -> InformationLossCategory { self.category }
    pub fn description(&self) -> &str { &self.description }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingEntry {
    source_item_id: String,
    outcome: TransformationOutcome,
    target_item_ids: Vec<String>,
    mapping_rule_id: Option<String>,
    information_losses: Vec<InformationLoss>,
    explanation: Option<String>,
}

impl MappingEntry {
    pub fn new(
        source_item_id: impl Into<String>,
        outcome: TransformationOutcome,
        mut target_item_ids: Vec<String>,
        mapping_rule_id: Option<String>,
        mut information_losses: Vec<InformationLoss>,
        explanation: Option<String>,
    ) -> Result<Self, InteropError> {
        let source_item_id = source_item_id.into();
        require_non_empty("mapping_source_item_id", &source_item_id)?;
        for id in &target_item_ids { require_non_empty("mapping_target_item_id", id)?; }
        target_item_ids.sort();
        if target_item_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(InteropError::DuplicateTargetItem { source_item_id });
        }
        for loss in &information_losses { require_non_empty("information_loss_id", &loss.loss_id)?; }
        information_losses.sort_by(|left, right| left.loss_id.cmp(&right.loss_id));
        if information_losses.windows(2).any(|pair| pair[0].loss_id == pair[1].loss_id) {
            return Err(InteropError::DuplicateInformationLoss { source_item_id });
        }
        if let Some(id) = &mapping_rule_id { require_non_empty("mapping_rule_id", id)?; }
        if let Some(note) = &explanation { require_non_empty("mapping_explanation", note)?; }

        let has_targets = !target_item_ids.is_empty();
        let has_losses = !information_losses.is_empty();
        let has_rule = mapping_rule_id.is_some();
        let has_explanation = explanation.is_some();

        let valid = match outcome {
            TransformationOutcome::ExactPreservation | TransformationOutcome::SemanticallyEquivalent =>
                has_targets && has_rule && !has_losses,
            TransformationOutcome::ExplicitlyLossy =>
                has_targets && has_rule && has_losses,
            TransformationOutcome::Unsupported =>
                !has_targets && !has_losses && has_explanation,
            TransformationOutcome::Ambiguous =>
                target_item_ids.len() >= 2 && !has_losses && has_explanation,
            TransformationOutcome::NotInScope =>
                !has_targets && !has_losses && has_explanation,
        };
        if !valid {
            return Err(InteropError::InvalidMappingOutcome { source_item_id, outcome });
        }

        Ok(Self { source_item_id, outcome, target_item_ids, mapping_rule_id, information_losses, explanation })
    }

    pub fn source_item_id(&self) -> &str { &self.source_item_id }
    pub const fn outcome(&self) -> TransformationOutcome { self.outcome }
    pub fn target_item_ids(&self) -> &[String] { &self.target_item_ids }
    pub fn mapping_rule_id(&self) -> Option<&str> { self.mapping_rule_id.as_deref() }
    pub fn information_losses(&self) -> &[InformationLoss] { &self.information_losses }
    pub fn explanation(&self) -> Option<&str> { self.explanation.as_deref() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingLedgerStatus {
    Complete,
    CompleteWithExplicitLosses,
    IncompleteUnsupported,
    IncompleteAmbiguous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingLedger {
    ledger_id: String,
    profile: MappingProfile,
    source_inventory: SemanticInventory,
    target_document_digest: String,
    #[serde(serialize_with = "serialize_map_values")]
    entries: BTreeMap<String, MappingEntry>,
    status: MappingLedgerStatus,
}

impl MappingLedger {
    pub fn new(
        ledger_id: impl Into<String>,
        profile: MappingProfile,
        source_inventory: SemanticInventory,
        target_document_digest: impl Into<String>,
        entries: Vec<MappingEntry>,
    ) -> Result<Self, InteropError> {
        let ledger_id = ledger_id.into();
        let target_document_digest = target_document_digest.into();
        require_non_empty("mapping_ledger_id", &ledger_id)?;
        require_non_empty("target_document_digest", &target_document_digest)?;

        let mut by_source = BTreeMap::new();
        for entry in entries {
            let source_id = entry.source_item_id.clone();
            let Some(source) = source_inventory.items.get(&source_id) else {
                return Err(InteropError::UnknownSourceSemanticItem { item_id: source_id });
            };
            if source.in_scope && entry.outcome == TransformationOutcome::NotInScope {
                return Err(InteropError::InScopeItemMarkedOutOfScope { item_id: source_id });
            }
            if !source.in_scope && entry.outcome != TransformationOutcome::NotInScope {
                return Err(InteropError::OutOfScopeItemMapped { item_id: source_id });
            }
            if by_source.insert(source_id.clone(), entry).is_some() {
                return Err(InteropError::DuplicateMappingEntry { source_item_id: source_id });
            }
        }

        let missing: Vec<String> = source_inventory.items.keys()
            .filter(|id| !by_source.contains_key(*id))
            .cloned()
            .collect();
        if !missing.is_empty() {
            return Err(InteropError::MissingMappingEntries { source_item_ids: missing });
        }

        let status = derive_ledger_status(by_source.values());
        Ok(Self { ledger_id, profile, source_inventory, target_document_digest, entries: by_source, status })
    }

    pub fn ledger_id(&self) -> &str { &self.ledger_id }
    pub fn profile(&self) -> &MappingProfile { &self.profile }
    pub fn source_inventory(&self) -> &SemanticInventory { &self.source_inventory }
    pub fn target_document_digest(&self) -> &str { &self.target_document_digest }
    pub fn entries(&self) -> &BTreeMap<String, MappingEntry> { &self.entries }
    pub const fn status(&self) -> MappingLedgerStatus { self.status }
}

fn derive_ledger_status<'a>(entries: impl Iterator<Item = &'a MappingEntry>) -> MappingLedgerStatus {
    let mut explicit_loss = false;
    let mut unsupported = false;
    let mut ambiguous = false;
    for entry in entries {
        match entry.outcome {
            TransformationOutcome::ExplicitlyLossy => explicit_loss = true,
            TransformationOutcome::Unsupported => unsupported = true,
            TransformationOutcome::Ambiguous => ambiguous = true,
            _ => {}
        }
    }
    if unsupported {
        MappingLedgerStatus::IncompleteUnsupported
    } else if ambiguous {
        MappingLedgerStatus::IncompleteAmbiguous
    } else if explicit_loss {
        MappingLedgerStatus::CompleteWithExplicitLosses
    } else {
        MappingLedgerStatus::Complete
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundTripStatus {
    SemanticallyEquivalent,
    EquivalentWithExplicitLosses,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoundTripDifference {
    source_item_id: String,
    source_digest: String,
    round_trip_digest: Option<String>,
    outcome: TransformationOutcome,
    information_loss_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoundTripResult {
    source_document_id: String,
    source_document_digest: String,
    round_trip_document_id: String,
    round_trip_document_digest: String,
    status: RoundTripStatus,
    preserved_item_ids: Vec<String>,
    differences: Vec<RoundTripDifference>,
    extra_round_trip_item_ids: Vec<String>,
}

impl RoundTripResult {
    pub fn compare(
        ledger: &MappingLedger,
        round_trip_inventory: &SemanticInventory,
    ) -> Result<Self, InteropError> {
        let source = &ledger.source_inventory;
        let mut preserved = Vec::new();
        let mut differences = Vec::new();
        for (source_id, source_item) in &source.items {
            if !source_item.in_scope { continue; }
            let entry = ledger.entries.get(source_id).expect("ledger covers every source item");
            let target_item = round_trip_inventory.items.get(source_id);
            if target_item.is_some_and(|target| target.semantic_digest == source_item.semantic_digest) {
                preserved.push(source_id.clone());
                continue;
            }
            match entry.outcome {
                TransformationOutcome::ExplicitlyLossy | TransformationOutcome::Unsupported | TransformationOutcome::Ambiguous => {
                    differences.push(RoundTripDifference {
                        source_item_id: source_id.clone(),
                        source_digest: source_item.semantic_digest.clone(),
                        round_trip_digest: target_item.map(|item| item.semantic_digest.clone()),
                        outcome: entry.outcome,
                        information_loss_ids: entry.information_losses.iter().map(|loss| loss.loss_id.clone()).collect(),
                    });
                }
                TransformationOutcome::ExactPreservation | TransformationOutcome::SemanticallyEquivalent => {
                    return Err(InteropError::UndeclaredSemanticLoss { item_id: source_id.clone() });
                }
                TransformationOutcome::NotInScope => {
                    return Err(InteropError::InScopeItemMarkedOutOfScope { item_id: source_id.clone() });
                }
            }
        }

        let extras = round_trip_inventory.items.keys()
            .filter(|id| !source.items.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>();
        let has_unresolved = differences.iter().any(|difference| {
            matches!(difference.outcome, TransformationOutcome::Unsupported | TransformationOutcome::Ambiguous)
        });
        let has_explicit_loss = differences.iter().any(|difference| difference.outcome == TransformationOutcome::ExplicitlyLossy);
        let status = if has_unresolved {
            RoundTripStatus::Incomplete
        } else if has_explicit_loss {
            RoundTripStatus::EquivalentWithExplicitLosses
        } else {
            RoundTripStatus::SemanticallyEquivalent
        };

        Ok(Self {
            source_document_id: source.document_id.clone(),
            source_document_digest: source.document_digest.clone(),
            round_trip_document_id: round_trip_inventory.document_id.clone(),
            round_trip_document_digest: round_trip_inventory.document_digest.clone(),
            status,
            preserved_item_ids: preserved,
            differences,
            extra_round_trip_item_ids: extras,
        })
    }

    pub fn source_document_id(&self) -> &str { &self.source_document_id }
    pub fn round_trip_document_id(&self) -> &str { &self.round_trip_document_id }
    pub const fn status(&self) -> RoundTripStatus { self.status }
    pub fn preserved_item_ids(&self) -> &[String] { &self.preserved_item_ids }
    pub fn differences(&self) -> &[RoundTripDifference] { &self.differences }
    pub fn extra_round_trip_item_ids(&self) -> &[String] { &self.extra_round_trip_item_ids }
}

fn serialize_map_values<K, V, S>(values: &BTreeMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    K: Ord,
    V: Serialize,
    S: Serializer,
{
    values.values().collect::<Vec<_>>().serialize(serializer)
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), InteropError> {
    if value.trim().is_empty() { return Err(InteropError::EmptyField { field }); }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteropError {
    EmptyField { field: &'static str },
    DuplicateSemanticItem { item_id: String },
    DuplicateSourceMapping { item_id: String },
    DuplicateTargetItem { source_item_id: String },
    DuplicateInformationLoss { source_item_id: String },
    InvalidMappingOutcome { source_item_id: String, outcome: TransformationOutcome },
    UnknownSourceSemanticItem { item_id: String },
    InScopeItemMarkedOutOfScope { item_id: String },
    OutOfScopeItemMapped { item_id: String },
    DuplicateMappingEntry { source_item_id: String },
    MissingMappingEntries { source_item_ids: Vec<String> },
    UndeclaredSemanticLoss { item_id: String },
}
impl core::fmt::Display for InteropError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for InteropError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(status: StandardPublicationStatus) -> MappingProfile {
        MappingProfile::new(
            "iec-81346-10-power-fabric",
            "0.1",
            "adapter:0.1.0",
            MappingDirection::RoundTrip,
            StandardReference::new("ISO 81346-10", None, "2022", None, status, Some("international".into())).unwrap(),
        ).unwrap()
    }
    fn source() -> SemanticInventory {
        SemanticInventory::new(
            "source-doc", "sha256:source",
            vec![
                SemanticItem::new("object:board", SemanticItemKind::Object, "urn:site", "digest:board", true).unwrap(),
                SemanticItem::new("relationship:feeder", SemanticItemKind::Relationship, "urn:site", "digest:feeder", true).unwrap(),
                SemanticItem::new("extension:vendor", SemanticItemKind::Extension, "urn:vendor", "digest:vendor", true).unwrap(),
                SemanticItem::new("object:out-of-scope", SemanticItemKind::Object, "urn:site", "digest:ignored", false).unwrap(),
            ],
        ).unwrap()
    }
    fn exact(id: &str, target: &str) -> MappingEntry {
        MappingEntry::new(id, TransformationOutcome::ExactPreservation, vec![target.into()],
            Some("rule:exact".into()), Vec::new(), None).unwrap()
    }
    fn ledger(entries: Vec<MappingEntry>) -> Result<MappingLedger, InteropError> {
        MappingLedger::new("ledger:1", profile(StandardPublicationStatus::Published), source(),
            "sha256:target", entries)
    }
    fn roundtrip(items: Vec<SemanticItem>) -> SemanticInventory {
        SemanticInventory::new("round-trip-doc", "sha256:roundtrip", items).unwrap()
    }

    #[test]
    fn published_standard_profile_is_normatively_eligible() {
        assert!(profile(StandardPublicationStatus::Published).normatively_eligible());
    }
    #[test]
    fn working_draft_is_not_silently_treated_as_published() {
        assert!(!profile(StandardPublicationStatus::WorkingDraft).normatively_eligible());
    }
    #[test]
    fn mapping_ledger_requires_one_entry_for_every_source_item() {
        let entries = vec![exact("object:board", "target:board"), exact("relationship:feeder", "target:feeder")];
        assert!(matches!(ledger(entries), Err(InteropError::MissingMappingEntries { .. })));
    }
    #[test]
    fn duplicate_mapping_entry_is_rejected() {
        assert!(matches!(ledger(vec![
            exact("object:board", "target:board"),
            exact("object:board", "target:board2"),
            exact("relationship:feeder", "target:feeder"),
            MappingEntry::new("object:out-of-scope", TransformationOutcome::NotInScope, Vec::new(), None, Vec::new(), Some("outside profile".into())).unwrap(),
        ]), Err(InteropError::DuplicateMappingEntry { .. })));
    }
    #[test]
    fn out_of_scope_items_require_explicit_not_in_scope_entry() {
        let entries = vec![
            exact("object:board", "target:board"),
            exact("relationship:feeder", "target:feeder"),
            exact("extension:vendor", "target:extension"),
            exact("object:out-of-scope", "target:bad"),
        ];
        assert!(matches!(ledger(entries), Err(InteropError::OutOfScopeItemMapped { .. })));
    }
    #[test]
    fn in_scope_item_cannot_be_hidden_as_out_of_scope() {
        let entries = vec![
            MappingEntry::new("object:board", TransformationOutcome::NotInScope, Vec::new(), None, Vec::new(), Some("not mapped".into())).unwrap(),
        ];
        assert!(matches!(ledger(entries), Err(InteropError::InScopeItemMarkedOutOfScope { .. })));
    }
    #[test]
    fn unsupported_and_ambiguous_outcomes_are_not_successful_mappings() {
        let unsupported = MappingEntry::new("object:board", TransformationOutcome::Unsupported, Vec::new(), None, Vec::new(), Some("no target concept".into())).unwrap();
        let ambiguous = MappingEntry::new("object:board", TransformationOutcome::Ambiguous, vec!["candidate:a".into(), "candidate:b".into()], None, Vec::new(), Some("two possible target concepts".into())).unwrap();
        assert_eq!(unsupported.outcome(), TransformationOutcome::Unsupported);
        assert_eq!(ambiguous.outcome(), TransformationOutcome::Ambiguous);
        assert!(matches!(MappingEntry::new("object:board", TransformationOutcome::ExactPreservation, Vec::new(), Some("r".into()), Vec::new(), None), Err(InteropError::InvalidMappingOutcome { .. })));
    }
    #[test]
    fn explicitly_lossy_mapping_requires_machine_readable_loss_record() {
        assert!(matches!(
            MappingEntry::new("object:board", TransformationOutcome::ExplicitlyLossy, vec!["target:board".into()], Some("rule".into()), Vec::new(), None),
            Err(InteropError::InvalidMappingOutcome { .. })
        ));
    }
    #[test]
    fn complete_ledger_distinguishes_explicit_losses_and_unresolved_items() {
        let losses = vec![InformationLoss::new("loss:unit", InformationLossCategory::Unit, "Original unit scale not representable").unwrap()];
        let ledger = ledger(vec![
            exact("object:board", "target:board"),
            MappingEntry::new("relationship:feeder", TransformationOutcome::ExplicitlyLossy, vec!["target:feeder".into()], Some("rule:feeder".into()), losses, None).unwrap(),
            exact("extension:vendor", "target:extension"),
            MappingEntry::new("object:out-of-scope", TransformationOutcome::NotInScope, Vec::new(), None, Vec::new(), Some("outside profile".into())).unwrap(),
        ]).unwrap();
        assert_eq!(ledger.status(), MappingLedgerStatus::CompleteWithExplicitLosses);
    }
    #[test]
    fn unsupported_mapping_makes_ledger_incomplete() {
        let ledger = ledger(vec![
            exact("object:board", "target:board"),
            exact("relationship:feeder", "target:feeder"),
            MappingEntry::new("extension:vendor", TransformationOutcome::Unsupported, Vec::new(), None, Vec::new(), Some("extension namespace unknown".into())).unwrap(),
            MappingEntry::new("object:out-of-scope", TransformationOutcome::NotInScope, Vec::new(), None, Vec::new(), Some("outside profile".into())).unwrap(),
        ]).unwrap();
        assert_eq!(ledger.status(), MappingLedgerStatus::IncompleteUnsupported);
    }
    #[test]
    fn semantically_equivalent_round_trip_preserves_source_semantics() {
        let ledger = ledger(vec![
            exact("object:board", "target:board"),
            exact("relationship:feeder", "target:feeder"),
            exact("extension:vendor", "target:extension"),
            MappingEntry::new("object:out-of-scope", TransformationOutcome::NotInScope, Vec::new(), None, Vec::new(), Some("outside profile".into())).unwrap(),
        ]).unwrap();
        let result = RoundTripResult::compare(&ledger, &roundtrip(vec![
            SemanticItem::new("object:board", SemanticItemKind::Object, "urn:site", "digest:board", true).unwrap(),
            SemanticItem::new("relationship:feeder", SemanticItemKind::Relationship, "urn:site", "digest:feeder", true).unwrap(),
            SemanticItem::new("extension:vendor", SemanticItemKind::Extension, "urn:vendor", "digest:vendor", true).unwrap(),
        ])).unwrap();
        assert_eq!(result.status(), RoundTripStatus::SemanticallyEquivalent);
        assert_eq!(result.preserved_item_ids().len(), 3);
    }
    #[test]
    fn silent_loss_after_exact_mapping_is_rejected() {
        let ledger = ledger(vec![
            exact("object:board", "target:board"),
            exact("relationship:feeder", "target:feeder"),
            exact("extension:vendor", "target:extension"),
            MappingEntry::new("object:out-of-scope", TransformationOutcome::NotInScope, Vec::new(), None, Vec::new(), Some("outside".into())).unwrap(),
        ]).unwrap();
        let result = RoundTripResult::compare(&ledger, &roundtrip(vec![
            SemanticItem::new("object:board", SemanticItemKind::Object, "urn:site", "digest:board", true).unwrap(),
            SemanticItem::new("relationship:feeder", SemanticItemKind::Relationship, "urn:site", "digest:feeder", true).unwrap(),
        ]));
        assert!(matches!(result, Err(InteropError::UndeclaredSemanticLoss { .. })));
    }
    #[test]
    fn explicit_loss_is_reported_not_hidden() {
        let loss = InformationLoss::new("loss:extension", InformationLossCategory::Extension, "Vendor extension not representable in target profile").unwrap();
        let ledger = ledger(vec![
            exact("object:board", "target:board"),
            exact("relationship:feeder", "target:feeder"),
            MappingEntry::new("extension:vendor", TransformationOutcome::ExplicitlyLossy, vec!["target:extension".into()], Some("rule:extension".into()), vec![loss], None).unwrap(),
            MappingEntry::new("object:out-of-scope", TransformationOutcome::NotInScope, Vec::new(), None, Vec::new(), Some("outside".into())).unwrap(),
        ]).unwrap();
        let result = RoundTripResult::compare(&ledger, &roundtrip(vec![
            SemanticItem::new("object:board", SemanticItemKind::Object, "urn:site", "digest:board", true).unwrap(),
            SemanticItem::new("relationship:feeder", SemanticItemKind::Relationship, "urn:site", "digest:feeder", true).unwrap(),
        ])).unwrap();
        assert_eq!(result.status(), RoundTripStatus::EquivalentWithExplicitLosses);
        assert_eq!(result.differences()[0].outcome, TransformationOutcome::ExplicitlyLossy);
    }
    #[test]
    fn extra_round_trip_items_are_reported_separately() {
        let ledger = ledger(vec![
            exact("object:board", "target:board"),
            exact("relationship:feeder", "target:feeder"),
            exact("extension:vendor", "target:extension"),
            MappingEntry::new("object:out-of-scope", TransformationOutcome::NotInScope, Vec::new(), None, Vec::new(), Some("outside".into())).unwrap(),
        ]).unwrap();
        let result = RoundTripResult::compare(&ledger, &roundtrip(vec![
            SemanticItem::new("object:board", SemanticItemKind::Object, "urn:site", "digest:board", true).unwrap(),
            SemanticItem::new("relationship:feeder", SemanticItemKind::Relationship, "urn:site", "digest:feeder", true).unwrap(),
            SemanticItem::new("extension:vendor", SemanticItemKind::Extension, "urn:vendor", "digest:vendor", true).unwrap(),
            SemanticItem::new("target:derived-extra", SemanticItemKind::Other, "urn:adapter", "digest:extra", true).unwrap(),
        ])).unwrap();
        assert_eq!(result.extra_round_trip_item_ids(), &["target:derived-extra".to_owned()]);
    }
    #[test]
    fn unresolved_mapping_keeps_round_trip_incomplete() {
        let ledger = ledger(vec![
            exact("object:board", "target:board"),
            exact("relationship:feeder", "target:feeder"),
            MappingEntry::new("extension:vendor", TransformationOutcome::Unsupported, Vec::new(), None, Vec::new(), Some("unknown vendor extension".into())).unwrap(),
            MappingEntry::new("object:out-of-scope", TransformationOutcome::NotInScope, Vec::new(), None, Vec::new(), Some("outside".into())).unwrap(),
        ]).unwrap();
        let result = RoundTripResult::compare(&ledger, &roundtrip(vec![
            SemanticItem::new("object:board", SemanticItemKind::Object, "urn:site", "digest:board", true).unwrap(),
            SemanticItem::new("relationship:feeder", SemanticItemKind::Relationship, "urn:site", "digest:feeder", true).unwrap(),
        ])).unwrap();
        assert_eq!(result.status(), RoundTripStatus::Incomplete);
    }
}
