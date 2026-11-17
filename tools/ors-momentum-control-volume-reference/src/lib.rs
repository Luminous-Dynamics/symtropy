//! Exact-vector advective momentum-flux reference for a finite control volume.
//!
//! This is a mathematical/software reference only. It does not model pressure
//! or stress traction, electromagnetic field momentum, material constitutive
//! response, structural dynamics, angular-momentum flux, or total energy.
//! Every port velocity is expressed in one declared inertial frame.

use std::collections::{BTreeMap, HashSet};

pub const EXPECTED_CASE_COUNT: usize = 6;
pub const EXPECTED_PORT_COUNT: usize = 14;
pub const MASS_BALANCE_REL_TOLERANCE: f64 = 1.0e-12;
pub const MASS_BALANCE_ABS_TOLERANCE_KG_S: f64 = 1.0e-12;
pub const MAX_PORT_COUNT_PER_CASE: usize = 1_024;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    pub fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    pub fn scale(self, scalar: f64) -> Self {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
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

    pub fn max_abs_component(self) -> f64 {
        self.x.abs().max(self.y.abs()).max(self.z.abs())
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
    pub velocity_inertial_m_s: Vector3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlVolumeScenario {
    pub case_id: String,
    /// dM_CV/dt in kg/s, positive when mass accumulates inside the CV.
    pub mass_accumulation_kg_s: f64,
    /// d/dt integral_CV(rho * v dV), in N, in the declared inertial frame.
    pub momentum_accumulation_n: Vector3,
    /// Explicit sum of all already-derived non-support external forces on the
    /// stream inside this CV, in N. This is not a residual or balancing sink.
    pub other_external_force_n: Vector3,
    pub ports: Vec<MassFluxPort>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaseEvaluation {
    pub case_id: String,
    pub mass_flow_in_kg_s: f64,
    pub mass_flow_out_kg_s: f64,
    pub mass_balance_residual_kg_s: f64,
    /// Sum(outflow mdot * v) - Sum(inflow mdot * v), in N.
    pub advective_momentum_flux_n: Vector3,
    /// dP_CV/dt + net advective momentum flux, in N.
    pub required_external_resultant_n: Vector3,
    /// Force exerted by the declared support interaction on the stream, in N.
    pub support_on_stream_force_n: Vector3,
    /// Newton's third-law counterpart: force exerted by the stream on support.
    pub stream_on_support_reaction_n: Vector3,
    /// Equal/opposite interaction closure, expected to be zero.
    pub reaction_closure_residual_n: Vector3,
    pub mass_balance_tolerance_kg_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub cases: Vec<CaseEvaluation>,
    pub case_count: usize,
    pub port_count: usize,
    pub max_abs_mass_balance_residual_kg_s: f64,
    pub max_abs_reaction_closure_residual_n: f64,
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

pub fn parse_corpus(cases_csv: &str, ports_csv: &str) -> Result<Vec<ControlVolumeScenario>, String> {
    let mut case_lines = cases_csv.lines();
    let case_header = case_lines
        .next()
        .ok_or_else(|| "empty scenario corpus".to_owned())?;
    if case_header
        != "case_id,mass_accumulation_kg_s,momentum_accumulation_x_n,momentum_accumulation_y_n,momentum_accumulation_z_n,other_external_force_x_n,other_external_force_y_n,other_external_force_z_n"
    {
        return Err("unexpected scenario corpus header".to_owned());
    }

    let mut scenarios = BTreeMap::new();
    for (line_index, line) in case_lines.enumerate() {
        if line.trim().is_empty() {
            return Err(format!("scenario line {}: blank row", line_index + 2));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 8 {
            return Err(format!("scenario line {}: expected eight columns", line_index + 2));
        }
        let case_id = fields[0].to_owned();
        if !valid_identifier(&case_id) {
            return Err(format!("scenario line {}: invalid case ID", line_index + 2));
        }
        let mass_accumulation_kg_s =
            parse_number(fields[1], &format!("scenario line {} mass accumulation", line_index + 2))?;
        let momentum_accumulation_n = parse_vector(
            &fields[2..5],
            &format!("scenario line {} momentum accumulation", line_index + 2),
        )?;
        let other_external_force_n = parse_vector(
            &fields[5..8],
            &format!("scenario line {} other external force", line_index + 2),
        )?;
        let scenario = ControlVolumeScenario {
            case_id: case_id.clone(),
            mass_accumulation_kg_s,
            momentum_accumulation_n,
            other_external_force_n,
            ports: Vec::new(),
        };
        if scenarios.insert(case_id.clone(), scenario).is_some() {
            return Err(format!("scenario line {}: duplicate case ID {case_id}", line_index + 2));
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
        .ok_or_else(|| "empty port corpus".to_owned())?;
    if port_header
        != "case_id,port_id,direction,mass_flow_kg_s,velocity_x_m_s,velocity_y_m_s,velocity_z_m_s"
    {
        return Err("unexpected port corpus header".to_owned());
    }

    let mut port_count = 0usize;
    for (line_index, line) in port_lines.enumerate() {
        if line.trim().is_empty() {
            return Err(format!("port line {}: blank row", line_index + 2));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 7 {
            return Err(format!("port line {}: expected seven columns", line_index + 2));
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
        let mass_flow_kg_s =
            parse_number(fields[3], &format!("port line {} mass flow", line_index + 2))?;
        if mass_flow_kg_s < 0.0 {
            return Err(format!("port line {}: negative mass-flow magnitude", line_index + 2));
        }
        let velocity_inertial_m_s = parse_vector(
            &fields[4..7],
            &format!("port line {} inertial velocity", line_index + 2),
        )?;
        let scenario = scenarios
            .get_mut(case_id)
            .ok_or_else(|| format!("port line {}: unknown case ID {case_id}", line_index + 2))?;
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

pub fn evaluate_scenario(scenario: &ControlVolumeScenario) -> Result<CaseEvaluation, String> {
    if !valid_identifier(&scenario.case_id) {
        return Err("invalid case ID".to_owned());
    }
    if scenario.ports.is_empty() {
        return Err(format!("{}: at least one mass-flux port is required", scenario.case_id));
    }
    if scenario.ports.len() > MAX_PORT_COUNT_PER_CASE {
        return Err(format!("{}: port limit exceeded", scenario.case_id));
    }
    if !scenario.mass_accumulation_kg_s.is_finite()
        || !scenario.momentum_accumulation_n.is_finite()
        || !scenario.other_external_force_n.is_finite()
    {
        return Err(format!("{}: non-finite scenario input", scenario.case_id));
    }

    let mut port_ids = HashSet::new();
    let mut mass_flow_in_kg_s = 0.0;
    let mut mass_flow_out_kg_s = 0.0;
    let mut advective_momentum_flux_n = Vector3::ZERO;

    for port in &scenario.ports {
        if !valid_identifier(&port.port_id) || !port_ids.insert(port.port_id.as_str()) {
            return Err(format!("{}: invalid or duplicate port ID", scenario.case_id));
        }
        if !port.mass_flow_kg_s.is_finite() || port.mass_flow_kg_s < 0.0 {
            return Err(format!("{}: invalid mass-flow magnitude", scenario.case_id));
        }
        if !port.velocity_inertial_m_s.is_finite() {
            return Err(format!("{}: non-finite port velocity", scenario.case_id));
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
        advective_momentum_flux_n =
            advective_momentum_flux_n.add(port.velocity_inertial_m_s.scale(signed_rate));
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
            "{}: mass balance mismatch or non-finite accumulation (residual {:.6e} kg/s, tolerance {:.6e} kg/s)",
            scenario.case_id, mass_balance_residual_kg_s, mass_balance_tolerance_kg_s
        ));
    }

    let required_external_resultant_n =
        scenario.momentum_accumulation_n.add(advective_momentum_flux_n);
    let support_on_stream_force_n =
        required_external_resultant_n.sub(scenario.other_external_force_n);
    let stream_on_support_reaction_n = support_on_stream_force_n.scale(-1.0);
    let reaction_closure_residual_n =
        support_on_stream_force_n.add(stream_on_support_reaction_n);

    let derived = [
        advective_momentum_flux_n,
        required_external_resultant_n,
        support_on_stream_force_n,
        stream_on_support_reaction_n,
        reaction_closure_residual_n,
    ];
    if derived.iter().any(|vector| !vector.is_finite()) {
        return Err(format!("{}: non-finite derived momentum result", scenario.case_id));
    }

    Ok(CaseEvaluation {
        case_id: scenario.case_id.clone(),
        mass_flow_in_kg_s,
        mass_flow_out_kg_s,
        mass_balance_residual_kg_s,
        advective_momentum_flux_n,
        required_external_resultant_n,
        support_on_stream_force_n,
        stream_on_support_reaction_n,
        reaction_closure_residual_n,
        mass_balance_tolerance_kg_s,
    })
}

pub fn evaluate_corpus(cases_csv: &str, ports_csv: &str) -> Result<EvaluationReport, String> {
    let scenarios = parse_corpus(cases_csv, ports_csv)?;
    let mut cases = Vec::with_capacity(scenarios.len());
    let mut port_count = 0usize;
    let mut max_abs_mass_balance_residual_kg_s: f64 = 0.0;
    let mut max_abs_reaction_closure_residual_n: f64 = 0.0;

    for scenario in &scenarios {
        port_count += scenario.ports.len();
        let result = evaluate_scenario(scenario)?;
        max_abs_mass_balance_residual_kg_s = max_abs_mass_balance_residual_kg_s
            .max(result.mass_balance_residual_kg_s.abs());
        max_abs_reaction_closure_residual_n = max_abs_reaction_closure_residual_n
            .max(result.reaction_closure_residual_n.norm());
        cases.push(result);
    }

    Ok(EvaluationReport {
        case_count: cases.len(),
        cases,
        port_count,
        max_abs_mass_balance_residual_kg_s,
        max_abs_reaction_closure_residual_n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASES: &str = include_str!("../cases-v0.1.csv");
    const PORTS: &str = include_str!("../ports-v0.1.csv");

    fn rotate_z(vector: Vector3, angle_rad: f64) -> Vector3 {
        let (sine, cosine) = angle_rad.sin_cos();
        Vector3::new(
            cosine * vector.x - sine * vector.y,
            sine * vector.x + cosine * vector.y,
            vector.z,
        )
    }

    fn rotate_x(vector: Vector3, angle_rad: f64) -> Vector3 {
        let (sine, cosine) = angle_rad.sin_cos();
        Vector3::new(
            vector.x,
            cosine * vector.y - sine * vector.z,
            sine * vector.y + cosine * vector.z,
        )
    }

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "expected {b:.15e}, got {a:.15e}, tolerance {tolerance:.3e}"
        );
    }

    fn close_vec(actual: Vector3, expected: Vector3, tolerance: f64) {
        close(actual.x, expected.x, tolerance);
        close(actual.y, expected.y, tolerance);
        close(actual.z, expected.z, tolerance);
    }

    fn scenario(case_id: &str) -> ControlVolumeScenario {
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
        assert_eq!(report.max_abs_reaction_closure_residual_n, 0.0);
    }

    #[test]
    fn straight_through_flow_has_zero_net_advective_momentum_flux() {
        let result = evaluate_scenario(&scenario("STRAIGHT_THROUGH")).expect("straight flow");
        close_vec(result.advective_momentum_flux_n, Vector3::ZERO, 0.0);
        close_vec(result.stream_on_support_reaction_n, Vector3::ZERO, 0.0);
    }

    #[test]
    fn ninety_degree_turn_matches_exact_vector_oracle() {
        let result = evaluate_scenario(&scenario("TURN_90_DEG")).expect("right-angle turn");
        close_vec(
            result.advective_momentum_flux_n,
            Vector3::new(-15.0, 15.0, 0.0),
            1.0e-12,
        );
        close_vec(
            result.stream_on_support_reaction_n,
            Vector3::new(15.0, -15.0, 0.0),
            1.0e-12,
        );
        close(
            result.stream_on_support_reaction_n.norm(),
            15.0 * 2.0_f64.sqrt(),
            1.0e-12,
        );
    }

    #[test]
    fn unsteady_control_volume_uses_both_mass_and_momentum_accumulation() {
        let result = evaluate_scenario(&scenario("UNSTEADY_STORAGE")).expect("unsteady case");
        close(result.mass_flow_in_kg_s - result.mass_flow_out_kg_s, 1.0, 0.0);
        close_vec(
            result.advective_momentum_flux_n,
            Vector3::new(-40.0, 30.0, 0.0),
            1.0e-12,
        );
        close_vec(
            result.required_external_resultant_n,
            Vector3::new(-38.0, 29.0, 0.5),
            1.0e-12,
        );
        close_vec(
            result.support_on_stream_force_n,
            Vector3::new(-39.0, 27.0, -2.5),
            1.0e-12,
        );
        close_vec(
            result.stream_on_support_reaction_n,
            Vector3::new(39.0, -27.0, 2.5),
            1.0e-12,
        );
    }

    #[test]
    fn multiple_inlets_and_outlets_sum_as_vectors_not_scalar_magnitudes() {
        let result = evaluate_scenario(&scenario("MULTIPORT_3D")).expect("multiport case");
        close_vec(
            result.advective_momentum_flux_n,
            Vector3::new(-20.0, -5.0, 5.0),
            1.0e-12,
        );
        close_vec(
            result.required_external_resultant_n,
            Vector3::new(-19.0, -3.0, 8.0),
            1.0e-12,
        );
        close_vec(
            result.stream_on_support_reaction_n,
            Vector3::new(21.0, 2.0, -4.0),
            1.0e-12,
        );
    }

    #[test]
    fn non_support_external_force_is_removed_before_solving_support_reaction() {
        let result = evaluate_scenario(&scenario("OTHER_FORCE_ISOLATION")).expect("external force case");
        close_vec(
            result.advective_momentum_flux_n,
            Vector3::new(-6.0, 8.0, 0.0),
            1.0e-12,
        );
        close_vec(
            result.support_on_stream_force_n,
            Vector3::new(-5.0, 5.0, -1.0),
            1.0e-12,
        );
        close_vec(
            result.stream_on_support_reaction_n,
            Vector3::new(5.0, -5.0, 1.0),
            1.0e-12,
        );
    }

    #[test]
    fn finite_angle_turn_matches_chord_length_formula_not_small_angle_surrogate() {
        let near_reversal = scenario("NEAR_REVERSAL_179");
        let result = evaluate_scenario(&near_reversal).expect("near-reversal");
        let ports = &near_reversal.ports;
        let inlet = ports.iter().find(|p| p.direction == FlowDirection::Inflow).unwrap();
        let outlet = ports.iter().find(|p| p.direction == FlowDirection::Outflow).unwrap();
        let speed = inlet.velocity_inertial_m_s.norm();
        let cosine = inlet.velocity_inertial_m_s.dot(outlet.velocity_inertial_m_s)
            / (speed * outlet.velocity_inertial_m_s.norm());
        let angle = cosine.clamp(-1.0, 1.0).acos();
        let exact_magnitude = inlet.mass_flow_kg_s * 2.0 * speed * (angle / 2.0).sin();
        close(result.stream_on_support_reaction_n.norm(), exact_magnitude, 1.0e-10);
        assert!(exact_magnitude > 19.9);
    }

    #[test]
    fn small_angle_sequence_converges_to_birch_turn_force_limit() {
        let speed = 12_000.0;
        let line_density = 0.75;
        let mass_flow = line_density * speed;
        let angles = [0.1, 1.0e-2, 1.0e-4, 1.0e-6];
        let mut previous_error = f64::INFINITY;

        for angle in angles {
            let inlet = Vector3::new(speed, 0.0, 0.0);
            let outlet = Vector3::new(speed * angle.cos(), speed * angle.sin(), 0.0);
            let test_case = ControlVolumeScenario {
                case_id: "SMALL_ANGLE".to_owned(),
                mass_accumulation_kg_s: 0.0,
                momentum_accumulation_n: Vector3::ZERO,
                other_external_force_n: Vector3::ZERO,
                ports: vec![
                    MassFluxPort {
                        port_id: "IN".to_owned(),
                        direction: FlowDirection::Inflow,
                        mass_flow_kg_s: mass_flow,
                        velocity_inertial_m_s: inlet,
                    },
                    MassFluxPort {
                        port_id: "OUT".to_owned(),
                        direction: FlowDirection::Outflow,
                        mass_flow_kg_s: mass_flow,
                        velocity_inertial_m_s: outlet,
                    },
                ],
            };
            let result = evaluate_scenario(&test_case).expect("finite turn case");
            let exact = result.stream_on_support_reaction_n.norm();
            let birch_small_angle = line_density * speed.powi(2) * angle;
            let relative_error = (exact - birch_small_angle).abs() / birch_small_angle;
            assert!(relative_error < previous_error);
            previous_error = relative_error;
        }
        assert!(previous_error < 4.2e-4);
    }

    #[test]
    fn orthogonal_rotation_covariance_preserves_scalar_residuals() {
        let original = scenario("UNSTEADY_STORAGE");
        let original_result = evaluate_scenario(&original).expect("original case");
        let rotated = ControlVolumeScenario {
            case_id: original.case_id.clone(),
            mass_accumulation_kg_s: original.mass_accumulation_kg_s,
            momentum_accumulation_n: rotate_x(
                rotate_z(original.momentum_accumulation_n, 0.37),
                -0.81,
            ),
            other_external_force_n: rotate_x(
                rotate_z(original.other_external_force_n, 0.37),
                -0.81,
            ),
            ports: original
                .ports
                .iter()
                .map(|port| MassFluxPort {
                    port_id: port.port_id.clone(),
                    direction: port.direction,
                    mass_flow_kg_s: port.mass_flow_kg_s,
                    velocity_inertial_m_s: rotate_x(
                        rotate_z(port.velocity_inertial_m_s, 0.37),
                        -0.81,
                    ),
                })
                .collect(),
        };
        let rotated_result = evaluate_scenario(&rotated).expect("rotated case");
        close_vec(
            rotated_result.advective_momentum_flux_n,
            rotate_x(rotate_z(original_result.advective_momentum_flux_n, 0.37), -0.81),
            1.0e-10,
        );
        close_vec(
            rotated_result.stream_on_support_reaction_n,
            rotate_x(
                rotate_z(original_result.stream_on_support_reaction_n, 0.37),
                -0.81,
            ),
            1.0e-10,
        );
        close(
            rotated_result.mass_balance_residual_kg_s,
            original_result.mass_balance_residual_kg_s,
            0.0,
        );
        close(
            rotated_result.stream_on_support_reaction_n.norm(),
            original_result.stream_on_support_reaction_n.norm(),
            1.0e-10,
        );
    }

    #[test]
    fn galilean_boost_covariance_holds_for_unsteady_control_volume() {
        let original = scenario("UNSTEADY_STORAGE");
        let original_result = evaluate_scenario(&original).expect("original case");
        let boost = Vector3::new(3.0, -2.0, 1.5);
        let boosted = ControlVolumeScenario {
            case_id: original.case_id.clone(),
            mass_accumulation_kg_s: original.mass_accumulation_kg_s,
            momentum_accumulation_n: original
                .momentum_accumulation_n
                .sub(boost.scale(original.mass_accumulation_kg_s)),
            other_external_force_n: original.other_external_force_n,
            ports: original
                .ports
                .iter()
                .map(|port| MassFluxPort {
                    port_id: port.port_id.clone(),
                    direction: port.direction,
                    mass_flow_kg_s: port.mass_flow_kg_s,
                    velocity_inertial_m_s: port.velocity_inertial_m_s.sub(boost),
                })
                .collect(),
        };
        let boosted_result = evaluate_scenario(&boosted).expect("boosted case");
        close_vec(
            boosted_result.required_external_resultant_n,
            original_result.required_external_resultant_n,
            1.0e-10,
        );
        close_vec(
            boosted_result.stream_on_support_reaction_n,
            original_result.stream_on_support_reaction_n,
            1.0e-10,
        );
    }

    #[test]
    fn port_order_permutation_does_not_change_results() {
        let mut original = scenario("MULTIPORT_3D");
        let expected = evaluate_scenario(&original).expect("original order");
        original.ports.reverse();
        let actual = evaluate_scenario(&original).expect("reversed order");
        close_vec(actual.advective_momentum_flux_n, expected.advective_momentum_flux_n, 0.0);
        close_vec(
            actual.stream_on_support_reaction_n,
            expected.stream_on_support_reaction_n,
            0.0,
        );
    }

    #[test]
    fn mismatched_mass_balance_fails_closed() {
        let mut invalid = scenario("UNSTEADY_STORAGE");
        invalid.mass_accumulation_kg_s = 0.9;
        assert!(evaluate_scenario(&invalid).is_err());
    }

    #[test]
    fn duplicate_port_ids_and_negative_flow_fail_closed() {
        let mut duplicate = scenario("TURN_90_DEG");
        duplicate.ports[1].port_id = duplicate.ports[0].port_id.clone();
        assert!(evaluate_scenario(&duplicate).is_err());

        let mut negative = scenario("TURN_90_DEG");
        negative.ports[0].mass_flow_kg_s = -1.0;
        assert!(evaluate_scenario(&negative).is_err());
    }

    #[test]
    fn non_finite_inputs_and_derived_overflow_fail_closed() {
        let mut non_finite = scenario("TURN_90_DEG");
        non_finite.ports[0].velocity_inertial_m_s.x = f64::NAN;
        assert!(evaluate_scenario(&non_finite).is_err());

        let mut overflow = scenario("TURN_90_DEG");
        overflow.ports[0].mass_flow_kg_s = f64::MAX;
        overflow.ports[0].velocity_inertial_m_s.x = f64::MAX;
        overflow.ports[1].mass_flow_kg_s = 0.0;
        overflow.mass_accumulation_kg_s = f64::MAX;
        assert!(evaluate_scenario(&overflow).is_err());
    }

    #[test]
    fn zero_flow_ports_are_valid_when_mass_and_momentum_ledgers_close() {
        let zero = ControlVolumeScenario {
            case_id: "ZERO_FLOW".to_owned(),
            mass_accumulation_kg_s: 0.0,
            momentum_accumulation_n: Vector3::ZERO,
            other_external_force_n: Vector3::ZERO,
            ports: vec![MassFluxPort {
                port_id: "ZERO".to_owned(),
                direction: FlowDirection::Inflow,
                mass_flow_kg_s: 0.0,
                velocity_inertial_m_s: Vector3::new(1.0, 2.0, 3.0),
            }],
        };
        let result = evaluate_scenario(&zero).expect("zero flow is invariant");
        close_vec(result.advective_momentum_flux_n, Vector3::ZERO, 0.0);
    }

    #[test]
    fn parser_rejects_duplicate_case_ids_and_unknown_port_cases() {
        let duplicate_cases = format!("{CASES}STRAIGHT_THROUGH,0,0,0,0,0,0,0\n");
        assert!(parse_corpus(&duplicate_cases, PORTS).is_err());

        let unknown_port = format!(
            "{PORTS}UNKNOWN_CASE,PORT,Inflow,1,0,0,0\n"
        );
        assert!(parse_corpus(CASES, &unknown_port).is_err());
    }

    #[test]
    fn parser_rejects_non_finite_or_invalid_direction_rows() {
        let bad_number = CASES.replace("STRAIGHT_THROUGH,0,0,0,0,0,0,0", "STRAIGHT_THROUGH,NaN,0,0,0,0,0,0");
        assert!(parse_corpus(&bad_number, PORTS).is_err());
        let bad_direction = PORTS.replace("Inflow", "Sideways");
        assert!(parse_corpus(CASES, &bad_direction).is_err());
    }

}
