// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Structural integrity checks for the synthetic public-rule golden-case corpus.
//! These checks do not execute the policy predicates or qualify the source rules.

use std::collections::{BTreeMap, BTreeSet};

const CASES_CSV: &str =
    include_str!("../../../../docs/tech/TRANSPORT_PANAMA_POLICY_GOLDEN_CASES_V0_1.csv");
const EXPECTED_FIELD_COUNT: usize = 17;
const EXPECTED_CASE_COUNT: usize = 19;

fn parse_record(line: &str) -> Result<Vec<String>, &'static str> {
    let mut fields = Vec::new();
    let mut value = String::new();
    let mut in_quotes = false;
    let mut after_quote = false;
    let mut characters = line.chars().peekable();

    while let Some(character) = characters.next() {
        if in_quotes {
            if character == '"' {
                if characters.peek().is_some_and(|next| *next == '"') {
                    characters.next();
                    value.push('"');
                } else {
                    in_quotes = false;
                    after_quote = true;
                }
            } else {
                value.push(character);
            }
        } else if after_quote {
            match character {
                ',' => {
                    fields.push(std::mem::take(&mut value));
                    after_quote = false;
                }
                ' ' | '\t' | '\r' => {}
                _ => return Err("unexpected character after a closing quote"),
            }
        } else {
            match character {
                ',' => fields.push(std::mem::take(&mut value)),
                '"' if value.is_empty() => in_quotes = true,
                '"' => return Err("quote inside an unquoted field"),
                _ => value.push(character),
            }
        }
    }

    if in_quotes {
        return Err("unterminated quoted field");
    }
    fields.push(value);
    Ok(fields)
}

fn parse_fixture_rows(csv: &str) -> Result<Vec<Vec<String>>, String> {
    csv.lines()
        .enumerate()
        .filter(|(_, line)| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .map(|(line_number, line)| {
            parse_record(line.trim_end_matches('\r'))
                .map_err(|reason| format!("CSV line {}: {reason}", line_number + 1))
        })
        .collect()
}

fn is_iso_date_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

#[test]
fn golden_case_csv_matches_header_shape_and_provenance_contract() {
    let rows = parse_fixture_rows(CASES_CSV).expect("golden-case CSV should parse");
    assert!(!rows.is_empty(), "CSV must include its header");

    let expected_header = [
        "case_id",
        "rule_id",
        "rule_published_at",
        "simulation_known_at",
        "booking_or_action_date",
        "required_arrival_date",
        "booking_period",
        "lock_system",
        "segment_or_slot_type",
        "prior_slot_same_date",
        "prior_slot_previous_date",
        "period_relation",
        "operation",
        "rule_relevant_facts",
        "expected_specific_rule_result",
        "nonclaim",
        "source_url",
    ];
    assert_eq!(rows[0].len(), EXPECTED_FIELD_COUNT, "header field count changed");
    assert_eq!(
        rows[0].iter().map(String::as_str).collect::<Vec<_>>(),
        expected_header
    );

    let mut case_ids = BTreeSet::new();
    for (row_index, row) in rows.iter().enumerate().skip(1) {
        assert_eq!(
            row.len(),
            EXPECTED_FIELD_COUNT,
            "row {} does not match the header width",
            row_index + 1
        );
        assert!(!row[0].trim().is_empty(), "case ID cannot be empty");
        assert!(case_ids.insert(row[0].clone()), "duplicate case ID {}", row[0]);
        assert!(!row[1].trim().is_empty(), "rule ID is required for {}", row[0]);
        assert!(is_iso_date_shape(&row[2]), "invalid rule publication date for {}", row[0]);
        assert!(is_iso_date_shape(&row[3]), "invalid simulation cutoff for {}", row[0]);
        assert!(!row[13].trim().is_empty(), "rule-relevant facts are required for {}", row[0]);
        assert!(!row[14].trim().is_empty(), "expected rule result is required for {}", row[0]);
        assert!(!row[15].trim().is_empty(), "claim boundary is required for {}", row[0]);
        assert!(
            row[16].starts_with("https://pancanal.com/"),
            "expected an official Canal source URL for {}",
            row[0]
        );
    }

    assert_eq!(
        case_ids.len(),
        EXPECTED_CASE_COUNT,
        "update EXPECTED_CASE_COUNT only after reviewing the changed case corpus"
    );
}

#[test]
fn golden_case_corpus_retains_critical_positive_negative_control_pairs() {
    let rows = parse_fixture_rows(CASES_CSV).expect("golden-case CSV should parse");
    let by_id = rows
        .iter()
        .skip(1)
        .map(|row| (row[0].as_str(), row))
        .collect::<BTreeMap<_, _>>();

    let expected_results = [
        ("POL-003", "rule_not_applicable_apply_Panamax_rules"),
        ("POL-004", "not_blocked_by_consecutive-date-rule"),
        ("POL-005", "blocked_by_same-period_consecutive-date-rule_when_other_conditions_match"),
        ("POL-011", "rule_unavailable_at_cutoff"),
        ("POL-015", "surcharge_threshold_met"),
        ("POL-016", "new_four-day_cancellation_threshold_not_met"),
    ];

    for (case_id, expected_fragment) in expected_results {
        let row = by_id.get(case_id).unwrap_or_else(|| panic!("missing negative control {case_id}"));
        assert!(
            row[14].contains(expected_fragment),
            "{case_id} expected result changed unexpectedly: {}",
            row[14]
        );
    }
}
