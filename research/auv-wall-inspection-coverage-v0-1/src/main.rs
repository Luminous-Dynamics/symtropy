use auv_wall_inspection_coverage::{
    ReturnReserveSnapshot, SurfaceIdentity, SurfaceObservation, SurfaceProfile,
    assess_return_reserve, evaluate_coverage,
};

fn main() {
    let identity = SurfaceIdentity {
        mission_id: "synthetic-wall-mission".to_owned(),
        surface_id: "synthetic-dam-panel".to_owned(),
        surface_revision: "surface-r1".to_owned(),
        frame_id: "wall-local-uv".to_owned(),
        frame_revision: 1,
        sensor_id: "synthetic-msis".to_owned(),
        sensor_revision: "sensor-r1".to_owned(),
        calibration_id: "sonar-calibration".to_owned(),
        calibration_revision: "cal-r1".to_owned(),
        calibration_digest: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_owned(),
    };
    let profile = SurfaceProfile {
        identity: identity.clone(),
        length_u_m: 2.0,
        height_v_m: 2.0,
        cell_size_u_m: 1.0,
        cell_size_v_m: 1.0,
        min_sonar_range_m: 0.3,
        max_sonar_range_m: 12.0,
        min_return_strength_db: 22.0,
        max_resolution_m: 0.1,
        max_incidence_angle_rad: 0.7,
        target_standoff_m: 1.5,
        standoff_tolerance_m: 0.15,
        max_standoff_uncertainty_m: 0.05,
        max_navigation_uncertainty_m: 0.1,
        max_footprint_uncertainty_m: 0.1,
        max_heading_error_rad: 0.2,
        minimum_independent_passes: 2,
        minimum_view_angle_separation_rad: 0.4,
    };
    let observations: Vec<SurfaceObservation> = [
        ("track-A", "ping-A", 0.0, 1.0),
        ("track-B", "ping-B", 0.8, 2.0),
    ]
    .into_iter()
    .map(
        |(pass_id, ping_id, view_angle_rad, acquired_at_s)| SurfaceObservation {
            observation_id: format!("observation-{ping_id}"),
            ping_id: ping_id.to_owned(),
            pass_id: pass_id.to_owned(),
            evidence_ref: format!("synthetic-evidence-{ping_id}"),
            mission_id: identity.mission_id.clone(),
            surface_id: identity.surface_id.clone(),
            surface_revision: identity.surface_revision.clone(),
            frame_id: identity.frame_id.clone(),
            frame_revision: identity.frame_revision,
            sensor_id: identity.sensor_id.clone(),
            sensor_revision: identity.sensor_revision.clone(),
            calibration_id: identity.calibration_id.clone(),
            calibration_revision: identity.calibration_revision.clone(),
            calibration_digest: identity.calibration_digest.clone(),
            acquired_at_s,
            center_u_m: 1.0,
            center_v_m: 1.0,
            half_width_u_m: 1.0,
            half_height_v_m: 1.0,
            footprint_uncertainty_m: 0.0,
            navigation_uncertainty_m: 0.0,
            sonar_range_m: 8.0,
            range_uncertainty_m: 0.1,
            resolution_m: 0.05,
            return_strength_db: 30.0,
            incidence_angle_rad: 0.2,
            standoff_m: 1.5,
            standoff_uncertainty_m: 0.02,
            heading_error_rad: 0.02,
            view_angle_rad,
        },
    )
    .collect();
    let coverage = evaluate_coverage(&profile, &observations);
    let reserve = assess_return_reserve(&ReturnReserveSnapshot {
        mission_id: identity.mission_id,
        energy_model_id: "synthetic-auv-energy".to_owned(),
        energy_model_revision: "model-r1".to_owned(),
        evidence_ref: "synthetic-energy-receipt".to_owned(),
        initial_energy_j: 1000.0,
        consumed_energy_j: 400.0,
        estimated_return_energy_j: Some(300.0),
        return_energy_uncertainty_j: 50.0,
        required_reserve_fraction: 0.1,
    });

    println!("suite=AUV-WALL-INSPECTION-COVERAGE-001");
    println!("surface_disposition={:?}", coverage.disposition);
    println!(
        "qualified_cells={}/{}",
        coverage.qualified_cells, coverage.expected_cells
    );
    println!("return_reserve_disposition={:?}", reserve.disposition);
    println!(
        "note=synthetic coverage and modeled reserve only; no asset condition or safe-return claim"
    );
}
