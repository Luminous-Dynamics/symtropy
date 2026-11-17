//! Deterministic synthetic comparison of tactile calibration-contact policies.
//!
//! This is a deliberately small nonlinear inverse-response experiment. It
//! compares the same target article, candidate pool and held-out domain under
//! equal contact budgets. It is not a physical sensor model or a measurement
//! uncertainty calculation.
//!
//! The active policies may use candidate stimulus coordinates and training
//! observations. They never receive held-out evaluator truth.

#![forbid(unsafe_code)]

pub mod adversarial;
pub mod adaptive_readout;
pub mod coupling_campaign;
pub mod coupling_identification;
pub mod field_campaign;
pub mod field_reconstruction;
pub mod model_forms;
pub mod observability_design;
pub mod operating_envelope;
pub mod self_test;
pub mod temporal_integrity;
pub mod wrench_consistency;

const PARAMS: usize = 8;
const STRATA: usize = 4;
const CONTACTS_PER_STRATUM: usize = 4;
const TARGET_BUDGET: usize = STRATA * CONTACTS_PER_STRATUM;
const RIDGE: f64 = 1.0e-8;
/// Synthetic-only primary/check reference agreement threshold; not a hardware tolerance.
pub const SYNTHETIC_REFERENCE_AGREEMENT_TOLERANCE_N: f64 = 0.02;
/// Synthetic-only raw-response resolution used by the observability fixture.
pub const SYNTHETIC_RESPONSE_RESOLUTION: f64 = 1.0e-12;
/// Required synthetic force-setpoint manifest. Physical operating ranges are
/// supplied by the measurement/phantom owner, never inferred from this array.
pub const SYNTHETIC_REQUIRED_FORCE_SETPOINTS_N: [f64; 6] = [0.0, 2.0, 4.0, 6.0, 8.0, 10.0];
/// Declared synthetic strata: 3 positions × 2 temperatures × 2 loading phases.
pub const SYNTHETIC_EXPECTED_CONDITION_COUNT: usize = 12;

type ConditionKey = (u64, u64, u64);
type ConditionGroup<'a> = (ConditionKey, Vec<&'a Candidate>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResponseObservabilityReport {
    pub accepted: bool,
    pub condition_count: usize,
    pub expected_force_setpoint_count: usize,
    pub missing_condition_count: usize,
    pub incomplete_condition_count: usize,
    pub checked_transition_count: usize,
    pub flatline_transition_count: usize,
    pub decreasing_transition_count: usize,
    pub affected_condition_count: usize,
    pub first_flatline_start_force_n: Option<f64>,
    pub maximum_observed_force_setpoint_n: f64,
    pub raw_response_resolution: f64,
}

/// Screen selector-visible raw-derived responses for non-increasing output
/// under an increasing normal-force setpoint, within fixed position,
/// temperature and loading-history conditions.
///
/// This synthetic guard is not a general observability proof and must not be
/// presented as a calibrated force-range certificate.
fn response_observability_check(
    candidates: &[Candidate],
    required_force_setpoints_n: &[f64],
    raw_response_resolution: f64,
) -> Result<ResponseObservabilityReport, String> {
    if candidates.is_empty()
        || required_force_setpoints_n.len() < 2
        || required_force_setpoints_n
            .iter()
            .any(|value| !value.is_finite())
        || required_force_setpoints_n
            .windows(2)
            .any(|pair| pair[1] <= pair[0])
        || !raw_response_resolution.is_finite()
        || raw_response_resolution < 0.0
    {
        return Err("observability check requires candidates, an increasing finite force manifest and a finite nonnegative response resolution".to_owned());
    }

    let mut conditions: Vec<ConditionGroup<'_>> = Vec::new();
    let mut maximum_force = f64::NEG_INFINITY;
    for candidate in candidates {
        if !candidate.force_setpoint_n.is_finite()
            || !candidate.position.is_finite()
            || !candidate.temperature.is_finite()
            || !candidate.phase.is_finite()
            || !candidate.features[1].is_finite()
        {
            return Err(
                "observability candidate contains non-finite stimulus or raw response".to_owned(),
            );
        }
        maximum_force = maximum_force.max(candidate.force_setpoint_n);
        let key = (
            candidate.position.to_bits(),
            candidate.temperature.to_bits(),
            candidate.phase.to_bits(),
        );
        if let Some((_, group)) = conditions.iter_mut().find(|(existing, _)| *existing == key) {
            group.push(candidate);
        } else {
            conditions.push((key, vec![candidate]));
        }
    }

    let mut checked_transition_count = 0;
    let mut flatline_transition_count = 0;
    let mut decreasing_transition_count = 0;
    let missing_condition_count =
        SYNTHETIC_EXPECTED_CONDITION_COUNT.saturating_sub(conditions.len());
    let mut incomplete_condition_count = 0;
    let mut affected_condition_count = 0;
    let mut first_flatline_start_force_n: Option<f64> = None;

    for (_, group) in &mut conditions {
        group.sort_by(|left, right| left.force_setpoint_n.total_cmp(&right.force_setpoint_n));
        let observed_setpoints: Vec<_> = group
            .iter()
            .map(|candidate| candidate.force_setpoint_n)
            .collect();
        let condition_incomplete = required_force_setpoints_n.iter().any(|required| {
            !observed_setpoints
                .iter()
                .any(|observed| observed.total_cmp(required).is_eq())
        });
        if condition_incomplete {
            incomplete_condition_count += 1;
        }
        let mut condition_affected = false;
        for pair in group.windows(2) {
            let lower = pair[0];
            let higher = pair[1];
            if higher.force_setpoint_n <= lower.force_setpoint_n {
                return Err(
                    "observability condition contains duplicate or unordered force setpoints"
                        .to_owned(),
                );
            }
            checked_transition_count += 1;
            let response_delta = higher.features[1] - lower.features[1];
            if response_delta <= raw_response_resolution {
                flatline_transition_count += 1;
                condition_affected = true;
                first_flatline_start_force_n = Some(
                    first_flatline_start_force_n.map_or(lower.force_setpoint_n, |current| {
                        current.min(lower.force_setpoint_n)
                    }),
                );
            }
            if response_delta < -raw_response_resolution {
                decreasing_transition_count += 1;
            }
        }
        if condition_affected {
            affected_condition_count += 1;
        }
    }

    Ok(ResponseObservabilityReport {
        accepted: flatline_transition_count == 0
            && incomplete_condition_count == 0
            && missing_condition_count == 0,
        condition_count: conditions.len(),
        expected_force_setpoint_count: required_force_setpoints_n.len(),
        missing_condition_count,
        incomplete_condition_count,
        checked_transition_count,
        flatline_transition_count,
        decreasing_transition_count,
        affected_condition_count,
        first_flatline_start_force_n,
        maximum_observed_force_setpoint_n: maximum_force,
        raw_response_resolution,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Candidate {
    id: usize,
    /// Commanded stimulus setpoint, not the reference-measured force.
    force_setpoint_n: f64,
    position: f64,
    temperature: f64,
    phase: f64,
    stratum: usize,
    cost_ms: u64,
    /// Raw-derived feature vector available to the selector.
    features: [f64; PARAMS],
}

/// Training labels are deliberately stored separately from selector-visible
/// stimuli. Policy functions receive only `&[Candidate]`.
#[derive(Clone, Debug, PartialEq)]
struct SyntheticDomain {
    candidates: Vec<Candidate>,
    /// Primary reference channel used for fitting.
    reference_force_observations_n: Vec<f64>,
    /// Separate synthetic check channel; never used to fit this model.
    independent_check_reference_force_observations_n: Vec<f64>,
    /// Synthetic fixture attestation for distinct reference calibration ancestry.
    /// Real use requires separately validated provenance, not this boolean.
    independent_reference_ancestry_attested: bool,
}

/// Pairwise reference-channel agreement is only a fault screen. Agreement does
/// not establish traceability or prove that the channels have independent
/// calibration ancestry / common-cause failure modes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferenceIntegrityReport {
    pub accepted: bool,
    pub sample_count: usize,
    pub max_abs_disagreement_n: f64,
    pub rms_disagreement_n: f64,
    pub tolerance_n: f64,
    pub independent_reference_ancestry_attested: bool,
    /// Always false: pairwise numerical agreement is not proof of independence.
    pub agreement_proves_independence: bool,
}

pub fn reference_integrity_check(
    primary_reference_n: &[f64],
    independent_check_reference_n: &[f64],
    tolerance_n: f64,
    independent_reference_ancestry_attested: bool,
) -> Result<ReferenceIntegrityReport, String> {
    if primary_reference_n.is_empty()
        || primary_reference_n.len() != independent_check_reference_n.len()
        || !tolerance_n.is_finite()
        || tolerance_n < 0.0
    {
        return Err(
            "reference channels require equal nonempty samples and a finite nonnegative tolerance"
                .to_owned(),
        );
    }
    let mut squared = 0.0;
    let mut maximum = 0.0_f64;
    for (primary, check) in primary_reference_n
        .iter()
        .zip(independent_check_reference_n)
    {
        if !primary.is_finite() || !check.is_finite() {
            return Err("reference channel contains non-finite sample".to_owned());
        }
        let difference = (primary - check).abs();
        maximum = maximum.max(difference);
        squared += difference * difference;
    }
    let count = primary_reference_n.len() as f64;
    Ok(ReferenceIntegrityReport {
        accepted: maximum <= tolerance_n && independent_reference_ancestry_attested,
        sample_count: primary_reference_n.len(),
        max_abs_disagreement_n: maximum,
        rms_disagreement_n: (squared / count).sqrt(),
        tolerance_n,
        independent_reference_ancestry_attested,
        // Pairwise numeric agreement can never prove distinct calibration
        // ancestry or rule out a shared systematic cause.
        agreement_proves_independence: false,
    })
}

/// Held-out truth is not attached to selector-visible candidate records.
#[derive(Clone, Debug, PartialEq)]
struct HeldoutDomain {
    candidates: Vec<Candidate>,
    evaluator_truth_force_n: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SensorProfile {
    raw_coefficients: [f64; PARAMS],
}

const TARGET_PROFILE: SensorProfile = SensorProfile {
    raw_coefficients: [0.12, 0.93, 0.004, 0.08, 0.025, 0.035, 0.020, 0.009],
};

const SOURCE_PROFILE: SensorProfile = SensorProfile {
    raw_coefficients: [0.19, 0.82, 0.006, 0.10, 0.020, 0.052, 0.030, 0.013],
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct SyntheticStimulus {
    profile: SensorProfile,
    force_n: f64,
    position: f64,
    temperature: f64,
    phase: f64,
    seed: u64,
    noise_scale: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct Fit {
    coefficients: [f64; PARAMS],
    inverse_information: [[f64; PARAMS]; PARAMS],
    residual_variance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolicyResult {
    pub policy_id: &'static str,
    pub source_reference_contacts: usize,
    pub target_reference_contacts: usize,
    pub source_reference_cost_ms: u64,
    pub target_reference_cost_ms: u64,
    pub total_estimated_fixture_cost_ms: u64,
    pub rmse_n: f64,
    pub p95_absolute_error_n: f64,
    pub worst_absolute_error_n: f64,
    pub interval_coverage_95: f64,
    pub mean_interval_width_95_n: f64,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkReport {
    pub seed: u64,
    pub response_observability: ResponseObservabilityReport,
    pub source_candidate_pool_size: usize,
    pub target_candidate_pool_size: usize,
    pub heldout_case_count: usize,
    pub target_contact_budget: usize,
    pub results: Vec<PolicyResult>,
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

fn unit_interval(state: &mut u64) -> f64 {
    let mantissa = xorshift64(state) >> 11;
    mantissa as f64 / ((1_u64 << 53) as f64)
}

fn deterministic_noise(seed: u64, index: usize, scale: f64) -> f64 {
    let mut state = seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    if state == 0 {
        state = 1;
    }
    (unit_interval(&mut state) - 0.5) * 2.0 * scale
}

fn polynomial_response(
    profile: SensorProfile,
    force_n: f64,
    position: f64,
    temperature: f64,
    phase: f64,
    noise: f64,
) -> f64 {
    let c = profile.raw_coefficients;
    c[0] + c[1] * force_n
        + c[2] * force_n * force_n
        + c[3] * position
        + c[4] * temperature
        + c[5] * phase
        + c[6] * force_n * position
        + c[7] * force_n * temperature
        + noise
}

fn design_features(raw: f64, position: f64, temperature: f64, phase: f64) -> [f64; PARAMS] {
    [
        1.0,
        raw,
        raw * raw,
        position,
        temperature,
        phase,
        raw * position,
        raw * temperature,
    ]
}

fn make_candidate(id: usize, stimulus: SyntheticStimulus) -> (Candidate, f64) {
    let SyntheticStimulus {
        profile,
        force_n,
        position,
        temperature,
        phase,
        seed,
        noise_scale,
    } = stimulus;
    let stratum = usize::from(temperature > 0.0) * 2 + usize::from(phase > 0.0);
    let raw = polynomial_response(
        profile,
        force_n,
        position,
        temperature,
        phase,
        deterministic_noise(seed, id, noise_scale),
    );
    let cost_ms = 500
        + (force_n * 5.0) as u64
        + if phase > 0.0 { 100 } else { 0 }
        + if position.abs() > 0.9 { 80 } else { 0 };

    let candidate = Candidate {
        id,
        force_setpoint_n: force_n,
        position,
        temperature,
        phase,
        stratum,
        cost_ms,
        features: design_features(raw, position, temperature, phase),
    };
    let reference_observation_n =
        force_n + deterministic_noise(seed ^ 0xD1B5_4A32_D192_ED03, id, 0.003);
    (candidate, reference_observation_n)
}

fn target_candidate_pool(seed: u64) -> SyntheticDomain {
    let mut candidates = Vec::with_capacity(72);
    let mut reference_force_observations_n = Vec::with_capacity(72);
    let mut independent_check_reference_force_observations_n = Vec::with_capacity(72);
    let mut id = 0;
    for force_index in 0..6 {
        let force = force_index as f64 * 2.0;
        for position in [-1.0, 0.0, 1.0] {
            for temperature in [-1.0, 1.0] {
                for phase in [-1.0, 1.0] {
                    let (candidate, reference_observation_n) = make_candidate(
                        id,
                        SyntheticStimulus {
                            profile: TARGET_PROFILE,
                            force_n: force,
                            position,
                            temperature,
                            phase,
                            seed: seed ^ 0xA5A5_5A5A_1234_5678,
                            noise_scale: 0.006,
                        },
                    );
                    candidates.push(candidate);
                    reference_force_observations_n.push(reference_observation_n);
                    independent_check_reference_force_observations_n
                        .push(force + deterministic_noise(seed ^ 0xA076_1D64_78BD_642F, id, 0.003));
                    id += 1;
                }
            }
        }
    }
    SyntheticDomain {
        candidates,
        reference_force_observations_n,
        independent_check_reference_force_observations_n,
        independent_reference_ancestry_attested: true,
    }
}

fn source_candidate_pool(seed: u64) -> SyntheticDomain {
    let mut candidates = Vec::with_capacity(72);
    let mut reference_force_observations_n = Vec::with_capacity(72);
    let mut independent_check_reference_force_observations_n = Vec::with_capacity(72);
    let mut id = 0;
    for force_index in 0..6 {
        let force = force_index as f64 * 2.0;
        for position in [-1.0, 0.0, 1.0] {
            for temperature in [-1.0, 1.0] {
                for phase in [-1.0, 1.0] {
                    let (candidate, reference_observation_n) = make_candidate(
                        id,
                        SyntheticStimulus {
                            profile: SOURCE_PROFILE,
                            force_n: force,
                            position,
                            temperature,
                            phase,
                            seed: seed ^ 0x5A5A_A5A5_8765_4321,
                            noise_scale: 0.006,
                        },
                    );
                    candidates.push(candidate);
                    reference_force_observations_n.push(reference_observation_n);
                    independent_check_reference_force_observations_n
                        .push(force + deterministic_noise(seed ^ 0xE703_7ED1_A0B4_28DB, id, 0.003));
                    id += 1;
                }
            }
        }
    }
    SyntheticDomain {
        candidates,
        reference_force_observations_n,
        independent_check_reference_force_observations_n,
        independent_reference_ancestry_attested: true,
    }
}

fn heldout_target_pool(seed: u64) -> HeldoutDomain {
    let mut candidates = Vec::with_capacity(60);
    let mut evaluator_truth_force_n = Vec::with_capacity(60);
    let mut id = 0;
    for force in [1.0, 3.0, 5.0, 7.0, 9.0] {
        for position in [-0.7, 0.25, 0.8] {
            for temperature in [-0.6, 0.6] {
                for phase in [-0.5, 0.5] {
                    let (candidate, _heldout_reference_observation_n) = make_candidate(
                        id,
                        SyntheticStimulus {
                            profile: TARGET_PROFILE,
                            force_n: force,
                            position,
                            temperature,
                            phase,
                            seed: seed ^ 0x1357_9BDF_2468_ACE0,
                            noise_scale: 0.006,
                        },
                    );
                    candidates.push(candidate);
                    evaluator_truth_force_n.push(force);
                    id += 1;
                }
            }
        }
    }
    HeldoutDomain {
        candidates,
        evaluator_truth_force_n,
    }
}

fn invert_matrix(matrix: [[f64; PARAMS]; PARAMS]) -> Result<[[f64; PARAMS]; PARAMS], String> {
    let mut augmented = [[0.0_f64; PARAMS * 2]; PARAMS];
    for (row_index, row_values) in augmented.iter_mut().enumerate() {
        for (column, value) in row_values.iter_mut().take(PARAMS).enumerate() {
            *value = matrix[row_index][column];
        }
        row_values[PARAMS + row_index] = 1.0;
    }

    for column in 0..PARAMS {
        let pivot = (column..PARAMS)
            .max_by(|left, right| {
                augmented[*left][column]
                    .abs()
                    .total_cmp(&augmented[*right][column].abs())
            })
            .ok_or_else(|| "matrix pivot selection failed".to_owned())?;

        if augmented[pivot][column].abs() <= 1.0e-14 {
            return Err("singular design matrix".to_owned());
        }

        if pivot != column {
            augmented.swap(pivot, column);
        }

        let divisor = augmented[column][column];
        for value in &mut augmented[column] {
            *value /= divisor;
        }

        let pivot_row = augmented[column];
        for (row_index, row_values) in augmented.iter_mut().enumerate() {
            if row_index == column {
                continue;
            }
            let factor = row_values[column];
            for (value, pivot_value) in row_values.iter_mut().zip(pivot_row.iter()) {
                *value -= factor * pivot_value;
            }
        }
    }

    let mut inverse = [[0.0; PARAMS]; PARAMS];
    for row in 0..PARAMS {
        for column in 0..PARAMS {
            inverse[row][column] = augmented[row][PARAMS + column];
        }
    }
    Ok(inverse)
}

fn quadratic_form(vector: &[f64; PARAMS], matrix: &[[f64; PARAMS]; PARAMS]) -> f64 {
    let mut value = 0.0;
    for row in 0..PARAMS {
        for column in 0..PARAMS {
            value += vector[row] * matrix[row][column] * vector[column];
        }
    }
    value.max(0.0)
}

fn fit_qualified_domain(
    domain: &SyntheticDomain,
    selected: &[usize],
    prior: Option<&[f64; PARAMS]>,
    prior_strength: f64,
) -> Result<Fit, String> {
    let observability = response_observability_check(
        &domain.candidates,
        &SYNTHETIC_REQUIRED_FORCE_SETPOINTS_N,
        SYNTHETIC_RESPONSE_RESOLUTION,
    )?;
    if !observability.accepted {
        return Err(format!(
            "response observability gate failed: {} flatline transitions across {} conditions; first plateau starts at {:?} N",
            observability.flatline_transition_count,
            observability.affected_condition_count,
            observability.first_flatline_start_force_n,
        ));
    }
    let integrity = reference_integrity_check(
        &domain.reference_force_observations_n,
        &domain.independent_check_reference_force_observations_n,
        SYNTHETIC_REFERENCE_AGREEMENT_TOLERANCE_N,
        domain.independent_reference_ancestry_attested,
    )?;
    if !integrity.accepted {
        return Err(format!(
            "reference integrity gate failed: max disagreement {:.6} N, tolerance {:.6} N, ancestry attested {}",
            integrity.max_abs_disagreement_n,
            integrity.tolerance_n,
            integrity.independent_reference_ancestry_attested,
        ));
    }
    fit_model(
        &domain.candidates,
        &domain.reference_force_observations_n,
        selected,
        prior,
        prior_strength,
    )
}

fn fit_model(
    candidates: &[Candidate],
    reference_force_observations_n: &[f64],
    selected: &[usize],
    prior: Option<&[f64; PARAMS]>,
    prior_strength: f64,
) -> Result<Fit, String> {
    if candidates.len() != reference_force_observations_n.len() {
        return Err("candidate and reference-observation cardinalities differ".to_owned());
    }
    if selected.len() < PARAMS {
        return Err("at least eight target/source contacts are required".to_owned());
    }

    let mut information = [[0.0_f64; PARAMS]; PARAMS];
    let mut rhs = [0.0_f64; PARAMS];

    for (index, information_row) in information.iter_mut().enumerate() {
        information_row[index] = prior_strength + RIDGE;
        if let Some(prior) = prior {
            rhs[index] = prior_strength * prior[index];
        }
    }

    for selected_index in selected {
        let candidate = candidates
            .get(*selected_index)
            .ok_or_else(|| "selected candidate index is out of bounds".to_owned())?;
        let reference_force_n = *reference_force_observations_n
            .get(*selected_index)
            .ok_or_else(|| "reference observation index is out of bounds".to_owned())?;
        for (row, (rhs_value, information_row)) in
            rhs.iter_mut().zip(information.iter_mut()).enumerate()
        {
            let row_feature = candidate.features[row];
            *rhs_value += row_feature * reference_force_n;
            for (column, value) in information_row.iter_mut().enumerate() {
                *value += row_feature * candidate.features[column];
            }
        }
    }

    let inverse_information = invert_matrix(information)?;
    let mut coefficients = [0.0; PARAMS];
    for row in 0..PARAMS {
        for column in 0..PARAMS {
            coefficients[row] += inverse_information[row][column] * rhs[column];
        }
    }

    let mut squared_error = 0.0;
    for selected_index in selected {
        let candidate = &candidates[*selected_index];
        let prediction = dot(&coefficients, &candidate.features);
        let reference_force_n = reference_force_observations_n[*selected_index];
        let error = reference_force_n - prediction;
        squared_error += error * error;
    }
    let degrees_of_freedom = selected.len().saturating_sub(PARAMS).max(1) as f64;
    let residual_variance = squared_error / degrees_of_freedom;

    Ok(Fit {
        coefficients,
        inverse_information,
        residual_variance: residual_variance.max(1.0e-12),
    })
}

fn dot(left: &[f64; PARAMS], right: &[f64; PARAMS]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn initial_stratum_candidate(candidates: &[Candidate], stratum: usize) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.stratum == stratum)
        .max_by(|(_, left), (_, right)| {
            let left_score =
                (left.force_setpoint_n / 10.0 - 0.5).abs() + left.position.abs() * 0.25;
            let right_score =
                (right.force_setpoint_n / 10.0 - 0.5).abs() + right.position.abs() * 0.25;
            left_score
                .total_cmp(&right_score)
                .then_with(|| right.id.cmp(&left.id))
        })
        .map(|(index, _)| index)
}

fn normalized_stimulus_distance(left: &Candidate, right: &Candidate) -> f64 {
    let force = (left.force_setpoint_n - right.force_setpoint_n) / 10.0;
    let position = (left.position - right.position) / 2.0;
    let temperature = (left.temperature - right.temperature) / 2.0;
    let phase = (left.phase - right.phase) / 2.0;
    force * force + position * position + temperature * temperature + phase * phase
}

fn select_uniform(candidates: &[Candidate]) -> Result<Vec<usize>, String> {
    let mut selected = Vec::with_capacity(TARGET_BUDGET);
    for stratum in 0..STRATA {
        let group: Vec<_> = candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| candidate.stratum == stratum)
            .map(|(index, _)| index)
            .collect();
        if group.len() != 18 {
            return Err("unexpected synthetic stratum cardinality".to_owned());
        }
        for local_index in [0, 5, 12, 17] {
            selected.push(group[local_index]);
        }
    }
    Ok(selected)
}

fn select_random_stratified(candidates: &[Candidate], seed: u64) -> Result<Vec<usize>, String> {
    let mut state = if seed == 0 { 1 } else { seed };
    let mut selected = Vec::with_capacity(TARGET_BUDGET);

    for stratum in 0..STRATA {
        let mut group: Vec<_> = candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| candidate.stratum == stratum)
            .map(|(index, _)| index)
            .collect();
        if group.len() != 18 {
            return Err("unexpected synthetic stratum cardinality".to_owned());
        }

        for position in (1..group.len()).rev() {
            let swap_with = (xorshift64(&mut state) as usize) % (position + 1);
            group.swap(position, swap_with);
        }
        selected.extend(group.into_iter().take(CONTACTS_PER_STRATUM));
    }

    Ok(selected)
}

fn select_maximin(candidates: &[Candidate]) -> Result<Vec<usize>, String> {
    let mut selected = Vec::with_capacity(TARGET_BUDGET);

    for stratum in 0..STRATA {
        let first = initial_stratum_candidate(candidates, stratum)
            .ok_or_else(|| "stratum has no initial maximin candidate".to_owned())?;
        let mut group_selected = vec![first];

        while group_selected.len() < CONTACTS_PER_STRATUM {
            let next = candidates
                .iter()
                .enumerate()
                .filter(|(index, candidate)| {
                    candidate.stratum == stratum && !group_selected.contains(index)
                })
                .max_by(|(_, left), (_, right)| {
                    let left_distance = group_selected
                        .iter()
                        .map(|selected_index| {
                            normalized_stimulus_distance(left, &candidates[*selected_index])
                        })
                        .fold(f64::INFINITY, f64::min);
                    let right_distance = group_selected
                        .iter()
                        .map(|selected_index| {
                            normalized_stimulus_distance(right, &candidates[*selected_index])
                        })
                        .fold(f64::INFINITY, f64::min);
                    left_distance
                        .total_cmp(&right_distance)
                        .then_with(|| right.id.cmp(&left.id))
                })
                .map(|(index, _)| index)
                .ok_or_else(|| "maximin selection exhausted a stratum".to_owned())?;
            group_selected.push(next);
        }

        selected.extend(group_selected);
    }

    Ok(selected)
}

fn select_d_optimal(candidates: &[Candidate]) -> Result<Vec<usize>, String> {
    let mut selected = Vec::with_capacity(TARGET_BUDGET);
    let mut selected_ids = Vec::with_capacity(TARGET_BUDGET);
    let mut counts = [0_usize; STRATA];

    // Start with one extreme reference contact in each mandatory stratum.
    for (stratum, count) in counts.iter_mut().enumerate() {
        let first = initial_stratum_candidate(candidates, stratum)
            .ok_or_else(|| "stratum has no initial D-optimal candidate".to_owned())?;
        selected.push(first);
        selected_ids.push(candidates[first].id);
        *count = 1;
    }

    while selected.len() < TARGET_BUDGET {
        let information = information_matrix(candidates, &selected, RIDGE)?;
        let inverse = invert_matrix(information)?;
        let least_covered = counts.iter().copied().min().unwrap_or(0);

        let next = candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                counts[candidate.stratum] == least_covered
                    && counts[candidate.stratum] < CONTACTS_PER_STRATUM
                    && !selected_ids.contains(&candidate.id)
            })
            .max_by(|(_, left), (_, right)| {
                let left_score = quadratic_form(&left.features, &inverse);
                let right_score = quadratic_form(&right.features, &inverse);
                left_score
                    .total_cmp(&right_score)
                    .then_with(|| right.id.cmp(&left.id))
            })
            .map(|(index, _)| index)
            .ok_or_else(|| "D-optimal selection exhausted eligible candidates".to_owned())?;

        selected.push(next);
        selected_ids.push(candidates[next].id);
        counts[candidates[next].stratum] += 1;
    }

    Ok(selected)
}

fn information_matrix(
    candidates: &[Candidate],
    selected: &[usize],
    ridge: f64,
) -> Result<[[f64; PARAMS]; PARAMS], String> {
    let mut information = [[0.0_f64; PARAMS]; PARAMS];
    for (index, information_row) in information.iter_mut().enumerate() {
        information_row[index] = ridge;
    }
    for selected_index in selected {
        let candidate = candidates
            .get(*selected_index)
            .ok_or_else(|| "selected candidate index is out of bounds".to_owned())?;
        for (row, information_row) in information.iter_mut().enumerate() {
            let row_feature = candidate.features[row];
            for (column, value) in information_row.iter_mut().enumerate() {
                *value += row_feature * candidate.features[column];
            }
        }
    }
    Ok(information)
}

fn source_prior(source_domain: &SyntheticDomain) -> Result<[f64; PARAMS], String> {
    let selected: Vec<_> = (0..source_domain.candidates.len()).collect();
    let fit = fit_qualified_domain(source_domain, &selected, None, 0.0)?;
    Ok(fit.coefficients)
}

fn evaluate(
    policy_id: &'static str,
    target_candidates: &[Candidate],
    target_selected: &[usize],
    source_reference_contacts: usize,
    source_reference_cost_ms: u64,
    fit: &Fit,
    heldout: &HeldoutDomain,
) -> Result<PolicyResult, String> {
    if target_selected.len() > target_candidates.len() {
        return Err("policy selected more target contacts than candidate pool".to_owned());
    }
    if heldout.candidates.is_empty()
        || heldout.candidates.len() != heldout.evaluator_truth_force_n.len()
    {
        return Err(
            "held-out stimulus and evaluator-truth cardinalities differ or are empty".to_owned(),
        );
    }

    let mut errors = Vec::with_capacity(heldout.candidates.len());
    let mut interval_widths = Vec::with_capacity(heldout.candidates.len());
    let mut covered = 0_usize;

    for (index, candidate) in heldout.candidates.iter().enumerate() {
        let prediction = dot(&fit.coefficients, &candidate.features);
        let error = heldout.evaluator_truth_force_n[index] - prediction;
        errors.push(error.abs());

        let variance = fit.residual_variance
            * (1.0 + quadratic_form(&candidate.features, &fit.inverse_information));
        let standard_uncertainty = variance.max(0.0).sqrt();
        let interval_width = 2.0 * 1.96 * standard_uncertainty;
        interval_widths.push(interval_width);

        if error.abs() <= 1.96 * standard_uncertainty {
            covered += 1;
        }
    }

    let count = errors.len() as f64;
    let rmse = (errors.iter().map(|error| error * error).sum::<f64>() / count).sqrt();
    let worst = errors.iter().copied().fold(0.0, f64::max);
    let mut ordered = errors.clone();
    ordered.sort_by(f64::total_cmp);
    let percentile_index = ((ordered.len() - 1) as f64 * 0.95).ceil() as usize;
    let p95 = ordered[percentile_index];
    let coverage = covered as f64 / count;
    let mean_width = interval_widths.iter().sum::<f64>() / count;
    let target_reference_cost_ms = target_selected.iter().try_fold(0_u64, |total, index| {
        let candidate = target_candidates
            .get(*index)
            .ok_or_else(|| "target selection index is out of bounds".to_owned())?;
        total
            .checked_add(candidate.cost_ms)
            .ok_or_else(|| "target synthetic cost accumulation overflow".to_owned())
    })?;
    let total_estimated_fixture_cost_ms = source_reference_cost_ms
        .checked_add(target_reference_cost_ms)
        .ok_or_else(|| "total synthetic cost accumulation overflow".to_owned())?;

    Ok(PolicyResult {
        policy_id,
        source_reference_contacts,
        target_reference_contacts: target_selected.len(),
        source_reference_cost_ms,
        target_reference_cost_ms,
        total_estimated_fixture_cost_ms,
        rmse_n: rmse,
        p95_absolute_error_n: p95,
        worst_absolute_error_n: worst,
        interval_coverage_95: coverage,
        mean_interval_width_95_n: mean_width,
        claim_class: "synthetic_model_estimate_only",
    })
}

fn candidate_cost_sum(candidates: &[Candidate]) -> Result<u64, String> {
    candidates.iter().try_fold(0_u64, |total, candidate| {
        total
            .checked_add(candidate.cost_ms)
            .ok_or_else(|| "source synthetic cost accumulation overflow".to_owned())
    })
}

fn evaluate_source_transfer(
    source_domain: &SyntheticDomain,
    heldout: &HeldoutDomain,
) -> Result<PolicyResult, String> {
    let selected: Vec<_> = (0..source_domain.candidates.len()).collect();
    let fit = fit_qualified_domain(source_domain, &selected, None, 0.0)?;
    // Source reference contacts are reported separately. They never count as
    // target-device calibration contacts and never inherit target identity.
    evaluate(
        "transfer_only_source_model",
        &[],
        &[],
        source_domain.candidates.len(),
        candidate_cost_sum(&source_domain.candidates)?,
        &fit,
        heldout,
    )
}

/// Run all frozen synthetic policies against one target profile and seed.
///
/// The fixed budget is four contacts in each temperature × loading-phase
/// stratum. The full-grid result is a reference-cost ceiling, not a fair
/// equal-budget competitor.
pub fn run_active_calibration_benchmark(seed: u64) -> Result<BenchmarkReport, String> {
    if seed == 0 {
        return Err("benchmark seed must be nonzero".to_owned());
    }

    let target_domain = target_candidate_pool(seed);
    let source_domain = source_candidate_pool(seed);
    let heldout = heldout_target_pool(seed);
    let target_candidates = &target_domain.candidates;
    let response_observability = response_observability_check(
        target_candidates,
        &SYNTHETIC_REQUIRED_FORCE_SETPOINTS_N,
        SYNTHETIC_RESPONSE_RESOLUTION,
    )?;
    if !response_observability.accepted {
        return Err("target response observability gate failed before policy selection".to_owned());
    }

    let all_target: Vec<_> = (0..target_candidates.len()).collect();
    let uniform = select_uniform(target_candidates)?;
    let random = select_random_stratified(target_candidates, seed)?;
    let maximin = select_maximin(target_candidates)?;
    let d_optimal = select_d_optimal(target_candidates)?;

    let mut results = Vec::new();

    let full_fit = fit_qualified_domain(&target_domain, &all_target, None, 0.0)?;
    results.push(evaluate(
        "target_full_grid_reference",
        target_candidates,
        &all_target,
        0,
        0,
        &full_fit,
        &heldout,
    )?);

    let uniform_fit = fit_qualified_domain(&target_domain, &uniform, None, 0.0)?;
    results.push(evaluate(
        "uniform_stratified",
        target_candidates,
        &uniform,
        0,
        0,
        &uniform_fit,
        &heldout,
    )?);

    let random_fit = fit_qualified_domain(&target_domain, &random, None, 0.0)?;
    results.push(evaluate(
        "random_stratified",
        target_candidates,
        &random,
        0,
        0,
        &random_fit,
        &heldout,
    )?);

    let maximin_fit = fit_qualified_domain(&target_domain, &maximin, None, 0.0)?;
    results.push(evaluate(
        "maximin_stratified",
        target_candidates,
        &maximin,
        0,
        0,
        &maximin_fit,
        &heldout,
    )?);

    let d_optimal_fit = fit_qualified_domain(&target_domain, &d_optimal, None, 0.0)?;
    results.push(evaluate(
        "d_optimal_stratified",
        target_candidates,
        &d_optimal,
        0,
        0,
        &d_optimal_fit,
        &heldout,
    )?);

    results.push(evaluate_source_transfer(&source_domain, &heldout)?);

    let prior = source_prior(&source_domain)?;
    let transfer_active_fit = fit_qualified_domain(&target_domain, &d_optimal, Some(&prior), 0.25)?;
    results.push(evaluate(
        "transfer_plus_d_optimal_target_adaptation",
        target_candidates,
        &d_optimal,
        source_domain.candidates.len(),
        candidate_cost_sum(&source_domain.candidates)?,
        &transfer_active_fit,
        &heldout,
    )?);

    Ok(BenchmarkReport {
        seed,
        response_observability,
        source_candidate_pool_size: source_domain.candidates.len(),
        target_candidate_pool_size: target_domain.candidates.len(),
        heldout_case_count: heldout.candidates.len(),
        target_contact_budget: TARGET_BUDGET,
        results,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetricSummary {
    pub mean: f64,
    pub median: f64,
    pub p95: f64,
    pub maximum: f64,
}

impl MetricSummary {
    fn from_samples(samples: &[f64]) -> Result<Self, String> {
        if samples.is_empty() || samples.iter().any(|sample| !sample.is_finite()) {
            return Err("metric summary requires nonempty finite samples".to_owned());
        }

        let mut ordered = samples.to_vec();
        ordered.sort_by(f64::total_cmp);
        let mean = ordered.iter().sum::<f64>() / ordered.len() as f64;
        let middle = ordered.len() / 2;
        let median = if ordered.len().is_multiple_of(2) {
            (ordered[middle - 1] + ordered[middle]) / 2.0
        } else {
            ordered[middle]
        };
        let p95_index = ((ordered.len() - 1) as f64 * 0.95).ceil() as usize;

        Ok(Self {
            mean,
            median,
            p95: ordered[p95_index],
            maximum: ordered[ordered.len() - 1],
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BootstrapMeanInterval {
    pub lower_95: f64,
    pub upper_95: f64,
}

fn bootstrap_mean_interval(
    samples: &[f64],
    seed: u64,
    resample_count: usize,
) -> Result<BootstrapMeanInterval, String> {
    if samples.len() < 2
        || samples.iter().any(|sample| !sample.is_finite())
        || seed == 0
        || resample_count < 100
    {
        return Err("bootstrap requires at least two finite paired samples, a nonzero seed and 100 resamples".to_owned());
    }

    let mut state = seed;
    let mut means = Vec::with_capacity(resample_count);
    for _ in 0..resample_count {
        let mut total = 0.0;
        for _ in 0..samples.len() {
            let index = (xorshift64(&mut state) as usize) % samples.len();
            total += samples[index];
        }
        means.push(total / samples.len() as f64);
    }

    means.sort_by(f64::total_cmp);
    let lower_index = ((means.len() - 1) as f64 * 0.025).floor() as usize;
    let upper_index = ((means.len() - 1) as f64 * 0.975).ceil() as usize;
    Ok(BootstrapMeanInterval {
        lower_95: means[lower_index],
        upper_95: means[upper_index],
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampaignPolicyResult {
    pub policy_id: &'static str,
    pub source_reference_contacts: usize,
    pub target_reference_contacts: usize,
    pub rmse_n: MetricSummary,
    pub p95_absolute_error_n: MetricSummary,
    pub worst_absolute_error_n: MetricSummary,
    pub prediction_interval_coverage_95: MetricSummary,
    pub mean_interval_width_95_n: MetricSummary,
    pub total_estimated_fixture_cost_ms: MetricSummary,
    /// Paired per-seed RMSE difference against the full target-grid arm.
    /// Negative values favor this policy on this synthetic response family.
    pub paired_rmse_delta_vs_full_grid_n: MetricSummary,
    /// Percentile bootstrap interval over paired seed-level RMSE deltas.
    /// This is synthetic-seed variability, not physical-population coverage.
    pub paired_rmse_delta_vs_full_grid_bootstrap_95: BootstrapMeanInterval,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampaignReport {
    pub first_seed: u64,
    pub replicate_count: usize,
    pub source_candidate_pool_size: usize,
    pub target_candidate_pool_size: usize,
    pub heldout_case_count: usize,
    pub target_contact_budget: usize,
    pub results: Vec<CampaignPolicyResult>,
}

/// Run a paired-seed campaign. Every policy sees the same synthetic source,
/// target and held-out domain for each seed. At least 50 replicates are required
/// so a single favorable seed cannot define the reported policy ordering.
///
/// This remains a synthetic robustness summary, not evidence of physical
/// metrological coverage or a substitute for the separately owned uncertainty
/// calculation.
pub fn run_active_calibration_campaign(
    first_seed: u64,
    replicate_count: usize,
) -> Result<CampaignReport, String> {
    if first_seed == 0 {
        return Err("campaign first seed must be nonzero".to_owned());
    }
    if replicate_count < 50 {
        return Err("campaign requires at least 50 paired seeds".to_owned());
    }
    let final_seed = first_seed
        .checked_add((replicate_count - 1) as u64)
        .ok_or_else(|| "campaign seed range overflow".to_owned())?;
    if final_seed == 0 {
        return Err("campaign seed range contains an invalid zero seed".to_owned());
    }

    let first_run = run_active_calibration_benchmark(first_seed)?;
    if first_run.results.is_empty() {
        return Err("single-seed benchmark returned no policy results".to_owned());
    }
    let policy_count = first_run.results.len();
    let mut per_policy_results =
        vec![Vec::<PolicyResult>::with_capacity(replicate_count); policy_count];
    let mut paired_rmse_deltas = vec![Vec::<f64>::with_capacity(replicate_count); policy_count];

    let full_grid_rmse = first_run.results[0].rmse_n;
    for (index, result) in first_run.results.iter().enumerate() {
        per_policy_results[index].push(result.clone());
        paired_rmse_deltas[index].push(result.rmse_n - full_grid_rmse);
    }

    for offset in 1..replicate_count {
        let seed = first_seed
            .checked_add(offset as u64)
            .ok_or_else(|| "campaign seed increment overflow".to_owned())?;
        let report = run_active_calibration_benchmark(seed)?;
        if report.results.len() != policy_count
            || report.target_candidate_pool_size != first_run.target_candidate_pool_size
            || report.source_candidate_pool_size != first_run.source_candidate_pool_size
            || report.heldout_case_count != first_run.heldout_case_count
            || report.target_contact_budget != first_run.target_contact_budget
        {
            return Err(format!("campaign schema drift at seed {seed}"));
        }

        let seed_full_grid_rmse = report.results[0].rmse_n;
        for (index, result) in report.results.into_iter().enumerate() {
            if result.policy_id != first_run.results[index].policy_id
                || result.source_reference_contacts
                    != first_run.results[index].source_reference_contacts
                || result.target_reference_contacts
                    != first_run.results[index].target_reference_contacts
            {
                return Err(format!("policy identity or budget drift at seed {seed}"));
            }
            paired_rmse_deltas[index].push(result.rmse_n - seed_full_grid_rmse);
            per_policy_results[index].push(result);
        }
    }

    let mut campaign_results = Vec::with_capacity(policy_count);
    for (policy_index, (results, paired_deltas)) in per_policy_results
        .into_iter()
        .zip(paired_rmse_deltas)
        .enumerate()
    {
        let first = results
            .first()
            .ok_or_else(|| "campaign policy result unexpectedly empty".to_owned())?;

        let collect_metric = |extract: fn(&PolicyResult) -> f64| {
            let samples: Vec<_> = results.iter().map(extract).collect();
            MetricSummary::from_samples(&samples)
        };
        let mut bootstrap_seed =
            first_seed ^ (policy_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        if bootstrap_seed == 0 {
            bootstrap_seed = 1;
        }
        let paired_bootstrap = bootstrap_mean_interval(&paired_deltas, bootstrap_seed, 2_000)?;

        campaign_results.push(CampaignPolicyResult {
            policy_id: first.policy_id,
            source_reference_contacts: first.source_reference_contacts,
            target_reference_contacts: first.target_reference_contacts,
            rmse_n: collect_metric(|result| result.rmse_n)?,
            p95_absolute_error_n: collect_metric(|result| result.p95_absolute_error_n)?,
            worst_absolute_error_n: collect_metric(|result| result.worst_absolute_error_n)?,
            prediction_interval_coverage_95: collect_metric(|result| result.interval_coverage_95)?,
            mean_interval_width_95_n: collect_metric(|result| result.mean_interval_width_95_n)?,
            total_estimated_fixture_cost_ms: collect_metric(|result| {
                result.total_estimated_fixture_cost_ms as f64
            })?,
            paired_rmse_delta_vs_full_grid_n: MetricSummary::from_samples(&paired_deltas)?,
            paired_rmse_delta_vs_full_grid_bootstrap_95: paired_bootstrap,
        });
    }

    Ok(CampaignReport {
        first_seed,
        replicate_count,
        source_candidate_pool_size: first_run.source_candidate_pool_size,
        target_candidate_pool_size: first_run.target_candidate_pool_size,
        heldout_case_count: first_run.heldout_case_count,
        target_contact_budget: first_run.target_contact_budget,
        results: campaign_results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_integrity_requires_agreement_and_ancestry_attestation() {
        let domain = target_candidate_pool(20261116);
        let report = reference_integrity_check(
            &domain.reference_force_observations_n,
            &domain.independent_check_reference_force_observations_n,
            SYNTHETIC_REFERENCE_AGREEMENT_TOLERANCE_N,
            domain.independent_reference_ancestry_attested,
        )
        .expect("valid reference agreement report");
        assert!(report.accepted);
        assert!(!report.agreement_proves_independence);
        assert!(report.independent_reference_ancestry_attested);

        let shared_cause = reference_integrity_check(
            &[1.25, 2.25, 3.25],
            &[1.25, 2.25, 3.25],
            SYNTHETIC_REFERENCE_AGREEMENT_TOLERANCE_N,
            false,
        )
        .expect("shared-cause reference report");
        assert!(!shared_cause.accepted);
        assert!(shared_cause.max_abs_disagreement_n == 0.0);
        assert!(!shared_cause.agreement_proves_independence);
    }

    #[test]
    fn malformed_reference_channels_fail_closed() {
        assert!(reference_integrity_check(&[1.0, 2.0], &[1.0], 0.02, true).is_err());
        assert!(reference_integrity_check(&[], &[], 0.02, true).is_err());
        assert!(reference_integrity_check(&[f64::NAN], &[1.0], 0.02, true).is_err());
        assert!(reference_integrity_check(&[1.0], &[1.0], -0.01, true).is_err());
    }

    #[test]
    fn nominal_response_is_observable_across_the_synthetic_force_grid() {
        let domain = target_candidate_pool(20261116);
        let report = response_observability_check(
            &domain.candidates,
            &SYNTHETIC_REQUIRED_FORCE_SETPOINTS_N,
            SYNTHETIC_RESPONSE_RESOLUTION,
        )
        .expect("nominal observability report");
        assert!(report.accepted);
        assert_eq!(report.condition_count, 12);
        assert_eq!(report.checked_transition_count, 60);
        assert_eq!(report.flatline_transition_count, 0);
    }

    #[test]
    fn flatline_and_decreasing_sensor_response_fail_observability() {
        let mut domain = target_candidate_pool(20261116);
        for candidate in &mut domain.candidates {
            if candidate.force_setpoint_n >= 8.0 {
                candidate.features[1] = 7.0;
            }
        }
        let report = response_observability_check(
            &domain.candidates,
            &SYNTHETIC_REQUIRED_FORCE_SETPOINTS_N,
            SYNTHETIC_RESPONSE_RESOLUTION,
        )
        .expect("saturation observability report");
        assert!(!report.accepted);
        assert!(report.flatline_transition_count > 0);
        assert!(report.affected_condition_count > 0);
        assert!(report.first_flatline_start_force_n.is_some());

        let mut decreasing = target_candidate_pool(20261116);
        for candidate in &mut decreasing.candidates {
            if candidate.force_setpoint_n == 8.0 {
                candidate.features[1] = 7.0;
            } else if candidate.force_setpoint_n == 10.0 {
                candidate.features[1] = 6.0;
            }
        }
        let decreasing_report = response_observability_check(
            &decreasing.candidates,
            &SYNTHETIC_REQUIRED_FORCE_SETPOINTS_N,
            SYNTHETIC_RESPONSE_RESOLUTION,
        )
        .expect("decreasing response report");
        assert!(decreasing_report.decreasing_transition_count > 0);
    }

    #[test]
    fn missing_declared_force_range_is_incomplete_observability_evidence() {
        let mut domain = target_candidate_pool(20261116);
        domain
            .candidates
            .retain(|candidate| candidate.force_setpoint_n <= 6.0);
        let report = response_observability_check(
            &domain.candidates,
            &SYNTHETIC_REQUIRED_FORCE_SETPOINTS_N,
            SYNTHETIC_RESPONSE_RESOLUTION,
        )
        .expect("incomplete force-range report");
        assert!(!report.accepted);
        assert_eq!(report.incomplete_condition_count, 12);
        assert_eq!(report.maximum_observed_force_setpoint_n, 6.0);
    }

    #[test]
    fn missing_condition_strata_do_not_pass_observability() {
        let mut domain = target_candidate_pool(20261116);
        domain.candidates.retain(|candidate| {
            !(candidate.position == -1.0 && candidate.temperature == 1.0 && candidate.phase == 1.0)
        });
        let report = response_observability_check(
            &domain.candidates,
            &SYNTHETIC_REQUIRED_FORCE_SETPOINTS_N,
            SYNTHETIC_RESPONSE_RESOLUTION,
        )
        .expect("missing condition report");
        assert!(!report.accepted);
        assert_eq!(report.missing_condition_count, 1);
    }

    #[test]
    fn qualified_fit_rejects_reference_disagreement_and_missing_ancestry() {
        let domain = target_candidate_pool(20261116);
        let selected: Vec<_> = (0..domain.candidates.len()).collect();
        assert!(fit_qualified_domain(&domain, &selected, None, 0.0).is_ok());

        let mut biased = domain.clone();
        for observation in &mut biased.reference_force_observations_n {
            *observation += 0.25;
        }
        assert!(fit_qualified_domain(&biased, &selected, None, 0.0).is_err());

        let mut unattested = domain;
        unattested.independent_reference_ancestry_attested = false;
        assert!(fit_qualified_domain(&unattested, &selected, None, 0.0).is_err());
    }

    #[test]
    fn selector_stimuli_do_not_contain_reference_labels_or_heldout_truth() {
        let target = target_candidate_pool(20261116);
        let heldout = heldout_target_pool(20261116);
        assert_eq!(target.candidates.len(), 72);
        assert_eq!(target.reference_force_observations_n.len(), 72);
        assert_eq!(heldout.candidates.len(), 60);
        assert_eq!(heldout.evaluator_truth_force_n.len(), 60);
        assert!(target.candidates.iter().all(|candidate| {
            [0.0, 2.0, 4.0, 6.0, 8.0, 10.0].contains(&candidate.force_setpoint_n)
        }));
        assert!(
            heldout.candidates.iter().all(|candidate| {
                [1.0, 3.0, 5.0, 7.0, 9.0].contains(&candidate.force_setpoint_n)
            })
        );
        assert!(
            target
                .reference_force_observations_n
                .iter()
                .any(|observed| { (*observed - 2.0).abs() > 0.0 })
        );
    }

    #[test]
    fn all_equal_budget_policies_cover_every_required_stratum() {
        let domain = target_candidate_pool(20261116);
        for selected in [
            select_uniform(&domain.candidates).expect("uniform selection"),
            select_random_stratified(&domain.candidates, 20261116).expect("random selection"),
            select_maximin(&domain.candidates).expect("maximin selection"),
            select_d_optimal(&domain.candidates).expect("D-optimal selection"),
        ] {
            assert_eq!(selected.len(), TARGET_BUDGET);
            let mut counts = [0_usize; STRATA];
            let mut unique = selected.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), TARGET_BUDGET);
            for index in selected {
                counts[domain.candidates[index].stratum] += 1;
            }
            assert_eq!(counts, [4, 4, 4, 4]);
        }
    }

    #[test]
    fn random_stratified_selection_is_seed_reproducible_and_seed_sensitive() {
        let domain = target_candidate_pool(20261116);
        let first = select_random_stratified(&domain.candidates, 42).expect("first seed");
        let replay = select_random_stratified(&domain.candidates, 42).expect("replay seed");
        let other = select_random_stratified(&domain.candidates, 43).expect("other seed");
        assert_eq!(first, replay);
        assert_ne!(first, other);
    }

    #[test]
    fn benchmark_reports_separate_model_transfer_and_target_adaptation_arms() {
        let report = run_active_calibration_benchmark(20261116).expect("benchmark runs");
        assert_eq!(report.source_candidate_pool_size, 72);
        assert_eq!(report.target_candidate_pool_size, 72);
        assert_eq!(report.heldout_case_count, 60);
        assert_eq!(report.target_contact_budget, 16);
        assert!(report.response_observability.accepted);
        assert_eq!(report.results.len(), 7);
        assert_eq!(report.results[0].source_reference_contacts, 0);
        assert_eq!(report.results[0].target_reference_contacts, 72);
        assert_eq!(report.results[1].target_reference_contacts, 16);
        assert_eq!(report.results[5].policy_id, "transfer_only_source_model");
        assert_eq!(report.results[5].source_reference_contacts, 72);
        assert_eq!(report.results[5].target_reference_contacts, 0);
        assert_eq!(report.results[6].source_reference_contacts, 72);
        assert_eq!(
            report.results[6].policy_id,
            "transfer_plus_d_optimal_target_adaptation"
        );
        assert_eq!(report.results[6].target_reference_contacts, 16);
        assert_eq!(
            report.results[6].total_estimated_fixture_cost_ms,
            report.results[6].source_reference_cost_ms + report.results[6].target_reference_cost_ms
        );
        for result in &report.results {
            assert!(result.rmse_n.is_finite());
            assert!(result.p95_absolute_error_n.is_finite());
            assert!(result.worst_absolute_error_n.is_finite());
            assert!((0.0..=1.0).contains(&result.interval_coverage_95));
            assert!(result.mean_interval_width_95_n.is_finite());
        }
    }

    #[test]
    fn paired_seed_bootstrap_is_deterministic_and_resamples_seed_level_deltas() {
        let samples = [-0.2, -0.1, 0.0, 0.1, 0.2];
        let first = bootstrap_mean_interval(&samples, 123, 2_000).expect("bootstrap interval");
        let replay = bootstrap_mean_interval(&samples, 123, 2_000).expect("deterministic replay");
        assert_eq!(first, replay);
        assert!(first.lower_95.is_finite() && first.upper_95.is_finite());
        assert!(first.lower_95 <= first.upper_95);
        assert!(first.lower_95 <= 0.0 && first.upper_95 >= 0.0);
    }

    #[test]
    fn campaign_requires_at_least_fifty_paired_replicates() {
        assert!(run_active_calibration_campaign(20261116, 49).is_err());
        assert!(run_active_calibration_campaign(0, 50).is_err());
    }

    #[test]
    fn fifty_seed_campaign_reports_paired_policy_distributions() {
        let report =
            run_active_calibration_campaign(20261116, 50).expect("fifty-seed campaign runs");
        assert_eq!(report.replicate_count, 50);
        assert_eq!(report.results.len(), 7);
        assert_eq!(report.target_candidate_pool_size, 72);
        assert_eq!(report.heldout_case_count, 60);

        let full_grid = &report.results[0];
        assert_eq!(full_grid.policy_id, "target_full_grid_reference");
        assert_eq!(full_grid.paired_rmse_delta_vs_full_grid_n.mean, 0.0);

        for policy in &report.results {
            assert!(policy.rmse_n.mean.is_finite());
            assert!(policy.rmse_n.median.is_finite());
            assert!(policy.rmse_n.p95.is_finite());
            assert!(policy.rmse_n.maximum.is_finite());
            assert!(policy.paired_rmse_delta_vs_full_grid_n.mean.is_finite());
            assert!(
                policy.paired_rmse_delta_vs_full_grid_bootstrap_95.lower_95
                    <= policy.paired_rmse_delta_vs_full_grid_bootstrap_95.upper_95
            );
            assert!((0.0..=1.0).contains(&policy.prediction_interval_coverage_95.mean));
        }
    }

    #[test]
    fn zero_seed_is_not_accepted_as_an_uninitialized_random_stream() {
        assert!(run_active_calibration_benchmark(0).is_err());
    }

    #[test]
    fn full_grid_and_active_plans_are_deterministic() {
        let first = run_active_calibration_benchmark(20261116).expect("first run");
        let replay = run_active_calibration_benchmark(20261116).expect("replay");
        assert_eq!(first, replay);
    }
}
