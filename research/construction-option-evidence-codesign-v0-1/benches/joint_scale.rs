use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::Instant;

use construction_observation_planning_core::{ObservationAction, WorkNeed};
use construction_option_evidence_codesign_core::{
    CampaignCostTreatment, OptionEvidenceCampaign, evaluate_option_evidence_packages,
};
use construction_production_optioneering_core::{
    EvidenceDisposition, FunctionalOutcomeContract, MeasurementBasis, OptionCandidate, OptionTier,
    PerformanceRequirement, ScenarioCase, ScenarioResult, StageDisposition, StageGateDecision,
};
use construction_robot_interop_core::resources::{ReservationBook, ReservationRequest};
use construction_spacetime_occupancy_core::{
    Bounds3Mm, OccupancyKind, SiteBinding, SpatialOccupancy, SpatialOccupancyBook,
};

fn contract() -> FunctionalOutcomeContract {
    FunctionalOutcomeContract {
        contract_id: "benchmark-partition-contract".to_owned(),
        contract_digest: "git-blob-sha1:benchmark-contract".to_owned(),
        project_context_digest: "git-blob-sha1:benchmark-context".to_owned(),
        unit_class_id: "non-loadbearing-partition".to_owned(),
        unit_symbol: "m2".to_owned(),
        required_units: 1,
        acceptance_contract_ref: "acceptance:benchmark-partition".to_owned(),
        currency_code: "USD".to_owned(),
        carbon_assessment_boundary_ref: "lca-boundary:benchmark-v1".to_owned(),
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

fn build_inputs(
    option_count: usize,
) -> (
    FunctionalOutcomeContract,
    Vec<StageGateDecision>,
    Vec<ScenarioCase>,
    Vec<OptionCandidate>,
    Vec<OptionEvidenceCampaign>,
) {
    let contract = contract();
    let stages = vec![
        StageGateDecision {
            tier: OptionTier::Avoid,
            disposition: StageDisposition::ClosedNoViableAlternative,
            accountable_owner_ref: "benchmark-client-owner".to_owned(),
            project_context_digest: contract.project_context_digest.clone(),
            evidence_refs: vec!["benchmark-avoid-evidence".to_owned()],
            rationale: "frozen benchmark: no viable avoid option".to_owned(),
        },
        StageGateDecision {
            tier: OptionTier::Switch,
            disposition: StageDisposition::OpenForEvaluation,
            accountable_owner_ref: "benchmark-client-owner".to_owned(),
            project_context_digest: contract.project_context_digest.clone(),
            evidence_refs: vec!["benchmark-switch-evidence".to_owned()],
            rationale: "evaluate matched switch options".to_owned(),
        },
    ];
    let scenarios = vec![ScenarioCase {
        scenario_id: "nominal".to_owned(),
    }];
    let mut options = Vec::new();
    let mut campaigns = Vec::new();

    for index in 0..option_count {
        let option_id = format!("option-{index:03}");
        let design_digest = format!("design-digest-{index:03}");
        let option_result = ScenarioResult {
            scenario_id: "nominal".to_owned(),
            measurement_basis: MeasurementBasis::FullRequiredQuantity,
            option_design_digest: design_digest.clone(),
            assessment_horizon_months: contract.assessment_horizon_months,
            acceptance_contract_digest: contract.contract_digest.clone(),
            accepted_option_design_digest: design_digest.clone(),
            acceptance: EvidenceDisposition::Qualified {
                authority_ref: format!("acceptance-authority-{index}"),
                receipt_ref: format!("acceptance-receipt-{index}"),
            },
            hard_gate_results: BTreeMap::from([(
                "design-authority".to_owned(),
                EvidenceDisposition::Qualified {
                    authority_ref: format!("design-authority-{index}"),
                    receipt_ref: format!("design-receipt-{index}"),
                },
            )]),
            accepted_units: 1,
            performance_values: BTreeMap::from([("fire-rating-minutes".to_owned(), 90)]),
            direct_cost_minor_units: 1_000 + index as u64 * 10,
            shared_cost_minor_units: 100,
            maintenance_cost_minor_units: 50,
            repair_cost_minor_units: 0,
            end_of_life_cost_minor_units: 0,
            elapsed_minutes: 300 + index as u64,
            workflow_crew_minutes: 200 + index as u64,
            energy_wh: 1_000 + index as u64 * 5,
            carbon_gco2e: 500 + index as u64 * 3,
            material_mass_kg_milli: 100_000,
            rework_units: 0,
        };
        options.push(OptionCandidate {
            option_id: option_id.clone(),
            tier: OptionTier::Switch,
            functional_contract_id: contract.contract_id.clone(),
            functional_contract_digest: contract.contract_digest.clone(),
            option_design_digest: design_digest.clone(),
            method_profile_digest: format!("method-{index:03}"),
            scheduler_profile_digest: "scheduler-benchmark-v1".to_owned(),
            fleet_profile_digest: "fleet-benchmark-v1".to_owned(),
            matched_reference_option_id: None,
            scenario_results: BTreeMap::from([("nominal".to_owned(), option_result)]),
        });

        let floor_id = "benchmark-floor-01".to_owned();
        let site_epoch = "benchmark-epoch-01".to_owned();
        let target_frame_id = "benchmark-grid-01".to_owned();
        let clock_domain_id = "benchmark-clock-01".to_owned();
        let task_id = format!("survey-task-{index:03}");
        let occupancy_id = format!("survey-occupancy-{index:03}");
        let reservation_id = format!("survey-reservation-{index:03}");
        let resource_id = format!("scanner-{index:03}");
        let binding = SiteBinding {
            floor_id: floor_id.clone(),
            site_epoch: site_epoch.clone(),
            design_revision_digest: design_digest.clone(),
            target_frame_id: target_frame_id.clone(),
        };
        let x = i64::try_from(index)
            .expect("benchmark index fits i64")
            .checked_mul(2_000)
            .expect("benchmark coordinate does not overflow");
        let occupancy = SpatialOccupancy {
            occupancy_id: occupancy_id.clone(),
            task_id: task_id.clone(),
            kind: OccupancyKind::InspectionAccess,
            binding: binding.clone(),
            bounds: Bounds3Mm {
                min: [x, 0, 0],
                max: [x + 1_000, 1_000, 1_000],
            },
            uncertainty_margin_mm: [10, 10, 10],
            clearance_margin_mm: [20, 20, 20],
            reservation: ReservationRequest {
                reservation_id,
                resource_id: resource_id.clone(),
                task_id: task_id.clone(),
                owner_id: format!("owner-{index:03}"),
                site_epoch: site_epoch.clone(),
                clock_domain_id: clock_domain_id.clone(),
                clock_uncertainty_ms: 0,
                interval_start_ms: 0,
                interval_end_ms: 60_000,
                expires_at_ms: 120_000,
                units: 1,
            },
        };
        let action = ObservationAction {
            action_id: format!("scan-{index:03}"),
            coverage_profile_ref: format!("coverage-profile-{index:03}"),
            calibration_ref: format!("calibration-{index:03}"),
            covered_region_ids: vec![format!("region-{index:03}")],
            occupancy,
            duration_minutes: 10,
            crew_minutes: 5,
            cost_minor_units: 20,
            cost_currency_code: "USD".to_owned(),
            energy_wh: 15,
        };
        let work_need = WorkNeed {
            work_unit_id: format!("work-unit-{index:03}"),
            region_id: format!("region-{index:03}"),
            uncertainty_bound_mm: [100, 100, 100],
        };
        let mut reservation_book =
            ReservationBook::new(site_epoch.clone(), clock_domain_id.clone(), 0)
                .expect("valid reservation book");
        reservation_book
            .register_resource(resource_id, 1)
            .expect("survey instrument resource");
        let occupancy_book =
            SpatialOccupancyBook::new(binding, clock_domain_id.clone())
                .expect("valid occupancy book");

        campaigns.push(OptionEvidenceCampaign {
            option_id,
            functional_contract_digest: contract.contract_digest.clone(),
            project_context_digest: contract.project_context_digest.clone(),
            option_design_digest: design_digest,
            floor_id,
            site_epoch,
            target_frame_id,
            clock_domain_id,
            currency_code: "USD".to_owned(),
            cost_treatment: CampaignCostTreatment::AdditionalToOptionLifecycleCost,
            cost_boundary_ref: format!("benchmark-campaign-cost-boundary-{index:03}"),
            work_needs: vec![work_need],
            actions: vec![action],
            occupancy_book,
            reservation_book,
        });
    }

    (contract, stages, scenarios, options, campaigns)
}

fn percentile(samples: &[u128], percentile: usize) -> u128 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() - 1) * percentile).div_ceil(100);
    sorted[index]
}

fn main() {
    for option_count in [2_usize, 4, 8, 16] {
        let (contract, stages, scenarios, options, campaigns) = build_inputs(option_count);
        let mut samples = Vec::with_capacity(25);
        let mut total_nodes = 0_usize;
        let mut frontier_count = 0_usize;

        for _ in 0..25 {
            let start = Instant::now();
            let report = evaluate_option_evidence_packages(
                black_box(&contract),
                black_box(&stages),
                black_box(&scenarios),
                black_box(&options),
                black_box(&campaigns),
                0,
                128,
            )
            .expect("frozen valid option/evidence package corpus");
            samples.push(start.elapsed().as_nanos());
            total_nodes += report
                .joint_evaluations
                .iter()
                .filter_map(|item| item.campaign_search.as_ref())
                .map(|search| search.nodes_visited)
                .sum::<usize>();
            frontier_count = report.proven_frontier_option_ids.len();
            assert!(report.joint_frontier_proven);
            black_box(report);
        }

        println!(
            "options={option_count} repetitions=25 p50_us={:.3} p95_us={:.3} p99_us={:.3} average_campaign_nodes={:.1} frontier_count={frontier_count}",
            percentile(&samples, 50) as f64 / 1_000.0,
            percentile(&samples, 95) as f64 / 1_000.0,
            percentile(&samples, 99) as f64 / 1_000.0,
            total_nodes as f64 / 25.0
        );
    }
}
