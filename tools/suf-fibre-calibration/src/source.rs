// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Semantic validation for a vendor-neutral wrapper around vendor-native DAS
//! source artifacts. This is not a waveform decoder or a JSON parser.

use core::fmt;

/// Errors for a source-observation envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceEnvelopeError {
    /// An identifier or required text field is empty.
    EmptyField(&'static str),
    /// The acquired artifact's SHA-256 string is malformed.
    InvalidSha256(&'static str),
    /// Source artifact length is zero.
    EmptyArtifact,
    /// Numeric metadata is non-finite or outside its declared domain.
    InvalidNumber(&'static str),
    /// Spatial-axis kind and unit disagree.
    AxisUnitMismatch,
    /// Physical fibre chainage lacks a calibration reference.
    MissingCalibrationReference,
    /// A raw source was marked as derived or has parent artifacts.
    InvalidRawLineage,
    /// A derivative has no parent or lacks its method/parameter digest.
    IncompleteDerivativeLineage,
    /// A derivative was incorrectly marked as vendor-native.
    DerivativeMarkedNative,
    /// The phenomenon belongs to a distinct sensing modality, not this DAS envelope.
    UnsupportedPhenomenon,
}

impl fmt::Display for SourceEnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(name) => write!(f, "required source field is empty: {name}"),
            Self::InvalidSha256(name) => write!(f, "invalid SHA-256 field: {name}"),
            Self::EmptyArtifact => write!(f, "source artifact byte length must be positive"),
            Self::InvalidNumber(name) => write!(f, "invalid numeric source field: {name}"),
            Self::AxisUnitMismatch => write!(f, "spatial axis kind and unit disagree"),
            Self::MissingCalibrationReference => {
                write!(
                    f,
                    "physical fibre chainage requires a calibration reference"
                )
            }
            Self::InvalidRawLineage => {
                write!(f, "raw vendor-native artifacts cannot have derived parents")
            }
            Self::IncompleteDerivativeLineage => {
                write!(
                    f,
                    "derived artifact requires parent digest, method and parameter digest"
                )
            }
            Self::DerivativeMarkedNative => {
                write!(f, "a derived artifact cannot be marked vendor-native")
            }
            Self::UnsupportedPhenomenon => {
                write!(
                    f,
                    "phenomenon is not supported by this DAS source-observation profile"
                )
            }
        }
    }
}

impl std::error::Error for SourceEnvelopeError {}

/// Result type for source-envelope validation.
pub type SourceEnvelopeResult<T> = Result<T, SourceEnvelopeError>;

/// Whether a waveform artifact is original vendor-native data or a derivative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactStage {
    /// Original interrogator/vendor artifact.
    RawVendorNative,
    /// Unit/axis normalization or format conversion.
    NormalizedDerivative,
    /// Filtered, resampled, denoised or otherwise processed signal.
    ProcessedDerivative,
    /// Feature, embedding, spectrogram, event candidate or other extracted data.
    FeatureDerivative,
}

/// Spatial axis represented by the samples in an artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleAxisKind {
    /// Interrogator distance along its configured optical path.
    OpticalRange,
    /// Vendor-defined channel/bin index.
    VendorChannelIndex,
    /// Fibre-relative length, only after a separately qualified calibration.
    PhysicalFibreChainage,
}

/// Explicit unit for the sample axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleAxisUnit {
    /// Metres, but only on the declared fibre-relative axis.
    Metre,
    /// Dimensionless vendor channel/bin index.
    ChannelIndex,
}

/// Source artifact reference and integrity metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceArtifact {
    /// Stable URI/identifier for the acquired artifact.
    pub artifact_uri: String,
    /// Media type of the exact bytes.
    pub media_type: String,
    /// Exact acquired byte length.
    pub byte_length: u64,
    /// SHA-256 digest of the exact artifact bytes.
    pub sha256: String,
    /// True only when the artifact is the original vendor-native output.
    pub source_native: bool,
    /// True when the caller has actually verified the digest against acquired bytes.
    pub digest_verified: bool,
}

impl SourceArtifact {
    /// Validate artifact metadata. Digest verification itself occurs when bytes
    /// are acquired/read and is not performed by this type.
    pub fn validate(&self) -> SourceEnvelopeResult<()> {
        if self.artifact_uri.trim().is_empty() {
            return Err(SourceEnvelopeError::EmptyField("artifact_uri"));
        }
        if self.media_type.trim().is_empty() {
            return Err(SourceEnvelopeError::EmptyField("media_type"));
        }
        if self.byte_length == 0 {
            return Err(SourceEnvelopeError::EmptyArtifact);
        }
        validate_sha256(&self.sha256, "source_artifact.sha256")
    }
}

/// Sample-axis metadata. No geographic axis exists in this source envelope.
#[derive(Clone, Debug, PartialEq)]
pub struct SampleAxis {
    /// Type of coordinate carried by sample indices.
    pub kind: SampleAxisKind,
    /// Unit for start/step.
    pub unit: SampleAxisUnit,
    /// First sample coordinate.
    pub start: f64,
    /// Positive coordinate increment between samples.
    pub step: f64,
    /// Number of samples represented.
    pub sample_count: u64,
    /// Exact calibration reference, mandatory for physical fibre chainage.
    pub calibration_reference: Option<String>,
}

impl SampleAxis {
    /// Validate the axis without turning it into a route or geographic position.
    pub fn validate(&self) -> SourceEnvelopeResult<()> {
        if !self.start.is_finite() {
            return Err(SourceEnvelopeError::InvalidNumber("spatial_axis.start"));
        }
        if !self.step.is_finite() || self.step <= 0.0 {
            return Err(SourceEnvelopeError::InvalidNumber("spatial_axis.step"));
        }
        if self.sample_count == 0 {
            return Err(SourceEnvelopeError::InvalidNumber(
                "spatial_axis.sample_count",
            ));
        }
        match (self.kind, self.unit) {
            (SampleAxisKind::VendorChannelIndex, SampleAxisUnit::ChannelIndex) => {}
            (SampleAxisKind::OpticalRange, SampleAxisUnit::Metre) => {}
            (SampleAxisKind::PhysicalFibreChainage, SampleAxisUnit::Metre) => {
                if self
                    .calibration_reference
                    .as_ref()
                    .is_none_or(|reference| reference.trim().is_empty())
                {
                    return Err(SourceEnvelopeError::MissingCalibrationReference);
                }
            }
            _ => return Err(SourceEnvelopeError::AxisUnitMismatch),
        }
        Ok(())
    }
}

/// Observed quantity metadata for a DAS stream.
#[derive(Clone, Debug, PartialEq)]
pub struct ObservedQuantity {
    /// Machine-readable phenomenon name.
    pub phenomenon: String,
    /// Source-defined unit string/identifier.
    pub unit: String,
    /// Quantity semantics and source definition.
    pub quantity_semantics: String,
    /// Vendor channel pitch, if supplied. Not spatial resolution.
    pub channel_pitch_m: Option<f64>,
    /// Declared spatial resolution, if supplied independently.
    pub spatial_resolution_m: Option<f64>,
}

impl ObservedQuantity {
    /// Validate quantity declarations while keeping channel pitch and resolution
    /// as separate source fields.
    pub fn validate(&self) -> SourceEnvelopeResult<()> {
        if !matches!(
            self.phenomenon.as_str(),
            "axial_strain" | "strain_rate" | "phase" | "amplitude" | "other"
        ) {
            return Err(SourceEnvelopeError::UnsupportedPhenomenon);
        }
        for (name, value) in [
            ("observed_property.phenomenon", self.phenomenon.as_str()),
            ("observed_property.unit", self.unit.as_str()),
            (
                "observed_property.quantity_semantics",
                self.quantity_semantics.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(SourceEnvelopeError::EmptyField(name));
            }
        }
        for (name, value) in [
            ("sensor.channel_pitch_m", self.channel_pitch_m),
            ("sensor.spatial_resolution_m", self.spatial_resolution_m),
        ] {
            if let Some(value) = value
                && (!value.is_finite() || value <= 0.0)
            {
                return Err(SourceEnvelopeError::InvalidNumber(name));
            }
        }
        Ok(())
    }
}

/// Derivation lineage for the source artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactLineage {
    /// Processing/retention stage.
    pub stage: ArtifactStage,
    /// Parent source/derivative artifact digests.
    pub parent_sha256: Vec<String>,
    /// Method identifier and revision, absent only for original vendor-native data.
    pub method_revision: Option<String>,
    /// SHA-256 digest of the exact processing parameters/configuration.
    pub parameters_sha256: Option<String>,
}

impl ArtifactLineage {
    /// Validate raw-vs-derived lineage.
    pub fn validate(&self, artifact: &SourceArtifact) -> SourceEnvelopeResult<()> {
        match self.stage {
            ArtifactStage::RawVendorNative => {
                if !artifact.source_native || !self.parent_sha256.is_empty() {
                    return Err(SourceEnvelopeError::InvalidRawLineage);
                }
            }
            ArtifactStage::NormalizedDerivative
            | ArtifactStage::ProcessedDerivative
            | ArtifactStage::FeatureDerivative => {
                if artifact.source_native {
                    return Err(SourceEnvelopeError::DerivativeMarkedNative);
                }
                if self.parent_sha256.is_empty()
                    || self
                        .method_revision
                        .as_ref()
                        .is_none_or(|value| value.trim().is_empty())
                {
                    return Err(SourceEnvelopeError::IncompleteDerivativeLineage);
                }
                for digest in &self.parent_sha256 {
                    validate_sha256(digest, "parent_sha256")?;
                }
                let parameters_digest = self
                    .parameters_sha256
                    .as_ref()
                    .ok_or(SourceEnvelopeError::IncompleteDerivativeLineage)?;
                validate_sha256(parameters_digest, "parameters_sha256")?;
            }
        }
        Ok(())
    }
}

/// Minimum semantic source-observation contract.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceObservationEnvelope {
    /// Source waveform/measurement artifact.
    pub source_artifact: SourceArtifact,
    /// Sample-axis type and extent.
    pub spatial_axis: SampleAxis,
    /// Observed property and unit semantics.
    pub observed_quantity: ObservedQuantity,
    /// Raw/derived processing lineage.
    pub lineage: ArtifactLineage,
}

impl SourceObservationEnvelope {
    /// Validate structural-semantic invariants. This does not parse vendor bytes,
    /// authenticate a source publisher, enforce permissions or prove event truth.
    pub fn validate(&self) -> SourceEnvelopeResult<()> {
        self.source_artifact.validate()?;
        self.spatial_axis.validate()?;
        self.observed_quantity.validate()?;
        self.lineage.validate(&self.source_artifact)?;
        Ok(())
    }
}

fn validate_sha256(value: &str, field: &'static str) -> SourceEnvelopeResult<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(SourceEnvelopeError::InvalidSha256(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn envelope() -> SourceObservationEnvelope {
        SourceObservationEnvelope {
            source_artifact: SourceArtifact {
                artifact_uri: "urn:example:suf:artifact:source-1".to_owned(),
                media_type: "application/vnd.example.das".to_owned(),
                byte_length: 4096,
                sha256: DIGEST.to_owned(),
                source_native: true,
                digest_verified: false,
            },
            spatial_axis: SampleAxis {
                kind: SampleAxisKind::OpticalRange,
                unit: SampleAxisUnit::Metre,
                start: 0.0,
                step: 1.0,
                sample_count: 1000,
                calibration_reference: None,
            },
            observed_quantity: ObservedQuantity {
                phenomenon: "strain_rate".to_owned(),
                unit: "microstrain/second".to_owned(),
                quantity_semantics: "synthetic strain-rate quantity".to_owned(),
                channel_pitch_m: Some(1.0),
                spatial_resolution_m: None,
            },
            lineage: ArtifactLineage {
                stage: ArtifactStage::RawVendorNative,
                parent_sha256: Vec::new(),
                method_revision: None,
                parameters_sha256: None,
            },
        }
    }

    #[test]
    fn accepts_well_formed_raw_vendor_native_envelope() {
        assert!(envelope().validate().is_ok());
    }

    #[test]
    fn does_not_equate_channel_pitch_with_spatial_resolution() {
        let value = envelope();
        assert_eq!(value.observed_quantity.channel_pitch_m, Some(1.0));
        assert_eq!(value.observed_quantity.spatial_resolution_m, None);
        assert!(value.validate().is_ok());
    }

    #[test]
    fn rejects_geographic_degrees_as_a_source_axis() {
        let mut value = envelope();
        value.spatial_axis.unit = SampleAxisUnit::ChannelIndex;
        assert_eq!(value.validate(), Err(SourceEnvelopeError::AxisUnitMismatch));
    }

    #[test]
    fn requires_calibration_for_physical_fibre_chainage() {
        let mut value = envelope();
        value.spatial_axis.kind = SampleAxisKind::PhysicalFibreChainage;
        assert_eq!(
            value.validate(),
            Err(SourceEnvelopeError::MissingCalibrationReference)
        );
    }

    #[test]
    fn accepts_physical_chainage_only_with_calibration_reference() {
        let mut value = envelope();
        value.spatial_axis.kind = SampleAxisKind::PhysicalFibreChainage;
        value.spatial_axis.calibration_reference = Some("calibration-r4".to_owned());
        assert!(value.validate().is_ok());
    }

    #[test]
    fn rejects_raw_source_with_parent_artifacts() {
        let mut value = envelope();
        value.lineage.parent_sha256.push(DIGEST.to_owned());
        assert_eq!(
            value.validate(),
            Err(SourceEnvelopeError::InvalidRawLineage)
        );
    }

    #[test]
    fn rejects_derivative_without_parent_and_method() {
        let mut value = envelope();
        value.source_artifact.source_native = false;
        value.lineage.stage = ArtifactStage::ProcessedDerivative;
        assert_eq!(
            value.validate(),
            Err(SourceEnvelopeError::IncompleteDerivativeLineage)
        );
    }

    #[test]
    fn accepts_derivative_with_complete_parentage() {
        let mut value = envelope();
        value.source_artifact.source_native = false;
        value.lineage = ArtifactLineage {
            stage: ArtifactStage::ProcessedDerivative,
            parent_sha256: vec![DIGEST.to_owned()],
            method_revision: Some("bandpass-filter-v1".to_owned()),
            parameters_sha256: Some(DIGEST.to_owned()),
        };
        assert!(value.validate().is_ok());
    }

    #[test]
    fn rejects_derivative_marked_as_native() {
        let mut value = envelope();
        value.lineage = ArtifactLineage {
            stage: ArtifactStage::FeatureDerivative,
            parent_sha256: vec![DIGEST.to_owned()],
            method_revision: Some("feature-extractor-v1".to_owned()),
            parameters_sha256: Some(DIGEST.to_owned()),
        };
        assert_eq!(
            value.validate(),
            Err(SourceEnvelopeError::DerivativeMarkedNative)
        );
    }

    #[test]
    fn rejects_unverified_digest_format() {
        let mut value = envelope();
        value.source_artifact.sha256 = "invalid".to_owned();
        assert_eq!(
            value.validate(),
            Err(SourceEnvelopeError::InvalidSha256("source_artifact.sha256"))
        );
    }

    #[test]
    fn rejects_electrical_current_as_a_das_phenomenon() {
        let mut value = envelope();
        value.observed_quantity.phenomenon = "electrical_current".to_owned();
        assert_eq!(
            value.validate(),
            Err(SourceEnvelopeError::UnsupportedPhenomenon)
        );
    }

    #[test]
    fn rejects_temperature_as_a_das_phenomenon() {
        let mut value = envelope();
        value.observed_quantity.phenomenon = "temperature".to_owned();
        assert_eq!(
            value.validate(),
            Err(SourceEnvelopeError::UnsupportedPhenomenon)
        );
    }

    #[test]
    fn rejects_invalid_sample_axis_step() {
        let mut value = envelope();
        value.spatial_axis.step = f64::NAN;
        assert_eq!(
            value.validate(),
            Err(SourceEnvelopeError::InvalidNumber("spatial_axis.step"))
        );
    }

    #[test]
    fn rejects_non_positive_channel_pitch() {
        let mut value = envelope();
        value.observed_quantity.channel_pitch_m = Some(0.0);
        assert_eq!(
            value.validate(),
            Err(SourceEnvelopeError::InvalidNumber("sensor.channel_pitch_m"))
        );
    }
}
