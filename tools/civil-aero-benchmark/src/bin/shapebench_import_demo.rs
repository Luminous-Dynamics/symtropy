use civil_aero_benchmark::shapebench::{
    RewardDirection, ShapeBenchRunContext, import_shape_bench_results_csv,
};

const SYNTHETIC_RESULTS: &str = include_str!("../../data/shapebench_results_fixture.csv");

fn main() {
    if let Err(error) = run() {
        eprintln!("shapebench_import=reject error={error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let context = ShapeBenchRunContext {
        task_id: "superwing-sw001-synthetic-fixture".to_owned(),
        environment_id: "SuperWing".to_owned(),
        method_id: "CMA-ES".to_owned(),
        source_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
        task_config_sha256: "1".repeat(64),
        run_manifest_sha256: "2".repeat(64),
        results_json_sha256: None,
        cost_ledger_sha256: None,
    };
    let report =
        import_shape_bench_results_csv(SYNTHETIC_RESULTS, context, RewardDirection::Maximize)?;
    print!("{}", report.to_csv());
    eprintln!(
        "shapebench_import=pass schema={} rows={} task={} environment={} method={} cost_ledger_reference_present={} import_sha256={} evidence_level={}",
        report.schema,
        report.rows.len(),
        report.context.task_id,
        report.context.environment_id,
        report.context.method_id,
        report.cost_ledger_reference_present(),
        report.import_sha256,
        report.evidence_level
    );
    Ok(())
}
