//! Empirical locomotion capability-envelope and cross-fitted tail auditor.
//!
//! This crate evaluates frozen benchmark outcomes. It does not certify
//! continuous-time safety or compute robust viability.

use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EPISODES: usize = 200_000;
pub const PPM_SCALE: u64 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelSuccess {
    pub level: u32,
    pub successes: u64,
    pub trials: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAxis {
    pub axis_id: String,
    pub unit_id: String,
    pub levels: Vec<LevelSuccess>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeStatus {
    QualifiedMonotone,
    NonMonotone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeCensoring {
    Bracketed,
    AllTestedLevelsPass,
    AllTestedLevelsFail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonotonicityViolation {
    pub lower_level: u32,
    pub higher_level: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityEnvelopeReport {
    pub axis_id: String,
    pub unit_id: String,
    pub status: EnvelopeStatus,
    /// Largest tested severity with empirical success rate >= 50%.
    pub largest_tested_passing_level: Option<u32>,
    /// Smallest tested severity with empirical success rate < 50%.
    pub smallest_tested_failing_level: Option<u32>,
    pub censoring: Option<EnvelopeCensoring>,
    pub monotonicity_violations: Vec<MonotonicityViolation>,
    pub raw_levels: Vec<LevelSuccess>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceTier {
    Simulation,
    Hardware,
    Biological,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeOutcome {
    pub episode_id: String,
    pub level_id: String,
    /// Integer reward/loss on the preregistered common unit scale.
    pub reward_units: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossFitProblem {
    pub manifest_digest: String,
    pub morphology_digest: String,
    pub physics_digest: String,
    pub controller_digest: String,
    pub reward_unit_id: String,
    pub evidence_tier: EvidenceTier,
    pub training_level_ids: Vec<String>,
    pub validation_level_ids: Vec<String>,
    pub heldout_level_ids: Vec<String>,
    pub worst_level_count: usize,
    pub tail_fraction_ppm: u64,
    pub rank_a_split_id: String,
    pub score_a_split_id: String,
    pub rank_b_split_id: String,
    pub score_b_split_id: String,
    pub rank_a: Vec<EpisodeOutcome>,
    pub score_a: Vec<EpisodeOutcome>,
    pub rank_b: Vec<EpisodeOutcome>,
    pub score_b: Vec<EpisodeOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldReport {
    pub ranking_split_id: String,
    pub scoring_split_id: String,
    /// Worst levels selected using only the ranking split.
    pub selected_level_ids: Vec<String>,
    pub scoring_episode_count: usize,
    pub tail_episode_count: usize,
    pub tail_reward_sum: i128,
    /// Exact tail mean = numerator / denominator.
    pub tail_mean_numerator: i128,
    pub tail_mean_denominator: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossFitReport {
    pub manifest_digest: String,
    pub morphology_digest: String,
    pub physics_digest: String,
    pub controller_digest: String,
    pub reward_unit_id: String,
    pub evidence_tier: EvidenceTier,
    pub fold_rank_a_score_b: FoldReport,
    pub fold_rank_b_score_a: FoldReport,
    /// Exact arithmetic mean of the two fold-specific CVaR estimates.
    pub combined_cvar_numerator: i128,
    pub combined_cvar_denominator: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityError {
    EmptyIdentifier,
    NoCapabilityLevels,
    ZeroTrials,
    SuccessesExceedTrials,
    DuplicateCapabilityLevel,
    CapabilityLevelsNotStrictlyIncreasing,
    ArithmeticOverflow,
    EmptySplitManifest,
    DuplicateLevelId,
    OverlappingLevelSplits,
    NoHeldoutLevels,
    InvalidWorstLevelCount,
    InvalidTailFraction,
    SplitEpisodeBudgetExceeded,
    DuplicateSplitIdentifier,
    DuplicateEpisodeId,
    EpisodeOutsideHeldoutManifest,
    SplitMissingHeldoutLevel,
    CrossFitEpisodeLeakage,
    EmptyEvaluationSplit,
    MissingRankLevel,
    MissingScoreLevel,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn checked_mul_u128(left: u128, right: u128) -> Result<u128, CapabilityError> {
    left.checked_mul(right)
        .ok_or(CapabilityError::ArithmeticOverflow)
}

fn checked_mul_i128(left: i128, right: i128) -> Result<i128, CapabilityError> {
    left.checked_mul(right)
        .ok_or(CapabilityError::ArithmeticOverflow)
}

fn checked_add_i128(left: i128, right: i128) -> Result<i128, CapabilityError> {
    left.checked_add(right)
        .ok_or(CapabilityError::ArithmeticOverflow)
}

fn at_least_half(successes: u64, trials: u64) -> bool {
    u128::from(successes) * 2 >= u128::from(trials)
}

fn success_rate_increases(
    lower: &LevelSuccess,
    higher: &LevelSuccess,
) -> Result<bool, CapabilityError> {
    let left = checked_mul_u128(u128::from(lower.successes), u128::from(higher.trials))?;
    let right = checked_mul_u128(u128::from(higher.successes), u128::from(lower.trials))?;
    Ok(right > left)
}

/// Estimates a discrete 50%-success bracket without interpolation. The axis
/// is qualified only if empirical success does not increase with severity.
pub fn estimate_capability_envelope(
    axis: &CapabilityAxis,
) -> Result<CapabilityEnvelopeReport, CapabilityError> {
    if !nonempty(&axis.axis_id) || !nonempty(&axis.unit_id) {
        return Err(CapabilityError::EmptyIdentifier);
    }
    if axis.levels.is_empty() {
        return Err(CapabilityError::NoCapabilityLevels);
    }

    let mut levels = axis.levels.clone();
    levels.sort_by_key(|item| item.level);
    for (index, item) in levels.iter().enumerate() {
        if item.trials == 0 {
            return Err(CapabilityError::ZeroTrials);
        }
        if item.successes > item.trials {
            return Err(CapabilityError::SuccessesExceedTrials);
        }
        if index > 0 && levels[index - 1].level == item.level {
            return Err(CapabilityError::DuplicateCapabilityLevel);
        }
    }
    if axis
        .levels
        .windows(2)
        .any(|pair| pair[0].level >= pair[1].level)
    {
        // Canonical input order is part of the evidence contract. Do not
        // silently sort a caller's level ladder.
        return Err(CapabilityError::CapabilityLevelsNotStrictlyIncreasing);
    }

    let mut violations = Vec::new();
    for pair in levels.windows(2) {
        if success_rate_increases(&pair[0], &pair[1])? {
            violations.push(MonotonicityViolation {
                lower_level: pair[0].level,
                higher_level: pair[1].level,
            });
        }
    }

    if !violations.is_empty() {
        return Ok(CapabilityEnvelopeReport {
            axis_id: axis.axis_id.clone(),
            unit_id: axis.unit_id.clone(),
            status: EnvelopeStatus::NonMonotone,
            largest_tested_passing_level: None,
            smallest_tested_failing_level: None,
            censoring: None,
            monotonicity_violations: violations,
            raw_levels: axis.levels.clone(),
        });
    }

    let largest_passing = levels
        .iter()
        .filter(|item| at_least_half(item.successes, item.trials))
        .map(|item| item.level)
        .max();
    let smallest_failing = levels
        .iter()
        .filter(|item| !at_least_half(item.successes, item.trials))
        .map(|item| item.level)
        .min();
    let censoring = match (largest_passing, smallest_failing) {
        (Some(_), Some(_)) => EnvelopeCensoring::Bracketed,
        (Some(_), None) => EnvelopeCensoring::AllTestedLevelsPass,
        (None, Some(_)) => EnvelopeCensoring::AllTestedLevelsFail,
        (None, None) => return Err(CapabilityError::NoCapabilityLevels),
    };

    Ok(CapabilityEnvelopeReport {
        axis_id: axis.axis_id.clone(),
        unit_id: axis.unit_id.clone(),
        status: EnvelopeStatus::QualifiedMonotone,
        largest_tested_passing_level: largest_passing,
        smallest_tested_failing_level: smallest_failing,
        censoring: Some(censoring),
        monotonicity_violations: Vec::new(),
        raw_levels: axis.levels.clone(),
    })
}

fn validate_level_partition<'a>(
    values: &'a [String],
) -> Result<BTreeSet<&'a str>, CapabilityError> {
    if values.is_empty() {
        return Err(CapabilityError::EmptySplitManifest);
    }
    let mut set = BTreeSet::new();
    for value in values {
        if !nonempty(value) {
            return Err(CapabilityError::EmptyIdentifier);
        }
        if !set.insert(value.as_str()) {
            return Err(CapabilityError::DuplicateLevelId);
        }
    }
    Ok(set)
}

fn validate_episode_split<'a>(
    split_id: &str,
    episodes: &'a [EpisodeOutcome],
    heldout_levels: &BTreeSet<&str>,
) -> Result<BTreeSet<&'a str>, CapabilityError> {
    if !nonempty(split_id) {
        return Err(CapabilityError::EmptyIdentifier);
    }
    if episodes.is_empty() {
        return Err(CapabilityError::EmptyEvaluationSplit);
    }
    if episodes.len() > MAX_EPISODES {
        return Err(CapabilityError::SplitEpisodeBudgetExceeded);
    }
    let mut ids = BTreeSet::new();
    let mut levels_seen = BTreeSet::new();
    for episode in episodes {
        if !nonempty(&episode.episode_id) || !nonempty(&episode.level_id) {
            return Err(CapabilityError::EmptyIdentifier);
        }
        if !ids.insert(episode.episode_id.as_str()) {
            return Err(CapabilityError::DuplicateEpisodeId);
        }
        if !heldout_levels.contains(episode.level_id.as_str()) {
            return Err(CapabilityError::EpisodeOutsideHeldoutManifest);
        }
        levels_seen.insert(episode.level_id.as_str());
    }
    if levels_seen != *heldout_levels {
        return Err(CapabilityError::SplitMissingHeldoutLevel);
    }
    Ok(ids)
}

fn average_rank_by_level(
    episodes: &[EpisodeOutcome],
    heldout_levels: &BTreeSet<&str>,
) -> Result<Vec<(String, i128, u64)>, CapabilityError> {
    let mut sums = BTreeMap::<&str, i128>::new();
    let mut counts = BTreeMap::<&str, u64>::new();
    for episode in episodes {
        let sum = sums.get(episode.level_id.as_str()).copied().unwrap_or(0);
        sums.insert(
            episode.level_id.as_str(),
            checked_add_i128(sum, i128::from(episode.reward_units))?,
        );
        let count = counts.get(episode.level_id.as_str()).copied().unwrap_or(0);
        counts.insert(
            episode.level_id.as_str(),
            count
                .checked_add(1)
                .ok_or(CapabilityError::ArithmeticOverflow)?,
        );
    }

    let mut ranked = Vec::with_capacity(heldout_levels.len());
    for level_id in heldout_levels {
        let sum = sums
            .get(level_id)
            .copied()
            .ok_or(CapabilityError::MissingRankLevel)?;
        let count = counts
            .get(level_id)
            .copied()
            .ok_or(CapabilityError::MissingRankLevel)?;
        ranked.push(((*level_id).to_owned(), sum, count));
    }
    // At most MAX_EPISODES observations per split, so the cross-products of
    // an i64 reward sum and a u64 count are well inside the i128 range.
    ranked.sort_by(|left, right| {
        (left.1 * i128::from(right.2))
            .cmp(&(right.1 * i128::from(left.2)))
            .then_with(|| left.0.cmp(&right.0))
    });
    Ok(ranked)
}

fn lower_tail_report(
    ranking_split_id: &str,
    scoring_split_id: &str,
    ranking_episodes: &[EpisodeOutcome],
    scoring_episodes: &[EpisodeOutcome],
    heldout_levels: &BTreeSet<&str>,
    worst_level_count: usize,
    tail_fraction_ppm: u64,
) -> Result<FoldReport, CapabilityError> {
    let ranked = average_rank_by_level(ranking_episodes, heldout_levels)?;
    let selected: BTreeSet<String> = ranked
        .into_iter()
        .take(worst_level_count)
        .map(|item| item.0)
        .collect();
    let mut selected_level_ids: Vec<String> = selected.iter().cloned().collect();
    selected_level_ids.sort();

    let mut score_values: Vec<&EpisodeOutcome> = scoring_episodes
        .iter()
        .filter(|episode| selected.contains(&episode.level_id))
        .collect();
    if score_values.is_empty() {
        return Err(CapabilityError::MissingScoreLevel);
    }
    let score_levels: BTreeSet<&str> = score_values
        .iter()
        .map(|episode| episode.level_id.as_str())
        .collect();
    if selected
        .iter()
        .any(|level| !score_levels.contains(level.as_str()))
    {
        return Err(CapabilityError::MissingScoreLevel);
    }

    score_values.sort_by(|left, right| {
        left.reward_units
            .cmp(&right.reward_units)
            .then_with(|| left.episode_id.cmp(&right.episode_id))
    });

    let scoring_episode_count = score_values.len();
    let numerator = checked_mul_u128(
        u128::try_from(scoring_episode_count).map_err(|_| CapabilityError::ArithmeticOverflow)?,
        u128::from(tail_fraction_ppm),
    )?;
    let tail_episode_count = usize::try_from(numerator.div_ceil(u128::from(PPM_SCALE)))
        .map_err(|_| CapabilityError::ArithmeticOverflow)?
        .max(1);

    let mut tail_sum = 0_i128;
    for episode in score_values.iter().take(tail_episode_count) {
        tail_sum = checked_add_i128(tail_sum, i128::from(episode.reward_units))?;
    }
    let tail_mean_denominator =
        u128::try_from(tail_episode_count).map_err(|_| CapabilityError::ArithmeticOverflow)?;

    Ok(FoldReport {
        ranking_split_id: ranking_split_id.to_owned(),
        scoring_split_id: scoring_split_id.to_owned(),
        selected_level_ids,
        scoring_episode_count,
        tail_episode_count,
        tail_reward_sum: tail_sum,
        tail_mean_numerator: tail_sum,
        tail_mean_denominator,
    })
}

/// Cross-fitted lower-tail score: the level ranking pass and tail-scoring pass
/// are disjoint, and the two roles are reversed for the second fold.
pub fn cross_fitted_cvar25(problem: &CrossFitProblem) -> Result<CrossFitReport, CapabilityError> {
    let identifiers = [
        problem.manifest_digest.as_str(),
        problem.morphology_digest.as_str(),
        problem.physics_digest.as_str(),
        problem.controller_digest.as_str(),
        problem.reward_unit_id.as_str(),
        problem.rank_a_split_id.as_str(),
        problem.score_a_split_id.as_str(),
        problem.rank_b_split_id.as_str(),
        problem.score_b_split_id.as_str(),
    ];
    if identifiers.iter().any(|value| !nonempty(value)) {
        return Err(CapabilityError::EmptyIdentifier);
    }
    if problem.worst_level_count == 0 {
        return Err(CapabilityError::InvalidWorstLevelCount);
    }
    if problem.tail_fraction_ppm == 0 || problem.tail_fraction_ppm > PPM_SCALE {
        return Err(CapabilityError::InvalidTailFraction);
    }

    let training = validate_level_partition(&problem.training_level_ids)?;
    let validation = validate_level_partition(&problem.validation_level_ids)?;
    let heldout = validate_level_partition(&problem.heldout_level_ids)?;
    if heldout.is_empty() {
        return Err(CapabilityError::NoHeldoutLevels);
    }
    if !training.is_disjoint(&validation)
        || !training.is_disjoint(&heldout)
        || !validation.is_disjoint(&heldout)
    {
        return Err(CapabilityError::OverlappingLevelSplits);
    }
    if problem.worst_level_count > heldout.len() {
        return Err(CapabilityError::InvalidWorstLevelCount);
    }

    let split_ids = [
        problem.rank_a_split_id.as_str(),
        problem.score_a_split_id.as_str(),
        problem.rank_b_split_id.as_str(),
        problem.score_b_split_id.as_str(),
    ];
    let mut unique_split_ids = BTreeSet::new();
    if split_ids.iter().any(|id| !unique_split_ids.insert(*id)) {
        return Err(CapabilityError::DuplicateSplitIdentifier);
    }

    let rank_a_ids = validate_episode_split(&problem.rank_a_split_id, &problem.rank_a, &heldout)?;
    let score_a_ids =
        validate_episode_split(&problem.score_a_split_id, &problem.score_a, &heldout)?;
    let rank_b_ids = validate_episode_split(&problem.rank_b_split_id, &problem.rank_b, &heldout)?;
    let score_b_ids =
        validate_episode_split(&problem.score_b_split_id, &problem.score_b, &heldout)?;

    if !rank_a_ids.is_disjoint(&score_b_ids)
        || !rank_b_ids.is_disjoint(&score_a_ids)
        || !rank_a_ids.is_disjoint(&rank_b_ids)
        || !score_a_ids.is_disjoint(&score_b_ids)
        || !rank_a_ids.is_disjoint(&score_a_ids)
        || !rank_b_ids.is_disjoint(&score_b_ids)
    {
        return Err(CapabilityError::CrossFitEpisodeLeakage);
    }

    let fold_rank_a_score_b = lower_tail_report(
        &problem.rank_a_split_id,
        &problem.score_b_split_id,
        &problem.rank_a,
        &problem.score_b,
        &heldout,
        problem.worst_level_count,
        problem.tail_fraction_ppm,
    )?;
    let fold_rank_b_score_a = lower_tail_report(
        &problem.rank_b_split_id,
        &problem.score_a_split_id,
        &problem.rank_b,
        &problem.score_a,
        &heldout,
        problem.worst_level_count,
        problem.tail_fraction_ppm,
    )?;

    let left = checked_mul_i128(
        fold_rank_a_score_b.tail_mean_numerator,
        i128::try_from(fold_rank_b_score_a.tail_mean_denominator)
            .map_err(|_| CapabilityError::ArithmeticOverflow)?,
    )?;
    let right = checked_mul_i128(
        fold_rank_b_score_a.tail_mean_numerator,
        i128::try_from(fold_rank_a_score_b.tail_mean_denominator)
            .map_err(|_| CapabilityError::ArithmeticOverflow)?,
    )?;
    let combined_cvar_numerator = checked_add_i128(left, right)?;
    let combined_cvar_denominator = checked_mul_u128(
        checked_mul_u128(
            fold_rank_a_score_b.tail_mean_denominator,
            fold_rank_b_score_a.tail_mean_denominator,
        )?,
        2,
    )?;

    Ok(CrossFitReport {
        manifest_digest: problem.manifest_digest.clone(),
        morphology_digest: problem.morphology_digest.clone(),
        physics_digest: problem.physics_digest.clone(),
        controller_digest: problem.controller_digest.clone(),
        reward_unit_id: problem.reward_unit_id.clone(),
        evidence_tier: problem.evidence_tier,
        fold_rank_a_score_b,
        fold_rank_b_score_a,
        combined_cvar_numerator,
        combined_cvar_denominator,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(level: u32, successes: u64, trials: u64) -> LevelSuccess {
        LevelSuccess {
            level,
            successes,
            trials,
        }
    }

    fn episode(id: &str, level_id: &str, reward: i64) -> EpisodeOutcome {
        EpisodeOutcome {
            episode_id: id.to_owned(),
            level_id: level_id.to_owned(),
            reward_units: reward,
        }
    }

    fn crossfit_problem() -> CrossFitProblem {
        CrossFitProblem {
            manifest_digest: "manifest-v1".to_owned(),
            morphology_digest: "go2-morphology".to_owned(),
            physics_digest: "mjx-physics-v1".to_owned(),
            controller_digest: "controller-v1".to_owned(),
            reward_unit_id: "reward_milli".to_owned(),
            evidence_tier: EvidenceTier::Simulation,
            training_level_ids: vec!["train-a".to_owned()],
            validation_level_ids: vec!["validation-a".to_owned()],
            heldout_level_ids: vec![
                "level-a".to_owned(),
                "level-b".to_owned(),
                "level-c".to_owned(),
                "level-d".to_owned(),
            ],
            worst_level_count: 2,
            tail_fraction_ppm: 250_000,
            rank_a_split_id: "rank-a".to_owned(),
            score_a_split_id: "score-a".to_owned(),
            rank_b_split_id: "rank-b".to_owned(),
            score_b_split_id: "score-b".to_owned(),
            rank_a: vec![
                episode("ra", "level-a", 100),
                episode("rb", "level-b", 80),
                episode("rc", "level-c", 60),
                episode("rd", "level-d", 40),
            ],
            score_a: vec![
                episode("sa-a1", "level-a", -20),
                episode("sa-a2", "level-a", 0),
                episode("sa-b1", "level-b", 5),
                episode("sa-b2", "level-b", 15),
                episode("sa-c1", "level-c", 40),
                episode("sa-c2", "level-c", 50),
                episode("sa-d1", "level-d", 80),
                episode("sa-d2", "level-d", 90),
            ],
            rank_b: vec![
                episode("rb-a", "level-a", 10),
                episode("rb-b", "level-b", 20),
                episode("rb-c", "level-c", 30),
                episode("rb-d", "level-d", 40),
            ],
            score_b: vec![
                episode("sb-a1", "level-a", 100),
                episode("sb-a2", "level-a", 110),
                episode("sb-b1", "level-b", 90),
                episode("sb-b2", "level-b", 95),
                episode("sb-c1", "level-c", 30),
                episode("sb-c2", "level-c", 40),
                episode("sb-d1", "level-d", 10),
                episode("sb-d2", "level-d", 20),
            ],
        }
    }

    #[test]
    fn monotone_capability_envelope_returns_discrete_50pct_bracket() {
        let report = estimate_capability_envelope(&CapabilityAxis {
            axis_id: "step_height".to_owned(),
            unit_id: "mm".to_owned(),
            levels: vec![
                level(10, 10, 10),
                level(20, 9, 10),
                level(30, 7, 10),
                level(40, 3, 10),
            ],
        })
        .expect("monotone envelope");
        assert_eq!(report.status, EnvelopeStatus::QualifiedMonotone);
        assert_eq!(report.largest_tested_passing_level, Some(30));
        assert_eq!(report.smallest_tested_failing_level, Some(40));
        assert_eq!(report.censoring, Some(EnvelopeCensoring::Bracketed));
    }

    #[test]
    fn nonmonotone_capability_suppresses_single_threshold() {
        let report = estimate_capability_envelope(&CapabilityAxis {
            axis_id: "gap_width".to_owned(),
            unit_id: "mm".to_owned(),
            levels: vec![
                level(10, 9, 10),
                level(20, 4, 10),
                level(30, 8, 10),
                level(40, 2, 10),
            ],
        })
        .expect("nonmonotone surface");
        assert_eq!(report.status, EnvelopeStatus::NonMonotone);
        assert_eq!(report.largest_tested_passing_level, None);
        assert_eq!(report.smallest_tested_failing_level, None);
        assert_eq!(report.monotonicity_violations.len(), 1);
        assert_eq!(report.monotonicity_violations[0].lower_level, 20);
        assert_eq!(report.monotonicity_violations[0].higher_level, 30);
    }

    #[test]
    fn all_passing_levels_are_right_censored() {
        let report = estimate_capability_envelope(&CapabilityAxis {
            axis_id: "slope".to_owned(),
            unit_id: "degrees".to_owned(),
            levels: vec![level(5, 9, 10), level(10, 8, 10)],
        })
        .expect("right censored");
        assert_eq!(
            report.censoring,
            Some(EnvelopeCensoring::AllTestedLevelsPass)
        );
        assert_eq!(report.largest_tested_passing_level, Some(10));
        assert_eq!(report.smallest_tested_failing_level, None);
    }

    #[test]
    fn all_failing_levels_are_left_censored() {
        let report = estimate_capability_envelope(&CapabilityAxis {
            axis_id: "gap".to_owned(),
            unit_id: "cm".to_owned(),
            levels: vec![level(5, 3, 10), level(10, 1, 10)],
        })
        .expect("left censored");
        assert_eq!(
            report.censoring,
            Some(EnvelopeCensoring::AllTestedLevelsFail)
        );
        assert_eq!(report.largest_tested_passing_level, None);
        assert_eq!(report.smallest_tested_failing_level, Some(5));
    }

    #[test]
    fn crossfit_ranks_worst_levels_on_one_split_and_scores_another() {
        let report = cross_fitted_cvar25(&crossfit_problem()).expect("crossfit");
        assert_eq!(
            report.fold_rank_a_score_b.selected_level_ids,
            vec!["level-c".to_owned(), "level-d".to_owned()]
        );
        assert_eq!(report.fold_rank_a_score_b.tail_reward_sum, 10);
        assert_eq!(
            report.fold_rank_b_score_a.selected_level_ids,
            vec!["level-a".to_owned(), "level-b".to_owned()]
        );
        assert_eq!(report.fold_rank_b_score_a.tail_reward_sum, -20);
        assert_eq!(report.combined_cvar_numerator, -10);
        assert_eq!(report.combined_cvar_denominator, 2);
    }

    #[test]
    fn ranking_and_scoring_episode_overlap_is_rejected() {
        let mut problem = crossfit_problem();
        problem.score_b[0].episode_id = problem.rank_a[0].episode_id.clone();
        assert_eq!(
            cross_fitted_cvar25(&problem),
            Err(CapabilityError::CrossFitEpisodeLeakage)
        );
    }

    #[test]
    fn train_and_heldout_level_overlap_is_rejected() {
        let mut problem = crossfit_problem();
        problem.training_level_ids.push("level-a".to_owned());
        assert_eq!(
            cross_fitted_cvar25(&problem),
            Err(CapabilityError::OverlappingLevelSplits)
        );
    }

    #[test]
    fn morphology_and_evidence_tier_remain_in_report_identity() {
        let mut problem = crossfit_problem();
        problem.evidence_tier = EvidenceTier::Hardware;
        let report = cross_fitted_cvar25(&problem).expect("hardware-tier report");
        assert_eq!(report.evidence_tier, EvidenceTier::Hardware);
        assert_eq!(report.morphology_digest, "go2-morphology");
        assert_eq!(report.physics_digest, "mjx-physics-v1");
    }

    #[test]
    fn empty_heldout_level_coverage_is_rejected() {
        let mut problem = crossfit_problem();
        problem.rank_a.pop();
        assert_eq!(
            cross_fitted_cvar25(&problem),
            Err(CapabilityError::SplitMissingHeldoutLevel)
        );
    }

    #[test]
    fn tail_fraction_must_be_positive_and_at_most_one() {
        let mut problem = crossfit_problem();
        problem.tail_fraction_ppm = 0;
        assert_eq!(
            cross_fitted_cvar25(&problem),
            Err(CapabilityError::InvalidTailFraction)
        );
    }
}
