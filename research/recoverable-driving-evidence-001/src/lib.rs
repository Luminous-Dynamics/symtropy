//! Bounded, dependency-free conformance checks for recoverable-driving campaign records.
//!
//! This is a finite-record protocol auditor. It does not authenticate upstream
//! evidence, run a simulator, establish physical causality, train a policy, or
//! certify safety.

use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EPISODES: usize = 100_000;
pub const MAX_BRANCHES: usize = 100_000;
pub const MAX_ACTORS_PER_BRANCH: usize = 1_024;
pub const MAX_EVENTS_PER_BRANCH: usize = 4_096;
pub const MAX_FIELDS_PER_BRANCH: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EpisodeDisposition {
    Completed,
    Failed,
    Incomplete,
    Censored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeRecord {
    pub episode_id: String,
    pub disposition: EpisodeDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InformationSourceRole {
    PolicyVisible,
    TeacherOnly,
    EvaluatorOnly,
    Future,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationField {
    pub field_id: String,
    pub source_role: InformationSourceRole,
    pub available_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRecord {
    pub query_id: String,
    pub issued_tick: u64,
    pub answer_available_tick: u64,
    pub outcome_available_tick: u64,
    pub inputs: Vec<InformationField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActorResponseOrigin {
    Recomputed,
    Replayed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorResponse {
    pub actor_id: String,
    pub policy_revision_digest: String,
    pub policy_input_digest: String,
    pub action_digest: String,
    pub randomness_digest: Option<String>,
    pub origin: ActorResponseOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RandomEventClass {
    Exogenous,
    Endogenous,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomEvent {
    pub event_id: String,
    pub shared_with_parent: bool,
    pub event_class: RandomEventClass,
    pub invariant_under_intervention: bool,
    pub key_digest: String,
    pub sampled_value_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchRecord {
    pub branch_id: String,
    pub episode_id: String,
    pub parent_branch_id: Option<String>,
    pub checkpoint_digest: String,
    pub recurrent_state_policy_id: String,
    pub observation_history_digest: String,
    pub simulator_revision: String,
    pub decision_tick: u64,
    pub outcome_tick: u64,
    pub intervention_digest: String,
    pub expected_actor_ids: Vec<String>,
    pub actor_responses: Vec<ActorResponse>,
    pub expected_random_event_ids: Vec<String>,
    pub random_events: Vec<RandomEvent>,
    pub policy_input_fields: Vec<InformationField>,
    pub query: Option<QueryRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrozenEvaluationControls {
    pub frozen: bool,
    pub teacher_blending_enabled: bool,
    pub online_updates_enabled: bool,
    pub exploration_noise_enabled: bool,
    pub teacher_inputs_visible: bool,
    pub future_outcomes_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignRecord {
    pub expected_episode_ids: Vec<String>,
    pub expected_branch_ids: Vec<String>,
    pub episodes: Vec<EpisodeRecord>,
    pub branches: Vec<BranchRecord>,
    pub evaluation_controls: FrozenEvaluationControls,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Finding {
    EmptyIdentity(&'static str),
    DuplicateExpectedEpisode,
    DuplicateExpectedBranch,
    DuplicateEpisode,
    DuplicateBranch,
    MissingEpisode,
    UnexpectedEpisode,
    MissingBranch,
    UnexpectedBranch,
    BranchReferencesMissingEpisode,
    MissingParent,
    CrossEpisodeParent,
    BranchBoundaryMismatch,
    LineageCycle,
    DuplicateActor,
    MissingExpectedActor,
    UnexpectedActor,
    MissingParentActorForReplay,
    ReplayedActorInputChanged,
    ReplayedActorPolicyChanged,
    ReplayedActorActionChanged,
    ReplayedActorRandomnessChanged,
    DuplicateRandomEvent,
    MissingExpectedRandomEvent,
    UnexpectedRandomEvent,
    SharedRandomKeyNotDeclared,
    DeclaredSharedKeyMismatch,
    SharedRandomKeyNotInvariantExogenous,
    BranchOutcomeBeforeDecision,
    SharedRandomValueChanged,
    RandomKeyEventIdentityChanged,
    DuplicatePolicyField,
    PolicyFieldNotVisible,
    PolicyFieldFromFuture,
    DuplicateQueryId,
    QueryIssuedAfterDecision,
    QueryAnswerNotAfterIssue,
    QueryOutcomeNotAfterIssue,
    DuplicateQueryInput,
    QueryInputNotPolicyVisible,
    QueryInputFromFuture,
    FrozenTeacherBlending,
    FrozenOnlineUpdate,
    FrozenExploration,
    FrozenTeacherInput,
    FrozenFutureOutcome,
    BoundExceeded(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportDisposition {
    Valid,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignSummary {
    pub expected_episodes: usize,
    pub observed_episodes: usize,
    pub completed_episodes: usize,
    pub failed_episodes: usize,
    pub incomplete_episodes: usize,
    pub censored_episodes: usize,
    pub expected_branches: usize,
    pub observed_branches: usize,
    pub actor_response_records: usize,
    pub random_event_records: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub disposition: ReportDisposition,
    pub summary: CampaignSummary,
    pub findings: Vec<Finding>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn duplicate<T: Ord + Clone>(values: &[T]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().any(|value| !seen.insert(value.clone()))
}

fn id_set(values: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    values.into_iter().collect()
}

fn validate_parent_graph(
    branches: &BTreeMap<String, &BranchRecord>,
    findings: &mut Vec<Finding>,
) {
    let mut marks: BTreeMap<String, u8> = BTreeMap::new();
    for start in branches.keys() {
        if marks.get(start).copied().unwrap_or(0) == 2 {
            continue;
        }
        let mut path: Vec<String> = Vec::new();
        let mut cursor = start.clone();
        loop {
            match marks.get(&cursor).copied().unwrap_or(0) {
                2 => break,
                1 => {
                    findings.push(Finding::LineageCycle);
                    break;
                }
                _ => {}
            }
            marks.insert(cursor.clone(), 1);
            path.push(cursor.clone());
            let Some(branch) = branches.get(&cursor) else {
                break;
            };
            let Some(parent) = &branch.parent_branch_id else {
                break;
            };
            if !branches.contains_key(parent) {
                break;
            }
            cursor = parent.clone();
        }
        for id in path {
            marks.insert(id, 2);
        }
    }
}

fn validate_branch_pair(branch: &BranchRecord, parent: &BranchRecord, findings: &mut Vec<Finding>) {
    if branch.episode_id != parent.episode_id {
        findings.push(Finding::CrossEpisodeParent);
        return;
    }
    if branch.checkpoint_digest != parent.checkpoint_digest
        || branch.recurrent_state_policy_id != parent.recurrent_state_policy_id
        || branch.observation_history_digest != parent.observation_history_digest
        || branch.simulator_revision != parent.simulator_revision
        || branch.decision_tick != parent.decision_tick
    {
        findings.push(Finding::BranchBoundaryMismatch);
    }

    let parent_actors: BTreeMap<&str, &ActorResponse> = parent
        .actor_responses
        .iter()
        .map(|actor| (actor.actor_id.as_str(), actor))
        .collect();
    for actor in &branch.actor_responses {
        if actor.origin == ActorResponseOrigin::Replayed {
            let Some(previous) = parent_actors.get(actor.actor_id.as_str()) else {
                findings.push(Finding::MissingParentActorForReplay);
                continue;
            };
            if actor.policy_input_digest != previous.policy_input_digest {
                findings.push(Finding::ReplayedActorInputChanged);
            }
            if actor.policy_revision_digest != previous.policy_revision_digest {
                findings.push(Finding::ReplayedActorPolicyChanged);
            }
            if actor.action_digest != previous.action_digest {
                findings.push(Finding::ReplayedActorActionChanged);
            }
            if actor.randomness_digest != previous.randomness_digest {
                findings.push(Finding::ReplayedActorRandomnessChanged);
            }
        }
    }

    let parent_events: BTreeMap<&str, &RandomEvent> = parent
        .random_events
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();
    let parent_keys: BTreeSet<&str> = parent
        .random_events
        .iter()
        .map(|event| event.key_digest.as_str())
        .collect();
    for event in &branch.random_events {
        if let Some(previous) = parent_events.get(event.event_id.as_str()) {
            let same_key = event.key_digest == previous.key_digest;
            if event.shared_with_parent != same_key {
                findings.push(if event.shared_with_parent {
                    Finding::DeclaredSharedKeyMismatch
                } else {
                    Finding::SharedRandomKeyNotDeclared
                });
            }
            if same_key {
                if event.event_class != RandomEventClass::Exogenous
                    || previous.event_class != RandomEventClass::Exogenous
                    || !event.invariant_under_intervention
                    || !previous.invariant_under_intervention
                {
                    findings.push(Finding::SharedRandomKeyNotInvariantExogenous);
                }
                if event.sampled_value_digest != previous.sampled_value_digest {
                    findings.push(Finding::SharedRandomValueChanged);
                }
            }
        } else {
            if event.shared_with_parent {
                findings.push(Finding::DeclaredSharedKeyMismatch);
            }
            if parent_keys.contains(event.key_digest.as_str()) {
                findings.push(Finding::RandomKeyEventIdentityChanged);
            }
        }
    }
}

fn validate_information_fields(
    fields: &[InformationField],
    available_by_tick: u64,
    query_inputs: bool,
    findings: &mut Vec<Finding>,
) {
    if duplicate(&fields.iter().map(|field| field.field_id.clone()).collect::<Vec<_>>()) {
        if query_inputs {
            findings.push(Finding::DuplicateQueryInput);
        } else {
            findings.push(Finding::DuplicatePolicyField);
        }
    }
    for field in fields {
        if field.source_role != InformationSourceRole::PolicyVisible {
            findings.push(if query_inputs {
                Finding::QueryInputNotPolicyVisible
            } else {
                Finding::PolicyFieldNotVisible
            });
        }
        if field.available_tick > available_by_tick {
            findings.push(if query_inputs {
                Finding::QueryInputFromFuture
            } else {
                Finding::PolicyFieldFromFuture
            });
        }
    }
}

/// Validate the expected/observed campaign denominators and finite branch evidence.
///
/// Findings are sorted and deduplicated so the output is stable under input
/// permutation. The function does not authenticate digests or prove declared
/// exogeneity/invariance; those remain upstream evidence obligations.
pub fn validate_campaign(campaign: &CampaignRecord) -> ValidationReport {
    let mut findings = Vec::new();

    let summary = CampaignSummary {
        expected_episodes: campaign.expected_episode_ids.len(),
        observed_episodes: campaign.episodes.len(),
        completed_episodes: campaign
            .episodes
            .iter()
            .filter(|episode| episode.disposition == EpisodeDisposition::Completed)
            .count(),
        failed_episodes: campaign
            .episodes
            .iter()
            .filter(|episode| episode.disposition == EpisodeDisposition::Failed)
            .count(),
        incomplete_episodes: campaign
            .episodes
            .iter()
            .filter(|episode| episode.disposition == EpisodeDisposition::Incomplete)
            .count(),
        censored_episodes: campaign
            .episodes
            .iter()
            .filter(|episode| episode.disposition == EpisodeDisposition::Censored)
            .count(),
        expected_branches: campaign.expected_branch_ids.len(),
        observed_branches: campaign.branches.len(),
        actor_response_records: campaign
            .branches
            .iter()
            .map(|branch| branch.actor_responses.len())
            .sum(),
        random_event_records: campaign
            .branches
            .iter()
            .map(|branch| branch.random_events.len())
            .sum(),
    };

    for (name, value) in [
        ("episodes", campaign.expected_episode_ids.len().max(campaign.episodes.len())),
        ("branches", campaign.expected_branch_ids.len().max(campaign.branches.len())),
    ] {
        let limit = if name == "episodes" { MAX_EPISODES } else { MAX_BRANCHES };
        if value > limit {
            findings.push(Finding::BoundExceeded(name));
        }
    }

    if duplicate(&campaign.expected_episode_ids) {
        findings.push(Finding::DuplicateExpectedEpisode);
    }
    if duplicate(&campaign.expected_branch_ids) {
        findings.push(Finding::DuplicateExpectedBranch);
    }
    if duplicate(&campaign.episodes.iter().map(|episode| episode.episode_id.clone()).collect::<Vec<_>>()) {
        findings.push(Finding::DuplicateEpisode);
    }
    if duplicate(&campaign.branches.iter().map(|branch| branch.branch_id.clone()).collect::<Vec<_>>()) {
        findings.push(Finding::DuplicateBranch);
    }

    let expected_episodes = id_set(campaign.expected_episode_ids.clone());
    let observed_episodes = id_set(campaign.episodes.iter().map(|item| item.episode_id.clone()));
    for _ in expected_episodes.difference(&observed_episodes) {
        findings.push(Finding::MissingEpisode);
    }
    for _ in observed_episodes.difference(&expected_episodes) {
        findings.push(Finding::UnexpectedEpisode);
    }

    let expected_branches = id_set(campaign.expected_branch_ids.clone());
    let observed_branches = id_set(campaign.branches.iter().map(|item| item.branch_id.clone()));
    for _ in expected_branches.difference(&observed_branches) {
        findings.push(Finding::MissingBranch);
    }
    for _ in observed_branches.difference(&expected_branches) {
        findings.push(Finding::UnexpectedBranch);
    }

    let episodes_by_id: BTreeSet<&str> =
        campaign.episodes.iter().map(|episode| episode.episode_id.as_str()).collect();
    let branches_by_id: BTreeMap<String, &BranchRecord> = campaign
        .branches
        .iter()
        .map(|branch| (branch.branch_id.clone(), branch))
        .collect();

    for branch in &campaign.branches {
        for (name, value) in [
            ("branch_id", branch.branch_id.as_str()),
            ("episode_id", branch.episode_id.as_str()),
            ("checkpoint_digest", branch.checkpoint_digest.as_str()),
            ("recurrent_state_policy_id", branch.recurrent_state_policy_id.as_str()),
            ("observation_history_digest", branch.observation_history_digest.as_str()),
            ("simulator_revision", branch.simulator_revision.as_str()),
            ("intervention_digest", branch.intervention_digest.as_str()),
        ] {
            if !nonempty(value) {
                findings.push(Finding::EmptyIdentity(name));
            }
        }

        if !episodes_by_id.contains(branch.episode_id.as_str()) {
            findings.push(Finding::BranchReferencesMissingEpisode);
        }
        if branch.outcome_tick < branch.decision_tick {
            findings.push(Finding::BranchOutcomeBeforeDecision);
        }

        if branch.actor_responses.len() > MAX_ACTORS_PER_BRANCH
            || branch.expected_actor_ids.len() > MAX_ACTORS_PER_BRANCH
        {
            findings.push(Finding::BoundExceeded("actors_per_branch"));
        }
        if branch.random_events.len() > MAX_EVENTS_PER_BRANCH
            || branch.expected_random_event_ids.len() > MAX_EVENTS_PER_BRANCH
        {
            findings.push(Finding::BoundExceeded("random_events_per_branch"));
        }
        if branch.policy_input_fields.len() > MAX_FIELDS_PER_BRANCH {
            findings.push(Finding::BoundExceeded("policy_fields_per_branch"));
        }
        if branch
            .query
            .as_ref()
            .is_some_and(|query| query.inputs.len() > MAX_FIELDS_PER_BRANCH)
        {
            findings.push(Finding::BoundExceeded("query_fields_per_branch"));
        }

        if duplicate(&branch.expected_actor_ids) || duplicate(&branch.actor_responses.iter().map(|a| a.actor_id.clone()).collect::<Vec<_>>()) {
            findings.push(Finding::DuplicateActor);
        }
        let expected_actors = id_set(branch.expected_actor_ids.clone());
        let observed_actors = id_set(branch.actor_responses.iter().map(|a| a.actor_id.clone()));
        for _ in expected_actors.difference(&observed_actors) {
            findings.push(Finding::MissingExpectedActor);
        }
        for _ in observed_actors.difference(&expected_actors) {
            findings.push(Finding::UnexpectedActor);
        }
        for actor in &branch.actor_responses {
            if !nonempty(&actor.actor_id)
                || !nonempty(&actor.policy_revision_digest)
                || !nonempty(&actor.policy_input_digest)
                || !nonempty(&actor.action_digest)
            {
                findings.push(Finding::EmptyIdentity("actor_response"));
            }
            if actor.origin == ActorResponseOrigin::Replayed && actor.randomness_digest.as_ref().is_some_and(|value| !nonempty(value)) {
                findings.push(Finding::EmptyIdentity("actor_randomness_digest"));
            }
        }

        if duplicate(&branch.expected_random_event_ids)
            || duplicate(&branch.random_events.iter().map(|event| event.event_id.clone()).collect::<Vec<_>>())
        {
            findings.push(Finding::DuplicateRandomEvent);
        }
        let expected_events = id_set(branch.expected_random_event_ids.clone());
        let observed_events = id_set(branch.random_events.iter().map(|event| event.event_id.clone()));
        for _ in expected_events.difference(&observed_events) {
            findings.push(Finding::MissingExpectedRandomEvent);
        }
        for _ in observed_events.difference(&expected_events) {
            findings.push(Finding::UnexpectedRandomEvent);
        }
        for event in &branch.random_events {
            if !nonempty(&event.event_id) || !nonempty(&event.key_digest) || !nonempty(&event.sampled_value_digest) {
                findings.push(Finding::EmptyIdentity("random_event"));
            }
        }

        validate_information_fields(
            &branch.policy_input_fields,
            branch.decision_tick,
            false,
            &mut findings,
        );

        if let Some(query) = &branch.query {
            if !nonempty(&query.query_id) {
                findings.push(Finding::EmptyIdentity("query_id"));
            }
            if query.issued_tick > branch.decision_tick {
                findings.push(Finding::QueryIssuedAfterDecision);
            }
            if query.answer_available_tick <= query.issued_tick {
                findings.push(Finding::QueryAnswerNotAfterIssue);
            }
            if query.outcome_available_tick <= query.issued_tick {
                findings.push(Finding::QueryOutcomeNotAfterIssue);
            }
            validate_information_fields(
                &query.inputs,
                query.issued_tick,
                true,
                &mut findings,
            );
        }
    }

    let query_ids = campaign
        .branches
        .iter()
        .filter_map(|branch| branch.query.as_ref().map(|query| query.query_id.clone()))
        .collect::<Vec<_>>();
    if duplicate(&query_ids) {
        findings.push(Finding::DuplicateQueryId);
    }

    for branch in &campaign.branches {
        if let Some(parent_id) = &branch.parent_branch_id {
            match branches_by_id.get(parent_id) {
                None => findings.push(Finding::MissingParent),
                Some(parent) => validate_branch_pair(branch, parent, &mut findings),
            }
        } else {
            if branch.actor_responses.iter().any(|actor| actor.origin == ActorResponseOrigin::Replayed) {
                findings.push(Finding::MissingParentActorForReplay);
            }
            if branch.random_events.iter().any(|event| event.shared_with_parent) {
                findings.push(Finding::DeclaredSharedKeyMismatch);
            }
        }
    }
    validate_parent_graph(&branches_by_id, &mut findings);

    let controls = campaign.evaluation_controls;
    if controls.frozen {
        if controls.teacher_blending_enabled {
            findings.push(Finding::FrozenTeacherBlending);
        }
        if controls.online_updates_enabled {
            findings.push(Finding::FrozenOnlineUpdate);
        }
        if controls.exploration_noise_enabled {
            findings.push(Finding::FrozenExploration);
        }
        if controls.teacher_inputs_visible {
            findings.push(Finding::FrozenTeacherInput);
        }
        if controls.future_outcomes_visible {
            findings.push(Finding::FrozenFutureOutcome);
        }
    }

    findings.sort();
    findings.dedup();
    let disposition = if findings.is_empty() {
        ReportDisposition::Valid
    } else {
        ReportDisposition::Rejected
    };
    ValidationReport {
        disposition,
        summary,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(origin: ActorResponseOrigin, input: &str, action: &str) -> ActorResponse {
        ActorResponse {
            actor_id: "lead-vehicle".to_string(),
            policy_revision_digest: "actor-policy-v1".to_string(),
            policy_input_digest: input.to_string(),
            action_digest: action.to_string(),
            randomness_digest: Some("actor-rng-0".to_string()),
            origin,
        }
    }

    fn exogenous_event() -> RandomEvent {
        RandomEvent {
            event_id: "weather-noise".to_string(),
            shared_with_parent: false,
            event_class: RandomEventClass::Exogenous,
            invariant_under_intervention: true,
            key_digest: "weather-key-1".to_string(),
            sampled_value_digest: "weather-value-1".to_string(),
        }
    }

    fn root_branch() -> BranchRecord {
        BranchRecord {
            branch_id: "root".to_string(),
            episode_id: "episode-1".to_string(),
            parent_branch_id: None,
            checkpoint_digest: "checkpoint-1".to_string(),
            recurrent_state_policy_id: "state-policy-1".to_string(),
            observation_history_digest: "history-1".to_string(),
            simulator_revision: "sim-1".to_string(),
            decision_tick: 10,
            outcome_tick: 20,
            intervention_digest: "action-root".to_string(),
            expected_actor_ids: vec!["lead-vehicle".to_string()],
            actor_responses: vec![actor(ActorResponseOrigin::Recomputed, "actor-input-1", "actor-action-1")],
            expected_random_event_ids: vec!["weather-noise".to_string()],
            random_events: vec![{
                let mut event = exogenous_event();
                event.shared_with_parent = true;
                event
            }],
            policy_input_fields: vec![InformationField {
                field_id: "camera-front".to_string(),
                source_role: InformationSourceRole::PolicyVisible,
                available_tick: 10,
            }],
            query: None,
        }
    }

    fn child_branch(id: &str, action: &str, actor_input: &str) -> BranchRecord {
        BranchRecord {
            branch_id: id.to_string(),
            episode_id: "episode-1".to_string(),
            parent_branch_id: Some("root".to_string()),
            checkpoint_digest: "checkpoint-1".to_string(),
            recurrent_state_policy_id: "state-policy-1".to_string(),
            observation_history_digest: "history-1".to_string(),
            simulator_revision: "sim-1".to_string(),
            decision_tick: 10,
            outcome_tick: 21,
            intervention_digest: action.to_string(),
            expected_actor_ids: vec!["lead-vehicle".to_string()],
            actor_responses: vec![actor(ActorResponseOrigin::Recomputed, actor_input, "actor-action-child")],
            expected_random_event_ids: vec!["weather-noise".to_string()],
            random_events: vec![exogenous_event()],
            policy_input_fields: vec![InformationField {
                field_id: "camera-front".to_string(),
                source_role: InformationSourceRole::PolicyVisible,
                available_tick: 10,
            }],
            query: None,
        }
    }

    fn valid_campaign() -> CampaignRecord {
        CampaignRecord {
            expected_episode_ids: vec!["episode-1".to_string()],
            expected_branch_ids: vec!["root".to_string(), "yield".to_string(), "proceed".to_string()],
            episodes: vec![EpisodeRecord {
                episode_id: "episode-1".to_string(),
                disposition: EpisodeDisposition::Completed,
            }],
            branches: vec![
                root_branch(),
                child_branch("yield", "action-yield", "actor-input-yield"),
                child_branch("proceed", "action-proceed", "actor-input-proceed"),
            ],
            evaluation_controls: FrozenEvaluationControls {
                frozen: true,
                teacher_blending_enabled: false,
                online_updates_enabled: false,
                exploration_noise_enabled: false,
                teacher_inputs_visible: false,
                future_outcomes_visible: false,
            },
        }
    }

    fn has(report: &ValidationReport, expected: Finding) -> bool {
        report.findings.contains(&expected)
    }

    #[test]
    fn accepts_complete_root_and_two_recomputed_sibling_interventions() {
        let report = validate_campaign(&valid_campaign());
        assert_eq!(report.disposition, ReportDisposition::Valid);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn rejects_missing_episode_and_keeps_expected_denominator() {
        let mut campaign = valid_campaign();
        campaign.episodes.clear();
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::MissingEpisode));
        assert_eq!(report.summary.expected_episodes, 1);
        assert_eq!(report.summary.observed_episodes, 0);
    }

    #[test]
    fn rejects_missing_expected_branch() {
        let mut campaign = valid_campaign();
        campaign.branches.pop();
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::MissingBranch));
    }

    #[test]
    fn rejects_duplicate_episode_and_branch_ids() {
        let mut campaign = valid_campaign();
        campaign.episodes.push(campaign.episodes[0].clone());
        campaign.branches.push(campaign.branches[0].clone());
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::DuplicateEpisode));
        assert!(has(&report, Finding::DuplicateBranch));
    }

    #[test]
    fn rejects_missing_parent() {
        let mut campaign = valid_campaign();
        campaign.branches[1].parent_branch_id = Some("absent".to_string());
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::MissingParent));
    }

    #[test]
    fn rejects_cross_episode_parent() {
        let mut campaign = valid_campaign();
        campaign.episodes.push(EpisodeRecord {
            episode_id: "episode-2".to_string(),
            disposition: EpisodeDisposition::Completed,
        });
        campaign.expected_episode_ids.push("episode-2".to_string());
        campaign.branches[1].episode_id = "episode-2".to_string();
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::CrossEpisodeParent));
    }

    #[test]
    fn rejects_decision_boundary_mismatch() {
        let mut campaign = valid_campaign();
        campaign.branches[1].observation_history_digest = "different-history".to_string();
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::BranchBoundaryMismatch));
    }

    #[test]
    fn rejects_lineage_cycle() {
        let mut campaign = valid_campaign();
        campaign.branches[0].parent_branch_id = Some("yield".to_string());
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::LineageCycle));
    }

    #[test]
    fn rejects_replayed_actor_action_when_policy_input_changes() {
        let mut campaign = valid_campaign();
        campaign.branches[1].actor_responses[0] =
            actor(ActorResponseOrigin::Replayed, "changed-actor-input", "actor-action-1");
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::ReplayedActorInputChanged));
    }

    #[test]
    fn accepts_replayed_actor_action_when_policy_input_and_output_are_identical() {
        let mut campaign = valid_campaign();
        campaign.branches[1].actor_responses[0] =
            actor(ActorResponseOrigin::Replayed, "actor-input-1", "actor-action-1");
        let report = validate_campaign(&campaign);
        assert!(!has(&report, Finding::ReplayedActorInputChanged));
        assert!(!has(&report, Finding::ReplayedActorActionChanged));
    }

    #[test]
    fn rejects_shared_random_key_for_endogenous_event() {
        let mut campaign = valid_campaign();
        campaign.branches[1].random_events[0].shared_with_parent = true;
        campaign.branches[1].random_events[0].event_class = RandomEventClass::Endogenous;
        campaign.branches[1].random_events[0].invariant_under_intervention = false;
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::SharedRandomKeyNotInvariantExogenous));
    }

    #[test]
    fn rejects_undocumented_shared_random_key() {
        let mut campaign = valid_campaign();
        campaign.branches[1].random_events[0].shared_with_parent = false;
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::SharedRandomKeyNotDeclared));
    }

    #[test]
    fn rejects_declared_shared_key_when_key_differs() {
        let mut campaign = valid_campaign();
        campaign.branches[1].random_events[0].shared_with_parent = true;
        campaign.branches[1].random_events[0].key_digest = "different-key".to_string();
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::DeclaredSharedKeyMismatch));
    }

    #[test]
    fn rejects_changed_value_for_shared_exogenous_key() {
        let mut campaign = valid_campaign();
        campaign.branches[1].random_events[0].sampled_value_digest = "different".to_string();
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::SharedRandomValueChanged));
    }

    #[test]
    fn accepts_shared_invariant_exogenous_event() {
        let report = validate_campaign(&valid_campaign());
        assert!(!has(&report, Finding::SharedRandomKeyNotInvariantExogenous));
        assert!(!has(&report, Finding::SharedRandomValueChanged));
    }

    #[test]
    fn rejects_reused_key_under_a_different_event_identity() {
        let mut campaign = valid_campaign();
        campaign.branches[1].random_events[0].event_id = "renamed-event".to_string();
        campaign.branches[1].random_events[0].shared_with_parent = false;
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::RandomKeyEventIdentityChanged));
    }

    #[test]
    fn rejects_root_event_claiming_parent_key_sharing() {
        let mut campaign = valid_campaign();
        campaign.branches[0].random_events[0].shared_with_parent = true;
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::DeclaredSharedKeyMismatch));
    }

    #[test]
    fn rejects_evaluator_only_and_future_policy_fields() {
        let mut campaign = valid_campaign();
        campaign.branches[0].policy_input_fields.push(InformationField {
            field_id: "oracle-collision".to_string(),
            source_role: InformationSourceRole::EvaluatorOnly,
            available_tick: 10,
        });
        campaign.branches[0].policy_input_fields.push(InformationField {
            field_id: "future-frame".to_string(),
            source_role: InformationSourceRole::PolicyVisible,
            available_tick: 11,
        });
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::PolicyFieldNotVisible));
        assert!(has(&report, Finding::PolicyFieldFromFuture));
    }

    #[test]
    fn rejects_query_using_teacher_input_and_future_answer() {
        let mut campaign = valid_campaign();
        campaign.branches[1].query = Some(QueryRecord {
            query_id: "query-1".to_string(),
            issued_tick: 10,
            answer_available_tick: 10,
            outcome_available_tick: 22,
            inputs: vec![InformationField {
                field_id: "teacher-control".to_string(),
                source_role: InformationSourceRole::TeacherOnly,
                available_tick: 9,
            }],
        });
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::QueryAnswerNotAfterIssue));
        assert!(has(&report, Finding::QueryInputNotPolicyVisible));
    }

    #[test]
    fn rejects_query_using_future_observation() {
        let mut campaign = valid_campaign();
        campaign.branches[1].query = Some(QueryRecord {
            query_id: "query-future".to_string(),
            issued_tick: 10,
            answer_available_tick: 12,
            outcome_available_tick: 22,
            inputs: vec![InformationField {
                field_id: "future-frame".to_string(),
                source_role: InformationSourceRole::PolicyVisible,
                available_tick: 11,
            }],
        });
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::QueryInputFromFuture));
    }

    #[test]
    fn rejects_frozen_evaluation_control_violations() {
        let mut campaign = valid_campaign();
        campaign.evaluation_controls.teacher_blending_enabled = true;
        campaign.evaluation_controls.online_updates_enabled = true;
        campaign.evaluation_controls.exploration_noise_enabled = true;
        campaign.evaluation_controls.teacher_inputs_visible = true;
        campaign.evaluation_controls.future_outcomes_visible = true;
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::FrozenTeacherBlending));
        assert!(has(&report, Finding::FrozenOnlineUpdate));
        assert!(has(&report, Finding::FrozenExploration));
        assert!(has(&report, Finding::FrozenTeacherInput));
        assert!(has(&report, Finding::FrozenFutureOutcome));
    }

    #[test]
    fn failed_incomplete_and_censored_episodes_remain_in_denominator() {
        let mut campaign = valid_campaign();
        campaign.expected_episode_ids.extend(["episode-2".to_string(), "episode-3".to_string()]);
        campaign.episodes.extend([
            EpisodeRecord { episode_id: "episode-2".to_string(), disposition: EpisodeDisposition::Failed },
            EpisodeRecord { episode_id: "episode-3".to_string(), disposition: EpisodeDisposition::Censored },
        ]);
        campaign.expected_branch_ids = vec!["root".to_string()];
        campaign.branches = vec![root_branch()];
        let report = validate_campaign(&campaign);
        assert_eq!(report.summary.observed_episodes, 3);
        assert_eq!(report.summary.failed_episodes, 1);
        assert_eq!(report.summary.censored_episodes, 1);
    }

    #[test]
    fn findings_are_order_independent() {
        let first = valid_campaign();
        let mut second = first.clone();
        second.episodes.reverse();
        second.branches.reverse();
        for branch in &mut second.branches {
            branch.actor_responses.reverse();
            branch.random_events.reverse();
        }
        assert_eq!(validate_campaign(&first), validate_campaign(&second));
    }

    #[test]
    fn rejects_branch_with_outcome_before_decision() {
        let mut campaign = valid_campaign();
        campaign.branches[1].outcome_tick = 9;
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::BranchOutcomeBeforeDecision));
    }

    #[test]
    fn rejects_unexpected_episode() {
        let mut campaign = valid_campaign();
        campaign.episodes.push(EpisodeRecord {
            episode_id: "unregistered".to_string(),
            disposition: EpisodeDisposition::Completed,
        });
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::UnexpectedEpisode));
    }

    #[test]
    fn rejects_missing_expected_actor_and_random_event() {
        let mut campaign = valid_campaign();
        campaign.branches[1].actor_responses.clear();
        campaign.branches[1].random_events.clear();
        let report = validate_campaign(&campaign);
        assert!(has(&report, Finding::MissingExpectedActor));
        assert!(has(&report, Finding::MissingExpectedRandomEvent));
    }
}
