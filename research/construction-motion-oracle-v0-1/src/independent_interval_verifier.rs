//! Independent interval-arithmetic verifier for fixed-orientation, linearly
//! translating capsule pairs.
//!
//! The production CCD kernel minimizes a convex quadratic using active-set
//! normal equations. This module deliberately uses a different method:
//! outward-rounded interval bounds over the full time × moving-axis ×
//! obstacle-axis parameter box. A box is discarded only when a conservative
//! lower distance bound is strictly greater than a conservative upper bound
//! on the contact radius.
//!
//! A complete search with every box discarded is a software-level no-contact
//! certificate for the declared affine capsule model. A parameter sample
//! whose outward distance upper bound is within the inward contact threshold
//! is a positive contact witness. Budget exhaustion or uncertain arithmetic
//! is UNRESOLVED, never clear.

use crate::co_translation_ccd::{
    AdaptiveCertificateError, AdaptiveCollisionCertificate, AdaptiveCollisionStatus,
    verify_adaptive_collision_certificate,
};
use crate::{Capsule, NormalizedTimeWindow, Vec3};

const CONTACT_EPSILON_MM: f64 = 1.0e-9;
const MAX_VERIFIER_DEPTH: u8 = 52;
const MAX_VERIFIER_NODES: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntervalVerifierError {
    InvalidRadius,
    NonFiniteInput,
    InvalidTimeWindow,
    InvalidConfiguration,
    InvalidUncertainty,
    ArithmeticOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntervalVerificationStatus {
    Clear,
    ContactWitness,
    Unresolved,
}

/// Deterministic axis-aligned bounds on the relative-motion coefficients.
///
/// These are set-membership bounds, not standard deviations or probabilities.
/// They widen coefficient intervals; correlations are discarded conservatively.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapsuleRelativeUncertainty {
    /// Bound on A.start - B.start, in millimetres per axis.
    pub origin_relative_mm: Vec3,
    /// Bound on delta_A - delta_B, in millimetres per axis.
    pub relative_translation_mm: Vec3,
    /// Bound on A.end - A.start, in millimetres per axis.
    pub moving_axis_mm: Vec3,
    /// Bound on B.end - B.start, in millimetres per axis.
    pub obstacle_axis_mm: Vec3,
}

impl CapsuleRelativeUncertainty {
    pub fn zero() -> Self {
        Self {
            origin_relative_mm: Vec3::new(0.0, 0.0, 0.0),
            relative_translation_mm: Vec3::new(0.0, 0.0, 0.0),
            moving_axis_mm: Vec3::new(0.0, 0.0, 0.0),
            obstacle_axis_mm: Vec3::new(0.0, 0.0, 0.0),
        }
    }

    fn validate(self) -> Result<(), IntervalVerifierError> {
        for bound in [
            self.origin_relative_mm,
            self.relative_translation_mm,
            self.moving_axis_mm,
            self.obstacle_axis_mm,
        ] {
            if !is_finite(bound) || bound.x < 0.0 || bound.y < 0.0 || bound.z < 0.0 {
                return Err(IntervalVerifierError::InvalidUncertainty);
            }
        }
        Ok(())
    }
}

impl Default for CapsuleRelativeUncertainty {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntervalVerifierConfig {
    pub maximum_nodes: u32,
    pub maximum_depth: u8,
}

impl Default for IntervalVerifierConfig {
    fn default() -> Self {
        Self {
            maximum_nodes: 100_000,
            maximum_depth: 40,
        }
    }
}

impl IntervalVerifierConfig {
    fn validate(self) -> Result<(), IntervalVerifierError> {
        if self.maximum_nodes == 0
            || self.maximum_nodes > MAX_VERIFIER_NODES
            || self.maximum_depth > MAX_VERIFIER_DEPTH
        {
            return Err(IntervalVerifierError::InvalidConfiguration);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntervalContactWitness {
    pub time_parameter: f64,
    pub moving_axis_parameter: f64,
    pub obstacle_axis_parameter: f64,
    pub distance_squared_upper_mm2: f64,
    pub contact_squared_lower_mm2: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntervalVerificationReceipt {
    pub window: NormalizedTimeWindow,
    pub config: IntervalVerifierConfig,
    pub status: IntervalVerificationStatus,
    pub parameter_boxes_visited: u32,
    pub parameter_boxes_pruned: u32,
    pub maximum_depth_reached: u8,
    pub witness: Option<IntervalContactWitness>,
}

impl IntervalVerificationReceipt {
    pub fn no_contact_proven(self) -> bool {
        self.status == IntervalVerificationStatus::Clear
    }

    pub fn contact_witness_established(self) -> bool {
        self.status == IntervalVerificationStatus::ContactWitness && self.witness.is_some()
    }
}

/// A separately computed interval-verification receipt for every leaf of one
/// adaptive certificate. This verifier re-runs the full time × axis-parameter
/// search per leaf; it does not trust producer narrow-phase outputs or pruned
/// AABB outcomes as evidence of clearance.
#[derive(Clone, Debug, PartialEq)]
pub struct IndependentAdaptiveNoContactReceipt {
    pub leaf_receipts: Vec<IntervalVerificationReceipt>,
    pub parameter_boxes_visited: u64,
    pub parameter_boxes_pruned: u64,
    pub maximum_depth_reached: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndependentAdaptiveVerificationError {
    CertificateReplay(AdaptiveCertificateError),
    CertificateNotNoContact,
    Interval(IntervalVerifierError),
    IndependentContactWitness { leaf_index: usize },
    IndependentUnresolved { leaf_index: usize },
    CounterOverflow,
}

/// Independently verify a producer's NoContact certificate using the interval
/// branch-and-bound algorithm, not the active-set narrow-phase solver.
///
/// Every terminal time leaf—including leaves the producer said were pruned by
/// AABB—is rechecked over the full time × moving-axis × obstacle-axis domain.
/// The function succeeds only when every leaf is independently proven clear.
pub fn verify_adaptive_no_contact_independently(
    certificate: &AdaptiveCollisionCertificate,
    config: IntervalVerifierConfig,
) -> Result<IndependentAdaptiveNoContactReceipt, IndependentAdaptiveVerificationError> {
    verify_adaptive_collision_certificate(certificate)
        .map_err(IndependentAdaptiveVerificationError::CertificateReplay)?;
    if certificate.report.status != AdaptiveCollisionStatus::NoContact {
        return Err(IndependentAdaptiveVerificationError::CertificateNotNoContact);
    }

    let mut leaf_receipts = Vec::with_capacity(certificate.leaves.len());
    let mut parameter_boxes_visited = 0_u64;
    let mut parameter_boxes_pruned = 0_u64;
    let mut maximum_depth_reached = 0_u8;

    for (leaf_index, leaf) in certificate.leaves.iter().enumerate() {
        let receipt = verify_capsule_interval_no_contact(
            certificate.moving,
            certificate.moving_translation,
            certificate.obstacle,
            certificate.obstacle_translation,
            leaf.window,
            config,
        )
        .map_err(IndependentAdaptiveVerificationError::Interval)?;

        match receipt.status {
            IntervalVerificationStatus::Clear => {}
            IntervalVerificationStatus::ContactWitness => {
                return Err(
                    IndependentAdaptiveVerificationError::IndependentContactWitness { leaf_index },
                );
            }
            IntervalVerificationStatus::Unresolved => {
                return Err(
                    IndependentAdaptiveVerificationError::IndependentUnresolved { leaf_index },
                );
            }
        }

        parameter_boxes_visited = parameter_boxes_visited
            .checked_add(u64::from(receipt.parameter_boxes_visited))
            .ok_or(IndependentAdaptiveVerificationError::CounterOverflow)?;
        parameter_boxes_pruned = parameter_boxes_pruned
            .checked_add(u64::from(receipt.parameter_boxes_pruned))
            .ok_or(IndependentAdaptiveVerificationError::CounterOverflow)?;
        maximum_depth_reached = maximum_depth_reached.max(receipt.maximum_depth_reached);
        leaf_receipts.push(receipt);
    }

    Ok(IndependentAdaptiveNoContactReceipt {
        leaf_receipts,
        parameter_boxes_visited,
        parameter_boxes_pruned,
        maximum_depth_reached,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Interval {
    lower: f64,
    upper: f64,
}

impl Interval {
    fn exact(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }

    fn point(value: f64) -> Self {
        Self {
            lower: value,
            upper: value,
        }
    }

    fn add(self, rhs: Self) -> Option<Self> {
        let lower = self.lower + rhs.lower;
        let upper = self.upper + rhs.upper;
        if !lower.is_finite() || !upper.is_finite() {
            return None;
        }
        let result = Self {
            lower: next_down(lower),
            upper: next_up(upper),
        };
        (result.lower.is_finite() && result.upper.is_finite()).then_some(result)
    }

    fn neg(self) -> Self {
        Self {
            lower: next_down(-self.upper),
            upper: next_up(-self.lower),
        }
    }

    fn multiply(self, rhs: Self) -> Option<Self> {
        let products = [
            self.lower * rhs.lower,
            self.lower * rhs.upper,
            self.upper * rhs.lower,
            self.upper * rhs.upper,
        ];
        if products.iter().any(|value| !value.is_finite()) {
            return None;
        }
        let lower = products.iter().copied().fold(f64::INFINITY, f64::min);
        let upper = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let result = Self {
            lower: next_down(lower),
            upper: next_up(upper),
        };
        (result.lower.is_finite() && result.upper.is_finite()).then_some(result)
    }

    fn maximum_absolute(self) -> f64 {
        self.lower.abs().max(self.upper.abs())
    }

    fn minimum_absolute(self) -> f64 {
        if self.lower > 0.0 {
            self.lower
        } else if self.upper < 0.0 {
            -self.upper
        } else {
            0.0
        }
    }

    fn width(self) -> f64 {
        self.upper - self.lower
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ParameterBox {
    time: Interval,
    moving_axis: Interval,
    obstacle_axis: Interval,
    depth: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct GeometryCoefficients {
    origin: [Interval; 3],
    relative_translation: [Interval; 3],
    moving_axis: [Interval; 3],
    negative_obstacle_axis: [Interval; 3],
    contact_squared_lower_mm2: f64,
    contact_squared_upper_mm2: f64,
}

/// Verify a fixed-orientation capsule pair over one normalized time interval.
///
/// The search covers the whole time × centerline-parameter domain. For each
/// parameter box, outward-rounded coordinate intervals enclose every possible
/// relative centerline vector. A box is pruned only when its conservative
/// squared-distance lower bound exceeds the conservative contact threshold.
pub fn verify_capsule_interval_no_contact(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    window: NormalizedTimeWindow,
    config: IntervalVerifierConfig,
) -> Result<IntervalVerificationReceipt, IntervalVerifierError> {
    verify_capsule_interval_no_contact_with_uncertainty(
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
        CapsuleRelativeUncertainty::zero(),
        window,
        config,
    )
}

/// Verify no contact for every realization inside the declared deterministic
/// axis-aligned coefficient uncertainty set.
///
/// `Clear` means the complete time × axis-parameter domain is separated for
/// every admissible coefficient realization. `ContactWitness` is emitted only
/// when every realization at one parameter sample is within the contact
/// threshold. Otherwise the result remains `Unresolved` when the widened
/// intervals cannot prove either statement.
pub fn verify_capsule_interval_no_contact_with_uncertainty(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    uncertainty: CapsuleRelativeUncertainty,
    window: NormalizedTimeWindow,
    config: IntervalVerifierConfig,
) -> Result<IntervalVerificationReceipt, IntervalVerifierError> {
    uncertainty.validate()?;
    validate_inputs(
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
        window,
        config,
    )?;
    let coefficients = build_coefficients(
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
        uncertainty,
    )?;

    let mut stack = vec![ParameterBox {
        time: Interval::exact(window.start_parameter, window.end_parameter),
        moving_axis: Interval::exact(0.0, 1.0),
        obstacle_axis: Interval::exact(0.0, 1.0),
        depth: 0,
    }];
    let mut visited = 0_u32;
    let mut pruned = 0_u32;
    let mut maximum_depth_reached = 0_u8;

    while let Some(parameter_box) = stack.pop() {
        if visited >= config.maximum_nodes {
            return Ok(IntervalVerificationReceipt {
                window,
                config,
                status: IntervalVerificationStatus::Unresolved,
                parameter_boxes_visited: visited,
                parameter_boxes_pruned: pruned,
                maximum_depth_reached,
                witness: None,
            });
        }
        visited += 1;
        maximum_depth_reached = maximum_depth_reached.max(parameter_box.depth);

        if lower_distance_squared(coefficients, parameter_box)?
            > coefficients.contact_squared_upper_mm2
        {
            pruned += 1;
            continue;
        }

        if let Some(witness) = contact_witness(coefficients, parameter_box)? {
            return Ok(IntervalVerificationReceipt {
                window,
                config,
                status: IntervalVerificationStatus::ContactWitness,
                parameter_boxes_visited: visited,
                parameter_boxes_pruned: pruned,
                maximum_depth_reached,
                witness: Some(witness),
            });
        }

        if parameter_box.depth >= config.maximum_depth {
            return Ok(IntervalVerificationReceipt {
                window,
                config,
                status: IntervalVerificationStatus::Unresolved,
                parameter_boxes_visited: visited,
                parameter_boxes_pruned: pruned,
                maximum_depth_reached,
                witness: None,
            });
        }

        let Some((left, right)) = split_parameter_box(parameter_box, coefficients) else {
            return Ok(IntervalVerificationReceipt {
                window,
                config,
                status: IntervalVerificationStatus::Unresolved,
                parameter_boxes_visited: visited,
                parameter_boxes_pruned: pruned,
                maximum_depth_reached,
                witness: None,
            });
        };
        stack.push(right);
        stack.push(left);
    }

    Ok(IntervalVerificationReceipt {
        window,
        config,
        status: IntervalVerificationStatus::Clear,
        parameter_boxes_visited: visited,
        parameter_boxes_pruned: pruned,
        maximum_depth_reached,
        witness: None,
    })
}

/// Re-run the independent interval search against the same frozen inputs.
pub fn replay_interval_verification_receipt(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    receipt: IntervalVerificationReceipt,
) -> Result<(), IntervalVerifierError> {
    let replay = verify_capsule_interval_no_contact(
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
        receipt.window,
        receipt.config,
    )?;
    if replay == receipt {
        Ok(())
    } else {
        Err(IntervalVerifierError::ArithmeticOverflow)
    }
}

fn validate_inputs(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    window: NormalizedTimeWindow,
    config: IntervalVerifierConfig,
) -> Result<(), IntervalVerifierError> {
    config.validate()?;
    if !window.start_parameter.is_finite()
        || !window.end_parameter.is_finite()
        || window.start_parameter < 0.0
        || window.end_parameter > 1.0
        || window.start_parameter >= window.end_parameter
    {
        return Err(IntervalVerifierError::InvalidTimeWindow);
    }
    for capsule in [moving, obstacle] {
        if !capsule.radius_mm.is_finite() || capsule.radius_mm < 0.0 {
            return Err(IntervalVerifierError::InvalidRadius);
        }
        if !is_finite(capsule.start) || !is_finite(capsule.end) {
            return Err(IntervalVerifierError::NonFiniteInput);
        }
    }
    if !is_finite(moving_translation) || !is_finite(obstacle_translation) {
        return Err(IntervalVerifierError::NonFiniteInput);
    }
    Ok(())
}

fn build_coefficients(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    uncertainty: CapsuleRelativeUncertainty,
) -> Result<GeometryCoefficients, IntervalVerifierError> {
    let mut origin = vector_difference(moving.start, obstacle.start)?;
    let mut relative_translation = vector_difference(moving_translation, obstacle_translation)?;
    let mut moving_axis = vector_difference(moving.end, moving.start)?;
    let mut obstacle_axis = vector_difference(obstacle.end, obstacle.start)?;

    for axis in 0..3 {
        origin[axis] = widen_interval(origin[axis], component(uncertainty.origin_relative_mm, axis))?;
        relative_translation[axis] = widen_interval(
            relative_translation[axis],
            component(uncertainty.relative_translation_mm, axis),
        )?;
        moving_axis[axis] =
            widen_interval(moving_axis[axis], component(uncertainty.moving_axis_mm, axis))?;
        obstacle_axis[axis] =
            widen_interval(obstacle_axis[axis], component(uncertainty.obstacle_axis_mm, axis))?;
    }
    let negative_obstacle_axis = obstacle_axis.map(Interval::neg);

    let radius_sum = moving.radius_mm + obstacle.radius_mm;
    if !radius_sum.is_finite() {
        return Err(IntervalVerifierError::ArithmeticOverflow);
    }
    let radius_sum_lower = next_down(radius_sum);
    let radius_sum_upper = next_up(radius_sum);
    let contact_lower = next_down(radius_sum_lower + CONTACT_EPSILON_MM).max(0.0);
    let contact_upper = next_up(radius_sum_upper + CONTACT_EPSILON_MM);
    let contact_squared_lower = next_down(contact_lower * contact_lower).max(0.0);
    let contact_squared_upper = next_up(contact_upper * contact_upper);
    if !contact_squared_lower.is_finite() || !contact_squared_upper.is_finite() {
        return Err(IntervalVerifierError::ArithmeticOverflow);
    }

    Ok(GeometryCoefficients {
        origin,
        relative_translation,
        moving_axis,
        negative_obstacle_axis,
        contact_squared_lower_mm2: contact_squared_lower,
        contact_squared_upper_mm2: contact_squared_upper,
    })
}

fn component(value: Vec3, axis: usize) -> f64 {
    match axis {
        0 => value.x,
        1 => value.y,
        _ => value.z,
    }
}

/// Widen a coefficient interval by a nonnegative input uncertainty, with
/// outward rounding after both bound operations.
fn widen_interval(
    value: Interval,
    margin: f64,
) -> Result<Interval, IntervalVerifierError> {
    if margin == 0.0 {
        return Ok(value);
    }
    let lower = value.lower - margin;
    let upper = value.upper + margin;
    if !lower.is_finite() || !upper.is_finite() {
        return Err(IntervalVerifierError::ArithmeticOverflow);
    }
    let widened = Interval {
        lower: next_down(lower),
        upper: next_up(upper),
    };
    if !widened.lower.is_finite() || !widened.upper.is_finite() {
        return Err(IntervalVerifierError::ArithmeticOverflow);
    }
    Ok(widened)
}

fn vector_difference(left: Vec3, right: Vec3) -> Result<[Interval; 3], IntervalVerifierError> {
    Ok([
        outward_difference(left.x, right.x)?,
        outward_difference(left.y, right.y)?,
        outward_difference(left.z, right.z)?,
    ])
}

fn outward_difference(left: f64, right: f64) -> Result<Interval, IntervalVerifierError> {
    let nearest = left - right;
    if !nearest.is_finite() {
        return Err(IntervalVerifierError::ArithmeticOverflow);
    }
    let result = Interval {
        lower: next_down(nearest),
        upper: next_up(nearest),
    };
    if !result.lower.is_finite() || !result.upper.is_finite() {
        return Err(IntervalVerifierError::ArithmeticOverflow);
    }
    Ok(result)
}

fn relative_vector_intervals(
    coefficients: GeometryCoefficients,
    parameter_box: ParameterBox,
) -> Result<[Interval; 3], IntervalVerifierError> {
    let mut result = [Interval::point(0.0); 3];
    for axis in 0..3 {
        let time_term = coefficients.relative_translation[axis]
            .multiply(parameter_box.time)
            .ok_or(IntervalVerifierError::ArithmeticOverflow)?;
        let moving_term = coefficients.moving_axis[axis]
            .multiply(parameter_box.moving_axis)
            .ok_or(IntervalVerifierError::ArithmeticOverflow)?;
        let obstacle_term = coefficients.negative_obstacle_axis[axis]
            .multiply(parameter_box.obstacle_axis)
            .ok_or(IntervalVerifierError::ArithmeticOverflow)?;
        result[axis] = coefficients.origin[axis]
            .add(time_term)
            .and_then(|value| value.add(moving_term))
            .and_then(|value| value.add(obstacle_term))
            .ok_or(IntervalVerifierError::ArithmeticOverflow)?;
    }
    Ok(result)
}

fn lower_distance_squared(
    coefficients: GeometryCoefficients,
    parameter_box: ParameterBox,
) -> Result<f64, IntervalVerifierError> {
    let relative = relative_vector_intervals(coefficients, parameter_box)?;
    let mut lower_squared = 0.0_f64;
    for component in relative {
        let minimum = component.minimum_absolute();
        let term = next_down(minimum * minimum).max(0.0);
        lower_squared = next_down(lower_squared + term).max(0.0);
    }
    if !lower_squared.is_finite() {
        return Err(IntervalVerifierError::ArithmeticOverflow);
    }
    Ok(lower_squared)
}

fn contact_witness(
    coefficients: GeometryCoefficients,
    parameter_box: ParameterBox,
) -> Result<Option<IntervalContactWitness>, IntervalVerifierError> {
    let time = midpoint(parameter_box.time);
    let moving_axis = midpoint(parameter_box.moving_axis);
    let obstacle_axis = midpoint(parameter_box.obstacle_axis);
    let point = ParameterBox {
        time: Interval::point(time),
        moving_axis: Interval::point(moving_axis),
        obstacle_axis: Interval::point(obstacle_axis),
        depth: parameter_box.depth,
    };
    let relative = relative_vector_intervals(coefficients, point)?;
    let mut upper_squared = 0.0_f64;
    for component in relative {
        let maximum = component.maximum_absolute();
        let term = next_up(maximum * maximum);
        upper_squared = next_up(upper_squared + term);
    }
    if !upper_squared.is_finite() {
        return Err(IntervalVerifierError::ArithmeticOverflow);
    }
    if upper_squared <= coefficients.contact_squared_lower_mm2 {
        Ok(Some(IntervalContactWitness {
            time_parameter: time,
            moving_axis_parameter: moving_axis,
            obstacle_axis_parameter: obstacle_axis,
            distance_squared_upper_mm2: upper_squared,
            contact_squared_lower_mm2: coefficients.contact_squared_lower_mm2,
        }))
    } else {
        Ok(None)
    }
}

fn split_parameter_box(
    parameter_box: ParameterBox,
    coefficients: GeometryCoefficients,
) -> Option<(ParameterBox, ParameterBox)> {
    // This score affects search order only, not the validity of interval pruning.
    let influence = [
        interval_vector_norm_upper(coefficients.relative_translation) * parameter_box.time.width(),
        interval_vector_norm_upper(coefficients.moving_axis) * parameter_box.moving_axis.width(),
        interval_vector_norm_upper(coefficients.negative_obstacle_axis)
            * parameter_box.obstacle_axis.width(),
    ];
    let selected = (0..3).max_by(|left, right| influence[*left].total_cmp(&influence[*right]))?;
    let source = match selected {
        0 => parameter_box.time,
        1 => parameter_box.moving_axis,
        _ => parameter_box.obstacle_axis,
    };
    let split = midpoint(source);
    if split <= source.lower || split >= source.upper || !split.is_finite() {
        return None;
    }
    let left_interval = Interval::exact(source.lower, split);
    let right_interval = Interval::exact(split, source.upper);
    let mut left = parameter_box;
    let mut right = parameter_box;
    left.depth = parameter_box.depth + 1;
    right.depth = parameter_box.depth + 1;
    match selected {
        0 => {
            left.time = left_interval;
            right.time = right_interval;
        }
        1 => {
            left.moving_axis = left_interval;
            right.moving_axis = right_interval;
        }
        _ => {
            left.obstacle_axis = left_interval;
            right.obstacle_axis = right_interval;
        }
    }
    Some((left, right))
}

fn interval_vector_norm_upper(values: [Interval; 3]) -> f64 {
    values[0]
        .maximum_absolute()
        .hypot(values[1].maximum_absolute())
        .hypot(values[2].maximum_absolute())
}

fn midpoint(interval: Interval) -> f64 {
    interval.lower + (interval.upper - interval.lower) * 0.5
}

fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
}

fn is_finite(point: Vec3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn point_capsule(x: f64, y: f64, z: f64, radius: f64) -> Capsule {
        Capsule::new(p(x, y, z), p(x, y, z), radius).unwrap()
    }

    fn window() -> NormalizedTimeWindow {
        NormalizedTimeWindow {
            start_parameter: 0.0,
            end_parameter: 1.0,
        }
    }

    #[test]
    fn independent_interval_algorithm_can_recheck_a_complete_adaptive_no_contact_certificate() {
        use crate::co_translation_ccd::{
            AdaptiveSubdivisionConfig, co_translating_capsule_adaptive_query_with_certificate,
        };

        let moving = point_capsule(0.0, 0.0, 0.0, 0.5);
        let obstacle = point_capsule(10.0, 0.0, 0.0, 0.5);
        let certificate = co_translating_capsule_adaptive_query_with_certificate(
            moving,
            p(0.0, 0.0, 0.0),
            obstacle,
            p(0.0, 0.0, 0.0),
            AdaptiveSubdivisionConfig {
                maximum_depth: 4,
                minimum_window_width: 1.0 / 16.0,
            },
        )
        .unwrap();
        assert_eq!(
            certificate.report.status,
            AdaptiveCollisionStatus::NoContact
        );

        let independent = verify_adaptive_no_contact_independently(
            &certificate,
            IntervalVerifierConfig::default(),
        )
        .unwrap();
        assert_eq!(independent.leaf_receipts.len(), certificate.leaves.len());
        assert!(
            independent
                .leaf_receipts
                .iter()
                .all(|receipt| receipt.status == IntervalVerificationStatus::Clear)
        );
        assert!(independent.parameter_boxes_visited > 0);
    }

    #[test]
    fn independent_no_contact_recheck_abstains_when_parameter_budget_is_exhausted() {
        use crate::co_translation_ccd::{
            AdaptiveSubdivisionConfig, co_translating_capsule_adaptive_query_with_certificate,
        };

        // Parallel diagonal axes whose AABBs overlap, but whose true axis
        // separation exceeds the radius sum. The active-set producer can
        // solve this bounded geometry, while the independent interval
        // verifier needs subdivision to eliminate the broad parameter box.
        let moving = Capsule::new(p(0.0, 0.0, 0.0), p(10.0, 10.0, 0.0), 1.0).unwrap();
        let obstacle = Capsule::new(p(0.0, 10.0, 0.0), p(10.0, 20.0, 0.0), 1.0).unwrap();
        assert!(moving.aabb().overlaps(obstacle.aabb()));

        let certificate = co_translating_capsule_adaptive_query_with_certificate(
            moving,
            p(0.0, 0.0, 0.0),
            obstacle,
            p(0.0, 0.0, 0.0),
            AdaptiveSubdivisionConfig {
                maximum_depth: 0,
                minimum_window_width: 1.0,
            },
        )
        .unwrap();
        assert_eq!(certificate.report.status, AdaptiveCollisionStatus::NoContact);

        assert_eq!(
            verify_adaptive_no_contact_independently(
                &certificate,
                IntervalVerifierConfig {
                    maximum_nodes: 1,
                    maximum_depth: 0,
                },
            ),
            Err(IndependentAdaptiveVerificationError::IndependentUnresolved { leaf_index: 0 })
        );
    }

    #[test]
    fn static_separation_is_proven_clear() {
        let result = verify_capsule_interval_no_contact(
            point_capsule(0.0, 0.0, 0.0, 0.5),
            p(0.0, 0.0, 0.0),
            point_capsule(10.0, 0.0, 0.0, 0.5),
            p(0.0, 0.0, 0.0),
            window(),
            IntervalVerifierConfig::default(),
        )
        .unwrap();
        assert_eq!(result.status, IntervalVerificationStatus::Clear);
        assert_eq!(result.parameter_boxes_visited, 1);
        assert!(result.no_contact_proven());
    }

    #[test]
    fn overlapping_aabbs_can_still_be_proven_clear_by_diagonal_distance() {
        let a = point_capsule(0.0, 0.0, 0.0, 0.6);
        let b = point_capsule(0.0, 0.9, 0.9, 0.6);
        assert!(a.aabb().overlaps(b.aabb()));
        let result = verify_capsule_interval_no_contact(
            a,
            p(0.0, 0.0, 0.0),
            b,
            p(0.0, 0.0, 0.0),
            window(),
            IntervalVerifierConfig::default(),
        )
        .unwrap();
        assert_eq!(result.status, IntervalVerificationStatus::Clear);
    }

    #[test]
    fn crossing_capsules_produce_a_positive_contact_witness() {
        let result = verify_capsule_interval_no_contact(
            point_capsule(-10.0, 0.0, 0.0, 0.5),
            p(20.0, 0.0, 0.0),
            point_capsule(10.0, 0.0, 0.0, 0.5),
            p(-20.0, 0.0, 0.0),
            window(),
            IntervalVerifierConfig::default(),
        )
        .unwrap();
        assert_eq!(result.status, IntervalVerificationStatus::ContactWitness);
        let witness = result.witness.unwrap();
        assert!((witness.time_parameter - 0.5).abs() < 1.0e-12);
        assert!(witness.distance_squared_upper_mm2 <= witness.contact_squared_lower_mm2);
    }

    #[test]
    fn off_center_contact_with_one_node_budget_abstains_instead_of_clearing() {
        let result = verify_capsule_interval_no_contact(
            point_capsule(0.0, 0.0, 0.0, 0.1),
            p(10.0, 0.0, 0.0),
            point_capsule(3.0, 0.0, 0.0, 0.1),
            p(0.0, 0.0, 0.0),
            window(),
            IntervalVerifierConfig {
                maximum_nodes: 1,
                maximum_depth: 0,
            },
        )
        .unwrap();
        assert_eq!(result.status, IntervalVerificationStatus::Unresolved);
    }

    #[test]
    fn interval_receipt_replays_deterministically() {
        let moving = point_capsule(0.0, 0.0, 0.0, 0.5);
        let obstacle = point_capsule(8.0, 0.0, 0.0, 0.5);
        let moving_translation = p(0.0, 0.0, 0.0);
        let obstacle_translation = p(0.0, 0.0, 0.0);
        let receipt = verify_capsule_interval_no_contact(
            moving,
            moving_translation,
            obstacle,
            obstacle_translation,
            window(),
            IntervalVerifierConfig::default(),
        )
        .unwrap();
        replay_interval_verification_receipt(
            moving,
            moving_translation,
            obstacle,
            obstacle_translation,
            receipt,
        )
        .unwrap();
    }

    #[test]
    fn outward_neighbors_bracket_positive_negative_and_zero_values() {
        for value in [
            -1.0e100,
            -1.0,
            -f64::MIN_POSITIVE,
            0.0,
            f64::MIN_POSITIVE,
            1.0,
            1.0e100,
        ] {
            assert!(next_down(value) <= value);
            assert!(next_up(value) >= value);
        }
    }

    #[test]
    fn zero_uncertainty_reproduces_the_nominal_receipt_exactly() {
        let moving = point_capsule(0.0, 0.0, 0.0, 0.5);
        let obstacle = point_capsule(5.0, 0.0, 0.0, 0.5);
        let nominal = verify_capsule_interval_no_contact(
            moving, p(0.0, 0.0, 0.0), obstacle, p(0.0, 0.0, 0.0),
            window(), IntervalVerifierConfig::default(),
        ).unwrap();
        let robust = verify_capsule_interval_no_contact_with_uncertainty(
            moving, p(0.0, 0.0, 0.0), obstacle, p(0.0, 0.0, 0.0),
            CapsuleRelativeUncertainty::zero(), window(), IntervalVerifierConfig::default(),
        ).unwrap();
        assert_eq!(nominal, robust);
    }

    #[test]
    fn bounded_uncertainty_that_remains_separated_can_still_prove_clear() {
        let result = verify_capsule_interval_no_contact_with_uncertainty(
            point_capsule(0.0, 0.0, 0.0, 0.5),
            p(0.0, 0.0, 0.0),
            point_capsule(5.0, 0.0, 0.0, 0.5),
            p(0.0, 0.0, 0.0),
            CapsuleRelativeUncertainty {
                origin_relative_mm: p(1.0, 0.2, 0.2),
                ..CapsuleRelativeUncertainty::zero()
            },
            window(), IntervalVerifierConfig::default(),
        ).unwrap();
        assert_eq!(result.status, IntervalVerificationStatus::Clear);
    }

    #[test]
    fn nominal_clear_path_with_uncertainty_overlap_is_never_clear() {
        let result = verify_capsule_interval_no_contact_with_uncertainty(
            point_capsule(0.0, 0.0, 0.0, 0.5), p(0.0, 0.0, 0.0),
            point_capsule(2.0, 0.0, 0.0, 0.5), p(0.0, 0.0, 0.0),
            CapsuleRelativeUncertainty {
                origin_relative_mm: p(1.5, 0.0, 0.0),
                ..CapsuleRelativeUncertainty::zero()
            },
            window(), IntervalVerifierConfig { maximum_nodes: 128, maximum_depth: 16 },
        ).unwrap();
        assert_eq!(result.status, IntervalVerificationStatus::Unresolved);
    }

    #[test]
    fn small_uncertainty_inside_contact_radius_produces_guaranteed_contact_witness() {
        let result = verify_capsule_interval_no_contact_with_uncertainty(
            point_capsule(0.0, 0.0, 0.0, 0.75), p(0.0, 0.0, 0.0),
            point_capsule(0.5, 0.0, 0.0, 0.75), p(0.0, 0.0, 0.0),
            CapsuleRelativeUncertainty {
                origin_relative_mm: p(0.1, 0.1, 0.1),
                ..CapsuleRelativeUncertainty::zero()
            },
            window(), IntervalVerifierConfig::default(),
        ).unwrap();
        assert_eq!(result.status, IntervalVerificationStatus::ContactWitness);
    }

    #[test]
    fn negative_and_nonfinite_uncertainty_reject() {
        let moving = point_capsule(0.0, 0.0, 0.0, 0.5);
        let obstacle = point_capsule(5.0, 0.0, 0.0, 0.5);
        for uncertainty in [
            CapsuleRelativeUncertainty { origin_relative_mm: p(-1.0, 0.0, 0.0), ..CapsuleRelativeUncertainty::zero() },
            CapsuleRelativeUncertainty { moving_axis_mm: p(0.0, f64::NAN, 0.0), ..CapsuleRelativeUncertainty::zero() },
            CapsuleRelativeUncertainty { obstacle_axis_mm: p(f64::INFINITY, 0.0, 0.0), ..CapsuleRelativeUncertainty::zero() },
        ] {
            assert_eq!(
                verify_capsule_interval_no_contact_with_uncertainty(
                    moving, p(0.0, 0.0, 0.0), obstacle, p(0.0, 0.0, 0.0),
                    uncertainty, window(), IntervalVerifierConfig::default(),
                ),
                Err(IntervalVerifierError::InvalidUncertainty)
            );
        }
    }

    #[test]
    fn uncertainty_overflow_is_an_error_not_clear() {
        let result = verify_capsule_interval_no_contact_with_uncertainty(
            point_capsule(0.0, 0.0, 0.0, 0.5), p(0.0, 0.0, 0.0),
            point_capsule(5.0, 0.0, 0.0, 0.5), p(0.0, 0.0, 0.0),
            CapsuleRelativeUncertainty {
                origin_relative_mm: p(f64::MAX, 0.0, 0.0),
                ..CapsuleRelativeUncertainty::zero()
            },
            window(), IntervalVerifierConfig::default(),
        );
        assert_eq!(result, Err(IntervalVerifierError::ArithmeticOverflow));
    }

    #[test]
    fn invalid_inputs_never_generate_clear_receipts() {
        let invalid = Capsule {
            radius_mm: f64::NAN,
            ..point_capsule(0.0, 0.0, 0.0, 0.5)
        };
        assert_eq!(
            verify_capsule_interval_no_contact(
                invalid,
                p(0.0, 0.0, 0.0),
                point_capsule(5.0, 0.0, 0.0, 0.5),
                p(0.0, 0.0, 0.0),
                window(),
                IntervalVerifierConfig::default(),
            ),
            Err(IntervalVerifierError::InvalidRadius)
        );
    }
}
