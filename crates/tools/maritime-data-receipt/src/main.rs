// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

mod adaptive_resilience;
mod cyport_benchmark_tasks;
mod cyport_feature_availability;
mod cyport_interaction_profile;
mod disruption_rehearsal;
mod forecast_applicability;
mod historical_corpus;
mod macro_connectivity;
mod port_identity;
mod s100_profile;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Number;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const MAX_RECEIPT_BYTES: usize = 1_000_000;
const BUFFER_SIZE: usize = 64 * 1024;
const DEFAULT_RECEIPT_PATH: &str =
    "crates/tools/maritime-data-receipt/tests/fixtures/synthetic-water-level-receipt.json";
const SCHEMA_ID: &str = "luminous.symtropy.maritime-data-receipt";
const SCHEMA_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JsonValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

struct StrictJsonVisitor;

impl<'de> Visitor<'de> for StrictJsonVisitor {
    type Value = JsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value with no duplicate object keys")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(JsonValue::Null)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(JsonValue::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(JsonValue::Number(Number::from(value)))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(JsonValue::Number(Number::from(value)))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Number::from_f64(value)
            .map(JsonValue::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(JsonValue::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(JsonValue::String(value))
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(JsonValue::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element::<JsonValue>()? {
            values.push(value);
        }
        Ok(JsonValue::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value::<JsonValue>()?;
            match values.entry(key) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(value);
                }
                std::collections::btree_map::Entry::Occupied(entry) => {
                    return Err(<A::Error as de::Error>::custom(format!(
                        "duplicate object key: {}",
                        entry.key()
                    )));
                }
            }
        }
        Ok(JsonValue::Object(values))
    }
}

impl<'de> Deserialize<'de> for JsonValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictJsonVisitor)
    }
}

#[derive(Debug, Serialize)]
struct ReceiptReport {
    status: &'static str,
    receipt_id: String,
    source_kind: String,
    receipt_path: String,
    receipt_sha256: String,
    artifact_path_checked: String,
    artifact_byte_length: u64,
    artifact_sha256: String,
    source_commit_binding: String,
    metadata_checks_passed: bool,
    byte_integrity_matches: bool,
    execution_performed: bool,
    engineering_admission: &'static str,
    navigation_authorized: bool,
    physical_truth_claim_authorized: bool,
    claim_ceiling: Vec<&'static str>,
}

#[derive(Debug)]
struct ArtifactExpectation {
    relative_path: String,
    byte_length: u64,
    sha256: String,
}

#[derive(Debug)]
struct ReceiptFacts {
    receipt_id: String,
    source_kind: String,
    product: String,
    dataset_id: String,
    dataset_revision: String,
    domain: String,
    coordinate_reference_system: Option<String>,
    vertical_datum: Option<String>,
    units: Vec<String>,
    time_coverage_start_utc: Option<String>,
    time_coverage_end_utc: Option<String>,
    survey_or_observation_time_utc: Option<String>,
    uncertainty_statement: String,
    artifact: ArtifactExpectation,
}

fn object<'a>(value: &'a JsonValue, path: &str) -> Result<&'a BTreeMap<String, JsonValue>, String> {
    match value {
        JsonValue::Object(value) => Ok(value),
        _ => Err(format!("{path}: expected object")),
    }
}

fn array<'a>(value: &'a JsonValue, path: &str) -> Result<&'a [JsonValue], String> {
    match value {
        JsonValue::Array(value) => Ok(value),
        _ => Err(format!("{path}: expected array")),
    }
}

fn field<'a>(value: &'a JsonValue, key: &str, path: &str) -> Result<&'a JsonValue, String> {
    object(value, path)?
        .get(key)
        .ok_or_else(|| format!("{path}: missing required field {key}"))
}

fn text<'a>(value: &'a JsonValue, path: &str) -> Result<&'a str, String> {
    match value {
        JsonValue::String(value) => Ok(value),
        _ => Err(format!("{path}: expected string")),
    }
}

fn optional_text<'a>(value: &'a JsonValue, path: &str) -> Result<Option<&'a str>, String> {
    match value {
        JsonValue::String(value) => Ok(Some(value)),
        JsonValue::Null => Ok(None),
        _ => Err(format!("{path}: expected string or null")),
    }
}

fn bool_value(value: &JsonValue, path: &str) -> Result<bool, String> {
    match value {
        JsonValue::Bool(value) => Ok(*value),
        _ => Err(format!("{path}: expected boolean")),
    }
}

fn integer(value: &JsonValue, path: &str) -> Result<u64, String> {
    match value {
        JsonValue::Number(value) => value
            .as_u64()
            .ok_or_else(|| format!("{path}: expected non-negative integer")),
        _ => Err(format!("{path}: expected integer")),
    }
}

fn exact_keys(value: &JsonValue, expected: &[&str], path: &str) -> Result<(), String> {
    let actual_map = object(value, path)?;
    let actual: BTreeSet<&str> = actual_map.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if actual == expected {
        return Ok(());
    }
    Err(format!(
        "{path}: field-set mismatch; missing={:?}, unknown={:?}",
        expected.difference(&actual).copied().collect::<Vec<_>>(),
        actual.difference(&expected).copied().collect::<Vec<_>>()
    ))
}

fn expect_text(value: &JsonValue, path: &str, expected: &str) -> Result<(), String> {
    let actual = text(value, path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{path}: expected {expected:?}, got {actual:?}"))
    }
}

fn require_nonempty_text<'a>(value: &'a JsonValue, path: &str) -> Result<&'a str, String> {
    let actual = text(value, path)?;
    if actual.trim().is_empty() {
        return Err(format!("{path}: empty strings are forbidden"));
    }
    Ok(actual)
}

fn optional_https_uri(value: &JsonValue, path: &str, required: bool) -> Result<(), String> {
    match optional_text(value, path)? {
        Some(uri) if uri.starts_with("https://") && uri.len() > "https://".len() => Ok(()),
        Some(_) => Err(format!("{path}: expected an HTTPS URI")),
        None if required => Err(format!("{path}: required HTTPS URI is missing")),
        None => Ok(()),
    }
}

fn fixed_digits(value: &str, start: usize, end: usize) -> Option<u32> {
    let bytes = value.as_bytes().get(start..end)?;
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    value.get(start..end)?.parse().ok()
}

fn valid_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
        || bytes.last() != Some(&b'Z')
    {
        return false;
    }

    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        fixed_digits(value, 0, 4),
        fixed_digits(value, 5, 7),
        fixed_digits(value, 8, 10),
        fixed_digits(value, 11, 13),
        fixed_digits(value, 14, 16),
        fixed_digits(value, 17, 19),
    ) else {
        return false;
    };

    let fractional_seconds_valid = match bytes.get(19) {
        Some(b'Z') => bytes.len() == 20,
        Some(b'.') => {
            bytes.len() >= 22 && bytes[20..bytes.len() - 1].iter().all(u8::is_ascii_digit)
        }
        _ => false,
    };
    if !fractional_seconds_valid
        || month == 0
        || month > 12
        || hour > 23
        || minute > 59
        || second > 59
    {
        return false;
    }

    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    day >= 1 && day <= days_in_month
}

fn timestamp_utc<'a>(
    value: &'a JsonValue,
    path: &str,
    required: bool,
) -> Result<Option<&'a str>, String> {
    match optional_text(value, path)? {
        Some(value) if valid_utc_timestamp(value) => Ok(Some(value)),
        Some(_) => Err(format!(
            "{path}: expected a valid RFC3339 UTC timestamp ending in Z"
        )),
        None if required => Err(format!("{path}: required UTC timestamp is missing")),
        None => Ok(None),
    }
}

fn lowercase_sha256(value: &str, path: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!(
            "{path}: expected 64 lowercase hexadecimal SHA-256 characters"
        ))
    }
}

fn validate_relative_artifact_path(value: &str) -> Result<(), String> {
    if value.contains('\\') || value.contains(':') || value.trim().is_empty() {
        return Err(
            "artifact.relative_path: backslashes, drive prefixes, and empty paths are forbidden"
                .to_owned(),
        );
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err("artifact.relative_path: absolute paths are forbidden".to_owned());
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(
                "artifact.relative_path: only normal relative path components are allowed"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

fn validate_receipt(root: &JsonValue) -> Result<ReceiptFacts, String> {
    exact_keys(
        root,
        &[
            "schema_id",
            "schema_version",
            "receipt_id",
            "source",
            "retrieval",
            "artifact",
            "data_context",
            "terms",
            "intended_use",
            "claim_ceiling",
        ],
        "root",
    )?;
    expect_text(field(root, "schema_id", "root")?, "schema_id", SCHEMA_ID)?;
    expect_text(
        field(root, "schema_version", "root")?,
        "schema_version",
        SCHEMA_VERSION,
    )?;
    let receipt_id =
        require_nonempty_text(field(root, "receipt_id", "root")?, "receipt_id")?.to_owned();

    let source = field(root, "source", "root")?;
    exact_keys(
        source,
        &[
            "kind",
            "publisher",
            "dataset_id",
            "product",
            "source_uri",
            "dataset_revision",
        ],
        "source",
    )?;
    let kind = require_nonempty_text(field(source, "kind", "source")?, "source.kind")?.to_owned();
    if kind != "synthetic_fixture" && kind != "public_dataset" {
        return Err("source.kind: unsupported source kind".to_owned());
    }
    require_nonempty_text(field(source, "publisher", "source")?, "source.publisher")?;
    let dataset_id =
        require_nonempty_text(field(source, "dataset_id", "source")?, "source.dataset_id")?
            .to_owned();
    let product = text(field(source, "product", "source")?, "source.product")?;
    let supported_products = [
        "water_level",
        "water_level_prediction",
        "currents",
        "hydrography",
        "port_facility",
        "other",
    ];
    if !supported_products.contains(&product) {
        return Err(format!("source.product: unsupported value {product:?}"));
    }
    let revision = text(
        field(source, "dataset_revision", "source")?,
        "source.dataset_revision",
    )?;
    let dataset_revision = revision.to_owned();
    let normalized_revision = revision.to_ascii_lowercase();
    if ["latest", "unknown", "unspecified"].contains(&normalized_revision.as_str()) {
        return Err(
            "source.dataset_revision must be pinned, not latest/unknown/unspecified".to_owned(),
        );
    }
    let source_uri_required = kind == "public_dataset";
    optional_https_uri(
        field(source, "source_uri", "source")?,
        "source.source_uri",
        source_uri_required,
    )?;
    if kind == "synthetic_fixture"
        && optional_text(field(source, "source_uri", "source")?, "source.source_uri")?.is_some()
    {
        return Err("synthetic_fixture: source_uri must be null".to_owned());
    }

    let retrieval = field(root, "retrieval", "root")?;
    exact_keys(
        retrieval,
        &[
            "method",
            "retrieved_at_utc",
            "request_uri",
            "http_status",
            "reported_content_type",
            "collector_version",
        ],
        "retrieval",
    )?;
    let method =
        require_nonempty_text(field(retrieval, "method", "retrieval")?, "retrieval.method")?;
    require_nonempty_text(
        field(retrieval, "reported_content_type", "retrieval")?,
        "retrieval.reported_content_type",
    )?;
    require_nonempty_text(
        field(retrieval, "collector_version", "retrieval")?,
        "retrieval.collector_version",
    )?;
    let retrieved_at = timestamp_utc(
        field(retrieval, "retrieved_at_utc", "retrieval")?,
        "retrieval.retrieved_at_utc",
        kind == "public_dataset",
    )?;
    let request_uri_required = kind == "public_dataset";
    optional_https_uri(
        field(retrieval, "request_uri", "retrieval")?,
        "retrieval.request_uri",
        request_uri_required,
    )?;
    if kind == "synthetic_fixture" {
        if method != "local_fixture"
            || retrieved_at.is_some()
            || optional_text(
                field(retrieval, "request_uri", "retrieval")?,
                "retrieval.request_uri",
            )?
            .is_some()
            || !matches!(
                field(retrieval, "http_status", "retrieval")?,
                JsonValue::Null
            )
        {
            return Err("synthetic_fixture: retrieval must be local_fixture with null retrieval/request/status fields".to_owned());
        }
    } else {
        if method != "http_get" && method != "manual_download" {
            return Err("public_dataset: method must be http_get or manual_download".to_owned());
        }
        if method == "http_get" {
            let status = integer(
                field(retrieval, "http_status", "retrieval")?,
                "retrieval.http_status",
            )?;
            if !(200..300).contains(&status) {
                return Err(
                    "public_dataset HTTP retrieval must record a 2xx response status".to_owned(),
                );
            }
        } else if !matches!(
            field(retrieval, "http_status", "retrieval")?,
            JsonValue::Null
        ) {
            let status = integer(
                field(retrieval, "http_status", "retrieval")?,
                "retrieval.http_status",
            )?;
            if !(100..=599).contains(&status) {
                return Err(
                    "retrieval.http_status must be null or in the 100..599 range".to_owned(),
                );
            }
        }
    }

    let artifact = field(root, "artifact", "root")?;
    exact_keys(
        artifact,
        &[
            "relative_path",
            "media_type",
            "byte_length",
            "hash_algorithm",
            "sha256",
            "bytes_scope",
        ],
        "artifact",
    )?;
    let relative_path = require_nonempty_text(
        field(artifact, "relative_path", "artifact")?,
        "artifact.relative_path",
    )?
    .to_owned();
    validate_relative_artifact_path(&relative_path)?;
    require_nonempty_text(
        field(artifact, "media_type", "artifact")?,
        "artifact.media_type",
    )?;
    let byte_length = integer(
        field(artifact, "byte_length", "artifact")?,
        "artifact.byte_length",
    )?;
    expect_text(
        field(artifact, "hash_algorithm", "artifact")?,
        "artifact.hash_algorithm",
        "sha-256",
    )?;
    let sha256 = require_nonempty_text(field(artifact, "sha256", "artifact")?, "artifact.sha256")?
        .to_owned();
    lowercase_sha256(&sha256, "artifact.sha256")?;
    expect_text(
        field(artifact, "bytes_scope", "artifact")?,
        "artifact.bytes_scope",
        "exact_file_bytes",
    )?;

    let context = field(root, "data_context", "root")?;
    exact_keys(
        context,
        &[
            "domain",
            "coordinate_reference_system",
            "vertical_datum",
            "units",
            "time_coverage_start_utc",
            "time_coverage_end_utc",
            "survey_or_observation_time_utc",
            "verification_state",
            "uncertainty_statement",
            "freshness_policy",
            "freshness_policy_description",
        ],
        "data_context",
    )?;
    let domain = require_nonempty_text(
        field(context, "domain", "data_context")?,
        "data_context.domain",
    )?;
    let supported_domains = [
        "water_level",
        "water_level_prediction",
        "currents",
        "hydrography",
        "port_facility",
        "other",
    ];
    if !supported_domains.contains(&domain) {
        return Err(format!("data_context.domain: unsupported value {domain:?}"));
    }
    if domain != product {
        return Err("data_context.domain must match source.product in v0.1".to_owned());
    }
    for key in ["coordinate_reference_system", "vertical_datum"] {
        let _ = optional_text(
            field(context, key, "data_context")?,
            &format!("data_context.{key}"),
        )?;
    }
    let coordinate_reference_system = optional_text(
        field(context, "coordinate_reference_system", "data_context")?,
        "data_context.coordinate_reference_system",
    )?
    .map(str::to_owned);
    let vertical_datum = optional_text(
        field(context, "vertical_datum", "data_context")?,
        "data_context.vertical_datum",
    )?
    .map(str::to_owned);
    if ["water_level", "water_level_prediction"].contains(&domain)
        && optional_text(
            field(context, "vertical_datum", "data_context")?,
            "data_context.vertical_datum",
        )?
        .is_none()
    {
        return Err(
            "water-level observations and predictions must carry an explicit vertical datum"
                .to_owned(),
        );
    }
    if domain == "hydrography"
        && optional_text(
            field(context, "coordinate_reference_system", "data_context")?,
            "data_context.coordinate_reference_system",
        )?
        .is_none()
    {
        return Err("hydrography receipt must carry a coordinate reference system".to_owned());
    }
    let units = array(
        field(context, "units", "data_context")?,
        "data_context.units",
    )?;
    if units.is_empty() {
        return Err("data_context.units must not be empty".to_owned());
    }
    let allowed_units = match domain {
        "water_level" | "water_level_prediction" | "hydrography" => {
            &["m", "cm", "mm", "ft", "in"][..]
        }
        "currents" => &[
            "cm/s", "m/s", "kn", "knot", "knots", "degrees", "degree", "deg",
        ][..],
        "port_facility" => &[
            "m",
            "cm",
            "mm",
            "ft",
            "in",
            "degrees_east",
            "degrees_north",
            "count",
        ][..],
        _ => &[
            "m",
            "cm",
            "mm",
            "ft",
            "in",
            "cm/s",
            "m/s",
            "kn",
            "knot",
            "knots",
            "degrees",
            "degree",
            "deg",
            "degrees_east",
            "degrees_north",
            "count",
            "dimensionless",
            "m^2",
            "m3/s",
            "kW",
            "MW",
            "kWh",
            "MWh",
            "seconds",
            "minutes",
            "hours",
        ][..],
    };
    let mut unique_units = BTreeSet::new();
    for (index, unit) in units.iter().enumerate() {
        let unit = require_nonempty_text(unit, &format!("data_context.units[{index}]"))?;
        if !allowed_units.contains(&unit) {
            return Err(format!(
                "data_context.units[{index}]: unit {unit:?} is not in the frozen v0.1 unit vocabulary"
            ));
        }
        if !unique_units.insert(unit) {
            return Err(format!("data_context.units: duplicate unit {unit:?}"));
        }
    }
    let coverage_start = timestamp_utc(
        field(context, "time_coverage_start_utc", "data_context")?,
        "data_context.time_coverage_start_utc",
        false,
    )?;
    let coverage_end = timestamp_utc(
        field(context, "time_coverage_end_utc", "data_context")?,
        "data_context.time_coverage_end_utc",
        false,
    )?;
    let observation_time = timestamp_utc(
        field(context, "survey_or_observation_time_utc", "data_context")?,
        "data_context.survey_or_observation_time_utc",
        false,
    )?;
    if let (Some(start), Some(end)) = (coverage_start, coverage_end)
        && start > end
    {
        return Err("data_context time coverage start must not be after end".to_owned());
    }
    if kind == "public_dataset" {
        match domain {
            "water_level" | "currents" | "hydrography" if observation_time.is_none() => {
                return Err(
                    "public observation/survey records must pin their observation or survey time"
                        .to_owned(),
                );
            }
            "water_level_prediction" if coverage_start.is_none() || coverage_end.is_none() => {
                return Err(
                    "public prediction records must bound their validity interval".to_owned(),
                );
            }
            _ => {}
        }
    }
    let receipt_units = units
        .iter()
        .enumerate()
        .map(|(index, unit)| {
            require_nonempty_text(unit, &format!("data_context.units[{index}]")).map(str::to_owned)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let verification_state = require_nonempty_text(
        field(context, "verification_state", "data_context")?,
        "data_context.verification_state",
    )?;
    if ![
        "preliminary",
        "verified",
        "prediction",
        "synthetic",
        "unknown",
    ]
    .contains(&verification_state)
    {
        return Err(format!(
            "data_context.verification_state: unsupported value {verification_state:?}"
        ));
    }
    let uncertainty_statement = require_nonempty_text(
        field(context, "uncertainty_statement", "data_context")?,
        "data_context.uncertainty_statement",
    )?
    .to_owned();
    let freshness = require_nonempty_text(
        field(context, "freshness_policy", "data_context")?,
        "data_context.freshness_policy",
    )?;
    if ![
        "not_applicable_synthetic",
        "policy_required",
        "historical_only",
        "policy_defined",
    ]
    .contains(&freshness)
    {
        return Err(format!(
            "data_context.freshness_policy: unsupported value {freshness:?}"
        ));
    }
    let freshness_description = require_nonempty_text(
        field(context, "freshness_policy_description", "data_context")?,
        "data_context.freshness_policy_description",
    )?;
    if freshness == "policy_defined"
        && freshness_description
            .to_ascii_lowercase()
            .contains("not applicable")
    {
        return Err("policy_defined freshness requires an actual policy description".to_owned());
    }
    if kind == "synthetic_fixture" {
        expect_text(
            field(context, "verification_state", "data_context")?,
            "data_context.verification_state",
            "synthetic",
        )?;
        expect_text(
            field(context, "freshness_policy", "data_context")?,
            "data_context.freshness_policy",
            "not_applicable_synthetic",
        )?;
        if observation_time.is_none() {
            return Err(
                "synthetic fixture requires its explicitly synthetic timestamp for test coverage"
                    .to_owned(),
            );
        }
    } else {
        if freshness == "not_applicable_synthetic" {
            return Err("public dataset cannot use the synthetic-only freshness policy".to_owned());
        }
        if !["preliminary", "verified", "prediction"].contains(&verification_state) {
            return Err(
                "public dataset requires a declared preliminary, verified, or prediction state"
                    .to_owned(),
            );
        }
        if domain == "water_level_prediction" && verification_state != "prediction" {
            return Err("water_level_prediction must be labelled prediction".to_owned());
        }
        if domain == "water_level" && verification_state == "prediction" {
            return Err("water_level observations must not be labelled prediction".to_owned());
        }
    }

    let terms = field(root, "terms", "root")?;
    exact_keys(
        terms,
        &[
            "review_status",
            "license_name",
            "license_uri",
            "reviewer",
            "reviewed_at_utc",
            "terms_sha256",
            "permitted_use_statement",
        ],
        "terms",
    )?;
    let review_status = require_nonempty_text(
        field(terms, "review_status", "terms")?,
        "terms.review_status",
    )?;
    if ![
        "not_applicable_synthetic",
        "review_required",
        "reviewed_permitted",
    ]
    .contains(&review_status)
    {
        return Err(format!(
            "terms.review_status: unsupported value {review_status:?}"
        ));
    }
    require_nonempty_text(field(terms, "license_name", "terms")?, "terms.license_name")?;
    require_nonempty_text(
        field(terms, "permitted_use_statement", "terms")?,
        "terms.permitted_use_statement",
    )?;
    if kind == "synthetic_fixture" {
        expect_text(
            field(terms, "review_status", "terms")?,
            "terms.review_status",
            "not_applicable_synthetic",
        )?;
        if optional_text(field(terms, "license_uri", "terms")?, "terms.license_uri")?.is_some()
            || optional_text(field(terms, "reviewer", "terms")?, "terms.reviewer")?.is_some()
            || timestamp_utc(
                field(terms, "reviewed_at_utc", "terms")?,
                "terms.reviewed_at_utc",
                false,
            )?
            .is_some()
            || optional_text(field(terms, "terms_sha256", "terms")?, "terms.terms_sha256")?
                .is_some()
        {
            return Err("synthetic_fixture terms fields must be null/not applicable".to_owned());
        }
    } else {
        if review_status != "reviewed_permitted" {
            return Err(
                "public dataset terms must have a recorded reviewed_permitted decision".to_owned(),
            );
        }
        optional_https_uri(
            field(terms, "license_uri", "terms")?,
            "terms.license_uri",
            true,
        )?;
        require_nonempty_text(field(terms, "reviewer", "terms")?, "terms.reviewer")?;
        timestamp_utc(
            field(terms, "reviewed_at_utc", "terms")?,
            "terms.reviewed_at_utc",
            true,
        )?;
        let terms_hash =
            require_nonempty_text(field(terms, "terms_sha256", "terms")?, "terms.terms_sha256")?;
        lowercase_sha256(terms_hash, "terms.terms_sha256")?;
    }

    let intended = field(root, "intended_use", "root")?;
    exact_keys(
        intended,
        &[
            "scope",
            "navigation_use_authorized",
            "physical_truth_claim_authorized",
            "independent_review_required",
        ],
        "intended_use",
    )?;
    let scope = require_nonempty_text(
        field(intended, "scope", "intended_use")?,
        "intended_use.scope",
    )?;
    if !["provenance_test_only", "offline_research", "planning_only"].contains(&scope) {
        return Err(format!("intended_use.scope: unsupported value {scope:?}"));
    }
    if bool_value(
        field(intended, "navigation_use_authorized", "intended_use")?,
        "intended_use.navigation_use_authorized",
    )? {
        return Err("navigation_use_authorized must remain false".to_owned());
    }
    if bool_value(
        field(intended, "physical_truth_claim_authorized", "intended_use")?,
        "intended_use.physical_truth_claim_authorized",
    )? {
        return Err("physical_truth_claim_authorized must remain false".to_owned());
    }
    if !bool_value(
        field(intended, "independent_review_required", "intended_use")?,
        "intended_use.independent_review_required",
    )? {
        return Err("independent_review_required must remain true".to_owned());
    }

    let claims = array(field(root, "claim_ceiling", "root")?, "claim_ceiling")?;
    if claims.len() < 3 {
        return Err("claim_ceiling must contain at least three explicit limitations".to_owned());
    }
    let mut claim_texts = BTreeSet::new();
    for (index, claim) in claims.iter().enumerate() {
        let claim = require_nonempty_text(claim, &format!("claim_ceiling[{index}]"))?;
        if !claim_texts.insert(claim) {
            return Err("claim_ceiling: duplicate entries are forbidden".to_owned());
        }
    }

    Ok(ReceiptFacts {
        receipt_id,
        source_kind: kind,
        product: product.to_owned(),
        dataset_id,
        dataset_revision,
        domain: domain.to_owned(),
        coordinate_reference_system,
        vertical_datum,
        units: receipt_units,
        time_coverage_start_utc: coverage_start.map(str::to_owned),
        time_coverage_end_utc: coverage_end.map(str::to_owned),
        survey_or_observation_time_utc: observation_time.map(str::to_owned),
        uncertainty_statement,
        artifact: ArtifactExpectation {
            relative_path,
            byte_length,
            sha256,
        },
    })
}

fn parse_strict(bytes: &[u8]) -> Result<JsonValue, String> {
    if bytes.len() > MAX_RECEIPT_BYTES {
        return Err(format!(
            "receipt exceeds the {MAX_RECEIPT_BYTES}-byte size bound"
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let root = JsonValue::deserialize(&mut deserializer)
        .map_err(|error| format!("invalid receipt JSON: {error}"))?;
    deserializer
        .end()
        .map_err(|error| format!("trailing receipt payload: {error}"))?;
    Ok(root)
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha256_file(path: &Path) -> Result<(u64, String), String> {
    let mut file = File::open(path)
        .map_err(|error| format!("cannot open artifact {}: {error}", path.display()))?;
    let mut buffer = [0_u8; BUFFER_SIZE];
    let mut hasher = Sha256::new();
    let mut length = 0_u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot read artifact {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        length = length
            .checked_add(read as u64)
            .ok_or_else(|| "artifact byte length overflow".to_owned())?;
        hasher.update(&buffer[..read]);
    }
    let digest = hasher.finalize();
    let hash = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok((length, hash))
}

fn verify(
    receipt_bytes: &[u8],
    artifact_path: &Path,
    receipt_path: &str,
    source_commit: &str,
) -> Result<ReceiptReport, String> {
    let root = parse_strict(receipt_bytes)?;
    let facts = validate_receipt(&root)?;
    let receipt_sha256 = sha256_bytes(receipt_bytes);
    let (observed_length, observed_sha256) = sha256_file(artifact_path)?;
    if observed_length != facts.artifact.byte_length {
        return Err(format!(
            "artifact length mismatch: receipt says {}, observed {}",
            facts.artifact.byte_length, observed_length
        ));
    }
    if observed_sha256 != facts.artifact.sha256 {
        return Err(format!(
            "artifact SHA-256 mismatch: receipt says {}, observed {}",
            facts.artifact.sha256, observed_sha256
        ));
    }
    Ok(ReceiptReport {
        status: "RECEIPT_AND_ARTIFACT_BYTES_MATCH_NOT_ADMITTED",
        receipt_id: facts.receipt_id,
        source_kind: facts.source_kind,
        receipt_path: receipt_path.to_owned(),
        receipt_sha256,
        artifact_path_checked: artifact_path.display().to_string(),
        artifact_byte_length: observed_length,
        artifact_sha256: observed_sha256,
        source_commit_binding: source_commit.to_owned(),
        metadata_checks_passed: true,
        byte_integrity_matches: true,
        execution_performed: false,
        engineering_admission: "NotAdmitted",
        navigation_authorized: false,
        physical_truth_claim_authorized: false,
        claim_ceiling: vec![
            "Receipt field, declared provenance, and exact-artifact byte-integrity checks only.",
            "This tool does not independently establish licence interpretation, publisher authenticity, observation quality, freshness, or physical truth.",
            "No navigation, under-keel clearance, engineering safety, certification, external approval, or physical-operation authority.",
        ],
    })
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let first_arg = args.next();
    if first_arg.as_deref() == Some("--validate-cyport-feature-availability") {
        let register_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --validate-cyport-feature-availability <register-json-path>"
                .to_owned()
        })?;
        if args.next().is_some() {
            return Err("too many arguments for CyPort feature availability validation".to_owned());
        }
        return cyport_feature_availability::run_cli(&register_path);
    }
    if first_arg.as_deref() == Some("--validate-cyport-benchmark-tasks") {
        let contract_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --validate-cyport-benchmark-tasks <contract-json-path>"
                .to_owned()
        })?;
        if args.next().is_some() {
            return Err("too many arguments for CyPort benchmark task validation".to_owned());
        }
        return cyport_benchmark_tasks::run_cli(&contract_path);
    }
    if first_arg.as_deref() == Some("--validate-cyport-episode-split-manifest") {
        let manifest_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --validate-cyport-episode-split-manifest <manifest-json-path>"
                .to_owned()
        })?;
        if args.next().is_some() {
            return Err(
                "too many arguments for CyPort episode split manifest validation".to_owned(),
            );
        }
        return cyport_interaction_profile::validate_episode_split_manifest_cli(&manifest_path);
    }
    if first_arg.as_deref() == Some("--verify-cyport-episode-split") {
        let csv_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --verify-cyport-episode-split <interaction-data-csv-path> <manifest-json-path>"
                .to_owned()
        })?;
        let manifest_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --verify-cyport-episode-split <interaction-data-csv-path> <manifest-json-path>"
                .to_owned()
        })?;
        if args.next().is_some() {
            return Err("too many arguments for CyPort source-to-split verification".to_owned());
        }
        return cyport_interaction_profile::verify_episode_split_cli(&csv_path, &manifest_path);
    }
    if first_arg.as_deref() == Some("--profile-cyport-interactions") {
        let csv_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --profile-cyport-interactions <interaction-data-csv-path>"
                .to_owned()
        })?;
        if args.next().is_some() {
            return Err("too many arguments for CyPort interaction profiling".to_owned());
        }
        return cyport_interaction_profile::run_cli(&csv_path);
    }
    if first_arg.as_deref() == Some("--validate-historical-corpus-manifest") {
        let manifest_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --validate-historical-corpus-manifest <manifest-json-path>".to_owned()
        })?;
        if args.next().is_some() {
            return Err(
                "too many arguments for historical corpus manifest verification".to_owned(),
            );
        }
        return historical_corpus::run_cli(&manifest_path);
    }
    if first_arg.as_deref() == Some("--evaluate-adaptive-resilience") {
        let fixture_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --evaluate-adaptive-resilience <fixture-json-path>"
                .to_owned()
        })?;
        if args.next().is_some() {
            return Err("too many arguments for adaptive resilience verification".to_owned());
        }
        return adaptive_resilience::run_cli(&fixture_path);
    }
    if first_arg.as_deref() == Some("--evaluate-disruption-rehearsal") {
        let scenario_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --evaluate-disruption-rehearsal <scenario-json-path>"
                .to_owned()
        })?;
        if args.next().is_some() {
            return Err("too many arguments for disruption rehearsal verification".to_owned());
        }
        return disruption_rehearsal::run_cli(&scenario_path);
    }
    if first_arg.as_deref() == Some("--evaluate-forecast-applicability") {
        let profile_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --evaluate-forecast-applicability <profile-json-path> <receipt-json-path> [artifact-path-override]".to_owned()
        })?;
        let receipt_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --evaluate-forecast-applicability <profile-json-path> <receipt-json-path> [artifact-path-override]".to_owned()
        })?;
        let artifact_override = args.next();
        if args.next().is_some() {
            return Err("too many arguments for forecast applicability verification".to_owned());
        }
        return forecast_applicability::run_cli(
            &profile_path,
            &receipt_path,
            artifact_override.as_deref(),
        );
    }
    if first_arg.as_deref() == Some("--validate-s100-profile") {
        let profile_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --validate-s100-profile <profile-json-path> <receipt-json-path> [artifact-path-override]".to_owned()
        })?;
        let receipt_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --validate-s100-profile <profile-json-path> <receipt-json-path> [artifact-path-override]".to_owned()
        })?;
        let artifact_override = args.next();
        if args.next().is_some() {
            return Err("too many arguments for S-100 profile verification".to_owned());
        }
        return s100_profile::run_cli(&profile_path, &receipt_path, artifact_override.as_deref());
    }
    if first_arg.as_deref() == Some("--validate-macro-connectivity-profile") {
        let profile_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --validate-macro-connectivity-profile <profile-json-path> <receipt-json-path> [artifact-path-override] [terms-snapshot-path-override]".to_owned()
        })?;
        let receipt_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --validate-macro-connectivity-profile <profile-json-path> <receipt-json-path> [artifact-path-override] [terms-snapshot-path-override]".to_owned()
        })?;
        let artifact_override = args.next();
        let terms_snapshot_override = args.next();
        if args.next().is_some() {
            return Err(
                "too many arguments for macro-connectivity profile verification".to_owned(),
            );
        }
        return macro_connectivity::run_cli(
            &profile_path,
            &receipt_path,
            artifact_override.as_deref(),
            terms_snapshot_override.as_deref(),
        );
    }
    if first_arg.as_deref() == Some("--verify-port-crosswalk-snapshots") {
        let crosswalk_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --verify-port-crosswalk-snapshots <crosswalk-json-path> <source-snapshot-path> <locode-registry-path>".to_owned()
        })?;
        let source_snapshot_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --verify-port-crosswalk-snapshots <crosswalk-json-path> <source-snapshot-path> <locode-registry-path>".to_owned()
        })?;
        let locode_registry_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --verify-port-crosswalk-snapshots <crosswalk-json-path> <source-snapshot-path> <locode-registry-path>".to_owned()
        })?;
        if args.next().is_some() {
            return Err("too many arguments for port crosswalk snapshot verification".to_owned());
        }
        return port_identity::run_cli_with_snapshots(
            &crosswalk_path,
            &source_snapshot_path,
            &locode_registry_path,
        );
    }
    if first_arg.as_deref() == Some("--validate-port-crosswalk") {
        let crosswalk_path = args.next().ok_or_else(|| {
            "usage: maritime-data-receipt --validate-port-crosswalk <crosswalk-json-path>"
                .to_owned()
        })?;
        if args.next().is_some() {
            return Err("too many arguments for port crosswalk verification".to_owned());
        }
        return port_identity::run_cli(&crosswalk_path);
    }
    let receipt_path = first_arg.unwrap_or_else(|| DEFAULT_RECEIPT_PATH.to_owned());
    let artifact_override = args.next();
    if args.next().is_some() {
        return Err(
            "usage: maritime-data-receipt [receipt-json-path] [optional-artifact-path-override]"
                .to_owned(),
        );
    }
    let receipt_metadata = fs::metadata(&receipt_path)
        .map_err(|error| format!("cannot stat receipt {}: {error}", receipt_path))?;
    if receipt_metadata.len() > MAX_RECEIPT_BYTES as u64 {
        return Err(format!(
            "receipt exceeds the {MAX_RECEIPT_BYTES}-byte size bound"
        ));
    }
    let receipt_bytes = fs::read(&receipt_path)
        .map_err(|error| format!("cannot read receipt {}: {error}", receipt_path))?;
    let root = parse_strict(&receipt_bytes)?;
    let facts = validate_receipt(&root)?;
    let artifact_path = artifact_override
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&facts.artifact.relative_path));
    let source_commit =
        env::var("MARITIME_SOURCE_COMMIT").unwrap_or_else(|_| "UNBOUND_LOCAL_SOURCE".to_owned());
    let report = verify(
        &receipt_bytes,
        &artifact_path,
        &receipt_path,
        &source_commit,
    )?;
    let output = serde_json::to_string(&report)
        .map_err(|error| format!("cannot serialize receipt verification report: {error}"))?;
    println!("{output}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL_MARITIME_DATA_RECEIPT: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt_bytes() -> Vec<u8> {
        include_bytes!("../tests/fixtures/synthetic-water-level-receipt.json").to_vec()
    }

    fn artifact_bytes() -> Vec<u8> {
        include_bytes!("../tests/fixtures/synthetic-water-level-payload.json").to_vec()
    }

    fn verify_bytes(receipt: &[u8], artifact: &[u8]) -> Result<ReceiptReport, String> {
        let receipt_value = parse_strict(receipt)?;
        let facts = validate_receipt(&receipt_value)?;
        let observed_hash = sha256_bytes(artifact);
        if artifact.len() as u64 != facts.artifact.byte_length {
            return Err("artifact length mismatch".to_owned());
        }
        if observed_hash != facts.artifact.sha256 {
            return Err("artifact SHA-256 mismatch".to_owned());
        }
        Ok(ReceiptReport {
            status: "RECEIPT_AND_ARTIFACT_BYTES_MATCH_NOT_ADMITTED",
            receipt_id: facts.receipt_id,
            source_kind: facts.source_kind,
            receipt_path: "test-receipt.json".to_owned(),
            receipt_sha256: sha256_bytes(receipt),
            artifact_path_checked: facts.artifact.relative_path,
            artifact_byte_length: artifact.len() as u64,
            artifact_sha256: observed_hash,
            source_commit_binding: "unit-test".to_owned(),
            metadata_checks_passed: true,
            byte_integrity_matches: true,
            execution_performed: false,
            engineering_admission: "NotAdmitted",
            navigation_authorized: false,
            physical_truth_claim_authorized: false,
            claim_ceiling: vec![
                "Receipt field, declared provenance, and exact-artifact byte-integrity checks only.",
                "This tool does not independently establish licence interpretation, publisher authenticity, observation quality, freshness, or physical truth.",
                "No navigation, under-keel clearance, engineering safety, certification, external approval, or physical-operation authority.",
            ],
        })
    }

    #[test]
    fn synthetic_receipt_and_payload_match_exact_bytes() {
        let receipt = receipt_bytes();
        let payload = artifact_bytes();
        let report = verify_bytes(&receipt, &payload).expect("synthetic receipt should verify");
        assert_eq!(report.source_kind, "synthetic_fixture");
        assert_eq!(report.artifact_byte_length, 293);
        assert_eq!(
            report.artifact_sha256,
            "3bdbe51ac1f41bd8a837e04ce6ea6ef3925dacd2e8ead3086b6c5a8632993844"
        );
        assert_eq!(
            report.status,
            "RECEIPT_AND_ARTIFACT_BYTES_MATCH_NOT_ADMITTED"
        );
        assert_eq!(report.engineering_admission, "NotAdmitted");
        assert!(!report.navigation_authorized);
    }

    #[test]
    fn changed_artifact_bytes_are_rejected() {
        let receipt = receipt_bytes();
        let mut payload = artifact_bytes();
        payload.push(b' ');
        assert!(verify_bytes(&receipt, &payload).is_err());
    }

    #[test]
    fn duplicate_receipt_object_keys_are_rejected() {
        let raw = br#"{"schema_id":"one","schema_id":"two"}"#;
        assert!(
            parse_strict(raw)
                .unwrap_err()
                .contains("duplicate object key")
        );
    }

    #[test]
    fn unknown_receipt_fields_are_rejected() {
        let raw = String::from_utf8(receipt_bytes()).expect("receipt is UTF-8");
        let mutated = raw.replacen(
            "\"schema_version\": \"0.1.0\",",
            "\"schema_version\": \"0.1.0\",\n  \"unexpected\": true,",
            1,
        );
        assert!(
            parse_strict(mutated.as_bytes())
                .and_then(|value| validate_receipt(&value))
                .is_err()
        );
    }

    #[test]
    fn water_level_without_vertical_datum_is_rejected() {
        let raw = String::from_utf8(receipt_bytes()).expect("receipt is UTF-8");
        let mutated = raw.replacen(
            "\"vertical_datum\": \"MLLW\"",
            "\"vertical_datum\": null",
            1,
        );
        let result = parse_strict(mutated.as_bytes()).and_then(|value| validate_receipt(&value));
        assert!(result.is_err());
    }

    #[test]
    fn artifact_path_traversal_is_rejected() {
        let raw = String::from_utf8(receipt_bytes()).expect("receipt is UTF-8");
        let mutated = raw.replacen(
            "crates/tools/maritime-data-receipt/tests/fixtures/synthetic-water-level-payload.json",
            "../outside.json",
            1,
        );
        let result = parse_strict(mutated.as_bytes()).and_then(|value| validate_receipt(&value));
        assert!(result.is_err());
    }

    #[test]
    fn synthetic_fixture_cannot_claim_a_real_license_review() {
        let raw = String::from_utf8(receipt_bytes()).expect("receipt is UTF-8");
        let mutated = raw.replacen(
            "\"review_status\": \"not_applicable_synthetic\"",
            "\"review_status\": \"reviewed_permitted\"",
            1,
        );
        let result = parse_strict(mutated.as_bytes()).and_then(|value| validate_receipt(&value));
        assert!(result.is_err());
    }

    #[test]
    fn receipt_with_unknown_verification_state_does_not_override_claim_ceiling() {
        let raw = String::from_utf8(receipt_bytes()).expect("receipt is UTF-8");
        let mutated = raw.replacen(
            "\"verification_state\": \"synthetic\"",
            "\"verification_state\": \"unknown\"",
            1,
        );
        let result = parse_strict(mutated.as_bytes()).and_then(|value| validate_receipt(&value));
        assert!(result.is_err());
    }

    #[test]
    fn missing_license_review_for_public_dataset_fails_closed() {
        let raw = String::from_utf8(receipt_bytes()).expect("receipt is UTF-8");
        let mut mutated = raw;
        for (from, to) in [
            (
                "\"kind\": \"synthetic_fixture\"",
                "\"kind\": \"public_dataset\"",
            ),
            (
                "\"source_uri\": null",
                "\"source_uri\": \"https://example.invalid/dataset/v1\"",
            ),
            (
                "\"method\": \"local_fixture\"",
                "\"method\": \"manual_download\"",
            ),
            (
                "\"retrieved_at_utc\": null",
                "\"retrieved_at_utc\": \"2026-10-15T06:00:00Z\"",
            ),
            (
                "\"request_uri\": null",
                "\"request_uri\": \"https://example.invalid/dataset/v1\"",
            ),
            (
                "\"verification_state\": \"synthetic\"",
                "\"verification_state\": \"verified\"",
            ),
            (
                "\"freshness_policy\": \"not_applicable_synthetic\"",
                "\"freshness_policy\": \"historical_only\"",
            ),
            (
                "\"freshness_policy_description\": \"Not applicable to an invented fixture; public datasets require an explicit policy statement and source-specific age limits.\"",
                "\"freshness_policy_description\": \"Historical-only test fixture; age policy still requires independent review.\"",
            ),
            (
                "\"publisher\": \"Luminous Dynamics\"",
                "\"publisher\": \"Test Issuer\"",
            ),
            (
                "\"dataset_id\": \"synthetic-water-level-example-v0\"",
                "\"dataset_id\": \"public-dataset-test-only\"",
            ),
        ] {
            assert!(
                mutated.contains(from),
                "test mutation anchor missing: {from}"
            );
            mutated = mutated.replacen(from, to, 1);
        }
        let result = parse_strict(mutated.as_bytes()).and_then(|value| validate_receipt(&value));
        assert!(
            result
                .expect_err("unreviewed public dataset must fail closed")
                .contains("terms must have a recorded reviewed_permitted decision")
        );
    }

    #[test]
    fn unknown_units_are_rejected() {
        let raw = String::from_utf8(receipt_bytes()).expect("receipt is UTF-8");
        let mutated = raw.replacen(
            "\"units\": [\n      \"m\"",
            "\"units\": [\n      \"furlong\"",
            1,
        );
        let result = parse_strict(mutated.as_bytes()).and_then(|value| validate_receipt(&value));
        assert!(result.is_err());
    }

    #[test]
    fn impossible_calendar_timestamp_is_rejected() {
        let raw = String::from_utf8(receipt_bytes()).expect("receipt is UTF-8");
        let mutated = raw.replacen("2026-10-14T12:00:00Z", "2026-02-31T12:00:00Z", 1);
        let result = parse_strict(mutated.as_bytes()).and_then(|value| validate_receipt(&value));
        assert!(result.is_err());
    }

    #[test]
    fn mutable_dataset_revision_is_rejected() {
        let raw = String::from_utf8(receipt_bytes()).expect("receipt is UTF-8");
        let mutated = raw.replacen(
            "\"dataset_revision\": \"v0.1.0\"",
            "\"dataset_revision\": \"latest\"",
            1,
        );
        let result = parse_strict(mutated.as_bytes()).and_then(|value| validate_receipt(&value));
        assert!(result.is_err());
    }

    #[test]
    fn receipt_schema_is_valid_json_with_expected_identity() {
        let schema = include_str!("../../../../docs/tech/MARITIME_DATA_RECEIPT_V0_1.schema.json");
        let value: serde_json::Value =
            serde_json::from_str(schema).expect("schema must be valid JSON");
        assert_eq!(
            value["properties"]["schema_id"]["const"],
            serde_json::Value::String(SCHEMA_ID.to_owned())
        );
        assert_eq!(
            value["properties"]["schema_version"]["const"],
            serde_json::Value::String(SCHEMA_VERSION.to_owned())
        );
    }
}
