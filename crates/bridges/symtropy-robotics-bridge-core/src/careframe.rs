// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Read-only Careframe collision-proxy assembly.
//!
//! This module imports the synthetic morphology into Symtropy's actual
//! PhysicsWorld<3> as static, oriented box colliders. It deliberately does
//! not add joint constraints, motors, inertias, or actuator commands. The
//! joint state determines a single frozen geometry arrangement only.

use std::collections::{BTreeMap, BTreeSet};

use nalgebra::{SMatrix, SVector};
use serde::Deserialize;
use symtropy_math::{HyperBox, Point, Rotor, Transform};
use symtropy_physics::{BodyHandle, BodyType, PhysicsWorld, RigidBody};

const MORPHOLOGY_SCHEMA: &str = "careframe-morphology-v0";
const GEOMETRY_FIDELITY: &str = "bounding_primitives_not_manufacturing_geometry";
const SYNTHETIC_EVIDENCE_CLASS: &str = "synthetic_morphology_proposal";
const SYNTHETIC_SOURCE: &str = "synthetic_placeholder";
const NO_INERTIA: &str = "not_provided_kinematic_only";

/// Static geometry proxies placed at one validated joint state.
///
/// These proxies are collidable objects in Symtropy's world but are not an
/// articulated mechanism. The assembly does not install constraints, motors,
/// controller state, or commands.
#[derive(Clone, Debug)]
pub struct CareframeStaticCollisionAssembly {
    pub model_id: String,
    pub revision: u64,
    pub link_handles: BTreeMap<String, BodyHandle>,
}

impl CareframeStaticCollisionAssembly {
    pub fn len(&self) -> usize {
        self.link_handles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.link_handles.is_empty()
    }

    pub fn handle_for_link(&self, link_id: &str) -> Option<BodyHandle> {
        self.link_handles.get(link_id).copied()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CareframeMorphology {
    schema_version: String,
    model_id: String,
    revision: u64,
    morphology_kind: String,
    evidence_class: String,
    geometry_fidelity: String,
    units: Units,
    root_link: String,
    links: Vec<MorphologyLink>,
    joints: Vec<MorphologyJoint>,
    controller: Controller,
    authority: Authority,
    caveats: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Units {
    length: String,
    mass: String,
    joint_angle: String,
    joint_effort: String,
    joint_velocity: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MorphologyLink {
    link_id: String,
    mass_kg: f64,
    collision: CollisionGeometry,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CollisionGeometry {
    shape: String,
    dimensions_m: [f64; 3],
    source: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MorphologyJoint {
    joint_id: String,
    parent_link: String,
    child_link: String,
    joint_type: String,
    origin_m: [f64; 3],
    axis_xyz: [f64; 3],
    lower_limit_rad: f64,
    upper_limit_rad: f64,
    max_effort_nm: f64,
    max_velocity_rad_s: f64,
    command_enabled: bool,
    source: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Controller {
    kinematics_only: bool,
    commands_enabled: bool,
    inertia_source: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Authority {
    simulation_only: bool,
    physical_actuation_authorized: bool,
    patient_contact_authorized: bool,
    clinical_use_authorized: bool,
}

/// Adds a synthetic morphology as static collision proxies at a bounded joint
/// state. Validation and forward kinematics complete before the world changes,
/// so malformed models or states leave the physics world untouched.
///
/// The result supplies oriented boxes for collision experiments. It does not
/// create a dynamic robot, articulated constraints, inertial properties, or
/// actuator interfaces.
pub fn spawn_static_collision_assembly(
    world: &mut PhysicsWorld<3>,
    morphology_json: &str,
    joint_state_json: &str,
    base_position: Point<3>,
) -> Result<CareframeStaticCollisionAssembly, Vec<String>> {
    let morphology: CareframeMorphology = serde_json::from_str(morphology_json)
        .map_err(|error| vec![format!("MORPHOLOGY_JSON_INVALID: {error}")])?;
    validate_morphology(&morphology)?;

    let joint_state: BTreeMap<String, f64> = serde_json::from_str(joint_state_json)
        .map_err(|error| vec![format!("JOINT_STATE_JSON_INVALID: {error}")])?;
    validate_joint_state(&morphology, &joint_state)?;
    let poses = compute_link_poses(&morphology, &joint_state, base_position)?;

    // Mutation occurs only after all schema, graph and joint-state validation.
    let mut link_handles = BTreeMap::new();
    for link in &morphology.links {
        let pose = poses
            .get(&link.link_id)
            .expect("validated graph computes a pose for every link")
            .clone();
        let half_extents = link
            .collision
            .dimensions_m
            .map(|dimension| dimension * 0.5);
        let collider = Box::new(HyperBox::<3>::new(half_extents));
        let body = RigidBody::new(
            BodyHandle(0),
            BodyType::Static,
            pose,
            collider,
            0.0,
            SVector::<f64, 3>::zeros(),
        );
        let handle = world.add_body(body);
        link_handles.insert(link.link_id.clone(), handle);
    }

    Ok(CareframeStaticCollisionAssembly {
        model_id: morphology.model_id,
        revision: morphology.revision,
        link_handles,
    })
}

fn valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn finite_vec3(values: &[f64; 3]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn validate_morphology(model: &CareframeMorphology) -> Result<(), Vec<String>> {
    let mut findings = BTreeSet::new();
    if model.schema_version != MORPHOLOGY_SCHEMA {
        findings.insert("MORPHOLOGY_SCHEMA_UNSUPPORTED".to_owned());
    }
    if model.model_id.trim().is_empty() || model.revision == 0 {
        findings.insert("MORPHOLOGY_ID_OR_REVISION_INVALID".to_owned());
    }
    if model.morphology_kind != "wheeled_upper_body_humanoid"
        || model.evidence_class != SYNTHETIC_EVIDENCE_CLASS
        || model.geometry_fidelity != GEOMETRY_FIDELITY
    {
        findings.insert("MORPHOLOGY_KIND_OR_EVIDENCE_UNSUPPORTED".to_owned());
    }
    if model.units.length != "m"
        || model.units.mass != "kg"
        || model.units.joint_angle != "rad"
        || model.units.joint_effort != "N_m"
        || model.units.joint_velocity != "rad_s"
    {
        findings.insert("MORPHOLOGY_UNITS_INVALID".to_owned());
    }
    if !model.controller.kinematics_only
        || model.controller.commands_enabled
        || model.controller.inertia_source != NO_INERTIA
    {
        findings.insert("MORPHOLOGY_CONTROLLER_NOT_STATIC_KINEMATIC_ONLY".to_owned());
    }
    if !model.authority.simulation_only
        || model.authority.physical_actuation_authorized
        || model.authority.patient_contact_authorized
        || model.authority.clinical_use_authorized
    {
        findings.insert("MORPHOLOGY_AUTHORITY_FORBIDDEN".to_owned());
    }
    if model.caveats.is_empty() || model.caveats.iter().any(|caveat| caveat.trim().is_empty()) {
        findings.insert("MORPHOLOGY_CAVEATS_REQUIRED".to_owned());
    }
    if !(2..=64).contains(&model.links.len()) {
        findings.insert("MORPHOLOGY_LINK_COUNT_OUT_OF_RANGE".to_owned());
    }
    if model.joints.is_empty()
        || model.joints.len() > 128
        || model.joints.len().saturating_add(1) != model.links.len()
    {
        findings.insert("MORPHOLOGY_EDGE_COUNT_INVALID".to_owned());
    }

    let mut link_ids = BTreeSet::new();
    for link in &model.links {
        if !valid_identifier(&link.link_id) || !link_ids.insert(link.link_id.clone()) {
            findings.insert("MORPHOLOGY_LINK_ID_INVALID".to_owned());
        }
        if !link.mass_kg.is_finite() || !(0.001..=250.0).contains(&link.mass_kg) {
            findings.insert("MORPHOLOGY_LINK_MASS_INVALID".to_owned());
        }
        if link.collision.shape != "box" || link.collision.source != SYNTHETIC_SOURCE {
            findings.insert("MORPHOLOGY_COLLISION_SHAPE_OR_SOURCE_UNSUPPORTED".to_owned());
        }
        if !finite_vec3(&link.collision.dimensions_m)
            || link
                .collision
                .dimensions_m
                .iter()
                .any(|dimension| *dimension <= 0.0 || *dimension > 2.0)
        {
            findings.insert("MORPHOLOGY_COLLISION_DIMENSIONS_INVALID".to_owned());
        }
    }
    if !link_ids.contains(&model.root_link) {
        findings.insert("MORPHOLOGY_ROOT_LINK_UNKNOWN".to_owned());
    }

    let mut joint_ids = BTreeSet::new();
    let mut incoming = BTreeMap::<String, usize>::new();
    let mut adjacency = BTreeMap::<String, Vec<String>>::new();
    for joint in &model.joints {
        if !valid_identifier(&joint.joint_id) || !joint_ids.insert(joint.joint_id.clone()) {
            findings.insert("MORPHOLOGY_JOINT_ID_INVALID".to_owned());
        }
        if !link_ids.contains(&joint.parent_link) || !link_ids.contains(&joint.child_link) {
            findings.insert("MORPHOLOGY_JOINT_LINK_UNKNOWN".to_owned());
        }
        if joint.parent_link == joint.child_link {
            findings.insert("MORPHOLOGY_SELF_EDGE".to_owned());
        }
        if link_ids.contains(&joint.child_link) {
            *incoming.entry(joint.child_link.clone()).or_default() += 1;
        }
        if link_ids.contains(&joint.parent_link)
            && link_ids.contains(&joint.child_link)
            && joint.parent_link != joint.child_link
        {
            adjacency
                .entry(joint.parent_link.clone())
                .or_default()
                .push(joint.child_link.clone());
        }
        if !finite_vec3(&joint.origin_m)
            || joint.origin_m.iter().any(|coordinate| coordinate.abs() > 10.0)
        {
            findings.insert("MORPHOLOGY_JOINT_ORIGIN_INVALID".to_owned());
        }
        if joint.command_enabled {
            findings.insert("MORPHOLOGY_JOINT_COMMAND_FORBIDDEN".to_owned());
        }
        if joint.source != SYNTHETIC_SOURCE {
            findings.insert("MORPHOLOGY_JOINT_SOURCE_UNSUPPORTED".to_owned());
        }

        match joint.joint_type.as_str() {
            "fixed" => {
                if joint.axis_xyz.iter().any(|axis| axis.abs() > 1.0e-8)
                    || joint.lower_limit_rad != 0.0
                    || joint.upper_limit_rad != 0.0
                    || joint.max_effort_nm != 0.0
                    || joint.max_velocity_rad_s != 0.0
                {
                    findings.insert("MORPHOLOGY_FIXED_JOINT_ENVELOPE_INVALID".to_owned());
                }
            }
            "revolute" => {
                let norm = joint.axis_xyz.iter().map(|axis| axis * axis).sum::<f64>().sqrt();
                if !finite_vec3(&joint.axis_xyz) || (norm - 1.0).abs() > 1.0e-3 {
                    findings.insert("MORPHOLOGY_REVOLUTE_AXIS_INVALID".to_owned());
                }
                if !joint.lower_limit_rad.is_finite()
                    || !joint.upper_limit_rad.is_finite()
                    || !(-50.0..=50.0).contains(&joint.lower_limit_rad)
                    || !(-50.0..=50.0).contains(&joint.upper_limit_rad)
                    || joint.lower_limit_rad >= joint.upper_limit_rad
                {
                    findings.insert("MORPHOLOGY_REVOLUTE_LIMITS_INVALID".to_owned());
                }
                if !joint.max_effort_nm.is_finite()
                    || !(0.001..=1000.0).contains(&joint.max_effort_nm)
                    || !joint.max_velocity_rad_s.is_finite()
                    || !(0.001..=20.0).contains(&joint.max_velocity_rad_s)
                {
                    findings.insert("MORPHOLOGY_JOINT_ENVELOPE_INVALID".to_owned());
                }
            }
            _ => {
                findings.insert("MORPHOLOGY_JOINT_TYPE_UNSUPPORTED".to_owned());
            }
        }
    }

    if incoming.get(&model.root_link).copied().unwrap_or(0) != 0 {
        findings.insert("MORPHOLOGY_ROOT_HAS_PARENT".to_owned());
    }
    for link_id in &link_ids {
        let expected = if link_id == &model.root_link { 0 } else { 1 };
        if incoming.get(link_id).copied().unwrap_or(0) != expected {
            findings.insert("MORPHOLOGY_PARENT_COUNT_INVALID".to_owned());
        }
    }

    let mut reached = BTreeSet::new();
    let mut pending = vec![model.root_link.clone()];
    while let Some(link_id) = pending.pop() {
        if !reached.insert(link_id.clone()) {
            continue;
        }
        if let Some(children) = adjacency.get(&link_id) {
            pending.extend(children.iter().cloned());
        }
    }
    if reached.len() != link_ids.len() {
        findings.insert("MORPHOLOGY_GRAPH_DISCONNECTED_OR_CYCLIC".to_owned());
    }

    if findings.is_empty() {
        Ok(())
    } else {
        Err(findings.into_iter().collect())
    }
}

fn validate_joint_state(
    model: &CareframeMorphology,
    state: &BTreeMap<String, f64>,
) -> Result<(), Vec<String>> {
    let mut findings = BTreeSet::new();
    for (joint_id, position) in state {
        let Some(joint) = model
            .joints
            .iter()
            .find(|joint| joint.joint_id.as_str() == joint_id.as_str())
        else {
            findings.insert("JOINT_STATE_UNKNOWN_ID".to_owned());
            continue;
        };
        if joint.joint_type != "revolute" {
            findings.insert("JOINT_STATE_FOR_FIXED_JOINT".to_owned());
            continue;
        }
        if !position.is_finite()
            || *position < joint.lower_limit_rad
            || *position > joint.upper_limit_rad
        {
            findings.insert("JOINT_STATE_OUT_OF_LIMITS".to_owned());
        }
    }
    for joint in &model.joints {
        if joint.joint_type == "revolute" && !state.contains_key(&joint.joint_id) {
            findings.insert("JOINT_STATE_MISSING_REVOLUTE_JOINT".to_owned());
        }
    }
    if findings.is_empty() {
        Ok(())
    } else {
        Err(findings.into_iter().collect())
    }
}

fn axis_angle_rotor(axis: [f64; 3], angle: f64) -> Rotor<3> {
    if angle.abs() < 1.0e-15 {
        return Rotor::identity();
    }
    let norm = axis.iter().map(|component| component * component).sum::<f64>().sqrt();
    let [x, y, z] = axis.map(|component| component / norm);
    let cosine = angle.cos();
    let sine = angle.sin();
    let one_minus_cosine = 1.0 - cosine;
    let matrix = SMatrix::<f64, 3, 3>::from_row_slice(&[
        cosine + x * x * one_minus_cosine,
        x * y * one_minus_cosine - z * sine,
        x * z * one_minus_cosine + y * sine,
        y * x * one_minus_cosine + z * sine,
        cosine + y * y * one_minus_cosine,
        y * z * one_minus_cosine - x * sine,
        z * x * one_minus_cosine - y * sine,
        z * y * one_minus_cosine + x * sine,
        cosine + z * z * one_minus_cosine,
    ]);
    Rotor::from_matrix(matrix)
}

fn compute_link_poses(
    model: &CareframeMorphology,
    state: &BTreeMap<String, f64>,
    base_position: Point<3>,
) -> Result<BTreeMap<String, Transform<3>>, Vec<String>> {
    let mut poses = BTreeMap::new();
    poses.insert(
        model.root_link.clone(),
        Transform::from_translation(base_position),
    );
    let mut processed = BTreeSet::new();
    while processed.len() < model.joints.len() {
        let mut progress = false;
        for (index, joint) in model.joints.iter().enumerate() {
            if processed.contains(&index) {
                continue;
            }
            let Some(parent_pose) = poses.get(&joint.parent_link).cloned() else {
                continue;
            };
            let angle = if joint.joint_type == "revolute" {
                state[&joint.joint_id]
            } else {
                0.0
            };
            let joint_transform = Transform {
                translation: Point::new(joint.origin_m),
                rotation: if joint.joint_type == "revolute" {
                    axis_angle_rotor(joint.axis_xyz, angle)
                } else {
                    Rotor::identity()
                },
            };
            poses.insert(
                joint.child_link.clone(),
                parent_pose.compose(&joint_transform),
            );
            processed.insert(index);
            progress = true;
        }
        if !progress {
            return Err(vec!["MORPHOLOGY_GRAPH_TRAVERSAL_STALLED".to_owned()]);
        }
    }
    Ok(poses)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MORPHOLOGY_JSON: &str = include_str!(
        "../../../../docs/medical-robotics/careframe-wheeled-upper-body-morphology-v0.json"
    );
    const ZERO_JOINT_STATE_JSON: &str = include_str!(
        "../../../../docs/medical-robotics/careframe-nominal-joint-state-v0.json"
    );

    fn world() -> PhysicsWorld<3> {
        PhysicsWorld::new(SVector::<f64, 3>::zeros())
    }

    #[test]
    fn morphology_import_creates_only_static_collision_proxies() {
        let mut physics = world();
        let assembly = spawn_static_collision_assembly(
            &mut physics,
            MORPHOLOGY_JSON,
            ZERO_JOINT_STATE_JSON,
            Point::origin(),
        )
        .expect("synthetic Careframe fixture imports");

        assert_eq!(assembly.len(), 16);
        assert_eq!(physics.body_count(), 16);
        assert!(physics.constraints.is_empty());
        assert_eq!(assembly.revision, 1);

        let torso_handle = assembly.handle_for_link("torso_shell").expect("torso link");
        let torso = physics.body(torso_handle).expect("torso body");
        assert_eq!(torso.body_type, BodyType::Static);
        assert!((torso.transform.translation.0[2] - 0.32).abs() < 1.0e-12);
        assert!(
            torso
                .collider
                .as_any()
                .downcast_ref::<HyperBox<3>>()
                .is_some(),
            "morphology bounding box becomes a Symtropy HyperBox collider"
        );
    }

    #[test]
    fn static_careframe_proxy_participates_in_physics_collision_detection() {
        let mut physics = world();
        let assembly = spawn_static_collision_assembly(
            &mut physics,
            MORPHOLOGY_JSON,
            ZERO_JOINT_STATE_JSON,
            Point::origin(),
        )
        .expect("synthetic Careframe fixture imports");
        assert!(assembly.handle_for_link("base_chassis").is_some());

        // The probe intentionally overlaps the base-chassis bounding box.
        let probe = physics.add_sphere(Point::new([0.29, 0.0, 0.0]), 0.05, 1.0);
        physics.step(0.002);

        assert!(
            !physics.contacts.is_empty(),
            "a dynamic probe overlapping a static Careframe box should enter the collision pipeline"
        );
        let body = physics.body(probe).expect("probe remains in physics world");
        assert!(
            body.transform.translation.0.iter().all(|value| value.is_finite()),
            "collision response should not produce non-finite probe coordinates"
        );
    }

    #[test]
    fn morphology_import_honours_rotated_joint_state_in_the_physics_world() {
        let mut state: BTreeMap<String, f64> =
            serde_json::from_str(ZERO_JOINT_STATE_JSON).expect("joint state parses");
        let yaw = 1.0_f64;
        state.insert("torso_yaw".to_owned(), yaw);
        let state_json = serde_json::to_string(&state).expect("joint state serializes");
        let mut physics = world();
        let assembly = spawn_static_collision_assembly(
            &mut physics,
            MORPHOLOGY_JSON,
            &state_json,
            Point::origin(),
        )
        .expect("bounded torso rotation imports");

        let left_arm_handle = assembly
            .handle_for_link("left_upper_arm")
            .expect("left arm");
        let left_arm = physics.body(left_arm_handle).expect("left arm body");
        assert!((left_arm.transform.translation.0[0] - (-yaw.sin() * 0.19)).abs() < 1.0e-10);
        assert!((left_arm.transform.translation.0[1] - (yaw.cos() * 0.19)).abs() < 1.0e-10);
        assert!((left_arm.transform.translation.0[2] - 0.56).abs() < 1.0e-12);
        let rotated_x = left_arm
            .transform
            .rotation
            .rotate_vector(&SVector::<f64, 3>::from([1.0, 0.0, 0.0]));
        assert!((rotated_x[0] - yaw.cos()).abs() < 1.0e-10);
        assert!((rotated_x[1] - yaw.sin()).abs() < 1.0e-10);
        assert!(left_arm.transform.rotation.is_proper_rotation(1.0e-8));

        let right_arm_handle = assembly
            .handle_for_link("right_upper_arm")
            .expect("right arm");
        let right_arm = physics.body(right_arm_handle).expect("right arm body");
        assert!((right_arm.transform.translation.0[0] - yaw.sin() * 0.19).abs() < 1.0e-10);
        assert!((right_arm.transform.translation.0[1] + yaw.cos() * 0.19).abs() < 1.0e-10);
        assert!(
            (right_arm.transform.translation.0[2] - left_arm.transform.translation.0[2]).abs()
                < 1.0e-12
        );
        assert!(right_arm.transform.rotation.is_proper_rotation(1.0e-8));
    }

    #[test]
    fn bounded_joint_state_sweep_keeps_every_imported_pose_finite_and_proper() {
        let morphology: CareframeMorphology =
            serde_json::from_str(MORPHOLOGY_JSON).expect("morphology parses");
        validate_morphology(&morphology).expect("frozen morphology is valid");
        let revolute = morphology
            .joints
            .iter()
            .filter(|joint| joint.joint_type == "revolute")
            .collect::<Vec<_>>();
        assert_eq!(revolute.len(), 9);

        // Deterministic boundary-rich coverage: no RNG or hidden seed dependency.
        for sample in 0..32usize {
            let mut state = BTreeMap::new();
            for (joint_index, joint) in revolute.iter().enumerate() {
                let slot = (sample * 17 + joint_index * 29) % 101;
                let fraction = slot as f64 / 100.0;
                let position = joint.lower_limit_rad
                    + fraction * (joint.upper_limit_rad - joint.lower_limit_rad);
                state.insert(joint.joint_id.clone(), position);
            }
            let state_json = serde_json::to_string(&state).expect("state serializes");
            let mut physics = world();
            let assembly = spawn_static_collision_assembly(
                &mut physics,
                MORPHOLOGY_JSON,
                &state_json,
                Point::new([sample as f64 * 0.25, -0.5, 0.25]),
            )
            .expect("all generated joint states remain within declared limits");
            assert_eq!(assembly.len(), 16);
            assert_eq!(physics.body_count(), 16);
            assert!(physics.constraints.is_empty());

            for handle in assembly.link_handles.values() {
                let body = physics.body(*handle).expect("each proxy resolves");
                assert_eq!(body.body_type, BodyType::Static);
                assert!(
                    body.transform.translation.0.iter().all(|value| value.is_finite()),
                    "non-finite link translation in sample {sample}"
                );
                assert!(
                    body.transform.rotation.is_proper_rotation(1.0e-7),
                    "improper rotation in sample {sample}"
                );
                assert!(body.collider.as_any().downcast_ref::<HyperBox<3>>().is_some());
            }
        }
    }

    #[test]
    fn unknown_fields_and_non_box_shapes_fail_closed() {
        let mut physics = world();
        let mut morphology: serde_json::Value =
            serde_json::from_str(MORPHOLOGY_JSON).expect("morphology parses");
        morphology["links"][0]["collision"]["unmodeled_shape_mode"] =
            serde_json::json!("silently-ignore");
        let invalid_json =
            serde_json::to_string(&morphology).expect("mutated morphology serializes");
        let error = spawn_static_collision_assembly(
            &mut physics,
            &invalid_json,
            ZERO_JOINT_STATE_JSON,
            Point::origin(),
        )
        .expect_err("unknown collision fields must reject");
        assert!(
            error
                .iter()
                .any(|finding| finding.starts_with("MORPHOLOGY_JSON_INVALID")),
            "{error:?}"
        );
        assert_eq!(physics.body_count(), 0);

        let mut morphology: serde_json::Value =
            serde_json::from_str(MORPHOLOGY_JSON).expect("morphology parses");
        morphology["links"][0]["collision"]["shape"] = serde_json::json!("capsule");
        let invalid_json =
            serde_json::to_string(&morphology).expect("mutated morphology serializes");
        let error = spawn_static_collision_assembly(
            &mut physics,
            &invalid_json,
            ZERO_JOINT_STATE_JSON,
            Point::origin(),
        )
        .expect_err("only declared box geometry is supported");
        assert!(
            error
                .iter()
                .any(|finding| finding == "MORPHOLOGY_COLLISION_SHAPE_OR_SOURCE_UNSUPPORTED"),
            "{error:?}"
        );
        assert_eq!(physics.body_count(), 0);
    }

    #[test]
    fn rejected_morphology_or_joint_state_does_not_mutate_the_world() {
        let mut physics = world();
        let mut invalid: serde_json::Value =
            serde_json::from_str(MORPHOLOGY_JSON).expect("morphology parses");
        invalid["authority"]["physical_actuation_authorized"] = serde_json::json!(true);
        let invalid_json = serde_json::to_string(&invalid).expect("invalid morphology serializes");
        let error = spawn_static_collision_assembly(
            &mut physics,
            &invalid_json,
            ZERO_JOINT_STATE_JSON,
            Point::origin(),
        )
        .expect_err("physical authority must never be admitted");
        assert!(error.iter().any(|finding| finding == "MORPHOLOGY_AUTHORITY_FORBIDDEN"));
        assert_eq!(physics.body_count(), 0);

        let mut state: serde_json::Value =
            serde_json::from_str(ZERO_JOINT_STATE_JSON).expect("joint state parses");
        state["torso_yaw"] = serde_json::json!(1.21);
        let state_json = serde_json::to_string(&state).expect("invalid state serializes");
        let error = spawn_static_collision_assembly(
            &mut physics,
            MORPHOLOGY_JSON,
            &state_json,
            Point::origin(),
        )
        .expect_err("out-of-range joint state must be rejected");
        assert!(error.iter().any(|finding| finding == "JOINT_STATE_OUT_OF_LIMITS"));
        assert_eq!(physics.body_count(), 0);
    }
}
