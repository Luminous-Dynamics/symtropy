// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use bevy::prelude::{Quat, Vec3};
use symthaea_bevy_brain::{
    ArtistCameraPoseSample, ArtistEyeObservation, ArtistEyePyramidLevel,
    ArtistEyeSpatialEvidence, ArtistTemporalConfig, ArtistTemporalError, ArtistTemporalFrame,
    ArtistTemporalTransition, ArtistTemporalWindow, EdgeOrientationEvidence,
    FocalHierarchyEvidence, FocalRegionEvidence, SilhouetteEvidence, StudioFrame,
    SymmetryEvidence, ValueMassEvidence,
};

fn observation(frame: u64, focal_x: f64, occupied: f64, luminance: f64) -> ArtistEyeObservation {
    ArtistEyeObservation {
        capture_id: format!("capture-{frame}"),
        revision_id: "r1".into(),
        frame: StudioFrame(frame),
        rendered_scene_hash: format!("scene-{frame}"),
        spatial: ArtistEyeSpatialEvidence {
            levels: vec![ArtistEyePyramidLevel {
                level: 0,
                width: 16,
                height: 16,
                mean_luminance: luminance,
                rms_contrast: 0.2,
                value_mass: ValueMassEvidence {
                    dark_fraction: 0.2,
                    mid_fraction: 0.6,
                    light_fraction: 0.2,
                },
                silhouette: SilhouetteEvidence {
                    estimated_background_luminance: 0.1,
                    occupied_fraction: occupied,
                    negative_space_fraction: 1.0 - occupied,
                    occupied_components: 1,
                    negative_space_components: 1,
                    largest_occupied_component_fraction: occupied,
                    largest_negative_component_fraction: 1.0 - occupied,
                    occupied_border_contact_fraction: 0.0,
                },
                edges: EdgeOrientationEvidence {
                    horizontal: 0.1,
                    vertical: 0.2,
                    diagonal_down: 0.05,
                    diagonal_up: 0.05,
                    mean_gradient_magnitude: 0.1,
                },
                symmetry: SymmetryEvidence {
                    left_right_mismatch: 0.1,
                    top_bottom_mismatch: 0.1,
                },
                focal_hierarchy: FocalHierarchyEvidence {
                    regions: vec![FocalRegionEvidence {
                        grid_x: 0,
                        grid_y: 0,
                        center_x_normalized: focal_x,
                        center_y_normalized: 0.5,
                        value_separation: 0.5,
                        local_contrast: 0.4,
                        local_edge_energy: 0.3,
                        salience_magnitude: 0.7,
                    }],
                    strongest_minus_second: 0.7,
                    strongest_fraction_of_reported_salience: 1.0,
                },
            }],
        },
    }
}

fn frame(frame: u64, focal_x: f64, occupied: f64, camera_x: f32) -> ArtistTemporalFrame {
    ArtistTemporalFrame {
        camera_stable_id: "art-camera".into(),
        spatial: observation(frame, focal_x, occupied, 0.4 + frame as f64 * 0.01),
        depth: None,
        camera_pose: Some(ArtistCameraPoseSample {
            position: Vec3::new(camera_x, 0.0, 0.0),
            rotation: Quat::IDENTITY,
        }),
    }
}

#[test]
fn transition_keeps_focal_and_camera_motion_separate() {
    let a = frame(10, 0.25, 0.3, 0.0);
    let b = frame(11, 0.75, 0.4, 1.0);
    let transition = ArtistTemporalTransition::between(
        &a,
        &b,
        ArtistTemporalConfig { max_frame_gap: 1 },
    )
    .unwrap();

    assert_eq!(transition.frame_gap, 1);
    assert!(transition.focal_migration[0]
        .strongest_region_migration
        .unwrap()
        > 0.49);
    assert!((transition.camera_motion.unwrap().translation_meters - 1.0).abs() < 1e-9);
    assert!((transition.visibility_change.occupied_fraction_delta - 0.1).abs() < 1e-9);
}

#[test]
fn non_monotonic_or_gapped_windows_fail_closed() {
    let a = frame(10, 0.25, 0.3, 0.0);
    let same = frame(10, 0.5, 0.3, 0.0);
    assert!(matches!(
        ArtistTemporalTransition::between(&a, &same, ArtistTemporalConfig::default()),
        Err(ArtistTemporalError::NonMonotonicFrames)
    ));

    let far = frame(20, 0.5, 0.3, 0.0);
    assert!(matches!(
        ArtistTemporalTransition::between(
            &a,
            &far,
            ArtistTemporalConfig { max_frame_gap: 2 }
        ),
        Err(ArtistTemporalError::FrameGapExceeded { .. })
    ));
}

#[test]
fn temporal_window_reports_descriptive_rhythm_without_score() {
    let window = ArtistTemporalWindow::build(
        vec![
            frame(1, 0.2, 0.3, 0.0),
            frame(2, 0.4, 0.35, 0.5),
            frame(3, 0.7, 0.4, 1.0),
        ],
        ArtistTemporalConfig { max_frame_gap: 1 },
    )
    .unwrap();

    assert_eq!(window.transitions.len(), 2);
    assert_eq!(window.rhythm.total_frame_span, 2);
    assert!(window.rhythm.mean_focal_migration.unwrap() > 0.0);
    assert!(window.rhythm.mean_camera_translation_meters.unwrap() > 0.0);
}
