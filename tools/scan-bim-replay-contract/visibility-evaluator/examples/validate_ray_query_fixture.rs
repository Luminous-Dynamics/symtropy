// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Validate the shared ray-query known-answer fixture with a small,
//! independent brute-force Möller–Trumbore reference implementation.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process::ExitCode;

type Vec3 = [f64; 3];
type Identity = [String; 2];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    fixture_version: String,
    fixture_id: String,
    coordinate_system: CoordinateSystem,
    query_contract: QueryContract,
    triangles: Vec<Triangle>,
    rays: Vec<Ray>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinateSystem {
    linear_unit: String,
    axis_order: String,
    handedness: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryContract {
    direction_normalization: String,
    intersection_epsilon_m: f64,
    nearest_hit_tie_epsilon_m: f64,
    triangle_sidedness: String,
    self_hit_exclusion: String,
    expected_hit_policy: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Triangle {
    surface_id: String,
    triangle_id: String,
    kind: String,
    a: Vec3,
    b: Vec3,
    c: Vec3,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ray {
    ray_id: String,
    origin: Vec3,
    direction: Vec3,
    tnear_m: f64,
    tfar_m: f64,
    exclude_target: Option<Identity>,
    expected: Expected,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    status: String,
    nearest_distance_m: Option<f64>,
    nearest_hits: Vec<Identity>,
}

#[derive(Debug, Serialize)]
struct FixtureReport<'a> {
    fixture_id: &'a str,
    triangle_count: usize,
    ray_count: usize,
    hit_count: usize,
    miss_count: usize,
    checked_nearest_identity_sets: usize,
    oracle: &'static str,
}

fn sub(left: Vec3, right: Vec3) -> Vec3 {
    [
        left[0] - right[0],
        left[1] - right[1],
        left[2] - right[2],
    ]
}

fn dot(left: Vec3, right: Vec3) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: Vec3, right: Vec3) -> Vec3 {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn norm(value: Vec3) -> f64 {
    dot(value, value).sqrt()
}

fn finite(value: Vec3) -> bool {
    value.iter().all(|coordinate| coordinate.is_finite())
}

fn valid_triangle(triangle: &Triangle) -> bool {
    if triangle.surface_id.trim().is_empty()
        || triangle.triangle_id.trim().is_empty()
        || !matches!(triangle.kind.as_str(), "same_building" | "external" | "unknown")
        || !finite(triangle.a)
        || !finite(triangle.b)
        || !finite(triangle.c)
    {
        return false;
    }
    let twice_area = norm(cross(sub(triangle.b, triangle.a), sub(triangle.c, triangle.a)));
    twice_area.is_finite() && twice_area > 1.0e-15
}

/// Return the ray parameter t for a two-sided triangle hit.
///
/// This is intentionally a small brute-force reference, not a production
/// acceleration structure. The query contract requires unit ray directions,
/// so t is expressed in metres.
fn ray_triangle_distance(origin: Vec3, direction: Vec3, triangle: &Triangle) -> Option<f64> {
    let edge_a = sub(triangle.b, triangle.a);
    let edge_b = sub(triangle.c, triangle.a);
    let p = cross(direction, edge_b);
    let determinant = dot(edge_a, p);
    if !determinant.is_finite() || determinant.abs() <= 1.0e-14 {
        return None;
    }

    let inverse = 1.0 / determinant;
    let offset = sub(origin, triangle.a);
    let u = dot(offset, p) * inverse;
    const BARYCENTRIC_EPSILON: f64 = 1.0e-12;
    if !u.is_finite() || u < -BARYCENTRIC_EPSILON || u > 1.0 + BARYCENTRIC_EPSILON {
        return None;
    }

    let q = cross(offset, edge_a);
    let v = dot(direction, q) * inverse;
    if !v.is_finite()
        || v < -BARYCENTRIC_EPSILON
        || u + v > 1.0 + BARYCENTRIC_EPSILON
    {
        return None;
    }

    let distance = dot(edge_b, q) * inverse;
    (distance.is_finite() && distance >= 0.0).then_some(distance)
}

fn validate_fixture(fixture: &Fixture) -> Result<FixtureReport<'_>, String> {
    if fixture.fixture_version != "0.1" || fixture.fixture_id.trim().is_empty() {
        return Err("fixture version or identity is invalid".to_owned());
    }
    if fixture.coordinate_system.linear_unit != "metre"
        || fixture.coordinate_system.axis_order != "xyz"
        || fixture.coordinate_system.handedness != "right_handed"
    {
        return Err("fixture coordinate semantics are unsupported".to_owned());
    }
    let contract = &fixture.query_contract;
    if contract.direction_normalization != "unit_length_required"
        || contract.triangle_sidedness != "two_sided"
        || contract.self_hit_exclusion != "exact_surface_id_and_triangle_id"
        || contract.expected_hit_policy != "all_hits_within_nearest_hit_tie_epsilon"
        || !contract.intersection_epsilon_m.is_finite()
        || contract.intersection_epsilon_m <= 0.0
        || !contract.nearest_hit_tie_epsilon_m.is_finite()
        || contract.nearest_hit_tie_epsilon_m <= 0.0
    {
        return Err("fixture query contract is unsupported or invalid".to_owned());
    }
    if fixture.triangles.is_empty() || fixture.rays.is_empty() {
        return Err("fixture must contain triangles and rays".to_owned());
    }

    let mut triangle_ids = HashSet::new();
    for triangle in &fixture.triangles {
        if !valid_triangle(triangle) {
            return Err(format!(
                "invalid or degenerate triangle {}/{}",
                triangle.surface_id, triangle.triangle_id
            ));
        }
        if !triangle_ids.insert((triangle.surface_id.as_str(), triangle.triangle_id.as_str())) {
            return Err("fixture contains duplicate triangle identity".to_owned());
        }
    }

    let mut ray_ids = HashSet::new();
    let mut hit_count = 0usize;
    let mut miss_count = 0usize;
    for ray in &fixture.rays {
        if ray.ray_id.trim().is_empty() || !ray_ids.insert(ray.ray_id.as_str()) {
            return Err("fixture contains an empty or duplicate ray_id".to_owned());
        }
        if !finite(ray.origin)
            || !finite(ray.direction)
            || (norm(ray.direction) - 1.0).abs() > 1.0e-10
            || !ray.tnear_m.is_finite()
            || !ray.tfar_m.is_finite()
            || ray.tnear_m < 0.0
            || ray.tfar_m <= ray.tnear_m
        {
            return Err(format!("ray {} has invalid origin/direction/clipping", ray.ray_id));
        }
        if let Some(excluded) = &ray.exclude_target {
            if !triangle_ids.contains(&(excluded[0].as_str(), excluded[1].as_str())) {
                return Err(format!(
                    "ray {} excludes a triangle not present in the fixture",
                    ray.ray_id
                ));
            }
        }

        let mut hits: Vec<(f64, Identity)> = fixture
            .triangles
            .iter()
            .filter(|triangle| {
                ray.exclude_target.as_ref().is_none_or(|excluded| {
                    excluded[0] != triangle.surface_id || excluded[1] != triangle.triangle_id
                })
            })
            .filter_map(|triangle| {
                ray_triangle_distance(ray.origin, ray.direction, triangle).and_then(|distance| {
                    (distance >= ray.tnear_m - contract.intersection_epsilon_m
                        && distance <= ray.tfar_m + contract.intersection_epsilon_m)
                        .then(|| {
                            (
                                distance,
                                [
                                    triangle.surface_id.clone(),
                                    triangle.triangle_id.clone(),
                                ],
                            )
                        })
                })
            })
            .collect();

        if hits.is_empty() {
            miss_count += 1;
            if ray.expected.status != "miss"
                || ray.expected.nearest_distance_m.is_some()
                || !ray.expected.nearest_hits.is_empty()
            {
                return Err(format!("ray {} expected miss but oracle found none", ray.ray_id));
            }
            continue;
        }

        hits.sort_by(|left, right| {
            left.0.total_cmp(&right.0).then_with(|| left.1.cmp(&right.1))
        });
        let nearest_distance = hits[0].0;
        let mut nearest_hits: Vec<Identity> = hits
            .iter()
            .take_while(|(distance, _)| {
                *distance - nearest_distance <= contract.nearest_hit_tie_epsilon_m
            })
            .map(|(_, identity)| identity.clone())
            .collect();
        nearest_hits.sort();

        let expected_distance = ray
            .expected
            .nearest_distance_m
            .ok_or_else(|| format!("ray {} expected hit without a distance", ray.ray_id))?;
        let mut expected_hits = ray.expected.nearest_hits.clone();
        expected_hits.sort();
        if ray.expected.status != "hit"
            || (nearest_distance - expected_distance).abs() > contract.intersection_epsilon_m
            || nearest_hits != expected_hits
        {
            return Err(format!(
                "ray {} mismatch: oracle distance={nearest_distance}, hits={nearest_hits:?}; expected distance={expected_distance}, hits={expected_hits:?}",
                ray.ray_id
            ));
        }
        hit_count += 1;
    }

    Ok(FixtureReport {
        fixture_id: &fixture.fixture_id,
        triangle_count: fixture.triangles.len(),
        ray_count: fixture.rays.len(),
        hit_count,
        miss_count,
        checked_nearest_identity_sets: fixture.rays.len(),
        oracle: "independent_bruteforce_moller_trumbore_f64_v1",
    })
}

fn main() -> ExitCode {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| {
            "research/scan-bim-rayquery-comparator-001/fixtures/ray-query-known-answers-v0-1.json"
                .to_owned()
        });
    let input = match fs::read_to_string(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("ray-query fixture: cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let fixture: Fixture = match serde_json::from_str(&input) {
        Ok(fixture) => fixture,
        Err(error) => {
            eprintln!("ray-query fixture: invalid strict fixture JSON: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report = match validate_fixture(&fixture) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("ray-query fixture: validation failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    match serde_json::to_writer(std::io::stdout().lock(), &report) {
        Ok(()) => {
            println!();
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("ray-query fixture: cannot serialize report: {error}");
            ExitCode::FAILURE
        }
    }
}
