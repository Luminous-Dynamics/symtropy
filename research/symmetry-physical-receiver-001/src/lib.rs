// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Independent receiver for a bounded symmetry-to-physical evidence handoff.
//!
//! V0 verifies content bindings and dense 3D coordinate-re-description laws.
//! It intentionally does not claim discrete-step equivariance, physical model
//! fidelity, or scientific-search pruning.

#![allow(clippy::needless_range_loop)]

use sha2::{Digest, Sha256};
use std::fmt;

pub type Vector3 = [f64; 3];
pub type Matrix3 = [[f64; 3]; 3];
pub type Jacobian2x3 = [[f64; 3]; 2];

const SOURCE_DOMAIN: &[u8] = b"LUMINOUS-PHYS-RECEIVER\0SOURCE\0V1";
const GENERATOR_DOMAIN: &[u8] = b"LUMINOUS-PHYS-RECEIVER\0GENERATORS\0V1";
const FIXTURE_DOMAIN: &[u8] = b"LUMINOUS-PHYS-RECEIVER\0FIXTURE\0V1";
const TASK_DOMAIN: &[u8] = b"LUMINOUS-PHYS-RECEIVER\0TASK\0V1";
const RECEIPT_DOMAIN: &[u8] = b"LUMINOUS-PHYS-RECEIVER\0RECEIPT\0V1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusV1 {
    Pass,
    Fail,
    Inconclusive,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKindV1 {
    InfinitesimalGenerator,
    FiniteTransformation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClassV1 {
    LearnedProposal,
    SampledNumeric,
    NumericLieBasis,
    ExactPolynomialIdentity,
    ExactFiniteGroup,
    FormalProof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceStatusV1 {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReceiverProfileV1 {
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
    pub symmetry_tolerance: f64,
    pub relative_cholesky_pivot: f64,
    pub maximum_coordinate_condition_inf: f64,
    pub maximum_encoded_bytes: usize,
}

impl Default for ReceiverProfileV1 {
    fn default() -> Self {
        Self {
            absolute_tolerance: 1.0e-11,
            relative_tolerance: 1.0e-10,
            symmetry_tolerance: 1.0e-12,
            relative_cholesky_pivot: 1.0e-14,
            maximum_coordinate_condition_inf: 1.0e8,
            maximum_encoded_bytes: 8 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiverErrorV1 {
    InvalidProfile,
    InvalidSourceIdentity,
    EmptyGeneratorSet,
    NonFiniteInput,
    NonFiniteIntermediate,
    SingularCoordinateTransform { pivot: usize },
    NonPositiveDefiniteMetric { pivot: usize },
    AsymmetricMetric,
    EncodingBudgetExceeded,
    EncodingLengthOverflow,
    SourceReceiptDigestMismatch,
    GeneratorSetDigestMismatch,
    FixtureDigestMismatch,
    TransformDigestMismatch,
    MassMetricDigestMismatch,
    TransformedMassMetricDigestMismatch,
    ForceCovectorDigestMismatch,
    ConstraintMapDigestMismatch,
    InitialStateDigestMismatch,
    TaskSemanticsDigestMismatch,
}

impl fmt::Display for ReceiverErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ReceiverErrorV1 {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMathReceiptV1 {
    pub repository: String,
    pub commit_sha: String,
    pub receipt_id: String,
    pub checker_id: String,
    pub checker_version: String,
    pub profile_id: String,
    pub action_id: String,
    pub coordinate_basis_id: String,
    pub metric_id: String,
    pub evidence_class: EvidenceClassV1,
    pub qualification_status: SourceStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSemanticsV1 {
    pub task_sha256: [u8; 32],
    pub candidate_domain_sha256: [u8; 32],
    pub objective_sha256: [u8; 32],
    pub feasibility_constraints_sha256: [u8; 32],
    pub initial_and_boundary_conditions_sha256: [u8; 32],
    pub observable_mapping_sha256: [u8; 32],
    pub answer_interpretation_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalFixtureV1 {
    pub action_kind: ActionKindV1,
    /// q = S y. S maps transformed-coordinate tangents to canonical tangents.
    pub transform: Matrix3,
    pub mass_q: Matrix3,
    pub mass_y_claimed: Matrix3,
    pub force_q: Vector3,
    pub force_y_claimed: Vector3,
    pub momentum_q: Vector3,
    pub momentum_y_claimed: Vector3,
    pub constraint_jacobian_q: Jacobian2x3,
    pub constraint_jacobian_y_claimed: Jacobian2x3,
    pub tangent_q: Vector3,
    pub tangent_y_claimed: Vector3,
    pub symmetry_matrices_q: Vec<Matrix3>,
    pub symmetry_matrices_y_claimed: Vec<Matrix3>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestBindingsV1 {
    pub source_receipt_sha256: [u8; 32],
    pub generator_set_sha256: [u8; 32],
    pub physical_fixture_sha256: [u8; 32],
    pub transform_sha256: [u8; 32],
    pub mass_metric_sha256: [u8; 32],
    pub transformed_mass_metric_sha256: [u8; 32],
    pub force_covectors_sha256: [u8; 32],
    pub constraint_map_sha256: [u8; 32],
    pub initial_state_sha256: [u8; 32],
    pub task_semantics_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverInputV1 {
    pub schema_version: u32,
    pub handoff_id: String,
    pub source_receipt: SourceMathReceiptV1,
    pub physical_fixture: PhysicalFixtureV1,
    pub task_before: TaskSemanticsV1,
    pub task_after: TaskSemanticsV1,
    pub digests: DigestBindingsV1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefectV1 {
    pub absolute: f64,
    pub relative: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverObligationsV1 {
    pub content_binding: StatusV1,
    pub mass_metric_congruence: StatusV1,
    pub force_covector_duality: StatusV1,
    pub momentum_one_form: StatusV1,
    pub constraint_jacobian: StatusV1,
    pub action_conjugation: StatusV1,
    pub generator_metric_isometry: StatusV1,
    pub tangent_norm: StatusV1,
    pub cotangent_dual_norm: StatusV1,
    pub canonical_pairing: StatusV1,
    pub full_task_semantics: StatusV1,
    pub discrete_step_equivariance: StatusV1,
    pub physical_observables: StatusV1,
    pub numerical_error_envelope: StatusV1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverDefectsV1 {
    pub mass_metric_congruence: DefectV1,
    pub force_covector_duality: DefectV1,
    pub momentum_one_form: DefectV1,
    pub constraint_jacobian: DefectV1,
    pub action_conjugation_max: f64,
    pub action_metric_max: f64,
    pub tangent_norm: DefectV1,
    pub cotangent_dual_norm: DefectV1,
    pub canonical_pairing: DefectV1,
    pub coordinate_condition_inf: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverReceiptV1 {
    pub schema_version: u32,
    pub handoff_id: String,
    pub receiver_status: StatusV1,
    pub obligations: ReceiverObligationsV1,
    pub defects: ReceiverDefectsV1,
    pub physical_equivalence_authority: &'static str,
    pub scientific_search_pruning_authorized: bool,
    pub receipt_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EncoderV1 {
    bytes: Vec<u8>,
    maximum: usize,
}

impl EncoderV1 {
    fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
        }
    }

    fn put(&mut self, bytes: &[u8]) -> Result<(), ReceiverErrorV1> {
        let size = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(ReceiverErrorV1::EncodingLengthOverflow)?;
        if size > self.maximum {
            return Err(ReceiverErrorV1::EncodingBudgetExceeded);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), ReceiverErrorV1> {
        self.put(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), ReceiverErrorV1> {
        self.put(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), ReceiverErrorV1> {
        self.put(&value.to_le_bytes())
    }

    fn usize(&mut self, value: usize) -> Result<(), ReceiverErrorV1> {
        self.u64(u64::try_from(value).map_err(|_| ReceiverErrorV1::EncodingLengthOverflow)?)
    }

    fn f64(&mut self, value: f64) -> Result<(), ReceiverErrorV1> {
        if !value.is_finite() {
            return Err(ReceiverErrorV1::NonFiniteInput);
        }
        self.put(&value.to_bits().to_le_bytes())
    }

    fn string(&mut self, value: &str) -> Result<(), ReceiverErrorV1> {
        self.usize(value.len())?;
        self.put(value.as_bytes())
    }

    fn digest(&mut self, value: &[u8; 32]) -> Result<(), ReceiverErrorV1> {
        self.put(value)
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut output = [0_u8; 32];
    output.copy_from_slice(&digest);
    output
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = sha256(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn encode_source_receipt(
    receipt: &SourceMathReceiptV1,
    profile: ReceiverProfileV1,
) -> Result<Vec<u8>, ReceiverErrorV1> {
    let mut out = EncoderV1::new(profile.maximum_encoded_bytes);
    out.put(SOURCE_DOMAIN)?;
    out.string(&receipt.repository)?;
    out.string(&receipt.commit_sha)?;
    out.string(&receipt.receipt_id)?;
    out.string(&receipt.checker_id)?;
    out.string(&receipt.checker_version)?;
    out.string(&receipt.profile_id)?;
    out.string(&receipt.action_id)?;
    out.string(&receipt.coordinate_basis_id)?;
    out.string(&receipt.metric_id)?;
    out.u8(match receipt.evidence_class {
        EvidenceClassV1::LearnedProposal => 1,
        EvidenceClassV1::SampledNumeric => 2,
        EvidenceClassV1::NumericLieBasis => 3,
        EvidenceClassV1::ExactPolynomialIdentity => 4,
        EvidenceClassV1::ExactFiniteGroup => 5,
        EvidenceClassV1::FormalProof => 6,
    })?;
    out.u8(match receipt.qualification_status {
        SourceStatusV1::Pass => 1,
        SourceStatusV1::Fail => 2,
        SourceStatusV1::Inconclusive => 3,
    })?;
    Ok(out.finish())
}

fn encode_generators(
    receipt: &SourceMathReceiptV1,
    fixture: &PhysicalFixtureV1,
    profile: ReceiverProfileV1,
) -> Result<Vec<u8>, ReceiverErrorV1> {
    let mut out = EncoderV1::new(profile.maximum_encoded_bytes);
    out.put(GENERATOR_DOMAIN)?;
    out.string(&receipt.action_id)?;
    out.u8(match fixture.action_kind {
        ActionKindV1::InfinitesimalGenerator => 1,
        ActionKindV1::FiniteTransformation => 2,
    })?;
    out.usize(fixture.symmetry_matrices_q.len())?;
    for matrix in &fixture.symmetry_matrices_q {
        encode_matrix3_into(&mut out, matrix)?;
    }
    Ok(out.finish())
}

fn encode_matrix3_into(
    out: &mut EncoderV1,
    matrix: &Matrix3,
) -> Result<(), ReceiverErrorV1> {
    for row in matrix {
        for value in row {
            out.f64(*value)?;
        }
    }
    Ok(())
}

fn encode_vector3_into(
    out: &mut EncoderV1,
    vector: &Vector3,
) -> Result<(), ReceiverErrorV1> {
    for value in vector {
        out.f64(*value)?;
    }
    Ok(())
}

fn encode_jacobian_into(
    out: &mut EncoderV1,
    jacobian: &Jacobian2x3,
) -> Result<(), ReceiverErrorV1> {
    for row in jacobian {
        for value in row {
            out.f64(*value)?;
        }
    }
    Ok(())
}

fn encode_fixture(
    fixture: &PhysicalFixtureV1,
    profile: ReceiverProfileV1,
) -> Result<Vec<u8>, ReceiverErrorV1> {
    let mut out = EncoderV1::new(profile.maximum_encoded_bytes);
    out.put(FIXTURE_DOMAIN)?;
    out.u8(match fixture.action_kind {
        ActionKindV1::InfinitesimalGenerator => 1,
        ActionKindV1::FiniteTransformation => 2,
    })?;
    encode_matrix3_into(&mut out, &fixture.transform)?;
    encode_matrix3_into(&mut out, &fixture.mass_q)?;
    encode_matrix3_into(&mut out, &fixture.mass_y_claimed)?;
    encode_vector3_into(&mut out, &fixture.force_q)?;
    encode_vector3_into(&mut out, &fixture.force_y_claimed)?;
    encode_vector3_into(&mut out, &fixture.momentum_q)?;
    encode_vector3_into(&mut out, &fixture.momentum_y_claimed)?;
    encode_jacobian_into(&mut out, &fixture.constraint_jacobian_q)?;
    encode_jacobian_into(&mut out, &fixture.constraint_jacobian_y_claimed)?;
    encode_vector3_into(&mut out, &fixture.tangent_q)?;
    encode_vector3_into(&mut out, &fixture.tangent_y_claimed)?;
    out.usize(fixture.symmetry_matrices_q.len())?;
    for matrix in &fixture.symmetry_matrices_q {
        encode_matrix3_into(&mut out, matrix)?;
    }
    out.usize(fixture.symmetry_matrices_y_claimed.len())?;
    for matrix in &fixture.symmetry_matrices_y_claimed {
        encode_matrix3_into(&mut out, matrix)?;
    }
    Ok(out.finish())
}

fn encode_task_semantics(
    semantics: &TaskSemanticsV1,
    profile: ReceiverProfileV1,
) -> Result<Vec<u8>, ReceiverErrorV1> {
    let mut out = EncoderV1::new(profile.maximum_encoded_bytes);
    out.put(TASK_DOMAIN)?;
    out.digest(&semantics.task_sha256)?;
    out.digest(&semantics.candidate_domain_sha256)?;
    out.digest(&semantics.objective_sha256)?;
    out.digest(&semantics.feasibility_constraints_sha256)?;
    out.digest(&semantics.initial_and_boundary_conditions_sha256)?;
    out.digest(&semantics.observable_mapping_sha256)?;
    out.digest(&semantics.answer_interpretation_sha256)?;
    Ok(out.finish())
}

fn validate_profile(profile: ReceiverProfileV1) -> Result<(), ReceiverErrorV1> {
    if !profile.absolute_tolerance.is_finite()
        || profile.absolute_tolerance < 0.0
        || !profile.relative_tolerance.is_finite()
        || profile.relative_tolerance < 0.0
        || !profile.symmetry_tolerance.is_finite()
        || profile.symmetry_tolerance < 0.0
        || !profile.relative_cholesky_pivot.is_finite()
        || profile.relative_cholesky_pivot <= 0.0
        || profile.relative_cholesky_pivot >= 1.0
        || !profile.maximum_coordinate_condition_inf.is_finite()
        || profile.maximum_coordinate_condition_inf < 1.0
        || profile.maximum_encoded_bytes == 0
    {
        return Err(ReceiverErrorV1::InvalidProfile);
    }
    Ok(())
}

fn transpose3(matrix: &Matrix3) -> Matrix3 {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row][column] = matrix[column][row];
        }
    }
    result
}

fn multiply3(left: &Matrix3, right: &Matrix3) -> Matrix3 {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            for inner in 0..3 {
                result[row][column] += left[row][inner] * right[inner][column];
            }
        }
    }
    result
}

fn apply3(matrix: &Matrix3, vector: &Vector3) -> Vector3 {
    let mut result = [0.0; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row] += matrix[row][column] * vector[column];
        }
    }
    result
}

fn multiply_jacobian2x3(jacobian: &Jacobian2x3, matrix: &Matrix3) -> Jacobian2x3 {
    let mut result = [[0.0; 3]; 2];
    for row in 0..2 {
        for column in 0..3 {
            for inner in 0..3 {
                result[row][column] += jacobian[row][inner] * matrix[inner][column];
            }
        }
    }
    result
}

fn max_matrix_difference3(left: &Matrix3, right: &Matrix3) -> f64 {
    let mut maximum = 0.0_f64;
    for row in 0..3 {
        for column in 0..3 {
            maximum = maximum.max((left[row][column] - right[row][column]).abs());
        }
    }
    maximum
}

fn max_jacobian_difference(left: &Jacobian2x3, right: &Jacobian2x3) -> f64 {
    let mut maximum = 0.0_f64;
    for row in 0..2 {
        for column in 0..3 {
            maximum = maximum.max((left[row][column] - right[row][column]).abs());
        }
    }
    maximum
}

fn max_vector_difference(left: &Vector3, right: &Vector3) -> f64 {
    (0..3)
        .map(|index| (left[index] - right[index]).abs())
        .fold(0.0_f64, f64::max)
}

fn matrix_inf(matrix: &Matrix3) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0_f64, f64::max)
}

fn inverse3(
    matrix: &Matrix3,
    profile: ReceiverProfileV1,
) -> Result<(Matrix3, f64), ReceiverErrorV1> {
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ReceiverErrorV1::NonFiniteInput);
    }
    let scale = matrix_inf(matrix);
    if !scale.is_finite() || scale == 0.0 {
        return Err(ReceiverErrorV1::SingularCoordinateTransform { pivot: 0 });
    }
    let threshold = profile.relative_cholesky_pivot * scale;
    let mut left = *matrix;
    let mut inverse = [[0.0; 3]; 3];
    for index in 0..3 {
        inverse[index][index] = 1.0;
    }

    for column in 0..3 {
        let pivot_row = (column..3)
            .max_by(|left_row, right_row| {
                left[*left_row][column]
                    .abs()
                    .total_cmp(&left[*right_row][column].abs())
            })
            .ok_or(ReceiverErrorV1::SingularCoordinateTransform { pivot: column })?;
        let pivot_magnitude = left[pivot_row][column].abs();
        if !pivot_magnitude.is_finite() || pivot_magnitude <= threshold {
            return Err(ReceiverErrorV1::SingularCoordinateTransform { pivot: column });
        }
        if pivot_row != column {
            left.swap(pivot_row, column);
            inverse.swap(pivot_row, column);
        }
        let pivot = left[column][column];
        for entry in 0..3 {
            left[column][entry] /= pivot;
            inverse[column][entry] /= pivot;
        }
        for row in 0..3 {
            if row == column {
                continue;
            }
            let factor = left[row][column];
            for entry in 0..3 {
                left[row][entry] -= factor * left[column][entry];
                inverse[row][entry] -= factor * inverse[column][entry];
            }
        }
    }

    if left.iter().flatten().any(|value| !value.is_finite())
        || inverse.iter().flatten().any(|value| !value.is_finite())
    {
        return Err(ReceiverErrorV1::NonFiniteIntermediate);
    }
    let condition = matrix_inf(matrix) * matrix_inf(&inverse);
    if !condition.is_finite() {
        return Err(ReceiverErrorV1::NonFiniteIntermediate);
    }
    Ok((inverse, condition))
}

fn cholesky3(
    matrix: &Matrix3,
    profile: ReceiverProfileV1,
) -> Result<Matrix3, ReceiverErrorV1> {
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ReceiverErrorV1::NonFiniteInput);
    }
    let scale = matrix_inf(matrix);
    if !scale.is_finite() || scale == 0.0 {
        return Err(ReceiverErrorV1::NonPositiveDefiniteMetric { pivot: 0 });
    }
    let mut defect = 0.0_f64;
    for row in 0..3 {
        for column in 0..3 {
            defect = defect.max((matrix[row][column] - matrix[column][row]).abs());
        }
    }
    if defect / scale > profile.symmetry_tolerance {
        return Err(ReceiverErrorV1::AsymmetricMetric);
    }
    let mut lower = [[0.0; 3]; 3];
    let pivot_threshold = profile.relative_cholesky_pivot * scale;
    for row in 0..3 {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for inner in 0..column {
                value -= lower[row][inner] * lower[column][inner];
            }
            if !value.is_finite() {
                return Err(ReceiverErrorV1::NonFiniteIntermediate);
            }
            if row == column {
                if value <= pivot_threshold {
                    return Err(ReceiverErrorV1::NonPositiveDefiniteMetric { pivot: row });
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
            if !lower[row][column].is_finite() {
                return Err(ReceiverErrorV1::NonFiniteIntermediate);
            }
        }
    }
    Ok(lower)
}

fn tangent_metric_norm(
    metric: &Matrix3,
    vector: &Vector3,
    profile: ReceiverProfileV1,
) -> Result<f64, ReceiverErrorV1> {
    let lower = cholesky3(metric, profile)?;
    let mut whitened = [0.0; 3];
    for column in 0..3 {
        for row in 0..3 {
            whitened[column] += lower[row][column] * vector[row];
        }
    }
    if whitened.iter().any(|value| !value.is_finite()) {
        return Err(ReceiverErrorV1::NonFiniteIntermediate);
    }
    let norm = whitened.iter().fold(0.0_f64, |acc, value| acc.hypot(*value));
    if norm.is_finite() {
        Ok(norm)
    } else {
        Err(ReceiverErrorV1::NonFiniteIntermediate)
    }
}

fn cotangent_dual_norm(
    metric: &Matrix3,
    covector: &Vector3,
    profile: ReceiverProfileV1,
) -> Result<f64, ReceiverErrorV1> {
    let lower = cholesky3(metric, profile)?;
    let mut whitened = [0.0; 3];
    for row in 0..3 {
        let mut value = covector[row];
        for column in 0..row {
            value -= lower[row][column] * whitened[column];
        }
        whitened[row] = value / lower[row][row];
        if !whitened[row].is_finite() {
            return Err(ReceiverErrorV1::NonFiniteIntermediate);
        }
    }
    let norm = whitened.iter().fold(0.0_f64, |acc, value| acc.hypot(*value));
    if norm.is_finite() {
        Ok(norm)
    } else {
        Err(ReceiverErrorV1::NonFiniteIntermediate)
    }
}

fn dot(left: &Vector3, right: &Vector3) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn close(defect: f64, scale: f64, profile: ReceiverProfileV1) -> StatusV1 {
    if !defect.is_finite() || !scale.is_finite() {
        return StatusV1::Fail;
    }
    if defect <= profile.absolute_tolerance + profile.relative_tolerance * scale.max(1.0) {
        StatusV1::Pass
    } else {
        StatusV1::Fail
    }
}

fn max_abs_matrix(matrix: &Matrix3) -> f64 {
    matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
}

fn action_metric_defect(
    action: &Matrix3,
    metric: &Matrix3,
    kind: ActionKindV1,
) -> Matrix3 {
    let action_t = transpose3(action);
    let left = multiply3(&action_t, metric);
    match kind {
        ActionKindV1::InfinitesimalGenerator => {
            let right = multiply3(metric, action);
            let mut result = [[0.0; 3]; 3];
            for row in 0..3 {
                for column in 0..3 {
                    result[row][column] = left[row][column] + right[row][column];
                }
            }
            result
        }
        ActionKindV1::FiniteTransformation => {
            let right = multiply3(&left, action);
            let mut result = right;
            for row in 0..3 {
                for column in 0..3 {
                    result[row][column] -= metric[row][column];
                }
            }
            result
        }
    }
}

fn validate_source_identity(receipt: &SourceMathReceiptV1) -> Result<(), ReceiverErrorV1> {
    if receipt.repository != "Luminous-Dynamics/symthaea"
        || receipt.commit_sha.len() != 40
        || !receipt.commit_sha.bytes().all(|byte| byte.is_ascii_hexdigit())
        || receipt.receipt_id.trim().is_empty()
        || receipt.checker_id.trim().is_empty()
        || receipt.checker_version.trim().is_empty()
        || receipt.profile_id.trim().is_empty()
        || receipt.action_id.trim().is_empty()
        || receipt.coordinate_basis_id.trim().is_empty()
        || receipt.metric_id.trim().is_empty()
    {
        return Err(ReceiverErrorV1::InvalidSourceIdentity);
    }
    Ok(())
}

fn encode_fixture_component_digests(
    fixture: &PhysicalFixtureV1,
    profile: ReceiverProfileV1,
) -> Result<DigestBindingsV1, ReceiverErrorV1> {
    let source_placeholder = [0_u8; 32];
    let generator_placeholder = [0_u8; 32];

    let physical_fixture_sha256 = sha256(&encode_fixture(fixture, profile)?);

    let mut transform_encoder = EncoderV1::new(profile.maximum_encoded_bytes);
    transform_encoder.put(b"LUMINOUS-PHYS-RECEIVER\0TRANSFORM\0V1")?;
    encode_matrix3_into(&mut transform_encoder, &fixture.transform)?;
    let transform_sha256 = sha256(&transform_encoder.finish());

    let mut mass_encoder = EncoderV1::new(profile.maximum_encoded_bytes);
    mass_encoder.put(b"LUMINOUS-PHYS-RECEIVER\0MASS-Q\0V1")?;
    encode_matrix3_into(&mut mass_encoder, &fixture.mass_q)?;
    let mass_metric_sha256 = sha256(&mass_encoder.finish());

    let mut transformed_mass_encoder = EncoderV1::new(profile.maximum_encoded_bytes);
    transformed_mass_encoder.put(b"LUMINOUS-PHYS-RECEIVER\0MASS-Y\0V1")?;
    encode_matrix3_into(&mut transformed_mass_encoder, &fixture.mass_y_claimed)?;
    let transformed_mass_metric_sha256 = sha256(&transformed_mass_encoder.finish());

    let mut force_encoder = EncoderV1::new(profile.maximum_encoded_bytes);
    force_encoder.put(b"LUMINOUS-PHYS-RECEIVER\0FORCE-COVECTORS\0V1")?;
    encode_vector3_into(&mut force_encoder, &fixture.force_q)?;
    encode_vector3_into(&mut force_encoder, &fixture.force_y_claimed)?;
    let force_covectors_sha256 = sha256(&force_encoder.finish());

    let mut constraint_encoder = EncoderV1::new(profile.maximum_encoded_bytes);
    constraint_encoder.put(b"LUMINOUS-PHYS-RECEIVER\0CONSTRAINT-MAP\0V1")?;
    encode_jacobian_into(&mut constraint_encoder, &fixture.constraint_jacobian_q)?;
    encode_jacobian_into(&mut constraint_encoder, &fixture.constraint_jacobian_y_claimed)?;
    let constraint_map_sha256 = sha256(&constraint_encoder.finish());

    let mut initial_encoder = EncoderV1::new(profile.maximum_encoded_bytes);
    initial_encoder.put(b"LUMINOUS-PHYS-RECEIVER\0INITIAL-STATE\0V1")?;
    encode_vector3_into(&mut initial_encoder, &fixture.tangent_q)?;
    encode_vector3_into(&mut initial_encoder, &fixture.tangent_y_claimed)?;
    let initial_state_sha256 = sha256(&initial_encoder.finish());

    Ok(DigestBindingsV1 {
        source_receipt_sha256: source_placeholder,
        generator_set_sha256: generator_placeholder,
        physical_fixture_sha256,
        transform_sha256,
        mass_metric_sha256,
        transformed_mass_metric_sha256,
        force_covectors_sha256,
        constraint_map_sha256,
        initial_state_sha256,
        task_semantics_sha256: [0_u8; 32],
    })
}

fn encode_task_digest(
    task: &TaskSemanticsV1,
    profile: ReceiverProfileV1,
) -> Result<[u8; 32], ReceiverErrorV1> {
    Ok(sha256(&encode_task_semantics(task, profile)?))
}

fn encode_generator_digest(
    receipt: &SourceMathReceiptV1,
    fixture: &PhysicalFixtureV1,
    profile: ReceiverProfileV1,
) -> Result<[u8; 32], ReceiverErrorV1> {
    Ok(sha256(&encode_generators(receipt, fixture, profile)?))
}

fn expected_digests(
    input: &ReceiverInputV1,
    profile: ReceiverProfileV1,
) -> Result<DigestBindingsV1, ReceiverErrorV1> {
    let receipt_bytes = encode_source_receipt(&input.source_receipt, profile)?;
    let mut digests = encode_fixture_component_digests(&input.physical_fixture, profile)?;
    digests.source_receipt_sha256 = sha256(&receipt_bytes);
    digests.generator_set_sha256 =
        encode_generator_digest(&input.source_receipt, &input.physical_fixture, profile)?;
    digests.task_semantics_sha256 = encode_task_digest(&input.task_before, profile)?;
    Ok(digests)
}

/// Build the digest binding expected by a receiver envelope adapter.
///
/// The adapter must copy these digests into the handoff envelope. Verification
/// recomputes them again; it never trusts this helper's output as a receipt.
pub fn compute_digest_bindings_v1(
    input: &ReceiverInputV1,
    profile: ReceiverProfileV1,
) -> Result<DigestBindingsV1, ReceiverErrorV1> {
    validate_profile(profile)?;
    validate_source_identity(&input.source_receipt)?;
    expected_digests(input, profile)
}

fn defect(left: f64, right: f64) -> DefectV1 {
    let absolute = (left - right).abs();
    DefectV1 {
        absolute,
        relative: absolute / left.abs().max(right.abs()).max(1.0),
    }
}

fn matrix_defect(left: &Matrix3, right: &Matrix3) -> DefectV1 {
    let absolute = max_matrix_difference3(left, right);
    let scale = max_abs_matrix(left).max(max_abs_matrix(right)).max(1.0);
    DefectV1 {
        absolute,
        relative: absolute / scale,
    }
}

fn vector_defect(left: &Vector3, right: &Vector3) -> DefectV1 {
    let absolute = max_vector_difference(left, right);
    let scale = left
        .iter()
        .chain(right.iter())
        .map(|value| value.abs())
        .fold(1.0_f64, f64::max);
    DefectV1 {
        absolute,
        relative: absolute / scale,
    }
}

fn jacobian_defect(left: &Jacobian2x3, right: &Jacobian2x3) -> DefectV1 {
    let absolute = max_jacobian_difference(left, right);
    let scale = left
        .iter()
        .chain(right.iter())
        .flatten()
        .map(|value| value.abs())
        .fold(1.0_f64, f64::max);
    DefectV1 {
        absolute,
        relative: absolute / scale,
    }
}

fn check_defect(value: DefectV1, profile: ReceiverProfileV1) -> StatusV1 {
    close(value.absolute, 1.0, profile)
}

fn aggregate_status(obligations: &ReceiverObligationsV1) -> StatusV1 {
    let statuses = [
        obligations.content_binding,
        obligations.mass_metric_congruence,
        obligations.force_covector_duality,
        obligations.momentum_one_form,
        obligations.constraint_jacobian,
        obligations.action_conjugation,
        obligations.generator_metric_isometry,
        obligations.tangent_norm,
        obligations.cotangent_dual_norm,
        obligations.canonical_pairing,
        obligations.full_task_semantics,
        obligations.discrete_step_equivariance,
        obligations.physical_observables,
        obligations.numerical_error_envelope,
    ];
    if statuses.contains(&StatusV1::Fail) {
        StatusV1::Fail
    } else if statuses.contains(&StatusV1::Inconclusive) {
        StatusV1::Inconclusive
    } else if statuses.contains(&StatusV1::NotApplicable) {
        StatusV1::Inconclusive
    } else {
        StatusV1::Pass
    }
}

fn encode_receipt_for_hash(
    receipt: &ReceiverReceiptV1,
    profile: ReceiverProfileV1,
) -> Result<Vec<u8>, ReceiverErrorV1> {
    let mut out = EncoderV1::new(profile.maximum_encoded_bytes);
    out.put(RECEIPT_DOMAIN)?;
    out.u32(receipt.schema_version)?;
    out.string(&receipt.handoff_id)?;
    out.u8(status_tag(receipt.receiver_status))?;
    for status in [
        receipt.obligations.content_binding,
        receipt.obligations.mass_metric_congruence,
        receipt.obligations.force_covector_duality,
        receipt.obligations.momentum_one_form,
        receipt.obligations.constraint_jacobian,
        receipt.obligations.action_conjugation,
        receipt.obligations.generator_metric_isometry,
        receipt.obligations.tangent_norm,
        receipt.obligations.cotangent_dual_norm,
        receipt.obligations.canonical_pairing,
        receipt.obligations.full_task_semantics,
        receipt.obligations.discrete_step_equivariance,
        receipt.obligations.physical_observables,
        receipt.obligations.numerical_error_envelope,
    ] {
        out.u8(status_tag(status))?;
    }
    for value in [
        receipt.defects.mass_metric_congruence.absolute,
        receipt.defects.mass_metric_congruence.relative,
        receipt.defects.force_covector_duality.absolute,
        receipt.defects.force_covector_duality.relative,
        receipt.defects.momentum_one_form.absolute,
        receipt.defects.momentum_one_form.relative,
        receipt.defects.constraint_jacobian.absolute,
        receipt.defects.constraint_jacobian.relative,
        receipt.defects.action_conjugation_max,
        receipt.defects.action_metric_max,
        receipt.defects.tangent_norm.absolute,
        receipt.defects.tangent_norm.relative,
        receipt.defects.cotangent_dual_norm.absolute,
        receipt.defects.cotangent_dual_norm.relative,
        receipt.defects.canonical_pairing.absolute,
        receipt.defects.canonical_pairing.relative,
        receipt.defects.coordinate_condition_inf,
    ] {
        out.f64(value)?;
    }
    out.string(receipt.physical_equivalence_authority)?;
    out.u8(u8::from(receipt.scientific_search_pruning_authorized))?;
    Ok(out.finish())
}

fn status_tag(status: StatusV1) -> u8 {
    match status {
        StatusV1::Pass => 1,
        StatusV1::Fail => 2,
        StatusV1::Inconclusive => 3,
        StatusV1::NotApplicable => 4,
    }
}

/// Independently verify a typed handoff payload and emit a receiver-owned receipt.
///
/// The source receipt and generator set are canonically re-encoded from the
/// exact source fields carried here. The physical fixture and task semantics
/// are likewise content-addressed. A schema adapter may populate this typed
/// structure from the versioned JSON envelope, but must not rewrite the bytes
/// after computing the upstream digests.
pub fn verify_receiver_v1(
    input: &ReceiverInputV1,
    profile: ReceiverProfileV1,
) -> Result<ReceiverReceiptV1, ReceiverErrorV1> {
    validate_profile(profile)?;
    validate_source_identity(&input.source_receipt)?;

    let computed = expected_digests(input, profile)?;
    if computed.source_receipt_sha256 != input.digests.source_receipt_sha256 {
        return Err(ReceiverErrorV1::SourceReceiptDigestMismatch);
    }
    if computed.generator_set_sha256 != input.digests.generator_set_sha256 {
        return Err(ReceiverErrorV1::GeneratorSetDigestMismatch);
    }
    if computed.physical_fixture_sha256 != input.digests.physical_fixture_sha256 {
        return Err(ReceiverErrorV1::FixtureDigestMismatch);
    }
    if computed.transform_sha256 != input.digests.transform_sha256 {
        return Err(ReceiverErrorV1::TransformDigestMismatch);
    }
    if computed.mass_metric_sha256 != input.digests.mass_metric_sha256 {
        return Err(ReceiverErrorV1::MassMetricDigestMismatch);
    }
    if computed.transformed_mass_metric_sha256 != input.digests.transformed_mass_metric_sha256 {
        return Err(ReceiverErrorV1::TransformedMassMetricDigestMismatch);
    }
    if computed.force_covectors_sha256 != input.digests.force_covectors_sha256 {
        return Err(ReceiverErrorV1::ForceCovectorDigestMismatch);
    }
    if computed.constraint_map_sha256 != input.digests.constraint_map_sha256 {
        return Err(ReceiverErrorV1::ConstraintMapDigestMismatch);
    }
    if computed.initial_state_sha256 != input.digests.initial_state_sha256 {
        return Err(ReceiverErrorV1::InitialStateDigestMismatch);
    }
    if computed.task_semantics_sha256 != input.digests.task_semantics_sha256
        || input.task_before != input.task_after
    {
        return Err(ReceiverErrorV1::TaskSemanticsDigestMismatch);
    }

    if input.physical_fixture.symmetry_matrices_q.is_empty()
        || input.physical_fixture.symmetry_matrices_q.len()
            != input.physical_fixture.symmetry_matrices_y_claimed.len()
    {
        return Err(ReceiverErrorV1::EmptyGeneratorSet);
    }

    let fixture = &input.physical_fixture;
    let (inverse_s, condition) = inverse3(&fixture.transform, profile)?;
    let mass_from_y = multiply3(
        &multiply3(&transpose3(&fixture.transform), &fixture.mass_q),
        &fixture.transform,
    );
    let force_from_q = apply3(&transpose3(&fixture.transform), &fixture.force_q);
    let momentum_from_q = apply3(&transpose3(&fixture.transform), &fixture.momentum_q);
    let jacobian_from_q = multiply_jacobian2x3(&fixture.constraint_jacobian_q, &fixture.transform);
    let tangent_from_q = apply3(&inverse_s, &fixture.tangent_q);

    let mass_defect = matrix_defect(&mass_from_y, &fixture.mass_y_claimed);
    let force_defect = vector_defect(&force_from_q, &fixture.force_y_claimed);
    let momentum_defect = vector_defect(&momentum_from_q, &fixture.momentum_y_claimed);
    let jacobian_defect = jacobian_defect(&jacobian_from_q, &fixture.constraint_jacobian_y_claimed);
    let tangent_defect = vector_defect(&tangent_from_q, &fixture.tangent_y_claimed);

    let force_pair_q = dot(&fixture.force_q, &fixture.tangent_q);
    let force_pair_y = dot(&fixture.force_y_claimed, &fixture.tangent_y_claimed);
    let pairing_defect = defect(force_pair_q, force_pair_y);

    // A covector dual norm must be measured with M^-1, not with M.
    let tangent_norm_q = tangent_metric_norm(&fixture.mass_q, &fixture.tangent_q, profile)?;
    let tangent_norm_y =
        tangent_metric_norm(&fixture.mass_y_claimed, &fixture.tangent_y_claimed, profile)?;
    let cotangent_norm_q = cotangent_dual_norm(&fixture.mass_q, &fixture.force_q, profile)?;
    let cotangent_norm_y =
        cotangent_dual_norm(&fixture.mass_y_claimed, &fixture.force_y_claimed, profile)?;
    let tangent_norm_defect = defect(tangent_norm_q, tangent_norm_y);
    let cotangent_norm_defect = defect(cotangent_norm_q, cotangent_norm_y);

    let mut action_conjugation_max = 0.0_f64;
    let mut action_metric_max = 0.0_f64;
    for (index, action_q) in fixture.symmetry_matrices_q.iter().enumerate() {
        let action_y = multiply3(&multiply3(&inverse_s, action_q), &fixture.transform);
        let claimed_action_y = &fixture.symmetry_matrices_y_claimed[index];
        action_conjugation_max =
            action_conjugation_max.max(max_matrix_difference3(&action_y, claimed_action_y));
        let defect_q = action_metric_defect(action_q, &fixture.mass_q, fixture.action_kind);
        let defect_y = action_metric_defect(claimed_action_y, &fixture.mass_y_claimed, fixture.action_kind);
        action_metric_max = action_metric_max.max(max_abs_matrix(&defect_q));
        action_metric_max = action_metric_max.max(max_abs_matrix(&defect_y));
    }

    let profile_admitted = condition <= profile.maximum_coordinate_condition_inf;
    let conditional = |status: StatusV1| {
        if !profile_admitted && status == StatusV1::Pass {
            StatusV1::Inconclusive
        } else {
            status
        }
    };

    let mass_status = conditional(check_defect(mass_defect, profile));
    let force_status = conditional(check_defect(force_defect, profile));
    let momentum_status = conditional(check_defect(momentum_defect, profile));
    let jacobian_status = conditional(check_defect(jacobian_defect, profile));
    let action_conjugation_status =
        conditional(close(action_conjugation_max, 1.0, profile));
    let action_metric_status = if input.source_receipt.qualification_status != SourceStatusV1::Pass {
        StatusV1::Inconclusive
    } else {
        conditional(close(action_metric_max, 1.0, profile))
    };
    let tangent_norm_status = conditional(check_defect(tangent_norm_defect, profile));
    let cotangent_norm_status = conditional(check_defect(cotangent_norm_defect, profile));
    let pairing_status = conditional(check_defect(pairing_defect, profile));
    let task_status = if input.task_before == input.task_after {
        StatusV1::Pass
    } else {
        StatusV1::Fail
    };

    // V0 does not possess an independent physical step-map receipt, a
    // measured observable mapping, or a rigorously enclosed forward error.
    let obligations = ReceiverObligationsV1 {
        content_binding: StatusV1::Pass,
        mass_metric_congruence: mass_status,
        force_covector_duality: force_status,
        momentum_one_form: momentum_status,
        constraint_jacobian: jacobian_status,
        action_conjugation: action_conjugation_status,
        generator_metric_isometry: action_metric_status,
        tangent_norm: tangent_norm_status,
        cotangent_dual_norm: cotangent_norm_status,
        canonical_pairing: pairing_status,
        full_task_semantics: task_status,
        discrete_step_equivariance: StatusV1::Inconclusive,
        physical_observables: StatusV1::Inconclusive,
        numerical_error_envelope: StatusV1::Inconclusive,
    };

    let mut receipt = ReceiverReceiptV1 {
        schema_version: 1,
        handoff_id: input.handoff_id.clone(),
        receiver_status: aggregate_status(&obligations),
        obligations,
        defects: ReceiverDefectsV1 {
            mass_metric_congruence: mass_defect,
            force_covector_duality: force_defect,
            momentum_one_form: momentum_defect,
            constraint_jacobian: jacobian_defect,
            action_conjugation_max,
            action_metric_max,
            tangent_norm: tangent_norm_defect,
            cotangent_dual_norm: cotangent_norm_defect,
            canonical_pairing: pairing_defect,
            coordinate_condition_inf: condition,
        },
        physical_equivalence_authority: "NONE",
        scientific_search_pruning_authorized: false,
        receipt_sha256: [0_u8; 32],
    };
    receipt.receipt_sha256 = sha256(&encode_receipt_for_hash(&receipt, profile)?);
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest_byte(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn task() -> TaskSemanticsV1 {
        TaskSemanticsV1 {
            task_sha256: digest_byte(1),
            candidate_domain_sha256: digest_byte(2),
            objective_sha256: digest_byte(3),
            feasibility_constraints_sha256: digest_byte(4),
            initial_and_boundary_conditions_sha256: digest_byte(5),
            observable_mapping_sha256: digest_byte(6),
            answer_interpretation_sha256: digest_byte(7),
        }
    }

    fn fixture() -> ReceiverInputV1 {
        let transform = [[1.0, 2.0, -1.0], [0.0, 1.0, 3.0], [0.0, 0.0, 1.0]];
        let mass_q = [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]];
        let force_q = [1.25, -2.0, 0.75];
        let momentum_q = [-0.5, 1.0, 2.5];
        let constraint_jacobian_q = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        let tangent_q = [0.5, -0.25, 1.0];

        let inverse_s = inverse3(&transform, ReceiverProfileV1::default()).unwrap().0;
        let mass_y_claimed = multiply3(
            &multiply3(&transpose3(&transform), &mass_q),
            &transform,
        );
        let force_y_claimed = apply3(&transpose3(&transform), &force_q);
        let momentum_y_claimed = apply3(&transpose3(&transform), &momentum_q);
        let constraint_jacobian_y_claimed =
            multiply_jacobian2x3(&constraint_jacobian_q, &transform);
        let tangent_y_claimed = apply3(&inverse_s, &tangent_q);

        let action_q = [
            [0.0, -0.5, 0.0],
            [0.5, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ];
        let action_y = multiply3(&multiply3(&inverse_s, &action_q), &transform);

        let mut input = ReceiverInputV1 {
            schema_version: 1,
            handoff_id: "receiver-dense-coordinate-fixture-v1".to_owned(),
            source_receipt: SourceMathReceiptV1 {
                repository: "Luminous-Dynamics/symthaea".to_owned(),
                commit_sha: "1".repeat(40),
                receipt_id: "numeric-lie-receipt-v1".to_owned(),
                checker_id: "symthaea-linear-lie-algebra".to_owned(),
                checker_version: "0.1".to_owned(),
                profile_id: "metric-whitened-lie-profile-v1".to_owned(),
                action_id: "so2-z-generator-v1".to_owned(),
                coordinate_basis_id: "canonical-q".to_owned(),
                metric_id: "mass-q-v1".to_owned(),
                evidence_class: EvidenceClassV1::NumericLieBasis,
                qualification_status: SourceStatusV1::Pass,
            },
            physical_fixture: PhysicalFixtureV1 {
                action_kind: ActionKindV1::InfinitesimalGenerator,
                transform,
                mass_q,
                mass_y_claimed,
                force_q,
                force_y_claimed,
                momentum_q,
                momentum_y_claimed,
                constraint_jacobian_q,
                constraint_jacobian_y_claimed,
                tangent_q,
                tangent_y_claimed,
                symmetry_matrices_q: vec![action_q],
                symmetry_matrices_y_claimed: vec![action_y],
            },
            task_before: task(),
            task_after: task(),
            digests: DigestBindingsV1 {
                source_receipt_sha256: [0; 32],
                generator_set_sha256: [0; 32],
                physical_fixture_sha256: [0; 32],
                transform_sha256: [0; 32],
                mass_metric_sha256: [0; 32],
                transformed_mass_metric_sha256: [0; 32],
                force_covectors_sha256: [0; 32],
                constraint_map_sha256: [0; 32],
                initial_state_sha256: [0; 32],
                task_semantics_sha256: [0; 32],
            },
        };
        input.digests = compute_digest_bindings_v1(&input, ReceiverProfileV1::default()).unwrap();
        input
    }

    #[test]
    fn dense_coordinate_receiver_checks_tensor_laws_and_remains_inconclusive_for_step_map() {
        let receipt = verify_receiver_v1(&fixture(), ReceiverProfileV1::default()).unwrap();
        assert_eq!(receipt.obligations.mass_metric_congruence, StatusV1::Pass);
        assert_eq!(receipt.obligations.force_covector_duality, StatusV1::Pass);
        assert_eq!(receipt.obligations.momentum_one_form, StatusV1::Pass);
        assert_eq!(receipt.obligations.constraint_jacobian, StatusV1::Pass);
        assert_eq!(receipt.obligations.action_conjugation, StatusV1::Pass);
        assert_eq!(receipt.obligations.generator_metric_isometry, StatusV1::Pass);
        assert_eq!(receipt.obligations.tangent_norm, StatusV1::Pass);
        assert_eq!(receipt.obligations.cotangent_dual_norm, StatusV1::Pass);
        assert_eq!(receipt.obligations.canonical_pairing, StatusV1::Pass);
        assert_eq!(receipt.obligations.full_task_semantics, StatusV1::Pass);
        assert_eq!(receipt.obligations.discrete_step_equivariance, StatusV1::Inconclusive);
        assert_eq!(receipt.obligations.physical_observables, StatusV1::Inconclusive);
        assert_eq!(receipt.receiver_status, StatusV1::Inconclusive);
        assert_eq!(receipt.physical_equivalence_authority, "NONE");
        assert!(!receipt.scientific_search_pruning_authorized);
    }

    #[test]
    fn changed_force_variance_is_rejected_as_physical_failure() {
        let mut input = fixture();
        input.physical_fixture.force_y_claimed = input.physical_fixture.force_q;
        input.digests = compute_digest_bindings_v1(&input, ReceiverProfileV1::default()).unwrap();
        let receipt = verify_receiver_v1(&input, ReceiverProfileV1::default()).unwrap();
        assert_eq!(receipt.obligations.force_covector_duality, StatusV1::Fail);
        assert_eq!(receipt.receiver_status, StatusV1::Fail);
    }

    #[test]
    fn changed_jacobian_variance_is_rejected() {
        let mut input = fixture();
        input.physical_fixture.constraint_jacobian_y_claimed = input.physical_fixture.constraint_jacobian_q;
        input.digests = compute_digest_bindings_v1(&input, ReceiverProfileV1::default()).unwrap();
        let receipt = verify_receiver_v1(&input, ReceiverProfileV1::default()).unwrap();
        assert_eq!(receipt.obligations.constraint_jacobian, StatusV1::Fail);
    }

    #[test]
    fn nonisometric_generator_fails_receiver_qualification() {
        let mut input = fixture();
        input.physical_fixture.symmetry_matrices_q[0][0][0] = 0.25;
        input.physical_fixture.symmetry_matrices_y_claimed[0][0][0] = 0.25;
        input.digests = compute_digest_bindings_v1(&input, ReceiverProfileV1::default()).unwrap();
        let receipt = verify_receiver_v1(&input, ReceiverProfileV1::default()).unwrap();
        assert_eq!(receipt.obligations.generator_metric_isometry, StatusV1::Fail);
    }

    #[test]
    fn changed_task_semantics_fails_closed_before_geometry_is_promoted() {
        let mut input = fixture();
        input.task_after.objective_sha256 = digest_byte(99);
        input.digests = compute_digest_bindings_v1(&input, ReceiverProfileV1::default()).unwrap();
        assert_eq!(
            verify_receiver_v1(&input, ReceiverProfileV1::default()),
            Err(ReceiverErrorV1::TaskSemanticsDigestMismatch)
        );
    }

    #[test]
    fn stale_fixture_digest_is_rejected() {
        let mut input = fixture();
        input.digests.physical_fixture_sha256 = digest_byte(42);
        assert_eq!(
            verify_receiver_v1(&input, ReceiverProfileV1::default()),
            Err(ReceiverErrorV1::FixtureDigestMismatch)
        );
    }

    #[test]
    fn ill_conditioned_rebasis_returns_inconclusive_for_coordinate_laws() {
        let mut input = fixture();
        input.physical_fixture.transform =
            [[1.0, 1.0e6, 0.0], [0.0, 1.0, 1.0e6], [0.0, 0.0, 1.0]];
        input.physical_fixture.mass_y_claimed = multiply3(
            &multiply3(
                &transpose3(&input.physical_fixture.transform),
                &input.physical_fixture.mass_q,
            ),
            &input.physical_fixture.transform,
        );
        let inverse =
            inverse3(&input.physical_fixture.transform, ReceiverProfileV1::default()).unwrap().0;
        input.physical_fixture.force_y_claimed =
            apply3(&transpose3(&input.physical_fixture.transform), &input.physical_fixture.force_q);
        input.physical_fixture.momentum_y_claimed =
            apply3(&transpose3(&input.physical_fixture.transform), &input.physical_fixture.momentum_q);
        input.physical_fixture.constraint_jacobian_y_claimed = multiply_jacobian2x3(
            &input.physical_fixture.constraint_jacobian_q,
            &input.physical_fixture.transform,
        );
        input.physical_fixture.tangent_y_claimed =
            apply3(&inverse, &input.physical_fixture.tangent_q);
        input.physical_fixture.symmetry_matrices_y_claimed[0] = multiply3(
            &multiply3(&inverse, &input.physical_fixture.symmetry_matrices_q[0]),
            &input.physical_fixture.transform,
        );
        input.digests = compute_digest_bindings_v1(&input, ReceiverProfileV1::default()).unwrap();
        let profile = ReceiverProfileV1 {
            maximum_coordinate_condition_inf: 1.0e4,
            ..ReceiverProfileV1::default()
        };
        let receipt = verify_receiver_v1(&input, profile).unwrap();
        assert_eq!(receipt.obligations.mass_metric_congruence, StatusV1::Inconclusive);
        assert_eq!(receipt.obligations.force_covector_duality, StatusV1::Inconclusive);
        assert_eq!(receipt.obligations.discrete_step_equivariance, StatusV1::Inconclusive);
    }

    #[test]
    fn singular_coordinate_map_is_rejected() {
        let mut input = fixture();
        input.physical_fixture.transform =
            [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 0.0, 1.0]];
        input.digests = compute_digest_bindings_v1(&input, ReceiverProfileV1::default()).unwrap();
        assert!(matches!(
            verify_receiver_v1(&input, ReceiverProfileV1::default()),
            Err(ReceiverErrorV1::SingularCoordinateTransform { .. })
        ));
    }

    #[test]
    fn nonfinite_physical_data_is_rejected_by_canonical_encoder() {
        let mut input = fixture();
        input.physical_fixture.force_q[0] = f64::NAN;
        assert_eq!(
            compute_digest_bindings_v1(&input, ReceiverProfileV1::default()),
            Err(ReceiverErrorV1::NonFiniteInput)
        );
    }

    #[test]
    fn receipt_is_deterministic_for_frozen_input_and_profile() {
        let input = fixture();
        let first = verify_receiver_v1(&input, ReceiverProfileV1::default()).unwrap();
        let second = verify_receiver_v1(&input, ReceiverProfileV1::default()).unwrap();
        assert_eq!(first, second);
        assert!(first.receipt_sha256.iter().any(|byte| *byte != 0));
    }
}
