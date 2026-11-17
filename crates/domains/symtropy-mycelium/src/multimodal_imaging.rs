// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Modality-aware evidence semantics for plant–mycorrhizal imaging.
//!
//! The module records what an instrument directly observes and maps that
//! observation to a bounded claim. It does not process raw CT/XRF/XANES/PET
//! data. It prevents a spatial image, tracer signal, labelled particle or
//! electrical trace from being silently promoted to nutrient delivery.
//!
//! In particular, no single modality observation can establish nutrient
//! delivery to a host. That claim requires a source-to-route-to-recipient
//! evidence chain and matched biological controls.

use std::fmt;

use crate::observation_evidence::{
    EvidenceReceiptError, ObservationEvidenceId, ObservationEvidenceReceipt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModalityObservationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagingModality {
    BrightfieldZStack,
    MicroComputedTomography,
    XrayFluorescence,
    XrayAbsorptionNearEdge,
    PositronEmissionTomography,
    FluorescentNanoparticleTracking,
    ElectricalSignalRecording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasuredObservable {
    FilamentMorphology,
    ElementalDistribution,
    ChemicalSpeciation,
    RadiotracerDynamics,
    LabelledParticleMovement,
    ElectricalSignalDynamics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiologicalClaim {
    HyphalMorphology2D,
    HyphalMorphology3D,
    HyphalConnectivity,
    ElementalDistribution,
    ChemicalSpeciation,
    RadiotracerDynamics,
    LabelledParticleMovement,
    ElectricalSignalDynamics,
    CytoplasmicFlow,
    MetabolicActivity,
    NutrientDeliveryToHost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimSupport {
    /// The modality directly measures the declared observable, with required
    /// calibration and identity metadata present.
    DirectObservable,
    /// The modality is relevant, but missing calibration, resolution, label,
    /// kinetics or orthogonal evidence prevents a direct quantitative claim.
    ContextualEvidenceOnly,
    /// This single observation cannot establish the requested claim.
    InsufficientAlone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialSupport {
    TwoDimensionalImage,
    ThreeDimensionalVolume,
    RegionOfInterest,
    BulkSample,
    WholeOrganism,
    FieldPlot,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantitationMode {
    Qualitative,
    Quantitative,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialResolutionUm {
    pub x_um: f64,
    pub y_um: f64,
    /// None means depth resolution is unknown or not applicable.
    pub z_um: Option<f64>,
}

impl SpatialResolutionUm {
    pub fn validate(self) -> Result<(), ModalityEvidenceError> {
        if !self.x_um.is_finite() || self.x_um <= 0.0 || !self.y_um.is_finite() || self.y_um <= 0.0
        {
            return Err(ModalityEvidenceError::InvalidSpatialResolution);
        }
        if self
            .z_um
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err(ModalityEvidenceError::InvalidSpatialResolution);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModalityObservation {
    pub id: ModalityObservationId,
    pub modality: ImagingModality,
    pub observable: MeasuredObservable,
    /// Source-linked acquisition receipt (image, volume, spectral map or
    /// time-series ROI). The receipt is validated separately.
    pub acquisition_evidence: ObservationEvidenceId,
    /// Optional calibration / reference-standard receipt.
    pub calibration_evidence: Option<ObservationEvidenceId>,
    /// Element, isotope, radiotracer or particle-label identity when relevant.
    pub analyte_or_label: Option<String>,
    pub spatial_support: SpatialSupport,
    pub spatial_resolution_um: Option<SpatialResolutionUm>,
    pub temporal_resolution_s: Option<f64>,
    pub quantitation: QuantitationMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalityEvidenceError {
    ZeroObservationId,
    ZeroEvidenceId,
    EmptyAnalyteOrLabel,
    InvalidSpatialResolution,
    InvalidTemporalResolution,
    ThreeDimensionalSupportRequiresDepthResolution,
    TwoDimensionalSupportCannotClaimThreeDimensionalResolution,
    ModalityObservableMismatch {
        modality: ImagingModality,
        observable: MeasuredObservable,
    },
    MissingAcquisitionReceipt(ObservationEvidenceId),
    MissingCalibrationReceipt(ObservationEvidenceId),
    InvalidAcquisitionReceipt {
        id: ObservationEvidenceId,
        source: EvidenceReceiptError,
    },
    InvalidCalibrationReceipt {
        id: ObservationEvidenceId,
        source: EvidenceReceiptError,
    },
    DuplicateReceipt(ObservationEvidenceId),
}

impl fmt::Display for ModalityEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroObservationId => write!(f, "modality observation ID must be non-zero"),
            Self::ZeroEvidenceId => write!(f, "evidence receipt ID must be non-zero"),
            Self::EmptyAnalyteOrLabel => write!(f, "analyte or tracer label must not be empty"),
            Self::InvalidSpatialResolution => {
                write!(f, "spatial resolution must be finite and positive")
            }
            Self::InvalidTemporalResolution => {
                write!(f, "temporal resolution must be finite and positive")
            }
            Self::ThreeDimensionalSupportRequiresDepthResolution => {
                write!(f, "3D spatial support requires calibrated depth resolution")
            }
            Self::TwoDimensionalSupportCannotClaimThreeDimensionalResolution => {
                write!(f, "2D support cannot be declared as a 3D volume")
            }
            Self::ModalityObservableMismatch {
                modality,
                observable,
            } => write!(
                f,
                "modality {:?} is incompatible with observable {:?}",
                modality, observable
            ),
            Self::MissingAcquisitionReceipt(id) => {
                write!(f, "missing acquisition evidence receipt {:?}", id)
            }
            Self::MissingCalibrationReceipt(id) => {
                write!(f, "missing calibration evidence receipt {:?}", id)
            }
            Self::InvalidAcquisitionReceipt { id, source } => {
                write!(f, "invalid acquisition receipt {:?}: {}", id, source)
            }
            Self::InvalidCalibrationReceipt { id, source } => {
                write!(f, "invalid calibration receipt {:?}: {}", id, source)
            }
            Self::DuplicateReceipt(id) => write!(f, "duplicate evidence receipt {:?}", id),
        }
    }
}

impl std::error::Error for ModalityEvidenceError {}

impl ModalityObservation {
    pub fn validate(&self) -> Result<(), ModalityEvidenceError> {
        if self.id.0 == 0 {
            return Err(ModalityEvidenceError::ZeroObservationId);
        }
        if self.acquisition_evidence.0 == 0 {
            return Err(ModalityEvidenceError::ZeroEvidenceId);
        }
        if self
            .calibration_evidence
            .is_some_and(|evidence| evidence.0 == 0)
        {
            return Err(ModalityEvidenceError::ZeroEvidenceId);
        }
        if self
            .analyte_or_label
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(ModalityEvidenceError::EmptyAnalyteOrLabel);
        }
        if let Some(resolution) = self.spatial_resolution_um {
            resolution.validate()?;
        }
        if self
            .temporal_resolution_s
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err(ModalityEvidenceError::InvalidTemporalResolution);
        }
        if self.spatial_support == SpatialSupport::ThreeDimensionalVolume
            && self
                .spatial_resolution_um
                .is_none_or(|resolution| resolution.z_um.is_none())
        {
            return Err(ModalityEvidenceError::ThreeDimensionalSupportRequiresDepthResolution);
        }
        if self.spatial_support == SpatialSupport::TwoDimensionalImage
            && self
                .spatial_resolution_um
                .is_some_and(|resolution| resolution.z_um.is_some())
        {
            return Err(
                ModalityEvidenceError::TwoDimensionalSupportCannotClaimThreeDimensionalResolution,
            );
        }
        let expected = match self.modality {
            ImagingModality::BrightfieldZStack | ImagingModality::MicroComputedTomography => {
                MeasuredObservable::FilamentMorphology
            }
            ImagingModality::XrayFluorescence => MeasuredObservable::ElementalDistribution,
            ImagingModality::XrayAbsorptionNearEdge => MeasuredObservable::ChemicalSpeciation,
            ImagingModality::PositronEmissionTomography => MeasuredObservable::RadiotracerDynamics,
            ImagingModality::FluorescentNanoparticleTracking => {
                MeasuredObservable::LabelledParticleMovement
            }
            ImagingModality::ElectricalSignalRecording => {
                MeasuredObservable::ElectricalSignalDynamics
            }
        };
        if self.observable != expected {
            return Err(ModalityEvidenceError::ModalityObservableMismatch {
                modality: self.modality,
                observable: self.observable,
            });
        }
        Ok(())
    }

    pub fn validate_against_receipts(
        &self,
        receipts: &[ObservationEvidenceReceipt],
    ) -> Result<(), ModalityEvidenceError> {
        self.validate()?;
        let mut receipt_by_id = std::collections::BTreeMap::new();
        for receipt in receipts {
            if receipt_by_id.insert(receipt.id, receipt).is_some() {
                return Err(ModalityEvidenceError::DuplicateReceipt(receipt.id));
            }
        }
        let acquisition = receipt_by_id.get(&self.acquisition_evidence).ok_or(
            ModalityEvidenceError::MissingAcquisitionReceipt(self.acquisition_evidence),
        )?;
        acquisition.validate().map_err(|source| {
            ModalityEvidenceError::InvalidAcquisitionReceipt {
                id: self.acquisition_evidence,
                source,
            }
        })?;
        if let Some(id) = self.calibration_evidence {
            let calibration = receipt_by_id
                .get(&id)
                .ok_or(ModalityEvidenceError::MissingCalibrationReceipt(id))?;
            calibration.validate().map_err(|source| {
                ModalityEvidenceError::InvalidCalibrationReceipt { id, source }
            })?;
        }
        Ok(())
    }

    /// Qualify one claim against this observation alone.
    ///
    /// Nutrient delivery to a host always needs a separate source-to-route-
    /// to-recipient chain and matched biological controls; no single modality
    /// is sufficient.
    pub fn assess_claim(
        &self,
        claim: BiologicalClaim,
    ) -> Result<ClaimSupport, ModalityEvidenceError> {
        self.validate()?;
        if claim == BiologicalClaim::NutrientDeliveryToHost {
            return Ok(ClaimSupport::InsufficientAlone);
        }
        let has_calibration = self.calibration_evidence.is_some();
        let has_label = self
            .analyte_or_label
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty());
        let has_spatial_resolution = self.spatial_resolution_um.is_some();
        let has_temporal_resolution = self.temporal_resolution_s.is_some();

        let support = match (self.modality, claim) {
            (
                ImagingModality::BrightfieldZStack | ImagingModality::MicroComputedTomography,
                BiologicalClaim::HyphalMorphology3D,
            ) => {
                if self.spatial_support == SpatialSupport::ThreeDimensionalVolume
                    && has_spatial_resolution
                    && has_calibration
                {
                    ClaimSupport::DirectObservable
                } else {
                    ClaimSupport::ContextualEvidenceOnly
                }
            }
            (
                ImagingModality::BrightfieldZStack | ImagingModality::MicroComputedTomography,
                BiologicalClaim::HyphalMorphology2D,
            ) => ClaimSupport::DirectObservable,
            (
                ImagingModality::BrightfieldZStack | ImagingModality::MicroComputedTomography,
                BiologicalClaim::HyphalConnectivity,
            ) => ClaimSupport::ContextualEvidenceOnly,
            (ImagingModality::XrayFluorescence, BiologicalClaim::ElementalDistribution) => {
                if has_calibration && has_label {
                    ClaimSupport::DirectObservable
                } else {
                    ClaimSupport::ContextualEvidenceOnly
                }
            }
            (ImagingModality::XrayAbsorptionNearEdge, BiologicalClaim::ChemicalSpeciation) => {
                if has_calibration && has_label {
                    ClaimSupport::DirectObservable
                } else {
                    ClaimSupport::ContextualEvidenceOnly
                }
            }
            (ImagingModality::PositronEmissionTomography, BiologicalClaim::RadiotracerDynamics) => {
                if has_calibration && has_label && has_temporal_resolution {
                    ClaimSupport::DirectObservable
                } else {
                    ClaimSupport::ContextualEvidenceOnly
                }
            }
            (ImagingModality::PositronEmissionTomography, BiologicalClaim::MetabolicActivity) => {
                ClaimSupport::ContextualEvidenceOnly
            }
            (
                ImagingModality::FluorescentNanoparticleTracking,
                BiologicalClaim::LabelledParticleMovement,
            ) => {
                if has_label && has_temporal_resolution {
                    ClaimSupport::DirectObservable
                } else {
                    ClaimSupport::ContextualEvidenceOnly
                }
            }
            (
                ImagingModality::ElectricalSignalRecording,
                BiologicalClaim::ElectricalSignalDynamics,
            ) => {
                if has_temporal_resolution && has_calibration {
                    ClaimSupport::DirectObservable
                } else {
                    ClaimSupport::ContextualEvidenceOnly
                }
            }
            _ => ClaimSupport::InsufficientAlone,
        };
        Ok(support)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        modality: ImagingModality,
        observable: MeasuredObservable,
    ) -> ModalityObservation {
        ModalityObservation {
            id: ModalityObservationId(1),
            modality,
            observable,
            acquisition_evidence: ObservationEvidenceId(10),
            calibration_evidence: Some(ObservationEvidenceId(11)),
            analyte_or_label: Some("P-31 tracer".to_owned()),
            spatial_support: SpatialSupport::ThreeDimensionalVolume,
            spatial_resolution_um: Some(SpatialResolutionUm {
                x_um: 5.0,
                y_um: 5.0,
                z_um: Some(10.0),
            }),
            temporal_resolution_s: Some(60.0),
            quantitation: QuantitationMode::Quantitative,
        }
    }

    #[test]
    fn micro_ct_supports_3d_morphology_not_nutrient_flow() {
        let item = observation(
            ImagingModality::MicroComputedTomography,
            MeasuredObservable::FilamentMorphology,
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::HyphalMorphology3D),
            Ok(ClaimSupport::DirectObservable)
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::CytoplasmicFlow),
            Ok(ClaimSupport::InsufficientAlone)
        );
    }

    #[test]
    fn brightfield_zstack_does_not_establish_chemical_composition() {
        let item = observation(
            ImagingModality::BrightfieldZStack,
            MeasuredObservable::FilamentMorphology,
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::ChemicalSpeciation),
            Ok(ClaimSupport::InsufficientAlone)
        );
    }

    #[test]
    fn xrf_supports_elemental_distribution_not_speciation() {
        let item = observation(
            ImagingModality::XrayFluorescence,
            MeasuredObservable::ElementalDistribution,
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::ElementalDistribution),
            Ok(ClaimSupport::DirectObservable)
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::ChemicalSpeciation),
            Ok(ClaimSupport::InsufficientAlone)
        );
    }

    #[test]
    fn xanes_supports_speciation_not_transport() {
        let item = observation(
            ImagingModality::XrayAbsorptionNearEdge,
            MeasuredObservable::ChemicalSpeciation,
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::ChemicalSpeciation),
            Ok(ClaimSupport::DirectObservable)
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::NutrientDeliveryToHost),
            Ok(ClaimSupport::InsufficientAlone)
        );
    }

    #[test]
    fn pet_tracer_dynamics_are_not_automatically_metabolic_rate() {
        let item = observation(
            ImagingModality::PositronEmissionTomography,
            MeasuredObservable::RadiotracerDynamics,
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::RadiotracerDynamics),
            Ok(ClaimSupport::DirectObservable)
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::MetabolicActivity),
            Ok(ClaimSupport::ContextualEvidenceOnly)
        );
    }

    #[test]
    fn labelled_particle_tracking_is_not_nutrient_delivery() {
        let item = observation(
            ImagingModality::FluorescentNanoparticleTracking,
            MeasuredObservable::LabelledParticleMovement,
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::LabelledParticleMovement),
            Ok(ClaimSupport::DirectObservable)
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::NutrientDeliveryToHost),
            Ok(ClaimSupport::InsufficientAlone)
        );
    }

    #[test]
    fn electrical_recording_supports_electrical_dynamics_only() {
        let item = observation(
            ImagingModality::ElectricalSignalRecording,
            MeasuredObservable::ElectricalSignalDynamics,
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::ElectricalSignalDynamics),
            Ok(ClaimSupport::DirectObservable)
        );
        assert_eq!(
            item.assess_claim(BiologicalClaim::CytoplasmicFlow),
            Ok(ClaimSupport::InsufficientAlone)
        );
    }

    #[test]
    fn calibration_and_analyte_identity_are_required_for_direct_xrf_support() {
        let mut item = observation(
            ImagingModality::XrayFluorescence,
            MeasuredObservable::ElementalDistribution,
        );
        item.calibration_evidence = None;
        assert_eq!(
            item.assess_claim(BiologicalClaim::ElementalDistribution),
            Ok(ClaimSupport::ContextualEvidenceOnly)
        );
        item.calibration_evidence = Some(ObservationEvidenceId(11));
        item.analyte_or_label = None;
        assert_eq!(
            item.assess_claim(BiologicalClaim::ElementalDistribution),
            Ok(ClaimSupport::ContextualEvidenceOnly)
        );
    }

    #[test]
    fn single_modality_never_proves_nutrient_delivery() {
        for item in [
            observation(
                ImagingModality::MicroComputedTomography,
                MeasuredObservable::FilamentMorphology,
            ),
            observation(
                ImagingModality::XrayFluorescence,
                MeasuredObservable::ElementalDistribution,
            ),
            observation(
                ImagingModality::XrayAbsorptionNearEdge,
                MeasuredObservable::ChemicalSpeciation,
            ),
            observation(
                ImagingModality::PositronEmissionTomography,
                MeasuredObservable::RadiotracerDynamics,
            ),
        ] {
            assert_eq!(
                item.assess_claim(BiologicalClaim::NutrientDeliveryToHost),
                Ok(ClaimSupport::InsufficientAlone)
            );
        }
    }

    #[test]
    fn incompatible_modality_observable_pair_is_rejected() {
        let mut item = observation(
            ImagingModality::XrayFluorescence,
            MeasuredObservable::ElementalDistribution,
        );
        item.observable = MeasuredObservable::ChemicalSpeciation;
        assert_eq!(
            item.validate(),
            Err(ModalityEvidenceError::ModalityObservableMismatch {
                modality: ImagingModality::XrayFluorescence,
                observable: MeasuredObservable::ChemicalSpeciation
            })
        );
    }

    #[test]
    fn three_dimensional_support_requires_depth_resolution() {
        let mut item = observation(
            ImagingModality::MicroComputedTomography,
            MeasuredObservable::FilamentMorphology,
        );
        item.spatial_resolution_um = Some(SpatialResolutionUm {
            x_um: 5.0,
            y_um: 5.0,
            z_um: None,
        });
        assert_eq!(
            item.validate(),
            Err(ModalityEvidenceError::ThreeDimensionalSupportRequiresDepthResolution)
        );
    }

    #[test]
    fn missing_temporal_resolution_downgrades_pet_to_contextual_evidence() {
        let mut item = observation(
            ImagingModality::PositronEmissionTomography,
            MeasuredObservable::RadiotracerDynamics,
        );
        item.temporal_resolution_s = None;
        assert_eq!(
            item.assess_claim(BiologicalClaim::RadiotracerDynamics),
            Ok(ClaimSupport::ContextualEvidenceOnly)
        );
    }
}
