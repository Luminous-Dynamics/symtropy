use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::f64::consts::PI;

pub const CORPUS_SCHEMA: &str =
    "luminous.orbital-feature-measurability.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str =
    "luminous.orbital-feature-measurability.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-FMQ-ANALYTIC-001";
pub const FEATURE_MODEL: &str = "CIRCLE_2D_CENTER_X_CENTER_Y_RADIUS";
const ANGLE_KEY_SCALE: f64 = 1_000_000.0;
const MATRIX_SCALE: f64 = 1_000_000_000.0;
const CONDITION_SCALE: f64 = 10_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QualityStatus {
    Measurable,
    IllConditioned,
    NotMeasurable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CaseDisposition {
    Feasible,
    Infeasible,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub observation_id: String,
    pub view_id: String,
    pub failure_domain_id: String,
    pub contour_angle_deg: f64,
    pub nominally_observed: bool,
    pub robustly_observed: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureCase {
    pub case_id: String,
    pub feature_id: String,
    pub feature_model: String,
    pub max_condition_number: f64,
    pub relative_rank_tolerance: f64,
    pub observations: Vec<Observation>,
    pub expected_nominal_status: QualityStatus,
    pub expected_robust_status: QualityStatus,
    pub expected_disposition: CaseDisposition,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<FeatureCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureDiagnostic {
    pub status: QualityStatus,
    pub observation_row_count: usize,
    pub unique_geometric_support_count: usize,
    pub distinct_failure_domain_count: usize,
    pub information_matrix_q1e9: [[i64; 3]; 3],
    pub eigenvalues_q1e9: [i64; 3],
    pub numerical_rank: usize,
    pub condition_number_q1e4: Option<u64>,
    pub max_condition_number_q1e4: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseReceipt {
    pub case_id: String,
    pub feature_id: String,
    pub nominal: FeatureDiagnostic,
    pub robust: FeatureDiagnostic,
    pub disposition: CaseDisposition,
    pub expected_nominal_status: QualityStatus,
    pub expected_robust_status: QualityStatus,
    pub expected_disposition: CaseDisposition,
    pub expected_outcome_matches: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorpusReceipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_outcome_matches: usize,
    pub expected_outcome_mismatches: usize,
    pub authority_granted: bool,
    pub measurement_uncertainty_estimated: bool,
    pub probability_of_detection_assessed: bool,
    pub physical_validation: String,
    pub cases: Vec<CaseReceipt>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unique_angle_keys(
    observations: &[Observation],
    robust_only: bool,
) -> Vec<i64> {
    let mut keys = BTreeSet::new();
    for observation in observations {
        let admitted = if robust_only {
            observation.robustly_observed
        } else {
            observation.nominally_observed
        };
        if admitted {
            keys.insert((observation.contour_angle_deg * ANGLE_KEY_SCALE).round() as i64);
        }
    }
    keys.into_iter().collect()
}

fn matrix_from_angles(angle_keys: &[i64]) -> [[f64; 3]; 3] {
    let mut matrix = [[0.0_f64; 3]; 3];
    if angle_keys.is_empty() {
        return matrix;
    }
    for key in angle_keys {
        let angle = (*key as f64 / ANGLE_KEY_SCALE) * PI / 180.0;
        let row = [-angle.cos(), -angle.sin(), -1.0];
        for i in 0..3 {
            for j in 0..3 {
                matrix[i][j] += row[i] * row[j];
            }
        }
    }
    let divisor = angle_keys.len() as f64;
    for row in &mut matrix {
        for value in row {
            *value /= divisor;
        }
    }
    matrix
}

fn determinant(matrix: [[f64; 3]; 3]) -> f64 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

fn symmetric_eigenvalues(matrix: [[f64; 3]; 3]) -> [f64; 3] {
    let q = (matrix[0][0] + matrix[1][1] + matrix[2][2]) / 3.0;
    let p2 = (matrix[0][0] - q).powi(2)
        + (matrix[1][1] - q).powi(2)
        + (matrix[2][2] - q).powi(2)
        + 2.0
            * (matrix[0][1].powi(2)
                + matrix[0][2].powi(2)
                + matrix[1][2].powi(2));
    let p = (p2 / 6.0).sqrt();
    if p <= f64::EPSILON {
        return [q.max(0.0); 3];
    }

    let mut normalized = [[0.0_f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            normalized[i][j] = matrix[i][j] / p;
        }
        normalized[i][i] -= q / p;
    }
    let r = (determinant(normalized) / 2.0).clamp(-1.0, 1.0);
    let phi = r.acos() / 3.0;
    let largest = q + 2.0 * p * phi.cos();
    let smallest = q + 2.0 * p * (phi + 2.0 * PI / 3.0).cos();
    let middle = 3.0 * q - largest - smallest;
    let mut values = [smallest.max(0.0), middle.max(0.0), largest.max(0.0)];
    values.sort_by(f64::total_cmp);
    values
}

fn quantize_nonnegative(value: f64, scale: f64) -> Result<i64, String> {
    if !value.is_finite() || value < 0.0 {
        return Err("FEATURE_INFORMATION_NUMERIC_INVALID".to_owned());
    }
    let scaled = (value * scale).round();
    if scaled > i64::MAX as f64 {
        return Err("FEATURE_INFORMATION_NUMERIC_OVERFLOW".to_owned());
    }
    Ok(scaled as i64)
}

fn diagnostic(
    case: &FeatureCase,
    robust_only: bool,
) -> Result<FeatureDiagnostic, String> {
    let angle_keys = unique_angle_keys(&case.observations, robust_only);
    let observation_row_count = case
        .observations
        .iter()
        .filter(|observation| {
            if robust_only {
                observation.robustly_observed
            } else {
                observation.nominally_observed
            }
        })
        .count();
    let mut domains = BTreeSet::new();
    for observation in &case.observations {
        let admitted = if robust_only {
            observation.robustly_observed
        } else {
            observation.nominally_observed
        };
        if admitted {
            domains.insert(observation.failure_domain_id.as_str());
        }
    }

    let matrix = matrix_from_angles(&angle_keys);
    let eigenvalues = symmetric_eigenvalues(matrix);
    let maximum = eigenvalues[2];
    let rank_threshold = maximum * case.relative_rank_tolerance;
    let numerical_rank = eigenvalues
        .iter()
        .filter(|value| **value > rank_threshold && maximum > 0.0)
        .count();
    let condition_number = if numerical_rank == 3 && eigenvalues[0] > 0.0 {
        Some(maximum / eigenvalues[0])
    } else {
        None
    };
    let status = match condition_number {
        None => QualityStatus::NotMeasurable,
        Some(value) if value > case.max_condition_number => QualityStatus::IllConditioned,
        Some(_) => QualityStatus::Measurable,
    };

    let mut information_matrix_q1e9 = [[0_i64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            information_matrix_q1e9[i][j] =
                quantize_nonnegative(matrix[i][j].abs(), MATRIX_SCALE)?;
        }
    }
    let eigenvalues_q1e9 = [
        quantize_nonnegative(eigenvalues[0], MATRIX_SCALE)?,
        quantize_nonnegative(eigenvalues[1], MATRIX_SCALE)?,
        quantize_nonnegative(eigenvalues[2], MATRIX_SCALE)?,
    ];
    let condition_number_q1e4 = condition_number
        .map(|value| quantize_nonnegative(value, CONDITION_SCALE))
        .transpose()?
        .map(|value| value as u64);

    Ok(FeatureDiagnostic {
        status,
        observation_row_count,
        unique_geometric_support_count: angle_keys.len(),
        distinct_failure_domain_count: domains.len(),
        information_matrix_q1e9,
        eigenvalues_q1e9,
        numerical_rank,
        condition_number_q1e4,
        max_condition_number_q1e4: quantize_nonnegative(
            case.max_condition_number,
            CONDITION_SCALE,
        )? as u64,
    })
}

fn validate_case(case: &FeatureCase) -> Result<(), String> {
    if case.case_id.trim().is_empty()
        || case.feature_id.trim().is_empty()
        || case.feature_model != FEATURE_MODEL
        || !case.max_condition_number.is_finite()
        || case.max_condition_number <= 1.0
        || !case.relative_rank_tolerance.is_finite()
        || case.relative_rank_tolerance <= 0.0
        || case.relative_rank_tolerance >= 1.0
        || case.observations.is_empty()
        || case.observations.len() > 4096
    {
        return Err("FEATURE_CASE_BOUNDS_INVALID".to_owned());
    }
    let mut observation_ids = BTreeSet::new();
    for observation in &case.observations {
        if observation.observation_id.trim().is_empty()
            || !observation_ids.insert(observation.observation_id.as_str())
            || observation.view_id.trim().is_empty()
            || observation.failure_domain_id.trim().is_empty()
            || !observation.contour_angle_deg.is_finite()
            || !(0.0..360.0).contains(&observation.contour_angle_deg)
            || (observation.robustly_observed && !observation.nominally_observed)
        {
            return Err("FEATURE_OBSERVATION_INVALID".to_owned());
        }
    }
    Ok(())
}

pub fn evaluate_case(case: &FeatureCase) -> Result<CaseReceipt, String> {
    validate_case(case)?;
    let nominal = diagnostic(case, false)?;
    let robust = diagnostic(case, true)?;
    let disposition = if nominal.status == QualityStatus::NotMeasurable {
        CaseDisposition::Infeasible
    } else if nominal.status == QualityStatus::Measurable
        && robust.status == QualityStatus::Measurable
    {
        CaseDisposition::Feasible
    } else {
        CaseDisposition::Indeterminate
    };
    let expected_outcome_matches = nominal.status == case.expected_nominal_status
        && robust.status == case.expected_robust_status
        && disposition == case.expected_disposition;

    Ok(CaseReceipt {
        case_id: case.case_id.clone(),
        feature_id: case.feature_id.clone(),
        nominal,
        robust,
        disposition,
        expected_nominal_status: case.expected_nominal_status,
        expected_robust_status: case.expected_robust_status,
        expected_disposition: case.expected_disposition,
        expected_outcome_matches,
    })
}

pub fn audit_corpus(bytes: &[u8], source_commit: &str) -> Result<CorpusReceipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: Corpus = serde_json::from_slice(bytes)
        .map_err(|error| format!("FEATURE_CORPUS_PARSE_FAILED:{error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling
            != "synthetic_2d_circle_geometric_identifiability_only"
        || corpus.cases.len() != 5
    {
        return Err("FEATURE_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }
    let mut case_ids = BTreeSet::new();
    let mut receipts = Vec::with_capacity(corpus.cases.len());
    let mut matches = 0;
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("FEATURE_CASE_ID_DUPLICATE".to_owned());
        }
        let receipt = evaluate_case(case)?;
        if receipt.expected_outcome_matches {
            matches += 1;
        }
        receipts.push(receipt);
    }
    Ok(CorpusReceipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        input_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: receipts.len(),
        expected_outcome_matches: matches,
        expected_outcome_mismatches: receipts.len() - matches,
        authority_granted: false,
        measurement_uncertainty_estimated: false,
        probability_of_detection_assessed: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        cases: receipts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(id: &str, angle: f64, nominal: bool, robust: bool) -> Observation {
        Observation {
            observation_id: id.to_owned(),
            view_id: format!("VIEW-{id}"),
            failure_domain_id: format!("DOMAIN-{id}"),
            contour_angle_deg: angle,
            nominally_observed: nominal,
            robustly_observed: robust,
        }
    }

    fn case(observations: Vec<Observation>, max_condition_number: f64) -> FeatureCase {
        FeatureCase {
            case_id: "TEST-CASE".to_owned(),
            feature_id: "HOLE-1".to_owned(),
            feature_model: FEATURE_MODEL.to_owned(),
            max_condition_number,
            relative_rank_tolerance: 1e-9,
            observations,
            expected_nominal_status: QualityStatus::Measurable,
            expected_robust_status: QualityStatus::Measurable,
            expected_disposition: CaseDisposition::Feasible,
        }
    }

    #[test]
    fn four_cardinal_contour_samples_have_condition_number_two() {
        let result = evaluate_case(&case(
            vec![
                observation("O0", 0.0, true, true),
                observation("O1", 90.0, true, true),
                observation("O2", 180.0, true, true),
                observation("O3", 270.0, true, true),
            ],
            100.0,
        ))
        .unwrap();
        assert_eq!(result.nominal.numerical_rank, 3);
        assert_eq!(result.nominal.condition_number_q1e4, Some(20_000));
        assert_eq!(result.nominal.eigenvalues_q1e9, [500_000_000, 500_000_000, 1_000_000_000]);
        assert_eq!(result.disposition, CaseDisposition::Feasible);
    }

    #[test]
    fn narrow_contour_arc_is_ill_conditioned_even_when_full_rank() {
        let angles = [315.0, 330.0, 345.0, 0.0, 15.0, 30.0, 45.0];
        let observations = angles
            .iter()
            .enumerate()
            .map(|(index, angle)| observation(&format!("O{index}"), *angle, true, true))
            .collect();
        let result = evaluate_case(&case(observations, 100.0)).unwrap();
        assert_eq!(result.nominal.numerical_rank, 3);
        assert_eq!(result.nominal.status, QualityStatus::IllConditioned);
        assert_eq!(result.disposition, CaseDisposition::Indeterminate);
    }

    #[test]
    fn repeated_same_direction_does_not_create_geometric_rank() {
        let observations = (0..4)
            .map(|index| observation(&format!("O{index}"), 0.0, true, true))
            .collect();
        let result = evaluate_case(&case(observations, 100.0)).unwrap();
        assert_eq!(result.nominal.observation_row_count, 4);
        assert_eq!(result.nominal.unique_geometric_support_count, 1);
        assert_eq!(result.nominal.numerical_rank, 1);
        assert_eq!(result.nominal.status, QualityStatus::NotMeasurable);
        assert_eq!(result.disposition, CaseDisposition::Infeasible);
    }

    #[test]
    fn nominally_adequate_but_robustly_incomplete_case_abstains() {
        let result = evaluate_case(&case(
            vec![
                observation("O0", 0.0, true, true),
                observation("O1", 90.0, true, true),
                observation("O2", 180.0, true, false),
                observation("O3", 270.0, true, false),
            ],
            100.0,
        ))
        .unwrap();
        assert_eq!(result.nominal.status, QualityStatus::Measurable);
        assert_eq!(result.robust.status, QualityStatus::NotMeasurable);
        assert_eq!(result.disposition, CaseDisposition::Indeterminate);
    }

    #[test]
    fn rigid_rotation_of_contour_support_preserves_spectrum() {
        let original = case(
            vec![
                observation("O0", 0.0, true, true),
                observation("O1", 90.0, true, true),
                observation("O2", 180.0, true, true),
                observation("O3", 270.0, true, true),
            ],
            100.0,
        );
        let rotated = case(
            vec![
                observation("O0", 37.0, true, true),
                observation("O1", 127.0, true, true),
                observation("O2", 217.0, true, true),
                observation("O3", 307.0, true, true),
            ],
            100.0,
        );
        let a = evaluate_case(&original).unwrap();
        let b = evaluate_case(&rotated).unwrap();
        assert_eq!(a.nominal.eigenvalues_q1e9, b.nominal.eigenvalues_q1e9);
        assert_eq!(a.nominal.condition_number_q1e4, b.nominal.condition_number_q1e4);
    }

    #[test]
    fn duplicate_observation_ids_are_rejected() {
        let mut item = case(vec![
            observation("DUP", 0.0, true, true),
            observation("DUP", 90.0, true, true),
            observation("O2", 180.0, true, true),
        ], 100.0);
        assert_eq!(validate_case(&item), Err("FEATURE_OBSERVATION_INVALID".to_owned()));
        item.case_id.clear();
        assert_eq!(validate_case(&item), Err("FEATURE_CASE_BOUNDS_INVALID".to_owned()));
    }
}
