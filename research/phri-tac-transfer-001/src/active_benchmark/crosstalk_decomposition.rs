//! Causal decomposition of mechanical load transfer and electrical readout coupling.
//!
//! The end-to-end contact-to-readout mapping is represented as E * M, where
//! M maps the contact field to an intermediate sensor-plane signal and E maps
//! that intermediate signal through the readout chain. End-to-end data alone
//! cannot identify M and E separately: multiple factorizations can have the
//! same product.
//!
//! Stage separation therefore requires independent intervention evidence:
//! mechanical-stage observations of the intermediate physical state, and
//! electrical injection/loopback observations that bypass the mechanical
//! contact path. A complete end-to-end holdout is still required.
//!
//! Receipt identifiers are references, not verified proof. This module does
//! not validate signatures, physical reference independence, calibration,
//! uncertainty, or hardware configuration. It is a synthetic admission
//! contract, not a sensor driver or authority system.

#![forbid(unsafe_code)]

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct LinearStageOperator {
    pub input_dimension: usize,
    pub output_dimension: usize,
    /// Row-major output_dimension × input_dimension matrix.
    pub coefficients: Vec<f64>,
    pub context_digest: String,
    pub operator_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StageObservationSet {
    pub input_dimension: usize,
    pub output_dimension: usize,
    pub pattern_count: usize,
    /// Row-major pattern × input_dimension matrix.
    pub inputs: Vec<f64>,
    /// Row-major pattern × output_dimension matrix.
    pub outputs: Vec<f64>,
    pub observation_digest: String,
    pub independent_reference_receipt_id: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub stage_context_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EndToEndObservationSet {
    pub input_dimension: usize,
    pub output_dimension: usize,
    pub pattern_count: usize,
    /// Row-major pattern × contact-field input matrix.
    pub contact_inputs: Vec<f64>,
    /// Row-major pattern × raw readout response matrix.
    pub readout_outputs: Vec<f64>,
    pub observation_digest: String,
    pub independent_reference_receipt_id: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub mechanical_context_digest: String,
    pub electrical_context_digest: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum DecompositionDisposition {
    StageSeparatedAndValidated,
    CompositionOnlyStagesUnresolved,
    NonIdentifiableFromStageEvidence,
    ModelMismatch,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct StageFitReport {
    pub design_rank: usize,
    pub required_rank: usize,
    pub full_column_rank: bool,
    pub rmse: Option<f64>,
    pub observation_digest: String,
    pub reference_ancestry_receipt_id: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct DecompositionAssessment {
    pub disposition: DecompositionDisposition,
    pub mechanical_stage_fit: Option<StageFitReport>,
    pub electrical_stage_fit: Option<StageFitReport>,
    pub end_to_end_rmse: Option<f64>,
    pub reasons: Vec<String>,
    pub claim_class: &'static str,
}

fn validate_operator(operator: &LinearStageOperator, name: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if operator.input_dimension == 0 || operator.output_dimension == 0 {
        errors.push(format!("{name} operator dimensions must be nonzero"));
    }
    let expected = operator.input_dimension.checked_mul(operator.output_dimension);
    if expected != Some(operator.coefficients.len()) {
        errors.push(format!("{name} operator coefficient dimensions do not match"));
    }
    if operator.coefficients.iter().any(|value| !value.is_finite()) {
        errors.push(format!("{name} operator contains non-finite coefficients"));
    }
    if operator.context_digest.trim().is_empty() {
        errors.push(format!("{name} operator context digest is missing"));
    }
    if operator.operator_digest.trim().is_empty() {
        errors.push(format!("{name} operator digest is missing"));
    }
    errors
}

fn validate_observations(
    observations: &StageObservationSet,
    operator: &LinearStageOperator,
    name: &str,
) -> Vec<String> {
    let mut errors = Vec::new();
    if observations.input_dimension != operator.input_dimension
        || observations.output_dimension != operator.output_dimension
    {
        errors.push(format!("{name} observation dimensions do not match the operator"));
    }
    if observations.pattern_count == 0
        || observations.inputs.len()
            != observations.pattern_count.saturating_mul(observations.input_dimension)
        || observations.outputs.len()
            != observations.pattern_count.saturating_mul(observations.output_dimension)
    {
        errors.push(format!("{name} observation arrays do not match pattern dimensions"));
    }
    if observations.inputs.iter().any(|value| !value.is_finite())
        || observations.outputs.iter().any(|value| !value.is_finite())
    {
        errors.push(format!("{name} observations contain non-finite values"));
    }
    if observations.observation_digest.trim().is_empty() {
        errors.push(format!("{name} observation digest is missing"));
    }
    if observations.independent_reference_receipt_id.trim().is_empty() {
        errors.push(format!("{name} independent reference receipt is missing"));
    }
    if observations.reference_ancestry_receipt_id.trim().is_empty() {
        errors.push(format!("{name} reference ancestry receipt is missing"));
    }
    if observations.common_cause_analysis_receipt_id.trim().is_empty() {
        errors.push(format!("{name} common-cause analysis receipt is missing"));
    }
    if observations.stage_context_digest != operator.context_digest {
        errors.push(format!("{name} stage context does not match its operator"));
    }
    errors
}

fn validate_end_to_end(
    observations: &EndToEndObservationSet,
    mechanical: &LinearStageOperator,
    electrical: &LinearStageOperator,
) -> Vec<String> {
    let mut errors = Vec::new();
    if observations.input_dimension != mechanical.input_dimension
        || observations.output_dimension != electrical.output_dimension
        || mechanical.output_dimension != electrical.input_dimension
    {
        errors.push("end-to-end dimensions do not compose as E*M".to_owned());
    }
    if observations.pattern_count == 0
        || observations.contact_inputs.len()
            != observations.pattern_count.saturating_mul(observations.input_dimension)
        || observations.readout_outputs.len()
            != observations.pattern_count.saturating_mul(observations.output_dimension)
    {
        errors.push("end-to-end observation arrays do not match pattern dimensions".to_owned());
    }
    if observations.contact_inputs.iter().any(|value| !value.is_finite())
        || observations.readout_outputs.iter().any(|value| !value.is_finite())
    {
        errors.push("end-to-end observations contain non-finite values".to_owned());
    }
    for (name, value) in [
        ("observation digest", observations.observation_digest.as_str()),
        ("independent reference receipt", observations.independent_reference_receipt_id.as_str()),
        ("reference ancestry receipt", observations.reference_ancestry_receipt_id.as_str()),
        ("common-cause analysis receipt", observations.common_cause_analysis_receipt_id.as_str()),
    ] {
        if value.trim().is_empty() {
            errors.push(format!("end-to-end {name} is missing"));
        }
    }
    if observations.mechanical_context_digest != mechanical.context_digest {
        errors.push("end-to-end mechanical context digest mismatch".to_owned());
    }
    if observations.electrical_context_digest != electrical.context_digest {
        errors.push("end-to-end electrical context digest mismatch".to_owned());
    }
    errors
}

fn matrix_rank(matrix: &[f64], row_count: usize, column_count: usize, tolerance: f64) -> usize {
    if row_count == 0 || column_count == 0 {
        return 0;
    }
    let mut reduced = matrix.to_vec();
    let scale = reduced
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return 0;
    }
    let threshold = tolerance * scale;
    let mut pivot_row = 0;
    for column in 0..column_count {
        if pivot_row >= row_count {
            break;
        }
        let pivot = (pivot_row..row_count).max_by(|left, right| {
            reduced[*left * column_count + column]
                .abs()
                .total_cmp(&reduced[*right * column_count + column].abs())
        });
        let Some(pivot) = pivot else {
            break;
        };
        if reduced[pivot * column_count + column].abs() <= threshold {
            continue;
        }
        if pivot != pivot_row {
            for swap_column in 0..column_count {
                reduced.swap(
                    pivot * column_count + swap_column,
                    pivot_row * column_count + swap_column,
                );
            }
        }
        let pivot_value = reduced[pivot_row * column_count + column];
        for row in (pivot_row + 1)..row_count {
            let factor = reduced[row * column_count + column] / pivot_value;
            reduced[row * column_count + column] = 0.0;
            for update_column in (column + 1)..column_count {
                reduced[row * column_count + update_column] -=
                    factor * reduced[pivot_row * column_count + update_column];
            }
        }
        pivot_row += 1;
    }
    pivot_row
}

fn stage_fit(
    observations: &StageObservationSet,
    operator: &LinearStageOperator,
    tolerance: f64,
) -> StageFitReport {
    let rank = matrix_rank(
        &observations.inputs,
        observations.pattern_count,
        observations.input_dimension,
        tolerance,
    );
    let mut squared_error = 0.0;
    for pattern in 0..observations.pattern_count {
        for output in 0..operator.output_dimension {
            let predicted = (0..operator.input_dimension)
                .map(|input| {
                    operator.coefficients[output * operator.input_dimension + input]
                        * observations.inputs[pattern * operator.input_dimension + input]
                })
                .sum::<f64>();
            let residual =
                predicted - observations.outputs[pattern * operator.output_dimension + output];
            squared_error += residual * residual;
        }
    }
    let sample_count = observations.pattern_count * operator.output_dimension;
    StageFitReport {
        design_rank: rank,
        required_rank: operator.input_dimension,
        full_column_rank: rank == operator.input_dimension,
        rmse: Some((squared_error / sample_count as f64).sqrt()),
        observation_digest: observations.observation_digest.clone(),
        reference_ancestry_receipt_id: observations.reference_ancestry_receipt_id.clone(),
    }
}

fn compose_operators(
    mechanical: &LinearStageOperator,
    electrical: &LinearStageOperator,
) -> Vec<f64> {
    let n = mechanical.input_dimension;
    let intermediate = mechanical.output_dimension;
    let out = electrical.output_dimension;
    let mut product = vec![0.0; out * n];
    for row in 0..out {
        for column in 0..n {
            product[row * n + column] = (0..intermediate)
                .map(|inner| {
                    electrical.coefficients[row * intermediate + inner]
                        * mechanical.coefficients[inner * n + column]
                })
                .sum();
        }
    }
    product
}

fn end_to_end_rmse(
    observations: &EndToEndObservationSet,
    composed: &[f64],
) -> f64 {
    let n = observations.input_dimension;
    let out = observations.output_dimension;
    let mut squared_error = 0.0;
    for pattern in 0..observations.pattern_count {
        for row in 0..out {
            let predicted = (0..n)
                .map(|column| {
                    composed[row * n + column]
                        * observations.contact_inputs[pattern * n + column]
                })
                .sum::<f64>();
            let residual = predicted - observations.readout_outputs[pattern * out + row];
            squared_error += residual * residual;
        }
    }
    (squared_error / (observations.pattern_count * out) as f64).sqrt()
}

/// Assess whether an end-to-end operator can be decomposed into identifiable
/// mechanical and electrical stages. Missing stage interventions return a
/// composition-only disposition even when E*M predicts held-out outputs.
pub fn assess_stage_decomposition(
    mechanical: &LinearStageOperator,
    electrical: &LinearStageOperator,
    mechanical_observations: Option<&StageObservationSet>,
    electrical_observations: Option<&StageObservationSet>,
    end_to_end_observations: &EndToEndObservationSet,
    rank_tolerance: f64,
    residual_tolerance: f64,
) -> DecompositionAssessment {
    let mut reasons = validate_operator(mechanical, "mechanical");
    reasons.extend(validate_operator(electrical, "electrical"));
    if !rank_tolerance.is_finite() || rank_tolerance <= 0.0 {
        reasons.push("rank tolerance must be finite and positive".to_owned());
    }
    if !residual_tolerance.is_finite() || residual_tolerance < 0.0 {
        reasons.push("residual tolerance must be finite and nonnegative".to_owned());
    }
    if mechanical.output_dimension != electrical.input_dimension {
        reasons.push("mechanical output dimension must equal electrical input dimension".to_owned());
    }
    reasons.extend(validate_end_to_end(end_to_end_observations, mechanical, electrical));
    if !reasons.is_empty() {
        return DecompositionAssessment {
            disposition: DecompositionDisposition::Rejected,
            mechanical_stage_fit: None,
            electrical_stage_fit: None,
            end_to_end_rmse: None,
            reasons,
            claim_class: "synthetic_stage_decomposition_contract_only",
        };
    }

    let composed = compose_operators(mechanical, electrical);
    let total_rmse = end_to_end_rmse(end_to_end_observations, &composed);
    if !total_rmse.is_finite() || total_rmse > residual_tolerance {
        return DecompositionAssessment {
            disposition: DecompositionDisposition::ModelMismatch,
            mechanical_stage_fit: None,
            electrical_stage_fit: None,
            end_to_end_rmse: Some(total_rmse),
            reasons: vec!["composed E*M operator exceeds end-to-end held-out residual tolerance".to_owned()],
            claim_class: "synthetic_stage_decomposition_contract_only",
        };
    }

    let (Some(mechanical_observations), Some(electrical_observations)) =
        (mechanical_observations, electrical_observations)
    else {
        return DecompositionAssessment {
            disposition: DecompositionDisposition::CompositionOnlyStagesUnresolved,
            mechanical_stage_fit: None,
            electrical_stage_fit: None,
            end_to_end_rmse: Some(total_rmse),
            reasons: vec![
                "end-to-end composition is validated but one or both stage-specific intervention datasets are absent".to_owned(),
            ],
            claim_class: "synthetic_stage_decomposition_contract_only",
        };
    };

    let mut stage_reasons = validate_observations(
        mechanical_observations,
        mechanical,
        "mechanical",
    );
    stage_reasons.extend(validate_observations(
        electrical_observations,
        electrical,
        "electrical",
    ));

    if !stage_reasons.is_empty() {
        return DecompositionAssessment {
            disposition: DecompositionDisposition::CompositionOnlyStagesUnresolved,
            mechanical_stage_fit: None,
            electrical_stage_fit: None,
            end_to_end_rmse: Some(total_rmse),
            reasons: stage_reasons,
            claim_class: "synthetic_stage_decomposition_contract_only",
        };
    }

    let ancestry_ids = [
        mechanical_observations.reference_ancestry_receipt_id.as_str(),
        electrical_observations.reference_ancestry_receipt_id.as_str(),
        end_to_end_observations.reference_ancestry_receipt_id.as_str(),
    ];
    if ancestry_ids[0] == ancestry_ids[1]
        || ancestry_ids[0] == ancestry_ids[2]
        || ancestry_ids[1] == ancestry_ids[2]
    {
        return DecompositionAssessment {
            disposition: DecompositionDisposition::CompositionOnlyStagesUnresolved,
            mechanical_stage_fit: None,
            electrical_stage_fit: None,
            end_to_end_rmse: Some(total_rmse),
            reasons: vec![
                "mechanical, electrical and end-to-end evidence share reference ancestry; stage independence is unresolved".to_owned(),
            ],
            claim_class: "synthetic_stage_decomposition_contract_only",
        };
    }

    let mechanical_fit = stage_fit(
        mechanical_observations,
        mechanical,
        rank_tolerance,
    );
    let electrical_fit = stage_fit(
        electrical_observations,
        electrical,
        rank_tolerance,
    );

    if !mechanical_fit.full_column_rank || !electrical_fit.full_column_rank {
        return DecompositionAssessment {
            disposition: DecompositionDisposition::NonIdentifiableFromStageEvidence,
            mechanical_stage_fit: Some(mechanical_fit),
            electrical_stage_fit: Some(electrical_fit),
            end_to_end_rmse: Some(total_rmse),
            reasons: vec![
                "one or both stage-specific intervention designs are rank deficient".to_owned(),
            ],
            claim_class: "synthetic_stage_decomposition_contract_only",
        };
    }

    let mechanical_rmse = mechanical_fit.rmse.unwrap_or(f64::INFINITY);
    let electrical_rmse = electrical_fit.rmse.unwrap_or(f64::INFINITY);
    if mechanical_rmse > residual_tolerance || electrical_rmse > residual_tolerance {
        return DecompositionAssessment {
            disposition: DecompositionDisposition::ModelMismatch,
            mechanical_stage_fit: Some(mechanical_fit),
            electrical_stage_fit: Some(electrical_fit),
            end_to_end_rmse: Some(total_rmse),
            reasons: vec![
                "one or both stage-specific operators exceed their held-out residual tolerance".to_owned(),
            ],
            claim_class: "synthetic_stage_decomposition_contract_only",
        };
    }

    DecompositionAssessment {
        disposition: DecompositionDisposition::StageSeparatedAndValidated,
        mechanical_stage_fit: Some(mechanical_fit),
        electrical_stage_fit: Some(electrical_fit),
        end_to_end_rmse: Some(total_rmse),
        reasons: Vec::new(),
        claim_class: "synthetic_stage_decomposition_contract_only",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> LinearStageOperator {
        LinearStageOperator {
            input_dimension: 2,
            output_dimension: 2,
            coefficients: vec![1.0, 0.0, 0.0, 1.0],
            context_digest: "sha256:stage-context".to_owned(),
            operator_digest: "sha256:identity".to_owned(),
        }
    }

    fn diagonal_mechanical() -> LinearStageOperator {
        LinearStageOperator {
            input_dimension: 2,
            output_dimension: 2,
            coefficients: vec![1.0, 0.0, 0.0, 2.0],
            context_digest: "sha256:mechanical-context".to_owned(),
            operator_digest: "sha256:mechanical-operator".to_owned(),
        }
    }

    fn diagonal_electrical() -> LinearStageOperator {
        LinearStageOperator {
            input_dimension: 2,
            output_dimension: 2,
            coefficients: vec![2.0, 0.0, 0.0, 3.0],
            context_digest: "sha256:electrical-context".to_owned(),
            operator_digest: "sha256:electrical-operator".to_owned(),
        }
    }

    fn inputs() -> Vec<f64> {
        vec![1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 0.5]
    }

    fn apply(operator: &LinearStageOperator, input_patterns: &[f64]) -> Vec<f64> {
        let pattern_count = input_patterns.len() / operator.input_dimension;
        let mut outputs = Vec::with_capacity(pattern_count * operator.output_dimension);
        for pattern in 0..pattern_count {
            for row in 0..operator.output_dimension {
                let value = (0..operator.input_dimension)
                    .map(|column| {
                        operator.coefficients[row * operator.input_dimension + column]
                            * input_patterns[pattern * operator.input_dimension + column]
                    })
                    .sum();
                outputs.push(value);
            }
        }
        outputs
    }

    fn observations(
        operator: &LinearStageOperator,
        inputs: Vec<f64>,
        ancestry: &str,
        digest: &str,
    ) -> StageObservationSet {
        StageObservationSet {
            input_dimension: operator.input_dimension,
            output_dimension: operator.output_dimension,
            pattern_count: inputs.len() / operator.input_dimension,
            outputs: apply(operator, &inputs),
            inputs,
            observation_digest: format!("sha256:{digest}"),
            independent_reference_receipt_id: format!("receipt:{digest}"),
            reference_ancestry_receipt_id: ancestry.to_owned(),
            common_cause_analysis_receipt_id: format!("receipt:cca-{digest}"),
            stage_context_digest: operator.context_digest.clone(),
        }
    }

    fn end_to_end(
        mechanical: &LinearStageOperator,
        electrical: &LinearStageOperator,
        contact_inputs: Vec<f64>,
    ) -> EndToEndObservationSet {
        let intermediate = apply(mechanical, &contact_inputs);
        let composed_outputs = apply(electrical, &intermediate);
        EndToEndObservationSet {
            input_dimension: mechanical.input_dimension,
            output_dimension: electrical.output_dimension,
            pattern_count: contact_inputs.len() / mechanical.input_dimension,
            contact_inputs,
            readout_outputs: composed_outputs,
            observation_digest: "sha256:end-to-end-observations".to_owned(),
            independent_reference_receipt_id: "receipt:end-to-end-reference".to_owned(),
            reference_ancestry_receipt_id: "ancestry:end-to-end".to_owned(),
            common_cause_analysis_receipt_id: "receipt:end-to-end-cca".to_owned(),
            mechanical_context_digest: mechanical.context_digest.clone(),
            electrical_context_digest: electrical.context_digest.clone(),
        }
    }

    #[test]
    fn independent_stage_interventions_qualify_a_synthetic_factorization() {
        let mechanical = diagonal_mechanical();
        let electrical = diagonal_electrical();
        let mechanical_inputs = inputs();
        let electrical_inputs = inputs();
        let end_inputs = inputs();
        let mechanical_observations = observations(
            &mechanical,
            mechanical_inputs,
            "ancestry:mechanical",
            "mechanical",
        );
        let electrical_observations = observations(
            &electrical,
            electrical_inputs,
            "ancestry:electrical",
            "electrical",
        );
        let end_to_end = end_to_end(&mechanical, &electrical, end_inputs);

        let result = assess_stage_decomposition(
            &mechanical,
            &electrical,
            Some(&mechanical_observations),
            Some(&electrical_observations),
            &end_to_end,
            1.0e-10,
            1.0e-10,
        );
        assert_eq!(
            result.disposition,
            DecompositionDisposition::StageSeparatedAndValidated
        );
        assert!(result.reasons.is_empty());
        assert!(result.end_to_end_rmse.unwrap() < 1.0e-12);
    }

    #[test]
    fn end_to_end_product_does_not_identify_its_factors() {
        let mechanical_a = identity();
        let electrical_a = diagonal_electrical();
        let mechanical_b = diagonal_electrical();
        let electrical_b = identity();
        let contact_inputs = inputs();
        let observations = end_to_end(&mechanical_a, &electrical_a, contact_inputs.clone());
        let alternative = end_to_end(&mechanical_b, &electrical_b, contact_inputs);
        assert_eq!(observations.readout_outputs, alternative.readout_outputs);

        let result = assess_stage_decomposition(
            &mechanical_a,
            &electrical_a,
            None,
            None,
            &observations,
            1.0e-10,
            1.0e-10,
        );
        assert_eq!(
            result.disposition,
            DecompositionDisposition::CompositionOnlyStagesUnresolved
        );
        assert!(result.end_to_end_rmse.unwrap() < 1.0e-12);
    }

    #[test]
    fn rank_deficient_stage_injection_keeps_the_decomposition_unresolved() {
        let mechanical = diagonal_mechanical();
        let electrical = diagonal_electrical();
        let repeated = vec![1.0, 1.0, 2.0, 2.0];
        let mechanical_observations = observations(
            &mechanical,
            repeated.clone(),
            "ancestry:mechanical",
            "mechanical",
        );
        let electrical_observations = observations(
            &electrical,
            inputs(),
            "ancestry:electrical",
            "electrical",
        );
        let end = end_to_end(&mechanical, &electrical, inputs());
        let result = assess_stage_decomposition(
            &mechanical,
            &electrical,
            Some(&mechanical_observations),
            Some(&electrical_observations),
            &end,
            1.0e-10,
            1.0e-10,
        );
        assert_eq!(
            result.disposition,
            DecompositionDisposition::NonIdentifiableFromStageEvidence
        );
        assert_eq!(result.mechanical_stage_fit.unwrap().design_rank, 1);
    }

    #[test]
    fn shared_reference_ancestry_prevents_stage_separation_claim() {
        let mechanical = diagonal_mechanical();
        let electrical = diagonal_electrical();
        let mechanical_observations = observations(
            &mechanical,
            inputs(),
            "ancestry:shared",
            "mechanical",
        );
        let electrical_observations = observations(
            &electrical,
            inputs(),
            "ancestry:shared",
            "electrical",
        );
        let end = end_to_end(&mechanical, &electrical, inputs());
        let result = assess_stage_decomposition(
            &mechanical,
            &electrical,
            Some(&mechanical_observations),
            Some(&electrical_observations),
            &end,
            1.0e-10,
            1.0e-10,
        );
        assert_eq!(
            result.disposition,
            DecompositionDisposition::CompositionOnlyStagesUnresolved
        );
    }

    #[test]
    fn wrong_stage_operator_is_detected_by_the_independent_stage_observations() {
        let mechanical = identity();
        let electrical = diagonal_electrical();
        let true_mechanical = diagonal_mechanical();
        let mechanical_observations = observations(
            &true_mechanical,
            inputs(),
            "ancestry:mechanical",
            "mechanical",
        );
        let electrical_observations = observations(
            &electrical,
            inputs(),
            "ancestry:electrical",
            "electrical",
        );
        let end = end_to_end(&true_mechanical, &electrical, inputs());
        let result = assess_stage_decomposition(
            &mechanical,
            &electrical,
            Some(&mechanical_observations),
            Some(&electrical_observations),
            &end,
            1.0e-10,
            1.0e-8,
        );
        assert_eq!(
            result.disposition,
            DecompositionDisposition::ModelMismatch
        );
    }
}
