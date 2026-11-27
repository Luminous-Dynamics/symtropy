use runtime_pose_admission_bench::{
    AdmissionDisposition, CalibrationProfile, RECEIPT_SCHEMA, RuntimePosePrediction,
    evaluate_manifest,
};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;

struct ParsedCase {
    input: RuntimePosePrediction,
    expected: String,
}

fn parse_bool(value: &str, name: &str) -> Result<bool, Box<dyn Error>> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid {name}: expected true or false").into()),
    }
}

fn parse_u16(value: &str, name: &str) -> Result<u16, Box<dyn Error>> {
    value
        .parse::<u16>()
        .map_err(|error| format!("invalid {name}: {error}").into())
}

fn parse_u32(value: &str, name: &str) -> Result<u32, Box<dyn Error>> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {name}: {error}").into())
}

fn parse_u64(value: &str, name: &str) -> Result<u64, Box<dyn Error>> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {name}: {error}").into())
}

fn parse_f64(value: &str, name: &str) -> Result<f64, Box<dyn Error>> {
    value
        .parse::<f64>()
        .map_err(|error| format!("invalid {name}: {error}").into())
}

fn parse_f64_array<const N: usize>(
    fields: &[&str],
    offset: usize,
    name: &str,
) -> Result<[f64; N], Box<dyn Error>> {
    let mut values = [0.0_f64; N];
    for (index, value) in values.iter_mut().enumerate() {
        *value = parse_f64(fields[offset + index], &format!("{name}_{index}"))?;
    }
    Ok(values)
}

fn parse_row(fields: &[&str], line_number: usize) -> Result<ParsedCase, Box<dyn Error>> {
    if fields.len() != 103 {
        return Err(format!(
            "line {line_number}: expected 103 CSV fields, found {}",
            fields.len()
        )
        .into());
    }

    let profile = CalibrationProfile {
        profile_id: fields[29].to_owned(),
        profile_revision: fields[30].to_owned(),
        profile_sha256: fields[31].to_owned(),
        profile_state: fields[32].to_owned(),
        dataset_id: fields[33].to_owned(),
        dataset_revision: fields[34].to_owned(),
        model_revision: fields[35].to_owned(),
        model_sha256: fields[36].to_owned(),
        sensor_mode: fields[37].to_owned(),
        frame_convention: fields[38].to_owned(),
        camera_calibration_revision: fields[39].to_owned(),
        camera_calibration_sha256: fields[40].to_owned(),
        target_population_id: fields[41].to_owned(),
        target_population_sha256: fields[43].to_owned(),
        calibration_split_sha256: fields[45].to_owned(),
        heldout_split_sha256: fields[46].to_owned(),
        calibration_method: fields[47].to_owned(),
        nominal_level_bps: parse_u16(fields[48], "profile_nominal_level_bps")?,
        sample_count: parse_u64(fields[49], "profile_sample_count")?,
        coverage_lower_ci_ppm: parse_u32(fields[50], "profile_coverage_lower_ci_ppm")?,
        required_coverage_lower_ci_ppm: parse_u32(fields[51], "required_coverage_lower_ci_ppm")?,
        source_bytes_authenticated: parse_bool(fields[52], "profile_source_bytes_authenticated")?,
    };

    let mut covariance = [0.0_f64; 36];
    for (index, value) in covariance.iter_mut().enumerate() {
        *value = parse_f64(fields[67 + index], &format!("covariance_{index}"))?;
    }

    Ok(ParsedCase {
        expected: fields[1].to_owned(),
        input: RuntimePosePrediction {
            case_id: fields[0].to_owned(),
            dataset_id: fields[2].to_owned(),
            dataset_revision: fields[3].to_owned(),
            dataset_sha256: fields[4].to_owned(),
            sample_id: fields[5].to_owned(),
            frame_id: fields[6].to_owned(),
            target_id: fields[7].to_owned(),
            trajectory_id: fields[8].to_owned(),
            sensor_mode: fields[9].to_owned(),
            frame_convention: fields[10].to_owned(),
            quaternion_order: fields[11].to_owned(),
            tangent_convention: fields[12].to_owned(),
            translation_unit: fields[13].to_owned(),
            rotation_unit: fields[14].to_owned(),
            model_revision: fields[15].to_owned(),
            model_sha256: fields[16].to_owned(),
            model_config_sha256: fields[17].to_owned(),
            camera_calibration_revision: fields[18].to_owned(),
            camera_calibration_sha256: fields[19].to_owned(),
            frame_bundle_revision: fields[20].to_owned(),
            frame_bundle_sha256: fields[21].to_owned(),
            frame_bundle_state: fields[22].to_owned(),
            frame_source_bytes_authenticated: parse_bool(
                fields[23],
                "frame_source_bytes_authenticated",
            )?,
            bundle_sample_id: fields[24].to_owned(),
            bundle_frame_id: fields[25].to_owned(),
            bundle_target_id: fields[26].to_owned(),
            bundle_dataset_revision: fields[27].to_owned(),
            bundle_camera_calibration_revision: fields[28].to_owned(),
            target_population_id: fields[42].to_owned(),
            target_population_sha256: fields[44].to_owned(),
            nominal_level_bps: parse_u16(fields[59], "nominal_level_bps")?,
            support_mode: fields[53].to_owned(),
            visible_fraction: parse_f64(fields[54], "visible_fraction")?,
            correspondence_count: parse_u32(fields[55], "correspondence_count")?,
            geometry_condition: parse_f64(fields[56], "geometry_condition")?,
            ood_flag: parse_bool(fields[57], "ood_flag")?,
            symmetry_ambiguous: parse_bool(fields[58], "symmetry_ambiguous")?,
            predicted_translation_m: parse_f64_array(fields, 60, "predicted_translation")?,
            predicted_quaternion_xyzw: parse_f64_array(fields, 63, "predicted_quaternion")?,
            covariance,
            profile,
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

fn render_numbers(values: &[f64]) -> String {
    values
        .iter()
        .map(|value| {
            let rendered = if value.is_nan() {
                "nan".to_owned()
            } else if *value == f64::INFINITY {
                "inf".to_owned()
            } else if *value == f64::NEG_INFINITY {
                "-inf".to_owned()
            } else {
                format!("{value:.12}")
            };
            format!("\"{rendered}\"")
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn render_result(result: &runtime_pose_admission_bench::AdmissionResult) -> String {
    let reasons = result
        .reason_codes
        .iter()
        .map(|reason| format!("\"{}\"", json_escape(reason)))
        .collect::<Vec<_>>()
        .join(",");
    let covariance = render_numbers(&result.covariance_row_major);
    format!(
        "{{\"case_id\":\"{}\",\"sample_id\":\"{}\",\"frame_id\":\"{}\",\"target_id\":\"{}\",\"trajectory_id\":\"{}\",\"dataset_id\":\"{}\",\"dataset_revision\":\"{}\",\"dataset_sha256\":\"{}\",\"model_revision\":\"{}\",\"model_sha256\":\"{}\",\"model_config_sha256\":\"{}\",\"camera_calibration_revision\":\"{}\",\"camera_calibration_sha256\":\"{}\",\"frame_bundle_revision\":\"{}\",\"frame_bundle_sha256\":\"{}\",\"frame_source_bytes_authenticated\":{},\"profile_id\":\"{}\",\"profile_revision\":\"{}\",\"profile_sha256\":\"{}\",\"profile_source_bytes_authenticated\":{},\"sensor_mode\":\"{}\",\"frame_convention\":\"{}\",\"tangent_convention\":\"{}\",\"covariance_units\":\"{}\",\"nominal_level_bps\":{},\"profile_sample_count\":{},\"profile_coverage_lower_ci_ppm\":{},\"predicted_translation_m\":[{}],\"predicted_quaternion_xyzw\":[{}],\"covariance_row_major\":[{}],\"disposition\":\"{}\",\"reason_codes\":[{}],\"candidate_only\":{},\"statistical_region_only\":{},\"deterministic_safety_bound\":{},\"physical_calibration_validated\":{},\"source_bytes_authenticated\":{},\"maneuver_authority_granted\":{}}}",
        json_escape(&result.case_id),
        json_escape(&result.sample_id),
        json_escape(&result.frame_id),
        json_escape(&result.target_id),
        json_escape(&result.trajectory_id),
        json_escape(&result.dataset_id),
        json_escape(&result.dataset_revision),
        json_escape(&result.dataset_sha256),
        json_escape(&result.model_revision),
        json_escape(&result.model_sha256),
        json_escape(&result.model_config_sha256),
        json_escape(&result.camera_calibration_revision),
        json_escape(&result.camera_calibration_sha256),
        json_escape(&result.frame_bundle_revision),
        json_escape(&result.frame_bundle_sha256),
        result.frame_source_bytes_authenticated,
        json_escape(&result.profile_id),
        json_escape(&result.profile_revision),
        json_escape(&result.profile_sha256),
        result.profile_source_bytes_authenticated,
        result.upstream_source_bytes_authenticated,
        result.source_authentication_verified_here,
        json_escape(&result.sensor_mode),
        json_escape(&result.frame_convention),
        json_escape(&result.tangent_convention),
        json_escape(&result.covariance_units),
        result.nominal_level_bps,
        result.profile_sample_count,
        result.profile_coverage_lower_ci_ppm,
        render_numbers(&result.predicted_translation_m),
        render_numbers(&result.predicted_quaternion_xyzw),
        covariance,
        result.disposition.as_str(),
        reasons,
        result.candidate_only,
        result.statistical_region_only,
        result.deterministic_safety_bound,
        result.physical_calibration_validated,
        result.source_bytes_authenticated,
        result.maneuver_authority_granted,
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
    let mut expected = std::collections::BTreeMap::new();
    let mut inputs = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case_id,") {
            continue;
        }
        let parsed = parse_row(&line.split(',').collect::<Vec<_>>(), index + 1)?;
        if !case_ids.insert(parsed.input.case_id.clone()) {
            return Err(format!("duplicate case ID: {}", parsed.input.case_id).into());
        }
        expected.insert(parsed.input.case_id.clone(), parsed.expected);
        inputs.push(parsed.input);
    }
    if inputs.is_empty() {
        return Err("empty runtime pose admission corpus".into());
    }

    let results = evaluate_manifest(&inputs);
    for result in &results {
        let disposition = expected
            .get(&result.case_id)
            .ok_or_else(|| format!("missing expected disposition for {}", result.case_id))?;
        if result.disposition.as_str() != disposition {
            return Err(format!(
                "{}: expected {}, received {}",
                result.case_id,
                disposition,
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
    let candidate_count = results
        .iter()
        .filter(|result| result.disposition == AdmissionDisposition::CandidateStatisticalRegion)
        .count();
    println!(
        "{{\"schema\":\"{}\",\"source_commit\":\"{}\",\"corpus_sha256\":\"{}\",\"covariance_order\":\"{}\",\"covariance_units\":\"{}\",\"case_count\":{},\"expected_disposition_matches\":{},\"expected_disposition_mismatches\":0,\"candidate_statistical_region_count\":{},\"claim_ready\":false,\"source_bytes_authenticated\":false,\"physical_calibration_validated\":false,\"deterministic_safety_bound\":false,\"maneuver_authority_granted\":false,\"contains_ground_truth_fields\":false,\"contains_offline_evaluation_labels\":false,\"results\":[{}]}}",
        RECEIPT_SCHEMA,
        json_escape(&source_commit),
        json_escape(&corpus_sha256),
        runtime_pose_admission_bench::TANGENT_CONVENTION,
        runtime_pose_admission_bench::COVARIANCE_UNITS,
        results.len(),
        results.len(),
        candidate_count,
        rendered
    );
    Ok(())
}
