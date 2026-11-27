use ev_bms_diagnostic_audit::split_manifest::audit_split_manifest;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut schema_path = None;
    let mut manifest_path = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--schema" => schema_path = args.next(),
            "--manifest" => manifest_path = args.next(),
            _ => {
                eprintln!("unknown argument: {arg}");
                return ExitCode::FAILURE;
            }
        }
    }
    let (Some(schema_path), Some(manifest_path)) = (schema_path, manifest_path) else {
        eprintln!("usage: ev-bms-split-manifest-audit --schema PATH --manifest PATH");
        return ExitCode::FAILURE;
    };
    let schema = match fs::read(&schema_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read schema {schema_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let manifest = match fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read manifest {manifest_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    match audit_split_manifest(&schema, &manifest) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(output) => {
                println!("{output}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("cannot serialize split audit report: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("EV BMS split audit failed: {error}");
            ExitCode::FAILURE
        }
    }
}
