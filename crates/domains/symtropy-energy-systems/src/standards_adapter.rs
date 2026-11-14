// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Standards adapter mapping ledger and semantic round-trip qualification.
//! This is a conformance contract, not a CIM/SCL parser or standards certification.

use crate::IrSchemaVersion;
use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StandardPublicationStatus {
    Published,
    WorkingDraft,
    TechnicalReport,
    NationalAdoption,
    Withdrawn,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StandardReference {
    standard_id: String,
    part: Option<String>,
    edition: String,
    amendments: Vec<String>,
    publication_status: StandardPublicationStatus,
    jurisdiction: Option<String>,
}

impl StandardReference {
    pub fn new(
        standard_id: impl Into<String>,
        part: Option<String>,
        edition: impl Into<String>,
        mut amendments: Vec<String>,
        publication_status: StandardPublicationStatus,
        jurisdiction: Option<String>,
    ) -> Result<Self, AdapterError> {
        let standard_id = standard_id.into();
        let edition = edition.into();
        require_non_empty("standard_id", &standard_id)?;
        require_non_empty("standard_edition", &edition)?;
        if let Some(value) = &part { require_non_empty("standard_part", value)?; }
        if let Some(value) = &jurisdiction { require_non_empty("jurisdiction", value)?; }
        for amendment in &amendments { require_non_empty("standard_amendment", amendment)?; }
        amendments.sort();
        if amendments.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(AdapterError::DuplicateStandardAmendment);
        }
        Ok(Self { standard_id, part, edition, amendments, publication_status, jurisdiction })
    }

    pub fn standard_id(&self) -> &str { &self.standard_id }
    pub fn part(&self) -> Option<&str> { self.part.as_deref() }
    pub fn edition(&self) -> &str { &self.edition }
    pub fn amendments(&self) -> &[String] { &self.amendments }
    pub const fn publication_status(&self) -> StandardPublicationStatus { self.publication_status }
    pub fn jurisdiction(&self) -> Option<&str> { self.jurisdiction.as_deref() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingProfile {
    profile_id: String,
    profile_version: String,
    standard: StandardReference,
    adapter_version: String,
    target_schema: IrSchemaVersion,
    scope_statement: String,
}

impl MappingProfile {
    pub fn new(
        profile_id: impl Into<String>,
        profile_version: impl Into<String>,
        standard: StandardReference,
        adapter_version: impl Into<String>,
        target_schema: IrSchemaVersion,
        scope_statement: impl Into<String>,
    ) -> Result<Self, AdapterError> {
        let profile_id = profile_id.into();
        let profile_version = profile_version.into();
        let adapter_version = adapter_version.into();
        let scope_statement = scope_statement.into();
        require_non_empty("profile_id", &profile_id)?;
        require_non_empty("profile_version", &profile_version)?;
        require_non_empty("adapter_version", &adapter_version)?;
        require_non_empty("scope_statement", &scope_statement)?;
        Ok(Self { profile_id, profile_version, standard, adapter_version, target_schema, scope_statement })
    }
    pub fn profile_id(&self) -> &str { &self.profile_id }
    pub fn profile_version(&self) -> &str { &self.profile_version }
    pub fn standard(&self) -> &StandardReference { &self.standard }
    pub fn adapter_version(&self) -> &str { &self.adapter_version }
    pub const fn target_schema(&self) -> IrSchemaVersion { self.target_schema }
    pub fn scope_statement(&self) -> &str { &self.scope_statement }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SourceSemanticRef {
    namespace: String,
    source_id: String,
    semantic_path: String,
}

impl SourceSemanticRef {
    pub fn new(namespace: impl Into<String>, source_id: impl Into<String>, semantic_path: impl Into<String>) -> Result<Self, AdapterError> {
        let namespace = namespace.into();
        let source_id = source_id.into();
        let semantic_path = semantic_path.into();
        require_non_empty("source_namespace", &namespace)?;
        require_non_empty("source_id", &source_id)?;
        require_non_empty("source_semantic_path", &semantic_path)?;
        Ok(Self { namespace, source_id, semantic_path })
    }
    pub fn namespace(&self) -> &str { &self.namespace }
    pub fn source_id(&self) -> &str { &self.source_id }
    pub fn semantic_path(&self) -> &str { &self.semantic_path }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct TargetSemanticRef {
    namespace: String,
    target_id: String,
    semantic_path: String,
}

impl TargetSemanticRef {
    pub fn new(namespace: impl Into<String>, target_id: impl Into<String>, semantic_path: impl Into<String>) -> Result<Self, AdapterError> {
        let namespace = namespace.into();
        let target_id = target_id.into();
        let semantic_path = semantic_path.into();
        require_non_empty("target_namespace", &namespace)?;
        require_non_empty("target_id", &target_id)?;
        require_non_empty("target_semantic_path", &semantic_path)?;
        Ok(Self { namespace, target_id, semantic_path })
    }
    pub fn namespace(&self) -> &str { &self.namespace }
    pub fn target_id(&self) -> &str { &self.target_id }
    pub fn semantic_path(&self) -> &str { &self.semantic_path }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingOutcome {
    ExactPreservation,
    SemanticEquivalent,
    LossExplicit,
    Unsupported,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LossEntry {
    source_path: String,
    explanation: String,
}

impl LossEntry {
    pub fn new(source_path: impl Into<String>, explanation: impl Into<String>) -> Result<Self, AdapterError> {
        let source_path = source_path.into();
        let explanation = explanation.into();
        require_non_empty("loss_source_path", &source_path)?;
        require_non_empty("loss_explanation", &explanation)?;
        Ok(Self { source_path, explanation })
    }
    pub fn source_path(&self) -> &str { &self.source_path }
    pub fn explanation(&self) -> &str { &self.explanation }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappingRecord {
    source: SourceSemanticRef,
    target: Option<TargetSemanticRef>,
    outcome: MappingOutcome,
    explanation: String,
    losses: Vec<LossEntry>,
}

impl MappingRecord {
    pub fn new(
        source: SourceSemanticRef,
        target: Option<TargetSemanticRef>,
        outcome: MappingOutcome,
        explanation: impl Into<String>,
        mut losses: Vec<LossEntry>,
    ) -> Result<Self, AdapterError> {
        let explanation = explanation.into();
        require_non_empty("mapping_explanation", &explanation)?;
        match outcome {
            MappingOutcome::ExactPreservation | MappingOutcome::SemanticEquivalent => {
                if target.is_none() || !losses.is_empty() {
                    return Err(AdapterError::MappingOutcomeShapeMismatch { outcome });
                }
            }
            MappingOutcome::LossExplicit => {
                if target.is_none() || losses.is_empty() {
                    return Err(AdapterError::MappingOutcomeShapeMismatch { outcome });
                }
            }
            MappingOutcome::Unsupported | MappingOutcome::Ambiguous => {
                if target.is_some() || !losses.is_empty() {
                    return Err(AdapterError::MappingOutcomeShapeMismatch { outcome });
                }
            }
        }
        losses.sort_by(|a, b| a.source_path.cmp(&b.source_path));
        if losses.windows(2).any(|pair| pair[0].source_path == pair[1].source_path) {
            return Err(AdapterError::DuplicateLossPath);
        }
        Ok(Self { source, target, outcome, explanation, losses })
    }
    pub fn source(&self) -> &SourceSemanticRef { &self.source }
    pub fn target(&self) -> Option<&TargetSemanticRef> { self.target.as_ref() }
    pub const fn outcome(&self) -> MappingOutcome { self.outcome }
    pub fn explanation(&self) -> &str { &self.explanation }
    pub fn losses(&self) -> &[LossEntry] { &self.losses }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionDisposition {
    PreservedOpaque,
    Mapped,
    UnsupportedExplicit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtensionMapping {
    source_namespace: String,
    source_extension_id: String,
    disposition: ExtensionDisposition,
    target_extension_id: Option<String>,
    source_payload_digest: String,
    target_payload_digest: Option<String>,
    explanation: String,
}

impl ExtensionMapping {
    pub fn new(
        source_namespace: impl Into<String>,
        source_extension_id: impl Into<String>,
        disposition: ExtensionDisposition,
        target_extension_id: Option<String>,
        source_payload_digest: impl Into<String>,
        target_payload_digest: Option<String>,
        explanation: impl Into<String>,
    ) -> Result<Self, AdapterError> {
        let source_namespace = source_namespace.into();
        let source_extension_id = source_extension_id.into();
        let source_payload_digest = source_payload_digest.into();
        let explanation = explanation.into();
        require_non_empty("source_extension_namespace", &source_namespace)?;
        require_non_empty("source_extension_id", &source_extension_id)?;
        require_non_empty("source_extension_payload_digest", &source_payload_digest)?;
        require_non_empty("extension_mapping_explanation", &explanation)?;
        if let Some(value) = &target_extension_id { require_non_empty("target_extension_id", value)?; }
        if let Some(value) = &target_payload_digest { require_non_empty("target_extension_payload_digest", value)?; }
        match disposition {
            ExtensionDisposition::PreservedOpaque => {
                if target_extension_id.is_none() || target_payload_digest.as_deref() != Some(source_payload_digest.as_str()) {
                    return Err(AdapterError::ExtensionDispositionShapeMismatch { disposition });
                }
            }
            ExtensionDisposition::Mapped => {
                if target_extension_id.is_none() || target_payload_digest.is_none() {
                    return Err(AdapterError::ExtensionDispositionShapeMismatch { disposition });
                }
            }
            ExtensionDisposition::UnsupportedExplicit => {
                if target_extension_id.is_some() || target_payload_digest.is_some() {
                    return Err(AdapterError::ExtensionDispositionShapeMismatch { disposition });
                }
            }
        }
        Ok(Self { source_namespace, source_extension_id, disposition, target_extension_id, source_payload_digest, target_payload_digest, explanation })
    }
    pub fn source_namespace(&self) -> &str { &self.source_namespace }
    pub fn source_extension_id(&self) -> &str { &self.source_extension_id }
    pub const fn disposition(&self) -> ExtensionDisposition { self.disposition }
    pub fn target_extension_id(&self) -> Option<&str> { self.target_extension_id.as_deref() }
    pub fn source_payload_digest(&self) -> &str { &self.source_payload_digest }
    pub fn target_payload_digest(&self) -> Option<&str> { self.target_payload_digest.as_deref() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterRunStatus {
    CompleteSemanticallyEquivalent,
    CompleteWithDeclaredLoss,
    IncompleteUnsupportedOrAmbiguous,
    IncompleteSourceInventory,
    IncompleteUnknownStandardStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdapterRun {
    profile: MappingProfile,
    source_input_digest: String,
    target_output_digest: String,
    source_inventory_complete: bool,
    source_semantic_item_count: usize,
    source_extension_count: usize,
    status: AdapterRunStatus,
    #[serde(serialize_with = "serialize_map_values")]
    records: BTreeMap<SourceSemanticRef, MappingRecord>,
    #[serde(serialize_with = "serialize_map_values")]
    extensions: BTreeMap<(String, String), ExtensionMapping>,
}

impl AdapterRun {
    pub fn new(
        profile: MappingProfile,
        source_input_digest: impl Into<String>,
        target_output_digest: impl Into<String>,
        source_inventory_complete: bool,
        source_semantic_item_count: usize,
        source_extension_count: usize,
        records: Vec<MappingRecord>,
        extensions: Vec<ExtensionMapping>,
    ) -> Result<Self, AdapterError> {
        let source_input_digest = source_input_digest.into();
        let target_output_digest = target_output_digest.into();
        require_non_empty("source_input_digest", &source_input_digest)?;
        require_non_empty("target_output_digest", &target_output_digest)?;
        if source_inventory_complete
            && (records.len() != source_semantic_item_count || extensions.len() != source_extension_count)
        {
            return Err(AdapterError::SourceInventoryCountMismatch {
                expected_semantic_items: source_semantic_item_count,
                actual_semantic_items: records.len(),
                expected_extensions: source_extension_count,
                actual_extensions: extensions.len(),
            });
        }
        let mut record_map = BTreeMap::new();
        for record in records {
            let source = record.source.clone();
            if record_map.insert(source.clone(), record).is_some() {
                return Err(AdapterError::DuplicateSourceSemanticReference { source });
            }
        }
        let mut extension_map = BTreeMap::new();
        for extension in extensions {
            let key = (extension.source_namespace.clone(), extension.source_extension_id.clone());
            if extension_map.insert(key.clone(), extension).is_some() {
                return Err(AdapterError::DuplicateSourceExtension { namespace: key.0, extension_id: key.1 });
            }
        }
        let status = if profile.standard.publication_status == StandardPublicationStatus::Unknown {
            AdapterRunStatus::IncompleteUnknownStandardStatus
        } else {
            derive_status(source_inventory_complete, record_map.values(), extension_map.values())
        };
        Ok(Self { profile, source_input_digest, target_output_digest, source_inventory_complete, source_semantic_item_count, source_extension_count, status, records: record_map, extensions: extension_map })
    }
    pub fn profile(&self) -> &MappingProfile { &self.profile }
    pub fn source_input_digest(&self) -> &str { &self.source_input_digest }
    pub fn target_output_digest(&self) -> &str { &self.target_output_digest }
    pub const fn source_inventory_complete(&self) -> bool { self.source_inventory_complete }
    pub const fn source_semantic_item_count(&self) -> usize { self.source_semantic_item_count }
    pub const fn source_extension_count(&self) -> usize { self.source_extension_count }
    pub const fn status(&self) -> AdapterRunStatus { self.status }
    pub fn records(&self) -> &BTreeMap<SourceSemanticRef, MappingRecord> { &self.records }
    pub fn extensions(&self) -> &BTreeMap<(String, String), ExtensionMapping> { &self.extensions }
}

fn derive_status<'a>(
    source_inventory_complete: bool,
    records: impl Iterator<Item = &'a MappingRecord>,
    extensions: impl Iterator<Item = &'a ExtensionMapping>,
) -> AdapterRunStatus {
    if !source_inventory_complete { return AdapterRunStatus::IncompleteSourceInventory; }
    let mut has_loss = false;
    let mut incomplete = false;
    for record in records {
        match record.outcome {
            MappingOutcome::ExactPreservation | MappingOutcome::SemanticEquivalent => {}
            MappingOutcome::LossExplicit => has_loss = true,
            MappingOutcome::Unsupported | MappingOutcome::Ambiguous => incomplete = true,
        }
    }
    for extension in extensions {
        match extension.disposition {
            ExtensionDisposition::PreservedOpaque | ExtensionDisposition::Mapped => {}
            ExtensionDisposition::UnsupportedExplicit => incomplete = true,
        }
    }
    if incomplete { AdapterRunStatus::IncompleteUnsupportedOrAmbiguous }
    else if has_loss { AdapterRunStatus::CompleteWithDeclaredLoss }
    else { AdapterRunStatus::CompleteSemanticallyEquivalent }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SemanticAtom {
    namespace: String,
    object_ref: String,
    semantic_path: String,
    lexical_value: String,
    unit: Option<String>,
}

impl SemanticAtom {
    pub fn new(
        namespace: impl Into<String>,
        object_ref: impl Into<String>,
        semantic_path: impl Into<String>,
        lexical_value: impl Into<String>,
        unit: Option<String>,
    ) -> Result<Self, AdapterError> {
        let namespace = namespace.into();
        let object_ref = object_ref.into();
        let semantic_path = semantic_path.into();
        let lexical_value = lexical_value.into();
        require_non_empty("semantic_atom_namespace", &namespace)?;
        require_non_empty("semantic_atom_object_ref", &object_ref)?;
        require_non_empty("semantic_atom_path", &semantic_path)?;
        if let Some(value) = &unit { require_non_empty("semantic_atom_unit", value)?; }
        Ok(Self { namespace, object_ref, semantic_path, lexical_value, unit })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SemanticRoundTripResult {
    profile_id: String,
    profile_version: String,
    source_atom_count: usize,
    returned_atom_count: usize,
    missing_atoms: Vec<SemanticAtom>,
    unexpected_atoms: Vec<SemanticAtom>,
    semantically_equivalent: bool,
}
impl SemanticRoundTripResult {
    pub fn profile_id(&self) -> &str { &self.profile_id }
    pub fn profile_version(&self) -> &str { &self.profile_version }
    pub const fn source_atom_count(&self) -> usize { self.source_atom_count }
    pub const fn returned_atom_count(&self) -> usize { self.returned_atom_count }
    pub fn missing_atoms(&self) -> &[SemanticAtom] { &self.missing_atoms }
    pub fn unexpected_atoms(&self) -> &[SemanticAtom] { &self.unexpected_atoms }
    pub const fn semantically_equivalent(&self) -> bool { self.semantically_equivalent }
}

pub fn compare_semantic_round_trip(
    profile: &MappingProfile,
    source_atoms: Vec<SemanticAtom>,
    returned_atoms: Vec<SemanticAtom>,
) -> Result<SemanticRoundTripResult, AdapterError> {
    let source = unique_atoms(source_atoms)?;
    let returned = unique_atoms(returned_atoms)?;
    let source_set: BTreeSet<SemanticAtom> = source.iter().cloned().collect();
    let returned_set: BTreeSet<SemanticAtom> = returned.iter().cloned().collect();
    let missing_atoms = source_set.difference(&returned_set).cloned().collect::<Vec<_>>();
    let unexpected_atoms = returned_set.difference(&source_set).cloned().collect::<Vec<_>>();
    let semantically_equivalent = missing_atoms.is_empty() && unexpected_atoms.is_empty();
    Ok(SemanticRoundTripResult {
        profile_id: profile.profile_id.clone(),
        profile_version: profile.profile_version.clone(),
        source_atom_count: source.len(),
        returned_atom_count: returned.len(),
        missing_atoms,
        unexpected_atoms,
        semantically_equivalent,
    })
}

fn unique_atoms(atoms: Vec<SemanticAtom>) -> Result<Vec<SemanticAtom>, AdapterError> {
    let mut unique = BTreeSet::new();
    for atom in atoms {
        if !unique.insert(atom.clone()) { return Err(AdapterError::DuplicateSemanticAtom { atom }); }
    }
    Ok(unique.into_iter().collect())
}

fn serialize_map_values<K, V, S>(values: &BTreeMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where K: Ord, V: Serialize, S: Serializer {
    values.values().collect::<Vec<_>>().serialize(serializer)
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), AdapterError> {
    if value.trim().is_empty() { return Err(AdapterError::EmptyField { field }); }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    EmptyField { field: &'static str },
    DuplicateStandardAmendment,
    MappingOutcomeShapeMismatch { outcome: MappingOutcome },
    DuplicateLossPath,
    ExtensionDispositionShapeMismatch { disposition: ExtensionDisposition },
    DuplicateSourceSemanticReference { source: SourceSemanticRef },
    DuplicateSourceExtension { namespace: String, extension_id: String },
    DuplicateSemanticAtom { atom: SemanticAtom },
    SourceInventoryCountMismatch { expected_semantic_items: usize, actual_semantic_items: usize, expected_extensions: usize, actual_extensions: usize },
}
impl core::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for AdapterError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(status: StandardPublicationStatus) -> MappingProfile {
        MappingProfile::new(
            "profile:cim-distribution", "0.1",
            StandardReference::new("IEC 61970", Some("301".into()), "2020",
                vec!["AMD1:2022".into()], status, None).unwrap(),
            "adapter:0.1.0", IrSchemaVersion::current(),
            "Synthetic mapping profile for test only",
        ).unwrap()
    }
    fn source(path: &str) -> SourceSemanticRef {
        SourceSemanticRef::new("urn:fixture:cim", "mRID:001", path).unwrap()
    }
    fn target(path: &str) -> TargetSemanticRef {
        TargetSemanticRef::new("urn:luminous:power-fabric", "asset:001", path).unwrap()
    }
    fn exact(path: &str) -> MappingRecord {
        MappingRecord::new(source(path), Some(target(path)), MappingOutcome::ExactPreservation,
            "One-to-one semantic mapping", Vec::new()).unwrap()
    }
    fn opaque_extension() -> ExtensionMapping {
        ExtensionMapping::new("urn:vendor:private", "vendor:extension:1",
            ExtensionDisposition::PreservedOpaque, Some("extension:1".into()),
            "sha256:payload", Some("sha256:payload".into()),
            "Opaque vendor payload retained byte-identically").unwrap()
    }

    #[test]
    fn standard_edition_and_publication_status_are_pinned() {
        let p = profile(StandardPublicationStatus::Published);
        assert_eq!(p.standard().standard_id(), "IEC 61970");
        assert_eq!(p.standard().part(), Some("301"));
        assert_eq!(p.standard().edition(), "2020");
        assert_eq!(p.standard().amendments(), &["AMD1:2022".to_owned()]);
        assert_eq!(p.standard().publication_status(), StandardPublicationStatus::Published);
    }
    #[test]
    fn working_draft_is_not_conflated_with_published_standard() {
        assert_ne!(profile(StandardPublicationStatus::Published).standard().publication_status(),
            profile(StandardPublicationStatus::WorkingDraft).standard().publication_status());
    }
    #[test]
    fn unknown_standard_publication_status_is_recorded_but_run_is_incomplete() {
        let profile = profile(StandardPublicationStatus::Unknown);
        let run = AdapterRun::new(profile, "source", "target", true, 0, 0, Vec::new(), Vec::new()).unwrap();
        assert_eq!(run.status(), AdapterRunStatus::IncompleteUnknownStandardStatus);
    }
    #[test]
    fn exact_mapping_requires_target_and_no_loss() {
        assert!(matches!(MappingRecord::new(source("voltage"), None, MappingOutcome::ExactPreservation,
            "no target", Vec::new()), Err(AdapterError::MappingOutcomeShapeMismatch { .. })));
        let loss = LossEntry::new("source.path", "precision was rounded").unwrap();
        assert!(matches!(MappingRecord::new(source("voltage"), Some(target("voltage")),
            MappingOutcome::ExactPreservation, "incorrectly declared exact", vec![loss]),
            Err(AdapterError::MappingOutcomeShapeMismatch { .. })));
    }
    #[test]
    fn explicit_loss_requires_target_and_machine_readable_loss_entry() {
        assert!(matches!(MappingRecord::new(source("phase"), Some(target("phase")),
            MappingOutcome::LossExplicit, "phase collapsed", Vec::new()),
            Err(AdapterError::MappingOutcomeShapeMismatch { .. })));
        let loss = LossEntry::new("ConductingEquipment.phases", "target profile does not encode phase order").unwrap();
        assert_eq!(MappingRecord::new(source("phase"), Some(target("phase")), MappingOutcome::LossExplicit,
            "phase collapsed", vec![loss]).unwrap().outcome(), MappingOutcome::LossExplicit);
    }
    #[test]
    fn unsupported_and_ambiguous_mappings_cannot_forge_a_target() {
        assert!(matches!(MappingRecord::new(source("unknown"), Some(target("unknown")),
            MappingOutcome::Unsupported, "not mapped", Vec::new()),
            Err(AdapterError::MappingOutcomeShapeMismatch { .. })));
        assert!(MappingRecord::new(source("ambiguous"), None, MappingOutcome::Ambiguous,
            "two possible target semantics", Vec::new()).is_ok());
    }
    #[test]
    fn run_derives_lossless_status_only_when_every_record_is_mapped() {
        let run = AdapterRun::new(profile(StandardPublicationStatus::Published), "sha256:source",
            "sha256:target", true, 1, 1, vec![exact("voltage")], vec![opaque_extension()]).unwrap();
        assert_eq!(run.status(), AdapterRunStatus::CompleteSemanticallyEquivalent);
    }
    #[test]
    fn explicit_loss_is_not_reported_as_lossless_success() {
        let loss = LossEntry::new("source.phase", "phase information was collapsed").unwrap();
        let record = MappingRecord::new(source("phase"), Some(target("phase")), MappingOutcome::LossExplicit,
            "declared phase loss", vec![loss]).unwrap();
        let run = AdapterRun::new(profile(StandardPublicationStatus::Published), "source", "target",
            true, 1, 0, vec![record], Vec::new()).unwrap();
        assert_eq!(run.status(), AdapterRunStatus::CompleteWithDeclaredLoss);
    }
    #[test]
    fn unsupported_semantics_make_run_incomplete() {
        let record = MappingRecord::new(source("vendor-protection"), None, MappingOutcome::Unsupported,
            "target profile has no reviewed equivalent", Vec::new()).unwrap();
        let run = AdapterRun::new(profile(StandardPublicationStatus::Published), "source", "target",
            true, 1, 0, vec![record], Vec::new()).unwrap();
        assert_eq!(run.status(), AdapterRunStatus::IncompleteUnsupportedOrAmbiguous);
    }
    #[test]
    fn incomplete_source_inventory_takes_precedence() {
        let run = AdapterRun::new(profile(StandardPublicationStatus::Published), "source", "target",
            false, 1, 0, vec![exact("voltage")], Vec::new()).unwrap();
        assert_eq!(run.status(), AdapterRunStatus::IncompleteSourceInventory);
    }
    #[test]
    fn opaque_extension_requires_identical_payload_digest() {
        assert!(matches!(ExtensionMapping::new("urn:vendor", "ext:1",
            ExtensionDisposition::PreservedOpaque, Some("target-ext:1".into()),
            "sha256:a", Some("sha256:b".into()), "not byte-identical"),
            Err(AdapterError::ExtensionDispositionShapeMismatch { .. })));
    }
    #[test]
    fn unsupported_extension_is_explicit_and_incomplete() {
        let extension = ExtensionMapping::new("urn:vendor", "ext:1",
            ExtensionDisposition::UnsupportedExplicit, None, "sha256:a", None,
            "unsupported by this target profile").unwrap();
        let run = AdapterRun::new(profile(StandardPublicationStatus::Published), "source", "target",
            true, 0, 1, Vec::new(), vec![extension]).unwrap();
        assert_eq!(run.status(), AdapterRunStatus::IncompleteUnsupportedOrAmbiguous);
    }
    #[test]
    fn semantic_round_trip_detects_missing_and_unexpected_atoms() {
        let p = profile(StandardPublicationStatus::Published);
        let voltage = SemanticAtom::new("urn:cim", "mRID:1", "BaseVoltage.nominalVoltage", "110000", Some("V".into())).unwrap();
        let name = SemanticAtom::new("urn:cim", "mRID:1", "IdentifiedObject.name", "Bus A", None).unwrap();
        let changed = SemanticAtom::new("urn:cim", "mRID:1", "BaseVoltage.nominalVoltage", "11000", Some("V".into())).unwrap();
        let result = compare_semantic_round_trip(&p, vec![voltage, name], vec![changed]).unwrap();
        assert!(!result.semantically_equivalent());
        assert_eq!(result.missing_atoms().len(), 2);
        assert_eq!(result.unexpected_atoms().len(), 1);
    }
    #[test]
    fn semantically_equal_values_compare_equal_independent_of_atom_order() {
        let p = profile(StandardPublicationStatus::Published);
        let a = SemanticAtom::new("urn:cim", "mrid:a", "name", "Bus", None).unwrap();
        let b = SemanticAtom::new("urn:cim", "mrid:b", "voltage", "110000", Some("V".into())).unwrap();
        let result = compare_semantic_round_trip(&p, vec![a.clone(), b.clone()], vec![b, a]).unwrap();
        assert!(result.semantically_equivalent());
    }

    #[test]
    fn complete_inventory_count_must_match_mapping_and_extension_records() {
        assert!(matches!(
            AdapterRun::new(
                profile(StandardPublicationStatus::Published),
                "source", "target", true, 2, 1,
                vec![exact("voltage")], vec![opaque_extension()]
            ),
            Err(AdapterError::SourceInventoryCountMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_source_references_are_rejected() {
        assert!(matches!(AdapterRun::new(profile(StandardPublicationStatus::Published), "source", "target",
            true, 2, 0, vec![exact("voltage"), exact("voltage")], Vec::new()),
            Err(AdapterError::DuplicateSourceSemanticReference { .. })));
    }
    #[test]
    fn duplicate_semantic_atoms_are_rejected_not_deduplicated() {
        let p = profile(StandardPublicationStatus::Published);
        let atom = SemanticAtom::new("urn:cim", "mrid:a", "name", "Bus", None).unwrap();
        assert!(matches!(compare_semantic_round_trip(&p, vec![atom.clone(), atom.clone()], vec![atom]),
            Err(AdapterError::DuplicateSemanticAtom { .. })));
    }
    #[test]
    fn duplicate_extension_source_identity_is_rejected() {
        assert!(matches!(AdapterRun::new(profile(StandardPublicationStatus::Published), "source", "target",
            true, 0, 2, Vec::new(), vec![opaque_extension(), opaque_extension()]),
            Err(AdapterError::DuplicateSourceExtension { .. })));
    }
    #[test]
    fn amendments_are_ordered_and_duplicates_rejected() {
        let standard = StandardReference::new("IEC", None, "2025",
            vec!["AMD2".into(), "AMD1".into()], StandardPublicationStatus::Published, None).unwrap();
        assert_eq!(standard.amendments(), &["AMD1".to_owned(), "AMD2".to_owned()]);
        assert!(matches!(StandardReference::new("IEC", None, "2025",
            vec!["AMD1".into(), "AMD1".into()], StandardPublicationStatus::Published, None),
            Err(AdapterError::DuplicateStandardAmendment)));
    }
}
