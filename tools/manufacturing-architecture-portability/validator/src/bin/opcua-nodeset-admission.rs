// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Read-only local NodeSet admission and OPC UA dependency-bundle analysis.

use mfg_architecture_profile_validator::nodeset_admission::{
    analyze_node_set_bundle, evaluate_server_namespace_metadata, verify_nodeset_artifact,
};
use serde_json::{Value, json};
use std::{env, fs, path::PathBuf, process::ExitCode};

fn read_json(path: &PathBuf) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))
}

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  opcua-nodeset-admission verify <standard-reference.json> <NodeSet2.xml>\n  opcua-nodeset-admission bundle <reference-a.json> <reference-b.json>...\n  opcua-nodeset-admission server <standard-reference.json> <snapshot.schema.json> <namespace-snapshot.json> [--require-full]"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(command) = args.next() else {
        return usage();
    };
    let command = command.to_string_lossy();
    let rest: Vec<PathBuf> = args.map(PathBuf::from).collect();

    match command.as_ref() {
        "verify" if rest.len() == 2 => {
            let manifest = match read_json(&rest[0]) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            };
            let xml = match fs::read(&rest[1]) {
                Ok(bytes) => bytes,
                Err(error) => {
                    eprintln!("{}: {error}", rest[1].display());
                    return ExitCode::FAILURE;
                }
            };
            match verify_nodeset_artifact(&xml, &manifest) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "admitted": report.admitted,
                            "model_uri": report.model_uri,
                            "model_version": report.model_version,
                            "model_publication_date": report.model_publication_date,
                            "artifact_sha256": report.artifact_sha256,
                            "namespace_uri_count": report.namespace_uri_count,
                            "required_model_count": report.required_model_count,
                            "checked_node_id_references": report.checked_node_id_references,
                            "claim_ceiling": "pinned artifact bytes and declared NodeSet metadata only; no live server import or endpoint conformance"
                        }))
                        .expect("report serializes")
                    );
                    ExitCode::SUCCESS
                }
                Err(errors) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "admitted": false,
                            "errors": errors
                        }))
                        .expect("error report serializes")
                    );
                    ExitCode::FAILURE
                }
            }
        }
        "bundle" if rest.len() >= 2 => {
            let mut manifests = Vec::with_capacity(rest.len());
            for path in &rest {
                match read_json(path) {
                    Ok(value) => manifests.push(value),
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            let report = analyze_node_set_bundle(&manifests);
            let dependency_skews: Vec<Value> = report
                .dependency_skews
                .iter()
                .map(|skew| {
                    json!({
                        "model_uri": skew.model_uri,
                        "requirements": skew.requirements.iter().map(|(version, date)| {
                            json!({"version":version,"publication_date":date})
                        }).collect::<Vec<_>>()
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ready_for_composition": report.ready_for_composition,
                    "status": report.status,
                    "dependency_skews": dependency_skews,
                    "claim_ceiling": "RequiredModel version consistency only; no compatibility or upgrade policy is inferred"
                }))
                .expect("bundle report serializes")
            );
            // A discovered skew is a successful analysis result, not a parser error.
            ExitCode::SUCCESS
        }
        "server" if rest.len() == 3 || rest.len() == 4 => {
            let manifest = match read_json(&rest[0]) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            };
            let schema = match read_json(&rest[1]) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            };
            let observed = match read_json(&rest[2]) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            };
            let require_full =
                rest.len() == 4 && rest[3].to_string_lossy().as_ref() == "--require-full";
            if rest.len() == 4 && !require_full {
                return usage();
            }
            if let Err(error) = jsonschema::draft202012::meta::validate(&schema) {
                eprintln!("invalid NamespaceSnapshot schema: {error}");
                return ExitCode::FAILURE;
            }
            let validator = match jsonschema::draft202012::options()
                .should_validate_formats(true)
                .build(&schema)
            {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("cannot build NamespaceSnapshot validator: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let schema_errors: Vec<String> = validator
                .iter_errors(&observed)
                .map(|error| format!("{}: {error}", error.instance_path()))
                .collect();
            if !schema_errors.is_empty() {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "schema_valid":false,
                        "schema_errors":schema_errors,
                        "metadata_matches":false
                    }))
                    .expect("schema report serializes")
                );
                return ExitCode::FAILURE;
            }
            match evaluate_server_namespace_metadata(&manifest, &observed) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "metadata_matches": report.metadata_matches,
                            "is_namespace_subset": report.subset,
                            "disposition": report.disposition,
                            "mismatches": report.mismatches,
                            "claim_ceiling": "NamespaceMetadata comparison only; even IsNamespaceSubset=false does not prove NodeId coverage or semantic conformance"
                        }))
                        .expect("server report serializes")
                    );
                    if !report.metadata_matches || (require_full && report.subset) {
                        ExitCode::FAILURE
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => usage(),
    }
}
