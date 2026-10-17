use spatial_resilience_qualifier::{
    validate_manifest_bytes, validate_timeline_fixtures_bytes, TimelineFixtureSummary,
    ValidationSummary, EMBEDDED_MANIFEST, EMBEDDED_ORACLE_CORPUS, EMBEDDED_ORACLE_SCHEMA,
    EMBEDDED_SCHEMA,
};
use std::env;
use std::fs;
use std::process;

fn usage() -> &'static str {
    "Usage: spatial-resilience-qualifier --check [--json] | --oracle-check [--json] | --validate MANIFEST.json SCHEMA.json [--json] | --oracle-validate CORPUS.json SCHEMA.json [--json]"
}

fn read_file(path: &str) -> Vec<u8> {
    match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("INPUT_READ_FAILED path={path:?}: {error}");
            process::exit(2);
        }
    }
}

fn emit_manifest(summary: &ValidationSummary, json_output: bool) {
    if json_output {
        match serde_json::to_string_pretty(summary) {
            Ok(serialized) => println!("{serialized}"),
            Err(error) => {
                eprintln!("REPORT_SERIALIZATION_FAILED: {error}");
                process::exit(2);
            }
        }
        return;
    }

    let state = if summary.passed { "PASS" } else { "FAIL" };
    println!("{state}: spatial-resilience benchmark manifest");
    println!(
        "benchmark_id={} version={} cases={} invariants={}",
        summary.benchmark_id.as_deref().unwrap_or("unknown"),
        summary.version.as_deref().unwrap_or("unknown"),
        summary.cases_seen,
        summary.invariant_registry_count
    );
    println!("manifest_sha256={}", summary.manifest_sha256);
    println!("schema_sha256={}", summary.schema_sha256);
    for failure in &summary.failures {
        println!(
            "{} path={} detail={}",
            failure.code, failure.path, failure.detail
        );
    }
}

fn emit_timeline(summary: &TimelineFixtureSummary, json_output: bool) {
    if json_output {
        match serde_json::to_string_pretty(summary) {
            Ok(serialized) => println!("{serialized}"),
            Err(error) => {
                eprintln!("REPORT_SERIALIZATION_FAILED: {error}");
                process::exit(2);
            }
        }
        return;
    }

    let state = if summary.passed { "PASS" } else { "FAIL" };
    println!("{state}: spatial-resilience timeline oracle");
    println!(
        "benchmark_id={} version={} cases={} cases_passed={}",
        summary.benchmark_id.as_deref().unwrap_or("unknown"),
        summary.version.as_deref().unwrap_or("unknown"),
        summary.cases_seen,
        summary.cases_passed
    );
    println!("corpus_sha256={}", summary.corpus_sha256);
    println!("schema_sha256={}", summary.schema_sha256);
    for failure in &summary.failures {
        println!(
            "{} path={} detail={}",
            failure.code, failure.path, failure.detail
        );
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [mode] if mode == "--check" => {
            let summary =
                validate_manifest_bytes(EMBEDDED_MANIFEST.as_bytes(), EMBEDDED_SCHEMA.as_bytes());
            emit_manifest(&summary, false);
            if !summary.passed {
                process::exit(1);
            }
        }
        [mode, flag] if mode == "--check" && flag == "--json" => {
            let summary =
                validate_manifest_bytes(EMBEDDED_MANIFEST.as_bytes(), EMBEDDED_SCHEMA.as_bytes());
            emit_manifest(&summary, true);
            if !summary.passed {
                process::exit(1);
            }
        }
        [mode] if mode == "--oracle-check" => {
            let summary = validate_timeline_fixtures_bytes(
                EMBEDDED_ORACLE_CORPUS.as_bytes(),
                EMBEDDED_ORACLE_SCHEMA.as_bytes(),
            );
            emit_timeline(&summary, false);
            if !summary.passed {
                process::exit(1);
            }
        }
        [mode, flag] if mode == "--oracle-check" && flag == "--json" => {
            let summary = validate_timeline_fixtures_bytes(
                EMBEDDED_ORACLE_CORPUS.as_bytes(),
                EMBEDDED_ORACLE_SCHEMA.as_bytes(),
            );
            emit_timeline(&summary, true);
            if !summary.passed {
                process::exit(1);
            }
        }
        [mode, manifest_path, schema_path] if mode == "--validate" => {
            let manifest = read_file(manifest_path);
            let schema = read_file(schema_path);
            let summary = validate_manifest_bytes(&manifest, &schema);
            emit_manifest(&summary, false);
            if !summary.passed {
                process::exit(1);
            }
        }
        [mode, manifest_path, schema_path, flag]
            if mode == "--validate" && flag == "--json" =>
        {
            let manifest = read_file(manifest_path);
            let schema = read_file(schema_path);
            let summary = validate_manifest_bytes(&manifest, &schema);
            emit_manifest(&summary, true);
            if !summary.passed {
                process::exit(1);
            }
        }
        [mode, corpus_path, schema_path] if mode == "--oracle-validate" => {
            let corpus = read_file(corpus_path);
            let schema = read_file(schema_path);
            let summary = validate_timeline_fixtures_bytes(&corpus, &schema);
            emit_timeline(&summary, false);
            if !summary.passed {
                process::exit(1);
            }
        }
        [mode, corpus_path, schema_path, flag]
            if mode == "--oracle-validate" && flag == "--json" =>
        {
            let corpus = read_file(corpus_path);
            let schema = read_file(schema_path);
            let summary = validate_timeline_fixtures_bytes(&corpus, &schema);
            emit_timeline(&summary, true);
            if !summary.passed {
                process::exit(1);
            }
        }
        _ => {
            eprintln!("{}", usage());
            process::exit(2);
        }
    }
}
