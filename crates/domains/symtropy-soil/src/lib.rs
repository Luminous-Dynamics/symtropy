// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Conservation-first soil-water accounting reference kernel.
//!
//! This crate accounts for supplied water fluxes in one control volume. Fluxes
//! are integrated depths in millimetres over the configured interval, not rates.
//! They are normalized to the same horizontal control area, so the water balance
//! is independent of area; area is retained for explicit volume conversions.
//!
//! This is bookkeeping, not a predictive soil model: precipitation, irrigation,
//! evapotranspiration, runoff, and drainage are inputs supplied by the caller.
//! No flux is inferred, clamped, or silently corrected by this crate.

use std::error::Error;
use std::fmt;

/// Numerical tolerance multiplier for the water-balance residual.
const RESIDUAL_EPSILON_MULTIPLIER: f64 = 64.0;

/// Water storage expressed as equivalent water depth in millimetres over the
/// control volume's horizontal footprint.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct WaterStorageMm(f64);

impl WaterStorageMm {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }

    fn as_depth(self) -> WaterDepthMm {
        WaterDepthMm(self.0)
    }
}

/// Water depth accumulated over one configured model interval, in millimetres.
/// Each interval flux uses this same temporal and spatial basis.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct WaterDepthMm(f64);

impl WaterDepthMm {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }

    /// Convert interval depth into volume for a horizontal control area.
    /// Volume in cubic metres = depth in millimetres * area in square metres / 1000.
    pub fn to_volume(self, area: ControlAreaM2) -> Result<WaterVolumeM3, SoilWaterError> {
        validate_positive("control_area_m2", area.0)?;
        validate_nonnegative("interval_depth_mm", self.0)?;

        // Scale the larger factor before multiplication so valid finite
        // conversions do not overflow only in an intermediate product.
        let volume = if self.0 >= 1000.0 {
            (self.0 / 1000.0) * area.0
        } else if area.0 >= 1000.0 {
            self.0 * (area.0 / 1000.0)
        } else {
            (self.0 * area.0) / 1000.0
        };
        if !volume.is_finite() {
            return Err(SoilWaterError::NonFiniteValue {
                field: "converted_volume_m3",
                value: volume,
            });
        }
        Ok(WaterVolumeM3(volume))
    }
}

/// Horizontal area of the control volume's surface footprint, in square metres.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ControlAreaM2(f64);

impl ControlAreaM2 {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }
}

/// Duration represented by one accounting step, in seconds. Fluxes are supplied
/// as integrated amounts for this interval; the kernel does not rescale them.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct TimeStepSeconds(f64);

impl TimeStepSeconds {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }
}

/// Water volume in cubic metres.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct WaterVolumeM3(f64);

impl WaterVolumeM3 {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }

    /// Convert volume to equivalent depth over a horizontal control area.
    pub fn to_depth(self, area: ControlAreaM2) -> Result<WaterDepthMm, SoilWaterError> {
        validate_positive("control_area_m2", area.0)?;
        validate_nonnegative("volume_m3", self.0)?;

        // Divide before multiplying to avoid an unnecessarily large
        // intermediate when volume and area are both finite and large.
        let depth = (self.0 / area.0) * 1000.0;
        if !depth.is_finite() {
            return Err(SoilWaterError::NonFiniteValue {
                field: "converted_depth_mm",
                value: depth,
            });
        }
        Ok(WaterDepthMm(depth))
    }
}

/// Non-negative flux depths accumulated during one timestep, normalized to the
/// same control-area footprint. Outflows are explicit inputs, never predictions.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct IntervalFluxes {
    pub precipitation: WaterDepthMm,
    pub irrigation: WaterDepthMm,
    pub evapotranspiration: WaterDepthMm,
    pub runoff: WaterDepthMm,
    pub drainage: WaterDepthMm,
}

impl IntervalFluxes {
    pub const fn zero() -> Self {
        Self {
            precipitation: WaterDepthMm::new(0.0),
            irrigation: WaterDepthMm::new(0.0),
            evapotranspiration: WaterDepthMm::new(0.0),
            runoff: WaterDepthMm::new(0.0),
            drainage: WaterDepthMm::new(0.0),
        }
    }
}

/// Current accounted water storage in the control volume.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilWaterState {
    pub storage: WaterStorageMm,
}

impl SoilWaterState {
    pub const fn new(storage: WaterStorageMm) -> Self {
        Self { storage }
    }
}

/// Validated accounting configuration. Capacity is a declared domain bound,
/// not an overflow/runoff model: above-capacity steps are rejected until the
/// caller supplies a physically justified overflow flux.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilWaterKernel {
    area: ControlAreaM2,
    interval: TimeStepSeconds,
    storage_capacity: WaterStorageMm,
}

impl SoilWaterKernel {
    pub fn new(
        area: ControlAreaM2,
        interval: TimeStepSeconds,
        storage_capacity: WaterStorageMm,
    ) -> Result<Self, SoilWaterError> {
        validate_positive("control_area_m2", area.0)?;
        validate_positive("time_step_seconds", interval.0)?;
        validate_positive("storage_capacity_mm", storage_capacity.0)?;

        Ok(Self {
            area,
            interval,
            storage_capacity,
        })
    }

    pub const fn area(self) -> ControlAreaM2 {
        self.area
    }

    pub const fn interval(self) -> TimeStepSeconds {
        self.interval
    }

    pub const fn storage_capacity(self) -> WaterStorageMm {
        self.storage_capacity
    }

    pub fn validate_state(&self, state: SoilWaterState) -> Result<(), SoilWaterError> {
        validate_nonnegative("storage_start_mm", state.storage.0)?;
        if state.storage.0 > self.storage_capacity.0 {
            return Err(SoilWaterError::StorageCapacityExceeded {
                requested_mm: state.storage.0,
                capacity_mm: self.storage_capacity.0,
            });
        }
        Ok(())
    }

    /// Account one timestep using the supplied interval fluxes:
    ///
    /// storage_next = storage_start + precipitation + irrigation
    ///                - evapotranspiration - runoff - drainage
    ///
    /// No negative-storage or capacity clamp is applied. Invalid inputs,
    /// impossible overdraw, and capacity violations return explicit errors.
    pub fn step(
        &self,
        state: SoilWaterState,
        fluxes: IntervalFluxes,
    ) -> Result<BalanceReport, SoilWaterError> {
        self.validate_state(state)?;
        validate_fluxes(fluxes)?;

        let inflow_mm = fluxes.precipitation.0 + fluxes.irrigation.0;
        let outflow_mm =
            fluxes.evapotranspiration.0 + fluxes.runoff.0 + fluxes.drainage.0;
        let available_mm = state.storage.0 + inflow_mm;

        if !inflow_mm.is_finite() {
            return Err(SoilWaterError::NonFiniteValue {
                field: "total_inflow_mm",
                value: inflow_mm,
            });
        }
        if !outflow_mm.is_finite() {
            return Err(SoilWaterError::NonFiniteValue {
                field: "total_outflow_mm",
                value: outflow_mm,
            });
        }
        if !available_mm.is_finite() {
            return Err(SoilWaterError::NonFiniteValue {
                field: "available_storage_mm",
                value: available_mm,
            });
        }
        if outflow_mm > available_mm {
            return Err(SoilWaterError::StorageOverdraw {
                available_mm,
                requested_outflow_mm: outflow_mm,
            });
        }

        let end_storage_mm = available_mm - outflow_mm;
        if !end_storage_mm.is_finite() {
            return Err(SoilWaterError::NonFiniteValue {
                field: "storage_end_mm",
                value: end_storage_mm,
            });
        }
        if end_storage_mm > self.storage_capacity.0 {
            return Err(SoilWaterError::StorageCapacityExceeded {
                requested_mm: end_storage_mm,
                capacity_mm: self.storage_capacity.0,
            });
        }

        let residual_mm = state.storage.0 + inflow_mm - outflow_mm - end_storage_mm;
        // Use the largest term instead of summing absolute terms; the sum
        // itself could overflow even when every input and the balance are finite.
        let residual_scale = state
            .storage
            .0
            .abs()
            .max(inflow_mm.abs())
            .max(outflow_mm.abs())
            .max(end_storage_mm.abs())
            .max(1.0);
        let residual_tolerance_mm =
            RESIDUAL_EPSILON_MULTIPLIER * f64::EPSILON * residual_scale * 4.0;
        if !residual_mm.is_finite() || residual_mm.abs() > residual_tolerance_mm {
            return Err(SoilWaterError::BalanceResidualExceeded {
                residual_mm,
                tolerance_mm: residual_tolerance_mm,
            });
        }

        let start_volume = state.storage.as_depth().to_volume(self.area)?;
        let end_volume = WaterStorageMm(end_storage_mm).as_depth().to_volume(self.area)?;
        let inflow_volume = WaterDepthMm(inflow_mm).to_volume(self.area)?;
        let outflow_volume = WaterDepthMm(outflow_mm).to_volume(self.area)?;

        Ok(BalanceReport {
            state_before: state,
            state_after: SoilWaterState::new(WaterStorageMm(end_storage_mm)),
            fluxes,
            inflow_mm: WaterDepthMm(inflow_mm),
            outflow_mm: WaterDepthMm(outflow_mm),
            residual_mm,
            residual_tolerance_mm,
            start_volume_m3: start_volume,
            end_volume_m3: end_volume,
            inflow_volume_m3: inflow_volume,
            outflow_volume_m3: outflow_volume,
            area: self.area,
            interval: self.interval,
        })
    }
}

/// Auditable accounting output. A report is returned only after input validation,
/// storage/capacity checks, and a scale-aware conservation check.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BalanceReport {
    pub state_before: SoilWaterState,
    pub state_after: SoilWaterState,
    pub fluxes: IntervalFluxes,
    pub inflow_mm: WaterDepthMm,
    pub outflow_mm: WaterDepthMm,
    pub residual_mm: f64,
    pub residual_tolerance_mm: f64,
    pub start_volume_m3: WaterVolumeM3,
    pub end_volume_m3: WaterVolumeM3,
    pub inflow_volume_m3: WaterVolumeM3,
    pub outflow_volume_m3: WaterVolumeM3,
    pub area: ControlAreaM2,
    pub interval: TimeStepSeconds,
}

/// Fail-closed validation errors. Bad inputs are not clamped into plausible
/// outputs.
#[derive(Debug, Clone, PartialEq)]
pub enum SoilWaterError {
    NonFiniteValue {
        field: &'static str,
        value: f64,
    },
    MustBePositive {
        field: &'static str,
        value: f64,
    },
    NegativeValue {
        field: &'static str,
        value: f64,
    },
    StorageOverdraw {
        available_mm: f64,
        requested_outflow_mm: f64,
    },
    StorageCapacityExceeded {
        requested_mm: f64,
        capacity_mm: f64,
    },
    BalanceResidualExceeded {
        residual_mm: f64,
        tolerance_mm: f64,
    },
}

impl fmt::Display for SoilWaterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue { field, value } => {
                write!(f, "{field} must be finite, got {value}")
            }
            Self::MustBePositive { field, value } => {
                write!(f, "{field} must be positive, got {value}")
            }
            Self::NegativeValue { field, value } => {
                write!(f, "{field} must be non-negative, got {value}")
            }
            Self::StorageOverdraw {
                available_mm,
                requested_outflow_mm,
            } => write!(
                f,
                "outflows ({requested_outflow_mm} mm) exceed available water ({available_mm} mm)"
            ),
            Self::StorageCapacityExceeded {
                requested_mm,
                capacity_mm,
            } => write!(
                f,
                "computed storage ({requested_mm} mm) exceeds capacity ({capacity_mm} mm)"
            ),
            Self::BalanceResidualExceeded {
                residual_mm,
                tolerance_mm,
            } => write!(
                f,
                "water-balance residual ({residual_mm} mm) exceeds tolerance ({tolerance_mm} mm)"
            ),
        }
    }
}

impl Error for SoilWaterError {}

fn validate_finite(field: &'static str, value: f64) -> Result<(), SoilWaterError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(SoilWaterError::NonFiniteValue { field, value })
    }
}

fn validate_positive(field: &'static str, value: f64) -> Result<(), SoilWaterError> {
    validate_finite(field, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(SoilWaterError::MustBePositive { field, value })
    }
}

fn validate_nonnegative(field: &'static str, value: f64) -> Result<(), SoilWaterError> {
    validate_finite(field, value)?;
    if value >= 0.0 {
        Ok(())
    } else {
        Err(SoilWaterError::NegativeValue { field, value })
    }
}

fn validate_fluxes(fluxes: IntervalFluxes) -> Result<(), SoilWaterError> {
    validate_nonnegative("precipitation_mm", fluxes.precipitation.0)?;
    validate_nonnegative("irrigation_mm", fluxes.irrigation.0)?;
    validate_nonnegative("evapotranspiration_mm", fluxes.evapotranspiration.0)?;
    validate_nonnegative("runoff_mm", fluxes.runoff.0)?;
    validate_nonnegative("drainage_mm", fluxes.drainage.0)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel(area_m2: f64, dt_seconds: f64, capacity_mm: f64) -> SoilWaterKernel {
        SoilWaterKernel::new(
            ControlAreaM2::new(area_m2),
            TimeStepSeconds::new(dt_seconds),
            WaterStorageMm::new(capacity_mm),
        )
        .expect("valid test configuration")
    }

    fn fluxes(p: f64, i: f64, et: f64, runoff: f64, drainage: f64) -> IntervalFluxes {
        IntervalFluxes {
            precipitation: WaterDepthMm::new(p),
            irrigation: WaterDepthMm::new(i),
            evapotranspiration: WaterDepthMm::new(et),
            runoff: WaterDepthMm::new(runoff),
            drainage: WaterDepthMm::new(drainage),
        }
    }

    #[test]
    fn zero_fluxes_preserve_storage_and_balance() {
        let model = kernel(10.0, 3600.0, 100.0);
        let report = model
            .step(
                SoilWaterState::new(WaterStorageMm::new(40.0)),
                IntervalFluxes::zero(),
            )
            .unwrap();

        assert_eq!(report.state_after.storage.value(), 40.0);
        assert_eq!(report.residual_mm, 0.0);
        assert!(report.residual_mm.abs() <= report.residual_tolerance_mm);
    }

    #[test]
    fn precipitation_and_irrigation_increase_storage() {
        let model = kernel(100.0, 86_400.0, 100.0);
        let report = model
            .step(
                SoilWaterState::new(WaterStorageMm::new(10.0)),
                fluxes(8.0, 4.0, 0.0, 0.0, 0.0),
            )
            .unwrap();

        assert_eq!(report.state_after.storage.value(), 22.0);
        assert_eq!(report.inflow_mm.value(), 12.0);
    }

    #[test]
    fn supplied_outflows_reduce_storage_without_being_predicted() {
        let model = kernel(25.0, 86_400.0, 100.0);
        let report = model
            .step(
                SoilWaterState::new(WaterStorageMm::new(50.0)),
                fluxes(10.0, 0.0, 5.0, 4.0, 11.0),
            )
            .unwrap();

        assert_eq!(report.state_after.storage.value(), 40.0);
        assert_eq!(report.outflow_mm.value(), 20.0);
    }

    #[test]
    fn zero_storage_is_a_valid_boundary_state() {
        let model = kernel(1.0, 1.0, 10.0);
        let report = model
            .step(
                SoilWaterState::new(WaterStorageMm::new(0.0)),
                IntervalFluxes::zero(),
            )
            .unwrap();

        assert_eq!(report.state_after.storage.value(), 0.0);
        assert!(
            model
                .step(
                    SoilWaterState::new(WaterStorageMm::new(0.0)),
                    fluxes(1.0, 0.0, 0.0, 0.0, 0.0),
                )
                .is_ok()
        );
    }

    #[test]
    fn unit_conversions_avoid_unnecessary_intermediate_overflow() {
        let huge_area = ControlAreaM2::new(1.0e308);
        let huge_volume = WaterDepthMm::new(1.0e308)
            .to_volume(ControlAreaM2::new(20.0))
            .unwrap();
        assert!((huge_volume.value() / 2.0e306 - 1.0).abs() < 1e-12);

        let huge_depth = WaterVolumeM3::new(1.0e308)
            .to_depth(huge_area)
            .unwrap();
        assert!((huge_depth.value() - 1000.0).abs() < 1e-12);
    }

    #[test]
    fn residual_tolerance_stays_finite_for_large_finite_balance_terms() {
        let model = kernel(1.0, 1.0, 1.5e308);
        let report = model
            .step(
                SoilWaterState::new(WaterStorageMm::new(1.0e308)),
                fluxes(0.0, 0.0, 5.0e307, 0.0, 0.0),
            )
            .unwrap();

        assert_eq!(report.state_after.storage.value(), 5.0e307);
        assert!(report.residual_tolerance_mm.is_finite());
        assert!(report.residual_mm.abs() <= report.residual_tolerance_mm);
    }

    #[test]
    fn depth_volume_conversion_is_area_explicit() {
        let area = ControlAreaM2::new(100.0);
        let volume = WaterDepthMm::new(1.0).to_volume(area).unwrap();

        assert!((volume.value() - 0.1).abs() < 1e-12);
        let depth = volume.to_depth(area).unwrap();
        assert!((depth.value() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn impossible_outflow_is_rejected_not_clamped() {
        let model = kernel(10.0, 3600.0, 100.0);
        let err = model
            .step(
                SoilWaterState::new(WaterStorageMm::new(3.0)),
                fluxes(0.0, 0.0, 2.0, 1.0, 1.0),
            )
            .unwrap_err();

        assert!(matches!(err, SoilWaterError::StorageOverdraw { .. }));
    }

    #[test]
    fn non_finite_flux_is_rejected() {
        let model = kernel(10.0, 3600.0, 100.0);
        let err = model
            .step(
                SoilWaterState::new(WaterStorageMm::new(10.0)),
                fluxes(f64::NAN, 0.0, 0.0, 0.0, 0.0),
            )
            .unwrap_err();

        assert!(matches!(
            err,
            SoilWaterError::NonFiniteValue {
                field: "precipitation_mm",
                ..
            }
        ));
    }

    #[test]
    fn negative_flux_is_rejected() {
        let model = kernel(10.0, 3600.0, 100.0);
        let err = model
            .step(
                SoilWaterState::new(WaterStorageMm::new(10.0)),
                fluxes(-0.01, 0.0, 0.0, 0.0, 0.0),
            )
            .unwrap_err();

        assert!(matches!(
            err,
            SoilWaterError::NegativeValue {
                field: "precipitation_mm",
                ..
            }
        ));
    }

    #[test]
    fn over_capacity_and_invalid_configuration_are_explicit() {
        let model = kernel(10.0, 3600.0, 100.0);
        let err = model
            .step(
                SoilWaterState::new(WaterStorageMm::new(95.0)),
                fluxes(10.0, 0.0, 0.0, 0.0, 0.0),
            )
            .unwrap_err();

        assert!(matches!(
            err,
            SoilWaterError::StorageCapacityExceeded { .. }
        ));
        assert!(matches!(
            SoilWaterKernel::new(
                ControlAreaM2::new(0.0),
                TimeStepSeconds::new(1.0),
                WaterStorageMm::new(10.0)
            ),
            Err(SoilWaterError::MustBePositive {
                field: "control_area_m2",
                ..
            })
        ));
        assert!(matches!(
            SoilWaterKernel::new(
                ControlAreaM2::new(1.0),
                TimeStepSeconds::new(f64::INFINITY),
                WaterStorageMm::new(10.0)
            ),
            Err(SoilWaterError::NonFiniteValue {
                field: "time_step_seconds",
                ..
            })
        ));
    }

    #[test]
    fn repeated_replay_is_bitwise_identical() {
        let model = kernel(5.0, 3600.0, 200.0);
        let initial = SoilWaterState::new(WaterStorageMm::new(40.0));
        let schedule = [
            fluxes(2.0, 0.0, 0.5, 0.0, 0.25),
            fluxes(0.0, 1.0, 0.5, 0.1, 0.2),
            fluxes(0.25, 0.0, 0.6, 0.0, 0.1),
            fluxes(0.0, 0.0, 0.7, 0.0, 0.15),
        ];

        fn run(
            model: SoilWaterKernel,
            initial: SoilWaterState,
            schedule: &[IntervalFluxes],
        ) -> Vec<BalanceReport> {
            let mut state = initial;
            let mut reports = Vec::new();
            for step_index in 0..64 {
                let flux = schedule[step_index % schedule.len()];
                let report = model.step(state, flux).unwrap();
                state = report.state_after;
                reports.push(report);
            }
            reports
        }

        assert_eq!(run(model, initial, &schedule), run(model, initial, &schedule));
    }
}
