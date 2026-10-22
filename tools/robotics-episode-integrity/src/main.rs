use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

const DEFAULT_CORPUS: &str = "docs/tech/robotics-plan-episode-integrity-corpus-v0.1.json";
const AUDITOR_SOURCE: &[u8] = include_bytes!("main.rs");
const CORPUS_SOURCE: &[u8] =
    include_bytes!("../../../docs/tech/robotics-plan-episode-integrity-corpus-v0.1.json");
const ELIGIBLE: &str = "eligible_for_simulated_plan";
const BLOCKED: &str = "blocked";
const REVISION_KEYS: [&str; 7] = [
    "world",
    "calibration",
    "tool_profile",
    "authorization",
    "localization",
    "resources",
    "permissions",
];
const GATE_KEYS: [&str; 8] = [
    "calibration_valid",
    "tool_compatible",
    "human_approval_valid",
    "localization_valid",
    "return_reserve_satisfied",
    "authorization_valid",
    "site_permission_valid",
    "environment_within_envelope",
];
const CLAIM_CEILING: &str = "Synthetic episode-contract fixtures only; no robot performance, physical safety, field readiness, or standards-conformity claim.";

#[derive(Debug, PartialEq, Eq)]
struct Evaluation {
    decision: &'static str,
    reason_codes: Vec<String>,
}

#[derive(Debug)]
struct ActivePlan {
    plan_id: String,
    plan_digest: String,
    bound_revisions: BTreeMap<String, u64>,
    validated: bool,
    invalidated: bool,
}

#[derive(Debug)]
struct BenchmarkReport {
    episode_count: usize,
    passed_episode_count: usize,
    failures: Vec<String>,
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn push_once(items: &mut Vec<String>, code: &str) {
    if !items.iter().any(|existing| existing == code) {
        items.push(code.to_owned());
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_revisions(value: &Value) -> Option<BTreeMap<String, u64>> {
    let object = value.as_object()?;
    if object.len() != REVISION_KEYS.len() {
        return None;
    }
    let mut revisions = BTreeMap::new();
    for key in REVISION_KEYS {
        let revision = value.get(key)?.as_u64()?;
        revisions.insert(key.to_owned(), revision);
    }
    Some(revisions)
}

fn parse_gates(value: &Value) -> Option<BTreeMap<String, bool>> {
    let object = value.as_object()?;
    if object.len() != GATE_KEYS.len() {
        return None;
    }
    let mut gates = BTreeMap::new();
    for key in GATE_KEYS {
        let status = value.get(key)?.as_bool()?;
        gates.insert(key.to_owned(), status);
    }
    Some(gates)
}

fn gates_satisfied(gates: &BTreeMap<String, bool>) -> bool {
    GATE_KEYS
        .iter()
        .all(|key| gates.get(*key).copied() == Some(true))
}

fn sequence_is_monotonic(events: &[Value]) -> bool {
    let mut previous = None;
    for event in events {
        let Some(sequence) = event.get("sequence").and_then(Value::as_u64) else {
            return false;
        };
        if previous.is_some_and(|value| sequence <= value) {
            return false;
        }
        previous = Some(sequence);
    }
    true
}

/// Independent JSON-value-based episode evaluator. It deliberately does not import
/// the static platform oracle's typed scenarios or blocker routines.
fn evaluate_episode(episode: &Value, initial_state: &Value) -> Evaluation {
    let events = match episode.get("events").and_then(Value::as_array) {
        Some(events) => events,
        None => {
            return Evaluation {
                decision: BLOCKED,
                reason_codes: vec!["EVENTS_MALFORMED".to_owned()],
            };
        }
    };
    if !sequence_is_monotonic(events) {
        return Evaluation {
            decision: BLOCKED,
            reason_codes: vec!["EVENT_SEQUENCE_NOT_MONOTONIC".to_owned()],
        };
    }

    let Some(mut revisions) = initial_state
        .get("revisions")
        .and_then(parse_revisions)
    else {
        return Evaluation {
            decision: BLOCKED,
            reason_codes: vec!["INITIAL_REVISION_SNAPSHOT_INVALID".to_owned()],
        };
    };
    let Some(mut gates) = initial_state.get("gates").and_then(parse_gates) else {
        return Evaluation {
            decision: BLOCKED,
            reason_codes: vec!["INITIAL_GATE_SNAPSHOT_INVALID".to_owned()],
        };
    };

    let mut active_plan: Option<ActivePlan> = None;
    let mut accepted = false;
    let mut reasons = Vec::new();

    if episode.get("expected_physical_actuation").and_then(Value::as_bool) != Some(false) {
        push_once(&mut reasons, "PHYSICAL_ACTUATION_FORBIDDEN");
    }

    for event in events {
        match str_at(event, "kind").unwrap_or_default() {
            "plan_proposed" => {
                if active_plan.is_some() {
                    push_once(&mut reasons, "PLAN_ALREADY_ACTIVE");
                    continue;
                }
                let Some(plan_id) = str_at(event, "plan_id").filter(|id| !id.trim().is_empty())
                else {
                    push_once(&mut reasons, "PLAN_ID_MISSING");
                    continue;
                };
                let Some(plan_digest) = str_at(event, "plan_digest")
                    .filter(|digest| valid_sha256(digest))
                else {
                    push_once(&mut reasons, "PLAN_DIGEST_INVALID");
                    continue;
                };
                let Some(bound_revisions) =
                    event.get("bound_revisions").and_then(parse_revisions)
                else {
                    push_once(&mut reasons, "PLAN_SNAPSHOT_MALFORMED");
                    continue;
                };
                active_plan = Some(ActivePlan {
                    plan_id: plan_id.to_owned(),
                    plan_digest: plan_digest.to_owned(),
                    bound_revisions,
                    validated: false,
                    invalidated: false,
                });
            }
            "plan_validated" => {
                let requested_id = str_at(event, "plan_id");
                if let Some(plan) = active_plan.as_mut() {
                    if requested_id != Some(plan.plan_id.as_str()) {
                        push_once(&mut reasons, "PLAN_ID_MISMATCH");
                        plan.validated = false;
                        continue;
                    }
                    if str_at(event, "plan_digest") != Some(plan.plan_digest.as_str()) {
                        push_once(&mut reasons, "PLAN_DIGEST_MISMATCH");
                        plan.validated = false;
                        continue;
                    }
                    plan.validated = !plan.invalidated
                        && plan.bound_revisions == revisions
                        && gates_satisfied(&gates)
                        && reasons.is_empty();
                } else {
                    push_once(&mut reasons, "PLAN_NOT_ACTIVE");
                }
            }
            "state_changed" => {
                let revision_key = str_at(event, "revision_key").unwrap_or_default();
                let new_revision = event.get("new_revision").and_then(Value::as_u64);
                if !REVISION_KEYS.contains(&revision_key) {
                    push_once(&mut reasons, "UNKNOWN_REVISION_KEY");
                } else if let Some(new_revision) = new_revision {
                    let current = revisions.get(revision_key).copied().unwrap_or(0);
                    if new_revision <= current {
                        push_once(&mut reasons, "REVISION_NOT_MONOTONIC");
                    } else {
                        revisions.insert(revision_key.to_owned(), new_revision);
                    }
                } else {
                    push_once(&mut reasons, "REVISION_VALUE_INVALID");
                }

                match event.get("gate_effects").and_then(Value::as_object) {
                    Some(effects) => {
                        for (key, value) in effects {
                            if !GATE_KEYS.contains(&key.as_str()) {
                                push_once(&mut reasons, "UNKNOWN_GATE_EFFECT");
                            } else if let Some(status) = value.as_bool() {
                                gates.insert(key.clone(), status);
                            } else {
                                push_once(&mut reasons, "GATE_EFFECT_VALUE_INVALID");
                            }
                        }
                    }
                    None => push_once(&mut reasons, "GATE_EFFECTS_MALFORMED"),
                }
                if let Some(plan) = active_plan.as_mut() {
                    plan.invalidated = true;
                    plan.validated = false;
                }
            }
            "plan_cancelled" => {
                let requested_id = str_at(event, "plan_id");
                if active_plan
                    .as_ref()
                    .is_some_and(|plan| requested_id == Some(plan.plan_id.as_str()))
                {
                    active_plan = None;
                } else {
                    push_once(&mut reasons, "PLAN_CANCEL_ID_MISMATCH");
                }
            }
            "simulated_plan_accepted" => {
                let before_attempt = reasons.len();
                let requested_id = str_at(event, "plan_id");
                if let Some(plan) = active_plan.as_ref() {
                    if requested_id != Some(plan.plan_id.as_str()) {
                        push_once(&mut reasons, "PLAN_ID_MISMATCH");
                    } else if str_at(event, "plan_digest") != Some(plan.plan_digest.as_str()) {
                        push_once(&mut reasons, "PLAN_DIGEST_MISMATCH");
                    } else {
                        if plan.invalidated || plan.bound_revisions != revisions {
                            push_once(&mut reasons, "PLAN_VALIDATION_STALE");
                        } else if !plan.validated {
                            push_once(&mut reasons, "PLAN_NOT_VALIDATED");
                        }
                        if !gates_satisfied(&gates) {
                            push_once(&mut reasons, "HARD_GATE_UNSATISFIED");
                        }
                        if reasons.len() == before_attempt && !plan.validated {
                            push_once(&mut reasons, "PLAN_NOT_VALIDATED");
                        }
                    }
                } else {
                    push_once(&mut reasons, "PLAN_NOT_ACTIVE");
                }

                if reasons.len() == before_attempt && before_attempt == 0 {
                    accepted = true;
                    active_plan = None;
                } else if reasons.len() == before_attempt && before_attempt > 0 {
                    push_once(&mut reasons, "PLAN_ACCEPTANCE_AFTER_PRIOR_VIOLATION");
                }
            }
            "physical_actuation_attempted" => {
                push_once(&mut reasons, "PHYSICAL_ACTUATION_FORBIDDEN");
            }
            _ => {
                push_once(&mut reasons, "UNKNOWN_EVENT_KIND");
            }
        }
    }

    if !accepted && !events.iter().any(|event| {
        str_at(event, "kind") == Some("simulated_plan_accepted")
    }) {
        push_once(&mut reasons, "NO_ACCEPTED_SIMULATED_PLAN");
    }

    let decision = if accepted && reasons.is_empty() {
        ELIGIBLE
    } else {
        BLOCKED
    };
    Evaluation {
        decision,
        reason_codes: reasons,
    }
}

fn run_benchmark(root: &Value) -> Result<BenchmarkReport, Vec<String>> {
    let mut errors = Vec::new();
    if str_at(root, "suite_id") != Some("ROBOT_PLAN_EPISODE_INTEGRITY") {
        errors.push("suite_id missing or unsupported".to_owned());
    }
    if str_at(root, "suite_version") != Some("0.1.0") {
        errors.push("suite_version missing or unsupported".to_owned());
    }
    if str_at(root, "evidence_class") != Some("synthetic") {
        errors.push("evidence_class must remain synthetic".to_owned());
    }
    if str_at(root, "execution_mode") != Some("plan_eligibility_only_no_actuation") {
        errors.push("execution_mode must prohibit actuation".to_owned());
    }
    if str_at(root, "claim_ceiling").is_none() {
        errors.push("claim_ceiling must be explicit".to_owned());
    }

    let episodes = match root.get("episodes").and_then(Value::as_array) {
        Some(episodes) => episodes,
        None => {
            return Err(vec!["episodes must be a JSON array".to_owned()]);
        }
    };
    if root
        .get("expected_episode_count")
        .and_then(Value::as_u64)
        != Some(episodes.len() as u64)
    {
        errors.push("expected_episode_count does not match corpus length".to_owned());
    }
    let initial_state = match root.get("initial_state") {
        Some(state) => state,
        None => return Err(vec!["initial_state is missing".to_owned()]),
    };
    if initial_state
        .get("revisions")
        .and_then(parse_revisions)
        .is_none()
    {
        errors.push("initial_state.revisions is malformed".to_owned());
    }
    if initial_state.get("gates").and_then(parse_gates).is_none() {
        errors.push("initial_state.gates is malformed".to_owned());
    }

    let mut episode_ids = BTreeSet::new();
    let mut passed = 0usize;
    for episode in episodes {
        let episode_id = str_at(episode, "episode_id").unwrap_or("<missing-id>");
        if !episode_ids.insert(episode_id.to_owned()) {
            errors.push(format!("duplicate episode_id '{episode_id}'"));
        }
        let expected_decision = str_at(episode, "expected_decision");
        let expected_reasons = episode
            .get("expected_reason_codes")
            .and_then(Value::as_array)
            .and_then(|items| items.iter().map(|item| item.as_str().map(str::to_owned)).collect::<Option<Vec<_>>>());
        if expected_decision.is_none() || expected_reasons.is_none() {
            errors.push(format!("{episode_id}: expected decision/reason schema malformed"));
            continue;
        }
        if episode
            .get("expected_physical_actuation")
            .and_then(Value::as_bool)
            != Some(false)
        {
            errors.push(format!("{episode_id}: physical actuation must remain false"));
        }

        let actual = evaluate_episode(episode, initial_state);
        if Some(actual.decision) == expected_decision
            && expected_reasons.as_ref() == Some(&actual.reason_codes)
        {
            passed += 1;
        } else {
            errors.push(format!(
                "{episode_id}: actual decision={} reasons={:?}; expected decision={} reasons={:?}",
                actual.decision,
                actual.reason_codes,
                expected_decision.unwrap_or("<missing>"),
                expected_reasons.as_deref().unwrap_or(&[])
            ));
        }
    }

    Ok(BenchmarkReport {
        episode_count: episodes.len(),
        passed_episode_count: passed,
        failures: errors,
    })
}

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_CORPUS.to_owned());
    let corpus_bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("corpus read failed for '{path}': {error}");
            process::exit(2);
        }
    };
    if path == DEFAULT_CORPUS
        && sha256_hex(&corpus_bytes) != sha256_hex(CORPUS_SOURCE)
    {
        eprintln!("runtime corpus differs from the corpus embedded in the compiled oracle");
        process::exit(2);
    }
    let corpus: Value = match serde_json::from_slice(&corpus_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("corpus JSON rejected: {error}");
            process::exit(2);
        }
    };
    let report = match run_benchmark(&corpus) {
        Ok(report) => report,
        Err(errors) => {
            for error in errors {
                eprintln!("REJECT: {error}");
            }
            process::exit(2);
        }
    };
    let failed = !report.failures.is_empty();
    let receipt = json!({
        "receipt_schema": "robotics-episode-integrity-receipt-v0",
        "suite_id": str_at(&corpus, "suite_id").unwrap_or("unknown"),
        "suite_version": str_at(&corpus, "suite_version").unwrap_or("unknown"),
        "corpus_path": path,
        "corpus_sha256": sha256_hex(&corpus_bytes),
        "auditor_source_sha256": sha256_hex(AUDITOR_SOURCE),
        "embedded_corpus_sha256": sha256_hex(CORPUS_SOURCE),
        "episode_count": report.episode_count,
        "passed_episode_count": report.passed_episode_count,
        "failed_episode_count": report.failures.len(),
        "failures": report.failures,
        "execution_mode": "plan_eligibility_only_no_actuation",
        "claim_ceiling": CLAIM_CEILING
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("receipt serialization cannot fail")
    );
    if failed {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Value {
        serde_json::from_slice(include_bytes!(
            "../../../docs/tech/robotics-plan-episode-integrity-corpus-v0.1.json"
        ))
        .expect("committed episode corpus parses")
    }

    #[test]
    fn frozen_episode_corpus_matches_expected_dispositions() {
        let report = run_benchmark(&corpus()).expect("corpus schema is valid");
        assert_eq!(report.episode_count, 16);
        assert_eq!(report.passed_episode_count, 16);
        assert!(report.failures.is_empty(), "{:?}", report.failures);
    }

    #[test]
    fn deterministic_revision_suffix_mutations_invalidate_validated_plans() {
        let root = corpus();
        let initial = &root["initial_state"];
        let baseline = root["episodes"][0].clone();
        for mutation_index in 0..256usize {
            let key = REVISION_KEYS[mutation_index % REVISION_KEYS.len()];
            let offset = (mutation_index / REVISION_KEYS.len() + 1) as u64;
            let mut episode = baseline.clone();
            let old_revision = initial["revisions"][key]
                .as_u64()
                .expect("base revision");
            let events = episode["events"]
                .as_array_mut()
                .expect("event array");
            events[2]["sequence"] = json!(4);
            events.insert(
                2,
                json!({
                    "sequence": 3,
                    "kind": "state_changed",
                    "revision_key": key,
                    "new_revision": old_revision + offset,
                    "gate_effects": {}
                }),
            );
            let result = evaluate_episode(&episode, initial);
            assert_eq!(result.decision, BLOCKED, "{key} offset {offset}");
            assert!(result
                .reason_codes
                .iter()
                .any(|reason| reason == "PLAN_VALIDATION_STALE"));
        }
    }

    #[test]
    fn every_hard_gate_failure_after_validation_invalidates_the_plan() {
        let root = corpus();
        let initial = &root["initial_state"];
        let baseline = root["episodes"][0].clone();
        for (index, gate_key) in GATE_KEYS.iter().enumerate() {
            let mut episode = baseline.clone();
            let current_revision = initial["revisions"]["world"]
                .as_u64()
                .expect("world revision");
            let events = episode["events"]
                .as_array_mut()
                .expect("event array");
            events[2]["sequence"] = json!(4);
            events.insert(
                2,
                json!({
                    "sequence": 3,
                    "kind": "state_changed",
                    "revision_key": "world",
                    "new_revision": current_revision + 1 + index as u64,
                    "gate_effects": { (gate_key): false }
                }),
            );
            let result = evaluate_episode(&episode, initial);
            assert_eq!(result.decision, BLOCKED, "{gate_key}");
            assert!(
                result.reason_codes.iter().any(|r| r == "PLAN_VALIDATION_STALE"),
                "{gate_key}: {:?}",
                result.reason_codes
            );
            assert!(
                result.reason_codes.iter().any(|r| r == "HARD_GATE_UNSATISFIED"),
                "{gate_key}: {:?}",
                result.reason_codes
            );
        }
    }

    #[test]
    fn restoring_a_gate_does_not_restore_old_plan_authority() {
        let root = corpus();
        let initial = &root["initial_state"];
        let mut episode = root["episodes"][0].clone();
        let world_revision = initial["revisions"]["world"].as_u64().expect("world revision");
        let events = episode["events"].as_array_mut().expect("event array");
        events[2]["sequence"] = json!(5);
        events.insert(2, json!({
            "sequence": 3,
            "kind": "state_changed",
            "revision_key": "world",
            "new_revision": world_revision + 1,
            "gate_effects": { "authorization_valid": false }
        }));
        events.insert(3, json!({
            "sequence": 4,
            "kind": "state_changed",
            "revision_key": "world",
            "new_revision": world_revision + 2,
            "gate_effects": { "authorization_valid": true }
        }));
        let result = evaluate_episode(&episode, initial);
        assert_eq!(result.decision, BLOCKED);
        assert!(result.reason_codes.iter().any(|r| r == "PLAN_VALIDATION_STALE"));
    }

    #[test]
    fn embedded_corpus_and_frozen_test_corpus_are_identical() {
        let frozen = corpus();
        let canonical = serde_json::to_vec(&frozen).expect("test corpus serializes");
        let parsed_embedded: Value = serde_json::from_slice(CORPUS_SOURCE).expect("embedded corpus parses");
        let embedded = serde_json::to_vec(&parsed_embedded).expect("embedded corpus serializes");
        assert_eq!(canonical, embedded);
        assert_eq!(
            sha256_hex(CORPUS_SOURCE),
            sha256_hex(include_bytes!(
                "../../../docs/tech/robotics-plan-episode-integrity-corpus-v0.1.json"
            ))
        );
    }

    #[test]
    fn unknown_events_and_replayed_acceptance_fail_closed() {
        let root = corpus();
        let initial = &root["initial_state"];
        let unknown = evaluate_episode(&root["episodes"][13], initial);
        assert_eq!(unknown.decision, BLOCKED);
        assert!(unknown.reason_codes.iter().any(|r| r == "UNKNOWN_EVENT_KIND"));

        let replay = evaluate_episode(&root["episodes"][11], initial);
        assert_eq!(replay.decision, BLOCKED);
        assert!(replay.reason_codes.iter().any(|r| r == "PLAN_NOT_ACTIVE"));
    }

    #[test]
    fn an_invalid_sequence_is_rejected_without_sorting() {
        let root = corpus();
        let initial = &root["initial_state"];
        let result = evaluate_episode(&root["episodes"][10], initial);
        assert_eq!(result.decision, BLOCKED);
        assert_eq!(result.reason_codes, vec!["EVENT_SEQUENCE_NOT_MONOTONIC"]);
    }

    #[test]
    fn plan_id_reuse_with_different_content_digest_is_rejected() {
        let root = corpus();
        let result = evaluate_episode(&root["episodes"][15], &root["initial_state"]);
        assert_eq!(result.decision, BLOCKED);
        assert_eq!(result.reason_codes, vec!["PLAN_DIGEST_MISMATCH"]);
    }

    #[test]
    fn malformed_plan_digest_is_rejected() {
        let root = corpus();
        let mut episode = root["episodes"][0].clone();
        episode["events"][0]["plan_digest"] = json!("not-a-digest");
        let result = evaluate_episode(&episode, &root["initial_state"]);
        assert_eq!(result.decision, BLOCKED);
        assert!(result.reason_codes.iter().any(|r| r == "PLAN_DIGEST_INVALID"));
    }

    #[test]
    fn source_and_corpus_digests_are_sha256_length() {
        assert_eq!(sha256_hex(AUDITOR_SOURCE).len(), 64);
        assert_eq!(sha256_hex(CORPUS_SOURCE).len(), 64);
    }
}
