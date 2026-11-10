// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Emit a deterministic, synthetic visibility request for the CLI workflow test.

use scan_bim_replay_contract::metrics::Point3;
use scan_bim_replay_contract::sampling::{
    AreaWeightedSamplingProfile, SurfaceTriangleInput, SurfaceVisibility,
    sample_reference_surfaces_for_visibility,
};
use scan_bim_visibility_evaluator::visibility::{
    AxisOrder, CoordinateHandedness, LinearUnit, OccluderCoverage, OccluderKind,
    OccluderTriangle, VisibilityProfile, VisibilityRequest, VisibilityView,
    canonical_occluder_manifest_sha256,
};
use std::process::ExitCode;

fn main() -> ExitCode {
    let target = OccluderTriangle {
        surface_id: "REFERENCE-WALL".to_owned(),
        triangle_id: "REFERENCE-WALL-TRI-001".to_owned(),
        a: Point3::new(5.0, -1.0, -1.0),
        b: Point3::new(5.0, 0.0, 1.0),
        c: Point3::new(5.0, 1.0, -1.0),
        kind: OccluderKind::SameBuilding,
    };

    let reference_triangle = SurfaceTriangleInput::new(
        target.surface_id.clone(),
        target.triangle_id.clone(),
        target.a,
        target.b,
        target.c,
        SurfaceVisibility::Unknown,
    );
    let reference_sampling = match sample_reference_surfaces_for_visibility(
        &[reference_triangle],
        AreaWeightedSamplingProfile {
            target_spacing_m: 0.5,
            seed: 42,
            max_samples: 100,
        },
    ) {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("synthetic request: reference sampling failed: {error}");
            return ExitCode::FAILURE;
        }
    };

    let occluders = vec![target];
    let request = VisibilityRequest {
        scene_id: "SYNTHETIC-ROOM-001".to_owned(),
        frame_id: "BUILDING-LOCAL".to_owned(),
        linear_unit: LinearUnit::Metre,
        axis_order: AxisOrder::Xyz,
        handedness: CoordinateHandedness::RightHanded,
        reference_model_sha256: "1".repeat(64),
        reference_sampling,
        occluder_manifest_sha256: canonical_occluder_manifest_sha256(&occluders),
        occluder_coverage: OccluderCoverage::Complete,
        views: vec![VisibilityView {
            view_id: "VIEW-001".to_owned(),
            sensor_id: "SENSOR-001".to_owned(),
            sensor_frame_id: "SENSOR-LOCAL".to_owned(),
            frame_id: "BUILDING-LOCAL".to_owned(),
            pose_revision_id: "POSE-REV-001".to_owned(),
            transform_chain_sha256: "2".repeat(64),
            origin: Point3::new(0.0, 0.0, 0.0),
            forward: Point3::new(1.0, 0.0, 0.0),
            right: Point3::new(0.0, 1.0, 0.0),
            up: Point3::new(0.0, 0.0, 1.0),
            horizontal_fov_rad: 1.2,
            vertical_fov_rad: 1.2,
            max_range_m: 30.0,
            pose_evidence_sha256: "3".repeat(64),
            configuration_sha256: "4".repeat(64),
        }],
        occluders,
        profile: VisibilityProfile {
            max_probe_count: 1_000,
            max_view_count: 8,
            max_occluder_triangles: 1_000,
            max_probe_view_evaluations: 8_000,
            max_ray_tests: 100_000,
            max_bvh_node_visits: 100_000,
            min_incidence_cosine: 0.0,
        },
    };

    match serde_json::to_writer(std::io::stdout().lock(), &request) {
        Ok(()) => {
            println!();
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("synthetic request: serialization failed: {error}");
            ExitCode::FAILURE
        }
    }
}
