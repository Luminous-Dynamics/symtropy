use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, env, fs, process};

const UPSTREAM_COMMIT: &str = "d088a26582f5cb79c6b4f2ad94f68ac7c50878cb";
const UPSTREAM_BLOB_SHA1: &str = "6bac8cfaf9f326f6405d5396b9cedd12941aba94";
const UPSTREAM_BLOB_SHA1_CASE_02: &str = "aa051d7c71a0fc2949ce69d67bce0ec54f4a11d1";
const UPSTREAM_REPOSITORY: &str =
    "https://github.com/ai-for-decision-making-tue/Job_Shop_Scheduling_Benchmark_Environments_and_Instances";
const UPSTREAM_INSTANCE_PATH: &str = "data/fjsp_sdst/fattahi/Fattahi_setup_01.fjs";
const UPSTREAM_INSTANCE_PATH_CASE_02: &str = "data/fjsp_sdst/fattahi/Fattahi_setup_02.fjs";
const UPSTREAM_BLOB_SHA1_CASE_03: &str = "01c1ff749cbad87206cbc4f2ba308c005dd8b1c9";
const UPSTREAM_INSTANCE_PATH_CASE_03: &str = "data/fjsp_sdst/fattahi/Fattahi_setup_03.fjs";
const CLAIM_CEILING: &str = "Pinned public benchmark input structures and exhaustive tiny-instance makespan oracles for named cases are validated only. No dispatching policy was executed, and no due-date, SOTA comparison, or production claim is established.";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: String,
    benchmark_id: String,
    source: SourcePin,
    expected_structure: ExpectedStructure,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct SourcePin {
    repository_url: String,
    commit: String,
    license: String,
    paper_doi: String,
    instance_path: String,
    instance_git_blob_sha1: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ExpectedStructure {
    jobs: usize,
    machines: usize,
    max_operations_per_job: usize,
    operations: usize,
    operation_alternative_counts: Vec<usize>,
    declared_average_machine_flexibility: f64,
    setup_matrix_dimensions: [usize; 3],
    setup_values_min: i64,
    setup_values_max: i64,
    primary_objective: String,
    supports_due_dates: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    status: String,
    benchmark_id: String,
    source_repository_url: String,
    source_commit: String,
    source_instance_path: String,
    expected_source_git_blob_sha1: String,
    source_license: String,
    paper_doi: String,
    observed_jobs: usize,
    observed_machines: usize,
    observed_max_operations_per_job: usize,
    observed_operations: usize,
    operation_alternative_counts: Vec<usize>,
    operations: Vec<OperationSummary>,
    declared_average_machine_flexibility: f64,
    setup_matrix_dimensions: [usize; 3],
    setup_values_min: i64,
    setup_values_max: i64,
    primary_objective: String,
    supports_due_dates: bool,
    exact_oracle: ExactOracleSummary,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct OperationSummary {
    operation_id: usize,
    job_id: usize,
    position_in_job: usize,
    machine_options: Vec<MachineOption>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct MachineOption {
    machine_id_upstream: usize,
    processing_time: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct ParsedInstance {
    jobs: usize,
    machines: usize,
    max_operations_per_job: usize,
    operation_alternative_counts: Vec<usize>,
    operations: Vec<OperationSummary>,
    declared_average_machine_flexibility: f64,
    setup_matrix_dimensions: [usize; 3],
    setup_values_min: i64,
    setup_values_max: i64,
    setup_times: Vec<Vec<Vec<u64>>>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ExactOracleSummary {
    algorithm: String,
    setup_semantics: String,
    candidate_assignment_sequence_combinations: usize,
    feasible_acyclic_schedules: usize,
    optimal_makespan: u64,
    optimal_machine_assignment_upstream_ids: Vec<usize>,
    optimal_machine_sequences_operation_ids: Vec<Vec<usize>>,
    optimal_schedule: Vec<OperationSchedule>,
    exhaustive_for_declared_instance: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct OperationSchedule {
    operation_id: usize,
    job_id: usize,
    position_in_job: usize,
    machine_id_upstream: usize,
    setup_time_before_processing: u64,
    processing_time: u64,
    processing_start: u64,
    completion_time: u64,
}

#[derive(Debug, Clone)]
struct OracleCandidate {
    makespan: u64,
    machine_ids_zero_based: Vec<usize>,
    machine_sequences: Vec<Vec<usize>>,
    schedule: Vec<OperationSchedule>,
}

#[derive(Debug, Default)]
struct OracleAccumulator {
    candidate_assignment_sequence_combinations: usize,
    feasible_acyclic_schedules: usize,
    best: Option<OracleCandidate>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("external_jobshop_parity_error: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: ev-external-jobshop-parity <instance.fjs> <manifest.json>".into());
    }
    let instance_text = fs::read_to_string(&args[1])
        .map_err(|error| format!("cannot_read_instance: {error}"))?;
    let manifest_text = fs::read_to_string(&args[2])
        .map_err(|error| format!("cannot_read_manifest: {error}"))?;
    let manifest: Manifest = serde_json::from_str(&manifest_text)
        .map_err(|error| format!("invalid_manifest_json: {error}"))?;
    validate_manifest(&manifest)?;
    let parsed = parse_instance(&instance_text)?;
    validate_expected_structure(&manifest.expected_structure, &parsed)?;
    let exact_oracle = solve_tiny_instance(&parsed)?;

    let report = Report {
        schema_version: "external-jobshop-input-parity/0.2".into(),
        status: "input_and_tiny_exact_makespan_oracle_validated_not_policy_benchmarked".into(),
        benchmark_id: manifest.benchmark_id,
        source_repository_url: manifest.source.repository_url,
        source_commit: manifest.source.commit,
        source_instance_path: manifest.source.instance_path,
        expected_source_git_blob_sha1: manifest.source.instance_git_blob_sha1,
        source_license: manifest.source.license,
        paper_doi: manifest.source.paper_doi,
        observed_jobs: parsed.jobs,
        observed_machines: parsed.machines,
        observed_max_operations_per_job: parsed.max_operations_per_job,
        observed_operations: parsed.operations.len(),
        operation_alternative_counts: parsed.operation_alternative_counts,
        operations: parsed.operations,
        declared_average_machine_flexibility: parsed.declared_average_machine_flexibility,
        setup_matrix_dimensions: parsed.setup_matrix_dimensions,
        setup_values_min: parsed.setup_values_min,
        setup_values_max: parsed.setup_values_max,
        primary_objective: manifest.expected_structure.primary_objective,
        supports_due_dates: manifest.expected_structure.supports_due_dates,
        exact_oracle,
        claim_ceiling: manifest.claim_ceiling,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("cannot_serialize_report: {error}"))?
    );
    Ok(())
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.schema_version != "external-jobshop-parity-manifest/0.2"
        || manifest.claim_ceiling != CLAIM_CEILING
    {
        return Err("unsupported_manifest_version_id_or_claim_ceiling".into());
    }

    let (
        expected_path,
        expected_blob,
        expected_jobs,
        expected_operations,
        expected_alternatives,
        expected_flexibility,
        expected_setup_dimensions,
        expected_setup_min,
        expected_setup_max,
    ) = match manifest.benchmark_id.as_str() {
        "FJSP-SDST-FATTAHI-001" => (
            UPSTREAM_INSTANCE_PATH,
            UPSTREAM_BLOB_SHA1,
            2,
            4,
            vec![2, 2, 2, 2],
            2.0,
            [2, 4, 4],
            3,
            8,
        ),
        "FJSP-SDST-FATTAHI-002" => (
            UPSTREAM_INSTANCE_PATH_CASE_02,
            UPSTREAM_BLOB_SHA1_CASE_02,
            2,
            4,
            vec![1, 2, 2, 1],
            1.5,
            [2, 4, 4],
            3,
            1_000_000,
        ),
        "FJSP-SDST-FATTAHI-003" => (
            UPSTREAM_INSTANCE_PATH_CASE_03,
            UPSTREAM_BLOB_SHA1_CASE_03,
            3,
            6,
            vec![1, 2, 2, 1, 2, 2],
            1.7,
            [2, 6, 6],
            6,
            1_000_000,
        ),
        _ => return Err("unsupported_benchmark_case".into()),
    };

    let source = &manifest.source;
    if source.repository_url != UPSTREAM_REPOSITORY
        || source.commit != UPSTREAM_COMMIT
        || source.license != "MIT"
        || source.paper_doi != "10.1007/s10472-026-10007-3"
        || source.instance_path != expected_path
        || source.instance_git_blob_sha1 != expected_blob
    {
        return Err("upstream_source_pin_mismatch".into());
    }
    if !is_lower_hex_40(&source.commit) || !is_lower_hex_40(&source.instance_git_blob_sha1) {
        return Err("upstream_revision_or_blob_digest_malformed".into());
    }

    let expected = &manifest.expected_structure;
    if expected.jobs != expected_jobs
        || expected.machines != 2
        || expected.max_operations_per_job != 2
        || expected.operations != expected_operations
        || expected.operation_alternative_counts != expected_alternatives
        || expected.declared_average_machine_flexibility != expected_flexibility
        || expected.setup_matrix_dimensions != expected_setup_dimensions
        || expected.setup_values_min != expected_setup_min
        || expected.setup_values_max != expected_setup_max
        || expected.primary_objective != "makespan"
        || expected.supports_due_dates
    {
        return Err("expected_structure_contract_mismatch".into());
    }
    Ok(())
}

fn is_lower_hex_40(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_instance(input: &str) -> Result<ParsedInstance, String> {
    let lines: Vec<&str> = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return Err("instance_has_no_nonempty_lines".into());
    }
    let header: Vec<&str> = lines[0].split_whitespace().collect();
    if header.len() != 3 {
        return Err("header_must_contain_job_machine_and_average_flexibility_fields".into());
    }
    let jobs = header[0]
        .parse::<usize>()
        .map_err(|_| "invalid_job_count_header".to_string())?;
    let machines = header[1]
        .parse::<usize>()
        .map_err(|_| "invalid_machine_count_header".to_string())?;
    let declared_average_machine_flexibility = header[2]
        .parse::<f64>()
        .map_err(|_| "invalid_average_machine_flexibility_header".to_string())?;
    if jobs == 0
        || machines == 0
        || !declared_average_machine_flexibility.is_finite()
        || declared_average_machine_flexibility <= 0.0
        || declared_average_machine_flexibility > machines as f64
    {
        return Err("invalid_job_machine_count_or_average_flexibility_header".into());
    }
    if jobs > 100_000 || machines > 10_000 {
        return Err("instance_dimensions_exceed_contract_limits".into());
    }

    let mut line_index = 1usize;
    let mut operations = Vec::new();
    let mut operation_alternative_counts = Vec::new();
    let mut max_observed_operations = 0usize;
    for job_id in 0..jobs {
        let line = lines
            .get(line_index)
            .ok_or_else(|| format!("missing_job_row_{job_id}"))?;
        line_index += 1;
        let values = parse_usize_tokens(line, &format!("job_row_{job_id}"))?;
        let operation_count = *values
            .first()
            .ok_or_else(|| format!("empty_job_row_{job_id}"))?;
        if operation_count == 0 || operation_count > 100_000 {
            return Err(format!("invalid_operation_count_for_job_{job_id}"));
        }
        max_observed_operations = max_observed_operations.max(operation_count);
        let mut cursor = 1usize;
        for position_in_job in 0..operation_count {
            let alternative_count = *values.get(cursor).ok_or_else(|| {
                format!("missing_alternative_count_job_{job_id}_op_{position_in_job}")
            })?;
            cursor += 1;
            if alternative_count == 0 || alternative_count > machines {
                return Err(format!(
                    "invalid_alternative_count_job_{job_id}_op_{position_in_job}"
                ));
            }
            let mut machine_options = Vec::with_capacity(alternative_count);
            let mut seen_machines = std::collections::BTreeSet::new();
            for _ in 0..alternative_count {
                let machine_id_upstream = *values.get(cursor).ok_or_else(|| {
                    format!("missing_machine_id_job_{job_id}_op_{position_in_job}")
                })?;
                let processing_time = *values.get(cursor + 1).ok_or_else(|| {
                    format!("missing_processing_time_job_{job_id}_op_{position_in_job}")
                })? as u64;
                cursor += 2;
                if machine_id_upstream == 0 || machine_id_upstream > machines {
                    return Err(format!(
                        "machine_id_out_of_range_job_{job_id}_op_{position_in_job}"
                    ));
                }
                if processing_time == 0 {
                    return Err(format!(
                        "processing_time_must_be_positive_job_{job_id}_op_{position_in_job}"
                    ));
                }
                if !seen_machines.insert(machine_id_upstream) {
                    return Err(format!(
                        "duplicate_machine_option_job_{job_id}_op_{position_in_job}"
                    ));
                }
                machine_options.push(MachineOption {
                    machine_id_upstream,
                    processing_time,
                });
            }
            operation_alternative_counts.push(alternative_count);
            operations.push(OperationSummary {
                operation_id: operations.len(),
                job_id,
                position_in_job,
                machine_options,
            });
        }
        if cursor != values.len() {
            return Err(format!("unexpected_trailing_tokens_job_{job_id}"));
        }
    }

    let operation_count = operations.len();
    let expected_setup_rows = machines
        .checked_mul(operation_count)
        .ok_or_else(|| "setup_matrix_dimension_overflow".to_string())?;
    let setup_rows = &lines[line_index..];
    if setup_rows.len() != expected_setup_rows {
        return Err(format!(
            "setup_row_count_mismatch_expected_{expected_setup_rows}_observed_{}",
            setup_rows.len()
        ));
    }
    let mut setup_min: Option<i64> = None;
    let mut setup_max: Option<i64> = None;
    let mut setup_times = vec![vec![vec![0u64; operation_count]; operation_count]; machines];
    for (row_index, row) in setup_rows.iter().enumerate() {
        let values = parse_i64_tokens(row, &format!("setup_row_{row_index}"))?;
        if values.len() != operation_count {
            return Err(format!(
                "setup_row_width_mismatch_row_{row_index}_expected_{operation_count}_observed_{}",
                values.len()
            ));
        }
        let machine_id = row_index / operation_count;
        let from_operation = row_index % operation_count;
        for (to_operation, value) in values.into_iter().enumerate() {
            if value < 0 {
                return Err(format!("negative_setup_time_row_{row_index}"));
            }
            setup_min = Some(setup_min.map_or(value, |current| current.min(value)));
            setup_max = Some(setup_max.map_or(value, |current| current.max(value)));
            setup_times[machine_id][from_operation][to_operation] = value as u64;
        }
    }

    let observed_average_machine_flexibility =
        operation_alternative_counts.iter().sum::<usize>() as f64
            / operation_alternative_counts.len() as f64;
    let declared_flexibility_tenths =
        (declared_average_machine_flexibility * 10.0).round() as i64;
    let observed_flexibility_tenths =
        (observed_average_machine_flexibility * 10.0).round() as i64;
    if declared_flexibility_tenths != observed_flexibility_tenths {
        return Err("declared_average_machine_flexibility_mismatch".into());
    }

    Ok(ParsedInstance {
        jobs,
        machines,
        max_operations_per_job: max_observed_operations,
        operation_alternative_counts,
        operations,
        declared_average_machine_flexibility,
        setup_matrix_dimensions: [machines, operation_count, operation_count],
        setup_values_min: setup_min.ok_or_else(|| "setup_matrix_has_no_values".to_string())?,
        setup_values_max: setup_max.ok_or_else(|| "setup_matrix_has_no_values".to_string())?,
        setup_times,
    })
}

fn parse_usize_tokens(line: &str, context: &str) -> Result<Vec<usize>, String> {
    line.split_whitespace()
        .map(|token| {
            token
                .parse::<usize>()
                .map_err(|_| format!("invalid_unsigned_integer_{context}_{token}"))
        })
        .collect()
}

fn parse_i64_tokens(line: &str, context: &str) -> Result<Vec<i64>, String> {
    line.split_whitespace()
        .map(|token| {
            token
                .parse::<i64>()
                .map_err(|_| format!("invalid_integer_{context}_{token}"))
        })
        .collect()
}

fn validate_expected_structure(
    expected: &ExpectedStructure,
    parsed: &ParsedInstance,
) -> Result<(), String> {
    if parsed.jobs != expected.jobs
        || parsed.machines != expected.machines
        || parsed.max_operations_per_job != expected.max_operations_per_job
        || parsed.operations.len() != expected.operations
        || parsed.operation_alternative_counts != expected.operation_alternative_counts
        || parsed.declared_average_machine_flexibility
            != expected.declared_average_machine_flexibility
        || parsed.setup_matrix_dimensions != expected.setup_matrix_dimensions
        || parsed.setup_values_min != expected.setup_values_min
        || parsed.setup_values_max != expected.setup_values_max
    {
        return Err("observed_input_structure_differs_from_frozen_manifest".into());
    }
    Ok(())
}

fn solve_tiny_instance(parsed: &ParsedInstance) -> Result<ExactOracleSummary, String> {
    if !(2..=3).contains(&parsed.jobs)
        || parsed.machines != 2
        || !(4..=6).contains(&parsed.operations.len())
        || parsed.operations.len() % parsed.jobs != 0
    {
        return Err(
            "exact_oracle_supports_two_or_three_jobs_two_machines_and_up_to_six_operations".into(),
        );
    }
    if parsed.setup_times.len() != parsed.machines
        || parsed.setup_times.iter().any(|machine| {
            machine.len() != parsed.operations.len()
                || machine.iter().any(|row| row.len() != parsed.operations.len())
        })
    {
        return Err("setup_tensor_dimensions_do_not_match_operations".into());
    }

    let mut machine_ids = vec![0usize; parsed.operations.len()];
    let mut processing_times = vec![0u64; parsed.operations.len()];
    let mut accumulator = OracleAccumulator::default();
    enumerate_assignments(
        0,
        parsed,
        &mut machine_ids,
        &mut processing_times,
        &mut accumulator,
    )?;
    let best = accumulator
        .best
        .ok_or_else(|| "exact_oracle_found_no_feasible_acyclic_schedule".to_string())?;
    Ok(ExactOracleSummary {
        algorithm: "exhaustive_machine_assignment_and_machine_sequence_enumeration".into(),
        setup_semantics: "anticipatory setup; start=max(job_ready, machine_predecessor_completion+transition_setup); first operation on a machine has zero setup".into(),
        candidate_assignment_sequence_combinations: accumulator
            .candidate_assignment_sequence_combinations,
        feasible_acyclic_schedules: accumulator.feasible_acyclic_schedules,
        optimal_makespan: best.makespan,
        optimal_machine_assignment_upstream_ids: best
            .machine_ids_zero_based
            .iter()
            .map(|id| id + 1)
            .collect(),
        optimal_machine_sequences_operation_ids: best.machine_sequences,
        optimal_schedule: best.schedule,
        exhaustive_for_declared_instance: true,
    })
}

fn enumerate_assignments(
    operation_index: usize,
    parsed: &ParsedInstance,
    machine_ids: &mut [usize],
    processing_times: &mut [u64],
    accumulator: &mut OracleAccumulator,
) -> Result<(), String> {
    if operation_index == parsed.operations.len() {
        let mut operations_by_machine = vec![Vec::new(); parsed.machines];
        for (operation_id, machine_id) in machine_ids.iter().copied().enumerate() {
            let Some(operations) = operations_by_machine.get_mut(machine_id) else {
                return Err("assignment_selected_machine_out_of_range".into());
            };
            operations.push(operation_id);
        }
        let permutations_by_machine: Vec<Vec<Vec<usize>>> = operations_by_machine
            .iter()
            .map(|operations| all_permutations(operations))
            .collect();
        let mut sequences = vec![Vec::new(); parsed.machines];
        return enumerate_machine_sequences(
            0,
            &permutations_by_machine,
            &mut sequences,
            parsed,
            machine_ids,
            processing_times,
            accumulator,
        );
    }

    let operation = &parsed.operations[operation_index];
    if operation.machine_options.is_empty() {
        return Err(format!("operation_{operation_index}_has_no_machine_options"));
    }
    for option in &operation.machine_options {
        let machine_id = option
            .machine_id_upstream
            .checked_sub(1)
            .ok_or_else(|| "machine_id_zero_is_invalid".to_string())?;
        if machine_id >= parsed.machines || option.processing_time == 0 {
            return Err(format!("invalid_machine_option_for_operation_{operation_index}"));
        }
        machine_ids[operation_index] = machine_id;
        processing_times[operation_index] = option.processing_time;
        enumerate_assignments(
            operation_index + 1,
            parsed,
            machine_ids,
            processing_times,
            accumulator,
        )?;
    }
    Ok(())
}

fn all_permutations(values: &[usize]) -> Vec<Vec<usize>> {
    let mut working = values.to_vec();
    let mut output = Vec::new();
    enumerate_permutations(&mut working, 0, &mut output);
    output
}

fn enumerate_permutations(working: &mut [usize], position: usize, output: &mut Vec<Vec<usize>>) {
    if position == working.len() {
        output.push(working.to_vec());
        return;
    }
    for next in position..working.len() {
        working.swap(position, next);
        enumerate_permutations(working, position + 1, output);
        working.swap(position, next);
    }
}

fn enumerate_machine_sequences(
    machine_index: usize,
    permutations_by_machine: &[Vec<Vec<usize>>],
    sequences: &mut [Vec<usize>],
    parsed: &ParsedInstance,
    machine_ids: &[usize],
    processing_times: &[u64],
    accumulator: &mut OracleAccumulator,
) -> Result<(), String> {
    if machine_index == permutations_by_machine.len() {
        accumulator.candidate_assignment_sequence_combinations += 1;
        if let Some(candidate) = evaluate_machine_sequences(
            parsed,
            machine_ids,
            processing_times,
            sequences,
        ) {
            accumulator.feasible_acyclic_schedules += 1;
            if accumulator
                .best
                .as_ref()
                .is_none_or(|best| candidate_is_better(&candidate, best))
            {
                accumulator.best = Some(candidate);
            }
        }
        return Ok(());
    }
    for sequence in &permutations_by_machine[machine_index] {
        sequences[machine_index] = sequence.clone();
        enumerate_machine_sequences(
            machine_index + 1,
            permutations_by_machine,
            sequences,
            parsed,
            machine_ids,
            processing_times,
            accumulator,
        )?;
    }
    Ok(())
}

fn candidate_is_better(candidate: &OracleCandidate, incumbent: &OracleCandidate) -> bool {
    candidate.makespan < incumbent.makespan
        || (candidate.makespan == incumbent.makespan
            && (candidate.machine_ids_zero_based < incumbent.machine_ids_zero_based
                || (candidate.machine_ids_zero_based == incumbent.machine_ids_zero_based
                    && candidate.machine_sequences < incumbent.machine_sequences)))
}

fn earliest_processing_start(
    job_predecessor_completion: Option<u64>,
    machine_predecessor_completion: Option<u64>,
    transition_setup: u64,
) -> Option<u64> {
    let job_ready = job_predecessor_completion.unwrap_or(0);
    let machine_ready = match machine_predecessor_completion {
        Some(completion) => completion.checked_add(transition_setup)?,
        None if transition_setup == 0 => 0,
        None => return None,
    };
    Some(job_ready.max(machine_ready))
}

fn evaluate_machine_sequences(
    parsed: &ParsedInstance,
    machine_ids: &[usize],
    processing_times: &[u64],
    machine_sequences: &[Vec<usize>],
) -> Option<OracleCandidate> {
    let operation_count = parsed.operations.len();
    let mut machine_predecessor = vec![None; operation_count];
    let mut job_predecessor = vec![None; operation_count];
    let mut setup_before = vec![0u64; operation_count];
    let mut successors = vec![Vec::new(); operation_count];
    let mut indegree = vec![0usize; operation_count];

    for (machine_id, sequence) in machine_sequences.iter().enumerate() {
        for pair in sequence.windows(2) {
            let previous = pair[0];
            let current = pair[1];
            if machine_ids.get(previous).copied() != Some(machine_id)
                || machine_ids.get(current).copied() != Some(machine_id)
                || machine_predecessor[current].replace(previous).is_some()
            {
                return None;
            }
            setup_before[current] = *parsed
                .setup_times
                .get(machine_id)?
                .get(previous)?
                .get(current)?;
            successors[previous].push(current);
            indegree[current] += 1;
        }
    }

    for operation in &parsed.operations {
        if operation.position_in_job == 0 {
            continue;
        }
        let predecessor = parsed.operations.iter().find(|candidate| {
            candidate.job_id == operation.job_id
                && candidate.position_in_job + 1 == operation.position_in_job
        })?;
        let current = operation.operation_id;
        let previous = predecessor.operation_id;
        job_predecessor[current] = Some(previous);
        if machine_predecessor[current] != Some(previous) {
            successors[previous].push(current);
            indegree[current] += 1;
        }
    }

    let mut ready = VecDeque::new();
    for (operation_id, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            ready.push_back(operation_id);
        }
    }
    let mut completion_times = vec![0u64; operation_count];
    let mut schedule_by_operation: Vec<Option<OperationSchedule>> =
        vec![None; operation_count];
    let mut visited = 0usize;

    while let Some(operation_id) = ready.pop_front() {
        let job_completion =
            job_predecessor[operation_id].map(|predecessor| completion_times[predecessor]);
        let machine_completion =
            machine_predecessor[operation_id].map(|predecessor| completion_times[predecessor]);
        let processing_start = earliest_processing_start(
            job_completion,
            machine_completion,
            setup_before[operation_id],
        )?;
        let completion_time = processing_start.checked_add(processing_times[operation_id])?;
        completion_times[operation_id] = completion_time;
        schedule_by_operation[operation_id] = Some(OperationSchedule {
            operation_id,
            job_id: parsed.operations[operation_id].job_id,
            position_in_job: parsed.operations[operation_id].position_in_job,
            machine_id_upstream: machine_ids[operation_id] + 1,
            setup_time_before_processing: setup_before[operation_id],
            processing_time: processing_times[operation_id],
            processing_start,
            completion_time,
        });
        visited += 1;
        for next in &successors[operation_id] {
            indegree[*next] = indegree[*next].checked_sub(1)?;
            if indegree[*next] == 0 {
                ready.push_back(*next);
            }
        }
    }

    if visited != operation_count {
        return None;
    }
    let schedule = schedule_by_operation.into_iter().collect::<Option<Vec<_>>>()?;
    let makespan = schedule
        .iter()
        .map(|operation| operation.completion_time)
        .max()?;
    Some(OracleCandidate {
        makespan,
        machine_ids_zero_based: machine_ids.to_vec(),
        machine_sequences: machine_sequences.to_vec(),
        schedule,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        serde_json::from_str(include_str!(
            "../fixtures/external_manifest_fattahi_setup_01_v0_2.json"
        ))
        .expect("case 01 manifest fixture parses")
    }

    fn manifest_case_02() -> Manifest {
        serde_json::from_str(include_str!(
            "../fixtures/external_manifest_fattahi_setup_02_v0_2.json"
        ))
        .expect("case 02 manifest fixture parses")
    }

    fn fixture() -> &'static str {
        include_str!("../fixtures/fattahi_setup_01.fjs")
    }

    fn fixture_case_02() -> &'static str {
        include_str!("../fixtures/fattahi_setup_02.fjs")
    }

    fn manifest_case_03() -> Manifest {
        serde_json::from_str(include_str!(
            "../fixtures/external_manifest_fattahi_setup_03_v0_2.json"
        ))
        .expect("case 03 manifest fixture parses")
    }

    fn fixture_case_03() -> &'static str {
        include_str!("../fixtures/fattahi_setup_03.fjs")
    }

    #[test]
    fn pinned_fattahi_fixture_matches_declared_structure() {
        let manifest = manifest();
        validate_manifest(&manifest).expect("manifest pin is valid");
        let parsed = parse_instance(fixture()).expect("upstream fixture parses");
        validate_expected_structure(&manifest.expected_structure, &parsed)
            .expect("declared dimensions match");
        assert_eq!(parsed.jobs, 2);
        assert_eq!(parsed.machines, 2);
        assert_eq!(parsed.operations.len(), 4);
        assert_eq!(parsed.operation_alternative_counts, vec![2, 2, 2, 2]);
        assert_eq!(parsed.declared_average_machine_flexibility, 2.0);
        assert_eq!(parsed.setup_matrix_dimensions, [2, 4, 4]);
        assert_eq!((parsed.setup_values_min, parsed.setup_values_max), (3, 8));
        assert_eq!(parsed.operations[0].machine_options[0].machine_id_upstream, 1);
        assert_eq!(parsed.operations[0].machine_options[0].processing_time, 25);
    }

    #[test]
    fn case_02_decimal_header_is_average_machine_flexibility() {
        let manifest = manifest_case_02();
        validate_manifest(&manifest).expect("case 02 manifest pin is valid");
        let parsed = parse_instance(fixture_case_02()).expect("case 02 fixture parses");
        validate_expected_structure(&manifest.expected_structure, &parsed)
            .expect("case 02 contract matches the fixture");
        assert_eq!(parsed.jobs, 2);
        assert_eq!(parsed.machines, 2);
        assert_eq!(parsed.max_operations_per_job, 2);
        assert_eq!(parsed.operations.len(), 4);
        assert_eq!(parsed.operation_alternative_counts, vec![1, 2, 2, 1]);
        assert_eq!(parsed.declared_average_machine_flexibility, 1.5);
        assert_eq!(parsed.setup_matrix_dimensions, [2, 4, 4]);
        assert_eq!((parsed.setup_values_min, parsed.setup_values_max), (3, 1_000_000));
    }

    #[test]
    fn exact_oracle_finds_case_02_makespan() {
        let parsed = parse_instance(fixture_case_02()).expect("case 02 fixture parses");
        let result = solve_tiny_instance(&parsed).expect("case 02 exact oracle completes");
        assert_eq!(result.candidate_assignment_sequence_combinations, 20);
        assert_eq!(result.feasible_acyclic_schedules, 11);
        assert_eq!(result.optimal_makespan, 112);
        assert_eq!(result.optimal_machine_assignment_upstream_ids, vec![1, 1, 2, 2]);
        assert_eq!(
            result.optimal_machine_sequences_operation_ids,
            vec![vec![0, 1], vec![2, 3]]
        );
        assert_eq!(
            result.optimal_schedule.iter()
                .map(|op| (op.operation_id, op.processing_start, op.completion_time))
                .collect::<Vec<_>>(),
            vec![(0, 0, 43), (1, 48, 112), (2, 0, 35), (3, 44, 87)]
        );
        assert!(result.exhaustive_for_declared_instance);
    }

    #[test]
    fn case_03_flexibility_metadata_and_six_operation_oracle_are_exact() {
        let manifest = manifest_case_03();
        validate_manifest(&manifest).expect("case 03 manifest pin is valid");
        let parsed = parse_instance(fixture_case_03()).expect("case 03 fixture parses");
        validate_expected_structure(&manifest.expected_structure, &parsed)
            .expect("case 03 contract matches the fixture");
        assert_eq!(parsed.jobs, 3);
        assert_eq!(parsed.machines, 2);
        assert_eq!(parsed.max_operations_per_job, 2);
        assert_eq!(parsed.operations.len(), 6);
        assert_eq!(parsed.operation_alternative_counts, vec![1, 2, 2, 1, 2, 2]);
        assert_eq!(parsed.declared_average_machine_flexibility, 1.7);
        assert_eq!(parsed.setup_matrix_dimensions, [2, 6, 6]);
        assert_eq!((parsed.setup_values_min, parsed.setup_values_max), (6, 1_000_000));

        let result = solve_tiny_instance(&parsed).expect("case 03 exact oracle completes");
        assert_eq!(result.candidate_assignment_sequence_combinations, 840);
        assert_eq!(result.feasible_acyclic_schedules, 298);
        assert_eq!(result.optimal_makespan, 233);
        assert_eq!(
            result.optimal_machine_assignment_upstream_ids,
            vec![1, 2, 2, 2, 1, 1]
        );
        assert_eq!(
            result.optimal_machine_sequences_operation_ids,
            vec![vec![0, 4, 5], vec![2, 1, 3]]
        );
        assert_eq!(
            result.optimal_schedule.iter()
                .map(|op| (op.operation_id, op.processing_start, op.completion_time))
                .collect::<Vec<_>>(),
            vec![
                (0, 0, 43),
                (1, 59, 154),
                (2, 0, 53),
                (3, 160, 233),
                (4, 51, 176),
                (5, 185, 228),
            ]
        );
    }

    #[test]
    fn header_average_flexibility_must_match_observed_alternatives() {
        let input = fixture_case_02().replacen("2 2 1.5", "2 2 1.7", 1);
        assert_eq!(
            parse_instance(&input).unwrap_err(),
            "declared_average_machine_flexibility_mismatch"
        );
    }

    #[test]
    fn legacy_manifest_schema_is_rejected() {
        assert!(serde_json::from_str::<Manifest>(include_str!(
            "../fixtures/external_manifest_v0_1.json"
        ))
        .is_err());
    }

    #[test]
    fn setup_can_overlap_job_wait_under_anticipatory_reference_semantics() {
        assert_eq!(earliest_processing_start(Some(10), Some(3), 5), Some(10));
        assert_eq!(earliest_processing_start(Some(4), Some(10), 5), Some(15));
        assert_eq!(earliest_processing_start(Some(7), None, 0), Some(7));
        assert_eq!(earliest_processing_start(Some(7), None, 1), None);
    }

    #[test]
    fn exact_oracle_exhaustively_finds_hand_checkable_makespan() {
        let parsed = parse_instance(fixture()).expect("fixture parses");
        let result = solve_tiny_instance(&parsed).expect("exact oracle completes");
        assert_eq!(result.candidate_assignment_sequence_combinations, 120);
        assert_eq!(result.feasible_acyclic_schedules, 52);
        assert_eq!(result.optimal_makespan, 70);
        assert_eq!(result.optimal_machine_assignment_upstream_ids, vec![2, 2, 1, 1]);
        assert_eq!(
            result.optimal_machine_sequences_operation_ids,
            vec![vec![2, 3], vec![0, 1]]
        );
        let schedule_times = result
            .optimal_schedule
            .iter()
            .map(|operation| {
                (
                    operation.operation_id,
                    operation.processing_start,
                    operation.completion_time,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            schedule_times,
            vec![(0, 0, 37), (1, 40, 64), (2, 0, 45), (3, 49, 70)]
        );
        assert!(result.exhaustive_for_declared_instance);
        assert_eq!(result.optimal_schedule.len(), 4);

        let schedule = &result.optimal_schedule;
        for operation in schedule {
            let eligible = &parsed.operations[operation.operation_id].machine_options;
            assert!(eligible.iter().any(|choice| {
                choice.machine_id_upstream == operation.machine_id_upstream
                    && choice.processing_time == operation.processing_time
            }));
            assert_eq!(
                operation.completion_time,
                operation.processing_start + operation.processing_time
            );
            if operation.position_in_job > 0 {
                let predecessor = schedule
                    .iter()
                    .find(|candidate| {
                        candidate.job_id == operation.job_id
                            && candidate.position_in_job + 1 == operation.position_in_job
                    })
                    .expect("precedence predecessor exists");
                assert!(operation.processing_start >= predecessor.completion_time);
            }
        }
        for (machine_index, sequence) in result
            .optimal_machine_sequences_operation_ids
            .iter()
            .enumerate()
        {
            for pair in sequence.windows(2) {
                let previous = schedule
                    .iter()
                    .find(|item| item.operation_id == pair[0])
                    .expect("previous operation exists");
                let current = schedule
                    .iter()
                    .find(|item| item.operation_id == pair[1])
                    .expect("current operation exists");
                let transition_setup = parsed.setup_times[machine_index][pair[0]][pair[1]];
                assert_eq!(current.setup_time_before_processing, transition_setup);
                assert!(current.processing_start >= previous.completion_time + transition_setup);
            }
            if let Some(&first) = sequence.first() {
                let first_operation = schedule
                    .iter()
                    .find(|item| item.operation_id == first)
                    .expect("first operation exists");
                assert_eq!(first_operation.setup_time_before_processing, 0);
            }
        }
    }

    #[test]
    fn wrong_source_revision_fails_closed() {
        let mut manifest = manifest();
        manifest.source.commit = "0000000000000000000000000000000000000000".into();
        assert_eq!(
            validate_manifest(&manifest).unwrap_err(),
            "upstream_source_pin_mismatch"
        );
    }

    #[test]
    fn manifest_cannot_smuggle_due_date_claims_into_setup_only_instance() {
        let mut manifest = manifest();
        manifest.expected_structure.supports_due_dates = true;
        assert_eq!(
            validate_manifest(&manifest).unwrap_err(),
            "expected_structure_contract_mismatch"
        );
    }

    #[test]
    fn missing_setup_rows_fail_closed() {
        let input = fixture().split("\n\n").next().expect("fixture header and jobs");
        assert!(parse_instance(input).is_err());
    }

    #[test]
    fn setup_matrix_width_mismatch_is_rejected() {
        let input = fixture().replacen("6 3 4 4 ", "6 3 4 ", 1);
        assert!(parse_instance(&input).is_err());
    }

    #[test]
    fn duplicate_machine_choice_is_rejected() {
        let input = fixture().replacen("2 2 1 25 2 37", "2 2 1 25 1 37", 1);
        assert!(parse_instance(&input).is_err());
    }

    #[test]
    fn zero_processing_time_is_rejected() {
        let input = fixture().replacen("2 2 1 25 2 37", "2 2 1 0 2 37", 1);
        assert!(parse_instance(&input).is_err());
    }
}
