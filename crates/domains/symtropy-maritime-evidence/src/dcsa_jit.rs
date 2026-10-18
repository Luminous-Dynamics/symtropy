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
    NANOS_PER_SECOND, ParsedTimestamp, TimeMeaning, TimestampParseError, ValidationPolicy,
    parse_rfc3339_timestamp,
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
pub fn parse_timestamp_message(
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

    let location_name = required_string(
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
    let legacy_facility_code = optional_string(&root, &["facilitySMDGCode"], "facilitySMDGCode")?;
    if let (Some(modern), Some(legacy)) = (&facility_code, legacy_facility_code)
        && modern.as_str() != legacy
    {
        return Err(DcsaJitParseError::ConflictingDuplicate {
            field: "facilityCode/facilitySMDGCode",
        });
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
    let event_datetime =
        parse_rfc3339_timestamp(required_string(&root, &["eventDateTime"], "eventDateTime")?)
            .map_err(DcsaJitParseError::InvalidTimestamp)?;

    let publisher_name =
        optional_string(&root, &["publisher", "partyName"], "publisher.partyName")?
            .map(str::to_owned);
    let publisher_role =
        optional_string(&root, &["publisherRole"], "publisherRole")?.map(str::to_owned);
    let carrier_export_voyage_number = optional_string(
        &root,
        &["carrierExportVoyageNumber"],
        "carrierExportVoyageNumber",
    )?
    .map(str::to_owned);
    let carrier_import_voyage_number = optional_string(
        &root,
        &["carrierImportVoyageNumber"],
        "carrierImportVoyageNumber",
    )?
    .map(str::to_owned);
    let transport_call_sequence_number = match value_at(&root, &["transportCallSequenceNumber"]) {
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
    let skew = i128::from(policy.allowed_future_clock_skew_seconds.max(0)) * NANOS_PER_SECOND;
    let latest_acceptable = policy.now_utc_nanos.saturating_add(skew);

    if context.source_event_time_utc_nanos > latest_acceptable {
        return Err(DcsaJitParseError::SourceEventTooFarInFuture);
    }
    if context.ingested_at_utc_nanos > latest_acceptable {
        return Err(DcsaJitParseError::IngestionTooFarInFuture);
    }
    if context.source_event_time_utc_nanos > context.ingested_at_utc_nanos.saturating_add(skew) {
        return Err(DcsaJitParseError::SourceEventAfterIngestion);
    }
    let max_age = i128::from(policy.max_source_age_seconds.max(0)) * NANOS_PER_SECOND;
    if policy.now_utc_nanos >= context.source_event_time_utc_nanos
        && policy.now_utc_nanos - context.source_event_time_utc_nanos > max_age
    {
        return Err(DcsaJitParseError::SourceRecordTooOld);
    }
    if matches!(
        &timestamp.event_classifier_code,
        EventClassifierCode::Actual
    ) && timestamp.event_datetime.utc_unix_nanos() > latest_acceptable
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
    use super::super::TimestampOffsetSemantics;
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
        let payload = include_str!("../fixtures/dcsa-jit-eta-berth.json");
        let parsed = parse_timestamp_message(payload, context("DCSA-ETA-01")).unwrap();
        assert_eq!(
            parsed.event_classifier_code.time_meaning(),
            TimeMeaning::Estimated
        );
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
        let pbp = include_str!("../fixtures/dcsa-jit-pta-pbp.json");
        let arrival = parse_timestamp_message(pbp, context("DCSA-PTA-PBP")).unwrap();
        assert_eq!(
            pco_field_candidate(&arrival),
            Ok(super::super::PcoField::ArrivalAtPilotBoardingPlace)
        );

        let departure = include_str!("../fixtures/dcsa-jit-ptd-berth.json");
        let departure = parse_timestamp_message(departure, context("DCSA-PTD-BERTH")).unwrap();
        assert_eq!(
            pco_field_candidate(&departure),
            Ok(super::super::PcoField::DepartureFromBerth)
        );
    }

    #[test]
    fn rejects_pbp_departure_and_terminal_fields_on_pbpl() {
        let payload = include_str!("../fixtures/dcsa-jit-invalid-pbpl-terminal.json");
        assert_eq!(
            parse_timestamp_message(payload, context("DCSA-UNSUPPORTED")),
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
            parse_timestamp_message(vessel_conflict, context("DCSA-DUP-VESSEL")),
            Err(DcsaJitParseError::ConflictingDuplicate {
                field: "vesselIMONumber"
            })
        );

        let mut port_conflict_value: serde_json::Value =
            serde_json::from_str(vessel_conflict).unwrap();
        port_conflict_value["vesselIMONumber"] = serde_json::Value::String("9990001".to_owned());
        port_conflict_value["UNLocationCode"] = serde_json::Value::String("ZZOTHER".to_owned());
        let port_conflict = port_conflict_value.to_string();

        assert!(matches!(
            parse_timestamp_message(&port_conflict, context("DCSA-DUP-PORT")),
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
            parse_timestamp_message(payload, context("DCSA-CODE")),
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
            parse_timestamp_message(&non_null_phase, context("DCSA-PHASE")),
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
        let parsed = parse_timestamp_message(payload, context("DCSA-UNKNOWN-OFFSET")).unwrap();
        assert_eq!(
            parsed.event_datetime.offset_semantics(),
            &TimestampOffsetSemantics::UnknownLocalOffset
        );
        let zulu_payload = payload.replace("2026-10-18T12:10:00-00:00", "2026-10-18T12:10:00Z");
        let zulu =
            parse_timestamp_message(&zulu_payload, context("DCSA-Z-UNKNOWN-OFFSET")).unwrap();
        assert_eq!(
            zulu.event_datetime.offset_semantics(),
            &TimestampOffsetSemantics::UnknownLocalOffset
        );
        assert_eq!(
            parsed.event_datetime.utc_unix_nanos(),
            zulu.event_datetime.utc_unix_nanos()
        );
        let invalid = payload.replace("2026-10-18T12:10:00-00:00", "2026-10-18T12:10:00");
        assert!(matches!(
            parse_timestamp_message(&invalid, context("DCSA-NO-ZONE")),
            Err(DcsaJitParseError::InvalidTimestamp(
                TimestampParseError::TooShort
            ))
        ));
    }

    #[test]
    fn exhaustively_classifies_v0_1_classifier_operation_facility_matrix() {
        let classifiers = ["EST", "REQ", "PLN", "ACT"];
        let berth_fixture = include_str!("../fixtures/dcsa-jit-eta-berth.json");
        let pbp_fixture = include_str!("../fixtures/dcsa-jit-pta-pbp.json");
        let mut accepted = 0usize;
        let mut rejected = 0usize;

        for classifier in classifiers {
            let berth_base = berth_fixture.replace(
                "\"eventClassifierCode\": \"EST\"",
                &format!("\"eventClassifierCode\": \"{classifier}\""),
            );
            for operation in ["ARRI", "DEPA"] {
                let payload = berth_base.replace(
                    "\"operationsEventTypeCode\": \"ARRI\"",
                    &format!("\"operationsEventTypeCode\": \"{operation}\""),
                );
                let parsed = parse_timestamp_message(
                    &payload,
                    context(&format!("DCSA-MATRIX-{classifier}-{operation}-BRTH")),
                )
                .unwrap();
                assert_eq!(parsed.facility_type_code, FacilityTypeCode::Berth);
                assert_eq!(
                    parsed.event_classifier_code.time_meaning(),
                    match classifier {
                        "EST" => TimeMeaning::Estimated,
                        "REQ" => TimeMeaning::Requested,
                        "PLN" => TimeMeaning::Planned,
                        "ACT" => TimeMeaning::Actual,
                        _ => unreachable!("classifier fixture is frozen"),
                    }
                );

                let candidate = pco_field_candidate(&parsed);
                if classifier == "PLN" && operation == "DEPA" {
                    assert_eq!(candidate, Ok(super::super::PcoField::DepartureFromBerth));
                } else {
                    assert_eq!(candidate, Err(DcsaJitParseError::UnsupportedPcoProjection));
                }
                accepted += 1;
            }

            let pbp_payload = pbp_fixture.replace(
                "\"eventClassifierCode\": \"PLN\"",
                &format!("\"eventClassifierCode\": \"{classifier}\""),
            );
            let pbp = parse_timestamp_message(
                &pbp_payload,
                context(&format!("DCSA-MATRIX-{classifier}-ARRI-PBPL")),
            )
            .unwrap();
            assert_eq!(pbp.facility_type_code, FacilityTypeCode::PilotBoardingPlace);
            assert_eq!(
                pbp.operations_event_type_code,
                OperationsEventTypeCode::Arrival
            );
            let candidate = pco_field_candidate(&pbp);
            if classifier == "PLN" {
                assert_eq!(
                    candidate,
                    Ok(super::super::PcoField::ArrivalAtPilotBoardingPlace)
                );
            } else {
                assert_eq!(candidate, Err(DcsaJitParseError::UnsupportedPcoProjection));
            }
            accepted += 1;

            let pbp_departure = pbp_payload.replace(
                "\"operationsEventTypeCode\": \"ARRI\"",
                "\"operationsEventTypeCode\": \"DEPA\"",
            );
            assert_eq!(
                parse_timestamp_message(
                    &pbp_departure,
                    context(&format!("DCSA-MATRIX-{classifier}-DEPA-PBPL")),
                ),
                Err(DcsaJitParseError::UnsupportedCombination(
                    "PBPL departure is outside the v0.1 profile"
                ))
            );
            rejected += 1;
        }

        assert_eq!(accepted, 12);
        assert_eq!(rejected, 4);
    }

    #[test]
    fn future_planned_timestamp_is_allowed_but_future_actual_is_not() {
        let planned_payload = include_str!("../fixtures/dcsa-jit-pta-pbp.json");
        let planned = parse_timestamp_message(planned_payload, context("DCSA-FUTURE-PLN")).unwrap();
        assert!(validate_timestamp(&planned, &policy()).is_ok());

        let actual_payload = planned_payload.replace(
            "\"eventClassifierCode\": \"PLN\"",
            "\"eventClassifierCode\": \"ACT\"",
        );
        let actual = parse_timestamp_message(&actual_payload, context("DCSA-FUTURE-ACT")).unwrap();
        assert_eq!(
            validate_timestamp(&actual, &policy()),
            Err(DcsaJitParseError::EventTimeTooFarInFuture)
        );
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
        let estimated = parse_timestamp_message(base, context("DCSA-EST")).unwrap();
        let requested_payload = base.replace("\"EST\"", "\"REQ\"");
        let requested = parse_timestamp_message(&requested_payload, context("DCSA-REQ")).unwrap();
        let planned_payload = base.replace("\"EST\"", "\"PLN\"");
        let planned = parse_timestamp_message(&planned_payload, context("DCSA-PLN")).unwrap();
        assert_eq!(estimated.facility_type_code, requested.facility_type_code);
        assert_eq!(requested.facility_type_code, planned.facility_type_code);
        assert_eq!(
            estimated.event_classifier_code.time_meaning(),
            TimeMeaning::Estimated
        );
        assert_eq!(
            requested.event_classifier_code.time_meaning(),
            TimeMeaning::Requested
        );
        assert_eq!(
            planned.event_classifier_code.time_meaning(),
            TimeMeaning::Planned
        );
    }
}
