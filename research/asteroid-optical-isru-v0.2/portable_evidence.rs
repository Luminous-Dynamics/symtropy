// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Optical ISRU portable-evidence profile validator and minimal RO-Crate 1.3 JSON-LD exporter.
// Run tests with:
//   rustc --edition=2021 --test research/asteroid-optical-isru-v0.2/portable_evidence.rs -o /tmp/isru-portable-evidence-tests
//   /tmp/isru-portable-evidence-tests

#![forbid(unsafe_code)]

//! Profile-level validation and minimal JSON-LD export for portable optical-ISRU evidence.
//!
//! This file deliberately does not implement a general JSON-LD or RO-Crate parser.
//! It validates a typed domain graph, emits a small RO-Crate 1.3-shaped JSON-LD
//! metadata document, and leaves full conformance validation to independent
//! RO-Crate tooling. Hashes and receipt identifiers are references, not proof
//! that an instrument reading or certificate is authentic.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

pub const RO_CRATE_VERSION: &str = "https://w3id.org/ro/crate/1.3";
pub const RO_CRATE_CONTEXT: &str = "https://w3id.org/ro/crate/1.3/context";
pub const PROV_NAMESPACE: &str = "http://www.w3.org/ns/prov#";
pub const QUDT_NAMESPACE: &str = "http://qudt.org/schema/qudt/";
pub const LUMINOUS_PROFILE_URI: &str =
    "https://luminousdynamics.io/profiles/optical-isru-evidence/0.1";

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn absolute_uri(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://") || value.starts_with("urn:")
}

fn valid_sha256(value: &str) -> bool {
    let hex = value.strip_prefix("sha256:").unwrap_or(value);
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c <= '\u{001f}' => {
                use std::fmt::Write;
                let _ = write!(&mut out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConformanceLevel {
    P0PackageValid,
    P1ProvenanceComplete,
    P2MetrologyQualified,
    P3Replayable,
    P4CrossLabComparable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityKind {
    CrateDataset,
    Laboratory,
    Rig,
    Instrument,
    Detector,
    OpticalSource,
    OpticalPath,
    MaterialFamily,
    FeedstockBatch,
    Sample,
    CalibrationCertificate,
    ReferenceMaterial,
    Method,
    Protocol,
    Software,
    Firmware,
    RawDataFile,
    DerivedDataFile,
    ProductLot,
    AuthorityReceipt,
    OptimizerFantasy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActivityKind {
    SamplePreparation,
    Calibration,
    OpticalReferenceCheck,
    QcmMeasurement,
    OpticalWitnessMeasurement,
    MaterialExposure,
    Capture,
    Purification,
    IndependentAssay,
    Maintenance,
    Requalification,
    DataTransformation,
    Decision,
    CampaignCloseout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgentKind {
    Person,
    Organization,
    SoftwareAgent,
    InstrumentAgent,
    Authority,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeClass {
    Entity(EntityKind),
    Activity(ActivityKind),
    Agent(AgentKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactMetadata {
    /// Path relative to the crate root or an absolute resolvable URI.
    pub location: String,
    pub media_type: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownField {
    pub field: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceNode {
    pub id: String,
    pub name: String,
    pub class: NodeClass,
    pub artifact: Option<ArtifactMetadata>,
    /// Unknown historical values remain explicit instead of defaulted/inferred.
    pub unknown_fields: Vec<UnknownField>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProvRelationKind {
    Used,
    WasGeneratedBy,
    WasAssociatedWith,
    WasAttributedTo,
    WasDerivedFrom,
    WasInformedBy,
}

impl ProvRelationKind {
    fn jsonld_property(self) -> &'static str {
        match self {
            Self::Used => "prov:used",
            Self::WasGeneratedBy => "prov:wasGeneratedBy",
            Self::WasAssociatedWith => "prov:wasAssociatedWith",
            Self::WasAttributedTo => "prov:wasAttributedTo",
            Self::WasDerivedFrom => "prov:wasDerivedFrom",
            Self::WasInformedBy => "prov:wasInformedBy",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProvRelation {
    pub kind: ProvRelationKind,
    pub subject_id: String,
    pub object_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementModality {
    QcmArealMass,
    OpticalWitnessTransmittance,
    OpticalWitnessReflectance,
    Temperature,
    CapturedMass,
    PurifiedMass,
    IndependentAssayMass,
    GasComposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementClaimKind {
    MetrologyObservation,
    IntermediateProxy,
    CapturedProduct,
    PurifiedProduct,
    IndependentProductAssay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationDisposition {
    EligibleAtMeasurementTime,
    Unknown,
    Revoked,
    Superseded,
    Expired,
    ScopeMismatch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementClaim {
    pub id: String,
    pub sample_id: String,
    pub instrument_id: String,
    pub calibration_certificate_id: String,
    pub activity_id: String,
    pub modality: MeasurementModality,
    pub claim_kind: MeasurementClaimKind,
    /// Must be a resolvable QUDT unit URI for P2.
    pub unit_uri: String,
    pub value: f64,
    pub absolute_uncertainty: f64,
    pub observed_at_utc_s: i64,
    pub recorded_at_utc_s: i64,
    pub calibration_disposition: CalibrationDisposition,
    pub calibration_verification_receipt_id: Option<String>,
    pub qualification_receipt_id: Option<String>,
    pub method_id: Option<String>,
    pub reference_material_id: Option<String>,
    pub laboratory_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductAcceptance {
    pub product_lot_id: String,
    pub independent_assay_claim_id: String,
    pub acceptance_activity_id: String,
    pub acceptance_authority_agent_id: String,
    pub acceptance_receipt_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntheticFantasy {
    pub id: String,
    pub policy_id: String,
    pub job_id: String,
    pub seed: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortableEvidenceProfile {
    pub crate_id: String,
    pub crate_name: String,
    pub profile_uri: String,
    pub profile_version: String,
    pub nodes: Vec<EvidenceNode>,
    pub relations: Vec<ProvRelation>,
    pub measurements: Vec<MeasurementClaim>,
    pub product_acceptance: Option<ProductAcceptance>,
    pub synthetic_fantasies: Vec<SyntheticFantasy>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileFinding {
    EmptyCrateIdentity,
    InvalidProfileUri,
    UnsupportedProfileVersion,
    DuplicateNode(String),
    InvalidNodeId(String),
    InvalidNodeName(String),
    InvalidArtifactLocation(String),
    MissingArtifact(String),
    InvalidArtifactMediaType(String),
    InvalidArtifactHash(String),
    DuplicateUnknownField {
        node_id: String,
        field: String,
    },
    InvalidUnknownField {
        node_id: String,
        field: String,
    },
    UnknownNodeReference {
        source_id: String,
        target_id: String,
    },
    InvalidProvRelation {
        kind: ProvRelationKind,
        subject_id: String,
        object_id: String,
    },
    DuplicateProvRelation(ProvRelation),
    ProvenanceNotConnectedToCrateRoot,
    MissingMeasurementNode(String),
    MissingMeasurementProvenance(String),
    DuplicateMeasurementClaim(String),
    MissingMeasurementReference {
        claim_id: String,
        field: &'static str,
    },
    InvalidMeasurementValue(String),
    InvalidMeasurementUncertainty(String),
    MeasurementTimeInverted(String),
    InvalidQudtUnit(String),
    InvalidModalityClaimPair(String),
    QcmCannotStandForOpticalTransmission(String),
    IneligibleCalibration(String),
    MissingCalibrationVerificationReceipt(String),
    InvalidCalibrationVerificationReceipt(String),
    MissingQualificationReceipt(String),
    InvalidQualificationReceipt(String),
    MissingMethod(String),
    MissingReferenceMaterial(String),
    MissingLaboratory(String),
    FantasyNotExplicitlyTyped(String),
    FantasyConnectedToPhysicalEvidence(String),
    MissingProductAcceptance,
    InvalidProductAcceptance(String),
    ProductAcceptanceNotIndependent(String),
    MissingReplayProtocol,
    MissingSoftwareVersion,
    MissingStageActivity,
    MissingCrossLabMethod,
    MissingCrossLabReferenceMaterial,
    MissingCrossLabUnit,
}

impl fmt::Display for ProfileFinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileValidation {
    pub level: ConformanceLevel,
    pub findings: Vec<ProfileFinding>,
}

impl ProfileValidation {
    pub fn conforms(&self) -> bool {
        self.findings.is_empty()
    }
}

impl PortableEvidenceProfile {
    fn node_index(&self) -> BTreeMap<&str, &EvidenceNode> {
        self.nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect()
    }

    fn has_relation(&self, kind: ProvRelationKind, subject_id: &str, object_id: &str) -> bool {
        self.relations.iter().any(|relation| {
            relation.kind == kind
                && relation.subject_id == subject_id
                && relation.object_id == object_id
        })
    }

    fn authority_receipt_is_qualified(
        &self,
        receipt_id: &str,
        nodes: &BTreeMap<&str, &EvidenceNode>,
    ) -> bool {
        if !matches!(
            nodes.get(receipt_id).map(|node| &node.class),
            Some(NodeClass::Entity(EntityKind::AuthorityReceipt))
        ) {
            return false;
        }

        self.relations.iter().any(|generated| {
            if generated.kind != ProvRelationKind::WasGeneratedBy
                || generated.subject_id != receipt_id
            {
                return false;
            }
            let activity_id = generated.object_id.as_str();
            if !matches!(
                nodes.get(activity_id).map(|node| &node.class),
                Some(NodeClass::Activity(_))
            ) {
                return false;
            }
            self.relations.iter().any(|associated| {
                associated.kind == ProvRelationKind::WasAssociatedWith
                    && associated.subject_id == activity_id
                    && matches!(
                        nodes.get(associated.object_id.as_str()).map(|node| &node.class),
                        Some(NodeClass::Agent(AgentKind::Authority))
                    )
            })
        })
    }

    fn relation_valid(kind: ProvRelationKind, subject: &NodeClass, object: &NodeClass) -> bool {
        match kind {
            ProvRelationKind::Used => {
                matches!(subject, NodeClass::Activity(_)) && matches!(object, NodeClass::Entity(_))
            }
            ProvRelationKind::WasGeneratedBy => {
                matches!(subject, NodeClass::Entity(_)) && matches!(object, NodeClass::Activity(_))
            }
            ProvRelationKind::WasAssociatedWith => {
                matches!(subject, NodeClass::Activity(_)) && matches!(object, NodeClass::Agent(_))
            }
            ProvRelationKind::WasAttributedTo => {
                matches!(subject, NodeClass::Entity(_)) && matches!(object, NodeClass::Agent(_))
            }
            ProvRelationKind::WasDerivedFrom => {
                matches!(subject, NodeClass::Entity(_)) && matches!(object, NodeClass::Entity(_))
            }
            ProvRelationKind::WasInformedBy => {
                matches!(subject, NodeClass::Activity(_))
                    && matches!(object, NodeClass::Activity(_))
            }
        }
    }

    fn validate_p0(&self, findings: &mut Vec<ProfileFinding>) {
        if !absolute_uri(&self.crate_id) || !nonempty(&self.crate_name) {
            findings.push(ProfileFinding::EmptyCrateIdentity);
        }
        if !absolute_uri(&self.profile_uri) {
            findings.push(ProfileFinding::InvalidProfileUri);
        }
        if self.profile_version != "0.1" {
            findings.push(ProfileFinding::UnsupportedProfileVersion);
        }

        let mut nodes = BTreeMap::new();
        for node in &self.nodes {
            if !absolute_uri(&node.id) && !node.id.starts_with("./") {
                findings.push(ProfileFinding::InvalidNodeId(node.id.clone()));
            }
            if !nonempty(&node.name) {
                findings.push(ProfileFinding::InvalidNodeName(node.id.clone()));
            }
            if nodes.insert(node.id.as_str(), node).is_some() {
                findings.push(ProfileFinding::DuplicateNode(node.id.clone()));
            }

            if matches!(
                node.class,
                NodeClass::Entity(EntityKind::RawDataFile | EntityKind::DerivedDataFile)
            ) && node.artifact.is_none()
            {
                findings.push(ProfileFinding::MissingArtifact(node.id.clone()));
            }

            if let Some(artifact) = &node.artifact {
                if !nonempty(&artifact.location)
                    || (!artifact.location.starts_with("./") && !absolute_uri(&artifact.location))
                {
                    findings.push(ProfileFinding::InvalidArtifactLocation(node.id.clone()));
                }
                if !nonempty(&artifact.media_type) {
                    findings.push(ProfileFinding::InvalidArtifactMediaType(node.id.clone()));
                }
                if !valid_sha256(&artifact.sha256) {
                    findings.push(ProfileFinding::InvalidArtifactHash(node.id.clone()));
                }
            }

            let mut unknown_names = BTreeSet::new();
            for unknown in &node.unknown_fields {
                if !nonempty(&unknown.field) || !nonempty(&unknown.reason) {
                    findings.push(ProfileFinding::InvalidUnknownField {
                        node_id: node.id.clone(),
                        field: unknown.field.clone(),
                    });
                }
                if !unknown_names.insert(unknown.field.as_str()) {
                    findings.push(ProfileFinding::DuplicateUnknownField {
                        node_id: node.id.clone(),
                        field: unknown.field.clone(),
                    });
                }
            }
        }

        let mut measurement_ids = BTreeSet::new();
        for claim in &self.measurements {
            if !measurement_ids.insert(claim.id.as_str()) {
                findings.push(ProfileFinding::DuplicateMeasurementClaim(claim.id.clone()));
            }
        }

        let mut fantasy_ids = BTreeSet::new();
        for fantasy in &self.synthetic_fantasies {
            if !fantasy_ids.insert(fantasy.id.as_str()) {
                findings.push(ProfileFinding::FantasyNotExplicitlyTyped(
                    fantasy.id.clone(),
                ));
            }
        }

        let mut relation_set = BTreeSet::new();
        for relation in &self.relations {
            if !relation_set.insert(relation.clone()) {
                findings.push(ProfileFinding::DuplicateProvRelation(relation.clone()));
            }
            let Some(subject) = nodes.get(relation.subject_id.as_str()) else {
                findings.push(ProfileFinding::UnknownNodeReference {
                    source_id: relation.subject_id.clone(),
                    target_id: relation.object_id.clone(),
                });
                continue;
            };
            let Some(object) = nodes.get(relation.object_id.as_str()) else {
                findings.push(ProfileFinding::UnknownNodeReference {
                    source_id: relation.subject_id.clone(),
                    target_id: relation.object_id.clone(),
                });
                continue;
            };
            if !Self::relation_valid(relation.kind, &subject.class, &object.class) {
                findings.push(ProfileFinding::InvalidProvRelation {
                    kind: relation.kind,
                    subject_id: relation.subject_id.clone(),
                    object_id: relation.object_id.clone(),
                });
            }
        }

        for fantasy in &self.synthetic_fantasies {
            if !nonempty(&fantasy.id) || !nonempty(&fantasy.policy_id) || !nonempty(&fantasy.job_id)
            {
                findings.push(ProfileFinding::FantasyNotExplicitlyTyped(
                    fantasy.id.clone(),
                ));
            }
            if nodes.contains_key(fantasy.id.as_str()) {
                findings.push(ProfileFinding::FantasyNotExplicitlyTyped(
                    fantasy.id.clone(),
                ));
            }
            if self.relations.iter().any(|relation| {
                relation.subject_id == fantasy.id || relation.object_id == fantasy.id
            }) {
                findings.push(ProfileFinding::FantasyConnectedToPhysicalEvidence(
                    fantasy.id.clone(),
                ));
            }
        }
    }

    fn validate_p1(&self, findings: &mut Vec<ProfileFinding>) {
        let nodes = self.node_index();
        let Some(root) = nodes.get("./") else {
            findings.push(ProfileFinding::ProvenanceNotConnectedToCrateRoot);
            return;
        };
        if root.class != NodeClass::Entity(EntityKind::CrateDataset) {
            findings.push(ProfileFinding::ProvenanceNotConnectedToCrateRoot);
        }

        let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for relation in &self.relations {
            adjacency
                .entry(relation.subject_id.as_str())
                .or_default()
                .push(relation.object_id.as_str());
            adjacency
                .entry(relation.object_id.as_str())
                .or_default()
                .push(relation.subject_id.as_str());
        }

        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::from(["./"]);
        while let Some(id) = queue.pop_front() {
            if !visited.insert(id) {
                continue;
            }
            if let Some(neighbours) = adjacency.get(id) {
                for neighbour in neighbours {
                    if !visited.contains(neighbour) {
                        queue.push_back(neighbour);
                    }
                }
            }
        }

        if self
            .nodes
            .iter()
            .any(|node| !visited.contains(node.id.as_str()))
        {
            findings.push(ProfileFinding::ProvenanceNotConnectedToCrateRoot);
        }

        for claim in &self.measurements {
            if !nodes.contains_key(claim.id.as_str()) {
                findings.push(ProfileFinding::MissingMeasurementNode(claim.id.clone()));
            }
        }
    }

    fn validate_p2(&self, findings: &mut Vec<ProfileFinding>) {
        let nodes = self.node_index();
        for claim in &self.measurements {
            for (field, id) in [
                ("sample_id", claim.sample_id.as_str()),
                ("instrument_id", claim.instrument_id.as_str()),
                (
                    "calibration_certificate_id",
                    claim.calibration_certificate_id.as_str(),
                ),
                ("activity_id", claim.activity_id.as_str()),
            ] {
                if !nodes.contains_key(id) {
                    findings.push(ProfileFinding::MissingMeasurementReference {
                        claim_id: claim.id.clone(),
                        field,
                    });
                }
            }
            if let Some(measurement_node) = nodes.get(claim.id.as_str()) {
                if !matches!(
                    measurement_node.class,
                    NodeClass::Entity(EntityKind::RawDataFile | EntityKind::DerivedDataFile)
                ) {
                    findings.push(ProfileFinding::MissingMeasurementNode(claim.id.clone()));
                }
            }
            if !self.has_relation(
                ProvRelationKind::WasGeneratedBy,
                &claim.id,
                &claim.activity_id,
            ) || !self.has_relation(ProvRelationKind::Used, &claim.activity_id, &claim.sample_id)
                || !self.has_relation(
                    ProvRelationKind::Used,
                    &claim.activity_id,
                    &claim.instrument_id,
                )
                || !self.has_relation(
                    ProvRelationKind::Used,
                    &claim.activity_id,
                    &claim.calibration_certificate_id,
                )
            {
                findings.push(ProfileFinding::MissingMeasurementProvenance(
                    claim.id.clone(),
                ));
            }
            if !claim.value.is_finite() {
                findings.push(ProfileFinding::InvalidMeasurementValue(claim.id.clone()));
            }
            if !claim.absolute_uncertainty.is_finite() || claim.absolute_uncertainty < 0.0 {
                findings.push(ProfileFinding::InvalidMeasurementUncertainty(
                    claim.id.clone(),
                ));
            }
            if claim.observed_at_utc_s > claim.recorded_at_utc_s {
                findings.push(ProfileFinding::MeasurementTimeInverted(claim.id.clone()));
            }
            if !claim.unit_uri.starts_with("http://qudt.org/vocab/unit/")
                && !claim.unit_uri.starts_with("https://qudt.org/vocab/unit/")
            {
                findings.push(ProfileFinding::InvalidQudtUnit(claim.id.clone()));
            }

            let qcm_is_proxy_only = claim.modality == MeasurementModality::QcmArealMass
                && claim.claim_kind == MeasurementClaimKind::MetrologyObservation;
            let optical_witness_is_optical = matches!(
                claim.modality,
                MeasurementModality::OpticalWitnessTransmittance
                    | MeasurementModality::OpticalWitnessReflectance
            ) && claim.claim_kind
                == MeasurementClaimKind::MetrologyObservation;

            let claim_pair_valid = match claim.claim_kind {
                MeasurementClaimKind::MetrologyObservation => {
                    qcm_is_proxy_only
                        || optical_witness_is_optical
                        || matches!(
                            claim.modality,
                            MeasurementModality::Temperature | MeasurementModality::GasComposition
                        )
                }
                MeasurementClaimKind::IntermediateProxy => matches!(
                    claim.modality,
                    MeasurementModality::Temperature | MeasurementModality::GasComposition
                ),
                MeasurementClaimKind::CapturedProduct => {
                    claim.modality == MeasurementModality::CapturedMass
                }
                MeasurementClaimKind::PurifiedProduct => {
                    claim.modality == MeasurementModality::PurifiedMass
                }
                MeasurementClaimKind::IndependentProductAssay => {
                    claim.modality == MeasurementModality::IndependentAssayMass
                }
            };
            if !claim_pair_valid {
                findings.push(ProfileFinding::InvalidModalityClaimPair(claim.id.clone()));
            }
            if claim.modality == MeasurementModality::QcmArealMass
                && claim.claim_kind != MeasurementClaimKind::MetrologyObservation
            {
                findings.push(ProfileFinding::QcmCannotStandForOpticalTransmission(
                    claim.id.clone(),
                ));
            }

            if claim.calibration_disposition != CalibrationDisposition::EligibleAtMeasurementTime {
                findings.push(ProfileFinding::IneligibleCalibration(claim.id.clone()));
            }
            match claim.calibration_verification_receipt_id.as_deref() {
                None | Some("") => {
                    findings.push(ProfileFinding::MissingCalibrationVerificationReceipt(
                        claim.id.clone(),
                    ));
                }
                Some(receipt_id) if !self.authority_receipt_is_qualified(receipt_id, &nodes) => {
                    findings.push(ProfileFinding::InvalidCalibrationVerificationReceipt(
                        claim.id.clone(),
                    ));
                }
                Some(_) => {}
            }
            match claim.qualification_receipt_id.as_deref() {
                None | Some("") => {
                    findings.push(ProfileFinding::MissingQualificationReceipt(
                        claim.id.clone(),
                    ));
                }
                Some(receipt_id) if !self.authority_receipt_is_qualified(receipt_id, &nodes) => {
                    findings.push(ProfileFinding::InvalidQualificationReceipt(
                        claim.id.clone(),
                    ));
                }
                Some(_) => {}
            }
        }

        if let Some(acceptance) = &self.product_acceptance {
            let assay = self
                .measurements
                .iter()
                .find(|claim| claim.id == acceptance.independent_assay_claim_id);
            match assay {
                Some(claim)
                    if claim.claim_kind == MeasurementClaimKind::IndependentProductAssay
                        && claim.modality == MeasurementModality::IndependentAssayMass
                        && claim.calibration_disposition
                            == CalibrationDisposition::EligibleAtMeasurementTime => {}
                _ => findings.push(ProfileFinding::ProductAcceptanceNotIndependent(
                    acceptance.independent_assay_claim_id.clone(),
                )),
            }

            for id in [
                acceptance.product_lot_id.as_str(),
                acceptance.acceptance_activity_id.as_str(),
                acceptance.acceptance_authority_agent_id.as_str(),
                acceptance.acceptance_receipt_id.as_str(),
            ] {
                if !nodes.contains_key(id) {
                    findings.push(ProfileFinding::InvalidProductAcceptance(id.to_string()));
                }
            }

            if !matches!(
                nodes
                    .get(acceptance.acceptance_receipt_id.as_str())
                    .map(|node| &node.class),
                Some(NodeClass::Entity(EntityKind::AuthorityReceipt))
            ) || !matches!(
                nodes
                    .get(acceptance.acceptance_activity_id.as_str())
                    .map(|node| &node.class),
                Some(NodeClass::Activity(_))
            ) || !matches!(
                nodes
                    .get(acceptance.acceptance_authority_agent_id.as_str())
                    .map(|node| &node.class),
                Some(NodeClass::Agent(_))
            ) {
                findings.push(ProfileFinding::InvalidProductAcceptance(
                    "acceptance-node-types".to_string(),
                ));
            }

            if !self.has_relation(
                ProvRelationKind::WasGeneratedBy,
                &acceptance.acceptance_receipt_id,
                &acceptance.acceptance_activity_id,
            ) || !self.has_relation(
                ProvRelationKind::Used,
                &acceptance.acceptance_activity_id,
                &acceptance.product_lot_id,
            ) || !self.has_relation(
                ProvRelationKind::Used,
                &acceptance.acceptance_activity_id,
                &acceptance.independent_assay_claim_id,
            ) || !self.has_relation(
                ProvRelationKind::WasAssociatedWith,
                &acceptance.acceptance_activity_id,
                &acceptance.acceptance_authority_agent_id,
            ) {
                findings.push(ProfileFinding::InvalidProductAcceptance(
                    "acceptance-provenance".to_string(),
                ));
            }
        } else {
            findings.push(ProfileFinding::MissingProductAcceptance);
        }
    }

    fn validate_p3(&self, findings: &mut Vec<ProfileFinding>) {
        let has_protocol = self
            .nodes
            .iter()
            .any(|node| node.class == NodeClass::Entity(EntityKind::Protocol));
        let has_software = self.nodes.iter().any(|node| {
            matches!(
                node.class,
                NodeClass::Entity(EntityKind::Software | EntityKind::Firmware)
            )
        });
        let has_stage_activity = self
            .nodes
            .iter()
            .any(|node| matches!(node.class, NodeClass::Activity(_)));
        if !has_protocol {
            findings.push(ProfileFinding::MissingReplayProtocol);
        }
        if !has_software {
            findings.push(ProfileFinding::MissingSoftwareVersion);
        }
        if !has_stage_activity {
            findings.push(ProfileFinding::MissingStageActivity);
        }
    }

    fn validate_p4(&self, findings: &mut Vec<ProfileFinding>) {
        let nodes = self.node_index();
        for claim in &self.measurements {
            if claim.method_id.as_deref().is_none_or(|id| {
                !matches!(
                    nodes.get(id).map(|node| &node.class),
                    Some(NodeClass::Entity(EntityKind::Method))
                )
            }) {
                findings.push(ProfileFinding::MissingCrossLabMethod);
            }
            if claim.reference_material_id.as_deref().is_none_or(|id| {
                !matches!(
                    nodes.get(id).map(|node| &node.class),
                    Some(NodeClass::Entity(EntityKind::ReferenceMaterial))
                )
            }) {
                findings.push(ProfileFinding::MissingCrossLabReferenceMaterial);
            }
            if !claim.unit_uri.starts_with("http://qudt.org/vocab/unit/")
                && !claim.unit_uri.starts_with("https://qudt.org/vocab/unit/")
            {
                findings.push(ProfileFinding::MissingCrossLabUnit);
            }
            if claim.laboratory_id.as_deref().is_none_or(|id| {
                !matches!(
                    nodes.get(id).map(|node| &node.class),
                    Some(NodeClass::Entity(EntityKind::Laboratory))
                )
            }) {
                findings.push(ProfileFinding::MissingLaboratory(claim.id.clone()));
            }
        }
    }

    pub fn validate(&self, level: ConformanceLevel) -> ProfileValidation {
        let mut findings = Vec::new();
        self.validate_p0(&mut findings);
        if matches!(
            level,
            ConformanceLevel::P1ProvenanceComplete
                | ConformanceLevel::P2MetrologyQualified
                | ConformanceLevel::P3Replayable
                | ConformanceLevel::P4CrossLabComparable
        ) {
            self.validate_p1(&mut findings);
        }
        if matches!(
            level,
            ConformanceLevel::P2MetrologyQualified
                | ConformanceLevel::P3Replayable
                | ConformanceLevel::P4CrossLabComparable
        ) {
            self.validate_p2(&mut findings);
        }
        if matches!(
            level,
            ConformanceLevel::P3Replayable | ConformanceLevel::P4CrossLabComparable
        ) {
            self.validate_p3(&mut findings);
        }
        if level == ConformanceLevel::P4CrossLabComparable {
            self.validate_p4(&mut findings);
        }
        ProfileValidation { level, findings }
    }

    /// Emit a compact flattened JSON-LD metadata document using the published
    /// RO-Crate 1.3 context plus PROV-O, QUDT and the Luminous profile namespace.
    /// This is an exporter for the typed profile, not a general JSON-LD serializer.
    pub fn to_ro_crate_jsonld(&self) -> String {
        let mut output = String::new();
        output.push_str("{\"@context\":[");
        output.push('"');
        output.push_str(RO_CRATE_CONTEXT);
        output.push_str("\",{");
        output.push_str("\"schema\":\"https://schema.org/\",");
        output.push_str("\"prov\":\"http://www.w3.org/ns/prov#\",");
        output.push_str("\"qudt\":\"http://qudt.org/schema/qudt/\",");
        output.push_str("\"unit\":\"http://qudt.org/vocab/unit/\",");
        output.push_str("\"lum\":\"https://luminousdynamics.io/ontology/isru#\"");
        output.push_str("}],\"@graph\":[");

        output.push_str("{\"@id\":\"ro-crate-metadata.json\",");
        output.push_str("\"@type\":\"CreativeWork\",");
        output.push_str("\"conformsTo\":{\"@id\":");
        output.push('"');
        output.push_str(RO_CRATE_VERSION);
        output.push_str("\"},\"about\":{\"@id\":\"./\"}},");

        output.push_str("{\"@id\":\"./\",\"@type\":[\"Dataset\",\"prov:Entity\"],");
        output.push_str("\"name\":\"");
        output.push_str(&json_escape(&self.crate_name));
        output.push_str("\",\"identifier\":\"");
        output.push_str(&json_escape(&self.crate_id));
        output.push_str("\",\"conformsTo\":[{\"@id\":\"");
        output.push_str(RO_CRATE_VERSION);
        output.push_str("\"},{\"@id\":\"");
        output.push_str(&json_escape(&self.profile_uri));
        output.push_str("\"}]");

        for relation in self
            .relations
            .iter()
            .filter(|relation| relation.subject_id == "./")
        {
            output.push_str(",\"");
            output.push_str(relation.kind.jsonld_property());
            output.push_str("\":{\"@id\":\"");
            output.push_str(&json_escape(&relation.object_id));
            output.push_str("\"}");
        }
        output.push('}');

        let mut sorted_nodes: Vec<&EvidenceNode> =
            self.nodes.iter().filter(|node| node.id != "./").collect();
        sorted_nodes.sort_by_key(|node| node.id.as_str());
        for node in sorted_nodes {
            output.push_str(",{\"@id\":\"");
            output.push_str(&json_escape(&node.id));
            output.push_str("\",\"name\":\"");
            output.push_str(&json_escape(&node.name));
            output.push_str("\",\"@type\":[\"");
            output.push_str(match node.class {
                NodeClass::Entity(_) => "prov:Entity",
                NodeClass::Activity(_) => "prov:Activity",
                NodeClass::Agent(_) => "prov:Agent",
            });
            output.push_str("\",\"");
            output.push_str(node_type_name(&node.class));
            output.push_str("\"]");

            if let Some(artifact) = &node.artifact {
                output.push_str(",\"contentUrl\":\"");
                output.push_str(&json_escape(&artifact.location));
                output.push_str("\",\"encodingFormat\":\"");
                output.push_str(&json_escape(&artifact.media_type));
                output.push_str("\",\"lum:sha256\":\"");
                output.push_str(&json_escape(&artifact.sha256));
                output.push('"');
            }

            if !node.unknown_fields.is_empty() {
                output.push_str(",\"lum:unknownFields\":[");
                for (index, unknown) in node.unknown_fields.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    output.push_str("{\"field\":\"");
                    output.push_str(&json_escape(&unknown.field));
                    output.push_str("\",\"reason\":\"");
                    output.push_str(&json_escape(&unknown.reason));
                    output.push_str("\"}");
                }
                output.push(']');
            }

            if let Some(claim) = self.measurements.iter().find(|claim| claim.id == node.id) {
                output.push_str(",\"lum:measurementClaim\":true,\"lum:sample\":{\"@id\":\"");
                output.push_str(&json_escape(&claim.sample_id));
                output.push_str("\"},\"lum:instrument\":{\"@id\":\"");
                output.push_str(&json_escape(&claim.instrument_id));
                output.push_str("\"},\"lum:calibrationCertificate\":{\"@id\":\"");
                output.push_str(&json_escape(&claim.calibration_certificate_id));
                output.push_str("\"},\"lum:activity\":{\"@id\":\"");
                output.push_str(&json_escape(&claim.activity_id));
                output.push_str("\"},\"lum:modality\":\"");
                output.push_str(modality_name(claim.modality));
                output.push_str("\",\"lum:claimKind\":\"");
                output.push_str(claim_kind_name(claim.claim_kind));
                output.push_str("\",\"lum:value\":");
                output.push_str(&claim.value.to_string());
                output.push_str(",\"lum:absoluteUncertainty\":");
                output.push_str(&claim.absolute_uncertainty.to_string());
                output.push_str(",\"qudt:unit\":{\"@id\":\"");
                output.push_str(&json_escape(&claim.unit_uri));
                output.push_str("\"},\"lum:observedAt\":");
                output.push_str(&claim.observed_at_utc_s.to_string());
                output.push_str(",\"lum:recordedAt\":");
                output.push_str(&claim.recorded_at_utc_s.to_string());
                output.push_str(",\"lum:calibrationDisposition\":\"");
                output.push_str(calibration_name(claim.calibration_disposition));
                output.push('"');
            }

            if let Some(acceptance) = self
                .product_acceptance
                .as_ref()
                .filter(|item| item.acceptance_receipt_id == node.id)
            {
                output.push_str(",\"lum:productLot\":{\"@id\":\"");
                output.push_str(&json_escape(&acceptance.product_lot_id));
                output.push_str("\"},\"lum:independentAssay\":{\"@id\":\"");
                output.push_str(&json_escape(&acceptance.independent_assay_claim_id));
                output.push_str("\"},\"lum:acceptanceActivity\":{\"@id\":\"");
                output.push_str(&json_escape(&acceptance.acceptance_activity_id));
                output.push_str("\"},\"lum:acceptanceAuthority\":{\"@id\":\"");
                output.push_str(&json_escape(&acceptance.acceptance_authority_agent_id));
                output.push_str("\"}");
            }

            for relation in self
                .relations
                .iter()
                .filter(|relation| relation.subject_id == node.id)
            {
                output.push_str(",\"");
                output.push_str(relation.kind.jsonld_property());
                output.push_str("\":{\"@id\":\"");
                output.push_str(&json_escape(&relation.object_id));
                output.push_str("\"}");
            }
            output.push('}');
        }

        for fantasy in &self.synthetic_fantasies {
            output.push_str(",{\"@id\":\"");
            output.push_str(&json_escape(&fantasy.id));
            output.push_str("\",\"@type\":[\"prov:Entity\",\"lum:OptimizerFantasy\"],");
            output.push_str("\"lum:isSyntheticFantasy\":true,\"lum:policyId\":\"");
            output.push_str(&json_escape(&fantasy.policy_id));
            output.push_str("\",\"lum:jobId\":\"");
            output.push_str(&json_escape(&fantasy.job_id));
            output.push('"');
            if let Some(seed) = fantasy.seed {
                output.push_str(",\"lum:seed\":");
                output.push_str(&seed.to_string());
            }
            output.push('}');
        }

        output.push_str("]}");
        output
    }
}

fn node_type_name(class: &NodeClass) -> &'static str {
    match class {
        NodeClass::Entity(kind) => match kind {
            EntityKind::CrateDataset => "schema:Dataset",
            EntityKind::Laboratory => "schema:Place",
            EntityKind::Rig
            | EntityKind::Instrument
            | EntityKind::Detector
            | EntityKind::OpticalSource
            | EntityKind::OpticalPath => "schema:IndividualProduct",
            EntityKind::RawDataFile | EntityKind::DerivedDataFile => "schema:MediaObject",
            EntityKind::MaterialFamily
            | EntityKind::FeedstockBatch
            | EntityKind::Sample
            | EntityKind::ProductLot
            | EntityKind::OptimizerFantasy => "schema:Thing",
            EntityKind::CalibrationCertificate
            | EntityKind::ReferenceMaterial
            | EntityKind::Method
            | EntityKind::Protocol
            | EntityKind::Software
            | EntityKind::Firmware
            | EntityKind::AuthorityReceipt => "schema:CreativeWork",
        },
        NodeClass::Activity(_) => "schema:Action",
        NodeClass::Agent(kind) => match kind {
            AgentKind::Person => "schema:Person",
            AgentKind::Organization | AgentKind::Authority => "schema:Organization",
            AgentKind::SoftwareAgent | AgentKind::InstrumentAgent => "schema:SoftwareApplication",
        },
    }
}

fn modality_name(modality: MeasurementModality) -> &'static str {
    match modality {
        MeasurementModality::QcmArealMass => "qcm-areal-mass",
        MeasurementModality::OpticalWitnessTransmittance => "optical-witness-transmittance",
        MeasurementModality::OpticalWitnessReflectance => "optical-witness-reflectance",
        MeasurementModality::Temperature => "temperature",
        MeasurementModality::CapturedMass => "captured-mass",
        MeasurementModality::PurifiedMass => "purified-mass",
        MeasurementModality::IndependentAssayMass => "independent-assay-mass",
        MeasurementModality::GasComposition => "gas-composition",
    }
}

fn claim_kind_name(kind: MeasurementClaimKind) -> &'static str {
    match kind {
        MeasurementClaimKind::MetrologyObservation => "metrology-observation",
        MeasurementClaimKind::IntermediateProxy => "intermediate-proxy",
        MeasurementClaimKind::CapturedProduct => "captured-product",
        MeasurementClaimKind::PurifiedProduct => "purified-product",
        MeasurementClaimKind::IndependentProductAssay => "independent-product-assay",
    }
}

fn calibration_name(status: CalibrationDisposition) -> &'static str {
    match status {
        CalibrationDisposition::EligibleAtMeasurementTime => "eligible-at-measurement-time",
        CalibrationDisposition::Unknown => "unknown",
        CalibrationDisposition::Revoked => "revoked",
        CalibrationDisposition::Superseded => "superseded",
        CalibrationDisposition::Expired => "expired",
        CalibrationDisposition::ScopeMismatch => "scope-mismatch",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, name: &str, class: NodeClass) -> EvidenceNode {
        EvidenceNode {
            id: id.to_string(),
            name: name.to_string(),
            class,
            artifact: None,
            unknown_fields: Vec::new(),
        }
    }

    fn artifact_node(id: &str, name: &str) -> EvidenceNode {
        let mut value = node(id, name, NodeClass::Entity(EntityKind::RawDataFile));
        let filename = id.rsplit(':').next().unwrap_or("observation");
        value.artifact = Some(ArtifactMetadata {
            location: format!("./data/{filename}.csv"),
            media_type: "text/csv".to_string(),
            sha256: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
        });
        value
    }

    fn profile() -> PortableEvidenceProfile {
        let mut nodes = vec![
            node(
                "./",
                "Optical ISRU campaign crate",
                NodeClass::Entity(EntityKind::CrateDataset),
            ),
            node(
                "urn:lab:one",
                "Lab One",
                NodeClass::Entity(EntityKind::Laboratory),
            ),
            node(
                "urn:lab:agent",
                "Laboratory organization",
                NodeClass::Agent(AgentKind::Organization),
            ),
            node(
                "urn:rig:one",
                "Optical rig",
                NodeClass::Entity(EntityKind::Rig),
            ),
            node(
                "urn:instrument:spec",
                "Spectrometer",
                NodeClass::Entity(EntityKind::Instrument),
            ),
            node(
                "urn:instrument:qcm",
                "Quartz crystal microbalance",
                NodeClass::Entity(EntityKind::Instrument),
            ),
            node(
                "urn:batch:one",
                "Feedstock batch",
                NodeClass::Entity(EntityKind::FeedstockBatch),
            ),
            node(
                "urn:sample:one",
                "Specimen",
                NodeClass::Entity(EntityKind::Sample),
            ),
            node(
                "urn:cal:one",
                "Spectrometer calibration cert",
                NodeClass::Entity(EntityKind::CalibrationCertificate),
            ),
            node(
                "urn:cal:qcm",
                "QCM calibration cert",
                NodeClass::Entity(EntityKind::CalibrationCertificate),
            ),
            node(
                "urn:method:one",
                "Witness method",
                NodeClass::Entity(EntityKind::Method),
            ),
            node(
                "urn:reference:one",
                "Optical reference tile",
                NodeClass::Entity(EntityKind::ReferenceMaterial),
            ),
            node(
                "urn:protocol:one",
                "Protocol v1",
                NodeClass::Entity(EntityKind::Protocol),
            ),
            node(
                "urn:software:one",
                "Analysis code",
                NodeClass::Entity(EntityKind::Software),
            ),
            node(
                "urn:activity:crate",
                "Evidence package assembly",
                NodeClass::Activity(ActivityKind::DataTransformation),
            ),
            node(
                "urn:activity:prep",
                "Sample preparation",
                NodeClass::Activity(ActivityKind::SamplePreparation),
            ),
            node(
                "urn:activity:qcm",
                "QCM deposition measurement",
                NodeClass::Activity(ActivityKind::QcmMeasurement),
            ),
            node(
                "urn:activity:measure",
                "Optical witness reading",
                NodeClass::Activity(ActivityKind::OpticalWitnessMeasurement),
            ),
            node(
                "urn:activity:capture",
                "Product capture",
                NodeClass::Activity(ActivityKind::Capture),
            ),
            node(
                "urn:activity:assay",
                "Independent product assay",
                NodeClass::Activity(ActivityKind::IndependentAssay),
            ),
            node(
                "urn:activity:acceptance",
                "Product acceptance",
                NodeClass::Activity(ActivityKind::Decision),
            ),
            node(
                "urn:operator:one",
                "Operator",
                NodeClass::Agent(AgentKind::Person),
            ),
            node(
                "urn:authority:one",
                "Assay authority",
                NodeClass::Agent(AgentKind::Authority),
            ),
            artifact_node("urn:observation:qcm", "QCM local deposition"),
            artifact_node("urn:observation:witness", "Optical witness transmission"),
            artifact_node("urn:observation:capture", "Captured product mass"),
            artifact_node("urn:observation:assay", "Independent product assay"),
            node(
                "urn:product:one",
                "Captured product lot",
                NodeClass::Entity(EntityKind::ProductLot),
            ),
            node(
                "urn:receipt:acceptance",
                "Product acceptance receipt",
                NodeClass::Entity(EntityKind::AuthorityReceipt),
            ),
        ];

        let mut relations = vec![
            ProvRelation {
                kind: ProvRelationKind::WasGeneratedBy,
                subject_id: "./".to_string(),
                object_id: "urn:activity:crate".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:crate".to_string(),
                object_id: "urn:lab:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasAssociatedWith,
                subject_id: "urn:activity:crate".to_string(),
                object_id: "urn:lab:agent".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasGeneratedBy,
                subject_id: "urn:sample:one".to_string(),
                object_id: "urn:activity:prep".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasDerivedFrom,
                subject_id: "urn:sample:one".to_string(),
                object_id: "urn:batch:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:prep".to_string(),
                object_id: "urn:batch:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:prep".to_string(),
                object_id: "urn:rig:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:prep".to_string(),
                object_id: "urn:protocol:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:prep".to_string(),
                object_id: "urn:method:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:prep".to_string(),
                object_id: "urn:software:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasAssociatedWith,
                subject_id: "urn:activity:prep".to_string(),
                object_id: "urn:lab:agent".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasGeneratedBy,
                subject_id: "urn:observation:qcm".to_string(),
                object_id: "urn:activity:qcm".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:qcm".to_string(),
                object_id: "urn:sample:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:qcm".to_string(),
                object_id: "urn:instrument:qcm".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:qcm".to_string(),
                object_id: "urn:cal:qcm".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasGeneratedBy,
                subject_id: "urn:observation:witness".to_string(),
                object_id: "urn:activity:measure".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:measure".to_string(),
                object_id: "urn:sample:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:measure".to_string(),
                object_id: "urn:instrument:spec".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:measure".to_string(),
                object_id: "urn:cal:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:measure".to_string(),
                object_id: "urn:method:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:measure".to_string(),
                object_id: "urn:reference:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasAssociatedWith,
                subject_id: "urn:activity:measure".to_string(),
                object_id: "urn:operator:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasGeneratedBy,
                subject_id: "urn:product:one".to_string(),
                object_id: "urn:activity:capture".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasDerivedFrom,
                subject_id: "urn:product:one".to_string(),
                object_id: "urn:sample:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasGeneratedBy,
                subject_id: "urn:observation:capture".to_string(),
                object_id: "urn:activity:capture".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:capture".to_string(),
                object_id: "urn:sample:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:capture".to_string(),
                object_id: "urn:rig:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:capture".to_string(),
                object_id: "urn:instrument:spec".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:capture".to_string(),
                object_id: "urn:cal:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasGeneratedBy,
                subject_id: "urn:observation:assay".to_string(),
                object_id: "urn:activity:assay".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:assay".to_string(),
                object_id: "urn:product:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:assay".to_string(),
                object_id: "urn:instrument:spec".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:assay".to_string(),
                object_id: "urn:cal:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:assay".to_string(),
                object_id: "urn:method:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:assay".to_string(),
                object_id: "urn:reference:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasAssociatedWith,
                subject_id: "urn:activity:assay".to_string(),
                object_id: "urn:authority:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasGeneratedBy,
                subject_id: "urn:receipt:acceptance".to_string(),
                object_id: "urn:activity:acceptance".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:acceptance".to_string(),
                object_id: "urn:product:one".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::Used,
                subject_id: "urn:activity:acceptance".to_string(),
                object_id: "urn:observation:assay".to_string(),
            },
            ProvRelation {
                kind: ProvRelationKind::WasAssociatedWith,
                subject_id: "urn:activity:acceptance".to_string(),
                object_id: "urn:authority:one".to_string(),
            },
        ];

        let common_witness = MeasurementClaim {
            id: "urn:observation:witness".to_string(),
            sample_id: "urn:sample:one".to_string(),
            instrument_id: "urn:instrument:spec".to_string(),
            calibration_certificate_id: "urn:cal:one".to_string(),
            activity_id: "urn:activity:measure".to_string(),
            modality: MeasurementModality::OpticalWitnessTransmittance,
            claim_kind: MeasurementClaimKind::MetrologyObservation,
            unit_uri: "http://qudt.org/vocab/unit/UNITLESS".to_string(),
            value: 0.93,
            absolute_uncertainty: 0.01,
            observed_at_utc_s: 100,
            recorded_at_utc_s: 101,
            calibration_disposition: CalibrationDisposition::EligibleAtMeasurementTime,
            calibration_verification_receipt_id: Some("urn:receipt:calibration:witness".to_string()),
            qualification_receipt_id: Some("urn:receipt:qualification:witness".to_string()),
            method_id: Some("urn:method:one".to_string()),
            reference_material_id: Some("urn:reference:one".to_string()),
            laboratory_id: Some("urn:lab:one".to_string()),
        };
        let qcm = MeasurementClaim {
            id: "urn:observation:qcm".to_string(),
            sample_id: "urn:sample:one".to_string(),
            instrument_id: "urn:instrument:qcm".to_string(),
            calibration_certificate_id: "urn:cal:qcm".to_string(),
            activity_id: "urn:activity:qcm".to_string(),
            modality: MeasurementModality::QcmArealMass,
            claim_kind: MeasurementClaimKind::MetrologyObservation,
            unit_uri: "http://qudt.org/vocab/unit/GM-PER-M2".to_string(),
            value: 0.000001,
            absolute_uncertainty: 0.0000001,
            observed_at_utc_s: 99,
            recorded_at_utc_s: 101,
            calibration_disposition: CalibrationDisposition::EligibleAtMeasurementTime,
            calibration_verification_receipt_id: Some("urn:receipt:calibration:qcm".to_string()),
            qualification_receipt_id: Some("urn:receipt:qualification:qcm".to_string()),
            method_id: Some("urn:method:one".to_string()),
            reference_material_id: Some("urn:reference:one".to_string()),
            laboratory_id: Some("urn:lab:one".to_string()),
        };
        let capture = MeasurementClaim {
            id: "urn:observation:capture".to_string(),
            sample_id: "urn:sample:one".to_string(),
            instrument_id: "urn:instrument:spec".to_string(),
            calibration_certificate_id: "urn:cal:one".to_string(),
            activity_id: "urn:activity:capture".to_string(),
            modality: MeasurementModality::CapturedMass,
            claim_kind: MeasurementClaimKind::CapturedProduct,
            unit_uri: "http://qudt.org/vocab/unit/GM".to_string(),
            value: 2.7,
            absolute_uncertainty: 0.05,
            observed_at_utc_s: 150,
            recorded_at_utc_s: 151,
            calibration_disposition: CalibrationDisposition::EligibleAtMeasurementTime,
            calibration_verification_receipt_id: Some("urn:receipt:calibration:capture".to_string()),
            qualification_receipt_id: Some("urn:receipt:qualification:capture".to_string()),
            method_id: Some("urn:method:one".to_string()),
            reference_material_id: Some("urn:reference:one".to_string()),
            laboratory_id: Some("urn:lab:one".to_string()),
        };
        let assay = MeasurementClaim {
            id: "urn:observation:assay".to_string(),
            sample_id: "urn:product:one".to_string(),
            instrument_id: "urn:instrument:spec".to_string(),
            calibration_certificate_id: "urn:cal:one".to_string(),
            activity_id: "urn:activity:assay".to_string(),
            modality: MeasurementModality::IndependentAssayMass,
            claim_kind: MeasurementClaimKind::IndependentProductAssay,
            unit_uri: "http://qudt.org/vocab/unit/GM".to_string(),
            value: 2.4,
            absolute_uncertainty: 0.03,
            observed_at_utc_s: 200,
            recorded_at_utc_s: 201,
            calibration_disposition: CalibrationDisposition::EligibleAtMeasurementTime,
            calibration_verification_receipt_id: Some("urn:receipt:calibration:assay".to_string()),
            qualification_receipt_id: Some("urn:receipt:qualification:assay".to_string()),
            method_id: Some("urn:method:one".to_string()),
            reference_material_id: Some("urn:reference:one".to_string()),
            laboratory_id: Some("urn:lab:one".to_string()),
        };

        for claim in &measurements {
            let suffix = claim.id.rsplit(':').next().unwrap_or("claim");
            for (receipt_id, activity_suffix, activity_name) in [
                (
                    claim.calibration_verification_receipt_id.as_deref().unwrap(),
                    "calibration-verification",
                    "Calibration receipt verification",
                ),
                (
                    claim.qualification_receipt_id.as_deref().unwrap(),
                    "measurement-qualification",
                    "Measurement qualification review",
                ),
            ] {
                let activity_id = format!("urn:activity:{activity_suffix}:{suffix}");
                nodes.push(node(
                    receipt_id,
                    "Authority verification receipt",
                    NodeClass::Entity(EntityKind::AuthorityReceipt),
                ));
                nodes.push(node(
                    &activity_id,
                    activity_name,
                    NodeClass::Activity(ActivityKind::Decision),
                ));
                relations.push(ProvRelation {
                    kind: ProvRelationKind::WasGeneratedBy,
                    subject_id: receipt_id.to_string(),
                    object_id: activity_id.clone(),
                });
                relations.push(ProvRelation {
                    kind: ProvRelationKind::WasAssociatedWith,
                    subject_id: activity_id.clone(),
                    object_id: "urn:authority:one".to_string(),
                });
                relations.push(ProvRelation {
                    kind: ProvRelationKind::Used,
                    subject_id: activity_id,
                    object_id: claim.id.clone(),
                });
            }
        }

        PortableEvidenceProfile {
            crate_id: "urn:crate:campaign-one".to_string(),
            crate_name: "Optical ISRU Campaign".to_string(),
            profile_uri: LUMINOUS_PROFILE_URI.to_string(),
            profile_version: "0.1".to_string(),
            nodes,
            relations,
            measurements: vec![qcm, common_witness, capture, assay],
            product_acceptance: Some(ProductAcceptance {
                product_lot_id: "urn:product:one".to_string(),
                independent_assay_claim_id: "urn:observation:assay".to_string(),
                acceptance_activity_id: "urn:activity:acceptance".to_string(),
                acceptance_authority_agent_id: "urn:authority:one".to_string(),
                acceptance_receipt_id: "urn:receipt:acceptance".to_string(),
            }),
            synthetic_fantasies: Vec::new(),
        }
    }

    #[test]
    fn minimal_fixture_passes_p0_and_p2_when_required_evidence_is_present() {
        let value = profile();
        assert!(value.validate(ConformanceLevel::P0PackageValid).conforms());
        assert!(value
            .validate(ConformanceLevel::P2MetrologyQualified)
            .conforms());
    }

    #[test]
    fn missing_measurement_calibration_and_qualification_receipts_fail_p2() {
        let mut value = profile();
        value.measurements[0].calibration_verification_receipt_id = None;
        value.measurements[0].qualification_receipt_id = None;
        let findings = value
            .validate(ConformanceLevel::P2MetrologyQualified)
            .findings;
        assert!(findings
            .iter()
            .any(|f| matches!(f, ProfileFinding::MissingCalibrationVerificationReceipt(_))));
        assert!(findings
            .iter()
            .any(|f| matches!(f, ProfileFinding::MissingQualificationReceipt(_))));
    }

    #[test]
    fn non_resolving_calibration_or_qualification_receipt_fails_p2() {
        let mut value = profile();
        let witness = value
            .measurements
            .iter_mut()
            .find(|claim| claim.id == "urn:observation:witness")
            .unwrap();
        witness.calibration_verification_receipt_id =
            Some("urn:receipt:missing".to_string());
        witness.qualification_receipt_id = Some("urn:receipt:also-missing".to_string());

        let findings = value.validate(ConformanceLevel::P2MetrologyQualified).findings;
        assert!(findings.contains(&ProfileFinding::InvalidCalibrationVerificationReceipt(
            "urn:observation:witness".to_string()
        )));
        assert!(findings.contains(&ProfileFinding::InvalidQualificationReceipt(
            "urn:observation:witness".to_string()
        )));
    }

    #[test]
    fn receipt_generated_without_authority_attribution_fails_p2() {
        let mut value = profile();
        value.relations.retain(|relation| {
            !(relation.kind == ProvRelationKind::WasAssociatedWith
                && relation.subject_id == "urn:activity:calibration-verification:witness")
        });
        assert!(value.validate(ConformanceLevel::P2MetrologyQualified).findings.contains(
            &ProfileFinding::InvalidCalibrationVerificationReceipt(
                "urn:observation:witness".to_string()
            )
        ));
    }

    #[test]
    fn retroactively_revoked_calibration_fails_p2() {
        let mut value = profile();
        let witness = value
            .measurements
            .iter_mut()
            .find(|claim| claim.id == "urn:observation:witness")
            .unwrap();
        witness.calibration_disposition = CalibrationDisposition::Revoked;
        assert!(value
            .validate(ConformanceLevel::P2MetrologyQualified)
            .findings
            .contains(&ProfileFinding::IneligibleCalibration(
                "urn:observation:witness".to_string()
            )));
    }

    #[test]
    fn qcm_mass_cannot_be_exported_as_optical_throughput() {
        let mut value = profile();
        value.measurements[0].modality = MeasurementModality::QcmArealMass;
        value.measurements[0].claim_kind = MeasurementClaimKind::CapturedProduct;
        let findings = value
            .validate(ConformanceLevel::P2MetrologyQualified)
            .findings;
        assert!(findings
            .iter()
            .any(|f| matches!(f, ProfileFinding::QcmCannotStandForOpticalTransmission(_))));
    }

    #[test]
    fn fantasy_is_explicitly_separate_and_cannot_join_physical_provenance_graph() {
        let mut value = profile();
        value.synthetic_fantasies.push(SyntheticFantasy {
            id: "urn:fantasy:one".to_string(),
            policy_id: "pessimistic-v1".to_string(),
            job_id: "job-17".to_string(),
            seed: Some(42),
        });
        assert!(value
            .validate(ConformanceLevel::P1ProvenanceComplete)
            .conforms());

        value.relations.push(ProvRelation {
            kind: ProvRelationKind::WasDerivedFrom,
            subject_id: "urn:observation:witness".to_string(),
            object_id: "urn:fantasy:one".to_string(),
        });
        assert!(value
            .validate(ConformanceLevel::P0PackageValid)
            .findings
            .contains(&ProfileFinding::FantasyConnectedToPhysicalEvidence(
                "urn:fantasy:one".to_string()
            )));
    }

    #[test]
    fn provenance_relation_types_are_checked_against_prov_o_roles() {
        let mut value = profile();
        value.relations.push(ProvRelation {
            kind: ProvRelationKind::Used,
            subject_id: "urn:sample:one".to_string(),
            object_id: "urn:activity:measure".to_string(),
        });
        assert!(value
            .validate(ConformanceLevel::P1ProvenanceComplete)
            .findings
            .iter()
            .any(|finding| matches!(finding, ProfileFinding::InvalidProvRelation { .. })));
    }

    #[test]
    fn unlinked_node_breaks_provenance_completeness_but_not_package_validity() {
        let mut value = profile();
        value.nodes.push(node(
            "urn:orphan:one",
            "Orphan",
            NodeClass::Entity(EntityKind::Method),
        ));
        assert!(value.validate(ConformanceLevel::P0PackageValid).conforms());
        assert!(value
            .validate(ConformanceLevel::P1ProvenanceComplete)
            .findings
            .contains(&ProfileFinding::ProvenanceNotConnectedToCrateRoot));
    }

    #[test]
    fn missing_cross_lab_method_reference_material_and_lab_fail_p4() {
        let mut value = profile();
        value.measurements[0].method_id = None;
        value.measurements[0].reference_material_id = None;
        value.measurements[0].laboratory_id = None;
        let findings = value
            .validate(ConformanceLevel::P4CrossLabComparable)
            .findings;
        assert!(findings.contains(&ProfileFinding::MissingCrossLabMethod));
        assert!(findings.contains(&ProfileFinding::MissingCrossLabReferenceMaterial));
        assert!(findings
            .iter()
            .any(|f| matches!(f, ProfileFinding::MissingLaboratory(_))));
    }

    #[test]
    fn missing_protocol_software_and_stage_execution_fail_p3() {
        let mut value = profile();
        value.nodes.retain(|node| {
            !matches!(
                node.class,
                NodeClass::Entity(EntityKind::Protocol | EntityKind::Software)
            )
        });
        value
            .nodes
            .retain(|node| !matches!(node.class, NodeClass::Activity(_)));
        let findings = value.validate(ConformanceLevel::P3Replayable).findings;
        assert!(findings.contains(&ProfileFinding::MissingReplayProtocol));
        assert!(findings.contains(&ProfileFinding::MissingSoftwareVersion));
        assert!(findings.contains(&ProfileFinding::MissingStageActivity));
    }

    #[test]
    fn product_acceptance_requires_independent_assay_claim() {
        let mut value = profile();
        value
            .product_acceptance
            .as_mut()
            .unwrap()
            .independent_assay_claim_id = "urn:observation:witness".to_string();
        assert!(value
            .validate(ConformanceLevel::P2MetrologyQualified)
            .findings
            .iter()
            .any(|finding| matches!(finding, ProfileFinding::ProductAcceptanceNotIndependent(_))));
    }

    #[test]
    fn invalid_hashes_and_units_fail_package_and_metrology_checks() {
        let mut value = profile();
        let file = value
            .nodes
            .iter_mut()
            .find(|node| node.id == "urn:observation:witness")
            .unwrap();
        file.artifact = Some(ArtifactMetadata {
            location: "./data/witness.csv".to_string(),
            media_type: "text/csv".to_string(),
            sha256: "not-a-hash".to_string(),
        });
        let witness = value
            .measurements
            .iter_mut()
            .find(|claim| claim.id == "urn:observation:witness")
            .unwrap();
        witness.unit_uri = "urn:unit:unknown".to_string();
        let findings = value
            .validate(ConformanceLevel::P2MetrologyQualified)
            .findings;
        assert!(findings.contains(&ProfileFinding::InvalidArtifactHash(
            "urn:observation:witness".to_string()
        )));
        assert!(findings.contains(&ProfileFinding::InvalidQudtUnit(
            "urn:observation:witness".to_string()
        )));
    }

    #[test]
    fn missing_unknowns_are_preserved_and_never_defaulted() {
        let mut value = profile();
        let sample = value
            .nodes
            .iter_mut()
            .find(|node| node.id == "urn:sample:one")
            .unwrap();
        sample.unknown_fields.push(UnknownField {
            field: "collection_location".to_string(),
            reason: "not recorded in source campaign".to_string(),
        });
        let json = value.to_ro_crate_jsonld();
        assert!(json.contains("collection_location"));
        assert!(json.contains("not recorded in source campaign"));
    }

    #[test]
    fn jsonld_export_contains_pinned_ro_crate_context_prov_and_qudt_terms() {
        let json = profile().to_ro_crate_jsonld();
        assert!(json.starts_with("{\"@context\":["));
        assert!(json.contains(RO_CRATE_CONTEXT));
        assert!(json.contains(RO_CRATE_VERSION));
        assert!(json.contains("prov:wasGeneratedBy"));
        assert!(json.contains("qudt:unit"));
        assert!(json.ends_with("]}"));
    }

    #[test]
    fn json_escape_handles_quotes_backslashes_and_control_characters() {
        let escaped = json_escape("optic \"A\"\\ test\nline");
        assert_eq!(escaped, "optic \\\"A\\\"\\\\ test\\nline");
    }

    #[test]
    fn duplicate_node_ids_are_rejected() {
        let mut value = profile();
        value.nodes.push(node(
            "urn:sample:one",
            "Duplicate",
            NodeClass::Entity(EntityKind::Sample),
        ));
        assert!(value
            .validate(ConformanceLevel::P0PackageValid)
            .findings
            .contains(&ProfileFinding::DuplicateNode("urn:sample:one".to_string())));
    }
}
