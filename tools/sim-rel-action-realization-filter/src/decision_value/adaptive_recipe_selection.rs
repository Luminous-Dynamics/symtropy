//! Deterministic Development-only adaptive selection of scientific challenge recipes.
//!
//! The selector is a pure, replayable decision rule. It never consumes
//! Calibration or SealedHoldout outcomes and it preserves per-agent diagnostics.

use super::counterexample_ledger::EvidenceSplit;
use super::scientific_challenge::ChallengeSplitRegistry;
use crate::Rational;
use std::collections::{BTreeMap, BTreeSet};

const MAX_ID_BYTES: usize = 256;
const CONFIDENCE_SCALE: u128 = 1_000_000;
const HOEFFDING_NUMERATOR: u128 = 2_000_000_000_000; // 2 * scale^2

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChallengeTaskValidity {
    Valid,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentChallengeOutcome {
    Solved,
    NotSolved,
    Abstained,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevelopmentOutcomeReceipt {
    pub attempt_id: String,
    pub task_id: String,
    pub recipe_id: String,
    pub recipe_version: String,
    /// Registry-supplied immutable recipe fingerprint. The selector verifies
    /// identity consistency but does not calculate or authenticate this digest.
    pub recipe_digest: String,
    pub mechanism_stratum: String,
    pub family_group_id: String,
    pub split: EvidenceSplit,
    pub agent_id: String,
    pub task_validity: ChallengeTaskValidity,
    /// Invalid tasks must have no solver outcome. Valid tasks must have one.
    pub outcome: Option<AgentChallengeOutcome>,
    pub evidence_artifact_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScientificRecipeDescriptor {
    pub recipe_id: String,
    pub recipe_version: String,
    pub recipe_digest: String,
    pub mechanism_stratum: String,
    pub difficulty_rank: u32,
    pub resource_units: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StratumCoverageTarget {
    pub mechanism_stratum: String,
    pub minimum_unique_valid_groups: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptiveRecipeSelectorConfig {
    pub minimum_valid_attempts_per_agent: usize,
    pub target_success_rate_lower: Rational,
    pub target_success_rate_upper: Rational,
    /// This is a fixed, pre-registered evaluation panel, not an adaptive field.
    pub agent_panel: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactConfidenceInterval {
    pub lower: Rational,
    pub upper: Rational,
    pub confidence_level_percent: u8,
    pub method: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentDifficultySummary {
    pub agent_id: String,
    pub valid_attempts: usize,
    pub solved: usize,
    pub abstained: usize,
    pub interval: Option<ExactConfidenceInterval>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecipeDifficultySummary {
    pub recipe_id: String,
    pub recipe_version: String,
    pub mechanism_stratum: String,
    pub difficulty_rank: u32,
    pub valid_attempts: usize,
    pub solved: usize,
    pub abstained: usize,
    pub invalid_task_attempts: usize,
    pub pooled_interval: Option<ExactConfidenceInterval>,
    pub per_agent: Vec<AgentDifficultySummary>,
    pub sample_deficit: usize,
    pub distance_from_target_band: Option<Rational>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StratumCoverageReceipt {
    pub mechanism_stratum: String,
    pub observed_unique_valid_groups: usize,
    pub required_unique_valid_groups: usize,
    pub deficit: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecipeSelectionReason {
    CoverageDeficit,
    InsufficientAgentSamples,
    TargetDifficultyBand,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptiveRecipeSelectionReceipt {
    pub selector_version: u32,
    pub prior_development_ledger_digest: String,
    pub prior_development_event_count: u64,
    pub selector_seed: u64,
    /// Deterministic tie-break seed derived from the caller seed and ledger digest.
    /// This is not a cryptographic commitment.
    pub tie_break_seed: u64,
    pub selected_recipe: ScientificRecipeDescriptor,
    pub reason: RecipeSelectionReason,
    pub selected_stratum_coverage: StratumCoverageReceipt,
    pub selected_recipe_summary: RecipeDifficultySummary,
    pub candidate_summaries: Vec<RecipeDifficultySummary>,
    pub invalid_task_attempts_excluded: usize,
    pub development_only: bool,
    pub sealed_outcomes_consulted: bool,
}

#[derive(Clone, Debug, Default)]
struct Counts {
    valid_attempts: usize,
    solved: usize,
    abstained: usize,
    invalid_task_attempts: usize,
}

#[derive(Clone, Debug, Default)]
struct AgentCounts {
    valid_attempts: usize,
    solved: usize,
    abstained: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn index(&mut self, length: usize) -> Result<usize, String> {
        if length == 0 {
            return Err("cannot choose from an empty recipe set".into());
        }
        let bound =
            u64::try_from(length).map_err(|_| "recipe candidate count exceeds u64".to_string())?;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let sample = self.next_u64();
            if sample >= threshold {
                return usize::try_from(sample % bound)
                    .map_err(|_| "selected candidate index exceeds usize".into());
            }
        }
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn increment(value: &mut usize, label: &str) -> Result<(), String> {
    *value = value
        .checked_add(1)
        .ok_or_else(|| format!("{label} count overflow"))?;
    Ok(())
}

fn ceil_div(numerator: u128, denominator: u128) -> u128 {
    numerator / denominator + if numerator % denominator != 0 { 1 } else { 0 }
}

fn integer_sqrt_ceil(value: u128) -> u128 {
    // All callers pass ceil(2e12 / n), which is at most 2e12.
    let mut low = 0u128;
    let mut high = 2_000_001u128;
    while low < high {
        let middle = low + (high - low) / 2;
        if middle.saturating_mul(middle) >= value {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    low
}

/// A conservative, exact-rational interval for a Bernoulli solve rate.
///
/// Hoeffding's inequality gives a two-sided 95% interval with radius
/// sqrt(log(40)/(2n)). Since log(40) < 4, radius sqrt(2/n) is conservative.
/// The integer square-root calculation rounds the radius upward in ppm, so
/// interval construction uses exact integer arithmetic and no floating point.
fn confidence_interval(solved: usize, attempts: usize) -> Result<ExactConfidenceInterval, String> {
    if attempts == 0 || solved > attempts {
        return Err("confidence interval requires 0 <= solved <= attempts and attempts > 0".into());
    }
    let n = u128::try_from(attempts).map_err(|_| "attempt count exceeds u128".to_string())?;
    let solved = u128::try_from(solved).map_err(|_| "success count exceeds u128".to_string())?;
    let radius_square = ceil_div(HOEFFDING_NUMERATOR, n);
    let radius_ppm = integer_sqrt_ceil(radius_square);
    let denominator = n
        .checked_mul(CONFIDENCE_SCALE)
        .ok_or_else(|| "confidence interval denominator overflow".to_string())?;
    let empirical = solved
        .checked_mul(CONFIDENCE_SCALE)
        .ok_or_else(|| "confidence interval numerator overflow".to_string())?;
    let radius = radius_ppm
        .checked_mul(n)
        .ok_or_else(|| "confidence interval radius overflow".to_string())?;
    let lower_numerator = empirical.saturating_sub(radius);
    let upper_numerator = empirical
        .checked_add(radius)
        .ok_or_else(|| "confidence interval upper bound overflow".to_string())?
        .min(denominator);
    Ok(ExactConfidenceInterval {
        lower: Rational::new(lower_numerator, denominator)?,
        upper: Rational::new(upper_numerator, denominator)?,
        confidence_level_percent: 95,
        method: "conservative Hoeffding bound; integer sqrt in ppm".into(),
    })
}

fn interval_distance(
    interval: &ExactConfidenceInterval,
    target_lower: Rational,
    target_upper: Rational,
) -> Result<Rational, String> {
    if interval.upper < target_lower {
        target_lower.checked_sub_nonnegative(interval.upper)
    } else if interval.lower > target_upper {
        interval.lower.checked_sub_nonnegative(target_upper)
    } else {
        Ok(Rational::zero())
    }
}

trait RationalSubtraction {
    fn checked_sub_nonnegative(self, other: Self) -> Result<Self, String>
    where
        Self: Sized;
}

impl RationalSubtraction for Rational {
    fn checked_sub_nonnegative(self, other: Self) -> Result<Self, String> {
        if self < other {
            return Err("non-negative rational subtraction underflow".into());
        }
        let common = gcd(self.denominator(), other.denominator());
        let left_scale = other.denominator() / common;
        let right_scale = self.denominator() / common;
        let left = self
            .numerator()
            .checked_mul(left_scale)
            .ok_or_else(|| "rational subtraction overflow".to_string())?;
        let right = other
            .numerator()
            .checked_mul(right_scale)
            .ok_or_else(|| "rational subtraction overflow".to_string())?;
        let denominator = self
            .denominator()
            .checked_mul(left_scale)
            .ok_or_else(|| "rational subtraction denominator overflow".to_string())?;
        Rational::new(left - right, denominator)
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn validate_config(config: &AdaptiveRecipeSelectorConfig) -> Result<(), String> {
    if config.minimum_valid_attempts_per_agent == 0 {
        return Err("minimum valid attempts per agent must be positive".into());
    }
    if config.target_success_rate_lower > config.target_success_rate_upper
        || !config.target_success_rate_lower.is_probability()
        || !config.target_success_rate_upper.is_probability()
    {
        return Err("target solve-rate interval must be ordered inside [0,1]".into());
    }
    if config.agent_panel.is_empty() {
        return Err("fixed agent panel must not be empty".into());
    }
    let mut agents = BTreeSet::new();
    for agent in &config.agent_panel {
        if !valid_id(agent) || !agents.insert(agent.clone()) {
            return Err("agent panel contains an invalid or duplicate agent ID".into());
        }
    }
    Ok(())
}

fn validate_candidates(candidates: &[ScientificRecipeDescriptor]) -> Result<(), String> {
    if candidates.is_empty() {
        return Err("recipe registry contains no candidates".into());
    }
    let mut identities = BTreeSet::new();
    for candidate in candidates {
        for (value, label) in [
            (&candidate.recipe_id, "recipe ID"),
            (&candidate.recipe_version, "recipe version"),
            (&candidate.recipe_digest, "recipe digest"),
            (&candidate.mechanism_stratum, "mechanism stratum"),
        ] {
            if !valid_id(value) {
                return Err(format!("{label} is empty or overlong"));
            }
        }
        if !identities.insert((
            candidate.recipe_id.clone(),
            candidate.recipe_version.clone(),
        )) {
            return Err("recipe registry contains duplicate ID/version pairs".into());
        }
    }
    Ok(())
}

fn summarize(
    candidate: &ScientificRecipeDescriptor,
    counts: &Counts,
    agents: &BTreeMap<String, AgentCounts>,
    panel: &[String],
    minimum_per_agent: usize,
    target_lower: Rational,
    target_upper: Rational,
) -> Result<RecipeDifficultySummary, String> {
    let pooled_interval = if counts.valid_attempts == 0 {
        None
    } else {
        Some(confidence_interval(counts.solved, counts.valid_attempts)?)
    };
    let distance = pooled_interval
        .as_ref()
        .map(|interval| interval_distance(interval, target_lower, target_upper))
        .transpose()?;

    let mut sample_deficit = 0usize;
    let mut per_agent = Vec::with_capacity(panel.len());
    for agent_id in panel {
        let counts = agents.get(agent_id).cloned().unwrap_or_default();
        sample_deficit = sample_deficit
            .checked_add(minimum_per_agent.saturating_sub(counts.valid_attempts))
            .ok_or_else(|| "agent sample deficit overflow".to_string())?;
        let interval = if counts.valid_attempts == 0 {
            None
        } else {
            Some(confidence_interval(counts.solved, counts.valid_attempts)?)
        };
        per_agent.push(AgentDifficultySummary {
            agent_id: agent_id.clone(),
            valid_attempts: counts.valid_attempts,
            solved: counts.solved,
            abstained: counts.abstained,
            interval,
        });
    }
    Ok(RecipeDifficultySummary {
        recipe_id: candidate.recipe_id.clone(),
        recipe_version: candidate.recipe_version.clone(),
        mechanism_stratum: candidate.mechanism_stratum.clone(),
        difficulty_rank: candidate.difficulty_rank,
        valid_attempts: counts.valid_attempts,
        solved: counts.solved,
        abstained: counts.abstained,
        invalid_task_attempts: counts.invalid_task_attempts,
        pooled_interval,
        per_agent,
        sample_deficit,
        distance_from_target_band: distance,
    })
}

fn digest_seed(selector_seed: u64, ledger_digest: &str) -> u64 {
    // Stable FNV-1a mixing is used only for deterministic tie-breaking, not as
    // an evidence-integrity digest or authentication mechanism.
    let mut hash = 0xcbf2_9ce4_8422_2325u64 ^ selector_seed;
    for byte in ledger_digest.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Select the next immutable recipe using Development outcomes only.
///
/// Invalid tasks remain visible in counts but do not change solve-rate or
/// mechanism-coverage estimates. Every supplied outcome is checked against
/// the pre-registered split registry and the fixed candidate/agent panel.
pub fn select_next_recipe(
    candidates: &[ScientificRecipeDescriptor],
    outcomes: &[DevelopmentOutcomeReceipt],
    coverage_targets: &[StratumCoverageTarget],
    config: &AdaptiveRecipeSelectorConfig,
    split_registry: &ChallengeSplitRegistry,
    prior_development_ledger_digest: &str,
    prior_development_event_count: u64,
    selector_seed: u64,
) -> Result<AdaptiveRecipeSelectionReceipt, String> {
    validate_config(config)?;
    validate_candidates(candidates)?;
    split_registry.validate()?;
    if !valid_id(prior_development_ledger_digest) {
        return Err("prior Development ledger digest is empty or overlong".into());
    }
    let mut target_by_stratum = BTreeMap::new();
    for target in coverage_targets {
        if !valid_id(&target.mechanism_stratum) || target.minimum_unique_valid_groups == 0 {
            return Err("coverage target has an invalid stratum or zero group target".into());
        }
        if target_by_stratum
            .insert(
                target.mechanism_stratum.clone(),
                target.minimum_unique_valid_groups,
            )
            .is_some()
        {
            return Err("coverage targets contain duplicate strata".into());
        }
    }
    let candidate_strata = candidates
        .iter()
        .map(|candidate| candidate.mechanism_stratum.as_str())
        .collect::<BTreeSet<_>>();
    if target_by_stratum
        .keys()
        .any(|stratum| !candidate_strata.contains(stratum.as_str()))
    {
        return Err("coverage target names a stratum without any registered recipe".into());
    }

    let mut candidate_by_identity = BTreeMap::new();
    for candidate in candidates {
        candidate_by_identity.insert(
            (
                candidate.recipe_id.as_str(),
                candidate.recipe_version.as_str(),
            ),
            candidate,
        );
    }
    let allowed_agents = config
        .agent_panel
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut attempt_ids = BTreeSet::new();
    let mut task_agent_attempts = BTreeSet::new();
    let mut recipe_counts = BTreeMap::<(String, String), Counts>::new();
    let mut agent_counts = BTreeMap::<(String, String), BTreeMap<String, AgentCounts>>::new();
    let mut groups_by_stratum = BTreeMap::<String, BTreeSet<String>>::new();
    let mut invalid_task_attempts = 0usize;

    for outcome in outcomes {
        for (value, label) in [
            (&outcome.attempt_id, "attempt ID"),
            (&outcome.task_id, "task ID"),
            (&outcome.recipe_id, "outcome recipe ID"),
            (&outcome.recipe_version, "outcome recipe version"),
            (&outcome.recipe_digest, "outcome recipe digest"),
            (&outcome.mechanism_stratum, "outcome mechanism stratum"),
            (&outcome.family_group_id, "outcome family-group ID"),
            (&outcome.agent_id, "outcome agent ID"),
            (
                &outcome.evidence_artifact_digest,
                "outcome evidence artifact digest",
            ),
        ] {
            if !valid_id(value) {
                return Err(format!("{label} is empty or overlong"));
            }
        }
        if outcome.split != EvidenceSplit::Development {
            return Err("adaptive selector rejects non-Development outcomes".into());
        }
        split_registry.validate_assignment(outcome.split, &outcome.family_group_id)?;
        if !allowed_agents.contains(outcome.agent_id.as_str()) {
            return Err("outcome agent is outside the fixed pre-registered panel".into());
        }
        if !attempt_ids.insert(outcome.attempt_id.clone()) {
            return Err("Development attempt ID is duplicated".into());
        }
        if !task_agent_attempts.insert((
            outcome.recipe_id.clone(),
            outcome.recipe_version.clone(),
            outcome.task_id.clone(),
            outcome.agent_id.clone(),
        )) {
            return Err("an agent has more than one attempt on the same recipe/task pair".into());
        }
        let identity = (outcome.recipe_id.as_str(), outcome.recipe_version.as_str());
        let candidate = candidate_by_identity.get(&identity).ok_or_else(|| {
            "outcome references a recipe outside the immutable registry".to_string()
        })?;
        if candidate.recipe_digest != outcome.recipe_digest
            || candidate.mechanism_stratum != outcome.mechanism_stratum
        {
            return Err("outcome recipe digest or mechanism stratum differs from registry".into());
        }

        let key = (outcome.recipe_id.clone(), outcome.recipe_version.clone());
        let counts = recipe_counts.entry(key.clone()).or_default();
        match (outcome.task_validity, outcome.outcome) {
            (ChallengeTaskValidity::Invalid, None) => {
                increment(&mut counts.invalid_task_attempts, "invalid task")?;
                increment(&mut invalid_task_attempts, "global invalid task")?;
                continue;
            }
            (ChallengeTaskValidity::Valid, Some(_)) => {}
            _ => return Err("task validity and solver outcome fields are inconsistent".into()),
        }

        increment(&mut counts.valid_attempts, "valid attempt")?;
        groups_by_stratum
            .entry(outcome.mechanism_stratum.clone())
            .or_default()
            .insert(outcome.family_group_id.clone());
        let agent = agent_counts
            .entry(key)
            .or_default()
            .entry(outcome.agent_id.clone())
            .or_default();
        increment(&mut agent.valid_attempts, "agent attempt")?;
        match outcome.outcome.expect("valid outcome checked above") {
            AgentChallengeOutcome::Solved => {
                increment(&mut counts.solved, "solved")?;
                increment(&mut agent.solved, "agent solved")?;
            }
            AgentChallengeOutcome::NotSolved => {}
            AgentChallengeOutcome::Abstained => {
                increment(&mut counts.abstained, "abstained")?;
                increment(&mut agent.abstained, "agent abstained")?;
            }
        }
    }

    let mut coverage = Vec::new();
    for (stratum, required) in &target_by_stratum {
        let observed = groups_by_stratum
            .get(stratum)
            .map(BTreeSet::len)
            .unwrap_or(0);
        coverage.push(StratumCoverageReceipt {
            mechanism_stratum: stratum.clone(),
            observed_unique_valid_groups: observed,
            required_unique_valid_groups: *required,
            deficit: required.saturating_sub(observed),
        });
    }
    let selected_coverage =
        coverage
            .iter()
            .filter(|entry| entry.deficit > 0)
            .max_by(|left, right| {
                let left_n = left.deficit as u128 * right.required_unique_valid_groups as u128;
                let right_n = right.deficit as u128 * left.required_unique_valid_groups as u128;
                left_n
                    .cmp(&right_n)
                    .then_with(|| right.mechanism_stratum.cmp(&left.mechanism_stratum))
            });
    let selected_stratum = selected_coverage.map(|item| item.mechanism_stratum.clone());

    let mut summaries = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let key = (
            candidate.recipe_id.clone(),
            candidate.recipe_version.clone(),
        );
        let counts = recipe_counts.get(&key).cloned().unwrap_or_default();
        let agents = agent_counts.get(&key).cloned().unwrap_or_default();
        summaries.push(summarize(
            candidate,
            &counts,
            &agents,
            &config.agent_panel,
            config.minimum_valid_attempts_per_agent,
            config.target_success_rate_lower,
            config.target_success_rate_upper,
        )?);
    }
    summaries.sort_by(|left, right| {
        left.mechanism_stratum
            .cmp(&right.mechanism_stratum)
            .then_with(|| left.recipe_id.cmp(&right.recipe_id))
            .then_with(|| left.recipe_version.cmp(&right.recipe_version))
    });

    let mut eligible = summaries
        .iter()
        .filter(|summary| {
            selected_stratum
                .as_ref()
                .is_none_or(|stratum| &summary.mechanism_stratum == stratum)
        })
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return Err("no recipe is eligible in the selected coverage stratum".into());
    }
    let has_under_sampled = eligible.iter().any(|item| item.sample_deficit > 0);
    let reason = if selected_stratum.is_some() {
        RecipeSelectionReason::CoverageDeficit
    } else if has_under_sampled {
        RecipeSelectionReason::InsufficientAgentSamples
    } else {
        RecipeSelectionReason::TargetDifficultyBand
    };

    if has_under_sampled {
        let maximum_deficit = eligible
            .iter()
            .map(|item| item.sample_deficit)
            .max()
            .unwrap_or(0);
        eligible.retain(|item| item.sample_deficit == maximum_deficit);
    } else {
        let minimum_distance = eligible
            .iter()
            .filter_map(|item| item.distance_from_target_band)
            .min();
        if let Some(distance) = minimum_distance {
            eligible.retain(|item| item.distance_from_target_band == Some(distance));
        }
    }
    let minimum_resource = eligible
        .iter()
        .map(|item| {
            candidates
                .iter()
                .find(|candidate| {
                    candidate.recipe_id == item.recipe_id
                        && candidate.recipe_version == item.recipe_version
                })
                .map(|candidate| candidate.resource_units)
                .unwrap_or(u64::MAX)
        })
        .min()
        .unwrap_or(u64::MAX);
    eligible.retain(|item| {
        candidates
            .iter()
            .find(|candidate| {
                candidate.recipe_id == item.recipe_id
                    && candidate.recipe_version == item.recipe_version
            })
            .is_some_and(|candidate| candidate.resource_units == minimum_resource)
    });

    let tie_break_seed = digest_seed(selector_seed, prior_development_ledger_digest);
    let mut tie_rng = SplitMix64::new(tie_break_seed);
    let selected_index = tie_rng.index(eligible.len())?;
    let selected_summary = (*eligible[selected_index]).clone();
    let selected_candidate = candidates
        .iter()
        .find(|candidate| {
            candidate.recipe_id == selected_summary.recipe_id
                && candidate.recipe_version == selected_summary.recipe_version
        })
        .ok_or_else(|| "selected recipe disappeared from immutable registry".to_string())?
        .to_owned();

    let selected_stratum_coverage = coverage
        .iter()
        .find(|item| item.mechanism_stratum == selected_candidate.mechanism_stratum)
        .cloned()
        .unwrap_or(StratumCoverageReceipt {
            mechanism_stratum: selected_candidate.mechanism_stratum.clone(),
            observed_unique_valid_groups: groups_by_stratum
                .get(&selected_candidate.mechanism_stratum)
                .map(BTreeSet::len)
                .unwrap_or(0),
            required_unique_valid_groups: 0,
            deficit: 0,
        });

    Ok(AdaptiveRecipeSelectionReceipt {
        selector_version: 1,
        prior_development_ledger_digest: prior_development_ledger_digest.into(),
        prior_development_event_count,
        selector_seed,
        tie_break_seed,
        selected_recipe: selected_candidate,
        reason,
        selected_stratum_coverage,
        selected_recipe_summary: selected_summary.clone(),
        candidate_summaries: summaries,
        invalid_task_attempts_excluded: invalid_task_attempts,
        development_only: true,
        sealed_outcomes_consulted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::super::counterexample_ledger::EvidenceSplit;
    use super::super::scientific_challenge::ChallengeSplitRegistry;
    use super::{
        AdaptiveRecipeSelectorConfig, AgentChallengeOutcome, ChallengeTaskValidity,
        DevelopmentOutcomeReceipt, RecipeSelectionReason, ScientificRecipeDescriptor,
        StratumCoverageTarget, confidence_interval, select_next_recipe,
    };
    use crate::Rational;

    fn q(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn registry() -> ChallengeSplitRegistry {
        ChallengeSplitRegistry {
            development_group_ids: vec!["dev-a".into(), "dev-b".into(), "dev-c".into()],
            calibration_group_ids: vec!["cal-a".into()],
            sealed_holdout_group_ids: vec!["sealed-a".into()],
        }
    }

    fn candidates() -> Vec<ScientificRecipeDescriptor> {
        vec![
            ScientificRecipeDescriptor {
                recipe_id: "sensor-shift-easy".into(),
                recipe_version: "v1".into(),
                recipe_digest: "digest-easy-v1".into(),
                mechanism_stratum: "sensor-fault".into(),
                difficulty_rank: 1,
                resource_units: 1,
            },
            ScientificRecipeDescriptor {
                recipe_id: "sensor-shift-hard".into(),
                recipe_version: "v1".into(),
                recipe_digest: "digest-hard-v1".into(),
                mechanism_stratum: "sensor-fault".into(),
                difficulty_rank: 2,
                resource_units: 1,
            },
            ScientificRecipeDescriptor {
                recipe_id: "omitted-physics".into(),
                recipe_version: "v1".into(),
                recipe_digest: "digest-physics-v1".into(),
                mechanism_stratum: "model-adequacy".into(),
                difficulty_rank: 1,
                resource_units: 2,
            },
        ]
    }

    fn config() -> AdaptiveRecipeSelectorConfig {
        AdaptiveRecipeSelectorConfig {
            minimum_valid_attempts_per_agent: 2,
            target_success_rate_lower: q(35, 100),
            target_success_rate_upper: q(70, 100),
            agent_panel: vec!["agent-a".into(), "agent-b".into()],
        }
    }

    fn receipt(
        attempt: &str,
        task: &str,
        group: &str,
        recipe: &str,
        digest: &str,
        stratum: &str,
        agent: &str,
        outcome: AgentChallengeOutcome,
    ) -> DevelopmentOutcomeReceipt {
        DevelopmentOutcomeReceipt {
            attempt_id: attempt.into(),
            task_id: task.into(),
            recipe_id: recipe.into(),
            recipe_version: "v1".into(),
            recipe_digest: digest.into(),
            mechanism_stratum: stratum.into(),
            family_group_id: group.into(),
            split: EvidenceSplit::Development,
            agent_id: agent.into(),
            task_validity: ChallengeTaskValidity::Valid,
            outcome: Some(outcome),
            evidence_artifact_digest: format!("artifact-{attempt}"),
        }
    }

    #[test]
    fn exact_hoeffding_interval_is_ordered_and_contains_empirical_rate() {
        let interval = confidence_interval(7, 10).unwrap();
        assert!(interval.lower <= q(7, 10));
        assert!(interval.upper >= q(7, 10));
        assert!(interval.lower <= interval.upper);
        assert_eq!(interval.confidence_level_percent, 95);
        assert!(confidence_interval(0, 0).is_err());
    }

    #[test]
    fn development_outcomes_only_and_stratum_coverage_has_priority() {
        let candidates = candidates();
        let outcomes = vec![
            receipt(
                "a1",
                "task-1",
                "dev-a",
                "sensor-shift-easy",
                "digest-easy-v1",
                "sensor-fault",
                "agent-a",
                AgentChallengeOutcome::Solved,
            ),
            receipt(
                "a2",
                "task-1",
                "dev-a",
                "sensor-shift-easy",
                "digest-easy-v1",
                "sensor-fault",
                "agent-b",
                AgentChallengeOutcome::Solved,
            ),
            receipt(
                "a3",
                "task-2",
                "dev-b",
                "sensor-shift-easy",
                "digest-easy-v1",
                "sensor-fault",
                "agent-a",
                AgentChallengeOutcome::NotSolved,
            ),
            receipt(
                "a4",
                "task-2",
                "dev-b",
                "sensor-shift-easy",
                "digest-easy-v1",
                "sensor-fault",
                "agent-b",
                AgentChallengeOutcome::NotSolved,
            ),
        ];
        let result = select_next_recipe(
            &candidates,
            &outcomes,
            &[
                StratumCoverageTarget {
                    mechanism_stratum: "sensor-fault".into(),
                    minimum_unique_valid_groups: 3,
                },
                StratumCoverageTarget {
                    mechanism_stratum: "model-adequacy".into(),
                    minimum_unique_valid_groups: 3,
                },
            ],
            &config(),
            &registry(),
            "development-ledger-sha256-placeholder",
            4,
            17,
        )
        .unwrap();
        assert_eq!(result.selected_recipe.mechanism_stratum, "model-adequacy");
        assert_eq!(result.reason, RecipeSelectionReason::CoverageDeficit);
        assert!(result.development_only);
        assert!(!result.sealed_outcomes_consulted);
        assert_eq!(result.invalid_task_attempts_excluded, 0);
    }

    #[test]
    fn invalid_tasks_are_excluded_from_difficulty_and_group_coverage() {
        let candidates = candidates();
        let mut invalid = receipt(
            "invalid-1",
            "bad-task",
            "dev-a",
            "sensor-shift-easy",
            "digest-easy-v1",
            "sensor-fault",
            "agent-a",
            AgentChallengeOutcome::Solved,
        );
        invalid.task_validity = ChallengeTaskValidity::Invalid;
        invalid.outcome = None;
        let result = select_next_recipe(
            &candidates,
            &[invalid],
            &[StratumCoverageTarget {
                mechanism_stratum: "sensor-fault".into(),
                minimum_unique_valid_groups: 2,
            }],
            &config(),
            &registry(),
            "ledger-digest",
            1,
            3,
        )
        .unwrap();
        assert_eq!(result.invalid_task_attempts_excluded, 1);
        assert_eq!(result.selected_recipe_summary.valid_attempts, 0);
        assert_eq!(
            result
                .selected_stratum_coverage
                .observed_unique_valid_groups,
            0
        );
    }

    #[test]
    fn calibration_and_holdout_outcomes_are_rejected_not_ignored() {
        for (split, group) in [
            (EvidenceSplit::Calibration, "cal-a"),
            (EvidenceSplit::SealedHoldout, "sealed-a"),
        ] {
            let mut outcome = receipt(
                "leak",
                "task-leak",
                group,
                "sensor-shift-easy",
                "digest-easy-v1",
                "sensor-fault",
                "agent-a",
                AgentChallengeOutcome::Solved,
            );
            outcome.split = split;
            assert!(
                select_next_recipe(
                    &candidates(),
                    &[outcome],
                    &[StratumCoverageTarget {
                        mechanism_stratum: "sensor-fault".into(),
                        minimum_unique_valid_groups: 1,
                    }],
                    &config(),
                    &registry(),
                    "ledger-digest",
                    1,
                    1,
                )
                .is_err()
            );
        }
    }

    #[test]
    fn duplicate_attempts_and_unregistered_agents_fail_closed() {
        let first = receipt(
            "attempt-1",
            "task-1",
            "dev-a",
            "sensor-shift-easy",
            "digest-easy-v1",
            "sensor-fault",
            "agent-a",
            AgentChallengeOutcome::Solved,
        );
        let mut duplicate = first.clone();
        duplicate.task_id = "task-2".into();
        assert!(
            select_next_recipe(
                &candidates(),
                &[first.clone(), duplicate],
                &[],
                &config(),
                &registry(),
                "ledger-digest",
                2,
                5,
            )
            .is_err()
        );

        let mut repeated_task = first.clone();
        repeated_task.attempt_id = "attempt-2".into();
        assert!(
            select_next_recipe(
                &candidates(),
                &[first.clone(), repeated_task],
                &[],
                &config(),
                &registry(),
                "ledger-digest",
                2,
                5,
            )
            .is_err()
        );

        let mut unknown = first;
        unknown.attempt_id = "attempt-unknown-agent".into();
        unknown.agent_id = "agent-c".into();
        assert!(
            select_next_recipe(
                &candidates(),
                &[unknown],
                &[],
                &config(),
                &registry(),
                "ledger-digest",
                1,
                5,
            )
            .is_err()
        );
    }

    #[test]
    fn exact_replay_is_deterministic_and_registry_mutation_is_rejected() {
        let candidates = candidates();
        let outcomes = vec![
            receipt(
                "attempt-1",
                "task-1",
                "dev-a",
                "sensor-shift-easy",
                "digest-easy-v1",
                "sensor-fault",
                "agent-a",
                AgentChallengeOutcome::Solved,
            ),
            receipt(
                "attempt-2",
                "task-1",
                "dev-a",
                "sensor-shift-easy",
                "digest-easy-v1",
                "sensor-fault",
                "agent-b",
                AgentChallengeOutcome::NotSolved,
            ),
        ];
        let first = select_next_recipe(
            &candidates,
            &outcomes,
            &[],
            &config(),
            &registry(),
            "same-ledger-digest",
            2,
            99,
        )
        .unwrap();
        let second = select_next_recipe(
            &candidates,
            &outcomes,
            &[],
            &config(),
            &registry(),
            "same-ledger-digest",
            2,
            99,
        )
        .unwrap();
        assert_eq!(first, second);

        let mut changed = outcomes;
        changed[0].recipe_digest = "mutated-digest".into();
        assert!(
            select_next_recipe(
                &candidates,
                &changed,
                &[],
                &config(),
                &registry(),
                "same-ledger-digest",
                2,
                99,
            )
            .is_err()
        );
    }
}
