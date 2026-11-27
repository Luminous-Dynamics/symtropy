use ev_nlr_profile_audit::replay::run_source_replay;
use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let mut schedule_root = None;
    let mut source_script = None;
    let mut output_dir = None;
    let mut source_revision = None;
    let mut report_path = None;
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--schedule-root" => schedule_root = args.next().map(PathBuf::from),
            "--source-script" => source_script = args.next().map(PathBuf::from),
            "--output-dir" => output_dir = args.next().map(PathBuf::from),
            "--source-revision" => source_revision = args.next(),
            "--report" => report_path = args.next().map(PathBuf::from),
            "--help" | "-h" => {
                println!("usage: ev-nlr-source-replay --schedule-root PATH --source-script PATH --output-dir PATH --source-revision SHA --report PATH");
                return ExitCode::SUCCESS;
            }
            _ => {
                eprintln!("unknown argument: {argument}");
                return ExitCode::FAILURE;
            }
        }
    }
    let (
        Some(schedule_root),
        Some(source_script),
        Some(output_dir),
        Some(source_revision),
        Some(report_path),
    ) = (
        schedule_root,
        source_script,
        output_dir,
        source_revision,
        report_path,
    )
    else {
        eprintln!("missing required arguments; pass --help for usage");
        return ExitCode::FAILURE;
    };
    match run_source_replay(
        &schedule_root,
        &source_script,
        &output_dir,
        &source_revision,
        &report_path,
    ) {
        Ok(envelope) => {
            println!(
                "generated {} profiles / {} intervals; report SHA-256 {}",
                envelope.report.generated_profile_count,
                envelope.report.generated_total_interval_count,
                envelope.report_sha256
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("NLR source replay failed: {error}");
            ExitCode::FAILURE
        }
    }
}
