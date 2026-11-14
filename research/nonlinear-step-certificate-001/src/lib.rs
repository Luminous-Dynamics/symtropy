// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Metric-whitened interval Krawczyk certificates for small nonlinear systems.
//!
//! The reference uses outward-rounded binary64 intervals. The frame metric is
//! bound to an exact integer coordinate map and exact integer Cholesky factor,
//! avoiding an unverified floating-point factorization in the proof inputs.
//! The nonlinear coefficients are binary64 values interpreted as exact dyadic
//! reals. This is a research reference, not yet a certificate for a production
//! multibody step.

#![forbid(unsafe_code)]
#![allow(clippy::needless_range_loop)]

use core::fmt;

pub type Vector2 = [f64; 2];
pub type Matrix2 = [[f64; 2]; 2];
pub type IntervalVector2 = [Interval; 2];
pub type IntervalMatrix2 = [[Interval; 2]; 2];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalErrorV1 {
    NonFiniteEndpoint,
    ReversedEndpoints,
    DivisionByZero,
    NegativeSquareRoot,
    ArithmeticOverflow,
}

impl fmt::Display for IntervalErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for IntervalErrorV1 {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        value
    } else if value == 0.0 {
        -f64::from_bits(1)
    } else if value > 0.0 {
        f64::from_bits(value.to_bits() - 1)
    } else {
        f64::from_bits(value.to_bits() + 1)
    }
}

fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        value
    } else if value == 0.0 {
        f64::from_bits(1)
    } else if value > 0.0 {
        f64::from_bits(value.to_bits() + 1)
    } else {
        f64::from_bits(value.to_bits() - 1)
    }
}

impl Interval {
    pub fn new(lower: f64, upper: f64) -> Result<Self, IntervalErrorV1> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(IntervalErrorV1::NonFiniteEndpoint);
        }
        if lower > upper {
            return Err(IntervalErrorV1::ReversedEndpoints);
        }
        Ok(Self { lower, upper })
    }

    pub fn point(value: f64) -> Result<Self, IntervalErrorV1> {
        Self::new(value, value)
    }

    pub fn contains_zero(self) -> bool {
        self.lower <= 0.0 && self.upper >= 0.0
    }

    pub fn midpoint(self) -> f64 {
        self.lower * 0.5 + self.upper * 0.5
    }

    pub fn abs_upper(self) -> f64 {
        next_up(self.lower.abs().max(self.upper.abs()))
    }

    pub fn add(self, rhs: Self) -> Result<Self, IntervalErrorV1> {
        Self::new(
            next_down(self.lower + rhs.lower),
            next_up(self.upper + rhs.upper),
        )
        .map_err(|_| IntervalErrorV1::ArithmeticOverflow)
    }

    pub fn sub(self, rhs: Self) -> Result<Self, IntervalErrorV1> {
        Self::new(
            next_down(self.lower - rhs.upper),
            next_up(self.upper - rhs.lower),
        )
        .map_err(|_| IntervalErrorV1::ArithmeticOverflow)
    }

    pub fn neg(self) -> Result<Self, IntervalErrorV1> {
        Self::new(-self.upper, -self.lower)
    }

    pub fn mul(self, rhs: Self) -> Result<Self, IntervalErrorV1> {
        let products = [
            self.lower * rhs.lower,
            self.lower * rhs.upper,
            self.upper * rhs.lower,
            self.upper * rhs.upper,
        ];
        if products.iter().any(|value| !value.is_finite()) {
            return Err(IntervalErrorV1::ArithmeticOverflow);
        }
        let lower = products.iter().copied().fold(f64::INFINITY, f64::min);
        let upper = products
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        Self::new(next_down(lower), next_up(upper))
            .map_err(|_| IntervalErrorV1::ArithmeticOverflow)
    }

    pub fn div(self, rhs: Self) -> Result<Self, IntervalErrorV1> {
        if rhs.contains_zero() {
            return Err(IntervalErrorV1::DivisionByZero);
        }
        let reciprocal_lower = 1.0 / rhs.upper;
        let reciprocal_upper = 1.0 / rhs.lower;
        if !reciprocal_lower.is_finite() || !reciprocal_upper.is_finite() {
            return Err(IntervalErrorV1::ArithmeticOverflow);
        }
        let reciprocal = Self::new(
            next_down(reciprocal_lower.min(reciprocal_upper)),
            next_up(reciprocal_lower.max(reciprocal_upper)),
        )
        .map_err(|_| IntervalErrorV1::ArithmeticOverflow)?;
        self.mul(reciprocal)
    }

    pub fn square(self) -> Result<Self, IntervalErrorV1> {
        if self.contains_zero() {
            let upper = self.lower.abs().max(self.upper.abs());
            let upper_square = upper * upper;
            if !upper_square.is_finite() {
                return Err(IntervalErrorV1::ArithmeticOverflow);
            }
            return Self::new(0.0, next_up(upper_square))
                .map_err(|_| IntervalErrorV1::ArithmeticOverflow);
        }
        self.mul(self)
    }

    pub fn sqrt(self) -> Result<Self, IntervalErrorV1> {
        if self.lower < 0.0 {
            return Err(IntervalErrorV1::NegativeSquareRoot);
        }
        let lower = if self.lower == 0.0 {
            0.0
        } else {
            next_down(self.lower.sqrt())
        };
        let upper = next_up(self.upper.sqrt());
        Self::new(lower, upper).map_err(|_| IntervalErrorV1::ArithmeticOverflow)
    }

    pub fn strictly_inside(self, outer: Self) -> bool {
        self.lower > outer.lower && self.upper < outer.upper
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertificateInputErrorV1 {
    InvalidIdentifier,
    InvalidProfile,
    InvalidSystem,
    InvalidFrame,
    BasisSingular,
    MetricFactorMismatch,
    IntegerArithmeticOverflow,
    ReceiptBindingMismatch,
    ReplayDidNotCertify,
    ReplayResultMismatch,
}

impl fmt::Display for CertificateInputErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CertificateInputErrorV1 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InconclusiveReasonV1 {
    SingularPreconditioner,
    PreconditionerOutsideProfile,
    ContractionNotEstablished,
    KrawczykImageNotIncluded,
    IntervalArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KrawczykProfileV1 {
    pub maximum_basis_entry: i64,
    pub maximum_preconditioner_inf_norm: f64,
    pub maximum_contraction_bound: f64,
    pub minimum_box_radius: f64,
    pub maximum_box_radius: f64,
}

impl Default for KrawczykProfileV1 {
    fn default() -> Self {
        Self {
            maximum_basis_entry: 64,
            maximum_preconditioner_inf_norm: 1.0e8,
            maximum_contraction_bound: 0.99,
            minimum_box_radius: 1.0e-12,
            maximum_box_radius: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricFrame2V1 {
    pub frame_id: String,
    /// Maps coordinates in this frame into the physical reference frame.
    pub basis_to_physical: [[i64; 2]; 2],
    /// Lower Cholesky factor of S^T S, for the identity physical metric.
    pub state_metric_lower: [[i64; 2]; 2],
    /// Lower Cholesky factor of the residual-space metric.
    pub residual_metric_lower: [[i64; 2]; 2],
}

fn checked_i128(value: Option<i128>) -> Result<i128, CertificateInputErrorV1> {
    value.ok_or(CertificateInputErrorV1::IntegerArithmeticOverflow)
}

fn integer_metric_from_lower(
    lower: [[i64; 2]; 2],
) -> Result<[[i128; 2]; 2], CertificateInputErrorV1> {
    let l00 = i128::from(lower[0][0]);
    let l10 = i128::from(lower[1][0]);
    let l11 = i128::from(lower[1][1]);
    let a00 = checked_i128(l00.checked_mul(l00))?;
    let a01 = checked_i128(l00.checked_mul(l10))?;
    let a11 = checked_i128(
        l10.checked_mul(l10)
            .and_then(|value| value.checked_add(l11.checked_mul(l11)?)),
    )?;
    Ok([[a00, a01], [a01, a11]])
}

fn integer_metric_from_basis(
    basis: [[i64; 2]; 2],
) -> Result<[[i128; 2]; 2], CertificateInputErrorV1> {
    let s00 = i128::from(basis[0][0]);
    let s01 = i128::from(basis[0][1]);
    let s10 = i128::from(basis[1][0]);
    let s11 = i128::from(basis[1][1]);
    let a00 = checked_i128(
        s00.checked_mul(s00)
            .and_then(|value| value.checked_add(s10.checked_mul(s10)?)),
    )?;
    let a01 = checked_i128(
        s00.checked_mul(s01)
            .and_then(|value| value.checked_add(s10.checked_mul(s11)?)),
    )?;
    let a11 = checked_i128(
        s01.checked_mul(s01)
            .and_then(|value| value.checked_add(s11.checked_mul(s11)?)),
    )?;
    Ok([[a00, a01], [a01, a11]])
}

impl MetricFrame2V1 {
    pub fn identity() -> Self {
        Self {
            frame_id: "physical-q".to_owned(),
            basis_to_physical: [[1, 0], [0, 1]],
            state_metric_lower: [[1, 0], [0, 1]],
            residual_metric_lower: [[1, 0], [0, 1]],
        }
    }

    pub fn upper_shear(shear: i64) -> Self {
        Self {
            frame_id: format!("dense-shear-{shear}"),
            basis_to_physical: [[1, shear], [0, 1]],
            state_metric_lower: [[1, 0], [shear, 1]],
            residual_metric_lower: [[1, 0], [0, 1]],
        }
    }

    pub fn new(
        frame_id: String,
        basis_to_physical: [[i64; 2]; 2],
        state_metric_lower: [[i64; 2]; 2],
        residual_metric_lower: [[i64; 2]; 2],
        profile: KrawczykProfileV1,
    ) -> Result<Self, CertificateInputErrorV1> {
        if frame_id.trim().is_empty() {
            return Err(CertificateInputErrorV1::InvalidIdentifier);
        }
        let frame = Self {
            frame_id,
            basis_to_physical,
            state_metric_lower,
            residual_metric_lower,
        };
        frame.validate(profile)?;
        Ok(frame)
    }

    pub fn validate(
        &self,
        profile: KrawczykProfileV1,
    ) -> Result<(), CertificateInputErrorV1> {
        // The integer frame matrices are converted to binary64 interval
        // endpoints below. Restrict entries to f64's exact-integer range.
        if profile.maximum_basis_entry <= 0 || profile.maximum_basis_entry > (1_i64 << 53) {
            return Err(CertificateInputErrorV1::InvalidProfile);
        }
        let all_entries = self
            .basis_to_physical
            .iter()
            .flatten()
            .chain(self.state_metric_lower.iter().flatten())
            .chain(self.residual_metric_lower.iter().flatten());
        if all_entries
            .clone()
            .any(|value| value.unsigned_abs() > profile.maximum_basis_entry as u64)
        {
            return Err(CertificateInputErrorV1::InvalidFrame);
        }

        let basis = self.basis_to_physical;
        let determinant = i128::from(basis[0][0])
            .checked_mul(i128::from(basis[1][1]))
            .and_then(|value| {
                value.checked_sub(i128::from(basis[0][1]).checked_mul(i128::from(basis[1][0]))?)
            })
            .ok_or(CertificateInputErrorV1::IntegerArithmeticOverflow)?;
        if determinant == 0 {
            return Err(CertificateInputErrorV1::BasisSingular);
        }

        for lower in [self.state_metric_lower, self.residual_metric_lower] {
            if lower[0][1] != 0 || lower[0][0] <= 0 || lower[1][1] <= 0 {
                return Err(CertificateInputErrorV1::InvalidFrame);
            }
        }

        if integer_metric_from_lower(self.state_metric_lower)?
            != integer_metric_from_basis(self.basis_to_physical)?
        {
            return Err(CertificateInputErrorV1::MetricFactorMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadraticSystem2V1 {
    pub system_id: &'static str,
    pub constant: Vector2,
    pub linear: Matrix2,
    /// Per-output coefficients [x^2, x*y, y^2].
    pub quadratic: [[f64; 3]; 2],
}

impl QuadraticSystem2V1 {
    pub fn validate(self) -> Result<(), CertificateInputErrorV1> {
        if self.system_id.trim().is_empty()
            || self.constant.iter().any(|value| !value.is_finite())
            || self.linear.iter().flatten().any(|value| !value.is_finite())
            || self.quadratic.iter().flatten().any(|value| !value.is_finite())
        {
            return Err(CertificateInputErrorV1::InvalidSystem);
        }
        Ok(())
    }

    pub fn known_root_fixture() -> Self {
        // Exact dyadic coefficients; (1/2, -1/4) is an exact root.
        Self {
            system_id: "coupled-quadratic-dyadic-v1",
            constant: [-0.53125, 0.25],
            linear: [[1.0, 0.125], [0.0, 1.0]],
            quadratic: [[0.25, 0.0, 0.0], [0.0, 0.125, 0.25]],
        }
    }

    fn evaluate(
        self,
        physical_state: IntervalVector2,
    ) -> Result<IntervalVector2, IntervalErrorV1> {
        let x = physical_state[0];
        let y = physical_state[1];
        let x2 = x.square()?;
        let xy = x.mul(y)?;
        let y2 = y.square()?;
        let mut output = [Interval::point(0.0)?, Interval::point(0.0)?];

        for (row, result) in output.iter_mut().enumerate() {
            let mut value = Interval::point(self.constant[row])?;
            value = value.add(Interval::point(self.linear[row][0])?.mul(x)?)?;
            value = value.add(Interval::point(self.linear[row][1])?.mul(y)?)?;
            value = value.add(Interval::point(self.quadratic[row][0])?.mul(x2)?)?;
            value = value.add(Interval::point(self.quadratic[row][1])?.mul(xy)?)?;
            value = value.add(Interval::point(self.quadratic[row][2])?.mul(y2)?)?;
            *result = value;
        }
        Ok(output)
    }

    fn jacobian(
        self,
        physical_state: IntervalVector2,
    ) -> Result<IntervalMatrix2, IntervalErrorV1> {
        let x = physical_state[0];
        let y = physical_state[1];
        let two = Interval::point(2.0)?;
        let mut jacobian = [
            [Interval::point(0.0)?, Interval::point(0.0)?],
            [Interval::point(0.0)?, Interval::point(0.0)?],
        ];

        for (row, entries) in jacobian.iter_mut().enumerate() {
            let xx = Interval::point(self.quadratic[row][0])?;
            let xy = Interval::point(self.quadratic[row][1])?;
            let yy = Interval::point(self.quadratic[row][2])?;

            let mut dx = Interval::point(self.linear[row][0])?;
            dx = dx.add(two.mul(xx)?.mul(x)?)?;
            dx = dx.add(xy.mul(y)?)?;

            let mut dy = Interval::point(self.linear[row][1])?;
            dy = dy.add(xy.mul(x)?)?;
            dy = dy.add(two.mul(yy)?.mul(y)?)?;

            entries[0] = dx;
            entries[1] = dy;
        }
        Ok(jacobian)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CertificateBindingV1 {
    pub system: QuadraticSystem2V1,
    pub frame: MetricFrame2V1,
    pub center_in_frame: Vector2,
    pub metric_box_radius: Vector2,
    pub profile: KrawczykProfileV1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KrawczykReceiptV1 {
    pub system_id: &'static str,
    /// Non-cryptographic display label only; exact replay uses the full binding.
    pub frame_id_tag: u64,
    /// Complete source problem and profile retained for independent replay.
    pub binding: CertificateBindingV1,
    pub krawczyk_image: IntervalVector2,
    pub state_box_in_frame: IntervalVector2,
    pub state_box_in_physical_frame: IntervalVector2,
    pub preconditioner: Matrix2,
    pub preconditioner_determinant: Interval,
    pub preconditioner_inf_norm: f64,
    pub contraction_bound_inf: f64,
    pub inclusion_margin_lower: f64,
    pub residual_dual_norm_upper: f64,
    pub metric_state_error_radius_upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InconclusiveReceiptV1 {
    pub system_id: &'static str,
    pub frame_id_tag: u64,
    pub reason: InconclusiveReasonV1,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CertificationOutcomeV1 {
    UniqueRootCertified(KrawczykReceiptV1),
    Inconclusive(InconclusiveReceiptV1),
}

fn frame_tag(frame_id: &str) -> u64 {
    frame_id
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn point_matrix(matrix: [[i64; 2]; 2]) -> Result<IntervalMatrix2, IntervalErrorV1> {
    Ok([
        [
            Interval::point(matrix[0][0] as f64)?,
            Interval::point(matrix[0][1] as f64)?,
        ],
        [
            Interval::point(matrix[1][0] as f64)?,
            Interval::point(matrix[1][1] as f64)?,
        ],
    ])
}

fn interval_identity() -> Result<IntervalMatrix2, IntervalErrorV1> {
    Ok([
        [Interval::point(1.0)?, Interval::point(0.0)?],
        [Interval::point(0.0)?, Interval::point(1.0)?],
    ])
}

fn interval_matrix_multiply(
    left: IntervalMatrix2,
    right: IntervalMatrix2,
) -> Result<IntervalMatrix2, IntervalErrorV1> {
    let mut result = [
        [Interval::point(0.0)?, Interval::point(0.0)?],
        [Interval::point(0.0)?, Interval::point(0.0)?],
    ];
    for (row, result_row) in result.iter_mut().enumerate() {
        for (column, output) in result_row.iter_mut().enumerate() {
            let mut value = Interval::point(0.0)?;
            for inner in 0..2 {
                value = value.add(left[row][inner].mul(right[inner][column])?)?;
            }
            *output = value;
        }
    }
    Ok(result)
}

fn interval_matrix_vector(
    matrix: IntervalMatrix2,
    vector: IntervalVector2,
) -> Result<IntervalVector2, IntervalErrorV1> {
    let mut result = [Interval::point(0.0)?, Interval::point(0.0)?];
    for (row, output) in result.iter_mut().enumerate() {
        let mut value = Interval::point(0.0)?;
        for (column, component) in vector.iter().enumerate() {
            value = value.add(matrix[row][column].mul(*component)?)?;
        }
        *output = value;
    }
    Ok(result)
}

fn solve_lower_interval(
    lower: [[i64; 2]; 2],
    rhs: IntervalVector2,
) -> Result<IntervalVector2, IntervalErrorV1> {
    let l00 = Interval::point(lower[0][0] as f64)?;
    let l10 = Interval::point(lower[1][0] as f64)?;
    let l11 = Interval::point(lower[1][1] as f64)?;
    let first = rhs[0].div(l00)?;
    let second = rhs[1].sub(l10.mul(first)?)?.div(l11)?;
    Ok([first, second])
}

fn solve_upper_transpose_interval(
    lower: [[i64; 2]; 2],
    rhs: IntervalVector2,
) -> Result<IntervalVector2, IntervalErrorV1> {
    let l00 = Interval::point(lower[0][0] as f64)?;
    let l10 = Interval::point(lower[1][0] as f64)?;
    let l11 = Interval::point(lower[1][1] as f64)?;
    let second = rhs[1].div(l11)?;
    let first = rhs[0].sub(l10.mul(second)?)?.div(l00)?;
    Ok([first, second])
}

fn interval_matrix_right_whiten(
    matrix: IntervalMatrix2,
    state_lower: [[i64; 2]; 2],
) -> Result<IntervalMatrix2, IntervalErrorV1> {
    let mut result = matrix;
    for row in 0..2 {
        let solved = solve_lower_interval(state_lower, [matrix[row][0], matrix[row][1]])?;
        result[row] = solved;
    }
    Ok(result)
}

fn interval_matrix_left_whiten(
    matrix: IntervalMatrix2,
    residual_lower: [[i64; 2]; 2],
) -> Result<IntervalMatrix2, IntervalErrorV1> {
    let mut result = matrix;
    for column in 0..2 {
        let solved =
            solve_lower_interval(residual_lower, [matrix[0][column], matrix[1][column]])?;
        result[0][column] = solved[0];
        result[1][column] = solved[1];
    }
    Ok(result)
}

fn frame_box_to_physical(
    frame: &MetricFrame2V1,
    center_in_frame: Vector2,
    box_in_whitened_state: IntervalVector2,
) -> Result<(IntervalVector2, IntervalVector2), IntervalErrorV1> {
    let delta_in_frame =
        solve_upper_transpose_interval(frame.state_metric_lower, box_in_whitened_state)?;
    let state_in_frame = [
        Interval::point(center_in_frame[0])?.add(delta_in_frame[0])?,
        Interval::point(center_in_frame[1])?.add(delta_in_frame[1])?,
    ];

    let basis = point_matrix(frame.basis_to_physical)?;
    let state_in_physical = interval_matrix_vector(basis, state_in_frame)?;
    Ok((state_in_frame, state_in_physical))
}

fn physical_center(
    frame: &MetricFrame2V1,
    center_in_frame: Vector2,
) -> Result<IntervalVector2, IntervalErrorV1> {
    let basis = point_matrix(frame.basis_to_physical)?;
    interval_matrix_vector(
        basis,
        [
            Interval::point(center_in_frame[0])?,
            Interval::point(center_in_frame[1])?,
        ],
    )
}

fn whitened_jacobian_box(
    system: QuadraticSystem2V1,
    frame: &MetricFrame2V1,
    center_in_frame: Vector2,
    box_in_whitened_state: IntervalVector2,
) -> Result<IntervalMatrix2, IntervalErrorV1> {
    let (_, physical_box) =
        frame_box_to_physical(frame, center_in_frame, box_in_whitened_state)?;
    let jacobian_physical = system.jacobian(physical_box)?;
    let basis = point_matrix(frame.basis_to_physical)?;
    let jacobian_frame = interval_matrix_multiply(jacobian_physical, basis)?;
    let right_whitened =
        interval_matrix_right_whiten(jacobian_frame, frame.state_metric_lower)?;
    interval_matrix_left_whiten(right_whitened, frame.residual_metric_lower)
}

fn whitened_residual_at_center(
    system: QuadraticSystem2V1,
    frame: &MetricFrame2V1,
    center_in_frame: Vector2,
) -> Result<IntervalVector2, IntervalErrorV1> {
    let physical = physical_center(frame, center_in_frame)?;
    let residual_physical = system.evaluate(physical)?;
    solve_lower_interval(frame.residual_metric_lower, residual_physical)
}

fn invert_point_matrix(matrix: IntervalMatrix2) -> Option<Matrix2> {
    let a = matrix[0][0].midpoint();
    let b = matrix[0][1].midpoint();
    let c = matrix[1][0].midpoint();
    let d = matrix[1][1].midpoint();
    let determinant = a * d - b * c;
    if !determinant.is_finite() || determinant == 0.0 {
        return None;
    }
    let result = [
        [d / determinant, -b / determinant],
        [-c / determinant, a / determinant],
    ];
    if result.iter().flatten().any(|value| !value.is_finite()) {
        return None;
    }
    Some(result)
}

fn point_matrix_interval(matrix: Matrix2) -> Result<IntervalMatrix2, IntervalErrorV1> {
    Ok([
        [
            Interval::point(matrix[0][0])?,
            Interval::point(matrix[0][1])?,
        ],
        [
            Interval::point(matrix[1][0])?,
            Interval::point(matrix[1][1])?,
        ],
    ])
}

fn point_matrix_determinant(matrix: Matrix2) -> Result<Interval, IntervalErrorV1> {
    let a = Interval::point(matrix[0][0])?;
    let b = Interval::point(matrix[0][1])?;
    let c = Interval::point(matrix[1][0])?;
    let d = Interval::point(matrix[1][1])?;
    a.mul(d)?.sub(b.mul(c)?)
}

fn preconditioner_inf_norm(matrix: Matrix2) -> f64 {
    let row0 = next_up(matrix[0][0].abs() + matrix[0][1].abs());
    let row1 = next_up(matrix[1][0].abs() + matrix[1][1].abs());
    row0.max(row1)
}

fn interval_matrix_inf_norm_upper(matrix: IntervalMatrix2) -> f64 {
    let row0 = next_up(matrix[0][0].abs_upper() + matrix[0][1].abs_upper());
    let row1 = next_up(matrix[1][0].abs_upper() + matrix[1][1].abs_upper());
    row0.max(row1)
}

fn negative_point_matrix_interval_vector(
    matrix: Matrix2,
    vector: IntervalVector2,
) -> Result<IntervalVector2, IntervalErrorV1> {
    let point = point_matrix_interval(matrix)?;
    let product = interval_matrix_vector(point, vector)?;
    Ok([product[0].neg()?, product[1].neg()?])
}

fn krawczyk_image(
    preconditioner: Matrix2,
    residual_at_center: IntervalVector2,
    jacobian_over_box: IntervalMatrix2,
    box_in_whitened_state: IntervalVector2,
) -> Result<IntervalVector2, IntervalErrorV1> {
    let c = point_matrix_interval(preconditioner)?;
    let c_j = interval_matrix_multiply(c, jacobian_over_box)?;
    let identity = interval_identity()?;
    let mut defect = identity;
    for row in 0..2 {
        for column in 0..2 {
            defect[row][column] = identity[row][column].sub(c_j[row][column])?;
        }
    }
    let centered = negative_point_matrix_interval_vector(preconditioner, residual_at_center)?;
    let image = interval_matrix_vector(defect, box_in_whitened_state)?;
    Ok([
        centered[0].add(image[0])?,
        centered[1].add(image[1])?,
    ])
}

fn residual_norm_upper(residual: IntervalVector2) -> Result<f64, IntervalErrorV1> {
    let first = residual[0].abs_upper();
    let second = residual[1].abs_upper();
    let first_sq = Interval::point(first)?.square()?;
    let second_sq = Interval::point(second)?.square()?;
    first_sq.add(second_sq)?.sqrt().map(|value| value.upper)
}

fn metric_box_radius_upper(radius: Vector2) -> Result<f64, IntervalErrorV1> {
    let first = Interval::point(radius[0])?.square()?;
    let second = Interval::point(radius[1])?.square()?;
    first.add(second)?.sqrt().map(|value| value.upper)
}

fn inclusion_margin_lower(
    inner: IntervalVector2,
    outer: IntervalVector2,
) -> Result<f64, IntervalErrorV1> {
    let left0 = Interval::point(inner[0].lower)?.sub(Interval::point(outer[0].lower)?)?;
    let right0 = Interval::point(outer[0].upper)?.sub(Interval::point(inner[0].upper)?)?;
    let left1 = Interval::point(inner[1].lower)?.sub(Interval::point(outer[1].lower)?)?;
    let right1 = Interval::point(outer[1].upper)?.sub(Interval::point(inner[1].upper)?)?;
    Ok([left0.lower, right0.lower, left1.lower, right1.lower]
        .into_iter()
        .fold(f64::INFINITY, f64::min))
}

fn inconclusive(
    system_id: &'static str,
    frame_id_tag: u64,
    reason: InconclusiveReasonV1,
) -> CertificationOutcomeV1 {
    CertificationOutcomeV1::Inconclusive(InconclusiveReceiptV1 {
        system_id,
        frame_id_tag,
        reason,
    })
}

fn interval_failure(
    system_id: &'static str,
    frame_id_tag: u64,
    _error: IntervalErrorV1,
) -> CertificationOutcomeV1 {
    inconclusive(
        system_id,
        frame_id_tag,
        InconclusiveReasonV1::IntervalArithmeticOverflow,
    )
}

/// Certify exactly one root in the supplied metric-whitened state box.
///
/// The nonlinear equations are specified in the physical reference frame.
/// The frame maps solver coordinates into that reference frame and binds
/// the exact state/residual metric factors used by the certificate.
pub fn certify_unique_root(
    system: QuadraticSystem2V1,
    frame: &MetricFrame2V1,
    center_in_frame: Vector2,
    metric_box_radius: Vector2,
    profile: KrawczykProfileV1,
) -> Result<CertificationOutcomeV1, CertificateInputErrorV1> {
    system.validate()?;
    frame.validate(profile)?;

    if !profile.maximum_preconditioner_inf_norm.is_finite()
        || profile.maximum_preconditioner_inf_norm <= 0.0
        || !profile.maximum_contraction_bound.is_finite()
        || profile.maximum_contraction_bound <= 0.0
        || profile.maximum_contraction_bound >= 1.0
        || !profile.minimum_box_radius.is_finite()
        || profile.minimum_box_radius <= 0.0
        || !profile.maximum_box_radius.is_finite()
        || profile.maximum_box_radius < profile.minimum_box_radius
        || metric_box_radius.iter().any(|radius| {
            !radius.is_finite()
                || *radius < profile.minimum_box_radius
                || *radius > profile.maximum_box_radius
        })
        || center_in_frame.iter().any(|value| !value.is_finite())
    {
        return Err(CertificateInputErrorV1::InvalidProfile);
    }

    let frame_id_tag = frame_tag(&frame.frame_id);
    let box_in_whitened_state = [
        Interval::new(-metric_box_radius[0], metric_box_radius[0])
            .map_err(|_| CertificateInputErrorV1::InvalidProfile)?,
        Interval::new(-metric_box_radius[1], metric_box_radius[1])
            .map_err(|_| CertificateInputErrorV1::InvalidProfile)?,
    ];

    let interval_work = (|| {
        let residual_at_center =
            whitened_residual_at_center(system, frame, center_in_frame)?;
        let jacobian_at_center = whitened_jacobian_box(
            system,
            frame,
            center_in_frame,
            [Interval::point(0.0)?, Interval::point(0.0)?],
        )?;
        let Some(preconditioner) = invert_point_matrix(jacobian_at_center) else {
            return Ok(inconclusive(
                system.system_id,
                frame_id_tag,
                InconclusiveReasonV1::SingularPreconditioner,
            ));
        };

        let preconditioner_determinant = point_matrix_determinant(preconditioner)?;
        if preconditioner_determinant.contains_zero() {
            return Ok(inconclusive(
                system.system_id,
                frame_id_tag,
                InconclusiveReasonV1::SingularPreconditioner,
            ));
        }

        let preconditioner_norm = preconditioner_inf_norm(preconditioner);
        if !preconditioner_norm.is_finite()
            || preconditioner_norm > profile.maximum_preconditioner_inf_norm
        {
            return Ok(inconclusive(
                system.system_id,
                frame_id_tag,
                InconclusiveReasonV1::PreconditionerOutsideProfile,
            ));
        }

        let jacobian_over_box = whitened_jacobian_box(
            system,
            frame,
            center_in_frame,
            box_in_whitened_state,
        )?;
        let mut defect = interval_identity()?;
        let c_j = interval_matrix_multiply(
            point_matrix_interval(preconditioner)?,
            jacobian_over_box,
        )?;
        for row in 0..2 {
            for column in 0..2 {
                defect[row][column] = defect[row][column].sub(c_j[row][column])?;
            }
        }

        let contraction = interval_matrix_inf_norm_upper(defect);
        if !contraction.is_finite() || contraction >= profile.maximum_contraction_bound {
            return Ok(inconclusive(
                system.system_id,
                frame_id_tag,
                InconclusiveReasonV1::ContractionNotEstablished,
            ));
        }

        let image = krawczyk_image(
            preconditioner,
            residual_at_center,
            jacobian_over_box,
            box_in_whitened_state,
        )?;
        let included = image[0].strictly_inside(box_in_whitened_state[0])
            && image[1].strictly_inside(box_in_whitened_state[1]);
        if !included {
            return Ok(inconclusive(
                system.system_id,
                frame_id_tag,
                InconclusiveReasonV1::KrawczykImageNotIncluded,
            ));
        }

        let margin = inclusion_margin_lower(image, box_in_whitened_state)?;
        if !margin.is_finite() || margin <= 0.0 {
            return Ok(inconclusive(
                system.system_id,
                frame_id_tag,
                InconclusiveReasonV1::KrawczykImageNotIncluded,
            ));
        }

        let (state_box_in_frame, state_box_in_physical_frame) =
            frame_box_to_physical(frame, center_in_frame, box_in_whitened_state)?;
        let residual_norm = residual_norm_upper(residual_at_center)?;
        let metric_radius = metric_box_radius_upper(metric_box_radius)?;

        Ok(CertificationOutcomeV1::UniqueRootCertified(
            KrawczykReceiptV1 {
                system_id: system.system_id,
                frame_id_tag,
                binding: CertificateBindingV1 {
                    system,
                    frame: (*frame).clone(),
                    center_in_frame,
                    metric_box_radius,
                    profile,
                },
                krawczyk_image: image,
                state_box_in_frame,
                state_box_in_physical_frame,
                preconditioner,
                preconditioner_determinant,
                preconditioner_inf_norm: preconditioner_norm,
                contraction_bound_inf: contraction,
                inclusion_margin_lower: margin,
                residual_dual_norm_upper: residual_norm,
                metric_state_error_radius_upper: metric_radius,
            },
        ))
    })();

    match interval_work {
        Ok(outcome) => Ok(outcome),
        Err(error) => Ok(interval_failure(system.system_id, frame_id_tag, error)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KrawczykVerificationReceiptV1 {
    pub system_id: &'static str,
    pub frame_id_tag: u64,
    pub replayed_metric_state_error_radius_upper: f64,
    pub replayed_contraction_bound_inf: f64,
    pub replayed_inclusion_margin_lower: f64,
}

fn same_f64_bits(left: f64, right: f64) -> bool {
    left.to_bits() == right.to_bits()
}

fn same_vector2_bits(left: Vector2, right: Vector2) -> bool {
    same_f64_bits(left[0], right[0]) && same_f64_bits(left[1], right[1])
}

fn same_matrix2_bits(left: Matrix2, right: Matrix2) -> bool {
    (0..2).all(|row| (0..2).all(|column| same_f64_bits(left[row][column], right[row][column])))
}

fn same_system_bits(left: QuadraticSystem2V1, right: QuadraticSystem2V1) -> bool {
    left.system_id == right.system_id
        && same_vector2_bits(left.constant, right.constant)
        && same_matrix2_bits(left.linear, right.linear)
        && (0..2).all(|row| {
            (0..3).all(|column| {
                same_f64_bits(left.quadratic[row][column], right.quadratic[row][column])
            })
        })
}

fn same_profile_bits(left: KrawczykProfileV1, right: KrawczykProfileV1) -> bool {
    left.maximum_basis_entry == right.maximum_basis_entry
        && same_f64_bits(
            left.maximum_preconditioner_inf_norm,
            right.maximum_preconditioner_inf_norm,
        )
        && same_f64_bits(
            left.maximum_contraction_bound,
            right.maximum_contraction_bound,
        )
        && same_f64_bits(left.minimum_box_radius, right.minimum_box_radius)
        && same_f64_bits(left.maximum_box_radius, right.maximum_box_radius)
}

fn same_binding_bits(left: &CertificateBindingV1, right: &CertificateBindingV1) -> bool {
    same_system_bits(left.system, right.system)
        && left.frame.frame_id == right.frame.frame_id
        && left.frame.basis_to_physical == right.frame.basis_to_physical
        && left.frame.state_metric_lower == right.frame.state_metric_lower
        && left.frame.residual_metric_lower == right.frame.residual_metric_lower
        && same_vector2_bits(left.center_in_frame, right.center_in_frame)
        && same_vector2_bits(left.metric_box_radius, right.metric_box_radius)
        && same_profile_bits(left.profile, right.profile)
}

fn same_interval_bits(left: Interval, right: Interval) -> bool {
    same_f64_bits(left.lower, right.lower) && same_f64_bits(left.upper, right.upper)
}

fn same_receipt_bits(left: &KrawczykReceiptV1, right: &KrawczykReceiptV1) -> bool {
    left.system_id == right.system_id
        && left.frame_id_tag == right.frame_id_tag
        && same_binding_bits(&left.binding, &right.binding)
        && (0..2).all(|index| {
            same_interval_bits(left.krawczyk_image[index], right.krawczyk_image[index])
                && same_interval_bits(left.state_box_in_frame[index], right.state_box_in_frame[index])
                && same_interval_bits(
                    left.state_box_in_physical_frame[index],
                    right.state_box_in_physical_frame[index],
                )
        })
        && same_matrix2_bits(left.preconditioner, right.preconditioner)
        && same_interval_bits(
            left.preconditioner_determinant,
            right.preconditioner_determinant,
        )
        && same_f64_bits(left.preconditioner_inf_norm, right.preconditioner_inf_norm)
        && same_f64_bits(left.contraction_bound_inf, right.contraction_bound_inf)
        && same_f64_bits(left.inclusion_margin_lower, right.inclusion_margin_lower)
        && same_f64_bits(left.residual_dual_norm_upper, right.residual_dual_norm_upper)
        && same_f64_bits(
            left.metric_state_error_radius_upper,
            right.metric_state_error_radius_upper,
        )
}

/// Independently replay a root receipt and require exact input/output bit agreement.
///
/// The caller supplies the expected binding from its frozen problem record. This
/// prevents a valid certificate for a different system, frame, box or profile
/// from being silently reused. This is exact structural replay, not a signature
/// or cryptographic digest.
pub fn verify_krawczyk_receipt(
    expected: &CertificateBindingV1,
    receipt: &KrawczykReceiptV1,
) -> Result<KrawczykVerificationReceiptV1, CertificateInputErrorV1> {
    if !same_binding_bits(expected, &receipt.binding) {
        return Err(CertificateInputErrorV1::ReceiptBindingMismatch);
    }
    let replay = certify_unique_root(
        expected.system,
        &expected.frame,
        expected.center_in_frame,
        expected.metric_box_radius,
        expected.profile,
    )?;
    let CertificationOutcomeV1::UniqueRootCertified(replayed) = replay else {
        return Err(CertificateInputErrorV1::ReplayDidNotCertify);
    };
    if !same_receipt_bits(receipt, &replayed) {
        return Err(CertificateInputErrorV1::ReplayResultMismatch);
    }
    Ok(KrawczykVerificationReceiptV1 {
        system_id: replayed.system_id,
        frame_id_tag: replayed.frame_id_tag,
        replayed_metric_state_error_radius_upper: replayed.metric_state_error_radius_upper,
        replayed_contraction_bound_inf: replayed.contraction_bound_inf,
        replayed_inclusion_margin_lower: replayed.inclusion_margin_lower,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approximate_physical_center() -> Vector2 {
        [0.501, -0.249]
    }

    fn approximate_shear_center(shear: i64) -> Vector2 {
        let physical = approximate_physical_center();
        [
            physical[0] - shear as f64 * physical[1],
            physical[1],
        ]
    }

    fn certified(outcome: CertificationOutcomeV1) -> KrawczykReceiptV1 {
        let CertificationOutcomeV1::UniqueRootCertified(receipt) = outcome else {
            panic!("expected unique root certificate");
        };
        receipt
    }

    fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
        while right != 0 {
            let remainder = left % right;
            left = right;
            right = remainder;
        }
        left
    }

    fn normalize_ratio(numerator: i128, denominator: i128) -> (i128, i128) {
        assert_ne!(denominator, 0);
        let sign = if denominator < 0 { -1 } else { 1 };
        let numerator = numerator * sign;
        let denominator = denominator.abs();
        if numerator == 0 {
            return (0, 1);
        }
        let divisor = gcd_u128(numerator.unsigned_abs(), denominator as u128) as i128;
        (numerator / divisor, denominator / divisor)
    }

    fn exact_f64_ratio(value: f64) -> (i128, i128) {
        let bits = value.to_bits();
        let negative = (bits >> 63) != 0;
        let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
        let fraction = bits & ((1_u64 << 52) - 1);
        let (significand, exponent) = if exponent_bits == 0 {
            (i128::from(fraction), -1074)
        } else {
            (
                i128::from((1_u64 << 52) | fraction),
                exponent_bits - 1023 - 52,
            )
        };
        let signed = if negative {
            -significand
        } else {
            significand
        };
        if signed == 0 {
            return (0, 1);
        }
        if exponent >= 0 {
            normalize_ratio(signed << exponent as u32, 1)
        } else {
            normalize_ratio(signed, 1_i128 << (-exponent) as u32)
        }
    }

    fn exact_add(left: (i128, i128), right: (i128, i128)) -> (i128, i128) {
        normalize_ratio(
            left.0 * right.1 + right.0 * left.1,
            left.1 * right.1,
        )
    }

    fn exact_sub(left: (i128, i128), right: (i128, i128)) -> (i128, i128) {
        normalize_ratio(
            left.0 * right.1 - right.0 * left.1,
            left.1 * right.1,
        )
    }

    fn exact_mul(left: (i128, i128), right: (i128, i128)) -> (i128, i128) {
        normalize_ratio(left.0 * right.0, left.1 * right.1)
    }

    fn exact_div(left: (i128, i128), right: (i128, i128)) -> (i128, i128) {
        assert_ne!(right.0, 0);
        normalize_ratio(left.0 * right.1, left.1 * right.0)
    }

    fn compare_f64_with_ratio(value: f64, numerator: i128, denominator: i128) -> std::cmp::Ordering {
        let (value_numerator, value_denominator) = exact_f64_ratio(value);
        (value_numerator * denominator).cmp(&(numerator * value_denominator))
    }

    fn assert_interval_contains_ratio(interval: Interval, rational: (i128, i128)) {
        assert!(rational.1 > 0);
        assert_ne!(
            compare_f64_with_ratio(interval.lower, rational.0, rational.1),
            std::cmp::Ordering::Greater,
            "lower endpoint exceeds exact rational"
        );
        assert_ne!(
            compare_f64_with_ratio(interval.upper, rational.0, rational.1),
            std::cmp::Ordering::Less,
            "upper endpoint is below exact rational"
        );
    }

    #[test]
    fn interval_arithmetic_contains_exact_dyadic_rational_operations() {
        for numerator in -31_i64..=31 {
            for denominator_power in [3_u32, 5, 8] {
                let denominator = (1_u64 << denominator_power) as f64;
                let left_value = numerator as f64 / denominator;
                let right_value = (numerator + 7) as f64 / (denominator * 2.0);
                if right_value == 0.0
                    || left_value == 0.0
                    || left_value == right_value
                    || left_value == -right_value
                {
                    continue;
                }
                let left = Interval::point(left_value).unwrap();
                let right = Interval::point(right_value).unwrap();
                let left_exact = exact_f64_ratio(left_value);
                let right_exact = exact_f64_ratio(right_value);

                assert_interval_contains_ratio(
                    left.add(right).unwrap(),
                    exact_add(left_exact, right_exact),
                );
                assert_interval_contains_ratio(
                    left.sub(right).unwrap(),
                    exact_sub(left_exact, right_exact),
                );
                assert_interval_contains_ratio(
                    left.mul(right).unwrap(),
                    exact_mul(left_exact, right_exact),
                );
                assert_interval_contains_ratio(
                    left.div(right).unwrap(),
                    exact_div(left_exact, right_exact),
                );
            }
        }
    }

    #[test]
    fn interval_addition_contains_exact_dyadic_sum() {
        let left = Interval::point(0.1).unwrap();
        let right = Interval::point(0.2).unwrap();
        let sum = left.add(right).unwrap();
        assert!(sum.lower <= 0.3);
        assert!(sum.upper >= 0.3);
        assert!(sum.lower < sum.upper);
    }

    #[test]
    fn interval_division_rejects_zero_crossing() {
        let numerator = Interval::point(1.0).unwrap();
        let denominator = Interval::new(-0.1, 0.1).unwrap();
        assert_eq!(
            numerator.div(denominator),
            Err(IntervalErrorV1::DivisionByZero)
        );
    }

    #[test]
    fn interval_multiplication_rejects_overflow() {
        let large = Interval::point(f64::MAX).unwrap();
        assert_eq!(
            large.mul(large),
            Err(IntervalErrorV1::ArithmeticOverflow)
        );
    }

    #[test]
    fn canonical_quadratic_root_is_uniquely_enclosed() {
        let system = QuadraticSystem2V1::known_root_fixture();
        let outcome = certify_unique_root(
            system,
            &MetricFrame2V1::identity(),
            approximate_physical_center(),
            [0.01, 0.01],
            KrawczykProfileV1::default(),
        )
        .unwrap();
        let receipt = certified(outcome);

        assert!(receipt.contraction_bound_inf < 1.0);
        assert!(receipt.inclusion_margin_lower > 0.0);
        assert!(receipt.metric_state_error_radius_upper > 0.0);
        assert!(receipt.state_box_in_physical_frame[0].lower < 0.5);
        assert!(receipt.state_box_in_physical_frame[0].upper > 0.5);
        assert!(receipt.state_box_in_physical_frame[1].lower < -0.25);
        assert!(receipt.state_box_in_physical_frame[1].upper > -0.25);
    }

    #[test]
    fn dense_nonorthogonal_rebasis_preserves_the_metric_error_certificate() {
        let system = QuadraticSystem2V1::known_root_fixture();
        let profile = KrawczykProfileV1::default();
        let canonical = certified(
            certify_unique_root(
                system,
                &MetricFrame2V1::identity(),
                approximate_physical_center(),
                [0.01, 0.01],
                profile,
            )
            .unwrap(),
        );

        let shear = 3;
        let dense = certified(
            certify_unique_root(
                system,
                &MetricFrame2V1::upper_shear(shear),
                approximate_shear_center(shear),
                [0.01, 0.01],
                profile,
            )
            .unwrap(),
        );

        assert!(
            (canonical.metric_state_error_radius_upper
                - dense.metric_state_error_radius_upper)
                .abs()
                < 1.0e-12
        );
        assert!(
            (canonical.contraction_bound_inf - dense.contraction_bound_inf).abs() < 1.0e-10
        );
        assert!(
            (canonical.residual_dual_norm_upper - dense.residual_dual_norm_upper).abs() < 1.0e-10
        );
        assert!(dense.state_box_in_physical_frame[0].lower < 0.5);
        assert!(dense.state_box_in_physical_frame[0].upper > 0.5);
        assert!(dense.state_box_in_physical_frame[1].lower < -0.25);
        assert!(dense.state_box_in_physical_frame[1].upper > -0.25);
    }

    #[test]
    fn certificate_retains_complete_problem_and_replays_bit_exactly() {
        let system = QuadraticSystem2V1::known_root_fixture();
        let frame = MetricFrame2V1::upper_shear(3);
        let center = approximate_shear_center(3);
        let radius = [0.01, 0.01];
        let profile = KrawczykProfileV1::default();
        let receipt = certified(
            certify_unique_root(system, &frame, center, radius, profile).unwrap(),
        );
        let verification = verify_krawczyk_receipt(&receipt.binding, &receipt).unwrap();
        assert_eq!(verification.system_id, system.system_id);
        assert_eq!(
            verification.replayed_metric_state_error_radius_upper,
            receipt.metric_state_error_radius_upper
        );
    }

    #[test]
    fn replay_rejects_changed_system_frame_box_and_profile() {
        let system = QuadraticSystem2V1::known_root_fixture();
        let frame = MetricFrame2V1::upper_shear(2);
        let center = approximate_shear_center(2);
        let radius = [0.01, 0.01];
        let profile = KrawczykProfileV1::default();
        let receipt = certified(
            certify_unique_root(system, &frame, center, radius, profile).unwrap(),
        );

        let mut altered_system = receipt.binding.clone();
        altered_system.system.constant[0] =
            f64::from_bits(altered_system.system.constant[0].to_bits() + 1);
        assert_eq!(
            verify_krawczyk_receipt(&altered_system, &receipt),
            Err(CertificateInputErrorV1::ReceiptBindingMismatch)
        );

        let mut altered_frame = receipt.binding.clone();
        altered_frame.frame.frame_id.push_str("-mutated");
        assert_eq!(
            verify_krawczyk_receipt(&altered_frame, &receipt),
            Err(CertificateInputErrorV1::ReceiptBindingMismatch)
        );

        let mut altered_box = receipt.binding.clone();
        altered_box.metric_box_radius[0] =
            f64::from_bits(altered_box.metric_box_radius[0].to_bits() + 1);
        assert_eq!(
            verify_krawczyk_receipt(&altered_box, &receipt),
            Err(CertificateInputErrorV1::ReceiptBindingMismatch)
        );

        let mut altered_profile = receipt.binding.clone();
        altered_profile.profile.maximum_contraction_bound = 0.98;
        assert_eq!(
            verify_krawczyk_receipt(&altered_profile, &receipt),
            Err(CertificateInputErrorV1::ReceiptBindingMismatch)
        );
    }

    #[test]
    fn metric_whitened_krawczyk_certificate_survives_seventeen_dense_shears() {
        let system = QuadraticSystem2V1::known_root_fixture();
        let profile = KrawczykProfileV1::default();
        let mut reference = None;
        for shear in -8_i64..=8 {
            let frame = MetricFrame2V1::upper_shear(shear);
            let center = approximate_shear_center(shear);
            let receipt = certified(
                certify_unique_root(
                    system,
                    &frame,
                    center,
                    [0.01, 0.01],
                    profile,
                )
                .unwrap(),
            );
            assert!(receipt.contraction_bound_inf < 1.0);
            assert!(receipt.inclusion_margin_lower > 0.0);
            assert!(receipt.state_box_in_physical_frame[0].lower < 0.5);
            assert!(receipt.state_box_in_physical_frame[0].upper > 0.5);
            assert!(receipt.state_box_in_physical_frame[1].lower < -0.25);
            assert!(receipt.state_box_in_physical_frame[1].upper > -0.25);
            if let Some((radius, contraction, residual)) = reference {
                assert!((radius - receipt.metric_state_error_radius_upper).abs() < 1.0e-10);
                assert!((contraction - receipt.contraction_bound_inf).abs() < 1.0e-8);
                assert!((residual - receipt.residual_dual_norm_upper).abs() < 1.0e-8);
            } else {
                reference = Some((
                    receipt.metric_state_error_radius_upper,
                    receipt.contraction_bound_inf,
                    receipt.residual_dual_norm_upper,
                ));
            }
        }
    }

    #[test]
    fn wrong_dense_metric_factor_is_rejected_before_certification() {
        let invalid = MetricFrame2V1::new(
            "wrong-metric".to_owned(),
            [[1, 3], [0, 1]],
            [[1, 0], [0, 1]],
            [[1, 0], [0, 1]],
            KrawczykProfileV1::default(),
        );
        assert_eq!(
            invalid,
            Err(CertificateInputErrorV1::MetricFactorMismatch)
        );
    }

    #[test]
    fn singular_coordinate_basis_is_rejected() {
        let invalid = MetricFrame2V1::new(
            "singular-basis".to_owned(),
            [[1, 2], [2, 4]],
            [[1, 0], [0, 1]],
            [[1, 0], [0, 1]],
            KrawczykProfileV1::default(),
        );
        assert_eq!(invalid, Err(CertificateInputErrorV1::BasisSingular));
    }

    #[test]
    fn altered_equation_is_not_misreported_as_certified() {
        let mut system = QuadraticSystem2V1::known_root_fixture();
        system.constant[0] += 1.0;
        let outcome = certify_unique_root(
            system,
            &MetricFrame2V1::identity(),
            approximate_physical_center(),
            [0.01, 0.01],
            KrawczykProfileV1::default(),
        )
        .unwrap();
        assert!(matches!(
            outcome,
            CertificationOutcomeV1::Inconclusive(
                InconclusiveReceiptV1 {
                    reason: InconclusiveReasonV1::KrawczykImageNotIncluded,
                    ..
                }
            )
        ));
    }

    #[test]
    fn singular_jacobian_is_inconclusive() {
        let system = QuadraticSystem2V1 {
            system_id: "singular-linear-system-v1",
            constant: [0.0, 0.0],
            linear: [[0.0, 0.0], [0.0, 1.0]],
            quadratic: [[0.0; 3]; 2],
        };
        let outcome = certify_unique_root(
            system,
            &MetricFrame2V1::identity(),
            [0.0, 0.0],
            [0.01, 0.01],
            KrawczykProfileV1::default(),
        )
        .unwrap();
        assert!(matches!(
            outcome,
            CertificationOutcomeV1::Inconclusive(
                InconclusiveReceiptV1 {
                    reason: InconclusiveReasonV1::SingularPreconditioner,
                    ..
                }
            )
        ));
    }

    #[test]
    fn basis_exceeding_declared_profile_is_rejected() {
        let profile = KrawczykProfileV1 {
            maximum_basis_entry: 4,
            ..KrawczykProfileV1::default()
        };
        let frame = MetricFrame2V1::new(
            "large-shear".to_owned(),
            [[1, 5], [0, 1]],
            [[1, 0], [5, 1]],
            [[1, 0], [0, 1]],
            profile,
        );
        assert_eq!(frame, Err(CertificateInputErrorV1::InvalidFrame));
    }

    #[test]
    fn frame_integer_limit_prevents_inexact_binary64_embedding() {
        let profile = KrawczykProfileV1 {
            maximum_basis_entry: (1_i64 << 53) + 1,
            ..KrawczykProfileV1::default()
        };
        let frame = MetricFrame2V1::new(
            "inexact-integer-frame".to_owned(),
            [[1, 1], [0, 1]],
            [[1, 0], [1, 1]],
            [[1, 0], [0, 1]],
            profile,
        );
        assert_eq!(frame, Err(CertificateInputErrorV1::InvalidProfile));
    }

    #[test]
    fn zero_box_radius_is_rejected() {
        let result = certify_unique_root(
            QuadraticSystem2V1::known_root_fixture(),
            &MetricFrame2V1::identity(),
            approximate_physical_center(),
            [0.0, 0.01],
            KrawczykProfileV1::default(),
        );
        assert_eq!(result, Err(CertificateInputErrorV1::InvalidProfile));
    }
}
