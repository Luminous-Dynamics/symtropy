// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use bevy::prelude::{Quat, Vec3};
use symthaea_bevy_brain::{
    attribute_transition_motion, ArtistCameraPoseSample, MotionAttributionConfig,
    ObjectBoundingBox, ObjectIdObservation, ObjectIdPlaneEvidence, ObjectIdRegistry,
    ObjectIdentityEvent, ObjectIdentityTransition, ObjectMotionAttribution,
    ObjectRasterEvidence, PersistentObjectFrame, SemanticObjectFrame, SemanticObjectState,
    StudioFrame,
};

fn semantic(frame: u64, x: f32, scene_hash: &str) -> SemanticObjectFrame {
    SemanticObjectFrame {
        revision_id: format!("r-{frame}"),
        frame: StudioFrame(frame),
        scene_hash: scene_hash.into(),
        objects: vec![SemanticObjectState {
            stable_id: "form".into(),
            parent_id: None,
            kind: "form".into(),
            material_id: Some("clay".into()),
            translation: [x, 0.0, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            authored_visible: true,
        }],
    }
}

fn raster(
    frame: u64,
    scene_hash: &str,
    registry: &ObjectIdRegistry,
    centroid_x: Option<f64>,
) -> ObjectIdObservation {
    let objects = centroid_x
        .map(|x| {
            vec![ObjectRasterEvidence {
                stable_id: "form".into(),
                raster_id: registry.raster_id("form").unwrap(),
                visible_pixels: 100,
                visible_fraction: 0.1,
                centroid_x_normalized: x,
                centroid_y_normalized: 0.5,
                bounding_box: ObjectBoundingBox {
                    min_x: 10,
                    min_y: 10,
                    max_x: 19,
                    max_y: 19,
                },
                touches_frame_border: false,
            }]
        })
        .unwrap_or_default();
    ObjectIdObservation {
        capture_id: format!("object-{frame}"),
        revision_id: format!("r-{frame}"),
        frame: StudioFrame(frame),
        rendered_scene_hash: scene_hash.into(),
        registry_digest: registry.digest().into(),
        evidence: ObjectIdPlaneEvidence {
            width: 100,
            height: 100,
            background_fraction: if objects.is_empty() { 1.0 } else { 0.9 },
            visible_object_count: objects.len() as u32,
            objects,
        },
    }
}

fn frame(
    registry: &ObjectIdRegistry,
    frame: u64,
    x: f32,
    centroid_x: Option<f64>,
    camera_x: f32,
) -> PersistentObjectFrame {
    let hash = format!("scene-{frame}");
    PersistentObjectFrame::new(
        "art-camera",
        semantic(frame, x, &hash),
        raster(frame, &hash, registry, centroid_x),
        registry,
        Some(ArtistCameraPoseSample {
            position: Vec3::new(camera_x, 0.0, 0.0),
            rotation: Quat::IDENTITY,
        }),
    )
    .unwrap()
}

#[test]
fn semantic_motion_and_screen_motion_are_not_collapsed() {
    let registry = ObjectIdRegistry::from_stable_ids(["form"]).unwrap();
    let a = frame(&registry, 10, 0.0, Some(0.25), 0.0);
    let b = frame(&registry, 11, 1.0, Some(0.40), 0.0);
    let transition = ObjectIdentityTransition::between(&a, &b, &registry, 2).unwrap();
    let object = transition.object("form").unwrap();
    let semantic = object.semantic_transform_delta.unwrap();
    assert!((semantic.translation_distance - 1.0).abs() < 1e-8);
    let screen = object.screen_trajectory.unwrap();
    assert!((screen.centroid_dx_normalized - 0.15).abs() < 1e-8);

    let attribution = attribute_transition_motion(&transition, MotionAttributionConfig::default())
        .unwrap();
    assert_eq!(
        attribution[0].attribution,
        ObjectMotionAttribution::SemanticTransformMotion
    );
}

#[test]
fn camera_motion_with_static_semantic_transform_stays_separate() {
    let registry = ObjectIdRegistry::from_stable_ids(["form"]).unwrap();
    let a = frame(&registry, 20, 0.0, Some(0.50), 0.0);
    let b = frame(&registry, 21, 0.0, Some(0.35), 1.0);
    let transition = ObjectIdentityTransition::between(&a, &b, &registry, 2).unwrap();
    let attribution = attribute_transition_motion(&transition, MotionAttributionConfig::default())
        .unwrap();
    assert_eq!(
        attribution[0].attribution,
        ObjectMotionAttribution::CameraMotionWithSemanticTransformStable
    );
}

#[test]
fn raster_loss_is_not_semantic_destruction_or_claimed_concealment() {
    let registry = ObjectIdRegistry::from_stable_ids(["form"]).unwrap();
    let a = frame(&registry, 30, 0.0, Some(0.5), 0.0);
    let b = frame(&registry, 31, 0.0, None, 0.0);
    let transition = ObjectIdentityTransition::between(&a, &b, &registry, 2).unwrap();
    let object = transition.object("form").unwrap();
    assert!(object.events.contains(&ObjectIdentityEvent::RasterVisibilityLost));
    assert!(!object.events.contains(&ObjectIdentityEvent::SemanticDestroyed));

    let attribution = attribute_transition_motion(&transition, MotionAttributionConfig::default())
        .unwrap();
    assert_eq!(attribution[0].attribution, ObjectMotionAttribution::VisibilityTransition);
}

#[test]
fn registry_change_inside_lineage_is_rejected() {
    let a_registry = ObjectIdRegistry::from_stable_ids(["form"]).unwrap();
    let b_registry = ObjectIdRegistry::from_stable_ids(["form", "other"]).unwrap();
    let a = frame(&a_registry, 40, 0.0, Some(0.5), 0.0);
    let b = frame(&b_registry, 41, 0.0, Some(0.5), 0.0);
    assert!(ObjectIdentityTransition::between(&a, &b, &a_registry, 2).is_err());
}
