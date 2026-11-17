//! Kinetic-energy control-volume reference with explicit power boundaries.
//!
//! This is a mechanical kinetic-energy bookkeeping reference only. It does
//! not claim total-energy or thermodynamic closure, electrical recovery,
//! potential/elastic/field energy, or thermal storage.

use std::collections::{BTreeMap, HashSet};

pub const EXPECTED_CASE_COUNT: usize = 6;
pub const EXPECTED_PORT_COUNT: usize = 14;
pub const MASS_BALANCE_REL_TOLERANCE: f64 = 1.0e-12;
pub const MASS_BALANCE_ABS_TOLERANCE_KG_S: f64 = 1.0e-12;
pub const POWER_BALANCE_REL_TOLERANCE: f64 = 1.0e-12;
pub const POWER_BALANCE_ABS_TOLERANCE_W: f64 = 1.0e-9;
pub const MAX_PORT_COUNT_PER_CASE: usize = 1_024;
const EXPECTED_CASE_HEADER: &str = concat!(
    "case_id,frame_id,boundary_id,epoch_s,mass_accumulation_kg_s,",
    "kinetic_energy_accumulation_w,nonport_external_power_on_cv_w,",
    "boundary_work_on_cv_w,mechanical_dissipation_rate_w"
);
const EXPECTED_PORT_HEADER: &str = concat!(
    "case_id,port_id,direction,mass_flow_kg_s,",
    "velocity_x_m_s,velocity_y_m_s,velocity_z_m_s"
);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn norm_squared(self) -> f64 {
        self.dot(self)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub fn scale(self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowDirection {
    Inflow,
    Outflow,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnergyFluxPort {
    pub port_id: String,
    pub direction: FlowDirection,
    pub mass_flow_kg_s: f64,
    pub velocity_inertial_m_s: Vector3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KineticEnergyScenario {
    pub case_id: String,
    pub frame_id: String,
    pub boundary_id: String,
    pub epoch_s: f64,
    pub mass_accumulation_kg_s: f64,
    /// dK_CV/dt, positive when kinetic energy accumulates in the CV.
    pub kinetic_energy_accumulation_w: f64,
    /// Signed non-port external power applied to the declared mechanical CV.
    pub nonport_external_power_on_cv_w: f64,
    /// Signed boundary work rate applied to the declared mechanical CV.
    pub boundary_work_on_cv_w: f64,
    /// Non-negative mechanical-energy removal rate from this kinetic ledger.
    pub mechanical_dissipation_rate_w: f64,
    pub ports: Vec<EnergyFluxPort>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortEnergyContribution {
    pub port_id: String,
    pub direction: FlowDirection,
    pub mass_flow_kg_s: f64,
    pub specific_kinetic_energy_j_kg: f64,
    /// Signed contribution: outflow positive, inflow negative.
    pub signed_kinetic_energy_flow_w: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaseEvaluation {
    pub case_id: String,
    pub frame_id: String,
    pub boundary_id: String,
    pub epoch_s: f64,
    pub port_count: usize,
    pub mass_accumulation_kg_s: f64,
    pub mass_flow_in_kg_s: f64,
    pub mass_flow_out_kg_s: f64,
    pub mass_balance_residual_kg_s: f64,
    pub mass_balance_tolerance_kg_s: f64,
    pub kinetic_energy_accumulation_w: f64,
    pub port_kinetic_energy_flow_in_w: f64,
    pub port_kinetic_energy_flow_out_w: f64,
    pub net_advective_kinetic_energy_flux_w: f64,
    pub nonport_external_power_on_cv_w: f64,
    pub boundary_work_on_cv_w: f64,
    pub mechanical_dissipation_rate_w: f64,
    pub net_declared_power_on_cv_w: f64,
    pub power_balance_residual_w: f64,
    pub power_balance_tolerance_w: f64,
    pub port_contributions: Vec<PortEnergyContribution>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub cases: Vec<CaseEvaluation>,
    pub case_count: usize,
    pub port_count: usize,
    pub max_abs_mass_balance_residual_kg_s: f64,
    pub max_abs_power_balance_residual_w: f64,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn parse_number(value: &str, at: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("{at}: invalid numeric value {value:?}"))?;
    if !parsed.is_finite() {
        return Err(format!("{at}: non-finite numeric value"));
    }
    Ok(parsed)
}

fn parse_vector(fields: &[&str], at: &str) -> Result<Vector3, String> {
    if fields.len() != 3 {
        return Err(format!("{at}: expected three vector components"));
    }
    Ok(Vector3::new(
        parse_number(fields[0], &format!("{at}.x"))?,
        parse_number(fields[1], &format!("{at}.y"))?,
        parse_number(fields[2], &format!("{at}.z"))?,
    ))
}

pub fn parse_corpus(
    cases_csv: &str,
    ports_csv: &str,
) -> Result<Vec<KineticEnergyScenario>, String> {
    let mut case_lines = cases_csv.lines();
    let header = case_lines
        .next()
        .ok_or_else(|| "empty kinetic-energy scenario corpus".to_owned())?;
    if header != EXPECTED_CASE_HEADER {
        return Err("unexpected kinetic-energy scenario header".to_owned());
    }

    let mut scenarios = BTreeMap::new();
    for (line_index, line) in case_lines.enumerate() {
        if line.trim().is_empty() {
            return Err(format!("scenario line {}: blank row", line_index + 2));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 9 {
            return Err(format!(
                "scenario line {}: expected nine columns",
                line_index + 2
            ));
        }

        let case_id = fields[0].to_owned();
        let frame_id = fields[1].to_owned();
        let boundary_id = fields[2].to_owned();
        if !valid_identifier(&case_id)
            || !valid_identifier(&frame_id)
            || !valid_identifier(&boundary_id)
        {
            return Err(format!(
                "scenario line {}: invalid case, frame or boundary ID",
                line_index + 2
            ));
        }

        let epoch_s = parse_number(
            fields[3],
            &format!("scenario line {} epoch", line_index + 2),
        )?;
        let mass_accumulation_kg_s = parse_number(
            fields[4],
            &format!("scenario line {} mass accumulation", line_index + 2),
        )?;
        let kinetic_energy_accumulation_w = parse_number(
            fields[5],
            &format!("scenario line {} kinetic-energy accumulation", line_index + 2),
        )?;
        let nonport_external_power_on_cv_w = parse_number(
            fields[6],
            &format!("scenario line {} non-port external power", line_index + 2),
        )?;
        let boundary_work_on_cv_w = parse_number(
            fields[7],
            &format!("scenario line {} boundary work", line_index + 2),
        )?;
        let mechanical_dissipation_rate_w = parse_number(
            fields[8],
            &format!("scenario line {} mechanical dissipation", line_index + 2),
        )?;

        let scenario = KineticEnergyScenario {
            case_id: case_id.clone(),
            frame_id,
            boundary_id,
            epoch_s,
            mass_accumulation_kg_s,
            kinetic_energy_accumulation_w,
            nonport_external_power_on_cv_w,
            boundary_work_on_cv_w,
            mechanical_dissipation_rate_w,
            ports: Vec::new(),
        };
        if scenarios.insert(case_id.clone(), scenario).is_some() {
            return Err(format!(
                "scenario line {}: duplicate case ID {case_id}",
                line_index + 2
            ));
        }
    }

    if scenarios.len() != EXPECTED_CASE_COUNT {
        return Err(format!(
            "expected exactly {} scenarios, found {}",
            EXPECTED_CASE_COUNT,
            scenarios.len()
        ));
    }

    let mut port_lines = ports_csv.lines();
    let port_header = port_lines
        .next()
        .ok_or_else(|| "empty kinetic-energy port corpus".to_owned())?;
    if port_header != EXPECTED_PORT_HEADER {
        return Err("unexpected kinetic-energy port header".to_owned());
    }

    let mut port_count = 0usize;
    for (line_index, line) in port_lines.enumerate() {
        if line.trim().is_empty() {
            return Err(format!("port line {}: blank row", line_index + 2));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 7 {
            return Err(format!(
                "port line {}: expected seven columns",
                line_index + 2
            ));
        }

        let case_id = fields[0];
        let port_id = fields[1].to_owned();
        if !valid_identifier(&port_id) {
            return Err(format!("port line {}: invalid port ID", line_index + 2));
        }
        let direction = match fields[2] {
            "Inflow" => FlowDirection::Inflow,
            "Outflow" => FlowDirection::Outflow,
            other => {
                return Err(format!(
                    "port line {}: unsupported direction {other:?}",
                    line_index + 2
                ));
            }
        };
        let mass_flow_kg_s = parse_number(
            fields[3],
            &format!("port line {} mass flow", line_index + 2),
        )?;
        if mass_flow_kg_s < 0.0 {
            return Err(format!(
                "port line {}: negative mass-flow magnitude",
                line_index + 2
            ));
        }
        let velocity_inertial_m_s = parse_vector(
            &fields[4..7],
            &format!("port line {} inertial velocity", line_index + 2),
        )?;

        let scenario = scenarios
            .get_mut(case_id)
            .ok_or_else(|| format!("port line {}: unknown case {case_id}", line_index + 2))?;
        if scenario.ports.iter().any(|port| port.port_id == port_id) {
            return Err(format!(
                "port line {}: duplicate port ID {port_id} in case {case_id}",
                line_index + 2
            ));
        }
        if scenario.ports.len() >= MAX_PORT_COUNT_PER_CASE {
            return Err(format!("case {case_id}: port limit exceeded"));
        }
        scenario.ports.push(EnergyFluxPort {
            port_id,
            direction,
            mass_flow_kg_s,
            velocity_inertial_m_s,
        });
        port_count += 1;
    }

    if port_count != EXPECTED_PORT_COUNT {
        return Err(format!(
            "expected exactly {} ports, found {port_count}",
            EXPECTED_PORT_COUNT
        ));
    }
    for scenario in scenarios.values() {
        if scenario.ports.is_empty() {
            return Err(format!("case {} has no mass-flux ports", scenario.case_id));
        }
    }

    Ok(scenarios.into_values().collect())
}

pub fn evaluate_scenario(scenario: &KineticEnergyScenario) -> Result<CaseEvaluation, String> {
    if !valid_identifier(&scenario.case_id)
        || !valid_identifier(&scenario.frame_id)
        || !valid_identifier(&scenario.boundary_id)
    {
        return Err("invalid case, frame or boundary ID".to_owned());
    }
    if !scenario.epoch_s.is_finite()
        || !scenario.mass_accumulation_kg_s.is_finite()
        || !scenario.kinetic_energy_accumulation_w.is_finite()
        || !scenario.nonport_external_power_on_cv_w.is_finite()
        || !scenario.boundary_work_on_cv_w.is_finite()
        || !scenario.mechanical_dissipation_rate_w.is_finite()
    {
        return Err(format!("{}: non-finite scenario input", scenario.case_id));
    }
    if scenario.mechanical_dissipation_rate_w < 0.0 {
        return Err(format!(
            "{}: mechanical dissipation must be non-negative",
            scenario.case_id
        ));
    }
    if scenario.ports.is_empty() {
        return Err(format!(
            "{}: at least one mass-flux port is required",
            scenario.case_id
        ));
    }
    if scenario.ports.len() > MAX_PORT_COUNT_PER_CASE {
        return Err(format!("{}: port limit exceeded", scenario.case_id));
    }

    let mut ordered_ports: Vec<&EnergyFluxPort> = scenario.ports.iter().collect();
    ordered_ports.sort_by(|left, right| left.port_id.cmp(&right.port_id));
    let mut port_ids = HashSet::new();
    let mut mass_flow_in_kg_s = 0.0;
    let mut mass_flow_out_kg_s = 0.0;
    let mut port_kinetic_energy_flow_in_w = 0.0;
    let mut port_kinetic_energy_flow_out_w = 0.0;
    let mut port_contributions = Vec::with_capacity(scenario.ports.len());

    for port in ordered_ports {
        if !valid_identifier(&port.port_id) || !port_ids.insert(port.port_id.as_str()) {
            return Err(format!(
                "{}: invalid or duplicate port ID",
                scenario.case_id
            ));
        }
        if !port.mass_flow_kg_s.is_finite() || port.mass_flow_kg_s < 0.0 {
            return Err(format!(
                "{}: invalid mass-flow magnitude",
                scenario.case_id
            ));
        }
        if !port.velocity_inertial_m_s.is_finite() {
            return Err(format!(
                "{}: non-finite port velocity",
                scenario.case_id
            ));
        }

        let velocity_squared = port.velocity_inertial_m_s.norm_squared();
        let specific_kinetic_energy_j_kg = 0.5 * velocity_squared;
        let kinetic_energy_flow_w = port.mass_flow_kg_s * specific_kinetic_energy_j_kg;
        if !velocity_squared.is_finite()
            || !specific_kinetic_energy_j_kg.is_finite()
            || !kinetic_energy_flow_w.is_finite()
        {
            return Err(format!(
                "{}: non-finite derived port kinetic energy",
                scenario.case_id
            ));
        }

        let signed_kinetic_energy_flow_w = match port.direction {
            FlowDirection::Inflow => {
                mass_flow_in_kg_s += port.mass_flow_kg_s;
                port_kinetic_energy_flow_in_w += kinetic_energy_flow_w;
                -kinetic_energy_flow_w
            }
            FlowDirection::Outflow => {
                mass_flow_out_kg_s += port.mass_flow_kg_s;
                port_kinetic_energy_flow_out_w += kinetic_energy_flow_w;
                kinetic_energy_flow_w
            }
        };
        port_contributions.push(PortEnergyContribution {
            port_id: port.port_id.clone(),
            direction: port.direction,
            mass_flow_kg_s: port.mass_flow_kg_s,
            specific_kinetic_energy_j_kg,
            signed_kinetic_energy_flow_w,
        });
    }

    let mass_balance_residual_kg_s =
        scenario.mass_accumulation_kg_s - (mass_flow_in_kg_s - mass_flow_out_kg_s);
    let mass_balance_scale = scenario
        .mass_accumulation_kg_s
        .abs()
        .max(mass_flow_in_kg_s + mass_flow_out_kg_s)
        .max(1.0);
    let mass_balance_tolerance_kg_s =
        MASS_BALANCE_ABS_TOLERANCE_KG_S.max(MASS_BALANCE_REL_TOLERANCE * mass_balance_scale);
    if !mass_flow_in_kg_s.is_finite()
        || !mass_flow_out_kg_s.is_finite()
        || !mass_balance_residual_kg_s.is_finite()
        || !mass_balance_tolerance_kg_s.is_finite()
        || mass_balance_residual_kg_s.abs() > mass_balance_tolerance_kg_s
    {
        return Err(format!(
            "{}: mass balance mismatch or non-finite accumulation",
            scenario.case_id
        ));
    }

    let net_advective_kinetic_energy_flux_w =
        port_kinetic_energy_flow_out_w - port_kinetic_energy_flow_in_w;
    let kinetic_energy_balance_left_w =
        scenario.kinetic_energy_accumulation_w + net_advective_kinetic_energy_flux_w;
    let net_declared_power_on_cv_w = scenario.nonport_external_power_on_cv_w
        + scenario.boundary_work_on_cv_w
        - scenario.mechanical_dissipation_rate_w;
    let power_balance_residual_w = kinetic_energy_balance_left_w - net_declared_power_on_cv_w;
    let power_balance_scale = scenario
        .kinetic_energy_accumulation_w
        .abs()
        .max(net_advective_kinetic_energy_flux_w.abs())
        .max(scenario.nonport_external_power_on_cv_w.abs())
        .max(scenario.boundary_work_on_cv_w.abs())
        .max(scenario.mechanical_dissipation_rate_w)
        .max(1.0);
    let power_balance_tolerance_w =
        POWER_BALANCE_ABS_TOLERANCE_W.max(POWER_BALANCE_REL_TOLERANCE * power_balance_scale);

    let derived = [
        net_advective_kinetic_energy_flux_w,
        kinetic_energy_balance_left_w,
        net_declared_power_on_cv_w,
        power_balance_residual_w,
        power_balance_tolerance_w,
    ];
    if derived.iter().any(|value| !value.is_finite())
        || !port_kinetic_energy_flow_in_w.is_finite()
        || !port_kinetic_energy_flow_out_w.is_finite()
        || power_balance_residual_w.abs() > power_balance_tolerance_w
    {
        return Err(format!(
            "{}: kinetic-energy balance mismatch or non-finite derived power",
            scenario.case_id
        ));
    }

    Ok(CaseEvaluation {
        case_id: scenario.case_id.clone(),
        frame_id: scenario.frame_id.clone(),
        boundary_id: scenario.boundary_id.clone(),
        epoch_s: scenario.epoch_s,
        port_count: scenario.ports.len(),
        mass_accumulation_kg_s: scenario.mass_accumulation_kg_s,
        mass_flow_in_kg_s,
        mass_flow_out_kg_s,
        mass_balance_residual_kg_s,
        mass_balance_tolerance_kg_s,
        kinetic_energy_accumulation_w: scenario.kinetic_energy_accumulation_w,
        port_kinetic_energy_flow_in_w,
        port_kinetic_energy_flow_out_w,
        net_advective_kinetic_energy_flux_w,
        nonport_external_power_on_cv_w: scenario.nonport_external_power_on_cv_w,
        boundary_work_on_cv_w: scenario.boundary_work_on_cv_w,
        mechanical_dissipation_rate_w: scenario.mechanical_dissipation_rate_w,
        net_declared_power_on_cv_w,
        power_balance_residual_w,
        power_balance_tolerance_w,
        port_contributions,
    })
}

pub fn evaluate_corpus(
    cases_csv: &str,
    ports_csv: &str,
) -> Result<EvaluationReport, String> {
    let scenarios = parse_corpus(cases_csv, ports_csv)?;
    let mut cases = Vec::with_capacity(scenarios.len());
    let mut port_count = 0usize;
    let mut max_abs_mass_balance_residual_kg_s: f64 = 0.0;
    let mut max_abs_power_balance_residual_w: f64 = 0.0;

    for scenario in &scenarios {
        port_count += scenario.ports.len();
        let result = evaluate_scenario(scenario)?;
        max_abs_mass_balance_residual_kg_s =
            max_abs_mass_balance_residual_kg_s.max(result.mass_balance_residual_kg_s.abs());
        max_abs_power_balance_residual_w =
            max_abs_power_balance_residual_w.max(result.power_balance_residual_w.abs());
        cases.push(result);
    }

    Ok(EvaluationReport {
        case_count: cases.len(),
        cases,
        port_count,
        max_abs_mass_balance_residual_kg_s,
        max_abs_power_balance_residual_w,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASES: &str = include_str!("../cases-v0.1.csv");
    const PORTS: &str = include_str!("../ports-v0.1.csv");

    fn scenario(case_id: &str) -> KineticEnergyScenario {
        parse_corpus(CASES, PORTS)
            .expect("frozen corpus parses")
            .into_iter()
            .find(|case| case.case_id == case_id)
            .expect("frozen case exists")
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected:.15e}, got {actual:.15e}, tolerance {tolerance:.3e}"
        );
    }

    #[test]
    fn frozen_corpus_has_six_scenarios_and_fourteen_ports() {
        let report = evaluate_corpus(CASES, PORTS).expect("corpus evaluates");
        assert_eq!(report.case_count, EXPECTED_CASE_COUNT);
        assert_eq!(report.port_count, EXPECTED_PORT_COUNT);
        assert_eq!(report.max_abs_mass_balance_residual_kg_s, 0.0);
        assert_eq!(report.max_abs_power_balance_residual_w, 0.0);
    }

    #[test]
    fn equal_speed_straight_through_flow_has_zero_kinetic_energy_flux() {
        let result = evaluate_scenario(&scenario("STRAIGHT_THROUGH")).expect("straight flow");
        close(result.net_advective_kinetic_energy_flux_w, 0.0, 0.0);
        close(result.kinetic_energy_accumulation_w, 0.0, 0.0);
    }

    #[test]
    fn speed_change_matches_analytic_kinetic_energy_flux() {
        let result = evaluate_scenario(&scenario("SPEED_CHANGE")).expect("speed change");
        close(result.port_kinetic_energy_flow_in_w, 2.0, 0.0);
        close(result.port_kinetic_energy_flow_out_w, 8.0, 0.0);
        close(result.net_advective_kinetic_energy_flux_w, 6.0, 0.0);
        close(result.kinetic_energy_accumulation_w, 1.0, 0.0);
        close(result.net_declared_power_on_cv_w, 7.0, 0.0);
        close(result.power_balance_residual_w, 0.0, 0.0);
    }

    #[test]
    fn unsteady_storage_includes_mass_and_kinetic_energy_accumulation() {
        let result = evaluate_scenario(&scenario("UNSTEADY_STORAGE")).expect("unsteady");
        close(result.mass_flow_in_kg_s - result.mass_flow_out_kg_s, 1.0, 0.0);
        close(result.port_kinetic_energy_flow_in_w, 8.0, 0.0);
        close(result.port_kinetic_energy_flow_out_w, 13.5, 0.0);
        close(result.net_advective_kinetic_energy_flux_w, 5.5, 0.0);
        close(result.kinetic_energy_accumulation_w, 1.5, 0.0);
    }

    #[test]
    fn boundary_work_and_mechanical_dissipation_remain_separate() {
        let result = evaluate_scenario(&scenario("WORK_AND_DISSIPATION")).expect("work");
        close(result.nonport_external_power_on_cv_w, 7.0, 0.0);
        close(result.boundary_work_on_cv_w, 2.0, 0.0);
        close(result.mechanical_dissipation_rate_w, 5.0, 0.0);
        close(result.net_declared_power_on_cv_w, 4.0, 0.0);
        close(result.power_balance_residual_w, 0.0, 0.0);
    }

    #[test]
    fn negative_external_power_is_preserved_without_recovery_inference() {
        let result = evaluate_scenario(&scenario("NEGATIVE_EXTERNAL_POWER")).expect("negative power");
        close(result.nonport_external_power_on_cv_w, -6.0, 0.0);
        close(result.boundary_work_on_cv_w, 2.0, 0.0);
        close(result.mechanical_dissipation_rate_w, 1.0, 0.0);
        close(result.kinetic_energy_accumulation_w, -5.0, 0.0);
        assert_eq!(result.nonport_external_power_on_cv_w.is_sign_negative(), true);
    }

    #[test]
    fn multiport_energy_flux_uses_each_port_speed_and_mass_rate() {
        let result = evaluate_scenario(&scenario("MULTIPORT_3D")).expect("multiport");
        close(result.port_kinetic_energy_flow_in_w, 17.0, 0.0);
        close(result.port_kinetic_energy_flow_out_w, 16.5, 0.0);
        close(result.net_advective_kinetic_energy_flux_w, -0.5, 0.0);
        close(result.power_balance_residual_w, 0.0, 0.0);
    }

    #[test]
    fn orthogonal_rotation_preserves_specific_kinetic_energy_flux() {
        let original = scenario("MULTIPORT_3D");
        let mut rotated = original.clone();
        for port in &mut rotated.ports {
            let v = port.velocity_inertial_m_s;
            port.velocity_inertial_m_s = Vector3::new(-v.y, v.x, v.z);
        }
        let original_result = evaluate_scenario(&original).expect("original");
        let rotated_result = evaluate_scenario(&rotated).expect("rotated");
        close(
            rotated_result.net_advective_kinetic_energy_flux_w,
            original_result.net_advective_kinetic_energy_flux_w,
            1.0e-12,
        );
    }

    #[test]
    fn port_order_permutation_is_deterministic_for_fractional_inputs() {
        let mut original = scenario("MULTIPORT_3D");
        original.ports[0].mass_flow_kg_s = 1.3;
        original.ports[1].mass_flow_kg_s = 0.7;
        original.ports[2].mass_flow_kg_s = 0.5;
        original.ports[3].mass_flow_kg_s = 1.5;
        let expected = evaluate_scenario(&original).expect("original order");
        original.ports.reverse();
        let actual = evaluate_scenario(&original).expect("reverse order");
        assert_eq!(
            actual.net_advective_kinetic_energy_flux_w.to_bits(),
            expected.net_advective_kinetic_energy_flux_w.to_bits()
        );
    }

    #[test]
    fn invalid_mass_power_and_dissipation_inputs_fail_closed() {
        let mut mass_mismatch = scenario("UNSTEADY_STORAGE");
        mass_mismatch.mass_accumulation_kg_s = 0.5;
        assert!(evaluate_scenario(&mass_mismatch).is_err());

        let mut power_mismatch = scenario("SPEED_CHANGE");
        power_mismatch.nonport_external_power_on_cv_w = 6.0;
        assert!(evaluate_scenario(&power_mismatch).is_err());

        let mut negative_dissipation = scenario("WORK_AND_DISSIPATION");
        negative_dissipation.mechanical_dissipation_rate_w = -1.0;
        assert!(evaluate_scenario(&negative_dissipation).is_err());

        let mut negative_mass_flow = scenario("SPEED_CHANGE");
        negative_mass_flow.ports[0].mass_flow_kg_s = -1.0;
        assert!(evaluate_scenario(&negative_mass_flow).is_err());

        let mut non_finite = scenario("SPEED_CHANGE");
        non_finite.ports[0].velocity_inertial_m_s.x = f64::NAN;
        assert!(evaluate_scenario(&non_finite).is_err());
    }
}
