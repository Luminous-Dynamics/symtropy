// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Ray casting: find the first intersection of a ray with physics bodies.
//!
//! Exact transformed queries are implemented for spheres, hyperboxes,
//! capsules, and half-spaces. Other shapes currently use a conservative
//! transformed bounding-sphere fallback.
//!
//! # Usage
//! ```ignore
//! let hit = raycast(&world, &origin, &direction, 100.0);
//! if let Some(hit) = hit {
//!     println!("hit body {:?} at distance {}", hit.body, hit.distance);
//! }
//! ```

use nalgebra::SVector;

use crate::body::{BodyHandle, RigidBody};
use crate::world::PhysicsWorld;
use symtropy_math::{Capsule, HalfSpace, HyperBox, Point, Sphere};

/// Result of a ray cast against the physics world.
#[derive(Clone, Debug)]
pub struct RayHit<const D: usize> {
    /// The body that was hit.
    pub body: BodyHandle,
    /// Distance from the ray origin to the hit point.
    pub distance: f64,
    /// Hit point in world space.
    pub point: SVector<f64, D>,
    /// Surface normal at the hit point (pointing toward the ray origin).
    pub normal: SVector<f64, D>,
}

/// Cast a ray through the physics world and return the closest hit.
///
/// `origin`: ray start point
/// `direction`: ray direction (will be normalized internally)
/// `max_distance`: maximum ray length (for performance and game design)
///
/// Returns `None` if no body is hit within `max_distance`.
/// Skips sensors by default.
pub fn raycast<const D: usize>(
    world: &PhysicsWorld<D>,
    origin: &SVector<f64, D>,
    direction: &SVector<f64, D>,
    max_distance: f64,
) -> Option<RayHit<D>> {
    let dir_norm = direction.norm();
    if dir_norm < 1e-15 {
        return None;
    }
    let dir = direction / dir_norm;

    let mut closest: Option<RayHit<D>> = None;

    for body in &world.bodies {
        if body.is_sensor {
            continue;
        }
        if let Some(hit) = raycast_body(body, origin, &dir, max_distance) {
            if closest
                .as_ref()
                .is_none_or(|current| hit.distance < current.distance)
            {
                closest = Some(hit);
            }
        }
    }

    closest
}

/// Cast a ray and return ALL hits (not just the closest), sorted by distance.
pub fn raycast_all<const D: usize>(
    world: &PhysicsWorld<D>,
    origin: &SVector<f64, D>,
    direction: &SVector<f64, D>,
    max_distance: f64,
) -> Vec<RayHit<D>> {
    let dir_norm = direction.norm();
    if dir_norm < 1e-15 {
        return Vec::new();
    }
    let dir = direction / dir_norm;

    let mut hits = Vec::new();

    for body in &world.bodies {
        if body.is_sensor {
            continue;
        }
        if let Some(hit) = raycast_body(body, origin, &dir, max_distance) {
            hits.push(hit);
        }
    }

    hits.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits
}

fn raycast_body<const D: usize>(
    body: &RigidBody<D>,
    origin_world: &SVector<f64, D>,
    direction_world: &SVector<f64, D>,
    max_distance: f64,
) -> Option<RayHit<D>> {
    let inverse = body.transform.inverse();
    let origin_local = inverse.transform_point(&Point(*origin_world)).0;
    let direction_local = inverse.transform_vector(direction_world);

    let local_hit = if let Some(sphere) = body.collider.as_any().downcast_ref::<Sphere<D>>() {
        ray_sphere_intersection(
            &origin_local,
            &direction_local,
            &sphere.center.0,
            sphere.radius,
        )
        .map(|t| {
            let point = origin_local + direction_local * t;
            let normal = (point - sphere.center.0).normalize();
            (t, normal)
        })
    } else if let Some(hyperbox) = body.collider.as_any().downcast_ref::<HyperBox<D>>() {
        ray_box_intersection(&origin_local, &direction_local, &hyperbox.half_extents)
    } else if let Some(capsule) = body.collider.as_any().downcast_ref::<Capsule<D>>() {
        ray_capsule_intersection(&origin_local, &direction_local, capsule)
    } else if let Some(halfspace) = body.collider.as_any().downcast_ref::<HalfSpace<D>>() {
        ray_halfspace_intersection(&origin_local, &direction_local, halfspace)
    } else {
        // Conservative compatibility fallback for shape types without an exact
        // ray query yet. This may report a bounding-sphere false positive, but
        // it never loses orientation of the sphere center.
        let (center, radius) = body.collider.bounding_sphere();
        ray_sphere_intersection(&origin_local, &direction_local, &center.0, radius).map(|t| {
            let point = origin_local + direction_local * t;
            let normal = (point - center.0).normalize();
            (t, normal)
        })
    }?;

    let (distance, normal_local) = local_hit;
    if distance <= 0.0 || distance > max_distance {
        return None;
    }

    let point = origin_world + direction_world * distance;
    let mut normal = body.transform.transform_vector(&normal_local);
    let normal_length = normal.norm();
    if normal_length < 1e-15 {
        return None;
    }
    normal /= normal_length;
    if normal.dot(direction_world) > 0.0 {
        normal = -normal;
    }

    Some(RayHit {
        body: body.handle,
        distance,
        point,
        normal,
    })
}

fn ray_box_intersection<const D: usize>(
    origin: &SVector<f64, D>,
    direction: &SVector<f64, D>,
    half_extents: &[f64; D],
) -> Option<(f64, SVector<f64, D>)> {
    let mut t_min = f64::NEG_INFINITY;
    let mut t_max = f64::INFINITY;
    let mut enter_axis = 0usize;
    let mut enter_sign = 0.0;
    let mut exit_axis = 0usize;
    let mut exit_sign = 0.0;

    for axis in 0..D {
        if direction[axis].abs() < 1e-15 {
            if origin[axis].abs() > half_extents[axis] {
                return None;
            }
            continue;
        }

        let inv = 1.0 / direction[axis];
        let mut near = (-half_extents[axis] - origin[axis]) * inv;
        let mut far = (half_extents[axis] - origin[axis]) * inv;
        let mut near_sign = -1.0;
        let mut far_sign = 1.0;
        if near > far {
            std::mem::swap(&mut near, &mut far);
            std::mem::swap(&mut near_sign, &mut far_sign);
        }
        if near > t_min {
            t_min = near;
            enter_axis = axis;
            enter_sign = near_sign;
        }
        if far < t_max {
            t_max = far;
            exit_axis = axis;
            exit_sign = far_sign;
        }
        if t_min > t_max {
            return None;
        }
    }

    let (t, axis, sign) = if t_min > 0.0 {
        (t_min, enter_axis, enter_sign)
    } else if t_max > 0.0 {
        (t_max, exit_axis, exit_sign)
    } else {
        return None;
    };
    let mut normal = SVector::<f64, D>::zeros();
    normal[axis] = sign;
    Some((t, normal))
}

fn ray_capsule_intersection<const D: usize>(
    origin: &SVector<f64, D>,
    direction: &SVector<f64, D>,
    capsule: &Capsule<D>,
) -> Option<(f64, SVector<f64, D>)> {
    if capsule.axis >= D {
        return None;
    }

    let mut best: Option<(f64, SVector<f64, D>)> = None;
    let mut o_perp = *origin;
    let mut d_perp = *direction;
    o_perp[capsule.axis] = 0.0;
    d_perp[capsule.axis] = 0.0;
    let a = d_perp.norm_squared();
    if a > 1e-15 {
        let b = 2.0 * o_perp.dot(&d_perp);
        let c = o_perp.norm_squared() - capsule.radius * capsule.radius;
        let discriminant = b * b - 4.0 * a * c;
        if discriminant >= 0.0 {
            let sqrt_disc = discriminant.sqrt();
            for t in [(-b - sqrt_disc) / (2.0 * a), (-b + sqrt_disc) / (2.0 * a)] {
                if t <= 0.0 {
                    continue;
                }
                let axial = origin[capsule.axis] + direction[capsule.axis] * t;
                if axial.abs() <= capsule.half_height {
                    let point = origin + direction * t;
                    let mut normal = point;
                    normal[capsule.axis] = 0.0;
                    let length = normal.norm();
                    if length > 1e-15 {
                        best = Some((t, normal / length));
                        break;
                    }
                }
            }
        }
    }

    for sign in [-1.0, 1.0] {
        let mut center = SVector::<f64, D>::zeros();
        center[capsule.axis] = sign * capsule.half_height;
        if let Some(t) = ray_sphere_intersection(origin, direction, &center, capsule.radius) {
            let replace = best.as_ref().is_none_or(|(current, _)| t < *current);
            if replace {
                let point = origin + direction * t;
                best = Some((t, (point - center).normalize()));
            }
        }
    }

    best
}

fn ray_halfspace_intersection<const D: usize>(
    origin: &SVector<f64, D>,
    direction: &SVector<f64, D>,
    halfspace: &HalfSpace<D>,
) -> Option<(f64, SVector<f64, D>)> {
    let normal_length = halfspace.normal.norm();
    if normal_length < 1e-15 {
        return None;
    }
    let normal = halfspace.normal / normal_length;
    let offset = halfspace.offset / normal_length;
    let denominator = normal.dot(direction);
    if denominator.abs() < 1e-15 {
        return None;
    }
    let t = (offset - normal.dot(origin)) / denominator;
    if t <= 0.0 {
        return None;
    }
    let hit_normal = if denominator < 0.0 { normal } else { -normal };
    Some((t, hit_normal))
}

/// Analytical ray-sphere intersection.
///
/// Returns the distance `t` to the nearest intersection point, or `None` if the ray misses.
/// Uses the standard quadratic formula: solve `|origin + t*dir - center|² = radius²`.
fn ray_sphere_intersection<const D: usize>(
    origin: &SVector<f64, D>,
    dir: &SVector<f64, D>, // must be unit length
    center: &SVector<f64, D>,
    radius: f64,
) -> Option<f64> {
    let oc = origin - center;
    let a = dir.dot(dir); // Should be ~1.0 for unit dir
    let b = 2.0 * oc.dot(dir);
    let c = oc.dot(&oc) - radius * radius;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }

    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);

    // Return the nearest positive intersection
    if t1 > 0.0 {
        Some(t1)
    } else if t2 > 0.0 {
        Some(t2)
    } else {
        None // Ray starts inside or behind the sphere
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::{Bivector, HyperBox, Point, Rotor, Sphere, Transform};

    #[test]
    fn ray_hits_sphere() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let h = world.add_sphere(Point::new([10.0, 0.0, 0.0]), 1.0, 1.0);

        let origin = SVector::from([0.0, 0.0, 0.0]);
        let dir = SVector::from([1.0, 0.0, 0.0]);

        let hit = raycast(&world, &origin, &dir, 100.0).unwrap();
        assert_eq!(hit.body, h);
        // Hit distance should be ~9.0 (sphere center at 10, radius 1)
        assert!(
            (hit.distance - 9.0).abs() < 0.1,
            "hit distance = {}, expected ~9.0",
            hit.distance
        );
    }

    #[test]
    fn ray_misses_sphere() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        world.add_sphere(Point::new([10.0, 5.0, 0.0]), 1.0, 1.0);

        let origin = SVector::from([0.0, 0.0, 0.0]);
        let dir = SVector::from([1.0, 0.0, 0.0]); // Shoots along X, sphere is at Y=5

        let hit = raycast(&world, &origin, &dir, 100.0);
        assert!(hit.is_none(), "ray should miss sphere at Y=5");
    }

    #[test]
    fn ray_max_distance() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        world.add_sphere(Point::new([10.0, 0.0, 0.0]), 1.0, 1.0);

        let origin = SVector::from([0.0, 0.0, 0.0]);
        let dir = SVector::from([1.0, 0.0, 0.0]);

        // Max distance 5 — sphere is at 10, should miss
        let hit = raycast(&world, &origin, &dir, 5.0);
        assert!(hit.is_none(), "ray should not reach sphere at distance 10");
    }

    #[test]
    fn ray_closest_hit() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let h1 = world.add_sphere(Point::new([5.0, 0.0, 0.0]), 1.0, 1.0);
        let h2 = world.add_sphere(Point::new([10.0, 0.0, 0.0]), 1.0, 1.0);

        let origin = SVector::from([0.0, 0.0, 0.0]);
        let dir = SVector::from([1.0, 0.0, 0.0]);

        let hit = raycast(&world, &origin, &dir, 100.0).unwrap();
        assert_eq!(hit.body, h1, "should hit the closer sphere");
    }

    #[test]
    fn raycast_all_returns_sorted() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        world.add_sphere(Point::new([10.0, 0.0, 0.0]), 1.0, 1.0);
        world.add_sphere(Point::new([5.0, 0.0, 0.0]), 1.0, 1.0);

        let origin = SVector::from([0.0, 0.0, 0.0]);
        let dir = SVector::from([1.0, 0.0, 0.0]);

        let hits = raycast_all(&world, &origin, &dir, 100.0);
        assert_eq!(hits.len(), 2);
        assert!(
            hits[0].distance < hits[1].distance,
            "hits should be sorted by distance"
        );
    }

    #[test]
    fn ray_skips_sensors() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let h = world.add_sphere(Point::new([5.0, 0.0, 0.0]), 1.0, 1.0);
        world.body_mut(h).unwrap().is_sensor = true;

        let origin = SVector::from([0.0, 0.0, 0.0]);
        let dir = SVector::from([1.0, 0.0, 0.0]);

        let hit = raycast(&world, &origin, &dir, 100.0);
        assert!(hit.is_none(), "ray should skip sensors");
    }

    #[test]
    fn ray_hit_normal_points_outward() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        world.add_sphere(Point::new([5.0, 0.0, 0.0]), 1.0, 1.0);

        let origin = SVector::from([0.0, 0.0, 0.0]);
        let dir = SVector::from([1.0, 0.0, 0.0]);

        let hit = raycast(&world, &origin, &dir, 100.0).unwrap();
        // Normal should point back toward the ray origin (negative X)
        assert!(hit.normal[0] < 0.0, "normal should face the ray origin");
    }

    #[test]
    fn ray_4d() {
        let mut world = PhysicsWorld::<4>::new(SVector::zeros());
        world.add_sphere(Point::new([0.0, 0.0, 0.0, 5.0]), 1.0, 1.0);

        let origin = SVector::from([0.0, 0.0, 0.0, 0.0]);
        let dir = SVector::from([0.0, 0.0, 0.0, 1.0]); // Cast along W axis

        let hit = raycast(&world, &origin, &dir, 100.0).unwrap();
        assert!(
            (hit.distance - 4.0).abs() < 0.1,
            "4D ray hit distance = {}, expected ~4.0",
            hit.distance
        );
    }

    #[test]
    fn ray_sphere_analytical_behind() {
        // Ray starts inside sphere — should not report hit (t < 0 for entry)
        let origin = SVector::from([0.0, 0.0, 0.0]);
        let dir = SVector::from([1.0, 0.0, 0.0]);
        let center = SVector::from([0.0, 0.0, 0.0]);
        let radius = 5.0;

        // Origin is at center — t1 < 0, t2 > 0
        let t = ray_sphere_intersection(&origin, &dir, &center, radius);
        // Should return t2 (exit point, forward along ray)
        assert!(t.is_some());
        assert!(t.unwrap() > 0.0);
    }

    #[test]
    fn ray_hits_rotated_thin_box_using_local_space_query() {
        use std::f64::consts::FRAC_PI_2;

        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let body = RigidBody::new(
            BodyHandle(0),
            crate::body::BodyType::Static,
            Transform {
                translation: Point::new([5.0, 0.0, 0.0]),
                rotation: Rotor::from_plane_angle(&Bivector::unit_plane(0, 1), FRAC_PI_2),
            },
            Box::new(HyperBox::<3>::new([2.0, 0.25, 0.25])),
            0.0,
            SVector::zeros(),
        );
        let handle = world.add_body(body);

        // The long local X axis has rotated into world Y. This ray would miss
        // if the collider were still treated as axis aligned in local space.
        let origin = SVector::from([0.0, 1.5, 0.0]);
        let direction = SVector::from([1.0, 0.0, 0.0]);
        let hit = raycast(&world, &origin, &direction, 100.0).unwrap();

        assert_eq!(hit.body, handle);
        assert!((hit.distance - 4.75).abs() < 1e-10);
        assert!(hit.normal[0] < -0.999);
    }

    #[test]
    fn ray_rejects_bounding_sphere_false_positive_for_box() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let body = RigidBody::new(
            BodyHandle(0),
            crate::body::BodyType::Static,
            Transform::from_translation(Point::new([5.0, 0.0, 0.0])),
            Box::new(HyperBox::<3>::new([2.0, 0.1, 0.1])),
            0.0,
            SVector::zeros(),
        );
        world.add_body(body);

        // The enclosing sphere reaches y ~= 2, but the box reaches only 0.1.
        let origin = SVector::from([0.0, 1.0, 0.0]);
        let direction = SVector::from([1.0, 0.0, 0.0]);
        assert!(raycast(&world, &origin, &direction, 100.0).is_none());
    }
}
