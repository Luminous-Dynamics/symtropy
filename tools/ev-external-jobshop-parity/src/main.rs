use serde::{Deserialize, Serialize};
use std::{env, fs, process};

const UPSTREAM_COMMIT: &str = "d088a26582f5cb79c6b4f2ad94f68ac7c50878cb";
const UPSTREAM_BLOB_SHA1: &str = "6bac8cfaf9f326f6405d5396b9cedd12941aba94";
const UPSTREAM_REPOSITORY: &str =
    "https://github.com/ai-for-decision-making-tue/Job_Shop_Scheduling_Benchmark_Environments_and_Instances";
const UPSTREAM_INSTANCE_PATH: &str = "data/fjsp_sdst/fattahi/Fattahi_setup_01.fjs";
const CLAIM_CEILING: &str = "Pinned public benchmark input structure validated only. No dispatching policy was executed and no scheduling performance, due-date result, SOTA comparison, or production claim is established.";

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
    setup_matrix_dimensions: [usize; 3],
    setup_values_min: i64,
    setup_values_max: i64,
    primary_objective: String,
    supports_due_dates: bool,
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
    setup_matrix_dimensions: [usize; 3],
    setup_values_min: i64,
    setup_values_max: i64,
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

    let report = Report {
        schema_version: "external-jobshop-input-parity/0.1".into(),
        status: "input_structure_validated_not_policy_benchmarked".into(),
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
        setup_matrix_dimensions: parsed.setup_matrix_dimensions,
        setup_values_min: parsed.setup_values_min,
        setup_values_max: parsed.setup_values_max,
        primary_objective: manifest.expected_structure.primary_objective,
        supports_due_dates: manifest.expected_structure.supports_due_dates,
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
    if manifest.schema_version != "external-jobshop-parity-manifest/0.1"
        || manifest.benchmark_id != "FJSP-SDST-FATTAHI-001"
        || manifest.claim_ceiling != CLAIM_CEILING
    {
        return Err("unsupported_manifest_version_id_or_claim_ceiling".into());
    }
    let source = &manifest.source;
    if source.repository_url != UPSTREAM_REPOSITORY
        || source.commit != UPSTREAM_COMMIT
        || source.license != "MIT"
        || source.paper_doi != "10.1007/s10472-026-10007-3"
        || source.instance_path != UPSTREAM_INSTANCE_PATH
        || source.instance_git_blob_sha1 != UPSTREAM_BLOB_SHA1
    {
        return Err("upstream_source_pin_mismatch".into());
    }
    if !is_lower_hex_40(&source.commit) || !is_lower_hex_40(&source.instance_git_blob_sha1) {
        return Err("upstream_revision_or_blob_digest_malformed".into());
    }
    let expected = &manifest.expected_structure;
    if expected.jobs != 2
        || expected.machines != 2
        || expected.max_operations_per_job != 2
        || expected.operations != 4
        || expected.operation_alternative_counts != vec![2, 2, 2, 2]
        || expected.setup_matrix_dimensions != [2, 4, 4]
        || expected.setup_values_min != 3
        || expected.setup_values_max != 8
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
    let header = parse_usize_tokens(lines[0], "header")?;
    if header.len() != 3 || header.iter().any(|value| *value == 0) {
        return Err("header_must_contain_positive_job_machine_and_operation_counts".into());
    }
    let jobs = header[0];
    let machines = header[1];
    let declared_max_operations = header[2];
    if jobs > 100_000 || machines > 10_000 || declared_max_operations > 100_000 {
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
        if operation_count == 0 || operation_count > declared_max_operations {
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
    for (row_index, row) in setup_rows.iter().enumerate() {
        let values = parse_i64_tokens(row, &format!("setup_row_{row_index}"))?;
        if values.len() != operation_count {
            return Err(format!(
                "setup_row_width_mismatch_row_{row_index}_expected_{operation_count}_observed_{}",
                values.len()
            ));
        }
        for value in values {
            if value < 0 {
                return Err(format!("negative_setup_time_row_{row_index}"));
            }
            setup_min = Some(setup_min.map_or(value, |current| current.min(value)));
            setup_max = Some(setup_max.map_or(value, |current| current.max(value)));
        }
    }

    Ok(ParsedInstance {
        jobs,
        machines,
        max_operations_per_job: max_observed_operations,
        operation_alternative_counts,
        operations,
        setup_matrix_dimensions: [machines, operation_count, operation_count],
        setup_values_min: setup_min.ok_or_else(|| "setup_matrix_has_no_values".to_string())?,
        setup_values_max: setup_max.ok_or_else(|| "setup_matrix_has_no_values".to_string())?,
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
        || parsed.setup_matrix_dimensions != expected.setup_matrix_dimensions
        || parsed.setup_values_min != expected.setup_values_min
        || parsed.setup_values_max != expected.setup_values_max
    {
        return Err("observed_input_structure_differs_from_frozen_manifest".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        serde_json::from_str(include_str!("../fixtures/external_manifest_v0_1.json"))
            .expect("manifest fixture parses")
    }

    fn fixture() -> &'static str {
        include_str!("../fixtures/fattahi_setup_01.fjs")
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
        assert_eq!(parsed.setup_matrix_dimensions, [2, 4, 4]);
        assert_eq!((parsed.setup_values_min, parsed.setup_values_max), (3, 8));
        assert_eq!(parsed.operations[0].machine_options[0].machine_id_upstream, 1);
        assert_eq!(parsed.operations[0].machine_options[0].processing_time, 25);
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
