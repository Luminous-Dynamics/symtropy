// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic receipt for the published NAFEMS Challenge 5 two-bar modal fixture.

use physical_transformation_reference::modal_truss::{
    ModalTrussBar, ModalTrussInput, ModalTrussMassFormulation, ModalTrussNode, ModalTrussResult,
    analyze_planar_modal_truss,
};
use std::env;
use std::process::ExitCode;

const REQUEST: &str =
    include_str!("../../fixtures/nafems_challenge5_modal_request_v0_1.txt");
const FIXTURE_ID: &str = "NAFEMS-C5-ORIGINAL-TRUSS";
const CONSISTENT_REFERENCE_HZ: [f64; 2] = [452.09, 1_114.6];
const LUMPED_REFERENCE_HZ: [f64; 2] = [369.13, 910.05];
const RELATIVE_ERROR_LIMIT: f64 = 2.0e-4;
const RESIDUAL_LIMIT: f64 = 1.0e-10;

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("nafems-modal-truss-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut source_commit: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--source-commit" => {
                if source_commit.is_some() {
                    return Err("--source-commit may be supplied only once".to_owned());
                }
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--source-commit requires a value".to_owned())?;
                source_commit = Some(normalize_source_commit(value)?);
                index += 2;
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }
    let source_commit = source_commit.ok_or_else(|| "--source-commit is required".to_owned())?;
    let canonical = REQUEST
        .strip_suffix('\n')
        .ok_or_else(|| "frozen request must contain exactly one final newline".to_owned())?;
    if canonical.contains('\n') || canonical.contains('\r') {
        return Err("frozen request must be one LF-terminated canonical line".to_owned());
    }
    let parsed = parse_request(canonical)?;
    if parsed.fixture_id != FIXTURE_ID {
        return Err("fixture identifier does not match the receipt contract".to_owned());
    }

    let consistent = analyze_planar_modal_truss(
        &parsed.input,
        ModalTrussMassFormulation::Consistent,
    )?;
    let lumped = analyze_planar_modal_truss(
        &parsed.input,
        ModalTrussMassFormulation::RowSumLumped,
    )?;
    verify_published_targets(&consistent, &CONSISTENT_REFERENCE_HZ, "consistent")?;
    verify_published_targets(&lumped, &LUMPED_REFERENCE_HZ, "row-sum-lumped")?;

    let nodes_json = parsed
        .input
        .nodes
        .iter()
        .map(|node| {
            format!(
                "{{\"id\":\"{}\",\"x_m\":{},\"y_m\":{},\"restrained_x\":{},\"restrained_y\":{}}}",
                node.id,
                format_number(node.x_m),
                format_number(node.y_m),
                node.restrained_x,
                node.restrained_y
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let bars_json = parsed
        .input
        .bars
        .iter()
        .map(|bar| {
            format!(
                "{{\"id\":\"{}\",\"node_i\":\"{}\",\"node_j\":\"{}\",\"area_m2\":{},\"radius_m\":{},\"youngs_modulus_pa\":{},\"density_kg_m3\":{}}}",
                bar.id,
                bar.node_i,
                bar.node_j,
                format_number(bar.area_m2),
                format_number((bar.area_m2 / std::f64::consts::PI).sqrt()),
                format_number(bar.youngs_modulus_pa),
                format_number(bar.density_kg_m3)
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    Ok(format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"modal-truss-receipt-v0.1\",\n",
            "  \"benchmark_id\": \"PHYS-MODAL-NAFEMS-C5-001\",\n",
            "  \"fixture_id\": \"{}\",\n",
            "  \"source_commit\": \"{}\",\n",
            "  \"evidence_class\": \"software-test-run-with-published-reference\",\n",
            "  \"result\": \"pass\",\n",
            "  \"request_canonical\": \"{}\",\n",
            "  \"input\": {{\"nodes\":[{}],\"bars\":[{}]}},\n",
            "  \"consistent_mass\": {},\n",
            "  \"row_sum_lumped_mass\": {},\n",
            "  \"verification\": {{\n",
            "    \"target_relative_error_limit\": {},\n",
            "    \"normalized_eigen_residual_limit\": {},\n",
            "    \"mass_orthogonality_residual_limit\": {},\n",
            "    \"published_frequency_targets\": \"pass\",\n",
            "    \"mode_shape_residuals\": \"pass\",\n",
            "    \"mass_orthogonality\": \"pass\",\n",
            "    \"physical_validation\": \"not_performed\"\n",
            "  }},\n",
            "  \"claim_ceiling\": \"original single-element-per-member NAFEMS Challenge 5 truss frequencies only; not the full refinement, constraint-equation, mode-shape sensitivity study, physical validation, or structural certification\"\n",
            "}}"
        ),
        parsed.fixture_id,
        source_commit,
        canonical,
        nodes_json,
        bars_json,
        result_json(&consistent, &CONSISTENT_REFERENCE_HZ),
        result_json(&lumped, &LUMPED_REFERENCE_HZ),
        format_number(RELATIVE_ERROR_LIMIT),
        format_number(RESIDUAL_LIMIT),
        format_number(RESIDUAL_LIMIT)
    ))
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedRequest {
    fixture_id: String,
    input: ModalTrussInput,
}

fn parse_request(canonical: &str) -> Result<ParsedRequest, String> {
    if canonical.contains('\n') || canonical.contains('\r') {
        return Err("canonical request must contain no line breaks".to_owned());
    }
    let mut schema: Option<&str> = None;
    let mut fixture_id: Option<&str> = None;
    let mut nodes = Vec::new();
    let mut bars = Vec::new();

    for item in canonical.split('|') {
        let (key, value) = item
            .split_once('=')
            .ok_or_else(|| format!("request field lacks '=' separator: {item}"))?;
        if key.is_empty() || value.is_empty() {
            return Err("request keys and values must be non-empty".to_owned());
        }
        match key {
            "schema" => {
                if schema.replace(value).is_some() {
                    return Err("duplicate schema field".to_owned());
                }
            }
            "fixture" => {
                if fixture_id.replace(value).is_some() {
                    return Err("duplicate fixture field".to_owned());
                }
            }
            "node" => nodes.push(parse_node(value)?),
            "bar" => bars.push(parse_bar(value)?),
            _ => return Err(format!("unsupported request field: {key}")),
        }
    }

    if schema != Some("modal-truss-request-v0.1") {
        return Err("unsupported or missing request schema".to_owned());
    }
    if nodes.len() != 3 || bars.len() != 2 {
        return Err("NAFEMS Challenge 5 fixture requires exactly three nodes and two bars".to_owned());
    }
    let fixture_id = fixture_id.ok_or_else(|| "missing fixture field".to_owned())?;
    Ok(ParsedRequest {
        fixture_id: fixture_id.to_owned(),
        input: ModalTrussInput { nodes, bars },
    })
}

fn parse_node(value: &str) -> Result<ModalTrussNode, String> {
    let fields: Vec<&str> = value.split(',').collect();
    if fields.len() != 5 {
        return Err("node request requires id,x,y,restrained_x,restrained_y".to_owned());
    }
    let x_m = parse_finite(fields[1], "node x")?;
    let y_m = parse_finite(fields[2], "node y")?;
    Ok(ModalTrussNode {
        id: fields[0].to_owned(),
        x_m,
        y_m,
        restrained_x: parse_bool(fields[3], "restrained_x")?,
        restrained_y: parse_bool(fields[4], "restrained_y")?,
    })
}

fn parse_bar(value: &str) -> Result<ModalTrussBar, String> {
    let fields: Vec<&str> = value.split(',').collect();
    if fields.len() != 6 {
        return Err("bar request requires id,node_i,node_j,radius_m,E_pa,density_kg_m3".to_owned());
    }
    let radius_m = parse_positive(fields[3], "bar radius")?;
    let area_m2 = std::f64::consts::PI * radius_m * radius_m;
    if !area_m2.is_finite() || area_m2 <= 0.0 {
        return Err("derived bar area is outside the supported numeric range".to_owned());
    }
    Ok(ModalTrussBar {
        id: fields[0].to_owned(),
        node_i: fields[1].to_owned(),
        node_j: fields[2].to_owned(),
        area_m2,
        youngs_modulus_pa: parse_positive(fields[4], "Young's modulus")?,
        density_kg_m3: parse_positive(fields[5], "density")?,
    })
}

fn parse_bool(value: &str, label: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{label} must be literal true or false")),
    }
}

fn parse_finite(value: &str, label: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("{label} must be numeric"))?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(format!("{label} must be finite"))
    }
}

fn parse_positive(value: &str, label: &str) -> Result<f64, String> {
    let parsed = parse_finite(value, label)?;
    if parsed <= 0.0 {
        return Err(format!("{label} must be positive"));
    }
    Ok(parsed)
}

fn verify_published_targets(
    result: &ModalTrussResult,
    expected_hz: &[f64; 2],
    label: &str,
) -> Result<(), String> {
    if result.modes.len() != 2 {
        return Err(format!("{label} returned {} modes, expected two", result.modes.len()));
    }
    if result.near_zero_mode_count != 0 {
        return Err(format!("{label} reported near-zero modes"));
    }
    if result.maximum_mass_orthogonality_residual >= RESIDUAL_LIMIT {
        return Err(format!("{label} mass orthogonality residual exceeded its limit"));
    }
    for (index, (mode, expected)) in result.modes.iter().zip(expected_hz.iter()).enumerate() {
        if mode.classification != physical_transformation_reference::modal_truss::ModalModeClassification::Elastic {
            return Err(format!("{label} mode {} is not classified elastic", index + 1));
        }
        if !mode.normalized_eigen_residual.is_finite()
            || mode.normalized_eigen_residual >= RESIDUAL_LIMIT
        {
            return Err(format!("{label} mode {} eigen residual exceeded its limit", index + 1));
        }
        let relative_error = (mode.frequency_hz - expected).abs() / expected.abs();
        if !relative_error.is_finite() || relative_error >= RELATIVE_ERROR_LIMIT {
            return Err(format!(
                "{label} mode {} frequency relative error {:.6e} exceeded {:.6e}",
                index + 1,
                relative_error,
                RELATIVE_ERROR_LIMIT
            ));
        }
    }
    Ok(())
}

fn result_json(result: &ModalTrussResult, expected_hz: &[f64; 2]) -> String {
    format!(
        "{{\"mass_formulation\":\"{}\",\"frequencies_hz\":{},\"omega_squared_rad2_s2\":{},\"published_reference_hz\":{},\"relative_errors\":{},\"normalized_eigen_residuals\":{},\"maximum_mass_orthogonality_residual\":{},\"near_zero_mode_count\":{}}}",
        match result.mass_formulation {
            ModalTrussMassFormulation::Consistent => "consistent",
            ModalTrussMassFormulation::RowSumLumped => "row_sum_lumped",
        },
        number_array(&result.modes.iter().map(|mode| mode.frequency_hz).collect::<Vec<_>>()),
        number_array(
            &result
                .modes
                .iter()
                .map(|mode| mode.omega_squared_rad2_s2)
                .collect::<Vec<_>>()
        ),
        number_array(expected_hz),
        number_array(
            &result
                .modes
                .iter()
                .zip(expected_hz.iter())
                .map(|(mode, expected)| (mode.frequency_hz - expected).abs() / expected.abs())
                .collect::<Vec<_>>()
        ),
        number_array(
            &result
                .modes
                .iter()
                .map(|mode| mode.normalized_eigen_residual)
                .collect::<Vec<_>>()
        ),
        format_number(result.maximum_mass_orthogonality_residual),
        result.near_zero_mode_count
    )
}

fn number_array(values: &[f64]) -> String {
    format!(
        "[{}]",
        values.iter().map(|value| format_number(*value)).collect::<Vec<_>>().join(",")
    )
}

fn format_number(value: f64) -> String {
    format!("{value:.15e}")
}

fn normalize_source_commit(value: &str) -> Result<String, String> {
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("--source-commit must be exactly 40 hexadecimal characters".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{REQUEST, normalize_source_commit, parse_request};

    #[test]
    fn source_commit_requires_exactly_40_hexadecimal_characters() {
        assert!(normalize_source_commit("0123456789abcdef0123456789abcdef01234567").is_ok());
        assert!(normalize_source_commit("0123456789abcdef0123456789abcdef0123456z").is_err());
        assert!(normalize_source_commit("not-a-commit").is_err());
    }

    #[test]
    fn frozen_request_drives_all_three_nodes_and_two_bars() {
        let canonical = REQUEST.strip_suffix('\n').expect("frozen request is LF-terminated");
        let parsed = parse_request(canonical).expect("canonical NAFEMS request");
        assert_eq!(parsed.fixture_id, "NAFEMS-C5-ORIGINAL-TRUSS");
        assert_eq!(parsed.input.nodes.len(), 3);
        assert_eq!(parsed.input.bars.len(), 2);
        assert_eq!(parsed.input.nodes[0].x_m, 0.0);
        assert_eq!(parsed.input.nodes[0].y_m, 0.0);
        assert_eq!(parsed.input.nodes[1].x_m, 0.0);
        assert_eq!(parsed.input.nodes[1].y_m, 1.0);
        assert_eq!(parsed.input.nodes[2].x_m, 1.0);
        assert_eq!(parsed.input.nodes[2].y_m, 1.0);
        assert!((parsed.input.bars[0].area_m2 - std::f64::consts::PI * 1.0e-4).abs() < 1.0e-18);
    }

    #[test]
    fn request_parser_rejects_unknown_duplicate_and_malformed_fields() {
        let canonical = REQUEST.strip_suffix('\n').expect("frozen request is LF-terminated");
        assert!(parse_request(&format!("{canonical}|rogue=1")).is_err());
        assert!(parse_request(&format!("{canonical}|schema=modal-truss-request-v0.1")).is_err());
        assert!(parse_request(&canonical.replace("schema=modal-truss-request-v0.1", "schema=modal-truss-request-v0.9")).is_err());
        assert!(parse_request("schema=modal-truss-request-v0.1|fixture=x|node=bad").is_err());
    }
}
