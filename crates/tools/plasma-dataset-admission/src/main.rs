use plasma_dataset_admission::{
    SourceManifest, audit_manifest, audit_manifest_from_directory,
};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(manifest_path) = args.next() else {
        eprintln!("usage: plasma-dataset-admission <manifest.json> [artifact-directory]");
        return ExitCode::from(2);
    };
    let artifact_directory = args.next();
    if args.next().is_some() {
        eprintln!("usage: plasma-dataset-admission <manifest.json> [artifact-directory]");
        return ExitCode::from(2);
    }

    let manifest_bytes = match fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("could not read manifest: {error}");
            return ExitCode::from(2);
        }
    };
    let manifest: SourceManifest = match serde_json::from_slice(&manifest_bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("invalid manifest: {error}");
            return ExitCode::from(2);
        }
    };

    let report = match artifact_directory {
        Some(directory) => audit_manifest_from_directory(&manifest, Path::new(&directory)),
        None => audit_manifest(&manifest, &BTreeMap::new()),
    };
    let report = match report {
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
