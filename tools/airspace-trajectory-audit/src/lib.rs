//! Synthetic-only, deterministic 4D trajectory conflict screening.
//!
//! Coordinates are local Cartesian metres in one explicitly named reference frame; z uses an
//! explicitly named vertical datum. Timestamps are UTC Unix milliseconds. Trajectories are
//! piecewise-linear between declared points and carry constant, conservative uncertainty bounds.
//!
//! This is a research benchmark kernel, not an ATC, UTM, clearance, dispatch, flight-control,
//! separation-assurance, or safety-certified system. Separation criteria must be explicitly
//! supplied for each synthetic scenario; this crate does not define operational minima.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::error::Error;
use std::fmt;

const MAX_EXACT_UTC_MILLIS: i64 = 9_007_199_254_740_991;

/// One point on a piecewise-linear declared trajectory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrajectoryPoint {
    /// UTC Unix timestamp in milliseconds.
    pub time_utc_ms: i64,
    /// Local Cartesian x coordinate in metres.
    pub x_m: f64,
    /// Local Cartesian y coordinate in metres.
    pub y_m: f64,
    /// Vertical coordinate in metres from the trajectory's declared vertical datum.
    pub z_m: f64,
}

/// A versioned, evidence-referenced trajectory for one synthetic or offline scenario.
///
/// Points are joined by straight segments in local Cartesian coordinates. Horizontal and
/// vertical uncertainty values are conservative bounds applied throughout the entire trajectory;
/// they are not standard deviations and are not a calibrated error distribution.
#[derive(Debug, Clone, PartialEq)]
pub struct Trajectory {
    /// Stable trajectory identifier.
    pub trajectory_id: String,
    /// Exact revision of this trajectory.
    pub revision: String,
    /// Explicit shared local Cartesian reference-frame identifier.
    pub reference_frame_id: String,
    /// Explicit vertical datum identifier.
    pub vertical_datum_id: String,
    /// Locator for the source data or synthetic scenario manifest.
    pub source_reference: String,
    /// SHA-256 syntax for the referenced source bytes; authenticity is not verified.
    pub source_sha256: String,
    /// Maximum horizontal radial deviation from the centerline, in metres.
    pub horizontal_uncertainty_bound_m: f64,
    /// Maximum vertical deviation from the centerline, in metres.
    pub vertical_uncertainty_bound_m: f64,
    /// Ordered trajectory points; each adjacent pair defines a linear segment.
    pub points: Vec<TrajectoryPoint>,
}

/// Scenario-supplied separation criteria with explicit jurisdiction and time validity.
///
/// Values are input parameters for the specific synthetic/offline case. They are not built-in
/// aviation standards or universal minima. The referenced document and digest are not authenticated.
#[derive(Debug, Clone, PartialEq)]
pub struct SeparationCriteria {
    /// Stable scenario criteria identifier.
    pub criteria_id: String,
    /// Exact criteria revision.
    pub revision: String,
    /// Jurisdiction or fictional scenario scope to which the criteria claim to apply.
    pub jurisdiction: String,
    /// Source locator for the criteria or synthetic scenario manifest.
    pub source_reference: String,
    /// SHA-256 syntax for referenced source bytes; authenticity is not verified.
    pub source_sha256: String,
    /// Earliest UTC Unix millisecond at which the criteria are declared valid.
    pub valid_from_utc_ms: i64,
    /// Latest UTC Unix millisecond at which the criteria are declared valid, inclusive.
    pub valid_until_utc_ms: i64,
    /// Required horizontal centerline separation in metres before uncertainty expansion.
    pub minimum_horizontal_separation_m: f64,
    /// Required vertical centerline separation in metres before uncertainty expansion.
    pub minimum_vertical_separation_m: f64,
}

/// Disposition of one pairwise trajectory audit.
///
/// Even a no-conflict result applies only to the supplied trajectory windows, declared bounds,
/// linear-motion model, coordinate frame, and criteria. It does not authorize a real operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictDisposition {
    /// At least one possible conflict is detected inside a criteria-valid interval.
    ConflictDetected,
    /// The declared trajectories have no overlapping time interval.
    NoTemporalOverlap,
    /// No conflict was detected and the criteria cover the full overlapping time interval.
    NoConflictDetectedWithinEvaluatedWindow,
    /// The criteria do not cover the full overlapping time interval, and no conflict was found
    /// in the portion that could be assessed.
    NotAssessable,
}

/// One point at which the conservative trajectory tubes violate the declared separation bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictEvent {
    /// Closest-approach UTC timestamp, rounded to the nearest millisecond within the segment pair.
    pub closest_approach_utc_ms: i64,
    /// Horizontal centerline separation at the reported time, in metres.
    pub horizontal_separation_m: f64,
    /// Absolute vertical centerline separation at the reported time, in metres.
    pub vertical_separation_m: f64,
    /// Zero-based segment index for the canonical first trajectory.
    pub segment_a_index: usize,
    /// Zero-based segment index for the canonical second trajectory.
    pub segment_b_index: usize,
}

/// Deterministic report for a single pairwise trajectory screening operation.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictAuditReport {
    /// Canonically ordered first trajectory identifier.
    pub trajectory_a_id: String,
    /// Revision of the first trajectory.
    pub trajectory_a_revision: String,
    /// Canonically ordered second trajectory identifier.
    pub trajectory_b_id: String,
    /// Revision of the second trajectory.
    pub trajectory_b_revision: String,
    /// Criteria identifier used by the audit.
    pub criteria_id: String,
    /// Revision of the criteria used by the audit.
    pub criteria_revision: String,
    /// Full temporal overlap of the declared trajectory windows, if any.
    pub temporal_overlap_utc_ms: Option<(i64, i64)>,
    /// Time interval actually evaluated under the criteria validity window, if any.
    pub evaluated_window_utc_ms: Option<(i64, i64)>,
    /// Whether criteria validity fully covers the declared trajectory overlap.
    ///
    /// This is true when the trajectories do not overlap in time because no concurrent interval
    /// requires separation-criteria coverage. For a real overlap, false means some interval is unknown.
    pub criteria_covers_declared_overlap: bool,
    /// Base horizontal criterion plus both trajectory horizontal uncertainty bounds, in metres.
    pub effective_horizontal_bound_m: f64,
    /// Base vertical criterion plus both trajectory vertical uncertainty bounds, in metres.
    pub effective_vertical_bound_m: f64,
    /// Pairwise screening result; never an operational authorization.
    pub disposition: ConflictDisposition,
    /// Reported conflict events, ordered deterministically by time and geometry.
    pub conflicts: Vec<ConflictEvent>,
    /// Always false: this prototype checks digest syntax, not source or signer authenticity.
    pub source_authenticity_verified: bool,
}

/// Invalid input or arithmetic condition that prevents a meaningful pairwise audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditInputError {
    /// A trajectory field or point sequence is missing, malformed, or outside representable time.
    InvalidTrajectory(&'static str),
    /// A criteria field, digest, threshold or validity interval is malformed.
    InvalidCriteria(&'static str),
    /// Both inputs use the same trajectory identifier.
    SameTrajectoryId,
    /// Trajectories name incompatible local Cartesian reference frames.
    ReferenceFrameMismatch,
    /// Trajectories name incompatible vertical datums.
    VerticalDatumMismatch,
    /// An arithmetic operation overflowed or produced a non-finite intermediate result.
    NumericOverflow,
    /// A non-zero product became zero due to floating-point underflow.
    NumericPrecisionLoss,
}

impl fmt::Display for AuditInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTrajectory(reason) => write!(f, "invalid trajectory: {reason}"),
            Self::InvalidCriteria(reason) => write!(f, "invalid separation criteria: {reason}"),
            Self::SameTrajectoryId => write!(f, "trajectory identifiers must be distinct"),
            Self::ReferenceFrameMismatch => {
                write!(f, "trajectory coordinate reference frames do not match")
            }
            Self::VerticalDatumMismatch => {
                write!(f, "trajectory vertical datums do not match")
            }
            Self::NumericOverflow => {
                write!(f, "numeric overflow during trajectory audit")
            }
            Self::NumericPrecisionLoss => {
                write!(f, "numeric precision loss during trajectory audit")
            }
        }
    }
}

impl Error for AuditInputError {}

/// One conservatively selected trajectory pair, expressed as indices into the input fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TrajectoryPairCandidate {
    /// Index of one trajectory in the input slice; always less than `second_index`.
    pub first_index: usize,
    /// Index of the other trajectory in the input slice.
    pub second_index: usize,
}

#[derive(Debug, Clone, Copy)]
struct ExpandedTrajectoryEnvelope {
    input_index: usize,
    start_utc_ms: i64,
    end_utc_ms: i64,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    min_z: f64,
    max_z: f64,
}

/// Generate conservative candidates for one fleet in a common Cartesian frame and vertical datum.
///
/// This sweep-and-prune broadphase only filters geometry candidates; callers must still run
/// `audit_trajectory_pair` for every returned pair. It fails closed unless the scenario criteria
/// cover the full min-start/max-end time span of the input slice, since otherwise pruning could
/// conceal a `NotAssessable` outcome. Inputs with mixed frames/datums are rejected rather than
/// silently compared.
///
/// Each trajectory's full path AABB is expanded by (base threshold + its own uncertainty) on each
/// side. This deliberately double-counts the base threshold across a pair, allowing false positives
/// in exchange for conservative candidate recall. Expanded bounds round outward by one representable
/// floating-point step. Non-finite expanded bounds fail closed with `NumericOverflow`.
///
/// Returned indices refer to the original slice, and pairs are sorted deterministically. This
/// prototype is not ATC/UTM, clearance, dispatch, or flight-control software.
pub fn broadphase_candidate_pairs(
    trajectories: &[Trajectory],
    criteria: &SeparationCriteria,
) -> Result<Vec<TrajectoryPairCandidate>, AuditInputError> {
    validate_criteria(criteria)?;
    if trajectories.is_empty() {
        return Ok(Vec::new());
    }

    let mut ids = std::collections::BTreeSet::new();
    for trajectory in trajectories {
        validate_trajectory(trajectory)?;
        if !ids.insert(trajectory.trajectory_id.as_str()) {
            return Err(AuditInputError::SameTrajectoryId);
        }
    }

    let frame = &trajectories[0].reference_frame_id;
    let datum = &trajectories[0].vertical_datum_id;
    if trajectories.iter().any(|t| &t.reference_frame_id != frame) {
        return Err(AuditInputError::ReferenceFrameMismatch);
    }
    if trajectories.iter().any(|t| &t.vertical_datum_id != datum) {
        return Err(AuditInputError::VerticalDatumMismatch);
    }

    let fleet_start = trajectories
        .iter()
        .map(|t| t.points[0].time_utc_ms)
        .min()
        .ok_or(AuditInputError::InvalidTrajectory(
            "trajectory has no points",
        ))?;
    let fleet_end = trajectories
        .iter()
        .map(|t| t.points[t.points.len() - 1].time_utc_ms)
        .max()
        .ok_or(AuditInputError::InvalidTrajectory(
            "trajectory has no points",
        ))?;
    if criteria.valid_from_utc_ms > fleet_start || criteria.valid_until_utc_ms < fleet_end {
        return Err(AuditInputError::InvalidCriteria(
            "broadphase requires criteria validity to cover the full fleet time span",
        ));
    }

    let mut envelopes = Vec::with_capacity(trajectories.len());
    for (input_index, trajectory) in trajectories.iter().enumerate() {
        let horizontal_margin = checked_add(
            criteria.minimum_horizontal_separation_m,
            trajectory.horizontal_uncertainty_bound_m,
        )?
        .next_up();
        let vertical_margin = checked_add(
            criteria.minimum_vertical_separation_m,
            trajectory.vertical_uncertainty_bound_m,
        )?
        .next_up();
        if !horizontal_margin.is_finite() || !vertical_margin.is_finite() {
            return Err(AuditInputError::NumericOverflow);
        }
        let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut min_z, mut max_z) = (f64::INFINITY, f64::NEG_INFINITY);
        for point in &trajectory.points {
            min_x = min_x.min(point.x_m);
            max_x = max_x.max(point.x_m);
            min_y = min_y.min(point.y_m);
            max_y = max_y.max(point.y_m);
            min_z = min_z.min(point.z_m);
            max_z = max_z.max(point.z_m);
        }
        envelopes.push(ExpandedTrajectoryEnvelope {
            input_index,
            start_utc_ms: trajectory.points[0].time_utc_ms,
            end_utc_ms: trajectory.points[trajectory.points.len() - 1].time_utc_ms,
            min_x: outward_lower(min_x, horizontal_margin)?,
            max_x: outward_upper(max_x, horizontal_margin)?,
            min_y: outward_lower(min_y, horizontal_margin)?,
            max_y: outward_upper(max_y, horizontal_margin)?,
            min_z: outward_lower(min_z, vertical_margin)?,
            max_z: outward_upper(max_z, vertical_margin)?,
        });
    }

    envelopes.sort_by(|a, b| {
        a.min_x
            .total_cmp(&b.min_x)
            .then_with(|| a.input_index.cmp(&b.input_index))
    });
    let mut active: Vec<ExpandedTrajectoryEnvelope> = Vec::new();
    let mut candidates = Vec::new();
    for current in envelopes {
        active.retain(|previous| previous.max_x >= current.min_x);
        for previous in &active {
            if previous.end_utc_ms < current.start_utc_ms
                || current.end_utc_ms < previous.start_utc_ms
            {
                continue;
            }
            if previous.max_y < current.min_y || current.max_y < previous.min_y {
                continue;
            }
            if previous.max_z < current.min_z || current.max_z < previous.min_z {
                continue;
            }
            let (first_index, second_index) = if previous.input_index < current.input_index {
                (previous.input_index, current.input_index)
            } else {
                (current.input_index, previous.input_index)
            };
            candidates.push(TrajectoryPairCandidate {
                first_index,
                second_index,
            });
        }
        active.push(current);
    }
    candidates.sort_unstable();
    candidates.dedup();
    Ok(candidates)
}

fn outward_lower(value: f64, margin: f64) -> Result<f64, AuditInputError> {
    let expanded = value - margin;
    if !expanded.is_finite() {
        return Err(AuditInputError::NumericOverflow);
    }
    let outward = expanded.next_down();
    if outward.is_finite() {
        Ok(outward)
    } else {
        Err(AuditInputError::NumericOverflow)
    }
}

fn outward_upper(value: f64, margin: f64) -> Result<f64, AuditInputError> {
    let expanded = value + margin;
    if !expanded.is_finite() {
        return Err(AuditInputError::NumericOverflow);
    }
    let outward = expanded.next_up();
    if outward.is_finite() {
        Ok(outward)
    } else {
        Err(AuditInputError::NumericOverflow)
    }
}

/// Audit two trajectories against explicit, time-bounded separation criteria.
///
/// The algorithm checks each overlapping pair of linear segments. It clips each segment-pair
/// interval to the criteria-valid interval, computes the interval where vertical separation is
/// within the conservatively expanded vertical bound, then minimizes horizontal separation over
/// that interval. Any detected conflict takes precedence over an incomplete criteria window.
/// If no conflict is found but criteria omit part of the trajectories' temporal overlap, the
/// result is NotAssessable.
///
/// Inputs with mismatched frames/datums, invalid provenance syntax, malformed point ordering,
/// unsupported time values, or numeric overflow return an error. No source or criteria authenticity
/// is inferred from the presence of references or hashes.
pub fn audit_trajectory_pair(
    first: &Trajectory,
    second: &Trajectory,
    criteria: &SeparationCriteria,
) -> Result<ConflictAuditReport, AuditInputError> {
    validate_trajectory(first)?;
    validate_trajectory(second)?;
    validate_criteria(criteria)?;

    if first.trajectory_id == second.trajectory_id {
        return Err(AuditInputError::SameTrajectoryId);
    }
    if first.reference_frame_id != second.reference_frame_id {
        return Err(AuditInputError::ReferenceFrameMismatch);
    }
    if first.vertical_datum_id != second.vertical_datum_id {
        return Err(AuditInputError::VerticalDatumMismatch);
    }

    let (a, b) = if first.trajectory_id <= second.trajectory_id {
        (first, second)
    } else {
        (second, first)
    };

    let effective_horizontal_bound_m = checked_add(
        checked_add(
            criteria.minimum_horizontal_separation_m,
            a.horizontal_uncertainty_bound_m,
        )?,
        b.horizontal_uncertainty_bound_m,
    )?;
    let effective_vertical_bound_m = checked_add(
        checked_add(
            criteria.minimum_vertical_separation_m,
            a.vertical_uncertainty_bound_m,
        )?,
        b.vertical_uncertainty_bound_m,
    )?;

    let a_start = a
        .points
        .first()
        .ok_or(AuditInputError::InvalidTrajectory(
            "at least two trajectory points are required",
        ))?
        .time_utc_ms;
    let a_end = a
        .points
        .last()
        .ok_or(AuditInputError::InvalidTrajectory(
            "at least two trajectory points are required",
        ))?
        .time_utc_ms;
    let b_start = b
        .points
        .first()
        .ok_or(AuditInputError::InvalidTrajectory(
            "at least two trajectory points are required",
        ))?
        .time_utc_ms;
    let b_end = b
        .points
        .last()
        .ok_or(AuditInputError::InvalidTrajectory(
            "at least two trajectory points are required",
        ))?
        .time_utc_ms;

    let overlap_start = a_start.max(b_start);
    let overlap_end = a_end.min(b_end);

    if overlap_start > overlap_end {
        return Ok(report(
            a,
            b,
            criteria,
            None,
            None,
            true,
            effective_horizontal_bound_m,
            effective_vertical_bound_m,
            ConflictDisposition::NoTemporalOverlap,
            Vec::new(),
        ));
    }

    let criteria_covers_declared_overlap =
        criteria.valid_from_utc_ms <= overlap_start && criteria.valid_until_utc_ms >= overlap_end;
    let evaluated_start = overlap_start.max(criteria.valid_from_utc_ms);
    let evaluated_end = overlap_end.min(criteria.valid_until_utc_ms);

    if evaluated_start > evaluated_end {
        return Ok(report(
            a,
            b,
            criteria,
            Some((overlap_start, overlap_end)),
            None,
            false,
            effective_horizontal_bound_m,
            effective_vertical_bound_m,
            ConflictDisposition::NotAssessable,
            Vec::new(),
        ));
    }

    let mut conflicts = Vec::new();
    for (index_a, segment_a) in a.points.windows(2).enumerate() {
        for (index_b, segment_b) in b.points.windows(2).enumerate() {
            let start = segment_a[0]
                .time_utc_ms
                .max(segment_b[0].time_utc_ms)
                .max(evaluated_start);
            let end = segment_a[1]
                .time_utc_ms
                .min(segment_b[1].time_utc_ms)
                .min(evaluated_end);
            if start > end {
                continue;
            }

            if let Some(event) = evaluate_segment_pair(
                segment_a[0],
                segment_a[1],
                segment_b[0],
                segment_b[1],
                start,
                end,
                effective_horizontal_bound_m,
                effective_vertical_bound_m,
                index_a,
                index_b,
            )? {
                conflicts.push(event);
            }
        }
    }

    let conflicts = sort_and_deduplicate_conflicts(conflicts);
    let disposition = if conflicts.is_empty() {
        if criteria_covers_declared_overlap {
            ConflictDisposition::NoConflictDetectedWithinEvaluatedWindow
        } else {
            ConflictDisposition::NotAssessable
        }
    } else {
        ConflictDisposition::ConflictDetected
    };

    Ok(report(
        a,
        b,
        criteria,
        Some((overlap_start, overlap_end)),
        Some((evaluated_start, evaluated_end)),
        criteria_covers_declared_overlap,
        effective_horizontal_bound_m,
        effective_vertical_bound_m,
        disposition,
        conflicts,
    ))
}

#[allow(clippy::too_many_arguments)]
fn report(
    a: &Trajectory,
    b: &Trajectory,
    criteria: &SeparationCriteria,
    temporal_overlap_utc_ms: Option<(i64, i64)>,
    evaluated_window_utc_ms: Option<(i64, i64)>,
    criteria_covers_declared_overlap: bool,
    effective_horizontal_bound_m: f64,
    effective_vertical_bound_m: f64,
    disposition: ConflictDisposition,
    conflicts: Vec<ConflictEvent>,
) -> ConflictAuditReport {
    ConflictAuditReport {
        trajectory_a_id: a.trajectory_id.clone(),
        trajectory_a_revision: a.revision.clone(),
        trajectory_b_id: b.trajectory_id.clone(),
        trajectory_b_revision: b.revision.clone(),
        criteria_id: criteria.criteria_id.clone(),
        criteria_revision: criteria.revision.clone(),
        temporal_overlap_utc_ms,
        evaluated_window_utc_ms,
        criteria_covers_declared_overlap,
        effective_horizontal_bound_m,
        effective_vertical_bound_m,
        disposition,
        conflicts,
        source_authenticity_verified: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_segment_pair(
    a0: TrajectoryPoint,
    a1: TrajectoryPoint,
    b0: TrajectoryPoint,
    b1: TrajectoryPoint,
    start_utc_ms: i64,
    end_utc_ms: i64,
    horizontal_bound_m: f64,
    vertical_bound_m: f64,
    segment_a_index: usize,
    segment_b_index: usize,
) -> Result<Option<ConflictEvent>, AuditInputError> {
    let a_start = interpolate_position(a0, a1, start_utc_ms)?;
    let a_end = interpolate_position(a0, a1, end_utc_ms)?;
    let b_start = interpolate_position(b0, b1, start_utc_ms)?;
    let b_end = interpolate_position(b0, b1, end_utc_ms)?;

    let dx_start = checked_sub(a_start.x_m, b_start.x_m)?;
    let dy_start = checked_sub(a_start.y_m, b_start.y_m)?;
    let dz_start = checked_sub(a_start.z_m, b_start.z_m)?;
    let dx_end = checked_sub(a_end.x_m, b_end.x_m)?;
    let dy_end = checked_sub(a_end.y_m, b_end.y_m)?;
    let dz_end = checked_sub(a_end.z_m, b_end.z_m)?;

    let Some((vertical_u_start, vertical_u_end)) =
        vertical_compatible_interval(dz_start, dz_end, vertical_bound_m)?
    else {
        return Ok(None);
    };

    let vx = checked_sub(dx_end, dx_start)?;
    let vy = checked_sub(dy_end, dy_start)?;
    let denominator = compensated_sum_products(vx, vx, vy, vy)?;
    let dot = compensated_sum_products(dx_start, vx, dy_start, vy)?;

    if denominator.0 == 0.0 && (denominator.1 != 0.0 || vx != 0.0 || vy != 0.0) {
        return Err(AuditInputError::NumericPrecisionLoss);
    }
    let unconstrained_u = if denominator.0 == 0.0 {
        vertical_u_start
    } else {
        compensated_divide((-dot.0, -dot.1), denominator)?
    };
    let u = unconstrained_u.clamp(vertical_u_start, vertical_u_end);

    let horizontal_x = checked_add(dx_start, checked_mul(vx, u)?)?;
    let horizontal_y = checked_add(dy_start, checked_mul(vy, u)?)?;
    let horizontal_separation_m = horizontal_x.hypot(horizontal_y);
    let vertical_delta = checked_sub(dz_end, dz_start)?;
    let vertical_separation_m = checked_add(dz_start, checked_mul(vertical_delta, u)?)?.abs();

    if !horizontal_separation_m.is_finite() || !vertical_separation_m.is_finite() {
        return Err(AuditInputError::NumericOverflow);
    }

    if horizontal_separation_m <= horizontal_bound_m && vertical_separation_m <= vertical_bound_m {
        let span_ms = end_utc_ms - start_utc_ms;
        let offset_f = (span_ms as f64) * u;
        if !offset_f.is_finite() || offset_f < 0.0 || offset_f > span_ms as f64 {
            return Err(AuditInputError::NumericOverflow);
        }
        let offset_ms = offset_f.round() as i64;
        let closest_approach_utc_ms = start_utc_ms
            .checked_add(offset_ms)
            .ok_or(AuditInputError::NumericOverflow)?;
        Ok(Some(ConflictEvent {
            closest_approach_utc_ms,
            horizontal_separation_m,
            vertical_separation_m,
            segment_a_index,
            segment_b_index,
        }))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy)]
struct Position {
    x_m: f64,
    y_m: f64,
    z_m: f64,
}

fn interpolate_position(
    start: TrajectoryPoint,
    end: TrajectoryPoint,
    time_utc_ms: i64,
) -> Result<Position, AuditInputError> {
    let span_ms = (end.time_utc_ms - start.time_utc_ms) as f64;
    if span_ms <= 0.0 || !span_ms.is_finite() {
        return Err(AuditInputError::InvalidTrajectory(
            "segment timestamps must be strictly increasing",
        ));
    }
    let elapsed_ms = (time_utc_ms - start.time_utc_ms) as f64;
    let fraction = elapsed_ms / span_ms;
    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
        return Err(AuditInputError::NumericOverflow);
    }

    Ok(Position {
        x_m: lerp_checked(start.x_m, end.x_m, fraction)?,
        y_m: lerp_checked(start.y_m, end.y_m, fraction)?,
        z_m: lerp_checked(start.z_m, end.z_m, fraction)?,
    })
}

fn lerp_checked(start: f64, end: f64, fraction: f64) -> Result<f64, AuditInputError> {
    checked_add(start, checked_mul(checked_sub(end, start)?, fraction)?)
}

/// Returns the closed normalized-u interval in [0, 1] where a linearly changing vertical
/// separation lies inside [-bound, bound].
fn vertical_compatible_interval(
    start_delta_m: f64,
    end_delta_m: f64,
    bound_m: f64,
) -> Result<Option<(f64, f64)>, AuditInputError> {
    let delta = checked_sub(end_delta_m, start_delta_m)?;
    if delta == 0.0 {
        return Ok(if start_delta_m.abs() <= bound_m {
            Some((0.0, 1.0))
        } else {
            None
        });
    }

    let lower_numerator = checked_sub(-bound_m, start_delta_m)?;
    let upper_numerator = checked_sub(bound_m, start_delta_m)?;
    let first_crossing = lower_numerator / delta;
    let second_crossing = upper_numerator / delta;
    if !first_crossing.is_finite() || !second_crossing.is_finite() {
        return Err(AuditInputError::NumericOverflow);
    }

    let u_start = 0.0_f64.max(first_crossing.min(second_crossing));
    let u_end = 1.0_f64.min(first_crossing.max(second_crossing));
    Ok(if u_start <= u_end {
        Some((u_start, u_end))
    } else {
        None
    })
}

fn sort_and_deduplicate_conflicts(mut events: Vec<ConflictEvent>) -> Vec<ConflictEvent> {
    events.sort_by(|left, right| {
        left.closest_approach_utc_ms
            .cmp(&right.closest_approach_utc_ms)
            .then_with(|| {
                left.horizontal_separation_m
                    .total_cmp(&right.horizontal_separation_m)
            })
            .then_with(|| {
                left.vertical_separation_m
                    .total_cmp(&right.vertical_separation_m)
            })
            .then_with(|| left.segment_a_index.cmp(&right.segment_a_index))
            .then_with(|| left.segment_b_index.cmp(&right.segment_b_index))
    });

    let mut deduplicated: Vec<ConflictEvent> = Vec::with_capacity(events.len());
    for event in events {
        match deduplicated.last_mut() {
            Some(previous) if previous.closest_approach_utc_ms == event.closest_approach_utc_ms => {
                if event.horizontal_separation_m < previous.horizontal_separation_m {
                    *previous = event;
                }
            }
            _ => deduplicated.push(event),
        }
    }
    deduplicated
}

fn validate_trajectory(trajectory: &Trajectory) -> Result<(), AuditInputError> {
    if trajectory.trajectory_id.trim().is_empty() {
        return Err(AuditInputError::InvalidTrajectory("trajectory id is empty"));
    }
    if trajectory.revision.trim().is_empty() {
        return Err(AuditInputError::InvalidTrajectory(
            "trajectory revision is empty",
        ));
    }
    if trajectory.reference_frame_id.trim().is_empty() {
        return Err(AuditInputError::InvalidTrajectory(
            "reference frame id is empty",
        ));
    }
    if trajectory.vertical_datum_id.trim().is_empty() {
        return Err(AuditInputError::InvalidTrajectory(
            "vertical datum id is empty",
        ));
    }
    if trajectory.source_reference.trim().is_empty() {
        return Err(AuditInputError::InvalidTrajectory(
            "source reference is empty",
        ));
    }
    if !is_sha256_hex(&trajectory.source_sha256) {
        return Err(AuditInputError::InvalidTrajectory(
            "source SHA-256 must contain 64 hexadecimal characters",
        ));
    }
    if !trajectory.horizontal_uncertainty_bound_m.is_finite()
        || trajectory.horizontal_uncertainty_bound_m < 0.0
        || !trajectory.vertical_uncertainty_bound_m.is_finite()
        || trajectory.vertical_uncertainty_bound_m < 0.0
    {
        return Err(AuditInputError::InvalidTrajectory(
            "uncertainty bounds must be finite and non-negative",
        ));
    }
    if trajectory.points.len() < 2 {
        return Err(AuditInputError::InvalidTrajectory(
            "at least two trajectory points are required",
        ));
    }

    for point in &trajectory.points {
        if point.time_utc_ms < 0 || point.time_utc_ms > MAX_EXACT_UTC_MILLIS {
            return Err(AuditInputError::InvalidTrajectory(
                "UTC milliseconds are outside the exact interpolation range",
            ));
        }
        if !point.x_m.is_finite() || !point.y_m.is_finite() || !point.z_m.is_finite() {
            return Err(AuditInputError::InvalidTrajectory(
                "coordinates must be finite",
            ));
        }
    }
    for pair in trajectory.points.windows(2) {
        if pair[1].time_utc_ms <= pair[0].time_utc_ms {
            return Err(AuditInputError::InvalidTrajectory(
                "point timestamps must be strictly increasing",
            ));
        }
    }
    Ok(())
}

fn validate_criteria(criteria: &SeparationCriteria) -> Result<(), AuditInputError> {
    if criteria.criteria_id.trim().is_empty() {
        return Err(AuditInputError::InvalidCriteria("criteria id is empty"));
    }
    if criteria.revision.trim().is_empty() {
        return Err(AuditInputError::InvalidCriteria(
            "criteria revision is empty",
        ));
    }
    if criteria.jurisdiction.trim().is_empty() {
        return Err(AuditInputError::InvalidCriteria("jurisdiction is empty"));
    }
    if criteria.source_reference.trim().is_empty() {
        return Err(AuditInputError::InvalidCriteria(
            "source reference is empty",
        ));
    }
    if !is_sha256_hex(&criteria.source_sha256) {
        return Err(AuditInputError::InvalidCriteria(
            "source SHA-256 must contain 64 hexadecimal characters",
        ));
    }
    if criteria.valid_from_utc_ms < 0
        || criteria.valid_until_utc_ms > MAX_EXACT_UTC_MILLIS
        || criteria.valid_from_utc_ms >= criteria.valid_until_utc_ms
    {
        return Err(AuditInputError::InvalidCriteria(
            "validity interval must be increasing and within the exact UTC range",
        ));
    }
    if !criteria.minimum_horizontal_separation_m.is_finite()
        || criteria.minimum_horizontal_separation_m <= 0.0
        || !criteria.minimum_vertical_separation_m.is_finite()
        || criteria.minimum_vertical_separation_m <= 0.0
    {
        return Err(AuditInputError::InvalidCriteria(
            "declared horizontal and vertical separation bounds must be positive and finite",
        ));
    }
    Ok(())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn checked_add(left: f64, right: f64) -> Result<f64, AuditInputError> {
    let result = left + right;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(AuditInputError::NumericOverflow)
    }
}

fn checked_sub(left: f64, right: f64) -> Result<f64, AuditInputError> {
    let result = left - right;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(AuditInputError::NumericOverflow)
    }
}

/// Sum two products with a retained floating-point residual for the minimizer's dot products.
///
/// Product residuals use fused multiply-add, and the summation uses error-free two-sum
/// transformations under ordinary IEEE-754 round-to-nearest semantics. Very small subnormal
/// products fail closed because the residual itself may not be representable.
fn compensated_sum_products(a: f64, b: f64, c: f64, d: f64) -> Result<(f64, f64), AuditInputError> {
    let (p, p_error) = product_with_residual(a, b)?;
    let (q, q_error) = product_with_residual(c, d)?;
    let (sum, sum_error) = two_sum(p, q)?;
    let (products_error, products_roundoff) = two_sum(p_error, q_error)?;
    let (low, low_roundoff) = two_sum(products_error, sum_error)?;
    let (high, renormalize_error) = two_sum(sum, low)?;
    let (low, renormalize_roundoff) = two_sum(renormalize_error, products_roundoff)?;
    let final_low = checked_add(checked_add(low, low_roundoff)?, renormalize_roundoff)?;
    Ok((high, final_low))
}

fn product_with_residual(a: f64, b: f64) -> Result<(f64, f64), AuditInputError> {
    let product = checked_mul(a, b)?;
    if product != 0.0 && product.abs() < f64::MIN_POSITIVE {
        return Err(AuditInputError::NumericPrecisionLoss);
    }
    let residual = a.mul_add(b, -product);
    if !residual.is_finite() {
        return Err(AuditInputError::NumericOverflow);
    }
    Ok((product, residual))
}

fn two_sum(a: f64, b: f64) -> Result<(f64, f64), AuditInputError> {
    let sum = checked_add(a, b)?;
    let b_virtual = sum - a;
    let a_virtual = sum - b_virtual;
    let b_roundoff = b - b_virtual;
    let a_roundoff = a - a_virtual;
    let residual = a_roundoff + b_roundoff;
    if !residual.is_finite() {
        return Err(AuditInputError::NumericOverflow);
    }
    Ok((sum, residual))
}

fn compensated_divide(
    numerator: (f64, f64),
    denominator: (f64, f64),
) -> Result<f64, AuditInputError> {
    let (numerator_high, numerator_low) = numerator;
    let (denominator_high, denominator_low) = denominator;
    if denominator_high == 0.0 {
        return Err(AuditInputError::NumericPrecisionLoss);
    }
    let quotient = numerator_high / denominator_high;
    if !quotient.is_finite() {
        return Err(AuditInputError::NumericOverflow);
    }
    if quotient == 0.0 && (numerator_high != 0.0 || numerator_low != 0.0) {
        return Err(AuditInputError::NumericPrecisionLoss);
    }

    // Residual of n - q*d, retaining the product of q and d_high with one fused rounding.
    let residual_high = (-quotient).mul_add(denominator_high, numerator_high);
    let denominator_correction = checked_mul(quotient, denominator_low)?;
    let residual = checked_add(
        checked_add(residual_high, numerator_low)?,
        -denominator_correction,
    )?;
    let correction = residual / denominator_high;
    if !correction.is_finite() {
        return Err(AuditInputError::NumericOverflow);
    }
    checked_add(quotient, correction)
}

fn checked_mul(left: f64, right: f64) -> Result<f64, AuditInputError> {
    let result = left * right;
    if !result.is_finite() {
        return Err(AuditInputError::NumericOverflow);
    }
    if left != 0.0 && right != 0.0 && result == 0.0 {
        return Err(AuditInputError::NumericPrecisionLoss);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(time_utc_ms: i64, x_m: f64, y_m: f64, z_m: f64) -> TrajectoryPoint {
        TrajectoryPoint {
            time_utc_ms,
            x_m,
            y_m,
            z_m,
        }
    }

    fn trajectory(
        id: &str,
        points: Vec<TrajectoryPoint>,
        horizontal_uncertainty_bound_m: f64,
        vertical_uncertainty_bound_m: f64,
    ) -> Trajectory {
        Trajectory {
            trajectory_id: id.to_owned(),
            revision: "rev-001".to_owned(),
            reference_frame_id: "SYNTHETIC-LOCAL-ENU-001".to_owned(),
            vertical_datum_id: "SYNTHETIC-VERTICAL-DATUM-001".to_owned(),
            source_reference: format!("fixture://trajectories/{id}"),
            source_sha256: "a".repeat(64),
            horizontal_uncertainty_bound_m,
            vertical_uncertainty_bound_m,
            points,
        }
    }

    fn criteria() -> SeparationCriteria {
        SeparationCriteria {
            criteria_id: "SYNTHETIC-SEPARATION-001".to_owned(),
            revision: "rev-001".to_owned(),
            jurisdiction: "fictional-test-only".to_owned(),
            source_reference: "fixture://criteria/synthetic-001".to_owned(),
            source_sha256: "b".repeat(64),
            valid_from_utc_ms: 0,
            valid_until_utc_ms: 30_000,
            minimum_horizontal_separation_m: 100.0,
            minimum_vertical_separation_m: 50.0,
        }
    }

    fn crossing_pair() -> (Trajectory, Trajectory) {
        (
            trajectory(
                "A",
                vec![
                    point(0, 0.0, 0.0, 100.0),
                    point(10_000, 1_000.0, 0.0, 100.0),
                ],
                0.0,
                0.0,
            ),
            trajectory(
                "B",
                vec![
                    point(0, 500.0, -500.0, 100.0),
                    point(10_000, 500.0, 500.0, 100.0),
                ],
                0.0,
                0.0,
            ),
        )
    }

    #[test]
    fn detects_intersecting_trajectories_at_closest_approach() {
        let (a, b) = crossing_pair();
        let report = audit_trajectory_pair(&a, &b, &criteria()).expect("valid fixture");
        assert_eq!(report.disposition, ConflictDisposition::ConflictDetected);
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].closest_approach_utc_ms, 5_000);
        assert_eq!(report.conflicts[0].horizontal_separation_m, 0.0);
        assert_eq!(report.conflicts[0].vertical_separation_m, 0.0);
        assert!(!report.source_authenticity_verified);
    }

    #[test]
    fn vertically_separated_crossing_is_not_a_declared_conflict() {
        let (a, mut b) = crossing_pair();
        for point in &mut b.points {
            point.z_m = 1_000.0;
        }
        let report = audit_trajectory_pair(&a, &b, &criteria()).expect("valid fixture");
        assert_eq!(
            report.disposition,
            ConflictDisposition::NoConflictDetectedWithinEvaluatedWindow
        );
        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn distant_parallel_paths_are_not_a_declared_conflict() {
        let a = trajectory(
            "A",
            vec![
                point(0, 0.0, 0.0, 100.0),
                point(10_000, 1_000.0, 0.0, 100.0),
            ],
            0.0,
            0.0,
        );
        let b = trajectory(
            "B",
            vec![
                point(0, 0.0, 500.0, 100.0),
                point(10_000, 1_000.0, 500.0, 100.0),
            ],
            0.0,
            0.0,
        );
        let report = audit_trajectory_pair(&a, &b, &criteria()).expect("valid fixture");
        assert_eq!(
            report.disposition,
            ConflictDisposition::NoConflictDetectedWithinEvaluatedWindow
        );
    }

    #[test]
    fn horizontal_threshold_is_inclusive_at_tangency() {
        let a = trajectory(
            "A",
            vec![
                point(0, 0.0, 0.0, 100.0),
                point(10_000, 1_000.0, 0.0, 100.0),
            ],
            0.0,
            0.0,
        );
        let b = trajectory(
            "B",
            vec![
                point(0, 0.0, 100.0, 100.0),
                point(10_000, 1_000.0, 100.0, 100.0),
            ],
            0.0,
            0.0,
        );
        let report = audit_trajectory_pair(&a, &b, &criteria()).expect("valid fixture");
        assert_eq!(report.disposition, ConflictDisposition::ConflictDetected);
        assert_eq!(report.conflicts[0].horizontal_separation_m, 100.0);
    }

    #[test]
    fn uncertainty_expands_conservative_centerline_bound() {
        let a = trajectory(
            "A",
            vec![
                point(0, 0.0, 0.0, 100.0),
                point(10_000, 1_000.0, 0.0, 100.0),
            ],
            10.1,
            0.0,
        );
        let b = trajectory(
            "B",
            vec![
                point(0, 0.0, 120.0, 100.0),
                point(10_000, 1_000.0, 120.0, 100.0),
            ],
            10.1,
            0.0,
        );
        let report = audit_trajectory_pair(&a, &b, &criteria()).expect("valid fixture");
        assert_eq!(report.disposition, ConflictDisposition::ConflictDetected);
        assert!((report.effective_horizontal_bound_m - 120.2).abs() < 1e-9);
    }

    #[test]
    fn vertical_boundary_is_inclusive() {
        let a = trajectory(
            "A",
            vec![
                point(0, 0.0, 0.0, 100.0),
                point(10_000, 1_000.0, 0.0, 100.0),
            ],
            0.0,
            0.0,
        );
        let b = trajectory(
            "B",
            vec![
                point(0, 0.0, 0.0, 150.0),
                point(10_000, 1_000.0, 0.0, 150.0),
            ],
            0.0,
            0.0,
        );
        let report = audit_trajectory_pair(&a, &b, &criteria()).expect("valid fixture");
        assert_eq!(report.disposition, ConflictDisposition::ConflictDetected);
        assert_eq!(report.conflicts[0].vertical_separation_m, 50.0);
    }

    #[test]
    fn trajectory_windows_without_time_overlap_report_no_temporal_overlap() {
        let a = trajectory(
            "A",
            vec![
                point(0, 0.0, 0.0, 100.0),
                point(10_000, 1_000.0, 0.0, 100.0),
            ],
            0.0,
            0.0,
        );
        let b = trajectory(
            "B",
            vec![
                point(20_000, 500.0, -500.0, 100.0),
                point(30_000, 500.0, 500.0, 100.0),
            ],
            0.0,
            0.0,
        );
        let report = audit_trajectory_pair(&a, &b, &criteria()).expect("valid fixture");
        assert_eq!(report.disposition, ConflictDisposition::NoTemporalOverlap);
        assert_eq!(report.temporal_overlap_utc_ms, None);
    }

    #[test]
    fn criteria_with_no_coverage_is_not_assessable() {
        let (a, b) = crossing_pair();
        let mut rule = criteria();
        rule.valid_from_utc_ms = 20_000;
        rule.valid_until_utc_ms = 30_000;
        let report = audit_trajectory_pair(&a, &b, &rule).expect("structurally valid fixture");
        assert_eq!(report.disposition, ConflictDisposition::NotAssessable);
        assert!(!report.criteria_covers_declared_overlap);
        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn conflict_in_covered_part_takes_precedence_over_incomplete_criteria_window() {
        let (a, b) = crossing_pair();
        let mut rule = criteria();
        rule.valid_from_utc_ms = 0;
        rule.valid_until_utc_ms = 5_000;
        let report = audit_trajectory_pair(&a, &b, &rule).expect("structurally valid fixture");
        assert_eq!(report.disposition, ConflictDisposition::ConflictDetected);
        assert!(!report.criteria_covers_declared_overlap);
    }

    #[test]
    fn partial_criteria_without_detected_conflict_remains_not_assessable() {
        let a = trajectory(
            "A",
            vec![
                point(0, 0.0, 0.0, 100.0),
                point(10_000, 1_000.0, 0.0, 100.0),
            ],
            0.0,
            0.0,
        );
        let b = trajectory(
            "B",
            vec![
                point(0, 0.0, 500.0, 100.0),
                point(10_000, 1_000.0, 500.0, 100.0),
            ],
            0.0,
            0.0,
        );
        let mut rule = criteria();
        rule.valid_until_utc_ms = 5_000;
        let report = audit_trajectory_pair(&a, &b, &rule).expect("structurally valid fixture");
        assert_eq!(report.disposition, ConflictDisposition::NotAssessable);
        assert!(!report.criteria_covers_declared_overlap);
    }

    #[test]
    fn reference_frame_mismatch_is_rejected() {
        let (a, mut b) = crossing_pair();
        b.reference_frame_id = "OTHER-FRAME".to_owned();
        assert_eq!(
            audit_trajectory_pair(&a, &b, &criteria()),
            Err(AuditInputError::ReferenceFrameMismatch)
        );
    }

    #[test]
    fn vertical_datum_mismatch_is_rejected() {
        let (a, mut b) = crossing_pair();
        b.vertical_datum_id = "OTHER-DATUM".to_owned();
        assert_eq!(
            audit_trajectory_pair(&a, &b, &criteria()),
            Err(AuditInputError::VerticalDatumMismatch)
        );
    }

    #[test]
    fn duplicate_or_decreasing_timestamps_are_rejected() {
        let (mut a, b) = crossing_pair();
        a.points[1].time_utc_ms = a.points[0].time_utc_ms;
        assert!(matches!(
            audit_trajectory_pair(&a, &b, &criteria()),
            Err(AuditInputError::InvalidTrajectory(_))
        ));
    }

    #[test]
    fn malformed_source_digest_is_rejected() {
        let (a, b) = crossing_pair();
        let mut rule = criteria();
        rule.source_sha256 = "not-a-digest".to_owned();
        assert!(matches!(
            audit_trajectory_pair(&a, &b, &rule),
            Err(AuditInputError::InvalidCriteria(_))
        ));
    }

    #[test]
    fn non_finite_coordinates_are_rejected() {
        let (mut a, b) = crossing_pair();
        a.points[0].x_m = f64::NAN;
        assert!(matches!(
            audit_trajectory_pair(&a, &b, &criteria()),
            Err(AuditInputError::InvalidTrajectory(_))
        ));
    }

    #[test]
    fn overflow_in_uncertainty_expansion_is_rejected() {
        let mut a = crossing_pair().0;
        let b = crossing_pair().1;
        a.horizontal_uncertainty_bound_m = f64::MAX;
        let mut other = b;
        other.horizontal_uncertainty_bound_m = f64::MAX;
        assert_eq!(
            audit_trajectory_pair(&a, &other, &criteria()),
            Err(AuditInputError::NumericOverflow)
        );
    }

    #[test]
    fn exact_single_instant_overlap_is_checked() {
        let a = trajectory(
            "A",
            vec![point(0, 0.0, 0.0, 100.0), point(10_000, 10.0, 0.0, 100.0)],
            0.0,
            0.0,
        );
        let b = trajectory(
            "B",
            vec![
                point(10_000, 10.0, 0.0, 100.0),
                point(20_000, 20.0, 0.0, 100.0),
            ],
            0.0,
            0.0,
        );
        let report = audit_trajectory_pair(&a, &b, &criteria()).expect("valid fixture");
        assert_eq!(report.disposition, ConflictDisposition::ConflictDetected);
        assert_eq!(report.conflicts[0].closest_approach_utc_ms, 10_000);
    }

    #[test]
    fn argument_order_does_not_change_the_report() {
        let (a, b) = crossing_pair();
        let forward = audit_trajectory_pair(&a, &b, &criteria()).expect("valid forward order");
        let reverse = audit_trajectory_pair(&b, &a, &criteria()).expect("valid reverse order");
        assert_eq!(forward, reverse);
    }

    #[test]
    fn same_trajectory_identity_is_rejected() {
        let (a, _) = crossing_pair();
        let mut b = a.clone();
        b.points[0].x_m += 1.0;
        assert_eq!(
            audit_trajectory_pair(&a, &b, &criteria()),
            Err(AuditInputError::SameTrajectoryId)
        );
    }
}
