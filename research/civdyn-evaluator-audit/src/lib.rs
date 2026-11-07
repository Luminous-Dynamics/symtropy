// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Evaluator reliability and plural adjudication reference.
//!
//! Human panel judgments are retained as a distribution, not collapsed into
//! one presumed truth label. Agreement, disagreement, judge scoring and
//! confidence calibration are reported separately. This is a measurement
//! kernel; it cannot establish that a rubric is legitimate or culturally true.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

const PROBABILITY_TOLERANCE: f64 = 1e-9;
const LOG_FLOOR: f64 = 1e-15;
const CALIBRATION_BINS: usize = 5;
const ALPHA_DENOMINATOR_TOLERANCE: f64 = 1e-12;

#[derive(Debug, Clone, PartialEq)]
pub struct PanelRating {
    /// Opaque rater ID. It must not be the ID of a model judge in this dataset.
    pub rater_id: String,
    pub category_index: usize,
    /// Optional rater self-confidence; not a correctness probability.
    pub reported_confidence: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JudgePrediction {
    pub judge_id: String,
    pub selected_category: usize,
    /// Full probability distribution in stable category-label order.
    pub probabilities: Vec<f64>,
    /// Optional self-reported confidence in the selected judgment.
    pub reported_confidence: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationCase {
    pub case_id: String,
    pub context_id: String,
    /// Explicit source-provided stratum label; never inferred from free text.
    pub cohort_id: String,
    pub ratings: Vec<PanelRating>,
    pub judge_predictions: Vec<JudgePrediction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatorDataset {
    pub rubric_id: String,
    pub rubric_version: String,
    pub category_labels: Vec<String>,
    pub cases: Vec<EvaluationCase>,
}

impl EvaluatorDataset {
    pub fn validate(&self) -> Result<(), EvaluatorError> {
        if self.rubric_id.trim().is_empty() {
            return Err(EvaluatorError::EmptyRubricId);
        }
        if self.rubric_version.trim().is_empty() {
            return Err(EvaluatorError::EmptyRubricVersion);
        }
        if self.category_labels.len() < 2 {
            return Err(EvaluatorError::TooFewCategories(self.category_labels.len()));
        }
        let mut category_ids = BTreeSet::new();
        for category in &self.category_labels {
            if category.trim().is_empty() {
                return Err(EvaluatorError::EmptyCategoryLabel);
            }
            if !category_ids.insert(category) {
                return Err(EvaluatorError::DuplicateCategoryLabel(category.clone()));
            }
        }
        if self.cases.is_empty() {
            return Err(EvaluatorError::EmptyCases);
        }

        let mut case_ids = BTreeSet::new();
        let mut all_rater_ids = BTreeSet::new();
        let mut all_judge_ids = BTreeSet::new();

        for case in &self.cases {
            if case.case_id.trim().is_empty() {
                return Err(EvaluatorError::EmptyCaseId);
            }
            if !case_ids.insert(case.case_id.as_str()) {
                return Err(EvaluatorError::DuplicateCaseId(case.case_id.clone()));
            }
            if case.context_id.trim().is_empty() {
                return Err(EvaluatorError::EmptyContextId {
                    case_id: case.case_id.clone(),
                });
            }
            if case.cohort_id.trim().is_empty() {
                return Err(EvaluatorError::EmptyCohortId {
                    case_id: case.case_id.clone(),
                });
            }
            if case.ratings.len() < 2 {
                return Err(EvaluatorError::TooFewPanelRatings {
                    case_id: case.case_id.clone(),
                    actual: case.ratings.len(),
                    minimum: 2,
                });
            }

            let mut local_raters = BTreeSet::new();
            for rating in &case.ratings {
                if rating.rater_id.trim().is_empty() {
                    return Err(EvaluatorError::EmptyRaterId {
                        case_id: case.case_id.clone(),
                    });
                }
                if !local_raters.insert(rating.rater_id.as_str()) {
                    return Err(EvaluatorError::DuplicateRaterCase {
                        case_id: case.case_id.clone(),
                        rater_id: rating.rater_id.clone(),
                    });
                }
                all_rater_ids.insert(rating.rater_id.as_str());
                if rating.category_index >= self.category_labels.len() {
                    return Err(EvaluatorError::InvalidPanelCategory {
                        case_id: case.case_id.clone(),
                        rater_id: rating.rater_id.clone(),
                        category_index: rating.category_index,
                        category_count: self.category_labels.len(),
                    });
                }
                validate_confidence(
                    rating.reported_confidence,
                    &case.case_id,
                    &rating.rater_id,
                    "panel",
                )?;
            }

            let mut local_judges = BTreeSet::new();
            for prediction in &case.judge_predictions {
                if prediction.judge_id.trim().is_empty() {
                    return Err(EvaluatorError::EmptyJudgeId {
                        case_id: case.case_id.clone(),
                    });
                }
                if !local_judges.insert(prediction.judge_id.as_str()) {
                    return Err(EvaluatorError::DuplicateJudgeCase {
                        case_id: case.case_id.clone(),
                        judge_id: prediction.judge_id.clone(),
                    });
                }
                all_judge_ids.insert(prediction.judge_id.as_str());
                if prediction.probabilities.len() != self.category_labels.len() {
                    return Err(EvaluatorError::ProbabilityLengthMismatch {
                        case_id: case.case_id.clone(),
                        judge_id: prediction.judge_id.clone(),
                        expected: self.category_labels.len(),
                        actual: prediction.probabilities.len(),
                    });
                }
                for (category_index, probability) in prediction.probabilities.iter().enumerate() {
                    if !probability.is_finite() || *probability < 0.0 {
                        return Err(EvaluatorError::InvalidProbability {
                            case_id: case.case_id.clone(),
                            judge_id: prediction.judge_id.clone(),
                            category_index,
                            value: *probability,
                        });
                    }
                }
                let sum = prediction.probabilities.iter().sum::<f64>();
                if (sum - 1.0).abs() > PROBABILITY_TOLERANCE {
                    return Err(EvaluatorError::ProbabilityMassMismatch {
                        case_id: case.case_id.clone(),
                        judge_id: prediction.judge_id.clone(),
                        sum,
                    });
                }
                if prediction.selected_category >= self.category_labels.len() {
                    return Err(EvaluatorError::InvalidJudgeCategory {
                        case_id: case.case_id.clone(),
                        judge_id: prediction.judge_id.clone(),
                        category_index: prediction.selected_category,
                        category_count: self.category_labels.len(),
                    });
                }
                let argmax = argmax(&prediction.probabilities);
                if argmax != prediction.selected_category {
                    return Err(EvaluatorError::SelectedCategoryNotArgmax {
                        case_id: case.case_id.clone(),
                        judge_id: prediction.judge_id.clone(),
                        selected_category: prediction.selected_category,
                        argmax_category: argmax,
                    });
                }
                validate_confidence(
                    prediction.reported_confidence,
                    &case.case_id,
                    &prediction.judge_id,
                    "judge",
                )?;
            }
        }

        if let Some(overlap) = all_rater_ids.intersection(&all_judge_ids).next() {
            return Err(EvaluatorError::JudgeAlsoListedAsPanelRater(
                (*overlap).to_owned(),
            ));
        }
        if all_judge_ids.is_empty() {
            return Err(EvaluatorError::NoJudgePredictions);
        }
        Ok(())
    }
}

fn validate_confidence(
    confidence: Option<f64>,
    case_id: &str,
    actor_id: &str,
    actor_kind: &'static str,
) -> Result<(), EvaluatorError> {
    if let Some(value) = confidence
        && (!value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Err(EvaluatorError::InvalidConfidence {
            case_id: case_id.to_owned(),
            actor_id: actor_id.to_owned(),
            actor_kind,
            value,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvaluatorError {
    EmptyRubricId,
    EmptyRubricVersion,
    TooFewCategories(usize),
    EmptyCategoryLabel,
    DuplicateCategoryLabel(String),
    EmptyCases,
    EmptyCaseId,
    DuplicateCaseId(String),
    EmptyContextId {
        case_id: String,
    },
    EmptyCohortId {
        case_id: String,
    },
    TooFewPanelRatings {
        case_id: String,
        actual: usize,
        minimum: usize,
    },
    EmptyRaterId {
        case_id: String,
    },
    DuplicateRaterCase {
        case_id: String,
        rater_id: String,
    },
    InvalidPanelCategory {
        case_id: String,
        rater_id: String,
        category_index: usize,
        category_count: usize,
    },
    EmptyJudgeId {
        case_id: String,
    },
    DuplicateJudgeCase {
        case_id: String,
        judge_id: String,
    },
    ProbabilityLengthMismatch {
        case_id: String,
        judge_id: String,
        expected: usize,
        actual: usize,
    },
    InvalidProbability {
        case_id: String,
        judge_id: String,
        category_index: usize,
        value: f64,
    },
    ProbabilityMassMismatch {
        case_id: String,
        judge_id: String,
        sum: f64,
    },
    InvalidJudgeCategory {
        case_id: String,
        judge_id: String,
        category_index: usize,
        category_count: usize,
    },
    SelectedCategoryNotArgmax {
        case_id: String,
        judge_id: String,
        selected_category: usize,
        argmax_category: usize,
    },
    InvalidConfidence {
        case_id: String,
        actor_id: String,
        actor_kind: &'static str,
        value: f64,
    },
    JudgeAlsoListedAsPanelRater(String),
    NoJudgePredictions,
}

impl fmt::Display for EvaluatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRubricId => write!(f, "rubric ID must not be empty"),
            Self::EmptyRubricVersion => write!(f, "rubric version must not be empty"),
            Self::TooFewCategories(count) => {
                write!(f, "at least two categories required, got {count}")
            }
            Self::EmptyCategoryLabel => write!(f, "category labels must not be empty"),
            Self::DuplicateCategoryLabel(label) => {
                write!(f, "duplicate category label: {label}")
            }
            Self::EmptyCases => write!(f, "evaluation dataset must contain cases"),
            Self::EmptyCaseId => write!(f, "case ID must not be empty"),
            Self::DuplicateCaseId(id) => write!(f, "duplicate case ID: {id}"),
            Self::EmptyContextId { case_id } => {
                write!(f, "case {case_id} has an empty context ID")
            }
            Self::EmptyCohortId { case_id } => {
                write!(f, "case {case_id} has an empty cohort ID")
            }
            Self::TooFewPanelRatings {
                case_id,
                actual,
                minimum,
            } => write!(
                f,
                "case {case_id} has {actual} panel ratings; at least {minimum} required"
            ),
            Self::EmptyRaterId { case_id } => {
                write!(f, "case {case_id} has an empty panel-rater ID")
            }
            Self::DuplicateRaterCase { case_id, rater_id } => {
                write!(f, "rater {rater_id} is duplicated for case {case_id}")
            }
            Self::InvalidPanelCategory {
                case_id,
                rater_id,
                category_index,
                category_count,
            } => write!(
                f,
                "panel rating {rater_id}/{case_id} has category {category_index} outside 0..{category_count}"
            ),
            Self::EmptyJudgeId { case_id } => {
                write!(f, "case {case_id} has an empty judge ID")
            }
            Self::DuplicateJudgeCase { case_id, judge_id } => {
                write!(
                    f,
                    "judge {judge_id} has duplicate predictions for case {case_id}"
                )
            }
            Self::ProbabilityLengthMismatch {
                case_id,
                judge_id,
                expected,
                actual,
            } => write!(
                f,
                "judge {judge_id}/{case_id} has {actual} probabilities; expected {expected}"
            ),
            Self::InvalidProbability {
                case_id,
                judge_id,
                category_index,
                value,
            } => write!(
                f,
                "judge {judge_id}/{case_id} category {category_index} has invalid probability {value}"
            ),
            Self::ProbabilityMassMismatch {
                case_id,
                judge_id,
                sum,
            } => {
                write!(
                    f,
                    "judge {judge_id}/{case_id} probabilities sum to {sum}, not one"
                )
            }
            Self::InvalidJudgeCategory {
                case_id,
                judge_id,
                category_index,
                category_count,
            } => write!(
                f,
                "judge {judge_id}/{case_id} selected category {category_index} outside 0..{category_count}"
            ),
            Self::SelectedCategoryNotArgmax {
                case_id,
                judge_id,
                selected_category,
                argmax_category,
            } => write!(
                f,
                "judge {judge_id}/{case_id} selected category {selected_category} but probability argmax is {argmax_category}"
            ),
            Self::InvalidConfidence {
                case_id,
                actor_id,
                actor_kind,
                value,
            } => write!(
                f,
                "{actor_kind} {actor_id}/{case_id} has confidence outside [0,1]: {value}"
            ),
            Self::JudgeAlsoListedAsPanelRater(id) => {
                write!(
                    f,
                    "judge {id} is also present in the independent panel-rater set"
                )
            }
            Self::NoJudgePredictions => {
                write!(f, "dataset must include at least one judge prediction")
            }
        }
    }
}

impl std::error::Error for EvaluatorError {}

#[derive(Debug, Clone, PartialEq)]
pub struct PanelMetrics {
    pub case_count: usize,
    pub rating_count: usize,
    pub pair_count: usize,
    pub mean_raters_per_case: f64,
    pub minimum_raters_per_case: usize,
    pub maximum_raters_per_case: usize,
    pub observed_pairwise_agreement: f64,
    /// Krippendorff alpha (nominal), appropriate for variable panel sizes and missing ratings.
    pub krippendorff_alpha_nominal: Option<f64>,
    pub mean_disagreement_entropy_bits: f64,
    pub unanimous_case_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CasePanelMetrics {
    pub case_id: String,
    pub context_id: String,
    pub cohort_id: String,
    pub rater_count: usize,
    pub plurality_category: usize,
    pub plurality_support: f64,
    pub disagreement_entropy_bits: f64,
    pub judge_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationBin {
    pub bin_index: usize,
    pub case_count: usize,
    pub mean_reported_confidence: f64,
    pub mean_panel_support: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JudgeMetrics {
    pub judge_id: String,
    pub case_count: usize,
    pub mean_soft_panel_brier_score: f64,
    /// Cross-entropy in nats, with probabilities clipped at LOG_FLOOR.
    pub mean_soft_panel_cross_entropy_nats: f64,
    pub mean_plurality_match_rate: f64,
    pub mean_panel_support_for_selected: f64,
    pub confidence_case_count: usize,
    pub mean_reported_confidence: Option<f64>,
    pub mean_absolute_confidence_support_error: Option<f64>,
    /// Expected calibration error using panel support for the selected class as the target.
    pub confidence_ece: Option<f64>,
    pub calibration_bins: Vec<CalibrationBin>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JudgeSliceMetrics {
    pub context_id: String,
    pub cohort_id: String,
    pub metrics: JudgeMetrics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PanelSliceMetrics {
    pub context_id: String,
    pub cohort_id: String,
    pub metrics: PanelMetrics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatorAuditReport {
    pub rubric_id: String,
    pub rubric_version: String,
    pub category_labels: Vec<String>,
    pub overall_panel: PanelMetrics,
    pub panel_slices: Vec<PanelSliceMetrics>,
    pub case_panels: Vec<CasePanelMetrics>,
    pub overall_judges: Vec<JudgeMetrics>,
    pub judge_slices: Vec<JudgeSliceMetrics>,
}

pub fn evaluate(dataset: &EvaluatorDataset) -> Result<EvaluatorAuditReport, EvaluatorError> {
    dataset.validate()?;
    let overall_panel = panel_metrics(&dataset.cases, dataset.category_labels.len());
    let case_panels = dataset
        .cases
        .iter()
        .map(|case| case_panel_metrics(case, dataset.category_labels.len()))
        .collect::<Vec<_>>();

    let mut panel_groups: BTreeMap<(String, String), Vec<&EvaluationCase>> = BTreeMap::new();
    for case in &dataset.cases {
        panel_groups
            .entry((case.context_id.clone(), case.cohort_id.clone()))
            .or_default()
            .push(case);
    }
    let panel_slices = panel_groups
        .into_iter()
        .map(|((context_id, cohort_id), cases)| PanelSliceMetrics {
            context_id,
            cohort_id,
            metrics: panel_metrics_from_refs(&cases, dataset.category_labels.len()),
        })
        .collect();

    let judge_ids = dataset
        .cases
        .iter()
        .flat_map(|case| {
            case.judge_predictions
                .iter()
                .map(|prediction| prediction.judge_id.clone())
        })
        .collect::<BTreeSet<_>>();
    let mut overall_judges = Vec::new();
    let mut judge_slices = Vec::new();

    for judge_id in judge_ids {
        let cases = dataset
            .cases
            .iter()
            .filter(|case| {
                case.judge_predictions
                    .iter()
                    .any(|prediction| prediction.judge_id == judge_id)
            })
            .collect::<Vec<_>>();
        overall_judges.push(judge_metrics(
            &judge_id,
            &cases,
            dataset.category_labels.len(),
        ));

        let mut groups: BTreeMap<(String, String), Vec<&EvaluationCase>> = BTreeMap::new();
        for case in &cases {
            groups
                .entry((case.context_id.clone(), case.cohort_id.clone()))
                .or_default()
                .push(*case);
        }
        for ((context_id, cohort_id), group_cases) in groups {
            judge_slices.push(JudgeSliceMetrics {
                context_id,
                cohort_id,
                metrics: judge_metrics(&judge_id, &group_cases, dataset.category_labels.len()),
            });
        }
    }

    overall_judges.sort_by(|left, right| left.judge_id.cmp(&right.judge_id));
    judge_slices.sort_by(|left, right| {
        left.metrics
            .judge_id
            .cmp(&right.metrics.judge_id)
            .then_with(|| left.context_id.cmp(&right.context_id))
            .then_with(|| left.cohort_id.cmp(&right.cohort_id))
    });

    Ok(EvaluatorAuditReport {
        rubric_id: dataset.rubric_id.clone(),
        rubric_version: dataset.rubric_version.clone(),
        category_labels: dataset.category_labels.clone(),
        overall_panel,
        panel_slices,
        case_panels,
        overall_judges,
        judge_slices,
    })
}

fn panel_metrics(cases: &[EvaluationCase], category_count: usize) -> PanelMetrics {
    let refs = cases.iter().collect::<Vec<_>>();
    panel_metrics_from_refs(&refs, category_count)
}

fn panel_metrics_from_refs(cases: &[&EvaluationCase], category_count: usize) -> PanelMetrics {
    let mut total_ratings = 0usize;
    let mut total_pairs = 0usize;
    let mut total_disagreeing_pairs = 0usize;
    let mut pooled_counts = vec![0usize; category_count];
    let mut total_entropy = 0.0;
    let mut unanimous_case_count = 0usize;
    let mut minimum_raters = usize::MAX;
    let mut maximum_raters = 0usize;

    for case in cases {
        let mut counts = vec![0usize; category_count];
        for rating in &case.ratings {
            counts[rating.category_index] += 1;
            pooled_counts[rating.category_index] += 1;
        }
        let n = case.ratings.len();
        total_ratings += n;
        minimum_raters = minimum_raters.min(n);
        maximum_raters = maximum_raters.max(n);
        let pairs = n * (n - 1);
        total_pairs += pairs;
        let agree_pairs = counts
            .iter()
            .map(|count| count * count.saturating_sub(1))
            .sum::<usize>();
        total_disagreeing_pairs += pairs - agree_pairs;
        total_entropy += entropy_bits_from_counts(&counts, n);
        if counts.contains(&n) {
            unanimous_case_count += 1;
        }
    }

    let observed_disagreement = if total_pairs == 0 {
        0.0
    } else {
        total_disagreeing_pairs as f64 / total_pairs as f64
    };
    let total = total_ratings as f64;
    let expected_agreement = pooled_counts
        .iter()
        .map(|count| *count as f64 * count.saturating_sub(1) as f64)
        .sum::<f64>()
        / (total * (total - 1.0));
    let expected_disagreement = 1.0 - expected_agreement;
    let alpha = if expected_disagreement <= ALPHA_DENOMINATOR_TOLERANCE {
        None
    } else {
        Some(1.0 - observed_disagreement / expected_disagreement)
    };

    PanelMetrics {
        case_count: cases.len(),
        rating_count: total_ratings,
        pair_count: total_pairs / 2,
        mean_raters_per_case: total_ratings as f64 / cases.len() as f64,
        minimum_raters_per_case: if cases.is_empty() { 0 } else { minimum_raters },
        maximum_raters_per_case: maximum_raters,
        observed_pairwise_agreement: 1.0 - observed_disagreement,
        krippendorff_alpha_nominal: alpha,
        mean_disagreement_entropy_bits: total_entropy / cases.len() as f64,
        unanimous_case_count,
    }
}

fn case_panel_metrics(case: &EvaluationCase, category_count: usize) -> CasePanelMetrics {
    let mut counts = vec![0usize; category_count];
    for rating in &case.ratings {
        counts[rating.category_index] += 1;
    }
    let plurality_category = counts
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.cmp(right.1).then_with(|| right.0.cmp(&left.0)))
        .map_or(0, |(index, _)| index);
    CasePanelMetrics {
        case_id: case.case_id.clone(),
        context_id: case.context_id.clone(),
        cohort_id: case.cohort_id.clone(),
        rater_count: case.ratings.len(),
        plurality_category,
        plurality_support: counts[plurality_category] as f64 / case.ratings.len() as f64,
        disagreement_entropy_bits: entropy_bits_from_counts(&counts, case.ratings.len()),
        judge_count: case.judge_predictions.len(),
    }
}

fn judge_metrics(judge_id: &str, cases: &[&EvaluationCase], category_count: usize) -> JudgeMetrics {
    let mut brier_sum = 0.0;
    let mut cross_entropy_sum = 0.0;
    let mut plurality_matches = 0usize;
    let mut selected_support_sum = 0.0;
    let mut confidence_sum = 0.0;
    let mut confidence_gap_sum = 0.0;
    let mut confidence_count = 0usize;
    let mut bins = [(0usize, 0.0_f64, 0.0_f64); CALIBRATION_BINS];

    for case in cases {
        let Some(prediction) = case
            .judge_predictions
            .iter()
            .find(|prediction| prediction.judge_id == judge_id)
        else {
            continue;
        };
        let target = panel_distribution(case, category_count);
        for (category, target_probability) in target.iter().enumerate().take(category_count) {
            let probability = prediction.probabilities[category];
            let delta = probability - target_probability;
            brier_sum += delta * delta;
            if *target_probability > 0.0 {
                cross_entropy_sum -= target_probability * probability.max(LOG_FLOOR).ln();
            }
        }

        let support = target[prediction.selected_category];
        selected_support_sum += support;
        let plurality = target.iter().copied().fold(0.0_f64, f64::max);
        if (support - plurality).abs() <= PROBABILITY_TOLERANCE {
            plurality_matches += 1;
        }

        if let Some(confidence) = prediction.reported_confidence {
            confidence_count += 1;
            confidence_sum += confidence;
            confidence_gap_sum += (confidence - support).abs();
            let bin_index =
                ((confidence * CALIBRATION_BINS as f64) as usize).min(CALIBRATION_BINS - 1);
            bins[bin_index].0 += 1;
            bins[bin_index].1 += confidence;
            bins[bin_index].2 += support;
        }
    }

    let count = cases
        .iter()
        .filter(|case| {
            case.judge_predictions
                .iter()
                .any(|prediction| prediction.judge_id == judge_id)
        })
        .count();
    let calibration_bins = bins
        .iter()
        .enumerate()
        .filter(|(_, (n, _, _))| *n > 0)
        .map(|(bin_index, (n, confidence, support))| CalibrationBin {
            bin_index,
            case_count: *n,
            mean_reported_confidence: *confidence / *n as f64,
            mean_panel_support: *support / *n as f64,
        })
        .collect::<Vec<_>>();
    let confidence_ece = if confidence_count == 0 {
        None
    } else {
        Some(
            calibration_bins
                .iter()
                .map(|bin| {
                    bin.case_count as f64 / confidence_count as f64
                        * (bin.mean_reported_confidence - bin.mean_panel_support).abs()
                })
                .sum(),
        )
    };

    JudgeMetrics {
        judge_id: judge_id.to_owned(),
        case_count: count,
        mean_soft_panel_brier_score: brier_sum / count as f64,
        mean_soft_panel_cross_entropy_nats: cross_entropy_sum / count as f64,
        mean_plurality_match_rate: plurality_matches as f64 / count as f64,
        mean_panel_support_for_selected: selected_support_sum / count as f64,
        confidence_case_count: confidence_count,
        mean_reported_confidence: if confidence_count == 0 {
            None
        } else {
            Some(confidence_sum / confidence_count as f64)
        },
        mean_absolute_confidence_support_error: if confidence_count == 0 {
            None
        } else {
            Some(confidence_gap_sum / confidence_count as f64)
        },
        confidence_ece,
        calibration_bins,
    }
}

fn panel_distribution(case: &EvaluationCase, category_count: usize) -> Vec<f64> {
    let mut counts = vec![0.0; category_count];
    for rating in &case.ratings {
        counts[rating.category_index] += 1.0;
    }
    for value in &mut counts {
        *value /= case.ratings.len() as f64;
    }
    counts
}

fn entropy_bits_from_counts(counts: &[usize], total: usize) -> f64 {
    counts
        .iter()
        .filter(|count| **count > 0)
        .map(|count| {
            let probability = *count as f64 / total as f64;
            -probability * probability.log2()
        })
        .sum()
}

fn argmax(values: &[f64]) -> usize {
    values
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(right.1).then_with(|| right.0.cmp(&left.0)))
        .map_or(0, |(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn judge_prediction(
        judge_id: &str,
        category: usize,
        probabilities: Vec<f64>,
        confidence: f64,
    ) -> JudgePrediction {
        JudgePrediction {
            judge_id: judge_id.into(),
            selected_category: category,
            probabilities,
            reported_confidence: Some(confidence),
        }
    }

    fn fixture_dataset() -> EvaluatorDataset {
        let mut cases = Vec::new();
        for index in 0..12 {
            let context_id = if index < 6 {
                "context_alpha"
            } else {
                "context_beta"
            };
            let cohort_id = if index % 2 == 0 {
                "community_a"
            } else {
                "community_b"
            };
            let panel_categories = if index < 4 {
                vec![0, 0, 0]
            } else if index < 8 {
                vec![1, 1, 2]
            } else {
                vec![3, 2, 3]
            };
            let counts = {
                let mut counts = vec![0.0; 4];
                for category in &panel_categories {
                    counts[*category] += 1.0 / panel_categories.len() as f64;
                }
                counts
            };
            let selected = counts
                .iter()
                .enumerate()
                .max_by(|left, right| left.1.total_cmp(right.1).then_with(|| right.0.cmp(&left.0)))
                .map_or(0, |(category, _)| category);
            let confidence = counts[selected];
            let overconfident_probabilities = (0..4)
                .map(|category| if category == selected { 0.97 } else { 0.01 })
                .collect::<Vec<_>>();
            cases.push(EvaluationCase {
                case_id: format!("case_{index:02}"),
                context_id: context_id.into(),
                cohort_id: cohort_id.into(),
                ratings: panel_categories
                    .iter()
                    .enumerate()
                    .map(|(rater_index, category_index)| PanelRating {
                        rater_id: format!("panel_rater_{rater_index}"),
                        category_index: *category_index,
                        reported_confidence: None,
                    })
                    .collect(),
                judge_predictions: vec![
                    judge_prediction("distribution_aware", selected, counts.clone(), confidence),
                    judge_prediction(
                        "overconfident_majority",
                        selected,
                        overconfident_probabilities,
                        0.99,
                    ),
                ],
            });
        }
        EvaluatorDataset {
            rubric_id: "synthetic_norm_rubric".into(),
            rubric_version: "0.1.0".into(),
            category_labels: vec![
                "category_a".into(),
                "category_b".into(),
                "category_c".into(),
                "category_d".into(),
            ],
            cases,
        }
    }

    #[test]
    fn panel_agreement_preserves_disagreement_instead_of_majority_collapsing() {
        let report = evaluate(&fixture_dataset()).unwrap();
        assert_eq!(report.overall_panel.case_count, 12);
        assert_eq!(report.overall_panel.rating_count, 36);
        assert_eq!(report.overall_panel.unanimous_case_count, 4);
        assert!(report.overall_panel.observed_pairwise_agreement > 0.5);
        assert!(report.overall_panel.observed_pairwise_agreement < 1.0);
        assert!(report.overall_panel.krippendorff_alpha_nominal.unwrap() > 0.0);
        assert!(report.overall_panel.mean_disagreement_entropy_bits > 0.0);
        assert_eq!(report.case_panels[0].disagreement_entropy_bits, 0.0);
        assert!(report.case_panels[4].disagreement_entropy_bits > 0.0);
    }

    #[test]
    fn distribution_aware_judge_scores_better_than_overconfident_majority() {
        let report = evaluate(&fixture_dataset()).unwrap();
        let aware = report
            .overall_judges
            .iter()
            .find(|judge| judge.judge_id == "distribution_aware")
            .unwrap();
        let overconfident = report
            .overall_judges
            .iter()
            .find(|judge| judge.judge_id == "overconfident_majority")
            .unwrap();
        assert!(aware.mean_soft_panel_brier_score < 1e-12);
        assert!(
            aware.mean_soft_panel_cross_entropy_nats
                < overconfident.mean_soft_panel_cross_entropy_nats
        );
        assert!(aware.confidence_ece.unwrap() < 1e-12);
        assert!(overconfident.confidence_ece.unwrap() > 0.1);
        assert_eq!(aware.mean_plurality_match_rate, 1.0);
        assert_eq!(overconfident.mean_plurality_match_rate, 1.0);
        assert!(
            overconfident
                .mean_absolute_confidence_support_error
                .unwrap()
                > 0.1
        );
    }

    #[test]
    fn context_and_cohort_slices_remain_separate() {
        let report = evaluate(&fixture_dataset()).unwrap();
        assert_eq!(report.panel_slices.len(), 4);
        assert_eq!(report.judge_slices.len(), 8);
        assert!(
            report
                .panel_slices
                .iter()
                .all(|slice| slice.metrics.case_count == 3)
        );
    }

    #[test]
    fn variable_panel_sizes_are_supported_without_imputing_missing_raters() {
        let mut dataset = fixture_dataset();
        dataset.cases[0].ratings.pop();
        dataset.cases[4].ratings.pop();
        let report = evaluate(&dataset).unwrap();
        assert_eq!(report.overall_panel.rating_count, 34);
        assert_eq!(report.overall_panel.minimum_raters_per_case, 2);
        assert_eq!(report.overall_panel.maximum_raters_per_case, 3);
    }

    #[test]
    fn duplicate_raters_and_model_judge_in_panel_fail_closed() {
        let mut duplicate = fixture_dataset();
        let repeated_rating = duplicate.cases[0].ratings[0].clone();
        duplicate.cases[0].ratings.push(repeated_rating);
        assert!(matches!(
            duplicate.validate(),
            Err(EvaluatorError::DuplicateRaterCase { .. })
        ));

        let mut judge_as_panel = fixture_dataset();
        judge_as_panel.cases[0].ratings[0].rater_id = "distribution_aware".into();
        assert!(matches!(
            judge_as_panel.validate(),
            Err(EvaluatorError::JudgeAlsoListedAsPanelRater(_))
        ));
    }

    #[test]
    fn probability_and_rubric_failures_are_explicit() {
        let mut invalid = fixture_dataset();
        invalid.cases[0].judge_predictions[0].probabilities[0] = f64::NAN;
        assert!(matches!(
            invalid.validate(),
            Err(EvaluatorError::InvalidProbability { .. })
        ));

        let mut mixed_rubric = fixture_dataset();
        mixed_rubric.rubric_id.clear();
        assert!(matches!(
            mixed_rubric.validate(),
            Err(EvaluatorError::EmptyRubricId)
        ));
    }

    #[test]
    fn undefined_chance_corrected_agreement_when_panel_has_zero_expected_disagreement() {
        let mut dataset = fixture_dataset();
        for case in &mut dataset.cases {
            for rating in &mut case.ratings {
                rating.category_index = 0;
            }
            for judge in &mut case.judge_predictions {
                judge.selected_category = 0;
                judge.probabilities = vec![1.0, 0.0, 0.0, 0.0];
                judge.reported_confidence = Some(1.0);
            }
        }
        let report = evaluate(&dataset).unwrap();
        assert_eq!(report.overall_panel.krippendorff_alpha_nominal, None);
    }
}
