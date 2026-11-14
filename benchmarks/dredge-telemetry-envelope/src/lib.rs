//! Profile-aware dredge telemetry observation validation.
//!
//! This crate does not implement DQM certification/protocols and never derives
//! dry sediment mass from raw telemetry. A pass means observation acceptance only.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlantProfileKind {
    Hopper,
    Mechanical,
    PipelineStandard,
    PipelineSmallBusiness,
    ScowMonitoring,
    ScowUllage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SensorKind {
    Draft,
    Displacement,
    SlurryDensity,
    SlurryVelocity,
    PumpState,
    BucketPosition,
    BucketState,
    BinUllage,
    BinVolume,
    HopperLoad,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorRule {
    pub kind: SensorKind,
    pub unit_id: String,
    pub minimum_milliunits: i64,
    pub maximum_milliunits: i64,
    pub maximum_age_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionRule {
    pub expected_crs: String,
    pub maximum_uncertainty_mm: u64,
    pub maximum_age_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DredgePlantProfile {
    pub profile_id: String,
    pub revision: String,
    pub kind: PlantProfileKind,
    pub required_sensors: Vec<SensorRule>,
    pub optional_sensors: Vec<SensorRule>,
    pub position_rule: Option<PositionRule>,
    pub valid_through_tick: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorReading {
    pub kind: SensorKind,
    pub sensor_id: String,
    pub value_milliunits: i64,
    pub unit_id: String,
    pub observed_at_tick: u64,
    pub calibrated: bool,
    pub calibration_valid_through_tick: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionObservation {
    pub longitude_microdegrees: i64,
    pub latitude_microdegrees: i64,
    pub crs: String,
    pub uncertainty_mm: u64,
    pub observed_at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryRecord {
    pub event_id: String,
    /// Caller-supplied digest binding; not calculated or cryptographically verified here.
    pub payload_digest: String,
    pub sequence: u64,
    pub plant_id: String,
    pub profile_id: String,
    pub profile_revision: String,
    pub profile_kind: PlantProfileKind,
    pub observed_at_tick: u64,
    pub received_at_tick: u64,
    pub position: Option<PositionObservation>,
    pub sensors: Vec<SensorReading>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TelemetryCursor {
    pub contiguous_sequence: u64,
    pub event_digests: BTreeMap<String, String>,
    pub event_records: BTreeMap<String, TelemetryRecord>,
    pub sequence_owners: BTreeMap<u64, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryDisposition {
    TelemetryAcceptedAsObservation,
    TelemetryAcceptedWithMissingOptionalFields,
    DuplicateExactReplay,
    UnknownReviewRequired,
    EvidenceInconsistent,
    UnsupportedPlantProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryReason {
    InvalidIdentity,
    InvalidDigestBinding,
    ProfileIdentityMismatch,
    ProfileExpired,
    ProfileCurrentnessUnknown,
    UnsupportedProfileShape,
    MissingRequiredSensor,
    MissingOptionalSensor,
    UnknownSensorKind,
    DuplicateSensorKind,
    InvalidSensorIdentity,
    SensorUnitMismatch,
    SensorValueOutOfRange,
    SensorObservationInFuture,
    SensorStale,
    SensorNotCalibrated,
    SensorCalibrationExpired,
    MissingPosition,
    PositionCrsMismatch,
    PositionUncertaintyExceeded,
    PositionCoordinateOutOfRange,
    PositionObservationInFuture,
    PositionStale,
    EventObservationInFuture,
    EventTooOld,
    DuplicateEventChangedPayload,
    SequenceCollision,
    OutOfOrderSequence,
    SequenceGap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryValidation {
    pub disposition: TelemetryDisposition,
    pub reasons: Vec<TelemetryReason>,
    /// Updated only for a new, identity-valid event. Exact replay is idempotent.
    pub next_cursor: TelemetryCursor,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn rule_map(rules: &[SensorRule]) -> BTreeMap<SensorKind, &SensorRule> {
    rules.iter().map(|rule| (rule.kind, rule)).collect()
}

fn profile_shape_valid(profile: &DredgePlantProfile) -> bool {
    if !nonempty(&profile.profile_id) || !nonempty(&profile.revision) {
        return false;
    }
    let required = rule_map(&profile.required_sensors);
    let optional = rule_map(&profile.optional_sensors);
    if required.len() != profile.required_sensors.len()
        || optional.len() != profile.optional_sensors.len()
    {
        return false;
    }
    if required.keys().any(|kind| optional.contains_key(kind)) {
        return false;
    }
    profile
        .required_sensors
        .iter()
        .chain(&profile.optional_sensors)
        .all(|rule| nonempty(&rule.unit_id) && rule.minimum_milliunits <= rule.maximum_milliunits)
        && profile
            .position_rule
            .as_ref()
            .is_some_and(|rule| nonempty(&rule.expected_crs))
}

fn validate_sensor(
    reading: &SensorReading,
    rule: &SensorRule,
    evaluation_tick: u64,
    reasons: &mut Vec<TelemetryReason>,
) -> (bool, bool) {
    let mut inconsistent = false;
    let mut unknown = false;
    if !nonempty(&reading.sensor_id) {
        reasons.push(TelemetryReason::InvalidSensorIdentity);
        inconsistent = true;
    }
    if reading.unit_id != rule.unit_id {
        reasons.push(TelemetryReason::SensorUnitMismatch);
        inconsistent = true;
    }
    if reading.value_milliunits < rule.minimum_milliunits
        || reading.value_milliunits > rule.maximum_milliunits
    {
        reasons.push(TelemetryReason::SensorValueOutOfRange);
        inconsistent = true;
    }
    if reading.observed_at_tick > evaluation_tick {
        reasons.push(TelemetryReason::SensorObservationInFuture);
        inconsistent = true;
    } else if evaluation_tick - reading.observed_at_tick > rule.maximum_age_ticks {
        reasons.push(TelemetryReason::SensorStale);
        unknown = true;
    }
    if !reading.calibrated {
        reasons.push(TelemetryReason::SensorNotCalibrated);
        unknown = true;
    }
    match reading.calibration_valid_through_tick {
        None => {
            reasons.push(TelemetryReason::SensorCalibrationExpired);
            unknown = true;
        }
        Some(expiry) if expiry < evaluation_tick => {
            reasons.push(TelemetryReason::SensorCalibrationExpired);
            unknown = true;
        }
        Some(_) => {}
    }
    (inconsistent, unknown)
}

fn validate_position(
    position: Option<&PositionObservation>,
    rule: &PositionRule,
    evaluation_tick: u64,
    reasons: &mut Vec<TelemetryReason>,
) -> (bool, bool) {
    let Some(position) = position else {
        reasons.push(TelemetryReason::MissingPosition);
        return (false, true);
    };
    let mut inconsistent = false;
    let mut unknown = false;
    if position.crs != rule.expected_crs || !nonempty(&position.crs) {
        reasons.push(TelemetryReason::PositionCrsMismatch);
        inconsistent = true;
    }
    if !(-180_000_000..=180_000_000).contains(&position.longitude_microdegrees)
        || !(-90_000_000..=90_000_000).contains(&position.latitude_microdegrees)
    {
        reasons.push(TelemetryReason::PositionCoordinateOutOfRange);
        inconsistent = true;
    }
    if position.uncertainty_mm > rule.maximum_uncertainty_mm {
        reasons.push(TelemetryReason::PositionUncertaintyExceeded);
        unknown = true;
    }
    if position.observed_at_tick > evaluation_tick {
        reasons.push(TelemetryReason::PositionObservationInFuture);
        inconsistent = true;
    } else if evaluation_tick - position.observed_at_tick > rule.maximum_age_ticks {
        reasons.push(TelemetryReason::PositionStale);
        unknown = true;
    }
    (inconsistent, unknown)
}

/// Validate one telemetry record against its exact external plant-profile contract.
///
/// The function does not perform network authentication, payload hashing, DQM
/// protocol handling, dry-mass conversion, or ledger transfer creation.
#[must_use]
pub fn validate_telemetry(
    profile: &DredgePlantProfile,
    record: &TelemetryRecord,
    cursor: &TelemetryCursor,
    evaluation_tick: u64,
    maximum_event_age_ticks: u64,
) -> TelemetryValidation {
    let mut reasons = Vec::new();
    let mut next_cursor = cursor.clone();
    let mut inconsistent = false;
    let mut unknown = false;
    let mut unsupported = false;
    let mut missing_optional = false;

    if !profile_shape_valid(profile) {
        reasons.push(TelemetryReason::UnsupportedProfileShape);
        unsupported = true;
    }
    if record.profile_kind != profile.kind
        || record.profile_id != profile.profile_id
        || record.profile_revision != profile.revision
    {
        reasons.push(TelemetryReason::ProfileIdentityMismatch);
        unsupported = true;
    }
    match profile.valid_through_tick {
        None => {
            reasons.push(TelemetryReason::ProfileCurrentnessUnknown);
            unknown = true;
        }
        Some(expiry) if expiry < evaluation_tick => {
            reasons.push(TelemetryReason::ProfileExpired);
            unknown = true;
        }
        Some(_) => {}
    }
    if !nonempty(&record.event_id) || !nonempty(&record.plant_id) {
        reasons.push(TelemetryReason::InvalidIdentity);
        inconsistent = true;
    }
    if !nonempty(&record.payload_digest) {
        reasons.push(TelemetryReason::InvalidDigestBinding);
        inconsistent = true;
    }
    if record.sequence == 0 {
        reasons.push(TelemetryReason::OutOfOrderSequence);
        inconsistent = true;
    }
    if record.observed_at_tick > record.received_at_tick
        || record.observed_at_tick > evaluation_tick
        || record.received_at_tick > evaluation_tick
    {
        reasons.push(TelemetryReason::EventObservationInFuture);
        inconsistent = true;
    } else if evaluation_tick - record.observed_at_tick > maximum_event_age_ticks {
        reasons.push(TelemetryReason::EventTooOld);
        unknown = true;
    }

    if let Some(existing_digest) = cursor.event_digests.get(&record.event_id) {
        let same_record = cursor.event_records.get(&record.event_id) == Some(record);
        if existing_digest == &record.payload_digest && same_record {
            return TelemetryValidation {
                disposition: TelemetryDisposition::DuplicateExactReplay,
                reasons: Vec::new(),
                next_cursor,
            };
        }
        reasons.push(TelemetryReason::DuplicateEventChangedPayload);
        inconsistent = true;
    }
    if let Some(existing_event) = cursor.sequence_owners.get(&record.sequence)
        && existing_event != &record.event_id
    {
        reasons.push(TelemetryReason::SequenceCollision);
        inconsistent = true;
    }
    if record.sequence <= cursor.contiguous_sequence
        && !cursor.sequence_owners.contains_key(&record.sequence)
    {
        reasons.push(TelemetryReason::OutOfOrderSequence);
        inconsistent = true;
    }

    let required = rule_map(&profile.required_sensors);
    let optional = rule_map(&profile.optional_sensors);
    let mut present = BTreeSet::new();

    for reading in &record.sensors {
        if !present.insert(reading.kind) {
            reasons.push(TelemetryReason::DuplicateSensorKind);
            inconsistent = true;
            continue;
        }
        let rule = required
            .get(&reading.kind)
            .or_else(|| optional.get(&reading.kind));
        let Some(rule) = rule else {
            reasons.push(TelemetryReason::UnknownSensorKind);
            unknown = true;
            continue;
        };
        let (bad, review) = validate_sensor(reading, rule, evaluation_tick, &mut reasons);
        inconsistent |= bad;
        unknown |= review;
    }

    for kind in required.keys() {
        if !present.contains(kind) {
            reasons.push(TelemetryReason::MissingRequiredSensor);
            unknown = true;
        }
    }
    for kind in optional.keys() {
        if !present.contains(kind) {
            reasons.push(TelemetryReason::MissingOptionalSensor);
            missing_optional = true;
        }
    }

    if let Some(rule) = &profile.position_rule {
        let (bad, review) = validate_position(
            record.position.as_ref(),
            rule,
            evaluation_tick,
            &mut reasons,
        );
        inconsistent |= bad;
        unknown |= review;
    }

    let is_gap = record.sequence > cursor.contiguous_sequence.saturating_add(1);
    if is_gap {
        reasons.push(TelemetryReason::SequenceGap);
        unknown = true;
    }

    if !inconsistent && !unsupported {
        next_cursor
            .event_digests
            .insert(record.event_id.clone(), record.payload_digest.clone());
        next_cursor
            .event_records
            .insert(record.event_id.clone(), record.clone());
        next_cursor
            .sequence_owners
            .insert(record.sequence, record.event_id.clone());
        while next_cursor
            .sequence_owners
            .contains_key(&next_cursor.contiguous_sequence.saturating_add(1))
        {
            next_cursor.contiguous_sequence = next_cursor.contiguous_sequence.saturating_add(1);
        }
    }

    let disposition = if inconsistent {
        TelemetryDisposition::EvidenceInconsistent
    } else if unsupported {
        TelemetryDisposition::UnsupportedPlantProfile
    } else if unknown {
        TelemetryDisposition::UnknownReviewRequired
    } else if missing_optional {
        TelemetryDisposition::TelemetryAcceptedWithMissingOptionalFields
    } else {
        TelemetryDisposition::TelemetryAcceptedAsObservation
    };

    TelemetryValidation {
        disposition,
        reasons,
        next_cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(kind: SensorKind, unit: &str) -> SensorRule {
        SensorRule {
            kind,
            unit_id: unit.into(),
            minimum_milliunits: 0,
            maximum_milliunits: 1_000_000,
            maximum_age_ticks: 10,
        }
    }

    fn profile(kind: PlantProfileKind) -> DredgePlantProfile {
        let (required_sensors, optional_sensors) = match kind {
            PlantProfileKind::Hopper => (
                vec![
                    rule(SensorKind::Draft, "mm"),
                    rule(SensorKind::Displacement, "kg"),
                ],
                vec![rule(SensorKind::HopperLoad, "kg")],
            ),
            PlantProfileKind::Mechanical => (
                vec![
                    rule(SensorKind::BucketPosition, "mm"),
                    rule(SensorKind::BucketState, "state"),
                ],
                vec![rule(SensorKind::Draft, "mm")],
            ),
            PlantProfileKind::PipelineStandard => (
                vec![
                    rule(SensorKind::SlurryDensity, "kg_m3"),
                    rule(SensorKind::SlurryVelocity, "mm_s"),
                ],
                vec![rule(SensorKind::PumpState, "state")],
            ),
            PlantProfileKind::PipelineSmallBusiness => (
                vec![rule(SensorKind::PumpState, "state")],
                vec![
                    rule(SensorKind::SlurryDensity, "kg_m3"),
                    rule(SensorKind::SlurryVelocity, "mm_s"),
                ],
            ),
            PlantProfileKind::ScowMonitoring => (
                vec![
                    rule(SensorKind::Draft, "mm"),
                    rule(SensorKind::Displacement, "kg"),
                ],
                vec![
                    rule(SensorKind::BinUllage, "mm"),
                    rule(SensorKind::BinVolume, "litre"),
                ],
            ),
            PlantProfileKind::ScowUllage => (
                vec![
                    rule(SensorKind::Draft, "mm"),
                    rule(SensorKind::Displacement, "kg"),
                    rule(SensorKind::BinUllage, "mm"),
                    rule(SensorKind::BinVolume, "litre"),
                ],
                vec![],
            ),
        };
        DredgePlantProfile {
            profile_id: "plant-profile-01".into(),
            revision: "profile-r7".into(),
            kind,
            required_sensors,
            optional_sensors,
            position_rule: Some(PositionRule {
                expected_crs: "EPSG:4326".into(),
                maximum_uncertainty_mm: 5_000,
                maximum_age_ticks: 5,
            }),
            valid_through_tick: Some(1_000),
        }
    }

    fn sensor(kind: SensorKind, unit: &str) -> SensorReading {
        SensorReading {
            kind,
            sensor_id: format!("sensor-{kind:?}"),
            value_milliunits: 500,
            unit_id: unit.into(),
            observed_at_tick: 98,
            calibrated: true,
            calibration_valid_through_tick: Some(500),
        }
    }

    fn record(kind: PlantProfileKind) -> TelemetryRecord {
        let p = profile(kind);
        TelemetryRecord {
            event_id: "event-1".into(),
            payload_digest: "sha256:synthetic-digest-1".into(),
            sequence: 1,
            plant_id: "plant-01".into(),
            profile_id: p.profile_id,
            profile_revision: p.revision,
            profile_kind: kind,
            observed_at_tick: 98,
            received_at_tick: 99,
            position: Some(PositionObservation {
                longitude_microdegrees: 18_000_000,
                latitude_microdegrees: -34_000_000,
                crs: "EPSG:4326".into(),
                uncertainty_mm: 1_000,
                observed_at_tick: 98,
            }),
            sensors: match kind {
                PlantProfileKind::Hopper => vec![
                    sensor(SensorKind::Draft, "mm"),
                    sensor(SensorKind::Displacement, "kg"),
                ],
                PlantProfileKind::Mechanical => vec![
                    sensor(SensorKind::BucketPosition, "mm"),
                    sensor(SensorKind::BucketState, "state"),
                ],
                PlantProfileKind::PipelineStandard => vec![
                    sensor(SensorKind::SlurryDensity, "kg_m3"),
                    sensor(SensorKind::SlurryVelocity, "mm_s"),
                ],
                PlantProfileKind::PipelineSmallBusiness => {
                    vec![sensor(SensorKind::PumpState, "state")]
                }
                PlantProfileKind::ScowMonitoring => vec![
                    sensor(SensorKind::Draft, "mm"),
                    sensor(SensorKind::Displacement, "kg"),
                ],
                PlantProfileKind::ScowUllage => vec![
                    sensor(SensorKind::Draft, "mm"),
                    sensor(SensorKind::Displacement, "kg"),
                    sensor(SensorKind::BinUllage, "mm"),
                    sensor(SensorKind::BinVolume, "litre"),
                ],
            },
        }
    }

    fn evaluate(
        p: &DredgePlantProfile,
        r: &TelemetryRecord,
        c: &TelemetryCursor,
    ) -> TelemetryValidation {
        validate_telemetry(p, r, c, 100, 10)
    }

    #[test]
    fn hopper_profile_accepts_declared_required_sensors() {
        let p = profile(PlantProfileKind::Hopper);
        let r = record(PlantProfileKind::Hopper);
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::TelemetryAcceptedWithMissingOptionalFields
        );
    }

    #[test]
    fn mechanical_profile_accepts_declared_bucket_signals() {
        let p = profile(PlantProfileKind::Mechanical);
        let r = record(PlantProfileKind::Mechanical);
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::TelemetryAcceptedWithMissingOptionalFields
        );
    }

    #[test]
    fn pipeline_standard_requires_density_and_velocity() {
        let p = profile(PlantProfileKind::PipelineStandard);
        let r = record(PlantProfileKind::PipelineStandard);
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::TelemetryAcceptedWithMissingOptionalFields
        );
    }

    #[test]
    fn pipeline_small_business_accepts_without_density_or_velocity() {
        let p = profile(PlantProfileKind::PipelineSmallBusiness);
        let r = record(PlantProfileKind::PipelineSmallBusiness);
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::TelemetryAcceptedWithMissingOptionalFields
        );
    }

    #[test]
    fn pipeline_small_business_preserves_density_and_velocity_when_present() {
        let p = profile(PlantProfileKind::PipelineSmallBusiness);
        let mut r = record(PlantProfileKind::PipelineSmallBusiness);
        r.sensors.push(sensor(SensorKind::SlurryDensity, "kg_m3"));
        r.sensors.push(sensor(SensorKind::SlurryVelocity, "mm_s"));
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::TelemetryAcceptedAsObservation
        );
    }

    #[test]
    fn scow_monitoring_does_not_require_ullage_sensors() {
        let p = profile(PlantProfileKind::ScowMonitoring);
        let r = record(PlantProfileKind::ScowMonitoring);
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::TelemetryAcceptedWithMissingOptionalFields
        );
    }

    #[test]
    fn scow_ullage_requires_bin_level_and_volume_sensors() {
        let p = profile(PlantProfileKind::ScowUllage);
        let r = record(PlantProfileKind::ScowUllage);
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::TelemetryAcceptedAsObservation
        );
    }

    #[test]
    fn plant_profile_mismatch_is_unsupported() {
        let p = profile(PlantProfileKind::PipelineStandard);
        let mut r = record(PlantProfileKind::PipelineSmallBusiness);
        r.profile_kind = PlantProfileKind::PipelineSmallBusiness;
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::UnsupportedPlantProfile
        );
    }

    #[test]
    fn profile_revision_mismatch_is_unsupported() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.profile_revision = "old-profile".into();
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::UnsupportedPlantProfile
        );
    }

    #[test]
    fn missing_required_sensor_requires_review() {
        let p = profile(PlantProfileKind::PipelineStandard);
        let mut r = record(PlantProfileKind::PipelineStandard);
        r.sensors.pop();
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn missing_optional_sensor_is_not_an_error() {
        let p = profile(PlantProfileKind::Hopper);
        let r = record(PlantProfileKind::Hopper);
        let result = evaluate(&p, &r, &TelemetryCursor::default());
        assert_eq!(
            result.disposition,
            TelemetryDisposition::TelemetryAcceptedWithMissingOptionalFields
        );
        assert!(
            result
                .reasons
                .contains(&TelemetryReason::MissingOptionalSensor)
        );
    }

    #[test]
    fn expired_profile_requires_review() {
        let mut p = profile(PlantProfileKind::Hopper);
        p.valid_through_tick = Some(99);
        let r = record(PlantProfileKind::Hopper);
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn future_sensor_observation_is_inconsistent() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.sensors[0].observed_at_tick = 101;
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn profile_without_validity_horizon_is_unknown() {
        let mut p = profile(PlantProfileKind::Hopper);
        p.valid_through_tick = None;
        let r = record(PlantProfileKind::Hopper);
        let result = evaluate(&p, &r, &TelemetryCursor::default());
        assert_eq!(
            result.disposition,
            TelemetryDisposition::UnknownReviewRequired
        );
        assert!(
            result
                .reasons
                .contains(&TelemetryReason::ProfileCurrentnessUnknown)
        );
    }

    #[test]
    fn position_outside_geographic_range_is_inconsistent() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.position.as_mut().unwrap().latitude_microdegrees = 91_000_000;
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn position_staleness_requires_review() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.position.as_mut().unwrap().observed_at_tick = 90;
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn missing_calibration_is_unknown() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.sensors[0].calibration_valid_through_tick = None;
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn expired_calibration_is_unknown() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.sensors[0].calibration_valid_through_tick = Some(99);
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn sensor_unit_mismatch_is_inconsistent() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.sensors[0].unit_id = "cm".into();
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn sensor_outside_profile_range_is_inconsistent() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.sensors[0].value_milliunits = 1_000_001;
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn duplicate_sensor_kind_is_inconsistent() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.sensors.push(r.sensors[0].clone());
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn missing_position_requires_review() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.position = None;
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn position_crs_mismatch_is_inconsistent() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.position.as_mut().unwrap().crs = "unknown".into();
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn excessive_position_uncertainty_requires_review() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.position.as_mut().unwrap().uncertainty_mm = 5_001;
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn exact_duplicate_event_is_idempotent() {
        let p = profile(PlantProfileKind::Hopper);
        let r = record(PlantProfileKind::Hopper);
        let first = evaluate(&p, &r, &TelemetryCursor::default());
        let replay = evaluate(&p, &r, &first.next_cursor);
        assert_eq!(
            replay.disposition,
            TelemetryDisposition::DuplicateExactReplay
        );
        assert_eq!(replay.next_cursor, first.next_cursor);
    }

    #[test]
    fn same_digest_does_not_hide_changed_record_fields() {
        let p = profile(PlantProfileKind::Hopper);
        let r = record(PlantProfileKind::Hopper);
        let first = evaluate(&p, &r, &TelemetryCursor::default());
        let mut changed = r;
        changed.sensors[0].value_milliunits += 1;
        assert_eq!(
            evaluate(&p, &changed, &first.next_cursor).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn duplicate_event_with_changed_digest_is_inconsistent() {
        let p = profile(PlantProfileKind::Hopper);
        let r = record(PlantProfileKind::Hopper);
        let first = evaluate(&p, &r, &TelemetryCursor::default());
        let mut changed = r;
        changed.payload_digest = "sha256:changed".into();
        assert_eq!(
            evaluate(&p, &changed, &first.next_cursor).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn sequence_collision_is_inconsistent() {
        let p = profile(PlantProfileKind::Hopper);
        let r = record(PlantProfileKind::Hopper);
        let first = evaluate(&p, &r, &TelemetryCursor::default());
        let mut collision = r;
        collision.event_id = "event-other".into();
        collision.payload_digest = "sha256:other".into();
        assert_eq!(
            evaluate(&p, &collision, &first.next_cursor).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn sequence_gap_is_unknown_and_recorded_as_pending_evidence() {
        let p = profile(PlantProfileKind::Hopper);
        let r = record(PlantProfileKind::Hopper);
        let first = evaluate(&p, &r, &TelemetryCursor::default());
        let mut gap = record(PlantProfileKind::Hopper);
        gap.event_id = "event-3".into();
        gap.sequence = 3;
        gap.payload_digest = "sha256:3".into();
        let result = evaluate(&p, &gap, &first.next_cursor);
        assert_eq!(
            result.disposition,
            TelemetryDisposition::UnknownReviewRequired
        );
        assert_eq!(result.next_cursor.contiguous_sequence, 1);
        assert!(result.reasons.contains(&TelemetryReason::SequenceGap));
    }

    #[test]
    fn filling_sequence_gap_advances_contiguous_cursor() {
        let p = profile(PlantProfileKind::Hopper);
        let first = evaluate(
            &p,
            &record(PlantProfileKind::Hopper),
            &TelemetryCursor::default(),
        );
        let mut gap = record(PlantProfileKind::Hopper);
        gap.event_id = "event-3".into();
        gap.sequence = 3;
        gap.payload_digest = "sha256:3".into();
        let pending = evaluate(&p, &gap, &first.next_cursor);
        let mut missing = record(PlantProfileKind::Hopper);
        missing.event_id = "event-2".into();
        missing.sequence = 2;
        missing.payload_digest = "sha256:2".into();
        let filled = evaluate(&p, &missing, &pending.next_cursor);
        assert_eq!(filled.next_cursor.contiguous_sequence, 3);
    }

    #[test]
    fn out_of_order_sequence_is_rejected() {
        let p = profile(PlantProfileKind::Hopper);
        let first = evaluate(
            &p,
            &record(PlantProfileKind::Hopper),
            &TelemetryCursor::default(),
        );
        let mut older = record(PlantProfileKind::Hopper);
        older.event_id = "event-old".into();
        older.sequence = 0;
        older.payload_digest = "sha256:old".into();
        assert_eq!(
            evaluate(&p, &older, &first.next_cursor).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn stale_event_requires_review() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.observed_at_tick = 1;
        r.received_at_tick = 2;
        assert_eq!(
            validate_telemetry(&p, &r, &TelemetryCursor::default(), 100, 10).disposition,
            TelemetryDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn event_clock_ahead_of_receiver_is_inconsistent() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.observed_at_tick = 101;
        r.received_at_tick = 99;
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn slurry_density_without_solids_fraction_never_produces_dry_mass() {
        let p = profile(PlantProfileKind::PipelineStandard);
        let r = record(PlantProfileKind::PipelineStandard);
        let result = evaluate(&p, &r, &TelemetryCursor::default());
        assert_eq!(
            result.disposition,
            TelemetryDisposition::TelemetryAcceptedWithMissingOptionalFields
        );
        // This crate has no dry-mass output type or conversion function by design.
    }

    #[test]
    fn invalid_profile_shape_is_unsupported() {
        let mut p = profile(PlantProfileKind::Hopper);
        p.required_sensors.push(p.required_sensors[0].clone());
        let r = record(PlantProfileKind::Hopper);
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::UnsupportedPlantProfile
        );
    }

    #[test]
    fn empty_digest_is_inconsistent() {
        let p = profile(PlantProfileKind::Hopper);
        let mut r = record(PlantProfileKind::Hopper);
        r.payload_digest.clear();
        assert_eq!(
            evaluate(&p, &r, &TelemetryCursor::default()).disposition,
            TelemetryDisposition::EvidenceInconsistent
        );
    }
}
