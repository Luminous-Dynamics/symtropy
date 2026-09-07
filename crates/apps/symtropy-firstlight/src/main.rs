// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

mod patch_conduit;

use patch_conduit::{patch_conduit_reference_facts, PatchConduitScenario};
use serde_json::json;
use symtropy_firstlight::{canonical_service_span, run_reference_sequence};

fn main() {
    let command = std::env::args().nth(1).unwrap_or_else(|| "demo".into());
    match command.as_str() {
        "demo" => run_demo(),
        "patch-conduit" => run_patch_conduit(),
        _ => {
            eprintln!("usage: symtropy-firstlight [demo|patch-conduit]");
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
    let result = PatchConduitScenario::canonical()
        .and_then(|scenario| {
            let facts = patch_conduit_reference_facts()?;
            let result = scenario.evaluate(&facts)?;
            let verified = scenario
                .verified_approaches(&result)?
                .into_iter()
                .map(|approach| {
                    json!({
                        "assembly": approach.subject.id.as_str(),
                        "plan": approach.plan.id.stable_id().as_str(),
                        "temporary_work": approach
                            .temporary_works
                            .iter()
                            .map(|work| format!("{:?}", work.kind))
                            .collect::<Vec<_>>(),
                    })
                })
                .collect::<Vec<_>>();
            let conditional = scenario
                .conditional_approaches(&result)?
                .into_iter()
                .map(|approach| approach.subject.id.as_str().to_owned())
                .collect::<Vec<_>>();
            Ok(json!({
                "functional_design": scenario.design.id.stable_id().as_str(),
                "design_revision": scenario.design.revision,
                "search_exhaustive": result.is_exhaustive(),
                "verified": verified,
                "conditional": conditional,
                "rejected_combinations": result.rejected_combinations,
            }))
        });

    match result {
        Ok(summary) => println!(
            "{}",
            serde_json::to_string_pretty(&summary).expect("serialize Patch Conduit summary")
        ),
        Err(error) => {
            eprintln!("Patch Conduit reference proof failed: {error}");
            std::process::exit(1);
        }
    }
}
