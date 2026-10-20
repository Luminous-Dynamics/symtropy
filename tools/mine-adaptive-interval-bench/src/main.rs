use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
    env,
    process::ExitCode,
};

const SUITE_ID: &str = "MINE-ACP-001";
const SUITE_VERSION: &str = "0.1.0";
const GENERATOR_VERSION: &str = "mine-adaptive-interval-streams-v0.1.0";
const VALIDATION_SEEDS: usize = 32;
const HELD_OUT_SEEDS: usize = 32;
const TICKS_PER_STREAM: usize = 64;
const STREAM_KIND_COUNT: usize = 12;
const NOMINAL_COVERAGE_PERCENT: usize = 90;
const FIXED_CALIBRATION_SIZE: usize = 64;
const ACI_TARGET_ALPHA: f64 = 0.10;
const ACI_LEARNING_RATE: f64 = 0.10;
const COP_LEARNING_RATE: f64 = 0.10;
const COP_INTEGRATOR_SCALE: f64 = 0.5;
const COP_WINDOW_SIZE: usize = 64;
const TARGET_COVERAGE: f64 = NOMINAL_COVERAGE_PERCENT as f64 / 100.0;
const COVERAGE_BAND_LOWER: f64 = 0.85;
const COVERAGE_BAND_UPPER: f64 = 0.95;
const COVERAGE_WINDOW_TICKS: usize = 8;
const MIN_LABELS_PER_COVERAGE_WINDOW: usize = 6;
const COVERAGE_BAND_SUSTAINED_WINDOWS: usize = 3;
const MAX_TELEMETRY_AGE: u8 = 1;
const TRUST_RESIDUAL_LIMIT: f64 = 3.0;
const HEALTHY_TICKS_TO_REENTER: usize = 3;
const INITIAL_RADIUS: f64 = 3.0;
const MAX_ADAPTIVE_RADIUS: f64 = 12.0;
const PROTECTED_MINIMUM: f64 = 0.0;
const PROTECTED_MAXIMUM: f64 = 100.0;

type Check<T> = Result<T, String>;

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value == value.to_ascii_lowercase()
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Split {
    Validation,
    HeldOut,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum ScenarioKind {
    Nominal,
    CovariateShift,
    ErrorScaleShift,
    AbruptShift,
    SlowDrift,
    Reversion,
    DelayedLabels,
    MissingLabels,
    StaleTelemetry,
    MissingTelemetry,
    SafetyBoundary,
    UnseenRegime,
}

impl ScenarioKind {
    fn all() -> [Self; STREAM_KIND_COUNT] {
        [
            Self::Nominal,
            Self::CovariateShift,
            Self::ErrorScaleShift,
            Self::AbruptShift,
            Self::SlowDrift,
            Self::Reversion,
            Self::DelayedLabels,
            Self::MissingLabels,
            Self::StaleTelemetry,
            Self::MissingTelemetry,
            Self::SafetyBoundary,
            Self::UnseenRegime,
        ]
    }

    fn incident_start(self) -> Option<usize> {
        match self {
            Self::Nominal => None,
            Self::StaleTelemetry | Self::MissingTelemetry => Some(24),
            Self::SafetyBoundary => Some(0),
            _ => Some(20),
        }
    }

    fn incident_active(self, tick: usize) -> bool {
        match self {
            Self::Nominal => false,
            Self::Reversion => (20..40).contains(&tick),
            Self::SafetyBoundary => true,
            Self::StaleTelemetry | Self::MissingTelemetry => tick >= 24,
            _ => tick >= 20,
        }
    }

    fn is_covariate_novel(self, tick: usize) -> bool {
        matches!(self, Self::CovariateShift | Self::UnseenRegime) && tick >= 20
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Policy {
    FixedSplitIntervalOnly,
    OnlineResidualFeedbackIntervalOnly,
    RollingQuantileIntervalOnly,
    MultiHorizonIntervalOnly,
    FixedSplitTrustGated,
    MultiHorizonTrustGated,
    MultiHorizonUnguardedNegativeControl,
    AciClippedReferenceIntervalOnly,
    CopReferenceIntervalOnly,
}

impl Policy {
    fn all() -> [Self; 9] {
        [
            Self::FixedSplitIntervalOnly,
            Self::OnlineResidualFeedbackIntervalOnly,
            Self::RollingQuantileIntervalOnly,
            Self::MultiHorizonIntervalOnly,
            Self::FixedSplitTrustGated,
            Self::MultiHorizonTrustGated,
            Self::MultiHorizonUnguardedNegativeControl,
            Self::AciClippedReferenceIntervalOnly,
            Self::CopReferenceIntervalOnly,
        ]
    }

    fn has_trust_gate(self) -> bool {
        matches!(
            self,
            Self::FixedSplitTrustGated | Self::MultiHorizonTrustGated
        )
    }

    fn is_diagnostic_only(self) -> bool {
        matches!(
            self,
            Self::FixedSplitIntervalOnly
                | Self::OnlineResidualFeedbackIntervalOnly
                | Self::RollingQuantileIntervalOnly
                | Self::MultiHorizonIntervalOnly
                | Self::AciClippedReferenceIntervalOnly
                | Self::CopReferenceIntervalOnly
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Decision {
    DiagnoseOnly,
    UseModel,
    AbstainMissingTelemetry,
    AbstainStaleTelemetry,
    AbstainRegimeNovelty,
    AbstainResidualOutOfEnvelope,
    AbstainRecoveryHoldoff,
    RejectUnsafeInterval,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Tick {
    tick: usize,
    prediction: f64,
    target: f64,
    target_label_available: bool,
    target_label_delay_ticks: usize,
    telemetry_available: bool,
    telemetry_age_periods: u8,
    monitor_residual_abs: Option<f64>,
    regime_novelty: bool,
    incident_active: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Stream {
    seed: u64,
    split: Split,
    kind: ScenarioKind,
    ticks: Vec<Tick>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct StepOutcome {
    tick: usize,
    radius: f64,
    decision: Decision,
    coverage: Option<bool>,
    label_matures_at: Option<usize>,
    interval_width: f64,
    hard_control: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Summary {
    split: Split,
    kind: ScenarioKind,
    policy: Policy,
    stream_count: usize,
    tick_count: usize,
    coverage_count: usize,
    coverage_denominator: usize,
    empirical_coverage: Option<f64>,
    signed_coverage_error: Option<f64>,
    absolute_coverage_error: Option<f64>,
    stream_coverage_count: usize,
    mean_stream_coverage: Option<f64>,
    p05_stream_coverage: Option<f64>,
    p95_stream_coverage: Option<f64>,
    coverage_recovery_attempts: usize,
    coverage_recovery_successes: usize,
    coverage_recovery_rate: Option<f64>,
    coverage_recovery_delay_mean_ticks: Option<f64>,
    coverage_recovery_delay_p95_ticks: Option<f64>,
    coverage_recovery_delay_max_ticks: Option<usize>,
    post_recovery_windows_in_band: usize,
    post_recovery_windows_observed: usize,
    post_recovery_persistence_rate: Option<f64>,
    mean_interval_width: f64,
    p95_interval_width: f64,
    max_interval_width: f64,
    model_use_steps: usize,
    diagnostic_only_steps: usize,
    abstention_steps: usize,
    unsafe_interval_rejections: usize,
    target_labels_missing: usize,
    label_updates_applied: usize,
    hard_control_acceptances: usize,
    incident_onsets: usize,
    onsets_detected: usize,
    onsets_detected_same_tick: usize,
    detection_delay_mean_ticks: Option<f64>,
    detection_delay_max_ticks: Option<usize>,
    independent_gate_audit_mismatches: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Receipt {
    result: String,
    suite_id: String,
    suite_version: String,
    generator_version: String,
    source_commit: String,
    calibration_set_sha256: String,
    generated_stream_set_sha256: String,
    validation_seed_count: usize,
    held_out_seed_count: usize,
    scenario_kind_count: usize,
    ticks_per_stream: usize,
    stream_count: usize,
    tick_count: usize,
    policy_count: usize,
    summary_count: usize,
    nominal_coverage_percent: usize,
    coverage_band_lower: f64,
    coverage_band_upper: f64,
    coverage_window_ticks: usize,
    min_labels_per_coverage_window: usize,
    coverage_band_sustained_windows: usize,
    trust_residual_limit: f64,
    max_telemetry_age_periods: u8,
    healthy_ticks_required_for_reentry: usize,
    guarded_hard_control_acceptances: usize,
    unguarded_negative_control_hard_control_acceptances: usize,
    independent_gate_audit_mismatches: usize,
    summaries: Vec<Summary>,
    claim_ceiling: String,
}

fn calibration_residuals() -> Vec<f64> {
    (0..FIXED_CALIBRATION_SIZE)
        .map(|index| ((index * 17 + 13) % 4) as f64)
        .collect()
}

fn quantile(values: &[f64], percentile: usize) -> Option<f64> {
    if values.is_empty() || percentile == 0 || percentile > 100 {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let numerator = percentile.checked_mul(sorted.len())?;
    let rank = numerator.checked_add(99)? / 100;
    Some(sorted[rank.max(1).min(sorted.len()) - 1])
}

fn fixed_split_radius(calibration: &[f64], miscoverage_percent: usize) -> Check<f64> {
    if calibration.is_empty() || miscoverage_percent == 0 || miscoverage_percent >= 100 {
        return Err("calibration must be non-empty and miscoverage must be in (0, 100)".to_owned());
    }
    let mut sorted = calibration.to_vec();
    if sorted
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("calibration residuals must be finite and nonnegative".to_owned());
    }
    sorted.sort_by(f64::total_cmp);
    let numerator = sorted
        .len()
        .checked_add(1)
        .and_then(|count| count.checked_mul(100 - miscoverage_percent))
        .ok_or_else(|| "finite-sample quantile rank overflow".to_owned())?;
    let rank = numerator
        .checked_add(99)
        .ok_or_else(|| "finite-sample quantile rounding overflow".to_owned())?
        / 100;
    if rank > sorted.len() {
        return Err(
            "calibration set is too small for the requested finite-sample quantile".to_owned(),
        );
    }
    Ok(sorted[rank.max(1) - 1])
}

fn nominal_error(seed: u64, tick: usize) -> f64 {
    ((seed + tick as u64 * 7) % 5) as f64 - 2.0
}

fn tick_error(seed: u64, kind: ScenarioKind, tick: usize) -> f64 {
    let base = nominal_error(seed, tick);
    match kind {
        ScenarioKind::Nominal
        | ScenarioKind::CovariateShift
        | ScenarioKind::StaleTelemetry
        | ScenarioKind::MissingTelemetry
        | ScenarioKind::SafetyBoundary => base,
        ScenarioKind::ErrorScaleShift if tick >= 20 => {
            let magnitude = 5.0 + ((seed + tick as u64) % 3) as f64;
            if (seed + tick as u64) % 2 == 0 {
                magnitude
            } else {
                -magnitude
            }
        }
        ScenarioKind::ErrorScaleShift => base,
        ScenarioKind::AbruptShift | ScenarioKind::DelayedLabels | ScenarioKind::MissingLabels
            if tick >= 20 =>
        {
            6.0
        }
        ScenarioKind::AbruptShift | ScenarioKind::DelayedLabels | ScenarioKind::MissingLabels => {
            base
        }
        ScenarioKind::SlowDrift if tick >= 20 => base + ((tick - 19).min(6) as f64),
        ScenarioKind::SlowDrift => base,
        ScenarioKind::Reversion if (20..40).contains(&tick) => 6.0,
        ScenarioKind::Reversion => base,
        ScenarioKind::UnseenRegime if tick >= 20 => 7.0,
        ScenarioKind::UnseenRegime => base,
    }
}

fn generate_stream(seed: u64, split: Split, kind: ScenarioKind) -> Stream {
    let ticks = (0..TICKS_PER_STREAM)
        .map(|tick| {
            let prediction = if kind == ScenarioKind::SafetyBoundary {
                1.0
            } else {
                50.0
            };
            let mut error = tick_error(seed, kind, tick);
            let mut target = prediction + error;
            if kind == ScenarioKind::SafetyBoundary {
                target = target.max(PROTECTED_MINIMUM);
                error = target - prediction;
            }
            let target_label_available = !(kind == ScenarioKind::MissingLabels
                && tick >= 20
                && (seed + tick as u64) % 3 == 0);
            let target_label_delay_ticks = if kind == ScenarioKind::DelayedLabels && tick >= 20 {
                5
            } else {
                0
            };
            let telemetry_available = !(kind == ScenarioKind::MissingTelemetry && tick >= 24);
            let telemetry_age_periods = if kind == ScenarioKind::StaleTelemetry && tick >= 24 {
                MAX_TELEMETRY_AGE + 1
            } else {
                0
            };
            // The present-state monitor is a distinct deterministic measurement
            // channel, with a separate seed stream from the forecast target labels.
            // Real deployment must establish this separation from sensor timing/data.
            let monitor_error = tick_error(seed.wrapping_add(7_919), kind, tick);
            let monitor_residual_abs = if telemetry_available {
                Some(monitor_error.abs())
            } else {
                None
            };
            Tick {
                tick,
                prediction,
                target,
                target_label_available,
                target_label_delay_ticks,
                telemetry_available,
                telemetry_age_periods,
                monitor_residual_abs,
                regime_novelty: kind.is_covariate_novel(tick),
                incident_active: kind.incident_active(tick),
            }
        })
        .collect();

    Stream {
        seed,
        split,
        kind,
        ticks,
    }
}

fn generate_streams() -> Vec<Stream> {
    let mut streams = Vec::with_capacity((VALIDATION_SEEDS + HELD_OUT_SEEDS) * STREAM_KIND_COUNT);
    for (split, seeds) in [
        (Split::Validation, 1_u64..=VALIDATION_SEEDS as u64),
        (
            Split::HeldOut,
            10_001_u64..=10_000_u64 + HELD_OUT_SEEDS as u64,
        ),
    ] {
        for seed in seeds {
            for kind in ScenarioKind::all() {
                streams.push(generate_stream(seed, split, kind));
            }
        }
    }
    streams
}

fn interval_is_unsafe(prediction: f64, radius: f64) -> bool {
    prediction - radius < PROTECTED_MINIMUM || prediction + radius > PROTECTED_MAXIMUM
}

fn is_hard_control(tick: &Tick, radius: f64) -> bool {
    !tick.telemetry_available
        || tick.telemetry_age_periods > MAX_TELEMETRY_AGE
        || tick.regime_novelty
        || tick
            .monitor_residual_abs
            .is_none_or(|residual| residual > TRUST_RESIDUAL_LIMIT)
        || interval_is_unsafe(tick.prediction, radius)
}

fn guarded_decision(
    tick: &Tick,
    radius: f64,
    latched: &mut bool,
    healthy_streak: &mut usize,
) -> Decision {
    if !tick.telemetry_available {
        *latched = true;
        *healthy_streak = 0;
        return Decision::AbstainMissingTelemetry;
    }
    if tick.telemetry_age_periods > MAX_TELEMETRY_AGE {
        *latched = true;
        *healthy_streak = 0;
        return Decision::AbstainStaleTelemetry;
    }
    if tick.regime_novelty {
        *latched = true;
        *healthy_streak = 0;
        return Decision::AbstainRegimeNovelty;
    }
    if tick
        .monitor_residual_abs
        .is_none_or(|residual| residual > TRUST_RESIDUAL_LIMIT)
    {
        *latched = true;
        *healthy_streak = 0;
        return Decision::AbstainResidualOutOfEnvelope;
    }
    if *latched {
        *healthy_streak += 1;
        if *healthy_streak >= HEALTHY_TICKS_TO_REENTER {
            *latched = false;
            *healthy_streak = 0;
        }
        return Decision::AbstainRecoveryHoldoff;
    }
    if interval_is_unsafe(tick.prediction, radius) {
        return Decision::RejectUnsafeInterval;
    }
    Decision::UseModel
}

fn coverage_band_recovery_delay(outcomes: &[StepOutcome], onset: usize) -> Option<usize> {
    if COVERAGE_WINDOW_TICKS == 0 || COVERAGE_BAND_SUSTAINED_WINDOWS == 0 || onset >= outcomes.len()
    {
        return None;
    }

    let mut last_matured_count = 0_usize;
    let mut sustained_windows = 0_usize;
    for end in onset..outcomes.len() {
        // Take the last W label outcomes that have actually matured by this time,
        // not the last W wall-clock ticks. Repeated clock ticks with no new mature
        // label do not count as additional successful windows.
        let matured = outcomes
            .iter()
            .filter(|outcome| outcome.tick >= onset && outcome.tick <= end)
            .filter_map(|outcome| {
                let matures_at = outcome.label_matures_at?;
                if matures_at <= end {
                    outcome.coverage
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if matured.len() == last_matured_count {
            continue;
        }
        last_matured_count = matured.len();
        if matured.len() < MIN_LABELS_PER_COVERAGE_WINDOW {
            continue;
        }
        let start = matured.len().saturating_sub(COVERAGE_WINDOW_TICKS);
        let window = &matured[start..];
        let covered = window.iter().filter(|covered| **covered).count();
        let empirical = covered as f64 / window.len() as f64;
        if (COVERAGE_BAND_LOWER..=COVERAGE_BAND_UPPER).contains(&empirical) {
            sustained_windows += 1;
            if sustained_windows >= COVERAGE_BAND_SUSTAINED_WINDOWS {
                return Some(end - onset + 1);
            }
        } else {
            sustained_windows = 0;
        }
    }
    None
}

fn post_recovery_window_counts(
    outcomes: &[StepOutcome],
    onset: usize,
) -> Option<(usize, usize)> {
    let recovery_delay = coverage_band_recovery_delay(outcomes, onset)?;
    let recovery_end = onset + recovery_delay - 1;
    let mut last_matured_count = 0_usize;
    let mut observed = 0_usize;
    let mut in_band = 0_usize;
    for end in recovery_end + 1..outcomes.len() {
        let matured = outcomes
            .iter()
            .filter(|outcome| outcome.tick >= onset && outcome.tick <= end)
            .filter_map(|outcome| {
                let matures_at = outcome.label_matures_at?;
                if matures_at <= end { outcome.coverage } else { None }
            })
            .collect::<Vec<_>>();
        if matured.len() == last_matured_count {
            continue;
        }
        last_matured_count = matured.len();
        if matured.len() < MIN_LABELS_PER_COVERAGE_WINDOW {
            continue;
        }
        let start = matured.len().saturating_sub(COVERAGE_WINDOW_TICKS);
        let window = &matured[start..];
        let covered = window.iter().filter(|covered| **covered).count();
        let empirical = covered as f64 / window.len() as f64;
        observed += 1;
        if (COVERAGE_BAND_LOWER..=COVERAGE_BAND_UPPER).contains(&empirical) {
            in_band += 1;
        }
    }
    Some((in_band, observed))
}

fn independently_replay_gate(stream: &Stream, radii: &[f64], actual: &[Decision]) -> usize {
    if radii.len() != stream.ticks.len() || actual.len() != stream.ticks.len() {
        return 1;
    }
    let mut locked = false;
    let mut healthy = 0_usize;
    let mut mismatches = 0_usize;

    for (index, tick) in stream.ticks.iter().enumerate() {
        // Independent expression of the admission contract: first handle invalid
        // observation channels, then persistent recovery holdoff, then interval safety.
        let expected = if !tick.telemetry_available {
            locked = true;
            healthy = 0;
            Decision::AbstainMissingTelemetry
        } else if tick.telemetry_age_periods > MAX_TELEMETRY_AGE {
            locked = true;
            healthy = 0;
            Decision::AbstainStaleTelemetry
        } else if tick.regime_novelty {
            locked = true;
            healthy = 0;
            Decision::AbstainRegimeNovelty
        } else if tick
            .monitor_residual_abs
            .map_or(true, |residual| residual > TRUST_RESIDUAL_LIMIT)
        {
            locked = true;
            healthy = 0;
            Decision::AbstainResidualOutOfEnvelope
        } else if locked {
            healthy += 1;
            if healthy == HEALTHY_TICKS_TO_REENTER {
                locked = false;
                healthy = 0;
            }
            Decision::AbstainRecoveryHoldoff
        } else if tick.prediction - radii[index] < PROTECTED_MINIMUM
            || tick.prediction + radii[index] > PROTECTED_MAXIMUM
        {
            Decision::RejectUnsafeInterval
        } else {
            Decision::UseModel
        };
        if expected != actual[index] {
            mismatches += 1;
        }
    }
    mismatches
}

fn online_radius_update(radius: f64, mature_residual: f64) -> f64 {
    if mature_residual > radius {
        (radius + 0.75).min(MAX_ADAPTIVE_RADIUS)
    } else {
        (radius * 0.98).max(1.0)
    }
}

fn rolling_radius(history: &[f64], window: usize, fallback: f64) -> f64 {
    let start = history.len().saturating_sub(window);
    let recent = &history[start..];
    if recent.len() < 8 {
        fallback
    } else {
        quantile(recent, NOMINAL_COVERAGE_PERCENT).unwrap_or(fallback)
    }
}

fn radius_for(
    policy: Policy,
    fixed_radius: f64,
    online_radius: f64,
    aci_alpha: f64,
    cop_threshold: f64,
    cop_integrator: f64,
    history: &[f64],
) -> f64 {
    match policy {
        Policy::FixedSplitIntervalOnly | Policy::FixedSplitTrustGated => fixed_radius,
        Policy::OnlineResidualFeedbackIntervalOnly => online_radius,
        Policy::RollingQuantileIntervalOnly => rolling_radius(history, 20, fixed_radius),
        Policy::AciClippedReferenceIntervalOnly => {
            if aci_alpha <= 1.0 / (history.len() + 1) as f64 {
                history
                    .iter()
                    .copied()
                    .reduce(f64::max)
                    .unwrap_or(fixed_radius)
            } else {
                let percentile = ((1.0 - aci_alpha.clamp(0.0, 1.0)) * 100.0)
                    .ceil()
                    .clamp(1.0, 100.0) as usize;
                quantile(history, percentile).unwrap_or(fixed_radius)
            }
        }
        Policy::CopReferenceIntervalOnly => (cop_threshold + cop_integrator).max(0.0),
        Policy::MultiHorizonIntervalOnly
        | Policy::MultiHorizonTrustGated
        | Policy::MultiHorizonUnguardedNegativeControl => {
            let short = rolling_radius(history, 8, fixed_radius);
            let long = rolling_radius(history, 32, fixed_radius);
            short.max(long)
        }
    }
}

fn update_aci_alpha(alpha: f64, covered: bool) -> f64 {
    if covered {
        alpha + ACI_LEARNING_RATE * ACI_TARGET_ALPHA
    } else {
        alpha - ACI_LEARNING_RATE * (1.0 - ACI_TARGET_ALPHA)
    }
}

fn update_cop_state(
    history: &[f64],
    cop_threshold: &mut f64,
    cop_integrator: &mut f64,
    covered: bool,
) {
    if history.is_empty() {
        return;
    }
    let start = history.len().saturating_sub(COP_WINDOW_SIZE);
    let window = &history[start..];
    let score_range = window.iter().copied().reduce(f64::max).unwrap_or(0.0)
        - window.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let learning_rate = COP_LEARNING_RATE * score_range;
    if learning_rate <= 0.0 {
        return;
    }

    // Reference-derived COP update (Hu, Wu, Xia & Zou, ICLR 2026). This harness
    // applies the update when the corresponding label matures, preserving causal
    // delayed-feedback timing. That event-time adaptation is not asserted to
    // reproduce the paper's exact finite-horizon guarantees.
    let gradient = if covered {
        ACI_TARGET_ALPHA
    } else {
        -(1.0 - ACI_TARGET_ALPHA)
    };
    let candidate = *cop_threshold - learning_rate * gradient;
    let empirical_cdf =
        window.iter().filter(|score| **score <= candidate).count() as f64 / window.len() as f64;
    let distribution_gradient = empirical_cdf - (1.0 - ACI_TARGET_ALPHA);
    *cop_threshold = candidate;
    *cop_integrator = -COP_INTEGRATOR_SCALE * learning_rate * distribution_gradient;
}

fn evaluate_stream(
    stream: &Stream,
    policy: Policy,
    calibration: &[f64],
    fixed_radius: f64,
) -> Check<(Vec<StepOutcome>, usize, usize)> {
    let uses_online_labels = !matches!(
        policy,
        Policy::FixedSplitIntervalOnly | Policy::FixedSplitTrustGated
    );
    let mut online_radius = INITIAL_RADIUS;
    let mut aci_alpha = ACI_TARGET_ALPHA;
    let mut cop_threshold = 0.0_f64;
    let mut cop_integrator = 0.0_f64;
    let mut history = calibration.to_vec();
    let mut pending_labels: VecDeque<(usize, f64, bool)> = VecDeque::new();
    let mut latched = false;
    let mut healthy_streak = 0_usize;
    let mut outcomes = Vec::with_capacity(stream.ticks.len());
    let mut label_updates_applied = 0_usize;
    let mut gate_decisions = Vec::with_capacity(stream.ticks.len());
    let mut gate_radii = Vec::with_capacity(stream.ticks.len());

    for tick in &stream.ticks {
        while uses_online_labels
            && pending_labels
                .front()
                .is_some_and(|(ready_at, _, _)| *ready_at <= tick.tick)
        {
            let (_, residual, covered_at_origin) =
                pending_labels.pop_front().expect("front item was checked");
            if policy == Policy::AciClippedReferenceIntervalOnly {
                // ACI's feedback is updated from the coverage outcome observed at
                // prediction time, not recomputed using the current interval.
                aci_alpha = update_aci_alpha(aci_alpha, covered_at_origin);
            }
            if policy == Policy::CopReferenceIntervalOnly {
                update_cop_state(
                    &history,
                    &mut cop_threshold,
                    &mut cop_integrator,
                    covered_at_origin,
                );
            }
            history.push(residual);
            if policy == Policy::OnlineResidualFeedbackIntervalOnly {
                online_radius = online_radius_update(online_radius, residual);
            }
            label_updates_applied += 1;
        }

        let radius = radius_for(
            policy,
            fixed_radius,
            online_radius,
            aci_alpha,
            cop_threshold,
            cop_integrator,
            &history,
        );
        if !radius.is_finite() || radius < 0.0 {
            return Err("interval radius became non-finite or negative".to_owned());
        }
        let width = 2.0 * radius;
        let coverage = if tick.target_label_available {
            Some((tick.target - tick.prediction).abs() <= radius)
        } else {
            None
        };
        let label_matures_at = if tick.target_label_available {
            Some(
                tick.tick
                    .checked_add(tick.target_label_delay_ticks)
                    .ok_or_else(|| "target-label maturity tick overflow".to_owned())?,
            )
        } else {
            None
        };

        let decision = if policy.is_diagnostic_only() {
            Decision::DiagnoseOnly
        } else if policy.has_trust_gate() {
            guarded_decision(tick, radius, &mut latched, &mut healthy_streak)
        } else {
            // Offline negative control only. It intentionally omits the admission
            // latch and must never be interpreted as an operating policy.
            Decision::UseModel
        };

        gate_decisions.push(decision);
        gate_radii.push(radius);
        outcomes.push(StepOutcome {
            tick: tick.tick,
            radius,
            decision,
            coverage,
            label_matures_at,
            interval_width: width,
            hard_control: is_hard_control(tick, radius),
        });

        if uses_online_labels && tick.target_label_available {
            let residual = (tick.target - tick.prediction).abs();
            let ready_at = label_matures_at.expect("available target label has a maturity tick");
            pending_labels.push_back((ready_at, residual, coverage.unwrap_or(false)));
        }
    }

    let audit_mismatches = if policy.has_trust_gate() {
        independently_replay_gate(stream, &gate_radii, &gate_decisions)
    } else {
        0
    };
    Ok((outcomes, label_updates_applied, audit_mismatches))
}

#[derive(Default)]
struct Accumulator {
    coverage_count: usize,
    coverage_denominator: usize,
    stream_coverages: Vec<f64>,
    coverage_recovery_attempts: usize,
    coverage_recovery_delays: Vec<usize>,
    post_recovery_windows_in_band: usize,
    post_recovery_windows_observed: usize,
    widths: Vec<f64>,
    model_use_steps: usize,
    diagnostic_only_steps: usize,
    abstention_steps: usize,
    unsafe_interval_rejections: usize,
    target_labels_missing: usize,
    label_updates_applied: usize,
    hard_control_acceptances: usize,
    incident_onsets: usize,
    onsets_detected: usize,
    onsets_detected_same_tick: usize,
    detection_delays: Vec<usize>,
    independent_gate_audit_mismatches: usize,
}

impl Accumulator {
    fn add_stream(
        &mut self,
        stream: &Stream,
        policy: Policy,
        outcomes: &[StepOutcome],
        label_updates_applied: usize,
        audit_mismatches: usize,
    ) {
        self.label_updates_applied += label_updates_applied;
        self.independent_gate_audit_mismatches += audit_mismatches;
        self.target_labels_missing += outcomes.iter().filter(|o| o.coverage.is_none()).count();
        let stream_coverage = outcomes
            .iter()
            .filter_map(|outcome| outcome.coverage)
            .collect::<Vec<_>>();
        if !stream_coverage.is_empty() {
            let covered = stream_coverage.iter().filter(|value| **value).count();
            self.stream_coverages
                .push(covered as f64 / stream_coverage.len() as f64);
        }
        if let Some(onset) = stream.kind.incident_start() {
            self.coverage_recovery_attempts += 1;
            if let Some(delay) = coverage_band_recovery_delay(outcomes, onset) {
                self.coverage_recovery_delays.push(delay);
            }
        }

        for outcome in outcomes {
            self.widths.push(outcome.interval_width);
            if let Some(covered) = outcome.coverage {
                self.coverage_denominator += 1;
                if covered {
                    self.coverage_count += 1;
                }
            }
            match outcome.decision {
                Decision::UseModel => self.model_use_steps += 1,
                Decision::DiagnoseOnly => self.diagnostic_only_steps += 1,
                Decision::AbstainMissingTelemetry
                | Decision::AbstainStaleTelemetry
                | Decision::AbstainRegimeNovelty
                | Decision::AbstainResidualOutOfEnvelope
                | Decision::AbstainRecoveryHoldoff => self.abstention_steps += 1,
                Decision::RejectUnsafeInterval => self.unsafe_interval_rejections += 1,
            }
            if outcome.decision == Decision::UseModel && outcome.hard_control {
                self.hard_control_acceptances += 1;
            }
        }

        if policy.has_trust_gate() {
            if let Some(onset) = stream.kind.incident_start() {
                self.incident_onsets += 1;
                let first_block = outcomes
                    .iter()
                    .find(|outcome| outcome.tick >= onset && outcome.decision != Decision::UseModel)
                    .map(|outcome| outcome.tick);
                if let Some(detected_at) = first_block {
                    self.onsets_detected += 1;
                    let delay = detected_at.saturating_sub(onset);
                    self.detection_delays.push(delay);
                    if delay == 0 {
                        self.onsets_detected_same_tick += 1;
                    }
                }
            }
        }
    }

    fn finish(self, split: Split, kind: ScenarioKind, policy: Policy) -> Summary {
        let mean_interval_width = if self.widths.is_empty() {
            0.0
        } else {
            self.widths.iter().sum::<f64>() / self.widths.len() as f64
        };
        let p95_interval_width = quantile(&self.widths, 95).unwrap_or(0.0);
        let max_interval_width = self.widths.iter().copied().reduce(f64::max).unwrap_or(0.0);
        let empirical_coverage = if self.coverage_denominator == 0 {
            None
        } else {
            Some(self.coverage_count as f64 / self.coverage_denominator as f64)
        };
        let signed_coverage_error = empirical_coverage.map(|coverage| coverage - TARGET_COVERAGE);
        let absolute_coverage_error = signed_coverage_error.map(f64::abs);
        let mean_stream_coverage = if self.stream_coverages.is_empty() {
            None
        } else {
            Some(self.stream_coverages.iter().sum::<f64>() / self.stream_coverages.len() as f64)
        };
        let p05_stream_coverage = quantile(&self.stream_coverages, 5);
        let p95_stream_coverage = quantile(&self.stream_coverages, 95);
        let coverage_recovery_successes = self.coverage_recovery_delays.len();
        let coverage_recovery_rate = if self.coverage_recovery_attempts == 0 {
            None
        } else {
            Some(coverage_recovery_successes as f64 / self.coverage_recovery_attempts as f64)
        };
        let coverage_recovery_delay_mean_ticks = if self.coverage_recovery_delays.is_empty() {
            None
        } else {
            Some(
                self.coverage_recovery_delays.iter().sum::<usize>() as f64
                    / self.coverage_recovery_delays.len() as f64,
            )
        };
        let recovery_delays_f64 = self
            .coverage_recovery_delays
            .iter()
            .map(|delay| *delay as f64)
            .collect::<Vec<_>>();
        let coverage_recovery_delay_p95_ticks = quantile(&recovery_delays_f64, 95);
        let coverage_recovery_delay_max_ticks = self.coverage_recovery_delays.iter().copied().max();
        let detection_delay_mean_ticks = if self.detection_delays.is_empty() {
            None
        } else {
            Some(
                self.detection_delays.iter().sum::<usize>() as f64
                    / self.detection_delays.len() as f64,
            )
        };
        let detection_delay_max_ticks = self.detection_delays.iter().copied().max();

        Summary {
            split,
            kind,
            policy,
            stream_count: VALIDATION_SEEDS,
            tick_count: VALIDATION_SEEDS * TICKS_PER_STREAM,
            coverage_count: self.coverage_count,
            coverage_denominator: self.coverage_denominator,
            empirical_coverage,
            signed_coverage_error,
            absolute_coverage_error,
            stream_coverage_count: self.stream_coverages.len(),
            mean_stream_coverage,
            p05_stream_coverage,
            p95_stream_coverage,
            coverage_recovery_attempts: self.coverage_recovery_attempts,
            coverage_recovery_successes,
            coverage_recovery_rate,
            coverage_recovery_delay_mean_ticks,
            coverage_recovery_delay_p95_ticks,
            coverage_recovery_delay_max_ticks,
            mean_interval_width,
            p95_interval_width,
            max_interval_width,
            model_use_steps: self.model_use_steps,
            diagnostic_only_steps: self.diagnostic_only_steps,
            abstention_steps: self.abstention_steps,
            unsafe_interval_rejections: self.unsafe_interval_rejections,
            target_labels_missing: self.target_labels_missing,
            label_updates_applied: self.label_updates_applied,
            hard_control_acceptances: self.hard_control_acceptances,
            incident_onsets: self.incident_onsets,
            onsets_detected: self.onsets_detected,
            onsets_detected_same_tick: self.onsets_detected_same_tick,
            detection_delay_mean_ticks,
            detection_delay_max_ticks,
            independent_gate_audit_mismatches: self.independent_gate_audit_mismatches,
        }
    }
}

fn make_receipt(source_commit: &str) -> Check<Receipt> {
    if !is_lower_hex(source_commit, 40) {
        return Err("source commit must be exactly 40 lowercase hexadecimal characters".to_owned());
    }

    let calibration = calibration_residuals();
    let fixed_radius = fixed_split_radius(&calibration, 100 - NOMINAL_COVERAGE_PERCENT)?;
    let calibration_bytes = serde_json::to_vec(&calibration)
        .map_err(|error| format!("cannot serialize calibration set: {error}"))?;
    let streams = generate_streams();
    let stream_bytes = serde_json::to_vec(&streams)
        .map_err(|error| format!("cannot serialize generated streams: {error}"))?;
    let calibration_set_sha256 = sha256(&calibration_bytes);
    let generated_stream_set_sha256 = sha256(&stream_bytes);
    let expected_stream_count = (VALIDATION_SEEDS + HELD_OUT_SEEDS) * STREAM_KIND_COUNT;
    let expected_tick_count = expected_stream_count * TICKS_PER_STREAM;

    if streams.len() != expected_stream_count
        || streams
            .iter()
            .any(|stream| stream.ticks.len() != TICKS_PER_STREAM)
        || streams
            .iter()
            .filter(|stream| stream.split == Split::Validation)
            .count()
            != VALIDATION_SEEDS * STREAM_KIND_COUNT
        || streams
            .iter()
            .filter(|stream| stream.split == Split::HeldOut)
            .count()
            != HELD_OUT_SEEDS * STREAM_KIND_COUNT
    {
        return Err(
            "generated stream corpus does not match the declared split contract".to_owned(),
        );
    }

    let mut accumulators: BTreeMap<(Split, ScenarioKind, Policy), Accumulator> = BTreeMap::new();
    let mut guarded_hard_control_acceptances = 0_usize;
    let mut unguarded_negative_control_hard_control_acceptances = 0_usize;
    let mut independent_gate_audit_mismatches = 0_usize;

    for stream in &streams {
        if stream
            .ticks
            .iter()
            .enumerate()
            .any(|(index, tick)| tick.tick != index)
        {
            return Err(format!(
                "{} {:?}: tick sequence is not contiguous",
                stream.seed, stream.kind
            ));
        }
        for policy in Policy::all() {
            let (outcomes, label_updates, audit_mismatches) =
                evaluate_stream(stream, policy, &calibration, fixed_radius)?;
            let key = (stream.split, stream.kind, policy);
            let accumulator = accumulators.entry(key).or_default();
            accumulator.add_stream(stream, policy, &outcomes, label_updates, audit_mismatches);

            let hard_accepts = outcomes
                .iter()
                .filter(|outcome| outcome.decision == Decision::UseModel && outcome.hard_control)
                .count();
            if policy.has_trust_gate() {
                guarded_hard_control_acceptances += hard_accepts;
                independent_gate_audit_mismatches += audit_mismatches;
            } else if policy == Policy::MultiHorizonUnguardedNegativeControl {
                unguarded_negative_control_hard_control_acceptances += hard_accepts;
            }
        }
    }

    let summaries = accumulators
        .into_iter()
        .map(|((split, kind, policy), accumulator)| accumulator.finish(split, kind, policy))
        .collect::<Vec<_>>();
    let expected_summary_count = 2 * STREAM_KIND_COUNT * Policy::all().len();
    if summaries.len() != expected_summary_count {
        return Err(format!(
            "expected {expected_summary_count} summary cells, got {}",
            summaries.len()
        ));
    }

    let per_cell_stream_count = VALIDATION_SEEDS;
    let per_cell_tick_count = per_cell_stream_count * TICKS_PER_STREAM;
    if summaries.iter().any(|summary| {
        summary.stream_count != per_cell_stream_count
            || summary.tick_count != per_cell_tick_count
            || summary.coverage_count > summary.coverage_denominator
            || !summary.mean_interval_width.is_finite()
            || !summary.p95_interval_width.is_finite()
            || !summary.max_interval_width.is_finite()
            || summary.stream_coverage_count != per_cell_stream_count
            || summary.coverage_recovery_successes > summary.coverage_recovery_attempts
            || summary.coverage_recovery_attempts > per_cell_stream_count
            || summary
                .coverage_recovery_rate
                .is_some_and(|rate| !rate.is_finite() || !(0.0..=1.0).contains(&rate))
            || (summary.coverage_recovery_attempts == 0 && summary.coverage_recovery_rate.is_some())
            || summary
                .absolute_coverage_error
                .is_some_and(|error| !error.is_finite() || !(0.0..=1.0).contains(&error))
            || summary
                .mean_stream_coverage
                .is_some_and(|coverage| !coverage.is_finite() || !(0.0..=1.0).contains(&coverage))
            || summary
                .p05_stream_coverage
                .is_some_and(|coverage| !coverage.is_finite() || !(0.0..=1.0).contains(&coverage))
            || summary
                .p95_stream_coverage
                .is_some_and(|coverage| !coverage.is_finite() || !(0.0..=1.0).contains(&coverage))
    }) {
        return Err("summary dimensions or numeric metrics are invalid".to_owned());
    }

    if guarded_hard_control_acceptances != 0 || independent_gate_audit_mismatches != 0 {
        return Err(format!(
            "guard contract failed: {guarded_hard_control_acceptances} hard-control acceptances, {independent_gate_audit_mismatches} independent gate mismatches"
        ));
    }
    if unguarded_negative_control_hard_control_acceptances == 0 {
        return Err("negative control failed to expose any hard-control admission".to_owned());
    }

    Ok(Receipt {
        result: "validated".to_owned(),
        suite_id: SUITE_ID.to_owned(),
        suite_version: SUITE_VERSION.to_owned(),
        generator_version: GENERATOR_VERSION.to_owned(),
        source_commit: source_commit.to_owned(),
        calibration_set_sha256,
        generated_stream_set_sha256,
        validation_seed_count: VALIDATION_SEEDS,
        held_out_seed_count: HELD_OUT_SEEDS,
        scenario_kind_count: STREAM_KIND_COUNT,
        ticks_per_stream: TICKS_PER_STREAM,
        stream_count: streams.len(),
        tick_count: expected_tick_count,
        policy_count: Policy::all().len(),
        summary_count: summaries.len(),
        nominal_coverage_percent: NOMINAL_COVERAGE_PERCENT,
        coverage_band_lower: COVERAGE_BAND_LOWER,
        coverage_band_upper: COVERAGE_BAND_UPPER,
        coverage_window_ticks: COVERAGE_WINDOW_TICKS,
        min_labels_per_coverage_window: MIN_LABELS_PER_COVERAGE_WINDOW,
        coverage_band_sustained_windows: COVERAGE_BAND_SUSTAINED_WINDOWS,
        trust_residual_limit: TRUST_RESIDUAL_LIMIT,
        max_telemetry_age_periods: MAX_TELEMETRY_AGE,
        healthy_ticks_required_for_reentry: HEALTHY_TICKS_TO_REENTER,
        guarded_hard_control_acceptances,
        unguarded_negative_control_hard_control_acceptances,
        independent_gate_audit_mismatches,
        summaries,
        claim_ceiling: "deterministic synthetic interval-adaptation and independent trust-gate benchmark; proxy policies are not faithful reproductions of COP, PACE, SAOCP, or another published method; no distribution-free coverage, mine-site calibration, field performance, or safety case is claimed".to_owned(),
    })
}

fn run() -> Check<ExitCode> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err("usage: mine-adaptive-interval-bench --source-commit <40-hex-sha>".to_owned());
    }
    let receipt = make_receipt(&args[1])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize adaptive-interval receipt: {error}"))?
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
    fn finite_sample_quantile_fails_closed_on_undersized_calibration() {
        let calibration = calibration_residuals();
        assert_eq!(
            3.0,
            fixed_split_radius(&calibration, 100 - NOMINAL_COVERAGE_PERCENT).expect("fixed q90")
        );
        assert!(fixed_split_radius(&[], 5).is_err());
        assert!(fixed_split_radius(&calibration, 0).is_err());
        assert!(fixed_split_radius(&calibration, 100).is_err());
        assert!(fixed_split_radius(&[1.0, 2.0], 1).is_err());
    }

    #[test]
    fn deterministic_corpus_has_disjoint_seed_splits_and_exact_dimensions() {
        let streams = generate_streams();
        assert_eq!(768, streams.len());
        assert_eq!(
            49_152,
            streams
                .iter()
                .map(|stream| stream.ticks.len())
                .sum::<usize>()
        );
        assert_eq!(
            384,
            streams
                .iter()
                .filter(|stream| stream.split == Split::Validation)
                .count()
        );
        assert_eq!(
            384,
            streams
                .iter()
                .filter(|stream| stream.split == Split::HeldOut)
                .count()
        );
        assert!(streams
            .iter()
            .all(|stream| stream.ticks.len() == TICKS_PER_STREAM));
        let first = serde_json::to_vec(&streams).expect("serialize first corpus");
        let second = serde_json::to_vec(&generate_streams()).expect("serialize second corpus");
        assert_eq!(sha256(&first), sha256(&second));
    }

    #[test]
    fn trust_gate_is_independent_of_interval_widening_and_has_reentry_holdoff() {
        let mut tick =
            generate_stream(1, Split::Validation, ScenarioKind::AbruptShift).ticks[20].clone();
        let mut locked = false;
        let mut healthy = 0;
        assert_eq!(
            Decision::AbstainResidualOutOfEnvelope,
            guarded_decision(&tick, 99.0, &mut locked, &mut healthy)
        );
        assert!(locked);

        tick.monitor_residual_abs = Some(0.0);
        tick.regime_novelty = false;
        for _ in 0..HEALTHY_TICKS_TO_REENTER {
            assert_eq!(
                Decision::AbstainRecoveryHoldoff,
                guarded_decision(&tick, 99.0, &mut locked, &mut healthy)
            );
        }
        // The interval remains unsafe even after trust recovers: it is rejected,
        // rather than made admissible by a wider radius or an expired latch.
        assert_eq!(
            Decision::RejectUnsafeInterval,
            guarded_decision(&tick, 99.0, &mut locked, &mut healthy)
        );
    }

    #[test]
    fn adaptive_interval_proxies_trade_width_for_coverage_without_authorizing_themselves() {
        let streams = generate_streams();
        let stream = streams
            .iter()
            .find(|stream| stream.seed == 10_001 && stream.kind == ScenarioKind::AbruptShift)
            .expect("held-out abrupt-shift stream");
        let calibration = calibration_residuals();
        let fixed_radius =
            fixed_split_radius(&calibration, 100 - NOMINAL_COVERAGE_PERCENT).expect("fixed radius");
        let (fixed, _, _) = evaluate_stream(
            stream,
            Policy::FixedSplitIntervalOnly,
            &calibration,
            fixed_radius,
        )
        .expect("evaluate fixed diagnostic");
        let (adaptive, _, _) = evaluate_stream(
            stream,
            Policy::OnlineResidualFeedbackIntervalOnly,
            &calibration,
            fixed_radius,
        )
        .expect("evaluate adaptive diagnostic");
        assert!(fixed
            .iter()
            .all(|step| step.decision == Decision::DiagnoseOnly));
        assert!(adaptive
            .iter()
            .all(|step| step.decision == Decision::DiagnoseOnly));
        assert!(adaptive[30].radius > fixed[30].radius);
    }

    #[test]
    fn reference_aci_and_cop_adapters_are_diagnostic_and_respect_label_maturity() {
        let stream = generate_stream(10_001, Split::HeldOut, ScenarioKind::DelayedLabels);
        let calibration = calibration_residuals();
        let fixed_radius =
            fixed_split_radius(&calibration, 100 - NOMINAL_COVERAGE_PERCENT).expect("fixed radius");

        for policy in [
            Policy::AciClippedReferenceIntervalOnly,
            Policy::CopReferenceIntervalOnly,
        ] {
            let (outcomes, label_updates, mismatches) =
                evaluate_stream(&stream, policy, &calibration, fixed_radius)
                    .expect("reference-derived policy evaluates");
            assert!(outcomes
                .iter()
                .all(|step| step.decision == Decision::DiagnoseOnly));
            assert_eq!(59, label_updates);
            assert_eq!(0, mismatches);
            assert!(outcomes
                .iter()
                .all(|step| step.radius.is_finite() && step.radius >= 0.0));
            assert!(outcomes[1].radius > 0.0);
        }
    }

    #[test]
    fn reference_adapters_match_hand_calculated_update_steps() {
        let covered_alpha = update_aci_alpha(0.1, true);
        let missed_alpha = update_aci_alpha(0.1, false);
        assert!((covered_alpha - 0.11).abs() < 1e-12);
        assert!((missed_alpha - 0.01).abs() < 1e-12);

        let small_history = [1.0, 2.0, 3.0, 4.0, 5.0];
        let fixed_radius = 3.0;
        assert_eq!(
            5.0,
            radius_for(
                Policy::AciClippedReferenceIntervalOnly,
                fixed_radius,
                fixed_radius,
                0.1,
                0.0,
                0.0,
                &small_history,
            )
        );
        assert_eq!(
            4.0,
            radius_for(
                Policy::AciClippedReferenceIntervalOnly,
                fixed_radius,
                fixed_radius,
                0.2,
                0.0,
                0.0,
                &small_history,
            )
        );

        let calibration = calibration_residuals();
        let mut cop_threshold = 0.0;
        let mut cop_integrator = 0.0;
        update_cop_state(&calibration, &mut cop_threshold, &mut cop_integrator, false);
        assert!((cop_threshold - 0.27).abs() < 1e-12);
        assert!((cop_integrator - 0.0975).abs() < 1e-12);
    }

    #[test]
    fn coverage_recovery_requires_sustained_band_and_mature_labels() {
        let make_outcome = |tick: usize, delay: usize| StepOutcome {
            tick,
            radius: 1.0,
            decision: Decision::DiagnoseOnly,
            coverage: Some(tick != 0 && tick != 8),
            label_matures_at: Some(tick + delay),
            interval_width: 2.0,
            hard_control: false,
        };
        let immediate = (0..10)
            .map(|tick| make_outcome(tick, 0))
            .collect::<Vec<_>>();
        assert_eq!(Some(9), coverage_band_recovery_delay(&immediate, 0));

        let delayed = (0..15)
            .map(|tick| make_outcome(tick, 5))
            .collect::<Vec<_>>();
        assert_eq!(Some(14), coverage_band_recovery_delay(&delayed, 0));
    }

    #[test]
    fn trust_gated_policies_reject_hard_controls_and_negative_control_exposes_them() {
        let receipt =
            make_receipt("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").expect("benchmark receipt");
        assert_eq!("validated", receipt.result);
        assert_eq!(768, receipt.stream_count);
        assert_eq!(49_152, receipt.tick_count);
        assert_eq!(9, receipt.policy_count);
        assert_eq!(216, receipt.summary_count);
        assert_eq!(0, receipt.guarded_hard_control_acceptances);
        assert_eq!(0, receipt.independent_gate_audit_mismatches);
        assert!(receipt.unguarded_negative_control_hard_control_acceptances > 0);
        assert_eq!(32, receipt.validation_seed_count);
        assert_eq!(32, receipt.held_out_seed_count);
    }

    #[test]
    fn receipt_rejects_malformed_source_identity() {
        assert!(make_receipt("short").is_err());
        assert!(make_receipt(&"A".repeat(40)).is_err());
    }
}
