//! Offline, append-only state-transition auditor for proposed manufacturing interventions.
//! It provides no machine interface and cannot authorize or execute an intervention.
//! Authorization signatures/artifact bytes must be verified by an external trust gateway.
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const HEADER: [&str; 17] = [
    "action_id",
    "sequence_no",
    "prior_state",
    "next_state",
    "proposal_id",
    "nonce",
    "context_sha256",
    "event_utc",
    "valid_until_utc",
    "actor_id",
    "model_version",
    "policy_revision",
    "authorization_sha256",
    "execution_sha256",
    "postcondition_sha256",
    "instrument_id",
    "detail_sha256",
];

const REPORT_HEADER: &str =
    "action_id,proposal_id,final_state,event_count,context_sha256,nonce,authorization_present,execution_present,postcondition_present,closed_verified\n";

#[derive(Clone, Debug)]
struct Event {
    action_id: String,
    sequence: u64,
    prior_state: String,
    next_state: String,
    proposal_id: String,
    nonce: String,
    context_sha256: String,
    event_utc: String,
    event_epoch: i64,
    valid_until_utc: String,
    valid_until_epoch: i64,
    actor_id: String,
    model_version: String,
    policy_revision: String,
    authorization_sha256: String,
    execution_sha256: String,
    postcondition_sha256: String,
    instrument_id: String,
    detail_sha256: String,
}

#[derive(Clone, Debug)]
struct ActionState {
    proposal_id: String,
    nonce: String,
    context_sha256: String,
    valid_until_utc: String,
    valid_until_epoch: i64,
    last_event_epoch: i64,
    next_sequence: u64,
    final_state: String,
    event_count: u64,
    model_version: Option<String>,
    policy_revision: Option<String>,
    authorization_sha256: Option<String>,
    execution_sha256: Option<String>,
    postcondition_sha256: Option<String>,
    postcondition_instrument_id: Option<String>,
}

#[derive(Clone, Debug)]
struct Report {
    action_id: String,
    proposal_id: String,
    final_state: String,
    event_count: u64,
    context_sha256: String,
    nonce: String,
    authorization_present: bool,
    execution_present: bool,
    postcondition_present: bool,
    closed_verified: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("assured-intervention-audit: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(format!(
            "usage: {} <intervention-events.csv> <audit-report.csv>",
            args.first().map(String::as_str).unwrap_or("assured-intervention-audit")
        ));
    }
    let content = fs::read_to_string(&args[1])
        .map_err(|error| format!("cannot read {}: {error}", args[1]))?;
    let events = parse_events(&content)?;
    let reports = audit_events(&events)?;
    write_report(&args[2], &reports)?;
    let closed = reports.iter().filter(|report| report.closed_verified).count();
    let incomplete = reports.len() - closed;
    eprintln!(
        "intervention audit complete: actions={}, independently_closed={}, open_or_exception={}; no authorization or execution was performed",
        reports.len(), closed, incomplete
    );
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_utc(value: &str, context: &str) -> Result<i64, String> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return Err(format!("{context}: expected canonical YYYY-MM-DDTHH:MM:SSZ"));
    }
    let part = |range: std::ops::Range<usize>| -> Result<i32, String> {
        if !bytes[range.clone()].iter().all(u8::is_ascii_digit) {
            return Err(format!("{context}: timestamp field is not numeric"));
        }
        std::str::from_utf8(&bytes[range])
            .map_err(|_| format!("{context}: invalid timestamp"))?
            .parse()
            .map_err(|_| format!("{context}: invalid timestamp number"))
    };
    let year = part(0..4)?;
    let month = part(5..7)?;
    let day = part(8..10)?;
    let hour = part(11..13)?;
    let minute = part(14..16)?;
    let second = part(17..19)?;
    if !(1900..=2200).contains(&year) || !(1..=12).contains(&month) {
        return Err(format!("{context}: year or month out of range"));
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => unreachable!(),
    };
    if !(1..=month_days).contains(&day)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return Err(format!("{context}: invalid calendar or time value"));
    }

    let mut days = 0_i64;
    if year >= 1970 {
        for current in 1970..year {
            days += if current % 4 == 0 && (current % 100 != 0 || current % 400 == 0) {
                366
            } else {
                365
            };
        }
    } else {
        for current in year..1970 {
            days -= if current % 4 == 0 && (current % 100 != 0 || current % 400 == 0) {
                366
            } else {
                365
            };
        }
    }
    for current_month in 1..month {
        days += match current_month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => unreachable!(),
        };
    }
    days += i64::from(day - 1);
    Ok(days * 86_400 + i64::from(hour * 3_600 + minute * 60 + second))
}

fn parse_events(content: &str) -> Result<Vec<Event>, String> {
    let mut lines = content.lines();
    let header = lines
        .next()
        .ok_or_else(|| "missing CSV header".to_string())?
        .trim_end_matches('\r');
    if header != HEADER.join(",") {
        return Err(format!("unexpected CSV header; expected {}", HEADER.join(",")));
    }

    let mut events = Vec::new();
    for (index, raw) in lines.enumerate() {
        let row_number = index + 2;
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() || line.contains('"') {
            return Err(format!("row {row_number}: blank rows and quoted CSV fields are unsupported"));
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() != HEADER.len() {
            return Err(format!(
                "row {row_number}: expected {} columns, got {}",
                HEADER.len(),
                fields.len()
            ));
        }
        let nonempty = |index: usize, name: &str| -> Result<String, String> {
            if fields[index].is_empty() {
                Err(format!("row {row_number}: {name} is empty"))
            } else {
                Ok(fields[index].to_string())
            }
        };
        let sequence = fields[1]
            .parse::<u64>()
            .map_err(|_| format!("row {row_number}: invalid sequence_no"))?;
        if sequence == 0 {
            return Err(format!("row {row_number}: sequence_no must be positive"));
        }
        let event_utc = nonempty(7, "event_utc")?;
        let event_epoch = parse_utc(&event_utc, "event_utc")?;
        let valid_until_utc = nonempty(8, "valid_until_utc")?;
        let valid_until_epoch = parse_utc(&valid_until_utc, "valid_until_utc")?;
        let context_sha256 = nonempty(6, "context_sha256")?;
        if !valid_sha256(&context_sha256) {
            return Err(format!("row {row_number}: malformed context_sha256"));
        }
        let detail_sha256 = nonempty(16, "detail_sha256")?;
        if !valid_sha256(&detail_sha256) {
            return Err(format!("row {row_number}: malformed detail_sha256"));
        }
        for (column, name) in [
            (12, "authorization_sha256"),
            (13, "execution_sha256"),
            (14, "postcondition_sha256"),
        ] {
            if !fields[column].is_empty() && !valid_sha256(fields[column]) {
                return Err(format!("row {row_number}: malformed {name}"));
            }
        }
        events.push(Event {
            action_id: nonempty(0, "action_id")?,
            sequence,
            prior_state: nonempty(2, "prior_state")?,
            next_state: nonempty(3, "next_state")?,
            proposal_id: nonempty(4, "proposal_id")?,
            nonce: nonempty(5, "nonce")?,
            context_sha256,
            event_utc,
            event_epoch,
            valid_until_utc,
            valid_until_epoch,
            actor_id: nonempty(9, "actor_id")?,
            model_version: fields[10].to_string(),
            policy_revision: fields[11].to_string(),
            authorization_sha256: fields[12].to_string(),
            execution_sha256: fields[13].to_string(),
            postcondition_sha256: fields[14].to_string(),
            instrument_id: fields[15].to_string(),
            detail_sha256,
        });
    }
    if events.is_empty() {
        return Err("event file has no records".to_string());
    }
    Ok(events)
}

fn is_terminal(state: &str) -> bool {
    matches!(state, "CLOSED" | "REJECTED" | "EXPIRED" | "RECOVERY_REQUIRED")
}

fn audit_events(events: &[Event]) -> Result<Vec<Report>, String> {
    let mut actions: BTreeMap<String, ActionState> = BTreeMap::new();
    let mut seen_nonces = BTreeSet::new();
    let mut seen_proposals = BTreeSet::new();

    for event in events {
        if let Some(state) = actions.get_mut(&event.action_id) {
            if is_terminal(&state.final_state) {
                return Err(format!(
                    "action {:?}: event follows terminal state {}",
                    event.action_id, state.final_state
                ));
            }
            if event.sequence != state.next_sequence {
                return Err(format!(
                    "action {:?}: expected sequence {}, got {}",
                    event.action_id, state.next_sequence, event.sequence
                ));
            }
            if event.prior_state != state.final_state {
                return Err(format!(
                    "action {:?}: prior_state {:?} does not match audited state {:?}",
                    event.action_id, event.prior_state, state.final_state
                ));
            }
            if event.proposal_id != state.proposal_id
                || event.nonce != state.nonce
                || event.context_sha256 != state.context_sha256
                || event.valid_until_utc != state.valid_until_utc
            {
                return Err(format!(
                    "action {:?}: proposal, nonce, context digest or expiry changed mid-chain",
                    event.action_id
                ));
            }
            if event.event_epoch < state.last_event_epoch {
                return Err(format!("action {:?}: event timestamps moved backwards", event.action_id));
            }

            match (event.prior_state.as_str(), event.next_state.as_str()) {
                ("PROPOSED", "MODEL_VALIDATED") => {
                    if event.model_version.is_empty() {
                        return Err(format!("action {:?}: model validation lacks model_version", event.action_id));
                    }
                    state.model_version = Some(event.model_version.clone());
                }
                ("MODEL_VALIDATED", "CONSTRAINT_CHECKED") => {
                    if event.policy_revision.is_empty() {
                        return Err(format!("action {:?}: constraint check lacks policy_revision", event.action_id));
                    }
                    state.policy_revision = Some(event.policy_revision.clone());
                }
                ("CONSTRAINT_CHECKED", "AUTHORIZED") => {
                    if state.model_version.is_none() || state.policy_revision.is_none() {
                        return Err(format!("action {:?}: authorization precedes model or policy evidence", event.action_id));
                    }
                    if event.authorization_sha256.is_empty()
                        || !valid_sha256(&event.authorization_sha256)
                    {
                        return Err(format!("action {:?}: authorization lacks a valid receipt digest", event.action_id));
                    }
                    if event.event_epoch > state.valid_until_epoch {
                        return Err(format!("action {:?}: authorization occurred after proposal expiry", event.action_id));
                    }
                    state.authorization_sha256 = Some(event.authorization_sha256.clone());
                }
                ("AUTHORIZED", "EXECUTED") => {
                    if state.authorization_sha256.is_none() {
                        return Err(format!("action {:?}: execution has no prior authorization", event.action_id));
                    }
                    if event.event_epoch > state.valid_until_epoch {
                        return Err(format!("action {:?}: execution occurred after authorization expiry", event.action_id));
                    }
                    if event.execution_sha256.is_empty() || !valid_sha256(&event.execution_sha256) {
                        return Err(format!("action {:?}: execution lacks a valid machine receipt digest", event.action_id));
                    }
                    state.execution_sha256 = Some(event.execution_sha256.clone());
                }
                ("EXECUTED", "POSTCONDITION_VERIFIED") => {
                    if state.execution_sha256.is_none() {
                        return Err(format!("action {:?}: postcondition precedes execution receipt", event.action_id));
                    }
                    if event.postcondition_sha256.is_empty()
                        || !valid_sha256(&event.postcondition_sha256)
                        || event.instrument_id.is_empty()
                    {
                        return Err(format!("action {:?}: postcondition lacks independent measurement evidence", event.action_id));
                    }
                    state.postcondition_sha256 = Some(event.postcondition_sha256.clone());
                    state.postcondition_instrument_id = Some(event.instrument_id.clone());
                }
                ("POSTCONDITION_VERIFIED", "CLOSED") => {
                    if state.postcondition_sha256.is_none()
                        || state.postcondition_instrument_id.is_none()
                    {
                        return Err(format!("action {:?}: CLOSED without independent postcondition", event.action_id));
                    }
                }
                ("PROPOSED" | "MODEL_VALIDATED" | "CONSTRAINT_CHECKED", "REJECTED") => {}
                ("AUTHORIZED", "EXPIRED") => {
                    if event.event_epoch <= state.valid_until_epoch {
                        return Err(format!("action {:?}: EXPIRED state recorded before expiry", event.action_id));
                    }
                }
                ("AUTHORIZED" | "EXECUTED", "ABORTED") => {}
                ("EXECUTED" | "ABORTED", "RECOVERY_REQUIRED") => {}
                (prior, next) => {
                    return Err(format!(
                        "action {:?}: forbidden state transition {prior} -> {next}",
                        event.action_id
                    ));
                }
            }
            state.final_state = event.next_state.clone();
            state.last_event_epoch = event.event_epoch;
            state.next_sequence += 1;
            state.event_count += 1;
        } else {
            if event.sequence != 1 || event.prior_state != "NONE" || event.next_state != "PROPOSED" {
                return Err(format!(
                    "action {:?}: first event must be NONE -> PROPOSED at sequence 1",
                    event.action_id
                ));
            }
            if event.valid_until_epoch <= event.event_epoch {
                return Err(format!("action {:?}: proposal is already expired at creation", event.action_id));
            }
            if !seen_nonces.insert(event.nonce.clone()) {
                return Err(format!("action {:?}: nonce reused by another action", event.action_id));
            }
            if !seen_proposals.insert(event.proposal_id.clone()) {
                return Err(format!("action {:?}: proposal_id is duplicated", event.action_id));
            }
            actions.insert(
                event.action_id.clone(),
                ActionState {
                    proposal_id: event.proposal_id.clone(),
                    nonce: event.nonce.clone(),
                    context_sha256: event.context_sha256.clone(),
                    valid_until_utc: event.valid_until_utc.clone(),
                    valid_until_epoch: event.valid_until_epoch,
                    last_event_epoch: event.event_epoch,
                    next_sequence: 2,
                    final_state: "PROPOSED".to_string(),
                    event_count: 1,
                    model_version: None,
                    policy_revision: None,
                    authorization_sha256: None,
                    execution_sha256: None,
                    postcondition_sha256: None,
                    postcondition_instrument_id: None,
                },
            );
        }
    }

    Ok(actions
        .into_iter()
        .map(|(action_id, state)| Report {
            action_id,
            proposal_id: state.proposal_id,
            final_state: state.final_state.clone(),
            event_count: state.event_count,
            context_sha256: state.context_sha256,
            nonce: state.nonce,
            authorization_present: state.authorization_sha256.is_some(),
            execution_present: state.execution_sha256.is_some(),
            postcondition_present: state.postcondition_sha256.is_some(),
            closed_verified: state.final_state == "CLOSED"
                && state.authorization_sha256.is_some()
                && state.execution_sha256.is_some()
                && state.postcondition_sha256.is_some()
                && state.postcondition_instrument_id.is_some(),
        })
        .collect())
}

fn write_report(path: &str, reports: &[Report]) -> Result<(), String> {
    let mut output = String::from(REPORT_HEADER);
    for report in reports {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            report.action_id,
            report.proposal_id,
            report.final_state,
            report.event_count,
            report.context_sha256,
            report.nonce,
            report.authorization_present,
            report.execution_present,
            report.postcondition_present,
            report.closed_verified
        ));
    }
    fs::write(path, output).map_err(|error| format!("cannot write {path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const DIGEST_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const DIGEST_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    fn event(
        sequence: u64,
        prior: &str,
        next: &str,
        timestamp: &str,
        model: &str,
        policy: &str,
        authorization: &str,
        execution: &str,
        postcondition: &str,
        instrument: &str,
        detail: &str,
        nonce: &str,
        context: &str,
    ) -> Event {
        Event {
            action_id: "ACT-001".to_string(),
            sequence,
            prior_state: prior.to_string(),
            next_state: next.to_string(),
            proposal_id: "PROP-001".to_string(),
            nonce: nonce.to_string(),
            context_sha256: context.to_string(),
            event_utc: timestamp.to_string(),
            event_epoch: parse_utc(timestamp, "test").unwrap(),
            valid_until_utc: "2026-10-31T00:00:00Z".to_string(),
            valid_until_epoch: parse_utc("2026-10-31T00:00:00Z", "test").unwrap(),
            actor_id: "actor-test".to_string(),
            model_version: model.to_string(),
            policy_revision: policy.to_string(),
            authorization_sha256: authorization.to_string(),
            execution_sha256: execution.to_string(),
            postcondition_sha256: postcondition.to_string(),
            instrument_id: instrument.to_string(),
            detail_sha256: detail.to_string(),
        }
    }

    fn valid_chain() -> Vec<Event> {
        vec![
            event(1, "NONE", "PROPOSED", "2026-10-30T09:00:00Z", "", "", "", "", "", "", DIGEST_A, "nonce-1", DIGEST_A),
            event(2, "PROPOSED", "MODEL_VALIDATED", "2026-10-30T09:01:00Z", "thermal-model-v3", "", "", "", "", "", DIGEST_B, "nonce-1", DIGEST_A),
            event(3, "MODEL_VALIDATED", "CONSTRAINT_CHECKED", "2026-10-30T09:02:00Z", "", "ROUTE-A-POLICY-7", "", "", "", "", DIGEST_C, "nonce-1", DIGEST_A),
            event(4, "CONSTRAINT_CHECKED", "AUTHORIZED", "2026-10-30T09:03:00Z", "", "", DIGEST_D, "", "", "", DIGEST_D, "nonce-1", DIGEST_A),
            event(5, "AUTHORIZED", "EXECUTED", "2026-10-30T09:04:00Z", "", "", "", DIGEST_E, "", "", DIGEST_E, "nonce-1", DIGEST_A),
            event(6, "EXECUTED", "POSTCONDITION_VERIFIED", "2026-10-30T09:05:00Z", "", "", "", "", DIGEST_B, "POWER-ANALYZER-01", DIGEST_B, "nonce-1", DIGEST_A),
            event(7, "POSTCONDITION_VERIFIED", "CLOSED", "2026-10-30T09:06:00Z", "", "", "", "", "", "", DIGEST_C, "nonce-1", DIGEST_A),
        ]
    }

    #[test]
    fn accepts_complete_authorized_executed_and_independently_verified_chain() {
        let report = audit_events(&valid_chain()).unwrap();
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].final_state, "CLOSED");
        assert!(report[0].closed_verified);
        assert!(report[0].authorization_present);
        assert!(report[0].execution_present);
        assert!(report[0].postcondition_present);
    }

    #[test]
    fn refuses_close_without_verified_postcondition() {
        let mut events = valid_chain();
        events[5].postcondition_sha256.clear();
        events[5].instrument_id.clear();
        assert!(audit_events(&events).unwrap_err().contains("postcondition lacks independent measurement evidence"));

        let mut events = valid_chain();
        events[6].prior_state = "EXECUTED".to_string();
        assert!(audit_events(&events).unwrap_err().contains("does not match audited state"));
    }

    #[test]
    fn refuses_replay_across_actions_and_context_mutation() {
        let mut replay = valid_chain();
        let mut second = valid_chain()[0].clone();
        second.action_id = "ACT-002".to_string();
        second.proposal_id = "PROP-002".to_string();
        replay.push(second);
        assert!(audit_events(&replay).unwrap_err().contains("nonce reused"));

        let mut changed = valid_chain();
        changed[3].context_sha256 = DIGEST_B.to_string();
        assert!(audit_events(&changed).unwrap_err().contains("context digest"));
    }

    #[test]
    fn refuses_expired_authorization_and_execution_after_expiry() {
        let mut events = valid_chain();
        events[3].event_epoch = parse_utc("2026-11-01T00:00:00Z", "test").unwrap();
        events[3].event_utc = "2026-11-01T00:00:00Z".to_string();
        assert!(audit_events(&events).unwrap_err().contains("authorization occurred after proposal expiry"));

        let mut events = valid_chain();
        events[4].event_epoch = parse_utc("2026-11-01T00:00:00Z", "test").unwrap();
        events[4].event_utc = "2026-11-01T00:00:00Z".to_string();
        assert!(audit_events(&events).unwrap_err().contains("timestamps moved backwards") || audit_events(&events).unwrap_err().contains("execution occurred after authorization expiry"));
    }

    #[test]
    fn rejects_invalid_transitions_and_duplicate_proposal_ids() {
        let mut events = valid_chain();
        events[5].next_state = "CLOSED".to_string();
        assert!(audit_events(&events).unwrap_err().contains("forbidden state transition"));

        let mut events = valid_chain();
        let mut second = valid_chain()[0].clone();
        second.action_id = "ACT-002".to_string();
        second.nonce = "nonce-2".to_string();
        events.push(second);
        assert!(audit_events(&events).unwrap_err().contains("proposal_id is duplicated"));
    }

    #[test]
    fn utc_parser_rejects_offsets_and_impossible_dates() {
        assert!(parse_utc("2024-02-29T23:59:59Z", "test").is_ok());
        assert!(parse_utc("2025-02-29T23:59:59Z", "test").is_err());
        assert!(parse_utc("2026-10-30T09:00:00+02:00", "test").is_err());
    }
}
