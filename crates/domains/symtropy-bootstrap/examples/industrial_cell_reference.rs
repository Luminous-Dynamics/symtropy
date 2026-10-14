//! Deterministic reference campaign for an autonomous industrial cell.
//!
//! All quantities in this example are synthetic, dimensionless planning fixtures.
//! They are NOT lunar process-yield, power, mass, cost, or qualification data.
//! Run with:
//!   cargo run -p symtropy-bootstrap --example industrial_cell_reference

use std::collections::BTreeMap;

use symtropy_bootstrap::{
    BootstrapCandidate, Capability, Dependency, DependencyClass, DependencyGraph, EnergyEvent,
    EnergyEventKind, EnergyLedger, ProcessRun, ProductionProcess, pareto_frontier,
    verify_energy_conservation,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    println!("industrial-cell-reference/v0.1");
    println!("fixture_class=synthetic_not_physical_qualification");

    // A locally serviceable seed cell still depends on imported metrology and
    // controller electronics. The model must not let a high local-mass ratio
    // conceal those critical capability blockers.
    let graph = DependencyGraph::new(
        [
            Capability::new("survey", 20, ["power_bus", "mobility", "sensors"]),
            Capability::new(
                "bulk_handling",
                30,
                ["survey", "regolith_feedstock", "mobility"],
            ),
            Capability::new(
                "local_block_production",
                30,
                ["bulk_handling", "processing_cell"],
            ),
            Capability::new(
                "precision_repair",
                20,
                [
                    "local_block_production",
                    "metrology",
                    "control_electronics",
                    "tools",
                ],
            ),
        ],
        [
            Dependency::new("power_bus", DependencyClass::LocalClosed),
            Dependency::new("mobility", DependencyClass::LocalClosed),
            Dependency::new("sensors", DependencyClass::ReplaceableBySubstitution),
            Dependency::new("regolith_feedstock", DependencyClass::LocalClosed),
            Dependency::new("processing_cell", DependencyClass::LocalClosed),
            Dependency::new("metrology", DependencyClass::ImportedDurable),
            Dependency::new("control_electronics", DependencyClass::ImportedDurable),
            Dependency::new("tools", DependencyClass::LocalClosed),
        ],
    );

    let closure = graph.evaluate(85_000, 15_000);
    if !closure.valid {
        return Err(format!(
            "reference capability graph invalid: {:?}",
            closure.definition_errors()
        ));
    }
    if closure.mass_closure_ppm != 850_000 {
        return Err(format!(
            "unexpected synthetic mass-closure result: {} ppm",
            closure.mass_closure_ppm
        ));
    }
    if closure.critical_closure_ppm != 800_000 {
        return Err(format!(
            "critical dependency blocker was not preserved: {} ppm",
            closure.critical_closure_ppm
        ));
    }
    if closure.fully_closed() {
        return Err(
            "imported metrology/control dependencies were incorrectly treated as closed"
                .to_string(),
        );
    }

    println!("mass_closure_ppm={}", closure.mass_closure_ppm);
    println!("critical_closure_ppm={}", closure.critical_closure_ppm);
    println!("industrial_closure=BLOCKED");
    println!("unresolved_dependencies=metrology,control_electronics");

    // A synthetic feedstock transformation emits two useful streams and one
    // waste stream. Product and waste masses must account for all input mass.
    let process = ProductionProcess::new(
        "regolith-sintering-v1",
        "prepared_regolith",
        ["sintered_block", "metal_rich_fraction"],
        "slag",
    );
    let outputs = [
        ("sintered_block".to_string(), 55_000_u64),
        ("metal_rich_fraction".to_string(), 15_000_u64),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    let valid_run = ProcessRun::new(
        "regolith-sintering-v1",
        "prepared_regolith",
        "synthetic-feed-batch-001",
        100_000,
        outputs,
        30_000,
        350,
    );
    process.validate_run_against_budget(&valid_run, 100_000, 400)?;

    // Negative control: one extra gram must fail. Never round an imbalance away
    // or promote an invalid product batch into accepted inventory.
    let invalid_outputs = [
        ("sintered_block".to_string(), 55_001_u64),
        ("metal_rich_fraction".to_string(), 15_000_u64),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    let invalid_run = ProcessRun::new(
        "regolith-sintering-v1",
        "prepared_regolith",
        "synthetic-feed-batch-NEGATIVE",
        100_000,
        invalid_outputs,
        30_000,
        350,
    );
    if process.validate_run(&invalid_run).is_ok() {
        return Err("negative control accepted a mass-imbalanced process run".to_string());
    }
    if process
        .validate_run_against_budget(&valid_run, 100_000, 349)
        .is_ok()
    {
        return Err("negative control accepted insufficient process energy".to_string());
    }

    println!("valid_process_mass_balance=PASS");
    println!("imbalanced_process_negative_control=REJECTED");
    println!("underfunded_process_negative_control=REJECTED");
    println!("synthetic_feed_g=100000");
    println!("synthetic_product_g=70000");
    println!("synthetic_waste_g=30000");
    println!("synthetic_energy_units=350");

    // Audit energy through the crate's causal ledger. These units are deliberately
    // abstract fixture units rather than kWh or a lunar power prediction.
    let initial_energy = [("cell-bus".to_string(), 1_000_u64)]
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    let mut energy = EnergyLedger::new(initial_energy.clone());
    energy.append(
        EnergyEvent::new(1, "cell-bus", 350, EnergyEventKind::Consumed)
            .with_event_id("synthetic-energy-event-001")
            .with_provenance("synthetic-work-order-001"),
    )?;
    verify_energy_conservation(&initial_energy, energy.events(), energy.state())?;
    if energy.state().get("cell-bus") != Some(&650) {
        return Err("energy ledger did not preserve the expected residual".to_string());
    }
    println!("energy_ledger=PASS");
    println!("synthetic_energy_remaining_units=650");

    // A Pareto frontier preserves meaningful trade-offs instead of hiding them
    // inside one opaque autonomy or efficiency score.
    let candidates = vec![
        BootstrapCandidate::new("local-berm-kit", 10, 2, 1_500, 500, 25, 10_000)?,
        BootstrapCandidate::new("sensor-calibration", 14, 3, 2_200, 300, 18, 20_000)?,
        BootstrapCandidate::new("repair-cell", 20, 4, 5_000, 400, 30, 30_000)?,
        BootstrapCandidate::new("wasteful-import", 5, 1, 6_000, 800, 50, 60_000)?,
    ];
    let frontier = pareto_frontier(&candidates);
    let frontier_ids = frontier
        .iter()
        .map(BootstrapCandidate::id)
        .collect::<Vec<_>>();
    if frontier_ids != ["local-berm-kit", "repair-cell", "sensor-calibration"] {
        return Err(format!(
            "unexpected deterministic Pareto frontier: {frontier_ids:?}"
        ));
    }
    if frontier_ids.contains(&"wasteful-import") {
        return Err("dominated bootstrap candidate escaped the Pareto filter".to_string());
    }

    println!("pareto_frontier={}", frontier_ids.join(","));
    println!("dominated_candidate=wasteful-import:EXCLUDED");
    println!("RESULT=PASS_SYNTHETIC_REFERENCE_CONTRACTS");
    println!("CLAIM_CEILING=no lunar process or robot performance is established");
    Ok(())
}
