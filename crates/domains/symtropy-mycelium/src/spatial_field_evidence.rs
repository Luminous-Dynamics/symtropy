// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Source-linked field observations and global predictions of hyphal density.
//!
//! Measured soil-core observations and spatial model predictions are distinct
//! types. Missing geolocation, depth, variance or method metadata remain
//! explicit; they are never imputed by this layer.
//!
//! This module also provides a geometric hyphal-volume fraction calculation.
//! It does not convert geometric volume into dry biomass, carbon stock or
//! nutrient flux: those require separately admitted density/turnover/chemistry
//! calibrations.

use std::fmt;

use crate::observation_evidence::{
    ObservationEvidenceId, ObservationEvidenceReceipt, Sha256Digest,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldStudyId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldSampleId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldRasterModelId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeographicCoordinate {
    pub latitude_degrees: f64,
    pub longitude_degrees: f64,
    /// Horizontal location uncertainty, when reported by the source.
    pub horizontal_uncertainty_km: Option<f64>,
}

impl GeographicCoordinate {
    pub fn validate(self) -> Result<(), SpatialFieldError> {
        if !self.latitude_degrees.is_finite()
            || !(-90.0..=90.0).contains(&self.latitude_degrees)
            || !self.longitude_degrees.is_finite()
            || !(-180.0..=180.0).contains(&self.longitude_degrees)
        {
            return Err(SpatialFieldError::InvalidCoordinate);
        }
        if self
            .horizontal_uncertainty_km
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(SpatialFieldError::InvalidCoordinateUncertainty);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilSamplingDepthCm {
    pub top_cm: f64,
    pub bottom_cm: f64,
}

impl SoilSamplingDepthCm {
    pub fn validate(self) -> Result<(), SpatialFieldError> {
        if !self.top_cm.is_finite()
            || !self.bottom_cm.is_finite()
            || self.top_cm < 0.0
            || self.bottom_cm <= self.top_cm
        {
            return Err(SpatialFieldError::InvalidSamplingDepth);
        }
        Ok(())
    }

    pub fn thickness_cm(self) -> f64 {
        self.bottom_cm - self.top_cm
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyphalLengthMeasurementMethod {
    SoilCoreMicroscopy,
    MembraneFilterMicroscopy,
    GridlineIntersection,
    ImageSegmentation,
    OtherDirectMethod,
    ProxyOrUnresolved,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HyphalLengthDensity {
    /// Directly measured or source-reported metres of hypha per cm^3 soil.
    pub metres_per_cm3_soil: f64,
    /// Standard error in the same units, if provided by the source.
    pub standard_error: Option<f64>,
}

impl HyphalLengthDensity {
    pub fn validate(self) -> Result<(), SpatialFieldError> {
        if !self.metres_per_cm3_soil.is_finite() || self.metres_per_cm3_soil < 0.0 {
            return Err(SpatialFieldError::InvalidHyphalLengthDensity);
        }
        if self
            .standard_error
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(SpatialFieldError::InvalidMeasurementUncertainty);
        }
        Ok(())
    }
}

/// One field measurement from one identified study/source record.
///
/// source_file_sha256 identifies the exact tabular/source file bytes.
/// source_record_key identifies the row or source-specific observation within
/// that file. observation_evidence, when present, links to an image/ROI receipt
/// for an associated microscopy acquisition; it is not required for every
/// historical literature-derived sample.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldHyphalDensityObservation {
    pub id: FieldSampleId,
    pub study: FieldStudyId,
    pub source_file_sha256: Sha256Digest,
    pub source_record_key: String,
    pub observation_evidence: Option<ObservationEvidenceId>,
    /// None means the source did not provide a reliable georeference.
    pub location: Option<GeographicCoordinate>,
    /// None means the sampled depth is unavailable in the source.
    pub sampling_depth: Option<SoilSamplingDepthCm>,
    pub density: HyphalLengthDensity,
    pub measurement_method: HyphalLengthMeasurementMethod,
    pub biological_replicate_count: Option<u32>,
}

impl FieldHyphalDensityObservation {
    pub fn validate(&self) -> Result<(), SpatialFieldError> {
        if self.source_file_sha256.is_zero() {
            return Err(SpatialFieldError::InvalidSourceDigest);
        }
        if self.source_record_key.trim().is_empty() || self.source_record_key.len() > 256 {
            return Err(SpatialFieldError::InvalidSourceRecordKey);
        }
        if let Some(location) = self.location {
            location.validate()?;
        }
        if let Some(depth) = self.sampling_depth {
            depth.validate()?;
        }
        self.density.validate()?;
        if self
            .biological_replicate_count
            .is_some_and(|count| count == 0)
        {
            return Err(SpatialFieldError::InvalidReplicateCount);
        }
        Ok(())
    }

    /// Validate the optional image/ROI receipt reference when one is present.
    pub fn validate_against_receipts(
        &self,
        receipts: &[ObservationEvidenceReceipt],
    ) -> Result<(), SpatialFieldError> {
        self.validate()?;
        if let Some(id) = self.observation_evidence {
            let receipt = receipts
                .iter()
                .find(|receipt| receipt.id == id)
                .ok_or(SpatialFieldError::MissingObservationEvidence(id))?;
            receipt
                .validate()
                .map_err(|_| SpatialFieldError::InvalidLinkedObservationEvidence(id))?;
        }
        Ok(())
    }

    /// Whether this record is eligible for geospatial training without
    /// imputing a coordinate or sampling depth.
    pub fn has_spatial_support(&self) -> bool {
        self.location.is_some() && self.sampling_depth.is_some()
    }
}

/// A cell from a spatial prediction raster. This is not a field observation.
#[derive(Debug, Clone, PartialEq)]
pub struct HyphalDensityRasterPrediction {
    pub model_id: FieldRasterModelId,
    pub model_revision_sha256: Sha256Digest,
    pub source_dataset_sha256: Sha256Digest,
    pub cell_center: GeographicCoordinate,
    pub cell_width_km: f64,
    pub cell_height_km: f64,
    pub predicted_density: f64,
    pub predicted_standard_deviation: f64,
    /// Scaled observation-intensity covariate; None means not provided.
    pub sampling_intensity: Option<f64>,
    pub training_study_count: Option<u32>,
}

impl HyphalDensityRasterPrediction {
    pub fn validate(&self) -> Result<(), SpatialFieldError> {
        if self.model_revision_sha256.is_zero() || self.source_dataset_sha256.is_zero() {
            return Err(SpatialFieldError::InvalidSourceDigest);
        }
        self.cell_center.validate()?;
        if !self.cell_width_km.is_finite()
            || self.cell_width_km <= 0.0
            || !self.cell_height_km.is_finite()
            || self.cell_height_km <= 0.0
        {
            return Err(SpatialFieldError::InvalidRasterCellGeometry);
        }
        if !self.predicted_density.is_finite() || self.predicted_density < 0.0 {
            return Err(SpatialFieldError::InvalidPredictedDensity);
        }
        if !self.predicted_standard_deviation.is_finite()
            || self.predicted_standard_deviation < 0.0
        {
            return Err(SpatialFieldError::InvalidPredictionUncertainty);
        }
        if self
            .sampling_intensity
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(SpatialFieldError::InvalidSamplingIntensity);
        }
        if self.training_study_count.is_some_and(|count| count == 0) {
            return Err(SpatialFieldError::InvalidTrainingStudyCount);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveHyphalRadiusUm {
    pub mean_radius_um: f64,
    pub standard_deviation_um: Option<f64>,
    pub imaged_hypha_count: u64,
    pub fungal_taxon_key: String,
    pub source_file_sha256: Sha256Digest,
    pub observation_evidence: Option<ObservationEvidenceId>,
}

impl EffectiveHyphalRadiusUm {
    pub fn validate(&self) -> Result<(), SpatialFieldError> {
        if !self.mean_radius_um.is_finite() || self.mean_radius_um <= 0.0 {
            return Err(SpatialFieldError::InvalidEffectiveRadius);
        }
        if self
            .standard_deviation_um
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(SpatialFieldError::InvalidRadiusUncertainty);
        }
        if self.imaged_hypha_count == 0 {
            return Err(SpatialFieldError::InvalidImagingSampleCount);
        }
        if self.fungal_taxon_key.trim().is_empty() || self.fungal_taxon_key.len() > 128 {
            return Err(SpatialFieldError::InvalidTaxonKey);
        }
        if self.source_file_sha256.is_zero() {
            return Err(SpatialFieldError::InvalidSourceDigest);
        }
        Ok(())
    }

    pub fn validate_against_receipts(
        &self,
        receipts: &[ObservationEvidenceReceipt],
    ) -> Result<(), SpatialFieldError> {
        self.validate()?;
        if let Some(id) = self.observation_evidence {
            let receipt = receipts
                .iter()
                .find(|receipt| receipt.id == id)
                .ok_or(SpatialFieldError::MissingObservationEvidence(id))?;
            receipt
                .validate()
                .map_err(|_| SpatialFieldError::InvalidLinkedObservationEvidence(id))?;
        }
        Ok(())
    }
}

/// Geometric hyphal volume divided by bulk soil volume. This is not dry
/// biomass and does not account for wall/lumen structure, branching overlap
/// or tissue density.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometricHyphalVolumeFraction(pub f64);

impl GeometricHyphalVolumeFraction {
    pub fn from_length_density_and_radius(
        density: HyphalLengthDensity,
        radius: EffectiveHyphalRadiusUm,
    ) -> Result<Self, SpatialFieldError> {
        density.validate()?;
        radius.validate()?;

        let radius_m = radius.mean_radius_um * 1.0e-6;
        let hyphal_volume_m3_per_cm3_soil =
            std::f64::consts::PI * radius_m * radius_m * density.metres_per_cm3_soil;
        // One cubic centimetre of bulk soil is 1e-6 cubic metres.
        let fraction = hyphal_volume_m3_per_cm3_soil / 1.0e-6;
        if !fraction.is_finite() || fraction < 0.0 {
            return Err(SpatialFieldError::NonFiniteDerivedQuantity);
        }
        Ok(Self(fraction))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpatialFieldError {
    InvalidCoordinate,
    InvalidCoordinateUncertainty,
    InvalidSamplingDepth,
    InvalidHyphalLengthDensity,
    InvalidMeasurementUncertainty,
    InvalidSourceDigest,
    InvalidSourceRecordKey,
    InvalidReplicateCount,
    InvalidRasterCellGeometry,
    InvalidPredictedDensity,
    InvalidPredictionUncertainty,
    InvalidSamplingIntensity,
    InvalidTrainingStudyCount,
    InvalidEffectiveRadius,
    InvalidRadiusUncertainty,
    InvalidImagingSampleCount,
    InvalidTaxonKey,
    MissingObservationEvidence(ObservationEvidenceId),
    InvalidLinkedObservationEvidence(ObservationEvidenceId),
    NonFiniteDerivedQuantity,
}

impl fmt::Display for SpatialFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCoordinate => write!(f, "latitude/longitude is outside valid bounds"),
            Self::InvalidCoordinateUncertainty => {
                write!(f, "horizontal coordinate uncertainty must be finite and non-negative")
            }
            Self::InvalidSamplingDepth => {
                write!(f, "soil sampling depth must be finite, non-negative and ordered")
            }
            Self::InvalidHyphalLengthDensity => {
                write!(f, "hyphal length density must be finite and non-negative")
            }
            Self::InvalidMeasurementUncertainty => {
                write!(f, "measurement standard error must be finite and non-negative")
            }
            Self::InvalidSourceDigest => write!(f, "source and model digests must not be zero"),
            Self::InvalidSourceRecordKey => {
                write!(f, "source record key must be non-empty and at most 256 bytes")
            }
            Self::InvalidReplicateCount => write!(f, "biological replicate count must be positive"),
            Self::InvalidRasterCellGeometry => {
                write!(f, "raster cell width and height must be finite and positive")
            }
            Self::InvalidPredictedDensity => {
                write!(f, "predicted hyphal density must be finite and non-negative")
            }
            Self::InvalidPredictionUncertainty => {
                write!(f, "prediction standard deviation must be finite and non-negative")
            }
            Self::InvalidSamplingIntensity => {
                write!(f, "sampling intensity must be finite and non-negative")
            }
            Self::InvalidTrainingStudyCount => {
                write!(f, "training study count must be positive when provided")
            }
            Self::InvalidEffectiveRadius => {
                write!(f, "effective hyphal radius must be finite and positive")
            }
            Self::InvalidRadiusUncertainty => {
                write!(f, "radius standard deviation must be finite and non-negative")
            }
            Self::InvalidImagingSampleCount => write!(f, "imaged hypha count must be positive"),
            Self::InvalidTaxonKey => {
                write!(f, "fungal taxon key must be non-empty and at most 128 bytes")
            }
            Self::MissingObservationEvidence(id) => {
                write!(f, "linked observation evidence {:?} is missing", id)
            }
            Self::InvalidLinkedObservationEvidence(id) => {
                write!(f, "linked observation evidence {:?} is invalid", id)
            }
            Self::NonFiniteDerivedQuantity => write!(f, "derived geometric volume fraction is invalid"),
        }
    }
}

impl std::error::Error for SpatialFieldError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> Sha256Digest {
        Sha256Digest([value; 32])
    }

    fn observation() -> FieldHyphalDensityObservation {
        FieldHyphalDensityObservation {
            id: FieldSampleId(1),
            study: FieldStudyId(10),
            source_file_sha256: digest(1),
            source_record_key: "study-10-core-1".to_string(),
            observation_evidence: None,
            location: Some(GeographicCoordinate {
                latitude_degrees: -26.2,
                longitude_degrees: 28.0,
                horizontal_uncertainty_km: Some(0.1),
            }),
            sampling_depth: Some(SoilSamplingDepthCm {
                top_cm: 0.0,
                bottom_cm: 10.0,
            }),
            density: HyphalLengthDensity {
                metres_per_cm3_soil: 2.5,
                standard_error: Some(0.2),
            },
            measurement_method: HyphalLengthMeasurementMethod::SoilCoreMicroscopy,
            biological_replicate_count: Some(5),
        }
    }

    fn prediction() -> HyphalDensityRasterPrediction {
        HyphalDensityRasterPrediction {
            model_id: FieldRasterModelId(1),
            model_revision_sha256: digest(2),
            source_dataset_sha256: digest(1),
            cell_center: GeographicCoordinate {
                latitude_degrees: -26.2,
                longitude_degrees: 28.0,
                horizontal_uncertainty_km: None,
            },
            cell_width_km: 1.0,
            cell_height_km: 1.0,
            predicted_density: 2.4,
            predicted_standard_deviation: 0.8,
            sampling_intensity: Some(0.65),
            training_study_count: Some(4),
        }
    }

    fn radius() -> EffectiveHyphalRadiusUm {
        EffectiveHyphalRadiusUm {
            mean_radius_um: 1.5,
            standard_deviation_um: Some(0.3),
            imaged_hypha_count: 10_000,
            fungal_taxon_key: "AMF-taxon-A".to_string(),
            source_file_sha256: digest(3),
            observation_evidence: None,
        }
    }

    #[test]
    fn field_measurement_requires_source_identity_and_valid_physical_units() {
        assert_eq!(observation().validate(), Ok(()));
        let mut item = observation();
        item.source_file_sha256 = digest(0);
        assert_eq!(item.validate(), Err(SpatialFieldError::InvalidSourceDigest));
        item = observation();
        item.density.metres_per_cm3_soil = -1.0;
        assert_eq!(
            item.validate(),
            Err(SpatialFieldError::InvalidHyphalLengthDensity)
        );
    }

    #[test]
    fn optional_field_image_receipt_must_resolve_and_validate() {
        let mut item = observation();
        item.observation_evidence = Some(ObservationEvidenceId(404));
        assert_eq!(
            item.validate_against_receipts(&[]),
            Err(SpatialFieldError::MissingObservationEvidence(
                ObservationEvidenceId(404)
            ))
        );
    }

    #[test]
    fn missing_geolocation_is_preserved_not_imputed() {
        let mut item = observation();
        item.location = None;
        assert_eq!(item.validate(), Ok(()));
        assert!(!item.has_spatial_support());
    }

    #[test]
    fn non_georeferenced_or_depth_unknown_records_are_not_spatial_training_rows() {
        let mut item = observation();
        item.sampling_depth = None;
        assert_eq!(item.validate(), Ok(()));
        assert!(!item.has_spatial_support());
    }

    #[test]
    fn invalid_geographic_bounds_and_depth_are_rejected() {
        let mut item = observation();
        item.location.as_mut().unwrap().latitude_degrees = 91.0;
        assert_eq!(item.validate(), Err(SpatialFieldError::InvalidCoordinate));
        item = observation();
        item.sampling_depth = Some(SoilSamplingDepthCm {
            top_cm: 10.0,
            bottom_cm: 5.0,
        });
        assert_eq!(item.validate(), Err(SpatialFieldError::InvalidSamplingDepth));
    }

    #[test]
    fn raster_prediction_is_a_distinct_validated_type() {
        assert_eq!(prediction().validate(), Ok(()));
        let mut item = prediction();
        item.predicted_standard_deviation = f64::NAN;
        assert_eq!(
            item.validate(),
            Err(SpatialFieldError::InvalidPredictionUncertainty)
        );
    }

    #[test]
    fn raster_prediction_rejects_invalid_sampling_support() {
        let mut item = prediction();
        item.sampling_intensity = Some(-0.1);
        assert_eq!(
            item.validate(),
            Err(SpatialFieldError::InvalidSamplingIntensity)
        );
    }

    #[test]
    fn radius_calibration_must_have_taxon_count_and_source_digest() {
        assert_eq!(radius().validate(), Ok(()));
        let mut item = radius();
        item.imaged_hypha_count = 0;
        assert_eq!(
            item.validate(),
            Err(SpatialFieldError::InvalidImagingSampleCount)
        );
    }

    #[test]
    fn geometric_volume_fraction_uses_explicit_length_and_radius_units() {
        let volume = GeometricHyphalVolumeFraction::from_length_density_and_radius(
            HyphalLengthDensity {
                metres_per_cm3_soil: 1.0,
                standard_error: None,
            },
            radius(),
        )
        .unwrap();
        let expected = std::f64::consts::PI * (1.5e-6_f64).powi(2) / 1.0e-6;
        assert!((volume.0 - expected).abs() < 1.0e-15);
        assert!(volume.0 > 0.0 && volume.0 < 1.0);
    }

    #[test]
    fn geometric_volume_fraction_does_not_return_dry_biomass() {
        let result = GeometricHyphalVolumeFraction::from_length_density_and_radius(
            HyphalLengthDensity {
                metres_per_cm3_soil: 100.0,
                standard_error: None,
            },
            radius(),
        )
        .unwrap();
        assert!(result.0 > 0.0);
    }

    #[test]
    fn invalid_measurement_and_radius_uncertainties_are_rejected() {
        let mut item = observation();
        item.density.standard_error = Some(f64::INFINITY);
        assert_eq!(
            item.validate(),
            Err(SpatialFieldError::InvalidMeasurementUncertainty)
        );
        let mut calibration = radius();
        calibration.mean_radius_um = f64::NAN;
        assert_eq!(
            calibration.validate(),
            Err(SpatialFieldError::InvalidEffectiveRadius)
        );
    }

    #[test]
    fn source_record_keys_are_bounded_and_replicate_counts_are_positive() {
        let mut item = observation();
        item.source_record_key = " ".to_string();
        assert_eq!(item.validate(), Err(SpatialFieldError::InvalidSourceRecordKey));
        item = observation();
        item.biological_replicate_count = Some(0);
        assert_eq!(item.validate(), Err(SpatialFieldError::InvalidReplicateCount));
    }
}
