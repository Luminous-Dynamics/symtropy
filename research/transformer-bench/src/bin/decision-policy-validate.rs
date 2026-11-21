//! Validate a complete, controlled manufacturing action×outcome utility policy.
//! Utility values are externally approved; this tool checks policy structure, scope,
//! effective dates and release-prohibited outcome invariants. It does not authorize release.
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const HEADER: [&str; 20] = [
    "policy_id", "policy_revision", "decision_case_id", "product_family", "test_id",
    "measurand", "unit", "outcome_class", "release_prohibited", "action_id",
    "is_release_action", "utility_value", "utility_unit", "policy_source_artifact_id",
    "policy_source_sha256", "approval_artifact_id", "approval_sha256",
    "effective_from_utc", "effective_to_utc", "safety_policy_revision",
];

#[derive(Clone, Debug)]
struct Row {
    policy_id: String,
    revision: String,
    decision_case: String,
    family: String,
    test_id: String,
    measurand: String,
    unit: String,
    outcome: String,
    release_prohibited: bool,
    action: String,
    is_release: bool,
    utility: f64,
    utility_unit: String,
    source_artifact: String,
    source_sha256: String,
    approval_artifact: String,
    approval_sha256: String,
    effective_from: String,
    effective_to: String,
    safety_revision: String,
}

#[derive(Clone, Debug)]
struct Report {
    policy_id: String,
    revision: String,
    decision_case: String,
    action_count: usize,
    outcome_count: usize,
    cell_count: usize,
    status: String,
    blockers: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("decision-policy-validate: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(format!(
            "usage: {} <policy.csv> <as-of-utc> <report.csv>",
            args.first().map(String::as_str).unwrap_or("decision-policy-validate")
        ));
    }
    let rows = read_policy(&args[1])?;
    let report = validate(&rows, &args[2])?;
    write_report(&args[3], &report)?;
    eprintln!(
        "decision policy {} revision {}: status={} actions={} outcomes={} cells={} blockers={}",
        report.policy_id, report.revision, report.status, report.action_count,
        report.outcome_count, report.cell_count,
        if report.blockers.is_empty() { "NONE".to_string() } else { report.blockers.join(";") }
    );
    Ok(())
}

fn parse_bool(value: &str, field: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{field}: expected true or false")),
    }
}

fn required(value: &str, field: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Err(format!("{field}: empty value"))
    } else {
        Ok(value.trim().to_string())
    }
}

fn finite(value: &str, field: &str) -> Result<f64, String> {
    let parsed: f64 = value.parse().map_err(|_| format!("{field}: invalid number {value:?}"))?;
    if parsed.is_finite() { Ok(parsed) } else { Err(format!("{field}: non-finite number")) }
}

fn sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

// Strict canonical UTC parser sufficient for comparison and Gregorian date validation.
fn utc_seconds(value: &str, field: &str) -> Result<i64, String> {
    let b = value.as_bytes();
    if b.len() != 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T'
        || b[13] != b':' || b[16] != b':' || b[19] != b'Z' {
        return Err(format!("{field}: expected YYYY-MM-DDTHH:MM:SSZ"));
    }
    let parse = |range: std::ops::Range<usize>| -> Result<i32, String> {
        if !b[range.clone()].iter().all(u8::is_ascii_digit) {
            return Err(format!("{field}: timestamp contains a non-digit"));
        }
        std::str::from_utf8(&b[range]).map_err(|_| format!("{field}: invalid UTF-8"))?
            .parse().map_err(|_| format!("{field}: invalid numeric field"))
    };
    let year = parse(0..4)?;
    let month = parse(5..7)?;
    let day = parse(8..10)?;
    let hour = parse(11..13)?;
    let minute = parse(14..16)?;
    let second = parse(17..19)?;
    if !(1900..=2200).contains(&year) || !(1..=12).contains(&month)
        || !(0..=23).contains(&hour) || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second) {
        return Err(format!("{field}: timestamp field out of range"));
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => unreachable!(),
    };
    if !(1..=month_days).contains(&day) {
        return Err(format!("{field}: impossible calendar date"));
    }
    let mut days = 0_i64;
    if year >= 1970 {
        for y in 1970..year {
            days += if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        }
    } else {
        for y in year..1970 {
            days -= if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        }
    }
    for m in 1..month {
        days += match m {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => unreachable!(),
        };
    }
    days += i64::from(day - 1);
    Ok(days * 86_400 + i64::from(hour * 3600 + minute * 60 + second))
}

fn read_policy(path: &str) -> Result<Vec<Row>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: missing header"))?.trim_end_matches('\r');
    if header != HEADER.join(",") {
        return Err(format!("{path}: unexpected header; expected {}", HEADER.join(",")));
    }
    let mut rows = Vec::new();
    for (index, raw) in lines.enumerate() {
        let line_no = index + 2;
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            return Err(format!("{path}:{line_no}: blank row"));
        }
        if line.contains('"') {
            return Err(format!("{path}:{line_no}: quoted CSV fields are unsupported"));
        }
        let f: Vec<String> = line.split(',').map(|value| value.trim().to_string()).collect();
        if f.len() != HEADER.len() {
            return Err(format!("{path}:{line_no}: expected {} columns, found {}", HEADER.len(), f.len()));
        }
        let from = required(&f[17], "effective_from_utc")?;
        let to = required(&f[18], "effective_to_utc")?;
        let from_seconds = utc_seconds(&from, "effective_from_utc")?;
        let to_seconds = utc_seconds(&to, "effective_to_utc")?;
        if to_seconds <= from_seconds {
            return Err(format!("{path}:{line_no}: effective interval must be positive"));
        }
        if !sha256(&f[14]) || !sha256(&f[16]) {
            return Err(format!("{path}:{line_no}: source and approval SHA-256 must be 64 hexadecimal characters"));
        }
        rows.push(Row {
            policy_id: required(&f[0], "policy_id")?,
            revision: required(&f[1], "policy_revision")?,
            decision_case: required(&f[2], "decision_case_id")?,
            family: required(&f[3], "product_family")?,
            test_id: required(&f[4], "test_id")?,
            measurand: required(&f[5], "measurand")?,
            unit: required(&f[6], "unit")?,
            outcome: required(&f[7], "outcome_class")?,
            release_prohibited: parse_bool(&f[8], "release_prohibited")?,
            action: required(&f[9], "action_id")?,
            is_release: parse_bool(&f[10], "is_release_action")?,
            utility: finite(&f[11], "utility_value")?,
            utility_unit: required(&f[12], "utility_unit")?,
            source_artifact: required(&f[13], "policy_source_artifact_id")?,
            source_sha256: f[14].clone(),
            approval_artifact: required(&f[15], "approval_artifact_id")?,
            approval_sha256: f[16].clone(),
            effective_from: from,
            effective_to: to,
            safety_revision: required(&f[19], "safety_policy_revision")?,
        });
    }
    if rows.is_empty() {
        return Err(format!("{path}: no policy rows"));
    }
    Ok(rows)
}

fn validate(rows: &[Row], as_of_utc: &str) -> Result<Report, String> {
    if rows.is_empty() {
        return Err("policy has no rows".into());
    }
    let as_of = utc_seconds(as_of_utc, "as_of_utc")?;
    let first = &rows[0];
    for row in rows {
        if row.policy_id.trim().is_empty() || row.revision.trim().is_empty()
            || row.decision_case.trim().is_empty() || row.source_artifact.trim().is_empty()
            || row.approval_artifact.trim().is_empty() || row.safety_revision.trim().is_empty()
            || !row.utility.is_finite() || !sha256(&row.source_sha256) || !sha256(&row.approval_sha256) {
            return Err("policy row has an empty identity, non-finite utility or invalid evidence digest".into());
        }
        if row.policy_id != first.policy_id || row.revision != first.revision
            || row.decision_case != first.decision_case || row.family != first.family
            || row.test_id != first.test_id || row.measurand != first.measurand
            || row.unit != first.unit || row.utility_unit != first.utility_unit
            || row.source_artifact != first.source_artifact || row.source_sha256 != first.source_sha256
            || row.approval_artifact != first.approval_artifact || row.approval_sha256 != first.approval_sha256
            || row.effective_from != first.effective_from || row.effective_to != first.effective_to
            || row.safety_revision != first.safety_revision {
            return Err("rows contain conflicting policy identity, scope, utility unit, approval or validity metadata".into());
        }
    }

    let mut actions = BTreeSet::new();
    let mut outcomes = BTreeSet::new();
    let mut cells = BTreeMap::new();
    let mut action_release_flags: BTreeMap<String, bool> = BTreeMap::new();
    let mut outcome_release_flags: BTreeMap<String, bool> = BTreeMap::new();
    for row in rows {
        actions.insert(row.action.clone());
        outcomes.insert(row.outcome.clone());
        if let Some(existing) = action_release_flags.insert(row.action.clone(), row.is_release) {
            if existing != row.is_release {
                return Err(format!("action {:?} has inconsistent is_release_action values", row.action));
            }
        }
        if let Some(existing) = outcome_release_flags.insert(row.outcome.clone(), row.release_prohibited) {
            if existing != row.release_prohibited {
                return Err(format!("outcome {:?} has inconsistent release_prohibited values", row.outcome));
            }
        }
        let key = (row.action.clone(), row.outcome.clone());
        if cells.insert(key, row.utility).is_some() {
            return Err(format!("duplicate action/outcome cell for {:?}/{:?}", row.action, row.outcome));
        }
    }
    if actions.len() < 2 || outcomes.len() < 2 {
        return Err("decision policy requires at least two actions and two outcome classes".into());
    }
    if action_release_flags.values().filter(|value| **value).count() != 1 {
        return Err("decision policy must declare exactly one release action".into());
    }
    if cells.len() != actions.len() * outcomes.len() {
        return Err(format!(
            "decision matrix incomplete: have {} cells for {} actions × {} outcomes",
            cells.len(), actions.len(), outcomes.len()
        ));
    }

    let release_action = action_release_flags.iter()
        .find_map(|(action, is_release)| is_release.then_some(action.clone()))
        .expect("exactly one release action validated");
    for outcome in &outcomes {
        if outcome_release_flags.get(outcome) == Some(&true) {
            let release_utility = cells[&(release_action.clone(), outcome.clone())];
            let best_alternative = actions.iter()
                .filter(|action| **action != release_action)
                .map(|action| cells[&(action.clone(), outcome.clone())])
                .fold(f64::NEG_INFINITY, f64::max);
            if release_utility >= best_alternative {
                return Err(format!(
                    "release action must not maximize utility for release-prohibited outcome {outcome:?}"
                ));
            }
        }
    }

    let active = as_of >= utc_seconds(&first.effective_from, "effective_from_utc")?
        && as_of <= utc_seconds(&first.effective_to, "effective_to_utc")?;
    let blockers = if active { Vec::new() } else { vec!["POLICY_OUTSIDE_EFFECTIVE_WINDOW".to_string()] };
    Ok(Report {
        policy_id: first.policy_id.clone(),
        revision: first.revision.clone(),
        decision_case: first.decision_case.clone(),
        action_count: actions.len(),
        outcome_count: outcomes.len(),
        cell_count: cells.len(),
        status: if blockers.is_empty() { "VALID_FOR_DECISION_SUPPORT" } else { "BLOCKED" }.into(),
        blockers,
    })
}

fn write_report(path: &str, report: &Report) -> Result<(), String> {
    let blockers = if report.blockers.is_empty() { "NONE".to_string() } else { report.blockers.join(";") };
    let output = format!(
        "policy_id,policy_revision,decision_case_id,action_count,outcome_count,cell_count,status,blockers\n{},{},{},{},{},{},{},{}\n",
        report.policy_id, report.revision, report.decision_case, report.action_count,
        report.outcome_count, report.cell_count, report.status, blockers
    );
    fs::write(path, output).map_err(|e| format!("cannot write {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(outcome: &str, prohibited: bool, action: &str, release: bool, utility: f64) -> Row {
        Row {
            policy_id: "POLICY-SYN-001".into(), revision: "REV-1".into(),
            decision_case: "TX-ROUTINE-TEST".into(), family: "POWER_TRANSFORMER".into(),
            test_id: "ROUTINE_TESTS".into(), measurand: "decision_state".into(), unit: "class".into(),
            outcome: outcome.into(), release_prohibited: prohibited, action: action.into(),
            is_release: release, utility, utility_unit: "SYNTHETIC_POINTS".into(),
            source_artifact: "POLICY-SOURCE".into(), source_sha256: "a".repeat(64),
            approval_artifact: "APPROVAL".into(), approval_sha256: "b".repeat(64),
            effective_from: "2026-01-01T00:00:00Z".into(), effective_to: "2027-12-31T23:59:59Z".into(),
            safety_revision: "SAFE-REV-1".into(),
        }
    }

    fn valid_rows() -> Vec<Row> {
        let mut rows = Vec::new();
        for (outcome, prohibited, release, rework, escalate) in [
            ("PASS", false, 10.0, 1.0, 2.0),
            ("FAIL", true, -100.0, 5.0, 3.0),
            ("INDETERMINATE", true, -50.0, 2.0, 6.0),
            ("OOD", true, -80.0, 1.0, 5.0),
        ] {
            rows.push(row(outcome, prohibited, "RELEASE", true, release));
            rows.push(row(outcome, prohibited, "REWORK", false, rework));
            rows.push(row(outcome, prohibited, "ESCALATE", false, escalate));
        }
        rows
    }

    #[test]
    fn complete_policy_is_valid_inside_its_effective_window() {
        let result = validate(&valid_rows(), "2026-10-30T00:00:00Z").unwrap();
        assert_eq!(result.status, "VALID_FOR_DECISION_SUPPORT");
        assert_eq!(result.action_count, 3);
        assert_eq!(result.outcome_count, 4);
        assert_eq!(result.cell_count, 12);
    }

    #[test]
    fn missing_action_outcome_cell_is_rejected() {
        let mut rows = valid_rows();
        rows.pop();
        assert!(validate(&rows, "2026-10-30T00:00:00Z").unwrap_err().contains("incomplete"));
    }

    #[test]
    fn release_must_not_be_optimal_for_release_prohibited_outcome() {
        let mut rows = valid_rows();
        let row = rows.iter_mut().find(|r| r.outcome == "FAIL" && r.action == "RELEASE").unwrap();
        row.utility = 99.0;
        assert!(validate(&rows, "2026-10-30T00:00:00Z").unwrap_err().contains("release action must not maximize"));
    }

    #[test]
    fn duplicate_cells_and_conflicting_metadata_are_rejected() {
        let mut rows = valid_rows();
        rows.push(rows[0].clone());
        assert!(validate(&rows, "2026-10-30T00:00:00Z").unwrap_err().contains("duplicate"));
        let mut rows = valid_rows();
        rows[1].utility_unit = "OTHER_UNIT".into();
        assert!(validate(&rows, "2026-10-30T00:00:00Z").unwrap_err().contains("conflicting policy"));
    }

    #[test]
    fn expired_policy_is_blocked_not_silently_reused() {
        let result = validate(&valid_rows(), "2028-01-01T00:00:00Z").unwrap();
        assert_eq!(result.status, "BLOCKED");
        assert_eq!(result.blockers, vec!["POLICY_OUTSIDE_EFFECTIVE_WINDOW"]);
    }

    #[test]
    fn invalid_timestamp_and_digest_are_rejected() {
        assert!(utc_seconds("2025-02-29T00:00:00Z", "test").is_err());
        let mut rows = valid_rows();
        rows[0].approval_sha256 = "tampered".into();
        assert!(validate(&rows, "2026-10-30T00:00:00Z").unwrap_err().contains("invalid evidence digest"));
    }
}
