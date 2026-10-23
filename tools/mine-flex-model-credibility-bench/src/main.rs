use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{env, process::ExitCode};

const SUITE_ID: &str = "MINE-MCRED-001";
const SUITE_VERSION: &str = "0.1.0";
const GENERATOR_VERSION: &str = "mine-model-credibility-fixtures-v0.1.0";
const VALIDATION_SEEDS: usize = 32;
const HELD_OUT_SEEDS: usize = 32;
const CASES_PER_SEED: usize = 6;
const STEPS_PER_CASE: usize = 8;
const CALIBRATION_SAMPLE_COUNT: usize = 64;
const MIS_COVERAGE_ALPHA_PERCENT: usize = 5;
const MAX_MEASUREMENT_AGE_PERIODS: u8 = 1;
const HEALTHY_STEPS_TO_REENTER: usize = 3;
const PREDICTION_ERROR_LIMIT: i64 = 2;
const PROTECTED_MINIMUM: i64 = 0;
const PROTECTED_MAXIMUM: i64 = 100;

type Check<T> = Result<T, String>;

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value == value.to_ascii_lowercase()
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Split {
    Validation,
    HeldOut,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ScenarioKind {
    Nominal,
    ResidualShift,
    StaleTelemetry,
    MissingObservation,
    SafetyBoundary,
    Recovery,
}

impl ScenarioKind {
    fn all() -> [Self; CASES_PER_SEED] {
        [
            Self::Nominal,
            Self::ResidualShift,
            Self::StaleTelemetry,
            Self::MissingObservation,
            Self::SafetyBoundary,
            Self::Recovery,
        ]
    }

    fn stable_index(self) -> u64 {
        match self {
            Self::Nominal => 0,
            Self::ResidualShift => 1,
            Self::StaleTelemetry => 2,
            Self::MissingObservation => 3,
            Self::SafetyBoundary => 4,
            Self::Recovery => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Disposition {
    UseModel,
    RejectUnsafeInterval,
    AbstainMissingObservation,
    AbstainStaleTelemetry,
    AbstainResidualOutOfEnvelope,
    AbstainRecoveryHoldoff,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Observation {
    tick: usize,
    predicted_state_liters: i64,
    observed_state_liters: Option<i64>,
    interval_lower_liters: i64,
    interval_upper_liters: i64,
    protected_minimum_liters: i64,
    protected_maximum_liters: i64,
    measurement_age_periods: u8,
    injected_shift_active: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Scenario {
    seed: u64,
    split: Split,
    kind: ScenarioKind,
    observations: Vec<Observation>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct StepResult {
    tick: usize,
    disposition: Disposition,
    known_answer_disposition: Disposition,
    absolute_residual_liters: Option<i64>,
    interval_covered: Option<bool>,
    interval_width_liters: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ScenarioResult {
    seed: u64,
    split: Split,
    kind: ScenarioKind,
    steps: Vec<StepResult>,
    independent_audit_mismatches: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Summary {
    split: Split,
    kind: ScenarioKind,
    scenario_count: usize,
    step_count: usize,
    model_acceptances: usize,
    unsafe_interval_rejections: usize,
    missing_observation_abstentions: usize,
    stale_telemetry_abstentions: usize,
    residual_shift_abstentions: usize,
    recovery_holdoff_abstentions: usize,
    injected_shift_steps: usize,
    model_acceptances_during_shift: usize,
    missing_or_stale_acceptances: usize,
    unsafe_interval_acceptances: usize,
    shift_onsets: usize,
    same_tick_shift_detections: usize,
    covered_observations: usize,
    observed_count: usize,
    interval_width_sum_liters: u64,
    interval_width_count: usize,
    known_answer_mismatches: usize,
    independent_audit_mismatches: usize,
    unguarded_baseline_acceptances: usize,
    unguarded_baseline_acceptances_during_shift: usize,
    unguarded_baseline_acceptances_missing_or_stale: usize,
    freshness_only_acceptances: usize,
    freshness_only_acceptances_during_shift: usize,
    freshness_only_acceptances_missing_or_stale: usize,
    residual_only_acceptances: usize,
    residual_only_acceptances_during_shift: usize,
    residual_only_acceptances_missing_or_stale: usize,
    residual_only_acceptances_during_recovery_holdoff: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Receipt {
    result: String,
    suite_id: String,
    suite_version: String,
    generator_version: String,
    source_commit: String,
    calibration_set_sha256: String,
    generated_case_set_sha256: String,
    validation_seed_count: usize,
    held_out_seed_count: usize,
    scenarios_per_seed: usize,
    steps_per_scenario: usize,
    scenario_count: usize,
    step_count: usize,
    mis_coverage_alpha_percent: usize,
    calibrated_absolute_error_limit_liters: i64,
    max_measurement_age_periods: u8,
    healthy_steps_required_for_reentry: usize,
    known_answer_mismatches: usize,
    independent_audit_mismatches: usize,
    model_acceptances_during_shift: usize,
    missing_or_stale_acceptances: usize,
    unsafe_interval_acceptances: usize,
    hard_violations_among_model_acceptances: usize,
    shift_onsets: usize,
    same_tick_shift_detections: usize,
    unguarded_baseline_acceptances_during_shift: usize,
    unguarded_baseline_missing_or_stale_acceptances: usize,
    freshness_only_acceptances_during_shift: usize,
    freshness_only_acceptances_missing_or_stale: usize,
    residual_only_acceptances_during_shift: usize,
    residual_only_acceptances_missing_or_stale: usize,
    residual_only_acceptances_during_recovery_holdoff: usize,
    covered_observations: usize,
    observed_count: usize,
    summaries: Vec<Summary>,
    scenarios: Vec<ScenarioResult>,
    claim_ceiling: String,
}

fn finite_sample_absolute_error_limit(calibration: &[u32], alpha_percent: usize) -> Check<i64> {
    if calibration.is_empty() || alpha_percent == 0 || alpha_percent >= 100 {
        return Err(
            "calibration set and miscoverage alpha must be non-empty and in (0, 100)".to_owned(),
        );
    }
    let mut sorted = calibration.to_vec();
    sorted.sort_unstable();
    let numerator = sorted
        .len()
        .checked_add(1)
        .and_then(|n| n.checked_mul(100 - alpha_percent))
        .ok_or_else(|| "finite-sample quantile rank overflow".to_owned())?;
    let rank = numerator
        .checked_add(99)
        .ok_or_else(|| "finite-sample quantile rounding overflow".to_owned())?
        / 100;
    if rank > sorted.len() {
        return Err("calibration sample is too small for a finite requested quantile".to_owned());
    }
    let one_based_rank = rank.max(1);
    Ok(i64::from(sorted[one_based_rank - 1]))
}

fn calibration_residuals() -> Vec<u32> {
    (0..CALIBRATION_SAMPLE_COUNT)
        .map(|index| ((index * 17 + 13) % 3) as u32)
        .collect()
}

fn signed_nominal_error(seed: u64, tick: usize, kind: ScenarioKind) -> i64 {
    let magnitude = ((seed + tick as u64 * 7 + kind.stable_index()) % 3) as i64;
    if (seed + tick as u64) % 2 == 0 {
        magnitude
    } else {
        -magnitude
    }
}

fn make_observation(seed: u64, kind: ScenarioKind, tick: usize) -> Observation {
    let injected_shift_active = match kind {
        ScenarioKind::ResidualShift => tick >= 3,
        ScenarioKind::Recovery => tick < 2,
        _ => false,
    };
    let predicted = if kind == ScenarioKind::SafetyBoundary && tick >= 4 {
        99
    } else {
        50
    };
    let signed_error = if injected_shift_active {
        8
    } else {
        signed_nominal_error(seed, tick, kind)
    };
    let observed = if kind == ScenarioKind::MissingObservation && tick == 4 {
        None
    } else if kind == ScenarioKind::SafetyBoundary && tick >= 4 {
        Some(predicted - signed_error.abs())
    } else {
        Some(predicted + signed_error)
    };
    let age = if kind == ScenarioKind::StaleTelemetry && tick == 4 {
        MAX_MEASUREMENT_AGE_PERIODS + 1
    } else {
        0
    };
    Observation {
        tick,
        predicted_state_liters: predicted,
        observed_state_liters: observed,
        interval_lower_liters: predicted - PREDICTION_ERROR_LIMIT,
        interval_upper_liters: predicted + PREDICTION_ERROR_LIMIT,
        protected_minimum_liters: PROTECTED_MINIMUM,
        protected_maximum_liters: PROTECTED_MAXIMUM,
        measurement_age_periods: age,
        injected_shift_active,
    }
}

fn generate_scenarios() -> Vec<Scenario> {
    let mut scenarios = Vec::with_capacity((VALIDATION_SEEDS + HELD_OUT_SEEDS) * CASES_PER_SEED);
    for (split, seeds) in [
        (Split::Validation, 1_u64..=VALIDATION_SEEDS as u64),
        (
            Split::HeldOut,
            10_001_u64..=10_000_u64 + HELD_OUT_SEEDS as u64,
        ),
    ] {
        for seed in seeds {
            for kind in ScenarioKind::all() {
                let observations = (0..STEPS_PER_CASE)
                    .map(|tick| make_observation(seed, kind, tick))
                    .collect();
                scenarios.push(Scenario {
                    seed,
                    split,
                    kind,
                    observations,
                });
            }
        }
    }
    scenarios
}

fn absolute_residual(observation: &Observation) -> Option<i64> {
    observation.observed_state_liters.map(|observed| {
        i64::try_from(observed.abs_diff(observation.predicted_state_liters)).unwrap_or(i64::MAX)
    })
}

fn validate_scenario(scenario: &Scenario) -> Check<()> {
    if scenario.observations.len() != STEPS_PER_CASE {
        return Err(format!(
            "{} {:?}: wrong timestep count",
            scenario.seed, scenario.kind
        ));
    }
    for (expected_tick, observation) in scenario.observations.iter().enumerate() {
        if observation.tick != expected_tick {
            return Err(format!(
                "{} {:?}: non-contiguous time grid",
                scenario.seed, scenario.kind
            ));
        }
        if observation.interval_lower_liters > observation.interval_upper_liters
            || observation.protected_minimum_liters > observation.protected_maximum_liters
        {
            return Err(format!(
                "{} {:?}: malformed interval",
                scenario.seed, scenario.kind
            ));
        }
    }
    Ok(())
}

fn known_answer_disposition(kind: ScenarioKind, tick: usize) -> Disposition {
    match kind {
        ScenarioKind::Nominal => Disposition::UseModel,
        ScenarioKind::ResidualShift if tick < 3 => Disposition::UseModel,
        ScenarioKind::ResidualShift => Disposition::AbstainResidualOutOfEnvelope,
        ScenarioKind::StaleTelemetry if tick < 4 => Disposition::UseModel,
        ScenarioKind::StaleTelemetry if tick == 4 => Disposition::AbstainStaleTelemetry,
        ScenarioKind::StaleTelemetry => Disposition::AbstainRecoveryHoldoff,
        ScenarioKind::MissingObservation if tick < 4 => Disposition::UseModel,
        ScenarioKind::MissingObservation if tick == 4 => Disposition::AbstainMissingObservation,
        ScenarioKind::MissingObservation => Disposition::AbstainRecoveryHoldoff,
        ScenarioKind::SafetyBoundary if tick < 4 => Disposition::UseModel,
        ScenarioKind::SafetyBoundary => Disposition::RejectUnsafeInterval,
        ScenarioKind::Recovery if tick < 2 => Disposition::AbstainResidualOutOfEnvelope,
        ScenarioKind::Recovery if tick < 5 => Disposition::AbstainRecoveryHoldoff,
        ScenarioKind::Recovery => Disposition::UseModel,
    }
}

fn freshness_only_disposition(observation: &Observation) -> Disposition {
    if observation.observed_state_liters.is_none() {
        Disposition::AbstainMissingObservation
    } else if observation.measurement_age_periods > MAX_MEASUREMENT_AGE_PERIODS {
        Disposition::AbstainStaleTelemetry
    } else if observation.interval_lower_liters < observation.protected_minimum_liters
        || observation.interval_upper_liters > observation.protected_maximum_liters
    {
        Disposition::RejectUnsafeInterval
    } else {
        Disposition::UseModel
    }
}

fn residual_only_disposition(observation: &Observation, error_limit: i64) -> Disposition {
    if observation.observed_state_liters.is_none() {
        Disposition::AbstainMissingObservation
    } else if absolute_residual(observation).is_some_and(|value| value > error_limit) {
        Disposition::AbstainResidualOutOfEnvelope
    } else if observation.interval_lower_liters < observation.protected_minimum_liters
        || observation.interval_upper_liters > observation.protected_maximum_liters
    {
        Disposition::RejectUnsafeInterval
    } else {
        Disposition::UseModel
    }
}

fn decide_scenario(scenario: &Scenario, error_limit: i64) -> Check<ScenarioResult> {
    validate_scenario(scenario)?;
    let mut degraded_latched = false;
    let mut healthy_streak = 0_usize;
    let mut steps = Vec::with_capacity(scenario.observations.len());

    for observation in &scenario.observations {
        let residual = absolute_residual(observation);
        let disposition = if observation.observed_state_liters.is_none() {
            degraded_latched = true;
            healthy_streak = 0;
            Disposition::AbstainMissingObservation
        } else if observation.measurement_age_periods > MAX_MEASUREMENT_AGE_PERIODS {
            degraded_latched = true;
            healthy_streak = 0;
            Disposition::AbstainStaleTelemetry
        } else if residual.is_some_and(|value| value > error_limit) {
            degraded_latched = true;
            healthy_streak = 0;
            Disposition::AbstainResidualOutOfEnvelope
        } else if degraded_latched {
            healthy_streak += 1;
            if healthy_streak >= HEALTHY_STEPS_TO_REENTER {
                degraded_latched = false;
                healthy_streak = 0;
            }
            Disposition::AbstainRecoveryHoldoff
        } else if observation.interval_lower_liters < observation.protected_minimum_liters
            || observation.interval_upper_liters > observation.protected_maximum_liters
        {
            Disposition::RejectUnsafeInterval
        } else {
            Disposition::UseModel
        };

        let interval_covered = observation.observed_state_liters.map(|observed| {
            observed >= observation.interval_lower_liters
                && observed <= observation.interval_upper_liters
        });
        steps.push(StepResult {
            tick: observation.tick,
            disposition,
            known_answer_disposition: known_answer_disposition(scenario.kind, observation.tick),
            absolute_residual_liters: residual,
            interval_covered,
            interval_width_liters: observation
                .interval_upper_liters
                .saturating_sub(observation.interval_lower_liters),
        });
    }

    let independent_audit_mismatches = independently_audit_scenario(scenario, &steps, error_limit);
    Ok(ScenarioResult {
        seed: scenario.seed,
        split: scenario.split,
        kind: scenario.kind,
        steps,
        independent_audit_mismatches,
    })
}

fn independently_audit_scenario(
    scenario: &Scenario,
    steps: &[StepResult],
    error_limit: i64,
) -> usize {
    if steps.len() != scenario.observations.len() {
        return 1;
    }
    let mut mismatches = 0_usize;
    let mut locked = false;
    let mut consecutive_healthy = 0_usize;

    for (input, output) in scenario.observations.iter().zip(steps) {
        let expected = match input.observed_state_liters {
            None => {
                locked = true;
                consecutive_healthy = 0;
                Disposition::AbstainMissingObservation
            }
            Some(_) if input.measurement_age_periods > MAX_MEASUREMENT_AGE_PERIODS => {
                locked = true;
                consecutive_healthy = 0;
                Disposition::AbstainStaleTelemetry
            }
            Some(_) if absolute_residual(input).is_some_and(|value| value > error_limit) => {
                locked = true;
                consecutive_healthy = 0;
                Disposition::AbstainResidualOutOfEnvelope
            }
            Some(_) if locked => {
                consecutive_healthy += 1;
                if consecutive_healthy >= HEALTHY_STEPS_TO_REENTER {
                    locked = false;
                    consecutive_healthy = 0;
                }
                Disposition::AbstainRecoveryHoldoff
            }
            Some(_)
                if input.interval_lower_liters < input.protected_minimum_liters
                    || input.interval_upper_liters > input.protected_maximum_liters =>
            {
                Disposition::RejectUnsafeInterval
            }
            Some(_) => Disposition::UseModel,
        };
        if output.disposition != expected || output.disposition != output.known_answer_disposition {
            mismatches += 1;
        }

        if output.disposition == Disposition::UseModel {
            let stale_or_missing = input.observed_state_liters.is_none()
                || input.measurement_age_periods > MAX_MEASUREMENT_AGE_PERIODS;
            let residual_outside =
                absolute_residual(input).is_some_and(|value| value > error_limit);
            let interval_unsafe = input.interval_lower_liters < input.protected_minimum_liters
                || input.interval_upper_liters > input.protected_maximum_liters;
            if stale_or_missing || residual_outside || interval_unsafe || locked {
                mismatches += 1;
            }
        }
    }
    mismatches
}

fn aggregate(
    scenarios: &[Scenario],
    results: &[ScenarioResult],
    split: Split,
    kind: ScenarioKind,
    error_limit: i64,
) -> Summary {
    let mut summary = Summary {
        split,
        kind,
        scenario_count: 0,
        step_count: 0,
        model_acceptances: 0,
        unsafe_interval_rejections: 0,
        missing_observation_abstentions: 0,
        stale_telemetry_abstentions: 0,
        residual_shift_abstentions: 0,
        recovery_holdoff_abstentions: 0,
        injected_shift_steps: 0,
        model_acceptances_during_shift: 0,
        missing_or_stale_acceptances: 0,
        unsafe_interval_acceptances: 0,
        shift_onsets: 0,
        same_tick_shift_detections: 0,
        covered_observations: 0,
        observed_count: 0,
        interval_width_sum_liters: 0,
        interval_width_count: 0,
        known_answer_mismatches: 0,
        independent_audit_mismatches: 0,
        unguarded_baseline_acceptances: 0,
        unguarded_baseline_acceptances_during_shift: 0,
        unguarded_baseline_acceptances_missing_or_stale: 0,
        freshness_only_acceptances: 0,
        freshness_only_acceptances_during_shift: 0,
        freshness_only_acceptances_missing_or_stale: 0,
        residual_only_acceptances: 0,
        residual_only_acceptances_during_shift: 0,
        residual_only_acceptances_missing_or_stale: 0,
        residual_only_acceptances_during_recovery_holdoff: 0,
    };

    for (scenario, result) in scenarios.iter().zip(results) {
        if scenario.split != split || scenario.kind != kind {
            continue;
        }
        summary.scenario_count += 1;
        summary.independent_audit_mismatches += result.independent_audit_mismatches;
        for (index, (input, output)) in scenario.observations.iter().zip(&result.steps).enumerate()
        {
            summary.step_count += 1;
            if output.disposition != output.known_answer_disposition {
                summary.known_answer_mismatches += 1;
            }
            match output.disposition {
                Disposition::UseModel => summary.model_acceptances += 1,
                Disposition::RejectUnsafeInterval => summary.unsafe_interval_rejections += 1,
                Disposition::AbstainMissingObservation => {
                    summary.missing_observation_abstentions += 1
                }
                Disposition::AbstainStaleTelemetry => summary.stale_telemetry_abstentions += 1,
                Disposition::AbstainResidualOutOfEnvelope => {
                    summary.residual_shift_abstentions += 1
                }
                Disposition::AbstainRecoveryHoldoff => summary.recovery_holdoff_abstentions += 1,
            }

            if input.injected_shift_active {
                summary.injected_shift_steps += 1;
                if output.disposition == Disposition::UseModel {
                    summary.model_acceptances_during_shift += 1;
                }
            }
            if output.disposition == Disposition::UseModel
                && (input.observed_state_liters.is_none()
                    || input.measurement_age_periods > MAX_MEASUREMENT_AGE_PERIODS)
            {
                summary.missing_or_stale_acceptances += 1;
            }
            if output.disposition == Disposition::UseModel
                && (input.interval_lower_liters < input.protected_minimum_liters
                    || input.interval_upper_liters > input.protected_maximum_liters)
            {
                summary.unsafe_interval_acceptances += 1;
            }

            let previous_shift =
                index > 0 && scenario.observations[index - 1].injected_shift_active;
            if input.injected_shift_active && !previous_shift {
                summary.shift_onsets += 1;
                if output.disposition == Disposition::AbstainResidualOutOfEnvelope {
                    summary.same_tick_shift_detections += 1;
                }
            }
            if let Some(covered) = output.interval_covered {
                summary.observed_count += 1;
                if covered {
                    summary.covered_observations += 1;
                }
            }
            let freshness_only = freshness_only_disposition(input);
            if freshness_only == Disposition::UseModel {
                summary.freshness_only_acceptances += 1;
                if input.injected_shift_active {
                    summary.freshness_only_acceptances_during_shift += 1;
                }
            }
            if freshness_only == Disposition::UseModel
                && (input.observed_state_liters.is_none()
                    || input.measurement_age_periods > MAX_MEASUREMENT_AGE_PERIODS)
            {
                summary.freshness_only_acceptances_missing_or_stale += 1;
            }

            let residual_only = residual_only_disposition(input, error_limit);
            if residual_only == Disposition::UseModel {
                summary.residual_only_acceptances += 1;
                if input.injected_shift_active {
                    summary.residual_only_acceptances_during_shift += 1;
                }
                if input.observed_state_liters.is_none()
                    || input.measurement_age_periods > MAX_MEASUREMENT_AGE_PERIODS
                {
                    summary.residual_only_acceptances_missing_or_stale += 1;
                }
                if scenario.kind == ScenarioKind::Recovery && (2..5).contains(&index) {
                    summary.residual_only_acceptances_during_recovery_holdoff += 1;
                }
            }

            if output.disposition != Disposition::RejectUnsafeInterval
                && input.interval_lower_liters >= input.protected_minimum_liters
                && input.interval_upper_liters <= input.protected_maximum_liters
            {
                // Baseline has no credibility, freshness or drift gate: it admits every
                // in-envelope interval even when its state estimate is untrustworthy.
                summary.unguarded_baseline_acceptances += 1;
                if input.injected_shift_active {
                    summary.unguarded_baseline_acceptances_during_shift += 1;
                }
                if input.observed_state_liters.is_none()
                    || input.measurement_age_periods > MAX_MEASUREMENT_AGE_PERIODS
                {
                    summary.unguarded_baseline_acceptances_missing_or_stale += 1;
                }
            }
            summary.interval_width_sum_liters = summary.interval_width_sum_liters.saturating_add(
                u64::try_from(output.interval_width_liters.max(0)).unwrap_or(u64::MAX),
            );
            summary.interval_width_count += 1;
        }
    }
    summary
}

fn make_receipt(source_commit: &str) -> Check<Receipt> {
    if !is_lower_hex(source_commit, 40) {
        return Err("source commit must be a 40-character lowercase hexadecimal SHA".to_owned());
    }

    let calibration = calibration_residuals();
    let error_limit = finite_sample_absolute_error_limit(&calibration, MIS_COVERAGE_ALPHA_PERCENT)?;
    let scenarios = generate_scenarios();
    let scenario_bytes = serde_json::to_vec(&scenarios)
        .map_err(|error| format!("cannot serialize generated scenarios: {error}"))?;
    let calibration_bytes = serde_json::to_vec(&calibration)
        .map_err(|error| format!("cannot serialize calibration residuals: {error}"))?;
    let results = scenarios
        .iter()
        .map(|scenario| decide_scenario(scenario, error_limit))
        .collect::<Check<Vec<_>>>()?;

    let mut summaries = Vec::new();
    for split in [Split::Validation, Split::HeldOut] {
        for kind in ScenarioKind::all() {
            summaries.push(aggregate(&scenarios, &results, split, kind, error_limit));
        }
    }

    let known_answer_mismatches = summaries
        .iter()
        .map(|summary| summary.known_answer_mismatches)
        .sum::<usize>();
    let independent_audit_mismatches = summaries
        .iter()
        .map(|summary| summary.independent_audit_mismatches)
        .sum::<usize>();
    let model_acceptances_during_shift = summaries
        .iter()
        .map(|summary| summary.model_acceptances_during_shift)
        .sum::<usize>();
    let missing_or_stale_acceptances = summaries
        .iter()
        .map(|summary| summary.missing_or_stale_acceptances)
        .sum::<usize>();
    let unsafe_interval_acceptances = summaries
        .iter()
        .map(|summary| summary.unsafe_interval_acceptances)
        .sum::<usize>();
    let shift_onsets = summaries
        .iter()
        .map(|summary| summary.shift_onsets)
        .sum::<usize>();
    let same_tick_shift_detections = summaries
        .iter()
        .map(|summary| summary.same_tick_shift_detections)
        .sum::<usize>();
    let unguarded_baseline_acceptances_during_shift = summaries
        .iter()
        .map(|summary| summary.unguarded_baseline_acceptances_during_shift)
        .sum::<usize>();
    let observed_count = summaries
        .iter()
        .map(|summary| summary.observed_count)
        .sum::<usize>();
    let covered_observations = summaries
        .iter()
        .map(|summary| summary.covered_observations)
        .sum::<usize>();
    let unguarded_baseline_missing_or_stale_acceptances = summaries
        .iter()
        .map(|summary| summary.unguarded_baseline_acceptances_missing_or_stale)
        .sum::<usize>();
    let freshness_only_acceptances_during_shift = summaries
        .iter()
        .map(|summary| summary.freshness_only_acceptances_during_shift)
        .sum::<usize>();
    let freshness_only_acceptances_missing_or_stale = summaries
        .iter()
        .map(|summary| summary.freshness_only_acceptances_missing_or_stale)
        .sum::<usize>();
    let residual_only_acceptances_during_shift = summaries
        .iter()
        .map(|summary| summary.residual_only_acceptances_during_shift)
        .sum::<usize>();
    let residual_only_acceptances_missing_or_stale = summaries
        .iter()
        .map(|summary| summary.residual_only_acceptances_missing_or_stale)
        .sum::<usize>();
    let residual_only_acceptances_during_recovery_holdoff = summaries
        .iter()
        .map(|summary| summary.residual_only_acceptances_during_recovery_holdoff)
        .sum::<usize>();

    let total_steps = scenarios
        .iter()
        .map(|scenario| scenario.observations.len())
        .sum::<usize>();
    let expected_steps = (VALIDATION_SEEDS + HELD_OUT_SEEDS) * CASES_PER_SEED * STEPS_PER_CASE;
    if scenarios.len() != (VALIDATION_SEEDS + HELD_OUT_SEEDS) * CASES_PER_SEED
        || total_steps != expected_steps
        || known_answer_mismatches != 0
        || independent_audit_mismatches != 0
        || model_acceptances_during_shift != 0
        || missing_or_stale_acceptances != 0
        || unsafe_interval_acceptances != 0
        || same_tick_shift_detections != shift_onsets
    {
        return Err("model-credibility acceptance contract failed".to_owned());
    }

    let hard_violations_among_model_acceptances = model_acceptances_during_shift
        + missing_or_stale_acceptances
        + unsafe_interval_acceptances
        + independent_audit_mismatches;

    Ok(Receipt {
        result: "validated".to_owned(),
        suite_id: SUITE_ID.to_owned(),
        suite_version: SUITE_VERSION.to_owned(),
        generator_version: GENERATOR_VERSION.to_owned(),
        source_commit: source_commit.to_owned(),
        calibration_set_sha256: sha256(&calibration_bytes),
        generated_case_set_sha256: sha256(&scenario_bytes),
        validation_seed_count: VALIDATION_SEEDS,
        held_out_seed_count: HELD_OUT_SEEDS,
        scenarios_per_seed: CASES_PER_SEED,
        steps_per_scenario: STEPS_PER_CASE,
        scenario_count: scenarios.len(),
        step_count: total_steps,
        mis_coverage_alpha_percent: MIS_COVERAGE_ALPHA_PERCENT,
        calibrated_absolute_error_limit_liters: error_limit,
        max_measurement_age_periods: MAX_MEASUREMENT_AGE_PERIODS,
        healthy_steps_required_for_reentry: HEALTHY_STEPS_TO_REENTER,
        known_answer_mismatches,
        independent_audit_mismatches,
        model_acceptances_during_shift,
        missing_or_stale_acceptances,
        unsafe_interval_acceptances,
        hard_violations_among_model_acceptances,
        shift_onsets,
        same_tick_shift_detections,
        unguarded_baseline_acceptances_during_shift,
        unguarded_baseline_missing_or_stale_acceptances,
        freshness_only_acceptances_during_shift,
        freshness_only_acceptances_missing_or_stale,
        residual_only_acceptances_during_shift,
        residual_only_acceptances_missing_or_stale,
        residual_only_acceptances_during_recovery_holdoff,
        covered_observations,
        observed_count,
        summaries,
        scenarios: results,
        claim_ceiling: "finite synthetic credibility-gate contract; empirical fixture coverage "
            .to_owned()
            + "is not a distribution-free guarantee; no site-calibrated model or safety case",
    })
}

fn run() -> Check<ExitCode> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: mine-flex-model-credibility-bench --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let receipt = make_receipt(&args[1])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize model-credibility receipt: {error}"))?
    );
    Ok(if receipt.result == "validated" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_sample_quantile_is_explicit_and_deterministic() {
        let calibration = calibration_residuals();
        assert_eq!(
            2,
            finite_sample_absolute_error_limit(&calibration, 5).expect("q95")
        );
        assert!(finite_sample_absolute_error_limit(&[], 5).is_err());
        assert!(finite_sample_absolute_error_limit(&calibration, 0).is_err());
        assert!(finite_sample_absolute_error_limit(&calibration, 1).is_err());
        assert!(finite_sample_absolute_error_limit(&calibration, 100).is_err());
    }

    #[test]
    fn every_synthetic_control_matches_its_frozen_disposition_oracle() {
        let receipt = make_receipt("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .expect("synthetic contract must validate");
        assert_eq!("validated", receipt.result);
        assert_eq!(384, receipt.scenario_count);
        assert_eq!(3072, receipt.step_count);
        assert_eq!(32, receipt.validation_seed_count);
        assert_eq!(32, receipt.held_out_seed_count);
        assert_eq!(0, receipt.known_answer_mismatches);
        assert_eq!(0, receipt.independent_audit_mismatches);
        assert_eq!(0, receipt.hard_violations_among_model_acceptances);
        assert_eq!(receipt.shift_onsets, receipt.same_tick_shift_detections);
        assert_eq!(12, receipt.summaries.len());
        assert_eq!(448, receipt.freshness_only_acceptances_during_shift);
        assert_eq!(0, receipt.freshness_only_acceptances_missing_or_stale);
        assert_eq!(0, receipt.residual_only_acceptances_during_shift);
        assert_eq!(64, receipt.residual_only_acceptances_missing_or_stale);
        assert_eq!(
            192,
            receipt.residual_only_acceptances_during_recovery_holdoff
        );
    }

    #[test]
    fn abrupt_shift_is_rejected_on_the_first_out_of_envelope_observation() {
        let limit = finite_sample_absolute_error_limit(&calibration_residuals(), 5).expect("q95");
        let scenario = generate_scenarios()
            .into_iter()
            .find(|scenario| scenario.seed == 1 && scenario.kind == ScenarioKind::ResidualShift)
            .expect("shift fixture");
        let result = decide_scenario(&scenario, limit).expect("evaluate shift");
        assert_eq!(
            Disposition::AbstainResidualOutOfEnvelope,
            result.steps[3].disposition
        );
        assert_eq!(0, result.steps[3].tick - 3);
        assert!(result.steps[3..]
            .iter()
            .all(|step| { step.disposition == Disposition::AbstainResidualOutOfEnvelope }));
    }

    #[test]
    fn reentry_requires_three_healthy_observations_and_one_more_step_to_use_model() {
        let limit = finite_sample_absolute_error_limit(&calibration_residuals(), 5).expect("q95");
        let scenario = generate_scenarios()
            .into_iter()
            .find(|scenario| scenario.seed == 1 && scenario.kind == ScenarioKind::Recovery)
            .expect("recovery fixture");
        let result = decide_scenario(&scenario, limit).expect("evaluate recovery");
        assert_eq!(
            Disposition::AbstainResidualOutOfEnvelope,
            result.steps[0].disposition
        );
        assert_eq!(
            Disposition::AbstainResidualOutOfEnvelope,
            result.steps[1].disposition
        );
        assert_eq!(
            Disposition::AbstainRecoveryHoldoff,
            result.steps[2].disposition
        );
        assert_eq!(
            Disposition::AbstainRecoveryHoldoff,
            result.steps[3].disposition
        );
        assert_eq!(
            Disposition::AbstainRecoveryHoldoff,
            result.steps[4].disposition
        );
        assert_eq!(Disposition::UseModel, result.steps[5].disposition);
    }

    #[test]
    fn missing_and_stale_measurements_never_get_model_acceptance() {
        let limit = finite_sample_absolute_error_limit(&calibration_residuals(), 5).expect("q95");
        for kind in [
            ScenarioKind::MissingObservation,
            ScenarioKind::StaleTelemetry,
        ] {
            let scenario = generate_scenarios()
                .into_iter()
                .find(|scenario| scenario.seed == 1 && scenario.kind == kind)
                .expect("telemetry fixture");
            let result = decide_scenario(&scenario, limit).expect("evaluate telemetry");
            assert_ne!(Disposition::UseModel, result.steps[4].disposition);
            assert!(result.steps[4..]
                .iter()
                .all(|step| { step.disposition != Disposition::UseModel }));
        }
    }

    #[test]
    fn a_healthy_model_cannot_admit_an_interval_that_crosses_a_protected_bound() {
        let limit = finite_sample_absolute_error_limit(&calibration_residuals(), 5).expect("q95");
        let scenario = generate_scenarios()
            .into_iter()
            .find(|scenario| scenario.seed == 1 && scenario.kind == ScenarioKind::SafetyBoundary)
            .expect("boundary fixture");
        let result = decide_scenario(&scenario, limit).expect("evaluate boundary");
        assert_eq!(
            Disposition::RejectUnsafeInterval,
            result.steps[4].disposition
        );
        assert!(result.steps[4..]
            .iter()
            .all(|step| { step.disposition == Disposition::RejectUnsafeInterval }));
    }

    #[test]
    fn adversarial_age_and_residual_grid_never_admits_invalid_observations() {
        let error_limit =
            finite_sample_absolute_error_limit(&calibration_residuals(), 5).expect("q95");
        let baseline = generate_scenarios()
            .into_iter()
            .find(|scenario| scenario.seed == 1 && scenario.kind == ScenarioKind::Nominal)
            .expect("nominal fixture");

        for age in 0..=3 {
            for residual in -5_i64..=5_i64 {
                let mut scenario = baseline.clone();
                scenario.observations[0].measurement_age_periods = age;
                scenario.observations[0].observed_state_liters =
                    Some(scenario.observations[0].predicted_state_liters + residual);
                let result = decide_scenario(&scenario, error_limit).expect("evaluate mutation");
                let disposition = result.steps[0].disposition;
                if age > MAX_MEASUREMENT_AGE_PERIODS {
                    assert_eq!(Disposition::AbstainStaleTelemetry, disposition);
                } else if residual.abs() > error_limit {
                    assert_eq!(Disposition::AbstainResidualOutOfEnvelope, disposition);
                } else {
                    assert_eq!(Disposition::UseModel, disposition);
                }
            }

            let mut missing = baseline.clone();
            missing.observations[0].measurement_age_periods = age;
            missing.observations[0].observed_state_liters = None;
            let result = decide_scenario(&missing, error_limit).expect("evaluate missing");
            assert_eq!(
                Disposition::AbstainMissingObservation,
                result.steps[0].disposition
            );
        }
    }

    #[test]
    fn widening_an_interval_across_a_protected_bound_never_admits_it() {
        let error_limit =
            finite_sample_absolute_error_limit(&calibration_residuals(), 5).expect("q95");
        let baseline = generate_scenarios()
            .into_iter()
            .find(|scenario| scenario.seed == 1 && scenario.kind == ScenarioKind::Nominal)
            .expect("nominal fixture");

        for lower in -5_i64..0 {
            let mut scenario = baseline.clone();
            scenario.observations[0].interval_lower_liters = lower;
            scenario.observations[0].interval_upper_liters = 52;
            let result = decide_scenario(&scenario, error_limit).expect("evaluate lower bound");
            assert_eq!(
                Disposition::RejectUnsafeInterval,
                result.steps[0].disposition
            );
        }
        let mut lower_boundary = baseline.clone();
        lower_boundary.observations[0].interval_lower_liters = 0;
        lower_boundary.observations[0].interval_upper_liters = 52;
        let result = decide_scenario(&lower_boundary, error_limit).expect("lower boundary");
        assert_eq!(Disposition::UseModel, result.steps[0].disposition);

        for upper in 100_i64..=105 {
            let mut scenario = baseline.clone();
            scenario.observations[0].interval_lower_liters = 48;
            scenario.observations[0].interval_upper_liters = upper;
            let result = decide_scenario(&scenario, error_limit).expect("evaluate upper bound");
            if upper == 100 {
                assert_eq!(Disposition::UseModel, result.steps[0].disposition);
            } else {
                assert_eq!(
                    Disposition::RejectUnsafeInterval,
                    result.steps[0].disposition
                );
            }
        }
    }

    #[test]
    fn receipt_is_byte_deterministic_and_rejects_invalid_source_identity() {
        let source = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let first = make_receipt(source).expect("first receipt");
        let second = make_receipt(source).expect("second receipt");
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first"),
            serde_json::to_vec(&second).expect("serialize second")
        );
        assert!(make_receipt("not-a-sha").is_err());
    }

    #[test]
    fn independent_audit_detects_a_forged_model_acceptance() {
        let limit = finite_sample_absolute_error_limit(&calibration_residuals(), 5).expect("q95");
        let scenario = generate_scenarios()
            .into_iter()
            .find(|scenario| scenario.seed == 1 && scenario.kind == ScenarioKind::ResidualShift)
            .expect("shift fixture");
        let mut result = decide_scenario(&scenario, limit).expect("evaluate shift");
        result.steps[3].disposition = Disposition::UseModel;
        assert!(independently_audit_scenario(&scenario, &result.steps, limit) > 0);
    }
}
