// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite set-map validator for passive mechanical docking capture envelopes.
//! It does not model contact mechanics or establish pose uniqueness.

use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process::ExitCode};

const HEADER: [&str; 15] = [
    "case_id",
    "evidence_class",
    "decision_time_us",
    "approach_set_sha256",
    "receiver_config_sha256",
    "guide_geometry_sha256",
    "frame_sha256",
    "calibration_sha256",
    "initial_approach_states",
    "approach_state_id",
    "post_state_id",
    "post_pose_id",
    "outcome",
    "transition_in_domain",
    "valid_until_us",
];
const MAX_APPROACH_STATES: usize = 64;
const MAX_TRANSITIONS: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Transition {
    post_state_id: String,
    post_pose_id: String,
    outcome: String,
    in_domain: bool,
    valid_until_us: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Case {
    case_id: String,
    evidence_class: String,
    decision_time_us: u64,
    approach_set_sha256: String,
    receiver_config_sha256: String,
    guide_geometry_sha256: String,
    frame_sha256: String,
    calibration_sha256: String,
    initial_approach_states: BTreeSet<String>,
    transitions: BTreeMap<String, Vec<Transition>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ApproachResult {
    approach_state_id: String,
    disposition: &'static str,
    possible_post_state_ids: BTreeSet<String>,
    possible_post_pose_ids: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evaluation {
    disposition: &'static str,
    approaches: Vec<ApproachResult>,
    capture_only_approach_count: usize,
    residual_captured_pose_ids: BTreeSet<String>,
    minimum_valid_until_us: u64,
}

fn parse_u64(value: &str, name: &str) -> Result<u64, String> {
    if value.is_empty() || value.trim() != value {
        return Err(format!("{name}: empty or untrimmed integer"));
    }
    value
        .parse::<u64>()
        .map_err(|_| format!("{name}: expected non-negative integer"))
}

fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn parse_set(value: &str, name: &str) -> Result<BTreeSet<String>, String> {
    let parts: Vec<&str> = value.split(';').collect();
    if parts.is_empty()
        || parts
            .iter()
            .any(|part| !valid_identifier(part))
    {
        return Err(format!("{name}: empty, untrimmed or invalid set member"));
    }
    let parsed: BTreeSet<String> = parts.iter().map(|part| (*part).to_string()).collect();
    if parsed.len() != parts.len() {
        return Err(format!("{name}: duplicate set member"));
    }
    Ok(parsed)
}

fn allowed_outcome(value: &str) -> bool {
    matches!(
        value,
        "CAPTURED"
            | "NOT_CAPTURED"
            | "JAMMED"
            | "REBOUND"
            | "SLIP"
            | "OUT_OF_DOMAIN"
            | "UNKNOWN"
    )
}

fn parse_case(input: &str) -> Result<Case, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else {
        return Err(vec!["input is empty".to_string()]);
    };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != HEADER.as_slice() {
        return Err(vec![format!(
            "header must contain exactly {} declared columns",
            HEADER.len()
        )]);
    }

    let mut metadata: Option<Vec<String>> = None;
    let mut case: Option<Case> = None;
    let mut post_pose_map = BTreeMap::<String, String>::new();
    let mut unique_edges = BTreeSet::<(String, String)>::new();
    let mut errors = Vec::new();
    let mut transition_count = 0usize;

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("line {line_no}: blank rows are not allowed"));
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != HEADER.len() {
            errors.push(format!(
                "line {line_no}: expected {} columns, found {}",
                HEADER.len(),
                fields.len()
            ));
            continue;
        }

        let row_metadata: Vec<String> =
            fields[..9].iter().map(|field| (*field).to_string()).collect();
        if let Some(first) = &metadata {
            if *first != row_metadata {
                errors.push(format!(
                    "line {line_no}: case identity, revision, frame, calibration or approach-set metadata differs"
                ));
                continue;
            }
        } else {
            metadata = Some(row_metadata);
            let decision_time_us = match parse_u64(fields[2], "decision_time_us") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(format!("line {line_no}: {error}"));
                    continue;
                }
            };
            let initial_approach_states =
                match parse_set(fields[8], "initial_approach_states") {
                    Ok(values) => values,
                    Err(error) => {
                        errors.push(format!("line {line_no}: {error}"));
                        continue;
                    }
                };

            if !valid_identifier(fields[0])
                || fields[1] != "SYNTHETIC_KNOWN_TRUTH"
                || !digest_valid(fields[3])
                || !digest_valid(fields[4])
                || !digest_valid(fields[5])
                || !digest_valid(fields[6])
                || !digest_valid(fields[7])
                || initial_approach_states.is_empty()
                || initial_approach_states.len() > MAX_APPROACH_STATES
            {
                errors.push(format!(
                    "line {line_no}: invalid case metadata, digest or initial approach-state set"
                ));
                continue;
            }

            case = Some(Case {
                case_id: fields[0].to_string(),
                evidence_class: fields[1].to_string(),
                decision_time_us,
                approach_set_sha256: fields[3].to_string(),
                receiver_config_sha256: fields[4].to_string(),
                guide_geometry_sha256: fields[5].to_string(),
                frame_sha256: fields[6].to_string(),
                calibration_sha256: fields[7].to_string(),
                initial_approach_states,
                transitions: BTreeMap::new(),
            });
        }

        let Some(parsed_case) = case.as_mut() else {
            errors.push(format!("line {line_no}: case metadata could not be initialized"));
            continue;
        };

        let approach_state_id = fields[9];
        let post_state_id = fields[10];
        let post_pose_id = fields[11];
        let outcome = fields[12];
        if !valid_identifier(approach_state_id)
            || !valid_identifier(post_state_id)
            || !valid_identifier(post_pose_id)
            || !allowed_outcome(outcome)
        {
            errors.push(format!("line {line_no}: invalid state, pose or outcome identifier"));
            continue;
        }
        if !parsed_case
            .initial_approach_states
            .contains(approach_state_id)
        {
            errors.push(format!(
                "line {line_no}: approach state {approach_state_id} is not in the declared initial set"
            ));
            continue;
        }

        let in_domain = match fields[13] {
            "true" => true,
            "false" => false,
            other => {
                errors.push(format!(
                    "line {line_no}: transition_in_domain must be true or false, found {other:?}"
                ));
                continue;
            }
        };
        if outcome == "OUT_OF_DOMAIN" && in_domain {
            errors.push(format!(
                "line {line_no}: OUT_OF_DOMAIN outcome contradicts transition_in_domain=true"
            ));
            continue;
        }

        let valid_until_us = match parse_u64(fields[14], "valid_until_us") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        if valid_until_us < parsed_case.decision_time_us {
            errors.push(format!(
                "line {line_no}: capture-envelope evidence expired before decision time"
            ));
            continue;
        }

        if let Some(existing_pose) = post_pose_map.get(post_state_id) {
            if existing_pose != post_pose_id {
                errors.push(format!(
                    "line {line_no}: post_state_id {post_state_id} maps to conflicting pose IDs"
                ));
                continue;
            }
        } else {
            post_pose_map.insert(post_state_id.to_string(), post_pose_id.to_string());
        }

        let edge_key = (approach_state_id.to_string(), post_state_id.to_string());
        if !unique_edges.insert(edge_key) {
            errors.push(format!(
                "line {line_no}: duplicate approach->post-state transition edge"
            ));
            continue;
        }

        parsed_case
            .transitions
            .entry(approach_state_id.to_string())
            .or_default()
            .push(Transition {
                post_state_id: post_state_id.to_string(),
                post_pose_id: post_pose_id.to_string(),
                outcome: outcome.to_string(),
                in_domain,
                valid_until_us,
            });
        transition_count += 1;
        if transition_count > MAX_TRANSITIONS {
            errors.push(format!("case exceeds {MAX_TRANSITIONS} transition rows"));
            break;
        }
    }

    let Some(mut parsed_case) = case else {
        return Err(if errors.is_empty() {
            vec!["no transition rows found".to_string()]
        } else {
            errors
        });
    };

    for approach_state in &parsed_case.initial_approach_states {
        if !parsed_case.transitions.contains_key(approach_state) {
            errors.push(format!(
                "declared approach state {approach_state} has no possible transition rows"
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    for transitions in parsed_case.transitions.values_mut() {
        transitions.sort();
    }
    Ok(parsed_case)
}

fn evaluate(case: &Case) -> Evaluation {
    let mut approaches = Vec::new();
    let mut capture_only_approach_count = 0usize;
    let mut residual_captured_pose_ids = BTreeSet::new();
    let mut any_out_of_domain = false;
    let mut any_unknown = false;

    for approach_state_id in &case.initial_approach_states {
        let transitions = case
            .transitions
            .get(approach_state_id)
            .expect("parser establishes transition coverage");

        let possible_post_pose_ids: BTreeSet<String> = transitions
            .iter()
            .map(|transition| transition.post_pose_id.clone())
            .collect();
        let possible_post_state_ids: BTreeSet<String> = transitions
            .iter()
            .map(|transition| transition.post_state_id.clone())
            .collect();
        let has_out_of_domain = transitions
            .iter()
            .any(|transition| !transition.in_domain || transition.outcome == "OUT_OF_DOMAIN");
        let has_unknown = transitions
            .iter()
            .any(|transition| transition.outcome == "UNKNOWN");
        let all_captured = transitions
            .iter()
            .all(|transition| transition.in_domain && transition.outcome == "CAPTURED");
        for transition in transitions {
            if transition.in_domain && transition.outcome == "CAPTURED" {
                residual_captured_pose_ids.insert(transition.post_pose_id.clone());
            }
        }

        let disposition = if has_out_of_domain {
            any_out_of_domain = true;
            "OUT_OF_DOMAIN_POSSIBILITY"
        } else if has_unknown {
            any_unknown = true;
            "UNKNOWN_POSSIBILITY"
        } else if all_captured {
            capture_only_approach_count += 1;
            "CAPTURED_ONLY"
        } else {
            let outcomes: BTreeSet<String> = transitions
                .iter()
                .map(|transition| transition.outcome.clone())
                .collect();
            if outcomes.len() == 1 {
                match outcomes.iter().next().map(String::as_str) {
                    Some("NOT_CAPTURED") => "NOT_CAPTURED_ONLY",
                    Some("JAMMED") => "JAMMED_ONLY",
                    Some("REBOUND") => "REBOUND_ONLY",
                    Some("SLIP") => "SLIP_ONLY",
                    _ => "MIXED_OUTCOMES",
                }
            } else {
                "MIXED_OUTCOMES"
            }
        };

        approaches.push(ApproachResult {
            approach_state_id: approach_state_id.clone(),
            disposition,
            possible_post_state_ids,
            possible_post_pose_ids,
        });
    }

    let disposition = if any_out_of_domain {
        "ABSTAIN_OUT_OF_DOMAIN_POSSIBILITY"
    } else if any_unknown {
        "ABSTAIN_UNKNOWN_POSSIBILITY"
    } else if capture_only_approach_count == case.initial_approach_states.len() {
        "CAPTURED_SET_MAP_ONLY"
    } else if capture_only_approach_count > 0 {
        "MIXED_CAPTURE_ENVELOPE"
    } else {
        "NO_UNIFORM_CAPTURE"
    };

    let minimum_valid_until_us = case
        .transitions
        .values()
        .flatten()
        .map(|transition| transition.valid_until_us)
        .min()
        .unwrap_or(0);

    Evaluation {
        disposition,
        approaches,
        capture_only_approach_count,
        residual_captured_pose_ids,
        minimum_valid_until_us,
    }
}

fn run(path: &str) -> Result<(), String> {
    let input = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {path}: {error}"))?;
    let case = parse_case(&input).map_err(|errors| errors.join("\n"))?;
    let result = evaluate(&case);

    println!("case_id={}", case.case_id);
    println!("evidence_class={}", case.evidence_class);
    println!("decision_time_us={}", case.decision_time_us);
    println!("approach_set_sha256={}", case.approach_set_sha256);
    println!("receiver_config_sha256={}", case.receiver_config_sha256);
    println!("guide_geometry_sha256={}", case.guide_geometry_sha256);
    println!("frame_sha256={}", case.frame_sha256);
    println!("calibration_sha256={}", case.calibration_sha256);
    println!("disposition={}", result.disposition);
    println!("minimum_valid_until_us={}", result.minimum_valid_until_us);
    println!(
        "capture_only_approach_count={}/{}",
        result.capture_only_approach_count,
        case.initial_approach_states.len()
    );
    println!(
        "residual_captured_pose_ids={}",
        result
            .residual_captured_pose_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(";")
    );
    for approach in &result.approaches {
        println!(
            "approach_state={}|{}|post_states={}|post_poses={}",
            approach.approach_state_id,
            approach.disposition,
            approach
                .possible_post_state_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(";"),
            approach
                .possible_post_pose_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(";")
        );
    }
    println!("pose_uniqueness=NOT_EVALUATED_MECHANICAL_CAPTURE_IS_NOT_A_POSE_SENSOR");
    println!("authority=NONE_SYNTHETIC_EVALUATION_ONLY");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: aero-wpt-mechanical-capture <case.tsv>");
        return ExitCode::from(2);
    }
    match run(&args[1]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ERROR: {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPLETE: &str =
        include_str!("../../../research/aero-wpt-mechanical-capture-v0/complete-capture.tsv");
    const MIXED: &str =
        include_str!("../../../research/aero-wpt-mechanical-capture-v0/mixed-capture.tsv");
    const OUT_OF_DOMAIN: &str =
        include_str!("../../../research/aero-wpt-mechanical-capture-v0/out-of-domain.tsv");

    fn parsed(input: &str) -> Case {
        parse_case(input).unwrap_or_else(|errors| panic!("{}", errors.join("\n")))
    }

    #[test]
    fn complete_capture_preserves_all_residual_pose_possibilities() {
        let result = evaluate(&parsed(COMPLETE));
        assert_eq!(result.disposition, "CAPTURED_SET_MAP_ONLY");
        assert_eq!(result.capture_only_approach_count, 2);
        assert_eq!(result.residual_captured_pose_ids.len(), 3);
        assert!(result.residual_captured_pose_ids.contains("POSE_LEFT_A"));
        assert!(result.residual_captured_pose_ids.contains("POSE_LEFT_B"));
        assert!(result.residual_captured_pose_ids.contains("POSE_RIGHT"));
        assert!(result.approaches.iter().all(|item| item.disposition == "CAPTURED_ONLY"));
    }

    #[test]
    fn mixed_approach_set_is_not_reported_as_complete_capture() {
        let result = evaluate(&parsed(MIXED));
        assert_eq!(result.disposition, "MIXED_CAPTURE_ENVELOPE");
        assert_eq!(result.capture_only_approach_count, 1);
        assert_eq!(result.residual_captured_pose_ids, BTreeSet::from(["POSE_A".to_string()]));
        assert_eq!(result.approaches[0].disposition, "CAPTURED_ONLY");
        assert_eq!(result.approaches[1].disposition, "NOT_CAPTURED_ONLY");
    }

    #[test]
    fn any_out_of_domain_possibility_abstains_for_the_case() {
        let result = evaluate(&parsed(OUT_OF_DOMAIN));
        assert_eq!(result.disposition, "ABSTAIN_OUT_OF_DOMAIN_POSSIBILITY");
        assert_eq!(result.capture_only_approach_count, 1);
    }

    #[test]
    fn unknown_possibility_is_not_silently_discarded() {
        let unknown = MIXED.replace("NOT_CAPTURED", "UNKNOWN");
        let result = evaluate(&parsed(&unknown));
        assert_eq!(result.disposition, "ABSTAIN_UNKNOWN_POSSIBILITY");
        assert_eq!(result.approaches[1].disposition, "UNKNOWN_POSSIBILITY");
    }

    #[test]
    fn jam_and_rebound_remain_distinct_outcomes() {
        let jam = MIXED.replace("NOT_CAPTURED", "JAMMED");
        let rebound = MIXED.replace("NOT_CAPTURED", "REBOUND");
        assert_eq!(evaluate(&parsed(&jam)).approaches[1].disposition, "JAMMED_ONLY");
        assert_eq!(
            evaluate(&parsed(&rebound)).approaches[1].disposition,
            "REBOUND_ONLY"
        );
    }

    #[test]
    fn mixed_capture_and_rebound_preserves_both_possibilities() {
        let extra = COMPLETE
            .lines()
            .nth(1)
            .expect("first captured transition")
            .replace("POST_LEFT_A", "POST_LEFT_REBOUND")
            .replace("POSE_LEFT_A", "POSE_LEFT_REBOUND")
            .replace("\tCAPTURED\ttrue\t30", "\tREBOUND\ttrue\t30");
        let input = format!("{COMPLETE}{extra}\n");
        let result = evaluate(&parsed(&input));
        assert_eq!(result.disposition, "MIXED_CAPTURE_ENVELOPE");
        assert_eq!(result.capture_only_approach_count, 1);
        assert_eq!(result.approaches[0].disposition, "MIXED_OUTCOMES");
        assert_eq!(
            result.residual_captured_pose_ids,
            BTreeSet::from([
                "POSE_LEFT_A".to_string(),
                "POSE_LEFT_B".to_string(),
                "POSE_RIGHT".to_string()
            ])
        );
    }

    #[test]
    fn no_uniform_capture_is_not_confused_with_capture() {
        let slip = MIXED
            .replace("\tNOT_CAPTURED\t", "\tSLIP\t")
            .replace("\tCAPTURED\t", "\tSLIP\t");
        let result = evaluate(&parsed(&slip));
        assert_eq!(result.disposition, "NO_UNIFORM_CAPTURE");
        assert_eq!(result.capture_only_approach_count, 0);
    }

    #[test]
    fn missing_approach_state_relation_is_rejected() {
        let missing = format!(
            "{}\n",
            COMPLETE.lines().take(2).collect::<Vec<_>>().join("\n")
        );
        assert!(parse_case(&missing).is_err());
    }

    #[test]
    fn duplicate_approach_to_post_edge_is_rejected() {
        let duplicate = format!(
            "{}{}",
            COMPLETE,
            COMPLETE.lines().nth(1).expect("first transition")
        );
        assert!(parse_case(&duplicate).is_err());
    }

    #[test]
    fn expired_capture_evidence_invalidates_the_case() {
        let expired = COMPLETE.replace("\t30", "\t4");
        assert!(parse_case(&expired).is_err());
    }

    #[test]
    fn conflicting_post_state_pose_mapping_is_rejected() {
        let conflict = COMPLETE.replace("POST_RIGHT\tPOSE_RIGHT", "POST_LEFT_A\tPOSE_RIGHT");
        assert!(parse_case(&conflict).is_err());
    }

    #[test]
    fn inconsistent_case_metadata_is_rejected() {
        let mismatch = COMPLETE.replacen("APPROACH_RIGHT", "DIFFERENT_APPROACH", 1);
        assert!(parse_case(&mismatch).is_err());
    }

    #[test]
    fn row_permutation_preserves_complete_capture_result() {
        let mut rows: Vec<&str> = COMPLETE.lines().collect();
        let header = rows.remove(0);
        rows.reverse();
        let permuted = format!("{header}\n{}\n", rows.join("\n"));
        assert_eq!(evaluate(&parsed(COMPLETE)), evaluate(&parsed(&permuted)));
    }

    #[test]
    fn out_of_domain_outcome_cannot_claim_in_domain_transition() {
        let malformed = OUT_OF_DOMAIN.replace("CAPTURED\tfalse", "OUT_OF_DOMAIN\ttrue");
        assert!(parse_case(&malformed).is_err());
    }
}
