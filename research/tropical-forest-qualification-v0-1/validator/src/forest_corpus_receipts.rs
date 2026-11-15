// SPDX-License-Identifier: Apache-2.0 OR MIT
//! CLI receipt adapters for the bounded forest uncertainty diagnostics.
//!
//! These adapters validate public synthetic corpora and emit deterministic,
//! source-digest-bound receipts. They do not establish ecological validity.

use crate::coverage_inference_kernel;
use crate::spatial_localization_kernel;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn string_at(root: &Value, key: &str) -> Result<String, String> {
    root.get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn number_at(root: &Value, key: &str) -> Result<f64, String> {
    let value = root.get(key).and_then(Value::as_f64)
        .ok_or_else(|| format!("{key} must be a number"))?;
    if !value.is_finite() {
        return Err(format!("{key} must be finite"));
    }
    Ok(value)
}

fn usize_at(root: &Value, key: &str) -> Result<usize, String> {
    let value = root.get(key).and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be a non-negative integer"))?;
    usize::try_from(value).map_err(|_| format!("{key} exceeds platform usize"))
}

fn close_enough(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance
}

fn check(id: &str, passed: bool, detail: impl Into<String>) -> Value {
    json!({"id": id, "passed": passed, "detail": detail.into()})
}

fn receipt(
    raw: &[u8],
    benchmark_id: &str,
    corpus_id: &str,
    result_name: &str,
    checks: Vec<Value>,
    extra: Value,
) -> Value {
    let passed = checks.iter().all(|item| item["passed"].as_bool() == Some(true));
    let passed_count = checks.iter()
        .filter(|item| item["passed"].as_bool() == Some(true))
        .count();
    let mut result = json!({
        "receipt_version": "0.1.0",
        "validator": "symtropy-tropical-forest-qualifier",
        "benchmark_id": benchmark_id,
        "corpus_id": corpus_id,
        "source_manifest_sha256": sha256_hex(raw),
        "result": if passed { result_name } else { "FAIL" },
        "checks_passed": passed_count,
        "checks_total": checks.len(),
        "checks": checks,
        "external_data_ingested": false,
        "ecological_validation": "NOT_ESTABLISHED"
    });
    if let (Some(target), Some(source)) = (result.as_object_mut(), extra.as_object()) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    result
}

pub fn validate_block_coverage_inference(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let benchmark_id = string_at(manifest, "benchmark_id")?;
    let corpus_id = string_at(manifest, "corpus_id")?;
    let version = string_at(manifest, "version")?;
    let method = string_at(manifest, "method")?;
    let unit = string_at(manifest, "unit")?;
    let independence = string_at(manifest, "independence_status")?;
    let claim_ceiling = string_at(manifest, "claim_ceiling")?;
    let confidence_level = number_at(manifest, "confidence_level")?;
    let cases = manifest.get("cases").and_then(Value::as_array)
        .ok_or_else(|| "cases must be an array".to_string())?;

    let mut checks = vec![check(
        "BLOCK-COVERAGE-CORPUS-IDENTITY-001",
        version == "0.1.0"
            && benchmark_id == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-BLOCK-COVERAGE-INFERENCE-001"
            && method == "TWO_SIDED_HOEFFDING"
            && unit == "DECLARED_INDEPENDENT_TEST_BLOCK_COVERAGE_INDICATOR"
            && independence == "INDEPENDENCE_ASSUMPTION_DECLARED_NOT_VERIFIED"
            && claim_ceiling.contains("does not verify block independence"),
        format!("version={version}; method={method}; independence={independence}"),
    )];

    let expected_ids: BTreeSet<&str> = [
        "three-blocks-one-covered",
        "three-blocks-zero-covered",
        "three-blocks-all-covered",
        "twenty-blocks-eighteen-covered",
        "one-hundred-blocks-ninety-covered",
        "738-blocks-702-covered",
        "pseudo-replication-observation-level-negative-control",
    ].into_iter().collect();
    let mut seen = BTreeSet::new();
    let mut accepted_count = 0usize;
    let mut negative_count = 0usize;
    let mut rows = Vec::new();

    for case in cases {
        let case_id = string_at(case, "case_id")?;
        let unique = seen.insert(case_id.clone());
        if case.get("covered_blocks").is_some() {
            accepted_count += 1;
            let covered = usize_at(case, "covered_blocks")?;
            let total = usize_at(case, "total_blocks")?;
            let result = coverage_inference_kernel::hoeffding_block_coverage_interval(
                covered,
                total,
                confidence_level,
            ).map_err(|error| format!("case {case_id}: {error}"))?;
            let expected = case.get("expected")
                .ok_or_else(|| format!("case {case_id} missing expected object"))?;
            let point_matches = close_enough(
                result.observed_block_coverage,
                number_at(expected, "observed_block_coverage")?,
                1e-12,
            );
            let radius_matches = close_enough(
                result.hoeffding_radius,
                number_at(expected, "hoeffding_radius")?,
                1e-12,
            );
            let lower_matches = close_enough(
                result.lower_bound,
                number_at(expected, "lower_bound")?,
                1e-12,
            );
            let upper_matches = close_enough(
                result.upper_bound,
                number_at(expected, "upper_bound")?,
                1e-12,
            );
            let case_passed = unique && covered <= total
                && point_matches && radius_matches && lower_matches && upper_matches
                && result.total_blocks == total
                && result.independence_status == "INDEPENDENT_BLOCK_ASSUMPTION_DECLARED_NOT_VERIFIED";
            checks.push(check(
                "BLOCK-COVERAGE-CASE-ORACLE-001",
                case_passed,
                format!("case={case_id}; n_blocks={total}; point={}; interval=[{},{}]",
                    result.observed_block_coverage, result.lower_bound, result.upper_bound),
            ));
            rows.push(json!({
                "case_id": case_id,
                "covered_blocks": covered,
                "total_blocks": total,
                "observed_block_coverage": result.observed_block_coverage,
                "confidence_level": result.confidence_level,
                "hoeffding_radius": result.hoeffding_radius,
                "lower_bound": result.lower_bound,
                "upper_bound": result.upper_bound,
                "independence_status": result.independence_status
            }));
        } else {
            negative_count += 1;
            let covered = usize_at(case, "covered_observations")?;
            let total = usize_at(case, "total_observations")?;
            let negative_unit = string_at(case, "unit")?;
            let disposition = string_at(case, "expected_disposition")?;
            if total == 0 || covered > total {
                return Err(format!("case {case_id} invalid negative-control counts"));
            }
            let count = f64::from(u32::try_from(total)
                .map_err(|_| format!("case {case_id} total too large"))?);
            let successes = f64::from(u32::try_from(covered)
                .map_err(|_| format!("case {case_id} covered too large"))?);
            let point = successes / count;
            let radius = ((2.0 / (1.0 - confidence_level)).ln() / (2.0 * count)).sqrt();
            let expected = case.get("expected_if_misused")
                .ok_or_else(|| format!("case {case_id} missing expected_if_misused"))?;
            let negative_arithmetic_matches =
                close_enough(point, number_at(expected, "observed_coverage")?, 1e-12)
                && close_enough(radius, number_at(expected, "hoeffding_radius")?, 1e-12)
                && close_enough((point - radius).max(0.0), number_at(expected, "lower_bound")?, 1e-12)
                && close_enough((point + radius).min(1.0), number_at(expected, "upper_bound")?, 1e-12);
            let rejected = unique
                && negative_unit == "CORRELATED_OBSERVATIONS_NOT_INDEPENDENT_BLOCKS"
                && disposition == "REJECT_AS_BLOCK_COVERAGE_INFERENCE"
                && negative_arithmetic_matches;
            checks.push(check(
                "BLOCK-COVERAGE-PSEUDO-REPLICATION-NEGATIVE-CONTROL-001",
                rejected,
                format!("case={case_id}; observations={total}; reject_as_block_inference={rejected}"),
            ));
            rows.push(json!({
                "case_id": case_id,
                "covered_observations": covered,
                "total_observations": total,
                "disposition": if rejected { "REJECT_AS_BLOCK_COVERAGE_INFERENCE" } else { "FAIL_CONTROL" }
            }));
        }
    }

    let exact_cases = cases.len() == expected_ids.len()
        && seen.len() == expected_ids.len()
        && seen.iter().all(|id| expected_ids.contains(id.as_str()));
    checks.push(check(
        "BLOCK-COVERAGE-CASE-SET-001",
        exact_cases && accepted_count == 6 && negative_count == 1,
        format!("unique_cases={}; accepted={accepted_count}; negative_controls={negative_count}", seen.len()),
    ));

    Ok(receipt(
        raw,
        &benchmark_id,
        &corpus_id,
        "PASS_SYNTHETIC_BLOCK_COVERAGE_INFERENCE",
        checks,
        json!({
            "accepted_case_count": accepted_count,
            "negative_control_count": negative_count,
            "coverage_unit": "DECLARED_INDEPENDENT_BLOCK",
            "independence_status": "INDEPENDENT_BLOCK_ASSUMPTION_DECLARED_NOT_VERIFIED",
            "dependent_block_inference": "NOT_IMPLEMENTED",
            "case_results": rows
        }),
    ))
}

fn expand_case(
    case: &Value,
    coordinate_system_id: &str,
    bandwidth_m: f64,
    alpha: f64,
    minimum_ess: f64,
) -> Result<(spatial_localization_kernel::SpatialLocalizationScenario, Vec<(usize, usize)>), String> {
    let calibration_groups = case.get("calibration").and_then(Value::as_array)
        .ok_or_else(|| "calibration must be an array".to_string())?;
    let test_groups = case.get("test").and_then(Value::as_array)
        .ok_or_else(|| "test must be an array".to_string())?;
    let mut calibration = Vec::new();
    for (group_index, group) in calibration_groups.iter().enumerate() {
        let count = usize_at(group, "count")?;
        if count == 0 || count > 10_000 || calibration.len().saturating_add(count) > 100_000 {
            return Err(format!("calibration group {group_index} exceeds expansion bounds"));
        }
        let x_m = number_at(group, "x_m")?;
        let y_m = number_at(group, "y_m")?;
        let residual = number_at(group, "residual")?;
        for item in 0..count {
            calibration.push(spatial_localization_kernel::SpatialCalibrationPoint {
                point_id: format!("cal-{group_index}-{item}"),
                x_m,
                y_m,
                absolute_residual: residual,
            });
        }
    }

    let mut test_points = Vec::new();
    let mut ranges = Vec::new();
    for (group_index, group) in test_groups.iter().enumerate() {
        let count = usize_at(group, "count")?;
        if count == 0 || count > 10_000 || test_points.len().saturating_add(count) > 100_000 {
            return Err(format!("test group {group_index} exceeds expansion bounds"));
        }
        let x_m = number_at(group, "x_m")?;
        let y_m = number_at(group, "y_m")?;
        let prediction = number_at(group, "prediction")?;
        let observation = number_at(group, "observation")?;
        let start = test_points.len();
        for item in 0..count {
            test_points.push(spatial_localization_kernel::SpatialTestPoint {
                point_id: format!("test-{group_index}-{item}"),
                x_m,
                y_m,
                point_prediction: prediction,
                observation,
            });
        }
        ranges.push((start, test_points.len()));
    }

    Ok((
        spatial_localization_kernel::SpatialLocalizationScenario {
            coordinate_system_id: coordinate_system_id.to_string(),
            bandwidth_m,
            alpha,
            minimum_effective_sample_size: minimum_ess,
            calibration_points: calibration,
            test_points,
        },
        ranges,
    ))
}

pub fn validate_spatial_localization(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let benchmark_id = string_at(manifest, "benchmark_id")?;
    let corpus_id = string_at(manifest, "corpus_id")?;
    let version = string_at(manifest, "version")?;
    let method = string_at(manifest, "method")?;
    let method_status = string_at(manifest, "method_status")?;
    let claim_ceiling = string_at(manifest, "claim_ceiling")?;
    let coordinates = manifest.get("coordinates")
        .ok_or_else(|| "coordinates object missing".to_string())?;
    let crs = string_at(coordinates, "required_crs")?;
    let units = string_at(coordinates, "units")?;
    let minimum_ess = number_at(coordinates, "minimum_effective_sample_size")?;
    let cases = manifest.get("cases").and_then(Value::as_array)
        .ok_or_else(|| "cases must be an array".to_string())?;

    let mut checks = vec![check(
        "SPATIAL-LOCALIZATION-CORPUS-IDENTITY-001",
        version == "0.1.0"
            && corpus_id == "FOREST-SPATIAL-LOCALIZATION-001"
            && benchmark_id == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && method == "GAUSSIAN_DISTANCE_WEIGHTED_RESIDUAL_QUANTILE"
            && method_status == "EXPERIMENTAL_LOCALIZED_DIAGNOSTIC_NO_GUARANTEE"
            && crs == "ONE_DECLARED_PROJECTED_METRIC_CRS"
            && units == "METRES"
            && claim_ceiling.contains("no finite-sample spatial coverage"),
        format!("version={version}; method={method}; status={method_status}; crs={crs}"),
    )];

    let expected_ids: BTreeSet<&str> = [
        "COLOCATED_TEN_CALIBRATION_POINTS",
        "POOLED_NINETY_PERCENT_HIDES_LOCAL_FAILURE",
        "REMOTE_SITE_NO_LOCAL_CALIBRATION_MASS",
        "NARROW_BANDWIDTH_LOW_WEIGHTED_MASS",
        "BANDWIDTH_WIDENING_ADDS_LOCAL_WEIGHT",
    ].into_iter().collect();
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();

    for case in cases {
        let case_id = string_at(case, "case_id")?;
        let unique = seen.insert(case_id.clone());
        let alpha = number_at(case, "alpha")?;
        let expected = case.get("expected");
        if let Some(comparisons) = case.get("comparisons").and_then(Value::as_array) {
            let mut comparison_rows = Vec::new();
            let mut comparison_passed = unique;
            for comparison in comparisons {
                let bandwidth = number_at(comparison, "bandwidth_m")?;
                let (scenario, _) = expand_case(case, &crs, bandwidth, alpha, minimum_ess)?;
                let result = spatial_localization_kernel::evaluate_spatial_localization(&scenario)
                    .map_err(|error| format!("spatial case {case_id}: {error}"))?;
                let prediction = result.predictions.first()
                    .ok_or_else(|| format!("spatial case {case_id} has no predictions"))?;
                let expected_mass = number_at(comparison, "expected_weighted_mass")?;
                let expected_radius = comparison.get("expected_radius")
                    .filter(|value| !value.is_null()).and_then(Value::as_f64);
                let radius_match = match (prediction.radius, expected_radius) {
                    (Some(actual), Some(wanted)) => close_enough(actual, wanted, 1e-12),
                    (None, None) => true,
                    _ => false,
                };
                let passed = close_enough(prediction.weighted_calibration_mass, expected_mass, 1e-10)
                    && radius_match;
                comparison_passed &= passed;
                comparison_rows.push(json!({
                    "bandwidth_m": bandwidth,
                    "weighted_calibration_mass": prediction.weighted_calibration_mass,
                    "effective_sample_size": prediction.effective_sample_size,
                    "radius": prediction.radius,
                    "status": prediction.status,
                    "matches_reference": passed
                }));
            }
            checks.push(check("SPATIAL-LOCALIZATION-BANDWIDTH-ORACLE-001",
                comparison_passed, format!("case={case_id}; comparisons={}", comparisons.len())));
            rows.push(json!({"case_id":case_id,"comparisons":comparison_rows}));
            continue;
        }

        let bandwidth = number_at(case, "bandwidth_m")?;
        let (scenario, ranges) = expand_case(case, &crs, bandwidth, alpha, minimum_ess)?;
        let result = spatial_localization_kernel::evaluate_spatial_localization(&scenario)
            .map_err(|error| format!("spatial case {case_id}: {error}"))?;
        let mut passed = unique && result.method_status == method_status;
        let expected = expected.ok_or_else(|| format!("case {case_id} expected object missing"))?;
        if let Some(wanted) = expected.get("pooled_radius").and_then(Value::as_f64) {
            passed &= result.pooled_radius.is_some_and(|actual| close_enough(actual, wanted, 1e-12));
        }

        let mut group_rows = Vec::new();
        for (group_index, (start, end)) in ranges.iter().enumerate() {
            let predictions = &result.predictions[*start..*end];
            let first = predictions.first()
                .ok_or_else(|| format!("case {case_id} has empty expanded test group"))?;
            let covered_count = predictions.iter()
                .filter(|prediction| prediction.covered == Some(true)).count();
            let uncovered_count = predictions.iter()
                .filter(|prediction| prediction.covered == Some(false)).count();
            let coverage = if covered_count + uncovered_count == predictions.len() {
                Some(f64::from(u32::try_from(covered_count).map_err(|_| "coverage count overflow")?)
                    / f64::from(u32::try_from(predictions.len()).map_err(|_| "prediction count overflow")?))
            } else {
                None
            };
            group_rows.push(json!({
                "group_index": group_index,
                "count": predictions.len(),
                "coverage": coverage,
                "radius": first.radius,
                "effective_sample_size": first.effective_sample_size,
                "weighted_calibration_mass": first.weighted_calibration_mass,
                "status": first.status
            }));

            if case_id == "POOLED_NINETY_PERCENT_HIDES_LOCAL_FAILURE" {
                let keys = match group_index {
                    0 => ("region_a_coverage", "region_a_local_radius", "region_a_ess"),
                    1 => ("region_b_coverage", "region_b_local_radius", "region_b_ess"),
                    _ => return Err(format!("case {case_id} has unexpected region count")),
                };
                let wanted_coverage = number_at(expected, keys.0)?;
                let wanted_radius = number_at(expected, keys.1)?;
                let wanted_ess = number_at(expected, keys.2)?;
                passed &= coverage.is_some_and(|value| close_enough(value, wanted_coverage, 1e-12))
                    && first.radius.is_some_and(|value| close_enough(value, wanted_radius, 1e-12))
                    && close_enough(first.effective_sample_size, wanted_ess, 1e-10);
            } else if ranges.len() == 1 {
                if let Some(wanted) = expected.get("local_radius") {
                    let wanted = if wanted.is_null() { None } else { wanted.as_f64() };
                    passed &= match (first.radius, wanted) {
                        (Some(actual), Some(wanted)) => close_enough(actual, wanted, 1e-12),
                        (None, None) => true,
                        _ => false,
                    };
                }
                if expected.get("weighted_calibration_mass").is_some() {
                    passed &= close_enough(first.weighted_calibration_mass,
                        number_at(expected, "weighted_calibration_mass")?, 1e-10);
                }
                if expected.get("effective_sample_size").is_some() {
                    passed &= close_enough(first.effective_sample_size,
                        number_at(expected, "effective_sample_size")?, 1e-10);
                }
                if expected.get("maximum_normalized_weight").is_some() {
                    passed &= close_enough(first.maximum_normalized_weight,
                        number_at(expected, "maximum_normalized_weight")?, 1e-12);
                }
                if expected.get("interval_width").is_some() {
                    passed &= first.interval_width.is_some_and(|actual| {
                        number_at(expected, "interval_width")
                            .is_ok_and(|wanted| close_enough(actual, wanted, 1e-12))
                    });
                }
                if let Some(wanted) = expected.get("covered").and_then(Value::as_bool) {
                    passed &= first.covered == Some(wanted);
                } else if expected.get("covered").is_some_and(Value::is_null) {
                    passed &= first.covered.is_none();
                }
                if let Some(wanted) = expected.get("disposition").and_then(Value::as_str) {
                    passed &= first.status == wanted;
                }
            }
        }
        if case_id == "POOLED_NINETY_PERCENT_HIDES_LOCAL_FAILURE" {
            let total = result.predictions.len();
            let covered = result.predictions.iter()
                .filter(|prediction| prediction.covered == Some(true)).count();
            let actual = f64::from(u32::try_from(covered).map_err(|_| "coverage count overflow")?)
                / f64::from(u32::try_from(total).map_err(|_| "test count overflow")?);
            passed &= close_enough(actual, number_at(expected, "aggregate_coverage")?, 1e-12);
        }
        checks.push(check("SPATIAL-LOCALIZATION-CASE-ORACLE-001", passed,
            format!("case={case_id}; groups={}; predictions={}; pooled_radius={:?}",
                ranges.len(), result.predictions.len(), result.pooled_radius)));
        rows.push(json!({
            "case_id": case_id,
            "pooled_radius": result.pooled_radius,
            "method_status": result.method_status,
            "groups": group_rows
        }));
    }

    checks.push(check(
        "SPATIAL-LOCALIZATION-CASE-SET-001",
        cases.len() == expected_ids.len()
            && seen.len() == expected_ids.len()
            && seen.iter().all(|id| expected_ids.contains(id.as_str())),
        format!("unique_cases={}; required={}", seen.len(), expected_ids.len()),
    ));

    let moran_cases = manifest.get("moran_i_reference_cases").and_then(Value::as_array)
        .ok_or_else(|| "moran_i_reference_cases must be an array".to_string())?;
    let expected_moran_ids: BTreeSet<&str> = [
        "positive-clustered-residuals",
        "negative-alternating-residuals",
        "no-neighbours",
        "zero-residual-variance",
    ].into_iter().collect();
    let mut moran_seen = BTreeSet::new();
    let mut moran_rows = Vec::new();
    for case in moran_cases {
        let case_id = string_at(case, "case_id")?;
        let radius = number_at(case, "neighbour_radius_m")?;
        let values = case.get("points").and_then(Value::as_array)
            .ok_or_else(|| format!("Moran case {case_id} points must be an array"))?;
        let mut points = Vec::new();
        for point in values {
            points.push(spatial_localization_kernel::SpatialResidualPoint {
                point_id: string_at(point, "id")?,
                x_m: number_at(point, "x")?,
                y_m: point.get("y").and_then(Value::as_f64).unwrap_or(0.0),
                residual: number_at(point, "residual")?,
            });
        }
        let actual = spatial_localization_kernel::global_morans_i(&points, radius)
            .map_err(|error| format!("Moran case {case_id}: {error}"))?;
        let expected_value = case.get("expected_morans_i")
            .filter(|value| !value.is_null()).and_then(Value::as_f64);
        let passed = moran_seen.insert(case_id.clone())
            && match (actual, expected_value) {
                (Some(actual), Some(wanted)) => close_enough(actual, wanted, 1e-12),
                (None, None) => true,
                _ => false,
            };
        checks.push(check("SPATIAL-MORANS-I-ORACLE-001", passed,
            format!("case={case_id}; actual={actual:?}; expected={expected_value:?}")));
        moran_rows.push(json!({"case_id":case_id,"morans_i":actual}));
    }
    checks.push(check(
        "SPATIAL-MORANS-I-CASE-SET-001",
        moran_seen.len() == expected_moran_ids.len()
            && moran_seen.iter().all(|id| expected_moran_ids.contains(id.as_str())),
        format!("moran_cases={}; required={}", moran_seen.len(), expected_moran_ids.len()),
    ));

    Ok(receipt(
        raw,
        &benchmark_id,
        &corpus_id,
        "PASS_SYNTHETIC_SPATIAL_LOCALIZATION",
        checks,
        json!({
            "spatial_method_status": method_status,
            "spatial_coverage_guarantee": "NOT_ESTABLISHED",
            "moran_inference_p_value": "NOT_IMPLEMENTED",
            "case_count": seen.len(),
            "moran_case_count": moran_seen.len(),
            "case_results": rows,
            "moran_results": moran_rows
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK_FIXTURE: &str =
        include_str!("../../cases/forest-block-coverage-inference-synthetic-v0.1.json");
    const SPATIAL_FIXTURE: &str =
        include_str!("../../cases/forest-spatial-localization-synthetic-v0.1.json");

    #[test]
    fn block_coverage_corpus_emits_digest_bound_pass_receipt() {
        let manifest: Value = serde_json::from_str(BLOCK_FIXTURE).expect("valid block fixture");
        let receipt = validate_block_coverage_inference(BLOCK_FIXTURE.as_bytes(), &manifest)
            .expect("block coverage receipt");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_BLOCK_COVERAGE_INFERENCE");
        assert_eq!(receipt["accepted_case_count"], 6);
        assert_eq!(receipt["negative_control_count"], 1);
        assert_eq!(receipt["external_data_ingested"], false);
    }

    #[test]
    fn block_coverage_receipt_rejects_mutated_interval_oracle() {
        let mut manifest: Value = serde_json::from_str(BLOCK_FIXTURE).expect("valid block fixture");
        manifest["cases"][0]["expected"]["upper_bound"] = json!(0.25);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutation");
        let receipt = validate_block_coverage_inference(&bytes, &manifest)
            .expect("semantic mutation returns failure receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn block_coverage_receipt_rejects_observation_level_pseudoreplication() {
        let mut manifest: Value = serde_json::from_str(BLOCK_FIXTURE).expect("valid block fixture");
        manifest["cases"][6]["expected_disposition"] = json!("ACCEPT_AS_INDEPENDENT_BLOCKS");
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutation");
        let receipt = validate_block_coverage_inference(&bytes, &manifest)
            .expect("semantic mutation returns failure receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn spatial_localization_corpus_emits_pass_receipt_for_local_failure_and_moran_cases() {
        let manifest: Value = serde_json::from_str(SPATIAL_FIXTURE).expect("valid spatial fixture");
        let receipt = validate_spatial_localization(SPATIAL_FIXTURE.as_bytes(), &manifest)
            .expect("spatial receipt");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_SPATIAL_LOCALIZATION");
        assert_eq!(receipt["case_count"], 5);
        assert_eq!(receipt["moran_case_count"], 4);
        assert_eq!(receipt["spatial_coverage_guarantee"], "NOT_ESTABLISHED");
    }

    #[test]
    fn spatial_localization_receipt_rejects_mutated_region_coverage() {
        let mut manifest: Value = serde_json::from_str(SPATIAL_FIXTURE).expect("valid spatial fixture");
        manifest["cases"][1]["expected"]["region_b_coverage"] = json!(0.9);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutation");
        let receipt = validate_spatial_localization(&bytes, &manifest)
            .expect("semantic mutation returns failure receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn spatial_localization_receipt_rejects_mutated_moran_reference() {
        let mut manifest: Value = serde_json::from_str(SPATIAL_FIXTURE).expect("valid spatial fixture");
        manifest["moran_i_reference_cases"][0]["expected_morans_i"] = json!(-1.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutation");
        let receipt = validate_spatial_localization(&bytes, &manifest)
            .expect("semantic mutation returns failure receipt");
        assert_eq!(receipt["result"], "FAIL");
    }
}
