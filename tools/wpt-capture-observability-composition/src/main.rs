// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite composition of a mechanical capture relation and robust observation support.
//! No physical models, sensor estimators, controllers or permits are implemented here.

use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process::ExitCode};

const CAPTURE_HEADER: [&str; 15] = [
    "case_id",
    "evidence_class",
    "decision_time_us",
    "initial_approach_states",
    "capture_map_sha256",
    "receiver_config_sha256",
    "guide_geometry_sha256",
    "frame_sha256",
    "calibration_sha256",
    "approach_state_id",
    "post_state_id",
    "pose_id",
    "outcome",
    "transition_in_domain",
    "valid_until_us",
];
const OBSERVATION_HEADER: [&str; 15] = [
    "case_id",
    "evidence_class",
    "decision_time_us",
    "observation_map_sha256",
    "calibration_set_sha256",
    "model_set_sha256",
    "frame_sha256",
    "observed_signature",
    "post_state_id",
    "pose_id",
    "model_id",
    "model_sha256",
    "fault_id",
    "predicted_signatures",
    "valid_until_us",
];
const MAX_APPROACH_STATES: usize = 64;
const MAX_CAPTURE_EDGES: usize = 4096;
const MAX_OBSERVATION_HYPOTHESES: usize = 8192;

#[derive(Clone, Debug, PartialEq, Eq)]
struct CaptureMeta {
    case_id: String,
    evidence_class: String,
    decision_time_us: u64,
    initial_approach_states: BTreeSet<String>,
    capture_map_sha256: String,
    receiver_config_sha256: String,
    guide_geometry_sha256: String,
    frame_sha256: String,
    calibration_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CaptureTransition {
    post_state_id: String,
    pose_id: String,
    outcome: String,
    in_domain: bool,
    valid_until_us: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CaptureCase {
    meta: CaptureMeta,
    transitions: BTreeMap<String, Vec<CaptureTransition>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ObservationMeta {
    case_id: String,
    evidence_class: String,
    decision_time_us: u64,
    observation_map_sha256: String,
    calibration_set_sha256: String,
    model_set_sha256: String,
    frame_sha256: String,
    observed_signature: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ObservationHypothesis {
    pose_id: String,
    model_id: String,
    model_sha256: String,
    fault_id: String,
    predicted_signatures: BTreeSet<String>,
    valid_until_us: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ObservationCase {
    meta: ObservationMeta,
    hypotheses: BTreeMap<String, Vec<ObservationHypothesis>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ApproachEvaluation {
    approach_state_id: String,
    capture_disposition: &'static str,
    robust_disposition: &'static str,
    possible_post_state_ids: BTreeSet<String>,
    possible_post_pose_ids: BTreeSet<String>,
    surviving_pose_ids: BTreeSet<String>,
    surviving_hypothesis_keys: BTreeSet<String>,
    surviving_model_ids: BTreeSet<String>,
    surviving_fault_ids: BTreeSet<String>,
    surviving_tuple_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evaluation {
    disposition: &'static str,
    approaches: Vec<ApproachEvaluation>,
    capture_only_approach_count: usize,
    total_capture_edge_count: usize,
    observation_map_sha256: String,
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
    if parts.is_empty() || parts.iter().any(|part| !valid_identifier(part)) {
        return Err(format!("{name}: empty, untrimmed or invalid set member"));
    }
    let parsed: BTreeSet<String> = parts.iter().map(|part| (*part).to_string()).collect();
    if parsed.len() != parts.len() {
        return Err(format!("{name}: duplicate set member"));
    }
    Ok(parsed)
}

fn allowed_capture_outcome(value: &str) -> bool {
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

fn parse_capture_case(input: &str) -> Result<CaptureCase, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else {
        return Err(vec!["capture input is empty".to_string()]);
    };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != CAPTURE_HEADER.as_slice() {
        return Err(vec![format!(
            "capture header must contain exactly {} declared columns",
            CAPTURE_HEADER.len()
        )]);
    }

    let mut metadata: Option<Vec<String>> = None;
    let mut case: Option<CaptureCase> = None;
    let mut post_pose_map = BTreeMap::<String, String>::new();
    let mut unique_edges = BTreeSet::<(String, String)>::new();
    let mut errors = Vec::new();
    let mut edge_count = 0usize;

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("capture line {line_no}: blank rows are not allowed"));
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != CAPTURE_HEADER.len() {
            errors.push(format!(
                "capture line {line_no}: expected {} columns, found {}",
                CAPTURE_HEADER.len(),
                fields.len()
            ));
            continue;
        }
        let row_metadata: Vec<String> =
            fields[..9].iter().map(|field| (*field).to_string()).collect();
        if let Some(first) = &metadata {
            if *first != row_metadata {
                errors.push(format!(
                    "capture line {line_no}: case-level metadata differs from first row"
                ));
                continue;
            }
        } else {
            metadata = Some(row_metadata);
            let decision_time_us = match parse_u64(fields[2], "decision_time_us") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(format!("capture line {line_no}: {error}"));
                    continue;
                }
            };
            let initial_approach_states =
                match parse_set(fields[3], "initial_approach_states") {
                    Ok(values) => values,
                    Err(error) => {
                        errors.push(format!("capture line {line_no}: {error}"));
                        continue;
                    }
                };
            if !valid_identifier(fields[0])
                || fields[1] != "SYNTHETIC_KNOWN_TRUTH"
                || initial_approach_states.is_empty()
                || initial_approach_states.len() > MAX_APPROACH_STATES
                || !digest_valid(fields[4])
                || !digest_valid(fields[5])
                || !digest_valid(fields[6])
                || !digest_valid(fields[7])
                || !digest_valid(fields[8])
            {
                errors.push(format!(
                    "capture line {line_no}: invalid case ID, evidence class, approach set or digest"
                ));
                continue;
            }
            case = Some(CaptureCase {
                meta: CaptureMeta {
                    case_id: fields[0].to_string(),
                    evidence_class: fields[1].to_string(),
                    decision_time_us,
                    initial_approach_states,
                    capture_map_sha256: fields[4].to_string(),
                    receiver_config_sha256: fields[5].to_string(),
                    guide_geometry_sha256: fields[6].to_string(),
                    frame_sha256: fields[7].to_string(),
                    calibration_sha256: fields[8].to_string(),
                },
                transitions: BTreeMap::new(),
            });
        }

        let Some(parsed_case) = case.as_mut() else {
            errors.push(format!("capture line {line_no}: case metadata was not initialized"));
            continue;
        };

        let approach = fields[9];
        let post_state = fields[10];
        let pose_id = fields[11];
        let outcome = fields[12];
        if !valid_identifier(approach)
            || !valid_identifier(post_state)
            || !valid_identifier(pose_id)
            || !allowed_capture_outcome(outcome)
        {
            errors.push(format!(
                "capture line {line_no}: invalid approach/post-state/pose or outcome"
            ));
            continue;
        }
        if !parsed_case.meta.initial_approach_states.contains(approach) {
            errors.push(format!(
                "capture line {line_no}: approach {approach} is not in initial_approach_states"
            ));
            continue;
        }
        let in_domain = match fields[13] {
            "true" => true,
            "false" => false,
            other => {
                errors.push(format!(
                    "capture line {line_no}: transition_in_domain must be true or false, found {other:?}"
                ));
                continue;
            }
        };
        if outcome == "OUT_OF_DOMAIN" && in_domain {
            errors.push(format!(
                "capture line {line_no}: OUT_OF_DOMAIN contradicts transition_in_domain=true"
            ));
            continue;
        }
        let valid_until_us = match parse_u64(fields[14], "valid_until_us") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("capture line {line_no}: {error}"));
                continue;
            }
        };
        if valid_until_us < parsed_case.meta.decision_time_us {
            errors.push(format!(
                "capture line {line_no}: capture evidence expired before decision time"
            ));
            continue;
        }
        if let Some(existing_pose) = post_pose_map.get(post_state) {
            if existing_pose != pose_id {
                errors.push(format!(
                    "capture line {line_no}: post-state {post_state} maps to conflicting pose IDs"
                ));
                continue;
            }
        } else {
            post_pose_map.insert(post_state.to_string(), pose_id.to_string());
        }
        let edge_key = (approach.to_string(), post_state.to_string());
        if !unique_edges.insert(edge_key) {
            errors.push(format!(
                "capture line {line_no}: duplicate approach/post-state edge"
            ));
            continue;
        }

        parsed_case
            .transitions
            .entry(approach.to_string())
            .or_default()
            .push(CaptureTransition {
                post_state_id: post_state.to_string(),
                pose_id: pose_id.to_string(),
                outcome: outcome.to_string(),
                in_domain,
                valid_until_us,
            });
        edge_count += 1;
        if edge_count > MAX_CAPTURE_EDGES {
            errors.push(format!("capture case exceeds {MAX_CAPTURE_EDGES} edges"));
            break;
        }
    }

    let Some(mut parsed_case) = case else {
        return Err(if errors.is_empty() {
            vec!["capture file has no transition rows".to_string()]
        } else {
            errors
        });
    };
    for approach in &parsed_case.meta.initial_approach_states {
        if !parsed_case.transitions.contains_key(approach) {
            errors.push(format!(
                "capture approach state {approach} has no possible transition"
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

fn parse_observation_case(input: &str) -> Result<ObservationCase, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else {
        return Err(vec!["observation input is empty".to_string()]);
    };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != OBSERVATION_HEADER.as_slice() {
        return Err(vec![format!(
            "observation header must contain exactly {} declared columns",
            OBSERVATION_HEADER.len()
        )]);
    }

    let mut metadata: Option<Vec<String>> = None;
    let mut case: Option<ObservationCase> = None;
    let mut model_digests = BTreeMap::<String, String>::new();
    let mut unique_tuples = BTreeSet::<(String, String, String)>::new();
    let mut errors = Vec::new();
    let mut hypothesis_count = 0usize;

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("observation line {line_no}: blank rows are not allowed"));
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != OBSERVATION_HEADER.len() {
            errors.push(format!(
                "observation line {line_no}: expected {} columns, found {}",
                OBSERVATION_HEADER.len(),
                fields.len()
            ));
            continue;
        }
        let row_metadata: Vec<String> =
            fields[..8].iter().map(|field| (*field).to_string()).collect();
        if let Some(first) = &metadata {
            if *first != row_metadata {
                errors.push(format!(
                    "observation line {line_no}: case-level metadata differs from first row"
                ));
                continue;
            }
        } else {
            metadata = Some(row_metadata);
            let decision_time_us = match parse_u64(fields[2], "decision_time_us") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(format!("observation line {line_no}: {error}"));
                    continue;
                }
            };
            if !valid_identifier(fields[0])
                || fields[1] != "SYNTHETIC_KNOWN_TRUTH"
                || !valid_identifier(fields[7])
                || !digest_valid(fields[3])
                || !digest_valid(fields[4])
                || !digest_valid(fields[5])
                || !digest_valid(fields[6])
            {
                errors.push(format!(
                    "observation line {line_no}: invalid case ID, evidence class, signature or digest"
                ));
                continue;
            }
            case = Some(ObservationCase {
                meta: ObservationMeta {
                    case_id: fields[0].to_string(),
                    evidence_class: fields[1].to_string(),
                    decision_time_us,
                    observation_map_sha256: fields[3].to_string(),
                    calibration_set_sha256: fields[4].to_string(),
                    model_set_sha256: fields[5].to_string(),
                    frame_sha256: fields[6].to_string(),
                    observed_signature: fields[7].to_string(),
                },
                hypotheses: BTreeMap::new(),
            });
        }

        let Some(parsed_case) = case.as_mut() else {
            errors.push(format!(
                "observation line {line_no}: case metadata was not initialized"
            ));
            continue;
        };
        let post_state = fields[8];
        let pose_id = fields[9];
        let model_id = fields[10];
        let model_sha256 = fields[11];
        let fault_id = fields[12];
        if !valid_identifier(post_state)
            || !valid_identifier(pose_id)
            || !valid_identifier(model_id)
            || !digest_valid(model_sha256)
            || !valid_identifier(fault_id)
        {
            errors.push(format!(
                "observation line {line_no}: invalid post-state/pose/model/fault ID or model digest"
            ));
            continue;
        }
        let predicted_signatures = match parse_set(fields[13], "predicted_signatures") {
            Ok(values) => values,
            Err(error) => {
                errors.push(format!("observation line {line_no}: {error}"));
                continue;
            }
        };
        let valid_until_us = match parse_u64(fields[14], "valid_until_us") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("observation line {line_no}: {error}"));
                continue;
            }
        };
        if valid_until_us < parsed_case.meta.decision_time_us {
            errors.push(format!(
                "observation line {line_no}: observation hypothesis expired before decision time"
            ));
            continue;
        }
        if let Some(existing) = model_digests.get(model_id) {
            if existing != model_sha256 {
                errors.push(format!(
                    "observation line {line_no}: model {model_id} has conflicting content digests"
                ));
                continue;
            }
        } else {
            model_digests.insert(model_id.to_string(), model_sha256.to_string());
        }
        let key = (
            post_state.to_string(),
            model_id.to_string(),
            fault_id.to_string(),
        );
        if !unique_tuples.insert(key) {
            errors.push(format!(
                "observation line {line_no}: duplicate post-state/model/fault tuple"
            ));
            continue;
        }

        parsed_case
            .hypotheses
            .entry(post_state.to_string())
            .or_default()
            .push(ObservationHypothesis {
                pose_id: pose_id.to_string(),
                model_id: model_id.to_string(),
                model_sha256: model_sha256.to_string(),
                fault_id: fault_id.to_string(),
                predicted_signatures,
                valid_until_us,
            });
        hypothesis_count += 1;
        if hypothesis_count > MAX_OBSERVATION_HYPOTHESES {
            errors.push(format!(
                "observation case exceeds {MAX_OBSERVATION_HYPOTHESES} tuples"
            ));
            break;
        }
    }

    let Some(mut parsed_case) = case else {
        return Err(if errors.is_empty() {
            vec!["observation file has no hypothesis rows".to_string()]
        } else {
            errors
        });
    };
    if parsed_case.hypotheses.is_empty() {
        errors.push("observation file has no hypotheses".to_string());
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    for hypotheses in parsed_case.hypotheses.values_mut() {
        hypotheses.sort();
    }
    Ok(parsed_case)
}

fn validate_join(capture: &CaptureCase, observation: &ObservationCase) -> Result<(), Vec<String>> {
    let c = &capture.meta;
    let o = &observation.meta;
    let mut errors = Vec::new();
    if c.case_id != o.case_id {
        errors.push("capture and observation case IDs differ".to_string());
    }
    if c.evidence_class != o.evidence_class {
        errors.push("capture and observation evidence classes differ".to_string());
    }
    if c.decision_time_us != o.decision_time_us {
        errors.push("capture and observation decision times differ".to_string());
    }
    if c.frame_sha256 != o.frame_sha256 {
        errors.push("capture and observation frame digests differ".to_string());
    }
    if c.calibration_sha256 != o.calibration_set_sha256 {
        errors.push("capture calibration digest differs from observation calibration-set digest".to_string());
    }

    let mut all_capture_states = BTreeMap::<String, String>::new();
    for transitions in capture.transitions.values() {
        for transition in transitions {
            if let Some(existing) = all_capture_states.get(&transition.post_state_id) {
                if existing != &transition.pose_id {
                    errors.push(format!(
                        "capture post-state {} has conflicting pose mappings",
                        transition.post_state_id
                    ));
                }
            } else {
                all_capture_states.insert(
                    transition.post_state_id.clone(),
                    transition.pose_id.clone(),
                );
            }
        }
    }

    for (post_state, hypotheses) in &observation.hypotheses {
        let Some(expected_pose) = all_capture_states.get(post_state) else {
            errors.push(format!(
                "observation support references post-state {post_state} absent from capture map"
            ));
            continue;
        };
        if hypotheses.iter().any(|hypothesis| &hypothesis.pose_id != expected_pose) {
            errors.push(format!(
                "observation post-state {post_state} pose mapping conflicts with capture map"
            ));
        }
    }

    for transitions in capture.transitions.values() {
        for transition in transitions {
            if transition.in_domain && transition.outcome == "CAPTURED"
                && !observation.hypotheses.contains_key(&transition.post_state_id)
            {
                errors.push(format!(
                    "captured post-state {} has no observation support",
                    transition.post_state_id
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn hypothesis_key(post_state: &str, hypothesis: &ObservationHypothesis) -> String {
    format!(
        "{post_state}|{}|{}|{}",
        hypothesis.model_id, hypothesis.fault_id, hypothesis.model_sha256
    )
}

fn evaluate(capture: &CaptureCase, observation: &ObservationCase) -> Evaluation {
    let mut approaches = Vec::new();
    let mut capture_only_approach_count = 0usize;
    let mut saw_out_of_domain = false;
    let mut saw_unknown = false;
    let mut saw_capture_not_guaranteed = false;
    let mut saw_no_support = false;
    let mut saw_ambiguous = false;

    for approach_state_id in &capture.meta.initial_approach_states {
        let transitions = capture
            .transitions
            .get(approach_state_id)
            .expect("parser establishes capture transition coverage");
        let possible_post_state_ids: BTreeSet<String> = transitions
            .iter()
            .map(|transition| transition.post_state_id.clone())
            .collect();
        let possible_post_pose_ids: BTreeSet<String> = transitions
            .iter()
            .map(|transition| transition.pose_id.clone())
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

        let capture_disposition = if has_out_of_domain {
            saw_out_of_domain = true;
            "OUT_OF_DOMAIN_POSSIBILITY"
        } else if has_unknown {
            saw_unknown = true;
            "UNKNOWN_POSSIBILITY"
        } else if all_captured {
            capture_only_approach_count += 1;
            "CAPTURED_ONLY"
        } else {
            saw_capture_not_guaranteed = true;
            "CAPTURE_NOT_GUARANTEED"
        };

        let mut surviving_pose_ids = BTreeSet::new();
        let mut surviving_hypothesis_keys = BTreeSet::new();
        let mut surviving_model_ids = BTreeSet::new();
        let mut surviving_fault_ids = BTreeSet::new();
        let mut surviving_tuple_count = 0usize;

        let robust_disposition = if capture_disposition != "CAPTURED_ONLY" {
            "NOT_EVALUATED_CAPTURE_NOT_GUARANTEED"
        } else {
            for transition in transitions {
                let hypotheses = observation
                    .hypotheses
                    .get(&transition.post_state_id)
                    .expect("validated join establishes observation support");
                for hypothesis in hypotheses {
                    if hypothesis
                        .predicted_signatures
                        .contains(&observation.meta.observed_signature)
                    {
                        surviving_pose_ids.insert(hypothesis.pose_id.clone());
                        surviving_hypothesis_keys
                            .insert(hypothesis_key(&transition.post_state_id, hypothesis));
                        surviving_model_ids.insert(hypothesis.model_id.clone());
                        surviving_fault_ids.insert(hypothesis.fault_id.clone());
                        surviving_tuple_count += 1;
                    }
                }
            }
            match surviving_pose_ids.len() {
                0 => {
                    saw_no_support = true;
                    "ABSTAIN_NO_SUPPORT"
                }
                1 => "POSE_UNIQUE_WITHIN_FINITE_HYPOTHESES",
                _ => {
                    saw_ambiguous = true;
                    "AMBIGUOUS_POSE"
                }
            }
        };

        approaches.push(ApproachEvaluation {
            approach_state_id: approach_state_id.clone(),
            capture_disposition,
            robust_disposition,
            possible_post_state_ids,
            possible_post_pose_ids,
            surviving_pose_ids,
            surviving_hypothesis_keys,
            surviving_model_ids,
            surviving_fault_ids,
            surviving_tuple_count,
        });
    }

    let disposition = if saw_out_of_domain {
        "ABSTAIN_OUT_OF_DOMAIN_POSSIBILITY"
    } else if saw_unknown {
        "ABSTAIN_UNKNOWN_POSSIBILITY"
    } else if saw_capture_not_guaranteed {
        "CAPTURE_NOT_GUARANTEED"
    } else if saw_no_support {
        "ABSTAIN_NO_SUPPORT"
    } else if saw_ambiguous {
        "AMBIGUOUS_AFTER_CAPTURE"
    } else if capture_only_approach_count == capture.meta.initial_approach_states.len() {
        "ROBUST_POSE_UNIQUE_AFTER_CAPTURE_WITHIN_FINITE_HYPOTHESES"
    } else {
        "NO_QUALIFIED_DISPOSITION"
    };

    let total_capture_edge_count = capture.transitions.values().map(Vec::len).sum();

    Evaluation {
        disposition,
        approaches,
        capture_only_approach_count,
        total_capture_edge_count,
        observation_map_sha256: observation.meta.observation_map_sha256.clone(),
    }
}

fn run(capture_path: &str, observation_path: &str) -> Result<(), String> {
    let capture_text = fs::read_to_string(capture_path)
        .map_err(|error| format!("cannot read {capture_path}: {error}"))?;
    let observation_text = fs::read_to_string(observation_path)
        .map_err(|error| format!("cannot read {observation_path}: {error}"))?;
    let capture = parse_capture_case(&capture_text).map_err(|errors| errors.join("\n"))?;
    let observation =
        parse_observation_case(&observation_text).map_err(|errors| errors.join("\n"))?;
    validate_join(&capture, &observation).map_err(|errors| errors.join("\n"))?;
    let result = evaluate(&capture, &observation);

    println!("case_id={}", capture.meta.case_id);
    println!("evidence_class={}", capture.meta.evidence_class);
    println!("decision_time_us={}", capture.meta.decision_time_us);
    println!("capture_map_sha256={}", capture.meta.capture_map_sha256);
    println!("observation_map_sha256={}", result.observation_map_sha256);
    println!("disposition={}", result.disposition);
    println!(
        "capture_only_approach_count={}/{}",
        result.capture_only_approach_count,
        capture.meta.initial_approach_states.len()
    );
    println!("capture_edge_count={}", result.total_capture_edge_count);
    for approach in &result.approaches {
        println!(
            "approach={}|capture={}|robust_pose={}|post_states={}|pre_observation_pose_count={}|surviving_poses={}|surviving_tuples={}|surviving_models={}|surviving_faults={}",
            approach.approach_state_id,
            approach.capture_disposition,
            approach.robust_disposition,
            approach.possible_post_state_ids.iter().cloned().collect::<Vec<_>>().join(";"),
            approach.possible_post_pose_ids.len(),
            approach.surviving_pose_ids.iter().cloned().collect::<Vec<_>>().join(";"),
            approach.surviving_tuple_count,
            approach.surviving_model_ids.len(),
            approach.surviving_fault_ids.len()
        );
    }
    println!("operational_permit=false");
    println!("authority=NONE_SYNTHETIC_COMPOSITION_ONLY");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: wpt-capture-observability-composition <capture.tsv> <observation.tsv>"
        );
        return ExitCode::from(2);
    }
    match run(&args[1], &args[2]) {
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

    const FLAT_CAPTURE: &str =
        include_str!("../../../research/wpt-capture-observability-composition-v0/flat-pad-capture.tsv");
    const FLAT_OBSERVATIONS: &str =
        include_str!("../../../research/wpt-capture-observability-composition-v0/flat-pad-observations.tsv");
    const GUIDE_CAPTURE: &str =
        include_str!("../../../research/wpt-capture-observability-composition-v0/v-guide-capture.tsv");
    const GUIDE_OBSERVATIONS: &str =
        include_str!("../../../research/wpt-capture-observability-composition-v0/v-guide-observations.tsv");
    const GUIDE_PROBE_OBSERVATIONS: &str =
        include_str!("../../../research/wpt-capture-observability-composition-v0/v-guide-probe-observations.tsv");
    const REBOUND_CAPTURE: &str =
        include_str!("../../../research/wpt-capture-observability-composition-v0/rebound-capture.tsv");
    const REBOUND_OBSERVATIONS: &str =
        include_str!("../../../research/wpt-capture-observability-composition-v0/rebound-observations.tsv");

    fn composed(capture_text: &str, observation_text: &str) -> (CaptureCase, ObservationCase, Evaluation) {
        let capture = parse_capture_case(capture_text)
            .unwrap_or_else(|errors| panic!("capture: {}", errors.join("\n")));
        let observation = parse_observation_case(observation_text)
            .unwrap_or_else(|errors| panic!("observation: {}", errors.join("\n")));
        validate_join(&capture, &observation)
            .unwrap_or_else(|errors| panic!("join: {}", errors.join("\n")));
        let result = evaluate(&capture, &observation);
        (capture, observation, result)
    }

    #[test]
    fn flat_pad_remains_ambiguous_after_capture() {
        let (_, _, result) = composed(FLAT_CAPTURE, FLAT_OBSERVATIONS);
        assert_eq!(result.disposition, "AMBIGUOUS_AFTER_CAPTURE");
        assert_eq!(result.approaches[0].possible_post_state_ids.len(), 3);
        assert_eq!(
            result.approaches[0].surviving_pose_ids,
            BTreeSet::from(["POSE_A".to_string(), "POSE_B".to_string()])
        );
    }

    #[test]
    fn passive_guide_reduces_state_set_but_does_not_resolve_the_alias() {
        let (flat_capture, _, flat_result) = composed(FLAT_CAPTURE, FLAT_OBSERVATIONS);
        let (_, _, guide_result) = composed(GUIDE_CAPTURE, GUIDE_OBSERVATIONS);
        assert_eq!(guide_result.disposition, "AMBIGUOUS_AFTER_CAPTURE");
        assert_eq!(flat_capture.meta.initial_approach_states.len(), 1);
        assert_eq!(flat_result.approaches[0].possible_post_state_ids.len(), 3);
        assert_eq!(guide_result.approaches[0].possible_post_state_ids.len(), 2);
        assert_eq!(
            guide_result.approaches[0].surviving_pose_ids,
            BTreeSet::from(["POSE_A".to_string(), "POSE_B".to_string()])
        );
    }

    #[test]
    fn guide_plus_qualified_joint_signature_resolves_one_pose_across_fault_models() {
        let (_, _, result) = composed(GUIDE_CAPTURE, GUIDE_PROBE_OBSERVATIONS);
        assert_eq!(
            result.disposition,
            "ROBUST_POSE_UNIQUE_AFTER_CAPTURE_WITHIN_FINITE_HYPOTHESES"
        );
        let approach = &result.approaches[0];
        assert_eq!(approach.robust_disposition, "POSE_UNIQUE_WITHIN_FINITE_HYPOTHESES");
        assert_eq!(approach.surviving_pose_ids, BTreeSet::from(["POSE_A".to_string()]));
        assert_eq!(approach.surviving_model_ids.len(), 2);
        assert_eq!(approach.surviving_fault_ids.len(), 2);
        assert_eq!(approach.surviving_tuple_count, 2);
    }

    #[test]
    fn a_possible_rebound_prevents_capture_and_pose_promotion() {
        let (_, _, result) = composed(REBOUND_CAPTURE, REBOUND_OBSERVATIONS);
        assert_eq!(result.disposition, "CAPTURE_NOT_GUARANTEED");
        assert_eq!(
            result.approaches[0].capture_disposition,
            "CAPTURE_NOT_GUARANTEED"
        );
        assert_eq!(
            result.approaches[0].robust_disposition,
            "NOT_EVALUATED_CAPTURE_NOT_GUARANTEED"
        );
    }

    #[test]
    fn missing_support_for_a_possible_captured_state_is_rejected() {
        let incomplete = GUIDE_OBSERVATIONS
            .lines()
            .filter(|line| !line.contains("\tPOST_B\t"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        let capture = parse_capture_case(GUIDE_CAPTURE).expect("valid capture map");
        let observation = parse_observation_case(&incomplete).expect("remaining rows parse");
        assert!(validate_join(&capture, &observation).is_err());
    }

    #[test]
    fn uncertainty_expansion_can_turn_unique_pose_into_ambiguity_but_never_remove_survivors() {
        let (_, _, base_result) = composed(GUIDE_CAPTURE, GUIDE_PROBE_OBSERVATIONS);
        let extra = GUIDE_PROBE_OBSERVATIONS
            .lines()
            .nth(3)
            .expect("POST_B drift row")
            .replace("\tOBS_B\t", "\tOBS_COMPOSITE_01\t")
            .replace("\tGAIN_DRIFT\t", "\tGAIN_DRIFT_EXTRA\t")
            .replace(
                "\t4444444444444444444444444444444444444444444444444444444444444444\t",
                "\t8888888888888888888888888888888888888888888888888888888888888888\t",
            );
        let expanded_text = format!("{GUIDE_PROBE_OBSERVATIONS}{extra}\n");
        let (_, _, expanded_result) = composed(GUIDE_CAPTURE, &expanded_text);
        let base = &base_result.approaches[0].surviving_hypothesis_keys;
        let expanded = &expanded_result.approaches[0].surviving_hypothesis_keys;
        assert!(base.is_subset(expanded));
        assert_eq!(base_result.disposition, "ROBUST_POSE_UNIQUE_AFTER_CAPTURE_WITHIN_FINITE_HYPOTHESES");
        assert_eq!(expanded_result.disposition, "AMBIGUOUS_AFTER_CAPTURE");
    }

    #[test]
    fn expired_observation_hypothesis_invalidates_the_case() {
        let expired = GUIDE_OBSERVATIONS.replace("\t100", "\t4");
        assert!(parse_observation_case(&expired).is_err());
    }

    #[test]
    fn model_id_cannot_reference_two_content_digests() {
        let conflicting = GUIDE_OBSERVATIONS.replace(
            "POST_A\tPOSE_A\tNOMINAL\t2222222222222222222222222222222222222222222222222222222222222222",
            "POST_A\tPOSE_A\tNOMINAL\t9999999999999999999999999999999999999999999999999999999999999999",
        );
        assert!(parse_observation_case(&conflicting).is_err());
    }

    #[test]
    fn inconsistent_frame_and_calibration_revisions_are_rejected() {
        let bad_frame = GUIDE_OBSERVATIONS.replace(
            "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let capture = parse_capture_case(GUIDE_CAPTURE).expect("capture parses");
        let observation = parse_observation_case(&bad_frame).expect("observation parses");
        assert!(validate_join(&capture, &observation).is_err());
    }

    #[test]
    fn row_permutation_preserves_flat_pad_result() {
        let mut capture_lines: Vec<&str> = FLAT_CAPTURE.lines().collect();
        let capture_header = capture_lines.remove(0);
        capture_lines.reverse();
        let capture_permuted = format!("{capture_header}\n{}\n", capture_lines.join("\n"));

        let mut observation_lines: Vec<&str> = FLAT_OBSERVATIONS.lines().collect();
        let observation_header = observation_lines.remove(0);
        observation_lines.reverse();
        let observation_permuted =
            format!("{observation_header}\n{}\n", observation_lines.join("\n"));

        let (_, _, original) = composed(FLAT_CAPTURE, FLAT_OBSERVATIONS);
        let (_, _, reordered) = composed(&capture_permuted, &observation_permuted);
        assert_eq!(original, reordered);
    }

    #[test]
    fn unsupported_observation_abstains_without_inventing_a_pose() {
        let unsupported = GUIDE_PROBE_OBSERVATIONS.replace(
            "\tOBS_COMPOSITE_01\tPOST_",
            "\tUNSEEN_SIGNATURE\tPOST_",
        );
        let (_, _, result) = composed(GUIDE_CAPTURE, &unsupported);
        assert_eq!(result.disposition, "ABSTAIN_NO_SUPPORT");
        assert!(result.approaches[0].surviving_pose_ids.is_empty());
    }

    #[test]
    fn capture_calibration_digest_must_match_observation_calibration_set() {
        let bad_capture = GUIDE_CAPTURE.replace(
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let capture = parse_capture_case(&bad_capture).expect("capture parses");
        let observation = parse_observation_case(GUIDE_OBSERVATIONS).expect("observations parse");
        assert!(validate_join(&capture, &observation).is_err());
    }
}
