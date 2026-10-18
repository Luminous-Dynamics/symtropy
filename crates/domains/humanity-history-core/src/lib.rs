// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Renderer-independent records and validation for evidence-aware historical reconstructions.
//!
//! This crate deliberately does not depend on Bevy, a database, a network client, or an AI
//! runtime. It validates the integrity of a small historical evidence package before a renderer
//! or research assistant consumes it. Validation is not a verdict that a historical claim is
//! true: source quality and scholarly interpretation remain reviewable research questions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Schema version for the first interoperable record format.
pub const SCHEMA_VERSION: &str = "0.1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dataset {
    pub schema_version: String,
    #[serde(default)]
    pub sources: Vec<SourceRecord>,
    #[serde(default)]
    pub claims: Vec<HistoricalClaim>,
    #[serde(default)]
    pub relations: Vec<KnowledgeRelation>,
    #[serde(default)]
    pub assets: Vec<AssetRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRecord {
    /// Stable identifier in this dataset; preserve source-native IDs where possible.
    pub id: String,
    pub title: String,
    /// Stable URI, catalogue identifier, or archival reference. This crate does not fetch it.
    pub source_uri: String,
    pub rights: RightsDeclaration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightsDeclaration {
    pub status: RightsStatus,
    /// Required when status is `open_license`; preserve the exact license identifier.
    #[serde(default)]
    pub license_id: Option<String>,
    /// Human-readable attribution to display when required by a license or community.
    #[serde(default)]
    pub attribution: Option<String>,
    /// Record the basis for a permission or other non-obvious rights decision.
    #[serde(default)]
    pub basis_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RightsStatus {
    PublicDomain,
    OpenLicense,
    PermissionGranted,
    Restricted,
    Unknown,
    CommunityProtocol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalClaim {
    pub id: String,
    pub proposition: String,
    pub mode: ClaimMode,
    #[serde(default)]
    pub valid_time: Option<TimeInterval>,
    #[serde(default)]
    pub evidence: Vec<EvidenceLink>,
    /// Required for hypotheses and counterfactuals; states why the interpretation is proposed.
    #[serde(default)]
    pub interpretation_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimMode {
    Attested,
    EvidenceBasedReconstruction,
    Hypothesis,
    Counterfactual,
}

impl ClaimMode {
    fn requires_evidence(self) -> bool {
        matches!(self, Self::Attested | Self::EvidenceBasedReconstruction)
    }

    fn requires_interpretation_note(self) -> bool {
        matches!(self, Self::Hypothesis | Self::Counterfactual)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeInterval {
    /// Calendar identifier or documented calendar convention. No implicit conversion is performed.
    pub calendar: String,
    /// Ordinal year value within the named calendar; not necessarily Gregorian.
    pub start_year: i32,
    pub end_year: i32,
    pub precision: DatePrecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatePrecision {
    ExactYear,
    ApproximateYear,
    Decade,
    Century,
    BroadRange,
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceLink {
    pub source_id: String,
    /// Page, paragraph, object number, figure, catalogue field, timestamp, or other source anchor.
    pub locator: String,
    pub relation: EvidenceRelation,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRelation {
    Supports,
    Challenges,
    Qualifies,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeRelation {
    pub id: String,
    pub from_claim_id: String,
    pub to_claim_id: String,
    pub kind: RelationKind,
    pub status: RelationStatus,
    #[serde(default)]
    pub evidence: Vec<EvidenceLink>,
    /// Hypothesized or counterfactual edges must explain their interpretive basis.
    #[serde(default)]
    pub interpretation_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    Precedes,
    Influenced,
    IndependentlyDeveloped,
    TransmittedVia,
    DerivedFrom,
    Enabled,
    Supports,
    Challenges,
    Qualifies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationStatus {
    Documented,
    Hypothesized,
    Counterfactual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id: String,
    /// Source or collection record establishing the asset's origin/provenance.
    pub source_id: String,
    pub rights: RightsDeclaration,
    /// Assets intended for distribution are subject to fail-closed rights validation.
    pub distributable: bool,
    /// SHA-256 of the exact asset bytes, represented by 64 hexadecimal characters.
    pub content_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationCode {
    EmptySchemaVersion,
    EmptyId,
    DuplicateId,
    EmptyField,
    InvalidTimeInterval,
    MissingEvidenceForClaim,
    MissingSourceReference,
    MissingEvidenceLocator,
    MissingClaimReference,
    MissingEvidenceForRelation,
    MissingInterpretationNote,
    InvalidRightsDeclaration,
    UndistributableRights,
    InvalidSha256,
    SelfRelation,
    TemporalContradiction,
    UnsupportedSchemaVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub record_id: String,
    pub code: ValidationCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.issues.len()
    }
}

fn issue(
    issues: &mut Vec<ValidationIssue>,
    record_id: impl Into<String>,
    code: ValidationCode,
    message: impl Into<String>,
) {
    issues.push(ValidationIssue {
        record_id: record_id.into(),
        code,
        message: message.into(),
    });
}

fn register_id(
    id: &str,
    seen: &mut BTreeSet<String>,
    record_id: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    if id.trim().is_empty() {
        issue(
            issues,
            if record_id.is_empty() { "<empty>" } else { record_id },
            ValidationCode::EmptyId,
            "record ID must not be empty",
        );
    } else if !seen.insert(id.to_owned()) {
        issue(
            issues,
            id,
            ValidationCode::DuplicateId,
            "record ID is already used by another record",
        );
    }
}

fn require_nonempty(
    value: &str,
    record_id: &str,
    field: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    if value.trim().is_empty() {
        issue(
            issues,
            record_id,
            ValidationCode::EmptyField,
            format!("{field} must not be empty"),
        );
    }
}

fn validate_rights(
    rights: &RightsDeclaration,
    record_id: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    match rights.status {
        RightsStatus::OpenLicense
            if rights.license_id.as_deref().unwrap_or("").trim().is_empty() =>
        {
            issue(
                issues,
                record_id,
                ValidationCode::InvalidRightsDeclaration,
                "open-license status requires an exact license identifier",
            );
        }
        RightsStatus::PermissionGranted
            if rights.basis_note.as_deref().unwrap_or("").trim().is_empty() =>
        {
            issue(
                issues,
                record_id,
                ValidationCode::InvalidRightsDeclaration,
                "permission-granted status requires a recorded permission basis",
            );
        }
        RightsStatus::PublicDomain
            if rights.basis_note.as_deref().unwrap_or("").trim().is_empty() =>
        {
            issue(
                issues,
                record_id,
                ValidationCode::InvalidRightsDeclaration,
                "public-domain status requires a documented determination basis",
            );
        }
        _ => {}
    }
}

fn validate_evidence_links(
    owner_id: &str,
    evidence: &[EvidenceLink],
    source_ids: &BTreeSet<String>,
    issues: &mut Vec<ValidationIssue>,
) {
    for link in evidence {
        if !source_ids.contains(&link.source_id) {
            issue(
                issues,
                owner_id,
                ValidationCode::MissingSourceReference,
                format!("evidence refers to unknown source ID '{}'", link.source_id),
            );
        }
        require_nonempty(&link.locator, owner_id, "evidence locator", issues);
    }
}

/// Validate IDs, references, claim modes, chronology intervals, rights, and provenance invariants.
///
/// This is a structural validator, not an automated historian: it cannot determine whether a
/// source is credible, whether an interpretation is correct, or whether cited evidence actually
/// entails its claim. Those require domain review and separate benchmark protocols.
pub fn validate(dataset: &Dataset) -> ValidationReport {
    let mut issues = Vec::new();
    let mut all_ids = BTreeSet::new();

    if dataset.schema_version.trim().is_empty() {
        issue(
            &mut issues,
            "dataset",
            ValidationCode::EmptySchemaVersion,
            "dataset schema version is required",
        );
    } else if dataset.schema_version != SCHEMA_VERSION {
        issue(
            &mut issues,
            "dataset",
            ValidationCode::UnsupportedSchemaVersion,
            format!(
                "schema version '{}' is not supported; expected '{}'",
                dataset.schema_version, SCHEMA_VERSION
            ),
        );
    }

    for source in &dataset.sources {
        register_id(&source.id, &mut all_ids, &source.id, &mut issues);
        require_nonempty(&source.title, &source.id, "source title", &mut issues);
        require_nonempty(&source.source_uri, &source.id, "source URI/identifier", &mut issues);
        validate_rights(&source.rights, &source.id, &mut issues);
    }

    let source_ids: BTreeSet<String> = dataset.sources.iter().map(|source| source.id.clone()).collect();
    let claim_ids: BTreeSet<String> = dataset.claims.iter().map(|claim| claim.id.clone()).collect();

    for claim in &dataset.claims {
        register_id(&claim.id, &mut all_ids, &claim.id, &mut issues);
        require_nonempty(&claim.proposition, &claim.id, "claim proposition", &mut issues);

        if claim.mode.requires_evidence()
            && !claim
                .evidence
                .iter()
                .any(|link| link.relation == EvidenceRelation::Supports)
        {
            issue(
                &mut issues,
                &claim.id,
                ValidationCode::MissingEvidenceForClaim,
                "attested claims and evidence-based reconstructions require at least one evidence link marked supports; challenging-only evidence is not positive support",
            );
        }
        if claim.mode.requires_interpretation_note()
            && claim.interpretation_note.as_deref().unwrap_or("").trim().is_empty()
        {
            issue(
                &mut issues,
                &claim.id,
                ValidationCode::MissingInterpretationNote,
                "hypotheses and counterfactuals require an interpretation note",
            );
        }

        if let Some(time) = &claim.valid_time {
            require_nonempty(&time.calendar, &claim.id, "time calendar", &mut issues);
            if time.start_year > time.end_year {
                issue(
                    &mut issues,
                    &claim.id,
                    ValidationCode::InvalidTimeInterval,
                    "time interval start_year must not exceed end_year in the declared calendar",
                );
            } else if time.precision == DatePrecision::ExactYear
                && time.start_year != time.end_year
            {
                issue(
                    &mut issues,
                    &claim.id,
                    ValidationCode::InvalidTimeInterval,
                    "exact_year precision requires identical start_year and end_year",
                );
            }
        }
        validate_evidence_links(&claim.id, &claim.evidence, &source_ids, &mut issues);
    }

    for relation in &dataset.relations {
        register_id(&relation.id, &mut all_ids, &relation.id, &mut issues);
        if !claim_ids.contains(&relation.from_claim_id) {
            issue(
                &mut issues,
                &relation.id,
                ValidationCode::MissingClaimReference,
                format!("relation refers to unknown from_claim_id '{}'", relation.from_claim_id),
            );
        }
        if !claim_ids.contains(&relation.to_claim_id) {
            issue(
                &mut issues,
                &relation.id,
                ValidationCode::MissingClaimReference,
                format!("relation refers to unknown to_claim_id '{}'", relation.to_claim_id),
            );
        }
        if !relation.from_claim_id.is_empty() && relation.from_claim_id == relation.to_claim_id {
            issue(
                &mut issues,
                &relation.id,
                ValidationCode::SelfRelation,
                "a relation must not connect a claim to itself",
            );
        }

        // Detect only contradictions that are decidable from intervals in the same calendar.
        // Overlapping/uncertain intervals are not treated as proof that an edge is false.
        let time_directed = matches!(
            relation.kind,
            RelationKind::Precedes
                | RelationKind::Influenced
                | RelationKind::TransmittedVia
                | RelationKind::DerivedFrom
                | RelationKind::Enabled
        );
        if time_directed {
            let from_time = dataset
                .claims
                .iter()
                .find(|claim| claim.id == relation.from_claim_id)
                .and_then(|claim| claim.valid_time.as_ref());
            let to_time = dataset
                .claims
                .iter()
                .find(|claim| claim.id == relation.to_claim_id)
                .and_then(|claim| claim.valid_time.as_ref());
            if let (Some(from), Some(to)) = (from_time, to_time) {
                if from.calendar == to.calendar && from.start_year > to.end_year {
                    issue(
                        &mut issues,
                        &relation.id,
                        ValidationCode::TemporalContradiction,
                        "directed temporal/influence relation conflicts with the non-overlapping intervals in the same calendar",
                    );
                }
            }
        }

        if relation.status == RelationStatus::Documented
            && !relation
                .evidence
                .iter()
                .any(|link| link.relation == EvidenceRelation::Supports)
        {
            issue(
                &mut issues,
                &relation.id,
                ValidationCode::MissingEvidenceForRelation,
                "documented relationships require at least one evidence link marked supports; challenging-only evidence cannot document the edge",
            );
        }
        if matches!(
            relation.status,
            RelationStatus::Hypothesized | RelationStatus::Counterfactual
        ) && relation.interpretation_note.as_deref().unwrap_or("").trim().is_empty()
        {
            issue(
                &mut issues,
                &relation.id,
                ValidationCode::MissingInterpretationNote,
                "hypothesized and counterfactual relationships require an interpretation note",
            );
        }
        validate_evidence_links(&relation.id, &relation.evidence, &source_ids, &mut issues);
    }

    for asset in &dataset.assets {
        register_id(&asset.id, &mut all_ids, &asset.id, &mut issues);
        if !source_ids.contains(&asset.source_id) {
            issue(
                &mut issues,
                &asset.id,
                ValidationCode::MissingSourceReference,
                format!("asset refers to unknown provenance source ID '{}'", asset.source_id),
            );
        }
        validate_rights(&asset.rights, &asset.id, &mut issues);
        if asset.content_sha256.len() != 64
            || !asset.content_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            issue(
                &mut issues,
                &asset.id,
                ValidationCode::InvalidSha256,
                "asset content_sha256 must contain exactly 64 hexadecimal characters",
            );
        }
        if asset.distributable
            && matches!(
                asset.rights.status,
                RightsStatus::Restricted | RightsStatus::Unknown | RightsStatus::CommunityProtocol
            )
        {
            issue(
                &mut issues,
                &asset.id,
                ValidationCode::UndistributableRights,
                "asset marked distributable has restricted, unknown, or community-protocol rights",
            );
        }
    }

    // Report order is stable regardless of source/vector insertion order.
    issues.sort();
    ValidationReport { issues }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rights_open() -> RightsDeclaration {
        RightsDeclaration {
            status: RightsStatus::OpenLicense,
            license_id: Some("CC0-1.0".to_owned()),
            attribution: None,
            basis_note: None,
        }
    }

    fn evidence(source_id: &str) -> EvidenceLink {
        EvidenceLink {
            source_id: source_id.to_owned(),
            locator: "fixture:paragraph-1".to_owned(),
            relation: EvidenceRelation::Supports,
            note: None,
        }
    }

    fn fixture() -> Dataset {
        Dataset {
            schema_version: SCHEMA_VERSION.to_owned(),
            sources: vec![SourceRecord {
                id: "fixture:source-1".to_owned(),
                title: "Synthetic test source (not a historical assertion)".to_owned(),
                source_uri: "urn:example:synthetic-source-1".to_owned(),
                rights: rights_open(),
            }],
            claims: vec![
                HistoricalClaim {
                    id: "fixture:claim-a".to_owned(),
                    proposition: "Synthetic fixture claim A".to_owned(),
                    mode: ClaimMode::Attested,
                    valid_time: Some(TimeInterval {
                        calendar: "fixture-year-scale".to_owned(),
                        start_year: 100,
                        end_year: 100,
                        precision: DatePrecision::ExactYear,
                    }),
                    evidence: vec![evidence("fixture:source-1")],
                    interpretation_note: None,
                },
                HistoricalClaim {
                    id: "fixture:claim-b".to_owned(),
                    proposition: "Synthetic fixture claim B".to_owned(),
                    mode: ClaimMode::Attested,
                    valid_time: None,
                    evidence: vec![evidence("fixture:source-1")],
                    interpretation_note: None,
                },
            ],
            relations: vec![KnowledgeRelation {
                id: "fixture:relation-1".to_owned(),
                from_claim_id: "fixture:claim-a".to_owned(),
                to_claim_id: "fixture:claim-b".to_owned(),
                kind: RelationKind::Supports,
                status: RelationStatus::Documented,
                evidence: vec![evidence("fixture:source-1")],
                interpretation_note: None,
            }],
            assets: vec![AssetRecord {
                id: "fixture:asset-1".to_owned(),
                source_id: "fixture:source-1".to_owned(),
                rights: RightsDeclaration {
                    status: RightsStatus::PublicDomain,
                    license_id: None,
                    attribution: None,
                    basis_note: Some("Synthetic fixture only".to_owned()),
                },
                distributable: true,
                content_sha256: "0".repeat(64),
            }],
        }
    }

    #[test]
    fn accepts_well_formed_synthetic_fixture() {
        let report = validate(&fixture());
        assert!(report.is_valid(), "{:#?}", report.issues);
    }

    #[test]
    fn attested_claim_without_evidence_fails_closed() {
        let mut data = fixture();
        data.claims[0].evidence.clear();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:claim-a"
                && item.code == ValidationCode::MissingEvidenceForClaim
        }));
    }

    #[test]
    fn challenging_only_evidence_does_not_support_an_attested_claim() {
        let mut data = fixture();
        data.claims[0].evidence[0].relation = EvidenceRelation::Challenges;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:claim-a"
                && item.code == ValidationCode::MissingEvidenceForClaim
        }));
    }

    #[test]
    fn challenging_evidence_is_preserved_alongside_support() {
        let mut data = fixture();
        let mut challenge = evidence("fixture:source-1");
        challenge.relation = EvidenceRelation::Challenges;
        challenge.locator = "fixture:paragraph-2".to_owned();
        data.claims[0].evidence.push(challenge);
        let report = validate(&data);
        assert!(report.is_valid(), "{:#?}", report.issues);
        assert_eq!(data.claims[0].evidence.len(), 2);
        assert!(data.claims[0]
            .evidence
            .iter()
            .any(|link| link.relation == EvidenceRelation::Challenges));
    }

    #[test]
    fn documented_relation_with_challenging_only_evidence_fails() {
        let mut data = fixture();
        data.relations[0].evidence[0].relation = EvidenceRelation::Challenges;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:relation-1"
                && item.code == ValidationCode::MissingEvidenceForRelation
        }));
    }

    #[test]
    fn dangling_source_reference_is_rejected() {
        let mut data = fixture();
        data.claims[0].evidence[0].source_id = "missing:source".to_owned();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.code == ValidationCode::MissingSourceReference
                && item.message.contains("missing:source")
        }));
    }

    #[test]
    fn reversed_time_interval_is_rejected() {
        let mut data = fixture();
        data.claims[0].valid_time.as_mut().unwrap().start_year = 500;
        data.claims[0].valid_time.as_mut().unwrap().end_year = 400;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:claim-a"
                && item.code == ValidationCode::InvalidTimeInterval
        }));
    }

    #[test]
    fn unknown_rights_cannot_be_distributed() {
        let mut data = fixture();
        data.assets[0].rights.status = RightsStatus::Unknown;
        data.assets[0].rights.license_id = None;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:asset-1"
                && item.code == ValidationCode::UndistributableRights
        }));
    }

    #[test]
    fn public_domain_asset_requires_recorded_basis() {
        let mut data = fixture();
        data.assets[0].rights.basis_note = None;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:asset-1"
                && item.code == ValidationCode::InvalidRightsDeclaration
        }));
    }

    #[test]
    fn open_license_requires_exact_identifier() {
        let mut data = fixture();
        data.assets[0].rights.license_id = None;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:asset-1"
                && item.code == ValidationCode::InvalidRightsDeclaration
        }));
    }

    #[test]
    fn documented_relation_requires_evidence() {
        let mut data = fixture();
        data.relations[0].evidence.clear();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:relation-1"
                && item.code == ValidationCode::MissingEvidenceForRelation
        }));
    }

    #[test]
    fn hypothesis_must_be_labelled_and_explained() {
        let mut data = fixture();
        data.claims[0].mode = ClaimMode::Hypothesis;
        data.claims[0].evidence.clear();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:claim-a"
                && item.code == ValidationCode::MissingInterpretationNote
        }));
        assert!(!report.issues.iter().any(|item| {
            item.record_id == "fixture:claim-a"
                && item.code == ValidationCode::MissingEvidenceForClaim
        }));
    }

    #[test]
    fn duplicate_ids_are_rejected_across_record_kinds() {
        let mut data = fixture();
        data.assets[0].id = data.claims[0].id.clone();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.code == ValidationCode::DuplicateId
                && item.record_id == "fixture:claim-a"
        }));
    }

    #[test]
    fn hypothesis_relation_requires_explanation() {
        let mut data = fixture();
        data.relations[0].status = RelationStatus::Hypothesized;
        data.relations[0].evidence.clear();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:relation-1"
                && item.code == ValidationCode::MissingInterpretationNote
        }));
    }

    #[test]
    fn backwards_temporal_edge_is_rejected_when_intervals_decide_it() {
        let mut data = fixture();
        data.claims[0].valid_time.as_mut().unwrap().start_year = 500;
        data.claims[0].valid_time.as_mut().unwrap().end_year = 500;
        data.claims[1].valid_time = Some(TimeInterval {
            calendar: "fixture-year-scale".to_owned(),
            start_year: 300,
            end_year: 350,
            precision: DatePrecision::BroadRange,
        });
        data.relations[0].kind = RelationKind::Influenced;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:relation-1"
                && item.code == ValidationCode::TemporalContradiction
        }));
    }

    #[test]
    fn overlapping_intervals_are_not_assumed_to_disprove_influence() {
        let mut data = fixture();
        data.claims[0].valid_time.as_mut().unwrap().start_year = 100;
        data.claims[0].valid_time.as_mut().unwrap().end_year = 120;
        data.claims[1].valid_time = Some(TimeInterval {
            calendar: "fixture-year-scale".to_owned(),
            start_year: 110,
            end_year: 140,
            precision: DatePrecision::BroadRange,
        });
        data.relations[0].kind = RelationKind::Influenced;
        assert!(
            validate(&data).issues.iter().all(|item| {
                item.code != ValidationCode::TemporalContradiction
            })
        );
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let mut data = fixture();
        data.schema_version = "99.0".to_owned();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "dataset"
                && item.code == ValidationCode::UnsupportedSchemaVersion
        }));
    }

    #[test]
    fn report_order_is_stable() {
        let mut data_a = fixture();
        data_a.claims[0].evidence.clear();

        let mut data_b = data_a.clone();
        data_b.claims.reverse();

        assert_eq!(validate(&data_a).issues, validate(&data_b).issues);
    }

    #[test]
    fn json_round_trip_preserves_records_and_uncertainty() {
        let data = fixture();
        let encoded = serde_json::to_string(&data).expect("serialize fixture");
        let decoded: Dataset = serde_json::from_str(&encoded).expect("deserialize fixture");
        assert_eq!(decoded, data);
        assert_eq!(
            decoded.claims[0].valid_time.as_ref().unwrap().start_year,
            100
        );
    }
}
