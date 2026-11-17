//! Synthetic known-answer benchmark for history-dependent tactile force response.
//!
//! The forward mapping is intentionally multi-valued in instantaneous signal
//! space: loading and unloading branches can produce the same raw response at
//! different physical forces. A causal history descriptor makes the synthetic
//! inverse single-valued for this declared model. Missing history must abstain.
//!
//! This is a small hysteresis identifiability fixture, not a calibrated model
//! of any commercial sensor or elastomer. Its purpose is to test whether
//! single-frame force inversion is being treated as an unjustified assumption.

#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum LoadingDirection {
    Loading,
    Unloading,
}

impl LoadingDirection {
    fn sign(self) -> f64 {
        match self {
            Self::Loading => 1.0,
            Self::Unloading => -1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum HysteresisPhase {
    Loading,
    Unloading,
    Holding,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CausalHistoryState {
    pub phase: HysteresisPhase,
    pub last_nonzero_direction: Option<LoadingDirection>,
    /// Normalized dwell/relaxation state in [0, 1].
    pub dwell_fraction: f64,
    pub history_trace_digest: String,
    pub previous_event_identity: String,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct SyntheticHysteresisModel {
    pub gain_response_units_per_n: f64,
    pub branch_offset_response_units: f64,
    pub dwell_offset_response_units: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum HistoryInverseDisposition {
    RecoveredWithQualifiedHistory,
    AbstainHistoryUnresolved,
    InvalidModelOrObservation,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct HistoryInverseReport {
    pub disposition: HistoryInverseDisposition,
    pub estimated_force_n: Option<f64>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct HysteresisCampaignReport {
    pub first_seed: u64,
    pub replicate_count: usize,
    pub paired_raw_observation_collisions: usize,
    pub mean_force_separation_for_equal_frames_n: f64,
    pub static_single_frame_rmse_n: f64,
    pub history_conditioned_rmse_n: f64,
    pub missing_history_abstentions: usize,
    pub missing_history_trials: usize,
    pub missing_history_abstention_rate: f64,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
struct PairedHistoryTrial {
    model: SyntheticHysteresisModel,
    loading_force_n: f64,
    unloading_force_n: f64,
    loading_raw_response: f64,
    unloading_raw_response: f64,
    loading_history: CausalHistoryState,
    unloading_history: CausalHistoryState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HistoryError {
    UnknownPhase,
    MissingDirection,
    MissingTrace,
    InvalidDwell,
}

fn validate_model(model: SyntheticHysteresisModel) -> Result<(), String> {
    if !model.gain_response_units_per_n.is_finite()
        || model.gain_response_units_per_n <= 0.0
        || !model.branch_offset_response_units.is_finite()
        || model.branch_offset_response_units < 0.0
        || !model.dwell_offset_response_units.is_finite()
        || model.dwell_offset_response_units < 0.0
    {
        return Err(
            "hysteresis model gain must be positive and finite; offsets finite and nonnegative"
                .to_owned(),
        );
    }
    Ok(())
}

fn direction_from_history(history: &CausalHistoryState) -> Result<LoadingDirection, HistoryError> {
    if !history.dwell_fraction.is_finite() || !(0.0..=1.0).contains(&history.dwell_fraction) {
        return Err(HistoryError::InvalidDwell);
    }
    if history.history_trace_digest.trim().is_empty()
        || history.previous_event_identity.trim().is_empty()
    {
        return Err(HistoryError::MissingTrace);
    }
    match history.phase {
        HysteresisPhase::Loading => Ok(LoadingDirection::Loading),
        HysteresisPhase::Unloading => Ok(LoadingDirection::Unloading),
        HysteresisPhase::Holding => history
            .last_nonzero_direction
            .ok_or(HistoryError::MissingDirection),
        HysteresisPhase::Unknown => Err(HistoryError::UnknownPhase),
    }
}

/// Synthetic stateful forward model. The response offset depends on loading
/// branch and dwell; the same instantaneous sensor response can map to
/// different forces on different histories.
pub fn forward_response(
    model: SyntheticHysteresisModel,
    force_n: f64,
    history: &CausalHistoryState,
) -> Result<f64, String> {
    validate_model(model)?;
    if !force_n.is_finite() || force_n < 0.0 {
        return Err("synthetic normal force must be finite and nonnegative".to_owned());
    }
    let direction = direction_from_history(history)
        .map_err(|error| format!("history state is invalid: {error:?}"))?;
    Ok(model.gain_response_units_per_n * force_n
        + model.branch_offset_response_units * direction.sign()
        + model.dwell_offset_response_units * history.dwell_fraction * direction.sign())
}

/// Static inverse baseline that deliberately ignores branch and dwell state.
pub fn static_single_frame_inverse(
    model: SyntheticHysteresisModel,
    raw_response: f64,
) -> Result<f64, String> {
    validate_model(model)?;
    if !raw_response.is_finite() {
        return Err("raw response must be finite".to_owned());
    }
    Ok(raw_response / model.gain_response_units_per_n)
}

/// History-conditioned inverse for the declared synthetic model. Unknown or
/// incomplete history returns an explicit abstention, never a static fallback.
pub fn history_conditioned_inverse(
    model: SyntheticHysteresisModel,
    raw_response: f64,
    history: &CausalHistoryState,
) -> HistoryInverseReport {
    let invalid = |reason: String| HistoryInverseReport {
        disposition: HistoryInverseDisposition::InvalidModelOrObservation,
        estimated_force_n: None,
        reason,
    };
    if let Err(error) = validate_model(model) {
        return invalid(error);
    }
    if !raw_response.is_finite() {
        return invalid("raw response must be finite".to_owned());
    }
    let direction = match direction_from_history(history) {
        Ok(value) => value,
        Err(error) => {
            return HistoryInverseReport {
                disposition: HistoryInverseDisposition::AbstainHistoryUnresolved,
                estimated_force_n: None,
                reason: format!("causal loading/dwell history is unresolved: {error:?}"),
            };
        }
    };
    let correction = model.branch_offset_response_units * direction.sign()
        + model.dwell_offset_response_units * history.dwell_fraction * direction.sign();
    let estimate = (raw_response - correction) / model.gain_response_units_per_n;
    if !estimate.is_finite() || estimate < 0.0 {
        return invalid("history-conditioned inverse produced an invalid force".to_owned());
    }
    HistoryInverseReport {
        disposition: HistoryInverseDisposition::RecoveredWithQualifiedHistory,
        estimated_force_n: Some(estimate),
        reason: "inverse uses the exact declared causal branch and dwell state".to_owned(),
    }
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

fn random_unit(state: &mut u64) -> f64 {
    (next_random(state) >> 11) as f64 / ((1_u64 << 53) as f64)
}

fn history(
    phase: HysteresisPhase,
    direction: Option<LoadingDirection>,
    dwell_fraction: f64,
    suffix: &str,
) -> CausalHistoryState {
    CausalHistoryState {
        phase,
        last_nonzero_direction: direction,
        dwell_fraction,
        history_trace_digest: format!("sha256:history-{suffix}"),
        previous_event_identity: format!("event:previous-{suffix}"),
    }
}

fn make_paired_trial(seed: u64) -> Result<PairedHistoryTrial, String> {
    let mut state = seed.max(1);
    let model = SyntheticHysteresisModel {
        gain_response_units_per_n: 0.05 + 0.10 * random_unit(&mut state),
        branch_offset_response_units: 0.2 + 0.6 * random_unit(&mut state),
        dwell_offset_response_units: 0.01 + 0.12 * random_unit(&mut state),
    };
    let loading_force_n = 5.0 + 5.0 * random_unit(&mut state);
    let loading_dwell = random_unit(&mut state);
    let unloading_dwell = random_unit(&mut state);
    let loading_history = history(
        HysteresisPhase::Loading,
        Some(LoadingDirection::Loading),
        loading_dwell,
        &format!("{seed}-load"),
    );
    let unloading_history = history(
        HysteresisPhase::Unloading,
        Some(LoadingDirection::Unloading),
        unloading_dwell,
        &format!("{seed}-unload"),
    );
    let loading_raw_response = forward_response(model, loading_force_n, &loading_history)?;
    // Solve exactly for a different unloading force with the same raw
    // instantaneous response under the opposite branch/dwell state.
    let unloading_force_n = (loading_raw_response
        + model.branch_offset_response_units
        + model.dwell_offset_response_units * unloading_dwell)
        / model.gain_response_units_per_n;
    let unloading_raw_response = forward_response(model, unloading_force_n, &unloading_history)?;
    Ok(PairedHistoryTrial {
        model,
        loading_force_n,
        unloading_force_n,
        loading_raw_response,
        unloading_raw_response,
        loading_history,
        unloading_history,
    })
}

/// Run paired deterministic trials where loading and unloading samples have
/// exactly the same raw response but different true forces. The evaluator truth
/// is kept outside the inverse functions.
pub fn run_hysteresis_identifiability_campaign(
    first_seed: u64,
    replicate_count: usize,
) -> Result<HysteresisCampaignReport, String> {
    if first_seed == 0 {
        return Err("hysteresis campaign seed must be nonzero".to_owned());
    }
    if replicate_count < 50 {
        return Err("hysteresis campaign requires at least fifty paired trials".to_owned());
    }
    let offset = u64::try_from(replicate_count - 1)
        .map_err(|_| "hysteresis replicate count exceeds u64".to_owned())?;
    first_seed
        .checked_add(offset)
        .ok_or_else(|| "hysteresis seed range overflows u64".to_owned())?;
    let evaluated_force_count = replicate_count
        .checked_mul(2)
        .ok_or_else(|| "hysteresis evaluator sample count overflows usize".to_owned())?;
    let evaluated_force_count_f64 = evaluated_force_count as f64;

    let mut static_squared_error = 0.0;
    let mut history_squared_error = 0.0;
    let mut force_separation = 0.0;
    let mut collisions = 0;
    let mut missing_history_abstentions = 0;
    let mut missing_history_trials = 0;

    for replicate in 0..replicate_count {
        let trial = make_paired_trial(first_seed + replicate as u64)?;
        if (trial.loading_raw_response - trial.unloading_raw_response).abs() > 1.0e-10 {
            return Err(
                "known-answer hysteresis pair failed to collide in raw observation space"
                    .to_owned(),
            );
        }
        if (trial.loading_force_n - trial.unloading_force_n).abs() <= 1.0e-8 {
            return Err(
                "known-answer hysteresis pair did not preserve distinct force labels".to_owned(),
            );
        }
        collisions += 1;
        force_separation += (trial.unloading_force_n - trial.loading_force_n).abs();

        let static_loading = static_single_frame_inverse(trial.model, trial.loading_raw_response)?;
        let static_unloading =
            static_single_frame_inverse(trial.model, trial.unloading_raw_response)?;
        static_squared_error += (static_loading - trial.loading_force_n).powi(2);
        static_squared_error += (static_unloading - trial.unloading_force_n).powi(2);

        let history_loading = history_conditioned_inverse(
            trial.model,
            trial.loading_raw_response,
            &trial.loading_history,
        );
        let history_unloading = history_conditioned_inverse(
            trial.model,
            trial.unloading_raw_response,
            &trial.unloading_history,
        );
        for (report, truth) in [
            (history_loading, trial.loading_force_n),
            (history_unloading, trial.unloading_force_n),
        ] {
            if report.disposition != HistoryInverseDisposition::RecoveredWithQualifiedHistory {
                return Err(
                    "known causal history was not admitted by history-conditioned inverse"
                        .to_owned(),
                );
            }
            let estimate = report
                .estimated_force_n
                .ok_or_else(|| "qualified history inverse omitted force estimate".to_owned())?;
            history_squared_error += (estimate - truth).powi(2);
        }

        let missing = history(
            HysteresisPhase::Unknown,
            None,
            0.0,
            &format!("{first_seed}-{replicate}-missing"),
        );
        for raw in [trial.loading_raw_response, trial.unloading_raw_response] {
            let report = history_conditioned_inverse(trial.model, raw, &missing);
            if report.disposition == HistoryInverseDisposition::AbstainHistoryUnresolved
                && report.estimated_force_n.is_none()
            {
                missing_history_abstentions += 1;
            }
            missing_history_trials += 1;
        }
    }

    Ok(HysteresisCampaignReport {
        first_seed,
        replicate_count,
        paired_raw_observation_collisions: collisions,
        mean_force_separation_for_equal_frames_n: force_separation / replicate_count as f64,
        static_single_frame_rmse_n: (static_squared_error / evaluated_force_count_f64).sqrt(),
        history_conditioned_rmse_n: (history_squared_error / evaluated_force_count_f64).sqrt(),
        missing_history_abstentions,
        missing_history_trials,
        missing_history_abstention_rate: missing_history_abstentions as f64
            / missing_history_trials as f64,
        claim_class: "synthetic_hysteresis_identifiability_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> SyntheticHysteresisModel {
        SyntheticHysteresisModel {
            gain_response_units_per_n: 0.1,
            branch_offset_response_units: 0.4,
            dwell_offset_response_units: 0.05,
        }
    }

    #[test]
    fn identical_instantaneous_response_can_encode_two_different_forces() {
        let model = model();
        let loading = history(
            HysteresisPhase::Loading,
            Some(LoadingDirection::Loading),
            0.2,
            "load",
        );
        let unloading = history(
            HysteresisPhase::Unloading,
            Some(LoadingDirection::Unloading),
            0.6,
            "unload",
        );
        let loading_force = 10.0;
        let raw = forward_response(model, loading_force, &loading).unwrap();
        let unloading_force = (raw
            + model.branch_offset_response_units
            + model.dwell_offset_response_units * unloading.dwell_fraction)
            / model.gain_response_units_per_n;
        let unloading_raw = forward_response(model, unloading_force, &unloading).unwrap();
        assert!((raw - unloading_raw).abs() < 1.0e-12);
        assert!((loading_force - unloading_force).abs() > 1.0);
        assert_eq!(
            static_single_frame_inverse(model, raw).unwrap(),
            static_single_frame_inverse(model, unloading_raw).unwrap()
        );
    }

    #[test]
    fn causal_history_resolves_the_force_ambiguity() {
        let model = model();
        let loading = history(
            HysteresisPhase::Loading,
            Some(LoadingDirection::Loading),
            0.2,
            "load",
        );
        let unloading = history(
            HysteresisPhase::Unloading,
            Some(LoadingDirection::Unloading),
            0.6,
            "unload",
        );
        let force_load = 10.0;
        let raw = forward_response(model, force_load, &loading).unwrap();
        let force_unload = (raw
            + model.branch_offset_response_units
            + model.dwell_offset_response_units * unloading.dwell_fraction)
            / model.gain_response_units_per_n;
        let report_load = history_conditioned_inverse(model, raw, &loading);
        let report_unload = history_conditioned_inverse(model, raw, &unloading);
        assert!((report_load.estimated_force_n.unwrap() - force_load).abs() < 1.0e-10);
        assert!((report_unload.estimated_force_n.unwrap() - force_unload).abs() < 1.0e-10);
    }

    #[test]
    fn missing_or_invalid_history_abstains_instead_of_using_static_fallback() {
        let unknown = history(HysteresisPhase::Unknown, None, 0.5, "unknown");
        let report = history_conditioned_inverse(model(), 1.4, &unknown);
        assert_eq!(
            report.disposition,
            HistoryInverseDisposition::AbstainHistoryUnresolved
        );
        assert!(report.estimated_force_n.is_none());

        let missing_direction = history(HysteresisPhase::Holding, None, 0.5, "hold");
        let report = history_conditioned_inverse(model(), 1.4, &missing_direction);
        assert_eq!(
            report.disposition,
            HistoryInverseDisposition::AbstainHistoryUnresolved
        );

        let bad_dwell = history(
            HysteresisPhase::Loading,
            Some(LoadingDirection::Loading),
            1.5,
            "bad-dwell",
        );
        let report = history_conditioned_inverse(model(), 1.4, &bad_dwell);
        assert_eq!(
            report.disposition,
            HistoryInverseDisposition::AbstainHistoryUnresolved
        );
    }

    #[test]
    fn paired_fifty_seed_campaign_quantifies_single_frame_error_and_history_value() {
        let report = run_hysteresis_identifiability_campaign(20261117, 50).unwrap();
        assert_eq!(report.replicate_count, 50);
        assert_eq!(report.paired_raw_observation_collisions, 50);
        assert!(report.mean_force_separation_for_equal_frames_n > 1.0);
        assert!(report.static_single_frame_rmse_n > 0.1);
        assert!(report.history_conditioned_rmse_n < 1.0e-10);
        assert_eq!(report.missing_history_abstention_rate, 1.0);
    }

    #[test]
    fn campaign_rejects_weak_or_overflowing_seed_manifests() {
        assert!(run_hysteresis_identifiability_campaign(1, 49).is_err());
        assert!(run_hysteresis_identifiability_campaign(0, 50).is_err());
        assert!(run_hysteresis_identifiability_campaign(u64::MAX, 50).is_err());
    }
}
