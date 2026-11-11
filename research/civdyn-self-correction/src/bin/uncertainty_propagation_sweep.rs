// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::uncertainty_propagation::{
    CovarianceMatrix, LinearTransform, NumericUncertaintyKind, PropagatedUncertainty,
    UncertaintySourceReceipt, UncertaintyState, UncertaintyVariable, propagate_linear_uncertainty,
};

const CSV_COLUMNS: usize = 23;

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn fmt_values(values: impl IntoIterator<Item = f64>) -> String {
    values
        .into_iter()
        .map(|value| format!("{value:.8}"))
        .collect::<Vec<_>>()
        .join(";")
}

fn source(
    id: &str,
    kind: NumericUncertaintyKind,
    digest: &str,
    variables: &[&str],
    dependence_group: Option<&str>,
) -> UncertaintySourceReceipt {
    UncertaintySourceReceipt {
        source_id: id.to_string(),
        kind,
        source_digest: digest.to_string(),
        affected_variable_ids: variables.iter().map(|value| value.to_string()).collect(),
        dependence_group_id: dependence_group.map(str::to_string),
    }
}

fn state(
    state_id: &str,
    ids: &[&str],
    values: &[f64],
    covariance: Vec<Vec<f64>>,
    sources: Vec<UncertaintySourceReceipt>,
) -> UncertaintyState {
    UncertaintyState {
        state_id: state_id.to_string(),
        measurement_unit: "canonical_service_units".to_string(),
        variables: ids
            .iter()
            .map(|id| UncertaintyVariable {
                variable_id: (*id).to_string(),
                measurement_unit: "canonical_service_units".to_string(),
            })
            .collect(),
        estimates: values.to_vec(),
        covariance: CovarianceMatrix { values: covariance },
        uncertainty_sources: sources,
    }
}

fn transform(
    input: &UncertaintyState,
    output_ids: &[&str],
    coefficients: Vec<Vec<f64>>,
    transform_id: &str,
) -> LinearTransform {
    LinearTransform {
        transform_id: transform_id.to_string(),
        transform_digest: format!("sha256-{transform_id}"),
        protocol_digest: "sha256-civdyn-uncertainty-protocol-v1".to_string(),
        input_state_id: input.state_id.clone(),
        output_state_id: format!("output-{transform_id}"),
        coefficients,
        output_variable_ids: output_ids.iter().map(|id| id.to_string()).collect(),
    }
}

fn emit(scenario: &str, result: &PropagatedUncertainty) {
    let input_variables = result
        .input_variables
        .iter()
        .map(|variable| variable.variable_id.as_str())
        .collect::<Vec<_>>()
        .join(";");
    let input_values = fmt_values(result.input_estimates.iter().copied());
    let input_covariance_flat = fmt_values(result.input_covariance.flatten());
    let jacobian_rows = result
        .jacobian
        .iter()
        .map(|row| fmt_values(row.iter().copied()))
        .collect::<Vec<_>>()
        .join("/");
    let format_sources = |sources: &[UncertaintySourceReceipt]| {
        sources
            .iter()
            .map(|source| {
                format!(
                    "{}~{}~{}~{}~{}",
                    source.source_id,
                    source.kind.name(),
                    source.source_digest,
                    source.affected_variable_ids.join(";"),
                    source.dependence_group_id.as_deref().unwrap_or("none")
                )
            })
            .collect::<Vec<_>>()
            .join("|")
    };
    let input_source_receipts = format_sources(&result.input_uncertainty_sources);
    let output_source_receipts = format_sources(&result.output_state.uncertainty_sources);

    for (index, variable) in result.output_state.variables.iter().enumerate() {
        let covariance_row =
            fmt_values(result.output_state.covariance.values[index].iter().copied());
        let correlation_row = result.correlation_matrix[index]
            .iter()
            .map(|value| {
                value
                    .map(|number| format!("{number:.8}"))
                    .unwrap_or_else(|| "undefined".to_string())
            })
            .collect::<Vec<_>>()
            .join(";");
        let fields = vec![
            scenario.to_string(),
            result.output_state.state_id.clone(),
            variable.variable_id.clone(),
            format!("{:.8}", result.output_state.estimates[index]),
            format!("{:.8}", result.output_state.covariance.values[index][index]),
            format!("{:.8}", result.standard_uncertainties[index]),
            covariance_row,
            correlation_row,
            result.input_state_id.clone(),
            input_variables.clone(),
            input_values.clone(),
            input_covariance_flat.clone(),
            result.transform_id.clone(),
            result.transform_digest.clone(),
            result.protocol_digest.clone(),
            jacobian_rows.clone(),
            variable.measurement_unit.clone(),
            result.input_uncertainty_sources.len().to_string(),
            input_source_receipts.clone(),
            result.output_state.uncertainty_sources.len().to_string(),
            output_source_receipts.clone(),
            result.jacobian.first().map_or(0, Vec::len).to_string(),
            result.output_state.variables.len().to_string(),
        ];
        assert_eq!(
            fields.len(),
            CSV_COLUMNS,
            "uncertainty propagation receipt must match the frozen schema"
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
}

fn cases() -> Result<Vec<(&'static str, PropagatedUncertainty)>, Box<dyn std::error::Error>> {
    let sum_difference = vec![vec![1.0, 1.0], vec![1.0, -1.0]];

    let correlated = state(
        "input-correlated",
        &["water-service", "energy-service"],
        &[100.0, 80.0],
        vec![vec![4.0, 3.0], vec![3.0, 9.0]],
        vec![
            source(
                "water-survey",
                NumericUncertaintyKind::SamplingDesign,
                "sha256-water-survey",
                &["water-service"],
                Some("common-household-sample"),
            ),
            source(
                "energy-survey",
                NumericUncertaintyKind::SamplingDesign,
                "sha256-energy-survey",
                &["energy-service"],
                Some("common-household-sample"),
            ),
        ],
    );
    let correlated_result = propagate_linear_uncertainty(
        &correlated,
        &transform(
            &correlated,
            &["combined-service", "service-difference"],
            sum_difference.clone(),
            "sum-difference-correlated",
        ),
    )?;

    let independent = state(
        "input-independent-explicit",
        &["water-service", "energy-service"],
        &[100.0, 80.0],
        vec![vec![4.0, 0.0], vec![0.0, 9.0]],
        vec![
            source(
                "water-source",
                NumericUncertaintyKind::SamplingDesign,
                "sha256-water-source",
                &["water-service"],
                None,
            ),
            source(
                "energy-source",
                NumericUncertaintyKind::SamplingDesign,
                "sha256-energy-source",
                &["energy-service"],
                None,
            ),
        ],
    );
    let independent_result = propagate_linear_uncertainty(
        &independent,
        &transform(
            &independent,
            &["combined-service", "service-difference"],
            sum_difference.clone(),
            "sum-difference-independent-declared",
        ),
    )?;

    let common_cause = state(
        "input-common-cause",
        &["meter-a", "meter-b"],
        &[10.0, 12.0],
        vec![vec![4.0, 4.0], vec![4.0, 4.0]],
        vec![source(
            "shared-calibration",
            NumericUncertaintyKind::Calibration,
            "sha256-shared-calibration",
            &["meter-a", "meter-b"],
            Some("calibration-cause-1"),
        )],
    );
    let common_cause_result = propagate_linear_uncertainty(
        &common_cause,
        &transform(
            &common_cause,
            &["meter-sum", "meter-difference"],
            sum_difference,
            "shared-cause-sum-difference",
        ),
    )?;

    let crosswalk = state(
        "input-crosswalk-uncertainty",
        &["observed-value", "crosswalk-adjustment"],
        &[100.0, 2.0],
        vec![vec![4.0, 0.5], vec![0.5, 1.0]],
        vec![
            source(
                "survey-sampling",
                NumericUncertaintyKind::SamplingDesign,
                "sha256-survey-sampling",
                &["observed-value"],
                None,
            ),
            source(
                "spatial-crosswalk",
                NumericUncertaintyKind::SpatialCrosswalk,
                "sha256-crosswalk",
                &["crosswalk-adjustment"],
                Some("boundary-allocation"),
            ),
        ],
    );
    let crosswalk_result = propagate_linear_uncertainty(
        &crosswalk,
        &transform(
            &crosswalk,
            &["harmonized-estimate"],
            vec![vec![1.0, 1.0]],
            "apply-crosswalk-adjustment",
        ),
    )?;

    Ok(vec![
        ("correlated_sum_difference", correlated_result),
        ("independent_covariance_ablation", independent_result),
        ("shared_common_cause", common_cause_result),
        ("explicit_crosswalk_uncertainty", crosswalk_result),
    ])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "scenario,output_state_id,output_variable_id,output_estimate,output_variance,standard_uncertainty,covariance_row,correlation_row,input_state_id,input_variable_ids,input_source_ids,input_covariance_flat,transform_id,transform_digest,protocol_digest,jacobian,measurement_unit,input_source_count,input_source_receipts,output_source_count,output_source_receipts,input_dimension,output_dimension"
    );
    for (scenario, result) in cases()? {
        emit(scenario, &result);
    }
    Ok(())
}
