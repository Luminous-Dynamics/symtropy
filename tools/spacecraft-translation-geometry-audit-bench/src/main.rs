use spacecraft_translation_geometry_audit_bench::{
    AuditReceipt, CropState, FrameAnnotations, FrameMetadata, FrameScore, LandmarkPair, MaskPair,
    MaskScore, ScopeKey, UnitKind, UnitScore, Visibility, audit_manifest,
};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;

const HEADER: &str = "sample_id\ttarget_id\tsession_id\tsequence_id\tunit_kind\tunit_id\tpartition\tdomain_id\tcondition_id\tcrop_stratum\tmodality\tood_group\tcalibration_revision\ttranslator_sha256\tsource_image_sha256\ttranslated_image_sha256\tsource_annotation_sha256\ttranslated_annotation_sha256\timage_width_px\timage_height_px\tframe_index\tframe_budget\tlandmark_tolerance_mpx\ttemporal_motion_tolerance_mpx\tdefault_mask_iou_floor_bps\trecord_kind\trecord_id\tsource_x_mpx\tsource_y_mpx\ttranslated_x_mpx\ttranslated_y_mpx\tsource_visibility\ttranslated_visibility\tsource_crop_state\ttranslated_crop_state\tsource_mask_hex\ttranslated_mask_hex\tmask_iou_floor_bps";

fn parse_u32(value: &str, field: &str) -> Result<u32, Box<dyn Error>> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {field}: {error}").into())
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

fn parse_u16(value: &str, field: &str) -> Result<u16, Box<dyn Error>> {
    value
        .parse::<u16>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn json_string(value: &str) -> String {
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

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn render_mask(score: &MaskScore) -> String {
    format!(
        concat!(
            "{{\"region_id\":{},\"intersection_pixels\":{},\"union_pixels\":{},",
            "\"iou_bps\":{},\"source_component_count\":{},",
            "\"translated_component_count\":{},\"passed\":{}}}"
        ),
        json_string(&score.region_id),
        score.intersection_pixels,
        score.union_pixels,
        score.iou_bps,
        score.source_component_count,
        score.translated_component_count,
        score.passed
    )
}

fn render_frame(frame: &FrameScore) -> String {
    let reasons = frame
        .reason_codes
        .iter()
        .map(|reason| json_string(reason))
        .collect::<Vec<_>>()
        .join(",");
    let masks = frame
        .mask_scores
        .iter()
        .map(render_mask)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{\"sample_id\":{},\"frame_index\":{},\"disposition\":{},",
            "\"reason_codes\":[{}],\"compared_landmark_count\":{},",
            "\"max_landmark_displacement_mpx\":{},",
            "\"p95_landmark_displacement_mpx\":{},\"mask_scores\":[{}]}}"
        ),
        json_string(&frame.sample_id),
        frame.frame_index,
        json_string(&frame.disposition),
        reasons,
        frame.compared_landmark_count,
        optional_u64(frame.max_landmark_displacement_mpx),
        optional_u64(frame.p95_landmark_displacement_mpx),
        masks
    )
}

fn render_unit(unit: &UnitScore) -> String {
    let reasons = unit
        .reason_codes
        .iter()
        .map(|reason| json_string(reason))
        .collect::<Vec<_>>()
        .join(",");
    let frames = unit
        .frames
        .iter()
        .map(render_frame)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{\"unit_id\":{},\"unit_kind\":{},\"frame_count\":{},",
            "\"passed_frame_count\":{},\"blocked_frame_count\":{},",
            "\"max_landmark_displacement_mpx\":{},",
            "\"p95_landmark_displacement_mpx\":{},",
            "\"max_temporal_motion_residual_mpx\":{},\"disposition\":{},",
            "\"reason_codes\":[{}],\"frames\":[{}]}}"
        ),
        json_string(&unit.unit_id),
        json_string(&unit.unit_kind),
        unit.frame_count,
        unit.passed_frame_count,
        unit.blocked_frame_count,
        optional_u64(unit.max_landmark_displacement_mpx),
        optional_u64(unit.p95_landmark_displacement_mpx),
        optional_u64(unit.max_temporal_motion_residual_mpx),
        json_string(&unit.disposition),
        reasons,
        frames
    )
}

fn render_receipt(receipt: &AuditReceipt, source_commit: &str, corpus_sha256: &str) -> String {
    let results = receipt
        .results
        .iter()
        .map(render_unit)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{\"schema\":{},\"source_commit\":{},\"corpus_sha256\":{},",
            "\"unit_count\":{},\"frame_count\":{},\"passed_unit_count\":{},",
            "\"blocked_unit_count\":{},\"image_pixels_inspected_here\":{},",
            "\"annotation_source_bytes_authenticated\":{},",
            "\"independent_metrology_verified_here\":{},",
            "\"physical_truth_established\":{},\"calibration_exchangeability_proven\":{},",
            "\"flight_qualification\":{},\"maneuver_authority_granted\":{},",
            "\"results\":[{}]}}"
        ),
        json_string(&receipt.schema),
        json_string(source_commit),
        json_string(corpus_sha256),
        receipt.unit_count,
        receipt.frame_count,
        receipt.passed_unit_count,
        receipt.blocked_unit_count,
        receipt.image_pixels_inspected_here,
        receipt.annotation_source_bytes_authenticated,
        receipt.independent_metrology_verified_here,
        receipt.physical_truth_established,
        receipt.calibration_exchangeability_proven,
        receipt.flight_qualification,
        receipt.maneuver_authority_granted,
        results
    )
}

fn parse_manifest(contents: &str) -> Result<Vec<FrameAnnotations>, Box<dyn Error>> {
    let mut lines = contents.lines();
    if lines.next() != Some(HEADER) {
        return Err("unexpected geometry audit TSV header".into());
    }
    let mut frames = BTreeMap::<String, FrameAnnotations>::new();

    for (index, line) in lines.enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 38 {
            return Err(format!(
                "line {}: expected 38 tab-separated fields, got {}",
                index + 2,
                fields.len()
            )
            .into());
        }

        let unit_kind = UnitKind::parse(fields[4])?;
        let width = parse_u32(fields[18], "image_width_px")?;
        let height = parse_u32(fields[19], "image_height_px")?;
        let frame_index = parse_u32(fields[20], "frame_index")?;
        let budget = parse_u32(fields[21], "frame_budget")?;
        let landmark_tolerance = parse_u64(fields[22], "landmark_tolerance_mpx")?;
        let temporal_tolerance = parse_u64(fields[23], "temporal_motion_tolerance_mpx")?;
        let default_iou_floor = parse_u16(fields[24], "default_mask_iou_floor_bps")?;

        let scope = ScopeKey {
            domain_id: fields[7].to_owned(),
            condition_id: fields[8].to_owned(),
            crop_stratum: fields[9].to_owned(),
            modality: fields[10].to_owned(),
            ood_group: fields[11].to_owned(),
            calibration_revision: fields[12].to_owned(),
            translator_sha256: fields[13].to_owned(),
            image_width_px: width,
            image_height_px: height,
            unit_kind,
            unit_frame_budget: budget,
            landmark_tolerance_mpx: landmark_tolerance,
            temporal_motion_tolerance_mpx: temporal_tolerance,
            default_mask_iou_floor_bps: default_iou_floor,
        };
        let metadata = FrameMetadata {
            sample_id: fields[0].to_owned(),
            target_id: fields[1].to_owned(),
            session_id: fields[2].to_owned(),
            sequence_id: fields[3].to_owned(),
            unit_id: fields[5].to_owned(),
            unit_kind,
            partition: fields[6].to_owned(),
            frame_index,
            scope,
            source_image_sha256: fields[14].to_owned(),
            translated_image_sha256: fields[15].to_owned(),
            source_annotation_sha256: fields[16].to_owned(),
            translated_annotation_sha256: fields[17].to_owned(),
        };

        let frame = frames
            .entry(metadata.sample_id.clone())
            .or_insert_with(|| FrameAnnotations {
                metadata: metadata.clone(),
                landmarks: Vec::new(),
                masks: Vec::new(),
            });
        if frame.metadata != metadata {
            return Err(format!(
                "line {}: frame metadata differs across records for sample {}",
                index + 2,
                metadata.sample_id
            )
            .into());
        }

        match fields[25] {
            "LANDMARK" => {
                if fields[35] != "NONE" || fields[36] != "NONE" || fields[37] != "NONE" {
                    return Err(
                        format!("line {}: LANDMARK row carries mask fields", index + 2).into(),
                    );
                }
                frame.landmarks.push(LandmarkPair {
                    landmark_id: fields[26].to_owned(),
                    source_x_mpx: parse_i64(fields[27], "source_x_mpx")?,
                    source_y_mpx: parse_i64(fields[28], "source_y_mpx")?,
                    translated_x_mpx: parse_i64(fields[29], "translated_x_mpx")?,
                    translated_y_mpx: parse_i64(fields[30], "translated_y_mpx")?,
                    source_visibility: Visibility::parse(fields[31])?,
                    translated_visibility: Visibility::parse(fields[32])?,
                    source_crop_state: CropState::parse(fields[33])?,
                    translated_crop_state: CropState::parse(fields[34])?,
                });
            }
            "MASK" => {
                if fields[27..35].iter().any(|field| *field != "NONE") {
                    return Err(
                        format!("line {}: MASK row carries landmark fields", index + 2).into(),
                    );
                }
                frame.masks.push(MaskPair {
                    region_id: fields[26].to_owned(),
                    source_mask_hex: fields[35].to_owned(),
                    translated_mask_hex: fields[36].to_owned(),
                    iou_floor_bps: parse_u16(fields[37], "mask_iou_floor_bps")?,
                });
            }
            other => {
                return Err(format!("line {}: unsupported record_kind {other}", index + 2).into());
            }
        }
    }

    Ok(frames.into_values().collect())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: spacecraft-translation-geometry-audit-bench <annotations.tsv> <source-commit> <corpus-sha256>"
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
    let frames = parse_manifest(&contents)?;
    let receipt = audit_manifest(&frames)?;
    println!("{}", render_receipt(&receipt, &args[2], &args[3]));
    Ok(())
}
