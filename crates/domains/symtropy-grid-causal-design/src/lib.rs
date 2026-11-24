// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Pre-registration and cluster-randomization audit for grid-flexibility causal M&V.
//!
//! This crate validates a randomized assignment plan before outcome data exist.
//! It is not a causal-effect estimator. It does not turn a historical baseline
//! forecast into a causal effect, and it does not grant dispatch or settlement
//! authority.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: &str = "0.1.0";
const BENCHMARK_ID: &str = "GRID-CAUSAL-DESIGN-AUDIT-001";
const RANDOMIZATION_ALGORITHM: &str = "sha256_rank_v1";
const CLAIM_CEILING: &str = "Pre-registration and deterministic cluster-randomization audit only. A passing design does not estimate an effect, prove no interference beyond the declared mapping, authenticate the external random-beacon source, establish transportability, qualify a grid product, authorize dispatch, or create settlement/payment authority.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DesignClass {
    ClusterRandomizedParallel,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrimaryEstimand {
    IntentionToTreat,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentArm {
    Treatment,
    Control,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TimeWindow {
    pub start_unix_ms: u64,
    pub end_unix_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RandomizationReceipt {
    pub algorithm: String,
    pub seed_commitment_digest: String,
    pub seed_reveal: String,
    pub seed_commitment_recorded_at_unix_ms: u64,
    pub public_randomness_source_id: String,
    pub public_randomness_value: String,
    pub public_randomness_digest: String,
    pub randomness_published_at_unix_ms: u64,
    pub assignment_issued_at_unix_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StratumAllocation {
    pub stratum_id: String,
    pub treated_cluster_count: usize,
    pub control_cluster_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClusterAssignment {
    pub cluster_id: String,
    pub stratum_id: String,
    /// The largest predeclared interference component used as the assignment
    /// boundary (for example, a depot-feeder island).
    pub interference_domain_id: String,
    pub assigned_arm: AssignmentArm,
    pub site_ids: Vec<String>,
    pub asset_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GridCausalDesignInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub experiment_id: String,
    pub design_class: DesignClass,
    pub primary_estimand: PrimaryEstimand,
    pub service_product_id: String,
    pub primary_outcome_id: String,
    pub measurement_boundary_id: String,
    pub protocol_reference: String,
    pub pre_registration_digest: String,
    pub frozen_covariate_digest: String,
    pub baseline_partition_digest: String,
    pub interference_mapping_digest: String,
    pub minimum_clusters_per_arm: usize,
    pub baseline_window: TimeWindow,
    pub pretrend_windows: Vec<TimeWindow>,
    pub placebo_windows: Vec<TimeWindow>,
    pub dispatch_window: TimeWindow,
    pub recovery_window: TimeWindow,
    pub negative_control_outcome_ids: Vec<String>,
    pub plan_frozen_at_unix_ms: u64,
    pub randomization: RandomizationReceipt,
    pub strata: Vec<StratumAllocation>,
    pub clusters: Vec<ClusterAssignment>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StratumAudit {
    pub stratum_id: String,
    pub treated_cluster_count: usize,
    pub control_cluster_count: usize,
    pub expected_treated_cluster_count: usize,
    pub expected_control_cluster_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GridCausalDesignAuditReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub input_digest: String,
    pub experiment_id: String,
    pub design_class: DesignClass,
    pub primary_estimand: PrimaryEstimand,
    pub design_qualified: bool,
    pub cluster_count: usize,
    pub site_count: usize,
    pub asset_count: usize,
    pub treated_cluster_count: usize,
    pub control_cluster_count: usize,
    pub stratum_count: usize,
    pub minimum_clusters_per_arm: usize,
    pub randomization_algorithm: String,
    pub randomization_assignment_digest: String,
    pub randomization_reproducible: bool,
    pub declared_pre_registration_precedes_randomization: bool,
    pub declared_public_randomness_published_after_freeze: bool,
    pub interference_domain_ids_unique_in_manifest: bool,
    pub pre_registration_document_authenticated: bool,
    pub interference_mapping_authenticated: bool,
    pub declared_timestamps_attested: bool,
    pub pretrend_windows_predeclared: bool,
    pub placebo_windows_predeclared: bool,
    pub negative_control_outcomes_predeclared: bool,
    pub cluster_level_inference_required: bool,
    pub external_randomness_source_authenticated: bool,
    pub causal_effect_estimated: bool,
    pub treatment_on_treated_identified: bool,
    pub settlement_authority_granted: bool,
    pub payment_authorized: bool,
    pub dispatch_authority_granted: bool,
    pub strata: Vec<StratumAudit>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesignAuditError {
    InvalidInput(String),
    MissingEvidence(String),
    InvalidDigest(String),
    InvalidTimeWindow(String),
    TimeOrderViolation(String),
    DuplicateIdentity(String),
    InterferenceDomainOverlap(String),
    StratumMismatch(String),
    RandomizationMismatch(String),
    InsufficientClusters(String),
    UnsupportedDesign(String),
}

impl core::fmt::Display for DesignAuditError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput(message)
            | Self::MissingEvidence(message)
            | Self::InvalidDigest(message)
            | Self::InvalidTimeWindow(message)
            | Self::TimeOrderViolation(message)
            | Self::DuplicateIdentity(message)
            | Self::InterferenceDomainOverlap(message)
            | Self::StratumMismatch(message)
            | Self::RandomizationMismatch(message)
            | Self::InsufficientClusters(message)
            | Self::UnsupportedDesign(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for DesignAuditError {}

fn require_nonempty(field: &str, value: &str) -> Result<(), DesignAuditError> {
    if value.trim().is_empty() {
        Err(DesignAuditError::MissingEvidence(format!(
            "{field} must not be empty"
        )))
    } else {
        Ok(())
    }
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn canonical_digest<T: Serialize>(value: &T, label: &str) -> Result<String, DesignAuditError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| DesignAuditError::InvalidInput(format!("{label} serialization failed: {error}")))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_digest(field: &str, digest: &str) -> Result<(), DesignAuditError> {
    if valid_sha256(digest) {
        Ok(())
    } else {
        Err(DesignAuditError::InvalidDigest(format!(
            "{field} must be sha256:<64 hexadecimal characters>"
        )))
    }
}

fn validate_window(name: &str, window: &TimeWindow) -> Result<(), DesignAuditError> {
    if window.start_unix_ms == 0 || window.end_unix_ms <= window.start_unix_ms {
        return Err(DesignAuditError::InvalidTimeWindow(format!(
            "{name} must be a non-empty positive interval"
        )));
    }
    Ok(())
}

fn overlaps(left: &TimeWindow, right: &TimeWindow) -> bool {
    left.start_unix_ms < right.end_unix_ms && right.start_unix_ms < left.end_unix_ms
}

fn sha256_hex(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn randomization_rank(input: &GridCausalDesignInput, stratum_id: &str, cluster_id: &str) -> String {
    let material = format!(
        "{}|{}|{}|{}|{}",
        input.experiment_id,
        input.randomization.seed_reveal,
        input.randomization.public_randomness_value,
        stratum_id,
        cluster_id
    );
    sha256_hex(&material)
}

fn assignment_digest(
    input: &GridCausalDesignInput,
) -> Result<String, DesignAuditError> {
    let mut records: Vec<_> = input.clusters.iter().collect();
    records.sort_by(|left, right| left.cluster_id.cmp(&right.cluster_id));
    canonical_digest(&records, "cluster assignment records")
}

fn validate_input(input: &GridCausalDesignInput) -> Result<(), DesignAuditError> {
    if input.schema_version != SCHEMA_VERSION || input.benchmark_id != BENCHMARK_ID {
        return Err(DesignAuditError::InvalidInput(
            "schema_version or benchmark_id mismatch".to_owned(),
        ));
    }

    for (field, value) in [
        ("experiment_id", input.experiment_id.as_str()),
        ("service_product_id", input.service_product_id.as_str()),
        ("primary_outcome_id", input.primary_outcome_id.as_str()),
        ("measurement_boundary_id", input.measurement_boundary_id.as_str()),
        ("protocol_reference", input.protocol_reference.as_str()),
        ("claim_ceiling", input.claim_ceiling.as_str()),
        (
            "public_randomness_source_id",
            input.randomization.public_randomness_source_id.as_str(),
        ),
        ("random seed reveal", input.randomization.seed_reveal.as_str()),
        (
            "public randomness value",
            input.randomization.public_randomness_value.as_str(),
        ),
    ] {
        require_nonempty(field, value)?;
    }

    for (field, digest) in [
        ("pre-registration digest", input.pre_registration_digest.as_str()),
        ("frozen covariate digest", input.frozen_covariate_digest.as_str()),
        ("baseline partition digest", input.baseline_partition_digest.as_str()),
        (
            "interference mapping digest",
            input.interference_mapping_digest.as_str(),
        ),
        (
            "randomization seed commitment",
            input.randomization.seed_commitment_digest.as_str(),
        ),
        (
            "public randomness digest",
            input.randomization.public_randomness_digest.as_str(),
        ),
    ] {
        validate_digest(field, digest)?;
    }

    if input.randomization.algorithm != RANDOMIZATION_ALGORITHM {
        return Err(DesignAuditError::UnsupportedDesign(format!(
            "unsupported randomization algorithm: {}",
            input.randomization.algorithm
        )));
    }
    if input.minimum_clusters_per_arm < 2 {
        return Err(DesignAuditError::InsufficientClusters(
            "minimum_clusters_per_arm must be at least two; final sample-size/power justification remains external"
                .to_owned(),
        ));
    }
    if input.strata.is_empty() || input.clusters.is_empty() {
        return Err(DesignAuditError::InvalidInput(
            "strata and clusters must not be empty".to_owned(),
        ));
    }
    if input.pretrend_windows.len() < 2 {
        return Err(DesignAuditError::InvalidInput(
            "at least two pretrend windows must be preregistered".to_owned(),
        ));
    }
    if input.placebo_windows.is_empty() {
        return Err(DesignAuditError::InvalidInput(
            "at least one placebo window must be preregistered".to_owned(),
        ));
    }
    if input.negative_control_outcome_ids.is_empty() {
        return Err(DesignAuditError::InvalidInput(
            "at least one negative-control outcome must be preregistered".to_owned(),
        ));
    }

    validate_window("baseline window", &input.baseline_window)?;
    validate_window("dispatch window", &input.dispatch_window)?;
    validate_window("recovery window", &input.recovery_window)?;
    if input.recovery_window.start_unix_ms != input.dispatch_window.end_unix_ms {
        return Err(DesignAuditError::InvalidTimeWindow(
            "recovery window must begin exactly when the dispatch window ends".to_owned(),
        ));
    }
    if input.plan_frozen_at_unix_ms == 0
        || input.randomization.seed_commitment_recorded_at_unix_ms
            > input.plan_frozen_at_unix_ms
        || input.baseline_window.end_unix_ms > input.plan_frozen_at_unix_ms
        || input.plan_frozen_at_unix_ms >= input.randomization.randomness_published_at_unix_ms
        || input.randomization.randomness_published_at_unix_ms
            > input.randomization.assignment_issued_at_unix_ms
        || input.randomization.assignment_issued_at_unix_ms
            >= input.dispatch_window.start_unix_ms
    {
        return Err(DesignAuditError::TimeOrderViolation(
            "baseline/seed commitment must precede freeze; public randomness follows freeze; assignment follows randomness and precedes dispatch"
                .to_owned(),
        ));
    }
    if sha256_hex(&input.randomization.seed_reveal)
        != input
            .randomization
            .seed_commitment_digest
            .strip_prefix("sha256:")
            .unwrap_or_default()
    {
        return Err(DesignAuditError::RandomizationMismatch(
            "revealed randomization seed does not match its pre-registration commitment"
                .to_owned(),
        ));
    }
    if sha256_hex(&input.randomization.public_randomness_value)
        != input
            .randomization
            .public_randomness_digest
            .strip_prefix("sha256:")
            .unwrap_or_default()
    {
        return Err(DesignAuditError::RandomizationMismatch(
            "public randomness value does not match its declared digest".to_owned(),
        ));
    }

    for (index, window) in input.pretrend_windows.iter().enumerate() {
        validate_window(&format!("pretrend window {index}"), window)?;
        if window.end_unix_ms > input.plan_frozen_at_unix_ms
            || overlaps(window, &input.dispatch_window)
            || overlaps(window, &input.recovery_window)
        {
            return Err(DesignAuditError::TimeOrderViolation(format!(
                "pretrend window {index} must be pre-freeze and outside treatment/recovery"
            )));
        }
    }
    for (index, window) in input.placebo_windows.iter().enumerate() {
        validate_window(&format!("placebo window {index}"), window)?;
        if window.end_unix_ms > input.plan_frozen_at_unix_ms
            || overlaps(window, &input.dispatch_window)
            || overlaps(window, &input.recovery_window)
        {
            return Err(DesignAuditError::TimeOrderViolation(format!(
                "placebo window {index} must be pre-freeze and outside treatment/recovery"
            )));
        }
    }

    // Baseline training, pretrend checks and placebo windows are deliberately
    // disjoint. Reusing the same observations for all three would disguise
    // leakage and make negative controls look stronger than they are.
    let mut diagnostic_windows: Vec<(&str, &TimeWindow)> =
        vec![("baseline", &input.baseline_window)];
    diagnostic_windows.extend(
        input
            .pretrend_windows
            .iter()
            .map(|window| ("pretrend", window)),
    );
    diagnostic_windows.extend(
        input
            .placebo_windows
            .iter()
            .map(|window| ("placebo", window)),
    );
    for (left_index, (left_name, left_window)) in diagnostic_windows.iter().enumerate() {
        for (right_name, right_window) in diagnostic_windows.iter().skip(left_index + 1) {
            if overlaps(left_window, right_window) {
                return Err(DesignAuditError::InvalidTimeWindow(format!(
                    "{left_name} and {right_name} windows must not overlap"
                )));
            }
        }
    }

    let mut stratum_ids = BTreeSet::new();
    let mut allocations = BTreeMap::new();
    for stratum in &input.strata {
        require_nonempty("stratum ID", &stratum.stratum_id)?;
        if stratum.treated_cluster_count == 0 || stratum.control_cluster_count == 0 {
            return Err(DesignAuditError::StratumMismatch(format!(
                "stratum {} must allocate at least one cluster to each arm",
                stratum.stratum_id
            )));
        }
        if !stratum_ids.insert(stratum.stratum_id.as_str()) {
            return Err(DesignAuditError::DuplicateIdentity(format!(
                "duplicate stratum ID: {}",
                stratum.stratum_id
            )));
        }
        allocations.insert(
            stratum.stratum_id.as_str(),
            (stratum.treated_cluster_count, stratum.control_cluster_count),
        );
    }

    let mut cluster_ids = BTreeSet::new();
    let mut interference_domains = BTreeSet::new();
    let mut sites = BTreeSet::new();
    let mut assets = BTreeSet::new();
    let mut observed_counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();

    for cluster in &input.clusters {
        for (field, value) in [
            ("cluster ID", cluster.cluster_id.as_str()),
            ("cluster stratum ID", cluster.stratum_id.as_str()),
            ("interference domain ID", cluster.interference_domain_id.as_str()),
        ] {
            require_nonempty(field, value)?;
        }
        if !cluster_ids.insert(cluster.cluster_id.as_str()) {
            return Err(DesignAuditError::DuplicateIdentity(format!(
                "duplicate cluster ID: {}",
                cluster.cluster_id
            )));
        }
        if !interference_domains.insert(cluster.interference_domain_id.as_str()) {
            return Err(DesignAuditError::InterferenceDomainOverlap(format!(
                "interference domain {} spans more than one randomized cluster; merge those units before randomization",
                cluster.interference_domain_id
            )));
        }
        if !allocations.contains_key(cluster.stratum_id.as_str()) {
            return Err(DesignAuditError::StratumMismatch(format!(
                "cluster {} references unknown stratum {}",
                cluster.cluster_id, cluster.stratum_id
            )));
        }
        if cluster.site_ids.is_empty() || cluster.asset_ids.is_empty() {
            return Err(DesignAuditError::MissingEvidence(format!(
                "cluster {} must bind at least one site and one asset",
                cluster.cluster_id
            )));
        }
        for site in &cluster.site_ids {
            require_nonempty("site ID", site)?;
            if !sites.insert(site.as_str()) {
                return Err(DesignAuditError::DuplicateIdentity(format!(
                    "site {site} is assigned to more than one randomized cluster"
                )));
            }
        }
        for asset in &cluster.asset_ids {
            require_nonempty("asset ID", asset)?;
            if !assets.insert(asset.as_str()) {
                return Err(DesignAuditError::DuplicateIdentity(format!(
                    "asset {asset} is assigned to more than one randomized cluster"
                )));
            }
        }
        let entry = observed_counts.entry(cluster.stratum_id.as_str()).or_insert((0, 0));
        match cluster.assigned_arm {
            AssignmentArm::Treatment => entry.0 += 1,
            AssignmentArm::Control => entry.1 += 1,
        }
    }

    if allocations.len() != observed_counts.len() {
        return Err(DesignAuditError::StratumMismatch(
            "every declared stratum must contain at least one cluster".to_owned(),
        ));
    }
    for (stratum_id, expected) in &allocations {
        if observed_counts.get(stratum_id).copied() != Some(*expected) {
            return Err(DesignAuditError::StratumMismatch(format!(
                "stratum {stratum_id} observed arm counts do not match the preregistered allocation"
            )));
        }
    }

    let mut expected_arm_by_cluster = BTreeMap::new();
    for stratum in &input.strata {
        let mut ranked: Vec<_> = input
            .clusters
            .iter()
            .filter(|cluster| cluster.stratum_id == stratum.stratum_id)
            .map(|cluster| {
                (
                    randomization_rank(input, &stratum.stratum_id, &cluster.cluster_id),
                    cluster.cluster_id.as_str(),
                )
            })
            .collect();
        ranked.sort();
        for (position, (_, cluster_id)) in ranked.iter().enumerate() {
            let expected_arm = if position < stratum.treated_cluster_count {
                AssignmentArm::Treatment
            } else {
                AssignmentArm::Control
            };
            expected_arm_by_cluster.insert(*cluster_id, expected_arm);
        }
    }
    for cluster in &input.clusters {
        if expected_arm_by_cluster.get(cluster.cluster_id.as_str())
            != Some(&cluster.assigned_arm)
        {
            return Err(DesignAuditError::RandomizationMismatch(format!(
                "cluster {} assignment does not reproduce from the committed seed and public random beacon",
                cluster.cluster_id
            )));
        }
    }

    let treatment_count = input
        .clusters
        .iter()
        .filter(|cluster| cluster.assigned_arm == AssignmentArm::Treatment)
        .count();
    let control_count = input.clusters.len() - treatment_count;
    if treatment_count < input.minimum_clusters_per_arm
        || control_count < input.minimum_clusters_per_arm
    {
        return Err(DesignAuditError::InsufficientClusters(format!(
            "observed treatment/control cluster counts {treatment_count}/{control_count} do not meet minimum {} per arm",
            input.minimum_clusters_per_arm
        )));
    }

    let mut negative_control_ids = BTreeSet::new();
    for outcome_id in &input.negative_control_outcome_ids {
        require_nonempty("negative-control outcome ID", outcome_id)?;
        if !negative_control_ids.insert(outcome_id.as_str()) {
            return Err(DesignAuditError::DuplicateIdentity(format!(
                "duplicate negative-control outcome ID: {outcome_id}"
            )));
        }
        if outcome_id == &input.primary_outcome_id {
            return Err(DesignAuditError::InvalidInput(
                "primary outcome cannot also be a negative-control outcome".to_owned(),
            ));
        }
    }

    Ok(())
}

/// Validates a frozen cluster-randomized intention-to-treat plan.
/// No outcome data are read or used by this audit.
pub fn audit_causal_design(
    input: &GridCausalDesignInput,
) -> Result<GridCausalDesignAuditReport, DesignAuditError> {
    validate_input(input)?;

    let mut stratum_audits = Vec::with_capacity(input.strata.len());
    let mut treated_count = 0;
    let mut control_count = 0;
    for stratum in &input.strata {
        let treated = input
            .clusters
            .iter()
            .filter(|cluster| {
                cluster.stratum_id == stratum.stratum_id
                    && cluster.assigned_arm == AssignmentArm::Treatment
            })
            .count();
        let control = input
            .clusters
            .iter()
            .filter(|cluster| {
                cluster.stratum_id == stratum.stratum_id
                    && cluster.assigned_arm == AssignmentArm::Control
            })
            .count();
        treated_count += treated;
        control_count += control;
        stratum_audits.push(StratumAudit {
            stratum_id: stratum.stratum_id.clone(),
            treated_cluster_count: treated,
            control_cluster_count: control,
            expected_treated_cluster_count: stratum.treated_cluster_count,
            expected_control_cluster_count: stratum.control_cluster_count,
        });
    }
    stratum_audits.sort_by(|left, right| left.stratum_id.cmp(&right.stratum_id));

    let assignment_digest = assignment_digest(input)?;
    let input_bytes = serde_json::to_vec(input)
        .map_err(|error| DesignAuditError::InvalidInput(format!("input serialization failed: {error}")))?;
    let input_digest = format!("sha256:{:x}", Sha256::digest(input_bytes));

    Ok(GridCausalDesignAuditReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        input_digest,
        experiment_id: input.experiment_id.clone(),
        design_class: input.design_class,
        primary_estimand: input.primary_estimand,
        design_qualified: true,
        cluster_count: input.clusters.len(),
        site_count: input.clusters.iter().map(|cluster| cluster.site_ids.len()).sum(),
        asset_count: input.clusters.iter().map(|cluster| cluster.asset_ids.len()).sum(),
        treated_cluster_count: treated_count,
        control_cluster_count: control_count,
        stratum_count: input.strata.len(),
        minimum_clusters_per_arm: input.minimum_clusters_per_arm,
        randomization_algorithm: input.randomization.algorithm.clone(),
        randomization_assignment_digest: assignment_digest,
        randomization_reproducible: true,
        declared_pre_registration_precedes_randomization: true,
        declared_public_randomness_published_after_freeze: true,
        interference_domain_ids_unique_in_manifest: true,
        pre_registration_document_authenticated: false,
        interference_mapping_authenticated: false,
        declared_timestamps_attested: false,
        pretrend_windows_predeclared: true,
        placebo_windows_predeclared: true,
        negative_control_outcomes_predeclared: true,
        cluster_level_inference_required: true,
        external_randomness_source_authenticated: false,
        causal_effect_estimated: false,
        treatment_on_treated_identified: false,
        settlement_authority_granted: false,
        payment_authorized: false,
        dispatch_authority_granted: false,
        strata: stratum_audits,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> GridCausalDesignInput {
        serde_json::from_str(include_str!("../fixtures/grid_causal_design_v0_1.json"))
            .expect("frozen causal design fixture must parse")
    }

    #[test]
    fn frozen_cluster_randomized_plan_is_reproducible_without_claiming_an_effect() {
        let report = audit_causal_design(&fixture()).unwrap();
        assert!(report.design_qualified);
        assert_eq!(report.cluster_count, 8);
        assert_eq!(report.treated_cluster_count, 4);
        assert_eq!(report.control_cluster_count, 4);
        assert_eq!(report.stratum_count, 2);
        assert!(report.randomization_reproducible);
        assert!(report.declared_pre_registration_precedes_randomization);
        assert!(report.declared_public_randomness_published_after_freeze);
        assert!(report.interference_domain_ids_unique_in_manifest);
        assert!(!report.pre_registration_document_authenticated);
        assert!(!report.interference_mapping_authenticated);
        assert!(!report.declared_timestamps_attested);
        assert!(report.cluster_level_inference_required);
        assert!(!report.external_randomness_source_authenticated);
        assert!(!report.causal_effect_estimated);
        assert!(!report.treatment_on_treated_identified);
        assert!(!report.settlement_authority_granted);
        assert!(!report.payment_authorized);
        assert!(!report.dispatch_authority_granted);
    }

    #[test]
    fn assignment_tampering_is_detected() {
        let mut input = fixture();
        input.clusters[0].assigned_arm = match input.clusters[0].assigned_arm {
            AssignmentArm::Treatment => AssignmentArm::Control,
            AssignmentArm::Control => AssignmentArm::Treatment,
        };
        assert!(matches!(
            audit_causal_design(&input),
            Err(DesignAuditError::StratumMismatch(_))
                | Err(DesignAuditError::RandomizationMismatch(_))
        ));
    }

    #[test]
    fn seed_commitment_and_randomness_digest_are_verified() {
        let mut seed = fixture();
        seed.randomization.seed_commitment_digest = format!("sha256:{}", "0".repeat(64));
        assert!(matches!(
            audit_causal_design(&seed),
            Err(DesignAuditError::RandomizationMismatch(_))
        ));

        let mut beacon = fixture();
        beacon.randomization.public_randomness_digest = format!("sha256:{}", "f".repeat(64));
        assert!(matches!(
            audit_causal_design(&beacon),
            Err(DesignAuditError::RandomizationMismatch(_))
        ));
    }

    #[test]
    fn assignment_cannot_precede_frozen_protocol_or_public_randomness() {
        let mut before_freeze = fixture();
        before_freeze.randomization.randomness_published_at_unix_ms =
            before_freeze.plan_frozen_at_unix_ms;
        assert!(matches!(
            audit_causal_design(&before_freeze),
            Err(DesignAuditError::TimeOrderViolation(_))
        ));

        let mut before_assignment = fixture();
        before_assignment.randomization.assignment_issued_at_unix_ms =
            before_assignment.randomization.randomness_published_at_unix_ms - 1;
        assert!(matches!(
            audit_causal_design(&before_assignment),
            Err(DesignAuditError::TimeOrderViolation(_))
        ));
    }

    #[test]
    fn shared_interference_domain_must_be_one_randomization_cluster() {
        let mut input = fixture();
        input.clusters[1].interference_domain_id =
            input.clusters[0].interference_domain_id.clone();
        assert!(matches!(
            audit_causal_design(&input),
            Err(DesignAuditError::InterferenceDomainOverlap(_))
        ));
    }

    #[test]
    fn site_and_asset_cannot_be_randomized_twice() {
        let mut duplicate_site = fixture();
        duplicate_site.clusters[1].site_ids[0] =
            duplicate_site.clusters[0].site_ids[0].clone();
        assert!(matches!(
            audit_causal_design(&duplicate_site),
            Err(DesignAuditError::DuplicateIdentity(_))
        ));

        let mut duplicate_asset = fixture();
        duplicate_asset.clusters[1].asset_ids[0] =
            duplicate_asset.clusters[0].asset_ids[0].clone();
        assert!(matches!(
            audit_causal_design(&duplicate_asset),
            Err(DesignAuditError::DuplicateIdentity(_))
        ));
    }

    #[test]
    fn missing_placebo_or_negative_control_is_rejected() {
        let mut no_placebo = fixture();
        no_placebo.placebo_windows.clear();
        assert!(matches!(
            audit_causal_design(&no_placebo),
            Err(DesignAuditError::InvalidInput(_))
        ));

        let mut no_negative_control = fixture();
        no_negative_control.negative_control_outcome_ids.clear();
        assert!(matches!(
            audit_causal_design(&no_negative_control),
            Err(DesignAuditError::InvalidInput(_))
        ));
    }

    #[test]
    fn each_stratum_must_match_its_preregistered_arm_counts() {
        let mut input = fixture();
        input.strata[0].treated_cluster_count += 1;
        assert!(matches!(
            audit_causal_design(&input),
            Err(DesignAuditError::StratumMismatch(_))
        ));
    }

    #[test]
    fn baseline_and_placebo_windows_cannot_leak_into_dispatch_or_recovery() {
        let mut input = fixture();
        input.placebo_windows[0] = TimeWindow {
            start_unix_ms: input.dispatch_window.start_unix_ms,
            end_unix_ms: input.dispatch_window.start_unix_ms + 100,
        };
        assert!(matches!(
            audit_causal_design(&input),
            Err(DesignAuditError::TimeOrderViolation(_))
        ));
    }

    #[test]
    fn negative_control_cannot_reuse_primary_outcome() {
        let mut input = fixture();
        input.negative_control_outcome_ids[0] = input.primary_outcome_id.clone();
        assert!(matches!(
            audit_causal_design(&input),
            Err(DesignAuditError::InvalidInput(_))
        ));
    }

    #[test]
    fn assignment_digest_is_invariant_to_cluster_input_order() {
        let first = audit_causal_design(&fixture()).unwrap();
        let mut reversed = fixture();
        reversed.clusters.reverse();
        let second = audit_causal_design(&reversed).unwrap();
        assert_eq!(
            first.randomization_assignment_digest,
            second.randomization_assignment_digest
        );
    }
}
