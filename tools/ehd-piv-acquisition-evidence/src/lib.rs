//! Content verification and dependence-aware partitioning for EHD-PIV evidence.
//!
//! This crate establishes byte identity and structural split consistency. It
//! does not authenticate an issuer, prove custody, establish physical truth,
//! validate instrument calibration, or prove that a reference is independent.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MANIFEST_SCHEMA_VERSION: &str = "ehd_piv_acquisition_manifest_v0_1";
pub const CANONICALIZATION_VERSION: &str = "serde_json_v1_sorted_inventory";
const HASH_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRecord {
    pub asset_id: String,
    pub role: String,
    /// UTF-8 path relative to the evidence root; forward slashes only.
    pub relative_path: String,
    pub byte_length: u64,
    pub media_type: String,
    /// Lowercase SHA-256 of the exact original bytes.
    pub sha256: String,
    pub acquisition_id: String,
    /// Immutable IDs for all declared dependence groups affecting this asset.
    pub dependence_group_ids: Vec<String>,
    pub reference_evidence_id: String,
    pub clock_domain: String,
    pub captured_at_utc: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedCauseEdge {
    pub acquisition_id: String,
    pub shared_cause_id: String,
    pub shared_cause_kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionManifest {
    pub schema_version: String,
    pub canonicalization_version: String,
    pub manifest_id: String,
    pub source_release_id: String,
    pub protocol_fingerprint: String,
    pub reference_assessment_id: String,
    pub observer_state_id: String,
    pub split_policy_digest: String,
    /// SHA-256 over the canonical sorted dependency-edge array.
    pub dependency_graph_sha256: String,
    pub assets: Vec<AssetRecord>,
    pub dependency_edges: Vec<SharedCauseEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceError {
    EmptyField { field: &'static str },
    UnsupportedSchemaVersion,
    InvalidSha256 { field: &'static str, value: String },
    EmptyAssetInventory,
    DuplicateAssetId(String),
    DuplicateRelativePath(String),
    DuplicateDependencyEdge(String),
    InconsistentSharedCauseKind { shared_cause_id: String, first: String, second: String },
    UnknownDependencyAcquisition(String),
    IncompleteDependenceComponent {
        component_id: String,
        missing_acquisition_id: String,
    },
    InvalidRelativePath(String),
    SymlinkNotAllowed(String),
    PathEscapesEvidenceRoot(String),
    ExpectedFile(String),
    ExpectedDirectory(String),
    SourceRootInvalid(String),
    ManifestDigestMismatch { expected: String, actual: String },
    AssetLengthMismatch { asset_id: String, expected: u64, actual: u64 },
    AssetDigestMismatch { asset_id: String, expected: String, actual: String },
    LengthOverflow,
    Io { path: String, message: String },
    Serialization(String),
    TooFewCohorts,
    EmptyCohort(&'static str),
    DuplicateCohortId(String),
    DuplicateCohortRole(String),
    DuplicateCohortMember { cohort_id: String, identity: String },
    CohortOverlap {
        left_cohort: String,
        right_cohort: String,
        dimension: &'static str,
        identity: String,
    },
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField { field } => write!(f, "{field} must be non-empty"),
            Self::UnsupportedSchemaVersion => write!(f, "unsupported manifest or canonicalization version"),
            Self::InvalidSha256 { field, value } => write!(f, "{field} is not lowercase SHA-256 hex: {value}"),
            Self::EmptyAssetInventory => write!(f, "manifest asset inventory is empty"),
            Self::DuplicateAssetId(id) => write!(f, "duplicate asset ID: {id}"),
            Self::DuplicateRelativePath(path) => write!(f, "duplicate asset path: {path}"),
            Self::DuplicateDependencyEdge(edge) => write!(f, "duplicate dependency edge: {edge}"),
            Self::InconsistentSharedCauseKind { shared_cause_id, first, second } => write!(f, "shared cause {shared_cause_id} has conflicting kinds: {first} and {second}"),
            Self::UnknownDependencyAcquisition(id) => write!(f, "dependency edge refers to unknown acquisition: {id}"),
            Self::IncompleteDependenceComponent { component_id, missing_acquisition_id } => write!(
                f,
                "cohort omits acquisition {missing_acquisition_id} from dependence component {component_id}"
            ),
            Self::InvalidRelativePath(path) => write!(f, "unsafe or non-canonical relative path: {path}"),
            Self::SymlinkNotAllowed(path) => write!(f, "symbolic link is not allowed in evidence path: {path}"),
            Self::PathEscapesEvidenceRoot(path) => write!(f, "path escapes evidence root: {path}"),
            Self::ExpectedFile(path) => write!(f, "expected a regular file: {path}"),
            Self::ExpectedDirectory(path) => write!(f, "expected a directory: {path}"),
            Self::SourceRootInvalid(path) => write!(f, "invalid evidence root: {path}"),
            Self::ManifestDigestMismatch { expected, actual } => write!(
                f,
                "manifest digest mismatch: expected {expected}, calculated {actual}"
            ),
            Self::AssetLengthMismatch { asset_id, expected, actual } => write!(
                f,
                "asset {asset_id} length mismatch: expected {expected}, read {actual}"
            ),
            Self::AssetDigestMismatch { asset_id, expected, actual } => write!(
                f,
                "asset {asset_id} SHA-256 mismatch: expected {expected}, calculated {actual}"
            ),
            Self::LengthOverflow => write!(f, "asset byte count overflowed u64"),
            Self::Io { path, message } => write!(f, "I/O failure at {path}: {message}"),
            Self::Serialization(message) => write!(f, "manifest serialization failed: {message}"),
            Self::TooFewCohorts => write!(f, "at least two cohort partitions are required"),
            Self::EmptyCohort(cohort) => write!(f, "cohort {cohort} has an empty required identity set"),
            Self::DuplicateCohortId(id) => write!(f, "duplicate cohort ID: {id}"),
            Self::DuplicateCohortRole(role) => write!(f, "duplicate cohort role: {role}"),
            Self::DuplicateCohortMember { cohort_id, identity } => {
                write!(f, "cohort {cohort_id} repeats identity {identity}")
            }
            Self::CohortOverlap { left_cohort, right_cohort, dimension, identity } => write!(
                f,
                "cohorts {left_cohort} and {right_cohort} overlap on {dimension}: {identity}"
            ),
        }
    }
}

impl Error for EvidenceError {}

fn non_empty(value: &str, field: &'static str) -> Result<(), EvidenceError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(EvidenceError::EmptyField { field });
    }
    Ok(())
}

pub fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_sha256(value: &str, field: &'static str) -> Result<(), EvidenceError> {
    if is_lowercase_sha256(value) {
        Ok(())
    } else {
        Err(EvidenceError::InvalidSha256 {
            field,
            value: value.to_owned(),
        })
    }
}

fn is_reserved_windows_component(component: &str) -> bool {
    let stem = component.split('.').next().unwrap_or_default().to_ascii_lowercase();
    matches!(stem.as_str(), "con" | "prn" | "aux" | "nul")
        || (1..=9).any(|number| {
            stem == format!("com{number}") || stem == format!("lpt{number}")
        })
}

fn validate_relative_path(value: &str) -> Result<PathBuf, EvidenceError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.contains('\\')
        || value.contains(':')
        || value.chars().any(char::is_control)
    {
        return Err(EvidenceError::InvalidRelativePath(value.to_owned()));
    }

    let mut path = PathBuf::new();
    for component in value.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.ends_with('.')
            || component.ends_with(' ')
            || is_reserved_windows_component(component)
        {
            return Err(EvidenceError::InvalidRelativePath(value.to_owned()));
        }
        path.push(component);
    }
    if path.as_os_str().is_empty() {
        return Err(EvidenceError::InvalidRelativePath(value.to_owned()));
    }
    Ok(path)
}

impl AcquisitionManifest {
    pub fn validate(&self) -> Result<(), EvidenceError> {
        if self.schema_version != MANIFEST_SCHEMA_VERSION
            || self.canonicalization_version != CANONICALIZATION_VERSION
        {
            return Err(EvidenceError::UnsupportedSchemaVersion);
        }
        non_empty(&self.manifest_id, "manifest_id")?;
        non_empty(&self.source_release_id, "source_release_id")?;
        non_empty(&self.protocol_fingerprint, "protocol_fingerprint")?;
        non_empty(&self.reference_assessment_id, "reference_assessment_id")?;
        non_empty(&self.observer_state_id, "observer_state_id")?;
        non_empty(&self.split_policy_digest, "split_policy_digest")?;
        validate_sha256(&self.split_policy_digest, "split_policy_digest")?;
        validate_sha256(&self.dependency_graph_sha256, "dependency_graph_sha256")?;

        if self.assets.is_empty() {
            return Err(EvidenceError::EmptyAssetInventory);
        }

        let mut asset_ids = HashSet::new();
        // Windows filesystems are commonly case-insensitive; reject ASCII-case
        // collisions consistently on every host rather than passing only on Linux.
        let mut paths = HashSet::new();
        let mut acquisition_ids = BTreeSet::new();

        for asset in &self.assets {
            non_empty(&asset.asset_id, "asset_id")?;
            non_empty(&asset.role, "role")?;
            non_empty(&asset.media_type, "media_type")?;
            non_empty(&asset.acquisition_id, "acquisition_id")?;
            non_empty(&asset.reference_evidence_id, "reference_evidence_id")?;
            non_empty(&asset.clock_domain, "clock_domain")?;
            non_empty(&asset.captured_at_utc, "captured_at_utc")?;
            if asset.byte_length == 0 {
                return Err(EvidenceError::EmptyField { field: "byte_length" });
            }
            validate_relative_path(&asset.relative_path)?;
            validate_sha256(&asset.sha256, "asset.sha256")?;
            if !asset_ids.insert(asset.asset_id.as_str()) {
                return Err(EvidenceError::DuplicateAssetId(asset.asset_id.clone()));
            }
            if !paths.insert(asset.relative_path.to_ascii_lowercase()) {
                return Err(EvidenceError::DuplicateRelativePath(asset.relative_path.clone()));
            }
            acquisition_ids.insert(asset.acquisition_id.as_str());

            if asset.dependence_group_ids.is_empty() {
                return Err(EvidenceError::EmptyField { field: "dependence_group_ids" });
            }
            let mut groups = HashSet::new();
            for group in &asset.dependence_group_ids {
                non_empty(group, "dependence_group_id")?;
                if !groups.insert(group.as_str()) {
                    return Err(EvidenceError::DuplicateCohortMember {
                        cohort_id: asset.asset_id.clone(),
                        identity: group.clone(),
                    });
                }
            }
        }

        let mut edge_ids: HashSet<(String, String, String)> = HashSet::new();
        let mut cause_kinds: BTreeMap<&str, &str> = BTreeMap::new();
        for edge in &self.dependency_edges {
            non_empty(&edge.acquisition_id, "dependency_edge.acquisition_id")?;
            non_empty(&edge.shared_cause_id, "dependency_edge.shared_cause_id")?;
            non_empty(&edge.shared_cause_kind, "dependency_edge.shared_cause_kind")?;
            if !acquisition_ids.contains(edge.acquisition_id.as_str()) {
                return Err(EvidenceError::UnknownDependencyAcquisition(edge.acquisition_id.clone()));
            }
            if let Some(previous_kind) = cause_kinds.get(edge.shared_cause_id.as_str()) {
                if *previous_kind != edge.shared_cause_kind {
                    return Err(EvidenceError::InconsistentSharedCauseKind {
                        shared_cause_id: edge.shared_cause_id.clone(),
                        first: (*previous_kind).to_owned(),
                        second: edge.shared_cause_kind.clone(),
                    });
                }
            } else {
                cause_kinds.insert(edge.shared_cause_id.as_str(), edge.shared_cause_kind.as_str());
            }
            let key = (
                edge.acquisition_id.clone(),
                edge.shared_cause_id.clone(),
                edge.shared_cause_kind.clone(),
            );
            if !edge_ids.insert(key.clone()) {
                return Err(EvidenceError::DuplicateDependencyEdge(format!(
                    "{}/{}/{}",
                    key.0, key.1, key.2
                )));
            }
        }

        let graph_digest = self.calculated_dependency_graph_sha256()?;
        if graph_digest != self.dependency_graph_sha256 {
            return Err(EvidenceError::ManifestDigestMismatch {
                expected: self.dependency_graph_sha256.clone(),
                actual: graph_digest,
            });
        }
        Ok(())
    }

    /// Seal the sorted dependency-edge inventory before sealing the manifest.
    pub fn set_dependency_graph_digest(&mut self) -> Result<(), EvidenceError> {
        self.dependency_graph_sha256 = self.calculated_dependency_graph_sha256()?;
        Ok(())
    }

    pub fn calculated_dependency_graph_sha256(&self) -> Result<String, EvidenceError> {
        let mut edges = self.dependency_edges.clone();
        edges.sort_by(|left, right| {
            (
                left.acquisition_id.as_str(),
                left.shared_cause_id.as_str(),
                left.shared_cause_kind.as_str(),
            )
                .cmp(&(
                    right.acquisition_id.as_str(),
                    right.shared_cause_id.as_str(),
                    right.shared_cause_kind.as_str(),
                ))
        });
        let mut groups_by_acquisition: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for asset in &self.assets {
            groups_by_acquisition
                .entry(asset.acquisition_id.clone())
                .or_default()
                .extend(asset.dependence_group_ids.iter().cloned());
        }
        let canonical_groups: Vec<(String, Vec<String>)> = groups_by_acquisition
            .into_iter()
            .map(|(acquisition_id, groups)| (acquisition_id, groups.into_iter().collect()))
            .collect();
        let bytes = serde_json::to_vec(&(canonical_groups, edges))
            .map_err(|error| EvidenceError::Serialization(error.to_string()))?;
        Ok(sha256_hex(&bytes))
    }

    /// Deterministic JSON bytes: fixed struct field order and sorted inventories.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EvidenceError> {
        self.validate()?;
        let mut canonical = self.clone();
        canonical.assets.sort_by(|left, right| left.asset_id.cmp(&right.asset_id));
        for asset in &mut canonical.assets {
            asset.dependence_group_ids.sort();
        }
        canonical.dependency_edges.sort_by(|left, right| {
            (
                left.acquisition_id.as_str(),
                left.shared_cause_id.as_str(),
                left.shared_cause_kind.as_str(),
            )
                .cmp(&(
                    right.acquisition_id.as_str(),
                    right.shared_cause_id.as_str(),
                    right.shared_cause_kind.as_str(),
                ))
        });
        serde_json::to_vec(&canonical)
            .map_err(|error| EvidenceError::Serialization(error.to_string()))
    }

    pub fn manifest_sha256(&self) -> Result<String, EvidenceError> {
        Ok(sha256_hex(&self.canonical_bytes()?))
    }

    pub fn acquisition_ids(&self) -> BTreeSet<String> {
        self.assets.iter().map(|asset| asset.acquisition_id.clone()).collect()
    }

    fn dependence_components(&self) -> Result<BTreeMap<String, String>, EvidenceError> {
        self.validate()?;
        let acquisition_ids: Vec<String> = self.acquisition_ids().into_iter().collect();
        let indexes: BTreeMap<&str, usize> = acquisition_ids
            .iter()
            .enumerate()
            .map(|(index, id)| (id.as_str(), index))
            .collect();

        let mut parent: Vec<usize> = (0..acquisition_ids.len()).collect();
        let mut by_cause: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for asset in &self.assets {
            let index = *indexes
                .get(asset.acquisition_id.as_str())
                .ok_or_else(|| EvidenceError::UnknownDependencyAcquisition(asset.acquisition_id.clone()))?;
            for group_id in &asset.dependence_group_ids {
                by_cause.entry(group_id.as_str()).or_default().push(index);
            }
        }
        for edge in &self.dependency_edges {
            let index = *indexes
                .get(edge.acquisition_id.as_str())
                .ok_or_else(|| EvidenceError::UnknownDependencyAcquisition(edge.acquisition_id.clone()))?;
            by_cause.entry(edge.shared_cause_id.as_str()).or_default().push(index);
        }

        for members in by_cause.values() {
            if let Some((&first, rest)) = members.split_first() {
                for &other in rest {
                    union(&mut parent, first, other);
                }
            }
        }

        let mut smallest_by_root: BTreeMap<usize, String> = BTreeMap::new();
        for (index, acquisition_id) in acquisition_ids.iter().enumerate() {
            let root = find_root(&mut parent, index);
            smallest_by_root
                .entry(root)
                .and_modify(|current| {
                    if acquisition_id < current {
                        *current = acquisition_id.clone();
                    }
                })
                .or_insert_with(|| acquisition_id.clone());
        }

        let mut component_by_acquisition = BTreeMap::new();
        for (index, acquisition_id) in acquisition_ids.iter().enumerate() {
            let root = find_root(&mut parent, index);
            let representative = smallest_by_root
                .get(&root)
                .expect("every acquisition has a component representative");
            component_by_acquisition.insert(
                acquisition_id.clone(),
                format!("component:{representative}"),
            );
        }
        Ok(component_by_acquisition)
    }

    pub fn cohort_partition(
        &self,
        cohort_id: impl Into<String>,
        role: CohortRole,
        selected_acquisition_ids: &[String],
    ) -> Result<CohortPartition, EvidenceError> {
        self.validate()?;
        let cohort_id = cohort_id.into();
        non_empty(&cohort_id, "cohort_id")?;
        if selected_acquisition_ids.is_empty() {
            return Err(EvidenceError::EmptyCohort("acquisition"));
        }
        let selected: BTreeSet<String> = selected_acquisition_ids.iter().cloned().collect();
        if selected.len() != selected_acquisition_ids.len() {
            return Err(EvidenceError::DuplicateCohortMember {
                cohort_id,
                identity: "acquisition_id".to_owned(),
            });
        }

        let all_acquisitions = self.acquisition_ids();
        for acquisition_id in &selected {
            if !all_acquisitions.contains(acquisition_id) {
                return Err(EvidenceError::UnknownDependencyAcquisition(acquisition_id.clone()));
            }
        }

        let components = self.dependence_components()?;
        let selected_components: BTreeSet<String> = selected
            .iter()
            .map(|id| components.get(id).expect("validated acquisition").clone())
            .collect();

        for (acquisition_id, component_id) in &components {
            if selected_components.contains(component_id) && !selected.contains(acquisition_id) {
                return Err(EvidenceError::IncompleteDependenceComponent {
                    component_id: component_id.clone(),
                    missing_acquisition_id: acquisition_id.clone(),
                });
            }
        }

        let mut asset_ids = BTreeSet::new();
        let mut reference_evidence_ids = BTreeSet::new();
        for asset in &self.assets {
            if selected.contains(&asset.acquisition_id) {
                asset_ids.insert(asset.asset_id.clone());
                reference_evidence_ids.insert(asset.reference_evidence_id.clone());
            }
        }

        Ok(CohortPartition {
            cohort_id,
            role,
            acquisition_ids: selected,
            dependence_component_ids: selected_components,
            asset_ids,
            reference_evidence_ids,
        })
    }
}

fn find_root(parent: &mut [usize], index: usize) -> usize {
    let mut root = index;
    while parent[root] != root {
        root = parent[root];
    }

    let mut current = index;
    while parent[current] != current {
        let next = parent[current];
        parent[current] = root;
        current = next;
    }
    root
}

fn union(parent: &mut [usize], left: usize, right: usize) {
    let left_root = find_root(parent, left);
    let right_root = find_root(parent, right);
    if left_root != right_root {
        let (small, large) = if left_root < right_root {
            (left_root, right_root)
        } else {
            (right_root, left_root)
        };
        parent[large] = small;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CohortRole {
    Training,
    ModelSelection,
    UncertaintyCalibration,
    SealedEvaluation,
    OutOfDomainChallenge,
}

impl fmt::Display for CohortRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Training => "training",
            Self::ModelSelection => "model_selection",
            Self::UncertaintyCalibration => "uncertainty_calibration",
            Self::SealedEvaluation => "sealed_evaluation",
            Self::OutOfDomainChallenge => "out_of_domain_challenge",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CohortPartition {
    cohort_id: String,
    role: CohortRole,
    acquisition_ids: BTreeSet<String>,
    dependence_component_ids: BTreeSet<String>,
    asset_ids: BTreeSet<String>,
    reference_evidence_ids: BTreeSet<String>,
}

impl CohortPartition {
    pub fn cohort_id(&self) -> &str {
        &self.cohort_id
    }

    pub fn role(&self) -> CohortRole {
        self.role
    }

    pub fn acquisition_ids(&self) -> &BTreeSet<String> {
        &self.acquisition_ids
    }

    pub fn dependence_component_ids(&self) -> &BTreeSet<String> {
        &self.dependence_component_ids
    }

    pub fn asset_ids(&self) -> &BTreeSet<String> {
        &self.asset_ids
    }

    pub fn reference_evidence_ids(&self) -> &BTreeSet<String> {
        &self.reference_evidence_ids
    }
}

pub fn validate_disjoint_cohorts(cohorts: &[CohortPartition]) -> Result<(), EvidenceError> {
    if cohorts.len() < 2 {
        return Err(EvidenceError::TooFewCohorts);
    }

    let mut cohort_ids = HashSet::new();
    let mut roles = HashSet::new();
    for cohort in cohorts {
        non_empty(&cohort.cohort_id, "cohort_id")?;
        if !cohort_ids.insert(cohort.cohort_id.as_str()) {
            return Err(EvidenceError::DuplicateCohortId(cohort.cohort_id.clone()));
        }
        let role = cohort.role.to_string();
        if !roles.insert(role.clone()) {
            return Err(EvidenceError::DuplicateCohortRole(role));
        }
        if cohort.acquisition_ids.is_empty() {
            return Err(EvidenceError::EmptyCohort("acquisition"));
        }
        if cohort.dependence_component_ids.is_empty() {
            return Err(EvidenceError::EmptyCohort("dependence component"));
        }
        if cohort.asset_ids.is_empty() {
            return Err(EvidenceError::EmptyCohort("asset"));
        }
        if cohort.reference_evidence_ids.is_empty() {
            return Err(EvidenceError::EmptyCohort("reference evidence"));
        }
    }

    for (left_index, left) in cohorts.iter().enumerate() {
        for right in cohorts.iter().skip(left_index + 1) {
            check_overlap(
                left,
                right,
                "acquisition_id",
                &left.acquisition_ids,
                &right.acquisition_ids,
            )?;
            check_overlap(
                left,
                right,
                "dependence_component",
                &left.dependence_component_ids,
                &right.dependence_component_ids,
            )?;
            check_overlap(left, right, "asset_id", &left.asset_ids, &right.asset_ids)?;
            check_overlap(
                left,
                right,
                "reference_evidence_id",
                &left.reference_evidence_ids,
                &right.reference_evidence_ids,
            )?;
        }
    }
    Ok(())
}

fn check_overlap(
    left: &CohortPartition,
    right: &CohortPartition,
    dimension: &'static str,
    left_values: &BTreeSet<String>,
    right_values: &BTreeSet<String>,
) -> Result<(), EvidenceError> {
    if let Some(identity) = left_values.intersection(right_values).next() {
        return Err(EvidenceError::CohortOverlap {
            left_cohort: left.cohort_id.clone(),
            right_cohort: right.cohort_id.clone(),
            dimension,
            identity: identity.clone(),
        });
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndependenceStatus {
    Independent,
    PartiallyShared,
    Dependent,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceUncertaintyStatus {
    Unknown,
    Quantified,
    ConservativelyBounded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossCovarianceStatus {
    Unknown,
    Estimated,
    ConservativelyBounded,
    ValidatedNegligible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CovarianceClaimScope {
    ResidualCovarianceOnly,
    PivOnlyDecompositionEligibleForIndependentReview,
}

/// Metadata gate only. It does not authenticate the cited assessments or
/// independently validate their statistical adequacy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceAssessment {
    pub assessment_id: String,
    pub independence_status: IndependenceStatus,
    pub uncertainty_status: ReferenceUncertaintyStatus,
    pub uncertainty_evidence_id: String,
    pub cross_covariance_status: CrossCovarianceStatus,
    pub cross_covariance_evidence_id: String,
}

impl ReferenceAssessment {
    pub fn claim_scope(&self) -> CovarianceClaimScope {
        let reference_is_bounded = self.uncertainty_status != ReferenceUncertaintyStatus::Unknown
            && !self.uncertainty_evidence_id.trim().is_empty();
        let cross_covariance_is_bounded =
            self.cross_covariance_status != CrossCovarianceStatus::Unknown
                && !self.cross_covariance_evidence_id.trim().is_empty();

        if !self.assessment_id.trim().is_empty()
            && reference_is_bounded
            && cross_covariance_is_bounded
        {
            CovarianceClaimScope::PivOnlyDecompositionEligibleForIndependentReview
        } else {
            CovarianceClaimScope::ResidualCovarianceOnly
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedAsset {
    asset_id: String,
    byte_length: u64,
    sha256: String,
}

impl VerifiedAsset {
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    pub fn byte_length(&self) -> u64 {
        self.byte_length
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedManifest {
    manifest_sha256: String,
    verified_assets: Vec<VerifiedAsset>,
}

impl VerifiedManifest {
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }

    pub fn verified_assets(&self) -> &[VerifiedAsset] {
        &self.verified_assets
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing into String cannot fail");
    }
    output
}

/// Stream an asset once; byte count and digest are computed from the same
/// observed byte stream to avoid validating a filename or caller-supplied hash.
pub fn verify_asset_reader<R: Read>(
    asset: &AssetRecord,
    reader: &mut R,
) -> Result<VerifiedAsset, EvidenceError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    let mut byte_length = 0_u64;

    loop {
        let read = reader.read(&mut buffer).map_err(|error| EvidenceError::Io {
            path: asset.relative_path.clone(),
            message: error.to_string(),
        })?;
        if read == 0 {
            break;
        }
        byte_length = byte_length
            .checked_add(read as u64)
            .ok_or(EvidenceError::LengthOverflow)?;
        hasher.update(&buffer[..read]);
    }

    let actual_digest = {
        let digest = hasher.finalize();
        let mut output = String::with_capacity(64);
        for byte in digest {
            use std::fmt::Write as _;
            write!(&mut output, "{byte:02x}").expect("writing into String cannot fail");
        }
        output
    };

    if byte_length != asset.byte_length {
        return Err(EvidenceError::AssetLengthMismatch {
            asset_id: asset.asset_id.clone(),
            expected: asset.byte_length,
            actual: byte_length,
        });
    }
    if actual_digest != asset.sha256 {
        return Err(EvidenceError::AssetDigestMismatch {
            asset_id: asset.asset_id.clone(),
            expected: asset.sha256.clone(),
            actual: actual_digest,
        });
    }

    Ok(VerifiedAsset {
        asset_id: asset.asset_id.clone(),
        byte_length,
        sha256: actual_digest,
    })
}

pub fn verify_manifest_directory(
    evidence_root: &Path,
    manifest: &AcquisitionManifest,
    expected_manifest_sha256: &str,
) -> Result<VerifiedManifest, EvidenceError> {
    validate_sha256(expected_manifest_sha256, "expected_manifest_sha256")?;
    let actual_manifest_sha256 = manifest.manifest_sha256()?;
    if actual_manifest_sha256 != expected_manifest_sha256 {
        return Err(EvidenceError::ManifestDigestMismatch {
            expected: expected_manifest_sha256.to_owned(),
            actual: actual_manifest_sha256,
        });
    }

    let root = fs::canonicalize(evidence_root).map_err(|error| EvidenceError::Io {
        path: evidence_root.display().to_string(),
        message: error.to_string(),
    })?;
    if !root.is_dir() {
        return Err(EvidenceError::SourceRootInvalid(root.display().to_string()));
    }

    let mut verified_assets = Vec::with_capacity(manifest.assets.len());
    for asset in &manifest.assets {
        let relative_path = validate_relative_path(&asset.relative_path)?;
        let candidate = root.join(relative_path);
        verify_no_symlink_components(&root, &candidate, &asset.relative_path)?;

        let canonical_path = fs::canonicalize(&candidate).map_err(|error| EvidenceError::Io {
            path: candidate.display().to_string(),
            message: error.to_string(),
        })?;
        if !canonical_path.starts_with(&root) {
            return Err(EvidenceError::PathEscapesEvidenceRoot(
                asset.relative_path.clone(),
            ));
        }

        let mut file = File::open(&canonical_path).map_err(|error| EvidenceError::Io {
            path: canonical_path.display().to_string(),
            message: error.to_string(),
        })?;
        let verified = verify_asset_reader(asset, &mut file)?;
        verified_assets.push(verified);
    }

    verified_assets.sort_by(|left, right| left.asset_id.cmp(&right.asset_id));
    Ok(VerifiedManifest {
        manifest_sha256: expected_manifest_sha256.to_owned(),
        verified_assets,
    })
}

fn verify_no_symlink_components(
    root: &Path,
    candidate: &Path,
    relative_path: &str,
) -> Result<(), EvidenceError> {
    let relative = candidate
        .strip_prefix(root)
        .map_err(|_| EvidenceError::PathEscapesEvidenceRoot(relative_path.to_owned()))?;
    let components: Vec<_> = relative.components().collect();
    let mut current = root.to_path_buf();

    for (index, component) in components.iter().enumerate() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current).map_err(|error| EvidenceError::Io {
            path: current.display().to_string(),
            message: error.to_string(),
        })?;
        if metadata.file_type().is_symlink() {
            return Err(EvidenceError::SymlinkNotAllowed(
                current.display().to_string(),
            ));
        }
        let is_last = index + 1 == components.len();
        if is_last && !metadata.is_file() {
            return Err(EvidenceError::ExpectedFile(current.display().to_string()));
        }
        if !is_last && !metadata.is_dir() {
            return Err(EvidenceError::ExpectedDirectory(current.display().to_string()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_root() -> PathBuf {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "ehd-piv-evidence-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn asset(
        id: &str,
        path: &str,
        acquisition: &str,
        reference: &str,
        bytes: &[u8],
        groups: &[&str],
    ) -> AssetRecord {
        AssetRecord {
            asset_id: id.to_owned(),
            role: "piv_image_pair".to_owned(),
            relative_path: path.to_owned(),
            byte_length: bytes.len() as u64,
            media_type: "application/octet-stream".to_owned(),
            sha256: sha256_hex(bytes),
            acquisition_id: acquisition.to_owned(),
            dependence_group_ids: groups.iter().map(|value| (*value).to_owned()).collect(),
            reference_evidence_id: reference.to_owned(),
            clock_domain: "camera-clock-v1".to_owned(),
            captured_at_utc: "2026-11-23T08:00:00Z".to_owned(),
        }
    }

    fn manifest() -> AcquisitionManifest {
        let mut value = AcquisitionManifest {
            schema_version: MANIFEST_SCHEMA_VERSION.to_owned(),
            canonicalization_version: CANONICALIZATION_VERSION.to_owned(),
            manifest_id: "manifest-001".to_owned(),
            source_release_id: "lab-run-2026-11".to_owned(),
            protocol_fingerprint: "piv-32px-75overlap-v1".to_owned(),
            reference_assessment_id: "reference-assessment-001".to_owned(),
            observer_state_id: "observer-state-baseline-v1".to_owned(),
            split_policy_digest: sha256_hex(b"split-policy-v1"),
            dependency_graph_sha256: String::new(),
            assets: vec![
                asset("asset-a", "raw/acq-a.bin", "acq-a", "ref-a", b"image-a", &["camera-cal-1"]),
                asset("asset-b", "raw/acq-b.bin", "acq-b", "ref-b", b"image-b", &["camera-cal-2"]),
                asset("asset-c", "raw/acq-c.bin", "acq-c", "ref-c", b"image-c", &["camera-cal-3"]),
            ],
            dependency_edges: vec![
                SharedCauseEdge {
                    acquisition_id: "acq-a".to_owned(),
                    shared_cause_id: "campaign-1".to_owned(),
                    shared_cause_kind: "campaign".to_owned(),
                },
                SharedCauseEdge {
                    acquisition_id: "acq-b".to_owned(),
                    shared_cause_id: "campaign-2".to_owned(),
                    shared_cause_kind: "campaign".to_owned(),
                },
                SharedCauseEdge {
                    acquisition_id: "acq-c".to_owned(),
                    shared_cause_id: "campaign-3".to_owned(),
                    shared_cause_kind: "campaign".to_owned(),
                },
            ],
        };
        value.set_dependency_graph_digest().unwrap();
        value
    }

    #[test]
    fn sha256_matches_published_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn streaming_asset_verification_checks_length_and_digest_from_same_stream() {
        let record = asset("asset", "raw.bin", "acq", "ref", b"source bytes", &["group"]);
        let result = verify_asset_reader(&record, &mut Cursor::new(b"source bytes")).unwrap();
        assert_eq!(result.byte_length, 12);
        assert_eq!(result.sha256, record.sha256);
    }

    #[test]
    fn changed_asset_bytes_fail_even_when_filename_is_unchanged() {
        let record = asset("asset", "raw.bin", "acq", "ref", b"original", &["group"]);
        let result = verify_asset_reader(&record, &mut Cursor::new(b"mutated!"));
        assert!(matches!(result, Err(EvidenceError::AssetDigestMismatch { .. })));
    }

    #[test]
    fn changed_asset_length_fails_before_acceptance() {
        let record = asset("asset", "raw.bin", "acq", "ref", b"original", &["group"]);
        let result = verify_asset_reader(&record, &mut Cursor::new(b"short"));
        assert!(matches!(result, Err(EvidenceError::AssetLengthMismatch { .. })));
    }

    #[test]
    fn canonical_manifest_digest_is_inventory_order_invariant() {
        let first = manifest();
        let mut second = first.clone();
        second.assets.reverse();
        second.dependency_edges.reverse();
        for record in &mut second.assets {
            record.dependence_group_ids.reverse();
        }
        assert_eq!(first.manifest_sha256().unwrap(), second.manifest_sha256().unwrap());
    }

    #[test]
    fn changing_any_bound_asset_digest_changes_the_manifest_digest() {
        let first = manifest();
        let mut second = first.clone();
        second.assets[0].sha256 = sha256_hex(b"different payload");
        assert_ne!(first.manifest_sha256().unwrap(), second.manifest_sha256().unwrap());
    }

    #[test]
    fn invalid_syntax_is_not_accepted_as_a_manifest_hash() {
        let mut value = manifest();
        value.dependency_graph_sha256 = "a".repeat(63);
        assert!(matches!(value.validate(), Err(EvidenceError::InvalidSha256 { .. })));
    }

    #[test]
    fn duplicate_asset_ids_and_paths_are_rejected() {
        let mut duplicate_id = manifest();
        duplicate_id.assets[1].asset_id = duplicate_id.assets[0].asset_id.clone();
        duplicate_id.set_dependency_graph_digest().unwrap();
        assert!(matches!(duplicate_id.validate(), Err(EvidenceError::DuplicateAssetId(_))));

        let mut duplicate_path = manifest();
        duplicate_path.assets[1].relative_path = duplicate_path.assets[0].relative_path.clone();
        assert!(matches!(duplicate_path.validate(), Err(EvidenceError::DuplicateRelativePath(_))));
    }

    #[test]
    fn traversal_and_absolute_paths_are_rejected() {
        for path in ["../secret.bin", "/etc/passwd", "raw/../../secret", "raw\\secret.bin"] {
            let mut value = manifest();
            value.assets[0].relative_path = path.to_owned();
            assert!(matches!(value.validate(), Err(EvidenceError::InvalidRelativePath(_))));
        }
    }

    #[test]
    fn ascii_case_collisions_are_rejected_for_windows_portability() {
        let mut value = manifest();
        value.assets[1].relative_path = "RAW/ACQ-A.BIN".to_owned();
        value.assets[0].relative_path = "raw/acq-a.bin".to_owned();
        assert!(matches!(
            value.validate(),
            Err(EvidenceError::DuplicateRelativePath(_))
        ));
    }

    #[test]
    fn windows_reserved_and_trailing_dot_paths_are_rejected() {
        for path in ["raw/CON.txt", "raw/NUL", "raw/trace."] {
            let mut value = manifest();
            value.assets[0].relative_path = path.to_owned();
            assert!(matches!(
                value.validate(),
                Err(EvidenceError::InvalidRelativePath(_))
            ));
        }
    }

    #[test]
    fn directory_verifier_rehashes_source_bytes_and_binds_manifest() {
        let root = temp_root();
        fs::create_dir_all(root.join("raw")).unwrap();
        fs::write(root.join("raw/acq-a.bin"), b"image-a").unwrap();
        fs::write(root.join("raw/acq-b.bin"), b"image-b").unwrap();
        fs::write(root.join("raw/acq-c.bin"), b"image-c").unwrap();

        let value = manifest();
        let digest = value.manifest_sha256().unwrap();
        let report = verify_manifest_directory(&root, &value, &digest).unwrap();
        assert_eq!(report.verified_assets.len(), 3);
        assert_eq!(report.manifest_sha256, digest);

        fs::write(root.join("raw/acq-a.bin"), b"changed").unwrap();
        let result = verify_manifest_directory(&root, &value, &digest);
        assert!(matches!(result, Err(EvidenceError::AssetDigestMismatch { .. })));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn directory_verifier_rejects_symlink_assets() {
        use std::os::unix::fs::symlink;

        let root = temp_root();
        let outside = root.with_extension("outside");
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, root.join("linked.bin")).unwrap();
        let mut value = manifest();
        value.assets.truncate(1);
        value.assets[0].relative_path = "linked.bin".to_owned();
        value.assets[0].byte_length = 7;
        value.assets[0].sha256 = sha256_hex(b"outside");
        value.dependency_edges.retain(|edge| edge.acquisition_id == "acq-a");
        value.set_dependency_graph_digest().unwrap();

        let result = verify_manifest_directory(&root, &value, &value.manifest_sha256().unwrap());
        assert!(matches!(result, Err(EvidenceError::SymlinkNotAllowed(_))));
        fs::remove_dir_all(root).unwrap();
        fs::remove_file(outside).unwrap();
    }

    #[test]
    fn transitive_shared_causes_form_one_non_splittable_component() {
        let mut value = manifest();
        value.dependency_edges.extend([
            SharedCauseEdge {
                acquisition_id: "acq-a".to_owned(),
                shared_cause_id: "camera-cal".to_owned(),
                shared_cause_kind: "camera_calibration".to_owned(),
            },
            SharedCauseEdge {
                acquisition_id: "acq-b".to_owned(),
                shared_cause_id: "camera-cal".to_owned(),
                shared_cause_kind: "camera_calibration".to_owned(),
            },
            SharedCauseEdge {
                acquisition_id: "acq-b".to_owned(),
                shared_cause_id: "tracer-batch".to_owned(),
                shared_cause_kind: "tracer_batch".to_owned(),
            },
            SharedCauseEdge {
                acquisition_id: "acq-c".to_owned(),
                shared_cause_id: "tracer-batch".to_owned(),
                shared_cause_kind: "tracer_batch".to_owned(),
            },
        ]);
        value.set_dependency_graph_digest().unwrap();

        let result = value.cohort_partition(
            "train",
            CohortRole::Training,
            &["acq-a".to_owned()],
        );
        assert!(matches!(
            result,
            Err(EvidenceError::IncompleteDependenceComponent { .. })
        ));
    }

    #[test]
    fn shared_asset_dependence_group_prevents_acquisition_split_without_edge_rows() {
        let mut value = manifest();
        value.assets[0].dependence_group_ids.push("shared-camera-calibration".to_owned());
        value.assets[1].dependence_group_ids.push("shared-camera-calibration".to_owned());
        value.set_dependency_graph_digest().unwrap();

        let result = value.cohort_partition(
            "train",
            CohortRole::Training,
            &["acq-a".to_owned()],
        );
        assert!(matches!(
            result,
            Err(EvidenceError::IncompleteDependenceComponent { .. })
        ));
    }

    #[test]
    fn all_split_roles_can_be_disjoint_at_the_dependence_component_level() {
        let value = manifest();
        let training = value
            .cohort_partition("train", CohortRole::Training, &["acq-a".to_owned()])
            .unwrap();
        let calibration = value
            .cohort_partition(
                "calibration",
                CohortRole::UncertaintyCalibration,
                &["acq-b".to_owned()],
            )
            .unwrap();
        let evaluation = value
            .cohort_partition(
                "test",
                CohortRole::SealedEvaluation,
                &["acq-c".to_owned()],
            )
            .unwrap();
        assert!(validate_disjoint_cohorts(&[training, calibration, evaluation]).is_ok());
    }

    #[test]
    fn cohort_overlap_by_reference_evidence_is_rejected() {
        let value = manifest();
        let left = value
            .cohort_partition("train", CohortRole::Training, &["acq-a".to_owned()])
            .unwrap();
        let mut right = value
            .cohort_partition(
                "test",
                CohortRole::SealedEvaluation,
                &["acq-b".to_owned()],
            )
            .unwrap();
        right.reference_evidence_ids.insert("ref-a".to_owned());
        assert!(matches!(
            validate_disjoint_cohorts(&[left, right]),
            Err(EvidenceError::CohortOverlap {
                dimension: "reference_evidence_id",
                ..
            })
        ));
    }

    #[test]
    fn unknown_reference_or_cross_covariance_allows_only_residual_covariance_claim() {
        let assessment = ReferenceAssessment {
            assessment_id: "reference-assessment-1".to_owned(),
            independence_status: IndependenceStatus::Unknown,
            uncertainty_status: ReferenceUncertaintyStatus::Quantified,
            uncertainty_evidence_id: "ref-uncertainty-1".to_owned(),
            cross_covariance_status: CrossCovarianceStatus::Unknown,
            cross_covariance_evidence_id: String::new(),
        };
        assert_eq!(
            assessment.claim_scope(),
            CovarianceClaimScope::ResidualCovarianceOnly
        );
    }

    #[test]
    fn quantified_reference_and_cross_covariance_are_only_eligible_for_review() {
        let assessment = ReferenceAssessment {
            assessment_id: "reference-assessment-1".to_owned(),
            independence_status: IndependenceStatus::PartiallyShared,
            uncertainty_status: ReferenceUncertaintyStatus::ConservativelyBounded,
            uncertainty_evidence_id: "ref-uncertainty-1".to_owned(),
            cross_covariance_status: CrossCovarianceStatus::Estimated,
            cross_covariance_evidence_id: "cross-covariance-1".to_owned(),
        };
        assert_eq!(
            assessment.claim_scope(),
            CovarianceClaimScope::PivOnlyDecompositionEligibleForIndependentReview
        );
    }
}
