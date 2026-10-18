// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Strict subset parser for DCSA JIT timestamp messages.
//!
//! This profile is intentionally separate from the IAPH-IHMA PCO JSON adapter.
//! In particular, an arrival at berth (BRTH) is not an arrival at the Pilot
//! Boarding Place (PBPL). No network calls, negotiation responses, bookings,
//! or schedule effects are performed here.

use super::{
    parse_rfc3339_timestamp, ParsedTimestamp, TimeMeaning, TimestampOffsetSemantics,
    TimestampParseError, ValidationPolicy, NANOS_PER_SECOND,
};

pub const DCSA_JIT_PROFILE: &str = "DCSA-JIT";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventClassifierCode {
    Estimated,
    Requested,
    Planned,
    Actual,
}

impl EventClassifierCode {
    fn parse(value: &str) -> Result<Self, DcsaJitParseError> {
        match value {
            "EST" => Ok(Self::Estimated),
            "REQ" => Ok(Self::Requested),
            "PLN" => Ok(Self::Planned),
            "ACT" => Ok(Self::Actual),
            _ => Err(DcsaJitParseError::UnsupportedCode {
                field: "eventClassifierCode",
                value: value.to_owned(),
            }),
        }
    }

    pub fn time_meaning(&self) -> TimeMeaning {
        match self {
            Self::Estimated => TimeMeaning::Estimated,
            Self::Requested => TimeMeaning::Requested,
            Self::Planned => TimeMeaning::Planned,
            Self::Actual => TimeMeaning::Actual,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationsEventTypeCode {
    Arrival,
    Departure,
}

impl OperationsEventTypeCode {
    fn parse(value: &str) -> Result<Self, DcsaJitParseError> {
        match value {
            "ARRI" => Ok(Self::Arrival),
            "DEPA" => Ok(Self::Departure),
            _ => Err(DcsaJitParseError::UnsupportedCode {
                field: "operationsEventTypeCode",
                value: value.to_owned(),
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FacilityTypeCode {
    Berth,
    PilotBoardingPlace,
}

impl FacilityTypeCode {
    fn parse(value: &str) -> Result<Self, DcsaJitParseError> {
        match value {
            "BRTH" => Ok(Self::Berth),
            "PBPL" => Ok(Self::PilotBoardingPlace),
            _ => Err(DcsaJitParseError::UnsupportedCode {
                field: "facilityTypeCode",
                value: value.to_owned(),
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DcsaIngestContext {
    pub observation_id: String,
    pub source_id: String,
    /// Source publication/update time, not DCSA eventDateTime.
    pub source_event_time_utc_nanos: i128,
    pub ingested_at_utc_nanos: i128,
    pub valid_until_utc_nanos: Option<i128>,
    pub schema_version: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DcsaJitTimestamp {
    pub profile: String,
    pub context: DcsaIngestContext,
    pub event_classifier_code: EventClassifierCode,
    pub operations_event_type_code: OperationsEventTypeCode,
    pub facility_type_code: FacilityTypeCode,
    /// v0.1 accepts only absent/null phase and service codes, never guesses
    /// how a non-null phase/service combination changes event semantics.
    pub port_call_phase_type_code: Option<String>,
    pub port_call_service_type_code: Option<String>,
    pub event_datetime: ParsedTimestamp,
    pub location_name: String,
    pub un_location_code: String,
    pub facility_code: Option<String>,
    pub facility_code_list_provider: Option<String>,
    pub vessel_imo_number: String,
    pub publisher_name: Option<String>,
    pub publisher_role: Option<String>,
    pub carrier_export_voyage_number: Option<String>,
    pub carrier_import_voyage_number: Option<String>,
    pub transport_call_sequence_number: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DcsaJitParseError {
    InvalidJson,
    MissingField(&'static str),
    WrongType(&'static str),
    EmptyField(&'static str),
    UnsupportedCode { field: &'static str, value: String },
    UnsupportedCombination(&'static str),
    UnsupportedPhaseOrService,
    ConflictingDuplicate { field: &'static str },
    InvalidTimestamp(TimestampParseError),
    InvalidIngestMetadata(&'static str),
    SourceRecordTooOld,
    SourceEventTooFarInFuture,
    IngestionTooFarInFuture,
    SourceEventAfterIngestion,
    EventTimeTooFarInFuture,
    InvalidValidityWindow,
    ObservationExpired,
    UnsupportedPcoProjection,
}

fn value_at<'a>(
    root: &'a serde_json::Value,
    path: &[&'static str],
) -> Option<&'a serde_json::Value> {
    let mut value = root;
    for key in path {
        value = value.get(*key)?;
    }
    Some(value)
}

fn required_string<'a>(
    root: &'a serde_json::Value,
    path: &[&'static str],
    label: &'static str,
) -> Result<&'a str, DcsaJitParseError> {
    let value = value_at(root, path).ok_or(DcsaJitParseError::MissingField(label))?;
    let text = value.as_str().ok_or(DcsaJitParseError::WrongType(label))?;
    if text.trim().is_empty() {
        return Err(DcsaJitParseError::EmptyField(label));
    }
    Ok(text)
}

fn optional_string<'a>(
    root: &'a serde_json::Value,
    path: &[&'static str],
    label: &'static str,
) -> Result<Option<&'a str>, DcsaJitParseError> {
    match value_at(root, path) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => {
            let text = value.as_str().ok_or(DcsaJitParseError::WrongType(label))?;
            if text.trim().is_empty() {
                return Err(DcsaJitParseError::EmptyField(label));
            }
            Ok(Some(text))
        }
    }
}

fn resolve_string_alias<'a>(
    root: &'a serde_json::Value,
    primary_path: &[&'static str],
    legacy_path: &[&'static str],
    label: &'static str,
) -> Result<&'a str, DcsaJitParseError> {
    let primary = optional_string(root, primary_path, label)?;
    let legacy = optional_string(root, legacy_path, label)?;
    match (primary, legacy) {
        (Some(a), Some(b)) if a != b => {
            Err(DcsaJitParseError::ConflictingDuplicate { field: label })
        }
        (Some(value), _) | (_, Some(value)) => Ok(value),
        (None, None) => Err(DcsaJitParseError::MissingField(label)),
    }
}

fn required_optional_code(
    root: &serde_json::Value,
    field: &'static str,
) -> Result<Option<String>, DcsaJitParseError> {
    optional_string(root, &[field], field).map(|value| value.map(str::to_owned))
}

/// Parse the reviewed DCSA JIT basic-timestamp subset:
/// EST/REQ/PLN/ACT × (ARRI/DEPA) × (BRTH/PBPL), with event-specific location
/// constraints and null/absent phase/service codes. Combinations outside this
/// subset are explicitly rejected pending a separate standards review.
pub fn parse_timestamp(
    payload: &str,
    context: DcsaIngestContext,
) -> Result<DcsaJitTimestamp, DcsaJitParseError> {
    if context.observation_id.trim().is_empty() {
        return Err(DcsaJitParseError::InvalidIngestMetadata("observation_id"));
    }
    if context.source_id.trim().is_empty() {
        return Err(DcsaJitParseError::InvalidIngestMetadata("source_id"));
    }
    if context.schema_version.trim().is_empty() {
        return Err(DcsaJitParseError::InvalidIngestMetadata("schema_version"));
    }

    let root: serde_json::Value =
        serde_json::from_str(payload).map_err(|_| DcsaJitParseError::InvalidJson)?;
    let event_classifier_code = EventClassifierCode::parse(required_string(
        &root,
        &["eventClassifierCode"],
        "eventClassifierCode",
    )?)?;
    let operations_event_type_code = OperationsEventTypeCode::parse(required_string(
        &root,
        &["operationsEventTypeCode"],
        "operationsEventTypeCode",
    )?)?;
    let facility_type_code = FacilityTypeCode::parse(required_string(
        &root,
        &["facilityTypeCode"],
        "facilityTypeCode",
    )?)?;

    if required_optional_code(&root, "portCallPhaseTypeCode")?.is_some()
        || required_optional_code(&root, "portCallServiceTypeCode")?.is_some()
    {
        return Err(DcsaJitParseError::UnsupportedPhaseOrService);
    }

    let location_name =
        required_string(
            &root,
            &["eventLocation", "locationName"],
            "eventLocation.locationName",
        )?
            .to_owned();
    let un_location_code = resolve_string_alias(
        &root,
        &["eventLocation", "UNLocationCode"],
        &["UNLocationCode"],
        "UNLocationCode",
    )?
    .to_owned();
    let facility_code = optional_string(
        &root,
        &["eventLocation", "facilityCode"],
        "eventLocation.facilityCode",
    )?
    .map(str::to_owned);
    let facility_code_list_provider = optional_string(
        &root,
        &["eventLocation", "facilityCodeListProvider"],
        "eventLocation.facilityCodeListProvider",
    )?
    .map(str::to_owned);
    let legacy_facility_code =
        optional_string(&root, &["facilitySMDGCode"], "facilitySMDGCode")?;
    if let (Some(modern), Some(legacy)) = (&facility_code, legacy_facility_code) {
        if modern.as_str() != legacy {
            return Err(DcsaJitParseError::ConflictingDuplicate {
                field: "facilityCode/facilitySMDGCode",
            });
        }
    }

    match &facility_type_code {
        FacilityTypeCode::Berth => {
            if facility_code.is_none() || facility_code_list_provider.is_none() {
                return Err(DcsaJitParseError::UnsupportedCombination(
                    "BRTH requires event-location facility code and code-list provider",
                ));
            }
        }
        FacilityTypeCode::PilotBoardingPlace => {
            if facility_code.is_some()
                || facility_code_list_provider.is_some()
                || legacy_facility_code.is_some()
            {
                return Err(DcsaJitParseError::UnsupportedCombination(
                    "PBPL must not carry terminal facility-code fields",
                ));
            }
            if operations_event_type_code != OperationsEventTypeCode::Arrival {
                return Err(DcsaJitParseError::UnsupportedCombination(
                    "PBPL departure is outside the v0.1 profile",
                ));
            }
        }
    }

    let vessel_imo_number = resolve_string_alias(
        &root,
        &["vessel", "vesselIMONumber"],
        &["vesselIMONumber"],
        "vesselIMONumber",
    )?
    .to_owned();
    let event_datetime = parse_rfc3339_timestamp(required_string(
        &root,
        &["eventDateTime"],
        "eventDateTime",
    )?)
    .map_err(DcsaJitParseError::InvalidTimestamp)?;

    let publisher_name = optional_string(
        &root,
        &["publisher", "partyName"],
        "publisher.partyName",
    )?
    .map(str::to_owned);
    let publisher_role = optional_string(&root, &["publisherRole"], "publisherRole")?
        .map(str::to_owned);
    let carrier_export_voyage_number =
        optional_string(&root, &["carrierExportVoyageNumber"], "carrierExportVoyageNumber")?
            .map(str::to_owned);
    let carrier_import_voyage_number =
        optional_string(&root, &["carrierImportVoyageNumber"], "carrierImportVoyageNumber")?
            .map(str::to_owned);
    let transport_call_sequence_number =
        match value_at(&root, &["transportCallSequenceNumber"]) {
        None | Some(serde_json::Value::Null) => None,
        Some(value) => Some(
            value
                .as_u64()
                .and_then(|number| u32::try_from(number).ok())
                .ok_or(DcsaJitParseError::WrongType("transportCallSequenceNumber"))?,
        ),
    };

    Ok(DcsaJitTimestamp {
        profile: DCSA_JIT_PROFILE.to_owned(),
        context,
        event_classifier_code,
        operations_event_type_code,
        facility_type_code,
        port_call_phase_type_code: None,
        port_call_service_type_code: None,
        event_datetime,
        location_name,
        un_location_code,
        facility_code,
        facility_code_list_provider,
        vessel_imo_number,
        publisher_name,
        publisher_role,
        carrier_export_voyage_number,
        carrier_import_voyage_number,
        transport_call_sequence_number,
    })
}

/// Validate provenance and temporal metadata using the same externally supplied
/// policy as the PCO profile. Event time is still distinct from source update
/// and ingest time.
pub fn validate_timestamp(
    timestamp: &DcsaJitTimestamp,
    policy: &ValidationPolicy,
) -> Result<(), DcsaJitParseError> {
    let context = &timestamp.context;
    let skew =
        i128::from(policy.allowed_future_clock_skew_seconds.max(0)) * NANOS_PER_SECOND;
    let latest_acceptable = policy.now_utc_nanos.saturating_add(skew);

    if context.source_event_time_utc_nanos > latest_acceptable {
        return Err(DcsaJitParseError::SourceEventTooFarInFuture);
    }
    if context.ingested_at_utc_nanos > latest_acceptable {
        return Err(DcsaJitParseError::IngestionTooFarInFuture);
    }
    if context.source_event_time_utc_nanos
        > context.ingested_at_utc_nanos.saturating_add(skew)
    {
        return Err(DcsaJitParseError::SourceEventAfterIngestion);
    }
    let max_age = i128::from(policy.max_source_age_seconds.max(0)) * NANOS_PER_SECOND;
    if policy.now_utc_nanos >= context.source_event_time_utc_nanos
        && policy.now_utc_nanos - context.source_event_time_utc_nanos > max_age
    {
        return Err(DcsaJitParseError::SourceRecordTooOld);
    }
    if timestamp.event_classifier_code == EventClassifierCode::Actual
        && timestamp.event_datetime.utc_unix_nanos()
            > latest_acceptable
    {
        return Err(DcsaJitParseError::EventTimeTooFarInFuture);
    }
    if let Some(valid_until) = context.valid_until_utc_nanos {
        if valid_until < context.source_event_time_utc_nanos {
            return Err(DcsaJitParseError::InvalidValidityWindow);
        }
        if valid_until <= policy.now_utc_nanos {
            return Err(DcsaJitParseError::ObservationExpired);
        }
    }
    Ok(())
}

/// Explain which DCSA concepts could project into the PCO floor. This is a
/// semantic mapping hint only: canonical identity and provenance mapping must
/// be supplied separately before creating a PCO observation.
pub fn pco_field_candidate(
    timestamp: &DcsaJitTimestamp,
) -> Result<super::PcoField, DcsaJitParseError> {
    match (
        &timestamp.event_classifier_code,
        &timestamp.operations_event_type_code,
        &timestamp.facility_type_code,
    ) {
        (
            EventClassifierCode::Planned,
            OperationsEventTypeCode::Arrival,
            FacilityTypeCode::PilotBoardingPlace,
        ) => Ok(super::PcoField::ArrivalAtPilotBoardingPlace),
        (
            EventClassifierCode::Planned,
            OperationsEventTypeCode::Departure,
            FacilityTypeCode::Berth,
        ) => Ok(super::PcoField::DepartureFromBerth),
        _ => Err(DcsaJitParseError::UnsupportedPcoProjection),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(id: &str) -> DcsaIngestContext {
        let now = super::parse_rfc3339_timestamp("2026-10-18T08:00:00Z")
            .expect("frozen test time parses")
            .utc_unix_nanos();
        DcsaIngestContext {
            observation_id: id.to_owned(),
            source_id: "DCSA-JIT-SYNTHETIC".to_owned(),
            source_event_time_utc_nanos: now - 60 * NANOS_PER_SECOND,
            ingested_at_utc_nanos: now - 30 * NANOS_PER_SECOND,
            valid_until_utc_nanos: Some(now + 3600 * NANOS_PER_SECOND),
            schema_version: "DCSA-JIT-1.2.0-beta1-test-profile".to_owned(),
        }
    }

    fn policy() -> ValidationPolicy {
        let now = super::parse_rfc3339_timestamp("2026-10-18T08:00:00Z")
            .expect("frozen test time parses")
            .utc_unix_nanos();
        ValidationPolicy::new(now)
    }

    #[test]
    fn preserves_eta_berth_without_calling_it_pco_pilot_boarding_place_arrival() {
        let payload = r#"{
            "publisher": {"partyName": "Synthetic Terminal"},
            "publisherRole": "TR",
            "eventClassifierCode": "EST",
            "operationsEventTypeCode": "ARRI",
            "facilityTypeCode": "BRTH",
            "portCallPhaseTypeCode": null,
            "portCallServiceTypeCode": null,
            "eventDateTime": "2026-10-18T12:10:00.000+01:00",
            "eventLocation": {
                "locationName": "SYNTHETIC BERTH A",
                "UNLocationCode": "ZZSYN",
                "facilityCode": "SYN1",
                "facilityCodeListProvider": "SMDG"
            },
            "UNLocationCode": "ZZSYN",
            "facilitySMDGCode": "SYN1",
            "vessel": {"vesselIMONumber": "9990001"},
            "vesselIMONumber": "9990001",
            "transportCallSequenceNumber": 1
        }"#;
        let parsed = parse_timestamp(payload, context("DCSA-ETA-01")).unwrap();
        assert_eq!(parsed.time_meaning(), TimeMeaning::Estimated);
        assert_eq!(parsed.facility_type_code, FacilityTypeCode::Berth);
        assert_eq!(parsed.location_name, "SYNTHETIC BERTH A");
        assert_eq!(parsed.un_location_code, "ZZSYN");
        assert_eq!(parsed.vessel_imo_number, "9990001");
        assert_eq!(
            pco_field_candidate(&parsed),
            Err(DcsaJitParseError::UnsupportedPcoProjection)
        );
        assert!(validate_timestamp(&parsed, &policy()).is_ok());
    }

    #[test]
    fn maps_only_planned_pilot_boarding_arrival_and_planned_berth_departure() {
        let pbp = r#"{
            "eventClassifierCode": "PLN",
            "operationsEventTypeCode": "ARRI",
            "facilityTypeCode": "PBPL",
            "portCallPhaseTypeCode": null,
            "portCallServiceTypeCode": null,
            "eventDateTime": "2026-10-18T12:10:00Z",
            "eventLocation": {"locationName": "SYNTHETIC PBP", "UNLocationCode": "ZZSYN"},
            "vessel": {"vesselIMONumber": "9990001"}
        }"#;
        let arrival = parse_timestamp(pbp, context("DCSA-PTA-PBP")).unwrap();
        assert_eq!(
            pco_field_candidate(&arrival),
            Ok(super::super::PcoField::ArrivalAtPilotBoardingPlace)
        );

        let departure = r#"{
            "eventClassifierCode": "PLN",
            "operationsEventTypeCode": "DEPA",
            "facilityTypeCode": "BRTH",
            "eventDateTime": "2026-10-19T17:20:50.52Z",
            "eventLocation": {
                "locationName": "SYNTHETIC BERTH A",
                "UNLocationCode": "ZZSYN",
                "facilityCode": "SYN1",
                "facilityCodeListProvider": "SMDG"
            },
            "vesselIMONumber": "9990001"
        }"#;
        let departure = parse_timestamp(departure, context("DCSA-PTD-BERTH")).unwrap();
        assert_eq!(
            pco_field_candidate(&departure),
            Ok(super::super::PcoField::DepartureFromBerth)
        );
    }

    #[test]
    fn rejects_pbp_departure_and_terminal_fields_on_pbpl() {
        let payload = r#"{
            "eventClassifierCode": "PLN",
            "operationsEventTypeCode": "DEPA",
            "facilityTypeCode": "PBPL",
            "eventDateTime": "2026-10-18T12:10:00Z",
            "eventLocation": {
                "locationName": "SYNTHETIC PBP",
                "UNLocationCode": "ZZSYN",
                "facilityCode": "SYN1",
                "facilityCodeListProvider": "SMDG"
            },
            "vesselIMONumber": "9990001"
        }"#;
        assert_eq!(
            parse_timestamp(payload, context("DCSA-UNSUPPORTED")),
            Err(DcsaJitParseError::UnsupportedCombination(
                "PBPL must not carry terminal facility-code fields"
            ))
        );
    }

    #[test]
    fn rejects_conflicting_duplicate_vessel_and_port_identifiers() {
        let vessel_conflict = r#"{
            "eventClassifierCode": "REQ",
            "operationsEventTypeCode": "ARRI",
            "facilityTypeCode": "BRTH",
            "eventDateTime": "2026-10-18T12:10:00Z",
            "eventLocation": {
                "locationName": "SYNTHETIC BERTH A",
                "UNLocationCode": "ZZSYN",
                "facilityCode": "SYN1",
                "facilityCodeListProvider": "SMDG"
            },
            "UNLocationCode": "ZZSYN",
            "facilitySMDGCode": "SYN1",
            "vessel": {"vesselIMONumber": "9990001"},
            "vesselIMONumber": "9990002"
        }"#;
        assert_eq!(
            parse_timestamp(vessel_conflict, context("DCSA-DUP-VESSEL")),
            Err(DcsaJitParseError::ConflictingDuplicate {
                field: "vesselIMONumber"
            })
        );

        let port_conflict = vessel_conflict.replace(
            "\"vesselIMONumber\": \"9990002\"",
            "\"vesselIMONumber\": \"9990001\"",
        ).replace(
            "\"UNLocationCode\": \"ZZSYN\"\n        },",
            "\"UNLocationCode\": \"ZZOTHER\"\n        },",
        );
        assert!(matches!(
            parse_timestamp(&port_conflict, context("DCSA-DUP-PORT")),
            Err(DcsaJitParseError::ConflictingDuplicate {
                field: "UNLocationCode"
            })
        ));
    }

    #[test]
    fn rejects_unknown_codes_and_non_null_phase_or_service() {
        let payload = r#"{
            "eventClassifierCode": "MAYBE",
            "operationsEventTypeCode": "ARRI",
            "facilityTypeCode": "BRTH",
            "eventDateTime": "2026-10-18T12:10:00Z",
            "eventLocation": {
                "locationName": "SYNTHETIC BERTH A",
                "UNLocationCode": "ZZSYN",
                "facilityCode": "SYN1",
                "facilityCodeListProvider": "SMDG"
            },
            "vesselIMONumber": "9990001"
        }"#;
        assert!(matches!(
            parse_timestamp(payload, context("DCSA-CODE")),
            Err(DcsaJitParseError::UnsupportedCode {
                field: "eventClassifierCode",
                ..
            })
        ));

        let non_null_phase = payload.replace("\"MAYBE\"", "\"PLN\"").replace(
            "\"eventClassifierCode\": \"PLN\"",
            "\"eventClassifierCode\": \"PLN\", \"portCallPhaseTypeCode\": \"INBD\"",
        );
        assert_eq!(
            parse_timestamp(&non_null_phase, context("DCSA-PHASE")),
            Err(DcsaJitParseError::UnsupportedPhaseOrService)
        );
    }

    #[test]
    fn preserves_unknown_local_offset_and_rejects_timezone_less_event_time() {
        let payload = r#"{
            "eventClassifierCode": "PLN",
            "operationsEventTypeCode": "ARRI",
            "facilityTypeCode": "PBPL",
            "eventDateTime": "2026-10-18T12:10:00-00:00",
            "eventLocation": {"locationName": "SYNTHETIC PBP", "UNLocationCode": "ZZSYN"},
            "vesselIMONumber": "9990001"
        }"#;
        let parsed = parse_timestamp(payload, context("DCSA-UNKNOWN-OFFSET")).unwrap();
        assert_eq!(
            parsed.event_datetime.offset_semantics(),
            &TimestampOffsetSemantics::UnknownLocalOffset
        );
        let invalid = payload.replace("2026-10-18T12:10:00-00:00", "2026-10-18T12:10:00");
        assert!(matches!(
            parse_timestamp(&invalid, context("DCSA-NO-ZONE")),
            Err(DcsaJitParseError::InvalidTimestamp(
                TimestampParseError::TooShort
            ))
        ));
    }

    #[test]
    fn classifier_changes_meaning_without_changing_location_semantics() {
        let base = r#"{
            "eventClassifierCode": "EST",
            "operationsEventTypeCode": "ARRI",
            "facilityTypeCode": "PBPL",
            "eventDateTime": "2026-10-18T12:10:00Z",
            "eventLocation": {"locationName": "SYNTHETIC PBP", "UNLocationCode": "ZZSYN"},
            "vesselIMONumber": "9990001"
        }"#;
        let estimated = parse_timestamp(base, context("DCSA-EST")).unwrap();
        let requested_payload = base.replace("\"EST\"", "\"REQ\"");
        let requested = parse_timestamp(&requested_payload, context("DCSA-REQ")).unwrap();
        let planned_payload = base.replace("\"EST\"", "\"PLN\"");
        let planned = parse_timestamp(&planned_payload, context("DCSA-PLN")).unwrap();
        assert_eq!(estimated.facility_type_code, requested.facility_type_code);
        assert_eq!(requested.facility_type_code, planned.facility_type_code);
        assert_eq!(estimated.time_meaning(), TimeMeaning::Estimated);
        assert_eq!(requested.time_meaning(), TimeMeaning::Requested);
        assert_eq!(planned.time_meaning(), TimeMeaning::Planned);
    }
}
