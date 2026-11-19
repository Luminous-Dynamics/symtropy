use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    env, fs,
    process::ExitCode,
};

const CLAIM_CEILING: &str = "Incremental synthetic/reference configuration screen only; not CAD, axle-load or restraint qualification, vehicle dynamics validation, roadworthiness, airworthiness, regulatory approval, measured performance or dispatch authority.";

#[derive(Debug, Deserialize)]
struct BaseReport {
    schema_version: String,
    catalog_id: String,
    results: Vec<BaseDesignResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct BaseDesignResult {
    design_id: String,
    evidence_class: String,
    status: BaseStatus,
    loaded_mass_kg: f64,
    maximum_gross_mass_kg: f64,
    mass_margin_kg: f64,
    usable_battery_energy_kwh: f64,
    battery_energy_demand_kwh_per_shift: f64,
    battery_energy_margin_kwh: f64,
    rated_continuous_power_kw: f64,
    required_continuous_power_kw: f64,
    continuous_power_margin_kw: f64,
    rated_peak_power_kw: f64,
    required_peak_power_kw: f64,
    peak_power_margin_kw: f64,
    grid_energy_to_replenish_kwh: f64,
    charger_rating_kw: f64,
    site_power_budget_kw: f64,
    simultaneous_vehicles: u32,
    charging_efficiency_fraction: f64,
    /// Conservative upper bound from simultaneous vehicles × effective per-vehicle charger cap.
    base_peak_site_input_power_kw_upper_bound: f64,
    effective_charger_power_kw: f64,
    estimated_recharge_hours: f64,
    available_window_hours: f64,
    charging_window_margin_hours: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum BaseStatus {
    PassesSyntheticScreen,
    PassesEvidenceBackedScreen,
    Blocked,
    Indeterminate,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CarrierCorpus {
    schema_version: String,
    programme_id: String,
    claim_ceiling: String,
    configurations: Vec<CarrierConfiguration>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CarrierConfiguration {
    config_id: String,
    base_design_id: String,
    architecture: String,
    evidence_class: String,
    evidence_reference: String,
    energy_boundary_note: String,
    /// Mass of pod, aircraft, mounts, dock, dedicated equipment and other
    /// additions not already present in the base EV design result.
    incremental_installed_mass_kg: Option<f64>,
    /// Additional cargo not already included in the base design's target payload.
    incremental_payload_mass_kg: Option<f64>,
    /// Additional energy withdrawn at the traction battery terminal, including
    /// the conversion losses between the truck HV bus and UAV charging system.
    incremental_vehicle_battery_draw_kwh: Option<f64>,
    /// Simultaneous additional demand on the truck HV bus.
    incremental_vehicle_bus_continuous_power_kw: Option<f64>,
    incremental_vehicle_bus_peak_power_kw: Option<f64>,
    /// Additional external site-meter energy; never double-count vehicle-battery draw.
    incremental_external_site_meter_energy_kwh: Option<f64>,
    /// Additional peak site input above the base vehicle charging load.
    incremental_external_site_peak_power_kw: Option<f64>,
    incremental_external_site_charging_window_hours: Option<f64>,
    /// Whether the additional external peak coincides with base vehicle charging.
    external_site_peak_coincident_with_vehicle_charging: Option<bool>,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Status {
    PassesSyntheticScreen,
    PassesEvidenceBackedScreen,
    Blocked,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct CarrierResult {
    config_id: String,
    base_design_id: String,
    architecture: String,
    base_evidence_class: String,
    incremental_evidence_class: String,
    status: Status,
    loaded_mass_with_module_kg: Option<f64>,
    gross_mass_limit_kg: f64,
    gross_mass_margin_after_module_kg: Option<f64>,
    total_vehicle_battery_demand_kwh: Option<f64>,
    usable_vehicle_battery_energy_kwh: f64,
    vehicle_battery_energy_margin_after_module_kwh: Option<f64>,
    total_vehicle_continuous_power_demand_kw: Option<f64>,
    vehicle_continuous_power_margin_after_module_kw: Option<f64>,
    total_vehicle_peak_power_demand_kw: Option<f64>,
    vehicle_peak_power_margin_after_module_kw: Option<f64>,
    total_vehicle_replenishment_grid_energy_kwh: Option<f64>,
    incremental_external_site_meter_energy_kwh: Option<f64>,
    external_site_recharge_hours_lower_bound: Option<f64>,
    external_site_charge_window_margin_hours_lower_bound: Option<f64>,
    external_site_recharge_is_peak_power_lower_bound: bool,
    combined_site_meter_energy_kwh: Option<f64>,
    total_vehicle_recharge_hours: Option<f64>,
    remaining_vehicle_charge_window_hours: Option<f64>,
    combined_site_peak_input_power_kw_upper_bound: Option<f64>,
    site_power_budget_kw: f64,
    site_peak_power_margin_kw_upper_bound: Option<f64>,
    missing_fields: Vec<String>,
    findings: Vec<String>,
    energy_boundary: String,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    programme_id: String,
    base_catalog_id: String,
    method: &'static str,
    results: Vec<CarrierResult>,
    claim_ceiling: &'static str,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!(
            "usage: ev_carrier_configuration_screen <vehicle-design-report.json> <carrier-configurations.json>"
        );
        return ExitCode::from(2);
    }

    let base_raw = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {}: {error}", args[0]);
            return ExitCode::from(2);
        }
    };
    let overlay_raw = match fs::read_to_string(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {}: {error}", args[1]);
            return ExitCode::from(2);
        }
    };

    let base: BaseReport = match serde_json::from_str(&base_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid base vehicle design report: {error}");
            return ExitCode::from(2);
        }
    };
    let overlay: CarrierCorpus = match serde_json::from_str(&overlay_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid carrier configuration input: {error}");
            return ExitCode::from(2);
        }
    };

    if let Err(error) = validate_inputs(&base, &overlay) {
        eprintln!("invalid carrier configuration input: {error}");
        return ExitCode::from(2);
    }

    let base_by_id: std::collections::BTreeMap<&str, &BaseDesignResult> = base
        .results
        .iter()
        .map(|result| (result.design_id.as_str(), result))
        .collect();

    let mut results: Vec<CarrierResult> = overlay
        .configurations
        .iter()
        .map(|configuration| {
            evaluate_configuration(
                configuration,
                base_by_id[configuration.base_design_id.as_str()],
            )
        })
        .collect();
    results.sort_by(|left, right| left.config_id.cmp(&right.config_id));

    let report = Report {
        schema_version: overlay.schema_version,
        programme_id: overlay.programme_id,
        base_catalog_id: base.catalog_id,
        method: "incremental carrier mass, traction-battery energy, HV-bus power and depot site-load screen composed over the existing EV design report; no aggregate score",
        results,
        claim_ceiling: CLAIM_CEILING,
    };

    match serde_json::to_string_pretty(&report) {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot serialize carrier report: {error}");
            ExitCode::from(2)
        }
    }
}

fn validate_inputs(base: &BaseReport, overlay: &CarrierCorpus) -> Result<(), String> {
    if base.schema_version != "0.1.0" || base.catalog_id != "EV-PLATFORM-001" {
        return Err("unsupported_base_vehicle_design_report".into());
    }
    if overlay.schema_version != "0.1.0" || overlay.programme_id != "EV-CARRIER-REF-01" {
        return Err("unsupported_carrier_configuration_schema_or_programme".into());
    }
    if overlay.claim_ceiling.trim().is_empty() || overlay.configurations.is_empty() {
        return Err("claim_ceiling_and_nonempty_configurations_are_required".into());
    }

    let mut base_ids = BTreeSet::new();
    for design in &base.results {
        if !base_ids.insert(design.design_id.as_str()) {
            return Err(format!("duplicate_base_design_id: {}", design.design_id));
        }
        for (field, value) in [
            ("loaded_mass_kg", design.loaded_mass_kg),
            ("maximum_gross_mass_kg", design.maximum_gross_mass_kg),
            ("mass_margin_kg", design.mass_margin_kg),
            ("usable_battery_energy_kwh", design.usable_battery_energy_kwh),
            ("battery_energy_demand_kwh_per_shift", design.battery_energy_demand_kwh_per_shift),
            ("battery_energy_margin_kwh", design.battery_energy_margin_kwh),
            ("rated_continuous_power_kw", design.rated_continuous_power_kw),
            ("required_continuous_power_kw", design.required_continuous_power_kw),
            ("continuous_power_margin_kw", design.continuous_power_margin_kw),
            ("rated_peak_power_kw", design.rated_peak_power_kw),
            ("required_peak_power_kw", design.required_peak_power_kw),
            ("peak_power_margin_kw", design.peak_power_margin_kw),
            ("grid_energy_to_replenish_kwh", design.grid_energy_to_replenish_kwh),
            ("charger_rating_kw", design.charger_rating_kw),
            ("site_power_budget_kw", design.site_power_budget_kw),
            ("charging_efficiency_fraction", design.charging_efficiency_fraction),
            ("base_peak_site_input_power_kw", design.base_peak_site_input_power_kw),
            ("effective_charger_power_kw", design.effective_charger_power_kw),
            ("estimated_recharge_hours", design.estimated_recharge_hours),
            ("available_window_hours", design.available_window_hours),
            ("charging_window_margin_hours", design.charging_window_margin_hours),
        ] {
            if !value.is_finite() {
                return Err(format!("base_{}_must_be_finite", field));
            }
        }
        if design.simultaneous_vehicles == 0
            || design.charging_efficiency_fraction <= 0.0
            || design.charging_efficiency_fraction > 1.0
            || design.effective_charger_power_kw <= 0.0
        {
            return Err(format!("base_{}_has_invalid_charging_envelope", design.design_id));
        }
    }

    let mut config_ids = BTreeSet::new();
    for (index, configuration) in overlay.configurations.iter().enumerate() {
        if configuration.config_id.trim().is_empty()
            || !config_ids.insert(configuration.config_id.as_str())
        {
            return Err(format!(
                "configurations[{index}].config_id: must be nonempty and unique"
            ));
        }
        if !base_ids.contains(configuration.base_design_id.as_str()) {
            return Err(format!(
                "configurations[{index}].base_design_id: unknown design {}",
                configuration.base_design_id
            ));
        }
        if ![
            "truck_only",
            "integrated_enclosed_bay",
            "detachable_drone_pod",
            "mobile_drone_station_trailer",
            "amr_cargo_handoff",
        ]
        .contains(&configuration.architecture.as_str())
        {
            return Err(format!(
                "configurations[{index}].architecture: unsupported value"
            ));
        }
        if ![
            "synthetic",
            "measured",
            "manufacturer",
            "literature",
            "estimate",
            "unknown",
        ]
        .contains(&configuration.evidence_class.as_str())
        {
            return Err(format!(
                "configurations[{index}].evidence_class: unsupported value"
            ));
        }
        if configuration.evidence_class != "synthetic"
            && configuration.evidence_class != "unknown"
            && configuration.evidence_reference.trim().is_empty()
        {
            return Err(format!(
                "configurations[{index}]: non-synthetic evidence requires a reference"
            ));
        }
        if configuration.energy_boundary_note.trim().is_empty()
            || configuration.notes.is_empty()
            || configuration.notes.iter().any(|note| note.trim().is_empty())
        {
            return Err(format!(
                "configurations[{index}]: energy boundary note and nonempty notes are required"
            ));
        }
        for (field, value) in [
            ("incremental_installed_mass_kg", configuration.incremental_installed_mass_kg),
            ("incremental_payload_mass_kg", configuration.incremental_payload_mass_kg),
            (
                "incremental_vehicle_battery_draw_kwh",
                configuration.incremental_vehicle_battery_draw_kwh,
            ),
            (
                "incremental_vehicle_bus_continuous_power_kw",
                configuration.incremental_vehicle_bus_continuous_power_kw,
            ),
            (
                "incremental_vehicle_bus_peak_power_kw",
                configuration.incremental_vehicle_bus_peak_power_kw,
            ),
            (
                "incremental_external_site_meter_energy_kwh",
                configuration.incremental_external_site_meter_energy_kwh,
            ),
            (
                "incremental_external_site_peak_power_kw",
                configuration.incremental_external_site_peak_power_kw,
            ),
            (
                "incremental_external_site_charging_window_hours",
                configuration.incremental_external_site_charging_window_hours,
            ),
        ] {
            if let Some(number) = value {
                if !number.is_finite() || number < 0.0 {
                    return Err(format!(
                        "configurations[{index}].{field}: must be finite and nonnegative"
                    ));
                }
            }
        }
        match (
            configuration.incremental_vehicle_bus_continuous_power_kw,
            configuration.incremental_vehicle_bus_peak_power_kw,
        ) {
            (Some(continuous), Some(peak)) if peak + 1e-9 < continuous => {
                return Err(format!(
                    "configurations[{index}]: incremental peak bus power is below continuous power"
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn evaluate_configuration(
    configuration: &CarrierConfiguration,
    base: &BaseDesignResult,
) -> CarrierResult {
    let mut missing_fields = Vec::new();
    for (field, value) in [
        ("incremental_installed_mass_kg", configuration.incremental_installed_mass_kg),
        ("incremental_payload_mass_kg", configuration.incremental_payload_mass_kg),
        (
            "incremental_vehicle_battery_draw_kwh",
            configuration.incremental_vehicle_battery_draw_kwh,
        ),
        (
            "incremental_vehicle_bus_continuous_power_kw",
            configuration.incremental_vehicle_bus_continuous_power_kw,
        ),
        (
            "incremental_vehicle_bus_peak_power_kw",
            configuration.incremental_vehicle_bus_peak_power_kw,
        ),
        (
            "incremental_external_site_meter_energy_kwh",
            configuration.incremental_external_site_meter_energy_kwh,
        ),
        (
            "incremental_external_site_peak_power_kw",
            configuration.incremental_external_site_peak_power_kw,
        ),
        (
            "incremental_external_site_charging_window_hours",
            configuration.incremental_external_site_charging_window_hours,
        ),
    ] {
        if value.is_none() {
            missing_fields.push(field.to_string());
        }
    }
    if configuration.incremental_external_site_peak_power_kw.unwrap_or(0.0) > 0.0
        && configuration.external_site_peak_coincident_with_vehicle_charging.is_none()
    {
        missing_fields.push("external_site_peak_coincident_with_vehicle_charging".into());
    }

    let installed_mass = configuration.incremental_installed_mass_kg;
    let incremental_payload = configuration.incremental_payload_mass_kg;
    let incremental_battery_draw = configuration.incremental_vehicle_battery_draw_kwh;
    let added_continuous_power = configuration.incremental_vehicle_bus_continuous_power_kw;
    let added_peak_power = configuration.incremental_vehicle_bus_peak_power_kw;
    let external_site_energy = configuration.incremental_external_site_meter_energy_kwh;
    let external_site_peak = configuration.incremental_external_site_peak_power_kw;
    let external_site_window = configuration.incremental_external_site_charging_window_hours;

    let loaded_mass = match (installed_mass, incremental_payload) {
        (Some(module), Some(payload)) => Some(base.loaded_mass_kg + module + payload),
        _ => None,
    };
    let gross_mass_margin = loaded_mass.map(|mass| base.maximum_gross_mass_kg - mass);

    let total_battery_demand = incremental_battery_draw
        .map(|draw| base.battery_energy_demand_kwh_per_shift + draw);
    let battery_energy_margin = incremental_battery_draw
        .map(|draw| base.battery_energy_margin_kwh - draw);

    let total_continuous_power = added_continuous_power
        .map(|power| base.required_continuous_power_kw + power);
    let continuous_power_margin = added_continuous_power
        .map(|power| base.continuous_power_margin_kw - power);
    let total_peak_power = added_peak_power
        .map(|power| base.required_peak_power_kw + power);
    let peak_power_margin = added_peak_power.map(|power| base.peak_power_margin_kw - power);

    let vehicle_draw_grid_energy = incremental_battery_draw
        .map(|draw| draw / base.charging_efficiency_fraction);
    let total_vehicle_grid_energy = vehicle_draw_grid_energy
        .map(|extra| base.grid_energy_to_replenish_kwh + extra);
    let total_vehicle_recharge_hours = total_vehicle_grid_energy
        .map(|energy| energy / base.effective_charger_power_kw);
    let remaining_charge_window = total_vehicle_recharge_hours
        .map(|hours| base.available_window_hours - hours);
    let combined_site_energy = match (total_vehicle_grid_energy, external_site_energy) {
        (Some(vehicle_energy), Some(site_energy)) => Some(vehicle_energy + site_energy),
        _ => None,
    };

    let external_site_recharge_lower_bound = match (external_site_energy, external_site_peak) {
        (Some(energy), Some(_)) if energy == 0.0 => Some(0.0),
        (Some(energy), Some(peak)) if peak > 0.0 => Some(energy / peak),
        _ => None,
    };
    let external_site_recharge_is_peak_power_lower_bound =
        matches!((external_site_energy, external_site_peak), (Some(energy), Some(peak)) if energy > 0.0 && peak > 0.0);
    if external_site_recharge_is_peak_power_lower_bound {
        missing_fields.push("external_site_charge_curve_or_time_series_profile".into());

    }
    let external_site_window_margin_lower_bound = match (
        external_site_recharge_lower_bound,
        external_site_window,
    ) {
        (Some(hours), Some(window)) => Some(window - hours),
        _ => None,
    };

    let combined_site_peak = match (
        external_site_peak,
        configuration.external_site_peak_coincident_with_vehicle_charging,
    ) {
        (Some(additional), Some(coincident)) => Some(if coincident {
            base.base_peak_site_input_power_kw_upper_bound + additional
        } else {
            base.base_peak_site_input_power_kw_upper_bound.max(additional)
        }),
        (Some(additional), None) if additional == 0.0 => {
            Some(base.base_peak_site_input_power_kw_upper_bound)
        }
        (Some(_), None) | (None, _) => None,
    };
    let site_peak_margin =
        combined_site_peak.map(|peak| base.site_power_budget_kw - peak);

    let mut findings = Vec::new();
    if base.status == BaseStatus::Blocked {
        findings.push("base_vehicle_design_is_blocked".into());
    }
    if let Some(margin) = gross_mass_margin {
        if margin < -1e-9 {
            findings.push("incremental_module_and_payload_exceed_gross_mass_limit".into());
        }
    }
    if let Some(margin) = battery_energy_margin {
        if margin < -1e-9 {
            findings.push(
                "incremental_vehicle_battery_draw_consumes_required_energy_reserve".into(),
            );
        }
    }
    if let Some(margin) = continuous_power_margin {
        if margin < -1e-9 {
            findings.push("combined_continuous_vehicle_bus_power_exceeds_rating".into());
        }
    }
    if let Some(margin) = peak_power_margin {
        if margin < -1e-9 {
            findings.push("combined_peak_vehicle_bus_power_exceeds_rating".into());
        }
    }
    if let Some(margin) = remaining_charge_window {
        if margin < -1e-9 {
            findings.push("additional_vehicle_battery_draw_breaks_declared_recharge_window".into());
        }
    }
    if let Some(margin) = site_peak_margin {
        if margin < -1e-9 {
            findings.push("combined_site_peak_exceeds_declared_import_budget".into());
        }
    }

    if let (Some(energy), Some(peak)) = (external_site_energy, external_site_peak) {
        if energy > 0.0 && peak == 0.0 {
            findings.push("external_site_energy_declared_without_site_charging_power".into());
        }
    }
    if let Some(margin) = external_site_window_margin_lower_bound {
        if margin < -1e-9 {
            findings.push(
                "external_site_recharge_peak_power_lower_bound_exceeds_declared_window".into(),
            );
        }
    }
    let known_blocker = base.status == BaseStatus::Blocked
        || gross_mass_margin.is_some_and(|margin| margin < -1e-9)
        || battery_energy_margin.is_some_and(|margin| margin < -1e-9)
        || continuous_power_margin.is_some_and(|margin| margin < -1e-9)
        || peak_power_margin.is_some_and(|margin| margin < -1e-9)
        || remaining_charge_window.is_some_and(|margin| margin < -1e-9)
        || site_peak_margin.is_some_and(|margin| margin < -1e-9)
        || matches!((external_site_energy, external_site_peak), (Some(energy), Some(peak)) if energy > 0.0 && peak == 0.0)
        || external_site_window_margin_lower_bound.is_some_and(|margin| margin < -1e-9);
    let base_indeterminate = base.status == BaseStatus::Indeterminate;
    let unknown_evidence = configuration.evidence_class == "unknown";
    let status = if known_blocker {
        Status::Blocked
    } else if !missing_fields.is_empty() || base_indeterminate || unknown_evidence {
        findings.push("unknown_module_or_site_inputs_prevent_rankable_configuration_result".into());
        Status::Indeterminate
    } else if base.status == BaseStatus::PassesSyntheticScreen
        || configuration.evidence_class == "synthetic"
    {
        findings.push("incremental_arithmetic_screen_only_not_a_product_qualification".into());
        Status::PassesSyntheticScreen
    } else {
        findings.push("specialist_vehicle_pod_and_site_validation_still_required".into());
        Status::PassesEvidenceBackedScreen
    };

    CarrierResult {
        config_id: configuration.config_id.clone(),
        base_design_id: configuration.base_design_id.clone(),
        architecture: configuration.architecture.clone(),
        base_evidence_class: base.evidence_class.clone(),
        incremental_evidence_class: configuration.evidence_class.clone(),
        status,
        loaded_mass_with_module_kg: loaded_mass,
        gross_mass_limit_kg: base.maximum_gross_mass_kg,
        gross_mass_margin_after_module_kg: gross_mass_margin,
        total_vehicle_battery_demand_kwh: total_battery_demand,
        usable_vehicle_battery_energy_kwh: base.usable_battery_energy_kwh,
        vehicle_battery_energy_margin_after_module_kwh: battery_energy_margin,
        total_vehicle_continuous_power_demand_kw: total_continuous_power,
        vehicle_continuous_power_margin_after_module_kw: continuous_power_margin,
        total_vehicle_peak_power_demand_kw: total_peak_power,
        vehicle_peak_power_margin_after_module_kw: peak_power_margin,
        total_vehicle_replenishment_grid_energy_kwh: total_vehicle_grid_energy,
        incremental_external_site_meter_energy_kwh: external_site_energy,
        external_site_recharge_hours_lower_bound: external_site_recharge_lower_bound,
        external_site_charge_window_margin_hours_lower_bound:
            external_site_window_margin_lower_bound,
        external_site_recharge_is_peak_power_lower_bound,
        combined_site_meter_energy_kwh: combined_site_energy,
        total_vehicle_recharge_hours,
        remaining_vehicle_charge_window_hours: remaining_charge_window,
        combined_site_peak_input_power_kw_upper_bound: combined_site_peak,
        site_power_budget_kw: base.site_power_budget_kw,
        site_peak_power_margin_kw_upper_bound: site_peak_margin,
        missing_fields,
        findings,
        energy_boundary: configuration.energy_boundary_note.clone(),
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_design(design_id: &str) -> BaseDesignResult {
        BaseDesignResult {
            design_id: design_id.into(),
            evidence_class: "synthetic".into(),
            status: BaseStatus::PassesSyntheticScreen,
            loaded_mass_kg: 20_200.0,
            maximum_gross_mass_kg: 26_000.0,
            mass_margin_kg: 5_800.0,
            usable_battery_energy_kwh: 294.462,
            battery_energy_demand_kwh_per_shift: 230.0,
            battery_energy_margin_kwh: 64.462,
            rated_continuous_power_kw: 240.0,
            required_continuous_power_kw: 200.0,
            continuous_power_margin_kw: 40.0,
            rated_peak_power_kw: 420.0,
            required_peak_power_kw: 330.0,
            peak_power_margin_kw: 90.0,
            grid_energy_to_replenish_kwh: 230.0 / 0.93,
            charger_rating_kw: 250.0,
            site_power_budget_kw: 500.0,
            simultaneous_vehicles: 2,
            charging_efficiency_fraction: 0.93,
            base_peak_site_input_power_kw_upper_bound: 500.0,
            effective_charger_power_kw: 250.0,
            estimated_recharge_hours: (230.0 / 0.93) / 250.0,
            available_window_hours: 4.0,
            charging_window_margin_hours: 4.0 - ((230.0 / 0.93) / 250.0),
        }
    }

    fn known_zero_config() -> CarrierConfiguration {
        CarrierConfiguration {
            config_id: "truck-only".into(),
            base_design_id: "urban-distribution-truck".into(),
            architecture: "truck_only".into(),
            evidence_class: "synthetic".into(),
            evidence_reference: "synthetic".into(),
            energy_boundary_note: "zero incremental energy; base EV screen only".into(),
            incremental_installed_mass_kg: Some(0.0),
            incremental_payload_mass_kg: Some(0.0),
            incremental_vehicle_battery_draw_kwh: Some(0.0),
            incremental_vehicle_bus_continuous_power_kw: Some(0.0),
            incremental_vehicle_bus_peak_power_kw: Some(0.0),
            incremental_external_site_meter_energy_kwh: Some(0.0),
            incremental_external_site_peak_power_kw: Some(0.0),
            incremental_external_site_charging_window_hours: Some(0.0),
            external_site_peak_coincident_with_vehicle_charging: Some(false),
            notes: vec!["control baseline".into()],
        }
    }

    #[test]
    fn truck_only_control_preserves_base_synthetic_screen_result() {
        let result = evaluate_configuration(
            &known_zero_config(),
            &base_design("urban-distribution-truck"),
        );
        assert_eq!(result.status, Status::PassesSyntheticScreen);
        assert_eq!(result.loaded_mass_with_module_kg, Some(20_200.0));
        assert_eq!(result.gross_mass_margin_after_module_kg, Some(5_800.0));
        assert_eq!(result.vehicle_battery_energy_margin_after_module_kwh, Some(64.462));
        assert_eq!(result.site_peak_power_margin_kw_upper_bound, Some(0.0));
        assert!(result.missing_fields.is_empty());
    }

    #[test]
    fn unknown_incremental_module_inputs_are_indeterminate_not_zero_filled() {
        let mut config = known_zero_config();
        config.architecture = "integrated_enclosed_bay".into();
        config.evidence_class = "unknown".into();
        config.incremental_installed_mass_kg = None;
        config.incremental_vehicle_battery_draw_kwh = None;
        let result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.status, Status::Indeterminate);
        assert!(result.missing_fields.iter().any(|field| field == "incremental_installed_mass_kg"));
        assert!(result.gross_mass_margin_after_module_kg.is_none());
    }

    #[test]
    fn known_incremental_mass_can_block_gross_mass() {
        let mut config = known_zero_config();
        config.incremental_installed_mass_kg = Some(6_000.0);
        let result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.status, Status::Blocked);
        assert!(result.findings.iter().any(|finding| finding.contains("gross_mass_limit")));
        assert_eq!(result.gross_mass_margin_after_module_kg, Some(-200.0));
    }

    #[test]
    fn vehicle_battery_draw_reduces_energy_margin_and_increases_grid_energy() {
        let mut config = known_zero_config();
        config.incremental_vehicle_battery_draw_kwh = Some(70.0);
        let result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.status, Status::Blocked);
        assert!(
            (result.vehicle_battery_energy_margin_after_module_kwh.unwrap() + 5.538).abs()
                < 1e-9
        );
        assert!(result.total_vehicle_replenishment_grid_energy_kwh.unwrap()
            > base_design("urban-distribution-truck").grid_energy_to_replenish_kwh);
    }

    #[test]
    fn incremental_bus_peak_is_added_to_base_peak_demand() {
        let mut config = known_zero_config();
        config.incremental_vehicle_bus_peak_power_kw = Some(100.0);
        let result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.status, Status::Blocked);
        assert_eq!(result.vehicle_peak_power_margin_after_module_kw, Some(-10.0));
    }

    #[test]
    fn external_site_recharge_reports_peak_power_lower_bound() {
        let mut config = known_zero_config();
        config.incremental_external_site_meter_energy_kwh = Some(20.0);
        config.incremental_external_site_peak_power_kw = Some(10.0);
        config.incremental_external_site_charging_window_hours = Some(3.0);
        config.external_site_peak_coincident_with_vehicle_charging = Some(false);
        let result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.external_site_recharge_hours_lower_bound, Some(2.0));
        assert_eq!(result.external_site_charge_window_margin_hours_lower_bound, Some(1.0));
        assert!(result.external_site_recharge_is_peak_power_lower_bound);
        assert_eq!(result.status, Status::Indeterminate);
        assert!(result.missing_fields.iter().any(|field| {
            field == "external_site_charge_curve_or_time_series_profile"
        }));
    }

    #[test]
    fn external_site_energy_is_not_rankable_from_peak_power_lower_bound_alone() {
        let mut config = known_zero_config();
        config.incremental_external_site_meter_energy_kwh = Some(20.0);
        config.incremental_external_site_peak_power_kw = Some(10.0);
        config.incremental_external_site_charging_window_hours = Some(3.0);
        config.external_site_peak_coincident_with_vehicle_charging = Some(false);
        let mut result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.external_site_recharge_hours_lower_bound, Some(2.0));
        assert_eq!(result.status, Status::Indeterminate);
        assert!(result.missing_fields.iter().any(|field| {
            field == "external_site_charge_curve_or_time_series_profile"
        }));
    }

    #[test]
    fn external_site_peak_power_lower_bound_outside_window_is_blocked() {
        let mut config = known_zero_config();
        config.incremental_external_site_meter_energy_kwh = Some(20.0);
        config.incremental_external_site_peak_power_kw = Some(10.0);
        config.incremental_external_site_charging_window_hours = Some(1.0);
        config.external_site_peak_coincident_with_vehicle_charging = Some(false);
        let result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.status, Status::Blocked);
        assert_eq!(result.external_site_charge_window_margin_hours_lower_bound, Some(-1.0));
    }

    #[test]
    fn external_site_zero_power_with_positive_energy_is_blocked() {
        let mut config = known_zero_config();
        config.incremental_external_site_meter_energy_kwh = Some(20.0);
        config.incremental_external_site_peak_power_kw = Some(0.0);
        config.incremental_external_site_charging_window_hours = Some(4.0);
        let result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.status, Status::Blocked);
        assert!(
            result
                .findings
                .iter()
                .any(|finding| finding.contains("without_site_charging_power"))
        );
    }

    #[test]
    fn coincident_external_site_charging_consumes_shared_import_headroom() {
        let mut config = known_zero_config();
        config.incremental_external_site_peak_power_kw = Some(50.0);
        config.incremental_external_site_charging_window_hours = Some(2.0);
        config.external_site_peak_coincident_with_vehicle_charging = Some(true);
        let result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.status, Status::Blocked);
        assert_eq!(result.site_peak_power_margin_kw_upper_bound, Some(-50.0));
    }

    #[test]
    fn noncoincident_external_site_charging_uses_larger_peak_not_sum() {
        let mut config = known_zero_config();
        config.incremental_external_site_peak_power_kw = Some(50.0);
        config.incremental_external_site_charging_window_hours = Some(2.0);
        config.external_site_peak_coincident_with_vehicle_charging = Some(false);
        let result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.site_peak_power_margin_kw_upper_bound, Some(0.0));
        assert_ne!(result.status, Status::Blocked);
    }

    #[test]
    fn unknown_external_site_charge_window_is_indeterminate() {
        let mut config = known_zero_config();
        config.incremental_external_site_meter_energy_kwh = Some(20.0);
        config.incremental_external_site_peak_power_kw = Some(10.0);
        config.incremental_external_site_charging_window_hours = None;
        config.external_site_peak_coincident_with_vehicle_charging = Some(false);
        let result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.status, Status::Indeterminate);
        assert!(result.missing_fields.iter().any(|field| {
            field == "incremental_external_site_charging_window_hours"
        }));
    }

    #[test]
    fn unknown_site_concurrency_is_indeterminate() {
        let mut config = known_zero_config();
        config.incremental_external_site_peak_power_kw = Some(50.0);
        config.incremental_external_site_charging_window_hours = Some(2.0);
        config.external_site_peak_coincident_with_vehicle_charging = None;
        let result = evaluate_configuration(&config, &base_design("urban-distribution-truck"));
        assert_eq!(result.status, Status::Indeterminate);
        assert!(
            result
                .missing_fields
                .iter()
                .any(|field| field == "external_site_peak_coincident_with_vehicle_charging")
        );
    }

    #[test]
    fn base_vehicle_blocker_cannot_be_overridden_by_pod_arithmetic() {
        let mut base = base_design("urban-distribution-truck");
        base.status = BaseStatus::Blocked;
        let result = evaluate_configuration(&known_zero_config(), &base);
        assert_eq!(result.status, Status::Blocked);
        assert!(result.findings.iter().any(|finding| finding == "base_vehicle_design_is_blocked"));
    }

    #[test]
    fn unknown_base_report_evidence_remains_indeterminate() {
        let mut base = base_design("urban-distribution-truck");
        base.status = BaseStatus::Indeterminate;
        let result = evaluate_configuration(&known_zero_config(), &base);
        assert_eq!(result.status, Status::Indeterminate);
    }
}
