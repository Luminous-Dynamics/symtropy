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

    fn components(self) -> [f64; 3] {
        [self.x, self.y, self.z]
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

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct WrenchObservation {
    pub force_n: Vec3,
    pub moment_nm: Vec3,
    /// Per-axis one-standard-deviation uncertainty; diagonal approximation only.
    pub force_standard_uncertainty_n: Vec3,
    pub moment_standard_uncertainty_nm: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ReferenceIndependenceDisposition {
    IndependentlyCharacterized,
    SharedCommonCause,
    Unresolved,
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
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct WrenchConsistencyReport {
    pub disposition: WrenchConsistencyDisposition,
    pub predicted_wrench: Option<WrenchVector>,
    pub residual_force_n: Option<Vec3>,
    pub residual_moment_nm: Option<Vec3>,
    pub standardized_residual_components: Option<[f64; 6]>,
    pub maximum_absolute_standardized_residual: Option<f64>,
    pub accepted_threshold: f64,
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

/// Construct the geometry-aware map from local taxel traction in pascals to
/// six-axis wrench in newtons and newton-metres.
pub fn build_wrench_operator(geometry: &TaxelGeometrySet) -> Result<WrenchOperator, String> {
    validate_geometry(geometry)?;
    let parameter_count = geometry.taxels.len() * 3;
    let mut coefficients = vec![0.0; 6 * parameter_count];

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
            for row in 0..6 {
                coefficients[row * parameter_count + parameter] = wrench_components[row];
            }
        }
    }

    Ok(WrenchOperator {
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
    })
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
        expected_sample_identity: &str,
        native_observation_set_digest: &str,
        wrench_observation_digest: &str,
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
        if self.sample_identity != expected_sample_identity {
            return Err("wrench evidence sample identity does not match the native tactile sample".to_owned());
        }
        if self.native_observation_set_digest != native_observation_set_digest
            || self.wrench_observation_digest != wrench_observation_digest
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

pub fn assess_wrench_consistency(
    operator: &WrenchOperator,
    local_traction_pa: &[f64],
    observed: &WrenchObservation,
    evidence: &WrenchReferenceEvidence,
    expected_sample_identity: &str,
    native_observation_set_digest: &str,
    wrench_observation_digest: &str,
    maximum_absolute_standardized_residual: f64,
) -> WrenchConsistencyReport {
    let base = |disposition, reasons| WrenchConsistencyReport {
        disposition,
        predicted_wrench: None,
        residual_force_n: None,
        residual_moment_nm: None,
        standardized_residual_components: None,
        maximum_absolute_standardized_residual: None,
        accepted_threshold: maximum_absolute_standardized_residual,
        reasons,
        claim_class: "synthetic_wrench_consistency_and_identifiability_only",
    };
    if !maximum_absolute_standardized_residual.is_finite()
        || maximum_absolute_standardized_residual <= 0.0
    {
        return base(
            WrenchConsistencyDisposition::InvalidObservation,
            vec!["standardized residual threshold must be finite and positive".to_owned()],
        );
    }
    if evidence.sample_identity != expected_sample_identity {
        return base(
            WrenchConsistencyDisposition::SampleOrBoundaryMismatch,
            vec!["wrench evidence sample identity does not match the native tactile sample".to_owned()],
        );
    }
    if let Err(error) = evidence.validate(
        operator,
        expected_sample_identity,
        native_observation_set_digest,
        wrench_observation_digest,
    ) {
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
    if !observed.force_n.is_finite()
        || !observed.moment_nm.is_finite()
        || !observed.force_standard_uncertainty_n.is_finite()
        || !observed.moment_standard_uncertainty_nm.is_finite()
        || observed
            .force_standard_uncertainty_n
            .components()
            .iter()
            .any(|value| *value <= 0.0)
        || observed
            .moment_standard_uncertainty_nm
            .components()
            .iter()
            .any(|value| *value <= 0.0)
    {
        return base(
            WrenchConsistencyDisposition::InvalidObservation,
            vec!["wrench observation and per-axis standard uncertainties must be finite, with positive uncertainties".to_owned()],
        );
    }
    let predicted = match integrate_tactile_field(operator, local_traction_pa) {
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
    let sigma = [
        observed.force_standard_uncertainty_n.x,
        observed.force_standard_uncertainty_n.y,
        observed.force_standard_uncertainty_n.z,
        observed.moment_standard_uncertainty_nm.x,
        observed.moment_standard_uncertainty_nm.y,
        observed.moment_standard_uncertainty_nm.z,
    ];
    let mut standardized = [0.0; 6];
    for index in 0..6 {
        standardized[index] = raw_residual[index] / sigma[index];
    }
    let maximum = standardized
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let disposition = if maximum <= maximum_absolute_standardized_residual {
        WrenchConsistencyDisposition::ConsistentWithinDeclaredUncertainty
    } else {
        WrenchConsistencyDisposition::Mismatch
    };
    WrenchConsistencyReport {
        disposition,
        predicted_wrench: Some(predicted),
        residual_force_n: Some(residual_force),
        residual_moment_nm: Some(residual_moment),
        standardized_residual_components: Some(standardized),
        maximum_absolute_standardized_residual: Some(maximum),
        accepted_threshold: maximum_absolute_standardized_residual,
        reasons: if disposition == WrenchConsistencyDisposition::Mismatch {
            vec!["integrated tactile field disagrees with the independent six-axis wrench beyond the declared per-axis uncertainty threshold".to_owned()]
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
    native_observation_set_digest: &str,
    native_design_matrix: &[f64],
    native_row_count: usize,
    reference_evidence: &WrenchReferenceEvidence,
    wrench_observation_digest: &str,
    expected_sample_identity: &str,
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
    reference_evidence.validate(
        operator,
        expected_sample_identity,
        native_observation_set_digest,
        wrench_observation_digest,
    )?;

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
        WrenchObservation {
            force_n: wrench.force_n,
            moment_nm: wrench.moment_nm,
            force_standard_uncertainty_n: Vec3 {
                x: 0.01,
                y: 0.01,
                z: 0.01,
            },
            moment_standard_uncertainty_nm: Vec3 {
                x: 0.001,
                y: 0.001,
                z: 0.001,
            },
        }
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
        let report = compare_field_identifiability(
            &operator,
            "sha256:native-observations",
            &native,
            3,
            &evidence(),
            "sha256:wrench-observation",
            1.0e-10,
        )
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
            &field,
            &observation,
            &evidence(),
            "sample:001",
            "sha256:native-observations",
            "sha256:wrench-observation",
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::ConsistentWithinDeclaredUncertainty
        );
        assert_eq!(report.maximum_absolute_standardized_residual, Some(0.0));

        let mut shared = evidence();
        shared.reference_independence = ReferenceIndependenceDisposition::SharedCommonCause;
        let report = assess_wrench_consistency(
            &operator,
            &field,
            &observation,
            &shared,
            "sample:001",
            "sha256:native-observations",
            "sha256:wrench-observation",
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
            &field,
            &observation,
            &wrong_frame,
            "sample:001",
            "sha256:native-observations",
            "sha256:wrench-observation",
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
            &field,
            &observation,
            &evidence(),
            "sample:001",
            "sha256:native-observations",
            "sha256:wrench-observation",
            3.0,
        );
        assert_eq!(report.disposition, WrenchConsistencyDisposition::Mismatch);

        let mut bad_timing = evidence();
        bad_timing.timestamp_alignment_uncertainty_ns = 1_000_000;
        let report = assess_wrench_consistency(
            &operator,
            &field,
            &observation_from_field(&operator, &field),
            &bad_timing,
            "sample:001",
            "sha256:native-observations",
            "sha256:wrench-observation",
            3.0,
        );
        assert_eq!(
            report.disposition,
            WrenchConsistencyDisposition::ReferenceChainUnresolved
        );
    }

    #[test]
    fn wrench_evidence_must_match_exact_native_sample_identity() {
        let operator = build_wrench_operator(&geometry()).expect("geometry");
        let field = field();
        let observation = observation_from_field(&operator, &field);
        let report = assess_wrench_consistency(
            &operator,
            &field,
            &observation,
            &evidence(),
            "sample:different",
            "sha256:native-observations",
            "sha256:wrench-observation",
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
}
