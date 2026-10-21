// SPDX-License-Identifier: AGPL-3.0-or-later
//! Strict parser for CalculiX reaction-only RR output.
//! This parser never treats RF/external-force output as support reactions.

use crate::{Check, sha256_hex};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const MAX_DECK_BYTES: usize = 16 * 1024 * 1024;
const MAX_DAT_BYTES: usize = 32 * 1024 * 1024;
const RR_HEADER_PREFIX: &str = "reaction forces (rfx,rfy,rfz) for set ";

#[derive(Clone, Debug)]
struct ReactionRow {
    node_id: u64,
    force: [f64; 3],
    source_line: usize,
    raw_line: String,
}

/// Parse one reaction-only RR section from an exact deck and .dat byte pair.
///
/// CCX output does not encode a trustworthy unit system. Numeric components are
/// returned as written; units must be inherited from the exact request/deck contract.
pub fn parse_reaction_only_rr_bytes(
    deck_bytes: &[u8],
    dat_bytes: &[u8],
    expected_nset: &str,
    expected_node_ids: &[u64],
) -> Check<Value> {
    if deck_bytes.is_empty() || deck_bytes.len() > MAX_DECK_BYTES {
        return Err("CalculiX input deck is empty or exceeds 16 MiB".to_owned());
    }
    if dat_bytes.is_empty() || dat_bytes.len() > MAX_DAT_BYTES {
        return Err("CalculiX .dat output is empty or exceeds 32 MiB".to_owned());
    }
    validate_token(expected_nset, "expected node set")?;
    if expected_node_ids.is_empty() {
        return Err("expected support-node inventory must not be empty".to_owned());
    }
    let expected: BTreeSet<u64> = expected_node_ids.iter().copied().collect();
    if expected.len() != expected_node_ids.len() {
        return Err("expected support-node inventory contains duplicate node IDs".to_owned());
    }

    let deck = std::str::from_utf8(deck_bytes)
        .map_err(|_| "CalculiX input deck must be UTF-8/ASCII text".to_owned())?;
    if !deck_declares_rr_for_set(deck, expected_nset)? {
        return Err(format!(
            "exact input deck does not explicitly request RR for node set '{expected_nset}'"
        ));
    }
    let dat = std::str::from_utf8(dat_bytes)
        .map_err(|_| "CalculiX .dat output must be UTF-8/ASCII text".to_owned())?;
    let lines: Vec<&str> = dat.lines().collect();
    let headers: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            line.trim()
                .to_ascii_lowercase()
                .starts_with(RR_HEADER_PREFIX)
                .then_some(index)
        })
        .collect();
    if headers.is_empty() {
        return Err("no reaction-only RR section found; RF/ordinary force sections are not accepted as reactions".to_owned());
    }
    if headers.len() != 1 {
        return Err("multiple RR sections are ambiguous in parser v0.1".to_owned());
    }

    let header_index = headers[0];
    let (observed_nset, time_value, raw_time) = parse_rr_header(lines[header_index])?;
    if !observed_nset.eq_ignore_ascii_case(expected_nset) {
        return Err(format!(
            "RR section node set '{observed_nset}' does not match expected '{expected_nset}'"
        ));
    }

    let mut rows = BTreeMap::<u64, ReactionRow>::new();
    for (index, raw) in lines.iter().enumerate().skip(header_index + 1) {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            if !rows.is_empty() {
                break;
            }
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("forces (")
            || lower.starts_with("reaction forces (")
            || lower.starts_with("internal forces ")
            || lower.starts_with("total forces ")
        {
            break;
        }
        let fields: Vec<&str> = trimmed
            .split(|character: char| character.is_whitespace() || character == ',')
            .filter(|field| !field.is_empty())
            .collect();
        if fields.is_empty() {
            continue;
        }
        let node_id = match fields[0].parse::<u64>() {
            Ok(value) => value,
            Err(_) => {
                let first = fields[0].as_bytes().first().copied().unwrap_or_default();
                if first.is_ascii_digit() || first == b'+' || first == b'-' {
                    return Err(format!(
                        "malformed RR row at .dat line {}: node ID is not an unsigned integer",
                        index + 1
                    ));
                }
                break;
            }
        };
        if fields.len() != 4 {
            return Err(format!(
                "malformed RR row at .dat line {}: expected node ID and exactly three force components",
                index + 1
            ));
        }
        if !expected.contains(&node_id) {
            return Err(format!(
                "RR output contains unexpected node ID {node_id} at .dat line {}",
                index + 1
            ));
        }
        let force = [
            parse_finite(fields[1], index + 1)?,
            parse_finite(fields[2], index + 1)?,
            parse_finite(fields[3], index + 1)?,
        ];
        if rows
            .insert(
                node_id,
                ReactionRow {
                    node_id,
                    force,
                    source_line: index + 1,
                    raw_line: trimmed.to_owned(),
                },
            )
            .is_some()
        {
            return Err(format!("duplicate RR row for node ID {node_id}"));
        }
    }

    let observed: BTreeSet<u64> = rows.keys().copied().collect();
    if observed != expected {
        let missing: Vec<u64> = expected.difference(&observed).copied().collect();
        let extra: Vec<u64> = observed.difference(&expected).copied().collect();
        return Err(format!(
            "RR node inventory mismatch; missing={missing:?}, extra={extra:?}"
        ));
    }

    let nodes: Vec<Value> = rows
        .values()
        .map(|row| {
            json!({
                "node_id": row.node_id,
                "force_components_as_written": row.force,
                "source_line": row.source_line,
                "raw_line": row.raw_line
            })
        })
        .collect();
    Ok(json!({
        "schema_version": "calculix-rr-parsed-reactions-v0.1",
        "parse_status": "PASS",
        "source_semantics": "reaction_only_RR",
        "deck_sha256": sha256_hex(deck_bytes),
        "dat_sha256": sha256_hex(dat_bytes),
        "node_set": observed_nset,
        "time_value_as_written": raw_time,
        "time_value": time_value,
        "expected_node_ids": expected.iter().copied().collect::<Vec<_>>(),
        "nodes": nodes,
        "unit_status": "NOT_ENCODED_IN_CCX_OUTPUT",
        "coordinate_frame_status": "NOT_INFERRED_BY_PARSER",
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "claim_ceiling": "one raw RR section parsed under a bound deck/node-set expectation; no execution authentication, unit inference, equilibrium pass, physical validation or solver qualification"
    }))
}

fn deck_declares_rr_for_set(deck: &str, expected_nset: &str) -> Check<bool> {
    let lines: Vec<&str> = deck.lines().collect();
    let mut matching_cards = 0usize;
    let mut has_rr = false;
    for (index, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("**") || !lower.starts_with("*node print") {
            continue;
        }
        let params: Vec<&str> = line.split(',').skip(1).collect();
        let card_nset = params.iter().find_map(|parameter| {
            let (key, value) = parameter.split_once('=')?;
            key.trim()
                .eq_ignore_ascii_case("nset")
                .then_some(value.trim())
        });
        if !card_nset.is_some_and(|value| value.eq_ignore_ascii_case(expected_nset)) {
            continue;
        }
        matching_cards += 1;
        for variable_line in lines.iter().skip(index + 1) {
            let variable = variable_line.trim();
            if variable.starts_with('*') && !variable.starts_with("**") {
                break;
            }
            if variable.starts_with("**") || variable.is_empty() {
                continue;
            }
            for field in variable
                .split(|character: char| character.is_whitespace() || character == ',')
                .filter(|field| !field.is_empty())
            {
                if field.eq_ignore_ascii_case("RR") {
                    has_rr = true;
                }
            }
        }
    }
    if matching_cards > 1 {
        return Err(format!(
            "input deck contains {matching_cards} *NODE PRINT cards for '{expected_nset}'; expected one unambiguous RR request"
        ));
    }
    Ok(matching_cards == 1 && has_rr)
}

fn parse_rr_header(line: &str) -> Check<(String, f64, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let suffix = lower
        .strip_prefix(RR_HEADER_PREFIX)
        .ok_or_else(|| "RR header does not match supported CalculiX grammar".to_owned())?;
    let (nset, time_text) = suffix
        .split_once(" and time ")
        .ok_or_else(|| "RR header is missing the 'and time' field".to_owned())?;
    let nset = nset.trim();
    if nset.is_empty() {
        return Err("RR header node-set name is empty".to_owned());
    }
    let raw_time = time_text.trim();
    let time = parse_finite(raw_time, 0)?;
    Ok((nset.to_owned(), time, raw_time.to_owned()))
}

fn parse_finite(value: &str, line_number: usize) -> Check<f64> {
    let normalized = value.replace(['d', 'D'], "E");
    let parsed = normalized.parse::<f64>().map_err(|_| {
        if line_number == 0 {
            format!("invalid RR header time value '{value}'")
        } else {
            format!("invalid RR numeric value '{value}' at .dat line {line_number}")
        }
    })?;
    if !parsed.is_finite() {
        return Err(if line_number == 0 {
            "RR header time must be finite".to_owned()
        } else {
            format!("RR numeric value at .dat line {line_number} must be finite")
        });
    }
    Ok(parsed)
}

fn validate_token(value: &str, label: &str) -> Check<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err(format!("{label} is not a valid stable token"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECK_WITH_RR: &str = "*STEP\n*NODE PRINT,NSET=FIXED\nRF, RR\n*END STEP\n";
    const RR_ONLY_OUTPUT: &str = " reaction forces (rfx,rfy,rfz) for set FIXED and time  0.1000000E+01\n  1 -1.000000E+03 0.000000E+00 0.000000E+00\n  2 1.000000E+03 0.000000E+00 0.000000E+00\n";
    const RF_AND_RR_OUTPUT: &str = " forces (fx,fy,fz) for set FIXED and time 0.1000000E+01\n  1 0.000000E+00 0.000000E+00 0.000000E+00\n  2 5.000000E+02 0.000000E+00 0.000000E+00\n\n reaction forces (rfx,rfy,rfz) for set FIXED and time 0.1000000E+01\n  1 -1.000000E+03 0.000000E+00 0.000000E+00\n  2 1.000000E+03 0.000000E+00 0.000000E+00\n";

    #[test]
    fn selects_reaction_only_block_not_ordinary_force_block() {
        let report = parse_reaction_only_rr_bytes(
            DECK_WITH_RR.as_bytes(),
            RF_AND_RR_OUTPUT.as_bytes(),
            "FIXED",
            &[1, 2],
        )
        .unwrap();
        assert_eq!(report["parse_status"], "PASS");
        assert_eq!(report["source_semantics"], "reaction_only_RR");
        assert_eq!(
            report["nodes"][0]["force_components_as_written"][0],
            -1000.0
        );
        assert_eq!(report["nodes"][1]["force_components_as_written"][0], 1000.0);
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(report["real_world_validation_status"], "UNRESOLVED");
    }

    #[test]
    fn RF_only_output_is_not_accepted_as_reaction_data() {
        let rf_only =
            " forces (fx,fy,fz) for set FIXED and time 1.0\n 1 0.0 0.0 0.0\n 2 0.0 0.0 0.0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                rf_only.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
    }

    #[test]
    fn deck_must_explicitly_request_rr() {
        let deck = "*STEP\n*NODE PRINT,NSET=FIXED\nRF\n*END STEP\n";
        assert!(
            parse_reaction_only_rr_bytes(
                deck.as_bytes(),
                RR_ONLY_OUTPUT.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
    }

    #[test]
    fn wrong_set_is_rejected() {
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                RR_ONLY_OUTPUT.as_bytes(),
                "SUPPORTS",
                &[1, 2],
            )
            .is_err()
        );
    }

    #[test]
    fn missing_unexpected_and_duplicate_nodes_are_rejected() {
        let missing = "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0\n1 0 0 0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                missing.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
        let unexpected =
            "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0\n1 0 0 0\n2 0 0 0\n3 0 0 0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                unexpected.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
        let duplicate =
            "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0\n1 0 0 0\n1 1 0 0\n2 0 0 0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                duplicate.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
    }

    #[test]
    fn malformed_nonfinite_and_fortran_exponents_are_handled_explicitly() {
        let fortran = "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0D+00\n1 -1.0D+03 0.0D+00 0.0D+00\n2 1.0D+03 0.0D+00 0.0D+00\n";
        let parsed = parse_reaction_only_rr_bytes(
            DECK_WITH_RR.as_bytes(),
            fortran.as_bytes(),
            "FIXED",
            &[1, 2],
        )
        .unwrap();
        assert_eq!(parsed["time_value"], 1.0);
        let nonfinite =
            "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0\n1 NaN 0 0\n2 1 0 0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                nonfinite.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
        let malformed =
            "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0\n1 1.0E+ 0 0\n2 1 0 0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                malformed.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
    }

    #[test]
    fn duplicate_sections_invalid_set_and_invalid_text_are_rejected() {
        let repeated = format!("{RR_ONLY_OUTPUT}\n{RR_ONLY_OUTPUT}");
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                repeated.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                RR_ONLY_OUTPUT.as_bytes(),
                "bad set!",
                &[1, 2],
            )
            .is_err()
        );
        assert!(
            parse_reaction_only_rr_bytes(DECK_WITH_RR.as_bytes(), &[0xff, 0xfe], "FIXED", &[1, 2],)
                .is_err()
        );
    }
}
