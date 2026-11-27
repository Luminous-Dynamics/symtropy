// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use mobility_rrvr_aerb_evaluator::{
    AcknowledgementState, AuthorityRelation, CandidateEpisode, CandidateSubmission, ClockStamp,
    CommandContext, DecisionLoss, DispatchState, Disposition, EffectState, EvaluatorWorld,
    EvaluatorWorldPack, EvidenceDomain, OPEN_WORLD_RESIDUAL, Observation, ObservationQuality,
    PermittedProbe, ProbeOutcomeSupport, ProbeSupport, SCHEMA_VERSION, SourceRole,
    WORLD_VOCABULARY_VERSION, canonical_sorted_json_sha256, sha256_hex,
};
use serde::Serialize;

const WORLD_IDS: [&str; 8] = [
    "not_dispatched",
    "dispatched_no_effect",
    "applied_once_ack_lost",
    "partial_effect",
    "duplicate_effect",
    "stale_authority_effect",
    "unresolved_command_holder",
    OPEN_WORLD_RESIDUAL,
];

const PROBE_IDS: [&str; 4] = [
    "read_endpoint_journal",
    "independent_pose_observation",
    "inventory_command_holders",
    "reconcile_effect_and_scope",
];

#[derive(Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn bounded(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u64() % upper_exclusive as u64) as usize
    }
}

fn shuffled_world_cycle(seed: u64, cycle: usize) -> Vec<&'static str> {
    let cycle_seed = seed.wrapping_add((cycle as u64).wrapping_mul(0xD134_2543_DE82_EF95));
    let mut rng = SplitMix64::new(cycle_seed);
    let mut worlds = WORLD_IDS.to_vec();
    for index in (1..worlds.len()).rev() {
        let other = rng.bounded(index + 1);
        worlds.swap(index, other);
    }
    worlds
}

fn digest_label(label: &str) -> String {
    sha256_hex(label.as_bytes())
}

fn world_record(world_id: &str) -> EvaluatorWorld {
    let (dispatch_state, effect_state, effect_count, authority_relation, acknowledgement_state) =
        match world_id {
            "not_dispatched" => (
                DispatchState::NotDispatched,
                EffectState::None,
                0,
                AuthorityRelation::NotApplicable,
                AcknowledgementState::Unknown,
            ),
            "dispatched_no_effect" => (
                DispatchState::Dispatched,
                EffectState::None,
                0,
                AuthorityRelation::CurrentEpoch,
                AcknowledgementState::Lost,
            ),
            "applied_once_ack_lost" => (
                DispatchState::Dispatched,
                EffectState::Once,
                1,
                AuthorityRelation::CurrentEpoch,
                AcknowledgementState::Lost,
            ),
            "partial_effect" => (
                DispatchState::Dispatched,
                EffectState::Partial,
                1,
                AuthorityRelation::CurrentEpoch,
                AcknowledgementState::Lost,
            ),
            "duplicate_effect" => (
                DispatchState::Dispatched,
                EffectState::Duplicate,
                2,
                AuthorityRelation::CurrentEpoch,
                AcknowledgementState::Lost,
            ),
            "stale_authority_effect" => (
                DispatchState::Dispatched,
                EffectState::StaleEpoch,
                1,
                AuthorityRelation::StaleEpoch,
                AcknowledgementState::Unknown,
            ),
            "unresolved_command_holder" => (
                DispatchState::Unknown,
                EffectState::Unknown,
                0,
                AuthorityRelation::UnresolvedHolder,
                AcknowledgementState::Unknown,
            ),
            OPEN_WORLD_RESIDUAL => (
                DispatchState::Unknown,
                EffectState::Unknown,
                0,
                AuthorityRelation::UnresolvedHolder,
                AcknowledgementState::Unknown,
            ),
            _ => unreachable!("world ID must be in the fixed catalog"),
        };

    EvaluatorWorld {
        world_id: world_id.to_owned(),
        dispatch_state,
        effect_state,
        effect_count,
        authority_relation,
        acknowledgement_state,
    }
}

fn loss_for(disposition: Disposition, world_id: &str) -> f64 {
    match disposition {
        Disposition::HoldReconcile => match world_id {
            "not_dispatched" | "dispatched_no_effect" => 0.35,
            "applied_once_ack_lost" => 0.05,
            "partial_effect" => 0.10,
            "duplicate_effect" => 0.15,
            "stale_authority_effect" | "unresolved_command_holder" => 0.25,
            OPEN_WORLD_RESIDUAL => 0.30,
            _ => unreachable!(),
        },
        Disposition::RequestObservation => 0.30,
        Disposition::Abstain => 0.45,
        Disposition::RecommendFreshAttempt => match world_id {
            "not_dispatched" | "dispatched_no_effect" => 0.0,
            "applied_once_ack_lost" | "partial_effect" => 0.95,
            "duplicate_effect" | "stale_authority_effect" | OPEN_WORLD_RESIDUAL => 1.0,
            "unresolved_command_holder" => 0.90,
            _ => unreachable!(),
        },
    }
}

fn outcome(id: &str, worlds: &[&str]) -> ProbeOutcomeSupport {
    ProbeOutcomeSupport {
        outcome_id: id.to_owned(),
        possible_world_ids: worlds.iter().map(|world| (*world).to_owned()).collect(),
    }
}

fn probe_supports() -> Vec<ProbeSupport> {
    vec![
        ProbeSupport {
            probe_id: "read_endpoint_journal".to_owned(),
            outcomes: vec![
                outcome(
                    "journal_no_effect_complete",
                    &[
                        "not_dispatched",
                        "dispatched_no_effect",
                        OPEN_WORLD_RESIDUAL,
                    ],
                ),
                outcome(
                    "journal_applied_once_complete",
                    &["applied_once_ack_lost", OPEN_WORLD_RESIDUAL],
                ),
                outcome(
                    "journal_partial_duplicate_or_stale",
                    &[
                        "partial_effect",
                        "duplicate_effect",
                        "stale_authority_effect",
                        OPEN_WORLD_RESIDUAL,
                    ],
                ),
                outcome(
                    "journal_incomplete_or_holder_unresolved",
                    &["unresolved_command_holder", OPEN_WORLD_RESIDUAL],
                ),
            ],
        },
        ProbeSupport {
            probe_id: "independent_pose_observation".to_owned(),
            outcomes: vec![
                outcome(
                    "no_pose_change",
                    &[
                        "not_dispatched",
                        "dispatched_no_effect",
                        "unresolved_command_holder",
                        OPEN_WORLD_RESIDUAL,
                    ],
                ),
                outcome(
                    "target_pose_once",
                    &[
                        "applied_once_ack_lost",
                        "unresolved_command_holder",
                        OPEN_WORLD_RESIDUAL,
                    ],
                ),
                outcome(
                    "partial_or_multiple_pose_change",
                    &[
                        "partial_effect",
                        "duplicate_effect",
                        "stale_authority_effect",
                        "unresolved_command_holder",
                        OPEN_WORLD_RESIDUAL,
                    ],
                ),
            ],
        },
        ProbeSupport {
            probe_id: "inventory_command_holders".to_owned(),
            outcomes: vec![
                outcome(
                    "holder_inventory_closed",
                    &[
                        "not_dispatched",
                        "dispatched_no_effect",
                        "applied_once_ack_lost",
                        "partial_effect",
                        "duplicate_effect",
                        "stale_authority_effect",
                        OPEN_WORLD_RESIDUAL,
                    ],
                ),
                outcome(
                    "unresolved_holder_found",
                    &["unresolved_command_holder", OPEN_WORLD_RESIDUAL],
                ),
            ],
        },
        ProbeSupport {
            probe_id: "reconcile_effect_and_scope".to_owned(),
            outcomes: vec![
                outcome(
                    "complete_no_effect",
                    &["not_dispatched", "dispatched_no_effect"],
                ),
                outcome("complete_applied_once", &["applied_once_ack_lost"]),
                outcome(
                    "partial_duplicate_or_stale",
                    &[
                        "partial_effect",
                        "duplicate_effect",
                        "stale_authority_effect",
                    ],
                ),
                outcome(
                    "scope_incomplete_or_unmodelled",
                    &["unresolved_command_holder", OPEN_WORLD_RESIDUAL],
                ),
            ],
        },
    ]
}

fn build_episode(index: usize, profile_sha256: &str) -> CandidateEpisode {
    let episode_id = format!("aerb-case-{index:06}");
    let event_ticks = 1_000_000_i64 + index as i64 * 10_000;
    let acquired_ticks = event_ticks + 100;
    let coordinator_digest = digest_label(&format!("{episode_id}:coordinator-record"));
    let journal_digest = digest_label(&format!("{episode_id}:endpoint-journal-observation"));

    CandidateEpisode {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: "AERB".to_owned(),
        episode_id: episode_id.clone(),
        scenario_id: format!("ambiguous-execution-{}", index % 8),
        attempt_id: format!("attempt-{index:06}"),
        trajectory_cluster_id: format!("cluster-{:06}", index / 4),
        evidence_domain: EvidenceDomain::ProcessModel,
        world_vocabulary_version: WORLD_VOCABULARY_VERSION.to_owned(),
        decision_cut_sequence: 4,
        ledger_frontier_sequence: 4,
        current_step: 40 + (index % 5) as u64,
        decision_deadline_step: 60,
        remaining_probe_budget_units: 5.0,
        command_context: CommandContext {
            command_id: format!("command-{index:06}"),
            operation_id: format!("operation-{index:06}"),
            payload_sha256: digest_label(&format!("{episode_id}:payload")),
            action_prefix_sha256: digest_label(&format!("{episode_id}:action-prefix")),
            endpoint_id: "manipulator-endpoint-01".to_owned(),
            endpoint_profile_sha256: profile_sha256.to_owned(),
            authority_epoch: 7 + (index % 3) as u64,
            queue_generation: 2 + (index % 5) as u64,
            execution_scope_id: "robot-cell-01".to_owned(),
            system_boundary_manifest_sha256: digest_label("aerb-generator-system-boundary-v0.1"),
            effect_producing_inventory_profile_sha256: digest_label(
                "aerb-generator-inventory-profile-v0.1",
            ),
        },
        observations: vec![
            Observation {
                observation_id: format!("obs-{index:06}-coordinator"),
                source_id: "coordinator-0".to_owned(),
                source_role: SourceRole::CoordinatorLog,
                quality: ObservationQuality::Admitted,
                ingest_sequence: 2,
                event_time: ClockStamp {
                    clock_domain_id: "coordinator-clock".to_owned(),
                    ticks_ns: event_ticks,
                    uncertainty_ns: 100,
                },
                acquired_time: ClockStamp {
                    clock_domain_id: "coordinator-clock".to_owned(),
                    ticks_ns: acquired_ticks,
                    uncertainty_ns: 100,
                },
                artifact_sha256: coordinator_digest,
                observation_kind: "command_dispatch_record".to_owned(),
            },
            Observation {
                observation_id: format!("obs-{index:06}-endpoint"),
                source_id: "endpoint-journal-0".to_owned(),
                source_role: SourceRole::EndpointJournal,
                quality: if index.is_multiple_of(17) {
                    ObservationQuality::Unknown
                } else if index.is_multiple_of(11) {
                    ObservationQuality::Stale
                } else {
                    ObservationQuality::Admitted
                },
                ingest_sequence: 4,
                event_time: ClockStamp {
                    clock_domain_id: "endpoint-clock".to_owned(),
                    ticks_ns: event_ticks - 500,
                    uncertainty_ns: 500,
                },
                acquired_time: ClockStamp {
                    clock_domain_id: "host-clock".to_owned(),
                    ticks_ns: acquired_ticks + 10_000,
                    uncertainty_ns: 1_000,
                },
                artifact_sha256: journal_digest,
                observation_kind: "endpoint_effect_acknowledgement_missing".to_owned(),
            },
        ],
        permitted_probes: vec![
            PermittedProbe {
                probe_id: PROBE_IDS[0].to_owned(),
                acquisition_cost_units: 1.0,
                result_delay_steps: 2,
            },
            PermittedProbe {
                probe_id: PROBE_IDS[1].to_owned(),
                acquisition_cost_units: 2.0,
                result_delay_steps: 5,
            },
            PermittedProbe {
                probe_id: PROBE_IDS[2].to_owned(),
                acquisition_cost_units: 1.0,
                result_delay_steps: 3,
            },
            PermittedProbe {
                probe_id: PROBE_IDS[3].to_owned(),
                acquisition_cost_units: 2.0,
                result_delay_steps: 5,
            },
        ],
    }
}

fn build_submission(episode: &CandidateEpisode) -> Result<CandidateSubmission, String> {
    let episode_json = serde_json::to_string(episode)
        .map_err(|error| format!("EPISODE_SERIALIZATION_FAILED: {error}"))?;
    let snapshot_sha256 = canonical_sorted_json_sha256(&episode_json)
        .map_err(|error| format!("EPISODE_DIGEST_FAILED: {error}"))?;

    let has_unqualified_observation = episode
        .observations
        .iter()
        .any(|observation| observation.quality != ObservationQuality::Admitted);
    let disposition = if has_unqualified_observation {
        Disposition::Abstain
    } else {
        Disposition::RequestObservation
    };
    let requested_probe_id = if disposition == Disposition::RequestObservation {
        Some("reconcile_effect_and_scope".to_owned())
    } else {
        None
    };

    Ok(CandidateSubmission {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: "AERB".to_owned(),
        episode_id: episode.episode_id.clone(),
        candidate_snapshot_sha256: snapshot_sha256,
        model_sha256: digest_label("aerb-generator-broad-support-baseline-v0.1"),
        observer_profile_sha256: digest_label("aerb-generator-broad-support-profile-v0.1"),
        possible_world_support_ids: WORLD_IDS.iter().map(|world| (*world).to_owned()).collect(),
        disposition,
        requested_probe_id,
        reason_codes: vec![
            "GENERATOR_BASELINE_NOT_A_LEARNED_AGENT".to_owned(),
            "PRESERVE_ALL_POSSIBLE_HISTORIES".to_owned(),
            "REQUEST_SCOPE_BOUND_RECONCILIATION".to_owned(),
        ],
        explanation_observation_ids: episode
            .observations
            .iter()
            .map(|observation| observation.observation_id.clone())
            .collect(),
    })
}

fn build_world_pack(
    episode_id: &str,
    hidden_truth_world_id: &str,
    evaluator_profile_sha256: &str,
) -> EvaluatorWorldPack {
    EvaluatorWorldPack {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: "AERB".to_owned(),
        episode_id: episode_id.to_owned(),
        hidden_truth_world_id: hidden_truth_world_id.to_owned(),
        evaluator_profile_sha256: evaluator_profile_sha256.to_owned(),
        maximum_worst_case_loss: 0.45,
        worlds: WORLD_IDS.iter().map(|world| world_record(world)).collect(),
        decision_losses: Disposition::ALL
            .iter()
            .flat_map(|disposition| {
                WORLD_IDS.iter().map(move |world_id| DecisionLoss {
                    disposition: *disposition,
                    world_id: (*world_id).to_owned(),
                    loss: loss_for(*disposition, world_id),
                })
            })
            .collect(),
        probe_outcome_supports: probe_supports(),
    }
}

#[derive(Serialize)]
struct CandidateCampaignManifest {
    schema_version: String,
    benchmark: String,
    generator_profile_id: String,
    generator_profile_sha256: String,
    scenario_count: usize,
    world_vocabulary_version: String,
    submission_baseline_id: String,
    episode_files: Vec<String>,
    submission_files: Vec<String>,
}

#[derive(Serialize)]
struct EvaluatorCampaignManifest {
    schema_version: String,
    benchmark: String,
    generator_profile_id: String,
    generator_profile_sha256: String,
    seed: u64,
    scenario_count: usize,
    hidden_truth_by_episode: BTreeMap<String, String>,
    claim_ceiling: String,
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("JSON_SERIALIZATION_FAILED: {error}"))?;
    fs::write(path, bytes).map_err(|error| format!("FILE_WRITE_FAILED {}: {error}", path.display()))
}

fn validate_disjoint_directories(candidate_dir: &Path, evaluator_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(candidate_dir)
        .map_err(|error| format!("CANDIDATE_DIR_CREATE_FAILED: {error}"))?;
    fs::create_dir_all(evaluator_dir)
        .map_err(|error| format!("EVALUATOR_DIR_CREATE_FAILED: {error}"))?;
    let candidate = candidate_dir
        .canonicalize()
        .map_err(|error| format!("CANDIDATE_DIR_CANONICALIZE_FAILED: {error}"))?;
    let evaluator = evaluator_dir
        .canonicalize()
        .map_err(|error| format!("EVALUATOR_DIR_CANONICALIZE_FAILED: {error}"))?;
    if candidate == evaluator
        || candidate.starts_with(&evaluator)
        || evaluator.starts_with(&candidate)
    {
        return Err("CANDIDATE_AND_EVALUATOR_OUTPUT_DIRECTORIES_MUST_BE_DISJOINT".to_owned());
    }
    if candidate.join("candidate_manifest.json").exists()
        || evaluator.join("evaluator_manifest.json").exists()
    {
        return Err("OUTPUT_DIRECTORY_ALREADY_CONTAINS_A_CAMPAIGN_MANIFEST".to_owned());
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 4 {
        return Err(
            "usage: aerb_generate <seed-u64> <scenario-count> <candidate-output-dir> <evaluator-output-dir>"
                .to_owned(),
        );
    }
    let seed: u64 = args[0]
        .parse()
        .map_err(|error| format!("SEED_INVALID: {error}"))?;
    let scenario_count: usize = args[1]
        .parse()
        .map_err(|error| format!("SCENARIO_COUNT_INVALID: {error}"))?;
    if !(8..=100_000).contains(&scenario_count) {
        return Err("SCENARIO_COUNT_MUST_BE_IN_8_TO_100000".to_owned());
    }

    let candidate_dir = PathBuf::from(&args[2]);
    let evaluator_dir = PathBuf::from(&args[3]);
    validate_disjoint_directories(&candidate_dir, &evaluator_dir)?;

    let generator_profile_id = "aerb-balanced-world-cycle-generator-v0.1";
    let generator_profile_sha256 = digest_label(generator_profile_id);
    let evaluator_profile_sha256 = digest_label("aerb-synthetic-evaluator-profile-v0.1");

    let mut episode_files = Vec::with_capacity(scenario_count);
    let mut submission_files = Vec::with_capacity(scenario_count);
    let mut hidden_truth_by_episode = BTreeMap::new();

    for index in 0..scenario_count {
        let cycle = index / WORLD_IDS.len();
        let position = index % WORLD_IDS.len();
        let hidden_truth = shuffled_world_cycle(seed, cycle)[position];
        let episode = build_episode(index, &generator_profile_sha256);
        let submission = build_submission(&episode)?;
        let world_pack =
            build_world_pack(&episode.episode_id, hidden_truth, &evaluator_profile_sha256);

        let stem = format!("episode-{index:06}");
        let episode_file = format!("{stem}.json");
        let submission_file = format!("submission-{index:06}.json");
        let world_pack_file = format!("world-pack-{index:06}.json");

        write_json(&candidate_dir.join(&episode_file), &episode)?;
        write_json(&candidate_dir.join(&submission_file), &submission)?;
        write_json(&evaluator_dir.join(&world_pack_file), &world_pack)?;

        episode_files.push(episode_file);
        submission_files.push(submission_file);
        hidden_truth_by_episode.insert(episode.episode_id.clone(), hidden_truth.to_owned());
    }

    let candidate_manifest = CandidateCampaignManifest {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: "AERB".to_owned(),
        generator_profile_id: generator_profile_id.to_owned(),
        generator_profile_sha256: generator_profile_sha256.clone(),
        scenario_count,
        world_vocabulary_version: WORLD_VOCABULARY_VERSION.to_owned(),
        submission_baseline_id: "broad-support-scope-reconciliation-request-v0.1".to_owned(),
        episode_files,
        submission_files,
    };
    let evaluator_manifest = EvaluatorCampaignManifest {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: "AERB".to_owned(),
        generator_profile_id: generator_profile_id.to_owned(),
        generator_profile_sha256,
        seed,
        scenario_count,
        hidden_truth_by_episode,
        claim_ceiling: "Deterministic synthetic coverage corpus only. The schedule is balanced by construction and is not a prior or estimate of real-world world frequencies.".to_owned(),
    };

    write_json(
        &candidate_dir.join("candidate_manifest.json"),
        &candidate_manifest,
    )?;
    write_json(
        &evaluator_dir.join("evaluator_manifest.json"),
        &evaluator_manifest,
    )?;

    println!(
        "Generated {scenario_count} AERB synthetic cases; candidate and evaluator artifacts are in separate directories."
    );
    println!(
        "World schedule: deterministic seed-shuffled cycles over eight states; no real-world prior is implied."
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("AERB_GENERATOR_FAILED: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_world_cycle_covers_all_eight_states_once() {
        for cycle in 0..12 {
            let values = shuffled_world_cycle(42, cycle);
            let unique = values
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(values.len(), WORLD_IDS.len());
            assert_eq!(unique.len(), WORLD_IDS.len());
            for world_id in WORLD_IDS {
                assert!(unique.contains(world_id));
            }
        }
    }

    #[test]
    fn fixed_seed_and_cycle_are_repeatable() {
        assert_eq!(shuffled_world_cycle(42, 3), shuffled_world_cycle(42, 3));
        assert_ne!(shuffled_world_cycle(42, 3), shuffled_world_cycle(43, 3));
    }

    #[test]
    fn generated_candidate_and_evaluator_artifacts_are_disjoint_and_valid() {
        let profile = digest_label("test-generator-profile");
        let episode = build_episode(3, &profile);
        let submission = build_submission(&episode).expect("candidate submission");
        let world_pack = build_world_pack(
            &episode.episode_id,
            "open_world_residual",
            &digest_label("test-evaluator-profile"),
        );
        let episode_json = serde_json::to_string(&episode).expect("episode JSON");
        let submission_json = serde_json::to_string(&submission).expect("submission JSON");
        let world_pack_json = serde_json::to_string(&world_pack).expect("evaluator JSON");

        let episode_value: serde_json::Value =
            serde_json::from_str(&episode_json).expect("episode value");
        let submission_value: serde_json::Value =
            serde_json::from_str(&submission_json).expect("submission value");
        let world_pack_value: serde_json::Value =
            serde_json::from_str(&world_pack_json).expect("world pack value");
        assert!(episode_value.get("hidden_truth_world_id").is_none());
        assert!(submission_value.get("hidden_truth_world_id").is_none());
        assert!(world_pack_value.get("hidden_truth_world_id").is_some());
        assert_eq!(
            episode_value["episode_id"].as_str(),
            Some("aerb-case-000003")
        );
        assert!(
            !episode_value["episode_id"]
                .as_str()
                .unwrap()
                .contains("seed")
        );

        let report = evaluate_json(&episode_json, &submission_json, &world_pack_json)
            .expect("generated candidate should satisfy evaluator contract");
        assert!(report.candidate_contract_valid);
        assert!(report.hidden_truth_in_candidate_support);
        assert!(report.open_world_residual_retained);
        assert_eq!(
            report.probe_admission,
            mobility_rrvr_aerb_evaluator::ProbeAdmission::Eligible
        );
    }

    #[test]
    fn generated_unqualified_observations_force_abstention() {
        let profile = digest_label("test-generator-profile");
        for index in [0usize, 11, 17] {
            let episode = build_episode(index, &profile);
            let submission = build_submission(&episode).expect("submission");
            assert_eq!(submission.disposition, Disposition::Abstain);
            assert!(submission.requested_probe_id.is_none());
        }
    }

    #[test]
    fn generated_clean_observations_request_scoped_reconciliation() {
        let profile = digest_label("test-generator-profile");
        let episode = build_episode(3, &profile);
        let submission = build_submission(&episode).expect("submission");
        assert_eq!(submission.disposition, Disposition::RequestObservation);
        assert_eq!(
            submission.requested_probe_id.as_deref(),
            Some("reconcile_effect_and_scope")
        );
    }

    #[test]
    fn generated_campaign_cycle_is_balanced_not_probability_weighted() {
        let mut counts = BTreeMap::new();
        for cycle in 0..3 {
            for world_id in shuffled_world_cycle(20261126, cycle) {
                *counts.entry(world_id).or_insert(0usize) += 1;
            }
        }
        assert!(
            WORLD_IDS
                .iter()
                .all(|world_id| counts.get(world_id) == Some(&3))
        );
    }

    #[test]
    fn overlapping_candidate_and_evaluator_directories_are_rejected() {
        let root = env::temp_dir().join(format!("aerb-generator-test-{}", std::process::id()));
        let candidate = root.join("candidate");
        let evaluator = candidate.join("evaluator");
        fs::create_dir_all(&candidate).expect("candidate dir");
        assert!(validate_disjoint_directories(&candidate, &evaluator).is_err());
        let _ = fs::remove_dir_all(root);
    }
}
