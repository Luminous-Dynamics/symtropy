use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process};

const G: f64 = 9.806_65;
const JOULES_PER_KWH: f64 = 3_600_000.0;
const CLAIM_CEILING: &str = "Deterministic simplified longitudinal mission screen with synthetic/reference inputs only; not a calibrated vehicle digital twin, battery electrochemistry model, safety qualification, homologation, or deployment authorization.";

#[derive(Debug, Deserialize)]
struct Catalog {
    schema_version: String,
    catalog_id: String,
    designs: Vec<Design>,
}

#[derive(Debug, Deserialize)]
struct Design {
    design_id: String,
    mass: Mass,
    battery: Battery,
}

#[derive(Debug, Deserialize)]
struct Mass {
    base_curb_mass_kg: f64,
    installed_module_mass_kg: f64,
    target_payload_kg: f64,
    maximum_gross_mass_kg: f64,
}

#[derive(Debug, Deserialize)]
struct Battery {
    nominal_pack_kwh: f64,
    state_of_health_fraction: f64,
    temperature_capacity_fraction: f64,
    reserve_fraction: f64,
    rated_peak_power_kw: f64,
}

#[derive(Debug, Deserialize)]
struct ProfileCorpus {
    schema_version: String,
    profile_set_id: String,
    claim_ceiling: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
struct Scenario {
    scenario_id: String,
    label: String,
    design_id: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    mission_functional_unit: String,
    initial_usable_soc_fraction: f64,
    auxiliary_load_kw: f64,
    nontravel_duration_hours: f64,
    nontravel_auxiliary_load_kw: f64,
    declared_peak_battery_power_kw: f64,
    rotating_mass_factor: f64,
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, Deserialize)]
struct Segment {
    segment_id: String,
    distance_m: f64,
    duration_s: f64,
    initial_speed_mps: f64,
    final_speed_mps: f64,
    grade_fraction: f64,
    rolling_resistance_coefficient: f64,
    drag_area_m2: f64,
    air_density_kg_per_m3: f64,
    drivetrain_efficiency_fraction: f64,
    regenerative_efficiency_fraction: f64,
    max_regen_battery_power_kw: f64,
    work_energy_demand_kwh: f64,
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
struct SegmentResult {
    segment_id: String,
    distance_m: f64,
    duration_s: f64,
    average_speed_mps: f64,
    rolling_force_n: f64,
    grade_force_n: f64,
    aerodynamic_force_n: f64,
    wheel_net_work_kwh: f64,
    positive_wheel_work_kwh: f64,
    braking_energy_available_kwh: f64,
    traction_battery_energy_kwh: f64,
    auxiliary_battery_energy_kwh: f64,
    declared_work_energy_kwh: f64,
    regen_energy_available_at_battery_kwh: f64,
    regen_energy_captured_kwh: f64,
    regen_energy_curtailed_kwh: f64,
    segment_demand_before_regen_kwh: f64,
    unserved_energy_kwh: f64,
    usable_soc_start_kwh: f64,
    usable_soc_end_kwh: f64,
    average_traction_power_kw: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ScenarioResult {
    scenario_id: String,
    label: String,
    design_id: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    mission_functional_unit: String,
    status: Status,
    loaded_mass_kg: f64,
    gross_mass_limit_kg: f64,
    gross_mass_margin_kg: f64,
    usable_battery_energy_kwh: f64,
    initial_usable_soc_kwh: f64,
    final_usable_soc_kwh: f64,
    declared_peak_battery_power_kw: f64,
    battery_peak_power_rating_kw: f64,
    peak_power_margin_kw: f64,
    total_distance_km: f64,
    total_travel_hours: f64,
    total_stationary_segment_hours: f64,
    total_profiled_segment_hours: f64,
    nontravel_duration_hours: f64,
    total_traction_battery_energy_kwh: f64,
    total_auxiliary_energy_kwh: f64,
    total_declared_work_energy_kwh: f64,
    total_regen_energy_available_at_battery_kwh: f64,
    total_regen_energy_captured_kwh: f64,
    total_regen_energy_curtailed_kwh: f64,
    net_battery_energy_demand_kwh: f64,
    energy_margin_vs_usable_capacity_kwh: f64,
    total_unserved_energy_kwh: f64,
    maximum_segment_average_traction_power_kw: f64,
    segments: Vec<SegmentResult>,
    findings: Vec<String>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    catalog_id: String,
    profile_set_id: String,
    method: &'static str,
    results: Vec<ScenarioResult>,
    claim_ceiling: &'static str,
}

fn main() {
    let mut args = env::args().skip(1);
    let catalog_path = args.next().unwrap_or_else(|| {
        "tools/rural-industrial-electrification-evaluator/fixtures/vehicle_designs_v0_1.json"
            .to_string()
    });
    let profile_path = args.next().unwrap_or_else(|| {
        "tools/rural-industrial-electrification-evaluator/fixtures/vehicle_missions_v0_2.json"
            .to_string()
    });
    let catalog_input = read_file(&catalog_path);
    let profile_input = read_file(&profile_path);

    let catalog: Catalog = match serde_json::from_str(&catalog_input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid vehicle catalog JSON: {error}");
            process::exit(2);
        }
    };
    let profiles: ProfileCorpus = match serde_json::from_str(&profile_input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid vehicle mission JSON: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_inputs(&catalog, &profiles) {
        eprintln!("invalid vehicle mission input: {error}");
        process::exit(2);
    }

    let design_by_id: std::collections::BTreeMap<&str, &Design> = catalog
        .designs
        .iter()
        .map(|design| (design.design_id.as_str(), design))
        .collect();

    let mut results: Vec<ScenarioResult> = profiles
        .scenarios
        .iter()
        .map(|scenario| simulate(scenario, design_by_id[scenario.design_id.as_str()]))
        .collect();
    results.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));

    let report = Report {
        schema_version: profiles.schema_version,
        catalog_id: catalog.catalog_id,
        profile_set_id: profiles.profile_set_id,
        method: "longitudinal force and kinetic-energy screen with explicit auxiliary/work demand, regenerative limits, and chronological usable-energy state",
        results,
        claim_ceiling: CLAIM_CEILING,
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize mission report: {error}");
            process::exit(2);
        }
    }
}

fn read_file(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    }
}

fn validate_inputs(catalog: &Catalog, profiles: &ProfileCorpus) -> Result<(), String> {
    if catalog.schema_version != "0.1.0" || catalog.catalog_id != "EV-PLATFORM-001" {
        return Err("unsupported_catalog_schema_or_id".to_string());
    }
    if profiles.schema_version != "0.2.0" || profiles.profile_set_id != "EV-MISSION-PHYSICS-001" {
        return Err("unsupported_mission_profile_schema_or_id".to_string());
    }
    if profiles.claim_ceiling.trim().is_empty()
        || profiles.scenarios.is_empty()
        || catalog.designs.is_empty()
    {
        return Err("claim_ceiling_and_nonempty_catalogs_are_required".to_string());
    }

    let mut design_ids = BTreeSet::new();
    for design in &catalog.designs {
        if !design_ids.insert(design.design_id.clone()) {
            return Err(format!("duplicate_design_id: {}", design.design_id));
        }
        for (name, value) in [
            ("base_curb_mass_kg", design.mass.base_curb_mass_kg),
            ("maximum_gross_mass_kg", design.mass.maximum_gross_mass_kg),
            ("nominal_pack_kwh", design.battery.nominal_pack_kwh),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("design_{}_must_be_finite_and_positive", name));
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
                return Err(format!("design_{}_must_be_finite_and_nonnegative", name));
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
        ] {
            if !value.is_finite() || value <= 0.0 || value > 1.0 {
                return Err(format!("design_{}_must_be_in_(0,1]", name));
            }
        }
        if !design.battery.reserve_fraction.is_finite()
            || design.battery.reserve_fraction < 0.0
            || design.battery.reserve_fraction >= 1.0
            || !design.battery.rated_peak_power_kw.is_finite()
            || design.battery.rated_peak_power_kw <= 0.0
        {
            return Err(format!("design_{}_invalid", design.design_id));
        }
    }

    let by_id: BTreeSet<&str> = design_ids.iter().map(String::as_str).collect();
    let mut scenario_ids = BTreeSet::new();
    for scenario in &profiles.scenarios {
        if !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err(format!("duplicate_scenario_id: {}", scenario.scenario_id));
        }
        if !by_id.contains(scenario.design_id.as_str()) {
            return Err(format!(
                "unknown_design_id: {} references {}",
                scenario.scenario_id, scenario.design_id
            ));
        }
        if scenario.scenario_id.trim().is_empty()
            || scenario.label.trim().is_empty()
            || scenario.mission_functional_unit.trim().is_empty()
            || scenario.evidence_note.trim().is_empty()
        {
            return Err(format!(
                "scenario_{}_text_fields_must_be_nonempty",
                scenario.scenario_id
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
        .contains(&scenario.evidence_class.as_str())
        {
            return Err(format!(
                "scenario_{}_unsupported_evidence_class",
                scenario.scenario_id
            ));
        }
        if scenario.evidence_class != "synthetic"
            && scenario.evidence_class != "unknown"
            && scenario.evidence_reference.trim().is_empty()
        {
            return Err(format!(
                "scenario_{}_non_synthetic_evidence_requires_reference",
                scenario.scenario_id
            ));
        }
        for (name, value) in [
            (
                "initial_usable_soc_fraction",
                scenario.initial_usable_soc_fraction,
            ),
            (
                "declared_peak_battery_power_kw",
                scenario.declared_peak_battery_power_kw,
            ),
            ("rotating_mass_factor", scenario.rotating_mass_factor),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!(
                    "scenario_{}_{}_must_be_finite_and_positive",
                    scenario.scenario_id, name
                ));
            }
        }
        if scenario.initial_usable_soc_fraction > 1.0 || scenario.rotating_mass_factor < 1.0 {
            return Err(format!(
                "scenario_{}_soc_or_rotating_mass_factor_out_of_domain",
                scenario.scenario_id
            ));
        }
        for (name, value) in [
            ("auxiliary_load_kw", scenario.auxiliary_load_kw),
            (
                "nontravel_duration_hours",
                scenario.nontravel_duration_hours,
            ),
            (
                "nontravel_auxiliary_load_kw",
                scenario.nontravel_auxiliary_load_kw,
            ),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!(
                    "scenario_{}_{}_must_be_finite_and_nonnegative",
                    scenario.scenario_id, name
                ));
            }
        }
        if scenario.segments.is_empty() {
            return Err(format!(
                "scenario_{}_must_contain_segments",
                scenario.scenario_id
            ));
        }

        let mut segment_ids = BTreeSet::new();
        for segment in &scenario.segments {
            if segment.segment_id.trim().is_empty()
                || !segment_ids.insert(segment.segment_id.clone())
            {
                return Err(format!(
                    "scenario_{}_segment_ids_must_be_unique",
                    scenario.scenario_id
                ));
            }
            for (name, value) in [
                ("distance_m", segment.distance_m),
                ("duration_s", segment.duration_s),
                ("initial_speed_mps", segment.initial_speed_mps),
                ("final_speed_mps", segment.final_speed_mps),
                (
                    "rolling_resistance_coefficient",
                    segment.rolling_resistance_coefficient,
                ),
                ("drag_area_m2", segment.drag_area_m2),
                ("air_density_kg_per_m3", segment.air_density_kg_per_m3),
                (
                    "drivetrain_efficiency_fraction",
                    segment.drivetrain_efficiency_fraction,
                ),
                (
                    "regenerative_efficiency_fraction",
                    segment.regenerative_efficiency_fraction,
                ),
                (
                    "max_regen_battery_power_kw",
                    segment.max_regen_battery_power_kw,
                ),
                ("work_energy_demand_kwh", segment.work_energy_demand_kwh),
            ] {
                if !value.is_finite() || value < 0.0 {
                    return Err(format!(
                        "scenario_{}_segment_{}_{}_must_be_finite_and_nonnegative",
                        scenario.scenario_id, segment.segment_id, name
                    ));
                }
            }
            if !segment.grade_fraction.is_finite() || segment.grade_fraction.abs() > 0.5 {
                return Err(format!(
                    "scenario_{}_segment_{}_grade_out_of_domain",
                    scenario.scenario_id, segment.segment_id
                ));
            }
            if segment.duration_s <= 0.0
                || segment.air_density_kg_per_m3 <= 0.0
                || segment.drivetrain_efficiency_fraction <= 0.0
                || segment.drivetrain_efficiency_fraction > 1.0
                || segment.regenerative_efficiency_fraction > 1.0
            {
                return Err(format!(
                    "scenario_{}_segment_{}_invalid_duration_or_efficiency",
                    scenario.scenario_id, segment.segment_id
                ));
            }
            if segment.rolling_resistance_coefficient > 0.2 {
                return Err(format!(
                    "scenario_{}_segment_{}_rolling_resistance_out_of_domain",
                    scenario.scenario_id, segment.segment_id
                ));
            }
            if segment.distance_m == 0.0 {
                if segment.initial_speed_mps > 1e-9 || segment.final_speed_mps > 1e-9 {
                    return Err(format!(
                        "scenario_{}_segment_{}_stationary_segment_must_have_zero_speed",
                        scenario.scenario_id, segment.segment_id
                    ));
                }
            } else if segment.distance_m / segment.duration_s <= 0.0 {
                return Err(format!(
                    "scenario_{}_segment_{}_average_speed_must_be_positive",
                    scenario.scenario_id, segment.segment_id
                ));
            }
        }
    }
    Ok(())
}

fn simulate(scenario: &Scenario, design: &Design) -> ScenarioResult {
    let mass = design.mass.base_curb_mass_kg
        + design.mass.installed_module_mass_kg
        + design.mass.target_payload_kg;
    let usable_capacity = design.battery.nominal_pack_kwh
        * design.battery.state_of_health_fraction
        * design.battery.temperature_capacity_fraction
        * (1.0 - design.battery.reserve_fraction);
    let initial_soc = usable_capacity * scenario.initial_usable_soc_fraction;
    let mut soc = initial_soc;
    let mut results = Vec::with_capacity(scenario.segments.len());
    let mut total_traction = 0.0;
    let mut total_aux = 0.0;
    let mut total_work = 0.0;
    let mut regen_available_total = 0.0;
    let mut regen_captured_total = 0.0;
    let mut regen_curtailed_total = 0.0;
    let mut unserved_total = 0.0;
    let mut distance_m = 0.0;
    let mut travel_seconds = 0.0;
    let mut stationary_segment_seconds = 0.0;
    let mut profiled_segment_seconds = 0.0;
    let mut max_avg_traction_power: f64 = 0.0;

    for segment in &scenario.segments {
        let soc_start = soc;
        let dt_h = segment.duration_s / 3600.0;
        let average_speed = segment.distance_m / segment.duration_s;
        let cos_grade = 1.0 / (1.0 + segment.grade_fraction * segment.grade_fraction).sqrt();
        let rolling_force = segment.rolling_resistance_coefficient * mass * G * cos_grade;
        let grade_force = mass * G * segment.grade_fraction * cos_grade;
        let aerodynamic_force = 0.5
            * segment.air_density_kg_per_m3
            * segment.drag_area_m2
            * average_speed
            * average_speed;

        let road_work_j = (rolling_force + grade_force + aerodynamic_force) * segment.distance_m;
        let kinetic_delta_j = 0.5
            * mass
            * scenario.rotating_mass_factor
            * (segment.final_speed_mps * segment.final_speed_mps
                - segment.initial_speed_mps * segment.initial_speed_mps);
        let net_wheel_work_j = road_work_j + kinetic_delta_j;
        let wheel_work_kwh = net_wheel_work_j / JOULES_PER_KWH;
        let positive_wheel_work_kwh = wheel_work_kwh.max(0.0);
        let braking_available_kwh = (-wheel_work_kwh).max(0.0);
        let traction_battery_kwh = positive_wheel_work_kwh / segment.drivetrain_efficiency_fraction;
        let aux_kwh = scenario.auxiliary_load_kw * dt_h;
        let before_regen_demand = traction_battery_kwh + aux_kwh + segment.work_energy_demand_kwh;
        let unserved = (before_regen_demand - soc).max(0.0);
        let draw = before_regen_demand.min(soc);
        soc = (soc - draw).max(0.0);

        let regen_available_battery =
            braking_available_kwh * segment.regenerative_efficiency_fraction;
        let regen_power_cap_kwh = segment.max_regen_battery_power_kw * dt_h;
        let regen_capture = regen_available_battery
            .min(regen_power_cap_kwh)
            .min((usable_capacity - soc).max(0.0));
        let regen_curtailed = (regen_available_battery - regen_capture).max(0.0);
        soc = (soc + regen_capture).min(usable_capacity);

        let avg_traction_power = if dt_h > 0.0 {
            traction_battery_kwh / dt_h
        } else {
            0.0
        };
        max_avg_traction_power = max_avg_traction_power.max(avg_traction_power);
        total_traction += traction_battery_kwh;
        total_aux += aux_kwh;
        total_work += segment.work_energy_demand_kwh;
        regen_available_total += regen_available_battery;
        regen_captured_total += regen_capture;
        regen_curtailed_total += regen_curtailed;
        unserved_total += unserved;
        distance_m += segment.distance_m;
        profiled_segment_seconds += segment.duration_s;
        if segment.distance_m > 0.0 {
            travel_seconds += segment.duration_s;
        } else {
            stationary_segment_seconds += segment.duration_s;
        }

        results.push(SegmentResult {
            segment_id: segment.segment_id.clone(),
            distance_m: segment.distance_m,
            duration_s: segment.duration_s,
            average_speed_mps: average_speed,
            rolling_force_n: rolling_force,
            grade_force_n: grade_force,
            aerodynamic_force_n: aerodynamic_force,
            wheel_net_work_kwh: wheel_work_kwh,
            positive_wheel_work_kwh,
            braking_energy_available_kwh: braking_available_kwh,
            traction_battery_energy_kwh: traction_battery_kwh,
            auxiliary_battery_energy_kwh: aux_kwh,
            declared_work_energy_kwh: segment.work_energy_demand_kwh,
            regen_energy_available_at_battery_kwh: regen_available_battery,
            regen_energy_captured_kwh: regen_capture,
            regen_energy_curtailed_kwh: regen_curtailed,
            segment_demand_before_regen_kwh: before_regen_demand,
            unserved_energy_kwh: unserved,
            usable_soc_start_kwh: soc_start,
            usable_soc_end_kwh: soc,
            average_traction_power_kw: avg_traction_power,
        });
    }

    let nontravel_aux = scenario.nontravel_auxiliary_load_kw * scenario.nontravel_duration_hours;
    total_aux += nontravel_aux;
    let nontravel_unserved = (nontravel_aux - soc).max(0.0);
    let draw_nontravel = nontravel_aux.min(soc);
    soc = (soc - draw_nontravel).max(0.0);
    unserved_total += nontravel_unserved;

    let net_demand = total_traction + total_aux + total_work - regen_captured_total;
    let energy_margin = usable_capacity - net_demand;
    let mass_margin = design.mass.maximum_gross_mass_kg - mass;
    let peak_margin = design.battery.rated_peak_power_kw - scenario.declared_peak_battery_power_kw;
    let mut findings = Vec::new();
    if mass_margin < -1e-9 {
        findings.push("loaded_mass_exceeds_gross_mass_limit".to_string());
    }
    if energy_margin < -1e-9 || unserved_total > 1e-9 {
        findings
            .push("mission_energy_exceeds_available_usable_battery_energy_or_reserve".to_string());
    }
    if peak_margin < -1e-9 || max_avg_traction_power > design.battery.rated_peak_power_kw + 1e-9 {
        findings.push("declared_or_segment_average_power_exceeds_peak_rating".to_string());
    }

    let status = if !findings.is_empty() {
        Status::Blocked
    } else if scenario.evidence_class == "unknown" {
        findings.push("unknown_evidence_blocks_decision_use".to_string());
        Status::Indeterminate
    } else if scenario.evidence_class == "synthetic" {
        findings.push("simplified_longitudinal_screen_is_not_vehicle_validation".to_string());
        Status::PassesSyntheticScreen
    } else {
        findings
            .push("physics_screen_only_independent_vehicle_validation_still_required".to_string());
        Status::PassesEvidenceBackedScreen
    };

    ScenarioResult {
        scenario_id: scenario.scenario_id.clone(),
        label: scenario.label.clone(),
        design_id: scenario.design_id.clone(),
        evidence_class: scenario.evidence_class.clone(),
        evidence_reference: scenario.evidence_reference.clone(),
        evidence_note: scenario.evidence_note.clone(),
        mission_functional_unit: scenario.mission_functional_unit.clone(),
        status,
        loaded_mass_kg: mass,
        gross_mass_limit_kg: design.mass.maximum_gross_mass_kg,
        gross_mass_margin_kg: mass_margin,
        usable_battery_energy_kwh: usable_capacity,
        initial_usable_soc_kwh: initial_soc,
        final_usable_soc_kwh: soc,
        declared_peak_battery_power_kw: scenario.declared_peak_battery_power_kw,
        battery_peak_power_rating_kw: design.battery.rated_peak_power_kw,
        peak_power_margin_kw: peak_margin,
        total_distance_km: distance_m / 1000.0,
        total_travel_hours: travel_seconds / 3600.0,
        total_stationary_segment_hours: stationary_segment_seconds / 3600.0,
        total_profiled_segment_hours: profiled_segment_seconds / 3600.0,
        nontravel_duration_hours: scenario.nontravel_duration_hours,
        total_traction_battery_energy_kwh: total_traction,
        total_auxiliary_energy_kwh: total_aux,
        total_declared_work_energy_kwh: total_work,
        total_regen_energy_available_at_battery_kwh: regen_available_total,
        total_regen_energy_captured_kwh: regen_captured_total,
        total_regen_energy_curtailed_kwh: regen_curtailed_total,
        net_battery_energy_demand_kwh: net_demand,
        energy_margin_vs_usable_capacity_kwh: energy_margin,
        total_unserved_energy_kwh: unserved_total,
        maximum_segment_average_traction_power_kw: max_avg_traction_power,
        segments: results,
        findings,
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_catalog() -> Catalog {
        Catalog {
            schema_version: "0.1.0".to_string(),
            catalog_id: "EV-PLATFORM-001".to_string(),
            designs: vec![Design {
                design_id: "sample".to_string(),
                mass: Mass {
                    base_curb_mass_kg: 1_000.0,
                    installed_module_mass_kg: 100.0,
                    target_payload_kg: 100.0,
                    maximum_gross_mass_kg: 1_500.0,
                },
                battery: Battery {
                    nominal_pack_kwh: 50.0,
                    state_of_health_fraction: 1.0,
                    temperature_capacity_fraction: 1.0,
                    reserve_fraction: 0.2,
                    rated_peak_power_kw: 100.0,
                },
            }],
        }
    }

    fn sample_scenario(segment: Segment) -> Scenario {
        Scenario {
            scenario_id: "test-route".to_string(),
            label: "synthetic route".to_string(),
            design_id: "sample".to_string(),
            evidence_class: "synthetic".to_string(),
            evidence_reference: "synthetic".to_string(),
            evidence_note: "test only".to_string(),
            mission_functional_unit: "test tasks".to_string(),
            initial_usable_soc_fraction: 1.0,
            auxiliary_load_kw: 0.0,
            nontravel_duration_hours: 0.0,
            nontravel_auxiliary_load_kw: 0.0,
            declared_peak_battery_power_kw: 50.0,
            rotating_mass_factor: 1.05,
            segments: vec![segment],
        }
    }

    fn flat_segment() -> Segment {
        Segment {
            segment_id: "flat".to_string(),
            distance_m: 1_000.0,
            duration_s: 100.0,
            initial_speed_mps: 10.0,
            final_speed_mps: 10.0,
            grade_fraction: 0.0,
            rolling_resistance_coefficient: 0.01,
            drag_area_m2: 0.6,
            air_density_kg_per_m3: 1.2,
            drivetrain_efficiency_fraction: 0.9,
            regenerative_efficiency_fraction: 0.7,
            max_regen_battery_power_kw: 20.0,
            work_energy_demand_kwh: 0.0,
        }
    }

    #[test]
    fn flat_road_consumes_positive_traction_energy() {
        let scenario = sample_scenario(flat_segment());
        let report = simulate(&scenario, &sample_catalog().designs[0]);
        assert!(report.total_traction_battery_energy_kwh > 0.0);
        assert_eq!(report.total_regen_energy_captured_kwh, 0.0);
    }

    #[test]
    fn uphill_grade_increases_road_work() {
        let flat = sample_scenario(flat_segment());
        let mut uphill_seg = flat_segment();
        uphill_seg.grade_fraction = 0.1;
        let uphill = sample_scenario(uphill_seg);
        let flat_result = simulate(&flat, &sample_catalog().designs[0]);
        let hill_result = simulate(&uphill, &sample_catalog().designs[0]);
        assert!(
            hill_result.total_traction_battery_energy_kwh
                > flat_result.total_traction_battery_energy_kwh
        );
    }

    #[test]
    fn downhill_braking_energy_is_capped_by_regen_power() {
        let mut segment = flat_segment();
        segment.grade_fraction = -0.2;
        segment.max_regen_battery_power_kw = 0.01;
        let mut scenario = sample_scenario(segment);
        scenario.initial_usable_soc_fraction = 0.5;
        let catalog = sample_catalog();
        let report = simulate(&scenario, &catalog.designs[0]);
        assert!(report.total_regen_energy_available_at_battery_kwh > 0.0);
        assert!(report.total_regen_energy_captured_kwh > 0.0);
        assert!(report.total_regen_energy_captured_kwh <= 0.01 * (100.0 / 3600.0) + 1e-9);
        assert!(report.total_regen_energy_curtailed_kwh > 0.0);
    }

    #[test]
    fn full_usable_pack_curtailed_by_regen_headroom() {
        let mut segment = flat_segment();
        segment.grade_fraction = -0.2;
        let scenario = sample_scenario(segment);
        let catalog = sample_catalog();
        let report = simulate(&scenario, &catalog.designs[0]);
        assert!(report.total_regen_energy_available_at_battery_kwh > 0.0);
        assert_eq!(report.total_regen_energy_captured_kwh, 0.0);
        assert!(report.total_regen_energy_curtailed_kwh > 0.0);
    }

    #[test]
    fn auxiliary_and_work_tool_energy_are_accounted() {
        let mut segment = flat_segment();
        segment.work_energy_demand_kwh = 4.0;
        let mut scenario = sample_scenario(segment);
        scenario.auxiliary_load_kw = 2.0;
        let report = simulate(&scenario, &sample_catalog().designs[0]);
        assert!((report.total_auxiliary_energy_kwh - (2.0 * 100.0 / 3600.0)).abs() < 1e-9);
        assert!((report.total_declared_work_energy_kwh - 4.0).abs() < 1e-9);
    }

    #[test]
    fn state_of_charge_cannot_hide_an_early_energy_shortfall() {
        let mut segment = flat_segment();
        segment.work_energy_demand_kwh = 80.0;
        let mut scenario = sample_scenario(segment);
        scenario.initial_usable_soc_fraction = 0.25;
        let report = simulate(&scenario, &sample_catalog().designs[0]);
        assert!(report.total_unserved_energy_kwh > 0.0);
        assert_eq!(report.status, Status::Blocked);
    }

    #[test]
    fn stationary_work_duration_is_not_counted_as_travel() {
        let mut stationary = flat_segment();
        stationary.distance_m = 0.0;
        stationary.duration_s = 7_200.0;
        stationary.initial_speed_mps = 0.0;
        stationary.final_speed_mps = 0.0;
        stationary.work_energy_demand_kwh = 1.0;
        let mut scenario = sample_scenario(stationary);
        scenario.segments.push(flat_segment());
        let catalog = sample_catalog();
        let report = simulate(&scenario, &catalog.designs[0]);
        assert!((report.total_travel_hours - 100.0 / 3600.0).abs() < 1e-9);
        assert!((report.total_stationary_segment_hours - 2.0).abs() < 1e-9);
        assert!((report.total_profiled_segment_hours - (2.0 + 100.0 / 3600.0)).abs() < 1e-9);
    }

    #[test]
    fn duplicate_segments_are_rejected() {
        let segment = flat_segment();
        let mut scenario = sample_scenario(segment.clone());
        scenario.segments.push(segment);
        let corpus = ProfileCorpus {
            schema_version: "0.2.0".to_string(),
            profile_set_id: "EV-MISSION-PHYSICS-001".to_string(),
            claim_ceiling: "test".to_string(),
            scenarios: vec![scenario],
        };
        assert!(validate_inputs(&sample_catalog(), &corpus).is_err());
    }

    #[test]
    fn input_order_does_not_change_sorted_report() {
        let mut a = sample_scenario(flat_segment());
        a.scenario_id = "z-last".to_string();
        let mut b = sample_scenario(flat_segment());
        b.scenario_id = "a-first".to_string();
        let catalog = sample_catalog();
        let design_a = &catalog.designs[0];
        let result_a = simulate(&a, design_a);
        let result_b = simulate(&b, design_a);
        let mut left = vec![result_a.clone(), result_b.clone()];
        let mut right = vec![result_b, result_a];
        left.sort_by(|x, y| x.scenario_id.cmp(&y.scenario_id));
        right.sort_by(|x, y| x.scenario_id.cmp(&y.scenario_id));
        assert_eq!(left, right);
        assert_eq!(left[0].scenario_id, "a-first");
    }
}
