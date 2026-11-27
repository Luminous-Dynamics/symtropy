use orbital_gravity_curvature_bound_bench::{
    CurvatureDisposition, CurvatureInput, CurvatureResult, EvidenceSet, ProviderEvidence,
    ProviderState, RECEIPT_SCHEMA, Scope, derive,
};
use std::env;
use std::error::Error;
use std::fs;

struct ParsedCase {
    input: CurvatureInput,
    expected: String,
}

fn parse_u64(value: &str, field: &str) -> Result<u64, Box<dyn Error>> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_u128(value: &str, field: &str) -> Result<u128, Box<dyn Error>> {
    value
        .parse::<u128>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_bool(value: &str, field: &str) -> Result<bool, Box<dyn Error>> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid {field}: expected true or false").into()),
    }
}

fn parse_state(value: &str, field: &str) -> Result<ProviderState, Box<dyn Error>> {
    ProviderState::parse(value).ok_or_else(|| format!("invalid {field} state").into())
}

fn parse_provider(fields: &[&str], offset: usize, name: &str) -> Result<ProviderEvidence, Box<dyn Error>> {
    Ok(ProviderEvidence {
        provider_id: fields[offset].to_owned(),
        provider_revision: fields[offset + 1].to_owned(),
        receipt_sha256: fields[offset + 2].to_owned(),
        scope_id: fields[offset + 3].to_owned(),
        state: parse_state(fields[offset + 4], name)?,
    })
}

fn parse_row(fields: &[&str], line_number: usize) -> Result<ParsedCase, Box<dyn Error>> {
    if fields.len() != 51 {
        return Err(format!(
            "line {line_number}: expected 51 comma-separated fields, found {}",
            fields.len()
        )
        .into());
    }

    let scope = Scope {
        scope_id: fields[2].to_owned(),
        action_id: fields[3].to_owned(),
        segment_id: fields[4].to_owned(),
        target_id: fields[5].to_owned(),
        chief_id: fields[6].to_owned(),
        servicer_id: fields[7].to_owned(),
        central_body_id: fields[8].to_owned(),
        keepout_profile_id: fields[9].to_owned(),
        keepout_geometry_revision: fields[10].to_owned(),
        transform_digest: fields[11].to_owned(),
        frame_id: fields[12].to_owned(),
        frame_revision: fields[13].to_owned(),
        time_basis_id: fields[14].to_owned(),
        dynamics_profile_revision: fields[15].to_owned(),
        start_tai_ns: parse_u64(fields[16], "start_tai_ns")?,
        end_tai_ns: parse_u64(fields[17], "end_tai_ns")?,
    };

    let evidence = EvidenceSet {
        mu: parse_provider(fields, 26, "mu")?,
        chief_radius: parse_provider(fields, 31, "chief_radius")?,
        relative_separation: parse_provider(fields, 36, "relative_separation")?,
        differential_acceleration: parse_provider(fields, 41, "differential_acceleration")?,
        transform_norm: parse_provider(fields, 46, "transform_norm")?,
    };

    Ok(ParsedCase {
        input: CurvatureInput {
            case_id: fields[0].to_owned(),
            scope,
            mu_upper_mm3_per_s2: parse_u128(fields[18], "mu_upper_mm3_per_s2")?,
            chief_radius_lower_mm: parse_u64(fields[19], "chief_radius_lower_mm")?,
            relative_separation_upper_mm: parse_u64(fields[20], "relative_separation_upper_mm")?,
            differential_acceleration_upper_mm_per_s2: parse_u64(
                fields[21],
                "differential_acceleration_upper_mm_per_s2",
            )?,
            transform_operator_norm_upper_scaled: parse_u64(
                fields[22],
                "transform_operator_norm_upper_scaled",
            )?,
            transform_time_invariant: parse_bool(fields[23], "transform_time_invariant")?,
            perturbation_inventory_id: fields[24].to_owned(),
            perturbation_inventory_complete: parse_bool(
                fields[25],
                "perturbation_inventory_complete",
            )?,
            evidence,
        },
        expected: fields[1].to_owned(),
    })
}

fn json_escape(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control.is_control() => {
                output.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => output.push(other),
        }
    }
    output
}

fn optional_u128(value: Option<u128>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn render_scope(scope: &Scope) -> String {
    format!(
        "{{\"scope_id\":\"{}\",\"action_id\":\"{}\",\"segment_id\":\"{}\",\"target_id\":\"{}\",\"chief_id\":\"{}\",\"servicer_id\":\"{}\",\"central_body_id\":\"{}\",\"keepout_profile_id\":\"{}\",\"keepout_geometry_revision\":\"{}\",\"transform_digest\":\"{}\",\"frame_id\":\"{}\",\"frame_revision\":\"{}\",\"time_basis_id\":\"{}\",\"dynamics_profile_revision\":\"{}\",\"start_tai_ns\":{},\"end_tai_ns\":{}}}",
        json_escape(&scope.scope_id),
        json_escape(&scope.action_id),
        json_escape(&scope.segment_id),
        json_escape(&scope.target_id),
        json_escape(&scope.chief_id),
        json_escape(&scope.servicer_id),
        json_escape(&scope.central_body_id),
        json_escape(&scope.keepout_profile_id),
        json_escape(&scope.keepout_geometry_revision),
        json_escape(&scope.transform_digest),
        json_escape(&scope.frame_id),
        json_escape(&scope.frame_revision),
        json_escape(&scope.time_basis_id),
        json_escape(&scope.dynamics_profile_revision),
        scope.start_tai_ns,
        scope.end_tai_ns,
    )
}

fn render_provider(provider: &ProviderEvidence) -> String {
    format!(
        "{{\"provider_id\":\"{}\",\"provider_revision\":\"{}\",\"receipt_sha256\":\"{}\",\"scope_id\":\"{}\",\"state\":\"{}\"}}",
        json_escape(&provider.provider_id),
        json_escape(&provider.provider_revision),
        json_escape(&provider.receipt_sha256),
        json_escape(&provider.scope_id),
        provider.state.as_str(),
    )
}

fn render_result(result: &CurvatureResult) -> String {
    let evidence = &result.evidence;
    let reasons = result
        .reason_codes
        .iter()
        .map(|reason| format!("\"{}\"", json_escape(reason)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"case_id\":\"{}\",\"scope\":{},\"disposition\":\"{}\",\"central_gravity_numerator\":{},\"differential_acceleration_numerator\":{},\"radial_domain_denominator\":{},\"exact_total_numerator\":{},\"exact_total_denominator\":{},\"m_upper_scaled_per_s2\":{},\"radial_slack_mm\":{},\"perturbation_inventory_id\":\"{}\",\"perturbation_inventory_complete\":{},\"evidence\":{{\"mu\":{},\"chief_radius\":{},\"relative_separation\":{},\"differential_acceleration\":{},\"transform_norm\":{}}},\"reason_codes\":[{}],\"bound_ready\":{},\"source_bytes_authenticated\":{},\"authority_granted\":{},\"physical_validation\":\"{}\"}}",
        json_escape(&result.case_id),
        render_scope(&result.scope),
        result.disposition.as_str(),
        optional_u128(result.central_gravity_numerator),
        optional_u128(result.differential_acceleration_numerator),
        optional_u128(result.radial_domain_denominator),
        optional_u128(result.exact_total_numerator),
        optional_u128(result.exact_total_denominator),
        optional_u128(result.m_upper_scaled_per_s2),
        result.radial_slack_mm.map_or_else(|| "null".to_owned(), |value| value.to_string()),
        json_escape(&result.perturbation_inventory_id),
        result.perturbation_inventory_complete,
        render_provider(&evidence.mu),
        render_provider(&evidence.chief_radius),
        render_provider(&evidence.relative_separation),
        render_provider(&evidence.differential_acceleration),
        render_provider(&evidence.transform_norm),
        reasons,
        result.bound_ready,
        result.source_bytes_authenticated,
        result.authority_granted,
        json_escape(&result.physical_validation),
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let corpus_path = args
        .next()
        .ok_or("usage: bench <corpus.csv> <source_commit> <corpus_sha256>")?;
    let source_commit = args.next().ok_or("missing source_commit")?;
    let corpus_sha256 = args.next().ok_or("missing corpus_sha256")?;
    let content = fs::read_to_string(&corpus_path)?;

    let mut results = Vec::new();
    let mut matches = 0_usize;
    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case_id,") {
            continue;
        }
        let fields = line.split(',').collect::<Vec<_>>();
        let parsed = parse_row(&fields, index + 1)?;
        let result = derive(&parsed.input);
        if result.disposition.as_str() != parsed.expected {
            return Err(format!(
                "{}: expected {}, received {}",
                parsed.input.case_id,
                parsed.expected,
                result.disposition.as_str()
            )
            .into());
        }
        matches += 1;
        results.push(result);
    }

    if results.is_empty() {
        return Err("empty curvature corpus".into());
    }

    let rendered = results.iter().map(render_result).collect::<Vec<_>>().join(",");
    let bound_ready_count = results
        .iter()
        .filter(|result| result.disposition == CurvatureDisposition::BoundReady)
        .count();
    println!(
        "{{\"schema\":\"{}\",\"source_commit\":\"{}\",\"corpus_sha256\":\"{}\",\"case_count\":{},\"expected_disposition_matches\":{},\"expected_disposition_mismatches\":0,\"bound_ready_count\":{},\"claim_ready\":false,\"source_bytes_authenticated\":false,\"authority_granted\":false,\"physical_validation\":\"NOT_ASSESSED\",\"results\":[{}]}}",
        RECEIPT_SCHEMA,
        json_escape(&source_commit),
        json_escape(&corpus_sha256),
        results.len(),
        matches,
        bound_ready_count,
        rendered
    );
    Ok(())
}
