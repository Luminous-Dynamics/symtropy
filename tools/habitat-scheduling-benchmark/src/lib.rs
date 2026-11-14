// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use habitat_scheduling_oracle::evaluate_fixture_bytes as evaluate_oracle_fixture_bytes;
use habitat_scheduling_planner::plan_fixture_bytes;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const REPORT_SCHEMA: &str = "luminous.habitat_schedule_benchmark_report.v0.1";
pub const CLAIM_CEILING: &str =
    "ten_case_synthetic_quality_scorecard_not_general_performance_or_real_world_authority";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ScorecardRow {
    pub case_id: String,
    pub oracle_feasible: bool,
    pub oracle_optimal_makespan: Option<u32>,
    pub priority_only_status: String,
    pub priority_only_makespan: Option<u32>,
    pub priority_only_missed_feasible_candidate: bool,
    pub fallback_status: String,
    pub fallback_states_expanded: usize,
    pub fallback_orders_evaluated: usize,
    pub heuristic_status: String,
    pub heuristic_makespan: Option<u32>,
    pub candidate_independently_validated: bool,
    pub absolute_makespan_gap: Option<u32>,
    pub relative_makespan_gap_ppm: Option<u64>,
    pub heuristic_missed_feasible_candidate: bool,
    pub false_infeasibility_claim: bool,
    pub candidate_conflicts_with_exact_oracle: bool,
    pub optimality_claim: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BenchmarkReport {
    pub schema_version: String,
    pub suite_id: String,
    pub scored_duration_scenario: String,
    pub fixture_sha256: String,
    pub fixture_bytes: usize,
    pub oracle_report_sha256: String,
    pub heuristic_report_sha256: String,
    pub total_case_count: usize,
    pub oracle_feasible_case_count: usize,
    pub priority_only_candidate_count: usize,
    pub priority_only_feasible_case_coverage_ppm: u64,
    pub priority_only_missed_feasible_candidate_count: usize,
    pub bounded_fallback_candidate_count: usize,
    pub bounded_fallback_states_expanded_total: usize,
    pub heuristic_candidate_count: usize,
    pub heuristic_feasible_case_coverage_ppm: u64,
    pub heuristic_missed_feasible_candidate_count: usize,
    pub false_infeasibility_claim_count: usize,
    pub candidate_oracle_conflict_count: usize,
    pub independently_validated_candidate_count: usize,
    pub conditional_mean_relative_gap_ppm: Option<u64>,
    pub conditional_max_relative_gap_ppm: Option<u64>,
    pub gap_metrics_are_conditional_on_candidate_found: bool,
    pub all_exact_reference_expectations_match: bool,
    pub all_emitted_candidates_pass_independent_validation: bool,
    pub no_false_infeasibility_claims: bool,
    pub optimality_claim_count: usize,
    pub claim_ceiling: String,
    pub rows: Vec<ScorecardRow>,
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

/// Runs the exact micro-oracle and heuristic planner separately, then compares their reports.
pub fn evaluate_fixture_bytes(bytes: &[u8]) -> Result<BenchmarkReport, String> {
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("benchmark fixture rejected: {error}"))?;
    let suite_id = root
        .get("suite_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "benchmark fixture has no suite_id".to_owned())?
        .to_owned();

    let oracle = evaluate_oracle_fixture_bytes(bytes)?;
    let heuristic = plan_fixture_bytes(bytes)?;
    if oracle.input_sha256 != sha256_hex(bytes)
        || heuristic.input_sha256 != sha256_hex(bytes)
        || oracle.input_bytes != bytes.len()
        || heuristic.input_bytes != bytes.len()
    {
        return Err(
            "oracle/planner report provenance does not match exact fixture bytes".to_owned(),
        );
    }
    if !oracle.all_expected_outcomes_match {
        return Err(
            "exact oracle disagrees with one or more frozen reference expectations".to_owned(),
        );
    }
    if oracle.case_count != heuristic.case_count || oracle.case_count == 0 {
        return Err("oracle and heuristic reports contain different case counts".to_owned());
    }

    let oracle_by_id: BTreeMap<&str, &habitat_scheduling_oracle::CaseResult> = oracle
        .case_results
        .iter()
        .map(|row| (row.case_id.as_str(), row))
        .collect();
    let heuristic_by_id: BTreeMap<&str, &habitat_scheduling_planner::CaseResult> = heuristic
        .case_results
        .iter()
        .map(|row| (row.case_id.as_str(), row))
        .collect();
    if oracle_by_id.len() != oracle.case_results.len()
        || heuristic_by_id.len() != heuristic.case_results.len()
        || oracle_by_id.keys().collect::<BTreeSet<_>>()
            != heuristic_by_id.keys().collect::<BTreeSet<_>>()
    {
        return Err("oracle and planner case IDs are not a unique one-to-one match".to_owned());
    }

    let mut rows = Vec::with_capacity(oracle.case_count);
    let mut reference_feasible = 0usize;
    let mut priority_only_candidates = 0usize;
    let mut priority_only_feasible_candidates = 0usize;
    let mut priority_only_misses = 0usize;
    let mut fallback_candidates = 0usize;
    let mut fallback_states_total = 0usize;
    let mut candidates = 0usize;
    let mut feasible_candidates = 0usize;
    let mut misses = 0usize;
    let mut false_infeasible = 0usize;
    let mut candidate_conflicts = 0usize;
    let mut validated_candidates = 0usize;
    let mut sum_relative_gap_ppm = 0u128;
    let mut max_relative_gap_ppm = 0u64;
    let mut relative_gap_count = 0usize;
    let mut optimality_claim_count = 0usize;

    for reference in &oracle.case_results {
        let candidate = heuristic_by_id[reference.case_id.as_str()];
        if candidate.optimality_claim != "NOT_PROVEN" {
            optimality_claim_count += 1;
        }
        let has_candidate = candidate.nominal.makespan.is_some();
        let candidate_validated = has_candidate
            && candidate.nominal.validation_status == "INDEPENDENT_VALIDATOR_PASSED";
        let priority_only_has_candidate = candidate.nominal.priority_only_makespan.is_some();
        if priority_only_has_candidate {
            priority_only_candidates += 1;
        }
        if reference.feasible && priority_only_has_candidate {
            priority_only_feasible_candidates += 1;
        }
        let priority_only_missed = reference.feasible && !priority_only_has_candidate;
        if priority_only_missed {
            priority_only_misses += 1;
        }
        let fallback_recovered =
            candidate.nominal.fallback_status.starts_with("CANDIDATE_FOUND");
        if fallback_recovered {
            fallback_candidates += 1;
            fallback_states_total += candidate.nominal.fallback_states_expanded;
        }
        if candidate_validated {
            validated_candidates += 1;
        }
        if reference.feasible {
            reference_feasible += 1;
        }
        if reference.feasible && candidate_validated {
            feasible_candidates += 1;
        }
        let missed = reference.feasible && !candidate_validated;
        if missed {
            misses += 1;
        }
        let false_claim =
            reference.feasible && candidate.nominal.status == "PROVEN_INFEASIBLE_PRECHECK";
        if false_claim {
            false_infeasible += 1;
        }
        let conflict = has_candidate && !reference.feasible;
        if conflict {
            candidate_conflicts += 1;
        }

        let gap = if reference.feasible && candidate_validated {
            let exact = reference
                .nominal_optimal_makespan
                .ok_or_else(|| {
                    format!("{}: feasible oracle case has no optimum", reference.case_id)
                })?;
            let found = candidate
                .nominal
                .makespan
                .ok_or_else(|| format!("{}: candidate status has no makespan", reference.case_id))?;
            let Some(absolute) = found.checked_sub(exact) else {
                return Err(format!(
                    concat!(
                        "{}: heuristic candidate beats the exact oracle; investigate ",
                        "reference/model mismatch"
                    ),
                    reference.case_id
                ));
            };
            let relative = if exact == 0 {
                if absolute == 0 { 0 } else { u64::MAX }
            } else {
                ((u128::from(absolute) * 1_000_000u128) / u128::from(exact)) as u64
            };
            sum_relative_gap_ppm += u128::from(relative);
            max_relative_gap_ppm = max_relative_gap_ppm.max(relative);
            relative_gap_count += 1;
            Some((absolute, relative))
        } else {
            None
        };
        if has_candidate {
            candidates += 1;
        }

        rows.push(ScorecardRow {
            case_id: reference.case_id.clone(),
            oracle_feasible: reference.feasible,
            oracle_optimal_makespan: reference.nominal_optimal_makespan,
            priority_only_status: candidate.nominal.priority_only_status.clone(),
            priority_only_makespan: candidate.nominal.priority_only_makespan,
            priority_only_missed_feasible_candidate: priority_only_missed,
            fallback_status: candidate.nominal.fallback_status.clone(),
            fallback_states_expanded: candidate.nominal.fallback_states_expanded,
            fallback_orders_evaluated: candidate.nominal.fallback_orders_evaluated,
            heuristic_status: candidate.nominal.status.clone(),
            heuristic_makespan: candidate.nominal.makespan,
            candidate_independently_validated: candidate_validated,
            absolute_makespan_gap: gap.map(|value| value.0),
            relative_makespan_gap_ppm: gap.map(|value| value.1),
            heuristic_missed_feasible_candidate: missed,
            false_infeasibility_claim: false_claim,
            candidate_conflicts_with_exact_oracle: conflict,
            optimality_claim: candidate.optimality_claim.clone(),
        });
    }

    let all_emitted_candidates_pass_independent_validation =
        heuristic.all_returned_schedules_independently_validated
            && rows.iter().all(|row| {
                row.heuristic_makespan.is_none() || row.candidate_independently_validated
            });
    let no_false_infeasibility_claims = false_infeasible == 0;
    let coverage = if reference_feasible == 0 {
        1_000_000
    } else {
        ((feasible_candidates as u128 * 1_000_000u128) / reference_feasible as u128) as u64
    };
    let priority_only_coverage = if reference_feasible == 0 {
        1_000_000
    } else {
        ((priority_only_feasible_candidates as u128 * 1_000_000u128)
            / reference_feasible as u128) as u64
    };
    let mean_gap = if relative_gap_count == 0 {
        None
    } else {
        Some((sum_relative_gap_ppm / relative_gap_count as u128) as u64)
    };

    Ok(BenchmarkReport {
        schema_version: REPORT_SCHEMA.to_owned(),
        suite_id,
        scored_duration_scenario: "nominal_duration_only".to_owned(),
        fixture_sha256: sha256_hex(bytes),
        fixture_bytes: bytes.len(),
        oracle_report_sha256: sha256_hex(
            &serde_json::to_vec(&oracle).map_err(|error| error.to_string())?,
        ),
        heuristic_report_sha256: sha256_hex(
            &serde_json::to_vec(&heuristic).map_err(|error| error.to_string())?,
        ),
        total_case_count: oracle.case_count,
        oracle_feasible_case_count: reference_feasible,
        priority_only_candidate_count: priority_only_candidates,
        priority_only_feasible_case_coverage_ppm: priority_only_coverage,
        priority_only_missed_feasible_candidate_count: priority_only_misses,
        bounded_fallback_candidate_count: fallback_candidates,
        bounded_fallback_states_expanded_total: fallback_states_total,
        heuristic_candidate_count: candidates,
        heuristic_feasible_case_coverage_ppm: coverage,
        heuristic_missed_feasible_candidate_count: misses,
        false_infeasibility_claim_count: false_infeasible,
        candidate_oracle_conflict_count: candidate_conflicts,
        independently_validated_candidate_count: validated_candidates,
        conditional_mean_relative_gap_ppm: mean_gap,
        conditional_max_relative_gap_ppm: if relative_gap_count == 0 {
            None
        } else {
            Some(max_relative_gap_ppm)
        },
        gap_metrics_are_conditional_on_candidate_found: true,
        all_exact_reference_expectations_match: oracle.all_expected_outcomes_match,
        all_emitted_candidates_pass_independent_validation,
        no_false_infeasibility_claims,
        optimality_claim_count,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] =
        include_bytes!("../../../docs/research/fixtures/HABITAT_SCHEDULING_MICRO_ORACLE_V0_1.json");

    #[test]
    fn exact_versus_heuristic_scorecard_exposes_feasible_case_missed_by_priorities() {
        let report = evaluate_fixture_bytes(FIXTURE).unwrap();
        assert_eq!(report.total_case_count, 10);
        assert_eq!(report.oracle_feasible_case_count, 6);
        assert_eq!(report.priority_only_candidate_count, 5);
        assert_eq!(report.priority_only_feasible_case_coverage_ppm, 833_333);
        assert_eq!(report.priority_only_missed_feasible_candidate_count, 1);
        assert_eq!(report.bounded_fallback_candidate_count, 1);
        assert_eq!(report.bounded_fallback_states_expanded_total, 326);
        assert_eq!(report.heuristic_candidate_count, 6);
        assert_eq!(report.heuristic_missed_feasible_candidate_count, 0);
        assert_eq!(report.false_infeasibility_claim_count, 0);
        assert_eq!(report.candidate_oracle_conflict_count, 0);
        assert_eq!(report.heuristic_feasible_case_coverage_ppm, 1_000_000);
        assert_eq!(report.conditional_mean_relative_gap_ppm, Some(0));
        assert_eq!(report.conditional_max_relative_gap_ppm, Some(0));
        assert_eq!(report.optimality_claim_count, 0);
        assert!(report.all_exact_reference_expectations_match);
        assert!(report.all_emitted_candidates_pass_independent_validation);
        assert!(report.no_false_infeasibility_claims);
    }

    #[test]
    fn benchmark_row_distinguishes_missed_candidate_from_infeasibility() {
        let report = evaluate_fixture_bytes(FIXTURE).unwrap();
        let row = report
            .rows
            .iter()
            .find(|row| row.case_id == "priority_rules_miss_feasible_schedule")
            .unwrap();
        assert!(row.oracle_feasible);
        assert_eq!(row.oracle_optimal_makespan, Some(19));
        assert!(row.priority_only_makespan.is_none());
        assert!(row.priority_only_missed_feasible_candidate);
        assert_eq!(
            row.priority_only_status,
            "NO_CANDIDATE_FOUND_NOT_PROOF_OF_INFEASIBILITY"
        );
        assert_eq!(
            row.fallback_status,
            "CANDIDATE_FOUND_SEARCH_COMPLETE_NOT_OPTIMAL_PROVEN"
        );
        assert_eq!(row.heuristic_makespan, Some(19));
        assert!(!row.heuristic_missed_feasible_candidate);
        assert!(!row.false_infeasibility_claim);
        assert_eq!(
            row.heuristic_status,
            "BOUNDED_FALLBACK_FEASIBLE_NOT_PROVEN_OPTIMAL"
        );
    }

    #[test]
    fn invalid_input_does_not_generate_a_quality_scorecard() {
        let mut value: Value = serde_json::from_slice(FIXTURE).unwrap();
        value["unrecognized_quality_override"] = Value::Bool(true);
        assert!(evaluate_fixture_bytes(value.to_string().as_bytes()).is_err());
    }
}
