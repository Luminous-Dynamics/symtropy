//! Paired 113-taxel structural-identifiability benchmark for six-axis wrench constraints.
//!
//! The synthetic contact field has three components per taxel: normal pressure,
//! tangent-u traction and tangent-v traction. Native tactile rows in this first
//! benchmark directly observe normal pressure at the admitted taxels. An
//! independent six-axis wrench adds aggregate force/moment equations.
//!
//! The benchmark deliberately does not reconstruct a field or use priors. It
//! asks exactly which scalar components are identifiable from native rows and
//! from native rows plus an independent wrench. A global wrench can increase
//! matrix rank without making most local taxel components identifiable.
//!
//! All values are structural/synthetic. No physical sensor, force calibration,
//! metrological uncertainty or contact safety result is implied.

#![forbid(unsafe_code)]

use super::wrench_consistency::{
    ReferenceIndependenceDisposition, TaxelGeometry, TaxelGeometrySet,
    WrenchIdentifiabilityReport, WrenchObservationBinding, WrenchReferenceEvidence,
    build_wrench_operator, compare_field_identifiability,
};

const TAXEL_COUNT: usize = 113;
const COMPONENTS_PER_TAXEL: usize = 3;
const PARAMETER_COUNT: usize = TAXEL_COUNT * COMPONENTS_PER_TAXEL;
const MODULE_COUNT: usize = 7;
const REPLICATE_COUNT_MINIMUM: usize = 50;
const RANK_TOLERANCE: f64 = 1.0e-10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum NativeCoverageScenario {
    AllNormalTaxelsObserved,
    OneNormalTaxelMissing,
    SevenToFourteenNormalTaxelsMissing,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct WrenchCoverageScenarioSummary {
    pub scenario: NativeCoverageScenario,
    pub replicate_count: usize,
    pub mean_missing_normal_taxels: f64,
    pub mean_native_observation_rank: f64,
    pub mean_wrench_constraint_rank: f64,
    pub mean_combined_rank: f64,
    pub mean_incremental_rank_from_wrench: f64,
    pub mean_native_null_space_dimension: f64,
    pub mean_combined_null_space_dimension: f64,
    pub mean_native_identifiable_scalars: f64,
    pub mean_combined_identifiable_scalars: f64,
    pub mean_newly_identifiable_normal_pressure_taxels: f64,
    pub mean_combined_identifiable_tangent_components: f64,
    pub mean_fully_identifiable_taxels: f64,
    pub maximum_combined_null_space_dimension: usize,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Wrench113TaxelCampaignReport {
    pub taxel_count: usize,
    pub module_count: usize,
    pub field_components_per_taxel: usize,
    pub total_field_parameters: usize,
    pub wrench_equation_count: usize,
    pub first_seed: u64,
    pub replicate_count: usize,
    pub geometry_digest: String,
    pub reference_frame_digest: String,
    pub results: Vec<WrenchCoverageScenarioSummary>,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
struct ReplicateMetrics {
    missing_normal_taxels: usize,
    report: WrenchIdentifiabilityReport,
}

fn geometry() -> TaxelGeometrySet {
    let tangent_u = super::wrench_consistency::Vec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    let tangent_v = super::wrench_consistency::Vec3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    let normal = super::wrench_consistency::Vec3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    let taxels = (0..TAXEL_COUNT)
        .map(|index| {
            let column = index % 15;
            let row = index / 15;
            let module = index * MODULE_COUNT / TAXEL_COUNT;
            TaxelGeometry {
                taxel_id: format!("module-{module}-taxel-{index:03}"),
                area_m2: 1.0e-4,
                application_point_m: super::wrench_consistency::Vec3 {
                    x: (column as f64 - 7.0) * 0.01,
                    y: (row as f64 - 3.5) * 0.01,
                    z: 0.0,
                },
                normal,
                tangent_u,
                tangent_v,
            }
        })
        .collect();
    TaxelGeometrySet {
        geometry_digest: "sha256:synthetic-113-taxel-planar-grid-v0.1".to_owned(),
        reference_frame_digest: "sha256:synthetic-113-taxel-wrench-frame-v0.1".to_owned(),
        contact_boundary_digest: "sha256:synthetic-113-taxel-contact-boundary-v0.1".to_owned(),
        wrench_origin_m: super::wrench_consistency::Vec3::default(),
        taxels,
    }
}

fn binding_and_evidence(seed: u64) -> (WrenchObservationBinding, WrenchReferenceEvidence) {
    let sample_identity = format!("synthetic-wrench-sample-{seed:016x}");
    let native_digest = format!("sha256:native-observations-{seed:016x}");
    let wrench_digest = format!("sha256:wrench-observation-{seed:016x}");
    let binding = WrenchObservationBinding {
        sample_identity: sample_identity.clone(),
        native_observation_set_digest: native_digest.clone(),
        wrench_observation_digest: wrench_digest.clone(),
    };
    let evidence = WrenchReferenceEvidence {
        native_observation_set_digest: native_digest,
        wrench_observation_digest: wrench_digest,
        sample_identity,
        reference_frame_digest: "sha256:synthetic-113-taxel-wrench-frame-v0.1".to_owned(),
        contact_boundary_digest: "sha256:synthetic-113-taxel-contact-boundary-v0.1".to_owned(),
        reference_instrument_receipt_id: format!("receipt:synthetic-ati-class-reference-{seed:016x}"),
        reference_ancestry_receipt_id: format!("receipt:independent-wrench-ancestry-{seed:016x}"),
        common_cause_analysis_receipt_id: format!("receipt:wrench-common-cause-{seed:016x}"),
        verifier_receipt_id: format!("receipt:wrench-verifier-{seed:016x}"),
        reference_independence: ReferenceIndependenceDisposition::IndependentlyCharacterized,
        timestamp_alignment_uncertainty_ns: 100_000,
        maximum_permitted_alignment_uncertainty_ns: 500_000,
    };
    (binding, evidence)
}

fn native_pressure_design(missing_taxels: &[usize]) -> Result<(Vec<f64>, usize), String> {
    let mut missing = missing_taxels.to_vec();
    missing.sort_unstable();
    missing.dedup();
    if missing.iter().any(|index| *index >= TAXEL_COUNT) {
        return Err("missing native taxel index is outside the 113-taxel array".to_owned());
    }
    let mut matrix = Vec::with_capacity((TAXEL_COUNT - missing.len()) * PARAMETER_COUNT);
    for taxel in 0..TAXEL_COUNT {
        if missing.binary_search(&taxel).is_ok() {
            continue;
        }
        let mut row = vec![0.0; PARAMETER_COUNT];
        // This synthetic native channel directly observes normal pressure only.
        row[taxel * COMPONENTS_PER_TAXEL] = 1.0;
        matrix.extend(row);
    }
    Ok((matrix, TAXEL_COUNT - missing.len()))
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

fn multi_missing_taxels(seed: u64) -> Vec<usize> {
    // One missing normal channel per module, deliberately distributed across
    // the contact surface so the three normal-force/moment equations are
    // independent over the missing support.
    let mandatory = [0_usize, 30, 34, 61, 68, 90, 107];
    let mut missing = mandatory.to_vec();
    let mut state = seed.max(1) ^ 0x9E37_79B9_7F4A_7C15;
    for module in 0..MODULE_COUNT {
        if xorshift64(&mut state) & 1 == 1 {
            let candidates: Vec<_> = (0..TAXEL_COUNT)
                .filter(|index| *index * MODULE_COUNT / TAXEL_COUNT == module)
                .filter(|index| !missing.contains(index))
                .collect();
            if !candidates.is_empty() {
                let selected = (xorshift64(&mut state) as usize) % candidates.len();
                missing.push(candidates[selected]);
            }
        }
    }
    missing.sort_unstable();
    missing.dedup();
    missing
}

fn evaluate_scenario(
    operator: &super::wrench_consistency::WrenchOperator,
    scenario: NativeCoverageScenario,
    seed: u64,
) -> Result<ReplicateMetrics, String> {
    let missing = match scenario {
        NativeCoverageScenario::AllNormalTaxelsObserved => Vec::new(),
        NativeCoverageScenario::OneNormalTaxelMissing => vec![56],
        NativeCoverageScenario::SevenToFourteenNormalTaxelsMissing => multi_missing_taxels(seed),
    };
    let (native_design, native_rows) = native_pressure_design(&missing)?;
    let (binding, evidence) = binding_and_evidence(seed);
    let report = compare_field_identifiability(
        operator,
        &native_design,
        native_rows,
        &evidence,
        &binding,
        RANK_TOLERANCE,
    )?;
    Ok(ReplicateMetrics {
        missing_normal_taxels: missing.len(),
        report,
    })
}

fn summarize(
    scenario: NativeCoverageScenario,
    replicates: &[ReplicateMetrics],
) -> Result<WrenchCoverageScenarioSummary, String> {
    if replicates.len() < REPLICATE_COUNT_MINIMUM {
        return Err("113-taxel wrench campaign summary requires at least fifty paired replicates".to_owned());
    }
    let mean = |extract: fn(&ReplicateMetrics) -> usize| {
        replicates
            .iter()
            .map(extract)
            .sum::<usize>() as f64
            / replicates.len() as f64
    };
    let mean_f64 = |extract: fn(&ReplicateMetrics) -> f64| {
        replicates.iter().map(extract).sum::<f64>() / replicates.len() as f64
    };
    let maximum_combined_null_space_dimension = replicates
        .iter()
        .map(|replicate| replicate.report.combined_null_space_dimension)
        .max()
        .unwrap_or(0);

    Ok(WrenchCoverageScenarioSummary {
        scenario,
        replicate_count: replicates.len(),
        mean_missing_normal_taxels: mean(|replicate| replicate.missing_normal_taxels),
        mean_native_observation_rank: mean(|replicate| replicate.report.native_observation_rank),
        mean_wrench_constraint_rank: mean(|replicate| replicate.report.wrench_constraint_rank),
        mean_combined_rank: mean(|replicate| replicate.report.combined_rank),
        mean_incremental_rank_from_wrench: mean(|replicate| replicate.report.incremental_rank_from_wrench),
        mean_native_null_space_dimension: mean(|replicate| replicate.report.native_null_space_dimension),
        mean_combined_null_space_dimension: mean(|replicate| replicate.report.combined_null_space_dimension),
        mean_native_identifiable_scalars: mean(|replicate| replicate.report.native_identifiable_scalar_count),
        mean_combined_identifiable_scalars: mean(|replicate| replicate.report.combined_identifiable_scalar_count),
        mean_newly_identifiable_normal_pressure_taxels: mean_f64(|replicate| {
            let native = TAXEL_COUNT - replicate.missing_normal_taxels;
            let combined = replicate
                .report
                .per_taxel
                .iter()
                .filter(|taxel| taxel.normal_pressure_estimable)
                .count();
            combined.saturating_sub(native) as f64
        }),
        mean_combined_identifiable_tangent_components: mean_f64(|replicate| {
            replicate
                .report
                .per_taxel
                .iter()
                .map(|taxel| {
                    usize::from(taxel.tangent_u_estimable)
                        + usize::from(taxel.tangent_v_estimable)
                })
                .sum::<usize>() as f64
        }),
        mean_fully_identifiable_taxels: mean(|replicate| {
            replicate.report.taxels_fully_identifiable_combined
        }),
        maximum_combined_null_space_dimension,
    })
}

/// Run paired full-coverage, one-missing-channel and multi-missing-channel
/// scenarios on the same 113-taxel geometry. The three arms demonstrate when
/// the aggregate wrench can recover a single missing scalar and when it only
/// adds global constraints without identifying individual taxels.
pub fn run_113_taxel_wrench_campaign(
    first_seed: u64,
    replicate_count: usize,
) -> Result<Wrench113TaxelCampaignReport, String> {
    if first_seed == 0 {
        return Err("113-taxel wrench campaign first seed must be nonzero".to_owned());
    }
    if replicate_count < REPLICATE_COUNT_MINIMUM {
        return Err("113-taxel wrench campaign requires at least fifty replicates".to_owned());
    }
    let final_seed = first_seed
        .checked_add(u64::try_from(replicate_count - 1).map_err(|_| {
            "113-taxel wrench campaign replicate count exceeds u64".to_owned()
        })?)
        .ok_or_else(|| "113-taxel wrench campaign seed range overflows u64".to_owned())?;
    if final_seed == 0 {
        return Err("113-taxel wrench campaign seed range is invalid".to_owned());
    }

    let geometry = geometry();
    let operator = build_wrench_operator(&geometry)?;
    let scenarios = [
        NativeCoverageScenario::AllNormalTaxelsObserved,
        NativeCoverageScenario::OneNormalTaxelMissing,
        NativeCoverageScenario::SevenToFourteenNormalTaxelsMissing,
    ];
    let mut summaries = Vec::with_capacity(scenarios.len());

    for scenario in scenarios {
        let mut replicates = Vec::with_capacity(replicate_count);
        for offset in 0..replicate_count {
            let seed = first_seed + offset as u64;
            replicates.push(evaluate_scenario(&operator, scenario, seed)?);
        }
        summaries.push(summarize(scenario, &replicates)?);
    }

    Ok(Wrench113TaxelCampaignReport {
        taxel_count: TAXEL_COUNT,
        module_count: MODULE_COUNT,
        field_components_per_taxel: COMPONENTS_PER_TAXEL,
        total_field_parameters: PARAMETER_COUNT,
        wrench_equation_count: 6,
        first_seed,
        replicate_count,
        geometry_digest: geometry.geometry_digest,
        reference_frame_digest: geometry.reference_frame_digest,
        results: summaries,
        claim_class: "paired_synthetic_113_taxel_wrench_identifiability_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_geometry_partitions_all_113_taxels_into_exactly_seven_modules() {
        let geometry = geometry();
        let module_ids: std::collections::BTreeSet<_> = geometry
            .taxels
            .iter()
            .enumerate()
            .map(|(index, _)| index * MODULE_COUNT / TAXEL_COUNT)
            .collect();
        assert_eq!(geometry.taxels.len(), TAXEL_COUNT);
        assert_eq!(module_ids.len(), MODULE_COUNT);
        assert!(module_ids.iter().copied().eq(0..MODULE_COUNT));
        for seed in 1..=100 {
            let missing = multi_missing_taxels(seed);
            assert!((7..=14).contains(&missing.len()));
            let missing_modules: std::collections::BTreeSet<_> =
                missing.iter().map(|index| *index * MODULE_COUNT / TAXEL_COUNT).collect();
            assert_eq!(missing_modules.len(), MODULE_COUNT);
        }
    }

    #[test]
    fn campaign_enforces_minimum_seed_count_and_seed_validity() {
        assert!(run_113_taxel_wrench_campaign(1, 49).is_err());
        assert!(run_113_taxel_wrench_campaign(0, 50).is_err());
        assert!(run_113_taxel_wrench_campaign(u64::MAX, 50).is_err());
    }

    #[test]
    fn six_axis_wrench_adds_global_rank_without_identifying_the_full_113_taxel_field() {
        let report = run_113_taxel_wrench_campaign(20261117, 50).expect("paired campaign");
        assert_eq!(report.taxel_count, 113);
        assert_eq!(report.module_count, 7);
        assert_eq!(report.field_components_per_taxel, 3);
        assert_eq!(report.total_field_parameters, 339);
        assert_eq!(report.wrench_equation_count, 6);
        assert_eq!(report.results.len(), 3);

        let full = &report.results[0];
        assert_eq!(full.mean_native_observation_rank, 113.0);
        assert_eq!(full.mean_wrench_constraint_rank, 6.0);
        assert_eq!(full.mean_incremental_rank_from_wrench, 3.0);
        assert_eq!(full.mean_native_identifiable_scalars, 113.0);
        assert_eq!(full.mean_combined_identifiable_scalars, 113.0);
        assert_eq!(full.mean_newly_identifiable_normal_pressure_taxels, 0.0);
        assert_eq!(full.mean_combined_identifiable_tangent_components, 0.0);
        assert_eq!(full.mean_fully_identifiable_taxels, 0.0);

        let one_missing = &report.results[1];
        assert_eq!(one_missing.mean_missing_normal_taxels, 1.0);
        assert_eq!(one_missing.mean_native_observation_rank, 112.0);
        assert_eq!(one_missing.mean_wrench_constraint_rank, 6.0);
        assert_eq!(one_missing.mean_incremental_rank_from_wrench, 4.0);
        assert_eq!(one_missing.mean_native_identifiable_scalars, 112.0);
        assert_eq!(one_missing.mean_combined_identifiable_scalars, 113.0);
        assert_eq!(one_missing.mean_newly_identifiable_normal_pressure_taxels, 1.0);
        assert_eq!(one_missing.mean_combined_identifiable_tangent_components, 0.0);
        assert_eq!(one_missing.mean_fully_identifiable_taxels, 0.0);

        let multiple_missing = &report.results[2];
        assert!(multiple_missing.mean_missing_normal_taxels >= 7.0);
        assert!(multiple_missing.mean_missing_normal_taxels <= 14.0);
        assert_eq!(multiple_missing.mean_wrench_constraint_rank, 6.0);
        assert_eq!(multiple_missing.mean_incremental_rank_from_wrench, 6.0);
        assert_eq!(
            multiple_missing.mean_native_identifiable_scalars,
            multiple_missing.mean_combined_identifiable_scalars
        );
        assert_eq!(multiple_missing.mean_newly_identifiable_normal_pressure_taxels, 0.0);
        assert_eq!(multiple_missing.mean_combined_identifiable_tangent_components, 0.0);
        assert_eq!(multiple_missing.mean_fully_identifiable_taxels, 0.0);
    }
}
