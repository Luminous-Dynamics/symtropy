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

const PARAMS: usize = 8;
const STRATA: usize = 4;
const CONTACTS_PER_STRATUM: usize = 4;
const TARGET_BUDGET: usize = STRATA * CONTACTS_PER_STRATUM;
const RIDGE: f64 = 1.0e-8;

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
    reference_force_observations_n: Vec<f64>,
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

fn make_candidate(
    id: usize,
    stimulus: SyntheticStimulus,
) -> (Candidate, f64) {
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
                    id += 1;
                }
            }
        }
    }
    SyntheticDomain {
        candidates,
        reference_force_observations_n,
    }
}

fn source_candidate_pool(seed: u64) -> SyntheticDomain {
    let mut candidates = Vec::with_capacity(72);
    let mut reference_force_observations_n = Vec::with_capacity(72);
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
                    id += 1;
                }
            }
        }
    }
    SyntheticDomain {
        candidates,
        reference_force_observations_n,
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
            let left_score = (left.force_n / 10.0 - 0.5).abs() + left.position.abs() * 0.25;
            let right_score = (right.force_n / 10.0 - 0.5).abs() + right.position.abs() * 0.25;
            left_score
                .total_cmp(&right_score)
                .then_with(|| right.id.cmp(&left.id))
        })
        .map(|(index, _)| index)
}

fn normalized_stimulus_distance(left: &Candidate, right: &Candidate) -> f64 {
    let force = (left.force_n - right.force_n) / 10.0;
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
    let fit = fit_model(
        &source_domain.candidates,
        &source_domain.reference_force_observations_n,
        &selected,
        None,
        0.0,
    )?;
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
        return Err("held-out stimulus and evaluator-truth cardinalities differ or are empty".to_owned());
    }

    let mut errors = Vec::with_capacity(heldout.len());
    let mut interval_widths = Vec::with_capacity(heldout.len());
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
    let fit = fit_model(
        &source_domain.candidates,
        &source_domain.reference_force_observations_n,
        &selected,
        None,
        0.0,
    )?;
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

    let all_target: Vec<_> = (0..target_candidates.len()).collect();
    let uniform = select_uniform(target_candidates)?;
    let random = select_random_stratified(target_candidates, seed)?;
    let maximin = select_maximin(target_candidates)?;
    let d_optimal = select_d_optimal(target_candidates)?;

    let mut results = Vec::new();

    let full_fit = fit_model(
        target_candidates,
        &target_domain.reference_force_observations_n,
        &all_target,
        None,
        0.0,
    )?;
    results.push(evaluate(
        "target_full_grid_reference",
        target_candidates,
        &all_target,
        0,
        0,
        &full_fit,
        &heldout,
    )?);

    let uniform_fit = fit_model(target_candidates, &target_domain.reference_force_observations_n, &uniform, None, 0.0)?;
    results.push(evaluate(
        "uniform_stratified",
        target_candidates,
        &uniform,
        0,
        0,
        &uniform_fit,
        &heldout,
    )?);

    let random_fit = fit_model(target_candidates, &target_domain.reference_force_observations_n, &random, None, 0.0)?;
    results.push(evaluate(
        "random_stratified",
        target_candidates,
        &random,
        0,
        0,
        &random_fit,
        &heldout,
    )?);

    let maximin_fit = fit_model(target_candidates, &target_domain.reference_force_observations_n, &maximin, None, 0.0)?;
    results.push(evaluate(
        "maximin_stratified",
        target_candidates,
        &maximin,
        0,
        0,
        &maximin_fit,
        &heldout,
    )?);

    let d_optimal_fit = fit_model(target_candidates, &target_domain.reference_force_observations_n, &d_optimal, None, 0.0)?;
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
    let transfer_active_fit = fit_model(
        target_candidates,
        &target_domain.reference_force_observations_n,
        &d_optimal,
        Some(&prior),
        0.25,
    )?;
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
        source_candidate_pool_size: source_domain.candidates.len(),
        target_candidate_pool_size: target_domain.candidates.len(),
        heldout_case_count: heldout.len(),
        target_contact_budget: TARGET_BUDGET,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_candidate_and_holdout_domains_are_distinct() {
        let candidates = target_candidate_pool(20261116);
        let heldout = heldout_target_pool(20261116);
        assert_eq!(candidates.len(), 72);
        assert_eq!(heldout.len(), 60);
        assert!(
            candidates
                .iter()
                .all(|candidate| { [0.0, 2.0, 4.0, 6.0, 8.0, 10.0].contains(&candidate.force_n) })
        );
        assert!(
            heldout
                .iter()
                .all(|candidate| { [1.0, 3.0, 5.0, 7.0, 9.0].contains(&candidate.force_n) })
        );
    }

    #[test]
    fn all_equal_budget_policies_cover_every_required_stratum() {
        let candidates = target_candidate_pool(20261116);
        for selected in [
            select_uniform(&candidates).expect("uniform selection"),
            select_random_stratified(&candidates, 20261116).expect("random selection"),
            select_maximin(&candidates).expect("maximin selection"),
            select_d_optimal(&candidates).expect("D-optimal selection"),
        ] {
            assert_eq!(selected.len(), TARGET_BUDGET);
            let mut counts = [0_usize; STRATA];
            let mut unique = selected.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), TARGET_BUDGET);
            for index in selected {
                counts[candidates[index].stratum] += 1;
            }
            assert_eq!(counts, [4, 4, 4, 4]);
        }
    }

    #[test]
    fn random_stratified_selection_is_seed_reproducible_and_seed_sensitive() {
        let candidates = target_candidate_pool(20261116);
        let first = select_random_stratified(&candidates, 42).expect("first seed");
        let replay = select_random_stratified(&candidates, 42).expect("replay seed");
        let other = select_random_stratified(&candidates, 43).expect("other seed");
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
