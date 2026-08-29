// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symthaea_bevy_brain::{
    assess_depth_takeover, ObjectCameraMotionEvidence, ObjectDepthFusionFrame,
    ObjectDepthPixel, ObjectIdRegistry, ObjectIdentityEvent, ObjectIdentityTransition,
    OcclusionAssessment, OcclusionSupportThresholds, OcclusionTransitionKind,
    PersistentObjectTransition, SemanticTransformDelta, StudioFrame,
};

fn thresholds() -> OcclusionSupportThresholds {
    // Synthetic regression thresholds only. Confirmatory VART-OCC-001 values
    // must be frozen prospectively and are deliberately not defined here.
    OcclusionSupportThresholds {
        minimum_takeover_fraction: 0.5,
        minimum_closer_fraction: 0.8,
        minimum_depth_margin_meters: 0.5,
        maximum_target_translation: 0.01,
        maximum_target_rotation: 0.01,
        maximum_target_scale_delta: 0.01,
        maximum_camera_translation: 0.01,
        maximum_camera_rotation: 0.01,
    }
}

fn transition(events: Vec<ObjectIdentityEvent>, target_motion: f64) -> ObjectIdentityTransition {
    ObjectIdentityTransition {
        camera_stable_id: "camera".into(),
        from_frame: StudioFrame(10),
        to_frame: StudioFrame(11),
        frame_gap: 1,
        from_revision: "r10".into(),
        to_revision: "r11".into(),
        from_scene_hash: "h10".into(),
        to_scene_hash: "h11".into(),
        camera_motion: Some(ObjectCameraMotionEvidence {
            translation_meters: 0.0,
            rotation_radians: 0.0,
        }),
        objects: vec![PersistentObjectTransition {
            stable_id: "target".into(),
            existed_from: true,
            existed_to: true,
            raster_visible_from: true,
            raster_visible_to: false,
            semantic_transform_delta: Some(SemanticTransformDelta {
                translation_distance: target_motion,
                rotation_radians: 0.0,
                scale_l2_delta: 0.0,
            }),
            screen_trajectory: None,
            events,
        }],
    }
}

fn fused(
    frame: u64,
    revision: &str,
    hash: &str,
    registry: &ObjectIdRegistry,
    pixels: &[(u32, Option<f32>)],
) -> ObjectDepthFusionFrame {
    ObjectDepthFusionFrame {
        object_capture_id: format!("objects-{frame}"),
        depth_capture_id: format!("depth-{frame}"),
        revision_id: revision.into(),
        frame: StudioFrame(frame),
        scene_hash: hash.into(),
        camera_stable_id: "camera".into(),
        width: pixels.len() as u32,
        height: 1,
        registry_digest: registry.digest().into(),
        objects: Vec::new(),
        pixels: pixels
            .iter()
            .map(|(raster_id, depth_meters)| ObjectDepthPixel {
                raster_id: *raster_id,
                depth_meters: *depth_meters,
            })
            .collect(),
    }
}

#[test]
fn visibility_loss_can_receive_depth_takeover_support() {
    let registry = ObjectIdRegistry::from_stable_ids(["target", "occluder"]).unwrap();
    let target = registry.raster_id("target").unwrap();
    let occluder = registry.raster_id("occluder").unwrap();
    let from = fused(
        10,
        "r10",
        "h10",
        &registry,
        &[(target, Some(10.0)), (target, Some(10.0)), (target, Some(10.0)), (0, None)],
    );
    let to = fused(
        11,
        "r11",
        "h11",
        &registry,
        &[(occluder, Some(4.0)), (occluder, Some(4.0)), (occluder, Some(4.0)), (0, None)],
    );

    let assessment = assess_depth_takeover(
        &transition(vec![ObjectIdentityEvent::RasterVisibilityLost], 0.0),
        &from,
        &to,
        &registry,
        "target",
        thresholds(),
    )
    .unwrap();

    match assessment {
        OcclusionAssessment::DepthTakeoverSupported {
            kind,
            target_stable_id,
            occluder: evidence,
        } => {
            assert_eq!(kind, OcclusionTransitionKind::VisibilityLoss);
            assert_eq!(target_stable_id, "target");
            assert_eq!(evidence.stable_id, "occluder");
            assert_eq!(evidence.takeover_fraction, 1.0);
            assert_eq!(evidence.closer_fraction, 1.0);
        }
        other => panic!("unexpected assessment: {other:?}"),
    }
}

#[test]
fn authored_hide_is_never_relabelled_as_depth_occlusion() {
    let registry = ObjectIdRegistry::from_stable_ids(["target", "occluder"]).unwrap();
    let target = registry.raster_id("target").unwrap();
    let occluder = registry.raster_id("occluder").unwrap();
    let from = fused(10, "r10", "h10", &registry, &[(target, Some(10.0))]);
    let to = fused(11, "r11", "h11", &registry, &[(occluder, Some(2.0))]);
    let assessment = assess_depth_takeover(
        &transition(
            vec![
                ObjectIdentityEvent::AuthoredVisibilityDisabled,
                ObjectIdentityEvent::RasterVisibilityLost,
            ],
            0.0,
        ),
        &from,
        &to,
        &registry,
        "target",
        thresholds(),
    )
    .unwrap();
    assert_eq!(assessment, OcclusionAssessment::NotAQualifyingVisibilityTransition);
}

#[test]
fn moving_target_fails_static_occlusion_precondition() {
    let registry = ObjectIdRegistry::from_stable_ids(["target", "occluder"]).unwrap();
    let target = registry.raster_id("target").unwrap();
    let occluder = registry.raster_id("occluder").unwrap();
    let from = fused(10, "r10", "h10", &registry, &[(target, Some(10.0))]);
    let to = fused(11, "r11", "h11", &registry, &[(occluder, Some(2.0))]);
    let assessment = assess_depth_takeover(
        &transition(vec![ObjectIdentityEvent::RasterVisibilityLost], 1.0),
        &from,
        &to,
        &registry,
        "target",
        thresholds(),
    )
    .unwrap();
    assert_eq!(assessment, OcclusionAssessment::StabilityPreconditionsFailed);
}
