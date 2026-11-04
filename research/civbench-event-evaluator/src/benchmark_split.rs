// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed benchmark partition auditing for water-network evaluation.
//!
//! A high sample count does not establish independent generalization. Samples
//! derived from the same physical event, capture session, pipe, network,
//! topology family, hydraulic model, demand profile, sensor layout, scenario
//! template, or time block can leak information across nominal train/test
//! boundaries. This module audits explicitly declared grouping dimensions and
//! exact-content digests before a benchmark score is interpreted.
//!
//! The caller chooses which dimensions must be exclusive for the declared
//! evaluation protocol. For example, a within-network unseen-event protocol
//! can require event/session/template disjointness while allowing the same
//! network; a cross-network protocol should additionally require network
//! disjointness. This validator does not infer the correct protocol, inspect
//! raw data, recompute digests, detect near-duplicates, or prove that a dataset
//! is representative. Those remain provenance and study-design obligations.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// A dataset partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataPartition {
    /// Model fitting data.
    Train,
    /// Model selection and calibration data.
    Validation,
    /// Frozen final evaluation data.
    Test,
}

impl DataPartition {
    /// Stable machine-readable partition name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Train => "train",
            Self::Validation => "validation",
            Self::Test => "test",
        }
    }
}

/// A provenance dimension whose values can create train/evaluation leakage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LeakageDimension {
    /// A physical leak, burst, or other independently labelled event.
    PhysicalEvent,
    /// A continuous field recording, logger capture, or monitoring session.
    CaptureSession,
    /// A physical pipe, valve, meter, or other asset identity.
    PipeAsset,
    /// The hydraulic network instance.
    Network,
    /// A topology family or template shared across network instances.
    TopologyFamily,
    /// A leak/scenario generator template or parameterized scenario family.
    ScenarioTemplate,
    /// The deployed sensor set and placement configuration.
    SensorLayout,
    /// Hydraulic simulator/model calibration lineage.
    HydraulicModel,
    /// Demand and operating-profile lineage.
    DemandProfile,
    /// A chronological block used for forward-in-time evaluation.
    TimeBlock,
}

impl LeakageDimension {
    /// Stable machine-readable dimension name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PhysicalEvent => "physical_event",
            Self::CaptureSession => "capture_session",
            Self::PipeAsset => "pipe_asset",
            Self::Network => "network",
            Self::TopologyFamily => "topology_family",
            Self::ScenarioTemplate => "scenario_template",
            Self::SensorLayout => "sensor_layout",
            Self::HydraulicModel => "hydraulic_model",
            Self::DemandProfile => "demand_profile",
            Self::TimeBlock => "time_block",
        }
    }
}

/// One declared provenance group attached to a sample.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeakageGroup {
    /// Provenance dimension.
    pub dimension: LeakageDimension,
    /// Stable identifier for the event/session/asset/etc.
    pub value: String,
}

/// One independently addressable sample in a benchmark manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkSample {
    /// Stable sample identifier, unique across the complete manifest.
    pub sample_id: String,
    /// Assigned partition.
    pub partition: DataPartition,
    /// Caller-computed SHA-256 digest of the canonical sample payload.
    ///
    /// Digest syntax is checked, but the raw sample is not available here and
    /// the digest is not recomputed or authenticated by this validator.
    pub content_digest: String,
    /// Provenance identifiers available for the sample.
    pub leakage_groups: Vec<LeakageGroup>,
}

/// A frozen split protocol and its required cross-partition firewalls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitPolicy {
    /// Stable protocol identifier, such as `cross_network_v1`.
    pub protocol_id: String,
    /// Caller-computed SHA-256 digest of the frozen canonical manifest bytes.
    /// This validator checks syntax but cannot recompute it without raw data.
    pub manifest_digest: String,
    /// Every listed dimension must be present on every sample and disjoint
    /// between train, validation, and test.
    pub exclusive_dimensions: BTreeSet<LeakageDimension>,
    /// At least one identical group value in each listed dimension must occur
    /// in train, validation, and test. This prevents a within-group protocol
    /// from silently becoming a cross-group transfer experiment.
    pub shared_dimensions: BTreeSet<LeakageDimension>,
}

/// Deterministic counts from an accepted split manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitAuditReport {
    /// Protocol identifier that was audited.
    pub protocol_id: String,
    /// Frozen input manifest digest (syntax-checked, not recomputed).
    pub manifest_digest: String,
    /// Cross-partition firewalls actually enforced.
    pub exclusive_dimensions: BTreeSet<LeakageDimension>,
    /// Dimensions required to remain common across all three partitions.
    pub shared_dimensions: BTreeSet<LeakageDimension>,
    /// Number of unique samples.
    pub sample_count: usize,
    /// Sample counts for all three required partitions.
    pub partition_counts: BTreeMap<DataPartition, usize>,
    /// Number of distinct values per declared provenance dimension.
    pub group_counts: BTreeMap<LeakageDimension, usize>,
    /// Number of distinct canonical sample digests.
    pub distinct_content_digests: usize,
}

impl SplitAuditReport {
    /// Render a stable summary; no source observations or group identifiers
    /// are exposed in this aggregate receipt.
    pub fn canonical_text(&self) -> String {
        let mut output = String::new();
        let _ = writeln!(&mut output, "CIVBENCH_SPLIT_AUDIT_V0.1");
        let _ = writeln!(&mut output, "protocol_id={}", self.protocol_id);
        let _ = writeln!(&mut output, "manifest_digest=sha256:{}", self.manifest_digest);
        for dimension in &self.exclusive_dimensions {
            let _ = writeln!(
                &mut output,
                "exclusive_dimension={}",
                dimension.as_str()
            );
        }
        for dimension in &self.shared_dimensions {
            let _ = writeln!(&mut output, "shared_dimension={}", dimension.as_str());
        }
        let _ = writeln!(&mut output, "sample_count={}", self.sample_count);
        for (partition, count) in &self.partition_counts {
            let _ = writeln!(&mut output, "partition={},count={count}", partition.as_str());
        }
        for (dimension, count) in &self.group_counts {
            let _ = writeln!(
                &mut output,
                "groups={},count={count}",
                dimension.as_str()
            );
        }
        let _ = writeln!(
            &mut output,
            "distinct_content_digests={}",
            self.distinct_content_digests
        );
        output
    }
}

/// Validation failure. Error codes are stable; subjects identify the failing
/// manifest item or provenance group without exposing sample payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitAuditError {
    /// Stable machine-readable failure category.
    pub code: &'static str,
    /// Sample, partition, or grouping key associated with the failure.
    pub subject: String,
}

impl SplitAuditError {
    fn new(code: &'static str, subject: impl Into<String>) -> Self {
        Self {
            code,
            subject: subject.into(),
        }
    }
}

impl std::fmt::Display for SplitAuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.subject)
    }
}

impl std::error::Error for SplitAuditError {}

fn valid_identifier(value: &str) -> bool {
    if value.is_empty() || value.len() > 256 || value.trim() != value {
        return false;
    }
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && bytes.all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn normalized_sha256(value: &str) -> Option<String> {
    let hex = value.strip_prefix("sha256:")?;
    if hex.len() != 64
        || !hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        || hex.bytes().all(|byte| byte == b'0')
    {
        return None;
    }
    Some(hex.to_ascii_lowercase())
}

/// Audit train/validation/test separation using explicit provenance groups.
///
/// The function always rejects repeated exact-content digests across distinct
/// sample IDs, and rejects any configured group value appearing in more than
/// one partition. Each of train, validation, and test must be non-empty.
/// Repeated group values within a single partition are expected and allowed.
///
/// The caller must choose dimensions appropriate to the generalization claim:
/// do not call a split "cross-network" unless `Network` is an exclusive
/// dimension; do not call it "forward temporal" unless `TimeBlock` is
/// exclusive and the caller has separately verified chronological ordering.
pub fn audit_benchmark_split(
    samples: &[BenchmarkSample],
    policy: &SplitPolicy,
) -> Result<SplitAuditReport, SplitAuditError> {
    if !valid_identifier(&policy.protocol_id) {
        return Err(SplitAuditError::new(
            "invalid_protocol_id",
            policy.protocol_id.as_str(),
        ));
    }
    let manifest_digest = normalized_sha256(&policy.manifest_digest).ok_or_else(|| {
        SplitAuditError::new("invalid_manifest_digest", policy.protocol_id.as_str())
    })?;
    if policy.exclusive_dimensions.is_empty() {
        return Err(SplitAuditError::new(
            "empty_exclusive_dimensions",
            policy.protocol_id.as_str(),
        ));
    }
    if !policy
        .exclusive_dimensions
        .is_disjoint(&policy.shared_dimensions)
    {
        return Err(SplitAuditError::new(
            "overlapping_policy_dimensions",
            policy.protocol_id.as_str(),
        ));
    }
    let required_dimensions: BTreeSet<_> = policy
        .exclusive_dimensions
        .union(&policy.shared_dimensions)
        .copied()
        .collect();
    if samples.is_empty() {
        return Err(SplitAuditError::new("empty_manifest", "no samples"));
    }

    let mut ordered_samples: Vec<&BenchmarkSample> = samples.iter().collect();
    ordered_samples.sort_by(|left, right| left.sample_id.cmp(&right.sample_id));

    let mut sample_ids = BTreeSet::new();
    let mut content_digests = BTreeMap::<String, String>::new();
    let mut group_partitions =
        BTreeMap::<(LeakageDimension, String), DataPartition>::new();
    let mut shared_group_partitions =
        BTreeMap::<(LeakageDimension, String), BTreeSet<DataPartition>>::new();
    let mut group_values =
        BTreeMap::<LeakageDimension, BTreeSet<String>>::new();
    let mut partition_counts = BTreeMap::from([
        (DataPartition::Train, 0_usize),
        (DataPartition::Validation, 0_usize),
        (DataPartition::Test, 0_usize),
    ]);

    for sample in ordered_samples {
        if !valid_identifier(&sample.sample_id) {
            return Err(SplitAuditError::new(
                "invalid_sample_id",
                sample.sample_id.as_str(),
            ));
        }
        if !sample_ids.insert(sample.sample_id.as_str()) {
            return Err(SplitAuditError::new(
                "duplicate_sample_id",
                sample.sample_id.as_str(),
            ));
        }

        let digest = normalized_sha256(&sample.content_digest).ok_or_else(|| {
            SplitAuditError::new("invalid_content_digest", sample.sample_id.as_str())
        })?;
        if let Some(previous_sample) =
            content_digests.insert(digest.clone(), sample.sample_id.clone())
        {
            return Err(SplitAuditError::new(
                "duplicate_content_digest",
                format!("{previous_sample},{}", sample.sample_id),
            ));
        }

        *partition_counts
            .get_mut(&sample.partition)
            .expect("all partitions initialized") += 1;

        let mut dimensions_in_sample = BTreeSet::new();
        for group in &sample.leakage_groups {
            if !valid_identifier(&group.value) {
                return Err(SplitAuditError::new(
                    "invalid_leakage_group",
                    format!("{}:{}", sample.sample_id, group.dimension.as_str()),
                ));
            }
            if !dimensions_in_sample.insert(group.dimension) {
                return Err(SplitAuditError::new(
                    "duplicate_sample_dimension",
                    format!("{}:{}", sample.sample_id, group.dimension.as_str()),
                ));
            }
            group_values
                .entry(group.dimension)
                .or_default()
                .insert(group.value.clone());

            let key = (group.dimension, group.value.clone());
            if policy.exclusive_dimensions.contains(&group.dimension) {
                if let Some(previous_partition) =
                    group_partitions.insert(key.clone(), sample.partition)
                {
                    if previous_partition != sample.partition {
                        return Err(SplitAuditError::new(
                            "cross_partition_group_overlap",
                            format!("{}={}", key.0.as_str(), key.1),
                        ));
                    }
                }
            }
            if policy.shared_dimensions.contains(&group.dimension) {
                shared_group_partitions
                    .entry(key)
                    .or_default()
                    .insert(sample.partition);
            }
        }

        for dimension in &required_dimensions {
            if !dimensions_in_sample.contains(dimension) {
                return Err(SplitAuditError::new(
                    "missing_required_dimension",
                    format!("{}:{}", sample.sample_id, dimension.as_str()),
                ));
            }
        }
    }

    for partition in [
        DataPartition::Train,
        DataPartition::Validation,
        DataPartition::Test,
    ] {
        if partition_counts.get(&partition).copied().unwrap_or_default() == 0 {
            return Err(SplitAuditError::new(
                "empty_partition",
                partition.as_str(),
            ));
        }
    }

    let all_partitions = BTreeSet::from([
        DataPartition::Train,
        DataPartition::Validation,
        DataPartition::Test,
    ]);
    for dimension in &policy.shared_dimensions {
        let common_group_exists = shared_group_partitions
            .iter()
            .any(|((group_dimension, _), partitions)| {
                group_dimension == dimension && partitions == &all_partitions
            });
        if !common_group_exists {
            return Err(SplitAuditError::new(
                "no_common_group_across_partitions",
                dimension.as_str(),
            ));
        }
    }

    let group_counts = group_values
        .into_iter()
        .map(|(dimension, values)| (dimension, values.len()))
        .collect();

    Ok(SplitAuditReport {
        protocol_id: policy.protocol_id.clone(),
        manifest_digest,
        exclusive_dimensions: policy.exclusive_dimensions.clone(),
        shared_dimensions: policy.shared_dimensions.clone(),
        sample_count: samples.len(),
        partition_counts,
        group_counts,
        distinct_content_digests: content_digests.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u64) -> String {
        format!("sha256:{value:064x}")
    }

    fn group(dimension: LeakageDimension, value: &str) -> LeakageGroup {
        LeakageGroup {
            dimension,
            value: value.to_owned(),
        }
    }

    fn sample(
        sample_id: &str,
        partition: DataPartition,
        digest_value: u64,
        event: &str,
        session: &str,
        network: &str,
    ) -> BenchmarkSample {
        BenchmarkSample {
            sample_id: sample_id.to_owned(),
            partition,
            content_digest: digest(digest_value),
            leakage_groups: vec![
                group(LeakageDimension::PhysicalEvent, event),
                group(LeakageDimension::CaptureSession, session),
                group(LeakageDimension::Network, network),
            ],
        }
    }

    fn event_session_policy() -> SplitPolicy {
        SplitPolicy {
            protocol_id: "within_network_unseen_event_v1".to_owned(),
            manifest_digest: digest(900),
            exclusive_dimensions: BTreeSet::from([
                LeakageDimension::PhysicalEvent,
                LeakageDimension::CaptureSession,
            ]),
            shared_dimensions: BTreeSet::from([LeakageDimension::Network]),
        }
    }

    fn valid_samples() -> Vec<BenchmarkSample> {
        vec![
            sample("train-a", DataPartition::Train, 1, "event-a", "session-a", "net-1"),
            sample("train-b", DataPartition::Train, 2, "event-b", "session-b", "net-1"),
            sample("validation-a", DataPartition::Validation, 3, "event-c", "session-c", "net-1"),
            sample("test-a", DataPartition::Test, 4, "event-d", "session-d", "net-1"),
        ]
    }

    #[test]
    fn within_network_protocol_allows_shared_network_but_blocks_event_and_session_leakage() {
        let report =
            audit_benchmark_split(&valid_samples(), &event_session_policy()).expect("valid split");
        assert_eq!(report.sample_count, 4);
        assert_eq!(report.partition_counts[&DataPartition::Train], 2);
        assert_eq!(report.partition_counts[&DataPartition::Validation], 1);
        assert_eq!(report.partition_counts[&DataPartition::Test], 1);
        assert_eq!(report.group_counts[&LeakageDimension::Network], 1);
    }

    #[test]
    fn cross_network_protocol_rejects_a_shared_network() {
        let mut policy = event_session_policy();
        policy.exclusive_dimensions.insert(LeakageDimension::Network);
        policy
            .shared_dimensions
            .remove(&LeakageDimension::Network);
        let error =
            audit_benchmark_split(&valid_samples(), &policy).expect_err("network overlap");
        assert_eq!(error.code, "cross_partition_group_overlap");
        assert_eq!(error.subject, "network=net-1");
    }

    #[test]
    fn cross_partition_physical_event_reuse_fails_closed() {
        let mut samples = valid_samples();
        samples[3].leakage_groups[0].value = "event-a".to_owned();
        let error = audit_benchmark_split(&samples, &event_session_policy())
            .expect_err("event reuse must fail");
        assert_eq!(error.code, "cross_partition_group_overlap");
        assert_eq!(error.subject, "physical_event=event-a");
    }

    #[test]
    fn exact_content_digest_reuse_across_distinct_samples_fails_closed() {
        let mut samples = valid_samples();
        samples[3].content_digest = samples[0].content_digest.clone();
        let error = audit_benchmark_split(&samples, &event_session_policy())
            .expect_err("duplicate payload must fail");
        assert_eq!(error.code, "duplicate_content_digest");
    }

    #[test]
    fn repeated_group_values_within_one_partition_are_allowed() {
        let mut samples = valid_samples();
        samples[1].leakage_groups[0].value = "event-a".to_owned();
        let report =
            audit_benchmark_split(&samples, &event_session_policy()).expect("same partition");
        assert_eq!(report.partition_counts[&DataPartition::Train], 2);
    }

    #[test]
    fn missing_required_group_is_rejected() {
        let mut samples = valid_samples();
        samples[2]
            .leakage_groups
            .retain(|group| group.dimension != LeakageDimension::CaptureSession);
        let error = audit_benchmark_split(&samples, &event_session_policy())
            .expect_err("missing capture session");
        assert_eq!(error.code, "missing_required_dimension");
    }

    #[test]
    fn repeated_dimension_in_one_sample_is_rejected() {
        let mut samples = valid_samples();
        samples[0]
            .leakage_groups
            .push(group(LeakageDimension::PhysicalEvent, "event-other"));
        let error = audit_benchmark_split(&samples, &event_session_policy())
            .expect_err("duplicate dimension");
        assert_eq!(error.code, "duplicate_sample_dimension");
    }

    #[test]
    fn every_partition_must_be_populated() {
        let mut samples = valid_samples();
        samples.retain(|sample| sample.partition != DataPartition::Validation);
        let error = audit_benchmark_split(&samples, &event_session_policy())
            .expect_err("empty validation partition");
        assert_eq!(error.code, "empty_partition");
        assert_eq!(error.subject, "validation");
    }

    #[test]
    fn duplicate_sample_ids_are_rejected() {
        let mut samples = valid_samples();
        samples[1].sample_id = samples[0].sample_id.clone();
        let error = audit_benchmark_split(&samples, &event_session_policy())
            .expect_err("duplicate sample id");
        assert_eq!(error.code, "duplicate_sample_id");
    }

    #[test]
    fn invalid_digest_syntax_is_rejected() {
        let mut samples = valid_samples();
        samples[0].content_digest = "sha256:00".to_owned();
        let error = audit_benchmark_split(&samples, &event_session_policy())
            .expect_err("malformed digest");
        assert_eq!(error.code, "invalid_content_digest");
    }

    #[test]
    fn whitespace_padded_identifiers_are_rejected() {
        let mut samples = valid_samples();
        samples[0].leakage_groups[0].value = " event-a".to_owned();
        let error = audit_benchmark_split(&samples, &event_session_policy())
            .expect_err("padded reference");
        assert_eq!(error.code, "invalid_leakage_group");
    }

    #[test]
    fn empty_policy_cannot_claim_an_audited_protocol() {
        let policy = SplitPolicy {
            protocol_id: "unconstrained".to_owned(),
            manifest_digest: digest(900),
            exclusive_dimensions: BTreeSet::new(),
            shared_dimensions: BTreeSet::new(),
        };
        let error =
            audit_benchmark_split(&valid_samples(), &policy).expect_err("empty policy");
        assert_eq!(error.code, "empty_exclusive_dimensions");
    }

    #[test]
    fn canonical_report_is_invariant_to_sample_order() {
        let samples = valid_samples();
        let first =
            audit_benchmark_split(&samples, &event_session_policy()).expect("valid split");
        let mut reversed = samples;
        reversed.reverse();
        let second =
            audit_benchmark_split(&reversed, &event_session_policy()).expect("permuted split");
        assert_eq!(first, second);
        assert_eq!(first.canonical_text(), second.canonical_text());
    }

    #[test]
    fn exact_digest_reuse_is_rejected_even_when_groups_differ() {
        let mut samples = valid_samples();
        let source_hex = samples[0].content_digest["sha256:".len()..].to_owned();
        samples[3].content_digest = format!("sha256:{}", source_hex.to_ascii_uppercase());
        let error = audit_benchmark_split(&samples, &event_session_policy())
            .expect_err("digest comparison is case insensitive");
        assert_eq!(error.code, "duplicate_content_digest");
    }

    #[test]
    fn invalid_protocol_and_empty_manifest_fail_closed() {
        let mut policy = event_session_policy();
        policy.protocol_id = "bad protocol".to_owned();
        assert_eq!(
            audit_benchmark_split(&valid_samples(), &policy)
                .expect_err("invalid protocol")
                .code,
            "invalid_protocol_id"
        );

        assert_eq!(
            audit_benchmark_split(&[], &event_session_policy())
                .expect_err("empty manifest")
                .code,
            "empty_manifest"
        );
    }
}
