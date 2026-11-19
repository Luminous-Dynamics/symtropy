//! Regime-bounded SO(3) transform audit for synthetic plasma-shaped fields.
//!
//! This crate verifies the software semantics of proper 3-D rotations on
//! scalars, polar/axial vectors and rank-2 tensors. It is a geometric
//! metamorphic reference, not a plasma solver or proof that a given physical
//! plasma regime has SO(3) symmetry.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

pub const SCHEMA_VERSION: &str = "luminous.plasma-symmetry-transform.v0.1";
pub const BENCHMARK_SCHEMA_VERSION: &str =
    "luminous.plasma-symmetry-transform-benchmark.v0.1";
pub const SO3_PROFILE_ID: &str = "synthetic-so3-scalar-vector-tensor-v0.1";
pub const EVIDENCE_CLASS: &str = "SYNTHETIC_ANALYTIC_TRANSFORM";
pub const CLAIM_CEILING: &str =
    "SO3_GEOMETRIC_TRANSFORM_SOFTWARE_ONLY_NO_PLASMA_REGIME_OR_PHYSICAL_VALIDITY_CLAIM";
const MATRIX_TOLERANCE: f64 = 1.0e-10;
const OUTPUT_TOLERANCE: f64 = 1.0e-10;

pub type Vec3 = [f64; 3];
pub type Mat3 = [[f64; 3]; 3];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformError {
    pub message: String,
}

impl TransformError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TransformError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarField {
    pub field_id: String,
    pub units: String,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorKind {
    Polar,
    Axial,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorField {
    pub field_id: String,
    pub units: String,
    pub kind: VectorKind,
    pub value: Vec3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TensorField {
    pub field_id: String,
    pub units: String,
    pub value: Mat3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldState {
    pub scalars: Vec<ScalarField>,
    pub vectors: Vec<VectorField>,
    pub tensors: Vec<TensorField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationRequest {
    pub schema_version: String,
    pub scenario_id: String,
    pub profile_id: String,
    pub rotation_matrix: Mat3,
    pub input: FieldState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformAuditMetrics {
    pub orthogonality_max_abs: f64,
    pub determinant_error: f64,
    pub scalar_invariance_max_abs: f64,
    pub vector_norm_invariance_max_abs: f64,
    pub tensor_trace_invariance_max_abs: f64,
    pub tensor_frobenius_invariance_max_abs: f64,
    pub tensor_determinant_invariance_max_abs: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationReceiptBody {
    pub schema_version: String,
    pub scenario_id: String,
    pub profile_id: String,
    pub transformation: String,
    pub evidence_class: String,
    pub physical_measurements: bool,
    pub claim_ceiling: String,
    pub input_sha256: String,
    pub rotation_matrix: Mat3,
    pub output: FieldState,
    pub metrics: TransformAuditMetrics,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationReceipt {
    pub body: RotationReceiptBody,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseAudit {
    pub case_id: String,
    pub actual_disposition: String,
    pub expected_disposition: String,
    pub matched_expectation: bool,
    pub error: Option<String>,
    pub receipt_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditReport {
    pub benchmark_id: String,
    pub case_count: usize,
    pub matched_expectations: usize,
    pub mismatched_expectations: usize,
    pub results: Vec<CaseAudit>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BenchmarkManifest {
    schema_version: String,
    benchmark_id: String,
    evidence_class: String,
    physical_measurements: bool,
    claim_ceiling: String,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCase {
    case_id: String,
    request: RotationRequest,
    expected_disposition: String,
}

fn digest_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn digest_serialized<T: Serialize>(value: &T) -> Result<String, TransformError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| TransformError::new(format!("serialization failed: {error}")))?;
    Ok(digest_bytes(&bytes))
}

fn non_empty(value: &str, field: &str) -> Result<(), TransformError> {
    if value.trim().is_empty() {
        return Err(TransformError::new(format!("{field} must be non-empty")));
    }
    Ok(())
}

fn finite_vec(value: &Vec3) -> bool {
    value.iter().all(|component| component.is_finite())
}

fn finite_mat(value: &Mat3) -> bool {
    value.iter().all(finite_vec)
}

fn determinant(matrix: &Mat3) -> f64 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

fn transpose(matrix: &Mat3) -> Mat3 {
    std::array::from_fn(|row| std::array::from_fn(|column| matrix[column][row]))
}

fn mat_mul(left: &Mat3, right: &Mat3) -> Mat3 {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..3)
                .map(|inner| left[row][inner] * right[inner][column])
                .sum()
        })
    })
}

fn mat_vec(matrix: &Mat3, vector: &Vec3) -> Vec3 {
    std::array::from_fn(|row| {
        (0..3)
            .map(|column| matrix[row][column] * vector[column])
            .sum()
    })
}

fn max_abs_vec(vector: &Vec3) -> f64 {
    vector.iter().map(|value| value.abs()).fold(0.0, f64::max)
}

fn max_abs_mat(matrix: &Mat3) -> f64 {
    matrix
        .iter()
        .flat_map(|row| row.iter())
        .map(|value| value.abs())
        .fold(0.0, f64::max)
}

fn vec_norm(vector: &Vec3) -> f64 {
    vector.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn frobenius_norm(matrix: &Mat3) -> f64 {
    matrix
        .iter()
        .flat_map(|row| row.iter())
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt()
}

fn trace(matrix: &Mat3) -> f64 {
    matrix[0][0] + matrix[1][1] + matrix[2][2]
}

fn rotate_tensor(rotation: &Mat3, tensor: &Mat3) -> Mat3 {
    let right = transpose(rotation);
    mat_mul(&mat_mul(rotation, tensor), &right)
}

fn validate_rotation(rotation: &Mat3) -> Result<(f64, f64), TransformError> {
    if !finite_mat(rotation) {
        return Err(TransformError::new("rotation matrix contains a non-finite value"));
    }

    let gram = mat_mul(rotation, &transpose(rotation));
    let identity: Mat3 = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];
    let orthogonality_max_abs = max_abs_mat(&std::array::from_fn(|row| {
        std::array::from_fn(|column| gram[row][column] - identity[row][column])
    }));
    let determinant_error = (determinant(rotation) - 1.0).abs();

    if orthogonality_max_abs > MATRIX_TOLERANCE {
        return Err(TransformError::new(format!(
            "matrix is not orthogonal: residual {orthogonality_max_abs:e}"
        )));
    }
    if determinant_error > MATRIX_TOLERANCE {
        return Err(TransformError::new(format!(
            "matrix is not a proper SO(3) rotation: determinant error {determinant_error:e}"
        )));
    }

    Ok((orthogonality_max_abs, determinant_error))
}

fn validate_state(state: &FieldState) -> Result<(), TransformError> {
    let mut scalar_ids = BTreeSet::new();
    for field in &state.scalars {
        non_empty(&field.field_id, "scalar field_id")?;
        non_empty(&field.units, "scalar units")?;
        if !scalar_ids.insert(field.field_id.as_str()) {
            return Err(TransformError::new(format!(
                "duplicate scalar field_id: {}",
                field.field_id
            )));
        }
        if !field.value.is_finite() {
            return Err(TransformError::new(format!(
                "scalar {} is non-finite",
                field.field_id
            )));
        }
    }

    let mut vector_ids = BTreeSet::new();
    for field in &state.vectors {
        non_empty(&field.field_id, "vector field_id")?;
        non_empty(&field.units, "vector units")?;
        if !vector_ids.insert(field.field_id.as_str()) {
            return Err(TransformError::new(format!(
                "duplicate vector field_id: {}",
                field.field_id
            )));
        }
        if !finite_vec(&field.value) {
            return Err(TransformError::new(format!(
                "vector {} contains a non-finite component",
                field.field_id
            )));
        }
    }

    let mut tensor_ids = BTreeSet::new();
    for field in &state.tensors {
        non_empty(&field.field_id, "tensor field_id")?;
        non_empty(&field.units, "tensor units")?;
        if !tensor_ids.insert(field.field_id.as_str()) {
            return Err(TransformError::new(format!(
                "duplicate tensor field_id: {}",
                field.field_id
            )));
        }
        if !finite_mat(&field.value) {
            return Err(TransformError::new(format!(
                "tensor {} contains a non-finite component",
                field.field_id
            )));
        }
    }

    if state.scalars.is_empty() && state.vectors.is_empty() && state.tensors.is_empty() {
        return Err(TransformError::new("field state must contain at least one field"));
    }
    Ok(())
}

fn validate_request(request: &RotationRequest) -> Result<(), TransformError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(TransformError::new("unsupported symmetry transform schema version"));
    }
    non_empty(&request.scenario_id, "scenario_id")?;
    if request.profile_id != SO3_PROFILE_ID {
        return Err(TransformError::new(format!(
            "unsupported transformation profile: {}",
            request.profile_id
        )));
    }
    validate_rotation(&request.rotation_matrix)?;
    validate_state(&request.input)?;
    Ok(())
}

fn evaluate_metrics(
    input: &FieldState,
    output: &FieldState,
    orthogonality_max_abs: f64,
    determinant_error: f64,
) -> Result<TransformAuditMetrics, TransformError> {
    if input.scalars.len() != output.scalars.len()
        || input.vectors.len() != output.vectors.len()
        || input.tensors.len() != output.tensors.len()
    {
        return Err(TransformError::new("input/output field inventory mismatch"));
    }

    let mut scalar_invariance_max_abs: f64 = 0.0;
    for (source, transformed) in input.scalars.iter().zip(&output.scalars) {
        if source.field_id != transformed.field_id || source.units != transformed.units {
            return Err(TransformError::new("scalar field identity changed during transform"));
        }
        scalar_invariance_max_abs = scalar_invariance_max_abs.max((source.value - transformed.value).abs());
    }

    let mut vector_norm_invariance_max_abs: f64 = 0.0;
    for (source, transformed) in input.vectors.iter().zip(&output.vectors) {
        if source.field_id != transformed.field_id
            || source.units != transformed.units
            || source.kind != transformed.kind
        {
            return Err(TransformError::new("vector field identity or kind changed during transform"));
        }
        vector_norm_invariance_max_abs =
            vector_norm_invariance_max_abs.max((vec_norm(&source.value) - vec_norm(&transformed.value)).abs());
    }

    let mut tensor_trace_invariance_max_abs: f64 = 0.0;
    let mut tensor_frobenius_invariance_max_abs: f64 = 0.0;
    let mut tensor_determinant_invariance_max_abs: f64 = 0.0;
    for (source, transformed) in input.tensors.iter().zip(&output.tensors) {
        if source.field_id != transformed.field_id || source.units != transformed.units {
            return Err(TransformError::new("tensor field identity changed during transform"));
        }
        tensor_trace_invariance_max_abs =
            tensor_trace_invariance_max_abs.max((trace(&source.value) - trace(&transformed.value)).abs());
        tensor_frobenius_invariance_max_abs = tensor_frobenius_invariance_max_abs
            .max((frobenius_norm(&source.value) - frobenius_norm(&transformed.value)).abs());
        tensor_determinant_invariance_max_abs =
            tensor_determinant_invariance_max_abs.max((determinant(&source.value) - determinant(&transformed.value)).abs());
    }

    Ok(TransformAuditMetrics {
        orthogonality_max_abs,
        determinant_error,
        scalar_invariance_max_abs,
        vector_norm_invariance_max_abs,
        tensor_trace_invariance_max_abs,
        tensor_frobenius_invariance_max_abs,
        tensor_determinant_invariance_max_abs,
    })
}

/// Applies an explicit proper 3-D rotation to a synthetic scalar/vector/tensor
/// state. Scalars are invariant; polar/axial vectors transform as Rv for SO(3);
/// rank-2 tensors transform as R T R^T. This function does not establish that
/// an arbitrary real plasma case possesses this symmetry.
pub fn apply_so3_rotation(request: &RotationRequest) -> Result<RotationReceipt, TransformError> {
    validate_request(request)?;
    let (orthogonality_max_abs, determinant_error) =
        validate_rotation(&request.rotation_matrix)?;

    let output = FieldState {
        scalars: request.input.scalars.clone(),
        vectors: request
            .input
            .vectors
            .iter()
            .map(|field| VectorField {
                field_id: field.field_id.clone(),
                units: field.units.clone(),
                kind: field.kind,
                value: mat_vec(&request.rotation_matrix, &field.value),
            })
            .collect(),
        tensors: request
            .input
            .tensors
            .iter()
            .map(|field| TensorField {
                field_id: field.field_id.clone(),
                units: field.units.clone(),
                value: rotate_tensor(&request.rotation_matrix, &field.value),
            })
            .collect(),
    };

    let metrics = evaluate_metrics(
        &request.input,
        &output,
        orthogonality_max_abs,
        determinant_error,
    )?;

    let body = RotationReceiptBody {
        schema_version: SCHEMA_VERSION.to_owned(),
        scenario_id: request.scenario_id.clone(),
        profile_id: request.profile_id.clone(),
        transformation: "SO3_PROPER_ROTATION".to_owned(),
        evidence_class: EVIDENCE_CLASS.to_owned(),
        physical_measurements: false,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        input_sha256: digest_serialized(request)?,
        rotation_matrix: request.rotation_matrix,
        output,
        metrics,
    };
    let receipt_sha256 = digest_serialized(&body)?;
    let receipt = RotationReceipt { body, receipt_sha256 };
    verify_receipt(request, &receipt)?;
    Ok(receipt)
}

/// Verifies request binding, receipt digest, field inventory and geometric
/// invariants. This is a software consistency check, not source authentication.
pub fn verify_receipt(
    request: &RotationRequest,
    receipt: &RotationReceipt,
) -> Result<(), TransformError> {
    let body = &receipt.body;
    if body.schema_version != SCHEMA_VERSION
        || body.profile_id != SO3_PROFILE_ID
        || body.transformation != "SO3_PROPER_ROTATION"
        || body.evidence_class != EVIDENCE_CLASS
        || body.physical_measurements
        || body.claim_ceiling != CLAIM_CEILING
    {
        return Err(TransformError::new("receipt has invalid schema, profile or claim ceiling"));
    }
    if body.scenario_id != request.scenario_id
        || body.input_sha256 != digest_serialized(request)?
        || body.rotation_matrix != request.rotation_matrix
    {
        return Err(TransformError::new("receipt does not bind the supplied request"));
    }
    if digest_serialized(body)? != receipt.receipt_sha256 {
        return Err(TransformError::new("receipt digest mismatch"));
    }

    let (orthogonality, determinant_error) = validate_rotation(&body.rotation_matrix)?;
    let expected_output = FieldState {
        scalars: request.input.scalars.clone(),
        vectors: request
            .input
            .vectors
            .iter()
            .map(|field| VectorField {
                field_id: field.field_id.clone(),
                units: field.units.clone(),
                kind: field.kind,
                value: mat_vec(&body.rotation_matrix, &field.value),
            })
            .collect(),
        tensors: request
            .input
            .tensors
            .iter()
            .map(|field| TensorField {
                field_id: field.field_id.clone(),
                units: field.units.clone(),
                value: rotate_tensor(&body.rotation_matrix, &field.value),
            })
            .collect(),
    };

    if body.output.scalars != expected_output.scalars
        || body.output.vectors != expected_output.vectors
        || body.output.tensors != expected_output.tensors
    {
        return Err(TransformError::new("transformed fields do not match the declared SO(3) action"));
    }

    let expected_metrics = evaluate_metrics(
        &request.input,
        &expected_output,
        orthogonality,
        determinant_error,
    )?;
    let metric_values = [
        (body.metrics.orthogonality_max_abs, expected_metrics.orthogonality_max_abs),
        (body.metrics.determinant_error, expected_metrics.determinant_error),
        (body.metrics.scalar_invariance_max_abs, expected_metrics.scalar_invariance_max_abs),
        (body.metrics.vector_norm_invariance_max_abs, expected_metrics.vector_norm_invariance_max_abs),
        (body.metrics.tensor_trace_invariance_max_abs, expected_metrics.tensor_trace_invariance_max_abs),
        (body.metrics.tensor_frobenius_invariance_max_abs, expected_metrics.tensor_frobenius_invariance_max_abs),
        (body.metrics.tensor_determinant_invariance_max_abs, expected_metrics.tensor_determinant_invariance_max_abs),
    ];
    if metric_values.iter().any(|(actual, expected)| {
        !actual.is_finite() || (actual - expected).abs() > OUTPUT_TOLERANCE
    }) {
        return Err(TransformError::new("receipt audit metrics do not match independently recomputed values"));
    }
    if metric_values.iter().any(|(actual, _)| *actual > OUTPUT_TOLERANCE) {
        return Err(TransformError::new("SO(3) invariance residual exceeds the declared tolerance"));
    }
    Ok(())
}

pub fn audit_manifest_json(source: &str) -> Result<AuditReport, TransformError> {
    let manifest: BenchmarkManifest = serde_json::from_str(source)
        .map_err(|error| TransformError::new(format!("invalid benchmark JSON: {error}")))?;
    if manifest.schema_version != BENCHMARK_SCHEMA_VERSION
        || manifest.evidence_class != EVIDENCE_CLASS
        || manifest.physical_measurements
        || manifest.claim_ceiling != CLAIM_CEILING
    {
        return Err(TransformError::new("benchmark manifest claim ceiling is invalid"));
    }
    non_empty(&manifest.benchmark_id, "benchmark_id")?;
    if manifest.cases.is_empty() {
        return Err(TransformError::new("benchmark manifest has no cases"));
    }

    let mut case_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(manifest.cases.len());
    let mut matched_expectations = 0usize;

    for case in manifest.cases {
        non_empty(&case.case_id, "case_id")?;
        if !case_ids.insert(case.case_id.clone()) {
            return Err(TransformError::new(format!("duplicate case_id: {}", case.case_id)));
        }
        if case.expected_disposition != "COMPLETED" && case.expected_disposition != "REJECTED" {
            return Err(TransformError::new(format!(
                "unsupported expected disposition in case {}",
                case.case_id
            )));
        }

        let attempt = apply_so3_rotation(&case.request);
        let (actual_disposition, error, receipt_sha256) = match attempt {
            Ok(receipt) => (
                "COMPLETED".to_owned(),
                None,
                Some(receipt.receipt_sha256),
            ),
            Err(error) => ("REJECTED".to_owned(), Some(error.to_string()), None),
        };
        let matched = actual_disposition == case.expected_disposition;
        if matched {
            matched_expectations += 1;
        }
        results.push(CaseAudit {
            case_id: case.case_id,
            actual_disposition,
            expected_disposition: case.expected_disposition,
            matched_expectation: matched,
            error,
            receipt_sha256,
        });
    }

    let case_count = results.len();
    Ok(AuditReport {
        benchmark_id: manifest.benchmark_id,
        case_count,
        matched_expectations,
        mismatched_expectations: case_count - matched_expectations,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_state() -> FieldState {
        FieldState {
            scalars: vec![
                ScalarField {
                    field_id: "electron_density".to_owned(),
                    units: "m^-3".to_owned(),
                    value: 2.0,
                },
                ScalarField {
                    field_id: "electron_temperature".to_owned(),
                    units: "eV".to_owned(),
                    value: 4.0,
                },
            ],
            vectors: vec![
                VectorField {
                    field_id: "fluid_velocity".to_owned(),
                    units: "m/s".to_owned(),
                    kind: VectorKind::Polar,
                    value: [1.0, 2.0, 3.0],
                },
                VectorField {
                    field_id: "magnetic_field".to_owned(),
                    units: "T".to_owned(),
                    kind: VectorKind::Axial,
                    value: [0.1, 0.2, 0.3],
                },
            ],
            tensors: vec![TensorField {
                field_id: "pressure_tensor".to_owned(),
                units: "Pa".to_owned(),
                value: [
                    [3.0, 0.25, 0.0],
                    [0.25, 2.0, 0.5],
                    [0.0, 0.5, 1.0],
                ],
            }],
        }
    }

    fn request(rotation_matrix: Mat3) -> RotationRequest {
        RotationRequest {
            schema_version: SCHEMA_VERSION.to_owned(),
            scenario_id: "synthetic-so3-test".to_owned(),
            profile_id: SO3_PROFILE_ID.to_owned(),
            rotation_matrix,
            input: base_state(),
        }
    }

    fn z_quarter_turn() -> Mat3 {
        [
            [0.0, -1.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
        ]
    }

    #[test]
    fn quarter_turn_rotates_vectors_and_rank_two_tensor_correctly() {
        let receipt = apply_so3_rotation(&request(z_quarter_turn())).expect("rotation");
        assert_eq!(receipt.body.output.scalars, base_state().scalars);
        assert_eq!(receipt.body.output.vectors[0].value, [-2.0, 1.0, 3.0]);
        assert_eq!(receipt.body.output.vectors[1].value, [-0.2, 0.1, 0.3]);
        assert_eq!(
            receipt.body.output.tensors[0].value,
            [
                [2.0, -0.25, -0.5],
                [-0.25, 3.0, 0.0],
                [-0.5, 0.0, 1.0],
            ]
        );
        verify_receipt(&request(z_quarter_turn()), &receipt).expect("receipt verification");
    }

    #[test]
    fn identity_is_exact_and_deterministic() {
        let identity: Mat3 = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let req = request(identity);
        let first = apply_so3_rotation(&req).expect("identity");
        let second = apply_so3_rotation(&req).expect("identity replay");
        assert_eq!(first, second);
        assert_eq!(first.body.output, req.input);
    }

    #[test]
    fn arbitrary_axis_rotation_preserves_declared_invariants() {
        let cosine = 0.5_f64;
        let sine = 3.0_f64.sqrt() / 2.0;
        let rotation: Mat3 = [
            [cosine, -sine, 0.0],
            [sine, cosine, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let receipt = apply_so3_rotation(&request(rotation)).expect("rotation");
        let metrics = &receipt.body.metrics;
        assert!(metrics.orthogonality_max_abs < OUTPUT_TOLERANCE);
        assert!(metrics.determinant_error < OUTPUT_TOLERANCE);
        assert!(metrics.scalar_invariance_max_abs < OUTPUT_TOLERANCE);
        assert!(metrics.vector_norm_invariance_max_abs < OUTPUT_TOLERANCE);
        assert!(metrics.tensor_trace_invariance_max_abs < OUTPUT_TOLERANCE);
        assert!(metrics.tensor_frobenius_invariance_max_abs < OUTPUT_TOLERANCE);
        assert!(metrics.tensor_determinant_invariance_max_abs < OUTPUT_TOLERANCE);
    }

    #[test]
    fn reflection_is_not_accepted_as_a_proper_rotation() {
        let reflection: Mat3 = [
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        assert!(apply_so3_rotation(&request(reflection)).is_err());
    }

    #[test]
    fn shear_is_not_accepted_as_a_rotation() {
        let shear: Mat3 = [
            [1.0, 0.1, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        assert!(apply_so3_rotation(&request(shear)).is_err());
    }

    #[test]
    fn duplicate_field_id_is_rejected() {
        let mut req = request(z_quarter_turn());
        req.input.vectors.push(req.input.vectors[0].clone());
        assert!(apply_so3_rotation(&req).is_err());
    }

    #[test]
    fn wrong_profile_and_tampered_receipt_are_rejected() {
        let mut wrong_profile = request(z_quarter_turn());
        wrong_profile.profile_id = "unqualified-lorentz-profile".to_owned();
        assert!(apply_so3_rotation(&wrong_profile).is_err());

        let req = request(z_quarter_turn());
        let mut receipt = apply_so3_rotation(&req).expect("rotation");
        receipt.body.output.vectors[0].value[0] += 1.0;
        assert!(verify_receipt(&req, &receipt).is_err());
    }

    #[test]
    fn frozen_benchmark_manifest_matches_all_expected_dispositions() {
        let source = include_str!("../../../../benchmarks/plasma/symmetry-transform-v0.json");
        let report = audit_manifest_json(source).expect("benchmark manifest");
        assert_eq!(report.case_count, 7);
        assert_eq!(report.mismatched_expectations, 0, "{report:#?}");
    }
}
