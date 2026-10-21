// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic receipt for the bounded NAFEMS axial-bar subproblem.

use physical_transformation_reference::modal_bar::{
    FixedFreeBar, MassFormulation, ModalFrequencies, analyze_fixed_free_bar,
};
use std::env;
use std::process::ExitCode;

const REQUEST: &str = include_str!("../../fixtures/modal_bar_request_v0_1.txt");
const FIXTURE_ID: &str = "NAFEMS-CHALLENGE-5-AXIAL-BAR";
const MESH_LADDER: [usize; 5] = [1, 2, 4, 8, 16];
const FIRST_MODE_RELATIVE_ERROR_LIMIT: f64 = 0.01;
const SECOND_MODE_RELATIVE_ERROR_LIMIT: f64 = 0.02;

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("modal-bar-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut source_commit: Option<String> = None;
    let mut fixture = "nafems-axial-bar";
    let mut fixture_was_supplied = false;
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
            "--fixture" => {
                if fixture_was_supplied {
                    return Err("--fixture may be supplied only once".to_owned());
                }
                fixture = args
                    .get(index + 1)
                    .ok_or_else(|| "--fixture requires a value".to_owned())?;
                fixture_was_supplied = true;
                index += 2;
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }
    let source_commit = source_commit.ok_or_else(|| "--source-commit is required".to_owned())?;
    if fixture != "nafems-axial-bar" {
        return Err(format!("unsupported fixture: {fixture}"));
    }

    let request = REQUEST
        .strip_suffix('\n')
        .ok_or_else(|| "frozen request must contain exactly one final newline".to_owned())?;
    if request.contains('\n') || request.contains('\r') {
        return Err("frozen request must be a single LF-terminated line".to_owned());
    }
    let frozen = parse_request(request)?;
    if frozen.fixture_id != FIXTURE_ID {
        return Err("frozen request fixture identifier does not match the CLI".to_owned());
    }

    let base = frozen.to_bar()?;
    let consistent = analyze_fixed_free_bar(base, MassFormulation::Consistent)?;
    let lumped = analyze_fixed_free_bar(base, MassFormulation::RowSumLumped)?;
    let ladder = build_mesh_ladder(base)?;
    verify_known_answers(&ladder, &consistent, &lumped)?;

    let ladder_json = ladder
        .iter()
        .map(|entry| {
            let second_consistent = entry
                .consistent
                .normalized_frequencies
                .get(1)
                .map(|value| format_number(*value))
                .unwrap_or_else(|| "null".to_owned());
            let second_lumped = entry
                .lumped
                .normalized_frequencies
                .get(1)
                .map(|value| format_number(*value))
                .unwrap_or_else(|| "null".to_owned());
            format!(
                "{{\"elements\":{},\"consistent_first_ratio\":{},\"lumped_first_ratio\":{},\"consistent_second_ratio\":{},\"lumped_second_ratio\":{}}}",
                entry.elements,
                format_number(entry.consistent.normalized_frequencies[0]),
                format_number(entry.lumped.normalized_frequencies[0]),
                second_consistent,
                second_lumped,
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let ratio_consistent_first = consistent.normalized_frequencies[0];
    let ratio_lumped_first = lumped.normalized_frequencies[0];
    let ratio_consistent_second = consistent.normalized_frequencies[1];
    let ratio_lumped_second = lumped.normalized_frequencies[1];

    Ok(format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"modal-bar-receipt-v0.1\",\n",
            "  \"benchmark_id\": \"PHYS-MODAL-BAR-001\",\n",
            "  \"fixture_id\": \"{}\",\n",
            "  \"source_commit\": \"{}\",\n",
            "  \"evidence_class\": \"software-test-run\",\n",
            "  \"result\": \"pass\",\n",
            "  \"request_canonical\": \"{}\",\n",
            "  \"input\": {{\"length_m\":{},\"radius_m\":{},\"area_m2\":{},\"youngs_modulus_pa\":{},\"density_kg_m3\":{},\"elements\":{}}},\n",
            "  \"mass_formulations\": {{\n",
            "    \"consistent\": {},\n",
            "    \"row_sum_lumped\": {}\n",
            "  }},\n",
            "  \"mesh_convergence\": [{}],\n",
            "  \"verification\": {{\n",
            "    \"analytical_one_element_known_answer\": \"pass\",\n",
            "    \"consistent_first_mode_above_continuum\": {},\n",
            "    \"lumped_first_mode_below_continuum\": {},\n",
            "    \"first_mode_relative_error_at_16_elements_below_1pct\": {},\n",
            "    \"second_mode_relative_error_at_16_elements_below_2pct\": {},\n",
            "    \"mesh_convergence\": \"pass\",\n",
            "    \"physical_validation\": \"not_performed\"\n",
            "  }},\n",
            "  \"claim_ceiling\": \"1D uniform fixed-free linear axial bar only; analytical/software verification, not full NAFEMS Challenge 5 2D truss reproduction, physical validation, modal test validation, or structural certification\"\n",
            "}}"
        ),
        frozen.fixture_id,
        source_commit,
        request,
        base.length_m,
        frozen.radius_m,
        base.area_m2,
        format_number(base.youngs_modulus_pa),
        format_number(base.density_kg_m3),
        base.elements,
        formulation_json(&consistent),
        formulation_json(&lumped),
        ladder_json,
        ratio_consistent_first > 1.0,
        ratio_lumped_first < 1.0,
        (ratio_consistent_first - 1.0).abs() < FIRST_MODE_RELATIVE_ERROR_LIMIT
            && (ratio_lumped_first - 1.0).abs() < FIRST_MODE_RELATIVE_ERROR_LIMIT,
        (ratio_consistent_second - 1.0).abs() < SECOND_MODE_RELATIVE_ERROR_LIMIT
            && (ratio_lumped_second - 1.0).abs() < SECOND_MODE_RELATIVE_ERROR_LIMIT,
    ))
}

#[derive(Clone)]
struct MeshEntry {
    elements: usize,
    consistent: ModalFrequencies,
    lumped: ModalFrequencies,
}

#[derive(Clone, Debug, PartialEq)]
struct ModalRequest {
    fixture_id: String,
    length_m: f64,
    radius_m: f64,
    youngs_modulus_pa: f64,
    density_kg_m3: f64,
    elements: usize,
}

impl ModalRequest {
    fn to_bar(&self) -> Result<FixedFreeBar, String> {
        if !self.radius_m.is_finite() || self.radius_m <= 0.0 {
            return Err("radius must be finite and positive".to_owned());
        }
        let area_m2 = std::f64::consts::PI * self.radius_m * self.radius_m;
        if !area_m2.is_finite() || area_m2 <= 0.0 {
            return Err("derived cross-sectional area is outside the supported numeric range".to_owned());
        }
        Ok(FixedFreeBar {
            length_m: self.length_m,
            area_m2,
            youngs_modulus_pa: self.youngs_modulus_pa,
            density_kg_m3: self.density_kg_m3,
            elements: self.elements,
        })
    }
}

fn parse_request(request: &str) -> Result<ModalRequest, String> {
    let mut fields = std::collections::BTreeMap::<&str, &str>::new();
    for part in request.split('|') {
        let (key, value) = part
            .split_once('=')
            .ok_or_else(|| format!("request field lacks '=' separator: {part}"))?;
        if key.is_empty() || value.is_empty() {
            return Err("request keys and values must be non-empty".to_owned());
        }
        if fields.insert(key, value).is_some() {
            return Err(format!("duplicate request field: {key}"));
        }
    }
    const REQUIRED_FIELDS: [&str; 7] = [
        "schema",
        "fixture",
        "length_m",
        "radius_m",
        "youngs_modulus_pa",
        "density_kg_m3",
        "elements",
    ];
    if fields.len() != REQUIRED_FIELDS.len()
        || REQUIRED_FIELDS.iter().any(|key| !fields.contains_key(key))
    {
        return Err("request must contain exactly the seven frozen schema fields".to_owned());
    }
    if fields["schema"] != "modal-bar-request-v0.1" {
        return Err("unsupported modal request schema".to_owned());
    }

    let length_m = parse_positive_number(&fields, "length_m")?;
    let radius_m = parse_positive_number(&fields, "radius_m")?;
    let youngs_modulus_pa = parse_positive_number(&fields, "youngs_modulus_pa")?;
    let density_kg_m3 = parse_positive_number(&fields, "density_kg_m3")?;
    let elements = fields["elements"]
        .parse::<usize>()
        .map_err(|_| "element count must be an unsigned integer".to_owned())?;
    Ok(ModalRequest {
        fixture_id: fields["fixture"].to_owned(),
        length_m,
        radius_m,
        youngs_modulus_pa,
        density_kg_m3,
        elements,
    })
}

fn parse_positive_number(
    fields: &std::collections::BTreeMap<&str, &str>,
    key: &str,
) -> Result<f64, String> {
    let value = fields
        .get(key)
        .ok_or_else(|| format!("missing request field: {key}"))?;
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("request field {key} is not numeric"))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(format!("request field {key} must be finite and positive"));
    }
    Ok(parsed)
}

fn build_mesh_ladder(base: FixedFreeBar) -> Result<Vec<MeshEntry>, String> {
    MESH_LADDER
        .iter()
        .map(|elements| {
            let mut bar = base;
            bar.elements = *elements;
            Ok(MeshEntry {
                elements: *elements,
                consistent: analyze_fixed_free_bar(bar, MassFormulation::Consistent)?,
                lumped: analyze_fixed_free_bar(bar, MassFormulation::RowSumLumped)?,
            })
        })
        .collect()
}

fn verify_known_answers(
    ladder: &[MeshEntry],
    consistent: &ModalFrequencies,
    lumped: &ModalFrequencies,
) -> Result<(), String> {
    let one = &ladder[0];
    close(
        one.consistent.normalized_frequencies[0],
        2.0 * 3.0_f64.sqrt() / std::f64::consts::PI,
        1.0e-12,
        "one-element consistent mass ratio",
    )?;
    close(
        one.lumped.normalized_frequencies[0],
        2.0 * 2.0_f64.sqrt() / std::f64::consts::PI,
        1.0e-12,
        "one-element lumped mass ratio",
    )?;
    for (formulation, ratios, greater) in [
        ("consistent", &consistent.normalized_frequencies, true),
        ("row-sum-lumped", &lumped.normalized_frequencies, false),
    ] {
        if ratios.len() != 16 {
            return Err(format!("{formulation} model did not return 16 axial modes"));
        }
        if (ratios[0] > 1.0) != greater {
            return Err(format!(
                "{formulation} first-mode ratio violates expected bracketing"
            ));
        }
        if (ratios[1] > 1.0) != greater {
            return Err(format!(
                "{formulation} second-mode ratio violates expected bracketing"
            ));
        }
    }
    for formulation in ["consistent", "row-sum-lumped"] {
        for mode_index in 0..2 {
            let error_values: Vec<f64> = ladder
                .iter()
                .filter_map(|entry| {
                    let result = match formulation {
                        "consistent" => &entry.consistent,
                        _ => &entry.lumped,
                    };
                    result
                        .normalized_frequencies
                        .get(mode_index)
                        .map(|ratio| (ratio - 1.0).abs())
                })
                .collect();
            if error_values.windows(2).any(|pair| pair[1] >= pair[0]) {
                return Err(format!(
                    "{formulation} mode {} does not converge monotonically on the frozen mesh ladder",
                    mode_index + 1
                ));
            }
        }
    }
    let first_limit = FIRST_MODE_RELATIVE_ERROR_LIMIT;
    let second_limit = SECOND_MODE_RELATIVE_ERROR_LIMIT;
    if (consistent.normalized_frequencies[0] - 1.0).abs() >= first_limit
        || (lumped.normalized_frequencies[0] - 1.0).abs() >= first_limit
        || (consistent.normalized_frequencies[1] - 1.0).abs() >= second_limit
        || (lumped.normalized_frequencies[1] - 1.0).abs() >= second_limit
    {
        return Err(
            "final mesh does not meet the declared first-/second-mode error limits".to_owned(),
        );
    }
    Ok(())
}

fn close(actual: f64, expected: f64, tolerance: f64, label: &str) -> Result<(), String> {
    if (actual - expected).abs() <= tolerance {
        Ok(())
    } else {
        Err(format!(
            "{label}: actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}"
        ))
    }
}

fn formulation_json(result: &ModalFrequencies) -> String {
    format!(
        "{{\"formulation\":\"{}\",\"frequencies_hz\":{},\"exact_frequencies_hz\":{},\"normalized_frequencies\":{}}}",
        match result.formulation {
            MassFormulation::Consistent => "consistent",
            MassFormulation::RowSumLumped => "row_sum_lumped",
        },
        number_array(&result.frequencies_hz),
        number_array(&result.exact_frequencies_hz),
        number_array(&result.normalized_frequencies),
    )
}

fn number_array(values: &[f64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format_number(*value))
            .collect::<Vec<_>>()
            .join(",")
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
    use super::{FIXTURE_ID, REQUEST, normalize_source_commit, parse_request};

    #[test]
    fn source_commit_requires_exactly_40_hexadecimal_characters() {
        assert!(normalize_source_commit("0123456789abcdef0123456789abcdef01234567").is_ok());
        assert!(normalize_source_commit("0123456789abcdef0123456789abcdef0123456z").is_err());
        assert!(normalize_source_commit("not-a-commit").is_err());
    }

    #[test]
    fn frozen_request_drives_the_reported_physical_parameters() {
        let canonical = REQUEST.strip_suffix('\n').expect("frozen request is LF-terminated");
        let request = parse_request(canonical).expect("valid frozen modal request");
        assert_eq!(request.fixture_id, FIXTURE_ID);
        assert_eq!(request.length_m, 1.0);
        assert_eq!(request.radius_m, 0.01);
        assert_eq!(request.youngs_modulus_pa, 210.0e9);
        assert_eq!(request.density_kg_m3, 7_800.0);
        assert_eq!(request.elements, 16);
        let bar = request.to_bar().expect("valid bar geometry");
        assert_eq!(bar.area_m2, std::f64::consts::PI * 0.01_f64 * 0.01_f64);
    }

    #[test]
    fn request_parser_rejects_duplicate_unknown_and_unsupported_fields() {
        let canonical = REQUEST.strip_suffix('\n').expect("frozen request is LF-terminated");
        assert!(parse_request(&format!("{canonical}|elements=2")).is_err());
        assert!(parse_request(&format!("{canonical}|unreviewed=1")).is_err());
        assert!(parse_request(&canonical.replace("schema=modal-bar-request-v0.1", "schema=modal-bar-request-v0.2")).is_err());
    }
}
