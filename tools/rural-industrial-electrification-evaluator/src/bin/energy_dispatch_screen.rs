use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process};

const CLAIM_CEILING: &str = "Deterministic interval dispatch screen with synthetic/reference inputs only; not a grid power-flow study, battery electrochemistry model, industrial safety assessment, or deployment authorization.";

#[derive(Debug, Clone, Deserialize)]
struct Benchmark {
    schema_version: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
struct Scenario {
    scenario_id: String,
    label: String,
    evidence_class: String,
    storage: StorageConfig,
    slots: Vec<Slot>,
}

#[derive(Debug, Clone, Deserialize)]
struct StorageConfig {
    capacity_kwh: f64,
    initial_soc_kwh: f64,
    minimum_soc_kwh: f64,
    maximum_charge_power_kw: f64,
    maximum_discharge_power_kw: f64,
    charge_efficiency_fraction: f64,
    discharge_efficiency_fraction: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct Slot {
    slot_id: String,
    duration_hours: f64,
    onsite_generation_available_kw: f64,
    grid_import_limit_kw: f64,
    critical_load_requested_kw: f64,
    flexible_load_requested_kw: f64,
    allow_storage_for_flexible_load: bool,
    allow_grid_charging: bool,
    grid_price_per_kwh: Option<f64>,
    grid_carbon_kg_per_kwh: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Status {
    AllDeclaredLoadsServed,
    FlexibleLoadShortfall,
    CriticalLoadShortfall,
    CriticalAndFlexibleShortfall,
}

#[derive(Debug, Clone, Serialize)]
struct SlotResult {
    slot_id: String,
    duration_hours: f64,
    onsite_generation_available_kwh: f64,
    grid_import_kwh: f64,
    critical_requested_kwh: f64,
    critical_served_kwh: f64,
    critical_unserved_kwh: f64,
    flexible_requested_kwh: f64,
    flexible_served_kwh: f64,
    flexible_unserved_kwh: f64,
    storage_charge_input_kwh: f64,
    storage_charge_from_generation_kwh: f64,
    storage_charge_from_grid_kwh: f64,
    storage_discharge_to_critical_kwh: f64,
    storage_discharge_to_flexible_kwh: f64,
    storage_charge_loss_kwh: f64,
    storage_discharge_loss_kwh: f64,
    curtailed_generation_kwh: f64,
    storage_soc_start_kwh: f64,
    storage_soc_end_kwh: f64,
    energy_balance_residual_kwh: f64,
    grid_cost: Option<f64>,
    grid_emissions_kg: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioResult {
    scenario_id: String,
    label: String,
    evidence_class: String,
    status: Status,
    total_onsite_generation_available_kwh: f64,
    total_grid_import_kwh: f64,
    total_critical_requested_kwh: f64,
    total_critical_served_kwh: f64,
    total_critical_unserved_kwh: f64,
    total_flexible_requested_kwh: f64,
    total_flexible_served_kwh: f64,
    total_flexible_unserved_kwh: f64,
    total_storage_charge_input_kwh: f64,
    total_storage_discharge_output_kwh: f64,
    total_storage_conversion_loss_kwh: f64,
    total_curtailed_generation_kwh: f64,
    final_storage_soc_kwh: f64,
    estimated_grid_cost: Option<f64>,
    estimated_grid_emissions_kg: Option<f64>,
    maximum_absolute_energy_balance_residual_kwh: f64,
    slots: Vec<SlotResult>,
    findings: Vec<String>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    method: &'static str,
    results: Vec<ScenarioResult>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct BatteryFlow {
    delivered_kwh: f64,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/rural-industrial-electrification-evaluator/fixtures/dispatch_profiles_v0_1.json"
            .to_string()
    });
    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };
    let benchmark: Benchmark = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid dispatch JSON: {error}");
            process::exit(2);
        }
    };
    if benchmark.schema_version != "0.1.0" || benchmark.scenarios.is_empty() {
        eprintln!("unsupported schema version or empty scenario set");
        process::exit(2);
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in &benchmark.scenarios {
        if !scenario_ids.insert(scenario.scenario_id.clone()) {
            eprintln!("duplicate scenario_id: {}", scenario.scenario_id);
            process::exit(2);
        }
        if let Some(error) = validate(scenario) {
            eprintln!("invalid scenario {}: {error}", scenario.scenario_id);
            process::exit(2);
        }
    }

    let mut results: Vec<ScenarioResult> = benchmark.scenarios.iter().map(simulate).collect();
    results.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
    let report = Report {
        schema_version: benchmark.schema_version,
        method: "generation-first, grid-capped, critical-load-priority interval dispatch with explicit battery state",
        results,
        claim_ceiling: CLAIM_CEILING,
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize report: {error}");
            process::exit(2);
        }
    }
}

fn validate(scenario: &Scenario) -> Option<String> {
    if scenario.scenario_id.trim().is_empty() || scenario.label.trim().is_empty() {
        return Some("scenario_id_and_label_must_be_nonempty".to_string());
    }
    if ![
        "synthetic",
        "measured",
        "manufacturer",
        "literature",
        "estimate",
    ]
    .contains(&scenario.evidence_class.as_str())
    {
        return Some("unsupported_evidence_class".to_string());
    }

    let b = &scenario.storage;
    for (name, value) in [
        ("capacity_kwh", b.capacity_kwh),
        ("initial_soc_kwh", b.initial_soc_kwh),
        ("minimum_soc_kwh", b.minimum_soc_kwh),
        ("maximum_charge_power_kw", b.maximum_charge_power_kw),
        ("maximum_discharge_power_kw", b.maximum_discharge_power_kw),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Some(format!("{name}_must_be_finite_and_nonnegative"));
        }
    }
    if b.capacity_kwh <= 0.0 {
        return Some("capacity_kwh_must_be_positive".to_string());
    }
    if b.minimum_soc_kwh > b.initial_soc_kwh || b.initial_soc_kwh > b.capacity_kwh {
        return Some("initial_soc_must_be_within_minimum_and_capacity".to_string());
    }
    if b.minimum_soc_kwh >= b.capacity_kwh {
        return Some("minimum_soc_must_be_below_capacity".to_string());
    }
    for (name, value) in [
        ("charge_efficiency_fraction", b.charge_efficiency_fraction),
        (
            "discharge_efficiency_fraction",
            b.discharge_efficiency_fraction,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 || value > 1.0 {
            return Some(format!("{name}_must_be_in_(0,1]"));
        }
    }
    if scenario.slots.is_empty() {
        return Some("scenario_must_contain_at_least_one_slot".to_string());
    }

    let mut slot_ids = BTreeSet::new();
    for slot in &scenario.slots {
        if slot.slot_id.trim().is_empty() || !slot_ids.insert(slot.slot_id.clone()) {
            return Some("slot_ids_must_be_nonempty_and_unique_within_scenario".to_string());
        }
        if !slot.duration_hours.is_finite() || slot.duration_hours <= 0.0 {
            return Some("slot_duration_must_be_finite_and_positive".to_string());
        }
        for (name, value) in [
            (
                "onsite_generation_available_kw",
                slot.onsite_generation_available_kw,
            ),
            ("grid_import_limit_kw", slot.grid_import_limit_kw),
            (
                "critical_load_requested_kw",
                slot.critical_load_requested_kw,
            ),
            (
                "flexible_load_requested_kw",
                slot.flexible_load_requested_kw,
            ),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Some(format!("{name}_must_be_finite_and_nonnegative"));
            }
        }
        if let Some(price) = slot.grid_price_per_kwh {
            if !price.is_finite() || price < 0.0 {
                return Some("grid_price_per_kwh_must_be_finite_and_nonnegative".to_string());
            }
        }
        if let Some(carbon) = slot.grid_carbon_kg_per_kwh {
            if !carbon.is_finite() || carbon < 0.0 {
                return Some("grid_carbon_kg_per_kwh_must_be_finite_and_nonnegative".to_string());
            }
        }
    }
    None
}

fn battery_discharge(
    requested_output_kwh: f64,
    duration_hours: f64,
    config: &StorageConfig,
    soc_kwh: &mut f64,
) -> BatteryFlow {
    if requested_output_kwh <= 0.0 || duration_hours <= 0.0 {
        return BatteryFlow { delivered_kwh: 0.0 };
    }
    let available_output_kwh =
        (*soc_kwh - config.minimum_soc_kwh).max(0.0) * config.discharge_efficiency_fraction;
    let delivered_kwh = requested_output_kwh
        .min(config.maximum_discharge_power_kw * duration_hours)
        .min(available_output_kwh);
    let soc_decrease_kwh = delivered_kwh / config.discharge_efficiency_fraction;
    *soc_kwh = (*soc_kwh - soc_decrease_kwh).max(config.minimum_soc_kwh);
    BatteryFlow { delivered_kwh }
}

fn battery_charge(
    requested_input_kwh: f64,
    duration_hours: f64,
    config: &StorageConfig,
    soc_kwh: &mut f64,
) -> f64 {
    if requested_input_kwh <= 0.0 || duration_hours <= 0.0 {
        return 0.0;
    }
    let input_power_limit_kwh = config.maximum_charge_power_kw * duration_hours;
    let room_input_kwh =
        (config.capacity_kwh - *soc_kwh).max(0.0) / config.charge_efficiency_fraction;
    let input_kwh = requested_input_kwh
        .min(input_power_limit_kwh)
        .min(room_input_kwh);
    *soc_kwh = (*soc_kwh + input_kwh * config.charge_efficiency_fraction).min(config.capacity_kwh);
    input_kwh
}

fn simulate(scenario: &Scenario) -> ScenarioResult {
    let b = &scenario.storage;
    let mut soc = b.initial_soc_kwh;
    let mut rows = Vec::with_capacity(scenario.slots.len());

    for slot in &scenario.slots {
        let dt = slot.duration_hours;
        let gen_available = slot.onsite_generation_available_kw;
        let mut gen_left = gen_available;
        let mut grid_left = slot.grid_import_limit_kw;
        let soc_start = soc;

        // Critical loads have the first claim on local generation and grid import.
        let critical_from_generation = gen_left.min(slot.critical_load_requested_kw);
        gen_left -= critical_from_generation;
        let critical_after_generation = slot.critical_load_requested_kw - critical_from_generation;
        let critical_from_grid = grid_left.min(critical_after_generation);
        grid_left -= critical_from_grid;
        let critical_shortage_after_grid = critical_after_generation - critical_from_grid;
        let critical_battery =
            battery_discharge(critical_shortage_after_grid * dt, dt, b, &mut soc);
        let critical_served = critical_from_generation * dt
            + critical_from_grid * dt
            + critical_battery.delivered_kwh;
        let critical_requested = slot.critical_load_requested_kw * dt;
        let critical_unserved = (critical_requested - critical_served).max(0.0);

        let mut flexible_from_generation = 0.0;
        let mut flexible_from_grid = 0.0;
        let mut flexible_battery_kwh = 0.0;
        let mut charge_from_generation = 0.0;
        let mut charge_from_grid = 0.0;

        // Do not serve discretionary loads while declared critical demand is unmet.
        if critical_unserved <= 1e-9 {
            flexible_from_generation = gen_left.min(slot.flexible_load_requested_kw);
            gen_left -= flexible_from_generation;
            let flexible_remaining =
                (slot.flexible_load_requested_kw - flexible_from_generation).max(0.0);
            flexible_from_grid = grid_left.min(flexible_remaining);
            grid_left -= flexible_from_grid;
            let flexible_after_direct = (flexible_remaining - flexible_from_grid).max(0.0);

            if slot.allow_storage_for_flexible_load && flexible_after_direct > 0.0 {
                let flow = battery_discharge(flexible_after_direct * dt, dt, b, &mut soc);
                flexible_battery_kwh = flow.delivered_kwh;
            }

            // Only surplus generation is used to charge by default; grid charging is explicit.
            let generation_charge_request = gen_left.max(0.0);
            charge_from_generation =
                battery_charge(generation_charge_request * dt, dt, b, &mut soc) / dt;
            gen_left = (gen_left - charge_from_generation).max(0.0);

            if slot.allow_grid_charging {
                let grid_charge_request = grid_left.max(0.0);
                charge_from_grid = battery_charge(grid_charge_request * dt, dt, b, &mut soc) / dt;
                grid_left = (grid_left - charge_from_grid).max(0.0);
            }
        }

        let flexible_served =
            flexible_from_generation * dt + flexible_from_grid * dt + flexible_battery_kwh;
        let flexible_requested = slot.flexible_load_requested_kw * dt;
        let flexible_unserved = (flexible_requested - flexible_served).max(0.0);
        let grid_import_kwh = (slot.grid_import_limit_kw - grid_left).max(0.0) * dt;
        let generation_curtailed_kwh = gen_left.max(0.0) * dt;
        let charge_from_generation_kwh = charge_from_generation * dt;
        let charge_from_grid_kwh = charge_from_grid * dt;
        let charge_input_kwh = charge_from_generation_kwh + charge_from_grid_kwh;
        let discharge_critical_kwh = critical_battery.delivered_kwh;
        let discharge_output_kwh = discharge_critical_kwh + flexible_battery_kwh;
        let charge_loss_kwh = charge_input_kwh * (1.0 - b.charge_efficiency_fraction);
        let discharge_loss_kwh =
            discharge_output_kwh * (1.0 / b.discharge_efficiency_fraction - 1.0);
        let energy_balance_residual = gen_available * dt + grid_import_kwh + discharge_output_kwh
            - critical_served
            - flexible_served
            - charge_input_kwh
            - generation_curtailed_kwh;

        let grid_cost = slot.grid_price_per_kwh.map(|price| price * grid_import_kwh);
        let grid_emissions = slot
            .grid_carbon_kg_per_kwh
            .map(|factor| factor * grid_import_kwh);

        rows.push(SlotResult {
            slot_id: slot.slot_id.clone(),
            duration_hours: dt,
            onsite_generation_available_kwh: gen_available * dt,
            grid_import_kwh,
            critical_requested_kwh: critical_requested,
            critical_served_kwh: critical_served,
            critical_unserved_kwh: critical_unserved,
            flexible_requested_kwh: flexible_requested,
            flexible_served_kwh: flexible_served,
            flexible_unserved_kwh: flexible_unserved,
            storage_charge_input_kwh: charge_input_kwh,
            storage_charge_from_generation_kwh: charge_from_generation_kwh,
            storage_charge_from_grid_kwh: charge_from_grid_kwh,
            storage_discharge_to_critical_kwh: discharge_critical_kwh,
            storage_discharge_to_flexible_kwh: flexible_battery_kwh,
            storage_charge_loss_kwh: charge_loss_kwh,
            storage_discharge_loss_kwh: discharge_loss_kwh,
            curtailed_generation_kwh: generation_curtailed_kwh,
            storage_soc_start_kwh: soc_start,
            storage_soc_end_kwh: soc,
            energy_balance_residual_kwh: energy_balance_residual,
            grid_cost,
            grid_emissions_kg: grid_emissions,
        });
    }

    let total = |f: fn(&SlotResult) -> f64| rows.iter().map(f).sum::<f64>();
    let priced = rows.iter().all(|row| row.grid_cost.is_some());
    let carbon_complete = rows.iter().all(|row| row.grid_emissions_kg.is_some());
    let critical_unserved = total(|row| row.critical_unserved_kwh);
    let flexible_unserved = total(|row| row.flexible_unserved_kwh);
    let total_grid_import = total(|row| row.grid_import_kwh);
    let total_charge = total(|row| row.storage_charge_input_kwh);
    let total_discharge =
        total(|row| row.storage_discharge_to_critical_kwh + row.storage_discharge_to_flexible_kwh);
    let total_losses = total(|row| row.storage_charge_loss_kwh + row.storage_discharge_loss_kwh);
    let total_curtailed = total(|row| row.curtailed_generation_kwh);
    let max_residual = rows
        .iter()
        .map(|row| row.energy_balance_residual_kwh.abs())
        .fold(0.0, f64::max);
    drop(total);

    let status = match (critical_unserved > 1e-9, flexible_unserved > 1e-9) {
        (false, false) => Status::AllDeclaredLoadsServed,
        (false, true) => Status::FlexibleLoadShortfall,
        (true, false) => Status::CriticalLoadShortfall,
        (true, true) => Status::CriticalAndFlexibleShortfall,
    };
    let findings = match status {
        Status::AllDeclaredLoadsServed => vec![],
        Status::FlexibleLoadShortfall => vec!["one_or_more_flexible_loads_unserved".to_string()],
        Status::CriticalLoadShortfall => vec!["critical_service_demand_unserved".to_string()],
        Status::CriticalAndFlexibleShortfall => vec![
            "critical_service_demand_unserved".to_string(),
            "one_or_more_flexible_loads_unserved".to_string(),
        ],
    };

    ScenarioResult {
        scenario_id: scenario.scenario_id.clone(),
        label: scenario.label.clone(),
        evidence_class: scenario.evidence_class.clone(),
        status,
        total_onsite_generation_available_kwh: total(|row| row.onsite_generation_available_kwh),
        total_grid_import_kwh: total_grid_import,
        total_critical_requested_kwh: total(|row| row.critical_requested_kwh),
        total_critical_served_kwh: total(|row| row.critical_served_kwh),
        total_critical_unserved_kwh: critical_unserved,
        total_flexible_requested_kwh: total(|row| row.flexible_requested_kwh),
        total_flexible_served_kwh: total(|row| row.flexible_served_kwh),
        total_flexible_unserved_kwh: flexible_unserved,
        total_storage_charge_input_kwh: total_charge,
        total_storage_discharge_output_kwh: total_discharge,
        total_storage_conversion_loss_kwh: total_losses,
        total_curtailed_generation_kwh: total_curtailed,
        final_storage_soc_kwh: soc,
        estimated_grid_cost: priced.then(|| rows.iter().filter_map(|row| row.grid_cost).sum()),
        estimated_grid_emissions_kg: carbon_complete
            .then(|| rows.iter().filter_map(|row| row.grid_emissions_kg).sum()),
        maximum_absolute_energy_balance_residual_kwh: max_residual,
        slots: rows,
        findings,
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario() -> Scenario {
        Scenario {
            scenario_id: "test".into(),
            label: "synthetic".into(),
            evidence_class: "synthetic".into(),
            storage: StorageConfig {
                capacity_kwh: 20.0,
                initial_soc_kwh: 10.0,
                minimum_soc_kwh: 2.0,
                maximum_charge_power_kw: 5.0,
                maximum_discharge_power_kw: 5.0,
                charge_efficiency_fraction: 0.9,
                discharge_efficiency_fraction: 0.9,
            },
            slots: vec![Slot {
                slot_id: "slot-1".into(),
                duration_hours: 1.0,
                onsite_generation_available_kw: 0.0,
                grid_import_limit_kw: 10.0,
                critical_load_requested_kw: 4.0,
                flexible_load_requested_kw: 3.0,
                allow_storage_for_flexible_load: false,
                allow_grid_charging: false,
                grid_price_per_kwh: Some(0.1),
                grid_carbon_kg_per_kwh: Some(0.4),
            }],
        }
    }

    #[test]
    fn serves_critical_before_flexible_load() {
        let mut s = scenario();
        s.slots[0].grid_import_limit_kw = 5.0;
        let result = simulate(&s);
        assert_eq!(result.status, Status::FlexibleLoadShortfall);
        assert!((result.total_critical_served_kwh - 4.0).abs() < 1e-9);
        assert!((result.total_flexible_served_kwh - 1.0).abs() < 1e-9);
    }

    #[test]
    fn discharges_battery_for_critical_shortfall_and_tracks_soc() {
        let mut s = scenario();
        s.slots[0].grid_import_limit_kw = 1.0;
        let result = simulate(&s);
        assert!((result.total_critical_served_kwh - 4.0).abs() < 1e-9);
        assert!(result.total_storage_discharge_output_kwh > 0.0);
        assert!(result.final_storage_soc_kwh < s.storage.initial_soc_kwh);
    }

    #[test]
    fn does_not_dispatch_flexible_load_if_critical_service_fails() {
        let mut s = scenario();
        s.storage.initial_soc_kwh = 2.0;
        s.slots[0].grid_import_limit_kw = 1.0;
        let result = simulate(&s);
        assert_eq!(result.status, Status::CriticalAndFlexibleShortfall);
        assert_eq!(result.total_flexible_served_kwh, 0.0);
    }

    #[test]
    fn stores_surplus_onsite_generation_with_losses_accounted() {
        let mut s = scenario();
        s.slots[0].onsite_generation_available_kw = 10.0;
        s.slots[0].critical_load_requested_kw = 2.0;
        s.slots[0].flexible_load_requested_kw = 1.0;
        let result = simulate(&s);
        assert!(result.total_storage_charge_input_kwh > 0.0);
        assert!(result.final_storage_soc_kwh > s.storage.initial_soc_kwh);
        assert!(result.total_curtailed_generation_kwh >= 0.0);
    }

    #[test]
    fn grid_import_never_exceeds_declared_limit() {
        let mut s = scenario();
        s.slots[0].grid_import_limit_kw = 2.0;
        s.slots[0].onsite_generation_available_kw = 0.0;
        let result = simulate(&s);
        assert!(result.slots[0].grid_import_kwh <= 2.0 + 1e-9);
    }

    #[test]
    fn missing_cost_data_remains_unknown() {
        let mut s = scenario();
        s.slots[0].grid_price_per_kwh = None;
        let result = simulate(&s);
        assert!(result.estimated_grid_cost.is_none());
    }

    #[test]
    fn missing_carbon_data_remains_unknown() {
        let mut s = scenario();
        s.slots[0].grid_carbon_kg_per_kwh = None;
        let result = simulate(&s);
        assert!(result.estimated_grid_emissions_kg.is_none());
    }

    #[test]
    fn energy_balance_closes_within_floating_tolerance() {
        let mut s = scenario();
        s.slots[0].onsite_generation_available_kw = 8.0;
        s.slots[0].critical_load_requested_kw = 5.0;
        s.slots[0].flexible_load_requested_kw = 4.0;
        s.slots[0].allow_grid_charging = true;
        let result = simulate(&s);
        assert!(result.maximum_absolute_energy_balance_residual_kwh < 1e-9);
    }

    #[test]
    fn rejects_soc_outside_declared_reserve_and_capacity() {
        let mut s = scenario();
        s.storage.initial_soc_kwh = 21.0;
        assert!(validate(&s).is_some());
    }

    #[test]
    fn stable_report_does_not_depend_on_scenario_input_order() {
        let a = scenario();
        let mut b = scenario();
        b.scenario_id = "aaa".into();
        let out1 = vec![simulate(&a), simulate(&b)];
        let out2 = vec![simulate(&b), simulate(&a)];
        let mut o1 = out1;
        let mut o2 = out2;
        o1.sort_by(|x, y| x.scenario_id.cmp(&y.scenario_id));
        o2.sort_by(|x, y| x.scenario_id.cmp(&y.scenario_id));
        assert_eq!(
            serde_json::to_string(&o1).unwrap(),
            serde_json::to_string(&o2).unwrap()
        );
    }
}
