//! Geometry-aware six-axis wrench consistency and field identifiability.
//!
//! Each taxel field value is represented as local normal pressure plus two
//! orthogonal tangential traction components (Pa). The geometry maps these
//! local traction components into a common reference frame. Area integration
//! produces force (N); the moment arm about the declared wrench origin produces
//! moment (N·m).
//!
//! A six-axis wrench is an aggregate constraint. This module separately reports
//! native design rank, wrench rank, combined rank and scalar estimability.
//! Wrench closure never implies that every taxel is identifiable.
//!
//! Reference receipt IDs are references only. Consumers must resolve and
//! cryptographically/physically verify the underlying evidence. The v0.1
//! uncertainty check uses diagonal component standard uncertainties, not a
//! full covariance model.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    fn subtract(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct TaxelGeometry {
    pub taxel_id: String,
    /// Effective contact area in square metres.
    pub area_m2: f64,
    /// Application point expressed in the declared reference frame, metres.
    pub application_point_m: Vec3,
    /// Unit local surface normal expressed in the declared reference frame.
    pub normal: Vec3,
    /// Unit local tangent basis; tangent_u × tangent_v must equal normal.
    pub tangent_u: Vec3,
    pub tangent_v: Vec3,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct RigidFrameTransform3 {
    pub transform_digest: String,
    pub source_frame_digest: String,
    pub target_frame_digest: String,
    /// Proper rotation matrix, row-major.
    pub rotation_row_major: [f64; 9],
    /// Translation from source-frame origin to target-frame origin, metres.
    pub translation_m: Vec3,
}

impl RigidFrameTransform3 {
    fn validate(&self) -> Result<(), String> {
        if self.transform_digest.trim().is_empty()
            || self.source_frame_digest.trim().is_empty()
            || self.target_frame_digest.trim().is_empty()
            || self.source_frame_digest == self.target_frame_digest
            || !self.translation_m.is_finite()
            || self
                .rotation_row_major
                .iter()
                .any(|value| !value.is_finite())
        {
            return Err(
                "rigid frame transform has missing identities or non-finite values".to_owned(),
            );
        }

        let rotation = &self.rotation_row_major;
        let tolerance = 1.0e-9;
        for row in 0..3 {
            for other_row in 0..3 {
                let dot = (0..3)
                    .map(|column| rotation[row * 3 + column] * rotation[other_row * 3 + column])
                    .sum::<f64>();
                let expected = if row == other_row { 1.0 } else { 0.0 };
                if (dot - expected).abs() > tolerance {
                    return Err("frame transform rotation must be orthonormal".to_owned());
                }
            }
        }
        let determinant = rotation[0] * (rotation[4] * rotation[8] - rotation[5] * rotation[7])
            - rotation[1] * (rotation[3] * rotation[8] - rotation[5] * rotation[6])
            + rotation[2] * (rotation[3] * rotation[7] - rotation[4] * rotation[6]);
        if (determinant - 1.0).abs() > tolerance {
            return Err(
                "frame transform rotation must be proper (determinant +1), not a reflection"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn rotate_vector(&self, vector: Vec3) -> Vec3 {
        let r = &self.rotation_row_major;
        Vec3 {
            x: r[0] * vector.x + r[1] * vector.y + r[2] * vector.z,
            y: r[3] * vector.x + r[4] * vector.y + r[5] * vector.z,
            z: r[6] * vector.x + r[7] * vector.y + r[8] * vector.z,
        }
    }

    fn transform_point(&self, point: Vec3) -> Vec3 {
        self.rotate_vector(point).add(self.translation_m)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct TaxelGeometrySet {
    pub geometry_digest: String,
    pub reference_frame_digest: String,
    pub contact_boundary_digest: String,
    pub wrench_origin_m: Vec3,
    pub taxels: Vec<TaxelGeometry>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct WrenchOperator {
    pub geometry_digest: String,
    pub reference_frame_digest: String,
    pub contact_boundary_digest: String,
    pub taxel_ids: Vec<String>,
    pub parameter_count: usize,
    /// Row-major 6 × (3 * taxel_count) matrix. Rows are Fx,Fy,Fz,Mx,My,Mz.
    /// Columns per taxel are normal pressure, tangent-u traction, tangent-v traction.
    pub coefficients: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct WrenchVector {
    pub force_n: Vec3,
    pub moment_nm: Vec3,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct WrenchCovariance6 {
    /// Full symmetric positive-definite 6×6 covariance, row-major.
    /// Component order: Fx,Fy,Fz,Mx,My,Mz.
    pub matrix_row_major: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct WrenchObservation {
    pub force_n: Vec3,
    pub moment_nm: Vec3,
    pub covariance: WrenchCovariance6,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ReferenceIndependenceDisposition {
    IndependentlyCharacterized,
    SharedCommonCause,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct WrenchObservationBinding {
    pub sample_identity: String,
    pub native_observation_set_digest: String,
    pub wrench_observation_digest: String,
}

impl WrenchObservationBinding {
    fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("sample_identity", self.sample_identity.as_str()),
            (
                "native_observation_set_digest",
                self.native_observation_set_digest.as_str(),
            ),
            (
                "wrench_observation_digest",
                self.wrench_observation_digest.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(format!(
                    "wrench observation binding field {name} is missing"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum FieldCovarianceStructure {
    IndependentDiagonalComponents,
    CorrelatedOrUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum FieldWrenchReferenceRelationship {
    IndependentFromWrenchReference,
    SharedReferenceOrCalibration,
    Unresolved,
}

/// A field mean plus marginal standard uncertainties and the evidence for the
/// covariance assumption. V0.1 admits only independent diagonal component
/// uncertainty; unknown/correlated fields must not be silently diagonalized.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct TractionFieldEstimate {
    pub estimate_digest: String,
    pub sample_identity: String,
    pub native_observation_set_digest: String,
    pub local_traction_pa: Vec<f64>,
    pub standard_uncertainties_pa: Vec<f64>,
    pub uncertainty_method_digest: String,
    pub uncertainty_reference_ancestry_receipt_id: String,
    pub uncertainty_common_cause_analysis_receipt_id: String,
    pub covariance_structure: FieldCovarianceStructure,
    pub relationship_to_wrench_reference: FieldWrenchReferenceRelationship,
}

impl TractionFieldEstimate {
    fn validate(
        &self,
        operator: &WrenchOperator,
        binding: &WrenchObservationBinding,
    ) -> Result<(), String> {
        for (name, value) in [
            ("estimate_digest", self.estimate_digest.as_str()),
            ("sample_identity", self.sample_identity.as_str()),
            (
                "native_observation_set_digest",
                self.native_observation_set_digest.as_str(),
            ),
            (
                "uncertainty_method_digest",
                self.uncertainty_method_digest.as_str(),
            ),
            (
                "uncertainty_reference_ancestry_receipt_id",
                self.uncertainty_reference_ancestry_receipt_id.as_str(),
            ),
            (
                "uncertainty_common_cause_analysis_receipt_id",
                self.uncertainty_common_cause_analysis_receipt_id.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(format!("traction-field estimate field {name} is missing"));
            }
        }
        if self.sample_identity != binding.sample_identity {
            return Err(
                "traction-field estimate sample identity does not match the wrench sample"
                    .to_owned(),
            );
        }
        if self.native_observation_set_digest != binding.native_observation_set_digest {
            return Err(
                "traction-field estimate does not bind the exact native observation set".to_owned(),
            );
        }
        if self.local_traction_pa.len() != operator.parameter_count
            || self
                .local_traction_pa
                .iter()
                .any(|value| !value.is_finite())
        {
            return Err(
                "traction-field estimate must contain exactly three finite Pa components per taxel"
                    .to_owned(),
            );
        }
        if self.standard_uncertainties_pa.len() != operator.parameter_count
            || self
                .standard_uncertainties_pa
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err("traction-field standard uncertainties must contain one finite nonnegative value per field component".to_owned());
        }
        if self.covariance_structure != FieldCovarianceStructure::IndependentDiagonalComponents {
            return Err(
                "field covariance is correlated or unknown; diagonal propagation is not admissible"
                    .to_owned(),
            );
        }
        if self.relationship_to_wrench_reference
            != FieldWrenchReferenceRelationship::IndependentFromWrenchReference
        {
            return Err("field uncertainty shares or may share wrench-reference/calibration ancestry; cross-covariance is unresolved".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct WrenchReferenceEvidence {
    pub native_observation_set_digest: String,
    pub wrench_observation_digest: String,
    pub sample_identity: String,
    pub reference_frame_digest: String,
    pub contact_boundary_digest: String,
    pub reference_instrument_receipt_id: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub verifier_receipt_id: String,
    pub reference_independence: ReferenceIndependenceDisposition,
    pub timestamp_alignment_uncertainty_ns: u64,
    pub maximum_permitted_alignment_uncertainty_ns: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum WrenchConsistencyDisposition {
    ConsistentWithinDeclaredUncertainty,
    Mismatch,
    ReferenceChainUnresolved,
    GeometryOrFrameMismatch,
    SampleOrBoundaryMismatch,
    InvalidObservation,
    InvalidField,
    FieldUncertaintyUnresolved,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct WrenchConsistencyReport {
    pub disposition: WrenchConsistencyDisposition,
    pub predicted_wrench: Option<WrenchVector>,
    pub residual_force_n: Option<Vec3>,
    pub residual_moment_nm: Option<Vec3>,
    /// Whitened residual components from the covariance Cholesky factor.
    pub whitened_residual_components: Option<[f64; 6]>,
    /// Mahalanobis distance sqrt(rᵀ Σ⁻¹ r); threshold is supplied by evaluator.
    pub mahalanobis_distance: Option<f64>,
    pub accepted_maximum_mahalanobis_distance: f64,
    /// Row-major 6×6 contribution propagated from the tactile field covariance.
    pub field_uncertainty_contribution_covariance_row_major: Option<Vec<f64>>,
    /// Row-major 6×6 total residual covariance after independent-source addition.
    pub residual_covariance_row_major: Option<Vec<f64>>,
    pub reasons: Vec<String>,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct TaxelComponentEstimability {
    pub taxel_id: String,
    pub normal_pressure_estimable: bool,
    pub tangent_u_estimable: bool,
    pub tangent_v_estimable: bool,
    pub fully_estimable: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct WrenchIdentifiabilityReport {
    pub native_observation_rank: usize,
    pub wrench_constraint_rank: usize,
    pub combined_rank: usize,
    pub incremental_rank_from_wrench: usize,
    pub parameter_count: usize,
    pub native_null_space_dimension: usize,
    pub combined_null_space_dimension: usize,
    pub native_identifiable_scalar_count: usize,
    pub combined_identifiable_scalar_count: usize,
    pub taxels_fully_identifiable_native: usize,
    pub taxels_fully_identifiable_combined: usize,
    pub per_taxel: Vec<TaxelComponentEstimability>,
    pub claim_class: &'static str,
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn row_space_basis(
    matrix: &[f64],
    row_count: usize,
    column_count: usize,
    tolerance: f64,
) -> Vec<Vec<f64>> {
    let mut basis: Vec<Vec<f64>> = Vec::with_capacity(row_count.min(column_count));
    for row_index in 0..row_count {
        let start = row_index * column_count;
        let mut residual = matrix[start..start + column_count].to_vec();
        if residual.iter().any(|value| !value.is_finite()) {
            return Vec::new();
        }
        let original_norm = dot(&residual, &residual).sqrt();
        if original_norm <= f64::EPSILON {
            continue;
        }
        for value in &mut residual {
            *value /= original_norm;
        }
        for _ in 0..2 {
            for direction in &basis {
                let projection = dot(direction, &residual);
                for (value, basis_value) in residual.iter_mut().zip(direction) {
                    *value -= projection * basis_value;
                }
            }
        }
        let residual_norm = dot(&residual, &residual).sqrt();
        if residual_norm > tolerance {
            for value in &mut residual {
                *value /= residual_norm;
            }
            basis.push(residual);
        }
    }
    basis
}

fn scalar_estimable(
    basis: &[Vec<f64>],
    parameter_index: usize,
    parameter_count: usize,
    tolerance: f64,
) -> bool {
    let mut projected = vec![0.0; parameter_count];
    for direction in basis {
        let coefficient = direction[parameter_index];
        for (value, basis_value) in projected.iter_mut().zip(direction) {
            *value += coefficient * basis_value;
        }
    }
    let mut residual = vec![0.0; parameter_count];
    residual[parameter_index] = 1.0;
    for (value, projection) in residual.iter_mut().zip(projected) {
        *value -= projection;
    }
    dot(&residual, &residual).sqrt() <= tolerance
}

fn validate_geometry(geometry: &TaxelGeometrySet) -> Result<(), String> {
    if geometry.geometry_digest.trim().is_empty()
        || geometry.reference_frame_digest.trim().is_empty()
        || geometry.contact_boundary_digest.trim().is_empty()
    {
        return Err("geometry, frame and contact-boundary digests must be nonempty".to_owned());
    }
    if !geometry.wrench_origin_m.is_finite() || geometry.taxels.is_empty() {
        return Err("wrench origin must be finite and at least one taxel is required".to_owned());
    }
    let mut taxel_ids = BTreeSet::new();
    for taxel in &geometry.taxels {
        if taxel.taxel_id.trim().is_empty() || !taxel_ids.insert(&taxel.taxel_id) {
            return Err("taxel IDs must be nonempty and unique".to_owned());
        }
        if !taxel.area_m2.is_finite() || taxel.area_m2 <= 0.0 {
            return Err(format!(
                "taxel {} area must be finite and positive",
                taxel.taxel_id
            ));
        }
        if !taxel.application_point_m.is_finite()
            || !taxel.normal.is_finite()
            || !taxel.tangent_u.is_finite()
            || !taxel.tangent_v.is_finite()
        {
            return Err(format!(
                "taxel {} geometry contains non-finite values",
                taxel.taxel_id
            ));
        }
        let tolerance = 1.0e-6;
        if (taxel.normal.norm() - 1.0).abs() > tolerance
            || (taxel.tangent_u.norm() - 1.0).abs() > tolerance
            || (taxel.tangent_v.norm() - 1.0).abs() > tolerance
            || taxel.normal.dot(taxel.tangent_u).abs() > tolerance
            || taxel.normal.dot(taxel.tangent_v).abs() > tolerance
            || taxel.tangent_u.dot(taxel.tangent_v).abs() > tolerance
        {
            return Err(format!(
                "taxel {} local basis must be orthonormal and unit length",
                taxel.taxel_id
            ));
        }
        let handedness_error = taxel
            .tangent_u
            .cross(taxel.tangent_v)
            .subtract(taxel.normal)
            .norm();
        if handedness_error > tolerance {
            return Err(format!(
                "taxel {} local basis is not right-handed (u × v must equal normal)",
                taxel.taxel_id
            ));
        }
    }
    Ok(())
}

/// Transform as-built taxel geometry from its sensor frame into the
/// declared wrench/reference frame. Positions and wrench origin receive the
/// rigid translation; normals and tangent vectors receive rotation only.
/// The returned digest must be supplied by the evidence/geometry registry and
/// should bind source geometry, transform and exact transformed output.
pub fn transform_geometry_to_reference(
    local_geometry: &TaxelGeometrySet,
    transform: &RigidFrameTransform3,
    transformed_geometry_digest: &str,
) -> Result<TaxelGeometrySet, String> {
    validate_geometry(local_geometry)?;
    transform.validate()?;
    if local_geometry.reference_frame_digest != transform.source_frame_digest {
        return Err("local geometry frame does not match transform source frame".to_owned());
    }
    if transformed_geometry_digest.trim().is_empty() {
        return Err("transformed geometry digest must not be empty".to_owned());
    }
    let taxels = local_geometry
        .taxels
        .iter()
        .map(|taxel| TaxelGeometry {
            taxel_id: taxel.taxel_id.clone(),
            area_m2: taxel.area_m2,
            application_point_m: transform.transform_point(taxel.application_point_m),
            normal: transform.rotate_vector(taxel.normal),
            tangent_u: transform.rotate_vector(taxel.tangent_u),
            tangent_v: transform.rotate_vector(taxel.tangent_v),
        })
        .collect();

    let transformed = TaxelGeometrySet {
        geometry_digest: transformed_geometry_digest.to_owned(),
        reference_frame_digest: transform.target_frame_digest.clone(),
        contact_boundary_digest: local_geometry.contact_boundary_digest.clone(),
        wrench_origin_m: transform.transform_point(local_geometry.wrench_origin_m),
        taxels,
    };
    validate_geometry(&transformed)?;
    Ok(transformed)
}

/// Construct the geometry-aware map from local taxel traction in pascals to
/// six-axis wrench in newtons and newton-metres.
pub fn build_wrench_operator(geometry: &TaxelGeometrySet) -> Result<WrenchOperator, String> {
    validate_geometry(geometry)?;
    let parameter_count = geometry
        .taxels
        .len()
        .checked_mul(3)
        .ok_or_else(|| "taxel traction parameter count overflows usize".to_owned())?;
    let coefficient_count = parameter_count
        .checked_mul(6)
        .ok_or_else(|| "six-axis wrench operator size overflows usize".to_owned())?;
    let mut coefficients = vec![0.0; coefficient_count];

    for (taxel_index, taxel) in geometry.taxels.iter().enumerate() {
        let arm = taxel.application_point_m.subtract(geometry.wrench_origin_m);
        let local_basis = [taxel.normal, taxel.tangent_u, taxel.tangent_v];
        for (component_index, basis) in local_basis.iter().enumerate() {
            let parameter = taxel_index * 3 + component_index;
            let force_per_pascal = basis.scale(taxel.area_m2);
            let moment_per_pascal = arm.cross(force_per_pascal);
            let wrench_components = [
                force_per_pascal.x,
                force_per_pascal.y,
                force_per_pascal.z,
                moment_per_pascal.x,
                moment_per_pascal.y,
                moment_per_pascal.z,
            ];
            if wrench_components.iter().any(|value| !value.is_finite()) {
                return Err(format!(
                    "taxel {} produces non-finite force/moment coefficients",
                    taxel.taxel_id
                ));
            }
            for (row, component) in wrench_components.iter().enumerate() {
                coefficients[row * parameter_count + parameter] = *component;
            }
        }
    }

    let operator = WrenchOperator {
        geometry_digest: geometry.geometry_digest.clone(),
        reference_frame_digest: geometry.reference_frame_digest.clone(),
        contact_boundary_digest: geometry.contact_boundary_digest.clone(),
        taxel_ids: geometry
            .taxels
            .iter()
            .map(|taxel| taxel.taxel_id.clone())
            .collect(),
        parameter_count,
        coefficients,
    };
    validate_operator(&operator)?;
    Ok(operator)
}

fn validate_operator(operator: &WrenchOperator) -> Result<(), String> {
    if operator.parameter_count == 0
        || operator.taxel_ids.is_empty()
        || operator.parameter_count != operator.taxel_ids.len().saturating_mul(3)
        || operator.coefficients.len() != 6_usize.saturating_mul(operator.parameter_count)
        || operator.coefficients.iter().any(|value| !value.is_finite())
        || operator.geometry_digest.trim().is_empty()
        || operator.reference_frame_digest.trim().is_empty()
        || operator.contact_boundary_digest.trim().is_empty()
    {
        return Err(
            "wrench operator dimensions, coefficients or context digests are invalid".to_owned(),
        );
    }
    Ok(())
}

fn validate_field(operator: &WrenchOperator, local_traction_pa: &[f64]) -> Result<(), String> {
    validate_operator(operator)?;
    if local_traction_pa.len() != operator.parameter_count
        || local_traction_pa.iter().any(|value| !value.is_finite())
    {
        return Err(
            "local traction field must contain exactly three finite Pa components per taxel"
                .to_owned(),
        );
    }
    Ok(())
}

pub fn integrate_tactile_field(
    operator: &WrenchOperator,
    local_traction_pa: &[f64],
) -> Result<WrenchVector, String> {
    validate_field(operator, local_traction_pa)?;
    let mut values = [0.0; 6];
    for (row, value) in values.iter_mut().enumerate() {
        let start = row * operator.parameter_count;
        *value = dot(
            &operator.coefficients[start..start + operator.parameter_count],
            local_traction_pa,
        );
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err("integrated tactile wrench overflowed to a non-finite value".to_owned());
    }
    Ok(WrenchVector {
        force_n: Vec3 {
            x: values[0],
            y: values[1],
            z: values[2],
        },
        moment_nm: Vec3 {
            x: values[3],
            y: values[4],
            z: values[5],
        },
    })
}

impl WrenchReferenceEvidence {
    fn validate(
        &self,
        operator: &WrenchOperator,
        binding: &WrenchObservationBinding,
    ) -> Result<(), String> {
        for (name, value) in [
            (
                "native_observation_set_digest",
                self.native_observation_set_digest.as_str(),
            ),
            (
                "wrench_observation_digest",
                self.wrench_observation_digest.as_str(),
            ),
            ("sample_identity", self.sample_identity.as_str()),
            (
                "reference_frame_digest",
                self.reference_frame_digest.as_str(),
            ),
            (
                "contact_boundary_digest",
                self.contact_boundary_digest.as_str(),
            ),
            (
                "reference_instrument_receipt_id",
                self.reference_instrument_receipt_id.as_str(),
            ),
            (
                "reference_ancestry_receipt_id",
                self.reference_ancestry_receipt_id.as_str(),
            ),
            (
                "common_cause_analysis_receipt_id",
                self.common_cause_analysis_receipt_id.as_str(),
            ),
            ("verifier_receipt_id", self.verifier_receipt_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("wrench reference evidence field {name} is missing"));
            }
        }
        if self.reference_frame_digest != operator.reference_frame_digest
            || self.contact_boundary_digest != operator.contact_boundary_digest
        {
            return Err(
                "wrench reference frame/contact boundary does not match the geometry operator"
                    .to_owned(),
            );
        }
        binding.validate()?;
        if self.sample_identity != binding.sample_identity {
            return Err(
                "wrench evidence sample identity does not match the native tactile sample"
                    .to_owned(),
            );
        }
        if self.native_observation_set_digest != binding.native_observation_set_digest
            || self.wrench_observation_digest != binding.wrench_observation_digest
        {
            return Err(
                "wrench evidence does not bind the exact native and wrench observation sets"
                    .to_owned(),
            );
        }
        if self.reference_independence
            != ReferenceIndependenceDisposition::IndependentlyCharacterized
        {
            return Err(
                "wrench reference ancestry/common-cause status is not independently characterized"
                    .to_owned(),
            );
        }
        if self.timestamp_alignment_uncertainty_ns > self.maximum_permitted_alignment_uncertainty_ns
        {
            return Err(
                "wrench and tactile sample synchronization uncertainty exceeds the permitted bound"
                    .to_owned(),
            );
        }
        Ok(())
    }
}

fn cholesky_wrench_covariance(covariance: &WrenchCovariance6) -> Result<[[f64; 6]; 6], String> {
    if covariance.matrix_row_major.len() != 36
        || covariance
            .matrix_row_major
            .iter()
            .any(|value| !value.is_finite())
    {
        return Err("wrench covariance must contain 36 finite row-major values".to_owned());
    }
    let mut matrix = [[0.0; 6]; 6];
    for (row_index, matrix_row) in matrix.iter_mut().enumerate() {
        for (column_index, value) in matrix_row.iter_mut().enumerate() {
            *value = covariance.matrix_row_major[row_index * 6 + column_index];
            let transpose = covariance.matrix_row_major[column_index * 6 + row_index];
            let scale = value.abs().max(transpose.abs()).max(f64::MIN_POSITIVE);
            if (*value - transpose).abs() > 1.0e-12 * scale {
                return Err("wrench covariance must be symmetric".to_owned());
            }
        }
    }

    let mut lower = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..=row {
            let correction = (0..column)
                .map(|index| lower[row][index] * lower[column][index])
                .sum::<f64>();
            if row == column {
                let diagonal = matrix[row][row] - correction;
                if !diagonal.is_finite() || diagonal <= 0.0 {
                    return Err("wrench covariance must be positive definite".to_owned());
                }
                lower[row][column] = diagonal.sqrt();
            } else {
                lower[row][column] = (matrix[row][column] - correction) / lower[column][column];
            }
        }
    }
    Ok(lower)
}

fn propagate_diagonal_field_covariance(
    operator: &WrenchOperator,
    standard_uncertainties_pa: &[f64],
) -> Result<[[f64; 6]; 6], String> {
    validate_operator(operator)?;
    if standard_uncertainties_pa.len() != operator.parameter_count
        || standard_uncertainties_pa
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("field uncertainty vector has invalid dimensions or values".to_owned());
    }

    let mut contribution = [[0.0; 6]; 6];
    for (row, contribution_row) in contribution.iter_mut().enumerate() {
        for (column, contribution_cell) in contribution_row.iter_mut().enumerate() {
            let covariance = standard_uncertainties_pa
                .iter()
                .enumerate()
                .map(|(parameter, sigma)| {
                    let row_coefficient =
                        operator.coefficients[row * operator.parameter_count + parameter];
                    let column_coefficient =
                        operator.coefficients[column * operator.parameter_count + parameter];
                    row_coefficient * column_coefficient * sigma * sigma
                })
                .sum::<f64>();
            *contribution_cell = covariance;
        }
    }
    if contribution
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err("propagated field covariance contains non-finite values".to_owned());
    }
    Ok(contribution)
}

fn combine_wrench_covariances(
    reference: &WrenchCovariance6,
    field_contribution: &[[f64; 6]; 6],
) -> Result<WrenchCovariance6, String> {
    // Validate the reference covariance on its own; a field contribution must
    // not repair an invalid or indefinite reference covariance.
    cholesky_wrench_covariance(reference)?;
    let mut combined = vec![0.0; 36];
    for row in 0..6 {
        for column in 0..6 {
            combined[row * 6 + column] =
                reference.matrix_row_major[row * 6 + column] + field_contribution[row][column];
        }
    }
    let result = WrenchCovariance6 {
        matrix_row_major: combined,
    };
    cholesky_wrench_covariance(&result)?;
    Ok(result)
}

fn covariance_matrix_to_row_major(matrix: &[[f64; 6]; 6]) -> Vec<f64> {
    matrix.iter().flat_map(|row| row.iter().copied()).collect()
}

fn whiten_wrench_residual(lower: &[[f64; 6]; 6], residual: &[f64; 6]) -> [f64; 6] {
    let mut whitened = [0.0; 6];
    for row in 0..6 {
        let correction = (0..row)
            .map(|column| lower[row][column] * whitened[column])
            .sum::<f64>();
        whitened[row] = (residual[row] - correction) / lower[row][row];
    }
    whitened
}

pub fn assess_wrench_consistency(
    operator: &WrenchOperator,
    field: &TractionFieldEstimate,
    observed: &WrenchObservation,
    evidence: &WrenchReferenceEvidence,
    binding: &WrenchObservationBinding,
    maximum_mahalanobis_distance: f64,
) -> WrenchConsistencyReport {
    let base = |disposition, reasons| WrenchConsistencyReport {
        disposition,
        predicted_wrench: None,
        residual_force_n: None,
        residual_moment_nm: None,
        whitened_residual_components: None,
        mahalanobis_distance: None,
        accepted_maximum_mahalanobis_distance: maximum_mahalanobis_distance,
        field_uncertainty_contribution_covariance_row_major: None,
        residual_covariance_row_major: None,
        reasons,
        claim_class: "synthetic_wrench_consistency_and_identifiability_only",
    };
    if !maximum_mahalanobis_distance.is_finite() || maximum_mahalanobis_distance <= 0.0 {
        return base(
            WrenchConsistencyDisposition::InvalidObservation,
            vec!["maximum Mahalanobis-distance threshold must be finite and positive".to_owned()],
        );
    }
    if evidence.sample_identity != binding.sample_identity {
        return base(
            WrenchConsistencyDisposition::SampleOrBoundaryMismatch,
            vec![
                "wrench evidence sample identity does not match the native tactile sample"
                    .to_owned(),
            ],
        );
    }
    if let Err(error) = evidence.validate(operator, binding) {
        let geometry_or_sample = error.contains("frame/contact boundary");
        return base(
            if geometry_or_sample {
                WrenchConsistencyDisposition::GeometryOrFrameMismatch
            } else {
                WrenchConsistencyDisposition::ReferenceChainUnresolved
            },
            vec![error],
        );
    }
    if let Err(error) = field.validate(operator, binding) {
        return base(
            WrenchConsistencyDisposition::FieldUncertaintyUnresolved,
            vec![error],
        );
    }
    if !observed.force_n.is_finite() || !observed.moment_nm.is_finite() {
        return base(
            WrenchConsistencyDisposition::InvalidObservation,
            vec!["wrench force and moment observations must be finite".to_owned()],
        );
    }

    let field_contribution =
        match propagate_diagonal_field_covariance(operator, &field.standard_uncertainties_pa) {
            Ok(value) => value,
            Err(error) => {
                return base(
                    WrenchConsistencyDisposition::FieldUncertaintyUnresolved,
                    vec![error],
                );
            }
        };
    let residual_covariance =
        match combine_wrench_covariances(&observed.covariance, &field_contribution) {
            Ok(value) => value,
            Err(error) => {
                return base(
                    WrenchConsistencyDisposition::InvalidObservation,
                    vec![error],
                );
            }
        };
    let covariance_cholesky = match cholesky_wrench_covariance(&residual_covariance) {
        Ok(value) => value,
        Err(error) => {
            return base(
                WrenchConsistencyDisposition::InvalidObservation,
                vec![error],
            );
        }
    };
    let predicted = match integrate_tactile_field(operator, &field.local_traction_pa) {
        Ok(value) => value,
        Err(error) => {
            return base(WrenchConsistencyDisposition::InvalidField, vec![error]);
        }
    };
    let residual_force = predicted.force_n.subtract(observed.force_n);
    let residual_moment = predicted.moment_nm.subtract(observed.moment_nm);
    let raw_residual = [
        residual_force.x,
        residual_force.y,
        residual_force.z,
        residual_moment.x,
        residual_moment.y,
        residual_moment.z,
    ];
    let whitened = whiten_wrench_residual(&covariance_cholesky, &raw_residual);
    let distance = dot(&whitened, &whitened).sqrt();
    if whitened.iter().any(|value| !value.is_finite()) || !distance.is_finite() {
        return base(
            WrenchConsistencyDisposition::InvalidObservation,
            vec!["whitened wrench residual or Mahalanobis distance is non-finite".to_owned()],
        );
    }
    let disposition = if distance <= maximum_mahalanobis_distance {
        WrenchConsistencyDisposition::ConsistentWithinDeclaredUncertainty
    } else {
        WrenchConsistencyDisposition::Mismatch
    };
    WrenchConsistencyReport {
        disposition,
        predicted_wrench: Some(predicted),
        residual_force_n: Some(residual_force),
        residual_moment_nm: Some(residual_moment),
        whitened_residual_components: Some(whitened),
        mahalanobis_distance: Some(distance),
        accepted_maximum_mahalanobis_distance: maximum_mahalanobis_distance,
        field_uncertainty_contribution_covariance_row_major: Some(covariance_matrix_to_row_major(
            &field_contribution,
        )),
        residual_covariance_row_major: Some(residual_covariance.matrix_row_major),
        reasons: if disposition == WrenchConsistencyDisposition::Mismatch {
            vec!["integrated tactile field disagrees with the independent six-axis wrench beyond the combined field/reference full-covariance Mahalanobis threshold".to_owned()]
        } else {
            Vec::new()
        },
        claim_class: "synthetic_wrench_consistency_and_identifiability_only",
    }
}

/// Compare native sensor design information with an independently admitted
/// six-axis wrench constraint. The wrench matrix can contribute at most six
/// structural constraints; priors are deliberately not accepted as rows.
pub fn compare_field_identifiability(
    operator: &WrenchOperator,
    native_design_matrix: &[f64],
    native_row_count: usize,
    reference_evidence: &WrenchReferenceEvidence,
    binding: &WrenchObservationBinding,
    tolerance: f64,
) -> Result<WrenchIdentifiabilityReport, String> {
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err("identifiability tolerance must be finite and positive".to_owned());
    }
    validate_operator(operator)?;
    if !native_design_matrix
        .len()
        .is_multiple_of(operator.parameter_count)
        || native_design_matrix.len() != native_row_count.saturating_mul(operator.parameter_count)
        || native_design_matrix.iter().any(|value| !value.is_finite())
    {
        return Err("native design matrix dimensions or values are invalid".to_owned());
    }
    reference_evidence.validate(operator, binding)?;

    let parameter_count = operator.parameter_count;
    let native_basis = row_space_basis(
        native_design_matrix,
        native_row_count,
        parameter_count,
        tolerance,
    );
    let wrench_basis = row_space_basis(&operator.coefficients, 6, parameter_count, tolerance);
    let mut combined = native_design_matrix.to_vec();
    combined.extend_from_slice(&operator.coefficients);
    let combined_row_count = native_row_count + 6;
    let combined_basis = row_space_basis(&combined, combined_row_count, parameter_count, tolerance);

    let native_estimable: Vec<_> = (0..parameter_count)
        .map(|index| scalar_estimable(&native_basis, index, parameter_count, tolerance * 10.0))
        .collect();
    let combined_estimable: Vec<_> = (0..parameter_count)
        .map(|index| scalar_estimable(&combined_basis, index, parameter_count, tolerance * 10.0))
        .collect();

    let per_taxel: Vec<_> = operator
        .taxel_ids
        .iter()
        .enumerate()
        .map(|(taxel_index, taxel_id)| {
            let start = taxel_index * 3;
            let normal_pressure_estimable = combined_estimable[start];
            let tangent_u_estimable = combined_estimable[start + 1];
            let tangent_v_estimable = combined_estimable[start + 2];
            TaxelComponentEstimability {
                taxel_id: taxel_id.clone(),
                normal_pressure_estimable,
                tangent_u_estimable,
                tangent_v_estimable,
                fully_estimable: normal_pressure_estimable
                    && tangent_u_estimable
                    && tangent_v_estimable,
            }
        })
        .collect();

    let native_scalar_count = native_estimable.iter().filter(|value| **value).count();
    let combined_scalar_count = combined_estimable.iter().filter(|value| **value).count();
    Ok(WrenchIdentifiabilityReport {
        native_observation_rank: native_basis.len(),
        wrench_constraint_rank: wrench_basis.len(),
        combined_rank: combined_basis.len(),
        incremental_rank_from_wrench: combined_basis.len().saturating_sub(native_basis.len()),
        parameter_count,
        native_null_space_dimension: parameter_count.saturating_sub(native_basis.len()),
        combined_null_space_dimension: parameter_count.saturating_sub(combined_basis.len()),
        native_identifiable_scalar_count: native_scalar_count,
        combined_identifiable_scalar_count: combined_scalar_count,
        taxels_fully_identifiable_native: (0..operator.taxel_ids.len())
            .filter(|index| {
                native_estimable[index * 3..index * 3 + 3]
                    .iter()
                    .all(|value| *value)
            })
            .count(),
        taxels_fully_identifiable_combined: per_taxel
            .iter()
            .filter(|taxel| taxel.fully_estimable)
            .count(),
        per_taxel,
        claim_class: "synthetic_wrench_consistency_and_identifiability_only",
    })
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SyntheticWrenchExampleReport {
    pub consistency: WrenchConsistencyReport,
    pub identifiability: WrenchIdentifiabilityReport,
    pub example_sample_identity: String,
    pub claim_class: &'static str,
}

/// Reproducible small known-answer example for the CLI scorecard.
/// This is a three-taxel synthetic geometry and is not a physical sensor result.
pub fn run_synthetic_wrench_example() -> Result<SyntheticWrenchExampleReport, String> {
    let tangent_u = Vec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    let tangent_v = Vec3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    let normal = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    let geometry = TaxelGeometrySet {
        geometry_digest: "sha256:synthetic-wrench-geometry-3-taxel".to_owned(),
        reference_frame_digest: "sha256:synthetic-wrench-frame".to_owned(),
        contact_boundary_digest: "sha256:synthetic-contact-boundary".to_owned(),
        wrench_origin_m: Vec3::default(),
        taxels: [-0.02, 0.0, 0.02]
            .into_iter()
            .enumerate()
            .map(|(index, x)| TaxelGeometry {
                taxel_id: format!("taxel-{index}"),
                area_m2: 1.0e-4,
                application_point_m: Vec3 { x, y: 0.0, z: 0.0 },
                normal,
                tangent_u,
                tangent_v,
            })
            .collect(),
    };
    let operator = build_wrench_operator(&geometry)?;
    let field = vec![
        10_000.0, 1_000.0, 500.0, 12_000.0, 1_500.0, 750.0, 8_000.0, 2_000.0, 1_000.0,
    ];
    let predicted = integrate_tactile_field(&operator, &field)?;
    let standard_deviations = [0.01_f64, 0.01, 0.01, 0.001, 0.001, 0.001];
    let mut covariance = vec![0.0; 36];
    for index in 0..6 {
        covariance[index * 6 + index] = standard_deviations[index].powi(2);
    }
    let observed = WrenchObservation {
        force_n: predicted.force_n,
        moment_nm: predicted.moment_nm,
        covariance: WrenchCovariance6 {
            matrix_row_major: covariance,
        },
    };
    let binding = WrenchObservationBinding {
        sample_identity: "sample:synthetic-001".to_owned(),
        native_observation_set_digest: "sha256:synthetic-native-observations".to_owned(),
        wrench_observation_digest: "sha256:synthetic-wrench-observation".to_owned(),
    };
    let evidence = WrenchReferenceEvidence {
        native_observation_set_digest: "sha256:synthetic-native-observations".to_owned(),
        wrench_observation_digest: "sha256:synthetic-wrench-observation".to_owned(),
        sample_identity: "sample:synthetic-001".to_owned(),
        reference_frame_digest: "sha256:synthetic-wrench-frame".to_owned(),
        contact_boundary_digest: "sha256:synthetic-contact-boundary".to_owned(),
        reference_instrument_receipt_id: "receipt:synthetic-six-axis-reference".to_owned(),
        reference_ancestry_receipt_id: "receipt:synthetic-reference-ancestry".to_owned(),
        common_cause_analysis_receipt_id: "receipt:synthetic-common-cause".to_owned(),
        verifier_receipt_id: "receipt:synthetic-verifier".to_owned(),
        reference_independence: ReferenceIndependenceDisposition::IndependentlyCharacterized,
        timestamp_alignment_uncertainty_ns: 100_000,
        maximum_permitted_alignment_uncertainty_ns: 500_000,
    };
    let field_estimate = TractionFieldEstimate {
        estimate_digest: "sha256:synthetic-field-estimate".to_owned(),
        sample_identity: "sample:synthetic-001".to_owned(),
        native_observation_set_digest: "sha256:synthetic-native-observations".to_owned(),
        local_traction_pa: field.clone(),
        standard_uncertainties_pa: vec![25.0; field.len()],
        uncertainty_method_digest: "sha256:synthetic-field-uncertainty-method".to_owned(),
        uncertainty_reference_ancestry_receipt_id: "receipt:synthetic-field-uncertainty-ancestry"
            .to_owned(),
        uncertainty_common_cause_analysis_receipt_id: "receipt:synthetic-field-uncertainty-cca"
            .to_owned(),
        covariance_structure: FieldCovarianceStructure::IndependentDiagonalComponents,
        relationship_to_wrench_reference:
            FieldWrenchReferenceRelationship::IndependentFromWrenchReference,
    };
    let consistency = assess_wrench_consistency(
        &operator,
        &field_estimate,
        &observed,
        &evidence,
        &binding,
        3.0,
    );
    let mut native = vec![0.0; 3 * operator.parameter_count];
    for taxel in 0..3 {
        native[taxel * operator.parameter_count + taxel * 3] = 1.0;
    }
    let identifiability =
        compare_field_identifiability(&operator, &native, 3, &evidence, &binding, 1.0e-10)?;

    Ok(SyntheticWrenchExampleReport {
        consistency,
        identifiability,
        example_sample_identity: "sample:synthetic-001".to_owned(),
        claim_class: "synthetic_wrench_consistency_and_identifiability_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> TaxelGeometrySet {
        let tangent_u = Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        let tangent_v = Vec3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let normal = Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        TaxelGeometrySet {
            geometry_digest: "sha256:geometry-3-taxel".to_owned(),
            reference_frame_digest: "sha256:frame-sensorized-body".to_owned(),
            contact_boundary_digest: "sha256:contact-boundary-01".to_owned(),
            wrench_origin_m: Vec3::default(),
            taxels: [-0.02, 0.0, 0.02]
                .into_iter()
                .enumerate()
                .map(|(index, x)| TaxelGeometry {
                    taxel_id: format!("taxel-{index}"),
                    area_m2: 1.0e-4,
                    application_point_m: Vec3 { x, y: 0.0, z: 0.0 },
                    normal,
                    tangent_u,
                    tangent_v,
                })
                .collect(),
        }
    }

    fn binding() -> WrenchObservationBinding {
        WrenchObservationBinding {
            sample_identity: "sample:001".to_owned(),
            native_observation_set_digest: "sha256:native-observations".to_owned(),
            wrench_observation_digest: "sha256:wrench-observation".to_owned(),
        }
    }

    fn traction_field_estimate(values: &[f64]) -> TractionFieldEstimate {
        TractionFieldEstimate {
            estimate_digest: "sha256:traction-field-estimate".to_owned(),
            sample_identity: "sample:001".to_owned(),
            native_observation_set_digest: "sha256:native-observations".to_owned(),
            local_traction_pa: values.to_vec(),
            standard_uncertainties_pa: vec![10.0; values.len()],
            uncertainty_method_digest: "sha256:field-uncertainty-method".to_owned(),
            uncertainty_reference_ancestry_receipt_id: "receipt:field-uncertainty-ancestry"
                .to_owned(),
            uncertainty_common_cause_analysis_receipt_id: "receipt:field-uncertainty-cca"
                .to_owned(),
            covariance_structure: FieldCovarianceStructure::IndependentDiagonalComponents,
            relationship_to_wrench_reference:
                FieldWrenchReferenceRelationship::IndependentFromWrenchReference,
        }
    }

    fn evidence() -> WrenchReferenceEvidence {
        WrenchReferenceEvidence {
            native_observation_set_digest: "sha256:native-observations".to_owned(),
            wrench_observation_digest: "sha256:wrench-observation".to_owned(),
            sample_identity: "sample:001".to_owned(),
            reference_frame_digest: "sha256:frame-sensorized-body".to_owned(),
            contact_boundary_digest: "sha256:contact-boundary-01".to_owned(),
            reference_instrument_receipt_id: "receipt:ati-reference".to_owned(),
            reference_ancestry_receipt_id: "receipt:ati-ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:wrench-cca".to_owned(),
            verifier_receipt_id: "receipt:wrench-verifier".to_owned(),
            reference_independence: ReferenceIndependenceDisposition::IndependentlyCharacterized,
            timestamp_alignment_uncertainty_ns: 100_000,
            maximum_permitted_alignment_uncertainty_ns: 500_000,
        }
    }

    fn field() -> Vec<f64> {
        // [normal pressure, tangent-u traction, tangent-v traction] per taxel (Pa).
        vec![
            10_000.0, 1_000.0, 500.0, 12_000.0, 1_500.0, 750.0, 8_000.0, 2_000.0, 1_000.0,
        ]
    }

    fn observation_from_field(operator: &WrenchOperator, field: &[f64]) -> WrenchObservation {
        let wrench = integrate_tactile_field(operator, field).unwrap();
        let standard_deviations = [0.01_f64, 0.01, 0.01, 0.001, 0.001, 0.001];
        let mut covariance = vec![0.0; 36];
        for index in 0..6 {
            covariance[index * 6 + index] = standard_deviations[index].powi(2);
        }
        WrenchObservation {
            force_n: wrench.force_n,
            moment_nm: wrench.moment_nm,
            covariance: WrenchCovariance6 {
                matrix_row_major: covariance,
            },
        }
    }

    #[test]
    fn rigid_frame_transform_rotates_vectors_and_translates_points_and_origin() {
        let local = TaxelGeometrySet {
            geometry_digest: "sha256:local-geometry".to_owned(),
            reference_frame_digest: "sha256:sensor-frame".to_owned(),
            contact_boundary_digest: "sha256:contact-boundary-01".to_owned(),
            wrench_origin_m: Vec3 {
                x: 0.1,
                y: 0.0,
                z: 0.0,
            },
            taxels: vec![TaxelGeometry {
                taxel_id: "taxel-a".to_owned(),
                area_m2: 1.0e-4,
                application_point_m: Vec3 {
                    x: 0.02,
                    y: 0.0,
                    z: 0.0,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tangent_u: Vec3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                tangent_v: Vec3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            }],
        };
        let transform = RigidFrameTransform3 {
            transform_digest: "sha256:sensor-to-body-transform".to_owned(),
            source_frame_digest: "sha256:sensor-frame".to_owned(),
            target_frame_digest: "sha256:body-frame".to_owned(),
            // +90 degrees around z.
            rotation_row_major: [0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            translation_m: Vec3 {
                x: 0.5,
                y: -0.2,
                z: 0.1,
            },
        };
        let transformed =
            transform_geometry_to_reference(&local, &transform, "sha256:body-frame-geometry")
                .expect("rigid frame transform");

        let taxel = &transformed.taxels[0];
        assert!((taxel.application_point_m.x - 0.5).abs() < 1.0e-12);
        assert!((taxel.application_point_m.y - -0.18).abs() < 1.0e-12);
        assert!((taxel.application_point_m.z - 0.1).abs() < 1.0e-12);
        assert!((taxel.normal.z - 1.0).abs() < 1.0e-12);
        assert!((taxel.tangent_u.y - 1.0).abs() < 1.0e-12);
        assert!((taxel.tangent_v.x + 1.0).abs() < 1.0e-12);
        assert!((transformed.wrench_origin_m.x - 0.5).abs() < 1.0e-12);
        assert!((transformed.wrench_origin_m.y - -0.1).abs() < 1.0e-12);
        assert_eq!(transformed.reference_frame_digest, "sha256:body-frame");
        assert!(build_wrench_operator(&transformed).is_ok());
    }

    #[test]
    fn frame_transform_rejects_reflection_nonorthogonal_and_wrong_source_frame() {
        let local = geometry();
        let mut reflection = RigidFrameTransform3 {
            transform_digest: "sha256:reflection".to_owned(),
            source_frame_digest: "sha256:frame-sensorized-body".to_owned(),
            target_frame_digest: "sha256:world-frame".to_owned(),
            rotation_row_major: [-1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            translation_m: Vec3::default(),
        };
        assert!(
            transform_geometry_to_reference(&local, &reflection, "sha256:transformed").is_err()
        );

        reflection.rotation_row_major = [1.0, 0.2, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        assert!(
            transform_geometry_to_reference(&local, &reflection, "sha256:transformed").is_err()
        );

        reflection.rotation_row_major = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        reflection.source_frame_digest = "sha256:wrong-source".to_owned();
        assert!(
            transform_geometry_to_reference(&local, &reflection, "sha256:transformed").is_err()
        );
    }

    #[test]
    fn geometry_operator_integrates_pressure_and_shear_to_six_axis_wrench() {
        let operator = build_wrench_operator(&geometry()).expect("valid geometry");
        assert_eq!(operator.parameter_count, 9);
        assert_eq!(operator.coefficients.len(), 54);
        let wrench = integrate_tactile_field(&operator, &field()).expect("field integration");
        assert!((wrench.force_n.z - 3.0).abs() < 1.0e-12);
        assert!((wrench.force_n.x - 0.45).abs() < 1.0e-12);
        assert!((wrench.force_n.y - 0.225).abs() < 1.0e-12);
        assert!(wrench.moment_nm.x.abs() < 1.0e-12);
        assert!((wrench.moment_nm.y - 0.004).abs() < 1.0e-12);
        assert!((wrench.moment_nm.z - 0.001).abs() < 1.0e-12);
    }

    #[test]
    fn different_local_fields_can_have_the_same_resultant_wrench() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let first = field();
        let mut second = first.clone();
        second[1] += 1_000.0;
        second[4] -= 1_000.0;
        let first_wrench = integrate_tactile_field(&operator, &first).unwrap();
        let second_wrench = integrate_tactile_field(&operator, &second).unwrap();
        assert_eq!(first_wrench, second_wrench);
        assert_ne!(first, second);
    }

    #[test]
    fn six_axis_wrench_adds_rank_without_falsely_identifying_every_taxel() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let mut native = vec![0.0; 3 * operator.parameter_count];
        for taxel in 0..3 {
            native[taxel * operator.parameter_count + taxel * 3] = 1.0;
        }
        let report =
            compare_field_identifiability(&operator, &native, 3, &evidence(), &binding(), 1.0e-10)
                .expect("identifiability report");
        assert_eq!(report.native_observation_rank, 3);
        assert_eq!(report.wrench_constraint_rank, 5);
        assert_eq!(report.combined_rank, 6);
        assert_eq!(report.incremental_rank_from_wrench, 3);
        assert_eq!(report.parameter_count, 9);
        assert_eq!(report.combined_null_space_dimension, 3);
        assert_eq!(report.combined_identifiable_scalar_count, 3);
        assert_eq!(report.taxels_fully_identifiable_combined, 0);
        assert!(report.per_taxel.iter().all(|taxel| {
            taxel.normal_pressure_estimable
                && !taxel.tangent_u_estimable
                && !taxel.tangent_v_estimable
                && !taxel.fully_estimable
        }));
    }

    #[test]
    fn wrench_consistency_requires_independent_reference_and_exact_geometry() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let field = field();
        let observation = observation_from_field(&operator, &field);
        let report = assess_wrench_consistency(
            &operator,
            &traction_field_estimate(&field),
            &observation,
            &evidence(),
            &binding(),
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::ConsistentWithinDeclaredUncertainty
        );
        assert_eq!(report.mahalanobis_distance, Some(0.0));

        let mut shared = evidence();
        shared.reference_independence = ReferenceIndependenceDisposition::SharedCommonCause;
        let report = assess_wrench_consistency(
            &operator,
            &traction_field_estimate(&field),
            &observation,
            &shared,
            &binding(),
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::ReferenceChainUnresolved
        );

        let mut wrong_frame = evidence();
        wrong_frame.reference_frame_digest = "sha256:wrong-frame".to_owned();
        let report = assess_wrench_consistency(
            &operator,
            &traction_field_estimate(&field),
            &observation,
            &wrong_frame,
            &binding(),
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::GeometryOrFrameMismatch
        );
    }

    #[test]
    fn force_or_moment_disagreement_and_clock_misalignment_are_rejected() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let field = field();
        let mut observation = observation_from_field(&operator, &field);
        observation.force_n.z += 0.2;
        let report = assess_wrench_consistency(
            &operator,
            &traction_field_estimate(&field),
            &observation,
            &evidence(),
            &binding(),
            3.0,
        );
        assert_eq!(report.disposition, WrenchConsistencyDisposition::Mismatch);

        let mut bad_timing = evidence();
        bad_timing.timestamp_alignment_uncertainty_ns = 1_000_000;
        let report = assess_wrench_consistency(
            &operator,
            &traction_field_estimate(&field),
            &observation_from_field(&operator, &field),
            &bad_timing,
            &binding(),
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::ReferenceChainUnresolved
        );
    }

    #[test]
    fn full_covariance_changes_joint_wrench_residual_admission() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let field = field();
        let mut observation = observation_from_field(&operator, &field);
        observation.force_n.x += 0.01;
        observation.force_n.y += 0.01;
        // Correlated Fx/Fy uncertainty (rho = 0.5); covariance remains SPD.
        observation.covariance.matrix_row_major[1] = 0.00005;
        observation.covariance.matrix_row_major[6] = 0.00005;
        let report = assess_wrench_consistency(
            &operator,
            &traction_field_estimate(&field),
            &observation,
            &evidence(),
            &binding(),
            1.2,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::ConsistentWithinDeclaredUncertainty
        );
        let distance = report.mahalanobis_distance.expect("joint distance");
        assert!((distance - 1.143324).abs() < 1.0e-5);
    }

    #[test]
    fn malformed_wrench_covariance_is_rejected() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let field = field();
        let mut asymmetric = observation_from_field(&operator, &field);
        asymmetric.covariance.matrix_row_major[1] = 0.00001;
        let report = assess_wrench_consistency(
            &operator,
            &traction_field_estimate(&field),
            &asymmetric,
            &evidence(),
            &binding(),
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::InvalidObservation
        );

        let mut non_positive_definite = observation_from_field(&operator, &field);
        non_positive_definite.covariance.matrix_row_major[0] = 0.0;
        let report = assess_wrench_consistency(
            &operator,
            &traction_field_estimate(&field),
            &non_positive_definite,
            &evidence(),
            &binding(),
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::InvalidObservation
        );
    }

    #[test]
    fn tactile_field_covariance_is_propagated_into_the_six_axis_residual() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let field = field();
        let estimate = traction_field_estimate(&field);
        let observation = observation_from_field(&operator, &field);
        let report = assess_wrench_consistency(
            &operator,
            &estimate,
            &observation,
            &evidence(),
            &binding(),
            3.0,
        );
        let contribution = report
            .field_uncertainty_contribution_covariance_row_major
            .expect("field covariance contribution");
        let total = report
            .residual_covariance_row_major
            .expect("combined residual covariance");
        assert_eq!(contribution.len(), 36);
        assert_eq!(total.len(), 36);
        assert!(contribution.iter().any(|value| *value > 0.0));
        for index in 0..36 {
            assert!(
                (total[index]
                    - observation.covariance.matrix_row_major[index]
                    - contribution[index])
                    .abs()
                    < 1.0e-12
            );
        }
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::ConsistentWithinDeclaredUncertainty
        );
    }

    #[test]
    fn shared_or_unknown_field_uncertainty_cannot_be_added_as_independent_covariance() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let field = field();
        let observation = observation_from_field(&operator, &field);

        let mut shared = traction_field_estimate(&field);
        shared.relationship_to_wrench_reference =
            FieldWrenchReferenceRelationship::SharedReferenceOrCalibration;
        let shared_report = assess_wrench_consistency(
            &operator,
            &shared,
            &observation,
            &evidence(),
            &binding(),
            3.0,
        );
        assert_eq!(
            shared_report.disposition,
            WrenchConsistencyDisposition::FieldUncertaintyUnresolved
        );

        let mut correlated = traction_field_estimate(&field);
        correlated.covariance_structure = FieldCovarianceStructure::CorrelatedOrUnknown;
        let correlated_report = assess_wrench_consistency(
            &operator,
            &correlated,
            &observation,
            &evidence(),
            &binding(),
            3.0,
        );
        assert_eq!(
            correlated_report.disposition,
            WrenchConsistencyDisposition::FieldUncertaintyUnresolved
        );
    }

    #[test]
    fn field_uncertainty_must_bind_the_exact_native_sample_and_observation_set() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let field = field();
        let observation = observation_from_field(&operator, &field);
        let mut wrong_native = traction_field_estimate(&field);
        wrong_native.native_observation_set_digest = "sha256:different-native".to_owned();
        let report = assess_wrench_consistency(
            &operator,
            &wrong_native,
            &observation,
            &evidence(),
            &binding(),
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::FieldUncertaintyUnresolved
        );

        let mut wrong_sample = traction_field_estimate(&field);
        wrong_sample.sample_identity = "sample:different".to_owned();
        let report = assess_wrench_consistency(
            &operator,
            &wrong_sample,
            &observation,
            &evidence(),
            &binding(),
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::FieldUncertaintyUnresolved
        );
    }

    #[test]
    fn wrench_evidence_must_match_exact_native_sample_identity() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let field = field();
        let observation = observation_from_field(&operator, &field);
        let report = assess_wrench_consistency(
            &operator,
            &traction_field_estimate(&field),
            &observation,
            &evidence(),
            &WrenchObservationBinding {
                sample_identity: "sample:different".to_owned(),
                native_observation_set_digest: "sha256:native-observations".to_owned(),
                wrench_observation_digest: "sha256:wrench-observation".to_owned(),
            },
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::SampleOrBoundaryMismatch
        );
    }

    #[test]
    fn malformed_geometry_or_field_fails_closed() {
        let mut invalid_geometry = geometry();
        invalid_geometry.taxels[0].area_m2 = 0.0;
        assert!(build_wrench_operator(&invalid_geometry).is_err());

        let mut left_handed = geometry();
        left_handed.taxels[0].tangent_v = Vec3 {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        };
        assert!(build_wrench_operator(&left_handed).is_err());

        let operator = build_wrench_operator(&geometry()).unwrap();
        assert!(integrate_tactile_field(&operator, &[1.0, 2.0]).is_err());
    }
    #[test]
    fn geometry_operator_rejects_finite_inputs_that_overflow_force_or_moment_coefficients() {
        let mut huge_area = geometry();
        huge_area.taxels[0].area_m2 = f64::MAX;
        huge_area.taxels[0].application_point_m.x = 2.0;
        assert!(build_wrench_operator(&huge_area).is_err());

        let mut huge_moment_arm = geometry();
        huge_moment_arm.taxels[0].application_point_m.x = 1.0e308;
        huge_moment_arm.taxels[0].area_m2 = 1.0e308;
        assert!(build_wrench_operator(&huge_moment_arm).is_err());
    }

    #[test]
    fn forged_operator_and_nonfinite_integrated_wrench_fail_closed() {
        let valid = build_wrench_operator(&geometry()).expect("geometry");
        let mut malformed = valid.clone();
        malformed.coefficients.pop();
        assert!(integrate_tactile_field(&malformed, &field()).is_err());

        let mut overflowing = valid;
        overflowing.coefficients[0] = f64::MAX;
        let mut extreme_field = field();
        extreme_field[0] = 2.0;
        assert!(integrate_tactile_field(&overflowing, &extreme_field).is_err());
    }

    #[test]
    fn overflowed_whitened_residual_is_invalid_not_a_mismatch_or_pass() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let field = field();
        let mut observation = observation_from_field(&operator, &field);
        observation.force_n.z = -f64::MAX;
        observation.covariance.matrix_row_major = vec![0.0; 36];
        for index in 0..6 {
            observation.covariance.matrix_row_major[index * 6 + index] = f64::MIN_POSITIVE;
        }
        let report = assess_wrench_consistency(
            &operator,
            &traction_field_estimate(&field),
            &observation,
            &evidence(),
            &binding(),
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::InvalidObservation
        );
    }
}
