// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Two-sided, policy-in-the-loop synthetic crossing producer.
//!
//! This is a bounded execution/conformance fixture, not a driving controller,
//! pedestrian model, empirical simulator, or safety authority.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use blake3::Hasher;
use nalgebra::SVector;
use road_egci_handoff::{
    BranchExport, BranchOutcome, CandidateUniverseDisposition, CorrespondenceReceipt,
    DecisionDisposition, DrawBundleAnchor, EpisodeExport, ExperimentManifest,
    ExternalAnchor, HypothesisSetDisposition, JointTransition, ObservationDelivery,
    ObservationDisposition, PolicyDecision, PolicyProfile, ReplayDisposition,
    ReplayEvidence, TransitionDisposition, export_to_sam, SamHandoff,
};
use symtropy_math::{Bivector, Point, Rotor, Sphere, Transform};
use symtropy_physics::{
    PhysicsWorld,
    body::{BodyHandle, BodyType, RigidBody},
    replay::{
        ReplayEvidenceContext, ReplayTape, WorldCommand,
        WorldSnapshot, apply_commands, execute_with_evidence, verify_replay_evidence,
    },
};
use symtropy_road_actor_adapter::{
    CoordinateFrameId, MotionProfile, PathStepInput, Point2, RouteId, StraightRoute,
    TickClock, plan_path_step,
};
use symtropy_road_actors::{
    ActorAction, ActorPolicy, ActorPolicyState, AllowedActorObservation,
    CautiousPausePolicy, CueKind, CueValue, MotionBounds, ObservationCue,
    ObservationProvenance, ScenarioActorId, ScenarioTick,
};

const DT: f64 = 0.2;
const TICK_HZ: u64 = 5;
const TICK_COUNT: u64 = 4;
const CONFLICT_X_M: f64 = 3.0;
const ACTOR_ZONE_HALF_WIDTH_M: f64 = 0.08;
const ACTOR_ID: &str = "actor-1";
const EGO_ID: &str = "ego";
const EPISODE_SCHEMA: &str = "symtropy.road-egci-reciprocal-episode.v0.2";

#[derive(Clone, Copy, Debug, PartialEq)]
struct EgoPolicyState {
    last_requested_speed_mps: f64,
    last_decision_tick: Option<u64>,
}

impl EgoPolicyState {
    const fn initial() -> Self {
        Self {
            last_requested_speed_mps: 0.0,
            last_decision_tick: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EgoAction {
    Continue,
    Yield,
}

impl EgoAction {
    const fn id(self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::Yield => "yield",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct EgoObservation {
    actor_in_conflict_zone: bool,
    speed_limit_mps: f64,
}

/// Small transparent reference rule. It has no access to actor policy state,
/// actor requested action, hidden world state, or future commands.
fn ego_policy(
    observation: EgoObservation,
) -> (EgoAction, f64) {
    if observation.actor_in_conflict_zone {
        (EgoAction::Yield, 0.0)
    } else {
        (EgoAction::Continue, observation.speed_limit_mps)
    }
}

#[derive(Debug)]
struct BranchRun {
    export: BranchExport,
}

#[derive(Debug)]
pub struct ReciprocalExperiment {
    pub episode: EpisodeExport,
    pub anchor: ExternalAnchor,
    pub handoff: SamHandoff,
}

pub fn run_reciprocal_experiment() -> Result<ReciprocalExperiment, String> {
    let base = run_branch(1.0, "ego-speed-1mps")?;
    let intervention = run_branch(2.0, "ego-speed-2mps")?;

    if base.export.parent_snapshot_digest != intervention.export.parent_snapshot_digest {
        return Err("paired branches did not share the exact initial snapshot".into());
    }

    let initial_actor_state = actor_state_digest(ActorPolicyState::waiting());
    let initial_ego_state = ego_state_digest(EgoPolicyState::initial());
    let action_domain_digest = digest("road-egci-action-domain-v0.1");

    let policies = vec![
        PolicyProfile {
            actor_id: ACTOR_ID.into(),
            policy_digest: digest("symtropy-road-actors:cautious_pause_v1:source-16886c48"),
            config_digest: digest("cautious_pause_v1:max=1.5:pause_ttc=2.5:entry_ttc=3.5:reaction_ticks=0"),
            initial_policy_state_digest: initial_actor_state,
            observation_profile_digest: actor_observation_profile_digest(),
            action_domain_digest: action_domain_digest.clone(),
        },
        PolicyProfile {
            actor_id: EGO_ID.into(),
            policy_digest: digest("road-egci-reference-ego-zone-yield-v0.1"),
            config_digest: digest("ego-observation=actor-zone+own-speed-limit;yield-if-zone"),
            initial_policy_state_digest: initial_ego_state,
            observation_profile_digest: ego_observation_profile_digest(),
            action_domain_digest: action_domain_digest.clone(),
        },
    ];

    let manifest = ExperimentManifest {
        experiment_id: "road-egci-reciprocal-crossing-v0.1".into(),
        scenario_digest: digest("scenario:two-kinematic-bodies:straight-crossing:v0.1"),
        odd_scope_digest: digest("odd:single-intersection:two-agents:four-ticks"),
        initial_world_snapshot_digest: base.export.parent_snapshot_digest.clone(),
        physics_profile_digest: digest("symtropy-physics:2d:kinematic-spheres:zero-gravity:v0.2"),
        execution_profile_digest: digest("single-threaded:5hz:dt=0.2:four-frames:runtime-replay"),
        observation_schedule_digest: digest("observation-schedule:delivered-at-tick:phase-aware-v0.2"),
        action_domain_digest,
        units_profile_digest: digest("SI:metres:seconds:metres-per-second:radians"),
        tick_hz_numerator: TICK_HZ,
        tick_hz_denominator: 1,
    };

    let episode = EpisodeExport {
        schema: EPISODE_SCHEMA.into(),
        manifest,
        policies,
        branches: vec![base.export, intervention.export],
    };

    let draw_bundle_anchors = episode
        .branches
        .iter()
        .map(|branch| DrawBundleAnchor {
            branch_id: branch.branch_id.clone(),
            expected_bundle_digest: branch.correspondences[0].draw_bundle_digest.clone(),
        })
        .collect();

    let anchor = ExternalAnchor {
        authority_reference: "local-synthetic-fixture-anchor-not-an-external-authority".into(),
        expected_episode_root: episode.episode_root().map_err(|error| format!("{error:?}"))?,
        draw_bundle_anchors,
    };
    let handoff = export_to_sam(&episode, &anchor).map_err(|error| format!("{error:?}"))?;

    Ok(ReciprocalExperiment {
        episode,
        anchor,
        handoff,
    })
}

fn run_branch(speed_limit_mps: f64, branch_id: &str) -> Result<BranchRun, String> {
    if !speed_limit_mps.is_finite() || !(0.0..=3.0).contains(&speed_limit_mps) || speed_limit_mps == 0.0 {
        return Err("declared ego speed limit is outside the finite fixture envelope".into());
    }

    let actor_id = ScenarioActorId::new(ACTOR_ID).map_err(|error| format!("{error:?}"))?;
    let actor_bounds = MotionBounds::new(1.5).map_err(|error| format!("{error:?}"))?;
    let actor_policy = CautiousPausePolicy::new(actor_bounds, 2.5, 3.5, 0)
        .map_err(|error| format!("{error:?}"))?;
    let actor_motion_profile = MotionProfile::new(actor_bounds, 10.0, 10.0)
        .map_err(|error| format!("{error:?}"))?;
    let actor_route = StraightRoute::new(
        RouteId::new("pedestrian-crossing-route-1").map_err(|error| format!("{error:?}"))?,
        CoordinateFrameId::new("intersection-local-metric-frame").map_err(|error| format!("{error:?}"))?,
        Point2::new(2.0, -1.0).map_err(|error| format!("{error:?}"))?,
        Point2::new(2.0, 1.0).map_err(|error| format!("{error:?}"))?,
    )
    .map_err(|error| format!("{error:?}"))?;
    let clock = TickClock::new(TICK_HZ).map_err(|error| format!("{error:?}"))?;

    let (mut live_world, ego_handle, actor_handle) = build_world()?;
    let mut tape = ReplayTape::<2>::default();
    let mut recorded_snapshots = Vec::with_capacity(TICK_COUNT as usize);
    let mut observations = Vec::new();
    let mut decisions = Vec::new();
let mut ego_visible = Vec::<String>::new();
    let mut actor_visible = Vec::<String>::new();
    let mut cue_index = BTreeMap::<String, ObservationDelivery>::new();
    let mut ego_prior_receipts = Vec::<String>::new();
    let mut actor_prior_receipts = Vec::<String>::new();

    let mut actor_state = ActorPolicyState::waiting();
    let mut actor_progress_m = 0.3;
    let mut actor_measured_speed_mps = 0.0;
    let mut ego_state = EgoPolicyState::initial();

    for tick in 0..TICK_COUNT {
        let ego_body = live_world
            .body(ego_handle)
            .ok_or_else(|| "ego body disappeared from PhysicsWorld".to_owned())?;
        let actor_body = live_world
            .body(actor_handle)
            .ok_or_else(|| "actor body disappeared from PhysicsWorld".to_owned())?;
        let ego_x = ego_body.transform.translation.0[0];
        let ego_speed = ego_body.linear_velocity[0].max(0.0);
        let actor_y = actor_body.transform.translation.0[1];

        let actor_in_conflict_zone = actor_y.abs() <= ACTOR_ZONE_HALF_WIDTH_M;
        let actor_ttc_seconds = if tick == 0 {
            5.0
        } else if ego_speed > 1e-9 {
            ((CONFLICT_X_M - ego_x).max(0.0) / ego_speed).max(0.0)
        } else {
            10.0
        };

        let phase = observation_phase(tick);
        let (new_observations, actor_cue_id, ego_zone_cue_id, speed_limit_cue_id) =
            make_tick_observations(
                tick,
                actor_ttc_seconds,
                actor_in_conflict_zone,
                speed_limit_mps,
                phase,
            );
        for cue in new_observations {
            let cue_id = cue.cue_id.clone();
            match cue.recipient_actor_id.as_str() {
                ACTOR_ID => actor_visible.push(cue_id.clone()),
                EGO_ID => ego_visible.push(cue_id.clone()),
                _ => return Err("observation builder emitted an unknown recipient".into()),
            }
            cue_index.insert(cue_id, cue.clone());
            observations.push(cue);
        }

        // The synthetic ego rule receives only its declared observation allow-list.
        // The speed limit is an ego-visible intervention cue; actor state is not passed.
        let ego_observation = EgoObservation {
            actor_in_conflict_zone,
            speed_limit_mps,
        };
        let (ego_action, requested_ego_speed) = ego_policy(ego_observation);
        let ego_prior_state_digest = ego_state_digest(ego_state);
        let ego_next_state = EgoPolicyState {
            last_requested_speed_mps: requested_ego_speed,
            last_decision_tick: Some(tick),
        };
        let ego_next_state_digest = ego_state_digest(ego_next_state);
        let ego_payload_digest = digest(&format!(
            "ego-action:{}:tick={tick}:speed={:.9}:state={}",
            ego_action.id(),
            requested_ego_speed,
            ego_next_state_digest
        ));

        // The actor policy receives only one validated TTC cue, never the ego's
        // policy state or command. TTC is computed from the last realized world.
        let provenance = ObservationProvenance::new(1, phase)
            .ok_or_else(|| "invalid observation provenance".to_owned())?;
        let actor_observation = AllowedActorObservation::new(
            ScenarioTick(tick),
            vec![ObservationCue::observed(
                CueKind::EgoTimeToConflictSeconds,
                provenance,
                CueValue::TimeToConflictSeconds(actor_ttc_seconds),
                ScenarioTick(tick.saturating_sub(1)),
                ScenarioTick(tick),
            )],
        )
        .map_err(|error| format!("{error:?}"))?;
        let actor_prior_state_digest = actor_state_digest(actor_state);
        let actor_transition = actor_policy
            .transition(actor_state, &actor_observation)
            .map_err(|error| format!("{error:?}"))?;
        let actor_action_id = actor_action_id(actor_transition.action());
        let next_actor_state = actor_transition.next_state();
        let path_receipt = plan_path_step(&PathStepInput {
            actor_id: &actor_id,
            policy_profile_id: actor_policy.profile_id(),
            policy_state: next_actor_state,
            route: Some(&actor_route),
            motion_profile: actor_motion_profile,
            clock,
            elapsed_ticks: 1,
            prior_progress_m,
            measured_forward_speed_mps: actor_measured_speed_mps,
        })
        .map_err(|error| format!("{error:?}"))?;
        let actor_next_state_digest = actor_state_digest(next_actor_state);
        let actor_payload_digest = digest(&format!(
            "actor-action:{actor_action_id}:tick={tick}:path-receipt={}:next-state={actor_next_state_digest}",
            hex(path_receipt.receipt_digest.as_slice())
        ));

        let (ego_applied_speed, ego_distance) =
            rate_limited_step(ego_speed, requested_ego_speed, DT)?;
        let ego_next_x = ego_x + ego_distance;
        let ego_command = kinematic_pose_command(
            ego_handle,
            [ego_next_x, 0.0],
            0.0,
            [ego_applied_speed, 0.0],
        );
        let actor_command = kinematic_pose_command(
            actor_handle,
            [path_receipt.target.position.x_m, path_receipt.target.position.y_m],
            path_receipt.target.heading_radians,
            [
                path_receipt.target.linear_velocity_mps.x_m,
                path_receipt.target.linear_velocity_mps.y_m,
            ],
        );

        let cutoff = event_base(tick);
        let (ego_decision_sequence, actor_decision_sequence) = (cutoff + 1, cutoff + 2);
        let (ego_latch_sequence, actor_latch_sequence) = (cutoff + 3, cutoff + 4);
        
        let ego_receipt_id = format!("decision-{EGO_ID}-{tick}");
        let actor_receipt_id = format!("decision-{ACTOR_ID}-{tick}");
        let ego_visible_digest = permitted_history_digest(
            EGO_ID,
            tick,
            cutoff,
            &ego_prior_state_digest,
            &ego_observation_profile_digest(),
            &ego_visible,
            &cue_index,
            &ego_prior_receipts,
        )?;
        let actor_visible_digest = permitted_history_digest(
            ACTOR_ID,
            tick,
            cutoff,
            &actor_prior_state_digest,
            &actor_observation_profile_digest(),
            &actor_visible,
            &cue_index,
            &actor_prior_receipts,
        )?;

        decisions.push(PolicyDecision {
            decision_receipt_id: ego_receipt_id.clone(),
            actor_id: EGO_ID.into(),
            tick,
            input_cutoff_event_sequence: cutoff,
            decision_event_sequence: ego_decision_sequence,
            policy_digest: ego_policy_digest(),
            config_digest: ego_config_digest(),
            prior_policy_state_digest: ego_prior_state_digest,
            next_policy_state_digest: Some(ego_next_state_digest),
            visible_cue_ids: ego_visible.clone(),
            consumed_cue_ids: vec![speed_limit_cue_id.clone(), ego_zone_cue_id.clone()],
            prior_action_receipt_ids: ego_prior_receipts.clone(),
            permitted_history_digest: ego_visible_digest,
            deadline_tick: tick + 1,
            command_latch_tick: Some(tick),
            command_latch_event_sequence: Some(ego_latch_sequence),
            action_id: Some(ego_action.id().into()),
            action_payload_digest: Some(ego_payload_digest),
            disposition: DecisionDisposition::Completed,
        });
        decisions.push(PolicyDecision {
            decision_receipt_id: actor_receipt_id.clone(),
            actor_id: ACTOR_ID.into(),
            tick,
            input_cutoff_event_sequence: cutoff,
            decision_event_sequence: actor_decision_sequence,
            policy_digest: actor_policy_digest(),
            config_digest: actor_config_digest(),
            prior_policy_state_digest: actor_prior_state_digest,
            next_policy_state_digest: Some(actor_next_state_digest),
            visible_cue_ids: actor_visible.clone(),
            consumed_cue_ids: vec![actor_cue_id.clone()],
            prior_action_receipt_ids: actor_prior_receipts.clone(),
            permitted_history_digest: actor_visible_digest,
            deadline_tick: tick + 1,
            command_latch_tick: Some(tick),
            command_latch_event_sequence: Some(actor_latch_sequence),
            action_id: Some(actor_action_id.into()),
            action_payload_digest: Some(actor_payload_digest),
            disposition: DecisionDisposition::Completed,
        });

        ego_prior_receipts.push(ego_receipt_id);
        actor_prior_receipts.push(actor_receipt_id);
        // Both decisions are complete before either command is latched.
        let commands = vec![ego_command, actor_command];
        tape.push_frame(DT, commands.clone());
        apply_commands(&mut live_world, &commands).map_err(|error| format!("{error:?}"))?;
        live_world.step(DT);

        recorded_snapshots.push(WorldSnapshot::capture(&live_world));
        ego_state = ego_next_state;
        actor_state = next_actor_state;
        actor_progress_m = path_receipt.next_progress_m;
        actor_measured_speed_mps = path_receipt.applied_forward_speed_mps;
    }

    let (mut mirror_world, _, _) = build_world()?;
    for (index, frame) in tape.frames.iter().enumerate() {
        apply_commands(&mut mirror_world, &frame.commands)
            .map_err(|error| format!("{error:?}"))?;
        mirror_world.step(frame.dt);
        if WorldSnapshot::capture(&mirror_world) != recorded_snapshots[index] {
            return Err(format!("fresh-world snapshot mismatch at tick {index}"));
        }
    }

    let replay_context = ReplayEvidenceContext {
        scenario_id: format!("road-egci-reciprocal-{branch_id}"),
        scenario_artifact_digest: digest("scenario:two-agent-crossing:v0.1"),
        engine_artifact_digest: digest("symtropy-physics-replay-engine:0.2.1"),
        initial_world_profile_digest: digest("world:ego-kinematic-sphere:actor-kinematic-sphere:gravity-zero"),
        environment_profile_digest: digest("environment:closed-synthetic-intersection:no-exogenous-force"),
        execution_protocol_digest: digest("single-threaded:three-ticks:0.2s:pose-command-v2"),
    };
    let (mut evidence_world, _, _) = build_world()?;
    let replay_receipt = execute_with_evidence(&mut evidence_world, &tape, &replay_context)
        .map_err(|error| format!("{error:?}"))?;
    let (mut verifier_world, _, _) = build_world()?;
    if !verify_replay_evidence(&mut verifier_world, &tape, &replay_context, &replay_receipt)
        .map_err(|error| format!("{error:?}"))?
    {
        return Err("physics replay receipt did not verify in an independent fresh world".into());
    }

    let mut transitions = Vec::with_capacity(TICK_COUNT as usize);
    let mut parent_snapshot = replay_receipt.initial_snapshot_digest.clone();
    for tick in 0..TICK_COUNT as usize {
        let child_snapshot = replay_receipt.tick_snapshot_digests[tick].clone();
        let tick_decisions: Vec<_> = decisions
            .iter()
            .filter(|decision| decision.tick == tick as u64)
            .cloned()
            .collect();
        transitions.push(JointTransition {
            tick: tick as u64,
            physics_event_sequence: physics_event_sequence(tick as u64),
            parent_snapshot_digest: parent_snapshot.clone(),
            child_snapshot_digest: child_snapshot.clone(),
            joint_action_digest: joint_action_digest(&tick_decisions),
            physics_step_receipt_digest: digest(&format!(
                "physics-step-commitment-v0.1:{}:{}:{tick}:{child_snapshot}",
                replay_receipt.receipt_digest,
                replay_receipt.input_tape_digest
            )),
            disposition: TransitionDisposition::Completed,
        });
        parent_snapshot = child_snapshot;
    }

    let correspondences = vec![CorrespondenceReceipt {
        hypothesis_id: "scenario-authored-actor-id-v0.1".into(),
        correspondence_digest: digest("map:scenario-actor-id-to-kinematic-body"),
        causal_dispositions_digest: digest("causal-dispositions:delivered-ego-zone-and-ttc-cues"),
        projection_input_digest: digest("projection-input:route-and-initial-body-profiles"),
        draw_bundle_digest: digest(&format!("draw-bundle:{branch_id}:actor-keyed-v0.1")),
        verifier_profile_digest: digest("verifier:scenario-authored-identity-map:v0.1"),
        verifier_receipt_digest: digest("verifier-result:identity-map-consistent:v0.1"),
    }];

    let mut branch = BranchExport {
        branch_id: branch_id.into(),
        parent_snapshot_digest: replay_receipt.initial_snapshot_digest.clone(),
        intervention_digest: digest(&format!("ego-speed-limit:{speed_limit_mps:.6}")),
        outcome: BranchOutcome::Completed,
        // No complete candidate-action support universe has been established.
        // This intentionally keeps the SAM query ineligible as a support claim.
        candidate_universe: CandidateUniverseDisposition::Incomplete,
        hypothesis_set: HypothesisSetDisposition::ExhaustiveWithinDeclaredModelFamily,
        correspondences,
        observations,
        decisions,
        transitions,
        replay: ReplayEvidence {
            replay_profile_digest: digest("physics-replay-profile:same-runtime-fresh-world:v0.2"),
            expected_branch_payload_root: String::new(),
            replayed_branch_payload_root: None,
            disposition: ReplayDisposition::ExactSameRuntimeBytes,
        },
    };
    let payload_root = branch.payload_root();
    branch.replay.expected_branch_payload_root = payload_root.clone();
    branch.replay.replayed_branch_payload_root = Some(payload_root);

    Ok(BranchRun { export: branch })
}

fn build_world() -> Result<(PhysicsWorld<2>, BodyHandle, BodyHandle), String> {
    let mut world = PhysicsWorld::<2>::new(SVector::<f64, 2>::zeros());
    let ego = RigidBody::<2>::new(
        BodyHandle(0),
        BodyType::Kinematic,
        Transform::from_translation(Point::new([0.0, 0.0])),
        Box::new(Sphere::<2>::new(Point::<2>::origin(), 0.12)),
        0.0,
        SVector::<f64, 2>::zeros(),
    );
    let actor = RigidBody::<2>::new(
        BodyHandle(0),
        BodyType::Kinematic,
        Transform::from_translation(Point::new([2.0, -0.7])),
        Box::new(Sphere::<2>::new(Point::<2>::origin(), 0.12)),
        0.0,
        SVector::<f64, 2>::zeros(),
    );
    let ego_handle = world.add_body(ego);
    let actor_handle = world.add_body(actor);
    if ego_handle == actor_handle {
        return Err("PhysicsWorld allocated duplicate handles".into());
    }
    Ok((world, ego_handle, actor_handle))
}

fn kinematic_pose_command(
    body: BodyHandle,
    translation: [f64; 2],
    heading_radians: f64,
    linear_velocity: [f64; 2],
) -> WorldCommand<2> {
    let plane = Bivector::<2>::unit_plane(0, 1);
    let rotation = Rotor::from_plane_angle(&plane, heading_radians);
    WorldCommand::SetKinematicPose {
        body,
        transform: Box::new(Transform {
            translation: Point::new(translation),
            rotation,
        }),
        linear_velocity: Box::new(SVector::from(linear_velocity)),
        angular_velocity: Box::new(Bivector::<2>::zero()),
    }
}

fn rate_limited_step(measured_speed: f64, requested_speed: f64, dt: f64) -> Result<(f64, f64), String> {
    if !measured_speed.is_finite()
        || !requested_speed.is_finite()
        || measured_speed < 0.0
        || requested_speed < 0.0
        || !dt.is_finite()
        || dt <= 0.0
    {
        return Err("ego kinematic input is outside the finite forward-motion envelope".into());
    }
    let delta = requested_speed - measured_speed;
    let applied = if delta >= 0.0 {
        measured_speed + delta.min(10.0 * dt)
    } else {
        measured_speed - (-delta).min(10.0 * dt)
    };
    let distance = 0.5 * (measured_speed + applied) * dt;
    if !applied.is_finite() || !distance.is_finite() {
        return Err("ego rate-limited motion produced a non-finite value".into());
    }
    Ok((applied, distance))
}

fn make_tick_observations(
    tick: u64,
    actor_ttc_seconds: f64,
    actor_in_conflict_zone: bool,
    speed_limit_mps: f64,
    phase: u64,
) -> (Vec<ObservationDelivery>, String, String, String) {
    let actor_cue_id = format!("actor-ego-ttc-{tick}");
    let ego_zone_cue_id = format!("ego-actor-zone-{tick}");
    let mut output = Vec::with_capacity(if tick == 0 { 3 } else { 2 });

    if tick == 0 {
        output.push(observation_delivery(
            actor_cue_id.clone(),
            "source-actor-ttc-t0",
            ACTOR_ID,
            digest(&format!("actor-ego-ttc:{actor_ttc_seconds:.9}")),
            tick,
            1,
            4,
            1,
            actor_observation_profile_digest(),
        ));
        output.push(observation_delivery(
            ego_zone_cue_id.clone(),
            "source-ego-zone-t0",
            EGO_ID,
            digest("actor-in-zone:false"),
            tick,
            2,
            5,
            1,
            ego_observation_profile_digest(),
        ));
        let speed_limit_cue_id = "ego-speed-limit-0".to_owned();
        output.push(observation_delivery(
            speed_limit_cue_id.clone(),
            &format!("source-ego-speed-limit:{speed_limit_mps:.6}"),
            EGO_ID,
            digest(&format!("ego-speed-limit:{speed_limit_mps:.9}")),
            tick,
            3,
            6,
            2,
            ego_observation_profile_digest(),
        ));
        return (output, actor_cue_id, ego_zone_cue_id, speed_limit_cue_id);
    }

    let delivery_base = phase + 1;
    let source = format!("world-snapshot-after-tick-{}", tick - 1);
    output.push(observation_delivery(
        actor_cue_id.clone(),
        &source,
        ACTOR_ID,
        digest(&format!("actor-ego-ttc:{actor_ttc_seconds:.9}")),
        tick,
        phase,
        delivery_base,
        tick + 1,
        actor_observation_profile_digest(),
    ));
    output.push(observation_delivery(
        ego_zone_cue_id.clone(),
        &source,
        EGO_ID,
        digest(&format!("actor-in-zone:{actor_in_conflict_zone}")),
        tick,
        phase,
        delivery_base + 1,
        tick + 2,
        ego_observation_profile_digest(),
    ));
    (
        output,
        actor_cue_id,
        ego_zone_cue_id,
        "ego-speed-limit-0".into(),
    )
}

fn observation_delivery(
    cue_id: String,
    source_event_id: &str,
    recipient_actor_id: &str,
    observation_payload_digest: String,
    tick: u64,
    generated_event_sequence: u64,
    delivered_event_sequence: u64,
    delivery_sequence: u64,
    visibility_profile_digest: String,
) -> ObservationDelivery {
    ObservationDelivery {
        cue_id,
        source_event_id: source_event_id.into(),
        recipient_actor_id: recipient_actor_id.into(),
        observation_payload_digest,
        // The cue is derived from the last committed physics snapshot and
        // delivered to the policy in the current decision epoch.
        observed_tick: tick.saturating_sub(1),
        generated_tick: tick,
        generated_event_sequence,
        delivered_tick: Some(tick),
        delivered_event_sequence: Some(delivered_event_sequence),
        delivery_sequence,
        visibility_profile_digest,
        disposition: ObservationDisposition::Delivered,
    }
}

fn observation_phase(tick: u64) -> u64 {
    if tick == 0 {
        1
    } else {
        13 + 9 * (tick - 1)
    }
}

fn event_base(tick: u64) -> u64 {
    if tick == 0 {
        7
    } else {
        observation_phase(tick) + 3
    }
}

fn physics_event_sequence(tick: u64) -> u64 {
    event_base(tick) + 5
}

fn actor_action_id(action: ActorAction) -> &'static str {
    match action {
        ActorAction::Wait => "wait",
        ActorAction::EnterCrossing => "enter-crossing",
        ActorAction::Continue => "continue",
        ActorAction::Pause => "pause",
        ActorAction::Complete => "complete",
    }
}

fn actor_state_digest(state: ActorPolicyState) -> String {
    let mode = match state.mode() {
        symtropy_road_actors::ActorMode::Waiting => "waiting",
        symtropy_road_actors::ActorMode::Crossing => "crossing",
        symtropy_road_actors::ActorMode::Paused => "paused",
        symtropy_road_actors::ActorMode::Complete => "complete",
    };
    let tick = state.last_decision_tick().map_or(u64::MAX, |value| value.0);
    digest(&format!(
        "actor-state:{mode}:speed={:016x}:last-tick={tick}",
        state.target_forward_speed_mps().to_bits()
    ))
}

fn ego_state_digest(state: EgoPolicyState) -> String {
    let tick = state.last_decision_tick.unwrap_or(u64::MAX);
    digest(&format!(
        "ego-state:last-requested-speed={:016x}:last-tick={tick}",
        state.last_requested_speed_mps.to_bits()
    ))
}

fn actor_observation_profile_digest() -> String {
    digest("actor-observation-allow-list:v0.1:ego-ttc-only")
}

fn ego_observation_profile_digest() -> String {
    digest("ego-observation-allow-list:v0.1:actor-zone+own-speed-limit")
}

fn actor_policy_digest() -> String {
    digest("symtropy-road-actors:cautious_pause_v1:source-16886c48")
}

fn actor_config_digest() -> String {
    digest("cautious_pause_v1:max=1.5:pause_ttc=2.5:entry_ttc=3.5:reaction_ticks=0")
}

fn ego_policy_digest() -> String {
    digest("road-egci-reference-ego-zone-yield-v0.1")
}

fn ego_config_digest() -> String {
    digest("ego-observation=actor-zone+own-speed-limit;yield-if-zone")
}

fn permitted_history_digest(
    actor_id: &str,
    tick: u64,
    input_cutoff_event_sequence: u64,
    prior_policy_state_digest: &str,
    observation_profile_digest: &str,
    visible_cue_ids: &[String],
    cues: &BTreeMap<String, ObservationDelivery>,
    prior_action_receipt_ids: &[String],
) -> Result<String, String> {
    let mut history = Hasher::new_derive_key("Symtropy EGCI permitted policy history v0.2");
    hash_str(&mut history, actor_id);
    hash_u64(&mut history, tick);
    hash_u64(&mut history, input_cutoff_event_sequence);
    hash_str(&mut history, prior_policy_state_digest);
    hash_str(&mut history, observation_profile_digest);
    hash_len(&mut history, visible_cue_ids.len());

    for cue_id in visible_cue_ids {
        let cue = cues
            .get(cue_id)
            .ok_or_else(|| format!("visible cue {cue_id} is missing"))?;
        let delivered_tick = cue
            .delivered_tick
            .ok_or_else(|| format!("visible cue {cue_id} was not delivered"))?;
        let delivered_sequence = cue
            .delivered_event_sequence
            .ok_or_else(|| format!("visible cue {cue_id} has no delivery event"))?;
        hash_str(&mut history, &cue.cue_id);
        hash_str(&mut history, &cue.source_event_id);
        hash_str(&mut history, &cue.observation_payload_digest);
        hash_u64(&mut history, cue.observed_tick);
        hash_u64(&mut history, cue.generated_tick);
        hash_u64(&mut history, cue.generated_event_sequence);
        hash_u64(&mut history, delivered_tick);
        hash_u64(&mut history, delivered_sequence);
        hash_u64(&mut history, cue.delivery_sequence);
        hash_str(&mut history, &cue.visibility_profile_digest);
    }
    hash_len(&mut history, prior_action_receipt_ids.len());
    for receipt_id in prior_action_receipt_ids {
        hash_str(&mut history, receipt_id);
    }
    Ok(hex(history.finalize().as_bytes()))
}

fn joint_action_digest(decisions: &[PolicyDecision]) -> String {
    let mut ordered: Vec<_> = decisions.iter().collect();
    ordered.sort_by(|left, right| left.actor_id.cmp(&right.actor_id));
    let mut h = Hasher::new_derive_key("Symtropy EGCI joint action receipt v0.1");
    hash_len(&mut h, ordered.len());
    for decision in ordered {
        hash_str(&mut h, &decision.actor_id);
        hash_str(&mut h, decision.action_id.as_deref().unwrap_or_default());
        hash_str(&mut h, decision.action_payload_digest.as_deref().unwrap_or_default());
    }
    hex(h.finalize().as_bytes())
}


fn observation_phase(tick: u64) -> u64 {
    if tick == 0 {
        1
    } else {
        13 + 9 * (tick - 1)
    }
}

fn event_base(tick: u64) -> u64 {
    if tick == 0 {
        7
    } else {
        observation_phase(tick) + 3
    }
}

fn physics_event_sequence(tick: u64) -> u64 {
    event_base(tick) + 5
}

fn hash_strs(h: &mut Hasher, values: &[String]) {
    hash_len(h, values.len());
    for value in values {
        hash_str(h, value);
    }
}

fn hash_str(h: &mut Hasher, value: &str) {
    hash_len(h, value.len());
    h.update(value.as_bytes());
}

fn hash_opt_str(h: &mut Hasher, value: Option<&str>) {
    match value {
        Some(value) => {
            h.update(&[1]);
            hash_str(h, value);
        }
        None => {
            h.update(&[0]);
        }
    }
}

fn hash_opt_u64(h: &mut Hasher, value: Option<u64>) {
    match value {
        Some(value) => {
            h.update(&[1]);
            hash_u64(h, value);
        }
        None => {
            h.update(&[0]);
        }
    }
}

fn hash_u64(h: &mut Hasher, value: u64) {
    h.update(&value.to_le_bytes());
}

fn hash_len(h: &mut Hasher, length: usize) {
    let length = u64::try_from(length).expect("finite episode length fits u64");
    h.update(&length.to_le_bytes());
}

fn digest(value: &str) -> String {
    blake3::hash(value.as_bytes()).to_hex().to_string()
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision<'a>(branch: &'a BranchExport, actor: &str, tick: u64) -> &'a PolicyDecision {
        branch.decisions.iter().find(|item| item.actor_id == actor && item.tick == tick).unwrap()
    }

    #[test]
    fn reciprocal_feedback_runs_only_after_its_observation_is_delivered() {
        let experiment = run_reciprocal_experiment().unwrap();
        let base = experiment
            .episode
            .branches
            .iter()
            .find(|branch| branch.branch_id == "ego-speed-1mps")
            .unwrap();
        let fast = experiment
            .episode
            .branches
            .iter()
            .find(|branch| branch.branch_id == "ego-speed-2mps")
            .unwrap();

        assert_eq!(decision(base, EGO_ID, 0).action_id.as_deref(), Some("continue"));
        assert_eq!(decision(fast, EGO_ID, 0).action_id.as_deref(), Some("continue"));
        assert_eq!(decision(base, ACTOR_ID, 0).action_id.as_deref(), Some("enter-crossing"));
        assert_eq!(decision(fast, ACTOR_ID, 0).action_id.as_deref(), Some("enter-crossing"));

        // Ego-to-actor: the tick-0 speed intervention only changes actor TTC
        // after the first physics step and observation delivery at tick 1.
        assert_eq!(decision(base, ACTOR_ID, 1).action_id.as_deref(), Some("continue"));
        assert_eq!(decision(fast, ACTOR_ID, 1).action_id.as_deref(), Some("pause"));
        assert_eq!(decision(base, ACTOR_ID, 2).action_id.as_deref(), Some("continue"));
        assert_eq!(decision(fast, ACTOR_ID, 2).action_id.as_deref(), Some("pause"));

        // Actor-to-ego: those actor actions produce different realized actor
        // positions; ego reacts only to the resulting tick-2 delivered zone cue.
        assert_eq!(decision(base, EGO_ID, 1).action_id.as_deref(), Some("continue"));
        assert_eq!(decision(fast, EGO_ID, 1).action_id.as_deref(), Some("continue"));
        assert_eq!(decision(base, EGO_ID, 2).action_id.as_deref(), Some("continue"));
        assert_eq!(decision(fast, EGO_ID, 2).action_id.as_deref(), Some("continue"));
        assert_eq!(decision(base, EGO_ID, 3).action_id.as_deref(), Some("yield"));
        assert_eq!(decision(fast, EGO_ID, 3).action_id.as_deref(), Some("continue"));

        assert_eq!(experiment.handoff.branches.len(), 2);
        assert!(experiment.handoff.branches.iter().all(|branch| {
            branch.queries.len() == 1
                && branch.queries[0].query.steps.len() == 4
                && !branch.queries[0].query.complete
        }));
    }

    #[test]
    fn event_ordinals_enforce_input_cutoff_decision_latch_and_physics_order() {
        let experiment = run_reciprocal_experiment().unwrap();
        for branch in &experiment.episode.branches {
            for decision in &branch.decisions {
                assert!(decision.input_cutoff_event_sequence < decision.decision_event_sequence);
                assert!(decision.command_latch_event_sequence.unwrap() > decision.decision_event_sequence);
                let cue = branch.observations.iter().find(|cue| {
                    cue.cue_id == decision.consumed_cue_ids.last().unwrap().as_str()
                }).unwrap();
                assert!(cue.delivered_event_sequence.unwrap() < decision.input_cutoff_event_sequence);
            }
            for transition in &branch.transitions {
                let at_tick: Vec<_> = branch.decisions.iter().filter(|item| item.tick == transition.tick).collect();
                assert!(at_tick.iter().all(|item| item.command_latch_event_sequence.unwrap() < transition.physics_event_sequence));
            }
        }
    }

    #[test]
    fn observations_bind_prior_physics_epoch_and_current_delivery_epoch() {
        let experiment = run_reciprocal_experiment().unwrap();
        for branch in &experiment.episode.branches {
            for cue in branch.observations.iter().filter(|cue| cue.generated_tick > 0) {
                assert_eq!(cue.observed_tick + 1, cue.generated_tick);
                assert_eq!(cue.delivered_tick, Some(cue.generated_tick));
                assert!(cue.generated_event_sequence < cue.delivered_event_sequence.unwrap());
            }
        }
    }

    #[test]
    fn post_cutoff_same_tick_cue_is_rejected_as_a_future_leak() {
        let mut experiment = run_reciprocal_experiment().unwrap();
        let branch = experiment
            .episode
            .branches
            .iter_mut()
            .find(|branch| branch.branch_id == "ego-speed-1mps")
            .unwrap();
        let cue = branch.observations.iter_mut().find(|cue| cue.cue_id == "actor-ego-ttc-1").unwrap();
        // Tick 1's shared cutoff is 16. Delivery at 16 is not in the closed input.
        cue.delivered_event_sequence = Some(16);
        let payload_root = branch.payload_root();
        branch.replay.expected_branch_payload_root = payload_root.clone();
        branch.replay.replayed_branch_payload_root = Some(payload_root);
        experiment.anchor.expected_episode_root = experiment.episode.episode_root().unwrap();
        assert!(matches!(
            export_to_sam(&experiment.episode, &experiment.anchor),
            Err(road_egci_handoff::HandoffError::FutureCueLeak)
        ));
    }

    #[test]
    fn every_policy_receipt_has_an_independent_state_chain_and_prior_action_lineage() {
        let experiment = run_reciprocal_experiment().unwrap();
        for branch in &experiment.episode.branches {
            for actor in [EGO_ID, ACTOR_ID] {
                let first = decision(branch, actor, 0);
                let second = decision(branch, actor, 1);
                let third = decision(branch, actor, 2);
                let fourth = decision(branch, actor, 3);
                assert_eq!(second.prior_policy_state_digest, first.next_policy_state_digest.clone().unwrap());
                assert_eq!(third.prior_policy_state_digest, second.next_policy_state_digest.clone().unwrap());
                assert_eq!(fourth.prior_policy_state_digest, third.next_policy_state_digest.clone().unwrap());
                assert_eq!(second.prior_action_receipt_ids, vec![first.decision_receipt_id.clone()]);
                assert_eq!(third.prior_action_receipt_ids, vec![
                    first.decision_receipt_id.clone(),
                    second.decision_receipt_id.clone(),
                ]);
                assert_eq!(fourth.prior_action_receipt_ids, vec![
                    first.decision_receipt_id.clone(),
                    second.decision_receipt_id.clone(),
                    third.decision_receipt_id.clone(),
                ]);
            }
        }
    }
}
