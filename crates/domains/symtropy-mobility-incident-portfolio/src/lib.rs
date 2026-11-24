// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Read-only adapter from mobility response-settlement event reports into the
//! existing incident-group-aware portfolio evaluator.
//!
//! One incident may contain many instruction events. Repeated events are not
//! treated as independent incidents: the upstream portfolio evaluator groups
//! on the explicitly supplied incident_group_id and evaluates the worst event
//! margin for that group.
//!
//! This adapter does not infer incident groups, future failure probabilities,
//! causal risk, ELCC, settlement, or operational authority.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use symtropy_flexibility_portfolio::{
    AssetAvailability, AssetDeliveryOutcome, FailureDomain, FailureDomainObservation,
    FailureDomainState, PortfolioAsset, PortfolioContract, PortfolioDefinition,
    PortfolioEventObservation, PortfolioIdentity, PortfolioResilienceReport,
    PortfolioStressEvent, evaluate_portfolio_resilience,
};

const SCHEMA_VERSION: &str = "0.1.0";
const BENCHMARK_ID: &str = "MOBILITY-INCIDENT-PORTFOLIO-001";
const SETTLEMENT_SOURCE_PR: u32 = 7321;
const SETTLEMENT_SOURCE_PATH: &str =
    "crates/domains/symtropy-mobility-response-settlement/src/lib.rs";
const MAX_ASSETS: usize = 128;
const MAX_EVENTS: usize = 10_000;
const MAX_FAILURE_DOMAINS: usize = 256;
const CLAIM_CEILING: &str = "Read-only composition of per-event mobility response settlement reports into empirical event and explicitly supplied independent-incident-group portfolio statistics. The adapter does not authenticate incident-group provenance, infer future incident probabilities or independence, establish causality, compute ELCC, authorize settlement/payment/market participation, qualify field hardware, or grant dispatch authority.";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SettlementReportArtifact {
    pub source_pr: u32,
    pub source_commit_sha: String,
    pub source_path: String,
    pub report: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MobilityPortfolioAssetInput {
    pub asset_id: String,
    pub site_id: String,
    pub pcc_id: String,
    pub measurement_boundary_id: String,
    pub offered_capacity_kw: f64,
    pub failure_domain_ids: Vec<String>,
    pub asset_evidence_id: String,
    pub asset_configuration_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MobilityPortfolioFailureDomainInput {
    pub domain_id: String,
    pub domain_kind: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MobilityFailureDomainState {
    Normal,
    Degraded,
    Failed,
    Unknown,
}

impl From<MobilityFailureDomainState> for FailureDomainState {
    fn from(value: MobilityFailureDomainState) -> Self {
        match value {
            MobilityFailureDomainState::Normal => Self::Normal,
            MobilityFailureDomainState::Degraded => Self::Degraded,
            MobilityFailureDomainState::Failed => Self::Failed,
            MobilityFailureDomainState::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MobilityPortfolioDomainObservation {
    pub domain_id: String,
    pub state: MobilityFailureDomainState,
    pub evidence_id: String,
    pub source_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MobilityPortfolioAssetEvent {
    pub asset_id: String,
    pub response_receipt_id: String,
    pub settlement_report: SettlementReportArtifact,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MobilityPortfolioEventInput {
    pub event_id: String,
    /// Must come from an independently sourced incident registry/receipt.
    /// The adapter deliberately does not infer this from time proximity.
    pub incident_group_id: String,
    pub incident_group_evidence_id: String,
    pub incident_group_source_digest: String,
    pub scenario_id: String,
    pub start_ns: u64,
    pub end_ns: u64,
    pub required_delivery_kw: f64,
    pub event_evidence_id: String,
    pub event_source_digest: String,
    pub asset_events: Vec<MobilityPortfolioAssetEvent>,
    pub failure_domain_observations: Vec<MobilityPortfolioDomainObservation>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MobilityIncidentPortfolioInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub portfolio_id: String,
    pub product_contract_id: String,
    pub minimum_event_count: usize,
    pub minimum_independent_incident_groups: usize,
    pub minimum_event_pass_fraction_basis_points: u16,
    pub minimum_incident_group_pass_fraction_basis_points: u16,
    pub shortfall_tail_alpha_basis_points: u16,
    pub assets: Vec<MobilityPortfolioAssetInput>,
    pub failure_domains: Vec<MobilityPortfolioFailureDomainInput>,
    pub events: Vec<MobilityPortfolioEventInput>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MobilityPortfolioAssetEventReceipt {
    pub event_id: String,
    pub incident_group_id: String,
    pub asset_id: String,
    pub source_pr: u32,
    pub source_commit_sha: String,
    pub settlement_report_digest: String,
    pub settlement_status: String,
    pub technical_reconciliation_passed: bool,
    /// Preserved exactly from the event report, including a negative value.
    pub signed_service_power_lower_bound_kw: Option<f64>,
    /// Only non-negative, technically reconciled values enter the portfolio engine.
    /// A negative or unqualified event is retained as unavailable/zero, never success.
    pub portfolio_delivered_lower_bound_kw: f64,
    pub portfolio_availability: AssetAvailability,
    pub exclusion_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct IncidentEvidenceReceipt {
    pub event_id: String,
    pub incident_group_id: String,
    pub incident_group_evidence_id: String,
    pub incident_group_source_digest: String,
    pub scenario_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MobilityIncidentPortfolioReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub input_digest: String,
    pub portfolio_id: String,
    pub product_contract_id: String,
    pub event_count: usize,
    pub independent_incident_group_count: usize,
    pub event_level_pass_fraction_basis_points: f64,
    pub incident_group_pass_fraction_basis_points: f64,
    pub passed_event_count: usize,
    pub passed_incident_group_count: usize,
    pub worst_incident_group_margin_kw: f64,
    pub maximum_event_shortfall_kw: f64,
    pub empirical_upper_tail_mean_incident_shortfall_kw: f64,
    pub largest_failure_domain_share_basis_points: f64,
    pub portfolio_report: PortfolioResilienceReport,
    pub asset_event_receipts: Vec<MobilityPortfolioAssetEventReceipt>,
    pub incident_evidence: Vec<IncidentEvidenceReceipt>,
    pub incident_group_membership_authenticated: bool,
    pub future_incident_probability_inferred: bool,
    pub independent_incident_assumption_proven: bool,
    pub causal_risk_established: bool,
    pub elcc_computed: bool,
    pub settlement_authority_granted: bool,
    pub payment_authorized: bool,
    pub market_eligibility_granted: bool,
    pub dispatch_authority_granted: bool,
    pub field_qualification_granted: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MobilityPortfolioError {
    InvalidInput(String),
    InvalidSourceArtifact(String),
    IdentityMismatch(String),
    DuplicateIdentity(String),
    MissingOutcome(String),
    InvalidIncidentEvidence(String),
    InvalidSettlementEvidence(String),
    InvalidPortfolio(String),
    Arithmetic(String),
}

impl core::fmt::Display for MobilityPortfolioError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput(message)
            | Self::InvalidSourceArtifact(message)
            | Self::IdentityMismatch(message)
            | Self::DuplicateIdentity(message)
            | Self::MissingOutcome(message)
            | Self::InvalidIncidentEvidence(message)
            | Self::InvalidSettlementEvidence(message)
            | Self::InvalidPortfolio(message)
            | Self::Arithmetic(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for MobilityPortfolioError {}

fn require_text(field: &str, value: &str) -> Result<(), MobilityPortfolioError> {
    if value.trim().is_empty() {
        Err(MobilityPortfolioError::InvalidInput(format!(
            "{field} must not be empty"
        )))
    } else {
        Ok(())
    }
}

fn sha256_shape(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn commit_shape(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn canonical_digest<T: Serialize>(value: &T, field: &str) -> Result<String, MobilityPortfolioError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| MobilityPortfolioError::Arithmetic(format!("{field} serialization: {error}")))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn json_string(report: &Value, key: &str) -> Result<String, MobilityPortfolioError> {
    report
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| MobilityPortfolioError::InvalidSettlementEvidence(format!(
            "settlement report field {key} must be a string"
        )))
}

fn json_bool(report: &Value, key: &str) -> Result<bool, MobilityPortfolioError> {
    report
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| MobilityPortfolioError::InvalidSettlementEvidence(format!(
            "settlement report field {key} must be boolean"
        )))
}

fn json_f64(report: &Value, key: &str) -> Result<f64, MobilityPortfolioError> {
    let value = report
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| MobilityPortfolioError::InvalidSettlementEvidence(format!(
            "settlement report field {key} must be numeric"
        )))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(MobilityPortfolioError::InvalidSettlementEvidence(format!(
            "settlement report field {key} must be finite"
        )))
    }
}

fn json_optional_f64(report: &Value, key: &str) -> Result<Option<f64>, MobilityPortfolioError> {
    let value = report.get(key).ok_or_else(|| {
        MobilityPortfolioError::InvalidSettlementEvidence(format!(
            "settlement report field {key} is missing"
        ))
    })?;
    if value.is_null() {
        return Ok(None);
    }
    let number = value.as_f64().ok_or_else(|| {
        MobilityPortfolioError::InvalidSettlementEvidence(format!(
            "settlement report field {key} must be numeric or null"
        ))
    })?;
    if number.is_finite() {
        Ok(Some(number))
    } else {
        Err(MobilityPortfolioError::InvalidSettlementEvidence(format!(
            "settlement report field {key} must be finite"
        )))
    }
}

fn canonicalize_input(input: &MobilityIncidentPortfolioInput) -> MobilityIncidentPortfolioInput {
    let mut canonical = input.clone();
    canonical.assets.sort_by(|a, b| a.asset_id.cmp(&b.asset_id));
    for asset in &mut canonical.assets {
        asset.failure_domain_ids.sort();
    }
    canonical.failure_domains.sort_by(|a, b| a.domain_id.cmp(&b.domain_id));
    canonical.events.sort_by(|a, b| {
        (a.start_ns, a.event_id.as_str()).cmp(&(b.start_ns, b.event_id.as_str()))
    });
    for event in &mut canonical.events {
        event.asset_events.sort_by(|a, b| a.asset_id.cmp(&b.asset_id));
        event
            .failure_domain_observations
            .sort_by(|a, b| a.domain_id.cmp(&b.domain_id));
    }
    canonical
}

fn validate_bundle(input: &MobilityIncidentPortfolioInput) -> Result<(), MobilityPortfolioError> {
    if input.schema_version != SCHEMA_VERSION || input.benchmark_id != BENCHMARK_ID {
        return Err(MobilityPortfolioError::InvalidInput(
            "schema or benchmark identity mismatch".to_owned(),
        ));
    }
    for (field, value) in [
        ("portfolio_id", input.portfolio_id.as_str()),
        ("product_contract_id", input.product_contract_id.as_str()),
        ("claim_ceiling", input.claim_ceiling.as_str()),
    ] {
        require_text(field, value)?;
    }
    if input.assets.is_empty()
        || input.assets.len() > MAX_ASSETS
        || input.failure_domains.is_empty()
        || input.failure_domains.len() > MAX_FAILURE_DOMAINS
        || input.events.is_empty()
        || input.events.len() > MAX_EVENTS
    {
        return Err(MobilityPortfolioError::InvalidInput(
            "portfolio asset/domain/event counts outside configured bounds".to_owned(),
        ));
    }

    let mut asset_ids = BTreeSet::new();
    let mut site_pcc_pairs = BTreeSet::new();
    for asset in &input.assets {
        for (field, value) in [
            ("asset ID", asset.asset_id.as_str()),
            ("site ID", asset.site_id.as_str()),
            ("PCC ID", asset.pcc_id.as_str()),
            ("measurement boundary ID", asset.measurement_boundary_id.as_str()),
            ("asset evidence ID", asset.asset_evidence_id.as_str()),
        ] {
            require_text(field, value)?;
        }
        if !sha256_shape(&asset.asset_configuration_digest) {
            return Err(MobilityPortfolioError::InvalidInput(
                "asset configuration digest must be sha256:<64 hex>".to_owned(),
            ));
        }
        if !asset.offered_capacity_kw.is_finite() || asset.offered_capacity_kw <= 0.0 {
            return Err(MobilityPortfolioError::InvalidInput(
                "offered asset capacity must be finite and positive".to_owned(),
            ));
        }
        if !asset_ids.insert(asset.asset_id.as_str()) {
            return Err(MobilityPortfolioError::DuplicateIdentity(format!(
                "duplicate asset ID {}",
                asset.asset_id
            )));
        }
        if !site_pcc_pairs.insert((asset.site_id.as_str(), asset.pcc_id.as_str())) {
            return Err(MobilityPortfolioError::InvalidInput(format!(
                "v0.1 portfolio evaluator allows one enrolled portfolio asset per site/PCC; aggregate co-located packs into a PCC resource before enrollment: {}/{}",
                asset.site_id, asset.pcc_id
            )));
        }
        if asset.failure_domain_ids.is_empty() {
            return Err(MobilityPortfolioError::InvalidInput(format!(
                "asset {} must declare at least one failure domain",
                asset.asset_id
            )));
        }
        let mut local_domains = BTreeSet::new();
        for domain in &asset.failure_domain_ids {
            require_text("asset failure-domain membership", domain)?;
            if !local_domains.insert(domain.as_str()) {
                return Err(MobilityPortfolioError::DuplicateIdentity(format!(
                    "asset {} repeats failure domain {domain}",
                    asset.asset_id
                )));
            }
        }
    }

    let mut domain_ids = BTreeSet::new();
    for domain in &input.failure_domains {
        require_text("failure-domain ID", &domain.domain_id)?;
        require_text("failure-domain kind", &domain.domain_kind)?;
        if !domain_ids.insert(domain.domain_id.as_str()) {
            return Err(MobilityPortfolioError::DuplicateIdentity(format!(
                "duplicate failure domain {}",
                domain.domain_id
            )));
        }
    }
    for asset in &input.assets {
        for domain in &asset.failure_domain_ids {
            if !domain_ids.contains(domain.as_str()) {
                return Err(MobilityPortfolioError::IdentityMismatch(format!(
                    "asset {} references undeclared failure domain {domain}",
                    asset.asset_id
                )));
            }
        }
    }

    let mut event_ids = BTreeSet::new();
    let mut event_receipt_digests = BTreeSet::new();
    let mut asset_evidence_ids = BTreeSet::new();
    let mut incident_groups: BTreeMap<&str, (&str, &str, &str)> = BTreeMap::new();
    let mut previous_start: Option<u64> = None;
    let mut all_asset_ids = asset_ids.clone();
    for event in &input.events {
        for (field, value) in [
            ("event ID", event.event_id.as_str()),
            ("incident group ID", event.incident_group_id.as_str()),
            ("incident group evidence ID", event.incident_group_evidence_id.as_str()),
            ("scenario ID", event.scenario_id.as_str()),
            ("event evidence ID", event.event_evidence_id.as_str()),
        ] {
            require_text(field, value)?;
        }
        if !sha256_shape(&event.incident_group_source_digest)
            || !sha256_shape(&event.event_source_digest)
        {
            return Err(MobilityPortfolioError::InvalidIncidentEvidence(
                "incident and event source digests must be SHA-256 shaped".to_owned(),
            ));
        }
        if event.start_ns == 0 || event.end_ns <= event.start_ns
            || !event.required_delivery_kw.is_finite()
            || event.required_delivery_kw <= 0.0
        {
            return Err(MobilityPortfolioError::InvalidInput(format!(
                "event {} has invalid time or required capacity",
                event.event_id
            )));
        }
        if previous_start.is_some_and(|previous| event.start_ns < previous) {
            return Err(MobilityPortfolioError::InvalidInput(
                "events must be provided in non-decreasing start-time order".to_owned(),
            ));
        }
        previous_start = Some(event.start_ns);
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(MobilityPortfolioError::DuplicateIdentity(format!(
                "duplicate event ID {}",
                event.event_id
            )));
        }
        if !event_receipt_digests.insert(event.event_source_digest.as_str()) {
            return Err(MobilityPortfolioError::DuplicateIdentity(
                "duplicate event source digest; event replay is not a new incident".to_owned(),
            ));
        }
        match incident_groups.get(event.incident_group_id.as_str()) {
            Some((evidence_id, source_digest, scenario))
                if *evidence_id != event.incident_group_evidence_id
                    || *source_digest != event.incident_group_source_digest
                    || *scenario != event.scenario_id =>
            {
                return Err(MobilityPortfolioError::InvalidIncidentEvidence(format!(
                    "incident group {} has conflicting evidence or scenario membership",
                    event.incident_group_id
                )));
            }
            Some(_) => {}
            None => {
                incident_groups.insert(
                    event.incident_group_id.as_str(),
                    (
                        event.incident_group_evidence_id.as_str(),
                        event.incident_group_source_digest.as_str(),
                        event.scenario_id.as_str(),
                    ),
                );
            }
        }
        if event.asset_events.len() != asset_ids.len() {
            return Err(MobilityPortfolioError::MissingOutcome(format!(
                "event {} has {} asset outcomes; expected {}",
                event.event_id,
                event.asset_events.len(),
                asset_ids.len()
            )));
        }
        let mut event_asset_ids = BTreeSet::new();
        for asset_event in &event.asset_events {
            if !asset_ids.contains(asset_event.asset_id.as_str()) {
                return Err(MobilityPortfolioError::IdentityMismatch(format!(
                    "event {} contains unregistered asset {}",
                    event.event_id, asset_event.asset_id
                )));
            }
            if !event_asset_ids.insert(asset_event.asset_id.as_str()) {
                return Err(MobilityPortfolioError::DuplicateIdentity(format!(
                    "event {} repeats asset {}",
                    event.event_id, asset_event.asset_id
                )));
            }
            if asset_event.settlement_report.source_pr != SETTLEMENT_SOURCE_PR
                || asset_event.settlement_report.source_path != SETTLEMENT_SOURCE_PATH
                || !commit_shape(&asset_event.settlement_report.source_commit_sha)
                || !asset_event.settlement_report.report.is_object()
            {
                return Err(MobilityPortfolioError::InvalidSourceArtifact(format!(
                    "event {} asset {} report must identify PR #{SETTLEMENT_SOURCE_PR}, exact source path and a hexadecimal source commit",
                    event.event_id, asset_event.asset_id
                )));
            }
            let report = &asset_event.settlement_report.report;
            if json_string(report, "event_id")? != event.event_id {
                return Err(MobilityPortfolioError::IdentityMismatch(format!(
                    "settlement report event ID does not match portfolio event {}",
                    event.event_id
                )));
            }
            if json_string(report, "asset_id")? != asset_event.asset_id {
                return Err(MobilityPortfolioError::IdentityMismatch(format!(
                    "settlement report asset ID does not match portfolio asset {}",
                    asset_event.asset_id
                )));
            }
            if json_string(report, "site_id")? != input.assets.iter().find(|a| a.asset_id == asset_event.asset_id).unwrap().site_id
                || json_string(report, "pcc_id")? != input.assets.iter().find(|a| a.asset_id == asset_event.asset_id).unwrap().pcc_id
                || json_string(report, "measurement_boundary_id")? != input.assets.iter().find(|a| a.asset_id == asset_event.asset_id).unwrap().measurement_boundary_id
                || json_string(report, "asset_configuration_digest")? != input.assets.iter().find(|a| a.asset_id == asset_event.asset_id).unwrap().asset_configuration_digest
            {
                return Err(MobilityPortfolioError::IdentityMismatch(format!(
                    "settlement report site/PCC/measurement/configuration does not match asset {}",
                    asset_event.asset_id
                )));
            }
            if json_string(report, "status")? == "rejected_mismatch" {
                return Err(MobilityPortfolioError::InvalidSettlementEvidence(format!(
                    "event {} asset {} has a rejected settlement reconciliation",
                    event.event_id, asset_event.asset_id
                )));
            }
            let source_digest = json_string(report, "input_digest")?;
            if !sha256_shape(&source_digest) {
                return Err(MobilityPortfolioError::InvalidSettlementEvidence(
                    "settlement report input digest must be SHA-256 shaped".to_owned(),
                ));
            }
            if !asset_evidence_ids.insert((asset_event.asset_id.as_str(), source_digest.as_str())) {
                return Err(MobilityPortfolioError::DuplicateIdentity(format!(
                    "asset {} reuses settlement source digest",
                    asset_event.asset_id
                )));
            }
        }

        let mut domain_observation_ids = BTreeSet::new();
        for observation in &event.failure_domain_observations {
            if !domain_ids.contains(observation.domain_id.as_str()) {
                return Err(MobilityPortfolioError::IdentityMismatch(format!(
                    "event {} references undeclared failure domain {}",
                    event.event_id, observation.domain_id
                )));
            }
            if !domain_observation_ids.insert(observation.domain_id.as_str()) {
                return Err(MobilityPortfolioError::DuplicateIdentity(format!(
                    "event {} repeats failure domain {}",
                    event.event_id, observation.domain_id
                )));
            }
            require_text("domain evidence ID", &observation.evidence_id)?;
            if !sha256_shape(&observation.source_digest) {
                return Err(MobilityPortfolioError::InvalidIncidentEvidence(
                    "failure-domain source digest must be SHA-256 shaped".to_owned(),
                ));
            }
        }
        if domain_observation_ids != domain_ids {
            return Err(MobilityPortfolioError::MissingOutcome(format!(
                "event {} lacks a state observation for one or more declared failure domains",
                event.event_id
            )));
        }
    }

    if event_ids.is_empty() || incident_groups.is_empty() {
        return Err(MobilityPortfolioError::InvalidInput(
            "portfolio panel must contain at least one event and incident group".to_owned(),
        ));
    }

    Ok(())
}

fn portfolio_digest(input: &MobilityIncidentPortfolioInput) -> Result<String, MobilityPortfolioError> {
    canonical_digest(&canonicalize_input(input), "mobility incident portfolio input")
}

pub fn evaluate_mobility_incident_portfolio(
    input: &MobilityIncidentPortfolioInput,
) -> Result<MobilityIncidentPortfolioReport, MobilityPortfolioError> {
    validate_bundle(input)?;
    let canonical_input = canonicalize_input(input);
    let input_digest = portfolio_digest(input)?;

    let asset_manifest = canonical_digest(&canonical_input.assets, "asset manifest")?;
    let domain_manifest = canonical_digest(&canonical_input.failure_domains, "failure-domain manifest")?;
    let identity = PortfolioIdentity::new(
        input.portfolio_id.clone(),
        input.product_contract_id.clone(),
        asset_manifest,
        domain_manifest,
    )
    .map_err(|error| MobilityPortfolioError::InvalidPortfolio(error.to_string()))?;

    let contract = PortfolioContract::new(
        input.product_contract_id.clone(),
        input.minimum_event_count,
        input.minimum_independent_incident_groups,
        input.minimum_event_pass_fraction_basis_points,
        input.minimum_incident_group_pass_fraction_basis_points,
        input.shortfall_tail_alpha_basis_points,
    )
    .map_err(|error| MobilityPortfolioError::InvalidPortfolio(error.to_string()))?;

    let failure_domains = canonical_input
        .failure_domains
        .iter()
        .map(|domain| {
            FailureDomain::new(domain.domain_id.clone(), domain.domain_kind.clone())
                .map_err(|error| MobilityPortfolioError::InvalidPortfolio(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let assets = canonical_input
        .assets
        .iter()
        .map(|asset| {
            let mut memberships = asset.failure_domain_ids.clone();
            memberships.sort();
            PortfolioAsset::new(
                asset.asset_id.clone(),
                asset.site_id.clone(),
                asset.pcc_id.clone(),
                asset.measurement_boundary_id.clone(),
                asset.offered_capacity_kw,
                memberships,
                asset.asset_evidence_id.clone(),
            )
            .map_err(|error| MobilityPortfolioError::InvalidPortfolio(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let portfolio = PortfolioDefinition::new(identity, contract, assets, failure_domains)
        .map_err(|error| MobilityPortfolioError::InvalidPortfolio(error.to_string()))?;

    let mut event_observations = Vec::with_capacity(canonical_input.events.len());
    let mut asset_event_receipts = Vec::with_capacity(
        canonical_input.events.len() * canonical_input.assets.len(),
    );
    let mut incident_evidence = Vec::with_capacity(canonical_input.events.len());

    for event in &canonical_input.events {
        let event_digest = canonical_digest(event, "portfolio event")?;
        let stress_event = PortfolioStressEvent::new(
            event.event_id.clone(),
            event.incident_group_id.clone(),
            event.scenario_id.clone(),
            event.start_ns,
            event.end_ns,
            event.required_delivery_kw,
            event.event_evidence_id.clone(),
            event_digest.clone(),
        )
        .map_err(|error| MobilityPortfolioError::InvalidPortfolio(error.to_string()))?;

        let mut outcomes = Vec::with_capacity(event.asset_events.len());
        for asset_event in &event.asset_events {
            let asset = canonical_input.assets.iter().find(|a| a.asset_id == asset_event.asset_id).unwrap();
            let report = &asset_event.settlement_report.report;
            let status = json_string(report, "status")?;
            let technically_reconciled = json_bool(report, "technical_reconciliation_passed")?;
            let signed_lower_bound =
                json_optional_f64(report, "minimum_dispatch_interval_lower_bound_kw")?;
            let mut exclusion_reason = None;

            let (delivered_lower_bound_kw, availability) = match (technically_reconciled, signed_lower_bound) {
                (true, Some(value)) if value >= 0.0 => {
                    if value > asset.offered_capacity_kw {
                        return Err(MobilityPortfolioError::InvalidSettlementEvidence(format!(
                            "asset {} report power lower bound exceeds registered offered capacity",
                            asset_event.asset_id
                        )));
                    }
                    (value, AssetAvailability::Available)
                }
                (true, Some(value)) => {
                    exclusion_reason = Some("negative_signed_service_power_increased_import".to_owned());
                    (0.0, AssetAvailability::Unavailable)
                }
                (true, None) => {
                    return Err(MobilityPortfolioError::InvalidSettlementEvidence(format!(
                        "technically reconciled event {} asset {} lacks dispatch-period power lower bound",
                        event.event_id, asset_event.asset_id
                    )));
                }
                (false, _) => {
                    exclusion_reason = Some(format!("upstream_event_not_technically_reconciled:{status}"));
                    (0.0, AssetAvailability::Unavailable)
                }
            };

            let source_digest = json_string(report, "input_digest")?;
            let response_receipt_id = format!(
                "{}:{}:{}",
                event.event_id, asset_event.asset_id, source_digest
            );
            outcomes.push(
                AssetDeliveryOutcome::new(
                    asset_event.asset_id.clone(),
                    delivered_lower_bound_kw,
                    availability,
                    response_receipt_id.clone(),
                    source_digest.clone(),
                )
                .map_err(|error| MobilityPortfolioError::InvalidPortfolio(error.to_string()))?,
            );
            asset_event_receipts.push(MobilityPortfolioAssetEventReceipt {
                event_id: event.event_id.clone(),
                incident_group_id: event.incident_group_id.clone(),
                asset_id: asset_event.asset_id.clone(),
                source_pr: asset_event.settlement_report.source_pr,
                source_commit_sha: asset_event.settlement_report.source_commit_sha.clone(),
                settlement_report_digest: canonical_digest(report, "settlement report")?,
                settlement_status: status,
                technical_reconciliation_passed: technically_reconciled,
                signed_service_power_lower_bound_kw: signed_lower_bound,
                portfolio_delivered_lower_bound_kw: delivered_lower_bound_kw,
                portfolio_availability: availability,
                exclusion_reason,
            });
        }

        let mut domain_observations = Vec::with_capacity(event.failure_domain_observations.len());
        for observation in &event.failure_domain_observations {
            domain_observations.push(
                FailureDomainObservation::new(
                    observation.domain_id.clone(),
                    observation.state.into(),
                    observation.evidence_id.clone(),
                    observation.source_digest.clone(),
                )
                .map_err(|error| MobilityPortfolioError::InvalidPortfolio(error.to_string()))?,
            );
        }

        event_observations.push(PortfolioEventObservation::new(
            stress_event,
            outcomes,
            domain_observations,
        ));
        incident_evidence.push(IncidentEvidenceReceipt {
            event_id: event.event_id.clone(),
            incident_group_id: event.incident_group_id.clone(),
            incident_group_evidence_id: event.incident_group_evidence_id.clone(),
            incident_group_source_digest: event.incident_group_source_digest.clone(),
            scenario_id: event.scenario_id.clone(),
        });
    }

    let portfolio_report = evaluate_portfolio_resilience(&portfolio, &event_observations)
        .map_err(|error| MobilityPortfolioError::InvalidPortfolio(error.to_string()))?;

    Ok(MobilityIncidentPortfolioReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        input_digest,
        portfolio_id: input.portfolio_id.clone(),
        product_contract_id: input.product_contract_id.clone(),
        event_count: portfolio_report.event_count,
        independent_incident_group_count: portfolio_report.independent_incident_group_count,
        event_level_pass_fraction_basis_points: portfolio_report.event_pass_fraction_basis_points,
        incident_group_pass_fraction_basis_points: portfolio_report.incident_group_pass_fraction_basis_points,
        passed_event_count: portfolio_report.passed_event_count,
        passed_incident_group_count: portfolio_report.passed_incident_group_count,
        worst_incident_group_margin_kw: portfolio_report.worst_incident_group_margin_kw,
        maximum_event_shortfall_kw: portfolio_report.maximum_event_shortfall_kw,
        empirical_upper_tail_mean_incident_shortfall_kw: portfolio_report.empirical_upper_tail_mean_incident_shortfall_kw,
        largest_failure_domain_share_basis_points: portfolio_report.largest_failure_domain_share_basis_points,
        portfolio_report,
        asset_event_receipts,
        incident_evidence,
        incident_group_membership_authenticated: false,
        future_incident_probability_inferred: false,
        independent_incident_assumption_proven: false,
        causal_risk_established: false,
        elcc_computed: false,
        settlement_authority_granted: false,
        payment_authorized: false,
        market_eligibility_granted: false,
        dispatch_authority_granted: false,
        field_qualification_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}
