use std::env;
use std::fs;
use std::process::ExitCode;

fn usage() {
    eprintln!("usage: shipyard-benchmark-audit manifest <manifest.json>");
    eprintln!("   or: shipyard-benchmark-audit report <manifest.json> <report.json>");
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        usage();
        return ExitCode::from(2);
    }
    let mode = args[1].as_str();
    let manifest = match fs::read_to_string(&args[2]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("input error: cannot read {}: {}", args[2], e);
            return ExitCode::from(2);
        }
    };
    let errors = match mode {
        "manifest" if args.len() == 3 => shipyard_benchmark_audit::validate_manifest_json(&manifest),
        "report" if args.len() == 4 => {
            let report = match fs::read_to_string(&args[3]) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("input error: cannot read {}: {}", args[3], e);
                    return ExitCode::from(2);
                }
            };
            shipyard_benchmark_audit::audit_report_json(&manifest, &report)
        }
        _ => {
            usage();
            return ExitCode::from(2);
        }
    };
    if errors.is_empty() {
        println!("VALID");
        ExitCode::SUCCESS
    } else {
        for error in &errors {
            println!("INVALID: {}", error);
        }
        ExitCode::from(1)
    }
}
