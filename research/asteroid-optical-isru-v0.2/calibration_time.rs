// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Standalone temporal calibration-eligibility reference.
// Run tests with:
//   rustc --edition=2021 --test research/asteroid-optical-isru-v0.2/calibration_time.rs -o /tmp/isru-calibration-tests
//   /tmp/isru-calibration-tests

#![forbid(unsafe_code)]

//! Bitemporal calibration-status assessment for scientific measurements.
//!
//! This module answers two different questions without rewriting observations:
//! (1) was a calibration declared valid at the measurement time? and
//! (2) what did the evidence system know about revocation/supersession at a
//! separately specified knowledge time?
//!
//! Event effective time controls the historical measurement assessment.
//! Event recorded time controls when that information became available.
//! This is a provenance/temporal kernel only: it does not authenticate issuers,
//! verify signatures, prove traceability, or establish physical calibration.

/// A later-recorded event may have an earlier effective time. This is
/// intentional: a retrospective revocation can invalidate earlier evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationEvent {
    Revoked {
        effective_at_utc_s: i64,
        recorded_at_utc_s: i64,
    },
    Superseded {
        effective_at_utc_s: i64,
        recorded_at_utc_s: i64,
    },
}

/// Declared calibration scope and validity interval.
///
/// The interval is half-open: valid_from <= measurement_time < valid_until.
/// Identity strings are opaque references, not proof of issuer authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibrationRecord {
    pub id: String,
    pub instrument_id: String,
    pub channel_id: String,
    pub valid_from_utc_s: i64,
    pub valid_until_utc_s: i64,
    pub events: Vec<CalibrationEvent>,
}

/// A measurement's declared instrument, channel, timestamp and calibration link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasurementReference {
    pub id: String,
    pub instrument_id: String,
    pub channel_id: String,
    pub measured_at_utc_s: i64,
    pub calibration_id: Option<String>,
}

/// Temporal result only. WithinDeclaredValidity is not a physical calibration
/// verdict and is not an external-authority verification result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationTemporalStatus {
    WithinDeclaredValidity,
    NotYetValid,
    Expired,
    Revoked {
        effective_at_utc_s: i64,
        recorded_at_utc_s: i64,
    },
    Superseded {
        effective_at_utc_s: i64,
        recorded_at_utc_s: i64,
    },
    MissingCalibrationReference,
    CalibrationNotFound,
    AmbiguousCalibrationIdentity,
    InstrumentMismatch,
    ChannelMismatch,
}

/// Structural failures are errors; missing or disqualifying evidence is
/// returned as a status so the original observation can remain in the record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemporalModelError {
    EmptyMeasurementId,
    EmptyMeasurementInstrumentId,
    EmptyMeasurementChannelId,
    EmptyCalibrationId,
    EmptyCalibrationInstrumentId,
    EmptyCalibrationChannelId,
    InvalidCalibrationValidityInterval,
}

impl std::fmt::Display for TemporalModelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TemporalModelError {}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_measurement(measurement: &MeasurementReference) -> Result<(), TemporalModelError> {
    if !nonempty(&measurement.id) {
        return Err(TemporalModelError::EmptyMeasurementId);
    }
    if !nonempty(&measurement.instrument_id) {
        return Err(TemporalModelError::EmptyMeasurementInstrumentId);
    }
    if !nonempty(&measurement.channel_id) {
        return Err(TemporalModelError::EmptyMeasurementChannelId);
    }
    Ok(())
}

fn validate_calibration(calibration: &CalibrationRecord) -> Result<(), TemporalModelError> {
    if !nonempty(&calibration.id) {
        return Err(TemporalModelError::EmptyCalibrationId);
    }
    if !nonempty(&calibration.instrument_id) {
        return Err(TemporalModelError::EmptyCalibrationInstrumentId);
    }
    if !nonempty(&calibration.channel_id) {
        return Err(TemporalModelError::EmptyCalibrationChannelId);
    }
    if calibration.valid_from_utc_s >= calibration.valid_until_utc_s {
        return Err(TemporalModelError::InvalidCalibrationValidityInterval);
    }
    Ok(())
}

/// Assess calibration declarations for one measurement.
///
/// knowledge_time_utc_s makes the query bitemporal:
/// - a revocation recorded after knowledge time is not yet visible to that
///   historical knowledge snapshot;
/// - once recorded, a revocation whose effective time is at or before the
///   measurement time is applied retrospectively.
///
/// Revocation takes precedence over supersession if both known events apply.
/// Within each event class, the earliest effective invalidation is returned.
/// Calibration validity is half-open: the instant at valid_until is expired.
pub fn assess_calibration_at(
    measurement: &MeasurementReference,
    calibrations: &[CalibrationRecord],
    knowledge_time_utc_s: i64,
) -> Result<CalibrationTemporalStatus, TemporalModelError> {
    validate_measurement(measurement)?;

    let Some(calibration_id) = measurement.calibration_id.as_deref() else {
        return Ok(CalibrationTemporalStatus::MissingCalibrationReference);
    };
    if !nonempty(calibration_id) {
        return Ok(CalibrationTemporalStatus::MissingCalibrationReference);
    }

    let mut matching = calibrations
        .iter()
        .filter(|calibration| calibration.id == calibration_id);
    let Some(calibration) = matching.next() else {
        return Ok(CalibrationTemporalStatus::CalibrationNotFound);
    };
    if matching.next().is_some() {
        return Ok(CalibrationTemporalStatus::AmbiguousCalibrationIdentity);
    }

    validate_calibration(calibration)?;

    if calibration.instrument_id != measurement.instrument_id {
        return Ok(CalibrationTemporalStatus::InstrumentMismatch);
    }
    if calibration.channel_id != measurement.channel_id {
        return Ok(CalibrationTemporalStatus::ChannelMismatch);
    }

    let applicable_revocation = calibration
        .events
        .iter()
        .filter_map(|event| match *event {
            CalibrationEvent::Revoked {
                effective_at_utc_s,
                recorded_at_utc_s,
            } if recorded_at_utc_s <= knowledge_time_utc_s
                && effective_at_utc_s <= measurement.measured_at_utc_s =>
            {
                Some((effective_at_utc_s, recorded_at_utc_s))
            }
            _ => None,
        })
        .min_by_key(|(effective_at, recorded_at)| (*effective_at, *recorded_at));

    if let Some((effective_at_utc_s, recorded_at_utc_s)) = applicable_revocation {
        return Ok(CalibrationTemporalStatus::Revoked {
            effective_at_utc_s,
            recorded_at_utc_s,
        });
    }

    let applicable_supersession = calibration
        .events
        .iter()
        .filter_map(|event| match *event {
            CalibrationEvent::Superseded {
                effective_at_utc_s,
                recorded_at_utc_s,
            } if recorded_at_utc_s <= knowledge_time_utc_s
                && effective_at_utc_s <= measurement.measured_at_utc_s =>
            {
                Some((effective_at_utc_s, recorded_at_utc_s))
            }
            _ => None,
        })
        .min_by_key(|(effective_at, recorded_at)| (*effective_at, *recorded_at));

    if let Some((effective_at_utc_s, recorded_at_utc_s)) = applicable_supersession {
        return Ok(CalibrationTemporalStatus::Superseded {
            effective_at_utc_s,
            recorded_at_utc_s,
        });
    }

    if measurement.measured_at_utc_s < calibration.valid_from_utc_s {
        return Ok(CalibrationTemporalStatus::NotYetValid);
    }
    if measurement.measured_at_utc_s >= calibration.valid_until_utc_s {
        return Ok(CalibrationTemporalStatus::Expired);
    }

    Ok(CalibrationTemporalStatus::WithinDeclaredValidity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calibration(events: Vec<CalibrationEvent>) -> CalibrationRecord {
        CalibrationRecord {
            id: "cal-17".to_string(),
            instrument_id: "spectrometer-2".to_string(),
            channel_id: "optical-reference".to_string(),
            valid_from_utc_s: 100,
            valid_until_utc_s: 300,
            events,
        }
    }

    fn measurement(time: i64) -> MeasurementReference {
        MeasurementReference {
            id: "measurement-42".to_string(),
            instrument_id: "spectrometer-2".to_string(),
            channel_id: "optical-reference".to_string(),
            measured_at_utc_s: time,
            calibration_id: Some("cal-17".to_string()),
        }
    }

    #[test]
    fn measurement_inside_declared_half_open_interval_is_temporally_valid() {
        let result = assess_calibration_at(&measurement(150), &[calibration(vec![])], 500);
        assert_eq!(
            result,
            Ok(CalibrationTemporalStatus::WithinDeclaredValidity)
        );
    }

    #[test]
    fn validity_start_is_inclusive_and_validity_end_is_exclusive() {
        let records = [calibration(vec![])];
        assert_eq!(
            assess_calibration_at(&measurement(100), &records, 500),
            Ok(CalibrationTemporalStatus::WithinDeclaredValidity)
        );
        assert_eq!(
            assess_calibration_at(&measurement(300), &records, 500),
            Ok(CalibrationTemporalStatus::Expired)
        );
    }

    #[test]
    fn measurement_before_validity_window_is_not_yet_valid() {
        assert_eq!(
            assess_calibration_at(&measurement(99), &[calibration(vec![])], 500),
            Ok(CalibrationTemporalStatus::NotYetValid)
        );
    }

    #[test]
    fn measurement_after_validity_window_is_expired() {
        assert_eq!(
            assess_calibration_at(&measurement(301), &[calibration(vec![])], 500),
            Ok(CalibrationTemporalStatus::Expired)
        );
    }

    #[test]
    fn retrospective_revocation_depends_on_knowledge_time_and_effective_time() {
        let record = calibration(vec![CalibrationEvent::Revoked {
            effective_at_utc_s: 140,
            recorded_at_utc_s: 220,
        }]);

        assert_eq!(
            assess_calibration_at(&measurement(150), &[record.clone()], 219),
            Ok(CalibrationTemporalStatus::WithinDeclaredValidity)
        );
        assert_eq!(
            assess_calibration_at(&measurement(150), &[record], 220),
            Ok(CalibrationTemporalStatus::Revoked {
                effective_at_utc_s: 140,
                recorded_at_utc_s: 220,
            })
        );
    }

    #[test]
    fn future_effective_revocation_does_not_invalidate_an_earlier_measurement() {
        let record = calibration(vec![CalibrationEvent::Revoked {
            effective_at_utc_s: 180,
            recorded_at_utc_s: 200,
        }]);
        assert_eq!(
            assess_calibration_at(&measurement(150), &[record], 250),
            Ok(CalibrationTemporalStatus::WithinDeclaredValidity)
        );
    }

    #[test]
    fn applicable_supersession_is_distinct_from_revocation() {
        let record = calibration(vec![CalibrationEvent::Superseded {
            effective_at_utc_s: 140,
            recorded_at_utc_s: 160,
        }]);
        assert_eq!(
            assess_calibration_at(&measurement(150), &[record], 200),
            Ok(CalibrationTemporalStatus::Superseded {
                effective_at_utc_s: 140,
                recorded_at_utc_s: 160,
            })
        );
    }

    #[test]
    fn instrument_and_channel_mismatches_are_not_accepted() {
        let mut wrong_instrument = measurement(150);
        wrong_instrument.instrument_id = "different-instrument".to_string();
        assert_eq!(
            assess_calibration_at(&wrong_instrument, &[calibration(vec![])], 500),
            Ok(CalibrationTemporalStatus::InstrumentMismatch)
        );

        let mut wrong_channel = measurement(150);
        wrong_channel.channel_id = "sample-channel".to_string();
        assert_eq!(
            assess_calibration_at(&wrong_channel, &[calibration(vec![])], 500),
            Ok(CalibrationTemporalStatus::ChannelMismatch)
        );
    }

    #[test]
    fn missing_unresolved_and_ambiguous_calibration_links_remain_explicit() {
        let mut missing = measurement(150);
        missing.calibration_id = None;
        assert_eq!(
            assess_calibration_at(&missing, &[], 500),
            Ok(CalibrationTemporalStatus::MissingCalibrationReference)
        );
        assert_eq!(
            assess_calibration_at(&measurement(150), &[], 500),
            Ok(CalibrationTemporalStatus::CalibrationNotFound)
        );

        let duplicate = calibration(vec![]);
        assert_eq!(
            assess_calibration_at(&measurement(150), &[duplicate.clone(), duplicate], 500),
            Ok(CalibrationTemporalStatus::AmbiguousCalibrationIdentity)
        );
    }

    #[test]
    fn invalid_calibration_interval_is_a_structural_error() {
        let mut invalid = calibration(vec![]);
        invalid.valid_from_utc_s = 300;
        invalid.valid_until_utc_s = 300;
        assert_eq!(
            assess_calibration_at(&measurement(301), &[invalid], 500),
            Err(TemporalModelError::InvalidCalibrationValidityInterval)
        );
    }

    #[test]
    fn event_input_order_does_not_change_revocation_assessment() {
        let revoked = CalibrationEvent::Revoked {
            effective_at_utc_s: 140,
            recorded_at_utc_s: 220,
        };
        let superseded = CalibrationEvent::Superseded {
            effective_at_utc_s: 130,
            recorded_at_utc_s: 180,
        };
        let forward = calibration(vec![revoked, superseded]);
        let reverse = calibration(vec![superseded, revoked]);

        let a = assess_calibration_at(&measurement(150), &[forward], 250);
        let b = assess_calibration_at(&measurement(150), &[reverse], 250);
        assert_eq!(a, b);
        assert!(matches!(a, Ok(CalibrationTemporalStatus::Revoked { .. })));
    }
}
