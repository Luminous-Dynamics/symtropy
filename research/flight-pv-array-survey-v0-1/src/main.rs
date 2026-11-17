use flight_pv_array_survey::{
    ArrayIdentity, CameraFlightEnvelopeSpec, CameraSensorModel, FlightFrame, IrImage,
    MissionEnergyEnvelope, ModuleBounds, RadiometricQc, RasterPlanSpec, RgbImage, SurveyProfile,
    SweepAxis, assess_return_reserve, compute_camera_flight_envelope, evaluate_survey,
    generate_raster_plan,
};

fn identity() -> ArrayIdentity {
    ArrayIdentity {
        mission_id: "synthetic-pv-mission".to_owned(),
        array_id: "synthetic-array-01".to_owned(),
        array_revision: "array-r1".to_owned(),
        plane_frame_id: "array-plane-uv".to_owned(),
        plane_frame_revision: 1,
        aircraft_id: "synthetic-quadrotor".to_owned(),
        aircraft_configuration_revision: "airframe-r1".to_owned(),
        gimbal_id: "gimbal-3axis".to_owned(),
        gimbal_mount_revision: "mount-r1".to_owned(),
        rgb_camera_id: "rgb-camera".to_owned(),
        rgb_camera_revision: "rgb-r1".to_owned(),
        rgb_calibration_id: "rgb-cal".to_owned(),
        rgb_calibration_revision: "rgb-cal-r1".to_owned(),
        rgb_calibration_digest: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_owned(),
        ir_camera_id: "ir-camera".to_owned(),
        ir_camera_revision: "ir-r1".to_owned(),
        ir_calibration_id: "ir-cal".to_owned(),
        ir_calibration_revision: "ir-cal-r1".to_owned(),
        ir_calibration_digest: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .to_owned(),
        ir_nuc_revision: "nuc-r1".to_owned(),
    }
}

fn rgb(id: &str, time: f64, identity: &ArrayIdentity) -> RgbImage {
    RgbImage {
        frame_ref: format!("rgb-frame-{id}"),
        camera_id: identity.rgb_camera_id.clone(),
        camera_revision: identity.rgb_camera_revision.clone(),
        calibration_id: identity.rgb_calibration_id.clone(),
        calibration_revision: identity.rgb_calibration_revision.clone(),
        calibration_digest: identity.rgb_calibration_digest.clone(),
        acquired_at_s: time,
        ground_sample_distance_m_per_pixel: 0.005,
        gsd_uncertainty_m_per_pixel: 0.001,
        exposure_time_s: 0.002,
        sharpness_score: 0.9,
        saturation_fraction: 0.001,
    }
}

fn ir(id: &str, time: f64, identity: &ArrayIdentity) -> IrImage {
    IrImage {
        frame_ref: format!("ir-frame-{id}"),
        camera_id: identity.ir_camera_id.clone(),
        camera_revision: identity.ir_camera_revision.clone(),
        calibration_id: identity.ir_calibration_id.clone(),
        calibration_revision: identity.ir_calibration_revision.clone(),
        calibration_digest: identity.ir_calibration_digest.clone(),
        nuc_revision: identity.ir_nuc_revision.clone(),
        nuc_current: true,
        acquired_at_s: time,
        ground_sample_distance_m_per_pixel: 0.02,
        gsd_uncertainty_m_per_pixel: 0.002,
        detector_integration_time_s: 0.004,
        radiometric_qc: RadiometricQc::Pass,
        irradiance_w_per_m2: 800.0,
        ambient_temperature_c: 25.0,
        relative_humidity_fraction: 0.40,
        wind_speed_m_per_s: 2.0,
        wind_gust_m_per_s: 3.0,
        cloud_variability_fraction: 0.03,
        emissivity: 0.95,
        reflected_apparent_temperature_c: Some(24.0),
        reflection_mitigated: false,
        saturation_fraction: 0.0,
        glare_fraction: 0.01,
    }
}

fn frame(
    id: &str,
    pass_id: &str,
    pass_index: u32,
    center_u: f64,
    view_angle: f64,
    time: f64,
    identity: &ArrayIdentity,
) -> FlightFrame {
    FlightFrame {
        frame_id: id.to_owned(),
        trigger_id: format!("trigger-{id}"),
        pass_id: pass_id.to_owned(),
        pass_index,
        sweep_axis: SweepAxis::U,
        evidence_ref: format!("evidence-{id}"),
        mission_id: identity.mission_id.clone(),
        array_id: identity.array_id.clone(),
        array_revision: identity.array_revision.clone(),
        plane_frame_id: identity.plane_frame_id.clone(),
        plane_frame_revision: identity.plane_frame_revision,
        aircraft_id: identity.aircraft_id.clone(),
        aircraft_configuration_revision: identity.aircraft_configuration_revision.clone(),
        gimbal_id: identity.gimbal_id.clone(),
        gimbal_mount_revision: identity.gimbal_mount_revision.clone(),
        acquired_at_s: time,
        center_u_m: center_u,
        center_v_m: 0.5,
        half_width_u_m: 1.1,
        half_height_v_m: 0.6,
        footprint_uncertainty_m: 0.01,
        pose_uncertainty_m: 0.01,
        camera_height_m: 20.0,
        height_uncertainty_m: 0.5,
        incidence_angle_rad: 0.05,
        view_angle_rad: view_angle,
        ground_speed_m_per_s: 1.3,
        ground_speed_uncertainty_m_per_s: 0.1,
        rgb: rgb(id, time, identity),
        ir: Some(ir(id, time + 0.01, identity)),
    }
}

fn main() {
    let identity = identity();
    let profile = SurveyProfile {
        identity: identity.clone(),
        modules: vec![
            ModuleBounds {
                module_id: "module-01".to_owned(),
                row: 0,
                column: 0,
                u_min_m: 0.0,
                u_max_m: 1.0,
                v_min_m: 0.0,
                v_max_m: 1.0,
            },
            ModuleBounds {
                module_id: "module-02".to_owned(),
                row: 0,
                column: 1,
                u_min_m: 1.0,
                u_max_m: 2.0,
                v_min_m: 0.0,
                v_max_m: 1.0,
            },
        ],
        max_rgb_gsd_m_per_pixel: 0.01,
        max_ir_gsd_m_per_pixel: 0.03,
        max_motion_blur_pixels: 1.0,
        minimum_rgb_sharpness: 0.5,
        maximum_rgb_saturation_fraction: 0.05,
        maximum_ir_saturation_fraction: 0.02,
        maximum_glare_fraction: 0.05,
        maximum_incidence_angle_rad: 0.35,
        minimum_height_m: 5.0,
        maximum_height_m: 30.0,
        maximum_pose_uncertainty_m: 0.05,
        maximum_footprint_uncertainty_m: 0.05,
        minimum_consecutive_overlap_fraction: 0.70,
        minimum_independent_passes: 2,
        minimum_view_angle_separation_rad: 0.30,
        require_radiometric_ir: true,
        minimum_irradiance_w_per_m2: 600.0,
        maximum_wind_speed_m_per_s: 5.0,
        maximum_wind_gust_m_per_s: 7.0,
        maximum_cloud_variability_fraction: 0.10,
        minimum_emissivity: 0.85,
        maximum_emissivity: 0.99,
        maximum_rgb_ir_skew_s: 0.10,
    };
    let camera_envelope = compute_camera_flight_envelope(&CameraFlightEnvelopeSpec {
        rgb: CameraSensorModel {
            horizontal_fov_rad: 1.0,
            vertical_fov_rad: 0.7,
            horizontal_pixels: 4000,
            vertical_pixels: 3000,
            maximum_gsd_m_per_pixel: 0.01,
            gsd_uncertainty_m_per_pixel: 0.001,
            integration_time_s: 0.002,
        },
        ir: CameraSensorModel {
            horizontal_fov_rad: 0.8,
            vertical_fov_rad: 0.6,
            horizontal_pixels: 640,
            vertical_pixels: 512,
            maximum_gsd_m_per_pixel: 0.03,
            gsd_uncertainty_m_per_pixel: 0.002,
            integration_time_s: 0.004,
        },
        requested_height_m: 25.0,
        minimum_height_m: 5.0,
        maximum_height_m: 30.0,
        maximum_motion_blur_pixels: 1.0,
        requested_maximum_ground_speed_m_per_s: 5.0,
        ground_speed_uncertainty_m_per_s: 0.2,
    })
    .expect("synthetic camera envelope should be valid");
    let raster_plan = generate_raster_plan(
        &profile,
        &RasterPlanSpec {
            footprint_half_width_m: 0.6,
            footprint_half_height_m: 0.6,
            forward_overlap_fraction: 0.20,
            side_overlap_fraction: 0.25,
            camera_height_m: 20.0,
            view_angle_rad: 0.0,
        },
    )
    .expect("synthetic raster plan should be valid");
    let observations = vec![
        frame("a0", "pass-a", 0, 1.0, 0.0, 1.0, &identity),
        frame("b0", "pass-b", 0, 1.0, 0.6, 2.0, &identity),
    ];
    let survey = evaluate_survey(&profile, &observations);
    let reserve = assess_return_reserve(&MissionEnergyEnvelope {
        mission_id: identity.mission_id,
        model_id: "synthetic-flight-energy".to_owned(),
        model_revision: "energy-r1".to_owned(),
        evidence_ref: "synthetic-energy-envelope".to_owned(),
        initial_energy_wh: 100.0,
        consumed_energy_wh: 40.0,
        estimated_return_energy_wh: Some(25.0),
        return_energy_uncertainty_wh: 5.0,
        required_reserve_fraction: 0.10,
    });

    println!("suite=FLIGHT-PV-ARRAY-SURVEY-001");
    println!("survey_disposition={:?}", survey.disposition);
    println!("p1_raster_waypoints={}", raster_plan.waypoints.len());
    println!(
        "p2_selected_height_m={:.3}",
        camera_envelope.selected_height_m
    );
    println!(
        "p2_maximum_commanded_speed_m_per_s={:.3}",
        camera_envelope.maximum_commanded_speed_m_per_s
    );
    println!(
        "p1_raster_route_length_lower_bound_m={:.3}",
        raster_plan.route_length_lower_bound_m
    );
    println!(
        "geometric_modules={}/{}",
        survey.geometrically_qualified_modules, survey.expected_modules
    );
    println!(
        "radiometric_modules={}/{}",
        survey.radiometrically_eligible_modules, survey.expected_modules
    );
    println!("return_reserve={:?}", reserve.disposition);
    println!("note=synthetic survey evidence only; not PV fault diagnosis or flight authority");
}
