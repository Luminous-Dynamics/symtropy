//! Transparent policy baselines for the versioned scientific-world episode protocol.
//!
//! These are comparison arms, not learned agents. The seeded-random generator
//! is reproducibility-only and is never suitable for protecting hidden seeds.

use crate::baseline::rank_by_one_step_eig;
use crate::episode::{
    AttemptDisposition, EPISODE_PROTOCOL_VERSION, EpisodeCommand, EpisodeFinish,
    EpisodeFinishDisposition, EpisodeRequest, EpisodeResponse,
};
use crate::{JointKernel, JointMass, Rational};
use std::collections::BTreeMap;
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
    /// Reconstruct the public candidate kernel, update its exact posterior, and
    /// choose the highest one-step joint-state EIG-per-cost action.
    OneStepJointEig,
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
    /// The public candidate model catalogue, priors, or action trace is invalid.
    InvalidPublicModel(String),
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PolicyError {}

/// Return one typed policy response for the supplied public view.
///
/// Model-based arms reconstruct their predictive kernel from the public
/// catalogue; they never receive the episode's hidden owner generator,
/// sampled physical/sensor state or evaluator result.
pub fn next_response(
    request: &EpisodeRequest,
    strategy: BaselineStrategy,
) -> Result<EpisodeResponse, PolicyError> {
    if request.protocol_version != EPISODE_PROTOCOL_VERSION {
        return Err(PolicyError::UnsupportedProtocol);
    }
    let view = &request.view;
    if view.terminal || view.finish.is_some() || !view.trace_complete {
        return Err(PolicyError::EpisodeClosed);
    }

    let candidates = public_candidate_ids(view);
    if candidates.is_empty() {
        return Err(PolicyError::EmptyCandidateCatalogue);
    }

    if strategy == BaselineStrategy::NoIntervention {
        return Ok(finish_response(
            EpisodeFinishDisposition::Abstain,
            candidates,
            "No-intervention baseline: preserve the public prior support and abstain without spending experiment budget.",
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

    let (kernel, belief) = public_kernel_and_belief(view)?;
    let compatible = compatible_candidate_ids(&kernel, &belief);
    let exhausted = view.accepted_attempts >= view.max_attempts
        || view.current_step >= view.max_world_steps
        || view.spent_cost_units >= view.max_cost_units;
    let affordable: Vec<&str> = view
        .actions
        .iter()
        .filter(|action| {
            action.cost_units <= view.max_cost_units.saturating_sub(view.spent_cost_units)
        })
        .map(|action| action.action_id.as_str())
        .collect();

    if strategy == BaselineStrategy::OneStepJointEig {
        if belief.is_empty() {
            return Ok(finish_response(
                EpisodeFinishDisposition::NoCompatibleModel,
                Vec::new(),
                "No declared candidate predictive model assigns positive likelihood to the observed evidence prefix.",
            ));
        }
        if exhausted || affordable.is_empty() {
            return Ok(finish_response(
                status_for_candidate_set(&compatible),
                compatible,
                "One-step joint EIG policy stopped at the frozen attempt, cost, or step budget.",
            ));
        }
        let ranked = rank_by_one_step_eig(&kernel, &belief)
            .map_err(PolicyError::InvalidPublicModel)?;
        let selected = ranked
            .iter()
            .find(|score| {
                view.actions
                    .iter()
                    .any(|action| action.action_id == score.action_id && action.cost_units
                        <= view.max_cost_units.saturating_sub(view.spent_cost_units))
            });
        if let Some(score) = selected {
            if score.expected_information_gain_nats > 1e-12 {
                return Ok(probe_response(&score.action_id));
            }
        }
        return Ok(finish_response(
            status_for_candidate_set(&compatible),
            compatible,
            "One-step joint EIG per cost is zero for every affordable intervention; stop rather than spend budget without expected mechanism information.",
        ));
    }

    let attempted: Vec<&str> = view
        .attempts
        .iter()
        .filter(|attempt| !matches!(
            &attempt.disposition,
            AttemptDisposition::RejectedUnknownAction
        ))
        .map(|attempt| attempt.requested_action_id.as_str())
        .collect();
    let affordable_untried: Vec<&str> = affordable
        .into_iter()
        .filter(|action_id| !attempted.contains(action_id))
        .collect();

    if exhausted || affordable_untried.is_empty() {
        return Ok(finish_response(
            status_for_candidate_set(&compatible),
            compatible,
            "Non-model-based baseline stopped when its untried public action set or frozen budget was exhausted.",
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
        BaselineStrategy::NoIntervention
        | BaselineStrategy::MetadataOnlyFirstCandidate
        | BaselineStrategy::OneStepJointEig => {
            unreachable!("early-returned baseline strategy")
        }
    };

    Ok(probe_response(selected))
}

/// Reconstruct a candidate-only kernel and apply the public evidence prefix.
pub fn public_kernel_and_belief(
    view: &crate::episode::EpisodeView,
) -> Result<(JointKernel, Vec<JointMass>), PolicyError> {
    let kernel = view
        .manifest
        .public_profile
        .model_catalogue
        .to_joint_kernel(
            &view.manifest.public_profile.candidate_hypotheses,
            &view.actions,
        )
        .map_err(PolicyError::InvalidPublicModel)?;
    let mut belief = kernel.initial_belief.clone();

    for attempt in &view.attempts {
        let (action_id, observation_id) = match &attempt.disposition {
            AttemptDisposition::Applied { observation_id } => {
                (attempt.requested_action_id.as_str(), observation_id.as_str())
            }
            AttemptDisposition::HazardTerminal => return Err(PolicyError::EpisodeClosed),
            _ => continue,
        };
        let action = kernel
            .action_ids
            .iter()
            .position(|candidate| candidate == action_id)
            .ok_or_else(|| PolicyError::InvalidPublicModel(
                "applied action is absent from the public model catalogue".to_owned(),
            ))?;
        let observation = kernel
            .observation_ids
            .iter()
            .position(|candidate| candidate == observation_id)
            .ok_or_else(|| PolicyError::InvalidPublicModel(
                "emitted observation is absent from the public model catalogue".to_owned(),
            ))?;

        let mut next: BTreeMap<(usize, usize, usize), Rational> = BTreeMap::new();
        for point in &belief {
            let row = kernel
                .transition_row(
                    point.hypothesis,
                    point.physical_state,
                    point.sensor_state,
                    action,
                )
                .ok_or_else(|| PolicyError::InvalidPublicModel(
                    "public predictive model lacks an action-conditioned state row".to_owned(),
                ))?;
            for outcome in &row.outcomes {
                if outcome.observation == Some(observation) {
                    let key = (
                        point.hypothesis,
                        outcome.next_physical_state,
                        outcome.next_sensor_state,
                    );
                    let weighted = point
                        .mass
                        .checked_mul(outcome.probability)
                        .map_err(PolicyError::InvalidPublicModel)?;
                    let current = next.get(&key).copied().unwrap_or(Rational::zero());
                    next.insert(
                        key,
                        current
                            .checked_add(weighted)
                            .map_err(PolicyError::InvalidPublicModel)?,
                    );
                }
            }
        }
        let total = next
            .values()
            .try_fold(Rational::zero(), |sum, mass| sum.checked_add(*mass))
            .map_err(PolicyError::InvalidPublicModel)?;
        if total.is_zero() {
            belief.clear();
            break;
        }
        let mut normalized = Vec::with_capacity(next.len());
        for ((hypothesis, physical_state, sensor_state), mass) in next {
            normalized.push(JointMass {
                hypothesis,
                physical_state,
                sensor_state,
                mass: mass
                    .checked_div(total)
                    .map_err(PolicyError::InvalidPublicModel)?,
            });
        }
        belief = normalized;
    }
    Ok((kernel, belief))
}

fn public_candidate_ids(view: &crate::episode::EpisodeView) -> Vec<String> {
    view.manifest
        .public_profile
        .candidate_hypotheses
        .iter()
        .map(|candidate| candidate.hypothesis_id.clone())
        .collect()
}

fn compatible_candidate_ids(kernel: &JointKernel, belief: &[JointMass]) -> Vec<String> {
    let mut present = vec![false; kernel.hypothesis_ids.len()];
    for point in belief {
        present[point.hypothesis] = true;
    }
    kernel
        .hypothesis_ids
        .iter()
        .zip(present)
        .filter_map(|(id, retained)| retained.then_some(id.clone()))
        .collect()
}

fn status_for_candidate_set(candidates: &[String]) -> EpisodeFinishDisposition {
    match candidates.len() {
        0 => EpisodeFinishDisposition::NoCompatibleModel,
        1 => EpisodeFinishDisposition::Identified,
        _ => EpisodeFinishDisposition::Ambiguous,
    }
}

fn probe_response(action_id: &str) -> EpisodeResponse {
    EpisodeResponse {
        protocol_version: EPISODE_PROTOCOL_VERSION.to_owned(),
        command: EpisodeCommand::Probe {
            action_id: action_id.to_owned(),
        },
    }
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
    use crate::model_catalogue::{PublicModelCatalogue, PublicProbability};
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
        let candidate_ids = kernel.hypothesis_ids.clone();
        let model_catalogue = PublicModelCatalogue::from_kernel(&kernel, &candidate_ids).unwrap();
        let mut manifest = EpisodeManifest {
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
                        prior_probability: PublicProbability::new(1, 2).unwrap(),
                    },
                    PublicHypothesis {
                        hypothesis_id: "candidate-b".to_owned(),
                        structural_family: "fixture".to_owned(),
                        description: "Candidate B produces the one observation.".to_owned(),
                        prior_probability: PublicProbability::new(1, 2).unwrap(),
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
                model_catalogue,
            },
        };
        manifest.model_profile_sha256 = manifest.public_profile.sha256().unwrap();
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
                    && finish.compatible_hypothesis_ids == vec!["candidate-a".to_owned(), "candidate-b".to_owned()]
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
    fn one_step_joint_eig_baseline_uses_the_public_catalogue_and_selects_information() {
        let response = next_response(&request(), BaselineStrategy::OneStepJointEig).unwrap();
        assert_eq!(
            response.command,
            EpisodeCommand::Probe {
                action_id: "measure-a".to_owned()
            }
        );
    }

    #[test]
    fn public_posterior_updates_from_only_the_emitted_observation_prefix() {
        let mut request = request();
        request.view.attempts.push(crate::episode::EpisodeAttempt {
            attempt_id: "attempt-00000000".to_owned(),
            ordinal: 0,
            requested_action_id: "measure-a".to_owned(),
            requested_cost_units: Some(1),
            charged_cost_units: 1,
            requested_at_step: 0,
            executed_at_step: Some(1),
            disposition: AttemptDisposition::Applied {
                observation_id: "zero".to_owned(),
            },
        });
        let (kernel, belief) = public_kernel_and_belief(&request.view).unwrap();
        assert_eq!(belief.len(), 1);
        assert_eq!(kernel.hypothesis_ids[belief[0].hypothesis], "candidate-a");
        assert_eq!(belief[0].mass, Rational::one());
        let response = next_response(&request, BaselineStrategy::OneStepJointEig).unwrap();
        assert!(matches!(
            response.command,
            EpisodeCommand::Finish { ref finish }
                if finish.disposition == EpisodeFinishDisposition::Identified
                    && finish.compatible_hypothesis_ids == vec!["candidate-a".to_owned()]
        ));
    }

    #[test]
    fn unknown_observation_symbols_fail_closed_before_posterior_update() {
        let mut request = request();
        request.view.attempts.push(crate::episode::EpisodeAttempt {
            attempt_id: "attempt-00000000".to_owned(),
            ordinal: 0,
            requested_action_id: "measure-a".to_owned(),
            requested_cost_units: Some(1),
            charged_cost_units: 1,
            requested_at_step: 0,
            executed_at_step: Some(1),
            disposition: AttemptDisposition::Applied {
                observation_id: "one".to_owned(),
            },
        });
        request.view.attempts[0].disposition = AttemptDisposition::Applied {
            observation_id: "not-declared".to_owned(),
        };
        assert!(matches!(
            public_kernel_and_belief(&request.view),
            Err(PolicyError::InvalidPublicModel(_))
        ));
    }

    #[test]
    fn one_step_joint_eig_updates_to_the_candidate_supported_by_the_observation() {
        let mut request = request();
        request.view.attempts.push(crate::episode::EpisodeAttempt {
            attempt_id: "attempt-00000000".to_owned(),
            ordinal: 0,
            requested_action_id: "measure-a".to_owned(),
            requested_cost_units: Some(1),
            charged_cost_units: 1,
            requested_at_step: 0,
            executed_at_step: Some(1),
            disposition: AttemptDisposition::Applied {
                observation_id: "one".to_owned(),
            },
        });
        let (kernel, belief) = public_kernel_and_belief(&request.view).unwrap();
        assert_eq!(belief.len(), 1);
        assert_eq!(kernel.hypothesis_ids[belief[0].hypothesis], "candidate-b");
    }

    #[test]
    fn one_step_joint_eig_detects_empty_public_version_space_on_hidden_out_of_family_control() {
        let mut request = request();
        let full_kernel = request
            .view
            .manifest
            .public_profile
            .model_catalogue
            .to_joint_kernel(
                &request.view.manifest.public_profile.candidate_hypotheses,
                &request.view.actions,
            )
            .unwrap();
        let candidate_b = vec!["candidate-b".to_owned()];
        let mut public_candidate = request
            .view
            .manifest
            .public_profile
            .candidate_hypotheses
            .iter()
            .find(|candidate| candidate.hypothesis_id == "candidate-b")
            .unwrap()
            .clone();
        public_candidate.prior_probability = PublicProbability::new(1, 1).unwrap();
        request
            .view
            .manifest
            .public_profile
            .candidate_hypotheses = vec![public_candidate];
        request
            .view
            .manifest
            .public_profile
            .model_catalogue = PublicModelCatalogue::from_kernel(&full_kernel, &candidate_b).unwrap();
        request.view.attempts.push(crate::episode::EpisodeAttempt {
            attempt_id: "attempt-00000000".to_owned(),
            ordinal: 0,
            requested_action_id: "measure-a".to_owned(),
            requested_cost_units: Some(1),
            charged_cost_units: 1,
            requested_at_step: 0,
            executed_at_step: Some(1),
            disposition: AttemptDisposition::Applied {
                observation_id: "zero".to_owned(),
            },
        });

        let (_, belief) = public_kernel_and_belief(&request.view).unwrap();
        assert!(belief.is_empty());
        let response = next_response(&request, BaselineStrategy::OneStepJointEig).unwrap();
        assert!(matches!(
            response.command,
            EpisodeCommand::Finish { ref finish }
                if finish.disposition == EpisodeFinishDisposition::NoCompatibleModel
                    && finish.compatible_hypothesis_ids.is_empty()
        ));
    }

    #[test]
    fn metadata_only_negative_control_reports_first_candidate_without_observations() {
        let response =
            next_response(&request(), BaselineStrategy::MetadataOnlyFirstCandidate).unwrap();
        assert!(matches!(
            response.command,
            EpisodeCommand::Finish { ref finish }
                if finish.disposition == EpisodeFinishDisposition::Identified
                    && finish.compatible_hypothesis_ids == vec!["candidate-a".to_owned()]
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
