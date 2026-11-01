//! Exact finite one-node thermal challenge generation and independent replay.
//!
//! Scope is intentionally narrow: a stable, lumped one-node thermal balance
//! with one heat pulse, bounded actuator delivery and an additive sensor bias.
//! This is a finite reference fixture, not a validated model of a real furnace.

use super::counterexample_ledger::EvidenceSplit;
use super::scientific_challenge::ChallengeSplitRegistry;
use crate::Rational;
use std::collections::BTreeSet;

const MAX_GRID_ITEMS: usize = 64;
const MAX_HORIZON: usize = 32;
const MAX_ID_BYTES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteThermalChallengeRecipe {
    pub task_id: String,
    pub recipe_id: String,
    pub recipe_version: String,
    pub family_group_id: String,
    pub split: EvidenceSplit,
    pub initial_temperature_grid: Vec<Rational>,
    pub ambient_temperature_grid: Vec<Rational>,
    pub heat_capacity_grid: Vec<Rational>,
    pub conductance_grid: Vec<Rational>,
    pub time_step_grid: Vec<Rational>,
    pub commanded_power_grid: Vec<Rational>,
    pub actuator_delivery_fraction_grid: Vec<Rational>,
    pub sensor_bias_grid: Vec<Rational>,
    pub safe_temperature_grid: Vec<Rational>,
    pub horizon: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicThermalDecisionTask {
    pub task_id: String,
    pub domain: String,
    pub initial_temperature_readback: Rational,
    pub ambient_temperature: Rational,
    pub commanded_power: Rational,
    pub horizon: usize,
    pub maximum_safe_temperature: Rational,
    pub question: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThermalPlantParameters {
    pub heat_capacity: Rational,
    pub thermal_conductance: Rational,
    pub ambient_temperature: Rational,
    pub time_step: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThermalStepRecord {
    pub step_index: usize,
    pub temperature_before: Rational,
    pub commanded_power: Rational,
    pub realized_power: Rational,
    pub temperature_after: Rational,
    pub measured_temperature_after: Rational,
    pub hazard: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThermalEpisodeReceipt {
    pub initial_temperature: Rational,
    pub actuator_delivery_fraction: Rational,
    pub sensor_bias: Rational,
    pub maximum_safe_temperature: Rational,
    pub steps: Vec<ThermalStepRecord>,
    pub remained_within_safe_envelope: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedThermalChallengeOracle {
    pub task_id: String,
    pub plant: ThermalPlantParameters,
    pub true_initial_temperature: Rational,
    pub episode: ThermalEpisodeReceipt,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThermalChallengeGenerationReceipt {
    pub task_id: String,
    pub recipe_id: String,
    pub recipe_version: String,
    pub family_group_id: String,
    pub split: EvidenceSplit,
    pub generator_version: u32,
    pub seed: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedFiniteThermalChallenge {
    pub public_task: PublicThermalDecisionTask,
    pub sealed_oracle: SealedThermalChallengeOracle,
    pub generation_receipt: ThermalChallengeGenerationReceipt,
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
            return Err("cannot sample an empty thermal recipe grid".into());
        }
        let bound = u64::try_from(length)
            .map_err(|_| "thermal recipe grid length exceeds u64".to_string())?;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let sample = self.next_u64();
            if sample >= threshold {
                return usize::try_from(sample % bound)
                    .map_err(|_| "thermal recipe index exceeds usize".into());
            }
        }
    }

    fn sample(&mut self, values: &[Rational]) -> Result<Rational, String> {
        values
            .get(self.index(values.len())?)
            .copied()
            .ok_or_else(|| "sampled thermal recipe value is absent".into())
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn validate_grid(
    values: &[Rational],
    label: &str,
    require_positive: bool,
    require_probability: bool,
) -> Result<(), String> {
    if values.is_empty() || values.len() > MAX_GRID_ITEMS {
        return Err(format!(
            "{label} must contain between 1 and {MAX_GRID_ITEMS} values"
        ));
    }
    let mut unique = BTreeSet::new();
    for value in values {
        if require_positive && value.is_zero() {
            return Err(format!("{label} must contain only positive values"));
        }
        if require_probability && !value.is_probability() {
            return Err(format!("{label} contains a value outside [0,1]"));
        }
        if !unique.insert(*value) {
            return Err(format!("{label} contains duplicate values"));
        }
    }
    Ok(())
}

fn subtract_nonnegative(left: Rational, right: Rational) -> Result<Rational, String> {
    if left < right {
        return Err("thermal balance subtraction would become negative".into());
    }
    let common = gcd(left.denominator(), right.denominator());
    let left_scale = right.denominator() / common;
    let right_scale = left.denominator() / common;
    let left_numerator = left
        .numerator()
        .checked_mul(left_scale)
        .ok_or_else(|| "thermal subtraction numerator overflow".to_string())?;
    let right_numerator = right
        .numerator()
        .checked_mul(right_scale)
        .ok_or_else(|| "thermal subtraction numerator overflow".to_string())?;
    let denominator = left
        .denominator()
        .checked_mul(left_scale)
        .ok_or_else(|| "thermal subtraction denominator overflow".to_string())?;
    Rational::new(left_numerator - right_numerator, denominator)
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

impl FiniteThermalChallengeRecipe {
    pub fn validate(&self, registry: &ChallengeSplitRegistry) -> Result<(), String> {
        for (value, label) in [
            (&self.task_id, "thermal task ID"),
            (&self.recipe_id, "thermal recipe ID"),
            (&self.recipe_version, "thermal recipe version"),
            (&self.family_group_id, "thermal family-group ID"),
        ] {
            if !valid_id(value) {
                return Err(format!("{label} is empty or overlong"));
            }
        }
        registry.validate_assignment(self.split, &self.family_group_id)?;
        if !(1..=MAX_HORIZON).contains(&self.horizon) {
            return Err(format!("thermal horizon must be in 1..={MAX_HORIZON}"));
        }
        validate_grid(
            &self.initial_temperature_grid,
            "initial-temperature grid",
            false,
            false,
        )?;
        validate_grid(
            &self.ambient_temperature_grid,
            "ambient-temperature grid",
            false,
            false,
        )?;
        validate_grid(
            &self.heat_capacity_grid,
            "heat-capacity grid",
            true,
            false,
        )?;
        validate_grid(
            &self.conductance_grid,
            "thermal-conductance grid",
            false,
            false,
        )?;
        validate_grid(
            &self.time_step_grid,
            "time-step grid",
            true,
            false,
        )?;
        validate_grid(
            &self.commanded_power_grid,
            "commanded-power grid",
            false,
            false,
        )?;
        validate_grid(
            &self.actuator_delivery_fraction_grid,
            "actuator-delivery grid",
            false,
            true,
        )?;
        validate_grid(
            &self.sensor_bias_grid,
            "sensor-bias grid",
            false,
            false,
        )?;
        validate_grid(
            &self.safe_temperature_grid,
            "safe-temperature grid",
            false,
            false,
        )?;

        let minimum_initial = *self.initial_temperature_grid.iter().min().unwrap();
        let maximum_initial = *self.initial_temperature_grid.iter().max().unwrap();
        let maximum_ambient = *self.ambient_temperature_grid.iter().max().unwrap();
        if minimum_initial < maximum_ambient {
            return Err(
                "all recipe initial temperatures must be at least every ambient temperature"
                    .into(),
            );
        }
        let minimum_capacity = *self.heat_capacity_grid.iter().min().unwrap();
        let maximum_conductance = *self.conductance_grid.iter().max().unwrap();
        let maximum_time_step = *self.time_step_grid.iter().max().unwrap();
        let cooling_product = maximum_conductance.checked_mul(maximum_time_step)?;
        if cooling_product > minimum_capacity {
            return Err(
                "thermal recipe violates the explicit-step stability contract dt*k/C <= 1"
                    .into(),
            );
        }
        if self
            .safe_temperature_grid
            .iter()
            .any(|limit| *limit < maximum_initial)
        {
            return Err(
                "safe-temperature grid cannot be below every initial temperature".into(),
            );
        }
        Ok(())
    }
}

fn simulate_episode(
    plant: &ThermalPlantParameters,
    initial_temperature: Rational,
    commanded_power: Rational,
    delivery_fraction: Rational,
    sensor_bias: Rational,
    maximum_safe_temperature: Rational,
    horizon: usize,
) -> Result<ThermalEpisodeReceipt, String> {
    if plant.heat_capacity.is_zero() || plant.time_step.is_zero() {
        return Err("thermal heat capacity and time step must be positive".into());
    }
    if initial_temperature < plant.ambient_temperature {
        return Err("initial temperature must not be below ambient in this model".into());
    }
    if !delivery_fraction.is_probability() {
        return Err("actuator delivery fraction must lie in [0,1]".into());
    }
    let dt_conductance = plant.time_step.checked_mul(plant.thermal_conductance)?;
    let decay_numerator = subtract_nonnegative(plant.heat_capacity, dt_conductance)?;
    let decay = decay_numerator.checked_div(plant.heat_capacity)?;
    let ambient_weight = dt_conductance.checked_div(plant.heat_capacity)?;
    let heating_weight = plant.time_step.checked_div(plant.heat_capacity)?;
    let realized_power = commanded_power.checked_mul(delivery_fraction)?;
    let delivered_energy = heating_weight.checked_mul(realized_power)?;

    let mut temperature = initial_temperature;
    let mut steps = Vec::with_capacity(horizon);
    let mut remained_within_safe_envelope = true;
    for step_index in 0..horizon {
        let retained_heat = decay.checked_mul(temperature)?;
        let ambient_heat = ambient_weight.checked_mul(plant.ambient_temperature)?;
        let next_temperature = retained_heat
            .checked_add(ambient_heat)?
            .checked_add(delivered_energy)?;
        let measured_temperature = next_temperature.checked_add(sensor_bias)?;
        let hazard = next_temperature > maximum_safe_temperature;
        remained_within_safe_envelope &= !hazard;
        steps.push(ThermalStepRecord {
            step_index,
            temperature_before: temperature,
            commanded_power,
            realized_power,
            temperature_after: next_temperature,
            measured_temperature_after: measured_temperature,
            hazard,
        });
        temperature = next_temperature;
        if hazard {
            break;
        }
    }

    Ok(ThermalEpisodeReceipt {
        initial_temperature,
        actuator_delivery_fraction: delivery_fraction,
        sensor_bias,
        maximum_safe_temperature,
        steps,
        remained_within_safe_envelope,
    })
}

/// Generate one exact finite thermal challenge. The seed is retained only in
/// the evaluator receipt; it is not included in the public task.
pub fn generate_finite_thermal_challenge(
    recipe: &FiniteThermalChallengeRecipe,
    registry: &ChallengeSplitRegistry,
    seed: u64,
) -> Result<GeneratedFiniteThermalChallenge, String> {
    recipe.validate(registry)?;
    let mut rng = SplitMix64::new(seed);
    let initial_temperature = rng.sample(&recipe.initial_temperature_grid)?;
    let ambient_temperature = rng.sample(&recipe.ambient_temperature_grid)?;
    let heat_capacity = rng.sample(&recipe.heat_capacity_grid)?;
    let thermal_conductance = rng.sample(&recipe.conductance_grid)?;
    let time_step = rng.sample(&recipe.time_step_grid)?;
    let commanded_power = rng.sample(&recipe.commanded_power_grid)?;
    let delivery_fraction = rng.sample(&recipe.actuator_delivery_fraction_grid)?;
    let sensor_bias = rng.sample(&recipe.sensor_bias_grid)?;
    let maximum_safe_temperature = rng.sample(&recipe.safe_temperature_grid)?;

    let plant = ThermalPlantParameters {
        heat_capacity,
        thermal_conductance,
        ambient_temperature,
        time_step,
    };
    let episode = simulate_episode(
        &plant,
        initial_temperature,
        commanded_power,
        delivery_fraction,
        sensor_bias,
        maximum_safe_temperature,
        recipe.horizon,
    )?;
    let initial_temperature_readback = initial_temperature.checked_add(sensor_bias)?;
    let public_task = PublicThermalDecisionTask {
        task_id: recipe.task_id.clone(),
        domain: "one-node-thermal-process".into(),
        initial_temperature_readback,
        ambient_temperature,
        commanded_power,
        horizon: recipe.horizon,
        maximum_safe_temperature,
        question: "Is this commanded thermal intervention safe for the full declared horizon? Abstain when the available readback does not justify a safe claim.".into(),
    };
    let sealed_oracle = SealedThermalChallengeOracle {
        task_id: recipe.task_id.clone(),
        plant,
        true_initial_temperature: initial_temperature,
        episode,
    };
    let generation_receipt = ThermalChallengeGenerationReceipt {
        task_id: recipe.task_id.clone(),
        recipe_id: recipe.recipe_id.clone(),
        recipe_version: recipe.recipe_version.clone(),
        family_group_id: recipe.family_group_id.clone(),
        split: recipe.split,
        generator_version: 1,
        seed,
    };
    Ok(GeneratedFiniteThermalChallenge {
        public_task,
        sealed_oracle,
        generation_receipt,
    })
}

/// Independently verify a generated episode using the rearranged discrete
/// energy-balance identity rather than the simulator's update expression:
///
/// C*T_after + dt*k*T_before
///   == C*T_before + dt*k*T_ambient + dt*P_realized
///
/// The identity catches trajectory, actuation, measurement and hazard tampering
/// within this declared discrete model. It does not validate the model against
/// a real thermal apparatus.
pub fn verify_finite_thermal_challenge(
    challenge: &GeneratedFiniteThermalChallenge,
) -> Result<(), String> {
    let public = &challenge.public_task;
    let oracle = &challenge.sealed_oracle;
    let receipt = &challenge.generation_receipt;
    if public.task_id != oracle.task_id || public.task_id != receipt.task_id {
        return Err("thermal task, oracle and receipt IDs differ".into());
    }
    if !valid_id(&public.task_id)
        || !valid_id(&receipt.recipe_id)
        || !valid_id(&receipt.recipe_version)
        || !valid_id(&receipt.family_group_id)
        || receipt.generator_version != 1
    {
        return Err("thermal task contains invalid identity or generator version".into());
    }
    if public.domain != "one-node-thermal-process" {
        return Err("thermal public task domain is not supported".into());
    }
    if public.ambient_temperature != oracle.plant.ambient_temperature
        || public.commanded_power
            != oracle
                .episode
                .steps
                .first()
                .map(|step| step.commanded_power)
                .unwrap_or(Rational::zero())
        || public.horizon == 0
        || public.horizon > MAX_HORIZON
        || public.maximum_safe_temperature != oracle.episode.maximum_safe_temperature
        || public.initial_temperature_readback
            != oracle
                .true_initial_temperature
                .checked_add(oracle.episode.sensor_bias)?
    {
        return Err("public thermal task does not match its sealed physical episode".into());
    }

    let plant = &oracle.plant;
    let episode = &oracle.episode;
    if plant.heat_capacity.is_zero()
        || plant.time_step.is_zero()
        || !episode.actuator_delivery_fraction.is_probability()
        || episode.steps.is_empty()
        || episode.steps.len() > public.horizon
        || episode.initial_temperature != oracle.true_initial_temperature
        || oracle.true_initial_temperature < plant.ambient_temperature
    {
        return Err("sealed thermal oracle violates the declared physical contract".into());
    }
    let dt_k = plant.time_step.checked_mul(plant.thermal_conductance)?;
    if dt_k > plant.heat_capacity {
        return Err("sealed thermal model violates dt*k/C <= 1".into());
    }
    let expected_realized_power = public
        .commanded_power
        .checked_mul(episode.actuator_delivery_fraction)?;
    let mut previous_temperature = oracle.true_initial_temperature;
    let mut saw_hazard = false;
    for (index, step) in episode.steps.iter().enumerate() {
        if saw_hazard {
            return Err("thermal episode continues after terminal hazard".into());
        }
        if step.step_index != index
            || step.temperature_before != previous_temperature
            || step.commanded_power != public.commanded_power
            || step.realized_power != expected_realized_power
        {
            return Err("thermal step identity or actuation receipt is inconsistent".into());
        }

        let left = plant
            .heat_capacity
            .checked_mul(step.temperature_after)?
            .checked_add(dt_k.checked_mul(step.temperature_before)?)?;
        let right = plant
            .heat_capacity
            .checked_mul(step.temperature_before)?
            .checked_add(dt_k.checked_mul(plant.ambient_temperature)?)?
            .checked_add(plant.time_step.checked_mul(step.realized_power)?)?;
        if left != right {
            return Err("thermal step violates the exact discrete energy-balance identity".into());
        }
        if step.measured_temperature_after
            != step.temperature_after.checked_add(episode.sensor_bias)?
        {
            return Err("thermal sensor readback does not match the sealed sensor-bias model".into());
        }
        let hazard = step.temperature_after > episode.maximum_safe_temperature;
        if step.hazard != hazard {
            return Err("thermal hazard flag does not match the declared safety envelope".into());
        }
        saw_hazard = hazard;
        previous_temperature = step.temperature_after;
    }
    if !saw_hazard && episode.steps.len() != public.horizon {
        return Err("non-hazard thermal episode does not cover its full horizon".into());
    }
    if episode.remained_within_safe_envelope == saw_hazard {
        return Err("thermal episode safe-envelope summary is inconsistent".into());
    }
    Ok(())
}

/// Regenerate a challenge from its immutable recipe and pre-registered
/// split registry, then require exact equality of public, sealed and receipt
/// artifacts. This is the stronger provenance check; the standalone verifier
/// above checks only the supplied episode against its declared discrete model.
pub fn verify_finite_thermal_challenge_from_recipe(
    challenge: &GeneratedFiniteThermalChallenge,
    recipe: &FiniteThermalChallengeRecipe,
    registry: &ChallengeSplitRegistry,
) -> Result<(), String> {
    verify_finite_thermal_challenge(challenge)?;
    let regenerated = generate_finite_thermal_challenge(
        recipe,
        registry,
        challenge.generation_receipt.seed,
    )?;
    if regenerated != *challenge {
        return Err(
            "regenerated public task, sealed thermal oracle or receipt differs from stored challenge"
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        FiniteThermalChallengeRecipe, generate_finite_thermal_challenge,
        verify_finite_thermal_challenge, verify_finite_thermal_challenge_from_recipe,
    };
    use super::super::scientific_challenge::ChallengeSplitRegistry;
    use crate::decision_value::counterexample_ledger::EvidenceSplit;
    use crate::Rational;

    fn q(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn registry() -> ChallengeSplitRegistry {
        ChallengeSplitRegistry {
            development_group_ids: vec!["thermal-dev".into()],
            calibration_group_ids: vec!["thermal-cal".into()],
            sealed_holdout_group_ids: vec!["thermal-holdout".into()],
        }
    }

    fn recipe() -> FiniteThermalChallengeRecipe {
        FiniteThermalChallengeRecipe {
            task_id: "thermal-task-001".into(),
            recipe_id: "one-node-thermal".into(),
            recipe_version: "v1".into(),
            family_group_id: "thermal-dev".into(),
            split: EvidenceSplit::Development,
            initial_temperature_grid: vec![q(80, 1), q(90, 1)],
            ambient_temperature_grid: vec![q(20, 1)],
            heat_capacity_grid: vec![q(10, 1), q(20, 1)],
            conductance_grid: vec![q(1, 1), q(2, 1)],
            time_step_grid: vec![q(1, 1)],
            commanded_power_grid: vec![q(5, 1), q(10, 1)],
            actuator_delivery_fraction_grid: vec![q(1, 2), q(1, 1)],
            sensor_bias_grid: vec![q(0, 1), q(2, 1)],
            safe_temperature_grid: vec![q(110, 1), q(120, 1)],
            horizon: 3,
        }
    }

    #[test]
    fn generation_is_deterministic_and_exactly_replayable() {
        let recipe = recipe();
        let registry = registry();
        let first = generate_finite_thermal_challenge(&recipe, &registry, 0x5eed).unwrap();
        let second = generate_finite_thermal_challenge(&recipe, &registry, 0x5eed).unwrap();
        assert_eq!(first, second);
        verify_finite_thermal_challenge(&first).unwrap();
        verify_finite_thermal_challenge_from_recipe(&first, &recipe, &registry).unwrap();
        assert!(!format!("{:?}", first.public_task).contains("seed"));
    }

    #[test]
    fn verifier_rejects_temperature_actuation_sensor_and_hazard_tampering() {
        let challenge = generate_finite_thermal_challenge(&recipe(), &registry(), 17).unwrap();

        let mut tampered = challenge.clone();
        tampered.sealed_oracle.episode.steps[0].temperature_after =
            tampered.sealed_oracle.episode.steps[0].temperature_after.checked_add(q(1, 1)).unwrap();
        assert!(verify_finite_thermal_challenge(&tampered).is_err());

        let mut tampered = challenge.clone();
        tampered.sealed_oracle.episode.steps[0].realized_power =
            tampered.sealed_oracle.episode.steps[0].realized_power.checked_add(q(1, 1)).unwrap();
        assert!(verify_finite_thermal_challenge(&tampered).is_err());

        let mut tampered = challenge.clone();
        tampered.sealed_oracle.episode.steps[0].measured_temperature_after =
            tampered.sealed_oracle.episode.steps[0].measured_temperature_after.checked_add(q(1, 1)).unwrap();
        assert!(verify_finite_thermal_challenge(&tampered).is_err());

        let mut tampered = challenge;
        tampered.sealed_oracle.episode.steps[0].hazard = !tampered.sealed_oracle.episode.steps[0].hazard;
        assert!(verify_finite_thermal_challenge(&tampered).is_err());
    }

    #[test]
    fn rejects_unstable_or_split_unregistered_recipes() {
        let mut invalid = recipe();
        invalid.time_step_grid = vec![q(100, 1)];
        assert!(generate_finite_thermal_challenge(&invalid, &registry(), 1).is_err());

        invalid = recipe();
        invalid.family_group_id = "unknown-group".into();
        assert!(generate_finite_thermal_challenge(&invalid, &registry(), 1).is_err());
    }

    #[test]
    fn calibration_and_sealed_challenges_remain_bound_to_their_registered_groups() {
        let mut calibration = recipe();
        calibration.split = EvidenceSplit::Calibration;
        calibration.family_group_id = "thermal-cal".into();
        let challenge =
            generate_finite_thermal_challenge(&calibration, &registry(), 21).unwrap();
        verify_finite_thermal_challenge(&challenge).unwrap();

        let mut sealed = recipe();
        sealed.split = EvidenceSplit::SealedHoldout;
        sealed.family_group_id = "thermal-holdout".into();
        let challenge = generate_finite_thermal_challenge(&sealed, &registry(), 22).unwrap();
        verify_finite_thermal_challenge(&challenge).unwrap();
    }
}
