// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civbench_event_evaluator::benchmark_split::{
    BenchmarkSample, DataPartition, LeakageDimension, LeakageGroup, SplitPolicy,
    SplitProtocolPreset, audit_benchmark_split,
};
use std::collections::BTreeSet;

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

fn policy() -> SplitPolicy {
    SplitPolicy {
        protocol: SplitProtocolPreset::WithinNetworkUnseenEvent,
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
        sample(
            "train-a",
            DataPartition::Train,
            1,
            "event-a",
            "session-a",
            "net-1",
        ),
        sample(
            "train-b",
            DataPartition::Train,
            2,
            "event-b",
            "session-b",
            "net-1",
        ),
        sample(
            "validation-a",
            DataPartition::Validation,
            3,
            "event-c",
            "session-c",
            "net-1",
        ),
        sample(
            "test-a",
            DataPartition::Test,
            4,
            "event-d",
            "session-d",
            "net-1",
        ),
    ]
}

#[test]
fn within_network_protocol_allows_shared_network_but_blocks_event_and_session_leakage() {
    let report = audit_benchmark_split(&valid_samples(), &policy()).expect("valid split");
    assert_eq!(report.sample_count, 4);
    assert_eq!(report.partition_counts[&DataPartition::Train], 2);
    assert_eq!(report.partition_counts[&DataPartition::Validation], 1);
    assert_eq!(report.partition_counts[&DataPartition::Test], 1);
    assert_eq!(report.group_counts[&LeakageDimension::Network], 1);
    assert_eq!(
        report.exclusive_dimensions,
        BTreeSet::from([
            LeakageDimension::PhysicalEvent,
            LeakageDimension::CaptureSession,
        ])
    );
    assert_eq!(
        report.shared_dimensions,
        BTreeSet::from([LeakageDimension::Network])
    );
    assert!(
        report
            .canonical_text()
            .contains(&format!("manifest_digest={}", policy().manifest_digest))
    );
}

#[test]
fn cross_network_protocol_rejects_a_shared_network() {
    let mut cross_network = policy();
    cross_network.protocol = SplitProtocolPreset::CrossNetwork;
    cross_network.protocol_id = cross_network.protocol.as_str().to_owned();
    cross_network
        .exclusive_dimensions
        .insert(LeakageDimension::Network);
    cross_network
        .shared_dimensions
        .remove(&LeakageDimension::Network);
    let error =
        audit_benchmark_split(&valid_samples(), &cross_network).expect_err("network overlap");
    assert_eq!(error.code, "cross_partition_group_overlap");
    assert_eq!(error.subject, "network=net-1");
}

#[test]
fn within_network_protocol_rejects_a_test_only_network() {
    let mut samples = valid_samples();
    if let Some(network) = samples[3]
        .leakage_groups
        .iter_mut()
        .find(|group| group.dimension == LeakageDimension::Network)
    {
        network.value = "net-2".to_owned();
    }
    let error = audit_benchmark_split(&samples, &policy())
        .expect_err("within-network split requires a common network");
    assert_eq!(error.code, "shared_group_not_in_all_partitions");
    assert_eq!(error.subject, "network");
}

#[test]
fn within_network_protocol_rejects_extra_networks_missing_from_any_partition() {
    let mut samples = valid_samples();
    samples.push(sample(
        "test-b",
        DataPartition::Test,
        5,
        "event-e",
        "session-e",
        "net-2",
    ));
    let error = audit_benchmark_split(&samples, &policy())
        .expect_err("all shared network groups must exist in all partitions");
    assert_eq!(error.code, "shared_group_not_in_all_partitions");
    assert_eq!(error.subject, "network");
}

#[test]
fn a_dimension_cannot_be_both_shared_and_exclusive() {
    let mut invalid_policy = policy();
    invalid_policy
        .exclusive_dimensions
        .insert(LeakageDimension::Network);
    let error = audit_benchmark_split(&valid_samples(), &invalid_policy)
        .expect_err("policy dimension conflict");
    assert_eq!(error.code, "overlapping_policy_dimensions");
}

#[test]
fn cross_partition_physical_event_reuse_fails_closed() {
    let mut samples = valid_samples();
    samples[3].leakage_groups[0].value = "event-a".to_owned();
    let error = audit_benchmark_split(&samples, &policy()).expect_err("event reuse must fail");
    assert_eq!(error.code, "cross_partition_group_overlap");
    assert_eq!(error.subject, "physical_event=event-a");
}

#[test]
fn exact_content_digest_reuse_across_distinct_samples_fails_closed() {
    let mut samples = valid_samples();
    samples[3].content_digest = samples[0].content_digest.clone();
    let error = audit_benchmark_split(&samples, &policy()).expect_err("duplicate payload");
    assert_eq!(error.code, "duplicate_content_digest");
}

#[test]
fn repeated_group_values_within_one_partition_are_allowed() {
    let mut samples = valid_samples();
    samples[1].leakage_groups[0].value = "event-a".to_owned();
    let report = audit_benchmark_split(&samples, &policy()).expect("same partition");
    assert_eq!(report.partition_counts[&DataPartition::Train], 2);
}

#[test]
fn missing_required_group_is_rejected() {
    let mut samples = valid_samples();
    samples[2]
        .leakage_groups
        .retain(|group| group.dimension != LeakageDimension::CaptureSession);
    let error = audit_benchmark_split(&samples, &policy()).expect_err("missing capture session");
    assert_eq!(error.code, "missing_required_dimension");
}

#[test]
fn repeated_dimension_in_one_sample_is_rejected() {
    let mut samples = valid_samples();
    samples[0]
        .leakage_groups
        .push(group(LeakageDimension::PhysicalEvent, "event-other"));
    let error = audit_benchmark_split(&samples, &policy()).expect_err("duplicate dimension");
    assert_eq!(error.code, "duplicate_sample_dimension");
}

#[test]
fn every_partition_must_be_populated() {
    let mut samples = valid_samples();
    samples.retain(|sample| sample.partition != DataPartition::Validation);
    let error = audit_benchmark_split(&samples, &policy()).expect_err("empty validation");
    assert_eq!(error.code, "empty_partition");
    assert_eq!(error.subject, "validation");
}

#[test]
fn duplicate_sample_ids_are_rejected() {
    let mut samples = valid_samples();
    samples[1].sample_id = samples[0].sample_id.clone();
    let error = audit_benchmark_split(&samples, &policy()).expect_err("duplicate sample ID");
    assert_eq!(error.code, "duplicate_sample_id");
}

#[test]
fn invalid_digest_syntax_is_rejected() {
    let mut samples = valid_samples();
    samples[0].content_digest = "sha256:00".to_owned();
    let error = audit_benchmark_split(&samples, &policy()).expect_err("malformed digest");
    assert_eq!(error.code, "invalid_content_digest");
}

#[test]
fn whitespace_padded_identifiers_are_rejected() {
    let mut samples = valid_samples();
    samples[0].leakage_groups[0].value = " event-a".to_owned();
    let error = audit_benchmark_split(&samples, &policy()).expect_err("padded reference");
    assert_eq!(error.code, "invalid_leakage_group");
}

#[test]
fn empty_policy_cannot_claim_an_audited_protocol() {
    let empty_policy = SplitPolicy {
        protocol: SplitProtocolPreset::CrossNetwork,
        protocol_id: "cross_network_v1".to_owned(),
        manifest_digest: digest(900),
        exclusive_dimensions: BTreeSet::new(),
        shared_dimensions: BTreeSet::new(),
    };
    let error = audit_benchmark_split(&valid_samples(), &empty_policy).expect_err("empty policy");
    assert_eq!(error.code, "empty_exclusive_dimensions");
}

#[test]
fn a_registered_protocol_cannot_be_claimed_with_incompatible_dimensions() {
    let mut invalid_policy = policy();
    invalid_policy.protocol = SplitProtocolPreset::CrossNetwork;
    invalid_policy.protocol_id = invalid_policy.protocol.as_str().to_owned();
    let error = audit_benchmark_split(&valid_samples(), &invalid_policy)
        .expect_err("within-network dimensions cannot claim cross-network transfer");
    assert_eq!(error.code, "protocol_dimensions_mismatch");
}

#[test]
fn a_custom_protocol_id_cannot_impersonate_a_registered_protocol() {
    let mut invalid_policy = policy();
    invalid_policy.protocol_id = "cross_network_v1".to_owned();
    let error = audit_benchmark_split(&valid_samples(), &invalid_policy)
        .expect_err("preset and ID differ");
    assert_eq!(error.code, "protocol_id_preset_mismatch");
}

#[test]
fn canonical_report_is_invariant_to_sample_order() {
    let samples = valid_samples();
    let first = audit_benchmark_split(&samples, &policy()).expect("valid split");
    let mut reversed = samples;
    reversed.reverse();
    let second = audit_benchmark_split(&reversed, &policy()).expect("permuted split");
    assert_eq!(first, second);
    assert_eq!(first.canonical_text(), second.canonical_text());
}

#[test]
fn digest_comparison_is_case_insensitive() {
    let mut samples = valid_samples();
    let source_hex = samples[0].content_digest["sha256:".len()..].to_owned();
    samples[3].content_digest = format!("sha256:{}", source_hex.to_ascii_uppercase());
    let error = audit_benchmark_split(&samples, &policy()).expect_err("same digest");
    assert_eq!(error.code, "duplicate_content_digest");
}

#[test]
fn malformed_manifest_digest_is_rejected() {
    let mut invalid_policy = policy();
    invalid_policy.manifest_digest = "sha256:00".to_owned();
    let error =
        audit_benchmark_split(&valid_samples(), &invalid_policy).expect_err("manifest digest");
    assert_eq!(error.code, "invalid_manifest_digest");
}

#[test]
fn invalid_protocol_and_empty_manifest_fail_closed() {
    let mut invalid_policy = policy();
    invalid_policy.protocol_id = "bad protocol".to_owned();
    assert_eq!(
        audit_benchmark_split(&valid_samples(), &invalid_policy)
            .expect_err("invalid protocol")
            .code,
        "invalid_protocol_id"
    );

    assert_eq!(
        audit_benchmark_split(&[], &policy())
            .expect_err("empty manifest")
            .code,
        "empty_manifest"
    );
}
