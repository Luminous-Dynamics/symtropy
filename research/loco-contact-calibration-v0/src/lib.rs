//! Fixed-point calibration metrics for continuous contact-reliability estimates.
//!
//! One invocation evaluates exactly one model/domain/split/channel/target
//! stratum. Callers must run separate strata rather than pooling heterogeneous
//! estimates. Observation-level metrics are descriptive; independent session
//! and episode counts remain visible for clustered statistical inference.

use std::collections::BTreeMap;

pub const SCALE_PPM: u32 = 1_000_000;
pub const MAX_BIN_COUNT: usize = 1_000;
const SCALE_SQUARED: u128 = 1_000_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContactChannel {
    InterfaceFields,
    NearFootWrenchVibroAcoustic,
    ProprioceptiveResidual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContactTarget {
    ContactPresence,
    NoSlipKinematicConstraintValidity,
    TractionFeasibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOutcome {
    Negative,
    Positive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbabilityObservation {
    pub observation_id: String,
    pub episode_id: String,
    pub exchangeability_unit_id: String,
    pub channel: ContactChannel,
    pub target: ContactTarget,
    pub model_digest: String,
    pub domain_digest: String,
    pub calibration_split_digest: String,
    /// Predicted probability as an integer in [0, 1_000_000] ppm.
    pub predicted_probability_ppm: u32,
    /// Independently adjudicated binary outcome.
    pub outcome: BinaryOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricDataset {
    pub channel: ContactChannel,
    pub target: ContactTarget,
    pub model_digest: String,
    pub domain_digest: String,
    pub calibration_split_digest: String,
    /// Equal-width bins. Values are assigned by floor(p * bins / 1e6);
    /// probability 1e6 is assigned to the final bin.
    pub bin_count: usize,
    pub observations: Vec<ProbabilityObservation>,
    /// Observation IDs reserved for fitting or final testing.
    pub forbidden_observation_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricError {
    EmptyIdentity,
    EmptyObservations,
    InvalidBinCount,
    EmptyObservationId,
    DuplicateObservationId(String),
    EmptyEpisodeId(String),
    EmptyExchangeabilityUnitId(String),
    EpisodeMapsToMultipleUnits(String),
    EmptyForbiddenObservationId,
    DuplicateForbiddenObservationId(String),
    CalibrationLeakage(String),
    MixedChannel(String),
    MixedTarget(String),
    MixedModelDigest(String),
    MixedDomainDigest(String),
    MixedCalibrationSplitDigest(String),
    InvalidProbability(String),
    ArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactRational {
    pub numerator: u128,
    pub denominator: u128,
}

/// Exact bin summaries. The bin boundaries are defined by the integer
/// assignment rule in the MetricDataset contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReliabilityBin {
    pub index: usize,
    pub observation_count: u128,
    pub predicted_probability_sum_ppm: u128,
    pub positive_outcome_count: u128,
    /// Absolute difference between predicted and observed probability sums,
    /// in ppm. Zero denotes exact in-bin calibration.
    pub absolute_gap_sum_ppm: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaximumCalibrationGap {
    pub bin_index: usize,
    /// Gap in ppm = numerator / denominator.
    pub gap_ppm: ExactRational,
    pub bin_observation_count: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationMetrics {
    pub channel: ContactChannel,
    pub target: ContactTarget,
    pub model_digest: String,
    pub domain_digest: String,
    pub calibration_split_digest: String,
    pub observation_count: u128,
    pub episode_count: u128,
    pub independent_unit_count: u128,
    /// Brier score = numerator / denominator, in [0, 1].
    pub brier_score: ExactRational,
    /// ECE in ppm = numerator / denominator.
    pub expected_calibration_error_ppm: ExactRational,
    /// MCE in ppm, with the exact worst-bin numerator and denominator.
    pub maximum_calibration_error: Option<MaximumCalibrationGap>,
    pub bins: Vec<ReliabilityBin>,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn checked_add(left: u128, right: u128) -> Result<u128, MetricError> {
    left.checked_add(right)
        .ok_or(MetricError::ArithmeticOverflow)
}

fn checked_mul(left: u128, right: u128) -> Result<u128, MetricError> {
    left.checked_mul(right)
        .ok_or(MetricError::ArithmeticOverflow)
}

fn validate_identity(value: &str) -> Result<(), MetricError> {
    if nonblank(value) {
        Ok(())
    } else {
        Err(MetricError::EmptyIdentity)
    }
}

fn probability_bin(probability_ppm: u32, bin_count: usize) -> usize {
    if probability_ppm == SCALE_PPM {
        return bin_count - 1;
    }
    ((u128::from(probability_ppm) * bin_count as u128) / u128::from(SCALE_PPM)) as usize
}

fn brier_error_magnitude(probability_ppm: u32, outcome: BinaryOutcome) -> u128 {
    let predicted = u128::from(probability_ppm);
    let expected = match outcome {
        BinaryOutcome::Negative => 0,
        BinaryOutcome::Positive => u128::from(SCALE_PPM),
    };
    predicted.abs_diff(expected)
}

fn checked_square_accumulate(total: u128, value: u128) -> Result<u128, MetricError> {
    checked_add(total, checked_mul(value, value)?)
}

fn compare_rationals(
    left: ExactRational,
    right: ExactRational,
) -> Result<std::cmp::Ordering, MetricError> {
    let left_cross = checked_mul(left.numerator, right.denominator)?;
    let right_cross = checked_mul(right.numerator, left.denominator)?;
    Ok(left_cross.cmp(&right_cross))
}

fn validate_dataset(dataset: &MetricDataset) -> Result<(), MetricError> {
    validate_identity(&dataset.model_digest)?;
    validate_identity(&dataset.domain_digest)?;
    validate_identity(&dataset.calibration_split_digest)?;
    if dataset.observations.is_empty() {
        return Err(MetricError::EmptyObservations);
    }
    if dataset.bin_count == 0 || dataset.bin_count > MAX_BIN_COUNT {
        return Err(MetricError::InvalidBinCount);
    }
    Ok(())
}

/// Evaluate one exact channel/target/model/domain/split stratum.
///
/// Brier score is returned as an exact rational in [0,1]. ECE and MCE are
/// returned as exact rational values in parts per million. Empty reliability
/// bins are retained in the report. No binning, threshold or calibration map
/// is learned from this evaluation set.
pub fn evaluate(dataset: &MetricDataset) -> Result<CalibrationMetrics, MetricError> {
    validate_dataset(dataset)?;

    let mut forbidden = BTreeMap::<String, ()>::new();
    for id in &dataset.forbidden_observation_ids {
        if !nonblank(id) {
            return Err(MetricError::EmptyForbiddenObservationId);
        }
        if forbidden.insert(id.clone(), ()).is_some() {
            return Err(MetricError::DuplicateForbiddenObservationId(id.clone()));
        }
    }

    let mut observation_ids = BTreeMap::<String, ()>::new();
    let mut episode_units = BTreeMap::<String, String>::new();
    let mut episodes = BTreeMap::<String, ()>::new();
    let mut units = BTreeMap::<String, ()>::new();
    let mut squared_error_sum = 0u128;
    let mut bins = (0..dataset.bin_count)
        .map(|index| ReliabilityBin {
            index,
            observation_count: 0,
            predicted_probability_sum_ppm: 0,
            positive_outcome_count: 0,
            absolute_gap_sum_ppm: 0,
        })
        .collect::<Vec<_>>();

    for observation in &dataset.observations {
        if !nonblank(&observation.observation_id) {
            return Err(MetricError::EmptyObservationId);
        }
        if observation_ids
            .insert(observation.observation_id.clone(), ())
            .is_some()
        {
            return Err(MetricError::DuplicateObservationId(
                observation.observation_id.clone(),
            ));
        }
        if forbidden.contains_key(&observation.observation_id) {
            return Err(MetricError::CalibrationLeakage(
                observation.observation_id.clone(),
            ));
        }
        if !nonblank(&observation.episode_id) {
            return Err(MetricError::EmptyEpisodeId(
                observation.observation_id.clone(),
            ));
        }
        if !nonblank(&observation.exchangeability_unit_id) {
            return Err(MetricError::EmptyExchangeabilityUnitId(
                observation.observation_id.clone(),
            ));
        }
        if observation.channel != dataset.channel {
            return Err(MetricError::MixedChannel(
                observation.observation_id.clone(),
            ));
        }
        if observation.target != dataset.target {
            return Err(MetricError::MixedTarget(observation.observation_id.clone()));
        }
        if observation.model_digest != dataset.model_digest {
            return Err(MetricError::MixedModelDigest(
                observation.observation_id.clone(),
            ));
        }
        if observation.domain_digest != dataset.domain_digest {
            return Err(MetricError::MixedDomainDigest(
                observation.observation_id.clone(),
            ));
        }
        if observation.calibration_split_digest != dataset.calibration_split_digest {
            return Err(MetricError::MixedCalibrationSplitDigest(
                observation.observation_id.clone(),
            ));
        }
        if observation.predicted_probability_ppm > SCALE_PPM {
            return Err(MetricError::InvalidProbability(
                observation.observation_id.clone(),
            ));
        }

        if let Some(existing_unit) = episode_units.get(&observation.episode_id) {
            if existing_unit != &observation.exchangeability_unit_id {
                return Err(MetricError::EpisodeMapsToMultipleUnits(
                    observation.episode_id.clone(),
                ));
            }
        } else {
            episode_units.insert(
                observation.episode_id.clone(),
                observation.exchangeability_unit_id.clone(),
            );
        }
        episodes.insert(observation.episode_id.clone(), ());
        units.insert(observation.exchangeability_unit_id.clone(), ());

        squared_error_sum = checked_square_accumulate(
            squared_error_sum,
            brier_error_magnitude(observation.predicted_probability_ppm, observation.outcome),
        )?;

        let index = probability_bin(observation.predicted_probability_ppm, dataset.bin_count);
        let bin = &mut bins[index];
        bin.observation_count = checked_add(bin.observation_count, 1)?;
        bin.predicted_probability_sum_ppm = checked_add(
            bin.predicted_probability_sum_ppm,
            u128::from(observation.predicted_probability_ppm),
        )?;
        if observation.outcome == BinaryOutcome::Positive {
            bin.positive_outcome_count = checked_add(bin.positive_outcome_count, 1)?;
        }
    }

    let observation_count =
        u128::try_from(dataset.observations.len()).map_err(|_| MetricError::ArithmeticOverflow)?;
    let episode_count =
        u128::try_from(episodes.len()).map_err(|_| MetricError::ArithmeticOverflow)?;
    let independent_unit_count =
        u128::try_from(units.len()).map_err(|_| MetricError::ArithmeticOverflow)?;
    let brier_denominator = checked_mul(observation_count, SCALE_SQUARED)?;

    let mut ece_numerator = 0u128;
    let mut maximum_gap: Option<MaximumCalibrationGap> = None;
    for bin in &mut bins {
        if bin.observation_count == 0 {
            continue;
        }
        let observed_positive_sum = checked_mul(bin.positive_outcome_count, u128::from(SCALE_PPM))?;
        bin.absolute_gap_sum_ppm = bin
            .predicted_probability_sum_ppm
            .abs_diff(observed_positive_sum);
        ece_numerator = checked_add(ece_numerator, bin.absolute_gap_sum_ppm)?;

        let gap = ExactRational {
            numerator: bin.absolute_gap_sum_ppm,
            denominator: bin.observation_count,
        };
        let replace = match &maximum_gap {
            None => true,
            Some(current) => {
                compare_rationals(
                    gap,
                    ExactRational {
                        numerator: current.gap_ppm.numerator,
                        denominator: current.gap_ppm.denominator,
                    },
                )? == std::cmp::Ordering::Greater
            }
        };
        if replace {
            maximum_gap = Some(MaximumCalibrationGap {
                bin_index: bin.index,
                gap_ppm: gap,
                bin_observation_count: bin.observation_count,
            });
        }
    }

    Ok(CalibrationMetrics {
        channel: dataset.channel,
        target: dataset.target,
        model_digest: dataset.model_digest.clone(),
        domain_digest: dataset.domain_digest.clone(),
        calibration_split_digest: dataset.calibration_split_digest.clone(),
        observation_count,
        episode_count,
        independent_unit_count,
        brier_score: ExactRational {
            numerator: squared_error_sum,
            denominator: brier_denominator,
        },
        expected_calibration_error_ppm: ExactRational {
            numerator: ece_numerator,
            denominator: observation_count,
        },
        maximum_calibration_error: maximum_gap,
        bins,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        id: &str,
        episode: &str,
        unit: &str,
        probability: u32,
        outcome: BinaryOutcome,
    ) -> ProbabilityObservation {
        ProbabilityObservation {
            observation_id: id.to_owned(),
            episode_id: episode.to_owned(),
            exchangeability_unit_id: unit.to_owned(),
            channel: ContactChannel::InterfaceFields,
            target: ContactTarget::ContactPresence,
            model_digest: "model-01".to_owned(),
            domain_digest: "rough-terrain-v1".to_owned(),
            calibration_split_digest: "eval-split-01".to_owned(),
            predicted_probability_ppm: probability,
            outcome,
        }
    }

    fn dataset(observations: Vec<ProbabilityObservation>, bin_count: usize) -> MetricDataset {
        MetricDataset {
            channel: ContactChannel::InterfaceFields,
            target: ContactTarget::ContactPresence,
            model_digest: "model-01".to_owned(),
            domain_digest: "rough-terrain-v1".to_owned(),
            calibration_split_digest: "eval-split-01".to_owned(),
            bin_count,
            observations,
            forbidden_observation_ids: vec!["fit-observation-01".to_owned()],
        }
    }

    #[test]
    fn perfect_binary_predictions_have_zero_brier_ece_and_mce() {
        let report = evaluate(&dataset(
            vec![
                observation("o1", "e1", "s1", 0, BinaryOutcome::Negative),
                observation("o2", "e2", "s2", SCALE_PPM, BinaryOutcome::Positive),
            ],
            10,
        ))
        .unwrap();
        assert_eq!(
            report.brier_score,
            ExactRational {
                numerator: 0,
                denominator: 2 * SCALE_SQUARED
            }
        );
        assert_eq!(
            report.expected_calibration_error_ppm,
            ExactRational {
                numerator: 0,
                denominator: 2
            }
        );
        assert_eq!(
            report.maximum_calibration_error.unwrap().gap_ppm,
            ExactRational {
                numerator: 0,
                denominator: 1
            }
        );
    }

    #[test]
    fn known_mixed_probability_fixture_has_exact_quarter_brier_and_zero_ece() {
        let report = evaluate(&dataset(
            vec![
                observation("o1", "e1", "s1", 500_000, BinaryOutcome::Negative),
                observation("o2", "e2", "s2", 500_000, BinaryOutcome::Positive),
            ],
            10,
        ))
        .unwrap();
        assert_eq!(
            report.brier_score,
            ExactRational {
                numerator: 500_000_000_000,
                denominator: 2 * SCALE_SQUARED
            }
        );
        assert_eq!(
            report.expected_calibration_error_ppm,
            ExactRational {
                numerator: 0,
                denominator: 2
            }
        );
    }

    #[test]
    fn overconfident_wrong_predictions_have_unit_brier_and_maximal_gap() {
        let report = evaluate(&dataset(
            vec![
                observation("o1", "e1", "s1", SCALE_PPM, BinaryOutcome::Negative),
                observation("o2", "e2", "s2", 0, BinaryOutcome::Positive),
            ],
            10,
        ))
        .unwrap();
        assert_eq!(
            report.brier_score,
            ExactRational {
                numerator: 2 * SCALE_SQUARED,
                denominator: 2 * SCALE_SQUARED
            }
        );
        assert_eq!(
            report.expected_calibration_error_ppm,
            ExactRational {
                numerator: 2 * u128::from(SCALE_PPM),
                denominator: 2
            }
        );
        assert_eq!(
            report.maximum_calibration_error.unwrap().gap_ppm,
            ExactRational {
                numerator: u128::from(SCALE_PPM),
                denominator: 1
            }
        );
    }

    #[test]
    fn zero_and_one_probabilities_land_in_first_and_last_bins() {
        let report = evaluate(&dataset(
            vec![
                observation("o1", "e1", "s1", 0, BinaryOutcome::Negative),
                observation("o2", "e2", "s2", SCALE_PPM, BinaryOutcome::Positive),
            ],
            10,
        ))
        .unwrap();
        assert_eq!(report.bins[0].observation_count, 1);
        assert_eq!(report.bins[9].observation_count, 1);
        assert_eq!(report.bins[1].observation_count, 0);
    }

    #[test]
    fn exact_bin_edge_goes_into_bin_on_its_right() {
        let report = evaluate(&dataset(
            vec![
                observation("o1", "e1", "s1", 99_999, BinaryOutcome::Negative),
                observation("o2", "e2", "s2", 100_000, BinaryOutcome::Negative),
            ],
            10,
        ))
        .unwrap();
        assert_eq!(report.bins[0].observation_count, 1);
        assert_eq!(report.bins[1].observation_count, 1);
    }

    #[test]
    fn all_positive_and_all_negative_strata_are_well_defined() {
        let positive = evaluate(&dataset(
            vec![
                observation("p1", "e1", "s1", 800_000, BinaryOutcome::Positive),
                observation("p2", "e2", "s2", 900_000, BinaryOutcome::Positive),
            ],
            4,
        ))
        .unwrap();
        let negative = evaluate(&dataset(
            vec![
                observation("n1", "e1", "s1", 100_000, BinaryOutcome::Negative),
                observation("n2", "e2", "s2", 200_000, BinaryOutcome::Negative),
            ],
            4,
        ))
        .unwrap();
        assert_eq!(positive.observation_count, 2);
        assert_eq!(negative.observation_count, 2);
    }

    #[test]
    fn observations_from_one_session_do_not_inflate_independent_unit_count() {
        let report = evaluate(&dataset(
            vec![
                observation(
                    "o1",
                    "episode-a",
                    "session-a",
                    800_000,
                    BinaryOutcome::Positive,
                ),
                observation(
                    "o2",
                    "episode-a",
                    "session-a",
                    700_000,
                    BinaryOutcome::Positive,
                ),
                observation(
                    "o3",
                    "episode-b",
                    "session-a",
                    600_000,
                    BinaryOutcome::Negative,
                ),
            ],
            5,
        ))
        .unwrap();
        assert_eq!(report.observation_count, 3);
        assert_eq!(report.episode_count, 2);
        assert_eq!(report.independent_unit_count, 1);
    }

    #[test]
    fn duplicate_observation_id_is_rejected() {
        let data = dataset(
            vec![
                observation("same", "e1", "s1", 500_000, BinaryOutcome::Positive),
                observation("same", "e2", "s2", 500_000, BinaryOutcome::Positive),
            ],
            5,
        );
        assert_eq!(
            evaluate(&data),
            Err(MetricError::DuplicateObservationId("same".to_owned()))
        );
    }

    #[test]
    fn episode_cannot_map_to_multiple_exchangeability_units() {
        let data = dataset(
            vec![
                observation(
                    "o1",
                    "same-episode",
                    "session-a",
                    500_000,
                    BinaryOutcome::Positive,
                ),
                observation(
                    "o2",
                    "same-episode",
                    "session-b",
                    500_000,
                    BinaryOutcome::Positive,
                ),
            ],
            5,
        );
        assert_eq!(
            evaluate(&data),
            Err(MetricError::EpisodeMapsToMultipleUnits(
                "same-episode".to_owned()
            ))
        );
    }

    #[test]
    fn fit_or_test_observation_leakage_is_rejected() {
        let data = dataset(
            vec![observation(
                "fit-observation-01",
                "e1",
                "s1",
                500_000,
                BinaryOutcome::Positive,
            )],
            5,
        );
        assert_eq!(
            evaluate(&data),
            Err(MetricError::CalibrationLeakage(
                "fit-observation-01".to_owned()
            ))
        );
    }

    #[test]
    fn mixed_channel_target_model_domain_or_split_is_rejected() {
        let mut a = observation("o1", "e1", "s1", 500_000, BinaryOutcome::Positive);
        a.channel = ContactChannel::ProprioceptiveResidual;
        assert_eq!(
            evaluate(&dataset(vec![a], 5)),
            Err(MetricError::MixedChannel("o1".to_owned()))
        );

        let mut a = observation("o1", "e1", "s1", 500_000, BinaryOutcome::Positive);
        a.target = ContactTarget::TractionFeasibility;
        assert_eq!(
            evaluate(&dataset(vec![a], 5)),
            Err(MetricError::MixedTarget("o1".to_owned()))
        );

        let mut a = observation("o1", "e1", "s1", 500_000, BinaryOutcome::Positive);
        a.model_digest = "other".to_owned();
        assert_eq!(
            evaluate(&dataset(vec![a], 5)),
            Err(MetricError::MixedModelDigest("o1".to_owned()))
        );

        let mut a = observation("o1", "e1", "s1", 500_000, BinaryOutcome::Positive);
        a.domain_digest = "new-terrain".to_owned();
        assert_eq!(
            evaluate(&dataset(vec![a], 5)),
            Err(MetricError::MixedDomainDigest("o1".to_owned()))
        );

        let mut a = observation("o1", "e1", "s1", 500_000, BinaryOutcome::Positive);
        a.calibration_split_digest = "other-split".to_owned();
        assert_eq!(
            evaluate(&dataset(vec![a], 5)),
            Err(MetricError::MixedCalibrationSplitDigest("o1".to_owned()))
        );
    }

    #[test]
    fn invalid_probability_and_bin_count_are_rejected() {
        let mut a = observation("o1", "e1", "s1", 500_000, BinaryOutcome::Positive);
        a.predicted_probability_ppm = SCALE_PPM + 1;
        assert_eq!(
            evaluate(&dataset(vec![a], 5)),
            Err(MetricError::InvalidProbability("o1".to_owned()))
        );

        let data = dataset(
            vec![observation(
                "o1",
                "e1",
                "s1",
                500_000,
                BinaryOutcome::Positive,
            )],
            0,
        );
        assert_eq!(evaluate(&data), Err(MetricError::InvalidBinCount));
        let data = dataset(
            vec![observation(
                "o1",
                "e1",
                "s1",
                500_000,
                BinaryOutcome::Positive,
            )],
            MAX_BIN_COUNT + 1,
        );
        assert_eq!(evaluate(&data), Err(MetricError::InvalidBinCount));
    }

    #[test]
    fn empty_bins_are_retained_in_the_reliability_diagram() {
        let report = evaluate(&dataset(
            vec![observation(
                "o1",
                "e1",
                "s1",
                100_000,
                BinaryOutcome::Negative,
            )],
            10,
        ))
        .unwrap();
        assert_eq!(report.bins.len(), 10);
        assert_eq!(report.bins[9].observation_count, 0);
    }

    #[test]
    fn source_order_does_not_change_any_metric() {
        let a = observation("o1", "e1", "s1", 200_000, BinaryOutcome::Negative);
        let b = observation("o2", "e2", "s2", 800_000, BinaryOutcome::Positive);
        let first = evaluate(&dataset(vec![a.clone(), b.clone()], 5)).unwrap();
        let second = evaluate(&dataset(vec![b, a], 5)).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn checked_accumulator_rejects_overflow() {
        assert_eq!(
            checked_square_accumulate(u128::MAX, 1),
            Err(MetricError::ArithmeticOverflow)
        );
        assert_eq!(
            checked_add(u128::MAX, 1),
            Err(MetricError::ArithmeticOverflow)
        );
        assert_eq!(
            checked_mul(u128::MAX, 2),
            Err(MetricError::ArithmeticOverflow)
        );
    }
}
