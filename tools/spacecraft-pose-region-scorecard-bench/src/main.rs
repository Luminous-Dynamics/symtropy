use spacecraft_pose_region_scorecard_bench::{
    NOMINAL_LEVELS, PoseRegion, PredictionInput, PoseScore, RECEIPT_SCHEMA, RegionScore,
    ScoreDisposition, Scorecard, evaluate_manifest,
};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;

struct ParsedCase {
    input: PredictionInput,
    expected: String,
}

fn parse_i64(value: &str, field: &str) -> Result<i64, Box<dyn Error>> {
    value
        .parse::<i64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_u64(value: &str, field: &str) -> Result<u64, Box<dyn Error>> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_f64(value: &str, field: &str) -> Result<f64, Box<dyn Error>> {
    value
        .parse::<f64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_region(fields: &[&str], offset: usize, level: u8) -> Result<PoseRegion, Box<dyn Error>> {
    Ok(PoseRegion {
        lower_um: [
            parse_i64(fields[offset], &format!("{level}_lower_x"))?,
            parse_i64(fields[offset + 2], &format!("{level}_lower_y"))?,
            parse_i64(fields[offset + 4], &format!("{level}_lower_z"))?,
        ],
        upper_um: [
            parse_i64(fields[offset + 1], &format!("{level}_upper_x"))?,
            parse_i64(fields[offset + 3], &format!("{level}_upper_y"))?,
            parse_i64(fields[offset + 5], &format!("{level}_upper_z"))?,
        ],
        orientation_radius_urad: parse_u64(fields[offset + 6], &format!("{level}_orientation_radius_urad"))?,
    })
}

fn parse_row(fields: &[&str], line_number: usize) -> Result<ParsedCase, Box<dyn Error>> {
    if fields.len() != 64 {
        return Err(format!(
            "line {line_number}: expected 64 comma-separated fields, found {}",
            fields.len()
        )
        .into());
    }

    let mut regions = Vec::with_capacity(5);
    for (index, level) in NOMINAL_LEVELS.iter().enumerate() {
        regions.push(parse_region(fields, 29 + index * 7, *level)?);
    }
    let regions: [PoseRegion; 5] = regions
        .try_into()
        .map_err(|_| "expected five nested pose regions")?;

    Ok(ParsedCase {
        expected: fields[1].to_owned(),
        input: PredictionInput {
            case_id: fields[0].to_owned(),
            sample_id: fields[2].to_owned(),
            target_id: fields[3].to_owned(),
            split: fields[4].to_owned(),
            crop_stratum: fields[5].to_owned(),
            visible_parts: fields[6].to_owned(),
            modality: fields[7].to_owned(),
            ood_group: fields[8].to_owned(),
            calibration_revision: fields[9].to_owned(),
            frame_id: fields[10].to_owned(),
            quaternion_order: fields[11].to_owned(),
            translation_unit: fields[12].to_owned(),
            pose_semantics: fields[13].to_owned(),
            status: fields[14].to_owned(),
            predicted_translation_um: [
                parse_i64(fields[15], "pred_tx_um")?,
                parse_i64(fields[16], "pred_ty_um")?,
                parse_i64(fields[17], "pred_tz_um")?,
            ],
            truth_translation_um: [
                parse_i64(fields[18], "truth_tx_um")?,
                parse_i64(fields[19], "truth_ty_um")?,
                parse_i64(fields[20], "truth_tz_um")?,
            ],
            predicted_quaternion: [
                parse_f64(fields[21], "pred_q0")?,
                parse_f64(fields[22], "pred_q1")?,
                parse_f64(fields[23], "pred_q2")?,
                parse_f64(fields[24], "pred_q3")?,
            ],
            truth_quaternion: [
                parse_f64(fields[25], "truth_q0")?,
                parse_f64(fields[26], "truth_q1")?,
                parse_f64(fields[27], "truth_q2")?,
                parse_f64(fields[28], "truth_q3")?,
            ],
            regions,
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

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn bool_array(values: [bool; 3]) -> String {
    format!("[{},{},{}]", values[0], values[1], values[2])
}

fn uint_array(values: [u64; 3]) -> String {
    format!("[{},{},{}]", values[0], values[1], values[2])
}

fn render_region(region: &RegionScore) -> String {
    format!(
        "{{\"nominal_percent\":{},\"translation_axis_covered\":{},\"translation_joint_covered\":{},\"orientation_covered\":{},\"joint_covered\":{},\"orientation_error_urad\":{},\"width_um\":{},\"box_volume_um3\":{},\"orientation_radius_urad\":{}}}",
        region.nominal_percent,
        bool_array(region.translation_axis_covered),
        region.translation_joint_covered,
        region.orientation_covered,
        region.joint_covered,
        region.orientation_error_urad,
        uint_array(region.width_um),
        region.box_volume_um3,
        region.orientation_radius_urad,
    )
}

fn render_result(result: &PoseScore) -> String {
    let reasons = result
        .reason_codes
        .iter()
        .map(|reason| format!("\"{}\"", json_escape(reason)))
        .collect::<Vec<_>>()
        .join(",");
    let regions = result
        .regions
        .iter()
        .map(render_region)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"case_id\":\"{}\",\"sample_id\":\"{}\",\"target_id\":\"{}\",\"split\":\"{}\",\"crop_stratum\":\"{}\",\"visible_parts\":\"{}\",\"modality\":\"{}\",\"ood_group\":\"{}\",\"calibration_revision\":\"{}\",\"frame_id\":\"{}\",\"disposition\":\"{}\",\"reason_codes\":[{}],\"translation_error_l2_um\":{},\"orientation_error_urad\":{},\"regions\":[{}]}}",
        json_escape(&result.case_id),
        json_escape(&result.sample_id),
        json_escape(&result.target_id),
        json_escape(&result.split),
        json_escape(&result.crop_stratum),
        json_escape(&result.visible_parts),
        json_escape(&result.modality),
        json_escape(&result.ood_group),
        json_escape(&result.calibration_revision),
        json_escape(&result.frame_id),
        result.disposition.as_str(),
        reasons,
        optional_u64(result.translation_error_l2_um),
        optional_u64(result.orientation_error_urad),
        regions,
    )
}

fn render_aggregate(level: &spacecraft_pose_region_scorecard_bench::LevelAggregate) -> String {
    format!(
        "{{\"nominal_percent\":{},\"accepted_count\":{},\"translation_axis_covered\":[{},{},{}],\"translation_joint_covered\":{},\"orientation_covered\":{},\"joint_covered\":{},\"false_confident\":{},\"sum_width_um\":[{},{},{}],\"sum_box_volume_um3\":{},\"sum_orientation_radius_urad\":{}}}",
        level.nominal_percent,
        level.accepted_count,
        level.translation_axis_covered[0],
        level.translation_axis_covered[1],
        level.translation_axis_covered[2],
        level.translation_joint_covered,
        level.orientation_covered,
        level.joint_covered,
        level.false_confident,
        level.sum_width_um[0],
        level.sum_width_um[1],
        level.sum_width_um[2],
        level.sum_box_volume_um3,
        level.sum_orientation_radius_urad,
    )
}

fn render_scorecard(card: &Scorecard) -> String {
    let levels = card.levels.iter().map(render_aggregate).collect::<Vec<_>>().join(",");
    let groups = card.groups.iter().map(|group| {
        let group_levels = group.levels.iter().map(render_aggregate).collect::<Vec<_>>().join(",");
        format!(
            "{{\"target_id\":\"{}\",\"crop_stratum\":\"{}\",\"modality\":\"{}\",\"ood_group\":\"{}\",\"sample_count\":{},\"accepted_count\":{},\"abstained_count\":{},\"missing_count\":{},\"invalid_count\":{},\"sum_translation_error_l2_um\":{},\"max_translation_error_l2_um\":{},\"sum_orientation_error_urad\":{},\"max_orientation_error_urad\":{},\"levels\":[{}]}}",
            json_escape(&group.target_id),
            json_escape(&group.crop_stratum),
            json_escape(&group.modality),
            json_escape(&group.ood_group),
            group.sample_count,
            group.accepted_count,
            group.abstained_count,
            group.missing_count,
            group.invalid_count,
            group.sum_translation_error_l2_um,
            group.max_translation_error_l2_um,
            group.sum_orientation_error_urad,
            group.max_orientation_error_urad,
            group_levels,
        )
    }).collect::<Vec<_>>().join(",");
    let results = card.results.iter().map(render_result).collect::<Vec<_>>().join(",");
    format!(
        "{{\"case_count\":{},\"scored_count\":{},\"abstained_count\":{},\"missing_count\":{},\"invalid_count\":{},\"duplicate_sample_count\":{},\"split_leakage_count\":{},\"levels\":[{}],\"groups\":[{}],\"results\":[{}]}}",
        card.case_count,
        card.scored_count,
        card.abstained_count,
        card.missing_count,
        card.invalid_count,
        card.duplicate_sample_count,
        card.split_leakage_count,
        levels,
        groups,
        results,
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let corpus_path = args.next().ok_or("usage: bench <corpus.csv> <source_commit> <corpus_sha256>")?;
    let source_commit = args.next().ok_or("missing source_commit")?;
    let corpus_sha256 = args.next().ok_or("missing corpus_sha256")?;
    let content = fs::read_to_string(&corpus_path)?;

    let mut inputs = Vec::new();
    let mut case_ids = BTreeSet::new();
    let mut expected = BTreeMap::new();
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
        return Err("empty pose region corpus".into());
    }

    let card = evaluate_manifest(&inputs);
    for result in &card.results {
        let expected_disposition = expected
            .get(&result.case_id)
            .ok_or_else(|| format!("missing expected result for {}", result.case_id))?;
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

    println!(
        "{{\"schema\":\"{}\",\"source_commit\":\"{}\",\"corpus_sha256\":\"{}\",\"claim_ready\":false,\"source_bytes_authenticated\":false,\"physical_validation\":\"NOT_ASSESSED\",\"authority_granted\":false,\"scorecard\":{}}}",
        RECEIPT_SCHEMA,
        json_escape(&source_commit),
        json_escape(&corpus_sha256),
        render_scorecard(&card),
    );
    Ok(())
}
