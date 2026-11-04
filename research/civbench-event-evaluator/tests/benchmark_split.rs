// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civbench_event_evaluator::benchmark_split::{
    BenchmarkSample, DataPartition, LeakageDimension, LeakageGroup, SplitPolicy,
    SplitProtocolPreset, audit_benchmark_split,
};

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

fn policy_for(protocol: SplitProtocolPreset) -> SplitPolicy {
    SplitPolicy {
        protocol,
        manifest_digest: digest(900),
    }
}

fn policy() -> SplitPolicy {
    policy_for(SplitProtocolPreset::WithinNetworkUnseenEvent)
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
        policy().protocol.exclusive_dimensions()
    );
    assert_eq!(
        report.shared_dimensions,
        policy().protocol.shared_dimensions()
    );
    assert!(
        report
            .canonical_text()
            .contains(&format!("manifest_digest={}", policy().manifest_digest))
    );
}

#[test]
fn cross_network_protocol_rejects_a_shared_network() {
    let cross_network = policy_for(SplitProtocolPreset::CrossNetwork);
    let error = audit_benchmark_split(&valid_samples(), &cross_network)
        .expect_err("network overlap must fail");
    assert_eq!(error.code, "cross_partition_group_overlap");
    assert_eq!(error.subject, "network=net-1");
}

#[test]
fn within_network_protocol_rejects_a_test_only_network() {
    let mut samples = valid_samples();
    samples[3]
        .leakage_groups
        .iter_mut()
        .find(|group| group.dimension == LeakageDimension::Network)
        .expect("network identity")
        .value = "net-2".to_owned();
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
    let report = audit_benchmark_split(&samples, &policy()).expect("same-partition group reuse");
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
fn each_registered_preset_has_an_immutable_protocol_identity_and_firewall() {
    let presets = [
        SplitProtocolPreset::WithinNetworkUnseenEvent,
        SplitProtocolPreset::NewOperatingRegime,
        SplitProtocolPreset::CrossNetwork,
        SplitProtocolPreset::CrossTopology,
        SplitProtocolPreset::SensorLayoutTransfer,
        SplitProtocolPreset::AssetTransfer,
    ];
    for preset in presets {
        let exclusive = preset.exclusive_dimensions();
        let shared = preset.shared_dimensions();
        assert!(!exclusive.is_empty(), "{}", preset.as_str());
        assert!(exclusive.is_disjoint(&shared), "{}", preset.as_str());
    }

    assert!(
        SplitProtocolPreset::CrossNetwork
            .exclusive_dimensions()
            .contains(&LeakageDimension::Network)
    );
    assert!(
        SplitProtocolPreset::CrossTopology
            .exclusive_dimensions()
            .contains(&LeakageDimension::TopologyFamily)
    );
    assert!(
        SplitProtocolPreset::NewOperatingRegime
            .exclusive_dimensions()
            .contains(&LeakageDimension::DemandProfile)
    );
    assert!(
        SplitProtocolPreset::SensorLayoutTransfer
            .exclusive_dimensions()
            .contains(&LeakageDimension::SensorLayout)
    );
    assert!(
        SplitProtocolPreset::AssetTransfer
            .exclusive_dimensions()
            .contains(&LeakageDimension::PipeAsset)
    );
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
fn empty_manifest_is_rejected() {
    let error = audit_benchmark_split(&[], &policy()).expect_err("empty manifest");
    assert_eq!(error.code, "empty_manifest");
}
