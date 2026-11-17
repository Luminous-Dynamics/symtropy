// SPDX-License-Identifier: AGPL-3.0-or-later

use mucoll_metrics_admission::audit_files;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, message)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

fn run() -> Result<(), (u8, String)> {
    let mut args = env::args().skip(1);
    let mut report: Option<PathBuf> = None;
    let mut policy: Option<PathBuf> = None;
    let mut input: Option<PathBuf> = None;
    let mut fragments_dir: Option<PathBuf> = None;
    let mut submission: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;

    while let Some(argument) = args.next() {
        let value = |args: &mut std::iter::Skip<std::env::Args>, flag: &str| {
            args.next().ok_or_else(|| (64, format!("{flag} requires a path")))
        };
        match argument.as_str() {
            "--report" => report = Some(PathBuf::from(value(&mut args, "--report")?)),
            "--policy" => policy = Some(PathBuf::from(value(&mut args, "--policy")?)),
            "--input" => input = Some(PathBuf::from(value(&mut args, "--input")?)),
            "--fragments-dir" => fragments_dir = Some(PathBuf::from(value(&mut args, "--fragments-dir")?)),
            "--submission" => submission = Some(PathBuf::from(value(&mut args, "--submission")?)),
            "--output" => output = Some(PathBuf::from(value(&mut args, "--output")?)),
            "--help" | "-h" => {
                println!("mucoll-metrics-admission --report metrics.json --policy POLICY.json --input reco.edm4hep.root --fragments-dir DIR --submission submission.json --output RECEIPT.json");
                return Ok(());
            }
            unknown => return Err((64, format!("unknown argument {unknown:?}; use --help"))),
        }
    }

    let report = report.ok_or_else(|| (64, "missing --report".to_owned()))?;
    let policy = policy.ok_or_else(|| (64, "missing --policy".to_owned()))?;
    let input = input.ok_or_else(|| (64, "missing --input".to_owned()))?;
    let fragments_dir = fragments_dir.ok_or_else(|| (64, "missing --fragments-dir".to_owned()))?;
    let submission = submission.ok_or_else(|| (64, "missing --submission".to_owned()))?;
    let output = output.ok_or_else(|| (64, "missing --output".to_owned()))?;

    match audit_files(&report, &policy, &input, &fragments_dir, &submission) {
        Ok(receipt) => {
            let bytes = serde_json::to_vec_pretty(&receipt)
                .map_err(|error| (70, format!("cannot serialize receipt: {error}")))?;
            std::fs::write(&output, [bytes.as_slice(), b"\n"].concat())
                .map_err(|error| (74, format!("cannot write receipt {}: {error}", output.display())))?;
            println!("artifact integrity admitted; physics comparison not evaluated; qualification abstains");
            Ok(())
        }
        Err(errors) => {
            let rendered = serde_json::to_string_pretty(&errors)
                .unwrap_or_else(|_| "metrics admission rejected".to_owned());
            Err((2, rendered))
        }
    }
}
