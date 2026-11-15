//! Independent exact finite-segment/capsule narrow-phase reference.
//!
//! The query is a moving sphere along a straight centre path against a stationary
//! capsule. It does not derive articulated kinematics or physical link geometry.

use super::Vec3;

const DEGENERATE_EPSILON_SQUARED: f64 = 1.0e-24;
const PARALLEL_EPSILON: f64 = 1.0e-14;
const CONTACT_EPSILON_MM: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    pub start: Vec3,
    pub end: Vec3,
    pub radius_mm: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapsuleError {
    InvalidRadius,
    NonFiniteCoordinate,
}

impl Capsule {
    pub fn new(start: Vec3, end: Vec3, radius_mm: f64) -> Result<Self, CapsuleError> {
        if !radius_mm.is_finite() || radius_mm < 0.0 {
            return Err(CapsuleError::InvalidRadius);
        }
        if !is_finite(start) || !is_finite(end) {
            return Err(CapsuleError::NonFiniteCoordinate);
        }
        Ok(Self {
            start,
            end,
            radius_mm,
        })
    }

    pub fn aabb(self) -> CapsuleAabb3 {
        CapsuleAabb3 {
            min: Vec3::new(
                self.start.x.min(self.end.x) - self.radius_mm,
                self.start.y.min(self.end.y) - self.radius_mm,
                self.start.z.min(self.end.z) - self.radius_mm,
            ),
            max: Vec3::new(
                self.start.x.max(self.end.x) + self.radius_mm,
                self.start.y.max(self.end.y) + self.radius_mm,
                self.start.z.max(self.end.z) + self.radius_mm,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapsuleAabb3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl CapsuleAabb3 {
    pub fn overlaps(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentClosestApproach {
    pub moving_parameter: f64,
    pub capsule_parameter: f64,
    pub moving_point: Vec3,
    pub capsule_axis_point: Vec3,
    pub distance_mm: f64,
}

/// Shortest distance between two finite 3D segments, including parallel and
/// degenerate point-segment cases. Parameters are clamped to [0, 1].
pub fn closest_segment_approach(p0: Vec3, p1: Vec3, q0: Vec3, q1: Vec3) -> SegmentClosestApproach {
    let u = p1.sub(p0);
    let v = q1.sub(q0);
    let w = p0.sub(q0);
    let a = u.dot(u);
    let b = u.dot(v);
    let c = v.dot(v);
    let d = u.dot(w);
    let e = v.dot(w);

    let (s, t);
    if a <= DEGENERATE_EPSILON_SQUARED && c <= DEGENERATE_EPSILON_SQUARED {
        s = 0.0;
        t = 0.0;
    } else if a <= DEGENERATE_EPSILON_SQUARED {
        s = 0.0;
        t = (e / c).clamp(0.0, 1.0);
    } else if c <= DEGENERATE_EPSILON_SQUARED {
        s = (-d / a).clamp(0.0, 1.0);
        t = 0.0;
    } else {
        let denominator = a * c - b * b;
        let mut s_value = if denominator.abs() > PARALLEL_EPSILON * (a * c).max(1.0) {
            ((b * e - c * d) / denominator).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mut t_value = (b * s_value + e) / c;
        if t_value < 0.0 {
            t_value = 0.0;
            s_value = (-d / a).clamp(0.0, 1.0);
        } else if t_value > 1.0 {
            t_value = 1.0;
            s_value = ((b - d) / a).clamp(0.0, 1.0);
        }
        s = s_value;
        t = t_value;
    }

    let moving_point = p0.add(u.scale(s));
    let capsule_axis_point = q0.add(v.scale(t));
    SegmentClosestApproach {
        moving_parameter: s,
        capsule_parameter: t,
        moving_point,
        capsule_axis_point,
        distance_mm: moving_point.distance(capsule_axis_point),
    }
}

/// Exact narrow-phase distance between the sphere-centre path and capsule axis.
pub fn moving_sphere_vs_capsule(
    path_start: Vec3,
    path_end: Vec3,
    moving_radius_mm: f64,
    obstacle: Capsule,
) -> Result<SegmentClosestApproach, CapsuleError> {
    validate_radius(moving_radius_mm)?;
    if !is_finite(path_start) || !is_finite(path_end) {
        return Err(CapsuleError::NonFiniteCoordinate);
    }
    Ok(closest_segment_approach(
        path_start,
        path_end,
        obstacle.start,
        obstacle.end,
    ))
}

pub fn moving_sphere_capsule_collision(
    path_start: Vec3,
    path_end: Vec3,
    moving_radius_mm: f64,
    obstacle: Capsule,
) -> Result<bool, CapsuleError> {
    let closest = moving_sphere_vs_capsule(path_start, path_end, moving_radius_mm, obstacle)?;
    Ok(closest.distance_mm <= moving_radius_mm + obstacle.radius_mm + CONTACT_EPSILON_MM)
}

pub fn endpoints_only_sphere_capsule_collision(
    path_start: Vec3,
    path_end: Vec3,
    moving_radius_mm: f64,
    obstacle: Capsule,
) -> Result<bool, CapsuleError> {
    validate_radius(moving_radius_mm)?;
    if !is_finite(path_start) || !is_finite(path_end) {
        return Err(CapsuleError::NonFiniteCoordinate);
    }
    let start = closest_segment_approach(path_start, path_start, obstacle.start, obstacle.end);
    let end = closest_segment_approach(path_end, path_end, obstacle.start, obstacle.end);
    let limit = moving_radius_mm + obstacle.radius_mm + CONTACT_EPSILON_MM;
    Ok(start.distance_mm <= limit || end.distance_mm <= limit)
}

/// Conservative swept-sphere AABB against capsule AABB; candidate only, not contact.
pub fn swept_sphere_capsule_aabb_candidate(
    path_start: Vec3,
    path_end: Vec3,
    moving_radius_mm: f64,
    obstacle: Capsule,
) -> Result<bool, CapsuleError> {
    validate_radius(moving_radius_mm)?;
    if !is_finite(path_start) || !is_finite(path_end) {
        return Err(CapsuleError::NonFiniteCoordinate);
    }
    let moving = CapsuleAabb3 {
        min: Vec3::new(
            path_start.x.min(path_end.x) - moving_radius_mm,
            path_start.y.min(path_end.y) - moving_radius_mm,
            path_start.z.min(path_end.z) - moving_radius_mm,
        ),
        max: Vec3::new(
            path_start.x.max(path_end.x) + moving_radius_mm,
            path_start.y.max(path_end.y) + moving_radius_mm,
            path_start.z.max(path_end.z) + moving_radius_mm,
        ),
    };
    Ok(moving.overlaps(obstacle.aabb()))
}

fn validate_radius(radius_mm: f64) -> Result<(), CapsuleError> {
    if radius_mm.is_finite() && radius_mm >= 0.0 {
        Ok(())
    } else {
        Err(CapsuleError::InvalidRadius)
    }
}

fn is_finite(p: Vec3) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.z.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    #[test]
    fn transverse_capsule_hit_between_clear_endpoints() {
        let obstacle = Capsule::new(p(0.0, -100.0, 0.0), p(0.0, 100.0, 0.0), 5.0).unwrap();
        let a = p(-1000.0, 0.0, 0.0);
        let b = p(1000.0, 0.0, 0.0);
        assert_eq!(
            moving_sphere_vs_capsule(a, b, 10.0, obstacle)
                .unwrap()
                .distance_mm,
            0.0
        );
        assert!(moving_sphere_capsule_collision(a, b, 10.0, obstacle).unwrap());
        assert!(!endpoints_only_sphere_capsule_collision(a, b, 10.0, obstacle).unwrap());
    }

    #[test]
    fn parallel_near_miss_outside_radius_sum_is_clear() {
        let obstacle = Capsule::new(p(20.0, 16.0, 0.0), p(80.0, 16.0, 0.0), 5.0).unwrap();
        let closest =
            moving_sphere_vs_capsule(p(0.0, 0.0, 0.0), p(100.0, 0.0, 0.0), 10.0, obstacle).unwrap();
        assert!((closest.distance_mm - 16.0).abs() < 1.0e-9);
        assert!(
            !moving_sphere_capsule_collision(p(0.0, 0.0, 0.0), p(100.0, 0.0, 0.0), 10.0, obstacle)
                .unwrap()
        );
    }

    #[test]
    fn exact_tangency_is_contact() {
        let obstacle = Capsule::new(p(20.0, 15.0, 0.0), p(80.0, 15.0, 0.0), 5.0).unwrap();
        assert!(
            moving_sphere_capsule_collision(p(0.0, 0.0, 0.0), p(100.0, 0.0, 0.0), 10.0, obstacle)
                .unwrap()
        );
    }

    #[test]
    fn skew_segments_use_three_dimensional_closest_approach() {
        let result = closest_segment_approach(
            p(0.0, 0.0, 0.0),
            p(10.0, 0.0, 0.0),
            p(5.0, -10.0, 3.0),
            p(5.0, 10.0, 3.0),
        );
        assert!((result.distance_mm - 3.0).abs() < 1.0e-9);
        assert!((result.moving_parameter - 0.5).abs() < 1.0e-9);
        assert!((result.capsule_parameter - 0.5).abs() < 1.0e-9);
    }

    #[test]
    fn parallel_collinear_and_overlapping_segments_have_zero_distance() {
        for (q0, q1) in [
            (p(2.0, 0.0, 0.0), p(8.0, 0.0, 0.0)),
            (p(12.0, 0.0, 0.0), p(18.0, 0.0, 0.0)),
            (p(-8.0, 0.0, 0.0), p(-2.0, 0.0, 0.0)),
        ] {
            assert!(
                closest_segment_approach(p(0.0, 0.0, 0.0), p(10.0, 0.0, 0.0), q0, q1).distance_mm
                    <= 1.0e-9
            );
        }
    }

    #[test]
    fn zero_length_capsule_axis_is_a_sphere() {
        let obstacle = Capsule::new(p(50.0, 8.0, 0.0), p(50.0, 8.0, 0.0), 6.0).unwrap();
        let closest =
            moving_sphere_vs_capsule(p(0.0, 0.0, 0.0), p(100.0, 0.0, 0.0), 2.0, obstacle).unwrap();
        assert!((closest.distance_mm - 8.0).abs() < 1.0e-9);
        assert!(
            moving_sphere_capsule_collision(p(0.0, 0.0, 0.0), p(100.0, 0.0, 0.0), 2.0, obstacle)
                .unwrap()
        );
    }

    #[test]
    fn zero_length_motion_is_point_vs_capsule() {
        let obstacle = Capsule::new(p(0.0, -5.0, 0.0), p(0.0, 5.0, 0.0), 2.0).unwrap();
        let point = p(0.0, 0.0, 0.0);
        assert!(moving_sphere_capsule_collision(point, point, 3.0, obstacle).unwrap());
    }

    #[test]
    fn broad_phase_can_be_conservative_false_positive() {
        let obstacle = Capsule::new(p(500.0, 15.0, 15.0), p(500.0, 15.0, 15.0), 5.0).unwrap();
        let a = p(0.0, 0.0, 0.0);
        let b = p(1000.0, 0.0, 0.0);
        assert!(swept_sphere_capsule_aabb_candidate(a, b, 10.0, obstacle).unwrap());
        assert!(!moving_sphere_capsule_collision(a, b, 10.0, obstacle).unwrap());
    }

    #[test]
    fn rigid_translation_and_quarter_turn_preserve_distance() {
        let a = p(-50.0, 0.0, 0.0);
        let b = p(50.0, 0.0, 0.0);
        let q0 = p(0.0, 0.0, 7.0);
        let q1 = p(0.0, 20.0, 7.0);
        let base = closest_segment_approach(a, b, q0, q1).distance_mm;
        let transform = |v: Vec3| p(100.0 - v.y, -30.0 + v.x, 25.0 + v.z);
        let moved =
            closest_segment_approach(transform(a), transform(b), transform(q0), transform(q1))
                .distance_mm;
        assert!((base - moved).abs() < 1.0e-9);
    }

    #[test]
    fn uniform_scaling_scales_distance_and_preserves_collision_truth() {
        let a = p(-100.0, 0.0, 0.0);
        let b = p(100.0, 0.0, 0.0);
        let obstacle = Capsule::new(p(0.0, 0.0, 30.0), p(0.0, 50.0, 30.0), 5.0).unwrap();
        let base = moving_sphere_vs_capsule(a, b, 10.0, obstacle).unwrap();
        let scaled_obstacle = Capsule::new(p(0.0, 0.0, 210.0), p(0.0, 350.0, 210.0), 35.0).unwrap();
        let scaled =
            moving_sphere_vs_capsule(a.scale(7.0), b.scale(7.0), 70.0, scaled_obstacle).unwrap();
        assert!((scaled.distance_mm - base.distance_mm * 7.0).abs() < 1.0e-8);
        assert_eq!(
            base.distance_mm <= 15.0 + CONTACT_EPSILON_MM,
            scaled.distance_mm <= 105.0 + CONTACT_EPSILON_MM
        );
    }

    #[test]
    fn endpoint_clamping_handles_capsule_beyond_path_end() {
        let result = closest_segment_approach(
            p(0.0, 0.0, 0.0),
            p(10.0, 0.0, 0.0),
            p(20.0, 5.0, 0.0),
            p(30.0, 5.0, 0.0),
        );
        assert_eq!(result.moving_point, p(10.0, 0.0, 0.0));
        assert_eq!(result.capsule_axis_point, p(20.0, 5.0, 0.0));
        assert!((result.distance_mm - 5.0_f64.hypot(10.0)).abs() < 1.0e-9);
    }

    #[test]
    fn rejects_negative_nonfinite_radii_and_nonfinite_coordinates() {
        assert_eq!(
            Capsule::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), -1.0),
            Err(CapsuleError::InvalidRadius)
        );
        assert_eq!(
            Capsule::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), f64::INFINITY),
            Err(CapsuleError::InvalidRadius)
        );
        let obstacle = Capsule::new(p(0.0, 0.0, 0.0), p(0.0, 1.0, 0.0), 1.0).unwrap();
        assert_eq!(
            moving_sphere_capsule_collision(p(f64::NAN, 0.0, 0.0), p(1.0, 0.0, 0.0), 1.0, obstacle),
            Err(CapsuleError::NonFiniteCoordinate)
        );
    }
}
