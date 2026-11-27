use orbital_inspection_chord_tube_bench::CaseInput;
use orbital_inspection_horizon_composition_bench::{
    BoundaryKind, Horizon, HorizonScope, RECEIPT_SCHEMA, Segment, evaluate,
};
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;

struct ParsedRow {
    horizon_id: String,
    expected: String,
    horizon_start_ns: u64,
    horizon_end_ns: u64,
    segment: Segment,
}

struct Group {
    horizon_start_ns: u64,
    horizon_end_ns: u64,
    expected: String,
    scope: Option<HorizonScope>,
    segments: Vec<Segment>,
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

fn parse_row(
    fields: &[&str],
    line_number: usize,
) -> Result<ParsedRow, Box<dyn Error>> {
    if fields.len() != 28 {
        return Err(format!(
            "line {line_number}: expected 28 comma-separated fields, found {}",
            fields.len()
        )
        .into());
    }

    let horizon_id = fields[0].to_owned();
    let expected = fields[1].to_owned();
    let horizon_start_ns = parse_u64(fields[2], "horizon_start_ns")?;
    let horizon_end_ns = parse_u64(fields[3], "horizon_end_ns")?;

    let scope = HorizonScope {
        action_id: fields[5].to_owned(),
        target_id: fields[6].to_owned(),
        keepout_profile_id: fields[7].to_owned(),
        keepout_geometry_revision: fields[8].to_owned(),
        transform_digest: fields[9].to_owned(),
        frame_id: fields[10].to_owned(),
        frame_revision: fields[11].to_owned(),
        time_basis_id: fields[12].to_owned(),
        dynamics_profile_revision: fields[13].to_owned(),
    };

    let start_tai_ns = parse_u64(fields[14], "segment_start_tai_ns")?;
    let end_tai_ns = parse_u64(fields[15], "segment_end_tai_ns")?;
    let start_boundary = BoundaryKind::parse(fields[18])
        .ok_or_else(|| format!("line {line_number}: unknown boundary kind"))?;

    let motion = CaseInput {
        case_id: fields[4].to_owned(),
        start: [
            parse_i64(fields[19], "q0x")?,
            parse_i64(fields[20], "q0y")?,
            parse_i64(fields[21], "q0z")?,
        ],
        end: [
            parse_i64(fields[22], "q1x")?,
            parse_i64(fields[23], "q1y")?,
            parse_i64(fields[24], "q1z")?,
        ],
        acceleration_bound_scaled_per_s2: parse_i64(
            fields[25],
            "acceleration_bound_scaled_per_s2",
        )?,
        duration_ns: parse_u64(fields[26], "duration_ns")?,
        endpoint_error_bound_scaled: parse_i64(fields[27], "endpoint_error_bound_scaled")?,
    };

    let segment = Segment {
        segment_id: fields[4].to_owned(),
        scope,
        start_tai_ns,
        end_tai_ns,
        start_event_id: fields[16].to_owned(),
        end_event_id: fields[17].to_owned(),
        start_boundary,
        motion,
    };

    Ok(ParsedRow {
        horizon_id,
        expected,
        horizon_start_ns,
        horizon_end_ns,
        segment,
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

fn render_result(result: &orbital_inspection_horizon_composition_bench::HorizonResult) -> String {
    let segments = result
        .segment_results
        .iter()
        .map(|segment| {
            format!(
                "{{\"segment_id\":\"{}\",\"disposition\":\"{}\"}}",
                json_escape(&segment.segment_id),
                json_escape(&segment.disposition)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"horizon_id\":\"{}\",\"disposition\":\"{}\",\"complete_time_coverage\":{},\"segment_count\":{},\"seam_count\":{},\"segments\":[{}]}}",
        json_escape(&result.horizon_id),
        result.disposition.as_str(),
        result.complete_time_coverage,
        result.segment_count,
        result.seam_count,
        segments
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
    let mut groups: BTreeMap<String, Group> = BTreeMap::new();

    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("horizon_id,") {
            continue;
        }
        let fields = line.split(',').collect::<Vec<_>>();
        let parsed = parse_row(&fields, index + 1)?;
        let ParsedRow {
            horizon_id,
            expected,
            horizon_start_ns,
            horizon_end_ns,
            segment,
        } = parsed;

        let group = groups.entry(horizon_id).or_insert_with(|| Group {
            horizon_start_ns,
            horizon_end_ns,
            expected: expected.clone(),
            scope: Some(segment.scope.clone()),
            segments: Vec::new(),
        });

        if group.horizon_start_ns != horizon_start_ns
            || group.horizon_end_ns != horizon_end_ns
            || group.expected != expected
        {
            return Err(format!(
                "line {}: inconsistent horizon header within case",
                index + 1
            )
            .into());
        }
        group.segments.push(segment);
    }

    if groups.is_empty() {
        return Err("empty horizon corpus".into());
    }

    let mut results = Vec::new();
    for (horizon_id, group) in groups {
        let scope = group.scope.ok_or("missing first segment scope")?;
        let horizon = Horizon {
            horizon_id,
            scope,
            start_tai_ns: group.horizon_start_ns,
            end_tai_ns: group.horizon_end_ns,
            segments: group.segments,
        };
        let result = evaluate(&horizon);
        if result.disposition.as_str() != group.expected {
            return Err(format!(
                "{}: expected {}, received {}",
                horizon.horizon_id,
                group.expected,
                result.disposition.as_str()
            )
            .into());
        }
        results.push(result);
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
        rendered
    );
    Ok(())
}
