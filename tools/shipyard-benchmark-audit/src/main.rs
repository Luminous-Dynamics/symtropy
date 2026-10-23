use std::env;
use std::fs;
use std::process::ExitCode;

fn usage() {
    eprintln!("usage: shipyard-benchmark-audit manifest <manifest.json>");
    eprintln!("   or: shipyard-benchmark-audit report <manifest.json> <report.json>");
    eprintln!("   or: shipyard-benchmark-audit verify-digest <artifact-file> <expected-sha256>");
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        usage();
        return ExitCode::from(2);
    }
    let mode = args[1].as_str();
    if mode == "verify-digest" {
        if args.len() != 4 {
            usage();
            return ExitCode::from(2);
        }
        let bytes = match fs::read(&args[2]) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("input error: cannot read {}: {}", args[2], e);
                return ExitCode::from(2);
            }
        };
        let expected = &args[3];
        if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
            eprintln!("input error: expected digest must contain 64 hexadecimal characters");
            return ExitCode::from(2);
        }
        let actual = shipyard_benchmark_audit::sha256_hex(&bytes);
        if actual.eq_ignore_ascii_case(expected) {
            println!("VALID SHA-256 {}", actual);
            return ExitCode::SUCCESS;
        }
        eprintln!(
            "INVALID: SHA-256 mismatch: expected {}, got {}",
            expected, actual
        );
        return ExitCode::from(1);
    }

    let manifest = match fs::read_to_string(&args[2]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("input error: cannot read {}: {}", args[2], e);
            return ExitCode::from(2);
        }
    };
    let errors = match mode {
        "manifest" if args.len() == 3 => {
            shipyard_benchmark_audit::validate_manifest_json(&manifest)
        }
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
