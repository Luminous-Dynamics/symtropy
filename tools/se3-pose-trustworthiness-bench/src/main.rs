use se3_pose_trustworthiness_bench::{
    ComputeState, EvaluationDisposition, EvaluationSummary, PoseDecision, PoseInput, PoseResult,
    RECEIPT_SCHEMA, evaluate_manifest, summarize,
};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;

struct ParsedCase {
    input: PoseInput,
    expected: String,
}

fn parse_f64(value: &str, name: &str) -> Result<f64, Box<dyn Error>> {
    value.parse::<f64>().map_err(|error| format!("invalid {name}: {error}").into())
}

fn parse_u16(value: &str, name: &str) -> Result<u16, Box<dyn Error>> {
    value.parse::<u16>().map_err(|error| format!("invalid {name}: {error}").into())
}

fn parse_u32(value: &str, name: &str) -> Result<u32, Box<dyn Error>> {
    value.parse::<u32>().map_err(|error| format!("invalid {name}: {error}").into())
}

fn parse_u64(value: &str, name: &str) -> Result<u64, Box<dyn Error>> {
    value.parse::<u64>().map_err(|error| format!("invalid {name}: {error}").into())
}

fn parse_bool(value: &str, name: &str) -> Result<bool, Box<dyn Error>> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid {name}: expected true or false").into()),
    }
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
    if fields.len() != 80 {
        return Err(format!(
            "line {line_number}: expected 80 CSV fields, found {}",
            fields.len()
        )
        .into());
    }
    let mut covariance = [0.0_f64; 36];
    for (index, value) in covariance.iter_mut().enumerate() {
        *value = parse_f64(fields[44 + index], &format!("covariance_{index}"))?;
    }

    let decision = PoseDecision::parse(fields[18]).ok_or("decision must be ACCEPT or ABSTAIN")?;
    let compute_state =
        ComputeState::parse(fields[27]).ok_or("compute_state must be MEASURED or NOT_MEASURED")?;

    Ok(ParsedCase {
        expected: fields[1].to_owned(),
        input: PoseInput {
            case_id: fields[0].to_owned(),
            sample_id: fields[2].to_owned(),
            target_id: fields[3].to_owned(),
            split: fields[4].to_owned(),
            dataset_id: fields[5].to_owned(),
            dataset_revision: fields[6].to_owned(),
            dataset_sha256: fields[7].to_owned(),
            model_revision: fields[8].to_owned(),
            model_sha256: fields[9].to_owned(),
            calibration_revision: fields[10].to_owned(),
            calibration_sha256: fields[11].to_owned(),
            sensor_mode: fields[12].to_owned(),
            frame_convention: fields[13].to_owned(),
            quaternion_order: fields[14].to_owned(),
            tangent_convention: fields[15].to_owned(),
            translation_unit: fields[16].to_owned(),
            rotation_unit: fields[17].to_owned(),
            decision,
            abstain_reason: fields[19].to_owned(),
            admission_confidence_bps: parse_u16(fields[20], "admission_confidence_bps")?,
            confidence_score_ppm: parse_u32(fields[21], "confidence_score_ppm")?,
            visible_fraction: parse_f64(fields[22], "visible_fraction")?,
            correspondence_count: parse_u32(fields[23], "correspondence_count")?,
            geometry_condition: parse_f64(fields[24], "geometry_condition")?,
            ood_flag: parse_bool(fields[25], "ood_flag")?,
            symmetry_ambiguous: parse_bool(fields[26], "symmetry_ambiguous")?,
            compute_state,
            latency_us: parse_u64(fields[28], "latency_us")?,
            energy_uj: parse_u64(fields[29], "energy_uj")?,
            predicted_translation_m: parse_f64_array(fields, 30, "predicted_translation")?,
            predicted_quaternion_xyzw: parse_f64_array(fields, 33, "predicted_quaternion")?,
            ground_truth_translation_m: parse_f64_array(fields, 37, "ground_truth_translation")?,
            ground_truth_quaternion_xyzw: parse_f64_array(fields, 40, "ground_truth_quaternion")?,
            covariance,
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

fn optional_decimal(value: Option<f64>) -> String {
    value.map_or_else(
        || "null".to_owned(),
        |number| format!("\"{number:.12}\""),
    )
}

fn optional_bool(value: Option<bool>) -> String {
    value.map_or_else(
        || "null".to_owned(),
        |flag| flag.to_string(),
    )
}

fn render_result(result: &PoseResult) -> String {
    let reasons = result
        .reason_codes
        .iter()
        .map(|reason| format!("\"{}\"", json_escape(reason)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"case_id\":\"{}\",\"sample_id\":\"{}\",\"target_id\":\"{}\",\"split\":\"{}\",\"dataset_id\":\"{}\",\"dataset_revision\":\"{}\",\"dataset_sha256\":\"{}\",\"model_revision\":\"{}\",\"model_sha256\":\"{}\",\"calibration_revision\":\"{}\",\"calibration_sha256\":\"{}\",\"sensor_mode\":\"{}\",\"frame_convention\":\"{}\",\"tangent_convention\":\"{}\",\"decision\":\"{}\",\"decision_reason\":\"{}\",\"disposition\":\"{}\",\"translation_error_m\":{},\"rotation_error_rad\":{},\"mahalanobis_squared\":{},\"covariance_log_determinant\":{},\"prediction_region_log_volume_95\":{},\"coverage_50\":{},\"coverage_80\":{},\"coverage_90\":{},\"coverage_95\":{},\"coverage_99\":{},\"confidence_score_ppm\":{},\"accepted_by_model\":{},\"observability_policy_violation\":{},\"false_confident_95\":{},\"benchmark_admissible\":{},\"visible_fraction\":\"{:.12}\",\"correspondence_count\":{},\"geometry_condition\":\"{:.12}\",\"ood_flag\":{},\"symmetry_ambiguous\":{},\"latency_us\":{},\"energy_uj\":{},\"reason_codes\":[{}],\"source_bytes_authenticated\":{},\"physical_calibration_validated\":{},\"maneuver_authority_granted\":{}}}",
        json_escape(&result.case_id),
        json_escape(&result.sample_id),
        json_escape(&result.target_id),
        json_escape(&result.split),
        json_escape(&result.dataset_id),
        json_escape(&result.dataset_revision),
        json_escape(&result.dataset_sha256),
        json_escape(&result.model_revision),
        json_escape(&result.model_sha256),
        json_escape(&result.calibration_revision),
        json_escape(&result.calibration_sha256),
        json_escape(&result.sensor_mode),
        json_escape(&result.frame_convention),
        json_escape(&result.tangent_convention),
        result.decision.as_str(),
        json_escape(&result.decision_reason),
        result.disposition.as_str(),
        optional_decimal(result.translation_error_m),
        optional_decimal(result.rotation_error_rad),
        optional_decimal(result.mahalanobis_squared),
        optional_decimal(result.covariance_log_determinant),
        optional_decimal(result.prediction_region_log_volume_95),
        optional_bool(result.coverage_50),
        optional_bool(result.coverage_80),
        optional_bool(result.coverage_90),
        optional_bool(result.coverage_95),
        optional_bool(result.coverage_99),
        result.confidence_score_ppm,
        result.accepted_by_model,
        result.observability_policy_violation,
        result.false_confident_95,
        result.benchmark_admissible,
        result.visible_fraction,
        result.correspondence_count,
        result.geometry_condition,
        result.ood_flag,
        result.symmetry_ambiguous,
        result.latency_us.map_or_else(|| "null".to_owned(), |value| value.to_string()),
        result.energy_uj.map_or_else(|| "null".to_owned(), |value| value.to_string()),
        reasons,
        result.source_bytes_authenticated,
        result.physical_calibration_validated,
        result.maneuver_authority_granted,
    )
}

fn render_summary(summary: &EvaluationSummary) -> String {
    let risk = summary
        .risk_coverage
        .iter()
        .map(|point| {
            format!(
                "{{\"confidence_threshold_ppm\":{},\"selected_count\":{},\"coverage_fraction\":{},\"false_confident_count\":{},\"selective_risk\":{}}}",
                point.confidence_threshold_ppm,
                point.selected_count,
                point.coverage_fraction,
                point.false_confident_count,
                point.selective_risk.map_or_else(|| "null".to_owned(), |value| format!("\"{value:.12}\"")),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"valid_count\":{},\"accepted_count\":{},\"accepted_pose_coverage_fraction\":{},\"abstained_count\":{},\"invalid_count\":{},\"accepted_false_confident_95_count\":{},\"accepted_coverage_50\":{},\"accepted_coverage_80\":{},\"accepted_coverage_90\":{},\"accepted_coverage_95\":{},\"accepted_coverage_99\":{},\"accepted_translation_p95_m\":{},\"accepted_rotation_p95_rad\":{},\"accepted_covariance_logdet_median\":{},\"accepted_95_region_log_volume_median\":{},\"risk_coverage\":[{}]}}",
        summary.valid_count,
        summary.accepted_count,
        summary.accepted_pose_coverage_fraction.map_or_else(|| "null".to_owned(), |value| format!("\"{value:.12}\"")),
        summary.abstained_count,
        summary.invalid_count,
        summary.accepted_false_confident_95_count,
        summary.accepted_coverage_50.map_or_else(|| "null".to_owned(), |value| format!("\"{value:.12}\"")),
        summary.accepted_coverage_80.map_or_else(|| "null".to_owned(), |value| format!("\"{value:.12}\"")),
        summary.accepted_coverage_90.map_or_else(|| "null".to_owned(), |value| format!("\"{value:.12}\"")),
        summary.accepted_coverage_95.map_or_else(|| "null".to_owned(), |value| format!("\"{value:.12}\"")),
        summary.accepted_coverage_99.map_or_else(|| "null".to_owned(), |value| format!("\"{value:.12}\"")),
        summary.accepted_translation_p95_m.map_or_else(|| "null".to_owned(), |value| format!("\"{value:.12}\"")),
        summary.accepted_rotation_p95_rad.map_or_else(|| "null".to_owned(), |value| format!("\"{value:.12}\"")),
        summary.accepted_covariance_logdet_median.map_or_else(|| "null".to_owned(), |value| format!("\"{value:.12}\"")),
        summary.accepted_95_region_log_volume_median.map_or_else(|| "null".to_owned(), |value| format!("\"{value:.12}\"")),
        risk,
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let corpus_path = args.next().ok_or("usage: bench <corpus.csv> <source_commit> <corpus_sha256>")?;
    let source_commit = args.next().ok_or("missing source_commit")?;
    let corpus_sha256 = args.next().ok_or("missing corpus_sha256")?;
    let content = fs::read_to_string(&corpus_path)?;

    let mut case_ids = BTreeSet::new();
    let mut parsed = Vec::new();
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
        if expected.insert(item.input.case_id.clone(), item.expected).is_some() {
            return Err(format!("duplicate expectation: {}", item.input.case_id).into());
        }
        parsed.push(item.input);
    }
    if parsed.is_empty() {
        return Err("empty SE(3) trustworthiness corpus".into());
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

    let summary = summarize(&results);
    let rendered = results.iter().map(render_result).collect::<Vec<_>>().join(",");
    println!(
        "{{\"schema\":\"{}\",\"source_commit\":\"{}\",\"corpus_sha256\":\"{}\",\"covariance_order\":\"{}\",\"covariance_units\":\"{}\",\"admission_level_bps\":9500,\"case_count\":{},\"expected_disposition_matches\":{},\"expected_disposition_mismatches\":0,\"claim_ready\":false,\"source_bytes_authenticated\":false,\"physical_calibration_validated\":false,\"maneuver_authority_granted\":false,\"summary\":{},\"results\":[{}]}}",
        RECEIPT_SCHEMA,
        json_escape(&source_commit),
        json_escape(&corpus_sha256),
        se3_pose_trustworthiness_bench::COVARIANCE_ORDER,
        se3_pose_trustworthiness_bench::COVARIANCE_UNIT,
        results.len(),
        results.len(),
        render_summary(&summary),
        rendered
    );
    Ok(())
}
