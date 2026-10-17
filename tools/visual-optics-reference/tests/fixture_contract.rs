// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde_json::Value;
use symtropy_visual_optics_reference::{
    CameraPoint3, PinholeIntrinsics, PixelPoint, field_of_view, hlg_scene_to_signal,
    hlg_signal_to_scene, linear_channel_to_srgb, nits_to_pq_signal, pq_signal_to_nits,
    project_point, ray_through_pixel, relative_luminance_from_srgb, srgb_channel_to_linear,
};

const PINHOLE_AXIS_FIXTURE: &str =
    include_str!("../../../docs/visual-technology/fixtures/projector-pinhole-axis-001.json");
const PINHOLE_FOV_FIXTURE: &str =
    include_str!("../../../docs/visual-technology/fixtures/pinhole-fov-centered-1280x720-001.json");
const PINHOLE_RAY_FIXTURE: &str =
    include_str!("../../../docs/visual-technology/fixtures/pinhole-ray-roundtrip-001.json");
const SRGB_FIXTURE: &str =
    include_str!("../../../docs/visual-technology/fixtures/srgb-transfer-and-luminance-001.json");
const PQ_FIXTURE: &str =
    include_str!("../../../docs/visual-technology/fixtures/pq-st2084-eotf-oetf-001.json");
const HLG_FIXTURE: &str =
    include_str!("../../../docs/visual-technology/fixtures/hlg-scene-oetf-001.json");

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
    assert_close(
        right - left,
        expected_h,
        tolerance,
        "analytic horizontal FOV",
    );
    assert_close(bottom - top, expected_v, tolerance, "analytic vertical FOV");
    assert_close(
        expected_h.to_degrees(),
        expected_h_deg,
        1e-10,
        "horizontal degrees",
    );
    assert_close(
        expected_v.to_degrees(),
        expected_v_deg,
        1e-10,
        "vertical degrees",
    );

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
fn hlg_fixture_matches_independent_scene_oetf_equations_and_reference() {
    let fixture = parse_fixture(HLG_FIXTURE);
    assert_fixture_is_not_self_qualifying(&fixture, "VIS-HLG-SCENE-OETF-REFERENCES-001");
    assert_eq!(
        value_at(&fixture, &["profile", "name"]).as_str(),
        Some("ITU-R BT.2100-3 Hybrid Log-Gamma scene OETF")
    );
    assert_eq!(
        value_at(&fixture, &["profile", "input_semantics"]).as_str(),
        Some("normalized scene-linear light")
    );
    assert_eq!(
        value_at(&fixture, &["profile", "output_semantics"]).as_str(),
        Some("normalized nonlinear HLG signal")
    );

    let a = number_at(&fixture, &["profile", "constants", "a"]);
    let b = number_at(&fixture, &["profile", "constants", "b"]);
    let c = number_at(&fixture, &["profile", "constants", "c"]);
    let scene_boundary = number_at(&fixture, &["profile", "branch_point_scene"]);
    let signal_boundary = number_at(&fixture, &["profile", "branch_point_signal"]);
    let tolerance = number_at(&fixture, &["tolerance"]);

    for case in value_at(&fixture, &["scene_to_signal_cases"])
        .as_array()
        .expect("HLG scene-to-signal cases must be an array")
    {
        let scene = number_at(case, &["scene_linear"]);
        let expected = number_at(case, &["expected_signal"]);
        let oracle = if scene <= scene_boundary {
            (3.0 * scene).sqrt()
        } else {
            a * (12.0 * scene - b).ln() + c
        };
        assert_close(oracle, expected, tolerance, "independent HLG OETF equation");
        assert_close(
            hlg_scene_to_signal(scene).expect("fixture scene value is in range"),
            expected,
            tolerance,
            "HLG OETF implementation",
        );
    }

    for case in value_at(&fixture, &["signal_to_scene_cases"])
        .as_array()
        .expect("HLG inverse cases must be an array")
    {
        let signal = number_at(case, &["signal"]);
        let expected = number_at(case, &["expected_scene_linear"]);
        let oracle = if signal <= signal_boundary {
            (signal * signal) / 3.0
        } else {
            (((signal - c) / a).exp() + b) / 12.0
        };
        assert_close(
            oracle,
            expected,
            tolerance,
            "independent inverse HLG equation",
        );
        assert_close(
            hlg_signal_to_scene(signal).expect("fixture HLG signal is in range"),
            expected,
            tolerance,
            "HLG inverse implementation",
        );
    }

    for item in value_at(&fixture, &["roundtrip_scene_linear"])
        .as_array()
        .expect("HLG round-trip values must be an array")
    {
        let scene = item.as_f64().expect("HLG round-trip value must be numeric");
        let signal = hlg_scene_to_signal(scene).expect("scene value is in range");
        let reconstructed = hlg_signal_to_scene(signal).expect("HLG encoded signal is in range");
        assert_close(reconstructed, scene, tolerance, "HLG scene OETF round trip");
    }
}

#[test]
fn pq_fixture_matches_independent_st2084_equations_and_reference() {
    let fixture = parse_fixture(PQ_FIXTURE);
    assert_fixture_is_not_self_qualifying(&fixture, "VIS-PQ-ST2084-REFERENCES-001");
    assert_eq!(
        value_at(&fixture, &["profile", "name"]).as_str(),
        Some("SMPTE ST 2084 / ITU-R BT.2100 PQ EOTF")
    );
    let m1 = number_at(&fixture, &["profile", "constants", "m1"]);
    let m2 = number_at(&fixture, &["profile", "constants", "m2"]);
    let c1 = number_at(&fixture, &["profile", "constants", "c1"]);
    let c2 = number_at(&fixture, &["profile", "constants", "c2"]);
    let c3 = number_at(&fixture, &["profile", "constants", "c3"]);
    let max_nits = array_number_at(&fixture, &["profile", "luminance_domain_nits"], 1);
    let signal_tolerance = number_at(&fixture, &["tolerance", "signal"]);
    let nits_tolerance = number_at(&fixture, &["tolerance", "luminance_nits"]);

    for case in value_at(&fixture, &["signal_to_nits_cases"])
        .as_array()
        .expect("PQ decode cases must be an array")
    {
        let signal = number_at(case, &["signal"]);
        let expected = number_at(case, &["expected_nits"]);
        let powered = signal.powf(1.0 / m2);
        let numerator = (powered - c1).max(0.0);
        let denominator = c2 - c3 * powered;
        let oracle = 10_000.0 * (numerator / denominator).powf(1.0 / m1);
        assert_close(
            oracle,
            expected,
            nits_tolerance,
            "independent PQ EOTF equation",
        );
        assert_close(
            pq_signal_to_nits(signal).expect("fixture signal is within PQ range"),
            expected,
            nits_tolerance,
            "PQ EOTF implementation",
        );
    }

    for case in value_at(&fixture, &["nits_to_signal_cases"])
        .as_array()
        .expect("PQ encode cases must be an array")
    {
        let nits = number_at(case, &["nits"]);
        let expected = number_at(case, &["expected_signal"]);
        // Canonical black selects exact zero from the EOTF's clipped black interval.
        let oracle = if nits == 0.0 {
            0.0
        } else {
            let normalized = (nits / max_nits).powf(m1);
            ((c1 + c2 * normalized) / (1.0 + c3 * normalized)).powf(m2)
        };
        assert_close(
            oracle,
            expected,
            signal_tolerance,
            "independent PQ OETF equation",
        );
        assert_close(
            nits_to_pq_signal(nits).expect("fixture luminance is within PQ range"),
            expected,
            signal_tolerance,
            "PQ OETF implementation",
        );
    }

    for item in value_at(&fixture, &["roundtrip_nits"])
        .as_array()
        .expect("PQ round-trip values must be an array")
    {
        let nits = item.as_f64().expect("PQ round-trip value must be numeric");
        let signal = nits_to_pq_signal(nits).expect("round-trip luminance is in range");
        let reconstructed = pq_signal_to_nits(signal).expect("encoded signal is in range");
        assert_close(
            reconstructed,
            nits,
            nits_tolerance,
            "PQ encode/decode round trip",
        );
    }
}

#[test]
fn srgb_fixture_matches_independent_transfer_and_luminance_equations() {
    let fixture = parse_fixture(SRGB_FIXTURE);
    assert_fixture_is_not_self_qualifying(&fixture, "VIS-SRGB-REFERENCES-001");
    assert_eq!(
        value_at(&fixture, &["profile", "name"]).as_str(),
        Some("sRGB")
    );
    assert_eq!(
        value_at(&fixture, &["profile", "channel_domain"])
            .as_array()
            .expect("channel domain must be an array")
            .len(),
        2
    );
    let tolerance = number_at(&fixture, &["tolerance"]);
    let encoded_threshold = number_at(&fixture, &["profile", "transfer_threshold_encoded"]);
    let linear_threshold = number_at(&fixture, &["profile", "transfer_threshold_linear"]);

    for case in value_at(&fixture, &["encoded_to_linear_cases"])
        .as_array()
        .expect("decode cases must be an array")
    {
        let encoded = number_at(case, &["encoded"]);
        let expected = number_at(case, &["expected_linear"]);
        let oracle = if encoded <= encoded_threshold {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        };
        assert_close(
            oracle,
            expected,
            tolerance,
            "independent sRGB decode equation",
        );
        assert_close(
            srgb_channel_to_linear(encoded).expect("fixture channel is in range"),
            expected,
            tolerance,
            "sRGB decode implementation",
        );
    }

    for case in value_at(&fixture, &["linear_to_encoded_cases"])
        .as_array()
        .expect("encode cases must be an array")
    {
        let linear = number_at(case, &["linear"]);
        let expected = number_at(case, &["expected_encoded"]);
        let oracle = if linear <= linear_threshold {
            12.92 * linear
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        assert_close(
            oracle,
            expected,
            tolerance,
            "independent sRGB encode equation",
        );
        assert_close(
            linear_channel_to_srgb(linear).expect("fixture channel is in range"),
            expected,
            tolerance,
            "sRGB encode implementation",
        );
    }

    for case in value_at(&fixture, &["relative_luminance_cases"])
        .as_array()
        .expect("luminance cases must be an array")
    {
        let rgb = value_at(case, &["rgb_srgb"])
            .as_array()
            .expect("RGB must be an array");
        let red = rgb[0].as_f64().expect("red channel must be numeric");
        let green = rgb[1].as_f64().expect("green channel must be numeric");
        let blue = rgb[2].as_f64().expect("blue channel must be numeric");
        let expected = number_at(case, &["expected_y"]);
        let oracle = 0.2126
            * if red <= encoded_threshold {
                red / 12.92
            } else {
                ((red + 0.055) / 1.055).powf(2.4)
            }
            + 0.7152
                * if green <= encoded_threshold {
                    green / 12.92
                } else {
                    ((green + 0.055) / 1.055).powf(2.4)
                }
            + 0.0722
                * if blue <= encoded_threshold {
                    blue / 12.92
                } else {
                    ((blue + 0.055) / 1.055).powf(2.4)
                };
        assert_close(
            oracle,
            expected,
            tolerance,
            "independent relative luminance",
        );
        assert_close(
            relative_luminance_from_srgb(red, green, blue).expect("fixture RGB is in gamut"),
            expected,
            tolerance,
            "relative luminance implementation",
        );
    }
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
    for (actual, expected) in oracle_ray.into_iter().zip(expected_ray) {
        assert_close(actual, expected, tolerance, "analytic ray component");
    }

    let actual_ray =
        ray_through_pixel(intrinsics, pixel).expect("finite pixel should back-project");
    assert_close(
        actual_ray.x,
        expected_ray[0],
        tolerance,
        "implementation ray x",
    );
    assert_close(
        actual_ray.y,
        expected_ray[1],
        tolerance,
        "implementation ray y",
    );
    assert_close(
        actual_ray.z,
        expected_ray[2],
        tolerance,
        "implementation ray z",
    );

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
    assert_close(
        oracle_u,
        expected_pixel[0],
        tolerance,
        "analytic reprojected u",
    );
    assert_close(
        oracle_v,
        expected_pixel[1],
        tolerance,
        "analytic reprojected v",
    );

    let actual_pixel = project_point(intrinsics, point).expect("depth-scaled ray should project");
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
