// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use mine_system_benchmark::cascade::{
    simulate, DependencyRule, Event, EventKind, LocalState, Scenario, Service,
};

fn service(id: &str, domain: &str) -> Service {
    Service {
        id: id.to_owned(),
        local_state: LocalState::Available,
        capacity: 10.0,
        minimum_capacity: 5.0,
        evidence_current: true,
        failure_domain: Some(domain.to_owned()),
        requirements: Vec::new(),
        critical: false,
    }
}

fn scenario(shared_pump_power: bool) -> Scenario {
    let mut pump_a = service("pump-a", "power-a");
    let mut pump_b = service(
        "pump-b",
        if shared_pump_power { "power-a" } else { "power-b" },
    );
    pump_a.critical = false;
    pump_b.critical = false;

    let mut dewatering = Service {
        id: "dewatering-service".to_owned(),
        local_state: LocalState::Available,
        capacity: 10.0,
        minimum_capacity: 5.0,
        evidence_current: true,
        failure_domain: None,
        requirements: vec![DependencyRule::AtLeast {
            service_ids: vec!["pump-a".to_owned(), "pump-b".to_owned()],
            minimum_viable: 1,
        }],
        critical: true,
    };

    // Keep both configurations identical except for the declared common-cause
    // power dependency. These fixtures are semantic controls, not physical data.
    dewatering.critical = true;

    Scenario {
        id: if shared_pump_power {
            "SYNTHETIC-SHARED-POWER".to_owned()
        } else {
            "SYNTHETIC-INDEPENDENT-POWER".to_owned()
        },
        horizon_tick: 12,
        max_services: 16,
        max_events: 16,
        services: vec![pump_a, pump_b, dewatering],
        events: vec![
            Event {
                at_tick: 4,
                id: "power-a-lost".to_owned(),
                kind: EventKind::SetFailureDomainAvailable {
                    domain: "power-a".to_owned(),
                    available: false,
                },
            },
            Event {
                at_tick: 9,
                id: "power-a-restored".to_owned(),
                kind: EventKind::SetFailureDomainAvailable {
                    domain: "power-a".to_owned(),
                    available: true,
                },
            },
        ],
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Synthetic mine-service cascade demo; no physical mine claim.");
    for shared in [true, false] {
        let report = simulate(&scenario(shared))?;
        println!(
            "{}: critical unavailable service-ticks={}, final dewatering={:?}",
            report.scenario_id,
            report.critical_unavailable_service_ticks,
            report.final_dispositions["dewatering-service"],
        );
    }
    Ok(())
}
