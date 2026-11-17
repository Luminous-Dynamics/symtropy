#![forbid(unsafe_code)]

//! Bounded analytical reference for an idealized two-reservoir heat transfer.
//!
//! This crate does not estimate whole-robot entropy production. It deliberately
//! implements one narrow, independently checkable process so its assumptions
//! and units remain visible at the API boundary.

use core::fmt;

/// Failure modes are explicit; invalid or incomplete inputs never become zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermoError {
    NonFiniteInput,
    NegativeHeatMagnitude,
    NonPositiveAbsoluteTemperature,
    InvalidInterval,
    HeatDirectionNotGuaranteed,
    InvalidPortManifest,
    IncompleteBoundary,
    NonFiniteResult,
    NegativeEntropyResult,
}

impl fmt::Display for ThermoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ThermoError {}

/// Non-negative heat transferred from the hot reservoir to the cold reservoir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeatJoules(f64);

impl HeatJoules {
    pub fn new(value: f64) -> Result<Self, ThermoError> {
        if !value.is_finite() {
            return Err(ThermoError::NonFiniteInput);
        }
        if value < 0.0 {
            return Err(ThermoError::NegativeHeatMagnitude);
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Absolute temperature. Celsius and temperature differences are not accepted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kelvin(f64);

impl Kelvin {
    pub fn new(value: f64) -> Result<Self, ThermoError> {
        if !value.is_finite() {
            return Err(ThermoError::NonFiniteInput);
        }
        if value <= 0.0 {
            return Err(ThermoError::NonPositiveAbsoluteTemperature);
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Physical thermodynamic entropy generated, in joules per kelvin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntropyJPerK(f64);

impl EntropyJPerK {
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Exergy destruction, in joules, under a declared environment reference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExergyJoules(f64);

impl ExergyJoules {
    pub const fn get(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransferResult {
    pub entropy_generated: EntropyJPerK,
    /// None means no environment reference was supplied; it is not zero exergy.
    pub exergy_destroyed: Option<ExergyJoules>,
}

/// Idealized isothermal-reservoir transfer from hot to cold.
///
/// The model assumes a heat magnitude Q >= 0 moves from a reservoir at Th to
/// one at Tc, with Th >= Tc. It is not a whole-robot entropy balance and does
/// not model finite-capacity bodies, heat-port networks, mass flow, friction,
/// battery chemistry, electrical resistance or transient thermal dynamics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoReservoirTransfer {
    heat: HeatJoules,
    hot: Kelvin,
    cold: Kelvin,
    environment: Option<Kelvin>,
}

impl TwoReservoirTransfer {
    pub fn new(
        heat: HeatJoules,
        hot: Kelvin,
        cold: Kelvin,
        environment: Option<Kelvin>,
    ) -> Result<Self, ThermoError> {
        if hot.get() < cold.get() {
            return Err(ThermoError::HeatDirectionNotGuaranteed);
        }
        Ok(Self {
            heat,
            hot,
            cold,
            environment,
        })
    }

    /// Applies S_gen = Q(1/T_cold - 1/T_hot).
    ///
    /// Exergy destruction is returned only when T0 is explicitly supplied:
    /// X_destroyed = T0 * S_gen.
    pub fn evaluate(self) -> Result<TransferResult, ThermoError> {
        let reciprocal_temperature_delta = (1.0 / self.cold.get()) - (1.0 / self.hot.get());
        if !reciprocal_temperature_delta.is_finite() {
            return Err(ThermoError::NonFiniteResult);
        }
        if reciprocal_temperature_delta < 0.0 {
            return Err(ThermoError::NegativeEntropyResult);
        }

        let entropy_value = self.heat.get() * reciprocal_temperature_delta;
        if !entropy_value.is_finite() {
            return Err(ThermoError::NonFiniteResult);
        }
        if entropy_value < 0.0 {
            return Err(ThermoError::NegativeEntropyResult);
        }

        let exergy_destroyed = match self.environment {
            Some(reference_temperature) => {
                let value = reference_temperature.get() * entropy_value;
                if !value.is_finite() {
                    return Err(ThermoError::NonFiniteResult);
                }
                if value < 0.0 {
                    return Err(ThermoError::NegativeEntropyResult);
                }
                Some(ExergyJoules(value))
            }
            None => None,
        };

        Ok(TransferResult {
            entropy_generated: EntropyJPerK(entropy_value),
            exergy_destroyed,
        })
    }
}

/// Closed interval with finite endpoints and lower <= upper.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteInterval {
    lower: f64,
    upper: f64,
}

impl FiniteInterval {
    pub fn new(lower: f64, upper: f64) -> Result<Self, ThermoError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(ThermoError::NonFiniteInput);
        }
        if lower > upper {
            return Err(ThermoError::InvalidInterval);
        }
        Ok(Self { lower, upper })
    }

    pub const fn lower(self) -> f64 {
        self.lower
    }

    pub const fn upper(self) -> f64 {
        self.upper
    }
}

/// Non-negative interval for heat magnitude in joules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeatInterval(FiniteInterval);

impl HeatInterval {
    pub fn new(lower_j: f64, upper_j: f64) -> Result<Self, ThermoError> {
        let interval = FiniteInterval::new(lower_j, upper_j)?;
        if interval.lower < 0.0 {
            return Err(ThermoError::NegativeHeatMagnitude);
        }
        Ok(Self(interval))
    }
}

/// Positive absolute-temperature interval in kelvin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemperatureInterval(FiniteInterval);

impl TemperatureInterval {
    pub fn new(lower_k: f64, upper_k: f64) -> Result<Self, ThermoError> {
        let interval = FiniteInterval::new(lower_k, upper_k)?;
        if interval.lower <= 0.0 {
            return Err(ThermoError::NonPositiveAbsoluteTemperature);
        }
        Ok(Self(interval))
    }
}

/// Bounds on generated thermodynamic entropy, in joules per kelvin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntropyInterval(FiniteInterval);

impl EntropyInterval {
    pub const fn lower_j_per_k(self) -> f64 {
        self.0.lower
    }

    pub const fn upper_j_per_k(self) -> f64 {
        self.0.upper
    }
}

/// Bounds on exergy destruction, in joules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExergyInterval(FiniteInterval);

impl ExergyInterval {
    pub const fn lower_j(self) -> f64 {
        self.0.lower
    }

    pub const fn upper_j(self) -> f64 {
        self.0.upper
    }
}

/// Conservative entropy bounds for independent rectangular input intervals.
///
/// Requires every possible hot temperature to be >= every possible cold
/// temperature (hot.lower >= cold.upper). Otherwise the declared direction is
/// not guaranteed and the function fails closed. The monotone corner formulas
/// are exact for this idealized two-reservoir equation.
pub fn entropy_generation_bounds(
    heat: HeatInterval,
    hot: TemperatureInterval,
    cold: TemperatureInterval,
) -> Result<EntropyInterval, ThermoError> {
    if hot.0.lower < cold.0.upper {
        return Err(ThermoError::HeatDirectionNotGuaranteed);
    }

    let lower = heat.0.lower * ((1.0 / cold.0.upper) - (1.0 / hot.0.lower));
    let upper = heat.0.upper * ((1.0 / cold.0.lower) - (1.0 / hot.0.upper));

    if !lower.is_finite() || !upper.is_finite() {
        return Err(ThermoError::NonFiniteResult);
    }
    if lower < 0.0 || upper < lower {
        return Err(ThermoError::NegativeEntropyResult);
    }

    Ok(EntropyInterval(FiniteInterval { lower, upper }))
}

/// Converts entropy-generation bounds to exergy-destruction bounds using an
/// explicitly declared fixed environment temperature T0.
pub fn exergy_destruction_bounds(
    entropy: EntropyInterval,
    environment: Kelvin,
) -> Result<ExergyInterval, ThermoError> {
    let lower = environment.get() * entropy.0.lower;
    let upper = environment.get() * entropy.0.upper;
    if !lower.is_finite() || !upper.is_finite() {
        return Err(ThermoError::NonFiniteResult);
    }
    if lower < 0.0 || upper < lower {
        return Err(ThermoError::NegativeEntropyResult);
    }
    Ok(ExergyInterval(FiniteInterval { lower, upper }))
}

/// Checks set completeness for the required heat-port identifiers.
///
/// This is a structural coverage check only. It does not authenticate the
/// sensors, establish calibration, validate timestamps or prove that a port's
/// physical heat flow was measured correctly.
pub fn validate_heat_port_coverage(
    required: &[&str],
    observed: &[&str],
) -> Result<(), ThermoError> {
    if required.is_empty()
        || required.iter().any(|id| id.is_empty())
        || observed.iter().any(|id| id.is_empty())
    {
        return Err(ThermoError::InvalidPortManifest);
    }

    for (i, id) in required.iter().enumerate() {
        if required[..i].contains(id) {
            return Err(ThermoError::InvalidPortManifest);
        }
    }
    for (i, id) in observed.iter().enumerate() {
        if observed[..i].contains(id) {
            return Err(ThermoError::IncompleteBoundary);
        }
    }

    if required.len() != observed.len() || required.iter().any(|id| !observed.contains(id)) {
        return Err(ThermoError::IncompleteBoundary);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        let tolerance = 1.0e-12 * expected.abs().max(1.0);
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}"
        );
    }

    fn kelvin(value: f64) -> Kelvin {
        Kelvin::new(value).expect("test Kelvin should be valid")
    }

    fn heat(value: f64) -> HeatJoules {
        HeatJoules::new(value).expect("test heat should be valid")
    }

    #[test]
    fn equal_temperature_transfer_has_zero_generation() {
        let result = TwoReservoirTransfer::new(
            heat(100.0),
            kelvin(300.0),
            kelvin(300.0),
            Some(kelvin(300.0)),
        )
        .expect("valid equal-temperature idealization")
        .evaluate()
        .expect("finite result");

        close(result.entropy_generated.get(), 0.0);
        close(result.exergy_destroyed.expect("T0 supplied").get(), 0.0);
    }

    #[test]
    fn hot_to_cold_reference_matches_analytic_answer() {
        let result = TwoReservoirTransfer::new(
            heat(100.0),
            kelvin(350.0),
            kelvin(300.0),
            Some(kelvin(300.0)),
        )
        .expect("valid direction")
        .evaluate()
        .expect("finite result");

        close(
            result.entropy_generated.get(),
            100.0 * (1.0 / 300.0 - 1.0 / 350.0),
        );
        close(
            result.exergy_destroyed.expect("T0 supplied").get(),
            300.0 * 100.0 * (1.0 / 300.0 - 1.0 / 350.0),
        );
    }

    #[test]
    fn exergy_is_unknown_without_environment_reference() {
        let result = TwoReservoirTransfer::new(heat(10.0), kelvin(330.0), kelvin(300.0), None)
            .expect("valid direction")
            .evaluate()
            .expect("finite result");

        assert!(result.exergy_destroyed.is_none());
    }

    #[test]
    fn reversed_temperature_order_is_rejected() {
        assert_eq!(
            TwoReservoirTransfer::new(heat(10.0), kelvin(290.0), kelvin(300.0), None,),
            Err(ThermoError::HeatDirectionNotGuaranteed)
        );
    }

    #[test]
    fn invalid_heat_and_temperature_are_rejected() {
        assert_eq!(
            HeatJoules::new(-1.0),
            Err(ThermoError::NegativeHeatMagnitude)
        );
        assert_eq!(HeatJoules::new(f64::NAN), Err(ThermoError::NonFiniteInput));
        assert_eq!(
            Kelvin::new(0.0),
            Err(ThermoError::NonPositiveAbsoluteTemperature)
        );
        assert_eq!(Kelvin::new(f64::INFINITY), Err(ThermoError::NonFiniteInput));
    }

    #[test]
    fn interval_corner_bounds_match_reference() {
        let heat_interval = HeatInterval::new(90.0, 110.0).unwrap();
        let hot = TemperatureInterval::new(345.0, 355.0).unwrap();
        let cold = TemperatureInterval::new(295.0, 305.0).unwrap();
        let entropy = entropy_generation_bounds(heat_interval, hot, cold).unwrap();

        let expected_lower = 90.0 * (1.0 / 305.0 - 1.0 / 345.0);
        let expected_upper = 110.0 * (1.0 / 295.0 - 1.0 / 355.0);
        close(entropy.lower_j_per_k(), expected_lower);
        close(entropy.upper_j_per_k(), expected_upper);

        let exergy = exergy_destruction_bounds(entropy, kelvin(300.0)).unwrap();
        close(exergy.lower_j(), 300.0 * expected_lower);
        close(exergy.upper_j(), 300.0 * expected_upper);
    }

    #[test]
    fn overlapping_temperature_intervals_do_not_claim_hot_to_cold_order() {
        let heat_interval = HeatInterval::new(1.0, 2.0).unwrap();
        let hot = TemperatureInterval::new(300.0, 360.0).unwrap();
        let cold = TemperatureInterval::new(290.0, 310.0).unwrap();
        assert_eq!(
            entropy_generation_bounds(heat_interval, hot, cold),
            Err(ThermoError::HeatDirectionNotGuaranteed)
        );
    }

    #[test]
    fn missing_or_duplicate_heat_ports_fail_closed() {
        let required = ["winding-to-housing", "housing-to-ambient"];
        assert_eq!(
            validate_heat_port_coverage(&required, &["winding-to-housing"]),
            Err(ThermoError::IncompleteBoundary)
        );
        assert_eq!(
            validate_heat_port_coverage(&required, &["winding-to-housing", "winding-to-housing",]),
            Err(ThermoError::IncompleteBoundary)
        );
        assert_eq!(validate_heat_port_coverage(&required, &required), Ok(()));
    }

    #[test]
    fn overflow_does_not_produce_a_finite_looking_result() {
        let result = TwoReservoirTransfer::new(
            heat(1.0e308),
            kelvin(2.0e-308),
            kelvin(1.0e-308),
            Some(kelvin(300.0)),
        )
        .expect("finite positive values")
        .evaluate();
        assert_eq!(result, Err(ThermoError::NonFiniteResult));
    }
}
