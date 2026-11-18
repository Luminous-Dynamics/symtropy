#![forbid(unsafe_code)]

//! Independent analytic references for the supported straight-segment and
//! upper-semicircle geometry families. This module does not call the envelope
//! producer's geometric expansion helpers.

pub mod field_layout_acceptance;
pub mod ifc_ids_preflight;
pub use ifc_ids_preflight::{
    IfcIdsPreflightError, IfcIdsPreflightReport, IfcIdsWorkPackageInput, IfcSourceEntity,
    IdsRequirementEvaluation, SourceEntityBinding, preflight_ifc_ids_work_package,
};
pub mod site_state_ledger;
pub mod cumulative_resource_capacity;
pub mod robust_capacity_profile;
pub use robust_capacity_profile::{
    CapacityAvailabilityWindow, RobustCapacityAssessment, RobustCapacityError,
    RobustCapacitySegment, RobustCumulativeCapacityReport,
    RobustCumulativeCapacityScenario, RobustResourceCapacitySummary,
    evaluate_robust_cumulative_resource_capacity,
};
pub use cumulative_resource_capacity::{
    CapacityDemandClaim, CapacityReservation, CapacityResourceDefinition,
    CapacityViolationSegment, CumulativeCapacityError, CumulativeCapacityReport,
    CumulativeCapacityScenario, ResourceCapacitySummary,
    evaluate_cumulative_resource_capacity,
};
pub mod dispatch_readiness;
pub use dispatch_readiness::{
    DispatchEnvelopeBinding, DispatchReadinessFailure, DispatchReadinessReceipt,
    DispatchReadinessRequest, OccupancyAssessmentReceipt,
    DISPATCH_READINESS_RECEIPT_SCHEMA_VERSION, DISPATCH_READINESS_REQUEST_SCHEMA_VERSION,
    check_dispatch_readiness, commit_dispatch_reservation, release_space_time_reservation,
    verify_dispatch_readiness_receipt_current,
};
pub use site_state_ledger::{
    ExclusiveResourceClaim, SiteResourceDefinition, SpaceTimeReservation,
    SpaceTimeReservationRecord,
};
pub use field_layout_acceptance::{
    CrewHours, DesignIntentEntity, DesignIntentManifest, EntityAcceptanceResult, EntityScoreReason,
    FieldAcceptanceError, FieldAcceptanceProfile, FieldAcceptanceScore, FieldEntityObservation,
    FieldLayoutEntityType, HumanWaiver, MeasurementDisposition, RefereeEntityState,
    RefereeMeasurement, RobotClaimedEntityState, score_field_layout_run,
};
pub use site_state_ledger::{
    RobotExecutionDisposition, SiteActorRole, SiteApplyOutcome, SiteEntityState, SiteEventEnvelope,
    SiteEventPayload, SiteLedgerError, SiteRefereeAssessment, SiteRejectionReason, SiteStateLedger,
    SiteWorkPackage,
};

use construction_motion_envelope::{
    AxisMm, Context, EvidenceRefs, MarginsMm, Point3Mm, Segment, build,
};

const COLLISION_EPSILON_MM: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }

    fn scale(self, scalar: f64) -> Self {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }

    fn dot(self, rhs: Self) -> f64 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    fn norm_squared(self) -> f64 {
        self.dot(self)
    }

    fn distance(self, rhs: Self) -> f64 {
        self.sub(rhs).norm_squared().sqrt()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb3 {
    pub fn contains(&self, other: Self) -> bool {
        self.min.x <= other.min.x
            && self.min.y <= other.min.y
            && self.min.z <= other.min.z
            && self.max.x >= other.max.x
            && self.max.y >= other.max.y
            && self.max.z >= other.max.z
    }

    pub fn overlaps(&self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    pub fn expand(&self, margin: Vec3) -> Self {
        Self {
            min: Vec3::new(
                self.min.x - margin.x,
                self.min.y - margin.y,
                self.min.z - margin.z,
            ),
            max: Vec3::new(
                self.max.x + margin.x,
                self.max.y + margin.y,
                self.max.z + margin.z,
            ),
        }
    }

    pub fn volume(&self) -> f64 {
        let dx = (self.max.x - self.min.x).max(0.0);
        let dy = (self.max.y - self.min.y).max(0.0);
        let dz = (self.max.z - self.min.z).max(0.0);
        dx * dy * dz
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnalyticPath {
    Linear {
        start: Vec3,
        end: Vec3,
    },
    /// Upper semicircle in the XY plane, centered at (0, 0, z_mm).
    /// It runs from (-radius, 0, z) through (0, radius, z) to (+radius, 0, z).
    UpperSemicircle {
        radius_mm: f64,
        z_mm: f64,
    },
}

impl AnalyticPath {
    pub fn endpoints(&self) -> (Vec3, Vec3) {
        match *self {
            Self::Linear { start, end } => (start, end),
            Self::UpperSemicircle { radius_mm, z_mm } => (
                Vec3::new(-radius_mm, 0.0, z_mm),
                Vec3::new(radius_mm, 0.0, z_mm),
            ),
        }
    }

    /// Exact coordinate extrema for the declared path family, before body radius.
    pub fn exact_center_aabb(&self) -> Aabb3 {
        match *self {
            Self::Linear { start, end } => Aabb3 {
                min: Vec3::new(start.x.min(end.x), start.y.min(end.y), start.z.min(end.z)),
                max: Vec3::new(start.x.max(end.x), start.y.max(end.y), start.z.max(end.z)),
            },
            Self::UpperSemicircle { radius_mm, z_mm } => Aabb3 {
                min: Vec3::new(-radius_mm, 0.0, z_mm),
                max: Vec3::new(radius_mm, radius_mm, z_mm),
            },
        }
    }

    /// Exact minimum centerline distance to a stationary point for the supported path.
    pub fn exact_distance_to_point(&self, point: Vec3) -> f64 {
        match *self {
            Self::Linear { start, end } => {
                let direction = end.sub(start);
                let denominator = direction.norm_squared();
                if denominator <= f64::EPSILON {
                    return start.distance(point);
                }
                let parameter = point.sub(start).dot(direction) / denominator;
                let clamped = parameter.clamp(0.0, 1.0);
                start.add(direction.scale(clamped)).distance(point)
            }
            Self::UpperSemicircle { radius_mm, z_mm } => {
                let angle = point.y.atan2(point.x).clamp(0.0, std::f64::consts::PI);
                let projected = Vec3::new(radius_mm * angle.cos(), radius_mm * angle.sin(), z_mm);
                let start = Vec3::new(-radius_mm, 0.0, z_mm);
                let end = Vec3::new(radius_mm, 0.0, z_mm);
                projected
                    .distance(point)
                    .min(start.distance(point))
                    .min(end.distance(point))
            }
        }
    }

    /// Exact collision truth for one moving sphere against one stationary sphere.
    pub fn collides_with_stationary_sphere(
        &self,
        moving_radius_mm: f64,
        obstacle_center: Vec3,
        obstacle_radius_mm: f64,
    ) -> bool {
        self.exact_distance_to_point(obstacle_center)
            <= moving_radius_mm + obstacle_radius_mm + COLLISION_EPSILON_MM
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalMargins {
    pub localization_mm: Vec3,
    pub tracking_mm: Vec3,
    pub model_mm: Vec3,
    pub numerical_mm: Vec3,
}

impl PhysicalMargins {
    pub fn total(self) -> Vec3 {
        Vec3::new(
            self.localization_mm.x + self.tracking_mm.x + self.model_mm.x + self.numerical_mm.x,
            self.localization_mm.y + self.tracking_mm.y + self.model_mm.y + self.numerical_mm.y,
            self.localization_mm.z + self.tracking_mm.z + self.model_mm.z + self.numerical_mm.z,
        )
    }
}

/// Independent expected physical outer AABB: exact analytic path extrema expanded
/// by the body sphere and explicitly stated physical uncertainty channels.
pub fn analytic_physical_outer(
    path: AnalyticPath,
    body_radius_mm: f64,
    margins: PhysicalMargins,
) -> Aabb3 {
    path.exact_center_aabb()
        .expand(Vec3::new(body_radius_mm, body_radius_mm, body_radius_mm))
        .expand(margins.total())
}

pub fn sphere_aabb(center: Vec3, radius_mm: f64) -> Aabb3 {
    Aabb3 {
        min: Vec3::new(
            center.x - radius_mm,
            center.y - radius_mm,
            center.z - radius_mm,
        ),
        max: Vec3::new(
            center.x + radius_mm,
            center.y + radius_mm,
            center.z + radius_mm,
        ),
    }
}

/// Endpoint-only baseline: true only if an endpoint sphere overlaps the obstacle.
pub fn endpoints_only_collision(
    path: AnalyticPath,
    moving_radius_mm: f64,
    obstacle_center: Vec3,
    obstacle_radius_mm: f64,
) -> bool {
    let (start, end) = path.endpoints();
    start.distance(obstacle_center) <= moving_radius_mm + obstacle_radius_mm
        || end.distance(obstacle_center) <= moving_radius_mm + obstacle_radius_mm
}

/// Straight-chord CCD baseline. For a curved trajectory this intentionally checks
/// only the endpoint chord, exposing the error made by substituting chord motion for arc motion.
pub fn chord_collision(
    path: AnalyticPath,
    moving_radius_mm: f64,
    obstacle_center: Vec3,
    obstacle_radius_mm: f64,
) -> bool {
    let (start, end) = path.endpoints();
    AnalyticPath::Linear { start, end }.collides_with_stationary_sphere(
        moving_radius_mm,
        obstacle_center,
        obstacle_radius_mm,
    )
}

/// Broad-phase AABB candidate using a conservative sphere AABB for the obstacle.
pub fn envelope_broad_phase_candidate(
    envelope: Aabb3,
    obstacle_center: Vec3,
    obstacle_radius_mm: f64,
) -> bool {
    envelope.overlaps(sphere_aabb(obstacle_center, obstacle_radius_mm))
}

/// Invoke the production v0.1 builder with an admitted primitive segment.
/// Reference extrema and collision truth are derived independently above.
pub fn build_kernel_slice(
    path: AnalyticPath,
    primitive_id: &str,
    radius_mm: u64,
    path_deviation_mm: AxisMm,
    other_physical_margins_mm: AxisMm,
    stopping_mm: AxisMm,
    policy_mm: AxisMm,
) -> construction_motion_envelope::EnvelopeSlice {
    let (start, end) = path.endpoints();
    let context = Context {
        project_id: "oracle-synthetic-project".into(),
        work_package_id: "oracle-work-package".into(),
        task_id: "oracle-task".into(),
        trajectory_id: "oracle-trajectory".into(),
        robot_id: "oracle-robot".into(),
        platform_config_sha256: "a".repeat(64),
        tool_config_sha256: "b".repeat(64),
        payload_config_sha256: "c".repeat(64),
        collision_model_sha256: "d".repeat(64),
        primitive_set_ref: "synthetic://oracle/primitive-manifest".into(),
        expected_primitive_ids: vec![primitive_id.to_owned()],
        calibration_ref: "synthetic://oracle/calibration".into(),
        controller_profile_ref: "synthetic://oracle/controller".into(),
        frame_id: "oracle-local-frame".into(),
        transform_sha256: "e".repeat(64),
        design_revision_sha256: "f".repeat(64),
        site_epoch: "oracle-site-epoch".into(),
        clock_domain_id: "oracle-monotonic-clock".into(),
        trajectory_profile_sha256: "1".repeat(64),
    };
    let evidence = EvidenceRefs {
        path_deviation: "synthetic://oracle/path-deviation".into(),
        localization: "synthetic://oracle/localization".into(),
        tracking: "synthetic://oracle/tracking".into(),
        model_uncertainty: "synthetic://oracle/model".into(),
        numerical_enclosure: "synthetic://oracle/numerical".into(),
        stopping_reachability: "synthetic://oracle/stopping".into(),
        policy_clearance: "synthetic://oracle/policy".into(),
    };
    let segment = Segment {
        slice_index: 0,
        primitive_id: primitive_id.into(),
        start_ms: 0,
        end_ms: 100,
        from_mm: point_to_mm(start),
        to_mm: point_to_mm(end),
        radius_mm,
        margins: MarginsMm {
            path_deviation: path_deviation_mm,
            localization: other_physical_margins_mm,
            tracking: AxisMm::default(),
            model_uncertainty: AxisMm::default(),
            numerical_enclosure: AxisMm::default(),
            stopping_reachability: stopping_mm,
            policy_clearance: policy_mm,
        },
        evidence,
    };
    build(&context, &[segment])
        .expect("frozen analytic fixture must satisfy the v0.1 input contract")
        .slices
        .into_iter()
        .next()
        .expect("one primitive and one time cell produce one slice")
}

fn point_to_mm(point: Vec3) -> Point3Mm {
    Point3Mm {
        x: point.x.round() as i64,
        y: point.y.round() as i64,
        z: point.z.round() as i64,
    }
}

pub fn aabb_from_kernel(aabb: construction_motion_envelope::AabbMm) -> Aabb3 {
    Aabb3 {
        min: Vec3::new(aabb.min.x as f64, aabb.min.y as f64, aabb.min.z as f64),
        max: Vec3::new(aabb.max.x as f64, aabb.max.y as f64, aabb.max.z as f64),
    }
}

pub fn axis(x: u64, y: u64, z: u64) -> AxisMm {
    AxisMm { x, y, z }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero() -> AxisMm {
        AxisMm::default()
    }

    #[test]
    fn straight_segment_envelope_contains_independent_analytic_reference() {
        let path = AnalyticPath::Linear {
            start: Vec3::new(-500.0, 20.0, 10.0),
            end: Vec3::new(700.0, 80.0, 30.0),
        };
        let physical_margins = PhysicalMargins {
            localization_mm: Vec3::new(4.0, 5.0, 6.0),
            tracking_mm: Vec3::new(3.0, 2.0, 1.0),
            model_mm: Vec3::new(2.0, 2.0, 2.0),
            numerical_mm: Vec3::new(1.0, 1.0, 1.0),
        };
        let expected = analytic_physical_outer(path, 35.0, physical_margins);
        let slice = build_kernel_slice(path, "tool", 35, zero(), axis(10, 10, 10), zero(), zero());
        assert!(aabb_from_kernel(slice.physical_swept_outer).contains(expected));
    }

    #[test]
    fn exact_semicircle_oracle_detects_understated_chord_deviation() {
        let path = AnalyticPath::UpperSemicircle {
            radius_mm: 1_000.0,
            z_mm: 0.0,
        };
        let expected = analytic_physical_outer(
            path,
            25.0,
            PhysicalMargins {
                localization_mm: Vec3::new(0.0, 0.0, 0.0),
                tracking_mm: Vec3::new(0.0, 0.0, 0.0),
                model_mm: Vec3::new(0.0, 0.0, 0.0),
                numerical_mm: Vec3::new(0.0, 0.0, 0.0),
            },
        );
        let missing_arc_bound =
            build_kernel_slice(path, "tool", 25, zero(), zero(), zero(), zero());
        assert!(!aabb_from_kernel(missing_arc_bound.physical_swept_outer).contains(expected));

        let qualified_claim =
            build_kernel_slice(path, "tool", 25, axis(0, 1_000, 0), zero(), zero(), zero());
        assert!(aabb_from_kernel(qualified_claim.physical_swept_outer).contains(expected));
    }

    #[test]
    fn exact_arc_collision_is_missed_by_endpoint_and_chord_baselines() {
        let path = AnalyticPath::UpperSemicircle {
            radius_mm: 1_000.0,
            z_mm: 0.0,
        };
        let obstacle = Vec3::new(0.0, 1_000.0, 0.0);
        let truth = path.collides_with_stationary_sphere(20.0, obstacle, 20.0);
        assert!(truth);
        assert!(!endpoints_only_collision(path, 20.0, obstacle, 20.0));
        assert!(!chord_collision(path, 20.0, obstacle, 20.0));
    }

    #[test]
    fn straight_thin_barrier_is_missed_by_endpoints_but_found_by_exact_ccd() {
        let path = AnalyticPath::Linear {
            start: Vec3::new(-1_000.0, 0.0, 0.0),
            end: Vec3::new(1_000.0, 0.0, 0.0),
        };
        let obstacle = Vec3::new(0.0, 0.0, 0.0);
        assert!(path.collides_with_stationary_sphere(10.0, obstacle, 5.0));
        assert!(!endpoints_only_collision(path, 10.0, obstacle, 5.0));
        assert!(chord_collision(path, 10.0, obstacle, 5.0));
    }

    #[test]
    fn aabb_broad_phase_is_conservative_but_can_report_false_positive() {
        let path = AnalyticPath::UpperSemicircle {
            radius_mm: 1_000.0,
            z_mm: 0.0,
        };
        let slice = build_kernel_slice(path, "tool", 20, axis(0, 1_000, 0), zero(), zero(), zero());
        let envelope = aabb_from_kernel(slice.physical_swept_outer);
        let obstacle = Vec3::new(1_015.0, 1_015.0, 0.0);
        assert!(envelope_broad_phase_candidate(envelope, obstacle, 10.0));
        assert!(!path.collides_with_stationary_sphere(20.0, obstacle, 10.0));
    }

    #[test]
    fn stopping_and_policy_expansions_are_not_physical_collision_truth() {
        let path = AnalyticPath::Linear {
            start: Vec3::new(-100.0, 0.0, 0.0),
            end: Vec3::new(100.0, 0.0, 0.0),
        };
        let slice = build_kernel_slice(
            path,
            "tool",
            10,
            zero(),
            zero(),
            axis(300, 0, 0),
            axis(100, 0, 0),
        );
        assert_eq!(slice.physical_swept_outer.min.x, -110);
        assert_eq!(slice.stopping_outer.min.x, -410);
        assert_eq!(slice.coordination_outer.min.x, -510);
        let obstacle = Vec3::new(-450.0, 0.0, 0.0);
        assert!(!path.collides_with_stationary_sphere(10.0, obstacle, 10.0));
        assert!(envelope_broad_phase_candidate(
            aabb_from_kernel(slice.coordination_outer),
            obstacle,
            10.0
        ));
    }

    #[test]
    fn increasing_uncertainty_never_shrinks_the_envelope() {
        let path = AnalyticPath::Linear {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(200.0, 0.0, 0.0),
        };
        let tight = build_kernel_slice(path, "tool", 10, zero(), zero(), zero(), zero());
        let wide = build_kernel_slice(path, "tool", 10, zero(), axis(30, 20, 10), zero(), zero());
        assert!(
            aabb_from_kernel(wide.physical_swept_outer)
                .contains(aabb_from_kernel(tight.physical_swept_outer))
        );
    }

    #[test]
    fn uniform_scale_preserves_collision_truth_and_scales_distance() {
        let path = AnalyticPath::Linear {
            start: Vec3::new(-100.0, 0.0, 0.0),
            end: Vec3::new(100.0, 0.0, 0.0),
        };
        let obstacle = Vec3::new(0.0, 25.0, 0.0);
        let first_distance = path.exact_distance_to_point(obstacle);
        let scaled = AnalyticPath::Linear {
            start: Vec3::new(-700.0, 0.0, 0.0),
            end: Vec3::new(700.0, 0.0, 0.0),
        };
        let scaled_obstacle = Vec3::new(0.0, 175.0, 0.0);
        let second_distance = scaled.exact_distance_to_point(scaled_obstacle);
        assert!((second_distance - first_distance * 7.0).abs() <= COLLISION_EPSILON_MM);
        assert_eq!(
            path.collides_with_stationary_sphere(20.0, obstacle, 10.0),
            scaled.collides_with_stationary_sphere(140.0, scaled_obstacle, 70.0)
        );
    }
    #[test]
    fn rigid_translation_and_quarter_turn_preserve_collision_truth() {
        let path = AnalyticPath::Linear {
            start: Vec3::new(-100.0, 0.0, 0.0),
            end: Vec3::new(100.0, 0.0, 0.0),
        };
        let obstacle = Vec3::new(0.0, 15.0, 0.0);
        let expected = path.collides_with_stationary_sphere(10.0, obstacle, 5.0);

        // Apply the same exact 90-degree rotation to path and obstacle.
        let rotated = AnalyticPath::Linear {
            start: Vec3::new(0.0, -100.0, 0.0),
            end: Vec3::new(0.0, 100.0, 0.0),
        };
        let rotated_obstacle = Vec3::new(-15.0, 0.0, 0.0);
        assert_eq!(
            expected,
            rotated.collides_with_stationary_sphere(10.0, rotated_obstacle, 5.0)
        );

        // A common translation must also preserve the exact answer.
        let translated = AnalyticPath::Linear {
            start: Vec3::new(600.0, -300.0, 50.0),
            end: Vec3::new(800.0, -300.0, 50.0),
        };
        let translated_obstacle = Vec3::new(700.0, -285.0, 50.0);
        assert_eq!(
            expected,
            translated.collides_with_stationary_sphere(10.0, translated_obstacle, 5.0)
        );
    }

    #[test]
    fn subdividing_a_linear_path_does_not_shrink_the_outer_union() {
        let whole_path = AnalyticPath::Linear {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(1_000.0, 400.0, 100.0),
        };
        let whole = aabb_from_kernel(
            build_kernel_slice(
                whole_path,
                "tool",
                50,
                axis(5, 5, 5),
                zero(),
                zero(),
                zero(),
            )
            .physical_swept_outer,
        );

        let mut partials = Vec::new();
        for index in 0..4 {
            let start_fraction = index as f64 / 4.0;
            let end_fraction = (index + 1) as f64 / 4.0;
            let start = Vec3::new(
                1_000.0 * start_fraction,
                400.0 * start_fraction,
                100.0 * start_fraction,
            );
            let end = Vec3::new(
                1_000.0 * end_fraction,
                400.0 * end_fraction,
                100.0 * end_fraction,
            );
            let partial = build_kernel_slice(
                AnalyticPath::Linear { start, end },
                "tool",
                50,
                axis(5, 5, 5),
                zero(),
                zero(),
                zero(),
            );
            partials.push(aabb_from_kernel(partial.physical_swept_outer));
        }

        let union = Aabb3 {
            min: Vec3::new(
                partials
                    .iter()
                    .map(|item| item.min.x)
                    .fold(f64::INFINITY, f64::min),
                partials
                    .iter()
                    .map(|item| item.min.y)
                    .fold(f64::INFINITY, f64::min),
                partials
                    .iter()
                    .map(|item| item.min.z)
                    .fold(f64::INFINITY, f64::min),
            ),
            max: Vec3::new(
                partials
                    .iter()
                    .map(|item| item.max.x)
                    .fold(f64::NEG_INFINITY, f64::max),
                partials
                    .iter()
                    .map(|item| item.max.y)
                    .fold(f64::NEG_INFINITY, f64::max),
                partials
                    .iter()
                    .map(|item| item.max.z)
                    .fold(f64::NEG_INFINITY, f64::max),
            ),
        };
        assert_eq!(whole, union);
    }
}
