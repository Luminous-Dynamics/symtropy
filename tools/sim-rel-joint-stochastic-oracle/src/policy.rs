//! Transparent policy baselines for the versioned scientific-world episode protocol.
//!
//! These are comparison arms, not learned agents. The seeded-random generator
//! is reproducibility-only and is never suitable for protecting hidden seeds.

use crate::episode::{
    EPISODE_PROTOCOL_VERSION, EpisodeCommand, EpisodeFinish, EpisodeFinishDisposition,
    EpisodeRequest, EpisodeResponse,
};
use std::fmt;

/// Policy arms that operate only on the public episode request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BaselineStrategy {
    /// Stop immediately and report the full declared candidate set.
    NoIntervention,
    /// Try each affordable public action once in catalogue order.
    FixedActionOrder,
    /// Select among unattempted affordable public actions with a reproducible seed.
    SeededRandom { seed: u64 },
    /// Negative control: identify the first public candidate from metadata alone.
    MetadataOnlyFirstCandidate,
}

/// Fail-closed baseline policy error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyError {
    /// The request uses a different public wire protocol version.
    UnsupportedProtocol,
    /// The episode is already terminal and cannot receive another policy action.
    EpisodeClosed,
    /// The public candidate catalogue is empty.
    EmptyCandidateCatalogue,
    /// The metadata-only control requires a first candidate.
    MissingFirstCandidate,
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PolicyError {}

/// Return one typed policy response for the supplied public view.
///
/// The policy sees no hidden generating mechanism, physical/sensor state,
/// transition kernel, sampled world seed, or evaluator output.
pub fn next_response(
    request: &EpisodeRequest,
    strategy: BaselineStrategy,
) -> Result<EpisodeResponse, PolicyError> {
    if request.protocol_version != EPISODE_PROTOCOL_VERSION {
        return Err(PolicyError::UnsupportedProtocol);
    }
    let view = &request.view;
    if view.terminal || view.finish.is_some() {
        return Err(PolicyError::EpisodeClosed);
    }

    let candidates: Vec<String> = view
        .manifest
        .public_profile
        .candidate_hypotheses
        .iter()
        .map(|candidate| candidate.hypothesis_id.clone())
        .collect();
    if candidates.is_empty() {
        return Err(PolicyError::EmptyCandidateCatalogue);
    }

    if strategy == BaselineStrategy::NoIntervention {
        return Ok(finish_response(
            EpisodeFinishDisposition::Abstain,
            candidates,
            "No-intervention baseline: preserve the declared candidate set and abstain without spending experiment budget.",
        ));
    }

    if strategy == BaselineStrategy::MetadataOnlyFirstCandidate {
        let first = candidates
            .first()
            .cloned()
            .ok_or(PolicyError::MissingFirstCandidate)?;
        return Ok(finish_response(
            EpisodeFinishDisposition::Identified,
            vec![first],
            "Metadata-only negative control: selects the first public candidate without using observations.",
        ));
    }

    let exhausted = view.accepted_attempts >= view.max_attempts
        || view.current_step >= view.max_world_steps
        || view.spent_cost_units >= view.max_cost_units;
    let attempted: Vec<&str> = view
        .attempts
        .iter()
        .map(|attempt| attempt.requested_action_id.as_str())
        .collect();
    let affordable_untried: Vec<&str> = view
        .actions
        .iter()
        .filter(|action| {
            !attempted.contains(&action.action_id.as_str())
                && action.cost_units <= view.max_cost_units.saturating_sub(view.spent_cost_units)
        })
        .map(|action| action.action_id.as_str())
        .collect();

    if exhausted || affordable_untried.is_empty() {
        let disposition = if candidates.len() > 1 {
            EpisodeFinishDisposition::Ambiguous
        } else {
            EpisodeFinishDisposition::Abstain
        };
        return Ok(finish_response(
            disposition,
            candidates,
            "Budget/action-order baseline: stop when the available public action set or frozen budget is exhausted.",
        ));
    }

    let selected = match strategy {
        BaselineStrategy::FixedActionOrder => affordable_untried[0],
        BaselineStrategy::SeededRandom { seed } => {
            let episode_hash = stable_hash(view.episode_id.as_bytes());
            let draw = mix64(
                seed ^ episode_hash ^ (view.attempts.len() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
            );
            affordable_untried[(draw as usize) % affordable_untried.len()]
        }
        BaselineStrategy::NoIntervention | BaselineStrategy::MetadataOnlyFirstCandidate => {
            unreachable!("early-returned baseline strategy")
        }
    };

    Ok(EpisodeResponse {
        protocol_version: EPISODE_PROTOCOL_VERSION.to_owned(),
        command: EpisodeCommand::Probe {
            action_id: selected.to_owned(),
        },
    })
}

fn finish_response(
    disposition: EpisodeFinishDisposition,
    compatible_hypothesis_ids: Vec<String>,
    rationale: &str,
) -> EpisodeResponse {
    EpisodeResponse {
        protocol_version: EPISODE_PROTOCOL_VERSION.to_owned(),
        command: EpisodeCommand::Finish {
            finish: EpisodeFinish {
                disposition,
                compatible_hypothesis_ids,
                rationale: rationale.to_owned(),
            },
        },
    }
}

fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xCBF2_9CE4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01B3)
    })
}

fn mix64(value: u64) -> u64 {
    let mut value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episode::{
        EpisodeConfig, EpisodeManifest, EpisodeProvider, PublicActionDescription,
        PublicHypothesis, PublicObservationDescription,
    };
    use crate::JointKernel;

    fn request() -> EpisodeRequest {
        let kernel = JointKernel {
            hypothesis_ids: vec!["candidate-a".to_owned(), "candidate-b".to_owned()],
            physical_state_count: 1,
            sensor_state_count: 1,
            action_ids: vec!["measure-a".to_owned(), "measure-b".to_owned()],
            observation_ids: vec!["zero".to_owned(), "one".to_owned()],
            action_costs: vec![1, 1],
            initial_belief: vec![
                crate::JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: crate::Rational::new(1, 2).unwrap(),
                },
                crate::JointMass {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: crate::Rational::new(1, 2).unwrap(),
                },
            ],
            rows: vec![
                crate::KernelRow {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    action: 0,
                    outcomes: vec![crate::JointOutcome {
                        probability: crate::Rational::one(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(0),
                    }],
                },
                crate::KernelRow {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    action: 1,
                    outcomes: vec![crate::JointOutcome {
                        probability: crate::Rational::one(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(0),
                    }],
                },
                crate::KernelRow {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    action: 0,
                    outcomes: vec![crate::JointOutcome {
                        probability: crate::Rational::one(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(1),
                    }],
                },
                crate::KernelRow {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    action: 1,
                    outcomes: vec![crate::JointOutcome {
                        probability: crate::Rational::one(),
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
            public_profile: crate::episode::PublicResearchProfile {
                task_description: "Identify which declared candidate matches the observations.".to_owned(),
                candidate_hypotheses: vec![
                    PublicHypothesis {
                        hypothesis_id: "candidate-a".to_owned(),
                        structural_family: "fixture".to_owned(),
                        description: "Candidate A produces the zero observation.".to_owned(),
                    },
                    PublicHypothesis {
                        hypothesis_id: "candidate-b".to_owned(),
                        structural_family: "fixture".to_owned(),
                        description: "Candidate B produces the one observation.".to_owned(),
                    },
                ],
                actions: vec![
                    PublicActionDescription {
                        action_id: "measure-a".to_owned(),
                        description: "Measure the declared signal.".to_owned(),
                        target: "signal".to_owned(),
                    },
                    PublicActionDescription {
                        action_id: "measure-b".to_owned(),
                        description: "Repeat the declared signal measurement.".to_owned(),
                        target: "signal".to_owned(),
                    },
                ],
                observations: vec![
                    PublicObservationDescription {
                        observation_id: "zero".to_owned(),
                        channel_id: "signal".to_owned(),
                        description: "The instrument returned zero.".to_owned(),
                        unit: "categorical".to_owned(),
                    },
                    PublicObservationDescription {
                        observation_id: "one".to_owned(),
                        channel_id: "signal".to_owned(),
                        description: "The instrument returned one.".to_owned(),
                        unit: "categorical".to_owned(),
                    },
                ],
            },
        };
        let provider = EpisodeProvider::new(
            "baseline-policy-fixture-0001",
            manifest,
            kernel,
            EpisodeConfig {
                max_attempts: 2,
                max_cost_units: 2,
                max_world_steps: 2,
            },
            7,
        )
        .unwrap();
        provider.request()
    }

    #[test]
    fn no_intervention_baseline_abstains_without_spending_budget() {
        let response = next_response(&request(), BaselineStrategy::NoIntervention).unwrap();
        assert!(matches!(
            response.command,
            EpisodeCommand::Finish { ref finish }
                if finish.disposition == EpisodeFinishDisposition::Abstain
                    && finish.compatible_hypothesis_ids == vec!["candidate-a", "candidate-b"]
        ));
    }

    #[test]
    fn fixed_order_baseline_uses_public_action_catalogue_order() {
        let response = next_response(&request(), BaselineStrategy::FixedActionOrder).unwrap();
        assert_eq!(
            response.command,
            EpisodeCommand::Probe {
                action_id: "measure-a".to_owned()
            }
        );
    }

    #[test]
    fn fixed_order_skips_previously_attempted_action() {
        let mut request = request();
        request.view.attempts.push(crate::episode::EpisodeAttempt {
            attempt_id: "attempt-00000000".to_owned(),
            ordinal: 0,
            requested_action_id: "measure-a".to_owned(),
            requested_cost_units: Some(1),
            charged_cost_units: 0,
            requested_at_step: 0,
            executed_at_step: None,
            disposition: crate::episode::AttemptDisposition::RejectedUnknownAction,
        });
        let response = next_response(&request, BaselineStrategy::FixedActionOrder).unwrap();
        assert_eq!(
            response.command,
            EpisodeCommand::Probe {
                action_id: "measure-b".to_owned()
            }
        );
    }

    #[test]
    fn seeded_random_baseline_is_replay_deterministic_for_same_public_view() {
        let request = request();
        let strategy = BaselineStrategy::SeededRandom { seed: 42 };
        assert_eq!(
            next_response(&request, strategy).unwrap(),
            next_response(&request, strategy).unwrap()
        );
    }

    #[test]
    fn metadata_only_negative_control_reports_first_candidate_without_observations() {
        let response =
            next_response(&request(), BaselineStrategy::MetadataOnlyFirstCandidate).unwrap();
        assert!(matches!(
            response.command,
            EpisodeCommand::Finish { ref finish }
                if finish.disposition == EpisodeFinishDisposition::Identified
                    && finish.compatible_hypothesis_ids == vec!["candidate-a"]
        ));
    }

    #[test]
    fn closed_episode_rejects_another_policy_step() {
        let mut request = request();
        request.view.terminal = true;
        assert_eq!(
            next_response(&request, BaselineStrategy::FixedActionOrder),
            Err(PolicyError::EpisodeClosed)
        );
    }
}
