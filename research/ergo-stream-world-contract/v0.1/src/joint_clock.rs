//! Joint inverse-affine clock and latent-event-time reconstruction.
//!
//! This module is a separately named MATH-SYNC-style reference formulation,
//! not the paired-anchor regression in `clock_estimators`. It requires known
//! synchronization-event correspondences and jointly estimates:
//!
//! - one latent common time per event;
//! - one inverse affine clock rate and offset per non-reference stream.
//!
//! For each observed stream/event pair, the residual is affine after fixing a
//! reference-stream temporal gauge. Rank deficiency is reported as
//! non-identifiability; no regularization silently manufactures a unique map.
//! Observation arrival order and transport delay are intentionally absent.

use std::collections::BTreeSet;
use std::fmt;

pub const MAX_JOINT_CLOCK_STREAMS_V0: usize = 8;
pub const MAX_JOINT_CLOCK_EVENTS_V0: usize = 128;
pub const MAX_JOINT_CLOCK_OBSERVATIONS_V0: usize = 2_048;
pub const JOINT_CLOCK_RANK_TOLERANCE_V0: f64 = 1.0e-12;
pub const JOINT_CLOCK_HUBER_C_V0: f64 = 1.345;
pub const JOINT_CLOCK_MAD_FLOOR_V0: f64 = 1.0e-6;
pub const JOINT_CLOCK_HUBER_MAX_ITERATIONS_V0: usize = 30;
pub const JOINT_CLOCK_HUBER_RELATIVE_TOLERANCE_V0: f64 = 1.0e-10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SensorEventObservationV0 {
    pub stream_id: u16,
    pub event_id: u32,
    /// Measurement timestamp in this stream's local sensor clock.
    pub local_timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointClockProblemV0 {
    pub stream_ids: Vec<u16>,
    pub event_ids: Vec<u32>,
    pub reference_stream_id: u16,
    pub observations: Vec<SensorEventObservationV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointClockParameterV0 {
    EventTime { event_id: u32 },
    InverseRate { stream_id: u16 },
    InverseOffset { stream_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointClockErrorV0 {
    EmptyStreams,
    EmptyEvents,
    EmptyObservations,
    TooManyStreams { found: usize, maximum: usize },
    TooManyEvents { found: usize, maximum: usize },
    TooManyObservations { found: usize, maximum: usize },
    DuplicateStreamId { stream_id: u16 },
    DuplicateEventId { event_id: u32 },
    UnknownReferenceStream { stream_id: u16 },
    UnknownObservationStream { stream_id: u16 },
    UnknownObservationEvent { event_id: u32 },
    DuplicateStreamEventObservation { stream_id: u16, event_id: u32 },
    InvalidRankTolerance,
    RankDeficient { rank: usize, parameters: usize },
    NonFiniteInput,
    NonFiniteEstimate,
    NonPositiveInverseRate { stream_id: u16 },
    InvalidRobustConfiguration,
    NumericalBreakdown,
}

impl fmt::Display for JointClockErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyStreams => formatter.write_str("at least one sensor stream is required"),
            Self::EmptyEvents => {
                formatter.write_str("at least one synchronization event is required")
            }
            Self::EmptyObservations => {
                formatter.write_str("at least one sensor/event timestamp is required")
            }
            Self::TooManyStreams { found, maximum } => {
                write!(formatter, "stream count {found} exceeds v0 limit {maximum}")
            }
            Self::TooManyEvents { found, maximum } => {
                write!(formatter, "event count {found} exceeds v0 limit {maximum}")
            }
            Self::TooManyObservations { found, maximum } => {
                write!(
                    formatter,
                    "observation count {found} exceeds v0 limit {maximum}"
                )
            }
            Self::DuplicateStreamId { stream_id } => {
                write!(formatter, "duplicate stream ID {stream_id}")
            }
            Self::DuplicateEventId { event_id } => {
                write!(formatter, "duplicate event ID {event_id}")
            }
            Self::UnknownReferenceStream { stream_id } => {
                write!(formatter, "reference stream {stream_id} is not declared")
            }
            Self::UnknownObservationStream { stream_id } => {
                write!(
                    formatter,
                    "observation references undeclared stream {stream_id}"
                )
            }
            Self::UnknownObservationEvent { event_id } => {
                write!(
                    formatter,
                    "observation references undeclared event {event_id}"
                )
            }
            Self::DuplicateStreamEventObservation {
                stream_id,
                event_id,
            } => {
                write!(
                    formatter,
                    "duplicate observation for stream {stream_id}, event {event_id}"
                )
            }
            Self::InvalidRankTolerance => {
                formatter.write_str("rank tolerance must be finite and strictly positive")
            }
            Self::RankDeficient { rank, parameters } => {
                write!(
                    formatter,
                    "joint clock design is not identifiable: rank {rank} for {parameters} parameters"
                )
            }
            Self::NonFiniteInput => {
                formatter.write_str("joint clock design contains a non-finite value")
            }
            Self::NonFiniteEstimate => {
                formatter.write_str("joint clock fit produced a non-finite estimate")
            }
            Self::NonPositiveInverseRate { stream_id } => {
                write!(
                    formatter,
                    "estimated inverse clock rate for stream {stream_id} is not positive"
                )
            }
            Self::InvalidRobustConfiguration => {
                formatter.write_str("Huber configuration is outside the fixed v0 protocol")
            }
            Self::NumericalBreakdown => {
                formatter.write_str("rank-revealing QR encountered a numerical breakdown")
            }
        }
    }
}

impl std::error::Error for JointClockErrorV0 {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InverseClockEstimateV0 {
    pub stream_id: u16,
    /// Common time = inverse_rate * local_time + inverse_offset.
    pub inverse_rate: f64,
    pub inverse_offset: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventTimeEstimateV0 {
    pub event_id: u32,
    pub common_time: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointClockDesignReportV0 {
    pub reference_stream_id: u16,
    pub observation_count: usize,
    pub parameter_count: usize,
    pub rank: usize,
    pub full_column_rank: bool,
    /// Largest/smallest accepted pivot magnitude of the column-scaled QR.
    /// This is a diagnostic proxy, not a condition number from an SVD.
    pub pivot_ratio: Option<f64>,
    pub rank_tolerance: f64,
    pub parameter_order: Vec<JointClockParameterV0>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointClockFitV0 {
    pub reference_stream_id: u16,
    pub event_times: Vec<EventTimeEstimateV0>,
    pub inverse_clocks: Vec<InverseClockEstimateV0>,
    pub design: JointClockDesignReportV0,
    pub residual_rms: f64,
    pub median_absolute_residual: f64,
    pub max_absolute_residual: f64,
    pub robust_scale: Option<f64>,
    pub iterations: usize,
    pub converged: bool,
}

#[derive(Debug, Clone)]
struct JointDesignV0 {
    stream_ids: Vec<u16>,
    event_ids: Vec<u32>,
    reference_stream_id: u16,
    matrix: Vec<Vec<f64>>,
    target: Vec<f64>,
    parameter_order: Vec<JointClockParameterV0>,
}

#[derive(Debug, Clone)]
struct QrSolveV0 {
    coefficients: Vec<f64>,
    rank: usize,
    pivot_ratio: Option<f64>,
}

fn build_joint_design_v0(
    problem: &JointClockProblemV0,
) -> Result<JointDesignV0, JointClockErrorV0> {
    if problem.stream_ids.is_empty() {
        return Err(JointClockErrorV0::EmptyStreams);
    }
    if problem.event_ids.is_empty() {
        return Err(JointClockErrorV0::EmptyEvents);
    }
    if problem.observations.is_empty() {
        return Err(JointClockErrorV0::EmptyObservations);
    }
    if problem.stream_ids.len() > MAX_JOINT_CLOCK_STREAMS_V0 {
        return Err(JointClockErrorV0::TooManyStreams {
            found: problem.stream_ids.len(),
            maximum: MAX_JOINT_CLOCK_STREAMS_V0,
        });
    }
    if problem.event_ids.len() > MAX_JOINT_CLOCK_EVENTS_V0 {
        return Err(JointClockErrorV0::TooManyEvents {
            found: problem.event_ids.len(),
            maximum: MAX_JOINT_CLOCK_EVENTS_V0,
        });
    }
    if problem.observations.len() > MAX_JOINT_CLOCK_OBSERVATIONS_V0 {
        return Err(JointClockErrorV0::TooManyObservations {
            found: problem.observations.len(),
            maximum: MAX_JOINT_CLOCK_OBSERVATIONS_V0,
        });
    }

    let mut stream_ids = problem.stream_ids.clone();
    stream_ids.sort_unstable();
    if stream_ids.windows(2).any(|window| window[0] == window[1]) {
        let stream_id = stream_ids
            .windows(2)
            .find(|window| window[0] == window[1])
            .map(|window| window[0])
            .unwrap_or_default();
        return Err(JointClockErrorV0::DuplicateStreamId { stream_id });
    }
    if !stream_ids.contains(&problem.reference_stream_id) {
        return Err(JointClockErrorV0::UnknownReferenceStream {
            stream_id: problem.reference_stream_id,
        });
    }

    let mut event_ids = problem.event_ids.clone();
    event_ids.sort_unstable();
    if event_ids.windows(2).any(|window| window[0] == window[1]) {
        let event_id = event_ids
            .windows(2)
            .find(|window| window[0] == window[1])
            .map(|window| window[0])
            .unwrap_or_default();
        return Err(JointClockErrorV0::DuplicateEventId { event_id });
    }

    let non_reference_streams = stream_ids
        .iter()
        .copied()
        .filter(|stream_id| *stream_id != problem.reference_stream_id)
        .collect::<Vec<_>>();
    let mut parameter_order = event_ids
        .iter()
        .map(|event_id| JointClockParameterV0::EventTime {
            event_id: *event_id,
        })
        .collect::<Vec<_>>();
    let mut parameter_columns = std::collections::BTreeMap::new();
    for (index, event_id) in event_ids.iter().enumerate() {
        parameter_columns.insert((*event_id, None), index);
    }

    let event_column_count = event_ids.len();
    for (index, stream_id) in non_reference_streams.iter().enumerate() {
        let inverse_rate_column = event_column_count + 2 * index;
        let inverse_offset_column = inverse_rate_column + 1;
        parameter_order.push(JointClockParameterV0::InverseRate {
            stream_id: *stream_id,
        });
        parameter_order.push(JointClockParameterV0::InverseOffset {
            stream_id: *stream_id,
        });
        parameter_columns.insert((0, Some(*stream_id)), inverse_rate_column);
        parameter_columns.insert((1, Some(*stream_id)), inverse_offset_column);
    }

    let parameter_count = event_column_count + 2 * non_reference_streams.len();
    let mut observations = problem.observations.clone();
    observations.sort_unstable();
    let mut seen_pairs = BTreeSet::new();
    let mut matrix = Vec::with_capacity(observations.len());
    let mut target = Vec::with_capacity(observations.len());

    for observation in &observations {
        if !stream_ids.contains(&observation.stream_id) {
            return Err(JointClockErrorV0::UnknownObservationStream {
                stream_id: observation.stream_id,
            });
        }
        if !event_ids.contains(&observation.event_id) {
            return Err(JointClockErrorV0::UnknownObservationEvent {
                event_id: observation.event_id,
            });
        }
        if !seen_pairs.insert((observation.stream_id, observation.event_id)) {
            return Err(JointClockErrorV0::DuplicateStreamEventObservation {
                stream_id: observation.stream_id,
                event_id: observation.event_id,
            });
        }

        let mut row = vec![0.0_f64; parameter_count];
        let event_column = parameter_columns[&(observation.event_id, None)];
        row[event_column] = 1.0;

        if observation.stream_id == problem.reference_stream_id {
            target.push(observation.local_timestamp as f64);
        } else {
            let inverse_rate_column = parameter_columns[&(0, Some(observation.stream_id))];
            let inverse_offset_column = parameter_columns[&(1, Some(observation.stream_id))];
            row[inverse_rate_column] = -(observation.local_timestamp as f64);
            row[inverse_offset_column] = -1.0;
            target.push(0.0);
        }
        matrix.push(row);
    }

    Ok(JointDesignV0 {
        stream_ids,
        event_ids,
        reference_stream_id: problem.reference_stream_id,
        matrix,
        target,
        parameter_order,
    })
}

fn weighted_rank_revealing_qr_v0(
    matrix: &[Vec<f64>],
    target: &[f64],
    weights: &[f64],
    rank_tolerance: f64,
) -> Result<QrSolveV0, JointClockErrorV0> {
    if !rank_tolerance.is_finite() || rank_tolerance <= 0.0 {
        return Err(JointClockErrorV0::InvalidRankTolerance);
    }
    let row_count = matrix.len();
    let column_count = matrix.first().map_or(0, Vec::len);
    if row_count == 0
        || column_count == 0
        || target.len() != row_count
        || weights.len() != row_count
    {
        return Err(JointClockErrorV0::EmptyObservations);
    }
    if matrix.iter().any(|row| row.len() != column_count) {
        return Err(JointClockErrorV0::NonFiniteInput);
    }

    let mut a = vec![vec![0.0_f64; column_count]; row_count];
    let mut b = vec![0.0_f64; row_count];
    for row_index in 0..row_count {
        let weight = weights[row_index];
        if !weight.is_finite() || weight <= 0.0 || !target[row_index].is_finite() {
            return Err(JointClockErrorV0::NonFiniteInput);
        }
        let root_weight = weight.sqrt();
        b[row_index] = target[row_index] * root_weight;
        for column_index in 0..column_count {
            let value = matrix[row_index][column_index];
            if !value.is_finite() {
                return Err(JointClockErrorV0::NonFiniteInput);
            }
            a[row_index][column_index] = value * root_weight;
        }
    }

    // Normalize columns before pivoted Householder QR. This preserves the
    // least-squares solution while reducing scale disparities between latent
    // times, inverse rates, and offsets.
    let mut column_scales = vec![1.0_f64; column_count];
    for column_index in 0..column_count {
        let norm = (0..row_count)
            .map(|row_index| a[row_index][column_index] * a[row_index][column_index])
            .sum::<f64>()
            .sqrt();
        if norm.is_finite() && norm > 0.0 {
            column_scales[column_index] = norm;
            for row in &mut a {
                row[column_index] /= norm;
            }
        }
    }

    let mut permutation = (0..column_count).collect::<Vec<_>>();
    let limit = row_count.min(column_count);
    let threshold = rank_tolerance * (row_count.max(column_count) as f64);
    let mut rank = 0_usize;
    let mut first_pivot: Option<f64> = None;
    let mut last_pivot: Option<f64> = None;

    for pivot_index in 0..limit {
        let mut best_column = pivot_index;
        let mut best_norm_squared = -1.0_f64;
        for (column_index, _) in a[0].iter().enumerate().skip(pivot_index) {
            let norm_squared = (pivot_index..row_count)
                .map(|row_index| a[row_index][column_index] * a[row_index][column_index])
                .sum::<f64>();
            if norm_squared > best_norm_squared {
                best_norm_squared = norm_squared;
                best_column = column_index;
            }
        }
        let best_norm = best_norm_squared.max(0.0).sqrt();
        if !best_norm.is_finite() {
            return Err(JointClockErrorV0::NumericalBreakdown);
        }
        if best_norm <= threshold {
            break;
        }

        if best_column != pivot_index {
            for row in &mut a {
                row.swap(pivot_index, best_column);
            }
            permutation.swap(pivot_index, best_column);
            column_scales.swap(pivot_index, best_column);
        }

        let x_norm = (pivot_index..row_count)
            .map(|row_index| a[row_index][pivot_index] * a[row_index][pivot_index])
            .sum::<f64>()
            .sqrt();
        let x_first = a[pivot_index][pivot_index];
        let alpha = if x_first >= 0.0 { -x_norm } else { x_norm };

        let mut householder = (pivot_index..row_count)
            .map(|row_index| a[row_index][pivot_index])
            .collect::<Vec<_>>();
        householder[0] -= alpha;
        let vector_norm_squared = householder.iter().map(|value| value * value).sum::<f64>();
        if !vector_norm_squared.is_finite() || vector_norm_squared <= f64::MIN_POSITIVE {
            return Err(JointClockErrorV0::NumericalBreakdown);
        }
        let factor = 2.0 / vector_norm_squared;

        for (column_index, _) in a[0].iter().enumerate().skip(pivot_index) {
            let dot = householder
                .iter()
                .enumerate()
                .map(|(offset, value)| value * a[pivot_index + offset][column_index])
                .sum::<f64>();
            let multiplier = factor * dot;
            for (offset, value) in householder.iter().enumerate() {
                a[pivot_index + offset][column_index] -= multiplier * value;
            }
        }
        let dot_b = householder
            .iter()
            .enumerate()
            .map(|(offset, value)| value * b[pivot_index + offset])
            .sum::<f64>();
        let multiplier_b = factor * dot_b;
        for (offset, value) in householder.iter().enumerate() {
            b[pivot_index + offset] -= multiplier_b * value;
        }

        a[pivot_index][pivot_index] = alpha;
        for row in a.iter_mut().skip(pivot_index + 1) {
            row[pivot_index] = 0.0;
        }

        let diagonal = alpha.abs();
        first_pivot.get_or_insert(diagonal);
        last_pivot = Some(diagonal);
        rank += 1;
    }

    if rank < column_count {
        return Ok(QrSolveV0 {
            coefficients: Vec::new(),
            rank,
            pivot_ratio: None,
        });
    }

    let mut pivoted_solution = vec![0.0_f64; column_count];
    for row_index in (0..column_count).rev() {
        let tail = ((row_index + 1)..column_count)
            .map(|column_index| a[row_index][column_index] * pivoted_solution[column_index])
            .sum::<f64>();
        let diagonal = a[row_index][row_index];
        if !diagonal.is_finite() || diagonal.abs() <= f64::MIN_POSITIVE {
            return Err(JointClockErrorV0::NumericalBreakdown);
        }
        pivoted_solution[row_index] = (b[row_index] - tail) / diagonal;
    }

    let mut coefficients = vec![0.0_f64; column_count];
    for pivoted_column in 0..column_count {
        let original_column = permutation[pivoted_column];
        coefficients[original_column] =
            pivoted_solution[pivoted_column] / column_scales[pivoted_column];
    }
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Err(JointClockErrorV0::NonFiniteEstimate);
    }

    let pivot_ratio = match (first_pivot, last_pivot) {
        (Some(largest), Some(smallest)) if smallest > 0.0 => Some(largest / smallest),
        _ => None,
    };

    Ok(QrSolveV0 {
        coefficients,
        rank,
        pivot_ratio,
    })
}

fn residuals_v0(matrix: &[Vec<f64>], target: &[f64], coefficients: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .zip(target)
        .map(|(row, expected)| {
            row.iter()
                .zip(coefficients)
                .map(|(value, coefficient)| value * coefficient)
                .sum::<f64>()
                - expected
        })
        .collect()
}

fn median_v0(values: &[f64]) -> Option<f64> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let midpoint = ordered.len() / 2;
    if ordered.len().is_multiple_of(2) {
        Some((ordered[midpoint - 1] + ordered[midpoint]) * 0.5)
    } else {
        Some(ordered[midpoint])
    }
}

fn make_design_report_v0(
    design: &JointDesignV0,
    rank_tolerance: f64,
    rank: usize,
    pivot_ratio: Option<f64>,
) -> JointClockDesignReportV0 {
    JointClockDesignReportV0 {
        reference_stream_id: design.reference_stream_id,
        observation_count: design.matrix.len(),
        parameter_count: design.parameter_order.len(),
        rank,
        full_column_rank: rank == design.parameter_order.len(),
        pivot_ratio,
        rank_tolerance,
        parameter_order: design.parameter_order.clone(),
    }
}

fn finish_fit_v0(
    design: &JointDesignV0,
    coefficients: Vec<f64>,
    iterations: usize,
    converged: bool,
    robust_scale: Option<f64>,
    design_report: JointClockDesignReportV0,
) -> Result<JointClockFitV0, JointClockErrorV0> {
    let residuals = residuals_v0(&design.matrix, &design.target, &coefficients);
    let absolute = residuals
        .iter()
        .map(|residual| residual.abs())
        .collect::<Vec<_>>();
    let residual_rms = (residuals
        .iter()
        .map(|residual| residual * residual)
        .sum::<f64>()
        / residuals.len() as f64)
        .sqrt();
    let median_absolute_residual =
        median_v0(&absolute).ok_or(JointClockErrorV0::NonFiniteEstimate)?;
    let max_absolute_residual = absolute.iter().copied().fold(0.0_f64, f64::max);

    let mut event_times = Vec::with_capacity(design.event_ids.len());
    for (index, event_id) in design.event_ids.iter().enumerate() {
        event_times.push(EventTimeEstimateV0 {
            event_id: *event_id,
            common_time: coefficients[index],
        });
    }

    let mut inverse_clocks = Vec::with_capacity(design.stream_ids.len());
    for stream_id in &design.stream_ids {
        if *stream_id == design.reference_stream_id {
            inverse_clocks.push(InverseClockEstimateV0 {
                stream_id: *stream_id,
                inverse_rate: 1.0,
                inverse_offset: 0.0,
            });
        } else {
            let stream_index = design
                .stream_ids
                .iter()
                .filter(|candidate| **candidate < *stream_id)
                .filter(|candidate| **candidate != design.reference_stream_id)
                .count();
            let gamma_column = design.event_ids.len() + 2 * stream_index;
            let kappa_column = gamma_column + 1;
            let inverse_rate = coefficients[gamma_column];
            if inverse_rate <= 0.0 {
                return Err(JointClockErrorV0::NonPositiveInverseRate {
                    stream_id: *stream_id,
                });
            }
            inverse_clocks.push(InverseClockEstimateV0 {
                stream_id: *stream_id,
                inverse_rate,
                inverse_offset: coefficients[kappa_column],
            });
        }
    }

    if !residual_rms.is_finite()
        || !median_absolute_residual.is_finite()
        || !max_absolute_residual.is_finite()
    {
        return Err(JointClockErrorV0::NonFiniteEstimate);
    }

    Ok(JointClockFitV0 {
        reference_stream_id: design.reference_stream_id,
        event_times,
        inverse_clocks,
        design: design_report,
        residual_rms,
        median_absolute_residual,
        max_absolute_residual,
        robust_scale,
        iterations,
        converged,
    })
}

/// Compute exact-form structural identifiability diagnostics using a
/// column-scaled, rank-revealing Householder QR. The reported rank is numeric,
/// using the explicit tolerance in the caller-visible receipt.
pub fn analyze_joint_clock_design_v0(
    problem: &JointClockProblemV0,
    rank_tolerance: f64,
) -> Result<JointClockDesignReportV0, JointClockErrorV0> {
    let design = build_joint_design_v0(problem)?;
    let weights = vec![1.0_f64; design.matrix.len()];
    let solved =
        weighted_rank_revealing_qr_v0(&design.matrix, &design.target, &weights, rank_tolerance)?;
    Ok(make_design_report_v0(
        &design,
        rank_tolerance,
        solved.rank,
        solved.pivot_ratio,
    ))
}

/// Joint OLS reconstruction of latent event times and inverse affine clocks.
pub fn fit_joint_clock_ols_v0(
    problem: &JointClockProblemV0,
) -> Result<JointClockFitV0, JointClockErrorV0> {
    let design = build_joint_design_v0(problem)?;
    let weights = vec![1.0_f64; design.matrix.len()];
    let solved = weighted_rank_revealing_qr_v0(
        &design.matrix,
        &design.target,
        &weights,
        JOINT_CLOCK_RANK_TOLERANCE_V0,
    )?;
    let report = make_design_report_v0(
        &design,
        JOINT_CLOCK_RANK_TOLERANCE_V0,
        solved.rank,
        solved.pivot_ratio,
    );
    if solved.rank < design.parameter_order.len() {
        return Err(JointClockErrorV0::RankDeficient {
            rank: solved.rank,
            parameters: design.parameter_order.len(),
        });
    }
    finish_fit_v0(&design, solved.coefficients, 0, true, None, report)
}

/// Joint Huber IRLS using the MATH-SYNC v0 constants: c=1.345, MAD scale
/// floor 1e-6, at most 30 iterations, and relative parameter-update tolerance
/// 1e-10. This reproduces the estimator shape; a fidelity claim still requires
/// differential comparison against the pinned upstream release and data.
pub fn fit_joint_clock_huber_v0(
    problem: &JointClockProblemV0,
) -> Result<JointClockFitV0, JointClockErrorV0> {
    let design = build_joint_design_v0(problem)?;
    let row_count = design.matrix.len();
    let unit_weights = vec![1.0_f64; row_count];
    let initial = weighted_rank_revealing_qr_v0(
        &design.matrix,
        &design.target,
        &unit_weights,
        JOINT_CLOCK_RANK_TOLERANCE_V0,
    )?;
    if initial.rank < design.parameter_order.len() {
        return Err(JointClockErrorV0::RankDeficient {
            rank: initial.rank,
            parameters: design.parameter_order.len(),
        });
    }

    let mut coefficients = initial.coefficients;
    let mut final_rank = initial.rank;
    let mut final_pivot_ratio = initial.pivot_ratio;
    let mut robust_scale = JOINT_CLOCK_MAD_FLOOR_V0;
    let mut converged = false;
    let mut iterations = 0_usize;

    for iteration in 1..=JOINT_CLOCK_HUBER_MAX_ITERATIONS_V0 {
        let residuals = residuals_v0(&design.matrix, &design.target, &coefficients);
        let residual_median = median_v0(&residuals).ok_or(JointClockErrorV0::NonFiniteEstimate)?;
        let deviations = residuals
            .iter()
            .map(|residual| (residual - residual_median).abs())
            .collect::<Vec<_>>();
        let mad = median_v0(&deviations).ok_or(JointClockErrorV0::NonFiniteEstimate)?;
        robust_scale = (1.4826 * mad).max(JOINT_CLOCK_MAD_FLOOR_V0);
        let cutoff = JOINT_CLOCK_HUBER_C_V0 * robust_scale;
        let weights = residuals
            .iter()
            .map(|residual| {
                let standardized = residual.abs();
                if standardized <= cutoff {
                    1.0
                } else {
                    cutoff / standardized
                }
            })
            .collect::<Vec<_>>();

        let next = weighted_rank_revealing_qr_v0(
            &design.matrix,
            &design.target,
            &weights,
            JOINT_CLOCK_RANK_TOLERANCE_V0,
        )?;
        if next.rank < design.parameter_order.len() {
            return Err(JointClockErrorV0::RankDeficient {
                rank: next.rank,
                parameters: design.parameter_order.len(),
            });
        }

        let delta_norm = next
            .coefficients
            .iter()
            .zip(&coefficients)
            .map(|(next_value, old_value)| (next_value - old_value) * (next_value - old_value))
            .sum::<f64>()
            .sqrt();
        let old_norm = coefficients
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        let relative_update = delta_norm / old_norm.max(1.0);
        coefficients = next.coefficients;
        final_rank = next.rank;
        final_pivot_ratio = next.pivot_ratio;
        iterations = iteration;

        if relative_update <= JOINT_CLOCK_HUBER_RELATIVE_TOLERANCE_V0 {
            converged = true;
            break;
        }
    }

    let report = make_design_report_v0(
        &design,
        JOINT_CLOCK_RANK_TOLERANCE_V0,
        final_rank,
        final_pivot_ratio,
    );
    finish_fit_v0(
        &design,
        coefficients,
        iterations,
        converged,
        Some(robust_scale),
        report,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_stream_problem(event_count: usize) -> JointClockProblemV0 {
        let event_ids = (0..event_count)
            .map(|index| 100 + u32::try_from(index).unwrap_or(u32::MAX))
            .collect::<Vec<_>>();
        let mut observations = Vec::with_capacity(event_count * 2);
        for (index, event_id) in event_ids.iter().enumerate() {
            let tau = 100_i64 + i64::try_from(index).unwrap_or(i64::MAX) * 100;
            observations.push(SensorEventObservationV0 {
                stream_id: 0,
                event_id: *event_id,
                local_timestamp: tau,
            });
            observations.push(SensorEventObservationV0 {
                stream_id: 1,
                event_id: *event_id,
                local_timestamp: 2 * tau + 100,
            });
        }
        JointClockProblemV0 {
            stream_ids: vec![0, 1],
            event_ids,
            reference_stream_id: 0,
            observations,
        }
    }

    #[test]
    fn joint_ols_recovers_event_times_and_inverse_clock() {
        let problem = two_stream_problem(4);
        let fit = fit_joint_clock_ols_v0(&problem).unwrap();
        assert_eq!(fit.design.rank, fit.design.parameter_count);
        assert_eq!(fit.reference_stream_id, 0);
        assert!(fit.residual_rms < 1.0e-8);
        assert_eq!(fit.event_times.len(), 4);
        for (index, event) in fit.event_times.iter().enumerate() {
            let expected = 100.0 + index as f64 * 100.0;
            assert!((event.common_time - expected).abs() < 1.0e-8);
        }
        let non_reference = fit
            .inverse_clocks
            .iter()
            .find(|clock| clock.stream_id == 1)
            .unwrap();
        assert!((non_reference.inverse_rate - 0.5).abs() < 1.0e-10);
        assert!((non_reference.inverse_offset + 50.0).abs() < 1.0e-8);
    }

    #[test]
    fn joint_huber_reduces_latent_time_error_under_a_gross_timestamp_outlier() {
        let mut problem = two_stream_problem(12);
        let corrupted_event = problem.event_ids[11];
        let outlier = problem
            .observations
            .iter_mut()
            .find(|observation| {
                observation.stream_id == 1 && observation.event_id == corrupted_event
            })
            .unwrap();
        outlier.local_timestamp += 500;
        let ols = fit_joint_clock_ols_v0(&problem).unwrap();
        let huber = fit_joint_clock_huber_v0(&problem).unwrap();

        let true_times = (0..12)
            .map(|index| 100.0 + index as f64 * 100.0)
            .collect::<Vec<_>>();
        let ols_error = ols
            .event_times
            .iter()
            .zip(&true_times)
            .map(|(estimate, truth)| (estimate.common_time - truth).abs())
            .sum::<f64>()
            / true_times.len() as f64;
        let huber_error = huber
            .event_times
            .iter()
            .zip(&true_times)
            .map(|(estimate, truth)| (estimate.common_time - truth).abs())
            .sum::<f64>()
            / true_times.len() as f64;
        assert!(huber_error < ols_error);
        assert!(huber.iterations <= JOINT_CLOCK_HUBER_MAX_ITERATIONS_V0);
    }

    #[test]
    fn rank_condition_rejects_one_shared_event_for_two_stream_affine_fit() {
        let problem = two_stream_problem(1);
        let report =
            analyze_joint_clock_design_v0(&problem, JOINT_CLOCK_RANK_TOLERANCE_V0).unwrap();
        assert_eq!(report.parameter_count, 3);
        assert_eq!(report.rank, 2);
        assert!(!report.full_column_rank);
        assert_eq!(
            fit_joint_clock_ols_v0(&problem),
            Err(JointClockErrorV0::RankDeficient {
                rank: 2,
                parameters: 3,
            })
        );
    }

    #[test]
    fn missing_event_observations_are_sparse_rows_not_interpolated() {
        let mut problem = two_stream_problem(5);
        let missing_event = problem.event_ids[2];
        problem.observations.retain(|observation| {
            !(observation.stream_id == 1 && observation.event_id == missing_event)
        });
        let fit = fit_joint_clock_ols_v0(&problem).unwrap();
        assert_eq!(fit.design.observation_count, 9);
        assert_eq!(fit.event_times.len(), 5);
        assert!(fit.residual_rms < 1.0e-8);
    }

    #[test]
    fn three_streams_recover_independent_native_rate_maps() {
        let event_ids = vec![10, 20, 30, 40];
        let common_times = [100_i64, 200, 400, 800];
        let mut observations = Vec::new();
        for (event_id, tau) in event_ids.iter().zip(common_times) {
            observations.push(SensorEventObservationV0 {
                stream_id: 0,
                event_id: *event_id,
                local_timestamp: tau,
            });
            observations.push(SensorEventObservationV0 {
                stream_id: 1,
                event_id: *event_id,
                local_timestamp: 2 * tau + 100,
            });
            if *event_id != 20 {
                observations.push(SensorEventObservationV0 {
                    stream_id: 2,
                    event_id: *event_id,
                    local_timestamp: 4 * tau - 200,
                });
            }
        }
        let problem = JointClockProblemV0 {
            stream_ids: vec![0, 1, 2],
            event_ids,
            reference_stream_id: 0,
            observations,
        };
        let fit = fit_joint_clock_ols_v0(&problem).unwrap();
        assert_eq!(fit.design.rank, fit.design.parameter_count);
        let stream_one = fit
            .inverse_clocks
            .iter()
            .find(|clock| clock.stream_id == 1)
            .unwrap();
        let stream_two = fit
            .inverse_clocks
            .iter()
            .find(|clock| clock.stream_id == 2)
            .unwrap();
        assert!((stream_one.inverse_rate - 0.5).abs() < 1.0e-10);
        assert!((stream_one.inverse_offset + 50.0).abs() < 1.0e-8);
        assert!((stream_two.inverse_rate - 0.25).abs() < 1.0e-10);
        assert!((stream_two.inverse_offset - 50.0).abs() < 1.0e-8);
    }

    #[test]
    fn observation_order_does_not_change_joint_fit() {
        let problem = two_stream_problem(6);
        let forward = fit_joint_clock_ols_v0(&problem).unwrap();
        let mut reversed_problem = problem;
        reversed_problem.observations.reverse();
        let reversed = fit_joint_clock_ols_v0(&reversed_problem).unwrap();
        for (left, right) in forward.event_times.iter().zip(&reversed.event_times) {
            assert!((left.common_time - right.common_time).abs() < 1.0e-9);
        }
        for (left, right) in forward.inverse_clocks.iter().zip(&reversed.inverse_clocks) {
            assert!((left.inverse_rate - right.inverse_rate).abs() < 1.0e-10);
            assert!((left.inverse_offset - right.inverse_offset).abs() < 1.0e-8);
        }
    }

    #[test]
    fn duplicate_sensor_event_pair_fails_closed() {
        let mut problem = two_stream_problem(2);
        problem.observations.push(problem.observations[0]);
        assert_eq!(
            fit_joint_clock_ols_v0(&problem),
            Err(JointClockErrorV0::DuplicateStreamEventObservation {
                stream_id: 0,
                event_id: 100,
            })
        );
    }

    #[test]
    fn reference_stream_is_gauge_fixed_to_identity() {
        let problem = two_stream_problem(3);
        let fit = fit_joint_clock_ols_v0(&problem).unwrap();
        let reference = fit
            .inverse_clocks
            .iter()
            .find(|clock| clock.stream_id == 0)
            .unwrap();
        assert_eq!(reference.inverse_rate, 1.0);
        assert_eq!(reference.inverse_offset, 0.0);
    }
}
