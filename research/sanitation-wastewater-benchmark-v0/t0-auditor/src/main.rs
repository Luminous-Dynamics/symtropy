#![forbid(unsafe_code)]

use sanitation_t0_auditor::audit_case;
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: sanitation-t0-auditor <case.json>");
        return ExitCode::from(64);
    };

    let input = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::from(66);
        }
    };

    match audit_case(&input) {
        Ok(report) => {
            match serde_json::to_string_pretty(&report) {
                Ok(rendered) => println!("{rendered}"),
                Err(error) => {
                    eprintln!("cannot render audit report: {error}");
                    return ExitCode::from(70);
                }
            }
            if report.oracle_match {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("audit input rejected: {error}");
            ExitCode::from(2)
        }
    }
}
