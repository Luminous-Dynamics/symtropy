// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic directed point-to-observable-surface accuracy primitives.
//!
//! This module makes correspondence semantics explicit rather than treating
//! every point-to-surface metric as interchangeable.
//!
//! - EuclideanClosestPointOnTriangle computes the true closest point on the
//!   closed triangle (including edges and vertices). This matches the current
//!   ISPRS results-page wording: reference samples to the closest submitted
//!   source-model surface.
//! - OrthogonalProjectionInsideTriangle computes perpendicular distance to a
//!   target plane only when the orthogonal projection falls inside that
//!   triangle. This is the separately named 2018 evaluation-framework rule.
//!
//! Sample generation, tessellation, polygon-with-holes handling, visibility,
//! registration and official area metrics remain caller responsibilities.

const BARYCENTRIC_EPSILON: f64 = 1.0e-10;
const DEGENERATE_AREA_EPSILON: f64 = 1.0e-15;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    fn scale(self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    fn norm(self) -> f64 {
        self.x.hypot(self.y).hypot(self.z)
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

/// A planar triangle in the shared coordinate frame, with an explicit
/// observability flag. Unobservable triangles are never correspondence targets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceTriangle {
    pub a: Point3,
    pub b: Point3,
    pub c: Point3,
    pub observable: bool,
}

impl SurfaceTriangle {
    pub const fn new(a: Point3, b: Point3, c: Point3, observable: bool) -> Self {
        Self {
            a,
            b,
            c,
            observable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccuracyDirection {
    ReferenceToCandidate,
    CandidateToReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccuracySamplePopulation {
    ReferenceSurfaceSamples,
    SourceSurfaceVertices,
    SourceSurfaceSamples,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrespondencePolicy {
    EuclideanClosestPointOnTriangle,
    OrthogonalProjectionInsideTriangle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccuracyCurvePoint {
    pub cutoff_m: f64,
    /// Median of accepted distances at this cutoff. None when no sample
    /// has an eligible correspondence within the cutoff.
    pub median_distance_m: Option<f64>,
    pub accepted_sample_count: usize,
    /// Samples with a valid surface correspondence but distance > cutoff.
    /// Samples with no eligible boundary-contained projection are reported
    /// separately at result level.
    pub rejected_above_cutoff_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DirectedAccuracyResult {
    pub direction: AccuracyDirection,
    pub sample_population: AccuracySamplePopulation,
    pub correspondence_policy: CorrespondencePolicy,
    pub sample_count: usize,
    pub correspondence_count: usize,
    pub no_correspondence_count: usize,
    /// The nearest eligible absolute perpendicular distance for each sample,
    /// in input order. None means no observable target surface accepted the
    /// orthogonal projection within its boundary.
    pub sample_distances_m: Vec<Option<f64>>,
    pub curve: Vec<AccuracyCurvePoint>,
}

/// Evaluate one explicitly named directed accuracy distribution.
///
/// All coordinates and cutoffs are in metres in one already-aligned frame.
/// Target triangle visibility and the sample population must be established by
/// upstream, independently controlled evidence. This function does not
/// estimate registration, sample surfaces, generate meshes, or infer
/// observability.
///
/// For ReferenceToCandidate the samples must be reference surface samples and
/// target_surfaces must be candidate-model triangles. The official results-page
/// profile uses EuclideanClosestPointOnTriangle.
/// For CandidateToReference the samples must be source vertices or source-surface
/// samples and target_surfaces must be observable reference triangles. The
/// 2018 evaluation-framework profile uses OrthogonalProjectionInsideTriangle.
//////
/// The selected policy is returned unchanged in the result. This function does
/// not infer a policy from a metric name.
pub fn evaluate_directed_accuracy_curve(
    direction: AccuracyDirection,
    sample_population: AccuracySamplePopulation,
    correspondence_policy: CorrespondencePolicy,
    samples: &[Point3],
    target_surfaces: &[SurfaceTriangle],
    cutoffs_m: &[f64],
) -> Result<DirectedAccuracyResult, String> {
    if samples.is_empty() {
        return Err("accuracy sample population must not be empty".to_owned());
    }
    if target_surfaces.is_empty() {
        return Err("target surface set must not be empty".to_owned());
    }
    if cutoffs_m.is_empty() {
        return Err("at least one accuracy cutoff is required".to_owned());
    }
    if direction == AccuracyDirection::ReferenceToCandidate
        && sample_population != AccuracySamplePopulation::ReferenceSurfaceSamples
    {
        return Err("reference_to_candidate requires reference_surface_samples".to_owned());
    }
    if direction == AccuracyDirection::CandidateToReference
        && sample_population == AccuracySamplePopulation::ReferenceSurfaceSamples
    {
        return Err(
            "candidate_to_reference requires source_surface_vertices or source_surface_samples"
                .to_owned(),
        );
    }

    let mut previous_cutoff = 0.0;
    for cutoff in cutoffs_m {
        if !cutoff.is_finite() || *cutoff <= 0.0 {
            return Err("accuracy cutoffs must be finite positive metres".to_owned());
        }
        if *cutoff <= previous_cutoff {
            return Err("accuracy cutoffs must be strictly increasing".to_owned());
        }
        previous_cutoff = *cutoff;
    }

    for (index, sample) in samples.iter().enumerate() {
        if !sample.is_finite() {
            return Err(format!("sample {index} contains a non-finite coordinate"));
        }
    }

    // Validate every declared triangle, including unobservable ones, so a
    // malformed reference cannot hide behind a visibility flag.
    let mut prepared = Vec::with_capacity(target_surfaces.len());
    for (index, triangle) in target_surfaces.iter().enumerate() {
        if !triangle.a.is_finite() || !triangle.b.is_finite() || !triangle.c.is_finite() {
            return Err(format!(
                "target triangle {index} contains a non-finite coordinate"
            ));
        }
        let ab = triangle.b.sub(triangle.a);
        let ac = triangle.c.sub(triangle.a);
        let cross = ab.cross(ac);
        let twice_area = cross.norm();
        if !twice_area.is_finite() || twice_area <= DEGENERATE_AREA_EPSILON {
            return Err(format!("target triangle {index} is degenerate"));
        }
        let normal = cross.scale(1.0 / twice_area);
        prepared.push((*triangle, normal, ab, ac));
    }

    let mut sample_distances_m = Vec::with_capacity(samples.len());
    let mut correspondence_count = 0;

    for sample in samples {
        let mut best_distance: Option<f64> = None;

        for (triangle, normal, ab, ac) in &prepared {
            if !triangle.observable {
                continue;
            }

            let distance = match correspondence_policy {
                CorrespondencePolicy::EuclideanClosestPointOnTriangle => Some(
                    sample
                        .sub(closest_point_on_triangle(
                            *sample, triangle.a, triangle.b, triangle.c,
                        ))
                        .norm(),
                ),
                CorrespondencePolicy::OrthogonalProjectionInsideTriangle => {
                    let signed_distance = sample.sub(triangle.a).dot(*normal);
                    let projection = sample.sub(normal.scale(signed_distance));
                    projection_inside_triangle(projection, triangle.a, *ab, *ac)
                        .then_some(signed_distance.abs())
                }
            };

            if let Some(distance) = distance {
                if !distance.is_finite() {
                    return Err(
                        "point-to-surface distance overflowed or became non-finite".to_owned()
                    );
                }
                if best_distance.is_none_or(|best| distance < best) {
                    best_distance = Some(distance);
                }
            }
        }

        if best_distance.is_some() {
            correspondence_count += 1;
        }
        sample_distances_m.push(best_distance);
    }

    let no_correspondence_count = samples.len() - correspondence_count;
    let mut curve = Vec::with_capacity(cutoffs_m.len());

    for cutoff in cutoffs_m {
        let mut accepted: Vec<f64> = sample_distances_m
            .iter()
            .flatten()
            .copied()
            .filter(|distance| *distance <= *cutoff)
            .collect();
        accepted.sort_by(f64::total_cmp);

        let accepted_sample_count = accepted.len();
        let rejected_above_cutoff_count = correspondence_count - accepted_sample_count;
        let median_distance_m = median_sorted(&accepted);

        curve.push(AccuracyCurvePoint {
            cutoff_m: *cutoff,
            median_distance_m,
            accepted_sample_count,
            rejected_above_cutoff_count,
        });
    }

    Ok(DirectedAccuracyResult {
        direction,
        sample_population,
        correspondence_policy,
        sample_count: samples.len(),
        correspondence_count,
        no_correspondence_count,
        sample_distances_m,
        curve,
    })
}

fn projection_inside_triangle(point: Point3, origin: Point3, ab: Point3, ac: Point3) -> bool {
    let ap = point.sub(origin);
    let d00 = ab.dot(ab);
    let d01 = ab.dot(ac);
    let d11 = ac.dot(ac);
    let d20 = ap.dot(ab);
    let d21 = ap.dot(ac);
    let denominator = d00 * d11 - d01 * d01;

    if !denominator.is_finite() || denominator <= DEGENERATE_AREA_EPSILON {
        return false;
    }

    let v = (d11 * d20 - d01 * d21) / denominator;
    let w = (d00 * d21 - d01 * d20) / denominator;
    let u = 1.0 - v - w;

    u >= -BARYCENTRIC_EPSILON && v >= -BARYCENTRIC_EPSILON && w >= -BARYCENTRIC_EPSILON
}

fn closest_point_on_triangle(point: Point3, a: Point3, b: Point3, c: Point3) -> Point3 {
    // Ericson's closest-point region tests. All regions include the closed
    // triangle boundary, unlike the 2018 plane-projection correspondence.
    let ab = b.sub(a);
    let ac = c.sub(a);
    let ap = point.sub(a);
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = point.sub(b);
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return a.add(ab.scale(v));
    }

    let cp = point.sub(c);
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a.add(ac.scale(w));
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b.add(c.sub(b).scale(w));
    }

    let denominator = va + vb + vc;
    let inverse = 1.0 / denominator;
    let v = vb * inverse;
    let w = vc * inverse;
    a.add(ab.scale(v)).add(ac.scale(w))
}

fn median_sorted(values: &[f64]) -> Option<f64> {
    match values.len() {
        0 => None,
        count if count % 2 == 1 => Some(values[count / 2]),
        count => {
            let upper = count / 2;
            Some(values[upper - 1] * 0.5 + values[upper] * 0.5)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AccuracyDirection, AccuracySamplePopulation, CorrespondencePolicy, Point3, SurfaceTriangle,
        evaluate_directed_accuracy_curve,
    };

    fn plane(z: f64, observable: bool) -> SurfaceTriangle {
        SurfaceTriangle::new(
            Point3::new(0.0, 0.0, z),
            Point3::new(2.0, 0.0, z),
            Point3::new(0.0, 2.0, z),
            observable,
        )
    }

    #[test]
    fn euclidean_closest_point_accepts_boundary_projection_and_uses_true_surface_distance() {
        // The plane projection is outside the triangle (x + y > 2), but the
        // closest point on the closed triangle lies on edge BC.
        let samples = [Point3::new(1.5, 1.0, 0.01)];
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::ReferenceToCandidate,
            AccuracySamplePopulation::ReferenceSurfaceSamples,
            CorrespondencePolicy::EuclideanClosestPointOnTriangle,
            &samples,
            &[plane(0.0, true)],
            &[0.35, 0.36],
        )
        .expect("the triangle boundary is part of the submitted surface");

        assert_eq!(result.correspondence_count, 1);
        let distance = result.sample_distances_m[0].expect("closest surface point");
        assert!((distance - (0.1251_f64).sqrt()).abs() < 1e-10);
        assert_eq!(result.curve[0].accepted_sample_count, 0);
        assert_eq!(result.curve[1].accepted_sample_count, 1);
        assert_eq!(
            result.correspondence_policy,
            CorrespondencePolicy::EuclideanClosestPointOnTriangle
        );
    }

    #[test]
    fn euclidean_closest_point_inside_face_matches_normal_offset() {
        let samples = [Point3::new(0.25, 0.25, 0.02)];
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::ReferenceToCandidate,
            AccuracySamplePopulation::ReferenceSurfaceSamples,
            CorrespondencePolicy::EuclideanClosestPointOnTriangle,
            &samples,
            &[plane(0.0, true)],
            &[0.03],
        )
        .expect("interior closest point should reduce to perpendicular distance");
        assert!((result.sample_distances_m[0].expect("distance") - 0.02).abs() < 1e-12);
    }

    #[test]
    fn orthogonal_projection_policy_keeps_the_2018_boundary_rejection() {
        let samples = [Point3::new(1.5, 1.0, 0.01)];
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::CandidateToReference,
            AccuracySamplePopulation::SourceSurfaceVertices,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &samples,
            &[plane(0.0, true)],
            &[0.1],
        )
        .expect("boundary-external plane projection is not a valid 2018 correspondence");
        assert_eq!(result.correspondence_count, 0);
        assert_eq!(result.no_correspondence_count, 1);
    }

    #[test]
    fn euclidean_policy_still_excludes_unobservable_target_triangles() {
        let samples = [Point3::new(0.25, 0.25, 0.01)];
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::ReferenceToCandidate,
            AccuracySamplePopulation::ReferenceSurfaceSamples,
            CorrespondencePolicy::EuclideanClosestPointOnTriangle,
            &samples,
            &[plane(0.0, false)],
            &[0.1],
        )
        .expect("unobservable targets must not establish a correspondence");
        assert_eq!(result.correspondence_count, 0);
        assert_eq!(result.no_correspondence_count, 1);
    }

    #[test]
    fn rejects_closest_point_distance_overflow() {
        // Finite coordinates may still overflow intermediate arithmetic.
        let samples = [Point3::new(f64::MAX, 0.0, 0.0)];
        let huge = SurfaceTriangle::new(
            Point3::new(-f64::MAX, 0.0, 0.0),
            Point3::new(-f64::MAX, 1.0, 0.0),
            Point3::new(-f64::MAX, 0.0, 1.0),
            true,
        );
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::ReferenceToCandidate,
            AccuracySamplePopulation::ReferenceSurfaceSamples,
            CorrespondencePolicy::EuclideanClosestPointOnTriangle,
            &samples,
            &[huge],
            &[1.0],
        );
        assert!(result.is_err());
    }

    #[test]
    fn computes_reference_to_candidate_perpendicular_distance() {
        let samples = [Point3::new(0.25, 0.25, 0.02)];
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::ReferenceToCandidate,
            AccuracySamplePopulation::ReferenceSurfaceSamples,
            CorrespondencePolicy::EuclideanClosestPointOnTriangle,
            &samples,
            &[plane(0.0, true)],
            &[0.01, 0.03],
        )
        .expect("parallel planar correspondence should be valid");

        assert_eq!(result.correspondence_count, 1);
        assert_eq!(result.no_correspondence_count, 0);
        assert!((result.sample_distances_m[0].expect("correspondence") - 0.02).abs() < 1e-12);
        assert_eq!(result.curve[0].accepted_sample_count, 0);
        assert_eq!(result.curve[0].median_distance_m, None);
        assert_eq!(result.curve[1].accepted_sample_count, 1);
        assert!((result.curve[1].median_distance_m.expect("median") - 0.02).abs() < 1e-12);
    }

    #[test]
    fn excludes_orthogonal_projection_outside_reference_boundary() {
        let samples = [Point3::new(1.5, 1.0, 0.01)];
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::CandidateToReference,
            AccuracySamplePopulation::SourceSurfaceVertices,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &samples,
            &[plane(0.0, true)],
            &[0.1],
        )
        .expect("an out-of-bound projection is a no-correspondence result");

        assert_eq!(result.correspondence_count, 0);
        assert_eq!(result.no_correspondence_count, 1);
        assert_eq!(result.sample_distances_m, vec![None]);
        assert_eq!(result.curve[0].accepted_sample_count, 0);
    }

    #[test]
    fn ignores_unobservable_target_surfaces() {
        let samples = [Point3::new(0.2, 0.2, 0.01)];
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::CandidateToReference,
            AccuracySamplePopulation::SourceSurfaceSamples,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &samples,
            &[plane(0.0, false)],
            &[0.1],
        )
        .expect("unobservable surfaces cannot establish a correspondence");

        assert_eq!(result.correspondence_count, 0);
        assert_eq!(result.no_correspondence_count, 1);
    }

    #[test]
    fn produces_cutoff_dependent_median_curve() {
        let samples = [
            Point3::new(0.2, 0.2, 0.01),
            Point3::new(0.4, 0.2, 0.05),
            Point3::new(0.6, 0.2, 0.20),
        ];
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::CandidateToReference,
            AccuracySamplePopulation::SourceSurfaceSamples,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &samples,
            &[plane(0.0, true)],
            &[0.02, 0.10, 0.25],
        )
        .expect("cutoff curve should be computable");

        assert_eq!(result.curve[0].accepted_sample_count, 1);
        assert!((result.curve[0].median_distance_m.expect("median") - 0.01).abs() < 1e-12);
        assert_eq!(result.curve[1].accepted_sample_count, 2);
        assert!((result.curve[1].median_distance_m.expect("median") - 0.03).abs() < 1e-12);
        assert_eq!(result.curve[2].accepted_sample_count, 3);
        assert!((result.curve[2].median_distance_m.expect("median") - 0.05).abs() < 1e-12);
        assert_eq!(result.curve[0].rejected_above_cutoff_count, 2);
        assert_eq!(result.no_correspondence_count, 0);
    }

    #[test]
    fn even_sample_median_averages_middle_pair() {
        let samples = [Point3::new(0.2, 0.2, 0.01), Point3::new(0.4, 0.2, 0.05)];
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::CandidateToReference,
            AccuracySamplePopulation::SourceSurfaceVertices,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &samples,
            &[plane(0.0, true)],
            &[0.10],
        )
        .expect("even number of observations must have conventional median");

        assert!((result.curve[0].median_distance_m.expect("median") - 0.03).abs() < 1e-12);
    }

    #[test]
    fn nearest_eligible_surface_wins_with_stable_tie_behavior() {
        let samples = [Point3::new(0.2, 0.2, 0.04)];
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::CandidateToReference,
            AccuracySamplePopulation::SourceSurfaceVertices,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &samples,
            &[plane(0.0, true), plane(0.02, true)],
            &[0.1],
        )
        .expect("two parallel target planes should be comparable");
        assert!((result.sample_distances_m[0].expect("nearest") - 0.02).abs() < 1e-12);
    }

    #[test]
    fn rejects_direction_and_population_mismatch() {
        let samples = [Point3::new(0.2, 0.2, 0.01)];
        let error = evaluate_directed_accuracy_curve(
            AccuracyDirection::ReferenceToCandidate,
            AccuracySamplePopulation::SourceSurfaceVertices,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &samples,
            &[plane(0.0, true)],
            &[0.1],
        )
        .expect_err("direction and sampling population must agree");
        assert!(error.contains("reference_to_candidate requires reference_surface_samples"));
    }

    #[test]
    fn rejects_invalid_and_non_monotone_cutoffs() {
        let samples = [Point3::new(0.2, 0.2, 0.01)];
        let invalid = evaluate_directed_accuracy_curve(
            AccuracyDirection::CandidateToReference,
            AccuracySamplePopulation::SourceSurfaceVertices,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &samples,
            &[plane(0.0, true)],
            &[0.1, 0.1],
        )
        .expect_err("cutoffs must be strictly increasing");
        assert!(invalid.contains("strictly increasing"));

        let non_positive = evaluate_directed_accuracy_curve(
            AccuracyDirection::CandidateToReference,
            AccuracySamplePopulation::SourceSurfaceVertices,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &samples,
            &[plane(0.0, true)],
            &[0.0],
        )
        .expect_err("cutoffs must be positive");
        assert!(non_positive.contains("finite positive metres"));
    }

    #[test]
    fn rejects_degenerate_and_non_finite_geometry() {
        let sample = [Point3::new(0.2, 0.2, 0.01)];
        let degenerate = SurfaceTriangle::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(2.0, 2.0, 2.0),
            true,
        );
        let error = evaluate_directed_accuracy_curve(
            AccuracyDirection::CandidateToReference,
            AccuracySamplePopulation::SourceSurfaceVertices,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &sample,
            &[degenerate],
            &[0.1],
        )
        .expect_err("degenerate reference triangle must fail closed");
        assert!(error.contains("degenerate"));

        let non_finite = [Point3::new(f64::NAN, 0.0, 0.0)];
        let error = evaluate_directed_accuracy_curve(
            AccuracyDirection::CandidateToReference,
            AccuracySamplePopulation::SourceSurfaceVertices,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &non_finite,
            &[plane(0.0, true)],
            &[0.1],
        )
        .expect_err("non-finite sample must fail closed");
        assert!(error.contains("non-finite"));
    }

    #[test]
    fn reports_direction_and_population_without_averaging() {
        let samples = [Point3::new(0.2, 0.2, 0.01)];
        let result = evaluate_directed_accuracy_curve(
            AccuracyDirection::CandidateToReference,
            AccuracySamplePopulation::SourceSurfaceVertices,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle,
            &samples,
            &[plane(0.0, true)],
            &[0.1],
        )
        .expect("direction metadata should be retained");
        assert_eq!(result.direction, AccuracyDirection::CandidateToReference);
        assert_eq!(
            result.sample_population,
            AccuracySamplePopulation::SourceSurfaceVertices
        );
        assert_eq!(
            result.correspondence_policy,
            CorrespondencePolicy::OrthogonalProjectionInsideTriangle
        );
    }
}
