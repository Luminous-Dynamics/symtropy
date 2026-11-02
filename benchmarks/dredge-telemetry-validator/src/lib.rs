//! Synthetic source-profiled dredge telemetry validator.
//!
//! Accepts or rejects observation envelopes only. It does not calculate dry
//! mass, certify DQM conformance, model sediment transport or authorize work.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlantType {
    Hopper,
    Mechanical,
    Pipeline,
    Scow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProfileKind {
    Hopper,
    Mechanical,
    PipelineStandard,
    PipelineSmallBusiness,
    ScowMonitoring,
    ScowUllage,
}

impl ProfileKind {
    #[must_use]
    pub fn plant_type(self) -> PlantType {
        match self {
            Self::Hopper => PlantType::Hopper,
            Self::Mechanical => PlantType::Mechanical,
            Self::PipelineStandard | Self::PipelineSmallBusiness => PlantType::Pipeline,
            Self::ScowMonitoring | Self::ScowUllage => PlantType::Scow,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SensorKind {
    Position,
    OperationalState,
    Draft,
    Displacement,
    SlurryDensity,
    SlurryVelocity,
    BinUllage,
    BinVolume,
    PumpPressure,
    CutterPower,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorRule {
    pub kind: SensorKind,
    pub unit_id: String,
    pub minimum_value: i64,
    pub maximum_value: i64,
    pub required_by_profile: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlantCertification {
    pub certification_id: String,
    pub plant_id: String,
    pub configuration_revision: String,
    pub valid_through_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DredgeTelemetryProfile {
    pub profile_id: String,
    pub profile_revision: String,
    pub plant_id: String,
    pub plant_type: PlantType,
    pub profile_kind: ProfileKind,
    pub configuration_revision: String,
    pub reference_datum: String,
    pub maximum_position_uncertainty_mm: u64,
    pub maximum_clock_lag_ticks: u64,
    pub maximum_sensor_age_ticks: u64,
    pub required_sensors: Vec<SensorRule>,
    pub optional_sensors: Vec<SensorRule>,
    pub certification: Option<PlantCertification>,
    pub instrumentation_plan_id: String,
    pub instrumentation_plan_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionFix {
    pub latitude_microdegrees: i32,
    pub longitude_microdegrees: i32,
    pub datum_id: String,
    pub horizontal_uncertainty_mm: u64,
    pub measured_at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorReading {
    pub kind: SensorKind,
    pub value: i64,
    pub unit_id: String,
    pub measured_at_tick: u64,
    pub instrument_id: String,
    pub calibration_id: String,
    pub calibrated: bool,
    pub calibration_valid_through_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScowLoadGeometry {
    SelfLevelled,
    Mounded,
    UnevenSettling,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DredgeTelemetryRecord {
    pub event_id: String,
    /// Digest supplied by the authenticated ingest boundary; not recomputed here.
    pub payload_digest: String,
    pub sequence_number: u64,
    pub observed_at_tick: u64,
    pub received_at_tick: u64,
    pub plant_id: String,
    pub profile_revision: String,
    pub configuration_revision: String,
    pub position: Option<PositionFix>,
    pub sensor_readings: Vec<SensorReading>,
    pub scow_load_geometry: Option<ScowLoadGeometry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryDisposition {
    TelemetryAcceptedAsObservation,
    TelemetryAcceptedWithMissingOptionalFields,
    UnknownReviewRequired,
    EvidenceInconsistent,
    UnsupportedPlantProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryReason {
    InvalidProfileIdentity,
    PlantProfilePairingMismatch,
    DuplicateSensorRule,
    InvalidSensorRule,
    InvalidRecordIdentity,
    PlantIdentityMismatch,
    ProfileRevisionMismatch,
    ConfigurationRevisionMismatch,
    MissingCertification,
    CertificationIdentityMismatch,
    CertificationExpired,
    MissingInstrumentationPlan,
    MissingPosition,
    PositionDatumMismatch,
    PositionOutsideRange,
    PositionUncertaintyExceeded,
    FutureObservation,
    TelemetryClockLagExceeded,
    SequenceNotIncreasing,
    SequenceGap,
    DuplicateEventReplay,
    PayloadDigestChanged,
    MissingRequiredSensor,
    MissingOptionalSensor,
    DuplicateSensorReading,
    UnsupportedSensor,
    SensorUnitMismatch,
    SensorValueOutsideQualifiedRange,
    SensorFutureTimestamp,
    SensorStale,
    MissingInstrumentIdentity,
    MissingCalibrationIdentity,
    SensorNotCalibrated,
    SensorCalibrationExpired,
    MissingScowGeometry,
    ScowVolumeEstimateUnreliable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryValidation {
    pub disposition: TelemetryDisposition,
    pub reasons: Vec<TelemetryReason>,
    pub missing_optional_sensors: Vec<SensorKind>,
    /// True only for an explicitly self-levelled scow load with an Ullage profile.
    pub scow_volume_interpretation_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SeenEvent {
    digest: String,
}

pub struct DredgeTelemetryValidator {
    profile: DredgeTelemetryProfile,
    last_sequence: Option<u64>,
    last_observed_tick: Option<u64>,
    seen_events: BTreeMap<String, SeenEvent>,
}

impl DredgeTelemetryValidator {
    #[must_use]
    pub fn new(profile: DredgeTelemetryProfile) -> Self {
        Self {
            profile,
            last_sequence: None,
            last_observed_tick: None,
            seen_events: BTreeMap::new(),
        }
    }

    pub fn profile(&self) -> &DredgeTelemetryProfile {
        &self.profile
    }

    /// Validate a single record and advance the replay cursor only on acceptance.
    pub fn validate(&mut self, record: &DredgeTelemetryRecord) -> TelemetryValidation {
        let mut reasons = Vec::new();
        let mut inconsistent = false;
        let mut unsupported = false;
        let mut unknown = false;
        let mut missing_optional = Vec::new();
        let mut scow_volume_supported = true;

        if !nonempty(&self.profile.profile_id)
            || !nonempty(&self.profile.profile_revision)
            || !nonempty(&self.profile.plant_id)
            || !nonempty(&self.profile.configuration_revision)
            || !nonempty(&self.profile.reference_datum)
        {
            reasons.push(TelemetryReason::InvalidProfileIdentity);
            inconsistent = true;
        }
        if self.profile.profile_kind.plant_type() != self.profile.plant_type {
            reasons.push(TelemetryReason::PlantProfilePairingMismatch);
            unsupported = true;
        }
        if !rules_valid(&self.profile.required_sensors)
            || !rules_valid(&self.profile.optional_sensors)
            || self.profile.required_sensors.iter().any(|required| {
                self.profile.optional_sensors.iter().any(|optional| optional.kind == required.kind)
            })
        {
            reasons.push(TelemetryReason::InvalidSensorRule);
            inconsistent = true;
        }
        if has_duplicate_rules(&self.profile.required_sensors)
            || has_duplicate_rules(&self.profile.optional_sensors)
        {
            reasons.push(TelemetryReason::DuplicateSensorRule);
            inconsistent = true;
        }

        if !nonempty(&record.event_id) || !nonempty(&record.payload_digest)
            || !nonempty(&record.plant_id) || !nonempty(&record.profile_revision)
            || !nonempty(&record.configuration_revision)
        {
            reasons.push(TelemetryReason::InvalidRecordIdentity);
            inconsistent = true;
        }
        if record.plant_id != self.profile.plant_id {
            reasons.push(TelemetryReason::PlantIdentityMismatch);
            inconsistent = true;
        }
        if record.profile_revision != self.profile.profile_revision {
            reasons.push(TelemetryReason::ProfileRevisionMismatch);
            inconsistent = true;
        }
        if record.configuration_revision != self.profile.configuration_revision {
            reasons.push(TelemetryReason::ConfigurationRevisionMismatch);
            inconsistent = true;
        }

        match &self.profile.certification {
            None => {
                reasons.push(TelemetryReason::MissingCertification);
                unknown = true;
            }
            Some(cert) => {
                if !nonempty(&cert.certification_id)
                    || cert.plant_id != self.profile.plant_id
                    || cert.configuration_revision != self.profile.configuration_revision
                {
                    reasons.push(TelemetryReason::CertificationIdentityMismatch);
                    inconsistent = true;
                } else if cert.valid_through_tick < record.observed_at_tick {
                    reasons.push(TelemetryReason::CertificationExpired);
                    unknown = true;
                }
            }
        }
        if !nonempty(&self.profile.instrumentation_plan_id)
            || !nonempty(&self.profile.instrumentation_plan_revision)
        {
            reasons.push(TelemetryReason::MissingInstrumentationPlan);
            unknown = true;
        }

        if record.observed_at_tick > record.received_at_tick {
            reasons.push(TelemetryReason::FutureObservation);
            inconsistent = true;
        } else if record.received_at_tick - record.observed_at_tick
            > self.profile.maximum_clock_lag_ticks
        {
            reasons.push(TelemetryReason::TelemetryClockLagExceeded);
            unknown = true;
        }
        if let Some(previous) = self.last_sequence {
            if record.sequence_number <= previous {
                reasons.push(TelemetryReason::SequenceNotIncreasing);
                inconsistent = true;
            } else if record.sequence_number != previous.saturating_add(1) {
                reasons.push(TelemetryReason::SequenceGap);
                unknown = true;
            }
        }
        if let Some(previous) = self.last_observed_tick {
            if record.observed_at_tick <= previous {
                reasons.push(TelemetryReason::SequenceNotIncreasing);
                inconsistent = true;
            }
        }
        if let Some(seen) = self.seen_events.get(&record.event_id) {
            reasons.push(if seen.digest == record.payload_digest {
                TelemetryReason::DuplicateEventReplay
            } else {
                TelemetryReason::PayloadDigestChanged
            });
            inconsistent = true;
        }

        match &record.position {
            None => {
                reasons.push(TelemetryReason::MissingPosition);
                unknown = true;
            }
            Some(position) => {
                if position.datum_id != self.profile.reference_datum {
                    reasons.push(TelemetryReason::PositionDatumMismatch);
                    inconsistent = true;
                }
                if position.latitude_microdegrees.abs() > 90_000_000
                    || position.longitude_microdegrees.abs() > 180_000_000
                {
                    reasons.push(TelemetryReason::PositionOutsideRange);
                    inconsistent = true;
                }
                if position.horizontal_uncertainty_mm
                    > self.profile.maximum_position_uncertainty_mm
                {
                    reasons.push(TelemetryReason::PositionUncertaintyExceeded);
                    unknown = true;
                }
                if position.measured_at_tick > record.received_at_tick {
                    reasons.push(TelemetryReason::SensorFutureTimestamp);
                    inconsistent = true;
                } else if record.received_at_tick - position.measured_at_tick
                    > self.profile.maximum_sensor_age_ticks
                {
                    reasons.push(TelemetryReason::SensorStale);
                    unknown = true;
                }
            }
        }

        let mut readings = BTreeMap::new();
        let mut duplicates = BTreeSet::new();
        for reading in &record.sensor_readings {
            if readings.insert(reading.kind, reading).is_some() {
                duplicates.insert(reading.kind);
            }
        }
        for _ in duplicates {
            reasons.push(TelemetryReason::DuplicateSensorReading);
            inconsistent = true;
        }

        let known_rules: BTreeMap<SensorKind, &SensorRule> = self.profile.required_sensors
            .iter()
            .chain(self.profile.optional_sensors.iter())
            .map(|rule| (rule.kind, rule))
            .collect();
        for kind in readings.keys() {
            if !known_rules.contains_key(kind) {
                reasons.push(TelemetryReason::UnsupportedSensor);
                unsupported = true;
            }
        }

        for rule in self.profile.required_sensors.iter().chain(self.profile.optional_sensors.iter()) {
            let Some(reading) = readings.get(&rule.kind) else {
                if rule.required_by_profile {
                    reasons.push(TelemetryReason::MissingRequiredSensor);
                    unknown = true;
                } else {
                    reasons.push(TelemetryReason::MissingOptionalSensor);
                    missing_optional.push(rule.kind);
                }
                continue;
            };

            if reading.unit_id != rule.unit_id || !nonempty(&reading.unit_id) {
                reasons.push(TelemetryReason::SensorUnitMismatch);
                inconsistent = true;
            }
            if reading.value < rule.minimum_value || reading.value > rule.maximum_value {
                reasons.push(TelemetryReason::SensorValueOutsideQualifiedRange);
                unknown = true;
            }
            if reading.measured_at_tick > record.received_at_tick {
                reasons.push(TelemetryReason::SensorFutureTimestamp);
                inconsistent = true;
            } else if record.received_at_tick - reading.measured_at_tick
                > self.profile.maximum_sensor_age_ticks
            {
                reasons.push(TelemetryReason::SensorStale);
                unknown = true;
            }
            if !nonempty(&reading.instrument_id) {
                reasons.push(TelemetryReason::MissingInstrumentIdentity);
                inconsistent = true;
            }
            if !nonempty(&reading.calibration_id) {
                reasons.push(TelemetryReason::MissingCalibrationIdentity);
                inconsistent = true;
            }
            if !reading.calibrated {
                reasons.push(TelemetryReason::SensorNotCalibrated);
                unknown = true;
            }
            if reading.calibration_valid_through_tick < record.observed_at_tick {
                reasons.push(TelemetryReason::SensorCalibrationExpired);
                unknown = true;
            }
        }

        if self.profile.profile_kind == ProfileKind::ScowUllage {
            match record.scow_load_geometry {
                None | Some(ScowLoadGeometry::Unknown) => {
                    reasons.push(TelemetryReason::MissingScowGeometry);
                    unknown = true;
                }
                Some(ScowLoadGeometry::Mounded | ScowLoadGeometry::UnevenSettling) => {
                    reasons.push(TelemetryReason::ScowVolumeEstimateUnreliable);
                    scow_volume_supported = false;
                }
                Some(ScowLoadGeometry::SelfLevelled) => {}
            }
        }

        // Evidence integrity and unsupported profile errors never become accepted.
        // Unknown evidence does not update the cursor, so a gap can be reconciled.
        let disposition = if inconsistent {
            TelemetryDisposition::EvidenceInconsistent
        } else if unsupported {
            TelemetryDisposition::UnsupportedPlantProfile
        } else if unknown {
            TelemetryDisposition::UnknownReviewRequired
        } else if !missing_optional.is_empty() {
            TelemetryDisposition::TelemetryAcceptedWithMissingOptionalFields
        } else {
            TelemetryDisposition::TelemetryAcceptedAsObservation
        };

        if matches!(
            disposition,
            TelemetryDisposition::TelemetryAcceptedAsObservation
                | TelemetryDisposition::TelemetryAcceptedWithMissingOptionalFields
        ) {
            self.last_sequence = Some(record.sequence_number);
            self.last_observed_tick = Some(record.observed_at_tick);
            self.seen_events.insert(
                record.event_id.clone(),
                SeenEvent { digest: record.payload_digest.clone() },
            );
        }

        TelemetryValidation {
            disposition,
            reasons,
            missing_optional_sensors: missing_optional,
            scow_volume_interpretation_supported: scow_volume_supported,
        }
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn rules_valid(rules: &[SensorRule]) -> bool {
    rules.iter().all(|rule| {
        nonempty(&rule.unit_id) && rule.minimum_value <= rule.maximum_value
    })
}

fn has_duplicate_rules(rules: &[SensorRule]) -> bool {
    let mut seen = BTreeSet::new();
    rules.iter().any(|rule| !seen.insert(rule.kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(kind: SensorKind, unit: &str, required: bool) -> SensorRule {
        SensorRule {
            kind,
            unit_id: unit.into(),
            minimum_value: 0,
            maximum_value: 10_000,
            required_by_profile: required,
        }
    }

    fn profile(kind: ProfileKind) -> DredgeTelemetryProfile {
        let (required_sensors, optional_sensors) = match kind {
            ProfileKind::PipelineStandard => (
                vec![
                    rule(SensorKind::OperationalState, "state-code", true),
                    rule(SensorKind::SlurryDensity, "kg-m3", true),
                    rule(SensorKind::SlurryVelocity, "m-s", true),
                ],
                vec![],
            ),
            ProfileKind::PipelineSmallBusiness => (
                vec![
                    rule(SensorKind::Position, "position-fix", true),
                    rule(SensorKind::OperationalState, "state-code", true),
                ],
                vec![
                    rule(SensorKind::SlurryDensity, "kg-m3", false),
                    rule(SensorKind::SlurryVelocity, "m-s", false),
                ],
            ),
            ProfileKind::ScowMonitoring => (
                vec![
                    rule(SensorKind::Position, "position-fix", true),
                    rule(SensorKind::Draft, "mm", true),
                    rule(SensorKind::Displacement, "kg", true),
                ],
                vec![],
            ),
            ProfileKind::ScowUllage => (
                vec![
                    rule(SensorKind::Position, "position-fix", true),
                    rule(SensorKind::Draft, "mm", true),
                    rule(SensorKind::Displacement, "kg", true),
                    rule(SensorKind::BinUllage, "mm", true),
                    rule(SensorKind::BinVolume, "litre", true),
                ],
                vec![],
            ),
            ProfileKind::Hopper => (
                vec![
                    rule(SensorKind::Position, "position-fix", true),
                    rule(SensorKind::OperationalState, "state-code", true),
                ],
                vec![],
            ),
            ProfileKind::Mechanical => (
                vec![
                    rule(SensorKind::Position, "position-fix", true),
                    rule(SensorKind::OperationalState, "state-code", true),
                ],
                vec![],
            ),
        };
        DredgeTelemetryProfile {
            profile_id: "contract-profile-1".into(),
            profile_revision: "profile-r2".into(),
            plant_id: "dredge-01".into(),
            plant_type: kind.plant_type(),
            profile_kind: kind,
            configuration_revision: "asbuilt-r8".into(),
            reference_datum: "EPSG:4326".into(),
            maximum_position_uncertainty_mm: 5_000,
            maximum_clock_lag_ticks: 5,
            maximum_sensor_age_ticks: 5,
            required_sensors,
            optional_sensors,
            certification: Some(PlantCertification {
                certification_id: "dqm-cert-21".into(),
                plant_id: "dredge-01".into(),
                configuration_revision: "asbuilt-r8".into(),
                valid_through_tick: 10_000,
            }),
            instrumentation_plan_id: "dpip-01".into(),
            instrumentation_plan_revision: "dpip-r3".into(),
        }
    }

    fn reading(kind: SensorKind, unit: &str, value: i64) -> SensorReading {
        SensorReading {
            kind,
            value,
            unit_id: unit.into(),
            measured_at_tick: 100,
            instrument_id: format!("instrument-{kind:?}"),
            calibration_id: format!("calibration-{kind:?}"),
            calibrated: true,
            calibration_valid_through_tick: 1_000,
        }
    }

    fn record(kind: ProfileKind) -> DredgeTelemetryRecord {
        let p = profile(kind);
        let mut sensor_readings = Vec::new();
        for rule in &p.required_sensors {
            if rule.kind != SensorKind::Position {
                sensor_readings.push(reading(rule.kind, &rule.unit_id, 1_000));
            }
        }
        DredgeTelemetryRecord {
            event_id: "event-1".into(),
            payload_digest: "sha256:abc123".into(),
            sequence_number: 1,
            observed_at_tick: 100,
            received_at_tick: 101,
            plant_id: p.plant_id,
            profile_revision: p.profile_revision,
            configuration_revision: p.configuration_revision,
            position: Some(PositionFix {
                latitude_microdegrees: -26_200_000,
                longitude_microdegrees: 28_000_000,
                datum_id: "EPSG:4326".into(),
                horizontal_uncertainty_mm: 1_000,
                measured_at_tick: 100,
            }),
            sensor_readings,
            scow_load_geometry: if kind == ProfileKind::ScowUllage {
                Some(ScowLoadGeometry::SelfLevelled)
            } else {
                None
            },
        }
    }

    #[test]
    fn pipeline_standard_requires_density_and_velocity() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let r = v.validate(&record(ProfileKind::PipelineStandard));
        assert_eq!(r.disposition, TelemetryDisposition::TelemetryAcceptedAsObservation);
    }

    #[test]
    fn pipeline_small_business_accepts_missing_optional_density_velocity_without_mass_inference() {
        let p = profile(ProfileKind::PipelineSmallBusiness);
        let mut v = DredgeTelemetryValidator::new(p);
        let r = v.validate(&record(ProfileKind::PipelineSmallBusiness));
        assert_eq!(r.disposition, TelemetryDisposition::TelemetryAcceptedWithMissingOptionalFields);
        assert!(r.missing_optional_sensors.contains(&SensorKind::SlurryDensity));
        assert!(r.missing_optional_sensors.contains(&SensorKind::SlurryVelocity));
    }

    #[test]
    fn pipeline_small_business_preserves_optional_instrumented_density_and_velocity() {
        let p = profile(ProfileKind::PipelineSmallBusiness);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineSmallBusiness);
        rec.sensor_readings.push(reading(SensorKind::SlurryDensity, "kg-m3", 1_025));
        rec.sensor_readings.push(reading(SensorKind::SlurryVelocity, "m-s", 2_500));
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::TelemetryAcceptedAsObservation);
    }

    #[test]
    fn scow_monitoring_requires_draft_and_displacement_without_ullage() {
        let p = profile(ProfileKind::ScowMonitoring);
        let mut v = DredgeTelemetryValidator::new(p);
        assert_eq!(v.validate(&record(ProfileKind::ScowMonitoring)).disposition,
            TelemetryDisposition::TelemetryAcceptedAsObservation);
    }

    #[test]
    fn scow_ullage_accepts_uniform_load_for_volume_interpretation_only() {
        let p = profile(ProfileKind::ScowUllage);
        let mut v = DredgeTelemetryValidator::new(p);
        let r = v.validate(&record(ProfileKind::ScowUllage));
        assert_eq!(r.disposition, TelemetryDisposition::TelemetryAcceptedAsObservation);
        assert!(r.scow_volume_interpretation_supported);
    }

    #[test]
    fn scow_ullage_mounded_load_preserves_raw_observation_but_marks_volume_unreliable() {
        let p = profile(ProfileKind::ScowUllage);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::ScowUllage);
        rec.scow_load_geometry = Some(ScowLoadGeometry::Mounded);
        let r = v.validate(&rec);
        assert_eq!(r.disposition, TelemetryDisposition::TelemetryAcceptedAsObservation);
        assert!(r.reasons.contains(&TelemetryReason::ScowVolumeEstimateUnreliable));
        assert!(!r.scow_volume_interpretation_supported);
    }

    #[test]
    fn plant_profile_type_mismatch_is_unsupported() {
        let mut p = profile(ProfileKind::ScowMonitoring);
        p.plant_type = PlantType::Pipeline;
        let mut v = DredgeTelemetryValidator::new(p);
        assert_eq!(v.validate(&record(ProfileKind::ScowMonitoring)).disposition,
            TelemetryDisposition::UnsupportedPlantProfile);
    }

    #[test]
    fn profile_revision_mismatch_is_inconsistent() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.profile_revision = "profile-r1".into();
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::EvidenceInconsistent);
    }

    #[test]
    fn configuration_revision_mismatch_is_inconsistent() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.configuration_revision = "asbuilt-r7".into();
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::EvidenceInconsistent);
    }

    #[test]
    fn expired_certification_requires_review() {
        let mut p = profile(ProfileKind::PipelineStandard);
        p.certification.as_mut().unwrap().valid_through_tick = 99;
        let mut v = DredgeTelemetryValidator::new(p);
        assert_eq!(v.validate(&record(ProfileKind::PipelineStandard)).disposition,
            TelemetryDisposition::UnknownReviewRequired);
    }

    #[test]
    fn certification_bound_to_other_configuration_is_inconsistent() {
        let mut p = profile(ProfileKind::PipelineStandard);
        p.certification.as_mut().unwrap().configuration_revision = "asbuilt-r7".into();
        let mut v = DredgeTelemetryValidator::new(p);
        assert_eq!(v.validate(&record(ProfileKind::PipelineStandard)).disposition,
            TelemetryDisposition::EvidenceInconsistent);
    }

    #[test]
    fn missing_required_sensor_requires_review() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.sensor_readings.retain(|s| s.kind != SensorKind::SlurryDensity);
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::UnknownReviewRequired);
    }

    #[test]
    fn unknown_sensor_is_not_silently_discarded() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.sensor_readings.push(reading(SensorKind::CutterPower, "kw", 10));
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::UnsupportedPlantProfile);
    }

    #[test]
    fn sensor_unit_mismatch_is_inconsistent() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.sensor_readings[0].unit_id = "lb-ft".into();
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::EvidenceInconsistent);
    }

    #[test]
    fn sensor_outside_qualified_range_requires_review() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.sensor_readings[0].value = 10_001;
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::UnknownReviewRequired);
    }

    #[test]
    fn uncalibrated_sensor_requires_review() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.sensor_readings[0].calibrated = false;
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::UnknownReviewRequired);
    }

    #[test]
    fn expired_sensor_calibration_requires_review() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.sensor_readings[0].calibration_valid_through_tick = 99;
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::UnknownReviewRequired);
    }

    #[test]
    fn missing_position_requires_review() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.position = None;
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::UnknownReviewRequired);
    }

    #[test]
    fn position_datum_mismatch_is_inconsistent() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.position.as_mut().unwrap().datum_id = "other-datum".into();
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::EvidenceInconsistent);
    }

    #[test]
    fn high_position_uncertainty_requires_review() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.position.as_mut().unwrap().horizontal_uncertainty_mm = 5_001;
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::UnknownReviewRequired);
    }

    #[test]
    fn future_event_timestamp_is_inconsistent() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.observed_at_tick = 102;
        rec.received_at_tick = 101;
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::EvidenceInconsistent);
    }

    #[test]
    fn duplicate_event_replay_is_inconsistent() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let rec = record(ProfileKind::PipelineStandard);
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::TelemetryAcceptedAsObservation);
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::EvidenceInconsistent);
    }

    #[test]
    fn changed_payload_digest_for_same_event_id_is_inconsistent() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let rec = record(ProfileKind::PipelineStandard);
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::TelemetryAcceptedAsObservation);
        let mut changed = rec;
        changed.payload_digest = "sha256:tampered".into();
        assert_eq!(v.validate(&changed).disposition, TelemetryDisposition::EvidenceInconsistent);
    }

    #[test]
    fn sequence_gap_does_not_advance_the_cursor() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let rec = record(ProfileKind::PipelineStandard);
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::TelemetryAcceptedAsObservation);
        let mut gap = record(ProfileKind::PipelineStandard);
        gap.event_id = "event-3".into();
        gap.payload_digest = "sha256:def456".into();
        gap.sequence_number = 3;
        gap.observed_at_tick = 102;
        gap.received_at_tick = 103;
        assert_eq!(v.validate(&gap).disposition, TelemetryDisposition::UnknownReviewRequired);
        let mut next = gap;
        next.event_id = "event-2".into();
        next.payload_digest = "sha256:abc222".into();
        next.sequence_number = 2;
        next.observed_at_tick = 101;
        next.received_at_tick = 102;
        assert_eq!(v.validate(&next).disposition, TelemetryDisposition::TelemetryAcceptedAsObservation);
    }

    #[test]
    fn stale_position_requires_review() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let mut rec = record(ProfileKind::PipelineStandard);
        rec.position.as_mut().unwrap().measured_at_tick = 90;
        assert_eq!(v.validate(&rec).disposition, TelemetryDisposition::UnknownReviewRequired);
    }

    #[test]
    fn missing_optional_sensors_are_not_zero_filled() {
        let p = profile(ProfileKind::PipelineSmallBusiness);
        let mut v = DredgeTelemetryValidator::new(p);
        let r = v.validate(&record(ProfileKind::PipelineSmallBusiness));
        assert!(r.missing_optional_sensors.contains(&SensorKind::SlurryDensity));
        assert!(r.missing_optional_sensors.contains(&SensorKind::SlurryVelocity));
    }

    #[test]
    fn missing_instrumentation_plan_requires_review() {
        let mut p = profile(ProfileKind::PipelineStandard);
        p.instrumentation_plan_revision = String::new();
        let mut v = DredgeTelemetryValidator::new(p);
        let r = v.validate(&record(ProfileKind::PipelineStandard));
        assert_eq!(r.disposition, TelemetryDisposition::UnknownReviewRequired);
    }

    #[test]
    fn telemetry_validation_never_emits_a_dry_mass_transfer() {
        let p = profile(ProfileKind::PipelineStandard);
        let mut v = DredgeTelemetryValidator::new(p);
        let r = v.validate(&record(ProfileKind::PipelineStandard));
        assert!(r.reasons.is_empty());
        // Output has no mass field by design.
    }
}
