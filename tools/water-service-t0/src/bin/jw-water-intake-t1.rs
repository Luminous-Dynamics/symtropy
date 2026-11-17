// SPDX-License-Identifier: AGPL-3.0-or-later
//! Offline T1 integrity verifier for one Watersearch suburb-history capture.
//! No network fetch, crawler, control, or actuation path is provided.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

const RECEIPT_SCHEMA: &str = "jw-watersearch-intake-t1-receipt-v0.1";
const ALLOWED_LEVELS: [&str; 8] = [
    "adequate",
    "fair",
    "pressure",
    "low",
    "warned",
    "no_water",
    "critical",
    "no_report",
];

fn field<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("{context}: missing '{key}'"))
}

fn string<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a str, String> {
    field(value, key, context)?
        .as_str()
        .ok_or_else(|| format!("{context}: '{key}' must be a string"))
}

fn optional_string<'a>(
    value: &'a Value,
    key: &str,
    context: &str,
) -> Result<Option<&'a str>, String> {
    let item = field(value, key, context)?;
    if item.is_null() {
        Ok(None)
    } else {
        item.as_str()
            .map(Some)
            .ok_or_else(|| format!("{context}: '{key}' must be a string or null"))
    }
}

fn integer(value: &Value, key: &str, context: &str) -> Result<u64, String> {
    field(value, key, context)?
        .as_u64()
        .ok_or_else(|| format!("{context}: '{key}' must be a non-negative integer"))
}

fn ensure_keys(value: &Value, expected: &[&str], context: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context}: must be an object"))?;
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let required: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != required {
        return Err(format!(
            "{context}: key set differs from the v0.1 closed contract"
        ));
    }
    Ok(())
}

fn parse_date(value: &str) -> Result<(i64, u32, u32), String> {
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("invalid YYYY-MM-DD date '{value}'"));
    }
    let year = parts[0]
        .parse::<i64>()
        .map_err(|_| format!("invalid year '{value}'"))?;
    let month = parts[1]
        .parse::<u32>()
        .map_err(|_| format!("invalid month '{value}'"))?;
    let day = parts[2]
        .parse::<u32>()
        .map_err(|_| format!("invalid day '{value}'"))?;
    if !(1..=12).contains(&month) {
        return Err(format!("month out of range in '{value}'"));
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return Err(format!("month out of range in '{value}'")),
    };
    if day == 0 || day > month_days {
        return Err(format!("day out of range in '{value}'"));
    }
    Ok((year, month, day))
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let year_of_era = y - era * 400;
    let month_prime = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn validate_utc_timestamp(value: &str) -> Result<(), String> {
    let (date, time) = value
        .split_once('T')
        .ok_or_else(|| "captured_at_utc must contain T".to_owned())?;
    parse_date(date)?;
    let clock = time
        .strip_suffix('Z')
        .ok_or_else(|| "captured_at_utc must end in Z".to_owned())?;
    let parts: Vec<&str> = clock.split(':').collect();
    if parts.len() != 3 {
        return Err("captured_at_utc must use HH:MM:SS".to_owned());
    }
    let hour = parts[0]
        .parse::<u32>()
        .map_err(|_| "invalid UTC hour".to_owned())?;
    let minute = parts[1]
        .parse::<u32>()
        .map_err(|_| "invalid UTC minute".to_owned())?;
    let second = parts[2]
        .parse::<u32>()
        .map_err(|_| "invalid UTC second".to_owned())?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err("captured_at_utc clock component out of range".to_owned());
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_raw_headers(bytes: &[u8]) -> Result<(u16, BTreeMap<String, String>), String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "raw response headers must be UTF-8/ASCII".to_owned())?;
    let mut status = None;
    let mut headers = BTreeMap::new();
    for raw_line in text.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if line.starts_with("HTTP/") {
            let code = line
                .split_whitespace()
                .nth(1)
                .ok_or_else(|| "invalid HTTP status line".to_owned())?
                .parse::<u16>()
                .map_err(|_| "invalid HTTP status code".to_owned())?;
            status = Some(code);
            headers.clear();
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| format!("invalid response header line '{line}'"))?;
        let key = name.trim().to_ascii_lowercase();
        if key.is_empty() {
            return Err("empty response header name".to_owned());
        }
        if headers
            .insert(key.clone(), value.trim().to_owned())
            .is_some()
        {
            return Err(format!("duplicate response header '{key}'"));
        }
    }
    let status = status.ok_or_else(|| "raw header block has no HTTP status line".to_owned())?;
    Ok((status, headers))
}

fn is_valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
}

fn validate_url(url: &str, mode: &str, slug: &str) -> Result<(), String> {
    let path = format!("/api/v1/suburbs/{slug}/history");
    let valid = match mode {
        "synthetic_fixture" => url == format!("https://fixture.invalid{path}"),
        "publisher_capture" => {
            url == format!("https://watersearch.co.za{path}")
                || url.starts_with(&format!("https://watersearch.co.za{path}?"))
        }
        _ => false,
    };
    if !valid {
        return Err(format!("URL is not permitted for capture mode '{mode}'"));
    }
    Ok(())
}

fn validate_capture(
    body_bytes: &[u8],
    header_bytes: &[u8],
    manifest_bytes: &[u8],
    schema: &Value,
) -> Result<Value, String> {
    let manifest: Value = serde_json::from_slice(manifest_bytes)
        .map_err(|error| format!("capture manifest JSON: {error}"))?;
    let body: Value = serde_json::from_slice(body_bytes)
        .map_err(|error| format!("response body JSON: {error}"))?;

    if string(schema, "$schema", "capture schema")?
        != "https://json-schema.org/draft/2020-12/schema"
        || field(schema, "additionalProperties", "capture schema")?.as_bool() != Some(false)
    {
        return Err("capture schema must be closed Draft 2020-12".to_owned());
    }

    ensure_keys(
        &manifest,
        &[
            "schema_version",
            "capture_mode",
            "evidence_class",
            "source",
            "capture",
            "rights_review",
            "notes",
        ],
        "capture manifest",
    )?;
    if string(&manifest, "schema_version", "manifest")? != "0.1.0" {
        return Err("unsupported capture manifest version".to_owned());
    }
    let mode = string(&manifest, "capture_mode", "manifest")?;
    let evidence_class = string(&manifest, "evidence_class", "manifest")?;
    match (mode, evidence_class) {
        ("synthetic_fixture", "synthetic_only")
        | ("publisher_capture", "publisher_reported_asset_status") => {}
        _ => return Err("capture_mode/evidence_class mismatch".to_owned()),
    }

    let source = field(&manifest, "source", "manifest")?;
    ensure_keys(
        source,
        &[
            "publisher",
            "underlying_facts_source",
            "url",
            "suburb_slug",
            "from_date",
            "to_date",
        ],
        "source",
    )?;
    if string(source, "publisher", "source")? != "Watersearch"
        || string(source, "underlying_facts_source", "source")? != "Johannesburg Water"
    {
        return Err("source attribution fields do not match the declared source".to_owned());
    }
    let source_url = string(source, "url", "source")?;
    let slug = string(source, "suburb_slug", "source")?;
    if !is_valid_slug(slug) {
        return Err("suburb_slug is not a lower-case URL slug".to_owned());
    }
    validate_url(source_url, mode, slug)?;
    let from_date = optional_string(source, "from_date", "source")?;
    let to_date = optional_string(source, "to_date", "source")?;
    let from_day = from_date
        .map(|d| parse_date(d).map(|(y, m, d)| days_from_civil(y, m, d)))
        .transpose()?;
    let to_day = to_date
        .map(|d| parse_date(d).map(|(y, m, d)| days_from_civil(y, m, d)))
        .transpose()?;
    if let (Some(start), Some(end)) = (from_day, to_day)
        && start > end
    {
        return Err("from_date is later than to_date".to_owned());
    }

    let capture = field(&manifest, "capture", "manifest")?;
    ensure_keys(
        capture,
        &[
            "method",
            "captured_at_utc",
            "http_status",
            "final_url",
            "content_type",
            "expected_release",
            "headers",
            "body_bytes",
            "body_sha256",
            "headers_bytes",
            "headers_sha256",
        ],
        "capture",
    )?;
    if string(capture, "method", "capture")? != "GET" {
        return Err("only GET captures are admissible".to_owned());
    }
    validate_utc_timestamp(string(capture, "captured_at_utc", "capture")?)?;
    if integer(capture, "http_status", "capture")? != 200 {
        return Err("capture HTTP status must be 200".to_owned());
    }
    let final_url = string(capture, "final_url", "capture")?;
    if final_url != source_url {
        return Err("final_url must equal the exact requested URL".to_owned());
    }
    validate_url(final_url, mode, slug)?;
    let content_type = string(capture, "content_type", "capture")?;
    if !content_type
        .to_ascii_lowercase()
        .starts_with("application/json")
    {
        return Err("content_type must be application/json".to_owned());
    }

    if integer(capture, "body_bytes", "capture")? != body_bytes.len() as u64
        || string(capture, "body_sha256", "capture")? != sha256(body_bytes)
    {
        return Err("raw response body byte count/SHA-256 mismatch".to_owned());
    }
    if integer(capture, "headers_bytes", "capture")? != header_bytes.len() as u64
        || string(capture, "headers_sha256", "capture")? != sha256(header_bytes)
    {
        return Err("raw response header byte count/SHA-256 mismatch".to_owned());
    }

    let (status_code, raw_headers) = parse_raw_headers(header_bytes)?;
    if status_code != 200 {
        return Err("raw header block does not end in HTTP 200".to_owned());
    }
    let normalized = field(capture, "headers", "capture")?;
    ensure_keys(
        normalized,
        &["content-type", "x-licence", "date"],
        "capture.headers",
    )?;
    if string(normalized, "content-type", "capture.headers")? != content_type {
        return Err("capture.content_type differs from normalized response header".to_owned());
    }
    for key in ["content-type", "x-licence", "date"] {
        let declared = string(normalized, key, "capture.headers")?;
        let actual = raw_headers
            .get(key)
            .ok_or_else(|| format!("raw header block lacks '{key}'"))?;
        if declared != actual {
            return Err(format!(
                "normalized header '{key}' differs from raw response header"
            ));
        }
    }
    if !raw_headers
        .get("content-type")
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("application/json"))
    {
        return Err("raw content-type is not application/json".to_owned());
    }

    ensure_keys(
        &body,
        &[
            "licence",
            "release",
            "source_view",
            "caveat",
            "suburb",
            "suburb_name",
            "days",
        ],
        "response body",
    )?;
    if string(&body, "source_view", "body")? != "api.suburb_history"
        || string(&body, "suburb", "body")? != slug
    {
        return Err("body source_view/suburb does not match request".to_owned());
    }
    if string(&body, "licence", "body")? != string(normalized, "x-licence", "capture.headers")? {
        return Err("body licence field and X-Licence header differ".to_owned());
    }
    let release = string(&body, "release", "body")?;
    let expected_release = string(capture, "expected_release", "capture")?;
    if release != expected_release {
        return Err("response release differs from capture manifest expected_release".to_owned());
    }
    if mode == "synthetic_fixture" {
        if release != "SYNTHETIC-ONLY"
            || string(normalized, "x-licence", "capture.headers")? != "SYNTHETIC-ONLY"
        {
            return Err("synthetic fixture must use SYNTHETIC-ONLY release/licence".to_owned());
        }
    } else {
        if release.len() != 19 || &release[10..11] != "-" {
            return Err("publisher release id must be YYYY-MM-DD-8hex".to_owned());
        }
        parse_date(&release[..10])?;
        let hash = &release[11..];
        if hash.len() != 8 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(
                "publisher release id hash must be eight hexadecimal characters".to_owned(),
            );
        }
        let licence = string(&body, "licence", "body")?;
        if !licence.contains("CC BY 4.0") {
            return Err("publisher response lacks the expected CC BY 4.0 assertion".to_owned());
        }
    }

    if string(&body, "caveat", "body")?.trim().is_empty()
        || string(&body, "suburb_name", "body")?.trim().is_empty()
    {
        return Err("body caveat/suburb_name must be non-empty".to_owned());
    }

    let rights = field(&manifest, "rights_review", "manifest")?;
    ensure_keys(rights, &["status", "review_reference"], "rights_review")?;
    let rights_status = string(rights, "status", "rights_review")?;
    let review_reference = optional_string(rights, "review_reference", "rights_review")?;
    match mode {
        "synthetic_fixture" => {
            if rights_status != "not_applicable_synthetic" || review_reference.is_some() {
                return Err("synthetic fixture cannot assert publisher rights review".to_owned());
            }
        }
        "publisher_capture" => {
            if rights_status == "approved" && review_reference.is_none() {
                return Err(
                    "approved rights review requires a non-empty review reference".to_owned(),
                );
            }
            if rights_status != "approved" && rights_status != "pending" {
                return Err(
                    "publisher capture rights status must be pending or approved".to_owned(),
                );
            }
            if rights_status == "pending" && review_reference.is_some() {
                return Err("pending rights review cannot carry an approval reference".to_owned());
            }
        }
        _ => return Err("unsupported capture mode".to_owned()),
    }

    let rows = field(&body, "days", "body")?
        .as_array()
        .ok_or_else(|| "body.days must be an array".to_owned())?;
    if rows.is_empty() {
        return Err("body.days must not be empty".to_owned());
    }
    let mut keys = BTreeSet::new();
    let mut assets = BTreeSet::new();
    let mut dates = BTreeSet::new();
    let mut no_report_count = 0usize;
    let mut bad_count = 0usize;
    let mut level_counts: BTreeMap<&str, usize> = BTreeMap::new();

    for (index, row) in rows.iter().enumerate() {
        let context = format!("days[{index}]");
        ensure_keys(
            row,
            &[
                "day",
                "asset_id",
                "asset_name",
                "level",
                "bad",
                "on_bypass",
                "boards",
                "wording",
            ],
            &context,
        )?;
        let day = string(row, "day", &context)?;
        let (year, month, date_day) = parse_date(day)?;
        let day_number = days_from_civil(year, month, date_day);
        if from_day.is_some_and(|start| day_number < start)
            || to_day.is_some_and(|end| day_number > end)
        {
            return Err(format!(
                "{context}: day falls outside requested date bounds"
            ));
        }
        let asset_id = string(row, "asset_id", &context)?;
        if asset_id.trim().is_empty() {
            return Err(format!("{context}: asset_id is empty"));
        }
        if string(row, "asset_name", &context)?.trim().is_empty()
            || string(row, "wording", &context)?.trim().is_empty()
        {
            return Err(format!("{context}: asset_name/wording is empty"));
        }
        let level = string(row, "level", &context)?;
        if !ALLOWED_LEVELS.contains(&level) {
            return Err(format!("{context}: unknown native level '{level}'"));
        }
        let bad = field(row, "bad", &context)?
            .as_bool()
            .ok_or_else(|| format!("{context}: bad must be boolean"))?;
        let expected_bad = matches!(level, "no_water" | "critical");
        if bad != expected_bad {
            return Err(format!("{context}: bad flag contradicts native level"));
        }
        field(row, "on_bypass", &context)?
            .as_bool()
            .ok_or_else(|| format!("{context}: on_bypass must be boolean"))?;
        let boards = integer(row, "boards", &context)?;
        if (level == "no_report" && boards != 0) || (level != "no_report" && boards == 0) {
            return Err(format!(
                "{context}: board count contradicts no_report state"
            ));
        }
        if level == "no_report" {
            no_report_count += 1;
        }
        if bad {
            bad_count += 1;
        }
        *level_counts.entry(level).or_default() += 1;
        if !keys.insert((day.to_owned(), asset_id.to_owned())) {
            return Err(format!("{context}: duplicate (asset_id, day) pair"));
        }
        assets.insert(asset_id);
        dates.insert(day.to_owned());
    }

    let admission_status = if mode == "synthetic_fixture" {
        "not_admitted_synthetic_conformance"
    } else if rights_status == "approved" {
        "rights_review_marked_approved_source_integrity_checked"
    } else {
        "pending_rights_review"
    };
    let claim_ceiling = if mode == "synthetic_fixture" {
        "synthetic_only"
    } else {
        "publisher_reported_asset_status_only"
    };

    Ok(json!({
        "receipt_schema_version": RECEIPT_SCHEMA,
        "capture_mode": mode,
        "evidence_class": evidence_class,
        "integrity_status": "PASS",
        "qualification_status": if mode == "synthetic_fixture" { "NotRun" } else { "capture_integrity_checked_only" },
        "admission_status": admission_status,
        "claim_ceiling": claim_ceiling,
        "publisher_release": release,
        "suburb": slug,
        "row_count": rows.len(),
        "unique_supply_count": assets.len(),
        "unique_day_count": dates.len(),
        "no_report_rows": no_report_count,
        "bad_rows": bad_count,
        "level_counts": level_counts,
        "body_bytes": body_bytes.len(),
        "body_sha256": sha256(body_bytes),
        "headers_bytes": header_bytes.len(),
        "headers_sha256": sha256(header_bytes),
        "capture_manifest_sha256": sha256(manifest_bytes),
        "response_schema_sha256": null,
        "rights_review_status": rights_status,
        "rights_review_reference": review_reference,
        "household_tap_status": "not_established",
        "water_quality": "not_established",
        "physical_restoration": "not_established",
        "causal_dependency": "not_established",
        "point_of_use_delivery": "not_established"
    }))
}

fn read_bytes(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("cannot read '{path}': {error}"))
}

fn read_json(path: &str) -> Result<(Vec<u8>, Value), String> {
    let bytes = read_bytes(path)?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON in '{path}': {error}"))?;
    Ok((bytes, value))
}

fn run(args: &[String]) -> Result<Value, String> {
    if args.len() != 6 {
        return Err(
            "usage: jw-water-intake-t1 BODY_JSON RAW_HEADERS CAPTURE_MANIFEST RESPONSE_SCHEMA CAPTURE_SCHEMA RECEIPT"
                .to_owned(),
        );
    }
    let body = read_bytes(&args[0])?;
    let headers = read_bytes(&args[1])?;
    let manifest_bytes = read_bytes(&args[2])?;
    let (schema_bytes, schema) = read_json(&args[3])?;
    let (capture_schema_bytes, capture_schema) = read_json(&args[4])?;
    if string(&capture_schema, "$schema", "capture manifest schema")?
        != "https://json-schema.org/draft/2020-12/schema"
        || field(
            &capture_schema,
            "additionalProperties",
            "capture manifest schema",
        )?
        .as_bool()
            != Some(false)
    {
        return Err("capture manifest schema must be closed Draft 2020-12".to_owned());
    }
    let mut receipt = validate_capture(&body, &headers, &manifest_bytes, &schema)?;
    let revision = env::var("JW_WATER_INTAKE_T1_REVISION")
        .map_err(|_| "JW_WATER_INTAKE_T1_REVISION must be the exact checkout SHA".to_owned())?;
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(
            "JW_WATER_INTAKE_T1_REVISION must be a 40-character hexadecimal SHA".to_owned(),
        );
    }
    let object = receipt
        .as_object_mut()
        .ok_or_else(|| "internal receipt is not an object".to_owned())?;
    object.insert("implementation_revision".to_owned(), json!(revision));
    object.insert(
        "response_schema_sha256".to_owned(),
        json!(sha256(&schema_bytes)),
    );
    object.insert(
        "capture_manifest_schema_sha256".to_owned(),
        json!(sha256(&capture_schema_bytes)),
    );
    object.insert(
        "cargo_lock_sha256".to_owned(),
        json!(sha256(include_bytes!("../../../../Cargo.lock"))),
    );
    object.insert(
        "package_manifest_sha256".to_owned(),
        json!(sha256(include_bytes!("../../Cargo.toml"))),
    );
    object.insert(
        "implementation_sha256".to_owned(),
        json!(sha256(include_bytes!("jw-water-intake-t1.rs"))),
    );
    object.insert(
        "capture_manifest_sha256".to_owned(),
        json!(sha256(&manifest_bytes)),
    );
    let output = Path::new(&args[5]);
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create receipt directory: {error}"))?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("serialize receipt: {error}"))?,
    )
    .map_err(|error| format!("cannot write receipt '{}': {error}", output.display()))?;
    Ok(receipt)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(receipt) => println!(
            "{}",
            serde_json::to_string_pretty(&receipt).unwrap_or_default()
        ),
        Err(error) => {
            eprintln!("jw-water-intake-t1: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Vec<u8>, Vec<u8>, Vec<u8>, Value) {
        let body = include_bytes!("../../../../docs/research/jw-water-intake-t1/watersearch_suburb_history.synthetic.v0.1.json").to_vec();
        let headers = include_bytes!(
            "../../../../docs/research/jw-water-intake-t1/response_headers.synthetic.v0.1.txt"
        )
        .to_vec();
        let manifest_bytes = include_bytes!(
            "../../../../docs/research/jw-water-intake-t1/capture_manifest.synthetic.v0.1.json"
        )
        .to_vec();
        let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        let schema: Value = serde_json::from_str(include_str!("../../../../docs/research/jw-water-intake-t1/watersearch_suburb_history_response.schema.v0.1.json")).unwrap();
        (
            body,
            headers,
            manifest_bytes,
            json!({"manifest":manifest,"schema":schema}),
        )
    }

    fn refresh_manifest(body: &[u8], headers: &[u8], manifest: &mut Value) -> Vec<u8> {
        manifest["capture"]["body_bytes"] = json!(body.len());
        manifest["capture"]["body_sha256"] = json!(sha256(body));
        manifest["capture"]["headers_bytes"] = json!(headers.len());
        manifest["capture"]["headers_sha256"] = json!(sha256(headers));
        serde_json::to_vec_pretty(manifest).unwrap()
    }

    #[test]
    fn synthetic_fixture_is_integrity_checked_but_not_admitted() {
        let (body, headers, manifest_bytes, bundle) = fixture();
        let receipt =
            validate_capture(&body, &headers, &manifest_bytes, &bundle["schema"]).unwrap();
        assert_eq!(receipt["integrity_status"], "PASS");
        assert_eq!(receipt["evidence_class"], "synthetic_only");
        assert_eq!(
            receipt["admission_status"],
            "not_admitted_synthetic_conformance"
        );
        assert_eq!(receipt["row_count"], 4);
        assert_eq!(receipt["unique_supply_count"], 2);
        assert_eq!(receipt["no_report_rows"], 1);
        assert_eq!(receipt["bad_rows"], 2);
        assert_eq!(receipt["household_tap_status"], "not_established");
    }

    #[test]
    fn tampered_body_is_rejected_by_sha256() {
        let (mut body, headers, manifest_bytes, bundle) = fixture();
        body.push(b' ');
        assert!(
            validate_capture(&body, &headers, &manifest_bytes, &bundle["schema"])
                .unwrap_err()
                .contains("body byte count/SHA-256 mismatch")
        );
    }

    #[test]
    fn no_report_cannot_be_recoded_as_bad_or_normal() {
        let (body, headers, manifest_bytes, bundle) = fixture();
        let mut value: Value = serde_json::from_slice(&body).unwrap();
        value["days"][1]["bad"] = json!(true);
        let changed = serde_json::to_vec_pretty(&value).unwrap();
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        let refreshed = refresh_manifest(&changed, &headers, &mut manifest);
        assert!(
            validate_capture(&changed, &headers, &refreshed, &bundle["schema"])
                .unwrap_err()
                .contains("bad flag contradicts native level")
        );
    }

    #[test]
    fn duplicate_supply_day_is_rejected() {
        let (body, headers, manifest_bytes, bundle) = fixture();
        let mut value: Value = serde_json::from_slice(&body).unwrap();
        value["days"][3]["asset_id"] = json!("synthetic-supply-a");
        let changed = serde_json::to_vec_pretty(&value).unwrap();
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        let refreshed = refresh_manifest(&changed, &headers, &mut manifest);
        assert!(
            validate_capture(&changed, &headers, &refreshed, &bundle["schema"])
                .unwrap_err()
                .contains("duplicate (asset_id, day)")
        );
    }

    #[test]
    fn x_licence_must_match_the_raw_header_and_body() {
        let (body, headers, manifest_bytes, bundle) = fixture();
        let changed_headers = String::from_utf8(headers.clone())
            .unwrap()
            .replace("SYNTHETIC-ONLY", "DIFFERENT-LICENCE")
            .into_bytes();
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        let refreshed = refresh_manifest(&body, &changed_headers, &mut manifest);
        assert!(
            validate_capture(&body, &changed_headers, &refreshed, &bundle["schema"])
                .unwrap_err()
                .contains("normalized header 'x-licence'")
        );
    }

    #[test]
    fn out_of_range_dates_are_rejected() {
        let (body, headers, manifest_bytes, bundle) = fixture();
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        manifest["source"]["from_date"] = json!("2026-02-02");
        manifest["capture"]["body_sha256"] = json!(sha256(&body));
        manifest["capture"]["headers_sha256"] = json!(sha256(&headers));
        let refreshed = serde_json::to_vec_pretty(&manifest).unwrap();
        assert!(
            validate_capture(&body, &headers, &refreshed, &bundle["schema"])
                .unwrap_err()
                .contains("outside requested date bounds")
        );
    }

    #[test]
    fn warned_is_a_distinct_non_bad_level_even_when_on_bypass() {
        let (body, headers, manifest_bytes, bundle) = fixture();
        let mut value: Value = serde_json::from_slice(&body).unwrap();
        value["days"][1]["level"] = json!("warned");
        value["days"][1]["bad"] = json!(false);
        value["days"][1]["on_bypass"] = json!(true);
        value["days"][1]["boards"] = json!(1);
        value["days"][1]["wording"] =
            json!("Synthetic warning that poor pressure to no water may occur.");
        let changed = serde_json::to_vec_pretty(&value).unwrap();
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        let refreshed = refresh_manifest(&changed, &headers, &mut manifest);
        let receipt = validate_capture(&changed, &headers, &refreshed, &bundle["schema"]).unwrap();
        assert_eq!(receipt["level_counts"]["warned"], 1);
        assert_eq!(receipt["bad_rows"], 2);
    }

    #[test]
    fn response_release_must_match_manifest_pin() {
        let (body, headers, manifest_bytes, bundle) = fixture();
        let mut body_value: Value = serde_json::from_slice(&body).unwrap();
        body_value["release"] = json!("SYNTHETIC-DIFFERENT");
        let changed = serde_json::to_vec_pretty(&body_value).unwrap();
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        let refreshed = refresh_manifest(&changed, &headers, &mut manifest);
        assert!(
            validate_capture(&changed, &headers, &refreshed, &bundle["schema"])
                .unwrap_err()
                .contains("response release differs from capture manifest expected_release")
        );
    }

    #[test]
    fn publisher_capture_requires_public_https_origin_and_rights_review_state() {
        let (body, headers, manifest_bytes, bundle) = fixture();
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        manifest["capture"]["expected_release"] = json!("2026-11-17-deadbeef");
        manifest["capture_mode"] = json!("publisher_capture");
        manifest["evidence_class"] = json!("publisher_reported_asset_status");
        manifest["source"]["url"] =
            json!("https://watersearch.co.za/api/v1/suburbs/synthetic-suburb/history");
        manifest["capture"]["final_url"] =
            json!("https://watersearch.co.za/api/v1/suburbs/synthetic-suburb/history");
        manifest["rights_review"]["status"] = json!("pending");
        let refreshed = serde_json::to_vec_pretty(&manifest).unwrap();
        assert!(
            validate_capture(&body, &headers, &refreshed, &bundle["schema"])
                .unwrap_err()
                .contains("publisher release id")
        );
    }

    #[test]
    fn publisher_capture_integrity_does_not_auto_admit_pending_rights() {
        let (body, headers, manifest_bytes, bundle) = fixture();
        let mut body_value: Value = serde_json::from_slice(&body).unwrap();
        body_value["release"] = json!("2026-11-17-deadbeef");
        body_value["licence"] =
            json!("CC BY 4.0. Credit Johannesburg Water for the underlying facts.");
        let publisher_body = serde_json::to_vec_pretty(&body_value).unwrap();

        let publisher_licence = "CC BY 4.0. Credit Johannesburg Water for the underlying facts.";
        let publisher_headers = String::from_utf8(headers)
            .unwrap()
            .replace("SYNTHETIC-ONLY", publisher_licence)
            .into_bytes();

        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        manifest["capture_mode"] = json!("publisher_capture");
        manifest["evidence_class"] = json!("publisher_reported_asset_status");
        manifest["source"]["url"] =
            json!("https://watersearch.co.za/api/v1/suburbs/synthetic-suburb/history");
        manifest["capture"]["final_url"] =
            json!("https://watersearch.co.za/api/v1/suburbs/synthetic-suburb/history");
        manifest["capture"]["headers"]["x-licence"] = json!(publisher_licence);
        manifest["rights_review"]["status"] = json!("pending");
        let pending_manifest = refresh_manifest(&publisher_body, &publisher_headers, &mut manifest);

        let pending = validate_capture(
            &publisher_body,
            &publisher_headers,
            &pending_manifest,
            &bundle["schema"],
        )
        .unwrap();
        assert_eq!(pending["integrity_status"], "PASS");
        assert_eq!(pending["evidence_class"], "publisher_reported_asset_status");
        assert_eq!(pending["admission_status"], "pending_rights_review");
        assert_eq!(pending["household_tap_status"], "not_established");

        manifest["rights_review"]["status"] = json!("approved");
        manifest["rights_review"]["review_reference"] = json!("synthetic-review-reference-only");
        let approved_manifest =
            refresh_manifest(&publisher_body, &publisher_headers, &mut manifest);
        let approved = validate_capture(
            &publisher_body,
            &publisher_headers,
            &approved_manifest,
            &bundle["schema"],
        )
        .unwrap();
        assert_eq!(
            approved["admission_status"],
            "rights_review_marked_approved_source_integrity_checked"
        );
        assert_eq!(
            approved["claim_ceiling"],
            "publisher_reported_asset_status_only"
        );
    }

    #[test]
    fn release_ids_are_date_bound_and_hash_suffixed() {
        let mut parts = "2026-11-17-ee99662f".split('-');
        let date = format!(
            "{}-{}-{}",
            parts.next().unwrap(),
            parts.next().unwrap(),
            parts.next().unwrap()
        );
        let hash = parts.next().unwrap();
        assert!(parse_date(&date).is_ok());
        assert_eq!(hash.len(), 8);
        assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(parse_date("2026-02-30").is_err());
    }
}
