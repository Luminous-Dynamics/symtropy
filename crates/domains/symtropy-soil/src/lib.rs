//! Conservation-first soil-water accounting reference kernel.
//!
//! This crate accounts for water storage and boundary fluxes. It does **not**
//! predict evapotranspiration, runoff, drainage, infiltration, soil hydraulic
//! parameters, crop responses, biochar effects, or material safety.
//!
//! All depths are expressed in millimeters over the same control area and time
//! interval. Inflow and outflow magnitudes are non-negative. If a caller's flux
//! estimates imply negative storage or storage above the configured capacity,
//! the step fails; the kernel does not silently clamp state or invent a flux.
//!
//! This is an accounting reference model, not a validated agricultural model.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use core::fmt;

/// Error returned for an invalid state, flux, or accounting transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoilWaterError {
    /// A depth was negative or not finite.
    InvalidDepth,
    /// Soil water capacity must be finite and strictly positive.
    InvalidCapacity,
    /// The control area must be finite and strictly positive.
    InvalidArea,
    /// Initial storage exceeded the configured capacity.
    InitialStorageExceedsCapacity,
    /// Supplied outflows exceed the water available in this step.
    OutflowExceedsAvailableWater,
    /// The computed storage exceeds the configured capacity; an outflow such
    /// as drainage or runoff may be missing or incorrectly specified.
    StorageExceedsCapacity,
    /// Arithmetic overflow or another non-finite intermediate was detected.
    NonFiniteArithmetic,
}

impl fmt::Display for SoilWaterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDepth => write!(f, "depth must be finite and non-negative"),
            Self::InvalidCapacity => write!(f, "capacity must be finite and strictly positive"),
            Self::InvalidArea => write!(f, "area must be finite and strictly positive"),
            Self::InitialStorageExceedsCapacity => {
                write!(f, "initial storage exceeds configured capacity")
            }
            Self::OutflowExceedsAvailableWater => {
                write!(f, "outflows exceed initial storage plus inflows")
            }
            Self::StorageExceedsCapacity => {
                write!(
                    f,
                    "computed storage exceeds capacity; account for runoff/drainage"
                )
            }
            Self::NonFiniteArithmetic => {
                write!(
                    f,
                    "non-finite intermediate encountered during water accounting"
                )
            }
        }
    }
}

impl std::error::Error for SoilWaterError {}

/// A non-negative water depth in millimeters over a declared control area.
///
/// Construction validates finiteness and non-negativity. This type deliberately
/// does not infer area or duration: all values combined in a step must already
/// represent equivalent depth over the same area and time interval.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct DepthMm(f64);

impl DepthMm {
    /// Constructs a validated millimeter depth.
    pub fn new(mm: f64) -> Result<Self, SoilWaterError> {
        if !mm.is_finite() || mm < 0.0 {
            return Err(SoilWaterError::InvalidDepth);
        }
        Ok(Self(mm))
    }

    /// Converts a non-negative depth in centimeters to millimeters.
    pub fn from_centimeters(cm: f64) -> Result<Self, SoilWaterError> {
        if !cm.is_finite() || cm < 0.0 {
            return Err(SoilWaterError::InvalidDepth);
        }
        let mm = cm * 10.0;
        if !mm.is_finite() {
            return Err(SoilWaterError::NonFiniteArithmetic);
        }
        Ok(Self(mm))
    }

    /// Converts a total water volume in liters over a surface area in square
    /// meters to equivalent depth in millimeters.
    ///
    /// Since one liter per square meter equals one millimeter of water,
    /// `1_000 L / 10 m² = 100 mm`.
    pub fn from_liters_over_area(
        volume_liters: f64,
        area_square_meters: f64,
    ) -> Result<Self, SoilWaterError> {
        if !area_square_meters.is_finite() || area_square_meters <= 0.0 {
            return Err(SoilWaterError::InvalidArea);
        }
        if !volume_liters.is_finite() || volume_liters < 0.0 {
            return Err(SoilWaterError::InvalidDepth);
        }
        let mm = volume_liters / area_square_meters;
        if !mm.is_finite() {
            return Err(SoilWaterError::NonFiniteArithmetic);
        }
        Ok(Self(mm))
    }

    /// Converts a constant non-negative rate in millimeters per hour integrated
    /// over a finite non-negative duration in hours.
    pub fn from_rate_mm_per_hour(
        rate_mm_per_hour: f64,
        duration_hours: f64,
    ) -> Result<Self, SoilWaterError> {
        if !rate_mm_per_hour.is_finite()
            || rate_mm_per_hour < 0.0
            || !duration_hours.is_finite()
            || duration_hours < 0.0
        {
            return Err(SoilWaterError::InvalidDepth);
        }
        let mm = rate_mm_per_hour * duration_hours;
        if !mm.is_finite() {
            return Err(SoilWaterError::NonFiniteArithmetic);
        }
        Ok(Self(mm))
    }

    /// Returns the depth in millimeters.
    pub fn as_mm(self) -> f64 {
        self.0
    }
}

/// Water storage and a hard upper bound for a single soil control volume.
///
/// The capacity is a caller-supplied admissibility bound. It is **not**
/// automatically equivalent to field capacity, saturation water content, or
/// plant-available water; those concepts require a defined soil profile and
/// soil-specific measurements/models.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilWaterState {
    storage: DepthMm,
    capacity: DepthMm,
}

impl SoilWaterState {
    /// Creates an initial state, requiring positive capacity and storage no
    /// greater than that capacity.
    pub fn new(storage_mm: f64, capacity_mm: f64) -> Result<Self, SoilWaterError> {
        if !capacity_mm.is_finite() || capacity_mm <= 0.0 {
            return Err(SoilWaterError::InvalidCapacity);
        }
        let storage = DepthMm::new(storage_mm)?;
        let capacity = DepthMm::new(capacity_mm)?;
        if storage.0 > capacity.0 {
            return Err(SoilWaterError::InitialStorageExceedsCapacity);
        }
        Ok(Self { storage, capacity })
    }

    /// Returns current storage in millimeters.
    pub fn storage_mm(self) -> f64 {
        self.storage.as_mm()
    }

    /// Returns the configured maximum storage in millimeters.
    pub fn capacity_mm(self) -> f64 {
        self.capacity.as_mm()
    }
}

/// Non-negative boundary flux magnitudes for one accounting step.
///
/// Fluxes must be integrated over the same step and converted to equivalent
/// depth over the same control area before construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterFluxes {
    precipitation: DepthMm,
    irrigation: DepthMm,
    evapotranspiration: DepthMm,
    runoff: DepthMm,
    drainage: DepthMm,
}

impl WaterFluxes {
    /// Constructs validated fluxes in millimeters for a single time step.
    pub fn new(
        precipitation_mm: f64,
        irrigation_mm: f64,
        evapotranspiration_mm: f64,
        runoff_mm: f64,
        drainage_mm: f64,
    ) -> Result<Self, SoilWaterError> {
        Ok(Self {
            precipitation: DepthMm::new(precipitation_mm)?,
            irrigation: DepthMm::new(irrigation_mm)?,
            evapotranspiration: DepthMm::new(evapotranspiration_mm)?,
            runoff: DepthMm::new(runoff_mm)?,
            drainage: DepthMm::new(drainage_mm)?,
        })
    }

    fn total_inflow(self) -> Result<f64, SoilWaterError> {
        let total = self.precipitation.0 + self.irrigation.0;
        if total.is_finite() {
            Ok(total)
        } else {
            Err(SoilWaterError::NonFiniteArithmetic)
        }
    }

    fn total_outflow(self) -> Result<f64, SoilWaterError> {
        let total = self.evapotranspiration.0 + self.runoff.0 + self.drainage.0;
        if total.is_finite() {
            Ok(total)
        } else {
            Err(SoilWaterError::NonFiniteArithmetic)
        }
    }
}

/// Auditable result of one conservation-only accounting step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterStepResult {
    /// State at the end of the step.
    pub state: SoilWaterState,
    /// Total precipitation plus irrigation, in millimeters.
    pub inflow_mm: f64,
    /// Total evapotranspiration plus runoff plus drainage, in millimeters.
    pub outflow_mm: f64,
    /// Numerical balance residual, in millimeters. Ideally near zero.
    ///
    /// Defined as `final storage + outflow - initial storage - inflow`.
    pub balance_residual_mm: f64,
}

/// Advances the accounting state with supplied boundary fluxes.
///
/// This function enforces the identity:
///
/// `final_storage = initial_storage + precipitation + irrigation
///                  - evapotranspiration - runoff - drainage`
///
/// The caller supplies all fluxes; none are predicted by this crate. Fluxes
/// that would cause negative storage or exceed the stated capacity are rejected
/// rather than clipped or silently reassigned.
pub fn advance(
    state: SoilWaterState,
    fluxes: WaterFluxes,
) -> Result<WaterStepResult, SoilWaterError> {
    let inflow = fluxes.total_inflow()?;
    let outflow = fluxes.total_outflow()?;
    let available = state.storage.0 + inflow;
    if !available.is_finite() {
        return Err(SoilWaterError::NonFiniteArithmetic);
    }
    if outflow > available {
        return Err(SoilWaterError::OutflowExceedsAvailableWater);
    }

    let next_storage = available - outflow;
    if !next_storage.is_finite() {
        return Err(SoilWaterError::NonFiniteArithmetic);
    }
    if next_storage > state.capacity.0 {
        return Err(SoilWaterError::StorageExceedsCapacity);
    }

    let new_state = SoilWaterState {
        storage: DepthMm(next_storage),
        capacity: state.capacity,
    };
    let residual = (next_storage + outflow) - available;
    if !residual.is_finite() {
        return Err(SoilWaterError::NonFiniteArithmetic);
    }

    Ok(WaterStepResult {
        state: new_state,
        inflow_mm: inflow,
        outflow_mm: outflow,
        balance_residual_mm: residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(storage_mm: f64, capacity_mm: f64) -> SoilWaterState {
        SoilWaterState::new(storage_mm, capacity_mm).expect("test state should be valid")
    }

    fn fluxes(p: f64, i: f64, et: f64, r: f64, d: f64) -> WaterFluxes {
        WaterFluxes::new(p, i, et, r, d).expect("test fluxes should be valid")
    }

    fn close(actual: f64, expected: f64) {
        let tolerance = 1.0e-12 * expected.abs().max(1.0);
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual}, expected {expected}, tolerance {tolerance}"
        );
    }

    #[test]
    fn zero_fluxes_preserve_storage() {
        let initial = state(30.0, 100.0);
        let result = advance(initial, fluxes(0.0, 0.0, 0.0, 0.0, 0.0)).unwrap();
        close(result.state.storage_mm(), 30.0);
        close(result.balance_residual_mm, 0.0);
    }

    #[test]
    fn precipitation_and_irrigation_increase_storage_after_losses() {
        let result = advance(state(30.0, 100.0), fluxes(12.0, 8.0, 2.0, 0.0, 0.0)).unwrap();
        close(result.inflow_mm, 20.0);
        close(result.outflow_mm, 2.0);
        close(result.state.storage_mm(), 48.0);
        close(result.balance_residual_mm, 0.0);
    }

    #[test]
    fn outflows_decrease_storage() {
        let result = advance(state(30.0, 100.0), fluxes(0.0, 0.0, 2.0, 3.0, 5.0)).unwrap();
        close(result.state.storage_mm(), 20.0);
        close(result.balance_residual_mm, 0.0);
    }

    #[test]
    fn exact_depletion_is_allowed_but_storage_cannot_go_negative() {
        let result = advance(state(10.0, 100.0), fluxes(5.0, 0.0, 15.0, 0.0, 0.0)).unwrap();
        close(result.state.storage_mm(), 0.0);
        let error = advance(state(10.0, 100.0), fluxes(0.0, 0.0, 10.1, 0.0, 0.0));
        assert_eq!(
            error.unwrap_err(),
            SoilWaterError::OutflowExceedsAvailableWater
        );
    }

    #[test]
    fn excess_rain_must_be_accounted_for_as_an_outflow() {
        let error = advance(state(90.0, 100.0), fluxes(20.0, 0.0, 0.0, 0.0, 0.0));
        assert_eq!(error.unwrap_err(), SoilWaterError::StorageExceedsCapacity);

        let result = advance(state(90.0, 100.0), fluxes(20.0, 0.0, 0.0, 5.0, 5.0)).unwrap();
        close(result.state.storage_mm(), 100.0);
        close(result.balance_residual_mm, 0.0);
    }

    #[test]
    fn rejects_negative_and_non_finite_depths() {
        assert_eq!(DepthMm::new(-0.1), Err(SoilWaterError::InvalidDepth));
        assert_eq!(DepthMm::new(f64::NAN), Err(SoilWaterError::InvalidDepth));
        assert_eq!(
            DepthMm::new(f64::INFINITY),
            Err(SoilWaterError::InvalidDepth)
        );
        assert_eq!(
            WaterFluxes::new(1.0, 0.0, -1.0, 0.0, 0.0),
            Err(SoilWaterError::InvalidDepth)
        );
    }

    #[test]
    fn rejects_invalid_capacity_and_initial_state() {
        assert_eq!(
            SoilWaterState::new(0.0, 0.0),
            Err(SoilWaterError::InvalidCapacity)
        );
        assert_eq!(
            SoilWaterState::new(0.0, f64::NAN),
            Err(SoilWaterError::InvalidCapacity)
        );
        assert_eq!(
            SoilWaterState::new(11.0, 10.0),
            Err(SoilWaterError::InitialStorageExceedsCapacity)
        );
    }

    #[test]
    fn unit_conversion_is_explicit_and_consistent() {
        let from_cm = DepthMm::from_centimeters(1.0).unwrap();
        let from_mm = DepthMm::new(10.0).unwrap();
        assert_eq!(from_cm, from_mm);
        assert_eq!(
            DepthMm::from_centimeters(-1.0),
            Err(SoilWaterError::InvalidDepth)
        );
    }

    #[test]
    fn area_and_time_unit_conversions_preserve_equivalent_depth() {
        let from_volume = DepthMm::from_liters_over_area(1_000.0, 10.0).unwrap();
        let from_rate = DepthMm::from_rate_mm_per_hour(20.0, 5.0).unwrap();
        let reference = DepthMm::new(100.0).unwrap();
        assert_eq!(from_volume, reference);
        assert_eq!(from_rate, reference);
        assert_eq!(
            DepthMm::from_liters_over_area(1.0, 0.0),
            Err(SoilWaterError::InvalidArea)
        );
        assert_eq!(
            DepthMm::from_rate_mm_per_hour(10.0, f64::INFINITY),
            Err(SoilWaterError::InvalidDepth)
        );
    }

    #[test]
    fn fixed_seed_property_sweep_preserves_mass_balance() {
        // Small deterministic xorshift generator keeps this test dependency-free
        // and exactly replayable on every platform.
        fn next(seed: &mut u64) -> f64 {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            (*seed as f64) / (u64::MAX as f64)
        }

        let mut seed = 0x5EED_7E22_2026_u64;
        for _case in 0..10_000 {
            let initial_mm = next(&mut seed) * 1_000.0;
            let precipitation_mm = next(&mut seed) * 25.0;
            let irrigation_mm = next(&mut seed) * 25.0;
            let available = initial_mm + precipitation_mm + irrigation_mm;
            let total_outflow = next(&mut seed) * available * 0.8;
            let evapotranspiration_mm = total_outflow * next(&mut seed);
            let remaining_after_et = total_outflow - evapotranspiration_mm;
            let runoff_mm = remaining_after_et * next(&mut seed);
            let drainage_mm = remaining_after_et - runoff_mm;

            let initial = state(initial_mm, 10_000.0);
            let fluxes = fluxes(
                precipitation_mm,
                irrigation_mm,
                evapotranspiration_mm,
                runoff_mm,
                drainage_mm,
            );
            let result = advance(initial, fluxes).expect("generated case is within bounds");
            let expected = initial_mm + precipitation_mm + irrigation_mm - total_outflow;
            let tolerance = 1.0e-9 * expected.abs().max(1.0);
            assert!(
                (result.state.storage_mm() - expected).abs() <= tolerance,
                "case {_case}: storage mismatch"
            );
            assert!(
                result.balance_residual_mm.abs() <= tolerance,
                "case {_case}: balance residual exceeded tolerance"
            );
            assert!(result.state.storage_mm() >= 0.0);
            assert!(result.state.storage_mm() <= result.state.capacity_mm());
        }
    }

    #[test]
    fn large_values_that_overflow_fail_closed() {
        let initial = state(f64::MAX / 2.0, f64::MAX);
        let inflow = fluxes(f64::MAX, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(
            advance(initial, inflow),
            Err(SoilWaterError::NonFiniteArithmetic)
        );
    }

    #[test]
    fn identical_inputs_replay_deterministically() {
        let initial = state(24.25, 80.0);
        let fluxes = fluxes(4.0, 2.5, 1.75, 0.25, 3.0);
        let first = advance(initial, fluxes).unwrap();
        let second = advance(initial, fluxes).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn multiple_steps_compose_without_hidden_water() {
        let initial = state(40.0, 120.0);
        let after_first = advance(initial, fluxes(10.0, 0.0, 2.0, 1.0, 3.0)).unwrap();
        let after_second = advance(after_first.state, fluxes(0.0, 5.0, 4.0, 1.0, 2.0)).unwrap();
        close(after_first.state.storage_mm(), 44.0);
        close(after_second.state.storage_mm(), 42.0);
        close(after_first.balance_residual_mm, 0.0);
        close(after_second.balance_residual_mm, 0.0);
    }
}
