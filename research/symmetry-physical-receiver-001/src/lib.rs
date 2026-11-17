// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Independent bounded receiver for mathematical-to-physical symmetry handoffs.
//!
//! This verifier checks exact source-artifact bytes and recomputes a frozen 3D
//! coordinate re-description. It never treats the source's PASS as a physical
//! PASS. Discrete step-map equivariance, physical observables, and numerical
//! forward-error qualification remain INCONCLUSIVE until separately evidenced.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;

pub type Vector3 = [f64; 3];
pub type Matrix3 = [[f64; 3]; 3];
pub type Jacobian2x3 = [[f64; 3]; 2];

pub const RECEIVER_SCHEMA_VERSION: &str = "luminous.symmetry_physical_receiver.v0.1";
pub const RECEIVER_PROFILE_ID: &str = "symtropy-dense-3d-receiver-profile-v0.1";
pub const RECEIVER_ID: &str = "symtropy-independent-physical-handoff-verifier-v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiverProfileV1 {
    pub relative_transform_pivot_tolerance: f64,
    pub maximum_transform_condition_inf: f64,
    pub maximum_relative_law_defect: f64,
    pub maximum_metric_symmetry_defect: f64,
}

impl Default for ReceiverProfileV1 {
    fn default() -> Self {
        Self {
            relative_transform_pivot_tolerance: 1.0e-12,
            maximum_transform_condition_inf: 1.0e8,
            maximum_relative_law_defect: 5.0e-10,
            maximum_metric_symmetry_defect: 1.0e-12,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiverErrorV1 {
    InvalidProfile,
    WrongPackageSchema,
    WrongEnvelopeSchema,
    NotAdmissibleFixture,
    SourceRepositoryMismatch,
    SourceCommitMismatch,
    SourceReceiptDigestMismatch,
    SourceReceiptMalformed,
    SourceReceiptIdentityMismatch,
    UnsupportedEvidenceClass,
    GeneratorDigestMismatch,
    PhysicalFixtureDigestMismatch,
    MassMetricDigestMismatch,
    ForceCovectorDigestMismatch,
    ConstraintMapDigestMismatch,
    InitialStateDigestMismatch,
    TaskSemanticDigestMismatch,
    ClaimAlreadyPromoted,
    InvalidIdentifier,
    NonFiniteInput,
    SingularCoordinateChange {
        pivot: usize,
    },
    CoordinateConditionOutOfEnvelope {
        estimate_bits: u64,
    },
    NonSymmetricMetric {
        defect_bits: u64,
    },
    NonPositiveDefiniteMetric {
        pivot: usize,
    },
    NonFiniteDerivedValue,
    WrongMetricCongruence {
        defect_bits: u64,
    },
    WrongForceCovectorLaw {
        defect_bits: u64,
    },
    WrongMomentumCovectorLaw {
        defect_bits: u64,
    },
    WrongConstraintJacobianLaw {
        defect_bits: u64,
    },
    WrongGeneratorConjugation {
        defect_bits: u64,
    },
    NonIsometricGenerator {
        generator_index: usize,
        defect_bits: u64,
    },
    PairingNotPreserved {
        defect_bits: u64,
    },
    TangentNormNotPreserved {
        defect_bits: u64,
    },
    CotangentNormNotPreserved {
        defect_bits: u64,
    },
    ReceiptSerializationFailure,
}

impl fmt::Display for ReceiverErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ReceiverErrorV1 {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMathReceiptRefV1 {
    pub repository: String,
    pub commit_sha: String,
    pub receipt_id: String,
    pub receipt_sha256: String,
    pub checker_id: String,
    pub checker_version: String,
    pub profile_id: String,
    pub action_id: String,
    pub coordinate_basis_id: String,
    pub metric_id: String,
    pub generator_set_sha256: String,
    pub evidence_class: String,
    pub qualification_status: String,
    pub group_order: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinateChangeRefV1 {
    pub kind: String,
    pub from_basis_id: String,
    pub to_basis_id: String,
    pub transform_sha256: String,
    pub transformed_mass_metric_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalProblemRefV1 {
    pub case_id: String,
    pub fixture_sha256: String,
    pub coordinate_change: CoordinateChangeRefV1,
    pub mass_metric_sha256: String,
    pub force_covectors_sha256: String,
    pub constraint_map_sha256: String,
    pub initial_state_sha256: String,
    pub solver_profile_id: String,
    pub integration_method: String,
    pub time_step_seconds: f64,
    pub units_profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSemanticsRefV1 {
    pub task_sha256: String,
    pub candidate_domain_sha256: String,
    pub objective_sha256: String,
    pub feasibility_constraints_sha256: String,
    pub initial_and_boundary_conditions_sha256: String,
    pub observable_mapping_sha256: String,
    pub answer_interpretation_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimsRefV1 {
    pub physical_step_equivariance_status: String,
    pub physical_equivalence_authority: String,
    pub scientific_search_pruning_authorized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffEnvelopeV1 {
    pub schema_version: String,
    pub fixture_mode: String,
    pub handoff_id: String,
    pub source_math_receipt: SourceMathReceiptRefV1,
    pub physical_problem: PhysicalProblemRefV1,
    pub full_task_semantics: TaskSemanticsRefV1,
    /// Upstream content is retained for schema compatibility but is never trusted.
    pub receiver_verification: Value,
    pub claims: ClaimsRefV1,
    pub claim_boundary: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalFixtureV1 {
    pub transform_s: Matrix3,
    pub mass_metric_q: Matrix3,
    pub force_covector_q: Vector3,
    pub momentum_covector_q: Vector3,
    pub constraint_jacobian_q: Jacobian2x3,
    pub initial_position_q: Vector3,
    pub initial_velocity_q: Vector3,
    pub declared_mass_metric_y: Matrix3,
    pub declared_force_covector_y: Vector3,
    pub declared_momentum_covector_y: Vector3,
    pub declared_constraint_jacobian_y: Jacobian2x3,
    pub declared_generators_y: Vec<Matrix3>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSemanticsArtifactsV1 {
    pub candidate_domain: String,
    pub objective: String,
    pub feasibility_constraints: String,
    pub initial_and_boundary_conditions: String,
    pub observable_mapping: String,
    pub answer_interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiverPackageV1 {
    pub schema_version: String,
    pub envelope: HandoffEnvelopeV1,
    /// Hex encoding of the exact source-receipt bytes; digest is over decoded bytes.
    pub source_receipt_bytes_hex: String,
    /// Generator matrices are canonically encoded by this verifier, independent of JSON whitespace.
    pub generator_set: Vec<Matrix3>,
    pub physical_fixture: PhysicalFixtureV1,
    pub task_semantics: TaskSemanticsArtifactsV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObligationStatusV1 {
    Pass,
    Fail,
    Inconclusive,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiverObligationsV1 {
    pub mass_metric_congruence: ObligationStatusV1,
    pub force_covector_duality: ObligationStatusV1,
    pub momentum_one_form: ObligationStatusV1,
    pub constraint_jacobian: ObligationStatusV1,
    pub generator_metric_isometry: ObligationStatusV1,
    pub discrete_step_equivariance: ObligationStatusV1,
    pub physical_observables: ObligationStatusV1,
    pub numerical_error_envelope: ObligationStatusV1,
    pub full_task_semantics: ObligationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReceiverDiagnosticsV1 {
    pub transform_condition_inf_estimate: f64,
    pub mass_metric_relative_symmetry_defect: f64,
    pub mass_congruence_relative_defect: f64,
    pub force_covector_relative_defect: f64,
    pub momentum_covector_relative_defect: f64,
    pub constraint_jacobian_relative_defect: f64,
    pub maximum_generator_conjugation_relative_defect: f64,
    pub maximum_generator_isometry_relative_defect_q: f64,
    pub maximum_generator_isometry_relative_defect_y: f64,
    pub force_pairing_relative_defect: f64,
    pub momentum_pairing_relative_defect: f64,
    pub tangent_norm_relative_defect: f64,
    pub force_dual_norm_relative_defect: f64,
    pub momentum_dual_norm_relative_defect: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReceiverReceiptPayloadV1 {
    pub schema_version: String,
    pub verifier_id: String,
    pub verifier_profile_id: String,
    pub handoff_id: String,
    pub source_repository: String,
    pub source_commit_sha: String,
    pub source_receipt_sha256: String,
    pub physical_case_id: String,
    pub status: ObligationStatusV1,
    pub physical_equivalence_authority: String,
    pub scientific_search_pruning_authorized: bool,
    pub obligations: ReceiverObligationsV1,
    pub diagnostics: ReceiverDiagnosticsV1,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReceiverReceiptV1 {
    #[serde(flatten)]
    pub payload: ReceiverReceiptPayloadV1,
    pub receiver_receipt_sha256: String,
}

/// A stable, explicit binary writer used for all structured evidence digests.
#[derive(Default)]
struct CanonicalWriterV1 {
    bytes: Vec<u8>,
}

impl CanonicalWriterV1 {
    fn field(&mut self, label: &str, payload: &[u8]) {
        self.bytes
            .extend_from_slice(&(label.len() as u64).to_le_bytes());
        self.bytes.extend_from_slice(label.as_bytes());
        self.bytes
            .extend_from_slice(&(payload.len() as u64).to_le_bytes());
        self.bytes.extend_from_slice(payload);
    }

    fn text(&mut self, label: &str, value: &str) {
        self.field(label, value.as_bytes());
    }

    fn number(&mut self, label: &str, value: f64) -> Result<(), ReceiverErrorV1> {
        if !value.is_finite() {
            return Err(ReceiverErrorV1::NonFiniteInput);
        }
        self.field(label, &value.to_bits().to_le_bytes());
        Ok(())
    }

    fn matrix3(&mut self, label: &str, matrix: &Matrix3) -> Result<(), ReceiverErrorV1> {
        let mut bytes = Vec::with_capacity(9 * 8);
        for row in matrix {
            for value in row {
                if !value.is_finite() {
                    return Err(ReceiverErrorV1::NonFiniteInput);
                }
                bytes.extend_from_slice(&value.to_bits().to_le_bytes());
            }
        }
        self.field(label, &bytes);
        Ok(())
    }

    fn vector3(&mut self, label: &str, vector: &Vector3) -> Result<(), ReceiverErrorV1> {
        let mut bytes = Vec::with_capacity(3 * 8);
        for value in vector {
            if !value.is_finite() {
                return Err(ReceiverErrorV1::NonFiniteInput);
            }
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        self.field(label, &bytes);
        Ok(())
    }

    fn jacobian2x3(&mut self, label: &str, jacobian: &Jacobian2x3) -> Result<(), ReceiverErrorV1> {
        let mut bytes = Vec::with_capacity(6 * 8);
        for row in jacobian {
            for value in row {
                if !value.is_finite() {
                    return Err(ReceiverErrorV1::NonFiniteInput);
                }
                bytes.extend_from_slice(&value.to_bits().to_le_bytes());
            }
        }
        self.field(label, &bytes);
        Ok(())
    }
}

fn hash_domain(domain: &str, payload: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"LUMINOUS-EVIDENCE-RECEIVER-V1\0");
    digest.update((domain.len() as u64).to_le_bytes());
    digest.update(domain.as_bytes());
    digest.update((payload.len() as u64).to_le_bytes());
    digest.update(payload);
    encode_hex(&digest.finalize())
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode_hex(value: &str) -> Result<Vec<u8>, ReceiverErrorV1> {
    if value.len() % 2 != 0 {
        return Err(ReceiverErrorV1::SourceReceiptMalformed);
    }
    let mut output = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let high = hex_nibble(pair[0]).ok_or(ReceiverErrorV1::SourceReceiptMalformed)?;
        let low = hex_nibble(pair[1]).ok_or(ReceiverErrorV1::SourceReceiptMalformed)?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

fn valid_git_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

fn canonical_matrix3_hash(domain: &str, matrix: &Matrix3) -> Result<String, ReceiverErrorV1> {
    let mut writer = CanonicalWriterV1::default();
    writer.matrix3("matrix-3x3-f64-le-v1", matrix)?;
    Ok(hash_domain(domain, &writer.bytes))
}

fn canonical_generators_hash(generators: &[Matrix3]) -> Result<String, ReceiverErrorV1> {
    let mut writer = CanonicalWriterV1::default();
    writer.field(
        "generator-count-u64-le-v1",
        &(generators.len() as u64).to_le_bytes(),
    );
    for (index, generator) in generators.iter().enumerate() {
        writer.matrix3(&format!("generator-{index}-3x3-f64-le-v1"), generator)?;
    }
    Ok(hash_domain("luminous.generator-set.v1", &writer.bytes))
}

fn canonical_fixture_hash(
    envelope: &HandoffEnvelopeV1,
    fixture: &PhysicalFixtureV1,
) -> Result<String, ReceiverErrorV1> {
    let mut writer = CanonicalWriterV1::default();
    let physical = &envelope.physical_problem;
    let change = &physical.coordinate_change;
    writer.text("case-id-v1", &physical.case_id);
    writer.text("from-basis-id-v1", &change.from_basis_id);
    writer.text("to-basis-id-v1", &change.to_basis_id);
    writer.text("solver-profile-id-v1", &physical.solver_profile_id);
    writer.text("integration-method-v1", &physical.integration_method);
    writer.text("units-profile-id-v1", &physical.units_profile_id);
    writer.number("time-step-seconds-f64-le-v1", physical.time_step_seconds)?;
    writer.matrix3("transform-s-v1", &fixture.transform_s)?;
    writer.matrix3("mass-metric-q-v1", &fixture.mass_metric_q)?;
    writer.vector3("force-covector-q-v1", &fixture.force_covector_q)?;
    writer.vector3("momentum-covector-q-v1", &fixture.momentum_covector_q)?;
    writer.jacobian2x3("constraint-jacobian-q-v1", &fixture.constraint_jacobian_q)?;
    writer.vector3("initial-position-q-v1", &fixture.initial_position_q)?;
    writer.vector3("initial-velocity-q-v1", &fixture.initial_velocity_q)?;
    writer.matrix3("declared-mass-metric-y-v1", &fixture.declared_mass_metric_y)?;
    writer.vector3(
        "declared-force-covector-y-v1",
        &fixture.declared_force_covector_y,
    )?;
    writer.vector3(
        "declared-momentum-covector-y-v1",
        &fixture.declared_momentum_covector_y,
    )?;
    writer.jacobian2x3(
        "declared-constraint-jacobian-y-v1",
        &fixture.declared_constraint_jacobian_y,
    )?;
    writer.field(
        "declared-generator-count-v1",
        &(fixture.declared_generators_y.len() as u64).to_le_bytes(),
    );
    for (index, generator) in fixture.declared_generators_y.iter().enumerate() {
        writer.matrix3(&format!("declared-generator-y-{index}-v1"), generator)?;
    }
    Ok(hash_domain("luminous.physical-fixture.v1", &writer.bytes))
}

fn canonical_force_covectors_hash(
    force: &Vector3,
    momentum: &Vector3,
) -> Result<String, ReceiverErrorV1> {
    let mut writer = CanonicalWriterV1::default();
    writer.vector3("force-covector-q-v1", force)?;
    writer.vector3("momentum-covector-q-v1", momentum)?;
    Ok(hash_domain("luminous.force-covectors.v1", &writer.bytes))
}

fn canonical_initial_state_hash(
    position: &Vector3,
    velocity: &Vector3,
) -> Result<String, ReceiverErrorV1> {
    let mut writer = CanonicalWriterV1::default();
    writer.vector3("initial-position-q-v1", position)?;
    writer.vector3("initial-velocity-q-v1", velocity)?;
    Ok(hash_domain("luminous.initial-state.v1", &writer.bytes))
}

fn canonical_task_hashes(
    task: &TaskSemanticsArtifactsV1,
) -> (String, String, String, String, String, String, String) {
    let fields = [
        ("candidate-domain", &task.candidate_domain),
        ("objective", &task.objective),
        ("feasibility-constraints", &task.feasibility_constraints),
        (
            "initial-and-boundary-conditions",
            &task.initial_and_boundary_conditions,
        ),
        ("observable-mapping", &task.observable_mapping),
        ("answer-interpretation", &task.answer_interpretation),
    ];
    let hashes: Vec<String> = fields
        .iter()
        .map(|(label, value)| {
            let mut writer = CanonicalWriterV1::default();
            writer.text(label, value);
            hash_domain("luminous.task-semantic-field.v1", &writer.bytes)
        })
        .collect();

    let mut task_writer = CanonicalWriterV1::default();
    for (index, (label, _)) in fields.iter().enumerate() {
        task_writer.text(label, &hashes[index]);
    }
    let task_hash = hash_domain("luminous.full-task-semantics.v1", &task_writer.bytes);
    (
        task_hash,
        hashes[0].clone(),
        hashes[1].clone(),
        hashes[2].clone(),
        hashes[3].clone(),
        hashes[4].clone(),
        hashes[5].clone(),
    )
}

fn source_receipt_binding(
    package: &ReceiverPackageV1,
    exact_bytes: &[u8],
    generator_hash: &str,
) -> Result<(), ReceiverErrorV1> {
    let reference = &package.envelope.source_math_receipt;
    if reference.repository != "Luminous-Dynamics/symthaea" {
        return Err(ReceiverErrorV1::SourceRepositoryMismatch);
    }
    if !valid_git_sha(&reference.commit_sha) {
        return Err(ReceiverErrorV1::SourceCommitMismatch);
    }
    if !valid_sha256(&reference.receipt_sha256)
        || hash_domain("luminous.source-math-receipt.raw-bytes.v1", exact_bytes)
            != reference.receipt_sha256
    {
        return Err(ReceiverErrorV1::SourceReceiptDigestMismatch);
    }
    if !valid_sha256(&reference.generator_set_sha256)
        || generator_hash != reference.generator_set_sha256
    {
        return Err(ReceiverErrorV1::GeneratorDigestMismatch);
    }

    let value: Value =
        serde_json::from_slice(exact_bytes).map_err(|_| ReceiverErrorV1::SourceReceiptMalformed)?;
    let string_field = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .ok_or(ReceiverErrorV1::SourceReceiptMalformed)
    };
    let identities_match = string_field("repository")? == reference.repository
        && string_field("commit_sha")? == reference.commit_sha
        && string_field("receipt_id")? == reference.receipt_id
        && string_field("checker_id")? == reference.checker_id
        && string_field("checker_version")? == reference.checker_version
        && string_field("profile_id")? == reference.profile_id
        && string_field("action_id")? == reference.action_id
        && string_field("coordinate_basis_id")? == reference.coordinate_basis_id
        && string_field("metric_id")? == reference.metric_id
        && string_field("generator_set_sha256")? == reference.generator_set_sha256
        && string_field("evidence_class")? == reference.evidence_class
        && string_field("qualification_status")? == reference.qualification_status;
    if !identities_match || reference.qualification_status != "PASS" {
        return Err(ReceiverErrorV1::SourceReceiptIdentityMismatch);
    }
    Ok(())
}

fn max_abs_matrix(matrix: &Matrix3) -> f64 {
    matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0, f64::max)
}

fn max_abs_vector(vector: &Vector3) -> f64 {
    vector.iter().map(|value| value.abs()).fold(0.0, f64::max)
}

fn max_abs_jacobian(matrix: &Jacobian2x3) -> f64 {
    matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0, f64::max)
}

fn relative_matrix_defect(left: &Matrix3, right: &Matrix3) -> f64 {
    let mut defect = 0.0_f64;
    for row in 0..3 {
        for column in 0..3 {
            defect = defect.max((left[row][column] - right[row][column]).abs());
        }
    }
    defect / 1.0_f64.max(max_abs_matrix(left)).max(max_abs_matrix(right))
}

fn relative_vector_defect(left: &Vector3, right: &Vector3) -> f64 {
    let mut defect = 0.0_f64;
    for index in 0..3 {
        defect = defect.max((left[index] - right[index]).abs());
    }
    defect / 1.0_f64.max(max_abs_vector(left)).max(max_abs_vector(right))
}

fn relative_jacobian_defect(left: &Jacobian2x3, right: &Jacobian2x3) -> f64 {
    let mut defect = 0.0_f64;
    for row in 0..2 {
        for column in 0..3 {
            defect = defect.max((left[row][column] - right[row][column]).abs());
        }
    }
    defect
        / 1.0_f64
            .max(max_abs_jacobian(left))
            .max(max_abs_jacobian(right))
}

fn transpose(matrix: &Matrix3) -> Matrix3 {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row][column] = matrix[column][row];
        }
    }
    result
}

fn multiply(left: &Matrix3, right: &Matrix3) -> Matrix3 {
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

fn apply(matrix: &Matrix3, vector: &Vector3) -> Vector3 {
    let mut result = [0.0; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row] += matrix[row][column] * vector[column];
        }
    }
    result
}

fn transpose_apply(matrix: &Matrix3, vector: &Vector3) -> Vector3 {
    apply(&transpose(matrix), vector)
}

fn multiply_jacobian(jacobian: &Jacobian2x3, matrix: &Matrix3) -> Jacobian2x3 {
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

fn inverse_and_condition(
    matrix: &Matrix3,
    profile: ReceiverProfileV1,
) -> Result<(Matrix3, f64), ReceiverErrorV1> {
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ReceiverErrorV1::NonFiniteInput);
    }
    let matrix_norm = matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max);
    if matrix_norm == 0.0 || !matrix_norm.is_finite() {
        return Err(ReceiverErrorV1::SingularCoordinateChange { pivot: 0 });
    }

    let threshold = profile.relative_transform_pivot_tolerance * matrix_norm;
    if !threshold.is_finite() {
        return Err(ReceiverErrorV1::NonFiniteDerivedValue);
    }
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
            .ok_or(ReceiverErrorV1::SingularCoordinateChange { pivot: column })?;
        let pivot_magnitude = left[pivot_row][column].abs();
        if !pivot_magnitude.is_finite() || pivot_magnitude <= threshold {
            return Err(ReceiverErrorV1::SingularCoordinateChange { pivot: column });
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
        return Err(ReceiverErrorV1::NonFiniteDerivedValue);
    }
    let inverse_norm = inverse
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max);
    let condition = matrix_norm * inverse_norm;
    if !condition.is_finite() {
        return Err(ReceiverErrorV1::NonFiniteDerivedValue);
    }
    if condition > profile.maximum_transform_condition_inf {
        return Err(ReceiverErrorV1::CoordinateConditionOutOfEnvelope {
            estimate_bits: condition.to_bits(),
        });
    }
    Ok((inverse, condition))
}

fn cholesky(
    metric: &Matrix3,
    profile: ReceiverProfileV1,
) -> Result<(Matrix3, f64), ReceiverErrorV1> {
    if metric.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ReceiverErrorV1::NonFiniteInput);
    }
    let scale = metric
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max);
    if !scale.is_finite() || scale == 0.0 {
        return Err(ReceiverErrorV1::NonPositiveDefiniteMetric { pivot: 0 });
    }
    let mut symmetry_defect = 0.0_f64;
    for row in 0..3 {
        for column in 0..3 {
            symmetry_defect =
                symmetry_defect.max((metric[row][column] - metric[column][row]).abs());
        }
    }
    let relative_symmetry = symmetry_defect / scale;
    if relative_symmetry > profile.maximum_metric_symmetry_defect {
        return Err(ReceiverErrorV1::NonSymmetricMetric {
            defect_bits: relative_symmetry.to_bits(),
        });
    }

    let mut lower = [[0.0; 3]; 3];
    let pivot_threshold = profile.relative_transform_pivot_tolerance * scale;
    for row in 0..3 {
        for column in 0..=row {
            let mut value = 0.5 * (metric[row][column] + metric[column][row]);
            for inner in 0..column {
                value -= lower[row][inner] * lower[column][inner];
            }
            if !value.is_finite() {
                return Err(ReceiverErrorV1::NonFiniteDerivedValue);
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
                return Err(ReceiverErrorV1::NonFiniteDerivedValue);
            }
        }
    }
    Ok((lower, relative_symmetry))
}

fn solve_spd(lower: &Matrix3, rhs: &Vector3) -> Result<Vector3, ReceiverErrorV1> {
    let mut intermediate = [0.0; 3];
    for row in 0..3 {
        let mut value = rhs[row];
        for column in 0..row {
            value -= lower[row][column] * intermediate[column];
        }
        intermediate[row] = value / lower[row][row];
    }
    let mut solution = [0.0; 3];
    for row in (0..3).rev() {
        let mut value = intermediate[row];
        for column in (row + 1)..3 {
            value -= lower[column][row] * solution[column];
        }
        solution[row] = value / lower[row][row];
        if !solution[row].is_finite() {
            return Err(ReceiverErrorV1::NonFiniteDerivedValue);
        }
    }
    Ok(solution)
}

fn tangent_norm(metric: &Matrix3, vector: &Vector3) -> Result<f64, ReceiverErrorV1> {
    let raised = apply(metric, vector);
    let square = dot(vector, &raised);
    if !square.is_finite() || square < -1.0e-12 {
        return Err(ReceiverErrorV1::NonFiniteDerivedValue);
    }
    Ok(square.max(0.0).sqrt())
}

fn cotangent_norm(lower: &Matrix3, covector: &Vector3) -> Result<f64, ReceiverErrorV1> {
    let raised = solve_spd(lower, covector)?;
    let square = dot(covector, &raised);
    if !square.is_finite() || square < -1.0e-12 {
        return Err(ReceiverErrorV1::NonFiniteDerivedValue);
    }
    Ok(square.max(0.0).sqrt())
}

fn dot(left: &Vector3, right: &Vector3) -> f64 {
    (0..3).map(|index| left[index] * right[index]).sum()
}

fn isometry_relative_defect(generator: &Matrix3, metric: &Matrix3) -> f64 {
    let gt_m = multiply(&transpose(generator), metric);
    let mg = multiply(metric, generator);
    let mut defect = 0.0_f64;
    let mut scale = 1.0_f64;
    for row in 0..3 {
        for column in 0..3 {
            defect = defect.max((gt_m[row][column] + mg[row][column]).abs());
            scale = scale
                .max(gt_m[row][column].abs())
                .max(mg[row][column].abs());
        }
    }
    defect / scale
}

fn compare_or_fail(
    defect: f64,
    maximum: f64,
    error: impl FnOnce(u64) -> ReceiverErrorV1,
) -> Result<f64, ReceiverErrorV1> {
    if !defect.is_finite() {
        return Err(ReceiverErrorV1::NonFiniteDerivedValue);
    }
    if defect > maximum {
        return Err(error(defect.to_bits()));
    }
    Ok(defect)
}

fn validate_profile(profile: ReceiverProfileV1) -> Result<(), ReceiverErrorV1> {
    if !profile.relative_transform_pivot_tolerance.is_finite()
        || profile.relative_transform_pivot_tolerance <= 0.0
        || profile.relative_transform_pivot_tolerance >= 1.0
        || !profile.maximum_transform_condition_inf.is_finite()
        || profile.maximum_transform_condition_inf < 1.0
        || !profile.maximum_relative_law_defect.is_finite()
        || profile.maximum_relative_law_defect <= 0.0
        || !profile.maximum_metric_symmetry_defect.is_finite()
        || profile.maximum_metric_symmetry_defect < 0.0
    {
        return Err(ReceiverErrorV1::InvalidProfile);
    }
    Ok(())
}

pub fn verify_package(
    package: &ReceiverPackageV1,
    profile: ReceiverProfileV1,
) -> Result<ReceiverReceiptV1, ReceiverErrorV1> {
    validate_profile(profile)?;
    if package.schema_version != RECEIVER_SCHEMA_VERSION {
        return Err(ReceiverErrorV1::WrongPackageSchema);
    }
    let envelope = &package.envelope;
    if envelope.schema_version != "luminous.symmetry_physical_handoff.v0.1" {
        return Err(ReceiverErrorV1::WrongEnvelopeSchema);
    }
    if envelope.fixture_mode != "admissible_receipt" {
        return Err(ReceiverErrorV1::NotAdmissibleFixture);
    }
    if envelope.handoff_id.trim().is_empty()
        || envelope.physical_problem.case_id.trim().is_empty()
        || envelope.source_math_receipt.receipt_id.trim().is_empty()
    {
        return Err(ReceiverErrorV1::InvalidIdentifier);
    }
    if envelope.claims.physical_step_equivariance_status != "INCONCLUSIVE"
        || envelope.claims.physical_equivalence_authority != "NONE"
        || envelope.claims.scientific_search_pruning_authorized
    {
        return Err(ReceiverErrorV1::ClaimAlreadyPromoted);
    }
    if envelope.source_math_receipt.evidence_class != "numeric_lie_basis" {
        return Err(ReceiverErrorV1::UnsupportedEvidenceClass);
    }
    if package.generator_set.is_empty() {
        return Err(ReceiverErrorV1::UnsupportedEvidenceClass);
    }
    if envelope.source_math_receipt.qualification_status != "PASS" {
        return Err(ReceiverErrorV1::SourceReceiptIdentityMismatch);
    }
    if envelope.physical_problem.coordinate_change.kind != "generalized_coordinate_rebasing" {
        return Err(ReceiverErrorV1::WrongEnvelopeSchema);
    }

    let source_bytes = decode_hex(&package.source_receipt_bytes_hex)?;
    let generators_hash = canonical_generators_hash(&package.generator_set)?;
    source_receipt_binding(package, &source_bytes, &generators_hash)?;

    let fixture = &package.physical_fixture;
    let physical = &envelope.physical_problem;
    let source = &envelope.source_math_receipt;

    let transform_hash = canonical_matrix3_hash("luminous.transform-s.v1", &fixture.transform_s)?;
    if transform_hash != physical.coordinate_change.transform_sha256 {
        return Err(ReceiverErrorV1::PhysicalFixtureDigestMismatch);
    }
    let fixture_hash = canonical_fixture_hash(envelope, fixture)?;
    if fixture_hash != physical.fixture_sha256 {
        return Err(ReceiverErrorV1::PhysicalFixtureDigestMismatch);
    }
    if canonical_matrix3_hash("luminous.mass-metric-q.v1", &fixture.mass_metric_q)?
        != physical.mass_metric_sha256
    {
        return Err(ReceiverErrorV1::MassMetricDigestMismatch);
    }
    if canonical_force_covectors_hash(&fixture.force_covector_q, &fixture.momentum_covector_q)?
        != physical.force_covectors_sha256
    {
        return Err(ReceiverErrorV1::ForceCovectorDigestMismatch);
    }
    {
        let mut writer = CanonicalWriterV1::default();
        writer.jacobian2x3("constraint-jacobian-q-v1", &fixture.constraint_jacobian_q)?;
        if hash_domain("luminous.constraint-map.v1", &writer.bytes)
            != physical.constraint_map_sha256
        {
            return Err(ReceiverErrorV1::ConstraintMapDigestMismatch);
        }
    }
    if canonical_initial_state_hash(&fixture.initial_position_q, &fixture.initial_velocity_q)?
        != physical.initial_state_sha256
    {
        return Err(ReceiverErrorV1::InitialStateDigestMismatch);
    }

    let (
        task_hash,
        domain_hash,
        objective_hash,
        feasibility_hash,
        boundary_hash,
        observable_hash,
        answer_hash,
    ) = canonical_task_hashes(&package.task_semantics);
    let task_ref = &envelope.full_task_semantics;
    if task_hash != task_ref.task_sha256
        || domain_hash != task_ref.candidate_domain_sha256
        || objective_hash != task_ref.objective_sha256
        || feasibility_hash != task_ref.feasibility_constraints_sha256
        || boundary_hash != task_ref.initial_and_boundary_conditions_sha256
        || observable_hash != task_ref.observable_mapping_sha256
        || answer_hash != task_ref.answer_interpretation_sha256
    {
        return Err(ReceiverErrorV1::TaskSemanticDigestMismatch);
    }

    let (lower_q, symmetry_defect) = cholesky(&fixture.mass_metric_q, profile)?;
    let (inverse_s, condition) = match inverse_and_condition(&fixture.transform_s, profile) {
        Ok(result) => result,
        Err(ReceiverErrorV1::CoordinateConditionOutOfEnvelope { estimate_bits }) => {
            return out_of_envelope_receipt(envelope, f64::from_bits(estimate_bits));
        }
        Err(error) => return Err(error),
    };
    let mass_y = multiply(
        &multiply(&transpose(&fixture.transform_s), &fixture.mass_metric_q),
        &fixture.transform_s,
    );
    let (lower_y, _) = cholesky(&mass_y, profile)?;

    let force_y = transpose_apply(&fixture.transform_s, &fixture.force_covector_q);
    let momentum_y = transpose_apply(&fixture.transform_s, &fixture.momentum_covector_q);
    let jacobian_y = multiply_jacobian(&fixture.constraint_jacobian_q, &fixture.transform_s);
    let mut generators_y = Vec::with_capacity(package.generator_set.len());
    for generator in &package.generator_set {
        generators_y.push(multiply(
            &multiply(&inverse_s, generator),
            &fixture.transform_s,
        ));
    }

    let mass_defect = relative_matrix_defect(&mass_y, &fixture.declared_mass_metric_y);
    let mass_defect = compare_or_fail(mass_defect, profile.maximum_relative_law_defect, |bits| {
        ReceiverErrorV1::WrongMetricCongruence { defect_bits: bits }
    })?;
    if canonical_matrix3_hash(
        "luminous.transformed-mass-metric-y.v1",
        &fixture.declared_mass_metric_y,
    )? != physical.coordinate_change.transformed_mass_metric_sha256
    {
        return Err(ReceiverErrorV1::MassMetricDigestMismatch);
    }

    let force_defect = compare_or_fail(
        relative_vector_defect(&force_y, &fixture.declared_force_covector_y),
        profile.maximum_relative_law_defect,
        |bits| ReceiverErrorV1::WrongForceCovectorLaw { defect_bits: bits },
    )?;
    let momentum_defect = compare_or_fail(
        relative_vector_defect(&momentum_y, &fixture.declared_momentum_covector_y),
        profile.maximum_relative_law_defect,
        |bits| ReceiverErrorV1::WrongMomentumCovectorLaw { defect_bits: bits },
    )?;
    let jacobian_defect = compare_or_fail(
        relative_jacobian_defect(&jacobian_y, &fixture.declared_constraint_jacobian_y),
        profile.maximum_relative_law_defect,
        |bits| ReceiverErrorV1::WrongConstraintJacobianLaw { defect_bits: bits },
    )?;

    if generators_y.len() != fixture.declared_generators_y.len() {
        return Err(ReceiverErrorV1::WrongGeneratorConjugation {
            defect_bits: f64::INFINITY.to_bits(),
        });
    }
    let mut max_generator_conjugation = 0.0_f64;
    let mut max_isometry_q = 0.0_f64;
    let mut max_isometry_y = 0.0_f64;
    for (index, (expected, declared)) in generators_y
        .iter()
        .zip(&fixture.declared_generators_y)
        .enumerate()
    {
        let conjugation_defect = relative_matrix_defect(expected, declared);
        max_generator_conjugation = max_generator_conjugation.max(conjugation_defect);
        compare_or_fail(
            conjugation_defect,
            profile.maximum_relative_law_defect,
            |bits| ReceiverErrorV1::WrongGeneratorConjugation { defect_bits: bits },
        )?;

        let isometry_q =
            isometry_relative_defect(&package.generator_set[index], &fixture.mass_metric_q);
        let isometry_y = isometry_relative_defect(expected, &mass_y);
        max_isometry_q = max_isometry_q.max(isometry_q);
        max_isometry_y = max_isometry_y.max(isometry_y);
        if isometry_q > profile.maximum_relative_law_defect
            || isometry_y > profile.maximum_relative_law_defect
        {
            return Err(ReceiverErrorV1::NonIsometricGenerator {
                generator_index: index,
                defect_bits: isometry_q.max(isometry_y).to_bits(),
            });
        }
    }

    let velocity_y = apply(&inverse_s, &fixture.initial_velocity_q);
    let force_pairing_q = dot(&fixture.force_covector_q, &fixture.initial_velocity_q);
    let force_pairing_y = dot(&force_y, &velocity_y);
    let momentum_pairing_q = dot(&fixture.momentum_covector_q, &fixture.initial_velocity_q);
    let momentum_pairing_y = dot(&momentum_y, &velocity_y);
    let force_pairing_defect = relative_scalar_defect(force_pairing_q, force_pairing_y);
    let momentum_pairing_defect = relative_scalar_defect(momentum_pairing_q, momentum_pairing_y);
    compare_or_fail(
        force_pairing_defect,
        profile.maximum_relative_law_defect,
        |bits| ReceiverErrorV1::PairingNotPreserved { defect_bits: bits },
    )?;
    compare_or_fail(
        momentum_pairing_defect,
        profile.maximum_relative_law_defect,
        |bits| ReceiverErrorV1::PairingNotPreserved { defect_bits: bits },
    )?;

    let tangent_norm_q = tangent_norm(&fixture.mass_metric_q, &fixture.initial_velocity_q)?;
    let tangent_norm_y = tangent_norm(&mass_y, &velocity_y)?;
    let tangent_defect = relative_scalar_defect(tangent_norm_q, tangent_norm_y);
    compare_or_fail(
        tangent_defect,
        profile.maximum_relative_law_defect,
        |bits| ReceiverErrorV1::TangentNormNotPreserved { defect_bits: bits },
    )?;

    let force_dual_q = cotangent_norm(&lower_q, &fixture.force_covector_q)?;
    let force_dual_y = cotangent_norm(&lower_y, &force_y)?;
    let force_dual_defect = relative_scalar_defect(force_dual_q, force_dual_y);
    compare_or_fail(
        force_dual_defect,
        profile.maximum_relative_law_defect,
        |bits| ReceiverErrorV1::CotangentNormNotPreserved { defect_bits: bits },
    )?;
    let momentum_dual_q = cotangent_norm(&lower_q, &fixture.momentum_covector_q)?;
    let momentum_dual_y = cotangent_norm(&lower_y, &momentum_y)?;
    let momentum_dual_defect = relative_scalar_defect(momentum_dual_q, momentum_dual_y);
    compare_or_fail(
        momentum_dual_defect,
        profile.maximum_relative_law_defect,
        |bits| ReceiverErrorV1::CotangentNormNotPreserved { defect_bits: bits },
    )?;

    if source.metric_id.trim().is_empty()
        || source.coordinate_basis_id != physical.coordinate_change.from_basis_id
        || envelope
            .physical_problem
            .solver_profile_id
            .trim()
            .is_empty()
        || envelope.physical_problem.units_profile_id.trim().is_empty()
        || envelope.physical_problem.time_step_seconds <= 0.0
        || !envelope.physical_problem.time_step_seconds.is_finite()
    {
        return Err(ReceiverErrorV1::InvalidIdentifier);
    }

    let obligations = ReceiverObligationsV1 {
        mass_metric_congruence: ObligationStatusV1::Pass,
        force_covector_duality: ObligationStatusV1::Pass,
        momentum_one_form: ObligationStatusV1::Pass,
        constraint_jacobian: ObligationStatusV1::Pass,
        generator_metric_isometry: ObligationStatusV1::Pass,
        discrete_step_equivariance: ObligationStatusV1::Inconclusive,
        physical_observables: ObligationStatusV1::Inconclusive,
        numerical_error_envelope: ObligationStatusV1::Inconclusive,
        full_task_semantics: ObligationStatusV1::Pass,
    };

    let diagnostics = ReceiverDiagnosticsV1 {
        transform_condition_inf_estimate: condition,
        mass_metric_relative_symmetry_defect: symmetry_defect,
        mass_congruence_relative_defect: mass_defect,
        force_covector_relative_defect: force_defect,
        momentum_covector_relative_defect: momentum_defect,
        constraint_jacobian_relative_defect: jacobian_defect,
        maximum_generator_conjugation_relative_defect: max_generator_conjugation,
        maximum_generator_isometry_relative_defect_q: max_isometry_q,
        maximum_generator_isometry_relative_defect_y: max_isometry_y,
        force_pairing_relative_defect: force_pairing_defect,
        momentum_pairing_relative_defect: momentum_pairing_defect,
        tangent_norm_relative_defect: tangent_defect,
        force_dual_norm_relative_defect: force_dual_defect,
        momentum_dual_norm_relative_defect: momentum_dual_defect,
    };

    let payload = ReceiverReceiptPayloadV1 {
        schema_version: RECEIVER_SCHEMA_VERSION.to_owned(),
        verifier_id: RECEIVER_ID.to_owned(),
        verifier_profile_id: RECEIVER_PROFILE_ID.to_owned(),
        handoff_id: envelope.handoff_id.clone(),
        source_repository: source.repository.clone(),
        source_commit_sha: source.commit_sha.clone(),
        source_receipt_sha256: source.receipt_sha256.clone(),
        physical_case_id: physical.case_id.clone(),
        status: ObligationStatusV1::Inconclusive,
        physical_equivalence_authority: "NONE".to_owned(),
        scientific_search_pruning_authorized: false,
        obligations,
        diagnostics,
        claim_boundary: "Bounded 3D coordinate re-description only; no discrete-step, physical-observable, nonlinear solver, or scientific pruning authority.".to_owned(),
    };
    sign_receipt(payload)
}

fn sign_receipt(payload: ReceiverReceiptPayloadV1) -> Result<ReceiverReceiptV1, ReceiverErrorV1> {
    let receipt_bytes =
        serde_json::to_vec(&payload).map_err(|_| ReceiverErrorV1::ReceiptSerializationFailure)?;
    let receipt_hash = hash_domain("luminous.receiver-owned-receipt.v1", &receipt_bytes);
    Ok(ReceiverReceiptV1 {
        payload,
        receiver_receipt_sha256: receipt_hash,
    })
}

fn out_of_envelope_receipt(
    envelope: &HandoffEnvelopeV1,
    condition_estimate: f64,
) -> Result<ReceiverReceiptV1, ReceiverErrorV1> {
    let obligations = ReceiverObligationsV1 {
        mass_metric_congruence: ObligationStatusV1::Inconclusive,
        force_covector_duality: ObligationStatusV1::Inconclusive,
        momentum_one_form: ObligationStatusV1::Inconclusive,
        constraint_jacobian: ObligationStatusV1::Inconclusive,
        generator_metric_isometry: ObligationStatusV1::Inconclusive,
        discrete_step_equivariance: ObligationStatusV1::Inconclusive,
        physical_observables: ObligationStatusV1::Inconclusive,
        numerical_error_envelope: ObligationStatusV1::Inconclusive,
        full_task_semantics: ObligationStatusV1::Pass,
    };
    let diagnostics = ReceiverDiagnosticsV1 {
        transform_condition_inf_estimate: condition_estimate,
        mass_metric_relative_symmetry_defect: 0.0,
        mass_congruence_relative_defect: 0.0,
        force_covector_relative_defect: 0.0,
        momentum_covector_relative_defect: 0.0,
        constraint_jacobian_relative_defect: 0.0,
        maximum_generator_conjugation_relative_defect: 0.0,
        maximum_generator_isometry_relative_defect_q: 0.0,
        maximum_generator_isometry_relative_defect_y: 0.0,
        force_pairing_relative_defect: 0.0,
        momentum_pairing_relative_defect: 0.0,
        tangent_norm_relative_defect: 0.0,
        force_dual_norm_relative_defect: 0.0,
        momentum_dual_norm_relative_defect: 0.0,
    };
    let payload = ReceiverReceiptPayloadV1 {
        schema_version: RECEIVER_SCHEMA_VERSION.to_owned(),
        verifier_id: RECEIVER_ID.to_owned(),
        verifier_profile_id: RECEIVER_PROFILE_ID.to_owned(),
        handoff_id: envelope.handoff_id.clone(),
        source_repository: envelope.source_math_receipt.repository.clone(),
        source_commit_sha: envelope.source_math_receipt.commit_sha.clone(),
        source_receipt_sha256: envelope.source_math_receipt.receipt_sha256.clone(),
        physical_case_id: envelope.physical_problem.case_id.clone(),
        status: ObligationStatusV1::Inconclusive,
        physical_equivalence_authority: "NONE".to_owned(),
        scientific_search_pruning_authorized: false,
        obligations,
        diagnostics,
        claim_boundary: format!(
            "INCONCLUSIVE: transform condition estimate {condition_estimate:.6e} exceeds the declared receiver profile; physical coordinate laws were not promoted."
        ),
    };
    sign_receipt(payload)
}

fn relative_scalar_defect(left: f64, right: f64) -> f64 {
    (left - right).abs() / 1.0_f64.max(left.abs()).max(right.abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profile() -> ReceiverProfileV1 {
        ReceiverProfileV1 {
            relative_transform_pivot_tolerance: 1.0e-14,
            maximum_transform_condition_inf: 1.0e6,
            maximum_relative_law_defect: 5.0e-10,
            maximum_metric_symmetry_defect: 1.0e-12,
        }
    }

    fn build_package() -> ReceiverPackageV1 {
        let generators = vec![[[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]]];
        let transform_s = [[1.0, 2.0, -1.0], [0.0, 1.0, 3.0], [0.0, 0.0, 1.0]];
        let mass_q = [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 5.0]];
        let force_q = [1.0, 2.0, 3.0];
        let momentum_q = [0.5, -1.0, 1.5];
        let jacobian_q = [[1.0, 0.0, 1.0], [0.0, 1.0, -1.0]];
        let position_q = [0.1, 0.2, 0.3];
        let velocity_q = [0.5, -0.25, 0.75];
        let inverse_s = inverse_and_condition(&transform_s, test_profile())
            .unwrap()
            .0;
        let mass_y = multiply(&multiply(&transpose(&transform_s), &mass_q), &transform_s);
        let force_y = transpose_apply(&transform_s, &force_q);
        let momentum_y = transpose_apply(&transform_s, &momentum_q);
        let jacobian_y = multiply_jacobian(&jacobian_q, &transform_s);
        let generators_y = generators
            .iter()
            .map(|generator| multiply(&multiply(&inverse_s, generator), &transform_s))
            .collect::<Vec<_>>();

        let fixture = PhysicalFixtureV1 {
            transform_s,
            mass_metric_q: mass_q,
            force_covector_q: force_q,
            momentum_covector_q: momentum_q,
            constraint_jacobian_q: jacobian_q,
            initial_position_q: position_q,
            initial_velocity_q: velocity_q,
            declared_mass_metric_y: mass_y,
            declared_force_covector_y: force_y,
            declared_momentum_covector_y: momentum_y,
            declared_constraint_jacobian_y: jacobian_y,
            declared_generators_y: generators_y,
        };

        let task = TaskSemanticsArtifactsV1 {
            candidate_domain: "candidate-domain-v0.1".to_owned(),
            objective: "objective-minimize-energy-v0.1".to_owned(),
            feasibility_constraints: "holonomic-constraint-set-v0.1".to_owned(),
            initial_and_boundary_conditions: "off-axis-initial-state-v0.1".to_owned(),
            observable_mapping: "position-velocity-momentum-observables-v0.1".to_owned(),
            answer_interpretation: "physical-state-equivalence-only-v0.1".to_owned(),
        };
        let task_hashes = canonical_task_hashes(&task);
        let generator_hash = canonical_generators_hash(&generators).unwrap();
        let source_receipt = serde_json::json!({
            "repository": "Luminous-Dynamics/symthaea",
            "commit_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "receipt_id": "synthetic-lie-basis-receipt-v0.1",
            "checker_id": "symthaea-linear-lie-algebra",
            "checker_version": "0.1.0",
            "profile_id": "numeric-lie-profile-v0.1",
            "action_id": "synthetic-so2-action",
            "coordinate_basis_id": "cartesian-q-v0.1",
            "metric_id": "mass-metric-q-v0.1",
            "generator_set_sha256": generator_hash,
            "evidence_class": "numeric_lie_basis",
            "qualification_status": "PASS"
        });
        let source_receipt_bytes = serde_json::to_vec(&source_receipt).unwrap();
        let source_receipt_sha = hash_domain(
            "luminous.source-math-receipt.raw-bytes.v1",
            &source_receipt_bytes,
        );
        let mut package = ReceiverPackageV1 {
            schema_version: RECEIVER_SCHEMA_VERSION.to_owned(),
            envelope: HandoffEnvelopeV1 {
                schema_version: "luminous.symmetry_physical_handoff.v0.1".to_owned(),
                fixture_mode: "admissible_receipt".to_owned(),
                handoff_id: "synthetic-dense-3d-receiver-test-v0.1".to_owned(),
                source_math_receipt: SourceMathReceiptRefV1 {
                    repository: "Luminous-Dynamics/symthaea".to_owned(),
                    commit_sha: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
                    receipt_id: "synthetic-lie-basis-receipt-v0.1".to_owned(),
                    receipt_sha256: source_receipt_sha,
                    checker_id: "symthaea-linear-lie-algebra".to_owned(),
                    checker_version: "0.1.0".to_owned(),
                    profile_id: "numeric-lie-profile-v0.1".to_owned(),
                    action_id: "synthetic-so2-action".to_owned(),
                    coordinate_basis_id: "cartesian-q-v0.1".to_owned(),
                    metric_id: "mass-metric-q-v0.1".to_owned(),
                    generator_set_sha256: generator_hash,
                    evidence_class: "numeric_lie_basis".to_owned(),
                    qualification_status: "PASS".to_owned(),
                    group_order: None,
                },
                physical_problem: PhysicalProblemRefV1 {
                    case_id: "synthetic-dense-3d-case-v0.1".to_owned(),
                    fixture_sha256: String::new(),
                    coordinate_change: CoordinateChangeRefV1 {
                        kind: "generalized_coordinate_rebasing".to_owned(),
                        from_basis_id: "cartesian-q-v0.1".to_owned(),
                        to_basis_id: "dense-y-v0.1".to_owned(),
                        transform_sha256: canonical_matrix3_hash(
                            "luminous.transform-s.v1",
                            &transform_s,
                        )
                        .unwrap(),
                        transformed_mass_metric_sha256: canonical_matrix3_hash(
                            "luminous.transformed-mass-metric-y.v1",
                            &mass_y,
                        )
                        .unwrap(),
                    },
                    mass_metric_sha256: canonical_matrix3_hash(
                        "luminous.mass-metric-q.v1",
                        &mass_q,
                    )
                    .unwrap(),
                    force_covectors_sha256: canonical_force_covectors_hash(&force_q, &momentum_q)
                        .unwrap(),
                    constraint_map_sha256: {
                        let mut writer = CanonicalWriterV1::default();
                        writer
                            .jacobian2x3("constraint-jacobian-q-v1", &jacobian_q)
                            .unwrap();
                        hash_domain("luminous.constraint-map.v1", &writer.bytes)
                    },
                    initial_state_sha256: canonical_initial_state_hash(&position_q, &velocity_q)
                        .unwrap(),
                    solver_profile_id: "constrained-midpoint-profile-v0.1".to_owned(),
                    integration_method: "implicit-midpoint".to_owned(),
                    time_step_seconds: 0.01,
                    units_profile_id: "SI-v1".to_owned(),
                },
                full_task_semantics: TaskSemanticsRefV1 {
                    task_sha256: task_hashes.0,
                    candidate_domain_sha256: task_hashes.1,
                    objective_sha256: task_hashes.2,
                    feasibility_constraints_sha256: task_hashes.3,
                    initial_and_boundary_conditions_sha256: task_hashes.4,
                    observable_mapping_sha256: task_hashes.5,
                    answer_interpretation_sha256: task_hashes.6,
                },
                receiver_verification: serde_json::json!({
                    "repository": "Luminous-Dynamics/symtropy",
                    "commit_sha": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "checker_id": RECEIVER_ID,
                    "checker_version": "0.1.0",
                    "profile_id": RECEIVER_PROFILE_ID,
                    "status": "INCONCLUSIVE",
                    "obligations": {
                        "mass_metric_congruence": "INCONCLUSIVE",
                        "force_covector_duality": "INCONCLUSIVE",
                        "momentum_one_form": "INCONCLUSIVE",
                        "constraint_jacobian": "INCONCLUSIVE",
                        "generator_metric_isometry": "INCONCLUSIVE",
                        "discrete_step_equivariance": "INCONCLUSIVE",
                        "physical_observables": "INCONCLUSIVE",
                        "numerical_error_envelope": "INCONCLUSIVE",
                        "full_task_semantics": "INCONCLUSIVE"
                    }
                }),
                claims: ClaimsRefV1 {
                    physical_step_equivariance_status: "INCONCLUSIVE".to_owned(),
                    physical_equivalence_authority: "NONE".to_owned(),
                    scientific_search_pruning_authorized: false,
                },
                claim_boundary: serde_json::json!({
                    "scope": "synthetic test fixture only",
                    "limitations": ["not a real Symthaea artifact"]
                }),
            },
            source_receipt_bytes_hex: encode_hex(&source_receipt_bytes),
            generator_set: generators,
            physical_fixture: fixture,
            task_semantics: task,
        };
        package.envelope.physical_problem.fixture_sha256 =
            canonical_fixture_hash(&package.envelope, &package.physical_fixture).unwrap();
        package
    }

    #[test]
    fn independent_receiver_passes_coordinate_laws_but_not_step_equivariance() {
        let package = build_package();
        let receipt = verify_package(&package, test_profile()).unwrap();
        assert_eq!(receipt.payload.status, ObligationStatusV1::Inconclusive);
        assert_eq!(
            receipt.payload.obligations.mass_metric_congruence,
            ObligationStatusV1::Pass
        );
        assert_eq!(
            receipt.payload.obligations.force_covector_duality,
            ObligationStatusV1::Pass
        );
        assert_eq!(
            receipt.payload.obligations.momentum_one_form,
            ObligationStatusV1::Pass
        );
        assert_eq!(
            receipt.payload.obligations.constraint_jacobian,
            ObligationStatusV1::Pass
        );
        assert_eq!(
            receipt.payload.obligations.generator_metric_isometry,
            ObligationStatusV1::Pass
        );
        assert_eq!(
            receipt.payload.obligations.full_task_semantics,
            ObligationStatusV1::Pass
        );
        assert_eq!(
            receipt.payload.obligations.discrete_step_equivariance,
            ObligationStatusV1::Inconclusive
        );
        assert_eq!(
            receipt.payload.obligations.physical_observables,
            ObligationStatusV1::Inconclusive
        );
        assert_eq!(
            receipt.payload.obligations.numerical_error_envelope,
            ObligationStatusV1::Inconclusive
        );
        assert_eq!(receipt.payload.physical_equivalence_authority, "NONE");
        assert!(!receipt.payload.scientific_search_pruning_authorized);
        assert!(valid_sha256(&receipt.receiver_receipt_sha256));
    }

    #[test]
    fn source_receipt_byte_mutation_is_rejected() {
        let mut package = build_package();
        package.source_receipt_bytes_hex.push_str("00");
        assert_eq!(
            verify_package(&package, test_profile()),
            Err(ReceiverErrorV1::SourceReceiptDigestMismatch)
        );
    }

    #[test]
    fn generator_mutation_without_digest_update_is_rejected() {
        let mut package = build_package();
        package.generator_set[0][0][1] = -2.0;
        assert_eq!(
            verify_package(&package, test_profile()),
            Err(ReceiverErrorV1::GeneratorDigestMismatch)
        );
    }

    #[test]
    fn wrong_force_variance_is_rejected_even_if_other_laws_pass() {
        let mut package = build_package();
        package.physical_fixture.declared_force_covector_y[0] += 0.25;
        package.envelope.physical_problem.fixture_sha256 =
            canonical_fixture_hash(&package.envelope, &package.physical_fixture).unwrap();
        assert_eq!(
            verify_package(&package, test_profile()),
            Err(ReceiverErrorV1::WrongForceCovectorLaw {
                defect_bits: relative_vector_defect(
                    &transpose_apply(
                        &package.physical_fixture.transform_s,
                        &package.physical_fixture.force_covector_q
                    ),
                    &package.physical_fixture.declared_force_covector_y
                )
                .to_bits()
            })
        );
    }

    #[test]
    fn wrong_constraint_jacobian_is_rejected() {
        let mut package = build_package();
        package.physical_fixture.declared_constraint_jacobian_y[0][1] += 0.5;
        package.envelope.physical_problem.fixture_sha256 =
            canonical_fixture_hash(&package.envelope, &package.physical_fixture).unwrap();
        assert!(matches!(
            verify_package(&package, test_profile()),
            Err(ReceiverErrorV1::WrongConstraintJacobianLaw { .. })
        ));
    }

    #[test]
    fn nonisometric_generator_is_rejected() {
        let mut package = build_package();
        package.generator_set[0][0][1] = -2.0;
        let generator_hash = canonical_generators_hash(&package.generator_set).unwrap();
        let inverse_s =
            inverse_and_condition(&package.physical_fixture.transform_s, test_profile())
                .unwrap()
                .0;
        package.physical_fixture.declared_generators_y = package
            .generator_set
            .iter()
            .map(|generator| {
                multiply(
                    &multiply(&inverse_s, generator),
                    &package.physical_fixture.transform_s,
                )
            })
            .collect();
        package.envelope.physical_problem.fixture_sha256 =
            canonical_fixture_hash(&package.envelope, &package.physical_fixture).unwrap();
        package.envelope.source_math_receipt.generator_set_sha256 = generator_hash.clone();
        let mut receipt: Value =
            serde_json::from_slice(&decode_hex(&package.source_receipt_bytes_hex).unwrap())
                .unwrap();
        receipt["generator_set_sha256"] = Value::String(generator_hash);
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
        package.source_receipt_bytes_hex = encode_hex(&receipt_bytes);
        package.envelope.source_math_receipt.receipt_sha256 =
            hash_domain("luminous.source-math-receipt.raw-bytes.v1", &receipt_bytes);
        assert!(matches!(
            verify_package(&package, test_profile()),
            Err(ReceiverErrorV1::NonIsometricGenerator { .. })
        ));
    }

    #[test]
    fn task_objective_mutation_is_rejected() {
        let mut package = build_package();
        package.task_semantics.objective = "different-objective-v0.1".to_owned();
        assert_eq!(
            verify_package(&package, test_profile()),
            Err(ReceiverErrorV1::TaskSemanticDigestMismatch)
        );
    }

    #[test]
    fn singular_transform_fails_closed() {
        let mut package = build_package();
        package.physical_fixture.transform_s = [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 0.0, 1.0]];
        package
            .envelope
            .physical_problem
            .coordinate_change
            .transform_sha256 = canonical_matrix3_hash(
            "luminous.transform-s.v1",
            &package.physical_fixture.transform_s,
        )
        .unwrap();
        package.envelope.physical_problem.fixture_sha256 =
            canonical_fixture_hash(&package.envelope, &package.physical_fixture).unwrap();
        assert_eq!(
            verify_package(&package, test_profile()),
            Err(ReceiverErrorV1::SingularCoordinateChange { pivot: 1 })
        );
    }

    #[test]
    fn out_of_envelope_transform_returns_inconclusive_not_a_false_failure_or_pass() {
        let mut package = build_package();
        package.physical_fixture.transform_s =
            [[1.0e-5, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        package
            .envelope
            .physical_problem
            .coordinate_change
            .transform_sha256 = canonical_matrix3_hash(
            "luminous.transform-s.v1",
            &package.physical_fixture.transform_s,
        )
        .unwrap();
        package.envelope.physical_problem.fixture_sha256 =
            canonical_fixture_hash(&package.envelope, &package.physical_fixture).unwrap();

        let profile = ReceiverProfileV1 {
            maximum_transform_condition_inf: 100.0,
            ..test_profile()
        };
        let receipt = verify_package(&package, profile).unwrap();
        assert_eq!(receipt.payload.status, ObligationStatusV1::Inconclusive);
        assert_eq!(
            receipt.payload.obligations.mass_metric_congruence,
            ObligationStatusV1::Inconclusive
        );
        assert_eq!(
            receipt.payload.obligations.full_task_semantics,
            ObligationStatusV1::Pass
        );
        assert_eq!(receipt.payload.physical_equivalence_authority, "NONE");
        assert!(!receipt.payload.scientific_search_pruning_authorized);
    }

    #[test]
    fn transformed_metric_digest_is_bound_to_exact_recomputed_congruence() {
        let mut package = build_package();
        package
            .envelope
            .physical_problem
            .coordinate_change
            .transformed_mass_metric_sha256 = "0".repeat(64);
        package.envelope.physical_problem.fixture_sha256 =
            canonical_fixture_hash(&package.envelope, &package.physical_fixture).unwrap();
        assert_eq!(
            verify_package(&package, test_profile()),
            Err(ReceiverErrorV1::MassMetricDigestMismatch)
        );
    }

    #[test]
    fn source_cannot_self_promote_physical_or_pruning_authority() {
        let mut package = build_package();
        package.envelope.claims.physical_step_equivariance_status = "PASS".to_owned();
        package.envelope.claims.physical_equivalence_authority = "BOUNDED_STEP_MAP_ONLY".to_owned();
        assert_eq!(
            verify_package(&package, test_profile()),
            Err(ReceiverErrorV1::ClaimAlreadyPromoted)
        );
        package.envelope.claims.physical_step_equivariance_status = "INCONCLUSIVE".to_owned();
        package.envelope.claims.physical_equivalence_authority = "NONE".to_owned();
        package.envelope.claims.scientific_search_pruning_authorized = true;
        assert_eq!(
            verify_package(&package, test_profile()),
            Err(ReceiverErrorV1::ClaimAlreadyPromoted)
        );
    }

    #[test]
    fn package_json_roundtrip_is_stable() {
        let package = build_package();
        let bytes = serde_json::to_vec(&package).unwrap();
        let decoded: ReceiverPackageV1 = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            verify_package(&package, test_profile()).unwrap(),
            verify_package(&decoded, test_profile()).unwrap()
        );
    }

    #[test]
    fn byte_identical_package_produces_byte_identical_receipt() {
        let package = build_package();
        let first = verify_package(&package, test_profile()).unwrap();
        let second = verify_package(&package, test_profile()).unwrap();
        assert_eq!(first, second);
    }
}
