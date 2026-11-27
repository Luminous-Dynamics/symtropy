// SPDX-License-Identifier: MIT

//! Outward-rounded interval forward-mode automatic differentiation.
//!
//! The implemented elementary operations are deliberately limited to a
//! transparent algebraic/rational subset plus square root. No platform libm
//! transcendental function is treated as rigorously enclosed.

use std::collections::HashSet;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    lower: f64,
    upper: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntervalError {
    NaNEndpoint,
    ReversedEndpoints,
    SquareRootOutsideRealDomain,
}

impl Interval {
    pub const ENTIRE: Self = Self {
        lower: f64::NEG_INFINITY,
        upper: f64::INFINITY,
    };

    pub const ZERO: Self = Self {
        lower: 0.0,
        upper: 0.0,
    };

    pub const ONE: Self = Self {
        lower: 1.0,
        upper: 1.0,
    };

    pub fn new(lower: f64, upper: f64) -> Result<Self, IntervalError> {
        if lower.is_nan() || upper.is_nan() {
            return Err(IntervalError::NaNEndpoint);
        }
        if lower > upper {
            return Err(IntervalError::ReversedEndpoints);
        }
        Ok(Self { lower, upper })
    }

    pub fn point(value: f64) -> Result<Self, IntervalError> {
        Self::new(value, value)
    }

    pub const fn lower(self) -> f64 {
        self.lower
    }

    pub const fn upper(self) -> f64 {
        self.upper
    }

    pub const fn contains_zero(self) -> bool {
        self.lower <= 0.0 && self.upper >= 0.0
    }

    pub const fn is_finite(self) -> bool {
        self.lower.is_finite() && self.upper.is_finite()
    }

    pub const fn is_point_zero(self) -> bool {
        self.lower == 0.0 && self.upper == 0.0
    }

    pub fn abs_upper_bound(self) -> Option<f64> {
        if !self.is_finite() {
            return None;
        }
        Some(self.lower.abs().max(self.upper.abs()))
    }

    /// Enclose x² without the dependency overestimate of x * x when x
    /// crosses zero. For X containing zero, the exact lower bound is zero.
    pub fn square(self) -> Self {
        if self.contains_zero() {
            let maximum = self.lower.abs().max(self.upper.abs());
            let upper = if maximum == 0.0 {
                0.0
            } else {
                next_up(maximum * maximum)
            };
            return Self { lower: 0.0, upper };
        }

        let left = self.lower * self.lower;
        let right = self.upper * self.upper;
        let lower = next_down(left.min(right)).max(0.0);
        let upper = next_up(left.max(right));
        Self { lower, upper }
    }

    pub fn sqrt(self) -> Result<Self, IntervalError> {
        // The complete interval must lie in the real domain. Clipping a
        // negative lower endpoint to zero would silently discard part of the
        // declared parameter box and produce an invalid certificate.
        if self.lower < 0.0 {
            return Err(IntervalError::SquareRootOutsideRealDomain);
        }
        let lower = if self.lower == 0.0 {
            0.0
        } else {
            next_down(self.lower.sqrt()).max(0.0)
        };
        let upper = if self.upper == f64::INFINITY {
            f64::INFINITY
        } else {
            next_up(self.upper.sqrt())
        };
        Self::new(lower, upper)
    }

    pub fn reciprocal(self) -> Self {
        if self.contains_zero() {
            return Self::ENTIRE;
        }

        let lower = outward_down(1.0 / self.upper);
        let upper = outward_up(1.0 / self.lower);
        // A reciprocal of an interval not containing zero is monotone
        // decreasing, for both positive and negative denominator intervals.
        Self { lower, upper }
    }
}

fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value.is_sign_negative() {
        f64::from_bits(bits - 1)
    } else {
        f64::from_bits(bits + 1)
    }
}

fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits((1_u64 << 63) | 1);
    }
    let bits = value.to_bits();
    if value.is_sign_negative() {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    }
}

fn outward_down(value: f64) -> f64 {
    if value.is_nan() {
        f64::NEG_INFINITY
    } else {
        next_down(value)
    }
}

fn outward_up(value: f64) -> f64 {
    if value.is_nan() {
        f64::INFINITY
    } else {
        next_up(value)
    }
}

impl Add for Interval {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let lower = if self.lower == 0.0 {
            rhs.lower
        } else if rhs.lower == 0.0 {
            self.lower
        } else {
            outward_down(self.lower + rhs.lower)
        };
        let upper = if self.upper == 0.0 {
            rhs.upper
        } else if rhs.upper == 0.0 {
            self.upper
        } else {
            outward_up(self.upper + rhs.upper)
        };
        Self { lower, upper }
    }
}

impl Neg for Interval {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            lower: -self.upper,
            upper: -self.lower,
        }
    }
}

impl Sub for Interval {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        subtract_intervals(self, rhs)
    }
}

fn subtract_intervals(left: Interval, right: Interval) -> Interval {
    left + (-right)
}

impl Mul for Interval {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        if self.is_point_zero() || rhs.is_point_zero() {
            return Self::ZERO;
        }

        let products = [
            self.lower * rhs.lower,
            self.lower * rhs.upper,
            self.upper * rhs.lower,
            self.upper * rhs.upper,
        ];
        if products.iter().any(|value| value.is_nan()) {
            // Indeterminate extended-real endpoint products (for example
            // zero times infinity) lose useful information, so return R.
            return Self::ENTIRE;
        }

        let lower = products.iter().copied().fold(f64::INFINITY, f64::min);
        let upper = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Self {
            lower: outward_down(lower),
            upper: outward_up(upper),
        }
    }
}

impl Div for Interval {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        divide_intervals(self, rhs)
    }
}

fn divide_intervals(numerator: Interval, denominator: Interval) -> Interval {
    if denominator.contains_zero() {
        return Interval::ENTIRE;
    }
    numerator * denominator.reciprocal()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntervalDual<const N: usize> {
    value: Interval,
    gradient: [Interval; N],
}

impl<const N: usize> IntervalDual<N> {
    pub const fn constant(value: Interval) -> Self {
        Self {
            value,
            gradient: [Interval::ZERO; N],
        }
    }

    fn variable(value: Interval, dimension: usize) -> Self {
        Self {
            value,
            gradient: std::array::from_fn(|index| {
                if index == dimension {
                    Interval::ONE
                } else {
                    Interval::ZERO
                }
            }),
        }
    }

    pub const fn value(self) -> Interval {
        self.value
    }

    pub const fn gradient(self) -> [Interval; N] {
        self.gradient
    }

    /// Enclose x² and its derivative 2x without interval self-dependency.
    pub fn square(self) -> Self {
        let two = Interval::point(2.0).expect("finite constant");
        Self {
            value: self.value.square(),
            gradient: std::array::from_fn(|index| two * self.value * self.gradient[index]),
        }
    }

    pub fn sqrt(self) -> Result<Self, IntervalError> {
        let value = self.value.sqrt()?;
        let gradient = if self.value.lower <= 0.0 {
            // The derivative of sqrt(x) is unbounded at x=0. If the input box
            // touches zero, no finite derivative enclosure is issued.
            [Interval::ENTIRE; N]
        } else {
            let denominator = value * Interval::point(2.0).expect("finite constant");
            std::array::from_fn(|index| self.gradient[index] / denominator)
        };
        Ok(Self { value, gradient })
    }

    pub fn powi(self, exponent: i32) -> Self {
        if exponent == 0 {
            return Self::constant(Interval::ONE);
        }
        if exponent == 2 {
            return self.square();
        }
        if exponent == -2 {
            let square = self.square();
            return Self::constant(Interval::ONE) / square;
        }
        let mut power = self;
        let mut remaining = exponent.unsigned_abs();
        let mut result = Self::constant(Interval::ONE);
        while remaining > 0 {
            if remaining & 1 == 1 {
                result = result * power;
            }
            remaining >>= 1;
            if remaining > 0 {
                power = power.square();
            }
        }
        if exponent < 0 {
            Self::constant(Interval::ONE) / result
        } else {
            result
        }
    }
}

impl<const N: usize> Add for IntervalDual<N> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value + rhs.value,
            gradient: std::array::from_fn(|index| self.gradient[index] + rhs.gradient[index]),
        }
    }
}

impl<const N: usize> Neg for IntervalDual<N> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            value: -self.value,
            gradient: std::array::from_fn(|index| -self.gradient[index]),
        }
    }
}

impl<const N: usize> Sub for IntervalDual<N> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        subtract_duals(self, rhs)
    }
}

fn subtract_duals<const N: usize>(
    left: IntervalDual<N>,
    right: IntervalDual<N>,
) -> IntervalDual<N> {
    left + (-right)
}

impl<const N: usize> Mul for IntervalDual<N> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value * rhs.value,
            gradient: std::array::from_fn(|index| product_rule_derivative(self, rhs, index)),
        }
    }
}

fn product_rule_derivative<const N: usize>(
    left: IntervalDual<N>,
    right: IntervalDual<N>,
    index: usize,
) -> Interval {
    left.gradient[index] * right.value + left.value * right.gradient[index]
}

impl<const N: usize> Div for IntervalDual<N> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        divide_duals(self, rhs)
    }
}

fn divide_duals<const N: usize>(
    numerator: IntervalDual<N>,
    denominator: IntervalDual<N>,
) -> IntervalDual<N> {
    let denominator_squared = denominator.value * denominator.value;
    IntervalDual {
        value: divide_intervals(numerator.value, denominator.value),
        gradient: std::array::from_fn(|index| {
            let top_left = numerator.gradient[index] * denominator.value;
            let top_right = numerator.value * denominator.gradient[index];
            divide_intervals(top_left - top_right, denominator_squared)
        }),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterDomain<const N: usize> {
    pub record_id: String,
    pub expression_id: String,
    pub cell_id: String,
    pub input_sha256: String,
    pub source_revision: String,
    pub implementation_id: String,
    pub artifact_sha256: String,
    pub environment_sha256: String,
    pub parameter_ids: [String; N],
    pub parameter_units: [String; N],
    pub parameter_bounds: [Interval; N],
    pub output_unit: String,
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub model_set_id: String,
    pub solver_run_id: String,
    pub geometry_sha256: String,
    pub material_sha256: String,
    pub requirement_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueEnclosureMethod {
    NaturalIntervalExtension,
    NaturalAndMeanValueIntersection,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedDerivativeEnclosure<const N: usize> {
    pub record_id: String,
    pub expression_id: String,
    pub cell_id: String,
    pub input_sha256: String,
    pub source_revision: String,
    pub implementation_id: String,
    pub artifact_sha256: String,
    pub environment_sha256: String,
    pub parameter_ids: [String; N],
    pub parameter_units: [String; N],
    pub parameter_bounds: [Interval; N],
    pub natural_value_enclosure: Interval,
    pub value_enclosure: Interval,
    pub value_enclosure_method: ValueEnclosureMethod,
    pub output_unit: String,
    pub derivative_intervals: [Interval; N],
    pub derivative_abs_supremum: [f64; N],
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub model_set_id: String,
    pub solver_run_id: String,
    pub geometry_sha256: String,
    pub material_sha256: String,
    pub requirement_id: String,
    pub arithmetic_contract: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntervalAdError {
    EmptyParameterDomain,
    InvalidIdentity,
    DuplicateParameterId,
    InvalidSha256 { field: &'static str },
    NonFiniteParameterBound { index: usize },
    UnboundedValueEnclosure,
    UnboundedDerivativeEnclosure { index: usize },
    SquareRootOutsideRealDomain,
    InvalidIntervalOperation,
    InconsistentValueEnclosures,
}

impl From<IntervalError> for IntervalAdError {
    fn from(error: IntervalError) -> Self {
        match error {
            IntervalError::SquareRootOutsideRealDomain => Self::SquareRootOutsideRealDomain,
            IntervalError::NaNEndpoint | IntervalError::ReversedEndpoints => {
                Self::InvalidIntervalOperation
            }
        }
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_digest(value: &str, field: &'static str) -> Result<(), IntervalAdError> {
    if valid_sha256(value) {
        Ok(())
    } else {
        Err(IntervalAdError::InvalidSha256 { field })
    }
}

fn validate_domain<const N: usize>(domain: &ParameterDomain<N>) -> Result<(), IntervalAdError> {
    if N == 0 {
        return Err(IntervalAdError::EmptyParameterDomain);
    }

    let identities = [
        domain.record_id.as_str(),
        domain.expression_id.as_str(),
        domain.cell_id.as_str(),
        domain.source_revision.as_str(),
        domain.implementation_id.as_str(),
        domain.output_unit.as_str(),
        domain.coordinate_frame_id.as_str(),
        domain.operating_condition_id.as_str(),
        domain.model_set_id.as_str(),
        domain.solver_run_id.as_str(),
        domain.requirement_id.as_str(),
    ];
    if identities.into_iter().any(|value| !nonempty(value))
        || domain.parameter_ids.iter().any(|value| !nonempty(value))
        || domain.parameter_units.iter().any(|value| !nonempty(value))
    {
        return Err(IntervalAdError::InvalidIdentity);
    }

    let mut unique_parameter_ids = HashSet::with_capacity(N);
    if domain
        .parameter_ids
        .iter()
        .any(|parameter_id| !unique_parameter_ids.insert(parameter_id.as_str()))
    {
        return Err(IntervalAdError::DuplicateParameterId);
    }

    validate_digest(&domain.input_sha256, "input_sha256")?;
    validate_digest(&domain.artifact_sha256, "artifact_sha256")?;
    validate_digest(&domain.environment_sha256, "environment_sha256")?;
    validate_digest(&domain.geometry_sha256, "geometry_sha256")?;
    validate_digest(&domain.material_sha256, "material_sha256")?;

    for (index, bounds) in domain.parameter_bounds.iter().enumerate() {
        if !bounds.is_finite() {
            return Err(IntervalAdError::NonFiniteParameterBound { index });
        }
    }
    Ok(())
}

/// Evaluate an algebraic/rational expression over a Cartesian interval box.
///
/// The closure must use the supported IntervalDual operations only. Returning
/// a value or derivative interval that is unbounded prevents a finite
/// derivative certificate from being produced.
pub fn certify_expression<const N: usize, F>(
    domain: &ParameterDomain<N>,
    evaluate: F,
) -> Result<ValidatedDerivativeEnclosure<N>, IntervalAdError>
where
    F: Fn([IntervalDual<N>; N]) -> Result<IntervalDual<N>, IntervalAdError>,
{
    validate_domain(domain)?;

    let variables =
        std::array::from_fn(|index| IntervalDual::variable(domain.parameter_bounds[index], index));
    let output = evaluate(variables)?;

    if !output.value.is_finite() {
        return Err(IntervalAdError::UnboundedValueEnclosure);
    }

    let derivative_intervals = output.gradient;
    let derivative_abs_supremum: [Option<f64>; N] =
        std::array::from_fn(|index| derivative_intervals[index].abs_upper_bound());
    let mut finite_suprema = [0.0; N];
    for (index, (supremum, output)) in derivative_abs_supremum
        .iter()
        .zip(finite_suprema.iter_mut())
        .enumerate()
    {
        let Some(supremum) = supremum else {
            return Err(IntervalAdError::UnboundedDerivativeEnclosure { index });
        };
        *output = *supremum;
    }

    Ok(ValidatedDerivativeEnclosure {
        record_id: domain.record_id.clone(),
        expression_id: domain.expression_id.clone(),
        cell_id: domain.cell_id.clone(),
        input_sha256: domain.input_sha256.clone(),
        source_revision: domain.source_revision.clone(),
        implementation_id: domain.implementation_id.clone(),
        artifact_sha256: domain.artifact_sha256.clone(),
        environment_sha256: domain.environment_sha256.clone(),
        parameter_ids: domain.parameter_ids.clone(),
        parameter_units: domain.parameter_units.clone(),
        parameter_bounds: domain.parameter_bounds,
        natural_value_enclosure: output.value,
        value_enclosure: output.value,
        value_enclosure_method: ValueEnclosureMethod::NaturalIntervalExtension,
        output_unit: domain.output_unit.clone(),
        derivative_intervals,
        derivative_abs_supremum: finite_suprema,
        coordinate_frame_id: domain.coordinate_frame_id.clone(),
        operating_condition_id: domain.operating_condition_id.clone(),
        model_set_id: domain.model_set_id.clone(),
        solver_run_id: domain.solver_run_id.clone(),
        geometry_sha256: domain.geometry_sha256.clone(),
        material_sha256: domain.material_sha256.clone(),
        requirement_id: domain.requirement_id.clone(),
        arithmetic_contract: "binary64-nearest-plus-one-ulp-outward-v0.1".to_owned(),
    })
}

fn interval_midpoint(value: Interval, index: usize) -> Result<f64, IntervalAdError> {
    let lower = value.lower();
    let upper = value.upper();
    let midpoint = if lower.is_sign_negative() == upper.is_sign_negative() {
        lower + (upper - lower) * 0.5
    } else {
        lower * 0.5 + upper * 0.5
    };
    if !midpoint.is_finite() || midpoint < lower || midpoint > upper {
        return Err(IntervalAdError::NonFiniteParameterBound { index });
    }
    Ok(midpoint)
}

/// Tighten a natural interval extension with the multivariate mean-value form.
///
/// For every point x in the declared convex box X, the integral mean-value
/// theorem gives f(x) in f(c) + sum_i D_i(X) * (X_i-c_i), where c is the
/// representable box centre and D_i(X) encloses the complete partial
/// derivative over X. Intersecting this with the natural extension preserves
/// enclosure validity while reducing dependency overestimation when possible.
pub fn certify_expression_centered<const N: usize, F>(
    domain: &ParameterDomain<N>,
    evaluate: F,
) -> Result<ValidatedDerivativeEnclosure<N>, IntervalAdError>
where
    F: Fn([IntervalDual<N>; N]) -> Result<IntervalDual<N>, IntervalAdError>,
{
    let mut result = certify_expression(domain, &evaluate)?;
    let mut centres = [0.0; N];
    for (index, centre) in centres.iter_mut().enumerate() {
        *centre = interval_midpoint(domain.parameter_bounds[index], index)?;
    }
    let centre_variables = std::array::from_fn(|index| {
        IntervalDual::constant(
            Interval::point(centres[index]).expect("finite representable centre"),
        )
    });
    let centre_value = evaluate(centre_variables)?.value();
    if !centre_value.is_finite() {
        return Err(IntervalAdError::UnboundedValueEnclosure);
    }

    let mut mean_value_enclosure = centre_value;
    for ((bounds, derivative), centre) in domain
        .parameter_bounds
        .iter()
        .zip(result.derivative_intervals.iter())
        .zip(centres.iter())
    {
        let centre_interval = Interval::point(*centre).map_err(IntervalAdError::from)?;
        let displacement = *bounds - centre_interval;
        mean_value_enclosure = mean_value_enclosure + *derivative * displacement;
    }
    if !mean_value_enclosure.is_finite() {
        return Err(IntervalAdError::UnboundedValueEnclosure);
    }

    let intersection = Interval::new(
        result
            .natural_value_enclosure
            .lower()
            .max(mean_value_enclosure.lower()),
        result
            .natural_value_enclosure
            .upper()
            .min(mean_value_enclosure.upper()),
    )
    .map_err(|_| IntervalAdError::InconsistentValueEnclosures)?;

    result.value_enclosure = intersection;
    result.value_enclosure_method = ValueEnclosureMethod::NaturalAndMeanValueIntersection;
    Ok(result)
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialDerivativeSupremumEvidence {
    pub source_record_id: String,
    pub source_revision: String,
    pub implementation_id: String,
    pub artifact_sha256: String,
    pub environment_sha256: String,
    pub cell_id: String,
    pub axis_parameter_ids: [String; 3],
    pub axis_bounds_m: [Interval; 3],
    pub derivative_abs_supremum_by_axis: [f64; 3],
    pub output_unit: String,
    pub derivative_unit: String,
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub model_set_id: String,
    pub solver_run_id: String,
    pub geometry_sha256: String,
    pub material_sha256: String,
    pub requirement_id: String,
    pub arithmetic_contract: String,
    pub value_enclosure_method: ValueEnclosureMethod,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpatialMappingError {
    InvalidExpectedIdentity,
    CellIdentityMismatch,
    RequirementIdentityMismatch,
    DuplicateSpatialAxisId,
    MissingSpatialAxisParameter { axis: usize },
    SpatialAxisUnitMismatch { axis: usize },
    UnboundedSpatialDerivative { axis: usize },
}

/// Map a validated scalar functional-margin expression to the spatial
/// certifier's XYZ derivative-supremum contract.
///
/// The record's parameter order may differ from XYZ; axes are explicitly
/// selected by parameter ID. This adapter does not create a centre margin
/// observation: the spatial certifier still requires an independent sample
/// and uncertainty interval at the exact cell centre. The value enclosure
/// is never converted to a centre-margin interval.
pub fn map_spatial_derivative_suprema<const N: usize>(
    record: &ValidatedDerivativeEnclosure<N>,
    expected_cell_id: &str,
    expected_requirement_id: &str,
    spatial_axis_parameter_ids: [&str; 3],
    length_unit: &str,
) -> Result<SpatialDerivativeSupremumEvidence, SpatialMappingError> {
    if expected_cell_id.trim().is_empty()
        || expected_requirement_id.trim().is_empty()
        || length_unit != "metre"
        || record.output_unit.trim().is_empty()
    {
        return Err(SpatialMappingError::InvalidExpectedIdentity);
    }
    if record.cell_id != expected_cell_id {
        return Err(SpatialMappingError::CellIdentityMismatch);
    }
    if record.requirement_id != expected_requirement_id {
        return Err(SpatialMappingError::RequirementIdentityMismatch);
    }
    if spatial_axis_parameter_ids
        .iter()
        .any(|parameter_id| parameter_id.trim().is_empty())
        || spatial_axis_parameter_ids[0] == spatial_axis_parameter_ids[1]
        || spatial_axis_parameter_ids[0] == spatial_axis_parameter_ids[2]
        || spatial_axis_parameter_ids[1] == spatial_axis_parameter_ids[2]
    {
        return Err(SpatialMappingError::DuplicateSpatialAxisId);
    }

    let mut axis_bounds_m = [Interval::ZERO; 3];
    let mut derivative_abs_supremum_by_axis = [0.0; 3];
    let mut axis_ids = std::array::from_fn(|axis| spatial_axis_parameter_ids[axis].to_owned());

    for axis in 0..3 {
        let parameter_id = spatial_axis_parameter_ids[axis];
        let Some(parameter_index) = record
            .parameter_ids
            .iter()
            .position(|candidate| candidate == parameter_id)
        else {
            return Err(SpatialMappingError::MissingSpatialAxisParameter { axis });
        };
        if record.parameter_units[parameter_index] != length_unit {
            return Err(SpatialMappingError::SpatialAxisUnitMismatch { axis });
        }
        let supremum = record.derivative_abs_supremum[parameter_index];
        if !supremum.is_finite() || supremum < 0.0 {
            return Err(SpatialMappingError::UnboundedSpatialDerivative { axis });
        }
        axis_bounds_m[axis] = record.parameter_bounds[parameter_index];
        derivative_abs_supremum_by_axis[axis] = supremum;
        axis_ids[axis] = parameter_id.to_owned();
    }

    Ok(SpatialDerivativeSupremumEvidence {
        source_record_id: record.record_id.clone(),
        source_revision: record.source_revision.clone(),
        implementation_id: record.implementation_id.clone(),
        artifact_sha256: record.artifact_sha256.clone(),
        environment_sha256: record.environment_sha256.clone(),
        cell_id: record.cell_id.clone(),
        axis_parameter_ids: axis_ids,
        axis_bounds_m,
        derivative_abs_supremum_by_axis,
        output_unit: record.output_unit.clone(),
        derivative_unit: format!("{}/{}", record.output_unit, length_unit),
        coordinate_frame_id: record.coordinate_frame_id.clone(),
        operating_condition_id: record.operating_condition_id.clone(),
        model_set_id: record.model_set_id.clone(),
        solver_run_id: record.solver_run_id.clone(),
        geometry_sha256: record.geometry_sha256.clone(),
        material_sha256: record.material_sha256.clone(),
        requirement_id: record.requirement_id.clone(),
        arithmetic_contract: record.arithmetic_contract.clone(),
        value_enclosure_method: record.value_enclosure_method,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn interval(lower: f64, upper: f64) -> Interval {
        Interval::new(lower, upper).expect("valid interval")
    }

    fn domain<const N: usize>(
        parameter_ids: [&str; N],
        parameter_bounds: [Interval; N],
    ) -> ParameterDomain<N> {
        ParameterDomain {
            record_id: "interval-ad-record-001".to_owned(),
            expression_id: "magnetic-test-expression".to_owned(),
            cell_id: "cell-001".to_owned(),
            input_sha256: DIGEST_A.to_owned(),
            source_revision: "test-source-v1".to_owned(),
            implementation_id: "magnetic-interval-ad-rust".to_owned(),
            artifact_sha256: DIGEST_B.to_owned(),
            environment_sha256: DIGEST_C.to_owned(),
            parameter_ids: parameter_ids.map(str::to_owned),
            parameter_units: std::array::from_fn(|_| "metre".to_owned()),
            parameter_bounds,
            output_unit: "dimensionless".to_owned(),
            coordinate_frame_id: "frame-001".to_owned(),
            operating_condition_id: "room-temperature".to_owned(),
            model_set_id: "model-set-001".to_owned(),
            solver_run_id: "solver-run-001".to_owned(),
            geometry_sha256: DIGEST_A.to_owned(),
            material_sha256: DIGEST_B.to_owned(),
            requirement_id: "functional-margin-001".to_owned(),
        }
    }

    fn spatial_domain() -> ParameterDomain<3> {
        let mut scope = domain(
            ["z", "x", "y"],
            [
                interval(-0.3, 0.3),
                interval(-1.0, 1.0),
                interval(-0.1, 0.1),
            ],
        );
        scope.cell_id = "spatial-cell-xyz".to_owned();
        scope.output_unit = "radian".to_owned();
        scope.requirement_id = "minimum-phase-margin".to_owned();
        scope
    }

    fn spatial_expression(
        [z, x, y]: [IntervalDual<3>; 3],
    ) -> Result<IntervalDual<3>, IntervalAdError> {
        let four = IntervalDual::constant(interval(4.0, 4.0));
        let two = IntervalDual::constant(interval(2.0, 2.0));
        let three = IntervalDual::constant(interval(3.0, 3.0));
        Ok(four * x + two * y + three * z)
    }

    #[test]
    fn interval_derivatives_map_to_spatial_xyz_by_parameter_identity() {
        let scope = spatial_domain();
        let enclosure = certify_expression_centered(&scope, spatial_expression)
            .expect("valid spatial functional expression");
        let mapped = map_spatial_derivative_suprema(
            &enclosure,
            "spatial-cell-xyz",
            "minimum-phase-margin",
            ["x", "y", "z"],
            "metre",
        )
        .expect("axis-aligned spatial handoff");

        assert_eq!(
            mapped.axis_parameter_ids,
            ["x".to_owned(), "y".to_owned(), "z".to_owned()]
        );
        assert_eq!(mapped.axis_bounds_m[0], interval(-1.0, 1.0));
        assert_eq!(mapped.axis_bounds_m[1], interval(-0.1, 0.1));
        assert_eq!(mapped.axis_bounds_m[2], interval(-0.3, 0.3));
        assert!(mapped.derivative_abs_supremum_by_axis[0] >= 4.0);
        assert!(mapped.derivative_abs_supremum_by_axis[1] >= 2.0);
        assert!(mapped.derivative_abs_supremum_by_axis[2] >= 3.0);
        assert_eq!(mapped.derivative_unit, "radian/metre");
        assert_eq!(mapped.source_record_id, enclosure.record_id);
        assert_eq!(mapped.cell_id, "spatial-cell-xyz");
    }

    #[test]
    fn spatial_handoff_rejects_a_different_cell_identity() {
        let scope = spatial_domain();
        let enclosure = certify_expression_centered(&scope, spatial_expression)
            .expect("valid spatial functional expression");
        assert_eq!(
            map_spatial_derivative_suprema(
                &enclosure,
                "another-cell",
                "minimum-phase-margin",
                ["x", "y", "z"],
                "metre",
            )
            .expect_err("cell identity must match"),
            SpatialMappingError::CellIdentityMismatch
        );
    }

    #[test]
    fn spatial_handoff_rejects_a_different_requirement_identity() {
        let scope = spatial_domain();
        let enclosure = certify_expression_centered(&scope, spatial_expression)
            .expect("valid spatial functional expression");
        assert_eq!(
            map_spatial_derivative_suprema(
                &enclosure,
                "spatial-cell-xyz",
                "different-requirement",
                ["x", "y", "z"],
                "metre",
            )
            .expect_err("requirement identity must match"),
            SpatialMappingError::RequirementIdentityMismatch
        );
    }

    #[test]
    fn spatial_handoff_rejects_missing_or_duplicate_axes() {
        let scope = spatial_domain();
        let enclosure = certify_expression_centered(&scope, spatial_expression)
            .expect("valid spatial functional expression");
        assert_eq!(
            map_spatial_derivative_suprema(
                &enclosure,
                "spatial-cell-xyz",
                "minimum-phase-margin",
                ["x", "y", "missing"],
                "metre",
            )
            .expect_err("missing z axis"),
            SpatialMappingError::MissingSpatialAxisParameter { axis: 2 }
        );
        assert_eq!(
            map_spatial_derivative_suprema(
                &enclosure,
                "spatial-cell-xyz",
                "minimum-phase-margin",
                ["x", "x", "z"],
                "metre",
            )
            .expect_err("duplicate axis identity"),
            SpatialMappingError::DuplicateSpatialAxisId
        );
    }

    #[test]
    fn spatial_handoff_rejects_non_metre_target_units() {
        let scope = spatial_domain();
        let enclosure = certify_expression_centered(&scope, spatial_expression)
            .expect("valid spatial functional expression");
        assert_eq!(
            map_spatial_derivative_suprema(
                &enclosure,
                "spatial-cell-xyz",
                "minimum-phase-margin",
                ["x", "y", "z"],
                "millimetre",
            )
            .expect_err("the spatial certifier's axis bounds are metres"),
            SpatialMappingError::InvalidExpectedIdentity
        );
    }

    #[test]
    fn spatial_handoff_rejects_non_metre_axis_units() {
        let mut scope = spatial_domain();
        scope.parameter_units[1] = "millimetre".to_owned();
        let enclosure = certify_expression_centered(&scope, spatial_expression)
            .expect("numeric expression still evaluates");
        assert_eq!(
            map_spatial_derivative_suprema(
                &enclosure,
                "spatial-cell-xyz",
                "minimum-phase-margin",
                ["x", "y", "z"],
                "metre",
            )
            .expect_err("implicit unit conversion is forbidden"),
            SpatialMappingError::SpatialAxisUnitMismatch { axis: 0 }
        );
    }

    #[test]
    fn spatial_mapping_does_not_create_a_centre_measurement_margin() {
        let scope = spatial_domain();
        let mut enclosure = certify_expression_centered(&scope, spatial_expression)
            .expect("valid spatial functional expression");
        enclosure.value_enclosure = interval(1000.0, 2000.0);
        let mapped = map_spatial_derivative_suprema(
            &enclosure,
            "spatial-cell-xyz",
            "minimum-phase-margin",
            ["x", "y", "z"],
            "metre",
        )
        .expect("derivative map is independent of the centre sample");
        assert_eq!(mapped.derivative_abs_supremum_by_axis.len(), 3);
        assert_eq!(mapped.output_unit, "radian");
    }

    #[test]
    fn centered_form_tightens_a_repeated_variable_dependency() {
        let scope = domain(["x"], [interval(-1.0, 1.0)]);
        let natural =
            certify_expression(&scope, |[x]| Ok(x - x)).expect("natural interval extension");
        let centered = certify_expression_centered(&scope, |[x]| Ok(x - x))
            .expect("centered mean-value enclosure");

        let natural_width = natural.value_enclosure.upper() - natural.value_enclosure.lower();
        let centered_width = centered.value_enclosure.upper() - centered.value_enclosure.lower();

        assert!(natural.value_enclosure.lower() <= 0.0);
        assert!(natural.value_enclosure.upper() >= 0.0);
        assert!(centered.value_enclosure.lower() <= 0.0);
        assert!(centered.value_enclosure.upper() >= 0.0);
        assert!(centered_width < natural_width);
        assert_eq!(
            centered.value_enclosure_method,
            ValueEnclosureMethod::NaturalAndMeanValueIntersection
        );
        assert_eq!(centered.natural_value_enclosure, natural.value_enclosure);
    }

    #[test]
    fn centered_enclosure_never_exceeds_the_natural_extension() {
        let scope = domain(["x"], [interval(0.5, 2.0)]);
        let natural = certify_expression(&scope, |[x]| Ok(x.square() - x))
            .expect("natural interval extension");
        let centered = certify_expression_centered(&scope, |[x]| Ok(x.square() - x))
            .expect("centered enclosure");
        assert!(centered.value_enclosure.lower() >= natural.value_enclosure.lower());
        assert!(centered.value_enclosure.upper() <= natural.value_enclosure.upper());
    }

    #[test]
    fn interval_addition_encloses_the_exact_integer_result() {
        let result = interval(1.0, 2.0) + interval(3.0, 4.0);
        assert!(result.lower() <= 4.0);
        assert!(result.upper() >= 6.0);
    }

    #[test]
    fn square_of_zero_crossing_interval_is_nonnegative_and_tight() {
        let result = interval(-2.0, 3.0).square();
        assert!(result.lower() >= 0.0);
        assert!(result.lower() <= 0.0);
        assert!(result.upper() >= 9.0);
        assert!(result.upper() < 9.0 + 1.0e-12);
    }

    #[test]
    fn interval_dual_square_uses_the_exact_chain_rule_enclosure() {
        let input = IntervalDual::<1>::variable(interval(-2.0, 3.0), 0);
        let result = input.square();
        assert!(result.value().lower() >= 0.0);
        assert!(result.value().upper() >= 9.0);
        assert!(result.gradient()[0].lower() <= -4.0);
        assert!(result.gradient()[0].upper() >= 6.0);
    }

    #[test]
    fn interval_multiplication_handles_all_sign_combinations() {
        let cases = [
            (interval(1.0, 2.0), interval(3.0, 4.0), 3.0, 8.0),
            (interval(-2.0, -1.0), interval(3.0, 4.0), -8.0, -3.0),
            (interval(-2.0, 1.0), interval(3.0, 4.0), -8.0, 4.0),
            (interval(-2.0, 1.0), interval(-3.0, 4.0), -8.0, 6.0),
        ];
        for (left, right, expected_lower, expected_upper) in cases {
            let result = left * right;
            assert!(result.lower() <= expected_lower);
            assert!(result.upper() >= expected_upper);
        }
    }

    #[test]
    fn division_by_zero_crossing_interval_is_unbounded() {
        let result = interval(1.0, 2.0) / interval(-1.0, 1.0);
        assert_eq!(result, Interval::ENTIRE);
    }

    #[test]
    fn zero_numerator_does_not_hide_a_zero_crossing_denominator() {
        let result = Interval::ZERO / interval(-1.0, 1.0);
        assert_eq!(result, Interval::ENTIRE);
    }

    #[test]
    fn reciprocal_of_negative_interval_preserves_order() {
        let result = interval(-4.0, -2.0).reciprocal();
        assert!(result.lower() <= -0.5);
        assert!(result.upper() >= -0.25);
        assert!(result.upper() < 0.0);
    }

    #[test]
    fn square_root_rejects_a_wholly_negative_interval() {
        assert_eq!(
            interval(-4.0, -1.0)
                .sqrt()
                .expect_err("outside real domain"),
            IntervalError::SquareRootOutsideRealDomain
        );
    }

    #[test]
    fn square_root_rejects_a_box_that_straddles_the_negative_domain() {
        assert_eq!(
            interval(-0.1, 4.0)
                .sqrt()
                .expect_err("complete box is not in the square-root domain"),
            IntervalError::SquareRootOutsideRealDomain
        );
    }

    #[test]
    fn square_root_encloses_a_nonnegative_interval() {
        let result = interval(4.0, 9.0).sqrt().expect("valid root");
        assert!(result.lower() <= 2.0);
        assert!(result.upper() >= 3.0);
    }

    #[test]
    fn quadratic_interval_ad_encloses_analytic_derivative_range() {
        let scope = domain(["x"], [interval(1.0, 2.0)]);
        let result =
            certify_expression(&scope, |[x]| Ok(x * x)).expect("bounded quadratic expression");
        assert!(result.derivative_intervals[0].lower() <= 2.0);
        assert!(result.derivative_intervals[0].upper() >= 4.0);
        assert!(result.derivative_abs_supremum[0] >= 4.0);
    }

    #[test]
    fn bilinear_interval_ad_encloses_both_partial_derivatives() {
        let scope = domain(["y", "z"], [interval(2.0, 3.0), interval(-2.0, -1.0)]);
        let result =
            certify_expression(&scope, |[y, z]| Ok(y * z)).expect("bounded bilinear expression");
        assert!(result.derivative_intervals[0].lower() <= -2.0);
        assert!(result.derivative_intervals[0].upper() >= -1.0);
        assert!(result.derivative_intervals[1].lower() <= 2.0);
        assert!(result.derivative_intervals[1].upper() >= 3.0);
        assert!(result.derivative_abs_supremum[0] >= 2.0);
        assert!(result.derivative_abs_supremum[1] >= 3.0);
    }

    #[test]
    fn rational_expression_gets_finite_derivative_enclosures_away_from_pole() {
        let scope = domain(["x"], [interval(1.0, 2.0)]);
        let result = certify_expression(&scope, |[x]| {
            let one = IntervalDual::constant(Interval::ONE);
            Ok(one / (x * x + one))
        })
        .expect("denominator is separated from zero");
        assert!(result.derivative_intervals[0].is_finite());
        assert!(result.derivative_abs_supremum[0].is_finite());
    }

    #[test]
    fn quadratic_derivative_is_enclosed_when_input_crosses_zero() {
        let scope = domain(["x"], [interval(-2.0, 3.0)]);
        let result =
            certify_expression(&scope, |[x]| Ok(x * x)).expect("bounded quadratic across zero");
        assert!(result.derivative_intervals[0].lower() <= -4.0);
        assert!(result.derivative_intervals[0].upper() >= 6.0);
        assert!(result.derivative_abs_supremum[0] >= 6.0);
    }

    #[test]
    fn derivative_interval_widens_with_a_wider_quadratic_input_box() {
        let narrow = domain(["x"], [interval(1.0, 1.5)]);
        let wide = domain(["x"], [interval(1.0, 2.0)]);
        let narrow_result = certify_expression(&narrow, |[x]| Ok(x * x)).expect("narrow interval");
        let wide_result = certify_expression(&wide, |[x]| Ok(x * x)).expect("wide interval");
        assert!(wide_result.derivative_abs_supremum[0] >= narrow_result.derivative_abs_supremum[0]);
    }

    #[test]
    fn denominator_crossing_zero_cannot_emit_a_finite_derivative_bound() {
        let scope = domain(["x"], [interval(-0.1, 0.1)]);
        let result =
            certify_expression(&scope, |[x]| Ok(IntervalDual::constant(Interval::ONE) / x));
        assert_eq!(
            result.expect_err("singular interval has no finite derivative bound"),
            IntervalAdError::UnboundedValueEnclosure
        );
    }

    #[test]
    fn square_root_box_touching_zero_has_unbounded_derivative() {
        let scope = domain(["x"], [interval(0.0, 1.0)]);
        let result = certify_expression(&scope, |[x]| {
            x.sqrt()
                .map_err(|_| IntervalAdError::SquareRootOutsideRealDomain)
        });
        assert_eq!(
            result.expect_err("derivative is unbounded at zero"),
            IntervalAdError::UnboundedDerivativeEnclosure { index: 0 }
        );
    }

    #[test]
    fn point_dipole_bz_expression_has_finite_cell_wide_derivative_bounds() {
        let scope = domain(
            ["x", "y", "z"],
            [
                interval(0.9, 1.1),
                interval(-0.05, 0.05),
                interval(0.2, 0.3),
            ],
        );

        // Dipole moment is aligned with z. The common physical constant is
        // omitted, leaving the dimensionless spatial dependence:
        // Bz = 3*z^2/r^5 - 1/r^3.
        let result = certify_expression(&scope, |[x, y, z]| {
            let r2 = x.square() + y.square() + z.square();
            let r = r2.sqrt()?;
            let r3 = r2 * r;
            let r5 = r3 * r2;
            let one = IntervalDual::constant(Interval::ONE);
            let three = IntervalDual::constant(interval(3.0, 3.0));
            let bz = three * z.square() / r5 - one / r3;
            Ok(bz)
        })
        .expect("box does not contain the dipole singularity");

        assert!(result.value_enclosure.is_finite());
        assert!(
            result
                .derivative_abs_supremum
                .iter()
                .all(|value| value.is_finite())
        );
        assert!(
            result
                .derivative_abs_supremum
                .iter()
                .all(|value| *value > 0.0)
        );
        assert_eq!(result.cell_id, "cell-001");
        assert_eq!(
            result.parameter_ids,
            ["x".to_owned(), "y".to_owned(), "z".to_owned()]
        );
    }

    #[test]
    fn invalid_parameter_digest_fails_closed() {
        let mut scope = domain(["x"], [interval(1.0, 2.0)]);
        scope.input_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            certify_expression(&scope, |[x]| Ok(x * x)).expect_err("invalid input digest"),
            IntervalAdError::InvalidSha256 {
                field: "input_sha256"
            }
        );
    }

    #[test]
    fn duplicate_parameter_ids_are_rejected() {
        let scope = domain(["x", "x"], [interval(1.0, 2.0), interval(2.0, 3.0)]);
        assert_eq!(
            certify_expression(&scope, |[x, y]| Ok(x + y))
                .expect_err("parameter ordering must be unambiguous"),
            IntervalAdError::DuplicateParameterId
        );
    }

    #[test]
    fn unbounded_parameter_boxes_are_rejected() {
        let scope = domain(["x"], [Interval::ENTIRE]);
        assert_eq!(
            certify_expression(&scope, |[x]| Ok(x * x)).expect_err("input domain must be bounded"),
            IntervalAdError::NonFiniteParameterBound { index: 0 }
        );
    }

    #[test]
    fn one_expression_can_be_replayed_over_multiple_parameter_boxes() {
        let narrow = domain(["x"], [interval(1.0, 2.0)]);
        let wide = domain(["x"], [interval(2.0, 3.0)]);
        let expression = |[x]: [IntervalDual<1>; 1]| Ok(x.square());
        let first = certify_expression(&narrow, expression).expect("first box");
        let second = certify_expression(&wide, expression).expect("second box");
        assert!(first.derivative_intervals[0].lower() <= 2.0);
        assert!(second.derivative_intervals[0].upper() >= 6.0);
        assert_ne!(first.parameter_bounds, second.parameter_bounds);
    }

    #[test]
    fn constant_expression_has_zero_derivative_supremum() {
        let scope = domain(["x"], [interval(-2.0, 2.0)]);
        let constant = IntervalDual::constant(interval(5.0, 5.0));
        let result = certify_expression(&scope, |_| Ok(constant)).expect("constant expression");
        assert_eq!(result.derivative_abs_supremum[0], 0.0);
    }

    #[test]
    fn signed_zero_is_preserved_and_recognized_as_zero() {
        let positive_zero = Interval::ZERO;
        let negative_zero = -positive_zero;
        assert!(negative_zero.contains_zero());
        assert_eq!(negative_zero.lower().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(negative_zero.upper().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(positive_zero + positive_zero, Interval::ZERO);
    }

    #[test]
    fn subnormal_product_is_enclosed_outward() {
        let exact = f64::MIN_POSITIVE * 0.5;
        let result = interval(f64::MIN_POSITIVE, f64::MIN_POSITIVE) * interval(0.5, 0.5);
        assert!(result.lower() <= exact);
        assert!(result.upper() >= exact);
        assert!(result.lower() < result.upper());
    }

    #[test]
    fn overflowed_value_cannot_be_promoted_to_a_finite_certificate() {
        let scope = domain(["x"], [interval(f64::MAX, f64::MAX)]);
        let result = certify_expression(&scope, |[x]| Ok(x * x));
        assert_eq!(
            result.expect_err("overflowed value is unbounded"),
            IntervalAdError::UnboundedValueEnclosure
        );
    }

    #[test]
    fn integer_power_uses_the_interval_chain_rule() {
        let scope = domain(["x"], [interval(1.0, 2.0)]);
        let result =
            certify_expression(&scope, |[x]| Ok(x.powi(3))).expect("bounded cubic expression");
        assert!(result.derivative_intervals[0].lower() <= 3.0);
        assert!(result.derivative_intervals[0].upper() >= 12.0);
        assert!(result.derivative_abs_supremum[0] >= 12.0);
    }
}
