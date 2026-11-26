use maritime_decision_time_replay::{
    adjudicate, build_policy_input, CargoLedger, EvaluatorTruth, FirstVisibleEvidencePolicy,
    GateState, Policy, ReplayCase, SourceEvent,
};
use std::error::Error;

const FIXTURES: &str = include_str!("../../fixtures/episodes.tsv");

fn parse_i64(value: &str, field: &str) -> Result<i64, Box<dyn Error>> {
    value
        .parse::<i64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_u64(value: &str, field: &str) -> Result<u64, Box<dyn Error>> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_bool(value: &str, field: &str) -> Result<bool, Box<dyn Error>> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid {field}: {value}").into()),
    }
}

fn parse_case(line: &str, line_number: usize) -> Result<ReplayCase, Box<dyn Error>> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() != 23 {
        return Err(format!(
            "fixture line {line_number}: expected 23 fields, got {}",
            fields.len()
        )
        .into());
    }

    Ok(ReplayCase {
        case_id: fields[0].to_owned(),
        decision_cutoff: parse_i64(fields[1], "decision_cutoff")?,
        subject_id: fields[2].to_owned(),
        source_event: SourceEvent {
            source_id: fields[3].to_owned(),
            event_id: fields[4].to_owned(),
            event_subject_id: fields[5].to_owned(),
            event_time: parse_i64(fields[6], "event_time")?,
            published_at: parse_i64(fields[7], "published_at")?,
            retrieved_at: parse_i64(fields[8], "retrieved_at")?,
            decision_use_from: parse_i64(fields[9], "decision_use_from")?,
            decision_use_until: parse_i64(fields[10], "decision_use_until")?,
            rights_clear: parse_bool(fields[11], "rights_clear")?,
        },
        evaluator_truth: EvaluatorTruth {
            physical: GateState::parse(fields[12])?,
            authority: GateState::parse(fields[13])?,
            participant_floor: GateState::parse(fields[14])?,
            cargo: CargoLedger {
                expected_units: parse_u64(fields[15], "expected_units")?,
                receiver_accepted_units: parse_u64(fields[16], "receiver_accepted_units")?,
                in_transit_units: parse_u64(fields[17], "in_transit_units")?,
                outstanding_units: parse_u64(fields[18], "outstanding_units")?,
                unresolved_units: parse_u64(fields[19], "unresolved_units")?,
                duplicated_units: parse_u64(fields[20], "duplicated_units")?,
            },
        },
        expected_disposition: fields[21].to_owned(),
        expected_reason: fields[22].to_owned(),
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let policy = FirstVisibleEvidencePolicy;
    let mut count = 0_usize;

    println!("MDTR_RECEIPT\t0.1.0");
    println!("case_id\tvisible_count\tpolicy_action\tevidence_ids\tdisposition\treason");

    for (index, raw_line) in FIXTURES.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let case = parse_case(line, index + 1)?;
        let policy_input = build_policy_input(&case);
        let action = policy.decide(&policy_input);
        let evaluation = adjudicate(
            &case.source_event,
            &case.subject_id,
            &policy_input,
            &action,
            &case.evaluator_truth,
        );

        let disposition = evaluation.disposition.as_str();
        let reason = evaluation.reason_string();
        if disposition != case.expected_disposition || reason != case.expected_reason {
            return Err(format!(
                "{}: expected {}/{} but received {}/{}",
                case.case_id, case.expected_disposition, case.expected_reason, disposition, reason
            )
            .into());
        }

        println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            case.case_id,
            policy_input.visible_events.len(),
            action.label(),
            action.evidence_ids(),
            disposition,
            reason
        );
        count += 1;
    }

    if count == 0 {
        return Err("empty MDTR fixture corpus".into());
    }

    println!("SUMMARY\tPASS\t{count}/{count}");
    Ok(())
}
