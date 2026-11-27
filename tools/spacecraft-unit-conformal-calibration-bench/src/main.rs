use spacecraft_unit_conformal_calibration_bench::{
    EvaluationReceipt, Observation, ObservationStatus, Partition, ScopeResult, ThresholdResult,
    UnitKind, WilsonIntervalPpm, evaluate_manifest,
};
use std::error::Error;
use std::fmt::Write as _;
use std::fs;

const HEADER: &str = "sample_id\ttarget_id\tsession_id\tsequence_id\tunit_kind\tunit_id\tsplit\tdomain_id\tcondition_id\tcrop_stratum\tmodality\tood_group\tcalibration_revision\tunit_frame_budget\tstatus\tscore_q32_32";

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

fn optional_u32(value: Option<u32>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn optional_interval(value: Option<&WilsonIntervalPpm>) -> String {
    value.map_or_else(
        || "null".to_owned(),
        |interval| {
            format!(
                "{{\"lower_ppm\":{},\"upper_ppm\":{}}}",
                interval.lower_ppm, interval.upper_ppm
            )
        },
    )
}

fn render_level(level: &ThresholdResult) -> String {
    format!(
        concat!(
            "{{\"nominal_level_bps\":{},",
            "\"calibration_frame_count\":{},\"calibration_unit_count\":{},",
            "\"framewise_order_statistic_rank\":{},\"unitwise_order_statistic_rank\":{},",
            "\"framewise_threshold_q32_32\":{},\"unitwise_threshold_q32_32\":{},",
            "\"framewise_finite\":{},\"unitwise_finite\":{},",
            "\"test_frame_count\":{},\"test_unit_count\":{},",
            "\"test_scored_frame_count\":{},\"test_abstained_frame_count\":{},",
            "\"test_missing_frame_count\":{},\"test_invalid_frame_count\":{},",
            "\"framewise_covered_frame_count\":{},",
            "\"framewise_conditional_coverage_ppm\":{},\"framewise_operational_success_ppm\":{},",
            "\"framewise_complete_unit_success_count\":{},",
            "\"framewise_complete_unit_wilson_95\":{},",
            "\"framewise_silent_miss_unit_count\":{},\"framewise_silent_miss_wilson_95\":{},",
            "\"unitwise_covered_frame_count\":{},",
            "\"unitwise_conditional_coverage_ppm\":{},\"unitwise_operational_success_ppm\":{},",
            "\"unitwise_complete_unit_success_count\":{},",
            "\"unitwise_complete_unit_wilson_95\":{},",
            "\"unitwise_silent_miss_unit_count\":{},\"unitwise_silent_miss_wilson_95\":{},",
            "\"unitwise_units_with_any_unscored_frame\":{},",
            "\"promotion_eligible_for_declared_scope\":{},\"promotion_reason\":{}}}"
        ),
        level.nominal_level_bps,
        level.calibration_frame_count,
        level.calibration_unit_count,
        level.framewise_order_statistic_rank,
        level.unitwise_order_statistic_rank,
        optional_u64(level.framewise_threshold_q32_32),
        optional_u64(level.unitwise_threshold_q32_32),
        level.framewise_finite,
        level.unitwise_finite,
        level.test_frame_count,
        level.test_unit_count,
        level.test_scored_frame_count,
        level.test_abstained_frame_count,
        level.test_missing_frame_count,
        level.test_invalid_frame_count,
        optional_u64(level.framewise_covered_frame_count),
        optional_u32(level.framewise_conditional_coverage_ppm),
        optional_u32(level.framewise_operational_success_ppm),
        optional_u64(level.framewise_complete_unit_success_count),
        optional_interval(level.framewise_complete_unit_wilson_95.as_ref()),
        optional_u64(level.framewise_silent_miss_unit_count),
        optional_interval(level.framewise_silent_miss_wilson_95.as_ref()),
        optional_u64(level.unitwise_covered_frame_count),
        optional_u32(level.unitwise_conditional_coverage_ppm),
        optional_u32(level.unitwise_operational_success_ppm),
        optional_u64(level.unitwise_complete_unit_success_count),
        optional_interval(level.unitwise_complete_unit_wilson_95.as_ref()),
        optional_u64(level.unitwise_silent_miss_unit_count),
        optional_interval(level.unitwise_silent_miss_wilson_95.as_ref()),
        optional_u64(level.unitwise_units_with_any_unscored_frame),
        level.promotion_eligible_for_declared_scope,
        json_string(&level.promotion_reason)
    )
}

fn render_scope(scope: &ScopeResult) -> String {
    let levels = scope
        .levels
        .iter()
        .map(render_level)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{\"domain_id\":{},\"condition_id\":{},\"crop_stratum\":{},",
            "\"modality\":{},\"ood_group\":{},\"calibration_revision\":{},",
            "\"unit_frame_budget\":{},\"unit_kind\":{},",
            "\"calibration_unscored_frame_count\":{},",
            "\"calibration_units_with_unscored_frames\":{},\"levels\":[{}]}}"
        ),
        json_string(&scope.domain_id),
        json_string(&scope.condition_id),
        json_string(&scope.crop_stratum),
        json_string(&scope.modality),
        json_string(&scope.ood_group),
        json_string(&scope.calibration_revision),
        scope.unit_frame_budget,
        json_string(&scope.unit_kind),
        scope.calibration_unscored_frame_count,
        scope.calibration_units_with_unscored_frames,
        levels
    )
}

fn render_receipt(receipt: &EvaluationReceipt, source_commit: &str, corpus_sha256: &str) -> String {
    let results = receipt
        .results
        .iter()
        .map(render_scope)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{\"schema\":{},\"source_commit\":{},\"corpus_sha256\":{},",
            "\"unit_kind\":{},\"observation_count\":{},\"scope_count\":{},",
            "\"claim_ready\":{},\"dataset_source_bytes_authenticated\":{},",
            "\"calibration_exchangeability_proven\":{},\"physical_calibration_validated\":{},",
            "\"deterministic_safety_bound\":{},\"maneuver_authority_granted\":{},",
            "\"results\":[{}]}}"
        ),
        json_string(&receipt.schema),
        json_string(source_commit),
        json_string(corpus_sha256),
        json_string(&receipt.unit_kind),
        receipt.observation_count,
        receipt.scope_count,
        receipt.claim_ready,
        receipt.dataset_source_bytes_authenticated,
        receipt.calibration_exchangeability_proven,
        receipt.physical_calibration_validated,
        receipt.deterministic_safety_bound,
        receipt.maneuver_authority_granted,
        results
    )
}

fn parse_rows(contents: &str) -> Result<Vec<Observation>, Box<dyn Error>> {
    let mut lines = contents.lines();
    if lines.next() != Some(HEADER) {
        return Err("unexpected TSV header".into());
    }

    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 16 {
            return Err(format!(
                "line {}: expected 16 tab-separated fields, got {}",
                index + 2,
                fields.len()
            )
            .into());
        }
        let frame_budget = fields[13]
            .parse::<u32>()
            .map_err(|error| format!("invalid unit_frame_budget: {error}"))?;
        let score = if fields[15] == "NONE" {
            None
        } else {
            Some(
                fields[15]
                    .parse::<u64>()
                    .map_err(|error| format!("invalid score_q32_32: {error}"))?,
            )
        };
        rows.push(Observation {
            sample_id: fields[0].to_owned(),
            target_id: fields[1].to_owned(),
            session_id: fields[2].to_owned(),
            sequence_id: fields[3].to_owned(),
            unit_kind: UnitKind::parse(fields[4])?,
            unit_id: fields[5].to_owned(),
            partition: Partition::parse(fields[6])?,
            domain_id: fields[7].to_owned(),
            condition_id: fields[8].to_owned(),
            crop_stratum: fields[9].to_owned(),
            modality: fields[10].to_owned(),
            ood_group: fields[11].to_owned(),
            calibration_revision: fields[12].to_owned(),
            unit_frame_budget: frame_budget,
            status: ObservationStatus::parse(fields[14])?,
            score_q32_32: score,
        });
    }
    Ok(rows)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: spacecraft-unit-conformal-calibration-bench <observations.tsv> <source-commit> <corpus-sha256>"
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
    let rows = parse_rows(&contents)?;
    let receipt = evaluate_manifest(&rows)?;
    println!("{}", render_receipt(&receipt, &args[2], &args[3]));
    Ok(())
}
