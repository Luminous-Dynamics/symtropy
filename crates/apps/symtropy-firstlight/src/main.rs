// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::json;
use symtropy_firstlight::{canonical_service_span, run_reference_sequence};

fn main() {
    let command = std::env::args().nth(1).unwrap_or_else(|| "demo".into());
    if command != "demo" {
        eprintln!("usage: symtropy-firstlight [demo]");
        std::process::exit(2);
    }
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
