use orbital_inspection_horizon_composition_bench::{HorizonDisposition, HorizonScope};
use orbital_passive_safety_terminal_bench::{
    BoundEnvelope, BoundState, PeriodWitness, RECEIPT_SCHEMA, TerminalCase, TerminalResult, evaluate,
};
use std::env;
use std::error::Error;
use std::fs;

struct ParsedCase {
    case: TerminalCase,
    expected: String,
}

fn parse_u64(value: &str, field: &str) -> Result<u64, Box<dyn Error>> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_i64(value: &str, field: &str) -> Result<i64, Box<dyn Error>> {
    value
        .parse::<i64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_scope(fields: &[&str], offset: usize) -> HorizonScope {
    HorizonScope {
        action_id: fields[offset].to_owned(),
        target_id: fields[offset + 1].to_owned(),
        keepout_profile_id: fields[offset + 2].to_owned(),
        keepout_geometry_revision: fields[offset + 3].to_owned(),
        transform_digest: fields[offset + 4].to_owned(),
        frame_id: fields[offset + 5].to_owned(),
        frame_revision: fields[offset + 6].to_owned(),
        time_basis_id: fields[offset + 7].to_owned(),
        dynamics_profile_revision: fields[offset + 8].to_owned(),
    }
}

fn parse_horizon_disposition(value: &str) -> Option<HorizonDisposition> {
    match value {
        "COMPLETE_CONDITIONAL_CLEAR" => Some(HorizonDisposition::CompleteConditionalClear),
        "NOT_PROVEN" => Some(HorizonDisposition::NotProven),
        "INVALID_INPUT" => Some(HorizonDisposition::InvalidInput),
        "INCOMPLETE_COVERAGE" => Some(HorizonDisposition::IncompleteCoverage),
        "SCOPE_MISMATCH" => Some(HorizonDisposition::ScopeMismatch),
        "BOUNDARY_MISMATCH" => Some(HorizonDisposition::BoundaryMismatch),
        "SEAM_INCOHERENT" => Some(HorizonDisposition::SeamIncoherent),
        "DUPLICATE_SEGMENT" => Some(HorizonDisposition::DuplicateSegment),
        "OVERFLOW" => Some(HorizonDisposition::Overflow),
        _ => None,
    }
}

fn parse_row(fields: &[&str], line_number: usize) -> Result<ParsedCase, Box<dyn Error>> {
    if fields.len() != 38 {
        return Err(format!(
            "line {line_number}: expected 38 comma-separated fields, found {}",
            fields.len()
        )
        .into());
    }

    let case_id = fields[0].to_owned();
    let expected = fields[1].to_owned();
    let scope = parse_scope(fields, 2);
    let terminal_epoch_tai_ns = parse_u64(fields[11], "terminal_epoch_tai_ns")?;
    let bounds = BoundEnvelope {
        delta_a_lower_scaled: parse_i64(fields[12], "delta_a_lower_scaled")?,
        delta_a_upper_scaled: parse_i64(fields[13], "delta_a_upper_scaled")?,
        y_lower_scaled: parse_i64(fields[14], "y_lower_scaled")?,
        y_upper_scaled: parse_i64(fields[15], "y_upper_scaled")?,
        rho_y_upper_scaled: parse_i64(fields[16], "rho_y_upper_scaled")?,
        provider_id: fields[17].to_owned(),
        provider_revision: fields[18].to_owned(),
        receipt_sha256: fields[19].to_owned(),
        state: BoundState::parse(fields[20])
            .ok_or_else(|| format!("line {line_number}: invalid bound source state"))?,
    };

    let period_fields = &fields[21..38];
    let period_witness = if period_fields.iter().all(|field| field.is_empty()) {
        None
    } else {
        let complete_time_coverage = match fields[25] {
            "true" => true,
            "false" => false,
            _ => return Err(format!("line {line_number}: invalid period complete flag").into()),
        };
        let disposition = parse_horizon_disposition(fields[26])
            .ok_or_else(|| format!("line {line_number}: invalid period disposition"))?;
        Some(PeriodWitness {
            horizon_id: fields[21].to_owned(),
            start_tai_ns: parse_u64(fields[22], "period_start_tai_ns")?,
            end_tai_ns: parse_u64(fields[23], "period_end_tai_ns")?,
            relative_period_ns: parse_u64(fields[24], "relative_period_ns")?,
            complete_time_coverage,
            disposition,
            segment_count: fields[27]
                .parse::<usize>()
                .map_err(|error| format!("invalid period segment_count: {error}"))?,
            receipt_sha256: fields[28].to_owned(),
            scope: parse_scope(fields, 29),
        })
    };

    Ok(ParsedCase {
        case: TerminalCase {
            case_id,
            scope,
            terminal_epoch_tai_ns,
            bounds,
            period_witness,
        },
        expected,
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

fn optional_number(value: Option<i128>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn render_result(result: &TerminalResult) -> String {
    let reasons = result
        .reason_codes
        .iter()
        .map(|reason| format!("\"{}\"", json_escape(reason)))
        .collect::<Vec<_>>()
        .join(",");
    let scope = &result.scope;
    let period_id = result
        .period_witness_id
        .as_deref()
        .map_or_else(|| "null".to_owned(), |value| format!("\"{}\"", json_escape(value)));
    let period_sha = result
        .period_witness_receipt_sha256
        .as_deref()
        .map_or_else(|| "null".to_owned(), |value| format!("\"{}\"", json_escape(value)));
    format!(
        "{{\"case_id\":\"{}\",\"scope\":{{\"action_id\":\"{}\",\"target_id\":\"{}\",\"keepout_profile_id\":\"{}\",\"keepout_geometry_revision\":\"{}\",\"transform_digest\":\"{}\",\"frame_id\":\"{}\",\"frame_revision\":\"{}\",\"time_basis_id\":\"{}\",\"dynamics_profile_revision\":\"{}\"}},\"terminal_epoch_tai_ns\":{},\"bounds_provider_id\":\"{}\",\"bounds_provider_revision\":\"{}\",\"bounds_receipt_sha256\":\"{}\",\"bounds_state\":\"{}\",\"period_witness_id\":{},\"period_witness_receipt_sha256\":{},\"disposition\":\"{}\",\"delta_a_branch\":\"{}\",\"margin_scaled\":{},\"reason_codes\":[{}],\"period_witness_consumed\":{}}}",
        json_escape(&result.case_id),
        json_escape(&scope.action_id),
        json_escape(&scope.target_id),
        json_escape(&scope.keepout_profile_id),
        json_escape(&scope.keepout_geometry_revision),
        json_escape(&scope.transform_digest),
        json_escape(&scope.frame_id),
        json_escape(&scope.frame_revision),
        json_escape(&scope.time_basis_id),
        json_escape(&scope.dynamics_profile_revision),
        result.terminal_epoch_tai_ns,
        json_escape(&result.bounds_provider_id),
        json_escape(&result.bounds_provider_revision),
        json_escape(&result.bounds_receipt_sha256),
        result.bounds_state.as_str(),
        period_id,
        period_sha,
        result.disposition.as_str(),
        json_escape(&result.delta_a_branch),
        optional_number(result.margin_scaled),
        reasons,
        result.period_witness_consumed,
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
        let result = evaluate(&parsed.case);
        if result.disposition.as_str() == parsed.expected {
            matches += 1;
        } else {
            return Err(format!(
                "{}: expected {}, received {}",
                parsed.case.case_id,
                parsed.expected,
                result.disposition.as_str()
            )
            .into());
        }
        results.push(result);
    }

    if results.is_empty() || matches == 0 {
        return Err("empty corpus or expected disposition mismatch".into());
    }

    let rendered = results
        .iter()
        .map(render_result)
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"schema\":\"{}\",\"source_commit\":\"{}\",\"corpus_sha256\":\"{}\",\"case_count\":{},\"expected_disposition_matches\":{},\"expected_disposition_mismatches\":{},\"claim_ready\":false,\"source_bytes_authenticated\":false,\"authority_granted\":false,\"physical_validation\":\"NOT_ASSESSED\",\"results\":[{}]}}",
        RECEIPT_SCHEMA,
        json_escape(&source_commit),
        json_escape(&corpus_sha256),
        results.len(),
        matches,
        rendered
    );
    Ok(())
}
