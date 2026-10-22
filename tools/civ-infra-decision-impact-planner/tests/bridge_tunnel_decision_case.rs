use civ_infra_decision_impact_planner::{plan, sha256_hex, PlannerInput};

const FIXTURE: &[u8] = include_bytes!("../fixtures/bridge-tunnel-decision-impact-v0.1.json");

fn run_fixture() -> civ_infra_decision_impact_planner::PlanReport {
    let input: PlannerInput = serde_json::from_slice(FIXTURE).expect("fixture must parse");
    let digest = sha256_hex(FIXTURE);
    plan(&input, &digest).expect("declared finite bridge/tunnel case must validate")
}

#[test]
fn targeted_investigation_ranking_matches_hand_checkable_reference() {
    let report = run_fixture();

    assert_eq!(report.world_id, "synthetic-bridge-tunnel-corridor-v0.1");
    assert_eq!(report.state_count, 4);
    assert_eq!(report.candidate_count, 3);
    assert_eq!(report.decision_signature_classes_before_observation, 4);
    assert_eq!(
        report
            .actions_ranked
            .iter()
            .map(|action| action.action_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "tunnel-groundwater-borehole",
            "bridge-scour-survey",
            "desktop-review-no-discrimination"
        ]
    );

    let tunnel = &report.actions_ranked[0].impact;
    assert_eq!(tunnel.worst_case_decision_signature_classes, 2);
    assert_eq!(tunnel.worst_case_decision_discordant_pairs_remaining, 1);
    assert_eq!(tunnel.worst_case_remaining_states, 2);
    assert_eq!(tunnel.total_decision_discordant_pairs_before_observation, 6);
    assert_eq!(tunnel.decision_discordant_pairs_separated, 4);

    let bridge = &report.actions_ranked[1].impact;
    assert_eq!(bridge.worst_case_decision_signature_classes, 3);
    assert_eq!(bridge.worst_case_decision_discordant_pairs_remaining, 3);
    assert_eq!(bridge.worst_case_remaining_states, 3);
    assert_eq!(bridge.decision_discordant_pairs_separated, 3);

    let desktop = &report.actions_ranked[2].impact;
    assert_eq!(desktop.worst_case_decision_signature_classes, 4);
    assert_eq!(desktop.worst_case_decision_discordant_pairs_remaining, 6);
    assert_eq!(desktop.worst_case_remaining_states, 4);
    assert_eq!(desktop.decision_discordant_pairs_separated, 0);

    assert!(report.claim_ceiling.contains("no_probability"));
    assert!(report.claim_ceiling.contains("physical_validity"));
}

#[test]
fn incomplete_investigation_outcomes_fail_closed() {
    let mut input: PlannerInput = serde_json::from_slice(FIXTURE).expect("fixture must parse");

    let outcome = input.actions[0]
        .outcomes
        .iter_mut()
        .find(|outcome| outcome.outcome_id == "no-specified-scour-indicator")
        .expect("expected bridge negative-control outcome");
    outcome.state_ids.retain(|state_id| state_id != "unresolved-deep-geology");

    let mutated_bytes = serde_json::to_vec(&input).expect("mutated input must serialize");
    let digest = sha256_hex(&mutated_bytes);
    let error = plan(&input, &digest).expect_err("omitted state must block planner run");

    assert!(error.contains("does not cover every state"), "{error}");
    assert!(error.contains("unresolved-deep-geology"), "{error}");
}
