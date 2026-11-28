#![forbid(unsafe_code)]

//! Local geometric observability for a pinhole-camera SE(3) pose linearization.
//!
//! This crate computes a Fisher-information matrix (FIM) from caller-supplied
//! 3D camera-frame landmarks and 2D observations. It is conditional on the
//! supplied correspondences, target geometry, intrinsics and pixel-noise
//! assumptions. It is not a pose estimator, covariance, calibrated probability,
//! source authenticator, or proof of physical truth.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const OBSERVABILITY_SCHEMA_VERSION: u32 = 1;
pub const TWIST_ORDER: &str = "left_camera_frame_scaled_xyz_then_rx_ry_rz";
const MAX_OBSERVATIONS: usize = 100_000;
const MIN_OBSERVATIONS: usize = 3;
const EIGEN_ITERATION_LIMIT: usize = 6 * 6 * 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraIntrinsics {
    pub width: u32,
    pub height: u32,
    pub fx_px: f64,
    pub fy_px: f64,
    pub cx_px: f64,
    pub cy_px: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LandmarkObservation {
    pub landmark_id: String,
    /// Landmark expressed in the camera frame at the declared linearization pose.
    pub point_camera_m: [f64; 3],
    /// Observed image coordinate in pixels.
    pub measured_pixel_uv: [f64; 2],
    /// Isotropic one-sigma keypoint measurement noise in pixels.
    pub sigma_px: f64,
    /// Optional fixed robust weight in [0, 1]; zero excludes the observation.
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservabilityRequest {
    pub schema_version: u32,
    pub episode_id: String,
    pub frame_id: String,
    pub sensor_id: String,
    pub coordinate_frame: String,
    pub transform_direction: String,
    pub twist_order: String,
    pub camera_intrinsics_sha256: String,
    pub target_geometry_sha256: String,
    pub correspondence_ledger_sha256: String,
    /// Translation perturbations are scaled by this characteristic target length
    /// so the six-dimensional local parameter vector is dimensionless:
    /// [dx/L, dy/L, dz/L, d_rx, d_ry, d_rz].
    pub translation_scale_m: f64,
    pub rank_relative_tolerance: f64,
    pub rank_absolute_tolerance: f64,
    pub camera: CameraIntrinsics,
    pub observations: Vec<LandmarkObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservabilityResult {
    pub schema_version: u32,
    pub episode_id: String,
    pub frame_id: String,
    pub sensor_id: String,
    pub coordinate_frame: String,
    pub transform_direction: String,
    pub twist_order: String,
    pub observation_count: usize,
    pub effective_observation_count: usize,
    pub translation_scale_m: f64,
    pub fim_6x6: [[f64; 6]; 6],
    /// Sorted ascending; each eigenvalue uses the declared dimensionless twist scaling.
    pub eigenvalues_ascending: [f64; 6],
    pub numerical_rank: usize,
    pub rank_threshold: f64,
    pub weakest_eigenvalue: f64,
    pub strongest_eigenvalue: f64,
    pub condition_number: Option<f64>,
    pub log_determinant: Option<f64>,
    pub participation_ratio: Option<f64>,
    /// Unit eigenvector in [dx/L,dy/L,dz/L,d_rx,d_ry,d_rz] order.
    pub weakest_mode: [f64; 6],
    pub weakest_mode_translation_norm: f64,
    pub weakest_mode_rotation_norm: f64,
    pub mean_reprojection_error_px: f64,
    pub p95_reprojection_error_px: f64,
    pub max_reprojection_error_px: f64,
    pub physical_truth_claim: bool,
    pub pose_probability_claim: bool,
    pub correspondence_authentication_claim: bool,
}

fn require_nonempty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} must not be empty"))
    } else {
        Ok(())
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn finite(value: f64, field: &str) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("{field} must be finite"))
    }
}

fn validate_request(request: &ObservabilityRequest) -> Result<(), String> {
    if request.schema_version != OBSERVABILITY_SCHEMA_VERSION {
        return Err(format!("unsupported observability schema {}", request.schema_version));
    }
    for (field, value) in [
        ("episode_id", request.episode_id.as_str()),
        ("frame_id", request.frame_id.as_str()),
        ("sensor_id", request.sensor_id.as_str()),
        ("coordinate_frame", request.coordinate_frame.as_str()),
        ("transform_direction", request.transform_direction.as_str()),
    ] {
        require_nonempty(value, field)?;
    }
    if request.twist_order != TWIST_ORDER {
        return Err("unsupported SE(3) twist order / perturbation convention".to_string());
    }
    for (field, digest) in [
        ("camera_intrinsics_sha256", request.camera_intrinsics_sha256.as_str()),
        ("target_geometry_sha256", request.target_geometry_sha256.as_str()),
        ("correspondence_ledger_sha256", request.correspondence_ledger_sha256.as_str()),
    ] {
        if !is_sha256(digest) {
            return Err(format!("{field} must be a lowercase SHA-256 digest"));
        }
    }
    if request.camera.width == 0 || request.camera.height == 0 {
        return Err("camera dimensions must be non-zero".to_string());
    }
    for (name, value) in [
        ("fx_px", request.camera.fx_px),
        ("fy_px", request.camera.fy_px),
        ("cx_px", request.camera.cx_px),
        ("cy_px", request.camera.cy_px),
        ("translation_scale_m", request.translation_scale_m),
        ("rank_relative_tolerance", request.rank_relative_tolerance),
        ("rank_absolute_tolerance", request.rank_absolute_tolerance),
    ] {
        finite(value, name)?;
    }
    if request.camera.fx_px <= 0.0 || request.camera.fy_px <= 0.0 {
        return Err("camera focal lengths must be positive".to_string());
    }
    if request.translation_scale_m <= 0.0 {
        return Err("translation_scale_m must be positive".to_string());
    }
    if !(0.0..=1.0).contains(&request.rank_relative_tolerance)
        || request.rank_relative_tolerance == 0.0
        || request.rank_absolute_tolerance <= 0.0
    {
        return Err("rank tolerances must be positive and relative tolerance must be <= 1".to_string());
    }
    if request.observations.len() < MIN_OBSERVATIONS
        || request.observations.len() > MAX_OBSERVATIONS
    {
        return Err(format!(
            "observation count must be in {MIN_OBSERVATIONS}..={MAX_OBSERVATIONS}"
        ));
    }

    let mut landmark_ids = BTreeSet::new();
    let mut effective = 0usize;
    for observation in &request.observations {
        require_nonempty(&observation.landmark_id, "landmark_id")?;
        if !landmark_ids.insert(&observation.landmark_id) {
            return Err(format!("duplicate landmark_id {}", observation.landmark_id));
        }
        for value in observation.point_camera_m {
            finite(value, "point_camera_m")?;
        }
        for value in observation.measured_pixel_uv {
            finite(value, "measured_pixel_uv")?;
        }
        finite(observation.sigma_px, "sigma_px")?;
        finite(observation.weight, "weight")?;
        if observation.point_camera_m[2] <= 1e-9 {
            return Err(format!(
                "landmark {} has non-positive camera depth",
                observation.landmark_id
            ));
        }
        if observation.sigma_px <= 0.0 {
            return Err(format!("landmark {} sigma_px must be positive", observation.landmark_id));
        }
        if !(0.0..=1.0).contains(&observation.weight) {
            return Err(format!("landmark {} weight must be in [0, 1]", observation.landmark_id));
        }
        effective += usize::from(observation.weight > 0.0);
    }
    if effective < MIN_OBSERVATIONS {
        return Err("at least three positive-weight observations are required".to_string());
    }
    Ok(())
}

fn project(camera: &CameraIntrinsics, point: [f64; 3]) -> [f64; 2] {
    [
        camera.fx_px * point[0] / point[2] + camera.cx_px,
        camera.fy_px * point[1] / point[2] + camera.cy_px,
    ]
}

fn point_jacobian(point: [f64; 3], translation_scale_m: f64) -> [[f64; 6]; 3] {
    let [x, y, z] = point;
    [
        [translation_scale_m, 0.0, 0.0, 0.0, z, -y],
        [0.0, translation_scale_m, 0.0, -z, 0.0, x],
        [0.0, 0.0, translation_scale_m, y, -x, 0.0],
    ]
}

fn projection_jacobian(camera: &CameraIntrinsics, point: [f64; 3]) -> [[f64; 3]; 2] {
    let [x, y, z] = point;
    let z2 = z * z;
    [
        [camera.fx_px / z, 0.0, -camera.fx_px * x / z2],
        [0.0, camera.fy_px / z, -camera.fy_px * y / z2],
    ]
}

fn multiply_2x3_3x6(left: [[f64; 3]; 2], right: [[f64; 6]; 3]) -> [[f64; 6]; 2] {
    let mut result = [[0.0; 6]; 2];
    for row in 0..2 {
        for column in 0..6 {
            for inner in 0..3 {
                result[row][column] += left[row][inner] * right[inner][column];
            }
        }
    }
    result
}

fn add_outer_product(fim: &mut [[f64; 6]; 6], jacobian: [[f64; 6]; 2], weight: f64) {
    for row in 0..6 {
        for column in 0..6 {
            fim[row][column] += weight
                * (jacobian[0][row] * jacobian[0][column]
                    + jacobian[1][row] * jacobian[1][column]);
        }
    }
}

fn symmetric_eigen_6(
    mut matrix: [[f64; 6]; 6],
) -> Result<([f64; 6], [[f64; 6]; 6]), String> {
    let mut vectors = [[0.0; 6]; 6];
    for (index, row) in vectors.iter_mut().enumerate() {
        row[index] = 1.0;
    }

    for _ in 0..EIGEN_ITERATION_LIMIT {
        let mut p = 0usize;
        let mut q = 1usize;
        let mut largest = 0.0f64;
        let mut diagonal_scale = 0.0f64;
        for row in 0..6 {
            diagonal_scale = diagonal_scale.max(matrix[row][row].abs());
            for column in (row + 1)..6 {
                let value = matrix[row][column].abs();
                if value > largest {
                    largest = value;
                    p = row;
                    q = column;
                }
            }
        }
        let tolerance = (diagonal_scale * 1e-14).max(1e-14);
        if largest <= tolerance {
            let mut eigenvalues = [0.0; 6];
            for index in 0..6 {
                eigenvalues[index] = matrix[index][index];
                if !eigenvalues[index].is_finite() {
                    return Err("FIM eigensolver produced non-finite eigenvalue".to_string());
                }
                if eigenvalues[index] < -tolerance {
                    return Err("FIM is not positive semidefinite within tolerance".to_string());
                }
                eigenvalues[index] = eigenvalues[index].max(0.0);
            }
            let mut pairs: Vec<_> = (0..6)
                .map(|index| {
                    (
                        eigenvalues[index],
                        std::array::from_fn::<_, 6, _>(|row| vectors[row][index]),
                    )
                })
                .collect();
            pairs.sort_by(|left, right| left.0.total_cmp(&right.0));
            let sorted_values = std::array::from_fn(|index| pairs[index].0);
            let sorted_vectors = std::array::from_fn(|row| {
                std::array::from_fn(|column| pairs[column].1[row])
            });
            return Ok((sorted_values, sorted_vectors));
        }

        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        let tau = (aqq - app) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let cosine = 1.0 / (1.0 + t * t).sqrt();
        let sine = t * cosine;

        for index in 0..6 {
            if index != p && index != q {
                let aip = matrix[index][p];
                let aiq = matrix[index][q];
                let new_p = cosine * aip - sine * aiq;
                let new_q = sine * aip + cosine * aiq;
                matrix[index][p] = new_p;
                matrix[p][index] = new_p;
                matrix[index][q] = new_q;
                matrix[q][index] = new_q;
            }
        }
        matrix[p][p] = app - t * apq;
        matrix[q][q] = aqq + t * apq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;

        for row in 0..6 {
            let vip = vectors[row][p];
            let viq = vectors[row][q];
            vectors[row][p] = cosine * vip - sine * viq;
            vectors[row][q] = sine * vip + cosine * viq;
        }
    }
    Err("FIM Jacobi eigensolver did not converge within its fixed iteration budget".to_string())
}

fn percentile_95(values: &mut [f64]) -> Result<f64, String> {
    if values.is_empty() {
        return Err("reprojection residual set is empty".to_string());
    }
    values.sort_by(f64::total_cmp);
    let position = 0.95 * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let weight = position - lower as f64;
    Ok(values[lower] * (1.0 - weight) + values[upper] * weight)
}

pub fn evaluate(request: &ObservabilityRequest) -> Result<ObservabilityResult, String> {
    validate_request(request)?;
    let mut fim = [[0.0; 6]; 6];
    let mut residuals = Vec::with_capacity(request.observations.len());
    let mut effective_observation_count = 0usize;

    for observation in &request.observations {
        let predicted = project(&request.camera, observation.point_camera_m);
        let du = predicted[0] - observation.measured_pixel_uv[0];
        let dv = predicted[1] - observation.measured_pixel_uv[1];
        residuals.push((du * du + dv * dv).sqrt());

        if observation.weight == 0.0 {
            continue;
        }
        let point_j = point_jacobian(observation.point_camera_m, request.translation_scale_m);
        let projection_j = projection_jacobian(&request.camera, observation.point_camera_m);
        let jacobian = multiply_2x3_3x6(projection_j, point_j);
        let information_weight = observation.weight / (observation.sigma_px * observation.sigma_px);
        add_outer_product(&mut fim, jacobian, information_weight);
        effective_observation_count += 1;
    }

    if fim.iter().flatten().any(|value| !value.is_finite()) {
        return Err("FIM accumulation produced non-finite values".to_string());
    }
    let (eigenvalues, eigenvectors) = symmetric_eigen_6(fim)?;
    let strongest = eigenvalues[5];
    let weakest = eigenvalues[0];
    let rank_threshold = request
        .rank_absolute_tolerance
        .max(request.rank_relative_tolerance * strongest);
    let numerical_rank = eigenvalues
        .iter()
        .filter(|value| **value > rank_threshold)
        .count();
    let condition_number = if numerical_rank == 6 && weakest > rank_threshold {
        Some(strongest / weakest)
    } else {
        None
    };
    let log_determinant = if numerical_rank == 6 {
        Some(eigenvalues.iter().map(|value| value.ln()).sum())
    } else {
        None
    };
    let trace = eigenvalues.iter().sum::<f64>();
    let square_sum = eigenvalues.iter().map(|value| value * value).sum::<f64>();
    let participation_ratio = (square_sum > 0.0).then_some(trace * trace / square_sum);
    let weakest_mode = std::array::from_fn(|index| eigenvectors[index][0]);
    let translation_norm = weakest_mode[..3].iter().map(|value| value * value).sum::<f64>().sqrt();
    let rotation_norm = weakest_mode[3..].iter().map(|value| value * value).sum::<f64>().sqrt();
    let p95_reprojection_error_px = percentile_95(&mut residuals.clone())?;
    let mean_reprojection_error_px =
        residuals.iter().sum::<f64>() / residuals.len() as f64;
    let max_reprojection_error_px = residuals.iter().copied().fold(0.0_f64, f64::max);

    Ok(ObservabilityResult {
        schema_version: OBSERVABILITY_SCHEMA_VERSION,
        episode_id: request.episode_id.clone(),
        frame_id: request.frame_id.clone(),
        sensor_id: request.sensor_id.clone(),
        coordinate_frame: request.coordinate_frame.clone(),
        transform_direction: request.transform_direction.clone(),
        twist_order: request.twist_order.clone(),
        observation_count: request.observations.len(),
        effective_observation_count,
        translation_scale_m: request.translation_scale_m,
        fim_6x6: fim,
        eigenvalues_ascending: eigenvalues,
        numerical_rank,
        rank_threshold,
        weakest_eigenvalue: weakest,
        strongest_eigenvalue: strongest,
        condition_number,
        log_determinant,
        participation_ratio,
        weakest_mode,
        weakest_mode_translation_norm: translation_norm,
        weakest_mode_rotation_norm: rotation_norm,
        mean_reprojection_error_px,
        p95_reprojection_error_px,
        max_reprojection_error_px,
        physical_truth_claim: false,
        pose_probability_claim: false,
        correspondence_authentication_claim: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(ch: char) -> String {
        ch.to_string().repeat(64)
    }

    fn observation(id: usize, point: [f64; 3], camera: &CameraIntrinsics) -> LandmarkObservation {
        LandmarkObservation {
            landmark_id: format!("landmark-{id}"),
            point_camera_m: point,
            measured_pixel_uv: project(camera, point),
            sigma_px: 0.5,
            weight: 1.0,
        }
    }

    fn camera() -> CameraIntrinsics {
        CameraIntrinsics {
            width: 640,
            height: 480,
            fx_px: 500.0,
            fy_px: 500.0,
            cx_px: 320.0,
            cy_px: 240.0,
        }
    }

    fn request(points: &[[f64; 3]]) -> ObservabilityRequest {
        let camera = camera();
        ObservabilityRequest {
            schema_version: 1,
            episode_id: "episode-1".to_string(),
            frame_id: "frame-1".to_string(),
            sensor_id: "camera-1".to_string(),
            coordinate_frame: "camera_optical".to_string(),
            transform_direction: "target_to_camera".to_string(),
            twist_order: TWIST_ORDER.to_string(),
            camera_intrinsics_sha256: sha('1'),
            target_geometry_sha256: sha('2'),
            correspondence_ledger_sha256: sha('3'),
            translation_scale_m: 1.0,
            rank_relative_tolerance: 1e-9,
            rank_absolute_tolerance: 1e-12,
            camera: camera.clone(),
            observations: points
                .iter()
                .enumerate()
                .map(|(index, point)| observation(index, *point, &camera))
                .collect(),
        }
    }

    #[test]
    fn non_coplanar_diverse_landmarks_produce_full_rank_information() {
        let points = [
            [-1.0, -1.0, 5.0],
            [1.0, -1.0, 5.0],
            [-1.0, 1.0, 5.0],
            [1.0, 1.0, 5.0],
            [-0.8, -0.7, 7.0],
            [0.9, -0.6, 8.0],
            [-0.7, 0.8, 6.0],
            [0.8, 0.9, 7.0],
        ];
        let result = evaluate(&request(&points)).unwrap();
        assert_eq!(result.numerical_rank, 6);
        assert!(result.condition_number.unwrap().is_finite());
        assert!(result.log_determinant.unwrap().is_finite());
        assert!((result.mean_reprojection_error_px).abs() < 1e-12);
        assert_eq!(result.physical_truth_claim, false);
        assert_eq!(result.pose_probability_claim, false);
        assert_eq!(result.correspondence_authentication_claim, false);
    }

    #[test]
    fn repeated_on_axis_points_leave_a_rotational_mode_unobservable() {
        let points = [
            [0.0, 0.0, 5.0],
            [0.0, 0.0, 6.0],
            [0.0, 0.0, 7.0],
            [0.0, 0.0, 8.0],
            [0.0, 0.0, 9.0],
            [0.0, 0.0, 10.0],
        ];
        let result = evaluate(&request(&points)).unwrap();
        assert!(result.numerical_rank < 6);
        assert!(result.condition_number.is_none());
        assert!(result.log_determinant.is_none());
        assert!(result.weakest_mode_rotation_norm > 0.99);
    }

    #[test]
    fn collinear_landmarks_expose_rank_deficiency() {
        let points = [
            [-1.0, 0.0, 5.0],
            [-0.5, 0.0, 5.5],
            [0.0, 0.0, 6.0],
            [0.5, 0.0, 6.5],
            [1.0, 0.0, 7.0],
            [1.5, 0.0, 7.5],
        ];
        let result = evaluate(&request(&points)).unwrap();
        assert!(result.numerical_rank < 6);
    }

    #[test]
    fn a_large_common_correspondence_bias_does_not_turn_fim_into_truth() {
        let points = [
            [-1.0, -1.0, 5.0],
            [1.0, -1.0, 5.0],
            [-1.0, 1.0, 5.0],
            [1.0, 1.0, 5.0],
            [-0.8, -0.7, 7.0],
            [0.9, -0.6, 8.0],
        ];
        let mut input = request(&points);
        for landmark in &mut input.observations {
            landmark.measured_pixel_uv[0] += 80.0;
            landmark.measured_pixel_uv[1] -= 45.0;
        }
        let result = evaluate(&input).unwrap();
        assert_eq!(result.numerical_rank, 6);
        assert!(result.mean_reprojection_error_px > 90.0);
        assert!(!result.correspondence_authentication_claim);
    }

    #[test]
    fn invalid_depth_noise_intrinsics_and_duplicate_ids_reject() {
        let points = [
            [-1.0, -1.0, 5.0],
            [1.0, -1.0, 5.0],
            [-1.0, 1.0, 5.0],
        ];
        let mut input = request(&points);
        input.observations[0].point_camera_m[2] = 0.0;
        assert!(evaluate(&input).is_err());

        let mut input = request(&points);
        input.observations[0].sigma_px = 0.0;
        assert!(evaluate(&input).is_err());

        let mut input = request(&points);
        input.camera.fx_px = f64::NAN;
        assert!(evaluate(&input).is_err());

        let mut input = request(&points);
        input.observations[1].landmark_id = input.observations[0].landmark_id.clone();
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn translation_scale_is_bound_to_the_information_matrix() {
        let points = [
            [-1.0, -1.0, 5.0],
            [1.0, -1.0, 5.0],
            [-1.0, 1.0, 5.0],
            [1.0, 1.0, 5.0],
            [-0.8, -0.7, 7.0],
            [0.9, -0.6, 8.0],
        ];
        let mut first = request(&points);
        let mut second = first.clone();
        second.translation_scale_m = 2.0;
        let a = evaluate(&first).unwrap();
        let b = evaluate(&second).unwrap();
        assert_ne!(a.fim_6x6, b.fim_6x6);
    }

    #[test]
    fn zero_weight_features_do_not_contribute_to_information() {
        let points = [
            [-1.0, -1.0, 5.0],
            [1.0, -1.0, 5.0],
            [-1.0, 1.0, 5.0],
            [1.0, 1.0, 5.0],
            [-0.8, -0.7, 7.0],
            [0.9, -0.6, 8.0],
        ];
        let mut input = request(&points);
        input.observations[0].weight = 0.0;
        let result = evaluate(&input).unwrap();
        assert_eq!(result.effective_observation_count, 5);
    }
}
