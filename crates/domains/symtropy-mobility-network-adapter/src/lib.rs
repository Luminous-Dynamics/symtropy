// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Read-only composition of mobility assurance receipts with the existing
//! locational DC network screen.
//!
//! This crate does not implement a power-flow solver. It maps the requested
//! site response to the exact PCC bus, validates upstream evidence bindings,
//! and delegates all DC flow/contingency arithmetic to
//! symtropy-dc-network-screen.
//!
//! A successful result is a deterministic DC screening result only. It is
//! not AC feasibility, adequacy/ELCC, market eligibility, interconnection
//! approval, field qualification, or dispatch authority.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use symtropy_dc_network_screen::{
    DcBranch, DcBus, DcFlexInterval, DcFlexibilityIdentity, DcNetworkContract,
    DcNetworkDeliverabilityReport, DcNetworkModel, evaluate_dc_network_deliverability,
};

const SCHEMA_VERSION: &str = "0.1.0";
const BENCHMARK_ID: &str = "MOBILITY-NETWORK-DELIVERY-ADAPTER-001";
const MAX_INTERVALS: usize = 96;
const MAX_BUSES: usize = 256;
const MAX_BRANCHES: usize = 512;
const MAX_RECEIPTS: usize = 5;
const MAX_ABS_MW: f64 = 1_000_000.0;
const CLAIM_CEILING: &str = "Read-only composition of mobility gate receipts with deterministic active-power DC screening. A requested response is not an observed meter delivery. DC screening is not AC feasibility, protection, transient stability, PRAS/ELCC adequacy contribution, market eligibility, interconnection approval, field qualification or dispatch authority.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum GateKind {
    MobilityFlexibility,
    ReserveRecovery,
    CommitmentLease,
    CommitmentJournal,
    PccMetrology,
}

impl GateKind {
    const REQUIRED: [Self; 5] = [
        Self::MobilityFlexibility,
        Self::ReserveRecovery,
        Self::CommitmentLease,
        Self::CommitmentJournal,
        Self::PccMetrology,
    ];
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateDisposition {
    Passed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpstreamGateReceipt {
    pub gate: GateKind,
    pub receipt_id: String,
    pub evidence_reference: String,
    pub digest: String,
    pub disposition: GateDisposition,
    pub offer_id: String,
    pub commitment_epoch_id: String,
    pub site_id: String,
    pub pcc_id: String,
    pub pcc_bus_id: Option<String>,
    pub asset_id: Option<String>,
    pub asset_configuration_digest: Option<String>,
    pub observed_at_unix_ms: u64,
    pub valid_from_unix_ms: u64,
    pub valid_until_unix_ms: u64,
    pub recovery_deadline_unix_ms: Option<u64>,
    pub network_model_digest: Option<String>,
    pub topology_digest: Option<String>,
    pub baseline_trace_digest: Option<String>,
    pub flexible_trace_digest: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NetworkBusInput {
    pub id: String,
    pub slack: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NetworkBranchInput {
    pub id: String,
    pub from_bus: String,
    pub to_bus: String,
    pub reactance_pu: f64,
    pub normal_limit_mw: f64,
    pub emergency_limit_mw: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NetworkModelInput {
    pub network_model_id: String,
    pub network_model_digest: String,
    pub topology_id: String,
    pub topology_digest: String,
    pub topology_effective_at_ns: u64,
    pub base_mva: f64,
    pub buses: Vec<NetworkBusInput>,
    pub branches: Vec<NetworkBranchInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MobilityResponseIntervalInput {
    pub start_unix_ms: u64,
    pub duration_ms: u64,
    pub baseline_non_slack_injections_mw: BTreeMap<String, f64>,
    pub requested_pcc_reduction_mw: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MobilityNetworkAdapterInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub offer_id: String,
    pub commitment_epoch_id: String,
    pub site_id: String,
    pub pcc_id: String,
    pub pcc_bus_id: String,
    pub asset_id: String,
    pub asset_configuration_digest: String,
    pub baseline_trace_digest: String,
    pub flexible_trace_digest: String,
    pub decision_cutoff_unix_ms: u64,
    pub horizon_start_unix_ms: u64,
    pub horizon_end_unix_ms: u64,
    pub required_recovery_by_unix_ms: u64,
    pub maximum_topology_age_ns: u64,
    pub flow_tolerance_mw: f64,
    pub pivot_tolerance: f64,
    pub require_n_minus_one: bool,
    pub network: NetworkModelInput,
    pub intervals: Vec<MobilityResponseIntervalInput>,
    pub n_minus_one_contingencies: Vec<String>,
    pub upstream_receipts: Vec<UpstreamGateReceipt>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompositeDisposition {
    HeldUnknownGate,
    RejectedFailedGate,
    ScreenedFullRequest,
    ScreenedPartialRequest,
    ScreenedUndeliverable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GateReceiptSummary {
    pub gate: GateKind,
    pub receipt_id: String,
    pub evidence_reference: String,
    pub digest: String,
    pub disposition: GateDisposition,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MobilityNetworkAdapterReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub input_digest: String,
    pub offer_id: String,
    pub commitment_epoch_id: String,
    pub site_id: String,
    pub pcc_id: String,
    pub pcc_bus_id: String,
    pub asset_id: String,
    pub asset_configuration_digest: String,
    pub network_model_digest: String,
    pub topology_digest: String,
    pub horizon_start_unix_ms: u64,
    pub horizon_end_unix_ms: u64,
    pub disposition: CompositeDisposition,
    /// Requested PCC trajectory volume, not actual measured delivery.
    pub site_requested_response_energy_mwh: f64,
    /// None when a mandatory upstream gate failed or remains unknown.
    pub maximum_dc_network_deliverable_response_energy_mwh: Option<f64>,
    /// Requested energy minus the maximum response admitted by the DC screen.
    pub dc_undeliverable_or_curtailed_energy_mwh: Option<f64>,
    pub energy_conservation_error_mwh: Option<f64>,
    pub full_requested_trajectory_dc_deliverable: Option<bool>,
    /// The adapter does not infer post-event meter delivery from a baseline receipt.
    pub actual_measured_pcc_response_energy_mwh: Option<f64>,
    pub dc_network_report: Option<DcNetworkDeliverabilityReport>,
    pub upstream_receipts: Vec<GateReceiptSummary>,
    pub actual_metered_delivery_assessed: bool,
    pub adequacy_contribution_computed: bool,
    pub ac_feasibility_established: bool,
    pub market_eligibility_granted: bool,
    pub interconnection_approval_granted: bool,
    pub field_qualification_granted: bool,
    pub dispatch_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    InvalidInput(String),
    MissingGateReceipt(GateKind),
    DuplicateGateReceipt(GateKind),
    DuplicateReceiptIdentity(String),
    MissingEvidence(String),
    InvalidDigest(String),
    IdentityMismatch(String),
    EvidenceTimeInvalid(String),
    InvalidHorizon(String),
    InvalidInterval(String),
    InvalidNetwork(String),
    NetworkScreen(String),
    Arithmetic(String),
}

impl core::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput(message)
            | Self::DuplicateReceiptIdentity(message)
            | Self::MissingEvidence(message)
            | Self::InvalidDigest(message)
            | Self::IdentityMismatch(message)
            | Self::EvidenceTimeInvalid(message)
            | Self::InvalidHorizon(message)
            | Self::InvalidInterval(message)
            | Self::InvalidNetwork(message)
            | Self::NetworkScreen(message)
            | Self::Arithmetic(message) => f.write_str(message),
            Self::MissingGateReceipt(gate) => write!(f, "missing required gate receipt: {gate:?}"),
            Self::DuplicateGateReceipt(gate) => {
                write!(f, "duplicate required gate receipt: {gate:?}")
            }
        }
    }
}

impl std::error::Error for AdapterError {}

fn require_nonempty(field: &str, value: &str) -> Result<(), AdapterError> {
    if value.trim().is_empty() {
        Err(AdapterError::MissingEvidence(format!(
            "{field} must not be empty"
        )))
    } else {
        Ok(())
    }
}

fn valid_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn check_sha256(field: &str, value: &str) -> Result<(), AdapterError> {
    if valid_sha256(value) {
        Ok(())
    } else {
        Err(AdapterError::InvalidDigest(format!(
            "{field} must be sha256:<64 hexadecimal characters>"
        )))
    }
}

fn checked_ms_to_ns(value: u64, field: &str) -> Result<u64, AdapterError> {
    value
        .checked_mul(1_000_000)
        .ok_or_else(|| AdapterError::Arithmetic(format!("{field} milliseconds-to-nanoseconds overflow")))
}

#[derive(Serialize)]
struct NetworkModelDigestPayload {
    network_model_id: String,
    base_mva: f64,
    buses: Vec<NetworkBusInput>,
    branches: Vec<NetworkBranchInput>,
}

#[derive(Serialize)]
struct TopologyDigestPayload {
    topology_id: String,
    topology_effective_at_ns: u64,
    bus_ids: Vec<String>,
    branch_ids: Vec<String>,
}

fn digest_serializable(value: &impl Serialize, field: &str) -> Result<String, AdapterError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        AdapterError::Arithmetic(format!("{field} canonical serialization failed: {error}"))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

/// Hashes the canonical network model payload independently of the declared digest.
/// Bus and branch order do not affect this value.
pub fn expected_network_model_digest(network: &NetworkModelInput) -> Result<String, AdapterError> {
    let mut buses = network.buses.clone();
    buses.sort_by(|a, b| a.id.cmp(&b.id));
    let mut branches = network.branches.clone();
    branches.sort_by(|a, b| a.id.cmp(&b.id));
    let payload = NetworkModelDigestPayload {
        network_model_id: network.network_model_id.clone(),
        base_mva: network.base_mva,
        buses,
        branches,
    };
    digest_serializable(&payload, "network model")
}

/// Hashes topology identity and membership separately from line parameters.
pub fn expected_topology_digest(network: &NetworkModelInput) -> Result<String, AdapterError> {
    let mut bus_ids: Vec<_> = network.buses.iter().map(|bus| bus.id.clone()).collect();
    let mut branch_ids: Vec<_> = network.branches.iter().map(|branch| branch.id.clone()).collect();
    bus_ids.sort();
    branch_ids.sort();
    let payload = TopologyDigestPayload {
        topology_id: network.topology_id.clone(),
        topology_effective_at_ns: network.topology_effective_at_ns,
        bus_ids,
        branch_ids,
    };
    digest_serializable(&payload, "network topology")
}

fn canonical_input_digest(input: &MobilityNetworkAdapterInput) -> Result<String, AdapterError> {
    let mut canonical = input.clone();
    canonical.upstream_receipts.sort_by_key(|receipt| receipt.gate);
    canonical.n_minus_one_contingencies.sort();
    canonical.network.buses.sort_by(|a, b| a.id.cmp(&b.id));
    canonical.network.branches.sort_by(|a, b| a.id.cmp(&b.id));
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| AdapterError::Arithmetic(format!("canonical input serialization failed: {error}")))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_receipts(
    input: &MobilityNetworkAdapterInput,
) -> Result<Vec<GateReceiptSummary>, AdapterError> {
    if input.upstream_receipts.len() != MAX_RECEIPTS {
        return Err(AdapterError::InvalidInput(format!(
            "expected exactly {MAX_RECEIPTS} mandatory upstream gate receipts"
        )));
    }

    let mut receipt_ids = BTreeSet::new();
    let mut evidence_refs = BTreeSet::new();
    let mut receipt_digests = BTreeSet::new();
    let mut gates = BTreeMap::new();
    let mut summaries = Vec::with_capacity(input.upstream_receipts.len());

    for receipt in &input.upstream_receipts {
        if gates.insert(receipt.gate, receipt).is_some() {
            return Err(AdapterError::DuplicateGateReceipt(receipt.gate));
        }
        require_nonempty("receipt ID", &receipt.receipt_id)?;
        require_nonempty("evidence reference", &receipt.evidence_reference)?;
        if !receipt_ids.insert(receipt.receipt_id.as_str()) {
            return Err(AdapterError::DuplicateReceiptIdentity(format!(
                "duplicate receipt ID: {}",
                receipt.receipt_id
            )));
        }
        if !evidence_refs.insert(receipt.evidence_reference.as_str()) {
            return Err(AdapterError::DuplicateReceiptIdentity(format!(
                "duplicate evidence reference: {}",
                receipt.evidence_reference
            )));
        }
        check_sha256("upstream receipt digest", &receipt.digest)?;
        if !receipt_digests.insert(receipt.digest.as_str()) {
            return Err(AdapterError::DuplicateReceiptIdentity(format!(
                "duplicate upstream receipt digest: {}",
                receipt.digest
            )));
        }

        if receipt.offer_id != input.offer_id
            || receipt.commitment_epoch_id != input.commitment_epoch_id
            || receipt.site_id != input.site_id
            || receipt.pcc_id != input.pcc_id
            || receipt
                .pcc_bus_id
                .as_deref()
                .is_some_and(|bus_id| bus_id != input.pcc_bus_id)
        {
            return Err(AdapterError::IdentityMismatch(format!(
                "{:?} receipt is bound to a different offer, epoch, site or PCC",
                receipt.gate
            )));
        }

        match receipt.gate {
            GateKind::MobilityFlexibility
            | GateKind::ReserveRecovery
            | GateKind::CommitmentLease
            | GateKind::CommitmentJournal => {
                if receipt.asset_id.as_deref() != Some(input.asset_id.as_str())
                    || receipt.asset_configuration_digest.as_deref()
                        != Some(input.asset_configuration_digest.as_str())
                {
                    return Err(AdapterError::IdentityMismatch(format!(
                        "{:?} receipt does not bind the exact asset/configuration",
                        receipt.gate
                    )));
                }
                if receipt.gate == GateKind::MobilityFlexibility
                    && receipt.pcc_bus_id.as_deref() != Some(input.pcc_bus_id.as_str())
                {
                    return Err(AdapterError::IdentityMismatch(
                        "mobility flexibility receipt must bind the exact PCC bus".to_owned(),
                    ));
                }
                if receipt.gate == GateKind::ReserveRecovery {
                    if receipt.recovery_deadline_unix_ms
                        != Some(input.required_recovery_by_unix_ms)
                        || receipt.valid_until_unix_ms < input.required_recovery_by_unix_ms
                    {
                        return Err(AdapterError::EvidenceTimeInvalid(
                            "reserve-recovery receipt does not cover the next required recovery deadline"
                                .to_owned(),
                        ));
                    }
                } else if receipt.recovery_deadline_unix_ms.is_some() {
                    return Err(AdapterError::IdentityMismatch(
                        "only the reserve-recovery receipt may declare a recovery deadline"
                            .to_owned(),
                    ));
                }
            }
            GateKind::PccMetrology => {
                if receipt.pcc_bus_id.as_deref() != Some(input.pcc_bus_id.as_str()) {
                    return Err(AdapterError::IdentityMismatch(
                        "PCC metrology receipt must bind the exact PCC bus".to_owned(),
                    ));
                }
                if receipt.recovery_deadline_unix_ms.is_some() {
                    return Err(AdapterError::IdentityMismatch(
                        "PCC metrology receipt must not declare a reserve-recovery deadline"
                            .to_owned(),
                    ));
                }
                if receipt.asset_id.is_some()
                    && receipt.asset_id.as_deref() != Some(input.asset_id.as_str())
                {
                    return Err(AdapterError::IdentityMismatch(
                        "PCC metrology receipt is bound to another asset".to_owned(),
                    ));
                }
                if receipt.asset_configuration_digest.is_some()
                    && receipt.asset_configuration_digest.as_deref()
                        != Some(input.asset_configuration_digest.as_str())
                {
                    return Err(AdapterError::IdentityMismatch(
                        "PCC metrology receipt is bound to another asset configuration".to_owned(),
                    ));
                }
                if receipt.baseline_trace_digest.as_deref()
                    != Some(input.baseline_trace_digest.as_str())
                    || receipt.flexible_trace_digest.as_deref()
                        != Some(input.flexible_trace_digest.as_str())
                {
                    return Err(AdapterError::IdentityMismatch(
                        "PCC metrology receipt does not bind the exact baseline/offer trace pair"
                            .to_owned(),
                    ));
                }
            }
        }

        if receipt.observed_at_unix_ms > input.decision_cutoff_unix_ms
            || receipt.valid_from_unix_ms > receipt.observed_at_unix_ms
            || receipt.observed_at_unix_ms > receipt.valid_until_unix_ms
            || receipt.valid_from_unix_ms > input.horizon_start_unix_ms
            || receipt.valid_until_unix_ms < input.horizon_end_unix_ms
        {
            return Err(AdapterError::EvidenceTimeInvalid(format!(
                "{:?} receipt is future-dated, stale or does not cover the complete offer horizon",
                receipt.gate
            )));
        }

        if let Some(digest) = receipt.network_model_digest.as_deref() {
            check_sha256("receipt network-model digest", digest)?;
            if digest != input.network.network_model_digest {
                return Err(AdapterError::IdentityMismatch(format!(
                    "{:?} receipt references a different network model",
                    receipt.gate
                )));
            }
        }
        if let Some(digest) = receipt.topology_digest.as_deref() {
            check_sha256("receipt topology digest", digest)?;
            if digest != input.network.topology_digest {
                return Err(AdapterError::IdentityMismatch(format!(
                    "{:?} receipt references a different topology",
                    receipt.gate
                )));
            }
        }
        if receipt.gate == GateKind::MobilityFlexibility
            && (receipt.network_model_digest.as_deref()
                != Some(input.network.network_model_digest.as_str())
                || receipt.topology_digest.as_deref()
                    != Some(input.network.topology_digest.as_str()))
        {
            return Err(AdapterError::IdentityMismatch(
                "mobility flexibility receipt must pin the exact network model and topology"
                    .to_owned(),
            ));
        }

        summaries.push(GateReceiptSummary {
            gate: receipt.gate,
            receipt_id: receipt.receipt_id.clone(),
            evidence_reference: receipt.evidence_reference.clone(),
            digest: receipt.digest.clone(),
            disposition: receipt.disposition,
        });
    }

    for gate in GateKind::REQUIRED {
        if !gates.contains_key(&gate) {
            return Err(AdapterError::MissingGateReceipt(gate));
        }
    }
    summaries.sort_by_key(|summary| summary.gate);
    Ok(summaries)
}

fn validate_input(input: &MobilityNetworkAdapterInput) -> Result<(), AdapterError> {
    if input.schema_version != SCHEMA_VERSION || input.benchmark_id != BENCHMARK_ID {
        return Err(AdapterError::InvalidInput(
            "schema or benchmark identity mismatch".to_owned(),
        ));
    }
    for (field, value) in [
        ("offer_id", input.offer_id.as_str()),
        ("commitment_epoch_id", input.commitment_epoch_id.as_str()),
        ("site_id", input.site_id.as_str()),
        ("pcc_id", input.pcc_id.as_str()),
        ("pcc_bus_id", input.pcc_bus_id.as_str()),
        ("asset_id", input.asset_id.as_str()),
        (
            "asset_configuration_digest",
            input.asset_configuration_digest.as_str(),
        ),
    ] {
        require_nonempty(field, value)?;
    }
    for (field, digest) in [
        ("asset configuration digest", input.asset_configuration_digest.as_str()),
        ("baseline trace digest", input.baseline_trace_digest.as_str()),
        ("flexible trace digest", input.flexible_trace_digest.as_str()),
        ("network model digest", input.network.network_model_digest.as_str()),
        ("topology digest", input.network.topology_digest.as_str()),
    ] {
        check_sha256(field, digest)?;
    }
    if input.baseline_trace_digest == input.flexible_trace_digest {
        return Err(AdapterError::InvalidDigest(
            "baseline and flexible trace digests must differ".to_owned(),
        ));
    }
    let expected_model_digest = expected_network_model_digest(&input.network)?;
    if input.network.network_model_digest != expected_model_digest {
        return Err(AdapterError::InvalidDigest(
            "network-model digest does not match canonical network content".to_owned(),
        ));
    }
    let expected_topology_digest = expected_topology_digest(&input.network)?;
    if input.network.topology_digest != expected_topology_digest {
        return Err(AdapterError::InvalidDigest(
            "topology digest does not match canonical topology membership".to_owned(),
        ));
    }
    if input.decision_cutoff_unix_ms == 0
        || input.horizon_start_unix_ms <= input.decision_cutoff_unix_ms
        || input.horizon_end_unix_ms <= input.horizon_start_unix_ms
        || input.required_recovery_by_unix_ms < input.horizon_end_unix_ms
    {
        return Err(AdapterError::InvalidHorizon(
            "decision cutoff must precede a non-empty offer horizon".to_owned(),
        ));
    }
    if input.intervals.is_empty() || input.intervals.len() > MAX_INTERVALS {
        return Err(AdapterError::InvalidHorizon(format!(
            "interval count must be between 1 and {MAX_INTERVALS}"
        )));
    }
    if input.network.buses.is_empty()
        || input.network.buses.len() > MAX_BUSES
        || input.network.branches.is_empty()
        || input.network.branches.len() > MAX_BRANCHES
    {
        return Err(AdapterError::InvalidNetwork(
            "network bus/branch count is empty or exceeds configured bounds".to_owned(),
        ));
    }
    if input.maximum_topology_age_ns == 0
        || !input.flow_tolerance_mw.is_finite()
        || input.flow_tolerance_mw <= 0.0
        || !input.pivot_tolerance.is_finite()
        || input.pivot_tolerance <= 0.0
    {
        return Err(AdapterError::InvalidNetwork(
            "topology age and numerical tolerances must be finite and positive".to_owned(),
        ));
    }

    let mut next_start = input.horizon_start_unix_ms;
    for (index, interval) in input.intervals.iter().enumerate() {
        if interval.start_unix_ms != next_start || interval.duration_ms == 0 {
            return Err(AdapterError::InvalidInterval(format!(
                "interval {index} is non-contiguous or has zero duration"
            )));
        }
        if !interval.requested_pcc_reduction_mw.is_finite()
            || interval.requested_pcc_reduction_mw < 0.0
            || interval.requested_pcc_reduction_mw > MAX_ABS_MW
            || interval
                .baseline_non_slack_injections_mw
                .values()
                .any(|value| !value.is_finite() || value.abs() > MAX_ABS_MW)
        {
            return Err(AdapterError::InvalidInterval(format!(
                "interval {index} has a non-finite or out-of-bounds MW quantity"
            )));
        }
        next_start = next_start
            .checked_add(interval.duration_ms)
            .ok_or_else(|| AdapterError::Arithmetic("interval end timestamp overflow".to_owned()))?;
    }
    if next_start != input.horizon_end_unix_ms {
        return Err(AdapterError::InvalidHorizon(
            "intervals do not cover the exact declared offer horizon".to_owned(),
        ));
    }
    if input.network.buses.iter().any(|bus| bus.id.trim().is_empty())
        || input.network.branches.iter().any(|branch| {
            branch.id.trim().is_empty()
                || branch.from_bus.trim().is_empty()
                || branch.to_bus.trim().is_empty()
        })
    {
        return Err(AdapterError::InvalidNetwork(
            "network bus and branch IDs must not be empty".to_owned(),
        ));
    }
    Ok(())
}

fn build_network(input: &MobilityNetworkAdapterInput) -> Result<DcNetworkModel, AdapterError> {
    let mut buses = input
        .network
        .buses
        .iter()
        .map(|bus| {
            DcBus::new(bus.id.clone(), bus.slack)
                .map_err(|error| AdapterError::InvalidNetwork(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    buses.sort_by(|a, b| a.id().cmp(b.id()));

    let mut branch_inputs = input.network.branches.clone();
    branch_inputs.sort_by(|a, b| a.id.cmp(&b.id));
    let branches = branch_inputs
        .iter()
        .map(|branch| {
            DcBranch::new(
                branch.id.clone(),
                branch.from_bus.clone(),
                branch.to_bus.clone(),
                branch.reactance_pu,
                branch.normal_limit_mw,
                branch.emergency_limit_mw,
            )
            .map_err(|error| AdapterError::InvalidNetwork(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    DcNetworkModel::new(
        input.network.network_model_id.clone(),
        input.network.network_model_digest.clone(),
        input.network.topology_id.clone(),
        input.network.topology_digest.clone(),
        input.network.topology_effective_at_ns,
        input.network.base_mva,
        buses,
        branches,
    )
    .map_err(|error| AdapterError::InvalidNetwork(error.to_string()))
}

fn requested_energy_mwh(
    intervals: &[MobilityResponseIntervalInput],
) -> Result<f64, AdapterError> {
    let energy = intervals.iter().try_fold(0.0_f64, |sum, interval| {
        let duration_hours = interval.duration_ms as f64 / 3_600_000.0;
        let increment = interval.requested_pcc_reduction_mw * duration_hours;
        let next = sum + increment;
        if next.is_finite() {
            Ok(next)
        } else {
            Err(AdapterError::Arithmetic(
                "requested response energy overflow".to_owned(),
            ))
        }
    })?;
    Ok(energy)
}

/// Evaluate mandatory gate receipts first, then delegate DC network
/// feasibility to symtropy-dc-network-screen.
pub fn evaluate_mobility_network_adapter(
    input: &MobilityNetworkAdapterInput,
) -> Result<MobilityNetworkAdapterReport, AdapterError> {
    validate_input(input)?;
    let input_digest = canonical_input_digest(input)?;
    let receipt_summaries = validate_receipts(input)?;
    let site_requested_energy = requested_energy_mwh(&input.intervals)?;

    let failed_gate = input
        .upstream_receipts
        .iter()
        .any(|receipt| receipt.disposition == GateDisposition::Failed);
    let unknown_gate = input
        .upstream_receipts
        .iter()
        .any(|receipt| receipt.disposition == GateDisposition::Unknown);

    let mut report = MobilityNetworkAdapterReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        input_digest,
        offer_id: input.offer_id.clone(),
        commitment_epoch_id: input.commitment_epoch_id.clone(),
        site_id: input.site_id.clone(),
        pcc_id: input.pcc_id.clone(),
        pcc_bus_id: input.pcc_bus_id.clone(),
        asset_id: input.asset_id.clone(),
        asset_configuration_digest: input.asset_configuration_digest.clone(),
        network_model_digest: input.network.network_model_digest.clone(),
        topology_digest: input.network.topology_digest.clone(),
        horizon_start_unix_ms: input.horizon_start_unix_ms,
        horizon_end_unix_ms: input.horizon_end_unix_ms,
        disposition: CompositeDisposition::HeldUnknownGate,
        site_requested_response_energy_mwh: site_requested_energy,
        maximum_dc_network_deliverable_response_energy_mwh: None,
        dc_undeliverable_or_curtailed_energy_mwh: None,
        energy_conservation_error_mwh: None,
        full_requested_trajectory_dc_deliverable: None,
        actual_measured_pcc_response_energy_mwh: None,
        dc_network_report: None,
        upstream_receipts: receipt_summaries,
        actual_metered_delivery_assessed: false,
        adequacy_contribution_computed: false,
        ac_feasibility_established: false,
        market_eligibility_granted: false,
        interconnection_approval_granted: false,
        field_qualification_granted: false,
        dispatch_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    };

    if failed_gate {
        report.disposition = CompositeDisposition::RejectedFailedGate;
        return Ok(report);
    }
    if unknown_gate {
        report.disposition = CompositeDisposition::HeldUnknownGate;
        return Ok(report);
    }

    let network = build_network(input)?;
    let identity = DcFlexibilityIdentity::new(
        input.site_id.clone(),
        input.pcc_id.clone(),
        input.pcc_bus_id.clone(),
        input.baseline_trace_digest.clone(),
        input.flexible_trace_digest.clone(),
    )
    .map_err(|error| AdapterError::InvalidNetwork(error.to_string()))?;

    let contract = DcNetworkContract::new(
        input.maximum_topology_age_ns,
        input.flow_tolerance_mw,
        input.pivot_tolerance,
        input.require_n_minus_one,
    )
    .map_err(|error| AdapterError::InvalidNetwork(error.to_string()))?;

    let mut dc_intervals = Vec::with_capacity(input.intervals.len());
    for interval in &input.intervals {
        let start_ns = checked_ms_to_ns(interval.start_unix_ms, "interval start")?;
        let duration_ns = checked_ms_to_ns(interval.duration_ms, "interval duration")?;
        let action = BTreeMap::from([(
            input.pcc_bus_id.clone(),
            interval.requested_pcc_reduction_mw,
        )]);
        dc_intervals.push(
            DcFlexInterval::new(
                start_ns,
                duration_ns,
                interval.baseline_non_slack_injections_mw.clone(),
                action,
            )
            .map_err(|error| AdapterError::InvalidInterval(error.to_string()))?,
        );
    }
    let mut contingencies = input.n_minus_one_contingencies.clone();
    contingencies.sort();
    contingencies.dedup();
    if contingencies.len() != input.n_minus_one_contingencies.len() {
        return Err(AdapterError::InvalidNetwork(
            "duplicate N-1 contingency branch ID".to_owned(),
        ));
    }

    let dc_report = evaluate_dc_network_deliverability(
        &identity,
        &network,
        &contract,
        &dc_intervals,
        &contingencies,
    )
    .map_err(|error| AdapterError::NetworkScreen(error.to_string()))?;

    let maximum_deliverable = dc_report.maximum_network_deliverable_reduction_energy_mwh;
    if !maximum_deliverable.is_finite()
        || maximum_deliverable < -input.flow_tolerance_mw
        || maximum_deliverable > site_requested_energy + input.flow_tolerance_mw
    {
        return Err(AdapterError::Arithmetic(
            "DC network-deliverable energy is outside the requested energy bounds".to_owned(),
        ));
    }
    let curtailed = (site_requested_energy - maximum_deliverable).max(0.0);
    let conservation_error = site_requested_energy - maximum_deliverable - curtailed;
    if conservation_error.abs() > input.flow_tolerance_mw.max(1.0e-9) {
        return Err(AdapterError::Arithmetic(
            "requested = network-deliverable + undeliverable energy conservation failed"
                .to_owned(),
        ));
    }

    report.disposition = if dc_report.network_deliverable {
        CompositeDisposition::ScreenedFullRequest
    } else if maximum_deliverable > input.flow_tolerance_mw {
        CompositeDisposition::ScreenedPartialRequest
    } else {
        CompositeDisposition::ScreenedUndeliverable
    };
    report.maximum_dc_network_deliverable_response_energy_mwh = Some(maximum_deliverable);
    report.dc_undeliverable_or_curtailed_energy_mwh = Some(curtailed);
    report.energy_conservation_error_mwh = Some(conservation_error);
    report.full_requested_trajectory_dc_deliverable = Some(dc_report.network_deliverable);
    report.dc_network_report = Some(dc_report);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn receipt(
        gate: GateKind,
        index: usize,
        disposition: GateDisposition,
        network_model_digest: &str,
        topology_digest: &str,
    ) -> UpstreamGateReceipt {
        let asset_scoped = gate != GateKind::PccMetrology;
        let pcc_bus_scoped = matches!(gate, GateKind::MobilityFlexibility | GateKind::PccMetrology);
        UpstreamGateReceipt {
            gate,
            receipt_id: format!("receipt-{index}"),
            evidence_reference: format!("evidence-ref-{index}"),
            digest: sha(char::from_u32('a' as u32 + index as u32).unwrap()),
            disposition,
            offer_id: "mobility-offer-001".to_owned(),
            commitment_epoch_id: "commitment-epoch-007".to_owned(),
            site_id: "depot-01".to_owned(),
            pcc_id: "pcc-01".to_owned(),
            pcc_bus_id: pcc_bus_scoped.then(|| "B".to_owned()),
            asset_id: asset_scoped.then(|| "ess-01".to_owned()),
            asset_configuration_digest: asset_scoped.then(|| sha('f')),
            observed_at_unix_ms: 90_000,
            valid_from_unix_ms: 0,
            valid_until_unix_ms: 4_000_000,
            recovery_deadline_unix_ms: (gate == GateKind::ReserveRecovery).then_some(3_900_000),
            network_model_digest: (gate == GateKind::MobilityFlexibility)
                .then(|| network_model_digest.to_owned()),
            topology_digest: (gate == GateKind::MobilityFlexibility)
                .then(|| topology_digest.to_owned()),
            baseline_trace_digest: (gate == GateKind::PccMetrology).then(|| sha('3')),
            flexible_trace_digest: (gate == GateKind::PccMetrology).then(|| sha('4')),
        }
    }

    fn fixture_input() -> MobilityNetworkAdapterInput {
        let mut network = NetworkModelInput {
            network_model_id: "triangle-model-v1".to_owned(),
            network_model_digest: String::new(),
            topology_id: "topology-2026-11-24".to_owned(),
            topology_digest: String::new(),
            topology_effective_at_ns: 0,
            base_mva: 100.0,
            buses: vec![
                NetworkBusInput { id: "S".to_owned(), slack: true },
                NetworkBusInput { id: "A".to_owned(), slack: false },
                NetworkBusInput { id: "B".to_owned(), slack: false },
            ],
            branches: vec![
                NetworkBranchInput { id: "S-A".to_owned(), from_bus: "S".to_owned(), to_bus: "A".to_owned(), reactance_pu: 0.1, normal_limit_mw: 100.0, emergency_limit_mw: 120.0 },
                NetworkBranchInput { id: "A-B".to_owned(), from_bus: "A".to_owned(), to_bus: "B".to_owned(), reactance_pu: 0.1, normal_limit_mw: 40.0, emergency_limit_mw: 60.0 },
                NetworkBranchInput { id: "S-B".to_owned(), from_bus: "S".to_owned(), to_bus: "B".to_owned(), reactance_pu: 0.1, normal_limit_mw: 45.0, emergency_limit_mw: 80.0 },
            ],
        };
        network.network_model_digest = expected_network_model_digest(&network).unwrap();
        network.topology_digest = expected_topology_digest(&network).unwrap();
        let model_digest = network.network_model_digest.clone();
        let topology_digest = network.topology_digest.clone();
        MobilityNetworkAdapterInput {
            schema_version: SCHEMA_VERSION.to_owned(),
            benchmark_id: BENCHMARK_ID.to_owned(),
            offer_id: "mobility-offer-001".to_owned(),
            commitment_epoch_id: "commitment-epoch-007".to_owned(),
            site_id: "depot-01".to_owned(),
            pcc_id: "pcc-01".to_owned(),
            pcc_bus_id: "B".to_owned(),
            asset_id: "ess-01".to_owned(),
            asset_configuration_digest: sha('f'),
            baseline_trace_digest: sha('3'),
            flexible_trace_digest: sha('4'),
            decision_cutoff_unix_ms: 90_000,
            horizon_start_unix_ms: 100_000,
            horizon_end_unix_ms: 3_700_000,
            required_recovery_by_unix_ms: 3_900_000,
            maximum_topology_age_ns: 4_000_000_000_000,
            flow_tolerance_mw: 1.0e-9,
            pivot_tolerance: 1.0e-12,
            require_n_minus_one: false,
            network,
            intervals: vec![MobilityResponseIntervalInput {
                start_unix_ms: 100_000,
                duration_ms: 3_600_000,
                baseline_non_slack_injections_mw: BTreeMap::from([
                    ("A".to_owned(), 100.0),
                    ("B".to_owned(), 0.0),
                ]),
                requested_pcc_reduction_mw: 30.0,
            }],
            n_minus_one_contingencies: vec!["S-A".to_owned(), "A-B".to_owned(), "S-B".to_owned()],
            upstream_receipts: vec![
                receipt(GateKind::MobilityFlexibility, 0, GateDisposition::Passed, &model_digest, &topology_digest),
                receipt(GateKind::ReserveRecovery, 1, GateDisposition::Passed, &model_digest, &topology_digest),
                receipt(GateKind::CommitmentLease, 2, GateDisposition::Passed, &model_digest, &topology_digest),
                receipt(GateKind::CommitmentJournal, 3, GateDisposition::Passed, &model_digest, &topology_digest),
                receipt(GateKind::PccMetrology, 4, GateDisposition::Passed, &model_digest, &topology_digest),
            ],
            claim_ceiling: "synthetic known-truth adapter fixture only".to_owned(),
        }
    }

    #[test]
    fn three_bus_known_truth_reports_partial_delivery_and_energy_conservation() {
        let report = evaluate_mobility_network_adapter(&fixture_input()).unwrap();
        assert_eq!(report.disposition, CompositeDisposition::ScreenedPartialRequest);
        assert!((report.site_requested_response_energy_mwh - 30.0).abs() < 1.0e-9);
        assert!((report.maximum_dc_network_deliverable_response_energy_mwh.unwrap() - 17.5).abs() < 1.0e-8);
        assert!((report.dc_undeliverable_or_curtailed_energy_mwh.unwrap() - 12.5).abs() < 1.0e-8);
        assert!(report.energy_conservation_error_mwh.unwrap().abs() < 1.0e-9);
        assert_eq!(report.full_requested_trajectory_dc_deliverable, Some(false));
        assert_eq!(report.dc_network_report.as_ref().unwrap().interval_count, 1);
        assert_eq!(report.actual_measured_pcc_response_energy_mwh, None);
        assert!(!report.actual_metered_delivery_assessed);
        assert!(!report.adequacy_contribution_computed);
        assert!(!report.ac_feasibility_established);
        assert!(!report.market_eligibility_granted);
        assert!(!report.interconnection_approval_granted);
        assert!(!report.field_qualification_granted);
        assert!(!report.dispatch_authority_granted);
    }

    #[test]
    fn unknown_gate_holds_without_running_network_screen() {
        let mut input = fixture_input();
        input.upstream_receipts[1].disposition = GateDisposition::Unknown;
        let report = evaluate_mobility_network_adapter(&input).unwrap();
        assert_eq!(report.disposition, CompositeDisposition::HeldUnknownGate);
        assert!(report.dc_network_report.is_none());
        assert!(report.maximum_dc_network_deliverable_response_energy_mwh.is_none());
    }

    #[test]
    fn failed_gate_rejects_even_when_other_receipts_pass() {
        let mut input = fixture_input();
        input.upstream_receipts[3].disposition = GateDisposition::Failed;
        let report = evaluate_mobility_network_adapter(&input).unwrap();
        assert_eq!(report.disposition, CompositeDisposition::RejectedFailedGate);
        assert!(report.dc_network_report.is_none());
    }

    #[test]
    fn missing_gate_is_rejected() {
        let mut input = fixture_input();
        input.upstream_receipts.pop();
        assert!(matches!(
            evaluate_mobility_network_adapter(&input),
            Err(AdapterError::InvalidInput(_))
        ));
    }

    #[test]
    fn duplicate_gate_receipt_is_rejected() {
        let mut input = fixture_input();
        input.upstream_receipts[4].gate = GateKind::CommitmentJournal;
        assert!(matches!(
            evaluate_mobility_network_adapter(&input),
            Err(AdapterError::DuplicateGateReceipt(GateKind::CommitmentJournal))
        ));
    }

    #[test]
    fn stale_or_future_gate_evidence_is_rejected() {
        let mut stale = fixture_input();
        stale.upstream_receipts[0].valid_until_unix_ms = stale.horizon_end_unix_ms - 1;
        assert!(matches!(
            evaluate_mobility_network_adapter(&stale),
            Err(AdapterError::EvidenceTimeInvalid(_))
        ));

        let mut future = fixture_input();
        future.upstream_receipts[0].observed_at_unix_ms = future.decision_cutoff_unix_ms + 1;
        assert!(matches!(
            evaluate_mobility_network_adapter(&future),
            Err(AdapterError::EvidenceTimeInvalid(_))
        ));
    }

    #[test]
    fn wrong_offer_epoch_asset_or_configuration_is_rejected() {
        let mut input = fixture_input();
        input.upstream_receipts[2].commitment_epoch_id = "other-epoch".to_owned();
        assert!(matches!(
            evaluate_mobility_network_adapter(&input),
            Err(AdapterError::IdentityMismatch(_))
        ));

        let mut input = fixture_input();
        input.upstream_receipts[1].asset_id = Some("other-asset".to_owned());
        assert!(matches!(
            evaluate_mobility_network_adapter(&input),
            Err(AdapterError::IdentityMismatch(_))
        ));

        let mut input = fixture_input();
        input.upstream_receipts[1].asset_configuration_digest = Some(sha('e'));
        assert!(matches!(
            evaluate_mobility_network_adapter(&input),
            Err(AdapterError::IdentityMismatch(_))
        ));
    }

    #[test]
    fn topology_digest_mismatch_is_rejected_before_dc_screen() {
        let mut input = fixture_input();
        input.upstream_receipts[0].topology_digest = Some(sha('9'));
        assert!(matches!(
            evaluate_mobility_network_adapter(&input),
            Err(AdapterError::IdentityMismatch(_))
        ));
    }

    #[test]
    fn mobility_and_meter_receipts_must_bind_the_exact_pcc_bus() {
        let mut input = fixture_input();
        input.upstream_receipts[0].pcc_bus_id = Some("A".to_owned());
        assert!(matches!(
            evaluate_mobility_network_adapter(&input),
            Err(AdapterError::IdentityMismatch(_))
        ));
    }

    #[test]
    fn reserve_recovery_receipt_must_cover_the_next_required_deadline() {
        let mut input = fixture_input();
        input.upstream_receipts[1].valid_until_unix_ms = 3_800_000;
        assert!(matches!(
            evaluate_mobility_network_adapter(&input),
            Err(AdapterError::EvidenceTimeInvalid(_))
        ));

        let mut wrong_deadline = fixture_input();
        wrong_deadline.upstream_receipts[1].recovery_deadline_unix_ms = Some(3_850_000);
        assert!(matches!(
            evaluate_mobility_network_adapter(&wrong_deadline),
            Err(AdapterError::EvidenceTimeInvalid(_))
        ));
    }

    #[test]
    fn model_and_topology_digests_must_match_canonical_content() {
        let mut changed_model = fixture_input();
        changed_model.network.branches[0].reactance_pu = 0.2;
        assert!(matches!(
            evaluate_mobility_network_adapter(&changed_model),
            Err(AdapterError::InvalidDigest(_))
        ));

        let mut changed_topology = fixture_input();
        changed_topology.network.topology_effective_at_ns = 1;
        assert!(matches!(
            evaluate_mobility_network_adapter(&changed_topology),
            Err(AdapterError::InvalidDigest(_))
        ));
    }

    #[test]
    fn pcc_receipt_must_bind_the_exact_baseline_and_offer_trace_pair() {
        let mut input = fixture_input();
        input.upstream_receipts[4].flexible_trace_digest = Some(sha('8'));
        assert!(matches!(
            evaluate_mobility_network_adapter(&input),
            Err(AdapterError::IdentityMismatch(_))
        ));
    }

    #[test]
    fn duplicate_receipt_references_and_digests_are_rejected() {
        let mut ref_input = fixture_input();
        ref_input.upstream_receipts[1].evidence_reference =
            ref_input.upstream_receipts[0].evidence_reference.clone();
        assert!(matches!(
            evaluate_mobility_network_adapter(&ref_input),
            Err(AdapterError::DuplicateReceiptIdentity(_))
        ));

        let mut digest_input = fixture_input();
        digest_input.upstream_receipts[1].digest = digest_input.upstream_receipts[0].digest.clone();
        assert!(matches!(
            evaluate_mobility_network_adapter(&digest_input),
            Err(AdapterError::DuplicateReceiptIdentity(_))
        ));
    }

    #[test]
    fn malformed_digest_and_duplicate_contingency_fail_closed() {
        let mut digest = fixture_input();
        digest.network.topology_digest = "sha256:bad".to_owned();
        assert!(matches!(
            evaluate_mobility_network_adapter(&digest),
            Err(AdapterError::InvalidDigest(_))
        ));

        let mut duplicate = fixture_input();
        duplicate.n_minus_one_contingencies.push("S-A".to_owned());
        assert!(matches!(
            evaluate_mobility_network_adapter(&duplicate),
            Err(AdapterError::InvalidNetwork(_))
        ));
    }

    #[test]
    fn pcc_bus_must_exist_and_arbitrary_action_bus_is_never_accepted() {
        let mut input = fixture_input();
        input.pcc_bus_id = "UNKNOWN".to_owned();
        input.upstream_receipts[0].pcc_bus_id = Some("UNKNOWN".to_owned());
        input.upstream_receipts[4].pcc_bus_id = Some("UNKNOWN".to_owned());
        assert!(matches!(
            evaluate_mobility_network_adapter(&input),
            Err(AdapterError::NetworkScreen(_))
        ));
    }

    #[test]
    fn intervals_must_cover_the_exact_offer_horizon_contiguously() {
        let mut gap = fixture_input();
        gap.intervals[0].duration_ms -= 1;
        assert!(matches!(
            evaluate_mobility_network_adapter(&gap),
            Err(AdapterError::InvalidHorizon(_))
        ));

        let mut discontinuous = fixture_input();
        discontinuous.intervals = vec![
            MobilityResponseIntervalInput {
                start_unix_ms: 100_000,
                duration_ms: 1_000,
                baseline_non_slack_injections_mw: BTreeMap::new(),
                requested_pcc_reduction_mw: 1.0,
            },
            MobilityResponseIntervalInput {
                start_unix_ms: 101_001,
                duration_ms: 3_598_999,
                baseline_non_slack_injections_mw: BTreeMap::new(),
                requested_pcc_reduction_mw: 1.0,
            },
        ];
        assert!(matches!(
            evaluate_mobility_network_adapter(&discontinuous),
            Err(AdapterError::InvalidInterval(_))
        ));
    }

    #[test]
    fn receipt_order_network_order_and_contingency_order_do_not_change_digest() {
        let first = evaluate_mobility_network_adapter(&fixture_input()).unwrap();
        let mut permuted = fixture_input();
        permuted.upstream_receipts.reverse();
        permuted.network.buses.reverse();
        permuted.network.branches.reverse();
        permuted.n_minus_one_contingencies.reverse();
        let second = evaluate_mobility_network_adapter(&permuted).unwrap();
        assert_eq!(first.input_digest, second.input_digest);
        assert_eq!(first.disposition, second.disposition);
        assert_eq!(
            first.maximum_dc_network_deliverable_response_energy_mwh,
            second.maximum_dc_network_deliverable_response_energy_mwh
        );
    }

    #[test]
    fn gate_unknown_or_failure_does_not_remove_requested_energy_from_denominator() {
        let mut input = fixture_input();
        input.upstream_receipts[0].disposition = GateDisposition::Unknown;
        let report = evaluate_mobility_network_adapter(&input).unwrap();
        assert!((report.site_requested_response_energy_mwh - 30.0).abs() < 1.0e-9);
        assert!(report.maximum_dc_network_deliverable_response_energy_mwh.is_none());
    }

    #[test]
    fn required_n_minus_one_is_passed_to_existing_network_solver() {
        let mut input = fixture_input();
        input.require_n_minus_one = true;
        let report = evaluate_mobility_network_adapter(&input).unwrap();
        assert!(report.dc_network_report.is_some());
        assert!(report.dc_network_report.as_ref().unwrap().n_minus_one_required);
    }
}
