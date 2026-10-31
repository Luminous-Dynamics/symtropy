//! Joint-state bootstrap particle filter for approximate stochastic inference.
//!
//! Each particle carries a fixed model-world identity together with its
//! mechanism, physical-state, and sensor-state tuple. Updating observations
//! changes the latent state but never swaps a particle into another world.

use super::{
    JointOutcome, PolicyPlan, Rational,
    robust::{ModelWorld, evaluate_policy_across_worlds},
};
use std::collections::BTreeMap;

const MAX_PARTICLES: usize = 100_000;

/// Configuration for the approximate joint bootstrap particle filter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleFilterConfig {
    pub particle_count: usize,
    pub seed: u64,
    /// Resample when pre-resampling ESS is below this fraction of N.
    pub resample_ess_ratio: f64,
}

impl Default for ParticleFilterConfig {
    fn default() -> Self {
        Self {
            particle_count: 4_096,
            seed: 0x9e37_79b9_7f4a_7c15,
            resample_ess_ratio: 0.5,
        }
    }
}

/// Diagnostics reported after filtering updates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParticleFilterDiagnostics {
    pub update_count: usize,
    pub resampling_count: usize,
    pub degeneracy_count: usize,
    pub effective_sample_size: f64,
    pub last_pre_resample_ess: f64,
    pub minimum_pre_resample_ess: f64,
}

/// Approximate one-step mechanism EIG from the current weighted particles.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleEigScore {
    pub action_index: usize,
    pub action_id: String,
    pub expected_information_gain_nats: f64,
    pub information_gain_per_cost: f64,
    pub hazard_probability: f64,
}

/// One latent joint-state particle. World identity is immutable across updates.
#[derive(Clone, Debug)]
struct Particle {
    world_index: usize,
    hypothesis: usize,
    physical_state: usize,
    sensor_state: usize,
    weight: f64,
}

/// A bootstrap filter over (world, mechanism, physical state, sensor state).
pub struct JointParticleFilter<'a> {
    worlds: &'a [ModelWorld],
    particles: Vec<Particle>,
    rng: XorShift64,
    config: ParticleFilterConfig,
    diagnostics: ParticleFilterDiagnostics,
}

impl<'a> JointParticleFilter<'a> {
    /// Initialize from a positive prior over model worlds and each world's exact joint prior.
    pub fn new(
        worlds: &'a [ModelWorld],
        world_priors: &[Rational],
        config: ParticleFilterConfig,
    ) -> Result<Self, String> {
        if worlds.is_empty() || world_priors.len() != worlds.len() {
            return Err("particle filter requires one positive prior per model world".into());
        }
        if config.particle_count == 0 || config.particle_count > MAX_PARTICLES {
            return Err(format!("particle count must be in 1..={MAX_PARTICLES}"));
        }
        if !config.resample_ess_ratio.is_finite()
            || config.resample_ess_ratio <= 0.0
            || config.resample_ess_ratio > 1.0
        {
            return Err("resample_ess_ratio must lie in (0, 1]".into());
        }
        let nominal = worlds
            .first()
            .map(|world| world.id.as_str())
            .ok_or_else(|| "particle filter requires at least one world".to_string())?;
        evaluate_policy_across_worlds(worlds, nominal, &PolicyPlan::Abstain, 0)?;

        let prior_sum = world_priors.iter().try_fold(
            Rational::zero(),
            |sum, prior| sum.checked_add(*prior),
        )?;
        if prior_sum != Rational::one() || world_priors.iter().any(|prior| prior.is_zero()) {
            return Err("model-world priors must be strictly positive and sum exactly to one".into());
        }

        let mut components = Vec::new();
        let mut component_mass = 0.0_f64;
        for (world_index, world) in worlds.iter().enumerate() {
            for point in &world.kernel.initial_belief {
                let mass = world_priors[world_index].checked_mul(point.mass)?;
                let probability = to_f64(mass);
                if probability > 0.0 {
                    component_mass += probability;
                    components.push((
                        world_index,
                        point.hypothesis,
                        point.physical_state,
                        point.sensor_state,
                        probability,
                    ));
                }
            }
        }
        if !component_mass.is_finite() || (component_mass - 1.0).abs() > 1e-12 {
            return Err("joint model-world initialization failed probability normalization".into());
        }

        let mut rng = XorShift64::new(config.seed);
        let particle_weight = 1.0 / config.particle_count as f64;
        let mut particles = Vec::with_capacity(config.particle_count);
        for _ in 0..config.particle_count {
            let draw = rng.uniform();
            let mut cumulative = 0.0_f64;
            let mut selected = components
                .last()
                .ok_or_else(|| "joint particle initialization has no support".to_string())?;
            for component in &components {
                cumulative += component.4 / component_mass;
                if draw < cumulative {
                    selected = component;
                    break;
                }
            }
            particles.push(Particle {
                world_index: selected.0,
                hypothesis: selected.1,
                physical_state: selected.2,
                sensor_state: selected.3,
                weight: particle_weight,
            });
        }

        let ess = config.particle_count as f64;
        Ok(Self {
            worlds,
            particles,
            rng,
            config,
            diagnostics: ParticleFilterDiagnostics {
                effective_sample_size: ess,
                last_pre_resample_ess: ess,
                minimum_pre_resample_ess: ess,
                ..ParticleFilterDiagnostics::default()
            },
        })
    }

    /// Initialize with a uniform prior over the supplied model worlds.
    pub fn with_uniform_world_prior(
        worlds: &'a [ModelWorld],
        config: ParticleFilterConfig,
    ) -> Result<Self, String> {
        if worlds.is_empty() {
            return Err("particle filter requires at least one model world".into());
        }
        let prior = Rational::new(1, worlds.len() as u128)?;
        let priors = vec![prior; worlds.len()];
        Self::new(worlds, &priors, config)
    }

    pub fn diagnostics(&self) -> ParticleFilterDiagnostics {
        self.diagnostics
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Approximate posterior over semantic mechanism hypotheses.
    pub fn hypothesis_posterior(&self) -> Vec<f64> {
        let hypothesis_count = self.worlds[0].kernel.hypothesis_ids.len();
        let mut posterior = vec![0.0_f64; hypothesis_count];
        for particle in &self.particles {
            posterior[particle.hypothesis] += particle.weight;
        }
        normalize_f64(&mut posterior);
        posterior
    }

    /// Approximate posterior over persistent model-world identities.
    pub fn model_world_posterior(&self) -> Vec<f64> {
        let mut posterior = vec![0.0_f64; self.worlds.len()];
        for particle in &self.particles {
            posterior[particle.world_index] += particle.weight;
        }
        normalize_f64(&mut posterior);
        posterior
    }

    /// Condition on a public observation. None denotes an observed terminal hazard.
    ///
    /// The update is transactional: if no particle supports the observation,
    /// the prior particle cloud is retained and a degeneracy is recorded.
    pub fn update(&mut self, action: usize, observation: Option<usize>) -> Result<(), String> {
        if action >= self.worlds[0].kernel.action_ids.len() {
            return Err("particle update uses an invalid action".into());
        }
        if observation.is_some_and(|value| value >= self.worlds[0].kernel.observation_ids.len()) {
            return Err("particle update uses an invalid observation ID".into());
        }

        let mut updated = Vec::with_capacity(self.particles.len());
        let mut total_weight = 0.0_f64;
        for particle in &self.particles {
            let kernel = &self.worlds[particle.world_index].kernel;
            let row_index = kernel.row_index(
                particle.hypothesis,
                particle.physical_state,
                particle.sensor_state,
                action,
            );
            let row = &kernel.rows[row_index];
            let mut likelihood = 0.0_f64;
            for outcome in &row.outcomes {
                if outcome.observation == observation {
                    likelihood += to_f64(outcome.probability);
                }
            }
            if likelihood <= 0.0 {
                let mut zero_weight = particle.clone();
                zero_weight.weight = 0.0;
                updated.push(zero_weight);
                continue;
            }

            let target = self.rng.uniform() * likelihood;
            let mut cumulative = 0.0_f64;
            let mut selected: Option<&JointOutcome> = None;
            for outcome in &row.outcomes {
                if outcome.observation == observation {
                    cumulative += to_f64(outcome.probability);
                    if target < cumulative {
                        selected = Some(outcome);
                        break;
                    }
                }
            }
            let selected = selected
                .or_else(|| {
                    row.outcomes
                        .iter()
                        .rev()
                        .find(|outcome| outcome.observation == observation)
                })
                .ok_or_else(|| "particle conditional outcome selection failed".to_string())?;

            let weight = particle.weight * likelihood;
            total_weight += weight;
            updated.push(Particle {
                world_index: particle.world_index,
                hypothesis: particle.hypothesis,
                physical_state: selected.next_physical_state,
                sensor_state: selected.next_sensor_state,
                weight,
            });
        }

        if !total_weight.is_finite() || total_weight <= f64::MIN_POSITIVE {
            self.diagnostics.degeneracy_count =
                self.diagnostics.degeneracy_count.saturating_add(1);
            return Err("particle cloud degenerated: no particles support the observation".into());
        }
        for particle in &mut updated {
            particle.weight /= total_weight;
        }

        let ess_before = effective_sample_size(&updated);
        self.diagnostics.update_count = self.diagnostics.update_count.saturating_add(1);
        self.diagnostics.last_pre_resample_ess = ess_before;
        self.diagnostics.minimum_pre_resample_ess = if self.diagnostics.update_count == 1 {
            ess_before
        } else {
            self.diagnostics.minimum_pre_resample_ess.min(ess_before)
        };

        if ess_before < self.config.resample_ess_ratio * self.particles.len() as f64 {
            updated = systematic_resample(&updated, &mut self.rng);
            self.diagnostics.resampling_count =
                self.diagnostics.resampling_count.saturating_add(1);
        }
        self.diagnostics.effective_sample_size = effective_sample_size(&updated);
        self.particles = updated;
        Ok(())
    }

    /// Rank actions by particle-estimated one-step EIG about the mechanism.
    ///
    /// The current joint particle posterior is propagated through each world-
    /// specific joint kernel. Hazard is treated as an observed terminal event.
    pub fn rank_actions_by_eig(&self) -> Result<Vec<ParticleEigScore>, String> {
        let kernel_reference = &self.worlds[0].kernel;
        let hypothesis_count = kernel_reference.hypothesis_ids.len();
        let mut prior = vec![0.0_f64; hypothesis_count];
        for particle in &self.particles {
            prior[particle.hypothesis] += particle.weight;
        }
        normalize_f64(&mut prior);
        let prior_entropy = entropy(&prior);

        let mut scores = Vec::with_capacity(kernel_reference.action_ids.len());
        for action in 0..kernel_reference.action_ids.len() {
            let mut by_event: BTreeMap<Option<usize>, Vec<f64>> = BTreeMap::new();
            for particle in &self.particles {
                let kernel = &self.worlds[particle.world_index].kernel;
                let row_index = kernel.row_index(
                    particle.hypothesis,
                    particle.physical_state,
                    particle.sensor_state,
                    action,
                );
                for outcome in &kernel.rows[row_index].outcomes {
                    let mass = particle.weight * to_f64(outcome.probability);
                    let event = by_event
                        .entry(outcome.observation)
                        .or_insert_with(|| vec![0.0; hypothesis_count]);
                    event[particle.hypothesis] += mass;
                }
            }

            let mut expected_entropy = 0.0_f64;
            let mut hazard_probability = 0.0_f64;
            for (event, masses) in by_event {
                let probability: f64 = masses.iter().sum();
                if event.is_none() {
                    hazard_probability += probability;
                }
                if probability > 0.0 {
                    let posterior: Vec<f64> =
                        masses.iter().map(|mass| mass / probability).collect();
                    expected_entropy += probability * entropy(&posterior);
                }
            }
            let information_gain = (prior_entropy - expected_entropy).max(0.0);
            let cost = kernel_reference.action_costs[action] as f64;
            scores.push(ParticleEigScore {
                action_index: action,
                action_id: kernel_reference.action_ids[action].clone(),
                expected_information_gain_nats: information_gain,
                information_gain_per_cost: information_gain / cost,
                hazard_probability,
            });
        }

        scores.sort_by(|left, right| {
            right
                .information_gain_per_cost
                .total_cmp(&left.information_gain_per_cost)
                .then_with(|| {
                    right.expected_information_gain_nats
                        .total_cmp(&left.expected_information_gain_nats)
                })
                .then_with(|| left.action_index.cmp(&right.action_index))
        });
        Ok(scores)
    }
}

fn normalize_f64(values: &mut [f64]) {
    let total: f64 = values.iter().sum();
    if total > 0.0 && total.is_finite() {
        for value in values {
            *value /= total;
        }
    }
}

fn entropy(probabilities: &[f64]) -> f64 {
    probabilities
        .iter()
        .filter(|probability| **probability > 0.0)
        .map(|probability| -*probability * probability.ln())
        .sum()
}

fn to_f64(value: Rational) -> f64 {
    value.numerator() as f64 / value.denominator() as f64
}

fn effective_sample_size(particles: &[Particle]) -> f64 {
    let sum_squares: f64 = particles
        .iter()
        .map(|particle| particle.weight * particle.weight)
        .sum();
    if sum_squares <= 0.0 || !sum_squares.is_finite() {
        0.0
    } else {
        1.0 / sum_squares
    }
}

fn systematic_resample(particles: &[Particle], rng: &mut XorShift64) -> Vec<Particle> {
    let count = particles.len();
    let step = 1.0 / count as f64;
    let offset = rng.uniform() * step;
    let mut cumulative = particles[0].weight;
    let mut source_index = 0;
    let mut result = Vec::with_capacity(count);
    for index in 0..count {
        let target = offset + index as f64 * step;
        while target > cumulative && source_index + 1 < count {
            source_index += 1;
            cumulative += particles[source_index].weight;
        }
        let mut selected = particles[source_index].clone();
        selected.weight = step;
        result.push(selected);
    }
    result
}

#[derive(Clone, Copy, Debug)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xa076_1d64_78bd_642f } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn uniform(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        mantissa as f64 / ((1_u64 << 53) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JointKernel, JointMass, KernelRow};

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn reliable_world(id: &str, reliability: Rational, deterministic: bool) -> ModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let p_zero = if hypothesis == 0 {
                reliability
            } else {
                Rational::one().checked_sub(reliability).unwrap()
            };
            let outcomes = if deterministic {
                vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(0),
                }]
            } else {
                vec![
                    JointOutcome {
                        probability: p_zero,
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(0),
                    },
                    JointOutcome {
                        probability: Rational::one().checked_sub(p_zero).unwrap(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(1),
                    },
                ]
            };
            rows.push(KernelRow {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes,
            });
        }
        ModelWorld {
            id: id.into(),
            kernel: JointKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                action_ids: vec!["read".into()],
                observation_ids: vec!["zero".into(), "one".into()],
                action_costs: vec![1],
                initial_belief: vec![
                    JointMass {
                        hypothesis: 0,
                        physical_state: 0,
                        sensor_state: 0,
                        mass: half(),
                    },
                    JointMass {
                        hypothesis: 1,
                        physical_state: 0,
                        sensor_state: 0,
                        mass: half(),
                    },
                ],
                rows,
            },
        }
    }

    #[test]
    fn posterior_tracks_hand_computable_binary_likelihood() {
        let world = reliable_world("nominal", Rational::new(3, 4).unwrap(), false);
        let mut filter = JointParticleFilter::with_uniform_world_prior(
            &[world],
            ParticleFilterConfig {
                particle_count: 30_000,
                seed: 1234,
                resample_ess_ratio: 0.01,
            },
        )
        .unwrap();
        filter.update(0, Some(0)).unwrap();
        let posterior = filter.hypothesis_posterior();
        assert!((posterior[0] - 0.75).abs() < 0.025);
        assert!((posterior[1] - 0.25).abs() < 0.025);
        assert!((posterior.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn model_identity_is_persistent_across_repeated_observations() {
        let worlds = vec![
            reliable_world("strong-sensor", Rational::new(9, 10).unwrap(), false),
            reliable_world("weak-sensor", Rational::new(3, 5).unwrap(), false),
        ];
        let mut filter = JointParticleFilter::with_uniform_world_prior(
            &worlds,
            ParticleFilterConfig {
                particle_count: 50_000,
                seed: 98765,
                resample_ess_ratio: 0.01,
            },
        )
        .unwrap();
        filter.update(0, Some(0)).unwrap();
        filter.update(0, Some(0)).unwrap();
        let posterior = filter.model_world_posterior();
        let exact_world_posterior = 0.41 / (0.41 + 0.26);
        assert!((posterior[0] - exact_world_posterior).abs() < 0.03);
        assert!((posterior[1] - (1.0 - exact_world_posterior)).abs() < 0.03);
    }

    #[test]
    fn particle_eig_matches_analytic_binary_channel_with_sampling_error() {
        let world = reliable_world("nominal", Rational::new(3, 4).unwrap(), false);
        let filter = JointParticleFilter::with_uniform_world_prior(
            &[world],
            ParticleFilterConfig {
                particle_count: 30_000,
                seed: 2026,
                resample_ess_ratio: 0.5,
            },
        )
        .unwrap();
        let score = &filter.rank_actions_by_eig().unwrap()[0];
        let exact = std::f64::consts::LN_2
            + 0.75 * 0.75_f64.ln()
            + 0.25 * 0.25_f64.ln();
        assert!((score.expected_information_gain_nats - exact).abs() < 0.02);
    }

    #[test]
    fn resampling_and_effective_sample_size_are_reported() {
        let world = reliable_world("nominal", Rational::new(3, 4).unwrap(), false);
        let mut filter = JointParticleFilter::with_uniform_world_prior(
            &[world],
            ParticleFilterConfig {
                particle_count: 4_000,
                seed: 42,
                resample_ess_ratio: 1.0,
            },
        )
        .unwrap();
        filter.update(0, Some(0)).unwrap();
        let diagnostics = filter.diagnostics();
        assert_eq!(diagnostics.update_count, 1);
        assert_eq!(diagnostics.resampling_count, 1);
        assert!(diagnostics.last_pre_resample_ess < 4_000.0);
        assert!((diagnostics.effective_sample_size - 4_000.0).abs() < 1e-6);
    }

    #[test]
    fn impossible_observation_is_transactional_and_records_degeneracy() {
        let world = reliable_world("deterministic", Rational::one(), true);
        let mut filter = JointParticleFilter::with_uniform_world_prior(
            &[world],
            ParticleFilterConfig {
                particle_count: 1_000,
                seed: 9,
                resample_ess_ratio: 0.5,
            },
        )
        .unwrap();
        let before = filter.hypothesis_posterior();
        assert!(filter.update(0, Some(1)).is_err());
        assert_eq!(filter.hypothesis_posterior(), before);
        assert_eq!(filter.diagnostics().degeneracy_count, 1);
        assert_eq!(filter.diagnostics().update_count, 0);
    }

    #[test]
    fn seed_reproducibly_controls_particle_initialization_and_updates() {
        let world = reliable_world("nominal", Rational::new(3, 4).unwrap(), false);
        let config = ParticleFilterConfig {
            particle_count: 10_000,
            seed: 88,
            resample_ess_ratio: 0.4,
        };
        let mut first = JointParticleFilter::with_uniform_world_prior(
            std::slice::from_ref(&world),
            config,
        )
        .unwrap();
        let mut second = JointParticleFilter::with_uniform_world_prior(
            std::slice::from_ref(&world),
            config,
        )
        .unwrap();
        first.update(0, Some(0)).unwrap();
        second.update(0, Some(0)).unwrap();
        assert_eq!(first.hypothesis_posterior(), second.hypothesis_posterior());
        assert_eq!(first.diagnostics(), second.diagnostics());
    }

    #[test]
    fn invalid_priors_caps_and_resampling_ratios_fail_closed() {
        let world = reliable_world("nominal", Rational::new(3, 4).unwrap(), false);
        assert!(JointParticleFilter::new(
            std::slice::from_ref(&world),
            &[Rational::new(1, 2).unwrap()],
            ParticleFilterConfig::default(),
        )
        .is_err());
        assert!(JointParticleFilter::with_uniform_world_prior(
            std::slice::from_ref(&world),
            ParticleFilterConfig {
                particle_count: 0,
                ..ParticleFilterConfig::default()
            },
        )
        .is_err());
        assert!(JointParticleFilter::with_uniform_world_prior(
            std::slice::from_ref(&world),
            ParticleFilterConfig {
                resample_ess_ratio: 0.0,
                ..ParticleFilterConfig::default()
            },
        )
        .is_err());
    }
}
