// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use robot_config_change_reference_evaluator::{
    PACKAGE_DIR, read_json, repository_root, validate_package,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_options(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut options = BTreeMap::new();
    let mut index = 0;
    while index < args.len() {
        let key = &args[index];
        if !key.starts_with("--") {
            return Err(format!("unexpected_argument:{key}"));
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing_value_for:{key}"))?;
        if value.starts_with("--") {
            return Err(format!("missing_value_for:{key}"));
        }
        if options.insert(key.clone(), value.clone()).is_some() {
            return Err(format!("duplicate_option:{key}"));
        }
        index += 2;
    }
    Ok(options)
}

fn take(options: &mut BTreeMap<String, String>, name: &str) -> Option<String> {
    options.remove(name)
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        None | Some("validate") => validate_command(&args[1..]),
        Some("evaluate") => evaluate_command(&args[1..]),
        Some("help" | "--help" | "-h") => {
            print_usage();
            Ok(())
        }
        Some(command) => Err(format!("unknown_command:{command}; use --help")),
    }
}

fn paths(
    args: &[String],
) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf, PathBuf, Option<PathBuf>), String> {
    let mut options = parse_options(args)?;
    let base = repository_root().join(PACKAGE_DIR);
    let corpus = take(&mut options, "--corpus")
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("robot-config-change-corpus-v0.1.json"));
    let oracle = take(&mut options, "--oracle")
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("development-oracle-v0.1.json"));
    let corpus_schema = take(&mut options, "--corpus-schema")
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("schemas/robot-config-change-corpus-v0.1.schema.json"));
    let oracle_schema = take(&mut options, "--oracle-schema")
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("schemas/development-oracle-v0.1.schema.json"));
    let result_schema = take(&mut options, "--result-schema")
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("schemas/development-result-v0.1.schema.json"));
    let output = take(&mut options, "--output").map(PathBuf::from);
    if !options.is_empty() {
        return Err(format!(
            "unknown_options:{}",
            options.keys().cloned().collect::<Vec<_>>().join(",")
        ));
    }
    Ok((
        corpus,
        oracle,
        corpus_schema,
        oracle_schema,
        result_schema,
        output,
    ))
}

fn load_inputs(
    args: &[String],
) -> Result<(Value, Value, Value, Value, Value, Option<PathBuf>), String> {
    let (
        corpus_path,
        oracle_path,
        corpus_schema_path,
        oracle_schema_path,
        result_schema_path,
        output,
    ) = paths(args)?;
    let corpus = read_json(&corpus_path)?;
    let oracle = read_json(&oracle_path)?;
    let corpus_schema = read_json(&corpus_schema_path)?;
    let oracle_schema = read_json(&oracle_schema_path)?;
    let result_schema = read_json(&result_schema_path)?;
    Ok((
        corpus,
        oracle,
        corpus_schema,
        oracle_schema,
        result_schema,
        output,
    ))
}

fn validate_command(args: &[String]) -> Result<(), String> {
    let (corpus, oracle, corpus_schema, oracle_schema, result_schema, output) = load_inputs(args)?;
    let result = validate_package(
        &corpus,
        &oracle,
        &corpus_schema,
        &oracle_schema,
        &result_schema,
    )
    .map_err(|errors| errors.join("\n"))?;
    if result["oracle_comparison"]["exact_disposition_and_reason_agreement"] != true {
        return Err(format!(
            "public_oracle_mismatch:{}",
            result["oracle_comparison"]["mismatches"]
        ));
    }
    if output.is_some() {
        return Err("--output is available only with the evaluate command".to_owned());
    }
    println!(
        "PASS: {} cases / {} claim results exactly match the public development oracle; critical changed-dependency preservation count={}",
        result["summary"]["case_count"],
        result["summary"]["claim_result_count"],
        result["summary"]["safety_critical_changed_dependency_preserved_count"]
    );
    Ok(())
}

fn evaluate_command(args: &[String]) -> Result<(), String> {
    let (corpus, oracle, corpus_schema, oracle_schema, result_schema, output) = load_inputs(args)?;
    let output_path = output.ok_or_else(|| "required_option_missing:--output".to_owned())?;
    let result = validate_package(
        &corpus,
        &oracle,
        &corpus_schema,
        &oracle_schema,
        &result_schema,
    )
    .map_err(|errors| errors.join("\n"))?;
    if result["oracle_comparison"]["exact_disposition_and_reason_agreement"] != true {
        return Err(format!(
            "public_oracle_mismatch:{}",
            result["oracle_comparison"]["mismatches"]
        ));
    }
    let mut bytes =
        serde_json::to_vec_pretty(&result).map_err(|error| format!("result_serialize:{error}"))?;
    bytes.push(b'\n');
    if let Some(parent) = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("output_directory_create:{}:{error}", parent.display()))?;
    }
    fs::write(&output_path, bytes)
        .map_err(|error| format!("result_write:{}:{error}", output_path.display()))?;
    println!(
        "PASS: wrote {} cases / {} exact oracle-matched claim results; corpus={} oracle={} output={}",
        result["summary"]["case_count"],
        result["summary"]["claim_result_count"],
        result["input_digests"]["corpus_canonical_json_sha256"],
        result["input_digests"]["oracle_canonical_json_sha256"],
        output_path.display()
    );
    Ok(())
}

fn print_usage() {
    println!(
        "ROBOT-CONFIG-CHANGE-QUAL-001 public synthetic reference evaluator

Commands:
  validate [--corpus PATH] [--oracle PATH]
           [--corpus-schema PATH] [--oracle-schema PATH] [--result-schema PATH]
      Validate Draft 2020-12 schemas, corpus/oracle references and exact
      disposition + reason-code agreement for all public development cases.

  evaluate --output PATH [same optional input/schema arguments]
      Validate and write a digest-bound development result JSON document.

Default inputs are the checked-in public RCQ v0.1 corpus, development oracle
and schemas. This evaluator implements only explicit rules for those fixtures.
It is not a production claim engine, physical qualifier, safety controller,
blind benchmark, certification tool or work-authorization path."
    );
}
