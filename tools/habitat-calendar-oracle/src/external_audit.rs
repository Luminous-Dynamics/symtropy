// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const REPORT_SCHEMA: &str = "luminous.ca_rcpsp_external_audit_report.v0.1";
pub const EXPECTED_INSTANCE_COUNT: usize = 12;
pub const EXPECTED_RESULT_ROW_COUNT: usize = 36;
pub const EXPECTED_SUCCESS_ROW_COUNT: usize = 30;
pub const EXPECTED_TIMEOUT_ROW_COUNT: usize = 6;
pub const EXPECTED_LOWER_BOUND_VIOLATION_COUNT: usize = 30;
pub const EXPECTED_NONFULL_CALENDAR_COMPLIANCE_COUNT: usize = 18;
const EXPECTED_SOLVER_IDS: &[&str] = &["maxplus", "ortools", "gurobi"];

const CLAIM_CEILING: &str =
    "source_data_lower_bound_audit_not_solver_ranking_or_project_estimate";
const CSV_PATH: &str =
    "docs/research/external/ca-rcpsp-v1.0/results/per_scenario_results.csv";
const CSV_SHA256: &str =
    "834ad0cf18b8062497d2a0a5387524c340145fd164d6a2071982d83688555ce1";

struct InstanceInput {
    path: &'static str,
    bytes: &'static [u8],
    expected_sha256: &'static str,
}

const INSTANCE_INPUTS: &[InstanceInput] = &[
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/tiny/tiny_manufacturing_demo.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/tiny/tiny_manufacturing_demo.json"),
        expected_sha256: "11906038ee3ac9e3979f6987523cf01d6e4ab56c488a684de44fbd1c54c06ed3",
    },
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/tiny/tiny_assembly_demo.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/tiny/tiny_assembly_demo.json"),
        expected_sha256: "9a480474a2af9d40cb2ea2c03046056421db86c6ce4b687fab664ace64b2043c",
    },
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/tiny/tiny_quality_demo.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/tiny/tiny_quality_demo.json"),
        expected_sha256: "22ca5908ae6ff41b166a9cf944b3f1a25ec141417af4878ec2e42881fe214363",
    },
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/small/small_disrupted_manufacturing_25.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/small/small_disrupted_manufacturing_25.json"),
        expected_sha256: "23d9c83bba9519ebdde39ef53d8f9745e355644238bee910aeace498dc3e4c8f",
    },
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/small/small_multi_resource_35.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/small/small_multi_resource_35.json"),
        expected_sha256: "910747c895761d350cddbb143659b251945d1cc5903de88a583abb56d6474495",
    },
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/small/small_shift_operations_40.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/small/small_shift_operations_40.json"),
        expected_sha256: "16cc51fca4ed158dd5f745c160d716bb9a0d0fd91dc1786ac1496b41204b3350",
    },
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/medium/medium_complex_manufacturing_120.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/medium/medium_complex_manufacturing_120.json"),
        expected_sha256: "badfaf46fe9e48cbed5f3fd1a85dc5eee31b0b016c6798013dd1ee6008484e18",
    },
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/medium/medium_multi_shift_factory_150.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/medium/medium_multi_shift_factory_150.json"),
        expected_sha256: "1fcc43e6c15ba4b44b79c9dc3039f5f9dde0e2e6d6775ada94c4e7bac63784fc",
    },
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/medium/medium_supply_chain_180.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/medium/medium_supply_chain_180.json"),
        expected_sha256: "d7dab8a80f61b6c98662f82af4870c3419fe0dcc710052c21dc8dd3869258928",
    },
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/large/large_industrial_scale_500.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/large/large_industrial_scale_500.json"),
        expected_sha256: "67b290d07f5795e40f1e89a7a0b7c28be3da0e9276b25cd2e8ea880f87b338cc",
    },
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/large/large_enterprise_manufacturing_750.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/large/large_enterprise_manufacturing_750.json"),
        expected_sha256: "e7881ea9405d62f9380e7a9b0928865038bcd988bc4178b8538d1ed53a284509",
    },
    InstanceInput {
        path: "docs/research/external/ca-rcpsp-v1.0/scenarios/large/large_mega_scale_factory_1000.json",
        bytes: include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/scenarios/large/large_mega_scale_factory_1000.json"),
        expected_sha256: "2e8279aecf4f5e60b95d966a07055c71676474c681d5a3c192fbe25edb065aa5",
    },
];

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct InstanceBound {
    pub scenario_id: String,
    pub source_path: String,
    pub source_sha256: String,
    pub task_count: usize,
    pub longest_task_duration_hours: f64,
    pub declared_expected_makespan_hours: f64,
    pub expected_makespan_respects_longest_task_bound: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SolverRowAudit {
    pub csv_record_number: usize,
    pub scenario_id: String,
    pub solver: String,
    pub success: bool,
    pub makespan_hours: Option<f64>,
    pub solve_time_seconds: Option<f64>,
    pub calendar_compliance: Option<f64>,
    pub resource_utilization: Option<f64>,
    pub longest_task_duration_hours: f64,
    pub reported_makespan_below_longest_task_bound: bool,
    pub source_result_status: String,
    pub notes: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ExternalAuditReport {
    pub schema_version: String,
    pub audit_status: String,
    pub source_dataset: String,
    pub source_data_doi: String,
    pub source_results_csv_sha256: String,
    pub source_artifacts_match_pinned_sha256: bool,
    pub instance_count: usize,
    pub total_task_count: usize,
    pub solver_result_row_count: usize,
    pub successful_solver_row_count: usize,
    pub timeout_or_failed_row_count: usize,
    pub successful_rows_below_longest_task_bound: usize,
    pub expected_performance_makespan_below_longest_task_bound_count: usize,
    pub successful_rows_with_calendar_compliance_below_one: usize,
    pub all_scenario_ids_joined_exactly_once_per_solver: bool,
    pub all_successful_rows_have_makespan: bool,
    pub all_timeout_rows_have_empty_makespan: bool,
    pub metric_semantics_require_source_reconciliation: bool,
    pub claim_ceiling: String,
    pub instance_bounds: Vec<InstanceBound>,
    pub solver_rows: Vec<SolverRowAudit>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn json_number(value: Option<&Value>, path: &str) -> Result<f64, String> {
    value
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("missing_or_non_numeric:{path}"))
}

fn parse_optional_number(value: &str, label: &str) -> Result<Option<f64>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parsed = trimmed
        .parse::<f64>()
        .map_err(|_| format!("invalid_number:{label}"))?;
    if !parsed.is_finite() || parsed < 0.0 {
        return Err(format!("out_of_range_number:{label}"));
    }
    Ok(Some(parsed))
}

fn parse_instance(input: &InstanceInput) -> Result<InstanceBound, String> {
    let actual_sha = sha256_hex(input.bytes);
    if actual_sha != input.expected_sha256 {
        return Err(format!("source_sha256_mismatch:{}", input.path));
    }
    let root: Value = serde_json::from_slice(input.bytes)
        .map_err(|error| format!("invalid_source_json:{}:{error}", input.path))?;
    let scenario_id = root
        .get("scenario_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing_scenario_id:{}", input.path))?
        .to_owned();
    let tasks = root
        .get("tasks")
        .and_then(Value::as_array)
        .filter(|tasks| !tasks.is_empty())
        .ok_or_else(|| format!("missing_tasks:{}", input.path))?;

    let mut task_ids = BTreeSet::new();
    let mut longest = 0.0_f64;
    for task in tasks {
        let task_id = task
            .get("task_id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("missing_task_id:{scenario_id}"))?;
        if !task_ids.insert(task_id) {
            return Err(format!("duplicate_task_id:{scenario_id}:{task_id}"));
        }
        let duration = json_number(task.get("duration"), "tasks[].duration")?;
        if duration <= 0.0 {
            return Err(format!("non_positive_task_duration:{scenario_id}:{task_id}"));
        }
        longest = longest.max(duration);
    }

    if let Some(metadata_count) = root
        .get("metadata")
        .and_then(|metadata| metadata.get("num_tasks"))
        .and_then(Value::as_u64)
    {
        if metadata_count as usize != tasks.len() {
            return Err(format!("task_count_metadata_mismatch:{scenario_id}"));
        }
    } else {
        return Err(format!("missing_task_count_metadata:{scenario_id}"));
    }

    let expected_makespan = json_number(
        root.get("expected_performance")
            .and_then(|expected| expected.get("makespan_hours")),
        "expected_performance.makespan_hours",
    )?;
    Ok(InstanceBound {
        scenario_id,
        source_path: input.path.to_owned(),
        source_sha256: actual_sha,
        task_count: tasks.len(),
        longest_task_duration_hours: longest,
        declared_expected_makespan_hours: expected_makespan,
        expected_makespan_respects_longest_task_bound: expected_makespan + 1e-9 >= longest,
    })
}

fn parse_results_csv(
    bytes: &[u8],
    longest_by_scenario: &BTreeMap<String, f64>,
) -> Result<Vec<SolverRowAudit>, String> {
    let actual_sha = sha256_hex(bytes);
    if actual_sha != CSV_SHA256 {
        return Err("source_sha256_mismatch:per_scenario_results.csv".to_owned());
    }
    let csv = std::str::from_utf8(bytes)
        .map_err(|error| format!("results_csv_not_utf8:{error}"))?;
    let mut lines = csv.lines();
    let header = lines
        .next()
        .ok_or_else(|| "results_csv_is_empty".to_owned())?;
    let expected_header = [
        "scenario",
        "solver",
        "makespan_hours",
        "solve_time_s",
        "success",
        "calendar_compliance",
        "resource_utilization",
        "notes",
    ];
    if header.trim_end_matches('\r').split(',').collect::<Vec<_>>() != expected_header.to_vec() {
        return Err("unsupported_results_csv_header".to_owned());
    }

    let mut output = Vec::new();
    for (index, raw_line) in lines.enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        // The pinned upstream artifact uses a simple, unquoted eight-column CSV.
        // Fail closed if future upstream bytes introduce quoting or embedded commas.
        if line.contains('"') {
            return Err(format!("unsupported_quoted_csv_record:{}", index + 2));
        }
        let columns: Vec<&str> = line.split(',').collect();
        if columns.len() != 8 {
            return Err(format!("wrong_results_column_count:{}", index + 2));
        }
        let scenario = columns[0].trim();
        let solver = columns[1].trim();
        if scenario.is_empty() || solver.is_empty() {
            return Err(format!("missing_scenario_or_solver:{}", index + 2));
        }
        let longest = *longest_by_scenario
            .get(scenario)
            .ok_or_else(|| format!("unknown_result_scenario:{scenario}"))?;
        let success = match columns[4].trim() {
            "True" => true,
            "False" => false,
            other => return Err(format!("unsupported_success_value:{other}")),
        };
        let makespan = parse_optional_number(columns[2], "makespan_hours")?;
        let solve_time = parse_optional_number(columns[3], "solve_time_s")?;
        let calendar_compliance =
            parse_optional_number(columns[5], "calendar_compliance")?;
        let resource_utilization =
            parse_optional_number(columns[6], "resource_utilization")?;
        if success && (makespan.is_none() || calendar_compliance.is_none()) {
            return Err(format!("successful_row_missing_required_metric:{}", index + 2));
        }
        if !success && makespan.is_some() {
            return Err(format!("timeout_row_has_makespan:{}", index + 2));
        }
        if calendar_compliance.is_some_and(|value| value > 1.0) {
            return Err(format!("calendar_compliance_exceeds_one:{}", index + 2));
        }
        if resource_utilization.is_some_and(|value| value > 1.0) {
            return Err(format!("resource_utilization_exceeds_one:{}", index + 2));
        }

        let violation = success && makespan.is_some_and(|value| value + 1e-9 < longest);
        output.push(SolverRowAudit {
            csv_record_number: index + 2,
            scenario_id: scenario.to_owned(),
            solver: solver.to_owned(),
            success,
            makespan_hours: makespan,
            solve_time_seconds: solve_time,
            calendar_compliance,
            resource_utilization,
            longest_task_duration_hours: longest,
            reported_makespan_below_longest_task_bound: violation,
            source_result_status: if success {
                "SOURCE_MARKED_SUCCESS".to_owned()
            } else {
                "SOURCE_MARKED_FAILED_OR_TIMEOUT".to_owned()
            },
            notes: columns[7].trim().to_owned(),
        });
    }
    Ok(output)
}

/// Audits the pinned upstream source corpus. A reproduced metric inconsistency is a
/// report finding, not a command failure or a solver-ranking claim.
pub fn audit_embedded_external_corpus() -> Result<ExternalAuditReport, String> {
    let mut instance_bounds = Vec::with_capacity(INSTANCE_INPUTS.len());
    let mut by_scenario: BTreeMap<String, f64> = BTreeMap::new();
    for input in INSTANCE_INPUTS {
        let bound = parse_instance(input)?;
        let scenario_id = bound.scenario_id.clone();
        if by_scenario
            .insert(scenario_id.clone(), bound.longest_task_duration_hours)
            .is_some()
        {
            return Err(format!("duplicate_source_scenario_id:{scenario_id}"));
        }
        instance_bounds.push(bound);
    }

    let solver_rows = parse_results_csv(
        include_bytes!("../../../docs/research/external/ca-rcpsp-v1.0/results/per_scenario_results.csv"),
        &by_scenario,
    )?;

    let mut pairs = BTreeSet::new();
    let mut pair_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for row in &solver_rows {
        if !pairs.insert((row.scenario_id.as_str(), row.solver.as_str())) {
            return Err(format!(
                "duplicate_scenario_solver_pair:{}:{}",
                row.scenario_id, row.solver
            ));
        }
        *pair_counts.entry(row.scenario_id.as_str()).or_default() += 1;
    }
    if by_scenario
        .keys()
        .any(|scenario| pair_counts.get(scenario.as_str()).copied() != Some(3))
        || solver_rows.len() != EXPECTED_RESULT_ROW_COUNT
    {
        return Err("results_csv_does_not_have_exactly_three_rows_per_scenario".to_owned());
    }
    let expected_solver_ids: BTreeSet<&str> = EXPECTED_SOLVER_IDS.iter().copied().collect();
    for scenario in by_scenario.keys() {
        let actual_solver_ids: BTreeSet<&str> = solver_rows
            .iter()
            .filter(|row| row.scenario_id == *scenario)
            .map(|row| row.solver.as_str())
            .collect();
        if actual_solver_ids != expected_solver_ids {
            return Err(format!(
                "unexpected_solver_set_for_scenario:{scenario}"
            ));
        }
    }

    let successful = solver_rows.iter().filter(|row| row.success).count();
    let timeouts = solver_rows.iter().filter(|row| !row.success).count();
    let lower_bound_violations = solver_rows
        .iter()
        .filter(|row| row.reported_makespan_below_longest_task_bound)
        .count();
    let nonfull_compliance = solver_rows
        .iter()
        .filter(|row| row.success && row.calendar_compliance.is_some_and(|value| value < 1.0))
        .count();
    let expected_bound_violations = instance_bounds
        .iter()
        .filter(|bound| !bound.expected_makespan_respects_longest_task_bound)
        .count();
    let all_scenario_ids_joined = solver_rows
        .iter()
        .all(|row| by_scenario.contains_key(row.scenario_id.as_str()));
    let all_successful_rows_have_makespan =
        solver_rows.iter().all(|row| !row.success || row.makespan_hours.is_some());
    let all_timeout_rows_have_empty_makespan =
        solver_rows.iter().all(|row| row.success || row.makespan_hours.is_none());

    Ok(ExternalAuditReport {
        schema_version: REPORT_SCHEMA.to_owned(),
        audit_status: if lower_bound_violations == 0 {
            "COMPLETE_NO_LONGEST_TASK_LOWER_BOUND_DISCREPANCY".to_owned()
        } else {
            "COMPLETE_PUBLISHED_METRIC_LOWER_BOUND_DISCREPANCIES_FOUND".to_owned()
        },
        source_dataset: "CA-RCPSP synthetic benchmark dataset".to_owned(),
        source_data_doi: "10.5281/zenodo.20553817".to_owned(),
        source_results_csv_sha256: sha256_hex(include_bytes!(
            "../../../docs/research/external/ca-rcpsp-v1.0/results/per_scenario_results.csv"
        )),
        source_artifacts_match_pinned_sha256: true,
        instance_count: instance_bounds.len(),
        total_task_count: instance_bounds.iter().map(|instance| instance.task_count).sum(),
        solver_result_row_count: solver_rows.len(),
        successful_solver_row_count: successful,
        timeout_or_failed_row_count: timeouts,
        successful_rows_below_longest_task_bound: lower_bound_violations,
        expected_performance_makespan_below_longest_task_bound_count: expected_bound_violations,
        successful_rows_with_calendar_compliance_below_one: nonfull_compliance,
        all_scenario_ids_joined_exactly_once_per_solver: all_scenario_ids_joined
            && pairs.len() == solver_rows.len(),
        all_successful_rows_have_makespan,
        all_timeout_rows_have_empty_makespan,
        metric_semantics_require_source_reconciliation: lower_bound_violations > 0,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        instance_bounds,
        solver_rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_source_corpus_reproduces_the_lower_bound_audit() {
        let report = audit_embedded_external_corpus().expect("source corpus should audit cleanly");
        assert_eq!(report.instance_count, EXPECTED_INSTANCE_COUNT);
        assert_eq!(report.total_task_count, 2819);
        assert_eq!(report.solver_result_row_count, EXPECTED_RESULT_ROW_COUNT);
        assert_eq!(report.successful_solver_row_count, EXPECTED_SUCCESS_ROW_COUNT);
        assert_eq!(report.timeout_or_failed_row_count, EXPECTED_TIMEOUT_ROW_COUNT);
        assert_eq!(
            report.successful_rows_below_longest_task_bound,
            EXPECTED_LOWER_BOUND_VIOLATION_COUNT
        );
        assert_eq!(
            report.successful_rows_with_calendar_compliance_below_one,
            EXPECTED_NONFULL_CALENDAR_COMPLIANCE_COUNT
        );
        assert_eq!(
            report.expected_performance_makespan_below_longest_task_bound_count,
            0
        );
        assert!(report.source_artifacts_match_pinned_sha256);
        assert!(report.all_scenario_ids_joined_exactly_once_per_solver);
        assert!(report.all_successful_rows_have_makespan);
        assert!(report.all_timeout_rows_have_empty_makespan);
        assert!(report.metric_semantics_require_source_reconciliation);
        assert_eq!(
            report.audit_status,
            "COMPLETE_PUBLISHED_METRIC_LOWER_BOUND_DISCREPANCIES_FOUND"
        );
    }

    #[test]
    fn longest_task_lower_bound_is_not_confused_with_project_expected_makespan() {
        let report = audit_embedded_external_corpus().expect("source corpus should audit cleanly");
        assert!(report
            .instance_bounds
            .iter()
            .all(|bound| bound.declared_expected_makespan_hours
                >= bound.longest_task_duration_hours));
    }

    #[test]
    fn all_successful_result_rows_are_disclosed_not_dropped_from_the_report() {
        let report = audit_embedded_external_corpus().expect("source corpus should audit cleanly");
        assert_eq!(
            report.solver_rows.len(),
            report.successful_solver_row_count + report.timeout_or_failed_row_count
        );
        assert_eq!(
            report
                .solver_rows
                .iter()
                .filter(|row| row.reported_makespan_below_longest_task_bound)
                .count(),
            EXPECTED_LOWER_BOUND_VIOLATION_COUNT
        );
    }
}
