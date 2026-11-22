// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded event-driven model of sustainable battery-backed service cycles.
//!
//! This composes aircraft turnaround, a finite interchangeable pack pool,
//! FCFS charging channels and a site import-power limit. It is a deterministic
//! capacity model, not a battery electrochemistry model, physical charger,
//! flight planner, weather/permission model or service authorization.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

const MAX_SCENARIOS: usize = 16;
const MAX_AIRCRAFT: u8 = 8;
const MAX_BATTERY_PACKS: u8 = 64;
const MAX_CHARGING_CHANNELS: u8 = 16;
const MAX_HORIZON_SECONDS: u64 = 7 * 24 * 60 * 60;
const MAX_EVENTS: usize = 200_000;
const MAX_POWER_W: u32 = 100_000_000;

const CLAIM_CEILING: &str = "Bounded synthetic cyclic service-capacity simulation only; all launched sorties are assumed to complete, and supplied mission, charge, turnaround, power and inventory values are assumptions unless separately sourced. No battery safety, field availability, aviation permission, grid interconnection, dispatch, market or physical authority.";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceArchitecture {
    pub architecture_id: String,
    pub description: String,
    /// One aircraft per service unit in this bounded v0.1 model.
    pub aircraft_count: u8,
    /// Includes packs installed on aircraft and spare packs.
    pub battery_pack_count: u8,
    /// All non-installed packs begin fully ready; no initial charging cohort.
    pub initial_ready_spares: u8,
    pub max_stored_spares_per_aircraft: u8,
    pub mission_duration_seconds: u32,
    /// Landing-to-next-takeoff time when a ready pack is available.
    pub operational_turnaround_seconds: u32,
    /// Fixed time to restore one depleted pack to the model's service-ready state.
    pub charge_duration_seconds: u32,
    pub parallel_charging_channels: u8,
    /// Scenario input-power proxy per simultaneously active charge channel.
    pub charger_input_power_w: u32,
    pub site_grid_import_limit_w: u32,
    /// Site load other than the modelled charger channels.
    pub site_fixed_load_w: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SustainableServiceCapacityInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub horizon_seconds: u64,
    pub architectures: Vec<ServiceArchitecture>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ArchitectureCapacityResult {
    pub architecture_id: String,
    pub description: String,
    pub horizon_seconds: u64,
    pub effective_charging_channels: u8,
    pub modelled_charging_power_w: u64,
    pub completed_sorties: u64,
    pub completed_mission_hours: f64,
    pub completed_cycles_per_hour: f64,
    pub completed_cycles_per_aircraft_hour: f64,
    pub charge_starts: u64,
    pub charge_completions: u64,
    pub charge_queue_peak: usize,
    pub charge_queue_at_horizon: usize,
    pub active_chargers_at_horizon: u8,
    pub ready_spares_at_horizon: u8,
    pub waiting_aircraft_peak: usize,
    pub waiting_aircraft_at_horizon: usize,
    pub aircraft_wait_seconds: u64,
    pub aircraft_wait_share_percent: f64,
    pub max_single_aircraft_wait_seconds: u64,
    pub mission_completion_events_after_horizon: usize,
    pub charger_input_energy_proxy_wh: f64,
    pub site_fixed_load_energy_wh: f64,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SustainableServiceCapacityReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub horizon_seconds: u64,
    pub architecture_results: Vec<ArchitectureCapacityResult>,
    pub capacity_is_physical_performance_claim: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Copy)]
enum ScheduledEvent {
    ChargeComplete { channel_id: u8 },
    MissionComplete { aircraft_id: u8 },
}

impl ScheduledEvent {
    fn order_key(self) -> (u8, u8) {
        match self {
            Self::ChargeComplete { channel_id } => (0, channel_id),
            Self::MissionComplete { aircraft_id } => (1, aircraft_id),
        }
    }
}

fn add_error(errors: &mut Vec<String>, value: impl Into<String>) {
    errors.push(value.into());
}

fn validate_architecture(
    scenario: &ServiceArchitecture,
    horizon_seconds: u64,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if scenario.architecture_id.trim().is_empty()
        || scenario.architecture_id.len() > 80
        || scenario.description.trim().is_empty()
        || scenario.description.len() > 1_024
        || scenario.aircraft_count == 0
        || scenario.aircraft_count > MAX_AIRCRAFT
        || scenario.battery_pack_count < scenario.aircraft_count
        || scenario.battery_pack_count > MAX_BATTERY_PACKS
        || scenario.initial_ready_spares
            != scenario.battery_pack_count.saturating_sub(scenario.aircraft_count)
        || scenario.max_stored_spares_per_aircraft > 8
        || scenario.initial_ready_spares
            > scenario
                .max_stored_spares_per_aircraft
                .saturating_mul(scenario.aircraft_count)
        || scenario.mission_duration_seconds == 0
        || u64::from(scenario.mission_duration_seconds) > horizon_seconds
        || scenario.operational_turnaround_seconds > 86_400
        || scenario.charge_duration_seconds == 0
        || scenario.charge_duration_seconds > 86_400
        || scenario.parallel_charging_channels > MAX_CHARGING_CHANNELS
        || scenario.charger_input_power_w == 0
        || scenario.charger_input_power_w > MAX_POWER_W
        || scenario.site_grid_import_limit_w > MAX_POWER_W
        || scenario.site_fixed_load_w > MAX_POWER_W
        || scenario.site_fixed_load_w > scenario.site_grid_import_limit_w
    {
        add_error(
            &mut errors,
            format!("architecture_identity_or_bounds_invalid:{}", scenario.architecture_id),
        );
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        Err(errors)
    } else {
        Ok(())
    }
}

fn effective_charging_channels(scenario: &ServiceArchitecture) -> u8 {
    let available_power = scenario
        .site_grid_import_limit_w
        .saturating_sub(scenario.site_fixed_load_w);
    let power_limited_channels = available_power / scenario.charger_input_power_w;
    u8::try_from(power_limited_channels)
        .unwrap_or(u8::MAX)
        .min(scenario.parallel_charging_channels)
}

fn energy_proxy_mwh(power_w: u32, seconds: u64) -> Result<u64, String> {
    let value = u128::from(power_w)
        .checked_mul(u128::from(seconds))
        .and_then(|value| value.checked_mul(1_000))
        .ok_or_else(|| "charging_energy_proxy_overflow".to_owned())?
        / 3_600;
    u64::try_from(value).map_err(|_| "charging_energy_proxy_overflow".to_owned())
}

fn start_queued_charges(
    now: u64,
    horizon: u64,
    charge_duration: u32,
    charger_power: u32,
    free_channels: &mut BTreeSet<u8>,
    charge_queue: &mut VecDeque<()>,
    timeline: &mut BTreeMap<u64, Vec<ScheduledEvent>>,
    charge_starts: &mut u64,
    charging_input_energy_mwh: &mut u64,
) -> Result<(), String> {
    while let (Some(channel_id), Some(())) =
        (free_channels.iter().next().copied(), charge_queue.front().copied())
    {
        charge_queue.pop_front();
        free_channels.remove(&channel_id);

        let completion_time = now
            .checked_add(u64::from(charge_duration))
            .ok_or_else(|| "charge_completion_time_overflow".to_owned())?;
        timeline
            .entry(completion_time)
            .or_default()
            .push(ScheduledEvent::ChargeComplete { channel_id });

        let charged_seconds = u64::from(charge_duration).min(horizon.saturating_sub(now));
        let energy = energy_proxy_mwh(charger_power, charged_seconds)?;
        *charging_input_energy_mwh = charging_input_energy_mwh
            .checked_add(energy)
            .ok_or_else(|| "charging_input_energy_accumulator_overflow".to_owned())?;
        *charge_starts = charge_starts
            .checked_add(1)
            .ok_or_else(|| "charge_start_count_overflow".to_owned())?;
    }
    Ok(())
}

pub fn evaluate_architecture_capacity(
    scenario: &ServiceArchitecture,
    horizon_seconds: u64,
) -> Result<ArchitectureCapacityResult, Vec<String>> {
    validate_architecture(scenario, horizon_seconds)?;

    let channels = effective_charging_channels(scenario);
    let modelled_charging_power_w =
        u64::from(channels).saturating_mul(u64::from(scenario.charger_input_power_w));
    let mut free_channels: BTreeSet<u8> = (0..channels).collect();
    let mut ready_spares = scenario.initial_ready_spares;
    let mut charge_queue: VecDeque<()> = VecDeque::new();
    let mut waiting_aircraft: VecDeque<(u8, u64)> = VecDeque::new();
    let mut timeline: BTreeMap<u64, Vec<ScheduledEvent>> = BTreeMap::new();

    let mut charge_starts = 0_u64;
    let mut charge_completions = 0_u64;
    let mut completed_sorties = 0_u64;
    let mut charge_queue_peak = 0_usize;
    let mut waiting_aircraft_peak = 0_usize;
    let mut aircraft_wait_seconds = 0_u64;
    let mut max_single_aircraft_wait_seconds = 0_u64;
    let mut charging_input_energy_mwh = 0_u64;
    let mut event_count = 0_usize;

    for aircraft_id in 0..scenario.aircraft_count {
        timeline
            .entry(u64::from(scenario.mission_duration_seconds))
            .or_default()
            .push(ScheduledEvent::MissionComplete { aircraft_id });
    }

    while let Some((&now, _)) = timeline.first_key_value() {
        if now > horizon_seconds {
            break;
        }
        let mut events = timeline.remove(&now).expect("first key was observed above");
        events.sort_by_key(|event| event.order_key());

        for event in events {
            event_count = event_count
                .checked_add(1)
                .ok_or_else(|| vec!["event_count_overflow".to_owned()])?;
            if event_count > MAX_EVENTS {
                return Err(vec!["bounded_event_limit_exceeded".to_owned()]);
            }

            match event {
                ScheduledEvent::ChargeComplete { channel_id } => {
                    if !free_channels.insert(channel_id) {
                        return Err(vec!["charging_channel_completed_while_free".to_owned()]);
                    }
                    charge_completions = charge_completions
                        .checked_add(1)
                        .ok_or_else(|| vec!["charge_completion_count_overflow".to_owned()])?;

                    if let Some((aircraft_id, wait_started_at)) = waiting_aircraft.pop_front() {
                        let wait = now.saturating_sub(wait_started_at);
                        aircraft_wait_seconds = aircraft_wait_seconds
                            .checked_add(wait)
                            .ok_or_else(|| vec!["aircraft_wait_accumulator_overflow".to_owned()])?;
                        max_single_aircraft_wait_seconds =
                            max_single_aircraft_wait_seconds.max(wait);

                        let next_mission = now
                            .checked_add(u64::from(scenario.operational_turnaround_seconds))
                            .and_then(|value| {
                                value.checked_add(u64::from(scenario.mission_duration_seconds))
                            })
                            .ok_or_else(|| {
                                vec!["next_mission_completion_time_overflow".to_owned()]
                            })?;
                        timeline
                            .entry(next_mission)
                            .or_default()
                            .push(ScheduledEvent::MissionComplete { aircraft_id });
                    } else {
                        ready_spares = ready_spares.checked_add(1).ok_or_else(|| {
                            vec!["ready_spare_count_overflow".to_owned()]
                        })?;
                    }

                    start_queued_charges(
                        now,
                        horizon_seconds,
                        scenario.charge_duration_seconds,
                        scenario.charger_input_power_w,
                        &mut free_channels,
                        &mut charge_queue,
                        &mut timeline,
                                        &mut charge_starts,
                        &mut charging_input_energy_mwh,
                    )
                    .map_err(|error| vec![error])?;
                }
                ScheduledEvent::MissionComplete { aircraft_id } => {
                    completed_sorties = completed_sorties
                        .checked_add(1)
                        .ok_or_else(|| vec!["completed_sortie_count_overflow".to_owned()])?;

                    charge_queue.push_back(());

                    if ready_spares > 0 {
                        ready_spares -= 1;
                        let next_mission = now
                            .checked_add(u64::from(scenario.operational_turnaround_seconds))
                            .and_then(|value| {
                                value.checked_add(u64::from(scenario.mission_duration_seconds))
                            })
                            .ok_or_else(|| {
                                vec!["next_mission_completion_time_overflow".to_owned()]
                            })?;
                        timeline
                            .entry(next_mission)
                            .or_default()
                            .push(ScheduledEvent::MissionComplete { aircraft_id });
                    } else {
                        waiting_aircraft.push_back((aircraft_id, now));
                        waiting_aircraft_peak =
                            waiting_aircraft_peak.max(waiting_aircraft.len());
                    }

                    start_queued_charges(
                        now,
                        horizon_seconds,
                        scenario.charge_duration_seconds,
                        scenario.charger_input_power_w,
                        &mut free_channels,
                        &mut charge_queue,
                        &mut timeline,
                                        &mut charge_starts,
                        &mut charging_input_energy_mwh,
                    )
                    .map_err(|error| vec![error])?;
                    charge_queue_peak = charge_queue_peak.max(charge_queue.len());
                }
            }
        }
    }

    for (_, wait_started_at) in &waiting_aircraft {
        let remaining_wait = horizon_seconds.saturating_sub(*wait_started_at);
        aircraft_wait_seconds = aircraft_wait_seconds
            .checked_add(remaining_wait)
            .ok_or_else(|| vec!["aircraft_wait_accumulator_overflow".to_owned()])?;
        max_single_aircraft_wait_seconds =
            max_single_aircraft_wait_seconds.max(remaining_wait);
    }

    let mission_seconds = completed_sorties
        .checked_mul(u64::from(scenario.mission_duration_seconds))
        .ok_or_else(|| vec!["completed_mission_time_overflow".to_owned()])?;
    let total_aircraft_seconds = u64::from(scenario.aircraft_count)
        .checked_mul(horizon_seconds)
        .ok_or_else(|| vec!["aircraft_time_denominator_overflow".to_owned()])?;
    let horizon_hours = horizon_seconds as f64 / 3_600.0;
    let completed_mission_hours = mission_seconds as f64 / 3_600.0;
    let completed_cycles_per_hour = completed_sorties as f64 / horizon_hours;
    let completed_cycles_per_aircraft_hour =
        completed_sorties as f64 * 3_600.0 / total_aircraft_seconds as f64;
    let aircraft_wait_share_percent =
        aircraft_wait_seconds as f64 * 100.0 / total_aircraft_seconds as f64;
    let site_fixed_load_energy_wh = scenario.site_fixed_load_w as f64 * horizon_hours;
    let charging_queue_at_horizon = charge_queue.len();
    let active_chargers_at_horizon =
        u8::try_from(channels as usize - free_channels.len()).unwrap_or(u8::MAX);
    let mission_completion_events_after_horizon = timeline
        .values()
        .flat_map(|events| events.iter())
        .filter(|event| matches!(event, ScheduledEvent::MissionComplete { .. }))
        .count();

    Ok(ArchitectureCapacityResult {
        architecture_id: scenario.architecture_id.clone(),
        description: scenario.description.clone(),
        horizon_seconds,
        effective_charging_channels: channels,
        modelled_charging_power_w,
        completed_sorties,
        completed_mission_hours,
        completed_cycles_per_hour,
        completed_cycles_per_aircraft_hour,
        charge_starts,
        charge_completions,
        charge_queue_peak,
        charge_queue_at_horizon: charging_queue_at_horizon,
        active_chargers_at_horizon,
        ready_spares_at_horizon: ready_spares,
        waiting_aircraft_peak,
        waiting_aircraft_at_horizon: waiting_aircraft.len(),
        aircraft_wait_seconds,
        aircraft_wait_share_percent,
        max_single_aircraft_wait_seconds,
        mission_completion_events_after_horizon,
        charger_input_energy_proxy_wh: charging_input_energy_mwh as f64 / 1_000.0,
        site_fixed_load_energy_wh,
        claim_ceiling: CLAIM_CEILING,
    })
}

pub fn evaluate_sustainable_service_capacity(
    input: &SustainableServiceCapacityInput,
) -> Result<SustainableServiceCapacityReport, Vec<String>> {
    let mut errors = Vec::new();

    if input.schema_version != "0.1.0"
        || input.benchmark_id != "SUSTAINABLE-MOBILITY-SERVICE-CAPACITY-001"
        || input.horizon_seconds == 0
        || input.horizon_seconds > MAX_HORIZON_SECONDS
        || input.architectures.is_empty()
        || input.architectures.len() > MAX_SCENARIOS
        || input.claim_ceiling.trim().is_empty()
    {
        add_error(&mut errors, "benchmark_identity_or_horizon_invalid");
    }

    let mut ids = BTreeSet::new();
    for scenario in &input.architectures {
        if !ids.insert(scenario.architecture_id.as_str()) {
            add_error(
                &mut errors,
                format!("architecture_id_duplicate:{}", scenario.architecture_id),
            );
        }
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut architecture_results = Vec::new();
    for scenario in &input.architectures {
        match evaluate_architecture_capacity(scenario, input.horizon_seconds) {
            Ok(result) => architecture_results.push(result),
            Err(scenario_errors) => errors.extend(scenario_errors),
        }
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    architecture_results.sort_by(|a, b| a.architecture_id.cmp(&b.architecture_id));

    Ok(SustainableServiceCapacityReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        horizon_seconds: input.horizon_seconds,
        architecture_results,
        capacity_is_physical_performance_claim: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> SustainableServiceCapacityInput {
        serde_json::from_str(include_str!(
            "../fixtures/sustainable_mobility_service_capacity_v0_1.json"
        ))
        .expect("frozen service capacity fixture must parse")
    }

    fn direct_charge() -> ServiceArchitecture {
        fixture().architectures[0].clone()
    }

    #[test]
    fn fixture_outputs_are_deterministic_and_sorted() {
        let input = fixture();
        let first = evaluate_sustainable_service_capacity(&input).expect("fixture");
        let second = evaluate_sustainable_service_capacity(&input).expect("replay");
        assert_eq!(
            serde_json::to_string(&first).expect("serialize first"),
            serde_json::to_string(&second).expect("serialize second")
        );
        assert!(first
            .architecture_results
            .windows(2)
            .all(|pair| pair[0].architecture_id <= pair[1].architecture_id));
        assert!(!first.capacity_is_physical_performance_claim);
    }

    #[test]
    fn zero_spare_direct_charge_has_longer_cycle_than_fast_swap_profile() {
        let input = fixture();
        let direct = evaluate_architecture_capacity(&input.architectures[0], input.horizon_seconds)
            .expect("direct charge");
        let dock = evaluate_architecture_capacity(&input.architectures[1], input.horizon_seconds)
            .expect("dock profile");
        assert!(direct.aircraft_wait_seconds > 0);
        assert!(dock.completed_sorties > direct.completed_sorties);
        assert!(dock.completed_cycles_per_hour > direct.completed_cycles_per_hour);
    }

    #[test]
    fn site_power_caps_effective_charging_channels() {
        let input = fixture();
        let unconstrained = evaluate_architecture_capacity(&input.architectures[1], input.horizon_seconds)
            .expect("unconstrained profile");
        let constrained = evaluate_architecture_capacity(&input.architectures[2], input.horizon_seconds)
            .expect("constrained profile");
        assert_eq!(unconstrained.effective_charging_channels, 2);
        assert_eq!(constrained.effective_charging_channels, 1);
        assert!(constrained.aircraft_wait_seconds >= unconstrained.aircraft_wait_seconds);
    }

    #[test]
    fn no_grid_headroom_means_no_charge_completion_after_pool_depletes() {
        let mut scenario = direct_charge();
        scenario.site_grid_import_limit_w = scenario.site_fixed_load_w;
        scenario.initial_ready_spares = 0;
        scenario.battery_pack_count = scenario.aircraft_count;
        let result = evaluate_architecture_capacity(&scenario, 10_000).expect("bounded zero grid");
        assert_eq!(result.effective_charging_channels, 0);
        assert_eq!(result.charge_completions, 0);
        assert!(result.waiting_aircraft_at_horizon > 0);
    }

    #[test]
    fn one_pack_baseline_waits_for_its_own_recharge() {
        let mut scenario = direct_charge();
        scenario.battery_pack_count = 1;
        scenario.initial_ready_spares = 0;
        scenario.mission_duration_seconds = 300;
        scenario.charge_duration_seconds = 1_620;
        scenario.operational_turnaround_seconds = 45;
        let result = evaluate_architecture_capacity(&scenario, 3_000).expect("one pack");
        assert_eq!(result.completed_sorties, 2);
        assert!(result.aircraft_wait_seconds >= 1_620);
    }

    #[test]
    fn extra_ready_inventory_reduces_wait_for_the_short_cycle_stress_case() {
        let mut one_spare = direct_charge();
        one_spare.battery_pack_count = 2;
        one_spare.initial_ready_spares = 1;
        one_spare.mission_duration_seconds = 300;
        one_spare.charge_duration_seconds = 1_620;
        one_spare.max_stored_spares_per_aircraft = 3;
        let mut three_spares = one_spare.clone();
        three_spares.battery_pack_count = 4;
        three_spares.initial_ready_spares = 3;
        let a = evaluate_architecture_capacity(&one_spare, 20_000).expect("one spare");
        let b = evaluate_architecture_capacity(&three_spares, 20_000).expect("three spares");
        assert!(b.aircraft_wait_seconds < a.aircraft_wait_seconds);
        assert!(b.completed_sorties >= a.completed_sorties);
    }

    #[test]
    fn zero_charge_or_mission_duration_is_rejected() {
        let mut scenario = direct_charge();
        scenario.charge_duration_seconds = 0;
        assert!(evaluate_architecture_capacity(&scenario, 10_000).is_err());
        let mut scenario = direct_charge();
        scenario.mission_duration_seconds = 0;
        assert!(evaluate_architecture_capacity(&scenario, 10_000).is_err());
    }

    #[test]
    fn pool_exceeding_declared_storage_capacity_is_rejected() {
        let mut scenario = direct_charge();
        scenario.battery_pack_count = 8;
        scenario.initial_ready_spares = 7;
        scenario.max_stored_spares_per_aircraft = 3;
        assert!(evaluate_architecture_capacity(&scenario, 10_000).is_err());
    }

    #[test]
    fn duplicate_architecture_ids_are_rejected() {
        let mut input = fixture();
        input.architectures[1].architecture_id = input.architectures[0].architecture_id.clone();
        let errors = evaluate_sustainable_service_capacity(&input).expect_err("duplicate");
        assert!(errors.iter().any(|error| error.starts_with("architecture_id_duplicate")));
    }

    #[test]
    fn no_more_chargers_than_grid_power_is_possible() {
        let mut scenario = direct_charge();
        scenario.parallel_charging_channels = 5;
        scenario.site_grid_import_limit_w = 1_250;
        scenario.site_fixed_load_w = 250;
        scenario.charger_input_power_w = 500;
        let result = evaluate_architecture_capacity(&scenario, 10_000).expect("two channels");
        assert_eq!(result.effective_charging_channels, 2);
        assert_eq!(result.modelled_charging_power_w, 1_000);
    }

    #[test]
    fn too_many_simulation_events_fails_closed() {
        let mut scenario = direct_charge();
        scenario.mission_duration_seconds = 1;
        scenario.charge_duration_seconds = 1;
        scenario.operational_turnaround_seconds = 0;
        let result = evaluate_architecture_capacity(&scenario, MAX_HORIZON_SECONDS);
        assert!(result.is_err());
    }

    #[test]
    fn sorties_never_exceed_initial_packs_plus_completed_recharges() {
        let input = fixture();
        for scenario in &input.architectures {
            let result = evaluate_architecture_capacity(scenario, input.horizon_seconds)
                .expect("scenario");
            assert!(
                result.completed_sorties
                    <= u64::from(scenario.battery_pack_count) + result.charge_completions,
                "pack reuse bound failed for {}",
                scenario.architecture_id
            );
            assert_eq!(
                result.completed_mission_hours,
                result.completed_sorties as f64
                    * f64::from(scenario.mission_duration_seconds)
                    / 3_600.0
            );
            assert!(result.effective_charging_channels <= scenario.parallel_charging_channels);
        }
    }

    #[test]
    fn input_architecture_order_does_not_change_canonical_report() {
        let input = fixture();
        let mut permuted = fixture();
        permuted.architectures.reverse();
        let first = evaluate_sustainable_service_capacity(&input).expect("first");
        let second = evaluate_sustainable_service_capacity(&permuted).expect("permuted");
        assert_eq!(
            serde_json::to_string(&first).expect("serialize first"),
            serde_json::to_string(&second).expect("serialize second")
        );
    }

    #[test]
    fn no_replenishment_case_matches_closed_form_mission_completion_bound() {
        let mut scenario = direct_charge();
        scenario.aircraft_count = 1;
        scenario.battery_pack_count = 5;
        scenario.initial_ready_spares = 4;
        scenario.max_stored_spares_per_aircraft = 4;
        scenario.mission_duration_seconds = 300;
        scenario.operational_turnaround_seconds = 45;
        scenario.parallel_charging_channels = 0;
        let horizon = 1_800_u64;
        let result = evaluate_architecture_capacity(&scenario, horizon).expect("bounded profile");
        let analytical = 1 + ((horizon - 300) / (300 + 45));
        assert_eq!(result.completed_sorties, analytical);
        assert_eq!(result.completed_sorties, 5);
    }

    #[test]
    fn fixed_site_load_is_included_separately_from_charging_energy() {
        let input = fixture();
        let result = evaluate_architecture_capacity(&input.architectures[1], input.horizon_seconds)
            .expect("dock profile");
        assert_eq!(
            result.site_fixed_load_energy_wh,
            input.architectures[1].site_fixed_load_w as f64
                * input.horizon_seconds as f64
                / 3_600.0
        );
        assert!(result.charger_input_energy_proxy_wh > 0.0);
    }

    #[test]
    fn battery_pool_conservation_holds_at_all_reported_end_states() {
        let input = fixture();
        for scenario in &input.architectures {
            let result = evaluate_architecture_capacity(scenario, input.horizon_seconds)
                .expect("scenario");
            let active_chargers = u64::from(result.active_chargers_at_horizon);
            let known_packs = u64::from(result.ready_spares_at_horizon)
                + result.charge_queue_at_horizon as u64
                + active_chargers
                + u64::from(scenario.aircraft_count)
                - result.waiting_aircraft_at_horizon as u64;
            assert_eq!(known_packs, u64::from(scenario.battery_pack_count));
        }
    }
}
