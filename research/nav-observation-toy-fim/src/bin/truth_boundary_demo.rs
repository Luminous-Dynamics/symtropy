use nav_observation_toy_fim::truth_boundary::{
    controller_propose, controller_decide_after_acquisition, score_evaluation, AcquisitionEvent, AcquisitionStatus,
    CandidateAction, ControllerVisibleInput, EvaluatorTruth, ResourceBudget,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let visible = ControllerVisibleInput {
        policy_revision: "nav-policy-v1".to_owned(),
        input_commitment: [0x11; 32],
        deterministic_seed: 7,
        belief_state_one_ppm: 800_000,
        false_safe_loss_micros: 10,
        false_degraded_loss_micros: 1,
        budget: ResourceBudget {
            energy_units: 4,
            latency_ticks: 10,
            bandwidth_units: 2,
        },
        candidates: vec![
            CandidateAction {
                id: "uwb-reobserve".to_owned(),
                energy_units: 2,
                latency_ticks: 4,
                bandwidth_units: 1,
                expected_decision_value_micros: 120,
            },
            CandidateAction {
                id: "camera-reobserve".to_owned(),
                energy_units: 3,
                latency_ticks: 5,
                bandwidth_units: 1,
                expected_decision_value_micros: 120,
            },
        ],
    };

    let proposal = controller_propose(&visible)?;
    let proposal_bytes = proposal.canonical_bytes();
    let action_id = match &proposal.disposition {
        nav_observation_toy_fim::truth_boundary::ProposalDisposition::Acquire { action_id } => action_id.clone(),
        nav_observation_toy_fim::truth_boundary::ProposalDisposition::Abstain { .. } => {
            return Err("unexpected abstention in known-answer demo".into());
        }
    };
    let acquisition = AcquisitionEvent {
        action_id,
        proposal_bytes: proposal_bytes.clone(),
        status: AcquisitionStatus::Succeeded {
            outcome_state_one_ppm: 700_000,
        },
    };

    let truth_one = EvaluatorTruth {
        service_state_one: true,
        truth_manifest_commitment: [0xaa; 32],
    };
    let truth_zero = EvaluatorTruth {
        service_state_one: false,
        truth_manifest_commitment: [0xbb; 32],
    };
    let controller_decision = controller_decide_after_acquisition(&proposal, &acquisition)?;
    let score_one = score_evaluation(&controller_decision, &truth_one, 10, 1)?;
    let score_zero = score_evaluation(&controller_decision, &truth_zero, 10, 1)?;
    println!("post_acquisition_claim_state_one={}", controller_decision.claim_state_one);
    println!("decision_confidence_ppm={:?}", controller_decision.decision_confidence_ppm);

    println!("proposal_bytes_len={}", proposal_bytes.len());
    println!("proposal_action={}", match proposal.disposition {
        nav_observation_toy_fim::truth_boundary::ProposalDisposition::Acquire { ref action_id } => action_id.as_str(),
        _ => "abstain",
    });
    println!("truth_swap_same_proposal_bytes=true");
    println!("truth_one_correct={} loss={}", score_one.correct, score_one.loss_micros);
    println!("truth_zero_correct={} loss={}", score_zero.correct, score_zero.loss_micros);
    println!("scope=in_process_type_boundary_test_not_os_sandbox_or_formal_noninterference_proof");
    Ok(())
}
