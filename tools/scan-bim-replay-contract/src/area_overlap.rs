// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Auditable planar area-overlap primitives for Scan-to-BIM completeness/correctness.
//!
//! This is a bounded v0.1 geometry kernel for convex planar surfaces. The caller
//! supplies the already-buffered reference boundary and its content digest. The
//! buffer-construction algorithm is intentionally not guessed here.
//!
//! Two published aggregation interpretations are kept separate:
//! - PairwiseSum2018 sums every compatible source/reference intersection.
//! - UnionPerReference2021 unions source intersections for each reference surface.
//!
//! Neither profile is labelled the canonical historical leaderboard scorer until
//! its buffer construction, parallel-angle threshold, and tessellation semantics
//! are reproduced from the original implementation.

use crate::metrics::Point3;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const GEOMETRY_EPSILON: f64 = 1.0e-10;
const MAX_SURFACES: usize = 100_000;
const MAX_VERTICES_PER_SURFACE: usize = 20_000;
const MAX_PAIRS: usize = 10_000_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

impl Point2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceVisibility {
    Observed,
    Interpreted,
    Unobservable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AreaAggregationPolicy {
    /// 2018 evaluation-framework equations: sum every compatible pairwise
    /// source/reference intersection area.
    PairwiseSum2018,
    /// 2021 final-results equations: union the intersecting source patches
    /// separately for each reference surface, then sum those per-reference unions.
    UnionPerReference2021,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceAreaSurface {
    pub surface_id: String,
    pub geometry_artifact_id: String,
    pub geometry_artifact_sha256: String,
    pub frame_id: String,
    /// Convex, planar, ordered polygon boundary in the common frame, metres.
    pub vertices: Vec<Point3>,
}

impl SourceAreaSurface {
    pub fn new(
        surface_id: impl Into<String>,
        geometry_artifact_id: impl Into<String>,
        geometry_artifact_sha256: impl Into<String>,
        frame_id: impl Into<String>,
        vertices: Vec<Point3>,
    ) -> Self {
        Self {
            surface_id: surface_id.into(),
            geometry_artifact_id: geometry_artifact_id.into(),
            geometry_artifact_sha256: geometry_artifact_sha256.into(),
            frame_id: frame_id.into(),
            vertices,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceAreaSurface {
    pub surface_id: String,
    pub geometry_artifact_id: String,
    pub geometry_artifact_sha256: String,
    pub frame_id: String,
    pub visibility: ReferenceVisibility,
    /// Convex, planar, ordered polygon boundary in the common frame, metres.
    pub vertices: Vec<Point3>,
    /// Convex boundary of the reference surface after a separately qualified
    /// buffer operation, expressed in this reference surface's local (u,v) plane.
    pub buffered_boundary_2d: Vec<Point2>,
    /// Exact artifact digest for the precomputed buffer geometry.
    pub buffer_artifact_id: Option<String>,
    pub buffer_artifact_sha256: Option<String>,
}

impl ReferenceAreaSurface {
    pub fn new(
        surface_id: impl Into<String>,
        geometry_artifact_id: impl Into<String>,
        geometry_artifact_sha256: impl Into<String>,
        frame_id: impl Into<String>,
        visibility: ReferenceVisibility,
        vertices: Vec<Point3>,
        buffered_boundary_2d: Vec<Point2>,
        buffer_artifact_id: Option<String>,
        buffer_artifact_sha256: Option<String>,
    ) -> Self {
        Self {
            surface_id: surface_id.into(),
            geometry_artifact_id: geometry_artifact_id.into(),
            geometry_artifact_sha256: geometry_artifact_sha256.into(),
            frame_id: frame_id.into(),
            visibility,
            vertices,
            buffered_boundary_2d,
            buffer_artifact_id,
            buffer_artifact_sha256,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AreaOverlapProfile {
    pub profile_id: String,
    pub metric_authority: AreaMetricAuthority,
    pub coordinate_unit: AreaCoordinateUnit,
    pub aggregation_policy: AreaAggregationPolicy,
    pub common_frame_id: String,
    /// Supplied from the benchmark implementation/profile; no default is inferred.
    pub max_parallel_angle_rad: f64,
    /// The reference boundary buffers were generated with this distance, metres.
    pub buffer_distance_m: f64,
    /// Names and versions the external buffer constructor that produced each
    /// ReferenceAreaSurface.buffered_boundary_2d.
    pub buffer_policy_id: String,
    pub planarity_tolerance_m: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairwiseAreaIntersection {
    pub source_surface_id: String,
    pub reference_surface_id: String,
    pub area_m2: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceAreaLedger {
    pub reference_surface_id: String,
    pub observable_reference_area_m2: f64,
    pub matched_area_m2: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaOverlapResult {
    pub profile_id: String,
    pub metric_authority: AreaMetricAuthority,
    pub coordinate_unit: AreaCoordinateUnit,
    pub source_geometry_artifact_ids: Vec<String>,
    pub source_geometry_artifact_digests: Vec<String>,
    pub reference_geometry_artifact_ids: Vec<String>,
    pub reference_geometry_artifact_digests: Vec<String>,
    pub buffer_artifact_ids: Vec<String>,
    pub buffer_artifact_digests: Vec<String>,
    pub aggregation_policy: AreaAggregationPolicy,
    pub max_parallel_angle_rad: f64,
    pub buffer_distance_m: f64,
    pub buffer_policy_id: String,
    pub source_surface_area_m2: f64,
    pub observable_reference_surface_area_m2: f64,
    pub interpreted_reference_area_excluded_m2: f64,
    pub unobservable_reference_area_excluded_m2: f64,
    pub matched_area_numerator_m2: f64,
    pub completeness: Option<f64>,
    pub correctness: Option<f64>,
    pub pairwise_intersections: Vec<PairwiseAreaIntersection>,
    pub reference_ledger: Vec<ReferenceAreaLedger>,
    pub skipped_non_parallel_pair_count: usize,
    /// Nonzero ratios are preserved rather than clamped. A value above one
    /// signals an incompatibility or overlapping-denominator condition.
    pub completeness_exceeds_one: bool,
    pub correctness_exceeds_one: bool,
}

#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn from_point(point: Point3) -> Self {
        Self {
            x: point.x,
            y: point.y,
            z: point.z,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn scale(self, value: f64) -> Self {
        Self {
            x: self.x * value,
            y: self.y * value,
            z: self.z * value,
        }
    }

    fn norm(self) -> f64 {
        self.x.hypot(self.y).hypot(self.z)
    }

    fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

#[derive(Debug, Clone, Copy)]
struct PlaneFrame {
    origin: Vec3,
    u: Vec3,
    v: Vec3,
    normal: Vec3,
}

#[derive(Debug, Clone)]
struct PreparedReference {
    surface_id: String,
    visibility: ReferenceVisibility,
    area_m2: f64,
    frame: PlaneFrame,
    buffered_2d: Vec<Point2>,
}

#[derive(Debug, Clone)]
struct PreparedSource {
    surface_id: String,
    area_m2: f64,
    normal: Vec3,
    vertices: Vec<Point3>,
}

#[derive(Debug, Clone, Copy)]
struct Line2 {
    slope: f64,
    intercept: f64,
}

impl Line2 {
    fn y(self, x: f64) -> f64 {
        self.slope * x + self.intercept
    }
}

#[derive(Debug, Clone, Copy)]
struct VerticalInterval {
    low: Line2,
    high: Line2,
    low_y_at_mid: f64,
    high_y_at_mid: f64,
}

fn finite_point2(point: Point2) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn cross2(a: Point2, b: Point2, c: Point2) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn polygon_signed_area(polygon: &[Point2]) -> f64 {
    let mut twice_area = 0.0;
    for index in 0..polygon.len() {
        let current = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        twice_area += current.x * next.y - next.x * current.y;
    }
    0.5 * twice_area
}

fn validate_convex_polygon(polygon: &[Point2], label: &str) -> Result<Vec<Point2>, String> {
    if polygon.len() < 3 || polygon.len() > MAX_VERTICES_PER_SURFACE {
        return Err(format!(
            "{label} must have between 3 and {MAX_VERTICES_PER_SURFACE} vertices"
        ));
    }
    if polygon.iter().any(|point| !finite_point2(*point)) {
        return Err(format!("{label} contains a non-finite coordinate"));
    }

    let scale = polygon
        .iter()
        .flat_map(|point| [point.x.abs(), point.y.abs()])
        .fold(1.0_f64, f64::max);
    let epsilon = scale * scale * GEOMETRY_EPSILON;
    let mut direction = 0_i8;

    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        let c = polygon[(index + 2) % polygon.len()];
        let turn = cross2(a, b, c);
        if !turn.is_finite() {
            return Err(format!("{label} cross product is non-finite"));
        }
        if turn.abs() <= epsilon {
            continue;
        }
        let current_direction = if turn > 0.0 { 1 } else { -1 };
        if direction != 0 && direction != current_direction {
            return Err(format!("{label} must be convex and consistently ordered"));
        }
        direction = current_direction;
    }
    if direction == 0 {
        return Err(format!("{label} is degenerate"));
    }

    let signed_area = polygon_signed_area(polygon);
    if !signed_area.is_finite() || signed_area.abs() <= epsilon {
        return Err(format!("{label} area is zero or non-finite"));
    }

    let mut normalized = polygon.to_vec();
    if signed_area < 0.0 {
        normalized.reverse();
    }
    Ok(normalized)
}

fn polygon_area(polygon: &[Point2]) -> f64 {
    polygon_signed_area(polygon).abs()
}

fn plane_frame(
    vertices: &[Point3],
    label: &str,
    planarity_tolerance_m: f64,
) -> Result<(PlaneFrame, Vec<Point2>, f64), String> {
    if vertices.len() < 3 || vertices.len() > MAX_VERTICES_PER_SURFACE {
        return Err(format!(
            "{label} must have between 3 and {MAX_VERTICES_PER_SURFACE} vertices"
        ));
    }
    let points: Vec<Vec3> = vertices.iter().copied().map(Vec3::from_point).collect();
    if points.iter().any(|point| !point.finite()) {
        return Err(format!("{label} contains a non-finite coordinate"));
    }

    let origin = points[0];
    let mut first_axis = None;
    let mut normal = None;
    for point in points.iter().skip(1) {
        let edge = point.sub(origin);
        let length = edge.norm();
        if length > GEOMETRY_EPSILON && first_axis.is_none() {
            first_axis = Some(edge.scale(1.0 / length));
        }
        if let Some(axis) = first_axis {
            let candidate = axis.cross(edge);
            let candidate_length = candidate.norm();
            if candidate_length > GEOMETRY_EPSILON {
                normal = Some(candidate.scale(1.0 / candidate_length));
                break;
            }
        }
    }
    let Some(u) = first_axis else {
        return Err(format!("{label} has no non-zero edge"));
    };
    let Some(normal) = normal else {
        return Err(format!("{label} is degenerate"));
    };
    let v = normal.cross(u);

    for (index, point) in points.iter().enumerate() {
        let distance = point.sub(origin).dot(normal).abs();
        if !distance.is_finite() || distance > planarity_tolerance_m {
            return Err(format!(
                "{label} is not planar within {planarity_tolerance_m} metres at vertex {index}"
            ));
        }
    }

    let frame = PlaneFrame {
        origin,
        u,
        v,
        normal,
    };
    let coordinates: Vec<Point2> = points
        .iter()
        .map(|point| {
            let local = point.sub(origin);
            Point2::new(local.dot(u), local.dot(v))
        })
        .collect();
    let coordinates = validate_convex_polygon(&coordinates, label)?;
    let area_m2 = polygon_area(&coordinates);
    if !area_m2.is_finite() || area_m2 <= 0.0 {
        return Err(format!("{label} has invalid area"));
    }
    Ok((frame, coordinates, area_m2))
}

fn project_to_plane(frame: PlaneFrame, point: Point3) -> Result<Point2, String> {
    let point = Vec3::from_point(point);
    if !point.finite() {
        return Err("source surface contains a non-finite coordinate".to_owned());
    }
    let local = point.sub(frame.origin);
    let projected = Point2::new(local.dot(frame.u), local.dot(frame.v));
    if !finite_point2(projected) {
        return Err("source projection produced a non-finite coordinate".to_owned());
    }
    Ok(projected)
}

fn inside_convex(edge_a: Point2, edge_b: Point2, point: Point2, epsilon: f64) -> bool {
    cross2(edge_a, edge_b, point) >= -epsilon
}

fn line_intersection(
    segment_start: Point2,
    segment_end: Point2,
    clip_start: Point2,
    clip_end: Point2,
) -> Result<Point2, String> {
    let start_distance = cross2(clip_start, clip_end, segment_start);
    let end_distance = cross2(clip_start, clip_end, segment_end);
    let denominator = start_distance - end_distance;
    if !denominator.is_finite() || denominator.abs() <= f64::MIN_POSITIVE {
        return Err("polygon clipping encountered a parallel edge intersection".to_owned());
    }
    let t = (start_distance / denominator).clamp(0.0, 1.0);
    let point = Point2::new(
        segment_start.x + t * (segment_end.x - segment_start.x),
        segment_start.y + t * (segment_end.y - segment_start.y),
    );
    if !finite_point2(point) {
        return Err("polygon clipping produced a non-finite coordinate".to_owned());
    }
    Ok(point)
}

fn convex_intersection(subject: &[Point2], clip: &[Point2]) -> Result<Vec<Point2>, String> {
    let mut output = validate_convex_polygon(subject, "projected source polygon")?;
    let clip = validate_convex_polygon(clip, "buffered reference boundary")?;
    let scale = subject
        .iter()
        .chain(clip.iter())
        .flat_map(|point| [point.x.abs(), point.y.abs()])
        .fold(1.0_f64, f64::max);
    let epsilon = scale * scale * GEOMETRY_EPSILON;

    for index in 0..clip.len() {
        if output.is_empty() {
            break;
        }
        let clip_start = clip[index];
        let clip_end = clip[(index + 1) % clip.len()];
        let input = std::mem::take(&mut output);
        let mut previous = *input.last().expect("non-empty polygon");
        let mut previous_inside = inside_convex(clip_start, clip_end, previous, epsilon);

        for current in input {
            let current_inside = inside_convex(clip_start, clip_end, current, epsilon);
            if current_inside {
                if !previous_inside {
                    output.push(line_intersection(
                        previous,
                        current,
                        clip_start,
                        clip_end,
                    )?);
                }
                output.push(current);
            } else if previous_inside {
                output.push(line_intersection(
                    previous,
                    current,
                    clip_start,
                    clip_end,
                )?);
            }
            previous = current;
            previous_inside = current_inside;
        }
    }

    if output.len() < 3 {
        return Ok(Vec::new());
    }
    let output = validate_convex_polygon(&output, "intersection polygon")?;
    if polygon_area(&output) <= epsilon {
        return Ok(Vec::new());
    }
    Ok(output)
}

fn segment_intersection_x(
    a: Point2,
    b: Point2,
    c: Point2,
    d: Point2,
) -> Option<f64> {
    let r = Point2::new(b.x - a.x, b.y - a.y);
    let s = Point2::new(d.x - c.x, d.y - c.y);
    let denominator = r.x * s.y - r.y * s.x;
    if denominator.abs() <= f64::MIN_POSITIVE {
        return None;
    }
    let q_minus_p = Point2::new(c.x - a.x, c.y - a.y);
    let t = (q_minus_p.x * s.y - q_minus_p.y * s.x) / denominator;
    let u = (q_minus_p.x * r.y - q_minus_p.y * r.x) / denominator;
    if (-GEOMETRY_EPSILON..=1.0 + GEOMETRY_EPSILON).contains(&t)
        && (-GEOMETRY_EPSILON..=1.0 + GEOMETRY_EPSILON).contains(&u)
    {
        Some(a.x + t * r.x)
    } else {
        None
    }
}

fn line_for_edge(a: Point2, b: Point2) -> Option<Line2> {
    let dx = b.x - a.x;
    if dx.abs() <= f64::MIN_POSITIVE {
        return None;
    }
    let slope = (b.y - a.y) / dx;
    let intercept = a.y - slope * a.x;
    (slope.is_finite() && intercept.is_finite()).then_some(Line2 { slope, intercept })
}

fn polygon_vertical_interval(polygon: &[Point2], x: f64) -> Option<VerticalInterval> {
    let mut crossings = Vec::with_capacity(2);
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        if x > a.x.min(b.x) && x < a.x.max(b.x) {
            if let Some(line) = line_for_edge(a, b) {
                let y = line.y(x);
                crossings.push((y, line));
            }
        }
    }
    if crossings.len() < 2 {
        return None;
    }
    crossings.sort_by(|left, right| left.0.total_cmp(&right.0));
    Some(VerticalInterval {
        low: crossings[0].1,
        high: crossings[crossings.len() - 1].1,
        low_y_at_mid: crossings[0].0,
        high_y_at_mid: crossings[crossings.len() - 1].0,
    })
}

/// Exact (up to floating-point arithmetic) union area for a small set of convex polygons.
///
/// The vertical sweep inserts every vertex x-coordinate and every proper polygon-edge
/// intersection x-coordinate. Within each resulting slab, edge order is fixed and
/// the union cross-section has linear boundaries, so trapezoidal integration is exact.
fn convex_union_area(polygons: &[Vec<Point2>]) -> Result<f64, String> {
    if polygons.is_empty() {
        return Ok(0.0);
    }
    if polygons.len() == 1 {
        return Ok(polygon_area(&polygons[0]));
    }

    let mut x_breaks = Vec::new();
    let mut edges = Vec::new();
    for polygon in polygons {
        for index in 0..polygon.len() {
            let a = polygon[index];
            let b = polygon[(index + 1) % polygon.len()];
            x_breaks.push(a.x);
            edges.push((a, b));
        }
    }

    for left in 0..edges.len() {
        for right in (left + 1)..edges.len() {
            if let Some(x) = segment_intersection_x(
                edges[left].0,
                edges[left].1,
                edges[right].0,
                edges[right].1,
            ) {
                if x.is_finite() {
                    x_breaks.push(x);
                }
            }
        }
    }

    x_breaks.sort_by(f64::total_cmp);
    let scale = x_breaks
        .iter()
        .map(|value| value.abs())
        .fold(1.0_f64, f64::max);
    x_breaks.dedup_by(|left, right| (*left - *right).abs() <= scale * GEOMETRY_EPSILON);

    let mut union_area = 0.0;
    for pair in x_breaks.windows(2) {
        let x_left = pair[0];
        let x_right = pair[1];
        let width = x_right - x_left;
        if !width.is_finite() || width <= scale * GEOMETRY_EPSILON {
            continue;
        }
        let x_mid = (x_left + x_right) * 0.5;

        let mut intervals: Vec<VerticalInterval> = polygons
            .iter()
            .filter_map(|polygon| polygon_vertical_interval(polygon, x_mid))
            .collect();
        intervals.sort_by(|left, right| left.low_y_at_mid.total_cmp(&right.low_y_at_mid));

        let mut merged: Vec<(Line2, Line2, f64)> = Vec::new();
        for interval in intervals {
            if let Some(last) = merged.last_mut() {
                if interval.low_y_at_mid <= last.2 + scale * GEOMETRY_EPSILON {
                    if interval.high_y_at_mid > last.2 {
                        last.1 = interval.high;
                        last.2 = interval.high_y_at_mid;
                    }
                    continue;
                }
            }
            merged.push((
                interval.low,
                interval.high,
                interval.high_y_at_mid,
            ));
        }

        for (low, high, _) in merged {
            let left_width = (high.y(x_left) - low.y(x_left)).max(0.0);
            let right_width = (high.y(x_right) - low.y(x_right)).max(0.0);
            let slab_area = 0.5 * (left_width + right_width) * width;
            if !slab_area.is_finite() || slab_area < 0.0 {
                return Err("polygon union area is non-finite".to_owned());
            }
            union_area += slab_area;
            if !union_area.is_finite() {
                return Err("polygon union area overflowed".to_owned());
            }
        }
    }
    Ok(union_area)
}

fn scale_point3(point: Point3, factor: f64) -> Point3 {
    Point3::new(point.x * factor, point.y * factor, point.z * factor)
}

fn scale_point2(point: Point2, factor: f64) -> Point2 {
    Point2::new(point.x * factor, point.y * factor)
}

fn prepare_source(
    source: &SourceAreaSurface,
    profile: &AreaOverlapProfile,
) -> Result<PreparedSource, String> {
    if source.surface_id.trim().is_empty()
        || source.geometry_artifact_id.trim().is_empty()
        || !digest_is_sha256(&source.geometry_artifact_sha256)
    {
        return Err("source surface requires a stable ID and content-addressed geometry artifact".to_owned());
    }
    if source.frame_id != profile.common_frame_id {
        return Err(format!(
            "source surface {} frame does not match profile frame",
            source.surface_id
        ));
    }
    let unit_scale = profile.coordinate_unit.metres_per_unit();
    let vertices: Vec<Point3> = source
        .vertices
        .iter()
        .map(|point| scale_point3(*point, unit_scale))
        .collect();
    let (frame, _, area_m2) = plane_frame(
        &vertices,
        "source surface",
        profile.planarity_tolerance_m,
    )?;
    Ok(PreparedSource {
        surface_id: source.surface_id.clone(),
        area_m2,
        normal: frame.normal,
        vertices,
    })
}

fn prepare_reference(
    reference: &ReferenceAreaSurface,
    profile: &AreaOverlapProfile,
) -> Result<PreparedReference, String> {
    if reference.surface_id.trim().is_empty()
        || reference.geometry_artifact_id.trim().is_empty()
        || !digest_is_sha256(&reference.geometry_artifact_sha256)
    {
        return Err("reference surface requires a stable ID and content-addressed geometry artifact".to_owned());
    }
    if reference.frame_id != profile.common_frame_id {
        return Err(format!(
            "reference surface {} frame does not match profile frame",
            reference.surface_id
        ));
    }
    let unit_scale = profile.coordinate_unit.metres_per_unit();
    let reference_vertices: Vec<Point3> = reference
        .vertices
        .iter()
        .map(|point| scale_point3(*point, unit_scale))
        .collect();
    let (frame, original_2d, area_m2) = plane_frame(
        &reference_vertices,
        "reference surface",
        profile.planarity_tolerance_m,
    )?;
    let buffered_vertices: Vec<Point2> = reference
        .buffered_boundary_2d
        .iter()
        .map(|point| scale_point2(*point, unit_scale))
        .collect();
    let buffered_2d = validate_convex_polygon(
        &buffered_vertices,
        "buffered reference boundary",
    )?;
    if reference.visibility == ReferenceVisibility::Observed {
        let buffer_id = reference
            .buffer_artifact_id
            .as_deref()
            .ok_or("observed reference surface requires a buffer artifact ID")?;
        if buffer_id.trim().is_empty() {
            return Err("buffer artifact ID must be non-empty".to_owned());
        }
        let digest = reference
            .buffer_artifact_sha256
            .as_deref()
            .ok_or("observed reference surface requires a buffer artifact digest")?;
        if !digest_is_sha256(digest) {
            return Err("buffer artifact digest must be lowercase SHA-256".to_owned());
        }
        for vertex in &original_2d {
            if !point_in_convex_polygon(*vertex, &buffered_2d) {
                return Err(format!(
                    "buffered boundary for observed reference {} does not contain its original boundary",
                    reference.surface_id
                ));
            }
        }
    }
    Ok(PreparedReference {
        surface_id: reference.surface_id.clone(),
        visibility: reference.visibility,
        area_m2,
        frame,
        buffered_2d,
    })
}

fn digest_is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn point_in_convex_polygon(point: Point2, polygon: &[Point2]) -> bool {
    let scale = polygon
        .iter()
        .flat_map(|vertex| [vertex.x.abs(), vertex.y.abs()])
        .fold(1.0_f64, f64::max);
    let epsilon = scale * scale * GEOMETRY_EPSILON;
    polygon.iter().enumerate().all(|(index, start)| {
        let end = polygon[(index + 1) % polygon.len()];
        cross2(*start, end, point) >= -epsilon
    })
}

fn projected_source_polygon(
    source: &PreparedSource,
    reference: &PreparedReference,
) -> Result<Vec<Point2>, String> {
    let projected: Vec<Point2> = source
        .vertices
        .iter()
        .map(|point| project_to_plane(reference.frame, *point))
        .collect::<Result<_, _>>()?;
    validate_convex_polygon(&projected, "projected source surface")
}

/// Validate a replay-bound area-overlap profile without accessing geometry.
pub fn validate_area_overlap_profile(profile: &AreaOverlapProfile) -> Result<(), String> {
    if profile.profile_id.trim().is_empty()
        || profile.metric_authority.trim().is_empty()
        || profile.common_frame_id.trim().is_empty()
        || profile.buffer_policy_id.trim().is_empty()
    {
        return Err("area overlap profile identifiers must be non-empty".to_owned());
    }
    let unit_scale = profile.coordinate_unit.metres_per_unit();
    if !unit_scale.is_finite() || unit_scale <= 0.0 {
        return Err("area overlap coordinate unit has invalid SI conversion".to_owned());
    }
    if !profile.max_parallel_angle_rad.is_finite()
        || profile.max_parallel_angle_rad < 0.0
        || profile.max_parallel_angle_rad > std::f64::consts::FRAC_PI_2
    {
        return Err("max_parallel_angle_rad must be finite and in [0, pi/2]".to_owned());
    }
    if !profile.planarity_tolerance_m.is_finite() || profile.planarity_tolerance_m <= 0.0 {
        return Err("planarity_tolerance_m must be finite and positive metres".to_owned());
    }
    match (profile.metric_authority, profile.aggregation_policy) {
        (AreaMetricAuthority::EvaluationFramework2018, AreaAggregationPolicy::PairwiseSum2018)
        | (AreaMetricAuthority::FinalResultsPaper2021, AreaAggregationPolicy::UnionPerReference2021)
        | (AreaMetricAuthority::ProjectDefined, _) => {}
        _ => {
            return Err(
                "published area aggregation policy must match its named metric authority".to_owned(),
            );
        }
    }
    if !profile.buffer_distance_m.is_finite() || profile.buffer_distance_m < 0.0 {
        return Err("buffer_distance_m must be finite and non-negative metres".to_owned());
    }
    Ok(())
}

/// Calculate audit-ready completeness/correctness numerators and denominators.
///
/// All source and reference polygons must be convex and planar, in the same declared
/// metre-valued frame. The reference buffer polygon must be supplied by a separately
/// versioned buffer constructor. This function does not guess the angle threshold,
/// infer visibility or create the buffer itself.
pub fn evaluate_area_overlap(
    profile: &AreaOverlapProfile,
    source_surfaces: &[SourceAreaSurface],
    reference_surfaces: &[ReferenceAreaSurface],
) -> Result<AreaOverlapResult, String> {
    validate_area_overlap_profile(profile)?;
    if source_surfaces.len() > MAX_SURFACES || reference_surfaces.len() > MAX_SURFACES {
        return Err("surface count exceeds the bounded area-overlap profile".to_owned());
    }
    if source_surfaces.is_empty() && reference_surfaces.is_empty() {
        return Err("area overlap requires at least one source or reference surface".to_owned());
    }
    if source_surfaces
        .len()
        .checked_mul(reference_surfaces.len())
        .is_none_or(|pairs| pairs > MAX_PAIRS)
    {
        return Err("source/reference pair count exceeds the bounded area-overlap profile".to_owned());
    }

    let mut source_ids = BTreeSet::new();
    let mut source_geometry_artifact_ids = BTreeSet::new();
    let mut source_geometry_artifact_digests = BTreeSet::new();
    let mut prepared_sources = Vec::with_capacity(source_surfaces.len());
    let mut source_surface_area_m2 = 0.0;
    for source in source_surfaces {
        if !source_ids.insert(source.surface_id.as_str()) {
            return Err(format!("duplicate source surface_id: {}", source.surface_id));
        }
        source_geometry_artifact_ids.insert(source.geometry_artifact_id.clone());
        source_geometry_artifact_digests.insert(source.geometry_artifact_sha256.clone());
        let prepared = prepare_source(source, profile)?;
        source_surface_area_m2 += prepared.area_m2;
        if !source_surface_area_m2.is_finite() {
            return Err("source surface area denominator overflowed".to_owned());
        }
        prepared_sources.push(prepared);
    }

    let mut reference_ids = BTreeSet::new();
    let mut reference_geometry_artifact_ids = BTreeSet::new();
    let mut reference_geometry_artifact_digests = BTreeSet::new();
    let mut buffer_artifact_ids = BTreeSet::new();
    let mut buffer_artifact_digests = BTreeSet::new();
    let mut prepared_references = Vec::with_capacity(reference_surfaces.len());
    let mut observable_reference_surface_area_m2 = 0.0;
    let mut interpreted_reference_area_excluded_m2 = 0.0;
    let mut unobservable_reference_area_excluded_m2 = 0.0;
    for reference in reference_surfaces {
        if !reference_ids.insert(reference.surface_id.as_str()) {
            return Err(format!(
                "duplicate reference surface_id: {}",
                reference.surface_id
            ));
        }
        reference_geometry_artifact_ids.insert(reference.geometry_artifact_id.clone());
        reference_geometry_artifact_digests.insert(reference.geometry_artifact_sha256.clone());
        if let Some(artifact_id) = &reference.buffer_artifact_id {
            buffer_artifact_ids.insert(artifact_id.clone());
        }
        if let Some(digest) = &reference.buffer_artifact_sha256 {
            buffer_artifact_digests.insert(digest.clone());
        }
        let prepared = prepare_reference(reference, profile)?;
        match prepared.visibility {
            ReferenceVisibility::Observed => observable_reference_surface_area_m2 += prepared.area_m2,
            ReferenceVisibility::Interpreted => interpreted_reference_area_excluded_m2 += prepared.area_m2,
            ReferenceVisibility::Unobservable => unobservable_reference_area_excluded_m2 += prepared.area_m2,
        }
        if !observable_reference_surface_area_m2.is_finite()
            || !interpreted_reference_area_excluded_m2.is_finite()
            || !unobservable_reference_area_excluded_m2.is_finite()
        {
            return Err("reference surface area denominator overflowed".to_owned());
        }
        prepared_references.push(prepared);
    }

    let minimum_normal_dot = profile.max_parallel_angle_rad.cos();
    let mut pairwise_intersections = Vec::new();
    let mut reference_ledger = Vec::new();
    let mut matched_area_numerator_m2 = 0.0;
    let mut skipped_non_parallel_pair_count = 0;

    for reference in &prepared_references {
        if reference.visibility != ReferenceVisibility::Observed {
            continue;
        }
        let mut intersection_polygons = Vec::new();
        let mut pairwise_reference_area = 0.0;

        for source in &prepared_sources {
            let parallel_measure = source.normal.dot(reference.frame.normal).abs();
            if !parallel_measure.is_finite() {
                return Err("source/reference parallel test produced a non-finite value".to_owned());
            }
            if parallel_measure + GEOMETRY_EPSILON < minimum_normal_dot {
                skipped_non_parallel_pair_count += 1;
                continue;
            }

            let projected = projected_source_polygon(source, reference)?;
            let intersection = convex_intersection(&projected, &reference.buffered_2d)?;
            if intersection.is_empty() {
                continue;
            }
            let area_m2 = polygon_area(&intersection);
            if !area_m2.is_finite() || area_m2 < 0.0 {
                return Err("source/reference intersection area is invalid".to_owned());
            }
            if area_m2 > 0.0 {
                pairwise_reference_area += area_m2;
                if !pairwise_reference_area.is_finite() {
                    return Err("pairwise intersection area overflowed".to_owned());
                }
                pairwise_intersections.push(PairwiseAreaIntersection {
                    source_surface_id: source.surface_id.clone(),
                    reference_surface_id: reference.surface_id.clone(),
                    area_m2,
                });
                intersection_polygons.push(intersection);
            }
        }

        let matched_reference_area_m2 = match profile.aggregation_policy {
            AreaAggregationPolicy::PairwiseSum2018 => pairwise_reference_area,
            AreaAggregationPolicy::UnionPerReference2021 => {
                convex_union_area(&intersection_polygons)?
            }
        };
        matched_area_numerator_m2 += matched_reference_area_m2;
        if !matched_area_numerator_m2.is_finite() {
            return Err("matched area numerator overflowed".to_owned());
        }
        reference_ledger.push(ReferenceAreaLedger {
            reference_surface_id: reference.surface_id.clone(),
            observable_reference_area_m2: reference.area_m2,
            matched_area_m2: matched_reference_area_m2,
        });
    }

    let completeness = (observable_reference_surface_area_m2 > 0.0)
        .then_some(matched_area_numerator_m2 / observable_reference_surface_area_m2);
    let correctness = (source_surface_area_m2 > 0.0)
        .then_some(matched_area_numerator_m2 / source_surface_area_m2);

    Ok(AreaOverlapResult {
        profile_id: profile.profile_id.clone(),
        metric_authority: profile.metric_authority,
        coordinate_unit: profile.coordinate_unit,
        source_geometry_artifact_ids: source_geometry_artifact_ids.into_iter().collect(),
        source_geometry_artifact_digests: source_geometry_artifact_digests.into_iter().collect(),
        reference_geometry_artifact_ids: reference_geometry_artifact_ids.into_iter().collect(),
        reference_geometry_artifact_digests: reference_geometry_artifact_digests.into_iter().collect(),
        buffer_artifact_ids: buffer_artifact_ids.into_iter().collect(),
        buffer_artifact_digests: buffer_artifact_digests.into_iter().collect(),
        aggregation_policy: profile.aggregation_policy,
        max_parallel_angle_rad: profile.max_parallel_angle_rad,
        buffer_distance_m: profile.buffer_distance_m,
        buffer_policy_id: profile.buffer_policy_id.clone(),
        source_surface_area_m2,
        observable_reference_surface_area_m2,
        interpreted_reference_area_excluded_m2,
        unobservable_reference_area_excluded_m2,
        matched_area_numerator_m2,
        completeness,
        correctness,
        pairwise_intersections,
        reference_ledger,
        skipped_non_parallel_pair_count,
        completeness_exceeds_one: completeness.is_some_and(|value| value > 1.0 + GEOMETRY_EPSILON),
        correctness_exceeds_one: correctness.is_some_and(|value| value > 1.0 + GEOMETRY_EPSILON),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AreaAggregationPolicy, AreaCoordinateUnit, AreaMetricAuthority, AreaOverlapProfile, Point2,
        ReferenceAreaSurface, ReferenceVisibility, SourceAreaSurface, evaluate_area_overlap,
        validate_area_overlap_profile,
    };
    use crate::metrics::Point3;

    fn square_3d(
        surface_id: &str,
        frame_id: &str,
        x: f64,
        y: f64,
        z: f64,
        width: f64,
        height: f64,
    ) -> SourceAreaSurface {
        SourceAreaSurface::new(
            surface_id,
            format!("source-model-{surface_id}"),
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            frame_id,
            vec![
                Point3::new(x, y, z),
                Point3::new(x + width, y, z),
                Point3::new(x + width, y + height, z),
                Point3::new(x, y + height, z),
            ],
        )
    }

    fn reference(
        surface_id: &str,
        visibility: ReferenceVisibility,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        buffered_boundary_2d: Vec<Point2>,
    ) -> ReferenceAreaSurface {
        ReferenceAreaSurface::new(
            surface_id,
            format!("reference-model-{surface_id}"),
            "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "LOCAL-A",
            visibility,
            vec![
                Point3::new(x, y, 0.0),
                Point3::new(x + width, y, 0.0),
                Point3::new(x + width, y + height, 0.0),
                Point3::new(x, y + height, 0.0),
            ],
            buffered_boundary_2d,
            Some(format!("buffer-{surface_id}")),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned()),
        )
    }

    fn buffer(x: f64, y: f64, width: f64, height: f64, offset: f64) -> Vec<Point2> {
        vec![
            Point2::new(x - offset, y - offset),
            Point2::new(x + width + offset, y - offset),
            Point2::new(x + width + offset, y + height + offset),
            Point2::new(x - offset, y + height + offset),
        ]
    }

    fn profile(policy: AreaAggregationPolicy) -> AreaOverlapProfile {
        AreaOverlapProfile {
            profile_id: "analytic-v0.1".to_owned(),
            metric_authority: match policy {
                AreaAggregationPolicy::PairwiseSum2018 => AreaMetricAuthority::EvaluationFramework2018,
                AreaAggregationPolicy::UnionPerReference2021 => AreaMetricAuthority::FinalResultsPaper2021,
            },
            coordinate_unit: AreaCoordinateUnit::Metre,
            aggregation_policy: policy,
            common_frame_id: "LOCAL-A".to_owned(),
            max_parallel_angle_rad: 0.01,
            buffer_distance_m: 0.0,
            buffer_policy_id: "synthetic-precomputed-buffer-v1".to_owned(),
            planarity_tolerance_m: 1.0e-6,
        }
    }

    #[test]
    fn published_metric_authority_must_match_its_aggregation_policy() {
        let mut profile = profile(AreaAggregationPolicy::UnionPerReference2021);
        profile.metric_authority = AreaMetricAuthority::EvaluationFramework2018;
        let error = validate_area_overlap_profile(&profile)
            .expect_err("2018 pairwise aggregation cannot be labelled as 2021 union");
        assert!(error.contains("must match its named metric authority"));
    }

    #[test]
    fn zero_or_non_finite_parallel_threshold_is_rejected_when_invalid() {
        let mut value = profile(AreaAggregationPolicy::PairwiseSum2018);
        value.max_parallel_angle_rad = f64::NAN;
        let error = validate_area_overlap_profile(&value)
            .expect_err("NaN parallel threshold is invalid");
        assert!(error.contains("max_parallel_angle_rad"));

        let mut value = profile(AreaAggregationPolicy::PairwiseSum2018);
        value.planarity_tolerance_m = 0.0;
        let error = validate_area_overlap_profile(&value)
            .expect_err("zero planarity tolerance is invalid");
        assert!(error.contains("planarity_tolerance_m"));
    }

    #[test]
    fn metre_and_millimetre_area_profiles_produce_identical_si_results() {
        let ref_surface = reference(
            "REF",
            ReferenceVisibility::Observed,
            0.0, 0.0, 2.0, 1.0,
            buffer(0.0, 0.0, 2.0, 1.0, 0.0),
        );
        let src_surface = square_3d("SRC", "LOCAL-A", 0.0, 0.0, 0.0, 2.0, 1.0);
        let metric_m = evaluate_area_overlap(
            &profile(AreaAggregationPolicy::PairwiseSum2018),
            std::slice::from_ref(&src_surface),
            std::slice::from_ref(&ref_surface),
        )
        .expect("metre input");

        let mut profile_mm = profile(AreaAggregationPolicy::PairwiseSum2018);
        profile_mm.coordinate_unit = AreaCoordinateUnit::Millimetre;
        let reference_mm = ReferenceAreaSurface::new(
            "REF",
            "reference-model-REF",
            "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "LOCAL-A",
            ReferenceVisibility::Observed,
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(2000.0, 0.0, 0.0),
                Point3::new(2000.0, 1000.0, 0.0),
                Point3::new(0.0, 1000.0, 0.0),
            ],
            buffer(0.0, 0.0, 2000.0, 1000.0, 0.0),
            Some("buffer-REF".to_owned()),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned()),
        );
        let source_mm = square_3d("SRC", "LOCAL-A", 0.0, 0.0, 0.0, 2000.0, 1000.0);
        let metric_mm = evaluate_area_overlap(&profile_mm, &[source_mm], &[reference_mm])
            .expect("millimetre input");
        assert!((metric_m.matched_area_numerator_m2 - metric_mm.matched_area_numerator_m2).abs() < 1e-12);
        assert!((metric_mm.source_surface_area_m2 - 2.0).abs() < 1e-12);
        assert_eq!(metric_mm.coordinate_unit, AreaCoordinateUnit::Millimetre);
    }

    #[test]
    fn area_result_carries_geometry_and_buffer_artifact_provenance() {
        let reference = reference(
            "REF",
            ReferenceVisibility::Observed,
            0.0, 0.0, 1.0, 1.0,
            buffer(0.0, 0.0, 1.0, 1.0, 0.0),
        );
        let source = square_3d("SRC", "LOCAL-A", 0.0, 0.0, 0.0, 1.0, 1.0);
        let result = evaluate_area_overlap(
            &profile(AreaAggregationPolicy::UnionPerReference2021),
            &[source],
            &[reference],
        )
        .expect("content-addressed geometry inputs");
        assert_eq!(result.source_geometry_artifact_ids, vec!["source-model-SRC"]);
        assert_eq!(result.reference_geometry_artifact_ids, vec!["reference-model-REF"]);
        assert_eq!(result.buffer_artifact_ids, vec!["buffer-REF"]);
        assert_eq!(result.source_geometry_artifact_digests.len(), 1);
        assert_eq!(result.reference_geometry_artifact_digests.len(), 1);
        assert_eq!(result.buffer_artifact_digests.len(), 1);
    }

    #[test]
    fn exact_match_produces_unit_completeness_and_correctness() {
        let reference = reference(
            "REF",
            ReferenceVisibility::Observed,
            0.0,
            0.0,
            2.0,
            1.0,
            buffer(0.0, 0.0, 2.0, 1.0, 0.0),
        );
        let source = square_3d("SRC", "LOCAL-A", 0.0, 0.0, 0.0, 2.0, 1.0);
        let result = evaluate_area_overlap(&profile(AreaAggregationPolicy::PairwiseSum2018), &[source], &[reference])
            .expect("exact matching rectangles");
        assert!((result.source_surface_area_m2 - 2.0).abs() < 1e-12);
        assert!((result.observable_reference_surface_area_m2 - 2.0).abs() < 1e-12);
        assert!((result.matched_area_numerator_m2 - 2.0).abs() < 1e-12);
        assert_eq!(result.completeness, Some(1.0));
        assert_eq!(result.correctness, Some(1.0));
    }

    #[test]
    fn partial_coverage_keeps_separate_reference_and_source_denominators() {
        let reference = reference(
            "REF",
            ReferenceVisibility::Observed,
            0.0,
            0.0,
            2.0,
            1.0,
            buffer(0.0, 0.0, 2.0, 1.0, 0.0),
        );
        let source = square_3d("SRC", "LOCAL-A", 0.0, 0.0, 0.0, 1.0, 1.0);
        let result = evaluate_area_overlap(&profile(AreaAggregationPolicy::PairwiseSum2018), &[source], &[reference])
            .expect("partial overlap");
        assert!((result.matched_area_numerator_m2 - 1.0).abs() < 1e-12);
        assert_eq!(result.completeness, Some(0.5));
        assert_eq!(result.correctness, Some(1.0));
    }

    #[test]
    fn precomputed_boundary_buffer_changes_overlap_without_changing_denominator() {
        let reference = reference(
            "REF",
            ReferenceVisibility::Observed,
            0.0,
            0.0,
            2.0,
            1.0,
            buffer(0.0, 0.0, 2.0, 1.0, 0.25),
        );
        let source = square_3d("SRC", "LOCAL-A", 2.1, 0.0, 0.0, 0.2, 1.0);
        let result = evaluate_area_overlap(&profile(AreaAggregationPolicy::PairwiseSum2018), &[source], &[reference])
            .expect("buffer admits outside boundary sliver");
        assert!((result.observable_reference_surface_area_m2 - 2.0).abs() < 1e-12);
        assert!((result.matched_area_numerator_m2 - 0.15).abs() < 1e-12);
    }

    #[test]
    fn interpreted_and_unobservable_reference_surfaces_are_excluded_from_reference_denominator() {
        let observed = reference(
            "OBS",
            ReferenceVisibility::Observed,
            0.0, 0.0, 1.0, 1.0,
            buffer(0.0, 0.0, 1.0, 1.0, 0.0),
        );
        let interpreted = reference(
            "INT",
            ReferenceVisibility::Interpreted,
            2.0, 0.0, 2.0, 1.0,
            buffer(2.0, 0.0, 2.0, 1.0, 0.0),
        );
        let unobservable = reference(
            "HID",
            ReferenceVisibility::Unobservable,
            5.0, 0.0, 3.0, 1.0,
            buffer(5.0, 0.0, 3.0, 1.0, 0.0),
        );
        let source = square_3d("SRC", "LOCAL-A", 0.0, 0.0, 0.0, 1.0, 1.0);
        let result = evaluate_area_overlap(
            &profile(AreaAggregationPolicy::UnionPerReference2021),
            &[source],
            &[observed, interpreted, unobservable],
        )
        .expect("excluded reference strata");
        assert!((result.observable_reference_surface_area_m2 - 1.0).abs() < 1e-12);
        assert!((result.interpreted_reference_area_excluded_m2 - 2.0).abs() < 1e-12);
        assert!((result.unobservable_reference_area_excluded_m2 - 3.0).abs() < 1e-12);
        assert_eq!(result.completeness, Some(1.0));
    }

    #[test]
    fn parallel_angle_threshold_is_explicit_and_rejects_incompatible_planes() {
        let reference = reference(
            "REF",
            ReferenceVisibility::Observed,
            0.0, 0.0, 1.0, 1.0,
            buffer(0.0, 0.0, 1.0, 1.0, 0.0),
        );
        let mut source = square_3d("SRC", "LOCAL-A", 0.0, 0.0, 0.0, 1.0, 1.0);
        source.vertices = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.1),
            Point3::new(0.0, 1.0, 0.1),
        ];
        let mut profile = profile(AreaAggregationPolicy::PairwiseSum2018);
        profile.max_parallel_angle_rad = 0.01;
        let result = evaluate_area_overlap(&profile, &[source], &[reference])
            .expect("tilted source is geometrically valid");
        assert_eq!(result.skipped_non_parallel_pair_count, 1);
        assert_eq!(result.matched_area_numerator_m2, 0.0);
    }

    #[test]
    fn pairwise_2018_and_union_per_reference_2021_are_distinct_on_overlapping_sources() {
        let reference = reference(
            "REF",
            ReferenceVisibility::Observed,
            0.0, 0.0, 2.0, 1.0,
            buffer(0.0, 0.0, 2.0, 1.0, 0.0),
        );
        let first = square_3d("SRC-A", "LOCAL-A", 0.0, 0.0, 0.0, 1.5, 1.0);
        let second = square_3d("SRC-B", "LOCAL-A", 0.5, 0.0, 0.0, 1.5, 1.0);
        let pairwise = evaluate_area_overlap(
            &profile(AreaAggregationPolicy::PairwiseSum2018),
            &[first.clone(), second.clone()],
            &[reference.clone()],
        )
        .expect("pairwise profile");
        let union = evaluate_area_overlap(
            &profile(AreaAggregationPolicy::UnionPerReference2021),
            &[first, second],
            &[reference],
        )
        .expect("union profile");
        assert!((pairwise.matched_area_numerator_m2 - 3.0).abs() < 1e-12);
        assert!((union.matched_area_numerator_m2 - 2.0).abs() < 1e-9);
        assert!(pairwise.completeness_exceeds_one);
        assert!((union.completeness.expect("completeness") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn empty_source_has_zero_completeness_and_undefined_correctness() {
        let reference = reference(
            "REF",
            ReferenceVisibility::Observed,
            0.0, 0.0, 2.0, 1.0,
            buffer(0.0, 0.0, 2.0, 1.0, 0.0),
        );
        let result = evaluate_area_overlap(
            &profile(AreaAggregationPolicy::UnionPerReference2021),
            &[],
            &[reference],
        )
        .expect("an empty source model is a valid zero-completeness case");
        assert_eq!(result.matched_area_numerator_m2, 0.0);
        assert_eq!(result.completeness, Some(0.0));
        assert_eq!(result.correctness, None);
    }

    #[test]
    fn empty_observable_reference_denominator_is_reported_as_undefined() {
        let source = square_3d("SRC", "LOCAL-A", 0.0, 0.0, 0.0, 1.0, 1.0);
        let interpreted = reference(
            "REF-INTERPRETED",
            ReferenceVisibility::Interpreted,
            0.0, 0.0, 1.0, 1.0,
            buffer(0.0, 0.0, 1.0, 1.0, 0.0),
        );
        let result = evaluate_area_overlap(
            &profile(AreaAggregationPolicy::UnionPerReference2021),
            &[source],
            &[interpreted],
        )
        .expect("interpreted references are excluded from observed denominator");
        assert_eq!(result.completeness, None);
        assert_eq!(result.correctness, Some(0.0));
    }

    #[test]
    fn empty_reference_set_does_not_fabricate_a_completeness_score() {
        let source = square_3d("SRC", "LOCAL-A", 0.0, 0.0, 0.0, 1.0, 1.0);
        let result = evaluate_area_overlap(
            &profile(AreaAggregationPolicy::UnionPerReference2021),
            &[source],
            &[],
        )
        .expect("empty reference set has a zero reference denominator");
        assert_eq!(result.completeness, None);
        assert_eq!(result.correctness, Some(0.0));
    }

    #[test]
    fn wrong_frame_non_convex_and_invalid_angle_fail_closed() {
        let ref_surface = reference(
            "REF",
            ReferenceVisibility::Observed,
            0.0, 0.0, 1.0, 1.0,
            buffer(0.0, 0.0, 1.0, 1.0, 0.0),
        );
        let mut source = square_3d("SRC", "OTHER", 0.0, 0.0, 0.0, 1.0, 1.0);
        let frame_error = evaluate_area_overlap(
            &profile(AreaAggregationPolicy::PairwiseSum2018),
            &[source.clone()],
            &[ref_surface.clone()],
        )
        .expect_err("source frame mismatch");
        assert!(frame_error.contains("frame does not match"));

        source.frame_id = "LOCAL-A".to_owned();
        source.vertices = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(1.0, 0.5, 0.0),
            Point3::new(2.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let shape_error = evaluate_area_overlap(
            &profile(AreaAggregationPolicy::PairwiseSum2018),
            &[source],
            &[ref_surface.clone()],
        )
        .expect_err("concave source surface is unsupported in v0.1");
        assert!(shape_error.contains("must be convex"));

        let mut invalid_profile = profile(AreaAggregationPolicy::PairwiseSum2018);
        invalid_profile.max_parallel_angle_rad = f64::NAN;
        let angle_error = evaluate_area_overlap(&invalid_profile, &[], &[ref_surface])
            .expect_err("NaN angle threshold is invalid");
        assert!(angle_error.contains("max_parallel_angle_rad"));
    }
}
