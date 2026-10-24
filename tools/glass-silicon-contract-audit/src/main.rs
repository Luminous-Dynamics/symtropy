use glass_silicon_contract_audit::{audit_adaptive_decision_campaign, audit_corpus, audit_decision_value_pool, audit_evaluation_receipt, audit_false_feasible_bound, audit_feasibility_corpus, audit_grouped_split_manifest, audit_relationship_graph, audit_mass_balance, hash_artifact_files};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn read_json(path: &str) -> Result<Value, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("invalid JSON in {path}: {error}"))
}

fn usage() {
    eprintln!(
        "Usage:\n  glass-silicon-contract-audit audit-corpus <schema.json> <corpus.json>\n  glass-silicon-contract-audit audit-feasibility <corpus.json>\n  glass-silicon-contract-audit audit-grouped-split <manifest.json>\n  glass-silicon-contract-audit audit-relationship-graph <graph.json>\n  glass-silicon-contract-audit audit-false-feasible-bound <summary.json>\n  glass-silicon-contract-audit audit-eval-receipt <receipt.json>\n  glass-silicon-contract-audit audit-decision-value <pool.json>\n  glass-silicon-contract-audit audit-adaptive-campaign <pool.json>\n  glass-silicon-contract-audit check-mass <ledger.json>\n  glass-silicon-contract-audit hash-artifacts <corpus.json> <artifact-id> <directory>"
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("audit-corpus") if args.len() == 4 => {
            let schema = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "benchmark_corpus",
                        "record_id": null,
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no physical or manufacturing claim is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let corpus = match read_json(&args[3]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "benchmark_corpus",
                        "record_id": null,
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no physical or manufacturing claim is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = audit_corpus(&schema, &corpus);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("audit-eval-receipt") if args.len() == 3 => {
            let receipt = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "evaluation_receipt_contract",
                        "receipt_id": null,
                        "claimed_evaluation_class": null,
                        "verified_evaluation_class": "UNSEALED_RETROSPECTIVE",
                        "result_disposition": null,
                        "metrics_checked": 0,
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; external custody, isolation and hidden-label separation are not verified."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = audit_evaluation_receipt(&receipt);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("audit-feasibility") if args.len() == 3 => {
            let corpus = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "synthetic_glass_feasibility_gate_corpus",
                        "benchmark_id": null,
                        "candidates": [],
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no physical, model, or manufacturing claim is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = audit_feasibility_corpus(&corpus);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("hash-artifacts") if args.len() == 5 => {
            let corpus = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "source_artifact_hash_receipt",
                        "artifact_id": args[3],
                        "files": [],
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no reuse rights or benchmark admission is established."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = hash_artifact_files(&corpus, &args[3], Path::new(&args[4]));
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("audit-false-feasible-bound") if args.len() == 3 => {
            let summary = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "false_feasible_rate_bound",
                        "benchmark_id": null,
                        "evidence_class": null,
                        "independent_negative_units": null,
                        "false_feasible_count": null,
                        "observed_false_feasible_rate": null,
                        "confidence_level": null,
                        "target_upper_bound": null,
                        "hoeffding_upper_bound": null,
                        "target_met": false,
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no model performance or false-feasible risk is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = audit_false_feasible_bound(&summary);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("audit-grouped-split") if args.len() == 3 => {
            // Read once so the reported SHA-256 fingerprints exactly the bytes parsed.
            let raw = match fs::read(&args[2]) {
                Ok(bytes) => bytes,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "grouped_split_manifest",
                        "benchmark_id": null,
                        "evidence_class": null,
                        "manifest_sha256": null,
                        "records_checked": 0,
                        "grouping_fields": [],
                        "split_counts": {},
                        "findings": [format!("INPUT_BYTES:{error}")],
                        "claim_ceiling": "Input parsing only; no leakage-free training pipeline or model performance is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let manifest: Value = match serde_json::from_slice(&raw) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "grouped_split_manifest",
                        "benchmark_id": null,
                        "evidence_class": null,
                        "manifest_sha256": null,
                        "records_checked": 0,
                        "grouping_fields": [],
                        "split_counts": {},
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no leakage-free training pipeline or model performance is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let manifest_sha256 = format!("{:x}", Sha256::digest(&raw));
            let mut report = audit_grouped_split_manifest(&manifest);
            report.manifest_sha256 = Some(manifest_sha256);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("audit-relationship-graph") if args.len() == 3 => {
            // Fingerprint exactly the raw bytes that are parsed, separately from the
            // auditor's canonical interpretation of the relationship graph.
            let raw = match fs::read(&args[2]) {
                Ok(bytes) => bytes,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "relationship_graph_split_isolation",
                        "schema_version": null,
                        "benchmark_id": null,
                        "evidence_class": null,
                        "relation_profile": null,
                        "graph_sha256": null,
                        "canonical_graph_sha256": null,
                        "nodes_checked": 0,
                        "edges_checked": 0,
                        "assignments_checked": 0,
                        "split_counts": {},
                        "witness_paths": [],
                        "findings": [format!("INPUT_BYTES:{error}")],
                        "claim_ceiling": "Input parsing only; no source truth, legal rights, model quality, or physical effect is certified."
                    }));
                    return ExitCode::from(2);
                }
            };
            let graph: Value = match serde_json::from_slice(&raw) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "relationship_graph_split_isolation",
                        "schema_version": null,
                        "benchmark_id": null,
                        "evidence_class": null,
                        "relation_profile": null,
                        "graph_sha256": null,
                        "canonical_graph_sha256": null,
                        "nodes_checked": 0,
                        "edges_checked": 0,
                        "assignments_checked": 0,
                        "split_counts": {},
                        "witness_paths": [],
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no source truth, legal rights, model quality, or physical effect is certified."
                    }));
                    return ExitCode::from(2);
                }
            };
            let graph_sha256 = format!("{:x}", Sha256::digest(&raw));
            let mut report = audit_relationship_graph(&graph);
            report.graph_sha256 = Some(graph_sha256);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("audit-decision-value") if args.len() == 3 => {
            let pool = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "synthetic_decision_value_pool",
                        "benchmark_id": null,
                        "pre_measurement_worst_case_regret": null,
                        "measurements": [],
                        "pareto_frontier": [],
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no physical, model, or manufacturing claim is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = audit_decision_value_pool(&pool);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("audit-adaptive-campaign") if args.len() == 3 => {
            let pool = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "synthetic_adaptive_decision_campaign",
                        "benchmark_id": null,
                        "pre_measurement_worst_case_regret": null,
                        "adaptive_oracle_worst_case_regret": null,
                        "policy_tree": null,
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no physical, model, or manufacturing claim is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = audit_adaptive_decision_campaign(&pool);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("check-mass") if args.len() == 3 => {
            let ledger = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "elemental_mass_balance",
                        "record_id": null,
                        "mass_unit": null,
                        "residuals": {},
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no physical or manufacturing claim is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = audit_mass_balance(&ledger);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        _ => {
            usage();
            ExitCode::from(64)
        }
    }
}
