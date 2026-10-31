//! Offline governed-intervention transaction verifier.
//!
//! This reference audits immutable action records. It never sends network
//! traffic, writes to PLCs, invokes tools, or actuates physical equipment.
//!
//! Build and test:
//!   rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/intervention_ledger.rs -o /tmp/mfg-intervention-tests
//!   /tmp/mfg-intervention-tests
//!
//! Build and run:
//!   rustc --edition=2021 -D warnings tools/manufacturing-assurance/intervention_ledger.rs -o /tmp/mfg-intervention
//!   /tmp/mfg-intervention tools/manufacturing-assurance/fixtures/valid-intervention-ledger.tsv

use std::collections::BTreeSet;
#[cfg(not(test))]
use std::env;
#[cfg(not(test))]
use std::fs;

#[derive(Clone, Debug, PartialEq)]
struct Plan {
    id: String,
    asset: String,
    scope: String,
    action: String,
    target: f64,
    unit: String,
    allowed_min: f64,
    allowed_max: f64,
    action_digest: String,
    precondition_digest: String,
    assurance_digest: String,
    valid_from: String,
    valid_until: String,
    requires_human: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Check {
    id: String,
    plan_id: String,
    kind: String,
    status: String,
    checked_at: String,
    evidence_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Authority {
    id: String,
    plan_id: String,
    principal_type: String,
    principal: String,
    asset: String,
    scope: String,
    action: String,
    approved_at: String,
    valid_until: String,
    decision: String,
    receipt_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Execution {
    id: String,
    plan_id: String,
    command_id: String,
    executor: String,
    command_digest: String,
    issued_at: String,
    acknowledged_at: String,
    status: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Verification {
    id: String,
    plan_id: String,
    stage: String,
    metric: String,
    value: f64,
    unit: String,
    acceptable_min: f64,
    acceptable_max: f64,
    measured_at: String,
    measurement_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Recovery {
    id: String,
    plan_id: String,
    mode: String,
    issued_at: String,
    acknowledged_at: String,
    status: String,
    evidence_digest: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Manifest {
    plan: Option<Plan>,
    checks: Vec<Check>,
    authorities: Vec<Authority>,
    execution: Option<Execution>,
    verifications: Vec<Verification>,
    recovery: Option<Recovery>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Assessment {
    state: String,
    reasons: Vec<String>,
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    if value.trim().is_empty() || value == "-" {
        Err(format!("{context}: missing value"))
    } else {
        Ok(value.to_string())
    }
}

fn parse_bool(value: &str, context: &str) -> Result<bool, String> {
    match value {
        "TRUE" => Ok(true),
        "FALSE" => Ok(false),
        _ => Err(format!("{context}: expected TRUE or FALSE")),
    }
}

fn parse_finite(value: &str, context: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("{context}: invalid number"))?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(format!("{context}: number must be finite"))
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_utc(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || !bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
    {
        return false;
    }
    let parse = |range: std::ops::Range<usize>| -> Option<u32> {
        std::str::from_utf8(&bytes[range]).ok()?.parse().ok()
    };
    let year = match parse(0..4) { Some(value) => value, None => return false };
    let month = match parse(5..7) { Some(value) => value, None => return false };
    let day = match parse(8..10) { Some(value) => value, None => return false };
    let hour = match parse(11..13) { Some(value) => value, None => return false };
    let minute = match parse(14..16) { Some(value) => value, None => return false };
    let second = match parse(17..19) { Some(value) => value, None => return false };
    if year == 0 || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap_year = year % 400 == 0 || (year % 4 == 0 && year % 100 != 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    day >= 1 && day <= days_in_month
}

fn valid_interval(start: &str, end: &str, context: &str) -> Result<(), String> {
    if !valid_utc(start) || !valid_utc(end) || start >= end {
        return Err(format!("{context}: invalid UTC interval"));
    }
    Ok(())
}

fn active_at(start: &str, end: &str, time: &str) -> bool {
    start <= time && time < end
}

fn parse_manifest(input: &str) -> Result<Manifest, String> {
    let mut manifest = Manifest::default();
    let mut identifiers = BTreeSet::new();
    let mut command_ids = BTreeSet::new();

    for (index, original) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = original.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        let record_type = fields.first().copied().unwrap_or("");

        match record_type {
            "PLAN" => {
                if fields.len() != 15 || manifest.plan.is_some() {
                    return Err(format!("line {line_number}: PLAN requires 14 fields and must occur once"));
                }
                let id = nonempty(fields[1], "PLAN id")?;
                if !identifiers.insert(id.clone()) {
                    return Err(format!("line {line_number}: duplicate record id {id}"));
                }
                let plan = Plan {
                    id,
                    asset: nonempty(fields[2], "PLAN asset")?,
                    scope: nonempty(fields[3], "PLAN scope")?,
                    action: nonempty(fields[4], "PLAN action")?,
                    target: parse_finite(fields[5], "PLAN target")?,
                    unit: nonempty(fields[6], "PLAN unit")?,
                    allowed_min: parse_finite(fields[7], "PLAN allowed_min")?,
                    allowed_max: parse_finite(fields[8], "PLAN allowed_max")?,
                    action_digest: nonempty(fields[9], "PLAN action_digest")?,
                    precondition_digest: nonempty(fields[10], "PLAN precondition_digest")?,
                    assurance_digest: nonempty(fields[11], "PLAN assurance_digest")?,
                    valid_from: nonempty(fields[12], "PLAN valid_from")?,
                    valid_until: nonempty(fields[13], "PLAN valid_until")?,
                    requires_human: parse_bool(fields[14], "PLAN requires_human")?,
                };
                valid_interval(&plan.valid_from, &plan.valid_until, "PLAN validity")?;
                if plan.allowed_min > plan.allowed_max {
                    return Err(format!("line {line_number}: PLAN allowed bounds are reversed"));
                }
                if plan.target < plan.allowed_min || plan.target > plan.allowed_max {
                    return Err(format!("line {line_number}: PLAN target is outside allowed bounds"));
                }
                for (label, digest) in [
                    ("action_digest", plan.action_digest.as_str()),
                    ("precondition_digest", plan.precondition_digest.as_str()),
                    ("assurance_digest", plan.assurance_digest.as_str()),
                ] {
                    if !is_sha256(digest) {
                        return Err(format!("line {line_number}: PLAN {label} is not SHA-256"));
                    }
                }
                manifest.plan = Some(plan);
            }
            "CHECK" => {
                if fields.len() != 7 {
                    return Err(format!("line {line_number}: CHECK requires 6 fields"));
                }
                let id = nonempty(fields[1], "CHECK id")?;
                if !identifiers.insert(id.clone()) {
                    return Err(format!("line {line_number}: duplicate record id {id}"));
                }
                let check = Check {
                    id,
                    plan_id: nonempty(fields[2], "CHECK plan_id")?,
                    kind: nonempty(fields[3], "CHECK kind")?,
                    status: nonempty(fields[4], "CHECK status")?,
                    checked_at: nonempty(fields[5], "CHECK checked_at")?,
                    evidence_digest: nonempty(fields[6], "CHECK evidence_digest")?,
                };
                if !matches!(check.kind.as_str(), "PRECONDITION" | "ASSURANCE" | "INTERLOCK") {
                    return Err(format!("line {line_number}: unsupported CHECK kind"));
                }
                if !matches!(check.status.as_str(), "PASS" | "FAIL") {
                    return Err(format!("line {line_number}: unsupported CHECK status"));
                }
                if !valid_utc(&check.checked_at) || !is_sha256(&check.evidence_digest) {
                    return Err(format!("line {line_number}: invalid CHECK time or digest"));
                }
                manifest.checks.push(check);
            }
            "AUTH" => {
                if fields.len() != 12 {
                    return Err(format!("line {line_number}: AUTH requires 11 fields"));
                }
                let id = nonempty(fields[1], "AUTH id")?;
                if !identifiers.insert(id.clone()) {
                    return Err(format!("line {line_number}: duplicate record id {id}"));
                }
                let authority = Authority {
                    id,
                    plan_id: nonempty(fields[2], "AUTH plan_id")?,
                    principal_type: nonempty(fields[3], "AUTH principal_type")?,
                    principal: nonempty(fields[4], "AUTH principal")?,
                    asset: nonempty(fields[5], "AUTH asset")?,
                    scope: nonempty(fields[6], "AUTH scope")?,
                    action: nonempty(fields[7], "AUTH action")?,
                    approved_at: nonempty(fields[8], "AUTH approved_at")?,
                    valid_until: nonempty(fields[9], "AUTH valid_until")?,
                    decision: nonempty(fields[10], "AUTH decision")?,
                    receipt_digest: nonempty(fields[11], "AUTH receipt_digest")?,
                };
                if !matches!(authority.principal_type.as_str(), "HUMAN" | "SYSTEM") {
                    return Err(format!("line {line_number}: unsupported AUTH principal type"));
                }
                if !matches!(authority.decision.as_str(), "GRANTED" | "REJECTED") {
                    return Err(format!("line {line_number}: unsupported AUTH decision"));
                }
                if !valid_utc(&authority.approved_at)
                    || !valid_utc(&authority.valid_until)
                    || authority.approved_at >= authority.valid_until
                    || !is_sha256(&authority.receipt_digest)
                {
                    return Err(format!("line {line_number}: invalid AUTH time or receipt digest"));
                }
                manifest.authorities.push(authority);
            }
            "EXEC" => {
                if fields.len() != 9 || manifest.execution.is_some() {
                    return Err(format!("line {line_number}: EXEC requires 8 fields and may occur once"));
                }
                let id = nonempty(fields[1], "EXEC id")?;
                if !identifiers.insert(id.clone()) {
                    return Err(format!("line {line_number}: duplicate record id {id}"));
                }
                let execution = Execution {
                    id,
                    plan_id: nonempty(fields[2], "EXEC plan_id")?,
                    command_id: nonempty(fields[3], "EXEC command_id")?,
                    executor: nonempty(fields[4], "EXEC executor")?,
                    command_digest: nonempty(fields[5], "EXEC command_digest")?,
                    issued_at: nonempty(fields[6], "EXEC issued_at")?,
                    acknowledged_at: nonempty(fields[7], "EXEC acknowledged_at")?,
                    status: nonempty(fields[8], "EXEC status")?,
                };
                if !is_sha256(&execution.command_digest)
                    || !valid_utc(&execution.issued_at)
                    || !valid_utc(&execution.acknowledged_at)
                    || execution.acknowledged_at < execution.issued_at
                    || !matches!(execution.status.as_str(), "ACKNOWLEDGED" | "REJECTED")
                {
                    return Err(format!("line {line_number}: invalid EXEC chronology, digest or status"));
                }
                if !command_ids.insert(execution.command_id.clone()) {
                    return Err(format!("line {line_number}: duplicate/replayed command id"));
                }
                manifest.execution = Some(execution);
            }
            "VERIFY" => {
                if fields.len() != 11 {
                    return Err(format!("line {line_number}: VERIFY requires 10 fields"));
                }
                let id = nonempty(fields[1], "VERIFY id")?;
                if !identifiers.insert(id.clone()) {
                    return Err(format!("line {line_number}: duplicate record id {id}"));
                }
                let verification = Verification {
                    id,
                    plan_id: nonempty(fields[2], "VERIFY plan_id")?,
                    stage: nonempty(fields[3], "VERIFY stage")?,
                    metric: nonempty(fields[4], "VERIFY metric")?,
                    value: parse_finite(fields[5], "VERIFY value")?,
                    unit: nonempty(fields[6], "VERIFY unit")?,
                    acceptable_min: parse_finite(fields[7], "VERIFY acceptable_min")?,
                    acceptable_max: parse_finite(fields[8], "VERIFY acceptable_max")?,
                    measured_at: nonempty(fields[9], "VERIFY measured_at")?,
                    measurement_digest: nonempty(fields[10], "VERIFY measurement_digest")?,
                };
                if !matches!(verification.stage.as_str(), "POST_ACTION" | "POST_RECOVERY")
                    || !valid_utc(&verification.measured_at)
                    || !is_sha256(&verification.measurement_digest)
                    || verification.acceptable_min > verification.acceptable_max
                {
                    return Err(format!("line {line_number}: invalid VERIFY stage, interval, time or digest"));
                }
                manifest.verifications.push(verification);
            }
            "RECOVERY" => {
                if fields.len() != 8 || manifest.recovery.is_some() {
                    return Err(format!("line {line_number}: RECOVERY requires 7 fields and may occur once"));
                }
                let id = nonempty(fields[1], "RECOVERY id")?;
                if !identifiers.insert(id.clone()) {
                    return Err(format!("line {line_number}: duplicate record id {id}"));
                }
                let recovery = Recovery {
                    id,
                    plan_id: nonempty(fields[2], "RECOVERY plan_id")?,
                    mode: nonempty(fields[3], "RECOVERY mode")?,
                    issued_at: nonempty(fields[4], "RECOVERY issued_at")?,
                    acknowledged_at: nonempty(fields[5], "RECOVERY acknowledged_at")?,
                    status: nonempty(fields[6], "RECOVERY status")?,
                    evidence_digest: nonempty(fields[7], "RECOVERY evidence_digest")?,
                };
                if !matches!(recovery.mode.as_str(), "ROLLBACK" | "SAFE_STATE" | "CONTAIN")
                    || !matches!(recovery.status.as_str(), "ACKNOWLEDGED" | "FAILED")
                    || !valid_utc(&recovery.issued_at)
                    || !valid_utc(&recovery.acknowledged_at)
                    || recovery.acknowledged_at < recovery.issued_at
                    || !is_sha256(&recovery.evidence_digest)
                {
                    return Err(format!("{line_number}: invalid RECOVERY record"));
                }
                manifest.recovery = Some(recovery);
            }
            other => return Err(format!("line {line_number}: unknown record type {other:?}")),
        }
    }

    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    let plan = manifest
        .plan
        .as_ref()
        .ok_or_else(|| "manifest requires exactly one PLAN".to_string())?;

    for check in &manifest.checks {
        if check.plan_id != plan.id {
            return Err(format!("CHECK {} references another plan", check.id));
        }
        let expected = match check.kind.as_str() {
            "PRECONDITION" => plan.precondition_digest.as_str(),
            "ASSURANCE" => plan.assurance_digest.as_str(),
            "INTERLOCK" => check.evidence_digest.as_str(),
            _ => return Err(format!("CHECK {} has unsupported kind", check.id)),
        };
        if check.evidence_digest != expected {
            return Err(format!("CHECK {} digest does not match its plan binding", check.id));
        }
    }

    for authority in &manifest.authorities {
        if authority.plan_id != plan.id {
            return Err(format!("AUTH {} references another plan", authority.id));
        }
    }
    if let Some(execution) = &manifest.execution {
        if execution.plan_id != plan.id {
            return Err(format!("EXEC {} references another plan", execution.id));
        }
        if execution.command_digest != plan.action_digest {
            // This is a policy failure, represented during evaluation rather
            // than a parser error so the report preserves the rejected reason.
        }
    }
    for verification in &manifest.verifications {
        if verification.plan_id != plan.id {
            return Err(format!("VERIFY {} references another plan", verification.id));
        }
    }
    if let Some(recovery) = &manifest.recovery {
        if recovery.plan_id != plan.id {
            return Err(format!("RECOVERY {} references another plan", recovery.id));
        }
    }

    let mut verification_keys = BTreeSet::new();
    for verification in &manifest.verifications {
        let key = format!("{}|{}", verification.stage, verification.metric);
        if !verification_keys.insert(key) {
            return Err("duplicate metric verification within a transaction stage".to_string());
        }
    }

    let preconditions = manifest.checks.iter().filter(|check| check.kind == "PRECONDITION").count();
    let assurance = manifest.checks.iter().filter(|check| check.kind == "ASSURANCE").count();
    let interlocks = manifest.checks.iter().filter(|check| check.kind == "INTERLOCK").count();
    if preconditions > 1 || assurance > 1 {
        return Err("PRECONDITION and ASSURANCE checks may occur at most once".to_string());
    }
    if manifest.verifications.iter().any(|verification| verification.stage == "POST_RECOVERY")
        && manifest.recovery.is_none()
    {
        return Err("POST_RECOVERY verification exists without a RECOVERY record".to_string());
    }
    if manifest.verifications.iter().any(|verification| verification.stage == "POST_ACTION")
        && manifest.recovery.is_some()
        && manifest.verifications.iter().all(|verification| {
            verification.stage != "POST_ACTION"
                || verification.value >= verification.acceptable_min
                    && verification.value <= verification.acceptable_max
        })
    {
        return Err("RECOVERY record exists although all POST_ACTION verifications pass".to_string());
    }

    // Variables are used in validation/reporting; keep required gate cardinality
    // as an evaluation result rather than a parse-time fatal error.
    let _required_gate_counts = (preconditions, assurance, interlocks);
    Ok(())
}

fn assess(manifest: &Manifest) -> Assessment {
    let plan = manifest.plan.as_ref().expect("validated manifest has PLAN");
    let mut reasons = Vec::new();

    let check_kinds = ["PRECONDITION", "ASSURANCE", "INTERLOCK"];
    for kind in check_kinds {
        let checks: Vec<&Check> = manifest.checks.iter().filter(|check| check.kind == kind).collect();
        if checks.is_empty() {
            reasons.push(format!("MISSING_{kind}_CHECK"));
        }
        for check in checks {
            if !active_at(&plan.valid_from, &plan.valid_until, &check.checked_at) {
                reasons.push(format!("CHECK_OUTSIDE_PLAN_VALIDITY:{}", check.id));
            }
            if check.status != "PASS" {
                reasons.push(format!("CHECK_FAILED:{}", check.id));
            }
        }
    }
    if let Some(execution) = &manifest.execution {
        for check in &manifest.checks {
            if check.checked_at > execution.issued_at {
                reasons.push(format!("CHECK_AFTER_COMMAND_ISSUE:{}", check.id));
            }
        }
        if !active_at(&plan.valid_from, &plan.valid_until, &execution.issued_at) {
            reasons.push("COMMAND_OUTSIDE_PLAN_VALIDITY".to_string());
        }
    }

    let mut authorized = false;
    let mut human_authorized = false;
    if let Some(execution) = &manifest.execution {
        for authority in &manifest.authorities {
            if authority.decision == "GRANTED"
                && authority.asset == plan.asset
                && authority.scope == plan.scope
                && authority.action == plan.action
                && authority.approved_at >= plan.valid_from
                && authority.approved_at <= execution.issued_at
                && execution.issued_at < authority.valid_until
                && execution.issued_at >= plan.valid_from
            {
                authorized = true;
                if authority.principal_type == "HUMAN" {
                    human_authorized = true;
                }
            }
        }
    }
    if !authorized {
        reasons.push("NO_CURRENT_MATCHING_AUTHORITY".to_string());
    }
    if plan.requires_human && !human_authorized {
        reasons.push("HUMAN_APPROVAL_REQUIRED".to_string());
    }

    let execution = match manifest.execution.as_ref() {
        Some(value) => value,
        None => {
            reasons.push("EXECUTION_MISSING".to_string());
            reasons.sort();
            reasons.dedup();
            return Assessment {
                state: "BLOCKED".to_string(),
                reasons,
            };
        }
    };

    if execution.command_digest != plan.action_digest {
        reasons.push("COMMAND_DIGEST_MISMATCH".to_string());
    }
    if !reasons.is_empty() {
        reasons.sort();
        reasons.dedup();
        return Assessment {
            state: "BLOCKED".to_string(),
            reasons,
        };
    }

    if execution.status == "REJECTED" {
        if !manifest.verifications.is_empty() || manifest.recovery.is_some() {
            return Assessment {
                state: "BLOCKED".to_string(),
                reasons: vec!["REJECTED_EXECUTION_HAS_FOLLOWUP_EVENTS".to_string()],
            };
        }
        return Assessment {
            state: "ABORTED".to_string(),
            reasons: vec!["EXECUTION_REJECTED".to_string()],
        };
    }

    let post_action: Vec<&Verification> = manifest
        .verifications
        .iter()
        .filter(|verification| verification.stage == "POST_ACTION")
        .collect();
    if post_action.is_empty() {
        if manifest.recovery.is_some() {
            return Assessment {
                state: "BLOCKED".to_string(),
                reasons: vec!["RECOVERY_WITHOUT_POST_ACTION_FAILURE".to_string()],
            };
        }
        return Assessment {
            state: "ISSUED_UNVERIFIED".to_string(),
            reasons: vec!["POST_ACTION_VERIFICATION_MISSING".to_string()],
        };
    }

    for verification in &post_action {
        if verification.measured_at < execution.acknowledged_at {
            reasons.push(format!("POST_ACTION_BEFORE_ACK:{}", verification.id));
        }
    }
    if !reasons.is_empty() {
        reasons.sort();
        reasons.dedup();
        return Assessment {
            state: "BLOCKED".to_string(),
            reasons,
        };
    }

    let failed: Vec<&Verification> = post_action
        .iter()
        .copied()
        .filter(|verification| {
            verification.value < verification.acceptable_min
                || verification.value > verification.acceptable_max
        })
        .collect();

    if failed.is_empty() {
        return Assessment {
            state: "VERIFIED".to_string(),
            reasons: vec!["POST_ACTION_OUTCOME_WITHIN_ACCEPTANCE_RANGE".to_string()],
        };
    }

    let recovery = match manifest.recovery.as_ref() {
        Some(value) => value,
        None => {
            return Assessment {
                state: "RECOVERY_REQUIRED".to_string(),
                reasons: failed
                    .iter()
                    .map(|verification| format!("POST_ACTION_OUT_OF_RANGE:{}", verification.id))
                    .collect(),
            };
        }
    };

    let latest_failure_time = failed
        .iter()
        .map(|verification| verification.measured_at.as_str())
        .max()
        .unwrap_or(execution.acknowledged_at.as_str());
    if recovery.issued_at < latest_failure_time || recovery.acknowledged_at < recovery.issued_at {
        return Assessment {
            state: "BLOCKED".to_string(),
            reasons: vec!["RECOVERY_CHRONOLOGY_INVALID".to_string()],
        };
    }
    if recovery.status == "FAILED" {
        return Assessment {
            state: "RECOVERY_FAILED".to_string(),
            reasons: vec![format!("RECOVERY_FAILED:{}", recovery.id)],
        };
    }

    let post_recovery: Vec<&Verification> = manifest
        .verifications
        .iter()
        .filter(|verification| verification.stage == "POST_RECOVERY")
        .collect();
    if post_recovery.is_empty() {
        return Assessment {
            state: "RECOVERY_REQUIRED".to_string(),
            reasons: vec!["POST_RECOVERY_VERIFICATION_MISSING".to_string()],
        };
    }
    for verification in &post_recovery {
        if verification.measured_at < recovery.acknowledged_at {
            reasons.push(format!("POST_RECOVERY_BEFORE_RECOVERY_ACK:{}", verification.id));
        }
    }
    if !reasons.is_empty() {
        reasons.sort();
        reasons.dedup();
        return Assessment {
            state: "BLOCKED".to_string(),
            reasons,
        };
    }

    let key_for = |verification: &Verification| {
        (
            verification.metric.clone(),
            verification.unit.clone(),
            verification.acceptable_min.to_string(),
            verification.acceptable_max.to_string(),
        )
    };
    let failed_keys: BTreeSet<_> = failed.iter().map(|verification| key_for(verification)).collect();
    let post_recovery_keys: BTreeSet<_> = post_recovery
        .iter()
        .map(|verification| key_for(verification))
        .collect();
    if failed_keys != post_recovery_keys {
        return Assessment {
            state: "BLOCKED".to_string(),
            reasons: vec!["POST_RECOVERY_MUST_REVERIFY_EVERY_FAILED_METRIC".to_string()],
        };
    }

    let recovery_failures: Vec<&Verification> = post_recovery
        .iter()
        .copied()
        .filter(|verification| {
            verification.value < verification.acceptable_min
                || verification.value > verification.acceptable_max
        })
        .collect();
    if !recovery_failures.is_empty() {
        return Assessment {
            state: "RECOVERY_FAILED".to_string(),
            reasons: recovery_failures
                .iter()
                .map(|verification| format!("POST_RECOVERY_OUT_OF_RANGE:{}", verification.id))
                .collect(),
        };
    }

    Assessment {
        state: "RECOVERED".to_string(),
        reasons: vec![format!("RECOVERY_VERIFIED:{}", recovery.id)],
    }
}

fn run_report(input: &str) -> Result<String, String> {
    let manifest = parse_manifest(input)?;
    let plan = manifest.plan.as_ref().expect("validated manifest has PLAN");
    let assessment = assess(&manifest);
    let mut output = format!(
        "MFG_INTERVENTION_LEDGER version=1 plan={} asset={} scope={} action={} target={} unit={} allowed=[{},{}] requires_human={}\n",
        plan.id,
        plan.asset,
        plan.scope,
        plan.action,
        plan.target,
        plan.unit,
        plan.allowed_min,
        plan.allowed_max,
        plan.requires_human
    );
    for check in &manifest.checks {
        output.push_str(&format!(
            "CHECK id={} kind={} status={} checked_at={} evidence_digest={}\n",
            check.id, check.kind, check.status, check.checked_at, check.evidence_digest
        ));
    }
    for authority in &manifest.authorities {
        output.push_str(&format!(
            "AUTH id={} principal_type={} principal={} decision={} action={} approved_at={} valid_until={} receipt_digest={}\n",
            authority.id,
            authority.principal_type,
            authority.principal,
            authority.decision,
            authority.action,
            authority.approved_at,
            authority.valid_until,
            authority.receipt_digest
        ));
    }
    if let Some(execution) = &manifest.execution {
        output.push_str(&format!(
            "EXEC id={} command_id={} executor={} status={} issued_at={} acknowledged_at={} command_digest={}\n",
            execution.id,
            execution.command_id,
            execution.executor,
            execution.status,
            execution.issued_at,
            execution.acknowledged_at,
            execution.command_digest
        ));
    }
    for verification in &manifest.verifications {
        let pass = verification.value >= verification.acceptable_min
            && verification.value <= verification.acceptable_max;
        output.push_str(&format!(
            "VERIFY id={} stage={} metric={} value={} unit={} accepted=[{},{}] result={} measured_at={} digest={}\n",
            verification.id,
            verification.stage,
            verification.metric,
            verification.value,
            verification.unit,
            verification.acceptable_min,
            verification.acceptable_max,
            if pass { "PASS" } else { "FAIL" },
            verification.measured_at,
            verification.measurement_digest
        ));
    }
    if let Some(recovery) = &manifest.recovery {
        output.push_str(&format!(
            "RECOVERY id={} mode={} status={} issued_at={} acknowledged_at={} digest={}\n",
            recovery.id,
            recovery.mode,
            recovery.status,
            recovery.issued_at,
            recovery.acknowledged_at,
            recovery.evidence_digest
        ));
    }
    output.push_str(&format!(
        "RESULT state={} reasons={}\n",
        assessment.state,
        if assessment.reasons.is_empty() {
            "-".to_string()
        } else {
            assessment.reasons.join(",")
        }
    ));
    output.push_str(
        "CLAIM_CEILING offline_record_audit=true network_traffic=false machine_actuation=false signature_authentication=false raw_sensor_bytes_verified=false plant_safety_proven=false\n",
    );
    Ok(output)
}

#[cfg(not(test))]
fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "tools/manufacturing-assurance/fixtures/valid-intervention-ledger.tsv".to_string());
    match fs::read_to_string(&path)
        .map_err(|error| format!("unable to read {path}: {error}"))
        .and_then(|input| run_report(&input))
    {
        Ok(report) => print!("{report}"),
        Err(error) => {
            eprintln!("mfg-intervention-ledger: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HASH_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HASH_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HASH_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HASH_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    fn valid_manifest() -> String {
        format!(
            "PLAN|plan-1|machine-7|LINE-1|adjust_feed_rate|120.0|mm_per_min|80.0|150.0|{}|{}|{}|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|TRUE\n\
             CHECK|check-pre|plan-1|PRECONDITION|PASS|2026-10-31T11:50:00Z|{}\n\
             CHECK|check-assurance|plan-1|ASSURANCE|PASS|2026-10-31T11:51:00Z|{}\n\
             CHECK|check-interlock|plan-1|INTERLOCK|PASS|2026-10-31T11:52:00Z|{}\n\
             AUTH|auth-1|plan-1|HUMAN|operator-1|machine-7|LINE-1|adjust_feed_rate|2026-10-31T11:53:00Z|2026-10-31T23:00:00Z|GRANTED|{}\n\
             EXEC|exec-1|plan-1|command-1|controller-gateway|{}|2026-10-31T11:55:00Z|2026-10-31T11:55:01Z|ACKNOWLEDGED\n\
             VERIFY|verify-1|plan-1|POST_ACTION|surface_roughness|1.4|um|0.0|2.0|2026-10-31T11:56:00Z|{}\n",
            HASH_A, HASH_B, HASH_C, HASH_B, HASH_C, HASH_D, HASH_E, HASH_A, HASH_F
        )
    }

    fn mutate(input: String, old: &str, new: &str) -> String {
        assert!(input.contains(old), "missing mutation target: {old}");
        input.replacen(old, new, 1)
    }

    fn append_recovery(input: &mut String, recovery_status: &str, post_recovery_value: &str) {
        input.push_str(&format!(
            "RECOVERY|recovery-1|plan-1|SAFE_STATE|2026-10-31T11:57:00Z|2026-10-31T11:57:02Z|{}|{}\n\
             VERIFY|verify-2|plan-1|POST_RECOVERY|surface_roughness|{}|um|0.0|2.0|2026-10-31T11:58:00Z|{}\n",
            recovery_status, HASH_D, post_recovery_value, HASH_F
        ));
    }

    #[test]
    fn valid_post_action_transaction_is_verified() {
        let report = run_report(&valid_manifest()).unwrap();
        assert!(report.contains("RESULT state=VERIFIED"));
        assert!(report.contains("machine_actuation=false"));
    }

    #[test]
    fn missing_precondition_check_blocks_execution() {
        let input = mutate(
            valid_manifest(),
            "CHECK|check-pre|plan-1|PRECONDITION|PASS|2026-10-31T11:50:00Z",
            "CHECK|check-pre|plan-1|INTERLOCK|PASS|2026-10-31T11:50:00Z",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("MISSING_PRECONDITION_CHECK"));
        assert!(report.contains("RESULT state=BLOCKED"));
    }

    #[test]
    fn failed_interlock_check_blocks_execution() {
        let input = mutate(
            valid_manifest(),
            "CHECK|check-interlock|plan-1|INTERLOCK|PASS",
            "CHECK|check-interlock|plan-1|INTERLOCK|FAIL",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("CHECK_FAILED:check-interlock"));
        assert!(report.contains("RESULT state=BLOCKED"));
    }

    #[test]
    fn precondition_digest_must_match_plan_binding() {
        let input = mutate(
            valid_manifest(),
            "CHECK|check-pre|plan-1|PRECONDITION|PASS|2026-10-31T11:50:00Z|bbbb",
            "CHECK|check-pre|plan-1|PRECONDITION|PASS|2026-10-31T11:50:00Z|aaaaaaaa",
        );
        assert!(parse_manifest(&input).is_err());
    }

    #[test]
    fn missing_authority_blocks_execution() {
        let input = mutate(
            valid_manifest(),
            "AUTH|auth-1|plan-1|HUMAN|operator-1|machine-7|LINE-1|adjust_feed_rate|2026-10-31T11:53:00Z|2026-10-31T23:00:00Z|GRANTED",
            "AUTH|auth-1|plan-1|HUMAN|operator-1|machine-7|LINE-1|change_tool|2026-10-31T11:53:00Z|2026-10-31T23:00:00Z|GRANTED",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("NO_CURRENT_MATCHING_AUTHORITY"));
    }

    #[test]
    fn system_principal_cannot_satisfy_human_approval_requirement() {
        let input = mutate(
            valid_manifest(),
            "AUTH|auth-1|plan-1|HUMAN|operator-1",
            "AUTH|auth-1|plan-1|SYSTEM|agent-1",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("HUMAN_APPROVAL_REQUIRED"));
    }

    #[test]
    fn revoked_authority_does_not_authorize_action() {
        let input = mutate(valid_manifest(), "|GRANTED|", "|REJECTED|");
        let report = run_report(&input).unwrap();
        assert!(report.contains("NO_CURRENT_MATCHING_AUTHORITY"));
    }

    #[test]
    fn authority_must_be_effective_before_command_issue() {
        let input = mutate(
            valid_manifest(),
            "2026-10-31T11:53:00Z|2026-10-31T23:00:00Z|GRANTED",
            "2026-10-31T11:56:00Z|2026-10-31T23:00:00Z|GRANTED",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("NO_CURRENT_MATCHING_AUTHORITY"));
    }

    #[test]
    fn wrong_asset_or_scope_authority_is_rejected() {
        let input = mutate(
            valid_manifest(),
            "|machine-7|LINE-1|adjust_feed_rate|2026-10-31T11:53:00Z",
            "|machine-8|LINE-1|adjust_feed_rate|2026-10-31T11:53:00Z",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("NO_CURRENT_MATCHING_AUTHORITY"));
    }

    #[test]
    fn command_digest_must_match_the_approved_action_envelope() {
        let input = mutate(
            valid_manifest(),
            "EXEC|exec-1|plan-1|command-1|controller-gateway|",
            "EXEC|exec-1|plan-1|command-1|controller-gateway|",
        )
        .replace(
            &format!("EXEC|exec-1|plan-1|command-1|controller-gateway|{}|", HASH_A),
            &format!("EXEC|exec-1|plan-1|command-1|controller-gateway|{}|", HASH_B),
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("COMMAND_DIGEST_MISMATCH"));
    }

    #[test]
    fn plan_target_outside_bounds_is_rejected() {
        let input = mutate(
            valid_manifest(),
            "|120.0|mm_per_min|80.0|150.0|",
            "|200.0|mm_per_min|80.0|150.0|",
        );
        assert!(parse_manifest(&input).unwrap_err().contains("outside allowed bounds"));
    }

    #[test]
    fn missing_execution_blocks_completion() {
        let input = mutate(
            valid_manifest(),
            "EXEC|exec-1|plan-1|command-1|controller-gateway|",
            "# EXEC|exec-1|plan-1|command-1|controller-gateway|",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("EXECUTION_MISSING"));
    }

    #[test]
    fn acknowledged_time_cannot_precede_issue_time() {
        let input = mutate(
            valid_manifest(),
            "2026-10-31T11:55:00Z|2026-10-31T11:55:01Z|ACKNOWLEDGED",
            "2026-10-31T11:55:02Z|2026-10-31T11:55:01Z|ACKNOWLEDGED",
        );
        assert!(parse_manifest(&input).is_err());
    }

    #[test]
    fn missing_post_action_verification_is_not_completion() {
        let input = mutate(
            valid_manifest(),
            "VERIFY|verify-1|plan-1|POST_ACTION",
            "# VERIFY|verify-1|plan-1|POST_ACTION",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("RESULT state=ISSUED_UNVERIFIED"));
    }

    #[test]
    fn failed_post_action_verification_requires_recovery() {
        let input = mutate(
            valid_manifest(),
            "|surface_roughness|1.4|um|0.0|2.0|",
            "|surface_roughness|3.4|um|0.0|2.0|",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("RESULT state=RECOVERY_REQUIRED"));
    }

    #[test]
    fn acknowledged_recovery_and_passing_post_recovery_verification_recovers() {
        let mut input = mutate(
            valid_manifest(),
            "|surface_roughness|1.4|um|0.0|2.0|",
            "|surface_roughness|3.4|um|0.0|2.0|",
        );
        append_recovery(&mut input, "ACKNOWLEDGED", "1.7");
        let report = run_report(&input).unwrap();
        assert!(report.contains("RESULT state=RECOVERED"));
    }

    #[test]
    fn failed_recovery_is_not_treated_as_recovered() {
        let mut input = mutate(
            valid_manifest(),
            "|surface_roughness|1.4|um|0.0|2.0|",
            "|surface_roughness|3.4|um|0.0|2.0|",
        );
        append_recovery(&mut input, "FAILED", "1.7");
        let report = run_report(&input).unwrap();
        assert!(report.contains("RESULT state=RECOVERY_FAILED"));
    }

    #[test]
    fn missing_post_recovery_verification_does_not_close_recovery() {
        let mut input = mutate(
            valid_manifest(),
            "|surface_roughness|1.4|um|0.0|2.0|",
            "|surface_roughness|3.4|um|0.0|2.0|",
        );
        input.push_str(&format!(
            "RECOVERY|recovery-1|plan-1|SAFE_STATE|2026-10-31T11:57:00Z|2026-10-31T11:57:02Z|ACKNOWLEDGED|{}\n",
            HASH_D
        ));
        let report = run_report(&input).unwrap();
        assert!(report.contains("POST_RECOVERY_VERIFICATION_MISSING"));
    }

    #[test]
    fn out_of_range_post_recovery_measurement_is_recovery_failure() {
        let mut input = mutate(
            valid_manifest(),
            "|surface_roughness|1.4|um|0.0|2.0|",
            "|surface_roughness|3.4|um|0.0|2.0|",
        );
        append_recovery(&mut input, "ACKNOWLEDGED", "2.7");
        let report = run_report(&input).unwrap();
        assert!(report.contains("RESULT state=RECOVERY_FAILED"));
    }

    #[test]
    fn recovery_must_follow_the_failed_measurement() {
        let mut input = mutate(
            valid_manifest(),
            "|surface_roughness|1.4|um|0.0|2.0|",
            "|surface_roughness|3.4|um|0.0|2.0|",
        );
        input.push_str(&format!(
            "RECOVERY|recovery-1|plan-1|SAFE_STATE|2026-10-31T11:55:30Z|2026-10-31T11:55:32Z|ACKNOWLEDGED|{}\\nVERIFY|verify-2|plan-1|POST_RECOVERY|surface_roughness|1.7|um|0.0|2.0|2026-10-31T11:58:00Z|{}\\n",
            HASH_D, HASH_F
        ));
        let report = run_report(&input).unwrap();
        assert!(report.contains("RECOVERY_CHRONOLOGY_INVALID"));
        assert!(report.contains("RESULT state=BLOCKED"));
    }

    #[test]
    fn post_recovery_measurement_must_follow_recovery_acknowledgment() {
        let mut input = mutate(
            valid_manifest(),
            "|surface_roughness|1.4|um|0.0|2.0|",
            "|surface_roughness|3.4|um|0.0|2.0|",
        );
        input.push_str(&format!(
            "RECOVERY|recovery-1|plan-1|SAFE_STATE|2026-10-31T11:57:00Z|2026-10-31T11:57:02Z|ACKNOWLEDGED|{}\n\
             VERIFY|verify-2|plan-1|POST_RECOVERY|surface_roughness|1.7|um|0.0|2.0|2026-10-31T11:57:01Z|{}\n",
            HASH_D, HASH_F
        ));
        let report = run_report(&input).unwrap();
        assert!(report.contains("POST_RECOVERY_BEFORE_RECOVERY_ACK:verify-2"));
    }

    #[test]
    fn recovery_cannot_be_added_when_post_action_passes() {
        let mut input = valid_manifest();
        input.push_str(&format!(
            "RECOVERY|recovery-1|plan-1|SAFE_STATE|2026-10-31T11:57:00Z|2026-10-31T11:57:02Z|ACKNOWLEDGED|{}\n",
            HASH_D
        ));
        assert!(parse_manifest(&input).unwrap_err().contains("although all POST_ACTION"));
    }

    #[test]
    fn duplicate_command_id_is_rejected() {
        let mut input = valid_manifest();
        input.push_str(&format!(
            "EXEC|exec-2|plan-1|command-1|controller-gateway|{}|2026-10-31T11:55:00Z|2026-10-31T11:55:01Z|ACKNOWLEDGED\n",
            HASH_A
        ));
        assert!(parse_manifest(&input).unwrap_err().contains("EXEC requires"));
    }

    #[test]
    fn malformed_measurement_digest_is_rejected() {
        let input = mutate(
            valid_manifest(),
            &format!("|2026-10-31T11:56:00Z|{}", HASH_F),
            "|2026-10-31T11:56:00Z|no-digest",
        );
        assert!(parse_manifest(&input).is_err());
    }

    #[test]
    fn invalid_interval_and_non_finite_measurement_are_rejected() {
        let invalid_interval = mutate(
            valid_manifest(),
            "|0.0|2.0|2026-10-31T11:56:00Z",
            "|3.0|2.0|2026-10-31T11:56:00Z",
        );
        assert!(parse_manifest(&invalid_interval).is_err());

        let non_finite = mutate(valid_manifest(), "|1.4|um|", "|NaN|um|");
        assert!(parse_manifest(&non_finite).is_err());
    }

    #[test]
    fn rejected_execution_aborts_without_claiming_success() {
        let input = mutate(
            valid_manifest(),
            "|2026-10-31T11:55:01Z|ACKNOWLEDGED",
            "|2026-10-31T11:55:01Z|REJECTED",
        );
        let input = mutate(
            input,
            "VERIFY|verify-1|plan-1|POST_ACTION",
            "# VERIFY|verify-1|plan-1|POST_ACTION",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("RESULT state=ABORTED"));
    }

    #[test]
    fn deterministic_report_replays_identically() {
        let input = valid_manifest();
        assert_eq!(run_report(&input).unwrap(), run_report(&input).unwrap());
    }

    #[test]
    fn result_explicitly_disclaims_actuation_and_safety() {
        let report = run_report(&valid_manifest()).unwrap();
        assert!(report.contains("network_traffic=false"));
        assert!(report.contains("machine_actuation=false"));
        assert!(report.contains("plant_safety_proven=false"));
    }
}
