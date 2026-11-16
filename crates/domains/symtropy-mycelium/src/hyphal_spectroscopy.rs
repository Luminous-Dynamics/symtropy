// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Source-linked hyphal SIP-Raman and SRS spectral observations.
//!
//! This module stores acquisition metadata and measured spectral bands. It
//! intentionally does not infer metabolic rate, nutrient flux or viability
//! from band intensity. Incident laser energy is a setting-derived proxy, not
//! absorbed dose or a phototoxicity threshold.

use std::fmt;

use crate::hyphal_graph::{HyphalGraph, SegmentId};
use crate::observation_evidence::{
    EvidenceGraphTarget, EvidenceReceiptError, ObservationEvidenceId, ObservationEvidenceReceipt,
    ObservationSubject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RamanObservationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RamanModality {
    SipSpontaneousRaman,
    SpontaneousRaman,
    StimulatedRamanScattering,
}

impl RamanModality {
    const fn required_subject(self) -> ObservationSubject {
        match self {
            Self::SipSpontaneousRaman | Self::SpontaneousRaman => ObservationSubject::RamanSpectrum,
            Self::StimulatedRamanScattering => ObservationSubject::RamanChemicalMap,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsotopeLabel {
    UnlabelledControl,
    DeuteratedGlucose,
    Carbon13Substrate,
    Nitrogen15Substrate,
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpectralStressor {
    None,
    Copper {
        concentration_umol_per_litre: f64,
    },
    Other {
        name: String,
        concentration_umol_per_litre: Option<f64>,
    },
}

impl SpectralStressor {
    fn validate(&self) -> Result<(), RamanEvidenceError> {
        match self {
            Self::None => Ok(()),
            Self::Copper {
                concentration_umol_per_litre,
            } => {
                if !concentration_umol_per_litre.is_finite() || *concentration_umol_per_litre < 0.0
                {
                    return Err(RamanEvidenceError::InvalidStressor);
                }
                Ok(())
            }
            Self::Other {
                name,
                concentration_umol_per_litre,
            } => {
                if name.trim().is_empty()
                    || concentration_umol_per_litre
                        .is_some_and(|value| !value.is_finite() || value < 0.0)
                {
                    return Err(RamanEvidenceError::InvalidStressor);
                }
                Ok(())
            }
        }
    }
}

/// Acquisition metadata. The incident-energy estimate is derived from nominal
/// settings only; it is not an absorbed dose or a biological damage estimate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RamanAcquisition {
    pub excitation_wavelength_nm: f64,
    pub laser_power_mw: f64,
    pub integration_time_ms: f64,
    pub exposure_count: u32,
    /// Optional measured spot area; without it, energy density is unknown.
    pub illuminated_spot_area_um2: Option<f64>,
}

impl RamanAcquisition {
    pub fn validate(&self) -> Result<(), RamanEvidenceError> {
        if !self.excitation_wavelength_nm.is_finite()
            || self.excitation_wavelength_nm <= 0.0
            || !self.laser_power_mw.is_finite()
            || self.laser_power_mw <= 0.0
            || !self.integration_time_ms.is_finite()
            || self.integration_time_ms <= 0.0
            || self.exposure_count == 0
            || self
                .illuminated_spot_area_um2
                .is_some_and(|area| !area.is_finite() || area <= 0.0)
        {
            return Err(RamanEvidenceError::InvalidAcquisition);
        }
        Ok(())
    }

    /// Nominal incident optical energy, in millijoules.
    ///
    /// This must not be called absorbed dose: optical coupling, sample
    /// absorption and full spot geometry are not represented by this value.
    pub fn nominal_incident_energy_mj(&self) -> f64 {
        self.laser_power_mw * self.integration_time_ms * f64::from(self.exposure_count) / 1_000.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RamanBandMeasurement {
    pub wavenumber_cm_inverse: f64,
    /// Raw detector intensity in arbitrary instrument units.
    pub raw_intensity_au: f64,
    /// Optional local background estimate in the same instrument units.
    pub background_intensity_au: Option<f64>,
}

impl RamanBandMeasurement {
    fn validate(&self) -> Result<(), RamanEvidenceError> {
        if !self.wavenumber_cm_inverse.is_finite()
            || self.wavenumber_cm_inverse <= 0.0
            || !self.raw_intensity_au.is_finite()
            || self.raw_intensity_au < 0.0
            || self
                .background_intensity_au
                .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(RamanEvidenceError::InvalidSpectralBand);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HyphalRamanObservation {
    pub id: RamanObservationId,
    pub source_receipt: ObservationEvidenceId,
    pub modality: RamanModality,
    pub segment: Option<SegmentId>,
    pub time_hours: f64,
    pub isotope_label: IsotopeLabel,
    pub stressor: SpectralStressor,
    pub acquisition: RamanAcquisition,
    /// Number of prior exposures to the same ROI, excluding this acquisition.
    pub prior_exposures_same_roi: u32,
    pub bands: Vec<RamanBandMeasurement>,
    /// Optional independent calibration/reference receipt. This does not
    /// itself convert spectral intensity to a metabolic rate.
    pub calibration_reference: Option<ObservationEvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RamanEvidenceError {
    InvalidTimestamp,
    InvalidAcquisition,
    InvalidStressor,
    InvalidSpectralBand,
    EmptySpectrum,
    NonIncreasingWavenumber,
    MissingSourceReceipt(ObservationEvidenceId),
    DuplicateSourceReceipt(ObservationEvidenceId),
    InvalidSourceReceipt {
        id: ObservationEvidenceId,
        source: EvidenceReceiptError,
    },
    WrongSourceSubject {
        id: ObservationEvidenceId,
        expected: ObservationSubject,
    },
    SegmentTargetMismatch(ObservationEvidenceId),
    MissingSegment(SegmentId),
    MissingCalibrationReceipt(ObservationEvidenceId),
    WrongCalibrationSubject(ObservationEvidenceId),
    CalibrationIsNotIndependent,
    EmptyIsotopeLabel,
}

impl fmt::Display for RamanEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimestamp => {
                write!(f, "Raman timestamp must be finite and non-negative")
            }
            Self::InvalidAcquisition => write!(f, "Raman acquisition settings are invalid"),
            Self::InvalidStressor => write!(f, "Raman stressor identity/concentration is invalid"),
            Self::InvalidSpectralBand => write!(f, "Raman spectral band contains invalid values"),
            Self::EmptySpectrum => write!(f, "Raman observation must contain at least one band"),
            Self::NonIncreasingWavenumber => {
                write!(
                    f,
                    "Raman bands must be sorted by strictly increasing wavenumber"
                )
            }
            Self::MissingSourceReceipt(id) => write!(f, "missing Raman source receipt {:?}", id),
            Self::DuplicateSourceReceipt(id) => {
                write!(f, "duplicate Raman source receipt {:?}", id)
            }
            Self::InvalidSourceReceipt { id, source } => {
                write!(f, "invalid Raman source receipt {:?}: {:?}", id, source)
            }
            Self::WrongSourceSubject { id, expected } => {
                write!(
                    f,
                    "Raman receipt {:?} has wrong subject; expected {:?}",
                    id, expected
                )
            }
            Self::SegmentTargetMismatch(id) => {
                write!(
                    f,
                    "Raman receipt {:?} does not target the stated hyphal segment",
                    id
                )
            }
            Self::MissingSegment(id) => {
                write!(f, "Raman observation references missing segment {:?}", id)
            }
            Self::MissingCalibrationReceipt(id) => {
                write!(f, "missing independent calibration receipt {:?}", id)
            }
            Self::WrongCalibrationSubject(id) => {
                write!(
                    f,
                    "Raman calibration receipt {:?} has the wrong subject",
                    id
                )
            }
            Self::CalibrationIsNotIndependent => {
                write!(
                    f,
                    "calibration receipt must differ from measured spectrum receipt"
                )
            }
            Self::EmptyIsotopeLabel => write!(f, "other isotope label must not be empty"),
        }
    }
}

impl std::error::Error for RamanEvidenceError {}

impl HyphalRamanObservation {
    pub fn validate(&self) -> Result<(), RamanEvidenceError> {
        if !self.time_hours.is_finite() || self.time_hours < 0.0 {
            return Err(RamanEvidenceError::InvalidTimestamp);
        }
        self.acquisition.validate()?;
        self.stressor.validate()?;
        if matches!(&self.isotope_label, IsotopeLabel::Other(name) if name.trim().is_empty()) {
            return Err(RamanEvidenceError::EmptyIsotopeLabel);
        }
        if self.bands.is_empty() {
            return Err(RamanEvidenceError::EmptySpectrum);
        }
        let mut previous = None;
        for band in &self.bands {
            band.validate()?;
            if previous.is_some_and(|last: f64| band.wavenumber_cm_inverse <= last) {
                return Err(RamanEvidenceError::NonIncreasingWavenumber);
            }
            previous = Some(band.wavenumber_cm_inverse);
        }
        if self.calibration_reference == Some(self.source_receipt) {
            return Err(RamanEvidenceError::CalibrationIsNotIndependent);
        }
        Ok(())
    }

    pub fn validate_against(
        &self,
        graph: &HyphalGraph,
        receipts: &[ObservationEvidenceReceipt],
    ) -> Result<(), RamanEvidenceError> {
        self.validate()?;
        let source_matches: Vec<_> = receipts
            .iter()
            .filter(|item| item.id == self.source_receipt)
            .collect();
        if source_matches.is_empty() {
            return Err(RamanEvidenceError::MissingSourceReceipt(
                self.source_receipt,
            ));
        }
        if source_matches.len() != 1 {
            return Err(RamanEvidenceError::DuplicateSourceReceipt(
                self.source_receipt,
            ));
        }
        let receipt = source_matches[0];
        receipt
            .validate()
            .map_err(|source| RamanEvidenceError::InvalidSourceReceipt {
                id: self.source_receipt,
                source,
            })?;
        let expected = self.modality.required_subject();
        if receipt.subject != expected {
            return Err(RamanEvidenceError::WrongSourceSubject {
                id: self.source_receipt,
                expected,
            });
        }
        if let Some(segment) = self.segment {
            if graph.segment(segment).is_none() {
                return Err(RamanEvidenceError::MissingSegment(segment));
            }
            if receipt.graph_target != Some(EvidenceGraphTarget::Segment(segment)) {
                return Err(RamanEvidenceError::SegmentTargetMismatch(
                    self.source_receipt,
                ));
            }
        }
        if let Some(calibration_id) = self.calibration_reference {
            let calibration_matches: Vec<_> = receipts
                .iter()
                .filter(|item| item.id == calibration_id)
                .collect();
            if calibration_matches.is_empty() {
                return Err(RamanEvidenceError::MissingCalibrationReceipt(
                    calibration_id,
                ));
            }
            if calibration_matches.len() != 1 {
                return Err(RamanEvidenceError::DuplicateSourceReceipt(calibration_id));
            }
            let calibration = calibration_matches[0];
            calibration
                .validate()
                .map_err(|source| RamanEvidenceError::InvalidSourceReceipt {
                    id: calibration_id,
                    source,
                })?;
            if calibration.subject != ObservationSubject::RamanCalibrationReference {
                return Err(RamanEvidenceError::WrongCalibrationSubject(calibration_id));
            }
        }
        Ok(())
    }

    /// A source spectrum plus an independent calibration receipt is not
    /// sufficient to claim a metabolic rate; the model and units must also be
    /// explicitly recorded by a qualified downstream analysis.
    pub const fn has_calibration_reference(&self) -> bool {
        self.calibration_reference.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyphal_graph::{
        HyphalNode, HyphalNodeKind, HyphalSegment, HyphalSegmentState, NodeId, Position3Mm,
    };
    use crate::observation_evidence::{
        EvidenceDisposition, ImageFrameIndex, ImageFrameReference, ImageRoiBoundsPx,
        ObservationRoiId, PhysicalSamplingCalibration, Sha256Digest,
    };

    fn receipt(
        id: u64,
        subject: ObservationSubject,
        target: Option<EvidenceGraphTarget>,
    ) -> ObservationEvidenceReceipt {
        ObservationEvidenceReceipt {
            id: ObservationEvidenceId(id),
            source_file_sha256: Sha256Digest([1; 32]),
            frame: ImageFrameReference {
                frame: ImageFrameIndex(2),
                slice: Some(crate::observation_evidence::ImageSliceIndex(4)),
                timestamp_s: 3_600.0,
            },
            roi: ImageRoiBoundsPx {
                id: ObservationRoiId(8),
                x_min: 0,
                y_min: 0,
                width_px: 12,
                height_px: 8,
            },
            calibration: PhysicalSamplingCalibration {
                pixel_width_um: 0.2,
                pixel_height_um: 0.2,
                voxel_depth_um: Some(1.0),
                frame_interval_s: None,
            },
            processor_name: "spectral-test".to_owned(),
            processor_revision: "v0.1.0".to_owned(),
            processor_parameters_sha256: Sha256Digest([2; 32]),
            subject,
            graph_target: target,
            graph_event: None,
            uncertainty: None,
            disposition: EvidenceDisposition::MachineGenerated,
        }
    }

    fn observation(modality: RamanModality) -> HyphalRamanObservation {
        HyphalRamanObservation {
            id: RamanObservationId(1),
            source_receipt: ObservationEvidenceId(10),
            modality,
            segment: Some(SegmentId(7)),
            time_hours: 1.0,
            isotope_label: IsotopeLabel::DeuteratedGlucose,
            stressor: SpectralStressor::Copper {
                concentration_umol_per_litre: 500.0,
            },
            acquisition: RamanAcquisition {
                excitation_wavelength_nm: 785.0,
                laser_power_mw: 2.0,
                integration_time_ms: 100.0,
                exposure_count: 3,
                illuminated_spot_area_um2: None,
            },
            prior_exposures_same_roi: 2,
            bands: vec![
                RamanBandMeasurement {
                    wavenumber_cm_inverse: 2_100.0,
                    raw_intensity_au: 50.0,
                    background_intensity_au: Some(3.0),
                },
                RamanBandMeasurement {
                    wavenumber_cm_inverse: 2_900.0,
                    raw_intensity_au: 100.0,
                    background_intensity_au: Some(10.0),
                },
            ],
            calibration_reference: None,
        }
    }

    fn graph() -> HyphalGraph {
        let mut graph = HyphalGraph::new();
        graph
            .add_node(HyphalNode {
                id: NodeId(1),
                kind: HyphalNodeKind::Anchor,
                position: Position3Mm::new(0.0, 0.0, 0.0),
            })
            .unwrap();
        graph
            .add_node(HyphalNode {
                id: NodeId(2),
                kind: HyphalNodeKind::ActiveTip,
                position: Position3Mm::new(1.0, 0.0, 0.0),
            })
            .unwrap();
        graph
            .add_segment(HyphalSegment {
                id: SegmentId(7),
                from: NodeId(1),
                to: NodeId(2),
                length_mm: 1.0,
                diameter_mm: Some(0.02),
                state: HyphalSegmentState::Active,
            })
            .unwrap();
        graph
    }

    #[test]
    fn sip_raman_requires_spectrum_subject() {
        let item = observation(RamanModality::SipSpontaneousRaman);
        let receipts = [receipt(
            10,
            ObservationSubject::RamanSpectrum,
            Some(EvidenceGraphTarget::Segment(SegmentId(7))),
        )];
        assert_eq!(item.validate_against(&graph(), &receipts), Ok(()));
    }

    #[test]
    fn srs_requires_chemical_map_subject() {
        let item = observation(RamanModality::StimulatedRamanScattering);
        let receipts = [receipt(
            10,
            ObservationSubject::RamanChemicalMap,
            Some(EvidenceGraphTarget::Segment(SegmentId(7))),
        )];
        assert_eq!(item.validate_against(&graph(), &receipts), Ok(()));
    }

    #[test]
    fn incident_energy_is_a_setting_proxy_not_absorbed_dose() {
        let item = observation(RamanModality::SpontaneousRaman);
        assert!((item.acquisition.nominal_incident_energy_mj() - 0.6).abs() < 1e-12);
        assert_eq!(item.acquisition.illuminated_spot_area_um2, None);
        assert!(!item.has_calibration_reference());
    }

    #[test]
    fn repeated_exposure_history_is_preserved() {
        let mut item = observation(RamanModality::SpontaneousRaman);
        item.prior_exposures_same_roi = 8;
        assert_eq!(item.prior_exposures_same_roi, 8);
    }

    #[test]
    fn wrong_modality_subject_is_rejected() {
        let item = observation(RamanModality::SpontaneousRaman);
        let receipts = [receipt(
            10,
            ObservationSubject::RamanChemicalMap,
            Some(EvidenceGraphTarget::Segment(SegmentId(7))),
        )];
        assert!(matches!(
            item.validate_against(&graph(), &receipts),
            Err(RamanEvidenceError::WrongSourceSubject { .. })
        ));
    }

    #[test]
    fn duplicate_and_missing_source_receipts_are_rejected() {
        let item = observation(RamanModality::SpontaneousRaman);
        assert_eq!(
            item.validate_against(&graph(), &[]),
            Err(RamanEvidenceError::MissingSourceReceipt(
                ObservationEvidenceId(10)
            ))
        );
        let source = receipt(
            10,
            ObservationSubject::RamanSpectrum,
            Some(EvidenceGraphTarget::Segment(SegmentId(7))),
        );
        assert!(matches!(
            item.validate_against(&graph(), &[source.clone(), source]),
            Err(RamanEvidenceError::DuplicateSourceReceipt(
                ObservationEvidenceId(10)
            ))
        ));
    }

    #[test]
    fn invalid_acquisition_spectrum_and_stressor_are_rejected() {
        let mut item = observation(RamanModality::SpontaneousRaman);
        item.acquisition.laser_power_mw = 0.0;
        assert_eq!(item.validate(), Err(RamanEvidenceError::InvalidAcquisition));

        item = observation(RamanModality::SpontaneousRaman);
        item.bands[0].wavenumber_cm_inverse = f64::NAN;
        assert_eq!(
            item.validate(),
            Err(RamanEvidenceError::InvalidSpectralBand)
        );

        item = observation(RamanModality::SpontaneousRaman);
        item.stressor = SpectralStressor::Copper {
            concentration_umol_per_litre: -1.0,
        };
        assert_eq!(item.validate(), Err(RamanEvidenceError::InvalidStressor));
    }

    #[test]
    fn wavenumbers_must_be_strictly_increasing() {
        let mut item = observation(RamanModality::SpontaneousRaman);
        item.bands[1].wavenumber_cm_inverse = item.bands[0].wavenumber_cm_inverse;
        assert_eq!(
            item.validate(),
            Err(RamanEvidenceError::NonIncreasingWavenumber)
        );
    }

    #[test]
    fn independent_calibration_receipt_is_required_when_reference_is_named() {
        let mut item = observation(RamanModality::SpontaneousRaman);
        item.calibration_reference = Some(ObservationEvidenceId(20));
        let source = receipt(
            10,
            ObservationSubject::RamanSpectrum,
            Some(EvidenceGraphTarget::Segment(SegmentId(7))),
        );
        let wrong_calibration = receipt(20, ObservationSubject::FluorescenceRoi, None);
        assert_eq!(
            item.validate_against(&graph(), std::slice::from_ref(&source)),
            Err(RamanEvidenceError::MissingCalibrationReceipt(
                ObservationEvidenceId(20)
            ))
        );
        assert_eq!(
            item.validate_against(&graph(), &[source, wrong_calibration]),
            Err(RamanEvidenceError::WrongCalibrationSubject(
                ObservationEvidenceId(20)
            ))
        );
    }

    #[test]
    fn segment_identity_and_receipt_target_are_both_checked() {
        let mut item = observation(RamanModality::SpontaneousRaman);
        item.segment = Some(SegmentId(99));
        let missing = receipt(10, ObservationSubject::RamanSpectrum, None);
        assert_eq!(
            item.validate_against(&graph(), &[missing]),
            Err(RamanEvidenceError::MissingSegment(SegmentId(99)))
        );
    }
}
