use plasma_dataset_admission::{SourceManifest, audit_manifest};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Component, Path};
use std::process::ExitCode;

fn safe_file_name(value: &str) -> bool {
    if value.trim().is_empty() || value.contains('/') || value.contains('\\\\') || value.contains('\\0') {
        return false;
    }
    let mut components = Path::new(value).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

fn load_local_artifacts(
    manifest: &SourceManifest,
    directory: &Path,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let root = fs::canonicalize(directory)
        .map_err(|error| format!("could not resolve artifact directory: {error}"))?;
    if !root.is_dir() {
        return Err("artifact directory is not a directory".to_owned());
    }

    let mut output = BTreeMap::new();
    for artifact in &manifest.artifacts {
        if !safe_file_name(&artifact.file_name) {
            continue;
        }
        let candidate = root.join(&artifact.file_name);
        let metadata = match fs::symlink_metadata(&candidate) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("could not inspect {}: {error}", artifact.file_name)),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!(
                "refusing non-regular or symlink artifact: {}",
                artifact.file_name
            ));
        }

        let resolved = fs::canonicalize(&candidate)
            .map_err(|error| format!("could not resolve {}: {error}", artifact.file_name))?;
        if !resolved.starts_with(&root) {
            return Err(format!(
                "artifact path escapes the supplied directory: {}",
                artifact.file_name
            ));
        }

        let bytes = fs::read(&resolved)
            .map_err(|error| format!("could not read {}: {error}", artifact.file_name))?;
        output.insert(artifact.artifact_id.clone(), bytes);
    }
    Ok(output)
}

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

    let local_artifacts = match artifact_directory {
        Some(directory) => match load_local_artifacts(&manifest, Path::new(&directory)) {
            Ok(files) => files,
            Err(error) => {
                eprintln!("could not load local artifacts: {error}");
                return ExitCode::from(2);
            }
        },
        None => BTreeMap::new(),
    };

    let report = match audit_manifest(&manifest, &local_artifacts) {
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
