//! Synthetic reciprocal road-policy producer.
//!
//! This crate composes the existing pure actor-policy core with the route-to-pose
//! adapter. It owns only the event-order / policy / pose-projection boundary.
//! A caller supplies per-agent time-to-conflict observations from its scenario
//! or world model, executes the returned joint pose targets in the physics
//! engine, and acknowledges the resulting physics-step snapshot digest before
//! the next policy tick.
//!
//! The built-in unit tests use synthetic observations. They do not validate
//! human behaviour, physical realism, collision avoidance or driving safety.

#![forbid(unsafe_code)]

use symtropy_road_actor_adapter::{
    AdapterError, CoordinateFrameId, MotionProfile, PathStepInput, PathStepReceipt, Point2,
    RouteId, StraightRoute, TickClock, plan_path_step,
};
use symtropy_road_actors::{
    ActorAction, ActorPolicy, ActorPolicyState, AllowedActorObservation, CautiousPausePolicy,
    CommittedAfterEntryPolicy, CueKind, CueValue, MotionBounds, ObservationCue, ObservationError,
    ObservationProvenance, PolicyError, ScenarioActorId, ScenarioTick,
};

#[derive(Clone, Debug, PartialEq)]
pub enum ProducerError {
    EmptyBranchId,
    InvalidActorId,
    InvalidObservedTimeToConflict,
    MissingPhysicsCommit,
    UnexpectedPhysicsCommit,
    InvalidPhysicsReceiptDigest,
    EventSequenceOverflow,
    Policy(PolicyError),
    Observation(ObservationError),
    Adapter(AdapterError),
    InvalidEventOrdering,
}

impl From<PolicyError> for ProducerError {
    fn from(value: PolicyError) -> Self {
        Self::Policy(value)
    }
}

impl From<ObservationError> for ProducerError {
    fn from(value: ObservationError) -> Self {
        Self::Observation(value)
    }
}

impl From<AdapterError> for ProducerError {
    fn from(value: AdapterError) -> Self {
        Self::Adapter(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActorStepReceipt {
    pub actor_id: String,
    pub policy_profile_id: &'static str,
    pub cue_source_actor_id: String,
    pub observed_time_to_conflict_seconds: f64,
    pub observation_generated_event_sequence: u64,
    pub observation_delivery_event_sequence: u64,
    pub input_cutoff_event_sequence: u64,
    pub decision_event_sequence: u64,
    pub pose_target_event_sequence: u64,
    pub command_latch_event_sequence: u64,
    pub prior_policy_state: ActorPolicyState,
    pub action: ActorAction,
    pub next_policy_state: ActorPolicyState,
    pub path_step: PathStepReceipt,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReciprocalStepReceipt {
    pub branch_id: String,
    pub tick: ScenarioTick,
    /// Physics event that produced the world snapshot from which these cues
    /// were generated. None is valid only for the initial scenario snapshot.
    pub prior_physics_event_sequence: Option<u64>,
    pub shared_input_cutoff_event_sequence: u64,
    pub ego: ActorStepReceipt,
    pub other_actor: ActorStepReceipt,
    pub max_command_latch_event_sequence: u64,
}

impl ReciprocalStepReceipt {
    pub fn validate_event_order(&self) -> Result<(), ProducerError> {
        let actors = [&self.ego, &self.other_actor];
        let mut all = Vec::with_capacity(10);

        for actor in actors {
            if actor.input_cutoff_event_sequence != self.shared_input_cutoff_event_sequence
                || actor.observation_generated_event_sequence
                    >= actor.observation_delivery_event_sequence
                || actor.observation_delivery_event_sequence
                    >= self.shared_input_cutoff_event_sequence
                || actor.decision_event_sequence <= self.shared_input_cutoff_event_sequence
                || actor.pose_target_event_sequence <= actor.decision_event_sequence
                || actor.command_latch_event_sequence <= actor.pose_target_event_sequence
            {
                return Err(ProducerError::InvalidEventOrdering);
            }
            if self
                .prior_physics_event_sequence
                .is_some_and(|prior| actor.observation_generated_event_sequence <= prior)
            {
                return Err(ProducerError::InvalidEventOrdering);
            }
            all.extend([
                actor.observation_generated_event_sequence,
                actor.observation_delivery_event_sequence,
                actor.decision_event_sequence,
                actor.pose_target_event_sequence,
                actor.command_latch_event_sequence,
            ]);
        }

        if actors[0].decision_event_sequence >= actors[1].pose_target_event_sequence
            || actors[1].decision_event_sequence >= actors[0].pose_target_event_sequence
        {
            return Err(ProducerError::InvalidEventOrdering);
        }

        all.sort_unstable();
        if all.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ProducerError::InvalidEventOrdering);
        }

        let expected_max_latch = self
            .ego
            .command_latch_event_sequence
            .max(self.other_actor.command_latch_event_sequence);
        if expected_max_latch != self.max_command_latch_event_sequence {
            return Err(ProducerError::InvalidEventOrdering);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicsCommitRecord {
    pub branch_id: String,
    pub tick: ScenarioTick,
    pub event_sequence: u64,
    /// Expected to be the independently produced post-step snapshot digest
    /// from the physics replay receipt. This crate validates format/order,
    /// not the authenticity of the physics producer.
    pub post_step_snapshot_digest: String,
}

#[derive(Clone, Copy, Debug)]
struct EventSequencer {
    next: u64,
}

impl EventSequencer {
    const fn new() -> Self {
        Self { next: 1 }
    }

    fn next(&mut self) -> Result<u64, ProducerError> {
        let value = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or(ProducerError::EventSequenceOverflow)?;
        Ok(value)
    }
}

/// Explicit synthetic policy profiles supported by the reciprocal conformance producer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorPolicyProfile {
    /// Waits for a sufficient gap and can pause while crossing on a fresh conflict cue.
    CautiousPause,
    /// Waits for entry, then continues after committing to the crossing.
    CommittedAfterEntry,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ActorPolicyInstance {
    CautiousPause(CautiousPausePolicy),
    CommittedAfterEntry(CommittedAfterEntryPolicy),
}

impl ActorPolicyInstance {
    fn new(profile: ActorPolicyProfile, bounds: MotionBounds) -> Result<Self, ProducerError> {
        match profile {
            ActorPolicyProfile::CautiousPause => Ok(Self::CautiousPause(CautiousPausePolicy::new(
                bounds, 2.5, 3.5, 0,
            )?)),
            ActorPolicyProfile::CommittedAfterEntry => Ok(Self::CommittedAfterEntry(
                CommittedAfterEntryPolicy::new(bounds, 3.5, 0)?,
            )),
        }
    }
}

impl ActorPolicy for ActorPolicyInstance {
    fn profile_id(&self) -> &'static str {
        match self {
            Self::CautiousPause(policy) => policy.profile_id(),
            Self::CommittedAfterEntry(policy) => policy.profile_id(),
        }
    }

    fn transition(
        &self,
        prior: ActorPolicyState,
        observation: &AllowedActorObservation,
    ) -> Result<symtropy_road_actors::ActorTransition, PolicyError> {
        match self {
            Self::CautiousPause(policy) => policy.transition(prior, observation),
            Self::CommittedAfterEntry(policy) => policy.transition(prior, observation),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ActorRouteSpec<'a> {
    actor_id: &'a str,
    route_id: &'a str,
    coordinate_frame_id: &'a str,
    start: Point2,
    end: Point2,
    observation_source_id: u64,
}

#[derive(Clone, Debug)]
struct ActorRuntime {
    id: ScenarioActorId,
    policy: ActorPolicyInstance,
    state: ActorPolicyState,
    route: StraightRoute,
    motion_profile: MotionProfile,
    clock: TickClock,
    progress_m: f64,
    measured_forward_speed_mps: f64,
    source_id: u64,
}

impl ActorRuntime {
    fn new(
        route_spec: ActorRouteSpec<'_>,
        policy: ActorPolicyInstance,
        motion_profile: MotionProfile,
        clock: TickClock,
    ) -> Result<Self, ProducerError> {
        Ok(Self {
            id: ScenarioActorId::new(route_spec.actor_id)
                .map_err(|_| ProducerError::InvalidActorId)?,
            policy,
            state: ActorPolicyState::waiting(),
            route: StraightRoute::new(
                RouteId::new(route_spec.route_id)?,
                CoordinateFrameId::new(route_spec.coordinate_frame_id)?,
                route_spec.start,
                route_spec.end,
            )?,
            motion_profile,
            clock,
            progress_m: 0.0,
            measured_forward_speed_mps: 0.0,
            source_id: route_spec.observation_source_id,
        })
    }

    fn plan(&self, decision_state: ActorPolicyState) -> Result<PathStepReceipt, ProducerError> {
        Ok(plan_path_step(&PathStepInput {
            actor_id: &self.id,
            policy_profile_id: self.policy.profile_id(),
            policy_state: decision_state,
            route: Some(&self.route),
            motion_profile: self.motion_profile,
            clock: self.clock,
            elapsed_ticks: 1,
            prior_progress_m: self.progress_m,
            measured_forward_speed_mps: self.measured_forward_speed_mps,
        })?)
    }

    fn commit_path_step(&mut self, state: ActorPolicyState, receipt: &PathStepReceipt) {
        self.state = state;
        self.progress_m = receipt.next_progress_m;
        self.measured_forward_speed_mps = receipt.applied_forward_speed_mps;
    }
}

/// A two-agent producer with branch-global event ordering.
///
/// Each call to `step` evaluates both policies against separate observations.
/// Both observations share a single input cutoff; all policy decisions occur
/// before pose projection and command latching. A later physics consumer must
/// call `record_physics_commit` before another policy tick can be produced.
#[derive(Clone, Debug)]
pub struct ReciprocalPolicyProducer {
    branch_id: String,
    next_tick: u64,
    sequence: EventSequencer,
    ego: ActorRuntime,
    other_actor: ActorRuntime,
    pending_physics_commit: bool,
    prior_physics_event_sequence: Option<u64>,
    last_latch_event_sequence: Option<u64>,
}

impl ReciprocalPolicyProducer {
    pub fn new(branch_id: impl Into<String>) -> Result<Self, ProducerError> {
        Self::new_with_profiles(
            branch_id,
            ActorPolicyProfile::CautiousPause,
            ActorPolicyProfile::CautiousPause,
        )
    }

    /// Construct the synthetic two-agent producer with independently selected
    /// policy profiles. Profile choices are configuration, not hidden policy input.
    pub fn new_with_profiles(
        branch_id: impl Into<String>,
        ego_profile: ActorPolicyProfile,
        other_actor_profile: ActorPolicyProfile,
    ) -> Result<Self, ProducerError> {
        let branch_id = branch_id.into();
        if branch_id.trim().is_empty() {
            return Err(ProducerError::EmptyBranchId);
        }

        let bounds = MotionBounds::new(1.5)?;
        let ego_policy = ActorPolicyInstance::new(ego_profile, bounds)?;
        let other_actor_policy = ActorPolicyInstance::new(other_actor_profile, bounds)?;
        let motion_profile = MotionProfile::new(bounds, 5.0, 5.0)?;
        let clock = TickClock::new(10)?;

        let ego = ActorRuntime::new(
            ActorRouteSpec {
                actor_id: "ego",
                route_id: "ego-eastbound",
                coordinate_frame_id: "intersection-frame",
                start: Point2::new(0.0, 0.0)?,
                end: Point2::new(100.0, 0.0)?,
                observation_source_id: 1,
            },
            ego_policy,
            motion_profile,
            clock,
        )?;
        let other_actor = ActorRuntime::new(
            ActorRouteSpec {
                actor_id: "actor-1",
                route_id: "actor-northbound",
                coordinate_frame_id: "intersection-frame",
                start: Point2::new(50.0, -50.0)?,
                end: Point2::new(50.0, 50.0)?,
                observation_source_id: 2,
            },
            other_actor_policy,
            motion_profile,
            clock,
        )?;

        Ok(Self {
            branch_id,
            next_tick: 0,
            sequence: EventSequencer::new(),
            ego,
            other_actor,
            pending_physics_commit: false,
            prior_physics_event_sequence: None,
            last_latch_event_sequence: None,
        })
    }

    pub fn next_tick(&self) -> ScenarioTick {
        ScenarioTick(self.next_tick)
    }

    pub fn prior_physics_event_sequence(&self) -> Option<u64> {
        self.prior_physics_event_sequence
    }

    pub fn ego_measured_forward_speed_mps(&self) -> f64 {
        self.ego.measured_forward_speed_mps
    }

    pub fn other_measured_forward_speed_mps(&self) -> f64 {
        self.other_actor.measured_forward_speed_mps
    }

    /// Produce one reciprocal decision/latch step.
    ///
    /// `observed_ttc_seconds[0]` is the TTC delivered to ego, sourced from
    /// the other actor's relevant state. `[1]` is delivered to actor-1,
    /// sourced from ego's relevant state. The scenario/world model owns how
    /// these values are estimated; this adapter never fabricates perception.
    pub fn step(
        &mut self,
        observed_ttc_seconds: [f64; 2],
    ) -> Result<ReciprocalStepReceipt, ProducerError> {
        if self.pending_physics_commit {
            return Err(ProducerError::MissingPhysicsCommit);
        }
        if observed_ttc_seconds
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(ProducerError::InvalidObservedTimeToConflict);
        }

        let tick = ScenarioTick(self.next_tick);
        let prior_physics_event_sequence = self.prior_physics_event_sequence;

        // Generate source observations from the same frozen pre-decision
        // snapshot, then deliver each only to its named recipient.
        let ego_generated = self.sequence.next()?;
        let actor_generated = self.sequence.next()?;
        let ego_delivered = self.sequence.next()?;
        let actor_delivered = self.sequence.next()?;
        let shared_cutoff = self.sequence.next()?;

        let ego_observation = AllowedActorObservation::new(
            tick,
            vec![ObservationCue::observed(
                CueKind::EgoTimeToConflictSeconds,
                ObservationProvenance::new(self.other_actor.source_id, ego_generated)
                    .expect("non-zero source and event sequence"),
                CueValue::TimeToConflictSeconds(observed_ttc_seconds[0]),
                tick,
                tick,
            )],
        )?;
        let actor_observation = AllowedActorObservation::new(
            tick,
            vec![ObservationCue::observed(
                CueKind::EgoTimeToConflictSeconds,
                ObservationProvenance::new(self.ego.source_id, actor_generated)
                    .expect("non-zero source and event sequence"),
                CueValue::TimeToConflictSeconds(observed_ttc_seconds[1]),
                tick,
                tick,
            )],
        )?;

        // Both policies consume their own closed, delivered observation view.
        // Decisions are committed before any action is projected or latched.
        let ego_prior_state = self.ego.state;
        let actor_prior_state = self.other_actor.state;
        let ego_transition = self
            .ego
            .policy
            .transition(ego_prior_state, &ego_observation)?;
        let actor_transition = self
            .other_actor
            .policy
            .transition(actor_prior_state, &actor_observation)?;
        let ego_decision_sequence = self.sequence.next()?;
        let actor_decision_sequence = self.sequence.next()?;

        // Pose projection is downstream of both decisions. The two pose
        // projection events precede both command latches.
        let ego_path = self.ego.plan(ego_transition.next_state())?;
        let actor_path = self.other_actor.plan(actor_transition.next_state())?;
        let ego_pose_sequence = self.sequence.next()?;
        let actor_pose_sequence = self.sequence.next()?;
        let ego_latch_sequence = self.sequence.next()?;
        let actor_latch_sequence = self.sequence.next()?;

        let ego_receipt = ActorStepReceipt {
            actor_id: self.ego.id.as_str().to_owned(),
            policy_profile_id: self.ego.policy.profile_id(),
            cue_source_actor_id: self.other_actor.id.as_str().to_owned(),
            observed_time_to_conflict_seconds: observed_ttc_seconds[0],
            observation_generated_event_sequence: ego_generated,
            observation_delivery_event_sequence: ego_delivered,
            input_cutoff_event_sequence: shared_cutoff,
            decision_event_sequence: ego_decision_sequence,
            pose_target_event_sequence: ego_pose_sequence,
            command_latch_event_sequence: ego_latch_sequence,
            prior_policy_state: ego_prior_state,
            action: ego_transition.action(),
            next_policy_state: ego_transition.next_state(),
            path_step: ego_path.clone(),
        };
        let actor_receipt = ActorStepReceipt {
            actor_id: self.other_actor.id.as_str().to_owned(),
            policy_profile_id: self.other_actor.policy.profile_id(),
            cue_source_actor_id: self.ego.id.as_str().to_owned(),
            observed_time_to_conflict_seconds: observed_ttc_seconds[1],
            observation_generated_event_sequence: actor_generated,
            observation_delivery_event_sequence: actor_delivered,
            input_cutoff_event_sequence: shared_cutoff,
            decision_event_sequence: actor_decision_sequence,
            pose_target_event_sequence: actor_pose_sequence,
            command_latch_event_sequence: actor_latch_sequence,
            prior_policy_state: actor_prior_state,
            action: actor_transition.action(),
            next_policy_state: actor_transition.next_state(),
            path_step: actor_path.clone(),
        };

        let max_latch = ego_latch_sequence.max(actor_latch_sequence);
        let receipt = ReciprocalStepReceipt {
            branch_id: self.branch_id.clone(),
            tick,
            prior_physics_event_sequence,
            shared_input_cutoff_event_sequence: shared_cutoff,
            ego: ego_receipt,
            other_actor: actor_receipt,
            max_command_latch_event_sequence: max_latch,
        };
        receipt.validate_event_order()?;

        self.ego
            .commit_path_step(ego_transition.next_state(), &ego_path);
        self.other_actor
            .commit_path_step(actor_transition.next_state(), &actor_path);
        self.pending_physics_commit = true;
        self.last_latch_event_sequence = Some(max_latch);
        Ok(receipt)
    }

    /// Acknowledge the completed physics transition after the consumer has
    /// executed the exact latched pose commands and captured its post-step
    /// replay snapshot. This does not authenticate the supplied digest.
    pub fn record_physics_commit(
        &mut self,
        post_step_snapshot_digest: impl Into<String>,
    ) -> Result<PhysicsCommitRecord, ProducerError> {
        if !self.pending_physics_commit {
            return Err(ProducerError::UnexpectedPhysicsCommit);
        }
        let digest = post_step_snapshot_digest.into();
        if !digest_is_valid(&digest) {
            return Err(ProducerError::InvalidPhysicsReceiptDigest);
        }
        let next_tick = self
            .next_tick
            .checked_add(1)
            .ok_or(ProducerError::EventSequenceOverflow)?;
        let event_sequence = self.sequence.next()?;
        if self
            .last_latch_event_sequence
            .is_some_and(|latch| event_sequence <= latch)
        {
            return Err(ProducerError::InvalidEventOrdering);
        }
        let record = PhysicsCommitRecord {
            branch_id: self.branch_id.clone(),
            tick: ScenarioTick(self.next_tick),
            event_sequence,
            post_step_snapshot_digest: digest,
        };
        self.prior_physics_event_sequence = Some(event_sequence);
        self.next_tick = next_tick;
        self.pending_physics_commit = false;
        Ok(record)
    }
}

fn digest_is_valid(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn producer() -> ReciprocalPolicyProducer {
        ReciprocalPolicyProducer::new("synthetic-crossing-001").unwrap()
    }

    fn coupled_ttc(other_speed_mps: f64) -> f64 {
        // Deliberately stylized observer fixture: faster crossing traffic
        // reduces the reported time-to-conflict. This is not a calibrated
        // kinematic TTC model and is not used outside these conformance tests.
        (4.0 - 2.0 * other_speed_mps).max(0.1)
    }

    #[test]
    fn both_policies_share_a_cutoff_but_receive_only_their_own_cues() {
        let mut producer = producer();
        let step = producer.step([4.0, 4.0]).unwrap();
        step.validate_event_order().unwrap();

        assert_eq!(step.ego.actor_id, "ego");
        assert_eq!(step.ego.cue_source_actor_id, "actor-1");
        assert_eq!(step.other_actor.actor_id, "actor-1");
        assert_eq!(step.other_actor.cue_source_actor_id, "ego");
        assert_eq!(
            step.ego.input_cutoff_event_sequence,
            step.other_actor.input_cutoff_event_sequence
        );
        assert_eq!(step.ego.action, ActorAction::EnterCrossing);
        assert_eq!(step.other_actor.action, ActorAction::EnterCrossing);
        assert!(
            step.ego.observation_delivery_event_sequence < step.shared_input_cutoff_event_sequence
        );
        assert!(
            step.other_actor.observation_delivery_event_sequence
                < step.shared_input_cutoff_event_sequence
        );
        assert!(step.max_command_latch_event_sequence > step.ego.decision_event_sequence);
        assert!(step.max_command_latch_event_sequence > step.other_actor.decision_event_sequence);
    }

    #[test]
    fn next_policy_tick_requires_an_acknowledged_physics_commit() {
        let mut producer = producer();
        let first = producer.step([4.0, 4.0]).unwrap();
        assert_eq!(
            producer.step([4.0, 4.0]),
            Err(ProducerError::MissingPhysicsCommit)
        );

        let physics = producer.record_physics_commit("a".repeat(64)).unwrap();
        assert!(physics.event_sequence > first.max_command_latch_event_sequence);
        let second = producer.step([3.0, 3.0]).unwrap();
        assert_eq!(
            second.prior_physics_event_sequence,
            Some(physics.event_sequence)
        );
        assert!(second.ego.observation_generated_event_sequence > physics.event_sequence);
        assert!(second.other_actor.observation_generated_event_sequence > physics.event_sequence);
    }

    #[test]
    fn other_agents_actions_change_the_next_reciprocal_observation_and_policy_response() {
        let mut producer = producer();

        let first = producer.step([4.0, 4.0]).unwrap();
        assert_eq!(first.ego.action, ActorAction::EnterCrossing);
        assert_eq!(first.other_actor.action, ActorAction::EnterCrossing);
        producer.record_physics_commit("b".repeat(64)).unwrap();

        let ego_ttc = coupled_ttc(producer.other_measured_forward_speed_mps());
        let actor_ttc = coupled_ttc(producer.ego_measured_forward_speed_mps());
        assert_eq!(ego_ttc, 3.0);
        assert_eq!(actor_ttc, 3.0);
        let second = producer.step([ego_ttc, actor_ttc]).unwrap();
        assert_eq!(second.ego.action, ActorAction::Continue);
        assert_eq!(second.other_actor.action, ActorAction::Continue);
        producer.record_physics_commit("c".repeat(64)).unwrap();

        let ego_ttc = coupled_ttc(producer.other_measured_forward_speed_mps());
        let actor_ttc = coupled_ttc(producer.ego_measured_forward_speed_mps());
        assert_eq!(ego_ttc, 2.0);
        assert_eq!(actor_ttc, 2.0);
        let third = producer.step([ego_ttc, actor_ttc]).unwrap();
        assert_eq!(third.ego.action, ActorAction::Pause);
        assert_eq!(third.other_actor.action, ActorAction::Pause);
    }

    #[test]
    fn asymmetric_reciprocal_observations_can_produce_distinct_joint_actions() {
        let mut producer = producer();
        producer.step([4.0, 4.0]).unwrap();
        producer.record_physics_commit("d".repeat(64)).unwrap();

        let step = producer.step([2.0, 4.0]).unwrap();
        assert_eq!(step.ego.action, ActorAction::Pause);
        assert_eq!(step.other_actor.action, ActorAction::Continue);
    }

    #[test]
    fn cautious_ego_and_committed_actor_are_compared_on_the_same_reciprocal_cues() {
        let mut producer = ReciprocalPolicyProducer::new_with_profiles(
            "cautious-vs-committed",
            ActorPolicyProfile::CautiousPause,
            ActorPolicyProfile::CommittedAfterEntry,
        )
        .unwrap();

        let entry = producer.step([4.0, 4.0]).unwrap();
        assert_eq!(entry.ego.action, ActorAction::EnterCrossing);
        assert_eq!(entry.other_actor.action, ActorAction::EnterCrossing);
        assert_eq!(entry.ego.policy_profile_id, CautiousPausePolicy::PROFILE_ID);
        assert_eq!(
            entry.other_actor.policy_profile_id,
            CommittedAfterEntryPolicy::PROFILE_ID
        );
        producer.record_physics_commit("f".repeat(64)).unwrap();

        let conflict = producer.step([2.0, 2.0]).unwrap();
        assert_eq!(conflict.ego.action, ActorAction::Pause);
        assert_eq!(conflict.other_actor.action, ActorAction::Continue);
        conflict.validate_event_order().unwrap();
    }

    #[test]
    fn event_order_rejects_a_delivery_at_or_after_the_shared_cutoff() {
        let mut producer = producer();
        let mut step = producer.step([4.0, 4.0]).unwrap();
        step.ego.observation_delivery_event_sequence = step.shared_input_cutoff_event_sequence;
        assert_eq!(
            step.validate_event_order(),
            Err(ProducerError::InvalidEventOrdering)
        );
    }

    #[test]
    fn physics_receipt_digest_is_required_and_must_be_hex() {
        let mut producer = producer();
        producer.step([4.0, 4.0]).unwrap();
        assert_eq!(
            producer.record_physics_commit("not-a-digest"),
            Err(ProducerError::InvalidPhysicsReceiptDigest)
        );
        assert_eq!(
            producer.record_physics_commit("g".repeat(64)),
            Err(ProducerError::InvalidPhysicsReceiptDigest)
        );
    }

    #[test]
    fn physics_commit_cannot_be_recorded_before_a_joint_action_is_latched() {
        let mut producer = producer();
        assert_eq!(
            producer.record_physics_commit("e".repeat(64)),
            Err(ProducerError::UnexpectedPhysicsCommit)
        );
    }
}
