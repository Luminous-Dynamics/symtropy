// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Deterministic, fail-closed actuator capability-envelope reference.
//!
//! The envelope stores empirically qualified torque limits for rectangular
//! speed/temperature cells. No extrapolation or interpolation is performed.
//! Authentication and provenance of the source measurements are upstream
//! requirements; a digest is an integrity identifier, not a signature.

use std::error::Error;
use std::fmt::{Display, Formatter};

pub const BASIS_POINT_SCALE: u64 = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Digest(pub [u8; 32]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvelopeError {
    EmptyArticleId,
    EmptyCalibrationId,
    InvalidValidityWindow,
    InvalidVoltageBand,
    InvalidDutyLimit,
    InvalidDeratingMargin,
    InsufficientSpeedBreakpoints,
    InsufficientTemperatureBreakpoints,
    UnorderedSpeedBreakpoints,
    UnorderedTemperatureBreakpoints,
    WrongCellCount,
    ZeroCellDuration,
}

impl Display for EnvelopeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyArticleId => write!(formatter, "article ID must not be empty"),
            Self::EmptyCalibrationId => write!(formatter, "calibration ID must not be empty"),
            Self::InvalidValidityWindow => write!(formatter, "valid_from must be earlier than valid_until"),
            Self::InvalidVoltageBand => write!(formatter, "minimum supply voltage must not exceed maximum"),
            Self::InvalidDutyLimit => write!(formatter, "maximum duty must be in 0..=10000 basis points"),
            Self::InvalidDeratingMargin => write!(formatter, "derating margin must be in 0..=10000 basis points"),
            Self::InsufficientSpeedBreakpoints => write!(formatter, "at least two speed breakpoints are required"),
            Self::InsufficientTemperatureBreakpoints => write!(formatter, "at least two temperature breakpoints are required"),
            Self::UnorderedSpeedBreakpoints => write!(formatter, "speed breakpoints must be strictly increasing"),
            Self::UnorderedTemperatureBreakpoints => write!(formatter, "temperature breakpoints must be strictly increasing"),
            Self::WrongCellCount => write!(formatter, "one cell is required for every adjacent speed/temperature rectangle"),
            Self::ZeroCellDuration => write!(formatter, "each qualified cell must have a positive maximum command duration"),
        }
    }
}

impl Error for EnvelopeError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QualifiedCell {
    /// Qualified positive-direction torque magnitude in mN·m.
    pub positive_limit_mnm: u64,
    /// Qualified negative-direction torque magnitude in mN·m.
    pub negative_limit_mnm: u64,
    /// Maximum uninterrupted command duration in milliseconds.
    pub max_command_duration_ms: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityEnvelope {
    article_id: String,
    calibration_id: String,
    calibration_digest: Digest,
    valid_from_ms: u64,
    valid_until_ms: u64,
    supply_voltage_min_mv: u32,
    supply_voltage_max_mv: u32,
    max_duty_basis_points: u16,
    derating_margin_basis_points: u16,
    /// Absolute motor speed breakpoints, ascending, in milli-RPM.
    speed_breakpoints_mrpm: Vec<u32>,
    /// Measured motor temperature breakpoints, ascending, in milli-degrees C.
    temperature_breakpoints_mc: Vec<i32>,
    /// Temperature-major, speed-minor cells.
    /// Index = temperature_interval * speed_interval_count + speed_interval.
    cells: Vec<QualifiedCell>,
}

impl CapabilityEnvelope {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        article_id: impl Into<String>,
        calibration_id: impl Into<String>,
        calibration_digest: Digest,
        valid_from_ms: u64,
        valid_until_ms: u64,
        supply_voltage_min_mv: u32,
        supply_voltage_max_mv: u32,
        max_duty_basis_points: u16,
        derating_margin_basis_points: u16,
        speed_breakpoints_mrpm: Vec<u32>,
        temperature_breakpoints_mc: Vec<i32>,
        cells: Vec<QualifiedCell>,
    ) -> Result<Self, EnvelopeError> {
        let article_id = article_id.into();
        let calibration_id = calibration_id.into();
        if article_id.trim().is_empty() {
            return Err(EnvelopeError::EmptyArticleId);
        }
        if calibration_id.trim().is_empty() {
            return Err(EnvelopeError::EmptyCalibrationId);
        }
        if valid_from_ms >= valid_until_ms {
            return Err(EnvelopeError::InvalidValidityWindow);
        }
        if supply_voltage_min_mv > supply_voltage_max_mv {
            return Err(EnvelopeError::InvalidVoltageBand);
        }
        if max_duty_basis_points > BASIS_POINT_SCALE as u16 {
            return Err(EnvelopeError::InvalidDutyLimit);
        }
        if derating_margin_basis_points > BASIS_POINT_SCALE as u16 {
            return Err(EnvelopeError::InvalidDeratingMargin);
        }
        if speed_breakpoints_mrpm.len() < 2 {
            return Err(EnvelopeError::InsufficientSpeedBreakpoints);
        }
        if temperature_breakpoints_mc.len() < 2 {
            return Err(EnvelopeError::InsufficientTemperatureBreakpoints);
        }
        if speed_breakpoints_mrpm.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(EnvelopeError::UnorderedSpeedBreakpoints);
        }
        if temperature_breakpoints_mc
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(EnvelopeError::UnorderedTemperatureBreakpoints);
        }
        let expected_cells = (speed_breakpoints_mrpm.len() - 1)
            .saturating_mul(temperature_breakpoints_mc.len() - 1);
        if cells.len() != expected_cells {
            return Err(EnvelopeError::WrongCellCount);
        }
        if cells.iter().any(|cell| cell.max_command_duration_ms == 0) {
            return Err(EnvelopeError::ZeroCellDuration);
        }

        Ok(Self {
            article_id,
            calibration_id,
            calibration_digest,
            valid_from_ms,
            valid_until_ms,
            supply_voltage_min_mv,
            supply_voltage_max_mv,
            max_duty_basis_points,
            derating_margin_basis_points,
            speed_breakpoints_mrpm,
            temperature_breakpoints_mc,
            cells,
        })
    }

    pub fn article_id(&self) -> &str {
        &self.article_id
    }

    pub fn calibration_id(&self) -> &str {
        &self.calibration_id
    }

    pub fn calibration_digest(&self) -> Digest {
        self.calibration_digest
    }

    pub fn speed_breakpoints_mrpm(&self) -> &[u32] {
        &self.speed_breakpoints_mrpm
    }

    pub fn temperature_breakpoints_mc(&self) -> &[i32] {
        &self.temperature_breakpoints_mc
    }

    pub fn cells(&self) -> &[QualifiedCell] {
        &self.cells
    }

    pub fn evaluate(&self, request: TorqueRequest) -> TorqueDecision {
        let mut decision = TorqueDecision {
            command_id: request.command_id.clone(),
            article_id: self.article_id.clone(),
            calibration_id: self.calibration_id.clone(),
            calibration_digest: self.calibration_digest,
            cell_index: None,
            derated_limit_mnm: 0,
            requested_torque_mnm: request.requested_torque_mnm,
            admission: Admission::Rejected(RejectionReason::InvalidTelemetry),
        };

        if !request.telemetry_complete {
            decision.admission = Admission::Rejected(RejectionReason::InvalidTelemetry);
            return decision;
        }
        if request.now_ms < self.valid_from_ms || request.now_ms > self.valid_until_ms {
            decision.admission = Admission::Rejected(RejectionReason::CalibrationNotCurrent);
            return decision;
        }
        if request.supply_voltage_mv < self.supply_voltage_min_mv
            || request.supply_voltage_mv > self.supply_voltage_max_mv
        {
            decision.admission = Admission::Rejected(RejectionReason::SupplyVoltageOutOfRange);
            return decision;
        }
        if request.duty_basis_points > self.max_duty_basis_points {
            decision.admission = Admission::Rejected(RejectionReason::DutyLimitExceeded);
            return decision;
        }

        let Some(speed_interval) = interval_index(
            &self.speed_breakpoints_mrpm,
            request.speed_mrpm,
        ) else {
            decision.admission = Admission::Rejected(RejectionReason::SpeedOutOfRange);
            return decision;
        };
        let Some(temperature_interval) = interval_index(
            &self.temperature_breakpoints_mc,
            request.temperature_mc,
        ) else {
            decision.admission = Admission::Rejected(RejectionReason::TemperatureOutOfRange);
            return decision;
        };

        let speed_interval_count = self.speed_breakpoints_mrpm.len() - 1;
        let cell_index = temperature_interval * speed_interval_count + speed_interval;
        let cell = self.cells[cell_index];
        decision.cell_index = Some(cell_index);

        if request.duration_ms == 0 || request.duration_ms > cell.max_command_duration_ms {
            decision.admission =
                Admission::Rejected(RejectionReason::CommandDurationExceeded);
            return decision;
        }

        let qualified_limit = if request.requested_torque_mnm < 0 {
            cell.negative_limit_mnm
        } else {
            cell.positive_limit_mnm
        };
        let derated_limit = (u128::from(qualified_limit)
            * u128::from(BASIS_POINT_SCALE - u64::from(self.derating_margin_basis_points))
            / u128::from(BASIS_POINT_SCALE)) as u64;
        decision.derated_limit_mnm = derated_limit;

        let requested_magnitude = request.requested_torque_mnm.unsigned_abs();
        if requested_magnitude > derated_limit {
            decision.admission = Admission::Rejected(RejectionReason::TorqueLimitExceeded);
            return decision;
        }

        decision.admission = Admission::Accepted;
        decision
    }
}

fn interval_index<T: Copy + Ord>(breakpoints: &[T], value: T) -> Option<usize> {
    if breakpoints.len() < 2 || value < breakpoints[0] || value > *breakpoints.last()? {
        return None;
    }
    let interval = breakpoints
        .partition_point(|breakpoint| *breakpoint <= value)
        .saturating_sub(1)
        .min(breakpoints.len() - 2);
    Some(interval)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TorqueRequest {
    pub command_id: String,
    pub requested_torque_mnm: i64,
    pub speed_mrpm: u32,
    pub temperature_mc: i32,
    pub supply_voltage_mv: u32,
    pub duty_basis_points: u16,
    pub duration_ms: u32,
    pub now_ms: u64,
    pub telemetry_complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectionReason {
    InvalidTelemetry,
    CalibrationNotCurrent,
    SupplyVoltageOutOfRange,
    DutyLimitExceeded,
    SpeedOutOfRange,
    TemperatureOutOfRange,
    CommandDurationExceeded,
    TorqueLimitExceeded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Admission {
    Accepted,
    Rejected(RejectionReason),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TorqueDecision {
    pub command_id: String,
    pub article_id: String,
    pub calibration_id: String,
    pub calibration_digest: Digest,
    pub cell_index: Option<usize>,
    pub derated_limit_mnm: u64,
    pub requested_torque_mnm: i64,
    pub admission: Admission,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Digest {
        Digest([byte; 32])
    }

    fn envelope(margin_bp: u16) -> CapabilityEnvelope {
        // 2 speed intervals × 2 temperature intervals; row-major by temperature.
        CapabilityEnvelope::new(
            "joint-article-01",
            "cal-2026-11-01",
            digest(42),
            1_000,
            9_000,
            22_000,
            26_000,
            7_000,
            margin_bp,
            vec![0, 1_000_000, 2_000_000],
            vec![20_000, 40_000, 60_000],
            vec![
                QualifiedCell { positive_limit_mnm: 1_000, negative_limit_mnm: 900, max_command_duration_ms: 500 },
                QualifiedCell { positive_limit_mnm: 800, negative_limit_mnm: 750, max_command_duration_ms: 400 },
                QualifiedCell { positive_limit_mnm: 700, negative_limit_mnm: 650, max_command_duration_ms: 300 },
                QualifiedCell { positive_limit_mnm: 500, negative_limit_mnm: 450, max_command_duration_ms: 200 },
            ],
        )
        .expect("valid fixture")
    }

    fn request(torque: i64, speed: u32, temperature: i32) -> TorqueRequest {
        TorqueRequest {
            command_id: "command-1".to_owned(),
            requested_torque_mnm: torque,
            speed_mrpm: speed,
            temperature_mc: temperature,
            supply_voltage_mv: 24_000,
            duty_basis_points: 5_000,
            duration_ms: 100,
            now_ms: 5_000,
            telemetry_complete: true,
        }
    }

    #[test]
    fn admits_positive_torque_inside_qualified_cell() {
        let decision = envelope(0).evaluate(request(999, 500_000, 30_000));
        assert_eq!(decision.admission, Admission::Accepted);
        assert_eq!(decision.cell_index, Some(0));
        assert_eq!(decision.derated_limit_mnm, 1_000);
    }

    #[test]
    fn positive_and_negative_torque_use_separate_limits() {
        let env = envelope(0);
        let positive = env.evaluate(request(950, 500_000, 30_000));
        let negative = env.evaluate(request(-950, 500_000, 30_000));
        assert_eq!(positive.admission, Admission::Accepted);
        assert_eq!(
            negative.admission,
            Admission::Rejected(RejectionReason::TorqueLimitExceeded)
        );
        assert_eq!(negative.derated_limit_mnm, 900);
    }

    #[test]
    fn cell_selection_is_deterministic_at_internal_breakpoints() {
        let decision = envelope(0).evaluate(request(700, 1_000_000, 40_000));
        // Exact internal breakpoints select the upper/right cell.
        assert_eq!(decision.cell_index, Some(3));
        assert_eq!(decision.derated_limit_mnm, 500);
    }

    #[test]
    fn outer_boundaries_are_included_but_extrapolation_is_rejected() {
        let env = envelope(0);
        assert_eq!(env.evaluate(request(400, 2_000_000, 60_000)).admission, Admission::Accepted);
        assert_eq!(
            env.evaluate(request(1, 2_000_001, 60_000)).admission,
            Admission::Rejected(RejectionReason::SpeedOutOfRange)
        );
        assert_eq!(
            env.evaluate(request(1, 1_000_000, 60_001)).admission,
            Admission::Rejected(RejectionReason::TemperatureOutOfRange)
        );
        assert_eq!(
            env.evaluate(request(1, 0, 19_999)).admission,
            Admission::Rejected(RejectionReason::TemperatureOutOfRange)
        );
    }

    #[test]
    fn derating_margin_only_reduces_qualified_limit() {
        let decision = envelope(1_000).evaluate(request(900, 500_000, 30_000));
        assert_eq!(decision.derated_limit_mnm, 900);
        assert_eq!(decision.admission, Admission::Accepted);
        let rejected = envelope(1_000).evaluate(request(901, 500_000, 30_000));
        assert_eq!(
            rejected.admission,
            Admission::Rejected(RejectionReason::TorqueLimitExceeded)
        );
    }

    #[test]
    fn rejects_expired_and_not_yet_valid_calibration() {
        let mut old = request(1, 100, 30_000);
        old.now_ms = 9_001;
        assert_eq!(
            envelope(0).evaluate(old).admission,
            Admission::Rejected(RejectionReason::CalibrationNotCurrent)
        );
        let mut future = request(1, 100, 30_000);
        future.now_ms = 999;
        assert_eq!(
            envelope(0).evaluate(future).admission,
            Admission::Rejected(RejectionReason::CalibrationNotCurrent)
        );
    }

    #[test]
    fn supply_duty_and_duration_are_hard_constraints() {
        let env = envelope(0);
        let mut low_voltage = request(1, 100, 30_000);
        low_voltage.supply_voltage_mv = 21_999;
        assert_eq!(
            env.evaluate(low_voltage).admission,
            Admission::Rejected(RejectionReason::SupplyVoltageOutOfRange)
        );
        let mut high_duty = request(1, 100, 30_000);
        high_duty.duty_basis_points = 7_001;
        assert_eq!(
            env.evaluate(high_duty).admission,
            Admission::Rejected(RejectionReason::DutyLimitExceeded)
        );
        let mut long_command = request(1, 1_500_000, 50_000);
        long_command.duration_ms = 201;
        assert_eq!(
            env.evaluate(long_command).admission,
            Admission::Rejected(RejectionReason::CommandDurationExceeded)
        );
    }

    #[test]
    fn missing_telemetry_fails_closed() {
        let mut incomplete = request(1, 100, 30_000);
        incomplete.telemetry_complete = false;
        assert_eq!(
            envelope(0).evaluate(incomplete).admission,
            Admission::Rejected(RejectionReason::InvalidTelemetry)
        );
    }

    #[test]
    fn rejects_malformed_grid_and_calibration_metadata() {
        assert_eq!(
            CapabilityEnvelope::new(
                "article", "cal", digest(1), 1, 2, 10, 5, 100, 0,
                vec![0, 1], vec![0, 1],
                vec![QualifiedCell { positive_limit_mnm: 1, negative_limit_mnm: 1, max_command_duration_ms: 1 }],
            ),
            Err(EnvelopeError::InvalidVoltageBand)
        );
        assert_eq!(
            CapabilityEnvelope::new(
                "article", "cal", digest(1), 1, 2, 10, 10, 10_001, 0,
                vec![0, 1], vec![0, 1],
                vec![QualifiedCell { positive_limit_mnm: 1, negative_limit_mnm: 1, max_command_duration_ms: 1 }],
            ),
            Err(EnvelopeError::InvalidDutyLimit)
        );
        assert_eq!(
            CapabilityEnvelope::new(
                "article", "cal", digest(1), 1, 2, 10, 10, 100, 0,
                vec![0, 1], vec![0, 1],
                vec![],
            ),
            Err(EnvelopeError::WrongCellCount)
        );
        assert_eq!(
            CapabilityEnvelope::new(
                "article", "cal", digest(1), 1, 2, 10, 10, 100, 0,
                vec![0, 1, 1], vec![0, 1],
                vec![
                    QualifiedCell { positive_limit_mnm: 1, negative_limit_mnm: 1, max_command_duration_ms: 1 },
                    QualifiedCell { positive_limit_mnm: 1, negative_limit_mnm: 1, max_command_duration_ms: 1 },
                ],
            ),
            Err(EnvelopeError::UnorderedSpeedBreakpoints)
        );
    }

    #[test]
    fn decision_is_deterministic_and_bound_to_article_and_calibration() {
        let env = envelope(500);
        let request = request(-500, 500_000, 30_000);
        let first = env.evaluate(request.clone());
        let second = env.evaluate(request);
        assert_eq!(first, second);
        assert_eq!(first.article_id, "joint-article-01");
        assert_eq!(first.calibration_id, "cal-2026-11-01");
        assert_eq!(first.calibration_digest, digest(42));
    }

    #[test]
    fn zero_torque_still_requires_a_qualified_operating_region() {
        let decision = envelope(0).evaluate(request(0, 3_000_000, 30_000));
        assert_eq!(
            decision.admission,
            Admission::Rejected(RejectionReason::SpeedOutOfRange)
        );
    }
}
