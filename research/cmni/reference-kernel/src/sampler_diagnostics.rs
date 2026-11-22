//! Modern multi-chain diagnostics for the research HMC comparator.
//!
//! Implements rank-normalized split-Rhat, folded rank-normalized split-Rhat,
//! a Geyer initial-positive-sequence ESS approximation, tail indicator ESS,
//! MCSE for the mean, and E-BFMI from retained Hamiltonian energies.
//!
//! This is a research implementation requiring independent validation against
//! the Stan posterior package / reference diagnostics before scientific use.
//! It is deliberately kept separate from the sampler.

use crate::{GroupedHmcChain, HorseshoeHmcChain, HorseshoeHmcDraw, ModelError};

#[derive(Debug, Clone, PartialEq)]
pub struct HmcDiagnosticSummary {
    pub chain_count: usize,
    pub draws_per_chain: usize,
    pub rank_normalized_split_rhat: f64,
    pub rank_normalized_folded_split_rhat: f64,
    pub maximum_rhat: f64,
    pub bulk_ess: f64,
    pub tail_ess: f64,
    /// Effective sample size for the raw coefficient mean.
    pub mean_ess: f64,
    /// MCSE of the raw coefficient mean, expressed in coefficient units.
    pub mcse_mean: f64,
    pub energy_bfmi_by_chain: Vec<f64>,
    pub minimum_energy_bfmi: f64,
    pub divergence_count: usize,
    pub meets_proposed_thresholds: bool,
}

/// Select a scalar quantity from each grouped HMC draw for convergence diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupedHmcQuantity {
    Coefficient(usize),
    /// Positive local scale (exp of the sampled log coordinate).
    LocalScale(usize),
    /// Raw sampled log-local-scale coordinate.
    LogLocalScale(usize),
    /// Positive group-global scale (exp of the sampled log coordinate).
    GlobalScale(usize),
    /// Raw sampled log-global-scale coordinate.
    LogGlobalScale(usize),
    /// Positive experiment normalization scale (exp of the sampled log coordinate).
    ExperimentScale(usize),
    /// Raw sampled log-experiment-scale coordinate.
    LogExperimentScale(usize),
}

/// Apply the same rank-normalized diagnostics to a named grouped-HMC quantity.
///
/// The implementation projects the selected scalar trace into the existing
/// independently tested single-parameter diagnostic path. It preserves the
/// grouped chain seeds, retained energies, divergence counts and exact draw
/// counts; no samples are resampled or truncated.
pub fn diagnose_grouped_hmc_quantity(
    chains: &[GroupedHmcChain],
    quantity: GroupedHmcQuantity,
) -> Result<HmcDiagnosticSummary, ModelError> {
    if chains.len() < 4 {
        return Err(ModelError::InvalidParameter(
            "sampler_diagnostics_require_at_least_four_chains",
        ));
    }
    let expected_draws = chains.first().map_or(0, |chain| chain.draws.len());
    if expected_draws < 8
        || chains
            .iter()
            .any(|chain| chain.draws.len() != expected_draws)
    {
        return Err(ModelError::InvalidParameter(
            "grouped_sampler_diagnostics_require_equal_chains_with_at_least_eight_draws",
        ));
    }

    let mut projected = Vec::with_capacity(chains.len());
    for chain in chains {
        let mut draws = Vec::with_capacity(chain.draws.len());
        for draw in &chain.draws {
            let value = match quantity {
                GroupedHmcQuantity::Coefficient(index) => draw.coefficients.get(index).copied(),
                GroupedHmcQuantity::LocalScale(index) => {
                    draw.log_local_scales.get(index).map(|value| value.exp())
                }
                GroupedHmcQuantity::LogLocalScale(index) => {
                    draw.log_local_scales.get(index).copied()
                }
                GroupedHmcQuantity::GlobalScale(index) => {
                    draw.log_global_scales.get(index).map(|value| value.exp())
                }
                GroupedHmcQuantity::LogGlobalScale(index) => {
                    draw.log_global_scales.get(index).copied()
                }
                GroupedHmcQuantity::ExperimentScale(index) => draw
                    .log_experiment_scales
                    .get(index)
                    .map(|value| value.exp()),
                GroupedHmcQuantity::LogExperimentScale(index) => {
                    draw.log_experiment_scales.get(index).copied()
                }
            }
            .ok_or(ModelError::MismatchedDataLengths)?;
            if !value.is_finite() || !draw.hamiltonian_energy.is_finite() {
                return Err(ModelError::NonFiniteValue(
                    "grouped_sampler_diagnostic_trace",
                ));
            }
            draws.push(HorseshoeHmcDraw {
                coefficients: vec![value],
                log_local_scales: Vec::new(),
                log_global_scale: 0.0,
                log_posterior: draw.log_posterior,
                hamiltonian_energy: draw.hamiltonian_energy,
            });
        }
        projected.push(HorseshoeHmcChain {
            seed: chain.seed,
            draws,
            transitions: chain.transitions,
            accepted_transitions: chain.accepted_transitions,
            divergences: chain.divergences,
            acceptance_rate: chain.acceptance_rate,
        });
    }
    diagnose_horseshoe_coefficient(&projected, 0)
}

/// Diagnose one coefficient across independently run HMC chains.
///
/// Requires at least four chains, all with at least eight retained draws.
/// The proposed qualification boolean uses Rhat < 1.01, bulk / tail ESS >=
/// 100 per chain, zero divergences and minimum E-BFMI >= 0.3. These are
/// conservative project gates based on published sampler guidance, not a
/// substitute for trace inspection or posterior predictive checks.
pub fn diagnose_horseshoe_coefficient(
    chains: &[HorseshoeHmcChain],
    coefficient_index: usize,
) -> Result<HmcDiagnosticSummary, ModelError> {
    if chains.len() < 4 {
        return Err(ModelError::InvalidParameter(
            "sampler_diagnostics_require_at_least_four_chains",
        ));
    }
    let minimum_draws = chains
        .iter()
        .map(|chain| chain.draws.len())
        .min()
        .unwrap_or(0);
    if minimum_draws < 8 {
        return Err(ModelError::InvalidParameter(
            "sampler_diagnostics_require_at_least_eight_draws_per_chain",
        ));
    }
    let mut seeds = Vec::with_capacity(chains.len());
    for chain in chains {
        if seeds.contains(&chain.seed) {
            return Err(ModelError::InvalidParameter(
                "sampler_diagnostics_require_unique_chain_seeds",
            ));
        }
        seeds.push(chain.seed);
        if chain.draws.len() != minimum_draws {
            return Err(ModelError::MismatchedDataLengths);
        }
        if chain.draws.iter().any(|draw| {
            coefficient_index >= draw.coefficients.len() || !draw.hamiltonian_energy.is_finite()
        }) {
            return Err(ModelError::MismatchedDataLengths);
        }
    }

    let raw_chains: Vec<Vec<f64>> = chains
        .iter()
        .map(|chain| {
            chain
                .draws
                .iter()
                .map(|draw| draw.coefficients[coefficient_index])
                .collect()
        })
        .collect();
    let split_raw = split_chains(&raw_chains)?;
    let rank_normalized = rank_normalize_chains(&split_raw)?;
    let rank_normalized_split_rhat = split_rhat(&rank_normalized)?;
    let pooled_raw: Vec<f64> = split_raw.iter().flatten().copied().collect();
    let median = quantile(&pooled_raw, 0.5)?;
    let folded: Vec<Vec<f64>> = split_raw
        .iter()
        .map(|chain| chain.iter().map(|value| (value - median).abs()).collect())
        .collect();
    let folded_rank_normalized = rank_normalize_chains(&folded)?;
    let rank_normalized_folded_split_rhat = split_rhat(&folded_rank_normalized)?;
    let maximum_rhat = rank_normalized_split_rhat.max(rank_normalized_folded_split_rhat);

    let bulk_ess = effective_sample_size(&rank_normalized)?;
    let lower_threshold = quantile(&pooled_raw, 0.05)?;
    let upper_threshold = quantile(&pooled_raw, 0.95)?;
    let lower_indicator: Vec<Vec<f64>> = split_raw
        .iter()
        .map(|chain| {
            chain
                .iter()
                .map(|value| if *value <= lower_threshold { 1.0 } else { 0.0 })
                .collect()
        })
        .collect();
    let upper_indicator: Vec<Vec<f64>> = split_raw
        .iter()
        .map(|chain| {
            chain
                .iter()
                .map(|value| if *value >= upper_threshold { 1.0 } else { 0.0 })
                .collect()
        })
        .collect();
    let tail_ess =
        effective_sample_size(&lower_indicator)?.min(effective_sample_size(&upper_indicator)?);

    // Mean MCSE must remain in the original coefficient units. Bulk ESS
    // is based on rank-normalized draws and is intentionally reported apart
    // from the raw-scale mean ESS used for this standard error.
    let mean_ess = effective_sample_size(&split_raw)?;
    let raw_variance_plus = variance_plus(&split_raw)?;
    let mcse_mean = if mean_ess > 0.0 && raw_variance_plus >= 0.0 {
        (raw_variance_plus / mean_ess).sqrt()
    } else {
        f64::INFINITY
    };
    let energy_bfmi_by_chain: Vec<f64> = chains
        .iter()
        .map(|chain| {
            let energies: Vec<f64> = chain
                .draws
                .iter()
                .map(|draw| draw.hamiltonian_energy)
                .collect();
            energy_bfmi(&energies)
        })
        .collect();
    let minimum_energy_bfmi = energy_bfmi_by_chain
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let divergence_count = chains.iter().map(|chain| chain.divergences).sum();
    let required_ess = 100.0 * chains.len() as f64;
    let meets_proposed_thresholds = maximum_rhat < 1.01
        && bulk_ess >= required_ess
        && tail_ess >= required_ess
        && divergence_count == 0
        && minimum_energy_bfmi >= 0.3;

    Ok(HmcDiagnosticSummary {
        chain_count: chains.len(),
        draws_per_chain: minimum_draws,
        rank_normalized_split_rhat,
        rank_normalized_folded_split_rhat,
        maximum_rhat,
        bulk_ess,
        tail_ess,
        mean_ess,
        mcse_mean,
        energy_bfmi_by_chain,
        minimum_energy_bfmi,
        divergence_count,
        meets_proposed_thresholds,
    })
}

fn split_chains(chains: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    if chains.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let minimum = chains.iter().map(Vec::len).min().unwrap_or(0);
    let half = minimum / 2;
    if half < 2 {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut split = Vec::with_capacity(chains.len() * 2);
    for chain in chains {
        if chain.iter().any(|value| !value.is_finite()) {
            return Err(ModelError::NonFiniteValue("sampler_diagnostic_draws"));
        }
        split.push(chain[..half].to_vec());
        split.push(chain[chain.len() - half..].to_vec());
    }
    Ok(split)
}

fn rank_normalize_chains(chains: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    if chains.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let total: usize = chains.iter().map(Vec::len).sum();
    if total < 4 || chains.iter().any(|chain| chain.is_empty()) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut ordered: Vec<(f64, usize, usize)> = Vec::with_capacity(total);
    for (chain_index, chain) in chains.iter().enumerate() {
        for (draw_index, value) in chain.iter().enumerate() {
            if !value.is_finite() {
                return Err(ModelError::NonFiniteValue("sampler_diagnostic_draws"));
            }
            ordered.push((*value, chain_index, draw_index));
        }
    }
    ordered.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut normalized: Vec<Vec<f64>> = chains.iter().map(|chain| vec![0.0; chain.len()]).collect();

    let mut start = 0usize;
    while start < ordered.len() {
        let mut end = start + 1;
        while end < ordered.len() && ordered[end].0 == ordered[start].0 {
            end += 1;
        }
        let average_rank = ((start + 1) as f64 + end as f64) / 2.0;
        let probability = ((average_rank - 0.375) / (total as f64 + 0.25))
            .clamp(f64::MIN_POSITIVE, 1.0 - f64::EPSILON);
        let normal_score = inverse_standard_normal_cdf(probability);
        for item in &ordered[start..end] {
            normalized[item.1][item.2] = normal_score;
        }
        start = end;
    }
    Ok(normalized)
}

fn inverse_standard_normal_cdf(probability: f64) -> f64 {
    // Acklam rational approximation.
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_690e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    const LOW: f64 = 0.024_25;
    const HIGH: f64 = 1.0 - LOW;
    if probability < LOW {
        let q = (-2.0 * probability.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if probability <= HIGH {
        let q = probability - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - probability).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    }
}

fn split_rhat(chains: &[Vec<f64>]) -> Result<f64, ModelError> {
    if chains.len() < 2 {
        return Err(ModelError::MismatchedDataLengths);
    }
    let count = chains[0].len();
    if count < 2 || chains.iter().any(|chain| chain.len() != count) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let means: Vec<f64> = chains
        .iter()
        .map(|chain| chain.iter().sum::<f64>() / count as f64)
        .collect();
    let variances: Vec<f64> = chains
        .iter()
        .zip(means.iter())
        .map(|(chain, mean)| {
            chain
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / (count - 1) as f64
        })
        .collect();
    let within = variances.iter().sum::<f64>() / chains.len() as f64;
    let overall_mean = means.iter().sum::<f64>() / means.len() as f64;
    let between = count as f64
        * means
            .iter()
            .map(|mean| (mean - overall_mean).powi(2))
            .sum::<f64>()
        / (means.len() - 1) as f64;
    if within == 0.0 {
        return if between == 0.0 {
            Ok(1.0)
        } else {
            Ok(f64::INFINITY)
        };
    }
    let variance_plus = (count as f64 - 1.0) / count as f64 * within + between / count as f64;
    Ok((variance_plus / within).sqrt())
}

fn variance_plus(chains: &[Vec<f64>]) -> Result<f64, ModelError> {
    if chains.len() < 2 {
        return Err(ModelError::MismatchedDataLengths);
    }
    let count = chains[0].len();
    if count < 2 || chains.iter().any(|chain| chain.len() != count) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let means: Vec<f64> = chains
        .iter()
        .map(|chain| chain.iter().sum::<f64>() / count as f64)
        .collect();
    let variances: Vec<f64> = chains
        .iter()
        .zip(means.iter())
        .map(|(chain, mean)| {
            chain
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / (count - 1) as f64
        })
        .collect();
    let within = variances.iter().sum::<f64>() / chains.len() as f64;
    let grand_mean = means.iter().sum::<f64>() / means.len() as f64;
    let between = count as f64
        * means
            .iter()
            .map(|mean| (mean - grand_mean).powi(2))
            .sum::<f64>()
        / (means.len() - 1) as f64;
    Ok((count as f64 - 1.0) / count as f64 * within + between / count as f64)
}

fn effective_sample_size(chains: &[Vec<f64>]) -> Result<f64, ModelError> {
    if chains.len() < 2 {
        return Err(ModelError::MismatchedDataLengths);
    }
    let count = chains[0].len();
    if count < 3 || chains.iter().any(|chain| chain.len() != count) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let means: Vec<f64> = chains
        .iter()
        .map(|chain| chain.iter().sum::<f64>() / count as f64)
        .collect();
    let variances: Vec<f64> = chains
        .iter()
        .zip(means.iter())
        .map(|(chain, mean)| {
            chain
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / (count - 1) as f64
        })
        .collect();
    let within = variances.iter().sum::<f64>() / chains.len() as f64;
    let plus = variance_plus(chains)?;
    if within <= 0.0 || plus <= 0.0 {
        return Ok(0.0);
    }

    let mut rho = Vec::with_capacity(count);
    rho.push(1.0);
    for lag in 1..count {
        let mut autocovariance = 0.0;
        for (chain, mean) in chains.iter().zip(means.iter()) {
            let sum = (0..(count - lag))
                .map(|index| (chain[index] - mean) * (chain[index + lag] - mean))
                .sum::<f64>();
            autocovariance += sum / (count - lag) as f64;
        }
        autocovariance /= chains.len() as f64;
        rho.push(1.0 - (within - autocovariance) / plus);
    }

    let mut positive_sequence_sum = 0.0;
    let mut previous_pair = f64::INFINITY;
    let mut index = 0usize;
    while index + 1 < rho.len() {
        let pair = rho[index] + rho[index + 1];
        if pair <= 0.0 {
            break;
        }
        let monotone_pair = pair.min(previous_pair);
        positive_sequence_sum += monotone_pair;
        previous_pair = monotone_pair;
        index += 2;
    }
    let integrated_time = (-1.0 + 2.0 * positive_sequence_sum).max(1.0e-9);
    Ok((chains.len() * count) as f64 / integrated_time)
}

fn quantile(values: &[f64], probability: f64) -> Result<f64, ModelError> {
    if values.is_empty() || !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let position = probability * (ordered.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        Ok(ordered[lower])
    } else {
        let fraction = position - lower as f64;
        Ok(ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction)
    }
}

fn energy_bfmi(energies: &[f64]) -> f64 {
    if energies.len() < 2 || energies.iter().any(|value| !value.is_finite()) {
        return 0.0;
    }
    let mean = energies.iter().sum::<f64>() / energies.len() as f64;
    let variance = energies
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (energies.len() - 1) as f64;
    if variance <= 0.0 {
        return 0.0;
    }
    let mean_squared_difference = energies
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).powi(2))
        .sum::<f64>()
        / (energies.len() - 1) as f64;
    mean_squared_difference / variance
}

#[cfg(test)]
mod grouped_hmc_tests {
    use super::*;
    use crate::GroupedHmcDraw;

    fn synthetic_grouped_chains() -> Vec<GroupedHmcChain> {
        (0..4)
            .map(|chain_index| {
                let draws = (0..32)
                    .map(|draw_index| {
                        let phase = draw_index as f64 * 0.11 + chain_index as f64 * 0.07;
                        GroupedHmcDraw {
                            coefficients: vec![phase.sin(), phase.cos()],
                            log_local_scales: vec![0.1 + phase.sin() * 0.01],
                            log_global_scales: vec![-1.0 + phase.cos() * 0.02, -0.5],
                            log_experiment_scales: vec![0.0 + phase.sin() * 0.001],
                            log_posterior: -0.5 * phase * phase,
                            hamiltonian_energy: 1.0 + phase.cos() * 0.1,
                        }
                    })
                    .collect();
                GroupedHmcChain {
                    seed: 1000 + chain_index as u64,
                    draws,
                    transitions: 32,
                    accepted_transitions: 30,
                    divergences: 0,
                    acceptance_rate: 30.0 / 32.0,
                }
            })
            .collect()
    }

    #[test]
    fn grouped_hmc_diagnostics_support_all_parameter_families() {
        let chains = synthetic_grouped_chains();
        for quantity in [
            GroupedHmcQuantity::Coefficient(0),
            GroupedHmcQuantity::LocalScale(0),
            GroupedHmcQuantity::LogLocalScale(0),
            GroupedHmcQuantity::GlobalScale(1),
            GroupedHmcQuantity::LogGlobalScale(1),
            GroupedHmcQuantity::ExperimentScale(0),
            GroupedHmcQuantity::LogExperimentScale(0),
        ] {
            let summary =
                diagnose_grouped_hmc_quantity(&chains, quantity).expect("grouped diagnostics");
            assert_eq!(summary.chain_count, 4);
            assert_eq!(summary.draws_per_chain, 32);
            assert_eq!(summary.divergence_count, 0);
            assert!(summary.maximum_rhat.is_finite());
        }
    }

    #[test]
    fn grouped_hmc_diagnostics_reject_missing_parameter_indices() {
        let chains = synthetic_grouped_chains();
        assert_eq!(
            diagnose_grouped_hmc_quantity(&chains, GroupedHmcQuantity::GlobalScale(9)),
            Err(ModelError::MismatchedDataLengths)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_chain(seed: u64, values: &[f64], energy_values: &[f64]) -> HorseshoeHmcChain {
        let draws = values
            .iter()
            .zip(energy_values.iter())
            .map(|(value, energy)| crate::HorseshoeHmcDraw {
                coefficients: vec![*value],
                log_local_scales: vec![0.0],
                log_global_scale: -1.0,
                log_posterior: -*value * *value,
                hamiltonian_energy: *energy,
            })
            .collect();
        HorseshoeHmcChain {
            seed,
            draws,
            transitions: values.len(),
            accepted_transitions: values.len(),
            divergences: 0,
            acceptance_rate: 1.0,
        }
    }

    fn deterministic_normal_samples(seed: u64, count: usize) -> Vec<f64> {
        let mut state = seed;
        let mut next_uniform = || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut value = state;
            value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            value ^= value >> 31;
            let mantissa = value >> 11;
            (mantissa as f64 + 0.5) / 9_007_199_254_740_992.0
        };
        let mut values = Vec::with_capacity(count);
        while values.len() < count {
            let first = next_uniform();
            let second = next_uniform();
            let radius = (-2.0 * first.ln()).sqrt();
            let angle = std::f64::consts::TAU * second;
            values.push(radius * angle.cos());
            if values.len() < count {
                values.push(radius * angle.sin());
            }
        }
        values
    }

    fn iid_like_chain(seed: u64, count: usize) -> HorseshoeHmcChain {
        let values = deterministic_normal_samples(seed, count);
        let energies = deterministic_normal_samples(seed.wrapping_add(17), count);
        let draws = values
            .iter()
            .zip(energies.iter())
            .map(|(value, energy)| crate::HorseshoeHmcDraw {
                coefficients: vec![*value],
                log_local_scales: vec![0.0],
                log_global_scale: -1.0,
                log_posterior: -0.5 * value * value,
                hamiltonian_energy: *energy,
            })
            .collect();
        HorseshoeHmcChain {
            seed,
            draws,
            transitions: count,
            accepted_transitions: count,
            divergences: 0,
            acceptance_rate: 1.0,
        }
    }

    #[test]
    fn independent_normal_control_meets_proposed_diagnostic_thresholds() {
        let chains: Vec<HorseshoeHmcChain> = (0..4)
            .map(|index| iid_like_chain(0xC0FFEE_u64 + index * 97, 2_000))
            .collect();
        let summary =
            diagnose_horseshoe_coefficient(&chains, 0).expect("IID-like diagnostic control");
        assert!(summary.maximum_rhat < 1.01, "Rhat={}", summary.maximum_rhat);
        assert!(summary.bulk_ess >= 400.0, "bulk ESS={}", summary.bulk_ess);
        assert!(summary.tail_ess >= 400.0, "tail ESS={}", summary.tail_ess);
        assert_eq!(summary.divergence_count, 0);
        assert!(summary.minimum_energy_bfmi >= 0.3);
        assert!(summary.meets_proposed_thresholds);
    }

    #[test]
    fn modern_diagnostics_require_at_least_four_chains() {
        let values: Vec<f64> = (0..20).map(|index| (index as f64).sin()).collect();
        let energies = vec![1.0; 20];
        let chains = vec![
            synthetic_chain(1, &values, &energies),
            synthetic_chain(2, &values, &energies),
        ];
        assert_eq!(
            diagnose_horseshoe_coefficient(&chains, 0),
            Err(ModelError::InvalidParameter(
                "sampler_diagnostics_require_at_least_four_chains"
            ))
        );
    }

    #[test]
    fn modern_diagnostics_reject_duplicate_chain_seeds() {
        let values: Vec<f64> = (0..20).map(|index| (index as f64).sin()).collect();
        let energies: Vec<f64> = (0..20).map(|index| (index as f64).cos()).collect();
        let chains = vec![
            synthetic_chain(7, &values, &energies),
            synthetic_chain(7, &values, &energies),
            synthetic_chain(8, &values, &energies),
            synthetic_chain(9, &values, &energies),
        ];
        assert_eq!(
            diagnose_horseshoe_coefficient(&chains, 0),
            Err(ModelError::InvalidParameter(
                "sampler_diagnostics_require_unique_chain_seeds"
            ))
        );
    }

    #[test]
    fn modern_diagnostics_return_rank_rhat_ess_and_energy_statistics() {
        let mut chains = Vec::new();
        for chain_index in 0..4 {
            let values: Vec<f64> = (0..80)
                .map(|index| ((index * 17 + chain_index * 13) as f64 * 0.37).sin())
                .collect();
            let energies: Vec<f64> = (0..80)
                .map(|index| 2.0 + ((index * 11 + chain_index) as f64 * 0.19).cos())
                .collect();
            chains.push(synthetic_chain(
                1000 + chain_index as u64,
                &values,
                &energies,
            ));
        }
        let summary = diagnose_horseshoe_coefficient(&chains, 0).expect("diagnostics");
        assert!(summary.maximum_rhat.is_finite());
        assert!(summary.bulk_ess > 0.0);
        assert!(summary.tail_ess > 0.0);
        assert!(summary.mcse_mean.is_finite());
        assert!(summary.minimum_energy_bfmi >= 0.0);
    }

    #[test]
    fn folded_rank_rhat_detects_cross_chain_scale_mismatch() {
        let mut chains = Vec::new();
        for (chain_index, scale) in [1.0, 1.0, 5.0, 5.0].iter().enumerate() {
            let values: Vec<f64> = (0..80)
                .map(|index| if index % 2 == 0 { *scale } else { -*scale })
                .collect();
            let energies: Vec<f64> = (0..80).map(|index| index as f64).collect();
            chains.push(synthetic_chain(
                3000 + chain_index as u64,
                &values,
                &energies,
            ));
            let _ = chain_index;
        }
        let summary = diagnose_horseshoe_coefficient(&chains, 0).expect("diagnostics");
        assert!(summary.rank_normalized_folded_split_rhat > 1.01);
        assert!(summary.maximum_rhat >= summary.rank_normalized_split_rhat);
    }

    #[test]
    fn zero_energy_variation_has_zero_ebfmi_and_fails_qualification() {
        let mut chains = Vec::new();
        for chain_index in 0..4 {
            let values: Vec<f64> = (0..80)
                .map(|index| ((index * 13 + chain_index * 7) as f64).sin())
                .collect();
            chains.push(synthetic_chain(
                2000 + chain_index as u64,
                &values,
                &vec![3.0; 80],
            ));
        }
        let summary = diagnose_horseshoe_coefficient(&chains, 0).expect("diagnostics");
        assert_eq!(summary.minimum_energy_bfmi, 0.0);
        assert!(!summary.meets_proposed_thresholds);
    }

    #[test]
    fn raw_mean_mcse_scales_with_original_coefficient_units() {
        let mut chains = Vec::new();
        for chain_index in 0..4 {
            let values: Vec<f64> = (0..80)
                .map(|index| ((index * 17 + chain_index * 13) as f64 * 0.37).sin())
                .collect();
            let energies: Vec<f64> = (0..80)
                .map(|index| 2.0 + ((index * 11 + chain_index) as f64 * 0.19).cos())
                .collect();
            chains.push(synthetic_chain(
                4000 + chain_index as u64,
                &values,
                &energies,
            ));
        }
        let base = diagnose_horseshoe_coefficient(&chains, 0).expect("base diagnostics");
        let scaled_chains: Vec<HorseshoeHmcChain> = chains
            .iter()
            .map(|chain| {
                let mut result = chain.clone();
                for draw in &mut result.draws {
                    draw.coefficients[0] *= 10.0;
                }
                result
            })
            .collect();
        let scaled = diagnose_horseshoe_coefficient(&scaled_chains, 0).expect("scaled diagnostics");
        assert!((scaled.mcse_mean / base.mcse_mean - 10.0).abs() < 1.0e-8);
        assert!((scaled.mean_ess - base.mean_ess).abs() < 1.0e-8);
    }

    #[test]
    fn raw_mean_mcse_is_infinite_when_raw_mean_ess_is_zero() {
        let chains = (0..4)
            .map(|chain_index| {
                let values = vec![chain_index as f64; 8];
                let energies: Vec<f64> = (0..8).map(|index| index as f64).collect();
                synthetic_chain(5000 + chain_index as u64, &values, &energies)
            })
            .collect::<Vec<_>>();
        let summary = diagnose_horseshoe_coefficient(&chains, 0).expect("diagnostics");
        assert_eq!(summary.mean_ess, 0.0);
        assert!(summary.mcse_mean.is_infinite());
    }
}
