#[derive(Debug, Clone, Copy, PartialEq)]
struct ReconciliationInterval {
    lower: f64,
    upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ClosureResult {
    status: &'static str,
    residual: Option<ReconciliationInterval>,
    reason: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct GeometryMeasurement {
    volume_m3: f64,
    expanded_uncertainty_m3: f64,
    coverage_fraction: f64,
    calibrated: bool,
    calibration_current: bool,
}

#[derive(Debug, Clone)]
struct MassMeasurement {
    batch_id: String,
    mass_kg: f64,
    expanded_uncertainty_kg: f64,
    calibrated: bool,
    calibration_current: bool,
}

#[derive(Debug, Clone)]
struct MassStream {
    stream_id: String,
    measurement: Option<MassMeasurement>,
}

fn validate_fraction(value: f64, name: &str) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{name} must be finite and in [0, 1]"));
    }
    Ok(())
}

fn classify_residual(residual: ReconciliationInterval, tolerance: f64) -> &'static str {
    if residual.lower >= -tolerance && residual.upper <= tolerance {
        "within_declared_tolerance"
    } else if residual.lower > tolerance || residual.upper < -tolerance {
        "outside_declared_tolerance"
    } else {
        "indeterminate_interval_overlaps_tolerance_boundary"
    }
}

fn reconcile_volume(
    planned_volume_m3: f64,
    measured: Option<GeometryMeasurement>,
    tolerance_fraction: f64,
    minimum_coverage_fraction: f64,
) -> Result<ClosureResult, String> {
    if !planned_volume_m3.is_finite() || planned_volume_m3 <= 0.0 {
        return Err("planned volume must be finite and positive".to_owned());
    }
    validate_fraction(tolerance_fraction, "volume tolerance fraction")?;
    validate_fraction(minimum_coverage_fraction, "minimum coverage fraction")?;

    let Some(measured) = measured else {
        return Ok(ClosureResult {
            status: "unknown_missing_as_built_measurement",
            residual: None,
            reason: "no measured as-built geometry was supplied",
        });
    };

    if !measured.volume_m3.is_finite()
        || measured.volume_m3 <= 0.0
        || !measured.expanded_uncertainty_m3.is_finite()
        || measured.expanded_uncertainty_m3 < 0.0
    {
        return Err("measured volume and uncertainty must be finite and valid".to_owned());
    }
    validate_fraction(measured.coverage_fraction, "measurement coverage fraction")?;

    if !measured.calibrated {
        return Ok(ClosureResult {
            status: "unknown_measurement_not_calibrated",
            residual: None,
            reason: "measurement calibration is absent or invalid",
        });
    }
    if !measured.calibration_current {
        return Ok(ClosureResult {
            status: "unknown_stale_calibration",
            residual: None,
            reason: "measurement calibration is stale",
        });
    }
    if measured.coverage_fraction < minimum_coverage_fraction {
        return Ok(ClosureResult {
            status: "unknown_insufficient_scan_coverage",
            residual: None,
            reason: "measured scan coverage is below the declared minimum",
        });
    }

    let actual = ReconciliationInterval {
        lower: (measured.volume_m3 - measured.expanded_uncertainty_m3).max(0.0),
        upper: measured.volume_m3 + measured.expanded_uncertainty_m3,
    };
    if !actual.upper.is_finite() {
        return Err("measured volume interval overflows".to_owned());
    }
    let residual = ReconciliationInterval {
        lower: actual.lower - planned_volume_m3,
        upper: actual.upper - planned_volume_m3,
    };
    let tolerance = planned_volume_m3 * tolerance_fraction;
    let status = classify_residual(residual, tolerance);

    Ok(ClosureResult {
        status,
        residual: Some(residual),
        reason: "residual interval compared with caller-declared tolerance",
    })
}

fn validate_mass_measurement(measurement: &MassMeasurement) -> Result<(), String> {
    if measurement.batch_id.trim().is_empty() {
        return Err("mass measurement batch id must not be empty".to_owned());
    }
    if !measurement.mass_kg.is_finite()
        || measurement.mass_kg < 0.0
        || !measurement.expanded_uncertainty_kg.is_finite()
        || measurement.expanded_uncertainty_kg < 0.0
        || !(measurement.mass_kg + measurement.expanded_uncertainty_kg).is_finite()
    {
        return Err("mass and uncertainty must be finite and non-negative".to_owned());
    }
    Ok(())
}

fn mass_interval(measurement: &MassMeasurement) -> ReconciliationInterval {
    ReconciliationInterval {
        lower: (measurement.mass_kg - measurement.expanded_uncertainty_kg).max(0.0),
        upper: measurement.mass_kg + measurement.expanded_uncertainty_kg,
    }
}

fn reconcile_mass_flow(
    input: Option<MassMeasurement>,
    outputs: &[MassStream],
    tolerance_fraction: f64,
) -> Result<ClosureResult, String> {
    validate_fraction(tolerance_fraction, "mass balance tolerance fraction")?;

    let Some(input) = input else {
        return Ok(ClosureResult {
            status: "unknown_missing_input_mass",
            residual: None,
            reason: "no calibrated input mass measurement was supplied",
        });
    };
    validate_mass_measurement(&input)?;
    if input.mass_kg <= 0.0 {
        return Err("input mass must be finite and positive".to_owned());
    }

    if !input.calibrated {
        return Ok(ClosureResult {
            status: "unknown_input_not_calibrated",
            residual: None,
            reason: "input mass calibration is absent or invalid",
        });
    }
    if !input.calibration_current {
        return Ok(ClosureResult {
            status: "unknown_stale_input_calibration",
            residual: None,
            reason: "input mass calibration is stale",
        });
    }
    if outputs.is_empty() {
        return Ok(ClosureResult {
            status: "unknown_no_disposition_streams",
            residual: None,
            reason: "no measured output or disposition streams were supplied",
        });
    }

    let mut seen_ids = std::collections::BTreeSet::new();
    let mut output_lower_kg = 0.0;
    let mut output_upper_kg = 0.0;

    for stream in outputs {
        if stream.stream_id.trim().is_empty() || !seen_ids.insert(stream.stream_id.as_str()) {
            return Err("mass stream ids must be non-empty and unique".to_owned());
        }
        let Some(measurement) = &stream.measurement else {
            return Ok(ClosureResult {
                status: "unknown_missing_disposition_measurement",
                residual: None,
                reason: "at least one material disposition stream has no mass measurement",
            });
        };
        validate_mass_measurement(measurement)?;
        if measurement.batch_id != input.batch_id {
            return Ok(ClosureResult {
                status: "unknown_material_batch_mismatch",
                residual: None,
                reason: "input and output mass records do not share the same batch id",
            });
        }
        if !measurement.calibrated {
            return Ok(ClosureResult {
                status: "unknown_output_not_calibrated",
                residual: None,
                reason: "at least one disposition stream is not calibrated",
            });
        }
        if !measurement.calibration_current {
            return Ok(ClosureResult {
                status: "unknown_stale_output_calibration",
                residual: None,
                reason: "at least one disposition stream has stale calibration",
            });
        }

        let interval = mass_interval(measurement);
        output_lower_kg += interval.lower;
        output_upper_kg += interval.upper;
    }
    if !output_lower_kg.is_finite() || !output_upper_kg.is_finite() {
        return Err("summed material disposition mass overflows".to_owned());
    }

    let input_interval = mass_interval(&input);
    let residual = ReconciliationInterval {
        lower: input_interval.lower - output_upper_kg,
        upper: input_interval.upper - output_lower_kg,
    };
    let tolerance = input.mass_kg * tolerance_fraction;
    let status = classify_residual(residual, tolerance);

    Ok(ClosureResult {
        status,
        residual: Some(residual),
        reason: "input minus disposition interval compared with caller-declared tolerance",
    })
}

pub(super) fn render_report_json() -> Result<String, String> {
    let within = reconcile_volume(
        1_000_000.0,
        Some(GeometryMeasurement {
            volume_m3: 1_010_000.0,
            expanded_uncertainty_m3: 5_000.0,
            coverage_fraction: 0.995,
            calibrated: true,
            calibration_current: true,
        }),
        0.02,
        0.95,
    )?;
    let outside = reconcile_volume(
        1_000_000.0,
        Some(GeometryMeasurement {
            volume_m3: 1_035_000.0,
            expanded_uncertainty_m3: 2_000.0,
            coverage_fraction: 0.995,
            calibrated: true,
            calibration_current: true,
        }),
        0.02,
        0.95,
    )?;
    let indeterminate = reconcile_volume(
        1_000_000.0,
        Some(GeometryMeasurement {
            volume_m3: 1_020_000.0,
            expanded_uncertainty_m3: 5_000.0,
            coverage_fraction: 0.995,
            calibrated: true,
            calibration_current: true,
        }),
        0.02,
        0.95,
    )?;
    let stale = reconcile_volume(
        1_000_000.0,
        Some(GeometryMeasurement {
            volume_m3: 1_000_000.0,
            expanded_uncertainty_m3: 1_000.0,
            coverage_fraction: 1.0,
            calibrated: true,
            calibration_current: false,
        }),
        0.02,
        0.95,
    )?;

    let balanced = reconcile_mass_flow(
        Some(MassMeasurement {
            batch_id: "SYNTHETIC-BATCH-001".to_owned(),
            mass_kg: 1_000_000.0,
            expanded_uncertainty_kg: 1_000.0,
            calibrated: true,
            calibration_current: true,
        }),
        &[
            MassStream {
                stream_id: "reuse".to_owned(),
                measurement: Some(MassMeasurement {
                    batch_id: "SYNTHETIC-BATCH-001".to_owned(),
                    mass_kg: 200_000.0,
                    expanded_uncertainty_kg: 500.0,
                    calibrated: true,
                    calibration_current: true,
                }),
            },
            MassStream {
                stream_id: "stockpile".to_owned(),
                measurement: Some(MassMeasurement {
                    batch_id: "SYNTHETIC-BATCH-001".to_owned(),
                    mass_kg: 500_000.0,
                    expanded_uncertainty_kg: 500.0,
                    calibrated: true,
                    calibration_current: true,
                }),
            },
            MassStream {
                stream_id: "process_feed".to_owned(),
                measurement: Some(MassMeasurement {
                    batch_id: "SYNTHETIC-BATCH-001".to_owned(),
                    mass_kg: 300_000.0,
                    expanded_uncertainty_kg: 500.0,
                    calibrated: true,
                    calibration_current: true,
                }),
            },
        ],
        0.01,
    )?;

    let format_closure = |result: ClosureResult| -> String {
        match result.residual {
            Some(interval) => format!(
                "{{\"status\":\"{}\",\"residual_lower\":{:.3},\"residual_upper\":{:.3},\"reason\":\"{}\"}}",
                result.status, interval.lower, interval.upper, result.reason
            ),
            None => format!(
                "{{\"status\":\"{}\",\"residual_lower\":null,\"residual_upper\":null,\"reason\":\"{}\"}}",
                result.status, result.reason
            ),
        }
    };

    Ok(format!(
        concat!(
            "{{\n",
            "  \"evidence_class\": \"synthetic_reconciliation_reference_examples\",\n",
            "  \"assumptions\": [\n",
            "    \"All examples are synthetic fixtures, not measured lunar or terrestrial construction data.\",\n",
            "    \"Tolerance fractions and minimum scan coverage are caller-declared example inputs, not lunar standards.\",\n",
            "    \"Residual intervals propagate the supplied expanded measurement bounds.\",\n",
            "    \"A result is unknown when a measurement is missing, uncalibrated, stale, incomplete, or belongs to another material batch.\"\n",
            "  ],\n",
            "  \"volume_tolerance_fraction\": 0.02,\n",
            "  \"minimum_scan_coverage_fraction\": 0.95,\n",
            "  \"volume_examples\": {{\n",
            "    \"within\": {},\n",
            "    \"outside\": {},\n",
            "    \"indeterminate\": {},\n",
            "    \"stale_calibration\": {}\n",
            "  }},\n",
            "  \"mass_flow_example\": {},\n",
            "  \"mass_flow_tolerance_fraction\": 0.01,\n",
            "  \"claim_ceiling\": \"synthetic data-contract and interval-arithmetic examples only; no physical construction acceptance\"\n",
            "}}"
        ),
        format_closure(within),
        format_closure(outside),
        format_closure(indeterminate),
        format_closure(stale),
        format_closure(balanced),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measurement(batch_id: &str, mass_kg: f64, uncertainty_kg: f64) -> MassMeasurement {
        MassMeasurement {
            batch_id: batch_id.to_owned(),
            mass_kg,
            expanded_uncertainty_kg: uncertainty_kg,
            calibrated: true,
            calibration_current: true,
        }
    }

    #[test]
    fn volume_reconciliation_distinguishes_pass_fail_and_indeterminate() {
        let within = reconcile_volume(
            1_000_000.0,
            Some(GeometryMeasurement {
                volume_m3: 1_010_000.0,
                expanded_uncertainty_m3: 5_000.0,
                coverage_fraction: 0.995,
                calibrated: true,
                calibration_current: true,
            }),
            0.02,
            0.95,
        )
        .expect("valid measurement");
        assert_eq!(within.status, "within_declared_tolerance");

        let outside = reconcile_volume(
            1_000_000.0,
            Some(GeometryMeasurement {
                volume_m3: 1_035_000.0,
                expanded_uncertainty_m3: 2_000.0,
                coverage_fraction: 0.995,
                calibrated: true,
                calibration_current: true,
            }),
            0.02,
            0.95,
        )
        .expect("valid measurement");
        assert_eq!(outside.status, "outside_declared_tolerance");

        let indeterminate = reconcile_volume(
            1_000_000.0,
            Some(GeometryMeasurement {
                volume_m3: 1_020_000.0,
                expanded_uncertainty_m3: 5_000.0,
                coverage_fraction: 0.995,
                calibrated: true,
                calibration_current: true,
            }),
            0.02,
            0.95,
        )
        .expect("valid measurement");
        assert_eq!(
            indeterminate.status,
            "indeterminate_interval_overlaps_tolerance_boundary"
        );
    }

    #[test]
    fn stale_or_missing_geometry_measurements_fail_closed_to_unknown() {
        let missing = reconcile_volume(100.0, None, 0.02, 0.95).expect("valid plan");
        assert_eq!(missing.status, "unknown_missing_as_built_measurement");

        let stale = reconcile_volume(
            100.0,
            Some(GeometryMeasurement {
                volume_m3: 100.0,
                expanded_uncertainty_m3: 1.0,
                coverage_fraction: 1.0,
                calibrated: true,
                calibration_current: false,
            }),
            0.02,
            0.95,
        )
        .expect("valid measurement");
        assert_eq!(stale.status, "unknown_stale_calibration");
    }

    #[test]
    fn mass_flow_closure_propagates_uncertainty() {
        let result = reconcile_mass_flow(
            Some(measurement("batch-7", 1_000_000.0, 1_000.0)),
            &[
                MassStream {
                    stream_id: "reuse".to_owned(),
                    measurement: Some(measurement("batch-7", 200_000.0, 500.0)),
                },
                MassStream {
                    stream_id: "stockpile".to_owned(),
                    measurement: Some(measurement("batch-7", 500_000.0, 500.0)),
                },
                MassStream {
                    stream_id: "process-feed".to_owned(),
                    measurement: Some(measurement("batch-7", 300_000.0, 500.0)),
                },
            ],
            0.01,
        )
        .expect("valid mass flow");
        assert_eq!(result.status, "within_declared_tolerance");
        let residual = result.residual.expect("measured residual");
        assert!(residual.lower <= 0.0);
        assert!(residual.upper >= 0.0);
    }

    #[test]
    fn missing_and_mismatched_mass_streams_remain_unknown() {
        let missing = reconcile_mass_flow(
            Some(measurement("batch-7", 10.0, 1.0)),
            &[MassStream {
                stream_id: "reuse".to_owned(),
                measurement: None,
            }],
            0.01,
        )
        .expect("missing stream is an unknown state");
        assert_eq!(missing.status, "unknown_missing_disposition_measurement");

        let mismatch = reconcile_mass_flow(
            Some(measurement("batch-7", 10.0, 1.0)),
            &[MassStream {
                stream_id: "stockpile".to_owned(),
                measurement: Some(measurement("batch-8", 10.0, 1.0)),
            }],
            0.01,
        )
        .expect("batch mismatch is an unknown state");
        assert_eq!(mismatch.status, "unknown_material_batch_mismatch");
    }

    #[test]
    fn rejects_invalid_volume_tolerance_and_duplicate_stream_ids() {
        assert!(reconcile_volume(-1.0, None, 0.02, 0.95).is_err());
        assert!(reconcile_volume(1.0, None, 1.1, 0.95).is_err());
        assert!(reconcile_volume(1.0, None, 0.02, f64::NAN).is_err());

        let input = Some(measurement("batch-7", 2.0, 0.1));
        let duplicate = [
            MassStream {
                stream_id: "same".to_owned(),
                measurement: Some(measurement("batch-7", 1.0, 0.1)),
            },
            MassStream {
                stream_id: "same".to_owned(),
                measurement: Some(measurement("batch-7", 1.0, 0.1)),
            },
        ];
        assert!(reconcile_mass_flow(input, &duplicate, 0.01).is_err());
    }

    #[test]
    fn synthetic_report_is_deterministic_and_discloses_claim_ceiling() {
        let first = render_report_json().expect("valid report");
        let second = render_report_json().expect("valid report");
        assert_eq!(first, second);
        assert!(first.contains("synthetic_reconciliation_reference_examples"));
        assert!(first.contains("outside_declared_tolerance"));
        assert!(first.contains("unknown_stale_calibration"));
        assert!(first.contains("no physical construction acceptance"));
    }
}
