use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_production_optioneering_core::{
    evaluate_option_set, EvidenceDisposition, FunctionalOutcomeContract, OptionCandidate,
    MeasurementBasis, OptionTier, PerformanceRequirement, ScenarioCase, ScenarioResult, StageDisposition,
    StageGateDecision,
};

fn contract() -> FunctionalOutcomeContract {
    FunctionalOutcomeContract {
        contract_id: "synthetic-partition-outcome-v1".to_owned(),
        contract_digest: "fixture:synthetic-partition-contract-v1".to_owned(),
        project_context_digest: "fixture:synthetic-project-context-v1".to_owned(),
        unit_class_id: "partition-accepted-m2".to_owned(),
        unit_symbol: "m2".to_owned(),
        required_units: 100,
        acceptance_contract_ref: "acceptance:partition-v1".to_owned(),
        currency_code: "USD".to_owned(),
        carbon_assessment_boundary_ref: "lca:cradle-to-end-of-life-v1".to_owned(),
        assessment_horizon_months: 240,
        required_gate_ids: vec!["design-authority".to_owned()],
        performance_requirements: vec![PerformanceRequirement {
            metric_id: "fire-rating-minutes".to_owned(),
            unit_symbol: "min".to_owned(),
            scale: 1,
            minimum_value: Some(60),
            maximum_value: None,
        }],
    }
}

fn qualified(id: &str) -> EvidenceDisposition {
    EvidenceDisposition::Qualified {
        authority_ref: format!("authority:{id}"),
        receipt_ref: format!("receipt:{id}"),
    }
}

fn scenario_result(
    index: usize,
    scenario_id: &str,
    cost: u64,
    duration: u64,
) -> ScenarioResult {
    let design = format!("design-option-{index:05}");
    ScenarioResult {
        scenario_id: scenario_id.to_owned(),
        measurement_basis: MeasurementBasis::FullRequiredQuantity,
        option_design_digest: design.clone(),
        assessment_horizon_months: 240,
        acceptance_contract_digest: "fixture:synthetic-partition-contract-v1".to_owned(),
        accepted_option_design_digest: design,
        acceptance: qualified(&format!("acceptance-{scenario_id}")),
        hard_gate_results: BTreeMap::from([(
            "design-authority".to_owned(),
            qualified("design-authority"),
        )]),
        accepted_units: 100,
        performance_values: BTreeMap::from([("fire-rating-minutes".to_owned(), 90)]),
        direct_cost_minor_units: cost,
        shared_cost_minor_units: 250,
        maintenance_cost_minor_units: 100,
        repair_cost_minor_units: 25,
        end_of_life_cost_minor_units: 50,
        elapsed_minutes: duration,
        workflow_crew_minutes: 10_000_u64.saturating_sub(index as u64 * 2),
        energy_wh: 50_000_u64.saturating_sub(index as u64 * 5),
        carbon_gco2e: 80_000_u64.saturating_sub(index as u64 * 8),
        material_mass_kg_milli: 150_000,
        rework_units: 1,
    }
}

fn options(size: usize) -> Vec<OptionCandidate> {
    (0..size)
        .map(|index| {
            let design = format!("design-option-{index:05}");
            let cost = 10_000_u64 + index as u64 * 10;
            let nominal_duration = 100_000_u64 - index as u64 * 25;
            let scenario_results = BTreeMap::from([
                (
                    "nominal".to_owned(),
                    scenario_result(index, "nominal", cost, nominal_duration),
                ),
                (
                    "outage-rework".to_owned(),
                    scenario_result(
                        index,
                        "outage-rework",
                        cost + 500,
                        nominal_duration + 4_000,
                    ),
                ),
            ]);
            OptionCandidate {
                option_id: format!("option-{index:05}"),
                tier: OptionTier::Switch,
                functional_contract_id: "synthetic-partition-outcome-v1".to_owned(),
                functional_contract_digest: "fixture:synthetic-partition-contract-v1".to_owned(),
                option_design_digest: design,
                method_profile_digest: format!("method-option-{index:05}"),
                scheduler_profile_digest: "scheduler-reference-v1".to_owned(),
                fleet_profile_digest: "fleet-reference-v1".to_owned(),
                matched_reference_option_id: None,
                scenario_results,
            }
        })
        .collect()
}

fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    assert!(!samples.is_empty(), "benchmark samples are non-empty");
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = (sorted.len() * percentile).div_ceil(100).max(1);
    sorted[rank - 1]
}

fn main() {
    let contract = contract();
    let stages = vec![
        StageGateDecision {
            tier: OptionTier::Avoid,
            disposition: StageDisposition::ClosedNoViableAlternative,
            accountable_owner_ref: "owner:need-assessment".to_owned(),
            project_context_digest: contract.project_context_digest.clone(),
            evidence_refs: vec!["evidence:avoid-screen".to_owned()],
            rationale: "synthetic benchmark closes the avoid stage".to_owned(),
        },
        StageGateDecision {
            tier: OptionTier::Switch,
            disposition: StageDisposition::OpenForEvaluation,
            accountable_owner_ref: "owner:method-assessment".to_owned(),
            project_context_digest: contract.project_context_digest.clone(),
            evidence_refs: vec!["evidence:switch-screen".to_owned()],
            rationale: "synthetic switched-method candidates are open for comparison".to_owned(),
        },
    ];
    let scenarios = vec![
        ScenarioCase {
            scenario_id: "nominal".to_owned(),
        },
        ScenarioCase {
            scenario_id: "outage-rework".to_owned(),
        },
    ];

    for size in [10_usize, 100, 500, 1_000, 2_000] {
        let candidates = options(size);
        let mut samples = Vec::with_capacity(11);
        let mut frontier_size = 0;
        for _ in 0..11 {
            let start = Instant::now();
            let report = evaluate_option_set(
                black_box(&contract),
                black_box(&stages),
                black_box(&scenarios),
                black_box(&candidates),
            )
            .expect("frozen robust optioneering fixture");
            samples.push(start.elapsed());
            frontier_size = report.frontier_option_ids.len();
        }
        println!(
            "options={size} scenarios=2 repetitions=11 frontier={frontier_size} evaluate_p50_ms={:.3} evaluate_p95_ms={:.3} evaluate_p99_ms={:.3}",
            percentile(&samples, 50).as_secs_f64() * 1_000.0,
            percentile(&samples, 95).as_secs_f64() * 1_000.0,
            percentile(&samples, 99).as_secs_f64() * 1_000.0
        );
        black_box(candidates);
    }
}
