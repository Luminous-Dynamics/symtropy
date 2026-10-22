use crate::{sha256_hex, DatasetManifest, MANIFEST_SCHEMA};
use csv::StringRecord;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PCBWORLD_CSV_RECEIPT_SCHEMA: &str = "pcbworld-per-rollout-import-receipt-v0.2";
pub const PCBWORLD_TIME_BUDGETS_SECONDS: &[u32] = &[30, 60, 180, 300, 1800];
const REQUIRED_COLUMNS: &[&str] = &[
    "board_id",
    "rollout_idx",
    "seed",
    "status",
    "success",
    "clean_pass",
    "routability",
    "wirelength_mm",
    "via_count",
    "track_count",
    "drv_errors_only_count",
    "drv_errors_and_promoted_count",
    "final_potential",
    "initial_potential",
    "potential_gain",
    "reward_config",
    "check_angle",
    "eval_status",
];

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PcbWorldDrcViolation {
    pub severity_label: String,
    pub severity: u32,
    pub error_code: u32,
    pub error_type: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub layer: i64,
    pub net_names: Vec<String>,
    pub is_error: bool,
    pub is_promoted: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PcbWorldReferenceMetrics {
    /// PCBWorld's published connectivity metric. This is not an independent DRC result.
    pub success: Option<bool>,
    /// PCBWorld's published clean_pass: success and zero ERROR-severity DRCs.
    /// It does not necessarily require zero promoted warnings.
    pub clean_pass: Option<bool>,
    pub routability: Option<f64>,
    pub ratsnest_reduction: Option<f64>,
    pub unrouted_count: Option<u32>,
    pub initial_unconnected: Option<u32>,
    pub wirelength_mm: Option<f64>,
    pub via_count: Option<u32>,
    pub track_count: Option<u32>,
    pub drc_violations: Option<u32>,
    pub drv_errors_only_count: Option<u32>,
    pub drv_errors_and_promoted_count: Option<u32>,
    pub track_angle_drv_count: Option<u32>,
    pub total_drv_count: Option<u32>,
    pub initial_unrouted_edges: Option<u32>,
    pub unrouted_edges_remaining: Option<u32>,
    pub final_potential: Option<f64>,
    pub initial_potential: Option<f64>,
    pub potential_gain: Option<f64>,
    pub per_board_rollout_time_secs: Option<f64>,
    pub eval_time_secs: Option<f64>,
    pub eval_status: Option<String>,
    pub reward_config: Option<String>,
    pub check_angle: Option<u32>,
    /// Present only when the source CSV actually includes a serialized violation list.
    pub drv_violations: Option<Vec<PcbWorldDrcViolation>>,
    /// False flags a potential-gain arithmetic inconsistency; values are preserved.
    pub potential_gain_consistent: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PcbWorldRolloutRow {
    pub board_id: String,
    pub rollout_idx: u32,
    pub seed: Option<i64>,
    pub checkpoint: Option<String>,
    pub status: String,
    pub artifact_path_present: bool,
    pub reference_metrics: PcbWorldReferenceMetrics,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PcbWorldCsvSummary {
    pub rows_attempted: usize,
    pub unique_boards: usize,
    pub rows_completed: usize,
    pub rows_crashed: usize,
    pub rows_truncated: usize,
    pub rows_skipped: usize,
    pub rows_with_success_metric: usize,
    pub rows_with_clean_pass_metric: usize,
    pub success_attempts: usize,
    pub clean_pass_attempts: usize,
    /// Explicitly true outcomes divided by all attempts; missing metrics stay in the denominator.
    pub success_rate_over_all_rows: f64,
    /// Explicitly clean outcomes divided by all attempts; missing metrics stay in the denominator.
    pub clean_pass_rate_over_all_rows: f64,
    /// Success among rows where the upstream CSV actually reports the metric.
    pub success_rate_over_metric_reporting_rows: Option<f64>,
    /// Clean pass among rows where the upstream CSV actually reports the metric.
    pub clean_pass_rate_over_metric_reporting_rows: Option<f64>,
    pub rows_with_routability: usize,
    pub mean_routability_over_recorded_rows: Option<f64>,
    pub rows_with_error_only_drc_count: usize,
    pub error_only_drc_total: u64,
    pub rows_with_promoted_drc_count: usize,
    pub error_and_promoted_drc_total: u64,
    pub rows_missing_reward_config: usize,
    pub rows_missing_check_angle: usize,
    pub scoring_policy_variants: Vec<String>,
    pub potential_gain_inconsistency_rows: usize,
    pub rows_with_serialized_drc_violations: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PcbWorldTimeBudgetScore {
    pub time_budget_seconds: u32,
    pub attempts_total: usize,
    pub attempts_with_runtime: usize,
    pub attempts_without_runtime: usize,
    pub clean_passes_within_budget: usize,
    pub clean_pass_attempts_over_budget: usize,
    pub clean_pass_attempts_without_runtime: usize,
    /// Clean attempts whose recorded final runtime is within the cap divided by all rows.
    /// Attempts without runtime are not counted as passes.
    pub clean_pass_rate_over_all_attempts: f64,
    /// Diagnostic only: the same successes divided by rows with known runtime.
    pub clean_pass_rate_over_timed_attempts: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PcbWorldCsvImportReceipt {
    pub schema: String,
    pub evaluator_git_head: String,
    pub input_csv_sha256: String,
    pub dataset_manifest_sha256: String,
    pub dataset_id: String,
    pub split_id: String,
    pub source_board_count: usize,
    pub row_count: usize,
    pub summary: PcbWorldCsvSummary,
    pub time_budget_sweep: Vec<PcbWorldTimeBudgetScore>,
    pub rows: Vec<PcbWorldRolloutRow>,
    pub authority: String,
    pub claim_ceiling: String,
}

/// Import the upstream per-rollout CSV while preserving its published metric semantics.
/// This function does not infer a clean board from a CSV self-label; the source's
/// clean_pass is retained independently of the evaluator's stricter route disposition.
pub fn import_pcbworld_per_rollout_csv(
    csv_bytes: &[u8],
    manifest_bytes: &[u8],
    evaluator_git_head: &str,
) -> Result<PcbWorldCsvImportReceipt, String> {
    if !is_commit_sha(evaluator_git_head) {
        return Err(
            "evaluator Git HEAD must be a 40- or 64-character hexadecimal commit ID".into(),
        );
    }

    let manifest: DatasetManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| format!("dataset manifest JSON/schema invalid: {error}"))?;
    crate::validate_manifest(&manifest)?;
    if manifest.schema != MANIFEST_SCHEMA {
        return Err(format!(
            "unsupported dataset manifest schema: {}",
            manifest.schema
        ));
    }

    let mut reader = csv::ReaderBuilder::new()
        .flexible(false)
        .from_reader(csv_bytes);
    let headers = reader
        .headers()
        .map_err(|error| format!("PCBWorld CSV header parse failed: {error}"))?
        .clone();
    let mut column_index = BTreeMap::new();
    for (index, name) in headers.iter().enumerate() {
        if name.trim().is_empty() {
            return Err("PCBWorld CSV has an empty header name".into());
        }
        if column_index.insert(name.to_string(), index).is_some() {
            return Err(format!("PCBWorld CSV has duplicate column name: {name}"));
        }
    }
    let mut missing_columns: Vec<&str> = REQUIRED_COLUMNS
        .iter()
        .copied()
        .filter(|name| !column_index.contains_key(*name))
        .collect();
    missing_columns.sort_unstable();
    if !missing_columns.is_empty() {
        return Err(format!(
            "PCBWorld per-rollout CSV is missing required columns: {}",
            missing_columns.join(", ")
        ));
    }

    let source_board_ids: BTreeSet<String> = manifest
        .boards
        .iter()
        .map(|board| board.board_id.clone())
        .collect();
    let mut seen_attempts: BTreeSet<(String, u32, Option<i64>)> = BTreeSet::new();
    let mut observed_board_ids = BTreeSet::new();
    let mut rows = Vec::new();

    for (offset, record) in reader.records().enumerate() {
        let row_number = offset + 2;
        let record = record
            .map_err(|error| format!("PCBWorld CSV row {row_number} is malformed: {error}"))?;
        let board_id = required_text(&record, &column_index, "board_id", row_number)?;
        if !source_board_ids.contains(&board_id) {
            return Err(format!(
                "PCBWorld CSV row {row_number} names board {board_id} outside the pinned manifest"
            ));
        }
        let rollout_idx = required_u32(&record, &column_index, "rollout_idx", row_number)?;
        let seed = optional_i64(&record, &column_index, "seed", row_number)?;
        if !seen_attempts.insert((board_id.clone(), rollout_idx, seed)) {
            return Err(format!(
                "duplicate PCBWorld attempt key at row {row_number}: board={board_id}, rollout_idx={rollout_idx}, seed={seed:?}"
            ));
        }
        observed_board_ids.insert(board_id.clone());

        let status = required_text(&record, &column_index, "status", row_number)?;
        let reference_metrics = parse_reference_metrics(&record, &column_index, row_number)?;
        rows.push(PcbWorldRolloutRow {
            board_id,
            rollout_idx,
            seed,
            checkpoint: optional_text(&record, &column_index, "ckpt"),
            status,
            artifact_path_present: optional_text(&record, &column_index, "artifact_path").is_some(),
            reference_metrics,
        });
    }

    if rows.is_empty() {
        return Err("PCBWorld per-rollout CSV has no data rows".into());
    }
    let missing_boards: Vec<String> = source_board_ids
        .difference(&observed_board_ids)
        .cloned()
        .collect();
    if !missing_boards.is_empty() {
        return Err(format!(
            "PCBWorld CSV omits {} pinned board(s), e.g. {}",
            missing_boards.len(),
            missing_boards
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // A cell must not silently mix scoring policies. Missing policy metadata is retained,
    // but successful post-hoc scoring rows must declare their reward config and angle.
    let mut policies = BTreeSet::new();
    for row in &rows {
        if row.reference_metrics.eval_status.as_deref() == Some("ok") {
            let reward_config =
                row.reference_metrics
                    .reward_config
                    .as_deref()
                    .ok_or_else(|| {
                        format!(
                            "{} rollout {}: eval_status=ok but reward_config is missing",
                            row.board_id, row.rollout_idx
                        )
                    })?;
            let check_angle = row.reference_metrics.check_angle.ok_or_else(|| {
                format!(
                    "{} rollout {}: eval_status=ok but check_angle is missing",
                    row.board_id, row.rollout_idx
                )
            })?;
            policies.insert(format!("{reward_config}|check_angle={check_angle}"));
        }
    }
    if policies.len() > 1 {
        return Err(format!(
            "PCBWorld CSV mixes DRC scoring policies in one import: {}",
            policies.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    rows.sort_by(|left, right| {
        left.board_id
            .cmp(&right.board_id)
            .then_with(|| left.rollout_idx.cmp(&right.rollout_idx))
            .then_with(|| left.seed.cmp(&right.seed))
    });
    let summary = summarize(&rows);
    let time_budget_sweep = summarize_time_budget_sweep(&rows);
    Ok(PcbWorldCsvImportReceipt {
        schema: PCBWORLD_CSV_RECEIPT_SCHEMA.into(),
        evaluator_git_head: evaluator_git_head.to_ascii_lowercase(),
        input_csv_sha256: sha256_hex(csv_bytes),
        dataset_manifest_sha256: sha256_hex(manifest_bytes),
        dataset_id: manifest.dataset_id,
        split_id: manifest.split_id,
        source_board_count: source_board_ids.len(),
        row_count: rows.len(),
        summary,
        time_budget_sweep,
        rows,
        authority: "pinned_pcbworld_per_rollout_metric_import_only_no_independent_router_or_physical_claim".into(),
        claim_ceiling: "This receipt preserves published PCBWorld per-rollout fields and the source CSV byte digest. Time-budget scores count only clean_pass rows with recorded rollout time at or below each budget; untimed rows remain in the denominator and are not credited. These are upstream self-reported metrics until raw board/DRC evidence is independently verified. They do not establish signal/power integrity, manufacturability, physical quality or production release.".into(),
    })
}

fn parse_reference_metrics(
    record: &StringRecord,
    columns: &BTreeMap<String, usize>,
    row_number: usize,
) -> Result<PcbWorldReferenceMetrics, String> {
    let value =
        |name: &str| -> Option<&str> { columns.get(name).and_then(|index| record.get(*index)) };
    let success = parse_optional_bool(value("success"), "success", row_number)?;
    let clean_pass = parse_optional_bool(value("clean_pass"), "clean_pass", row_number)?;
    let routability = parse_optional_f64(value("routability"), "routability", row_number)?;
    if routability.is_some_and(|number| !(0.0..=1.0).contains(&number)) {
        return Err(format!(
            "PCBWorld CSV row {row_number}: routability must be within [0,1]"
        ));
    }
    let drv_errors_only_count = parse_optional_u32(
        value("drv_errors_only_count"),
        "drv_errors_only_count",
        row_number,
    )?;
    let drv_errors_and_promoted_count = parse_optional_u32(
        value("drv_errors_and_promoted_count"),
        "drv_errors_and_promoted_count",
        row_number,
    )?;

    if let (Some(clean), Some(success_value), Some(error_count)) =
        (clean_pass, success, drv_errors_only_count)
    {
        let expected_clean = success_value && error_count == 0;
        if clean != expected_clean {
            return Err(format!(
                "PCBWorld CSV row {row_number}: clean_pass={clean} contradicts PCBWorld definition success && drv_errors_only_count==0"
            ));
        }
    }

    let drv_violations = match value("drv_violations") {
        Some(cell) if !is_missing(cell) => {
            let records: Vec<PcbWorldDrcViolation> =
                serde_json::from_str(cell).map_err(|error| {
                    format!("PCBWorld CSV row {row_number}: drv_violations JSON invalid: {error}")
                })?;
            validate_violation_counts(
                &records,
                drv_errors_only_count,
                drv_errors_and_promoted_count,
                row_number,
            )?;
            Some(records)
        }
        _ => None,
    };

    let final_potential =
        parse_optional_f64(value("final_potential"), "final_potential", row_number)?;
    let initial_potential =
        parse_optional_f64(value("initial_potential"), "initial_potential", row_number)?;
    let potential_gain = parse_optional_f64(value("potential_gain"), "potential_gain", row_number)?;
    let potential_gain_consistent = match (final_potential, initial_potential, potential_gain) {
        (Some(final_value), Some(initial_value), Some(gain)) => Some(
            (gain - (final_value - initial_value)).abs()
                <= 1e-4_f64.max(final_value.abs().max(initial_value.abs()).max(gain.abs()) * 1e-6),
        ),
        _ => None,
    };
    let check_angle = parse_optional_u32(value("check_angle"), "check_angle", row_number)?;
    if check_angle.is_some_and(|angle| !matches!(angle, 45 | 90)) {
        return Err(format!(
            "PCBWorld CSV row {row_number}: check_angle must be 45 or 90"
        ));
    }

    Ok(PcbWorldReferenceMetrics {
        success,
        clean_pass,
        routability,
        ratsnest_reduction: parse_optional_f64(
            value("ratsnest_reduction"),
            "ratsnest_reduction",
            row_number,
        )?,
        unrouted_count: parse_optional_u32(value("unrouted_count"), "unrouted_count", row_number)?,
        initial_unconnected: parse_optional_u32(
            value("initial_unconnected"),
            "initial_unconnected",
            row_number,
        )?,
        wirelength_mm: nonnegative_f64(value("wirelength_mm"), "wirelength_mm", row_number)?,
        via_count: parse_optional_u32(value("via_count"), "via_count", row_number)?,
        track_count: parse_optional_u32(value("track_count"), "track_count", row_number)?,
        drc_violations: parse_optional_u32(value("drc_violations"), "drc_violations", row_number)?,
        drv_errors_only_count,
        drv_errors_and_promoted_count,
        track_angle_drv_count: parse_optional_u32(
            value("track_angle_drv_count"),
            "track_angle_drv_count",
            row_number,
        )?,
        total_drv_count: parse_optional_u32(
            value("total_drv_count"),
            "total_drv_count",
            row_number,
        )?,
        initial_unrouted_edges: parse_optional_u32(
            value("initial_unrouted_edges"),
            "initial_unrouted_edges",
            row_number,
        )?,
        unrouted_edges_remaining: parse_optional_u32(
            value("unrouted_edges_remaining"),
            "unrouted_edges_remaining",
            row_number,
        )?,
        final_potential,
        initial_potential,
        potential_gain,
        per_board_rollout_time_secs: nonnegative_f64(
            value("per_board_rollout_time"),
            "per_board_rollout_time",
            row_number,
        )?,
        eval_time_secs: nonnegative_f64(value("eval_time_sec"), "eval_time_sec", row_number)?,
        eval_status: optional_text(record, columns, "eval_status"),
        reward_config: optional_text(record, columns, "reward_config"),
        check_angle,
        drv_violations,
        potential_gain_consistent,
    })
}

fn summarize_time_budget_sweep(rows: &[PcbWorldRolloutRow]) -> Vec<PcbWorldTimeBudgetScore> {
    PCBWORLD_TIME_BUDGETS_SECONDS
        .iter()
        .copied()
        .map(|budget| {
            let attempts_with_runtime = rows
                .iter()
                .filter(|row| row.reference_metrics.per_board_rollout_time_secs.is_some())
                .count();
            let attempts_without_runtime = rows.len() - attempts_with_runtime;
            let clean_passes_within_budget = rows
                .iter()
                .filter(|row| {
                    row.reference_metrics.clean_pass == Some(true)
                        && row
                            .reference_metrics
                            .per_board_rollout_time_secs
                            .is_some_and(|seconds| seconds <= f64::from(budget))
                })
                .count();
            let clean_pass_attempts_over_budget = rows
                .iter()
                .filter(|row| {
                    row.reference_metrics.clean_pass == Some(true)
                        && row
                            .reference_metrics
                            .per_board_rollout_time_secs
                            .is_some_and(|seconds| seconds > f64::from(budget))
                })
                .count();
            let clean_pass_attempts_without_runtime = rows
                .iter()
                .filter(|row| {
                    row.reference_metrics.clean_pass == Some(true)
                        && row.reference_metrics.per_board_rollout_time_secs.is_none()
                })
                .count();

            PcbWorldTimeBudgetScore {
                time_budget_seconds: budget,
                attempts_total: rows.len(),
                attempts_with_runtime,
                attempts_without_runtime,
                clean_passes_within_budget,
                clean_pass_attempts_over_budget,
                clean_pass_attempts_without_runtime,
                clean_pass_rate_over_all_attempts: clean_passes_within_budget as f64
                    / rows.len() as f64,
                clean_pass_rate_over_timed_attempts: if attempts_with_runtime == 0 {
                    None
                } else {
                    Some(clean_passes_within_budget as f64 / attempts_with_runtime as f64)
                },
            }
        })
        .collect()
}

fn summarize(rows: &[PcbWorldRolloutRow]) -> PcbWorldCsvSummary {
    let success_values: Vec<bool> = rows
        .iter()
        .filter_map(|row| row.reference_metrics.success)
        .collect();
    let clean_values: Vec<bool> = rows
        .iter()
        .filter_map(|row| row.reference_metrics.clean_pass)
        .collect();
    let routability: Vec<f64> = rows
        .iter()
        .filter_map(|row| row.reference_metrics.routability)
        .collect();
    let error_counts: Vec<u32> = rows
        .iter()
        .filter_map(|row| row.reference_metrics.drv_errors_only_count)
        .collect();
    let promoted_counts: Vec<u32> = rows
        .iter()
        .filter_map(|row| row.reference_metrics.drv_errors_and_promoted_count)
        .collect();
    let policies: BTreeSet<String> = rows
        .iter()
        .filter_map(|row| {
            match (
                row.reference_metrics.reward_config.as_deref(),
                row.reference_metrics.check_angle,
            ) {
                (Some(config), Some(angle))
                    if row.reference_metrics.eval_status.as_deref() == Some("ok") =>
                {
                    Some(format!("{config}|check_angle={angle}"))
                }
                _ => None,
            }
        })
        .collect();

    PcbWorldCsvSummary {
        rows_attempted: rows.len(),
        unique_boards: rows
            .iter()
            .map(|row| row.board_id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        rows_completed: rows.iter().filter(|row| row.status == "completed").count(),
        rows_crashed: rows.iter().filter(|row| row.status == "crashed").count(),
        rows_truncated: rows.iter().filter(|row| row.status == "truncated").count(),
        rows_skipped: rows
            .iter()
            .filter(|row| row.status.starts_with("skipped"))
            .count(),
        rows_with_success_metric: success_values.len(),
        rows_with_clean_pass_metric: clean_values.len(),
        success_attempts: success_values.iter().filter(|value| **value).count(),
        clean_pass_attempts: clean_values.iter().filter(|value| **value).count(),
        success_rate_over_all_rows: success_values.iter().filter(|value| **value).count() as f64
            / rows.len() as f64,
        clean_pass_rate_over_all_rows: clean_values.iter().filter(|value| **value).count() as f64
            / rows.len() as f64,
        success_rate_over_metric_reporting_rows: if success_values.is_empty() {
            None
        } else {
            Some(
                success_values.iter().filter(|value| **value).count() as f64
                    / success_values.len() as f64,
            )
        },
        clean_pass_rate_over_metric_reporting_rows: if clean_values.is_empty() {
            None
        } else {
            Some(
                clean_values.iter().filter(|value| **value).count() as f64
                    / clean_values.len() as f64,
            )
        },
        rows_with_routability: routability.len(),
        mean_routability_over_recorded_rows: mean(&routability),
        rows_with_error_only_drc_count: error_counts.len(),
        error_only_drc_total: error_counts.iter().map(|value| u64::from(*value)).sum(),
        rows_with_promoted_drc_count: promoted_counts.len(),
        error_and_promoted_drc_total: promoted_counts.iter().map(|value| u64::from(*value)).sum(),
        rows_missing_reward_config: rows
            .iter()
            .filter(|row| row.reference_metrics.reward_config.is_none())
            .count(),
        rows_missing_check_angle: rows
            .iter()
            .filter(|row| row.reference_metrics.check_angle.is_none())
            .count(),
        scoring_policy_variants: policies.into_iter().collect(),
        potential_gain_inconsistency_rows: rows
            .iter()
            .filter(|row| row.reference_metrics.potential_gain_consistent == Some(false))
            .count(),
        rows_with_serialized_drc_violations: rows
            .iter()
            .filter(|row| row.reference_metrics.drv_violations.is_some())
            .count(),
    }
}

fn validate_violation_counts(
    records: &[PcbWorldDrcViolation],
    error_only: Option<u32>,
    error_and_promoted: Option<u32>,
    row_number: usize,
) -> Result<(), String> {
    if records.iter().any(|record| {
        !record.x_mm.is_finite()
            || !record.y_mm.is_finite()
            || record.severity_label.trim().is_empty()
            || record.error_type.trim().is_empty()
    }) {
        return Err(format!(
            "PCBWorld CSV row {row_number}: DRC violation list contains invalid fields"
        ));
    }
    let actual_error_only = records.iter().filter(|record| record.is_error).count() as u32;
    let actual_error_and_promoted =
        records.iter().filter(|record| record.is_promoted).count() as u32;
    if error_only.is_some_and(|value| value != actual_error_only) {
        return Err(format!("PCBWorld CSV row {row_number}: serialized DRC violation list disagrees with error-only count"));
    }
    if error_and_promoted.is_some_and(|value| value != actual_error_and_promoted) {
        return Err(format!("PCBWorld CSV row {row_number}: serialized DRC violation list disagrees with promoted-warning count"));
    }
    Ok(())
}

fn required_text(
    record: &StringRecord,
    columns: &BTreeMap<String, usize>,
    name: &str,
    row_number: usize,
) -> Result<String, String> {
    optional_text(record, columns, name)
        .ok_or_else(|| format!("PCBWorld CSV row {row_number}: {name} must not be empty"))
}

fn optional_text(
    record: &StringRecord,
    columns: &BTreeMap<String, usize>,
    name: &str,
) -> Option<String> {
    let value = columns
        .get(name)
        .and_then(|index| record.get(*index))?
        .trim();
    if is_missing(value) {
        None
    } else {
        Some(value.to_string())
    }
}

fn required_u32(
    record: &StringRecord,
    columns: &BTreeMap<String, usize>,
    name: &str,
    row_number: usize,
) -> Result<u32, String> {
    optional_u32(record, columns, name, row_number)?
        .ok_or_else(|| format!("PCBWorld CSV row {row_number}: {name} must not be empty"))
}

fn optional_u32(
    record: &StringRecord,
    columns: &BTreeMap<String, usize>,
    name: &str,
    row_number: usize,
) -> Result<Option<u32>, String> {
    parse_optional_u32(
        columns.get(name).and_then(|index| record.get(*index)),
        name,
        row_number,
    )
}

fn optional_i64(
    record: &StringRecord,
    columns: &BTreeMap<String, usize>,
    name: &str,
    row_number: usize,
) -> Result<Option<i64>, String> {
    let Some(value) = columns.get(name).and_then(|index| record.get(*index)) else {
        return Ok(None);
    };
    if is_missing(value) {
        return Ok(None);
    }
    match value.trim().parse::<i64>() {
        Ok(parsed) => Ok(Some(parsed)),
        Err(_) => {
            let parsed = value.trim().parse::<f64>().map_err(|error| {
                format!("PCBWorld CSV row {row_number}: {name} integer parse failed for {value:?}: {error}")
            })?;
            if !parsed.is_finite()
                || parsed.fract() != 0.0
                || parsed < i64::MIN as f64
                || parsed > i64::MAX as f64
            {
                return Err(format!(
                    "PCBWorld CSV row {row_number}: {name} must be an integer, got {value:?}"
                ));
            }
            Ok(Some(parsed as i64))
        }
    }
}

fn parse_optional_bool(
    value: Option<&str>,
    name: &str,
    row_number: usize,
) -> Result<Option<bool>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_ascii_lowercase();
    if is_missing(&value) {
        return Ok(None);
    }
    match value.as_str() {
        "true" | "1" => Ok(Some(true)),
        "false" | "0" => Ok(Some(false)),
        _ => Err(format!(
            "PCBWorld CSV row {row_number}: {name} boolean parse failed for {value:?}"
        )),
    }
}

fn parse_optional_u32(
    value: Option<&str>,
    name: &str,
    row_number: usize,
) -> Result<Option<u32>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if is_missing(value) {
        return Ok(None);
    }
    let parsed = value.parse::<f64>().map_err(|error| {
        format!("PCBWorld CSV row {row_number}: {name} numeric parse failed for {value:?}: {error}")
    })?;
    if !parsed.is_finite() || parsed < 0.0 || parsed.fract() != 0.0 || parsed > u32::MAX as f64 {
        return Err(format!(
            "PCBWorld CSV row {row_number}: {name} must be a non-negative integer, got {value:?}"
        ));
    }
    Ok(Some(parsed as u32))
}

fn parse_optional_f64(
    value: Option<&str>,
    name: &str,
    row_number: usize,
) -> Result<Option<f64>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if is_missing(value) {
        return Ok(None);
    }
    let parsed = value.parse::<f64>().map_err(|error| {
        format!("PCBWorld CSV row {row_number}: {name} numeric parse failed for {value:?}: {error}")
    })?;
    if !parsed.is_finite() {
        return Err(format!(
            "PCBWorld CSV row {row_number}: {name} must be finite, got {value:?}"
        ));
    }
    Ok(Some(parsed))
}

fn nonnegative_f64(
    value: Option<&str>,
    name: &str,
    row_number: usize,
) -> Result<Option<f64>, String> {
    let parsed = parse_optional_f64(value, name, row_number)?;
    if parsed.is_some_and(|number| number < 0.0) {
        return Err(format!(
            "PCBWorld CSV row {row_number}: {name} cannot be negative"
        ));
    }
    Ok(parsed)
}

fn is_missing(value: &str) -> bool {
    let value = value.trim();
    value.is_empty()
        || matches!(
            value.to_ascii_lowercase().as_str(),
            "nan" | "none" | "null" | "na"
        )
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn is_commit_sha(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
      "schema":"pcb-route-dataset-manifest-v0.1",
      "benchmark_id":"pcbworld",
      "benchmark_version":"v1.1.1",
      "dataset_id":"d3-pilot",
      "split_id":"easy.test",
      "evidence_class":"external_benchmark",
      "source_url":"https://github.com/LGAI-Research/PCBWorld",
      "source_split_config_sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "source_split_commit_sha":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
      "source_split_name":"easy.test",
      "intake_metadata_sha256":"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
      "boards":[
        {
          "board_id":"board-a",
          "input_board_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          "required_nets":2,
          "license":"MIT",
          "source_url":"https://github.com/example/board-a",
          "relative_path":"boards/board-a.kicad_pcb"
        },
        {
          "board_id":"board-b",
          "input_board_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
          "required_nets":3,
          "license":"MIT",
          "source_url":"https://github.com/example/board-b",
          "relative_path":"boards/board-b.kicad_pcb"
        }
      ]
    }"#;

    const HEADER: &str = "board_id,rollout_idx,seed,status,success,clean_pass,routability,wirelength_mm,via_count,track_count,drv_errors_only_count,drv_errors_and_promoted_count,final_potential,initial_potential,potential_gain,reward_config,check_angle,eval_status,drv_violations,per_board_rollout_time\n";

    const CSV: &str = r#"board_id,rollout_idx,seed,status,success,clean_pass,routability,wirelength_mm,via_count,track_count,drv_errors_only_count,drv_errors_and_promoted_count,final_potential,initial_potential,potential_gain,reward_config,check_angle,eval_status,drv_violations,per_board_rollout_time
board-a,0,17,completed,true,true,1.0,10.0,0,8,0,1,2.0,1.0,1.0,pcbworld_reward,45,ok,"[{""severity_label"":""warning"",""severity"":1,""error_code"":12,""error_type"":""tracks_crossing"",""x_mm"":1.0,""y_mm"":2.0,""layer"":0,""net_names"":[""GND""],""is_error"":false,""is_promoted"":true}]",30
board-a,1,18,completed,false,false,0.5,20.0,2,15,1,2,0.0,1.0,-1.0,pcbworld_reward,45,ok,"",200
board-b,0,17,completed,true,true,1.0,5.0,0,3,0,0,1.0,0.0,1.0,pcbworld_reward,45,ok,"[]",240
board-b,1,19,crashed,,,,,,,,,,,,,,,,
"#;

    const EVALUATOR_HEAD: &str = "ffffffffffffffffffffffffffffffffffffffff";

    #[test]
    fn imports_published_metric_semantics_and_keeps_promoted_warnings_separate() {
        let receipt =
            import_pcbworld_per_rollout_csv(CSV.as_bytes(), MANIFEST.as_bytes(), EVALUATOR_HEAD)
                .expect("PCBWorld metrics CSV import");
        assert_eq!(receipt.schema, PCBWORLD_CSV_RECEIPT_SCHEMA);
        assert_eq!(receipt.row_count, 4);
        assert_eq!(receipt.source_board_count, 2);
        assert_eq!(receipt.summary.unique_boards, 2);
        assert_eq!(receipt.summary.success_attempts, 2);
        assert_eq!(receipt.summary.clean_pass_attempts, 2);
        assert_eq!(receipt.summary.rows_crashed, 1);
        assert_eq!(receipt.summary.rows_with_success_metric, 3);
        assert_eq!(receipt.summary.rows_with_clean_pass_metric, 3);
        assert!((receipt.summary.success_rate_over_all_rows - 0.5).abs() < 1e-9);
        assert!((receipt.summary.clean_pass_rate_over_all_rows - 0.5).abs() < 1e-9);
        assert!(
            (receipt
                .summary
                .success_rate_over_metric_reporting_rows
                .unwrap()
                - 2.0 / 3.0)
                .abs()
                < 1e-9
        );
        assert!(
            (receipt
                .summary
                .clean_pass_rate_over_metric_reporting_rows
                .unwrap()
                - 2.0 / 3.0)
                .abs()
                < 1e-9
        );
        assert_eq!(receipt.summary.error_only_drc_total, 1);
        assert_eq!(receipt.summary.error_and_promoted_drc_total, 3);
        assert_eq!(
            receipt.summary.scoring_policy_variants,
            vec!["pcbworld_reward|check_angle=45"]
        );

        let promoted_only = &receipt.rows[0].reference_metrics;
        assert_eq!(promoted_only.success, Some(true));
        assert_eq!(promoted_only.clean_pass, Some(true));
        assert_eq!(promoted_only.drv_errors_only_count, Some(0));
        assert_eq!(promoted_only.drv_errors_and_promoted_count, Some(1));
        assert_eq!(promoted_only.drv_violations.as_ref().unwrap().len(), 1);
        assert_eq!(
            promoted_only.drv_violations.as_ref().unwrap()[0].error_code,
            12
        );
        assert_eq!(promoted_only.potential_gain_consistent, Some(true));
    }

    #[test]
    fn time_budget_sweep_preserves_all_attempts_and_requires_known_runtime() {
        let receipt =
            import_pcbworld_per_rollout_csv(CSV.as_bytes(), MANIFEST.as_bytes(), EVALUATOR_HEAD)
                .expect("PCBWorld metrics CSV import");
        assert_eq!(
            receipt.time_budget_sweep.iter().map(|score| score.time_budget_seconds).collect::<Vec<_>>(),
            vec![30, 60, 180, 300, 1800]
        );
        let thirty = receipt.time_budget_sweep.iter().find(|score| score.time_budget_seconds == 30).unwrap();
        assert_eq!(thirty.attempts_total, 4);
        assert_eq!(thirty.attempts_with_runtime, 3);
        assert_eq!(thirty.attempts_without_runtime, 1);
        assert_eq!(thirty.clean_passes_within_budget, 1);
        assert_eq!(thirty.clean_pass_attempts_over_budget, 1);
        assert_eq!(thirty.clean_pass_attempts_without_runtime, 0);
        assert!((thirty.clean_pass_rate_over_all_attempts - 0.25).abs() < 1e-9);
        assert!((thirty.clean_pass_rate_over_timed_attempts.unwrap() - 1.0 / 3.0).abs() < 1e-9);

        let three_minutes = receipt.time_budget_sweep.iter().find(|score| score.time_budget_seconds == 180).unwrap();
        assert_eq!(three_minutes.clean_passes_within_budget, 1);
        let five_minutes = receipt.time_budget_sweep.iter().find(|score| score.time_budget_seconds == 300).unwrap();
        assert_eq!(five_minutes.clean_passes_within_budget, 2);

        // A source row that claims clean but has no runtime remains in every
        // denominator and must not be credited as a time-bounded pass.
        let untimed_clean = CSV.replace(",240\n", ",\n");
        let untimed = import_pcbworld_per_rollout_csv(
            untimed_clean.as_bytes(),
            MANIFEST.as_bytes(),
            EVALUATOR_HEAD,
        )
        .expect("untimed source row remains importable");
        assert!(untimed.time_budget_sweep.iter().all(|score| score.attempts_total == 4));
        assert!(untimed.time_budget_sweep.iter().all(|score| score.clean_pass_attempts_without_runtime == 1));
        assert!(untimed.time_budget_sweep.iter().all(|score| score.clean_passes_within_budget == 1));
    }

    #[test]
    fn rejects_clean_pass_that_contradicts_upstream_error_only_definition() {
        let tampered = CSV.replace(
            "board-a,0,17,completed,true,true,1.0,10.0,0,8,0,1,2.0,1.0,1.0,pcbworld_reward",
            "board-a,0,17,completed,true,true,1.0,10.0,0,8,1,1,2.0,1.0,1.0,pcbworld_reward",
        );
        assert!(import_pcbworld_per_rollout_csv(
            tampered.as_bytes(),
            MANIFEST.as_bytes(),
            EVALUATOR_HEAD,
        )
        .unwrap_err()
        .contains("clean_pass=true contradicts PCBWorld definition"));
    }

    #[test]
    fn rejects_policy_mixing_and_missing_manifest_boards() {
        let mixed_policy = CSV.replacen(
            "pcbworld_reward,45,ok,\"\"",
            "alternate_reward,45,ok,\"\"",
            1,
        );
        assert!(import_pcbworld_per_rollout_csv(
            mixed_policy.as_bytes(),
            MANIFEST.as_bytes(),
            EVALUATOR_HEAD,
        )
        .unwrap_err()
        .contains("mixes DRC scoring policies"));

        let one_board_only = format!(
            "{HEADER}{}",
            CSV.lines()
                .skip(1)
                .filter(|line| !line.starts_with("board-b,"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert!(import_pcbworld_per_rollout_csv(
            one_board_only.as_bytes(),
            MANIFEST.as_bytes(),
            EVALUATOR_HEAD,
        )
        .unwrap_err()
        .contains("omits 1 pinned board"));
    }

    #[test]
    fn rejects_invalid_routability_duplicate_attempts_and_schema_drift() {
        let invalid_range = CSV.replace(",1.0,10.0,0,8,0,1,", ",1.2,10.0,0,8,0,1,");
        assert!(import_pcbworld_per_rollout_csv(
            invalid_range.as_bytes(),
            MANIFEST.as_bytes(),
            EVALUATOR_HEAD,
        )
        .unwrap_err()
        .contains("routability must be within [0,1]"));

        let lines: Vec<&str> = CSV.lines().collect();
        let duplicate = format!("{HEADER}{}\n{}\n{}\n", lines[1], lines[1], lines[3]);
        assert!(import_pcbworld_per_rollout_csv(
            duplicate.as_bytes(),
            MANIFEST.as_bytes(),
            EVALUATOR_HEAD,
        )
        .unwrap_err()
        .contains("duplicate PCBWorld attempt key"));

        let missing_header =
            "board_id,rollout_idx,status\nboard-a,0,completed\nboard-b,0,completed\n";
        assert!(import_pcbworld_per_rollout_csv(
            missing_header.as_bytes(),
            MANIFEST.as_bytes(),
            EVALUATOR_HEAD,
        )
        .unwrap_err()
        .contains("missing required columns"));
    }
}
