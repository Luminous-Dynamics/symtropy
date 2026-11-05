#![forbid(unsafe_code)]

use construction_robot_trace_verifier::verify_trace_bytes;
use serde_json::json;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 || args[1] != "verify" {
        eprintln!("usage: construction-trace-verify verify TRACE.json [--expected-root SHA256]");
        std::process::exit(2);
    }

    let mut expected_root: Option<String> = None;
    let mut index = 3;
    while index < args.len() {
        if args[index] == "--expected-root" && index + 1 < args.len() {
            expected_root = Some(args[index + 1].clone());
            index += 2;
        } else {
            eprintln!("unknown or incomplete argument: {}", args[index]);
            std::process::exit(2);
        }
    }

    let bytes = match fs::read(&args[2]) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{}", json!({"integrity":"rejected","error":error.to_string()}));
            std::process::exit(1);
        }
    };

    match verify_trace_bytes(&bytes, expected_root.as_deref()) {
        Ok(report) => {
            println!(
                "{}",
                json!({
                    "integrity": report.integrity,
                    "completeness": report.completeness,
                    "external_anchor": report.external_anchor,
                    "trace_id": report.trace_id,
                    "event_count": report.event_count,
                    "verified_payload_bytes": report.verified_payload_bytes,
                    "final_trace_digest": report.final_trace_digest
                })
            );
        }
        Err(error) => {
            eprintln!("{}", json!({"integrity":"rejected","error":error.to_string()}));
            std::process::exit(1);
        }
    }
}
