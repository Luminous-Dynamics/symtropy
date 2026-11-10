// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use maritime_context_bundle::verify;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

fn repo_root() -> Result<PathBuf, String> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .map_err(|error| format!("repository root unavailable: {error}"))
}

fn read_repo_file(root: &Path, relative: &str) -> Result<Vec<u8>, String> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("unsafe repository path: {relative}"));
    }
    let canonical = root
        .join(path)
        .canonicalize()
        .map_err(|error| format!("{relative}: {error}"))?;
    if !canonical.starts_with(root) {
        return Err(format!("repository path escapes root: {relative}"));
    }
    fs::read(canonical).map_err(|error| format!("{relative}: {error}"))
}

fn run() -> Result<(), String> {
    let root = repo_root()?;
    let mut args = env::args().skip(1);
    let schema_path = args.next().unwrap_or_else(|| {
        "docs/tech/MARITIME_OPERATIONAL_CONTEXT_BUNDLE_V0_1.schema.json".to_owned()
    });
    let corpus_path = args.next().unwrap_or_else(|| {
        "benchmarks/maritime/operational-context-bundle-corpus-v0.1.json".to_owned()
    });
    if args.next().is_some() {
        return Err("usage: maritime-context-bundle [schema-path] [corpus-path]".to_owned());
    }

    let schema = read_repo_file(&root, &schema_path)?;
    let corpus = read_repo_file(&root, &corpus_path)?;
    let result = verify(&schema, &corpus)?;

    if result.mismatches.is_empty() && result.matched == result.total {
        println!(
            "MOCB_CORPUS_PASS cases={} matched={} schema={} semantic={} sha256={}",
            result.total,
            result.matched,
            result.schema_cases,
            result.semantic_cases,
            result.corpus_sha256
        );
        Ok(())
    } else {
        eprintln!(
            "MOCB_CORPUS_FAIL cases={} matched={} schema={} semantic={} sha256={}",
            result.total,
            result.matched,
            result.schema_cases,
            result.semantic_cases,
            result.corpus_sha256
        );
        for failure in result.mismatches {
            eprintln!("{failure}");
        }
        Err("MOCB corpus did not match its frozen oracle".to_owned())
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("MOCB_ORACLE_ERROR {error}");
            ExitCode::FAILURE
        }
    }
}
