// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Analytical 2-DOF complex frequency-response oracle and sensor-observation
// generator. This is a synthetic known-answer benchmark, not an FEM solver,
// aircraft model, SHM classifier, NDE/POD estimator or structural clearance.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const CORPUS: &[u8] = include_bytes!("../data/frf-corpus-v0.1.json");

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    fn add(self, rhs: Self) -> Self {
        Self::new(self.re + rhs.re, self.im + rhs.im)
    }

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.re - rhs.re, self.im - rhs.im)
    }

    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.re * rhs.re - self.im * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }

    fn scale(self, value: f64) -> Self {
        Self::new(self.re * value, self.im * value)
    }

    fn div(self, rhs: Self) -> Result<Self> {
        let denominator = rhs.re * rhs.re + rhs.im * rhs.im;
        if !denominator.is_finite() || denominator <= 1.0e-28 {
            return fail("dynamic-stiffness determinant is singular or nearly singular");
        }
        Ok(Self::new(
            (self.re * rhs.re + self.im * rhs.im) / denominator,
            (self.im * rhs.re - self.re * rhs.im) / denominator,
        ))
    }

    fn magnitude(self) -> f64 {
        self.re.hypot(self.im)
    }

    fn phase(self) -> f64 {
        self.im.atan2(self.re)
    }

    fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct System {
    mass_kg: [f64; 2],
    stiffness_n_m: [[f64; 2]; 2],
    damping_ns_m: [[f64; 2]; 2],
    force_n: [Complex; 2],
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sensor {
    sensor_id: String,
    dof: u8,
    available: bool,
    gain: f64,
    phase_deg: f64,
    bias: Complex,
    fault: String,
    #[serde(default)]
    stuck_value: Option<Complex>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedObservedChannel {
    sensor_id: String,
    value: Option<Complex>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedFrequency {
    frequency_hz: f64,
    nominal_response_m_per_n: [Complex; 2],
    physical_response_m_per_n: [Complex; 2],
    observed_channels: Vec<ExpectedObservedChannel>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    case_id: String,
    state: String,
    nominal_model: System,
    physical_model: System,
    sensors: Vec<Sensor>,
    expected_disposition: String,
    frequencies_hz: Vec<f64>,
    expected_observation_rank: usize,
    expected_reference: Vec<ExpectedFrequency>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitSystem {
    mass: String,
    stiffness: String,
    damping: String,
    force: String,
    response: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tolerance {
    absolute: f64,
    relative: f64,
    phase_rad: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: String,
    corpus_id: String,
    evidence_class: String,
    claim_ceiling: String,
    physical_pod_claim: bool,
    aircraft_validation_claim: bool,
    frequency_convention: String,
    response_convention: String,
    unit_system: UnitSystem,
    tolerance: Tolerance,
    cases: Vec<Case>,
}

#[derive(Clone, Debug, Serialize)]
struct ObservedChannel {
    sensor_id: String,
    value: Option<Complex>,
}

#[derive(Clone, Debug, Serialize)]
struct ComputedFrequency {
    frequency_hz: f64,
    omega_rad_s: f64,
    nominal_response_m_per_n: [Complex; 2],
    physical_response_m_per_n: [Complex; 2],
    observed_channels: Vec<ObservedChannel>,
    nominal_vs_physical_complex_residual_m_per_n: [Complex; 2],
}

#[derive(Clone, Debug, Serialize)]
struct ComputedCase {
    case_id: String,
    state: String,
    scenario_sha256: String,
    expected_disposition: String,
    observation_rank: usize,
    frequencies: Vec<ComputedFrequency>,
}

#[derive(Clone, Debug, Serialize)]
struct OracleResult {
    schema_version: &'static str,
    corpus_id: String,
    corpus_sha256: String,
    case_count: usize,
    frequency_count_per_case: usize,
    computed_cases: Vec<ComputedCase>,
    physical_pod: &'static str,
    aircraft_validation: &'static str,
    structural_clearance: &'static str,
    claim_ceiling: &'static str,
}

#[derive(Debug)]
struct OracleError(String);

impl std::fmt::Display for OracleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for OracleError {}

type Result<T> = std::result::Result<T, OracleError>;

fn fail<T>(message: impl Into<String>) -> Result<T> {
    Err(OracleError(message.into()))
}

fn require(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition { Ok(()) } else { fail(message) }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn is_symmetric(matrix: [[f64; 2]; 2], tolerance: f64) -> bool {
    (matrix[0][1] - matrix[1][0]).abs() <= tolerance
}

fn validate_system(system: &System, label: &str) -> Result<()> {
    for (index, mass) in system.mass_kg.iter().enumerate() {
        require(
            mass.is_finite() && *mass > 0.0,
            format!("{label}: mass {} must be finite and positive", index + 1),
        )?;
    }
    for matrix in [&system.stiffness_n_m, &system.damping_ns_m] {
        for row in matrix {
            for value in row {
                require(
                    value.is_finite(),
                    format!("{label}: non-finite matrix entry"),
                )?;
            }
        }
    }
    require(
        is_symmetric(system.stiffness_n_m, 1.0e-12),
        format!("{label}: stiffness matrix is not symmetric"),
    )?;
    require(
        is_symmetric(system.damping_ns_m, 1.0e-12),
        format!("{label}: damping matrix is not symmetric"),
    )?;

    let k = system.stiffness_n_m;
    let k_det = k[0][0] * k[1][1] - k[0][1] * k[1][0];
    require(
        k[0][0] > 0.0 && k_det > 0.0,
        format!("{label}: stiffness matrix is not positive definite"),
    )?;

    let c = system.damping_ns_m;
    let c_det = c[0][0] * c[1][1] - c[0][1] * c[1][0];
    require(
        c[0][0] >= 0.0 && c[1][1] >= 0.0 && c_det >= -1.0e-10,
        format!("{label}: damping matrix is not positive semidefinite"),
    )?;

    for force in system.force_n {
        require(force.is_finite(), format!("{label}: non-finite force"))?;
    }
    Ok(())
}

fn frf(system: &System, frequency_hz: f64) -> Result<[Complex; 2]> {
    require(
        frequency_hz.is_finite() && frequency_hz > 0.0,
        "frequency must be finite and positive in Hz",
    )?;
    let omega = 2.0 * std::f64::consts::PI * frequency_hz;
    let k = system.stiffness_n_m;
    let m = system.mass_kg;
    let c = system.damping_ns_m;

    let d00 = Complex::new(k[0][0] - omega * omega * m[0], omega * c[0][0]);
    let d01 = Complex::new(k[0][1], omega * c[0][1]);
    let d10 = Complex::new(k[1][0], omega * c[1][0]);
    let d11 = Complex::new(k[1][1] - omega * omega * m[1], omega * c[1][1]);

    let determinant = d00.mul(d11).sub(d01.mul(d10));
    let numerator_0 = d11.mul(system.force_n[0]).sub(d01.mul(system.force_n[1]));
    let numerator_1 = d00.mul(system.force_n[1]).sub(d10.mul(system.force_n[0]));
    let response = [numerator_0.div(determinant)?, numerator_1.div(determinant)?];
    require(
        response.iter().all(|value| value.is_finite()),
        "FRF result contains a non-finite value",
    )?;
    Ok(response)
}

fn sensor_observation(sensor: &Sensor, response: [Complex; 2]) -> Result<Option<Complex>> {
    if !sensor.available || sensor.fault == "DROPOUT" {
        return Ok(None);
    }
    if sensor.fault == "STUCK_AT" {
        return sensor
            .stuck_value
            .filter(|value| value.is_finite())
            .map(Some)
            .ok_or_else(|| {
                OracleError(format!(
                    "{}: STUCK_AT sensor has no finite stuck value",
                    sensor.sensor_id
                ))
            });
    }

    let phase = sensor.phase_deg.to_radians();
    let gain = Complex::new(sensor.gain * phase.cos(), sensor.gain * phase.sin());
    Ok(Some(
        gain.mul(response[sensor.dof as usize]).add(sensor.bias),
    ))
}

fn observation_rank(sensors: &[Sensor]) -> usize {
    let dofs = sensors
        .iter()
        .filter(|sensor| {
            sensor.available && sensor.fault != "DROPOUT" && sensor.fault != "STUCK_AT"
        })
        .map(|sensor| sensor.dof)
        .collect::<BTreeSet<_>>();
    dofs.len()
}

fn compare_complex(
    actual: Complex,
    expected: Complex,
    tolerance: &Tolerance,
    label: &str,
) -> Result<()> {
    let delta = actual.sub(expected).magnitude();
    let threshold = tolerance.absolute + tolerance.relative * expected.magnitude();
    require(
        delta <= threshold,
        format!("{label}: complex error {delta:e} exceeds tolerance {threshold:e}"),
    )?;
    let actual_phase = actual.phase();
    let expected_phase = expected.phase();
    let phase_error = (actual_phase - expected_phase + std::f64::consts::PI)
        .rem_euclid(2.0 * std::f64::consts::PI)
        - std::f64::consts::PI;
    require(
        phase_error.abs() <= tolerance.phase_rad,
        format!(
            "{label}: phase error {} rad exceeds {} rad",
            phase_error.abs(),
            tolerance.phase_rad
        ),
    )?;
    Ok(())
}

fn validate_sensor(sensor: &Sensor, case_id: &str) -> Result<()> {
    require(
        !sensor.sensor_id.trim().is_empty(),
        format!("{case_id}: empty sensor ID"),
    )?;
    require(
        sensor.dof <= 1,
        format!("{case_id}: sensor DOF must be 0 or 1"),
    )?;
    require(
        sensor.gain.is_finite() && sensor.gain >= 0.0,
        format!("{case_id}: sensor gain must be finite and non-negative"),
    )?;
    require(
        sensor.phase_deg.is_finite(),
        format!("{case_id}: non-finite sensor phase"),
    )?;
    require(
        sensor.bias.is_finite(),
        format!("{case_id}: non-finite sensor bias"),
    )?;
    require(
        matches!(
            sensor.fault.as_str(),
            "NONE" | "GAIN_PHASE" | "BIAS" | "DROPOUT" | "STUCK_AT"
        ),
        format!("{case_id}: unsupported fault mode {}", sensor.fault),
    )?;
    if sensor.fault == "STUCK_AT" {
        require(
            sensor.stuck_value.is_some_and(Complex::is_finite),
            format!("{case_id}: STUCK_AT requires finite stuck_value"),
        )?;
    }
    Ok(())
}

fn validate_case_with_digest(
    case: &Case,
    tolerance: &Tolerance,
    corpus_sha256: &str,
) -> Result<ComputedCase> {
    require(!case.case_id.trim().is_empty(), "case ID is empty")?;
    require(
        matches!(case.state.as_str(), "HH" | "HF" | "DH" | "DF"),
        format!("{}: state must be HH/HF/DH/DF", case.case_id),
    )?;
    require(
        !case.frequencies_hz.is_empty(),
        format!("{}: frequency grid is empty", case.case_id),
    )?;
    validate_system(&case.nominal_model, &format!("{} nominal", case.case_id))?;
    validate_system(&case.physical_model, &format!("{} physical", case.case_id))?;

    let mut ids = BTreeSet::new();
    for sensor in &case.sensors {
        validate_sensor(sensor, &case.case_id)?;
        require(
            ids.insert(sensor.sensor_id.as_str()),
            format!("{}: duplicate sensor ID {}", case.case_id, sensor.sensor_id),
        )?;
    }
    require(
        case.expected_observation_rank <= 2,
        format!(
            "{}: observation rank exceeds system dimension",
            case.case_id
        ),
    )?;
    require(
        observation_rank(&case.sensors) == case.expected_observation_rank,
        format!(
            "{}: observation rank does not match frozen answer",
            case.case_id
        ),
    )?;
    require(
        case.expected_reference.len() == case.frequencies_hz.len(),
        format!("{}: expected reference length mismatch", case.case_id),
    )?;

    let mut computed_frequencies = Vec::with_capacity(case.frequencies_hz.len());
    for (index, frequency_hz) in case.frequencies_hz.iter().enumerate() {
        let expected = &case.expected_reference[index];
        require(
            (expected.frequency_hz - *frequency_hz).abs() <= 1.0e-12,
            format!(
                "{}: frequency schedule mismatch at index {index}",
                case.case_id
            ),
        )?;
        let nominal = frf(&case.nominal_model, *frequency_hz)?;
        let physical = frf(&case.physical_model, *frequency_hz)?;
        for dof in 0..2 {
            compare_complex(
                nominal[dof],
                expected.nominal_response_m_per_n[dof],
                tolerance,
                &format!("{} nominal dof {dof} at {frequency_hz} Hz", case.case_id),
            )?;
            compare_complex(
                physical[dof],
                expected.physical_response_m_per_n[dof],
                tolerance,
                &format!("{} physical dof {dof} at {frequency_hz} Hz", case.case_id),
            )?;
        }

        require(
            expected.observed_channels.len() == case.sensors.len(),
            format!("{}: observed channel count mismatch", case.case_id),
        )?;
        let mut observed = Vec::with_capacity(case.sensors.len());
        for (sensor_index, sensor) in case.sensors.iter().enumerate() {
            let expected_channel = &expected.observed_channels[sensor_index];
            require(
                expected_channel.sensor_id == sensor.sensor_id,
                format!("{}: sensor order/ID mismatch", case.case_id),
            )?;
            let value = sensor_observation(sensor, physical)?;
            match (value, expected_channel.value) {
                (Some(actual), Some(expected_value)) => compare_complex(
                    actual,
                    expected_value,
                    tolerance,
                    &format!(
                        "{} {} at {} Hz",
                        case.case_id, sensor.sensor_id, frequency_hz
                    ),
                )?,
                (None, None) => {}
                _ => {
                    return fail(format!(
                        "{}: missing channel semantics mismatch for {}",
                        case.case_id, sensor.sensor_id
                    ));
                }
            }
            observed.push(ObservedChannel {
                sensor_id: sensor.sensor_id.clone(),
                value,
            });
        }

        computed_frequencies.push(ComputedFrequency {
            frequency_hz: *frequency_hz,
            omega_rad_s: 2.0 * std::f64::consts::PI * frequency_hz,
            nominal_response_m_per_n: nominal,
            physical_response_m_per_n: physical,
            observed_channels: observed,
            nominal_vs_physical_complex_residual_m_per_n: [
                physical[0].sub(nominal[0]),
                physical[1].sub(nominal[1]),
            ],
        });
    }

    let scenario_identity = format!("{corpus_sha256}:{}", case.case_id);
    Ok(ComputedCase {
        case_id: case.case_id.clone(),
        state: case.state.clone(),
        scenario_sha256: sha256_hex(scenario_identity.as_bytes()),
        expected_disposition: case.expected_disposition.clone(),
        observation_rank: case.expected_observation_rank,
        frequencies: computed_frequencies,
    })
}

fn validate_case(case: &Case, tolerance: &Tolerance) -> Result<ComputedCase> {
    validate_case_with_digest(case, tolerance, "synthetic-unit-test")
}

fn evaluate(corpus_bytes: &[u8]) -> Result<OracleResult> {
    let corpus: Corpus = serde_json::from_slice(corpus_bytes)
        .map_err(|error| OracleError(format!("corpus JSON error: {error}")))?;
    require(
        corpus.schema_version == "aero-virtual-sensing-frf-corpus/0.1.0",
        "unsupported corpus schema version",
    )?;
    require(!corpus.corpus_id.trim().is_empty(), "corpus ID is empty")?;
    require(
        corpus.evidence_class == "synthetic_analytical_known_answer",
        "corpus evidence class changed",
    )?;
    require(
        corpus.claim_ceiling == "analytical_2dof_frequency_response_and_sensor_mapping_only",
        "corpus claim ceiling changed",
    )?;
    require(
        !corpus.physical_pod_claim,
        "synthetic FRF corpus cannot claim physical POD",
    )?;
    require(
        !corpus.aircraft_validation_claim,
        "synthetic FRF corpus cannot claim aircraft validation",
    )?;
    require(
        corpus.frequency_convention == "f_hz; omega_rad_s = 2*pi*f_hz",
        "frequency convention changed",
    )?;
    require(
        corpus.response_convention
            == "x = inverse(K - omega^2*M + i*omega*C) * F; e^(+i*omega*t) phasor",
        "response/phasor convention changed",
    )?;
    require(
        corpus.unit_system.mass == "kg"
            && corpus.unit_system.stiffness == "N/m"
            && corpus.unit_system.damping == "N*s/m"
            && corpus.unit_system.force == "N"
            && corpus.unit_system.response == "m/N",
        "unit system convention changed",
    )?;
    require(
        corpus.tolerance.absolute.is_finite()
            && corpus.tolerance.relative.is_finite()
            && corpus.tolerance.phase_rad.is_finite()
            && corpus.tolerance.absolute >= 0.0
            && corpus.tolerance.relative >= 0.0
            && corpus.tolerance.phase_rad >= 0.0,
        "invalid tolerance contract",
    )?;
    require(!corpus.cases.is_empty(), "corpus contains no cases")?;

    let corpus_sha256 = sha256_hex(corpus_bytes);
    let mut ids = BTreeSet::new();
    let mut computed_cases = Vec::with_capacity(corpus.cases.len());
    let mut frequency_count = None;
    for case in &corpus.cases {
        require(ids.insert(case.case_id.as_str()), "duplicate case ID")?;
        let computed = validate_case_with_digest(case, &corpus.tolerance, &corpus_sha256)?;
        if let Some(expected_count) = frequency_count {
            require(
                computed.frequencies.len() == expected_count,
                "frequency count differs across frozen cases",
            )?;
        } else {
            frequency_count = Some(computed.frequencies.len());
        }
        computed_cases.push(computed);
    }

    Ok(OracleResult {
        schema_version: "aero-virtual-sensing-frf-result/0.1.0",
        corpus_id: corpus.corpus_id,
        corpus_sha256,
        case_count: computed_cases.len(),
        frequency_count_per_case: frequency_count.unwrap_or(0),
        computed_cases,
        physical_pod: "NOT_ASSESSED",
        aircraft_validation: "NOT_ASSESSED",
        structural_clearance: "NOT_AUTHORIZED",
        claim_ceiling: "analytical_2dof_frequency_response_and_sensor_mapping_only",
    })
}

fn main() {
    match evaluate(CORPUS) {
        Ok(result) => match serde_json::to_string(&result) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("frf_oracle_serialize_error={error}");
                std::process::exit(2);
            }
        },
        Err(error) => {
            eprintln!("frf_oracle_error={error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        let corpus: Corpus = serde_json::from_slice(CORPUS).expect("corpus JSON");
        evaluate(CORPUS).expect("frozen FRF corpus must validate");
        corpus
    }

    #[test]
    fn frozen_complex_frf_known_answers_match() {
        let result = evaluate(CORPUS).unwrap();
        assert_eq!(result.case_count, 9);
        assert_eq!(result.frequency_count_per_case, 9);
        assert_eq!(result.physical_pod, "NOT_ASSESSED");
        assert_eq!(result.aircraft_validation, "NOT_ASSESSED");
        assert_eq!(result.structural_clearance, "NOT_AUTHORIZED");
    }

    #[test]
    fn frequencies_are_hz_and_angular_frequency_is_2pi_f() {
        let result = evaluate(CORPUS).unwrap();
        let item = &result.computed_cases[0].frequencies[0];
        assert!((item.omega_rad_s - 2.0 * std::f64::consts::PI * 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn dropout_is_missing_not_zero_response() {
        let result = evaluate(CORPUS).unwrap();
        let dropout = result
            .computed_cases
            .iter()
            .find(|case| case.case_id == "FRF-HF-DROPOUT")
            .unwrap();
        assert!(dropout.frequencies[0].observed_channels[1].value.is_none());
    }

    #[test]
    fn duplicate_sensor_ids_are_rejected() {
        let mut corpus = corpus();
        corpus.cases[0].sensors[1].sensor_id = corpus.cases[0].sensors[0].sensor_id.clone();
        assert!(validate_case(&corpus.cases[0], &corpus.tolerance).is_err());
    }

    #[test]
    fn invalid_sensor_dof_is_rejected() {
        let mut corpus = corpus();
        corpus.cases[0].sensors[0].dof = 2;
        assert!(validate_case(&corpus.cases[0], &corpus.tolerance).is_err());
    }

    #[test]
    fn non_positive_mass_is_rejected() {
        let mut corpus = corpus();
        corpus.cases[0].physical_model.mass_kg[0] = 0.0;
        assert!(validate_case(&corpus.cases[0], &corpus.tolerance).is_err());
    }

    #[test]
    fn non_positive_definite_stiffness_is_rejected() {
        let mut corpus = corpus();
        corpus.cases[0].physical_model.stiffness_n_m = [[1.0, 2.0], [2.0, 1.0]];
        assert!(validate_case(&corpus.cases[0], &corpus.tolerance).is_err());
    }

    #[test]
    fn missing_channel_cannot_be_replaced_by_zero() {
        let mut corpus = corpus();
        corpus.cases[0].expected_reference[0].observed_channels[1].value = Some(Complex::default());
        assert!(validate_case(&corpus.cases[0], &corpus.tolerance).is_err());
    }

    #[test]
    fn phase_sign_change_is_detected() {
        let mut corpus = corpus();
        corpus.cases[1].sensors[1].phase_deg = -corpus.cases[1].sensors[1].phase_deg;
        assert!(validate_case(&corpus.cases[1], &corpus.tolerance).is_err());
    }

    #[test]
    fn singular_dynamic_stiffness_is_rejected() {
        let system = System {
            mass_kg: [1.0, 1.0],
            stiffness_n_m: [[1.0, 0.0], [0.0, 4.0]],
            damping_ns_m: [[0.0, 0.0], [0.0, 0.0]],
            force_n: [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
        };
        assert!(frf(&system, 1.0 / (2.0 * std::f64::consts::PI)).is_err());
    }

    #[test]
    fn out_of_order_frequency_schedule_does_not_match_frozen_answers() {
        let mut corpus = corpus();
        corpus.cases[0].frequencies_hz.swap(0, 1);
        assert!(validate_case(&corpus.cases[0], &corpus.tolerance).is_err());
    }
}
