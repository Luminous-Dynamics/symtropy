//! Deterministic recipe-based generation of exact finite scientific decision tasks.
//!
//! Public agent-facing task data, sealed oracle values and generation receipts
//! are separate typed artifacts. The deterministic generator is not
//! cryptographic; opaque task IDs and seed custody are external responsibilities.

use super::approximate_blackwell::verify_approximate_blackwell_dominance;
use super::counterexample_ledger::{EvidenceSplit, ModelFamilyLedgerSnapshot};
use super::experiment_ordering::{
    BlackwellGarblingWitness, ChannelDecisionLossTable, FiniteObservationChannel,
    solve_binary_le_cam_deficiency, solve_binary_task_weighted_deficiency,
};
use crate::Rational;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChallengeObjective {
    ClassicalWorstStateDeficiency,
    PriorLossSpanWeightedDeficiency,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChallengeSplitRegistry {
    pub development_group_ids: Vec<String>,
    pub calibration_group_ids: Vec<String>,
    pub sealed_holdout_group_ids: Vec<String>,
}

impl ChallengeSplitRegistry {
    pub fn from_ledger_snapshot(
        snapshot: &ModelFamilyLedgerSnapshot,
        development_group_ids: Vec<String>,
    ) -> Self {
        Self {
            development_group_ids,
            calibration_group_ids: snapshot
                .reserved_calibration_groups
                .iter()
                .map(|group| group.family_group_id.clone())
                .collect(),
            sealed_holdout_group_ids: snapshot
                .reserved_holdout_groups
                .iter()
                .map(|group| group.family_group_id.clone())
                .collect(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut all_groups = BTreeSet::<String>::new();
        for (label, values) in [
            ("Development", &self.development_group_ids),
            ("Calibration", &self.calibration_group_ids),
            ("SealedHoldout", &self.sealed_holdout_group_ids),
        ] {
            let mut local = BTreeSet::<String>::new();
            for group_id in values {
                validate_id(group_id, "registered family-group ID")?;
                if !local.insert(group_id.clone()) {
                    return Err(format!("{label} group IDs must be unique"));
                }
                if !all_groups.insert(group_id.clone()) {
                    return Err("family-group IDs must be disjoint across evidence splits".into());
                }
            }
        }
        Ok(())
    }

    pub fn validate_assignment(
        &self,
        split: EvidenceSplit,
        family_group_id: &str,
    ) -> Result<(), String> {
        self.validate()?;
        validate_id(family_group_id, "challenge family-group ID")?;
        let allowed = match split {
            EvidenceSplit::Development => &self.development_group_ids,
            EvidenceSplit::Calibration => &self.calibration_group_ids,
            EvidenceSplit::SealedHoldout => &self.sealed_holdout_group_ids,
        };
        if !allowed
            .iter()
            .any(|registered| registered == family_group_id)
        {
            return Err("challenge split/group pair was not pre-registered".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryChannelChallengeRecipe {
    /// Opaque ID for one concrete task instance, not a recipe-family ID.
    pub task_id: String,
    pub recipe_id: String,
    pub recipe_version: String,
    pub generator_id: String,
    pub generator_version: String,
    pub family_group_id: String,
    pub split: EvidenceSplit,
    pub objective: ChallengeObjective,
    pub state_count: usize,
    pub probability_grid: Vec<Rational>,
    pub prior_weight_grid: Vec<Rational>,
    pub loss_value_grid: Vec<Rational>,
    pub abstention_loss_grid: Vec<Rational>,
}

impl BinaryChannelChallengeRecipe {
    pub fn validate(&self, registry: &ChallengeSplitRegistry) -> Result<(), String> {
        for (value, label) in [
            (&self.task_id, "challenge task ID"),
            (&self.recipe_id, "challenge recipe ID"),
            (&self.recipe_version, "challenge recipe version"),
            (&self.generator_id, "challenge generator ID"),
            (&self.generator_version, "challenge generator version"),
            (&self.family_group_id, "challenge family-group ID"),
        ] {
            validate_id(value, label)?;
        }
        registry.validate_assignment(self.split, &self.family_group_id)?;
        if !(2..=13).contains(&self.state_count) {
            return Err("binary-channel challenge state count must be in 2..=13".into());
        }
        validate_probability_grid(&self.probability_grid, "probability grid")?;
        validate_nonnegative_grid(&self.prior_weight_grid, "prior-weight grid")?;
        if self.prior_weight_grid.iter().all(|weight| weight.is_zero()) {
            return Err("prior-weight grid must contain a positive value".into());
        }
        validate_nonnegative_grid(&self.loss_value_grid, "loss-value grid")?;
        validate_nonnegative_grid(&self.abstention_loss_grid, "abstention-loss grid")?;
        Ok(())
    }
}

/// Agent-facing problem statement. It intentionally contains no seed, oracle
/// objective, reference witness or evaluator receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicScientificDecisionTask {
    pub task_id: String,
    pub recipe_id: String,
    pub recipe_version: String,
    pub domain: String,
    pub objective: ChallengeObjective,
    pub source_channel: FiniteObservationChannel,
    pub target_channel: FiniteObservationChannel,
    pub prior: Vec<Rational>,
    pub losses: ChannelDecisionLossTable,
}

/// Evaluator-only exact reference answer. Do not serialize this into the
/// public task payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedScientificDecisionOracle {
    pub task_id: String,
    pub objective: ChallengeObjective,
    pub objective_value: Rational,
    pub optimal_garbling: BlackwellGarblingWitness,
    pub statewise_total_variation: Vec<Rational>,
    pub statewise_loss_spans: Vec<Rational>,
    pub prior_loss_weights: Vec<Rational>,
    pub optimality_verified: bool,
}

/// Evaluator-only provenance for deterministic regeneration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChallengeGenerationReceipt {
    pub task_id: String,
    pub recipe_id: String,
    pub recipe_version: String,
    pub generator_id: String,
    pub generator_version: String,
    pub family_group_id: String,
    pub split: EvidenceSplit,
    pub seed: u64,
    pub generation_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedScientificDecisionChallenge {
    /// Only this field is passed to the agent.
    pub public_task: PublicScientificDecisionTask,
    /// Retained by the evaluator.
    pub sealed_oracle: SealedScientificDecisionOracle,
    /// Retained by the evaluator.
    pub generation_receipt: ChallengeGenerationReceipt,
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
            return Err("cannot sample from an empty recipe grid".into());
        }
        let bound = u64::try_from(length).map_err(|_| "recipe grid length exceeds u64")?;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let sample = self.next_u64();
            if sample >= threshold {
                return usize::try_from(sample % bound)
                    .map_err(|_| "sampled recipe index exceeds usize".into());
            }
        }
    }

    fn sample(&mut self, values: &[Rational]) -> Result<Rational, String> {
        Ok(values[self.index(values.len())?])
    }
}

fn validate_id(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > 256 {
        return Err(format!(
            "{label} must be non-empty and no longer than 256 bytes"
        ));
    }
    Ok(())
}

fn validate_probability_grid(values: &[Rational], label: &str) -> Result<(), String> {
    if values.is_empty() {
        return Err(format!("{label} must be non-empty"));
    }
    if values.iter().any(|value| !value.is_probability()) {
        return Err(format!("{label} contains a value outside [0,1]"));
    }
    Ok(())
}

fn validate_nonnegative_grid(values: &[Rational], label: &str) -> Result<(), String> {
    // Rational stores unsigned numerators and strictly positive denominators,
    // so any constructed Rational is nonnegative.
    if values.is_empty() {
        return Err(format!("{label} must be non-empty"));
    }
    Ok(())
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn subtract_nonnegative(left: Rational, right: Rational) -> Result<Rational, String> {
    if left < right {
        return Err("non-negative rational subtraction would underflow".into());
    }
    let common = gcd(left.denominator(), right.denominator());
    let left_scale = right.denominator() / common;
    let right_scale = left.denominator() / common;
    let left_numerator = left
        .numerator()
        .checked_mul(left_scale)
        .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
    let right_numerator = right
        .numerator()
        .checked_mul(right_scale)
        .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
    let denominator = left
        .denominator()
        .checked_mul(left_scale)
        .ok_or_else(|| "rational subtraction denominator overflow".to_string())?;
    let numerator = left_numerator
        .checked_sub(right_numerator)
        .ok_or_else(|| "rational subtraction underflow".to_string())?;
    Rational::new(numerator, denominator)
}

fn normalize_prior(raw_weights: &[Rational]) -> Result<Vec<Rational>, String> {
    if raw_weights.is_empty() {
        return Err("prior weight vector must be non-empty".into());
    }
    let total = raw_weights
        .iter()
        .copied()
        .try_fold(Rational::zero(), |sum, value| sum.checked_add(value))?;
    if total.is_zero() {
        return Err("prior weights cannot all be zero".into());
    }
    raw_weights
        .iter()
        .map(|weight| weight.checked_div(total))
        .collect()
}

fn state_loss_spans(
    losses: &ChannelDecisionLossTable,
    state_count: usize,
) -> Result<Vec<Rational>, String> {
    if losses.losses.iter().any(|row| row.len() != state_count) {
        return Err("loss table has the wrong hidden-state dimension".into());
    }
    let mut spans = Vec::with_capacity(state_count);
    for state in 0..state_count {
        let mut minimum = losses.abstention_loss;
        let mut maximum = losses.abstention_loss;
        for row in &losses.losses {
            minimum = minimum.min(row[state]);
            maximum = maximum.max(row[state]);
        }
        spans.push(subtract_nonnegative(maximum, minimum)?);
    }
    Ok(spans)
}

fn make_loss_table(
    state_count: usize,
    wrong_decision_losses: &[Rational],
    abstention_loss: Rational,
) -> Result<ChannelDecisionLossTable, String> {
    if wrong_decision_losses.len() != state_count {
        return Err("one wrong-decision loss is required per hidden state".into());
    }
    let mut losses = vec![vec![Rational::zero(); state_count]; state_count];
    for (decision, row) in losses.iter_mut().enumerate() {
        for (state, entry) in row.iter_mut().enumerate() {
            if decision != state {
                *entry = wrong_decision_losses[state];
            }
        }
    }
    let table = ChannelDecisionLossTable {
        decision_ids: (0..state_count)
            .map(|index| format!("declare-state-{index}"))
            .collect(),
        losses,
        abstention_loss,
    };
    table.validate(state_count)?;
    Ok(table)
}

fn channel_from_first_output(
    channel_id: String,
    state_count: usize,
    first_probabilities: &[Rational],
) -> Result<FiniteObservationChannel, String> {
    if first_probabilities.len() != state_count {
        return Err("channel recipe has the wrong number of state probabilities".into());
    }
    let probabilities = first_probabilities
        .iter()
        .map(|first| Ok(vec![*first, subtract_nonnegative(Rational::one(), *first)?]))
        .collect::<Result<Vec<_>, String>>()?;
    let channel = FiniteObservationChannel {
        channel_id,
        state_ids: (0..state_count)
            .map(|index| format!("mechanism-{index}"))
            .collect(),
        observation_ids: vec!["readout-zero".into(), "readout-one".into()],
        probabilities,
    };
    channel.validate()?;
    Ok(channel)
}

fn oracle_from_minimax(
    task: &PublicScientificDecisionTask,
) -> Result<SealedScientificDecisionOracle, String> {
    let solution = solve_binary_le_cam_deficiency(&task.source_channel, &task.target_channel)?;
    let audit = verify_approximate_blackwell_dominance(
        &task.source_channel,
        &task.target_channel,
        &solution.dominance.witness,
        solution.optimal_deficiency,
    )?;
    if !solution.optimality_verified || audit.maximum_state_residual != solution.optimal_deficiency
    {
        return Err("exact minimax solver failed independent residual replay".into());
    }
    let spans = state_loss_spans(&task.losses, task.source_channel.state_ids.len())?;
    let weights = task
        .prior
        .iter()
        .zip(&spans)
        .map(|(probability, span)| probability.checked_mul(*span))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SealedScientificDecisionOracle {
        task_id: task.task_id.clone(),
        objective: task.objective,
        objective_value: solution.optimal_deficiency,
        optimal_garbling: solution.dominance.witness,
        statewise_total_variation: audit
            .state_residuals
            .iter()
            .map(|residual| residual.total_variation)
            .collect(),
        statewise_loss_spans: spans,
        prior_loss_weights: weights,
        optimality_verified: true,
    })
}

fn oracle_from_weighted(
    task: &PublicScientificDecisionTask,
) -> Result<SealedScientificDecisionOracle, String> {
    let solution = solve_binary_task_weighted_deficiency(
        &task.source_channel,
        &task.target_channel,
        &task.prior,
        &task.losses,
    )?;
    let audit = verify_approximate_blackwell_dominance(
        &task.source_channel,
        &task.target_channel,
        &solution.dominance.witness,
        Rational::one(),
    )?;
    let spans = state_loss_spans(&task.losses, task.source_channel.state_ids.len())?;
    let weights = task
        .prior
        .iter()
        .zip(&spans)
        .map(|(probability, span)| probability.checked_mul(*span))
        .collect::<Result<Vec<_>, _>>()?;
    let recomputed = audit
        .state_residuals
        .iter()
        .zip(&weights)
        .try_fold(Rational::zero(), |sum, (residual, weight)| {
            sum.checked_add(weight.checked_mul(residual.total_variation)?)
        })?;
    if !solution.optimality_verified || recomputed != solution.weighted_objective {
        return Err("exact weighted solver failed independent residual/objective replay".into());
    }
    Ok(SealedScientificDecisionOracle {
        task_id: task.task_id.clone(),
        objective: task.objective,
        objective_value: solution.weighted_objective,
        optimal_garbling: solution.dominance.witness,
        statewise_total_variation: audit
            .state_residuals
            .iter()
            .map(|residual| residual.total_variation)
            .collect(),
        statewise_loss_spans: spans,
        prior_loss_weights: weights,
        optimality_verified: true,
    })
}

pub fn generate_binary_channel_challenge(
    recipe: &BinaryChannelChallengeRecipe,
    registry: &ChallengeSplitRegistry,
    seed: u64,
) -> Result<GeneratedScientificDecisionChallenge, String> {
    recipe.validate(registry)?;
    let mut rng = SplitMix64::new(seed);
    let mut source_first = Vec::with_capacity(recipe.state_count);
    let mut target_first = Vec::with_capacity(recipe.state_count);
    let mut raw_prior = Vec::with_capacity(recipe.state_count);
    let mut wrong_losses = Vec::with_capacity(recipe.state_count);

    for _ in 0..recipe.state_count {
        source_first.push(rng.sample(&recipe.probability_grid)?);
        target_first.push(rng.sample(&recipe.probability_grid)?);
        raw_prior.push(rng.sample(&recipe.prior_weight_grid)?);
        wrong_losses.push(rng.sample(&recipe.loss_value_grid)?);
    }
    if raw_prior.iter().all(|weight| weight.is_zero()) {
        let positive = recipe
            .prior_weight_grid
            .iter()
            .copied()
            .find(|weight| !weight.is_zero())
            .ok_or_else(|| "recipe contains no positive prior-weight atom".to_string())?;
        let index = rng.index(recipe.state_count)?;
        raw_prior[index] = positive;
    }
    let prior = normalize_prior(&raw_prior)?;
    let abstention_loss = rng.sample(&recipe.abstention_loss_grid)?;
    let losses = make_loss_table(recipe.state_count, &wrong_losses, abstention_loss)?;

    let public_task = PublicScientificDecisionTask {
        task_id: recipe.task_id.clone(),
        recipe_id: recipe.recipe_id.clone(),
        recipe_version: recipe.recipe_version.clone(),
        domain: "finite-stochastic-experiment-comparison".into(),
        objective: recipe.objective,
        source_channel: channel_from_first_output(
            format!("{}:source", recipe.task_id),
            recipe.state_count,
            &source_first,
        )?,
        target_channel: channel_from_first_output(
            format!("{}:target", recipe.task_id),
            recipe.state_count,
            &target_first,
        )?,
        prior,
        losses,
    };

    let sealed_oracle = match recipe.objective {
        ChallengeObjective::ClassicalWorstStateDeficiency => oracle_from_minimax(&public_task)?,
        ChallengeObjective::PriorLossSpanWeightedDeficiency => oracle_from_weighted(&public_task)?,
    };
    let generation_receipt = ChallengeGenerationReceipt {
        task_id: recipe.task_id.clone(),
        recipe_id: recipe.recipe_id.clone(),
        recipe_version: recipe.recipe_version.clone(),
        generator_id: recipe.generator_id.clone(),
        generator_version: recipe.generator_version.clone(),
        family_group_id: recipe.family_group_id.clone(),
        split: recipe.split,
        seed,
        generation_version: 1,
    };

    Ok(GeneratedScientificDecisionChallenge {
        public_task,
        sealed_oracle,
        generation_receipt,
    })
}

/// Recompute the exact optimum as well as replaying the stored witness. This
/// verifies optimality against the current exact oracle, not mere feasibility.
pub fn verify_generated_challenge_oracle(
    challenge: &GeneratedScientificDecisionChallenge,
) -> Result<(), String> {
    let public = &challenge.public_task;
    let oracle = &challenge.sealed_oracle;
    let receipt = &challenge.generation_receipt;
    if public.task_id != oracle.task_id || public.task_id != receipt.task_id {
        return Err("public task, sealed oracle and receipt task IDs differ".into());
    }
    if public.recipe_id != receipt.recipe_id || public.recipe_version != receipt.recipe_version {
        return Err("public recipe identity differs from sealed receipt".into());
    }
    if !oracle.optimality_verified {
        return Err("sealed oracle is missing its optimality flag".into());
    }
    match public.objective {
        ChallengeObjective::ClassicalWorstStateDeficiency => {
            let recomputed = oracle_from_minimax(public)?;
            if &recomputed != oracle {
                return Err("sealed minimax oracle differs from recomputed exact optimum".into());
            }
        }
        ChallengeObjective::PriorLossSpanWeightedDeficiency => {
            let recomputed = oracle_from_weighted(public)?;
            if &recomputed != oracle {
                return Err("sealed weighted oracle differs from recomputed exact optimum".into());
            }
        }
    }
    Ok(())
}

/// Regenerate all three artifacts from the immutable recipe, split registry
/// and evaluator-held seed. Exact typed equality detects tampering.
pub fn verify_generated_challenge_from_recipe(
    challenge: &GeneratedScientificDecisionChallenge,
    recipe: &BinaryChannelChallengeRecipe,
    registry: &ChallengeSplitRegistry,
) -> Result<(), String> {
    let regenerated =
        generate_binary_channel_challenge(recipe, registry, challenge.generation_receipt.seed)?;
    if regenerated != *challenge {
        return Err(
            "regenerated public/oracle/receipt artifacts differ from stored challenge".into(),
        );
    }
    verify_generated_challenge_oracle(challenge)
}

#[cfg(test)]
mod tests {
    use super::{
        BinaryChannelChallengeRecipe, ChallengeObjective, ChallengeSplitRegistry,
        generate_binary_channel_challenge, verify_generated_challenge_from_recipe,
        verify_generated_challenge_oracle,
    };
    use crate::Rational;
    use crate::decision_value::counterexample_ledger::EvidenceSplit;

    fn q(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn registry() -> ChallengeSplitRegistry {
        ChallengeSplitRegistry {
            development_group_ids: vec!["dev-mechanism-family".into()],
            calibration_group_ids: vec!["cal-mechanism-family".into()],
            sealed_holdout_group_ids: vec!["sealed-mechanism-family".into()],
        }
    }

    fn recipe(objective: ChallengeObjective) -> BinaryChannelChallengeRecipe {
        BinaryChannelChallengeRecipe {
            task_id: "challenge-001".into(),
            recipe_id: "binary-channel-recipe".into(),
            recipe_version: "v1".into(),
            generator_id: "splitmix64-exact-grid".into(),
            generator_version: "1".into(),
            family_group_id: "dev-mechanism-family".into(),
            split: EvidenceSplit::Development,
            objective,
            state_count: 3,
            probability_grid: vec![q(0, 1), q(1, 4), q(1, 2), q(3, 4), q(1, 1)],
            prior_weight_grid: vec![q(0, 1), q(1, 2), q(1, 1), q(2, 1)],
            loss_value_grid: vec![q(1, 2), q(1, 1), q(2, 1), q(5, 1)],
            abstention_loss_grid: vec![q(1, 2), q(1, 1), q(3, 1)],
        }
    }

    #[test]
    fn generation_is_deterministic_and_recipe_replayable() {
        let registry = registry();
        let recipe = recipe(ChallengeObjective::PriorLossSpanWeightedDeficiency);
        let first = generate_binary_channel_challenge(&recipe, &registry, 0x5eed).unwrap();
        let second = generate_binary_channel_challenge(&recipe, &registry, 0x5eed).unwrap();
        assert_eq!(first, second);
        verify_generated_challenge_oracle(&first).unwrap();
        verify_generated_challenge_from_recipe(&first, &recipe, &registry).unwrap();
    }

    #[test]
    fn different_seed_instances_resample_channels_without_public_seed_field() {
        let registry = registry();
        let recipe_a = recipe(ChallengeObjective::ClassicalWorstStateDeficiency);
        let mut recipe_b = recipe_a.clone();
        recipe_b.task_id = "challenge-002".into();
        let first = generate_binary_channel_challenge(&recipe_a, &registry, 17).unwrap();
        let second = generate_binary_channel_challenge(&recipe_b, &registry, 18).unwrap();
        assert_ne!(
            first.public_task.source_channel,
            second.public_task.source_channel
        );
        assert_eq!(first.generation_receipt.seed, 17);
        assert!(!format!("{:?}", first.public_task).contains("seed"));
        verify_generated_challenge_oracle(&first).unwrap();
        verify_generated_challenge_oracle(&second).unwrap();
    }

    #[test]
    fn public_task_and_sealed_oracle_are_separate_typed_artifacts() {
        let challenge = generate_binary_channel_challenge(
            &recipe(ChallengeObjective::PriorLossSpanWeightedDeficiency),
            &registry(),
            91,
        )
        .unwrap();
        assert!(challenge.sealed_oracle.optimality_verified);
        assert!(challenge.public_task.source_channel.validate().is_ok());
        assert!(challenge.public_task.target_channel.validate().is_ok());
    }

    #[test]
    fn split_registry_rejects_unregistered_and_overlapping_groups() {
        let mut invalid = recipe(ChallengeObjective::ClassicalWorstStateDeficiency);
        invalid.family_group_id = "sealed-mechanism-family".into();
        assert!(generate_binary_channel_challenge(&invalid, &registry(), 1).is_err());

        let invalid_registry = ChallengeSplitRegistry {
            development_group_ids: vec!["same".into()],
            calibration_group_ids: vec!["same".into()],
            sealed_holdout_group_ids: vec![],
        };
        assert!(
            generate_binary_channel_challenge(
                &recipe(ChallengeObjective::ClassicalWorstStateDeficiency),
                &invalid_registry,
                1,
            )
            .is_err()
        );
    }

    #[test]
    fn calibration_and_sealed_holdout_recipes_are_split_bound() {
        let mut calibration_recipe = recipe(ChallengeObjective::ClassicalWorstStateDeficiency);
        calibration_recipe.split = EvidenceSplit::Calibration;
        calibration_recipe.family_group_id = "cal-mechanism-family".into();
        let calibration =
            generate_binary_channel_challenge(&calibration_recipe, &registry(), 301).unwrap();
        verify_generated_challenge_oracle(&calibration).unwrap();

        let mut sealed_recipe = recipe(ChallengeObjective::PriorLossSpanWeightedDeficiency);
        sealed_recipe.split = EvidenceSplit::SealedHoldout;
        sealed_recipe.family_group_id = "sealed-mechanism-family".into();
        let sealed = generate_binary_channel_challenge(&sealed_recipe, &registry(), 302).unwrap();
        verify_generated_challenge_oracle(&sealed).unwrap();
    }

    #[test]
    fn malformed_grids_and_zero_prior_weight_grid_fail_closed() {
        let mut invalid = recipe(ChallengeObjective::ClassicalWorstStateDeficiency);
        invalid.probability_grid = vec![q(3, 2)];
        assert!(generate_binary_channel_challenge(&invalid, &registry(), 1).is_err());

        invalid = recipe(ChallengeObjective::ClassicalWorstStateDeficiency);
        invalid.prior_weight_grid = vec![Rational::zero()];
        assert!(generate_binary_channel_challenge(&invalid, &registry(), 1).is_err());
    }

    #[test]
    fn oracle_replay_detects_public_task_or_sealed_value_tampering() {
        let mut challenge = generate_binary_channel_challenge(
            &recipe(ChallengeObjective::PriorLossSpanWeightedDeficiency),
            &registry(),
            144,
        )
        .unwrap();
        challenge.public_task.prior.swap(0, 1);
        assert!(verify_generated_challenge_oracle(&challenge).is_err());

        let mut challenge = generate_binary_channel_challenge(
            &recipe(ChallengeObjective::ClassicalWorstStateDeficiency),
            &registry(),
            145,
        )
        .unwrap();
        challenge.sealed_oracle.objective_value =
            if challenge.sealed_oracle.objective_value.is_zero() {
                Rational::one()
            } else {
                Rational::zero()
            };
        assert!(verify_generated_challenge_oracle(&challenge).is_err());
    }
}
