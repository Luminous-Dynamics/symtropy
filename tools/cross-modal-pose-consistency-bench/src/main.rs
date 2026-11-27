use cross_modal_pose_consistency_bench::{
    ConsistencyReceipt, DependencyDigests, EstimateStatus, GatePolicy, Modality,
    PoseEstimate, Q30_ONE, evaluate_pair,
};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;

const HEADER: &str = "sample_id\tframe_id\ttarget_id\tmax_timestamp_skew_ms\tmax_translation_disagreement_um\tmax_orientation_chord_q30\trgb_timestamp_ms\tevent_timestamp_ms\tframe_convention\ttransform_direction\ttranslation_unit\tquaternion_order\trgb_tx_um\trgb_ty_um\trgb_tz_um\tevent_tx_um\tevent_ty_um\tevent_tz_um\trgb_qx_q30\trgb_qy_q30\trgb_qz_q30\trgb_qw_q30\tevent_qx_q30\tevent_qy_q30\tevent_qz_q30\tevent_qw_q30\trgb_status\tevent_status\trgb_ood\tevent_ood\trgb_low_observability\tevent_low_observability\trgb_dependencies_complete\tevent_dependencies_complete\trgb_clock_sha256\tevent_clock_sha256\trgb_optics_sha256\tevent_optics_sha256\trgb_extrinsics_sha256\tevent_extrinsics_sha256\trgb_model_sha256\tevent_model_sha256\trgb_training_data_sha256\tevent_training_data_sha256\trgb_target_geometry_sha256\tevent_target_geometry_sha256\trgb_renderer_sha256\tevent_renderer_sha256\trgb_annotation_sha256\tevent_annotation_sha256\trgb_translator_sha256\tevent_translator_sha256";

fn parse<T: std::str::FromStr>(value: &str, field: &str) -> Result<T, Box<dyn Error>>
where
    T::Err: std::fmt::Display,
{
    value
        .parse::<T>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_bool(value: &str, field: &str) -> Result<bool, Box<dyn Error>> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid boolean {field}: {value}").into()),
    }
}

fn dependencies(fields: &[&str], offset: usize, complete: bool) -> DependencyDigests {
    DependencyDigests {
        clock: fields[offset].to_owned(),
        optics: fields[offset + 1].to_owned(),
        extrinsics: fields[offset + 2].to_owned(),
        model: fields[offset + 3].to_owned(),
        training_data: fields[offset + 4].to_owned(),
        target_geometry: fields[offset + 5].to_owned(),
        renderer: fields[offset + 6].to_owned(),
        annotation: fields[offset + 7].to_owned(),
        translator: fields[offset + 8].to_owned(),
        declarations_complete: complete,
    }
}

fn render_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control.is_control() => {
                let _ = write!(output, "\\u{:04x}", control as u32);
            }
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

fn render_strings(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| render_string(value))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn render_result(result: &ConsistencyReceipt) -> String {
    format!(
        concat!(
            "{{\"disposition\":{},\"reason_codes\":{},\"sample_id\":{},\"frame_id\":{},",
            "\"target_id\":{},\"timestamp_skew_ms\":{},",
            "\"translation_disagreement_um\":{},\"orientation_chord_q30\":{},",
            "\"shared_dependency_kinds\":{},\"rgb_estimate_preserved\":{},",
            "\"event_estimate_preserved\":{},\"independence_proven\":{},",
            "\"source_bytes_authenticated\":{},\"physical_truth_established\":{},",
            "\"deterministic_safety_bound\":{},\"maneuver_authority_granted\":{}}}"
        ),
        render_string(&result.disposition),
        render_strings(&result.reason_codes),
        render_string(&result.sample_id),
        render_string(&result.frame_id),
        render_string(&result.target_id),
        result.timestamp_skew_ms,
        optional_u64(result.translation_disagreement_um),
        optional_u64(result.orientation_chord_q30),
        render_strings(&result.shared_dependency_kinds),
        result.rgb_estimate_preserved,
        result.event_estimate_preserved,
        result.independence_proven,
        result.source_bytes_authenticated,
        result.physical_truth_established,
        result.deterministic_safety_bound,
        result.maneuver_authority_granted
    )
}

fn parse_pair(fields: &[&str], line: usize) -> Result<(PoseEstimate, PoseEstimate, GatePolicy), Box<dyn Error>> {
    if fields.len() != 54 {
        return Err(format!("line {line}: expected 54 fields, got {}", fields.len()).into());
    }

    let policy = GatePolicy {
        maximum_timestamp_skew_ms: parse(fields[3], "maximum_timestamp_skew_ms")?,
        maximum_translation_disagreement_um: parse(fields[4], "maximum_translation_disagreement_um")?,
        maximum_orientation_chord_q30: parse(fields[5], "maximum_orientation_chord_q30")?,
        require_complete_dependency_declarations: true,
        shared_dependencies_fail_closed: true,
    };
    let frame_convention = fields[8].to_owned();
    let transform_direction = fields[9].to_owned();
    let translation_unit = fields[10].to_owned();
    let quaternion_order = fields[11].to_owned();

    let rgb_complete = parse_bool(fields[32], "rgb_dependencies_complete")?;
    let event_complete = parse_bool(fields[33], "event_dependencies_complete")?;
    let rgb = PoseEstimate {
        modality: Modality::Rgb,
        sample_id: fields[0].to_owned(),
        frame_id: fields[1].to_owned(),
        target_id: fields[2].to_owned(),
        timestamp_ms: parse(fields[6], "rgb_timestamp_ms")?,
        frame_convention: frame_convention.clone(),
        transform_direction: transform_direction.clone(),
        translation_unit: translation_unit.clone(),
        quaternion_order: quaternion_order.clone(),
        translation_um: [
            parse(fields[12], "rgb_tx_um")?,
            parse(fields[13], "rgb_ty_um")?,
            parse(fields[14], "rgb_tz_um")?,
        ],
        quaternion_xyzw_q30: [
            parse(fields[18], "rgb_qx_q30")?,
            parse(fields[19], "rgb_qy_q30")?,
            parse(fields[20], "rgb_qz_q30")?,
            parse(fields[21], "rgb_qw_q30")?,
        ],
        status: EstimateStatus::parse(fields[26])?,
        ood: parse_bool(fields[28], "rgb_ood")?,
        low_observability: parse_bool(fields[30], "rgb_low_observability")?,
        dependencies: dependencies(fields, 34, rgb_complete),
    };
    let event = PoseEstimate {
        modality: Modality::Event,
        sample_id: fields[0].to_owned(),
        frame_id: fields[1].to_owned(),
        target_id: fields[2].to_owned(),
        timestamp_ms: parse(fields[7], "event_timestamp_ms")?,
        frame_convention,
        transform_direction,
        translation_unit,
        quaternion_order,
        translation_um: [
            parse(fields[15], "event_tx_um")?,
            parse(fields[16], "event_ty_um")?,
            parse(fields[17], "event_tz_um")?,
        ],
        quaternion_xyzw_q30: [
            parse(fields[22], "event_qx_q30")?,
            parse(fields[23], "event_qy_q30")?,
            parse(fields[24], "event_qz_q30")?,
            parse(fields[25], "event_qw_q30")?,
        ],
        status: EstimateStatus::parse(fields[27])?,
        ood: parse_bool(fields[29], "event_ood")?,
        low_observability: parse_bool(fields[31], "event_low_observability")?,
        dependencies: dependencies(fields, 35, event_complete),
    };
    Ok((rgb, event, policy))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: cross-modal-pose-consistency-bench <pairs.tsv> <source-commit> <corpus-sha256>"
                .into(),
        );
    }
    if args[2].trim().is_empty()
        || args[3].len() != 64
        || !args[3].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("invalid source commit or corpus SHA-256 argument".into());
    }

    let contents = fs::read_to_string(&args[1])?;
    let mut lines = contents.lines();
    if lines.next() != Some(HEADER) {
        return Err("unexpected cross-modal TSV header".into());
    }

    let mut sample_ids = BTreeSet::new();
    let mut results = Vec::new();
    for (index, line) in lines.enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let (rgb, event, policy) = parse_pair(&fields, index + 2)?;
        if !sample_ids.insert(rgb.sample_id.as_str()) {
            return Err(format!("duplicate pair sample_id {}", rgb.sample_id).into());
        }
        results.push(evaluate_pair(&rgb, &event, &policy)?);
    }
    if results.is_empty() {
        return Err("cross-modal manifest has no pairs".into());
    }

    let body = results.iter().map(render_result).collect::<Vec<_>>().join(",");
    println!(
        "{{\"schema\":{},\"source_commit\":{},\"corpus_sha256\":{},\"pair_count\":{},\"results\":[{}],\"independence_proven\":false,\"physical_truth_established\":false,\"deterministic_safety_bound\":false,\"maneuver_authority_granted\":false}}",
        render_string(cross_modal_pose_consistency_bench::RECEIPT_SCHEMA),
        render_string(&args[2]),
        render_string(&args[3]),
        results.len(),
        body
    );
    let _ = Q30_ONE;
    Ok(())
}
