use serde::{Deserialize, Serialize};
use std::{env, fs, process};

const CLAIM_CEILING: &str = "Synthetic or externally supplied duty-cycle screening only; not a vehicle simulation, battery qualification, grid study, safety approval, cost result, or field-performance claim.";

#[derive(Debug, Clone, Deserialize)]
struct Benchmark {
    schema_version: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
struct Scenario {
    scenario_id: String,
    label: String,
    evidence_class: Option<String>,
    demand_kwh_per_shift: Option<f64>,
    pack_nominal_kwh: Option<f64>,
    state_of_health_fraction: Option<f64>,
    temperature_capacity_fraction: Option<f64>,
    reserve_fraction: Option<f64>,
    charger_power_kw: Option<f64>,
    charger_efficiency_fraction: Option<f64>,
    charge_window_hours: Option<f64>,
    site_charge_power_budget_kw: Option<f64>,
    simultaneous_chargers: Option<u32>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Status {
    FeasibleUnderDeclaredScreen,
    NotFeasibleUnderDeclaredScreen,
    Indeterminate,
    InvalidInput,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioResult {
    scenario_id: String,
    label: String,
    evidence_class: Option<String>,
    status: Status,
    usable_pack_energy_kwh: Option<f64>,
    required_grid_energy_kwh: Option<f64>,
    effective_charging_power_kw: Option<f64>,
    recharge_hours: Option<f64>,
    missing_fields: Vec<String>,
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

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/rural-industrial-electrification-evaluator/fixtures/benchmark_v0_1.json".to_string()
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
            eprintln!("invalid benchmark JSON: {error}");
            process::exit(2);
        }
    };

    if benchmark.schema_version != "0.1.0" {
        eprintln!("unsupported schema_version: {}", benchmark.schema_version);
        process::exit(2);
    }

    let report = Report {
        schema_version: benchmark.schema_version,
        method: "per-shift battery-energy and site-charging envelope screen",
        results: benchmark.scenarios.iter().map(evaluate).collect(),
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

fn evaluate(s: &Scenario) -> ScenarioResult {
    let mut findings = Vec::new();
    let mut missing = Vec::new();

    for (name, value) in [
        ("demand_kwh_per_shift", s.demand_kwh_per_shift),
        ("pack_nominal_kwh", s.pack_nominal_kwh),
        ("state_of_health_fraction", s.state_of_health_fraction),
        (
            "temperature_capacity_fraction",
            s.temperature_capacity_fraction,
        ),
        ("reserve_fraction", s.reserve_fraction),
        ("charger_power_kw", s.charger_power_kw),
        ("charger_efficiency_fraction", s.charger_efficiency_fraction),
        ("charge_window_hours", s.charge_window_hours),
        ("site_charge_power_budget_kw", s.site_charge_power_budget_kw),
    ] {
        if value.is_none() {
            missing.push(name.to_string());
        }
    }
    if s.simultaneous_chargers.is_none() {
        missing.push("simultaneous_chargers".to_string());
    }
    if s.evidence_class.is_none() {
        missing.push("evidence_class".to_string());
    }

    if let Some(error) = validate_present_values(s) {
        findings.push(error);
        return result(
            s,
            Status::InvalidInput,
            None,
            None,
            None,
            None,
            missing,
            findings,
        );
    }

    let usable = match (
        s.pack_nominal_kwh,
        s.state_of_health_fraction,
        s.temperature_capacity_fraction,
        s.reserve_fraction,
    ) {
        (Some(pack), Some(soh), Some(temp), Some(reserve)) => {
            Some(pack * soh * temp * (1.0 - reserve))
        }
        _ => None,
    };

    let grid_energy = match (s.demand_kwh_per_shift, s.charger_efficiency_fraction) {
        (Some(demand), Some(efficiency)) => Some(demand / efficiency),
        _ => None,
    };

    let effective_power = match (
        s.charger_power_kw,
        s.site_charge_power_budget_kw,
        s.simultaneous_chargers,
    ) {
        (Some(charger), Some(site_budget), Some(concurrent)) if concurrent > 0 => {
            Some(charger.min(site_budget / f64::from(concurrent)))
        }
        _ => None,
    };

    let recharge_hours = match (grid_energy, effective_power) {
        (Some(energy), Some(power)) if power > 0.0 => Some(energy / power),
        _ => None,
    };

    if let (Some(demand), Some(usable_energy)) = (s.demand_kwh_per_shift, usable) {
        if demand > usable_energy + 1e-9 {
            findings.push("shift_energy_exceeds_derated_usable_pack_energy".to_string());
        }
    }

    if let Some(power) = effective_power {
        if power <= 0.0 {
            findings.push("site_power_budget_provides_no_charging_capacity".to_string());
        }
    }

    if let (Some(hours), Some(window)) = (recharge_hours, s.charge_window_hours) {
        if hours > window + 1e-9 {
            findings.push("recharge_time_exceeds_available_charge_window".to_string());
        }
    }

    let status = if !findings.is_empty() {
        Status::NotFeasibleUnderDeclaredScreen
    } else if !missing.is_empty() {
        findings.push("required_inputs_missing; candidate_is_not_rankable".to_string());
        Status::Indeterminate
    } else {
        Status::FeasibleUnderDeclaredScreen
    };

    result(
        s,
        status,
        usable,
        grid_energy,
        effective_power,
        recharge_hours,
        missing,
        findings,
    )
}

fn validate_present_values(s: &Scenario) -> Option<String> {
    let positive = [
        ("demand_kwh_per_shift", s.demand_kwh_per_shift),
        ("pack_nominal_kwh", s.pack_nominal_kwh),
        ("charger_power_kw", s.charger_power_kw),
        ("charge_window_hours", s.charge_window_hours),
    ];
    for (name, value) in positive {
        if let Some(number) = value {
            if !number.is_finite() || number <= 0.0 {
                return Some(format!("{name}_must_be_finite_and_positive"));
            }
        }
    }

    let nonnegative = [("site_charge_power_budget_kw", s.site_charge_power_budget_kw)];
    for (name, value) in nonnegative {
        if let Some(number) = value {
            if !number.is_finite() || number < 0.0 {
                return Some(format!("{name}_must_be_finite_and_nonnegative"));
            }
        }
    }

    for (name, value) in [
        ("state_of_health_fraction", s.state_of_health_fraction),
        (
            "temperature_capacity_fraction",
            s.temperature_capacity_fraction,
        ),
        ("charger_efficiency_fraction", s.charger_efficiency_fraction),
    ] {
        if let Some(number) = value {
            if !number.is_finite() || number <= 0.0 || number > 1.0 {
                return Some(format!("{name}_must_be_in_(0,1]"));
            }
        }
    }

    if let Some(reserve) = s.reserve_fraction {
        if !reserve.is_finite() || !(0.0..1.0).contains(&reserve) {
            return Some("reserve_fraction_must_be_in_[0,1)".to_string());
        }
    }

    if matches!(s.simultaneous_chargers, Some(0)) {
        return Some("simultaneous_chargers_must_be_positive".to_string());
    }

    if let Some(class) = &s.evidence_class {
        if ![
            "synthetic",
            "measured",
            "manufacturer",
            "literature",
            "estimate",
        ]
        .contains(&class.as_str())
        {
            return Some("unsupported_evidence_class".to_string());
        }
    }

    None
}

fn result(
    s: &Scenario,
    status: Status,
    usable: Option<f64>,
    grid_energy: Option<f64>,
    effective_power: Option<f64>,
    recharge_hours: Option<f64>,
    missing_fields: Vec<String>,
    findings: Vec<String>,
) -> ScenarioResult {
    ScenarioResult {
        scenario_id: s.scenario_id.clone(),
        label: s.label.clone(),
        evidence_class: s.evidence_class.clone(),
        status,
        usable_pack_energy_kwh: usable,
        required_grid_energy_kwh: grid_energy,
        effective_charging_power_kw: effective_power,
        recharge_hours,
        missing_fields,
        findings,
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Scenario {
        Scenario {
            scenario_id: "test".into(),
            label: "test fixture".into(),
            evidence_class: Some("synthetic".into()),
            demand_kwh_per_shift: Some(20.0),
            pack_nominal_kwh: Some(80.0),
            state_of_health_fraction: Some(0.9),
            temperature_capacity_fraction: Some(0.9),
            reserve_fraction: Some(0.15),
            charger_power_kw: Some(11.0),
            charger_efficiency_fraction: Some(0.92),
            charge_window_hours: Some(8.0),
            site_charge_power_budget_kw: Some(22.0),
            simultaneous_chargers: Some(2),
        }
    }

    #[test]
    fn accepts_candidate_inside_declared_energy_and_charge_envelope() {
        let result = evaluate(&base());
        assert_eq!(result.status, Status::FeasibleUnderDeclaredScreen);
        assert!((result.usable_pack_energy_kwh.unwrap() - 55.08).abs() < 1e-9);
        assert!((result.required_grid_energy_kwh.unwrap() - (20.0 / 0.92)).abs() < 1e-9);
        assert!(result.recharge_hours.unwrap() < 8.0);
    }

    #[test]
    fn rejects_insufficient_usable_pack_energy() {
        let mut scenario = base();
        scenario.demand_kwh_per_shift = Some(100.0);
        let result = evaluate(&scenario);
        assert_eq!(result.status, Status::NotFeasibleUnderDeclaredScreen);
        assert!(
            result
                .findings
                .iter()
                .any(|f| f == "shift_energy_exceeds_derated_usable_pack_energy")
        );
    }

    #[test]
    fn rejects_insufficient_site_charge_window() {
        let mut scenario = base();
        scenario.charge_window_hours = Some(0.25);
        let result = evaluate(&scenario);
        assert_eq!(result.status, Status::NotFeasibleUnderDeclaredScreen);
        assert!(
            result
                .findings
                .iter()
                .any(|f| f == "recharge_time_exceeds_available_charge_window")
        );
    }

    #[test]
    fn missing_grid_capacity_is_indeterminate_not_zero_or_pass() {
        let mut scenario = base();
        scenario.site_charge_power_budget_kw = None;
        let result = evaluate(&scenario);
        assert_eq!(result.status, Status::Indeterminate);
        assert!(
            result
                .missing_fields
                .iter()
                .any(|f| f == "site_charge_power_budget_kw")
        );
        assert!(result.effective_charging_power_kw.is_none());
    }

    #[test]
    fn zero_grid_capacity_fails_closed() {
        let mut scenario = base();
        scenario.site_charge_power_budget_kw = Some(0.0);
        let result = evaluate(&scenario);
        assert_eq!(result.status, Status::NotFeasibleUnderDeclaredScreen);
        assert!(
            result
                .findings
                .iter()
                .any(|f| f == "site_power_budget_provides_no_charging_capacity")
        );
    }

    #[test]
    fn invalid_fraction_is_rejected() {
        let mut scenario = base();
        scenario.charger_efficiency_fraction = Some(1.2);
        let result = evaluate(&scenario);
        assert_eq!(result.status, Status::InvalidInput);
    }

    #[test]
    fn zero_concurrent_chargers_is_rejected() {
        let mut scenario = base();
        scenario.simultaneous_chargers = Some(0);
        let result = evaluate(&scenario);
        assert_eq!(result.status, Status::InvalidInput);
    }

    #[test]
    fn unknown_evidence_class_is_rejected() {
        let mut scenario = base();
        scenario.evidence_class = Some("best_guess".into());
        let result = evaluate(&scenario);
        assert_eq!(result.status, Status::InvalidInput);
    }
}
