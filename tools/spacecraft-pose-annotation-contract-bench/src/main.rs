use spacecraft_pose_annotation_contract_bench::{
    AnnotationDisposition, PoseInput, PoseResult, RECEIPT_SCHEMA, evaluate_manifest,
};
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;

struct ParsedCase {
    input: PoseInput,
    expected: String,
}

fn parse_u32(value: &str, field: &str) -> Result<u32, Box<dyn Error>> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_u16(value: &str, field: &str) -> Result<u16, Box<dyn Error>> {
    value
        .parse::<u16>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_f64(value: &str, field: &str) -> Result<f64, Box<dyn Error>> {
    value
        .parse::<f64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_bool(value: &str, field: &str) -> Result<bool, Box<dyn Error>> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid {field}: expected true or false").into()),
    }
}

fn parse_row(fields: &[&str], line_number: usize) -> Result<ParsedCase, Box<dyn Error>> {
    if fields.len() != 32 {
        return Err(format!(
            "line {line_number}: expected 32 comma-separated fields, found {}",
            fields.len()
        )
        .into());
    }

    Ok(ParsedCase {
        expected: fields[1].to_owned(),
        input: PoseInput {
            case_id: fields[0].to_owned(),
            sample_id: fields[2].to_owned(),
            split: fields[3].to_owned(),
            annotation_release: fields[4].to_owned(),
            bbox3d_present: parse_bool(fields[5], "bbox3d_present")?,
            keypoint_id: fields[6].to_owned(),
            keypoint_count: parse_u16(fields[7], "keypoint_count")?,
            pose_semantics: fields[8].to_owned(),
            quaternion_order: fields[9].to_owned(),
            camera_axis_convention: fields[10].to_owned(),
            projection_axis_convention: fields[11].to_owned(),
            length_unit: fields[12].to_owned(),
            image_width: parse_u32(fields[13], "image_width")?,
            image_height: parse_u32(fields[14], "image_height")?,
            fx: parse_f64(fields[15], "fx")?,
            fy: parse_f64(fields[16], "fy")?,
            cx: parse_f64(fields[17], "cx")?,
            cy: parse_f64(fields[18], "cy")?,
            quaternion: [
                parse_f64(fields[19], "qx")?,
                parse_f64(fields[20], "qy")?,
                parse_f64(fields[21], "qz")?,
                parse_f64(fields[22], "qw")?,
            ],
            camera_translation: [
                parse_f64(fields[23], "tx")?,
                parse_f64(fields[24], "ty")?,
                parse_f64(fields[25], "tz")?,
            ],
            keypoint_world: [
                parse_f64(fields[26], "kpx")?,
                parse_f64(fields[27], "kpy")?,
                parse_f64(fields[28], "kpz")?,
            ],
            observed_pixel: [parse_f64(fields[29], "u")?, parse_f64(fields[30], "v")?],
            reprojection_tolerance_px: parse_f64(fields[31], "tolerance_px")?,
        },
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

fn optional_i64(value: Option<i64>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn render_result(result: &PoseResult) -> String {
    let reasons = result
        .reason_codes
        .iter()
        .map(|reason| format!("\"{}\"", json_escape(reason)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"case_id\":\"{}\",\"sample_id\":\"{}\",\"split\":\"{}\",\"disposition\":\"{}\",\"keypoint_id\":\"{}\",\"quaternion_norm_error_scaled\":{},\"projected_u_micro_px\":{},\"projected_v_micro_px\":{},\"reprojection_error_micro_px\":{},\"reason_codes\":[{}],\"pose_contract_version\":\"{}\",\"source_annotation_release\":\"{}\",\"source_bytes_authenticated\":{},\"pose_ground_truth_physically_validated\":{}}}",
        json_escape(&result.case_id),
        json_escape(&result.sample_id),
        json_escape(&result.split),
        result.disposition.as_str(),
        json_escape(&result.keypoint_id),
        optional_i64(result.quaternion_norm_error_scaled),
        optional_i64(result.projected_u_micro_px),
        optional_i64(result.projected_v_micro_px),
        optional_u64(result.reprojection_error_micro_px),
        reasons,
        json_escape(&result.pose_contract_version),
        json_escape(&result.source_annotation_release),
        result.source_bytes_authenticated,
        result.pose_ground_truth_physically_validated,
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

    let mut parsed = Vec::new();
    let mut expected = BTreeMap::new();
    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case_id,") {
            continue;
        }
        let item = parse_row(&line.split(',').collect::<Vec<_>>(), index + 1)?;
        if expected
            .insert(item.input.case_id.clone(), item.expected.clone())
            .is_some()
        {
            return Err(format!("duplicate case ID: {}", item.input.case_id).into());
        }
        parsed.push(item.input);
    }

    if parsed.is_empty() {
        return Err("empty pose annotation corpus".into());
    }

    let results = evaluate_manifest(&parsed);
    for result in &results {
        let expected_disposition = expected
            .get(&result.case_id)
            .ok_or_else(|| format!("missing expected disposition for {}", result.case_id))?;
        if result.disposition.as_str() != expected_disposition {
            return Err(format!(
                "{}: expected {}, received {}",
                result.case_id,
                expected_disposition,
                result.disposition.as_str()
            )
            .into());
        }
    }

    let rendered = results
        .iter()
        .map(render_result)
        .collect::<Vec<_>>()
        .join(",");
    let consistent_count = results
        .iter()
        .filter(|result| result.disposition == AnnotationDisposition::GeometricallyConsistent)
        .count();
    println!(
        "{{\"schema\":\"{}\",\"source_commit\":\"{}\",\"corpus_sha256\":\"{}\",\"case_count\":{},\"expected_disposition_matches\":{},\"expected_disposition_mismatches\":0,\"geometrically_consistent_count\":{},\"claim_ready\":false,\"source_bytes_authenticated\":false,\"pose_ground_truth_physically_validated\":false,\"authority_granted\":false,\"results\":[{}]}}",
        RECEIPT_SCHEMA,
        json_escape(&source_commit),
        json_escape(&corpus_sha256),
        results.len(),
        results.len(),
        consistent_count,
        rendered
    );
    Ok(())
}
