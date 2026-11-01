// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Effects-disabled Earth-to-orbit logistics chain admission and accounting.
//!
//! This crate deliberately does not optimize routes or solve transport / orbital
//! dynamics. It validates a supplied, ordered payload chain against declared
//! capacity windows and opaque specialist qualification receipts. A qualified
//! receipt digest is only an identity commitment here; callers must independently
//! verify the receipt, issuer, policy and model context before relying on it.
//!
//! The first increment is intentionally conservative:
//! - payload mass remains invariant across each transport hop;
//! - capacity is checked as aggregate concurrent mass on a physical resource;
//! - operation intervals are half-open [start, end);
//! - unknown capacity or orbit compatibility never becomes a pass;
//! - fault scenarios are deterministic sets, not invented probabilities;
//! - a passing plan is not a booking, permit, launch authorization or safety case.

use std::collections::{BTreeMap, BTreeSet};

/// Schema version for the first Earth-to-orbit chain contract.
pub const SCHEMA_VERSION: &str = "symtropy.earth-orbit-chain.v1";

/// Evidence state for a claim supplied by a domain specialist.
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

/// A source-qualified concurrency envelope for one physical resource.
///
/// The interval is half-open [valid_from, valid_until). A zero capacity is a
/// real zero, while missing capacity is represented by the absence of a window
/// and therefore produces a fail-closed finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityWindowV1 {
    pub id: String,
    pub resource_id: String,
    pub valid_from_epoch_seconds: u64,
    pub valid_until_epoch_seconds: u64,
    pub max_concurrent_mass_grams: u64,
    pub evidence: EvidenceStatus,
}

/// One cargo lot's occupation of one physical resource between two chain nodes.
///
/// start/end describe the whole resource occupancy interval for this v1
/// accounting contract. More detailed timetabling, dwell and service calendars
/// remain owned by the transport / launch specialist models.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferV1 {
    pub id: String,
    pub cargo_lot_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub resource_id: String,
    pub capacity_window_id: String,
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
    pub capacity_windows: Vec<CapacityWindowV1>,
    /// Transfers are grouped by cargo lot and ordered by their time intervals.
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
    UnknownCapacityWindow,
    UnknownFailureDomain,
    SelfTransfer,
    InvalidTimeInterval,
    CapacityWindowResourceMismatch,
    CapacityWindowDoesNotCoverTransfer,
    OverlappingCapacityWindows,
    TransferMassMismatch,
    DiscontinuousCargoPath,
    OverlappingCargoHops,
    CargoPathCycle,
    WrongOrbitDestination,
    LateDelivery,
    CapacityExceeded,
    ArithmeticOverflow,
    MalformedReceiptDigest,
    EvidenceUnknown,
    EvidenceUnsupported,
    DuplicateDependency,
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
    let mut window_by_id = BTreeMap::new();
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

    for window in &campaign.capacity_windows {
        if !valid_id(&window.id) || !valid_id(&window.resource_id) {
            push_finding(&mut findings, FindingCode::InvalidIdentifier, &window.id);
        }
        if window.valid_from_epoch_seconds >= window.valid_until_epoch_seconds {
            push_finding(&mut findings, FindingCode::InvalidTimeInterval, &window.id);
        }
        validate_evidence(&window.evidence, &window.id, &mut findings);
        if window_by_id.insert(window.id.clone(), window).is_some() {
            push_finding(&mut findings, FindingCode::DuplicateIdentifier, &window.id);
        }
    }

    // Two overlapping capacity snapshots for one physical asset are ambiguous:
    // choosing the larger or newer-looking value would silently arbitrate evidence.
    for left_index in 0..campaign.capacity_windows.len() {
        for right_index in (left_index + 1)..campaign.capacity_windows.len() {
            let left = &campaign.capacity_windows[left_index];
            let right = &campaign.capacity_windows[right_index];
            if left.resource_id == right.resource_id
                && intervals_overlap(
                    left.valid_from_epoch_seconds,
                    left.valid_until_epoch_seconds,
                    right.valid_from_epoch_seconds,
                    right.valid_until_epoch_seconds,
                )
            {
                push_finding(
                    &mut findings,
                    FindingCode::OverlappingCapacityWindows,
                    &left.resource_id,
                );
            }
        }
    }

    let mut transfers_by_cargo: BTreeMap<String, Vec<&TransferV1>> = BTreeMap::new();
    let mut events_by_window: BTreeMap<String, Vec<(u64, bool, u64, String)>> = BTreeMap::new();

    for transfer in &campaign.transfers {
        if !valid_id(&transfer.id)
            || !valid_id(&transfer.cargo_lot_id)
            || !valid_id(&transfer.from_node_id)
            || !valid_id(&transfer.to_node_id)
            || !valid_id(&transfer.resource_id)
            || !valid_id(&transfer.capacity_window_id)
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

        match window_by_id.get(&transfer.capacity_window_id) {
            None => push_finding(
                &mut findings,
                FindingCode::UnknownCapacityWindow,
                &transfer.capacity_window_id,
            ),
            Some(window) => {
                if window.resource_id != transfer.resource_id {
                    push_finding(
                        &mut findings,
                        FindingCode::CapacityWindowResourceMismatch,
                        &transfer.id,
                    );
                }
                if transfer.starts_at_epoch_seconds < window.valid_from_epoch_seconds
                    || transfer.ends_at_epoch_seconds > window.valid_until_epoch_seconds
                {
                    push_finding(
                        &mut findings,
                        FindingCode::CapacityWindowDoesNotCoverTransfer,
                        &transfer.id,
                    );
                }
                if matches!(window.evidence, EvidenceStatus::Qualified { .. })
                    && transfer.starts_at_epoch_seconds < transfer.ends_at_epoch_seconds
                    && window.resource_id == transfer.resource_id
                    && transfer.starts_at_epoch_seconds >= window.valid_from_epoch_seconds
                    && transfer.ends_at_epoch_seconds <= window.valid_until_epoch_seconds
                {
                    let events = events_by_window
                        .entry(transfer.capacity_window_id.clone())
                        .or_default();
                    // false=end, true=start. At equal timestamps, end events run first
                    // to preserve half-open interval semantics and permit adjacency.
                    events.push((
                        transfer.ends_at_epoch_seconds,
                        false,
                        transfer.mass_grams,
                        transfer.id.clone(),
                    ));
                    events.push((
                        transfer.starts_at_epoch_seconds,
                        true,
                        transfer.mass_grams,
                        transfer.id.clone(),
                    ));
                }
            }
        }
    }

    // Check cargo continuity in time order; an input vector's insertion order does
    // not affect the result. A v1 cargo lot follows one linear route, not a split flow.
    for cargo in &campaign.cargo_lots {
        let mut legs = transfers_by_cargo.remove(&cargo.id).unwrap_or_default();
        if legs.is_empty() {
            push_finding(&mut findings, FindingCode::EmptyChain, &cargo.id);
            continue;
        }
        legs.sort_by(|left, right| {
            (
                left.starts_at_epoch_seconds,
                left.ends_at_epoch_seconds,
                left.id.as_str(),
            )
                .cmp(&(
                    right.starts_at_epoch_seconds,
                    right.ends_at_epoch_seconds,
                    right.id.as_str(),
                ))
        });
        let mut visited_nodes = BTreeSet::new();
        if let Some(first) = legs.first() {
            visited_nodes.insert(first.from_node_id.clone());
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
            if !visited_nodes.insert(previous.to_node_id.clone()) {
                push_finding(&mut findings, FindingCode::CargoPathCycle, &previous.id);
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
    // campaign. This catches misspelled controls instead of reporting fake resilience.
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

    // Aggregate simultaneous mass by exact physical resource capacity window.
    for (window_id, events) in &mut events_by_window {
        events.sort_by(|left, right| (left.0, left.1, left.3.as_str()).cmp(&(
            right.0,
            right.1,
            right.3.as_str(),
        )));
        let Some(window) = window_by_id.get(window_id) else {
            continue;
        };
        let mut concurrent_mass = 0_u64;
        for (_, is_start, mass_grams, transfer_id) in events {
            let next = if *is_start {
                concurrent_mass.checked_add(*mass_grams)
            } else {
                concurrent_mass.checked_sub(*mass_grams)
            };
            match next {
                Some(value) => concurrent_mass = value,
                None => push_finding(
                    &mut findings,
                    FindingCode::ArithmeticOverflow,
                    transfer_id,
                ),
            }
            if concurrent_mass > window.max_concurrent_mass_grams {
                push_finding(
                    &mut findings,
                    FindingCode::CapacityExceeded,
                    window_id,
                );
            }
        }
    }

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
    let final_disposition = if findings.iter().any(|finding| !is_evidence_finding(finding.code)) {
        CampaignDisposition::Infeasible
    } else if !findings.is_empty() {
        CampaignDisposition::NeedsEvidence
    } else {
        CampaignDisposition::Ready
    };

    let accepted_delivered_cargo_mass_grams = if final_disposition == CampaignDisposition::Ready {
        total_cargo_mass_grams
    } else {
        0
    };

    let fault_outcomes = if final_disposition == CampaignDisposition::Ready {
        evaluate_faults(campaign)
    } else {
        Vec::new()
    };

    CampaignReportV1 {
        schema_version: SCHEMA_VERSION,
        disposition: final_disposition,
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
            let cargo_failed = campaign
                .transfers
                .iter()
                .filter(|transfer| transfer.cargo_lot_id == cargo.id)
                .any(|transfer| {
                    transfer
                        .dependency_ids
                        .iter()
                        .any(|dependency| failed.contains(dependency.as_str()))
                        && {
                            failed_transfer_ids.push(transfer.id.clone());
                            true
                        }
                });
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
    results.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    results
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
        FindingCode::MalformedReceiptDigest
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

fn intervals_overlap(left_start: u64, left_end: u64, right_start: u64, right_end: u64) -> bool {
    left_start < right_end && right_start < left_end
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
            capacity_windows: vec![
                CapacityWindowV1 {
                    id: "rail-window".to_owned(),
                    resource_id: "rail-section-1".to_owned(),
                    valid_from_epoch_seconds: 0,
                    valid_until_epoch_seconds: 100,
                    max_concurrent_mass_grams: 2_000,
                    evidence: qualified(),
                },
                CapacityWindowV1 {
                    id: "road-window".to_owned(),
                    resource_id: "road-1".to_owned(),
                    valid_from_epoch_seconds: 0,
                    valid_until_epoch_seconds: 100,
                    max_concurrent_mass_grams: 2_000,
                    evidence: qualified(),
                },
                CapacityWindowV1 {
                    id: "launch-window".to_owned(),
                    resource_id: "range-slot-1".to_owned(),
                    valid_from_epoch_seconds: 0,
                    valid_until_epoch_seconds: 100,
                    max_concurrent_mass_grams: 2_000,
                    evidence: qualified(),
                },
            ],
            transfers: vec![
                TransferV1 {
                    id: "transfer-rail".to_owned(),
                    cargo_lot_id: "payload-1".to_owned(),
                    from_node_id: "factory".to_owned(),
                    to_node_id: "port".to_owned(),
                    resource_id: "rail-section-1".to_owned(),
                    capacity_window_id: "rail-window".to_owned(),
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
                    capacity_window_id: "road-window".to_owned(),
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
                    capacity_window_id: "launch-window".to_owned(),
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
    fn valid_chain_reconciles_payload_mass_and_destination() {
        let report = evaluate_campaign(&baseline());
        assert_eq!(report.disposition, CampaignDisposition::Ready);
        assert_eq!(report.total_cargo_mass_grams, 1_000);
        assert_eq!(report.accepted_delivered_cargo_mass_grams, 1_000);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn overlapping_cargo_occupancy_is_aggregated_per_physical_resource() {
        let mut campaign = baseline();
        campaign.cargo_lots.push(CargoLotV1 {
            id: "payload-2".to_owned(),
            mass_grams: 1_500,
        });
        campaign.transfers.push(TransferV1 {
            id: "second-rail-transfer".to_owned(),
            cargo_lot_id: "payload-2".to_owned(),
            from_node_id: "factory".to_owned(),
            to_node_id: "port".to_owned(),
            resource_id: "rail-section-1".to_owned(),
            capacity_window_id: "rail-window".to_owned(),
            mass_grams: 1_500,
            starts_at_epoch_seconds: 15,
            ends_at_epoch_seconds: 18,
            dependency_ids: vec!["rail-power".to_owned()],
        });
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::Infeasible);
        assert!(has_finding(&report, FindingCode::CapacityExceeded));
    }

    #[test]
    fn half_open_adjacent_transfers_can_reuse_capacity() {
        let mut campaign = baseline();
        campaign.cargo_lots.push(CargoLotV1 {
            id: "payload-2".to_owned(),
            mass_grams: 2_000,
        });
        campaign.transfers.extend([
            TransferV1 {
                id: "second-rail-transfer".to_owned(),
                cargo_lot_id: "payload-2".to_owned(),
                from_node_id: "factory".to_owned(),
                to_node_id: "port".to_owned(),
                resource_id: "rail-section-1".to_owned(),
                capacity_window_id: "rail-window".to_owned(),
                mass_grams: 2_000,
                starts_at_epoch_seconds: 20,
                ends_at_epoch_seconds: 30,
                dependency_ids: vec!["rail-power".to_owned()],
            },
            TransferV1 {
                id: "second-road-transfer".to_owned(),
                cargo_lot_id: "payload-2".to_owned(),
                from_node_id: "port".to_owned(),
                to_node_id: "range".to_owned(),
                resource_id: "road-1".to_owned(),
                capacity_window_id: "road-window".to_owned(),
                mass_grams: 2_000,
                starts_at_epoch_seconds: 35,
                ends_at_epoch_seconds: 45,
                dependency_ids: vec!["access-road".to_owned()],
            },
            TransferV1 {
                id: "second-launch-transfer".to_owned(),
                cargo_lot_id: "payload-2".to_owned(),
                from_node_id: "range".to_owned(),
                to_node_id: "orbit".to_owned(),
                resource_id: "range-slot-1".to_owned(),
                capacity_window_id: "launch-window".to_owned(),
                mass_grams: 2_000,
                starts_at_epoch_seconds: 50,
                ends_at_epoch_seconds: 60,
                dependency_ids: vec!["range-tracking".to_owned()],
            },
        ]);
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::Ready);
        assert_eq!(report.total_cargo_mass_grams, 3_000);
    }

    #[test]
    fn unknown_capacity_fails_closed_as_needs_evidence() {
        let mut campaign = baseline();
        campaign.capacity_windows[0].evidence = EvidenceStatus::Unknown;
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, CampaignDisposition::NeedsEvidence);
        assert_eq!(report.accepted_delivered_cargo_mass_grams, 0);
        assert!(has_finding(&report, FindingCode::EvidenceUnknown));
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
    fn unsupported_or_malformed_orbit_receipts_cannot_be_promoted_to_ready() {
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
    fn overlapping_capacity_snapshots_for_one_resource_are_rejected() {
        let mut campaign = baseline();
        campaign.capacity_windows.push(CapacityWindowV1 {
            id: "rail-window-conflict".to_owned(),
            resource_id: "rail-section-1".to_owned(),
            valid_from_epoch_seconds: 50,
            valid_until_epoch_seconds: 150,
            max_concurrent_mass_grams: 9_000,
            evidence: qualified(),
        });
        let report = evaluate_campaign(&campaign);
        assert!(has_finding(
            &report,
            FindingCode::OverlappingCapacityWindows
        ));
    }

    #[test]
    fn launch_compatibility_must_not_be_assumed_for_a_different_destination() {
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
}
