//! Fixed-step HMC for grouped AIACHNE-style horseshoe posterior targets.
//!
//! Unlike the scalar comparator in `horseshoe_hmc.rs`, this state includes
//! multiple global scales and optional experiment normalization scales. It
//! evaluates a prepared model so the observation covariance factorization is
//! reused across all leapfrog substeps.
//!
//! This is fixed-step HMC. It does not implement NUTS, dual-averaging step-size
//! adaptation, mass-matrix adaptation, or the original AIACHNE Bayesian Monte
//! Carlo algorithm. Independent-chain qualification is separate.

use crate::horseshoe_hmc::SplitMix64;
use crate::GroupedRawScaleHorseshoePosterior;
use crate::{
    GlobalScalePrior, HorseshoeHmcConfig, ModelError, PreparedGroupedRawScaleHorseshoeModel,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub struct GroupedHmcDraw {
    pub coefficients: Vec<f64>,
    pub log_local_scales: Vec<f64>,
    pub log_global_scales: Vec<f64>,
    pub log_experiment_scales: Vec<f64>,
    pub log_posterior: f64,
    /// Joint Hamiltonian energy at the beginning of the transition producing
    /// this retained draw.
    pub hamiltonian_energy: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupedHmcChain {
    pub seed: u64,
    pub draws: Vec<GroupedHmcDraw>,
    pub transitions: usize,
    pub accepted_transitions: usize,
    pub divergences: usize,
    pub acceptance_rate: f64,
}

/// Independently seeded starting state for one grouped HMC chain.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupedHmcInitialization {
    pub seed: u64,
    pub initial_coefficients: Vec<f64>,
    pub initial_log_local_scales: Vec<f64>,
    pub initial_log_global_scales: Vec<f64>,
    pub initial_log_experiment_scales: Vec<f64>,
}

#[derive(Debug, Clone)]
struct GroupedHmcState {
    coefficients: Vec<f64>,
    log_local_scales: Vec<f64>,
    log_global_scales: Vec<f64>,
    log_experiment_scales: Vec<f64>,
}

impl GroupedHmcState {
    fn flatten(&self) -> Vec<f64> {
        let capacity = self.coefficients.len()
            + self.log_local_scales.len()
            + self.log_global_scales.len()
            + self.log_experiment_scales.len();
        let mut values = Vec::with_capacity(capacity);
        values.extend_from_slice(&self.coefficients);
        values.extend_from_slice(&self.log_local_scales);
        values.extend_from_slice(&self.log_global_scales);
        values.extend_from_slice(&self.log_experiment_scales);
        values
    }

    fn from_flat(
        values: &[f64],
        coefficient_count: usize,
        global_scale_count: usize,
        experiment_scale_count: usize,
    ) -> Result<Self, ModelError> {
        let expected = coefficient_count
            .checked_mul(2)
            .and_then(|v| v.checked_add(global_scale_count))
            .and_then(|v| v.checked_add(experiment_scale_count))
            .ok_or(ModelError::InvalidParameter(
                "grouped_hmc_state_dimension_overflow",
            ))?;
        if values.len() != expected || coefficient_count == 0 || global_scale_count == 0 {
            return Err(ModelError::MismatchedDataLengths);
        }
        let local_start = coefficient_count;
        let global_start = coefficient_count * 2;
        let experiment_start = global_start + global_scale_count;
        Ok(Self {
            coefficients: values[..local_start].to_vec(),
            log_local_scales: values[local_start..global_start].to_vec(),
            log_global_scales: values[global_start..experiment_start].to_vec(),
            log_experiment_scales: values[experiment_start..].to_vec(),
        })
    }

    fn draw(&self, log_posterior: f64, hamiltonian_energy: f64) -> GroupedHmcDraw {
        GroupedHmcDraw {
            coefficients: self.coefficients.clone(),
            log_local_scales: self.log_local_scales.clone(),
            log_global_scales: self.log_global_scales.clone(),
            log_experiment_scales: self.log_experiment_scales.clone(),
            log_posterior,
            hamiltonian_energy,
        }
    }
}

/// Run one reproducible fixed-step grouped HMC chain.
///
/// `global_scale_index_by_coefficient` and the experiment observation mapping
/// must be frozen before sampling. Initial states should be independently
/// dispersed across chains, and each chain must use a unique seed.
#[allow(clippy::too_many_arguments)]
pub fn run_grouped_horseshoe_hmc_chain(
    model: &PreparedGroupedRawScaleHorseshoeModel,
    initial_coefficients: &[f64],
    initial_log_local_scales: &[f64],
    initial_log_global_scales: &[f64],
    initial_log_experiment_scales: &[f64],
    global_scale_index_by_coefficient: &[usize],
    experiment_scale_index_by_observation: &[usize],
    fixed_coefficient_scale: f64,
    global_scale_prior: GlobalScalePrior,
    config: HorseshoeHmcConfig,
) -> Result<GroupedHmcChain, ModelError> {
    let coefficient_count = model.coefficient_count();
    let global_scale_count = initial_log_global_scales.len();
    let experiment_scale_count = initial_log_experiment_scales.len();
    if initial_coefficients.len() != coefficient_count
        || initial_log_local_scales.len() != coefficient_count
        || global_scale_count == 0
        || global_scale_index_by_coefficient.len() != coefficient_count
        || (experiment_scale_count == 0 && !experiment_scale_index_by_observation.is_empty())
        || (experiment_scale_count > 0
            && experiment_scale_index_by_observation.len() != model.observation_count())
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if config.retained_draws == 0 || config.leapfrog_steps == 0 {
        return Err(ModelError::InvalidParameter(
            "grouped_hmc_draw_and_leapfrog_counts_must_be_positive",
        ));
    }
    if !config.step_size.is_finite() {
        return Err(ModelError::NonFiniteValue("grouped_hmc_step_size"));
    }
    if config.step_size <= 0.0 {
        return Err(ModelError::NonPositiveValue("grouped_hmc_step_size"));
    }
    if !config.divergence_energy_error_threshold.is_finite() {
        return Err(ModelError::NonFiniteValue(
            "grouped_hmc_divergence_threshold",
        ));
    }
    if config.divergence_energy_error_threshold <= 0.0 {
        return Err(ModelError::NonPositiveValue(
            "grouped_hmc_divergence_threshold",
        ));
    }
    let transitions = config
        .discarded_transitions
        .checked_add(config.retained_draws)
        .ok_or(ModelError::InvalidParameter(
            "grouped_hmc_transition_count_overflow",
        ))?;

    let mut state = GroupedHmcState {
        coefficients: initial_coefficients.to_vec(),
        log_local_scales: initial_log_local_scales.to_vec(),
        log_global_scales: initial_log_global_scales.to_vec(),
        log_experiment_scales: initial_log_experiment_scales.to_vec(),
    };
    let mut target = evaluate_state(
        model,
        &state,
        global_scale_index_by_coefficient,
        experiment_scale_index_by_observation,
        fixed_coefficient_scale,
        global_scale_prior,
    )?;
    let mut rng = SplitMix64::new(config.seed);
    let mut draws = Vec::with_capacity(config.retained_draws);
    let mut accepted_transitions = 0usize;
    let mut divergences = 0usize;

    for transition in 0..transitions {
        let (next_state, next_target, accepted, divergent, sampled_energy) = transition_state(
            model,
            &state,
            &target,
            global_scale_index_by_coefficient,
            experiment_scale_index_by_observation,
            fixed_coefficient_scale,
            global_scale_prior,
            config,
            &mut rng,
        );
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

    Ok(GroupedHmcChain {
        seed: config.seed,
        draws,
        transitions,
        accepted_transitions,
        divergences,
        acceptance_rate: accepted_transitions as f64 / transitions as f64,
    })
}

/// Run at least four independently seeded grouped fixed-step HMC chains.
///
/// The initialization list supplies the random seed and starting state for
/// each chain. Seeds must be unique. This runner does not adapt step sizes or
/// construct NUTS trees; convergence qualification is performed separately.
#[allow(clippy::too_many_arguments)]
pub fn run_grouped_horseshoe_hmc_chains(
    model: &PreparedGroupedRawScaleHorseshoeModel,
    initializations: &[GroupedHmcInitialization],
    global_scale_index_by_coefficient: &[usize],
    experiment_scale_index_by_observation: &[usize],
    fixed_coefficient_scale: f64,
    global_scale_prior: GlobalScalePrior,
    config: HorseshoeHmcConfig,
) -> Result<Vec<GroupedHmcChain>, ModelError> {
    if initializations.len() < 4 {
        return Err(ModelError::InvalidParameter(
            "grouped_hmc_requires_at_least_four_chains",
        ));
    }
    let mut seeds = BTreeSet::new();
    for initialization in initializations {
        if !seeds.insert(initialization.seed) {
            return Err(ModelError::InvalidParameter(
                "grouped_hmc_chain_seeds_must_be_unique",
            ));
        }
    }

    let mut chains = Vec::with_capacity(initializations.len());
    for initialization in initializations {
        let chain_config = HorseshoeHmcConfig {
            seed: initialization.seed,
            ..config
        };
        chains.push(run_grouped_horseshoe_hmc_chain(
            model,
            &initialization.initial_coefficients,
            &initialization.initial_log_local_scales,
            &initialization.initial_log_global_scales,
            &initialization.initial_log_experiment_scales,
            global_scale_index_by_coefficient,
            experiment_scale_index_by_observation,
            fixed_coefficient_scale,
            global_scale_prior,
            chain_config,
        )?);
    }
    Ok(chains)
}

#[allow(clippy::too_many_arguments)]
fn transition_state(
    model: &PreparedGroupedRawScaleHorseshoeModel,
    current_state: &GroupedHmcState,
    current_target: &GroupedRawScaleHorseshoePosterior,
    coefficient_groups: &[usize],
    experiment_groups: &[usize],
    fixed_coefficient_scale: f64,
    prior: GlobalScalePrior,
    config: HorseshoeHmcConfig,
    rng: &mut SplitMix64,
) -> (
    GroupedHmcState,
    GroupedRawScaleHorseshoePosterior,
    bool,
    bool,
    f64,
) {
    let current = current_state.flatten();
    let dimension = current.len();
    let mut position = current.clone();
    let mut momentum: Vec<f64> = (0..dimension).map(|_| rng.normal()).collect();
    let initial_hamiltonian = -current_target.log_posterior + kinetic_energy(&momentum);
    let gradient = flatten_gradient(current_target);
    for (momentum_value, gradient_value) in momentum.iter_mut().zip(gradient.iter()) {
        *momentum_value += 0.5 * config.step_size * gradient_value;
    }

    let mut proposal_target = current_target.clone();
    let coefficient_count = current_state.coefficients.len();
    let global_count = current_state.log_global_scales.len();
    let experiment_count = current_state.log_experiment_scales.len();

    for step in 0..config.leapfrog_steps {
        for (position_value, momentum_value) in position.iter_mut().zip(momentum.iter()) {
            *position_value += config.step_size * momentum_value;
        }
        let proposal_state = match GroupedHmcState::from_flat(
            &position,
            coefficient_count,
            global_count,
            experiment_count,
        ) {
            Ok(value) => value,
            Err(_) => {
                return (
                    current_state.clone(),
                    current_target.clone(),
                    false,
                    true,
                    initial_hamiltonian,
                );
            }
        };
        proposal_target = match evaluate_state(
            model,
            &proposal_state,
            coefficient_groups,
            experiment_groups,
            fixed_coefficient_scale,
            prior,
        ) {
            Ok(value) => value,
            Err(_) => {
                return (
                    current_state.clone(),
                    current_target.clone(),
                    false,
                    true,
                    initial_hamiltonian,
                );
            }
        };
        let gradient = flatten_gradient(&proposal_target);
        let momentum_fraction = if step + 1 == config.leapfrog_steps {
            0.5
        } else {
            1.0
        };
        for (momentum_value, gradient_value) in momentum.iter_mut().zip(gradient.iter()) {
            *momentum_value += momentum_fraction * config.step_size * gradient_value;
        }
    }

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
        let state = match GroupedHmcState::from_flat(
            &position,
            coefficient_count,
            global_count,
            experiment_count,
        ) {
            Ok(value) => value,
            Err(_) => {
                return (
                    current_state.clone(),
                    current_target.clone(),
                    false,
                    true,
                    initial_hamiltonian,
                );
            }
        };
        (state, proposal_target, true, false, initial_hamiltonian)
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
    model: &PreparedGroupedRawScaleHorseshoeModel,
    state: &GroupedHmcState,
    coefficient_groups: &[usize],
    experiment_groups: &[usize],
    fixed_coefficient_scale: f64,
    prior: GlobalScalePrior,
) -> Result<GroupedRawScaleHorseshoePosterior, ModelError> {
    model.evaluate(PreparedGroupedHorseshoeEvaluation {
        coefficients: &state.coefficients,
        log_local_scales: &state.log_local_scales,
        log_global_scales: &state.log_global_scales,
        global_scale_index_by_coefficient: coefficient_groups,
        log_experiment_scales: &state.log_experiment_scales,
        experiment_scale_index_by_observation: experiment_groups,
        fixed_coefficient_scale,
        global_scale_prior: prior,
    })
}

fn flatten_gradient(target: &GroupedRawScaleHorseshoePosterior) -> Vec<f64> {
    let mut gradient = Vec::with_capacity(
        target.coefficient_gradient.len()
            + target.log_local_scale_gradient.len()
            + target.log_global_scale_gradients.len()
            + target.log_experiment_scale_gradients.len(),
    );
    gradient.extend_from_slice(&target.coefficient_gradient);
    gradient.extend_from_slice(&target.log_local_scale_gradient);
    gradient.extend_from_slice(&target.log_global_scale_gradients);
    gradient.extend_from_slice(&target.log_experiment_scale_gradients);
    gradient
}

fn kinetic_energy(momentum: &[f64]) -> f64 {
    0.5 * momentum.iter().map(|value| value * value).sum::<f64>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (
        PreparedGroupedRawScaleHorseshoeModel,
        Vec<usize>,
        Vec<usize>,
    ) {
        let model = PreparedGroupedRawScaleHorseshoeModel::new(
            &[12.0, 18.0],
            &[10.0, 20.0],
            &[vec![1.0, 0.3, -0.2], vec![0.2, -0.5, 0.4]],
            &[vec![0.04, 0.005], vec![0.005, 0.09]],
        )
        .expect("prepared model");
        (model, vec![0, 1, 2], vec![0, 0])
    }

    #[test]
    fn grouped_hmc_is_deterministic_for_a_fixed_seed() {
        let (model, coefficient_groups, experiment_groups) = fixture();
        let config = HorseshoeHmcConfig {
            discarded_transitions: 2,
            retained_draws: 4,
            step_size: 1.0e-5,
            leapfrog_steps: 2,
            divergence_energy_error_threshold: 1000.0,
            seed: 12345,
        };
        let first = run_grouped_horseshoe_hmc_chain(
            &model,
            &[0.001, -0.002, 0.003],
            &[0.8_f64.ln(), 1.1_f64.ln(), 0.9_f64.ln()],
            &[0.6_f64.ln(), 1.0_f64.ln(), 0.8_f64.ln()],
            &[1.02_f64.ln()],
            &coefficient_groups,
            &experiment_groups,
            1.0e-2,
            GlobalScalePrior::HalfNormal { scale: 1.0 },
            config,
        )
        .expect("first chain");
        let second = run_grouped_horseshoe_hmc_chain(
            &model,
            &[0.001, -0.002, 0.003],
            &[0.8_f64.ln(), 1.1_f64.ln(), 0.9_f64.ln()],
            &[0.6_f64.ln(), 1.0_f64.ln(), 0.8_f64.ln()],
            &[1.02_f64.ln()],
            &coefficient_groups,
            &experiment_groups,
            1.0e-2,
            GlobalScalePrior::HalfNormal { scale: 1.0 },
            config,
        )
        .expect("second chain");
        assert_eq!(first, second);
        assert_eq!(first.seed, 12345);
        assert_eq!(first.draws.len(), 4);
        assert!(first.draws.iter().all(|draw| {
            draw.coefficients.len() == 3
                && draw.log_local_scales.len() == 3
                && draw.log_global_scales.len() == 3
                && draw.log_experiment_scales.len() == 1
                && draw.log_posterior.is_finite()
                && draw.hamiltonian_energy.is_finite()
        }));
    }

    fn initialization(seed: u64, offset: f64) -> GroupedHmcInitialization {
        GroupedHmcInitialization {
            seed,
            initial_coefficients: vec![0.001 + offset, -0.002 - offset, 0.003 + 0.5 * offset],
            initial_log_local_scales: vec![0.8_f64.ln(), 1.1_f64.ln(), 0.9_f64.ln()],
            initial_log_global_scales: vec![0.6_f64.ln(), 1.0_f64.ln(), 0.8_f64.ln()],
            initial_log_experiment_scales: vec![1.02_f64.ln()],
        }
    }

    #[test]
    fn grouped_multi_chain_runner_requires_four_unique_seeds() {
        let (model, coefficient_groups, experiment_groups) = fixture();
        let config = HorseshoeHmcConfig {
            discarded_transitions: 1,
            retained_draws: 3,
            step_size: 1.0e-5,
            leapfrog_steps: 2,
            divergence_energy_error_threshold: 1000.0,
            seed: 0,
        };
        let initializations = vec![
            initialization(101, -0.0003),
            initialization(102, -0.0001),
            initialization(103, 0.0001),
            initialization(104, 0.0003),
        ];
        let chains = run_grouped_horseshoe_hmc_chains(
            &model,
            &initializations,
            &coefficient_groups,
            &experiment_groups,
            1.0e-2,
            GlobalScalePrior::HalfNormal { scale: 1.0 },
            config,
        )
        .expect("four independent chains");
        assert_eq!(chains.len(), 4);
        assert_eq!(
            chains.iter().map(|chain| chain.seed).collect::<Vec<_>>(),
            vec![101, 102, 103, 104]
        );
        assert!(chains.iter().all(|chain| {
            chain.draws.len() == 3
                && chain.transitions == 4
                && chain
                    .draws
                    .iter()
                    .all(|draw| draw.log_posterior.is_finite())
        }));
    }

    #[test]
    fn grouped_multi_chain_runner_rejects_too_few_chains_and_duplicate_seeds() {
        let (model, coefficient_groups, experiment_groups) = fixture();
        let config = HorseshoeHmcConfig {
            discarded_transitions: 0,
            retained_draws: 1,
            step_size: 1.0e-5,
            leapfrog_steps: 1,
            divergence_energy_error_threshold: 1000.0,
            seed: 0,
        };
        let three = vec![
            initialization(201, -0.0002),
            initialization(202, 0.0),
            initialization(203, 0.0002),
        ];
        assert_eq!(
            run_grouped_horseshoe_hmc_chains(
                &model,
                &three,
                &coefficient_groups,
                &experiment_groups,
                1.0e-2,
                GlobalScalePrior::HalfNormal { scale: 1.0 },
                config
            ),
            Err(ModelError::InvalidParameter(
                "grouped_hmc_requires_at_least_four_chains"
            ))
        );

        let duplicate = vec![
            initialization(301, -0.0003),
            initialization(302, -0.0001),
            initialization(302, 0.0001),
            initialization(304, 0.0003),
        ];
        assert_eq!(
            run_grouped_horseshoe_hmc_chains(
                &model,
                &duplicate,
                &coefficient_groups,
                &experiment_groups,
                1.0e-2,
                GlobalScalePrior::HalfNormal { scale: 1.0 },
                config
            ),
            Err(ModelError::InvalidParameter(
                "grouped_hmc_chain_seeds_must_be_unique"
            ))
        );
    }

    #[test]
    fn grouped_hmc_rejects_empty_global_scale_group() {
        let (model, _, _) = fixture();
        let result = run_grouped_horseshoe_hmc_chain(
            &model,
            &[0.001, -0.002, 0.003],
            &[0.8_f64.ln(), 1.1_f64.ln(), 0.9_f64.ln()],
            &[0.6_f64.ln(), 1.0_f64.ln()],
            &[],
            &[0, 0, 0],
            &[],
            1.0e-2,
            GlobalScalePrior::HalfNormal { scale: 1.0 },
            HorseshoeHmcConfig {
                discarded_transitions: 0,
                retained_draws: 1,
                step_size: 1.0e-5,
                leapfrog_steps: 1,
                divergence_energy_error_threshold: 1000.0,
                seed: 7,
            },
        );
        assert!(matches!(
            result,
            Err(ModelError::InvalidParameter(
                "prepared_grouped_global_scale_group_empty"
            ))
        ));
    }
}
