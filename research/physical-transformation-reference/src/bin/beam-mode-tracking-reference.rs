// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic mesh-to-mesh mode-tracking receipt using MAC on exact
//! common physical stations and a global maximum-weight assignment. Undefined
//! MAC entries remain null instead of being converted into confident matches.

use physical_transformation_reference::beam_mode_tracking::{
    canonical_beam_mode_tracking_request, run_beam_mode_tracking,
};
use physical_transformation_reference::beam_pin_assembly::nafems_nbc05_released_joint_beam_fixture;
use std::env;
use std::process::ExitCode;

const FROZEN_REQUEST: &str =
    include_str!("../../fixtures/beam_mode_tracking_request_v0_1.txt");
const ELEMENT_COUNTS: [usize; 4] = [1, 2, 4, 8];
const REQUESTED_MODES: usize = 6;

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("beam-mode-tracking-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
    let canonical = canonical_beam_mode_tracking_request(
        &nodes,
        &members,
        &ELEMENT_COUNTS,
        REQUESTED_MODES,
    )?;
    if canonical != FROZEN_REQUEST.trim_end_matches('\n') {
        return Err("canonical mode-tracking request differs from frozen fixture bytes".to_owned());
    }
    let report = run_beam_mode_tracking(&nodes, &members, &ELEMENT_COUNTS, REQUESTED_MODES)?;
    let total_pairs = report.levels.iter()
        .filter_map(|level| level.pairs_from_previous.as_ref())
        .map(|pairs| pairs.len())
        .sum::<usize>();
    let defined_combined_mac_count = report.levels.iter()
        .filter_map(|level| level.pairs_from_previous.as_ref())
        .flat_map(|pairs| pairs.iter())
        .filter(|pair| pair.mac.scaled_combined_mac.is_some())
        .count();
    let unresolved_combined_mac_count = total_pairs - defined_combined_mac_count;
    let pairing_status = if unresolved_combined_mac_count == 0 { "complete" } else { "partial" };

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"beam-mode-tracking-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BEAM-MODE-TRACKING-001\",",
            "\"references\":{{\"challenge\":\"https://www.nafems.org/downloads/nbc5_challenge.pdf\",\"solution\":\"https://www.nafems.org/downloads/nbr5v3.pdf\"}},",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"deterministic-software-mode-tracking-run\",",
            "\"result\":\"pass\",",
            "\"elements_per_member\":{},",
            "\"requested_mode_count\":{},",
            "\"rotation_scale_reference_length_m\":{},",
            "\"matching_algorithm\":{},",
            "\"mac_coordinate_policy\":{},",
            "\"pairing_status\":{},",
            "\"total_mode_pairs\":{},",
            "\"defined_combined_mac_count\":{},",
            "\"unresolved_combined_mac_count\":{},",
            "\"levels\":{},",
            "\"claim_ceiling\":\"common-station mode-shape tracking over 1/2/4/8 small meshes only; not the 200-element/member NAFEMS reproduction, experimental validation or structural safety\"",
            "}}"
        ),
        json_quote(&source_commit),
        json_quote(&canonical),
        usize_array_json(&report.elements_per_member),
        report.requested_mode_count,
        number(report.rotation_scale_reference_length_m),
        json_quote(report.matching_algorithm),
        json_quote(report.mac_coordinate_policy),
        json_quote(pairing_status),
        total_pairs,
        defined_combined_mac_count,
        unresolved_combined_mac_count,
        levels_json(&report.levels),
    ))
}

fn levels_json(levels: &[physical_transformation_reference::beam_mode_tracking::TrackedMeshLevel]) -> String {
    format!(
        "[{}]",
        levels.iter().map(|level| {
            format!(
                concat!(
                    "{{",
                    "\"elements_per_member\":{},",
                    "\"total_element_count\":{},",
                    "\"free_dof_count\":{},",
                    "\"frequencies_hz\":{},",
                    "\"maximum_normalized_eigen_residual\":{},",
                    "\"maximum_mass_norm_residual\":{},",
                    "\"maximum_mass_orthogonality_residual\":{},",
                    "\"pairs_from_previous\":{}",
                    "}}"
                ),
                level.elements_per_member,
                level.total_element_count,
                level.free_dof_count,
                float_array_json(&level.frequencies_hz),
                number(level.maximum_normalized_eigen_residual),
                number(level.maximum_mass_norm_residual),
                number(level.maximum_mass_orthogonality_residual),
                pairs_json(&level.pairs_from_previous),
            )
        }).collect::<Vec<_>>().join(",")
    )
}

fn pairs_json(pairs: &Option<Vec<physical_transformation_reference::beam_mode_tracking::TrackedModePair>>) -> String {
    let Some(pairs) = pairs else { return "null".to_owned(); };
    format!(
        "[{}]",
        pairs.iter().map(|pair| {
            format!(
                concat!(
                    "{{",
                    "\"previous_mode_ordinal\":{},",
                    "\"current_mode_ordinal\":{},",
                    "\"previous_frequency_hz\":{},",
                    "\"current_frequency_hz\":{},",
                    "\"relative_frequency_change\":{},",
                    "\"translation_mac\":{},",
                    "\"rotation_mac\":{},",
                    "\"scaled_combined_mac\":{},",
                    "\"common_translation_dof_count\":{},",
                    "\"common_rotation_dof_count\":{}",
                    "}}"
                ),
                pair.previous_mode_ordinal,
                pair.current_mode_ordinal,
                number(pair.previous_frequency_hz),
                number(pair.current_frequency_hz),
                number(pair.relative_frequency_change),
                optional_number(pair.mac.translation_mac),
                optional_number(pair.mac.rotation_mac),
                optional_number(pair.mac.scaled_combined_mac),
                pair.mac.common_translation_dof_count,
                pair.mac.common_rotation_dof_count,
            )
        }).collect::<Vec<_>>().join(",")
    )
}

fn optional_number(value: Option<f64>) -> String {
    value.map(number).unwrap_or_else(|| "null".to_owned())
}

fn number(value: f64) -> String {
    format!("{value:.17e}")
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err("usage: beam-mode-tracking-reference --source-commit <40-char-git-sha>".to_owned());
    }
    let value = &args[1];
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn json_quote(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => escaped.push_str(&format!("\\u{:04x}", character as u32)),
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

fn usize_array_json(values: &[usize]) -> String {
    format!("[{}]", values.iter().map(usize::to_string).collect::<Vec<_>>().join(","))
}

fn float_array_json(values: &[f64]) -> String {
    format!("[{}]", values.iter().map(|value| number(*value)).collect::<Vec<_>>().join(","))
}

#[cfg(test)]
mod tests {
    use super::parse_source_commit;

    #[test]
    fn source_commit_is_mandatory_and_strict() {
        assert!(parse_source_commit().is_err());
    }
}
