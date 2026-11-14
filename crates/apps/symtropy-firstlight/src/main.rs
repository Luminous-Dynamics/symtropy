// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

mod patch_conduit;
mod patch_conduit_diagnostics;
mod patch_conduit_exact_plan;
mod patch_conduit_execution;
mod patch_conduit_staged_execution;

use patch_conduit::{PatchConduitScenario, patch_conduit_reference_facts};
use patch_conduit_diagnostics::run_reference_pressure_diagnostics;
use patch_conduit_exact_plan::ExactPatchConduitProfile;
use patch_conduit_execution::PatchConduitExecutionProfile;
use patch_conduit_staged_execution::run_reference_bypass_execution;
use serde_json::json;
use symtropy_firstlight::{canonical_service_span, run_reference_sequence};

fn main() {
    let command = std::env::args().nth(1).unwrap_or_else(|| "demo".into());
    match command.as_str() {
        "demo" => run_demo(),
        "patch-conduit" => run_patch_conduit(),
        "patch-conduit-exact-plan" => run_patch_conduit_exact_plan(),
        "patch-conduit-execute" => run_patch_conduit_execute(),
        "patch-conduit-diagnose" => run_patch_conduit_diagnose(),
        _ => {
            eprintln!(
                "usage: symtropy-firstlight [demo|patch-conduit|patch-conduit-exact-plan|patch-conduit-execute|patch-conduit-diagnose]"
            );
            std::process::exit(2);
        }
    }
}

fn run_demo() {
    match run_reference_sequence(42) {
        Ok(session) => {
            let route = session
                .world
                .crawler
                .qualify_route(&canonical_service_span(42));
            let summary = json!({
                "content_version": symtropy_firstlight::FIRSTLIGHT_CONTENT_VERSION,
                "phase": format!("{:?}", session.world.phase),
                "simulation_tick": session.world.clock.tick(),
                "bent_feeder_outcome": format!("{:?}", session.world.bent_feeder.outcome),
                "catastrophe_phase": format!("{:?}", session.world.catastrophe.phase),
                "failed_domains": session.world.catastrophe.failed_domains(),
                "crawler_gross_mass_kg": session.world.crawler.gross_mass_kg(),
                "service_span_class": format!("{:?}", route.class),
                "event_head": session.events.head_hash(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&summary).expect("serialize summary")
            );
        }
        Err(error) => {
            eprintln!("Firstlight reference sequence failed: {error}");
            std::process::exit(1);
        }
    }
}

fn run_patch_conduit() {
    let scenario = match PatchConduitScenario::canonical() {
        Ok(scenario) => scenario,
        Err(error) => {
            eprintln!("Patch Conduit reference proof failed: {error}");
            std::process::exit(1);
        }
    };
    let execution = match PatchConduitExecutionProfile::compile(&scenario) {
        Ok(execution) => execution,
        Err(error) => {
            eprintln!("Patch Conduit executable-contract compilation failed: {error}");
            std::process::exit(1);
        }
    };
    let facts = match patch_conduit_reference_facts() {
        Ok(facts) => facts,
        Err(error) => {
            eprintln!("Patch Conduit reference evidence failed: {error}");
            std::process::exit(1);
        }
    };
    let result = match scenario.evaluate(&facts) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("Patch Conduit functional evaluation failed: {error}");
            std::process::exit(1);
        }
    };
    let resolved = match scenario.verified_approaches(&result) {
        Ok(resolved) => resolved,
        Err(error) => {
            eprintln!("Patch Conduit verified-plan resolution failed: {error}");
            std::process::exit(1);
        }
    };

    let mut verified = Vec::with_capacity(resolved.len());
    for approach in resolved {
        let Some(executable) = execution.approach_for_subject(&approach.subject) else {
            eprintln!(
                "Patch Conduit executable profile is missing {}",
                approach.subject.id
            );
            std::process::exit(1);
        };
        verified.push(json!({
            "assembly": approach.subject.id.as_str(),
            "plan": executable.plan.id.stable_id().as_str(),
            "plan_revision": executable.plan.revision,
            "process_steps": executable.plan.steps().len(),
            "temporary_work": executable
                .temporary_works
                .iter()
                .map(|work| format!("{:?}", work.kind))
                .collect::<Vec<_>>(),
        }));
    }

    let conditional = match scenario.conditional_approaches(&result) {
        Ok(approaches) => approaches
            .into_iter()
            .map(|approach| approach.subject.id.as_str().to_owned())
            .collect::<Vec<_>>(),
        Err(error) => {
            eprintln!("Patch Conduit conditional-plan resolution failed: {error}");
            std::process::exit(1);
        }
    };

    let summary = json!({
        "functional_design": scenario.design.id.stable_id().as_str(),
        "design_revision": scenario.design.revision,
        "search_exhaustive": result.is_exhaustive(),
        "verified": verified,
        "conditional": conditional,
        "rejected_combinations": result.rejected_combinations,
        "process_contracts": execution.catalog.processes().len(),
        "capability_needs": execution.catalog.capability_needs().len(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).expect("serialize Patch Conduit summary")
    );
}

fn run_patch_conduit_exact_plan() {
    let scenario = match PatchConduitScenario::canonical() {
        Ok(scenario) => scenario,
        Err(error) => {
            eprintln!("Patch Conduit exact-plan scenario failed: {error}");
            std::process::exit(1);
        }
    };
    let exact = match ExactPatchConduitProfile::compile(&scenario) {
        Ok(exact) => exact,
        Err(error) => {
            eprintln!("Patch Conduit exact-plan projection failed: {error}");
            std::process::exit(1);
        }
    };

    let approaches = exact
        .approaches()
        .iter()
        .map(|approach| {
            json!({
                "assembly": approach.subject.id.as_str(),
                "plan": approach.plan.plan().id.stable_id().as_str(),
                "plan_revision": approach.plan.plan().revision,
                "exact_process_bindings": approach.plan.process_bindings().len(),
                "temporary_work_contracts": approach.temporary_works.len(),
            })
        })
        .collect::<Vec<_>>();
    let summary = json!({
        "approaches": approaches,
        "total_exact_steps": exact.total_exact_steps(),
        "runtime_execution_claimed": false,
        "commissioning_claimed": false,
        "authorization_claimed": false,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).expect("serialize Patch Conduit exact-plan summary")
    );
}

fn run_patch_conduit_execute() {
    match run_reference_bypass_execution() {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .expect("serialize Patch Conduit staged-execution report")
        ),
        Err(error) => {
            eprintln!("Patch Conduit staged execution failed: {error}");
            std::process::exit(1);
        }
    }
}

fn run_patch_conduit_diagnose() {
    match run_reference_pressure_diagnostics() {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .expect("serialize Patch Conduit diagnostic report")
        ),
        Err(error) => {
            eprintln!("Patch Conduit diagnostic proof failed: {error}");
            std::process::exit(1);
        }
    }
}
