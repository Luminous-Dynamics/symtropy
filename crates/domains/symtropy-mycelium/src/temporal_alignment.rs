// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-gated alignment of observations from independent acquisition clocks.
//!
//! Graph images, flow cameras, Raman/SRS instruments, isotope probes and
//! electrical sensors may use different clocks. Their raw timestamps are not
//! comparable until an admitted synchronization observation establishes a
//! transform into one reference clock.
//!
//! Uncertainty is represented as a conservative absolute error bound. The
//! affine transform propagates source timestamp, anchor and rate bounds by
//! linear worst-case addition; it does not assume independent Gaussian errors.
//! Temporal ordering is reported only when the bounded intervals are disjoint.

use std::collections::BTreeMap;
use std::fmt;

use crate::observation_evidence::{
    ObservationEvidenceId, ObservationEvidenceReceipt, ObservationSubject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExperimentClockId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClockCalibrationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawExperimentTimestamp {
    pub clock: ExperimentClockId,
    /// Raw time in seconds in this clock's own epoch.
    pub seconds: f64,
    /// Conservative absolute timestamp error bound in seconds.
    pub error_bound_s: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockCalibration {
    pub id: ClockCalibrationId,
    pub source_clock: ExperimentClockId,
    pub reference_clock: ExperimentClockId,
    /// Inclusive validity interval in the source clock.
    pub source_valid_from_s: f64,
    pub source_valid_to_s: f64,
    /// Reference-clock time corresponding to source_valid_from_s.
    pub reference_at_valid_from_s: f64,
    /// Positive source-to-reference clock rate ratio.
    pub rate: f64,
    /// Error bound on reference_at_valid_from_s, in seconds.
    pub anchor_error_bound_s: f64,
    /// Absolute error bound on rate, in reference seconds per source second.
    pub rate_error_bound: f64,
    /// Receipt for the synchronization pulse/trigger used to fit this transform.
    pub evidence: ObservationEvidenceId,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignedExperimentTimestamp {
    pub source_clock: ExperimentClockId,
    pub reference_clock: ExperimentClockId,
    pub reference_seconds: f64,
    pub error_bound_s: f64,
    pub calibration: Option<ClockCalibrationId>,
    pub calibration_evidence: Option<ObservationEvidenceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalOrdering {
    FirstBeforeSecond,
    SecondBeforeFirst,
    IndeterminateOverlap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalAlignmentError {
    InvalidRawTimestamp,
    InvalidCalibration,
    SameClockCalibration(ExperimentClockId),
    DuplicateCalibrationId(ClockCalibrationId),
    DuplicateSourceClock(ExperimentClockId),
    MissingCalibrationReceipt(ObservationEvidenceId),
    DuplicateCalibrationReceipt(ObservationEvidenceId),
    InvalidCalibrationReceipt {
        id: ObservationEvidenceId,
        reason: String,
    },
    WrongCalibrationReceiptSubject(ObservationEvidenceId),
    MissingClockCalibration(ExperimentClockId),
    TimestampOutsideCalibration {
        clock: ExperimentClockId,
        seconds_bits: u64,
    },
    NonFiniteAlignedTimestamp,
}

impl fmt::Display for TemporalAlignmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRawTimestamp => {
                write!(
                    f,
                    "raw timestamp and error bound must be finite and non-negative"
                )
            }
            Self::InvalidCalibration => {
                write!(
                    f,
                    "clock calibration bounds, rate and uncertainties are invalid"
                )
            }
            Self::SameClockCalibration(clock) => {
                write!(f, "clock {:?} cannot be calibrated to itself", clock)
            }
            Self::DuplicateCalibrationId(id) => {
                write!(f, "duplicate clock calibration identity {:?}", id)
            }
            Self::DuplicateSourceClock(clock) => {
                write!(
                    f,
                    "source clock {:?} already has a direct calibration",
                    clock
                )
            }
            Self::MissingCalibrationReceipt(id) => {
                write!(f, "missing clock synchronization receipt {:?}", id)
            }
            Self::DuplicateCalibrationReceipt(id) => {
                write!(f, "duplicate clock synchronization receipt {:?}", id)
            }
            Self::InvalidCalibrationReceipt { id, reason } => {
                write!(
                    f,
                    "invalid clock synchronization receipt {:?}: {}",
                    id, reason
                )
            }
            Self::WrongCalibrationReceiptSubject(id) => {
                write!(
                    f,
                    "receipt {:?} is not a clock synchronization observation",
                    id
                )
            }
            Self::MissingClockCalibration(clock) => {
                write!(f, "no admitted calibration for source clock {:?}", clock)
            }
            Self::TimestampOutsideCalibration {
                clock,
                seconds_bits,
            } => {
                write!(
                    f,
                    "timestamp {} on clock {:?} is outside calibration validity",
                    f64::from_bits(*seconds_bits),
                    clock
                )
            }
            Self::NonFiniteAlignedTimestamp => {
                write!(
                    f,
                    "aligned timestamp or propagated error bound is not finite"
                )
            }
        }
    }
}

impl std::error::Error for TemporalAlignmentError {}

impl RawExperimentTimestamp {
    pub fn validate(self) -> Result<(), TemporalAlignmentError> {
        if !self.seconds.is_finite()
            || self.seconds < 0.0
            || !self.error_bound_s.is_finite()
            || self.error_bound_s < 0.0
        {
            return Err(TemporalAlignmentError::InvalidRawTimestamp);
        }
        Ok(())
    }
}

impl ClockCalibration {
    pub fn validate(&self) -> Result<(), TemporalAlignmentError> {
        if self.source_clock == self.reference_clock {
            return Err(TemporalAlignmentError::SameClockCalibration(
                self.source_clock,
            ));
        }
        if !self.source_valid_from_s.is_finite()
            || !self.source_valid_to_s.is_finite()
            || self.source_valid_from_s < 0.0
            || self.source_valid_to_s < self.source_valid_from_s
            || !self.reference_at_valid_from_s.is_finite()
            || self.reference_at_valid_from_s < 0.0
            || !self.rate.is_finite()
            || self.rate <= 0.0
            || !self.anchor_error_bound_s.is_finite()
            || self.anchor_error_bound_s < 0.0
            || !self.rate_error_bound.is_finite()
            || self.rate_error_bound < 0.0
            || self.rate_error_bound >= self.rate
        {
            return Err(TemporalAlignmentError::InvalidCalibration);
        }
        Ok(())
    }

    fn align(
        &self,
        timestamp: RawExperimentTimestamp,
    ) -> Result<AlignedExperimentTimestamp, TemporalAlignmentError> {
        self.validate()?;
        timestamp.validate()?;
        if timestamp.seconds < self.source_valid_from_s
            || timestamp.seconds > self.source_valid_to_s
        {
            return Err(TemporalAlignmentError::TimestampOutsideCalibration {
                clock: timestamp.clock,
                seconds_bits: timestamp.seconds.to_bits(),
            });
        }

        let elapsed = timestamp.seconds - self.source_valid_from_s;
        let reference_seconds = self.reference_at_valid_from_s + elapsed * self.rate;
        // Linear worst-case propagation; no Gaussian/independence assumption.
        let error_bound_s = self.rate * timestamp.error_bound_s
            + self.anchor_error_bound_s
            + elapsed * self.rate_error_bound;

        if !reference_seconds.is_finite() || !error_bound_s.is_finite() {
            return Err(TemporalAlignmentError::NonFiniteAlignedTimestamp);
        }

        Ok(AlignedExperimentTimestamp {
            source_clock: timestamp.clock,
            reference_clock: self.reference_clock,
            reference_seconds,
            error_bound_s,
            calibration: Some(self.id),
            calibration_evidence: Some(self.evidence),
        })
    }
}

/// Aligns raw clocks directly to one experiment reference clock.
///
/// This first version intentionally does not compose calibration chains. Each
/// non-reference clock needs one direct, source-receipted calibration to the
/// reference clock; this avoids silently compounding untracked transforms.
#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentTimeAligner {
    reference_clock: ExperimentClockId,
    calibrations: BTreeMap<ExperimentClockId, ClockCalibration>,
    calibration_ids: BTreeMap<ClockCalibrationId, ExperimentClockId>,
}

impl ExperimentTimeAligner {
    pub fn new(reference_clock: ExperimentClockId) -> Self {
        Self {
            reference_clock,
            calibrations: BTreeMap::new(),
            calibration_ids: BTreeMap::new(),
        }
    }

    pub const fn reference_clock(&self) -> ExperimentClockId {
        self.reference_clock
    }

    pub fn add_calibration(
        &mut self,
        calibration: ClockCalibration,
        receipts: &[ObservationEvidenceReceipt],
    ) -> Result<(), TemporalAlignmentError> {
        calibration.validate()?;
        if calibration.reference_clock != self.reference_clock {
            return Err(TemporalAlignmentError::InvalidCalibration);
        }
        if self.calibration_ids.contains_key(&calibration.id) {
            return Err(TemporalAlignmentError::DuplicateCalibrationId(
                calibration.id,
            ));
        }
        if self.calibrations.contains_key(&calibration.source_clock) {
            return Err(TemporalAlignmentError::DuplicateSourceClock(
                calibration.source_clock,
            ));
        }

        let mut matching = receipts
            .iter()
            .filter(|receipt| receipt.id == calibration.evidence);
        let receipt = matching
            .next()
            .ok_or(TemporalAlignmentError::MissingCalibrationReceipt(
                calibration.evidence,
            ))?;
        if matching.next().is_some() {
            return Err(TemporalAlignmentError::DuplicateCalibrationReceipt(
                calibration.evidence,
            ));
        }
        receipt
            .validate()
            .map_err(|error| TemporalAlignmentError::InvalidCalibrationReceipt {
                id: calibration.evidence,
                reason: error.to_string(),
            })?;
        if receipt.subject != ObservationSubject::ClockSynchronization {
            return Err(TemporalAlignmentError::WrongCalibrationReceiptSubject(
                calibration.evidence,
            ));
        }

        self.calibration_ids
            .insert(calibration.id, calibration.source_clock);
        self.calibrations
            .insert(calibration.source_clock, calibration);
        Ok(())
    }

    pub fn align(
        &self,
        timestamp: RawExperimentTimestamp,
    ) -> Result<AlignedExperimentTimestamp, TemporalAlignmentError> {
        timestamp.validate()?;
        if timestamp.clock == self.reference_clock {
            return Ok(AlignedExperimentTimestamp {
                source_clock: timestamp.clock,
                reference_clock: self.reference_clock,
                reference_seconds: timestamp.seconds,
                error_bound_s: timestamp.error_bound_s,
                calibration: None,
                calibration_evidence: None,
            });
        }

        let calibration = self.calibrations.get(&timestamp.clock).ok_or(
            TemporalAlignmentError::MissingClockCalibration(timestamp.clock),
        )?;
        calibration.align(timestamp)
    }

    pub fn calibration_for(&self, source_clock: ExperimentClockId) -> Option<&ClockCalibration> {
        self.calibrations.get(&source_clock)
    }

    /// Determine order only when the two conservative time intervals are disjoint.
    /// Touching intervals count as overlap because their order is not established.
    pub fn compare(
        first: AlignedExperimentTimestamp,
        second: AlignedExperimentTimestamp,
    ) -> Result<TemporalOrdering, TemporalAlignmentError> {
        if first.reference_clock != second.reference_clock
            || !first.reference_seconds.is_finite()
            || !second.reference_seconds.is_finite()
            || !first.error_bound_s.is_finite()
            || !second.error_bound_s.is_finite()
            || first.error_bound_s < 0.0
            || second.error_bound_s < 0.0
        {
            return Err(TemporalAlignmentError::NonFiniteAlignedTimestamp);
        }

        let first_lower = first.reference_seconds - first.error_bound_s;
        let first_upper = first.reference_seconds + first.error_bound_s;
        let second_lower = second.reference_seconds - second.error_bound_s;
        let second_upper = second.reference_seconds + second.error_bound_s;

        if first_upper < second_lower {
            Ok(TemporalOrdering::FirstBeforeSecond)
        } else if second_upper < first_lower {
            Ok(TemporalOrdering::SecondBeforeFirst)
        } else {
            Ok(TemporalOrdering::IndeterminateOverlap)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation_evidence::{
        ImageFrameIndex, ImageFrameReference, ImageRoiBoundsPx, ObservationRoiId,
        PhysicalSamplingCalibration, Sha256Digest,
    };

    fn receipt(id: u64, subject: ObservationSubject) -> ObservationEvidenceReceipt {
        ObservationEvidenceReceipt {
            id: ObservationEvidenceId(id),
            source_file_sha256: Sha256Digest([1; 32]),
            frame: ImageFrameReference {
                frame: ImageFrameIndex(1),
                slice: None,
                timestamp_s: 1.0,
            },
            roi: ImageRoiBoundsPx {
                id: ObservationRoiId(1),
                x_min: 0,
                y_min: 0,
                width_px: 10,
                height_px: 10,
            },
            calibration: PhysicalSamplingCalibration {
                pixel_width_um: 1.0,
                pixel_height_um: 1.0,
                voxel_depth_um: None,
                frame_interval_s: Some(1.0),
            },
            processor_name: "clock_trigger_parser".to_owned(),
            processor_revision: "v1".to_owned(),
            processor_parameters_sha256: Sha256Digest([2; 32]),
            subject,
            graph_target: None,
            graph_event: None,
            uncertainty: None,
            disposition: crate::observation_evidence::EvidenceDisposition::MachineGenerated,
        }
    }

    fn calibration(evidence: u64, source_clock: u64) -> ClockCalibration {
        ClockCalibration {
            id: ClockCalibrationId(source_clock + 100),
            source_clock: ExperimentClockId(source_clock),
            reference_clock: ExperimentClockId(1),
            source_valid_from_s: 0.0,
            source_valid_to_s: 100.0,
            reference_at_valid_from_s: 10.0,
            rate: 1.000_1,
            anchor_error_bound_s: 0.002,
            rate_error_bound: 0.000_001,
            evidence: ObservationEvidenceId(evidence),
        }
    }

    #[test]
    fn reference_clock_timestamp_aligns_without_calibration() {
        let aligner = ExperimentTimeAligner::new(ExperimentClockId(1));
        let aligned = aligner
            .align(RawExperimentTimestamp {
                clock: ExperimentClockId(1),
                seconds: 42.0,
                error_bound_s: 0.01,
            })
            .unwrap();
        assert_eq!(aligned.reference_seconds, 42.0);
        assert_eq!(aligned.error_bound_s, 0.01);
        assert_eq!(aligned.calibration, None);
        assert_eq!(aligned.calibration_evidence, None);
    }

    #[test]
    fn independent_clock_requires_a_receipted_calibration() {
        let mut aligner = ExperimentTimeAligner::new(ExperimentClockId(1));
        assert_eq!(
            aligner.align(RawExperimentTimestamp {
                clock: ExperimentClockId(2),
                seconds: 5.0,
                error_bound_s: 0.0,
            }),
            Err(TemporalAlignmentError::MissingClockCalibration(
                ExperimentClockId(2)
            ))
        );

        aligner
            .add_calibration(
                calibration(20, 2),
                &[receipt(20, ObservationSubject::ClockSynchronization)],
            )
            .unwrap();
        let aligned = aligner
            .align(RawExperimentTimestamp {
                clock: ExperimentClockId(2),
                seconds: 5.0,
                error_bound_s: 0.01,
            })
            .unwrap();
        assert!((aligned.reference_seconds - 15.0005).abs() < 1e-12);
        assert!(aligned.error_bound_s > 0.012);
        assert_eq!(aligned.calibration, Some(ClockCalibrationId(102)));
        assert_eq!(
            aligned.calibration_evidence,
            Some(ObservationEvidenceId(20))
        );
    }

    #[test]
    fn wrong_receipt_subject_cannot_calibrate_a_clock() {
        let mut aligner = ExperimentTimeAligner::new(ExperimentClockId(1));
        assert_eq!(
            aligner.add_calibration(
                calibration(20, 2),
                &[receipt(20, ObservationSubject::RamanSpectrum)],
            ),
            Err(TemporalAlignmentError::WrongCalibrationReceiptSubject(
                ObservationEvidenceId(20)
            ))
        );
    }

    #[test]
    fn missing_and_duplicate_calibration_receipts_are_rejected() {
        let mut aligner = ExperimentTimeAligner::new(ExperimentClockId(1));
        assert_eq!(
            aligner.add_calibration(calibration(20, 2), &[]),
            Err(TemporalAlignmentError::MissingCalibrationReceipt(
                ObservationEvidenceId(20)
            ))
        );
        let item = receipt(20, ObservationSubject::ClockSynchronization);
        assert_eq!(
            aligner.add_calibration(calibration(20, 2), &[item.clone(), item]),
            Err(TemporalAlignmentError::DuplicateCalibrationReceipt(
                ObservationEvidenceId(20)
            ))
        );
    }

    #[test]
    fn duplicate_calibration_ids_and_source_clocks_are_rejected() {
        let mut aligner = ExperimentTimeAligner::new(ExperimentClockId(1));
        let first = calibration(20, 2);
        aligner
            .add_calibration(
                first,
                &[receipt(20, ObservationSubject::ClockSynchronization)],
            )
            .unwrap();

        let mut duplicate_id = calibration(21, 3);
        duplicate_id.id = first.id;
        assert_eq!(
            aligner.add_calibration(
                duplicate_id,
                &[receipt(21, ObservationSubject::ClockSynchronization)]
            ),
            Err(TemporalAlignmentError::DuplicateCalibrationId(first.id))
        );

        let mut duplicate_clock = calibration(22, 2);
        duplicate_clock.id = ClockCalibrationId(999);
        assert_eq!(
            aligner.add_calibration(
                duplicate_clock,
                &[receipt(22, ObservationSubject::ClockSynchronization)]
            ),
            Err(TemporalAlignmentError::DuplicateSourceClock(
                ExperimentClockId(2)
            ))
        );
    }

    #[test]
    fn timestamps_outside_calibration_validity_are_rejected() {
        let mut aligner = ExperimentTimeAligner::new(ExperimentClockId(1));
        aligner
            .add_calibration(
                calibration(20, 2),
                &[receipt(20, ObservationSubject::ClockSynchronization)],
            )
            .unwrap();
        assert!(matches!(
            aligner.align(RawExperimentTimestamp {
                clock: ExperimentClockId(2),
                seconds: 101.0,
                error_bound_s: 0.0,
            }),
            Err(TemporalAlignmentError::TimestampOutsideCalibration { .. })
        ));
    }

    #[test]
    fn invalid_clock_calibration_parameters_are_rejected() {
        let mut item = calibration(20, 2);
        item.rate = 0.0;
        assert_eq!(
            item.validate(),
            Err(TemporalAlignmentError::InvalidCalibration)
        );
        item = calibration(20, 2);
        item.rate_error_bound = item.rate;
        assert_eq!(
            item.validate(),
            Err(TemporalAlignmentError::InvalidCalibration)
        );
        item = calibration(20, 2);
        item.source_valid_to_s = f64::NAN;
        assert_eq!(
            item.validate(),
            Err(TemporalAlignmentError::InvalidCalibration)
        );
    }

    #[test]
    fn disjoint_time_intervals_establish_order() {
        let first = AlignedExperimentTimestamp {
            source_clock: ExperimentClockId(1),
            reference_clock: ExperimentClockId(1),
            reference_seconds: 10.0,
            error_bound_s: 0.2,
            calibration: None,
            calibration_evidence: None,
        };
        let second = AlignedExperimentTimestamp {
            source_clock: ExperimentClockId(2),
            reference_clock: ExperimentClockId(1),
            reference_seconds: 11.0,
            error_bound_s: 0.3,
            calibration: Some(ClockCalibrationId(102)),
            calibration_evidence: Some(ObservationEvidenceId(20)),
        };
        assert_eq!(
            ExperimentTimeAligner::compare(first, second),
            Ok(TemporalOrdering::FirstBeforeSecond)
        );
        assert_eq!(
            ExperimentTimeAligner::compare(second, first),
            Ok(TemporalOrdering::SecondBeforeFirst)
        );
    }

    #[test]
    fn overlapping_or_touching_time_intervals_are_indeterminate() {
        let make = |time, error| AlignedExperimentTimestamp {
            source_clock: ExperimentClockId(1),
            reference_clock: ExperimentClockId(1),
            reference_seconds: time,
            error_bound_s: error,
            calibration: None,
            calibration_evidence: None,
        };
        assert_eq!(
            ExperimentTimeAligner::compare(make(10.0, 0.5), make(10.5, 0.5)),
            Ok(TemporalOrdering::IndeterminateOverlap)
        );
        assert_eq!(
            ExperimentTimeAligner::compare(make(10.0, 0.5), make(11.0, 0.5)),
            Ok(TemporalOrdering::IndeterminateOverlap)
        );
    }

    #[test]
    fn timestamps_from_different_reference_clocks_cannot_be_compared() {
        let first = AlignedExperimentTimestamp {
            source_clock: ExperimentClockId(1),
            reference_clock: ExperimentClockId(1),
            reference_seconds: 1.0,
            error_bound_s: 0.0,
            calibration: None,
            calibration_evidence: None,
        };
        let second = AlignedExperimentTimestamp {
            source_clock: ExperimentClockId(2),
            reference_clock: ExperimentClockId(2),
            reference_seconds: 2.0,
            error_bound_s: 0.0,
            calibration: None,
            calibration_evidence: None,
        };
        assert_eq!(
            ExperimentTimeAligner::compare(first, second),
            Err(TemporalAlignmentError::NonFiniteAlignedTimestamp)
        );
    }

    #[test]
    fn raw_timestamps_reject_negative_or_non_finite_values() {
        let mut item = RawExperimentTimestamp {
            clock: ExperimentClockId(1),
            seconds: -0.1,
            error_bound_s: 0.0,
        };
        assert_eq!(
            item.validate(),
            Err(TemporalAlignmentError::InvalidRawTimestamp)
        );
        item.seconds = f64::NAN;
        assert_eq!(
            item.validate(),
            Err(TemporalAlignmentError::InvalidRawTimestamp)
        );
        item.seconds = 1.0;
        item.error_bound_s = -0.1;
        assert_eq!(
            item.validate(),
            Err(TemporalAlignmentError::InvalidRawTimestamp)
        );
    }
}
