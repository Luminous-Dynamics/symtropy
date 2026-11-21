// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic, read-only carrier/UAV sortie co-availability screen.
//!
//! This evaluator checks declared capability and timing envelopes. It is not
//! a route optimizer, flight controller, airspace authority, battery model,
//! road-vehicle dynamics model, or operational dispatch authorization.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const COAVAILABILITY_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    FreshInDomain,
    Unknown,
    Stale,
    Conflicted,
    Revoked,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AirspaceStatus {
    Approved,
    Prohibited,
    Unknown,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeasibilityVerdict {
    Feasible,
    Infeasible,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarrierStop {
    pub stop_id: String,
    pub node_id: String,
    /// Earliest possible carrier arrival, in scenario-relative ms.
    pub arrival_earliest_ms: u64,
    /// Latest possible carrier arrival; carrier presence is not guaranteed before this.
    pub arrival_latest_ms: u64,
    /// Earliest possible carrier departure; carrier presence is not guaranteed after this.
    pub departure_earliest_ms: u64,
    /// Latest possible carrier departure, in scenario-relative ms.
    pub departure_latest_ms: u64,
    /// Pad resources physically available at this exact route stop.
    pub available_pad_resource_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarrierEnvelope {
    pub carrier_id: String,
    pub route_timing_evidence: EvidenceStatus,
    pub site_resource_evidence: EvidenceStatus,
    pub route_stops: Vec<CarrierStop>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AircraftEnvelope {
    pub aircraft_id: String,
    pub capability_evidence: EvidenceStatus,
    pub maximum_payload_mass_g: u64,
    /// Usable battery energy at the stated operating conditions, Wh.
    pub usable_energy_wh: u64,
    /// Required protected reserve, Wh, supplied by the aircraft owner.
    pub reserve_energy_wh: u64,
    pub maximum_peak_power_w: u64,
    pub maximum_continuous_power_w: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SortieProposal {
    pub sortie_id: String,
    pub payload_mass_g: u64,
    pub launch_stop_id: String,
    pub recovery_stop_id: String,
    pub launch_pad_resource_id: String,
    pub recovery_pad_resource_id: String,
    /// Half-open launch-service start window; launch service must finish by close.
    pub launch_window_open_ms: u64,
    pub launch_window_close_ms: u64,
    /// Half-open recovery-service start window; recovery service must finish by close.
    pub recovery_window_open_ms: u64,
    pub recovery_window_close_ms: u64,
    pub launch_service_ms: u64,
    pub recovery_service_ms: u64,
    pub flight_duration_ms: u64,
    /// Battery-terminal mission energy, excluding the protected reserve.
    pub flight_energy_wh: u64,
    pub peak_power_w: u64,
    pub continuous_power_w: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatingDomainEnvelope {
    pub permission: AirspaceStatus,
    pub permission_evidence: EvidenceStatus,
    pub weather_evidence: EvidenceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceReservation {
    pub reservation_id: String,
    pub resource_id: String,
    /// Half-open occupied interval [start_ms, end_ms).
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoAvailabilityInput {
    pub schema_version: u16,
    pub scenario_id: String,
    pub carrier: CarrierEnvelope,
    pub aircraft: AircraftEnvelope,
    pub sortie: SortieProposal,
    pub operating_domain: OperatingDomainEnvelope,
    pub existing_reservations: Vec<ResourceReservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoAvailabilityReport {
    pub schema_version: u16,
    /// SHA-256 of normalized typed input; integrity binding, not source authentication.
    pub input_digest_sha256: String,
    pub scenario_id: String,
    pub sortie_id: String,
    pub verdict: FeasibilityVerdict,
    /// Start of the carrier-side launch service interval.
    pub selected_launch_time_ms: Option<u64>,
    /// Start of the carrier-side recovery service interval after launch service + flight.
    pub selected_recovery_time_ms: Option<u64>,
    pub payload_margin_g: Option<u64>,
    pub energy_margin_wh: Option<u64>,
    pub peak_power_margin_w: Option<u64>,
    pub continuous_power_margin_w: Option<u64>,
    pub findings: Vec<String>,
    pub energy_boundary: String,
    pub dispatch_authorized: bool,
    pub physical_command_emitted: bool,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Copy)]
struct StopWindow {
    index: usize,
    arrival_latest_ms: u64,
    departure_earliest_ms: u64,
}

fn structural_errors(input: &CoAvailabilityInput) -> Vec<String> {
    let mut errors = Vec::new();
    if input.schema_version != COAVAILABILITY_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version: expected {}, got {}",
            COAVAILABILITY_SCHEMA_VERSION, input.schema_version
        ));
    }
    if input.scenario_id.trim().is_empty()
        || input.carrier.carrier_id.trim().is_empty()
        || input.aircraft.aircraft_id.trim().is_empty()
        || input.sortie.sortie_id.trim().is_empty()
        || input.sortie.launch_stop_id.trim().is_empty()
        || input.sortie.recovery_stop_id.trim().is_empty()
        || input.sortie.launch_pad_resource_id.trim().is_empty()
        || input.sortie.recovery_pad_resource_id.trim().is_empty()
    {
        errors.push(
            "scenario, asset, sortie, node and resource identifiers must be non-empty".into(),
        );
    }
    if input.carrier.route_stops.is_empty() {
        errors.push("carrier.route_stops: must not be empty".into());
    }
    let mut stop_ids = BTreeSet::new();
    for (index, stop) in input.carrier.route_stops.iter().enumerate() {
        if stop.stop_id.trim().is_empty() || stop.node_id.trim().is_empty() {
            errors.push(format!(
                "carrier.route_stops[{index}]: stop and node identifiers must be non-empty"
            ));
        }
        if !stop_ids.insert(stop.stop_id.as_str()) {
            errors.push(format!(
                "carrier.route_stops[{index}].stop_id: duplicate route-stop identity"
            ));
        }
        if !(stop.arrival_earliest_ms <= stop.arrival_latest_ms
            && stop.arrival_latest_ms < stop.departure_earliest_ms
            && stop.departure_earliest_ms <= stop.departure_latest_ms)
        {
            errors.push(format!(
                "carrier.route_stops[{index}]: require arrival_early <= arrival_late < depart_early <= depart_late"
            ));
        }
        let mut pad_ids = BTreeSet::new();
        for (pad_index, pad_id) in stop.available_pad_resource_ids.iter().enumerate() {
            if pad_id.trim().is_empty() || !pad_ids.insert(pad_id.as_str()) {
                errors.push(format!(
                    "carrier.route_stops[{index}].available_pad_resource_ids[{pad_index}]: empty or duplicate pad identity"
                ));
            }
        }
    }
    for (index, pair) in input.carrier.route_stops.windows(2).enumerate() {
        if pair[0].departure_latest_ms > pair[1].arrival_earliest_ms {
            errors.push(format!(
                "carrier.route_stops[{index}..{}]: route stop time envelopes overlap or reverse",
                index + 1
            ));
        }
    }
    let sortie = &input.sortie;
    if sortie.launch_window_open_ms >= sortie.launch_window_close_ms {
        errors.push("sortie.launch_window: must be a non-empty half-open interval".into());
    }
    if sortie.recovery_window_open_ms >= sortie.recovery_window_close_ms {
        errors.push("sortie.recovery_window: must be a non-empty half-open interval".into());
    }
    if sortie.launch_service_ms == 0
        || sortie.recovery_service_ms == 0
        || sortie.flight_duration_ms == 0
    {
        errors.push("sortie service and flight durations must be non-zero".into());
    }
    if sortie.payload_mass_g == 0
        || sortie.flight_energy_wh == 0
        || sortie.peak_power_w == 0
        || sortie.continuous_power_w == 0
    {
        errors.push("sortie payload, energy and power requirements must be non-zero".into());
    }
    if sortie.continuous_power_w > sortie.peak_power_w {
        errors.push("sortie.continuous_power_w: cannot exceed peak power".into());
    }
    if input.aircraft.maximum_payload_mass_g == 0
        || input.aircraft.usable_energy_wh == 0
        || input.aircraft.maximum_peak_power_w == 0
        || input.aircraft.maximum_continuous_power_w == 0
    {
        errors.push("aircraft capability limits must be non-zero".into());
    }
    if input.aircraft.reserve_energy_wh == 0 {
        errors.push(
            "aircraft.reserve_energy_wh: an explicit non-zero protected reserve is required".into(),
        );
    }
    if input.aircraft.maximum_continuous_power_w > input.aircraft.maximum_peak_power_w {
        errors.push("aircraft continuous power limit cannot exceed peak power limit".into());
    }
    let mut reservation_ids = BTreeSet::new();
    for (index, reservation) in input.existing_reservations.iter().enumerate() {
        if reservation.reservation_id.trim().is_empty() || reservation.resource_id.trim().is_empty()
        {
            errors.push(format!(
                "existing_reservations[{index}]: identifiers must be non-empty"
            ));
        }
        if reservation.start_ms >= reservation.end_ms {
            errors.push(format!(
                "existing_reservations[{index}]: interval must be non-empty"
            ));
        }
        if !reservation_ids.insert(reservation.reservation_id.as_str()) {
            errors.push(format!(
                "existing_reservations[{index}].reservation_id: duplicate identity"
            ));
        }
    }
    errors
}

fn canonical_input_digest(input: &CoAvailabilityInput) -> Result<String, String> {
    let mut canonical = input.clone();
    for stop in &mut canonical.carrier.route_stops {
        stop.available_pad_resource_ids.sort();
    }
    canonical.existing_reservations.sort_by(|left, right| {
        (
            left.resource_id.as_str(),
            left.start_ms,
            left.end_ms,
            left.reservation_id.as_str(),
        )
            .cmp(&(
                right.resource_id.as_str(),
                right.start_ms,
                right.end_ms,
                right.reservation_id.as_str(),
            ))
    });
    let encoded = serde_json::to_vec(&canonical)
        .map_err(|error| format!("cannot serialize canonical co-availability input: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn record_evidence_status(
    status: EvidenceStatus,
    label: &str,
    findings: &mut Vec<String>,
    indeterminate: &mut bool,
) {
    if status != EvidenceStatus::FreshInDomain {
        *indeterminate = true;
        findings.push(format!("INDETERMINATE: {label} evidence is {status:?}"));
    }
}

fn find_stop(input: &CoAvailabilityInput, stop_id: &str) -> Option<StopWindow> {
    input
        .carrier
        .route_stops
        .iter()
        .enumerate()
        .find_map(|(index, stop)| {
            (stop.stop_id == stop_id).then_some(StopWindow {
                index,
                arrival_latest_ms: stop.arrival_latest_ms,
                departure_earliest_ms: stop.departure_earliest_ms,
            })
        })
}

fn find_first_available_launch(
    reservations: &[ResourceReservation],
    launch_pad_id: &str,
    aircraft_id: &str,
    recovery_pad_id: &str,
    earliest_launch_ms: u64,
    latest_launch_ms: u64,
    launch_service_ms: u64,
    launch_to_recovery_offset_ms: u64,
    recovery_service_ms: u64,
) -> Result<Option<(u64, u64)>, String> {
    let mut candidate = earliest_launch_ms;

    // Every conflict advances the candidate beyond at least one finite
    // reservation. The loop is bounded because there are finitely many
    // reservations and candidate time is strictly increasing.
    for _ in 0..=reservations.len() {
        if candidate > latest_launch_ms {
            return Ok(None);
        }
        let launch_end = candidate
            .checked_add(launch_service_ms)
            .ok_or_else(|| "launch service interval overflows".to_string())?;
        let recovery_start = candidate
            .checked_add(launch_to_recovery_offset_ms)
            .ok_or_else(|| "recovery start overflows".to_string())?;
        let recovery_end = recovery_start
            .checked_add(recovery_service_ms)
            .ok_or_else(|| "recovery service interval overflows".to_string())?;

        let mut next_candidate = candidate;
        let mut blocked = false;
        for reservation in reservations {
            let overlaps_launch_pad = reservation.resource_id == launch_pad_id
                && candidate < reservation.end_ms
                && reservation.start_ms < launch_end;
            let overlaps_aircraft = reservation.resource_id == aircraft_id
                && candidate < reservation.end_ms
                && reservation.start_ms < recovery_end;
            let overlaps_recovery_pad = reservation.resource_id == recovery_pad_id
                && recovery_start < reservation.end_ms
                && reservation.start_ms < recovery_end;

            if overlaps_launch_pad || overlaps_aircraft || overlaps_recovery_pad {
                blocked = true;
                if overlaps_launch_pad || overlaps_aircraft {
                    next_candidate = next_candidate.max(reservation.end_ms);
                }
                if overlaps_recovery_pad {
                    let candidate_after_recovery_reservation = reservation
                        .end_ms
                        .checked_sub(launch_to_recovery_offset_ms)
                        .ok_or_else(|| "recovery reservation offset underflows".to_string())?;
                    next_candidate = next_candidate.max(candidate_after_recovery_reservation);
                }
            }
        }

        if !blocked {
            return Ok(Some((candidate, recovery_start)));
        }
        if next_candidate <= candidate {
            return Err("resource conflict did not advance the launch candidate".into());
        }
        candidate = next_candidate;
    }

    Ok(None)
}

/// Evaluate whether one proposed sortie can fit the carrier's supported stop
/// windows and the supplied capability / operating-domain envelopes.
///
/// A FEASIBLE result is only a contract-level result over declared inputs.
/// It does not authorize flight, reserve a resource, or emit a command.
pub fn evaluate_coavailability(
    input: &CoAvailabilityInput,
) -> Result<CoAvailabilityReport, Vec<String>> {
    let errors = structural_errors(input);
    if !errors.is_empty() {
        return Err(errors);
    }
    let input_digest_sha256 = canonical_input_digest(input).map_err(|error| vec![error])?;

    let mut findings = Vec::new();
    let mut known_infeasible = false;
    let mut indeterminate = false;

    let sortie = &input.sortie;
    let aircraft = &input.aircraft;

    let payload_margin = aircraft
        .maximum_payload_mass_g
        .checked_sub(sortie.payload_mass_g);
    if payload_margin.is_none() {
        known_infeasible = true;
        findings.push("INFEASIBLE: payload exceeds aircraft maximum".into());
    }

    let required_energy = sortie
        .flight_energy_wh
        .checked_add(aircraft.reserve_energy_wh);
    let energy_margin =
        required_energy.and_then(|required| aircraft.usable_energy_wh.checked_sub(required));
    match (required_energy, energy_margin) {
        (None, _) => {
            known_infeasible = true;
            findings.push("INFEASIBLE: flight energy plus reserve overflows".into());
        }
        (Some(_), None) => {
            known_infeasible = true;
            findings.push(
                "INFEASIBLE: usable aircraft energy does not cover mission plus reserve".into(),
            );
        }
        (Some(_), Some(_)) => {}
    }

    let peak_margin = aircraft
        .maximum_peak_power_w
        .checked_sub(sortie.peak_power_w);
    if peak_margin.is_none() {
        known_infeasible = true;
        findings.push("INFEASIBLE: peak power demand exceeds aircraft capability".into());
    }
    let continuous_margin = aircraft
        .maximum_continuous_power_w
        .checked_sub(sortie.continuous_power_w);
    if continuous_margin.is_none() {
        known_infeasible = true;
        findings.push("INFEASIBLE: continuous power demand exceeds aircraft capability".into());
    }

    match input.operating_domain.permission {
        AirspaceStatus::Approved => {}
        AirspaceStatus::Prohibited | AirspaceStatus::Revoked => {
            known_infeasible = true;
            findings.push("INFEASIBLE: operating permission is prohibited or revoked".into());
        }
        AirspaceStatus::Unknown | AirspaceStatus::Expired => {
            indeterminate = true;
            findings.push("INDETERMINATE: current operating permission is not established".into());
        }
    }
    record_evidence_status(
        input.carrier.route_timing_evidence,
        "carrier route timing",
        &mut findings,
        &mut indeterminate,
    );
    record_evidence_status(
        input.carrier.site_resource_evidence,
        "carrier site / pad resource",
        &mut findings,
        &mut indeterminate,
    );
    record_evidence_status(
        aircraft.capability_evidence,
        "aircraft capability",
        &mut findings,
        &mut indeterminate,
    );
    record_evidence_status(
        input.operating_domain.permission_evidence,
        "operating permission",
        &mut findings,
        &mut indeterminate,
    );
    record_evidence_status(
        input.operating_domain.weather_evidence,
        "weather",
        &mut findings,
        &mut indeterminate,
    );

    let launch_stop = find_stop(input, &sortie.launch_stop_id);
    let recovery_stop = find_stop(input, &sortie.recovery_stop_id);
    let mut selected_launch = None;
    let mut selected_recovery = None;

    match (launch_stop, recovery_stop) {
        (None, _) => {
            known_infeasible = true;
            findings.push("INFEASIBLE: launch stop is not in the carrier itinerary".into());
        }
        (_, None) => {
            known_infeasible = true;
            findings.push("INFEASIBLE: recovery stop is not in the carrier itinerary".into());
        }
        (Some(launch), Some(recovery)) => {
            if launch.index > recovery.index {
                known_infeasible = true;
                findings
                    .push("INFEASIBLE: recovery stop precedes launch stop on carrier route".into());
            } else {
                let launch_pad_available = input.carrier.route_stops[launch.index]
                    .available_pad_resource_ids
                    .iter()
                    .any(|pad_id| pad_id == &sortie.launch_pad_resource_id);
                let recovery_pad_available = input.carrier.route_stops[recovery.index]
                    .available_pad_resource_ids
                    .iter()
                    .any(|pad_id| pad_id == &sortie.recovery_pad_resource_id);
                if !launch_pad_available {
                    known_infeasible = true;
                    findings.push(
                        "INFEASIBLE: launch pad is not declared at the launch route stop".into(),
                    );
                }
                if !recovery_pad_available {
                    known_infeasible = true;
                    findings.push(
                        "INFEASIBLE: recovery pad is not declared at the recovery route stop"
                            .into(),
                    );
                }
                let launch_latest_from_window = sortie
                    .launch_window_close_ms
                    .checked_sub(sortie.launch_service_ms);
                let launch_latest_from_stop = launch
                    .departure_earliest_ms
                    .checked_sub(sortie.launch_service_ms);
                let recovery_latest_from_window = sortie
                    .recovery_window_close_ms
                    .checked_sub(sortie.recovery_service_ms);
                let recovery_latest_from_stop = recovery
                    .departure_earliest_ms
                    .checked_sub(sortie.recovery_service_ms);
                // Recovery follows the launch service interval and then the flight.
                let launch_to_recovery_offset = sortie
                    .launch_service_ms
                    .checked_add(sortie.flight_duration_ms);

                match (
                    launch_latest_from_window,
                    launch_latest_from_stop,
                    recovery_latest_from_window,
                    recovery_latest_from_stop,
                    launch_to_recovery_offset,
                ) {
                    (
                        Some(launch_window_hi),
                        Some(launch_stop_hi),
                        Some(recovery_window_hi),
                        Some(recovery_stop_hi),
                        Some(launch_to_recovery_offset),
                    ) => {
                        let launch_lo = sortie.launch_window_open_ms.max(launch.arrival_latest_ms);
                        let launch_hi = launch_window_hi.min(launch_stop_hi);
                        let recovery_lo = sortie
                            .recovery_window_open_ms
                            .max(recovery.arrival_latest_ms);
                        let recovery_hi = recovery_window_hi.min(recovery_stop_hi);

                        let recovery_launch_lo =
                            recovery_lo.saturating_sub(launch_to_recovery_offset);
                        let recovery_launch_hi = recovery_hi.checked_sub(launch_to_recovery_offset);

                        if let Some(recovery_launch_hi) = recovery_launch_hi {
                            let feasible_launch_lo = launch_lo.max(recovery_launch_lo);
                            let feasible_launch_hi = launch_hi.min(recovery_launch_hi);
                            if feasible_launch_lo <= feasible_launch_hi {
                                match find_first_available_launch(
                                    &input.existing_reservations,
                                    &sortie.launch_pad_resource_id,
                                    &aircraft.aircraft_id,
                                    &sortie.recovery_pad_resource_id,
                                    feasible_launch_lo,
                                    feasible_launch_hi,
                                    sortie.launch_service_ms,
                                    launch_to_recovery_offset,
                                    sortie.recovery_service_ms,
                                ) {
                                    Ok(Some((launch_time, recovery_time))) => {
                                        selected_launch = Some(launch_time);
                                        selected_recovery = Some(recovery_time);
                                    }
                                    Ok(None) => {
                                        known_infeasible = true;
                                        findings.push(
                                            "INFEASIBLE: no conflict-free launch and recovery slot exists"
                                                .into(),
                                        );
                                    }
                                    Err(error) => {
                                        known_infeasible = true;
                                        findings.push(format!("INFEASIBLE: {error}"));
                                    }
                                }
                            } else {
                                known_infeasible = true;
                                findings.push(
                                    "INFEASIBLE: no launch time fits carrier and sortie windows"
                                        .into(),
                                );
                            }
                        } else {
                            known_infeasible = true;
                            findings.push(
                                "INFEASIBLE: recovery window closes before flight recovery".into(),
                            );
                        }
                    }
                    _ => {
                        known_infeasible = true;
                        findings.push(
                            "INFEASIBLE: service duration does not fit the carrier or site window"
                                .into(),
                        );
                    }
                }
            }
        }
    }

    if findings.is_empty() {
        findings.push(
            "FEASIBLE: supplied capability, permission and timing envelopes admit sortie".into(),
        );
    }
    let verdict = if known_infeasible {
        FeasibilityVerdict::Infeasible
    } else if indeterminate {
        FeasibilityVerdict::Indeterminate
    } else {
        FeasibilityVerdict::Feasible
    };

    Ok(CoAvailabilityReport {
        schema_version: COAVAILABILITY_SCHEMA_VERSION,
        input_digest_sha256,
        scenario_id: input.scenario_id.clone(),
        sortie_id: sortie.sortie_id.clone(),
        verdict,
        selected_launch_time_ms: selected_launch,
        selected_recovery_time_ms: selected_recovery,
        payload_margin_g: payload_margin,
        energy_margin_wh: energy_margin,
        peak_power_margin_w: peak_margin,
        continuous_power_margin_w: continuous_margin,
        findings,
        energy_boundary: "UAV battery energy + reserve; excludes carrier energy and charging losses".into(),
        dispatch_authorized: false,
        physical_command_emitted: false,
        claim_ceiling: "Contract feasibility only; not flight, airworthiness, safety or performance authorization".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> CoAvailabilityInput {
        CoAvailabilityInput {
            schema_version: 1,
            scenario_id: "synthetic-carrier-uav-v1".into(),
            carrier: CarrierEnvelope {
                carrier_id: "carrier-01".into(),
                route_timing_evidence: EvidenceStatus::FreshInDomain,
                site_resource_evidence: EvidenceStatus::FreshInDomain,
                route_stops: vec![
                    CarrierStop {
                        stop_id: "stop-01".into(),
                        node_id: "depot".into(),
                        arrival_earliest_ms: 0,
                        arrival_latest_ms: 0,
                        departure_earliest_ms: 1_000_000,
                        departure_latest_ms: 1_000_000,
                        available_pad_resource_ids: vec!["pad-depot-1".into()],
                    },
                    CarrierStop {
                        stop_id: "stop-02".into(),
                        node_id: "hub-b".into(),
                        arrival_earliest_ms: 1_300_000,
                        arrival_latest_ms: 1_300_000,
                        departure_earliest_ms: 1_900_000,
                        departure_latest_ms: 1_900_000,
                        available_pad_resource_ids: vec!["pad-hub-b-1".into()],
                    },
                ],
            },
            aircraft: AircraftEnvelope {
                aircraft_id: "uav-01".into(),
                capability_evidence: EvidenceStatus::FreshInDomain,
                maximum_payload_mass_g: 2_000,
                usable_energy_wh: 500,
                reserve_energy_wh: 100,
                maximum_peak_power_w: 1_500,
                maximum_continuous_power_w: 800,
            },
            sortie: SortieProposal {
                sortie_id: "sortie-01".into(),
                payload_mass_g: 1_000,
                launch_stop_id: "stop-01".into(),
                recovery_stop_id: "stop-02".into(),
                launch_pad_resource_id: "pad-depot-1".into(),
                recovery_pad_resource_id: "pad-hub-b-1".into(),
                launch_window_open_ms: 100_000,
                launch_window_close_ms: 300_000,
                recovery_window_open_ms: 1_300_000,
                recovery_window_close_ms: 1_700_000,
                launch_service_ms: 60_000,
                recovery_service_ms: 60_000,
                flight_duration_ms: 1_200_000,
                flight_energy_wh: 300,
                peak_power_w: 1_200,
                continuous_power_w: 600,
            },
            operating_domain: OperatingDomainEnvelope {
                permission: AirspaceStatus::Approved,
                permission_evidence: EvidenceStatus::FreshInDomain,
                weather_evidence: EvidenceStatus::FreshInDomain,
            },
            existing_reservations: vec![],
        }
    }

    #[test]
    fn canonical_input_digest_ignores_pad_and_reservation_input_order() {
        let mut first = input();
        first.carrier.route_stops[0]
            .available_pad_resource_ids
            .push("secondary-pad".into());
        first.existing_reservations = vec![
            ResourceReservation {
                reservation_id: "reservation-a".into(),
                resource_id: "unused-pad-a".into(),
                start_ms: 10,
                end_ms: 20,
            },
            ResourceReservation {
                reservation_id: "reservation-b".into(),
                resource_id: "unused-pad-b".into(),
                start_ms: 30,
                end_ms: 40,
            },
        ];
        let mut second = first.clone();
        second.carrier.route_stops[0]
            .available_pad_resource_ids
            .reverse();
        second.existing_reservations.reverse();

        let first_report = evaluate_coavailability(&first).expect("first input");
        let second_report = evaluate_coavailability(&second).expect("permuted input");
        assert_eq!(
            first_report.input_digest_sha256,
            second_report.input_digest_sha256
        );
        assert_eq!(first_report.verdict, second_report.verdict);
    }

    #[test]
    fn canonical_input_digest_changes_when_mission_requirement_changes() {
        let first = evaluate_coavailability(&input()).expect("first input");
        let mut changed = input();
        changed.sortie.flight_energy_wh += 1;
        let second = evaluate_coavailability(&changed).expect("changed input");
        assert_ne!(first.input_digest_sha256, second.input_digest_sha256);
    }

    #[test]
    fn input_digest_is_lowercase_sha256() {
        let report = evaluate_coavailability(&input()).expect("valid input");
        assert_eq!(report.input_digest_sha256.len(), 64);
        assert!(
            report
                .input_digest_sha256
                .chars()
                .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
        );
    }

    #[test]
    fn accepts_synthetic_sortie_with_overlapping_carrier_windows() {
        let report = evaluate_coavailability(&input()).expect("valid input");
        assert_eq!(report.verdict, FeasibilityVerdict::Feasible);
        assert_eq!(report.selected_launch_time_ms, Some(100_000));
        assert_eq!(report.selected_recovery_time_ms, Some(1_360_000));
        assert_eq!(report.energy_margin_wh, Some(100));
        assert!(!report.dispatch_authorized);
        assert!(!report.physical_command_emitted);
    }

    #[test]
    fn rejects_payload_over_limit() {
        let mut test = input();
        test.sortie.payload_mass_g = 2_001;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
    }

    #[test]
    fn rejects_mission_that_consumes_protected_reserve() {
        let mut test = input();
        test.sortie.flight_energy_wh = 401;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
    }

    #[test]
    fn rejects_peak_and_continuous_power_excess() {
        let mut test = input();
        test.sortie.peak_power_w = 1_501;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.contains("peak power"))
        );
    }

    #[test]
    fn stale_carrier_route_timing_is_indeterminate() {
        let mut test = input();
        test.carrier.route_timing_evidence = EvidenceStatus::Stale;
        let report = evaluate_coavailability(&test).expect("valid input");
        assert_eq!(report.verdict, FeasibilityVerdict::Indeterminate);
    }

    #[test]
    fn stale_site_resource_evidence_is_indeterminate() {
        let mut test = input();
        test.carrier.site_resource_evidence = EvidenceStatus::Stale;
        let report = evaluate_coavailability(&test).expect("valid input");
        assert_eq!(report.verdict, FeasibilityVerdict::Indeterminate);
    }

    #[test]
    fn approved_but_stale_permission_evidence_is_indeterminate() {
        let mut test = input();
        test.operating_domain.permission_evidence = EvidenceStatus::Stale;
        let report = evaluate_coavailability(&test).expect("valid input");
        assert_eq!(report.verdict, FeasibilityVerdict::Indeterminate);
    }

    #[test]
    fn unknown_permission_is_indeterminate_not_allowed() {
        let mut test = input();
        test.operating_domain.permission = AirspaceStatus::Unknown;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Indeterminate);
    }

    #[test]
    fn stale_weather_is_indeterminate() {
        let mut test = input();
        test.operating_domain.weather_evidence = EvidenceStatus::Stale;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Indeterminate);
    }

    #[test]
    fn rejects_recovery_window_without_possible_rendezvous() {
        let mut test = input();
        test.sortie.recovery_window_open_ms = 1_800_000;
        test.sortie.recovery_window_close_ms = 1_900_000;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
    }

    #[test]
    fn rejects_pad_resource_not_attached_to_the_selected_stop() {
        let mut test = input();
        test.sortie.recovery_pad_resource_id = "pad-at-another-site".into();
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.contains("not declared"))
        );
    }

    #[test]
    fn shifts_to_next_free_resource_slot_instead_of_rejecting_first_slot() {
        let mut test = input();
        test.sortie.launch_window_close_ms = 400_000;
        test.existing_reservations.push(ResourceReservation {
            reservation_id: "prior-uav-operation".into(),
            resource_id: "uav-01".into(),
            start_ms: 90_000,
            end_ms: 180_000,
        });
        let report = evaluate_coavailability(&test).expect("valid input");
        assert_eq!(report.verdict, FeasibilityVerdict::Feasible);
        assert_eq!(report.selected_launch_time_ms, Some(180_000));
        assert_eq!(report.selected_recovery_time_ms, Some(1_440_000));
    }

    #[test]
    fn rejects_aircraft_double_booking_during_flight_or_turnaround() {
        let mut test = input();
        test.existing_reservations.push(ResourceReservation {
            reservation_id: "other-uav-operation".into(),
            resource_id: "uav-01".into(),
            start_ms: 700_000,
            end_ms: 800_000,
        });
        let report = evaluate_coavailability(&test).expect("valid input");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.contains("aircraft is reserved"))
        );
    }

    #[test]
    fn finds_next_launch_pad_slot_after_reservation() {
        let mut test = input();
        test.sortie.launch_window_close_ms = 400_000;
        test.existing_reservations.push(ResourceReservation {
            reservation_id: "other-sortie".into(),
            resource_id: "pad-depot-1".into(),
            start_ms: 120_000,
            end_ms: 180_000,
        });
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Feasible);
        assert_eq!(report.selected_launch_time_ms, Some(180_000));
    }

    #[test]
    fn rejects_launch_pad_conflict_when_entire_window_is_reserved() {
        let mut test = input();
        test.sortie.launch_window_close_ms = 200_000;
        test.existing_reservations.push(ResourceReservation {
            reservation_id: "other-sortie".into(),
            resource_id: "pad-depot-1".into(),
            start_ms: 120_000,
            end_ms: 180_000,
        });
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
    }

    #[test]
    fn rejects_recovery_before_launch_in_carrier_route() {
        let mut test = input();
        test.sortie.launch_stop_id = "stop-02".into();
        test.sortie.recovery_stop_id = "stop-01".into();
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
    }

    #[test]
    fn uses_latest_arrival_and_earliest_departure_for_guaranteed_presence() {
        let mut test = input();
        test.carrier.route_stops[1].arrival_earliest_ms = 1_200_000;
        test.carrier.route_stops[1].arrival_latest_ms = 1_550_000;
        test.carrier.route_stops[1].departure_earliest_ms = 1_610_000;
        test.carrier.route_stops[1].departure_latest_ms = 1_700_000;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.contains("no launch time"))
        );
    }

    #[test]
    fn rejects_continuous_power_excess() {
        let mut test = input();
        test.sortie.continuous_power_w = 801;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.contains("continuous power"))
        );
    }

    #[test]
    fn revoked_permission_is_infeasible() {
        let mut test = input();
        test.operating_domain.permission = AirspaceStatus::Revoked;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
    }

    #[test]
    fn unsupported_aircraft_capability_is_indeterminate() {
        let mut test = input();
        test.aircraft.capability_evidence = EvidenceStatus::Unsupported;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Indeterminate);
    }

    #[test]
    fn accepts_exact_energy_reserve_boundary() {
        let mut test = input();
        test.sortie.flight_energy_wh = 400;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Feasible);
        assert_eq!(report.energy_margin_wh, Some(0));
    }

    #[test]
    fn same_stop_return_requires_carrier_to_remain_until_recovery() {
        let mut test = input();
        test.sortie.recovery_stop_id = "stop-01".into();
        test.sortie.recovery_window_open_ms = 500_000;
        test.sortie.recovery_window_close_ms = 900_000;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
    }

    #[test]
    fn finds_next_recovery_pad_slot_after_reservation() {
        let mut test = input();
        test.existing_reservations.push(ResourceReservation {
            reservation_id: "other-sortie".into(),
            resource_id: "pad-hub-b-1".into(),
            start_ms: 1_350_000,
            end_ms: 1_400_000,
        });
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Feasible);
        assert_eq!(report.selected_launch_time_ms, Some(140_000));
        assert_eq!(report.selected_recovery_time_ms, Some(1_400_000));
    }

    #[test]
    fn rejects_recovery_pad_conflict_when_entire_window_is_reserved() {
        let mut test = input();
        test.existing_reservations.push(ResourceReservation {
            reservation_id: "other-sortie".into(),
            resource_id: "pad-hub-b-1".into(),
            start_ms: 1_300_000,
            end_ms: 1_700_000,
        });
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
    }

    #[test]
    fn rejects_non_chronological_carrier_route_stop_envelopes() {
        let mut test = input();
        test.carrier.route_stops[1].arrival_earliest_ms = 900_000;
        test.carrier.route_stops[1].arrival_latest_ms = 1_300_000;
        let errors = evaluate_coavailability(&test).expect_err("overlapping route chronology");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("time envelopes overlap or reverse"))
        );
    }

    #[test]
    fn rejects_zero_protected_reserve() {
        let mut test = input();
        test.aircraft.reserve_energy_wh = 0;
        let errors = evaluate_coavailability(&test).expect_err("reserve must be explicit");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("non-zero protected reserve"))
        );
    }

    #[test]
    fn rejects_rendezvous_that_ignores_launch_service_duration() {
        let mut test = input();
        test.sortie.recovery_window_close_ms = 1_350_000;
        test.sortie.recovery_service_ms = 20_000;
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Infeasible);
    }

    #[test]
    fn launch_service_time_is_added_before_flight_duration() {
        let mut test = input();
        test.sortie.launch_service_ms = 180_000;
        test.sortie.flight_duration_ms = 1_200_000;
        test.sortie.recovery_window_open_ms = 1_400_000;
        let report = evaluate_coavailability(&test).expect("valid input");
        assert_eq!(report.verdict, FeasibilityVerdict::Feasible);
        assert_eq!(report.selected_launch_time_ms, Some(100_000));
        assert_eq!(report.selected_recovery_time_ms, Some(1_480_000));
    }

    #[test]
    fn rejects_duplicate_route_stop_identity() {
        let mut test = input();
        test.carrier.route_stops.push(CarrierStop {
            stop_id: "stop-01".into(),
            node_id: "depot".into(),
            arrival_earliest_ms: 2_000_000,
            arrival_latest_ms: 2_000_000,
            departure_earliest_ms: 3_000_000,
            departure_latest_ms: 3_000_000,
            available_pad_resource_ids: vec!["pad-depot-1".into()],
        });
        assert!(evaluate_coavailability(&test).is_err());
    }

    #[test]
    fn rejects_unknown_json_fields() {
        let json = serde_json::to_value(input()).expect("serialize");
        let mut object = json.as_object().expect("object").clone();
        object.insert("optimistic_guess".into(), serde_json::json!(true));
        assert!(serde_json::from_value::<CoAvailabilityInput>(object.into()).is_err());
    }

    #[test]
    fn half_open_reservations_allow_exact_adjacency() {
        let mut test = input();
        test.existing_reservations.push(ResourceReservation {
            reservation_id: "completed-before-launch".into(),
            resource_id: "pad-depot-1".into(),
            start_ms: 40_000,
            end_ms: 100_000,
        });
        let report = evaluate_coavailability(&test).expect("valid structure");
        assert_eq!(report.verdict, FeasibilityVerdict::Feasible);
    }
}
