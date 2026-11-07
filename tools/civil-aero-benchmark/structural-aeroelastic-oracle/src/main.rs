// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Bounded analytical oracle only. This is not a finite-element solver,
// aeroelastic solver, aircraft model, design allowable, or certification tool.

use std::fmt::Write as _;

const CONFIGURATION_SHA256: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const GEOMETRY_SHA256: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const LOADING_STATE_SHA256: &str =
    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const MISSION_PHASE_SHA256: &str =
    "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const ENVIRONMENT_SHA256: &str =
    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

#[derive(Clone, Copy, Debug, PartialEq)]
struct Vec3([f64; 3]);

impl Vec3 {
    const ZERO: Self = Self([0.0, 0.0, 0.0]);

    fn add(self, other: Self) -> Self {
        Self([
            self.0[0] + other.0[0],
            self.0[1] + other.0[1],
            self.0[2] + other.0[2],
        ])
    }

    fn scale(self, factor: f64) -> Self {
        Self([
            self.0[0] * factor,
            self.0[1] * factor,
            self.0[2] * factor,
        ])
    }

    fn cross(self, other: Self) -> Self {
        Self([
            self.0[1] * other.0[2] - self.0[2] * other.0[1],
            self.0[2] * other.0[0] - self.0[0] * other.0[2],
            self.0[0] * other.0[1] - self.0[1] * other.0[0],
        ])
    }

    fn is_finite(self) -> bool {
        self.0.iter().all(|value| value.is_finite())
    }

    fn max_abs_difference(self, other: Self) -> f64 {
        (0..3)
            .map(|axis| (self.0[axis] - other.0[axis]).abs())
            .fold(0.0, f64::max)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StateIdentity {
    configuration_sha256: &'static str,
    geometry_sha256: &'static str,
    loading_state_sha256: &'static str,
    mission_phase_sha256: &'static str,
    environment_sha256: &'static str,
}

const IDENTITY: StateIdentity = StateIdentity {
    configuration_sha256: CONFIGURATION_SHA256,
    geometry_sha256: GEOMETRY_SHA256,
    loading_state_sha256: LOADING_STATE_SHA256,
    mission_phase_sha256: MISSION_PHASE_SHA256,
    environment_sha256: ENVIRONMENT_SHA256,
};

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_identity(identity: StateIdentity) -> Result<(), String> {
    let digests = [
        identity.configuration_sha256,
        identity.geometry_sha256,
        identity.loading_state_sha256,
        identity.mission_phase_sha256,
        identity.environment_sha256,
    ];
    if digests.iter().all(|digest| is_sha256_hex(digest)) {
        Ok(())
    } else {
        Err("identity contains a malformed SHA-256 digest".to_owned())
    }
}

fn require_same_identity(left: StateIdentity, right: StateIdentity) -> Result<(), String> {
    validate_identity(left)?;
    validate_identity(right)?;
    if left == right {
        Ok(())
    } else {
        Err("analysis products refer to different aircraft/mission states".to_owned())
    }
}

fn require_positive_finite(value: f64, field: &str) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{field} must be finite and positive"))
    }
}

fn require_finite(value: f64, field: &str) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("{field} must be finite"))
    }
}

#[derive(Clone, Copy, Debug)]
struct BeamInput {
    youngs_modulus_pa: f64,
    second_moment_m4: f64,
    span_m: f64,
    tip_force_n: f64,
    uniform_load_n_per_m: f64,
    extreme_fibre_distance_m: f64,
}

#[derive(Clone, Copy, Debug)]
struct BeamResult {
    root_moment_nm: f64,
    tip_deflection_m: f64,
    tip_slope_rad: f64,
    root_bending_stress_pa: f64,
}

fn cantilever_beam(input: BeamInput) -> Result<BeamResult, String> {
    require_positive_finite(input.youngs_modulus_pa, "youngs_modulus_pa")?;
    require_positive_finite(input.second_moment_m4, "second_moment_m4")?;
    require_positive_finite(input.span_m, "span_m")?;
    require_finite(input.tip_force_n, "tip_force_n")?;
    require_finite(input.uniform_load_n_per_m, "uniform_load_n_per_m")?;
    require_positive_finite(input.extreme_fibre_distance_m, "extreme_fibre_distance_m")?;

    let length = input.span_m;
    let ei = input.youngs_modulus_pa * input.second_moment_m4;
    let root_moment = input.tip_force_n * length
        + input.uniform_load_n_per_m * length.powi(2) / 2.0;
    let tip_deflection = input.tip_force_n * length.powi(3) / (3.0 * ei)
        + input.uniform_load_n_per_m * length.powi(4) / (8.0 * ei);
    let tip_slope = input.tip_force_n * length.powi(2) / (2.0 * ei)
        + input.uniform_load_n_per_m * length.powi(3) / (6.0 * ei);
    let root_stress =
        root_moment * input.extreme_fibre_distance_m / input.second_moment_m4;

    for (field, value) in [
        ("root_moment_nm", root_moment),
        ("tip_deflection_m", tip_deflection),
        ("tip_slope_rad", tip_slope),
        ("root_bending_stress_pa", root_stress),
    ] {
        require_finite(value, field)?;
    }

    Ok(BeamResult {
        root_moment_nm: root_moment,
        tip_deflection_m: tip_deflection,
        tip_slope_rad: tip_slope,
        root_bending_stress_pa: root_stress,
    })
}

#[derive(Clone, Copy, Debug)]
struct WingBoxInput {
    cap_area_m2: f64,
    cap_separation_m: f64,
    median_width_m: f64,
    median_height_m: f64,
    wall_thickness_m: f64,
    youngs_modulus_pa: f64,
    shear_modulus_pa: f64,
    root_bending_moment_nm: f64,
    applied_torque_nm: f64,
    span_m: f64,
}

#[derive(Clone, Copy, Debug)]
struct WingBoxResult {
    bending_second_moment_m4: f64,
    bending_stress_pa: f64,
    torsion_constant_m4: f64,
    torsional_shear_flow_n_per_m: f64,
    wall_shear_stress_pa: f64,
    tip_twist_rad: f64,
}

fn idealized_wing_box(input: WingBoxInput) -> Result<WingBoxResult, String> {
    for (field, value) in [
        ("cap_area_m2", input.cap_area_m2),
        ("cap_separation_m", input.cap_separation_m),
        ("median_width_m", input.median_width_m),
        ("median_height_m", input.median_height_m),
        ("wall_thickness_m", input.wall_thickness_m),
        ("youngs_modulus_pa", input.youngs_modulus_pa),
        ("shear_modulus_pa", input.shear_modulus_pa),
        ("span_m", input.span_m),
    ] {
        require_positive_finite(value, field)?;
    }
    require_finite(input.root_bending_moment_nm, "root_bending_moment_nm")?;
    require_finite(input.applied_torque_nm, "applied_torque_nm")?;

    // Two-cap idealization for bending: skin/stringer contributions are omitted.
    let second_moment =
        2.0 * input.cap_area_m2 * (input.cap_separation_m / 2.0).powi(2);
    // Single-cell, uniform-thickness, thin-wall Bredt-Batho approximation.
    let enclosed_area = input.median_width_m * input.median_height_m;
    let wall_integral = 2.0
        * (input.median_width_m + input.median_height_m)
        / input.wall_thickness_m;
    let torsion_constant = 4.0 * enclosed_area.powi(2) / wall_integral;
    let shear_flow = input.applied_torque_nm / (2.0 * enclosed_area);
    let wall_shear_stress = shear_flow / input.wall_thickness_m;
    let bending_stress = input.root_bending_moment_nm
        * (input.cap_separation_m / 2.0)
        / second_moment;
    let tip_twist = input.applied_torque_nm * input.span_m
        / (input.shear_modulus_pa * torsion_constant);

    let outputs = [
        ("bending_second_moment_m4", second_moment),
        ("bending_stress_pa", bending_stress),
        ("torsion_constant_m4", torsion_constant),
        ("torsional_shear_flow_n_per_m", shear_flow),
        ("wall_shear_stress_pa", wall_shear_stress),
        ("tip_twist_rad", tip_twist),
    ];
    for (field, value) in outputs {
        require_finite(value, field)?;
    }

    Ok(WingBoxResult {
        bending_second_moment_m4: second_moment,
        bending_stress_pa: bending_stress,
        torsion_constant_m4: torsion_constant,
        torsional_shear_flow_n_per_m: shear_flow,
        wall_shear_stress_pa: wall_shear_stress,
        tip_twist_rad: tip_twist,
    })
}

#[derive(Clone, Copy, Debug)]
struct PointLoad {
    x_m: f64,
    force_n: Vec3,
    couple_nm: Vec3,
}

#[derive(Clone, Copy, Debug)]
struct StationLoad {
    x_m: f64,
    force_n: Vec3,
    couple_nm: Vec3,
}

fn point_load_resultant(loads: &[PointLoad]) -> Result<(Vec3, Vec3), String> {
    let mut total_force = Vec3::ZERO;
    let mut total_moment = Vec3::ZERO;
    for load in loads {
        require_finite(load.x_m, "load station")?;
        if !load.force_n.is_finite() || !load.couple_nm.is_finite() {
            return Err("load contains a non-finite force or couple".to_owned());
        }
        total_force = total_force.add(load.force_n);
        total_moment = total_moment
            .add(Vec3([load.x_m, 0.0, 0.0]).cross(load.force_n))
            .add(load.couple_nm);
    }
    Ok((total_force, total_moment))
}

fn station_load_resultant(loads: &[StationLoad]) -> Result<(Vec3, Vec3), String> {
    let mut total_force = Vec3::ZERO;
    let mut total_moment = Vec3::ZERO;
    for load in loads {
        require_finite(load.x_m, "target station")?;
        if !load.force_n.is_finite() || !load.couple_nm.is_finite() {
            return Err("transferred load contains a non-finite force or couple".to_owned());
        }
        total_force = total_force.add(load.force_n);
        total_moment = total_moment
            .add(Vec3([load.x_m, 0.0, 0.0]).cross(load.force_n))
            .add(load.couple_nm);
    }
    Ok((total_force, total_moment))
}

fn transfer_spanwise_loads(
    stations_m: &[f64],
    source_loads: &[PointLoad],
) -> Result<Vec<StationLoad>, String> {
    if stations_m.len() < 2 {
        return Err("at least two target stations are required".to_owned());
    }
    for station in stations_m {
        require_finite(*station, "target station")?;
    }
    if stations_m.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err("target stations must be strictly increasing".to_owned());
    }

    let mut target = stations_m
        .iter()
        .map(|station| StationLoad {
            x_m: *station,
            force_n: Vec3::ZERO,
            couple_nm: Vec3::ZERO,
        })
        .collect::<Vec<_>>();

    for load in source_loads {
        require_finite(load.x_m, "source load station")?;
        if !load.force_n.is_finite() || !load.couple_nm.is_finite() {
            return Err("source load contains a non-finite force or couple".to_owned());
        }
        if load.x_m < stations_m[0] || load.x_m > stations_m[stations_m.len() - 1] {
            return Err("source load lies outside the target span".to_owned());
        }

        let exact = stations_m.iter().position(|station| {
            (*station - load.x_m).abs() <= 1.0e-12
        });
        let contributions: Vec<(usize, f64)> = if let Some(index) = exact {
            vec![(index, 1.0)]
        } else {
            let Some(left) = stations_m
                .windows(2)
                .position(|pair| pair[0] < load.x_m && load.x_m < pair[1])
            else {
                return Err("source load could not be bracketed".to_owned());
            };
            let width = stations_m[left + 1] - stations_m[left];
            let right_weight = (load.x_m - stations_m[left]) / width;
            vec![(left, 1.0 - right_weight), (left + 1, right_weight)]
        };

        for (index, weight) in contributions {
            target[index].force_n =
                target[index].force_n.add(load.force_n.scale(weight));
            target[index].couple_nm =
                target[index].couple_nm.add(load.couple_nm.scale(weight));
        }
    }

    let (source_force, source_moment) = point_load_resultant(source_loads)?;
    let (target_force, target_moment) = station_load_resultant(&target)?;
    let force_residual = source_force.max_abs_difference(target_force);
    let moment_residual = source_moment.max_abs_difference(target_moment);
    let force_scale = source_force.0.iter().map(|v| v.abs()).fold(1.0, f64::max);
    let moment_scale = source_moment.0.iter().map(|v| v.abs()).fold(1.0, f64::max);
    if force_residual > 1.0e-10 * force_scale
        || moment_residual > 1.0e-10 * moment_scale
    {
        return Err("load transfer failed force/moment conservation".to_owned());
    }

    Ok(target)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CouplingStatus {
    Converged,
    NonConverged,
}

#[derive(Clone, Copy, Debug)]
struct CouplingResult {
    status: CouplingStatus,
    tip_deflection_m: Option<f64>,
    last_iterate_m: Option<f64>,
    residual_m: Option<f64>,
    gain: f64,
    iterations: usize,
}

fn solve_static_aeroelastic(
    compliance_m_per_n: f64,
    baseline_lift_n: f64,
    lift_feedback_n_per_m: f64,
    tolerance_m: f64,
    max_iterations: usize,
) -> Result<CouplingResult, String> {
    require_positive_finite(compliance_m_per_n, "compliance_m_per_n")?;
    require_finite(baseline_lift_n, "baseline_lift_n")?;
    require_finite(lift_feedback_n_per_m, "lift_feedback_n_per_m")?;
    require_positive_finite(tolerance_m, "tolerance_m")?;
    if max_iterations == 0 {
        return Err("max_iterations must be positive".to_owned());
    }

    let gain = compliance_m_per_n * lift_feedback_n_per_m;
    let mut deflection = 0.0;
    for iteration in 1..=max_iterations {
        let next = compliance_m_per_n
            * (baseline_lift_n + lift_feedback_n_per_m * deflection);
        if !next.is_finite() {
            return Ok(CouplingResult {
                status: CouplingStatus::NonConverged,
                tip_deflection_m: None,
                last_iterate_m: None,
                residual_m: None,
                gain,
                iterations: iteration,
            });
        }
        deflection = next;
        let equation_residual = (deflection
            - compliance_m_per_n
                * (baseline_lift_n + lift_feedback_n_per_m * deflection))
            .abs();
        if equation_residual <= tolerance_m {
            return Ok(CouplingResult {
                status: CouplingStatus::Converged,
                tip_deflection_m: Some(deflection),
                last_iterate_m: Some(deflection),
                residual_m: Some(equation_residual),
                gain,
                iterations: iteration,
            });
        }
    }

    let equation_residual = (deflection
        - compliance_m_per_n
            * (baseline_lift_n + lift_feedback_n_per_m * deflection))
        .abs();
    Ok(CouplingResult {
        status: CouplingStatus::NonConverged,
        tip_deflection_m: None,
        last_iterate_m: Some(deflection),
        residual_m: Some(equation_residual),
        gain,
        iterations: max_iterations,
    })
}

fn beam_case() -> BeamResult {
    cantilever_beam(BeamInput {
        youngs_modulus_pa: 70.0e9,
        second_moment_m4: 2.0e-6,
        span_m: 4.0,
        tip_force_n: 1_000.0,
        uniform_load_n_per_m: 200.0,
        extreme_fibre_distance_m: 0.05,
    })
    .expect("locked beam case must be valid")
}

fn wing_box_case() -> WingBoxResult {
    idealized_wing_box(WingBoxInput {
        cap_area_m2: 1.5e-3,
        cap_separation_m: 0.25,
        median_width_m: 0.6,
        median_height_m: 0.25,
        wall_thickness_m: 1.5e-3,
        youngs_modulus_pa: 70.0e9,
        shear_modulus_pa: 27.0e9,
        root_bending_moment_nm: 12_000.0,
        applied_torque_nm: 250.0,
        span_m: 4.0,
    })
    .expect("locked wing-box case must be valid")
}

fn load_transfer_case() -> (Vec3, Vec3, Vec3, Vec3, f64, f64) {
    let stations = [0.0, 2.0, 4.0];
    let source = [
        PointLoad {
            x_m: 1.0,
            force_n: Vec3([0.0, -1_000.0, 0.0]),
            couple_nm: Vec3([10.0, 20.0, 30.0]),
        },
        PointLoad {
            x_m: 3.0,
            force_n: Vec3([0.0, -500.0, 200.0]),
            couple_nm: Vec3([-2.0, 5.0, 7.0]),
        },
    ];
    let target = transfer_spanwise_loads(&stations, &source)
        .expect("locked load-transfer case must conserve resultants");
    let (source_force, source_moment) =
        point_load_resultant(&source).expect("source loads must be finite");
    let (target_force, target_moment) =
        station_load_resultant(&target).expect("target loads must be finite");
    (
        source_force,
        source_moment,
        target_force,
        target_moment,
        source_force.max_abs_difference(target_force),
        source_moment.max_abs_difference(target_moment),
    )
}

fn coupled_case() -> CouplingResult {
    solve_static_aeroelastic(1.0e-4, 1_000.0, 2_000.0, 1.0e-12, 100)
        .expect("locked coupled case must be valid")
}

fn divergent_control() -> CouplingResult {
    solve_static_aeroelastic(1.0e-4, 1_000.0, 12_000.0, 1.0e-12, 40)
        .expect("locked divergent control parameters must be valid")
}

fn json_number(value: Option<f64>) -> String {
    match value {
        Some(number) if number.is_finite() => format!("{number:.17e}"),
        _ => "null".to_owned(),
    }
}

fn json_vec(value: Vec3) -> String {
    format!(
        "[{:.17e},{:.17e},{:.17e}]",
        value.0[0], value.0[1], value.0[2]
    )
}

fn result_json() -> String {
    let beam = beam_case();
    let wing = wing_box_case();
    let (
        source_force,
        source_moment,
        target_force,
        target_moment,
        force_residual,
        moment_residual,
    ) = load_transfer_case();
    let coupled = coupled_case();
    let divergent = divergent_control();

    let coupled_status = match coupled.status {
        CouplingStatus::Converged => "CONVERGED",
        CouplingStatus::NonConverged => "NON_CONVERGED",
    };
    let divergent_status = match divergent.status {
        CouplingStatus::Converged => "CONVERGED",
        CouplingStatus::NonConverged => "NON_CONVERGED",
    };

    let mut output = String::new();
    write!(
        output,
        concat!(
            "{{",
            "\"schema_version\":\"aero-structural-analytic-oracle/0.1.0\",",
            "\"claim_ceiling\":\"bounded_analytical_software_verification_only\",",
            "\"aircraft_feasibility\":\"NOT_ASSESSED\",",
            "\"analysis_identity\":{{",
            "\"configuration_sha256\":\"{}\",",
            "\"geometry_sha256\":\"{}\",",
            "\"loading_state_sha256\":\"{}\",",
            "\"mission_phase_sha256\":\"{}\",",
            "\"environment_sha256\":\"{}\"}},",
            "\"identity_binding\":\"MATCHED\",",
            "\"beam\":{{",
            "\"root_moment_nm\":{:.17e},",
            "\"tip_deflection_m\":{:.17e},",
            "\"tip_slope_rad\":{:.17e},",
            "\"root_bending_stress_pa\":{:.17e}}},",
            "\"wing_box\":{{",
            "\"bending_second_moment_m4\":{:.17e},",
            "\"bending_stress_pa\":{:.17e},",
            "\"torsion_constant_m4\":{:.17e},",
            "\"torsional_shear_flow_n_per_m\":{:.17e},",
            "\"wall_shear_stress_pa\":{:.17e},",
            "\"tip_twist_rad\":{:.17e}}},",
            "\"load_transfer\":{{",
            "\"source_force_n\":{},",
            "\"mapped_force_n\":{},",
            "\"source_moment_nm\":{},",
            "\"mapped_moment_nm\":{},",
            "\"force_residual_n\":{:.17e},",
            "\"moment_residual_nm\":{:.17e}}},",
            "\"static_aeroelastic\":{{",
            "\"status\":\"{}\",",
            "\"gain\":{:.17e},",
            "\"iterations\":{},",
            "\"tip_deflection_m\":{},",
            "\"last_iterate_m\":{},",
            "\"residual_m\":{}}},",
            "\"divergent_control\":{{",
            "\"status\":\"{}\",",
            "\"gain\":{:.17e},",
            "\"iterations\":{},",
            "\"tip_deflection_m\":{},",
            "\"last_iterate_m\":{},",
            "\"residual_m\":{}}},",
            "\"unassessed_aircraft_claims\":[",
            "\"full_envelope_strength\",",
            "\"buckling_and_local_instability\",",
            "\"dynamic_flutter_and_buffet_clearance\",",
            "\"fatigue_and_service_life\",",
            "\"candidate_specific_validation\",",
            "\"airworthiness_and_certification\"]",
            "}}"
        ),
        IDENTITY.configuration_sha256,
        IDENTITY.geometry_sha256,
        IDENTITY.loading_state_sha256,
        IDENTITY.mission_phase_sha256,
        IDENTITY.environment_sha256,
        beam.root_moment_nm,
        beam.tip_deflection_m,
        beam.tip_slope_rad,
        beam.root_bending_stress_pa,
        wing.bending_second_moment_m4,
        wing.bending_stress_pa,
        wing.torsion_constant_m4,
        wing.torsional_shear_flow_n_per_m,
        wing.wall_shear_stress_pa,
        wing.tip_twist_rad,
        json_vec(source_force),
        json_vec(target_force),
        json_vec(source_moment),
        json_vec(target_moment),
        force_residual,
        moment_residual,
        coupled_status,
        coupled.gain,
        coupled.iterations,
        json_number(coupled.tip_deflection_m),
        json_number(coupled.last_iterate_m),
        json_number(coupled.residual_m),
        divergent_status,
        divergent.gain,
        divergent.iterations,
        json_number(divergent.tip_deflection_m),
        json_number(divergent.last_iterate_m),
        json_number(divergent.residual_m),
    )
    .expect("writing to String cannot fail");
    output
}

fn main() {
    if let Err(error) = validate_identity(IDENTITY) {
        eprintln!("identity_error={error}");
        std::process::exit(2);
    }
    println!("{}", result_json());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance * expected.abs().max(1.0),
            "actual={actual:.16e}, expected={expected:.16e}"
        );
    }

    #[test]
    fn beam_matches_locked_known_answer() {
        let result = beam_case();
        close(result.root_moment_nm, 5_600.0, 1.0e-12);
        close(result.tip_deflection_m, 0.1980952380952381, 1.0e-12);
        close(result.tip_slope_rad, 0.07238095238095238, 1.0e-12);
        close(result.root_bending_stress_pa, 140.0e6, 1.0e-12);
    }

    #[test]
    fn idealized_wing_box_matches_locked_known_answer() {
        let result = wing_box_case();
        close(result.bending_second_moment_m4, 4.6875e-5, 1.0e-12);
        close(result.bending_stress_pa, 32.0e6, 1.0e-12);
        close(result.torsion_constant_m4, 7.941176470588236e-5, 1.0e-12);
        close(result.torsional_shear_flow_n_per_m, 833.3333333333334, 1.0e-12);
        close(result.wall_shear_stress_pa, 555_555.5555555556, 1.0e-12);
        close(result.tip_twist_rad, 0.0004663923182441701, 1.0e-12);
    }

    #[test]
    fn spanwise_load_transfer_conserves_force_and_moment() {
        let (_, _, _, _, force_residual, moment_residual) = load_transfer_case();
        assert!(force_residual <= 1.0e-10);
        assert!(moment_residual <= 1.0e-10);
    }

    #[test]
    fn load_outside_structural_span_is_rejected() {
        let stations = [0.0, 2.0, 4.0];
        let loads = [PointLoad {
            x_m: 4.1,
            force_n: Vec3([0.0, 1.0, 0.0]),
            couple_nm: Vec3::ZERO,
        }];
        assert!(transfer_spanwise_loads(&stations, &loads).is_err());
    }

    #[test]
    fn duplicate_or_reversed_target_stations_are_rejected() {
        let loads = [PointLoad {
            x_m: 1.0,
            force_n: Vec3([0.0, 1.0, 0.0]),
            couple_nm: Vec3::ZERO,
        }];
        assert!(transfer_spanwise_loads(&[0.0, 2.0, 2.0], &loads).is_err());
        assert!(transfer_spanwise_loads(&[0.0, 3.0, 2.0], &loads).is_err());
    }

    #[test]
    fn stale_configuration_or_geometry_identity_is_rejected() {
        let mut stale = IDENTITY;
        stale.geometry_sha256 =
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert!(require_same_identity(IDENTITY, stale).is_err());
    }

    #[test]
    fn divergent_fixed_point_is_reported_not_promoted() {
        let result = divergent_control();
        assert_eq!(result.status, CouplingStatus::NonConverged);
        assert!(result.tip_deflection_m.is_none());
        assert_eq!(result.iterations, 40);
    }

    #[test]
    fn non_positive_stiffness_inputs_are_rejected() {
        let invalid = BeamInput {
            youngs_modulus_pa: 0.0,
            second_moment_m4: 2.0e-6,
            span_m: 4.0,
            tip_force_n: 1_000.0,
            uniform_load_n_per_m: 200.0,
            extreme_fibre_distance_m: 0.05,
        };
        assert!(cantilever_beam(invalid).is_err());
    }

    #[test]
    fn non_positive_wall_thickness_is_rejected() {
        let bad = WingBoxInput {
            cap_area_m2: 1.5e-3,
            cap_separation_m: 0.25,
            median_width_m: 0.6,
            median_height_m: 0.25,
            wall_thickness_m: 0.0,
            youngs_modulus_pa: 70.0e9,
            shear_modulus_pa: 27.0e9,
            root_bending_moment_nm: 12_000.0,
            applied_torque_nm: 250.0,
            span_m: 4.0,
        };
        assert!(idealized_wing_box(bad).is_err());
    }
}
