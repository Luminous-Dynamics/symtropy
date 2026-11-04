// SPDX-License-Identifier: AGPL-3.0-or-later
//! Passivity-preserving interpolation for position-indexed WPT matrix atlases.
//!
//! The atlas never invents endpoint qualifications. It consumes matrices whose
//! passivity lower bounds and validation receipts have already been established
//! by an upstream matrix validator, then uses convex interpolation and a
//! separately validated segment residual to propagate those bounds.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "wpt-matrix-atlas-v1";
const MAX_COILS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatingPoint {
    pub frequency_hz: f64,
    pub temperature_c: f64,
    pub dc_bias_currents_a: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixSample {
    pub position_m: f64,
    pub matrix_h: Vec<Vec<f64>>,
    pub entrywise_absolute_bound_h: Vec<Vec<f64>>,
    /// Lower eigenvalue bound from the upstream physical-matrix validator.
    pub minimum_eigenvalue_lower_bound_h: f64,
    /// Upstream floating-point guard in henries, included in the claimed lower bound.
    pub numerical_tolerance_h: f64,
    pub matrix_digest: String,
    pub source_artifact_digest: String,
    pub validator_receipt_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtlasSegment {
    pub from_sample_index: usize,
    pub to_sample_index: usize,
    /// Frobenius-norm upper bound on the physical matrix's deviation from the
    /// convex interpolant over the full segment, not merely at its midpoint.
    pub interpolation_error_frobenius_bound_h: f64,
    pub error_bound_method: String,
    pub validation_receipt_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixAtlas {
    pub schema_version: String,
    pub atlas_id: String,
    pub geometry_digest: String,
    pub matrix_kind: String,
    pub coil_ids: Vec<String>,
    pub operating_point: OperatingPoint,
    pub samples: Vec<MatrixSample>,
    pub segments: Vec<AtlasSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterpolationResult {
    pub disposition: String,
    pub position_m: f64,
    pub matrix_kind: String,
    pub coil_ids: Vec<String>,
    pub matrix_h: Vec<Vec<f64>>,
    pub entrywise_absolute_bound_h: Vec<Vec<f64>>,
    pub from_sample_index: usize,
    pub to_sample_index: usize,
    pub interpolation_fraction: f64,
    pub interpolation_error_frobenius_bound_h: f64,
    pub minimum_eigenvalue_lower_bound_h: f64,
    pub source_receipt_digests: Vec<String>,
    pub proof_scope: String,
}

#[derive(Debug, Clone, Copy)]
pub struct RefinementPolicy {
    pub maximum_interpolation_error_frobenius_bound_h: f64,
    pub minimum_required_eigenvalue_margin_h: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefinementCandidate {
    pub segment_index: usize,
    pub position_m: f64,
    pub priority_score: f64,
    pub triggers: Vec<String>,
}

impl MatrixAtlas {
    /// Validate shape, identity, domain, matrix symmetry and evidence references.
    ///
    /// The receipt digests are references, not signatures. Consumers must verify
    /// the referenced upstream receipts independently before trusting an atlas.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err("schema_version must be wpt-matrix-atlas-v1".into());
        }
        if self.atlas_id.trim().is_empty() {
            return Err("atlas_id must be non-empty".into());
        }
        if !valid_sha256(&self.geometry_digest) {
            return Err("geometry_digest must be sha256:<64 lowercase hex>".into());
        }
        if self.matrix_kind != "SMALL_SIGNAL" && self.matrix_kind != "DIFFERENTIAL" {
            return Err("matrix_kind must be SMALL_SIGNAL or DIFFERENTIAL".into());
        }

        let n = self.coil_ids.len();
        if !(2..=MAX_COILS).contains(&n) {
            return Err("coil count must be in 2..=32".into());
        }
        let mut seen = std::collections::HashSet::with_capacity(n);
        for coil_id in &self.coil_ids {
            if coil_id.trim().is_empty() || !seen.insert(coil_id) {
                return Err("coil IDs must be non-empty and unique".into());
            }
        }

        if !self.operating_point.frequency_hz.is_finite()
            || self.operating_point.frequency_hz <= 0.0
            || !self.operating_point.temperature_c.is_finite()
            || self.operating_point.dc_bias_currents_a.len() != n
            || self
                .operating_point
                .dc_bias_currents_a
                .iter()
                .any(|value| !value.is_finite())
        {
            return Err("invalid operating point or DC-bias vector".into());
        }

        if self.samples.len() < 2 {
            return Err("atlas requires at least two matrix samples".into());
        }
        if self.segments.len() != self.samples.len() - 1 {
            return Err(
                "one interpolation segment is required between each adjacent sample".into(),
            );
        }

        for (index, sample) in self.samples.iter().enumerate() {
            if !sample.position_m.is_finite()
                || (index > 0 && sample.position_m <= self.samples[index - 1].position_m)
            {
                return Err("sample positions must be finite and strictly increasing".into());
            }
            if index > 0 && !(sample.position_m - self.samples[index - 1].position_m).is_finite() {
                return Err("adjacent sample position span must be finite".into());
            }
            if !sample.minimum_eigenvalue_lower_bound_h.is_finite()
                || sample.minimum_eigenvalue_lower_bound_h < 0.0
            {
                return Err(
                    "each sample must carry a finite non-negative passivity lower bound".into(),
                );
            }
            if !valid_sha256(&sample.matrix_digest)
                || !valid_sha256(&sample.source_artifact_digest)
                || !valid_sha256(&sample.validator_receipt_digest)
            {
                return Err(
                    "matrix, source and validator receipt digests must be sha256 values".into(),
                );
            }
            validate_matrix(
                &sample.matrix_h,
                &sample.entrywise_absolute_bound_h,
                n,
                sample.numerical_tolerance_h,
                sample.minimum_eigenvalue_lower_bound_h,
            )?;
        }

        for (index, segment) in self.segments.iter().enumerate() {
            if segment.from_sample_index != index || segment.to_sample_index != index + 1 {
                return Err("segments must connect adjacent samples in position order".into());
            }
            if !segment.interpolation_error_frobenius_bound_h.is_finite()
                || segment.interpolation_error_frobenius_bound_h < 0.0
            {
                return Err(
                    "segment interpolation error bound must be finite and non-negative".into(),
                );
            }
            if !matches!(
                segment.error_bound_method.as_str(),
                "HELD_OUT_DENSE_REFERENCE" | "ANALYTIC_DERIVATIVE_BOUND" | "FORMAL_INTERVAL_BOUND"
            ) {
                return Err("unsupported interpolation error-bound method".into());
            }
            if !valid_sha256(&segment.validation_receipt_digest) {
                return Err("segment validation receipt digest must be sha256".into());
            }
        }

        Ok(())
    }

    /// Interpolate the complete matrix inside the sampled domain only.
    ///
    /// Convex combinations preserve PSD. The lower eigenvalue margin is
    /// propagated from the already-qualified endpoints and reduced by the
    /// independently validated segment residual. A non-positive lower bound
    /// returns UNRESOLVED; the method never projects a matrix into the PSD cone.
    pub fn interpolate_at(&self, position_m: f64) -> Result<InterpolationResult, String> {
        self.validate()?;
        if !position_m.is_finite() {
            return Err("requested position must be finite".into());
        }
        let first = &self.samples[0];
        let last = self.samples.last().expect("validated non-empty samples");
        if position_m < first.position_m || position_m > last.position_m {
            return Err(
                "EXTRAPOLATION_REJECTED: requested position lies outside sampled domain".into(),
            );
        }

        if let Some(index) = self
            .samples
            .iter()
            .position(|sample| sample.position_m == position_m)
        {
            let sample = &self.samples[index];
            return Ok(InterpolationResult {
                disposition: "PASSIVE_CERTIFIED_CONDITIONAL".into(),
                position_m,
                matrix_kind: self.matrix_kind.clone(),
                coil_ids: self.coil_ids.clone(),
                matrix_h: sample.matrix_h.clone(),
                entrywise_absolute_bound_h: sample.entrywise_absolute_bound_h.clone(),
                from_sample_index: index,
                to_sample_index: index,
                interpolation_fraction: 0.0,
                interpolation_error_frobenius_bound_h: 0.0,
                minimum_eigenvalue_lower_bound_h: sample.minimum_eigenvalue_lower_bound_h,
                source_receipt_digests: vec![sample.validator_receipt_digest.clone()],
                proof_scope: "ENDPOINT_RECEIPT_REQUIRES_EXTERNAL_VERIFICATION".into(),
            });
        }

        let index = self
            .samples
            .windows(2)
            .position(|pair| position_m > pair[0].position_m && position_m < pair[1].position_m)
            .ok_or_else(|| {
                "requested position is not covered by an interpolation segment".to_string()
            })?;
        let left = &self.samples[index];
        let right = &self.samples[index + 1];
        let segment = &self.segments[index];
        let span = right.position_m - left.position_m;
        if !span.is_finite() || span <= 0.0 {
            return Err("interpolation segment span is not finite and positive".into());
        }
        let alpha = (position_m - left.position_m) / span;
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err("interpolation fraction is outside the closed segment".into());
        }
        let matrix_h = interpolate_matrix(&left.matrix_h, &right.matrix_h, alpha)?;
        let entrywise_absolute_bound_h = interpolate_matrix(
            &left.entrywise_absolute_bound_h,
            &right.entrywise_absolute_bound_h,
            alpha,
        )?;
        let endpoint_lower_bound = (1.0 - alpha) * left.minimum_eigenvalue_lower_bound_h
            + alpha * right.minimum_eigenvalue_lower_bound_h;
        let lower_bound = endpoint_lower_bound - segment.interpolation_error_frobenius_bound_h;
        if !lower_bound.is_finite() {
            return Err("interpolated eigenvalue lower bound overflow".into());
        }

        let disposition = if lower_bound < 0.0 {
            "UNRESOLVED"
        } else if segment.error_bound_method == "HELD_OUT_DENSE_REFERENCE" {
            "EMPIRICAL_MARGIN_POSITIVE"
        } else {
            "PASSIVE_CERTIFIED_CONDITIONAL"
        };
        let proof_scope = if segment.error_bound_method == "HELD_OUT_DENSE_REFERENCE" {
            "HELD_OUT_POSITIONS_ONLY_NOT_A_CONTINUOUS_INTERVAL_PROOF"
        } else {
            "CONDITIONAL_ON_EXTERNAL_VERIFICATION_OF_ENDPOINT_AND_SEGMENT_RECEIPTS"
        };

        Ok(InterpolationResult {
            disposition: disposition.into(),
            position_m,
            matrix_kind: self.matrix_kind.clone(),
            coil_ids: self.coil_ids.clone(),
            matrix_h,
            entrywise_absolute_bound_h,
            from_sample_index: index,
            to_sample_index: index + 1,
            interpolation_fraction: alpha,
            interpolation_error_frobenius_bound_h: segment.interpolation_error_frobenius_bound_h,
            minimum_eigenvalue_lower_bound_h: lower_bound,
            source_receipt_digests: vec![
                left.validator_receipt_digest.clone(),
                right.validator_receipt_digest.clone(),
                segment.validation_receipt_digest.clone(),
            ],
            proof_scope: proof_scope.into(),
        })
    }

    /// Return segment midpoints requiring new FEM/measurement samples under
    /// the caller's error and passivity-margin policy.
    pub fn refinement_candidates(&self, policy: RefinementPolicy) -> Result<Vec<f64>, String> {
        Ok(self
            .refinement_plan(policy)?
            .into_iter()
            .map(|candidate| candidate.position_m)
            .collect())
    }

    /// Rank candidate field-solve locations by normalized model-risk signals.
    ///
    /// V1 uses only the segment interpolation envelope and passivity margin;
    /// service sensitivity / value-of-information signals are intentionally
    /// left for a later benchmark with a validated circuit model.
    pub fn refinement_plan(
        &self,
        policy: RefinementPolicy,
    ) -> Result<Vec<RefinementCandidate>, String> {
        self.validate()?;
        if !policy
            .maximum_interpolation_error_frobenius_bound_h
            .is_finite()
            || policy.maximum_interpolation_error_frobenius_bound_h < 0.0
            || !policy.minimum_required_eigenvalue_margin_h.is_finite()
            || policy.minimum_required_eigenvalue_margin_h < 0.0
        {
            return Err("refinement policy bounds must be finite and non-negative".into());
        }

        let mut candidates = Vec::new();
        for (index, segment) in self.segments.iter().enumerate() {
            let left = &self.samples[index];
            let right = &self.samples[index + 1];
            let conservative_margin = left
                .minimum_eigenvalue_lower_bound_h
                .min(right.minimum_eigenvalue_lower_bound_h)
                - segment.interpolation_error_frobenius_bound_h;

            let error_ratio = if policy.maximum_interpolation_error_frobenius_bound_h == 0.0 {
                if segment.interpolation_error_frobenius_bound_h > 0.0 {
                    f64::MAX
                } else {
                    0.0
                }
            } else {
                segment.interpolation_error_frobenius_bound_h
                    / policy.maximum_interpolation_error_frobenius_bound_h
            };
            let margin_deficit = (policy.minimum_required_eigenvalue_margin_h
                - conservative_margin)
                .max(0.0);
            let endpoint_scale = left
                .minimum_eigenvalue_lower_bound_h
                .max(right.minimum_eigenvalue_lower_bound_h);
            let margin_scale = policy
                .minimum_required_eigenvalue_margin_h
                .max(0.01 * endpoint_scale)
                .max(1e-15);
            let margin_ratio = margin_deficit / margin_scale;

            let mut triggers = Vec::new();
            if segment.interpolation_error_frobenius_bound_h
                > policy.maximum_interpolation_error_frobenius_bound_h
            {
                triggers.push("INTERPOLATION_ERROR_EXCEEDED".into());
            }
            if conservative_margin < policy.minimum_required_eigenvalue_margin_h {
                triggers.push("PASSIVITY_MARGIN_BELOW_POLICY".into());
            }
            if !triggers.is_empty() {
                let priority_score = error_ratio.max(margin_ratio);
                if !priority_score.is_finite() {
                    return Err("refinement priority score overflow".into());
                }
                candidates.push(RefinementCandidate {
                    segment_index: index,
                    position_m: left.position_m + 0.5 * (right.position_m - left.position_m),
                    priority_score,
                    triggers,
                });
            }
        }
        candidates.sort_by(|left, right| {
            right
                .priority_score
                .total_cmp(&left.priority_score)
                .then_with(|| left.segment_index.cmp(&right.segment_index))
        });
        Ok(candidates)
    }
}

fn validate_matrix(
    matrix: &[Vec<f64>],
    bounds: &[Vec<f64>],
    n: usize,
    numerical_tolerance_h: f64,
    declared_minimum_eigenvalue_lower_bound_h: f64,
) -> Result<(), String> {
    if matrix.len() != n || bounds.len() != n {
        return Err("matrix and bound row counts must match coil count".into());
    }
    if !numerical_tolerance_h.is_finite() || numerical_tolerance_h < 0.0 {
        return Err("numerical tolerance must be finite and non-negative".into());
    }
    if !declared_minimum_eigenvalue_lower_bound_h.is_finite()
        || declared_minimum_eigenvalue_lower_bound_h < 0.0
    {
        return Err("declared passivity lower bound must be finite and non-negative".into());
    }
    for row in matrix {
        if row.len() != n {
            return Err("matrix must be square and match coil count".into());
        }
        for value in row {
            if !value.is_finite() {
                return Err("matrix entries must be finite".into());
            }
        }
    }
    for row in bounds {
        if row.len() != n {
            return Err("entrywise bounds must be square and match coil count".into());
        }
        if row.iter().any(|value| !value.is_finite() || *value < 0.0) {
            return Err("entrywise absolute bounds must be finite and non-negative".into());
        }
    }
    for (i, row) in matrix.iter().enumerate() {
        if row[i] <= bounds[i][i] {
            return Err("self-inductance lower bound must remain positive".into());
        }
        for (j, value) in row.iter().enumerate().skip(i + 1) {
            if *value != matrix[j][i] {
                return Err("matrix must be exactly symmetric before atlas interpolation".into());
            }
        }
    }
    let input_uncertainty_bound = bounds
        .iter()
        .flatten()
        .fold(0.0_f64, |norm, value| norm.hypot(*value));
    let (minimum_eigenvalue, eigensolver_residual_bound) = smallest_symmetric_eigenvalue(matrix)?;
    let independently_computed_lower_bound = minimum_eigenvalue
        - input_uncertainty_bound
        - eigensolver_residual_bound
        - numerical_tolerance_h;
    if !independently_computed_lower_bound.is_finite() {
        return Err("matrix eigenvalue lower bound is non-finite".into());
    }
    if independently_computed_lower_bound < 0.0 {
        return Err("matrix plus entrywise uncertainty is not proven positive semidefinite".into());
    }
    if declared_minimum_eigenvalue_lower_bound_h > independently_computed_lower_bound {
        return Err(
            "declared eigenvalue lower bound is more optimistic than the independently computed bound"
                .into(),
        );
    }
    Ok(())
}

// Index-based pair rotations are intrinsic to the cyclic Jacobi eigensolver.
// A local range-loop allowance keeps that numerical kernel readable.
#[allow(clippy::needless_range_loop)]
fn smallest_symmetric_eigenvalue(matrix: &[Vec<f64>]) -> Result<(f64, f64), String> {
    let n = matrix.len();
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if !scale.is_finite() {
        return Err("eigensolver: non-finite matrix scale".into());
    }
    if scale == 0.0 {
        return Ok((0.0, 0.0));
    }

    let mut values: Vec<Vec<f64>> = matrix
        .iter()
        .map(|row| row.iter().map(|value| value / scale).collect())
        .collect();
    let threshold = 16.0 * f64::EPSILON;
    let mut converged = false;
    for _sweep in 0..100 {
        let mut largest_off_diagonal = 0.0_f64;
        for p in 0..n {
            for q in (p + 1)..n {
                largest_off_diagonal = largest_off_diagonal.max(values[p][q].abs());
            }
        }
        if largest_off_diagonal <= threshold {
            converged = true;
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                let apq = values[p][q];
                if apq.abs() <= threshold {
                    continue;
                }
                let app = values[p][p];
                let aqq = values[q][q];
                let tau = (aqq - app) / (2.0 * apq);
                let sign = if tau >= 0.0 { 1.0 } else { -1.0 };
                let t = sign / (tau.abs() + tau.hypot(1.0));
                let cosine = 1.0 / (1.0 + t * t).sqrt();
                let sine = t * cosine;
                values[p][p] = app - t * apq;
                values[q][q] = aqq + t * apq;
                values[p][q] = 0.0;
                values[q][p] = 0.0;
                for k in 0..n {
                    if k == p || k == q {
                        continue;
                    }
                    let akp = values[k][p];
                    let akq = values[k][q];
                    let new_kp = cosine * akp - sine * akq;
                    let new_kq = sine * akp + cosine * akq;
                    values[k][p] = new_kp;
                    values[p][k] = new_kp;
                    values[k][q] = new_kq;
                    values[q][k] = new_kq;
                }
            }
        }
    }
    if !converged {
        let mut largest_off_diagonal = 0.0_f64;
        for p in 0..n {
            for q in (p + 1)..n {
                largest_off_diagonal = largest_off_diagonal.max(values[p][q].abs());
            }
        }
        if largest_off_diagonal > 1.0e-12 {
            return Err(format!(
                "eigensolver: Jacobi method did not converge (off-diagonal residual {largest_off_diagonal:e})"
            ));
        }
    }
    let minimum = (0..n)
        .map(|index| values[index][index])
        .fold(f64::INFINITY, f64::min);
    let residual_norm = values
        .iter()
        .enumerate()
        .flat_map(|(row_index, row)| {
            row.iter()
                .enumerate()
                .filter(move |(column_index, _)| *column_index != row_index)
                .map(|(_, value)| *value)
        })
        .fold(0.0_f64, |norm, value| norm.hypot(value));
    let eigenvalue = minimum * scale;
    let residual_bound = residual_norm * scale;
    if eigenvalue.is_finite() && residual_bound.is_finite() {
        Ok((eigenvalue, residual_bound))
    } else {
        Err("eigensolver: non-finite eigenvalue or residual".into())
    }
}

fn interpolate_matrix(
    left: &[Vec<f64>],
    right: &[Vec<f64>],
    alpha: f64,
) -> Result<Vec<Vec<f64>>, String> {
    left.iter()
        .zip(right)
        .map(|(left_row, right_row)| {
            left_row
                .iter()
                .zip(right_row)
                .map(|(left_value, right_value)| {
                    let value = (1.0 - alpha) * left_value + alpha * right_value;
                    if value.is_finite() {
                        Ok(value)
                    } else {
                        Err("interpolated matrix contains a non-finite value".into())
                    }
                })
                .collect()
        })
        .collect()
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> MatrixAtlas {
        let digest = |digit: char| format!("sha256:{}", digit.to_string().repeat(64));
        MatrixAtlas {
            schema_version: SCHEMA_VERSION.into(),
            atlas_id: "SYNTHETIC-2COIL-TRAJECTORY".into(),
            geometry_digest: digest('a'),
            matrix_kind: "SMALL_SIGNAL".into(),
            coil_ids: vec!["TX".into(), "RX".into()],
            operating_point: OperatingPoint {
                frequency_hz: 85_000.0,
                temperature_c: 25.0,
                dc_bias_currents_a: vec![0.0, 0.0],
            },
            samples: vec![
                MatrixSample {
                    position_m: 0.0,
                    matrix_h: vec![vec![10e-6, 1e-6], vec![1e-6, 8e-6]],
                    entrywise_absolute_bound_h: vec![vec![0.0; 2]; 2],
                    minimum_eigenvalue_lower_bound_h: 7.5e-6,
                    numerical_tolerance_h: 1e-12,
                    matrix_digest: digest('b'),
                    source_artifact_digest: digest('c'),
                    validator_receipt_digest: digest('d'),
                },
                MatrixSample {
                    position_m: 10.0,
                    matrix_h: vec![vec![9e-6, 2e-6], vec![2e-6, 9e-6]],
                    entrywise_absolute_bound_h: vec![vec![0.0; 2]; 2],
                    minimum_eigenvalue_lower_bound_h: 6.9e-6,
                    numerical_tolerance_h: 1e-12,
                    matrix_digest: digest('e'),
                    source_artifact_digest: digest('f'),
                    validator_receipt_digest: digest('1'),
                },
            ],
            segments: vec![AtlasSegment {
                from_sample_index: 0,
                to_sample_index: 1,
                interpolation_error_frobenius_bound_h: 0.2e-6,
                error_bound_method: "HELD_OUT_DENSE_REFERENCE".into(),
                validation_receipt_digest: digest('2'),
            }],
        }
    }

    #[test]
    fn convex_interpolation_preserves_qualified_passivity_margin() {
        let atlas = fixture();
        let result = atlas.interpolate_at(5.0).unwrap();
        assert_eq!(result.disposition, "EMPIRICAL_MARGIN_POSITIVE");
        assert!((result.matrix_h[0][0] - 9.5e-6).abs() < 1e-18);
        assert!((result.matrix_h[0][1] - 1.5e-6).abs() < 1e-18);
        assert!((result.matrix_h[1][1] - 8.5e-6).abs() < 1e-18);
        assert!((result.minimum_eigenvalue_lower_bound_h - 7.0e-6).abs() < 1e-18);
        assert_eq!(result.source_receipt_digests.len(), 3);
    }

    #[test]
    fn exact_sample_uses_its_own_qualification_receipt() {
        let atlas = fixture();
        let result = atlas.interpolate_at(10.0).unwrap();
        assert_eq!(result.disposition, "PASSIVE_CERTIFIED_CONDITIONAL");
        assert_eq!(result.from_sample_index, 1);
        assert_eq!(result.to_sample_index, 1);
        assert_eq!(result.interpolation_error_frobenius_bound_h, 0.0);
        assert_eq!(result.source_receipt_digests.len(), 1);
    }

    #[test]
    fn refuses_extrapolation_outside_the_sampled_geometry_domain() {
        let atlas = fixture();
        assert!(
            atlas
                .interpolate_at(-0.001)
                .unwrap_err()
                .contains("EXTRAPOLATION_REJECTED")
        );
        assert!(
            atlas
                .interpolate_at(10.001)
                .unwrap_err()
                .contains("EXTRAPOLATION_REJECTED")
        );
    }

    #[test]
    fn returns_unresolved_when_segment_error_consumes_psd_margin() {
        let mut atlas = fixture();
        atlas.segments[0].interpolation_error_frobenius_bound_h = 8e-6;
        let result = atlas.interpolate_at(5.0).unwrap();
        assert_eq!(result.disposition, "UNRESOLVED");
        assert!(result.minimum_eigenvalue_lower_bound_h < 0.0);
    }

    #[test]
    fn asks_for_refinement_when_error_or_margin_exceeds_policy() {
        let atlas = fixture();
        let candidates = atlas
            .refinement_candidates(RefinementPolicy {
                maximum_interpolation_error_frobenius_bound_h: 0.1e-6,
                minimum_required_eigenvalue_margin_h: 0.0,
            })
            .unwrap();
        assert_eq!(candidates, vec![5.0]);
    }

    #[test]
    fn refinement_plan_prioritizes_segments_with_larger_normalized_error() {
        let mut atlas = fixture();
        let mut third = atlas.samples[1].clone();
        third.position_m = 20.0;
        third.matrix_digest = format!("sha256:{}", "3".repeat(64));
        third.source_artifact_digest = format!("sha256:{}", "4".repeat(64));
        third.validator_receipt_digest = format!("sha256:{}", "5".repeat(64));
        atlas.samples.push(third);
        atlas.segments[0].interpolation_error_frobenius_bound_h = 0.3e-6;
        atlas.segments.push(AtlasSegment {
            from_sample_index: 1,
            to_sample_index: 2,
            interpolation_error_frobenius_bound_h: 0.6e-6,
            error_bound_method: "HELD_OUT_DENSE_REFERENCE".into(),
            validation_receipt_digest: format!("sha256:{}", "6".repeat(64)),
        });

        let plan = atlas
            .refinement_plan(RefinementPolicy {
                maximum_interpolation_error_frobenius_bound_h: 0.2e-6,
                minimum_required_eigenvalue_margin_h: 0.0,
            })
            .unwrap();
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].segment_index, 1);
        assert_eq!(plan[1].segment_index, 0);
        assert!(plan[0].priority_score > plan[1].priority_score);
        assert_eq!(plan[0].position_m, 15.0);
        assert_eq!(plan[1].position_m, 5.0);
        assert!(plan
            .iter()
            .all(|candidate| candidate.triggers.contains(&"INTERPOLATION_ERROR_EXCEEDED".into())));
    }

    #[test]
    fn rejects_asymmetric_or_nonfinite_matrix_samples() {
        let mut atlas = fixture();
        atlas.samples[0].matrix_h[0][1] = 3e-6;
        assert!(atlas.validate().unwrap_err().contains("symmetric"));
        let mut atlas = fixture();
        atlas.samples[0].matrix_h[0][1] = f64::NAN;
        assert!(atlas.validate().unwrap_err().contains("finite"));
    }

    #[test]
    fn rejects_missing_or_invalid_source_receipts() {
        let mut atlas = fixture();
        atlas.samples[0].validator_receipt_digest = "not-a-digest".into();
        assert!(atlas.validate().unwrap_err().contains("digests"));
    }

    #[test]
    fn rejects_nonpassive_endpoint_lower_bound() {
        let mut atlas = fixture();
        atlas.samples[1].minimum_eigenvalue_lower_bound_h = -1e-6;
        assert!(
            atlas
                .validate()
                .unwrap_err()
                .contains("passivity lower bound")
        );
    }

    #[test]
    fn rejects_duplicate_coil_ids_and_nonadjacent_segments() {
        let mut atlas = fixture();
        atlas.coil_ids[1] = "TX".into();
        assert!(atlas.validate().unwrap_err().contains("unique"));
        let mut atlas = fixture();
        atlas.segments[0].to_sample_index = 0;
        assert!(atlas.validate().unwrap_err().contains("adjacent"));
    }

    #[test]
    fn rejects_invalid_matrix_kinds_and_bound_values() {
        let mut atlas = fixture();
        atlas.matrix_kind = "SECANT".into();
        assert!(atlas.validate().unwrap_err().contains("matrix_kind"));
        let mut atlas = fixture();
        atlas.samples[0].entrywise_absolute_bound_h[0][1] = -1.0;
        assert!(atlas.validate().unwrap_err().contains("non-negative"));
    }

    #[test]
    fn rejects_globally_indefinite_endpoint_even_with_optimistic_receipt_margin() {
        let mut atlas = fixture();
        atlas.samples[0].matrix_h = vec![
            vec![1e-6, 0.9e-6, 0.9e-6],
            vec![0.9e-6, 1e-6, -0.9e-6],
            vec![0.9e-6, -0.9e-6, 1e-6],
        ];
        atlas.samples[0].entrywise_absolute_bound_h = vec![vec![0.0; 3]; 3];
        atlas.samples[0].minimum_eigenvalue_lower_bound_h = 0.1e-6;
        atlas.coil_ids = vec!["TX".into(), "RX-1".into(), "RX-2".into()];
        atlas.operating_point.dc_bias_currents_a = vec![0.0; 3];
        atlas.samples[1].matrix_h = vec![
            vec![1e-6, 0.0, 0.0],
            vec![0.0, 1e-6, 0.0],
            vec![0.0, 0.0, 1e-6],
        ];
        atlas.samples[1].entrywise_absolute_bound_h = vec![vec![0.0; 3]; 3];
        assert!(
            atlas
                .validate()
                .unwrap_err()
                .contains("not proven positive semidefinite")
        );
    }

    #[test]
    fn rejects_overstated_endpoint_eigenvalue_lower_bound() {
        let mut atlas = fixture();
        atlas.samples[0].minimum_eigenvalue_lower_bound_h = 9e-6;
        assert!(atlas.validate().unwrap_err().contains("more optimistic"));
    }

    #[test]
    fn rejects_endpoint_whose_declared_error_bounds_cross_psd_boundary() {
        let mut atlas = fixture();
        atlas.samples[0].entrywise_absolute_bound_h = vec![vec![0.0, 6e-6], vec![6e-6, 0.0]];
        assert!(
            atlas
                .validate()
                .unwrap_err()
                .contains("not proven positive semidefinite")
        );
    }

    #[test]
    fn rejects_an_optimistic_endpoint_eigenvalue_claim() {
        let mut atlas = fixture();
        atlas.samples[0].minimum_eigenvalue_lower_bound_h = 9.0e-6;
        assert!(
            atlas
                .validate()
                .unwrap_err()
                .contains("more optimistic than the independently computed bound")
        );
    }

    #[test]
    fn reports_dense_held_out_bounds_as_empirical_not_certified() {
        let atlas = fixture();
        let result = atlas.interpolate_at(5.0).unwrap();
        assert_eq!(result.disposition, "EMPIRICAL_MARGIN_POSITIVE");
        assert_eq!(
            result.proof_scope,
            "HELD_OUT_POSITIONS_ONLY_NOT_A_CONTINUOUS_INTERVAL_PROOF"
        );
    }

    #[test]
    fn only_formal_or_analytic_segment_bounds_receive_conditional_certificate_status() {
        let mut atlas = fixture();
        atlas.segments[0].error_bound_method = "FORMAL_INTERVAL_BOUND".into();
        let result = atlas.interpolate_at(5.0).unwrap();
        assert_eq!(result.disposition, "PASSIVE_CERTIFIED_CONDITIONAL");
        assert!(result.proof_scope.contains("EXTERNAL_VERIFICATION"));
    }

    #[test]
    fn rejects_non_psd_endpoint_even_when_digest_and_claim_are_present() {
        let mut atlas = fixture();
        atlas.samples[0].matrix_h = vec![vec![1e-6, 1.2e-6], vec![1.2e-6, 1e-6]];
        atlas.samples[0].minimum_eigenvalue_lower_bound_h = 0.0;
        assert!(
            atlas
                .validate()
                .unwrap_err()
                .contains("not proven positive semidefinite")
        );
    }
}
