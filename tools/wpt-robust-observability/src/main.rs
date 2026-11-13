// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact finite-set robust pose-identifiability evaluator.
//! This is not a sensor estimator, fault detector, physical model, or permit issuer.

use std::collections::BTreeSet;
use std::{env, fs, process::ExitCode};

const HEADER: [&str; 13] = [
    "case_id",
    "evidence_class",
    "observed_class",
    "decision_time_us",
    "frame_sha256",
    "calibration_set_sha256",
    "model_set_sha256",
    "pose_id",
    "model_id",
    "model_sha256",
    "fault_id",
    "predicted_outcomes",
    "valid_until_us",
];
const MAX_HYPOTHESES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Hypothesis {
    pose_id: String,
    model_id: String,
    model_sha256: String,
    fault_id: String,
    predicted_outcomes: BTreeSet<String>,
    valid_until_us: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Case {
    case_id: String,
    evidence_class: String,
    observed_class: String,
    decision_time_us: u64,
    frame_sha256: String,
    calibration_set_sha256: String,
    model_set_sha256: String,
    hypotheses: Vec<Hypothesis>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evaluation {
    disposition: &'static str,
    surviving_pose_ids: BTreeSet<String>,
    surviving_model_ids: BTreeSet<String>,
    surviving_fault_ids: BTreeSet<String>,
    surviving_tuple_count: usize,
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

fn parse_set(value: &str, name: &str) -> Result<BTreeSet<String>, String> {
    let parts: Vec<&str> = value.split(';').collect();
    if parts.is_empty()
        || parts
            .iter()
            .any(|part| part.is_empty() || part.trim() != *part)
    {
        return Err(format!("{name}: empty or untrimmed set member"));
    }
    let parsed: BTreeSet<String> = parts.iter().map(|part| (*part).to_string()).collect();
    if parsed.len() != parts.len() {
        return Err(format!("{name}: duplicate set member"));
    }
    Ok(parsed)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !value.chars().any(char::is_control)
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
    let mut unique_tuples = BTreeSet::new();
    let mut model_digests = std::collections::BTreeMap::<String, String>::new();
    let mut errors = Vec::new();

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
            fields[..7].iter().map(|field| (*field).to_string()).collect();
        if let Some(first) = &metadata {
            if *first != row_metadata {
                errors.push(format!(
                    "line {line_no}: case-level observation, timing, frame or set digest differs"
                ));
                continue;
            }
        } else {
            metadata = Some(row_metadata);
            let decision_time_us = match parse_u64(fields[3], "decision_time_us") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(format!("line {line_no}: {error}"));
                    continue;
                }
            };
            if !valid_identifier(fields[0])
                || fields[1] != "SYNTHETIC_KNOWN_TRUTH"
                || !valid_identifier(fields[2])
                || !digest_valid(fields[4])
                || !digest_valid(fields[5])
                || !digest_valid(fields[6])
            {
                errors.push(format!(
                    "line {line_no}: invalid case ID, evidence class, observed class or digest"
                ));
                continue;
            }
            case = Some(Case {
                case_id: fields[0].to_string(),
                evidence_class: fields[1].to_string(),
                observed_class: fields[2].to_string(),
                decision_time_us,
                frame_sha256: fields[4].to_string(),
                calibration_set_sha256: fields[5].to_string(),
                model_set_sha256: fields[6].to_string(),
                hypotheses: Vec::new(),
            });
        }

        let Some(parsed_case) = case.as_mut() else {
            errors.push(format!("line {line_no}: case metadata could not be initialized"));
            continue;
        };

        let pose_id = fields[7];
        let model_id = fields[8];
        let model_sha256 = fields[9];
        let fault_id = fields[10];
        if !valid_identifier(pose_id)
            || !valid_identifier(model_id)
            || !digest_valid(model_sha256)
            || !valid_identifier(fault_id)
        {
            errors.push(format!(
                "line {line_no}: invalid pose/model/fault ID or model digest"
            ));
            continue;
        }

        let predicted_outcomes = match parse_set(fields[11], "predicted_outcomes") {
            Ok(values) => values,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        let valid_until_us = match parse_u64(fields[12], "valid_until_us") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        if valid_until_us < parsed_case.decision_time_us {
            errors.push(format!(
                "line {line_no}: admissible hypothesis evidence expired before decision time"
            ));
            continue;
        }

        if let Some(existing_digest) = model_digests.get(model_id) {
            if existing_digest != model_sha256 {
                errors.push(format!(
                    "line {line_no}: model_id {model_id} maps to conflicting model digests"
                ));
                continue;
            }
        } else {
            model_digests.insert(model_id.to_string(), model_sha256.to_string());
        }

        let tuple_key = (
            pose_id.to_string(),
            model_id.to_string(),
            fault_id.to_string(),
        );
        if !unique_tuples.insert(tuple_key) {
            errors.push(format!(
                "line {line_no}: duplicate (pose_id, model_id, fault_id) hypothesis tuple"
            ));
            continue;
        }

        parsed_case.hypotheses.push(Hypothesis {
            pose_id: pose_id.to_string(),
            model_id: model_id.to_string(),
            model_sha256: model_sha256.to_string(),
            fault_id: fault_id.to_string(),
            predicted_outcomes,
            valid_until_us,
        });
        if parsed_case.hypotheses.len() > MAX_HYPOTHESES {
            errors.push(format!("case exceeds {MAX_HYPOTHESES} hypothesis tuples"));
            break;
        }
    }

    let Some(mut parsed_case) = case else {
        return Err(if errors.is_empty() {
            vec!["no hypothesis rows found".to_string()]
        } else {
            errors
        });
    };
    if parsed_case.hypotheses.is_empty() {
        errors.push("case has no admissible hypothesis tuples".to_string());
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    parsed_case.hypotheses.sort();
    Ok(parsed_case)
}

fn evaluate(case: &Case) -> Evaluation {
    let survivors: Vec<&Hypothesis> = case
        .hypotheses
        .iter()
        .filter(|hypothesis| hypothesis.predicted_outcomes.contains(&case.observed_class))
        .collect();

    let surviving_pose_ids: BTreeSet<String> = survivors
        .iter()
        .map(|hypothesis| hypothesis.pose_id.clone())
        .collect();
    let surviving_model_ids: BTreeSet<String> = survivors
        .iter()
        .map(|hypothesis| hypothesis.model_id.clone())
        .collect();
    let surviving_fault_ids: BTreeSet<String> = survivors
        .iter()
        .map(|hypothesis| hypothesis.fault_id.clone())
        .collect();

    let disposition = match surviving_pose_ids.len() {
        0 => "ABSTAIN_NO_MODEL_SUPPORT",
        1 => "POSE_UNIQUE_WITHIN_DECLARED_FINITE_HYPOTHESES",
        _ => "AMBIGUOUS_POSE",
    };

    Evaluation {
        disposition,
        surviving_pose_ids,
        surviving_model_ids,
        surviving_fault_ids,
        surviving_tuple_count: survivors.len(),
    }
}

fn run(path: &str) -> Result<(), String> {
    let input = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {path}: {error}"))?;
    let case = parse_case(&input).map_err(|errors| errors.join("\n"))?;
    let result = evaluate(&case);

    println!("case_id={}", case.case_id);
    println!("evidence_class={}", case.evidence_class);
    println!("observed_class={}", case.observed_class);
    println!("decision_time_us={}", case.decision_time_us);
    println!("frame_sha256={}", case.frame_sha256);
    println!("calibration_set_sha256={}", case.calibration_set_sha256);
    println!("model_set_sha256={}", case.model_set_sha256);
    println!("disposition={}", result.disposition);
    println!("surviving_tuple_count={}", result.surviving_tuple_count);
    println!("surviving_pose_ids={}", result.surviving_pose_ids.iter().cloned().collect::<Vec<_>>().join(";"));
    println!("surviving_model_count={}", result.surviving_model_ids.len());
    println!("surviving_fault_count={}", result.surviving_fault_ids.len());
    println!("authority=NONE_SYNTHETIC_EVALUATION_ONLY");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: wpt-robust-observability <case.tsv>");
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

    const NOMINAL: &str =
        include_str!("../../../research/wpt-robust-observability-v0/nominal-unique.tsv");
    const DRIFT: &str =
        include_str!("../../../research/wpt-robust-observability-v0/model-drift-ambiguous.tsv");
    const DROPOUT: &str =
        include_str!("../../../research/wpt-robust-observability-v0/sensor-dropout-ambiguous.tsv");

    fn parsed(input: &str) -> Case {
        parse_case(input).unwrap_or_else(|errors| panic!("{}", errors.join("\n")))
    }

    fn survivor_tuple_keys(case: &Case) -> BTreeSet<(String, String, String)> {
        case.hypotheses
            .iter()
            .filter(|hypothesis| hypothesis.predicted_outcomes.contains(&case.observed_class))
            .map(|hypothesis| (
                hypothesis.pose_id.clone(),
                hypothesis.model_id.clone(),
                hypothesis.fault_id.clone(),
            ))
            .collect()
    }

    #[test]
    fn nominal_case_has_one_pose() {
        let result = evaluate(&parsed(NOMINAL));
        assert_eq!(result.disposition, "POSE_UNIQUE_WITHIN_DECLARED_FINITE_HYPOTHESES");
        assert_eq!(result.surviving_pose_ids, BTreeSet::from(["POSE_A".to_string()]));
        assert_eq!(result.surviving_tuple_count, 1);
    }

    #[test]
    fn admissible_model_drift_preserves_global_pose_ambiguity() {
        let case = parsed(DRIFT);
        let result = evaluate(&case);
        assert_eq!(result.disposition, "AMBIGUOUS_POSE");
        assert_eq!(
            result.surviving_pose_ids,
            BTreeSet::from(["POSE_A".to_string(), "POSE_B".to_string()])
        );
        assert_eq!(result.surviving_model_ids.len(), 2);
    }

    #[test]
    fn sensor_dropout_fault_cannot_be_mistaken_for_unique_pose() {
        let result = evaluate(&parsed(DROPOUT));
        assert_eq!(result.disposition, "AMBIGUOUS_POSE");
        assert_eq!(result.surviving_pose_ids.len(), 2);
        assert!(result.surviving_fault_ids.contains("CHANNEL_DROPOUT"));
    }

    #[test]
    fn expanding_admissible_hypotheses_never_shrinks_survivors() {
        let full = parsed(DRIFT);
        let nominal_only_input = format!(
            "{}\n",
            DRIFT.lines().take(2).collect::<Vec<_>>().join("\n")
        );
        let nominal_only = parsed(&nominal_only_input);
        let before = survivor_tuple_keys(&nominal_only);
        let after = survivor_tuple_keys(&full);
        assert!(before.is_subset(&after));
        let before_poses: BTreeSet<String> = before.iter().map(|key| key.0.clone()).collect();
        let after_poses: BTreeSet<String> = after.iter().map(|key| key.0.clone()).collect();
        assert!(before_poses.is_subset(&after_poses));
        assert_eq!(evaluate(&nominal_only).disposition, "POSE_UNIQUE_WITHIN_DECLARED_FINITE_HYPOTHESES");
        assert_eq!(evaluate(&full).disposition, "AMBIGUOUS_POSE");
    }

    #[test]
    fn incompatible_added_hypothesis_does_not_change_survivors() {
        let base = parsed(NOMINAL);
        let extra = NOMINAL
            .lines()
            .nth(1)
            .expect("nominal hypothesis row")
            .replace("POSE_A", "POSE_C")
            .replace("SIG_A;SIG_B", "OTHER");
        let expanded_input = format!("{NOMINAL}{extra}\n");
        let expanded = parsed(&expanded_input);
        assert_eq!(evaluate(&base), evaluate(&expanded));
        assert_eq!(expanded.hypotheses.len(), base.hypotheses.len() + 1);
    }

    #[test]
    fn expired_admissible_hypothesis_invalidates_entire_case() {
        let expired = NOMINAL.replace("\t20", "\t4");
        assert!(parse_case(&expired).is_err());
    }

    #[test]
    fn duplicate_pose_model_fault_tuple_is_rejected() {
        let duplicate = format!("{}{}", NOMINAL, NOMINAL.lines().nth(1).unwrap_or(""));
        assert!(parse_case(&duplicate).is_err());
    }

    #[test]
    fn mismatched_case_metadata_is_rejected() {
        let mismatched = NOMINAL.replacen("SIG_A\t5", "SIG_B\t5", 1);
        assert!(parse_case(&mismatched).is_err());
    }

    #[test]
    fn one_model_id_cannot_alias_different_model_digests() {
        let conflicting = NOMINAL.replace(
            "POSE_B\tNOMINAL\tdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "POSE_B\tNOMINAL\teeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        );
        assert!(parse_case(&conflicting).is_err());
    }

    #[test]
    fn malformed_digest_is_rejected() {
        let malformed = NOMINAL.replacen(&"a".repeat(64), "not-a-digest", 1);
        assert!(parse_case(&malformed).is_err());
    }

    #[test]
    fn row_permutation_preserves_evaluation() {
        let mut rows: Vec<&str> = DRIFT.lines().collect();
        let header = rows.remove(0);
        rows.reverse();
        let permuted = format!("{header}\n{}\n", rows.join("\n"));
        assert_eq!(evaluate(&parsed(DRIFT)), evaluate(&parsed(&permuted)));
    }

    #[test]
    fn unsupported_observation_abstains() {
        let unsupported = NOMINAL.replace("SIG_A\t5", "UNSEEN\t5");
        let result = evaluate(&parsed(&unsupported));
        assert_eq!(result.disposition, "ABSTAIN_NO_MODEL_SUPPORT");
        assert_eq!(result.surviving_tuple_count, 0);
    }
}
