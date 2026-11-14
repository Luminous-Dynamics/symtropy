use std::hint::black_box;
use std::time::Instant;

use construction_automation_portfolio_core::{
    solve_portfolio, CostBreakdown, EligibilityStatus, ExecutionMode, ModeEligibility, ModeOption,
    PortfolioPolicy, PortfolioProblem, RequiredWorkUnit,
};
use construction_robot_interop_core::resources::{ReservationBook, ReservationRequest};
use construction_spacetime_occupancy_core::{SiteBinding, SpatialOccupancyBook};
 
fn request(id: &str, resource_id: &str, start: u64, end: u64) -> ReservationRequest {
    ReservationRequest {
        reservation_id: format!("reservation-{id}"),
        resource_id: resource_id.to_owned(),
        task_id: id.to_owned(),
        owner_id: format!("owner-{id}"),
        site_epoch: "site-epoch-1".to_owned(),
        clock_domain_id: "site-clock-1".to_owned(),
        clock_uncertainty_ms: 0,
        interval_start_ms: start,
        interval_end_ms: end,
        expires_at_ms: end + 10_000,
        units: 1,
    }
}

fn option(id: &str, unit: &str, mode: ExecutionMode, start: u64, end: u64) -> ModeOption {
    ModeOption {
        option_id: id.to_owned(),
        work_unit_id: unit.to_owned(),
        mode,
        eligibility: ModeEligibility {
            status: EligibilityStatus::Eligible,
            capability_profile_ref: Some(format!("capability:{id}")),
            qualification_ref: Some(format!("qualification:{id}")),
            authority_scope_ref: Some(format!("authority:{id}")),
            reason: None,
        },
        scheduled_start_ms: start,
        scheduled_end_ms: end,
        acceptance_contract_ref: format!("acceptance:{unit}"),
        crew_minutes_by_role: std::collections::BTreeMap::from([("crew".to_owned(), 10)]),
        skilled_crew_minutes: 5,
        energy_wh: 20,
        cost: CostBreakdown {
            labour_minor_units: 100,
            equipment_minor_units: if mode == ExecutionMode::Robot { 100 } else { 0 },
            ..CostBreakdown::default()
        },
        reservation_requests: vec![request(
            id,
            if mode == ExecutionMode::Robot { "robot-fleet" } else { "human-crew" },
            start,
            end,
        )],
        spatial_occupancy: None,
    }
}

fn run(size: usize) {
    let mut units = Vec::with_capacity(size);
    for index in 0..size {
        let id = format!("panel-{index:03}");
        let start = index as u64 * 10_000;
        units.push(RequiredWorkUnit {
            work_unit_id: id.clone(),
            functional_unit_class: "panel".to_owned(),
            acceptance_contract_ref: format!("acceptance:{id}"),
            predecessor_work_unit_ids: Vec::new(),
            mode_options: vec![
                option(&format!("human-{id}"), &id, ExecutionMode::Human, start, start + 5_000),
                option(&format!("robot-{id}"), &id, ExecutionMode::Robot, start, start + 3_000),
            ],
        });
    }

    let mut resources = ReservationBook::new("site-epoch-1", "site-clock-1", 0)
        .expect("resource book");
    resources.register_resource("human-crew", u32::MAX).expect("human capacity");
    resources.register_resource("robot-fleet", u32::MAX).expect("robot capacity");
    let occupancies = SpatialOccupancyBook::new(
        SiteBinding {
            floor_id: "benchmark-floor".to_owned(),
            site_epoch: "site-epoch-1".to_owned(),
            design_revision_digest: "benchmark-design".to_owned(),
            target_frame_id: "benchmark-grid".to_owned(),
        },
        "site-clock-1",
    )
    .expect("occupancy book");
    let problem = PortfolioProblem {
        currency_code: "USD".to_owned(),
        project_start_ms: 0,
        max_search_states: 1_000_000,
        work_units: units,
        initial_reservations: resources,
        initial_occupancies: occupancies,
    };
    let start = Instant::now();
    let result = solve_portfolio(&problem, PortfolioPolicy::ExactPareto)
        .expect("portfolio search");
    let elapsed = start.elapsed();
    println!(
        "work_units={size} evaluated_states={} feasible={} frontier={} exhaustive={} elapsed_ms={:.3}",
        result.evaluated_states,
        result.feasible_portfolio_count,
        result.pareto_frontier.len(),
        result.completeness == construction_automation_portfolio_core::SearchCompleteness::Exhaustive,
        elapsed.as_secs_f64() * 1_000.0
    );
    black_box(result);
}

fn main() {
    for size in [2_usize, 4, 6, 8, 10, 12] {
        run(size);
    }
}
