#![forbid(unsafe_code)]

//! Deterministic geometric spray-cone coverage estimates.
//!
//! This crate answers a deliberately narrow question: given a nozzle's idealized
//! conical envelope and a triangulated target mesh, what fraction of the mesh
//! surface is geometrically visible to that nozzle without line-of-sight
//! occlusion?
//!
//! This is NOT CFD, droplet transport, wetting, impingement, soil removal,
//! drying, hygiene or sanitization. It is a cheap geometry-screening layer to
//! prioritize designs for physical spray testing and higher-fidelity models.

use std::f64::consts::PI;

const MAX_SAMPLES_PER_TRIANGLE: u32 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn norm_squared(self) -> f64 {
        self.dot(self)
    }

    pub fn norm(self) -> f64 {
        self.norm_squared().sqrt()
    }

    pub fn normalized(self) -> Option<Self> {
        let norm = self.norm();
        if norm.is_finite() && norm > 0.0 {
            Some(self.scale(1.0 / norm))
        } else {
            None
        }
    }

    pub fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    pub fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    pub fn scale(self, scalar: f64) -> Self {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}

impl Triangle {
    pub fn area_m2(self) -> f64 {
        self.b.sub(self.a).cross(self.c.sub(self.a)).norm() * 0.5
    }

    fn is_finite(self) -> bool {
        self.a.is_finite() && self.b.is_finite() && self.c.is_finite()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TriangleMesh {
    pub triangles: Vec<Triangle>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SprayCone {
    pub origin_m: Vec3,
    /// Main flow direction. It is normalized internally.
    pub axis: Vec3,
    /// Idealized cone half-angle in radians, strictly between zero and PI/2.
    pub half_angle_rad: f64,
    pub max_range_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverageConfig {
    pub samples_per_triangle: u32,
    /// Fractional absolute normal-incidence cosine threshold in [0, 1].
    /// This is a geometric impact-angle descriptor, not a cleaning threshold.
    pub min_incidence_cosine: f64,
    /// Distance below which rays are treated as numerically coincident.
    pub epsilon_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriangleCoverage {
    pub triangle_index: usize,
    pub area_m2: f64,
    pub covered_samples: u32,
    pub total_samples: u32,
    pub estimated_covered_area_m2: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoverageReport {
    pub total_surface_area_m2: f64,
    pub estimated_covered_area_m2: f64,
    /// Area-weighted geometrical visibility fraction, not wetting or cleaning.
    pub geometrically_visible_area_fraction: f64,
    /// Area-weighted absolute normal-incidence cosine for visible samples.
    /// 1 is normal incidence; 0 is grazing. Zero if no samples are visible.
    pub mean_visible_abs_incidence_cosine: f64,
    /// Fraction of total target area both visible and above configured incidence.
    pub favorable_incidence_area_fraction: f64,
    pub favorable_incidence_threshold: f64,
    pub total_samples: u64,
    pub visible_samples: u64,
    pub favorable_incidence_samples: u64,
    pub triangles: Vec<TriangleCoverage>,
}

fn halton(mut index: u32, base: u32) -> f64 {
    let mut fraction = 1.0;
    let mut value = 0.0;
    while index > 0 {
        fraction /= base as f64;
        value += fraction * (index % base) as f64;
        index /= base;
    }
    value
}

/// Deterministically sample points uniformly over a triangle using a Halton
/// sequence transformed to barycentric coordinates.
fn sample_triangle(triangle: Triangle, sample_index: u32) -> Vec3 {
    let u = halton(sample_index + 1, 2).sqrt();
    let v = halton(sample_index + 1, 3);
    triangle
        .a
        .scale(1.0 - u)
        .add(triangle.b.scale(u * (1.0 - v)))
        .add(triangle.c.scale(u * v))
}

/// Möller–Trumbore ray/triangle intersection. Returns ray distance in metres.
fn ray_triangle_distance(
    origin: Vec3,
    direction_unit: Vec3,
    triangle: Triangle,
    epsilon: f64,
) -> Option<f64> {
    let edge1 = triangle.b.sub(triangle.a);
    let edge2 = triangle.c.sub(triangle.a);
    let p = direction_unit.cross(edge2);
    let determinant = edge1.dot(p);

    // Scale-aware parallelism test (determinant has units of area).
    let determinant_tolerance = f64::EPSILON * edge1.norm() * edge2.norm() * 16.0;
    if determinant.abs() <= determinant_tolerance {
        return None;
    }

    let inverse_determinant = 1.0 / determinant;
    let offset = origin.sub(triangle.a);
    let u = offset.dot(p) * inverse_determinant;
    let barycentric_epsilon = 1.0e-12;
    if u < -barycentric_epsilon || u > 1.0 + barycentric_epsilon {
        return None;
    }

    let q = offset.cross(edge1);
    let v = direction_unit.dot(q) * inverse_determinant;
    if v < -barycentric_epsilon || u + v > 1.0 + barycentric_epsilon {
        return None;
    }

    let distance = edge2.dot(q) * inverse_determinant;
    if distance > epsilon && distance.is_finite() {
        Some(distance)
    } else {
        None
    }
}

fn is_occluded(
    occlusion_mesh: &TriangleMesh,
    origin: Vec3,
    direction_unit: Vec3,
    target_distance: f64,
    epsilon: f64,
) -> bool {
    occlusion_mesh.triangles.iter().any(|triangle| {
        ray_triangle_distance(origin, direction_unit, *triangle, epsilon)
            .is_some_and(|hit_distance| hit_distance < target_distance - epsilon)
    })
}

/// Estimate geometric surface visibility through an idealized nozzle cone.
///
/// Each target triangle is sampled independently and its estimated covered
/// area is weighted by its true area. The occlusion mesh should contain all
/// opaque geometry, including target surfaces and any racks/walls that can
/// block line of sight. Hits at the target endpoint are not considered occlusion.
/// Sampling is deterministic for fixed meshes and configuration.
pub fn estimate_coverage(
    target_mesh: &TriangleMesh,
    occlusion_mesh: &TriangleMesh,
    nozzle: SprayCone,
    config: CoverageConfig,
) -> Result<CoverageReport, &'static str> {
    if target_mesh.triangles.is_empty() {
        return Err("target mesh must contain at least one triangle");
    }
    if target_mesh.triangles.iter().any(|triangle| !triangle.is_finite())
        || occlusion_mesh.triangles.iter().any(|triangle| !triangle.is_finite())
    {
        return Err("target and occlusion mesh vertices must be finite");
    }
    if target_mesh.triangles.iter().any(|triangle| {
        !triangle.area_m2().is_finite() || triangle.area_m2() <= 0.0
    }) || occlusion_mesh.triangles.iter().any(|triangle| {
        !triangle.area_m2().is_finite() || triangle.area_m2() <= 0.0
    }) {
        return Err("target and occlusion meshes cannot contain degenerate triangles");
    }
    if !nozzle.origin_m.is_finite() || !nozzle.axis.is_finite() {
        return Err("nozzle origin and axis must be finite");
    }
    let axis = nozzle.axis.normalized().ok_or("nozzle axis must be non-zero")?;
    if !(nozzle.half_angle_rad.is_finite()
        && nozzle.half_angle_rad > 0.0
        && nozzle.half_angle_rad < PI / 2.0)
    {
        return Err("spray cone half-angle must be in (0, PI/2)");
    }
    if !(nozzle.max_range_m.is_finite() && nozzle.max_range_m > 0.0) {
        return Err("spray maximum range must be finite and positive");
    }
    if config.samples_per_triangle == 0
        || config.samples_per_triangle > MAX_SAMPLES_PER_TRIANGLE
    {
        return Err("samples_per_triangle must be between 1 and 100000");
    }
    if !(config.epsilon_m.is_finite() && config.epsilon_m > 0.0) {
        return Err("epsilon_m must be finite and positive");
    }
    if !(config.min_incidence_cosine.is_finite()
        && (0.0..=1.0).contains(&config.min_incidence_cosine))
    {
        return Err("min_incidence_cosine must be finite and in [0, 1]");
    }

    let cone_cosine = nozzle.half_angle_rad.cos();
    let mut total_surface_area_m2 = 0.0;
    let mut estimated_covered_area_m2 = 0.0;
    let mut total_samples = 0_u64;
    let mut visible_samples = 0_u64;
    let mut favorable_incidence_samples = 0_u64;
    let mut incidence_weighted_area_m2 = 0.0;
    let mut favorable_incidence_area_m2 = 0.0;
    let mut triangle_results = Vec::with_capacity(mesh.triangles.len());

    for (triangle_index, triangle) in target_mesh.triangles.iter().enumerate() {
        let area_m2 = triangle.area_m2();
        total_surface_area_m2 += area_m2;
        let triangle_normal = triangle
            .b
            .sub(triangle.a)
            .cross(triangle.c.sub(triangle.a))
            .normalized()
            .ok_or("target triangle normal is undefined")?;
        let sample_area_m2 = area_m2 / config.samples_per_triangle as f64;
        let mut covered_samples = 0_u32;

        for sample_index in 0..config.samples_per_triangle {
            let point = sample_triangle(*triangle, sample_index);
            let ray = point.sub(nozzle.origin_m);
            let distance = ray.norm();
            total_samples += 1;

            if !distance.is_finite()
                || distance <= config.epsilon_m
                || distance > nozzle.max_range_m
            {
                continue;
            }

            let direction = ray.scale(1.0 / distance);
            if direction.dot(axis) < cone_cosine {
                continue;
            }

            if !is_occluded(
                occlusion_mesh,
                nozzle.origin_m,
                direction,
                distance,
                config.epsilon_m,
            ) {
                covered_samples += 1;
                visible_samples += 1;
            }
        }

        let covered_area = area_m2
            * covered_samples as f64
            / config.samples_per_triangle as f64;
        estimated_covered_area_m2 += covered_area;
        triangle_results.push(TriangleCoverage {
            triangle_index,
            area_m2,
            covered_samples,
            total_samples: config.samples_per_triangle,
            estimated_covered_area_m2: covered_area,
        });
    }

    if !total_surface_area_m2.is_finite()
        || !estimated_covered_area_m2.is_finite()
        || !incidence_weighted_area_m2.is_finite()
        || !favorable_incidence_area_m2.is_finite()
        || total_surface_area_m2 <= 0.0
    {
        return Err("derived surface area is non-finite or non-positive");
    }

    Ok(CoverageReport {
        total_surface_area_m2,
        estimated_covered_area_m2,
        geometrically_visible_area_fraction: estimated_covered_area_m2
            / total_surface_area_m2,
        mean_visible_abs_incidence_cosine: if estimated_covered_area_m2 > 0.0 {
            incidence_weighted_area_m2 / estimated_covered_area_m2
        } else {
            0.0
        },
        favorable_incidence_area_fraction: favorable_incidence_area_m2 / total_surface_area_m2,
        favorable_incidence_threshold: config.min_incidence_cosine,
        total_samples,
        visible_samples,
        favorable_incidence_samples,
        triangles: triangle_results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle_at_z(z: f64, half_width: f64) -> Triangle {
        Triangle {
            a: Vec3::new(-half_width, -half_width, z),
            b: Vec3::new(half_width, -half_width, z),
            c: Vec3::new(0.0, half_width, z),
        }
    }

    fn standard_config() -> CoverageConfig {
        CoverageConfig {
            samples_per_triangle: 256,
            min_incidence_cosine: 0.5,
            epsilon_m: 1.0e-8,
        }
    }

    fn standard_nozzle() -> SprayCone {
        SprayCone {
            origin_m: Vec3::new(0.0, 0.0, 0.0),
            axis: Vec3::new(0.0, 0.0, 1.0),
            half_angle_rad: 0.5,
            max_range_m: 5.0,
        }
    }

    #[test]
    fn triangle_inside_cone_and_unoccluded_is_fully_visible() {
        let mesh = TriangleMesh {
            triangles: vec![triangle_at_z(1.0, 0.1)],
        };
        let report = estimate_coverage(&mesh, &mesh, standard_nozzle(), standard_config()).unwrap();
        assert!((report.geometrically_visible_area_fraction - 1.0).abs() < 1.0e-12);
        assert_eq!(report.visible_samples, 256);
    }

    #[test]
    fn triangle_outside_cone_is_not_visible() {
        let mesh = TriangleMesh {
            triangles: vec![triangle_at_z(1.0, 0.1)],
        };
        let mut nozzle = standard_nozzle();
        nozzle.axis = Vec3::new(1.0, 0.0, 0.0);
        nozzle.half_angle_rad = 0.1;
        let report = estimate_coverage(&mesh, &mesh, nozzle, standard_config()).unwrap();
        assert_eq!(report.geometrically_visible_area_fraction, 0.0);
    }

    #[test]
    fn near_surface_occludes_far_surface() {
        let occlusion_mesh = TriangleMesh {
            triangles: vec![triangle_at_z(1.0, 0.2), triangle_at_z(2.0, 0.2)],
        };
        let target_mesh = TriangleMesh {
            triangles: vec![triangle_at_z(2.0, 0.2)],
        };
        let report = estimate_coverage(
            &target_mesh,
            &occlusion_mesh,
            standard_nozzle(),
            standard_config(),
        )
        .unwrap();
        assert_eq!(report.triangles.len(), 1);
        assert_eq!(report.triangles[0].covered_samples, 0);
    }

    #[test]
    fn maximum_range_limits_geometric_coverage() {
        let mesh = TriangleMesh {
            triangles: vec![triangle_at_z(1.0, 0.1)],
        };
        let mut nozzle = standard_nozzle();
        nozzle.max_range_m = 0.5;
        let report = estimate_coverage(&mesh, &mesh, nozzle, standard_config()).unwrap();
        assert_eq!(report.geometrically_visible_area_fraction, 0.0);
    }

    #[test]
    fn incidence_metrics_distinguish_normal_from_grazing_visibility() {
        let mesh = TriangleMesh {
            triangles: vec![triangle_at_z(1.0, 0.1)],
        };
        let normal = estimate_coverage(
            &mesh,
            &mesh,
            standard_nozzle(),
            standard_config(),
        )
        .unwrap();
        assert!((normal.mean_visible_abs_incidence_cosine - 1.0).abs() < 1.0e-12);
        assert_eq!(normal.favorable_incidence_area_fraction, 1.0);

        let mut grazing_nozzle = standard_nozzle();
        grazing_nozzle.origin_m = Vec3::new(-5.0, 0.0, 0.0);
        grazing_nozzle.axis = Vec3::new(1.0, 0.0, 0.2);
        grazing_nozzle.half_angle_rad = 0.5;
        let grazing = estimate_coverage(
            &mesh,
            &mesh,
            grazing_nozzle,
            standard_config(),
        )
        .unwrap();
        assert!(grazing.geometrically_visible_area_fraction > 0.0);
        assert!(grazing.mean_visible_abs_incidence_cosine < 0.5);
        assert_eq!(grazing.favorable_incidence_area_fraction, 0.0);
    }

    #[test]
    fn rejects_invalid_incidence_threshold() {
        let mesh = TriangleMesh {
            triangles: vec![triangle_at_z(1.0, 0.1)],
        };
        let mut config = standard_config();
        config.min_incidence_cosine = f64::NAN;
        assert!(estimate_coverage(&mesh, &mesh, standard_nozzle(), config).is_err());
        config.min_incidence_cosine = 1.1;
        assert!(estimate_coverage(&mesh, &mesh, standard_nozzle(), config).is_err());
    }

    #[test]
    fn output_is_deterministic_for_identical_inputs() {
        let mesh = TriangleMesh {
            triangles: vec![triangle_at_z(1.0, 0.1), triangle_at_z(2.0, 0.15)],
        };
        let first = estimate_coverage(&mesh, &mesh, standard_nozzle(), standard_config()).unwrap();
        let second = estimate_coverage(&mesh, &mesh, standard_nozzle(), standard_config()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn rejects_degenerate_geometry_and_invalid_sampling() {
        let degenerate = TriangleMesh {
            triangles: vec![Triangle {
                a: Vec3::new(0.0, 0.0, 1.0),
                b: Vec3::new(0.0, 0.0, 1.0),
                c: Vec3::new(0.0, 0.0, 1.0),
            }],
        };
        assert!(estimate_coverage(&degenerate, &degenerate, standard_nozzle(), standard_config()).is_err());

        let valid = TriangleMesh {
            triangles: vec![triangle_at_z(1.0, 0.1)],
        };
        let invalid_config = CoverageConfig {
            samples_per_triangle: 0,
            min_incidence_cosine: 0.5,
            epsilon_m: 1.0e-8,
        };
        assert!(estimate_coverage(&valid, &valid, standard_nozzle(), invalid_config).is_err());
    }
}
