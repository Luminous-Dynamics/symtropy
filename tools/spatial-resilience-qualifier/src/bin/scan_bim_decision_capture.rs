// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Run the Scan-to-BIM decision-sufficient capture adapter.

use spatial_resilience_qualifier::{
    EMBEDDED_SCAN_BIM_DECISION_CASE, EMBEDDED_SCAN_BIM_DECISION_SCHEMA,
    evaluate_scan_bim_decision_bytes,
};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;

fn usage() -> &'static str {
    "Usage: scan_bim_decision_capture --check [--json] | --validate CASE.json SCHEMA.json [--json]"
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mode = args.next().ok_or_else(|| usage().to_owned())?;
    let (case_bytes, schema_bytes, json_only) = match mode.as_str() {
        "--check" => {
            let json_only = match args.next().as_deref() {
                None => false,
                Some("--json") => true,
                Some(_) => return Err(usage().to_owned()),
            };
            if args.next().is_some() {
                return Err(usage().to_owned());
            }
            (
                EMBEDDED_SCAN_BIM_DECISION_CASE.as_bytes().to_vec(),
                EMBEDDED_SCAN_BIM_DECISION_SCHEMA.as_bytes().to_vec(),
                json_only,
            )
        }
        "--validate" => {
            let case_path = args.next().ok_or_else(|| usage().to_owned())?;
            let schema_path = args.next().ok_or_else(|| usage().to_owned())?;
            let json_only = match args.next().as_deref() {
                None => false,
                Some("--json") => true,
                Some(_) => return Err(usage().to_owned()),
            };
            if args.next().is_some() {
                return Err(usage().to_owned());
            }
            let case_bytes = fs::read(&case_path)
                .map_err(|error| format!("cannot read case {case_path:?}: {error}"))?;
            let schema_bytes = fs::read(&schema_path)
                .map_err(|error| format!("cannot read schema {schema_path:?}: {error}"))?;
            (case_bytes, schema_bytes, json_only)
        }
        _ => return Err(usage().to_owned()),
    };

    let report = evaluate_scan_bim_decision_bytes(&case_bytes, &schema_bytes)
        .map_err(|failures| {
            failures
                .into_iter()
                .map(|failure| {
                    format!("{} path={} detail={}", failure.code, failure.path, failure.detail)
                })
                .collect::<Vec<_>>()
                .join("\n")
        })?;
    let serialized = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("cannot serialize report: {error}"))?;
    if json_only {
        let stdout = io::stdout();
        let mut writer = stdout.lock();
        writer
            .write_all(&serialized)
            .and_then(|_| writer.write_all(b"\n"))
            .map_err(|error| format!("cannot write report: {error}"))?;
    } else {
        println!(
            "PASS: Scan-to-BIM decision capture case={} actions={} evidence_qualified={} decision_accepted={} decision_relevant={} decision_rejected={}",
            report.case_id,
            report.action_count,
            report.evidence_qualified_action_count,
            report.decision_accepted_action_count,
            report.decision_relevant_action_count,
            report.decision_rejected_action_count
        );
        println!(
            "model_revision={} requirement_revision={}",
            report.model_revision, report.requirement_revision
        );
        println!(
            "recommended_action_order={:?}",
            report.decision.recommended_action_order
        );
        for action in &report.action_qualifications {
            println!(
                "action={} evidence={} targets={}/{} rejection_codes={:?}",
                action.action_id,
                action.evidence_state,
                action.qualified_target_count,
                action.target_count,
                action.rejection_codes
            );
        }
        println!("input_sha256={}", report.input_sha256);
        println!("schema_sha256={}", report.schema_sha256);
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("scan_bim_decision_capture: {error}");
            ExitCode::FAILURE
        }
    }
}
