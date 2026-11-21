//! Episode-level split-conformal calibration for state-estimation uncertainty.
//!
//! One maximum nonconformity score is consumed per independent session/trial
//! block. Within-block episode, foot and timestep correlation is retained by
//! reducing the complete declared block to one maximum score.
//!
//! The caller owns score construction. This crate only validates identities,
//! split leakage, independent-unit cardinality and the exact conformal rank.
//! Its statistical guarantee is marginal coverage under exchangeability of
//! the declared blocks within the exact frozen operating domain.

use std::collections::BTreeMap;

pub const SCALE_PPM: u32 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConformalError {
    EmptyIdentity,
    InvalidAlpha,
    EmptyCalibrationBlocks,
    EmptyExchangeabilityUnitId,
    DuplicateExchangeabilityUnit(String),
    EmptyEpisodeId,
    DuplicateEpisodeId(String),
    EmptyEpisodeSet,
    EmptyForbiddenEpisodeId,
    DuplicateForbiddenEpisodeId(String),
    CalibrationLeakage(String),
    MixedModelDigest(String),
    MixedTargetDigest(String),
    MixedDomainDigest(String),
    MixedScoreDefinitionDigest(String),
    MixedCalibrationSplitDigest(String),
    ArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EpisodeOutcome {
    Completed,
    Failed,
    Aborted,
}

/// Outcomes are retained for completed, failed and aborted episodes. No
/// episode is silently filtered from a calibration block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeRecord {
    pub episode_id: String,
    pub outcome: EpisodeOutcome,
}

/// One independent exchangeability unit (normally a session or trial block).
/// The score must be the maximum over the frozen target across all timesteps,
/// feet, dimensions and episodes listed in this block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationBlock {
    pub exchangeability_unit_id: String,
    pub episodes: Vec<EpisodeRecord>,
    pub maximum_nonconformity_score: u64,
    pub model_digest: String,
    pub target_digest: String,
    pub domain_digest: String,
    pub score_definition_digest: String,
    pub calibration_split_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationDataset {
    pub model_digest: String,
    pub target_digest: String,
    pub domain_digest: String,
    pub score_definition_digest: String,
    pub calibration_split_digest: String,
    /// Miscoverage alpha in parts per million, strictly between 0 and 1e6.
    pub alpha_ppm: u32,
    pub blocks: Vec<CalibrationBlock>,
    /// Complete ID manifest for fit and final-test episodes. Calibration
    /// overlap is rejected. The caller must supply this complete manifest.
    pub forbidden_episode_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformalThreshold {
    Finite(u64),
    /// The finite-sample rank is n+1; no finite threshold attains the
    /// requested distribution-free coverage at this calibration size.
    Unbounded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationReceipt {
    pub model_digest: String,
    pub target_digest: String,
    pub domain_digest: String,
    pub score_definition_digest: String,
    pub calibration_split_digest: String,
    pub alpha_ppm: u32,
    pub independent_unit_count: usize,
    pub episode_count: usize,
    /// One-based order-statistic rank. This can be n+1.
    pub rank_1based: u128,
    pub threshold: ConformalThreshold,
    pub exchangeability_unit: String,
    pub coverage_claim: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationMismatch {
    ModelDigest,
    TargetDigest,
    DomainDigest,
    ScoreDefinitionDigest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationIdentity<'a> {
    pub model_digest: &'a str,
    pub target_digest: &'a str,
    pub domain_digest: &'a str,
    pub score_definition_digest: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationDecision {
    Applicable { threshold: ConformalThreshold },
    NotApplicable { reason: ApplicationMismatch },
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_identity(value: &str) -> Result<(), ConformalError> {
    if nonblank(value) {
        Ok(())
    } else {
        Err(ConformalError::EmptyIdentity)
    }
}

fn ceil_div(numerator: u128, denominator: u128) -> Result<u128, ConformalError> {
    let quotient = numerator / denominator;
    if numerator.is_multiple_of(denominator) {
        Ok(quotient)
    } else {
        quotient
            .checked_add(1)
            .ok_or(ConformalError::ArithmeticOverflow)
    }
}

fn conformal_rank(n: u128, alpha_ppm: u32) -> Result<u128, ConformalError> {
    if alpha_ppm == 0 || alpha_ppm >= SCALE_PPM {
        return Err(ConformalError::InvalidAlpha);
    }
    let n_plus_one = n.checked_add(1).ok_or(ConformalError::ArithmeticOverflow)?;
    let retained_mass = u128::from(SCALE_PPM - alpha_ppm);
    let numerator = n_plus_one
        .checked_mul(retained_mass)
        .ok_or(ConformalError::ArithmeticOverflow)?;
    ceil_div(numerator, u128::from(SCALE_PPM))
}

/// Exact split-conformal order statistic: `k = ceil((n + 1) * (1 - alpha))`.
///
/// When k > n, this returns an unbounded threshold rather than incorrectly
/// clamping to the maximum observed calibration score.
pub fn calibrate(dataset: &CalibrationDataset) -> Result<CalibrationReceipt, ConformalError> {
    validate_identity(&dataset.model_digest)?;
    validate_identity(&dataset.target_digest)?;
    validate_identity(&dataset.domain_digest)?;
    validate_identity(&dataset.score_definition_digest)?;
    validate_identity(&dataset.calibration_split_digest)?;

    if dataset.alpha_ppm == 0 || dataset.alpha_ppm >= SCALE_PPM {
        return Err(ConformalError::InvalidAlpha);
    }
    if dataset.blocks.is_empty() {
        return Err(ConformalError::EmptyCalibrationBlocks);
    }

    let mut block_ids = BTreeMap::<String, ()>::new();
    let mut episode_ids = BTreeMap::<String, ()>::new();
    let mut forbidden_ids = BTreeMap::<String, ()>::new();
    let mut scores = Vec::with_capacity(dataset.blocks.len());
    let mut episode_count = 0usize;

    for id in &dataset.forbidden_episode_ids {
        if !nonblank(id) {
            return Err(ConformalError::EmptyForbiddenEpisodeId);
        }
        if forbidden_ids.insert(id.clone(), ()).is_some() {
            return Err(ConformalError::DuplicateForbiddenEpisodeId(id.clone()));
        }
    }

    for block in &dataset.blocks {
        if !nonblank(&block.exchangeability_unit_id) {
            return Err(ConformalError::EmptyExchangeabilityUnitId);
        }
        if block_ids
            .insert(block.exchangeability_unit_id.clone(), ())
            .is_some()
        {
            return Err(ConformalError::DuplicateExchangeabilityUnit(
                block.exchangeability_unit_id.clone(),
            ));
        }
        if block.episodes.is_empty() {
            return Err(ConformalError::EmptyEpisodeSet);
        }

        if block.model_digest != dataset.model_digest {
            return Err(ConformalError::MixedModelDigest(
                block.exchangeability_unit_id.clone(),
            ));
        }
        if block.target_digest != dataset.target_digest {
            return Err(ConformalError::MixedTargetDigest(
                block.exchangeability_unit_id.clone(),
            ));
        }
        if block.domain_digest != dataset.domain_digest {
            return Err(ConformalError::MixedDomainDigest(
                block.exchangeability_unit_id.clone(),
            ));
        }
        if block.score_definition_digest != dataset.score_definition_digest {
            return Err(ConformalError::MixedScoreDefinitionDigest(
                block.exchangeability_unit_id.clone(),
            ));
        }
        if block.calibration_split_digest != dataset.calibration_split_digest {
            return Err(ConformalError::MixedCalibrationSplitDigest(
                block.exchangeability_unit_id.clone(),
            ));
        }

        for episode in &block.episodes {
            if !nonblank(&episode.episode_id) {
                return Err(ConformalError::EmptyEpisodeId);
            }
            if episode_ids.insert(episode.episode_id.clone(), ()).is_some() {
                return Err(ConformalError::DuplicateEpisodeId(
                    episode.episode_id.clone(),
                ));
            }
            if forbidden_ids.contains_key(&episode.episode_id) {
                return Err(ConformalError::CalibrationLeakage(
                    episode.episode_id.clone(),
                ));
            }
            episode_count = episode_count
                .checked_add(1)
                .ok_or(ConformalError::ArithmeticOverflow)?;
        }
        scores.push(block.maximum_nonconformity_score);
    }

    let n = u128::try_from(scores.len()).map_err(|_| ConformalError::ArithmeticOverflow)?;
    let rank = conformal_rank(n, dataset.alpha_ppm)?;

    let threshold = if rank > n {
        ConformalThreshold::Unbounded
    } else {
        scores.sort_unstable();
        ConformalThreshold::Finite(scores[(rank - 1) as usize])
    };

    Ok(CalibrationReceipt {
        model_digest: dataset.model_digest.clone(),
        target_digest: dataset.target_digest.clone(),
        domain_digest: dataset.domain_digest.clone(),
        score_definition_digest: dataset.score_definition_digest.clone(),
        calibration_split_digest: dataset.calibration_split_digest.clone(),
        alpha_ppm: dataset.alpha_ppm,
        independent_unit_count: dataset.blocks.len(),
        episode_count,
        rank_1based: rank,
        threshold,
        exchangeability_unit: "declared independent session/trial block".to_owned(),
        coverage_claim: "finite-sample marginal whole-unit coverage under exchangeability in the exact declared domain; not safety or arbitrary-shift coverage".to_owned(),
    })
}

/// A receipt applies only to the exact model, target, domain and score
/// definition from which it was calibrated.
pub fn check_application(
    receipt: &CalibrationReceipt,
    identity: ApplicationIdentity<'_>,
) -> ApplicationDecision {
    if receipt.model_digest != identity.model_digest {
        return ApplicationDecision::NotApplicable {
            reason: ApplicationMismatch::ModelDigest,
        };
    }
    if receipt.target_digest != identity.target_digest {
        return ApplicationDecision::NotApplicable {
            reason: ApplicationMismatch::TargetDigest,
        };
    }
    if receipt.domain_digest != identity.domain_digest {
        return ApplicationDecision::NotApplicable {
            reason: ApplicationMismatch::DomainDigest,
        };
    }
    if receipt.score_definition_digest != identity.score_definition_digest {
        return ApplicationDecision::NotApplicable {
            reason: ApplicationMismatch::ScoreDefinitionDigest,
        };
    }
    ApplicationDecision::Applicable {
        threshold: receipt.threshold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(unit: &str, episodes: &[(&str, EpisodeOutcome)], score: u64) -> CalibrationBlock {
        CalibrationBlock {
            exchangeability_unit_id: unit.to_owned(),
            episodes: episodes
                .iter()
                .map(|(id, outcome)| EpisodeRecord {
                    episode_id: (*id).to_owned(),
                    outcome: *outcome,
                })
                .collect(),
            maximum_nonconformity_score: score,
            model_digest: "model-01".to_owned(),
            target_digest: "pose-foot-state-v2".to_owned(),
            domain_digest: "hard-terrain-gait-suite-v1".to_owned(),
            score_definition_digest: "max-normalized-error-all-feet-all-times-v1".to_owned(),
            calibration_split_digest: "calibration-split-01".to_owned(),
        }
    }

    fn dataset(blocks: Vec<CalibrationBlock>, alpha_ppm: u32) -> CalibrationDataset {
        CalibrationDataset {
            model_digest: "model-01".to_owned(),
            target_digest: "pose-foot-state-v2".to_owned(),
            domain_digest: "hard-terrain-gait-suite-v1".to_owned(),
            score_definition_digest: "max-normalized-error-all-feet-all-times-v1".to_owned(),
            calibration_split_digest: "calibration-split-01".to_owned(),
            alpha_ppm,
            blocks,
            forbidden_episode_ids: vec!["fit-episode-01".to_owned(), "test-episode-01".to_owned()],
        }
    }

    #[test]
    fn n19_alpha_five_percent_uses_rank_19() {
        let blocks = (0..19)
            .map(|i| {
                let episode = format!("episode-{i}");
                block(
                    &format!("session-{i}"),
                    &[(episode.as_str(), EpisodeOutcome::Completed)],
                    i as u64,
                )
            })
            .collect::<Vec<_>>();
        let receipt = calibrate(&dataset(blocks, 50_000)).unwrap();
        assert_eq!(receipt.independent_unit_count, 19);
        assert_eq!(receipt.rank_1based, 19);
        assert_eq!(receipt.threshold, ConformalThreshold::Finite(18));
    }

    #[test]
    fn n18_alpha_five_percent_is_unbounded_not_clamped() {
        let blocks = (0..18)
            .map(|i| {
                let episode = format!("episode-{i}");
                block(
                    &format!("session-{i}"),
                    &[(episode.as_str(), EpisodeOutcome::Completed)],
                    i as u64,
                )
            })
            .collect::<Vec<_>>();
        let receipt = calibrate(&dataset(blocks, 50_000)).unwrap();
        assert_eq!(receipt.rank_1based, 19);
        assert_eq!(receipt.threshold, ConformalThreshold::Unbounded);
    }

    #[test]
    fn correlated_episodes_share_one_independent_unit() {
        let blocks = vec![
            block(
                "session-a",
                &[
                    ("episode-a1", EpisodeOutcome::Completed),
                    ("episode-a2", EpisodeOutcome::Failed),
                ],
                90,
            ),
            block(
                "session-b",
                &[
                    ("episode-b1", EpisodeOutcome::Completed),
                    ("episode-b2", EpisodeOutcome::Aborted),
                    ("episode-b3", EpisodeOutcome::Completed),
                ],
                40,
            ),
        ];
        let receipt = calibrate(&dataset(blocks, 200_000)).unwrap();
        assert_eq!(receipt.independent_unit_count, 2);
        assert_eq!(receipt.episode_count, 5);
        assert_eq!(receipt.rank_1based, 3);
        assert_eq!(receipt.threshold, ConformalThreshold::Unbounded);
    }

    #[test]
    fn failed_and_aborted_episodes_are_not_filtered() {
        let blocks = vec![
            block("session-a", &[("episode-a", EpisodeOutcome::Failed)], 100),
            block("session-b", &[("episode-b", EpisodeOutcome::Aborted)], 90),
            block("session-c", &[("episode-c", EpisodeOutcome::Completed)], 10),
        ];
        let receipt = calibrate(&dataset(blocks, 500_000)).unwrap();
        assert_eq!(receipt.independent_unit_count, 3);
        assert_eq!(receipt.episode_count, 3);
        assert_eq!(receipt.rank_1based, 2);
        assert_eq!(receipt.threshold, ConformalThreshold::Finite(90));
    }

    #[test]
    fn duplicate_exchangeability_unit_is_rejected() {
        let blocks = vec![
            block(
                "same-session",
                &[("episode-a", EpisodeOutcome::Completed)],
                1,
            ),
            block(
                "same-session",
                &[("episode-b", EpisodeOutcome::Completed)],
                2,
            ),
        ];
        assert_eq!(
            calibrate(&dataset(blocks, 100_000)),
            Err(ConformalError::DuplicateExchangeabilityUnit(
                "same-session".to_owned()
            ))
        );
    }

    #[test]
    fn duplicate_episode_id_is_rejected() {
        let blocks = vec![
            block(
                "session-a",
                &[("episode-same", EpisodeOutcome::Completed)],
                1,
            ),
            block(
                "session-b",
                &[("episode-same", EpisodeOutcome::Completed)],
                2,
            ),
        ];
        assert_eq!(
            calibrate(&dataset(blocks, 100_000)),
            Err(ConformalError::DuplicateEpisodeId(
                "episode-same".to_owned()
            ))
        );
    }

    #[test]
    fn calibration_test_split_leakage_is_rejected() {
        let blocks = vec![block(
            "session-a",
            &[("test-episode-01", EpisodeOutcome::Completed)],
            1,
        )];
        assert_eq!(
            calibrate(&dataset(blocks, 100_000)),
            Err(ConformalError::CalibrationLeakage(
                "test-episode-01".to_owned()
            ))
        );
    }

    #[test]
    fn mixed_domain_model_target_score_or_split_is_rejected() {
        let mut b = block("session-a", &[("episode-a", EpisodeOutcome::Completed)], 1);
        b.domain_digest = "new-terrain".to_owned();
        assert_eq!(
            calibrate(&dataset(vec![b], 100_000)),
            Err(ConformalError::MixedDomainDigest("session-a".to_owned()))
        );

        let mut b = block("session-a", &[("episode-a", EpisodeOutcome::Completed)], 1);
        b.model_digest = "new-model".to_owned();
        assert_eq!(
            calibrate(&dataset(vec![b], 100_000)),
            Err(ConformalError::MixedModelDigest("session-a".to_owned()))
        );

        let mut b = block("session-a", &[("episode-a", EpisodeOutcome::Completed)], 1);
        b.target_digest = "different-target".to_owned();
        assert_eq!(
            calibrate(&dataset(vec![b], 100_000)),
            Err(ConformalError::MixedTargetDigest("session-a".to_owned()))
        );

        let mut b = block("session-a", &[("episode-a", EpisodeOutcome::Completed)], 1);
        b.score_definition_digest = "mean-per-tick-not-episode-max".to_owned();
        assert_eq!(
            calibrate(&dataset(vec![b], 100_000)),
            Err(ConformalError::MixedScoreDefinitionDigest(
                "session-a".to_owned()
            ))
        );

        let mut b = block("session-a", &[("episode-a", EpisodeOutcome::Completed)], 1);
        b.calibration_split_digest = "other-split".to_owned();
        assert_eq!(
            calibrate(&dataset(vec![b], 100_000)),
            Err(ConformalError::MixedCalibrationSplitDigest(
                "session-a".to_owned()
            ))
        );
    }

    #[test]
    fn invalid_alpha_and_empty_identity_are_rejected() {
        let blocks = vec![block(
            "session-a",
            &[("episode-a", EpisodeOutcome::Completed)],
            5,
        )];
        assert_eq!(
            calibrate(&dataset(blocks.clone(), 0)),
            Err(ConformalError::InvalidAlpha)
        );
        assert_eq!(
            calibrate(&dataset(blocks.clone(), SCALE_PPM)),
            Err(ConformalError::InvalidAlpha)
        );

        let mut d = dataset(blocks, 100_000);
        d.domain_digest.clear();
        assert_eq!(calibrate(&d), Err(ConformalError::EmptyIdentity));
    }

    #[test]
    fn ties_and_input_permutation_are_deterministic() {
        let a = block("session-a", &[("episode-a", EpisodeOutcome::Completed)], 7);
        let b = block("session-b", &[("episode-b", EpisodeOutcome::Completed)], 7);
        let c = block("session-c", &[("episode-c", EpisodeOutcome::Completed)], 10);
        let first = calibrate(&dataset(vec![a.clone(), b.clone(), c.clone()], 500_000)).unwrap();
        let second = calibrate(&dataset(vec![c, b, a], 500_000)).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.threshold, ConformalThreshold::Finite(7));
    }

    #[test]
    fn arithmetic_overflow_is_reported_explicitly() {
        assert_eq!(
            conformal_rank(u128::MAX, 50_000),
            Err(ConformalError::ArithmeticOverflow)
        );
    }

    #[test]
    fn exact_rank_boundary_uses_ceiling_integer_arithmetic() {
        let blocks = vec![
            block("session-1", &[("e1", EpisodeOutcome::Completed)], 1),
            block("session-2", &[("e2", EpisodeOutcome::Completed)], 2),
            block("session-3", &[("e3", EpisodeOutcome::Completed)], 3),
        ];
        // n=3, alpha=25%: ceil(4 * 0.75) = 3.
        let receipt = calibrate(&dataset(blocks, 250_000)).unwrap();
        assert_eq!(receipt.rank_1based, 3);
        assert_eq!(receipt.threshold, ConformalThreshold::Finite(3));
    }

    #[test]
    fn all_zero_and_maximum_scores_are_preserved() {
        let zeros = vec![
            block("session-1", &[("e1", EpisodeOutcome::Completed)], 0),
            block("session-2", &[("e2", EpisodeOutcome::Completed)], 0),
            block("session-3", &[("e3", EpisodeOutcome::Completed)], 0),
        ];
        assert_eq!(
            calibrate(&dataset(zeros, 250_000)).unwrap().threshold,
            ConformalThreshold::Finite(0)
        );

        let maxima = vec![
            block("session-1", &[("m1", EpisodeOutcome::Completed)], u64::MAX),
            block("session-2", &[("m2", EpisodeOutcome::Completed)], u64::MAX),
            block("session-3", &[("m3", EpisodeOutcome::Completed)], u64::MAX),
        ];
        assert_eq!(
            calibrate(&dataset(maxima, 250_000)).unwrap().threshold,
            ConformalThreshold::Finite(u64::MAX)
        );
    }

    #[test]
    fn application_rejects_new_domain_and_other_model() {
        let blocks = vec![
            block("session-1", &[("e1", EpisodeOutcome::Completed)], 1),
            block("session-2", &[("e2", EpisodeOutcome::Completed)], 2),
            block("session-3", &[("e3", EpisodeOutcome::Completed)], 3),
        ];
        let receipt = calibrate(&dataset(blocks, 250_000)).unwrap();
        let same = ApplicationIdentity {
            model_digest: "model-01",
            target_digest: "pose-foot-state-v2",
            domain_digest: "hard-terrain-gait-suite-v1",
            score_definition_digest: "max-normalized-error-all-feet-all-times-v1",
        };
        assert_eq!(
            check_application(&receipt, same),
            ApplicationDecision::Applicable {
                threshold: ConformalThreshold::Finite(3)
            }
        );

        let ood = ApplicationIdentity {
            domain_digest: "loose-sand-new-domain",
            ..same
        };
        assert_eq!(
            check_application(&receipt, ood),
            ApplicationDecision::NotApplicable {
                reason: ApplicationMismatch::DomainDigest
            }
        );

        let other_model = ApplicationIdentity {
            model_digest: "other-model",
            ..same
        };
        assert_eq!(
            check_application(&receipt, other_model),
            ApplicationDecision::NotApplicable {
                reason: ApplicationMismatch::ModelDigest
            }
        );
    }
}
