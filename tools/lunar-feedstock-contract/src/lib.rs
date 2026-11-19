// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeltaStatus {
    ResolvedIncrease,
    ResolvedDecrease,
    IndeterminateOverlap,
    UnknownMissingMeasurement,
}

impl DeltaStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::ResolvedIncrease => "resolved_increase",
            Self::ResolvedDecrease => "resolved_decrease",
            Self::IndeterminateOverlap => "indeterminate_interval_overlap",
            Self::UnknownMissingMeasurement => "unknown_missing_measurement",
        }
    }
}

fn finite_number(value: &Value) -> Option<f64> {
    value.as_f64().filter(|number| number.is_finite())
}

fn measurement(value: &Value) -> Option<(f64, f64)> {
    if value.get("status")?.as_str()? == "unknown" {
        return None;
    }
    let estimate = finite_number(value.get("value")?)?;
    let uncertainty = finite_number(value.get("standard_uncertainty")?)?;
    if uncertainty < 0.0 {
        return None;
    }
    Some((estimate, uncertainty))
}

fn status_for_delta(delta: f64, uncertainty: f64) -> DeltaStatus {
    let lower = delta - uncertainty;
    let upper = delta + uncertainty;
    if lower > 0.0 {
        DeltaStatus::ResolvedIncrease
    } else if upper < 0.0 {
        DeltaStatus::ResolvedDecrease
    } else {
        DeltaStatus::IndeterminateOverlap
    }
}

fn compare_measurements(from: &Value, to: &Value) -> (Value, bool) {
    let (Some((from_value, from_u)), Some((to_value, to_u))) =
        (measurement(from), measurement(to))
    else {
        return (
            json!({
                "from": from.get("value").cloned().unwrap_or(Value::Null),
                "to": to.get("value").cloned().unwrap_or(Value::Null),
                "delta": null,
                "conservative_combined_uncertainty_bound": null,
                "conservative_interval": null,
                "status": DeltaStatus::UnknownMissingMeasurement.as_str(),
            }),
            false,
        );
    };

    let from_unit = from.get("unit").and_then(Value::as_str);
    let to_unit = to.get("unit").and_then(Value::as_str);
    if from_unit.is_none() || to_unit.is_none() {
        return (
            json!({
                "from": from_value,
                "to": to_value,
                "delta": null,
                "conservative_combined_uncertainty_bound": null,
                "conservative_interval": null,
                "status": "unknown_missing_unit",
            }),
            false,
        );
    }
    if from_unit != to_unit {
        return (
            json!({
                "from": from_value,
                "to": to_value,
                "from_unit": from_unit,
                "to_unit": to_unit,
                "delta": null,
                "conservative_combined_uncertainty_bound": null,
                "conservative_interval": null,
                "status": "incompatible_units",
            }),
            false,
        );
    }

    let delta = to_value - from_value;
    let combined_u = from_u + to_u;
    let lower = delta - combined_u;
    let upper = delta + combined_u;
    let status = status_for_delta(delta, combined_u);

    (
        json!({
            "from": from_value,
            "to": to_value,
            "unit": from_unit,
            "delta": delta,
            "conservative_combined_uncertainty_bound": combined_u,
            "conservative_interval": {"lower": lower, "upper": upper},
            "status": status.as_str(),
        }),
        true,
    )
}

fn sample<'a>(run: &'a Value, sample_id: &str) -> Result<&'a Value, String> {
    run.get("quality_samples")
        .and_then(Value::as_array)
        .and_then(|samples| {
            samples
                .iter()
                .find(|entry| entry.get("sample_id").and_then(Value::as_str) == Some(sample_id))
        })
        .ok_or_else(|| format!("quality sample not found: {sample_id}"))
}

fn phase_map(sample: &Value) -> Result<Vec<(String, Value)>, String> {
    let fractions = sample
        .get("phase_fractions")
        .and_then(Value::as_array)
        .ok_or_else(|| "sample has no phase_fractions array".to_owned())?;

    let mut result: Vec<(String, Value)> = Vec::with_capacity(fractions.len());
    for entry in fractions {
        let phase = entry
            .get("phase_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "phase fraction is missing phase_id".to_owned())?;
        if result.iter().any(|(seen, _)| seen.as_str() == phase) {
            return Err(format!("duplicate phase_id in assay: {phase}"));
        }
        result.push((phase.to_owned(), entry.get("mass_fraction").cloned().unwrap_or(Value::Null)));
    }
    Ok(result)
}

fn size_bin_key(entry: &Value) -> Result<(f64, f64), String> {
    let low_measurement = entry
        .get("d_min_um")
        .ok_or_else(|| "size bin has no d_min_um".to_owned())?;
    let high_measurement = entry
        .get("d_max_um")
        .ok_or_else(|| "size bin has no d_max_um".to_owned())?;
    if low_measurement.get("unit").and_then(Value::as_str) != Some("um")
        || high_measurement.get("unit").and_then(Value::as_str) != Some("um")
    {
        return Err("size-bin bounds must use micrometres (um)".to_owned());
    }
    let low = measurement(low_measurement)
        .map(|pair| pair.0)
        .ok_or_else(|| "size bin has unknown/invalid d_min_um".to_owned())?;
    let high = measurement(high_measurement)
        .map(|pair| pair.0)
        .ok_or_else(|| "size bin has unknown/invalid d_max_um".to_owned())?;
    if low <= 0.0 || high <= low {
        return Err("size bin bounds must be positive and ordered".to_owned());
    }
    Ok((low, high))
}

fn compare_size_bins(from: &Value, to: &Value) -> Result<(Value, bool), String> {
    let from_bins = from
        .get("size_bins")
        .and_then(Value::as_array)
        .ok_or_else(|| "source sample has no size_bins array".to_owned())?;
    let to_bins = to
        .get("size_bins")
        .and_then(Value::as_array)
        .ok_or_else(|| "destination sample has no size_bins array".to_owned())?;

    if from_bins.len() != to_bins.len() {
        return Ok((json!({
            "status": "incompatible_bin_partitions",
            "from_bin_count": from_bins.len(),
            "to_bin_count": to_bins.len(),
            "measurements_available": false,
            "bins": [],
            "claim_ceiling": "No bin-by-bin change is inferred when the two assays use different bin partitions."
        }), false));
    }

    let mut output = Vec::with_capacity(from_bins.len());
    let mut all_measurements_available = true;
    for from_bin in from_bins {
        let from_key = size_bin_key(from_bin)?;
        let to_bin = to_bins
            .iter()
            .find(|candidate| size_bin_key(candidate).ok() == Some(from_key));
        let Some(to_bin) = to_bin else {
            return Ok((json!({
                "status": "incompatible_bin_partitions",
                "from_bin_count": from_bins.len(),
                "to_bin_count": to_bins.len(),
                "measurements_available": false,
                "bins": [],
                "claim_ceiling": "No bin-by-bin change is inferred when the two assays use different bin edges."
            }), false));
        };

        let (comparison, available) = compare_measurements(
            from_bin.get("mass_fraction").unwrap_or(&Value::Null),
            to_bin.get("mass_fraction").unwrap_or(&Value::Null),
        );
        all_measurements_available &= available;
        output.push(json!({
            "d_min_um": from_key.0,
            "d_max_um": from_key.1,
            "mass_fraction_change": comparison,
        }));
    }

    Ok((json!({
        "status": "matched_bin_partitions",
        "from_bin_count": from_bins.len(),
        "to_bin_count": to_bins.len(),
        "measurements_available": all_measurements_available,
        "bins": output,
        "claim_ceiling": "Bin-wise changes compare reported fractions only; they do not establish a physical segregation mechanism."
    }), all_measurements_available))
}

pub fn compare_quality_samples(
    run: &Value,
    from_sample_id: &str,
    to_sample_id: &str,
) -> Result<Value, String> {
    let from = sample(run, from_sample_id)?;
    let to = sample(run, to_sample_id)?;

    let from_stream = from
        .get("stream_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "source sample lacks stream_id".to_owned())?;
    let to_stream = to
        .get("stream_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "destination sample lacks stream_id".to_owned())?;
    let from_point = from
        .get("sample_point")
        .and_then(Value::as_str)
        .ok_or_else(|| "source sample lacks sample_point".to_owned())?;
    let to_point = to
        .get("sample_point")
        .and_then(Value::as_str)
        .ok_or_else(|| "destination sample lacks sample_point".to_owned())?;

    let from_phases = phase_map(from)?;
    let to_phases = phase_map(to)?;
    let mut phase_results = Vec::new();

    for (phase_id, from_measurement) in &from_phases {
        let Some((_, to_measurement)) = to_phases.iter().find(|(candidate, _)| candidate == phase_id)
        else {
            phase_results.push(json!({
                "phase_id": phase_id,
                "status": DeltaStatus::UnknownMissingMeasurement.as_str(),
                "reason": "phase is absent from destination assay"
            }));
            continue;
        };
        let (comparison, _) = compare_measurements(from_measurement, to_measurement);
        phase_results.push(json!({
            "phase_id": phase_id,
            "mass_fraction_change": comparison
        }));
    }

    for (phase_id, _) in &to_phases {
        if !from_phases.iter().any(|(candidate, _)| candidate == phase_id) {
            phase_results.push(json!({
                "phase_id": phase_id,
                "status": DeltaStatus::UnknownMissingMeasurement.as_str(),
                "reason": "phase is absent from source assay"
            }));
        }
    }

    let (target_size, target_size_available) = compare_measurements(
        from.get("target_size_mass_fraction").unwrap_or(&Value::Null),
        to.get("target_size_mass_fraction").unwrap_or(&Value::Null),
    );
    let (size_bins, size_bins_available) = compare_size_bins(from, to)?;

    let has_unknown_phase = phase_results.iter().any(|entry| {
        let status = entry
            .get("status")
            .or_else(|| entry.get("mass_fraction_change").and_then(|change| change.get("status")))
            .and_then(Value::as_str);
        status.is_some_and(|value| value.starts_with("unknown_"))
    });

    let has_incompatible_units = phase_results.iter().any(|entry| {
        entry.get("mass_fraction_change")
            .and_then(|change| change.get("status"))
            .and_then(Value::as_str) == Some("incompatible_units")
    }) || target_size.get("status").and_then(Value::as_str) == Some("incompatible_units")
        || size_bins.get("bins").and_then(Value::as_array).is_some_and(|bins| {
            bins.iter().any(|entry| {
                entry.get("mass_fraction_change")
                    .and_then(|change| change.get("status"))
                    .and_then(Value::as_str) == Some("incompatible_units")
            })
        });

    let report_status = if size_bins.get("status").and_then(Value::as_str)
        == Some("incompatible_bin_partitions")
    {
        "indeterminate_incompatible_size_bins"
    } else if has_incompatible_units {
        "indeterminate_incompatible_units"
    } else if has_unknown_phase || !target_size_available || !size_bins_available {
        "indeterminate_missing_quality_measurement"
    } else {
        "comparisons_available"
    };

    Ok(json!({
        "report_class": "process_stage_quality_drift_comparison",
        "evidence_class": run.get("evidence_class").cloned().unwrap_or(Value::Null),
        "run_id": run.get("run_id").cloned().unwrap_or(Value::Null),
        "from": {
            "sample_id": from_sample_id,
            "stream_id": from_stream,
            "sample_point": from_point,
        },
        "to": {
            "sample_id": to_sample_id,
            "stream_id": to_stream,
            "sample_point": to_point,
        },
        "phase_fraction_changes": phase_results,
        "target_size_mass_fraction_change": target_size,
        "size_bin_changes": size_bins,
        "overall_status": report_status,
        "claim_ceiling": "This report describes paired assay changes with combined standard uncertainty. It does not establish causation, a segregation mechanism, assay accuracy, physical process performance or lunar qualification."
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run_fixture() -> Value {
        json!({
          "run_id": "SYN-DRIFT-001",
          "evidence_class": "synthetic_fixture",
          "quality_samples": [
            {
              "sample_id": "A",
              "stream_id": "S-A",
              "sample_point": "separator_outlet",
              "phase_fractions": [
                {"phase_id":"plagioclase","mass_fraction":{"value":0.70,"standard_uncertainty":0.01,"unit":"kg/kg","status":"synthetic"}},
                {"phase_id":"pyroxene","mass_fraction":{"value":0.20,"standard_uncertainty":0.01,"unit":"kg/kg","status":"synthetic"}}
              ],
              "size_bins": [
                {"d_min_um":{"value":20,"standard_uncertainty":0.1,"unit":"um","status":"synthetic"},"d_max_um":{"value":200,"standard_uncertainty":0.1,"unit":"um","status":"synthetic"},"mass_fraction":{"value":0.9,"standard_uncertainty":0.01,"unit":"kg/kg","status":"synthetic"}},
                {"d_min_um":{"value":3,"standard_uncertainty":0.1,"unit":"um","status":"synthetic"},"d_max_um":{"value":20,"standard_uncertainty":0.1,"unit":"um","status":"synthetic"},"mass_fraction":{"value":0.1,"standard_uncertainty":0.01,"unit":"kg/kg","status":"synthetic"}}
              ],
              "target_size_mass_fraction":{"value":0.9,"standard_uncertainty":0.01,"unit":"kg/kg","status":"synthetic"}
            },
            {
              "sample_id": "B",
              "stream_id": "S-B",
              "sample_point": "reactor_inlet",
              "phase_fractions": [
                {"phase_id":"plagioclase","mass_fraction":{"value":0.73,"standard_uncertainty":0.01,"unit":"kg/kg","status":"synthetic"}},
                {"phase_id":"pyroxene","mass_fraction":{"value":0.17,"standard_uncertainty":0.01,"unit":"kg/kg","status":"synthetic"}}
              ],
              "size_bins": [
                {"d_min_um":{"value":20,"standard_uncertainty":0.1,"unit":"um","status":"synthetic"},"d_max_um":{"value":200,"standard_uncertainty":0.1,"unit":"um","status":"synthetic"},"mass_fraction":{"value":0.85,"standard_uncertainty":0.01,"unit":"kg/kg","status":"synthetic"}},
                {"d_min_um":{"value":3,"standard_uncertainty":0.1,"unit":"um","status":"synthetic"},"d_max_um":{"value":20,"standard_uncertainty":0.1,"unit":"um","status":"synthetic"},"mass_fraction":{"value":0.15,"standard_uncertainty":0.01,"unit":"kg/kg","status":"synthetic"}}
              ],
              "target_size_mass_fraction":{"value":0.85,"standard_uncertainty":0.01,"unit":"kg/kg","status":"synthetic"}
            }
          ]
        })
    }

    #[test]
    fn resolves_phase_increase_only_when_combined_interval_excludes_zero() {
        let report = compare_quality_samples(&run_fixture(), "A", "B").unwrap();
        let phase = &report["phase_fraction_changes"][0]["mass_fraction_change"];
        assert_eq!(phase["status"], "resolved_increase");
        assert!((phase["delta"].as_f64().unwrap() - 0.03).abs() < 1e-12);
    }

    #[test]
    fn reports_indeterminate_when_change_interval_overlaps_zero() {
        let mut run = run_fixture();
        run["quality_samples"][1]["phase_fractions"][0]["mass_fraction"]["value"] = json!(0.711);
        let report = compare_quality_samples(&run, "A", "B").unwrap();
        assert_eq!(
            report["phase_fraction_changes"][0]["mass_fraction_change"]["status"],
            "indeterminate_interval_overlap"
        );
    }

    #[test]
    fn unknown_assay_measurement_is_not_replaced_with_zero() {
        let mut run = run_fixture();
        run["quality_samples"][1]["phase_fractions"][0]["mass_fraction"]["status"] = json!("unknown");
        run["quality_samples"][1]["phase_fractions"][0]["mass_fraction"]["value"] = Value::Null;
        run["quality_samples"][1]["phase_fractions"][0]["mass_fraction"]["standard_uncertainty"] = Value::Null;
        let report = compare_quality_samples(&run, "A", "B").unwrap();
        assert_eq!(report["overall_status"], "indeterminate_missing_quality_measurement");
    }

    #[test]
    fn different_size_bin_partitions_are_not_compared_as_if_aligned() {
        let mut run = run_fixture();
        run["quality_samples"][1]["size_bins"][0]["d_max_um"]["value"] = json!(180.0);
        let report = compare_quality_samples(&run, "A", "B").unwrap();
        assert_eq!(report["overall_status"], "indeterminate_incompatible_size_bins");
        assert_eq!(report["size_bin_changes"]["status"], "incompatible_bin_partitions");
    }

    #[test]
    fn refuses_to_subtract_measurements_with_different_units() {
        let mut run = run_fixture();
        run["quality_samples"][1]["phase_fractions"][0]["mass_fraction"]["unit"] = json!("g/kg");
        let report = compare_quality_samples(&run, "A", "B").unwrap();
        assert_eq!(report["overall_status"], "indeterminate_incompatible_units");
        assert_eq!(
            report["phase_fraction_changes"][0]["mass_fraction_change"]["status"],
            "incompatible_units"
        );
    }

    #[test]
    fn rejects_duplicate_phase_ids() {
        let mut run = run_fixture();
        run["quality_samples"][0]["phase_fractions"][1]["phase_id"] = json!("plagioclase");
        assert!(compare_quality_samples(&run, "A", "B").is_err());
    }
}
