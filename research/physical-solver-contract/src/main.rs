// SPDX-License-Identifier: AGPL-3.0-or-later
use physical_solver_contract::{
    equilibrium::assess_equilibrium_bytes, mesh_convergence::assess_mesh_refinement_study_bytes,
    reference_compare::compare_reference_observables, validate_request_bytes,
    validate_solver_receipt_bytes, verify_solver_output_artifact_bytes,
};
use std::{env, fs, path::Path, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("physical-solver-contract: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut request_path: Option<String> = None;
    let mut receipt_path: Option<String> = None;
    let mut mesh_study_path: Option<String> = None;
    let mut equilibrium_manifest_path: Option<String> = None;
    let mut reaction_force_output_id: Option<String> = None;
    let mut reaction_moment_output_id: Option<String> = None;
    let mut artifact_root: Option<String> = None;
    let mut reference_path: Option<String> = None;
    let mut observation_output_id: Option<String> = None;
    let mut validator_revision: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--request" => {
                if request_path.is_some() {
                    return Err("--request may be supplied only once".to_owned());
                }
                request_path = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--request requires a path".to_owned())?
                        .clone(),
                );
                i += 2;
            }
            "--mesh-study" => {
                if mesh_study_path.is_some() {
                    return Err("--mesh-study may be supplied only once".to_owned());
                }
                mesh_study_path = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--mesh-study requires a manifest path".to_owned())?
                        .clone(),
                );
                i += 2;
            }
            "--equilibrium-manifest" => {
                if equilibrium_manifest_path.is_some() {
                    return Err("--equilibrium-manifest may be supplied only once".to_owned());
                }
                equilibrium_manifest_path = Some(
                    args.get(i + 1)
                        .ok_or_else(|| {
                            "--equilibrium-manifest requires a manifest path".to_owned()
                        })?
                        .clone(),
                );
                i += 2;
            }
            "--reaction-force-output-id" => {
                if reaction_force_output_id.is_some() {
                    return Err("--reaction-force-output-id may be supplied only once".to_owned());
                }
                reaction_force_output_id = Some(
                    args.get(i + 1)
                        .ok_or_else(|| {
                            "--reaction-force-output-id requires an output ID".to_owned()
                        })?
                        .clone(),
                );
                i += 2;
            }
            "--reaction-moment-output-id" => {
                if reaction_moment_output_id.is_some() {
                    return Err("--reaction-moment-output-id may be supplied only once".to_owned());
                }
                reaction_moment_output_id = Some(
                    args.get(i + 1)
                        .ok_or_else(|| {
                            "--reaction-moment-output-id requires an output ID".to_owned()
                        })?
                        .clone(),
                );
                i += 2;
            }
            "--receipt" => {
                if receipt_path.is_some() {
                    return Err("--receipt may be supplied only once".to_owned());
                }
                receipt_path = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--receipt requires a path".to_owned())?
                        .clone(),
                );
                i += 2;
            }
            "--artifact-root" => {
                if artifact_root.is_some() {
                    return Err("--artifact-root may be supplied only once".to_owned());
                }
                artifact_root = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--artifact-root requires a directory".to_owned())?
                        .clone(),
                );
                i += 2;
            }
            "--reference" => {
                if reference_path.is_some() {
                    return Err("--reference may be supplied only once".to_owned());
                }
                reference_path = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--reference requires a path".to_owned())?
                        .clone(),
                );
                i += 2;
            }
            "--observation-output-id" => {
                if observation_output_id.is_some() {
                    return Err("--observation-output-id may be supplied only once".to_owned());
                }
                observation_output_id = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--observation-output-id requires an ID".to_owned())?
                        .clone(),
                );
                i += 2;
            }
            "--validator-revision" => {
                if validator_revision.is_some() {
                    return Err("--validator-revision may be supplied only once".to_owned());
                }
                validator_revision = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--validator-revision requires a value".to_owned())?
                        .clone(),
                );
                i += 2;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    if let Some(study_path) = mesh_study_path.as_deref() {
        if request_path.is_some()
            || receipt_path.is_some()
            || equilibrium_manifest_path.is_some()
            || reaction_force_output_id.is_some()
            || reaction_moment_output_id.is_some()
            || reference_path.is_some()
            || observation_output_id.is_some()
            || validator_revision.is_some()
        {
            return Err(
                "--mesh-study cannot be combined with request, receipt, reference, output-ID, or validator-revision options"
                    .to_owned(),
            );
        }
        let root =
            artifact_root.ok_or_else(|| "--mesh-study requires --artifact-root".to_owned())?;
        let study_bytes = fs::read(study_path)
            .map_err(|error| format!("could not read mesh-study manifest {study_path}: {error}"))?;
        let report = assess_mesh_refinement_study_bytes(&study_bytes, Path::new(&root))?;
        println!(
            "{}",
            serde_json::to_string(&report)
                .map_err(|error| format!("report serialization failed: {error}"))?
        );
        return Ok(());
    }

    if let Some(manifest_path) = equilibrium_manifest_path {
        if mesh_study_path.is_some()
            || reference_path.is_some()
            || observation_output_id.is_some()
            || validator_revision.is_some()
        {
            return Err("--equilibrium-manifest cannot be combined with mesh-study, reference-comparison, or request-validator options".to_owned());
        }
        let request_path =
            request_path.ok_or_else(|| "--equilibrium-manifest requires --request".to_owned())?;
        let receipt_path =
            receipt_path.ok_or_else(|| "--equilibrium-manifest requires --receipt".to_owned())?;
        let root = artifact_root
            .ok_or_else(|| "--equilibrium-manifest requires --artifact-root".to_owned())?;
        let force_id = reaction_force_output_id.ok_or_else(|| {
            "--equilibrium-manifest requires --reaction-force-output-id".to_owned()
        })?;
        let moment_id = reaction_moment_output_id.ok_or_else(|| {
            "--equilibrium-manifest requires --reaction-moment-output-id".to_owned()
        })?;
        let request_bytes = fs::read(&request_path)
            .map_err(|error| format!("could not read {request_path}: {error}"))?;
        let receipt_bytes = fs::read(&receipt_path)
            .map_err(|error| format!("could not read {receipt_path}: {error}"))?;
        let manifest_bytes = fs::read(&manifest_path)
            .map_err(|error| format!("could not read {manifest_path}: {error}"))?;
        let report = assess_equilibrium_bytes(
            &request_bytes,
            &receipt_bytes,
            Path::new(&root),
            &manifest_bytes,
            &force_id,
            &moment_id,
        )?;
        println!(
            "{}",
            serde_json::to_string(&report)
                .map_err(|error| format!("report serialization failed: {error}"))?
        );
        return Ok(());
    }
    if reaction_force_output_id.is_some() || reaction_moment_output_id.is_some() {
        return Err("--reaction-force-output-id and --reaction-moment-output-id require --equilibrium-manifest".to_owned());
    }

    let request_path = request_path.ok_or_else(|| {
        "usage: physical-solver-contract --request <path> --validator-revision <git-sha> OR --request <path> --receipt <receipt-path> [--artifact-root <directory>] [--reference <manifest.json> --observation-output-id <output-id>] OR --mesh-study <manifest.json> --artifact-root <study-root> OR --request <path> --receipt <path> --artifact-root <root> --equilibrium-manifest <manifest.json> --reaction-force-output-id <id> --reaction-moment-output-id <id>".to_owned()
    })?;
    let request_bytes = fs::read(&request_path)
        .map_err(|error| format!("could not read {request_path}: {error}"))?;

    let report = if let Some(receipt_path) = receipt_path {
        if validator_revision.is_some() {
            return Err("--validator-revision is only valid for request validation".to_owned());
        }
        let receipt_bytes = fs::read(&receipt_path)
            .map_err(|error| format!("could not read {receipt_path}: {error}"))?;
        match (reference_path, observation_output_id, artifact_root) {
            (Some(reference_path), Some(output_id), Some(root)) => {
                let reference_bytes = fs::read(&reference_path)
                    .map_err(|error| format!("could not read {reference_path}: {error}"))?;
                compare_reference_observables(
                    &request_bytes,
                    &receipt_bytes,
                    Path::new(&root),
                    &output_id,
                    &reference_bytes,
                )?
            }
            (Some(_), _, _) => {
                return Err(
                    "--reference requires --observation-output-id and --artifact-root".to_owned(),
                );
            }
            (None, Some(_), _) => {
                return Err("--observation-output-id requires --reference".to_owned());
            }
            (None, None, Some(root)) => verify_solver_output_artifact_bytes(
                &request_bytes,
                &receipt_bytes,
                Path::new(&root),
            )?,
            (None, None, None) => validate_solver_receipt_bytes(&request_bytes, &receipt_bytes)?,
        }
    } else {
        if artifact_root.is_some() || reference_path.is_some() || observation_output_id.is_some() {
            return Err(
                "--artifact-root, --reference and --observation-output-id require --receipt"
                    .to_owned(),
            );
        }
        let revision = validator_revision.ok_or_else(|| {
            "request validation requires --validator-revision <git-sha>".to_owned()
        })?;
        validate_request_bytes(&request_bytes, &revision)?
    };

    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|error| format!("report serialization failed: {error}"))?
    );
    Ok(())
}
