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
    network: &str,
    topology: &str,
) -> BenchmarkSample {
    BenchmarkSample {
        sample_id: sample_id.to_owned(),
        partition,
        content_digest: digest(digest_value),
        leakage_groups: vec![
            group(
                LeakageDimension::PhysicalEvent,
                &format!("event-{sample_id}"),
            ),
            group(
                LeakageDimension::CaptureSession,
                &format!("session-{sample_id}"),
            ),
            group(LeakageDimension::PipeAsset, &format!("pipe-{sample_id}")),
            group(LeakageDimension::Network, network),
            group(LeakageDimension::TopologyFamily, topology),
            group(
                LeakageDimension::ScenarioTemplate,
                &format!("template-{sample_id}"),
            ),
            group(
                LeakageDimension::SensorLayout,
                &format!("sensors-{sample_id}"),
            ),
            group(
                LeakageDimension::HydraulicModel,
                &format!("model-{sample_id}"),
            ),
            group(
                LeakageDimension::DemandProfile,
                &format!("demand-{sample_id}"),
            ),
            group(LeakageDimension::TimeBlock, &format!("time-{sample_id}")),
        ],
    }
}

fn policy(protocol: SplitProtocolPreset) -> SplitPolicy {
    SplitPolicy {
        protocol,
        manifest_digest: digest(900),
    }
}

fn within_network_policy() -> SplitPolicy {
    policy(SplitProtocolPreset::WithinNetworkUnseenEvent)
}

fn valid_within_network_samples() -> Vec<BenchmarkSample> {
    vec![
        sample("train-a", DataPartition::Train, 1, "net-1", "topology-1"),
        sample("train-b", DataPartition::Train, 2, "net-1", "topology-1"),
        sample(
            "validation-a",
            DataPartition::Validation,
            3,
            "net-1",
            "topology-1",
        ),
        sample("test-a", DataPartition::Test, 4, "net-1", "topology-1"),
    ]
}

#[test]
fn within_network_protocol_allows_common_network_and_topology() {
    let report = audit_benchmark_split(&valid_within_network_samples(), &within_network_policy())
        .expect("valid within-network split");
    assert_eq!(report.sample_count, 4);
    assert_eq!(report.partition_counts[&DataPartition::Train], 2);
    assert_eq!(report.partition_counts[&DataPartition::Validation], 1);
    assert_eq!(report.partition_counts[&DataPartition::Test], 1);
    assert_eq!(report.group_counts[&LeakageDimension::Network], 1);
    assert_eq!(report.group_counts[&LeakageDimension::TopologyFamily], 1);
    assert_eq!(
        report.exclusive_dimensions,
        within_network_policy().protocol.exclusive_dimensions()
    );
    assert_eq!(
        report.shared_dimensions,
        within_network_policy().protocol.shared_dimensions()
    );
    assert!(report.canonical_text().contains(&format!(
        "manifest_digest={}",
        within_network_policy().manifest_digest
    )));
}

#[test]
fn cross_network_protocol_rejects_a_shared_network() {
    let error = audit_benchmark_split(
        &valid_within_network_samples(),
        &policy(SplitProtocolPreset::CrossNetwork),
    )
    .expect_err("network overlap must fail");
    assert_eq!(error.code, "cross_partition_group_overlap");
    assert_eq!(error.subject, "network=net-1");
}

#[test]
fn cross_network_protocol_rejects_test_only_topology_family() {
    let samples = vec![
        sample("train-a", DataPartition::Train, 1, "net-1", "topology-1"),
        sample(
            "validation-a",
            DataPartition::Validation,
            2,
            "net-2",
            "topology-1",
        ),
        sample("test-a", DataPartition::Test, 3, "net-3", "topology-2"),
    ];
    let error = audit_benchmark_split(&samples, &policy(SplitProtocolPreset::CrossNetwork))
        .expect_err("network transfer must hold topology family common");
    assert_eq!(error.code, "shared_group_not_in_all_partitions");
    assert_eq!(error.subject, "topology_family");
}

#[test]
fn cross_network_protocol_accepts_disjoint_networks() {
    let samples = vec![
        sample("train-a", DataPartition::Train, 1, "net-1", "topology-1"),
        sample(
            "validation-a",
            DataPartition::Validation,
            2,
            "net-2",
            "topology-1",
        ),
        sample("test-a", DataPartition::Test, 3, "net-3", "topology-1"),
    ];
    let report = audit_benchmark_split(&samples, &policy(SplitProtocolPreset::CrossNetwork))
        .expect("disjoint networks");
    assert_eq!(report.partition_counts[&DataPartition::Test], 1);
}

#[test]
fn within_network_protocol_rejects_a_test_only_network() {
    let mut samples = valid_within_network_samples();
    samples[3]
        .leakage_groups
        .iter_mut()
        .find(|group| group.dimension == LeakageDimension::Network)
        .expect("network identity")
        .value = "net-2".to_owned();
    let error = audit_benchmark_split(&samples, &within_network_policy())
        .expect_err("within-network split requires common network");
    assert_eq!(error.code, "shared_group_not_in_all_partitions");
    assert_eq!(error.subject, "network");
}

#[test]
fn within_network_protocol_rejects_extra_networks_missing_from_any_partition() {
    let mut samples = valid_within_network_samples();
    samples.push(sample(
        "test-b",
        DataPartition::Test,
        5,
        "net-2",
        "topology-1",
    ));
    let error = audit_benchmark_split(&samples, &within_network_policy())
        .expect_err("every shared network group must occur in all partitions");
    assert_eq!(error.code, "shared_group_not_in_all_partitions");
    assert_eq!(error.subject, "network");
}

#[test]
fn scenario_template_reuse_across_partitions_fails_closed() {
    let mut samples = valid_within_network_samples();
    let train_template = samples[0]
        .leakage_groups
        .iter()
        .find(|group| group.dimension == LeakageDimension::ScenarioTemplate)
        .expect("train template")
        .value
        .clone();
    samples[3]
        .leakage_groups
        .iter_mut()
        .find(|group| group.dimension == LeakageDimension::ScenarioTemplate)
        .expect("test template")
        .value = train_template.clone();
    let error = audit_benchmark_split(&samples, &within_network_policy())
        .expect_err("scenario template reuse must fail");
    assert_eq!(error.code, "cross_partition_group_overlap");
    assert_eq!(error.subject, format!("scenario_template={train_template}"));
}

#[test]
fn cross_partition_physical_event_reuse_fails_closed() {
    let mut samples = valid_within_network_samples();
    samples[3].leakage_groups[0].value = "event-train-a".to_owned();
    let error = audit_benchmark_split(&samples, &within_network_policy())
        .expect_err("event reuse must fail");
    assert_eq!(error.code, "cross_partition_group_overlap");
    assert_eq!(error.subject, "physical_event=event-train-a");
}

#[test]
fn cross_topology_protocol_rejects_shared_topology_family() {
    let samples = vec![
        sample("train-a", DataPartition::Train, 1, "net-1", "topology-1"),
        sample(
            "validation-a",
            DataPartition::Validation,
            2,
            "net-2",
            "topology-1",
        ),
        sample("test-a", DataPartition::Test, 3, "net-3", "topology-1"),
    ];
    let error = audit_benchmark_split(&samples, &policy(SplitProtocolPreset::CrossTopology))
        .expect_err("topology family overlap must fail");
    assert_eq!(error.code, "cross_partition_group_overlap");
    assert_eq!(error.subject, "topology_family=topology-1");
}

#[test]
fn sensor_layout_transfer_requires_common_networks_but_disjoint_sensor_layouts() {
    let samples = valid_within_network_samples();
    let report =
        audit_benchmark_split(&samples, &policy(SplitProtocolPreset::SensorLayoutTransfer))
            .expect("same network with held-out sensor layouts");
    assert_eq!(report.group_counts[&LeakageDimension::Network], 1);
    assert_eq!(report.group_counts[&LeakageDimension::SensorLayout], 4);
}

#[test]
fn exact_content_digest_reuse_across_distinct_samples_fails_closed() {
    let mut samples = valid_within_network_samples();
    samples[3].content_digest = samples[0].content_digest.clone();
    let error =
        audit_benchmark_split(&samples, &within_network_policy()).expect_err("duplicate payload");
    assert_eq!(error.code, "duplicate_content_digest");
}

#[test]
fn repeated_group_values_within_one_partition_are_allowed() {
    let mut samples = valid_within_network_samples();
    samples[1].leakage_groups[0].value = "event-train-a".to_owned();
    let report = audit_benchmark_split(&samples, &within_network_policy())
        .expect("same-partition group reuse");
    assert_eq!(report.partition_counts[&DataPartition::Train], 2);
}

#[test]
fn missing_required_group_is_rejected() {
    let mut samples = valid_within_network_samples();
    samples[2]
        .leakage_groups
        .retain(|group| group.dimension != LeakageDimension::CaptureSession);
    let error = audit_benchmark_split(&samples, &within_network_policy())
        .expect_err("missing capture session");
    assert_eq!(error.code, "missing_required_dimension");
}

#[test]
fn repeated_dimension_in_one_sample_is_rejected() {
    let mut samples = valid_within_network_samples();
    samples[0]
        .leakage_groups
        .push(group(LeakageDimension::PhysicalEvent, "event-other"));
    let error =
        audit_benchmark_split(&samples, &within_network_policy()).expect_err("duplicate dimension");
    assert_eq!(error.code, "duplicate_sample_dimension");
}

#[test]
fn every_partition_must_be_populated() {
    let mut samples = valid_within_network_samples();
    samples.retain(|sample| sample.partition != DataPartition::Validation);
    let error = audit_benchmark_split(&samples, &within_network_policy())
        .expect_err("empty validation partition");
    assert_eq!(error.code, "empty_partition");
    assert_eq!(error.subject, "validation");
}

#[test]
fn duplicate_sample_ids_are_rejected() {
    let mut samples = valid_within_network_samples();
    samples[1].sample_id = samples[0].sample_id.clone();
    let error =
        audit_benchmark_split(&samples, &within_network_policy()).expect_err("duplicate sample ID");
    assert_eq!(error.code, "duplicate_sample_id");
}

#[test]
fn invalid_digest_syntax_is_rejected() {
    let mut samples = valid_within_network_samples();
    samples[0].content_digest = "sha256:00".to_owned();
    let error =
        audit_benchmark_split(&samples, &within_network_policy()).expect_err("malformed digest");
    assert_eq!(error.code, "invalid_content_digest");
}

#[test]
fn whitespace_padded_identifiers_are_rejected() {
    let mut samples = valid_within_network_samples();
    samples[0].leakage_groups[0].value = " event-train-a".to_owned();
    let error =
        audit_benchmark_split(&samples, &within_network_policy()).expect_err("padded reference");
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
    let samples = valid_within_network_samples();
    let first = audit_benchmark_split(&samples, &within_network_policy()).expect("valid split");
    let mut reversed = samples;
    reversed.reverse();
    let second =
        audit_benchmark_split(&reversed, &within_network_policy()).expect("permuted split");
    assert_eq!(first, second);
    assert_eq!(first.canonical_text(), second.canonical_text());
}

#[test]
fn digest_comparison_is_case_insensitive() {
    let mut samples = valid_within_network_samples();
    let source_hex = samples[0].content_digest["sha256:".len()..].to_owned();
    samples[3].content_digest = format!("sha256:{}", source_hex.to_ascii_uppercase());
    let error = audit_benchmark_split(&samples, &within_network_policy()).expect_err("same digest");
    assert_eq!(error.code, "duplicate_content_digest");
}

#[test]
fn malformed_manifest_digest_is_rejected() {
    let mut invalid_policy = within_network_policy();
    invalid_policy.manifest_digest = "sha256:00".to_owned();
    let error = audit_benchmark_split(&valid_within_network_samples(), &invalid_policy)
        .expect_err("manifest digest");
    assert_eq!(error.code, "invalid_manifest_digest");
}

#[test]
fn empty_manifest_is_rejected() {
    let error = audit_benchmark_split(&[], &within_network_policy()).expect_err("empty manifest");
    assert_eq!(error.code, "empty_manifest");
}
