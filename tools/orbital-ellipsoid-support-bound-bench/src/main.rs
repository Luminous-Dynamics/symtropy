use orbital_ellipsoid_support_bound_bench::{
    EvidenceSet, ProviderEvidence, ProviderState, RECEIPT_SCHEMA, SupportInput, SupportResult,
    SupportScope, derive,
};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;

struct ParsedCase {
    input: SupportInput,
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

fn parse_state(value: &str, field: &str) -> Result<ProviderState, Box<dyn Error>> {
    ProviderState::parse(value).ok_or_else(|| format!("invalid {field} state").into())
}

fn parse_provider(
    fields: &[&str],
    offset: usize,
    name: &str,
) -> Result<ProviderEvidence, Box<dyn Error>> {
    Ok(ProviderEvidence {
        provider_id: fields[offset].to_owned(),
        provider_revision: fields[offset + 1].to_owned(),
        receipt_sha256: fields[offset + 2].to_owned(),
        scope_id: fields[offset + 3].to_owned(),
        state: parse_state(fields[offset + 4], name)?,
    })
}

fn parse_row(fields: &[&str], line_number: usize) -> Result<ParsedCase, Box<dyn Error>> {
    if fields.len() != 33 {
        return Err(format!(
            "line {line_number}: expected 33 comma-separated fields, found {}",
            fields.len()
        )
        .into());
    }

    let scope = SupportScope {
        scope_id: fields[2].to_owned(),
        terminal_action_id: fields[3].to_owned(),
        target_id: fields[4].to_owned(),
        keepout_profile_id: fields[5].to_owned(),
        keepout_geometry_revision: fields[6].to_owned(),
        matrix_sha256: fields[7].to_owned(),
        matrix_convention: fields[8].to_owned(),
        frame_id: fields[9].to_owned(),
        frame_revision: fields[10].to_owned(),
        transform_digest: fields[11].to_owned(),
        direction_sha256: fields[12].to_owned(),
        direction_convention: fields[13].to_owned(),
        length_unit: fields[14].to_owned(),
        terminal_epoch_tai_ns: parse_u64(fields[15], "terminal_epoch_tai_ns")?,
    };

    Ok(ParsedCase {
        input: SupportInput {
            case_id: fields[0].to_owned(),
            scope,
            q_lower_numerator: parse_u128(fields[16], "q_lower_numerator")?,
            q_lower_denominator: parse_u128(fields[17], "q_lower_denominator")?,
            evidence: EvidenceSet {
                matrix_spd: parse_provider(fields, 18, "matrix_spd")?,
                direction_unit: parse_provider(fields, 23, "direction_unit")?,
                quadratic_lower: parse_provider(fields, 28, "quadratic_lower")?,
            },
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

fn render_scope(scope: &SupportScope) -> String {
    format!(
        "{{\"scope_id\":\"{}\",\"terminal_action_id\":\"{}\",\"target_id\":\"{}\",\"keepout_profile_id\":\"{}\",\"keepout_geometry_revision\":\"{}\",\"matrix_sha256\":\"{}\",\"matrix_convention\":\"{}\",\"frame_id\":\"{}\",\"frame_revision\":\"{}\",\"transform_digest\":\"{}\",\"direction_sha256\":\"{}\",\"direction_convention\":\"{}\",\"length_unit\":\"{}\",\"terminal_epoch_tai_ns\":{}}}",
        json_escape(&scope.scope_id),
        json_escape(&scope.terminal_action_id),
        json_escape(&scope.target_id),
        json_escape(&scope.keepout_profile_id),
        json_escape(&scope.keepout_geometry_revision),
        json_escape(&scope.matrix_sha256),
        json_escape(&scope.matrix_convention),
        json_escape(&scope.frame_id),
        json_escape(&scope.frame_revision),
        json_escape(&scope.transform_digest),
        json_escape(&scope.direction_sha256),
        json_escape(&scope.direction_convention),
        json_escape(&scope.length_unit),
        scope.terminal_epoch_tai_ns,
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

fn render_result(result: &SupportResult) -> String {
    let evidence = &result.evidence;
    let reasons = result
        .reason_codes
        .iter()
        .map(|reason| format!("\"{}\"", json_escape(reason)))
        .collect::<Vec<_>>()
        .join(",");
    let rho = result
        .rho_y_upper_um
        .map_or_else(|| "null".to_owned(), |value| value.to_string());
    let ceiling = result
        .q_scaled_ceiling
        .map_or_else(|| "null".to_owned(), |value| value.to_string());
    let floor = result
        .floor_sqrt
        .map_or_else(|| "null".to_owned(), |value| value.to_string());
    let perfect = result
        .perfect_square
        .map_or_else(|| "null".to_owned(), |value| value.to_string());
    format!(
        "{{\"case_id\":\"{}\",\"scope\":{},\"disposition\":\"{}\",\"q_lower_numerator\":{},\"q_lower_denominator\":{},\"q_unit\":\"{}\",\"rho_y_upper_um\":{},\"rho_unit\":\"{}\",\"scale_um_per_m\":{},\"q_scaled_ceiling\":{},\"floor_sqrt\":{},\"perfect_square\":{},\"evidence\":{{\"matrix_spd\":{},\"direction_unit\":{},\"quadratic_lower\":{}}},\"reason_codes\":[{}],\"bound_ready\":{},\"source_bytes_authenticated\":{},\"physical_geometry_validated\":{},\"authority_granted\":{}}}",
        json_escape(&result.case_id),
        render_scope(&result.scope),
        result.disposition.as_str(),
        result.q_lower_numerator,
        result.q_lower_denominator,
        json_escape(&result.q_unit),
        rho,
        json_escape(&result.rho_unit),
        result.scale_um_per_m,
        ceiling,
        floor,
        perfect,
        render_provider(&evidence.matrix_spd),
        render_provider(&evidence.direction_unit),
        render_provider(&evidence.quadratic_lower),
        reasons,
        result.bound_ready,
        result.source_bytes_authenticated,
        result.physical_geometry_validated,
        result.authority_granted,
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

    let mut case_ids = BTreeSet::new();
    let mut results = Vec::new();
    let mut matches = 0_usize;
    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case_id,") {
            continue;
        }
        let parsed = parse_row(&line.split(',').collect::<Vec<_>>(), index + 1)?;
        if !case_ids.insert(parsed.input.case_id.clone()) {
            return Err(format!("duplicate case ID: {}", parsed.input.case_id).into());
        }
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
        return Err("empty ellipsoid support corpus".into());
    }

    let rendered = results
        .iter()
        .map(render_result)
        .collect::<Vec<_>>()
        .join(",");
    let bound_ready_count = results.iter().filter(|result| result.bound_ready).count();
    println!(
        "{{\"schema\":\"{}\",\"source_commit\":\"{}\",\"corpus_sha256\":\"{}\",\"case_count\":{},\"expected_disposition_matches\":{},\"expected_disposition_mismatches\":0,\"bound_ready_count\":{},\"claim_ready\":false,\"source_bytes_authenticated\":false,\"physical_geometry_validated\":false,\"authority_granted\":false,\"results\":[{}]}}",
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
