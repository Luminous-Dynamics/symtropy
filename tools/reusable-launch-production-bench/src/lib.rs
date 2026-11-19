use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetState {
    Planned,
    InProduction,
    Inspected,
    AcceptanceTested,
    Ready,
    Flown,
    Recovered,
    Inspection,
    Refurbishment,
    Requalified,
    Quarantined,
    Failed,
    Lost,
    Retired,
    NeedsEvidence,
}

impl AssetState {
    fn permits(self, next: Self) -> bool {
        use AssetState::*;
        matches!(
            (self, next),
            (Planned, InProduction)
                | (InProduction, Inspected)
                | (Inspected, AcceptanceTested)
                | (Inspected, Quarantined)
                | (AcceptanceTested, Ready)
                | (AcceptanceTested, Quarantined)
                | (Ready, Flown)
                | (Flown, Recovered)
                | (Flown, Failed)
                | (Flown, Lost)
                | (Recovered, Inspection)
                | (Inspection, Refurbishment)
                | (Inspection, Quarantined)
                | (Inspection, Retired)
                | (Refurbishment, Requalified)
                | (Refurbishment, Quarantined)
                | (Requalified, Ready)
                | (Requalified, Retired)
                | (Quarantined, Retired)
                | (Quarantined, NeedsEvidence)
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateEvent {
    pub event_id: String,
    pub from_state: AssetState,
    pub to_state: AssetState,
    pub tick: u64,
    #[serde(default)]
    pub flight_id: Option<String>,
    pub evidence_ref: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub serial: String,
    pub configuration: String,
    pub initial_state: AssetState,
    pub events: Vec<StateEvent>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlightRecord {
    pub flight_id: String,
    pub stage_serial: String,
    pub mission_success: bool,
    pub recovered: bool,
    pub payload_accepted: bool,
    pub accepted_payload_kg: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationCapacity {
    pub station: String,
    pub parallel_slots: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceTask {
    pub task_id: String,
    pub asset_serial: String,
    pub station: String,
    pub release_tick: u64,
    pub duration_ticks: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostModel {
    pub manufacture_scu_per_asset: u64,
    pub launch_operations_scu_per_attempt: u64,
    pub recovery_scu_per_recovered_asset: u64,
    pub inspection_scu_per_task: u64,
    pub refurbishment_scu_per_task: u64,
    pub loss_scu_per_lost_asset: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub suite_id: String,
    pub suite_version: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub assets: Vec<Asset>,
    pub flights: Vec<FlightRecord>,
    pub station_capacities: Vec<StationCapacity>,
    pub maintenance_tasks: Vec<MaintenanceTask>,
    pub cost_model: CostModel,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ScheduledTask {
    pub task_id: String,
    pub asset_serial: String,
    pub station: String,
    pub slot: u64,
    pub release_tick: u64,
    pub start_tick: u64,
    pub finish_tick: u64,
    pub wait_ticks: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct AssetSummary {
    pub serial: String,
    pub final_state: AssetState,
    pub flight_cycles: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CostBreakdown {
    pub manufacturing_scu: u64,
    pub launch_operations_scu: u64,
    pub recovery_scu: u64,
    pub inspection_scu: u64,
    pub refurbishment_scu: u64,
    pub loss_scu: u64,
    pub total_lifecycle_scu: u64,
    pub cost_per_accepted_kg_microscu: Option<u64>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Metrics {
    pub asset_count: u64,
    pub flight_attempts: u64,
    pub mission_successes: u64,
    pub recovered_stages: u64,
    pub failed_flights: u64,
    pub lost_stages: u64,
    pub payload_acceptances: u64,
    pub accepted_payload_kg: u64,
    pub completed_reflight_cycles: u64,
    pub turnaround_sample_count: u64,
    pub turnaround_total_ticks: u64,
    pub turnaround_max_ticks: u64,
    pub maintenance_task_count: u64,
    pub maintenance_total_wait_ticks: u64,
    pub maintenance_max_wait_ticks: u64,
    pub costs: CostBreakdown,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Report {
    pub suite_id: String,
    pub suite_version: String,
    pub scenario_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub result: String,
    pub assets: Vec<AssetSummary>,
    pub scheduled_tasks: Vec<ScheduledTask>,
    pub metrics: Metrics,
}

fn checked_add(a: u64, b: u64, label: &str) -> Result<u64, String> {
    a.checked_add(b)
        .ok_or_else(|| format!("{label}: integer overflow"))
}

fn checked_mul(a: u64, b: u64, label: &str) -> Result<u64, String> {
    a.checked_mul(b)
        .ok_or_else(|| format!("{label}: integer overflow"))
}

fn non_empty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label}: must not be empty"))
    } else {
        Ok(())
    }
}

fn schedule_tasks(
    fixture: &Fixture,
    asset_ids: &BTreeSet<String>,
) -> Result<Vec<ScheduledTask>, String> {
    let mut capacities = BTreeMap::<String, usize>::new();
    for capacity in &fixture.station_capacities {
        non_empty(&capacity.station, "station")?;
        if capacity.parallel_slots == 0 {
            return Err(format!("station {} has zero parallel slots", capacity.station));
        }
        let slots = usize::try_from(capacity.parallel_slots)
            .map_err(|_| {
                format!(
                    "station {} slot count exceeds platform limits",
                    capacity.station
                )
            })?;
        if slots > 64 {
            return Err(format!(
                "station {} exceeds the v0.1 limit of 64 parallel slots",
                capacity.station
            ));
        }
        if capacities.insert(capacity.station.clone(), slots).is_some() {
            return Err(format!("duplicate station capacity: {}", capacity.station));
        }
    }

    let mut seen_tasks = BTreeSet::new();
    let mut tasks = fixture.maintenance_tasks.iter().collect::<Vec<_>>();
    for task in &tasks {
        non_empty(&task.task_id, "task_id")?;
        non_empty(&task.asset_serial, "task asset_serial")?;
        if !asset_ids.contains(&task.asset_serial) {
            return Err(format!(
                "task {} references unknown asset {}",
                task.task_id, task.asset_serial
            ));
        }
        if !seen_tasks.insert(task.task_id.as_str()) {
            return Err(format!("duplicate maintenance task: {}", task.task_id));
        }
        if task.duration_ticks == 0 {
            return Err(format!("task {} has zero duration", task.task_id));
        }
        if !capacities.contains_key(&task.station) {
            return Err(format!(
                "task {} references station without capacity: {}",
                task.task_id, task.station
            ));
        }
    }

    tasks.sort_by_key(|task| (&task.station, task.release_tick, &task.task_id));

    let mut availability = capacities
        .iter()
        .map(|(station, slots)| (station.clone(), vec![0_u64; *slots]))
        .collect::<BTreeMap<_, _>>();
    let mut schedule = Vec::with_capacity(tasks.len());

    for task in tasks {
        let slots = availability
            .get_mut(&task.station)
            .ok_or_else(|| format!("missing station schedule: {}", task.station))?;
        let (slot_index, available_tick) = slots
            .iter()
            .copied()
            .enumerate()
            .min_by_key(|(index, tick)| (*tick, *index))
            .ok_or_else(|| format!("station {} has no capacity slots", task.station))?;
        let start_tick = task.release_tick.max(available_tick);
        let finish_tick = checked_add(start_tick, task.duration_ticks, "maintenance finish tick")?;
        let wait_ticks = start_tick - task.release_tick;
        slots[slot_index] = finish_tick;
        schedule.push(ScheduledTask {
            task_id: task.task_id.clone(),
            asset_serial: task.asset_serial.clone(),
            station: task.station.clone(),
            slot: slot_index as u64,
            release_tick: task.release_tick,
            start_tick,
            finish_tick,
            wait_ticks,
        });
    }

    schedule.sort_by_key(|task| {
        (
            &task.station,
            task.start_tick,
            task.slot,
            &task.task_id,
        )
    });
    Ok(schedule)
}

pub fn evaluate(fixture: &Fixture) -> Result<Report, String> {
    if fixture.suite_id != "reusable-launch-production" || fixture.suite_version != "0.1.0" {
        return Err("unsupported suite identity or version".to_owned());
    }
    if fixture.evidence_class != "synthetic" {
        return Err("v0.1 fixture must be explicitly classified as synthetic".to_owned());
    }
    non_empty(&fixture.claim_ceiling, "claim_ceiling")?;
    if fixture.assets.is_empty() || fixture.flights.is_empty() {
        return Err("fixture must contain at least one asset and one flight".to_owned());
    }

    let mut asset_ids = BTreeSet::new();
    let mut global_event_ids = BTreeSet::new();
    let mut flown_by = BTreeMap::<String, String>::new();
    let mut recovered_ids = BTreeSet::new();
    let mut terminal_failure_ids = BTreeSet::new();
    let mut failed_ids = BTreeSet::new();
    let mut lost_ids = BTreeSet::new();
    let mut turnaround_ticks = Vec::<u64>::new();
    let mut asset_summaries = Vec::new();

    for asset in &fixture.assets {
        non_empty(&asset.serial, "asset serial")?;
        non_empty(&asset.configuration, "asset configuration")?;
        if !asset_ids.insert(asset.serial.clone()) {
            return Err(format!("duplicate serialized asset: {}", asset.serial));
        }

        let mut current_state = asset.initial_state;
        let mut last_tick = 0_u64;
        let mut active_flight: Option<String> = None;
        let mut last_recovery_tick: Option<u64> = None;
        let mut cycles = 0_u64;

        for event in &asset.events {
            non_empty(&event.event_id, "event_id")?;
            non_empty(&event.evidence_ref, "event evidence_ref")?;
            if !global_event_ids.insert(event.event_id.as_str()) {
                return Err(format!("duplicate event ID: {}", event.event_id));
            }
            if event.tick < last_tick {
                return Err(format!("asset {} has non-monotonic event ticks", asset.serial));
            }
            if event.from_state != current_state {
                return Err(format!(
                    "asset {} event {} has stale from_state {:?}; current state is {:?}",
                    asset.serial, event.event_id, event.from_state, current_state
                ));
            }
            if !event.from_state.permits(event.to_state) {
                return Err(format!(
                    "asset {} has forbidden transition {:?} -> {:?}",
                    asset.serial, event.from_state, event.to_state
                ));
            }

            let is_flight_transition = matches!(
                event.to_state,
                AssetState::Flown | AssetState::Recovered | AssetState::Failed | AssetState::Lost
            );
            if is_flight_transition != event.flight_id.is_some() {
                return Err(format!(
                    "asset {} event {} must carry flight_id only for flight-state transitions",
                    asset.serial, event.event_id
                ));
            }

            match event.to_state {
                AssetState::Flown => {
                    let flight_id = event.flight_id.as_ref().expect("checked above");
                    non_empty(flight_id, "flight_id")?;
                    if active_flight.is_some()
                        || flown_by
                            .insert(flight_id.clone(), asset.serial.clone())
                            .is_some()
                    {
                        return Err(format!("duplicate or overlapping flight ID: {flight_id}"));
                    }
                    if cycles > 0 {
                        let recovered_tick = last_recovery_tick.take().ok_or_else(|| {
                            format!(
                                "asset {} attempted a reflight without a recorded recovery",
                                asset.serial
                            )
                        })?;
                        turnaround_ticks.push(event.tick - recovered_tick);
                    }
                    active_flight = Some(flight_id.clone());
                    cycles = checked_add(cycles, 1, "flight cycles")?;
                }
                AssetState::Recovered => {
                    let flight_id = event.flight_id.as_ref().expect("checked above");
                    if active_flight.as_ref() != Some(flight_id) {
                        return Err(format!("recovery references non-active flight {flight_id}"));
                    }
                    if !recovered_ids.insert(flight_id.clone()) {
                        return Err(format!("duplicate recovery event for flight {flight_id}"));
                    }
                    active_flight = None;
                    last_recovery_tick = Some(event.tick);
                }
                AssetState::Failed | AssetState::Lost => {
                    let flight_id = event.flight_id.as_ref().expect("checked above");
                    if active_flight.as_ref() != Some(flight_id) {
                        return Err(format!(
                            "terminal event references non-active flight {flight_id}"
                        ));
                    }
                    if !terminal_failure_ids.insert(flight_id.clone()) {
                        return Err(format!("duplicate terminal failure for flight {flight_id}"));
                    }
                    match event.to_state {
                        AssetState::Failed => {
                            failed_ids.insert(flight_id.clone());
                        }
                        AssetState::Lost => {
                            lost_ids.insert(flight_id.clone());
                        }
                        _ => unreachable!("matched only failed or lost states"),
                    }
                    active_flight = None;
                    last_recovery_tick = None;
                }
                _ => {}
            }

            current_state = event.to_state;
            last_tick = event.tick;
        }

        asset_summaries.push(AssetSummary {
            serial: asset.serial.clone(),
            final_state: current_state,
            flight_cycles: cycles,
        });
    }

    let mut seen_flights = BTreeSet::new();
    let mut mission_successes = 0_u64;
    let mut recovered_count = 0_u64;
    let mut failed_count = 0_u64;
    let mut lost_count = 0_u64;
    let mut accepted_count = 0_u64;
    let mut accepted_payload_kg = 0_u64;

    for flight in &fixture.flights {
        non_empty(&flight.flight_id, "flight_id")?;
        if !seen_flights.insert(flight.flight_id.as_str()) {
            return Err(format!("duplicate flight record: {}", flight.flight_id));
        }
        if !asset_ids.contains(&flight.stage_serial) {
            return Err(format!("flight {} references unknown stage", flight.flight_id));
        }
        if flown_by.get(&flight.flight_id) != Some(&flight.stage_serial) {
            return Err(format!(
                "flight {} has no matching Flown event for its stage",
                flight.flight_id
            ));
        }
        if flight.payload_accepted
            && (!flight.mission_success || flight.accepted_payload_kg == 0)
        {
            return Err(format!(
                "flight {} accepts payload without mission success and positive mass",
                flight.flight_id
            ));
        }
        if !flight.payload_accepted && flight.accepted_payload_kg != 0 {
            return Err(format!(
                "flight {} has payload mass without payload acceptance",
                flight.flight_id
            ));
        }
        if flight.recovered != recovered_ids.contains(&flight.flight_id) {
            return Err(format!(
                "flight {} recovery record contradicts the asset event ledger",
                flight.flight_id
            ));
        }
        if flight.recovered == terminal_failure_ids.contains(&flight.flight_id) {
            return Err(format!(
                "flight {} must have exactly one recovery or terminal-failure outcome",
                flight.flight_id
            ));
        }
        if flight.mission_success {
            mission_successes = checked_add(mission_successes, 1, "mission successes")?;
        }
        if flight.recovered {
            recovered_count = checked_add(recovered_count, 1, "recovered stages")?;
        } else if failed_ids.contains(&flight.flight_id) {
            failed_count = checked_add(failed_count, 1, "failed flights")?;
        }
        if lost_ids.contains(&flight.flight_id) {
            lost_count = checked_add(lost_count, 1, "lost stages")?;
        }
        if flight.payload_accepted {
            accepted_count = checked_add(accepted_count, 1, "payload acceptances")?;
            accepted_payload_kg = checked_add(
                accepted_payload_kg,
                flight.accepted_payload_kg,
                "accepted payload mass",
            )?;
        }
    }

    if seen_flights != flown_by.keys().map(String::as_str).collect::<BTreeSet<_>>() {
        return Err("flight records and Flown events are not a one-to-one match".to_owned());
    }
    if recovered_ids.iter().any(|id| !seen_flights.contains(id.as_str()))
        || terminal_failure_ids.iter().any(|id| !seen_flights.contains(id.as_str()))
        || failed_ids.iter().any(|id| !seen_flights.contains(id.as_str()))
        || lost_ids.iter().any(|id| !seen_flights.contains(id.as_str()))
    {
        return Err("orphan recovery or terminal-failure event".to_owned());
    }

    let scheduled_tasks = schedule_tasks(fixture, &asset_ids)?;
    let inspection_tasks = scheduled_tasks
        .iter()
        .filter(|task| task.station == "inspection")
        .count() as u64;
    let refurbishment_tasks = scheduled_tasks
        .iter()
        .filter(|task| task.station == "refurbishment")
        .count() as u64;
    if scheduled_tasks
        .iter()
        .any(|task| task.station != "inspection" && task.station != "refurbishment")
    {
        return Err("v0.1 supports only inspection and refurbishment task classes".to_owned());
    }

    let costs = &fixture.cost_model;
    let manufacturing_scu = checked_mul(
        fixture.assets.len() as u64,
        costs.manufacture_scu_per_asset,
        "manufacturing cost",
    )?;
    let launch_operations_scu = checked_mul(
        fixture.flights.len() as u64,
        costs.launch_operations_scu_per_attempt,
        "launch operations cost",
    )?;
    let recovery_scu = checked_mul(
        recovered_count,
        costs.recovery_scu_per_recovered_asset,
        "recovery cost",
    )?;
    let inspection_scu = checked_mul(
        inspection_tasks,
        costs.inspection_scu_per_task,
        "inspection cost",
    )?;
    let refurbishment_scu = checked_mul(
        refurbishment_tasks,
        costs.refurbishment_scu_per_task,
        "refurbishment cost",
    )?;
    let loss_scu = checked_mul(lost_count, costs.loss_scu_per_lost_asset, "loss cost")?;
    let total_lifecycle_scu = [
        manufacturing_scu,
        launch_operations_scu,
        recovery_scu,
        inspection_scu,
        refurbishment_scu,
        loss_scu,
    ]
    .into_iter()
    .try_fold(0_u64, |sum, value| checked_add(sum, value, "total lifecycle cost"))?;
    let cost_per_accepted_kg_microscu = if accepted_payload_kg == 0 {
        None
    } else {
        Some(
            checked_mul(total_lifecycle_scu, 1_000_000, "cost per accepted kilogram")?
                / accepted_payload_kg,
        )
    };

    let costs = CostBreakdown {
        manufacturing_scu,
        launch_operations_scu,
        recovery_scu,
        inspection_scu,
        refurbishment_scu,
        loss_scu,
        total_lifecycle_scu,
        cost_per_accepted_kg_microscu,
    };

    let maintenance_total_wait_ticks = scheduled_tasks.iter().try_fold(0_u64, |sum, task| {
        checked_add(sum, task.wait_ticks, "maintenance wait")
    })?;
    let maintenance_max_wait_ticks = scheduled_tasks
        .iter()
        .map(|task| task.wait_ticks)
        .max()
        .unwrap_or(0);
    let turnaround_total_ticks = turnaround_ticks.iter().try_fold(0_u64, |sum, ticks| {
        checked_add(sum, *ticks, "turnaround total")
    })?;
    let turnaround_max_ticks = turnaround_ticks.iter().copied().max().unwrap_or(0);

    asset_summaries.sort_by(|a, b| a.serial.cmp(&b.serial));

    Ok(Report {
        suite_id: fixture.suite_id.clone(),
        suite_version: fixture.suite_version.clone(),
        scenario_id: "FLEET_LIFECYCLE_V0_1".to_owned(),
        evidence_class: fixture.evidence_class.clone(),
        claim_ceiling: fixture.claim_ceiling.clone(),
        result: "pass".to_owned(),
        assets: asset_summaries,
        scheduled_tasks,
        metrics: Metrics {
            asset_count: fixture.assets.len() as u64,
            flight_attempts: fixture.flights.len() as u64,
            mission_successes,
            recovered_stages: recovered_count,
            failed_flights: failed_count,
            lost_stages: lost_count,
            payload_acceptances: accepted_count,
            accepted_payload_kg,
            completed_reflight_cycles: turnaround_ticks.len() as u64,
            turnaround_sample_count: turnaround_ticks.len() as u64,
            turnaround_total_ticks,
            turnaround_max_ticks,
            maintenance_task_count: fixture.maintenance_tasks.len() as u64,
            maintenance_total_wait_ticks,
            maintenance_max_wait_ticks,
            costs,
        },
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(bytes);
    let mut hexadecimal = String::with_capacity(64);
    for byte in digest {
        write!(&mut hexadecimal, "{byte:02x}").expect("writing to String is infallible");
    }
    hexadecimal
}

pub fn is_valid_source_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!("../fixtures/v0.1/reference.json"))
            .expect("frozen synthetic fixture must parse")
    }

    #[test]
    fn frozen_fixture_replays_expected_accounting() {
        let report = evaluate(&fixture()).expect("fixture should pass");
        assert_eq!(report.metrics.asset_count, 3);
        assert_eq!(report.metrics.flight_attempts, 4);
        assert_eq!(report.metrics.mission_successes, 3);
        assert_eq!(report.metrics.recovered_stages, 3);
        assert_eq!(report.metrics.failed_flights, 0);
        assert_eq!(report.metrics.lost_stages, 1);
        assert_eq!(report.metrics.payload_acceptances, 3);
        assert_eq!(report.metrics.accepted_payload_kg, 23_000);
        assert_eq!(report.metrics.completed_reflight_cycles, 1);
        assert_eq!(report.metrics.turnaround_sample_count, 1);
        assert_eq!(report.metrics.turnaround_total_ticks, 14);
        assert_eq!(report.metrics.turnaround_max_ticks, 14);
        assert_eq!(report.metrics.maintenance_task_count, 5);
        assert_eq!(report.metrics.maintenance_total_wait_ticks, 1);
        assert_eq!(report.metrics.maintenance_max_wait_ticks, 1);
        assert_eq!(report.metrics.costs.total_lifecycle_scu, 482);
        assert_eq!(report.metrics.costs.cost_per_accepted_kg_microscu, Some(20_956));
    }

    #[test]
    fn inspection_failure_quarantines_asset_without_counting_reflight() {
        let report = evaluate(&fixture()).expect("fixture should pass");
        let asset = report.assets.iter().find(|asset| asset.serial == "STAGE-002").unwrap();
        assert_eq!(asset.final_state, AssetState::Quarantined);
        assert_eq!(asset.flight_cycles, 1);
        assert_eq!(report.metrics.completed_reflight_cycles, 1);
        let reusable = report
            .assets
            .iter()
            .find(|asset| asset.serial == "STAGE-001")
            .unwrap();
        assert_eq!(reusable.flight_cycles, 2);
        assert_eq!(reusable.final_state, AssetState::Ready);
    }

    #[test]
    fn forbidden_state_transition_is_rejected() {
        let mut data = fixture();
        data.assets[0].events[0].to_state = AssetState::Requalified;
        assert!(evaluate(&data).unwrap_err().contains("forbidden transition"));
    }

    #[test]
    fn duplicate_event_identity_is_rejected() {
        let mut data = fixture();
        data.assets[1].events[0].event_id = data.assets[0].events[0].event_id.clone();
        assert!(evaluate(&data).unwrap_err().contains("duplicate event ID"));
    }

    #[test]
    fn payload_acceptance_without_mission_success_is_rejected() {
        let mut data = fixture();
        data.flights[0].mission_success = false;
        assert!(evaluate(&data).unwrap_err().contains("accepts payload"));
    }

    #[test]
    fn recovery_record_must_match_state_event() {
        let mut data = fixture();
        data.flights[0].recovered = false;
        assert!(evaluate(&data).unwrap_err().contains("recovery record contradicts"));
    }

    #[test]
    fn failed_vehicle_is_not_counted_as_a_lost_stage() {
        let mut data = fixture();
        data.assets[2].events[5].to_state = AssetState::Failed;
        let report = evaluate(&data).expect("failed-flight fixture should pass");
        assert_eq!(report.metrics.failed_flights, 1);
        assert_eq!(report.metrics.lost_stages, 0);
        assert_eq!(report.metrics.costs.loss_scu, 0);
    }

    #[test]
    fn missing_station_capacity_is_rejected() {
        let mut data = fixture();
        data.station_capacities.pop();
        assert!(evaluate(&data).unwrap_err().contains("without capacity"));
    }

    #[test]
    fn zero_duration_task_is_rejected() {
        let mut data = fixture();
        data.maintenance_tasks[0].duration_ticks = 0;
        assert!(evaluate(&data).unwrap_err().contains("zero duration"));
    }

    #[test]
    fn duplicate_serialized_asset_is_rejected() {
        let mut data = fixture();
        data.assets[1].serial = data.assets[0].serial.clone();
        assert!(evaluate(&data).unwrap_err().contains("duplicate serialized asset"));
    }

    #[test]
    fn task_order_is_deterministic_and_respects_single_slot_capacity() {
        let report = evaluate(&fixture()).expect("fixture should pass");
        let inspection = report
            .scheduled_tasks
            .iter()
            .filter(|task| task.station == "inspection")
            .collect::<Vec<_>>();
        assert_eq!(inspection.len(), 3);
        assert_eq!(inspection[0].task_id, "inspect-stage-001");
        assert_eq!(inspection[0].start_tick, 12);
        assert_eq!(inspection[0].finish_tick, 15);
        assert_eq!(inspection[1].task_id, "inspect-stage-002");
        assert_eq!(inspection[1].start_tick, 15);
        assert_eq!(inspection[1].finish_tick, 17);
        assert_eq!(inspection[2].task_id, "inspect-stage-001-second");
        assert_eq!(inspection[2].start_tick, 27);
        assert_eq!(inspection[2].finish_tick, 30);
    }

    #[test]
    fn fixture_is_explicitly_synthetic() {
        let mut data = fixture();
        data.evidence_class = "measured".to_owned();
        assert!(evaluate(&data).unwrap_err().contains("explicitly classified as synthetic"));
    }

    #[test]
    fn source_commit_validator_rejects_non_sha_values() {
        assert!(is_valid_source_commit("0123456789abcdef0123456789abcdef01234567"));
        assert!(!is_valid_source_commit("main"));
        assert!(!is_valid_source_commit("0123456789abcdef0123456789abcdef0123456z"));
    }
}
