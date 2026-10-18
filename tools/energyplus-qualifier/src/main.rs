use energyplus_qualifier::{
    validate_fixture_corpus_bytes, validate_receipt_bytes, validate_spec_bytes,
    EMBEDDED_FIXTURE_CORPUS, EMBEDDED_FIXTURE_SCHEMA, EMBEDDED_RECEIPT_SCHEMA,
    EMBEDDED_SPEC, EMBEDDED_SPEC_SCHEMA,
};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn usage() -> &'static str {
    concat!(
        "Usage: energyplus-qualifier --check-spec [--json] | ",
        "--check-fixtures [--json] | ",
        "--validate-receipt RECEIPT.json EVIDENCE_ROOT [--json]"
    )
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

fn emit_spec(summary: &energyplus_qualifier::SpecValidationSummary, json_output: bool) {
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
    println!("{state}: EnergyPlus A0 adapter envelope");
    println!(
        "benchmark_id={} version={}",
        summary.benchmark_id.as_deref().unwrap_or("unknown"),
        summary.version.as_deref().unwrap_or("unknown")
    );
    println!("spec_sha256={}", summary.spec_sha256);
    println!("schema_sha256={}", summary.schema_sha256);
    for finding in &summary.findings {
        println!(
            "{} path={} detail={}",
            finding.code, finding.path, finding.detail
        );
    }
}

fn emit_fixtures(summary: &energyplus_qualifier::FixtureValidationSummary, json_output: bool) {
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
    println!("{state}: synthetic EnergyPlus receipt-gate fixtures");
    println!(
        "benchmark_id={} version={} fixtures={} fixtures_passed={}",
        summary.benchmark_id.as_deref().unwrap_or("unknown"),
        summary.version.as_deref().unwrap_or("unknown"),
        summary.fixtures_seen,
        summary.fixtures_passed
    );
    println!("corpus_sha256={}", summary.corpus_sha256);
    println!("fixture_schema_sha256={}", summary.fixture_schema_sha256);
    println!("receipt_schema_sha256={}", summary.receipt_schema_sha256);
    for result in &summary.results {
        println!(
            "fixture={} disposition={} passed={}",
            result.fixture_id, result.actual_disposition, result.passed
        );
        if !result.actual_blocker_codes.is_empty() {
            println!("  blockers={:?}", result.actual_blocker_codes);
        }
    }
    for finding in &summary.findings {
        println!(
            "{} path={} detail={}",
            finding.code, finding.path, finding.detail
        );
    }
}

fn emit_gate(result: &energyplus_qualifier::GateResult, json_output: bool) {
    if json_output {
        match serde_json::to_string_pretty(result) {
            Ok(serialized) => println!("{serialized}"),
            Err(error) => {
                eprintln!("REPORT_SERIALIZATION_FAILED: {error}");
                process::exit(2);
            }
        }
        return;
    }

    println!(
        "disposition={} receipt={} schema_valid={}",
        result.disposition, result.receipt_id, result.schema_valid
    );
    if !result.blocker_codes.is_empty() {
        println!("blockers={:?}", result.blocker_codes);
    }
    println!(
        "claim_ceiling=receipt integrity only; not physical validation or safety approval"
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [mode] if mode == "--check-spec" => {
            let summary = validate_spec_bytes(
                EMBEDDED_SPEC.as_bytes(),
                EMBEDDED_SPEC_SCHEMA.as_bytes(),
            );
            emit_spec(&summary, false);
            if !summary.passed {
                process::exit(1);
            }
        }
        [mode, flag] if mode == "--check-spec" && flag == "--json" => {
            let summary = validate_spec_bytes(
                EMBEDDED_SPEC.as_bytes(),
                EMBEDDED_SPEC_SCHEMA.as_bytes(),
            );
            emit_spec(&summary, true);
            if !summary.passed {
                process::exit(1);
            }
        }
        [mode] if mode == "--check-fixtures" => {
            let summary = validate_fixture_corpus_bytes(
                EMBEDDED_FIXTURE_CORPUS.as_bytes(),
                EMBEDDED_FIXTURE_SCHEMA.as_bytes(),
                EMBEDDED_RECEIPT_SCHEMA.as_bytes(),
                EMBEDDED_SPEC.as_bytes(),
                EMBEDDED_SPEC_SCHEMA.as_bytes(),
            );
            emit_fixtures(&summary, false);
            if !summary.passed {
                process::exit(1);
            }
        }
        [mode, flag] if mode == "--check-fixtures" && flag == "--json" => {
            let summary = validate_fixture_corpus_bytes(
                EMBEDDED_FIXTURE_CORPUS.as_bytes(),
                EMBEDDED_FIXTURE_SCHEMA.as_bytes(),
                EMBEDDED_RECEIPT_SCHEMA.as_bytes(),
                EMBEDDED_SPEC.as_bytes(),
                EMBEDDED_SPEC_SCHEMA.as_bytes(),
            );
            emit_fixtures(&summary, true);
            if !summary.passed {
                process::exit(1);
            }
        }
        [mode, receipt_path, evidence_root] if mode == "--validate-receipt" => {
            let spec_summary = validate_spec_bytes(
                EMBEDDED_SPEC.as_bytes(),
                EMBEDDED_SPEC_SCHEMA.as_bytes(),
            );
            emit_spec(&spec_summary, false);
            if !spec_summary.passed {
                process::exit(1);
            }
            let receipt_bytes = read_file(receipt_path);
            let result = validate_receipt_bytes(
                &receipt_bytes,
                EMBEDDED_SPEC.as_bytes(),
                EMBEDDED_RECEIPT_SCHEMA.as_bytes(),
                Path::new(evidence_root),
            );
            emit_gate(&result, false);
            if result.disposition != "eligible_for_reference_comparison" {
                process::exit(1);
            }
        }
        [mode, receipt_path, evidence_root, flag]
            if mode == "--validate-receipt" && flag == "--json" =>
        {
            let spec_summary = validate_spec_bytes(
                EMBEDDED_SPEC.as_bytes(),
                EMBEDDED_SPEC_SCHEMA.as_bytes(),
            );
            emit_spec(&spec_summary, false);
            if !spec_summary.passed {
                process::exit(1);
            }
            let receipt_bytes = read_file(receipt_path);
            let result = validate_receipt_bytes(
                &receipt_bytes,
                EMBEDDED_SPEC.as_bytes(),
                EMBEDDED_RECEIPT_SCHEMA.as_bytes(),
                Some(Path::new(evidence_root)),
            );
            emit_gate(&result, true);
            if result.disposition != "eligible_for_reference_comparison" {
                process::exit(1);
            }
        }
        _ => {
            eprintln!("{}", usage());
            process::exit(2);
        }
    }
}
