// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-qualified evidence contract for connectome edge tables.
//!
//! This crate preserves raw endpoint IDs, annotation states and source coverage.
//! It deliberately does not interpret missing annotation as absent connectivity.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Maximum accepted number of edge records in one in-memory evidence object.
pub const MAX_EDGE_RECORDS: usize = 5_000_000;

/// A validated SHA-256 digest, encoded as 64 lowercase hexadecimal characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    /// Parses a 64-character hexadecimal SHA-256 digest.
    pub fn parse(value: &str) -> Result<Self, EvidenceError> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(EvidenceError::MalformedSha256);
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    /// Returns the canonical lowercase hexadecimal representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Declared completeness of the frozen edge table, bounded to a named scope.
///
/// Completeness is about the supplied source snapshot and scope. It never proves
/// biological impossibility outside that scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeCoverage {
    /// The edge table is declared complete within this exact snapshot scope.
    CompleteWithinSnapshot { scope: String },
    /// The edge table is known to be partial within this scope.
    Partial { scope: String, reason: String },
}

impl EdgeCoverage {
    fn validate(&self) -> Result<(), EvidenceError> {
        match self {
            Self::CompleteWithinSnapshot { scope } if scope.trim().is_empty() => {
                Err(EvidenceError::InvalidCoverageField("scope"))
            }
            Self::Partial { scope, reason }
                if scope.trim().is_empty() || reason.trim().is_empty() =>
            {
                Err(EvidenceError::InvalidCoverageField("scope or reason"))
            }
            _ => Ok(()),
        }
    }
}

/// Immutable identity and provenance supplied by the source-format adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceManifest {
    /// Dataset name or accession, e.g. BANC.
    pub dataset_id: String,
    /// Public release or frozen source version.
    pub release_id: String,
    /// Optional CAVE/materialization ID or timestamp.
    pub materialization_id: Option<String>,
    /// Revision/identity of the exact synapse or edge table.
    pub edge_table_revision: String,
    /// Exact archive filename whose bytes were hashed.
    pub archive_file_name: String,
    /// SHA-256 of the acquired archive bytes, not a catalogue MD5.
    pub archive_sha256: Sha256Digest,
    /// Completeness statement for the exact edge-table scope.
    pub edge_coverage: EdgeCoverage,
}

impl SourceManifest {
    /// Validates required source identity and coverage metadata.
    pub fn validate(&self) -> Result<(), EvidenceError> {
        for (field, value) in [
            ("dataset_id", self.dataset_id.as_str()),
            ("release_id", self.release_id.as_str()),
            ("edge_table_revision", self.edge_table_revision.as_str()),
            ("archive_file_name", self.archive_file_name.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(EvidenceError::InvalidSourceField(field));
            }
        }
        if self
            .materialization_id
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(EvidenceError::InvalidSourceField("materialization_id"));
        }
        self.edge_coverage.validate()
    }
}

/// Why an endpoint's annotation is not fully resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnnotationGapReason {
    /// No canonical/root identity could be established.
    RootIdentityUnavailable,
    /// The identity mapping is ambiguous.
    AmbiguousMapping,
    /// The endpoint is outside the published annotation scope.
    OutsideAnnotationScope,
    /// The endpoint has an identity but no cell-type label.
    NotCellTyped,
    /// The source did not establish proofreading.
    NotProofread,
    /// A source-specific reason not represented above.
    Other,
}

/// Annotation state for a source endpoint.
///
/// Proofreading is independent of cell-type annotation. An identified but
/// untyped neuron remains an identified endpoint and is not silently discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointAnnotation {
    /// Canonical neuron identity and cell type are available.
    CellTyped {
        canonical_neuron_id: String,
        proofread: bool,
        cell_type: String,
    },
    /// Canonical identity is available, but a cell type is not.
    IdentifiedUnclassified {
        canonical_neuron_id: String,
        proofread: bool,
        reason: AnnotationGapReason,
    },
    /// The raw endpoint is retained, but canonical identity is unresolved.
    Unresolved { reason: AnnotationGapReason },
}

impl EndpointAnnotation {
    /// Returns the canonical/root neuron ID when available.
    #[must_use]
    pub fn canonical_neuron_id(&self) -> Option<&str> {
        match self {
            Self::CellTyped {
                canonical_neuron_id,
                ..
            }
            | Self::IdentifiedUnclassified {
                canonical_neuron_id,
                ..
            } => Some(canonical_neuron_id),
            Self::Unresolved { .. } => None,
        }
    }

    /// Returns the cell-type label only when it is explicitly available.
    #[must_use]
    pub fn cell_type(&self) -> Option<&str> {
        match self {
            Self::CellTyped { cell_type, .. } => Some(cell_type),
            Self::IdentifiedUnclassified { .. } | Self::Unresolved { .. } => None,
        }
    }

    /// Returns the independent proofreading flag, when an identity is known.
    #[must_use]
    pub fn is_proofread(&self) -> bool {
        match self {
            Self::CellTyped { proofread, .. } | Self::IdentifiedUnclassified { proofread, .. } => {
                *proofread
            }
            Self::Unresolved { .. } => false,
        }
    }

    /// True when a canonical/root neuron identity is available.
    #[must_use]
    pub fn is_identified(&self) -> bool {
        self.canonical_neuron_id().is_some()
    }

    fn validate(&self) -> bool {
        match self {
            Self::CellTyped {
                canonical_neuron_id,
                cell_type,
                ..
            } => !canonical_neuron_id.trim().is_empty() && !cell_type.trim().is_empty(),
            Self::IdentifiedUnclassified {
                canonical_neuron_id,
                ..
            } => !canonical_neuron_id.trim().is_empty(),
            Self::Unresolved { .. } => true,
        }
    }
}

/// One endpoint of an observed edge, retaining the source's raw ID verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointEvidence {
    /// Snapshot-local endpoint identity from the raw source edge table.
    pub raw_endpoint_id: String,
    /// Independently qualified annotation state.
    pub annotation: EndpointAnnotation,
}

impl EndpointEvidence {
    /// Creates a validated endpoint record.
    pub fn new(
        raw_endpoint_id: impl Into<String>,
        annotation: EndpointAnnotation,
    ) -> Result<Self, EvidenceError> {
        let endpoint = Self {
            raw_endpoint_id: raw_endpoint_id.into(),
            annotation,
        };
        endpoint.validate(EndpointSide::Presynaptic)?;
        Ok(endpoint)
    }

    fn validate(&self, side: EndpointSide) -> Result<(), EvidenceError> {
        if self.raw_endpoint_id.trim().is_empty() {
            return Err(EvidenceError::InvalidRawEndpointId(side));
        }
        if !self.annotation.validate() {
            return Err(EvidenceError::InvalidAnnotation(side));
        }
        Ok(())
    }
}

/// Which side of an edge failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointSide {
    Presynaptic,
    Postsynaptic,
}

/// One observed, directed source edge record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeRecord {
    /// Stable row/edge identity within the exact source snapshot.
    pub edge_record_id: String,
    pub presynaptic: EndpointEvidence,
    pub postsynaptic: EndpointEvidence,
    /// Positive integer synapse count or source-declared edge weight unit.
    pub synapse_count: u64,
}

impl EdgeRecord {
    /// Creates one directed edge record.
    pub fn new(
        edge_record_id: impl Into<String>,
        presynaptic: EndpointEvidence,
        postsynaptic: EndpointEvidence,
        synapse_count: u64,
    ) -> Result<Self, EvidenceError> {
        let edge = Self {
            edge_record_id: edge_record_id.into(),
            presynaptic,
            postsynaptic,
            synapse_count,
        };
        edge.validate()?;
        Ok(edge)
    }

    fn validate(&self) -> Result<(), EvidenceError> {
        if self.edge_record_id.trim().is_empty() {
            return Err(EvidenceError::InvalidEdgeRecordId);
        }
        if self.synapse_count == 0 {
            return Err(EvidenceError::ZeroSynapseCount);
        }
        self.presynaptic.validate(EndpointSide::Presynaptic)?;
        self.postsynaptic.validate(EndpointSide::Postsynaptic)?;
        Ok(())
    }
}

/// Explicit validation failures. No invalid state is silently coerced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError {
    InvalidSourceField(&'static str),
    InvalidCoverageField(&'static str),
    MalformedSha256,
    TooManyEdgeRecords,
    InvalidEdgeRecordId,
    DuplicateEdgeRecordId,
    ZeroSynapseCount,
    InvalidRawEndpointId(EndpointSide),
    InvalidAnnotation(EndpointSide),
    ConflictingEndpointAnnotation,
    InvalidLookupEndpoint,
    SynapseCountOverflow,
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSourceField(field) => {
                write!(formatter, "source manifest field {field} is empty")
            }
            Self::InvalidCoverageField(field) => {
                write!(formatter, "edge coverage field {field} is empty")
            }
            Self::MalformedSha256 => {
                write!(
                    formatter,
                    "archive SHA-256 must be exactly 64 hexadecimal characters"
                )
            }
            Self::TooManyEdgeRecords => {
                write!(
                    formatter,
                    "edge record count exceeds the configured safety limit"
                )
            }
            Self::InvalidEdgeRecordId => write!(formatter, "edge record ID is empty"),
            Self::DuplicateEdgeRecordId => write!(formatter, "duplicate edge record ID"),
            Self::ZeroSynapseCount => write!(formatter, "edge synapse count must be positive"),
            Self::InvalidRawEndpointId(side) => {
                write!(formatter, "{side:?} raw endpoint ID is empty")
            }
            Self::InvalidAnnotation(side) => {
                write!(formatter, "{side:?} endpoint annotation is malformed")
            }
            Self::ConflictingEndpointAnnotation => {
                write!(
                    formatter,
                    "one raw endpoint has conflicting annotations in this snapshot"
                )
            }
            Self::InvalidLookupEndpoint => {
                write!(formatter, "lookup endpoint IDs must be non-empty")
            }
            Self::SynapseCountOverflow => {
                write!(formatter, "aggregate synapse count exceeds u64")
            }
        }
    }
}

impl std::error::Error for EvidenceError {}

/// Counts for one endpoint side. Denominators are edge-record endpoint occurrences.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EndpointCoverageCount {
    pub total_occurrences: usize,
    pub identified: usize,
    pub proofread: usize,
    pub cell_typed: usize,
}

impl EndpointCoverageCount {
    /// Fraction of endpoint occurrences with canonical identities.
    #[must_use]
    pub fn identified_fraction(self) -> CoverageFraction {
        CoverageFraction {
            numerator: self.identified,
            denominator: self.total_occurrences,
        }
    }

    /// Fraction of endpoint occurrences marked proofread.
    #[must_use]
    pub fn proofread_fraction(self) -> CoverageFraction {
        CoverageFraction {
            numerator: self.proofread,
            denominator: self.total_occurrences,
        }
    }

    /// Fraction of endpoint occurrences with cell-type labels.
    #[must_use]
    pub fn cell_typed_fraction(self) -> CoverageFraction {
        CoverageFraction {
            numerator: self.cell_typed,
            denominator: self.total_occurrences,
        }
    }
}

/// A fraction that never hides its denominator or converts an empty denominator to zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageFraction {
    pub numerator: usize,
    pub denominator: usize,
}

impl CoverageFraction {
    /// Returns None for a zero denominator; otherwise returns numerator / denominator.
    #[must_use]
    pub fn as_f64(self) -> Option<f64> {
        if self.denominator == 0 {
            None
        } else {
            Some(self.numerator as f64 / self.denominator as f64)
        }
    }
}

/// Endpoint- and edge-level annotation coverage for the supplied edge records.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EvidenceCoverageSummary {
    pub edge_records: usize,
    pub total_synapse_count: u64,
    pub presynaptic: EndpointCoverageCount,
    pub postsynaptic: EndpointCoverageCount,
    pub both_endpoints_identified: usize,
    pub both_endpoints_proofread: usize,
    pub both_endpoints_cell_typed: usize,
}

impl EvidenceCoverageSummary {
    /// Fraction of edge records whose two endpoints both have canonical IDs.
    #[must_use]
    pub fn both_identified_fraction(self) -> CoverageFraction {
        CoverageFraction {
            numerator: self.both_endpoints_identified,
            denominator: self.edge_records,
        }
    }

    /// Fraction of edge records whose two endpoints are both proofread.
    #[must_use]
    pub fn both_proofread_fraction(self) -> CoverageFraction {
        CoverageFraction {
            numerator: self.both_endpoints_proofread,
            denominator: self.edge_records,
        }
    }

    /// Fraction of edge records whose two endpoints both have cell types.
    #[must_use]
    pub fn both_cell_typed_fraction(self) -> CoverageFraction {
        CoverageFraction {
            numerator: self.both_endpoints_cell_typed,
            denominator: self.edge_records,
        }
    }
}

/// Validated edge records and the exact source manifest that qualifies them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectomeEvidence {
    manifest: SourceManifest,
    edges: Vec<EdgeRecord>,
    coverage: EvidenceCoverageSummary,
}

impl ConnectomeEvidence {
    /// Validates source identity, edge rows and annotation consistency.
    pub fn new(manifest: SourceManifest, edges: Vec<EdgeRecord>) -> Result<Self, EvidenceError> {
        manifest.validate()?;
        if edges.len() > MAX_EDGE_RECORDS {
            return Err(EvidenceError::TooManyEdgeRecords);
        }

        let mut edge_ids = BTreeSet::new();
        let mut endpoint_annotations = BTreeMap::<String, EndpointAnnotation>::new();
        let mut coverage = EvidenceCoverageSummary {
            edge_records: edges.len(),
            ..EvidenceCoverageSummary::default()
        };

        for edge in &edges {
            edge.validate()?;
            if !edge_ids.insert(edge.edge_record_id.as_str()) {
                return Err(EvidenceError::DuplicateEdgeRecordId);
            }

            for (endpoint, side) in [
                (&edge.presynaptic, EndpointSide::Presynaptic),
                (&edge.postsynaptic, EndpointSide::Postsynaptic),
            ] {
                let _ = side;
                match endpoint_annotations.get(&endpoint.raw_endpoint_id) {
                    Some(existing) if existing != &endpoint.annotation => {
                        return Err(EvidenceError::ConflictingEndpointAnnotation);
                    }
                    Some(_) => {}
                    None => {
                        endpoint_annotations.insert(
                            endpoint.raw_endpoint_id.clone(),
                            endpoint.annotation.clone(),
                        );
                    }
                }
            }

            coverage.total_synapse_count = coverage
                .total_synapse_count
                .checked_add(edge.synapse_count)
                .ok_or(EvidenceError::SynapseCountOverflow)?;

            update_endpoint_coverage(&mut coverage.presynaptic, &edge.presynaptic.annotation);
            update_endpoint_coverage(&mut coverage.postsynaptic, &edge.postsynaptic.annotation);

            let pre = &edge.presynaptic.annotation;
            let post = &edge.postsynaptic.annotation;
            if pre.is_identified() && post.is_identified() {
                coverage.both_endpoints_identified += 1;
            }
            if pre.is_proofread() && post.is_proofread() {
                coverage.both_endpoints_proofread += 1;
            }
            if pre.cell_type().is_some() && post.cell_type().is_some() {
                coverage.both_endpoints_cell_typed += 1;
            }
        }

        Ok(Self {
            manifest,
            edges,
            coverage,
        })
    }

    /// Returns the immutable source manifest.
    #[must_use]
    pub fn manifest(&self) -> &SourceManifest {
        &self.manifest
    }

    /// Returns every validated source edge record, including untyped endpoints.
    #[must_use]
    pub fn edges(&self) -> &[EdgeRecord] {
        &self.edges
    }

    /// Returns coverage with explicit pre/post denominators and edge-record counts.
    #[must_use]
    pub fn coverage_summary(&self) -> EvidenceCoverageSummary {
        self.coverage
    }

    /// Projects a typed graph with mutually exclusive exclusion accounting.
    #[must_use]
    pub fn project_typed(&self, policy: TypedProjectionPolicy) -> TypedProjection {
        let mut projected_edges = Vec::new();
        let mut exclusions = ProjectionExclusionCounts::default();

        for edge in &self.edges {
            let pre_id = edge.presynaptic.annotation.canonical_neuron_id();
            let post_id = edge.postsynaptic.annotation.canonical_neuron_id();
            let pre_type = edge.presynaptic.annotation.cell_type();
            let post_type = edge.postsynaptic.annotation.cell_type();

            if pre_id.is_none() || post_id.is_none() {
                if pre_id.is_none() && post_id.is_none() {
                    exclusions.missing_both_endpoint_identities += 1;
                } else if pre_id.is_none() {
                    exclusions.missing_presynaptic_identity += 1;
                } else {
                    exclusions.missing_postsynaptic_identity += 1;
                }
                continue;
            }

            if pre_type.is_none() || post_type.is_none() {
                if pre_type.is_none() && post_type.is_none() {
                    exclusions.missing_both_cell_types += 1;
                } else if pre_type.is_none() {
                    exclusions.missing_presynaptic_cell_type += 1;
                } else {
                    exclusions.missing_postsynaptic_cell_type += 1;
                }
                continue;
            }

            if policy == TypedProjectionPolicy::RequireCellTypeAndProofread {
                let pre_proofread = edge.presynaptic.annotation.is_proofread();
                let post_proofread = edge.postsynaptic.annotation.is_proofread();
                if !pre_proofread || !post_proofread {
                    if !pre_proofread && !post_proofread {
                        exclusions.both_endpoints_not_proofread += 1;
                    } else if !pre_proofread {
                        exclusions.presynaptic_not_proofread += 1;
                    } else {
                        exclusions.postsynaptic_not_proofread += 1;
                    }
                    continue;
                }
            }

            if let (Some(pre_id), Some(post_id), Some(pre_type), Some(post_type)) =
                (pre_id, post_id, pre_type, post_type)
            {
                projected_edges.push(TypedConnectomeEdge {
                    edge_record_id: edge.edge_record_id.clone(),
                    presynaptic_neuron_id: pre_id.to_owned(),
                    postsynaptic_neuron_id: post_id.to_owned(),
                    presynaptic_cell_type: pre_type.to_owned(),
                    postsynaptic_cell_type: post_type.to_owned(),
                    synapse_count: edge.synapse_count,
                });
            }
        }

        TypedProjection {
            policy,
            source_edge_records: self.edges.len(),
            edges: projected_edges,
            exclusions,
        }
    }

    /// Looks up a directed pair using source raw IDs, not projected array positions.
    ///
    /// An absent pair is unknown under partial coverage. A complete-snapshot
    /// result is scoped to the manifest and is not a biological absence claim.
    pub fn lookup_raw_edge(
        &self,
        presynaptic_raw_id: &str,
        postsynaptic_raw_id: &str,
    ) -> Result<EdgeLookup, EvidenceError> {
        if presynaptic_raw_id.trim().is_empty() || postsynaptic_raw_id.trim().is_empty() {
            return Err(EvidenceError::InvalidLookupEndpoint);
        }

        let mut records = 0_usize;
        let mut synapse_count = 0_u64;
        for edge in &self.edges {
            if edge.presynaptic.raw_endpoint_id == presynaptic_raw_id
                && edge.postsynaptic.raw_endpoint_id == postsynaptic_raw_id
            {
                records += 1;
                synapse_count = synapse_count
                    .checked_add(edge.synapse_count)
                    .ok_or(EvidenceError::SynapseCountOverflow)?;
            }
        }

        if records > 0 {
            return Ok(EdgeLookup::Observed {
                edge_records: records,
                synapse_count,
            });
        }

        Ok(match &self.manifest.edge_coverage {
            EdgeCoverage::CompleteWithinSnapshot { scope } => {
                EdgeLookup::NotObservedInCompleteSnapshotScope {
                    scope: scope.clone(),
                }
            }
            EdgeCoverage::Partial { scope, reason } => EdgeLookup::UnknownUnderPartialCoverage {
                scope: scope.clone(),
                reason: reason.clone(),
            },
        })
    }
}

fn update_endpoint_coverage(counts: &mut EndpointCoverageCount, annotation: &EndpointAnnotation) {
    counts.total_occurrences += 1;
    if annotation.is_identified() {
        counts.identified += 1;
    }
    if annotation.is_proofread() {
        counts.proofread += 1;
    }
    if annotation.cell_type().is_some() {
        counts.cell_typed += 1;
    }
}

/// How strict a typed projection should be about endpoint proofreading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedProjectionPolicy {
    /// Both endpoint identities and cell-type labels are required.
    RequireCellType,
    /// Cell-type labels and affirmative proofreading for both endpoints are required.
    RequireCellTypeAndProofread,
}

/// A projected edge with both endpoint identities and cell types resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedConnectomeEdge {
    pub edge_record_id: String,
    pub presynaptic_neuron_id: String,
    pub postsynaptic_neuron_id: String,
    pub presynaptic_cell_type: String,
    pub postsynaptic_cell_type: String,
    pub synapse_count: u64,
}

/// Mutually exclusive reasons why source edges were excluded from a typed projection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectionExclusionCounts {
    pub missing_both_endpoint_identities: usize,
    pub missing_presynaptic_identity: usize,
    pub missing_postsynaptic_identity: usize,
    pub missing_both_cell_types: usize,
    pub missing_presynaptic_cell_type: usize,
    pub missing_postsynaptic_cell_type: usize,
    pub both_endpoints_not_proofread: usize,
    pub presynaptic_not_proofread: usize,
    pub postsynaptic_not_proofread: usize,
}

impl ProjectionExclusionCounts {
    /// Number of source edge records excluded, with no double-counting.
    #[must_use]
    pub fn total(self) -> usize {
        self.missing_both_endpoint_identities
            + self.missing_presynaptic_identity
            + self.missing_postsynaptic_identity
            + self.missing_both_cell_types
            + self.missing_presynaptic_cell_type
            + self.missing_postsynaptic_cell_type
            + self.both_endpoints_not_proofread
            + self.presynaptic_not_proofread
            + self.postsynaptic_not_proofread
    }
}

/// Auditable result of lossy typed-graph projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedProjection {
    pub policy: TypedProjectionPolicy,
    pub source_edge_records: usize,
    pub edges: Vec<TypedConnectomeEdge>,
    pub exclusions: ProjectionExclusionCounts,
}

impl TypedProjection {
    /// Confirms that every source edge was included or assigned one exclusion reason.
    #[must_use]
    pub fn all_source_edges_accounted_for(&self) -> bool {
        self.edges.len() + self.exclusions.total() == self.source_edge_records
    }
}

/// Result of looking up a directed pair in the source edge table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeLookup {
    /// One or more matching edge records were observed.
    Observed {
        edge_records: usize,
        synapse_count: u64,
    },
    /// No matching row exists in a declared complete snapshot scope.
    NotObservedInCompleteSnapshotScope { scope: String },
    /// No matching row exists, but the edge table is known to be partial.
    UnknownUnderPartialCoverage { scope: String, reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> Sha256Digest {
        Sha256Digest::parse(&"a".repeat(64)).expect("valid synthetic digest")
    }

    fn manifest(edge_coverage: EdgeCoverage) -> SourceManifest {
        SourceManifest {
            dataset_id: "SYNTHETIC-FLY".to_owned(),
            release_id: "fixture-v1".to_owned(),
            materialization_id: Some("snapshot-001".to_owned()),
            edge_table_revision: "edges-v1".to_owned(),
            archive_file_name: "synthetic-edges.csv".to_owned(),
            archive_sha256: digest(),
            edge_coverage,
        }
    }

    fn typed(raw: &str, canonical: &str, cell_type: &str, proofread: bool) -> EndpointEvidence {
        EndpointEvidence::new(
            raw,
            EndpointAnnotation::CellTyped {
                canonical_neuron_id: canonical.to_owned(),
                proofread,
                cell_type: cell_type.to_owned(),
            },
        )
        .expect("valid typed endpoint")
    }

    fn unclassified(raw: &str, canonical: &str, proofread: bool) -> EndpointEvidence {
        EndpointEvidence::new(
            raw,
            EndpointAnnotation::IdentifiedUnclassified {
                canonical_neuron_id: canonical.to_owned(),
                proofread,
                reason: AnnotationGapReason::NotCellTyped,
            },
        )
        .expect("valid identified endpoint")
    }

    fn unresolved(raw: &str) -> EndpointEvidence {
        EndpointEvidence::new(
            raw,
            EndpointAnnotation::Unresolved {
                reason: AnnotationGapReason::RootIdentityUnavailable,
            },
        )
        .expect("valid unresolved endpoint")
    }

    fn edge(id: &str, pre: EndpointEvidence, post: EndpointEvidence, weight: u64) -> EdgeRecord {
        EdgeRecord::new(id, pre, post, weight).expect("valid synthetic edge")
    }

    #[test]
    fn sha256_parser_is_strict_and_canonicalizes_case() {
        let upper = "A".repeat(64);
        assert_eq!(
            Sha256Digest::parse(&upper)
                .expect("valid upper hex")
                .as_str(),
            "a".repeat(64)
        );
        assert_eq!(
            Sha256Digest::parse("abc"),
            Err(EvidenceError::MalformedSha256)
        );
        assert_eq!(
            Sha256Digest::parse(&format!("{}g", "a".repeat(63))),
            Err(EvidenceError::MalformedSha256)
        );
    }

    #[test]
    fn manifest_rejects_missing_source_identity() {
        let mut source = manifest(EdgeCoverage::Partial {
            scope: "whole-edge-table".to_owned(),
            reason: "synthetic partial fixture".to_owned(),
        });
        source.dataset_id.clear();
        assert_eq!(
            source.validate(),
            Err(EvidenceError::InvalidSourceField("dataset_id"))
        );
    }

    #[test]
    fn manifest_rejects_empty_partial_coverage_reason() {
        let source = manifest(EdgeCoverage::Partial {
            scope: "whole-edge-table".to_owned(),
            reason: " ".to_owned(),
        });
        assert_eq!(
            source.validate(),
            Err(EvidenceError::InvalidCoverageField("scope or reason"))
        );
    }

    #[test]
    fn zero_weight_and_empty_endpoint_ids_fail_closed() {
        let pre = typed("raw-pre", "root-pre", "sensory", true);
        let post = typed("raw-post", "root-post", "motor", true);
        assert_eq!(
            EdgeRecord::new("e0", pre.clone(), post.clone(), 0),
            Err(EvidenceError::ZeroSynapseCount)
        );
        assert_eq!(
            EndpointEvidence::new(
                " ",
                EndpointAnnotation::Unresolved {
                    reason: AnnotationGapReason::RootIdentityUnavailable,
                }
            ),
            Err(EvidenceError::InvalidRawEndpointId(
                EndpointSide::Presynaptic
            ))
        );
    }

    #[test]
    fn duplicate_edge_record_ids_are_rejected() {
        let source = manifest(EdgeCoverage::Partial {
            scope: "all-edges".to_owned(),
            reason: "fixture".to_owned(),
        });
        let edges = vec![
            edge(
                "same",
                typed("a", "A", "sensory", true),
                typed("b", "B", "motor", true),
                1,
            ),
            edge(
                "same",
                typed("c", "C", "sensory", true),
                typed("d", "D", "motor", true),
                2,
            ),
        ];
        assert_eq!(
            ConnectomeEvidence::new(source, edges),
            Err(EvidenceError::DuplicateEdgeRecordId)
        );
    }

    #[test]
    fn repeated_raw_endpoint_with_conflicting_annotation_is_rejected() {
        let source = manifest(EdgeCoverage::Partial {
            scope: "all-edges".to_owned(),
            reason: "fixture".to_owned(),
        });
        let edges = vec![
            edge(
                "e1",
                typed("shared", "root-1", "sensory", true),
                typed("b", "B", "motor", true),
                1,
            ),
            edge(
                "e2",
                typed("shared", "root-1", "interneuron", true),
                typed("c", "C", "motor", true),
                1,
            ),
        ];
        assert_eq!(
            ConnectomeEvidence::new(source, edges),
            Err(EvidenceError::ConflictingEndpointAnnotation)
        );
    }

    #[test]
    fn coverage_keeps_presynaptic_and_postsynaptic_denominators_separate() {
        let source = manifest(EdgeCoverage::Partial {
            scope: "all-edges".to_owned(),
            reason: "synthetic coverage fixture".to_owned(),
        });
        let graph = ConnectomeEvidence::new(
            source,
            vec![
                edge(
                    "e1",
                    typed("p1", "P1", "sensory", true),
                    typed("q1", "Q1", "motor", true),
                    7,
                ),
                edge("e2", unclassified("p2", "P2", true), unresolved("q2"), 3),
                edge("e3", unresolved("p3"), unclassified("q3", "Q3", false), 2),
            ],
        )
        .expect("valid evidence graph");

        let summary = graph.coverage_summary();
        assert_eq!(summary.edge_records, 3);
        assert_eq!(summary.total_synapse_count, 12);
        assert_eq!(summary.presynaptic.total_occurrences, 3);
        assert_eq!(summary.presynaptic.identified, 2);
        assert_eq!(summary.presynaptic.proofread, 2);
        assert_eq!(summary.presynaptic.cell_typed, 1);
        assert_eq!(summary.postsynaptic.total_occurrences, 3);
        assert_eq!(summary.postsynaptic.identified, 2);
        assert_eq!(summary.postsynaptic.proofread, 1);
        assert_eq!(summary.postsynaptic.cell_typed, 1);
        assert_eq!(summary.both_endpoints_identified, 1);
        assert_eq!(summary.both_endpoints_proofread, 1);
        assert_eq!(summary.both_endpoints_cell_typed, 1);
        assert_eq!(
            summary.presynaptic.cell_typed_fraction(),
            CoverageFraction {
                numerator: 1,
                denominator: 3
            }
        );
        assert_eq!(summary.both_cell_typed_fraction().as_f64(), Some(1.0 / 3.0));
    }

    #[test]
    fn typed_projection_reports_every_excluded_edge() {
        let source = manifest(EdgeCoverage::Partial {
            scope: "all-edges".to_owned(),
            reason: "synthetic coverage fixture".to_owned(),
        });
        let graph = ConnectomeEvidence::new(
            source,
            vec![
                edge(
                    "e1",
                    typed("p1", "P1", "sensory", true),
                    typed("q1", "Q1", "motor", true),
                    2,
                ),
                edge("e2", unclassified("p2", "P2", true), unresolved("q2"), 1),
                edge("e3", unresolved("p3"), unclassified("q3", "Q3", false), 1),
            ],
        )
        .expect("valid evidence graph");

        let projection = graph.project_typed(TypedProjectionPolicy::RequireCellType);
        assert_eq!(projection.edges.len(), 1);
        assert_eq!(projection.exclusions.missing_postsynaptic_identity, 1);
        assert_eq!(projection.exclusions.missing_presynaptic_identity, 1);
        assert_eq!(projection.exclusions.total(), 2);
        assert!(projection.all_source_edges_accounted_for());
        assert_eq!(projection.source_edge_records, 3);
    }

    #[test]
    fn proofread_requirement_is_independent_of_cell_type_requirement() {
        let source = manifest(EdgeCoverage::Partial {
            scope: "all-edges".to_owned(),
            reason: "fixture".to_owned(),
        });
        let graph = ConnectomeEvidence::new(
            source,
            vec![edge(
                "e1",
                typed("p1", "P1", "sensory", false),
                typed("q1", "Q1", "motor", true),
                4,
            )],
        )
        .expect("valid evidence graph");

        let cell_typed = graph.project_typed(TypedProjectionPolicy::RequireCellType);
        let proofread = graph.project_typed(TypedProjectionPolicy::RequireCellTypeAndProofread);
        assert_eq!(cell_typed.edges.len(), 1);
        assert_eq!(proofread.edges.len(), 0);
        assert_eq!(proofread.exclusions.presynaptic_not_proofread, 1);
        assert!(proofread.all_source_edges_accounted_for());
    }

    #[test]
    fn observed_edge_is_returned_even_under_partial_coverage() {
        let source = manifest(EdgeCoverage::Partial {
            scope: "edge-table".to_owned(),
            reason: "only a bounded subset was exported".to_owned(),
        });
        let graph = ConnectomeEvidence::new(
            source,
            vec![
                edge(
                    "e1",
                    typed("raw-a", "A", "sensory", true),
                    typed("raw-b", "B", "motor", true),
                    4,
                ),
                edge(
                    "e2",
                    typed("raw-a", "A", "sensory", true),
                    typed("raw-b", "B", "motor", true),
                    3,
                ),
            ],
        )
        .expect("valid evidence graph");

        assert_eq!(
            graph.lookup_raw_edge("raw-a", "raw-b"),
            Ok(EdgeLookup::Observed {
                edge_records: 2,
                synapse_count: 7
            })
        );
    }

    #[test]
    fn missing_pair_is_unknown_under_partial_coverage() {
        let source = manifest(EdgeCoverage::Partial {
            scope: "edge-table-v1".to_owned(),
            reason: "partial archive export".to_owned(),
        });
        let graph = ConnectomeEvidence::new(source, vec![]).expect("empty partial table is valid");

        assert_eq!(
            graph.lookup_raw_edge("raw-a", "raw-b"),
            Ok(EdgeLookup::UnknownUnderPartialCoverage {
                scope: "edge-table-v1".to_owned(),
                reason: "partial archive export".to_owned()
            })
        );
    }

    #[test]
    fn missing_pair_in_complete_snapshot_is_scoped_not_biological_absence() {
        let source = manifest(EdgeCoverage::CompleteWithinSnapshot {
            scope: "snapshot-001/synapses-v2".to_owned(),
        });
        let graph = ConnectomeEvidence::new(source, vec![]).expect("empty complete table is valid");

        assert_eq!(
            graph.lookup_raw_edge("raw-a", "raw-b"),
            Ok(EdgeLookup::NotObservedInCompleteSnapshotScope {
                scope: "snapshot-001/synapses-v2".to_owned()
            })
        );
    }

    #[test]
    fn lookup_rejects_empty_raw_endpoint_ids() {
        let source = manifest(EdgeCoverage::Partial {
            scope: "scope".to_owned(),
            reason: "partial".to_owned(),
        });
        let graph = ConnectomeEvidence::new(source, vec![]).expect("valid empty graph");
        assert_eq!(
            graph.lookup_raw_edge("", "target"),
            Err(EvidenceError::InvalidLookupEndpoint)
        );
    }

    #[test]
    fn endpoint_annotation_accessors_do_not_infer_missing_metadata() {
        let identified = unclassified("raw", "root", true);
        assert_eq!(identified.annotation.canonical_neuron_id(), Some("root"));
        assert!(identified.annotation.is_identified());
        assert!(identified.annotation.is_proofread());
        assert_eq!(identified.annotation.cell_type(), None);

        let unknown = unresolved("unknown-raw");
        assert_eq!(unknown.annotation.canonical_neuron_id(), None);
        assert!(!unknown.annotation.is_identified());
        assert!(!unknown.annotation.is_proofread());
        assert_eq!(unknown.annotation.cell_type(), None);
    }

    #[test]
    fn empty_coverage_denominator_is_not_reported_as_zero_percent() {
        let summary = EvidenceCoverageSummary::default();
        assert_eq!(summary.presynaptic.identified_fraction().as_f64(), None);
        assert_eq!(summary.both_identified_fraction().as_f64(), None);
    }
}
