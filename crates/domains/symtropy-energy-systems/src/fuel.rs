// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fuel-batch identity and evidence-bound property records.
//!
//! This module intentionally stores no default fuel values. A fuel property
//! must carry its evidence classification and source reference; uncertainty
//! may remain unknown rather than being silently treated as zero.

use serde::Serialize;
use std::fmt;

use crate::LowerHeatingValueJPerKg;

/// Broad fuel family. This is a classification, not evidence that a batch is
/// suitable or approved for any engine configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FuelFamily {
    Diesel,
    HydrotreatedVegetableOil,
    Biodiesel,
    NaturalGas,
    Biomethane,
    LiquefiedPetroleumGas,
    Methanol,
    Ethanol,
    Hydrogen,
    Ammonia,
    Other(String),
}

/// Evidence classes deliberately distinguish measurements from claims,
/// standards, literature and analyst scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EvidenceClass {
    LaboratoryMeasurement,
    SupplierCertificate,
    GovernmentDataset,
    PublishedLiterature,
    StandardOrRegulation,
    AnalystScenario,
}

/// A reference to the provenance of a property observation.
///
/// source_uri may be a URL, DOI, laboratory report identifier or controlled
/// repository reference. This layer does not claim that the referenced source
/// is authentic; an independent evidence verifier must resolve that question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceReference {
    source_id: String,
    source_uri: String,
    method: String,
    observed_at: String,
    evidence_class: EvidenceClass,
}

impl EvidenceReference {
    pub fn new(
        source_id: impl Into<String>,
        source_uri: impl Into<String>,
        method: impl Into<String>,
        observed_at: impl Into<String>,
        evidence_class: EvidenceClass,
    ) -> Result<Self, FuelRecordError> {
        let source_id = source_id.into();
        let source_uri = source_uri.into();
        let method = method.into();
        let observed_at = observed_at.into();

        for (field, value) in [
            ("source_id", source_id.as_str()),
            ("source_uri", source_uri.as_str()),
            ("method", method.as_str()),
            ("observed_at", observed_at.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(FuelRecordError::MissingEvidenceField { field });
            }
        }

        Ok(Self {
            source_id,
            source_uri,
            method,
            observed_at,
            evidence_class,
        })
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn source_uri(&self) -> &str {
        &self.source_uri
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn observed_at(&self) -> &str {
        &self.observed_at
    }

    pub fn evidence_class(&self) -> EvidenceClass {
        self.evidence_class
    }
}

/// A property value plus optional standard uncertainty and its evidence.
///
/// The standard uncertainty uses the same unit as value. None means unknown,
/// not zero uncertainty.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FuelProperty<T> {
    value: T,
    standard_uncertainty: Option<f64>,
    evidence: EvidenceReference,
}

impl<T> FuelProperty<T> {
    pub fn new(
        value: T,
        standard_uncertainty: Option<f64>,
        evidence: EvidenceReference,
    ) -> Result<Self, FuelRecordError> {
        if let Some(uncertainty) = standard_uncertainty
            && (!uncertainty.is_finite() || uncertainty < 0.0)
        {
            return Err(FuelRecordError::InvalidUncertainty { value: uncertainty });
        }

        Ok(Self {
            value,
            standard_uncertainty,
            evidence,
        })
    }

    pub const fn value(&self) -> &T {
        &self.value
    }

    pub const fn standard_uncertainty(&self) -> Option<f64> {
        self.standard_uncertainty
    }

    pub const fn evidence(&self) -> &EvidenceReference {
        &self.evidence
    }
}

/// Mass density in kilograms per cubic metre.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DensityKgPerCubicMeter(f64);

impl DensityKgPerCubicMeter {
    pub fn new(value: f64) -> Result<Self, FuelRecordError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(FuelRecordError::InvalidDensity { value });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// One sampled fuel batch. A batch is not an engine-compatibility approval.
///
/// The lower heating value is required and evidence-bound. Other properties
/// remain optional until sourced; absence of density or composition does not
/// imply a default value.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FuelBatch {
    batch_id: String,
    family: FuelFamily,
    grade: String,
    sampled_at: String,
    lower_heating_value: FuelProperty<LowerHeatingValueJPerKg>,
    density_kg_per_m3: Option<FuelProperty<DensityKgPerCubicMeter>>,
}

impl FuelBatch {
    pub fn new(
        batch_id: impl Into<String>,
        family: FuelFamily,
        grade: impl Into<String>,
        sampled_at: impl Into<String>,
        lower_heating_value: FuelProperty<LowerHeatingValueJPerKg>,
        density_kg_per_m3: Option<FuelProperty<DensityKgPerCubicMeter>>,
    ) -> Result<Self, FuelRecordError> {
        let batch_id = batch_id.into();
        let grade = grade.into();
        let sampled_at = sampled_at.into();

        for (field, value) in [
            ("batch_id", batch_id.as_str()),
            ("grade", grade.as_str()),
            ("sampled_at", sampled_at.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(FuelRecordError::MissingBatchField { field });
            }
        }

        Ok(Self {
            batch_id,
            family,
            grade,
            sampled_at,
            lower_heating_value,
            density_kg_per_m3,
        })
    }

    pub fn batch_id(&self) -> &str {
        &self.batch_id
    }

    pub const fn family(&self) -> &FuelFamily {
        &self.family
    }

    pub fn grade(&self) -> &str {
        &self.grade
    }

    pub const fn lower_heating_value(&self) -> &FuelProperty<LowerHeatingValueJPerKg> {
        &self.lower_heating_value
    }

    pub const fn density(&self) -> Option<&FuelProperty<DensityKgPerCubicMeter>> {
        self.density_kg_per_m3.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FuelRecordError {
    MissingEvidenceField { field: &'static str },
    MissingBatchField { field: &'static str },
    InvalidUncertainty { value: f64 },
    InvalidDensity { value: f64 },
}

impl fmt::Display for FuelRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEvidenceField { field } => {
                write!(f, "fuel evidence field {field} must not be empty")
            }
            Self::MissingBatchField { field } => {
                write!(f, "fuel batch field {field} must not be empty")
            }
            Self::InvalidUncertainty { value } => {
                write!(
                    f,
                    "standard uncertainty must be finite and non-negative; got {value}"
                )
            }
            Self::InvalidDensity { value } => {
                write!(f, "fuel density must be finite and positive; got {value}")
            }
        }
    }
}

impl std::error::Error for FuelRecordError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn lhv_property() -> FuelProperty<LowerHeatingValueJPerKg> {
        let evidence = EvidenceReference::new(
            "LAB-REPORT-001",
            "urn:luminous:test:lab-report-001",
            "bomb_calorimetry",
            "2026-10-01T12:00:00Z",
            EvidenceClass::LaboratoryMeasurement,
        )
        .unwrap();

        FuelProperty::new(
            LowerHeatingValueJPerKg::new(43_000_000.0).unwrap(),
            Some(100_000.0),
            evidence,
        )
        .unwrap()
    }

    #[test]
    fn fuel_batch_requires_sourced_lower_heating_value() {
        let batch = FuelBatch::new(
            "BATCH-001",
            FuelFamily::Diesel,
            "reference grade",
            "2026-10-01T12:00:00Z",
            lhv_property(),
            None,
        )
        .unwrap();

        assert_eq!(batch.batch_id(), "BATCH-001");
        assert_eq!(
            *batch.lower_heating_value().value(),
            LowerHeatingValueJPerKg::new(43_000_000.0).unwrap()
        );
        assert_eq!(
            batch.lower_heating_value().evidence().evidence_class(),
            EvidenceClass::LaboratoryMeasurement
        );
        assert!(batch.density().is_none());
    }

    #[test]
    fn unknown_uncertainty_remains_unknown() {
        let evidence = EvidenceReference::new(
            "VENDOR-SHEET-001",
            "urn:luminous:test:vendor-sheet-001",
            "published_specification",
            "2026-10-01",
            EvidenceClass::SupplierCertificate,
        )
        .unwrap();

        let property = FuelProperty::new(
            LowerHeatingValueJPerKg::new(42_000_000.0).unwrap(),
            None,
            evidence,
        )
        .unwrap();

        assert_eq!(property.standard_uncertainty(), None);
    }

    #[test]
    fn rejects_negative_or_non_finite_property_uncertainty() {
        let evidence = EvidenceReference::new(
            "LAB-REPORT-002",
            "urn:luminous:test:lab-report-002",
            "laboratory_test",
            "2026-10-01",
            EvidenceClass::LaboratoryMeasurement,
        )
        .unwrap();

        assert!(matches!(
            FuelProperty::new(10.0, Some(-0.1), evidence.clone()),
            Err(FuelRecordError::InvalidUncertainty { .. })
        ));
        assert!(matches!(
            FuelProperty::new(10.0, Some(f64::NAN), evidence),
            Err(FuelRecordError::InvalidUncertainty { .. })
        ));
    }

    #[test]
    fn rejects_empty_evidence_references() {
        assert!(matches!(
            EvidenceReference::new(
                "",
                "urn:luminous:test:empty",
                "test",
                "2026-10-01",
                EvidenceClass::AnalystScenario
            ),
            Err(FuelRecordError::MissingEvidenceField { field: "source_id" })
        ));
    }

    #[test]
    fn rejects_empty_batch_identity_fields() {
        assert!(matches!(
            FuelBatch::new(
                " ",
                FuelFamily::Other("synthetic".to_string()),
                "grade",
                "2026-10-01",
                lhv_property(),
                None
            ),
            Err(FuelRecordError::MissingBatchField { field: "batch_id" })
        ));
    }

    #[test]
    fn density_requires_positive_finite_value() {
        assert!(DensityKgPerCubicMeter::new(0.0).is_err());
        assert!(DensityKgPerCubicMeter::new(f64::INFINITY).is_err());
        assert!(DensityKgPerCubicMeter::new(850.0).is_ok());
    }
}
