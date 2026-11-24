// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Read-only reconciliation of independently generated mobility response reports.
//!
//! This crate does not recalibrate meters, estimate no-dispatch baselines, track
//! dispatch response, simulate reserves, or solve network power flow. It consumes
//! the serialized reports from those owning components, verifies cross-report
//! identities and time boundaries, and emits a conservative technical summary.
//!
//! Declared source commit IDs and report-content digests are preserved, but this
//! bridge does not authenticate GitHub, producers, signatures, or source receipts.
//! A technically reconciled event is not a market settlement or payment authority.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: &str = "0.1.0";
const BENCHMARK_ID: &str = "MOBILITY-RESPONSE-SETTLEMENT-BRIDGE-001";
const GRID_RESPONSE_PR: u32 = 3649;
const PCC_METROLOGY_PR: u32 = 3772;
const COUNTERFACTUAL_BASELINE_PR: u32 = 3807;
const MOBILITY_NETWORK_PR: u32 = 7313;
const GRID_RESPONSE_PATH: &str = "crates/domains/symtropy-energy-systems/src/grid_response.rs";
const PCC_METROLOGY_PATH: &str = "crates/domains/symtropy-pcc-metrology/src/lib.rs";
const COUNTERFACTUAL_BASELINE_PATH: &str =
    "crates/domains/symtropy-counterfactual-baseline/src/lib.rs";
const MOBILITY_NETWORK_PATH: &str = "crates/domains/symtropy-mobility-network-adapter/src/lib.rs";
const CLAIM_CEILING: &str = "Read-only reconciliation of dispatch tracking, covariance-aware PCC metrology, counterfactual baseline/rebound, and locational DC network reports. The conservative lower bound is a technical evidence summary, not a joint event probability unless the upstream owner proves dependence; it is not causal identification, market settlement, payment authorization, ELCC, AC feasibility, field qualification, or dispatch authority.";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceReportArtifact {
    pub source_pr: u32,
    pub source_commit_sha: String,
    pub source_path: String,
    pub report: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MobilityResponseSettlementInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub event_id: String,
    pub offer_id: String,
    pub commitment_epoch_id: String,
    pub service_product_id: String,
    pub measurement_boundary_id: String,
    pub baseline_model_id: String,
    pub response_configuration_digest: String,
    pub site_id: String,
    pub pcc_id: String,
    pub meter_chain_configuration_digest: String,
    pub asset_id: String,
    pub asset_configuration_digest: String,
    pub pcc_bus_id: String,
    pub network_model_digest: String,
    pub topology_digest: String,
    pub instruction_issued_at_ns: u64,
    pub dispatch_end_ns: u64,
    pub observation_horizon_end_ns: u64,
    pub maximum_dispatch_energy_disagreement_kwh: f64,
    pub maximum_net_energy_disagreement_kwh: f64,
    pub grid_response: SourceReportArtifact,
    pub pcc_metrology: SourceReportArtifact,
    pub counterfactual_baseline: SourceReportArtifact,
    pub mobility_network: SourceReportArtifact,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationStatus {
    TechnicallyReconciled,
    NetworkLimited,
    NotTechnicallyQualified,
    RejectedMismatch,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SourceArtifactDigest {
    pub source_pr: u32,
    pub source_commit_sha: String,
    pub source_path: String,
    pub canonical_report_digest: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MobilityResponseSettlementReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub input_digest: String,
    pub event_id: String,
    pub offer_id: String,
    pub commitment_epoch_id: String,
    pub service_product_id: String,
    pub site_id: String,
    pub pcc_id: String,
    pub pcc_bus_id: String,
    pub asset_id: String,
    pub asset_configuration_digest: String,
    pub baseline_model_id: String,
    pub measurement_boundary_id: String,
    pub instruction_issued_at_ns: u64,
    pub dispatch_end_ns: u64,
    pub observation_horizon_end_ns: u64,
    pub status: ReconciliationStatus,
    pub technical_reconciliation_passed: bool,
    pub grid_response_passed: bool,
    pub pcc_metrology_qualified: bool,
    pub counterfactual_baseline_qualified: bool,
    pub baseline_event_qualified: bool,
    pub full_request_dc_network_deliverable: bool,
    pub required_n_minus_one_passed: bool,
    pub requested_response_energy_kwh: f64,
    pub dispatch_tracking_signed_delivered_energy_kwh: f64,
    pub pcc_point_net_reduction_energy_kwh: f64,
    pub pcc_summed_interval_lower_bound_energy_kwh: f64,
    pub counterfactual_net_reduction_energy_kwh: f64,
    pub counterfactual_conservative_net_reduction_energy_kwh: f64,
    pub dispatch_tracking_vs_metrology_dispatch_delta_kwh: f64,
    pub metrology_vs_counterfactual_net_delta_kwh: f64,
    pub maximum_dc_network_deliverable_energy_kwh: f64,
    /// Minimum dispatch-period power lower bound across all dispatch intervals,
    /// after PCC metrology, counterfactual baseline, and per-interval DC caps.
    /// This is service-period kW, not event+recovery energy divided by duration.
    pub minimum_dispatch_interval_lower_bound_kw: Option<f64>,
    pub conservative_net_response_lower_bound_kwh: Option<f64>,
    pub conservative_lower_bound_is_joint_probability: bool,
    pub pcc_sub_interval_transients_unverified: bool,
    pub response_sub_sample_transients_unverified: bool,
    pub source_artifacts: Vec<SourceArtifactDigest>,
    pub source_artifact_signatures_verified: bool,
    pub settlement_authority_granted: bool,
    pub payment_authorized: bool,
    pub market_eligibility_granted: bool,
    pub dispatch_authority_granted: bool,
    pub interconnection_approval_granted: bool,
    pub field_qualification_granted: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeError {
    InvalidInput(String),
    InvalidSourceArtifact(String),
    MissingField(String),
    InvalidField(String),
    IdentityMismatch(String),
    TimeBoundaryMismatch(String),
    IntervalMismatch(String),
    DuplicateEvidence(String),
    Arithmetic(String),
    EnergyMismatch(String),
}

impl core::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput(message)
            | Self::InvalidSourceArtifact(message)
            | Self::MissingField(message)
            | Self::InvalidField(message)
            | Self::IdentityMismatch(message)
            | Self::TimeBoundaryMismatch(message)
            | Self::IntervalMismatch(message)
            | Self::DuplicateEvidence(message)
            | Self::Arithmetic(message)
            | Self::EnergyMismatch(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for BridgeError {}

fn value<'a>(root: &'a Value, pointer: &str) -> Result<&'a Value, BridgeError> {
    root.pointer(pointer)
        .ok_or_else(|| BridgeError::MissingField(pointer.to_owned()))
}

fn string(root: &Value, pointer: &str) -> Result<String, BridgeError> {
    value(root, pointer)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| BridgeError::InvalidField(format!("{pointer} must be a string")))
}

fn number(root: &Value, pointer: &str) -> Result<f64, BridgeError> {
    let result = value(root, pointer)?
        .as_f64()
        .ok_or_else(|| BridgeError::InvalidField(format!("{pointer} must be numeric")))?;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(BridgeError::InvalidField(format!(
            "{pointer} must be finite"
        )))
    }
}

fn unsigned(root: &Value, pointer: &str) -> Result<u64, BridgeError> {
    value(root, pointer)?
        .as_u64()
        .ok_or_else(|| BridgeError::InvalidField(format!("{pointer} must be an unsigned integer")))
}

fn boolean(root: &Value, pointer: &str) -> Result<bool, BridgeError> {
    value(root, pointer)?
        .as_bool()
        .ok_or_else(|| BridgeError::InvalidField(format!("{pointer} must be boolean")))
}

fn array<'a>(root: &'a Value, pointer: &str) -> Result<&'a Vec<Value>, BridgeError> {
    value(root, pointer)?
        .as_array()
        .ok_or_else(|| BridgeError::InvalidField(format!("{pointer} must be an array")))
}

fn nonempty(name: &str, candidate: &str) -> Result<(), BridgeError> {
    if candidate.trim().is_empty() {
        Err(BridgeError::InvalidInput(format!(
            "{name} must not be empty"
        )))
    } else {
        Ok(())
    }
}

fn sha256_shape(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn commit_shape(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn digest_value(value: &Value, field: &str) -> Result<String, BridgeError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| BridgeError::Arithmetic(format!("{field} serialization: {error}")))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_artifact(
    artifact: &SourceReportArtifact,
    expected_pr: u32,
    expected_path: &str,
    label: &str,
) -> Result<SourceArtifactDigest, BridgeError> {
    if artifact.source_pr != expected_pr {
        return Err(BridgeError::InvalidSourceArtifact(format!(
            "{label} must reference source PR #{expected_pr}"
        )));
    }
    if artifact.source_path != expected_path {
        return Err(BridgeError::InvalidSourceArtifact(format!(
            "{label} source path mismatch"
        )));
    }
    if !commit_shape(&artifact.source_commit_sha) {
        return Err(BridgeError::InvalidSourceArtifact(format!(
            "{label} source commit must be a 40-character hexadecimal SHA"
        )));
    }
    if !artifact.report.is_object() {
        return Err(BridgeError::InvalidSourceArtifact(format!(
            "{label} report must be a JSON object"
        )));
    }
    Ok(SourceArtifactDigest {
        source_pr: artifact.source_pr,
        source_commit_sha: artifact.source_commit_sha.clone(),
        source_path: artifact.source_path.clone(),
        canonical_report_digest: digest_value(&artifact.report, label)?,
    })
}

fn validate_input(
    input: &MobilityResponseSettlementInput,
) -> Result<Vec<SourceArtifactDigest>, BridgeError> {
    if input.schema_version != SCHEMA_VERSION || input.benchmark_id != BENCHMARK_ID {
        return Err(BridgeError::InvalidInput(
            "schema_version or benchmark_id mismatch".to_owned(),
        ));
    }
    for (field, candidate) in [
        ("event_id", input.event_id.as_str()),
        ("offer_id", input.offer_id.as_str()),
        ("commitment_epoch_id", input.commitment_epoch_id.as_str()),
        ("service_product_id", input.service_product_id.as_str()),
        (
            "measurement_boundary_id",
            input.measurement_boundary_id.as_str(),
        ),
        ("baseline_model_id", input.baseline_model_id.as_str()),
        (
            "response_configuration_digest",
            input.response_configuration_digest.as_str(),
        ),
        ("site_id", input.site_id.as_str()),
        ("pcc_id", input.pcc_id.as_str()),
        (
            "meter_chain_configuration_digest",
            input.meter_chain_configuration_digest.as_str(),
        ),
        ("asset_id", input.asset_id.as_str()),
        (
            "asset_configuration_digest",
            input.asset_configuration_digest.as_str(),
        ),
        ("pcc_bus_id", input.pcc_bus_id.as_str()),
        ("claim_ceiling", input.claim_ceiling.as_str()),
    ] {
        nonempty(field, candidate)?;
    }
    for (field, digest) in [
        (
            "response configuration digest",
            input.response_configuration_digest.as_str(),
        ),
        (
            "meter-chain configuration digest",
            input.meter_chain_configuration_digest.as_str(),
        ),
        (
            "asset configuration digest",
            input.asset_configuration_digest.as_str(),
        ),
        ("network model digest", input.network_model_digest.as_str()),
        ("topology digest", input.topology_digest.as_str()),
    ] {
        if !sha256_shape(digest) {
            return Err(BridgeError::InvalidInput(format!(
                "{field} must be sha256:<64 hexadecimal characters>"
            )));
        }
    }
    if input.instruction_issued_at_ns == 0
        || input.dispatch_end_ns <= input.instruction_issued_at_ns
        || input.observation_horizon_end_ns < input.dispatch_end_ns
        || !input.maximum_dispatch_energy_disagreement_kwh.is_finite()
        || input.maximum_dispatch_energy_disagreement_kwh < 0.0
        || !input.maximum_net_energy_disagreement_kwh.is_finite()
        || input.maximum_net_energy_disagreement_kwh < 0.0
    {
        return Err(BridgeError::InvalidInput(
            "invalid event time horizon or energy tolerance".to_owned(),
        ));
    }

    let artifacts = vec![
        validate_artifact(
            &input.grid_response,
            GRID_RESPONSE_PR,
            GRID_RESPONSE_PATH,
            "grid_response",
        )?,
        validate_artifact(
            &input.pcc_metrology,
            PCC_METROLOGY_PR,
            PCC_METROLOGY_PATH,
            "pcc_metrology",
        )?,
        validate_artifact(
            &input.counterfactual_baseline,
            COUNTERFACTUAL_BASELINE_PR,
            COUNTERFACTUAL_BASELINE_PATH,
            "counterfactual_baseline",
        )?,
        validate_artifact(
            &input.mobility_network,
            MOBILITY_NETWORK_PR,
            MOBILITY_NETWORK_PATH,
            "mobility_network",
        )?,
    ];
    let unique_sources: BTreeSet<_> = artifacts.iter().map(|a| a.source_pr).collect();
    if unique_sources.len() != artifacts.len() {
        return Err(BridgeError::InvalidSourceArtifact(
            "source report PR identities must be unique".to_owned(),
        ));
    }
    Ok(artifacts)
}

fn check_identity(observed: String, expected: &str, label: &str) -> Result<(), BridgeError> {
    if observed == expected {
        Ok(())
    } else {
        Err(BridgeError::IdentityMismatch(format!(
            "{label}: expected {expected}, observed {observed}"
        )))
    }
}

fn same_u64(observed: u64, expected: u64, label: &str) -> Result<(), BridgeError> {
    if observed == expected {
        Ok(())
    } else {
        Err(BridgeError::TimeBoundaryMismatch(format!(
            "{label}: expected {expected}, observed {observed}"
        )))
    }
}

fn sum_interval_energy(intervals: &[Value], pointer: &str) -> Result<f64, BridgeError> {
    intervals
        .iter()
        .enumerate()
        .try_fold(0.0_f64, |sum, (index, interval)| {
            let energy = number(interval, pointer)?;
            let next = sum + energy;
            if next.is_finite() {
                Ok(next)
            } else {
                Err(BridgeError::Arithmetic(format!(
                    "interval energy accumulation overflow at index {index}"
                )))
            }
        })
}

fn validate_report_identities(input: &MobilityResponseSettlementInput) -> Result<(), BridgeError> {
    let grid = &input.grid_response.report;
    let pcc = &input.pcc_metrology.report;
    let baseline = &input.counterfactual_baseline.report;
    let network = &input.mobility_network.report;

    check_identity(
        string(grid, "/identity/event_id")?,
        &input.event_id,
        "grid event ID",
    )?;
    check_identity(
        string(grid, "/identity/service_product_id")?,
        &input.service_product_id,
        "service product ID",
    )?;
    check_identity(
        string(grid, "/identity/measurement_boundary_id")?,
        &input.measurement_boundary_id,
        "grid measurement boundary",
    )?;
    check_identity(
        string(grid, "/identity/baseline_method_id")?,
        &input.baseline_model_id,
        "baseline method/model ID",
    )?;
    check_identity(
        string(grid, "/identity/response_configuration_digest")?,
        &input.response_configuration_digest,
        "response configuration digest",
    )?;

    check_identity(
        string(pcc, "/identity/site_id")?,
        &input.site_id,
        "PCC metrology site",
    )?;
    check_identity(
        string(pcc, "/identity/pcc_id")?,
        &input.pcc_id,
        "PCC metrology PCC",
    )?;
    check_identity(
        string(pcc, "/identity/configuration_digest")?,
        &input.meter_chain_configuration_digest,
        "meter-chain configuration digest",
    )?;

    check_identity(
        string(baseline, "/identity/site_id")?,
        &input.site_id,
        "counterfactual baseline site",
    )?;
    check_identity(
        string(baseline, "/identity/pcc_id")?,
        &input.pcc_id,
        "counterfactual baseline PCC",
    )?;
    check_identity(
        string(baseline, "/identity/baseline_model_id")?,
        &input.baseline_model_id,
        "counterfactual baseline model",
    )?;

    check_identity(
        string(network, "/offer_id")?,
        &input.offer_id,
        "network adapter offer",
    )?;
    check_identity(
        string(network, "/commitment_epoch_id")?,
        &input.commitment_epoch_id,
        "network adapter commitment epoch",
    )?;
    check_identity(
        string(network, "/site_id")?,
        &input.site_id,
        "network adapter site",
    )?;
    check_identity(
        string(network, "/pcc_id")?,
        &input.pcc_id,
        "network adapter PCC",
    )?;
    check_identity(
        string(network, "/pcc_bus_id")?,
        &input.pcc_bus_id,
        "network adapter PCC bus",
    )?;
    check_identity(
        string(network, "/asset_id")?,
        &input.asset_id,
        "network adapter asset",
    )?;
    check_identity(
        string(network, "/asset_configuration_digest")?,
        &input.asset_configuration_digest,
        "network adapter asset configuration",
    )?;
    check_identity(
        string(network, "/network_model_digest")?,
        &input.network_model_digest,
        "network model digest",
    )?;
    check_identity(
        string(network, "/topology_digest")?,
        &input.topology_digest,
        "network topology digest",
    )?;
    Ok(())
}

fn validate_timelines(
    input: &MobilityResponseSettlementInput,
) -> Result<(usize, usize), BridgeError> {
    let grid = &input.grid_response.report;
    let pcc = &input.pcc_metrology.report;
    let baseline = &input.counterfactual_baseline.report;
    let network = &input.mobility_network.report;
    let mut evidence_ids = BTreeSet::new();

    same_u64(
        unsigned(grid, "/instruction_issued_at_ns")?,
        input.instruction_issued_at_ns,
        "dispatch instruction issue time",
    )?;
    same_u64(
        unsigned(grid, "/service_end_ns")?,
        input.dispatch_end_ns,
        "dispatch service end",
    )?;
    same_u64(
        unsigned(baseline, "/event_start_ns")?,
        input.instruction_issued_at_ns,
        "counterfactual event start",
    )?;
    same_u64(
        unsigned(baseline, "/dispatch_end_ns")?,
        input.dispatch_end_ns,
        "counterfactual dispatch end",
    )?;
    same_u64(
        unsigned(baseline, "/observation_horizon_end_ns")?,
        input.observation_horizon_end_ns,
        "counterfactual observation/recovery horizon",
    )?;

    let network_start_ns = unsigned(network, "/horizon_start_unix_ms")?
        .checked_mul(1_000_000)
        .ok_or_else(|| {
            BridgeError::Arithmetic("network horizon start conversion overflow".to_owned())
        })?;
    let network_end_ns = unsigned(network, "/horizon_end_unix_ms")?
        .checked_mul(1_000_000)
        .ok_or_else(|| {
            BridgeError::Arithmetic("network horizon end conversion overflow".to_owned())
        })?;
    let network_recovery_ns = unsigned(network, "/required_recovery_by_unix_ms")?
        .checked_mul(1_000_000)
        .ok_or_else(|| {
            BridgeError::Arithmetic("network recovery conversion overflow".to_owned())
        })?;
    same_u64(
        network_start_ns,
        input.instruction_issued_at_ns,
        "network offer start",
    )?;
    same_u64(network_end_ns, input.dispatch_end_ns, "network offer end")?;
    same_u64(
        network_recovery_ns,
        input.observation_horizon_end_ns,
        "network required recovery deadline",
    )?;

    let pcc_intervals = array(pcc, "/intervals")?;
    let baseline_intervals = array(baseline, "/intervals")?;
    if pcc_intervals.is_empty() || pcc_intervals.len() != baseline_intervals.len() {
        return Err(BridgeError::IntervalMismatch(
            "PCC metrology and baseline event interval counts differ or are empty".to_owned(),
        ));
    }
    same_u64(
        unsigned(pcc, "/interval_count")?,
        pcc_intervals.len() as u64,
        "PCC metrology interval count",
    )?;
    same_u64(
        unsigned(baseline, "/interval_count")?,
        baseline_intervals.len() as u64,
        "baseline event interval count",
    )?;

    let mut dispatch_count = 0_usize;
    let mut recovery_count = 0_usize;
    let mut previous_end = None;
    for (index, (pcc_interval, baseline_interval)) in
        pcc_intervals.iter().zip(baseline_intervals).enumerate()
    {
        let start = unsigned(pcc_interval, "/interval_start_ns")?;
        let duration = unsigned(pcc_interval, "/interval_duration_ns")?;
        if duration == 0 {
            return Err(BridgeError::IntervalMismatch(format!(
                "interval {index} has zero duration"
            )));
        }
        let end = start
            .checked_add(duration)
            .ok_or_else(|| BridgeError::Arithmetic("PCC interval end overflow".to_owned()))?;
        if previous_end.is_some_and(|expected| expected != start) {
            return Err(BridgeError::IntervalMismatch(format!(
                "PCC intervals are not contiguous at index {index}"
            )));
        }
        previous_end = Some(end);
        same_u64(
            unsigned(baseline_interval, "/interval_start_ns")?,
            start,
            "baseline/PCC interval start",
        )?;
        same_u64(
            unsigned(baseline_interval, "/interval_duration_ns")?,
            duration,
            "baseline/PCC interval duration",
        )?;
        let phase = string(baseline_interval, "/phase")?;
        if !boolean(pcc_interval, "/interval_qualified")?
            || !boolean(baseline_interval, "/interval_qualified")?
        {
            return Err(BridgeError::IntervalMismatch(format!(
                "PCC/baseline interval {index} is not qualified"
            )));
        }
        let baseline_id = string(pcc_interval, "/baseline_evidence_id")?;
        let candidate_id = string(pcc_interval, "/candidate_evidence_id")?;
        let baseline_event_id = string(baseline_interval, "/evidence_id")?;
        if !evidence_ids.insert(baseline_id)
            || !evidence_ids.insert(candidate_id.clone())
            || candidate_id != baseline_event_id
        {
            return Err(BridgeError::DuplicateEvidence(format!(
                "PCC/baseline evidence IDs are reused or inconsistent at interval {index}"
            )));
        }
        match phase.as_str() {
            "dispatch" => dispatch_count += 1,
            "recovery" => recovery_count += 1,
            _ => {
                return Err(BridgeError::IntervalMismatch(format!(
                    "unknown baseline interval phase {phase}"
                )));
            }
        }
    }
    same_u64(
        unsigned(baseline, "/dispatch_interval_count")?,
        dispatch_count as u64,
        "dispatch interval count",
    )?;
    same_u64(
        unsigned(baseline, "/recovery_interval_count")?,
        recovery_count as u64,
        "recovery interval count",
    )?;
    if recovery_count == 0 {
        return Err(BridgeError::IntervalMismatch(
            "counterfactual baseline has no explicit recovery intervals".to_owned(),
        ));
    }

    let dc_report = value(network, "/dc_network_report")?;
    let network_intervals = array(dc_report, "/intervals")?;
    if network_intervals.len() != dispatch_count {
        return Err(BridgeError::IntervalMismatch(
            "DC network intervals must match the dispatch interval count exactly".to_owned(),
        ));
    }
    let sample_interval_ns = unsigned(grid, "/sample_interval_ns")?;
    let sample_count = unsigned(grid, "/sample_count")? as usize;
    let samples = array(grid, "/sample_receipts")?;
    if sample_interval_ns == 0 || sample_count != samples.len() || sample_count != dispatch_count {
        return Err(BridgeError::IntervalMismatch(
            "grid-response samples do not match dispatch interval count/resolution".to_owned(),
        ));
    }
    let dispatch_intervals: Vec<_> = baseline_intervals
        .iter()
        .filter(|interval| {
            interval
                .get("phase")
                .and_then(Value::as_str)
                .is_some_and(|phase| phase == "dispatch")
        })
        .collect();
    let mut network_interval_index = 0_usize;
    for (index, (sample, baseline_interval)) in samples.iter().zip(dispatch_intervals).enumerate() {
        let elapsed = unsigned(sample, "/elapsed_ns")?;
        let expected_elapsed = (index as u64 + 1)
            .checked_mul(sample_interval_ns)
            .ok_or_else(|| {
                BridgeError::Arithmetic("grid sample elapsed-time overflow".to_owned())
            })?;
        same_u64(
            elapsed,
            expected_elapsed,
            "grid response sample elapsed time",
        )?;

        let interval_start = unsigned(baseline_interval, "/interval_start_ns")?;
        let interval_duration = unsigned(baseline_interval, "/interval_duration_ns")?;
        same_u64(
            interval_duration,
            sample_interval_ns,
            "dispatch interval sample resolution",
        )?;
        let expected_start = input
            .instruction_issued_at_ns
            .checked_add(
                (index as u64)
                    .checked_mul(sample_interval_ns)
                    .ok_or_else(|| {
                        BridgeError::Arithmetic("dispatch interval offset overflow".to_owned())
                    })?,
            )
            .ok_or_else(|| {
                BridgeError::Arithmetic("dispatch interval start overflow".to_owned())
            })?;
        same_u64(interval_start, expected_start, "dispatch interval start")?;
        let network_interval = &network_intervals[network_interval_index];
        same_u64(
            unsigned(network_interval, "/interval_start_ns")?,
            interval_start,
            "network/dispatch interval start",
        )?;
        same_u64(
            unsigned(network_interval, "/interval_duration_ns")?,
            interval_duration,
            "network/dispatch interval duration",
        )?;
        let network_max_kw = number(
            network_interval,
            "/maximum_network_deliverable_reduction_mw",
        )? * 1_000.0;
        if network_max_kw < 0.0 {
            return Err(BridgeError::InvalidField(
                "per-interval network deliverability cannot be negative".to_owned(),
            ));
        }
        network_interval_index += 1;

        let baseline_id = string(sample, "/baseline_evidence_id")?;
        let response_id = string(sample, "/response_evidence_id")?;
        if !evidence_ids.insert(baseline_id.clone()) || !evidence_ids.insert(response_id.clone()) {
            return Err(BridgeError::DuplicateEvidence(
                "baseline or response evidence reference is reused across intervals".to_owned(),
            ));
        }
        check_identity(
            string(pcc_intervals.get(index).unwrap(), "/baseline_evidence_id")?,
            &baseline_id,
            "PCC/grid baseline evidence reference",
        )?;
        check_identity(
            string(pcc_intervals.get(index).unwrap(), "/candidate_evidence_id")?,
            &response_id,
            "PCC/grid response evidence reference",
        )?;
        check_identity(
            string(baseline_interval, "/evidence_id")?,
            &response_id,
            "baseline/grid response evidence reference",
        )?;
    }

    if previous_end != Some(input.observation_horizon_end_ns) {
        return Err(BridgeError::IntervalMismatch(
            "PCC/baseline intervals do not end at the declared recovery horizon".to_owned(),
        ));
    }
    Ok((dispatch_count, recovery_count))
}

fn evaluate_component_flags(
    input: &MobilityResponseSettlementInput,
) -> Result<(bool, bool, bool, bool, bool, bool), BridgeError> {
    let grid = &input.grid_response.report;
    let pcc = &input.pcc_metrology.report;
    let baseline = &input.counterfactual_baseline.report;
    let network = &input.mobility_network.report;

    let grid_passed = boolean(grid, "/passed")?
        && boolean(grid, "/response_latency_pass")?
        && boolean(grid, "/sustained_delivery_pass")?
        && boolean(grid, "/underdelivery_energy_pass")?
        && boolean(grid, "/overshoot_energy_pass")?
        && boolean(grid, "/ramp_limit_pass")?;
    let pcc_passed = boolean(pcc, "/metrology_qualified")?;
    let baseline_qualified = boolean(baseline, "/baseline_qualification_passed")?;
    let baseline_event = boolean(baseline, "/event_qualified")?
        && boolean(baseline, "/event_and_recovery_included_in_supplied_horizon")?
        && boolean(baseline, "/workload_obligations_reconciled")?
        && boolean(baseline, "/physical_resource_states_reconciled")?;

    let network_passed = string(network, "/disposition")? == "screened_full_request"
        && boolean(network, "/full_requested_trajectory_dc_deliverable")?
        && boolean(
            value(network, "/dc_network_report")?,
            "/network_deliverable",
        )?;
    let dc = value(network, "/dc_network_report")?;
    let n1_required = boolean(dc, "/n_minus_one_required")?;
    let n1_passed = !n1_required
        || unsigned(dc, "/n_minus_one_feasible_interval_count")?
            == unsigned(dc, "/interval_count")?;

    let upstream = array(network, "/upstream_receipts")?;
    if upstream.len() != 5 {
        return Err(BridgeError::InvalidField(
            "network adapter report must preserve all five upstream mobility gate receipts"
                .to_owned(),
        ));
    }
    let required_gates: BTreeSet<String> = [
        "mobility_flexibility".to_owned(),
        "reserve_recovery".to_owned(),
        "commitment_lease".to_owned(),
        "commitment_journal".to_owned(),
        "pcc_metrology".to_owned(),
    ]
    .into_iter()
    .collect();
    let mut gate_names = BTreeSet::new();
    let mut all_upstream_gates_passed = true;
    for receipt in upstream {
        if string(receipt, "/disposition")? != "passed" {
            all_upstream_gates_passed = false;
        }
        let gate = string(receipt, "/gate")?;
        if !gate_names.insert(gate) {
            return Err(BridgeError::DuplicateEvidence(
                "network adapter contains duplicate upstream gate kinds".to_owned(),
            ));
        }
    }
    if gate_names != required_gates {
        return Err(BridgeError::InvalidField(
            "network adapter upstream gate set is not the exact five-gate contract".to_owned(),
        ));
    }
    if !all_upstream_gates_passed {
        return Ok((
            grid_passed,
            pcc_passed,
            baseline_qualified,
            baseline_event,
            false,
            false,
        ));
    }
    for field in [
        "/actual_metered_delivery_assessed",
        "/adequacy_contribution_computed",
        "/ac_feasibility_established",
        "/market_eligibility_granted",
        "/interconnection_approval_granted",
        "/field_qualification_granted",
        "/dispatch_authority_granted",
    ] {
        if boolean(network, field)? {
            return Err(BridgeError::InvalidField(format!(
                "network report improperly promotes authority or an uncomputed claim: {field}"
            )));
        }
    }
    Ok((
        grid_passed,
        pcc_passed,
        baseline_qualified,
        baseline_event,
        network_passed,
        n1_passed,
    ))
}

fn minimum_dispatch_interval_lower_bound_kw(
    input: &MobilityResponseSettlementInput,
) -> Result<f64, BridgeError> {
    let pcc = &input.pcc_metrology.report;
    let baseline = &input.counterfactual_baseline.report;
    let network = &input.mobility_network.report;
    let pcc_intervals = array(pcc, "/intervals")?;
    let baseline_intervals = array(baseline, "/intervals")?;
    let network_intervals = array(value(network, "/dc_network_report")?, "/intervals")?;
    let mut lower_bounds = Vec::new();

    for (index, (pcc_interval, baseline_interval)) in
        pcc_intervals.iter().zip(baseline_intervals).enumerate()
    {
        if string(baseline_interval, "/phase")? != "dispatch" {
            continue;
        }
        let network_interval = network_intervals.get(lower_bounds.len()).ok_or_else(|| {
            BridgeError::IntervalMismatch(
                "missing per-interval DC network result for dispatch interval".to_owned(),
            )
        })?;
        same_u64(
            unsigned(pcc_interval, "/interval_start_ns")?,
            unsigned(network_interval, "/interval_start_ns")?,
            "PCC/network dispatch interval start",
        )?;
        same_u64(
            unsigned(pcc_interval, "/interval_duration_ns")?,
            unsigned(network_interval, "/interval_duration_ns")?,
            "PCC/network dispatch interval duration",
        )?;
        let pcc_lower_kw =
            number(pcc_interval, "/conservative_reduction_lower_bound_w")? / 1_000.0;
        let baseline_lower_kw =
            number(baseline_interval, "/conservative_reduction_lower_bound_kw")?;
        let network_lower_kw =
            number(network_interval, "/maximum_network_deliverable_reduction_mw")? * 1_000.0;
        if network_lower_kw < 0.0 {
            return Err(BridgeError::InvalidField(
                "network-deliverable power cap must be non-negative".to_owned(),
            ));
        }
        let interval_lower = pcc_lower_kw.min(baseline_lower_kw).min(network_lower_kw);
        if !interval_lower.is_finite() {
            return Err(BridgeError::Arithmetic(
                "dispatch interval service-power lower bound is non-finite".to_owned(),
            ));
        }
        let _interval_index = index;
        lower_bounds.push(interval_lower);
    }
    lower_bounds.into_iter().reduce(f64::min).ok_or_else(|| {
        BridgeError::IntervalMismatch(
            "no dispatch interval is available for a service-power lower bound".to_owned(),
        )
    })
}

fn canonical_input_digest(input: &MobilityResponseSettlementInput) -> Result<String, BridgeError> {
    let bytes = serde_json::to_vec(input)
        .map_err(|error| BridgeError::Arithmetic(format!("input serialization failed: {error}")))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

pub fn evaluate_mobility_response_settlement(
    input: &MobilityResponseSettlementInput,
) -> Result<MobilityResponseSettlementReport, BridgeError> {
    let artifacts = validate_input(input)?;
    validate_report_identities(input)?;
    let _interval_counts = validate_timelines(input)?;
    let (
        grid_response_passed,
        pcc_metrology_qualified,
        baseline_qualified,
        baseline_event_qualified,
        full_request_dc_network_deliverable,
        required_n_minus_one_passed,
    ) = evaluate_component_flags(input)?;

    let grid = &input.grid_response.report;
    let pcc = &input.pcc_metrology.report;
    let baseline = &input.counterfactual_baseline.report;
    let network = &input.mobility_network.report;

    let pcc_intervals = array(pcc, "/intervals")?;
    let baseline_intervals = array(baseline, "/intervals")?;
    let measured_pcc_dispatch_kwh = pcc_intervals
        .iter()
        .zip(baseline_intervals)
        .filter(|(_, baseline_interval)| {
            baseline_interval
                .get("phase")
                .and_then(Value::as_str)
                .is_some_and(|phase| phase == "dispatch")
        })
        .try_fold(0.0_f64, |sum, (pcc_interval, _)| {
            let wh = number(pcc_interval, "/measured_reduction_energy_wh")?;
            let next = sum + wh / 1_000.0;
            if next.is_finite() {
                Ok(next)
            } else {
                Err(BridgeError::Arithmetic(
                    "dispatch PCC energy sum overflow".to_owned(),
                ))
            }
        })?;

    let report_pcc_net_lower_kwh = number(pcc, "/summed_interval_lower_bound_energy_wh")? / 1_000.0;
    let report_pcc_net_point_kwh = number(pcc, "/net_reduction_energy_wh")? / 1_000.0;
    let computed_pcc_net_lower_kwh =
        sum_interval_energy(pcc_intervals, "/lower_bound_reduction_energy_wh")? / 1_000.0;
    let computed_pcc_net_point_kwh =
        sum_interval_energy(pcc_intervals, "/measured_reduction_energy_wh")? / 1_000.0;
    let pcc_gross_reduction_kwh = number(pcc, "/gross_reduction_energy_wh")? / 1_000.0;
    let pcc_gross_rebound_kwh = number(pcc, "/gross_rebound_energy_wh")? / 1_000.0;
    if (pcc_gross_reduction_kwh - pcc_gross_rebound_kwh - report_pcc_net_point_kwh).abs()
        > input.maximum_net_energy_disagreement_kwh
    {
        return Err(BridgeError::EnergyMismatch(
            "PCC gross reduction minus rebound does not equal reported signed net energy"
                .to_owned(),
        ));
    }
    if (computed_pcc_net_lower_kwh - report_pcc_net_lower_kwh).abs()
        > input.maximum_net_energy_disagreement_kwh
        || (computed_pcc_net_point_kwh - report_pcc_net_point_kwh).abs()
            > input.maximum_net_energy_disagreement_kwh
    {
        return Err(BridgeError::Arithmetic(
            "PCC report totals do not reconcile with its interval receipts".to_owned(),
        ));
    }

    let grid_dispatch_kwh = number(grid, "/delivered_reduction_energy_kwh")?;
    let dispatch_delta = grid_dispatch_kwh - measured_pcc_dispatch_kwh;
    let pcc_net_point = report_pcc_net_point_kwh;
    let baseline_net_point = number(baseline, "/net_reduction_energy_kwh")?;
    let baseline_net_lower = number(baseline, "/conservative_net_reduction_energy_kwh")?;
    let net_delta = pcc_net_point - baseline_net_point;
    let baseline_gross_reduction_kwh = number(baseline, "/gross_reduction_energy_kwh")?;
    let baseline_gross_rebound_kwh = number(baseline, "/gross_rebound_energy_kwh")?;
    if (baseline_gross_reduction_kwh - baseline_gross_rebound_kwh - baseline_net_point).abs()
        > input.maximum_net_energy_disagreement_kwh
    {
        return Err(BridgeError::EnergyMismatch(
            "counterfactual gross reduction minus rebound does not equal signed net energy"
                .to_owned(),
        ));
    }
    let baseline_interval_lower_kwh = sum_interval_energy(
        array(baseline, "/intervals")?,
        "/conservative_reduction_energy_kwh",
    )?;
    if (baseline_interval_lower_kwh - baseline_net_lower).abs()
        > input.maximum_net_energy_disagreement_kwh
    {
        return Err(BridgeError::EnergyMismatch(
            "counterfactual conservative net energy does not equal summed interval lower bounds"
                .to_owned(),
        ));
    }
    let network_max_kwh = number(
        network,
        "/maximum_dc_network_deliverable_response_energy_mwh",
    )? * 1_000.0;
    if network_max_kwh < 0.0 {
        return Err(BridgeError::InvalidField(
            "network maximum deliverable response energy must be non-negative".to_owned(),
        ));
    }

    let requested_kwh = number(grid, "/requested_reduction_energy_kwh")?;
    let target_reduction_kw = number(grid, "/target_reduction_kw")?;
    let service_duration_ns = input.dispatch_end_ns - input.instruction_issued_at_ns;
    let expected_requested_kwh =
        target_reduction_kw * service_duration_ns as f64 / 3_600_000_000_000.0;
    if (requested_kwh - expected_requested_kwh).abs()
        > input.maximum_dispatch_energy_disagreement_kwh
    {
        return Err(BridgeError::EnergyMismatch(
            "dispatch target power × service duration does not equal requested energy".to_owned(),
        ));
    }
    let network_requested_kwh = number(network, "/site_requested_response_energy_mwh")? * 1_000.0;
    if (requested_kwh - network_requested_kwh).abs()
        > input.maximum_dispatch_energy_disagreement_kwh
    {
        return Err(BridgeError::EnergyMismatch(
            "grid instruction and mobility network offer energy differ".to_owned(),
        ));
    }

    let dispatch_point_agrees =
        dispatch_delta.abs() <= input.maximum_dispatch_energy_disagreement_kwh;
    let net_point_agrees = net_delta.abs() <= input.maximum_net_energy_disagreement_kwh;
    if !dispatch_point_agrees || !net_point_agrees {
        let report = build_report(
            input,
            artifacts,
            ReconciliationStatus::RejectedMismatch,
            false,
            grid_response_passed,
            pcc_metrology_qualified,
            baseline_qualified,
            baseline_event_qualified,
            full_request_dc_network_deliverable,
            required_n_minus_one_passed,
            requested_kwh,
            grid_dispatch_kwh,
            report_pcc_net_point_kwh,
            report_pcc_net_lower_kwh,
            baseline_net_point,
            baseline_net_lower,
            dispatch_delta,
            net_delta,
            network_max_kwh,
            None,
            None,
            boolean(baseline, "/event_lower_bound_is_joint_probability")?,
            boolean(pcc, "/sub_interval_transients_unverified")?,
            boolean(grid, "/sub_sample_transients_unverified")?,
        );
        return Ok(report);
    }

    let all_physical_evidence_passed = grid_response_passed
        && pcc_metrology_qualified
        && baseline_qualified
        && baseline_event_qualified
        && required_n_minus_one_passed;
    let uncapped_conservative_lower = report_pcc_net_lower_kwh.min(baseline_net_lower);
    let network_cap_conflict =
        uncapped_conservative_lower > network_max_kwh + input.maximum_net_energy_disagreement_kwh;
    let lower_bound = if all_physical_evidence_passed && !network_cap_conflict {
        Some(uncapped_conservative_lower)
    } else {
        None
    };

    let mut status = ReconciliationStatus::NotTechnicallyQualified;
    let mut reconciled = false;
    if network_cap_conflict {
        status = ReconciliationStatus::RejectedMismatch;
    } else if let Some(_signed_net_response) = lower_bound {
        if !full_request_dc_network_deliverable {
            status = ReconciliationStatus::NetworkLimited;
            reconciled = true;
        } else {
            status = ReconciliationStatus::TechnicallyReconciled;
            reconciled = true;
        }
    }

    Ok(build_report(
        input,
        artifacts,
        status,
        reconciled,
        grid_response_passed,
        pcc_metrology_qualified,
        baseline_qualified,
        baseline_event_qualified,
        full_request_dc_network_deliverable,
        required_n_minus_one_passed,
        requested_kwh,
        grid_dispatch_kwh,
        report_pcc_net_point_kwh,
        report_pcc_net_lower_kwh,
        baseline_net_point,
        baseline_net_lower,
        dispatch_delta,
        net_delta,
        network_max_kwh,
        if all_physical_evidence_passed {
            Some(minimum_dispatch_interval_lower_bound_kw(input)?)
        } else {
            None
        },
        lower_bound,
        boolean(baseline, "/event_lower_bound_is_joint_probability")?,
        boolean(pcc, "/sub_interval_transients_unverified")?,
        boolean(grid, "/sub_sample_transients_unverified")?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn build_report(
    input: &MobilityResponseSettlementInput,
    source_artifacts: Vec<SourceArtifactDigest>,
    status: ReconciliationStatus,
    technical_reconciliation_passed: bool,
    grid_response_passed: bool,
    pcc_metrology_qualified: bool,
    counterfactual_baseline_qualified: bool,
    baseline_event_qualified: bool,
    full_request_dc_network_deliverable: bool,
    required_n_minus_one_passed: bool,
    requested_response_energy_kwh: f64,
    dispatch_tracking_signed_delivered_energy_kwh: f64,
    pcc_point_net_reduction_energy_kwh: f64,
    pcc_summed_interval_lower_bound_energy_kwh: f64,
    counterfactual_net_reduction_energy_kwh: f64,
    counterfactual_conservative_net_reduction_energy_kwh: f64,
    dispatch_tracking_vs_metrology_dispatch_delta_kwh: f64,
    metrology_vs_counterfactual_net_delta_kwh: f64,
    maximum_dc_network_deliverable_energy_kwh: f64,
    minimum_dispatch_interval_lower_bound_kw: Option<f64>,
    conservative_net_response_lower_bound_kwh: Option<f64>,
    conservative_lower_bound_is_joint_probability: bool,
    pcc_sub_interval_transients_unverified: bool,
    response_sub_sample_transients_unverified: bool,
) -> MobilityResponseSettlementReport {
    MobilityResponseSettlementReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        input_digest: canonical_input_digest(input).unwrap_or_else(|_| "sha256:invalid".to_owned()),
        event_id: input.event_id.clone(),
        offer_id: input.offer_id.clone(),
        commitment_epoch_id: input.commitment_epoch_id.clone(),
        service_product_id: input.service_product_id.clone(),
        site_id: input.site_id.clone(),
        pcc_id: input.pcc_id.clone(),
        pcc_bus_id: input.pcc_bus_id.clone(),
        asset_id: input.asset_id.clone(),
        asset_configuration_digest: input.asset_configuration_digest.clone(),
        baseline_model_id: input.baseline_model_id.clone(),
        measurement_boundary_id: input.measurement_boundary_id.clone(),
        instruction_issued_at_ns: input.instruction_issued_at_ns,
        dispatch_end_ns: input.dispatch_end_ns,
        observation_horizon_end_ns: input.observation_horizon_end_ns,
        status,
        technical_reconciliation_passed,
        grid_response_passed,
        pcc_metrology_qualified,
        counterfactual_baseline_qualified,
        baseline_event_qualified,
        full_request_dc_network_deliverable,
        required_n_minus_one_passed,
        requested_response_energy_kwh,
        dispatch_tracking_signed_delivered_energy_kwh,
        pcc_point_net_reduction_energy_kwh,
        pcc_summed_interval_lower_bound_energy_kwh,
        counterfactual_net_reduction_energy_kwh,
        counterfactual_conservative_net_reduction_energy_kwh,
        dispatch_tracking_vs_metrology_dispatch_delta_kwh,
        metrology_vs_counterfactual_net_delta_kwh,
        maximum_dc_network_deliverable_energy_kwh,
        minimum_dispatch_interval_lower_bound_kw,
        conservative_net_response_lower_bound_kwh,
        conservative_lower_bound_is_joint_probability,
        pcc_sub_interval_transients_unverified,
        response_sub_sample_transients_unverified,
        source_artifacts,
        source_artifact_signatures_verified: false,
        settlement_authority_granted: false,
        payment_authorized: false,
        market_eligibility_granted: false,
        dispatch_authority_granted: false,
        interconnection_approval_granted: false,
        field_qualification_granted: false,
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn digest(ch: char) -> String {
        format!("sha256:{}", ch.to_string().repeat(64))
    }

    fn sha_commit(ch: char) -> String {
        ch.to_string().repeat(40)
    }

    fn report_artifact(pr: u32, source_path: &str, report: Value) -> SourceReportArtifact {
        SourceReportArtifact {
            source_pr: pr,
            source_commit_sha: sha_commit(char::from_u32(b'a' as u32 + (pr % 6) as u32).unwrap()),
            source_path: source_path.to_owned(),
            report,
        }
    }

    fn input() -> MobilityResponseSettlementInput {
        let event_start = 100_000_000_000_u64;
        let sample = 1_000_000_000_u64;
        let dispatch_end = event_start + sample * 2;
        let horizon_end = dispatch_end + sample * 2;

        let grid_samples = json!([
            {
                "elapsed_ns": sample,
                "baseline_pcc_import_kw": 100.0,
                "observed_response_pcc_import_kw": 90.0,
                "delivered_reduction_kw": 10.0,
                "delivered_fraction": 1.0,
                "interval_compliant": true,
                "underdelivery_kw": 0.0,
                "overshoot_kw": 0.0,
                "measured_ramp_kw_per_second": 0.0,
                "baseline_evidence_id": "base-1",
                "response_evidence_id": "resp-1"
            },
            {
                "elapsed_ns": sample * 2,
                "baseline_pcc_import_kw": 100.0,
                "observed_response_pcc_import_kw": 90.0,
                "delivered_reduction_kw": 10.0,
                "delivered_fraction": 1.0,
                "interval_compliant": true,
                "underdelivery_kw": 0.0,
                "overshoot_kw": 0.0,
                "measured_ramp_kw_per_second": 0.0,
                "baseline_evidence_id": "base-2",
                "response_evidence_id": "resp-2"
            }
        ]);
        let grid_report = json!({
            "identity": {
                "event_id": "event-001",
                "service_product_id": "regulating-reserve",
                "measurement_boundary_id": "site-pcc-import",
                "baseline_method_id": "baseline-model-v1",
                "response_configuration_digest": digest('f')
            },
            "instruction_issued_at_ns": event_start,
            "service_end_ns": dispatch_end,
            "sample_interval_ns": sample,
            "sample_count": 2,
            "target_reduction_kw": 10.0,
            "requested_reduction_energy_kwh": 0.005555555555555556,
            "delivered_reduction_energy_kwh": 0.005555555555555556,
            "underdelivery_energy_kwh": 0.0,
            "overshoot_energy_kwh": 0.0,
            "delivered_energy_fraction": 1.0,
            "underdelivery_energy_fraction": 0.0,
            "overshoot_energy_fraction": 0.0,
            "response_latency_ns": 0,
            "maximum_response_latency_ns": sample,
            "compliant_sample_count": 2,
            "compliant_sample_fraction": 1.0,
            "maximum_underdelivery_kw": 0.0,
            "maximum_overshoot_kw": 0.0,
            "maximum_measured_ramp_kw_per_second": 0.0,
            "response_latency_pass": true,
            "sustained_delivery_pass": true,
            "underdelivery_energy_pass": true,
            "overshoot_energy_pass": true,
            "ramp_limit_pass": true,
            "passed": true,
            "sub_sample_transients_unverified": true,
            "sample_receipts": grid_samples
        });

        let intervals = json!([
            {
                "interval_start_ns": event_start,
                "interval_duration_ns": sample,
                "baseline_corrected_power_w": 100000.0,
                "candidate_corrected_power_w": 90000.0,
                "baseline_standard_uncertainty_w": 100.0,
                "candidate_standard_uncertainty_w": 100.0,
                "corrected_error_correlation": 0.9,
                "measured_reduction_w": 10000.0,
                "reduction_standard_uncertainty_w": 44.721,
                "expanded_uncertainty_w": 89.442,
                "time_alignment_uncertainty_bound_w": 0.0,
                "conservative_reduction_lower_bound_w": 9910.558,
                "relative_expanded_uncertainty": 0.0089442,
                "response_threshold_pass": true,
                "expanded_uncertainty_pass": true,
                "interval_qualified": true,
                "measured_reduction_energy_wh": 2.777777777777778,
                "lower_bound_reduction_energy_wh": 2.752932777777778,
                "baseline_evidence_id": "base-1",
                "candidate_evidence_id": "resp-1"
            },
            {
                "interval_start_ns": event_start + sample,
                "interval_duration_ns": sample,
                "baseline_corrected_power_w": 100000.0,
                "candidate_corrected_power_w": 90000.0,
                "baseline_standard_uncertainty_w": 100.0,
                "candidate_standard_uncertainty_w": 100.0,
                "corrected_error_correlation": 0.9,
                "measured_reduction_w": 10000.0,
                "reduction_standard_uncertainty_w": 44.721,
                "expanded_uncertainty_w": 89.442,
                "time_alignment_uncertainty_bound_w": 0.0,
                "conservative_reduction_lower_bound_w": 9910.558,
                "relative_expanded_uncertainty": 0.0089442,
                "response_threshold_pass": true,
                "expanded_uncertainty_pass": true,
                "interval_qualified": true,
                "measured_reduction_energy_wh": 2.777777777777778,
                "lower_bound_reduction_energy_wh": 2.752932777777778,
                "baseline_evidence_id": "base-2",
                "candidate_evidence_id": "resp-2"
            },
            {
                "interval_start_ns": dispatch_end,
                "interval_duration_ns": sample,
                "baseline_corrected_power_w": 100000.0,
                "candidate_corrected_power_w": 100000.0,
                "baseline_standard_uncertainty_w": 100.0,
                "candidate_standard_uncertainty_w": 100.0,
                "corrected_error_correlation": 0.9,
                "measured_reduction_w": 0.0,
                "reduction_standard_uncertainty_w": 44.721,
                "expanded_uncertainty_w": 89.442,
                "time_alignment_uncertainty_bound_w": 0.0,
                "conservative_reduction_lower_bound_w": -89.442,
                "relative_expanded_uncertainty": 0.0,
                "response_threshold_pass": false,
                "expanded_uncertainty_pass": true,
                "interval_qualified": true,
                "measured_reduction_energy_wh": 0.0,
                "lower_bound_reduction_energy_wh": -0.024845,
                "baseline_evidence_id": "base-3",
                "candidate_evidence_id": "resp-3"
            },
            {
                "interval_start_ns": dispatch_end + sample,
                "interval_duration_ns": sample,
                "baseline_corrected_power_w": 100000.0,
                "candidate_corrected_power_w": 100500.0,
                "baseline_standard_uncertainty_w": 100.0,
                "candidate_standard_uncertainty_w": 100.0,
                "corrected_error_correlation": 0.9,
                "measured_reduction_w": -500.0,
                "reduction_standard_uncertainty_w": 44.721,
                "expanded_uncertainty_w": 89.442,
                "time_alignment_uncertainty_bound_w": 0.0,
                "conservative_reduction_lower_bound_w": -589.442,
                "relative_expanded_uncertainty": 0.0,
                "response_threshold_pass": false,
                "expanded_uncertainty_pass": true,
                "interval_qualified": true,
                "measured_reduction_energy_wh": -0.138888888888889,
                "lower_bound_reduction_energy_wh": -0.163734,
                "baseline_evidence_id": "base-4",
                "candidate_evidence_id": "resp-4"
            }
        ]);
        let pcc_report = json!({
            "identity": {
                "site_id": "depot-01",
                "pcc_id": "pcc-01",
                "meter_id": "meter-01",
                "configuration_digest": digest('3'),
                "calibration_id": "cal-01",
                "calibration_certificate_digest": digest('4')
            },
            "calibration_id": "cal-01",
            "calibration_certificate_digest": digest('4'),
            "clock_alignment_model_id": "clock-v1",
            "baseline_clock_uncertainty_ns": 10,
            "candidate_clock_uncertainty_ns": 10,
            "combined_clock_uncertainty_ns": 20,
            "baseline_maximum_absolute_power_slew_w_per_s": 1.0,
            "candidate_maximum_absolute_power_slew_w_per_s": 1.0,
            "coverage_factor": 2.0,
            "coverage_assumption": "synthetic fixture only",
            "minimum_response_w": 1000.0,
            "interval_count": 4,
            "qualifying_interval_count": 2,
            "qualifying_interval_fraction": 0.5,
            "gross_reduction_energy_wh": 5.555555555555556,
            "gross_rebound_energy_wh": 0.138888888888889,
            "net_reduction_energy_wh": 5.416666666666667,
            "summed_interval_lower_bound_energy_wh": 5.317286555555556,
            "metrology_qualified": true,
            "sub_interval_transients_unverified": true,
            "intervals": intervals
        });

        let baseline_intervals = json!([
            {"point_id":"point-1","phase":"dispatch","regime_id":"nominal","interval_start_ns":event_start,"interval_duration_ns":sample,"predicted_no_dispatch_power_kw":100.0,"baseline_lower_power_bound_kw":99.9,"measured_dispatch_power_kw":90.0,"measured_dispatch_upper_power_bound_kw":90.1,"baseline_residual_lower_quantile_kw":0.1,"metrology_expanded_uncertainty_kw":0.1,"reduction_point_estimate_kw":10.0,"conservative_reduction_lower_bound_kw":9.8,"conservative_reduction_energy_kwh":0.002722222222222222,"interval_qualified":true,"evidence_id":"resp-1"},
            {"point_id":"point-2","phase":"dispatch","regime_id":"nominal","interval_start_ns":event_start+sample,"interval_duration_ns":sample,"predicted_no_dispatch_power_kw":100.0,"baseline_lower_power_bound_kw":99.9,"measured_dispatch_power_kw":90.0,"measured_dispatch_upper_power_bound_kw":90.1,"baseline_residual_lower_quantile_kw":0.1,"metrology_expanded_uncertainty_kw":0.1,"reduction_point_estimate_kw":10.0,"conservative_reduction_lower_bound_kw":9.8,"conservative_reduction_energy_kwh":0.002722222222222222,"interval_qualified":true,"evidence_id":"resp-2"},
            {"point_id":"point-3","phase":"recovery","regime_id":"nominal","interval_start_ns":dispatch_end,"interval_duration_ns":sample,"predicted_no_dispatch_power_kw":100.0,"baseline_lower_power_bound_kw":99.9,"measured_dispatch_power_kw":100.0,"measured_dispatch_upper_power_bound_kw":100.1,"baseline_residual_lower_quantile_kw":0.1,"metrology_expanded_uncertainty_kw":0.1,"reduction_point_estimate_kw":0.0,"conservative_reduction_lower_bound_kw":-0.2,"conservative_reduction_energy_kwh":-0.000055555555555556,"interval_qualified":true,"evidence_id":"resp-3"},
            {"point_id":"point-4","phase":"recovery","regime_id":"nominal","interval_start_ns":dispatch_end+sample,"interval_duration_ns":sample,"predicted_no_dispatch_power_kw":100.0,"baseline_lower_power_bound_kw":99.9,"measured_dispatch_power_kw":100.5,"measured_dispatch_upper_power_bound_kw":100.6,"baseline_residual_lower_quantile_kw":0.1,"metrology_expanded_uncertainty_kw":0.1,"reduction_point_estimate_kw":-0.5,"conservative_reduction_lower_bound_kw":-0.7,"conservative_reduction_energy_kwh":-0.000194444444444444,"interval_qualified":true,"evidence_id":"resp-4"}
        ]);
        let baseline_report = json!({
            "identity": {
                "site_id": "depot-01",
                "pcc_id": "pcc-01",
                "baseline_model_id": "baseline-model-v1",
                "feature_set_digest": digest('5'),
                "training_data_digest": digest('6'),
                "calibration_data_digest": digest('7'),
                "evaluation_data_digest": digest('8')
            },
            "baseline_qualification_passed": true,
            "interval_count": 4,
            "dispatch_interval_count": 2,
            "recovery_interval_count": 2,
            "qualifying_interval_count": 4,
            "qualifying_interval_fraction_basis_points": 10000.0,
            "gross_reduction_energy_kwh": 0.005555555555555556,
            "gross_rebound_energy_kwh": 0.00025,
            "net_reduction_energy_kwh": 0.005305555555555556,
            "conservative_net_reduction_energy_kwh": 0.005194444444444444,
            "minimum_response_kw": 1.0,
            "event_qualified": true,
            "event_lower_bound_is_joint_probability": false,
            "event_and_recovery_included_in_supplied_horizon": true,
            "recovery_evidence_id": "recovery-01",
            "recovery_source_digest": digest('9'),
            "workload_reconciliation_receipt_id": "workload-01",
            "physical_state_reconciliation_receipt_id": "physical-01",
            "workload_obligations_reconciled": true,
            "physical_resource_states_reconciled": true,
            "event_start_ns": event_start,
            "dispatch_end_ns": dispatch_end,
            "observation_horizon_end_ns": horizon_end,
            "intervals": baseline_intervals,
            "claim_boundary": "synthetic baseline event only"
        });

        let network_receipts = json!([
            {"gate":"mobility_flexibility","disposition":"passed"},
            {"gate":"reserve_recovery","disposition":"passed"},
            {"gate":"commitment_lease","disposition":"passed"},
            {"gate":"commitment_journal","disposition":"passed"},
            {"gate":"pcc_metrology","disposition":"passed"}
        ]);
        let network_report = json!({
            "offer_id":"mobility-offer-001",
            "commitment_epoch_id":"commitment-epoch-007",
            "site_id":"depot-01",
            "pcc_id":"pcc-01",
            "pcc_bus_id":"B",
            "asset_id":"ess-01",
            "asset_configuration_digest":digest('f'),
            "network_model_digest":digest('1'),
            "topology_digest":digest('2'),
            "horizon_start_unix_ms":100000,
            "horizon_end_unix_ms":102000,
            "required_recovery_by_unix_ms":104000,
            "disposition":"screened_full_request",
            "site_requested_response_energy_mwh":0.000005555555555556,
            "maximum_dc_network_deliverable_response_energy_mwh":0.000005555555555556,
            "full_requested_trajectory_dc_deliverable":true,
            "dc_network_report":{"interval_count":2,"network_deliverable":true,"n_minus_one_required":true,"n_minus_one_feasible_interval_count":2},
            "upstream_receipts":network_receipts,
            "actual_metered_delivery_assessed":false,
            "adequacy_contribution_computed":false,
            "ac_feasibility_established":false,
            "market_eligibility_granted":false,
            "interconnection_approval_granted":false,
            "field_qualification_granted":false,
            "dispatch_authority_granted":false
        });

        MobilityResponseSettlementInput {
            schema_version: SCHEMA_VERSION.to_owned(),
            benchmark_id: BENCHMARK_ID.to_owned(),
            event_id: "event-001".to_owned(),
            offer_id: "mobility-offer-001".to_owned(),
            commitment_epoch_id: "commitment-epoch-007".to_owned(),
            service_product_id: "regulating-reserve".to_owned(),
            measurement_boundary_id: "site-pcc-import".to_owned(),
            baseline_model_id: "baseline-model-v1".to_owned(),
            response_configuration_digest: digest('f'),
            site_id: "depot-01".to_owned(),
            pcc_id: "pcc-01".to_owned(),
            meter_chain_configuration_digest: digest('3'),
            asset_id: "ess-01".to_owned(),
            asset_configuration_digest: digest('f'),
            pcc_bus_id: "B".to_owned(),
            network_model_digest: digest('1'),
            topology_digest: digest('2'),
            instruction_issued_at_ns: event_start,
            dispatch_end_ns: dispatch_end,
            observation_horizon_end_ns: horizon_end,
            maximum_dispatch_energy_disagreement_kwh: 0.0001,
            maximum_net_energy_disagreement_kwh: 0.0002,
            grid_response: report_artifact(GRID_RESPONSE_PR, GRID_RESPONSE_PATH, grid_report),
            pcc_metrology: report_artifact(PCC_METROLOGY_PR, PCC_METROLOGY_PATH, pcc_report),
            counterfactual_baseline: report_artifact(
                COUNTERFACTUAL_BASELINE_PR,
                COUNTERFACTUAL_BASELINE_PATH,
                baseline_report,
            ),
            mobility_network: report_artifact(
                MOBILITY_NETWORK_PR,
                MOBILITY_NETWORK_PATH,
                network_report,
            ),
            claim_ceiling: "synthetic fixture only".to_owned(),
        }
    }

    #[test]
    fn matched_four_report_bundle_reconciles_without_minting_settlement_authority() {
        let report = evaluate_mobility_response_settlement(&input()).unwrap();
        assert_eq!(report.status, ReconciliationStatus::TechnicallyReconciled);
        assert!(report.technical_reconciliation_passed);
        assert_eq!(report.source_artifacts.len(), 4);
        assert_eq!(report.requested_response_energy_kwh, 0.005555555555555556);
        assert!(report.conservative_net_response_lower_bound_kwh.unwrap() > 0.0050);
        assert!(!report.conservative_lower_bound_is_joint_probability);
        assert!(!report.source_artifact_signatures_verified);
        assert!(!report.settlement_authority_granted);
        assert!(!report.payment_authorized);
        assert!(!report.market_eligibility_granted);
        assert!(!report.dispatch_authority_granted);
    }

    #[test]
    fn rejects_source_pr_path_or_commit_identity_mismatch() {
        let mut bad = input();
        bad.grid_response.source_pr = 7313;
        assert!(matches!(
            evaluate_mobility_response_settlement(&bad),
            Err(BridgeError::InvalidSourceArtifact(_))
        ));
        let mut bad = input();
        bad.pcc_metrology.source_path = "wrong/path.rs".to_owned();
        assert!(matches!(
            evaluate_mobility_response_settlement(&bad),
            Err(BridgeError::InvalidSourceArtifact(_))
        ));
        let mut bad = input();
        bad.counterfactual_baseline.source_commit_sha = "not-a-commit".to_owned();
        assert!(matches!(
            evaluate_mobility_response_settlement(&bad),
            Err(BridgeError::InvalidSourceArtifact(_))
        ));
    }

    #[test]
    fn rejects_cross_report_identity_and_time_mismatch() {
        let mut bad = input();
        bad.grid_response.report["identity"]["event_id"] = json!("other-event");
        assert!(matches!(
            evaluate_mobility_response_settlement(&bad),
            Err(BridgeError::IdentityMismatch(_))
        ));
        let mut bad = input();
        bad.counterfactual_baseline.report["observation_horizon_end_ns"] = json!(999);
        assert!(matches!(
            evaluate_mobility_response_settlement(&bad),
            Err(BridgeError::TimeBoundaryMismatch(_))
        ));
    }

    #[test]
    fn exact_interval_alignment_is_mandatory() {
        let mut bad = input();
        bad.pcc_metrology.report["intervals"][2]["interval_start_ns"] = json!(999);
        assert!(matches!(
            evaluate_mobility_response_settlement(&bad),
            Err(BridgeError::IntervalMismatch(_)) | Err(BridgeError::TimeBoundaryMismatch(_))
        ));
    }

    #[test]
    fn dispatch_sample_ids_must_reconcile_across_reports() {
        let mut bad = input();
        bad.pcc_metrology.report["intervals"][0]["candidate_evidence_id"] = json!("other");
        bad.counterfactual_baseline.report["intervals"][0]["evidence_id"] = json!("other");
        assert!(matches!(
            evaluate_mobility_response_settlement(&bad),
            Err(BridgeError::IdentityMismatch(_))
        ));
    }

    #[test]
    fn energy_disagreement_cannot_be_silently_clamped() {
        let mut bad = input();
        bad.grid_response.report["delivered_reduction_energy_kwh"] = json!(-1.0);
        let report = evaluate_mobility_response_settlement(&bad).unwrap();
        assert_eq!(report.status, ReconciliationStatus::RejectedMismatch);
        assert!(!report.technical_reconciliation_passed);
        assert!(report.dispatch_tracking_signed_delivered_energy_kwh < 0.0);
    }

    #[test]
    fn network_limited_offer_is_not_reported_as_fully_deliverable() {
        let mut limited = input();
        limited.mobility_network.report["disposition"] = json!("screened_partial_request");
        limited.mobility_network.report["full_requested_trajectory_dc_deliverable"] = json!(false);
        limited.mobility_network.report["dc_network_report"]["network_deliverable"] = json!(false);
        limited.mobility_network.report["maximum_dc_network_deliverable_response_energy_mwh"] =
            json!(0.0000053);
        let report = evaluate_mobility_response_settlement(&limited).unwrap();
        assert_eq!(report.status, ReconciliationStatus::NetworkLimited);
        assert!(report.technical_reconciliation_passed);
        assert!(!report.full_request_dc_network_deliverable);
        assert!(report.conservative_net_response_lower_bound_kwh.unwrap() < 0.0052);
    }

    #[test]
    fn measured_lower_bound_above_network_cap_is_a_reconciliation_mismatch() {
        let mut bad = input();
        bad.mobility_network.report["maximum_dc_network_deliverable_response_energy_mwh"] =
            json!(0.000004);
        bad.mobility_network.report["disposition"] = json!("screened_partial_request");
        bad.mobility_network.report["full_requested_trajectory_dc_deliverable"] = json!(false);
        bad.mobility_network.report["dc_network_report"]["network_deliverable"] = json!(false);
        let report = evaluate_mobility_response_settlement(&bad).unwrap();
        assert_eq!(report.status, ReconciliationStatus::RejectedMismatch);
        assert!(!report.technical_reconciliation_passed);
        assert_eq!(report.conservative_net_response_lower_bound_kwh, None);
    }

    #[test]
    fn negative_conservative_net_response_is_preserved_not_clamped() {
        let mut bad = input();
        bad.counterfactual_baseline.report["conservative_net_reduction_energy_kwh"] =
            json!(-0.0001);
        bad.counterfactual_baseline.report["intervals"][3]["conservative_reduction_energy_kwh"] =
            json!(-0.005494444444444444);
        let report = evaluate_mobility_response_settlement(&bad).unwrap();
        assert_eq!(report.status, ReconciliationStatus::TechnicallyReconciled);
        assert!(report.technical_reconciliation_passed);
        assert!(report.conservative_net_response_lower_bound_kwh.unwrap() < 0.0);
    }

    #[test]
    fn upstream_component_failure_never_creates_a_positive_bound() {
        let mut bad = input();
        bad.pcc_metrology.report["metrology_qualified"] = json!(false);
        let report = evaluate_mobility_response_settlement(&bad).unwrap();
        assert_eq!(report.status, ReconciliationStatus::NotTechnicallyQualified);
        assert!(!report.technical_reconciliation_passed);
        assert_eq!(report.conservative_net_response_lower_bound_kwh, None);
    }

    #[test]
    fn counterfactual_recovery_horizon_is_non_negotiable() {
        let mut bad = input();
        bad.counterfactual_baseline.report["event_and_recovery_included_in_supplied_horizon"] =
            json!(false);
        let report = evaluate_mobility_response_settlement(&bad).unwrap();
        assert_eq!(report.status, ReconciliationStatus::NotTechnicallyQualified);
        assert_eq!(report.conservative_net_response_lower_bound_kwh, None);
    }

    #[test]
    fn duplicate_sample_evidence_ids_are_rejected() {
        let mut bad = input();
        bad.grid_response.report["sample_receipts"][1]["response_evidence_id"] = json!("resp-1");
        bad.pcc_metrology.report["intervals"][1]["candidate_evidence_id"] = json!("resp-1");
        bad.counterfactual_baseline.report["intervals"][1]["evidence_id"] = json!("resp-1");
        assert!(matches!(
            evaluate_mobility_response_settlement(&bad),
            Err(BridgeError::DuplicateEvidence(_))
        ));
    }

    #[test]
    fn missing_gate_is_detected_even_if_another_gate_failed() {
        let mut bad = input();
        bad.mobility_network.report["upstream_receipts"][0]["disposition"] = json!("failed");
        bad.mobility_network.report["upstream_receipts"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(matches!(
            evaluate_mobility_response_settlement(&bad),
            Err(BridgeError::InvalidField(_))
        ));
    }

    #[test]
    fn changed_source_report_content_changes_its_report_digest() {
        let first = evaluate_mobility_response_settlement(&input()).unwrap();
        let mut changed = input();
        changed.pcc_metrology.report["coverage_factor"] = json!(2.5);
        let second = evaluate_mobility_response_settlement(&changed).unwrap();
        let first_digest = &first.source_artifacts[1].canonical_report_digest;
        let second_digest = &second.source_artifacts[1].canonical_report_digest;
        assert_ne!(first_digest, second_digest);
    }

    #[test]
    fn temporal_interval_reordering_is_rejected_instead_of_normalized() {
        let first = evaluate_mobility_response_settlement(&input()).unwrap();
        let mut permuted = input();
        let reversed = permuted.pcc_metrology.report["intervals"]
            .as_array()
            .unwrap()
            .iter()
            .rev()
            .cloned()
            .collect();
        permuted.pcc_metrology.report["intervals"] = Value::Array(reversed);
        assert!(evaluate_mobility_response_settlement(&permuted).is_err());
        assert_eq!(first.source_artifacts.len(), 4);
    }
}
