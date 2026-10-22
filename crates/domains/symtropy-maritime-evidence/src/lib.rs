// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
#![forbid(unsafe_code)]

//! Typed and fail-closed handling of port-call observations.
//!
//! This crate is an evidence boundary, not a port authority or PCO API client.
//! It preserves source semantics, parses a strict RFC 3339 timestamp subset,
//! validates freshness/provenance metadata, and reports competing values
//! explicitly. A conflict is never resolved by "last write wins".

use std::collections::{BTreeMap, BTreeSet};

pub const NANOS_PER_SECOND: i128 = 1_000_000_000;
pub const PCO_GUIDE_VERSION: &str = "IAPH-IHMA-PCO-Guide-1.0";
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimestampOffsetSemantics {
    /// Timestamp provides an explicit numeric offset. A +00:00 offset expresses
    /// UTC as the preferred reference point.
    ExplicitOffset,
    /// RFC 9557 treats Z as an unknown local offset; -00:00 carries the same
    /// semantics. The original lexeme remains preserved separately.
    UnknownLocalOffset,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PcoField {
    TerminalIdentifier,
    BerthIdentifier,
    ArrivalAtPilotBoardingPlace,
    DepartureFromBerth,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeMeaning {
    Planned,
    Requested,
    Estimated,
    Forecast,
    Actual,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParsedTimestamp {
    /// Verbatim source representation. Never overwritten by normalization.
    source_text: String,
    /// Nanoseconds since Unix epoch, normalized to UTC.
    utc_unix_nanos: i128,
    /// Numerical offset used to normalize the instant, in minutes east of UTC.
    source_offset_minutes: i16,
    /// Preserve offset knowledge semantics per RFC 9557. The original source
    /// text preserves the exact notation, including Z versus -00:00.
    offset_semantics: TimestampOffsetSemantics,
    /// Number of fractional-second digits present in the source (0..=9).
    fractional_digits: u8,
}

impl ParsedTimestamp {
    pub fn source_text(&self) -> &str {
        &self.source_text
    }

    pub fn utc_unix_nanos(&self) -> i128 {
        self.utc_unix_nanos
    }

    pub fn source_offset_minutes(&self) -> i16 {
        self.source_offset_minutes
    }

    pub fn offset_semantics(&self) -> &TimestampOffsetSemantics {
        &self.offset_semantics
    }

    pub fn fractional_digits(&self) -> u8 {
        self.fractional_digits
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservedValue {
    Identifier(String),
    Timestamp {
        meaning: TimeMeaning,
        value: ParsedTimestamp,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PcoObservation {
    pub observation_id: String,
    /// Raw PCO source value from shipReportingSystemEntryPortCode.
    pub port_id: String,
    /// Raw PCO source value from shipStayReferenceNumber.
    pub port_call_id: String,
    /// Raw PCO source value from shipIMONumber; not normalized here.
    pub ship_imo_number: String,
    pub field: PcoField,
    /// For time events: source location text such as pilotBoardingPlaceName or
    /// berthName. It remains display/context data, not a canonical location ID.
    pub location_context: Option<String>,
    /// Standard code or source-system field name; retained verbatim.
    pub source_field_code: String,
    pub value: ObservedValue,
    pub source_id: String,
    /// When the source says this record was authored/updated, not the time
    /// described by a future planned-arrival/departure value.
    pub source_event_time_utc_nanos: i128,
    pub ingested_at_utc_nanos: i128,
    pub valid_until_utc_nanos: Option<i128>,
    pub schema_version: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationPolicy {
    pub now_utc_nanos: i128,
    pub max_source_age_seconds: i64,
    pub allowed_future_clock_skew_seconds: i64,
    /// Explicit master-data allowlists. Empty registries fail closed for
    /// terminal/berth identifiers.
    pub known_terminal_ids: BTreeSet<String>,
    pub known_berth_ids: BTreeSet<String>,
}

impl ValidationPolicy {
    pub fn new(now_utc_nanos: i128) -> Self {
        Self {
            now_utc_nanos,
            max_source_age_seconds: 6 * 60 * 60,
            allowed_future_clock_skew_seconds: 120,
            known_terminal_ids: BTreeSet::new(),
            known_berth_ids: BTreeSet::new(),
        }
    }
}

/// Metadata supplied by the ingest adapter; it is not inferred from the PCO
/// event time. This keeps source update time distinct from planned arrival or
/// departure time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestContext {
    pub observation_id: String,
    pub source_id: String,
    pub source_event_time_utc_nanos: i128,
    pub ingested_at_utc_nanos: i128,
    pub valid_until_utc_nanos: Option<i128>,
    pub schema_version: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PcoPayloadParseError {
    InvalidJson,
    MissingField(&'static str),
    WrongType(&'static str),
    EmptyLocationContext,
    InvalidTimestamp(TimestampParseError),
}

fn required_json_string<'a>(
    root: &'a serde_json::Value,
    path: &[&'static str],
    label: &'static str,
) -> Result<&'a str, PcoPayloadParseError> {
    let mut value = root;
    for key in path {
        value = value
            .get(*key)
            .ok_or(PcoPayloadParseError::MissingField(label))?;
    }
    value.as_str().ok_or(PcoPayloadParseError::WrongType(label))
}

/// Translate the PCO Guide's planned-arrival API response shape into a typed
/// observation. Caller-owned ingest metadata remains separate from event time.
pub fn parse_pco_arrival_response(
    payload: &str,
    context: IngestContext,
) -> Result<PcoObservation, PcoPayloadParseError> {
    let root: serde_json::Value =
        serde_json::from_str(payload).map_err(|_| PcoPayloadParseError::InvalidJson)?;
    let ship_imo_number =
        required_json_string(&root, &["shipIMONumber"], "shipIMONumber")?.to_owned();
    let port_call_id = required_json_string(
        &root,
        &["shipStayReferenceNumber"],
        "shipStayReferenceNumber",
    )?
    .to_owned();
    let port_id = required_json_string(
        &root,
        &["shipReportingSystemEntryPortCode"],
        "shipReportingSystemEntryPortCode",
    )?
    .to_owned();
    let location = required_json_string(
        &root,
        &["dateTimeArrivalPlanned", "pilotBoardingPlaceName"],
        "dateTimeArrivalPlanned.pilotBoardingPlaceName",
    )?
    .to_owned();
    if location.trim().is_empty() {
        return Err(PcoPayloadParseError::EmptyLocationContext);
    }
    let timestamp = required_json_string(
        &root,
        &["dateTimeArrivalPlanned", "plannedTimeOfArrival"],
        "dateTimeArrivalPlanned.plannedTimeOfArrival",
    )?;
    let value =
        parse_rfc3339_timestamp(timestamp).map_err(PcoPayloadParseError::InvalidTimestamp)?;

    Ok(PcoObservation {
        observation_id: context.observation_id,
        port_id,
        port_call_id,
        ship_imo_number,
        field: PcoField::ArrivalAtPilotBoardingPlace,
        location_context: Some(location),
        source_field_code: "dateTimeArrivalPlanned.plannedTimeOfArrival".to_owned(),
        value: ObservedValue::Timestamp {
            meaning: TimeMeaning::Planned,
            value,
        },
        source_id: context.source_id,
        source_event_time_utc_nanos: context.source_event_time_utc_nanos,
        ingested_at_utc_nanos: context.ingested_at_utc_nanos,
        valid_until_utc_nanos: context.valid_until_utc_nanos,
        schema_version: context.schema_version,
    })
}

/// Translate the PCO Guide's planned-departure API response shape.
pub fn parse_pco_departure_response(
    payload: &str,
    context: IngestContext,
) -> Result<PcoObservation, PcoPayloadParseError> {
    let root: serde_json::Value =
        serde_json::from_str(payload).map_err(|_| PcoPayloadParseError::InvalidJson)?;
    let ship_imo_number =
        required_json_string(&root, &["shipIMONumber"], "shipIMONumber")?.to_owned();
    let port_call_id = required_json_string(
        &root,
        &["shipStayReferenceNumber"],
        "shipStayReferenceNumber",
    )?
    .to_owned();
    let port_id = required_json_string(
        &root,
        &["shipReportingSystemEntryPortCode"],
        "shipReportingSystemEntryPortCode",
    )?
    .to_owned();
    let location = required_json_string(
        &root,
        &["dateTimeDeparturePlanned", "berthName"],
        "dateTimeDeparturePlanned.berthName",
    )?
    .to_owned();
    if location.trim().is_empty() {
        return Err(PcoPayloadParseError::EmptyLocationContext);
    }
    let timestamp = required_json_string(
        &root,
        &["dateTimeDeparturePlanned", "plannedTimeOfDeparture"],
        "dateTimeDeparturePlanned.plannedTimeOfDeparture",
    )?;
    let value =
        parse_rfc3339_timestamp(timestamp).map_err(PcoPayloadParseError::InvalidTimestamp)?;

    Ok(PcoObservation {
        observation_id: context.observation_id,
        port_id,
        port_call_id,
        ship_imo_number,
        field: PcoField::DepartureFromBerth,
        location_context: Some(location),
        source_field_code: "dateTimeDeparturePlanned.plannedTimeOfDeparture".to_owned(),
        value: ObservedValue::Timestamp {
            meaning: TimeMeaning::Planned,
            value,
        },
        source_id: context.source_id,
        source_event_time_utc_nanos: context.source_event_time_utc_nanos,
        ingested_at_utc_nanos: context.ingested_at_utc_nanos,
        valid_until_utc_nanos: context.valid_until_utc_nanos,
        schema_version: context.schema_version,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimestampParseError {
    NonAscii,
    TooShort,
    InvalidDateTimeSeparators,
    InvalidDigits,
    InvalidCalendarDate,
    InvalidClockTime,
    InvalidTimezone,
    EmptyFraction,
    FractionTooPrecise,
    TrailingCharacters,
    LeapSecondUnsupported,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RejectionReason {
    MissingObservationId,
    DuplicateObservationId,
    MissingPortId,
    MissingPortCallId,
    MissingShipImoNumber,
    MissingLocationContext,
    PortCallVesselIdentityConflict,
    MissingSourceFieldCode,
    MissingSourceId,
    MissingSchemaVersion,
    EmptyIdentifier,
    InvalidValueForField,
    UnknownTerminal(String),
    UnknownBerth(String),
    SourceEventTooFarInFuture,
    IngestionTooFarInFuture,
    SourceEventAfterIngestion,
    SourceRecordTooOld,
    ActualTimeTooFarInFuture,
    InvalidValidityWindow,
    ObservationExpired,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RejectedObservation {
    pub observation_id: String,
    pub reason: RejectionReason,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObservationKey {
    pub port_id: String,
    pub port_call_id: String,
    pub ship_imo_number: String,
    pub field: PcoField,
    /// A planned arrival and an actual arrival are different semantic keys.
    pub time_meaning: Option<TimeMeaning>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationResolution {
    /// Candidate observations agree on the normalized semantic value.
    Consistent {
        key: ObservationKey,
        candidates: Vec<PcoObservation>,
    },
    /// Sources disagree on the same port call, field and time meaning.
    /// No candidate is selected automatically.
    Conflict {
        key: ObservationKey,
        candidates: Vec<PcoObservation>,
    },
}

impl ObservationResolution {
    pub fn key(&self) -> &ObservationKey {
        match self {
            Self::Consistent { key, .. } | Self::Conflict { key, .. } => key,
        }
    }

    pub fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict { .. })
    }

    /// Return a value only when the semantic group has no disagreement.
    pub fn agreed_value(&self) -> Option<&ObservedValue> {
        match self {
            Self::Consistent { candidates, .. } => candidates.first().map(|o| &o.value),
            Self::Conflict { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationReport {
    pub accepted: Vec<PcoObservation>,
    pub rejected: Vec<RejectedObservation>,
    pub resolutions: Vec<ObservationResolution>,
}

impl ObservationReport {
    pub fn has_conflicts(&self) -> bool {
        self.resolutions
            .iter()
            .any(ObservationResolution::is_conflict)
    }

    /// Read-only projection containing only non-conflicting semantic groups.
    /// Consent and schedule authority remain outside this crate.
    pub fn uncontested(&self) -> Vec<&PcoObservation> {
        let keys = self
            .resolutions
            .iter()
            .filter_map(|resolution| match resolution {
                ObservationResolution::Consistent { key, .. } => Some((*key).clone()),
                ObservationResolution::Conflict { .. } => None,
            })
            .collect::<BTreeSet<_>>();
        self.accepted
            .iter()
            .filter(|observation| keys.contains(&observation_key(observation)))
            .collect()
    }
}

/// Parse the supported RFC 3339 subset:
/// YYYY-MM-DDTHH:MM:SS[.1-9 digits](Z|+HH:MM|-HH:MM).
///
/// This parser avoids a runtime timezone database. Leap-second notation and
/// sub-nanosecond precision are rejected, not rounded. The original text and
/// offset are preserved for audit. Offset semantics follow RFC 9557: Z and
/// -00:00 indicate a known UTC instant with unknown local offset; +00:00
/// declares UTC as the preferred reference point.
pub fn parse_rfc3339_timestamp(input: &str) -> Result<ParsedTimestamp, TimestampParseError> {
    if !input.is_ascii() {
        return Err(TimestampParseError::NonAscii);
    }
    let bytes = input.as_bytes();
    if bytes.len() < 20 {
        return Err(TimestampParseError::TooShort);
    }
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || !matches!(bytes[10], b'T' | b't')
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return Err(TimestampParseError::InvalidDateTimeSeparators);
    }

    let year = parse_digits(bytes, 0, 4)? as i64;
    let month = parse_digits(bytes, 5, 2)? as u32;
    let day = parse_digits(bytes, 8, 2)? as u32;
    let hour = parse_digits(bytes, 11, 2)? as u32;
    let minute = parse_digits(bytes, 14, 2)? as u32;
    let second = parse_digits(bytes, 17, 2)? as u32;

    if year == 0 || month == 0 || month > 12 || day == 0 {
        return Err(TimestampParseError::InvalidCalendarDate);
    }
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return Err(TimestampParseError::InvalidCalendarDate),
    };
    if day > days_in_month {
        return Err(TimestampParseError::InvalidCalendarDate);
    }
    if hour > 23 || minute > 59 || second > 60 {
        return Err(TimestampParseError::InvalidClockTime);
    }
    if second == 60 {
        return Err(TimestampParseError::LeapSecondUnsupported);
    }

    let mut index = 19;
    let mut fractional_digits = 0u8;
    let mut fraction_nanos = 0i128;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let fraction_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let digits = index - fraction_start;
        if digits == 0 {
            return Err(TimestampParseError::EmptyFraction);
        }
        if digits > 9 {
            return Err(TimestampParseError::FractionTooPrecise);
        }
        fractional_digits = digits as u8;
        let fraction_value = parse_digits(bytes, fraction_start, digits)?;
        fraction_nanos = i128::from(fraction_value) * 10_i128.pow(u32::from(9 - fractional_digits));
    }

    let (offset_minutes, offset_semantics, consumed) = match bytes.get(index).copied() {
        Some(b'Z' | b'z') if index + 1 == bytes.len() => (
            0i16,
            TimestampOffsetSemantics::UnknownLocalOffset,
            index + 1,
        ),
        Some(b'+' | b'-') => {
            if bytes.len() != index + 6 || bytes[index + 3] != b':' {
                return Err(TimestampParseError::InvalidTimezone);
            }
            let offset_hour = parse_digits(bytes, index + 1, 2)? as i16;
            let offset_minute = parse_digits(bytes, index + 4, 2)? as i16;
            if offset_hour > 23 || offset_minute > 59 {
                return Err(TimestampParseError::InvalidTimezone);
            }
            let sign = if bytes[index] == b'+' { 1 } else { -1 };
            let minutes = sign * (offset_hour * 60 + offset_minute);
            let semantics = if sign == -1 && offset_hour == 0 && offset_minute == 0 {
                TimestampOffsetSemantics::UnknownLocalOffset
            } else {
                TimestampOffsetSemantics::ExplicitOffset
            };
            (minutes, semantics, index + 6)
        }
        _ => return Err(TimestampParseError::InvalidTimezone),
    };
    if consumed != bytes.len() {
        return Err(TimestampParseError::TrailingCharacters);
    }

    let days = days_from_civil(year, month, day);
    let local_seconds = i128::from(days) * 86_400
        + i128::from(hour) * 3_600
        + i128::from(minute) * 60
        + i128::from(second);
    let utc_seconds = local_seconds - i128::from(offset_minutes) * 60;
    let utc_unix_nanos = utc_seconds * NANOS_PER_SECOND + fraction_nanos;

    Ok(ParsedTimestamp {
        source_text: input.to_owned(),
        utc_unix_nanos,
        source_offset_minutes: offset_minutes,
        offset_semantics,
        fractional_digits,
    })
}

fn parse_digits(bytes: &[u8], start: usize, length: usize) -> Result<u64, TimestampParseError> {
    let end = start
        .checked_add(length)
        .ok_or(TimestampParseError::InvalidDigits)?;
    let slice = bytes
        .get(start..end)
        .ok_or(TimestampParseError::InvalidDigits)?;
    if slice.is_empty() || !slice.iter().all(u8::is_ascii_digit) {
        return Err(TimestampParseError::InvalidDigits);
    }
    slice.iter().try_fold(0u64, |value, digit| {
        value
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(*digit - b'0')))
            .ok_or(TimestampParseError::InvalidDigits)
    })
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = (if year >= 0 { year } else { year - 399 }) / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Validate the source/provenance envelope. Future timestamps inside a payload
/// are allowed because planned arrivals/departures can be future events.
pub fn validate_observation(
    observation: &PcoObservation,
    policy: &ValidationPolicy,
) -> Result<(), RejectionReason> {
    if observation.observation_id.trim().is_empty() {
        return Err(RejectionReason::MissingObservationId);
    }
    if observation.port_id.trim().is_empty() {
        return Err(RejectionReason::MissingPortId);
    }
    if observation.port_call_id.trim().is_empty() {
        return Err(RejectionReason::MissingPortCallId);
    }
    if observation.ship_imo_number.trim().is_empty() {
        return Err(RejectionReason::MissingShipImoNumber);
    }
    if observation.source_field_code.trim().is_empty() {
        return Err(RejectionReason::MissingSourceFieldCode);
    }
    if observation.source_id.trim().is_empty() {
        return Err(RejectionReason::MissingSourceId);
    }
    if observation.schema_version.trim().is_empty() {
        return Err(RejectionReason::MissingSchemaVersion);
    }

    match (&observation.field, &observation.value) {
        (PcoField::TerminalIdentifier, ObservedValue::Identifier(value)) => {
            if value.trim().is_empty() {
                return Err(RejectionReason::EmptyIdentifier);
            }
            if !policy.known_terminal_ids.contains(value) {
                return Err(RejectionReason::UnknownTerminal(value.clone()));
            }
        }
        (PcoField::BerthIdentifier, ObservedValue::Identifier(value)) => {
            if value.trim().is_empty() {
                return Err(RejectionReason::EmptyIdentifier);
            }
            if !policy.known_berth_ids.contains(value) {
                return Err(RejectionReason::UnknownBerth(value.clone()));
            }
        }
        (PcoField::ArrivalAtPilotBoardingPlace, ObservedValue::Timestamp { .. })
        | (PcoField::DepartureFromBerth, ObservedValue::Timestamp { .. })
            if observation
                .location_context
                .as_ref()
                .is_some_and(|location| !location.trim().is_empty()) => {}
        (
            PcoField::ArrivalAtPilotBoardingPlace | PcoField::DepartureFromBerth,
            ObservedValue::Timestamp { .. },
        ) => return Err(RejectionReason::MissingLocationContext),
        _ => return Err(RejectionReason::InvalidValueForField),
    }

    let skew = i128::from(policy.allowed_future_clock_skew_seconds.max(0)) * NANOS_PER_SECOND;
    let latest_acceptable = policy.now_utc_nanos.saturating_add(skew);
    if let ObservedValue::Timestamp {
        meaning: TimeMeaning::Actual,
        value,
    } = &observation.value
        && value.utc_unix_nanos > latest_acceptable
    {
        return Err(RejectionReason::ActualTimeTooFarInFuture);
    }
    if observation.source_event_time_utc_nanos > latest_acceptable {
        return Err(RejectionReason::SourceEventTooFarInFuture);
    }
    if observation.ingested_at_utc_nanos > latest_acceptable {
        return Err(RejectionReason::IngestionTooFarInFuture);
    }
    if observation.source_event_time_utc_nanos
        > observation.ingested_at_utc_nanos.saturating_add(skew)
    {
        return Err(RejectionReason::SourceEventAfterIngestion);
    }
    let max_age = i128::from(policy.max_source_age_seconds.max(0)) * NANOS_PER_SECOND;
    if policy.now_utc_nanos >= observation.source_event_time_utc_nanos
        && policy.now_utc_nanos - observation.source_event_time_utc_nanos > max_age
    {
        return Err(RejectionReason::SourceRecordTooOld);
    }
    if let Some(valid_until) = observation.valid_until_utc_nanos {
        if valid_until < observation.source_event_time_utc_nanos {
            return Err(RejectionReason::InvalidValidityWindow);
        }
        if valid_until <= policy.now_utc_nanos {
            return Err(RejectionReason::ObservationExpired);
        }
    }
    Ok(())
}

/// Validate observations, reject duplicate IDs as a set, then resolve
/// semantically equivalent records. Input order cannot change the report.
pub fn resolve_observations(
    input: &[PcoObservation],
    policy: &ValidationPolicy,
) -> ObservationReport {
    let mut id_counts = BTreeMap::<String, usize>::new();
    for observation in input {
        if !observation.observation_id.trim().is_empty() {
            *id_counts
                .entry(observation.observation_id.clone())
                .or_insert(0) += 1;
        }
    }

    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for observation in input {
        let reason = if observation.observation_id.trim().is_empty() {
            Some(RejectionReason::MissingObservationId)
        } else if id_counts
            .get(&observation.observation_id)
            .copied()
            .unwrap_or(0)
            > 1
        {
            Some(RejectionReason::DuplicateObservationId)
        } else {
            validate_observation(observation, policy).err()
        };

        if let Some(reason) = reason {
            rejected.push(RejectedObservation {
                observation_id: observation.observation_id.clone(),
                reason,
            });
        } else {
            accepted.push(observation.clone());
        }
    }

    // A port-call reference is scoped to its reporting port, but one
    // (port, call-reference) pair must not name multiple vessels. Quarantine
    // all otherwise-valid observations for the ambiguous identity.
    let mut vessel_ids_by_call = BTreeMap::<(String, String), BTreeSet<String>>::new();
    for observation in &accepted {
        vessel_ids_by_call
            .entry((
                observation.port_id.clone(),
                observation.port_call_id.clone(),
            ))
            .or_default()
            .insert(observation.ship_imo_number.clone());
    }
    let conflicting_calls = vessel_ids_by_call
        .into_iter()
        .filter_map(|(call_key, vessels)| (vessels.len() > 1).then_some(call_key))
        .collect::<BTreeSet<_>>();
    let mut identity_checked = Vec::with_capacity(accepted.len());
    for observation in accepted.drain(..) {
        let call_key = (
            observation.port_id.clone(),
            observation.port_call_id.clone(),
        );
        if conflicting_calls.contains(&call_key) {
            rejected.push(RejectedObservation {
                observation_id: observation.observation_id.clone(),
                reason: RejectionReason::PortCallVesselIdentityConflict,
            });
        } else {
            identity_checked.push(observation);
        }
    }
    accepted = identity_checked;

    accepted.sort_by(|a, b| {
        a.source_id
            .cmp(&b.source_id)
            .then(a.observation_id.cmp(&b.observation_id))
            .then(
                a.source_event_time_utc_nanos
                    .cmp(&b.source_event_time_utc_nanos),
            )
    });
    rejected.sort();
    let mut groups = BTreeMap::<ObservationKey, Vec<PcoObservation>>::new();
    for observation in &accepted {
        groups
            .entry(observation_key(observation))
            .or_default()
            .push(observation.clone());
    }

    let resolutions = groups
        .into_iter()
        .map(|(key, candidates)| {
            if candidates.first().is_some_and(|first| {
                candidates
                    .iter()
                    .all(|other| same_semantic_value(first, other))
            }) {
                ObservationResolution::Consistent { key, candidates }
            } else {
                ObservationResolution::Conflict { key, candidates }
            }
        })
        .collect();

    ObservationReport {
        accepted,
        rejected,
        resolutions,
    }
}

fn observation_key(observation: &PcoObservation) -> ObservationKey {
    let time_meaning = match &observation.value {
        ObservedValue::Timestamp { meaning, .. } => Some(meaning.clone()),
        ObservedValue::Identifier(_) => None,
    };
    ObservationKey {
        port_id: observation.port_id.clone(),
        port_call_id: observation.port_call_id.clone(),
        ship_imo_number: observation.ship_imo_number.clone(),
        field: observation.field.clone(),
        time_meaning,
    }
}

fn same_semantic_value(left: &PcoObservation, right: &PcoObservation) -> bool {
    match (&left.value, &right.value) {
        (ObservedValue::Identifier(a), ObservedValue::Identifier(b)) => a == b,
        (
            ObservedValue::Timestamp {
                meaning: a_meaning,
                value: a,
            },
            ObservedValue::Timestamp {
                meaning: b_meaning,
                value: b,
            },
        ) => {
            a_meaning == b_meaning
                && a.utc_unix_nanos == b.utc_unix_nanos
                && left.location_context == right.location_context
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(value: &str) -> ParsedTimestamp {
        parse_rfc3339_timestamp(value).expect("frozen timestamp fixture should parse")
    }

    fn policy() -> ValidationPolicy {
        let now = utc("2026-10-18T08:00:00Z").utc_unix_nanos;
        let mut policy = ValidationPolicy::new(now);
        policy.max_source_age_seconds = 6 * 60 * 60;
        policy.allowed_future_clock_skew_seconds = 120;
        policy.known_terminal_ids.insert("TERM-1".to_owned());
        policy.known_berth_ids.insert("BERTH-1".to_owned());
        policy
    }

    fn observation(
        id: &str,
        source: &str,
        field: PcoField,
        value: ObservedValue,
    ) -> PcoObservation {
        let now = policy().now_utc_nanos;
        PcoObservation {
            observation_id: id.to_owned(),
            port_id: "PORT-SYNTHETIC-1".to_owned(),
            port_call_id: "CALL-SYNTHETIC-1".to_owned(),
            ship_imo_number: "IMO-SYNTHETIC-1".to_owned(),
            field,
            location_context: Some("LOCATION-SYNTHETIC-1".to_owned()),
            source_field_code: "synthetic-field-v1".to_owned(),
            value,
            source_id: source.to_owned(),
            source_event_time_utc_nanos: now - 60 * NANOS_PER_SECOND,
            ingested_at_utc_nanos: now - 30 * NANOS_PER_SECOND,
            valid_until_utc_nanos: Some(now + 3600 * NANOS_PER_SECOND),
            schema_version: PCO_GUIDE_VERSION.to_owned(),
        }
    }

    fn arrival(id: &str, source: &str, meaning: TimeMeaning, timestamp: &str) -> PcoObservation {
        observation(
            id,
            source,
            PcoField::ArrivalAtPilotBoardingPlace,
            ObservedValue::Timestamp {
                meaning,
                value: utc(timestamp),
            },
        )
    }

    fn ingest_context(id: &str, source: &str) -> IngestContext {
        let now = policy().now_utc_nanos;
        IngestContext {
            observation_id: id.to_owned(),
            source_id: source.to_owned(),
            source_event_time_utc_nanos: now - 60 * NANOS_PER_SECOND,
            ingested_at_utc_nanos: now - 30 * NANOS_PER_SECOND,
            valid_until_utc_nanos: Some(now + 3600 * NANOS_PER_SECOND),
            schema_version: PCO_GUIDE_VERSION.to_owned(),
        }
    }

    #[test]
    fn maps_pco_planned_arrival_payload_without_collapsing_identity_or_time() {
        let json = include_str!("../fixtures/pco-arrival-valid.json");
        let item =
            parse_pco_arrival_response(json, ingest_context("OBS-PCO-ARRIVAL", "PCO-TESTBED"))
                .unwrap();
        assert_eq!(item.ship_imo_number, "9990001");
        assert_eq!(item.port_id, "ZZSYN");
        assert_eq!(item.port_call_id, "SYNTH-CALL-001");
        assert_eq!(item.location_context.as_deref(), Some("SYNTH-PBP-01"));
        assert_eq!(
            item.source_field_code,
            "dateTimeArrivalPlanned.plannedTimeOfArrival"
        );
        assert!(matches!(
            item.value,
            ObservedValue::Timestamp {
                meaning: TimeMeaning::Planned,
                ..
            }
        ));
        assert!(validate_observation(&item, &policy()).is_ok());
    }

    #[test]
    fn maps_pco_planned_departure_payload_and_preserves_berth_name() {
        let json = include_str!("../fixtures/pco-departure-valid.json");
        let item =
            parse_pco_departure_response(json, ingest_context("OBS-PCO-DEPARTURE", "PCO-TESTBED"))
                .unwrap();
        assert_eq!(item.location_context.as_deref(), Some("SYNTH-BERTH-01"));
        assert_eq!(
            item.source_field_code,
            "dateTimeDeparturePlanned.plannedTimeOfDeparture"
        );
        assert_eq!(item.port_call_id, "SYNTH-CALL-001");
        assert!(validate_observation(&item, &policy()).is_ok());
    }

    #[test]
    fn pco_adapter_rejects_missing_location_and_malformed_time() {
        let missing_location = include_str!("../fixtures/pco-arrival-missing-location.json");
        assert!(matches!(
            parse_pco_arrival_response(
                missing_location,
                ingest_context("OBS-PCO-MISSING", "PCO-TESTBED")
            ),
            Err(PcoPayloadParseError::MissingField(
                "dateTimeArrivalPlanned.pilotBoardingPlaceName"
            ))
        ));

        let malformed_time = include_str!("../fixtures/pco-arrival-malformed-time.json");
        assert!(matches!(
            parse_pco_arrival_response(
                malformed_time,
                ingest_context("OBS-PCO-BAD-TIME", "PCO-TESTBED")
            ),
            Err(PcoPayloadParseError::InvalidTimestamp(
                TimestampParseError::TooShort
            ))
        ));
    }

    #[test]
    fn normalizes_offset_equivalent_timestamps_and_preserves_source_text() {
        let offset = utc("2026-10-18T12:00:00+02:00");
        let zulu = utc("2026-10-18T10:00:00Z");
        assert_eq!(offset.utc_unix_nanos, zulu.utc_unix_nanos);
        assert_eq!(offset.source_offset_minutes, 120);
        assert_eq!(zulu.source_offset_minutes, 0);
        assert_eq!(offset.source_text, "2026-10-18T12:00:00+02:00");
    }

    #[test]
    fn rfc9557_z_and_negative_zero_preserve_unknown_local_offset_semantics() {
        let unknown = utc("2026-10-18T10:00:00-00:00");
        let utc_designator = utc("2026-10-18T10:00:00Z");
        let explicit_zero = utc("2026-10-18T10:00:00+00:00");

        assert_eq!(unknown.utc_unix_nanos, utc_designator.utc_unix_nanos);
        assert_eq!(unknown.utc_unix_nanos, explicit_zero.utc_unix_nanos);
        assert_eq!(
            unknown.offset_semantics(),
            &TimestampOffsetSemantics::UnknownLocalOffset
        );
        assert_eq!(
            utc_designator.offset_semantics(),
            &TimestampOffsetSemantics::UnknownLocalOffset
        );
        assert_eq!(
            explicit_zero.offset_semantics(),
            &TimestampOffsetSemantics::ExplicitOffset
        );
        assert_eq!(unknown.source_text(), "2026-10-18T10:00:00-00:00");
    }

    #[test]
    fn parses_fractional_seconds_without_truncation_up_to_nanoseconds() {
        let whole = utc("2026-10-18T10:00:00Z");
        let fractional = utc("2026-10-18T10:00:00.125Z");
        assert_eq!(
            fractional.utc_unix_nanos - whole.utc_unix_nanos,
            125_000_000
        );
        assert_eq!(fractional.fractional_digits, 3);
        assert!(parse_rfc3339_timestamp("2026-10-18T10:00:00.1234567891Z").is_err());
    }

    #[test]
    fn rejects_invalid_calendar_time_and_timezone_inputs() {
        assert_eq!(
            parse_rfc3339_timestamp("2026-02-29T10:00:00Z"),
            Err(TimestampParseError::InvalidCalendarDate)
        );
        assert_eq!(
            parse_rfc3339_timestamp("2026-01-01T23:59:60Z"),
            Err(TimestampParseError::LeapSecondUnsupported)
        );
        assert!(parse_rfc3339_timestamp("2026-10-18T10:00:00").is_err());
        assert!(parse_rfc3339_timestamp("2026-10-18T10:00:00+99:00").is_err());
    }

    #[test]
    fn future_planned_arrival_is_valid_payload_not_future_source_metadata() {
        let item = arrival(
            "OBS-1",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T14:00:00+02:00",
        );
        assert!(validate_observation(&item, &policy()).is_ok());
    }

    #[test]
    fn future_actual_event_is_rejected_while_future_plan_is_allowed() {
        let planned = arrival(
            "OBS-FUTURE-PLAN",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T14:00:00Z",
        );
        let actual = arrival(
            "OBS-FUTURE-ACTUAL",
            "SOURCE-B",
            TimeMeaning::Actual,
            "2026-10-18T14:00:00Z",
        );
        assert!(validate_observation(&planned, &policy()).is_ok());
        assert_eq!(
            validate_observation(&actual, &policy()),
            Err(RejectionReason::ActualTimeTooFarInFuture)
        );
    }

    #[test]
    fn planned_and_actual_values_are_separate_semantic_groups() {
        let planned = arrival(
            "OBS-PLANNED",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T08:00:00Z",
        );
        let actual = arrival(
            "OBS-ACTUAL",
            "SOURCE-B",
            TimeMeaning::Actual,
            "2026-10-18T08:01:00Z",
        );
        let report = resolve_observations(&[actual, planned], &policy());
        assert_eq!(report.rejected.len(), 0);
        assert_eq!(report.resolutions.len(), 2);
        assert!(!report.has_conflicts());
    }

    #[test]
    fn equivalent_instants_from_different_offsets_are_consistent() {
        let a = arrival(
            "OBS-A",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00+02:00",
        );
        let b = arrival(
            "OBS-B",
            "SOURCE-B",
            TimeMeaning::Planned,
            "2026-10-18T10:00:00Z",
        );
        let report = resolve_observations(&[a, b], &policy());
        assert!(!report.has_conflicts());
        assert!(matches!(
            &report.resolutions[0],
            ObservationResolution::Consistent { .. }
        ));
        assert!(report.resolutions[0].agreed_value().is_some());
    }

    #[test]
    fn conflicting_location_contexts_are_not_mistaken_for_equal_timestamps() {
        let mut a = arrival(
            "OBS-LOCATION-A",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00Z",
        );
        let mut b = arrival(
            "OBS-LOCATION-B",
            "SOURCE-B",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00Z",
        );
        a.location_context = Some("PILOT-BOARDING-PLACE-A".to_owned());
        b.location_context = Some("PILOT-BOARDING-PLACE-B".to_owned());

        let report = resolve_observations(&[a, b], &policy());
        assert!(report.has_conflicts());
        assert!(report.resolutions[0].agreed_value().is_none());
    }

    #[test]
    fn missing_location_context_is_rejected_for_time_observations() {
        let mut item = arrival(
            "OBS-NO-LOCATION",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00Z",
        );
        item.location_context = None;
        assert_eq!(
            validate_observation(&item, &policy()),
            Err(RejectionReason::MissingLocationContext)
        );
    }

    #[test]
    fn conflicting_sources_are_never_resolved_by_arrival_order() {
        let a = arrival(
            "OBS-A",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00Z",
        );
        let b = arrival(
            "OBS-B",
            "SOURCE-B",
            TimeMeaning::Planned,
            "2026-10-18T13:00:00Z",
        );
        let report = resolve_observations(&[b.clone(), a.clone()], &policy());
        assert!(report.has_conflicts());
        assert!(
            report
                .resolutions
                .iter()
                .all(|resolution| resolution.agreed_value().is_none() == resolution.is_conflict())
        );
        let reversed = resolve_observations(&[a, b], &policy());
        assert_eq!(report, reversed);
    }

    #[test]
    fn stale_source_records_are_rejected_explicitly() {
        let mut item = arrival(
            "OBS-STALE",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T14:00:00Z",
        );
        item.source_event_time_utc_nanos = policy().now_utc_nanos - 7 * 60 * 60 * NANOS_PER_SECOND;
        assert_eq!(
            validate_observation(&item, &policy()),
            Err(RejectionReason::SourceRecordTooOld)
        );
    }

    #[test]
    fn future_source_and_ingestion_metadata_are_rejected() {
        let mut item = arrival(
            "OBS-FUTURE",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T14:00:00Z",
        );
        item.source_event_time_utc_nanos = policy().now_utc_nanos + 10 * 60 * NANOS_PER_SECOND;
        assert_eq!(
            validate_observation(&item, &policy()),
            Err(RejectionReason::SourceEventTooFarInFuture)
        );

        item.source_event_time_utc_nanos = policy().now_utc_nanos - NANOS_PER_SECOND;
        item.ingested_at_utc_nanos = policy().now_utc_nanos + 10 * 60 * NANOS_PER_SECOND;
        assert_eq!(
            validate_observation(&item, &policy()),
            Err(RejectionReason::IngestionTooFarInFuture)
        );
    }

    #[test]
    fn unknown_berths_and_field_payload_mismatches_fail_closed() {
        let unknown = observation(
            "OBS-UNKNOWN-BERTH",
            "SOURCE-A",
            PcoField::BerthIdentifier,
            ObservedValue::Identifier("BERTH-UNKNOWN".to_owned()),
        );
        assert_eq!(
            validate_observation(&unknown, &policy()),
            Err(RejectionReason::UnknownBerth("BERTH-UNKNOWN".to_owned()))
        );
        let mismatch = observation(
            "OBS-WRONG-TYPE",
            "SOURCE-A",
            PcoField::DepartureFromBerth,
            ObservedValue::Identifier("BERTH-1".to_owned()),
        );
        assert_eq!(
            validate_observation(&mismatch, &policy()),
            Err(RejectionReason::InvalidValueForField)
        );
    }

    #[test]
    fn expired_or_inverted_validity_windows_are_rejected() {
        let mut item = arrival(
            "OBS-EXPIRED",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T14:00:00Z",
        );
        item.valid_until_utc_nanos = Some(policy().now_utc_nanos);
        assert_eq!(
            validate_observation(&item, &policy()),
            Err(RejectionReason::ObservationExpired)
        );
        item.valid_until_utc_nanos = Some(item.source_event_time_utc_nanos - NANOS_PER_SECOND);
        assert_eq!(
            validate_observation(&item, &policy()),
            Err(RejectionReason::InvalidValidityWindow)
        );
    }

    #[test]
    fn duplicate_observation_ids_reject_every_copy() {
        let a = arrival(
            "OBS-DUP",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00Z",
        );
        let b = arrival(
            "OBS-DUP",
            "SOURCE-B",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00Z",
        );
        let report = resolve_observations(&[a, b], &policy());
        assert!(report.accepted.is_empty());
        assert_eq!(report.rejected.len(), 2);
        assert!(
            report
                .rejected
                .iter()
                .all(|item| item.reason == RejectionReason::DuplicateObservationId)
        );
    }

    #[test]
    fn same_observation_set_has_stable_report_under_input_permutation() {
        let a = arrival(
            "OBS-A",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00Z",
        );
        let b = arrival(
            "OBS-B",
            "SOURCE-B",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00Z",
        );
        let c = arrival(
            "OBS-C",
            "SOURCE-C",
            TimeMeaning::Planned,
            "2026-10-18T13:00:00Z",
        );
        let first = resolve_observations(&[a.clone(), b.clone(), c.clone()], &policy());
        let second = resolve_observations(&[c, a, b], &policy());
        assert_eq!(first, second);
    }

    #[test]
    fn source_event_after_ingestion_beyond_clock_skew_is_rejected() {
        let mut item = arrival(
            "OBS-ORDER",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T14:00:00Z",
        );
        let now = policy().now_utc_nanos;
        item.source_event_time_utc_nanos = now - 10 * NANOS_PER_SECOND;
        item.ingested_at_utc_nanos = now - 300 * NANOS_PER_SECOND;
        assert_eq!(
            validate_observation(&item, &policy()),
            Err(RejectionReason::SourceEventAfterIngestion)
        );
    }

    #[test]
    fn uncontested_projection_omits_every_conflicted_semantic_group() {
        let planned_a = arrival(
            "OBS-PLAN-A",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00Z",
        );
        let planned_b = arrival(
            "OBS-PLAN-B",
            "SOURCE-B",
            TimeMeaning::Planned,
            "2026-10-18T13:00:00Z",
        );
        let departure = observation(
            "OBS-DEPARTURE",
            "SOURCE-C",
            PcoField::DepartureFromBerth,
            ObservedValue::Timestamp {
                meaning: TimeMeaning::Planned,
                value: utc("2026-10-18T18:00:00Z"),
            },
        );
        let report = resolve_observations(&[planned_a, planned_b, departure], &policy());
        assert!(report.has_conflicts());
        let uncontested = report.uncontested();
        assert_eq!(uncontested.len(), 1);
        assert_eq!(uncontested[0].field, PcoField::DepartureFromBerth);
        assert!(
            uncontested
                .iter()
                .all(|item| { item.field != PcoField::ArrivalAtPilotBoardingPlace })
        );
    }

    #[test]
    fn missing_vessel_identity_is_rejected() {
        let mut item = arrival(
            "OBS-NO-IMO",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T14:00:00Z",
        );
        item.ship_imo_number.clear();
        assert_eq!(
            validate_observation(&item, &policy()),
            Err(RejectionReason::MissingShipImoNumber)
        );
    }

    #[test]
    fn conflicting_vessel_ids_for_one_port_call_reject_the_entire_call() {
        let mut a = arrival(
            "OBS-IDENTITY-A",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00Z",
        );
        let mut b = arrival(
            "OBS-IDENTITY-B",
            "SOURCE-B",
            TimeMeaning::Planned,
            "2026-10-18T13:00:00Z",
        );
        a.field = PcoField::ArrivalAtPilotBoardingPlace;
        b.field = PcoField::DepartureFromBerth;
        b.ship_imo_number = "IMO-SYNTHETIC-2".to_owned();

        let report = resolve_observations(&[a.clone(), b.clone()], &policy());
        assert!(report.accepted.is_empty());
        assert_eq!(report.rejected.len(), 2);
        assert!(report.resolutions.is_empty());
        assert!(
            report
                .rejected
                .iter()
                .all(|item| { item.reason == RejectionReason::PortCallVesselIdentityConflict })
        );

        let reversed = resolve_observations(&[b, a], &policy());
        assert_eq!(report, reversed);
    }

    #[test]
    fn same_call_reference_at_different_ports_is_not_a_vessel_collision() {
        let mut a = arrival(
            "OBS-PORT-A",
            "SOURCE-A",
            TimeMeaning::Planned,
            "2026-10-18T12:00:00Z",
        );
        let mut b = arrival(
            "OBS-PORT-B",
            "SOURCE-B",
            TimeMeaning::Planned,
            "2026-10-18T13:00:00Z",
        );
        a.port_id = "PORT-SYNTHETIC-A".to_owned();
        b.port_id = "PORT-SYNTHETIC-B".to_owned();
        b.ship_imo_number = "IMO-SYNTHETIC-2".to_owned();

        let report = resolve_observations(&[a, b], &policy());
        assert_eq!(report.rejected.len(), 0);
        assert_eq!(report.accepted.len(), 2);
        assert_eq!(report.resolutions.len(), 2);
    }

    #[test]
    fn identifiers_require_explicit_registry_entries() {
        let terminal = observation(
            "OBS-TERMINAL",
            "SOURCE-A",
            PcoField::TerminalIdentifier,
            ObservedValue::Identifier("TERM-UNKNOWN".to_owned()),
        );
        assert_eq!(
            validate_observation(&terminal, &policy()),
            Err(RejectionReason::UnknownTerminal("TERM-UNKNOWN".to_owned()))
        );
    }
}
