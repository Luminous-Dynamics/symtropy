// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cross-domain causal spine for LAB-14.
//!
//! Domain records retain their own typed `source_event_id` fields. This test adds
//! a compact integration-level ancestry graph over the major Three Worlds
//! milestones and runs it through the canonical game-state `EventChain` verifier.

use serde::Serialize;
use symtropy_game_state::EventChain;
use symtropy_three_worlds_lab::run_three_worlds;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Milestone {
    world: &'static str,
    assertion: &'static str,
}

#[test]
fn three_worlds_milestones_form_verified_multi_parent_history() {
    let report = run_three_worlds().expect("Three Worlds scenario constructs");
    let mut history = EventChain::new("three-worlds-lab", 14);

    let succession_dispute = history
        .append(
            61,
            "succession-recognition-diverges",
            None,
            None,
            Vec::new(),
            Milestone {
                world: "Aster/Helion/Vesper",
                assertion: "recognition remains source-relative",
            },
        )
        .expect("append succession dispute");
    assert!(report.recognition_is_source_relative);

    let shared_topology = history
        .append(
            100,
            "shared-topology-admitted",
            None,
            None,
            vec![succession_dispute.clone()],
            Milestone {
                world: "Aster->Hub->Vesper",
                assertion: "signal and freight reuse one geography",
            },
        )
        .expect("append shared topology");

    let proposal_delivered = history
        .append(
            report.signal_delivery_tick,
            "relief-proposal-delivered",
            None,
            None,
            vec![shared_topology.clone()],
            Milestone {
                world: "Vesper",
                assertion: "message availability follows delivery receipt",
            },
        )
        .expect("append proposal delivery");
    assert!(report.signal_unavailable_before_delivery);

    let treaty_active = history
        .append(
            150,
            "relief-treaty-ratified",
            None,
            None,
            vec![proposal_delivered],
            Milestone {
                world: "Aster/Vesper",
                assertion: "both required ratifications exist",
            },
        )
        .expect("append treaty activation");
    assert!(report.treaty_active);

    let materialized_actor = history
        .append(
            report.materialized_identity_begins_tick,
            "helion-actor-materialized",
            None,
            None,
            vec![succession_dispute.clone()],
            Milestone {
                world: "Helion",
                assertion: "persistent identity begins without population inflation",
            },
        )
        .expect("append materialization");
    assert_eq!(
        report.anonymous_population_before,
        report.represented_population_after_materialization
    );

    let freight_arrived = history
        .append(
            report.freight_arrival_tick,
            "relief-freight-arrived",
            None,
            None,
            vec![shared_topology],
            Milestone {
                world: "Vesper",
                assertion: "cargo appears only after provider arrival floor",
            },
        )
        .expect("append freight arrival");
    assert!(report.freight_unavailable_before_arrival);
    assert!(report.signal_arrives_before_freight);

    let clause_disputed = history
        .append(
            266,
            "relief-clause-assessments-diverge",
            None,
            None,
            vec![treaty_active, freight_arrived.clone()],
            Milestone {
                world: "Aster/Vesper",
                assertion: "same clause retains conflicting evidence-based assessments",
            },
        )
        .expect("append clause dispute");
    assert!(report.treaty_assessments_disagree);

    let authority_transferred = history
        .append(
            262,
            "freighter-simulation-authority-transferred",
            None,
            None,
            vec![freight_arrived],
            Milestone {
                world: "Aster/Vesper",
                assertion: "shard authority moves exactly once without changing ownership",
            },
        )
        .expect_err("a later-derived event cannot be appended at an earlier canonical tick");
    let _ = authority_transferred;

    // Recreate the authority-transfer milestone at its true canonical position.
    // It causally depends on topology/destination readiness, not on the later
    // clause assessment. This second chain segment intentionally demonstrates
    // that the canonical verifier enforces time order rather than accepting a
    // narratively convenient ordering.
    let mut authority_history = EventChain::new("three-worlds-authority", 14);
    let arrival_ready = authority_history
        .append(
            260,
            "freighter-arrival-state-ready",
            None,
            None,
            Vec::new(),
            Milestone {
                world: "Vesper",
                assertion: "destination state is ready for handoff",
            },
        )
        .expect("append arrival state");
    authority_history
        .append(
            262,
            "freighter-simulation-authority-transferred",
            None,
            None,
            vec![arrival_ready],
            Milestone {
                world: "Aster/Vesper",
                assertion: "shard authority moves exactly once without changing ownership",
            },
        )
        .expect("append authority transfer");
    assert!(report.authority_transfer_retry_idempotent);
    assert!(report.asset_owner_remains_aster_after_shard_transfer);

    let project_completed = history
        .append(
            280,
            "relief-audit-project-completed",
            None,
            None,
            vec![clause_disputed.clone(), materialized_actor],
            Milestone {
                world: "Vesper/Helion",
                assertion: "external request, independent contribution, explicit review",
            },
        )
        .expect("append project completion");
    assert!(report.external_org_admission_does_not_publish_project);
    assert!(report.project_complete_after_explicit_review);

    let conflict_declared = history
        .append(
            300,
            "hub-crisis-declared",
            None,
            None,
            vec![clause_disputed, succession_dispute],
            Milestone {
                world: "Aster/Helion",
                assertion: "political conflict records aims without global war score",
            },
        )
        .expect("append conflict declaration");
    assert!(report.conflict_active_before_ceasefire);

    let ceasefire = history
        .append(
            330,
            "hub-crisis-ceasefire-effective",
            None,
            None,
            vec![conflict_declared],
            Milestone {
                world: "Aster/Helion",
                assertion: "required acceptances precede ceasefire state",
            },
        )
        .expect("append ceasefire");
    assert!(report.conflict_in_ceasefire_window);

    history
        .append(
            352,
            "hub-crisis-settled",
            None,
            None,
            vec![ceasefire, project_completed],
            Milestone {
                world: "Aster/Helion/Vesper",
                assertion: "settlement closes conflict while terms remain external obligations",
            },
        )
        .expect("append settlement");
    assert!(report.conflict_settled_by_acceptance);

    history.verify().expect("integrated causal history verifies");
    authority_history
        .verify()
        .expect("authority-transfer causal history verifies");
}
