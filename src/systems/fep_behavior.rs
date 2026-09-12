// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! FEP-driven NPC behavior: free energy gradient minimization.

use bevy::prelude::*;
use std::collections::HashMap;
use symthaea_fep::Observation;

use crate::components::{
    CrewNpc, MoveTarget, NoiseEmitter, NpcActionEvent, NpcActionKind, NullDrone, Player,
    PowerJunction, WaterPump, WorldFeedbackEvent,
};
use crate::resources::{EnergyWell, PhysicsWorldRes, TutorialScenarioRes, TutorialStep};
use crate::systems::fep_perception::{
    LocalScalarSample, LocalTargetSample, PerceivedWorldFrame, local_noise_risk_cue,
    presented_distress_cue, strongest_local_sample, strongest_local_target,
};
use symtropy_render_bridge::PhysicsBody;

const FEP_OBSERVATION_DIM: usize = 6;
/// Abstract game-space range for direct local equipment/noise/social sensing.
const LOCAL_PERCEPTION_RANGE: f32 = 300.0;
/// Jack's authored close-threat awareness remains intentionally tighter than general sensing.
const LOCAL_DRONE_ATTENTION_RANGE: f32 = 250.0;
/// Close inspection/repair distance at which private diagnostic state may be consulted.
const LOCAL_DIAGNOSTIC_RANGE: f32 = 30.0;
/// Deterministic state-transition threshold for Leo's relapse warning event.
const LEO_RELAPSE_ALERT_THRESHOLD: f32 = 0.8;
/// Danger is refreshed every behavior pass, but retains a tiny horizon for future
/// sensor adapters that may update less frequently.
const DANGER_MEMORY_GENERATIONS: u64 = 2;
/// Infrastructure estimates may remain useful longer than a transient noise cue.
const INFRASTRUCTURE_MEMORY_GENERATIONS: u64 = 180;

/// Preserve the existing six-dimensional FEP model while making every slot explicit.
///
/// Self-state may enter directly. External danger/water/power values are supplied by
/// the observer-local perception adapter rather than authoritative global resources.
fn fep_observation_values(
    energy_fraction: f64,
    self_allostatic_load: f64,
    perceived_danger: f64,
    caution: f64,
    perceived_water: f64,
    perceived_power: f64,
) -> [f64; FEP_OBSERVATION_DIM] {
    [
        energy_fraction.clamp(0.0, 1.0),
        self_allostatic_load.clamp(0.0, 1.0),
        perceived_danger.clamp(0.0, 1.0),
        caution.clamp(0.0, 1.0),
        perceived_water.clamp(0.0, 1.0),
        perceived_power.clamp(0.0, 1.0),
    ]
}

fn blend_toward_local_target(
    direction: nalgebra::SVector<f64, 2>,
    npc_pos: Vec2,
    target_pos: Vec2,
    existing_weight: f64,
    target_weight: f64,
) -> nalgebra::SVector<f64, 2> {
    let delta = nalgebra::SVector::from([
        (target_pos.x - npc_pos.x) as f64,
        (target_pos.y - npc_pos.y) as f64,
    ]);
    let norm = delta.norm();
    if !norm.is_finite() || norm <= 2.0 {
        return direction;
    }
    direction * existing_weight + (delta / norm) * target_weight
}

/// Convert an observer-facing output scalar into degradation salience.
///
/// Hidden failure causes are intentionally absent from this API. A fully performing
/// machine presents no degradation cue even if some undisclosed internal flag exists.
fn presented_output_degradation(output: f32) -> f64 {
    if !output.is_finite() {
        return 0.0;
    }
    f64::from((1.0 - output.clamp(0.0, 1.0)).clamp(0.0, 1.0))
}

/// Convert observable pump operation into degradation salience without exposing cause.
fn presented_pump_degradation(is_running: bool, efficiency: f32) -> f64 {
    let visible_output = if is_running {
        efficiency.clamp(0.0, 1.0)
    } else {
        0.0
    };
    presented_output_degradation(visible_output)
}

/// True only for a finite upward crossing of a finite threshold.
///
/// This gives authored events a deterministic transition gate rather than a per-frame
/// random chance. Already-above states and non-finite values fail closed.
fn crossed_upward_threshold(previous: f32, current: f32, threshold: f32) -> bool {
    previous.is_finite()
        && current.is_finite()
        && threshold.is_finite()
        && previous < threshold
        && current >= threshold
}

/// Run the FEP perception-action cycle for each crew NPC.
pub fn fep_behavior_system(
    mut npcs: Query<
        (
            Entity,
            &mut CrewNpc,
            &Transform,
            &mut MoveTarget,
            &mut NoiseEmitter,
            &PhysicsBody,
            Option<&mut crate::systems::psychology::PsychologicalNeeds>,
        ),
        (With<CrewNpc>, Without<Player>),
    >,
    player_query: Query<(&Transform, &PhysicsBody), With<Player>>,
    other_npcs: Query<(&Transform, &PhysicsBody), (With<CrewNpc>, Without<Player>)>,
    wells: Query<(&Transform, &EnergyWell)>,
    physics: Res<PhysicsWorldRes>,
    power_junctions: Query<(&Transform, &PowerJunction)>,
    water_pumps: Query<(&Transform, &WaterPump)>,
    drones: Query<(&Transform, &NullDrone)>,
    other_noise_sources: Query<(&Transform, &NoiseEmitter), Without<CrewNpc>>,
    time: Res<Time>,
    tutorial_res: Option<Res<TutorialScenarioRes>>,
    mut perceived_world: Local<HashMap<Entity, PerceivedWorldFrame>>,
) {
    let Some((player_tf, player_body)) = player_query.iter().next() else {
        return;
    };
    let player_pos = player_tf.translation.truncate();

    // These inputs remain owned by the consciousness-physics provider. They are not
    // target-memory records and FEP-05 deliberately does not reinterpret them as such.
    let well_data: Vec<(nalgebra::SVector<f64, 2>, f64)> = wells
        .iter()
        .filter(|(_, w)| w.is_active())
        .map(|(tf, w)| {
            (
                nalgebra::SVector::from([tf.translation.x as f64, tf.translation.y as f64]),
                w.fraction_remaining(),
            )
        })
        .collect();

    let mut all_agents: Vec<(nalgebra::SVector<f64, 2>, [f64; 9])> = Vec::new();

    if let Some(entity) = physics.consciousness.entities.get(&player_body.handle) {
        all_agents.push((
            nalgebra::SVector::from([player_pos.x as f64, player_pos.y as f64]),
            entity.harmony_activations,
        ));
    }
    for (tf, body) in &other_npcs {
        if let Some(entity) = physics.consciousness.entities.get(&body.handle) {
            all_agents.push((
                nalgebra::SVector::from([tf.translation.x as f64, tf.translation.y as f64]),
                entity.harmony_activations,
            ));
        }
    }

    // Pre-gather observer-facing NPC presentation before mutable iteration. Exact
    // allostatic state remains self/private state; other agents receive only the coarse
    // outward distress tier defined by the perception adapter.
    struct NpcInfo {
        entity: Entity,
        name: String,
        pos: Vec2,
        distress_cue: f64,
        noise_level: f32,
    }
    let npc_infos: Vec<NpcInfo> = npcs
        .iter()
        .map(|(entity, npc, tf, _, noise, _, psych)| NpcInfo {
            entity,
            name: npc.name.clone(),
            pos: tf.translation.truncate(),
            distress_cue: presented_distress_cue(
                psych.as_ref().map_or(0.0, |p| p.allostatic_load),
            ),
            noise_level: noise.level,
        })
        .collect();

    let mut noise_samples: Vec<LocalScalarSample> = other_noise_sources
        .iter()
        .map(|(tf, noise)| LocalScalarSample {
            position: tf.translation.truncate(),
            value: f64::from(noise.level.clamp(0.0, 1.0)),
        })
        .collect();
    noise_samples.extend(npc_infos.iter().map(|info| LocalScalarSample {
        position: info.pos,
        value: f64::from(info.noise_level.clamp(0.0, 1.0)),
    }));

    // World-facing scalar samples update estimates only when machines are locally
    // observable. Hidden explanatory causes such as `WaterPump::is_sabotaged` are not
    // projected into these channels.
    let power_samples: Vec<LocalScalarSample> = power_junctions
        .iter()
        .map(|(tf, junction)| LocalScalarSample {
            position: tf.translation.truncate(),
            value: f64::from(junction.output.clamp(0.0, 1.0)),
        })
        .collect();
    let water_samples: Vec<LocalScalarSample> = water_pumps
        .iter()
        .map(|(tf, pump)| LocalScalarSample {
            position: tf.translation.truncate(),
            value: if pump.is_running {
                f64::from(pump.efficiency.clamp(0.0, 1.0))
            } else {
                0.0
            },
        })
        .collect();

    // Actionable infrastructure presentation is separate from scalar world estimates.
    // Local attention receives visible/operational degradation, never hidden failure flags.
    let junction_targets: Vec<LocalTargetSample> = power_junctions
        .iter()
        .filter_map(|(tf, junction)| {
            let degradation = presented_output_degradation(junction.output);
            (degradation > 0.0).then_some(LocalTargetSample {
                position: tf.translation.truncate(),
                salience: degradation,
            })
        })
        .collect();
    let water_targets: Vec<LocalTargetSample> = water_pumps
        .iter()
        .filter_map(|(tf, pump)| {
            let degradation = presented_pump_degradation(pump.is_running, pump.efficiency);
            (degradation > 0.0).then_some(LocalTargetSample {
                position: tf.translation.truncate(),
                salience: degradation,
            })
        })
        .collect();
    let drone_targets: Vec<LocalTargetSample> = drones
        .iter()
        .map(|(tf, _)| LocalTargetSample {
            position: tf.translation.truncate(),
            salience: 1.0,
        })
        .collect();

    for (entity, mut npc, npc_tf, mut target, mut noise, body, mut psych) in &mut npcs {
        let npc_pos = npc_tf.translation.truncate();
        let pos = nalgebra::SVector::from([npc_pos.x as f64, npc_pos.y as f64]);

        let (energy_frac, harmony, _prediction_error, npc_phi) = physics
            .consciousness
            .entities
            .get(&body.handle)
            .map(|e| {
                (
                    e.energy.fraction_remaining(),
                    e.harmony_activations,
                    e.prediction_error,
                    e.phi(),
                )
            })
            .unwrap_or((1.0, [0.5; 9], 0.0, 0.5));

        let frame = perceived_world.entry(entity).or_default();
        frame.advance();

        // Danger is local risk-cue strength, not a copy of hidden Leviathan phase.
        // The cue includes the NPC's own emitted noise and nearby observable emitters.
        frame.observe_danger(
            local_noise_risk_cue(npc_pos, &noise_samples, LOCAL_PERCEPTION_RANGE),
            1.0,
            DANGER_MEMORY_GENERATIONS,
        );
        if let Some((value, confidence)) =
            strongest_local_sample(npc_pos, &water_samples, LOCAL_PERCEPTION_RANGE)
        {
            frame.observe_water(value, confidence, INFRASTRUCTURE_MEMORY_GENERATIONS);
        }
        if let Some((value, confidence)) =
            strongest_local_sample(npc_pos, &power_samples, LOCAL_PERCEPTION_RANGE)
        {
            frame.observe_power(value, confidence, INFRASTRUCTURE_MEMORY_GENERATIONS);
        }

        let perceived_danger = frame.danger_signal();
        let perceived_water = frame.water_estimate();
        let perceived_power = frame.power_estimate();

        // There is no physical Leviathan entity/location in this slice. The previous
        // implementation incorrectly used the Fusion Core's position as a threat
        // source. Preserve uncertainty rather than inventing a location.
        let danger_source: Option<nalgebra::SVector<f64, 2>> = None;

        let self_allostatic_load = psych
            .as_ref()
            .map_or(0.0, |p| p.allostatic_load)
            .clamp(0.0, 1.0) as f64;
        let values = fep_observation_values(
            energy_frac,
            self_allostatic_load,
            perceived_danger,
            npc.caution as f64,
            perceived_water,
            perceived_power,
        );
        let obs = Observation::new(values.to_vec(), 0.8, "game");
        let _perception = npc.fep.perceive(&obs);

        // This collection remains an input to the consciousness-physics field equation,
        // not a list of agents that the NPC is asserted to know about. FEP-05 therefore
        // leaves provider-owned field semantics unchanged while constraining authored goals.
        let nearby: Vec<_> = all_agents
            .iter()
            .filter(|(agent_pos, _)| {
                let d = (agent_pos - pos).norm();
                d > 2.0
            })
            .cloned()
            .collect();

        let mut direction = symtropy_consciousness_physics::fep_gradient::free_energy_gradient_phi(
            &pos,
            energy_frac,
            Some(npc_phi),
            &harmony,
            &nearby,
            &well_data,
            danger_source.as_ref(),
            perceived_danger,
        );

        // EMERGENT CRISIS MODIFIER: low perceived water may pull the agent only toward
        // a locally observed degraded water asset. A global well or arbitrary room index
        // is no longer treated as a known water-pump location.
        if npc_phi > 0.6
            && perceived_water < 0.3
            && let Some(observed) =
                strongest_local_target(npc_pos, &water_targets, LOCAL_PERCEPTION_RANGE)
        {
            direction = blend_toward_local_target(direction, npc_pos, observed.position, 0.5, 0.5);
        }

        // ARCHETYPE-SPECIFIC ACTIVE INFERENCE ACTIONS & GOALS

        // 1. Kael (Engineer) & Leo (Young Tech): respond only to locally presented
        // junction degradation rather than scanning hidden failure flags in the world.
        if npc.name.contains("Kael") || npc.name.contains("Leo") {
            if let Some(observed) =
                strongest_local_target(npc_pos, &junction_targets, LOCAL_PERCEPTION_RANGE)
            {
                direction =
                    blend_toward_local_target(direction, npc_pos, observed.position, 0.4, 0.6);
            }

            // Nearby Kael presence is directly observable at this distance; no hidden
            // psychological state of Kael is read here.
            if npc.name.contains("Leo") {
                let kael_pos = npc_infos
                    .iter()
                    .find(|info| info.name.contains("Kael"))
                    .map(|info| info.pos);
                if let Some(kael_p) = kael_pos
                    && npc_pos.distance(kael_p) < 100.0
                    && let Some(ref mut p) = psych
                {
                    p.allostatic_load = (p.allostatic_load - 0.05 * time.delta_secs()).max(0.0);
                }
            }
        }

        // Soren's tutorial step is treated as an explicit authored assignment, but it
        // no longer grants world-wide pump coordinates: movement still requires a local
        // degraded-pump presentation.
        if npc.name.contains("Soren")
            && let Some(ref tutorial) = tutorial_res
            && tutorial.step == TutorialStep::CoopRepairing
            && let Some(observed) =
                strongest_local_target(npc_pos, &water_targets, LOCAL_PERCEPTION_RANGE)
        {
            direction = blend_toward_local_target(direction, npc_pos, observed.position, 0.2, 0.8);
        }

        // 2. Mira (Medic): exact allostatic load remains private to each resident.
        // Target selection uses only a coarse outward distress presentation and locality.
        if npc.name.contains("Mira") {
            let distress_targets: Vec<LocalTargetSample> = npc_infos
                .iter()
                .filter(|info| info.entity != entity && info.distress_cue > 0.0)
                .map(|info| LocalTargetSample {
                    position: info.pos,
                    salience: info.distress_cue,
                })
                .collect();
            if let Some(observed) =
                strongest_local_target(npc_pos, &distress_targets, LOCAL_PERCEPTION_RANGE)
            {
                direction =
                    blend_toward_local_target(direction, npc_pos, observed.position, 0.3, 0.7);
            }
        }

        // 3. Jack (Convoy Lead): drone presence is locally observable. Deterministic
        // attention selection replaces query-order-dependent nearest-target scans.
        if npc.name.contains("Jack")
            && let Some(observed) =
                strongest_local_target(npc_pos, &drone_targets, LOCAL_DRONE_ATTENTION_RANGE)
        {
            direction = blend_toward_local_target(direction, npc_pos, observed.position, 0.2, 0.8);
        }

        // 4. PR-4 (Robot): target visible operational degradation only. The hidden
        // `is_sabotaged` cause remains available only after entering local diagnostic range.
        if npc.name.contains("PR-4")
            && let Some(observed) =
                strongest_local_target(npc_pos, &water_targets, LOCAL_PERCEPTION_RANGE)
        {
            direction = blend_toward_local_target(direction, npc_pos, observed.position, 0.4, 0.6);
        }

        let dir_vec = Vec2::new(direction[0] as f32, direction[1] as f32);
        let load = psych.as_ref().map(|p| p.allostatic_load).unwrap_or(0.0);
        let engagement = psych.as_ref().map(|p| p.engagement).unwrap_or(1.0);

        let effective_engagement = if perceived_danger > 0.5 {
            engagement.max(0.5)
        } else {
            engagement
        };

        if dir_vec.length_squared() > 0.01 && effective_engagement > 0.15 {
            let speed = if energy_frac < 0.2 {
                90.0
            } else if perceived_danger > 0.5 {
                100.0
            } else {
                50.0
            };

            let psych_factor = (1.0 - load * 0.4) * effective_engagement;
            target.target = Some(npc_pos + dir_vec * 100.0);
            target.speed = speed * (1.0 - npc.caution * 0.3) * psych_factor;
            noise.level = if speed > 80.0 { 0.1 } else { 0.03 };
        } else {
            target.target = None;
            target.speed = 0.0;
            noise.level = 0.0;
        }

        let load_caution_boost = if load > 0.6 { 0.02 } else { 0.0 };
        if perceived_danger > 0.5 {
            npc.caution = (npc.caution + 0.05 + load_caution_boost).min(1.0);
        } else {
            npc.caution = (npc.caution - 0.02).max(0.0);
        }
    }
}

/// System applying repairs, healing, and drone neutralization when adjacent to targets.
pub fn npc_action_system(
    actors: Query<(Entity, &CrewNpc, &Transform)>,
    mut needs_query: Query<&mut crate::systems::psychology::PsychologicalNeeds>,
    mut power_junctions: Query<(&Transform, &mut PowerJunction)>,
    mut water_pumps: Query<(&Transform, &mut WaterPump)>,
    mut drones: Query<(Entity, &Transform, &mut NullDrone)>,
    mut commands: Commands,
    time: Res<Time>,
    mut action_writer: MessageWriter<NpcActionEvent>,
    mut feedback_writer: MessageWriter<WorldFeedbackEvent>,
    tutorial_res: Option<Res<TutorialScenarioRes>>,
) {
    let dt = time.delta_secs();

    for (actor_entity, npc, tf) in &actors {
        let npc_pos = tf.translation.truncate();

        // 1. Kael (Engineer) repairs PowerJunction
        if npc.name.contains("Kael") {
            for (j_tf, mut junction) in &mut power_junctions {
                if junction.is_damaged {
                    let j_pos = j_tf.translation.truncate();
                    if npc_pos.distance(j_pos) < 30.0 {
                        let is_pr4_adjacent = actors.iter().any(|(_, other_npc, other_tf)| {
                            other_npc.name.contains("PR-4")
                                && other_tf.translation.truncate().distance(j_pos) < 30.0
                        });

                        if is_pr4_adjacent {
                            junction.is_damaged = false;
                            junction.output = 1.0;
                            action_writer.write(NpcActionEvent {
                                actor: actor_entity,
                                actor_name: npc.name.clone(),
                                target: None,
                                target_name: "Power Junction".to_string(),
                                action_kind: NpcActionKind::RepairJunction,
                                intensity: 1.0,
                                success_delta: 1.0,
                                settlement_metric_delta: 0.2,
                            });
                            feedback_writer.write(WorldFeedbackEvent {
                                position: j_pos,
                                message: "JUNCTION STABILIZED (100%)".to_string(),
                                color: Color::srgb(0.2, 0.9, 0.4),
                            });
                        } else if junction.output < 0.8 {
                            let old_out = junction.output;
                            junction.output = (junction.output + 0.3 * dt).min(0.8);
                            if old_out < 0.8 && junction.output >= 0.8 {
                                action_writer.write(NpcActionEvent {
                                    actor: actor_entity,
                                    actor_name: npc.name.clone(),
                                    target: None,
                                    target_name: "Power Junction".to_string(),
                                    action_kind: NpcActionKind::RepairJunction,
                                    intensity: 0.5,
                                    success_delta: 0.8,
                                    settlement_metric_delta: 0.1,
                                });
                                feedback_writer.write(WorldFeedbackEvent {
                                    position: j_pos,
                                    message: "JUNCTION RESTORED TO 80% (NEEDS PR-4)".to_string(),
                                    color: Color::srgb(0.9, 0.6, 0.2),
                                });
                            }
                        }
                    }
                }
            }
        }

        // 2. PR-4 (Robot) repairs WaterPump. Hidden sabotage state is diagnostic
        // information and may be consulted only after the actor is physically local.
        if npc.name.contains("PR-4") {
            for (p_tf, mut pump) in &mut water_pumps {
                let p_pos = p_tf.translation.truncate();
                if npc_pos.distance(p_pos) >= LOCAL_DIAGNOSTIC_RANGE {
                    continue;
                }

                let is_under_coop_tutorial = if let Some(ref tutorial) = tutorial_res {
                    tutorial.step == TutorialStep::CoopRepairing && pump.efficiency < 1.0
                } else {
                    false
                };

                // This is the explicit close-diagnostic boundary. `is_sabotaged` may affect
                // repair execution here, but never remote attention or navigation.
                if pump.is_sabotaged || is_under_coop_tutorial {
                    let is_assistant_adjacent = actors.iter().any(|(_, other_npc, other_tf)| {
                        (other_npc.name.contains("Nadia") || other_npc.name.contains("Soren"))
                            && other_tf.translation.truncate().distance(p_pos)
                                < LOCAL_DIAGNOSTIC_RANGE
                    });

                    if is_assistant_adjacent {
                        pump.is_sabotaged = false;
                        pump.efficiency = 1.0;
                        pump.is_running = true;
                        action_writer.write(NpcActionEvent {
                            actor: actor_entity,
                            actor_name: npc.name.clone(),
                            target: None,
                            target_name: "Water Pump".to_string(),
                            action_kind: NpcActionKind::RepairPump,
                            intensity: 1.0,
                            success_delta: 1.0,
                            settlement_metric_delta: 0.2,
                        });
                        feedback_writer.write(WorldFeedbackEvent {
                            position: p_pos,
                            message: "WATER PUMP PURIFIED (100%)".to_string(),
                            color: Color::srgb(0.2, 0.9, 0.4),
                        });
                    } else if pump.efficiency < 0.7 {
                        let old_eff = pump.efficiency;
                        pump.efficiency = (pump.efficiency + 0.3 * dt).min(0.7);
                        pump.is_running = true;
                        if old_eff < 0.7 && pump.efficiency >= 0.7 {
                            pump.is_sabotaged = false; // Online but partial
                            action_writer.write(NpcActionEvent {
                                actor: actor_entity,
                                actor_name: npc.name.clone(),
                                target: None,
                                target_name: "Water Pump".to_string(),
                                action_kind: NpcActionKind::RepairPump,
                                intensity: 0.5,
                                success_delta: 0.7,
                                settlement_metric_delta: 0.1,
                            });
                            feedback_writer.write(WorldFeedbackEvent {
                                position: p_pos,
                                message: "WATER PUMP ONLINE (CONTAMINATED)".to_string(),
                                color: Color::srgb(0.9, 0.6, 0.2),
                            });
                        }
                    }
                }
            }
        }

        // 3. Mira (Medic) heals stressed crew member
        if npc.name.contains("Mira") {
            for (other_entity, other_npc, other_tf) in &actors {
                if other_entity != actor_entity {
                    let other_pos = other_tf.translation.truncate();
                    if npc_pos.distance(other_pos) < 30.0
                        && let Ok(mut other_psych) = needs_query.get_mut(other_entity)
                        && other_psych.allostatic_load > 0.4
                    {
                        let kael_pos = actors
                            .iter()
                            .find(|(_, o_npc, _)| o_npc.name.contains("Kael"))
                            .map(|(_, _, o_tf)| o_tf.translation.truncate());
                        let kael_far = kael_pos.is_none_or(|kp| other_pos.distance(kp) > 120.0);

                        if kael_far && other_npc.name.contains("Leo") {
                            // Relapse state.
                            let old_load = other_psych.allostatic_load;
                            other_psych.allostatic_load =
                                (other_psych.allostatic_load + 0.05 * dt).min(1.0);

                            // Emit once on the meaningful state transition instead of using
                            // ambient RNG, which made event history frame-rate/randomness dependent.
                            if crossed_upward_threshold(
                                old_load,
                                other_psych.allostatic_load,
                                LEO_RELAPSE_ALERT_THRESHOLD,
                            ) {
                                action_writer.write(NpcActionEvent {
                                    actor: actor_entity,
                                    actor_name: npc.name.clone(),
                                    target: Some(other_entity),
                                    target_name: other_npc.name.clone(),
                                    action_kind: NpcActionKind::HealStress,
                                    intensity: 0.0,
                                    success_delta: -0.1,
                                    settlement_metric_delta: -0.05,
                                });
                                feedback_writer.write(WorldFeedbackEvent {
                                    position: other_pos,
                                    message: "LEO RELAPSING (KAEL FAR)".to_string(),
                                    color: Color::srgb(0.9, 0.2, 0.2),
                                });
                            }
                        } else {
                            // Successful healing
                            let old_load = other_psych.allostatic_load;
                            other_psych.allostatic_load =
                                (other_psych.allostatic_load - 0.15 * dt).max(0.0);
                            other_psych.social_satiation =
                                (other_psych.social_satiation + 0.1 * dt).min(1.0);

                            // Emit healer success event
                            if old_load > 0.4 && other_psych.allostatic_load <= 0.4 {
                                action_writer.write(NpcActionEvent {
                                    actor: actor_entity,
                                    actor_name: npc.name.clone(),
                                    target: Some(other_entity),
                                    target_name: other_npc.name.clone(),
                                    action_kind: NpcActionKind::HealStress,
                                    intensity: 1.0,
                                    success_delta: 0.15,
                                    settlement_metric_delta: 0.1,
                                });
                                feedback_writer.write(WorldFeedbackEvent {
                                    position: other_pos,
                                    message: "CREW STRESS STABILIZED (-15%)".to_string(),
                                    color: Color::srgb(0.2, 0.8, 0.9),
                                });
                            }
                        }
                    }
                }
            }
        }

        // 4. Jack (Convoy Lead) deals damage to / destroys NullDrone
        if npc.name.contains("Jack") {
            for (drone_entity, d_tf, mut drone) in &mut drones {
                let d_pos = d_tf.translation.truncate();
                if npc_pos.distance(d_pos) < 30.0 {
                    drone.integrity -= 0.5 * dt;

                    // Gunfire stress: increase stress of nearby crew members
                    for (other_ent, _other_npc, other_tf) in &actors {
                        if other_ent != actor_entity {
                            let dist = other_tf.translation.truncate().distance(npc_pos);
                            if dist < 150.0
                                && let Ok(mut other_psych) = needs_query.get_mut(other_ent)
                            {
                                other_psych.allostatic_load =
                                    (other_psych.allostatic_load + 0.02 * dt).min(1.0);
                            }
                        }
                    }

                    if drone.integrity <= 0.0 {
                        commands.entity(drone_entity).despawn();

                        action_writer.write(NpcActionEvent {
                            actor: actor_entity,
                            actor_name: npc.name.clone(),
                            target: None,
                            target_name: "NullDrone".to_string(),
                            action_kind: NpcActionKind::CombatDrone,
                            intensity: 1.0,
                            success_delta: 1.0,
                            settlement_metric_delta: 0.15,
                        });
                        feedback_writer.write(WorldFeedbackEvent {
                            position: d_pos,
                            message: "NULL DRONE DISABLED".to_string(),
                            color: Color::srgb(0.9, 0.2, 0.2),
                        });
                    }
                }
            }
        }
    }
}

/// Apply NPC movement intent to physics bodies.
pub fn npc_movement_system(
    query: Query<
        (
            &Transform,
            &MoveTarget,
            &symtropy_render_bridge::PhysicsBody,
        ),
        With<CrewNpc>,
    >,
    mut physics: ResMut<PhysicsWorldRes>,
    tile_grid: Option<Res<crate::resources::TileGrid>>,
) {
    for (tf, target, body_comp) in &query {
        if let Some(body) = physics.world.body_mut(body_comp.handle) {
            if let Some(dest) = target.target {
                let pos = tf.translation.truncate();
                let dir = dest - pos;
                let dist = dir.length();
                if dist > 2.0 && target.speed > 0.0 {
                    let norm = dir.normalize();
                    let mut vx = norm.x as f64 * target.speed as f64;
                    let mut vy = norm.y as f64 * target.speed as f64;

                    if let Some(ref grid) = tile_grid {
                        let dt = 1.0 / 64.0_f32;
                        let new_x = tf.translation.x + vx as f32 * dt;
                        let new_y = tf.translation.y + vy as f32 * dt;
                        if !grid.is_walkable(new_x, tf.translation.y) {
                            vx = 0.0;
                        }
                        if !grid.is_walkable(tf.translation.x, new_y) {
                            vy = 0.0;
                        }
                    }

                    body.linear_velocity = nalgebra::SVector::from([vx, vy]);
                } else {
                    body.linear_velocity = nalgebra::SVector::from([0.0, 0.0]);
                }
            } else {
                body.linear_velocity = nalgebra::SVector::from([0.0, 0.0]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fep_observation_frame_remains_six_dimensional_and_bounded() {
        let values = fep_observation_values(1.2, -0.1, 0.5, 0.4, 0.3, 2.0);
        assert_eq!(values.len(), FEP_OBSERVATION_DIM);
        assert_eq!(values, [1.0, 0.0, 0.5, 0.4, 0.3, 1.0]);
    }

    #[test]
    fn second_fep_slot_is_explicit_self_state() {
        let values = fep_observation_values(0.8, 0.65, 0.2, 0.4, 0.5, 0.6);
        assert_eq!(values[1], 0.65);
    }

    #[test]
    fn external_fep_slots_accept_perceived_values_without_global_state_types() {
        let values = fep_observation_values(0.8, 0.2, 0.7, 0.4, 0.25, 0.9);
        assert_eq!(values[2], 0.7);
        assert_eq!(values[4], 0.25);
        assert_eq!(values[5], 0.9);
    }

    #[test]
    fn local_target_blend_refuses_zero_distance_false_precision() {
        let direction = nalgebra::SVector::from([0.25, -0.25]);
        assert_eq!(
            blend_toward_local_target(direction, Vec2::ZERO, Vec2::ZERO, 0.4, 0.6),
            direction
        );
    }

    #[test]
    fn presentation_helpers_depend_only_on_observable_machine_output() {
        assert_eq!(presented_output_degradation(1.0), 0.0);
        assert_eq!(presented_output_degradation(0.25), 0.75);
        assert_eq!(presented_pump_degradation(true, 1.0), 0.0);
        assert_eq!(presented_pump_degradation(false, 1.0), 1.0);
        assert_eq!(presented_pump_degradation(true, f32::NAN), 0.0);
    }

    #[test]
    fn upward_threshold_crossing_is_single_transition_and_fails_closed() {
        assert!(crossed_upward_threshold(0.79, 0.8, 0.8));
        assert!(!crossed_upward_threshold(0.8, 0.81, 0.8));
        assert!(!crossed_upward_threshold(0.79, 0.79, 0.8));
        assert!(!crossed_upward_threshold(f32::NAN, 0.9, 0.8));
        assert!(!crossed_upward_threshold(0.7, f32::NAN, 0.8));
        assert!(!crossed_upward_threshold(0.7, 0.9, f32::NAN));
    }
}