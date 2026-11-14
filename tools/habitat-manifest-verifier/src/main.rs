// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use habitat_manifest_verifier::{verify_manifest_bytes, ManifestKind};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args_os();
    let _program = args.next();
    let Some(kind_arg) = args.next() else {
        eprintln!("usage: habitat-manifest-verifier <whole-property|terrain-native> <manifest.json>");
        return ExitCode::from(2);
    };
    let Some(path) = args.next() else {
        eprintln!("error: expected a manifest path");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("error: expected exactly a corpus kind and one manifest path");
        return ExitCode::from(2);
    }

    let kind = match kind_arg.to_str().ok_or_else(|| "corpus kind is not UTF-8".to_owned())
        .and_then(ManifestKind::parse) {
        Ok(kind) => kind,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: could not read {:?}: {error}", path);
            return ExitCode::from(2);
        }
    };

    let report = match verify_manifest_bytes(kind, &bytes) {
        Ok(report) => report,
        Err(errors) => {
            for error in errors {
                eprintln!("manifest rejected: {error}");
            }
            return ExitCode::from(3);
        }
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: could not serialize verifier report: {error}");
            ExitCode::from(4)
        }
    }
}
