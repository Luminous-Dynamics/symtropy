//! First-order, steady-level-flight screening for non-weaponized civil research aircraft.
//!
//! This crate is deliberately not a CFD solver, airworthiness tool, or flight controller.
//! It computes a transparent analytical estimate from caller-supplied parameters and labels
//! every result screening_only. Parameters and declared provenance are not independently
//! authenticated by this crate.

pub mod budget;
pub mod policy_benchmark;
pub mod cfd_admission;
pub mod comparison;
pub mod coupon;
pub mod dpw8_submission;
pub mod grid_convergence;
pub mod interval_calibration;
pub mod integrity;
pub mod reference_data;
pub mod robust_voi;
pub mod unsteady_spectrum;
pub mod robustness;
pub mod run_artifacts;
pub mod section_polar;
pub mod selective_risk;
pub mod voi;

use std::collections::HashSet;
use std::f64::consts::PI;

pub const MODEL_VERSION: &str = "civil-aero-screen/0.1.0";
pub const GRAVITY_M_S2: f64 = 9.806_65;
pub const INPUT_HEADER: &str = "case_id,parameter_basis,source_ref,source_sha256,airframe_mass_kg,payload_mass_kg,wing_area_m2,aspect_ratio,oswald_efficiency,cd0,cruise_speed_m_s,air_density_kg_m3,propulsive_efficiency,battery_energy_wh,reserve_fraction,route_distance_km,assumed_max_lift_coefficient";
pub const OUTPUT_HEADER: &str = "case_id,parameter_basis,source_ref,source_sha256,model_version,evidence_level,total_mass_kg,dynamic_pressure_pa,required_lift_coefficient,estimated_drag_coefficient,estimated_drag_n,estimated_lift_to_drag,route_energy_wh,usable_battery_energy_wh,energy_margin_wh,energy_wh_per_payload_km,lift_screen_pass,energy_screen_pass,combined_screen_pass";

#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub case_id: String,
    pub parameter_basis: String,
    pub source_ref: String,
    pub source_sha256: String,
    pub airframe_mass_kg: f64,
    pub payload_mass_kg: f64,
    pub wing_area_m2: f64,
    pub aspect_ratio: f64,
    pub oswald_efficiency: f64,
    pub cd0: f64,
    pub cruise_speed_m_s: f64,
    pub air_density_kg_m3: f64,
    pub propulsive_efficiency: f64,
    pub battery_energy_wh: f64,
    pub reserve_fraction: f64,
    pub route_distance_km: f64,
    pub assumed_max_lift_coefficient: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResultRow {
    pub case: Case,
    pub total_mass_kg: f64,
    pub dynamic_pressure_pa: f64,
    pub required_lift_coefficient: f64,
    pub estimated_drag_coefficient: f64,
    pub estimated_drag_n: f64,
    pub estimated_lift_to_drag: f64,
    pub route_energy_wh: f64,
    pub usable_battery_energy_wh: f64,
    pub energy_margin_wh: f64,
    pub energy_wh_per_payload_km: f64,
    pub lift_screen_pass: bool,
    pub energy_screen_pass: bool,
}

impl Case {
    pub fn validate(&self) -> Result<(), String> {
        if self.case_id.trim().is_empty() {
            return Err("case_id must not be empty".to_owned());
        }
        if !matches!(
            self.parameter_basis.as_str(),
            "synthetic" | "estimate" | "measured"
        ) {
            return Err(format!(
                "case {}: parameter_basis must be synthetic, estimate, or measured",
                self.case_id
            ));
        }
        if self.source_ref.trim().is_empty() {
            return Err(format!(
                "case {}: source_ref must not be empty",
                self.case_id
            ));
        }
        if self.parameter_basis == "synthetic" {
            if self.source_sha256 != "-" {
                return Err(format!(
                    "case {}: synthetic rows must use '-' for source_sha256",
                    self.case_id
                ));
            }
        } else if !is_sha256_hex(&self.source_sha256) {
            return Err(format!(
                "case {}: estimate/measured rows require a 64-character hexadecimal source_sha256",
                self.case_id
            ));
        }

        for (name, value) in [
            ("airframe_mass_kg", self.airframe_mass_kg),
            ("payload_mass_kg", self.payload_mass_kg),
            ("wing_area_m2", self.wing_area_m2),
            ("aspect_ratio", self.aspect_ratio),
            ("oswald_efficiency", self.oswald_efficiency),
            ("cd0", self.cd0),
            ("cruise_speed_m_s", self.cruise_speed_m_s),
            ("air_density_kg_m3", self.air_density_kg_m3),
            ("propulsive_efficiency", self.propulsive_efficiency),
            ("battery_energy_wh", self.battery_energy_wh),
            ("reserve_fraction", self.reserve_fraction),
            ("route_distance_km", self.route_distance_km),
            (
                "assumed_max_lift_coefficient",
                self.assumed_max_lift_coefficient,
            ),
        ] {
            if !value.is_finite() {
                return Err(format!("case {}: {name} must be finite", self.case_id));
            }
        }

        for (name, value) in [
            ("airframe_mass_kg", self.airframe_mass_kg),
            ("payload_mass_kg", self.payload_mass_kg),
            ("wing_area_m2", self.wing_area_m2),
            ("aspect_ratio", self.aspect_ratio),
            ("cd0", self.cd0),
            ("cruise_speed_m_s", self.cruise_speed_m_s),
            ("air_density_kg_m3", self.air_density_kg_m3),
            ("battery_energy_wh", self.battery_energy_wh),
            ("route_distance_km", self.route_distance_km),
            (
                "assumed_max_lift_coefficient",
                self.assumed_max_lift_coefficient,
            ),
        ] {
            if value <= 0.0 {
                return Err(format!("case {}: {name} must be > 0", self.case_id));
            }
        }
        if !(0.0 < self.oswald_efficiency && self.oswald_efficiency <= 1.0) {
            return Err(format!(
                "case {}: oswald_efficiency must be in (0, 1]",
                self.case_id
            ));
        }
        if !(0.0 < self.propulsive_efficiency && self.propulsive_efficiency <= 1.0) {
            return Err(format!(
                "case {}: propulsive_efficiency must be in (0, 1]",
                self.case_id
            ));
        }
        if !(0.0..1.0).contains(&self.reserve_fraction) {
            return Err(format!(
                "case {}: reserve_fraction must be in [0, 1)",
                self.case_id
            ));
        }
        Ok(())
    }

    pub fn evaluate(&self) -> Result<ResultRow, String> {
        self.validate()?;

        let total_mass_kg = self.airframe_mass_kg + self.payload_mass_kg;
        let dynamic_pressure_pa = 0.5 * self.air_density_kg_m3 * self.cruise_speed_m_s.powi(2);
        let required_lift_coefficient =
            total_mass_kg * GRAVITY_M_S2 / (dynamic_pressure_pa * self.wing_area_m2);
        let induced_drag_coefficient =
            required_lift_coefficient.powi(2) / (PI * self.oswald_efficiency * self.aspect_ratio);
        let estimated_drag_coefficient = self.cd0 + induced_drag_coefficient;
        let estimated_drag_n = dynamic_pressure_pa * self.wing_area_m2 * estimated_drag_coefficient;

        // For steady, level, still-air cruise: electrical energy = drag * distance / total efficiency.
        // N*m is joules; dividing by 3600 converts joules to watt-hours.
        let route_energy_wh = estimated_drag_n * (self.route_distance_km * 1000.0)
            / self.propulsive_efficiency
            / 3600.0;
        let usable_battery_energy_wh = self.battery_energy_wh * (1.0 - self.reserve_fraction);
        let energy_margin_wh = usable_battery_energy_wh - route_energy_wh;
        let energy_wh_per_payload_km =
            route_energy_wh / (self.payload_mass_kg * self.route_distance_km);
        let estimated_lift_to_drag = required_lift_coefficient / estimated_drag_coefficient;
        let lift_screen_pass = required_lift_coefficient < self.assumed_max_lift_coefficient;
        let energy_screen_pass = route_energy_wh <= usable_battery_energy_wh;

        let values = [
            total_mass_kg,
            dynamic_pressure_pa,
            required_lift_coefficient,
            estimated_drag_coefficient,
            estimated_drag_n,
            estimated_lift_to_drag,
            route_energy_wh,
            usable_battery_energy_wh,
            energy_margin_wh,
            energy_wh_per_payload_km,
        ];
        if values.iter().any(|v| !v.is_finite()) {
            return Err(format!(
                "case {}: calculation produced non-finite output",
                self.case_id
            ));
        }

        Ok(ResultRow {
            case: self.clone(),
            total_mass_kg,
            dynamic_pressure_pa,
            required_lift_coefficient,
            estimated_drag_coefficient,
            estimated_drag_n,
            estimated_lift_to_drag,
            route_energy_wh,
            usable_battery_energy_wh,
            energy_margin_wh,
            energy_wh_per_payload_km,
            lift_screen_pass,
            energy_screen_pass,
        })
    }
}

impl ResultRow {
    pub fn to_csv_row(&self) -> String {
        format!(
            "{},{},{},{},{},screening_only,{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{},{},{}",
            self.case.case_id,
            self.case.parameter_basis,
            self.case.source_ref,
            self.case.source_sha256,
            MODEL_VERSION,
            self.total_mass_kg,
            self.dynamic_pressure_pa,
            self.required_lift_coefficient,
            self.estimated_drag_coefficient,
            self.estimated_drag_n,
            self.estimated_lift_to_drag,
            self.route_energy_wh,
            self.usable_battery_energy_wh,
            self.energy_margin_wh,
            self.energy_wh_per_payload_km,
            self.lift_screen_pass,
            self.energy_screen_pass,
            self.lift_screen_pass && self.energy_screen_pass,
        )
    }
}

pub fn parse_cases_csv(input: &str) -> Result<Vec<Case>, String> {
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| "input CSV is empty".to_owned())?
        .trim_end_matches('\r');
    if header != INPUT_HEADER {
        return Err(format!(
            "unexpected CSV header; expected exactly: {INPUT_HEADER}"
        ));
    }

    let mut cases = Vec::new();
    let mut seen_ids = HashSet::new();
    for (line_index, raw_line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        if line.contains('"') {
            return Err(format!(
                "line {line_number}: quoted CSV fields are not supported; use comma-free values"
            ));
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() != INPUT_HEADER.split(',').count() {
            return Err(format!(
                "line {line_number}: expected {} fields, got {}",
                INPUT_HEADER.split(',').count(),
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
        let case = Case {
            case_id: fields[0].to_owned(),
            parameter_basis: fields[1].to_owned(),
            source_ref: fields[2].to_owned(),
            source_sha256: fields[3].to_owned(),
            airframe_mass_kg: number(4, "airframe_mass_kg")?,
            payload_mass_kg: number(5, "payload_mass_kg")?,
            wing_area_m2: number(6, "wing_area_m2")?,
            aspect_ratio: number(7, "aspect_ratio")?,
            oswald_efficiency: number(8, "oswald_efficiency")?,
            cd0: number(9, "cd0")?,
            cruise_speed_m_s: number(10, "cruise_speed_m_s")?,
            air_density_kg_m3: number(11, "air_density_kg_m3")?,
            propulsive_efficiency: number(12, "propulsive_efficiency")?,
            battery_energy_wh: number(13, "battery_energy_wh")?,
            reserve_fraction: number(14, "reserve_fraction")?,
            route_distance_km: number(15, "route_distance_km")?,
            assumed_max_lift_coefficient: number(16, "assumed_max_lift_coefficient")?,
        };
        case.validate()
            .map_err(|e| format!("line {line_number}: {e}"))?;
        if !seen_ids.insert(case.case_id.clone()) {
            return Err(format!(
                "line {line_number}: duplicate case_id {}",
                case.case_id
            ));
        }
        cases.push(case);
    }

    if cases.is_empty() {
        return Err("input must contain at least one data row".to_owned());
    }
    Ok(cases)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn computes_dimensionally_consistent_baseline() {
        let result = base_case().evaluate().expect("valid case");
        assert!((result.total_mass_kg - 2.5).abs() < 1e-12);
        assert!((result.dynamic_pressure_pa - 137.8125).abs() < 1e-10);
        assert!(result.required_lift_coefficient > 0.0);
        assert!(result.estimated_drag_coefficient > result.case.cd0);
        assert!(result.estimated_drag_n > 0.0);
        assert!((result.route_energy_wh - 26.045_244_747_108_647).abs() < 1e-9);
        assert!((result.estimated_lift_to_drag - 8.045_376_630_298_913).abs() < 1e-10);
        assert!(result.energy_wh_per_payload_km > 0.0);
        assert!(result.to_csv_row().contains("screening_only"));
    }

    #[test]
    fn greater_zero_lift_drag_increases_required_energy() {
        let baseline = base_case().evaluate().expect("valid baseline");
        let mut changed = base_case();
        changed.cd0 *= 1.1;
        let perturbed = changed.evaluate().expect("valid perturbed");
        assert!(perturbed.route_energy_wh > baseline.route_energy_wh);
    }

    #[test]
    fn greater_mass_increases_required_lift_and_energy() {
        let baseline = base_case().evaluate().expect("valid baseline");
        let mut changed = base_case();
        changed.airframe_mass_kg *= 1.1;
        let perturbed = changed.evaluate().expect("valid perturbed");
        assert!(perturbed.required_lift_coefficient > baseline.required_lift_coefficient);
        assert!(perturbed.route_energy_wh > baseline.route_energy_wh);
    }

    #[test]
    fn longer_route_increases_energy_linearly_for_same_cruise_state() {
        let baseline = base_case().evaluate().expect("valid baseline");
        let mut changed = base_case();
        changed.route_distance_km *= 2.0;
        let perturbed = changed.evaluate().expect("valid perturbed");
        assert!((perturbed.route_energy_wh / baseline.route_energy_wh - 2.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_nonphysical_efficiency_and_nonfinite_inputs() {
        let mut bad_eta = base_case();
        bad_eta.propulsive_efficiency = 1.01;
        assert!(bad_eta.evaluate().is_err());
        let mut bad_mass = base_case();
        bad_mass.airframe_mass_kg = f64::NAN;
        assert!(bad_mass.evaluate().is_err());
    }

    #[test]
    fn screen_flags_are_not_airworthiness_claims() {
        let mut case = base_case();
        case.battery_energy_wh = 1.0;
        let result = case
            .evaluate()
            .expect("valid input but insufficient battery");
        assert!(!result.energy_screen_pass);
        assert!(result.lift_screen_pass);
    }

    #[test]
    fn parser_fails_closed_on_header_mutation_duplicate_ids_and_unpinned_data() {
        let fixture = format!(
            "{}\nbaseline,synthetic,synthetic-fixture-v0.1,-,2,0.5,0.55,8,0.8,0.035,15,1.225,0.65,160,0.2,20,1.2\n",
            INPUT_HEADER
        );
        assert_eq!(parse_cases_csv(&fixture).expect("valid fixture").len(), 1);
        assert!(parse_cases_csv("case_id,wrong\nx,y\n").is_err());
        let duplicate = format!(
            "{}\nbaseline,synthetic,synthetic-fixture-v0.1,-,2,0.5,0.55,8,0.8,0.035,15,1.225,0.65,160,0.2,20,1.2\nbaseline,synthetic,synthetic-fixture-v0.1,-,2,0.5,0.55,8,0.8,0.035,15,1.225,0.65,160,0.2,20,1.2\n",
            INPUT_HEADER
        );
        assert!(parse_cases_csv(&duplicate).is_err());
        let unpinned = format!(
            "{}\nmeasured,measured,local-test,-,2,0.5,0.55,8,0.8,0.035,15,1.225,0.65,160,0.2,20,1.2\n",
            INPUT_HEADER
        );
        assert!(parse_cases_csv(&unpinned).is_err());
    }
}
