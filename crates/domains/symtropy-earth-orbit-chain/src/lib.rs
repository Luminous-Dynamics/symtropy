// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Effects-disabled Earth-to-orbit logistics chain admission and accounting.
//!
//! This crate does not optimize routes, arbitrate resource capacity, or solve
//! transport / orbital dynamics. It consumes a planning-qualification receipt
//! from the authoritative time-windowed resource allocator (Symtropy #1639),
//! reconciles stable cargo mass and ordered transfers, and consumes an orbit
//! compatibility receipt from the launch/orbital specialist.
//!
//! A receipt digest is an identity commitment, not proof of truth by itself.
//! The caller must independently verify its issuer, signature, source lineage,
//! policy, model context and validity before constructing this input.
//!
//! The first increment is intentionally conservative:
//! - payload mass remains invariant across each transport hop;
//! - upstream resource capacity is not recalculated or double-owned here;
//! - operation intervals are half-open [start, end);
//! - unknown resource or orbit evidence never becomes a pass;
//! - fault scenarios are deterministic sets, not invented probabilities;
//! - a passing plan is not a booking, permit, launch authorization or safety case.

use std::collections::{BTreeMap, BTreeSet};

/// Schema version for the first Earth-to-orbit chain contract.
pub const SCHEMA_VERSION: &str = "symtropy.earth-orbit-chain.v1";

/// Evidence state for a specialist qualification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceStatus {
    /// A caller-verified qualification receipt is expected to be supplied.
    /// This crate validates only that its SHA-256 identity is well-formed.
    Qualified { receipt_sha256: String },
    /// No usable evidence is available. This is never interpreted as zero risk.
    Unknown,
    /// The current model or evidence adapter does not support this claim.
    Unsupported { reason: String },
}

/// Status returned by the authoritative physical-resource capacity owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceQualificationStatus {
    /// The resource owner qualified this use for planning over the stated window.
    /// This does not reserve the resource or authorize dispatch.
    QualifiedForPlanning { receipt_sha256: String },
    /// The resource owner rejected the proposed operation.
    Rejected { reason: String },
    /// The resource owner has not established a usable result.
    Unknown,
    /// The resource owner cannot assess this resource or operation class.
    Unsupported { reason: String },
}

/// Functional role of a node in the supplied chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeRole {
    Supplier,
    InlandTerminal,
    PortOrAirportGateway,
    PayloadIntegration,
    LaunchRange,
    LaunchVehicle,
    OrbitalTransfer,
    OrbitDestination,
    Other,
}

/// A facility, terminal, vehicle/service boundary, or mission endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainNodeV1 {
    pub id: String,
    pub role: NodeRole,
}

/// Cargo mass is integer grams so this accounting layer can reconcile it exactly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoLotV1 {
    pub id: String,
    pub mass_grams: u64,
}

/// An opaque planning qualification from the transport resource owner.
///
/// This is intentionally a receipt, not a second representation of capacity.
/// Shared-resource and interval-capacity calculations remain owned by #1639.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceQualificationV1 {
    pub id: String,
    pub resource_id: String,
    pub valid_from_epoch_seconds: u64,
    pub valid_until_epoch_seconds: u64,
    pub status: ResourceQualificationStatus,
}

/// One cargo lot's occupation of one physical resource between two chain nodes.
///
/// start/end describe the whole resource occupancy interval for this v1 accounting
/// contract. More detailed timetabling, dwell and service calendars remain owned
/// by the transport / launch specialist models.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferV1 {
    pub id: String,
    pub cargo_lot_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub resource_id: String,
    pub resource_qualification_id: String,
    pub mass_grams: u64,
    pub starts_at_epoch_seconds: u64,
    pub ends_at_epoch_seconds: u64,
    /// Physical single points or common dependencies used by this transfer:
    /// e.g. rail section, access road, port, power feeder, tracking site, or range.
    pub dependency_ids: Vec<String>,
}

/// The destination orbit/service and its specialist compatibility receipt.
///
/// The receipt is produced by the orbital mechanics / launch performance owner.
/// This crate does not recompute inclination, azimuth, trajectory or insertion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrbitServiceV1 {
    pub target_orbit_id: String,
    pub destination_node_id: String,
    pub compatibility: EvidenceStatus,
}

/// A deterministic disruption case. No probability is implied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaultScenarioV1 {
    pub id: String,
    pub failed_dependency_ids: Vec<String>,
}

/// Complete, caller-supplied chain for one campaign.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampaignV1 {
    pub id: String,
    pub cargo_lots: Vec<CargoLotV1>,
    pub nodes: Vec<ChainNodeV1>,
    pub resource_qualifications: Vec<ResourceQualificationV1>,
    /// Transfers are ordered by their cargo lot's time intervals.
    pub transfers: Vec<TransferV1>,
    pub orbit_service: OrbitServiceV1,
    pub required_delivery_by_epoch_seconds: u64,
    pub fault_scenarios: Vec<FaultScenarioV1>,
}

/// Stable finding identifiers; consumers should branch on these values, not prose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingCode {
    InvalidIdentifier,
    DuplicateIdentifier,
    EmptyCampaign,
    EmptyCargoLot,
    EmptyChain,
    UnknownCargoLot,
    UnknownNode,
    MissingResourceQualification,
    UnknownResourceQualification,
    RejectedResourceQualification,
    UnsupportedResourceQualification,
    ResourceQualificationResourceMismatch,
    ResourceQualificationDoesNotCoverTransfer,
    TransferMassMismatch,
    DiscontinuousCargoPath,
    OverlappingCargoHops,
    CargoPathCycle,
    WrongOrbitDestination,
    LateDelivery,
    ArithmeticOverflow,
    MalformedReceiptDigest,
    EvidenceUnknown,
    EvidenceUnsupported,
    DuplicateDependency,
    UnknownFailureDomain,
    DuplicateFaultScenario,
}

/// One deterministic validation finding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub code: FindingCode,
    pub subject_id: String,
}

/// Result class; evidence gaps are not represented as feasible or infeasible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampaignDisposition {
    Ready,
    NeedsEvidence,
    Infeasible,
}

/// Result for one deterministic failure-domain set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaultOutcomeV1 {
    pub scenario_id: String,
    pub delivered_mass_grams: u64,
    pub unmet_mass_grams: u64,
    pub failed_transfer_ids: Vec<String>,
}

/// Independent accounting and admission report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampaignReportV1 {
    pub schema_version: &'static str,
    pub disposition: CampaignDisposition,
    pub total_cargo_mass_grams: u64,
    /// Zero unless the entire baseline campaign is Ready.
    pub accepted_delivered_cargo_mass_grams: u64,
    pub findings: Vec<Finding>,
    pub fault_outcomes: Vec<FaultOutcomeV1>,
}

/// Evaluate a supplied chain. No IO, ambient clock, random state or optimization.
pub fn evaluate_campaign(campaign: &CampaignV1) -> CampaignReportV1 {
    let mut findings = Vec::new();

    if !valid_id(&campaign.id)
        || !valid_id(&campaign.orbit_service.target_orbit_id)
        || !valid_id(&campaign.orbit_service.destination_node_id)
    {
        push_finding(&mut findings, FindingCode::InvalidIdentifier, &campaign.id);
    }
    if campaign.cargo_lots.is_empty() {
        push_finding(&mut findings, FindingCode::EmptyCampaign, &campaign.id);
    }
    if campaign.transfers.is_empty() {
        push_finding(&mut findings, FindingCode::EmptyChain, &campaign.id);
    }

    let mut cargo_by_id = BTreeMap::new();
    let mut node_by_id = BTreeMap::new();
    let mut qualification_by_id = BTreeMap::new();
    let mut transfer_ids = BTreeSet::new();
    let mut known_dependencies = BTreeSet::new();

    for cargo in &campaign.cargo_lots {
        if !valid_id(&cargo.id) {
            push_finding(&mut findings, FindingCode::InvalidIdentifier, &cargo.id);
        }
        if cargo.mass_grams == 0 {
            push_finding(&mut findings, FindingCode::EmptyCargoLot, &cargo.id);
        }
        if cargo_by_id.insert(cargo.id.clone(), cargo).is_some() {
            push_finding(&mut findings, FindingCode::DuplicateIdentifier, &cargo.id);
        }
    }

    for node in &campaign.nodes {
        if !valid_id(&node.id) {
            push_finding(&mut findings, FindingCode::InvalidIdentifier, &node.id);
        }
        if node_by_id.insert(node.id.clone(), node).is_some() {
            push_finding(&mut findings, FindingCode::DuplicateIdentifier, &node.id);
        }
    }

    for qualification in &campaign.resource_qualifications {
        if !valid_id(&qualification.id) || !valid_id(&qualification.resource_id) {
            push_finding(
                &mut findings,
                FindingCode::InvalidIdentifier,
                &qualification.id,
            );
        }
        if qualification.valid_from_epoch_seconds >= qualification.valid_until_epoch_seconds {
            push_finding(
                &mut findings,
                FindingCode::InvalidTimeInterval,
                &qualification.id,
            );
        }
        validate_resource_status(
            &qualification.status,
            &qualification.id,
            &mut findings,
        );
        if qualification_by_id
            .insert(qualification.id.clone(), qualification)
            .is_some()
        {
            push_finding(
                &mut findings,
                FindingCode::DuplicateIdentifier,
                &qualification.id,
            );
        }
    }

    let mut transfers_by_cargo: BTreeMap<String, Vec<&TransferV1>> = BTreeMap::new();
    for transfer in &campaign.transfers {
        if !valid_id(&transfer.id)
            || !valid_id(&transfer.cargo_lot_id)
            || !valid_id(&transfer.from_node_id)
            || !valid_id(&transfer.to_node_id)
            || !valid_id(&transfer.resource_id)
            || !valid_id(&transfer.resource_qualification_id)
        {
            push_finding(&mut findings, FindingCode::InvalidIdentifier, &transfer.id);
        }
        if !transfer_ids.insert(transfer.id.clone()) {
            push_finding(&mut findings, FindingCode::DuplicateIdentifier, &transfer.id);
        }
        if !cargo_by_id.contains_key(&transfer.cargo_lot_id) {
            push_finding(&mut findings, FindingCode::UnknownCargoLot, &transfer.cargo_lot_id);
        } else if let Some(cargo) = cargo_by_id.get(&transfer.cargo_lot_id) {
            if transfer.mass_grams != cargo.mass_grams {
                push_finding(&mut findings, FindingCode::TransferMassMismatch, &transfer.id);
            }
            transfers_by_cargo
                .entry(transfer.cargo_lot_id.clone())
                .or_default()
                .push(transfer);
        }
        if !node_by_id.contains_key(&transfer.from_node_id) {
            push_finding(&mut findings, FindingCode::UnknownNode, &transfer.from_node_id);
        }
        if !node_by_id.contains_key(&transfer.to_node_id) {
            push_finding(&mut findings, FindingCode::UnknownNode, &transfer.to_node_id);
        }
        if transfer.from_node_id == transfer.to_node_id {
            push_finding(&mut findings, FindingCode::SelfTransfer, &transfer.id);
        }
        if transfer.starts_at_epoch_seconds >= transfer.ends_at_epoch_seconds {
            push_finding(&mut findings, FindingCode::InvalidTimeInterval, &transfer.id);
        }
        let mut local_dependencies = BTreeSet::new();
        for dependency_id in &transfer.dependency_ids {
            if !valid_id(dependency_id) {
                push_finding(&mut findings, FindingCode::InvalidIdentifier, dependency_id);
            }
            if !local_dependencies.insert(dependency_id.clone()) {
                push_finding(&mut findings, FindingCode::DuplicateDependency, dependency_id);
            }
            known_dependencies.insert(dependency_id.clone());
        }

        match qualification_by_id.get(&transfer.resource_qualification_id) {
            None => push_finding(
                &mut findings,
                FindingCode::MissingResourceQualification,
                &transfer.resource_qualification_id,
            ),
            Some(qualification) => {
                if qualification.resource_id != transfer.resource_id {
                    push_finding(
                        &mut findings,
                        FindingCode::ResourceQualificationResourceMismatch,
                        &transfer.id,
                    );
                }
                if transfer.starts_at_epoch_seconds < qualification.valid_from_epoch_seconds
                    || transfer.ends_at_epoch_seconds > qualification.valid_until_epoch_seconds
                {
                    push_finding(
                        &mut findings,
                        FindingCode::ResourceQualificationDoesNotCoverTransfer,
                        &transfer.id,
                    );
                }
                if matches!(
                    &qualification.status,
                    ResourceQualificationStatus::Unknown
                ) {
                    push_finding(
                        &mut findings,
                        FindingCode::UnknownResourceQualification,
                        &qualification.id,
                    );
                }
            }
        }
    }

    // Check cargo continuity in time order; input insertion order does not affect
    // the result. A v1 cargo lot follows one linear route, not split/merged flow.
    for cargo in &campaign.cargo_lots {
        let mut legs = transfers_by_cargo.remove(&cargo.id).unwrap_or_default();
        if legs.is_empty() {
            push_finding(&mut findings, FindingCode::EmptyChain, &cargo.id);
            continue;
        }
        legs.sort_by_key(|leg| {
            (
                leg.starts_at_epoch_seconds,
                leg.ends_at_epoch_seconds,
                leg.id.clone(),
            )
        });
        let mut visited_nodes = BTreeSet::new();
        if let Some(first) = legs.first() {
            visited_nodes.insert(first.from_node_id.clone());
        }
        for leg in &legs {
            if !visited_nodes.insert(leg.to_node_id.clone()) {
                push_finding(&mut findings, FindingCode::CargoPathCycle, &leg.id);
            }
        }
        for pair in legs.windows(2) {
            let previous = pair[0];
            let next = pair[1];
            if previous.to_node_id != next.from_node_id {
                push_finding(
                    &mut findings,
                    FindingCode::DiscontinuousCargoPath,
                    &next.id,
                );
            }
            if previous.ends_at_epoch_seconds > next.starts_at_epoch_seconds {
                push_finding(&mut findings, FindingCode::OverlappingCargoHops, &next.id);
            }
        }
        if let Some(last) = legs.last() {
            if last.to_node_id != campaign.orbit_service.destination_node_id {
                push_finding(
                    &mut findings,
                    FindingCode::WrongOrbitDestination,
                    &last.id,
                );
            }
            if last.ends_at_epoch_seconds > campaign.required_delivery_by_epoch_seconds {
                push_finding(&mut findings, FindingCode::LateDelivery, &last.id);
            }
        }
    }

    // Every fault-domain reference must name a dependency actually used in this
    // campaign. This catches misspelled controls rather than reporting fake resilience.
    let mut fault_scenario_ids = BTreeSet::new();
    for scenario in &campaign.fault_scenarios {
        if !valid_id(&scenario.id) {
            push_finding(&mut findings, FindingCode::InvalidIdentifier, &scenario.id);
        }
        if !fault_scenario_ids.insert(scenario.id.clone()) {
            push_finding(
                &mut findings,
                FindingCode::DuplicateFaultScenario,
                &scenario.id,
            );
        }
        let mut scenario_dependencies = BTreeSet::new();
        for dependency in &scenario.failed_dependency_ids {
            if !valid_id(dependency) {
                push_finding(&mut findings, FindingCode::InvalidIdentifier, dependency);
            }
            if !scenario_dependencies.insert(dependency.clone()) {
                push_finding(&mut findings, FindingCode::DuplicateDependency, dependency);
            }
            if !known_dependencies.contains(dependency) {
                push_finding(
                    &mut findings,
                    FindingCode::UnknownFailureDomain,
                    dependency,
                );
            }
        }
    }

    match node_by_id.get(&campaign.orbit_service.destination_node_id) {
        Some(node) if node.role == NodeRole::OrbitDestination => {}
        _ => push_finding(
            &mut findings,
            FindingCode::WrongOrbitDestination,
            &campaign.orbit_service.destination_node_id,
        ),
    }
    validate_evidence(
        &campaign.orbit_service.compatibility,
        &campaign.orbit_service.target_orbit_id,
        &mut findings,
    );

    let total_cargo_mass_grams = campaign
        .cargo_lots
        .iter()
        .try_fold(0_u64, |sum, cargo| sum.checked_add(cargo.mass_grams));
    if total_cargo_mass_grams.is_none() {
        push_finding(
            &mut findings,
            FindingCode::ArithmeticOverflow,
            &campaign.id,
        );
    }
    let total_cargo_mass_grams = total_cargo_mass_grams.unwrap_or(0);
    findings.sort();
    findings.dedup();

    let disposition = if findings.iter().any(|finding| !is_evidence_finding(finding.code)) {
        CampaignDisposition::Infeasible
    } else if !findings.is_empty() {
        CampaignDisposition::NeedsEvidence
    } else {
        CampaignDisposition::Ready
    };

    let accepted_delivered_cargo_mass_grams = if disposition == CampaignDisposition::Ready {
        total_cargo_mass_grams
    } else {
        0
    };
    let fault_outcomes = if disposition == CampaignDisposition::Ready {
        evaluate_faults(campaign)
    } else {
        Vec::new()
    };

    CampaignReportV1 {
        schema_version: SCHEMA_VERSION,
        disposition,
        total_cargo_mass_grams,
        accepted_delivered_cargo_mass_grams,
        findings,
        fault_outcomes,
    }
}

fn evaluate_faults(campaign: &CampaignV1) -> Vec<FaultOutcomeV1> {
    let mut results = Vec::new();
    for scenario in &campaign.fault_scenarios {
        let failed: BTreeSet<&str> = scenario
            .failed_dependency_ids
            .iter()
            .map(String::as_str)
            .collect();
        let mut failed_transfer_ids = Vec::new();
        let mut delivered_mass_grams = 0_u64;
        let mut unmet_mass_grams = 0_u64;

        for cargo in &campaign.cargo_lots {
            let mut cargo_failed = false;
            for transfer in campaign
                .transfers
                .iter()
                .filter(|transfer| transfer.cargo_lot_id == cargo.id)
            {
                if transfer
                    .dependency_ids
                    .iter()
                    .any(|dependency| failed.contains(dependency.as_str()))
                {
                    failed_transfer_ids.push(transfer.id.clone());
                    cargo_failed = true;
                }
            }
            if cargo_failed {
                unmet_mass_grams = unmet_mass_grams.saturating_add(cargo.mass_grams);
            } else {
                delivered_mass_grams = delivered_mass_grams.saturating_add(cargo.mass_grams);
            }
        }
        failed_transfer_ids.sort();
        failed_transfer_ids.dedup();
        results.push(FaultOutcomeV1 {
            scenario_id: scenario.id.clone(),
            delivered_mass_grams,
            unmet_mass_grams,
            failed_transfer_ids,
        });
    }
    results.sort_by_key(|result| result.scenario_id.clone());
    results
}

fn validate_resource_status(
    status: &ResourceQualificationStatus,
    subject_id: &str,
    findings: &mut Vec<Finding>,
) {
    match status {
        ResourceQualificationStatus::QualifiedForPlanning { receipt_sha256 } => {
            if !is_sha256(receipt_sha256) {
                push_finding(
                    findings,
                    FindingCode::MalformedReceiptDigest,
                    subject_id,
                );
            }
        }
        ResourceQualificationStatus::Rejected { .. } => {
            push_finding(
                findings,
                FindingCode::RejectedResourceQualification,
                subject_id,
            );
        }
        ResourceQualificationStatus::Unknown => {
            push_finding(
                findings,
                FindingCode::UnknownResourceQualification,
                subject_id,
            );
        }
        ResourceQualificationStatus::Unsupported { .. } => {
            push_finding(
                findings,
                FindingCode::UnsupportedResourceQualification,
                subject_id,
            );
        }
    }
}

fn validate_evidence(evidence: &EvidenceStatus, subject_id: &str, findings: &mut Vec<Finding>) {
    match evidence {
        EvidenceStatus::Qualified { receipt_sha256 } => {
            if !is_sha256(receipt_sha256) {
                push_finding(
                    findings,
                    FindingCode::MalformedReceiptDigest,
                    subject_id,
                );
            }
        }
        EvidenceStatus::Unknown => {
            push_finding(findings, FindingCode::EvidenceUnknown, subject_id);
        }
        EvidenceStatus::Unsupported { .. } => {
            push_finding(findings, FindingCode::EvidenceUnsupported, subject_id);
        }
    }
}

fn is_evidence_finding(code: FindingCode) -> bool {
    matches!(
        code,
        FindingCode::MissingResourceQualification
            | FindingCode::UnknownResourceQualification
            | FindingCode::UnsupportedResourceQualification
            | FindingCode::MalformedReceiptDigest
            | FindingCode::EvidenceUnknown
            | FindingCode::EvidenceUnsupported
    )
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn push_finding(findings: &mut Vec<Finding>, code: FindingCode, subject_id: &str) {
    findings.push(Finding {
        code,
        subject_id: subject_id.to_owned(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qualified() -> EvidenceStatus {
        EvidenceStatus::Qualified {
            receipt_sha256: "a".repeat(64),
        }
    }

    fn resource_qualified() -> ResourceQualificationStatus {
        ResourceQualificationStatus::QualifiedForPlanning {
            receipt_sha256: "b".repeat(64),
        }
    }

    fn baseline() -> CampaignV1 {
        CampaignV1 {
            id: "campaign-1".to_owned(),
            cargo_lots: vec![CargoLotV1 {
                id: "payload-1".to_owned(),
                mass_grams: 1_000,
            }],
            nodes: vec![
                ChainNodeV1 {
                    id: "factory".to_owned(),
                    role: NodeRole::Supplier,
                },
                ChainNodeV1 {
                    id: "port".to_owned(),
                    role: NodeRole::PortOrAirportGateway,
                },
                ChainNodeV1 {
                    id: "range".to_owned(),
                    role: NodeRole::LaunchRange,
                },
                ChainNodeV1 {
                    id: "orbit".to_owned(),
                    role: NodeRole::OrbitDestination,
                },
            ],
            resource_qualifications: vec![
                ResourceQualificationV1 {
                    id: "rail-qualification".to_owned(),
                    resource_id: "rail-section-1".to_owned(),
                    valid_from_epoch_seconds: 0,
                    valid_until_epoch_seconds: 100,
                    status: resource_qualified(),
                },
                ResourceQualificationV1 {
                    id: "road-qualification".to_owned(),
                    resource_id: "road-1".to_owned(),
                    valid_from_epoch_seconds: 0,
                    valid_until_epoch_seconds: 100,
                    status: resource_qualified(),
                },
                ResourceQualificationV1 {
                    id: "launch-qualification".to_owned(),
                    resource_id: "range-slot-1".to_owned(),
                    valid_from_epoch_seconds: 0,
                    valid_until_epoch_seconds: 100,
                    status: resource_qualified(),
                },
            ],
            transfers: vec![
                TransferV1 {
                    id: "transfer-rail".to_owned(),
                    cargo_lot_id: "payload-1".to_owned(),
                    from_node_id: "factory".to_owned(),
                    to_node_id: "port".to_owned(),
                    resource_id: "rail-section-1".to_owned(),
                    resource_qualification_id: "rail-qualification".to_owned(),
                    mass_grams: 1_000,
                    starts_at_epoch_seconds: 10,
                    ends_at_epoch_seconds: 20,
                    dependency_ids: vec!["rail-power".to_owned()],
                },
                TransferV1 {
                    id: "transfer-road".to_owned(),
                    cargo_lot_id: "payload-1".to_owned(),
                    from_node_id: "port".to_owned(),
                    to_node_id: "range".to_owned(),
                    resource_id: "road-1".to_owned(),
                    resource_qualification_id: "road-qualification".to_owned(),
                    mass_grams: 1_000,
                    starts_at_epoch_seconds: 25,
                    ends_at_epoch_seconds: 35,
                    dependency_ids: vec!["access-road".to_owned()],
                },
                TransferV1 {
                    id: "transfer-launch".to_owned(),
                    cargo_lot_id: "payload-1".to_owned(),
                    from_node_id: "range".to_owned(),
                    to_node_id: "orbit".to_owned(),
                    resource_id: "range-slot-1".to_owned(),
                    resource_qualification_id: "launch-qualification".to_owned(),
                    mass_grams: 1_000,
                    starts_at_epoch_seconds: 40,
                    ends_at_epoch_seconds: 50,
                    dependency_ids: vec!["range-tracking".to_owned()],
                },
            ],
            orbit_service: OrbitServiceV1 {
                target_orbit_id: "leo-550km".to_owned(),
                destination_node_id: "orbit".to_owned(),
                compatibility: qualified(),
            },
            required_delivery_by_epoch_seconds: 80,
            fault_scenarios: vec![FaultScenarioV1 {
                id: "rail-power-loss".to_owned(),
                failed_dependency_ids: vec!["rail-power".to_owned()],
            }],
        }
    }

    fn has_finding(report: &CampaignReportV1, code: FindingCode) -> bool {
        report.findings.iter().any(|finding| finding.code == code)
    }

    #[test]
    fn valid_chain_reconciles_cargo_mass_and_destination() {
        let report = evaluate_campaign(&baseline());
        assert_eq!(report.disposition, CampaignDisposition::Ready);
        assert_eq!(report.total_cargo_mass_grams, 1_000);
        assert_eq!(report.accepted_delivered_cargo_mass_grams, 1_000);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn unknown_resource_qualification_fails_closed() {
        let mut campaign = baseline();
        campaign.resource_qualifications[0].status = ResourceQualificationStatus::Unknown;
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::NeedsEvidence);
        assert_eq!(report.accepted_delivered_cargo_mass_grams, 0);
        assert!(has_finding(
            &report,
            FindingCode::UnknownResourceQualification
        ));
    }

    #[test]
    fn rejected_resource_qualification_makes_the_chain_infeasible() {
        let mut campaign = baseline();
        campaign.resource_qualifications[0].status =
            ResourceQualificationStatus::Rejected {
                reason: "capacity owner rejected the proposed plan".to_owned(),
            };
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::Infeasible);
        assert!(has_finding(
            &report,
            FindingCode::RejectedResourceQualification
        ));
    }

    #[test]
    fn resource_qualification_must_match_resource_and_cover_operation_window() {
        let mut campaign = baseline();
        campaign.resource_qualifications[0].resource_id = "different-rail".to_owned();
        campaign.resource_qualifications[0].valid_until_epoch_seconds = 15;
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::Infeasible);
        assert!(has_finding(
            &report,
            FindingCode::ResourceQualificationResourceMismatch
        ));
        assert!(has_finding(
            &report,
            FindingCode::ResourceQualificationDoesNotCoverTransfer
        ));
    }

    #[test]
    fn payload_mass_must_be_conserved_across_every_hop() {
        let mut campaign = baseline();
        campaign.transfers[1].mass_grams = 999;
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::Infeasible);
        assert!(has_finding(&report, FindingCode::TransferMassMismatch));
    }

    #[test]
    fn every_cargo_path_must_be_contiguous_and_temporally_ordered() {
        let mut campaign = baseline();
        campaign.transfers[1].from_node_id = "factory".to_owned();
        campaign.transfers[1].starts_at_epoch_seconds = 19;
        let report = evaluate_campaign(&campaign);
        assert!(has_finding(&report, FindingCode::DiscontinuousCargoPath));
        assert!(has_finding(&report, FindingCode::OverlappingCargoHops));
    }

    #[test]
    fn orbit_compatibility_must_be_qualified_by_the_specialist_owner() {
        let mut campaign = baseline();
        campaign.orbit_service.compatibility = EvidenceStatus::Unknown;
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::NeedsEvidence);
        assert!(has_finding(&report, FindingCode::EvidenceUnknown));
    }

    #[test]
    fn deterministic_fault_scenarios_report_lost_service_without_probabilities() {
        let report = evaluate_campaign(&baseline());
        assert_eq!(report.fault_outcomes.len(), 1);
        assert_eq!(report.fault_outcomes[0].delivered_mass_grams, 0);
        assert_eq!(report.fault_outcomes[0].unmet_mass_grams, 1_000);
        assert_eq!(
            report.fault_outcomes[0].failed_transfer_ids,
            vec!["transfer-rail".to_owned()]
        );
    }

    #[test]
    fn malformed_resource_orbit_receipt_cannot_be_promoted_to_ready() {
        let mut campaign = baseline();
        campaign.orbit_service.compatibility = EvidenceStatus::Qualified {
            receipt_sha256: "not-a-digest".to_owned(),
        };
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::NeedsEvidence);
        assert!(has_finding(
            &report,
            FindingCode::MalformedReceiptDigest
        ));
    }

    #[test]
    fn missing_resource_qualification_is_not_unlimited_capacity() {
        let mut campaign = baseline();
        campaign.transfers[0].resource_qualification_id = "missing-resource-proof".to_owned();
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::NeedsEvidence);
        assert!(has_finding(
            &report,
            FindingCode::MissingResourceQualification
        ));
    }

    #[test]
    fn orbit_destination_must_match_the_declared_mission_endpoint() {
        let mut campaign = baseline();
        campaign.orbit_service.destination_node_id = "range".to_owned();
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::Infeasible);
        assert!(has_finding(&report, FindingCode::WrongOrbitDestination));
    }

    #[test]
    fn late_delivery_is_not_reported_as_success() {
        let mut campaign = baseline();
        campaign.required_delivery_by_epoch_seconds = 45;
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::Infeasible);
        assert!(has_finding(&report, FindingCode::LateDelivery));
    }

    #[test]
    fn cargo_path_cycles_are_rejected() {
        let mut campaign = baseline();
        campaign.nodes.push(ChainNodeV1 {
            id: "detour".to_owned(),
            role: NodeRole::Other,
        });
        campaign.transfers[1].to_node_id = "detour".to_owned();
        campaign.transfers[2].from_node_id = "detour".to_owned();
        campaign.transfers[2].to_node_id = "factory".to_owned();
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::Infeasible);
        assert!(has_finding(&report, FindingCode::CargoPathCycle));
    }
}
