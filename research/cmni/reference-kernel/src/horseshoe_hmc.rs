//! Fixed-step Hamiltonian Monte Carlo comparator for the raw-scale horseshoe target.
//!
//! This is a small research sampler, not a reproduction of the AIACHNE BMC
//! implementation and not a production NUTS sampler. Step size is fixed by the
//! caller; discarded transitions are not adapted. Convergence must be assessed
//! across multiple independently seeded chains using qualified rank-normalized
//! diagnostics before interpreting posterior summaries.

use crate::{
    GlobalScalePrior, ModelError, PreparedRawScaleHorseshoeModel, RawScaleHorseshoePosterior,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorseshoeHmcConfig {
    pub discarded_transitions: usize,
    pub retained_draws: usize,
    pub step_size: f64,
    pub leapfrog_steps: usize,
    pub divergence_energy_error_threshold: f64,
    pub seed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HorseshoeHmcDraw {
    pub coefficients: Vec<f64>,
    pub log_local_scales: Vec<f64>,
    pub log_global_scale: f64,
    pub log_posterior: f64,
    /// Joint Hamiltonian energy at the start of the transition producing this draw.
    pub hamiltonian_energy: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HorseshoeHmcChain {
    /// Seed used for the chain's momentum stream.
    pub seed: u64,
    pub draws: Vec<HorseshoeHmcDraw>,
    pub transitions: usize,
    pub accepted_transitions: usize,
    pub divergences: usize,
    pub acceptance_rate: f64,
}

#[derive(Debug, Clone)]
struct HmcState {
    coefficients: Vec<f64>,
    log_local_scales: Vec<f64>,
    log_global_scale: f64,
}

impl HmcState {
    fn flatten(&self) -> Vec<f64> {
        let mut values =
            Vec::with_capacity(self.coefficients.len() + self.log_local_scales.len() + 1);
        values.extend_from_slice(&self.coefficients);
        values.extend_from_slice(&self.log_local_scales);
        values.push(self.log_global_scale);
        values
    }

    fn from_flat(values: &[f64], coefficient_count: usize) -> Result<Self, ModelError> {
        if values.len() != coefficient_count * 2 + 1 {
            return Err(ModelError::MismatchedDataLengths);
        }
        Ok(Self {
            coefficients: values[..coefficient_count].to_vec(),
            log_local_scales: values[coefficient_count..2 * coefficient_count].to_vec(),
            log_global_scale: values[2 * coefficient_count],
        })
    }

    fn draw(&self, log_posterior: f64, hamiltonian_energy: f64) -> HorseshoeHmcDraw {
        HorseshoeHmcDraw {
            coefficients: self.coefficients.clone(),
            log_local_scales: self.log_local_scales.clone(),
            log_global_scale: self.log_global_scale,
            log_posterior,
            hamiltonian_energy,
        }
    }
}

/// Run one reproducible fixed-step HMC chain.
///
/// The caller should create several chains with independent seeds and overdispersed
/// initial states. This implementation performs no dual averaging, mass-matrix
/// adaptation, NUTS tree building, or rank-normalized convergence analysis.
pub fn run_horseshoe_hmc_chain(
    model: &PreparedRawScaleHorseshoeModel,
    initial_coefficients: &[f64],
    initial_log_local_scales: &[f64],
    initial_log_global_scale: f64,
    global_scale_prior: GlobalScalePrior,
    config: HorseshoeHmcConfig,
) -> Result<HorseshoeHmcChain, ModelError> {
    let coefficient_count = model.coefficient_count();
    if initial_coefficients.len() != coefficient_count
        || initial_log_local_scales.len() != coefficient_count
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if config.retained_draws == 0 || config.leapfrog_steps == 0 {
        return Err(ModelError::InvalidParameter(
            "hmc_draw_and_leapfrog_counts_must_be_positive",
        ));
    }
    if !config.step_size.is_finite() {
        return Err(ModelError::NonFiniteValue("hmc_step_size"));
    }
    if config.step_size <= 0.0 {
        return Err(ModelError::NonPositiveValue("hmc_step_size"));
    }
    if !config.divergence_energy_error_threshold.is_finite() {
        return Err(ModelError::NonFiniteValue("hmc_divergence_threshold"));
    }
    if config.divergence_energy_error_threshold <= 0.0 {
        return Err(ModelError::NonPositiveValue("hmc_divergence_threshold"));
    }
    let transitions = config
        .discarded_transitions
        .checked_add(config.retained_draws)
        .ok_or(ModelError::InvalidParameter(
            "hmc_transition_count_overflow",
        ))?;

    let mut state = HmcState {
        coefficients: initial_coefficients.to_vec(),
        log_local_scales: initial_log_local_scales.to_vec(),
        log_global_scale: initial_log_global_scale,
    };
    let mut target = evaluate_state(model, &state, global_scale_prior)?;
    let mut rng = SplitMix64::new(config.seed);
    let mut draws = Vec::with_capacity(config.retained_draws);
    let mut accepted_transitions = 0usize;
    let mut divergences = 0usize;

    for transition in 0..transitions {
        let (next_state, next_target, accepted, divergent, sampled_energy) =
            hmc_transition(model, &state, &target, global_scale_prior, config, &mut rng);
        if accepted {
            state = next_state;
            target = next_target;
            accepted_transitions += 1;
        }
        if divergent {
            divergences += 1;
        }
        if transition >= config.discarded_transitions {
            draws.push(state.draw(target.log_posterior, sampled_energy));
        }
    }

    Ok(HorseshoeHmcChain {
        seed: config.seed,
        draws,
        transitions,
        accepted_transitions,
        divergences,
        acceptance_rate: accepted_transitions as f64 / transitions as f64,
    })
}

fn hmc_transition(
    model: &PreparedRawScaleHorseshoeModel,
    current_state: &HmcState,
    current_target: &RawScaleHorseshoePosterior,
    prior: GlobalScalePrior,
    config: HorseshoeHmcConfig,
    rng: &mut SplitMix64,
) -> (HmcState, RawScaleHorseshoePosterior, bool, bool, f64) {
    let current = current_state.flatten();
    let dimension = current.len();
    let mut position = current.clone();
    let mut momentum: Vec<f64> = (0..dimension).map(|_| rng.normal()).collect();
    let initial_hamiltonian = -current_target.log_posterior + kinetic_energy(&momentum);

    let current_gradient = flatten_gradient(current_target);
    for index in 0..dimension {
        momentum[index] += 0.5 * config.step_size * current_gradient[index];
    }

    let mut proposal_target = current_target.clone();
    for step in 0..config.leapfrog_steps {
        for index in 0..dimension {
            position[index] += config.step_size * momentum[index];
        }
        let proposal_state = match HmcState::from_flat(&position, current_state.coefficients.len())
        {
            Ok(state) => state,
            Err(_) => {
                return (
                    current_state.clone(),
                    current_target.clone(),
                    false,
                    true,
                    initial_hamiltonian,
                )
            }
        };
        proposal_target = match evaluate_state(model, &proposal_state, prior) {
            Ok(target) => target,
            Err(_) => {
                return (
                    current_state.clone(),
                    current_target.clone(),
                    false,
                    true,
                    initial_hamiltonian,
                )
            }
        };
        let gradient = flatten_gradient(&proposal_target);
        let scale = if step + 1 == config.leapfrog_steps {
            0.5
        } else {
            1.0
        };
        for index in 0..dimension {
            momentum[index] += scale * config.step_size * gradient[index];
        }
    }

    // Momentum reversal makes the deterministic leapfrog proposal involutive.
    for value in &mut momentum {
        *value = -*value;
    }
    let proposal_hamiltonian = -proposal_target.log_posterior + kinetic_energy(&momentum);
    let energy_error = proposal_hamiltonian - initial_hamiltonian;
    if !energy_error.is_finite() || energy_error.abs() > config.divergence_energy_error_threshold {
        return (
            current_state.clone(),
            current_target.clone(),
            false,
            true,
            initial_hamiltonian,
        );
    }
    let log_acceptance = -energy_error;
    let accepted = log_acceptance >= 0.0 || rng.uniform_open().ln() < log_acceptance;
    if accepted {
        let next_state = match HmcState::from_flat(&position, current_state.coefficients.len()) {
            Ok(state) => state,
            Err(_) => {
                return (
                    current_state.clone(),
                    current_target.clone(),
                    false,
                    true,
                    initial_hamiltonian,
                )
            }
        };
        (
            next_state,
            proposal_target,
            true,
            false,
            initial_hamiltonian,
        )
    } else {
        (
            current_state.clone(),
            current_target.clone(),
            false,
            false,
            initial_hamiltonian,
        )
    }
}

fn evaluate_state(
    model: &PreparedRawScaleHorseshoeModel,
    state: &HmcState,
    prior: GlobalScalePrior,
) -> Result<RawScaleHorseshoePosterior, ModelError> {
    model.evaluate(
        &state.coefficients,
        &state.log_local_scales,
        state.log_global_scale,
        prior,
    )
}

fn flatten_gradient(target: &RawScaleHorseshoePosterior) -> Vec<f64> {
    let mut gradient = Vec::with_capacity(
        target.coefficient_gradient.len() + target.log_local_scale_gradient.len() + 1,
    );
    gradient.extend_from_slice(&target.coefficient_gradient);
    gradient.extend_from_slice(&target.log_local_scale_gradient);
    gradient.push(target.log_global_scale_gradient);
    gradient
}

fn kinetic_energy(momentum: &[f64]) -> f64 {
    0.5 * momentum.iter().map(|value| value * value).sum::<f64>()
}

#[derive(Debug, Clone)]
pub(crate) struct SplitMix64 {
    state: u64,
    spare_normal: Option<f64>,
}

impl SplitMix64 {
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            state: seed,
            spare_normal: None,
        }
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    pub(crate) fn uniform_open(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        (mantissa as f64 + 0.5) / 9_007_199_254_740_992.0
    }

    pub(crate) fn normal(&mut self) -> f64 {
        if let Some(value) = self.spare_normal.take() {
            return value;
        }
        let first = self.uniform_open();
        let second = self.uniform_open();
        let radius = (-2.0 * first.ln()).sqrt();
        let angle = std::f64::consts::TAU * second;
        self.spare_normal = Some(radius * angle.sin());
        radius * angle.cos()
    }
}

/// Classic split-Rhat for one coefficient across independently run chains.
///
/// This basic unranked diagnostic is a screening metric only. It is not the
/// rank-normalized, folded Rhat recommended for modern Bayesian analysis.
/// Chains must use independent seeds and dispersed starting states.
pub fn split_rhat_for_coefficient(
    chains: &[HorseshoeHmcChain],
    coefficient_index: usize,
) -> Result<f64, ModelError> {
    if chains.len() < 2 {
        return Err(ModelError::MismatchedDataLengths);
    }
    let draws_per_chain = chains
        .iter()
        .map(|chain| chain.draws.len())
        .min()
        .unwrap_or(0);
    let half = draws_per_chain / 2;
    if half < 2 {
        return Err(ModelError::MismatchedDataLengths);
    }
    for chain in chains {
        if chain
            .draws
            .iter()
            .any(|draw| coefficient_index >= draw.coefficients.len())
        {
            return Err(ModelError::MismatchedDataLengths);
        }
    }

    let mut split_chains: Vec<Vec<f64>> = Vec::with_capacity(chains.len() * 2);
    for chain in chains {
        split_chains.push(
            chain.draws[..half]
                .iter()
                .map(|draw| draw.coefficients[coefficient_index])
                .collect(),
        );
        split_chains.push(
            chain.draws[draws_per_chain - half..draws_per_chain]
                .iter()
                .map(|draw| draw.coefficients[coefficient_index])
                .collect(),
        );
    }
    let split_count = split_chains.len();
    let means: Vec<f64> = split_chains
        .iter()
        .map(|values| values.iter().sum::<f64>() / half as f64)
        .collect();
    let variances: Vec<f64> = split_chains
        .iter()
        .zip(means.iter())
        .map(|(values, mean)| {
            values
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / (half - 1) as f64
        })
        .collect();
    let within = variances.iter().sum::<f64>() / split_count as f64;
    let mean_of_means = means.iter().sum::<f64>() / split_count as f64;
    let between = half as f64
        * means
            .iter()
            .map(|mean| (mean - mean_of_means).powi(2))
            .sum::<f64>()
        / (split_count - 1) as f64;
    if within == 0.0 {
        return if between == 0.0 {
            Ok(1.0)
        } else {
            Ok(f64::INFINITY)
        };
    }
    let variance_estimate = (half as f64 - 1.0) / half as f64 * within + between / half as f64;
    Ok((variance_estimate / within).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_model() -> PreparedRawScaleHorseshoeModel {
        PreparedRawScaleHorseshoeModel::new(
            &[11.0, 19.0],
            &[10.0, 20.0],
            &[vec![0.0], vec![1.0]],
            &[vec![0.04, 0.01], vec![0.01, 0.09]],
        )
        .expect("prepared model")
    }

    fn config(seed: u64) -> HorseshoeHmcConfig {
        HorseshoeHmcConfig {
            discarded_transitions: 3,
            retained_draws: 8,
            step_size: 0.005,
            leapfrog_steps: 4,
            divergence_energy_error_threshold: 100.0,
            seed,
        }
    }

    #[test]
    fn same_seed_replays_identical_chain() {
        let model = small_model();
        let prior = GlobalScalePrior::HalfNormal { scale: 0.5 };
        let first = run_horseshoe_hmc_chain(&model, &[0.1], &[0.0], -1.0, prior, config(1234))
            .expect("first chain");
        let second = run_horseshoe_hmc_chain(&model, &[0.1], &[0.0], -1.0, prior, config(1234))
            .expect("second chain");
        assert_eq!(first, second);
    }

    #[test]
    fn chain_receipt_preserves_the_random_seed() {
        let seed = 0xA11CE_u64;
        let chain = run_horseshoe_hmc_chain(
            &small_model(),
            &[0.1],
            &[0.0],
            -1.0,
            GlobalScalePrior::HalfNormal { scale: 0.5 },
            config(seed),
        )
        .expect("chain");
        assert_eq!(chain.seed, seed);
    }

    #[test]
    fn chain_records_requested_draws_and_valid_acceptance_rate() {
        let chain = run_horseshoe_hmc_chain(
            &small_model(),
            &[0.1],
            &[0.0],
            -1.0,
            GlobalScalePrior::HalfNormal { scale: 0.5 },
            config(99),
        )
        .expect("chain");
        assert_eq!(chain.draws.len(), 8);
        assert_eq!(chain.transitions, 11);
        assert!((0.0..=1.0).contains(&chain.acceptance_rate));
        assert!(chain
            .draws
            .iter()
            .all(|draw| draw.log_posterior.is_finite()));
    }

    #[test]
    fn split_rhat_is_one_for_identical_constant_chains() {
        let draw = HorseshoeHmcDraw {
            coefficients: vec![0.25],
            log_local_scales: vec![0.0],
            log_global_scale: -1.0,
            log_posterior: -3.0,
            hamiltonian_energy: 4.0,
        };
        let chain = HorseshoeHmcChain {
            seed: 42,
            draws: vec![draw.clone(); 8],
            transitions: 8,
            accepted_transitions: 8,
            divergences: 0,
            acceptance_rate: 1.0,
        };
        assert_eq!(
            split_rhat_for_coefficient(&[chain.clone(), chain], 0),
            Ok(1.0)
        );
    }

    #[test]
    fn split_rhat_detects_separated_chains() {
        let first = HorseshoeHmcChain {
            seed: 100,
            draws: (0..8)
                .map(|_| HorseshoeHmcDraw {
                    coefficients: vec![0.0],
                    log_local_scales: vec![0.0],
                    log_global_scale: -1.0,
                    log_posterior: -1.0,
                    hamiltonian_energy: 2.0,
                })
                .collect(),
            transitions: 8,
            accepted_transitions: 8,
            divergences: 0,
            acceptance_rate: 1.0,
        };
        let second = HorseshoeHmcChain {
            seed: 101,
            draws: (0..8)
                .map(|_| HorseshoeHmcDraw {
                    coefficients: vec![10.0],
                    log_local_scales: vec![0.0],
                    log_global_scale: -1.0,
                    log_posterior: -1.0,
                    hamiltonian_energy: 2.0,
                })
                .collect(),
            transitions: 8,
            accepted_transitions: 8,
            divergences: 0,
            acceptance_rate: 1.0,
        };
        assert!(split_rhat_for_coefficient(&[first, second], 0).expect("Rhat") > 1.1);
    }

    #[test]
    fn rejects_invalid_configuration_and_insufficient_chains() {
        let mut bad = config(1);
        bad.step_size = 0.0;
        assert_eq!(
            run_horseshoe_hmc_chain(
                &small_model(),
                &[0.1],
                &[0.0],
                -1.0,
                GlobalScalePrior::HalfNormal { scale: 0.5 },
                bad,
            ),
            Err(ModelError::NonPositiveValue("hmc_step_size"))
        );
        assert_eq!(
            split_rhat_for_coefficient(&[], 0),
            Err(ModelError::MismatchedDataLengths)
        );
    }
}
