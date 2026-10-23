use shipyard_disturbance_tape::validate_file;
use std::env;
use std::path::Path;
use std::process::ExitCode;

fn usage() {
    eprintln!(
        "usage: shipyard-disturbance-tape validate <tape.json> <expected-sha256> <as-of-tick> <policy-snapshot.json> <evaluator-stream.json>"
    );
}

fn fail(message: impl AsRef<str>) -> ExitCode {
    eprintln!("{}", message.as_ref());
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 7 || args[1] != "validate" {
        usage();
        return ExitCode::from(2);
    }

    let as_of_tick = match args[4].parse::<u64>() {
        Ok(value) => value,
        Err(_) => return fail("as-of-tick must be a non-negative u64 integer"),
    };
    if args[2] == args[5] || args[2] == args[6] || args[5] == args[6] {
        return fail("input, policy snapshot, and evaluator stream must use distinct output paths");
    }

    let report = match validate_file(
        Path::new(&args[2]),
        &args[3],
        as_of_tick,
        Path::new(&args[5]),
        Path::new(&args[6]),
    ) {
        Ok(report) => report,
        Err(error) => return fail(error),
    };

    println!("VALID shipyard disturbance tape");
    println!("episode_id={}", report.episode_id);
    println!("event_count={}", report.event_count);
    println!("as_of_tick={}", report.as_of_tick);
    println!("source_sha256={}", report.digest_sha256);
    println!("policy_snapshot={}", args[5]);
    println!("evaluator_stream={}", args[6]);
    eprintln!("SECURITY: do not expose the evaluator stream or full tape to the candidate policy");
    ExitCode::SUCCESS
}
