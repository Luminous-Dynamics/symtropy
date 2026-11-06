use serde_json::Value;
use std::collections::BTreeSet;

const DIMENSIONS: &[&str] = &[
    "parent_block",
    "specimen",
    "rock_cohort",
    "rock_family",
    "source_study",
    "campaign",
    "site",
    "tunnel",
    "mine",
    "rig",
    "machine",
    "cutter_geometry",
    "tool_serial",
    "radial_position",
    "sensor_configuration",
    "time_window",
];

const TASKS: &[&str] = &[
    "LOAD_RECONSTRUCTION",
    "WEAR_STATE_CLASSIFICATION",
    "WEAR_SEVERITY_REGRESSION",
    "ANOMALY_EVENT_DETECTION",
    "RUL_DISTRIBUTION",
    "SENSOR_DRIFT_DETECTION",
    "OOD_DETECTION",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationReport {
    pub evaluation_id: String,
    pub disposition: String,
    pub model_run_count: usize,
    pub metric_count: usize,
    pub warnings: Vec<String>,
}

fn violation(
    errors: &mut Vec<Violation>,
    code: &str,
    path: &str,
    detail: impl Into<String>,
) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn non_empty<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn count(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn check_digest(
    value: &Value,
    key: &str,
    path: &str,
    errors: &mut Vec<Violation>,
) {
    match non_empty(value, key) {
        Some(digest) if is_digest(digest) => {}
        _ => violation(
            errors,
            "INVALID_SHA256",
            &format!("{path}.{key}"),
            "expected exactly 64 hexadecimal characters",
        ),
    }
}

fn string_set(
    value: &Value,
    key: &str,
    path: &str,
    errors: &mut Vec<Violation>,
) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    match value.get(key).and_then(Value::as_array) {
        Some(items) => {
            for (index, item) in items.iter().enumerate() {
                match item.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    Some(item) => {
                        if !result.insert(item.to_owned()) {
                            violation(
                                errors,
                                "DUPLICATE_ARRAY_VALUE",
                                &format!("{path}.{key}[{index}]"),
                                format!("duplicate value {item}"),
                            );
                        }
                    }
                    None => violation(
                        errors,
                        "INVALID_ARRAY_VALUE",
                        &format!("{path}.{key}[{index}]"),
                        "expected a non-empty string",
                    ),
                }
            }
        }
        None => violation(
            errors,
            "MISSING_STRING_ARRAY",
            &format!("{path}.{key}"),
            "expected an array of strings",
        ),
    }
    result
}

fn groups(
    split: &Value,
    key: &str,
    declared_dimensions: &BTreeSet<String>,
    errors: &mut Vec<Violation>,
) -> BTreeSet<(String, String)> {
    let mut output = BTreeSet::new();
    let path = format!("split.{key}");
    let Some(items) = split.get(key).and_then(Value::as_array) else {
        violation(
            errors,
            "MISSING_GROUP_ARRAY",
            &path,
            "expected an array of group records",
        );
        return output;
    };

    for (index, item) in items.iter().enumerate() {
        let dimension = non_empty(item, "dimension");
        let group_id = non_empty(item, "group_id");
        match (dimension, group_id) {
            (Some(dimension), Some(group_id)) => {
                if !DIMENSIONS.contains(&dimension) {
                    violation(
                        errors,
                        "UNKNOWN_GROUP_DIMENSION",
                        &format!("{path}[{index}].dimension"),
                        format!("unknown dimension {dimension}"),
                    );
                }
                if !declared_dimensions.contains(dimension) {
                    violation(
                        errors,
                        "UNDECLARED_GROUP_DIMENSION",
                        &format!("{path}[{index}].dimension"),
                        format!("dimension {dimension} is not declared by the dataset"),
                    );
                }
                let item = (dimension.to_owned(), group_id.to_owned());
                if !output.insert(item.clone()) {
                    violation(
                        errors,
                        "DUPLICATE_GROUP",
                        &format!("{path}[{index}]"),
                        format!("duplicate group {}={}", item.0, item.1),
                    );
                }
            }
            _ => violation(
                errors,
                "INVALID_GROUP",
                &format!("{path}[{index}]"),
                "each group requires non-empty dimension and group_id",
            ),
        }
    }
    output
}

fn require_number(
    metric: &Value,
    key: &str,
    path: &str,
    minimum: Option<f64>,
    maximum: Option<f64>,
    errors: &mut Vec<Violation>,
) -> Option<f64> {
    match number(metric, key) {
        Some(value)
            if minimum.is_none_or(|min| value >= min)
                && maximum.is_none_or(|max| value <= max) =>
        {
            Some(value)
        }
        _ => {
            violation(
                errors,
                "INVALID_METRIC_VALUE",
                &format!("{path}.{key}"),
                "required metric is absent, non-finite or outside its permitted range",
            );
            None
        }
    }
}

fn validate_metric(
    metric: &Value,
    path: &str,
    test_count: u64,
    calibration_count: u64,
    split_id: &str,
    errors: &mut Vec<Violation>,
    warnings: &mut Vec<String>,
) {
    let task = non_empty(metric, "task");
    let Some(task) = task else {
        violation(errors, "MISSING_TASK", &format!("{path}.task"), "task is required");
        return;
    };
    if !TASKS.contains(&task) {
        violation(
            errors,
            "UNKNOWN_TASK",
            &format!("{path}.task"),
            format!("unsupported task {task}"),
        );
        return;
    }

    if non_empty(metric, "evaluation_partition") != Some("TEST") {
        violation(
            errors,
            "NON_TEST_METRIC",
            &format!("{path}.evaluation_partition"),
            "benchmark outcome metrics must be evaluated on TEST only",
        );
    }

    let sample_count = count(metric, "sample_count");
    match sample_count {
        Some(value) if value > 0 && value <= test_count => {}
        _ => violation(
            errors,
            "INVALID_METRIC_SAMPLE_COUNT",
            &format!("{path}.sample_count"),
            "metric sample_count must be positive and no greater than the test partition",
        ),
    }

    let expected_truth = if task == "LOAD_RECONSTRUCTION" {
        "CALIBRATED_REFERENCE_SENSOR"
    } else {
        "INDEPENDENT_INSPECTION"
    };
    if non_empty(metric, "truth_evidence_role") != Some(expected_truth) {
        violation(
            errors,
            "NON_INDEPENDENT_TRUTH",
            &format!("{path}.truth_evidence_role"),
            format!("task {task} requires {expected_truth}; model-derived and process-sensor labels are not independent truth"),
        );
    }

    match task {
        "LOAD_RECONSTRUCTION" | "WEAR_SEVERITY_REGRESSION" => {
            require_number(metric, "mae", path, Some(0.0), None, errors);
            require_number(metric, "rmse", path, Some(0.0), None, errors);
            require_number(metric, "bias", path, None, None, errors);
            if task == "LOAD_RECONSTRUCTION" {
                // Negative R² is valid and must not be schema-rejected.
                require_number(metric, "r2", path, None, None, errors);
            }
        }
        "WEAR_STATE_CLASSIFICATION" => {
            require_number(metric, "macro_f1", path, Some(0.0), Some(1.0), errors);
            match metric.get("per_class_recall").and_then(Value::as_object) {
                Some(classes) if !classes.is_empty() => {
                    for (class, value) in classes {
                        match value.as_f64() {
                            Some(recall) if recall.is_finite() && (0.0..=1.0).contains(&recall) => {}
                            _ => violation(
                                errors,
                                "INVALID_CLASS_RECALL",
                                &format!("{path}.per_class_recall.{class}"),
                                "per-class recall must be finite and within [0, 1]",
                            ),
                        }
                    }
                }
                _ => violation(
                    errors,
                    "MISSING_CLASS_RECALL",
                    &format!("{path}.per_class_recall"),
                    "a non-empty per-class recall map is required",
                ),
            }
        }
        "ANOMALY_EVENT_DETECTION" => {
            require_number(
                metric,
                "false_alarms_per_100_operating_hours",
                path,
                Some(0.0),
                None,
                errors,
            );
            require_number(
                metric,
                "median_detection_delay_hours",
                path,
                Some(0.0),
                None,
                errors,
            );
            match (
                count(metric, "missed_critical_events"),
                count(metric, "total_critical_events"),
            ) {
                (Some(missed), Some(total)) if missed <= total => {
                    if total == 0 {
                        warnings.push(format!(
                            "{path}: no critical events in test set; severe-event recall is not estimable"
                        ));
                    }
                }
                _ => violation(
                    errors,
                    "INVALID_EVENT_COUNTS",
                    &format!("{path}.missed_critical_events"),
                    "missed_critical_events and total_critical_events are required and missed cannot exceed total",
                ),
            }
        }
        "RUL_DISTRIBUTION" => {
            require_number(metric, "mae", path, Some(0.0), None, errors);
            require_number(metric, "pinball_loss", path, Some(0.0), None, errors);
            match non_empty(metric, "censoring_handling") {
                Some("NONE" | "RIGHT_CENSORED" | "INTERVAL_CENSORED" | "COMPETING_RISKS") => {}
                _ => violation(
                    errors,
                    "MISSING_CENSORING_POLICY",
                    &format!("{path}.censoring_handling"),
                    "RUL metrics require an explicit censoring policy",
                ),
            }
            let interval = metric.get("prediction_interval");
            if let Some(interval) = interval {
                let nominal = number(interval, "nominal_coverage");
                let observed = number(interval, "observed_coverage");
                let width = number(interval, "mean_width");
                if !nominal.is_some_and(|v| v > 0.0 && v <= 1.0) {
                    violation(
                        errors,
                        "INVALID_NOMINAL_COVERAGE",
                        &format!("{path}.prediction_interval.nominal_coverage"),
                        "nominal coverage must be in (0, 1]",
                    );
                }
                if !observed.is_some_and(|v| (0.0..=1.0).contains(&v)) {
                    violation(
                        errors,
                        "INVALID_OBSERVED_COVERAGE",
                        &format!("{path}.prediction_interval.observed_coverage"),
                        "observed coverage must be in [0, 1]",
                    );
                }
                if !width.is_some_and(|v| v >= 0.0) {
                    violation(
                        errors,
                        "INVALID_INTERVAL_WIDTH",
                        &format!("{path}.prediction_interval.mean_width"),
                        "mean interval width must be finite and non-negative",
                    );
                }
                if non_empty(interval, "calibration_partition") != Some("HELD_OUT_CALIBRATION") {
                    violation(
                        errors,
                        "CALIBRATION_LEAKAGE",
                        &format!("{path}.prediction_interval.calibration_partition"),
                        "prediction intervals must be calibrated on a held-out calibration partition, never TEST",
                    );
                }
                if non_empty(interval, "calibration_split_ref") != Some(split_id) {
                    violation(
                        errors,
                        "CALIBRATION_SPLIT_MISMATCH",
                        &format!("{path}.prediction_interval.calibration_split_ref"),
                        "calibration reference must bind to this evaluation split",
                    );
                }
                match count(interval, "calibration_sample_count") {
                    Some(value) if value > 0 && value <= calibration_count => {}
                    _ => violation(
                        errors,
                        "INVALID_CALIBRATION_COUNT",
                        &format!("{path}.prediction_interval.calibration_sample_count"),
                        "calibration sample count must be positive and no greater than the declared calibration partition",
                    ),
                }
            } else {
                violation(
                    errors,
                    "MISSING_PREDICTION_INTERVAL",
                    &format!("{path}.prediction_interval"),
                    "RUL distribution tasks require a prediction interval",
                );
            }
        }
        "SENSOR_DRIFT_DETECTION" | "OOD_DETECTION" => {
            let cases = count(metric, "ood_case_count");
            let detected = count(metric, "ood_detected_count");
            let false_accepts = count(metric, "ood_false_accept_count");
            let abstentions = count(metric, "abstention_count");
            match (cases, detected, false_accepts, abstentions) {
                (Some(cases), Some(detected), Some(false_accepts), Some(abstentions))
                    if cases > 0
                        && detected <= cases
                        && false_accepts <= cases
                        && abstentions <= cases => {}
                _ => violation(
                    errors,
                    "INVALID_OOD_COUNTS",
                    &format!("{path}.ood_case_count"),
                    "OOD/drift counts must be present, cases positive, and all outcomes no greater than cases",
                ),
            }
        }
        _ => {}
    }
}

pub fn evaluate_json(source: &str) -> Result<EvaluationReport, Vec<Violation>> {
    let value: Value = serde_json::from_str(source).map_err(|error| {
        vec![Violation {
            code: "INVALID_JSON".to_owned(),
            path: "$".to_owned(),
            detail: error.to_string(),
        }]
    })?;
    evaluate(&value)
}

pub fn evaluate(value: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if !value.is_object() {
        return Err(vec![Violation {
            code: "ROOT_NOT_OBJECT".to_owned(),
            path: "$".to_owned(),
            detail: "evaluation root must be a JSON object".to_owned(),
        }]);
    }

    if value.get("schema_version").and_then(Value::as_u64) != Some(1) {
        violation(&mut errors, "UNSUPPORTED_SCHEMA", "schema_version", "schema_version must equal 1");
    }
    let evaluation_id = non_empty(value, "evaluation_id").map(str::to_owned);
    if evaluation_id.is_none() {
        violation(&mut errors, "MISSING_EVALUATION_ID", "evaluation_id", "evaluation_id must be non-empty");
    }
    match non_empty(value, "evidence_class") {
        Some("SYNTHETIC_CONFORMANCE_FIXTURE") => {
            warnings.push("synthetic fixture: no physical performance or transfer claim is supported".to_owned());
        }
        Some("MODEL_EVALUATION") => {}
        _ => violation(
            &mut errors,
            "INVALID_EVIDENCE_CLASS",
            "evidence_class",
            "evidence_class must be SYNTHETIC_CONFORMANCE_FIXTURE or MODEL_EVALUATION",
        ),
    }

    let dataset = value.get("dataset");
    if dataset.is_none() {
        violation(&mut errors, "MISSING_DATASET", "dataset", "dataset object is required");
    }
    let Some(dataset) = dataset else {
        return Err(errors);
    };
    let dataset_id = non_empty(dataset, "dataset_id").map(str::to_owned);
    if dataset_id.is_none() {
        violation(&mut errors, "MISSING_DATASET_ID", "dataset.dataset_id", "dataset_id must be non-empty");
    }
    let dataset_count = count(dataset, "sample_count");
    if !dataset_count.is_some_and(|n| n > 0) {
        violation(&mut errors, "INVALID_DATASET_COUNT", "dataset.sample_count", "sample_count must be positive");
    }
    check_digest(dataset, "manifest_sha256", "dataset", &mut errors);
    check_digest(dataset, "source_records_sha256", "dataset", &mut errors);
    let dataset_dimensions = string_set(dataset, "group_dimensions", "dataset", &mut errors);
    for dimension in &dataset_dimensions {
        if !DIMENSIONS.contains(&dimension.as_str()) {
            violation(
                &mut errors,
                "UNKNOWN_DATASET_DIMENSION",
                "dataset.group_dimensions",
                format!("unknown dimension {dimension}"),
            );
        }
    }
    let protocol_refs = string_set(dataset, "measurement_protocol_refs", "dataset", &mut errors);
    if protocol_refs.is_empty() {
        violation(
            &mut errors,
            "MISSING_MEASUREMENT_PROTOCOL",
            "dataset.measurement_protocol_refs",
            "at least one measurement protocol reference is required",
        );
    }

    let ground_truth = value.get("ground_truth");
    if ground_truth.is_none() {
        violation(&mut errors, "MISSING_GROUND_TRUTH", "ground_truth", "ground_truth object is required");
    }
    if let Some(gt) = ground_truth {
        for key in ["authority_ref", "measurement_method", "calibration_ref", "uncertainty_budget_ref"] {
            if non_empty(gt, key).is_none() {
                violation(
                    &mut errors,
                    "MISSING_GROUND_TRUTH_REFERENCE",
                    &format!("ground_truth.{key}"),
                    "non-empty provenance reference is required",
                );
            }
        }
        if gt.get("independent").and_then(Value::as_bool) != Some(true) {
            violation(
                &mut errors,
                "GROUND_TRUTH_NOT_INDEPENDENT",
                "ground_truth.independent",
                "physical wear/failure truth must be independent of evaluated model inputs and outputs",
            );
        }
        match non_empty(gt, "assessor_blinding") {
            Some("BLINDED") => {}
            Some("UNBLINDED" | "NOT_POSSIBLE") => {
                warnings.push("ground-truth assessor was not blinded; retain this limitation in interpretation".to_owned());
            }
            _ => violation(
                &mut errors,
                "INVALID_BLINDING_STATUS",
                "ground_truth.assessor_blinding",
                "assessor_blinding must be BLINDED, UNBLINDED or NOT_POSSIBLE",
            ),
        }
    }

    let split = value.get("split");
    if split.is_none() {
        violation(&mut errors, "MISSING_SPLIT", "split", "split object is required");
    }
    let Some(split) = split else {
        return Err(errors);
    };
    let split_id = non_empty(split, "split_id").map(str::to_owned);
    if split_id.is_none() {
        violation(&mut errors, "MISSING_SPLIT_ID", "split.split_id", "split_id must be non-empty");
    }
    let strategy = non_empty(split, "strategy").unwrap_or("");
    let valid_strategies = [
        "RandomRowDiagnostic",
        "GroupHoldout",
        "TemporalHoldout",
        "ExternalSite",
        "ProspectiveField",
    ];
    if !valid_strategies.contains(&strategy) {
        violation(&mut errors, "INVALID_SPLIT_STRATEGY", "split.strategy", "unsupported split strategy");
    }
    let holdout_dimensions = string_set(split, "holdout_dimensions", "split", &mut errors);
    for dimension in &holdout_dimensions {
        if !DIMENSIONS.contains(&dimension.as_str()) {
            violation(
                &mut errors,
                "UNKNOWN_HOLDOUT_DIMENSION",
                "split.holdout_dimensions",
                format!("unknown dimension {dimension}"),
            );
        }
        if !dataset_dimensions.contains(dimension) {
            violation(
                &mut errors,
                "HOLDOUT_DIMENSION_NOT_IN_DATASET",
                "split.holdout_dimensions",
                format!("holdout dimension {dimension} is not declared by dataset"),
            );
        }
    }

    let train_count = count(split, "train_sample_count");
    let calibration_count = count(split, "calibration_sample_count");
    let test_count = count(split, "test_sample_count");
    match (train_count, calibration_count, test_count, dataset_count) {
        (Some(train), Some(calibration), Some(test), Some(total))
            if train > 0 && test > 0 && train.checked_add(calibration).and_then(|n| n.checked_add(test)) == Some(total) => {}
        _ => violation(
            &mut errors,
            "SPLIT_COUNT_MISMATCH",
            "split",
            "train + calibration + test counts must equal dataset.sample_count; train and test must be positive",
        ),
    }
    for key in ["train_sample_ids_sha256", "calibration_sample_ids_sha256", "test_sample_ids_sha256"] {
        check_digest(split, key, "split", &mut errors);
    }

    let train_groups = groups(split, "train_groups", &dataset_dimensions, &mut errors);
    let calibration_groups = groups(split, "calibration_groups", &dataset_dimensions, &mut errors);
    let test_groups = groups(split, "test_groups", &dataset_dimensions, &mut errors);

    if strategy == "RandomRowDiagnostic" {
        warnings.push("random-row split is diagnostic only and cannot support any transfer claim".to_owned());
    } else {
        if holdout_dimensions.is_empty() {
            violation(
                &mut errors,
                "EMPTY_HOLDOUT_DIMENSIONS",
                "split.holdout_dimensions",
                "non-random splits require at least one holdout dimension",
            );
        }
        if train_groups.is_empty() || test_groups.is_empty() {
            violation(
                &mut errors,
                "EMPTY_GROUP_MANIFEST",
                "split",
                "non-random splits require non-empty train and test group manifests",
            );
        }
        for dimension in &holdout_dimensions {
            let train_dimension = train_groups.iter().any(|(d, _)| d == dimension);
            let test_dimension = test_groups.iter().any(|(d, _)| d == dimension);
            if !train_dimension || !test_dimension {
                violation(
                    &mut errors,
                    "HOLDOUT_GROUPS_MISSING",
                    "split",
                    format!("both train and test must declare groups for holdout dimension {dimension}"),
                );
            }
            for (test_d, test_id) in &test_groups {
                if test_d == dimension
                    && (train_groups.contains(&(test_d.clone(), test_id.clone()))
                        || calibration_groups.contains(&(test_d.clone(), test_id.clone())))
                {
                    violation(
                        &mut errors,
                        "TEST_GROUP_OVERLAP",
                        "split.test_groups",
                        format!("test group {dimension}={test_id} appears in train or calibration"),
                    );
                }
            }
        }
    }

    if strategy == "TemporalHoldout" && !holdout_dimensions.contains("time_window") {
        violation(
            &mut errors,
            "TEMPORAL_DIMENSION_REQUIRED",
            "split.holdout_dimensions",
            "TemporalHoldout requires time_window",
        );
    }
    if strategy == "ExternalSite" && !holdout_dimensions.contains("site") {
        violation(
            &mut errors,
            "EXTERNAL_SITE_DIMENSION_REQUIRED",
            "split.holdout_dimensions",
            "ExternalSite requires site as a held-out dimension",
        );
    }
    if strategy == "ProspectiveField"
        && (!holdout_dimensions.contains("site") || !holdout_dimensions.contains("time_window"))
    {
        violation(
            &mut errors,
            "PROSPECTIVE_DIMENSIONS_REQUIRED",
            "split.holdout_dimensions",
            "ProspectiveField requires both site and time_window holdouts",
        );
    }

    let model_runs = value.get("model_runs").and_then(Value::as_array);
    if model_runs.is_none_or(Vec::is_empty) {
        violation(
            &mut errors,
            "MISSING_MODEL_RUNS",
            "model_runs",
            "at least one model run is required",
        );
    }
    let Some(model_runs) = model_runs else {
        return Err(errors);
    };
    if model_runs.is_empty() {
        return Err(errors);
    }

    let mut model_ids = BTreeSet::new();
    let mut metric_count = 0usize;
    for (model_index, model) in model_runs.iter().enumerate() {
        let path = format!("model_runs[{model_index}]");
        let model_id = non_empty(model, "model_id");
        match model_id {
            Some(id) => {
                if !model_ids.insert(id.to_owned()) {
                    violation(&mut errors, "DUPLICATE_MODEL_ID", &format!("{path}.model_id"), format!("duplicate model_id {id}"));
                }
            }
            None => violation(&mut errors, "MISSING_MODEL_ID", &format!("{path}.model_id"), "model_id must be non-empty"),
        }
        if dataset_id.as_deref().is_some_and(|id| non_empty(model, "dataset_ref") != Some(id)) {
            violation(
                &mut errors,
                "MODEL_DATASET_MISMATCH",
                &format!("{path}.dataset_ref"),
                "model dataset_ref must match the evaluation dataset",
            );
        }
        if split_id.as_deref().is_some_and(|id| non_empty(model, "training_split_ref") != Some(id)) {
            violation(
                &mut errors,
                "MODEL_SPLIT_MISMATCH",
                &format!("{path}.training_split_ref"),
                "model training_split_ref must match the declared split",
            );
        }
        for key in ["model_revision"] {
            if non_empty(model, key).is_none() {
                violation(&mut errors, "MISSING_MODEL_REVISION", &format!("{path}.{key}"), "model revision is required");
            }
        }
        check_digest(model, "code_sha256", &path, &mut errors);
        check_digest(model, "configuration_sha256", &path, &mut errors);

        let metrics = model.get("metrics").and_then(Value::as_array);
        let Some(metrics) = metrics.filter(|items| !items.is_empty()) else {
            violation(
                &mut errors,
                "MISSING_MODEL_METRICS",
                &format!("{path}.metrics"),
                "every model run requires at least one test metric",
            );
            continue;
        };
        let mut tasks = BTreeSet::new();
        for (metric_index, metric) in metrics.iter().enumerate() {
            metric_count += 1;
            let metric_path = format!("{path}.metrics[{metric_index}]");
            if let Some(task) = non_empty(metric, "task") {
                if !tasks.insert(task.to_owned()) {
                    violation(
                        &mut errors,
                        "DUPLICATE_MODEL_TASK",
                        &format!("{metric_path}.task"),
                        format!("model reports task {task} more than once"),
                    );
                }
            }
            validate_metric(
                metric,
                &metric_path,
                test_count.unwrap_or(0),
                calibration_count.unwrap_or(0),
                split_id.as_deref().unwrap_or(""),
                &mut errors,
                &mut warnings,
            );
        }
    }

    if !errors.is_empty() {
        errors.sort_by(|a, b| (&a.path, &a.code, &a.detail).cmp(&(&b.path, &b.code, &b.detail)));
        errors.dedup();
        return Err(errors);
    }

    let disposition = match strategy {
        "RandomRowDiagnostic" => "DiagnosticOnly",
        "ExternalSite" => "ExternalSiteManifest",
        "ProspectiveField" => "ProspectiveFieldManifest",
        _ => "GroupedHoldoutManifest",
    };

    Ok(EvaluationReport {
        evaluation_id: evaluation_id.unwrap_or_default(),
        disposition: disposition.to_owned(),
        model_run_count: model_runs.len(),
        metric_count,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-cutter-health-evaluation.v0_1.json"
        ))
        .expect("fixture JSON must parse")
    }

    fn codes(value: &Value) -> Vec<String> {
        evaluate(value)
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|error| error.code)
            .collect()
    }

    fn has_code(value: &Value, expected: &str) -> bool {
        codes(value).iter().any(|code| code == expected)
    }

    fn metric_mut<'a>(value: &'a mut Value, model_id: &str, task: &str) -> &'a mut Value {
        value["model_runs"]
            .as_array_mut()
            .expect("model_runs array")
            .iter_mut()
            .find(|model| model["model_id"] == model_id)
            .expect("model")
            ["metrics"]
            .as_array_mut()
            .expect("metrics array")
            .iter_mut()
            .find(|metric| metric["task"] == task)
            .expect("metric")
    }

    #[test]
    fn accepts_synthetic_external_site_manifest() {
        let report = evaluate(&fixture()).expect("valid fixture");
        assert_eq!(report.disposition, "ExternalSiteManifest");
        assert_eq!(report.model_run_count, 2);
        assert_eq!(report.metric_count, 6);
        assert!(report.warnings.iter().any(|w| w.contains("synthetic fixture")));
    }

    #[test]
    fn rejects_non_object_root() {
        let errors = evaluate(&json!([])).expect_err("root must be object");
        assert_eq!(errors[0].code, "ROOT_NOT_OBJECT");
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(evaluate_json("{").is_err());
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let mut value = fixture();
        value["schema_version"] = json!(2);
        assert!(has_code(&value, "UNSUPPORTED_SCHEMA"));
    }

    #[test]
    fn rejects_invalid_dataset_digest() {
        let mut value = fixture();
        value["dataset"]["manifest_sha256"] = json!("bad");
        assert!(has_code(&value, "INVALID_SHA256"));
    }

    #[test]
    fn rejects_dataset_count_mismatch() {
        let mut value = fixture();
        value["dataset"]["sample_count"] = json!(21);
        assert!(has_code(&value, "SPLIT_COUNT_MISMATCH"));
    }

    #[test]
    fn rejects_duplicate_group_dimension() {
        let mut value = fixture();
        value["dataset"]["group_dimensions"] = json!(["site", "site"]);
        assert!(has_code(&value, "DUPLICATE_ARRAY_VALUE"));
    }

    #[test]
    fn rejects_group_dimension_not_in_dataset() {
        let mut value = fixture();
        value["split"]["test_groups"][0]["dimension"] = json!("machine");
        assert!(has_code(&value, "UNDECLARED_GROUP_DIMENSION"));
    }

    #[test]
    fn rejects_test_site_overlap_with_training() {
        let mut value = fixture();
        value["split"]["test_groups"][2]["group_id"] = json!("SITE-TRAIN");
        assert!(has_code(&value, "TEST_GROUP_OVERLAP"));
    }

    #[test]
    fn rejects_test_rig_overlap_with_calibration() {
        let mut value = fixture();
        value["split"]["test_groups"][3]["group_id"] = json!("RIG-TRAIN");
        assert!(has_code(&value, "TEST_GROUP_OVERLAP"));
    }

    #[test]
    fn external_site_requires_site_holdout() {
        let mut value = fixture();
        value["split"]["holdout_dimensions"] = json!(["parent_block", "rig"]);
        assert!(has_code(&value, "EXTERNAL_SITE_DIMENSION_REQUIRED"));
    }

    #[test]
    fn temporal_holdout_requires_time_window() {
        let mut value = fixture();
        value["split"]["strategy"] = json!("TemporalHoldout");
        value["split"]["holdout_dimensions"] = json!(["site"]);
        assert!(has_code(&value, "TEMPORAL_DIMENSION_REQUIRED"));
    }

    #[test]
    fn prospective_field_requires_site_and_time() {
        let mut value = fixture();
        value["split"]["strategy"] = json!("ProspectiveField");
        value["split"]["holdout_dimensions"] = json!(["site"]);
        assert!(has_code(&value, "PROSPECTIVE_DIMENSIONS_REQUIRED"));
    }

    #[test]
    fn random_row_split_is_diagnostic_only() {
        let mut value = fixture();
        value["split"]["strategy"] = json!("RandomRowDiagnostic");
        value["split"]["holdout_dimensions"] = json!([]);
        value["split"]["train_groups"] = json!([]);
        value["split"]["calibration_groups"] = json!([]);
        value["split"]["test_groups"] = json!([]);
        let report = evaluate(&value).expect("diagnostic split is valid");
        assert_eq!(report.disposition, "DiagnosticOnly");
        assert!(report.warnings.iter().any(|w| w.contains("diagnostic only")));
    }

    #[test]
    fn rejects_non_independent_ground_truth() {
        let mut value = fixture();
        value["ground_truth"]["independent"] = json!(false);
        assert!(has_code(&value, "GROUND_TRUTH_NOT_INDEPENDENT"));
    }

    #[test]
    fn warns_when_assessor_was_not_blinded() {
        let mut value = fixture();
        value["ground_truth"]["assessor_blinding"] = json!("UNBLINDED");
        let report = evaluate(&value).expect("unblinded is disclosed");
        assert!(report.warnings.iter().any(|w| w.contains("not blinded")));
    }

    #[test]
    fn rejects_non_test_prediction_metric() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "WEAR_STATE_CLASSIFICATION")
            ["evaluation_partition"] = json!("TRAIN");
        assert!(has_code(&value, "NON_TEST_METRIC"));
    }

    #[test]
    fn rejects_non_independent_wear_labels() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "WEAR_STATE_CLASSIFICATION")
            ["truth_evidence_role"] = json!("MODEL_DERIVED");
        assert!(has_code(&value, "NON_INDEPENDENT_TRUTH"));
    }

    #[test]
    fn rejects_test_metric_count_larger_than_test_partition() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "WEAR_STATE_CLASSIFICATION")
            ["sample_count"] = json!(5);
        assert!(has_code(&value, "INVALID_METRIC_SAMPLE_COUNT"));
    }

    #[test]
    fn rejects_out_of_range_macro_f1() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "WEAR_STATE_CLASSIFICATION")
            ["macro_f1"] = json!(1.1);
        assert!(has_code(&value, "INVALID_METRIC_VALUE"));
    }

    #[test]
    fn rejects_out_of_range_per_class_recall() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "WEAR_STATE_CLASSIFICATION")
            ["per_class_recall"]["FRACTURE"] = json!(-0.1);
        assert!(has_code(&value, "INVALID_CLASS_RECALL"));
    }

    #[test]
    fn rejects_duplicate_model_ids() {
        let mut value = fixture();
        value["model_runs"][1]["model_id"] = json!("PHYSICS-WEAR-BASELINE");
        assert!(has_code(&value, "DUPLICATE_MODEL_ID"));
    }

    #[test]
    fn rejects_model_dataset_mismatch() {
        let mut value = fixture();
        value["model_runs"][0]["dataset_ref"] = json!("OTHER-DATASET");
        assert!(has_code(&value, "MODEL_DATASET_MISMATCH"));
    }

    #[test]
    fn rejects_invalid_event_counts() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "ANOMALY_EVENT_DETECTION")
            ["missed_critical_events"] = json!(3);
        assert!(has_code(&value, "INVALID_EVENT_COUNTS"));
    }

    #[test]
    fn rejects_negative_false_alarm_rate() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "ANOMALY_EVENT_DETECTION")
            ["false_alarms_per_100_operating_hours"] = json!(-1.0);
        assert!(has_code(&value, "INVALID_METRIC_VALUE"));
    }

    #[test]
    fn rejects_rul_without_censoring_policy() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "RUL_DISTRIBUTION")
            .as_object_mut()
            .expect("metric object")
            .remove("censoring_handling");
        assert!(has_code(&value, "MISSING_CENSORING_POLICY"));
    }

    #[test]
    fn rejects_test_set_used_for_interval_calibration() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "RUL_DISTRIBUTION")
            ["prediction_interval"]["calibration_partition"] = json!("TEST");
        assert!(has_code(&value, "CALIBRATION_LEAKAGE"));
    }

    #[test]
    fn rejects_calibration_count_larger_than_calibration_partition() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "RUL_DISTRIBUTION")
            ["prediction_interval"]["calibration_sample_count"] = json!(5);
        assert!(has_code(&value, "INVALID_CALIBRATION_COUNT"));
    }

    #[test]
    fn rejects_interval_with_zero_nominal_coverage() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "RUL_DISTRIBUTION")
            ["prediction_interval"]["nominal_coverage"] = json!(0.0);
        assert!(has_code(&value, "INVALID_NOMINAL_COVERAGE"));
    }

    #[test]
    fn rejects_negative_interval_width() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "RUL_DISTRIBUTION")
            ["prediction_interval"]["mean_width"] = json!(-3.0);
        assert!(has_code(&value, "INVALID_INTERVAL_WIDTH"));
    }

    #[test]
    fn rejects_unknown_task() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "WEAR_STATE_CLASSIFICATION")
            ["task"] = json!("MAGIC_ACCURACY");
        assert!(has_code(&value, "UNKNOWN_TASK"));
    }

    #[test]
    fn rejects_duplicate_task_within_model() {
        let mut value = fixture();
        let first = value["model_runs"][1]["metrics"][0].clone();
        value["model_runs"][1]["metrics"].as_array_mut().unwrap().push(first);
        assert!(has_code(&value, "DUPLICATE_MODEL_TASK"));
    }

    #[test]
    fn rejects_missing_rul_prediction_interval() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "RUL_DISTRIBUTION")
            .as_object_mut()
            .expect("metric object")
            .remove("prediction_interval");
        assert!(has_code(&value, "MISSING_PREDICTION_INTERVAL"));
    }

    #[test]
    fn warns_when_test_has_no_critical_events() {
        let mut value = fixture();
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "ANOMALY_EVENT_DETECTION")
            ["total_critical_events"] = json!(0);
        metric_mut(&mut value, "ATTENTION-WEAR-MODEL", "ANOMALY_EVENT_DETECTION")
            ["missed_critical_events"] = json!(0);
        let report = evaluate(&value).expect("zero-event cohort is permitted but limited");
        assert!(report.warnings.iter().any(|w| w.contains("not estimable")));
    }

    #[test]
    fn allows_negative_r2_for_load_reconstruction() {
        let mut value = fixture();
        let metric = json!({
            "task": "LOAD_RECONSTRUCTION",
            "evaluation_partition": "TEST",
            "sample_count": 4,
            "truth_evidence_role": "CALIBRATED_REFERENCE_SENSOR",
            "mae": 1.0,
            "rmse": 1.4,
            "bias": -0.2,
            "r2": -0.1
        });
        value["model_runs"][0]["metrics"].as_array_mut().unwrap().push(metric);
        let report = evaluate(&value).expect("negative R2 is mathematically valid");
        assert_eq!(report.metric_count, 7);
    }
}
