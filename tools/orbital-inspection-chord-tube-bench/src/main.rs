use orbital_inspection_chord_tube_bench::{CaseInput, CaseOutput, RECEIPT_SCHEMA, verify};
use std::env;
use std::error::Error;
use std::fs;

fn parse_i64(value: &str, field: &str) -> Result<i64, Box<dyn Error>> {
    value
        .parse::<i64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_row(line: &str, line_number: usize) -> Result<CaseInput, Box<dyn Error>> {
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() != 11 {
        return Err(format!("line {line_number}: expected 11 comma-separated fields").into());
    }
    let case_id = fields[0].to_owned();
    let start = [
        parse_i64(fields[1], "start_x")?,
        parse_i64(fields[2], "start_y")?,
        parse_i64(fields[3], "start_z")?,
    ];
    let end = [
        parse_i64(fields[4], "end_x")?,
        parse_i64(fields[5], "end_y")?,
        parse_i64(fields[6], "end_z")?,
    ];
    let acceleration_bound_scaled_per_s2 =
        parse_i64(fields[7], "acceleration_bound_scaled_per_s2")?;
    let duration_ns = fields[8]
        .parse::<u64>()
        .map_err(|error| format!("invalid duration_ns: {error}"))?;
    let endpoint_error_bound_scaled = parse_i64(fields[9], "endpoint_error_bound_scaled")?;

    Ok(CaseInput {
        case_id,
        start,
        end,
        acceleration_bound_scaled_per_s2,
        duration_ns,
        endpoint_error_bound_scaled,
    })
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn optional_number(value: Option<i128>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn render_result(result: &CaseOutput) -> String {
    format!(
        "{{\"case_id\":\"{}\",\"disposition\":\"{}\",\"curvature_error_scaled\":{},\"total_tube_error_scaled\":{}}}",
        json_escape(&result.case_id),
        result.disposition.as_str(),
        optional_number(result.curvature_error_scaled),
        optional_number(result.total_tube_error_scaled),
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
    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case_id,") {
            continue;
        }
        let input = parse_row(line, index + 1)?;
        results.push(verify(&input));
    }

    let rendered = results
        .iter()
        .map(render_result)
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"schema\":\"{}\",\"source_commit\":\"{}\",\"corpus_sha256\":\"{}\",\"case_count\":{},\"claim_ready\":false,\"source_bytes_authenticated\":false,\"authority_granted\":false,\"physical_validation\":\"NOT_ASSESSED\",\"results\":[{}]}}",
        RECEIPT_SCHEMA,
        json_escape(&source_commit),
        json_escape(&corpus_sha256),
        results.len(),
        rendered,
    );
    Ok(())
}
