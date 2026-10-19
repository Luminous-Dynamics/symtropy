// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::env;
use std::fs;
use std::process;

const EXPECTED_SHA256: &str = "0199bc69cae5e74be3d668244f708da78450c669be641d254b5ca6aa712dda56";
const EXPECTED_BYTES: usize = 3_346_122;
const EXPECTED_ROWS: usize = 2_054;
const EXPECTED_COLUMNS: usize = 154;
const EXPECTED_MISSING_CELLS: usize = 1_142;
const EXPECTED_TOOLS: [u32; 13] = [3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16];
const FEATURES: [&str; 3] = ["AE_RMS", "F_c_RMS", "F_p_RMS"];

#[derive(Debug)]
struct Row {
    tool: u32,
    values: Vec<Option<f64>>,
}

#[derive(Debug)]
struct Dataset {
    headers: Vec<String>,
    rows: Vec<Row>,
    missing_cells: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct ErrorAccumulator {
    rows: usize,
    absolute_error_sum: f64,
    squared_error_sum: f64,
}

impl ErrorAccumulator {
    fn push(&mut self, actual: f64, predicted: f64) -> Result<(), String> {
        let error = predicted - actual;
        if !error.is_finite() {
            return Err("prediction produced a non-finite error".to_owned());
        }
        self.rows += 1;
        self.absolute_error_sum += error.abs();
        self.squared_error_sum += error * error;
        Ok(())
    }

    fn finish(self) -> Result<MetricSummary, String> {
        if self.rows == 0 {
            return Err("cannot summarize a metric with zero observations".to_owned());
        }
        let count = self.rows as f64;
        let mae = self.absolute_error_sum / count;
        let rmse = (self.squared_error_sum / count).sqrt();
        if !mae.is_finite() || !rmse.is_finite() {
            return Err("metric computation produced a non-finite result".to_owned());
        }
        Ok(MetricSummary {
            rows: self.rows,
            mae,
            rmse,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct MetricSummary {
    rows: usize,
    mae: f64,
    rmse: f64,
}

#[derive(Debug)]
struct ModelResult {
    intercept: f64,
    slope: f64,
    pooled: MetricSummary,
    macro_mae: f64,
    macro_rmse: f64,
    per_tool: BTreeMap<u32, MetricSummary>,
}

#[derive(Debug)]
struct ToolFold {
    held_out_tool: u32,
    training_tools: BTreeSet<u32>,
    training_rows: usize,
    test_rows: usize,
    models: BTreeMap<String, ModelResult>,
}

#[derive(Debug, Default)]
struct TransferAccumulator {
    pooled: ErrorAccumulator,
    per_tool: BTreeMap<u32, MetricSummary>,
}

impl TransferAccumulator {
    fn record(&mut self, tool: u32, result: &ModelResult) -> Result<(), String> {
        let metric = *result
            .per_tool
            .get(&tool)
            .ok_or_else(|| format!("fold omitted its held-out tool {tool}"))?;
        if self.per_tool.insert(tool, metric).is_some() {
            return Err(format!("tool {tool} was evaluated more than once"));
        }
        self.pooled.rows = self
            .pooled
            .rows
            .checked_add(result.pooled.rows)
            .ok_or_else(|| "transfer metric row count overflow".to_owned())?;
        self.pooled.absolute_error_sum += result.pooled.mae * result.pooled.rows as f64;
        self.pooled.squared_error_sum +=
            result.pooled.rmse * result.pooled.rmse * result.pooled.rows as f64;
        if !self.pooled.absolute_error_sum.is_finite() || !self.pooled.squared_error_sum.is_finite()
        {
            return Err("transfer metric aggregation became non-finite".to_owned());
        }
        Ok(())
    }

    fn json(&self) -> Result<String, String> {
        let pooled = self.pooled.finish()?;
        if self.per_tool.is_empty() {
            return Err("cannot aggregate leave-one-tool-out without folds".to_owned());
        }
        let count = self.per_tool.len() as f64;
        let macro_mae = self.per_tool.values().map(|m| m.mae).sum::<f64>() / count;
        let macro_rmse = self.per_tool.values().map(|m| m.rmse).sum::<f64>() / count;
        if !macro_mae.is_finite() || !macro_rmse.is_finite() {
            return Err("macro-tool transfer metric is non-finite".to_owned());
        }
        let (worst_tool, worst_metric) = self
            .per_tool
            .iter()
            .max_by(|a, b| a.1.mae.total_cmp(&b.1.mae))
            .ok_or_else(|| "cannot identify worst tool without per-tool metrics".to_owned())?;
        let per_tool: Vec<String> = self
            .per_tool
            .iter()
            .map(|(tool, metric)| format!("\"{tool}\":{}", metric_json(*metric)))
            .collect();
        Ok(format!(
            "{{\"pooled\":{},\"macro_tool\":{{\"mae\":{:.12},\"rmse\":{:.12}}},\"worst_tool\":{{\"tool\":{},\"metrics\":{}}},\"per_tool\":{{{}}}}}",
            metric_json(pooled),
            macro_mae,
            macro_rmse,
            worst_tool,
            metric_json(*worst_metric),
            per_tool.join(",")
        ))
    }
}

fn parse_csv(text: &str) -> Result<Dataset, String> {
    let mut lines = text.lines();
    let header_line = lines
        .next()
        .ok_or_else(|| "source file is empty".to_owned())?;

    // This deliberately narrow parser admits the pinned numeric feature table.
    // A future quoted-field schema must be reviewed rather than guessed.
    if header_line.contains('"') {
        return Err("quoted CSV headers are outside the source contract".to_owned());
    }
    let headers: Vec<String> = header_line.split(',').map(str::to_owned).collect();
    if headers.iter().any(String::is_empty) {
        return Err("header contains an empty column name".to_owned());
    }

    let mut unique_headers = BTreeSet::new();
    for header in &headers {
        if !unique_headers.insert(header.as_str()) {
            return Err(format!("duplicate column name: {header}"));
        }
    }

    let tool_index = column_index(&headers, "tool")?;
    let mut required_indices = BTreeSet::from([tool_index, column_index(&headers, "Vb")?]);
    for feature in FEATURES {
        required_indices.insert(column_index(&headers, feature)?);
    }

    let mut rows = Vec::new();
    let mut missing_cells = 0;
    for (offset, line) in lines.enumerate() {
        let line_number = offset + 2;
        if line.is_empty() {
            return Err(format!("blank row at line {line_number}"));
        }
        if line.contains('"') {
            return Err(format!(
                "quoted cells are outside the source contract at line {line_number}"
            ));
        }
        let cells: Vec<&str> = line.split(',').collect();
        if cells.len() != headers.len() {
            return Err(format!(
                "row {line_number} has {} cells; expected {}",
                cells.len(),
                headers.len()
            ));
        }

        let tool = cells[tool_index]
            .parse::<u32>()
            .map_err(|_| format!("invalid integer tool ID at line {line_number}"))?;
        let mut values = Vec::with_capacity(cells.len());
        for (column, cell) in cells.iter().enumerate() {
            if cell.is_empty() {
                if required_indices.contains(&column) {
                    return Err(format!(
                        "missing required value at line {line_number}, column {}",
                        headers[column]
                    ));
                }
                missing_cells += 1;
                values.push(None);
                continue;
            }
            let value = cell.parse::<f64>().map_err(|_| {
                format!(
                    "non-numeric value at line {line_number}, column {}",
                    headers[column]
                )
            })?;
            if !value.is_finite() {
                return Err(format!(
                    "non-finite value at line {line_number}, column {}",
                    headers[column]
                ));
            }
            values.push(Some(value));
        }
        rows.push(Row { tool, values });
    }

    if rows.is_empty() {
        return Err("source has a header but no data rows".to_owned());
    }
    Ok(Dataset {
        headers,
        rows,
        missing_cells,
    })
}

fn value(row: &Row, index: usize) -> Result<f64, String> {
    row.values
        .get(index)
        .copied()
        .flatten()
        .ok_or_else(|| format!("required numeric value is missing at column {index}"))
}

fn column_index(headers: &[String], name: &str) -> Result<usize, String> {
    headers
        .iter()
        .position(|header| header == name)
        .ok_or_else(|| format!("required column is missing: {name}"))
}

fn validate_locked_source(dataset: &Dataset, source_bytes: &[u8]) -> Result<(), String> {
    if source_bytes.len() != EXPECTED_BYTES {
        return Err(format!(
            "source byte count mismatch: got {}, expected {}",
            source_bytes.len(),
            EXPECTED_BYTES
        ));
    }
    let actual_sha = sha256_hex(source_bytes);
    if actual_sha != EXPECTED_SHA256 {
        return Err(format!(
            "source SHA-256 mismatch: got {actual_sha}, expected {EXPECTED_SHA256}"
        ));
    }
    if dataset.rows.len() != EXPECTED_ROWS {
        return Err(format!(
            "row count mismatch: got {}, expected {}",
            dataset.rows.len(),
            EXPECTED_ROWS
        ));
    }
    if dataset.headers.len() != EXPECTED_COLUMNS {
        return Err(format!(
            "column count mismatch: got {}, expected {}",
            dataset.headers.len(),
            EXPECTED_COLUMNS
        ));
    }
    if dataset.missing_cells != EXPECTED_MISSING_CELLS {
        return Err(format!(
            "missing cell count mismatch: got {}, expected {}",
            dataset.missing_cells, EXPECTED_MISSING_CELLS
        ));
    }

    let actual_tools: BTreeSet<u32> = dataset.rows.iter().map(|row| row.tool).collect();
    let expected_tools: BTreeSet<u32> = EXPECTED_TOOLS.into_iter().collect();
    if actual_tools != expected_tools {
        return Err(format!(
            "tool identity set mismatch: got {:?}, expected {:?}",
            actual_tools, expected_tools
        ));
    }

    let text = std::str::from_utf8(source_bytes)
        .map_err(|error| format!("source is not valid UTF-8: {error}"))?;
    let line_count = text.lines().count();
    if line_count != EXPECTED_ROWS + 1 {
        return Err(format!(
            "physical line count mismatch: got {line_count}, expected {}",
            EXPECTED_ROWS + 1
        ));
    }
    let unique_rows: HashSet<&str> = text.lines().skip(1).collect();
    if unique_rows.len() != EXPECTED_ROWS {
        return Err(format!(
            "duplicate source rows detected: {} unique rows among {}",
            unique_rows.len(),
            EXPECTED_ROWS
        ));
    }
    Ok(())
}

fn source_split(tools: &BTreeSet<u32>) -> (BTreeSet<u32>, BTreeSet<u32>) {
    let mut train = BTreeSet::new();
    let mut test = BTreeSet::new();
    for (index, tool) in tools.iter().copied().enumerate() {
        // Numeric IDs are sorted; every fourth identity from index zero is held
        // out. Records are grouped by tool, never randomly split by row.
        if index.is_multiple_of(4) {
            test.insert(tool);
        } else {
            train.insert(tool);
        }
    }
    (train, test)
}

fn tool_counts(rows: &[&Row]) -> BTreeMap<u32, usize> {
    let mut counts = BTreeMap::new();
    for row in rows {
        *counts.entry(row.tool).or_insert(0) += 1;
    }
    counts
}

fn fit_tool_balanced_constant(rows: &[&Row], target_index: usize) -> Result<f64, String> {
    let mut sums: BTreeMap<u32, (f64, usize)> = BTreeMap::new();
    for row in rows {
        let entry = sums.entry(row.tool).or_insert((0.0, 0));
        entry.0 += value(row, target_index)?;
        entry.1 += 1;
    }
    if sums.is_empty() {
        return Err("cannot fit baseline without training rows".to_owned());
    }

    let mean_of_tool_means = sums
        .values()
        .map(|(sum, count)| sum / *count as f64)
        .sum::<f64>()
        / sums.len() as f64;
    if !mean_of_tool_means.is_finite() {
        return Err("constant baseline fit is non-finite".to_owned());
    }
    Ok(mean_of_tool_means)
}

fn fit_tool_balanced_linear(
    rows: &[&Row],
    counts: &BTreeMap<u32, usize>,
    feature_index: usize,
    target_index: usize,
) -> Result<(f64, f64), String> {
    let mut weight_sum = 0.0;
    let mut x_sum = 0.0;
    let mut y_sum = 0.0;
    for row in rows {
        let count = counts
            .get(&row.tool)
            .ok_or_else(|| "training tool has no weight denominator".to_owned())?;
        let weight = 1.0 / *count as f64;
        weight_sum += weight;
        x_sum += weight * value(row, feature_index)?;
        y_sum += weight * value(row, target_index)?;
    }
    if weight_sum <= 0.0 || !weight_sum.is_finite() {
        return Err("invalid weighted training denominator".to_owned());
    }

    let x_mean = x_sum / weight_sum;
    let y_mean = y_sum / weight_sum;
    let mut xx = 0.0;
    let mut xy = 0.0;
    for row in rows {
        let count = counts
            .get(&row.tool)
            .ok_or_else(|| "training tool has no weight denominator".to_owned())?;
        let weight = 1.0 / *count as f64;
        let dx = value(row, feature_index)? - x_mean;
        let dy = value(row, target_index)? - y_mean;
        xx += weight * dx * dx;
        xy += weight * dx * dy;
    }
    if !xx.is_finite() || xx <= f64::EPSILON {
        return Err(format!(
            "feature column {feature_index} has no usable training variation"
        ));
    }

    let slope = xy / xx;
    let intercept = y_mean - slope * x_mean;
    if !slope.is_finite() || !intercept.is_finite() {
        return Err("linear fit produced non-finite coefficients".to_owned());
    }
    Ok((intercept, slope))
}

fn evaluate_model(
    dataset: &Dataset,
    held_out_tools: &BTreeSet<u32>,
    target_index: usize,
    feature_index: Option<usize>,
    intercept: f64,
    slope: f64,
) -> Result<ModelResult, String> {
    let mut pooled = ErrorAccumulator::default();
    let mut by_tool: BTreeMap<u32, ErrorAccumulator> = BTreeMap::new();
    for row in &dataset.rows {
        if !held_out_tools.contains(&row.tool) {
            continue;
        }
        let x = feature_index.map_or(Ok(0.0), |index| value(row, index))?;
        let prediction = intercept + slope * x;
        let actual = value(row, target_index)?;
        pooled.push(actual, prediction)?;
        by_tool
            .entry(row.tool)
            .or_default()
            .push(actual, prediction)?;
    }

    let pooled = pooled.finish()?;
    let mut per_tool = BTreeMap::new();
    for (tool, errors) in by_tool {
        per_tool.insert(tool, errors.finish()?);
    }
    if per_tool.is_empty() {
        return Err("held-out split contains no tools".to_owned());
    }

    let divisor = per_tool.len() as f64;
    let macro_mae = per_tool.values().map(|metric| metric.mae).sum::<f64>() / divisor;
    let macro_rmse = per_tool.values().map(|metric| metric.rmse).sum::<f64>() / divisor;
    Ok(ModelResult {
        intercept,
        slope,
        pooled,
        macro_mae,
        macro_rmse,
        per_tool,
    })
}

fn fit_leave_one_tool_out(dataset: &Dataset) -> Result<Vec<ToolFold>, String> {
    let tool_ids: BTreeSet<u32> = dataset.rows.iter().map(|row| row.tool).collect();
    if tool_ids.len() < 2 {
        return Err("leave-one-tool-out requires at least two distinct tools".to_owned());
    }
    let target_index = column_index(&dataset.headers, "Vb")?;
    let mut folds = Vec::with_capacity(tool_ids.len());

    for held_out_tool in tool_ids.iter().copied() {
        let test_tools = BTreeSet::from([held_out_tool]);
        let training_tools: BTreeSet<u32> = tool_ids.difference(&test_tools).copied().collect();
        if training_tools.is_empty() || !training_tools.is_disjoint(&test_tools) {
            return Err("leave-one-tool-out produced an invalid group split".to_owned());
        }

        let training_rows: Vec<&Row> = dataset
            .rows
            .iter()
            .filter(|row| training_tools.contains(&row.tool))
            .collect();
        let test_rows = dataset
            .rows
            .iter()
            .filter(|row| row.tool == held_out_tool)
            .count();
        if training_rows.is_empty() || test_rows == 0 {
            return Err(format!(
                "leave-one-tool-out fold for tool {held_out_tool} has an empty side"
            ));
        }
        let training_counts = tool_counts(&training_rows);
        let mut models = BTreeMap::new();

        let constant = fit_tool_balanced_constant(&training_rows, target_index)?;
        let constant_result =
            evaluate_model(dataset, &test_tools, target_index, None, constant, 0.0)?;
        models.insert("constant_train_tool_balanced".to_owned(), constant_result);

        // This feature set and estimator are frozen before inspecting the
        // leave-one-tool-out results: no fold-specific feature selection.
        for feature in FEATURES {
            let feature_index = column_index(&dataset.headers, feature)?;
            let (intercept, slope) = fit_tool_balanced_linear(
                &training_rows,
                &training_counts,
                feature_index,
                target_index,
            )?;
            let result = evaluate_model(
                dataset,
                &test_tools,
                target_index,
                Some(feature_index),
                intercept,
                slope,
            )?;
            models.insert(format!("ols_{feature}"), result);
        }

        for result in models.values() {
            if result.per_tool.len() != 1 || !result.per_tool.contains_key(&held_out_tool) {
                return Err(format!(
                    "model result for tool {held_out_tool} is not group-local"
                ));
            }
        }

        folds.push(ToolFold {
            held_out_tool,
            training_tools,
            training_rows: training_rows.len(),
            test_rows,
            models,
        });
    }

    let held_out_tools: BTreeSet<u32> = folds.iter().map(|fold| fold.held_out_tool).collect();
    if held_out_tools != tool_ids || folds.len() != tool_ids.len() {
        return Err("leave-one-tool-out did not produce exactly one fold per tool".to_owned());
    }
    Ok(folds)
}

fn model_ranking_by_mae(models: &BTreeMap<String, ModelResult>) -> Vec<String> {
    let mut ranking: Vec<(String, f64)> = models
        .iter()
        .map(|(name, result)| (name.clone(), result.pooled.mae))
        .collect();
    ranking.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    ranking.into_iter().map(|(name, _)| name).collect()
}

fn tool_transfer_diagnostics(dataset: &Dataset, source_sha: &str) -> Result<String, String> {
    let folds = fit_leave_one_tool_out(dataset)?;
    let tools: BTreeSet<u32> = dataset.rows.iter().map(|row| row.tool).collect();
    let identity = format!(
        "MFG-DATA-002-LOTO-V1\nsource_sha256={source_sha}\ntools={}\ntarget=Vb\nfeatures={}\n",
        ids_json(&tools),
        FEATURES.join(",")
    );
    let identity_sha = sha256_hex(identity.as_bytes());
    let mut aggregates: BTreeMap<String, TransferAccumulator> = BTreeMap::new();
    let mut ranking_signatures = BTreeSet::new();
    let mut fold_reports = Vec::with_capacity(folds.len());

    for fold in &folds {
        let ranking = model_ranking_by_mae(&fold.models);
        ranking_signatures.insert(ranking.join("|"));
        for (name, result) in &fold.models {
            aggregates
                .entry(name.clone())
                .or_default()
                .record(fold.held_out_tool, result)?;
        }
        let models: Vec<String> = fold
            .models
            .iter()
            .map(|(name, result)| format!("\"{name}\":{}", model_json(result)))
            .collect();
        fold_reports.push(format!(
            "{{\"held_out_tool\":{},\"training_tools\":{},\"test_tools\":[{}],\"training_rows\":{},\"test_rows\":{},\"model_ranking_by_mae\":[{}],\"models\":{{{}}}}}",
            fold.held_out_tool,
            ids_json(&fold.training_tools),
            fold.held_out_tool,
            fold.training_rows,
            fold.test_rows,
            ranking.iter().map(|name| format!("\"{name}\"")).collect::<Vec<_>>().join(","),
            models.join(",")
        ));
    }

    if folds.len() != tools.len() {
        return Err("leave-one-tool-out fold count does not match tool count".to_owned());
    }
    let mut aggregate_models = Vec::new();
    for (name, accumulator) in &aggregates {
        aggregate_models.push(format!("\"{name}\":{}", accumulator.json()?));
    }
    let tool_ids_json = ids_json(&tools);
    Ok(format!(
        "{{\"schema_version\":\"0.1.0\",\"model_id\":\"MFG-DATA-002\",\"evidence_class\":\"external_dataset_evaluation_unqualified_for_process_control\",\"source\":{{\"repository\":\"Vicomtech/dataset-machine-tool-wear\",\"commit\":\"fa7941101847541cdd31339d6481ee61d152d80f\",\"file\":\"Dataset_Machine_tool_wear.csv\",\"bytes\":{},\"sha256\":\"{}\",\"label\":\"Vb\",\"label_units\":\"not specified by source; values remain source-native\"}},\"diagnostic\":{{\"method\":\"leave-one-tool-out\",\"identity_sha256\":\"{}\",\"tools\":{},\"fold_count\":{},\"features\":[{}],\"target\":\"Vb\"}},\"folds\":[{}],\"aggregate\":{{\"models\":{{{}}},\"ranking_signature_count\":{}}},\"limits\":[\"This is a tool-group transfer diagnostic, not machine-independent validation.\",\"Vb physical units are not specified by the source README and are not inferred.\",\"Feature-table scores do not qualify raw-signal processing, sensor calibration, maintenance policy, or process control.\",\"The frozen MFG-DATA-001 four-tool holdout report is emitted separately and is unchanged by this diagnostic.\"]}}",
        EXPECTED_BYTES,
        source_sha,
        identity_sha,
        tool_ids_json,
        folds.len(),
        FEATURES
            .iter()
            .map(|feature| format!("\"{feature}\""))
            .collect::<Vec<_>>()
            .join(","),
        fold_reports.join(","),
        aggregate_models.join(","),
        ranking_signatures.len()
    ))
}

fn ids_json(ids: &BTreeSet<u32>) -> String {
    let values: Vec<String> = ids.iter().map(u32::to_string).collect();
    format!("[{}]", values.join(","))
}

fn metric_json(metric: MetricSummary) -> String {
    format!(
        "{{\"rows\":{},\"mae\":{:.12},\"rmse\":{:.12}}}",
        metric.rows, metric.mae, metric.rmse
    )
}

fn model_json(result: &ModelResult) -> String {
    let per_tool: Vec<String> = result
        .per_tool
        .iter()
        .map(|(tool, metric)| format!("\"{tool}\":{}", metric_json(*metric)))
        .collect();
    format!(
        "{{\"intercept\":{:.12},\"slope\":{:.12},\"pooled\":{},\"macro_tool\":{{\"mae\":{:.12},\"rmse\":{:.12}}},\"per_tool\":{{{}}}}}",
        result.intercept,
        result.slope,
        metric_json(result.pooled),
        result.macro_mae,
        result.macro_rmse,
        per_tool.join(",")
    )
}

fn report(dataset: &Dataset, source_sha: &str) -> Result<String, String> {
    let tool_ids: BTreeSet<u32> = dataset.rows.iter().map(|row| row.tool).collect();
    let (train_tools, test_tools) = source_split(&tool_ids);
    let train_rows: Vec<&Row> = dataset
        .rows
        .iter()
        .filter(|row| train_tools.contains(&row.tool))
        .collect();
    let train_counts = tool_counts(&train_rows);
    let target_index = column_index(&dataset.headers, "Vb")?;

    let constant = fit_tool_balanced_constant(&train_rows, target_index)?;
    let constant_result = evaluate_model(dataset, &test_tools, target_index, None, constant, 0.0)?;
    let mut models = vec![format!(
        "\"constant_train_tool_balanced\":{}",
        model_json(&constant_result)
    )];

    for feature in FEATURES {
        let feature_index = column_index(&dataset.headers, feature)?;
        let (intercept, slope) =
            fit_tool_balanced_linear(&train_rows, &train_counts, feature_index, target_index)?;
        let result = evaluate_model(
            dataset,
            &test_tools,
            target_index,
            Some(feature_index),
            intercept,
            slope,
        )?;
        models.push(format!("\"ols_{feature}\":{}", model_json(&result)));
    }

    let train_rows_count = train_rows.len();
    let test_rows_count = dataset.rows.len() - train_rows_count;
    let split_identity = format!(
        "MFG-DATA-001-SPLIT-V1\nsource_sha256={source_sha}\ntrain_tools={}\ntest_tools={}\n",
        ids_json(&train_tools),
        ids_json(&test_tools)
    );
    let split_sha = sha256_hex(split_identity.as_bytes());
    Ok(format!(
        "{{\"schema_version\":\"0.1.0\",\"model_id\":\"MFG-DATA-001\",\"evidence_class\":\"external_dataset_evaluation_unqualified_for_process_control\",\"source\":{{\"repository\":\"Vicomtech/dataset-machine-tool-wear\",\"commit\":\"fa7941101847541cdd31339d6481ee61d152d80f\",\"file\":\"Dataset_Machine_tool_wear.csv\",\"bytes\":{},\"sha256\":\"{}\",\"license_declared_in_source_file\":\"CC-BY-SA-4.0\",\"label\":\"Vb\",\"label_units\":\"not specified by source; values remain source-native\"}},\"dataset_diagnostics\":{{\"rows\":{},\"columns\":{},\"distinct_tools\":{},\"exact_duplicate_rows\":0,\"non_finite_cells\":0,\"missing_cells\":{}}},\"split\":{{\"method\":\"numeric tool IDs sorted ascending; hold out every fourth tool starting at index zero\",\"identity_sha256\":\"{}\",\"train_tools\":{},\"test_tools\":{},\"train_rows\":{},\"test_rows\":{}}},\"models\":{{{}}},\"limits\":[\"This feature-table benchmark does not validate raw-signal processing or a complete maintenance policy.\",\"Tool IDs are held out as groups; performance on four held-out tools is not evidence of machine-independent generalization.\",\"No prediction interval, maintenance command, manufacturing release, or process-control qualification is emitted.\",\"Vb physical units are not specified by the pinned source README and are not inferred here.\"]}}",
        EXPECTED_BYTES,
        source_sha,
        dataset.rows.len(),
        dataset.headers.len(),
        tool_ids.len(),
        dataset.missing_cells,
        split_sha,
        ids_json(&train_tools),
        ids_json(&test_tools),
        train_rows_count,
        test_rows_count,
        models.join(",")
    ))
}

fn sha256_hex(input: &[u8]) -> String {
    #[rustfmt::skip]
    const K: [u32; 64] = [
        0x428a2f98,
        0x71374491,
        0xb5c0fbcf,
        0xe9b5dba5,
        0x3956c25b,
        0x59f111f1,
        0x923f82a4,
        0xab1c5ed5,
        0xd807aa98,
        0x12835b01,
        0x243185be,
        0x550c7dc3,
        0x72be5d74,
        0x80deb1fe,
        0x9bdc06a7,
        0xc19bf174,
        0xe49b69c1,
        0xefbe4786,
        0x0fc19dc6,
        0x240ca1cc,
        0x2de92c6f,
        0x4a7484aa,
        0x5cb0a9dc,
        0x76f988da,
        0x983e5152,
        0xa831c66d,
        0xb00327c8,
        0xbf597fc7,
        0xc6e00bf3,
        0xd5a79147,
        0x06ca6351,
        0x14292967,
        0x27b70a85,
        0x2e1b2138,
        0x4d2c6dfc,
        0x53380d13,
        0x650a7354,
        0x766a0abb,
        0x81c2c92e,
        0x92722c85,
        0xa2bfe8a1,
        0xa81a664b,
        0xc24b8b70,
        0xc76c51a3,
        0xd192e819,
        0xd6990624,
        0xf40e3585,
        0x106aa070,
        0x19a4c116,
        0x1e376c08,
        0x2748774c,
        0x34b0bcb5,
        0x391c0cb3,
        0x4ed8aa4a,
        0x5b9cca4f,
        0x682e6ff3,
        0x748f82ee,
        0x78a5636f,
        0x84c87814,
        0x8cc70208,
        0x90befffa,
        0xa4506ceb,
        0xbef9a3f7,
        0xc67178f2,
    ];
    let mut padded = input.to_vec();
    let bit_length = (input.len() as u64).wrapping_mul(8);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_length.to_be_bytes());

    let mut state = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    for block in padded.chunks_exact(64) {
        let mut schedule = [0u32; 64];
        for (index, slot) in schedule.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *slot = u32::from_be_bytes([
                block[offset],
                block[offset + 1],
                block[offset + 2],
                block[offset + 3],
            ]);
        }
        let mut index = 16;
        while index < 64 {
            let a = schedule[index - 15];
            let b = schedule[index - 2];
            let sigma0 = a.rotate_right(7) ^ a.rotate_right(18) ^ (a >> 3);
            let sigma1 = b.rotate_right(17) ^ b.rotate_right(19) ^ (b >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(sigma0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(sigma1);
            index += 1;
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for (index, constant) in K.iter().enumerate() {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sum1)
                .wrapping_add(choose)
                .wrapping_add(*constant)
                .wrapping_add(schedule[index]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }

    state.iter().map(|value| format!("{value:08x}")).collect()
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "--help" {
        println!("Usage: toolwear-eval --data PATH [--loto]");
        println!("Default output is the frozen MFG-DATA-001 split.");
        println!("--loto emits the separate MFG-DATA-002 leave-one-tool-out diagnostic.");
        return Ok(());
    }
    let loto_mode = args.len() == 4 && args[1] == "--data" && args[3] == "--loto";
    let frozen_mode = args.len() == 3 && args[1] == "--data";
    if !frozen_mode && !loto_mode {
        return Err("usage: toolwear-eval --data PATH [--loto] (or --help)".to_owned());
    }

    let source_bytes = fs::read(&args[2])
        .map_err(|error| format!("failed to read source CSV {}: {error}", args[2]))?;
    let source_sha = sha256_hex(&source_bytes);
    if source_sha != EXPECTED_SHA256 {
        return Err(format!(
            "source SHA-256 mismatch: got {source_sha}, expected {EXPECTED_SHA256}"
        ));
    }
    if source_bytes.len() != EXPECTED_BYTES {
        return Err(format!(
            "source byte count mismatch: got {}, expected {}",
            source_bytes.len(),
            EXPECTED_BYTES
        ));
    }
    let text = std::str::from_utf8(&source_bytes)
        .map_err(|error| format!("source is not valid UTF-8: {error}"))?;
    let dataset = parse_csv(text)?;
    validate_locked_source(&dataset, &source_bytes)?;
    let output = if loto_mode {
        tool_transfer_diagnostics(&dataset, &source_sha)?
    } else {
        report(&dataset, &source_sha)?
    };
    println!("{output}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("toolwear-eval rejected input: {error}");
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_standard_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn parses_numeric_feature_fixture() {
        let text = "tool,Vb,AE_RMS,F_c_RMS,F_p_RMS\n3,1,0.5,2,4\n4,2,0.6,3,5\n";
        let dataset = parse_csv(text).expect("valid fixture should parse");
        assert_eq!(dataset.rows.len(), 2);
        assert_eq!(dataset.headers.len(), 5);
        assert_eq!(dataset.rows[0].tool, 3);
        let target = column_index(&dataset.headers, "Vb").unwrap();
        assert_eq!(dataset.rows[1].values[target], Some(2.0));
    }

    #[test]
    fn rejects_width_changes_and_non_finite_cells() {
        assert!(parse_csv("tool,Vb,AE_RMS,F_c_RMS,F_p_RMS\n3,1,2,3\n").is_err());
        assert!(parse_csv("tool,Vb,AE_RMS,F_c_RMS,F_p_RMS\n3,NaN,2,3,4\n").is_err());
        assert!(parse_csv("tool,Vb,AE_RMS,F_c_RMS,F_p_RMS\n3,1,2,3,4\n3,1,2,3\n").is_err());
    }

    #[test]
    fn allows_missing_values_only_in_unused_columns() {
        let text = "tool,Vb,AE_RMS,F_c_RMS,F_p_RMS,aux\n3,1,2,3,4,\n";
        let dataset = parse_csv(text).expect("unused source columns may be missing");
        assert_eq!(dataset.missing_cells, 1);
        assert!(parse_csv("tool,Vb,AE_RMS,F_c_RMS,F_p_RMS,aux\n3,1,2,3,,5\n").is_err());
    }

    #[test]
    fn deterministic_split_uses_whole_tools() {
        let tool_ids: BTreeSet<u32> = EXPECTED_TOOLS.into_iter().collect();
        let (train, test) = source_split(&tool_ids);
        assert_eq!(test, [3, 7, 12, 16].into_iter().collect());
        assert_eq!(
            train,
            [4, 5, 6, 9, 10, 11, 13, 14, 15].into_iter().collect()
        );
        assert!(train.is_disjoint(&test));
        for tool in tool_ids {
            assert!(train.contains(&tool) ^ test.contains(&tool));
        }
    }

    #[test]
    fn tool_balanced_linear_fit_recovers_a_known_line() {
        let text =
            "tool,Vb,AE_RMS,F_c_RMS,F_p_RMS\n3,3,1,1,10\n3,5,2,2,10\n4,7,3,3,10\n4,9,4,4,10\n";
        let dataset = parse_csv(text).expect("valid fixture should parse");
        let rows: Vec<&Row> = dataset.rows.iter().collect();
        let counts = tool_counts(&rows);
        let target = column_index(&dataset.headers, "Vb").unwrap();
        let feature = column_index(&dataset.headers, "AE_RMS").unwrap();
        let (intercept, slope) = fit_tool_balanced_linear(&rows, &counts, feature, target).unwrap();
        assert!((intercept - 1.0).abs() < 1e-10);
        assert!((slope - 2.0).abs() < 1e-10);
    }

    fn transfer_fixture() -> Dataset {
        let text = concat!(
            "tool,Vb,AE_RMS,F_c_RMS,F_p_RMS\n",
            "3,1,0.2,1,3\n",
            "3,2,0.4,2,2\n",
            "4,3,0.6,3,4\n",
            "4,5,0.8,4,5\n",
            "5,6,1.0,5,7\n",
            "5,7,1.2,6,8\n",
            "6,9,1.4,7,10\n",
            "6,10,1.6,8,11\n"
        );
        parse_csv(text).expect("transfer fixture should parse")
    }

    #[test]
    fn leave_one_tool_out_covers_each_tool_once_without_leakage() {
        let dataset = transfer_fixture();
        let folds = fit_leave_one_tool_out(&dataset).unwrap();
        let expected: BTreeSet<u32> = dataset.rows.iter().map(|row| row.tool).collect();
        let held_out: BTreeSet<u32> = folds.iter().map(|fold| fold.held_out_tool).collect();
        assert_eq!(folds.len(), expected.len());
        assert_eq!(held_out, expected);
        for fold in &folds {
            assert!(!fold.training_tools.contains(&fold.held_out_tool));
            assert_eq!(fold.training_tools.len() + 1, expected.len());
            assert_eq!(fold.training_rows + fold.test_rows, dataset.rows.len());
            assert!(fold.test_rows > 0);
            assert_eq!(fold.models.len(), FEATURES.len() + 1);
            for result in fold.models.values() {
                assert_eq!(
                    result.per_tool.keys().copied().collect::<BTreeSet<_>>(),
                    [fold.held_out_tool].into_iter().collect()
                );
            }
        }
    }

    #[test]
    fn target_shuffling_cannot_change_group_fold_assignments() {
        let mut dataset = transfer_fixture();
        let target = column_index(&dataset.headers, "Vb").unwrap();
        let before = fit_leave_one_tool_out(&dataset).unwrap();
        let before_assignment: Vec<(u32, BTreeSet<u32>, usize, usize)> = before
            .iter()
            .map(|fold| {
                (
                    fold.held_out_tool,
                    fold.training_tools.clone(),
                    fold.training_rows,
                    fold.test_rows,
                )
            })
            .collect();

        let mut labels: Vec<f64> = dataset
            .rows
            .iter()
            .map(|row| value(row, target).unwrap())
            .collect();
        labels.rotate_left(3);
        for (row, label) in dataset.rows.iter_mut().zip(labels) {
            row.values[target] = Some(label);
        }

        let after = fit_leave_one_tool_out(&dataset).unwrap();
        let after_assignment: Vec<(u32, BTreeSet<u32>, usize, usize)> = after
            .iter()
            .map(|fold| {
                (
                    fold.held_out_tool,
                    fold.training_tools.clone(),
                    fold.training_rows,
                    fold.test_rows,
                )
            })
            .collect();
        assert_eq!(before_assignment, after_assignment);
    }

    #[test]
    fn column_reordering_does_not_change_named_feature_or_label_binding() {
        let original = transfer_fixture();
        let reordered_names = ["F_p_RMS", "tool", "Vb", "F_c_RMS", "AE_RMS"];
        let indices: Vec<usize> = reordered_names
            .iter()
            .map(|name| column_index(&original.headers, name).unwrap())
            .collect();
        let reordered = Dataset {
            headers: reordered_names
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
            rows: original
                .rows
                .iter()
                .map(|row| Row {
                    tool: row.tool,
                    values: indices.iter().map(|index| row.values[*index]).collect(),
                })
                .collect(),
            missing_cells: original.missing_cells,
        };
        assert_eq!(
            tool_transfer_diagnostics(&original, "same-source-identity").unwrap(),
            tool_transfer_diagnostics(&reordered, "same-source-identity").unwrap()
        );
    }

    #[test]
    fn loto_report_is_byte_deterministic() {
        let dataset = transfer_fixture();
        let first = tool_transfer_diagnostics(&dataset, "test-source-sha").unwrap();
        let second = tool_transfer_diagnostics(&dataset, "test-source-sha").unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn loto_report_exposes_worst_tool_and_ranking_stability() {
        let dataset = transfer_fixture();
        let json = tool_transfer_diagnostics(&dataset, "test-source-sha").unwrap();
        assert!(json.contains("\"model_id\":\"MFG-DATA-002\""));
        assert!(json.contains("\"method\":\"leave-one-tool-out\""));
        assert!(json.contains("\"worst_tool\""));
        assert!(json.contains("\"ranking_signature_count\""));
        assert!(json.contains("\"model_ranking_by_mae\""));
        assert!(json.contains("\"fold_count\":4"));
    }

    #[test]
    fn leave_one_tool_out_rejects_single_tool_dataset() {
        let text = "tool,Vb,AE_RMS,F_c_RMS,F_p_RMS\n3,1,0.5,2,4\n3,2,0.6,3,5\n";
        let dataset = parse_csv(text).unwrap();
        assert!(fit_leave_one_tool_out(&dataset).is_err());
    }

    #[test]
    fn same_length_modified_source_fails_the_source_lock() {
        let dataset = transfer_fixture();
        let altered = vec![b'x'; EXPECTED_BYTES];
        let error = validate_locked_source(&dataset, &altered).unwrap_err();
        assert!(error.contains("source SHA-256 mismatch"));
    }

    #[test]
    fn constant_feature_fails_closed() {
        let text =
            "tool,Vb,AE_RMS,F_c_RMS,F_p_RMS\n3,3,1,1,10\n3,5,1,2,10\n4,7,1,3,10\n4,9,1,4,10\n";
        let dataset = parse_csv(text).expect("valid fixture should parse");
        let rows: Vec<&Row> = dataset.rows.iter().collect();
        let counts = tool_counts(&rows);
        let target = column_index(&dataset.headers, "Vb").unwrap();
        let feature = column_index(&dataset.headers, "AE_RMS").unwrap();
        assert!(fit_tool_balanced_linear(&rows, &counts, feature, target).is_err());
    }
}
