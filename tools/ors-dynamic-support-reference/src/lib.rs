//! Reduced-order reference for a synthetic two-mass force-port transient.
//!
//! This is NOT an orbital-ring, magnetic bearing, levitation, field, or
//! structural model. It tests only a synthetic linear spring/damper between
//! two lumped masses. The input and output claim class is
//! SYNTHETIC_NOT_PHYSICAL.
//!
//! Equal-and-opposite external bias forces hold the preloaded spring at rest.
//! At the configured release time both biases are removed. The resulting
//! damped oscillator has a separate closed-form test oracle.

use std::error::Error;
use std::fmt;

pub const SYNTHETIC_FORCE_LAW_ID: &str = "synthetic-linear-spring-damper-v1";
pub const SYNTHETIC_CLAIM_CLASS: &str = "SYNTHETIC_NOT_PHYSICAL";
pub const MAX_TIME_STEPS: usize = 1_000_000;
const EXPECTED_HEADER: &str = "case_id,force_law_id,claim_class,mass_stream_kg,mass_support_kg,stiffness_n_m,damping_n_s_m,preload_n,release_time_s,duration_s,dt_s,expected_final_separation_m,expected_final_relative_velocity_mps,absolute_tolerance";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelError {
    InvalidCaseId,
    UnsupportedForceLaw,
    PhysicalClaimNotAdmitted,
    NonFiniteInput,
    NonPositiveMass,
    NonPositiveStiffness,
    NegativeDamping,
    NegativePreload,
    InvalidDuration,
    ReleaseOutsideDuration,
    NonIntegralTimeGrid,
    TooManyTimeSteps,
    NonFiniteResult,
    InvalidCorpus,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidCaseId => "case ID must use only ASCII letters, digits, '_' or '-'",
            Self::UnsupportedForceLaw => "force law is not admitted by this synthetic reference",
            Self::PhysicalClaimNotAdmitted => "physical claim classes are not admitted by this reference",
            Self::NonFiniteInput => "all numeric inputs must be finite",
            Self::NonPositiveMass => "both lumped masses must be positive",
            Self::NonPositiveStiffness => "synthetic spring stiffness must be positive",
            Self::NegativeDamping => "synthetic damping must be non-negative",
            Self::NegativePreload => "preload must be non-negative",
            Self::InvalidDuration => "duration and time step must be positive",
            Self::ReleaseOutsideDuration => "release time must lie in [0, duration]",
            Self::NonIntegralTimeGrid => "duration and release time must align to the fixed time step",
            Self::TooManyTimeSteps => "time grid exceeds the fixed maximum step count",
            Self::NonFiniteResult => "derived state or ledger value is non-finite",
            Self::InvalidCorpus => "frozen corpus has an invalid header, row, value or duplicate ID",
        };
        f.write_str(message)
    }
}

impl Error for ModelError {}

#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub case_id: String,
    pub force_law_id: String,
    pub claim_class: String,
    pub mass_stream_kg: f64,
    pub mass_support_kg: f64,
    pub stiffness_n_m: f64,
    pub damping_n_s_m: f64,
    pub preload_n: f64,
    pub release_time_s: f64,
    pub duration_s: f64,
    pub dt_s: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoldenCase {
    pub scenario: Scenario,
    pub expected_final_separation_m: f64,
    pub expected_final_relative_velocity_mps: f64,
    pub absolute_tolerance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimulationReport {
    pub time_steps: usize,
    pub initial_energy_j: f64,
    pub final_separation_m: f64,
    pub final_relative_velocity_mps: f64,
    pub peak_abs_separation_m: f64,
    pub peak_abs_relative_velocity_mps: f64,
    pub peak_abs_port_force_n: f64,
    pub final_dissipated_energy_j: f64,
    pub final_external_work_j: f64,
    pub max_abs_linear_momentum_residual_kg_mps: f64,
    pub max_abs_energy_balance_residual_j: f64,
}

fn valid_case_id(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
        })
}

fn finite(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn integer_steps(value_s: f64, dt_s: f64) -> Result<usize, ModelError> {
    let ratio = value_s / dt_s;
    if !ratio.is_finite() || ratio < 0.0 {
        return Err(ModelError::NonIntegralTimeGrid);
    }
    let rounded = ratio.round();
    let tolerance = 1.0e-10 * ratio.abs().max(1.0);
    if (ratio - rounded).abs() > tolerance {
        return Err(ModelError::NonIntegralTimeGrid);
    }
    if rounded > MAX_TIME_STEPS as f64 {
        return Err(ModelError::TooManyTimeSteps);
    }
    Ok(rounded as usize)
}

pub fn validate_scenario(scenario: &Scenario) -> Result<(usize, usize), ModelError> {
    if !valid_case_id(&scenario.case_id) {
        return Err(ModelError::InvalidCaseId);
    }
    if scenario.force_law_id != SYNTHETIC_FORCE_LAW_ID {
        return Err(ModelError::UnsupportedForceLaw);
    }
    if scenario.claim_class != SYNTHETIC_CLAIM_CLASS {
        return Err(ModelError::PhysicalClaimNotAdmitted);
    }

    let numeric = [
        scenario.mass_stream_kg,
        scenario.mass_support_kg,
        scenario.stiffness_n_m,
        scenario.damping_n_s_m,
        scenario.preload_n,
        scenario.release_time_s,
        scenario.duration_s,
        scenario.dt_s,
    ];
    if !finite(&numeric) {
        return Err(ModelError::NonFiniteInput);
    }
    if scenario.mass_stream_kg <= 0.0 || scenario.mass_support_kg <= 0.0 {
        return Err(ModelError::NonPositiveMass);
    }
    if scenario.stiffness_n_m <= 0.0 {
        return Err(ModelError::NonPositiveStiffness);
    }
    if scenario.damping_n_s_m < 0.0 {
        return Err(ModelError::NegativeDamping);
    }
    if scenario.preload_n < 0.0 {
        return Err(ModelError::NegativePreload);
    }
    if scenario.duration_s <= 0.0 || scenario.dt_s <= 0.0 {
        return Err(ModelError::InvalidDuration);
    }
    if scenario.release_time_s < 0.0 || scenario.release_time_s > scenario.duration_s {
        return Err(ModelError::ReleaseOutsideDuration);
    }

    let steps = integer_steps(scenario.duration_s, scenario.dt_s)?;
    let release_step = integer_steps(scenario.release_time_s, scenario.dt_s)?;
    if steps == 0 {
        return Err(ModelError::InvalidDuration);
    }
    if steps > MAX_TIME_STEPS {
        return Err(ModelError::TooManyTimeSteps);
    }
    Ok((steps, release_step))
}

fn port_force(state: &[f64; 6], scenario: &Scenario) -> f64 {
    let separation = state[0] - state[2];
    let relative_velocity = state[1] - state[3];
    -scenario.stiffness_n_m * separation - scenario.damping_n_s_m * relative_velocity
}

fn derivative(
    state: &[f64; 6],
    bias_force_n: f64,
    scenario: &Scenario,
) -> Result<[f64; 6], ModelError> {
    let relative_velocity = state[1] - state[3];
    let force_on_stream = port_force(state, scenario);
    let force_on_stream_total = force_on_stream + bias_force_n;
    let force_on_support_total = -force_on_stream - bias_force_n;
    let output = [
        state[1],
        force_on_stream_total / scenario.mass_stream_kg,
        state[3],
        force_on_support_total / scenario.mass_support_kg,
        scenario.damping_n_s_m * relative_velocity * relative_velocity,
        bias_force_n * relative_velocity,
    ];
    if finite(&output) {
        Ok(output)
    } else {
        Err(ModelError::NonFiniteResult)
    }
}

fn add_scaled(state: &[f64; 6], slope: &[f64; 6], scale: f64) -> [f64; 6] {
    let mut output = [0.0; 6];
    for index in 0..6 {
        output[index] = state[index] + slope[index] * scale;
    }
    output
}

fn rk4_step(
    state: &[f64; 6],
    step_s: f64,
    bias_force_n: f64,
    scenario: &Scenario,
) -> Result<[f64; 6], ModelError> {
    let k1 = derivative(state, bias_force_n, scenario)?;
    let stage2 = add_scaled(state, &k1, 0.5 * step_s);
    let k2 = derivative(&stage2, bias_force_n, scenario)?;
    let stage3 = add_scaled(state, &k2, 0.5 * step_s);
    let k3 = derivative(&stage3, bias_force_n, scenario)?;
    let stage4 = add_scaled(state, &k3, step_s);
    let k4 = derivative(&stage4, bias_force_n, scenario)?;

    let mut output = [0.0; 6];
    for index in 0..6 {
        output[index] = state[index]
            + (step_s / 6.0)
                * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
    }
    if finite(&output) {
        Ok(output)
    } else {
        Err(ModelError::NonFiniteResult)
    }
}

fn mechanical_energy(state: &[f64; 6], scenario: &Scenario) -> f64 {
    let separation = state[0] - state[2];
    let kinetic = 0.5 * scenario.mass_stream_kg * state[1] * state[1]
        + 0.5 * scenario.mass_support_kg * state[3] * state[3];
    let spring = 0.5 * scenario.stiffness_n_m * separation * separation;
    kinetic + spring
}

/// Run the declared two-mass synthetic force-port case.
///
/// State slots 4 and 5 accumulate viscous dissipation and external bias work.
/// The energy residual is E_mechanical + E_dissipated - W_external - E_initial.
pub fn evaluate(scenario: &Scenario) -> Result<SimulationReport, ModelError> {
    let (steps, release_step) = validate_scenario(scenario)?;
    let initial_separation = scenario.preload_n / scenario.stiffness_n_m;
    if !initial_separation.is_finite() {
        return Err(ModelError::NonFiniteResult);
    }

    // Symmetric coordinates set the initial centre of mass and total momentum to zero.
    let mut state = [
        0.5 * initial_separation,
        0.0,
        -0.5 * initial_separation,
        0.0,
        0.0,
        0.0,
    ];
    let initial_energy = mechanical_energy(&state, scenario);
    if !initial_energy.is_finite() {
        return Err(ModelError::NonFiniteResult);
    }

    let total_mass = scenario.mass_stream_kg + scenario.mass_support_kg;
    if !total_mass.is_finite() {
        return Err(ModelError::NonFiniteResult);
    }

    let mut peak_abs_separation_m = initial_separation.abs();
    let mut peak_abs_relative_velocity_mps = 0.0_f64;
    let mut peak_abs_port_force_n = scenario.preload_n.abs();
    let mut max_abs_linear_momentum_residual_kg_mps = 0.0_f64;
    let mut max_abs_energy_balance_residual_j = 0.0_f64;

    for step_index in 0..steps {
        // The event is grid-aligned, so no RK stage straddles its discontinuity.
        let bias = if step_index < release_step {
            scenario.preload_n
        } else {
            0.0
        };
        peak_abs_port_force_n = peak_abs_port_force_n.max(port_force(&state, scenario).abs());
        state = rk4_step(&state, scenario.dt_s, bias, scenario)?;

        let separation = state[0] - state[2];
        let relative_velocity = state[1] - state[3];
        let momentum = scenario.mass_stream_kg * state[1]
            + scenario.mass_support_kg * state[3];
        let energy_residual =
            mechanical_energy(&state, scenario) + state[4] - state[5] - initial_energy;

        if !finite(&[separation, relative_velocity, momentum, energy_residual]) {
            return Err(ModelError::NonFiniteResult);
        }

        peak_abs_separation_m = peak_abs_separation_m.max(separation.abs());
        peak_abs_relative_velocity_mps =
            peak_abs_relative_velocity_mps.max(relative_velocity.abs());
        max_abs_linear_momentum_residual_kg_mps =
            max_abs_linear_momentum_residual_kg_mps.max(momentum.abs());
        max_abs_energy_balance_residual_j =
            max_abs_energy_balance_residual_j.max(energy_residual.abs());
    }

    Ok(SimulationReport {
        time_steps: steps,
        initial_energy_j: initial_energy,
        final_separation_m: state[0] - state[2],
        final_relative_velocity_mps: state[1] - state[3],
        peak_abs_separation_m,
        peak_abs_relative_velocity_mps,
        peak_abs_port_force_n,
        final_dissipated_energy_j: state[4],
        final_external_work_j: state[5],
        max_abs_linear_momentum_residual_kg_mps,
        max_abs_energy_balance_residual_j,
    })
}

pub fn compare_to_golden(case: &GoldenCase, report: &SimulationReport) -> bool {
    let state_match = (report.final_separation_m - case.expected_final_separation_m).abs()
        <= case.absolute_tolerance
        && (report.final_relative_velocity_mps - case.expected_final_relative_velocity_mps).abs()
            <= case.absolute_tolerance;
    let momentum_scale = (case.scenario.mass_stream_kg + case.scenario.mass_support_kg)
        * report.peak_abs_relative_velocity_mps.max(1.0);
    let momentum_match =
        report.max_abs_linear_momentum_residual_kg_mps <= 1.0e-10 * momentum_scale.max(1.0);
    let energy_scale = report.initial_energy_j.max(1.0);
    let energy_match =
        report.max_abs_energy_balance_residual_j <= 1.0e-6 * energy_scale;
    state_match && momentum_match && energy_match
}

fn parse_number(value: &str) -> Result<f64, ModelError> {
    value.parse::<f64>().map_err(|_| ModelError::InvalidCorpus)
}

/// Parse the frozen CSV corpus. Quoted fields are intentionally unsupported.
pub fn parse_corpus(input: &str) -> Result<Vec<GoldenCase>, ModelError> {
    let mut lines = input.lines();
    if lines.next().map(str::trim) != Some(EXPECTED_HEADER) {
        return Err(ModelError::InvalidCorpus);
    }

    let mut cases: Vec<GoldenCase> = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 14 {
            return Err(ModelError::InvalidCorpus);
        }
        let scenario = Scenario {
            case_id: fields[0].to_owned(),
            force_law_id: fields[1].to_owned(),
            claim_class: fields[2].to_owned(),
            mass_stream_kg: parse_number(fields[3])?,
            mass_support_kg: parse_number(fields[4])?,
            stiffness_n_m: parse_number(fields[5])?,
            damping_n_s_m: parse_number(fields[6])?,
            preload_n: parse_number(fields[7])?,
            release_time_s: parse_number(fields[8])?,
            duration_s: parse_number(fields[9])?,
            dt_s: parse_number(fields[10])?,
        };
        let expected_final_separation_m = parse_number(fields[11])?;
        let expected_final_relative_velocity_mps = parse_number(fields[12])?;
        let absolute_tolerance = parse_number(fields[13])?;

        validate_scenario(&scenario)?;
        if !finite(&[
            expected_final_separation_m,
            expected_final_relative_velocity_mps,
            absolute_tolerance,
        ]) || absolute_tolerance <= 0.0
        {
            return Err(ModelError::InvalidCorpus);
        }
        if cases.iter().any(|existing| existing.scenario.case_id == scenario.case_id) {
            return Err(ModelError::InvalidCorpus);
        }
        cases.push(GoldenCase {
            scenario,
            expected_final_separation_m,
            expected_final_relative_velocity_mps,
            absolute_tolerance,
        });
    }

    if cases.is_empty() {
        return Err(ModelError::InvalidCorpus);
    }
    Ok(cases)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario() -> Scenario {
        Scenario {
            case_id: "TEST_RELEASE".to_owned(),
            force_law_id: SYNTHETIC_FORCE_LAW_ID.to_owned(),
            claim_class: SYNTHETIC_CLAIM_CLASS.to_owned(),
            mass_stream_kg: 1_000.0,
            mass_support_kg: 2_000.0,
            stiffness_n_m: 1_000_000.0,
            damping_n_s_m: 2_000.0,
            preload_n: 100_000.0,
            release_time_s: 0.0,
            duration_s: 0.4,
            dt_s: 0.0001,
        }
    }

    fn expect_error(input: Scenario, expected: ModelError) {
        assert_eq!(validate_scenario(&input), Err(expected));
    }

    fn exact_underdamped(
        mass_stream_kg: f64,
        mass_support_kg: f64,
        stiffness_n_m: f64,
        damping_n_s_m: f64,
        preload_n: f64,
        elapsed_s: f64,
    ) -> (f64, f64) {
        let reduced_mass =
            mass_stream_kg * mass_support_kg / (mass_stream_kg + mass_support_kg);
        let natural_frequency = (stiffness_n_m / reduced_mass).sqrt();
        let damping_ratio =
            damping_n_s_m / (2.0 * (stiffness_n_m * reduced_mass).sqrt());
        assert!(damping_ratio < 1.0);
        let damped_frequency =
            natural_frequency * (1.0 - damping_ratio * damping_ratio).sqrt();
        let amplitude = preload_n / stiffness_n_m;
        let phase = damping_ratio / (1.0 - damping_ratio * damping_ratio).sqrt();
        let angle = damped_frequency * elapsed_s;
        let cosine = angle.cos();
        let sine = angle.sin();
        let decay = (-damping_ratio * natural_frequency * elapsed_s).exp();
        let bracket = cosine + phase * sine;
        let separation = amplitude * decay * bracket;
        let velocity = amplitude
            * decay
            * (-damped_frequency * sine
                + phase * damped_frequency * cosine
                - damping_ratio * natural_frequency * bracket);
        (separation, velocity)
    }

    #[test]
    fn frozen_corpus_matches_independent_closed_form_cases() {
        let cases = parse_corpus(include_str!("../cases-v0.1.csv")).expect("valid corpus");
        assert_eq!(cases.len(), 7);
        for case in &cases {
            let report = evaluate(&case.scenario).expect("simulation");
            assert!(compare_to_golden(case, &report), "case {}", case.scenario.case_id);
            let elapsed = case.scenario.duration_s - case.scenario.release_time_s;
            if case.scenario.preload_n > 0.0 && elapsed > 0.0 {
                let (expected_y, expected_v) = exact_underdamped(
                    case.scenario.mass_stream_kg,
                    case.scenario.mass_support_kg,
                    case.scenario.stiffness_n_m,
                    case.scenario.damping_n_s_m,
                    case.scenario.preload_n,
                    elapsed,
                );
                assert!(
                    (report.final_separation_m - expected_y).abs() <= case.absolute_tolerance,
                    "{} separation: {} vs {}",
                    case.scenario.case_id,
                    report.final_separation_m,
                    expected_y
                );
                assert!(
                    (report.final_relative_velocity_mps - expected_v).abs()
                        <= case.absolute_tolerance,
                    "{} velocity: {} vs {}",
                    case.scenario.case_id,
                    report.final_relative_velocity_mps,
                    expected_v
                );
            }
        }
    }

    #[test]
    fn equal_and_opposite_port_preserves_linear_momentum() {
        for case in parse_corpus(include_str!("../cases-v0.1.csv")).expect("valid corpus") {
            let report = evaluate(&case.scenario).expect("simulation");
            assert!(report.max_abs_linear_momentum_residual_kg_mps < 1.0e-7);
        }
    }

    #[test]
    fn mechanical_energy_plus_heat_minus_external_work_closes() {
        for case in parse_corpus(include_str!("../cases-v0.1.csv")).expect("valid corpus") {
            let report = evaluate(&case.scenario).expect("simulation");
            assert!(
                report.max_abs_energy_balance_residual_j
                    <= 1.0e-6 * report.initial_energy_j.max(1.0),
                "{} energy residual {}",
                case.scenario.case_id,
                report.max_abs_energy_balance_residual_j
            );
            assert!(report.final_dissipated_energy_j >= 0.0);
        }
    }

    #[test]
    fn zero_damping_is_conservative_within_rk4_error() {
        let mut input = scenario();
        input.damping_n_s_m = 0.0;
        let report = evaluate(&input).expect("simulation");
        assert!(report.final_dissipated_energy_j.abs() < 1.0e-12);
        assert!(report.max_abs_energy_balance_residual_j < 1.0e-3);
    }

    #[test]
    fn zero_preload_is_an_equilibrium_control() {
        let mut input = scenario();
        input.case_id = "ZERO_PRELOAD".to_owned();
        input.preload_n = 0.0;
        let report = evaluate(&input).expect("simulation");
        assert_eq!(report.final_separation_m, 0.0);
        assert_eq!(report.final_relative_velocity_mps, 0.0);
        assert_eq!(report.peak_abs_relative_velocity_mps, 0.0);
    }

    #[test]
    fn release_at_duration_is_a_preloaded_equilibrium_control() {
        let mut input = scenario();
        input.case_id = "NO_RELEASE".to_owned();
        input.release_time_s = input.duration_s;
        let report = evaluate(&input).expect("simulation");
        assert!((report.final_separation_m - 0.1).abs() < 1.0e-12);
        assert!(report.final_relative_velocity_mps.abs() < 1.0e-12);
    }

    #[test]
    fn timestep_refinement_reduces_error_against_closed_form() {
        let mut coarse = scenario();
        coarse.dt_s = 0.0002;
        let mut fine = scenario();
        fine.dt_s = 0.0001;
        let coarse_report = evaluate(&coarse).expect("coarse simulation");
        let fine_report = evaluate(&fine).expect("fine simulation");
        let (exact_y, exact_v) = exact_underdamped(
            fine.mass_stream_kg,
            fine.mass_support_kg,
            fine.stiffness_n_m,
            fine.damping_n_s_m,
            fine.preload_n,
            fine.duration_s,
        );
        let coarse_error = (coarse_report.final_separation_m - exact_y).abs()
            + (coarse_report.final_relative_velocity_mps - exact_v).abs();
        let fine_error = (fine_report.final_separation_m - exact_y).abs()
            + (fine_report.final_relative_velocity_mps - exact_v).abs();
        assert!(fine_error < coarse_error);
    }

    #[test]
    fn rejects_zero_mass() {
        let mut input = scenario();
        input.mass_stream_kg = 0.0;
        expect_error(input, ModelError::NonPositiveMass);
    }

    #[test]
    fn rejects_zero_stiffness() {
        let mut input = scenario();
        input.stiffness_n_m = 0.0;
        expect_error(input, ModelError::NonPositiveStiffness);
    }

    #[test]
    fn rejects_negative_damping() {
        let mut input = scenario();
        input.damping_n_s_m = -1.0;
        expect_error(input, ModelError::NegativeDamping);
    }

    #[test]
    fn rejects_negative_preload() {
        let mut input = scenario();
        input.preload_n = -1.0;
        expect_error(input, ModelError::NegativePreload);
    }

    #[test]
    fn rejects_nonfinite_inputs() {
        let mut input = scenario();
        input.mass_stream_kg = f64::NAN;
        expect_error(input, ModelError::NonFiniteInput);
    }

    #[test]
    fn rejects_unknown_force_law() {
        let mut input = scenario();
        input.force_law_id = "magnetic-bearing-v1".to_owned();
        expect_error(input, ModelError::UnsupportedForceLaw);
    }

    #[test]
    fn rejects_physical_claim_promotion() {
        let mut input = scenario();
        input.claim_class = "PHYSICAL".to_owned();
        expect_error(input, ModelError::PhysicalClaimNotAdmitted);
    }

    #[test]
    fn rejects_non_aligned_release_event() {
        let mut input = scenario();
        input.release_time_s = 0.00015;
        expect_error(input, ModelError::NonIntegralTimeGrid);
    }

    #[test]
    fn rejects_release_after_duration() {
        let mut input = scenario();
        input.release_time_s = input.duration_s + input.dt_s;
        expect_error(input, ModelError::ReleaseOutsideDuration);
    }

    #[test]
    fn rejects_excessive_step_count() {
        let mut input = scenario();
        input.duration_s = 200.0;
        input.dt_s = 0.0001;
        expect_error(input, ModelError::TooManyTimeSteps);
    }

    #[test]
    fn rejects_nonfinite_derived_initial_separation() {
        let mut input = scenario();
        input.preload_n = 1.0e308;
        input.stiffness_n_m = 1.0e-308;
        assert_eq!(evaluate(&input), Err(ModelError::NonFiniteResult));
    }

    #[test]
    fn rejects_invalid_corpus_header_and_duplicate_ids() {
        let corpus = include_str!("../cases-v0.1.csv");
        assert_eq!(parse_corpus("case_id,wrong\n"), Err(ModelError::InvalidCorpus));
        let first_case = corpus.lines().nth(1).expect("first case");
        let duplicate = format!("{EXPECTED_HEADER}\n{first_case}\n{first_case}\n");
        assert_eq!(parse_corpus(&duplicate), Err(ModelError::InvalidCorpus));
    }

    #[test]
    fn rejects_invalid_case_identifier() {
        let mut input = scenario();
        input.case_id = "bad id".to_owned();
        expect_error(input, ModelError::InvalidCaseId);
    }
}
