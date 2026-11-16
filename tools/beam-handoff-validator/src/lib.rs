use csv::{ReaderBuilder, StringRecord};
use jsonschema::Validator;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path};

const MOMENT_ABS_TOLERANCE: f64 = 1.0e-18;

#[derive(Debug, Clone)]
pub struct ValidationSummary {
    pub handoff_id: String,
    pub stage: String,
    pub schema_version: String,
    pub checks: Vec<String>,
    pub asset_sha256: Option<String>,
    pub macro_particle_count: Option<u64>,
}

pub fn validate_files(
    schema_path: &Path,
    envelope_path: &Path,
) -> Result<ValidationSummary, Vec<String>> {
    let schema_bytes = fs::read(schema_path)
        .map_err(|error| vec![format!("cannot read schema {}: {error}", schema_path.display())])?;
    let envelope_bytes = fs::read(envelope_path).map_err(|error| {
        vec![format!(
            "cannot read handoff {}: {error}",
            envelope_path.display()
        )]
    })?;

    let schema: Value = serde_json::from_slice(&schema_bytes)
        .map_err(|error| vec![format!("schema is not valid JSON: {error}")])?;
    let envelope: Value = serde_json::from_slice(&envelope_bytes)
        .map_err(|error| vec![format!("handoff is not valid JSON: {error}")])?;

    if !jsonschema::draft202012::meta::is_valid(&schema) {
        return Err(vec![
            "schema document does not conform to the JSON Schema Draft 2020-12 meta-schema".to_owned(),
        ]);
    }

    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| vec![format!("cannot compile JSON Schema: {error}")])?;
    let base_dir = envelope_path.parent().unwrap_or_else(|| Path::new("."));

    validate_document(&validator, &envelope, base_dir)
}

fn validate_document(
    validator: &Validator,
    envelope: &Value,
    base_dir: &Path,
) -> Result<ValidationSummary, Vec<String>> {
    let schema_errors: Vec<String> = validator
        .iter_errors(envelope)
        .map(|error| format!("schema{}: {error}", error.instance_path()))
        .collect();

    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut checks = vec![
        "JSON Schema Draft 2020-12 validation passed".to_owned(),
        "required fields and declared types passed".to_owned(),
    ];

    validate_qualification(envelope)?;
    checks.push("qualification and handoff status are consistent".to_owned());

    validate_population_ledger(envelope)?;
    checks.push("species-aware weighted population ledger passed".to_owned());

    let expected_population =
        number_at(envelope, &["population_ledger", "surviving_weighted_count"])?;
    let represented_population = number_at(envelope, &["distribution", "weighted_population"])?;
    let relative_tolerance =
        number_at(envelope, &["population_ledger", "balance_tolerance_relative"])?;

    if !close_relative(expected_population, represented_population, relative_tolerance) {
        return Err(vec![format!(
            "distribution weighted population {represented_population} does not match surviving ledger population {expected_population}"
        )]);
    }
    checks.push("distribution population matches the surviving ledger population".to_owned());

    let representation = string_at(envelope, &["distribution", "representation"])?;
    let (asset_sha256, macro_particle_count) = if representation == "summary_only" {
        (None, None)
    } else {
        validate_distribution_asset(envelope, base_dir, relative_tolerance)?
    };

    if representation == "summary_only" {
        checks.push("summary-only representation remains explicitly non-particle-resolved".to_owned());
    } else {
        let correlations_preserved = envelope
            .get("distribution")
            .and_then(|value| value.get("correlations_preserved"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !correlations_preserved {
            return Err(vec![
                "full/reduced distribution handoff must explicitly preserve joint phase-space correlations".to_owned(),
            ]);
        }
        checks.push("distribution asset path is confined to the envelope directory".to_owned());
        checks.push("asset byte length and SHA-256 digest match".to_owned());
        checks.push("CSV particle identities, weights and ancestry passed".to_owned());
        checks.push("particle count, weighted population and effective sample size passed".to_owned());
        checks.push("weighted six-dimensional mean and covariance passed".to_owned());
    }

    Ok(ValidationSummary {
        handoff_id: string_at(envelope, &["handoff_id"])?,
        stage: string_at(envelope, &["stage"])?,
        schema_version: string_at(envelope, &["schema_version"])?,
        checks,
        asset_sha256,
        macro_particle_count,
    })
}

fn validate_qualification(envelope: &Value) -> Result<(), Vec<String>> {
    let qualification_status = string_at(envelope, &["qualification", "status"])?;
    let tier = string_at(envelope, &["qualification", "tier"])?;
    let handoff_status = string_at(envelope, &["handoff", "acceptance_status"])?;

    if handoff_status == "qualified" && qualification_status != "qualified" {
        return Err(vec![
            "handoff cannot be qualified when stage qualification is not qualified".to_owned(),
        ]);
    }

    if handoff_status == "qualified" && tier == "schema_only" {
        return Err(vec![
            "schema-only validation cannot qualify a physical stage handoff".to_owned(),
        ]);
    }

    if qualification_status == "qualified" && tier == "schema_only" {
        return Err(vec![
            "schema-only evidence cannot assert qualified physical performance".to_owned(),
        ]);
    }

    Ok(())
}

fn validate_population_ledger(envelope: &Value) -> Result<(), Vec<String>> {
    let scope = string_at(envelope, &["population_ledger", "accounting_scope"])?;
    let input = number_at(envelope, &["population_ledger", "input_weighted_count"])?;
    let generated = number_at(envelope, &["population_ledger", "generated_weighted_count"])?;
    let surviving = number_at(envelope, &["population_ledger", "surviving_weighted_count"])?;
    let declared_residual =
        number_at(envelope, &["population_ledger", "balance_residual_weighted_count"])?;
    let tolerance =
        number_at(envelope, &["population_ledger", "balance_tolerance_relative"])?;

    if tolerance < 0.0 {
        return Err(vec!["population balance tolerance must be non-negative".to_owned()]);
    }

    if scope == "particle_production" {
        if input != 0.0 {
            return Err(vec![
                "particle_production requires input_weighted_count=0; source-species population belongs in source_accounting".to_owned(),
            ]);
        }

        let source_count = number_at(
            envelope,
            &["source_accounting", "source_population_weighted_count"],
        )?;
        if source_count <= 0.0 {
            return Err(vec![
                "particle production requires a positive source-species population".to_owned(),
            ]);
        }
    }

    let losses = envelope
        .get("population_ledger")
        .and_then(|value| value.get("losses"))
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["population_ledger.losses must be an array".to_owned()])?;

    let mut loss_sum = 0.0;
    let mut causes = HashSet::new();

    for loss in losses {
        let cause = loss
            .get("cause")
            .and_then(Value::as_str)
            .ok_or_else(|| vec!["each loss entry requires a cause".to_owned()])?;

        if !causes.insert(cause.to_owned()) {
            return Err(vec![format!(
                "duplicate loss cause {cause:?}; aggregate each cause into one ledger entry"
            )]);
        }

        let weighted_count = loss
            .get("weighted_count")
            .and_then(Value::as_f64)
            .ok_or_else(|| vec![format!("loss cause {cause:?} has no numeric weighted_count")])?;

        if weighted_count < 0.0 {
            return Err(vec![format!(
                "loss cause {cause:?} has a negative weighted_count"
            )]);
        }

        loss_sum += weighted_count;
    }

    let accounted_population = surviving + loss_sum;
    let source_population = if scope == "particle_production" {
        generated
    } else {
        input + generated
    };
    let computed_residual = source_population - accounted_population;
    let allowed_residual = tolerance * source_population.abs().max(1.0);

    if (computed_residual - declared_residual).abs() > allowed_residual {
        return Err(vec![format!(
            "declared balance residual {declared_residual} differs from computed residual {computed_residual}"
        )]);
    }

    if computed_residual.abs() > allowed_residual {
        return Err(vec![format!(
            "weighted population does not conserve within tolerance: residual {computed_residual}, allowed {allowed_residual}"
        )]);
    }

    Ok(())
}

fn validate_distribution_asset(
    envelope: &Value,
    base_dir: &Path,
    relative_tolerance: f64,
) -> Result<(Option<String>, Option<u64>), Vec<String>> {
    let distribution = envelope
        .get("distribution")
        .ok_or_else(|| vec!["missing distribution object".to_owned()])?;
    let asset = distribution
        .get("asset")
        .ok_or_else(|| vec!["full/reduced distribution requires an asset".to_owned()])?;

    let uri = asset
        .get("uri")
        .and_then(Value::as_str)
        .ok_or_else(|| vec!["distribution asset URI must be a string".to_owned()])?;

    let relative_path = Path::new(uri);
    if uri.trim().is_empty()
        || uri.contains("://")
        || uri.contains('\\')
        || uri.contains('?')
        || uri.contains('#')
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(vec![format!(
            "asset URI must be a confined relative path, got {uri:?}"
        )]);
    }

    let root = fs::canonicalize(base_dir).map_err(|error| {
        vec![format!(
            "cannot resolve envelope directory {}: {error}",
            base_dir.display()
        )]
    })?;
    let joined = base_dir.join(relative_path);
    let resolved = fs::canonicalize(&joined)
        .map_err(|error| vec![format!("cannot resolve asset {}: {error}", joined.display())])?;

    if !resolved.starts_with(&root) {
        return Err(vec![format!(
            "asset path escapes envelope directory: {}",
            resolved.display()
        )]);
    }

    let bytes = fs::read(&resolved)
        .map_err(|error| vec![format!("cannot read asset {}: {error}", resolved.display())])?;
    let byte_length = asset
        .get("byte_length")
        .and_then(Value::as_u64)
        .ok_or_else(|| vec!["asset.byte_length must be an unsigned integer".to_owned()])?;

    if bytes.len() as u64 != byte_length {
        return Err(vec![format!(
            "asset byte length mismatch: declared {byte_length}, found {}",
            bytes.len()
        )]);
    }

    let declared_digest = asset
        .get("sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| vec!["asset.sha256 must be a string".to_owned()])?;
    let actual_digest = sha256_hex(&bytes);

    if declared_digest != actual_digest {
        return Err(vec![format!(
            "asset SHA-256 mismatch: declared {declared_digest}, computed {actual_digest}"
        )]);
    }

    let format = asset
        .get("format")
        .and_then(Value::as_str)
        .ok_or_else(|| vec!["asset.format must be a string".to_owned()])?;
    let adapter_version = asset
        .get("adapter_version")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| vec!["asset.adapter_version must be a non-empty string".to_owned()])?;

    if format != "text/csv" {
        return Err(vec![format!(
            "no validator adapter is registered for asset format {format:?}; available adapter: text/csv"
        )]);
    }

    let particle_count = validate_csv_distribution(&resolved, envelope, relative_tolerance)
        .map_err(|error| vec![format!("CSV adapter {adapter_version}: {error}")])?;

    Ok((Some(actual_digest), Some(particle_count)))
}

fn validate_csv_distribution(
    path: &Path,
    envelope: &Value,
    relative_tolerance: f64,
) -> Result<u64, String> {
    let mut reader = ReaderBuilder::new()
        .comment(Some(b'#'))
        .from_path(path)
        .map_err(|error| format!("cannot open CSV: {error}"))?;
    let headers = reader
        .headers()
        .map_err(|error| format!("cannot read CSV headers: {error}"))?
        .clone();

    let required_columns = [
        "particle_id",
        "x_m",
        "px_GeV_c",
        "y_m",
        "py_GeV_c",
        "z_m",
        "pz_GeV_c",
        "weight",
    ];
    let mut indices = Vec::with_capacity(required_columns.len());

    for name in required_columns {
        let index = headers
            .iter()
            .position(|header| header == name)
            .ok_or_else(|| format!("CSV is missing required column {name:?}"))?;
        indices.push(index);
    }

    let ancestry_preserved = envelope
        .get("distribution")
        .and_then(|value| value.get("ancestry_preserved"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ancestry_index = headers.iter().position(|header| header == "ancestry_id");

    if ancestry_preserved && ancestry_index.is_none() {
        return Err("ancestry_preserved=true but CSV has no ancestry_id column".to_owned());
    }

    let mut ids = HashSet::new();
    let mut points: Vec<([f64; 6], f64)> = Vec::new();
    let mut weight_sum = 0.0;
    let mut weight_square_sum = 0.0;

    for (row_index, row) in reader.records().enumerate() {
        let record = row.map_err(|error| format!("CSV row {}: {error}", row_index + 2))?;
        let particle_id = field(&record, indices[0], "particle_id")?;

        if particle_id.trim().is_empty() || !ids.insert(particle_id.to_owned()) {
            return Err(format!(
                "CSV row {} has an empty or duplicate particle_id {particle_id:?}",
                row_index + 2
            ));
        }

        if let Some(index) = ancestry_index {
            let ancestry = field(&record, index, "ancestry_id")?;
            if ancestry_preserved && ancestry.trim().is_empty() {
                return Err(format!(
                    "CSV row {} has empty ancestry_id while ancestry_preserved=true",
                    row_index + 2
                ));
            }
        }

        let mut coordinates = [0.0_f64; 6];
        for (coordinate_index, coordinate) in coordinates.iter_mut().enumerate() {
            let name = required_columns[coordinate_index + 1];
            let raw = field(&record, indices[coordinate_index + 1], name)?;
            let parsed = raw
                .parse::<f64>()
                .map_err(|error| format!("CSV row {} column {name}: {error}", row_index + 2))?;

            if !parsed.is_finite() {
                return Err(format!(
                    "CSV row {} column {name} is not finite",
                    row_index + 2
                ));
            }
            *coordinate = parsed;
        }

        let weight_raw = field(&record, indices[7], "weight")?;
        let weight = weight_raw
            .parse::<f64>()
            .map_err(|error| format!("CSV row {} weight: {error}", row_index + 2))?;

        if !weight.is_finite() || weight < 0.0 {
            return Err(format!(
                "CSV row {} has a non-finite or negative particle weight",
                row_index + 2
            ));
        }

        weight_sum += weight;
        weight_square_sum += weight * weight;
        points.push((coordinates, weight));
    }

    let count = u64::try_from(points.len())
        .map_err(|error| format!("particle count cannot be represented as u64: {error}"))?;
    let declared_count = envelope
        .get("distribution")
        .and_then(|value| value.get("macro_particle_count"))
        .and_then(Value::as_u64)
        .ok_or_else(|| "distribution.macro_particle_count must be an unsigned integer".to_owned())?;

    if count != declared_count {
        return Err(format!(
            "CSV particle count mismatch: declared {declared_count}, found {count}"
        ));
    }

    if weight_sum <= 0.0 || weight_square_sum <= 0.0 {
        return Err("CSV must contain a positive total particle weight".to_owned());
    }

    let declared_weight = number_at(envelope, &["distribution", "weighted_population"])
        .map_err(|error| error.join("; "))?;
    if !close_relative(weight_sum, declared_weight, relative_tolerance) {
        return Err(format!(
            "CSV weighted population mismatch: declared {declared_weight}, computed {weight_sum}"
        ));
    }

    let effective_sample_size = weight_sum * weight_sum / weight_square_sum;
    let declared_ess = number_at(envelope, &["distribution", "effective_sample_size"])
        .map_err(|error| error.join("; "))?;
    if !close_relative(effective_sample_size, declared_ess, relative_tolerance) {
        return Err(format!(
            "effective sample size mismatch: declared {declared_ess}, computed {effective_sample_size}"
        ));
    }

    let mut mean = [0.0_f64; 6];
    for (coordinates, weight) in &points {
        for (index, value) in mean.iter_mut().enumerate() {
            *value += coordinates[index] * weight;
        }
    }
    for value in &mut mean {
        *value /= weight_sum;
    }

    let mut covariance = [[0.0_f64; 6]; 6];
    for (coordinates, weight) in &points {
        for (row_index, row) in covariance.iter_mut().enumerate() {
            for (column_index, value) in row.iter_mut().enumerate() {
                *value += weight
                    * (coordinates[row_index] - mean[row_index])
                    * (coordinates[column_index] - mean[column_index]);
            }
        }
    }
    for row in &mut covariance {
        for value in row {
            *value /= weight_sum;
        }
    }

    let declared_mean =
        number_array_at::<6>(envelope, &["distribution", "summary", "mean_6d"], 6)
            .map_err(|error| error.join("; "))?;
    for (index, (computed, declared)) in mean.iter().zip(declared_mean.iter()).enumerate() {
        if !close_moment(*computed, *declared) {
            return Err(format!(
                "weighted mean mismatch at coordinate {index}: declared {declared}, computed {computed}"
            ));
        }
    }

    let declared_covariance =
        matrix_array_at::<6>(envelope, &["distribution", "summary", "covariance_6d"], 6)
            .map_err(|error| error.join("; "))?;

    for (row_index, (computed_row, declared_row)) in
        covariance.iter().zip(declared_covariance.iter()).enumerate()
    {
        for (column_index, (computed, declared)) in
            computed_row.iter().zip(declared_row.iter()).enumerate()
        {
            if !close_moment(*computed, *declared) {
                return Err(format!(
                    "weighted covariance mismatch at [{row_index}][{column_index}]: declared {declared}, computed {computed}"
                ));
            }

            if !close_moment(
                *declared,
                declared_covariance[column_index][row_index],
            ) {
                return Err(format!(
                    "declared covariance is not symmetric at [{row_index}][{column_index}]"
                ));
            }
        }

        if declared_row[row_index] < -MOMENT_ABS_TOLERANCE {
            return Err(format!(
                "declared covariance has negative diagonal at [{row_index}][{row_index}]"
            ));
        }
    }

    Ok(count)
}

fn field<'a>(
    record: &'a StringRecord,
    index: usize,
    name: &str,
) -> Result<&'a str, String> {
    record
        .get(index)
        .ok_or_else(|| format!("missing value for CSV column {name:?}"))
}

fn number_at(root: &Value, path: &[&str]) -> Result<f64, Vec<String>> {
    let value = path
        .iter()
        .try_fold(root, |current, key| current.get(*key))
        .ok_or_else(|| vec![format!("missing numeric field {}", path.join("."))])?;

    value
        .as_f64()
        .filter(|number| number.is_finite())
        .ok_or_else(|| vec![format!("field {} must be finite numeric data", path.join("."))])
}

fn string_at(root: &Value, path: &[&str]) -> Result<String, Vec<String>> {
    let value = path
        .iter()
        .try_fold(root, |current, key| current.get(*key))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| vec![format!("field {} must be a non-empty string", path.join("."))])?;

    Ok(value.to_owned())
}

fn number_array_at<const N: usize>(
    root: &Value,
    path: &[&str],
    expected: usize,
) -> Result<[f64; N], Vec<String>> {
    let values = path
        .iter()
        .try_fold(root, |current, key| current.get(*key))
        .and_then(Value::as_array)
        .ok_or_else(|| vec![format!("field {} must be an array", path.join("."))])?;

    if values.len() != expected || expected != N {
        return Err(vec![format!(
            "field {} must have exactly {expected} entries",
            path.join(".")
        )]);
    }

    let mut output = [0.0_f64; N];
    for (index, value) in values.iter().enumerate() {
        output[index] = value.as_f64().filter(|number| number.is_finite()).ok_or_else(|| {
            vec![format!(
                "field {}[{index}] must be finite numeric data",
                path.join(".")
            )]
        })?;
    }

    Ok(output)
}

fn matrix_array_at<const N: usize>(
    root: &Value,
    path: &[&str],
    expected: usize,
) -> Result<[[f64; N]; N], Vec<String>> {
    let rows = path
        .iter()
        .try_fold(root, |current, key| current.get(*key))
        .and_then(Value::as_array)
        .ok_or_else(|| vec![format!("field {} must be a matrix", path.join("."))])?;

    if rows.len() != expected || expected != N {
        return Err(vec![format!(
            "field {} must have exactly {expected} rows",
            path.join(".")
        )]);
    }

    let mut output = [[0.0_f64; N]; N];
    for (row_index, row) in rows.iter().enumerate() {
        let values = row.as_array().ok_or_else(|| {
            vec![format!(
                "field {}[{row_index}] must be an array",
                path.join(".")
            )]
        })?;
        if values.len() != expected {
            return Err(vec![format!(
                "field {}[{row_index}] must have exactly {expected} columns",
                path.join(".")
            )]);
        }

        for (column_index, value) in values.iter().enumerate() {
            output[row_index][column_index] =
                value.as_f64().filter(|number| number.is_finite()).ok_or_else(|| {
                    vec![format!(
                        "field {}[{row_index}][{column_index}] must be finite numeric data",
                        path.join(".")
                    )]
                })?;
        }
    }

    Ok(output)
}

fn close_relative(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance * left.abs().max(right.abs()).max(1.0)
}

fn close_moment(left: f64, right: f64) -> bool {
    (left - right).abs() <= MOMENT_ABS_TOLERANCE + 1.0e-9 * left.abs().max(right.abs())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture_paths() -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let docs = root.join("docs/research/muon-collider");
        (
            docs.join("beam-state-handoff.schema.json"),
            docs.join("fixtures/beam-handoff.synthetic.example.json"),
            docs.join("fixtures/beam-handoff.full-distribution.synthetic.example.json"),
        )
    }

    fn load_json(path: &Path) -> Value {
        let bytes = fs::read(path).expect("fixture must be readable");
        serde_json::from_slice(&bytes).expect("fixture must be valid JSON")
    }

    fn compiled_validator(schema_path: &Path) -> Validator {
        let schema: Value =
            serde_json::from_slice(&fs::read(schema_path).expect("schema must be readable"))
                .expect("schema JSON must parse");
        assert!(jsonschema::draft202012::meta::is_valid(&schema));
        jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("schema must compile")
    }

    #[test]
    fn summary_only_fixture_is_valid_but_not_physically_qualified() {
        let (schema, summary, _) = fixture_paths();
        let validator = compiled_validator(&schema);
        let envelope = load_json(&summary);
        let result = validate_document(
            &validator,
            &envelope,
            summary.parent().expect("fixtures directory"),
        );
        assert!(result.is_ok(), "{result:?}");
        let report = result.expect("summary fixture should validate");
        assert!(report.asset_sha256.is_none());
        assert!(report.macro_particle_count.is_none());
        assert_eq!(
            envelope["qualification"]["status"].as_str(),
            Some("not_evaluated")
        );
    }

    #[test]
    fn full_distribution_fixture_checks_digest_and_six_dimensional_moments() {
        let (schema, _, full) = fixture_paths();
        let validator = compiled_validator(&schema);
        let envelope = load_json(&full);
        let result = validate_document(
            &validator,
            &envelope,
            full.parent().expect("fixtures directory"),
        );
        assert!(result.is_ok(), "{result:?}");
        let report = result.expect("full fixture should validate");
        assert_eq!(
            report.asset_sha256.as_deref(),
            Some("759c2c5372f4fd4fe959a65b7a70be6dcde3113df8a233944ad17e6f205f81b9")
        );
        assert_eq!(report.macro_particle_count, Some(8));
    }

    #[test]
    fn negative_control_missing_coordinate_mapping_is_rejected() {
        let (schema, _, full) = fixture_paths();
        let validator = compiled_validator(&schema);
        let mut envelope = load_json(&full);
        envelope["coordinates"]
            .as_object_mut()
            .expect("coordinates object")
            .remove("native_coordinate_mapping");
        let result = validate_document(
            &validator,
            &envelope,
            full.parent().expect("fixtures directory"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn negative_control_missing_covariance_definition_is_rejected() {
        let (schema, _, full) = fixture_paths();
        let validator = compiled_validator(&schema);
        let mut envelope = load_json(&full);
        envelope["distribution"]["summary"]
            .as_object_mut()
            .expect("summary object")
            .remove("covariance_definition");
        let result = validate_document(
            &validator,
            &envelope,
            full.parent().expect("fixtures directory"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn negative_control_malformed_digest_is_rejected() {
        let (schema, _, full) = fixture_paths();
        let validator = compiled_validator(&schema);
        let mut envelope = load_json(&full);
        envelope["distribution"]["asset"]["sha256"] = json!("not-a-digest");
        let result = validate_document(
            &validator,
            &envelope,
            full.parent().expect("fixtures directory"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn negative_control_corrupted_digest_is_rejected() {
        let (schema, _, full) = fixture_paths();
        let validator = compiled_validator(&schema);
        let mut envelope = load_json(&full);
        envelope["distribution"]["asset"]["sha256"] =
            json!("0000000000000000000000000000000000000000000000000000000000000000");
        let result = validate_document(
            &validator,
            &envelope,
            full.parent().expect("fixtures directory"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn negative_control_population_residual_is_rejected() {
        let (schema, _, full) = fixture_paths();
        let validator = compiled_validator(&schema);
        let mut envelope = load_json(&full);
        envelope["population_ledger"]["losses"][0]["weighted_count"] = json!(2.0);
        let result = validate_document(
            &validator,
            &envelope,
            full.parent().expect("fixtures directory"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn negative_control_distribution_population_mismatch_is_rejected() {
        let (schema, _, full) = fixture_paths();
        let validator = compiled_validator(&schema);
        let mut envelope = load_json(&full);
        envelope["distribution"]["weighted_population"] = json!(9.0);
        let result = validate_document(
            &validator,
            &envelope,
            full.parent().expect("fixtures directory"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn negative_control_particle_production_requires_source_accounting() {
        let (schema, _, full) = fixture_paths();
        let validator = compiled_validator(&schema);
        let mut envelope = load_json(&full);
        envelope["population_ledger"]["accounting_scope"] = json!("particle_production");
        let result = validate_document(
            &validator,
            &envelope,
            full.parent().expect("fixtures directory"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn negative_control_schema_only_cannot_qualify_a_handoff() {
        let (schema, summary, _) = fixture_paths();
        let validator = compiled_validator(&schema);
        let mut envelope = load_json(&summary);
        envelope["handoff"]["acceptance_status"] = json!("qualified");
        let result = validate_document(
            &validator,
            &envelope,
            summary.parent().expect("fixtures directory"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn negative_control_asset_path_traversal_is_rejected() {
        let (schema, _, full) = fixture_paths();
        let validator = compiled_validator(&schema);
        let mut envelope = load_json(&full);
        envelope["distribution"]["asset"]["uri"] = json!("../../Cargo.toml");
        let result = validate_document(
            &validator,
            &envelope,
            full.parent().expect("fixtures directory"),
        );
        assert!(result.is_err());
    }
}
