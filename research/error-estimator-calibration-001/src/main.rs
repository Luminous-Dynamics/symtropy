// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::env;
use std::fs;
use std::process::ExitCode;

use error_estimator_calibration_reference::{analyze_dataset, DatasetV1};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: calibrate-error-estimates <dataset.tsv> [calibration-quantile]");
        return ExitCode::FAILURE;
    };
    let quantile = match args.next() {
        Some(value) => match value.parse::<f64>() {
            Ok(parsed) => parsed,
            Err(_) => {
                eprintln!("invalid calibration quantile");
                return ExitCode::FAILURE;
            }
        },
        None => 0.99,
    };

    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("could not read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let dataset = match DatasetV1::parse_tsv(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("dataset rejected: {error}");
            return ExitCode::FAILURE;
        }
    };
    let reports = match analyze_dataset(&dataset, quantile) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("analysis inconclusive: {error}");
            return ExitCode::FAILURE;
        }
    };

    println!("simulator\tversion\tmethod\tcontact\testimator\tcalibration_n\tevaluation_n\tmultiplier\tcoverage\twilson95_lower\twilson95_upper\tfalse_accept_fraction\tfalse_reject_fraction\ttrue_accept\tfalse_accept\tfalse_reject\ttrue_reject\tobserved_error_p50\tobserved_error_p95\twall_ms_p50\twall_ms_p95\tmemory_p95_bytes");
    for report in reports {
        let wilson = report.evaluation_coverage_wilson_95;
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            report.simulator,
            report.simulator_version,
            report.method,
            report.contact_regime,
            report.estimator_kind,
            report.calibration_sample_count,
            report.evaluation_sample_count,
            fmt_option(report.calibration_multiplier),
            fmt_option(report.evaluation_coverage),
            wilson.map(|value| value.lower.to_string()).unwrap_or_else(|| "NA".to_owned()),
            wilson.map(|value| value.upper.to_string()).unwrap_or_else(|| "NA".to_owned()),
            fmt_option(report.evaluation_false_accept_fraction),
            fmt_option(report.evaluation_false_reject_fraction),
            report.confusion.true_accept,
            report.confusion.false_accept,
            report.confusion.false_reject,
            report.confusion.true_reject,
            report.evaluation_global_error_p50,
            report.evaluation_global_error_p95,
            report.evaluation_wall_time_p50_ms,
            report.evaluation_wall_time_p95_ms,
            report.evaluation_peak_memory_p95_bytes,
        );
    }

    ExitCode::SUCCESS
}

fn fmt_option(value: Option<f64>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "NA".to_owned())
}
