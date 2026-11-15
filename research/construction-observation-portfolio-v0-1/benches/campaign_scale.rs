use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_observation_planning_core::WorkNeed;
use construction_observation_portfolio_core::{
    plan_greedy_observation_campaign, plan_observation_campaign,
};
use construction_robot_interop_core::resources::{ReservationBook, ReservationRequest};
use construction_spacetime_occupancy_core::{
    Bounds3Mm, OccupancyKind, SiteBinding, SpatialOccupancy, SpatialOccupancyBook,
};
use construction_observation_planning_core::ObservationAction;

fn binding() -> SiteBinding {
    SiteBinding {
        floor_id: "portfolio-benchmark-floor".to_owned(),
        site_epoch: "site-epoch-1".to_owned(),
        design_revision_digest: "benchmark-design-digest".to_owned(),
        target_frame_id: "project-grid-1".to_owned(),
    }
}

fn action(index: usize) -> ObservationAction {
    let region_index = index / 2;
    let action_id = format!("scan-{index:02}");
    let task_id = format!("task-{index:02}");
    let x = i64::try_from(index).expect("small action index") * 2_000;

    ObservationAction {
        action_id: action_id.clone(),
        coverage_profile_ref: format!("coverage:{action_id}"),
        calibration_ref: format!("calibration:{action_id}"),
        covered_region_ids: vec![format!("region-{region_index:02}")],
        occupancy: SpatialOccupancy {
            occupancy_id: format!("occupancy-{index:02}"),
            task_id: task_id.clone(),
            kind: OccupancyKind::InspectionAccess,
            binding: binding(),
            bounds: Bounds3Mm {
                min: [x, 0, 0],
                max: [x + 500, 500, 1_000],
            },
            uncertainty_margin_mm: [0, 0, 0],
            clearance_margin_mm: [0, 0, 0],
            reservation: ReservationRequest {
                reservation_id: format!("reservation-{index:02}"),
                resource_id: format!("resource-{index:02}"),
                task_id,
                owner_id: format!("owner-{index:02}"),
                site_epoch: "site-epoch-1".to_owned(),
                clock_domain_id: "site-clock-1".to_owned(),
                clock_uncertainty_ms: 0,
                interval_start_ms: 0,
                interval_end_ms: 60_000,
                expires_at_ms: 61_000,
                units: 1,
            },
        },
        duration_minutes: 1,
        crew_minutes: 5 + u64::try_from(index % 3).expect("small crew delta"),
        cost_minor_units: 100 + u64::try_from(index % 5).expect("small cost delta") * 10,
        cost_currency_code: "USD".to_owned(),
        energy_wh: 50 + u64::try_from(index % 7).expect("small energy delta"),
    }
}

fn percentile(samples: &mut [Duration], percentile: usize) -> Duration {
    samples.sort_unstable();
    let index = ((samples.len() - 1) * percentile).div_ceil(100);
    samples[index]
}

fn main() {
    for action_count in [8_usize, 12, 16, 20] {
        let actions: Vec<ObservationAction> =
            (0..action_count).map(action).collect();
        let need_count = action_count / 2;
        let needs: Vec<WorkNeed> = (0..need_count)
            .map(|index| WorkNeed {
                work_unit_id: format!("unit-{index:02}"),
                region_id: format!("region-{index:02}"),
                uncertainty_bound_mm: [100, 100, 100],
            })
            .collect();

        let occupancy =
            SpatialOccupancyBook::new(binding(), "site-clock-1").expect("occupancy book");
        let mut reservations =
            ReservationBook::new("site-epoch-1", "site-clock-1", 0).expect("reservation book");
        for index in 0..action_count {
            reservations
                .register_resource(&format!("resource-{index:02}"), 1)
                .expect("resource");
        }

        let mut exact_samples = Vec::new();
        let mut greedy_samples = Vec::new();
        let mut exact_nodes = 0;
        let mut exact_actions = 0;
        let mut exact_crew = 0;
        let mut greedy_crew = 0;

        for _ in 0..5 {
            let start = Instant::now();
            let exact = plan_observation_campaign(
                black_box(&actions),
                black_box(&needs),
                black_box(&occupancy),
                black_box(&reservations),
                0,
                1_000_000,
            )
            .expect("bounded exact campaign");
            exact_samples.push(start.elapsed());
            assert!(exact.optimality_proven);
            exact_nodes = exact.nodes_visited;
            exact_actions = exact.best_campaign.selected_actions.len();
            exact_crew = exact.best_campaign.total_crew_minutes;

            let start = Instant::now();
            let greedy = plan_greedy_observation_campaign(
                black_box(&actions),
                black_box(&needs),
                black_box(&occupancy),
                black_box(&reservations),
                0,
            )
            .expect("greedy campaign");
            greedy_samples.push(start.elapsed());
            greedy_crew = greedy.campaign.total_crew_minutes;
        }

        let exact_p50 = percentile(&mut exact_samples, 50);
        let exact_p95 = percentile(&mut exact_samples, 95);
        let greedy_p50 = percentile(&mut greedy_samples, 50);
        let greedy_p95 = percentile(&mut greedy_samples, 95);

        println!(
            "actions={action_count} needs={need_count} exact_nodes={exact_nodes} exact_actions={exact_actions} exact_crew_minutes={exact_crew} greedy_crew_minutes={greedy_crew} exact_p50_us={:.3} exact_p95_us={:.3} greedy_p50_us={:.3} greedy_p95_us={:.3}",
            exact_p50.as_secs_f64() * 1_000_000.0,
            exact_p95.as_secs_f64() * 1_000_000.0,
            greedy_p50.as_secs_f64() * 1_000_000.0,
            greedy_p95.as_secs_f64() * 1_000_000.0
        );
    }
}
