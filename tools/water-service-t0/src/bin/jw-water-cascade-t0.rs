// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only T0 evaluator for the Johannesburg water service cascade contract.
//! Synthetic conformance only: this binary has no network, control, or actuation path.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

const BENCHMARK_ID: &str = "JW-WATER-CASCADE-T0";
const CLAIM_CEILING: &str = "synthetic_contract_only";
const MAX_C41_NO_REPORT_GAP: usize = 4;

fn field<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("{context}: missing field '{key}'"))
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

fn case_id(case: &Value) -> Result<&str, String> {
    string(case, "case_id", "case")
}

fn reports(case: &Value) -> Result<&Vec<Value>, String> {
    let id = case_id(case)?;
    field(case, "utility_reports", id)?
        .as_array()
        .ok_or_else(|| format!("{id}: utility_reports must be an array"))
}

fn status_vector(case: &Value) -> Result<Vec<String>, String> {
    reports(case)?
        .iter()
        .enumerate()
        .map(|(index, row)| {
            string(row, "status", &format!("{} report {index}", case_id(case)?)).map(str::to_owned)
        })
        .collect()
}

fn severe(status: &str) -> bool {
    matches!(status, "no_water" | "critical")
}

/// Returns the number of publisher-derived episodes and the largest no_report gap
/// between severe reports. The input is a conformance fixture, not raw utility data.
fn c41_counts(case: &Value) -> Result<(usize, usize), String> {
    let mut episodes = 0usize;
    let mut gap = 0usize;
    let mut maximum_gap = 0usize;
    let mut previous_severe = false;

    for row in reports(case)? {
        let status = string(row, "status", case_id(case)?)?;
        if severe(status) {
            if !previous_severe || gap > MAX_C41_NO_REPORT_GAP {
                episodes += 1;
            }
            maximum_gap = maximum_gap.max(gap);
            previous_severe = true;
            gap = 0;
        } else if status == "no_report" && previous_severe {
            gap += 1;
        } else if previous_severe {
            previous_severe = false;
            gap = 0;
        }
    }
    Ok((episodes, maximum_gap))
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

/// Proleptic Gregorian civil date to days from 1970-01-01.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let year_of_era = y - era * 400;
    let month_prime = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Parse an ISO-8601 timestamp with an explicit Z or numeric UTC offset.
fn parse_offset_timestamp(value: &str) -> Result<i64, String> {
    let (date, time_and_zone) = value
        .split_once('T')
        .ok_or_else(|| format!("timestamp lacks T: '{value}'"))?;
    let (year, month, day) = parse_date(date)?;
    let (clock, offset_minutes) = if let Some(clock) = time_and_zone.strip_suffix('Z') {
        (clock, 0i64)
    } else {
        let split_at = time_and_zone
            .char_indices()
            .rev()
            .find(|(_, ch)| *ch == '+' || *ch == '-')
            .map(|(index, _)| index)
            .ok_or_else(|| format!("timestamp lacks explicit UTC offset: '{value}'"))?;
        let clock = &time_and_zone[..split_at];
        let offset = &time_and_zone[split_at..];
        if offset.len() != 6 || &offset[3..4] != ":" {
            return Err(format!("invalid UTC offset in '{value}'"));
        }
        let hours = offset[1..3]
            .parse::<i64>()
            .map_err(|_| format!("invalid offset hours in '{value}'"))?;
        let minutes = offset[4..6]
            .parse::<i64>()
            .map_err(|_| format!("invalid offset minutes in '{value}'"))?;
        if hours > 23 || minutes > 59 {
            return Err(format!("UTC offset out of range in '{value}'"));
        }
        let sign = if offset.starts_with('-') { -1 } else { 1 };
        (clock, sign * (hours * 60 + minutes))
    };

    let clock_parts: Vec<&str> = clock.split(':').collect();
    if clock_parts.len() != 3 {
        return Err(format!("timestamp must include HH:MM:SS: '{value}'"));
    }
    let hour = clock_parts[0]
        .parse::<i64>()
        .map_err(|_| format!("invalid hour in '{value}'"))?;
    let minute = clock_parts[1]
        .parse::<i64>()
        .map_err(|_| format!("invalid minute in '{value}'"))?;
    let second = clock_parts[2]
        .parse::<i64>()
        .map_err(|_| format!("invalid second in '{value}'"))?;
    if hour > 23 || minute > 59 || second != 0 {
        return Err(format!(
            "clock time invalid or fractional precision unsupported in '{value}'"
        ));
    }
    Ok(days_from_civil(year, month, day) * 1_440 + hour * 60 + minute - offset_minutes)
}

/// A daily SAST return row bounds the event to a whole valid-day interval.
fn notice_delay_bounds(case: &Value) -> Result<Option<(i64, i64)>, String> {
    let notice = field(case, "notice", case_id(case)?)?;
    if notice.is_null() {
        return Ok(None);
    }
    let announcement = string(notice, "announced_completion_at", case_id(case)?)?;
    let return_date = string(notice, "first_return_valid_date", case_id(case)?)?;
    let announced_minute = parse_offset_timestamp(announcement)?;
    let (year, month, day) = parse_date(return_date)?;
    let return_start = days_from_civil(year, month, day) * 1_440 - 120; // SAST = UTC+02:00
    let return_end = return_start + 1_440;
    let lower_minutes = return_start - announced_minute;
    let upper_minutes = return_end - announced_minute;
    if lower_minutes < 0 || upper_minutes <= lower_minutes {
        return Err(format!("{}: invalid return-day interval", case_id(case)?));
    }
    if lower_minutes % 60 != 0 || upper_minutes % 60 != 0 {
        return Err(format!(
            "{}: delay bounds are not whole hours",
            case_id(case)?
        ));
    }
    Ok(Some((lower_minutes / 60, upper_minutes / 60)))
}

fn evaluate_case(case: &Value) -> Result<Value, String> {
    let id = case_id(case)?;
    let context = id.to_owned();
    let family = string(case, "family", &context)?;
    let status_values = status_vector(case)?;
    let disposition;
    let mut reported_episode_count: Option<usize> = None;
    let mut no_board_gap: Option<usize> = None;
    let mut censoring = "NOT_APPLICABLE";
    let mut duration_lower_bound: Option<f64> = None;
    let mut asset_join = "NOT_ATTEMPTED";
    let mut temporal_join = "NOT_ATTEMPTED";
    let mut dependency_conclusion = "NO_CAUSAL_CLAIM";
    let mut point_of_use_conclusion = "NOT_MEASURED";
    let mut delay_lower: Option<i64> = None;
    let mut delay_upper: Option<i64> = None;

    match family {
        "no_report" => {
            if status_values.iter().any(|status| status == "no_report") {
                disposition = "NO_REPORT_PRESERVED";
            } else {
                return Err(format!("{id}: no_report family lacks no_report"));
            }
        }
        "reported_status" => disposition = "REPORTED_STATUS_ONLY",
        "multi_supply" => {
            let supply_ids: BTreeSet<&str> = reports(case)?
                .iter()
                .map(|row| string(row, "supply_id", &context))
                .collect::<Result<_, _>>()?;
            if supply_ids.len() < 2 {
                return Err(format!("{id}: fewer than two supply IDs"));
            }
            disposition = "PER_SUPPLY_VECTOR_ONLY";
        }
        "bypass" => {
            if !reports(case)?
                .iter()
                .any(|row| row.get("bypass").and_then(Value::as_bool) == Some(true))
            {
                return Err(format!("{id}: bypass case lacks bypass=true"));
            }
            disposition = "BYPASS_STATUS_SEPARATE";
        }
        "scheduled_closure" => {
            if !reports(case)?
                .iter()
                .any(|row| row.get("scheduled_closure").and_then(Value::as_bool) == Some(true))
            {
                return Err(format!("{id}: scheduled-closure flag missing"));
            }
            disposition = "SCHEDULED_CLOSURE_SEPARATE";
        }
        "c41_bridge" | "censored_duration" => {
            let outage = field(case, "outage", &context)?;
            if outage.is_null() || string(outage, "method_id", &context)? != "C41" {
                return Err(format!("{id}: C41 outage metadata missing or unsupported"));
            }
            let (episodes, gap) = c41_counts(case)?;
            let declared_episodes = field(outage, "episode_count", &context)?
                .as_u64()
                .ok_or_else(|| format!("{id}: episode_count must be an integer"))?
                as usize;
            let declared_gap = field(outage, "no_board_days_between_severe_reports", &context)?
                .as_u64()
                .ok_or_else(|| format!("{id}: no_board gap must be an integer"))?
                as usize;
            if episodes != declared_episodes || gap != declared_gap {
                return Err(format!(
                    "{id}: C41 mismatch: derived episodes={episodes}, gap={gap}; declared episodes={declared_episodes}, gap={declared_gap}"
                ));
            }
            reported_episode_count = Some(episodes);
            no_board_gap = Some(gap);
            let end_kind = string(outage, "reported_end_kind", &context)?;
            if !matches!(
                end_kind,
                "right_censored" | "fair_off_bypass" | "split_at_bridge_cap"
            ) {
                return Err(format!("{id}: unsupported reported_end_kind '{end_kind}'"));
            }
            if end_kind == "split_at_bridge_cap" && gap <= MAX_C41_NO_REPORT_GAP {
                return Err(format!(
                    "{id}: split_at_bridge_cap requires more than four no_report days"
                ));
            }
            if end_kind == "fair_off_bypass" {
                let last_reported = reports(case)?
                    .iter()
                    .rev()
                    .find(|row| row.get("status").and_then(Value::as_str) != Some("no_report"))
                    .ok_or_else(|| format!("{id}: fair/off-bypass return row missing"))?;
                if string(last_reported, "status", &context)? != "fair"
                    || field(last_reported, "bypass", &context)?.as_bool() != Some(false)
                {
                    return Err(format!("{id}: reported end is not fair/off-bypass"));
                }
            }
            if end_kind == "right_censored" {
                censoring = "RIGHT_CENSORED";
                duration_lower_bound = field(outage, "duration_hours_lower_bound", &context)?
                    .as_f64()
                    .filter(|value| value.is_finite() && *value >= 0.0);
                if duration_lower_bound.is_none() {
                    return Err(format!(
                        "{id}: censored duration lower bound missing/invalid"
                    ));
                }
            } else {
                censoring = "NOT_CENSORED";
            }
            disposition = if family == "censored_duration" {
                "RIGHT_CENSORED_LOWER_BOUND"
            } else if gap > MAX_C41_NO_REPORT_GAP {
                "C41_GAP_OVER_CAP_NOT_BRIDGED"
            } else {
                "C41_GAP_WITHIN_CAP"
            };
        }
        "notice_delay" | "asset_join" => {
            let notice = field(case, "notice", &context)?;
            if notice.is_null() {
                return Err(format!("{id}: notice record missing"));
            }
            let notice_supply = string(notice, "supply_id", &context)?;
            let return_date = string(notice, "first_return_valid_date", &context)?;
            let exact_supply_match = reports(case)?
                .iter()
                .all(|row| row.get("supply_id").and_then(Value::as_str) == Some(notice_supply));
            let qualifying_return_row = reports(case)?.iter().any(|row| {
                row.get("supply_id").and_then(Value::as_str) == Some(notice_supply)
                    && row.get("valid_date").and_then(Value::as_str) == Some(return_date)
                    && row.get("status").and_then(Value::as_str) == Some("fair")
                    && row.get("bypass").and_then(Value::as_bool) == Some(false)
            });
            if exact_supply_match && qualifying_return_row {
                asset_join = "EXACT_SUPPLY_MATCH";
                let (lower, upper) = notice_delay_bounds(case)?
                    .ok_or_else(|| format!("{id}: notice return interval missing"))?;
                delay_lower = Some(lower);
                delay_upper = Some(upper);
                temporal_join = "DATE_INTERVAL_ORDERED_AFTER_NOTICE";
                disposition = "NOTICE_TO_REPORTED_RETURN_DELAY";
            } else {
                asset_join = "REJECTED_SUPPLY_MISMATCH";
                temporal_join = "NOT_JOINED";
                disposition = "SUPPLY_ID_JOIN_REJECTED";
            }
        }
        "timebase" => {
            let hydrometric = field(case, "hydrometric", &context)?;
            if string(hydrometric, "time_basis", &context)? == "unspecified" {
                temporal_join = "INDETERMINATE_TIMEBASE";
                disposition = "TIMEBASE_INDETERMINATE";
            } else {
                return Err(format!("{id}: expected an unspecified timebase"));
            }
        }
        "hydrometric_semantics" => {
            let hydrometric = field(case, "hydrometric", &context)?;
            if string(hydrometric, "parameter", &context)? == "stage"
                && optional_string(hydrometric, "rating_curve_ref", &context)?.is_none()
            {
                temporal_join = "NOT_JOINED";
                disposition = "STAGE_NOT_DISCHARGE";
            } else {
                return Err(format!(
                    "{id}: stage-without-rating negative control failed"
                ));
            }
        }
        "dependency" => {
            let dependency = field(case, "dependency", &context)?;
            if string(dependency, "kind", &context)? == "unknown" {
                dependency_conclusion = "UNKNOWN_NO_CAUSAL_CLAIM";
                temporal_join = "COARSE_DAILY_ALIGNMENT";
                disposition = "UNKNOWN_DEPENDENCY_ASSOCIATION_ONLY";
            } else {
                return Err(format!(
                    "{id}: unknown dependency unexpectedly has evidence"
                ));
            }
        }
        "point_of_use" => {
            let local = field(case, "point_of_use", &context)?;
            if local.is_null() || string(local, "evidence_class", &context)? != "synthetic_only" {
                return Err(format!(
                    "{id}: local synthetic point-of-use observation missing"
                ));
            }
            point_of_use_conclusion = "SYNTHETIC_LOCAL_PRESSURE_FLOW_ONLY";
            temporal_join = "LOCAL_MEASUREMENT_ONLY";
            disposition = "LOCAL_POINT_OF_USE_SYNTHETIC_ONLY";
        }
        "claim_ceiling" => {
            temporal_join = "NOT_JOINED";
            disposition = "SYNTHETIC_TO_FIELD_PROMOTION_REJECTED";
        }
        other => return Err(format!("{id}: unknown family '{other}'")),
    }

    let requested_claim = string(case, "requested_claim", &context)?;
    let claim_result = if requested_claim == "field_qualified" {
        "REJECT_FIELD_CLAIM"
    } else {
        "RETAIN_SYNTHETIC_CEILING"
    };

    Ok(json!({
        "case_id": id,
        "expected": {
            "disposition": disposition,
            "reported_status_vector": status_values,
            "reported_episode_count": reported_episode_count,
            "no_board_days_between_severe_reports": no_board_gap,
            "censoring": censoring,
            "duration_hours_lower_bound": duration_lower_bound,
            "asset_join": asset_join,
            "temporal_join": temporal_join,
            "dependency_conclusion": dependency_conclusion,
            "point_of_use_conclusion": point_of_use_conclusion,
            "claim_result": claim_result,
            "claim_ceiling": CLAIM_CEILING,
            "notice_delay_lower_bound_hours": delay_lower,
            "notice_delay_upper_bound_hours": delay_upper
        }
    }))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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

fn validate_source_manifest(manifest: &Value) -> Result<(), String> {
    let sources = field(manifest, "sources", "source manifest")?
        .as_array()
        .ok_or_else(|| "source manifest: sources must be an array".to_owned())?;
    for source in sources {
        if field(source, "raw_bytes_acquired", "source manifest entry")?.as_bool() != Some(false) {
            return Err("T0 source manifest must not claim source bytes are acquired".to_owned());
        }
        if !field(source, "sha256", "source manifest entry")?.is_null() {
            return Err("T0 source manifest must keep unacquired source SHA-256 null".to_owned());
        }
    }
    Ok(())
}

fn validate_schema_markers(input_schema: &Value, oracle_schema: &Value) -> Result<(), String> {
    for (name, schema) in [("input", input_schema), ("oracle", oracle_schema)] {
        let dialect = string(schema, "$schema", &format!("{name} schema"))?;
        if !dialect.ends_with("/draft/2020-12/schema") {
            return Err(format!("{name} schema is not Draft 2020-12"));
        }
        if field(schema, "additionalProperties", &format!("{name} schema"))?.as_bool()
            != Some(false)
        {
            return Err(format!("{name} schema root must be closed"));
        }
    }
    Ok(())
}

fn run(args: &[String]) -> Result<Value, String> {
    if args.len() != 6 {
        return Err("usage: jw-water-cascade-t0 INPUT ORACLE INPUT_SCHEMA ORACLE_SCHEMA SOURCE_MANIFEST RECEIPT".to_owned());
    }
    let (input_bytes, input) = read_json(&args[0])?;
    let (oracle_bytes, oracle) = read_json(&args[1])?;
    let (input_schema_bytes, input_schema) = read_json(&args[2])?;
    let (oracle_schema_bytes, oracle_schema) = read_json(&args[3])?;
    let (manifest_bytes, source_manifest) = read_json(&args[4])?;

    if string(&input, "benchmark_id", "input")? != BENCHMARK_ID
        || string(&oracle, "benchmark_id", "oracle")? != BENCHMARK_ID
    {
        return Err("benchmark ID mismatch".to_owned());
    }
    if string(&input, "evidence_class", "input")? != "synthetic_only"
        || string(&oracle, "evidence_class", "oracle")? != "synthetic_only"
    {
        return Err("T0 evidence class must remain synthetic_only".to_owned());
    }
    let source_context = field(&input, "source_context", "input")?;
    if string(source_context, "time_zone", "source_context")? != "Africa/Johannesburg" {
        return Err("notice valid-day interval requires Africa/Johannesburg time zone".to_owned());
    }
    validate_schema_markers(&input_schema, &oracle_schema)?;
    validate_source_manifest(&source_manifest)?;

    let input_cases = field(&input, "cases", "input")?
        .as_array()
        .ok_or_else(|| "input cases must be an array".to_owned())?;
    let oracle_cases = field(&oracle, "cases", "oracle")?
        .as_array()
        .ok_or_else(|| "oracle cases must be an array".to_owned())?;
    if input_cases.len() != 15 || oracle_cases.len() != 15 {
        return Err("JW-WATER-CASCADE-T0 requires exactly 15 frozen cases".to_owned());
    }
    let input_ids: BTreeSet<String> = input_cases
        .iter()
        .map(case_id)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(str::to_owned)
        .collect();
    let oracle_ids: BTreeSet<String> = oracle_cases
        .iter()
        .map(case_id)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(str::to_owned)
        .collect();
    if input_ids.len() != input_cases.len() || oracle_ids.len() != oracle_cases.len() {
        return Err("duplicate case ID in input or oracle".to_owned());
    }
    if input_ids != oracle_ids {
        return Err("input/oracle case identity mismatch".to_owned());
    }

    let oracle_by_id: BTreeMap<&str, &Value> = oracle_cases
        .iter()
        .map(|case| Ok((case_id(case)?, case)))
        .collect::<Result<_, String>>()?;
    let mut results = Vec::with_capacity(input_cases.len());
    let mut mismatches = Vec::new();
    for case in input_cases {
        let actual = evaluate_case(case)?;
        let id = case_id(case)?;
        let expected = oracle_by_id
            .get(id)
            .ok_or_else(|| format!("{id}: oracle case missing"))?;
        let expected_object = field(expected, "expected", id)?;
        let actual_object = field(&actual, "expected", id)?;
        if actual_object != expected_object {
            mismatches
                .push(json!({"case_id":id,"actual":actual_object,"expected":expected_object}));
        }
        results.push(actual);
    }
    if !mismatches.is_empty() {
        return Err(format!(
            "{} oracle mismatch(es): {}",
            mismatches.len(),
            serde_json::to_string(&mismatches).unwrap_or_default()
        ));
    }

    results.sort_by(|left, right| {
        left.get("case_id")
            .and_then(Value::as_str)
            .cmp(&right.get("case_id").and_then(Value::as_str))
    });
    let revision = env::var("JW_WATER_CASCADE_REVISION")
        .map_err(|_| "JW_WATER_CASCADE_REVISION must be the exact checkout SHA".to_owned())?;
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("JW_WATER_CASCADE_REVISION must be a 40-character hexadecimal SHA".to_owned());
    }

    let case_count = results.len();
    let receipt = json!({
        "receipt_schema_version": "jw-water-cascade-t0-receipt-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "implementation_revision": revision,
        "evidence_class": "synthetic_only",
        "qualification_status": "NotRun",
        "claim_ceiling": CLAIM_CEILING,
        "case_count": case_count,
        "matched_case_count": case_count,
        "mismatch_count": 0,
        "input_sha256": sha256(&input_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "input_schema_sha256": sha256(&input_schema_bytes),
        "oracle_schema_sha256": sha256(&oracle_schema_bytes),
        "source_manifest_sha256": sha256(&manifest_bytes),
        "cargo_lock_sha256": sha256(include_bytes!("../../../../Cargo.lock")),
        "package_manifest_sha256": sha256(include_bytes!("../../Cargo.toml")),
        "implementation_sha256": sha256(include_bytes!("jw-water-cascade-t0.rs")),
        "case_results": results
    });
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

    Ok(json!({
        "benchmark_id": BENCHMARK_ID,
        "case_count": case_count,
        "matched_case_count": case_count,
        "mismatch_count": 0,
        "qualification_status": "NotRun",
        "evidence_class": "synthetic_only",
        "receipt_path": output.display().to_string(),
        "implementation_revision": revision
    }))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(summary) => println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_default()
        ),
        Err(error) => {
            eprintln!("jw-water-cascade-t0: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_math_handles_leap_day_and_epoch() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(
            days_from_civil(2024, 3, 1) - days_from_civil(2024, 2, 28),
            2
        );
        assert_eq!(
            days_from_civil(2025, 3, 1) - days_from_civil(2025, 2, 28),
            1
        );
    }

    #[test]
    fn offset_timestamp_converts_to_utc_minutes() {
        let expected = days_from_civil(2026, 2, 4) * 1_440 + 16 * 60;
        assert_eq!(
            parse_offset_timestamp("2026-02-04T18:00:00+02:00").unwrap(),
            expected
        );
        assert_eq!(
            parse_offset_timestamp("2026-02-04T16:00:00Z").unwrap(),
            expected
        );
    }

    #[test]
    fn daily_return_row_produces_interval_not_fake_timestamp() {
        let announced = parse_offset_timestamp("2026-02-04T18:00:00+02:00").unwrap();
        let start = days_from_civil(2026, 2, 6) * 1_440 - 120;
        assert_eq!((start - announced) / 60, 30);
        assert_eq!((start + 1_440 - announced) / 60, 54);
    }

    #[test]
    fn c41_four_day_gap_stays_one_episode() {
        let rows = [
            "no_water",
            "no_report",
            "no_report",
            "no_report",
            "no_report",
            "no_water",
        ];
        assert_eq!(c41_from_statuses(&rows), (1, 4));
    }

    #[test]
    fn c41_five_day_gap_splits_episodes() {
        let rows = [
            "no_water",
            "no_report",
            "no_report",
            "no_report",
            "no_report",
            "no_report",
            "no_water",
        ];
        assert_eq!(c41_from_statuses(&rows), (2, 5));
    }

    #[test]
    fn source_manifest_does_not_claim_acquired_bytes() {
        let manifest: Value = serde_json::from_str(include_str!(
            "../../../../docs/research/jw-water-cascade/jw_water_cascade_t0_source_manifest.v0.1.json"
        ))
        .unwrap();
        assert!(validate_source_manifest(&manifest).is_ok());
    }

    #[test]
    fn all_fifteen_cases_match_the_separate_oracle() {
        let input: Value = serde_json::from_str(include_str!(
            "../../../../docs/research/jw-water-cascade/jw_water_cascade_t0_input.v0.1.json"
        ))
        .unwrap();
        let oracle: Value = serde_json::from_str(include_str!(
            "../../../../docs/research/jw-water-cascade/jw_water_cascade_t0_oracle.v0.1.json"
        ))
        .unwrap();
        let expected: BTreeMap<&str, &Value> = oracle["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|case| (case["case_id"].as_str().unwrap(), &case["expected"]))
            .collect();
        let mut count = 0usize;
        for case in input["cases"].as_array().unwrap() {
            let result = evaluate_case(case).unwrap();
            let id = result["case_id"].as_str().unwrap();
            assert_eq!(&result["expected"], *expected.get(id).unwrap(), "case {id}");
            count += 1;
        }
        assert_eq!(count, 15);
    }

    fn c41_from_statuses(statuses: &[&str]) -> (usize, usize) {
        let mut episodes = 0usize;
        let mut gap = 0usize;
        let mut max_gap = 0usize;
        let mut active = false;
        for status in statuses {
            if severe(status) {
                if !active || gap > MAX_C41_NO_REPORT_GAP {
                    episodes += 1;
                }
                max_gap = max_gap.max(gap);
                active = true;
                gap = 0;
            } else if *status == "no_report" && active {
                gap += 1;
            } else if active {
                active = false;
                gap = 0;
            }
        }
        (episodes, max_gap)
    }
}
