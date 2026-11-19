use plasma_dataset_admission::{SourceManifest, audit_manifest};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: plasma-dataset-admission <manifest.json>");
        return ExitCode::from(2);
    };

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("could not read manifest: {error}");
            return ExitCode::from(2);
        }
    };
    let manifest: SourceManifest = match serde_json::from_slice(&bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("invalid manifest: {error}");
            return ExitCode::from(2);
        }
    };
    let report = match audit_manifest(&manifest) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("audit failed: {error}");
            return ExitCode::from(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("could not serialize audit report: {error}");
            ExitCode::from(2)
        }
    }
}
