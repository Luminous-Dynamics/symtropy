//! Deterministic stress-scenario screening for candidate cruise-energy estimates.
//!
//! Scenarios are explicit perturbations, not probability distributions or validated operating
//! envelopes. Every result remains screening_only; this module does not certify an aircraft.

use std::collections::HashSet;

use crate::{Case, ResultRow, MODEL_VERSION};

pub const STRESS_INPUT_HEADER: &str = "scenario_id,scenario_basis,source_ref,airframe_mass_factor,cd0_factor,cruise_speed_factor,air_density_factor,propulsive_efficiency_factor,battery_energy_factor,route_distance_factor,assumed_max_lift_coefficient_factor";
pub const STRESS_OUTPUT_HEADER: &str = "case_id,scenario_id,scenario_basis,evidence_level,total_mass_kg,required_lift_coefficient,estimated_drag_coefficient,estimated_drag_n,estimated_lift_to_drag,route_energy_wh,usable_battery_energy_wh,energy_margin_wh,lift_margin_coefficient,lift_screen_pass,energy_screen_pass,scenario_screen_pass";

#[derive(Debug, Clone, PartialEq)]
pub struct StressScenario {
    pub scenario_id: String,
    pub scenario_basis: String,
    pub source_ref: String,
    pub airframe_mass_factor: f64,
    pub cd0_factor: f64,
    pub cruise_speed_factor: f64,
    pub air_density_factor: f64,
    pub propulsive_efficiency_factor: f64,
    pub battery_energy_factor: f64,
    pub route_distance_factor: f64,
    pub assumed_max_lift_coefficient_factor: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StressResult {
    pub scenario_id: String,
    pub scenario_basis: String,
    pub total_mass_kg: f64,
    pub required_lift_coefficient: f64,
    pub estimated_drag_coefficient: f64,
    pub estimated_drag_n: f64,
    pub estimated_lift_to_drag: f64,
    pub route_energy_wh: f64,
    pub usable_battery_energy_wh: f64,
    pub energy_margin_wh: f64,
    pub lift_margin_coefficient: f64,
    pub lift_screen_pass: bool,
    pub energy_screen_pass: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StressReport {
    pub model_version: &'static str,
    pub evidence_level: &'static str,
    pub case_id: String,
    pub scenario_basis: String,
    pub scenario_count: usize,
    pub passed_count: usize,
    pub lift_failure_count: usize,
    pub energy_failure_count: usize,
    pub worst_energy_scenario: String,
    pub worst_energy_margin_scenario: String,
    pub lowest_lift_margin_scenario: String,
    pub worst_energy_margin_wh: f64,
    pub lowest_lift_margin_coefficient: f64,
    pub robust_screen_pass: bool,
    pub results: Vec<StressResult>,
}

impl StressScenario {
    pub fn validate(&self) -> Result<(), String> {
        if self.scenario_id.trim().is_empty() {
            return Err("scenario_id must not be empty".to_owned());
        }
        if self.scenario_basis != "synthetic_stress" {
            return Err(format!(
                "scenario {}: v0.1 accepts synthetic_stress only; do not mislabel a stress set as a measured envelope",
                self.scenario_id
            ));
        }
        if self.source_ref.trim().is_empty() {
            return Err(format!("scenario {}: source_ref must not be empty", self.scenario_id));
        }
        for (name, value) in [
            ("airframe_mass_factor", self.airframe_mass_factor),
            ("cd0_factor", self.cd0_factor),
            ("cruise_speed_factor", self.cruise_speed_factor),
            ("air_density_factor", self.air_density_factor),
            ("propulsive_efficiency_factor", self.propulsive_efficiency_factor),
            ("battery_energy_factor", self.battery_energy_factor),
            ("route_distance_factor", self.route_distance_factor),
            (
                "assumed_max_lift_coefficient_factor",
                self.assumed_max_lift_coefficient_factor,
            ),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!(
                    "scenario {}: {name} must be finite and greater than zero",
                    self.scenario_id
                ));
            }
        }
        Ok(())
    }

    fn apply(&self, case: &Case) -> Result<Case, String> {
        self.validate()?;
        let mut changed = case.clone();
        changed.airframe_mass_kg *= self.airframe_mass_factor;
        changed.cd0 *= self.cd0_factor;
        changed.cruise_speed_m_s *= self.cruise_speed_factor;
        changed.air_density_kg_m3 *= self.air_density_factor;
        changed.propulsive_efficiency *= self.propulsive_efficiency_factor;
        changed.battery_energy_wh *= self.battery_energy_factor;
        changed.route_distance_km *= self.route_distance_factor;
        changed.assumed_max_lift_coefficient *= self.assumed_max_lift_coefficient_factor;
        changed.validate().map_err(|error| {
            format!(
                "scenario {} makes candidate {} invalid: {error}",
                self.scenario_id, case.case_id
            )
        })?;
        Ok(changed)
    }
}

pub fn run_stress_suite(case: &Case, scenarios: &[StressScenario]) -> Result<StressReport, String> {
    case.validate()?;
    if scenarios.is_empty() {
        return Err("stress suite must contain at least one scenario".to_owned());
    }

    let mut seen = HashSet::new();
    let mut results = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        scenario.validate()?;
        if !seen.insert(scenario.scenario_id.clone()) {
            return Err(format!("duplicate stress scenario ID: {}", scenario.scenario_id));
        }

        let changed = scenario.apply(case)?;
        let row: ResultRow = changed.evaluate()?;
        results.push(StressResult {
            scenario_id: scenario.scenario_id.clone(),
            scenario_basis: scenario.scenario_basis.clone(),
            total_mass_kg: row.total_mass_kg,
            required_lift_coefficient: row.required_lift_coefficient,
            estimated_drag_coefficient: row.estimated_drag_coefficient,
            estimated_drag_n: row.estimated_drag_n,
            estimated_lift_to_drag: row.estimated_lift_to_drag,
            route_energy_wh: row.route_energy_wh,
            usable_battery_energy_wh: row.usable_battery_energy_wh,
            energy_margin_wh: row.energy_margin_wh,
            lift_margin_coefficient: changed.assumed_max_lift_coefficient
                - row.required_lift_coefficient,
            lift_screen_pass: row.lift_screen_pass,
            energy_screen_pass: row.energy_screen_pass,
        });
    }

    let passed_count = results
        .iter()
        .filter(|row| row.lift_screen_pass && row.energy_screen_pass)
        .count();
    let lift_failure_count = results.iter().filter(|row| !row.lift_screen_pass).count();
    let energy_failure_count = results.iter().filter(|row| !row.energy_screen_pass).count();
    let worst_energy = results
        .iter()
        .max_by(|a, b| a.route_energy_wh.total_cmp(&b.route_energy_wh))
        .expect("non-empty scenarios");
    let worst_margin = results
        .iter()
        .min_by(|a, b| a.energy_margin_wh.total_cmp(&b.energy_margin_wh))
        .expect("non-empty scenarios");
    let lowest_lift_margin = results
        .iter()
        .min_by(|a, b| a.lift_margin_coefficient.total_cmp(&b.lift_margin_coefficient))
        .expect("non-empty scenarios");

    Ok(StressReport {
        model_version: MODEL_VERSION,
        evidence_level: "screening_only",
        case_id: case.case_id.clone(),
        scenario_basis: "synthetic_stress_not_probability".to_owned(),
        scenario_count: results.len(),
        passed_count,
        lift_failure_count,
        energy_failure_count,
        worst_energy_scenario: worst_energy.scenario_id.clone(),
        worst_energy_margin_scenario: worst_margin.scenario_id.clone(),
        lowest_lift_margin_scenario: lowest_lift_margin.scenario_id.clone(),
        worst_energy_margin_wh: worst_margin.energy_margin_wh,
        lowest_lift_margin_coefficient: lowest_lift_margin.lift_margin_coefficient,
        robust_screen_pass: passed_count == results.len(),
        results,
    })
}

pub fn parse_stress_scenarios_csv(input: &str) -> Result<Vec<StressScenario>, String> {
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| "stress-scenario CSV is empty".to_owned())?
        .trim_end_matches('\r');
    if header != STRESS_INPUT_HEADER {
        return Err(format!(
            "unexpected stress-scenario CSV header; expected exactly: {STRESS_INPUT_HEADER}"
        ));
    }

    let mut scenarios = Vec::new();
    let mut seen = HashSet::new();
    for (line_index, raw_line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        if line.contains('"') {
            return Err(format!("line {line_number}: quoted CSV fields are not supported"));
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() != STRESS_INPUT_HEADER.split(',').count() {
            return Err(format!(
                "line {line_number}: expected {} fields, got {}",
                STRESS_INPUT_HEADER.split(',').count(),
                fields.len()
            ));
        }
        let number = |index: usize, name: &str| -> Result<f64, String> {
            let value = fields[index]
                .parse::<f64>()
                .map_err(|_| format!("line {line_number}: {name} must be a decimal number"))?;
            if !value.is_finite() {
                return Err(format!("line {line_number}: {name} must be finite"));
            }
            Ok(value)
        };
        let scenario = StressScenario {
            scenario_id: fields[0].to_owned(),
            scenario_basis: fields[1].to_owned(),
            source_ref: fields[2].to_owned(),
            airframe_mass_factor: number(3, "airframe_mass_factor")?,
            cd0_factor: number(4, "cd0_factor")?,
            cruise_speed_factor: number(5, "cruise_speed_factor")?,
            air_density_factor: number(6, "air_density_factor")?,
            propulsive_efficiency_factor: number(7, "propulsive_efficiency_factor")?,
            battery_energy_factor: number(8, "battery_energy_factor")?,
            route_distance_factor: number(9, "route_distance_factor")?,
            assumed_max_lift_coefficient_factor: number(10, "assumed_max_lift_coefficient_factor")?,
        };
        scenario
            .validate()
            .map_err(|error| format!("line {line_number}: {error}"))?;
        if !seen.insert(scenario.scenario_id.clone()) {
            return Err(format!(
                "line {line_number}: duplicate scenario_id {}",
                scenario.scenario_id
            ));
        }
        scenarios.push(scenario);
    }
    if scenarios.is_empty() {
        return Err("stress-scenario CSV must contain at least one data row".to_owned());
    }
    Ok(scenarios)
}

pub fn stress_result_csv_row(case_id: &str, result: &StressResult) -> String {
    format!(
        "{},{},{},screening_only,{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{},{},{}",
        case_id,
        result.scenario_id,
        result.scenario_basis,
        // The factor values that produced these measurements remain in the input CSV.
        // The output is deliberately about scenario outcomes, not a claim of probability.
        result.total_mass_kg,
        result.required_lift_coefficient,
        result.estimated_drag_coefficient,
        result.estimated_drag_n,
        result.estimated_lift_to_drag,
        result.route_energy_wh,
        result.usable_battery_energy_wh,
        result.energy_margin_wh,
        result.lift_margin_coefficient,
        result.lift_screen_pass,
        result.energy_screen_pass,
        result.lift_screen_pass && result.energy_screen_pass,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Case;

    fn base_case() -> Case {
        Case {
            case_id: "baseline".to_owned(),
            parameter_basis: "synthetic".to_owned(),
            source_ref: "synthetic-fixture-v0.1".to_owned(),
            source_sha256: "-".to_owned(),
            airframe_mass_kg: 2.0,
            payload_mass_kg: 0.5,
            wing_area_m2: 0.55,
            aspect_ratio: 8.0,
            oswald_efficiency: 0.8,
            cd0: 0.035,
            cruise_speed_m_s: 15.0,
            air_density_kg_m3: 1.225,
            propulsive_efficiency: 0.65,
            battery_energy_wh: 160.0,
            reserve_fraction: 0.2,
            route_distance_km: 20.0,
            assumed_max_lift_coefficient: 1.2,
        }
    }

    fn scenario(id: &str) -> StressScenario {
        StressScenario {
            scenario_id: id.to_owned(),
            scenario_basis: "synthetic_stress".to_owned(),
            source_ref: "synthetic-stress-fixture-v0.1".to_owned(),
            airframe_mass_factor: 1.0,
            cd0_factor: 1.0,
            cruise_speed_factor: 1.0,
            air_density_factor: 1.0,
            propulsive_efficiency_factor: 1.0,
            battery_energy_factor: 1.0,
            route_distance_factor: 1.0,
            assumed_max_lift_coefficient_factor: 1.0,
        }
    }

    #[test]
    fn nominal_stress_scenario_matches_nominal_candidate() {
        let candidate = base_case();
        let nominal = candidate.evaluate().expect("candidate");
        let report = run_stress_suite(&candidate, &[scenario("nominal")]).expect("stress suite");
        assert!(report.robust_screen_pass);
        assert_eq!(report.scenario_basis, "synthetic_stress_not_probability");
        assert!((report.results[0].route_energy_wh - nominal.route_energy_wh).abs() < 1e-12);
        assert!((report.results[0].lift_margin_coefficient
            - (candidate.assumed_max_lift_coefficient - nominal.required_lift_coefficient))
            .abs()
            < 1e-12);
    }

    #[test]
    fn adverse_energy_scenario_is_reported_without_probability_claim() {
        let mut candidate = base_case();
        candidate.battery_energy_wh = 40.0;
        let mut nominal = scenario("nominal");
        let mut combined = scenario("combined_adverse");
        combined.airframe_mass_factor = 1.05;
        combined.cd0_factor = 1.10;
        combined.cruise_speed_factor = 1.05;
        combined.air_density_factor = 0.90;
        combined.propulsive_efficiency_factor = 0.90;
        combined.battery_energy_factor = 0.90;
        combined.route_distance_factor = 1.10;
        combined.assumed_max_lift_coefficient_factor = 0.90;
        nominal.source_ref = combined.source_ref.clone();
        let report = run_stress_suite(&candidate, &[nominal, combined]).expect("stress suite");
        assert!(!report.robust_screen_pass);
        assert!(report.energy_failure_count > 0);
        assert_eq!(report.worst_energy_scenario, "combined_adverse");
    }

    #[test]
    fn rejects_duplicate_ids_and_non_synthetic_scenario_basis() {
        let candidate = base_case();
        assert!(run_stress_suite(&candidate, &[scenario("dup"), scenario("dup")]).is_err());
        let mut unqualified = scenario("physical-envelope");
        unqualified.scenario_basis = "measured".to_owned();
        assert!(run_stress_suite(&candidate, &[unqualified]).is_err());
    }

    #[test]
    fn scenario_csv_rejects_bad_header_duplicate_ids_and_zero_factor() {
        let csv = format!(
            "{}\nnominal,synthetic_stress,fixture,1,1,1,1,1,1,1,1\n",
            STRESS_INPUT_HEADER
        );
        assert_eq!(parse_stress_scenarios_csv(&csv).expect("valid").len(), 1);
        assert!(parse_stress_scenarios_csv("scenario_id,wrong\nx,y\n").is_err());
        let dup = format!(
            "{}\nnominal,synthetic_stress,fixture,1,1,1,1,1,1,1,1\nnominal,synthetic_stress,fixture,1,1,1,1,1,1,1,1\n",
            STRESS_INPUT_HEADER
        );
        assert!(parse_stress_scenarios_csv(&dup).is_err());
        let zero = format!(
            "{}\ninvalid,synthetic_stress,fixture,0,1,1,1,1,1,1,1\n",
            STRESS_INPUT_HEADER
        );
        assert!(parse_stress_scenarios_csv(&zero).is_err());
    }
}
