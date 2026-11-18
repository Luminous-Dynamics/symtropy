// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use symtropy_material_flow_benchmark::{
    ScenarioManifest,
    procedure::{ProcedureEvidenceBundle, ProcedureGraph},
};

fn read(path: &str) -> Result<String, ExitCode> {
    fs::read_to_string(path).map_err(|error| {
        eprintln!("cannot read {path}: {error}");
        ExitCode::from(2)
    })
}

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(scenario_path) = args.next() else {
        eprintln!(
            "usage: {} <scenario.json> <procedure.json> <evidence-bundle.json>",
            program.to_string_lossy()
        );
        return ExitCode::from(2);
    };
    let Some(procedure_path) = args.next() else {
        eprintln!("expected procedure and evidence-bundle paths");
        return ExitCode::from(2);
    };
    let Some(evidence_path) = args.next() else {
        eprintln!("expected evidence-bundle path");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly three input paths");
        return ExitCode::from(2);
    }

    let scenario_path = scenario_path.to_string_lossy();
    let procedure_path = procedure_path.to_string_lossy();
    let evidence_path = evidence_path.to_string_lossy();
    let scenario_raw = match read(scenario_path.as_ref()) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let procedure_raw = match read(procedure_path.as_ref()) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let evidence_raw = match read(evidence_path.as_ref()) {
        Ok(value) => value,
        Err(code) => return code,
    };

    let scenario: ScenarioManifest = match serde_json::from_str(&scenario_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid scenario JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let procedure: ProcedureGraph = match serde_json::from_str(&procedure_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid procedure JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let evidence: ProcedureEvidenceBundle = match serde_json::from_str(&evidence_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid evidence bundle JSON: {error}");
            return ExitCode::from(1);
        }
    };

    match procedure.admit_evidence(&scenario, &evidence) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("cannot encode evidence-admission report: {error}");
                ExitCode::from(1)
            }
        },
        Err(errors) => {
            eprintln!("procedure evidence bundle rejected:");
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::from(1)
        }
    }
}
