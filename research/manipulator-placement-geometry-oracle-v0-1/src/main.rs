use manipulator_placement_geometry_oracle::{
    PartSpecification, PlacementObservation, WorkcellIdentity, WorkcellProfile, evaluate_placements,
};

fn main() {
    let identity = WorkcellIdentity {
        workpiece_id: "synthetic-workpiece-01".to_owned(),
        workpiece_revision: "workpiece-r1".to_owned(),
        fixture_id: "synthetic-fixture-01".to_owned(),
        fixture_revision: "fixture-r1".to_owned(),
        frame_id: "cell:world".to_owned(),
        frame_revision: 1,
        tool_id: "gripper:parallel-jaw".to_owned(),
        tool_mount_revision: "mount-r1".to_owned(),
        calibration_id: "calibration:reference".to_owned(),
        calibration_revision: "cal-r1".to_owned(),
        calibration_digest: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_owned(),
    };
    let parts = ["A", "B", "C", "D"]
        .into_iter()
        .enumerate()
        .map(|(index, suffix)| PartSpecification {
            component_id: format!("component-{suffix}"),
            target_position_m: [index as f64 * 0.1, 0.0, 0.0],
            target_orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            target_seat_depth_m: 0.02,
            translation_tolerance_m: 0.01,
            orientation_tolerance_rad: 0.01,
            seat_depth_tolerance_m: 0.002,
        })
        .collect();
    let profile = WorkcellProfile {
        identity: identity.clone(),
        parts,
    };
    let observations: Vec<PlacementObservation> = ["A", "B", "C", "D"]
        .into_iter()
        .enumerate()
        .map(|(index, suffix)| PlacementObservation {
            observation_id: format!("observation-{suffix}"),
            evidence_ref: format!("synthetic-evidence-{suffix}"),
            workpiece_id: identity.workpiece_id.clone(),
            workpiece_revision: identity.workpiece_revision.clone(),
            fixture_id: identity.fixture_id.clone(),
            fixture_revision: identity.fixture_revision.clone(),
            component_id: format!("component-{suffix}"),
            frame_id: identity.frame_id.clone(),
            frame_revision: identity.frame_revision,
            tool_id: identity.tool_id.clone(),
            tool_mount_revision: identity.tool_mount_revision.clone(),
            calibration_id: identity.calibration_id.clone(),
            calibration_revision: identity.calibration_revision.clone(),
            calibration_digest: identity.calibration_digest.clone(),
            measured_position_m: [index as f64 * 0.1, 0.0, 0.0],
            measured_orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            measured_seat_depth_m: 0.02,
            translation_uncertainty_m: 0.001,
            orientation_uncertainty_rad: 0.001,
            seat_depth_uncertainty_m: 0.0002,
        })
        .collect();

    let result = evaluate_placements(&profile, &observations);
    println!("suite=MANIPULATOR-PLACEMENT-GEOMETRY-ORACLE-001");
    println!("disposition={:?}", result.disposition);
    println!("expected_parts={}", result.expected_parts);
    println!("observed_parts={}", result.observed_parts);
    println!("note=geometric disposition only; not production acceptance or actuation authority");
}
