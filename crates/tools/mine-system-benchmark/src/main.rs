// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use mine_system_benchmark::{
    evaluate, BalanceLedger, CampaignCase, ConstraintState, EvidenceState, ProductState,
    CandidateMetrics,
};

fn main() {
    // Synthetic fixture for exercising accounting and admission semantics.
    // Values are illustrative and are not measured mine performance.
    let case = CampaignCase {
        scenario_id: "MS-001-NOMINAL-SYNTHETIC".to_owned(),
        evidence: vec![EvidenceState::Qualified],
        hard_constraints: vec![ConstraintState::Satisfied],
        product_state: ProductState::MeetsSpecification,
        mass_ledger_kg_per_shift: BalanceLedger {
            opening: 100_000.0,
            inputs: 50_000.0,
            recoveries: 0.0,
            outputs: 120_000.0,
            losses: 10_000.0,
            closing: 20_000.0,
            tolerance: 1e-9,
        },
        saleable_product_kg_per_shift: 42_000.0,
        product_mass_tolerance_kg: 1e-9,
        energy_ledger_kwh: BalanceLedger {
            opening: 30.0,
            inputs: 100.0,
            recoveries: 5.0,
            outputs: 110.0,
            losses: 5.0,
            closing: 20.0,
            tolerance: 1e-9,
        },
        energy_intensity_tolerance_kwh_per_tonne: 1e-9,
        metrics: CandidateMetrics {
            comparison_basis_id: "MS-001-SYNTHETIC-BASIS-V1",
            accepted_tonnes_per_shift: 42.0,
            lifecycle_cost_per_saleable_tonne: 80.0,
            energy_kwh_per_saleable_tonne: 110.0 / 42.0,
            water_m3_per_saleable_tonne: 0.2,
            emissions_kg_co2e_per_saleable_tonne: 4.0,
        },
    };

    let result = evaluate(&case);
    println!("MINE-SYSTEM-001 reference evaluator");
    println!("scenario: {}", case.scenario_id);
    println!("disposition: {}", result.disposition);
    println!(
        "mass residual (kg): {:.9}",
        result.mass_residual_kg.unwrap_or(f64::NAN)
    );
    println!(
        "energy residual (kWh): {:.9}",
        result.energy_residual_kwh.unwrap_or(f64::NAN)
    );
    println!(
        "accepted output (synthetic t/shift): {:.3}",
        case.metrics.accepted_tonnes_per_shift
    );
    println!("claim ceiling: synthetic contract fixture only");

    for reason in result.reasons {
        println!("reason: {reason}");
    }

    if result.disposition != mine_system_benchmark::Disposition::Eligible {
        std::process::exit(1);
    }
}
