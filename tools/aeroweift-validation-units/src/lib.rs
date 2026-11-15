//! Aviation validation denominator and independence integrity oracle.
//!
//! This crate derives sample counts from exact independent-unit identities and preserves
//! nested dependence clusters across split boundaries. It does not decide whether source
//! clusters are physically independent, calculate bootstrap intervals, or authenticate
//! source manifests. It composes the qualified interval methods owned by Mycelix.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

/// Statistical unit whose identity defines the denominator for a validation claim.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum IndependentUnitKind {
    /// One complete aircraft flight.
    Flight,
    /// One adjudicated encounter event.
    Encounter,
    /// One sensor acquisition trial or experiment.
    AcquisitionTrial,
    /// One independent sensor collection session.
    SensorSession,
    /// One airport-day or equivalent environment campaign.
    AirportDay,
    /// One independently specified traffic/scenario generator family.
    TrafficGeneratorFamily,
    /// One independently specified model implementation/family.
    ModelFamily,
}

/// Frozen partition role for an observation/unit.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SplitRole {
    /// Model fitting and parameter calibration.
    Calibration,
    /// Adaptive development and experiment selection.
    Development,
    /// Frozen confirmatory evaluation.
    Confirmatory,
    /// Untouched challenge set.
    Challenge,
}

/// Higher-level dependence cluster that can contain several independent-unit IDs.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ClusterDimension {
    /// Aircraft tail or configuration lineage.
    AircraftConfigurationFamily,
    /// Sensor, antenna, calibration or acquisition-stack lineage.
    SensorConfigurationFamily,
    /// Flight/session grouping.
    FlightSession,
    /// Airport, route and day grouping.
    AirportDay,
    /// Spatial/temporal weather or operating-environment campaign.
    EnvironmentCampaign,
    /// Shared encounter, traffic episode or scenario family.
    ScenarioFamily,
    /// Fitted traffic or environment generator lineage.
    TrafficGeneratorFamily,
    /// Model form, implementation or shared model-training lineage.
    ModelFamily,
}

/// One source-declared membership of an independent unit in a dependence cluster.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClusterMembership {
    /// Cluster dimension.
    pub dimension: ClusterDimension,
    /// Exact stable cluster identity in the source namespace.
    pub cluster_id: String,
}

/// One observation or metric-bearing record nested within an independent unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationUnit {
    /// Stable observation/record identity.
    pub observation_id: String,
    /// Exact independent-unit identity selected by the frozen estimand.
    pub independent_unit_id: String,
    /// Split assignment.
    pub split: SplitRole,
    /// Required source-declared higher-level dependence memberships.
    pub clusters: Vec<ClusterMembership>,
}

/// A dependence dimension whose cluster identities must not cross two splits.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SplitSeparation {
    /// Dependence dimension whose IDs must remain isolated.
    pub dimension: ClusterDimension,
    /// Earlier/training split.
    pub left: SplitRole,
    /// Held-out split.
    pub right: SplitRole,
}

/// One independently reported metric denominator claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DenominatorClaim {
    /// Metric identity.
    pub metric_id: String,
    /// Split from which the metric was computed.
    pub split: SplitRole,
    /// Reported number of independent units. Must equal the derived unique ID count.
    pub reported_independent_unit_count: u64,
    /// Dependence dimension used for the reported top-level cluster count.
    pub top_level_cluster_dimension: ClusterDimension,
    /// Reported number of distinct top-level clusters.
    pub reported_top_level_cluster_count: u64,
    /// Minimum number of independent units required by the metric's frozen policy.
    pub minimum_independent_units: u64,
}

/// Frozen policy for the identity/dependence manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitIntegrityPolicy {
    /// Digest of the exact dataset/source manifest.
    pub source_manifest_sha256: String,
    /// Digest of the exact unit/split policy.
    pub policy_sha256: String,
    /// Statistical unit that defines the denominator for the campaign.
    pub independent_unit_kind: IndependentUnitKind,
    /// Dependence dimensions that every independent unit must declare.
    pub required_cluster_dimensions: Vec<ClusterDimension>,
    /// Split pairs for which shared cluster IDs are prohibited.
    pub required_split_separations: Vec<SplitSeparation>,
    /// Whether every observation must carry at least one source record identifier.
    pub require_observation_identity: bool,
}

/// Derived count for one split.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitUnitSummary {
    /// Split identity.
    pub split: SplitRole,
    /// Exact count of distinct independent-unit IDs.
    pub independent_unit_count: u64,
    /// Distinct cluster counts by dimension.
    pub cluster_counts: BTreeMap<ClusterDimension, u64>,
    /// Raw observation/record count; never substituted for independent units.
    pub observation_count: u64,
}

/// Output of exact identity and denominator verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitIntegrityReport {
    /// Exact source manifest digest.
    pub source_manifest_sha256: String,
    /// Exact policy digest.
    pub policy_sha256: String,
    /// Unit type defining this campaign's denominator.
    pub independent_unit_kind: IndependentUnitKind,
    /// Derived counts, one per represented split.
    pub split_summaries: Vec<SplitUnitSummary>,
    /// Number of denominator claims matched to exact unit/cluster identities.
    pub verified_denominator_claims: usize,
}

/// Typed failure; no positive report is returned after any invariant fails.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitIntegrityError {
    /// Policy/source manifest identity is malformed.
    InvalidPolicy,
    /// Policy has duplicate or missing required cluster dimensions.
    InvalidClusterPolicy,
    /// Duplicate observation identity.
    DuplicateObservationId,
    /// Invalid or empty unit identity.
    InvalidIndependentUnitIdentity,
    /// An observation omits a required cluster dimension.
    MissingRequiredCluster,
    /// An observation duplicates or contradicts one cluster dimension.
    DuplicateOrConflictingClusterMembership,
    /// One independent unit is assigned to more than one split.
    IndependentUnitSplitLeakage,
    /// The same required dependence cluster crosses a protected split boundary.
    DependenceClusterSplitLeakage,
    /// A denominator claim is malformed or has an unsupported cluster dimension.
    InvalidDenominatorClaim,
    /// Reported denominator differs from the derived unit identity count.
    IndependentUnitCountMismatch,
    /// Reported top-level cluster count differs from the derived cluster count.
    TopLevelClusterCountMismatch,
    /// Derived independent-unit count is below the frozen minimum.
    InsufficientIndependentUnits,
    /// A required split is missing from the source manifest.
    MissingSplit,
}

/// Validate SHA-256 text shape only; byte authentication is external.
fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_policy(policy: &UnitIntegrityPolicy) -> Result<(), UnitIntegrityError> {
    if !is_sha256(&policy.source_manifest_sha256) || !is_sha256(&policy.policy_sha256) {
        return Err(UnitIntegrityError::InvalidPolicy);
    }
    if policy.required_cluster_dimensions.is_empty() {
        return Err(UnitIntegrityError::InvalidClusterPolicy);
    }
    let required: BTreeSet<ClusterDimension> =
        policy.required_cluster_dimensions.iter().copied().collect();
    if required.len() != policy.required_cluster_dimensions.len() {
        return Err(UnitIntegrityError::InvalidClusterPolicy);
    }
    let mut separations = BTreeSet::new();
    for separation in &policy.required_split_separations {
        if separation.left == separation.right
            || !required.contains(&separation.dimension)
            || !separations.insert(*separation)
        {
            return Err(UnitIntegrityError::InvalidClusterPolicy);
        }
    }
    Ok(())
}

fn cluster_map(
    clusters: &[ClusterMembership],
) -> Result<BTreeMap<ClusterDimension, String>, UnitIntegrityError> {
    let mut output = BTreeMap::new();
    for membership in clusters {
        if membership.cluster_id.trim().is_empty()
            || output
                .insert(membership.dimension, membership.cluster_id.clone())
                .is_some()
        {
            return Err(UnitIntegrityError::DuplicateOrConflictingClusterMembership);
        }
    }
    Ok(output)
}

#[derive(Clone)]
struct UnitRecord {
    split: SplitRole,
    clusters: BTreeMap<ClusterDimension, String>,
}

/// Validate the exact unit manifest and derive the metric denominators.
///
/// Repeated observations are counted in observation_count, but each independent
/// unit ID contributes exactly one to independent_unit_count. All records carrying
/// a unit ID must agree on split and cluster memberships.
pub fn validate_unit_manifest(
    observations: &[ObservationUnit],
    denominator_claims: &[DenominatorClaim],
    policy: &UnitIntegrityPolicy,
) -> Result<UnitIntegrityReport, UnitIntegrityError> {
    validate_policy(policy)?;

    let required: BTreeSet<ClusterDimension> =
        policy.required_cluster_dimensions.iter().copied().collect();

    let mut observation_ids = BTreeSet::new();
    let mut units: BTreeMap<String, UnitRecord> = BTreeMap::new();
    let mut split_observation_counts: BTreeMap<SplitRole, u64> = BTreeMap::new();
    let mut split_cluster_ids: BTreeMap<(SplitRole, ClusterDimension), BTreeSet<String>> =
        BTreeMap::new();

    for observation in observations {
        if policy.require_observation_identity && observation.observation_id.trim().is_empty() {
            return Err(UnitIntegrityError::DuplicateObservationId);
        }
        if !observation_ids.insert(observation.observation_id.as_str()) {
            return Err(UnitIntegrityError::DuplicateObservationId);
        }
        if observation.independent_unit_id.trim().is_empty() {
            return Err(UnitIntegrityError::InvalidIndependentUnitIdentity);
        }

        let memberships = cluster_map(&observation.clusters)?;
        if required
            .iter()
            .any(|dimension| !memberships.contains_key(dimension))
        {
            return Err(UnitIntegrityError::MissingRequiredCluster);
        }

        if let Some(existing) = units.get(&observation.independent_unit_id) {
            if existing.split != observation.split {
                return Err(UnitIntegrityError::IndependentUnitSplitLeakage);
            }
            if existing.clusters != memberships {
                return Err(UnitIntegrityError::DuplicateOrConflictingClusterMembership);
            }
        } else {
            units.insert(
                observation.independent_unit_id.clone(),
                UnitRecord {
                    split: observation.split,
                    clusters: memberships.clone(),
                },
            );
        }

        *split_observation_counts.entry(observation.split).or_default() += 1;
        for (dimension, cluster_id) in memberships {
            split_cluster_ids
                .entry((observation.split, dimension))
                .or_default()
                .insert(cluster_id);
        }
    }

    if observations.is_empty() {
        return Err(UnitIntegrityError::MissingSplit);
    }

    for separation in &policy.required_split_separations {
        let left = split_cluster_ids
            .get(&(separation.left, separation.dimension))
            .cloned()
            .unwrap_or_default();
        let right = split_cluster_ids
            .get(&(separation.right, separation.dimension))
            .cloned()
            .unwrap_or_default();
        if left.iter().any(|cluster| right.contains(cluster)) {
            return Err(UnitIntegrityError::DependenceClusterSplitLeakage);
        }
    }

    let mut summaries_by_split: BTreeMap<SplitRole, SplitUnitSummary> = BTreeMap::new();
    for (unit_id, record) in &units {
        let _identity_key = unit_id;
        let summary = summaries_by_split
            .entry(record.split)
            .or_insert_with(|| SplitUnitSummary {
                split: record.split,
                independent_unit_count: 0,
                cluster_counts: BTreeMap::new(),
                observation_count: *split_observation_counts.get(&record.split).unwrap_or(&0),
            });
        summary.independent_unit_count += 1;
        for dimension in &required {
            let distinct_count = split_cluster_ids
                .get(&(record.split, *dimension))
                .map_or(0, |values| values.len() as u64);
            summary.cluster_counts.insert(*dimension, distinct_count);
        }
    }

    for claim in denominator_claims {
        if claim.metric_id.trim().is_empty()
            || claim.reported_independent_unit_count == 0
            || claim.reported_top_level_cluster_count == 0
            || claim.minimum_independent_units == 0
            || !required.contains(&claim.top_level_cluster_dimension)
        {
            return Err(UnitIntegrityError::InvalidDenominatorClaim);
        }
        let summary = summaries_by_split
            .get(&claim.split)
            .ok_or(UnitIntegrityError::MissingSplit)?;

        if summary.independent_unit_count != claim.reported_independent_unit_count {
            return Err(UnitIntegrityError::IndependentUnitCountMismatch);
        }
        let derived_clusters = *summary
            .cluster_counts
            .get(&claim.top_level_cluster_dimension)
            .unwrap_or(&0);
        if derived_clusters != claim.reported_top_level_cluster_count {
            return Err(UnitIntegrityError::TopLevelClusterCountMismatch);
        }
        if summary.independent_unit_count < claim.minimum_independent_units {
            return Err(UnitIntegrityError::InsufficientIndependentUnits);
        }
    }

    Ok(UnitIntegrityReport {
        source_manifest_sha256: policy.source_manifest_sha256.clone(),
        policy_sha256: policy.policy_sha256.clone(),
        independent_unit_kind: policy.independent_unit_kind,
        split_summaries: summaries_by_split.into_values().collect(),
        verified_denominator_claims: denominator_claims.len(),
    })
}
