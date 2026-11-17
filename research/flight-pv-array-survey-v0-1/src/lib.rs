//! Quality-qualified RGB/IR survey coverage for a synthetic photovoltaic array.
//!
//! This is an evaluator-side geometry and acquisition-quality reference. It
//! does not process pixels, diagnose PV faults, authorize flight, or accept
//! maintenance actions.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const MAX_MODULES: usize = 100_000;
const MAX_ABSOLUTE_COORDINATE_M: f64 = 1.0e6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayIdentity {
    pub mission_id: String,
    pub array_id: String,
    pub array_revision: String,
    pub plane_frame_id: String,
    pub plane_frame_revision: u64,
    pub aircraft_id: String,
    pub aircraft_configuration_revision: String,
    pub gimbal_id: String,
    pub gimbal_mount_revision: String,
    pub rgb_camera_id: String,
    pub rgb_camera_revision: String,
    pub rgb_calibration_id: String,
    pub rgb_calibration_revision: String,
    pub rgb_calibration_digest: String,
    pub ir_camera_id: String,
    pub ir_camera_revision: String,
    pub ir_calibration_id: String,
    pub ir_calibration_revision: String,
    pub ir_calibration_digest: String,
    pub ir_nuc_revision: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleBounds {
    pub module_id: String,
    pub row: usize,
    pub column: usize,
    pub u_min_m: f64,
    pub u_max_m: f64,
    pub v_min_m: f64,
    pub v_max_m: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurveyProfile {
    pub identity: ArrayIdentity,
    pub modules: Vec<ModuleBounds>,
    pub max_rgb_gsd_m_per_pixel: f64,
    pub max_ir_gsd_m_per_pixel: f64,
    pub max_motion_blur_pixels: f64,
    pub minimum_rgb_sharpness: f64,
    pub maximum_rgb_saturation_fraction: f64,
    pub maximum_ir_saturation_fraction: f64,
    pub maximum_glare_fraction: f64,
    pub maximum_incidence_angle_rad: f64,
    pub minimum_height_m: f64,
    pub maximum_height_m: f64,
    pub maximum_pose_uncertainty_m: f64,
    pub maximum_footprint_uncertainty_m: f64,
    pub minimum_consecutive_overlap_fraction: f64,
    pub minimum_independent_passes: usize,
    pub minimum_view_angle_separation_rad: f64,
    pub require_radiometric_ir: bool,
    pub minimum_irradiance_w_per_m2: f64,
    pub maximum_wind_speed_m_per_s: f64,
    pub maximum_wind_gust_m_per_s: f64,
    pub maximum_cloud_variability_fraction: f64,
    pub minimum_emissivity: f64,
    pub maximum_emissivity: f64,
    pub maximum_rgb_ir_skew_s: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SweepAxis {
    U,
    V,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RadiometricQc {
    Pass,
    Glare,
    Saturated,
    Drift,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RgbImage {
    pub frame_ref: String,
    pub camera_id: String,
    pub camera_revision: String,
    pub calibration_id: String,
    pub calibration_revision: String,
    pub calibration_digest: String,
    pub acquired_at_s: f64,
    pub ground_sample_distance_m_per_pixel: f64,
    pub gsd_uncertainty_m_per_pixel: f64,
    pub exposure_time_s: f64,
    pub sharpness_score: f64,
    pub saturation_fraction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrImage {
    pub frame_ref: String,
    pub camera_id: String,
    pub camera_revision: String,
    pub calibration_id: String,
    pub calibration_revision: String,
    pub calibration_digest: String,
    pub nuc_revision: String,
    pub nuc_current: bool,
    pub acquired_at_s: f64,
    pub ground_sample_distance_m_per_pixel: f64,
    pub gsd_uncertainty_m_per_pixel: f64,
    pub detector_integration_time_s: f64,
    pub radiometric_qc: RadiometricQc,
    pub irradiance_w_per_m2: f64,
    pub ambient_temperature_c: f64,
    pub relative_humidity_fraction: f64,
    pub wind_speed_m_per_s: f64,
    pub wind_gust_m_per_s: f64,
    pub cloud_variability_fraction: f64,
    pub emissivity: f64,
    pub reflected_apparent_temperature_c: Option<f64>,
    /// True when the acquisition geometry or documented method mitigates reflection.
    pub reflection_mitigated: bool,
    pub saturation_fraction: f64,
    pub glare_fraction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlightFrame {
    pub frame_id: String,
    pub trigger_id: String,
    pub pass_id: String,
    pub pass_index: u32,
    pub sweep_axis: SweepAxis,
    pub evidence_ref: String,
    pub mission_id: String,
    pub array_id: String,
    pub array_revision: String,
    pub plane_frame_id: String,
    pub plane_frame_revision: u64,
    pub aircraft_id: String,
    pub aircraft_configuration_revision: String,
    pub gimbal_id: String,
    pub gimbal_mount_revision: String,
    pub acquired_at_s: f64,
    /// Projected axis-aligned image footprint in the frozen array-plane frame.
    pub center_u_m: f64,
    pub center_v_m: f64,
    pub half_width_u_m: f64,
    pub half_height_v_m: f64,
    pub footprint_uncertainty_m: f64,
    pub pose_uncertainty_m: f64,
    pub camera_height_m: f64,
    pub height_uncertainty_m: f64,
    pub incidence_angle_rad: f64,
    pub view_angle_rad: f64,
    pub ground_speed_m_per_s: f64,
    pub ground_speed_uncertainty_m_per_s: f64,
    pub rgb: RgbImage,
    pub ir: Option<IrImage>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryFrameDisposition {
    Qualified,
    QualityRejected,
    InvalidEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RadiometricFrameDisposition {
    EligibleForSimplifiedSurvey,
    NotEligible,
    Unknown,
    InvalidEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleGeometryDisposition {
    Qualified,
    InsufficientIndependentPasses,
    Unobserved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleRadiometricDisposition {
    EligibleForSimplifiedSurvey,
    NotEligible,
    Unknown,
    InvalidEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurveyDisposition {
    Qualified,
    Incomplete,
    InvalidEvidence,
    Inconsistent,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameAssessment {
    pub frame_id: String,
    pub trigger_id: String,
    pub pass_id: String,
    pub pass_index: u32,
    pub acquired_at_s: f64,
    pub geometry: GeometryFrameDisposition,
    pub radiometric: RadiometricFrameDisposition,
    pub consecutive_overlap_fraction: Option<f64>,
    pub rgb_gsd_upper_bound_m_per_pixel: Option<f64>,
    pub ir_gsd_upper_bound_m_per_pixel: Option<f64>,
    pub predicted_rgb_blur_upper_bound_px: Option<f64>,
    pub predicted_ir_blur_upper_bound_px: Option<f64>,
    pub covered_modules: Vec<String>,
    pub geometry_reason: Option<String>,
    pub radiometric_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleAssessment {
    pub module_id: String,
    pub row: usize,
    pub column: usize,
    pub geometric_passes: usize,
    pub qualified_geometric_passes: usize,
    pub radiometric_passes: usize,
    pub qualified_radiometric_passes: usize,
    pub geometry: ModuleGeometryDisposition,
    pub radiometric: ModuleRadiometricDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurveyReport {
    pub disposition: SurveyDisposition,
    pub expected_modules: usize,
    pub geometrically_qualified_modules: usize,
    pub radiometrically_eligible_modules: usize,
    pub unobserved_modules: usize,
    pub rejected_frames: usize,
    pub invalid_frames: usize,
    pub invalid_radiometric_frames: usize,
    /// Stable order by pass ID, pass index, then frame ID.
    pub frames: Vec<FrameAssessment>,
    /// Stable order by module row, column, then module ID.
    pub modules: Vec<ModuleAssessment>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RasterPlanSpec {
    /// Conservative half-width/half-height of the qualified image footprint in array-plane metres.
    pub footprint_half_width_m: f64,
    pub footprint_half_height_m: f64,
    /// Requested overlap between consecutive images along each lane, in [0, 1).
    pub forward_overlap_fraction: f64,
    /// Requested overlap between adjacent lanes, in [0, 1).
    pub side_overlap_fraction: f64,
    pub camera_height_m: f64,
    pub view_angle_rad: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaypointKind {
    Survey,
    LaneTransition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RasterWaypoint {
    pub ordinal: usize,
    pub lane_index: usize,
    pub point_index: usize,
    pub pass_id: String,
    pub kind: WaypointKind,
    pub center_u_m: f64,
    pub center_v_m: f64,
    pub camera_height_m: f64,
    pub view_angle_rad: f64,
    /// Modules fully inside this planned nominal footprint.
    pub covered_module_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RasterMissionPlan {
    pub waypoints: Vec<RasterWaypoint>,
    pub route_length_lower_bound_m: f64,
    pub forward_spacing_m: f64,
    pub side_spacing_m: f64,
    pub requested_forward_overlap_fraction: f64,
    pub requested_side_overlap_fraction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraSensorModel {
    pub horizontal_fov_rad: f64,
    pub vertical_fov_rad: f64,
    pub horizontal_pixels: u32,
    pub vertical_pixels: u32,
    pub maximum_gsd_m_per_pixel: f64,
    pub gsd_uncertainty_m_per_pixel: f64,
    /// RGB exposure time or IR detector integration time, in seconds.
    pub integration_time_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraFlightEnvelopeSpec {
    pub rgb: CameraSensorModel,
    pub ir: CameraSensorModel,
    pub requested_height_m: f64,
    pub minimum_height_m: f64,
    pub maximum_height_m: f64,
    pub maximum_motion_blur_pixels: f64,
    pub requested_maximum_ground_speed_m_per_s: f64,
    pub ground_speed_uncertainty_m_per_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraFlightEnvelope {
    pub selected_height_m: f64,
    pub gsd_limit_height_m: f64,
    pub rgb_gsd_horizontal_m_per_pixel: f64,
    pub rgb_gsd_vertical_m_per_pixel: f64,
    pub ir_gsd_horizontal_m_per_pixel: f64,
    pub ir_gsd_vertical_m_per_pixel: f64,
    pub common_footprint_half_width_m: f64,
    pub common_footprint_half_height_m: f64,
    pub rgb_motion_blur_speed_limit_m_per_s: f64,
    pub ir_motion_blur_speed_limit_m_per_s: f64,
    /// Commanded speed ceiling after subtracting the declared speed uncertainty.
    pub maximum_commanded_speed_m_per_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MissionEnergyEnvelope {
    pub mission_id: String,
    pub model_id: String,
    pub model_revision: String,
    pub evidence_ref: String,
    pub initial_energy_wh: f64,
    pub consumed_energy_wh: f64,
    pub estimated_return_energy_wh: Option<f64>,
    pub return_energy_uncertainty_wh: f64,
    pub required_reserve_fraction: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReserveDisposition {
    Sufficient,
    Insufficient,
    Unknown,
    InvalidEvidence,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReserveAssessment {
    pub disposition: ReserveDisposition,
    pub remaining_energy_wh: Option<f64>,
    pub conservative_return_requirement_wh: Option<f64>,
    pub required_reserve_wh: Option<f64>,
    pub margin_wh: Option<f64>,
    pub reason: Option<String>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn fraction(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn valid_bounds(module: &ModuleBounds) -> bool {
    [
        module.u_min_m,
        module.u_max_m,
        module.v_min_m,
        module.v_max_m,
    ]
    .iter()
    .all(|value| value.is_finite() && value.abs() <= MAX_ABSOLUTE_COORDINATE_M)
        && module.u_max_m > module.u_min_m
        && module.v_max_m > module.v_min_m
}

fn validate_profile(profile: &SurveyProfile) -> Result<(), String> {
    let identity = &profile.identity;
    if !nonempty(&identity.mission_id)
        || !nonempty(&identity.array_id)
        || !nonempty(&identity.array_revision)
        || !nonempty(&identity.plane_frame_id)
        || !nonempty(&identity.aircraft_id)
        || !nonempty(&identity.aircraft_configuration_revision)
        || !nonempty(&identity.gimbal_id)
        || !nonempty(&identity.gimbal_mount_revision)
        || !nonempty(&identity.rgb_camera_id)
        || !nonempty(&identity.rgb_camera_revision)
        || !nonempty(&identity.rgb_calibration_id)
        || !nonempty(&identity.rgb_calibration_revision)
        || !nonempty(&identity.ir_camera_id)
        || !nonempty(&identity.ir_camera_revision)
        || !nonempty(&identity.ir_calibration_id)
        || !nonempty(&identity.ir_calibration_revision)
        || !nonempty(&identity.ir_nuc_revision)
    {
        return Err(
            "mission, array, frame, aircraft, gimbal or camera identity is incomplete".to_owned(),
        );
    }
    if !valid_digest(&identity.rgb_calibration_digest)
        || !valid_digest(&identity.ir_calibration_digest)
    {
        return Err("RGB/IR calibration digest must be lowercase SHA-256 hex".to_owned());
    }
    if profile.modules.is_empty() || profile.modules.len() > MAX_MODULES {
        return Err("array module roster is empty or exceeds supported size".to_owned());
    }

    let mut ids = BTreeSet::new();
    let mut coordinates = BTreeSet::new();
    for module in &profile.modules {
        if !nonempty(&module.module_id) || !valid_bounds(module) {
            return Err(format!(
                "module {} has missing identity or invalid bounds",
                module.module_id
            ));
        }
        if !ids.insert(module.module_id.as_str()) {
            return Err(format!("duplicate module identity: {}", module.module_id));
        }
        if !coordinates.insert((module.row, module.column)) {
            return Err(format!(
                "duplicate module grid coordinate: row {}, column {}",
                module.row, module.column
            ));
        }
    }

    if !profile.max_rgb_gsd_m_per_pixel.is_finite()
        || profile.max_rgb_gsd_m_per_pixel <= 0.0
        || !profile.max_ir_gsd_m_per_pixel.is_finite()
        || profile.max_ir_gsd_m_per_pixel <= 0.0
        || !profile.max_motion_blur_pixels.is_finite()
        || profile.max_motion_blur_pixels <= 0.0
        || !profile.minimum_rgb_sharpness.is_finite()
        || profile.minimum_rgb_sharpness < 0.0
        || !fraction(profile.maximum_rgb_saturation_fraction)
        || !fraction(profile.maximum_ir_saturation_fraction)
        || !fraction(profile.maximum_glare_fraction)
        || !profile.maximum_incidence_angle_rad.is_finite()
        || profile.maximum_incidence_angle_rad < 0.0
        || profile.maximum_incidence_angle_rad > std::f64::consts::FRAC_PI_2
        || !profile.minimum_height_m.is_finite()
        || profile.minimum_height_m < 0.0
        || !profile.maximum_height_m.is_finite()
        || profile.maximum_height_m <= profile.minimum_height_m
        || !finite_nonnegative(profile.maximum_pose_uncertainty_m)
        || !finite_nonnegative(profile.maximum_footprint_uncertainty_m)
        || !fraction(profile.minimum_consecutive_overlap_fraction)
        || profile.minimum_independent_passes == 0
        || !profile.minimum_view_angle_separation_rad.is_finite()
        || profile.minimum_view_angle_separation_rad < 0.0
        || profile.minimum_view_angle_separation_rad > std::f64::consts::PI
        || !profile.minimum_irradiance_w_per_m2.is_finite()
        || profile.minimum_irradiance_w_per_m2 < 0.0
        || !profile.maximum_wind_speed_m_per_s.is_finite()
        || profile.maximum_wind_speed_m_per_s < 0.0
        || !profile.maximum_wind_gust_m_per_s.is_finite()
        || profile.maximum_wind_gust_m_per_s < 0.0
        || !fraction(profile.maximum_cloud_variability_fraction)
        || !profile.minimum_emissivity.is_finite()
        || !profile.maximum_emissivity.is_finite()
        || profile.minimum_emissivity <= 0.0
        || profile.maximum_emissivity > 1.0
        || profile.minimum_emissivity > profile.maximum_emissivity
        || !profile.maximum_rgb_ir_skew_s.is_finite()
        || profile.maximum_rgb_ir_skew_s < 0.0
    {
        return Err(
            "PV survey profile contains invalid sensor, geometry, environment or quality limits"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_frame_identity(profile: &SurveyProfile, frame: &FlightFrame) -> Result<(), String> {
    let expected = &profile.identity;
    if !nonempty(&frame.frame_id)
        || !nonempty(&frame.trigger_id)
        || !nonempty(&frame.pass_id)
        || !nonempty(&frame.evidence_ref)
        || !nonempty(&frame.rgb.frame_ref)
    {
        return Err("frame, trigger, pass or RGB evidence identity is missing".to_owned());
    }
    if frame.mission_id != expected.mission_id
        || frame.array_id != expected.array_id
        || frame.array_revision != expected.array_revision
    {
        return Err("mission/array identity or revision does not match profile".to_owned());
    }
    if frame.plane_frame_id != expected.plane_frame_id
        || frame.plane_frame_revision != expected.plane_frame_revision
    {
        return Err("array-plane frame identity/revision does not match profile".to_owned());
    }
    if frame.aircraft_id != expected.aircraft_id
        || frame.aircraft_configuration_revision != expected.aircraft_configuration_revision
        || frame.gimbal_id != expected.gimbal_id
        || frame.gimbal_mount_revision != expected.gimbal_mount_revision
    {
        return Err(
            "aircraft configuration or gimbal mount revision does not match profile".to_owned(),
        );
    }
    let rgb = &frame.rgb;
    if rgb.camera_id != expected.rgb_camera_id
        || rgb.camera_revision != expected.rgb_camera_revision
        || rgb.calibration_id != expected.rgb_calibration_id
        || rgb.calibration_revision != expected.rgb_calibration_revision
        || rgb.calibration_digest != expected.rgb_calibration_digest
    {
        return Err(
            "RGB camera/calibration identity or revision does not match profile".to_owned(),
        );
    }
    Ok(())
}

fn validate_frame_values(frame: &FlightFrame) -> Result<(), String> {
    let values = [
        frame.acquired_at_s,
        frame.center_u_m,
        frame.center_v_m,
        frame.half_width_u_m,
        frame.half_height_v_m,
        frame.footprint_uncertainty_m,
        frame.pose_uncertainty_m,
        frame.camera_height_m,
        frame.height_uncertainty_m,
        frame.incidence_angle_rad,
        frame.view_angle_rad,
        frame.ground_speed_m_per_s,
        frame.ground_speed_uncertainty_m_per_s,
        frame.rgb.acquired_at_s,
        frame.rgb.ground_sample_distance_m_per_pixel,
        frame.rgb.gsd_uncertainty_m_per_pixel,
        frame.rgb.exposure_time_s,
        frame.rgb.sharpness_score,
        frame.rgb.saturation_fraction,
    ];
    if !values.iter().all(|value| value.is_finite()) {
        return Err("frame contains a non-finite pose, camera or quality value".to_owned());
    }
    if frame.acquired_at_s < 0.0
        || frame.half_width_u_m <= 0.0
        || frame.half_height_v_m <= 0.0
        || !finite_nonnegative(frame.footprint_uncertainty_m)
        || !finite_nonnegative(frame.pose_uncertainty_m)
        || frame.camera_height_m < 0.0
        || !finite_nonnegative(frame.height_uncertainty_m)
        || frame.incidence_angle_rad < 0.0
        || !finite_nonnegative(frame.ground_speed_m_per_s)
        || !finite_nonnegative(frame.ground_speed_uncertainty_m_per_s)
        || frame.rgb.acquired_at_s < 0.0
        || frame.rgb.ground_sample_distance_m_per_pixel <= 0.0
        || !finite_nonnegative(frame.rgb.gsd_uncertainty_m_per_pixel)
        || frame.rgb.exposure_time_s <= 0.0
        || !fraction(frame.rgb.saturation_fraction)
        || frame.rgb.sharpness_score < 0.0
    {
        return Err("frame contains invalid geometry, time, exposure or uncertainty".to_owned());
    }
    Ok(())
}

fn validate_ir_identity(profile: &SurveyProfile, ir: &IrImage) -> Result<(), String> {
    let expected = &profile.identity;
    if !nonempty(&ir.frame_ref) {
        return Err("radiometric image reference is missing".to_owned());
    }
    if ir.camera_id != expected.ir_camera_id
        || ir.camera_revision != expected.ir_camera_revision
        || ir.calibration_id != expected.ir_calibration_id
        || ir.calibration_revision != expected.ir_calibration_revision
        || ir.calibration_digest != expected.ir_calibration_digest
        || ir.nuc_revision != expected.ir_nuc_revision
    {
        return Err(
            "IR camera/calibration/NUC identity or revision does not match profile".to_owned(),
        );
    }
    Ok(())
}

fn validate_ir_values(ir: &IrImage) -> Result<(), String> {
    let values = [
        ir.acquired_at_s,
        ir.ground_sample_distance_m_per_pixel,
        ir.gsd_uncertainty_m_per_pixel,
        ir.detector_integration_time_s,
        ir.irradiance_w_per_m2,
        ir.ambient_temperature_c,
        ir.relative_humidity_fraction,
        ir.wind_speed_m_per_s,
        ir.wind_gust_m_per_s,
        ir.cloud_variability_fraction,
        ir.emissivity,
        ir.saturation_fraction,
        ir.glare_fraction,
    ];
    if !values.iter().all(|value| value.is_finite())
        || ir
            .reflected_apparent_temperature_c
            .is_some_and(|value| !value.is_finite())
    {
        return Err("radiometric image contains a non-finite value".to_owned());
    }
    if ir.acquired_at_s < 0.0
        || ir.ground_sample_distance_m_per_pixel <= 0.0
        || !finite_nonnegative(ir.gsd_uncertainty_m_per_pixel)
        || ir.detector_integration_time_s <= 0.0
        || ir.irradiance_w_per_m2 < 0.0
        || !fraction(ir.relative_humidity_fraction)
        || ir.wind_speed_m_per_s < 0.0
        || ir.wind_gust_m_per_s < 0.0
        || !fraction(ir.cloud_variability_fraction)
        || ir.emissivity <= 0.0
        || ir.emissivity > 1.0
        || !fraction(ir.saturation_fraction)
        || !fraction(ir.glare_fraction)
    {
        return Err("radiometric image contains invalid ranges or fractions".to_owned());
    }
    Ok(())
}

fn image_blur_upper_bound_px(
    ground_speed_m_per_s: f64,
    speed_uncertainty_m_per_s: f64,
    integration_time_s: f64,
    gsd_m_per_pixel: f64,
    gsd_uncertainty_m_per_pixel: f64,
) -> Option<f64> {
    let speed_upper = ground_speed_m_per_s + speed_uncertainty_m_per_s;
    let gsd_lower = gsd_m_per_pixel - gsd_uncertainty_m_per_pixel;
    if !speed_upper.is_finite() || !gsd_lower.is_finite() || gsd_lower <= 0.0 {
        return None;
    }
    let blur = speed_upper * integration_time_s / gsd_lower;
    blur.is_finite().then_some(blur)
}

fn overlap_fraction(previous: &FlightFrame, current: &FlightFrame) -> f64 {
    let Some((previous_u_min, previous_u_max, previous_v_min, previous_v_max)) =
        conservative_footprint(previous)
    else {
        return 0.0;
    };
    let Some((current_u_min, current_u_max, current_v_min, current_v_max)) =
        conservative_footprint(current)
    else {
        return 0.0;
    };
    let (left_min, left_max, right_min, right_max) = match current.sweep_axis {
        SweepAxis::U => (previous_u_min, previous_u_max, current_u_min, current_u_max),
        SweepAxis::V => (previous_v_min, previous_v_max, current_v_min, current_v_max),
    };
    let overlap = (left_max.min(right_max) - left_min.max(right_min)).max(0.0);
    let left_width = left_max - left_min;
    let right_width = right_max - right_min;
    overlap / left_width.min(right_width)
}

fn conservative_footprint(frame: &FlightFrame) -> Option<(f64, f64, f64, f64)> {
    let shrink = frame.pose_uncertainty_m + frame.footprint_uncertainty_m;
    if !shrink.is_finite() {
        return None;
    }
    let half_u = frame.half_width_u_m - shrink;
    let half_v = frame.half_height_v_m - shrink;
    if half_u <= 0.0 || half_v <= 0.0 {
        return None;
    }
    Some((
        frame.center_u_m - half_u,
        frame.center_u_m + half_u,
        frame.center_v_m - half_v,
        frame.center_v_m + half_v,
    ))
}

fn frame_covers_module(frame: &FlightFrame, module: &ModuleBounds) -> bool {
    let Some((u_min, u_max, v_min, v_max)) = conservative_footprint(frame) else {
        return false;
    };
    module.u_min_m >= u_min
        && module.u_max_m <= u_max
        && module.v_min_m >= v_min
        && module.v_max_m <= v_max
}

fn radiometric_disposition(
    profile: &SurveyProfile,
    frame: &FlightFrame,
    blur_upper_bound_px: Option<f64>,
) -> (RadiometricFrameDisposition, Option<String>) {
    let Some(ir) = &frame.ir else {
        return (
            RadiometricFrameDisposition::Unknown,
            Some("matched radiometric IR frame is absent".to_owned()),
        );
    };
    if let Err(error) = validate_ir_identity(profile, ir) {
        return (RadiometricFrameDisposition::InvalidEvidence, Some(error));
    }
    if let Err(error) = validate_ir_values(ir) {
        return (RadiometricFrameDisposition::InvalidEvidence, Some(error));
    }
    if (frame.rgb.acquired_at_s - ir.acquired_at_s).abs() > profile.maximum_rgb_ir_skew_s {
        return (
            RadiometricFrameDisposition::NotEligible,
            Some("RGB/IR acquisition skew exceeds pairing tolerance".to_owned()),
        );
    }
    if !ir.nuc_current {
        return (
            RadiometricFrameDisposition::NotEligible,
            Some("IR non-uniformity correction is not current".to_owned()),
        );
    }
    if ir.ground_sample_distance_m_per_pixel + ir.gsd_uncertainty_m_per_pixel
        > profile.max_ir_gsd_m_per_pixel
    {
        return (
            RadiometricFrameDisposition::NotEligible,
            Some("IR ground sample distance is too coarse".to_owned()),
        );
    }
    if blur_upper_bound_px.is_none_or(|blur| blur > profile.max_motion_blur_pixels) {
        return (
            RadiometricFrameDisposition::NotEligible,
            Some("IR motion-blur upper bound exceeds profile".to_owned()),
        );
    }
    if ir.radiometric_qc != RadiometricQc::Pass {
        return (
            RadiometricFrameDisposition::NotEligible,
            Some(format!(
                "IR radiometric QC disposition is {:?}",
                ir.radiometric_qc
            )),
        );
    }
    if ir.irradiance_w_per_m2 < profile.minimum_irradiance_w_per_m2 {
        return (
            RadiometricFrameDisposition::NotEligible,
            Some("irradiance is below the simplified-survey profile".to_owned()),
        );
    }
    if ir.wind_speed_m_per_s > profile.maximum_wind_speed_m_per_s
        || ir.wind_gust_m_per_s > profile.maximum_wind_gust_m_per_s
    {
        return (
            RadiometricFrameDisposition::NotEligible,
            Some("wind speed or gust exceeds the radiometric profile".to_owned()),
        );
    }
    if ir.cloud_variability_fraction > profile.maximum_cloud_variability_fraction {
        return (
            RadiometricFrameDisposition::NotEligible,
            Some("cloud/irradiance variability exceeds the profile".to_owned()),
        );
    }
    if ir.emissivity < profile.minimum_emissivity || ir.emissivity > profile.maximum_emissivity {
        return (
            RadiometricFrameDisposition::NotEligible,
            Some("emissivity assumption is outside the declared profile".to_owned()),
        );
    }
    if ir.saturation_fraction > profile.maximum_ir_saturation_fraction
        || ir.glare_fraction > profile.maximum_glare_fraction
    {
        return (
            RadiometricFrameDisposition::NotEligible,
            Some("IR saturation or glare fraction exceeds profile".to_owned()),
        );
    }
    if ir.reflected_apparent_temperature_c.is_none() && !ir.reflection_mitigated {
        return (
            RadiometricFrameDisposition::NotEligible,
            Some(
                "reflected apparent temperature is unavailable and reflection is not mitigated"
                    .to_owned(),
            ),
        );
    }
    (
        RadiometricFrameDisposition::EligibleForSimplifiedSurvey,
        None,
    )
}

fn separated_view_count(views: &BTreeMap<String, f64>, minimum_separation: f64) -> usize {
    let mut selected_angles = Vec::new();
    for angle in views.values() {
        if selected_angles
            .iter()
            .all(|selected| angular_separation(*selected, *angle) >= minimum_separation)
        {
            selected_angles.push(*angle);
        }
    }
    selected_angles.len()
}

fn angular_separation(left: f64, right: f64) -> f64 {
    let difference = (left - right).abs().rem_euclid(std::f64::consts::TAU);
    difference.min(std::f64::consts::TAU - difference)
}

fn validate_camera_model(model: &CameraSensorModel) -> Result<(), String> {
    if !model.horizontal_fov_rad.is_finite()
        || model.horizontal_fov_rad <= 0.0
        || model.horizontal_fov_rad >= std::f64::consts::PI
        || !model.vertical_fov_rad.is_finite()
        || model.vertical_fov_rad <= 0.0
        || model.vertical_fov_rad >= std::f64::consts::PI
        || model.horizontal_pixels == 0
        || model.vertical_pixels == 0
        || !model.maximum_gsd_m_per_pixel.is_finite()
        || model.maximum_gsd_m_per_pixel <= 0.0
        || !finite_nonnegative(model.gsd_uncertainty_m_per_pixel)
        || model.gsd_uncertainty_m_per_pixel >= model.maximum_gsd_m_per_pixel
        || !model.integration_time_s.is_finite()
        || model.integration_time_s <= 0.0
    {
        return Err("camera model contains invalid FOV, resolution, GSD or integration time".to_owned());
    }
    Ok(())
}

fn gsd_at_height(height_m: f64, fov_rad: f64, pixels: u32) -> f64 {
    2.0 * height_m * (fov_rad * 0.5).tan() / f64::from(pixels)
}

fn gsd_height_limit(model: &CameraSensorModel) -> f64 {
    let allowed_gsd = model.maximum_gsd_m_per_pixel - model.gsd_uncertainty_m_per_pixel;
    let horizontal_height =
        allowed_gsd * f64::from(model.horizontal_pixels) / (2.0 * (model.horizontal_fov_rad * 0.5).tan());
    let vertical_height =
        allowed_gsd * f64::from(model.vertical_pixels) / (2.0 * (model.vertical_fov_rad * 0.5).tan());
    horizontal_height.min(vertical_height)
}

fn motion_blur_speed_limit(
    model: &CameraSensorModel,
    horizontal_gsd: f64,
    vertical_gsd: f64,
    maximum_blur_pixels: f64,
) -> Option<f64> {
    let conservative_gsd = horizontal_gsd
        .min(vertical_gsd)
        - model.gsd_uncertainty_m_per_pixel;
    if !conservative_gsd.is_finite() || conservative_gsd <= 0.0 {
        return None;
    }
    let speed = maximum_blur_pixels * conservative_gsd / model.integration_time_s;
    speed.is_finite().then_some(speed)
}

/// Choose a common RGB/IR altitude and speed envelope from camera acquisition
/// constraints. Assumes the camera pair is co-mounted, boresight-aligned and
/// uses the same ground-speed vector; actual calibration/pose validation stays
/// with the sensor and physical-evidence owners.
pub fn compute_camera_flight_envelope(
    spec: &CameraFlightEnvelopeSpec,
) -> Result<CameraFlightEnvelope, String> {
    validate_camera_model(&spec.rgb)?;
    validate_camera_model(&spec.ir)?;
    if !spec.requested_height_m.is_finite()
        || !spec.minimum_height_m.is_finite()
        || !spec.maximum_height_m.is_finite()
        || spec.minimum_height_m <= 0.0
        || spec.maximum_height_m < spec.minimum_height_m
        || !spec.maximum_motion_blur_pixels.is_finite()
        || spec.maximum_motion_blur_pixels <= 0.0
        || !spec.requested_maximum_ground_speed_m_per_s.is_finite()
        || spec.requested_maximum_ground_speed_m_per_s <= 0.0
        || !finite_nonnegative(spec.ground_speed_uncertainty_m_per_s)
    {
        return Err("camera flight envelope contains invalid altitude, speed or blur limits".to_owned());
    }

    let gsd_limit_height_m = gsd_height_limit(&spec.rgb)
        .min(gsd_height_limit(&spec.ir))
        .min(spec.maximum_height_m);
    if !gsd_limit_height_m.is_finite() || gsd_limit_height_m < spec.minimum_height_m {
        return Err("no altitude satisfies both RGB and IR GSD limits inside the allowed height range".to_owned());
    }
    let selected_height_m = spec
        .requested_height_m
        .clamp(spec.minimum_height_m, spec.maximum_height_m)
        .min(gsd_limit_height_m);

    let rgb_gsd_horizontal = gsd_at_height(
        selected_height_m,
        spec.rgb.horizontal_fov_rad,
        spec.rgb.horizontal_pixels,
    );
    let rgb_gsd_vertical = gsd_at_height(
        selected_height_m,
        spec.rgb.vertical_fov_rad,
        spec.rgb.vertical_pixels,
    );
    let ir_gsd_horizontal = gsd_at_height(
        selected_height_m,
        spec.ir.horizontal_fov_rad,
        spec.ir.horizontal_pixels,
    );
    let ir_gsd_vertical = gsd_at_height(
        selected_height_m,
        spec.ir.vertical_fov_rad,
        spec.ir.vertical_pixels,
    );

    let rgb_horizontal_upper = rgb_gsd_horizontal + spec.rgb.gsd_uncertainty_m_per_pixel;
    let rgb_vertical_upper = rgb_gsd_vertical + spec.rgb.gsd_uncertainty_m_per_pixel;
    let ir_horizontal_upper = ir_gsd_horizontal + spec.ir.gsd_uncertainty_m_per_pixel;
    let ir_vertical_upper = ir_gsd_vertical + spec.ir.gsd_uncertainty_m_per_pixel;
    if rgb_horizontal_upper > spec.rgb.maximum_gsd_m_per_pixel
        || rgb_vertical_upper > spec.rgb.maximum_gsd_m_per_pixel
        || ir_horizontal_upper > spec.ir.maximum_gsd_m_per_pixel
        || ir_vertical_upper > spec.ir.maximum_gsd_m_per_pixel
    {
        return Err("selected altitude violates a conservative RGB or IR GSD limit".to_owned());
    }

    let rgb_speed_limit = motion_blur_speed_limit(
        &spec.rgb,
        rgb_gsd_horizontal,
        rgb_gsd_vertical,
        spec.maximum_motion_blur_pixels,
    )
    .ok_or_else(|| "RGB GSD uncertainty leaves no positive motion-blur speed envelope".to_owned())?;
    let ir_speed_limit = motion_blur_speed_limit(
        &spec.ir,
        ir_gsd_horizontal,
        ir_gsd_vertical,
        spec.maximum_motion_blur_pixels,
    )
    .ok_or_else(|| "IR GSD uncertainty leaves no positive motion-blur speed envelope".to_owned())?;
    let maximum_commanded_speed = rgb_speed_limit
        .min(ir_speed_limit)
        .min(spec.requested_maximum_ground_speed_m_per_s)
        - spec.ground_speed_uncertainty_m_per_s;
    if !maximum_commanded_speed.is_finite() || maximum_commanded_speed <= 0.0 {
        return Err("speed uncertainty consumes the entire camera blur-limited speed envelope".to_owned());
    }

    let rgb_half_width = selected_height_m * (spec.rgb.horizontal_fov_rad * 0.5).tan();
    let rgb_half_height = selected_height_m * (spec.rgb.vertical_fov_rad * 0.5).tan();
    let ir_half_width = selected_height_m * (spec.ir.horizontal_fov_rad * 0.5).tan();
    let ir_half_height = selected_height_m * (spec.ir.vertical_fov_rad * 0.5).tan();

    Ok(CameraFlightEnvelope {
        selected_height_m,
        gsd_limit_height_m,
        rgb_gsd_horizontal_m_per_pixel: rgb_gsd_horizontal,
        rgb_gsd_vertical_m_per_pixel: rgb_gsd_vertical,
        ir_gsd_horizontal_m_per_pixel: ir_gsd_horizontal,
        ir_gsd_vertical_m_per_pixel: ir_gsd_vertical,
        common_footprint_half_width_m: rgb_half_width.min(ir_half_width),
        common_footprint_half_height_m: rgb_half_height.min(ir_half_height),
        rgb_motion_blur_speed_limit_m_per_s: rgb_speed_limit,
        ir_motion_blur_speed_limit_m_per_s: ir_speed_limit,
        maximum_commanded_speed_m_per_s: maximum_commanded_speed,
    })
}

fn axis_centers(
    min: f64,
    max: f64,
    footprint_half_extent: f64,
    overlap_fraction: f64,
) -> Result<Vec<f64>, String> {
    let extent = max - min;
    let footprint = footprint_half_extent * 2.0;
    if !extent.is_finite() || extent <= 0.0 || !footprint.is_finite() || footprint <= 0.0 {
        return Err("raster axis or footprint extent is invalid".to_owned());
    }
    if footprint >= extent {
        return Ok(vec![(min + max) * 0.5]);
    }

    let spacing = footprint * (1.0 - overlap_fraction);
    let start = min + footprint_half_extent;
    let end = max - footprint_half_extent;
    let intervals = ((end - start) / spacing).ceil();
    if !intervals.is_finite() || intervals > MAX_MODULES as f64 {
        return Err("raster axis requires too many waypoints".to_owned());
    }
    let count = intervals as usize + 1;
    if count > MAX_MODULES {
        return Err("raster axis exceeds the waypoint limit".to_owned());
    }

    let mut centers = Vec::with_capacity(count);
    for index in 0..count {
        let value = (start + index as f64 * spacing).min(end);
        if centers
            .last()
            .is_none_or(|previous: &f64| (value - previous).abs() > 1.0e-9)
        {
            centers.push(value);
        }
        if value >= end {
            break;
        }
    }
    if centers
        .last()
        .is_none_or(|last| (end - last).abs() > 1.0e-9)
    {
        centers.push(end);
    }
    Ok(centers)
}

fn planned_footprint_covers(
    center_u: f64,
    center_v: f64,
    half_width: f64,
    half_height: f64,
    module: &ModuleBounds,
) -> bool {
    module.u_min_m >= center_u - half_width
        && module.u_max_m <= center_u + half_width
        && module.v_min_m >= center_v - half_height
        && module.v_max_m <= center_v + half_height
}

/// Generate a deterministic boustrophedon/lawnmower route over the module-layout bounds.
///
/// This is a geometric P1 route baseline, not a dynamics-aware trajectory or
/// flight-control command. Route length omits turn radius, acceleration,
/// wind compensation, obstacles and vehicle-specific feasibility.
pub fn generate_raster_plan(
    profile: &SurveyProfile,
    spec: &RasterPlanSpec,
) -> Result<RasterMissionPlan, String> {
    validate_profile(profile)?;
    if !spec.footprint_half_width_m.is_finite()
        || spec.footprint_half_width_m <= 0.0
        || !spec.footprint_half_height_m.is_finite()
        || spec.footprint_half_height_m <= 0.0
        || !fraction(spec.forward_overlap_fraction)
        || spec.forward_overlap_fraction >= 1.0
        || !fraction(spec.side_overlap_fraction)
        || spec.side_overlap_fraction >= 1.0
        || !spec.camera_height_m.is_finite()
        || spec.camera_height_m <= 0.0
        || !spec.view_angle_rad.is_finite()
        || spec.view_angle_rad < -std::f64::consts::PI
        || spec.view_angle_rad > std::f64::consts::PI
    {
        return Err(
            "raster planning profile contains invalid footprint, overlap, height or view angle"
                .to_owned(),
        );
    }

    let min_u = profile
        .modules
        .iter()
        .map(|module| module.u_min_m)
        .fold(f64::INFINITY, f64::min);
    let max_u = profile
        .modules
        .iter()
        .map(|module| module.u_max_m)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_v = profile
        .modules
        .iter()
        .map(|module| module.v_min_m)
        .fold(f64::INFINITY, f64::min);
    let max_v = profile
        .modules
        .iter()
        .map(|module| module.v_max_m)
        .fold(f64::NEG_INFINITY, f64::max);

    let u_centers = axis_centers(
        min_u,
        max_u,
        spec.footprint_half_width_m,
        spec.forward_overlap_fraction,
    )?;
    let v_centers = axis_centers(
        min_v,
        max_v,
        spec.footprint_half_height_m,
        spec.side_overlap_fraction,
    )?;
    let total_points = u_centers
        .len()
        .checked_mul(v_centers.len())
        .ok_or_else(|| "raster waypoint count overflowed".to_owned())?;
    if total_points > MAX_MODULES {
        return Err("raster mission exceeds the waypoint limit".to_owned());
    }

    let mut waypoints = Vec::with_capacity(total_points);
    let mut previous_position: Option<(f64, f64, f64)> = None;
    let mut route_length = 0.0;

    for (lane_index, center_v) in v_centers.iter().copied().enumerate() {
        let mut lane_points = u_centers.clone();
        if lane_index % 2 == 1 {
            lane_points.reverse();
        }
        let pass_id = format!("raster-lane-{lane_index:04}");
        for (point_index, center_u) in lane_points.iter().copied().enumerate() {
            let kind = if lane_index > 0 && point_index == 0 {
                WaypointKind::LaneTransition
            } else {
                WaypointKind::Survey
            };
            let current = (center_u, center_v, spec.camera_height_m);
            if let Some(previous) = previous_position {
                let distance = (current.0 - previous.0)
                    .hypot(current.1 - previous.1)
                    .hypot(current.2 - previous.2);
                route_length += distance;
            }
            previous_position = Some(current);
            let covered_module_ids = profile
                .modules
                .iter()
                .filter(|module| {
                    planned_footprint_covers(
                        center_u,
                        center_v,
                        spec.footprint_half_width_m,
                        spec.footprint_half_height_m,
                        module,
                    )
                })
                .map(|module| module.module_id.clone())
                .collect();

            waypoints.push(RasterWaypoint {
                ordinal: waypoints.len(),
                lane_index,
                point_index,
                pass_id: pass_id.clone(),
                kind,
                center_u_m: center_u,
                center_v_m: center_v,
                camera_height_m: spec.camera_height_m,
                view_angle_rad: spec.view_angle_rad,
                covered_module_ids,
            });
        }
    }

    Ok(RasterMissionPlan {
        waypoints,
        route_length_lower_bound_m: route_length,
        forward_spacing_m: spec.footprint_half_width_m
            * 2.0
            * (1.0 - spec.forward_overlap_fraction),
        side_spacing_m: spec.footprint_half_height_m * 2.0 * (1.0 - spec.side_overlap_fraction),
        requested_forward_overlap_fraction: spec.forward_overlap_fraction,
        requested_side_overlap_fraction: spec.side_overlap_fraction,
    })
}

/// Evaluate a module-level aerial PV survey. The output qualifies acquisition
/// evidence only; it cannot diagnose a fault or establish module health.
pub fn evaluate_survey(profile: &SurveyProfile, observations: &[FlightFrame]) -> SurveyReport {
    if let Err(error) = validate_profile(profile) {
        return SurveyReport {
            disposition: SurveyDisposition::InvalidEvidence,
            expected_modules: profile.modules.len(),
            geometrically_qualified_modules: 0,
            radiometrically_eligible_modules: 0,
            unobserved_modules: profile.modules.len(),
            rejected_frames: 0,
            invalid_frames: 0,
            invalid_radiometric_frames: 0,
            frames: Vec::new(),
            modules: Vec::new(),
            errors: vec![error],
        };
    }

    let mut errors = Vec::new();
    let mut frame_ids = BTreeSet::new();
    let mut trigger_ids = BTreeSet::new();
    let mut pass_indices = BTreeSet::new();
    let mut ordered: Vec<&FlightFrame> = observations.iter().collect();
    ordered.sort_by(|left, right| {
        left.pass_id
            .cmp(&right.pass_id)
            .then_with(|| left.pass_index.cmp(&right.pass_index))
            .then_with(|| left.frame_id.cmp(&right.frame_id))
    });

    let mut assessments = Vec::<FrameAssessment>::with_capacity(ordered.len());
    let mut geometry_views = BTreeMap::<String, BTreeMap<String, f64>>::new();
    let mut radiometric_views = BTreeMap::<String, BTreeMap<String, f64>>::new();
    let mut radiometric_invalid_modules = BTreeSet::<String>::new();
    let mut valid_frame_by_key = BTreeMap::<(String, u32), &FlightFrame>::new();

    for frame in &ordered {
        if !frame_ids.insert(frame.frame_id.as_str()) {
            errors.push(format!("duplicate frame ID: {}", frame.frame_id));
            continue;
        }
        if !trigger_ids.insert(frame.trigger_id.as_str()) {
            errors.push(format!(
                "duplicate/replayed trigger ID: {}",
                frame.trigger_id
            ));
            continue;
        }
        if !pass_indices.insert((frame.pass_id.as_str(), frame.pass_index)) {
            errors.push(format!(
                "duplicate frame index {} in pass {}",
                frame.pass_index, frame.pass_id
            ));
            continue;
        }

        let identity_result = validate_frame_identity(profile, frame);
        let value_result = validate_frame_values(frame);
        if let Some(error) = identity_result.err().or_else(|| value_result.err()) {
            assessments.push(FrameAssessment {
                frame_id: frame.frame_id.clone(),
                trigger_id: frame.trigger_id.clone(),
                pass_id: frame.pass_id.clone(),
                pass_index: frame.pass_index,
                acquired_at_s: frame.acquired_at_s,
                geometry: GeometryFrameDisposition::InvalidEvidence,
                radiometric: RadiometricFrameDisposition::InvalidEvidence,
                consecutive_overlap_fraction: None,
                rgb_gsd_upper_bound_m_per_pixel: None,
                ir_gsd_upper_bound_m_per_pixel: None,
                predicted_rgb_blur_upper_bound_px: None,
                predicted_ir_blur_upper_bound_px: None,
                covered_modules: Vec::new(),
                geometry_reason: Some(error),
                radiometric_reason: Some("frame identity or values are invalid".to_owned()),
            });
            continue;
        }

        let previous = if frame.pass_index == 0 {
            None
        } else {
            valid_frame_by_key
                .get(&(frame.pass_id.clone(), frame.pass_index - 1))
                .copied()
        };
        let overlap = previous.map(|previous_frame| overlap_fraction(previous_frame, frame));
        let rgb_gsd_upper =
            frame.rgb.ground_sample_distance_m_per_pixel + frame.rgb.gsd_uncertainty_m_per_pixel;
        let rgb_blur = image_blur_upper_bound_px(
            frame.ground_speed_m_per_s,
            frame.ground_speed_uncertainty_m_per_s,
            frame.rgb.exposure_time_s,
            frame.rgb.ground_sample_distance_m_per_pixel,
            frame.rgb.gsd_uncertainty_m_per_pixel,
        );
        let ir_gsd_upper = frame
            .ir
            .as_ref()
            .map(|ir| ir.ground_sample_distance_m_per_pixel + ir.gsd_uncertainty_m_per_pixel);
        let ir_blur = frame.ir.as_ref().and_then(|ir| {
            image_blur_upper_bound_px(
                frame.ground_speed_m_per_s,
                frame.ground_speed_uncertainty_m_per_s,
                ir.detector_integration_time_s,
                ir.ground_sample_distance_m_per_pixel,
                ir.gsd_uncertainty_m_per_pixel,
            )
        });

        let mut geometry_reason = None;
        if (frame.acquired_at_s - frame.rgb.acquired_at_s).abs() > profile.maximum_rgb_ir_skew_s {
            geometry_reason =
                Some("RGB image timestamp differs from frame pose timestamp".to_owned());
        } else if rgb_gsd_upper > profile.max_rgb_gsd_m_per_pixel {
            geometry_reason = Some("RGB ground sample distance is too coarse".to_owned());
        } else if rgb_blur.is_none_or(|blur| blur > profile.max_motion_blur_pixels) {
            geometry_reason = Some("RGB motion-blur upper bound exceeds profile".to_owned());
        } else if frame.rgb.sharpness_score < profile.minimum_rgb_sharpness {
            geometry_reason = Some("RGB sharpness score is below profile".to_owned());
        } else if frame.rgb.saturation_fraction > profile.maximum_rgb_saturation_fraction {
            geometry_reason = Some("RGB saturation fraction exceeds profile".to_owned());
        } else if frame.incidence_angle_rad > profile.maximum_incidence_angle_rad {
            geometry_reason = Some("camera incidence angle exceeds profile".to_owned());
        } else if frame.camera_height_m - frame.height_uncertainty_m < profile.minimum_height_m
            || frame.camera_height_m + frame.height_uncertainty_m > profile.maximum_height_m
        {
            geometry_reason = Some("camera height plus uncertainty is outside profile".to_owned());
        } else if frame.pose_uncertainty_m > profile.maximum_pose_uncertainty_m
            || frame.footprint_uncertainty_m > profile.maximum_footprint_uncertainty_m
        {
            geometry_reason = Some("pose or footprint uncertainty exceeds profile".to_owned());
        } else if frame.pass_index > 0 && previous.is_none() {
            geometry_reason =
                Some("preceding frame is absent; along-track overlap is unknown".to_owned());
        } else if overlap.is_some_and(|value| value < profile.minimum_consecutive_overlap_fraction)
        {
            geometry_reason = Some("consecutive along-track overlap is below profile".to_owned());
        }

        let geometry = if geometry_reason.is_some() {
            GeometryFrameDisposition::QualityRejected
        } else {
            GeometryFrameDisposition::Qualified
        };
        let (radiometric, radiometric_reason) = if geometry != GeometryFrameDisposition::Qualified {
            (
                RadiometricFrameDisposition::NotEligible,
                Some("geometric acquisition gates failed".to_owned()),
            )
        } else {
            radiometric_disposition(profile, frame, ir_blur)
        };

        let mut covered_modules = Vec::new();
        if geometry == GeometryFrameDisposition::Qualified {
            for module in &profile.modules {
                if frame_covers_module(frame, module) {
                    covered_modules.push(module.module_id.clone());
                    geometry_views
                        .entry(module.module_id.clone())
                        .or_default()
                        .entry(frame.pass_id.clone())
                        .or_insert(frame.view_angle_rad);
                    match radiometric {
                        RadiometricFrameDisposition::EligibleForSimplifiedSurvey => {
                            radiometric_views
                                .entry(module.module_id.clone())
                                .or_default()
                                .entry(frame.pass_id.clone())
                                .or_insert(frame.view_angle_rad);
                        }
                        RadiometricFrameDisposition::InvalidEvidence => {
                            radiometric_invalid_modules.insert(module.module_id.clone());
                        }
                        RadiometricFrameDisposition::NotEligible
                        | RadiometricFrameDisposition::Unknown => {}
                    }
                }
            }
        }

        assessments.push(FrameAssessment {
            frame_id: frame.frame_id.clone(),
            trigger_id: frame.trigger_id.clone(),
            pass_id: frame.pass_id.clone(),
            pass_index: frame.pass_index,
            acquired_at_s: frame.acquired_at_s,
            geometry,
            radiometric,
            consecutive_overlap_fraction: overlap,
            rgb_gsd_upper_bound_m_per_pixel: Some(rgb_gsd_upper),
            ir_gsd_upper_bound_m_per_pixel: ir_gsd_upper,
            predicted_rgb_blur_upper_bound_px: rgb_blur,
            predicted_ir_blur_upper_bound_px: ir_blur,
            covered_modules,
            geometry_reason,
            radiometric_reason,
        });
        if geometry == GeometryFrameDisposition::Qualified {
            valid_frame_by_key.insert((frame.pass_id.clone(), frame.pass_index), frame);
        }
    }

    assessments.sort_by(|left, right| {
        left.pass_id
            .cmp(&right.pass_id)
            .then_with(|| left.pass_index.cmp(&right.pass_index))
            .then_with(|| left.frame_id.cmp(&right.frame_id))
    });

    let mut modules = Vec::with_capacity(profile.modules.len());
    let mut geometrically_qualified = 0;
    let mut radiometrically_eligible = 0;
    let mut unobserved = 0;
    for module in &profile.modules {
        let geometric = geometry_views.get(&module.module_id);
        let radiometric = radiometric_views.get(&module.module_id);
        let geometry_passes = geometric.map_or(0, BTreeMap::len);
        let qualified_geometry_passes = geometric.map_or(0, |views| {
            separated_view_count(views, profile.minimum_view_angle_separation_rad)
        });
        let radiometric_passes = radiometric.map_or(0, BTreeMap::len);
        let qualified_radiometric_passes = radiometric.map_or(0, |views| {
            separated_view_count(views, profile.minimum_view_angle_separation_rad)
        });

        let geometry_disposition =
            if qualified_geometry_passes >= profile.minimum_independent_passes {
                geometrically_qualified += 1;
                ModuleGeometryDisposition::Qualified
            } else if geometry_passes == 0 {
                unobserved += 1;
                ModuleGeometryDisposition::Unobserved
            } else {
                ModuleGeometryDisposition::InsufficientIndependentPasses
            };

        let radiometric_disposition = if radiometric_invalid_modules.contains(&module.module_id) {
            ModuleRadiometricDisposition::InvalidEvidence
        } else if qualified_radiometric_passes >= profile.minimum_independent_passes {
            radiometrically_eligible += 1;
            ModuleRadiometricDisposition::EligibleForSimplifiedSurvey
        } else if radiometric_passes == 0 {
            ModuleRadiometricDisposition::Unknown
        } else {
            ModuleRadiometricDisposition::NotEligible
        };

        modules.push(ModuleAssessment {
            module_id: module.module_id.clone(),
            row: module.row,
            column: module.column,
            geometric_passes: geometry_passes,
            qualified_geometric_passes: qualified_geometry_passes,
            radiometric_passes,
            qualified_radiometric_passes,
            geometry: geometry_disposition,
            radiometric: radiometric_disposition,
        });
    }
    modules.sort_by(|left, right| {
        left.row
            .cmp(&right.row)
            .then_with(|| left.column.cmp(&right.column))
            .then_with(|| left.module_id.cmp(&right.module_id))
    });

    let invalid_frames = assessments
        .iter()
        .filter(|frame| frame.geometry == GeometryFrameDisposition::InvalidEvidence)
        .count();
    let invalid_radiometric_frames = assessments
        .iter()
        .filter(|frame| frame.radiometric == RadiometricFrameDisposition::InvalidEvidence)
        .count();
    let rejected_frames = assessments
        .iter()
        .filter(|frame| frame.geometry == GeometryFrameDisposition::QualityRejected)
        .count();
    let incomplete_geometry = modules
        .iter()
        .any(|module| module.geometry != ModuleGeometryDisposition::Qualified);
    let incomplete_radiometry = profile.require_radiometric_ir
        && modules.iter().any(|module| {
            module.radiometric != ModuleRadiometricDisposition::EligibleForSimplifiedSurvey
        });

    let disposition = if !errors.is_empty() {
        SurveyDisposition::Inconsistent
    } else if invalid_frames > 0
        || (profile.require_radiometric_ir && invalid_radiometric_frames > 0)
    {
        SurveyDisposition::InvalidEvidence
    } else if incomplete_geometry || incomplete_radiometry {
        SurveyDisposition::Incomplete
    } else {
        SurveyDisposition::Qualified
    };

    SurveyReport {
        disposition,
        expected_modules: profile.modules.len(),
        geometrically_qualified_modules: geometrically_qualified,
        radiometrically_eligible_modules: radiometrically_eligible,
        unobserved_modules: unobserved,
        rejected_frames,
        invalid_frames,
        invalid_radiometric_frames,
        frames: assessments,
        modules,
        errors,
    }
}

/// Compute a model-supplied return-energy envelope. This is not physical
/// battery estimation or a reachable-return guarantee.
pub fn assess_return_reserve(snapshot: &MissionEnergyEnvelope) -> ReserveAssessment {
    let invalid = if !nonempty(&snapshot.mission_id)
        || !nonempty(&snapshot.model_id)
        || !nonempty(&snapshot.model_revision)
        || !nonempty(&snapshot.evidence_ref)
    {
        Some("mission, energy model or evidence identity is incomplete".to_owned())
    } else if !snapshot.initial_energy_wh.is_finite()
        || snapshot.initial_energy_wh <= 0.0
        || !finite_nonnegative(snapshot.consumed_energy_wh)
        || snapshot.consumed_energy_wh > snapshot.initial_energy_wh
        || !finite_nonnegative(snapshot.return_energy_uncertainty_wh)
        || !snapshot.required_reserve_fraction.is_finite()
        || !(0.0..=1.0).contains(&snapshot.required_reserve_fraction)
        || snapshot
            .estimated_return_energy_wh
            .is_some_and(|value| !finite_nonnegative(value))
    {
        Some("energy envelope has invalid or inconsistent numeric values".to_owned())
    } else {
        None
    };
    if let Some(reason) = invalid {
        return ReserveAssessment {
            disposition: ReserveDisposition::InvalidEvidence,
            remaining_energy_wh: None,
            conservative_return_requirement_wh: None,
            required_reserve_wh: None,
            margin_wh: None,
            reason: Some(reason),
        };
    }

    let remaining = snapshot.initial_energy_wh - snapshot.consumed_energy_wh;
    let reserve = snapshot.initial_energy_wh * snapshot.required_reserve_fraction;
    let Some(estimated_return) = snapshot.estimated_return_energy_wh else {
        return ReserveAssessment {
            disposition: ReserveDisposition::Unknown,
            remaining_energy_wh: Some(remaining),
            conservative_return_requirement_wh: None,
            required_reserve_wh: Some(reserve),
            margin_wh: None,
            reason: Some("return-energy estimate is unavailable".to_owned()),
        };
    };
    let conservative_return = estimated_return + snapshot.return_energy_uncertainty_wh;
    let margin = remaining - conservative_return - reserve;
    if !conservative_return.is_finite() || !reserve.is_finite() || !margin.is_finite() {
        return ReserveAssessment {
            disposition: ReserveDisposition::InvalidEvidence,
            remaining_energy_wh: None,
            conservative_return_requirement_wh: None,
            required_reserve_wh: None,
            margin_wh: None,
            reason: Some("return reserve arithmetic overflowed".to_owned()),
        };
    }
    let disposition = if margin >= 0.0 {
        ReserveDisposition::Sufficient
    } else {
        ReserveDisposition::Insufficient
    };
    ReserveAssessment {
        disposition,
        remaining_energy_wh: Some(remaining),
        conservative_return_requirement_wh: Some(conservative_return),
        required_reserve_wh: Some(reserve),
        margin_wh: Some(margin),
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_RGB: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_IR: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn identity() -> ArrayIdentity {
        ArrayIdentity {
            mission_id: "mission:pv-001".to_owned(),
            array_id: "array:roof-01".to_owned(),
            array_revision: "array-r2".to_owned(),
            plane_frame_id: "array-plane:uv".to_owned(),
            plane_frame_revision: 4,
            aircraft_id: "quadrotor:survey-01".to_owned(),
            aircraft_configuration_revision: "airframe-r1".to_owned(),
            gimbal_id: "gimbal:3-axis".to_owned(),
            gimbal_mount_revision: "mount-r2".to_owned(),
            rgb_camera_id: "camera:rgb-01".to_owned(),
            rgb_camera_revision: "rgb-r3".to_owned(),
            rgb_calibration_id: "cal:rgb-01".to_owned(),
            rgb_calibration_revision: "rgb-cal-r1".to_owned(),
            rgb_calibration_digest: DIGEST_RGB.to_owned(),
            ir_camera_id: "camera:ir-01".to_owned(),
            ir_camera_revision: "ir-r2".to_owned(),
            ir_calibration_id: "cal:ir-01".to_owned(),
            ir_calibration_revision: "ir-cal-r4".to_owned(),
            ir_calibration_digest: DIGEST_IR.to_owned(),
            ir_nuc_revision: "nuc-r5".to_owned(),
        }
    }

    fn profile() -> SurveyProfile {
        SurveyProfile {
            identity: identity(),
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
        }
    }

    fn rgb_image(frame_id: &str, time: f64) -> RgbImage {
        let identity = identity();
        RgbImage {
            frame_ref: format!("rgb:{frame_id}"),
            camera_id: identity.rgb_camera_id,
            camera_revision: identity.rgb_camera_revision,
            calibration_id: identity.rgb_calibration_id,
            calibration_revision: identity.rgb_calibration_revision,
            calibration_digest: identity.rgb_calibration_digest,
            acquired_at_s: time,
            ground_sample_distance_m_per_pixel: 0.005,
            gsd_uncertainty_m_per_pixel: 0.001,
            exposure_time_s: 0.002,
            sharpness_score: 0.9,
            saturation_fraction: 0.001,
        }
    }

    fn ir_image(frame_id: &str, time: f64) -> IrImage {
        let identity = identity();
        IrImage {
            frame_ref: format!("ir:{frame_id}"),
            camera_id: identity.ir_camera_id,
            camera_revision: identity.ir_camera_revision,
            calibration_id: identity.ir_calibration_id,
            calibration_revision: identity.ir_calibration_revision,
            calibration_digest: identity.ir_calibration_digest,
            nuc_revision: identity.ir_nuc_revision,
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
        pass: &str,
        index: u32,
        center_u: f64,
        angle: f64,
        time: f64,
    ) -> FlightFrame {
        let identity = identity();
        FlightFrame {
            frame_id: id.to_owned(),
            trigger_id: format!("trigger:{id}"),
            pass_id: pass.to_owned(),
            pass_index: index,
            sweep_axis: SweepAxis::U,
            evidence_ref: format!("evidence:{id}"),
            mission_id: identity.mission_id,
            array_id: identity.array_id,
            array_revision: identity.array_revision,
            plane_frame_id: identity.plane_frame_id,
            plane_frame_revision: identity.plane_frame_revision,
            aircraft_id: identity.aircraft_id,
            aircraft_configuration_revision: identity.aircraft_configuration_revision,
            gimbal_id: identity.gimbal_id,
            gimbal_mount_revision: identity.gimbal_mount_revision,
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
            view_angle_rad: angle,
            ground_speed_m_per_s: 1.3,
            ground_speed_uncertainty_m_per_s: 0.1,
            rgb: rgb_image(id, time),
            ir: Some(ir_image(id, time + 0.01)),
        }
    }

    fn two_pass_frames() -> Vec<FlightFrame> {
        vec![
            frame("a0", "pass-a", 0, 1.0, 0.0, 1.0),
            frame("b0", "pass-b", 0, 1.0, 0.6, 2.0),
        ]
    }

    fn raster_spec() -> RasterPlanSpec {
        RasterPlanSpec {
            footprint_half_width_m: 0.6,
            footprint_half_height_m: 0.3,
            forward_overlap_fraction: 0.20,
            side_overlap_fraction: 0.25,
            camera_height_m: 20.0,
            view_angle_rad: 0.0,
        }
    }

    #[test]
    fn raster_plan_covers_all_modules_in_two_boustrophedon_lanes() {
        let mut survey_profile = profile();
        let mut spec = raster_spec();
        spec.footprint_half_height_m = 0.6;
        survey_profile.modules.extend([
            ModuleBounds {
                module_id: "module-03".to_owned(),
                row: 1,
                column: 0,
                u_min_m: 0.0,
                u_max_m: 1.0,
                v_min_m: 1.0,
                v_max_m: 2.0,
            },
            ModuleBounds {
                module_id: "module-04".to_owned(),
                row: 1,
                column: 1,
                u_min_m: 1.0,
                u_max_m: 2.0,
                v_min_m: 1.0,
                v_max_m: 2.0,
            },
        ]);
        let plan = generate_raster_plan(&survey_profile, &spec).unwrap();
        assert_eq!(plan.waypoints.len(), 4);
        let covered: BTreeSet<&str> = plan
            .waypoints
            .iter()
            .flat_map(|waypoint| waypoint.covered_module_ids.iter().map(String::as_str))
            .collect();
        assert_eq!(
            covered,
            BTreeSet::from(["module-01", "module-02", "module-03", "module-04"])
        );
        assert_eq!(plan.waypoints[0].lane_index, 0);
        assert_eq!(plan.waypoints[1].lane_index, 0);
        assert_eq!(plan.waypoints[2].lane_index, 1);
        assert_eq!(plan.waypoints[2].kind, WaypointKind::LaneTransition);
        assert!(plan.route_length_lower_bound_m > 0.0);
    }

    #[test]
    fn raster_plan_alternates_direction_to_reduce_lane_transition() {
        let mut survey_profile = profile();
        survey_profile.modules.extend([
            ModuleBounds {
                module_id: "module-03".to_owned(),
                row: 1,
                column: 0,
                u_min_m: 0.0,
                u_max_m: 1.0,
                v_min_m: 1.0,
                v_max_m: 2.0,
            },
            ModuleBounds {
                module_id: "module-04".to_owned(),
                row: 1,
                column: 1,
                u_min_m: 1.0,
                u_max_m: 2.0,
                v_min_m: 1.0,
                v_max_m: 2.0,
            },
        ]);
        let plan = generate_raster_plan(&survey_profile, &raster_spec()).unwrap();
        assert!(plan.waypoints[1].center_u_m > plan.waypoints[0].center_u_m);
        assert!(plan.waypoints[3].center_u_m < plan.waypoints[2].center_u_m);
        assert_eq!(plan.waypoints[1].center_u_m, plan.waypoints[2].center_u_m);
    }

    #[test]
    fn raster_spacing_respects_declared_overlap() {
        let plan = generate_raster_plan(&profile(), &raster_spec()).unwrap();
        assert!((plan.forward_spacing_m - 0.96).abs() < 1e-12);
        assert!((plan.side_spacing_m - 0.45).abs() < 1e-12);
        assert_eq!(plan.requested_forward_overlap_fraction, 0.20);
        assert_eq!(plan.requested_side_overlap_fraction, 0.25);
    }

    #[test]
    fn invalid_full_overlap_or_nonfinite_footprint_is_rejected() {
        let mut spec = raster_spec();
        spec.side_overlap_fraction = 1.0;
        assert!(generate_raster_plan(&profile(), &spec).is_err());
        spec = raster_spec();
        spec.footprint_half_width_m = f64::NAN;
        assert!(generate_raster_plan(&profile(), &spec).is_err());
    }

    #[test]
    fn raster_plan_is_deterministic_and_module_order_invariant() {
        let mut survey_profile = profile();
        survey_profile.modules.extend([
            ModuleBounds {
                module_id: "module-03".to_owned(),
                row: 1,
                column: 0,
                u_min_m: 0.0,
                u_max_m: 1.0,
                v_min_m: 1.0,
                v_max_m: 2.0,
            },
            ModuleBounds {
                module_id: "module-04".to_owned(),
                row: 1,
                column: 1,
                u_min_m: 1.0,
                u_max_m: 2.0,
                v_min_m: 1.0,
                v_max_m: 2.0,
            },
        ]);
        let first = generate_raster_plan(&survey_profile, &raster_spec()).unwrap();
        survey_profile.modules.reverse();
        let second = generate_raster_plan(&survey_profile, &raster_spec()).unwrap();
        assert_eq!(first, second);
    }

    fn energy() -> MissionEnergyEnvelope {
        MissionEnergyEnvelope {
            mission_id: "mission:pv-001".to_owned(),
            model_id: "energy-model:quad-01".to_owned(),
            model_revision: "energy-r1".to_owned(),
            evidence_ref: "synthetic-energy-envelope".to_owned(),
            initial_energy_wh: 100.0,
            consumed_energy_wh: 40.0,
            estimated_return_energy_wh: Some(25.0),
            return_energy_uncertainty_wh: 5.0,
            required_reserve_fraction: 0.10,
        }
    }

    #[test]
    fn two_quality_qualified_passes_survey_every_module() {
        let result = evaluate_survey(&profile(), &two_pass_frames());
        assert_eq!(result.disposition, SurveyDisposition::Qualified);
        assert_eq!(result.expected_modules, 2);
        assert_eq!(result.geometrically_qualified_modules, 2);
        assert_eq!(result.radiometrically_eligible_modules, 2);
    }

    #[test]
    fn waypoint_or_image_without_module_footprint_cannot_qualify_module() {
        let result = evaluate_survey(&profile(), &[]);
        assert_eq!(result.disposition, SurveyDisposition::Incomplete);
        assert_eq!(result.geometrically_qualified_modules, 0);
        assert_eq!(result.unobserved_modules, 2);
    }

    #[test]
    fn partial_module_footprint_does_not_count_as_complete_module() {
        let mut frame = frame("a0", "pass-a", 0, 0.5, 0.0, 1.0);
        frame.half_width_u_m = 0.5;
        let result = evaluate_survey(&profile(), &[frame]);
        assert_eq!(result.geometrically_qualified_modules, 0);
    }

    #[test]
    fn too_coarse_rgb_gsd_rejects_geometric_coverage() {
        let mut frames = two_pass_frames();
        frames[0].rgb.ground_sample_distance_m_per_pixel = 0.02;
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].geometry,
            GeometryFrameDisposition::QualityRejected
        );
    }

    #[test]
    fn motion_blur_is_derived_from_speed_exposure_and_gsd() {
        let mut frames = two_pass_frames();
        frames[0].rgb.exposure_time_s = 0.10;
        frames[0].ground_speed_m_per_s = 2.0;
        let result = evaluate_survey(&profile(), &frames);
        assert!(result.frames[0].predicted_rgb_blur_upper_bound_px.unwrap() > 1.0);
        assert_eq!(
            result.frames[0].geometry,
            GeometryFrameDisposition::QualityRejected
        );
    }

    #[test]
    fn low_irradiance_preserves_geometry_but_rejects_radiometric_eligibility() {
        let mut frames = two_pass_frames();
        frames[0].ir.as_mut().unwrap().irradiance_w_per_m2 = 300.0;
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].geometry,
            GeometryFrameDisposition::Qualified
        );
        assert_eq!(
            result.frames[0].radiometric,
            RadiometricFrameDisposition::NotEligible
        );
        assert_eq!(result.disposition, SurveyDisposition::Incomplete);
    }

    #[test]
    fn stale_nuc_rejects_radiometric_eligibility_not_geometric_coverage() {
        let mut frames = two_pass_frames();
        frames[0].ir.as_mut().unwrap().nuc_current = false;
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].geometry,
            GeometryFrameDisposition::Qualified
        );
        assert_eq!(
            result.frames[0].radiometric,
            RadiometricFrameDisposition::NotEligible
        );
    }

    #[test]
    fn glare_and_radiometric_saturation_reject_ir_eligibility() {
        let mut frames = two_pass_frames();
        frames[0].ir.as_mut().unwrap().glare_fraction = 0.20;
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].radiometric,
            RadiometricFrameDisposition::NotEligible
        );
    }

    #[test]
    fn absent_ir_keeps_geometric_result_but_radiometry_unknown() {
        let mut frames = two_pass_frames();
        frames[0].ir = None;
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].geometry,
            GeometryFrameDisposition::Qualified
        );
        assert_eq!(
            result.frames[0].radiometric,
            RadiometricFrameDisposition::Unknown
        );
    }

    #[test]
    fn rgb_ir_timestamp_skew_rejects_radiometric_pair() {
        let mut frames = two_pass_frames();
        frames[0].ir.as_mut().unwrap().acquired_at_s += 1.0;
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].radiometric,
            RadiometricFrameDisposition::NotEligible
        );
    }

    #[test]
    fn stale_array_or_plane_revision_is_invalid_evidence() {
        let mut frames = two_pass_frames();
        frames[0].array_revision = "old-array".to_owned();
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].geometry,
            GeometryFrameDisposition::InvalidEvidence
        );
        assert_eq!(result.disposition, SurveyDisposition::InvalidEvidence);
    }

    #[test]
    fn stale_rgb_calibration_is_invalid_evidence() {
        let mut frames = two_pass_frames();
        frames[0].rgb.calibration_digest =
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned();
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].geometry,
            GeometryFrameDisposition::InvalidEvidence
        );
    }

    #[test]
    fn stale_ir_calibration_is_invalid_evidence_for_radiometry_only() {
        let mut frames = two_pass_frames();
        frames[0].ir.as_mut().unwrap().calibration_digest =
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned();
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].geometry,
            GeometryFrameDisposition::Qualified
        );
        assert_eq!(
            result.frames[0].radiometric,
            RadiometricFrameDisposition::InvalidEvidence
        );
    }

    #[test]
    fn duplicate_trigger_replay_is_inconsistent() {
        let mut frames = two_pass_frames();
        frames[1].trigger_id = frames[0].trigger_id.clone();
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(result.disposition, SurveyDisposition::Inconsistent);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn duplicate_frame_id_is_inconsistent() {
        let mut frames = two_pass_frames();
        frames[1].frame_id = frames[0].frame_id.clone();
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(result.disposition, SurveyDisposition::Inconsistent);
    }

    #[test]
    fn insufficient_consecutive_overlap_rejects_later_frame() {
        let mut frames = vec![
            frame("a0", "pass-a", 0, 0.8, 0.0, 1.0),
            frame("a1", "pass-a", 1, 2.0, 0.0, 2.0),
        ];
        frames[0].half_width_u_m = 0.6;
        frames[1].half_width_u_m = 0.6;
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[1].geometry,
            GeometryFrameDisposition::QualityRejected
        );
        assert!(result.frames[1].consecutive_overlap_fraction.unwrap() < 0.70);
    }

    #[test]
    fn independent_pass_count_not_frame_count_qualifies_module() {
        let frames = vec![
            frame("a0", "pass-a", 0, 1.0, 0.0, 1.0),
            frame("a1", "pass-a", 1, 1.0, 0.0, 2.0),
        ];
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(result.geometrically_qualified_modules, 0);
        assert!(
            result
                .modules
                .iter()
                .all(|module| module.geometric_passes == 1)
        );
    }

    #[test]
    fn inadequate_view_angle_separation_does_not_qualify_module() {
        let mut frames = two_pass_frames();
        frames[1].view_angle_rad = 0.1;
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(result.geometrically_qualified_modules, 0);
    }

    #[test]
    fn excessive_wind_blocks_radiometric_survey_but_not_rgb_geometry() {
        let mut frames = two_pass_frames();
        frames[0].ir.as_mut().unwrap().wind_gust_m_per_s = 12.0;
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].geometry,
            GeometryFrameDisposition::Qualified
        );
        assert_eq!(
            result.frames[0].radiometric,
            RadiometricFrameDisposition::NotEligible
        );
    }

    #[test]
    fn rgb_frame_pose_timestamp_skew_rejects_geometry() {
        let mut frames = two_pass_frames();
        frames[0].rgb.acquired_at_s += 1.0;
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].geometry,
            GeometryFrameDisposition::QualityRejected
        );
        assert!(
            result.frames[0]
                .geometry_reason
                .as_deref()
                .unwrap()
                .contains("timestamp")
        );
    }

    #[test]
    fn stale_ir_calibration_is_top_level_invalid_when_ir_is_required() {
        let mut frames = two_pass_frames();
        frames[0].ir.as_mut().unwrap().calibration_digest =
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned();
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(result.disposition, SurveyDisposition::InvalidEvidence);
        assert_eq!(result.invalid_radiometric_frames, 1);
        assert_eq!(
            result.modules[0].radiometric,
            ModuleRadiometricDisposition::InvalidEvidence
        );
    }

    #[test]
    fn invalid_ir_calibration_is_not_unknown_or_not_eligible() {
        let mut frames = two_pass_frames();
        frames[0].ir.as_mut().unwrap().calibration_revision = "old-cal".to_owned();
        let result = evaluate_survey(&profile(), &frames);
        assert_eq!(
            result.frames[0].radiometric,
            RadiometricFrameDisposition::InvalidEvidence
        );
        assert_eq!(
            result.modules[0].radiometric,
            ModuleRadiometricDisposition::InvalidEvidence
        );
    }

    #[test]
    fn observation_order_does_not_change_module_results() {
        let frames = two_pass_frames();
        let mut reversed = frames.clone();
        reversed.reverse();
        assert_eq!(
            evaluate_survey(&profile(), &frames),
            evaluate_survey(&profile(), &reversed)
        );
    }

    #[test]
    fn return_reserve_sufficient_with_uncertainty_and_required_margin() {
        let result = assess_return_reserve(&energy());
        assert_eq!(result.disposition, ReserveDisposition::Sufficient);
        assert_eq!(result.margin_wh, Some(20.0));
    }

    #[test]
    fn missing_return_estimate_is_unknown_not_sufficient() {
        let mut snapshot = energy();
        snapshot.estimated_return_energy_wh = None;
        let result = assess_return_reserve(&snapshot);
        assert_eq!(result.disposition, ReserveDisposition::Unknown);
        assert!(result.margin_wh.is_none());
    }

    #[test]
    fn reserve_arithmetic_overflow_is_invalid_evidence() {
        let mut snapshot = energy();
        snapshot.initial_energy_wh = f64::MAX;
        snapshot.consumed_energy_wh = 0.0;
        snapshot.estimated_return_energy_wh = Some(f64::MAX);
        snapshot.return_energy_uncertainty_wh = f64::MAX;
        snapshot.required_reserve_fraction = 0.0;
        let result = assess_return_reserve(&snapshot);
        assert_eq!(result.disposition, ReserveDisposition::InvalidEvidence);
    }
}
