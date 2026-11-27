use maritime_capability_to_service::kite::Gate;
use maritime_capability_to_service::kite_oracle::DecisionContext;
use maritime_capability_to_service::policy_tree::{
    validate_non_anticipative_policy_tree, PlannedStage, PolicyTreeAction, ScenarioPolicyPath,
};

fn context() -> DecisionContext {
    DecisionContext {
        instance_id: "R1-MULTISTAGE-TINY-001".to_owned(),
        instance_version: "v0.2".to_owned(),
        service_id: "RECEIVER-SERVICE".to_owned(),
        service_deadline: 300,
        expected_units: 10,
    }
}

fn action(id: &str) -> PolicyTreeAction {
    PolicyTreeAction {
        action_id: id.to_owned(),
        admissibility: Gate::Yes,
        hard_constraints_passed: true,
        protected_floor_breach_unit_hours: 0,
    }
}

fn stage(epoch: u64, cutoff: u64, digest: &str, selected: &str) -> PlannedStage {
    PlannedStage {
        decision_epoch: epoch,
        decision_cutoff: cutoff,
        observable_state_digest: digest.to_owned(),
        available_actions: vec![action("ABSTAIN"), action("LOCAL"), action("IMPORT")],
        selected_action_id: selected.to_owned(),
    }
}

fn path(id: &str, stages: Vec<PlannedStage>) -> ScenarioPolicyPath {
    ScenarioPolicyPath {
        scenario_id: id.to_owned(),
        context: context(),
        stages,
    }
}

#[test]
fn public_validator_allows_adaptation_only_after_observable_histories_diverge() {
    let paths = vec![
        path(
            "SCENARIO-A",
            vec![
                stage(1, 100, &"a".repeat(64), "LOCAL"),
                stage(2, 200, &"b".repeat(64), "LOCAL"),
            ],
        ),
        path(
            "SCENARIO-B",
            vec![
                stage(1, 100, &"a".repeat(64), "LOCAL"),
                stage(2, 200, &"c".repeat(64), "IMPORT"),
            ],
        ),
    ];

    let report = validate_non_anticipative_policy_tree(&paths).expect("valid policy tree");
    assert_eq!(report.scenario_count, 2);
    assert_eq!(report.stage_decision_count, 4);
    assert_eq!(report.information_set_count, 3);
    assert_eq!(report.shared_information_set_count, 1);
    assert_eq!(report.maximum_scenarios_per_information_set, 2);
    assert_eq!(
        report.information_sets[0].selected_action_id,
        "LOCAL"
    );
}

#[test]
fn public_validator_rejects_a_hidden_shock_specific_early_action() {
    let paths = vec![
        path("SCENARIO-A", vec![stage(1, 100, &"a".repeat(64), "LOCAL")]),
        path("SCENARIO-B", vec![stage(1, 100, &"a".repeat(64), "IMPORT")]),
    ];

    let error = validate_non_anticipative_policy_tree(&paths).unwrap_err();
    assert!(error.contains("non-anticipativity violation"));
}
