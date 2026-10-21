use pcb_route_benchmark::{compare_runs, evaluate_report_bytes};
use std::{
    env, fs,
    io::{self, Write},
    process::ExitCode,
};

fn main() -> ExitCode {
    match run() {
        Ok(bytes) => match io::stdout().write_all(&bytes) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("cannot write benchmark receipt: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("pcb-route-benchmark: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<Vec<u8>, String> {
    let evaluator_head = env::var("PCB_ROUTE_BENCHMARK_GIT_HEAD").map_err(|_| {
        "PCB_ROUTE_BENCHMARK_GIT_HEAD must bind the exact evaluator commit".to_string()
    })?;
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [command, report_path, manifest_path] if command == "validate" => {
            let report = fs::read(report_path)
                .map_err(|error| format!("cannot read {report_path}: {error}"))?;
            let manifest = fs::read(manifest_path)
                .map_err(|error| format!("cannot read {manifest_path}: {error}"))?;
            let validated = evaluate_report_bytes(&report, &manifest, &evaluator_head)?;
            serde_json::to_vec_pretty(&validated.receipt)
                .map_err(|error| format!("cannot serialize validation receipt: {error}"))
        }
        [command, baseline_path, treatment_path, manifest_path] if command == "compare" => {
            let baseline = fs::read(baseline_path)
                .map_err(|error| format!("cannot read {baseline_path}: {error}"))?;
            let treatment = fs::read(treatment_path)
                .map_err(|error| format!("cannot read {treatment_path}: {error}"))?;
            let manifest = fs::read(manifest_path)
                .map_err(|error| format!("cannot read {manifest_path}: {error}"))?;
            let baseline = evaluate_report_bytes(&baseline, &manifest, &evaluator_head)?;
            let treatment = evaluate_report_bytes(&treatment, &manifest, &evaluator_head)?;
            let comparison = compare_runs(&baseline, &treatment, &evaluator_head)?;
            serde_json::to_vec_pretty(&comparison)
                .map_err(|error| format!("cannot serialize paired comparison: {error}"))
        }
        _ => Err("usage: pcb-route-benchmark validate REPORT.json MANIFEST.json | compare BASELINE.json TREATMENT.json MANIFEST.json".into()),
    }
}
