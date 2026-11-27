use spacecraft_multimodal_frame_sync_bench::{
    BundleDisposition, BundleInput, BundleResult, ModalityRecord, RECEIPT_SCHEMA,
    SPACESENSE_REVISION, evaluate_manifest,
};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;

struct ParsedCase {
    input: BundleInput,
    expected: String,
}

fn parse_bool(value: &str, field: &str) -> Result<bool, Box<dyn Error>> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid {field}: expected true or false").into()),
    }
}

fn parse_modality(
    fields: &[&str],
    offset: usize,
    name: &str,
) -> Result<ModalityRecord, Box<dyn Error>> {
    Ok(ModalityRecord {
        timestamp: fields[offset].to_owned(),
        frame_id: fields[offset + 1].to_owned(),
        sensor_id: fields[offset + 2].to_owned(),
        path: fields[offset + 3].to_owned(),
        sha256: fields[offset + 4].to_owned(),
        present: parse_bool(fields[offset + 5], name)?,
    })
}

fn parse_row(fields: &[&str], line_number: usize) -> Result<ParsedCase, Box<dyn Error>> {
    if fields.len() != 57 {
        return Err(format!(
            "line {line_number}: expected 57 comma-separated fields, found {}",
            fields.len()
        )
        .into());
    }

    Ok(ParsedCase {
        expected: fields[1].to_owned(),
        input: BundleInput {
            case_id: fields[0].to_owned(),
            dataset_id: fields[2].to_owned(),
            dataset_revision: fields[3].to_owned(),
            license: fields[4].to_owned(),
            intended_use: fields[5].to_owned(),
            spacecraft_id: fields[6].to_owned(),
            trajectory_id: fields[7].to_owned(),
            sample_id: fields[8].to_owned(),
            frame_id: fields[9].to_owned(),
            split: fields[10].to_owned(),
            timestamp_format: fields[11].to_owned(),
            capture_timestamp: fields[12].to_owned(),
            corrected_pose_revision: fields[13].to_owned(),
            pose_csv_revision: fields[14].to_owned(),
            camera_frame: fields[15].to_owned(),
            world_frame: fields[16].to_owned(),
            lidar_coordinate_frame: fields[17].to_owned(),
            pose_semantics: fields[18].to_owned(),
            quaternion_convention: fields[19].to_owned(),
            pose_length_unit: fields[20].to_owned(),
            depth_unit: fields[21].to_owned(),
            lidar_unit: fields[22].to_owned(),
            camera_calibration_revision: fields[23].to_owned(),
            camera_calibration_sha256: fields[24].to_owned(),
            lidar_to_camera_revision: fields[25].to_owned(),
            lidar_to_camera_sha256: fields[26].to_owned(),
            rgb: parse_modality(fields, 27, "rgb_present")?,
            depth: parse_modality(fields, 33, "depth_present")?,
            segmentation: parse_modality(fields, 39, "segmentation_present")?,
            lidar: parse_modality(fields, 45, "lidar_present")?,
            pose: parse_modality(fields, 51, "pose_present")?,
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

fn render_map(map: &std::collections::BTreeMap<String, String>) -> String {
    map.iter()
        .map(|(key, value)| format!("\"{}\":\"{}\",", json_escape(key), json_escape(value)))
        .collect::<Vec<_>>()
        .join(",")
}

fn render_result(result: &BundleResult) -> String {
    let reasons = result
        .reason_codes
        .iter()
        .map(|reason| format!("\"{}\"", json_escape(reason)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"case_id\":\"{}\",\"dataset_id\":\"{}\",\"dataset_revision\":\"{}\",\"license\":\"{}\",\"spacecraft_id\":\"{}\",\"trajectory_id\":\"{}\",\"sample_id\":\"{}\",\"frame_id\":\"{}\",\"split\":\"{}\",\"capture_timestamp\":\"{}\",\"pose_csv_revision\":\"{}\",\"corrected_pose_revision\":\"{}\",\"camera_calibration_revision\":\"{}\",\"camera_calibration_sha256\":\"{}\",\"lidar_to_camera_revision\":\"{}\",\"lidar_to_camera_sha256\":\"{}\",\"lidar_coordinate_frame\":\"{}\",\"pose_semantics\":\"{}\",\"disposition\":\"{}\",\"modality_sha256\":{{{}}},\"modality_paths\":{{{}}},\"reason_codes\":[{}],\"bundle_admissible\":{},\"source_bytes_authenticated\":{},\"calibration_physically_validated\":{},\"authority_granted\":{}}}",
        json_escape(&result.case_id),
        json_escape(&result.dataset_id),
        json_escape(&result.dataset_revision),
        json_escape(&result.license),
        json_escape(&result.spacecraft_id),
        json_escape(&result.trajectory_id),
        json_escape(&result.sample_id),
        json_escape(&result.frame_id),
        json_escape(&result.split),
        json_escape(&result.capture_timestamp),
        json_escape(&result.pose_csv_revision),
        json_escape(&result.corrected_pose_revision),
        json_escape(&result.camera_calibration_revision),
        json_escape(&result.camera_calibration_sha256),
        json_escape(&result.lidar_to_camera_revision),
        json_escape(&result.lidar_to_camera_sha256),
        json_escape(&result.lidar_coordinate_frame),
        json_escape(&result.pose_semantics),
        result.disposition.as_str(),
        render_map(&result.modality_sha256),
        render_map(&result.modality_paths),
        reasons,
        result.bundle_admissible,
        result.source_bytes_authenticated,
        result.calibration_physically_validated,
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

    let mut parsed = Vec::new();
    let mut case_ids = BTreeSet::new();
    let mut expected = std::collections::BTreeMap::new();
    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case_id,") {
            continue;
        }
        let item = parse_row(&line.split(',').collect::<Vec<_>>(), index + 1)?;
        if !case_ids.insert(item.input.case_id.clone()) {
            return Err(format!("duplicate case ID: {}", item.input.case_id).into());
        }
        if expected
            .insert(item.input.case_id.clone(), item.expected)
            .is_some()
        {
            return Err(format!("duplicate expected row: {}", item.input.case_id).into());
        }
        parsed.push(item.input);
    }

    if parsed.is_empty() {
        return Err("empty multimodal bundle corpus".into());
    }

    let mut results = evaluate_manifest(&parsed);
    results.sort_by_key(|result| result.sample_id.clone());

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
    let synchronized_count = results
        .iter()
        .filter(|result| result.disposition == BundleDisposition::Synchronized)
        .count();
    println!(
        "{{\"schema\":\"{}\",\"source_commit\":\"{}\",\"corpus_sha256\":\"{}\",\"dataset_revision\":\"{}\",\"pose_correction_date\":\"2026-05-19\",\"case_count\":{},\"expected_disposition_matches\":{},\"expected_disposition_mismatches\":0,\"synchronized_count\":{},\"claim_ready\":false,\"source_bytes_authenticated\":false,\"calibration_physically_validated\":false,\"authority_granted\":false,\"results\":[{}]}}",
        RECEIPT_SCHEMA,
        json_escape(&source_commit),
        json_escape(&corpus_sha256),
        SPACESENSE_REVISION,
        results.len(),
        results.len(),
        synchronized_count,
        rendered
    );
    Ok(())
}
