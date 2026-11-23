// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Score matched RRVR benchmark episodes from immutable, artifact-linked records.
//!
//! This scorer computes statistics only. It does not execute policies,
//! authenticate evidence producers, or independently adjudicate ground truth.

use mobility_rrvr_runtime_trace::benchmark::BenchmarkCorpus;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::process::ExitCode;

fn digest_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 && args.len() != 3 && args.len() != 5 {
        return Err(concat!(
            "usage: rrvr_benchmark_score <benchmark-corpus.json> [report.json] ",
            "[--bootstrap-replicates N]"
        )
        .to_owned());
    }
    let bootstrap_replicates = if args.len() == 5 {
        if args[3] != "--bootstrap-replicates" {
            return Err("unknown benchmark scorer option".to_owned());
        }
        Some(
            args[4]
                .parse::<u32>()
                .map_err(|error| format!("invalid bootstrap replicate count: {error}"))?,
        )
    } else {
        None
    };

    let corpus_bytes =
        fs::read(&args[1]).map_err(|error| format!("read RRVR benchmark corpus: {error}"))?;
    let source_corpus_sha256 = digest_hex(&corpus_bytes);
    let corpus: BenchmarkCorpus = serde_json::from_slice(&corpus_bytes)
        .map_err(|error| format!("strict benchmark corpus parse failed: {error}"))?;
    let report = corpus
        .validate_and_score(&source_corpus_sha256)
        .map_err(|error| format!("benchmark corpus rejected: {error:?}"))?;

    let output = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("serialize RRVR benchmark report: {error}"))?;
    if args.len() >= 3 {
        fs::write(&args[2], &output)
            .map_err(|error| format!("write RRVR benchmark report: {error}"))?;
    }

    let bootstrap_report = bootstrap_replicates
        .map(|replicates| {
            corpus
                .cluster_bootstrap(&source_corpus_sha256, replicates)
                .map_err(|error| format!("cluster bootstrap rejected: {error:?}"))
        })
        .transpose()?;
    let bootstrap_output = bootstrap_report
        .as_ref()
        .map(serde_json::to_vec_pretty)
        .transpose()
        .map_err(|error| format!("serialize cluster bootstrap report: {error}"))?;
    if let (Some(path), Some(bytes)) = (args.get(2), bootstrap_output.as_ref()) {
        fs::write(format!("{path}.bootstrap.json"), bytes)
            .map_err(|error| format!("write cluster bootstrap report: {error}"))?;
    }
    let summary = json!({
        "validation": "passed",
        "benchmark_id": report.benchmark_id,
        "suite_revision": report.suite_revision,
        "source_corpus_sha256": source_corpus_sha256,
        "matched_pair_count": report.matched_pair_count,
        "scenario_cluster_count": report.scenario_cluster_count,
        "arm_count": report.arm_summaries.len(),
        "paired_comparison_count": report.paired_comparisons.len(),
        "report_sha256": digest_hex(&output),
        "scoring_method": report.scoring_method,
        "claim_note": report.claim_note,
        "cluster_bootstrap_requested": bootstrap_report.is_some(),
        "cluster_bootstrap_report_sha256": bootstrap_output.as_ref().map(|bytes| digest_hex(bytes)),
        "cluster_bootstrap_seed_sha256": bootstrap_report.as_ref().map(|value| value.bootstrap_seed_sha256.clone()),
        "cluster_bootstrap_replicates": bootstrap_report.as_ref().map(|value| value.replicate_count),
    });
    if args.len() >= 3 {
        let summary_path = format!("{}.summary.json", args[2]);
        let summary_bytes = serde_json::to_vec_pretty(&summary)
            .map_err(|error| format!("serialize benchmark summary: {error}"))?;
        fs::write(&summary_path, summary_bytes)
            .map_err(|error| format!("write benchmark summary: {error}"))?;
    }

    let rendered = String::from_utf8(output)
        .map_err(|error| format!("benchmark report is not UTF-8: {error}"))?;
    println!("{rendered}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
