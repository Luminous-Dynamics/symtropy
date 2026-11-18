use sim_rel_joint_stochastic_oracle::episode::{
    EpisodeCommand, EpisodeConfig, EpisodeEvaluation, EpisodeFinishDisposition, EpisodeManifest,
    EpisodeProvider, EpisodeRequest, EpisodeResponse, PublicActionDescription, PublicHypothesis,
    PublicObservationDescription, PublicResearchProfile,
};
use sim_rel_joint_stochastic_oracle::{JointKernel, JointMass, JointOutcome, KernelRow, Rational};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn fixture_provider(seed: u64) -> EpisodeProvider {
    let kernel = JointKernel {
        hypothesis_ids: vec!["candidate-a".to_owned(), "candidate-b".to_owned()],
        physical_state_count: 1,
        sensor_state_count: 1,
        action_ids: vec!["measure-a".to_owned(), "measure-b".to_owned()],
        observation_ids: vec!["zero".to_owned(), "one".to_owned()],
        action_costs: vec![1, 1],
        initial_belief: vec![
            JointMass {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                mass: Rational::new(1, 2).unwrap(),
            },
            JointMass {
                hypothesis: 1,
                physical_state: 0,
                sensor_state: 0,
                mass: Rational::new(1, 2).unwrap(),
            },
        ],
        rows: vec![
            KernelRow {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes: vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(0),
                }],
            },
            KernelRow {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                action: 1,
                outcomes: vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(0),
                }],
            },
            KernelRow {
                hypothesis: 1,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes: vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(1),
                }],
            },
            KernelRow {
                hypothesis: 1,
                physical_state: 0,
                sensor_state: 0,
                action: 1,
                outcomes: vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(1),
                }],
            },
        ],
    };
    let manifest = EpisodeManifest {
        contract_sha256: "a".repeat(64),
        world_model_sha256: "b".repeat(64),
        model_profile_sha256: "c".repeat(64),
        world_recipe_sha256: "d".repeat(64),
        clock_profile_sha256: "e".repeat(64),
        time_origin_sha256: "f".repeat(64),
        public_profile: PublicResearchProfile {
            task_description:
                "Identify which declared candidate mechanism matches the categorical outcomes."
                    .to_owned(),
            candidate_hypotheses: vec![
                PublicHypothesis {
                    hypothesis_id: "candidate-a".to_owned(),
                    structural_family: "process-fixture".to_owned(),
                    description: "Candidate A yields zero for both measurements.".to_owned(),
                },
                PublicHypothesis {
                    hypothesis_id: "candidate-b".to_owned(),
                    structural_family: "process-fixture".to_owned(),
                    description: "Candidate B yields one for both measurements.".to_owned(),
                },
            ],
            actions: vec![
                PublicActionDescription {
                    action_id: "measure-a".to_owned(),
                    description: "Measure the primary signal.".to_owned(),
                    target: "primary-signal".to_owned(),
                },
                PublicActionDescription {
                    action_id: "measure-b".to_owned(),
                    description: "Measure the replicate signal.".to_owned(),
                    target: "replicate-signal".to_owned(),
                },
            ],
            observations: vec![
                PublicObservationDescription {
                    observation_id: "zero".to_owned(),
                    channel_id: "signal".to_owned(),
                    description: "Instrument returned the zero category.".to_owned(),
                    unit: "categorical".to_owned(),
                },
                PublicObservationDescription {
                    observation_id: "one".to_owned(),
                    channel_id: "signal".to_owned(),
                    description: "Instrument returned the one category.".to_owned(),
                    unit: "categorical".to_owned(),
                },
            ],
        },
    };
    EpisodeProvider::new(
        format!("process-baseline-fixture-{seed:08}"),
        manifest,
        kernel,
        EpisodeConfig {
            max_attempts: 2,
            max_cost_units: 2,
            max_world_steps: 2,
        },
        seed,
    )
    .unwrap()
}

fn run_child(strategy: &str, seed: u64) -> EpisodeEvaluation {
    let executable = env!("CARGO_BIN_EXE_episode_baseline_agent");
    let mut child = Command::new(executable)
        .arg("--strategy")
        .arg(strategy)
        .arg("--seed")
        .arg(seed.to_string())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn baseline policy child");
    let mut child_stdin = child.stdin.take().expect("child stdin");
    let child_stdout = child.stdout.take().expect("child stdout");
    let mut child_stdout = BufReader::new(child_stdout);
    let mut provider = fixture_provider(seed);

    loop {
        let request: EpisodeRequest = provider.request();
        let frame = serde_json::to_vec(&request).expect("serialize public request");
        child_stdin.write_all(&frame).expect("write request");
        child_stdin.write_all(b"\n").expect("write JSONL delimiter");
        child_stdin.flush().expect("flush request");

        let mut response_frame = String::new();
        let count = child_stdout
            .read_line(&mut response_frame)
            .expect("read child response");
        assert!(count > 0, "baseline child exited before responding");
        let response: EpisodeResponse =
            serde_json::from_str(&response_frame).expect("parse typed policy response");
        let finishing = matches!(&response.command, EpisodeCommand::Finish { .. });
        provider
            .apply_response(response)
            .expect("apply policy command in trusted runner");

        if finishing {
            drop(child_stdin);
            let status = child.wait().expect("wait for child policy");
            assert!(status.success(), "baseline process must exit cleanly");
            break;
        }
    }

    provider
        .evaluate_after_finish()
        .expect("evaluate exact candidate set after child exits")
}

#[test]
fn fixed_order_baseline_runs_through_real_jsonl_child_and_exact_evaluator() {
    let evaluation = run_child("fixed_order", 3);
    assert_eq!(evaluation.applied_interventions, 2);
    assert_eq!(evaluation.total_requests, 2);
    assert_eq!(
        evaluation.reported_disposition,
        EpisodeFinishDisposition::Ambiguous
    );
    assert!(!evaluation.version_space_exact_match);
    assert!(!evaluation.disposition_correct);
    assert_eq!(evaluation.correct_candidate_count, 1);
}

#[test]
fn no_intervention_baseline_uses_same_wire_and_evaluator_custody() {
    let evaluation = run_child("no_intervention", 3);
    assert_eq!(evaluation.applied_interventions, 0);
    assert_eq!(evaluation.total_requests, 0);
    assert_eq!(
        evaluation.reported_disposition,
        EpisodeFinishDisposition::Abstain
    );
    assert!(evaluation.safe_abstention);
    assert!(evaluation.version_space_exact_match);
    assert!(evaluation.disposition_correct);
}
