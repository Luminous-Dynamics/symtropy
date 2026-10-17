use spatial_resilience_qualifier::{
    validate_manifest_bytes, EMBEDDED_MANIFEST, EMBEDDED_SCHEMA,
};
use std::env;
use std::fs;
use std::process;

fn usage() -> &'static str {
    "Usage: spatial-resilience-qualifier --check [--json] | --validate MANIFEST.json SCHEMA.json [--json]"
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

fn emit(summary: &spatial_resilience_qualifier::ValidationSummary, json_output: bool) {
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

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (manifest_bytes, schema_bytes, json_output) = match args.as_slice() {
        [mode] if mode == "--check" => (
            EMBEDDED_MANIFEST.as_bytes().to_vec(),
            EMBEDDED_SCHEMA.as_bytes().to_vec(),
            false,
        ),
        [mode, flag] if mode == "--check" && flag == "--json" => (
            EMBEDDED_MANIFEST.as_bytes().to_vec(),
            EMBEDDED_SCHEMA.as_bytes().to_vec(),
            true,
        ),
        [mode, manifest_path, schema_path] if mode == "--validate" => (
            read_file(manifest_path),
            read_file(schema_path),
            false,
        ),
        [mode, manifest_path, schema_path, flag]
            if mode == "--validate" && flag == "--json" =>
        {
            (
                read_file(manifest_path),
                read_file(schema_path),
                true,
            )
        }
        _ => {
            eprintln!("{}", usage());
            process::exit(2);
        }
    };

    let summary = validate_manifest_bytes(&manifest_bytes, &schema_bytes);
    emit(&summary, json_output);
    if !summary.passed {
        process::exit(1);
    }
}
