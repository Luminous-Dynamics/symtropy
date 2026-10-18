use civ_infra_adaptive_investigation::evaluate_bytes;
use std::env;
use std::fs;

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if !(2..=3).contains(&args.len()) {
        return Err("usage: civ-infra-adaptive-investigation INPUT.json [REPORT.json]".to_owned());
    }
    let input_path = &args[1];
    let bytes = fs::read(input_path)
        .map_err(|error| format!("cannot read input {input_path:?}: {error}"))?;
    let report = evaluate_bytes(&bytes)?;
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("cannot serialize policy report: {error}"))?;

    if let Some(output_path) = args.get(2) {
        fs::write(output_path, &report_bytes)
            .map_err(|error| format!("cannot write report {output_path:?}: {error}"))?;
        println!(
            "report_written={} input_sha256={} leaves={} supported={} support_failures={}",
            output_path,
            report.input_sha256,
            report.leaves.len(),
            report.supported_leaf_count,
            report.support_failure_leaf_count
        );
    } else {
        println!(
            "{}",
            String::from_utf8(report_bytes)
                .map_err(|error| format!("report is not valid UTF-8: {error}"))?
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("adaptive investigation evaluation failed: {error}");
        std::process::exit(1);
    }
}
