//! Paired spatial-excitation design comparison for coupling-operator identification.
//!
//! The same synthetic local operator and held-out spatial patterns are used
//! across three training designs:
//! - isolated one-taxel-at-a-time impulses;
//! - randomized dense nonnegative multi-point patterns;
//! - repeated common-mode patterns that are intentionally rank deficient.
//!
//! Reports distinguish physical trial count (pattern count) from the number of
//! active taxel-site excitations. Neither is asserted to equal laboratory
//! reference burden: that requires a fixture-specific cost model.

#![forbid(unsafe_code)]

use super::coupling_identification::{
    CouplingIdentificationConfig, CouplingIdentificationProblem, CouplingRowDisposition,
    identify_coupling_operator,
};

const TAXEL_COUNT: usize = 7;
const MODULE_IDS: [u32; TAXEL_COUNT] = [0, 0, 0, 0, 1, 1, 1];
const SUPPORT_RADIUS: usize = 1;
const REQUIRED_REPLICATES: usize = 50;
const RANDOM_TRAINING_PATTERNS: usize = 5;
const HELDOUT_PATTERN_COUNT: usize = 7;
const RESPONSE_NOISE_SCALE: f64 = 1.0e-4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum CouplingExcitationDesign {
    IsolatedSingleTaxel,
    CyclicSparseOrthogonal,
    RandomDenseMultiPoint,
    RepeatedCommonMode,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CouplingDesignSummary {
    pub design: CouplingExcitationDesign,
    pub replicate_count: usize,
    pub training_pattern_count: usize,
    pub mean_active_taxel_site_excitations: f64,
    pub mean_identified_output_rows: f64,
    pub mean_underidentified_output_rows: f64,
    pub mean_coefficient_rmse: Option<f64>,
    pub mean_heldout_response_rmse: Option<f64>,
    pub maximum_heldout_response_rmse: Option<f64>,
    pub evaluated_replicates: usize,
    /// Negative means lower held-out error than isolated probes.
    pub paired_heldout_rmse_delta_mean_vs_isolated: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CouplingDesignCampaignReport {
    pub taxel_count: usize,
    pub module_count: usize,
    pub support_radius: usize,
    pub first_seed: u64,
    pub replicate_count: usize,
    pub heldout_pattern_count: usize,
    pub results: Vec<CouplingDesignSummary>,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
struct SyntheticCouplingCase {
    kernel: Vec<f64>,
    heldout_stimuli: Vec<f64>,
    heldout_responses: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ReplicateResult {
    active_site_excitations: usize,
    identified_rows: usize,
    underidentified_rows: usize,
    coefficient_rmse: Option<f64>,
    heldout_rmse: Option<f64>,
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

fn random_unit(state: &mut u64) -> f64 {
    (next_random(state) >> 11) as f64 / ((1_u64 << 53) as f64)
}

fn make_kernel(seed: u64) -> Vec<f64> {
    let mut state = seed ^ 0x8EBC_6AF0_9C88_C6E3;
    if state == 0 {
        state = 1;
    }
    let mut matrix = vec![0.0; TAXEL_COUNT * TAXEL_COUNT];
    for output in 0..TAXEL_COUNT {
        matrix[output * TAXEL_COUNT + output] = 0.68 + 0.12 * random_unit(&mut state);
        for source in 0..TAXEL_COUNT {
            if source != output
                && MODULE_IDS[source] == MODULE_IDS[output]
                && source.abs_diff(output) == 1
            {
                matrix[output * TAXEL_COUNT + source] = 0.08 + 0.05 * random_unit(&mut state);
            }
        }
    }
    matrix
}

fn random_patterns(pattern_count: usize, seed: u64) -> Vec<f64> {
    let mut state = seed.max(1);
    (0..pattern_count * TAXEL_COUNT)
        .map(|_| 0.1 + random_unit(&mut state))
        .collect()
}

fn isolated_patterns() -> Vec<f64> {
    let mut stimuli = vec![0.0; TAXEL_COUNT * TAXEL_COUNT];
    for taxel in 0..TAXEL_COUNT {
        stimuli[taxel * TAXEL_COUNT + taxel] = 1.0;
    }
    stimuli
}

fn common_mode_patterns(pattern_count: usize) -> Vec<f64> {
    vec![1.0; pattern_count * TAXEL_COUNT]
}

fn cyclic_sparse_orthogonal_patterns() -> Vec<f64> {
    // The assumed local kernel has support radius one within each module.
    // Give each taxel a nonnegative one-hot signature in a three-pattern
    // cycle, resetting the cycle at module boundaries. Every three adjacent
    // source taxels therefore have linearly independent design columns.
    let mut stimuli = vec![0.0; 3 * TAXEL_COUNT];
    let mut module_local_positions = std::collections::BTreeMap::<u32, usize>::new();
    for taxel in 0..TAXEL_COUNT {
        let position = module_local_positions.entry(MODULE_IDS[taxel]).or_default();
        let pattern = *position % 3;
        stimuli[pattern * TAXEL_COUNT + taxel] = 1.0;
        *position += 1;
    }
    stimuli
}

fn apply_kernel(
    stimuli: &[f64],
    pattern_count: usize,
    kernel: &[f64],
    noise_seed: u64,
) -> Vec<f64> {
    let mut state = noise_seed.max(1);
    let mut responses = Vec::with_capacity(pattern_count * TAXEL_COUNT);
    for pattern in 0..pattern_count {
        for output in 0..TAXEL_COUNT {
            let predicted = (0..TAXEL_COUNT)
                .map(|source| {
                    kernel[output * TAXEL_COUNT + source] * stimuli[pattern * TAXEL_COUNT + source]
                })
                .sum::<f64>();
            let noise = (random_unit(&mut state) - 0.5) * 2.0 * RESPONSE_NOISE_SCALE;
            responses.push(predicted + noise);
        }
    }
    responses
}

fn make_case(seed: u64) -> SyntheticCouplingCase {
    let kernel = make_kernel(seed);
    let heldout_stimuli = random_patterns(HELDOUT_PATTERN_COUNT, seed ^ 0xA076_1D64_78BD_642F);
    let heldout_responses = apply_kernel(
        &heldout_stimuli,
        HELDOUT_PATTERN_COUNT,
        &kernel,
        seed ^ 0xE703_7ED1_A0B4_28DB,
    );
    SyntheticCouplingCase {
        kernel,
        heldout_stimuli,
        heldout_responses,
    }
}

fn training_design(design: CouplingExcitationDesign, seed: u64) -> (Vec<f64>, usize) {
    match design {
        CouplingExcitationDesign::IsolatedSingleTaxel => {
            let stimuli = isolated_patterns();
            (stimuli, TAXEL_COUNT)
        }
        CouplingExcitationDesign::CyclicSparseOrthogonal => {
            (cyclic_sparse_orthogonal_patterns(), 3)
        }
        CouplingExcitationDesign::RandomDenseMultiPoint => {
            let stimuli = random_patterns(RANDOM_TRAINING_PATTERNS, seed ^ 0xD1B5_4A32_D192_ED03);
            (stimuli, RANDOM_TRAINING_PATTERNS)
        }
        CouplingExcitationDesign::RepeatedCommonMode => {
            let stimuli = common_mode_patterns(RANDOM_TRAINING_PATTERNS);
            (stimuli, RANDOM_TRAINING_PATTERNS)
        }
    }
}

fn active_site_excitations(stimuli: &[f64]) -> usize {
    stimuli.iter().filter(|value| **value > 0.0).count()
}

fn evaluate_design(
    case: &SyntheticCouplingCase,
    design: CouplingExcitationDesign,
    seed: u64,
) -> Result<ReplicateResult, String> {
    let (training_stimuli, training_pattern_count) = training_design(design, seed);
    let training_responses = apply_kernel(
        &training_stimuli,
        training_pattern_count,
        &case.kernel,
        seed ^ 0x5899_65CC_7537_4CC3,
    );
    let active_site_excitations = active_site_excitations(&training_stimuli);
    let problem = CouplingIdentificationProblem {
        taxel_count: TAXEL_COUNT,
        module_ids: MODULE_IDS.to_vec(),
        support_radius: SUPPORT_RADIUS,
        training_stimuli,
        training_responses,
        heldout_stimuli: case.heldout_stimuli.clone(),
        heldout_responses: case.heldout_responses.clone(),
        admitted_output_channels: vec![true; TAXEL_COUNT],
        forward_profile_digest: format!("sha256:coupling-design-{seed:016x}"),
    };
    let report = identify_coupling_operator(&problem, &CouplingIdentificationConfig::default())?;

    let mut coefficient_squared_error = 0.0;
    let mut coefficient_count = 0;
    let mut heldout_rmse = Vec::new();
    for row in &report.rows {
        if row.disposition != CouplingRowDisposition::IdentifiedFromTrainingPatterns {
            continue;
        }
        if let Some(error) = row.heldout_rmse {
            heldout_rmse.push(error);
        }
        for (source, estimate) in row
            .source_taxel_indices
            .iter()
            .zip(&row.coefficient_estimates)
        {
            if let Some(value) = estimate {
                let truth = case.kernel[row.output_taxel_index * TAXEL_COUNT + source];
                let error = value - truth;
                coefficient_squared_error += error * error;
                coefficient_count += 1;
            }
        }
    }

    let identified_rows = report.identified_output_row_count;
    let underidentified_rows = report.underidentified_output_row_count;
    let coefficient_rmse = (coefficient_count > 0)
        .then(|| (coefficient_squared_error / coefficient_count as f64).sqrt());
    let mean_heldout_rmse = (!heldout_rmse.is_empty())
        .then(|| heldout_rmse.iter().sum::<f64>() / heldout_rmse.len() as f64);

    Ok(ReplicateResult {
        active_site_excitations,
        identified_rows,
        underidentified_rows,
        coefficient_rmse,
        heldout_rmse: mean_heldout_rmse,
    })
}

fn summarize(
    design: CouplingExcitationDesign,
    results: &[ReplicateResult],
    isolated_baseline: &[ReplicateResult],
) -> Result<CouplingDesignSummary, String> {
    if results.len() < REQUIRED_REPLICATES || results.len() != isolated_baseline.len() {
        return Err("coupling design summary requires at least fifty paired replicates".to_owned());
    }
    let mean = |values: &[f64]| -> Option<f64> {
        (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
    };
    let coefficient_values: Vec<_> = results
        .iter()
        .filter_map(|result| result.coefficient_rmse)
        .collect();
    let heldout_values: Vec<_> = results
        .iter()
        .filter_map(|result| result.heldout_rmse)
        .collect();
    let paired_deltas: Vec<_> = results
        .iter()
        .zip(isolated_baseline)
        .filter_map(|(result, baseline)| Some(result.heldout_rmse? - baseline.heldout_rmse?))
        .collect();

    Ok(CouplingDesignSummary {
        design,
        replicate_count: results.len(),
        training_pattern_count: match design {
            CouplingExcitationDesign::IsolatedSingleTaxel => TAXEL_COUNT,
            CouplingExcitationDesign::CyclicSparseOrthogonal => 3,
            CouplingExcitationDesign::RandomDenseMultiPoint
            | CouplingExcitationDesign::RepeatedCommonMode => RANDOM_TRAINING_PATTERNS,
        },
        mean_active_taxel_site_excitations: results
            .iter()
            .map(|result| result.active_site_excitations as f64)
            .sum::<f64>()
            / results.len() as f64,
        mean_identified_output_rows: results
            .iter()
            .map(|result| result.identified_rows as f64)
            .sum::<f64>()
            / results.len() as f64,
        mean_underidentified_output_rows: results
            .iter()
            .map(|result| result.underidentified_rows as f64)
            .sum::<f64>()
            / results.len() as f64,
        mean_coefficient_rmse: mean(&coefficient_values),
        mean_heldout_response_rmse: mean(&heldout_values),
        maximum_heldout_response_rmse: heldout_values.iter().copied().reduce(f64::max),
        evaluated_replicates: heldout_values.len(),
        paired_heldout_rmse_delta_mean_vs_isolated: mean(&paired_deltas),
    })
}

/// Compare three spatial excitation designs on identical paired synthetic
/// kernels and held-out patterns. The active-site count is a stimulus-density
/// descriptor, not a claim about physical reference-contact cost.
pub fn run_coupling_design_campaign(
    first_seed: u64,
    replicate_count: usize,
) -> Result<CouplingDesignCampaignReport, String> {
    if first_seed == 0 {
        return Err("coupling design first seed must be nonzero".to_owned());
    }
    if replicate_count < REQUIRED_REPLICATES {
        return Err("coupling design campaign requires at least fifty replicates".to_owned());
    }
    let last_offset = u64::try_from(replicate_count - 1)
        .map_err(|_| "coupling design replicate count exceeds u64".to_owned())?;
    first_seed
        .checked_add(last_offset)
        .ok_or_else(|| "coupling design seed range overflows u64".to_owned())?;

    let designs = [
        CouplingExcitationDesign::IsolatedSingleTaxel,
        CouplingExcitationDesign::CyclicSparseOrthogonal,
        CouplingExcitationDesign::RandomDenseMultiPoint,
        CouplingExcitationDesign::RepeatedCommonMode,
    ];
    let mut all_results: Vec<Vec<ReplicateResult>> = designs
        .iter()
        .map(|_| Vec::with_capacity(replicate_count))
        .collect();

    for replicate in 0..replicate_count {
        let seed = first_seed + replicate as u64;
        let case = make_case(seed);
        for (design_index, design) in designs.iter().copied().enumerate() {
            all_results[design_index].push(evaluate_design(&case, design, seed)?);
        }
    }

    let isolated = &all_results[0];
    let results = designs
        .iter()
        .copied()
        .enumerate()
        .map(|(index, design)| summarize(design, &all_results[index], isolated))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CouplingDesignCampaignReport {
        taxel_count: TAXEL_COUNT,
        module_count: MODULE_IDS
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        support_radius: SUPPORT_RADIUS,
        first_seed,
        replicate_count,
        heldout_pattern_count: HELDOUT_PATTERN_COUNT,
        results,
        claim_class: "paired_synthetic_coupling_design_campaign_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_enforces_paired_replicate_minimum_and_nonzero_seed() {
        assert!(run_coupling_design_campaign(1, 49).is_err());
        assert!(run_coupling_design_campaign(0, 50).is_err());
        assert!(run_coupling_design_campaign(u64::MAX, 50).is_err());
    }

    #[test]
    fn sparse_orthogonal_codes_identify_local_rows_with_fewer_pattern_trials() {
        let report = run_coupling_design_campaign(20261117, 50).expect("campaign");
        assert_eq!(report.taxel_count, 7);
        assert_eq!(report.module_count, 2);
        assert_eq!(report.replicate_count, 50);
        assert_eq!(report.results.len(), 4);

        let isolated = &report.results[0];
        let coded = &report.results[1];
        let randomized = &report.results[2];
        let common_mode = &report.results[3];

        for identified in [isolated, coded, randomized] {
            assert_eq!(identified.mean_identified_output_rows, TAXEL_COUNT as f64);
            assert_eq!(identified.mean_underidentified_output_rows, 0.0);
            assert!(identified.mean_coefficient_rmse.unwrap() < 1.0e-3);
            assert!(identified.mean_heldout_response_rmse.unwrap() < 1.0e-3);
        }
        assert_eq!(common_mode.mean_identified_output_rows, 0.0);
        assert_eq!(
            common_mode.mean_underidentified_output_rows,
            TAXEL_COUNT as f64
        );

        assert_eq!(isolated.training_pattern_count, TAXEL_COUNT);
        assert_eq!(coded.training_pattern_count, 3);
        assert_eq!(coded.mean_active_taxel_site_excitations, TAXEL_COUNT as f64);
        assert_eq!(
            isolated.mean_active_taxel_site_excitations,
            TAXEL_COUNT as f64
        );
        assert!(
            coded.training_pattern_count < isolated.training_pattern_count,
            "sparse orthogonal coding should require fewer physical pattern trials than isolated impulses"
        );
        assert!(
            coded
                .paired_heldout_rmse_delta_mean_vs_isolated
                .unwrap()
                .abs()
                < 1.0e-3,
            "coded and isolated designs should have matched held-out response quality under this synthetic model"
        );

        assert_eq!(randomized.training_pattern_count, RANDOM_TRAINING_PATTERNS);
        assert!(
            randomized.mean_active_taxel_site_excitations
                > coded.mean_active_taxel_site_excitations,
            "dense randomized codes must report their greater total active taxel-site excitation burden"
        );
        assert!(common_mode.mean_coefficient_rmse.is_none());
        assert!(common_mode.mean_heldout_response_rmse.is_none());
    }
}
