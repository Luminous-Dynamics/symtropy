//! Standalone 0D heat and seawater energy-balance verifier.
//!
//! This is an arithmetic/accounting verifier for one declared steady-state
//! interval. It verifies the IT heat split, chiller work, declared pump-work
//! contribution, seawater mass-flow heat pickup, and optional site-supplied
//! outlet-temperature / temperature-rise limits.
//!
//! It is deliberately NOT a heat-exchanger design tool, 3D CFD solver,
//! hydrodynamic or thermal-plume model, ecological impact assessment, or
//! environmental authorization. Inputs and limits are caller-supplied and are
//! not authenticated or validated against physical instruments by this crate.

use core::fmt;
use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoolingMode {
    DirectSeawater,
    PartialEconomiserPlusChiller,
    ChillerOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SiteThermalLimits {
    /// Optional site-authorized maximum discharge temperature (°C).
    pub maximum_discharge_temperature_c: Option<f64>,
    /// Optional site-authorized maximum intake-to-discharge temperature rise (°C).
    pub maximum_temperature_rise_c: Option<f64>,
    /// Explicit tolerance for comparing site temperature limits (°C).
    pub temperature_tolerance_c: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoolingCase {
    pub case_id: String,
    pub configuration_id: String,
    pub meter_boundary_id: String,
    pub intake_observation_id: String,
    pub discharge_observation_id: String,
    pub mode: CoolingMode,
    /// Average IT electrical load assumed to become heat at this boundary (W).
    pub it_heat_load_w: f64,
    /// IT heat transferred directly to seawater by an economiser/heat exchanger (W).
    pub direct_heat_to_seawater_w: f64,
    /// IT heat removed at the chiller evaporator (W thermal).
    pub chiller_evaporator_heat_w: f64,
    /// Chiller electric input (W).
    pub chiller_electric_input_w: f64,
    /// Water-loop pump electric input (W).
    pub water_pump_electric_input_w: f64,
    /// Other declared cooling-plant auxiliary electric input (W).
    pub other_cooling_auxiliary_electric_input_w: f64,
    /// Declared fraction of pump electric input ultimately entering the seawater
    /// heat-rejection boundary, in [0, 1].
    pub pump_heat_fraction_to_seawater: f64,
    /// Other declared heat entering the seawater boundary (W); no default is inferred.
    pub other_heat_to_seawater_w: f64,
    pub seawater_mass_flow_kg_per_s: f64,
    pub seawater_specific_heat_j_per_kg_k: f64,
    pub seawater_intake_temperature_c: f64,
    pub seawater_discharge_temperature_c: f64,
    pub interval_duration_seconds: f64,
    /// Absolute tolerance for IT-side and seawater-side heat balance (W).
    pub heat_balance_tolerance_w: f64,
    pub site_limits: SiteThermalLimits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoolingViolationCode {
    ItHeatBalance,
    SeawaterHeatBalance,
    MaximumDischargeTemperature,
    MaximumTemperatureRise,
}

impl fmt::Display for CoolingViolationCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ItHeatBalance => write!(f, "IT_HEAT_BALANCE"),
            Self::SeawaterHeatBalance => write!(f, "SEAWATER_HEAT_BALANCE"),
            Self::MaximumDischargeTemperature => write!(f, "MAX_DISCHARGE_TEMPERATURE"),
            Self::MaximumTemperatureRise => write!(f, "MAX_TEMPERATURE_RISE"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoolingViolation {
    pub code: CoolingViolationCode,
    pub observed: f64,
    pub allowed: f64,
    pub unit: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoolingBalanceReport {
    pub case_id: String,
    pub configuration_id: String,
    pub meter_boundary_id: String,
    pub mode: CoolingMode,
    pub passed: bool,
    pub it_heat_balance_residual_w: f64,
    pub seawater_heat_balance_residual_w: f64,
    pub it_heat_energy_kwh: f64,
    pub cooling_electric_energy_kwh: f64,
    pub seawater_heat_rejected_energy_kwh: f64,
    pub seawater_heat_pickup_w: f64,
    pub expected_seawater_heat_rejection_w: f64,
    pub seawater_temperature_rise_c: f64,
    pub chiller_cop: Option<f64>,
    pub violations: Vec<CoolingViolation>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerificationError {
    MissingIdentity(&'static str),
    InvalidQuantity { field: &'static str, value: f64 },
    InvalidCoolingMode { mode: CoolingMode, reason: &'static str },
    NumericalOverflow(&'static str),
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity(field) => write!(f, "{field} must not be empty"),
            Self::InvalidQuantity { field, value } => {
                write!(f, "{field} must be finite and in range; got {value}")
            }
            Self::InvalidCoolingMode { mode, reason } => {
                write!(f, "cooling mode {mode:?} is inconsistent: {reason}")
            }
            Self::NumericalOverflow(operation) => {
                write!(f, "non-finite arithmetic during {operation}")
            }
        }
    }
}

impl Error for VerificationError {}

fn require_id(field: &'static str, value: &str) -> Result<(), VerificationError> {
    if value.trim().is_empty() {
        return Err(VerificationError::MissingIdentity(field));
    }
    Ok(())
}

fn require_finite(field: &'static str, value: f64) -> Result<(), VerificationError> {
    if !value.is_finite() {
        return Err(VerificationError::InvalidQuantity { field, value });
    }
    Ok(())
}

fn require_non_negative(field: &'static str, value: f64) -> Result<(), VerificationError> {
    require_finite(field, value)?;
    if value < 0.0 {
        return Err(VerificationError::InvalidQuantity { field, value });
    }
    Ok(())
}

fn require_positive(field: &'static str, value: f64) -> Result<(), VerificationError> {
    require_finite(field, value)?;
    if value <= 0.0 {
        return Err(VerificationError::InvalidQuantity { field, value });
    }
    Ok(())
}

fn finite(value: f64, operation: &'static str) -> Result<f64, VerificationError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(VerificationError::NumericalOverflow(operation))
    }
}

fn validate(case: &CoolingCase) -> Result<(), VerificationError> {
    require_id("case ID", &case.case_id)?;
    require_id("configuration ID", &case.configuration_id)?;
    require_id("meter boundary ID", &case.meter_boundary_id)?;
    require_id("intake observation ID", &case.intake_observation_id)?;
    require_id("discharge observation ID", &case.discharge_observation_id)?;

    for (field, value) in [
        ("IT heat load W", case.it_heat_load_w),
        ("direct heat to seawater W", case.direct_heat_to_seawater_w),
        ("chiller evaporator heat W", case.chiller_evaporator_heat_w),
        ("chiller electric input W", case.chiller_electric_input_w),
        ("water-pump electric input W", case.water_pump_electric_input_w),
        ("other cooling auxiliary electric input W", case.other_cooling_auxiliary_electric_input_w),
        ("other heat to seawater W", case.other_heat_to_seawater_w),
        ("heat balance tolerance W", case.heat_balance_tolerance_w),
        ("temperature tolerance C", case.site_limits.temperature_tolerance_c),
    ] {
        require_non_negative(field, value)?;
    }
    require_positive("seawater mass flow kg/s", case.seawater_mass_flow_kg_per_s)?;
    require_positive(
        "seawater specific heat J/(kg K)",
        case.seawater_specific_heat_j_per_kg_k,
    )?;
    require_positive("interval duration seconds", case.interval_duration_seconds)?;
    require_finite("seawater intake temperature C", case.seawater_intake_temperature_c)?;
    require_finite(
        "seawater discharge temperature C",
        case.seawater_discharge_temperature_c,
    )?;
    require_finite(
        "pump heat fraction to seawater",
        case.pump_heat_fraction_to_seawater,
    )?;
    if !(0.0..=1.0).contains(&case.pump_heat_fraction_to_seawater) {
        return Err(VerificationError::InvalidQuantity {
            field: "pump heat fraction to seawater",
            value: case.pump_heat_fraction_to_seawater,
        });
    }

    let temperature_rise =
        case.seawater_discharge_temperature_c - case.seawater_intake_temperature_c;
    if !temperature_rise.is_finite() || temperature_rise < 0.0 {
        return Err(VerificationError::InvalidQuantity {
            field: "seawater temperature rise C",
            value: temperature_rise,
        });
    }

    if let Some(value) = case.site_limits.maximum_discharge_temperature_c {
        require_finite("maximum discharge temperature C", value)?;
    }
    if let Some(value) = case.site_limits.maximum_temperature_rise_c {
        require_non_negative("maximum temperature rise C", value)?;
    }

    if case.it_heat_load_w <= 0.0 {
        return Err(VerificationError::InvalidQuantity {
            field: "IT heat load W",
            value: case.it_heat_load_w,
        });
    }

    let direct = case.direct_heat_to_seawater_w;
    let evaporator = case.chiller_evaporator_heat_w;
    let chiller_power = case.chiller_electric_input_w;
    match case.mode {
        CoolingMode::DirectSeawater => {
            if direct <= 0.0 || evaporator != 0.0 || chiller_power != 0.0 {
                return Err(VerificationError::InvalidCoolingMode {
                    mode: case.mode,
                    reason: "requires direct heat transfer and zero chiller duty/input",
                });
            }
        }
        CoolingMode::PartialEconomiserPlusChiller => {
            if direct <= 0.0 || evaporator <= 0.0 || chiller_power <= 0.0 {
                return Err(VerificationError::InvalidCoolingMode {
                    mode: case.mode,
                    reason: "requires positive direct transfer, chiller duty and chiller input",
                });
            }
        }
        CoolingMode::ChillerOnly => {
            if direct != 0.0 || evaporator <= 0.0 || chiller_power <= 0.0 {
                return Err(VerificationError::InvalidCoolingMode {
                    mode: case.mode,
                    reason: "requires zero direct transfer and positive chiller duty/input",
                });
            }
        }
    }
    if (evaporator > 0.0) != (chiller_power > 0.0) {
        return Err(VerificationError::InvalidCoolingMode {
            mode: case.mode,
            reason: "chiller heat duty and electric input must both be zero or both be positive",
        });
    }

    Ok(())
}

/// Verify one steady-state interval against a declared cooling boundary.
///
/// IT-side balance:
/// IT heat load = direct heat transfer + chiller evaporator heat.
///
/// Seawater-side balance:
/// m_dot * cp * (T_out - T_in) = direct heat + chiller evaporator heat
/// + chiller electric input + declared pump heat fraction + other heat.
///
/// This assumes no thermal storage change within the interval, no unlisted
/// heat path, and that all supplied measurements refer to compatible
/// synchronized boundaries. Those assumptions are caller declarations, not
/// inferred physical facts.
pub fn verify_cooling_balance(
    case: &CoolingCase,
) -> Result<CoolingBalanceReport, VerificationError> {
    validate(case)?;

    let it_removed_w = finite(
        case.direct_heat_to_seawater_w + case.chiller_evaporator_heat_w,
        "IT heat removal sum",
    )?;
    let it_heat_balance_residual_w = finite(
        it_removed_w - case.it_heat_load_w,
        "IT heat balance residual",
    )?;

    let pump_heat_to_seawater_w = finite(
        case.water_pump_electric_input_w * case.pump_heat_fraction_to_seawater,
        "pump heat to seawater",
    )?;
    let expected_seawater_heat_rejection_w = finite(
        case.direct_heat_to_seawater_w
            + case.chiller_evaporator_heat_w
            + case.chiller_electric_input_w
            + pump_heat_to_seawater_w
            + case.other_heat_to_seawater_w,
        "expected seawater heat rejection",
    )?;

    let delta_t_c = finite(
        case.seawater_discharge_temperature_c - case.seawater_intake_temperature_c,
        "seawater temperature rise",
    )?;
    let seawater_heat_pickup_w = finite(
        case.seawater_mass_flow_kg_per_s
            * case.seawater_specific_heat_j_per_kg_k
            * delta_t_c,
        "seawater heat pickup",
    )?;
    let seawater_heat_balance_residual_w = finite(
        seawater_heat_pickup_w - expected_seawater_heat_rejection_w,
        "seawater heat balance residual",
    )?;

    let mut violations = Vec::new();
    if it_heat_balance_residual_w.abs() > case.heat_balance_tolerance_w {
        violations.push(CoolingViolation {
            code: CoolingViolationCode::ItHeatBalance,
            observed: it_removed_w,
            allowed: case.it_heat_load_w,
            unit: "W",
        });
    }
    if seawater_heat_balance_residual_w.abs() > case.heat_balance_tolerance_w {
        violations.push(CoolingViolation {
            code: CoolingViolationCode::SeawaterHeatBalance,
            observed: seawater_heat_pickup_w,
            allowed: expected_seawater_heat_rejection_w,
            unit: "W",
        });
    }
    if let Some(limit) = case.site_limits.maximum_discharge_temperature_c {
        if case.seawater_discharge_temperature_c
            > limit + case.site_limits.temperature_tolerance_c
        {
            violations.push(CoolingViolation {
                code: CoolingViolationCode::MaximumDischargeTemperature,
                observed: case.seawater_discharge_temperature_c,
                allowed: limit,
                unit: "degC",
            });
        }
    }
    if let Some(limit) = case.site_limits.maximum_temperature_rise_c {
        if delta_t_c > limit + case.site_limits.temperature_tolerance_c {
            violations.push(CoolingViolation {
                code: CoolingViolationCode::MaximumTemperatureRise,
                observed: delta_t_c,
                allowed: limit,
                unit: "degC",
            });
        }
    }

    let interval_hours = case.interval_duration_seconds / 3600.0;
    let it_heat_energy_kwh =
        finite(case.it_heat_load_w * interval_hours / 1000.0, "IT heat energy")?;
    let cooling_electric_energy_kwh = finite(
        (case.chiller_electric_input_w
            + case.water_pump_electric_input_w
            + case.other_cooling_auxiliary_electric_input_w)
            * interval_hours
            / 1000.0,
        "cooling electric energy",
    )?;
    let seawater_heat_rejected_energy_kwh = finite(
        seawater_heat_pickup_w * interval_hours / 1000.0,
        "seawater rejected heat energy",
    )?;
    let chiller_cop = if case.chiller_electric_input_w > 0.0 {
        Some(finite(
            case.chiller_evaporator_heat_w / case.chiller_electric_input_w,
            "chiller COP",
        )?)
    } else {
        None
    };

    Ok(CoolingBalanceReport {
        case_id: case.case_id.clone(),
        configuration_id: case.configuration_id.clone(),
        meter_boundary_id: case.meter_boundary_id.clone(),
        mode: case.mode,
        passed: violations.is_empty(),
        it_heat_balance_residual_w,
        seawater_heat_balance_residual_w,
        it_heat_energy_kwh,
        cooling_electric_energy_kwh,
        seawater_heat_rejected_energy_kwh,
        seawater_heat_pickup_w,
        expected_seawater_heat_rejection_w,
        seawater_temperature_rise_c: delta_t_c,
        chiller_cop,
        violations,
        claim_ceiling: "0D steady-state arithmetic under caller-supplied synchronized boundaries and site limits only; not heat-exchanger design, CFD, plume prediction, ecological assessment, environmental compliance or field qualification.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balanced_case(mode: CoolingMode) -> CoolingCase {
        let (direct, evaporator, chiller_power) = match mode {
            CoolingMode::DirectSeawater => (100_000.0, 0.0, 0.0),
            CoolingMode::PartialEconomiserPlusChiller => (60_000.0, 40_000.0, 10_000.0),
            CoolingMode::ChillerOnly => (0.0, 100_000.0, 25_000.0),
        };
        let pump = 5_000.0;
        let pump_fraction = 1.0;
        let other = 0.0;
        let expected = direct + evaporator + chiller_power + pump * pump_fraction + other;
        let flow = 5.0;
        let cp = 4180.0;
        let intake = 20.0;
        let delta_t = expected / (flow * cp);
        CoolingCase {
            case_id: "synthetic-cooling-case".to_owned(),
            configuration_id: "synthetic-three-mode-v0".to_owned(),
            meter_boundary_id: "synthetic-cooling-boundary".to_owned(),
            intake_observation_id: "synthetic-intake-observation".to_owned(),
            discharge_observation_id: "synthetic-discharge-observation".to_owned(),
            mode,
            it_heat_load_w: 100_000.0,
            direct_heat_to_seawater_w: direct,
            chiller_evaporator_heat_w: evaporator,
            chiller_electric_input_w: chiller_power,
            water_pump_electric_input_w: pump,
            other_cooling_auxiliary_electric_input_w: 0.0,
            pump_heat_fraction_to_seawater: pump_fraction,
            other_heat_to_seawater_w: other,
            seawater_mass_flow_kg_per_s: flow,
            seawater_specific_heat_j_per_kg_k: cp,
            seawater_intake_temperature_c: intake,
            seawater_discharge_temperature_c: intake + delta_t,
            interval_duration_seconds: 900.0,
            heat_balance_tolerance_w: 1e-6,
            site_limits: SiteThermalLimits {
                maximum_discharge_temperature_c: None,
                maximum_temperature_rise_c: None,
                temperature_tolerance_c: 1e-9,
            },
        }
    }

    #[test]
    fn direct_seawater_case_closes_both_heat_balances() {
        let report = verify_cooling_balance(&balanced_case(CoolingMode::DirectSeawater)).unwrap();
        assert!(report.passed, "{:?}", report.violations);
        assert!(report.chiller_cop.is_none());
        assert_eq!(report.cooling_electric_energy_kwh, 1.25);
    }

    #[test]
    fn partial_economiser_case_closes_both_heat_balances() {
        let report =
            verify_cooling_balance(&balanced_case(CoolingMode::PartialEconomiserPlusChiller))
                .unwrap();
        assert!(report.passed, "{:?}", report.violations);
        assert_eq!(report.chiller_cop, Some(4.0));
    }

    #[test]
    fn chiller_only_case_closes_both_heat_balances() {
        let report = verify_cooling_balance(&balanced_case(CoolingMode::ChillerOnly)).unwrap();
        assert!(report.passed, "{:?}", report.violations);
        assert_eq!(report.chiller_cop, Some(4.0));
    }

    #[test]
    fn it_heat_mismatch_is_a_reported_violation() {
        let mut case = balanced_case(CoolingMode::DirectSeawater);
        case.it_heat_load_w += 100.0;
        let report = verify_cooling_balance(&case).unwrap();
        assert!(!report.passed);
        assert!(report.violations.iter().any(|v| v.code == CoolingViolationCode::ItHeatBalance));
    }

    #[test]
    fn seawater_heat_mismatch_is_a_reported_violation() {
        let mut case = balanced_case(CoolingMode::DirectSeawater);
        case.seawater_discharge_temperature_c += 0.1;
        let report = verify_cooling_balance(&case).unwrap();
        assert!(!report.passed);
        assert!(report.violations.iter().any(|v| v.code == CoolingViolationCode::SeawaterHeatBalance));
    }

    #[test]
    fn site_discharge_temperature_limit_is_not_universally_hardcoded() {
        let mut case = balanced_case(CoolingMode::DirectSeawater);
        case.site_limits.maximum_discharge_temperature_c = Some(22.0);
        let report = verify_cooling_balance(&case).unwrap();
        assert!(!report.passed);
        assert!(report.violations.iter().any(|v| v.code == CoolingViolationCode::MaximumDischargeTemperature));
    }

    #[test]
    fn site_temperature_rise_limit_is_enforced_separately() {
        let mut case = balanced_case(CoolingMode::DirectSeawater);
        case.site_limits.maximum_temperature_rise_c = Some(4.0);
        let report = verify_cooling_balance(&case).unwrap();
        assert!(!report.passed);
        assert!(report.violations.iter().any(|v| v.code == CoolingViolationCode::MaximumTemperatureRise));
    }

    #[test]
    fn pump_heat_fraction_is_explicit_and_affects_water_balance() {
        let mut case = balanced_case(CoolingMode::DirectSeawater);
        case.pump_heat_fraction_to_seawater = 0.0;
        let report = verify_cooling_balance(&case).unwrap();
        assert!(!report.passed);
        assert!(report.violations.iter().any(|v| v.code == CoolingViolationCode::SeawaterHeatBalance));
    }

    #[test]
    fn wrong_mode_semantics_fail_closed() {
        let mut case = balanced_case(CoolingMode::DirectSeawater);
        case.chiller_electric_input_w = 10_000.0;
        assert!(matches!(
            verify_cooling_balance(&case),
            Err(VerificationError::InvalidCoolingMode { .. })
        ));
    }

    #[test]
    fn zero_or_missing_flow_is_invalid() {
        let mut case = balanced_case(CoolingMode::DirectSeawater);
        case.seawater_mass_flow_kg_per_s = 0.0;
        assert!(matches!(
            verify_cooling_balance(&case),
            Err(VerificationError::InvalidQuantity { .. })
        ));
    }

    #[test]
    fn non_finite_temperature_is_invalid() {
        let mut case = balanced_case(CoolingMode::DirectSeawater);
        case.seawater_intake_temperature_c = f64::NAN;
        assert!(matches!(
            verify_cooling_balance(&case),
            Err(VerificationError::InvalidQuantity { .. })
        ));
    }

    #[test]
    fn pump_heat_fraction_outside_unit_interval_is_invalid() {
        let mut case = balanced_case(CoolingMode::DirectSeawater);
        case.pump_heat_fraction_to_seawater = 1.01;
        assert!(matches!(
            verify_cooling_balance(&case),
            Err(VerificationError::InvalidQuantity { .. })
        ));
    }

    #[test]
    fn negative_temperature_rise_is_invalid() {
        let mut case = balanced_case(CoolingMode::DirectSeawater);
        case.seawater_discharge_temperature_c = case.seawater_intake_temperature_c - 0.1;
        assert!(matches!(
            verify_cooling_balance(&case),
            Err(VerificationError::InvalidQuantity { .. })
        ));
    }

    #[test]
    fn interval_energy_is_integrated_in_kwh() {
        let report = verify_cooling_balance(&balanced_case(CoolingMode::DirectSeawater)).unwrap();
        assert!((report.it_heat_energy_kwh - 25.0).abs() < 1e-12);
        assert!((report.cooling_electric_energy_kwh - 1.25).abs() < 1e-12);
    }

    #[test]
    fn identifiers_are_required_for_all_sensor_boundaries() {
        let mut case = balanced_case(CoolingMode::DirectSeawater);
        case.intake_observation_id.clear();
        assert!(matches!(
            verify_cooling_balance(&case),
            Err(VerificationError::MissingIdentity(_))
        ));
    }
}
