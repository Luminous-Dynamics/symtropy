// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::heterogeneity_adequacy::{
    HeterogeneityProfile, OutcomeObservation, StratumDefinition,
};
use civdyn_self_correction::joint_adequacy::{
    assess_joint_predictive_draws, JointComponentSpec, JointMetricConfig, JointObservation,
    JointPredictiveDraw, JointStatisticSpec,
};
use civdyn_self_correction::model_adequacy::{IndependentUnitKind, SplitManifest, SplitRole};

const DRAW_COUNT: usize = 20;
const CSV_COLUMNS: usize = 36;
const SOURCE_DIGEST: &str = "sha256-heterogeneity-reference-source";
const CALIBRATION_DIGEST: &str = "sha256-heterogeneity-calibration";
const MANIFEST_DIGEST: &str = "sha256-heterogeneity-split-manifest";

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "heterogeneity receipt must match frozen schema"
    );
    println!(
        "{}",
        fields
            .iter()
            .map(|field| csv_field(field))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn definitions() -> Vec<StratumDefinition> {
    vec![
        StratumDefinition {
            id: "group-a".to_string(),
            minimum_observations: 2,
        },
        StratumDefinition {
            id: "group-b".to_string(),
            minimum_observations: 2,
        },
    ]
}

fn reference_observations() -> Vec<OutcomeObservation> {
    vec![
        OutcomeObservation {
            observation_id: "a-1".to_string(),
            stratum_id: "group-a".to_string(),
            value: 0.0,
        },
        OutcomeObservation {
            observation_id: "a-2".to_string(),
            stratum_id: "group-a".to_string(),
            value: 2.0,
        },
        OutcomeObservation {
            observation_id: "b-1".to_string(),
            stratum_id: "group-b".to_string(),
            value: 8.0,
        },
        OutcomeObservation {
            observation_id: "b-2".to_string(),
            stratum_id: "group-b".to_string(),
            value: 10.0,
        },
    ]
}

fn pooled_moment_matched_collapse() -> Vec<OutcomeObservation> {
    let offset = 17.0_f64.sqrt();
    vec![
        OutcomeObservation {
            observation_id: "a-1".to_string(),
            stratum_id: "group-a".to_string(),
            value: 5.0 - offset,
        },
        OutcomeObservation {
            observation_id: "a-2".to_string(),
            stratum_id: "group-a".to_string(),
            value: 5.0 + offset,
        },
        OutcomeObservation {
            observation_id: "b-1".to_string(),
            stratum_id: "group-b".to_string(),
            value: 5.0 - offset,
        },
        OutcomeObservation {
            observation_id: "b-2".to_string(),
            stratum_id: "group-b".to_string(),
            value: 5.0 + offset,
        },
    ]
}

fn schema(reference: &HeterogeneityProfile) -> JointStatisticSpec {
    let components = reference
        .joint_component_values()
        .expect("reference heterogeneity profile is complete")
        .into_iter()
        .map(|value| JointComponentSpec {
            id: value.component_id,
            measurement_unit: value.measurement_unit,
            reference_center: value.value,
            reference_scale: 1.0,
        })
        .collect();

    JointStatisticSpec {
        id: "stratified-behavioral-heterogeneity".to_string(),
        source_family: "synthetic-stratified-known-answer".to_string(),
        source_digest: SOURCE_DIGEST.to_string(),
        population: "two-stratum-synthetic-population".to_string(),
        context: "same-pooled-moments-different-heterogeneity".to_string(),
        independent_unit: IndependentUnitKind::Region,
        role: SplitRole::Validation,
        primary: true,
        required: true,
        minimum_draws: DRAW_COUNT,
        components,
    }
}

fn manifest() -> SplitManifest {
    SplitManifest {
        id: "heterogeneity-split-v1".to_string(),
        independent_unit: IndependentUnitKind::Region,
        calibration_data_digest: CALIBRATION_DIGEST.to_string(),
        manifest_digest: MANIFEST_DIGEST.to_string(),
        calibration_units: vec!["calibration-region".to_string()],
        validation_units: vec!["held-out-region".to_string()],
        challenge_units: Vec::new(),
    }
}

fn assess_candidate(
    profile: &HeterogeneityProfile,
    specification: &JointStatisticSpec,
    candidate: &str,
    model_digest: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let split = manifest();
    let reference_components = specification
        .components
        .iter()
        .map(|component| civdyn_self_correction::joint_adequacy::JointComponentValue {
            component_id: component.id.clone(),
            measurement_unit: component.measurement_unit.clone(),
            value: component.reference_center,
        })
        .collect();
    let observation = JointObservation {
        statistic_id: specification.id.clone(),
        independent_unit_id: "held-out-region".to_string(),
        sample_size: 4,
        source_digest: SOURCE_DIGEST.to_string(),
        observation_digest: "sha256-heterogeneity-held-out-observation".to_string(),
        split_manifest_digest: MANIFEST_DIGEST.to_string(),
        components: reference_components,
    };
    let candidate_components = profile.joint_component_values()?;
    let draw_set_digest = format!("sha256-heterogeneity-draws-{candidate}");
    let draws: Vec<JointPredictiveDraw> = (0..DRAW_COUNT)
        .map(|draw_index| JointPredictiveDraw {
            statistic_id: specification.id.clone(),
            independent_unit_id: "held-out-region".to_string(),
            sample_size: 4,
            draw_index: draw_index as u64,
            model_digest: model_digest.to_string(),
            draw_set_digest: draw_set_digest.clone(),
            calibration_data_digest: Some(CALIBRATION_DIGEST.to_string()),
            split_manifest_digest: MANIFEST_DIGEST.to_string(),
            components: candidate_components.clone(),
        })
        .collect();

    let config = JointMetricConfig {
        energy_beta: 1.0,
        variogram_power: 1.0,
        pair_weights: vec![
            1.0;
            specification.components.len() * (specification.components.len() - 1) / 2
        ],
    };
    let result = assess_joint_predictive_draws(
        specification,
        &split,
        SplitRole::Validation,
        &observation,
        &draws,
        &config,
    )?;

    let group_a = profile
        .strata
        .iter()
        .find(|stratum| stratum.stratum_id == "group-a")
        .expect("group A in declared fingerprint");
    let group_b = profile
        .strata
        .iter()
        .find(|stratum| stratum.stratum_id == "group-b")
        .expect("group B in declared fingerprint");

    Ok(vec![
        candidate.to_string(),
        profile.observation_count.to_string(),
        profile.stratum_count.to_string(),
        format!("{:.8}", profile.global_mean),
        format!("{:.8}", profile.global_population_variance),
        format!("{:.8}", profile.global_p10),
        format!("{:.8}", profile.global_median),
        format!("{:.8}", profile.global_p90),
        format!("{:.8}", profile.within_stratum_variance),
        format!("{:.8}", profile.between_stratum_variance),
        format!("{:.12}", profile.variance_decomposition_residual),
        group_a.observed_count.to_string(),
        format!("{:.8}", group_a.sample_share),
        format!("{:.8}", group_a.mean.expect("assessed group A")),
        format!("{:.8}", group_a.population_variance.expect("assessed group A")),
        format!("{:.8}", group_a.p10.expect("assessed group A")),
        format!("{:.8}", group_a.median.expect("assessed group A")),
        format!("{:.8}", group_a.p90.expect("assessed group A")),
        group_b.observed_count.to_string(),
        format!("{:.8}", group_b.sample_share),
        format!("{:.8}", group_b.mean.expect("assessed group B")),
        format!("{:.8}", group_b.population_variance.expect("assessed group B")),
        format!("{:.8}", group_b.p10.expect("assessed group B")),
        format!("{:.8}", group_b.median.expect("assessed group B")),
        format!("{:.8}", group_b.p90.expect("assessed group B")),
        format!("{:.6}", result.energy_beta),
        format!("{:.16}", result.energy_score),
        format!("{:.6}", result.variogram_power),
        format!("{:.16}", result.variogram_score),
        result.predictive_draw_count.to_string(),
        result.model_digest,
        result.draw_set_digest,
        result.source_digest,
        result.observation_digest,
        result.calibration_data_digest,
        result.split_manifest_digest,
    ])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reference = HeterogeneityProfile::summarize(&definitions(), &reference_observations())?;
    let collapsed = HeterogeneityProfile::summarize(
        &definitions(),
        &pooled_moment_matched_collapse(),
    )?;
    let specification = schema(&reference);

    println!(
        "candidate,observation_count,stratum_count,global_mean,global_population_variance,global_p10,global_median,global_p90,within_stratum_variance,between_stratum_variance,variance_decomposition_residual,group_a_count,group_a_sample_share,group_a_mean,group_a_population_variance,group_a_p10,group_a_median,group_a_p90,group_b_count,group_b_sample_share,group_b_mean,group_b_population_variance,group_b_p10,group_b_median,group_b_p90,energy_beta,energy_score,variogram_power,variogram_score,draw_count,model_digest,draw_set_digest,source_digest,observation_digest,calibration_digest,split_manifest_digest"
    );
    emit(assess_candidate(
        &reference,
        &specification,
        "stratified_reference",
        "sha256-heterogeneity-model-reference",
    )?);
    emit(assess_candidate(
        &collapsed,
        &specification,
        "pooled_moment_matched_collapse",
        "sha256-heterogeneity-model-collapsed",
    )?);
    Ok(())
}
