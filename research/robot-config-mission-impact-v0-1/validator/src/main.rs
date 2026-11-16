// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use robot_mission_impact_package_check::{generator, read_json, repository_root, schema_is_valid, validate_instance, validate_package};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        None | Some("validate") => validate(),
        Some("generate") => generate(&args[1..]),
        Some("help" | "--help" | "-h") => {
            print_usage();
            Ok(())
        }
        Some(command) => Err(format!("unknown_command:{command}; use --help")),
    }
}

fn validate() -> Result<(), String> {
    validate_package(&repository_root()).map_err(|errors| errors.join("\n"))?;
    println!("PASS: schemas, public development fixtures and cross-file invariants are valid");
    Ok(())
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

fn take_option(
    options: &mut BTreeMap<String, String>,
    name: &str,
) -> Option<String> {
    options.remove(name)
}

fn parse_seed(value: &str) -> Result<u64, String> {
    let digits = value.strip_prefix("0x").unwrap_or(value);
    if digits.is_empty() || digits.len() > 16 {
        return Err("seed_must_be_one_to_sixteen_hex_digits".to_owned());
    }
    u64::from_str_radix(digits, 16).map_err(|_| "seed_not_valid_hex_u64".to_owned())
}

fn generate(args: &[String]) -> Result<(), String> {
    let mut options = parse_options(args)?;
    let seed = take_option(&mut options, "--seed")
        .ok_or_else(|| "required_option_missing:--seed".to_owned())
        .and_then(|value| parse_seed(&value))?;
    let count = take_option(&mut options, "--count")
        .ok_or_else(|| "required_option_missing:--count".to_owned())
        .and_then(|value| {
            value
                .parse::<usize>()
                .map_err(|_| "count_not_a_positive_integer".to_owned())
        })?;
    let output = take_option(&mut options, "--output")
        .ok_or_else(|| "required_option_missing:--output".to_owned())
        .map(PathBuf::from)?;

    let root = repository_root();
    let default_base = root.join("research/robot-config-mission-impact-v0-1");
    let profile_path = take_option(&mut options, "--profile")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_base.join("challenge-distribution-v0.1.json"));
    let schema_path = take_option(&mut options, "--profile-schema")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            default_base.join("schemas/challenge-distribution-v0.1.schema.json")
        });
    if !options.is_empty() {
        return Err(format!(
            "unknown_options:{}",
            options.keys().cloned().collect::<Vec<_>>().join(",")
        ));
    }

    let profile: Value = read_json(&profile_path).map_err(|error| error.to_string())?;
    let schema: Value = read_json(&schema_path).map_err(|error| error.to_string())?;
    if !schema_is_valid(&schema) {
        return Err("distribution_profile_schema_meta_invalid".to_owned());
    }
    let schema_errors = validate_instance(&schema, &profile);
    if !schema_errors.is_empty() {
        return Err(format!(
            "distribution_profile_schema_invalid:{}",
            schema_errors.join(",")
        ));
    }
    let profile_errors = generator::distribution_profile_invariants(&profile);
    if !profile_errors.is_empty() {
        return Err(format!(
            "distribution_profile_invariants_invalid:{}",
            profile_errors.join(",")
        ));
    }

    let manifest = generator::generate_manifest(&profile, seed, count)
        .map_err(|errors| errors.join("\n"))?;
    let manifest_schema_path =
        default_base.join("schemas/challenge-manifest-v0.1.schema.json");
    let manifest_schema: Value =
        read_json(&manifest_schema_path).map_err(|error| error.to_string())?;
    if !schema_is_valid(&manifest_schema) {
        return Err("challenge_manifest_schema_meta_invalid".to_owned());
    }
    let manifest_errors = validate_instance(&manifest_schema, &manifest);
    if !manifest_errors.is_empty() {
        return Err(format!(
            "generated_manifest_schema_invalid:{}",
            manifest_errors.join(",")
        ));
    }

    let mut bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|error| format!("manifest_serialize:{error}"))?;
    bytes.push(b'\n');
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("output_parent_create:{}:{error}", parent.display()))?;
    }
    fs::write(&output, bytes)
        .map_err(|error| format!("manifest_write:{}:{error}", output.display()))?;

    println!(
        "PASS: generated {} deterministic public synthetic challenges; seed={} digest={} output={}",
        count,
        generator::seed_hex(seed),
        manifest["manifest_digest"].as_str().unwrap_or("missing"),
        output.display()
    );
    Ok(())
}

fn print_usage() {
    println!(
        "Robot mission-impact package checker / stochastic challenge generator

Commands:
  validate
      Validate all checked-in schemas, public fixtures and cross-file invariants.

  generate --seed HEX --count N --output PATH [--profile PATH] [--profile-schema PATH]
      Generate a deterministic public synthetic challenge manifest.
      HEX is one to sixteen hexadecimal digits, optionally prefixed by 0x.
      N must be in 1..=10000. The default profile/schema are the checked-in
      v0.1 public development profile and its closed JSON Schema.

The development PRNG is reproducible, not cryptographically secure. This
command does not create evaluator-held seeds, simulate physical scenes,
qualify a robot, or authorize physical trials."
    );
}
