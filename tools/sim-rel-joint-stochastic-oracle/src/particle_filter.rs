//! Fully adapted joint-state particle filter for finite stochastic kernels.
//!
//! Particles retain mechanism, physical state and sensor state as one tuple.
//! At each observed event, the filter weights particles by the exact kernel
//! likelihood and samples next physical/sensor state conditional on that event.
//! The model kernel is borrowed immutably for the filter lifetime, preventing
//! accidental model changes mid-campaign. This is not nested SMC-squared.

use super::{JointKernel, JointMass, Rational};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

const MAX_PARTICLES: usize = 1_000_000;
const MAX_BELIEF_TUPLES: usize = 65_536;

/// Particle in the joint mechanism / physical / sensor state space.
#[derive(Clone, Debug, PartialEq)]
pub struct JointParticle {
    pub hypothesis: usize,
    pub physical_state: usize,
    pub sensor_state: usize,
    pub weight: f64,
}

/// Configuration for deterministic particle initialization and resampling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleFilterConfig {
    pub particle_count: usize,
    /// Resample when ESS is below this fraction of the target particle count.
    pub resample_ess_fraction: f64,
    pub seed: u64,
}

impl Default for ParticleFilterConfig {
    fn default() -> Self {
        Self {
            particle_count: 1024,
            resample_ess_fraction: 0.5,
            seed: 0x5046_5349_4D52_454C,
        }
    }
}

/// Diagnostics for one joint-state observation update.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleFilterDiagnostics {
    pub predictive_probability_estimate: f64,
    pub log_predictive_probability_estimate: f64,
    pub effective_sample_size_before_resampling: f64,
    pub effective_sample_size_after_resampling: f64,
    pub particles_before_resampling: usize,
    pub particles_after_update: usize,
    pub particles_without_event_support: usize,
    pub support_loss_fraction: f64,
    pub degenerate: bool,
    pub terminal_hazard: bool,
    pub resampled: bool,
}

/// Aggregate particle posterior over unique joint tuples.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticlePosteriorEntry {
    pub hypothesis: usize,
    pub physical_state: usize,
    pub sensor_state: usize,
    pub probability: f64,
}

/// Accuracy and runtime record for one particle-count point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleFilterBenchmarkRecord {
    pub particle_count: usize,
    pub predictive_probability_exact: Rational,
    pub predictive_probability_estimate: f64,
    pub predictive_probability_absolute_error: f64,
    pub joint_posterior_total_variation_distance: f64,
    pub joint_posterior_squared_error: f64,
    pub mechanism_posterior_squared_error: f64,
    pub effective_sample_size_before_resampling: f64,
    pub effective_sample_size_after_resampling: f64,
    pub support_loss_fraction: f64,
    pub resampled: bool,
    pub elapsed_nanos: u128,
}

/// Run a paired particle-count sweep against the exact one-step posterior.
///
/// Each particle count uses the same seed. Runtime includes initialization and
/// one observation update and is descriptive (not a stable correctness metric).
pub fn benchmark_particle_counts(
    kernel: &JointKernel,
    belief: &[JointMass],
    action: usize,
    observation: Option<usize>,
    particle_counts: &[usize],
    resample_ess_fraction: f64,
    seed: u64,
) -> Result<Vec<ParticleFilterBenchmarkRecord>, String> {
    if particle_counts.is_empty() {
        return Err("particle-count sweep must contain at least one value".into());
    }
    let (exact_predictive, exact_posterior) =
        exact_joint_posterior_after_observation(kernel, belief, action, observation)?;
    let mut records = Vec::with_capacity(particle_counts.len());

    for particle_count in particle_counts {
        let start = Instant::now();
        let mut filter = JointStateParticleFilter::from_joint_belief(
            kernel,
            belief,
            ParticleFilterConfig {
                particle_count: *particle_count,
                resample_ess_fraction,
                seed,
            },
        )?;
        let diagnostics = filter.observe(action, observation)?;
        let elapsed_nanos = start.elapsed().as_nanos();
        let posterior = filter.posterior();
        let total_variation = filter.total_variation_distance(&exact_posterior)?;
        let (joint_squared_error, mechanism_squared_error) =
            posterior_squared_errors(kernel, &posterior, &exact_posterior)?;

        records.push(ParticleFilterBenchmarkRecord {
            particle_count: *particle_count,
            predictive_probability_exact: exact_predictive,
            predictive_probability_estimate: diagnostics.predictive_probability_estimate,
            predictive_probability_absolute_error: (
                diagnostics.predictive_probability_estimate - to_f64(exact_predictive)
            ).abs(),
            joint_posterior_total_variation_distance: total_variation,
            joint_posterior_squared_error: joint_squared_error,
            mechanism_posterior_squared_error: mechanism_squared_error,
            effective_sample_size_before_resampling:
                diagnostics.effective_sample_size_before_resampling,
            effective_sample_size_after_resampling:
                diagnostics.effective_sample_size_after_resampling,
            support_loss_fraction: diagnostics.support_loss_fraction,
            resampled: diagnostics.resampled,
            elapsed_nanos,
        });
    }
    Ok(records)
}

fn posterior_squared_errors(
    kernel: &JointKernel,
    approximate: &[ParticlePosteriorEntry],
    exact: &[JointMass],
) -> Result<(f64, f64), String> {
    let mut joint_approximate = BTreeMap::new();
    for entry in approximate {
        joint_approximate.insert(
            (entry.hypothesis, entry.physical_state, entry.sensor_state),
            entry.probability,
        );
    }
    let mut joint_exact = BTreeMap::new();
    for point in exact {
        joint_exact.insert(
            (point.hypothesis, point.physical_state, point.sensor_state),
            to_f64(point.mass),
        );
    }
    let keys: BTreeSet<_> = joint_approximate
        .keys()
        .chain(joint_exact.keys())
        .copied()
        .collect();
    let joint_squared_error = keys.iter().fold(0.0, |sum, key| {
        let delta = joint_approximate.get(key).copied().unwrap_or(0.0)
            - joint_exact.get(key).copied().unwrap_or(0.0);
        sum + delta * delta
    });

    let mut mechanism_approximate = vec![0.0_f64; kernel.hypothesis_ids.len()];
    let mut mechanism_exact = vec![0.0_f64; kernel.hypothesis_ids.len()];
    for entry in approximate {
        mechanism_approximate[entry.hypothesis] += entry.probability;
    }
    for point in exact {
        mechanism_exact[point.hypothesis] += to_f64(point.mass);
    }
    let mechanism_squared_error = mechanism_approximate
        .iter()
        .zip(mechanism_exact.iter())
        .map(|(approximate, exact)| {
            let delta = approximate - exact;
            delta * delta
        })
        .sum();
    Ok((joint_squared_error, mechanism_squared_error))
}

/// Deterministic joint-state particle filter.
#[derive(Clone, Debug, PartialEq)]
pub struct JointStateParticleFilter<'kernel> {
    kernel: &'kernel JointKernel,
    particles: Vec<JointParticle>,
    target_particle_count: usize,
    resample_ess_fraction: f64,
    rng_state: u64,
    diagnostics: Option<ParticleFilterDiagnostics>,
    terminated_by_hazard: bool,
}

impl<'kernel> JointStateParticleFilter<'kernel> {
    /// Sample the supplied exact joint prior into a weighted particle set.
    ///
    /// The kernel is borrowed immutably and remains the model for the entire
    /// filter lifetime; each alternative world requires a separate filter.
    pub fn from_joint_belief(
        kernel: &'kernel JointKernel,
        initial_belief: &[JointMass],
        config: ParticleFilterConfig,
    ) -> Result<Self, String> {
        kernel.validate()?;
        validate_joint_belief(kernel, initial_belief)?;
        validate_config(config)?;

        // Systematic initialization is lower variance than independent draws
        // and exactly preserves equal-mass strata when the requested count allows it.
        let mut rng = DeterministicRng::new(config.seed);
        let mut prior_particles: Vec<JointParticle> = initial_belief
            .iter()
            .map(|point| JointParticle {
                hypothesis: point.hypothesis,
                physical_state: point.physical_state,
                sensor_state: point.sensor_state,
                weight: to_f64(point.mass),
            })
            .collect();
        normalize_weights(&mut prior_particles)?;
        let particles = systematic_resample(
            &prior_particles,
            config.particle_count,
            &mut rng,
        )?;

        Ok(Self {
            kernel,
            particles,
            target_particle_count: config.particle_count,
            resample_ess_fraction: config.resample_ess_fraction,
            rng_state: rng.state,
            diagnostics: None,
            terminated_by_hazard: false,
        })
    }

    pub fn particles(&self) -> &[JointParticle] {
        &self.particles
    }

    pub fn diagnostics(&self) -> Option<ParticleFilterDiagnostics> {
        self.diagnostics
    }

    /// Whether a hazard observation has terminally ended this campaign.
    pub const fn is_terminated(&self) -> bool {
        self.terminated_by_hazard
    }

    /// Approximate posterior over unique joint hidden tuples.
    pub fn posterior(&self) -> Vec<ParticlePosteriorEntry> {
        let mut mass: BTreeMap<(usize, usize, usize), f64> = BTreeMap::new();
        for particle in &self.particles {
            let key = (
                particle.hypothesis,
                particle.physical_state,
                particle.sensor_state,
            );
            *mass.entry(key).or_insert(0.0) += particle.weight;
        }
        mass.into_iter()
            .map(|((hypothesis, physical_state, sensor_state), probability)| {
                ParticlePosteriorEntry {
                    hypothesis,
                    physical_state,
                    sensor_state,
                    probability,
                }
            })
            .collect()
    }

    /// Apply one public observation under the bound immutable kernel, preserving
    /// joint physical/sensor state.
    ///
    /// The update is atomic: an impossible observation or invalid state returns
    /// an error without changing particles, RNG state, or diagnostics.
    pub fn observe(
        &mut self,
        action: usize,
        observation: Option<usize>,
    ) -> Result<ParticleFilterDiagnostics, String> {
        if self.terminated_by_hazard {
            return Err("particle-filter campaign has already terminated on a hazard".into());
        }
        let kernel = self.kernel;
        kernel.validate()?;
        if action >= kernel.action_ids.len() {
            return Err("particle filter action index is out of range".into());
        }
        if observation.is_some_and(|id| id >= kernel.observation_ids.len()) {
            return Err("particle filter observation index is out of range".into());
        }
        self.validate_state()?;

        let mut next_rng = DeterministicRng {
            state: self.rng_state,
        };
        let mut updated = Vec::with_capacity(self.particles.len());
        let mut predictive_probability = 0.0_f64;
        let mut particles_without_event_support = 0_usize;

        for particle in &self.particles {
            let row_index = kernel.row_index(
                particle.hypothesis,
                particle.physical_state,
                particle.sensor_state,
                action,
            );
            let row = &kernel.rows[row_index];
            let mut likelihood = Rational::zero();
            for outcome in &row.outcomes {
                if outcome.observation == observation {
                    likelihood = likelihood.checked_add(outcome.probability)?;
                }
            }
            if likelihood.is_zero() {
                particles_without_event_support += 1;
                continue;
            }

            let likelihood_f64 = to_f64(likelihood);
            let unnormalized_weight = particle.weight * likelihood_f64;
            predictive_probability += unnormalized_weight;
            if !unnormalized_weight.is_finite() || unnormalized_weight <= 0.0 {
                return Err("particle filter encountered non-finite or zero weighted likelihood".into());
            }

            let selected = sample_conditioned_outcome(
                &row.outcomes,
                observation,
                likelihood,
                &mut next_rng,
            )?;
            updated.push(JointParticle {
                hypothesis: particle.hypothesis,
                physical_state: selected.next_physical_state,
                sensor_state: selected.next_sensor_state,
                weight: unnormalized_weight,
            });
        }

        if predictive_probability <= 0.0 || !predictive_probability.is_finite() {
            return Err("particle-support failure: observed event has no support in the current particle set; this is not evidence of model impossibility".into());
        }
        if updated.is_empty() {
            return Err("observation removed all particles".into());
        }

        for particle in &mut updated {
            particle.weight /= predictive_probability;
        }
        normalize_weights(&mut updated)?;
        let ess_before = effective_sample_size(&updated)?;
        let particle_count_before = updated.len();
        let should_resample = ess_before
            < self.resample_ess_fraction * self.target_particle_count as f64;

        let (particles, ess_after, resampled) = if should_resample {
            let resampled = systematic_resample(
                &updated,
                self.target_particle_count,
                &mut next_rng,
            )?;
            (
                resampled,
                self.target_particle_count as f64,
                true,
            )
        } else {
            (updated, ess_before, false)
        };

        let diagnostics = ParticleFilterDiagnostics {
            predictive_probability_estimate: predictive_probability,
            log_predictive_probability_estimate: predictive_probability.ln(),
            effective_sample_size_before_resampling: ess_before,
            effective_sample_size_after_resampling: ess_after,
            particles_before_resampling: particle_count_before,
            particles_after_update: particles.len(),
            particles_without_event_support,
            support_loss_fraction: particles_without_event_support as f64
                / self.particles.len() as f64,
            degenerate: ess_before <= 1.0 + 1e-9,
            terminal_hazard: observation.is_none(),
            resampled,
        };

        self.particles = particles;
        self.rng_state = next_rng.state;
        self.diagnostics = Some(diagnostics);
        self.terminated_by_hazard = observation.is_none();
        Ok(diagnostics)
    }

    /// Total-variation distance between the particle posterior and exact belief.
    pub fn total_variation_distance(
        &self,
        exact: &[JointMass],
    ) -> Result<f64, String> {
        let mut exact_mass: BTreeMap<(usize, usize, usize), f64> = BTreeMap::new();
        let mut exact_total = Rational::zero();
        for point in exact {
            let key = (
                point.hypothesis,
                point.physical_state,
                point.sensor_state,
            );
            *exact_mass.entry(key).or_insert(0.0) += to_f64(point.mass);
            exact_total = exact_total.checked_add(point.mass)?;
        }
        if exact_total != Rational::one() {
            return Err("exact comparison belief must sum exactly to one".into());
        }

        let approximate: BTreeMap<(usize, usize, usize), f64> = self
            .posterior()
            .into_iter()
            .map(|entry| {
                (
                    (
                        entry.hypothesis,
                        entry.physical_state,
                        entry.sensor_state,
                    ),
                    entry.probability,
                )
            })
            .collect();
        let keys: BTreeSet<_> = exact_mass
            .keys()
            .chain(approximate.keys())
            .copied()
            .collect();
        let distance = keys.iter().fold(0.0, |sum, key| {
            sum + (exact_mass.get(key).copied().unwrap_or(0.0)
                - approximate.get(key).copied().unwrap_or(0.0))
                .abs()
        }) * 0.5;
        Ok(distance)
    }

    fn validate_state(&self) -> Result<(), String> {
        let kernel = self.kernel;
        if self.particles.is_empty()
            || self.particles.len() > self.target_particle_count
            || self.target_particle_count == 0
        {
            return Err("particle filter has an invalid particle count".into());
        }
        let mut total_weight = 0.0_f64;
        for particle in &self.particles {
            if particle.hypothesis >= kernel.hypothesis_ids.len()
                || particle.physical_state >= kernel.physical_state_count
                || particle.sensor_state >= kernel.sensor_state_count
                || !particle.weight.is_finite()
                || particle.weight <= 0.0
            {
                return Err("particle filter contains an invalid joint state or weight".into());
            }
            total_weight += particle.weight;
        }
        if !total_weight.is_finite() || (total_weight - 1.0).abs() > 1e-9 {
            return Err("particle filter weights must be normalized".into());
        }
        Ok(())
    }
}

/// Exact Bayesian update for one observed outcome, used as a reference oracle.
pub fn exact_joint_posterior_after_observation(
    kernel: &JointKernel,
    belief: &[JointMass],
    action: usize,
    observation: Option<usize>,
) -> Result<(Rational, Vec<JointMass>), String> {
    kernel.validate()?;
    validate_joint_belief(kernel, belief)?;
    if action >= kernel.action_ids.len() {
        return Err("exact posterior action index is out of range".into());
    }
    if observation.is_some_and(|id| id >= kernel.observation_ids.len()) {
        return Err("exact posterior observation index is out of range".into());
    }

    let mut states: BTreeMap<(usize, usize, usize), Rational> = BTreeMap::new();
    for point in belief {
        let row_index = kernel.row_index(
            point.hypothesis,
            point.physical_state,
            point.sensor_state,
            action,
        );
        for outcome in &kernel.rows[row_index].outcomes {
            if outcome.observation != observation {
                continue;
            }
            let mass = point.mass.checked_mul(outcome.probability)?;
            let key = (
                point.hypothesis,
                outcome.next_physical_state,
                outcome.next_sensor_state,
            );
            let old = states.get(&key).copied().unwrap_or_else(Rational::zero);
            states.insert(key, old.checked_add(mass)?);
        }
    }

    let predictive_probability = states.values().try_fold(
        Rational::zero(),
        |sum, mass| sum.checked_add(*mass),
    )?;
    if predictive_probability.is_zero() {
        return Err("observed event is impossible under the exact joint belief".into());
    }
    let posterior = states
        .into_iter()
        .map(|((hypothesis, physical_state, sensor_state), mass)| {
            Ok(JointMass {
                hypothesis,
                physical_state,
                sensor_state,
                mass: mass.checked_div(predictive_probability)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((predictive_probability, posterior))
}

fn validate_joint_belief(
    kernel: &JointKernel,
    belief: &[JointMass],
) -> Result<(), String> {
    if belief.is_empty() || belief.len() > MAX_BELIEF_TUPLES {
        return Err("joint prior is empty or exceeds its tuple cap".into());
    }
    let mut tuples = BTreeSet::new();
    let mut total = Rational::zero();
    for point in belief {
        if point.hypothesis >= kernel.hypothesis_ids.len()
            || point.physical_state >= kernel.physical_state_count
            || point.sensor_state >= kernel.sensor_state_count
            || point.mass.is_zero()
        {
            return Err("joint prior contains an invalid tuple or non-positive mass".into());
        }
        if !tuples.insert((
            point.hypothesis,
            point.physical_state,
            point.sensor_state,
        )) {
            return Err("joint prior contains duplicate tuples".into());
        }
        total = total.checked_add(point.mass)?;
    }
    if total != Rational::one() {
        return Err("joint prior probability mass must sum exactly to one".into());
    }
    Ok(())
}

fn validate_config(config: ParticleFilterConfig) -> Result<(), String> {
    if config.particle_count == 0 || config.particle_count > MAX_PARTICLES {
        return Err(format!("particle count must be in 1..={MAX_PARTICLES}"));
    }
    if !config.resample_ess_fraction.is_finite()
        || !(0.0..=1.0).contains(&config.resample_ess_fraction)
    {
        return Err("resampling ESS fraction must lie in [0, 1]".into());
    }
    Ok(())
}


fn sample_conditioned_outcome(
    outcomes: &[super::JointOutcome],
    observation: Option<usize>,
    likelihood: Rational,
    rng: &mut DeterministicRng,
) -> Result<&super::JointOutcome, String> {
    let likelihood_f64 = to_f64(likelihood);
    let target = rng.next_f64() * likelihood_f64;
    let mut cumulative = 0.0;
    let mut last_match = None;
    for outcome in outcomes {
        if outcome.observation != observation {
            continue;
        }
        last_match = Some(outcome);
        cumulative += to_f64(outcome.probability);
        if target < cumulative {
            return Ok(outcome);
        }
    }
    last_match.ok_or_else(|| "conditioned event has no matching outcome".into())
}

fn normalize_weights(particles: &mut [JointParticle]) -> Result<(), String> {
    let total = particles.iter().try_fold(0.0_f64, |sum, particle| {
        let next = sum + particle.weight;
        if next.is_finite() {
            Ok(next)
        } else {
            Err("particle weight sum overflowed".to_string())
        }
    })?;
    if total <= 0.0 {
        return Err("particle weights have zero total mass".into());
    }
    for particle in particles {
        particle.weight /= total;
    }
    Ok(())
}

fn effective_sample_size(particles: &[JointParticle]) -> Result<f64, String> {
    let squared_weight_sum = particles.iter().try_fold(0.0_f64, |sum, particle| {
        let next = sum + particle.weight * particle.weight;
        if next.is_finite() {
            Ok(next)
        } else {
            Err("particle ESS calculation overflowed".to_string())
        }
    })?;
    if squared_weight_sum <= 0.0 {
        return Err("particle ESS is undefined for zero weights".into());
    }
    Ok(1.0 / squared_weight_sum)
}

fn systematic_resample(
    particles: &[JointParticle],
    target_count: usize,
    rng: &mut DeterministicRng,
) -> Result<Vec<JointParticle>, String> {
    if particles.is_empty() || target_count == 0 {
        return Err("systematic resampling requires non-empty input and target".into());
    }
    let step = 1.0 / target_count as f64;
    let start = rng.next_f64() * step;
    let mut output = Vec::with_capacity(target_count);
    let mut index = 0_usize;
    let mut cumulative = particles[0].weight;
    for sample_index in 0..target_count {
        let target = start + sample_index as f64 * step;
        while target >= cumulative && index + 1 < particles.len() {
            index += 1;
            cumulative += particles[index].weight;
        }
        let mut selected = particles[index].clone();
        selected.weight = step;
        output.push(selected);
    }
    Ok(output)
}

fn to_f64(value: Rational) -> f64 {
    value.numerator() as f64 / value.denominator() as f64
}

#[derive(Clone, Copy, Debug)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0xA076_1D64_78BD_642F
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.state = value;
        value.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JointMass, JointOutcome, KernelRow};

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn quarter() -> Rational {
        Rational::new(1, 4).unwrap()
    }

    fn three_quarters() -> Rational {
        Rational::new(3, 4).unwrap()
    }

    fn noisy_kernel() -> JointKernel {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let (p_zero, p_one) = if hypothesis == 0 {
                (three_quarters(), quarter())
            } else {
                (quarter(), three_quarters())
            };
            rows.push(KernelRow {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes: vec![
                    JointOutcome {
                        probability: p_zero,
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(0),
                    },
                    JointOutcome {
                        probability: p_one,
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(1),
                    },
                ],
            });
        }
        JointKernel {
            hypothesis_ids: vec!["h0".into(), "h1".into()],
            physical_state_count: 1,
            sensor_state_count: 1,
            action_ids: vec!["measure".into()],
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
        }
    }

    #[test]
    fn particle_filter_matches_exact_binary_channel_posterior_with_small_error() {
        let kernel = noisy_kernel();
        let (predictive, exact) = exact_joint_posterior_after_observation(
            &kernel,
            &kernel.initial_belief,
            0,
            Some(0),
        ).unwrap();
        assert_eq!(predictive, half());

        let mut filter = JointStateParticleFilter::from_joint_belief(
            &kernel,
            &kernel.initial_belief,
            ParticleFilterConfig {
                particle_count: 8192,
                resample_ess_fraction: 0.1,
                seed: 7,
            },
        ).unwrap();
        let diagnostics = filter.observe(0, Some(0)).unwrap();
        assert!(!diagnostics.resampled);
        assert!(diagnostics.effective_sample_size_before_resampling > 1000.0);
        assert!(filter.total_variation_distance(&exact).unwrap() < 0.04);
    }

    #[test]
    fn systematic_resampling_is_deterministic_and_reports_ess() {
        let kernel = noisy_kernel();
        let config = ParticleFilterConfig {
            particle_count: 256,
            resample_ess_fraction: 0.99,
            seed: 1234,
        };
        let mut left = JointStateParticleFilter::from_joint_belief(
            &kernel,
            &kernel.initial_belief,
            config,
        ).unwrap();
        let mut right = left.clone();
        let left_diag = left.observe(0, Some(0)).unwrap();
        let right_diag = right.observe(0, Some(0)).unwrap();
        assert_eq!(left, right);
        assert_eq!(left_diag, right_diag);
        assert!(left_diag.resampled);
        assert_eq!(left_diag.particles_after_update, config.particle_count);
        assert!(
            (left_diag.effective_sample_size_after_resampling
                - config.particle_count as f64)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn impossible_observation_fails_without_mutating_particle_state() {
        let mut kernel = noisy_kernel();
        for row in &mut kernel.rows {
            row.outcomes.retain(|outcome| outcome.observation == Some(0));
            row.outcomes[0].probability = Rational::one();
        }
        let mut filter = JointStateParticleFilter::from_joint_belief(
            &kernel,
            &kernel.initial_belief,
            ParticleFilterConfig {
                particle_count: 64,
                resample_ess_fraction: 0.5,
                seed: 4,
            },
        ).unwrap();
        let before = filter.clone();
        assert!(filter.observe(0, Some(1)).is_err());
        assert_eq!(filter, before);
    }

    #[test]
    fn particle_count_sweep_reports_exact_posterior_error_and_ess() {
        let kernel = noisy_kernel();
        let records = benchmark_particle_counts(
            &kernel,
            &kernel.initial_belief,
            0,
            Some(0),
            &[32, 128, 512],
            0.5,
            0x51EE_D123,
        ).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(
            records.iter().map(|record| record.particle_count).collect::<Vec<_>>(),
            vec![32, 128, 512]
        );
        assert!(records.iter().all(|record| {
            record.predictive_probability_exact == half()
                && record.joint_posterior_total_variation_distance.is_finite()
                && record.joint_posterior_squared_error.is_finite()
                && record.mechanism_posterior_squared_error.is_finite()
                && record.effective_sample_size_before_resampling >= 1.0
                && record.elapsed_nanos > 0
        }));

        let repeat = benchmark_particle_counts(
            &kernel,
            &kernel.initial_belief,
            0,
            Some(0),
            &[32, 128, 512],
            0.5,
            0x51EE_D123,
        ).unwrap();
        for (left, right) in records.iter().zip(repeat.iter()) {
            assert_eq!(
                left.predictive_probability_estimate,
                right.predictive_probability_estimate
            );
            assert_eq!(
                left.joint_posterior_total_variation_distance,
                right.joint_posterior_total_variation_distance
            );
            assert_eq!(
                left.effective_sample_size_before_resampling,
                right.effective_sample_size_before_resampling
            );
        }
    }

    #[test]
    fn support_loss_diagnostics_distinguish_particle_support_from_model_probability() {
        let mut kernel = noisy_kernel();
        // A third observation is structurally possible only under h1.
        kernel.observation_ids.push("rare".into());
        for row in &mut kernel.rows {
            if row.hypothesis == 1 {
                row.outcomes.push(JointOutcome {
                    probability: Rational::new(1, 4).unwrap(),
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(2),
                });
                row.outcomes[0].probability = Rational::new(1, 2).unwrap();
                row.outcomes[1].probability = Rational::new(1, 4).unwrap();
            }
        }
        // Initial systematic sampling exactly splits the 2-hypothesis prior,
        // so this event loses support from all h0 particles but retains h1 support.
        let mut filter = JointStateParticleFilter::from_joint_belief(
            &kernel,
            &kernel.initial_belief,
            ParticleFilterConfig {
                particle_count: 256,
                resample_ess_fraction: 0.1,
                seed: 9,
            },
        ).unwrap();
        let diagnostics = filter.observe(0, Some(2)).unwrap();
        assert_eq!(diagnostics.particles_without_event_support, 128);
        assert!((diagnostics.support_loss_fraction - 0.5).abs() < 1e-12);
        assert!(!diagnostics.degenerate);
        assert_eq!(filter.posterior().len(), 1);
        assert_eq!(filter.posterior()[0].hypothesis, 1);
    }

    #[test]
    fn calibration_updates_sensor_state_without_destroying_mechanism_physical_correlation() {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            for physical_state in 0..2 {
                for sensor_state in 0..2 {
                    for action in 0..2 {
                        let (next_physical, next_sensor, observation) = if action == 0 {
                            (physical_state, sensor_state, physical_state ^ sensor_state)
                        } else {
                            (physical_state, 0, 0)
                        };
                        rows.push(KernelRow {
                            hypothesis,
                            physical_state,
                            sensor_state,
                            action,
                            outcomes: vec![JointOutcome {
                                probability: Rational::one(),
                                next_physical_state: next_physical,
                                next_sensor_state: next_sensor,
                                observation: Some(observation),
                            }],
                        });
                    }
                }
            }
        }
        let kernel = JointKernel {
            hypothesis_ids: vec!["h0".into(), "h1".into()],
            physical_state_count: 2,
            sensor_state_count: 2,
            action_ids: vec!["measure".into(), "calibrate".into()],
            observation_ids: vec!["zero".into(), "one".into()],
            action_costs: vec![1, 1],
            initial_belief: vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 1,
                    mass: half(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 1,
                    sensor_state: 0,
                    mass: half(),
                },
            ],
            rows,
        };
        let mut filter = JointStateParticleFilter::from_joint_belief(
            &kernel,
            &kernel.initial_belief,
            ParticleFilterConfig {
                particle_count: 1024,
                resample_ess_fraction: 0.25,
                seed: 91,
            },
        ).unwrap();

        let (exact_probability, exact_posterior) =
            exact_joint_posterior_after_observation(
                &kernel,
                &kernel.initial_belief,
                1,
                Some(0),
            ).unwrap();
        assert_eq!(exact_probability, Rational::one());

        filter.observe(1, Some(0)).unwrap();
        assert_eq!(filter.total_variation_distance(&exact_posterior).unwrap(), 0.0);

        let posterior = filter.posterior();
        assert_eq!(posterior.len(), 2);
        assert!(posterior.iter().all(|entry| entry.sensor_state == 0));
        assert!(posterior.iter().any(|entry| {
            entry.hypothesis == 0
                && entry.physical_state == 0
                && (entry.probability - 0.5).abs() < 1e-12
        }));
        assert!(posterior.iter().any(|entry| {
            entry.hypothesis == 1
                && entry.physical_state == 1
                && (entry.probability - 0.5).abs() < 1e-12
        }));
    }

    #[test]
    fn rare_but_possible_event_is_reported_as_particle_support_failure() {
        let kernel = JointKernel {
            hypothesis_ids: vec!["common".into(), "rare".into()],
            physical_state_count: 1,
            sensor_state_count: 1,
            action_ids: vec!["measure".into()],
            observation_ids: vec!["common-event".into(), "rare-event".into()],
            action_costs: vec![1],
            initial_belief: vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: Rational::new(99, 100).unwrap(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: Rational::new(1, 100).unwrap(),
                },
            ],
            rows: vec![
                KernelRow {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    action: 0,
                    outcomes: vec![JointOutcome {
                        probability: Rational::one(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(0),
                    }],
                },
                KernelRow {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    action: 0,
                    outcomes: vec![JointOutcome {
                        probability: Rational::one(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(1),
                    }],
                },
            ],
        };
        let (exact_predictive, _) = exact_joint_posterior_after_observation(
            &kernel,
            &kernel.initial_belief,
            0,
            Some(1),
        ).unwrap();
        assert_eq!(exact_predictive, Rational::new(1, 100).unwrap());

        // A single particle and this deterministic seed select the 99% state.
        // The 1% observation is possible under the exact model but absent from
        // this approximate particle support.
        let mut filter = JointStateParticleFilter::from_joint_belief(
            &kernel,
            &kernel.initial_belief,
            ParticleFilterConfig {
                particle_count: 1,
                resample_ess_fraction: 0.5,
                seed: 1,
            },
        ).unwrap();
        let before = filter.clone();
        let error = filter.observe(0, Some(1)).unwrap_err();
        assert!(error.contains("particle-support failure"));
        assert_eq!(filter, before);
    }

    #[test]
    fn hazard_observation_terminates_particle_filter_campaign() {
        let kernel = JointKernel {
            hypothesis_ids: vec!["h0".into()],
            physical_state_count: 1,
            sensor_state_count: 1,
            action_ids: vec!["dangerous-test".into()],
            observation_ids: vec!["safe".into()],
            action_costs: vec![1],
            initial_belief: vec![JointMass {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                mass: Rational::one(),
            }],
            rows: vec![KernelRow {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes: vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: None,
                }],
            }],
        };
        let mut filter = JointStateParticleFilter::from_joint_belief(
            &kernel,
            &kernel.initial_belief,
            ParticleFilterConfig {
                particle_count: 32,
                resample_ess_fraction: 0.5,
                seed: 5,
            },
        ).unwrap();
        let diagnostics = filter.observe(0, None).unwrap();
        assert!(diagnostics.terminal_hazard);
        assert!(filter.is_terminated());
        let before = filter.clone();
        assert!(filter.observe(0, None).is_err());
        assert_eq!(filter, before);
    }

    #[test]
    fn invalid_configuration_and_joint_prior_fail_closed() {
        let kernel = noisy_kernel();
        assert!(JointStateParticleFilter::from_joint_belief(
            &kernel,
            &kernel.initial_belief,
            ParticleFilterConfig {
                particle_count: 0,
                ..ParticleFilterConfig::default()
            },
        ).is_err());
        assert!(JointStateParticleFilter::from_joint_belief(
            &kernel,
            &kernel.initial_belief,
            ParticleFilterConfig {
                particle_count: 32,
                resample_ess_fraction: f64::NAN,
                seed: 1,
            },
        ).is_err());

        let mut bad_prior = kernel.initial_belief.clone();
        bad_prior[0].mass = quarter();
        assert!(JointStateParticleFilter::from_joint_belief(
            &kernel,
            &bad_prior,
            ParticleFilterConfig::default(),
        ).is_err());
    }
}
