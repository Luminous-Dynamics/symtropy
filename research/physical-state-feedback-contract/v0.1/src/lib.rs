// SPDX-License-Identifier: AGPL-3.0-or-later
//! Evidence-aware physical state feedback types.
//!
//! This dependency-free crate defines a minimal contract for queryable physical
//! state. It intentionally contains no engine implementation or serializer.
//! Adapters must preserve epistemic status, units, frames and uncertainty
//! semantics rather than silently inventing unavailable values.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const SCHEMA_VERSION: &str = "physical-state-feedback-v0.1";
const SHA256_HEX_LEN: usize = 64;
const QUATERNION_NORM_TOLERANCE: f64 = 1.0e-6;
// All matrix admission tolerances are dimensionless because validation
// first normalizes by the matrix's largest absolute entry.
const MATRIX_RELATIVE_TOLERANCE: f64 = 1.0e-10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationError {
    pub path: String,
    pub reason: String,
}

impl ValidationError {
    fn new(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
        }
    }
}

impl Display for ValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.reason)
    }
}

impl Error for ValidationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldStatus {
    Measured,
    Estimated,
    Predicted,
    Assumed,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Uncertainty {
    /// Uncertainty was not quantified. This is not a zero-width interval.
    NotQuantified {
        reason: String,
    },
    ScalarStdDev {
        standard_deviation: f64,
    },
    Interval {
        lower: f64,
        upper: f64,
    },
    Covariance3 {
        matrix: [[f64; 3]; 3],
        /// Product unit of the vector component units, e.g. m^2 or m^2/s^2.
        unit: String,
        /// Coordinate frame in which this covariance is expressed.
        frame_id: String,
    },
    EmpiricalSamples {
        values: Vec<f64>,
    },
}

impl Uncertainty {
    pub fn validate(&self, path: &str) -> Result<(), ValidationError> {
        match self {
            Self::NotQuantified { reason } => {
                if reason.trim().is_empty() {
                    return Err(ValidationError::new(
                        path,
                        "NotQuantified requires a non-empty reason",
                    ));
                }
            }
            Self::ScalarStdDev { standard_deviation } => {
                if !standard_deviation.is_finite() || *standard_deviation < 0.0 {
                    return Err(ValidationError::new(
                        path,
                        "standard deviation must be finite and non-negative",
                    ));
                }
            }
            Self::Interval { lower, upper } => {
                if !lower.is_finite() || !upper.is_finite() || lower > upper {
                    return Err(ValidationError::new(
                        path,
                        "interval bounds must be finite and lower <= upper",
                    ));
                }
            }
            Self::Covariance3 {
                matrix,
                unit,
                frame_id,
            } => {
                if unit.trim().is_empty() || frame_id.trim().is_empty() {
                    return Err(ValidationError::new(
                        path,
                        "covariance requires an explicit unit and coordinate frame",
                    ));
                }
                validate_symmetric_psd3(matrix, path)?;
            }
            Self::EmpiricalSamples { values } => {
                if values.len() < 2 || values.iter().any(|value| !value.is_finite()) {
                    return Err(ValidationError::new(
                        path,
                        "empirical uncertainty requires at least two finite samples",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn matrix3_max_abs(matrix: &[[f64; 3]; 3]) -> f64 {
    matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |scale, value| scale.max(value.abs()))
}

fn normalize_matrix3(matrix: &[[f64; 3]; 3], scale: f64) -> [[f64; 3]; 3] {
    std::array::from_fn(|row| std::array::from_fn(|column| matrix[row][column] / scale))
}

fn validate_symmetric_psd3(matrix: &[[f64; 3]; 3], path: &str) -> Result<(), ValidationError> {
    for row in matrix {
        if row.iter().any(|value| !value.is_finite()) {
            return Err(ValidationError::new(path, "matrix entries must be finite"));
        }
    }

    // Positive scaling does not change symmetry or positive-semidefinite
    // admissibility. Normalize before evaluating minors so their tolerances
    // do not carry the matrix's physical units or overflow at large scales.
    let scale = matrix3_max_abs(matrix);
    if scale == 0.0 {
        return Ok(());
    }
    let normalized = normalize_matrix3(matrix, scale);

    for (i, row) in normalized.iter().enumerate() {
        if row[i] < -MATRIX_RELATIVE_TOLERANCE {
            return Err(ValidationError::new(
                path,
                "matrix diagonal must be non-negative",
            ));
        }
        for (j, _) in row.iter().enumerate().skip(i + 1) {
            if (normalized[i][j] - normalized[j][i]).abs() > MATRIX_RELATIVE_TOLERANCE {
                return Err(ValidationError::new(path, "matrix must be symmetric"));
            }
            let minor = normalized[i][i] * normalized[j][j] - normalized[i][j] * normalized[j][i];
            if minor < -MATRIX_RELATIVE_TOLERANCE {
                return Err(ValidationError::new(
                    path,
                    "matrix must be positive semidefinite",
                ));
            }
        }
    }
    let determinant = normalized[0][0]
        * (normalized[1][1] * normalized[2][2] - normalized[1][2] * normalized[2][1])
        - normalized[0][1]
            * (normalized[1][0] * normalized[2][2] - normalized[1][2] * normalized[2][0])
        + normalized[0][2]
            * (normalized[1][0] * normalized[2][1] - normalized[1][1] * normalized[2][0]);
    if determinant < -MATRIX_RELATIVE_TOLERANCE {
        return Err(ValidationError::new(
            path,
            "matrix must be positive semidefinite",
        ));
    }
    Ok(())
}

/// Validate the rigid-body inertia tensor's principal-moment triangle inequality.
fn validate_physical_inertia3(matrix: &[[f64; 3]; 3], path: &str) -> Result<(), ValidationError> {
    validate_symmetric_psd3(matrix, path)?;
    let scale = matrix3_max_abs(matrix);
    if scale == 0.0 {
        return Ok(());
    }
    // Eigenvalue ratios and the physical inertia triangle inequality are
    // invariant under positive scale. Diagonalize the normalized tensor so
    // both the Jacobi stopping threshold and triangle tolerance are relative.
    let mut diagonalized = normalize_matrix3(matrix, scale);
    for _ in 0..24 {
        let candidates = [(0usize, 1usize), (0usize, 2usize), (1usize, 2usize)];
        let (p, q) = candidates
            .into_iter()
            .max_by(|(p1, q1), (p2, q2)| {
                diagonalized[*p1][*q1]
                    .abs()
                    .total_cmp(&diagonalized[*p2][*q2].abs())
            })
            .expect("fixed candidate set is non-empty");
        let off_diagonal = diagonalized[p][q];
        if off_diagonal.abs() <= MATRIX_RELATIVE_TOLERANCE {
            break;
        }

        let tau = (diagonalized[q][q] - diagonalized[p][p]) / (2.0 * off_diagonal);
        let sign = if tau >= 0.0 { 1.0 } else { -1.0 };
        let tangent = sign / (tau.abs() + (1.0 + tau * tau).sqrt());
        let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
        let sine = tangent * cosine;

        let pp = diagonalized[p][p];
        let qq = diagonalized[q][q];
        let mut column_p = [0.0; 3];
        let mut column_q = [0.0; 3];
        column_p[p] = pp - tangent * off_diagonal;
        column_q[q] = qq + tangent * off_diagonal;

        for (k, row) in diagonalized.iter().enumerate() {
            if k != p && k != q {
                let kp = row[p];
                let kq = row[q];
                column_p[k] = cosine * kp - sine * kq;
                column_q[k] = sine * kp + cosine * kq;
            }
        }

        let mut rotated = diagonalized;
        for (row, (value_p, value_q)) in rotated
            .iter_mut()
            .zip(column_p.iter().copied().zip(column_q.iter().copied()))
        {
            row[p] = value_p;
            row[q] = value_q;
        }
        rotated[p] = column_p;
        rotated[q] = column_q;
        diagonalized = rotated;
    }

    // A bounded iteration count is not itself a convergence certificate.
    // Refuse to classify the inertia tensor if Jacobi's residual is still
    // above the declared dimensionless tolerance.
    let maximum_off_diagonal = [
        diagonalized[0][1].abs(),
        diagonalized[0][2].abs(),
        diagonalized[1][2].abs(),
    ]
    .into_iter()
    .fold(0.0_f64, f64::max);
    if !maximum_off_diagonal.is_finite() || maximum_off_diagonal > MATRIX_RELATIVE_TOLERANCE {
        return Err(ValidationError::new(
            path,
            "principal-moment eigensolve did not converge to the relative tolerance",
        ));
    }

    let mut principal = [diagonalized[0][0], diagonalized[1][1], diagonalized[2][2]];
    principal.sort_by(f64::total_cmp);
    let largest = principal[2].max(0.0);
    let lower_two_sum = principal[0].max(0.0) + principal[1].max(0.0);
    let tolerance = MATRIX_RELATIVE_TOLERANCE.max(largest * 1.0e-8);
    if largest > lower_two_sum + tolerance {
        return Err(ValidationError::new(
            path,
            "principal moments violate the rigid-body inertia triangle inequality",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Provenance {
    pub source_sha256: Option<String>,
    pub model_build_sha256: Option<String>,
    pub synthetic_fixture_id: Option<String>,
    pub derivation: Option<String>,
}

impl Provenance {
    fn validate(&self, status: FieldStatus, path: &str) -> Result<(), ValidationError> {
        for (name, digest) in [
            ("source_sha256", self.source_sha256.as_deref()),
            ("model_build_sha256", self.model_build_sha256.as_deref()),
        ] {
            if let Some(digest) = digest {
                validate_sha256(digest, &format!("{path}.{name}"))?;
            }
        }
        if self
            .synthetic_fixture_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
            || self
                .derivation
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(ValidationError::new(
                path,
                "fixture ID and derivation, when present, must be non-empty",
            ));
        }
        if status != FieldStatus::Unavailable
            && self.source_sha256.is_none()
            && self.model_build_sha256.is_none()
            && self.synthetic_fixture_id.is_none()
            && self.derivation.is_none()
        {
            return Err(ValidationError::new(
                path,
                "available values require source, model, fixture or derivation provenance",
            ));
        }
        match status {
            FieldStatus::Measured
                if self.source_sha256.is_none() && self.synthetic_fixture_id.is_none() =>
            {
                return Err(ValidationError::new(
                    path,
                    "MEASURED values require a source digest or explicit synthetic fixture ID",
                ));
            }
            FieldStatus::Predicted
                if self.model_build_sha256.is_none() && self.synthetic_fixture_id.is_none() =>
            {
                return Err(ValidationError::new(
                    path,
                    "PREDICTED values require a model build digest or explicit synthetic fixture ID",
                ));
            }
            FieldStatus::Estimated
                if self.derivation.is_none()
                    || (self.source_sha256.is_none()
                        && self.model_build_sha256.is_none()
                        && self.synthetic_fixture_id.is_none()) =>
            {
                return Err(ValidationError::new(
                    path,
                    "ESTIMATED values require a derivation and source/model/fixture provenance",
                ));
            }
            FieldStatus::Assumed
                if self.derivation.is_none() && self.synthetic_fixture_id.is_none() =>
            {
                return Err(ValidationError::new(
                    path,
                    "ASSUMED values require a derivation or explicit synthetic fixture ID",
                ));
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldMetadata {
    pub status: FieldStatus,
    pub unit: String,
    pub frame_id: Option<String>,
    pub uncertainty: Option<Uncertainty>,
    pub provenance: Provenance,
    pub unavailable_reason: Option<String>,
}

impl FieldMetadata {
    fn validate(&self, path: &str) -> Result<(), ValidationError> {
        if self.unit.trim().is_empty() {
            return Err(ValidationError::new(path, "unit must be explicit"));
        }
        if self
            .frame_id
            .as_deref()
            .is_some_and(|frame| frame.trim().is_empty())
        {
            return Err(ValidationError::new(
                path,
                "frame ID must be non-empty when present",
            ));
        }
        self.provenance.validate(self.status, path)?;
        if let Some(Uncertainty::Covariance3 { unit, frame_id, .. }) = &self.uncertainty {
            let expected_unit = match self.unit.as_str() {
                "m" => "m^2",
                "m/s" => "m^2/s^2",
                "rad/s" => "rad^2/s^2",
                "m/s^2" => "m^2/s^4",
                "rad" => "rad^2",
                other => {
                    return Err(ValidationError::new(
                        path,
                        format!("Covariance3 is not defined for field unit {other:?} in v0.1"),
                    ));
                }
            };
            if unit != expected_unit {
                return Err(ValidationError::new(
                    path,
                    format!(
                        "covariance unit must be {expected_unit:?} for field unit {:?}",
                        self.unit
                    ),
                ));
            }
            if self.frame_id.as_deref() != Some(frame_id.as_str()) {
                return Err(ValidationError::new(
                    path,
                    "covariance frame must match the field frame",
                ));
            }
        }
        match self.status {
            FieldStatus::Unavailable => {
                if self.unavailable_reason.as_deref().is_none_or(str::is_empty) {
                    return Err(ValidationError::new(
                        path,
                        "UNAVAILABLE requires an explicit reason",
                    ));
                }
                if self.uncertainty.is_some() {
                    return Err(ValidationError::new(
                        path,
                        "UNAVAILABLE fields cannot claim quantified uncertainty",
                    ));
                }
            }
            _ => {
                if self.unavailable_reason.is_some() {
                    return Err(ValidationError::new(
                        path,
                        "available fields must not carry unavailable_reason",
                    ));
                }
                self.uncertainty
                    .as_ref()
                    .ok_or_else(|| {
                        ValidationError::new(
                            path,
                            "available numeric fields require quantified or explicitly NotQuantified uncertainty",
                        )
                    })?
                    .validate(&format!("{path}.uncertainty"))?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldValue<T> {
    pub value: Option<T>,
    pub metadata: FieldMetadata,
}

impl<T> FieldValue<T> {
    pub fn validate(&self, path: &str) -> Result<(), ValidationError> {
        self.metadata.validate(path)?;
        match (self.metadata.status, self.value.is_some()) {
            (FieldStatus::Unavailable, false) => Ok(()),
            (FieldStatus::Unavailable, true) => Err(ValidationError::new(
                path,
                "UNAVAILABLE must not contain a fabricated/default value",
            )),
            (_, true) => Ok(()),
            (_, false) => Err(ValidationError::new(
                path,
                "available field status requires a value",
            )),
        }
    }

    pub fn validate_expected_unit(
        &self,
        path: &str,
        expected_unit: &str,
    ) -> Result<(), ValidationError> {
        self.validate(path)?;
        if self.metadata.unit != expected_unit {
            return Err(ValidationError::new(
                path,
                format!(
                    "expected canonical unit {expected_unit:?}, got {:?}",
                    self.metadata.unit
                ),
            ));
        }
        Ok(())
    }

    pub fn validate_frame(&self, path: &str, expected_frame: &str) -> Result<(), ValidationError> {
        self.validate(path)?;
        if self.metadata.frame_id.as_deref() != Some(expected_frame) {
            return Err(ValidationError::new(
                path,
                format!("expected frame {expected_frame:?}"),
            ));
        }
        Ok(())
    }
}

pub trait FiniteValue {
    fn is_finite(&self) -> bool;
}

impl FiniteValue for f64 {
    fn is_finite(&self) -> bool {
        f64::is_finite(*self)
    }
}

impl FiniteValue for [f64; 3] {
    fn is_finite(&self) -> bool {
        self.iter().all(|value| value.is_finite())
    }
}

impl FiniteValue for [f64; 4] {
    fn is_finite(&self) -> bool {
        self.iter().all(|value| value.is_finite())
    }
}

impl FiniteValue for [f64; 9] {
    fn is_finite(&self) -> bool {
        self.iter().all(|value| value.is_finite())
    }
}

impl<T: FiniteValue> FieldValue<T> {
    pub fn validate_finite(&self, path: &str) -> Result<(), ValidationError> {
        self.validate(path)?;
        if self.value.as_ref().is_some_and(|value| !value.is_finite()) {
            return Err(ValidationError::new(path, "numeric value must be finite"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pose3 {
    /// Translation in metres in the parent/world frame.
    pub translation_m: FieldValue<[f64; 3]>,
    /// Unit quaternion in x, y, z, w order.
    pub rotation_xyzw: FieldValue<[f64; 4]>,
    /// Explicit child frame represented by this transform.
    pub child_frame_id: String,
    /// Contract v0.1 uses the world-from-entity transform direction.
    pub direction: TransformDirection,
}

impl Pose3 {
    fn validate(&self, path: &str, expected_frame: &str) -> Result<(), ValidationError> {
        if self.child_frame_id.trim().is_empty() || self.child_frame_id == expected_frame {
            return Err(ValidationError::new(
                path,
                "child frame must be non-empty and distinct from the parent/world frame",
            ));
        }
        self.translation_m
            .validate_expected_unit(&format!("{path}.translation"), "m")?;
        self.translation_m
            .validate_frame(&format!("{path}.translation"), expected_frame)?;
        self.translation_m
            .validate_finite(&format!("{path}.translation"))?;
        self.rotation_xyzw
            .validate_expected_unit(&format!("{path}.rotation"), "1")?;
        self.rotation_xyzw
            .validate_frame(&format!("{path}.rotation"), expected_frame)?;
        self.rotation_xyzw
            .validate_finite(&format!("{path}.rotation"))?;

        if self.translation_m.value.is_none() {
            return Err(ValidationError::new(
                path,
                "entity pose translation cannot be unavailable",
            ));
        }
        let quaternion = self.rotation_xyzw.value.as_ref().ok_or_else(|| {
            ValidationError::new(path, "entity pose rotation cannot be unavailable")
        })?;
        let norm = quaternion
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        if (norm - 1.0).abs() > QUATERNION_NORM_TOLERANCE {
            return Err(ValidationError::new(
                path,
                format!("quaternion norm must be 1 ± {QUATERNION_NORM_TOLERANCE}"),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContactStatus {
    NoContact,
    InContact,
    Unavailable { reason: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactObservation {
    pub status: ContactStatus,
    /// Provenance and uncertainty for the contact classification itself.
    pub metadata: FieldMetadata,
    pub partner_entity_ids: Vec<String>,
    pub impulse_ns: Option<FieldValue<[f64; 3]>>,
}

impl ContactObservation {
    fn validate(
        &self,
        path: &str,
        entity_id: &str,
        world_frame: &str,
    ) -> Result<(), ValidationError> {
        self.metadata.validate(&format!("{path}.metadata"))?;
        if self.metadata.unit != "1" {
            return Err(ValidationError::new(
                path,
                "contact classification must use unit '1'",
            ));
        }
        if self.metadata.frame_id.as_deref() != Some(world_frame) {
            return Err(ValidationError::new(
                path,
                "contact classification must explicitly identify the world frame",
            ));
        }
        let partners = self.partner_entity_ids.iter().collect::<BTreeSet<_>>();
        if partners.len() != self.partner_entity_ids.len()
            || self
                .partner_entity_ids
                .iter()
                .any(|value| value.as_str() == entity_id)
        {
            return Err(ValidationError::new(
                path,
                "contact partners must be unique and cannot include the entity itself",
            ));
        }
        match &self.status {
            ContactStatus::NoContact => {
                if self.metadata.status == FieldStatus::Unavailable {
                    return Err(ValidationError::new(
                        path,
                        "NoContact requires an available, provenance-backed classification",
                    ));
                }
                if !self.partner_entity_ids.is_empty() || self.impulse_ns.is_some() {
                    return Err(ValidationError::new(
                        path,
                        "NoContact cannot include partners or an impulse",
                    ));
                }
            }
            ContactStatus::InContact => {
                if self.metadata.status == FieldStatus::Unavailable {
                    return Err(ValidationError::new(
                        path,
                        "InContact requires an available, provenance-backed classification",
                    ));
                }
                if self.partner_entity_ids.is_empty() {
                    return Err(ValidationError::new(
                        path,
                        "InContact requires at least one partner entity",
                    ));
                }
                if let Some(impulse) = &self.impulse_ns {
                    impulse.validate_expected_unit(&format!("{path}.impulse"), "N*s")?;
                    impulse.validate_finite(&format!("{path}.impulse"))?;
                }
            }
            ContactStatus::Unavailable { reason } => {
                if self.metadata.status != FieldStatus::Unavailable
                    || self.metadata.unavailable_reason.as_deref() != Some(reason.as_str())
                    || reason.trim().is_empty()
                    || !self.partner_entity_ids.is_empty()
                    || self.impulse_ns.is_some()
                {
                    return Err(ValidationError::new(
                        path,
                        "Unavailable contact requires a reason and no fabricated partners/impulse",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityState {
    pub entity_id: String,
    pub generation: u64,
    pub parent_entity_id: Option<String>,
    pub pose: Pose3,
    pub linear_velocity_m_s: FieldValue<[f64; 3]>,
    pub angular_velocity_rad_s: FieldValue<[f64; 3]>,
    pub linear_acceleration_m_s2: FieldValue<[f64; 3]>,
    pub mass_kg: FieldValue<f64>,
    /// Row-major 3x3 inertia tensor in kg*m^2.
    pub inertia_kg_m2: FieldValue<[f64; 9]>,
    pub contact: ContactObservation,
}

impl EntityState {
    fn validate(&self, path: &str, world_frame: &str) -> Result<(), ValidationError> {
        if self.entity_id.trim().is_empty() {
            return Err(ValidationError::new(path, "entity ID must be non-empty"));
        }
        if self
            .parent_entity_id
            .as_deref()
            .is_some_and(|parent| parent.trim().is_empty() || parent == self.entity_id)
        {
            return Err(ValidationError::new(
                path,
                "parent ID must be non-empty and cannot be self",
            ));
        }
        self.pose.validate(&format!("{path}.pose"), world_frame)?;
        for (name, field, unit) in [
            ("linear_velocity", &self.linear_velocity_m_s, "m/s"),
            ("angular_velocity", &self.angular_velocity_rad_s, "rad/s"),
            (
                "linear_acceleration",
                &self.linear_acceleration_m_s2,
                "m/s^2",
            ),
        ] {
            field.validate_expected_unit(&format!("{path}.{name}"), unit)?;
            field.validate_frame(&format!("{path}.{name}"), world_frame)?;
            field.validate_finite(&format!("{path}.{name}"))?;
        }
        self.mass_kg
            .validate_expected_unit(&format!("{path}.mass"), "kg")?;
        self.mass_kg.validate_finite(&format!("{path}.mass"))?;
        if self.mass_kg.value.is_some_and(|mass| mass <= 0.0) {
            return Err(ValidationError::new(
                format!("{path}.mass"),
                "available rigid-body mass must be strictly positive",
            ));
        }
        self.inertia_kg_m2
            .validate_expected_unit(&format!("{path}.inertia"), "kg*m^2")?;
        self.inertia_kg_m2
            .validate_frame(&format!("{path}.inertia"), &self.pose.child_frame_id)?;
        self.inertia_kg_m2
            .validate_finite(&format!("{path}.inertia"))?;
        if let Some(values) = &self.inertia_kg_m2.value {
            let matrix = [
                [values[0], values[1], values[2]],
                [values[3], values[4], values[5]],
                [values[6], values[7], values[8]],
            ];
            validate_physical_inertia3(&matrix, &format!("{path}.inertia"))?;
        }
        self.contact
            .validate(&format!("{path}.contact"), &self.entity_id, world_frame)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformConvention {
    RightHandedXyzw,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformDirection {
    WorldFromEntity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotIdentity {
    pub snapshot_sha256: String,
    pub model_build_sha256: String,
    pub configuration_sha256: String,
    pub world_id: String,
    pub episode_id: String,
    pub trajectory_id: String,
    pub step_index: u64,
    pub simulation_time_ns: u64,
    pub clock_source: String,
    pub world_frame: String,
    pub transform_convention: TransformConvention,
    pub unit_system: String,
}

impl SnapshotIdentity {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_sha256(&self.snapshot_sha256, "identity.snapshot_sha256")?;
        validate_sha256(&self.model_build_sha256, "identity.model_build_sha256")?;
        validate_sha256(&self.configuration_sha256, "identity.configuration_sha256")?;
        for (name, value) in [
            ("world_id", self.world_id.as_str()),
            ("episode_id", self.episode_id.as_str()),
            ("trajectory_id", self.trajectory_id.as_str()),
            ("clock_source", self.clock_source.as_str()),
            ("world_frame", self.world_frame.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ValidationError::new(
                    format!("identity.{name}"),
                    "must be non-empty",
                ));
            }
        }
        if self.unit_system != "SI" {
            return Err(ValidationError::new(
                "identity.unit_system",
                "v0.1 requires canonical SI units",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateSnapshot {
    pub schema_version: String,
    pub identity: SnapshotIdentity,
    pub entities: Vec<EntityState>,
}

impl StateSnapshot {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(ValidationError::new(
                "schema_version",
                format!("expected {SCHEMA_VERSION:?}"),
            ));
        }
        self.identity.validate()?;
        if self.entities.is_empty() {
            return Err(ValidationError::new(
                "entities",
                "snapshot must contain at least one entity",
            ));
        }
        let mut entity_ids = BTreeSet::new();
        let mut frame_ids = BTreeSet::new();
        for (index, entity) in self.entities.iter().enumerate() {
            entity.validate(&format!("entities[{index}]"), &self.identity.world_frame)?;
            if !entity_ids.insert(entity.entity_id.as_str()) {
                return Err(ValidationError::new(
                    format!("entities[{index}].entity_id"),
                    "duplicate entity ID in snapshot",
                ));
            }
            if !frame_ids.insert(entity.pose.child_frame_id.as_str()) {
                return Err(ValidationError::new(
                    format!("entities[{index}].pose.child_frame_id"),
                    "duplicate child frame ID in snapshot",
                ));
            }
        }
        for (index, entity) in self.entities.iter().enumerate() {
            if let Some(parent) = &entity.parent_entity_id
                && !entity_ids.contains(parent.as_str())
            {
                return Err(ValidationError::new(
                    format!("entities[{index}].parent_entity_id"),
                    "parent entity is not present in this snapshot",
                ));
            }
            for partner in &entity.contact.partner_entity_ids {
                if !entity_ids.contains(partner.as_str()) {
                    return Err(ValidationError::new(
                        format!("entities[{index}].contact"),
                        format!("unknown contact partner entity {partner:?}"),
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn validate_snapshot_sequence(snapshots: &[StateSnapshot]) -> Result<(), ValidationError> {
    if snapshots.is_empty() {
        return Err(ValidationError::new(
            "snapshots",
            "sequence must contain at least one snapshot",
        ));
    }
    let mut snapshot_digests = BTreeSet::new();
    let mut last_generation_by_entity = BTreeMap::<String, u64>::new();
    let mut previously_present = BTreeSet::<String>::new();

    for (index, snapshot) in snapshots.iter().enumerate() {
        snapshot.validate()?;
        let current = &snapshot.identity;
        if !snapshot_digests.insert(current.snapshot_sha256.as_str()) {
            return Err(ValidationError::new(
                format!("snapshots[{index}].identity.snapshot_sha256"),
                "snapshot digest must be unique within a trajectory",
            ));
        }

        let mut currently_present = BTreeSet::new();
        for entity in &snapshot.entities {
            if let Some(previous_generation) = last_generation_by_entity.get(&entity.entity_id) {
                if previously_present.contains(&entity.entity_id) {
                    if entity.generation != *previous_generation {
                        return Err(ValidationError::new(
                            format!("snapshots[{index}].entities"),
                            "an entity generation cannot change while the entity remains present",
                        ));
                    }
                } else if entity.generation <= *previous_generation {
                    return Err(ValidationError::new(
                        format!("snapshots[{index}].entities"),
                        "a reappearing entity ID must increment its generation",
                    ));
                }
            }
            last_generation_by_entity.insert(entity.entity_id.clone(), entity.generation);
            currently_present.insert(entity.entity_id.clone());
        }
        previously_present = currently_present;

        if index == 0 {
            continue;
        }
        let previous = &snapshots[index - 1].identity;
        if current.trajectory_id != previous.trajectory_id
            || current.episode_id != previous.episode_id
            || current.world_id != previous.world_id
            || current.model_build_sha256 != previous.model_build_sha256
            || current.configuration_sha256 != previous.configuration_sha256
            || current.clock_source != previous.clock_source
            || current.world_frame != previous.world_frame
            || current.transform_convention != previous.transform_convention
            || current.unit_system != previous.unit_system
        {
            return Err(ValidationError::new(
                format!("snapshots[{index}].identity"),
                "sequence crossed world, episode, trajectory, model or configuration identity",
            ));
        }
        if current.step_index <= previous.step_index
            || current.simulation_time_ns <= previous.simulation_time_ns
        {
            return Err(ValidationError::new(
                format!("snapshots[{index}].identity"),
                "step index and simulation time must be strictly increasing",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StateField {
    Pose,
    LinearVelocity,
    AngularVelocity,
    LinearAcceleration,
    Mass,
    Inertia,
    Contact,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateQuery {
    pub snapshot_sha256: String,
    pub entity_ids: Vec<String>,
    /// Required evidence focus. v0.1 returns complete stable EntityState
    /// records (including explicitly unavailable fields), not a sparse schema.
    pub fields: Vec<StateField>,
}

impl StateQuery {
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_sha256(&self.snapshot_sha256, "query.snapshot_sha256")?;
        if self.entity_ids.is_empty() || self.fields.is_empty() {
            return Err(ValidationError::new(
                "query",
                "at least one entity ID and field are required",
            ));
        }
        let ids = self.entity_ids.iter().collect::<BTreeSet<_>>();
        let fields = self.fields.iter().collect::<BTreeSet<_>>();
        if ids.len() != self.entity_ids.len() || fields.len() != self.fields.len() {
            return Err(ValidationError::new(
                "query",
                "entity IDs and requested fields must be unique",
            ));
        }
        if self.entity_ids.iter().any(|id| id.trim().is_empty()) {
            return Err(ValidationError::new(
                "query.entity_ids",
                "entity IDs must be non-empty",
            ));
        }
        Ok(())
    }

    pub fn validate_result(&self, result: &StateSnapshot) -> Result<(), ValidationError> {
        self.validate()?;
        result.validate()?;
        if result.identity.snapshot_sha256 != self.snapshot_sha256 {
            return Err(ValidationError::new(
                "query.result.snapshot_sha256",
                "returned snapshot does not match the requested immutable snapshot",
            ));
        }
        let entity_by_id = result
            .entities
            .iter()
            .map(|entity| (entity.entity_id.as_str(), entity))
            .collect::<BTreeMap<_, _>>();
        let mut allowed = BTreeSet::<&str>::new();
        let mut pending = self
            .entity_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        while let Some(entity_id) = pending.pop() {
            if !allowed.insert(entity_id) {
                continue;
            }
            let entity = entity_by_id.get(entity_id).ok_or_else(|| {
                ValidationError::new(
                    "query.result.entities",
                    format!("requested or required context entity {entity_id:?} is missing"),
                )
            })?;
            if let Some(parent) = entity.parent_entity_id.as_deref() {
                pending.push(parent);
            }
            pending.extend(entity.contact.partner_entity_ids.iter().map(String::as_str));
        }
        let returned = entity_by_id.keys().copied().collect::<BTreeSet<_>>();
        if returned != allowed {
            return Err(ValidationError::new(
                "query.result.entities",
                "result must contain requested entities and their transitive parent/contact context only",
            ));
        }
        Ok(())
    }
}

/// Implementations query the immutable snapshot named in the request.
/// The shared reference expresses the read-only boundary at the Rust API level.
pub trait ReadOnlyStateQuery {
    type Error: Display;

    fn query(&self, request: &StateQuery) -> Result<StateSnapshot, Self::Error>;
}

/// An action-bound request for exactly one externally visible state transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepRequest {
    pub pre_snapshot_sha256: String,
    pub model_build_sha256: String,
    pub configuration_sha256: String,
    pub world_id: String,
    pub episode_id: String,
    pub trajectory_id: String,
    pub clock_source: String,
    pub world_frame: String,
    pub transform_convention: TransformConvention,
    pub unit_system: String,
    pub pre_step_index: u64,
    pub pre_simulation_time_ns: u64,
    pub action_stream_sha256: String,
    pub duration_ns: u64,
}

impl StepRequest {
    pub fn from_snapshot(
        pre_state: &StateSnapshot,
        action_stream_sha256: impl Into<String>,
        duration_ns: u64,
    ) -> Result<Self, ValidationError> {
        pre_state.validate()?;
        let request = Self {
            pre_snapshot_sha256: pre_state.identity.snapshot_sha256.clone(),
            model_build_sha256: pre_state.identity.model_build_sha256.clone(),
            configuration_sha256: pre_state.identity.configuration_sha256.clone(),
            world_id: pre_state.identity.world_id.clone(),
            episode_id: pre_state.identity.episode_id.clone(),
            trajectory_id: pre_state.identity.trajectory_id.clone(),
            clock_source: pre_state.identity.clock_source.clone(),
            world_frame: pre_state.identity.world_frame.clone(),
            transform_convention: pre_state.identity.transform_convention,
            unit_system: pre_state.identity.unit_system.clone(),
            pre_step_index: pre_state.identity.step_index,
            pre_simulation_time_ns: pre_state.identity.simulation_time_ns,
            action_stream_sha256: action_stream_sha256.into(),
            duration_ns,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_sha256(&self.pre_snapshot_sha256, "step.pre_snapshot_sha256")?;
        validate_sha256(&self.model_build_sha256, "step.model_build_sha256")?;
        validate_sha256(&self.configuration_sha256, "step.configuration_sha256")?;
        validate_sha256(&self.action_stream_sha256, "step.action_stream_sha256")?;
        for (name, value) in [
            ("world_id", self.world_id.as_str()),
            ("episode_id", self.episode_id.as_str()),
            ("trajectory_id", self.trajectory_id.as_str()),
            ("clock_source", self.clock_source.as_str()),
            ("world_frame", self.world_frame.as_str()),
            ("unit_system", self.unit_system.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ValidationError::new(
                    format!("step.{name}"),
                    "must be non-empty",
                ));
            }
        }
        if self.unit_system != "SI" {
            return Err(ValidationError::new(
                "step.unit_system",
                "v0.1 requires canonical SI units",
            ));
        }
        if self.duration_ns == 0 {
            return Err(ValidationError::new(
                "step.duration_ns",
                "step duration must be positive",
            ));
        }
        self.pre_step_index
            .checked_add(1)
            .ok_or_else(|| ValidationError::new("step.pre_step_index", "step index overflow"))?;
        self.pre_simulation_time_ns
            .checked_add(self.duration_ns)
            .ok_or_else(|| ValidationError::new("step.duration_ns", "simulation time overflow"))?;
        Ok(())
    }

    pub fn validate_result(&self, result: &StateSnapshot) -> Result<(), ValidationError> {
        self.validate()?;
        result.validate()?;
        let expected_step = self.pre_step_index + 1;
        let expected_time = self.pre_simulation_time_ns + self.duration_ns;
        let identity = &result.identity;
        if identity.snapshot_sha256 == self.pre_snapshot_sha256 {
            return Err(ValidationError::new(
                "step.result.snapshot_sha256",
                "a state transition must produce a new snapshot identity",
            ));
        }
        if identity.model_build_sha256 != self.model_build_sha256
            || identity.configuration_sha256 != self.configuration_sha256
            || identity.world_id != self.world_id
            || identity.episode_id != self.episode_id
            || identity.trajectory_id != self.trajectory_id
            || identity.clock_source != self.clock_source
            || identity.world_frame != self.world_frame
            || identity.transform_convention != self.transform_convention
            || identity.unit_system != self.unit_system
        {
            return Err(ValidationError::new(
                "step.result.identity",
                "result must preserve model, configuration, world, episode and trajectory identity",
            ));
        }
        if identity.step_index != expected_step || identity.simulation_time_ns != expected_time {
            return Err(ValidationError::new(
                "step.result.identity",
                "result step and simulation time must equal the pre-state plus one step and the requested duration",
            ));
        }
        Ok(())
    }
}

/// Implementations mutate the simulation only through an explicit action-bound step.
/// Observation/query uses ReadOnlyStateQuery and is intentionally a separate trait.
pub trait ActionConditionedStepper {
    type Error: Display;

    fn step(&mut self, request: &StepRequest) -> Result<StateSnapshot, Self::Error>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReplayClass {
    Deterministic,
    Stochastic {
        maximum_state_divergence: f64,
        divergence_metric: String,
    },
    NonReplayable {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplayReceipt {
    pub initial_snapshot_sha256: String,
    pub action_stream_sha256: String,
    pub model_build_sha256: String,
    pub configuration_sha256: String,
    pub solver_sha256: String,
    pub output_snapshot_sha256: String,
    pub random_seed_sha256: Option<String>,
    pub replay_class: ReplayClass,
}

impl ReplayReceipt {
    /// Verify that this replay receipt describes the exact action-bound transition.
    pub fn validate_for_step(
        &self,
        request: &StepRequest,
        output: &StateSnapshot,
    ) -> Result<(), ValidationError> {
        self.validate()?;
        request.validate_result(output)?;
        if self.initial_snapshot_sha256 != request.pre_snapshot_sha256
            || self.action_stream_sha256 != request.action_stream_sha256
            || self.model_build_sha256 != request.model_build_sha256
            || self.configuration_sha256 != request.configuration_sha256
            || self.output_snapshot_sha256 != output.identity.snapshot_sha256
        {
            return Err(ValidationError::new(
                "replay",
                "replay receipt digests do not match the action-bound state transition",
            ));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        for (name, digest) in [
            ("initial_snapshot_sha256", &self.initial_snapshot_sha256),
            ("action_stream_sha256", &self.action_stream_sha256),
            ("model_build_sha256", &self.model_build_sha256),
            ("configuration_sha256", &self.configuration_sha256),
            ("solver_sha256", &self.solver_sha256),
            ("output_snapshot_sha256", &self.output_snapshot_sha256),
        ] {
            validate_sha256(digest, &format!("replay.{name}"))?;
        }
        if let Some(seed) = &self.random_seed_sha256 {
            validate_sha256(seed, "replay.random_seed_sha256")?;
        }
        match &self.replay_class {
            ReplayClass::Deterministic => {}
            ReplayClass::Stochastic {
                maximum_state_divergence,
                divergence_metric,
            } => {
                if !maximum_state_divergence.is_finite()
                    || *maximum_state_divergence < 0.0
                    || divergence_metric.trim().is_empty()
                {
                    return Err(ValidationError::new(
                        "replay.replay_class",
                        "stochastic replay requires a finite non-negative divergence bound and metric",
                    ));
                }
            }
            ReplayClass::NonReplayable { reason } => {
                if reason.trim().is_empty() {
                    return Err(ValidationError::new(
                        "replay.replay_class",
                        "NonReplayable requires an explicit reason",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn validate_sha256(value: &str, path: &str) -> Result<(), ValidationError> {
    if value.len() != SHA256_HEX_LEN || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ValidationError::new(
            path,
            "expected exactly 64 hexadecimal SHA-256 characters",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(
        status: FieldStatus,
        unit: &str,
        frame_id: Option<&str>,
        uncertainty: Option<Uncertainty>,
    ) -> FieldMetadata {
        FieldMetadata {
            status,
            unit: unit.to_owned(),
            frame_id: frame_id.map(str::to_owned),
            uncertainty,
            provenance: Provenance {
                source_sha256: (status == FieldStatus::Measured).then(|| "a".repeat(64)),
                model_build_sha256: (status == FieldStatus::Predicted).then(|| "b".repeat(64)),
                synthetic_fixture_id: None,
                derivation: (status == FieldStatus::Estimated)
                    .then(|| "central difference".to_owned()),
            },
            unavailable_reason: (status == FieldStatus::Unavailable)
                .then(|| "not exposed by this model".to_owned()),
        }
    }

    fn available<T>(
        value: T,
        unit: &str,
        frame: Option<&str>,
        uncertainty: Uncertainty,
    ) -> FieldValue<T> {
        FieldValue {
            value: Some(value),
            metadata: metadata(FieldStatus::Measured, unit, frame, Some(uncertainty)),
        }
    }

    fn unavailable<T>(unit: &str, frame: Option<&str>) -> FieldValue<T> {
        FieldValue {
            value: None,
            metadata: metadata(FieldStatus::Unavailable, unit, frame, None),
        }
    }

    fn zero_covariance(unit: &str) -> Uncertainty {
        Uncertainty::Covariance3 {
            matrix: [[0.0; 3]; 3],
            unit: unit.to_owned(),
            frame_id: "world".to_owned(),
        }
    }

    fn valid_entity(entity_id: &str) -> EntityState {
        EntityState {
            entity_id: entity_id.to_owned(),
            generation: 0,
            parent_entity_id: None,
            pose: Pose3 {
                translation_m: available(
                    [0.0, 0.0, 0.0],
                    "m",
                    Some("world"),
                    zero_covariance("m^2"),
                ),
                rotation_xyzw: available(
                    [0.0, 0.0, 0.0, 1.0],
                    "1",
                    Some("world"),
                    Uncertainty::NotQuantified {
                        reason: "orientation covariance not supplied by fixture".to_owned(),
                    },
                ),
                child_frame_id: format!("{entity_id}/frame"),
                direction: TransformDirection::WorldFromEntity,
            },
            linear_velocity_m_s: available(
                [0.0, 0.0, 0.0],
                "m/s",
                Some("world"),
                zero_covariance("m^2/s^2"),
            ),
            angular_velocity_rad_s: unavailable("rad/s", Some("world")),
            linear_acceleration_m_s2: unavailable("m/s^2", Some("world")),
            mass_kg: available(
                1.0,
                "kg",
                None,
                Uncertainty::ScalarStdDev {
                    standard_deviation: 0.01,
                },
            ),
            inertia_kg_m2: unavailable("kg*m^2", Some(&format!("{entity_id}/frame"))),
            contact: ContactObservation {
                status: ContactStatus::NoContact,
                metadata: metadata(
                    FieldStatus::Measured,
                    "1",
                    Some("world"),
                    Some(Uncertainty::NotQuantified {
                        reason: "contact classifier uncertainty not supplied".to_owned(),
                    }),
                ),
                partner_entity_ids: Vec::new(),
                impulse_ns: None,
            },
        }
    }

    fn valid_snapshot(step_index: u64, simulation_time_ns: u64) -> StateSnapshot {
        StateSnapshot {
            schema_version: SCHEMA_VERSION.to_owned(),
            identity: SnapshotIdentity {
                snapshot_sha256: format!("{:064x}", step_index + 1),
                model_build_sha256: "b".repeat(64),
                configuration_sha256: "c".repeat(64),
                world_id: "world-1".to_owned(),
                episode_id: "episode-1".to_owned(),
                trajectory_id: "trajectory-1".to_owned(),
                step_index,
                simulation_time_ns,
                clock_source: "solver_monotonic_clock".to_owned(),
                world_frame: "world".to_owned(),
                transform_convention: TransformConvention::RightHandedXyzw,
                unit_system: "SI".to_owned(),
            },
            entities: vec![valid_entity("body-1")],
        }
    }

    #[test]
    fn valid_snapshot_accepts_measured_and_explicitly_unavailable_fields() {
        assert!(valid_snapshot(0, 0).validate().is_ok());
    }

    #[test]
    fn unavailable_field_cannot_carry_a_silent_zero() {
        let field = unavailable::<f64>("kg", None);
        assert!(field.validate("mass").is_ok());
        let fabricated = FieldValue {
            value: Some(0.0),
            metadata: metadata(FieldStatus::Unavailable, "kg", None, None),
        };
        assert!(fabricated.validate("mass").is_err());
    }

    #[test]
    fn measured_values_require_source_or_synthetic_provenance() {
        let field = FieldValue {
            value: Some(1.0),
            metadata: FieldMetadata {
                status: FieldStatus::Measured,
                unit: "kg".to_owned(),
                frame_id: None,
                uncertainty: Some(Uncertainty::NotQuantified {
                    reason: "scale uncertainty not supplied".to_owned(),
                }),
                provenance: Provenance::default(),
                unavailable_reason: None,
            },
        };
        assert!(field.validate("mass").is_err());
    }

    #[test]
    fn malformed_uncertainty_is_rejected() {
        assert!(
            Uncertainty::Interval {
                lower: 2.0,
                upper: 1.0
            }
            .validate("interval")
            .is_err()
        );
        assert!(
            Uncertainty::Covariance3 {
                matrix: [[1.0, 2.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                unit: "m^2".to_owned(),
                frame_id: "world".to_owned(),
            }
            .validate("covariance")
            .is_err()
        );
    }

    #[test]
    fn covariance_requires_matching_squared_unit_and_frame() {
        let valid = Uncertainty::Covariance3 {
            matrix: [[0.0; 3]; 3],
            unit: "m^2".to_owned(),
            frame_id: "world".to_owned(),
        };
        let field = available([0.0, 0.0, 0.0], "m", Some("world"), valid);
        assert!(field.validate("position").is_ok());

        let wrong_unit = available(
            [0.0, 0.0, 0.0],
            "m",
            Some("world"),
            Uncertainty::Covariance3 {
                matrix: [[0.0; 3]; 3],
                unit: "m".to_owned(),
                frame_id: "world".to_owned(),
            },
        );
        assert!(wrong_unit.validate("position").is_err());

        let wrong_frame = available(
            [0.0, 0.0, 0.0],
            "m",
            Some("world"),
            Uncertainty::Covariance3 {
                matrix: [[0.0; 3]; 3],
                unit: "m^2".to_owned(),
                frame_id: "body".to_owned(),
            },
        );
        assert!(wrong_frame.validate("position").is_err());
    }

    #[test]
    fn non_unit_quaternion_is_rejected() {
        let mut snapshot = valid_snapshot(0, 0);
        snapshot.entities[0].pose.rotation_xyzw.value = Some([0.0, 0.0, 0.0, 2.0]);
        assert!(snapshot.validate().is_err());
    }

    #[test]
    fn contact_classification_must_have_its_own_provenance() {
        let mut snapshot = valid_snapshot(0, 0);
        let reason = "contact channel not implemented by this engine".to_owned();
        let mut unavailable_metadata = metadata(FieldStatus::Unavailable, "1", Some("world"), None);
        unavailable_metadata.unavailable_reason = Some(reason.clone());
        snapshot.entities[0].contact = ContactObservation {
            status: ContactStatus::Unavailable {
                reason: reason.clone(),
            },
            metadata: unavailable_metadata,
            partner_entity_ids: Vec::new(),
            impulse_ns: None,
        };
        assert!(snapshot.validate().is_ok());

        snapshot.entities[0].contact = ContactObservation {
            status: ContactStatus::NoContact,
            metadata: metadata(FieldStatus::Unavailable, "1", Some("world"), None),
            partner_entity_ids: Vec::new(),
            impulse_ns: None,
        };
        assert!(snapshot.validate().is_err());
    }

    #[test]
    fn entity_inertia_must_be_symmetric_positive_semidefinite() {
        let mut snapshot = valid_snapshot(0, 0);
        snapshot.entities[0].inertia_kg_m2 = available(
            [2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0],
            "kg*m^2",
            Some("body-1/frame"),
            Uncertainty::NotQuantified {
                reason: "synthetic fixture does not quantify inertia uncertainty".to_owned(),
            },
        );
        assert!(snapshot.validate().is_ok());

        snapshot.entities[0].inertia_kg_m2.value =
            Some([2.0, 0.5, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0]);
        assert!(snapshot.validate().is_err());
    }

    #[test]
    fn snapshot_sequence_rejects_world_frame_or_clock_changes() {
        let first = valid_snapshot(0, 0);
        let mut second = valid_snapshot(1, 1_000_000);
        second.identity.clock_source = "other_clock".to_owned();
        assert!(validate_snapshot_sequence(&[first.clone(), second]).is_err());

        let mut second = valid_snapshot(1, 1_000_000);
        second.identity.world_frame = "other_world_frame".to_owned();
        assert!(validate_snapshot_sequence(&[first, second]).is_err());
    }

    #[test]
    fn duplicate_entity_ids_are_rejected() {
        let mut snapshot = valid_snapshot(0, 0);
        snapshot.entities.push(valid_entity("body-1"));
        assert!(snapshot.validate().is_err());
    }

    #[test]
    fn unknown_contact_partner_is_rejected() {
        let mut snapshot = valid_snapshot(0, 0);
        snapshot.entities[0].contact = ContactObservation {
            status: ContactStatus::InContact,
            metadata: metadata(
                FieldStatus::Measured,
                "1",
                Some("world"),
                Some(Uncertainty::NotQuantified {
                    reason: "synthetic contact fixture".to_owned(),
                }),
            ),
            partner_entity_ids: vec!["missing-body".to_owned()],
            impulse_ns: None,
        };
        assert!(snapshot.validate().is_err());
    }

    #[test]
    fn snapshot_sequences_require_monotonic_identity_and_time() {
        let first = valid_snapshot(0, 0);
        let second = valid_snapshot(1, 1_000_000);
        assert!(validate_snapshot_sequence(&[first.clone(), second]).is_ok());
        let out_of_order = valid_snapshot(1, 0);
        assert!(validate_snapshot_sequence(&[first, out_of_order]).is_err());
    }

    #[test]
    fn reappearing_entity_id_requires_a_new_generation() {
        let first = valid_snapshot(0, 0);
        let mut middle = valid_snapshot(1, 1_000_000);
        middle.entities[0] = valid_entity("other-body");

        let mut reappeared_old_generation = valid_snapshot(2, 2_000_000);
        assert!(
            validate_snapshot_sequence(&[
                first.clone(),
                middle.clone(),
                reappeared_old_generation.clone()
            ])
            .is_err()
        );

        reappeared_old_generation.entities[0].generation = 1;
        assert!(validate_snapshot_sequence(&[first, middle, reappeared_old_generation]).is_ok());
    }

    #[test]
    fn snapshot_sequences_reject_duplicate_snapshot_digests() {
        let first = valid_snapshot(0, 0);
        let mut second = valid_snapshot(1, 1_000_000);
        second.identity.snapshot_sha256 = first.identity.snapshot_sha256.clone();
        assert!(validate_snapshot_sequence(&[first, second]).is_err());
    }

    #[test]
    fn query_rejects_a_stale_or_wrong_snapshot_result() {
        let query = StateQuery {
            snapshot_sha256: "a".repeat(64),
            entity_ids: vec!["body-1".to_owned()],
            fields: vec![StateField::Pose, StateField::LinearVelocity],
        };
        assert!(query.validate_result(&valid_snapshot(0, 0)).is_err());
    }

    #[test]
    fn query_requires_exact_entity_set() {
        let mut query = StateQuery {
            snapshot_sha256: valid_snapshot(0, 0).identity.snapshot_sha256,
            entity_ids: vec!["body-2".to_owned()],
            fields: vec![StateField::Pose],
        };
        let result = valid_snapshot(0, 0);
        assert!(query.validate_result(&result).is_err());
        query.entity_ids = vec!["body-1".to_owned()];
        assert!(query.validate_result(&result).is_ok());

        let mut context_result = valid_snapshot(0, 0);
        let partner = valid_entity("partner-1");
        context_result.entities.push(partner);
        context_result.entities[0].contact = ContactObservation {
            status: ContactStatus::InContact,
            metadata: metadata(
                FieldStatus::Measured,
                "1",
                Some("world"),
                Some(Uncertainty::NotQuantified {
                    reason: "synthetic contact fixture has no calibrated classifier error"
                        .to_owned(),
                }),
            ),
            partner_entity_ids: vec!["partner-1".to_owned()],
            impulse_ns: None,
        };
        assert!(query.validate_result(&context_result).is_ok());

        context_result.entities.push(valid_entity("unrelated-body"));
        assert!(query.validate_result(&context_result).is_err());
    }

    #[test]
    fn step_request_binds_the_result_to_pre_state_action_duration_and_identity() {
        let pre_state = valid_snapshot(0, 0);
        let request = StepRequest::from_snapshot(&pre_state, "d".repeat(64), 1_000_000).unwrap();
        let result = valid_snapshot(1, 1_000_000);
        assert!(request.validate_result(&result).is_ok());

        let wrong_time = valid_snapshot(1, 2_000_000);
        assert!(request.validate_result(&wrong_time).is_err());

        let mut wrong_model = valid_snapshot(1, 1_000_000);
        wrong_model.identity.model_build_sha256 = "e".repeat(64);
        assert!(request.validate_result(&wrong_model).is_err());

        let same_snapshot = pre_state.clone();
        assert!(request.validate_result(&same_snapshot).is_err());
    }

    #[test]
    fn step_request_rejects_zero_duration_and_invalid_action_digest() {
        let pre_state = valid_snapshot(0, 0);
        assert!(StepRequest::from_snapshot(&pre_state, "invalid", 1).is_err());
        assert!(StepRequest::from_snapshot(&pre_state, "d".repeat(64), 0).is_err());
    }

    #[test]
    fn replay_receipt_must_match_the_exact_step_request_and_output() {
        let pre_state = valid_snapshot(0, 0);
        let output = valid_snapshot(1, 1_000_000);
        let action_digest = "d".repeat(64);
        let request =
            StepRequest::from_snapshot(&pre_state, action_digest.clone(), 1_000_000).unwrap();
        let receipt = ReplayReceipt {
            initial_snapshot_sha256: pre_state.identity.snapshot_sha256.clone(),
            action_stream_sha256: action_digest,
            model_build_sha256: pre_state.identity.model_build_sha256.clone(),
            configuration_sha256: pre_state.identity.configuration_sha256.clone(),
            solver_sha256: "e".repeat(64),
            output_snapshot_sha256: output.identity.snapshot_sha256.clone(),
            random_seed_sha256: None,
            replay_class: ReplayClass::Deterministic,
        };
        assert!(receipt.validate_for_step(&request, &output).is_ok());

        let mut wrong_receipt = receipt.clone();
        wrong_receipt.action_stream_sha256 = "f".repeat(64);
        assert!(wrong_receipt.validate_for_step(&request, &output).is_err());
    }

    #[test]
    fn predicted_and_estimated_fields_require_status_specific_provenance() {
        let predicted = FieldValue {
            value: Some(0.5),
            metadata: FieldMetadata {
                status: FieldStatus::Predicted,
                unit: "m".to_owned(),
                frame_id: Some("world".to_owned()),
                uncertainty: Some(Uncertainty::NotQuantified {
                    reason: "model emits no variance".to_owned(),
                }),
                provenance: Provenance {
                    model_build_sha256: Some("b".repeat(64)),
                    ..Provenance::default()
                },
                unavailable_reason: None,
            },
        };
        assert!(predicted.validate("predicted").is_ok());

        let missing_model = FieldValue {
            metadata: FieldMetadata {
                provenance: Provenance {
                    derivation: Some("model prediction".to_owned()),
                    ..Provenance::default()
                },
                ..predicted.metadata.clone()
            },
            ..predicted
        };
        assert!(missing_model.validate("predicted").is_err());
    }

    #[test]
    fn inertia_tensor_must_be_symmetric_positive_semidefinite() {
        let valid = [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]];
        assert!(validate_physical_inertia3(&valid, "inertia").is_ok());

        let asymmetric = [[2.0, 0.5, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]];
        assert!(validate_physical_inertia3(&asymmetric, "inertia").is_err());

        let negative_moment = [[-1.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]];
        assert!(validate_physical_inertia3(&negative_moment, "inertia").is_err());

        let triangle_violation = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 3.0]];
        assert!(validate_physical_inertia3(&triangle_violation, "inertia").is_err());
    }

    #[test]
    fn covariance_psd_validation_is_invariant_under_positive_rescaling() {
        let positive_semidefinite = [[2.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 1.0]];
        let indefinite = [[1.0, 2.0, 0.0], [2.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

        for scale in [1.0e-150, 1.0, 1.0e150] {
            let scaled_psd = std::array::from_fn(|row| {
                std::array::from_fn(|column| positive_semidefinite[row][column] * scale)
            });
            let scaled_indefinite = std::array::from_fn(|row| {
                std::array::from_fn(|column| indefinite[row][column] * scale)
            });

            assert!(
                validate_symmetric_psd3(&scaled_psd, "covariance").is_ok(),
                "PSD matrix rejected at positive scale {scale:e}"
            );
            assert!(
                validate_symmetric_psd3(&scaled_indefinite, "covariance").is_err(),
                "indefinite matrix accepted at positive scale {scale:e}"
            );
        }
    }

    #[test]
    fn inertia_triangle_validation_is_invariant_under_positive_rescaling() {
        let valid = [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
        let invalid_triangle = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 3.0]];

        for scale in [1.0e-150, 1.0, 1.0e150] {
            let scaled_valid =
                std::array::from_fn(|row| std::array::from_fn(|column| valid[row][column] * scale));
            let scaled_invalid = std::array::from_fn(|row| {
                std::array::from_fn(|column| invalid_triangle[row][column] * scale)
            });

            assert!(
                validate_physical_inertia3(&scaled_valid, "inertia").is_ok(),
                "physically realizable inertia rejected at positive scale {scale:e}"
            );
            assert!(
                validate_physical_inertia3(&scaled_invalid, "inertia").is_err(),
                "triangle-violating inertia accepted at positive scale {scale:e}"
            );
        }
    }

    #[test]
    fn inertia_triangle_validation_is_rotation_covariant_for_anisotropic_tensors() {
        // A non-axis-aligned orthogonal frame change creates off-diagonal
        // entries while preserving the tensor's principal moments.
        let rotation = [[0.6, -0.8, 0.0], [0.8, 0.6, 0.0], [0.0, 0.0, 1.0]];
        let rotate_tensor = |tensor: [[f64; 3]; 3]| {
            std::array::from_fn(|row| {
                std::array::from_fn(|column| {
                    (0..3)
                        .map(|left| {
                            (0..3)
                                .map(|right| {
                                    rotation[row][left]
                                        * tensor[left][right]
                                        * rotation[column][right]
                                })
                                .sum::<f64>()
                        })
                        .sum::<f64>()
                })
            })
        };

        let valid_principal = [[1.5, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.5]];
        let invalid_principal = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 4.0]];
        let valid_rotated = rotate_tensor(valid_principal);
        let invalid_rotated = rotate_tensor(invalid_principal);

        assert!((valid_rotated[0][1]).abs() > 0.1);
        assert!((invalid_rotated[0][1]).abs() > 0.1);
        assert!(validate_physical_inertia3(&valid_rotated, "inertia").is_ok());
        assert!(validate_physical_inertia3(&invalid_rotated, "inertia").is_err());
    }

    #[test]
    fn replay_receipt_distinguishes_deterministic_and_stochastic_modes() {
        let digest = "a".repeat(64);
        let receipt = ReplayReceipt {
            initial_snapshot_sha256: digest.clone(),
            action_stream_sha256: digest.clone(),
            model_build_sha256: digest.clone(),
            configuration_sha256: digest.clone(),
            solver_sha256: digest.clone(),
            output_snapshot_sha256: digest,
            random_seed_sha256: None,
            replay_class: ReplayClass::Stochastic {
                maximum_state_divergence: 0.001,
                divergence_metric: "se3_state_l2".to_owned(),
            },
        };
        assert!(receipt.validate().is_ok());

        let invalid = ReplayReceipt {
            replay_class: ReplayClass::Stochastic {
                maximum_state_divergence: f64::NAN,
                divergence_metric: String::new(),
            },
            ..receipt
        };
        assert!(invalid.validate().is_err());
    }
}
