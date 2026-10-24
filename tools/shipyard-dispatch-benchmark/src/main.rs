use shipyard_dispatch_benchmark::{build_corpus_bundle, write_corpus_bundle, CorpusConfig};
use std::path::PathBuf;
use std::process::ExitCode;

fn usage() {
    eprintln!(
        "usage: shipyard-dispatch-benchmark --episodes <multiple-of-8> --seed <u64> --out <dir> --policy-revision <sha-or-id> --evaluator-revision <sha-or-id>"
    );
}

fn parse_args() -> Result<CorpusConfig, String> {
    let mut episodes = None;
    let mut seed = None;
    let mut output = None::<PathBuf>;
    let mut policy_revision = None;
    let mut evaluator_revision = None;
    let mut args = std::env::args().skip(1);

    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--episodes" => {
                episodes = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| "--episodes must be a positive integer".to_owned())?,
                );
            }
            "--seed" => {
                seed = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| "--seed must be a non-negative u64 integer".to_owned())?,
                );
            }
            "--out" => output = Some(PathBuf::from(value)),
            "--policy-revision" => policy_revision = Some(value),
            "--evaluator-revision" => evaluator_revision = Some(value),
            "--help" | "-h" => {
                usage();
                return Err("help requested".to_owned());
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }

    if output.is_none() {
        return Err("--out is required".to_owned());
    }
    Ok(CorpusConfig {
        master_seed: seed.ok_or_else(|| "--seed is required".to_owned())?,
        episode_count: episodes.ok_or_else(|| "--episodes is required".to_owned())?,
        policy_revision: policy_revision
            .ok_or_else(|| "--policy-revision is required".to_owned())?,
        evaluator_revision: evaluator_revision
            .ok_or_else(|| "--evaluator-revision is required".to_owned())?,
    })
}

fn run() -> Result<(), String> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    if raw_args.len() == 1 && (raw_args[0] == "--help" || raw_args[0] == "-h") {
        usage();
        return Ok(());
    }

    let mut output_dir = None;
    let mut args = raw_args.iter();
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("missing value for {flag}"))?;
        if flag == "--out" {
            output_dir = Some(PathBuf::from(value));
        }
    }
    let output_dir = output_dir.ok_or_else(|| "--out is required".to_owned())?;
    let config = parse_args()?;
    let bundle = build_corpus_bundle(&config)?;
    write_corpus_bundle(&bundle, &output_dir)?;

    println!("status=CORPUS_WRITTEN");
    println!("generator_version=shipyard-paired-dispatch-corpus-v0");
    println!("episode_count={}", config.episode_count);
    println!("ledger_rows={}", bundle.ledger_jsonl.lines().count());
    println!("artifact_count={}", bundle.artifacts.len());
    println!("manifest_sha256={}", bundle.manifest_sha256);
    println!("ledger_sha256={}", bundle.ledger_sha256);
    println!("paired_summary_sha256={}", bundle.paired_summary_sha256);
    println!("run_receipt_sha256={}", bundle.run_receipt_sha256);
    println!("output_dir={}", output_dir.display());
    eprintln!("WARNING: synthetic corpus only; the runner does not enforce compute or wall-clock budgets.");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            usage();
            ExitCode::from(2)
        }
    }
}
