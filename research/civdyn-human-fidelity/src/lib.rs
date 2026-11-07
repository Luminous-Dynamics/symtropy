// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Human-behaviour fidelity decomposition.
//!
//! R1-style individual response scoring and R2-style population-distribution
//! scoring remain separate. Ordered contexts also receive a trajectory
//! transition check. Synthetic examples are software controls, not human data.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

const PROBABILITY_TOLERANCE: f64 = 1e-9;
const LOG_FLOOR: f64 = 1e-15;
const TAIL_FREQUENCY_MAX: f64 = 0.10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedTrial {
    /// Opaque, source-issued participant key. Never place direct identifiers here.
    pub participant_id: String,
    pub context_id: String,
    /// Declared cohort/slice label; never inferred from free text.
    pub cohort_id: String,
    pub step_index: u32,
    pub choice: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimulatedTrial {
    pub participant_id: String,
    pub context_id: String,
    pub cohort_id: String,
    pub step_index: u32,
    /// Predictive choice distribution in option-label order.
    pub choice_probabilities: Vec<f64>,
    /// One generated action from the simulated trajectory.
    pub generated_choice: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HumanFidelityDataset {
    pub option_labels: Vec<String>,
    pub observed: Vec<ObservedTrial>,
    pub simulated: Vec<SimulatedTrial>,
}

impl HumanFidelityDataset {
    pub fn validate(&self) -> Result<(), FidelityError> {
        if self.option_labels.len() < 2 {
            return Err(FidelityError::TooFewOptions(self.option_labels.len()));
        }
        let mut labels = BTreeSet::new();
        for label in &self.option_labels {
            if label.trim().is_empty() {
                return Err(FidelityError::EmptyOptionLabel);
            }
            if !labels.insert(label) {
                return Err(FidelityError::DuplicateOptionLabel(label.clone()));
            }
        }
        if self.observed.is_empty() || self.simulated.is_empty() {
            return Err(FidelityError::EmptyTrials);
        }

        let mut observed = BTreeMap::new();
        let mut simulated = BTreeMap::new();
        validate_observed(self, &mut observed)?;
        validate_simulated(self, &mut simulated)?;

        if observed.len() != simulated.len() {
            return Err(FidelityError::TrialCountMismatch {
                observed: observed.len(),
                simulated: simulated.len(),
            });
        }
        for (key, actual) in &observed {
            let Some(prediction) = simulated.get(key) else {
                return Err(FidelityError::MissingPrediction {
                    participant_id: key.0.clone(),
                    step_index: key.1,
                });
            };
            if actual.context_id != prediction.context_id
                || actual.cohort_id != prediction.cohort_id
            {
                return Err(FidelityError::MatchedTrialMetadataMismatch {
                    participant_id: key.0.clone(),
                    step_index: key.1,
                });
            }
        }
        for key in simulated.keys() {
            if !observed.contains_key(key) {
                return Err(FidelityError::UnmatchedPrediction {
                    participant_id: key.0.clone(),
                    step_index: key.1,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FidelityError {
    TooFewOptions(usize),
    EmptyOptionLabel,
    DuplicateOptionLabel(String),
    EmptyTrials,
    EmptyIdentifier {
        side: &'static str,
        field: &'static str,
        index: usize,
    },
    DuplicateParticipantStep {
        side: &'static str,
        participant_id: String,
        step_index: u32,
    },
    InconsistentParticipantCohort {
        side: &'static str,
        participant_id: String,
    },
    InvalidObservedChoice {
        participant_id: String,
        step_index: u32,
        choice: usize,
        option_count: usize,
    },
    ProbabilityLengthMismatch {
        participant_id: String,
        step_index: u32,
        expected: usize,
        actual: usize,
    },
    InvalidProbability {
        participant_id: String,
        step_index: u32,
        option: usize,
        value: f64,
    },
    ProbabilityMassMismatch {
        participant_id: String,
        step_index: u32,
        sum: f64,
    },
    InvalidGeneratedChoice {
        participant_id: String,
        step_index: u32,
        choice: usize,
        option_count: usize,
    },
    TrialCountMismatch {
        observed: usize,
        simulated: usize,
    },
    MissingPrediction {
        participant_id: String,
        step_index: u32,
    },
    UnmatchedPrediction {
        participant_id: String,
        step_index: u32,
    },
    MatchedTrialMetadataMismatch {
        participant_id: String,
        step_index: u32,
    },
    InvalidCoverageRequest {
        requested: usize,
        available: usize,
    },
    EmptyBootstrapSizes,
    InvalidBootstrapReplicateCount(usize),
}

impl fmt::Display for FidelityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewOptions(count) => {
                write!(f, "at least two choice options are required, got {count}")
            }
            Self::EmptyOptionLabel => write!(f, "option labels must not be empty"),
            Self::DuplicateOptionLabel(label) => write!(f, "duplicate option label: {label}"),
            Self::EmptyTrials => write!(f, "observed and simulated trial sets must be non-empty"),
            Self::EmptyIdentifier { side, field, index } => {
                write!(f, "{side} trial {index} has an empty {field}")
            }
            Self::DuplicateParticipantStep {
                side,
                participant_id,
                step_index,
            } => write!(
                f,
                "{side} has duplicate participant/step key ({participant_id}, {step_index})"
            ),
            Self::InconsistentParticipantCohort {
                side,
                participant_id,
            } => write!(
                f,
                "{side} assigns participant {participant_id} to multiple declared cohorts"
            ),
            Self::InvalidObservedChoice {
                participant_id,
                step_index,
                choice,
                option_count,
            } => write!(
                f,
                "observed choice {choice} for {participant_id} at step {step_index} is outside 0..{option_count}"
            ),
            Self::ProbabilityLengthMismatch {
                participant_id,
                step_index,
                expected,
                actual,
            } => write!(
                f,
                "prediction for {participant_id} at step {step_index} has {actual} probabilities; expected {expected}"
            ),
            Self::InvalidProbability {
                participant_id,
                step_index,
                option,
                value,
            } => write!(
                f,
                "prediction for {participant_id} at step {step_index}, option {option}, has invalid probability {value}"
            ),
            Self::ProbabilityMassMismatch {
                participant_id,
                step_index,
                sum,
            } => write!(
                f,
                "prediction probabilities for {participant_id} at step {step_index} sum to {sum}, not one"
            ),
            Self::InvalidGeneratedChoice {
                participant_id,
                step_index,
                choice,
                option_count,
            } => write!(
                f,
                "generated choice {choice} for {participant_id} at step {step_index} is outside 0..{option_count}"
            ),
            Self::TrialCountMismatch {
                observed,
                simulated,
            } => write!(
                f,
                "observed and simulated trial counts differ: {observed} vs {simulated}"
            ),
            Self::MissingPrediction {
                participant_id,
                step_index,
            } => write!(
                f,
                "missing simulated prediction for {participant_id} at step {step_index}"
            ),
            Self::UnmatchedPrediction {
                participant_id,
                step_index,
            } => write!(
                f,
                "simulated prediction has no matched observation for {participant_id} at step {step_index}"
            ),
            Self::MatchedTrialMetadataMismatch {
                participant_id,
                step_index,
            } => write!(
                f,
                "context or cohort mismatch for matched trial ({participant_id}, {step_index})"
            ),
            Self::InvalidCoverageRequest {
                requested,
                available,
            } => write!(
                f,
                "requested population coverage {requested} exceeds available participants {available}"
            ),
            Self::EmptyBootstrapSizes => {
                write!(f, "bootstrap population-size request must not be empty")
            }
            Self::InvalidBootstrapReplicateCount(count) => {
                write!(f, "bootstrap replicate count must be positive, got {count}")
            }
        }
    }
}

impl std::error::Error for FidelityError {}

#[derive(Debug, Clone, PartialEq)]
pub struct SliceMetrics {
    pub context_id: String,
    pub cohort_id: String,
    pub participant_count: usize,
    pub trial_count: usize,
    /// Per-trial multiclass Brier score; lower is better.
    pub mean_brier_score: f64,
    /// Per-trial negative log likelihood; lower is better.
    pub mean_log_loss: f64,
    /// Argmax choice accuracy against the matched observed action.
    pub top_choice_accuracy: f64,
    pub observed_choice_frequency: Vec<f64>,
    /// Frequency of generated actions, not averaged probability mass.
    pub generated_choice_frequency: Vec<f64>,
    /// Average predictive probability distribution.
    pub mean_predicted_probability: Vec<f64>,
    /// Total-variation distance between observed and generated action distributions.
    pub total_variation: f64,
    /// Jensen-Shannon divergence in bits between observed and generated action distributions.
    pub jensen_shannon_bits: f64,
    pub observed_entropy_bits: f64,
    pub generated_entropy_bits: f64,
    pub entropy_delta_bits: f64,
    pub observed_tail_option_count: usize,
    pub generated_tail_option_count: usize,
    /// Fraction of observed low-frequency options that are generated at least once.
    pub tail_option_coverage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationResult {
    /// R1-style individual scoring and R2-style population scoring are both visible.
    pub overall: SliceMetrics,
    pub slices: Vec<SliceMetrics>,
    /// TVD between empirical and generated adjacent-step transition distributions.
    pub trajectory_transition_tvd: Option<f64>,
    pub transition_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoveragePoint {
    pub participant_count: usize,
    pub trial_count: usize,
    pub mean_brier_score: f64,
    pub total_variation: f64,
    pub jensen_shannon_bits: f64,
    pub entropy_delta_bits: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapConfig {
    pub seed: u64,
    pub replicate_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapMetric {
    MeanBrierScore,
    MeanLogLoss,
    TopChoiceAccuracy,
    TotalVariation,
    JensenShannonBits,
    EntropyDeltaBits,
    TrajectoryTransitionTvd,
    TailOptionCoverage,
}

impl BootstrapMetric {
    pub const ALL: [Self; 8] = [
        Self::MeanBrierScore,
        Self::MeanLogLoss,
        Self::TopChoiceAccuracy,
        Self::TotalVariation,
        Self::JensenShannonBits,
        Self::EntropyDeltaBits,
        Self::TrajectoryTransitionTvd,
        Self::TailOptionCoverage,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::MeanBrierScore => "mean_brier_score",
            Self::MeanLogLoss => "mean_log_loss",
            Self::TopChoiceAccuracy => "top_choice_accuracy",
            Self::TotalVariation => "total_variation",
            Self::JensenShannonBits => "jensen_shannon_bits",
            Self::EntropyDeltaBits => "entropy_delta_bits",
            Self::TrajectoryTransitionTvd => "trajectory_transition_tvd",
            Self::TailOptionCoverage => "tail_option_coverage",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapMetricSummary {
    pub metric: BootstrapMetric,
    pub valid_replicates: usize,
    pub undefined_replicates: usize,
    pub mean: Option<f64>,
    pub p2_5: Option<f64>,
    pub p50: Option<f64>,
    pub p97_5: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapCoveragePoint {
    pub participant_count: usize,
    pub replicate_count: usize,
    /// Mean number of trial records retained in each resampled participant-cluster replicate.
    pub mean_trial_count_per_replicate: f64,
    pub metrics: Vec<BootstrapMetricSummary>,
}

type TrialKey = (String, u32);
type MatchedTrial<'a> = (&'a ObservedTrial, &'a SimulatedTrial);

pub fn evaluate(dataset: &HumanFidelityDataset) -> Result<EvaluationResult, FidelityError> {
    dataset.validate()?;
    let observed: BTreeMap<TrialKey, &ObservedTrial> = dataset
        .observed
        .iter()
        .map(|trial| ((trial.participant_id.clone(), trial.step_index), trial))
        .collect();
    let simulated: BTreeMap<TrialKey, &SimulatedTrial> = dataset
        .simulated
        .iter()
        .map(|trial| ((trial.participant_id.clone(), trial.step_index), trial))
        .collect();

    let mut matched = observed
        .iter()
        .map(|(key, actual)| (*actual, simulated[key]))
        .collect::<Vec<_>>();
    matched.sort_by(|left, right| {
        left.0
            .participant_id
            .cmp(&right.0.participant_id)
            .then_with(|| left.0.step_index.cmp(&right.0.step_index))
    });

    let overall = compute_slice("__all__", "__all__", &matched, dataset.option_labels.len());
    let mut groups: BTreeMap<(String, String), Vec<MatchedTrial<'_>>> = BTreeMap::new();
    for pair in &matched {
        groups
            .entry((pair.0.context_id.clone(), pair.0.cohort_id.clone()))
            .or_default()
            .push(*pair);
    }
    let slices = groups
        .into_iter()
        .map(|((context, cohort), trials)| {
            compute_slice(&context, &cohort, &trials, dataset.option_labels.len())
        })
        .collect();

    let (trajectory_transition_tvd, transition_count) =
        trajectory_transition_metrics(&matched, dataset.option_labels.len());

    Ok(EvaluationResult {
        overall,
        slices,
        trajectory_transition_tvd,
        transition_count,
    })
}

/// Deterministic prefix coverage curve over lexicographically sorted opaque IDs.
///
/// This is a reproducible diagnostic, not an inferential subsampling estimator.
/// For claims about uncertainty or saturation, repeat over predeclared seeded
/// subsamples and source-appropriate clustered intervals.
pub fn population_coverage_curve(
    dataset: &HumanFidelityDataset,
    requested_sizes: &[usize],
) -> Result<Vec<CoveragePoint>, FidelityError> {
    dataset.validate()?;
    let participants = dataset
        .observed
        .iter()
        .map(|trial| trial.participant_id.clone())
        .collect::<BTreeSet<_>>();
    let available = participants.len();
    let mut sizes = requested_sizes.to_vec();
    sizes.sort_unstable();
    sizes.dedup();

    let mut points = Vec::with_capacity(sizes.len());
    for requested in sizes {
        if requested == 0 || requested > available {
            return Err(FidelityError::InvalidCoverageRequest {
                requested,
                available,
            });
        }
        let selected = participants
            .iter()
            .take(requested)
            .cloned()
            .collect::<BTreeSet<_>>();
        let observed = dataset
            .observed
            .iter()
            .filter(|trial| selected.contains(&trial.participant_id))
            .cloned()
            .collect::<Vec<_>>();
        let simulated = dataset
            .simulated
            .iter()
            .filter(|trial| selected.contains(&trial.participant_id))
            .cloned()
            .collect::<Vec<_>>();
        let subset = HumanFidelityDataset {
            option_labels: dataset.option_labels.clone(),
            observed,
            simulated,
        };
        let result = evaluate(&subset)?;
        points.push(CoveragePoint {
            participant_count: requested,
            trial_count: result.overall.trial_count,
            mean_brier_score: result.overall.mean_brier_score,
            total_variation: result.overall.total_variation,
            jensen_shannon_bits: result.overall.jensen_shannon_bits,
            entropy_delta_bits: result.overall.entropy_delta_bits,
        });
    }
    Ok(points)
}


/// Participant-cluster bootstrap for the human-fidelity sample-coverage curve.
///
/// Each draw resamples unique participant IDs with replacement and carries every
/// observed/simulated context-step record for that participant together. These
/// intervals quantify participant-composition uncertainty conditional on the
/// supplied dataset; they are not predictive intervals or external-generalization
/// intervals. Cohort proportions are not stratified in v0.
pub fn bootstrap_population_coverage_curve(
    dataset: &HumanFidelityDataset,
    requested_sizes: &[usize],
    config: BootstrapConfig,
) -> Result<Vec<BootstrapCoveragePoint>, FidelityError> {
    dataset.validate()?;
    if config.replicate_count == 0 {
        return Err(FidelityError::InvalidBootstrapReplicateCount(0));
    }
    if requested_sizes.is_empty() {
        return Err(FidelityError::EmptyBootstrapSizes);
    }

    let participants = dataset
        .observed
        .iter()
        .map(|trial| trial.participant_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let available = participants.len();
    let mut sizes = requested_sizes.to_vec();
    sizes.sort_unstable();
    sizes.dedup();

    let mut points = Vec::with_capacity(sizes.len());
    for requested in sizes {
        if requested == 0 || requested > available {
            return Err(FidelityError::InvalidCoverageRequest {
                requested,
                available,
            });
        }

        // Give each population size an independent, reproducible random stream.
        let size_seed = config.seed
            ^ (requested as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mut rng = SplitMix64::new(size_seed);
        let mut values: [Vec<f64>; 8] =
            std::array::from_fn(|_| Vec::with_capacity(config.replicate_count));
        let mut undefined = [0usize; 8];
        let mut trial_count_total = 0usize;

        for replicate_index in 0..config.replicate_count {
            let sampled = resample_participant_clusters(
                dataset,
                &participants,
                requested,
                replicate_index,
                &mut rng,
            );
            trial_count_total += sampled.observed.len();
            let result = evaluate(&sampled)?;
            let replicate_values = bootstrap_metric_values(&result);
            for (metric_index, value) in replicate_values.into_iter().enumerate() {
                if let Some(value) = value {
                    values[metric_index].push(value);
                } else {
                    undefined[metric_index] += 1;
                }
            }
        }

        let metrics = BootstrapMetric::ALL
            .into_iter()
            .enumerate()
            .map(|(index, metric)| summarize_bootstrap_metric(
                metric,
                &values[index],
                undefined[index],
            ))
            .collect();

        points.push(BootstrapCoveragePoint {
            participant_count: requested,
            replicate_count: config.replicate_count,
            mean_trial_count_per_replicate:
                trial_count_total as f64 / config.replicate_count as f64,
            metrics,
        });
    }

    Ok(points)
}

fn bootstrap_metric_values(result: &EvaluationResult) -> [Option<f64>; 8] {
    [
        Some(result.overall.mean_brier_score),
        Some(result.overall.mean_log_loss),
        Some(result.overall.top_choice_accuracy),
        Some(result.overall.total_variation),
        Some(result.overall.jensen_shannon_bits),
        Some(result.overall.entropy_delta_bits),
        result.trajectory_transition_tvd,
        result.overall.tail_option_coverage,
    ]
}

fn summarize_bootstrap_metric(
    metric: BootstrapMetric,
    values: &[f64],
    undefined_replicates: usize,
) -> BootstrapMetricSummary {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let (mean, p2_5, p50, p97_5) = if sorted.is_empty() {
        (None, None, None, None)
    } else {
        let average = sorted.iter().sum::<f64>() / sorted.len() as f64;
        (
            Some(average),
            Some(bootstrap_percentile(&sorted, 0.025)),
            Some(bootstrap_percentile(&sorted, 0.50)),
            Some(bootstrap_percentile(&sorted, 0.975)),
        )
    };
    BootstrapMetricSummary {
        metric,
        valid_replicates: sorted.len(),
        undefined_replicates,
        mean,
        p2_5,
        p50,
        p97_5,
    }
}

fn bootstrap_percentile(sorted: &[f64], probability: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = probability * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let weight = position - lower as f64;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

fn resample_participant_clusters(
    dataset: &HumanFidelityDataset,
    participants: &[String],
    sample_size: usize,
    replicate_index: usize,
    rng: &mut SplitMix64,
) -> HumanFidelityDataset {
    let mut observed = Vec::new();
    let mut simulated = Vec::new();

    for draw_index in 0..sample_size {
        let source_id = &participants[rng.next_index(participants.len())];
        // New IDs identify bootstrap draws, not source participants. All repeated
        // contexts for one draw share this ID, and separate draws remain distinct.
        let bootstrap_id = format!("bootstrap_{replicate_index:06}_{draw_index:06}");

        for trial in dataset
            .observed
            .iter()
            .filter(|trial| trial.participant_id == *source_id)
        {
            let mut copy = trial.clone();
            copy.participant_id = bootstrap_id.clone();
            observed.push(copy);
        }
        for trial in dataset
            .simulated
            .iter()
            .filter(|trial| trial.participant_id == *source_id)
        {
            let mut copy = trial.clone();
            copy.participant_id = bootstrap_id.clone();
            simulated.push(copy);
        }
    }

    HumanFidelityDataset {
        option_labels: dataset.option_labels.clone(),
        observed,
        simulated,
    }
}

fn validate_observed<'a>(
    dataset: &'a HumanFidelityDataset,
    records: &mut BTreeMap<TrialKey, &'a ObservedTrial>,
) -> Result<(), FidelityError> {
    let mut cohort_by_participant = BTreeMap::new();
    for (index, trial) in dataset.observed.iter().enumerate() {
        validate_identifiers(
            "observed",
            index,
            &trial.participant_id,
            &trial.context_id,
            &trial.cohort_id,
        )?;
        if trial.choice >= dataset.option_labels.len() {
            return Err(FidelityError::InvalidObservedChoice {
                participant_id: trial.participant_id.clone(),
                step_index: trial.step_index,
                choice: trial.choice,
                option_count: dataset.option_labels.len(),
            });
        }
        validate_cohort(
            "observed",
            &trial.participant_id,
            &trial.cohort_id,
            &mut cohort_by_participant,
        )?;
        let key = (trial.participant_id.clone(), trial.step_index);
        if records.insert(key.clone(), trial).is_some() {
            return Err(FidelityError::DuplicateParticipantStep {
                side: "observed",
                participant_id: key.0,
                step_index: key.1,
            });
        }
    }
    Ok(())
}

fn validate_simulated<'a>(
    dataset: &'a HumanFidelityDataset,
    records: &mut BTreeMap<TrialKey, &'a SimulatedTrial>,
) -> Result<(), FidelityError> {
    let mut cohort_by_participant = BTreeMap::new();
    for (index, trial) in dataset.simulated.iter().enumerate() {
        validate_identifiers(
            "simulated",
            index,
            &trial.participant_id,
            &trial.context_id,
            &trial.cohort_id,
        )?;
        if trial.choice_probabilities.len() != dataset.option_labels.len() {
            return Err(FidelityError::ProbabilityLengthMismatch {
                participant_id: trial.participant_id.clone(),
                step_index: trial.step_index,
                expected: dataset.option_labels.len(),
                actual: trial.choice_probabilities.len(),
            });
        }
        for (option, probability) in trial.choice_probabilities.iter().enumerate() {
            if !probability.is_finite() || *probability < 0.0 {
                return Err(FidelityError::InvalidProbability {
                    participant_id: trial.participant_id.clone(),
                    step_index: trial.step_index,
                    option,
                    value: *probability,
                });
            }
        }
        let sum = trial.choice_probabilities.iter().sum::<f64>();
        if (sum - 1.0).abs() > PROBABILITY_TOLERANCE {
            return Err(FidelityError::ProbabilityMassMismatch {
                participant_id: trial.participant_id.clone(),
                step_index: trial.step_index,
                sum,
            });
        }
        if trial.generated_choice >= dataset.option_labels.len() {
            return Err(FidelityError::InvalidGeneratedChoice {
                participant_id: trial.participant_id.clone(),
                step_index: trial.step_index,
                choice: trial.generated_choice,
                option_count: dataset.option_labels.len(),
            });
        }
        validate_cohort(
            "simulated",
            &trial.participant_id,
            &trial.cohort_id,
            &mut cohort_by_participant,
        )?;
        let key = (trial.participant_id.clone(), trial.step_index);
        if records.insert(key.clone(), trial).is_some() {
            return Err(FidelityError::DuplicateParticipantStep {
                side: "simulated",
                participant_id: key.0,
                step_index: key.1,
            });
        }
    }
    Ok(())
}

fn validate_identifiers(
    side: &'static str,
    index: usize,
    participant_id: &str,
    context_id: &str,
    cohort_id: &str,
) -> Result<(), FidelityError> {
    for (field, value) in [
        ("participant_id", participant_id),
        ("context_id", context_id),
        ("cohort_id", cohort_id),
    ] {
        if value.trim().is_empty() {
            return Err(FidelityError::EmptyIdentifier { side, field, index });
        }
    }
    Ok(())
}

fn validate_cohort(
    side: &'static str,
    participant_id: &str,
    cohort_id: &str,
    cohort_by_participant: &mut BTreeMap<String, String>,
) -> Result<(), FidelityError> {
    if let Some(existing) = cohort_by_participant.get(participant_id) {
        if existing != cohort_id {
            return Err(FidelityError::InconsistentParticipantCohort {
                side,
                participant_id: participant_id.to_owned(),
            });
        }
    } else {
        cohort_by_participant.insert(participant_id.to_owned(), cohort_id.to_owned());
    }
    Ok(())
}

fn compute_slice(
    context_id: &str,
    cohort_id: &str,
    trials: &[MatchedTrial<'_>],
    option_count: usize,
) -> SliceMetrics {
    let mut observed_counts = vec![0.0; option_count];
    let mut generated_counts = vec![0.0; option_count];
    let mut probability_sums = vec![0.0; option_count];
    let mut brier_sum = 0.0;
    let mut log_loss_sum = 0.0;
    let mut top_choice_correct = 0usize;

    for (observed, simulated) in trials {
        observed_counts[observed.choice] += 1.0;
        generated_counts[simulated.generated_choice] += 1.0;
        let predicted_choice = argmax(&simulated.choice_probabilities);
        if predicted_choice == observed.choice {
            top_choice_correct += 1;
        }
        for (option, probability) in simulated.choice_probabilities.iter().enumerate() {
            probability_sums[option] += probability;
            let target = if option == observed.choice { 1.0 } else { 0.0 };
            brier_sum += (probability - target).powi(2);
        }
        log_loss_sum -= simulated.choice_probabilities[observed.choice]
            .max(LOG_FLOOR)
            .ln();
    }

    let trial_count = trials.len();
    let denom = trial_count as f64;
    normalize(&mut observed_counts);
    normalize(&mut generated_counts);
    for probability in &mut probability_sums {
        *probability /= denom;
    }

    let observed_entropy_bits = entropy_bits(&observed_counts);
    let generated_entropy_bits = entropy_bits(&generated_counts);
    let tail_options = observed_counts
        .iter()
        .enumerate()
        .filter(|(_, frequency)| **frequency > 0.0 && **frequency <= TAIL_FREQUENCY_MAX)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let generated_tail_option_count = generated_counts
        .iter()
        .filter(|frequency| **frequency > 0.0 && **frequency <= TAIL_FREQUENCY_MAX)
        .count();
    let tail_option_coverage = if tail_options.is_empty() {
        None
    } else {
        let covered = tail_options
            .iter()
            .filter(|index| generated_counts[**index] > 0.0)
            .count();
        Some(covered as f64 / tail_options.len() as f64)
    };

    SliceMetrics {
        context_id: context_id.to_owned(),
        cohort_id: cohort_id.to_owned(),
        participant_count: trials
            .iter()
            .map(|(observed, _)| observed.participant_id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        trial_count,
        mean_brier_score: brier_sum / denom,
        mean_log_loss: log_loss_sum / denom,
        top_choice_accuracy: top_choice_correct as f64 / denom,
        observed_choice_frequency: observed_counts.clone(),
        generated_choice_frequency: generated_counts.clone(),
        mean_predicted_probability: probability_sums,
        total_variation: total_variation(&observed_counts, &generated_counts),
        jensen_shannon_bits: jensen_shannon_bits(&observed_counts, &generated_counts),
        observed_entropy_bits,
        generated_entropy_bits,
        entropy_delta_bits: generated_entropy_bits - observed_entropy_bits,
        observed_tail_option_count: tail_options.len(),
        generated_tail_option_count,
        tail_option_coverage,
    }
}

fn trajectory_transition_metrics(
    trials: &[MatchedTrial<'_>],
    option_count: usize,
) -> (Option<f64>, usize) {
    let mut per_participant: BTreeMap<&str, Vec<MatchedTrial<'_>>> = BTreeMap::new();
    for pair in trials {
        per_participant
            .entry(pair.0.participant_id.as_str())
            .or_default()
            .push(*pair);
    }

    let mut observed_transitions = vec![0.0; option_count * option_count];
    let mut generated_transitions = vec![0.0; option_count * option_count];
    let mut transition_count = 0usize;

    for sequence in per_participant.values_mut() {
        sequence.sort_by_key(|pair| pair.0.step_index);
        for adjacent in sequence.windows(2) {
            let (observed_before, simulated_before) = adjacent[0];
            let (observed_after, simulated_after) = adjacent[1];
            if observed_after.step_index != observed_before.step_index + 1 {
                continue;
            }
            observed_transitions[observed_before.choice * option_count + observed_after.choice] +=
                1.0;
            generated_transitions[simulated_before.generated_choice * option_count
                + simulated_after.generated_choice] += 1.0;
            transition_count += 1;
        }
    }

    if transition_count == 0 {
        return (None, 0);
    }
    normalize(&mut observed_transitions);
    normalize(&mut generated_transitions);
    (
        Some(total_variation(
            &observed_transitions,
            &generated_transitions,
        )),
        transition_count,
    )
}

fn argmax(values: &[f64]) -> usize {
    values
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(right.1).then_with(|| right.0.cmp(&left.0)))
        .map_or(0, |(index, _)| index)
}

fn normalize(values: &mut [f64]) {
    let sum = values.iter().sum::<f64>();
    if sum > 0.0 {
        for value in values {
            *value /= sum;
        }
    }
}

fn entropy_bits(distribution: &[f64]) -> f64 {
    distribution
        .iter()
        .filter(|probability| **probability > 0.0)
        .map(|probability| -probability * probability.log2())
        .sum()
}

fn total_variation(left: &[f64], right: &[f64]) -> f64 {
    0.5 * left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - right).abs())
        .sum::<f64>()
}

fn jensen_shannon_bits(left: &[f64], right: &[f64]) -> f64 {
    let midpoint = left
        .iter()
        .zip(right)
        .map(|(left, right)| (left + right) / 2.0)
        .collect::<Vec<_>>();
    0.5 * kl_divergence_bits(left, &midpoint) + 0.5 * kl_divergence_bits(right, &midpoint)
}

fn kl_divergence_bits(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .filter(|(probability, _)| **probability > 0.0)
        .map(|(probability, reference)| probability * (probability / reference).log2())
        .sum()
}


#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// Unbiased bounded integer draw using rejection sampling.
    fn next_index(&mut self, upper_exclusive: usize) -> usize {
        assert!(upper_exclusive > 0);
        let upper = upper_exclusive as u64;
        let accepted_zone = u64::MAX - (u64::MAX % upper);
        loop {
            let value = self.next_u64();
            if value < accepted_zone {
                return (value % upper) as usize;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Vec<String> {
        ["A", "B", "C", "D"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    fn fixture_dataset(participant_count: usize, mode_collapse: bool) -> HumanFidelityDataset {
        let mut observed = Vec::new();
        let mut simulated = Vec::new();
        for participant in 0..participant_count {
            let participant_id = format!("participant_{participant:03}");
            let cohort_id = format!("cohort_{}", participant % 3);
            for step_index in 0..3_u32 {
                let context_draw = ((participant * 7 + step_index as usize * 11) % 100) as u32;
                let choice = match participant % 3 {
                    0 => {
                        if context_draw < 6 {
                            2
                        } else if context_draw < 10 {
                            3
                        } else if context_draw < 30 {
                            1
                        } else {
                            0
                        }
                    }
                    1 => {
                        if context_draw < 6 {
                            2
                        } else if context_draw < 10 {
                            3
                        } else if context_draw < 30 {
                            0
                        } else {
                            1
                        }
                    }
                    _ => {
                        if context_draw < 6 {
                            2
                        } else if context_draw < 10 {
                            1
                        } else if context_draw < 30 {
                            0
                        } else {
                            3
                        }
                    }
                };
                let context_id = format!("scenario_{step_index}");
                observed.push(ObservedTrial {
                    participant_id: participant_id.clone(),
                    context_id: context_id.clone(),
                    cohort_id: cohort_id.clone(),
                    step_index,
                    choice,
                });
                let mut probabilities = if mode_collapse {
                    vec![0.82, 0.10, 0.06, 0.02]
                } else {
                    vec![0.1; 4]
                };
                if !mode_collapse {
                    probabilities[choice] = 0.7;
                }
                simulated.push(SimulatedTrial {
                    participant_id: participant_id.clone(),
                    context_id,
                    cohort_id: cohort_id.clone(),
                    step_index,
                    choice_probabilities: probabilities,
                    generated_choice: if mode_collapse { 0 } else { choice },
                });
            }
        }
        HumanFidelityDataset {
            option_labels: options(),
            observed,
            simulated,
        }
    }

    #[test]
    fn matched_contextual_policy_can_have_good_micro_and_population_fidelity() {
        let dataset = fixture_dataset(60, false);
        let result = evaluate(&dataset).unwrap();
        assert!(result.overall.mean_brier_score < 0.13);
        assert_eq!(result.overall.total_variation, 0.0);
        assert_eq!(result.overall.jensen_shannon_bits, 0.0);
        assert_eq!(result.trajectory_transition_tvd, Some(0.0));
        assert_eq!(result.slices.len(), 9);
    }

    #[test]
    fn macro_mode_collapse_is_detected_even_when_probabilities_are_well_formed() {
        let dataset = fixture_dataset(60, true);
        let result = evaluate(&dataset).unwrap();
        assert!(result.overall.total_variation > 0.25);
        assert!(result.overall.jensen_shannon_bits > 0.1);
        assert!(result.overall.entropy_delta_bits < 0.0);
        assert!(result.trajectory_transition_tvd.unwrap() > 0.0);
        assert_eq!(result.overall.tail_option_coverage, Some(0.0));
    }

    #[test]
    fn high_entropy_and_tail_option_coverage_are_not_hidden_by_accuracy() {
        let result = evaluate(&fixture_dataset(60, false)).unwrap();
        assert!(result.overall.observed_entropy_bits > 1.0);
        assert!(result.overall.tail_option_coverage.unwrap() > 0.0);
        assert!(result.overall.generated_tail_option_count > 0);
    }

    #[test]
    fn cohort_and_context_metrics_are_reported_separately() {
        let result = evaluate(&fixture_dataset(60, true)).unwrap();
        let contexts = result
            .slices
            .iter()
            .map(|slice| slice.context_id.as_str())
            .collect::<BTreeSet<_>>();
        let cohorts = result
            .slices
            .iter()
            .map(|slice| slice.cohort_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(contexts.len(), 3);
        assert_eq!(cohorts.len(), 3);
        assert!(
            result
                .slices
                .iter()
                .all(|slice| slice.participant_count == 20)
        );
    }

    #[test]
    fn population_coverage_curve_is_deterministic_and_reports_distinct_sample_sizes() {
        let dataset = fixture_dataset(60, true);
        let sizes = [10, 20, 30, 50, 60, 20];
        let first = population_coverage_curve(&dataset, &sizes).unwrap();
        let second = population_coverage_curve(&dataset, &sizes).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first
                .iter()
                .map(|point| point.participant_count)
                .collect::<Vec<_>>(),
            vec![10, 20, 30, 50, 60]
        );
        assert_eq!(first.last().unwrap().trial_count, 180);
    }

    #[test]
    fn trajectory_metric_uses_adjacent_ordered_contexts_not_pooled_choices() {
        let result = evaluate(&fixture_dataset(60, true)).unwrap();
        assert_eq!(result.transition_count, 120);
        assert!(result.trajectory_transition_tvd.unwrap() > 0.0);
    }

    #[test]
    fn mismatched_sample_splits_fail_closed() {
        let mut dataset = fixture_dataset(10, false);
        dataset.simulated.pop();
        assert!(matches!(
            dataset.validate(),
            Err(FidelityError::TrialCountMismatch { .. })
        ));
    }

    #[test]
    fn malformed_probabilities_and_unknown_options_fail_closed() {
        let mut dataset = fixture_dataset(10, false);
        dataset.simulated[0].choice_probabilities[0] = f64::NAN;
        assert!(matches!(
            dataset.validate(),
            Err(FidelityError::InvalidProbability { .. })
        ));

        let mut invalid_choice = fixture_dataset(10, false);
        invalid_choice.observed[0].choice = 9;
        assert!(matches!(
            invalid_choice.validate(),
            Err(FidelityError::InvalidObservedChoice { .. })
        ));
    }

    #[test]
    fn duplicate_and_cohort_inconsistent_participant_keys_fail_closed() {
        let mut duplicate = fixture_dataset(10, false);
        duplicate.observed.push(duplicate.observed[0].clone());
        assert!(matches!(
            duplicate.validate(),
            Err(FidelityError::DuplicateParticipantStep { .. })
        ));

        let mut inconsistent = fixture_dataset(10, false);
        inconsistent.observed[1].cohort_id = "different_cohort".into();
        assert!(matches!(
            inconsistent.validate(),
            Err(FidelityError::InconsistentParticipantCohort { .. })
        ));
    }

    #[test]
    fn coverage_curve_rejects_zero_and_oversized_requests() {
        let dataset = fixture_dataset(10, false);
        assert!(matches!(
            population_coverage_curve(&dataset, &[0]),
            Err(FidelityError::InvalidCoverageRequest { .. })
        ));
        assert!(matches!(
            population_coverage_curve(&dataset, &[11]),
            Err(FidelityError::InvalidCoverageRequest { .. })
        ));
    }


    #[test]
    fn participant_cluster_bootstrap_replays_exactly_and_keeps_cluster_trajectories() {
        let dataset = fixture_dataset(60, true);
        let config = BootstrapConfig {
            seed: 0x4855_4D53_494D_3032,
            replicate_count: 40,
        };
        let first = bootstrap_population_coverage_curve(&dataset, &[10, 20, 60], config).unwrap();
        let second = bootstrap_population_coverage_curve(&dataset, &[10, 20, 60], config).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.iter().map(|point| point.participant_count).collect::<Vec<_>>(),
            vec![10, 20, 60]
        );
        for point in &first {
            assert_eq!(point.replicate_count, 40);
            assert!((point.mean_trial_count_per_replicate
                - point.participant_count as f64 * 3.0)
                .abs() < 1e-12);
            assert!(point.metrics.iter().all(|metric| {
                metric.valid_replicates + metric.undefined_replicates == 40
            }));
        }
    }

    #[test]
    fn different_bootstrap_seeds_change_the_participant_composition_distribution() {
        let dataset = fixture_dataset(60, true);
        let first = bootstrap_population_coverage_curve(
            &dataset,
            &[60],
            BootstrapConfig { seed: 1, replicate_count: 100 },
        ).unwrap();
        let second = bootstrap_population_coverage_curve(
            &dataset,
            &[60],
            BootstrapConfig { seed: 2, replicate_count: 100 },
        ).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn undefined_transition_metrics_are_counted_not_imputed() {
        let mut dataset = fixture_dataset(30, false);
        dataset.observed.retain(|trial| trial.step_index == 0);
        dataset.simulated.retain(|trial| trial.step_index == 0);
        let result = bootstrap_population_coverage_curve(
            &dataset,
            &[20],
            BootstrapConfig { seed: 77, replicate_count: 25 },
        ).unwrap();
        let transition = result[0].metrics.iter()
            .find(|metric| metric.metric == BootstrapMetric::TrajectoryTransitionTvd)
            .unwrap();
        assert_eq!(transition.valid_replicates, 0);
        assert_eq!(transition.undefined_replicates, 25);
        assert_eq!(transition.mean, None);
        assert_eq!(transition.p2_5, None);
    }

    #[test]
    fn bootstrap_rejects_invalid_sizes_and_replicate_counts() {
        let dataset = fixture_dataset(10, false);
        assert!(matches!(
            bootstrap_population_coverage_curve(
                &dataset,
                &[0],
                BootstrapConfig { seed: 1, replicate_count: 10 }
            ),
            Err(FidelityError::InvalidCoverageRequest { .. })
        ));
        assert!(matches!(
            bootstrap_population_coverage_curve(
                &dataset,
                &[11],
                BootstrapConfig { seed: 1, replicate_count: 10 }
            ),
            Err(FidelityError::InvalidCoverageRequest { .. })
        ));
        assert!(matches!(
            bootstrap_population_coverage_curve(
                &dataset,
                &[5],
                BootstrapConfig { seed: 1, replicate_count: 0 }
            ),
            Err(FidelityError::InvalidBootstrapReplicateCount(0))
        ));
        assert!(matches!(
            bootstrap_population_coverage_curve(
                &dataset,
                &[],
                BootstrapConfig { seed: 1, replicate_count: 10 }
            ),
            Err(FidelityError::EmptyBootstrapSizes)
        ));
    }

    #[test]
    fn option_label_errors_fail_closed() {
        let mut dataset = fixture_dataset(10, false);
        dataset.option_labels[1] = dataset.option_labels[0].clone();
        assert!(matches!(
            dataset.validate(),
            Err(FidelityError::DuplicateOptionLabel(_))
        ));
    }
}
