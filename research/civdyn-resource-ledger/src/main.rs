// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic two-region shock illustration. All numbers are fictional.

use civdyn_resource_ledger::{
    Production, Region, ResourceVector, StepInput, TransferPolicy, TransferRequest, World,
};

fn main() {
    let mut world = World::new(vec![
        Region {
            id: "region-a".into(),
            population: 100.0,
            opening_stock: ResourceVector::new(500.0, 2_000.0, 1_000.0),
            essential_demand_per_capita: ResourceVector::new(1.0, 4.0, 2.0),
        },
        Region {
            id: "region-b".into(),
            population: 100.0,
            opening_stock: ResourceVector::new(50.0, 300.0, 250.0),
            essential_demand_per_capita: ResourceVector::new(1.0, 4.0, 2.0),
        },
    ])
    .expect("valid synthetic two-region world");

    println!("scenario=synthetic_only; units=water_m3,energy_kwh,food_kg");
    println!(
        "tick,region,water_coverage,energy_coverage,food_coverage,closing_water_m3,closing_energy_kwh,closing_food_kg,residual_water_m3,residual_energy_kwh,residual_food_kg"
    );

    for tick in 0..4 {
        let drought = tick >= 2;
        let grid_disruption = tick == 3;
        let production = vec![
            Production {
                resources: ResourceVector::new(
                    if drought { 20.0 } else { 100.0 },
                    if grid_disruption { 50.0 } else { 500.0 },
                    80.0,
                ),
            },
            Production {
                resources: ResourceVector::new(
                    if drought { 5.0 } else { 20.0 },
                    if grid_disruption { 20.0 } else { 100.0 },
                    30.0,
                ),
            },
        ];
        let transfers = vec![TransferRequest {
            from_region: 0,
            to_region: 1,
            amount: ResourceVector::new(40.0, 120.0, 40.0),
        }];

        let report = world
            .step(
                &StepInput {
                    production,
                    transfers,
                },
                TransferPolicy::PreserveOpeningReserve { fraction: 0.1 },
            )
            .expect("synthetic scenario must conserve all resource stocks");

        for outcome in report.regions {
            let residual = outcome.ledger.conservation_residual();
            println!(
                "{},{},{},{},{},{:.3},{:.3},{:.3},{:.12},{:.12},{:.12}",
                report.tick,
                outcome.region_id,
                fmt(outcome.coverage.water),
                fmt(outcome.coverage.energy),
                fmt(outcome.coverage.food),
                outcome.closing_stock.water_m3,
                outcome.closing_stock.energy_kwh,
                outcome.closing_stock.food_kg,
                residual.water_m3,
                residual.energy_kwh,
                residual.food_kg,
            );
        }
    }
}

fn fmt(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.3}"))
        .unwrap_or_else(|| "undefined".into())
}
