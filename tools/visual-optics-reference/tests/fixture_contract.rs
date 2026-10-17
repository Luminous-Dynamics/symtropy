// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde_json::Value;
use symtropy_visual_optics_reference::{
    CameraPoint3, PinholeIntrinsics, PixelPoint, field_of_view, project_point, ray_through_pixel,
};

const PINHOLE_AXIS_FIXTURE: &str =
    include_str!("../../../docs/visual-technology/fixtures/projector-pinhole-axis-001.json");
const PINHOLE_FOV_FIXTURE: &str =
    include_str!("../../../docs/visual-technology/fixtures/pinhole-fov-centered-1280x720-001.json");
const PINHOLE_RAY_FIXTURE: &str =
    include_str!("../../../docs/visual-technology/fixtures/pinhole-ray-roundtrip-001.json");

fn parse_fixture(text: &str) -> Value {
    serde_json::from_str(text).expect("checked-in visual fixture must be valid JSON")
}

fn value_at<'a>(root: &'a Value, path: &[&str]) -> &'a Value {
    let mut current = root;
    for key in path {
        current = current
            .get(*key)
            .unwrap_or_else(|| panic!("fixture is missing key {key}"));
    }
    current
}

fn number_at(root: &Value, path: &[&str]) -> f64 {
    value_at(root, path)
        .as_f64()
        .unwrap_or_else(|| panic!("fixture value at {path:?} must be a number"))
}

fn array_number_at(root: &Value, path: &[&str], index: usize) -> f64 {
    value_at(root, path)
        .as_array()
        .unwrap_or_else(|| panic!("fixture value at {path:?} must be an array"))
        .get(index)
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("fixture array {path:?} lacks numeric index {index}"))
}

fn intrinsics_from_fixture(fixture: &Value) -> PinholeIntrinsics {
    PinholeIntrinsics {
        fx: number_at(fixture, &["intrinsics", "fx_px"]),
        fy: number_at(fixture, &["intrinsics", "fy_px"]),
        cx: number_at(fixture, &["intrinsics", "cx_px"]),
        cy: number_at(fixture, &["intrinsics", "cy_px"]),
    }
}

fn assert_close(actual: f64, expected: f64, tolerance: f64, label: &str) {
    assert!(
        actual.is_finite() && expected.is_finite() && (actual - expected).abs() <= tolerance,
        "{label}: actual={actual}, expected={expected}, tolerance={tolerance}"
    );
}

fn assert_fixture_is_not_self_qualifying(fixture: &Value, expected_case_id: &str) {
    assert_eq!(
        value_at(fixture, &["case_id"]).as_str(),
        Some(expected_case_id)
    );
    assert_eq!(
        value_at(fixture, &["status"]).as_str(),
        Some("UNQUALIFIED"),
        "fixture metadata must not self-declare qualification"
    );
    assert_eq!(
        value_at(fixture, &["evidence_class"]).as_str(),
        Some("ANALYTIC_REFERENCE")
    );
}

#[test]
fn pinhole_axis_fixture_matches_independent_equation_and_reference() {
    let fixture = parse_fixture(PINHOLE_AXIS_FIXTURE);
    assert_fixture_is_not_self_qualifying(&fixture, "VIS-PROJ-PINHOLE-AXIS-001");

    let intrinsics = intrinsics_from_fixture(&fixture);
    let point = CameraPoint3 {
        x: array_number_at(&fixture, &["input_world_position_m"], 0),
        y: array_number_at(&fixture, &["input_world_position_m"], 1),
        z: array_number_at(&fixture, &["input_world_position_m"], 2),
    };
    let expected_u = array_number_at(&fixture, &["expected_image_position_px"], 0);
    let expected_v = array_number_at(&fixture, &["expected_image_position_px"], 1);
    let tolerance = number_at(&fixture, &["tolerance_px"]);

    // Independent equation oracle, intentionally not calling project_point.
    let oracle_u = intrinsics.fx * point.x / point.z + intrinsics.cx;
    let oracle_v = intrinsics.fy * point.y / point.z + intrinsics.cy;
    assert_close(oracle_u, expected_u, tolerance, "analytic u");
    assert_close(oracle_v, expected_v, tolerance, "analytic v");

    let actual = project_point(intrinsics, point).expect("valid pinhole input should project");
    assert_close(actual.u, expected_u, tolerance, "implementation u");
    assert_close(actual.v, expected_v, tolerance, "implementation v");
}

#[test]
fn centered_field_of_view_fixture_matches_independent_equation_and_reference() {
    let fixture = parse_fixture(PINHOLE_FOV_FIXTURE);
    assert_fixture_is_not_self_qualifying(&fixture, "VIS-FOV-CENTERED-1280x720-001");

    let intrinsics = intrinsics_from_fixture(&fixture);
    let width = u32::try_from(
        value_at(&fixture, &["image_dimensions_px", "width"])
            .as_u64()
            .expect("fixture width must be an unsigned integer"),
    )
    .expect("fixture width must fit u32");
    let height = u32::try_from(
        value_at(&fixture, &["image_dimensions_px", "height"])
            .as_u64()
            .expect("fixture height must be an unsigned integer"),
    )
    .expect("fixture height must fit u32");
    let tolerance = number_at(&fixture, &["tolerance_radians"]);
    let expected_h = number_at(&fixture, &["expected", "horizontal_fov_radians"]);
    let expected_v = number_at(&fixture, &["expected", "vertical_fov_radians"]);
    let expected_h_deg = number_at(&fixture, &["expected", "horizontal_fov_degrees"]);
    let expected_v_deg = number_at(&fixture, &["expected", "vertical_fov_degrees"]);

    // Independent boundary-angle oracle; this catches a centered-only shortcut.
    let left = ((0.0 - intrinsics.cx) / intrinsics.fx).atan();
    let right = ((f64::from(width) - intrinsics.cx) / intrinsics.fx).atan();
    let top = ((0.0 - intrinsics.cy) / intrinsics.fy).atan();
    let bottom = ((f64::from(height) - intrinsics.cy) / intrinsics.fy).atan();
    assert_close(right - left, expected_h, tolerance, "analytic horizontal FOV");
    assert_close(bottom - top, expected_v, tolerance, "analytic vertical FOV");
    assert_close(expected_h.to_degrees(), expected_h_deg, 1e-10, "horizontal degrees");
    assert_close(expected_v.to_degrees(), expected_v_deg, 1e-10, "vertical degrees");

    let actual = field_of_view(intrinsics, width, height)
        .expect("valid intrinsics and dimensions should produce FOV");
    assert_close(
        actual.horizontal_radians,
        expected_h,
        tolerance,
        "implementation horizontal FOV",
    );
    assert_close(
        actual.vertical_radians,
        expected_v,
        tolerance,
        "implementation vertical FOV",
    );
}

#[test]
fn pixel_ray_roundtrip_fixture_matches_independent_equations_and_reference() {
    let fixture = parse_fixture(PINHOLE_RAY_FIXTURE);
    assert_fixture_is_not_self_qualifying(&fixture, "VIS-RAY-ROUNDTRIP-001");

    let intrinsics = intrinsics_from_fixture(&fixture);
    let pixel = PixelPoint {
        u: number_at(&fixture, &["input_pixel", "u"]),
        v: number_at(&fixture, &["input_pixel", "v"]),
    };
    let depth = number_at(&fixture, &["scale_depth"]);
    let tolerance = number_at(&fixture, &["tolerance"]);
    let expected_ray = [
        number_at(&fixture, &["expected_ray", "x"]),
        number_at(&fixture, &["expected_ray", "y"]),
        number_at(&fixture, &["expected_ray", "z"]),
    ];
    let expected_point = [
        number_at(&fixture, &["expected_point_at_depth", "x"]),
        number_at(&fixture, &["expected_point_at_depth", "y"]),
        number_at(&fixture, &["expected_point_at_depth", "z"]),
    ];
    let expected_pixel = [
        number_at(&fixture, &["expected_reprojected_pixel", "u"]),
        number_at(&fixture, &["expected_reprojected_pixel", "v"]),
    ];

    // Independent back-projection oracle, intentionally not calling ray_through_pixel.
    let oracle_ray = [
        (pixel.u - intrinsics.cx) / intrinsics.fx,
        (pixel.v - intrinsics.cy) / intrinsics.fy,
        1.0,
    ];
    for index in 0..3 {
        assert_close(
            oracle_ray[index],
            expected_ray[index],
            tolerance,
            "analytic ray component",
        );
    }

    let actual_ray =
        ray_through_pixel(intrinsics, pixel).expect("finite pixel should back-project");
    assert_close(actual_ray.x, expected_ray[0], tolerance, "implementation ray x");
    assert_close(actual_ray.y, expected_ray[1], tolerance, "implementation ray y");
    assert_close(actual_ray.z, expected_ray[2], tolerance, "implementation ray z");

    let point = CameraPoint3 {
        x: expected_ray[0] * depth,
        y: expected_ray[1] * depth,
        z: expected_ray[2] * depth,
    };
    for (actual, expected) in [
        (point.x, expected_point[0]),
        (point.y, expected_point[1]),
        (point.z, expected_point[2]),
    ] {
        assert_close(actual, expected, tolerance, "analytic depth-scaled point");
    }

    // Independent reprojection oracle, intentionally not calling project_point.
    let oracle_u = intrinsics.fx * point.x / point.z + intrinsics.cx;
    let oracle_v = intrinsics.fy * point.y / point.z + intrinsics.cy;
    assert_close(oracle_u, expected_pixel[0], tolerance, "analytic reprojected u");
    assert_close(oracle_v, expected_pixel[1], tolerance, "analytic reprojected v");

    let actual_pixel =
        project_point(intrinsics, point).expect("depth-scaled ray should project");
    assert_close(
        actual_pixel.u,
        expected_pixel[0],
        tolerance,
        "implementation reprojected u",
    );
    assert_close(
        actual_pixel.v,
        expected_pixel[1],
        tolerance,
        "implementation reprojected v",
    );
}
