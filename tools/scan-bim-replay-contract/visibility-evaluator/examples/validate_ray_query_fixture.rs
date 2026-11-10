// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Differentially validate the canonical ray-query fixture against:
//! (1) an independent brute-force f64 Moller-Trumbore implementation; and
//! (2) the evaluator's build-once deterministic median-split BVH.

use scan_bim_replay_contract::metrics::Point3;
use scan_bim_visibility_evaluator::visibility::{
    OccluderKind, OccluderTriangle, RayQueryIndexV1, RayQueryV1,
};
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
    clipping_policy: String,
    clip_endpoint_tolerance_m: f64,
    nearest_hit_tie_epsilon_m: f64,
    determinant_epsilon_m2: f64,
    barycentric_epsilon: f64,
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
    brute_force_checked_nearest_identity_sets: usize,
    bvh_checked_nearest_identity_sets: usize,
    bvh_hit_count: usize,
    bvh_miss_count: usize,
    bvh_node_count: usize,
    bvh_leaf_count: usize,
    bvh_ray_test_count: usize,
    bvh_node_visit_count: usize,
    occluder_manifest_sha256: String,
    brute_force_oracle: &'static str,
    bvh_engine: &'static str,
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
    value[0].hypot(value[1]).hypot(value[2])
}

fn finite(value: Vec3) -> bool {
    value.iter().all(|coordinate| coordinate.is_finite())
}

fn point3(value: Vec3) -> Point3 {
    Point3::new(value[0], value[1], value[2])
}

fn occluder_kind(value: &str) -> Result<OccluderKind, String> {
    match value {
        "same_building" => Ok(OccluderKind::SameBuilding),
        "external" => Ok(OccluderKind::External),
        "unknown" => Ok(OccluderKind::Unknown),
        _ => Err(format!("unsupported occluder kind {value}")),
    }
}

fn validate_triangle(triangle: &Triangle) -> bool {
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

/// Independent brute-force two-sided Moller-Trumbore primitive intersection.
/// It intentionally does not call the evaluator's triangle kernel or BVH.
fn ray_triangle_distance(
    origin: Vec3,
    direction: Vec3,
    triangle: &Triangle,
    determinant_epsilon_m2: f64,
    barycentric_epsilon: f64,
) -> Option<f64> {
    let edge_a = sub(triangle.b, triangle.a);
    let edge_b = sub(triangle.c, triangle.a);
    let p = cross(direction, edge_b);
    let determinant = dot(edge_a, p);
    if !determinant.is_finite() || determinant.abs() <= determinant_epsilon_m2 {
        return None;
    }

    let inverse = 1.0 / determinant;
    let offset = sub(origin, triangle.a);
    let u = dot(offset, p) * inverse;
    if !u.is_finite()
        || u < -barycentric_epsilon
        || u > 1.0 + barycentric_epsilon
    {
        return None;
    }

    let q = cross(offset, edge_a);
    let v = dot(direction, q) * inverse;
    if !v.is_finite()
        || v < -barycentric_epsilon
        || u + v > 1.0 + barycentric_epsilon
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
    if contract.clipping_policy != "inclusive_interval_with_epsilon_tolerance"
        || contract.triangle_sidedness != "two_sided"
        || contract.self_hit_exclusion != "exact_surface_id_and_triangle_id"
        || contract.expected_hit_policy != "all_hits_within_nearest_hit_tie_epsilon"
        || !contract.clip_endpoint_tolerance_m.is_finite()
        || contract.clip_endpoint_tolerance_m != 1.0e-8
        || !contract.nearest_hit_tie_epsilon_m.is_finite()
        || contract.nearest_hit_tie_epsilon_m != 1.0e-8
        || !contract.determinant_epsilon_m2.is_finite()
        || contract.determinant_epsilon_m2 != 1.0e-12
        || !contract.barycentric_epsilon.is_finite()
        || contract.barycentric_epsilon != 1.0e-12
    {
        return Err(
            "fixture query tolerances do not match the pinned evaluator BVH v1 profile"
                .to_owned(),
        );
    }
    if fixture.triangles.is_empty() || fixture.rays.is_empty() {
        return Err("fixture must contain triangles and rays".to_owned());
    }

    let mut triangle_ids = HashSet::new();
    for triangle in &fixture.triangles {
        if !validate_triangle(triangle) {
            return Err(format!(
                "invalid or degenerate triangle {}/{}",
                triangle.surface_id, triangle.triangle_id
            ));
        }
        if !triangle_ids.insert((triangle.surface_id.as_str(), triangle.triangle_id.as_str())) {
            return Err("fixture contains duplicate triangle identity".to_owned());
        }
    }

    let occluders: Vec<OccluderTriangle> = fixture
        .triangles
        .iter()
        .map(|triangle| {
            Ok(OccluderTriangle {
                surface_id: triangle.surface_id.clone(),
                triangle_id: triangle.triangle_id.clone(),
                a: point3(triangle.a),
                b: point3(triangle.b),
                c: point3(triangle.c),
                kind: occluder_kind(&triangle.kind)?,
            })
        })
        .collect::<Result<_, String>>()?;
    // The BVH is built exactly once for the entire known-answer ray batch.
    let bvh_index = RayQueryIndexV1::build(&occluders)?;
    let (bvh_node_count, bvh_leaf_count) = bvh_index.build_stats();

    let mut ray_ids = HashSet::new();
    let mut hit_count = 0usize;
    let mut miss_count = 0usize;
    let mut bvh_hit_count = 0usize;
    let mut bvh_miss_count = 0usize;
    let mut bvh_ray_test_count = 0usize;
    let mut bvh_node_visit_count = 0usize;

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

        // Independent O(n) brute-force reference.
        let mut brute_hits: Vec<(f64, Identity)> = fixture
            .triangles
            .iter()
            .filter(|triangle| {
                ray.exclude_target.as_ref().is_none_or(|excluded| {
                    excluded[0] != triangle.surface_id || excluded[1] != triangle.triangle_id
                })
            })
            .filter_map(|triangle| {
                ray_triangle_distance(
                    ray.origin,
                    ray.direction,
                    triangle,
                    contract.determinant_epsilon_m2,
                    contract.barycentric_epsilon,
                )
                .and_then(|distance| {
                    (distance >= ray.tnear_m - contract.clip_endpoint_tolerance_m
                        && distance <= ray.tfar_m + contract.clip_endpoint_tolerance_m)
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

        brute_hits.sort_by(|left, right| {
            left.0.total_cmp(&right.0).then_with(|| left.1.cmp(&right.1))
        });
        let brute_nearest_distance = brute_hits.first().map(|hit| hit.0);
        let mut brute_nearest_hits: Vec<Identity> = brute_hits
            .iter()
            .take_while(|(distance, _)| {
                brute_nearest_distance
                    .is_some_and(|nearest| *distance - nearest <= contract.nearest_hit_tie_epsilon_m)
            })
            .map(|(_, identity)| identity.clone())
            .collect();
        brute_nearest_hits.sort();

        // Same exact ray/triangle inventory through the production candidate BVH.
        let bvh_receipt = bvh_index.query(
            &RayQueryV1 {
                origin: point3(ray.origin),
                direction: point3(ray.direction),
                tnear_m: ray.tnear_m,
                tfar_m: ray.tfar_m,
                exclude_target: ray.exclude_target.clone(),
            },
            100_000,
            100_000,
        )?;
        bvh_ray_test_count = bvh_ray_test_count
            .checked_add(bvh_receipt.ray_test_count)
            .ok_or("BVH ray-test counter overflowed")?;
        bvh_node_visit_count = bvh_node_visit_count
            .checked_add(bvh_receipt.bvh_node_visit_count)
            .ok_or("BVH node-visit counter overflowed")?;

        let bvh_nearest_hits: Vec<Identity> = bvh_receipt
            .nearest_hits
            .iter()
            .map(|hit| [hit.surface_id.clone(), hit.triangle_id.clone()])
            .collect();
        if bvh_nearest_hits != brute_nearest_hits {
            return Err(format!(
                "ray {} BVH/brute-force identity disagreement: BVH={bvh_nearest_hits:?}, brute={brute_nearest_hits:?}",
                ray.ray_id
            ));
        }
        match (bvh_receipt.nearest_hits.first(), brute_nearest_distance) {
            (Some(bvh_hit), Some(brute_distance)) => {
                if (bvh_hit.distance_m - brute_distance).abs()
                    > contract.clip_endpoint_tolerance_m
                {
                    return Err(format!(
                        "ray {} BVH/brute-force distance disagreement: BVH={}, brute={brute_distance}",
                        ray.ray_id, bvh_hit.distance_m
                    ));
                }
                bvh_hit_count += 1;
            }
            (None, None) => bvh_miss_count += 1,
            _ => {
                return Err(format!(
                    "ray {} BVH/brute-force hit/miss disagreement",
                    ray.ray_id
                ));
            }
        }

        let mut expected_hits = ray.expected.nearest_hits.clone();
        expected_hits.sort();
        let expected_distance = ray.expected.nearest_distance_m;
        match (ray.expected.status.as_str(), brute_nearest_distance) {
            ("hit", Some(actual_distance)) => {
                let expected = expected_distance
                    .ok_or_else(|| format!("ray {} expected hit without distance", ray.ray_id))?;
                if (actual_distance - expected).abs()
                    > contract.clip_endpoint_tolerance_m
                    || brute_nearest_hits != expected_hits
                {
                    return Err(format!(
                        "ray {} brute-force mismatch: distance={actual_distance}, hits={brute_nearest_hits:?}; expected={expected}, hits={expected_hits:?}",
                        ray.ray_id
                    ));
                }
                hit_count += 1;
            }
            ("miss", None) => {
                if expected_distance.is_some() || !expected_hits.is_empty() {
                    return Err(format!(
                        "ray {} miss fixture contains a distance or hit identity",
                        ray.ray_id
                    ));
                }
                miss_count += 1;
            }
            _ => {
                return Err(format!(
                    "ray {} expected status/distance disagrees with brute-force result",
                    ray.ray_id
                ));
            }
        }
    }

    if hit_count != bvh_hit_count || miss_count != bvh_miss_count {
        return Err("aggregate BVH and brute-force hit/miss counts disagree".to_owned());
    }

    Ok(FixtureReport {
        fixture_id: &fixture.fixture_id,
        triangle_count: fixture.triangles.len(),
        ray_count: fixture.rays.len(),
        hit_count,
        miss_count,
        brute_force_checked_nearest_identity_sets: fixture.rays.len(),
        bvh_checked_nearest_identity_sets: fixture.rays.len(),
        bvh_hit_count,
        bvh_miss_count,
        bvh_node_count,
        bvh_leaf_count,
        bvh_ray_test_count,
        bvh_node_visit_count,
        occluder_manifest_sha256: bvh_index.occluder_manifest_sha256().to_owned(),
        brute_force_oracle: "independent_bruteforce_moller_trumbore_f64_v1",
        bvh_engine: "evaluator_bvh_median_v1",
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
            return ExitCode::FAILURE;
        }
    }
}
