//! Exact advective angular-momentum flux reference about a fixed inertial origin.
//!
//! This is a mathematical/software reference only. It does not model stress
//! traction, electromagnetic field angular momentum, structural constitutive
//! response, flexible-body gyroscopic terms, or total energy. Port positions
//! and velocities are expressed in one declared inertial frame; port positions
//! are relative to one declared fixed origin.

use std::collections::{BTreeMap, HashSet};

pub const EXPECTED_CASE_COUNT: usize = 6;
pub const EXPECTED_PORT_COUNT: usize = 14;
pub const MASS_BALANCE_REL_TOLERANCE: f64 = 1.0e-12;
pub const MASS_BALANCE_ABS_TOLERANCE_KG_S: f64 = 1.0e-12;
pub const MAX_PORT_COUNT_PER_CASE: usize = 1_024;
const EXPECTED_PORT_HEADER: &str = concat!(
    "case_id,port_id,direction,mass_flow_kg_s,",
    "position_x_m,position_y_m,position_z_m,",
    "velocity_x_m_s,velocity_y_m_s,velocity_z_m_s"
);
const EXPECTED_CASE_HEADER: &str = concat!(
    "case_id,frame_id,origin_id,epoch_s,mass_accumulation_kg_s,",
    "momentum_accumulation_x_n,momentum_accumulation_y_n,momentum_accumulation_z_n,",
    "angular_momentum_accumulation_x_n_m,angular_momentum_accumulation_y_n_m,",
    "angular_momentum_accumulation_z_n_m,other_external_force_x_n,",
    "other_external_force_y_n,other_external_force_z_n,other_external_torque_x_n_m,",
    "other_external_torque_y_n_m,other_external_torque_z_n_m"
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

    pub fn scale(self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl std::ops::Add for Vector3 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl std::ops::Sub for Vector3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowDirection {
    Inflow,
    Outflow,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MassFluxPort {
    pub port_id: String,
    pub direction: FlowDirection,
    pub mass_flow_kg_s: f64,
    /// Inertial position relative to the scenario's fixed origin, in metres.
    pub position_from_origin_m: Vector3,
    /// Absolute inertial velocity, in metres per second.
    pub velocity_inertial_m_s: Vector3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AngularMomentumScenario {
    pub case_id: String,
    pub frame_id: String,
    pub origin_id: String,
    pub epoch_s: f64,
    pub mass_accumulation_kg_s: f64,
    pub linear_momentum_accumulation_n: Vector3,
    pub angular_momentum_accumulation_n_m: Vector3,
    pub other_external_force_n: Vector3,
    pub other_external_torque_n_m: Vector3,
    pub ports: Vec<MassFluxPort>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaseEvaluation {
    pub case_id: String,
    pub frame_id: String,
    pub origin_id: String,
    pub epoch_s: f64,
    pub port_count: usize,
    pub mass_accumulation_kg_s: f64,
    pub mass_flow_in_kg_s: f64,
    pub mass_flow_out_kg_s: f64,
    pub mass_balance_residual_kg_s: f64,
    pub linear_momentum_flux_n: Vector3,
    pub required_external_force_n: Vector3,
    pub support_on_stream_force_n: Vector3,
    pub stream_on_support_reaction_n: Vector3,
    pub force_reaction_closure_residual_n: Vector3,
    pub angular_momentum_accumulation_n_m: Vector3,
    pub other_external_force_n: Vector3,
    pub other_external_torque_n_m: Vector3,
    /// Sum(outflow mdot * (r cross v)) - sum(inflow mdot * (r cross v)).
    pub advective_angular_momentum_flux_n_m: Vector3,
    pub required_external_torque_n_m: Vector3,
    pub support_on_stream_torque_n_m: Vector3,
    pub stream_on_support_reaction_torque_n_m: Vector3,
    pub torque_reaction_closure_residual_n_m: Vector3,
    pub mass_balance_tolerance_kg_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub cases: Vec<CaseEvaluation>,
    pub case_count: usize,
    pub port_count: usize,
    pub max_abs_mass_balance_residual_kg_s: f64,
    pub max_abs_force_reaction_closure_residual_n: f64,
    pub max_abs_torque_reaction_closure_residual_n_m: f64,
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
) -> Result<Vec<AngularMomentumScenario>, String> {
    let mut case_lines = cases_csv.lines();
    let header = case_lines
        .next()
        .ok_or_else(|| "empty angular-momentum scenario corpus".to_owned())?;
    if header != EXPECTED_CASE_HEADER {
        return Err("unexpected angular-momentum scenario header".to_owned());
    }

    let mut scenarios = BTreeMap::new();
    for (line_index, line) in case_lines.enumerate() {
        if line.trim().is_empty() {
            return Err(format!("scenario line {}: blank row", line_index + 2));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 17 {
            return Err(format!(
                "scenario line {}: expected seventeen columns",
                line_index + 2
            ));
        }
        let case_id = fields[0].to_owned();
        let frame_id = fields[1].to_owned();
        let origin_id = fields[2].to_owned();
        if !valid_identifier(&case_id)
            || !valid_identifier(&frame_id)
            || !valid_identifier(&origin_id)
        {
            return Err(format!(
                "scenario line {}: invalid case, frame or origin ID",
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
        let linear_momentum_accumulation_n = parse_vector(
            &fields[5..8],
            &format!("scenario line {} linear momentum accumulation", line_index + 2),
        )?;
        let angular_momentum_accumulation_n_m = parse_vector(
            &fields[8..11],
            &format!("scenario line {} angular momentum accumulation", line_index + 2),
        )?;
        let other_external_force_n = parse_vector(
            &fields[11..14],
            &format!("scenario line {} other external force", line_index + 2),
        )?;
        let other_external_torque_n_m = parse_vector(
            &fields[14..17],
            &format!("scenario line {} other external torque", line_index + 2),
        )?;

        let scenario = AngularMomentumScenario {
            case_id: case_id.clone(),
            frame_id,
            origin_id,
            epoch_s,
            mass_accumulation_kg_s,
            linear_momentum_accumulation_n,
            angular_momentum_accumulation_n_m,
            other_external_force_n,
            other_external_torque_n_m,
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
        .ok_or_else(|| "empty angular-momentum port corpus".to_owned())?;
    if port_header
        != EXPECTED_PORT_HEADER
    {
        return Err("unexpected angular-momentum port header".to_owned());
    }

    let mut port_count = 0usize;
    for (line_index, line) in port_lines.enumerate() {
        if line.trim().is_empty() {
            return Err(format!("port line {}: blank row", line_index + 2));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 10 {
            return Err(format!("port line {}: expected ten columns", line_index + 2));
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
        let position_from_origin_m = parse_vector(
            &fields[4..7],
            &format!("port line {} position from origin", line_index + 2),
        )?;
        let velocity_inertial_m_s = parse_vector(
            &fields[7..10],
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
        scenario.ports.push(MassFluxPort {
            port_id,
            direction,
            mass_flow_kg_s,
            position_from_origin_m,
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

pub fn evaluate_scenario(scenario: &AngularMomentumScenario) -> Result<CaseEvaluation, String> {
    if !valid_identifier(&scenario.case_id)
        || !valid_identifier(&scenario.frame_id)
        || !valid_identifier(&scenario.origin_id)
    {
        return Err("invalid case, frame or origin ID".to_owned());
    }
    if !scenario.epoch_s.is_finite() {
        return Err(format!("{}: non-finite evaluation epoch", scenario.case_id));
    }
    if scenario.ports.is_empty() {
        return Err(format!("{}: at least one mass-flux port is required", scenario.case_id));
    }
    if scenario.ports.len() > MAX_PORT_COUNT_PER_CASE {
        return Err(format!("{}: port limit exceeded", scenario.case_id));
    }
    if !scenario.mass_accumulation_kg_s.is_finite()
        || !scenario.linear_momentum_accumulation_n.is_finite()
        || !scenario.angular_momentum_accumulation_n_m.is_finite()
        || !scenario.other_external_force_n.is_finite()
        || !scenario.other_external_torque_n_m.is_finite()
    {
        return Err(format!("{}: non-finite scenario input", scenario.case_id));
    }

    let mut ordered_ports: Vec<&MassFluxPort> = scenario.ports.iter().collect();
    ordered_ports.sort_by(|left, right| left.port_id.cmp(&right.port_id));
    let mut port_ids = HashSet::new();
    let mut mass_flow_in_kg_s = 0.0;
    let mut mass_flow_out_kg_s = 0.0;
    let mut linear_momentum_flux_n = Vector3::ZERO;
    let mut advective_angular_momentum_flux_n_m = Vector3::ZERO;

    for port in ordered_ports {
        if !valid_identifier(&port.port_id) || !port_ids.insert(port.port_id.as_str()) {
            return Err(format!("{}: invalid or duplicate port ID", scenario.case_id));
        }
        if !port.mass_flow_kg_s.is_finite() || port.mass_flow_kg_s < 0.0 {
            return Err(format!("{}: invalid mass-flow magnitude", scenario.case_id));
        }
        if !port.position_from_origin_m.is_finite() || !port.velocity_inertial_m_s.is_finite() {
            return Err(format!("{}: non-finite port position or velocity", scenario.case_id));
        }

        let signed_rate = match port.direction {
            FlowDirection::Inflow => {
                mass_flow_in_kg_s += port.mass_flow_kg_s;
                -port.mass_flow_kg_s
            }
            FlowDirection::Outflow => {
                mass_flow_out_kg_s += port.mass_flow_kg_s;
                port.mass_flow_kg_s
            }
        };
        linear_momentum_flux_n =
            linear_momentum_flux_n + port.velocity_inertial_m_s.scale(signed_rate);
        let specific_angular_momentum =
            port.position_from_origin_m.cross(port.velocity_inertial_m_s);
        advective_angular_momentum_flux_n_m = advective_angular_momentum_flux_n_m
            + specific_angular_momentum.scale(signed_rate);
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

    let required_external_force_n =
        scenario.linear_momentum_accumulation_n + linear_momentum_flux_n;
    let support_on_stream_force_n = required_external_force_n - scenario.other_external_force_n;
    let stream_on_support_reaction_n = support_on_stream_force_n.scale(-1.0);
    let force_reaction_closure_residual_n =
        support_on_stream_force_n + stream_on_support_reaction_n;

    let required_external_torque_n_m =
        scenario.angular_momentum_accumulation_n_m + advective_angular_momentum_flux_n_m;
    let support_on_stream_torque_n_m =
        required_external_torque_n_m - scenario.other_external_torque_n_m;
    let stream_on_support_reaction_torque_n_m = support_on_stream_torque_n_m.scale(-1.0);
    let torque_reaction_closure_residual_n_m =
        support_on_stream_torque_n_m + stream_on_support_reaction_torque_n_m;

    let derived = [
        linear_momentum_flux_n,
        required_external_force_n,
        support_on_stream_force_n,
        stream_on_support_reaction_n,
        force_reaction_closure_residual_n,
        advective_angular_momentum_flux_n_m,
        required_external_torque_n_m,
        support_on_stream_torque_n_m,
        stream_on_support_reaction_torque_n_m,
        torque_reaction_closure_residual_n_m,
    ];
    if derived.iter().any(|vector| !vector.is_finite()) {
        return Err(format!("{}: non-finite derived momentum or torque", scenario.case_id));
    }

    Ok(CaseEvaluation {
        case_id: scenario.case_id.clone(),
        frame_id: scenario.frame_id.clone(),
        origin_id: scenario.origin_id.clone(),
        epoch_s: scenario.epoch_s,
        port_count: scenario.ports.len(),
        mass_accumulation_kg_s: scenario.mass_accumulation_kg_s,
        mass_flow_in_kg_s,
        mass_flow_out_kg_s,
        mass_balance_residual_kg_s,
        linear_momentum_flux_n,
        required_external_force_n,
        support_on_stream_force_n,
        stream_on_support_reaction_n,
        force_reaction_closure_residual_n,
        angular_momentum_accumulation_n_m: scenario.angular_momentum_accumulation_n_m,
        other_external_force_n: scenario.other_external_force_n,
        other_external_torque_n_m: scenario.other_external_torque_n_m,
        advective_angular_momentum_flux_n_m,
        required_external_torque_n_m,
        support_on_stream_torque_n_m,
        stream_on_support_reaction_torque_n_m,
        torque_reaction_closure_residual_n_m,
        mass_balance_tolerance_kg_s,
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
    let mut max_abs_force_reaction_closure_residual_n: f64 = 0.0;
    let mut max_abs_torque_reaction_closure_residual_n_m: f64 = 0.0;

    for scenario in &scenarios {
        port_count += scenario.ports.len();
        let result = evaluate_scenario(scenario)?;
        max_abs_mass_balance_residual_kg_s =
            max_abs_mass_balance_residual_kg_s.max(result.mass_balance_residual_kg_s.abs());
        max_abs_force_reaction_closure_residual_n = max_abs_force_reaction_closure_residual_n
            .max(result.force_reaction_closure_residual_n.norm());
        max_abs_torque_reaction_closure_residual_n_m = max_abs_torque_reaction_closure_residual_n_m
            .max(result.torque_reaction_closure_residual_n_m.norm());
        cases.push(result);
    }

    Ok(EvaluationReport {
        case_count: cases.len(),
        cases,
        port_count,
        max_abs_mass_balance_residual_kg_s,
        max_abs_force_reaction_closure_residual_n,
        max_abs_torque_reaction_closure_residual_n_m,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASES: &str = include_str!("../cases-v0.1.csv");
    const PORTS: &str = include_str!("../ports-v0.1.csv");

    fn rotate_z(vector: Vector3, angle: f64) -> Vector3 {
        let (sine, cosine) = angle.sin_cos();
        Vector3::new(
            cosine * vector.x - sine * vector.y,
            sine * vector.x + cosine * vector.y,
            vector.z,
        )
    }

    fn rotate_x(vector: Vector3, angle: f64) -> Vector3 {
        let (sine, cosine) = angle.sin_cos();
        Vector3::new(
            vector.x,
            cosine * vector.y - sine * vector.z,
            sine * vector.y + cosine * vector.z,
        )
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected:.15e}, got {actual:.15e}, tolerance {tolerance:.3e}"
        );
    }

    fn close_vec(actual: Vector3, expected: Vector3, tolerance: f64) {
        close(actual.x, expected.x, tolerance);
        close(actual.y, expected.y, tolerance);
        close(actual.z, expected.z, tolerance);
    }

    fn scenario(case_id: &str) -> AngularMomentumScenario {
        parse_corpus(CASES, PORTS)
            .expect("frozen corpus parses")
            .into_iter()
            .find(|case| case.case_id == case_id)
            .expect("frozen case exists")
    }

    #[test]
    fn frozen_corpus_has_six_cases_and_fourteen_ports() {
        let report = evaluate_corpus(CASES, PORTS).expect("corpus evaluates");
        assert_eq!(report.case_count, EXPECTED_CASE_COUNT);
        assert_eq!(report.port_count, EXPECTED_PORT_COUNT);
        assert_eq!(report.max_abs_mass_balance_residual_kg_s, 0.0);
        assert_eq!(report.max_abs_force_reaction_closure_residual_n, 0.0);
        assert_eq!(report.max_abs_torque_reaction_closure_residual_n_m, 0.0);
    }

    #[test]
    fn radial_through_flow_has_zero_angular_momentum_flux() {
        let result = evaluate_scenario(&scenario("RADIAL_THROUGH")).expect("radial flow");
        close_vec(result.advective_angular_momentum_flux_n_m, Vector3::ZERO, 0.0);
        close_vec(result.stream_on_support_reaction_torque_n_m, Vector3::ZERO, 0.0);
    }

    #[test]
    fn tangential_flow_matches_the_hand_calculated_moment_arm() {
        let result = evaluate_scenario(&scenario("TANGENTIAL_FLOW")).expect("tangential flow");
        close_vec(
            result.advective_angular_momentum_flux_n_m,
            Vector3::new(0.0, 0.0, 2.0),
            1.0e-12,
        );
        close_vec(
            result.stream_on_support_reaction_torque_n_m,
            Vector3::new(0.0, 0.0, 2.0),
            1.0e-12,
        );
    }

    #[test]
    fn off_axis_turn_reproduces_vector_flux_and_torque() {
        let result = evaluate_scenario(&scenario("OFF_AXIS_TURN")).expect("off-axis turn");
        close_vec(
            result.advective_angular_momentum_flux_n_m,
            Vector3::new(0.0, 0.0, -12.0),
            1.0e-12,
        );
        close_vec(
            result.required_external_torque_n_m,
            Vector3::new(1.0, 2.0, -12.0),
            1.0e-12,
        );
        close_vec(
            result.support_on_stream_torque_n_m,
            Vector3::new(1.0, 2.0, -10.0),
            1.0e-12,
        );
    }

    #[test]
    fn multiport_3d_case_sums_each_port_cross_product() {
        let result = evaluate_scenario(&scenario("MULTIPORT_3D")).expect("multiport case");
        close_vec(
            result.advective_angular_momentum_flux_n_m,
            Vector3::new(-6.0, 8.0, -1.0),
            1.0e-12,
        );
        close_vec(
            result.required_external_torque_n_m,
            Vector3::new(-4.0, 8.0, -2.0),
            1.0e-12,
        );
        close_vec(
            result.stream_on_support_reaction_torque_n_m,
            Vector3::new(5.0, -6.0, 5.0),
            1.0e-12,
        );
    }

    #[test]
    fn unsteady_control_volume_preserves_linear_and_angular_accumulation() {
        let result = evaluate_scenario(&scenario("UNSTEADY_STORAGE")).expect("unsteady case");
        close(result.mass_flow_in_kg_s - result.mass_flow_out_kg_s, 1.0, 0.0);
        close_vec(
            result.linear_momentum_flux_n,
            Vector3::new(6.0, -8.0, 0.0),
            1.0e-12,
        );
        close_vec(
            result.advective_angular_momentum_flux_n_m,
            Vector3::new(0.0, 0.0, -14.0),
            1.0e-12,
        );
        close_vec(
            result.required_external_torque_n_m,
            Vector3::new(0.0, 0.0, -11.0),
            1.0e-12,
        );
        close_vec(
            result.stream_on_support_reaction_torque_n_m,
            Vector3::new(0.0, 0.0, 10.0),
            1.0e-12,
        );
    }

    #[test]
    fn other_external_force_and_torque_are_subtracted_once() {
        let result = evaluate_scenario(&scenario("TORQUE_ISOLATION")).expect("torque isolation");
        close_vec(
            result.advective_angular_momentum_flux_n_m,
            Vector3::new(0.0, 0.0, -4.0),
            1.0e-12,
        );
        close_vec(
            result.support_on_stream_torque_n_m,
            Vector3::new(0.0, 0.0, -2.0),
            1.0e-12,
        );
    }

    #[test]
    fn orthogonal_rotation_covariance_preserves_torque_norms() {
        let original = scenario("UNSTEADY_STORAGE");
        let original_result = evaluate_scenario(&original).expect("original");
        let rotated = AngularMomentumScenario {
            case_id: original.case_id.clone(),
            frame_id: "ROTATED_INERTIAL".to_owned(),
            origin_id: original.origin_id.clone(),
            epoch_s: original.epoch_s,
            mass_accumulation_kg_s: original.mass_accumulation_kg_s,
            linear_momentum_accumulation_n: rotate_x(
                rotate_z(original.linear_momentum_accumulation_n, 0.37),
                -0.81,
            ),
            angular_momentum_accumulation_n_m: rotate_x(
                rotate_z(original.angular_momentum_accumulation_n_m, 0.37),
                -0.81,
            ),
            other_external_force_n: rotate_x(
                rotate_z(original.other_external_force_n, 0.37),
                -0.81,
            ),
            other_external_torque_n_m: rotate_x(
                rotate_z(original.other_external_torque_n_m, 0.37),
                -0.81,
            ),
            ports: original
                .ports
                .iter()
                .map(|port| MassFluxPort {
                    port_id: port.port_id.clone(),
                    direction: port.direction,
                    mass_flow_kg_s: port.mass_flow_kg_s,
                    position_from_origin_m: rotate_x(
                        rotate_z(port.position_from_origin_m, 0.37),
                        -0.81,
                    ),
                    velocity_inertial_m_s: rotate_x(
                        rotate_z(port.velocity_inertial_m_s, 0.37),
                        -0.81,
                    ),
                })
                .collect(),
        };
        let rotated_result = evaluate_scenario(&rotated).expect("rotated");
        close_vec(
            rotated_result.advective_angular_momentum_flux_n_m,
            rotate_x(
                rotate_z(original_result.advective_angular_momentum_flux_n_m, 0.37),
                -0.81,
            ),
            1.0e-10,
        );
        close(
            rotated_result.support_on_stream_torque_n_m.norm(),
            original_result.support_on_stream_torque_n_m.norm(),
            1.0e-10,
        );
    }

    #[test]
    fn origin_translation_transforms_accumulation_flux_and_reaction_consistently() {
        let original = scenario("UNSTEADY_STORAGE");
        let original_result = evaluate_scenario(&original).expect("original");
        let shift = Vector3::new(0.5, -0.2, 0.3);
        let translated = AngularMomentumScenario {
            case_id: original.case_id.clone(),
            frame_id: original.frame_id.clone(),
            origin_id: "O1".to_owned(),
            epoch_s: original.epoch_s,
            mass_accumulation_kg_s: original.mass_accumulation_kg_s,
            linear_momentum_accumulation_n: original.linear_momentum_accumulation_n,
            angular_momentum_accumulation_n_m: original.angular_momentum_accumulation_n_m
                - shift.cross(original.linear_momentum_accumulation_n),
            other_external_force_n: original.other_external_force_n,
            other_external_torque_n_m: original.other_external_torque_n_m
                - shift.cross(original.other_external_force_n),
            ports: original
                .ports
                .iter()
                .map(|port| MassFluxPort {
                    port_id: port.port_id.clone(),
                    direction: port.direction,
                    mass_flow_kg_s: port.mass_flow_kg_s,
                    position_from_origin_m: port.position_from_origin_m - shift,
                    velocity_inertial_m_s: port.velocity_inertial_m_s,
                })
                .collect(),
        };
        let translated_result = evaluate_scenario(&translated).expect("translated origin");
        close_vec(
            translated_result.required_external_torque_n_m,
            original_result.required_external_torque_n_m
                - shift.cross(original_result.required_external_force_n),
            1.0e-10,
        );
        close_vec(
            translated_result.support_on_stream_torque_n_m,
            original_result.support_on_stream_torque_n_m
                - shift.cross(original_result.support_on_stream_force_n),
            1.0e-10,
        );
        close_vec(
            translated_result.stream_on_support_reaction_torque_n_m,
            original_result.stream_on_support_reaction_torque_n_m
                - shift.cross(original_result.stream_on_support_reaction_n),
            1.0e-10,
        );
    }

    #[test]
    fn port_order_permutation_does_not_change_angular_momentum_flux() {
        let mut original = scenario("MULTIPORT_3D");
        let expected = evaluate_scenario(&original).expect("original order");
        original.ports.reverse();
        let actual = evaluate_scenario(&original).expect("reversed order");
        close_vec(
            actual.advective_angular_momentum_flux_n_m,
            expected.advective_angular_momentum_flux_n_m,
            0.0,
        );
        close_vec(
            actual.support_on_stream_torque_n_m,
            expected.support_on_stream_torque_n_m,
            0.0,
        );
    }

    #[test]
    fn mass_balance_mismatch_duplicate_ports_negative_rates_and_non_finite_values_fail_closed() {
        let mut mass_mismatch = scenario("UNSTEADY_STORAGE");
        mass_mismatch.mass_accumulation_kg_s = 0.5;
        assert!(evaluate_scenario(&mass_mismatch).is_err());

        let mut duplicate = scenario("TANGENTIAL_FLOW");
        duplicate.ports[1].port_id = duplicate.ports[0].port_id.clone();
        assert!(evaluate_scenario(&duplicate).is_err());

        let mut negative = scenario("TANGENTIAL_FLOW");
        negative.ports[0].mass_flow_kg_s = -1.0;
        assert!(evaluate_scenario(&negative).is_err());

        let mut non_finite = scenario("TANGENTIAL_FLOW");
        non_finite.ports[0].position_from_origin_m.x = f64::NAN;
        assert!(evaluate_scenario(&non_finite).is_err());
    }
}
