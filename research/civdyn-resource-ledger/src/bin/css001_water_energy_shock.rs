// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! CSS-001: deterministic synthetic two-region water/energy/food shock.
//!
//! This is a mechanism benchmark fixture, not a forecast for any real region.
//! It compares no-transfer, fixed-solidarity and adaptive essential-service
//! deficit policies under identical stocks, demand and production.

use civdyn_resource_ledger::{
    Production, Region, ResourceVector, StepInput, TransferPolicy, TransferRequest, World,
};

const TICKS: usize = 4;
const SERVICE_FLOOR: f64 = 0.90;
const DONOR_RESERVE_TICKS: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Policy {
    NoTransfer,
    FixedSolidarity,
    AdaptiveServiceDeficit,
}

impl Policy {
    const ALL: [Self; 3] = [
        Self::NoTransfer,
        Self::FixedSolidarity,
        Self::AdaptiveServiceDeficit,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::NoTransfer => "no_transfer",
            Self::FixedSolidarity => "fixed_solidarity",
            Self::AdaptiveServiceDeficit => "adaptive_service_deficit",
        }
    }
}

#[derive(Debug)]
struct ScenarioResult {
    policy: Policy,
    reports: Vec<civdyn_resource_ledger::StepReport>,
    requested: ResourceVector,
    delivered: ResourceVector,
    floor_violations: usize,
}

fn initial_world() -> World {
    World::new(vec![
        Region {
            id: "synthetic_north".into(),
            population: 1_000.0,
            opening_stock: ResourceVector::new(1_000.0, 5_000.0, 4_000.0),
            essential_demand_per_capita: ResourceVector::new(0.10, 1.0, 0.50),
        },
        Region {
            id: "synthetic_south".into(),
            population: 800.0,
            opening_stock: ResourceVector::new(200.0, 3_000.0, 1_500.0),
            essential_demand_per_capita: ResourceVector::new(0.12, 1.25, 0.60),
        },
    ])
    .expect("synthetic CSS-001 regions must validate")
}

fn production_at(tick: usize) -> Vec<Production> {
    let shocked = tick >= 2;
    vec![
        Production {
            resources: if shocked {
                ResourceVector::new(10.0, 100.0, 70.0)
            } else {
                ResourceVector::new(40.0, 200.0, 100.0)
            },
        },
        Production {
            resources: if shocked {
                ResourceVector::new(5.0, 40.0, 25.0)
            } else {
                ResourceVector::new(10.0, 80.0, 40.0)
            },
        },
    ]
}

fn essential_demand(region: &Region) -> ResourceVector {
    ResourceVector::new(
        region.essential_demand_per_capita.water_m3 * region.population,
        region.essential_demand_per_capita.energy_kwh * region.population,
        region.essential_demand_per_capita.food_kg * region.population,
    )
}

/// Transfer only the receiving region's current-tick essential-service deficit,
/// while preserving half a tick of the donor's essential demand after service.
/// This is a transparent one-step controller, not a general optimizer.
fn adaptive_request(world: &World, production: &[Production]) -> TransferRequest {
    let donor = &world.regions()[0];
    let receiver = &world.regions()[1];
    let donor_demand = essential_demand(donor);
    let receiver_demand = essential_demand(receiver);
    let donor_production = production[0].resources;
    let receiver_production = production[1].resources;

    fn deficit_transfer(
        donor_stock: f64,
        donor_prod: f64,
        donor_demand: f64,
        receiver_stock: f64,
        receiver_prod: f64,
        receiver_demand: f64,
    ) -> f64 {
        let receiver_deficit = (receiver_demand - (receiver_stock + receiver_prod)).max(0.0);
        let donor_surplus =
            (donor_stock + donor_prod - donor_demand - DONOR_RESERVE_TICKS * donor_demand).max(0.0);
        receiver_deficit.min(donor_surplus)
    }

    TransferRequest {
        from_region: 0,
        to_region: 1,
        amount: ResourceVector::new(
            deficit_transfer(
                donor.opening_stock.water_m3,
                donor_production.water_m3,
                donor_demand.water_m3,
                receiver.opening_stock.water_m3,
                receiver_production.water_m3,
                receiver_demand.water_m3,
            ),
            deficit_transfer(
                donor.opening_stock.energy_kwh,
                donor_production.energy_kwh,
                donor_demand.energy_kwh,
                receiver.opening_stock.energy_kwh,
                receiver_production.energy_kwh,
                receiver_demand.energy_kwh,
            ),
            deficit_transfer(
                donor.opening_stock.food_kg,
                donor_production.food_kg,
                donor_demand.food_kg,
                receiver.opening_stock.food_kg,
                receiver_production.food_kg,
                receiver_demand.food_kg,
            ),
        ),
    }
}

fn transfers_for(policy: Policy, world: &World, production: &[Production]) -> Vec<TransferRequest> {
    match policy {
        Policy::NoTransfer => Vec::new(),
        Policy::FixedSolidarity => vec![TransferRequest {
            from_region: 0,
            to_region: 1,
            amount: ResourceVector::new(60.0, 250.0, 80.0),
        }],
        Policy::AdaptiveServiceDeficit => {
            let request = adaptive_request(world, production);
            if request.amount.water_m3 == 0.0
                && request.amount.energy_kwh == 0.0
                && request.amount.food_kg == 0.0
            {
                Vec::new()
            } else {
                vec![request]
            }
        }
    }
}

fn simulate(policy: Policy) -> ScenarioResult {
    let mut world = initial_world();
    let mut reports = Vec::with_capacity(TICKS);
    let mut requested = ResourceVector::default();
    let mut delivered = ResourceVector::default();
    let mut floor_violations = 0usize;

    for tick in 0..TICKS {
        let production = production_at(tick);
        let transfers = transfers_for(policy, &world, &production);
        for transfer in &transfers {
            requested.water_m3 += transfer.amount.water_m3;
            requested.energy_kwh += transfer.amount.energy_kwh;
            requested.food_kg += transfer.amount.food_kg;
        }

        let report = world
            .step(
                &StepInput {
                    production,
                    transfers,
                },
                TransferPolicy::Unrestricted,
            )
            .expect("CSS-001 inputs must conserve resources");

        for receipt in &report.transfers {
            delivered.water_m3 += receipt.delivered.water_m3;
            delivered.energy_kwh += receipt.delivered.energy_kwh;
            delivered.food_kg += receipt.delivered.food_kg;
        }
        for region in &report.regions {
            for coverage in [
                region.coverage.water,
                region.coverage.energy,
                region.coverage.food,
            ] {
                if coverage.map(|value| value < SERVICE_FLOOR).unwrap_or(false) {
                    floor_violations += 1;
                }
            }
            assert!(
                region.ledger.is_conserved(),
                "ledger conservation must hold"
            );
        }
        reports.push(report);
    }

    ScenarioResult {
        policy,
        reports,
        requested,
        delivered,
        floor_violations,
    }
}

fn main() {
    println!(
        "scenario=CSS-001; epistemic_class=synthetic_benchmark; geography=abstract_two_region"
    );
    println!(
        "tick,policy,region,water_coverage,energy_coverage,food_coverage,unmet_water_m3,unmet_energy_kwh,unmet_food_kg,closing_water_m3,closing_energy_kwh,closing_food_kg,residual_water_m3,residual_energy_kwh,residual_food_kg"
    );
    for policy in Policy::ALL {
        let result = simulate(policy);
        for report in &result.reports {
            for region in &report.regions {
                let residual = region.ledger.conservation_residual();
                println!(
                    "{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.3},{:.3},{:.3},{:.12},{:.12},{:.12}",
                    report.tick,
                    result.policy.name(),
                    region.region_id,
                    fmt(region.coverage.water),
                    fmt(region.coverage.energy),
                    fmt(region.coverage.food),
                    region.unmet.water_m3,
                    region.unmet.energy_kwh,
                    region.unmet.food_kg,
                    region.closing_stock.water_m3,
                    region.closing_stock.energy_kwh,
                    region.closing_stock.food_kg,
                    residual.water_m3,
                    residual.energy_kwh,
                    residual.food_kg,
                );
            }
        }
        println!(
            "summary,{},{},{:.3},{:.3},{:.3},{},",
            result.policy.name(),
            TICKS,
            result.requested.water_m3,
            result.requested.energy_kwh,
            result.requested.food_kg,
            result.floor_violations,
        );
        println!(
            "delivered,{},{:.3},{:.3},{:.3}",
            result.policy.name(),
            result.delivered.water_m3,
            result.delivered.energy_kwh,
            result.delivered.food_kg,
        );
    }
}

fn fmt(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.3}"))
        .unwrap_or_else(|| "undefined".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_shock_causes_late_receiver_service_failures_without_transfers() {
        let result = simulate(Policy::NoTransfer);
        assert!(result.floor_violations > 0);
        let last_south = result
            .reports
            .last()
            .unwrap()
            .regions
            .iter()
            .find(|region| region.region_id == "synthetic_south")
            .unwrap();
        assert!(last_south.coverage.water.unwrap() < SERVICE_FLOOR);
        assert!(last_south.coverage.energy.unwrap() < SERVICE_FLOOR);
        assert!(last_south.coverage.food.unwrap() < SERVICE_FLOOR);
    }

    #[test]
    fn fixed_solidarity_and_adaptive_service_deficit_keep_all_service_floors() {
        for policy in [Policy::FixedSolidarity, Policy::AdaptiveServiceDeficit] {
            let result = simulate(policy);
            assert_eq!(result.floor_violations, 0, "{:?}", policy);
            assert!(
                result
                    .reports
                    .iter()
                    .flat_map(|report| &report.regions)
                    .all(|region| region.ledger.is_conserved())
            );
        }
    }

    #[test]
    fn adaptive_policy_uses_no_more_transfers_than_fixed_policy_per_resource() {
        let fixed = simulate(Policy::FixedSolidarity);
        let adaptive = simulate(Policy::AdaptiveServiceDeficit);
        assert!(adaptive.delivered.water_m3 < fixed.delivered.water_m3);
        assert!(adaptive.delivered.energy_kwh < fixed.delivered.energy_kwh);
        assert!(adaptive.delivered.food_kg < fixed.delivered.food_kg);
    }

    #[test]
    fn adaptive_policy_preserves_donor_service_and_half_tick_buffer() {
        let result = simulate(Policy::AdaptiveServiceDeficit);
        for report in &result.reports {
            let donor = report
                .regions
                .iter()
                .find(|region| region.region_id == "synthetic_north")
                .unwrap();
            assert_eq!(donor.coverage.water, Some(1.0));
            assert_eq!(donor.coverage.energy, Some(1.0));
            assert_eq!(donor.coverage.food, Some(1.0));
            assert!(donor.closing_stock.water_m3 >= 50.0);
            assert!(donor.closing_stock.energy_kwh >= 500.0);
            assert!(donor.closing_stock.food_kg >= 250.0);
        }
    }

    #[test]
    fn analytical_transfer_totals_are_resource_specific_and_reproducible() {
        let fixed = simulate(Policy::FixedSolidarity);
        assert_eq!(fixed.floor_violations, 0);
        assert_eq!(fixed.delivered.water_m3, 240.0);
        assert_eq!(fixed.delivered.energy_kwh, 1_000.0);
        assert_eq!(fixed.delivered.food_kg, 320.0);

        let adaptive = simulate(Policy::AdaptiveServiceDeficit);
        assert_eq!(adaptive.floor_violations, 0);
        assert_eq!(adaptive.delivered.water_m3, 154.0);
        assert_eq!(adaptive.delivered.energy_kwh, 760.0);
        assert_eq!(adaptive.delivered.food_kg, 290.0);

        let no_transfer = simulate(Policy::NoTransfer);
        assert_eq!(no_transfer.floor_violations, 4);
    }

    #[test]
    fn resource_ledger_is_deterministic_for_each_policy() {
        for policy in Policy::ALL {
            assert_eq!(simulate(policy).reports, simulate(policy).reports);
        }
    }
}
