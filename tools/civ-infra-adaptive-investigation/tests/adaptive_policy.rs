use civ_infra_adaptive_investigation::{
    ActionInput, CandidatePolicy, LeafTerminal, ModelSupport, OutcomeInput, PolicyBranch,
    PolicyInput, SupportFailureTerminal, evaluate_bytes,
};
use std::collections::BTreeSet;

const FIXTURE: &[u8] = include_bytes!("../fixtures/adaptive-investigation-v0.1.json");

fn report_from(bytes: &[u8]) -> civ_infra_adaptive_investigation::PolicyReport {
    evaluate_bytes(bytes).expect("fixture should satisfy the declared synthetic contract")
}

fn input_from(bytes: &[u8]) -> PolicyInput {
    serde_json::from_slice(bytes).expect("fixture should parse")
}

#[test]
fn evaluates_the_declared_adaptive_policy_and_emits_support_failure_leaves() {
    let report = report_from(FIXTURE);
    assert_eq!(
        report.world_id,
        "synthetic-rail-bridge-tunnel-adaptive-v0.1"
    );
    assert_eq!(report.state_count, 5);
    assert_eq!(report.candidate_count, 3);
    assert_eq!(report.action_count, 3);
    assert_eq!(report.policy_depth_limit, 2);
    assert_eq!(report.support_failure_leaf_count, 3);
    assert_eq!(report.supported_leaf_count, 4);

    let failures = report
        .leaves
        .iter()
        .filter(|leaf| leaf.terminal == LeafTerminal::ReviewRequired)
        .collect::<Vec<_>>();
    assert_eq!(failures.len(), 3);
    assert!(failures.iter().any(|leaf| {
        leaf.path.len() == 1
            && leaf.path[0].action_id == "bridge-screen"
            && leaf.path[0].outcome_id == "out_of_model_support"
            && leaf.state_count == 5
    }));
    assert!(
        failures
            .iter()
            .all(|leaf| leaf.state_count > 0 && leaf.decision_signature_class_count > 0)
    );
    assert!(report.claim_ceiling.contains("no_physical_measurement"));
}

#[test]
fn overlapping_outcomes_preserve_marginal_and_unresolved_worlds_on_both_branches() {
    let report = report_from(FIXTURE);
    let clear = report
        .leaves
        .iter()
        .find(|leaf| {
            leaf.terminal == LeafTerminal::DecisionLeaf
                && leaf.path.iter().any(|step| {
                    step.action_id == "bridge-screen" && step.outcome_id == "bridge-clear"
                })
                && leaf
                    .path
                    .iter()
                    .any(|step| step.action_id == "tunnel-borehole")
                && leaf.path.iter().any(|step| step.outcome_id == "tunnel-dry")
        })
        .expect("clear/dry leaf should exist");
    let concern = report
        .leaves
        .iter()
        .find(|leaf| {
            leaf.terminal == LeafTerminal::DecisionLeaf
                && leaf.path.iter().any(|step| {
                    step.action_id == "bridge-screen" && step.outcome_id == "bridge-concern"
                })
                && leaf
                    .path
                    .iter()
                    .any(|step| step.outcome_id == "bridge-indicator")
        })
        .expect("concern/indicator leaf should exist");
    assert!(clear.state_ids.iter().any(|id| id == "marginal-ground"));
    assert!(clear.state_ids.iter().any(|id| id == "unresolved-geology"));
    assert!(concern.state_ids.iter().any(|id| id == "marginal-ground"));
    assert!(
        concern
            .state_ids
            .iter()
            .any(|id| id == "unresolved-geology")
    );
    assert!(clear.decision_signature_class_count > 1);
    assert!(concern.decision_signature_class_count > 1);
}

#[test]
fn exact_input_bytes_are_bound_to_report_and_replay_is_deterministic() {
    let first = evaluate_bytes(FIXTURE).expect("first run");
    let second = evaluate_bytes(FIXTURE).expect("second run");
    assert_eq!(first, second);
    assert_eq!(
        first.input_sha256,
        civ_infra_adaptive_investigation::sha256_hex(FIXTURE)
    );
    assert_eq!(
        serde_json::to_vec(&first).expect("serialize first"),
        serde_json::to_vec(&second).expect("serialize second")
    );
}

#[test]
fn unknown_state_reference_is_rejected() {
    let mut input = input_from(FIXTURE);
    input.actions[0].outcomes[0]
        .compatible_state_ids
        .push("not-a-declared-world".to_owned());
    let bytes = serde_json::to_vec(&input).expect("serialize changed input");
    let error = evaluate_bytes(&bytes).expect_err("unknown state must fail closed");
    assert!(error.contains("unknown state"), "{error}");
}

#[test]
fn duplicate_edge_inside_one_outcome_is_rejected_but_cross_outcome_overlap_is_allowed() {
    let mut input = input_from(FIXTURE);
    input.actions[0].outcomes[0]
        .compatible_state_ids
        .push("stable-dry-ground".to_owned());
    let bytes = serde_json::to_vec(&input).expect("serialize changed input");
    let error = evaluate_bytes(&bytes).expect_err("duplicate edge must be rejected");
    assert!(error.contains("duplicate compatibility edge"), "{error}");

    let input = input_from(FIXTURE);
    let bridge_screen = &input.actions[0];
    let clear = &bridge_screen.outcomes[0].compatible_state_ids;
    let concern = &bridge_screen.outcomes[1].compatible_state_ids;
    assert!(clear.contains(&"marginal-ground".to_owned()));
    assert!(concern.contains(&"marginal-ground".to_owned()));
}

#[test]
fn policy_cycle_is_rejected() {
    let mut input = input_from(FIXTURE);
    input
        .candidate_policy
        .branches
        .push(civ_infra_adaptive_investigation::PolicyBranch {
            at_action_id: "tunnel-borehole".to_owned(),
            outcome_id: "tunnel-dry".to_owned(),
            next_action_id: "bridge-screen".to_owned(),
        });
    input.candidate_policy.max_supported_action_depth = 3;
    let bytes = serde_json::to_vec(&input).expect("serialize changed input");
    let error = evaluate_bytes(&bytes).expect_err("cycle must fail closed");
    assert!(error.contains("cycle"), "{error}");
}

#[test]
fn branch_to_unknown_outcome_is_rejected() {
    let mut input = input_from(FIXTURE);
    input.candidate_policy.branches[0].outcome_id = "invented-observation".to_owned();
    let bytes = serde_json::to_vec(&input).expect("serialize changed input");
    let error = evaluate_bytes(&bytes).expect_err("branch outcome must exist");
    assert!(error.contains("unknown outcome"), "{error}");
}

#[test]
fn leaf_discordant_pair_count_matches_independent_small_case_oracle() {
    let report = report_from(FIXTURE);
    let leaf = report
        .leaves
        .iter()
        .find(|leaf| {
            leaf.terminal == LeafTerminal::DecisionLeaf
                && leaf
                    .path
                    .iter()
                    .any(|step| step.outcome_id == "bridge-indicator")
        })
        .expect("bridge-indicator leaf");
    let input = input_from(FIXTURE);
    let state_map = input
        .state_signatures
        .iter()
        .map(|state| (state.state_id.as_str(), &state.signature))
        .collect::<std::collections::BTreeMap<_, _>>();
    let ids = leaf
        .state_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut signatures = BTreeSet::new();
    let mut discordant_pairs = 0_u64;
    for i in 0..ids.len() {
        signatures.insert(
            serde_json::to_string(state_map.get(ids[i]).expect("state")).expect("signature"),
        );
        for j in i + 1..ids.len() {
            let left = state_map.get(ids[i]).expect("left state");
            let right = state_map.get(ids[j]).expect("right state");
            if left != right {
                discordant_pairs += 1;
            }
        }
    }
    assert_eq!(leaf.decision_discordant_pairs, discordant_pairs);
    assert_eq!(leaf.decision_signature_class_count, signatures.len());
}

#[test]
fn out_of_model_support_never_drops_unknown_design_states() {
    let report = report_from(FIXTURE);
    let failure = report
        .leaves
        .iter()
        .find(|leaf| {
            leaf.terminal == LeafTerminal::ReviewRequired
                && leaf.path.len() == 1
                && leaf.path[0].outcome_id == "out_of_model_support"
                && leaf.path[0].action_id == "bridge-screen"
        })
        .expect("root support failure");
    let active = failure.state_ids.iter().cloned().collect::<BTreeSet<_>>();
    assert!(active.contains("unresolved-geology"));
    assert_eq!(failure.state_count, 5);
}

#[test]
fn nested_support_failure_preserves_exactly_the_active_state_set() {
    let mut input = input_from(FIXTURE);
    let tunnel = input
        .actions
        .iter_mut()
        .find(|action| action.action_id == "tunnel-borehole")
        .expect("tunnel-borehole action");
    tunnel.model_support = civ_infra_adaptive_investigation::ModelSupport::Open;

    let bytes = serde_json::to_vec(&input).expect("serialize changed input");
    let report = evaluate_bytes(&bytes).expect("nested support failure should be represented");

    let leaf = report
        .leaves
        .iter()
        .find(|leaf| {
            leaf.terminal == LeafTerminal::ReviewRequired
                && leaf.path.len() == 2
                && leaf.path[0].action_id == "bridge-screen"
                && leaf.path[0].outcome_id == "bridge-clear"
                && leaf.path[1].action_id == "tunnel-borehole"
                && leaf.path[1].outcome_id == "out_of_model_support"
        })
        .expect("nested review leaf after bridge-clear");
    assert_eq!(
        leaf.state_ids,
        vec![
            "marginal-ground".to_owned(),
            "stable-dry-ground".to_owned(),
            "tunnel-groundwater-risk".to_owned(),
            "unresolved-geology".to_owned(),
        ]
    );
    assert_eq!(leaf.state_count, 4);
    assert!(!leaf.state_ids.iter().any(|id| id == "bridge-scour-risk"));
}

#[test]
fn semantic_report_is_invariant_to_record_order_while_digest_tracks_exact_bytes() {
    let baseline = report_from(FIXTURE);
    let mut input = input_from(FIXTURE);

    input.candidate_ids.reverse();
    input.state_signatures.reverse();
    for state in &mut input.state_signatures {
        state.signature.feasible_candidates.reverse();
        state.signature.infeasible_candidates.reverse();
        state.signature.unknown_candidates.reverse();
        state.signature.frontier_candidates.reverse();
    }
    input.actions.reverse();
    for action in &mut input.actions {
        action.outcomes.reverse();
        for outcome in &mut action.outcomes {
            outcome.compatible_state_ids.reverse();
        }
    }
    input.candidate_policy.branches.reverse();

    let permuted_bytes = serde_json::to_vec(&input).expect("permuted input serializes");
    let mut permuted_report =
        evaluate_bytes(&permuted_bytes).expect("permuted input remains semantically valid");
    assert_ne!(baseline.input_sha256, permuted_report.input_sha256);

    // Semantic equivalence does not erase provenance: only normalize the digest for comparison.
    permuted_report.input_sha256 = baseline.input_sha256.clone();
    assert_eq!(baseline, permuted_report);
}

#[test]
fn exponential_policy_expansion_fails_closed_at_the_work_budget() {
    const LAYERS: usize = 6;
    const WIDTH: usize = 12;

    let mut input = input_from(FIXTURE);
    let mut actions = Vec::new();
    let mut branches = Vec::new();
    for layer in 0..LAYERS {
        for index in 0..WIDTH {
            let action_id = format!("layer-{layer:02}-action-{index:02}");
            let outcomes = (0..WIDTH)
                .map(|outcome_index| OutcomeInput {
                    outcome_id: format!("outcome-{outcome_index:02}"),
                    compatible_state_ids: vec!["stable-dry-ground".to_owned()],
                })
                .collect();

            if layer + 1 < LAYERS {
                for outcome_index in 0..WIDTH {
                    branches.push(PolicyBranch {
                        at_action_id: action_id.clone(),
                        outcome_id: format!("outcome-{outcome_index:02}"),
                        next_action_id: format!(
                            "layer-{:02}-action-{:02}",
                            layer + 1,
                            outcome_index
                        ),
                    });
                }
            }

            actions.push(ActionInput {
                action_id: action_id.clone(),
                description: format!("Synthetic branching action {action_id}"),
                compatibility_report_ref: format!("synthetic:policy-budget#{action_id}"),
                compatibility_report_sha256: None,
                model_support: ModelSupport::Open,
                outcomes,
            });
        }
    }
    input.actions = actions;
    input.candidate_policy = CandidatePolicy {
        root_action_id: "layer-00-action-00".to_owned(),
        branches,
        support_failure_terminal: SupportFailureTerminal::ReviewRequired,
        max_supported_action_depth: LAYERS,
    };

    let bytes = serde_json::to_vec(&input).expect("branching policy serializes");
    let error = evaluate_bytes(&bytes).expect_err("exponential expansion must stop at a budget");
    assert!(
        error.contains("policy expansion exceeds leaf budget")
            || error.contains("policy evaluation exceeds operation budget"),
        "{error}"
    );
}

#[test]
fn excessive_leaf_state_memberships_fail_closed() {
    let mut input = input_from(FIXTURE);
    let base_state = input
        .state_signatures
        .first()
        .expect("fixture has at least one state")
        .clone();
    let mut states = Vec::new();
    for index in 0..1_000usize {
        let mut state = base_state.clone();
        state.state_id = format!("state-{index:04}");
        state.upstream_case_ref = format!("synthetic:membership-budget#{index:04}");
        states.push(state);
    }
    let state_ids = states
        .iter()
        .map(|state| state.state_id.clone())
        .collect::<Vec<_>>();
    input.state_signatures = states;
    input.actions = vec![ActionInput {
        action_id: "wide-root".to_owned(),
        description: "Synthetic wide report membership-budget control".to_owned(),
        compatibility_report_ref: "synthetic:membership-budget#wide-root".to_owned(),
        compatibility_report_sha256: None,
        model_support: ModelSupport::Open,
        outcomes: (0..101usize)
            .map(|index| OutcomeInput {
                outcome_id: format!("outcome-{index:03}"),
                compatible_state_ids: state_ids.clone(),
            })
            .collect(),
    }];
    input.candidate_policy = CandidatePolicy {
        root_action_id: "wide-root".to_owned(),
        branches: Vec::new(),
        support_failure_terminal: SupportFailureTerminal::ReviewRequired,
        max_supported_action_depth: 1,
    };

    let bytes = serde_json::to_vec(&input).expect("wide policy serializes");
    let error = evaluate_bytes(&bytes).expect_err("oversized report must not be returned");
    assert!(
        error.contains("policy expansion exceeds state-membership budget"),
        "{error}"
    );
}
