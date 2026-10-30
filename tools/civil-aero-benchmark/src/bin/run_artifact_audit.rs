use std::{collections::BTreeMap, env, fs, path::Path, process};

use civil_aero_benchmark::run_artifacts::{
    ARTIFACT_MANIFEST_SCHEMA, CfdArtifactManifest, CfdArtifactRecord, CfdArtifactRole,
    verify_artifact_manifest,
};

const MANIFEST_HEADER: &str = "run_id,artifact_role,relative_path,byte_length,sha256";
const OUTPUT_HEADER: &str = "run_id,canonical_manifest_sha256,artifact_count,artifact_role,relative_path,byte_length,sha256";

fn main() {
    if let Err(error) = run() {
        eprintln!("CFD artifact audit rejected input: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        return Err("usage: run_artifact_audit <artifact-root> <manifest.csv>".to_owned());
    }
    let artifact_root = Path::new(&arguments[0]);
    let manifest_path = &arguments[1];
    let manifest_text = fs::read_to_string(manifest_path)
        .map_err(|error| format!("cannot read manifest {manifest_path}: {error}"))?;
    let manifests = parse_manifest_csv(&manifest_text)?;

    let mut reports = Vec::with_capacity(manifests.len());
    for manifest in &manifests {
        reports.push(verify_artifact_manifest(artifact_root, manifest)?);
    }

    eprintln!(
        "cfd_artifact_audit=byte_length_and_sha256_verified runs={} roles_per_run={} manifest_signature=absent source_authentication=not_established experimental_validation=not_claimed",
        reports.len(),
        CfdArtifactRole::REQUIRED.len()
    );
    println!("{OUTPUT_HEADER}");
    for report in &reports {
        for artifact in &report.artifacts {
            println!(
                "{},{},{},{},{},{},{}",
                report.run_id,
                report.canonical_manifest_sha256,
                report.artifacts.len(),
                artifact.role.as_str(),
                artifact.relative_path,
                artifact.byte_length,
                artifact.sha256
            );
        }
    }
    Ok(())
}

fn parse_manifest_csv(input: &str) -> Result<Vec<CfdArtifactManifest>, String> {
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| "manifest CSV is empty".to_owned())?
        .trim_end_matches('\r');
    if header != MANIFEST_HEADER {
        return Err(format!(
            "unexpected manifest CSV header; expected exactly: {MANIFEST_HEADER}"
        ));
    }

    let mut by_run: BTreeMap<String, Vec<CfdArtifactRecord>> = BTreeMap::new();
    let mut row_count = 0;
    for (index, raw_line) in lines.enumerate() {
        let line_number = index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let fields = line.split(',').map(str::trim).collect::<Vec<_>>();
        if fields.len() != 5 || fields.iter().any(|field| field.is_empty()) {
            return Err(format!(
                "line {line_number}: manifest row must contain five non-empty comma-free fields"
            ));
        }
        let run_id = fields[0].to_owned();
        if run_id.contains('|') || run_id.contains('\n') || run_id.contains('\r') {
            return Err(format!("line {line_number}: run_id contains a forbidden delimiter"));
        }
        let role = CfdArtifactRole::parse_role(fields[1])
            .ok_or_else(|| format!("line {line_number}: unknown artifact role {}", fields[1]))?;
        let byte_length = fields[3]
            .parse::<u64>()
            .map_err(|error| format!("line {line_number}: invalid byte_length: {error}"))?;
        let record = CfdArtifactRecord {
            role,
            relative_path: fields[2].to_owned(),
            byte_length,
            sha256: fields[4].to_owned(),
        };
        by_run.entry(run_id).or_default().push(record);
        row_count += 1;
    }
    if row_count == 0 {
        return Err("manifest CSV contains no artifact rows".to_owned());
    }

    Ok(by_run
        .into_iter()
        .map(|(run_id, artifacts)| CfdArtifactManifest {
            schema: ARTIFACT_MANIFEST_SCHEMA.to_owned(),
            run_id,
            artifacts,
        })
        .collect())
}
