//! Produce a digest-bound benchmark ledger from the end-to-end Energy Studio run.
//!
//! Only completed, passing independent audits enter the ledger. The report
//! separates generation/validation/compilation/dispatch/audit timings from
//! one-time binary build cost, and records exact byte digests for replay.

use std::{
    collections::BTreeSet,
    env,
    error::Error,
    fs,
    io,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing string field {key}")).into())
}

fn required_u64(value: &Value, key: &str) -> Result<u64, Box<dyn Error>> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing unsigned field {key}")).into())
}

fn required_bool(value: &Value, key: &str) -> Result<bool, Box<dyn Error>> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing bool field {key}")).into())
}

fn read_json(path: &Path) -> Result<(Vec<u8>, Value), Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let value = serde_json::from_slice(&bytes)?;
    Ok((bytes, value))
}

fn read_elapsed_ns(dir: &Path, phase: &str) -> Result<u64, Box<dyn Error>> {
    let raw = fs::read_to_string(dir.join(format!("{phase}.elapsed_ns")))?;
    Ok(raw.trim().parse::<u64>()?)
}

fn expected_candidate_ids() -> Vec<String> {
    let mut ids = vec![
        "synthetic-grid-only".to_owned(),
        "synthetic-pv-grid".to_owned(),
        "synthetic-pv-battery-grid".to_owned(),
    ];
    for pv in [1, 2, 3] {
        for capacity in [1, 2, 3] {
            for battery_power in [1, 2] {
                for grid_power in [1, 2] {
                    ids.push(format!(
                        "synthetic-parametric-pv-{pv}kw-battery-{capacity}kwh-battery-power-{battery_power}kw-grid-{grid_power}kw"
                    ));
                }
            }
        }
    }
    ids
}

#[derive(Debug, Serialize)]
struct PhaseTimings {
    generation_ns: u64,
    schema_validation_ns: u64,
    compilation_ns: u64,
    receipt_verification_ns: u64,
    dispatch_ns: u64,
    independent_audit_ns: u64,
    measured_pipeline_total_ns: u64,
}

#[derive(Debug, Serialize)]
struct CandidateBenchmark {
    candidate_id: String,
    candidate_family: String,
    generation_measurement_mode: String,
    candidate_sha256: String,
    compilation_sha256: String,
    dispatch_request_sha256: String,
    dispatch_report_sha256: String,
    independent_audit_sha256: String,
    synthetic: bool,
    case_id: String,
    solver_reported_search_is_exact: bool,
    solver_reported_frontier_complete: bool,
    solver_evaluated_action_expansions: u64,
    solver_feasible_complete_schedules: u64,
    solver_pareto_frontier_count: u64,
    independent_audit_status: String,
    independent_reference_frontier_status: String,
    independent_reference_expansions: u64,
    independent_reference_feasible_complete_schedules: u64,
    independent_objective_claim_status: String,
    independently_recovered_frontier: Vec<Value>,
    audit_check_count: usize,
    timings: PhaseTimings,
}

#[derive(Debug, Serialize)]
struct SourceHeads {
    symthaea: String,
    symtropy_contract: String,
    sol_atlas: String,
    symtropy_auditor: String,
}

#[derive(Debug, Serialize)]
struct BenchmarkSummary {
    expected_candidate_count: usize,
    audited_candidate_count: usize,
    reference_template_count: usize,
    parametric_variant_count: usize,
    failed_or_unknown_candidate_count: usize,
    shared_design_space_generation_ns: u64,
    total_solver_action_expansions: u64,
    total_independent_reference_expansions: u64,
    total_generation_ns: u64,
    total_schema_validation_ns: u64,
    total_compilation_ns: u64,
    total_receipt_verification_ns: u64,
    total_dispatch_ns: u64,
    total_independent_audit_ns: u64,
    max_dispatch_ns: u64,
    mean_dispatch_ns: u64,
}

#[derive(Debug, Serialize)]
struct EnergyStudioBenchmarkReport {
    schema_version: &'static str,
    benchmark_id: String,
    created_by_workflow_run: String,
    git_ref: String,
    source_heads: SourceHeads,
    host_os: &'static str,
    host_arch: &'static str,
    rust_toolchain: &'static str,
    measurement_scope: &'static str,
    design_space_manifest_sha256: String,
    benchmark_summary: BenchmarkSummary,
    candidates: Vec<CandidateBenchmark>,
}

fn require_sha_match(
    label: &str,
    actual: &str,
    expected: &str,
) -> Result<(), Box<dyn Error>> {
    if actual != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} mismatch: reported {actual}, recomputed {expected}"),
        )
        .into());
    }
    Ok(())
}

fn benchmark_candidate(
    root: &Path,
    candidate_id: &str,
) -> Result<CandidateBenchmark, Box<dyn Error>> {
    let dir = root.join(candidate_id);
    let (candidate_bytes, candidate) = read_json(&dir.join("candidate.json"))?;
    let (compilation_bytes, compilation) = read_json(&dir.join("compilation.json"))?;
    let (request_bytes, request) = read_json(&dir.join("dispatch-request.json"))?;
    let (dispatch_bytes, dispatch) = read_json(&dir.join("dispatch-report.json"))?;
    let (audit_bytes, audit) = read_json(&dir.join("independent-audit.json"))?;

    if required_str(&candidate, "candidate_id")? != candidate_id {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "candidate ID does not match output directory",
        )
        .into());
    }
    if required_str(&compilation, "candidate_id")? != candidate_id
        || required_str(&request, "case_id")? != format!("candidate:{candidate_id}")
        || required_bool(&candidate, "synthetic")? != required_bool(&request, "synthetic")?
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "candidate, compilation and dispatch-request identities disagree",
        )
        .into());
    }
    let receipt = dispatch.get("receipt").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "dispatch report has no receipt object")
    })?;
    let assessment = dispatch.get("assessment").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "dispatch report has no assessment object")
    })?;

    let candidate_digest = sha256_hex(&candidate_bytes);
    let compilation_digest = sha256_hex(&compilation_bytes);
    let request_digest = sha256_hex(&request_bytes);
    let dispatch_digest = sha256_hex(&dispatch_bytes);
    let audit_digest = sha256_hex(&audit_bytes);

    require_sha_match(
        "compilation candidate digest",
        required_str(&compilation, "candidate_sha256")?,
        &candidate_digest,
    )?;
    require_sha_match(
        "compilation dispatch request digest",
        required_str(&compilation, "dispatch_request_sha256")?,
        &request_digest,
    )?;
    require_sha_match(
        "dispatch receipt request digest",
        required_str(receipt, "input_sha256")?,
        &request_digest,
    )?;
    require_sha_match(
        "audit request digest",
        required_str(&audit, "request_sha256")?,
        &request_digest,
    )?;
    require_sha_match(
        "audit run-report digest",
        required_str(&audit, "run_report_sha256")?,
        &dispatch_digest,
    )?;

    let solver_is_exact = required_bool(assessment, "search_is_exact")?;
    let solver_frontier_complete = required_bool(assessment, "pareto_frontier_complete")?;
    let solver_expansions = required_u64(assessment, "evaluated_action_expansions")?;
    let solver_feasible_schedules = required_u64(assessment, "feasible_complete_schedules")?;
    let solver_frontier_count = required_u64(assessment, "pareto_frontier_count")?;
    let audit_status = required_str(&audit, "status")?.to_owned();
    let reference_status = required_str(&audit, "reference_frontier_status")?.to_owned();
    let reference_expansions = audit
        .get("reference_expansions")
        .and_then(Value::as_u64)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "reference expansions are missing"))?;
    let reference_schedules = audit
        .get("reference_feasible_complete_schedules")
        .and_then(Value::as_u64)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "reference feasible schedule count is missing"))?;
    let objective_claim = required_str(&audit, "objective_claim_status")?.to_owned();
    let frontier = audit
        .get("reference_frontier_objectives")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "independent frontier is missing"))?
        .clone();
    let audit_checks = audit
        .get("checks")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "audit checks are missing"))?;
    let checks_all_pass = audit_checks.iter().all(|item| {
        item.get("status").and_then(Value::as_str) == Some("pass")
    });

    if audit_status != "pass"
        || reference_status != "pass"
        || objective_claim != "verified_for_bounded_model_case"
        || !checks_all_pass
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "candidate {candidate_id} failed independent qualification (audit={audit_status}, reference={reference_status}, objective={objective_claim})"
            ),
        )
        .into());
    }

    let (generation_ns, generation_measurement_mode) =
        if candidate_id.starts_with("synthetic-parametric-") {
            let raw = fs::read_to_string(root.join("manifest-generation.elapsed_ns"))?;
            (raw.trim().parse::<u64>()? / 36, "amortized_shared_manifest_export".to_owned())
        } else {
            (read_elapsed_ns(&dir, "generation")?, "individual_candidate_export".to_owned())
        };
    let timings = PhaseTimings {
        generation_ns,
        schema_validation_ns: read_elapsed_ns(&dir, "schema")?,
        compilation_ns: read_elapsed_ns(&dir, "compilation")?,
        receipt_verification_ns: read_elapsed_ns(&dir, "receipt")?,
        dispatch_ns: read_elapsed_ns(&dir, "dispatch")?,
        independent_audit_ns: read_elapsed_ns(&dir, "audit")?,
        measured_pipeline_total_ns: 0,
    };
    let measured_total = timings.generation_ns
        .saturating_add(timings.schema_validation_ns)
        .saturating_add(timings.compilation_ns)
        .saturating_add(timings.receipt_verification_ns)
        .saturating_add(timings.dispatch_ns)
        .saturating_add(timings.independent_audit_ns);
    let timings = PhaseTimings {
        measured_pipeline_total_ns: measured_total,
        ..timings
    };

    let family = if candidate_id.starts_with("synthetic-parametric-") {
        "parametric_sizing".to_owned()
    } else {
        "reference_topology".to_owned()
    };
    Ok(CandidateBenchmark {
        candidate_id: candidate_id.to_owned(),
        candidate_family: family,
        generation_measurement_mode,
        candidate_sha256: candidate_digest,
        compilation_sha256: compilation_digest,
        dispatch_request_sha256: request_digest,
        dispatch_report_sha256: dispatch_digest,
        independent_audit_sha256: audit_digest,
        synthetic: required_bool(&candidate, "synthetic")?,
        case_id: required_str(assessment, "case_id")?.to_owned(),
        solver_reported_search_is_exact: solver_is_exact,
        solver_reported_frontier_complete: solver_frontier_complete,
        solver_evaluated_action_expansions: solver_expansions,
        solver_feasible_complete_schedules: solver_feasible_schedules,
        solver_pareto_frontier_count: solver_frontier_count,
        independent_audit_status: audit_status,
        independent_reference_frontier_status: reference_status,
        independent_reference_expansions: reference_expansions,
        independent_reference_feasible_complete_schedules: reference_schedules,
        independent_objective_claim_status: objective_claim,
        independently_recovered_frontier: frontier,
        audit_check_count: audit_checks.len(),
        timings,
    })
}

fn sum_checked<F>(
    candidates: &[CandidateBenchmark],
    mut select: F,
) -> Result<u64, Box<dyn Error>>
where
    F: FnMut(&CandidateBenchmark) -> u64,
{
    candidates.iter().try_fold(0_u64, |total, candidate| {
        total.checked_add(select(candidate)).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "benchmark metric sum overflow").into()
        })
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_benchmark_report <candidate-root-directory> <output-report.json>",
        )
        .into());
    }
    let root = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let expected_ids = expected_candidate_ids();
    let expected_set = expected_ids.iter().cloned().collect::<BTreeSet<_>>();
    let manifest_path = root.join("energy-design-space-manifest.json");
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    if manifest.get("requested_candidate_count").and_then(Value::as_u64) != Some(36)
        || manifest.get("candidates").and_then(Value::as_array).map(Vec::len) != Some(36)
        || manifest.get("rejected_candidates").and_then(Value::as_array).map(Vec::len) != Some(0)
    {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "design-space manifest is not the full 36-candidate sweep").into());
    }

    let mut observed_ids = fs::read_dir(&root)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().ok().is_some_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("synthetic-"))
        .collect::<Vec<_>>();
    observed_ids.sort();
    let mut expected_sorted = expected_ids.clone();
    expected_sorted.sort();
    if observed_ids != expected_sorted || observed_ids.len() != 39 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "candidate directory set does not match the frozen 39-case benchmark (observed {})",
                observed_ids.len()
            ),
        )
        .into());
    }
    if expected_set.len() != 39 {
        return Err(io::Error::other("internal expected candidate list has duplicate IDs").into());
    }

    let mut candidates = expected_ids
        .iter()
        .map(|id| benchmark_candidate(&root, id))
        .collect::<Result<Vec<_>, _>>()?;
    candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    let solver_expansions = sum_checked(&candidates, |candidate| candidate.solver_evaluated_action_expansions)?;
    let reference_expansions = sum_checked(&candidates, |candidate| candidate.independent_reference_expansions)?;
    let total_generation = sum_checked(&candidates, |candidate| candidate.timings.generation_ns)?;
    let shared_design_space_generation = fs::read_to_string(root.join("manifest-generation.elapsed_ns"))?
        .trim()
        .parse::<u64>()?;
    let total_schema = sum_checked(&candidates, |candidate| candidate.timings.schema_validation_ns)?;
    let total_compilation = sum_checked(&candidates, |candidate| candidate.timings.compilation_ns)?;
    let total_receipt = sum_checked(&candidates, |candidate| candidate.timings.receipt_verification_ns)?;
    let total_dispatch = sum_checked(&candidates, |candidate| candidate.timings.dispatch_ns)?;
    let total_audit = sum_checked(&candidates, |candidate| candidate.timings.independent_audit_ns)?;
    let max_dispatch = candidates.iter().map(|candidate| candidate.timings.dispatch_ns).max().unwrap_or(0);
    let mean_dispatch = total_dispatch / candidates.len() as u64;

    let report = EnergyStudioBenchmarkReport {
        schema_version: "energy-studio-benchmark-report-v0.1",
        benchmark_id: "energy-studio-synthetic-sizing-sweep-v0.1".to_owned(),
        created_by_workflow_run: env::var("GITHUB_RUN_ID").unwrap_or_else(|_| "local".to_owned()),
        git_ref: env::var("GITHUB_SHA").unwrap_or_else(|_| "unknown".to_owned()),
        source_heads: SourceHeads {
            symthaea: env::var("SYMTHAEA_PIN")?,
            symtropy_contract: env::var("SYMTROPY_CONTRACT_PIN")?,
            sol_atlas: env::var("SOL_ATLAS_PIN")?,
            symtropy_auditor: env::var("GITHUB_SHA").unwrap_or_else(|_| "unknown".to_owned()),
        },
        host_os: env::consts::OS,
        host_arch: env::consts::ARCH,
        rust_toolchain: "1.96.0",
        measurement_scope: "Per-candidate wall-clock process runtimes from a warmed build; excludes one-time dependency compilation and runner provisioning. These synthetic finite-case measurements are not a real-site or SOTA comparison.",
        design_space_manifest_sha256: sha256_hex(&manifest_bytes),
        benchmark_summary: BenchmarkSummary {
            expected_candidate_count: 39,
            audited_candidate_count: candidates.len(),
            reference_template_count: 3,
            parametric_variant_count: 36,
            failed_or_unknown_candidate_count: 0,
            shared_design_space_generation_ns: shared_design_space_generation,
            total_solver_action_expansions: solver_expansions,
            total_independent_reference_expansions: reference_expansions,
            total_generation_ns: total_generation,
            total_schema_validation_ns: total_schema,
            total_compilation_ns: total_compilation,
            total_receipt_verification_ns: total_receipt,
            total_dispatch_ns: total_dispatch,
            total_independent_audit_ns: total_audit,
            max_dispatch_ns: max_dispatch,
            mean_dispatch_ns: mean_dispatch,
        },
        candidates,
    };
    let output_bytes = serde_json::to_vec_pretty(&report)?;
    fs::write(&output, &output_bytes)?;
    println!(
        "PASS: benchmark ledger recorded {} audited candidates; total dispatch={} ns; mean dispatch={} ns; max dispatch={} ns",
        report.benchmark_summary.audited_candidate_count,
        report.benchmark_summary.total_dispatch_ns,
        report.benchmark_summary.mean_dispatch_ns,
        report.benchmark_summary.max_dispatch_ns
    );
    println!("report_sha256={}", sha256_hex(&output_bytes));
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy benchmark report failed: {error}");
        std::process::exit(2);
    }
}
