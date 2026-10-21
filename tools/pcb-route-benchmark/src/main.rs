use pcb_route_benchmark::{
    build_dataset_manifest, compare_replicated_runs, compare_runs, evaluate_report_bytes,
    kicad_drc::verify_run_artifacts_with_kicad_drc_policy,
    pcbworld_metrics::import_pcbworld_per_rollout_csv, verify_dataset_files, verify_run_artifacts,
    ValidatedRun,
};
use std::{
    env, fs,
    io::{self, Write},
    path::Path,
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
        [command, split_config_path, intake_path, dataset_root]
            if command == "build-manifest" =>
        {
            let split_config = fs::read(split_config_path)
                .map_err(|error| format!("cannot read {split_config_path}: {error}"))?;
            let intake = fs::read(intake_path)
                .map_err(|error| format!("cannot read {intake_path}: {error}"))?;
            let manifest =
                build_dataset_manifest(&split_config, &intake, Path::new(dataset_root))?;
            serde_json::to_vec_pretty(&manifest)
                .map_err(|error| format!("cannot serialize dataset manifest: {error}"))
        }
        [command, manifest_path, dataset_root] if command == "verify-dataset" => {
            let manifest = fs::read(manifest_path)
                .map_err(|error| format!("cannot read {manifest_path}: {error}"))?;
            let receipt =
                verify_dataset_files(&manifest, Path::new(dataset_root), &evaluator_head)?;
            serde_json::to_vec_pretty(&receipt)
                .map_err(|error| format!("cannot serialize dataset verification receipt: {error}"))
        }
        [command, report_path, manifest_path, run_root] if command == "verify-run-artifacts" => {
            let report = fs::read(report_path)
                .map_err(|error| format!("cannot read {report_path}: {error}"))?;
            let manifest = fs::read(manifest_path)
                .map_err(|error| format!("cannot read {manifest_path}: {error}"))?;
            let receipt = verify_run_artifacts(
                &report,
                &manifest,
                Path::new(run_root),
                &evaluator_head,
            )?;
            serde_json::to_vec_pretty(&receipt)
                .map_err(|error| format!("cannot serialize run-artifact verification receipt: {error}"))
        }
        [command, report_path, manifest_path, run_root, policy_path]
            if command == "verify-kicad-run-artifacts" =>
        {
            let report = fs::read(report_path)
                .map_err(|error| format!("cannot read {report_path}: {error}"))?;
            let manifest = fs::read(manifest_path)
                .map_err(|error| format!("cannot read {manifest_path}: {error}"))?;
            let policy = fs::read(policy_path)
                .map_err(|error| format!("cannot read {policy_path}: {error}"))?;
            let receipt = verify_run_artifacts_with_kicad_drc_policy(
                &report,
                &manifest,
                Path::new(run_root),
                &policy,
                &evaluator_head,
            )?;
            serde_json::to_vec_pretty(&receipt).map_err(|error| {
                format!("cannot serialize raw-KiCad run-artifact receipt: {error}")
            })
        }
        [command, csv_path, manifest_path] if command == "import-pcbworld-csv" => {
            let csv_bytes = fs::read(csv_path)
                .map_err(|error| format!("cannot read {csv_path}: {error}"))?;
            let manifest_bytes = fs::read(manifest_path)
                .map_err(|error| format!("cannot read {manifest_path}: {error}"))?;
            let receipt = import_pcbworld_per_rollout_csv(
                &csv_bytes,
                &manifest_bytes,
                &evaluator_head,
            )?;
            serde_json::to_vec_pretty(&receipt)
                .map_err(|error| format!("cannot serialize PCBWorld CSV receipt: {error}"))
        }
        [command, report_path, manifest_path] if command == "validate" => {
            let report = fs::read(report_path)
                .map_err(|error| format!("cannot read {report_path}: {error}"))?;
            let manifest = fs::read(manifest_path)
                .map_err(|error| format!("cannot read {manifest_path}: {error}"))?;
            let validated = evaluate_report_bytes(&report, &manifest, &evaluator_head)?;
            serde_json::to_vec_pretty(&validated.receipt)
                .map_err(|error| format!("cannot serialize validation receipt: {error}"))
        }
        [command, baseline_dir, treatment_dir, manifest_path] if command == "compare-series" => {
            let manifest = fs::read(manifest_path)
                .map_err(|error| format!("cannot read {manifest_path}: {error}"))?;
            let baseline = load_report_series(baseline_dir, &manifest, &evaluator_head)?;
            let treatment = load_report_series(treatment_dir, &manifest, &evaluator_head)?;
            let comparison = compare_replicated_runs(&baseline, &treatment, &evaluator_head)?;
            serde_json::to_vec_pretty(&comparison)
                .map_err(|error| format!("cannot serialize replicated comparison: {error}"))
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
        _ => Err("usage: pcb-route-benchmark build-manifest D3.json INTAKE_SPEC.json DATASET_ROOT | verify-dataset MANIFEST.json DATASET_ROOT | verify-run-artifacts REPORT.json MANIFEST.json RUN_ROOT | verify-kicad-run-artifacts REPORT.json MANIFEST.json RUN_ROOT POLICY.json | import-pcbworld-csv PER_ROLLOUT.csv MANIFEST.json | validate REPORT.json MANIFEST.json | compare BASELINE.json TREATMENT.json MANIFEST.json | compare-series BASELINE_RUN_DIR TREATMENT_RUN_DIR MANIFEST.json".into()),
    }
}

fn load_report_series(
    directory: &str,
    manifest_bytes: &[u8],
    evaluator_git_head: &str,
) -> Result<Vec<ValidatedRun>, String> {
    let root = Path::new(directory);
    let mut paths = fs::read_dir(root)
        .map_err(|error| format!("cannot read run series directory {directory}: {error}"))?
        .map(|entry| {
            entry.map(|entry| entry.path()).map_err(|error| {
                format!("cannot enumerate run series directory {directory}: {error}")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    });
    paths.sort();
    if paths.is_empty() {
        return Err(format!(
            "run series directory {directory} contains no JSON run reports"
        ));
    }

    paths
        .into_iter()
        .map(|path| {
            let report_bytes = fs::read(&path)
                .map_err(|error| format!("cannot read run report {}: {error}", path.display()))?;
            evaluate_report_bytes(&report_bytes, manifest_bytes, evaluator_git_head)
                .map_err(|error| format!("run report {} rejected: {error}", path.display()))
        })
        .collect()
}
