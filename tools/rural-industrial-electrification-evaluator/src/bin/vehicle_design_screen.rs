use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process};

const CLAIM_CEILING: &str = "Synthetic/reference design-screen arithmetic only; not a production vehicle specification, physical validation, safety qualification, homologation, supplier quote, or deployment authorization.";

#[derive(Debug, Deserialize)]
struct Catalog {
    schema_version: String,
    catalog_id: String,
    claim_ceiling: String,
    designs: Vec<Design>,
}

#[derive(Debug, Clone, Deserialize)]
struct Design {
    design_id: String,
    label: String,
    family: String,
    architecture: String,
    mission_functional_unit: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    mass: Mass,
    battery: Battery,
    mission: Mission,
    charging: Charging,
    design_notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Mass {
    base_curb_mass_kg: f64,
    installed_module_mass_kg: f64,
    target_payload_kg: f64,
    maximum_gross_mass_kg: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct Battery {
    nominal_pack_kwh: f64,
    state_of_health_fraction: f64,
    temperature_capacity_fraction: f64,
    reserve_fraction: f64,
    rated_continuous_power_kw: f64,
    rated_peak_power_kw: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct Mission {
    battery_energy_demand_kwh_per_shift: f64,
    required_continuous_power_kw: f64,
    required_peak_power_kw: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct Charging {
    strategy: String,
    charger_rating_kw: f64,
    site_power_budget_kw: f64,
    simultaneous_vehicles: u32,
    efficiency_fraction: f64,
    available_window_hours: f64,
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
struct DesignResult {
    design_id: String,
    label: String,
    family: String,
    architecture: String,
    mission_functional_unit: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    status: Status,
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
    effective_charger_power_kw: f64,
    estimated_recharge_hours: f64,
    available_window_hours: f64,
    charging_window_margin_hours: f64,
    charging_strategy: String,
    findings: Vec<String>,
    design_notes: Vec<String>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    catalog_id: String,
    method: &'static str,
    results: Vec<DesignResult>,
    claim_ceiling: &'static str,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/rural-industrial-electrification-evaluator/fixtures/vehicle_designs_v0_1.json"
            .to_string()
    });
    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };
    let catalog: Catalog = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid electric mobility design JSON: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_catalog(&catalog) {
        eprintln!("invalid electric mobility catalog: {error}");
        process::exit(2);
    }

    let report = evaluate_catalog(&catalog);
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize design report: {error}");
            process::exit(2);
        }
    }
}

fn validate_catalog(catalog: &Catalog) -> Result<(), String> {
    if catalog.schema_version != "0.1.0" || catalog.catalog_id != "EV-PLATFORM-001" {
        return Err("unsupported_schema_or_catalog_id".to_string());
    }
    if catalog.claim_ceiling.trim().is_empty() || catalog.designs.is_empty() {
        return Err("claim_ceiling_and_nonempty_designs_are_required".to_string());
    }

    let mut ids = BTreeSet::new();
    for design in &catalog.designs {
        if !ids.insert(design.design_id.clone()) {
            return Err(format!("duplicate_design_id: {}", design.design_id));
        }
        validate_design(design).map_err(|error| format!("{}: {error}", design.design_id))?;
    }
    Ok(())
}

fn validate_design(design: &Design) -> Result<(), String> {
    for (name, value) in [
        ("design_id", design.design_id.as_str()),
        ("label", design.label.as_str()),
        ("family", design.family.as_str()),
        ("architecture", design.architecture.as_str()),
        (
            "mission_functional_unit",
            design.mission_functional_unit.as_str(),
        ),
        ("evidence_note", design.evidence_note.as_str()),
        ("charging_strategy", design.charging.strategy.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{name}_must_be_nonempty"));
        }
    }
    if ![
        "synthetic",
        "measured",
        "manufacturer",
        "literature",
        "estimate",
        "unknown",
    ]
    .contains(&design.evidence_class.as_str())
    {
        return Err("unsupported_evidence_class".to_string());
    }
    if design.evidence_class != "synthetic"
        && design.evidence_class != "unknown"
        && design.evidence_reference.trim().is_empty()
    {
        return Err("non_synthetic_evidence_requires_reference".to_string());
    }

    for (name, value) in [
        ("base_curb_mass_kg", design.mass.base_curb_mass_kg),
        ("maximum_gross_mass_kg", design.mass.maximum_gross_mass_kg),
        ("nominal_pack_kwh", design.battery.nominal_pack_kwh),
        (
            "rated_continuous_power_kw",
            design.battery.rated_continuous_power_kw,
        ),
        ("rated_peak_power_kw", design.battery.rated_peak_power_kw),
        (
            "battery_energy_demand_kwh_per_shift",
            design.mission.battery_energy_demand_kwh_per_shift,
        ),
        (
            "required_continuous_power_kw",
            design.mission.required_continuous_power_kw,
        ),
        (
            "required_peak_power_kw",
            design.mission.required_peak_power_kw,
        ),
        ("charger_rating_kw", design.charging.charger_rating_kw),
        ("site_power_budget_kw", design.charging.site_power_budget_kw),
        (
            "available_window_hours",
            design.charging.available_window_hours,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name}_must_be_finite_and_positive"));
        }
    }

    for (name, value) in [
        (
            "installed_module_mass_kg",
            design.mass.installed_module_mass_kg,
        ),
        ("target_payload_kg", design.mass.target_payload_kg),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!("{name}_must_be_finite_and_nonnegative"));
        }
    }

    for (name, value) in [
        (
            "state_of_health_fraction",
            design.battery.state_of_health_fraction,
        ),
        (
            "temperature_capacity_fraction",
            design.battery.temperature_capacity_fraction,
        ),
        (
            "charging_efficiency_fraction",
            design.charging.efficiency_fraction,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 || value > 1.0 {
            return Err(format!("{name}_must_be_in_(0,1]"));
        }
    }
    if !design.battery.reserve_fraction.is_finite()
        || design.battery.reserve_fraction < 0.0
        || design.battery.reserve_fraction >= 1.0
    {
        return Err("reserve_fraction_must_be_in_[0,1)".to_string());
    }
    if design.charging.simultaneous_vehicles == 0 {
        return Err("simultaneous_vehicles_must_be_positive".to_string());
    }
    if design.battery.rated_peak_power_kw < design.battery.rated_continuous_power_kw {
        return Err("rated_peak_power_must_be_at_least_rated_continuous_power".to_string());
    }
    if design.mission.required_peak_power_kw < design.mission.required_continuous_power_kw {
        return Err("required_peak_power_must_be_at_least_required_continuous_power".to_string());
    }
    if design.design_notes.is_empty()
        || design
            .design_notes
            .iter()
            .any(|note| note.trim().is_empty())
    {
        return Err("design_notes_must_be_nonempty".to_string());
    }
    Ok(())
}

fn evaluate_catalog(catalog: &Catalog) -> Report {
    let mut results: Vec<DesignResult> = catalog.designs.iter().map(evaluate_design).collect();
    results.sort_by(|a, b| a.design_id.cmp(&b.design_id));
    Report {
        schema_version: catalog.schema_version.clone(),
        catalog_id: catalog.catalog_id.clone(),
        method: "deterministic mass, derated energy, distinct continuous/peak power, and shared-site charging-window screen; no aggregate score",
        results,
        claim_ceiling: CLAIM_CEILING,
    }
}

fn evaluate_design(design: &Design) -> DesignResult {
    let loaded_mass_kg = design.mass.base_curb_mass_kg
        + design.mass.installed_module_mass_kg
        + design.mass.target_payload_kg;
    let mass_margin_kg = design.mass.maximum_gross_mass_kg - loaded_mass_kg;

    let usable_battery_energy_kwh = design.battery.nominal_pack_kwh
        * design.battery.state_of_health_fraction
        * design.battery.temperature_capacity_fraction
        * (1.0 - design.battery.reserve_fraction);
    let battery_energy_margin_kwh =
        usable_battery_energy_kwh - design.mission.battery_energy_demand_kwh_per_shift;

    let continuous_power_margin_kw =
        design.battery.rated_continuous_power_kw - design.mission.required_continuous_power_kw;
    let peak_power_margin_kw =
        design.battery.rated_peak_power_kw - design.mission.required_peak_power_kw;

    let grid_energy_to_replenish_kwh =
        design.mission.battery_energy_demand_kwh_per_shift / design.charging.efficiency_fraction;
    let shared_site_power_kw =
        design.charging.site_power_budget_kw / f64::from(design.charging.simultaneous_vehicles);
    let effective_charger_power_kw = design.charging.charger_rating_kw.min(shared_site_power_kw);
    let estimated_recharge_hours = grid_energy_to_replenish_kwh / effective_charger_power_kw;
    let charging_window_margin_hours =
        design.charging.available_window_hours - estimated_recharge_hours;

    let mut findings = Vec::new();
    if mass_margin_kg < -1e-9 {
        findings.push("loaded_mass_exceeds_declared_gross_mass_limit".to_string());
    }
    if battery_energy_margin_kwh < -1e-9 {
        findings.push("derated_usable_battery_energy_does_not_cover_shift".to_string());
    }
    if continuous_power_margin_kw < -1e-9 {
        findings.push("continuous_power_rating_below_declared_mission_demand".to_string());
    }
    if peak_power_margin_kw < -1e-9 {
        findings.push("peak_power_rating_below_declared_mission_demand".to_string());
    }
    if charging_window_margin_hours < -1e-9 {
        findings.push(
            "recharge_does_not_fit_declared_window_under_constant_power_assumption".to_string(),
        );
    }

    let status = if !findings.is_empty() {
        Status::Blocked
    } else if design.evidence_class == "unknown" {
        findings.push("unknown_evidence_blocks_decision_use".to_string());
        Status::Indeterminate
    } else if design.evidence_class == "synthetic" {
        findings
            .push("arithmetic_screen_only_synthetic_input_is_not_design_qualification".to_string());
        Status::PassesSyntheticScreen
    } else {
        findings.push("screen_only_specialist_validation_still_required".to_string());
        Status::PassesEvidenceBackedScreen
    };

    DesignResult {
        design_id: design.design_id.clone(),
        label: design.label.clone(),
        family: design.family.clone(),
        architecture: design.architecture.clone(),
        mission_functional_unit: design.mission_functional_unit.clone(),
        evidence_class: design.evidence_class.clone(),
        evidence_reference: design.evidence_reference.clone(),
        evidence_note: design.evidence_note.clone(),
        status,
        loaded_mass_kg,
        maximum_gross_mass_kg: design.mass.maximum_gross_mass_kg,
        mass_margin_kg,
        usable_battery_energy_kwh,
        battery_energy_demand_kwh_per_shift: design.mission.battery_energy_demand_kwh_per_shift,
        battery_energy_margin_kwh,
        rated_continuous_power_kw: design.battery.rated_continuous_power_kw,
        required_continuous_power_kw: design.mission.required_continuous_power_kw,
        continuous_power_margin_kw,
        rated_peak_power_kw: design.battery.rated_peak_power_kw,
        required_peak_power_kw: design.mission.required_peak_power_kw,
        peak_power_margin_kw,
        grid_energy_to_replenish_kwh,
        effective_charger_power_kw,
        estimated_recharge_hours,
        available_window_hours: design.charging.available_window_hours,
        charging_window_margin_hours,
        charging_strategy: design.charging.strategy.clone(),
        findings,
        design_notes: design.design_notes.clone(),
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_design() -> Design {
        Design {
            design_id: "sample-vehicle".to_string(),
            label: "Sample synthetic EV".to_string(),
            family: "test".to_string(),
            architecture: "battery_electric".to_string(),
            mission_functional_unit: "jobs per shift".to_string(),
            evidence_class: "synthetic".to_string(),
            evidence_reference: "synthetic".to_string(),
            evidence_note: "Unit test fixture only.".to_string(),
            mass: Mass {
                base_curb_mass_kg: 1_500.0,
                installed_module_mass_kg: 100.0,
                target_payload_kg: 300.0,
                maximum_gross_mass_kg: 2_100.0,
            },
            battery: Battery {
                nominal_pack_kwh: 100.0,
                state_of_health_fraction: 0.9,
                temperature_capacity_fraction: 0.8,
                reserve_fraction: 0.2,
                rated_continuous_power_kw: 80.0,
                rated_peak_power_kw: 150.0,
            },
            mission: Mission {
                battery_energy_demand_kwh_per_shift: 24.0,
                required_continuous_power_kw: 60.0,
                required_peak_power_kw: 120.0,
            },
            charging: Charging {
                strategy: "depot".to_string(),
                charger_rating_kw: 22.0,
                site_power_budget_kw: 33.0,
                simultaneous_vehicles: 2,
                efficiency_fraction: 0.9,
                available_window_hours: 4.0,
            },
            design_notes: vec!["Synthetic test only.".to_string()],
        }
    }

    fn sample_catalog(designs: Vec<Design>) -> Catalog {
        Catalog {
            schema_version: "0.1.0".to_string(),
            catalog_id: "EV-PLATFORM-001".to_string(),
            claim_ceiling: "Synthetic test only.".to_string(),
            designs,
        }
    }

    #[test]
    fn usable_energy_applies_soh_temperature_and_reserve() {
        let result = evaluate_design(&sample_design());
        assert!((result.usable_battery_energy_kwh - 57.6).abs() < 1e-9);
        assert!((result.battery_energy_margin_kwh - 33.6).abs() < 1e-9);
    }

    #[test]
    fn loaded_mass_includes_installed_module_and_payload() {
        let result = evaluate_design(&sample_design());
        assert!((result.loaded_mass_kg - 1_900.0).abs() < 1e-9);
        assert!((result.mass_margin_kg - 200.0).abs() < 1e-9);
    }

    #[test]
    fn shared_site_power_limits_effective_charger_rate() {
        let mut design = sample_design();
        design.charging.site_power_budget_kw = 10.0;
        design.charging.simultaneous_vehicles = 4;
        let result = evaluate_design(&design);
        assert!((result.effective_charger_power_kw - 2.5).abs() < 1e-9);
        assert_eq!(result.status, Status::Blocked);
        assert!(result.findings.iter().any(|finding| finding
            == "recharge_does_not_fit_declared_window_under_constant_power_assumption"));
    }

    #[test]
    fn battery_energy_shortfall_blocks_candidate() {
        let mut design = sample_design();
        design.mission.battery_energy_demand_kwh_per_shift = 80.0;
        let result = evaluate_design(&design);
        assert_eq!(result.status, Status::Blocked);
        assert!(
            result
                .findings
                .iter()
                .any(|finding| finding == "derated_usable_battery_energy_does_not_cover_shift")
        );
    }

    #[test]
    fn gross_mass_overload_blocks_candidate() {
        let mut design = sample_design();
        design.mass.target_payload_kg = 700.0;
        let result = evaluate_design(&design);
        assert_eq!(result.status, Status::Blocked);
        assert!(result.mass_margin_kg < 0.0);
    }

    #[test]
    fn continuous_power_shortfall_blocks_candidate() {
        let mut design = sample_design();
        design.mission.required_continuous_power_kw = 90.0;
        let result = evaluate_design(&design);
        assert_eq!(result.status, Status::Blocked);
        assert!(
            result
                .findings
                .iter()
                .any(|finding| finding == "continuous_power_rating_below_declared_mission_demand")
        );
    }

    #[test]
    fn peak_power_shortfall_blocks_candidate() {
        let mut design = sample_design();
        design.mission.required_peak_power_kw = 180.0;
        let result = evaluate_design(&design);
        assert_eq!(result.status, Status::Blocked);
        assert!(
            result
                .findings
                .iter()
                .any(|finding| finding == "peak_power_rating_below_declared_mission_demand")
        );
    }

    #[test]
    fn insufficient_charge_window_blocks_candidate() {
        let mut design = sample_design();
        design.charging.available_window_hours = 0.5;
        let result = evaluate_design(&design);
        assert_eq!(result.status, Status::Blocked);
        assert!(result.charging_window_margin_hours < 0.0);
    }

    #[test]
    fn unknown_evidence_is_indeterminate_not_a_pass() {
        let mut design = sample_design();
        design.evidence_class = "unknown".to_string();
        design.evidence_reference.clear();
        let result = evaluate_design(&design);
        assert_eq!(result.status, Status::Indeterminate);
        assert!(
            result
                .findings
                .iter()
                .any(|finding| finding == "unknown_evidence_blocks_decision_use")
        );
    }

    #[test]
    fn sourced_evidence_requires_a_reference() {
        let mut design = sample_design();
        design.evidence_class = "manufacturer".to_string();
        design.evidence_reference.clear();
        assert_eq!(
            validate_design(&design).unwrap_err(),
            "non_synthetic_evidence_requires_reference"
        );
    }

    #[test]
    fn invalid_reserve_fraction_is_rejected() {
        let mut design = sample_design();
        design.battery.reserve_fraction = 1.0;
        assert_eq!(
            validate_design(&design).unwrap_err(),
            "reserve_fraction_must_be_in_[0,1)"
        );
    }

    #[test]
    fn duplicate_design_ids_are_rejected() {
        let design = sample_design();
        let catalog = sample_catalog(vec![design.clone(), design]);
        assert_eq!(
            validate_catalog(&catalog).unwrap_err(),
            "duplicate_design_id: sample-vehicle"
        );
    }

    #[test]
    fn report_order_is_deterministic_under_input_permutation() {
        let mut first = sample_design();
        first.design_id = "z-last".to_string();
        let mut second = sample_design();
        second.design_id = "a-first".to_string();
        let left = evaluate_catalog(&sample_catalog(vec![first.clone(), second.clone()]));
        let right = evaluate_catalog(&sample_catalog(vec![second, first]));
        assert_eq!(left.results, right.results);
        assert_eq!(left.results[0].design_id, "a-first");
    }
}
