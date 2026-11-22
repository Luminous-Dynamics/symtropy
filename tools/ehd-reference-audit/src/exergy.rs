//! Manufactured neutral ideal-gas flow-exergy and control-volume reference.
//!
//! Scope is intentionally limited to a fixed-composition, calorically perfect
//! neutral gas and already-reduced boundary exergy rates. This is not a plasma
//! exergy model: charged species, chemistry, field energy, radiation and
//! non-equilibrium entropy production require separately qualified closures.

use std::error::Error;
use std::fmt;

/// Neutral ideal-gas thermodynamic state at a flow boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IdealGasFlowState {
    pub temperature_k: f64,
    pub pressure_pa: f64,
    pub speed_m_s: f64,
    pub elevation_m: f64,
}

/// Declared environmental dead state for physical flow exergy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeadState {
    pub temperature_k: f64,
    pub pressure_pa: f64,
    pub elevation_m: f64,
}

/// Heat transfer rate, positive into the control volume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatExergyTransfer {
    pub heat_rate_into_cv_w: f64,
    pub boundary_temperature_k: f64,
}

/// Reduced control-volume exergy balance inputs.
///
/// Mass-flow exergy rates are signed physical flow-exergy fluxes in the
/// declared inlet/outlet directions. They may be negative for a state below
/// the reference pressure; do not clamp them to zero.
#[derive(Clone, Debug, PartialEq)]
pub struct ExergyBalanceInput {
    pub dead_state_temperature_k: f64,
    /// Electrical/mechanical work exergy supplied across the boundary.
    pub work_exergy_input_w: f64,
    pub heat_transfers: Vec<HeatExergyTransfer>,
    pub mass_flow_exergy_in_w: f64,
    pub mass_flow_exergy_out_w: f64,
    /// Other outgoing exergy, e.g. measured radiation or exported work.
    pub other_exergy_outputs_w: f64,
    /// dB_CV/dt; positive means exergy accumulation in the control volume.
    pub stored_exergy_accumulation_w: f64,
}

/// Scale-aware tolerance for the exergy balance residual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExergyBalanceTolerance {
    pub absolute_w: f64,
    pub relative: f64,
}

/// Reconciled exergy balance and second-law disposition.
#[derive(Clone, Debug, PartialEq)]
pub struct ExergyBalanceReceipt {
    pub work_exergy_input_w: f64,
    pub heat_exergy_input_w: f64,
    pub mass_flow_exergy_in_w: f64,
    pub mass_flow_exergy_out_w: f64,
    pub other_exergy_outputs_w: f64,
    pub stored_exergy_accumulation_w: f64,
    pub total_exergy_input_w: f64,
    pub total_exergy_output_w: f64,
    /// Input - output - storage accumulation.
    pub balance_residual_w: f64,
    /// T0 * S_gen, set to zero only when a negative residual is within the
    /// declared numerical tolerance. The signed residual is always retained.
    pub exergy_destruction_w: f64,
    pub entropy_generation_w_per_k: f64,
    pub allowed_roundoff_bound_w: f64,
    pub negative_residual_within_roundoff_bound: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExergyError {
    InvalidDeadState,
    InvalidFlowState,
    InvalidGasProperties,
    InvalidGravity,
    InvalidHeatBoundaryTemperature { index: usize },
    NonFiniteInput,
    InvalidTolerance,
    NonFiniteDerivedValue,
    SecondLawViolation { residual_w: f64, allowed_w: f64 },
}

impl fmt::Display for ExergyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDeadState => write!(f, "dead-state temperature and pressure must be finite and positive"),
            Self::InvalidFlowState => write!(f, "ideal-gas flow state has invalid temperature or pressure"),
            Self::InvalidGasProperties => write!(f, "ideal-gas cp and specific gas constant must be finite, positive, and cp > R"),
            Self::InvalidGravity => write!(f, "gravitational acceleration must be finite and non-negative"),
            Self::InvalidHeatBoundaryTemperature { index } => write!(f, "heat boundary temperature at index {index} must be finite and positive"),
            Self::NonFiniteInput => write!(f, "exergy balance contains a non-finite input"),
            Self::InvalidTolerance => write!(f, "exergy tolerances must be finite and non-negative"),
            Self::NonFiniteDerivedValue => write!(f, "exergy calculation produced a non-finite value"),
            Self::SecondLawViolation { residual_w, allowed_w } => write!(
                f,
                "negative exergy destruction violates the second law: residual={residual_w} W, allowed roundoff={allowed_w} W"
            ),
        }
    }
}

impl Error for ExergyError {}

/// Stable evaluation of x - ln(1+x), non-negative for x > -1.
fn x_minus_ln_one_plus_x(x: f64) -> f64 {
    if x.abs() < 1.0e-4 {
        // Alternating Taylor expansion; factor x^2 to avoid cancellation.
        let polynomial = 0.5
            + x * (-1.0 / 3.0
                + x * (1.0 / 4.0
                    + x * (-1.0 / 5.0
                        + x * (1.0 / 6.0
                            + x * (-1.0 / 7.0 + x * (1.0 / 8.0))))));
        x * x * polynomial
    } else {
        x - x.ln_1p()
    }
}

/// Specific physical flow exergy (J/kg) for a fixed-composition, calorically
/// perfect neutral ideal gas relative to a declared dead state.
///
/// b = cp[(T-T0)-T0 ln(T/T0)] + R T0 ln(p/p0)
///     + speed^2/2 + g(z-z0)
///
/// Chemical exergy and composition changes are deliberately unsupported.
pub fn physical_flow_exergy_ideal_gas(
    state: IdealGasFlowState,
    dead_state: DeadState,
    cp_j_per_kg_k: f64,
    gas_constant_j_per_kg_k: f64,
    gravity_m_s2: f64,
) -> Result<f64, ExergyError> {
    if !dead_state.temperature_k.is_finite()
        || dead_state.temperature_k <= 0.0
        || !dead_state.pressure_pa.is_finite()
        || dead_state.pressure_pa <= 0.0
    {
        return Err(ExergyError::InvalidDeadState);
    }
    if !state.temperature_k.is_finite()
        || state.temperature_k <= 0.0
        || !state.pressure_pa.is_finite()
        || state.pressure_pa <= 0.0
        || !state.speed_m_s.is_finite()
        || !state.elevation_m.is_finite()
    {
        return Err(ExergyError::InvalidFlowState);
    }
    if !cp_j_per_kg_k.is_finite()
        || !gas_constant_j_per_kg_k.is_finite()
        || cp_j_per_kg_k <= 0.0
        || gas_constant_j_per_kg_k <= 0.0
        || cp_j_per_kg_k <= gas_constant_j_per_kg_k
    {
        return Err(ExergyError::InvalidGasProperties);
    }
    if !gravity_m_s2.is_finite() || gravity_m_s2 < 0.0 {
        return Err(ExergyError::InvalidGravity);
    }

    let t0 = dead_state.temperature_k;
    let thermal_fraction = (state.temperature_k - t0) / t0;
    let thermal_exergy = cp_j_per_kg_k * t0 * x_minus_ln_one_plus_x(thermal_fraction);
    let pressure_exergy = gas_constant_j_per_kg_k
        * t0
        * (state.pressure_pa / dead_state.pressure_pa).ln();
    let kinetic_exergy = 0.5 * state.speed_m_s * state.speed_m_s;
    let potential_exergy =
        gravity_m_s2 * (state.elevation_m - dead_state.elevation_m);
    let result = thermal_exergy + pressure_exergy + kinetic_exergy + potential_exergy;

    if !result.is_finite() {
        return Err(ExergyError::NonFiniteDerivedValue);
    }
    Ok(result)
}

/// Exergy transferred with heat into the control volume (W).
pub fn heat_exergy_transfer_w(
    heat_rate_into_cv_w: f64,
    boundary_temperature_k: f64,
    dead_state_temperature_k: f64,
) -> Result<f64, ExergyError> {
    if !heat_rate_into_cv_w.is_finite()
        || !boundary_temperature_k.is_finite()
        || !dead_state_temperature_k.is_finite()
    {
        return Err(ExergyError::NonFiniteInput);
    }
    if boundary_temperature_k <= 0.0 || dead_state_temperature_k <= 0.0 {
        return Err(ExergyError::InvalidDeadState);
    }
    let value = heat_rate_into_cv_w
        * (1.0 - dead_state_temperature_k / boundary_temperature_k);
    if !value.is_finite() {
        return Err(ExergyError::NonFiniteDerivedValue);
    }
    Ok(value)
}

/// Reconcile a reduced control-volume exergy balance.
///
/// dB/dt = W_ex,in + Q_ex,in + B_mass,in - B_mass,out
///         - B_other,out - T0*S_gen
///
/// A materially negative destruction residual is rejected. A small negative
/// residual within the declared absolute-plus-relative numerical bound is
/// retained in the receipt and assigned zero qualified destruction; it is not
/// silently discarded.
pub fn reconcile_exergy_balance(
    input: &ExergyBalanceInput,
    tolerance: ExergyBalanceTolerance,
) -> Result<ExergyBalanceReceipt, ExergyError> {
    let scalar_inputs = [
        input.dead_state_temperature_k,
        input.work_exergy_input_w,
        input.mass_flow_exergy_in_w,
        input.mass_flow_exergy_out_w,
        input.other_exergy_outputs_w,
        input.stored_exergy_accumulation_w,
        tolerance.absolute_w,
        tolerance.relative,
    ];
    if !scalar_inputs.iter().all(|value| value.is_finite()) {
        return Err(ExergyError::NonFiniteInput);
    }
    if input.dead_state_temperature_k <= 0.0
        || input.work_exergy_input_w < 0.0
        || input.other_exergy_outputs_w < 0.0
    {
        return Err(ExergyError::InvalidDeadState);
    }
    if tolerance.absolute_w < 0.0 || tolerance.relative < 0.0 {
        return Err(ExergyError::InvalidTolerance);
    }

    let mut heat_exergy_input_w = 0.0_f64;
    for (index, transfer) in input.heat_transfers.iter().enumerate() {
        let transfer_exergy = heat_exergy_transfer_w(
            transfer.heat_rate_into_cv_w,
            transfer.boundary_temperature_k,
            input.dead_state_temperature_k,
        )
        .map_err(|error| match error {
            ExergyError::InvalidDeadState => {
                ExergyError::InvalidHeatBoundaryTemperature { index }
            }
            other => other,
        })?;
        heat_exergy_input_w += transfer_exergy;
    }

    let total_exergy_input_w = input.work_exergy_input_w
        + heat_exergy_input_w
        + input.mass_flow_exergy_in_w;
    let total_exergy_output_w =
        input.mass_flow_exergy_out_w + input.other_exergy_outputs_w;
    let balance_residual_w = total_exergy_input_w
        - total_exergy_output_w
        - input.stored_exergy_accumulation_w;
    let scale_w = input.work_exergy_input_w.abs()
        + heat_exergy_input_w.abs()
        + input.mass_flow_exergy_in_w.abs()
        + input.mass_flow_exergy_out_w.abs()
        + input.other_exergy_outputs_w.abs()
        + input.stored_exergy_accumulation_w.abs();
    let allowed_roundoff_bound_w = tolerance.absolute_w + tolerance.relative * scale_w;

    if !heat_exergy_input_w.is_finite()
        || !total_exergy_input_w.is_finite()
        || !total_exergy_output_w.is_finite()
        || !balance_residual_w.is_finite()
        || !allowed_roundoff_bound_w.is_finite()
    {
        return Err(ExergyError::NonFiniteDerivedValue);
    }
    if balance_residual_w < -allowed_roundoff_bound_w {
        return Err(ExergyError::SecondLawViolation {
            residual_w: balance_residual_w,
            allowed_w: allowed_roundoff_bound_w,
        });
    }

    let negative_residual_within_roundoff_bound = balance_residual_w < 0.0;
    let exergy_destruction_w = balance_residual_w.max(0.0);
    let entropy_generation_w_per_k =
        exergy_destruction_w / input.dead_state_temperature_k;

    Ok(ExergyBalanceReceipt {
        work_exergy_input_w: input.work_exergy_input_w,
        heat_exergy_input_w,
        mass_flow_exergy_in_w: input.mass_flow_exergy_in_w,
        mass_flow_exergy_out_w: input.mass_flow_exergy_out_w,
        other_exergy_outputs_w: input.other_exergy_outputs_w,
        stored_exergy_accumulation_w: input.stored_exergy_accumulation_w,
        total_exergy_input_w,
        total_exergy_output_w,
        balance_residual_w,
        exergy_destruction_w,
        entropy_generation_w_per_k,
        allowed_roundoff_bound_w,
        negative_residual_within_roundoff_bound,
    })
}

/// Manufactured sanity checks for the fixed-composition ideal-gas reference.
pub fn reference_self_check() -> Result<(), ExergyError> {
    let dead = DeadState {
        temperature_k: 300.0,
        pressure_pa: 101_325.0,
        elevation_m: 0.0,
    };
    let dead_state_flow = IdealGasFlowState {
        temperature_k: 300.0,
        pressure_pa: 101_325.0,
        speed_m_s: 0.0,
        elevation_m: 0.0,
    };
    let at_dead_state =
        physical_flow_exergy_ideal_gas(dead_state_flow, dead, 1005.0, 287.0, 9.80665)?;
    if at_dead_state.abs() > 1.0e-12 {
        return Err(ExergyError::NonFiniteDerivedValue);
    }

    let heat_exergy =
        heat_exergy_transfer_w(1000.0, 600.0, dead.temperature_k)?;
    if (heat_exergy - 500.0).abs() > 1.0e-12 {
        return Err(ExergyError::NonFiniteDerivedValue);
    }

    let reversible = ExergyBalanceInput {
        dead_state_temperature_k: 300.0,
        work_exergy_input_w: 1000.0,
        heat_transfers: Vec::new(),
        mass_flow_exergy_in_w: 0.0,
        mass_flow_exergy_out_w: 1000.0,
        other_exergy_outputs_w: 0.0,
        stored_exergy_accumulation_w: 0.0,
    };
    let zero_destroyed = reconcile_exergy_balance(
        &reversible,
        ExergyBalanceTolerance {
            absolute_w: 1.0e-10,
            relative: 1.0e-12,
        },
    )?;
    if zero_destroyed.exergy_destruction_w != 0.0
        || zero_destroyed.entropy_generation_w_per_k != 0.0
    {
        return Err(ExergyError::NonFiniteDerivedValue);
    }

    let irreversible = ExergyBalanceInput {
        mass_flow_exergy_out_w: 900.0,
        ..reversible.clone()
    };
    let positive_destroyed = reconcile_exergy_balance(
        &irreversible,
        ExergyBalanceTolerance {
            absolute_w: 1.0e-10,
            relative: 1.0e-12,
        },
    )?;
    if (positive_destroyed.exergy_destruction_w - 100.0).abs() > 1.0e-12
        || (positive_destroyed.entropy_generation_w_per_k - (1.0 / 3.0)).abs()
            > 1.0e-12
    {
        return Err(ExergyError::NonFiniteDerivedValue);
    }

    let impossible = ExergyBalanceInput {
        mass_flow_exergy_out_w: 1100.0,
        ..reversible
    };
    if !matches!(
        reconcile_exergy_balance(
            &impossible,
            ExergyBalanceTolerance {
                absolute_w: 1.0e-10,
                relative: 1.0e-12,
            }
        ),
        Err(ExergyError::SecondLawViolation { .. })
    ) {
        return Err(ExergyError::NonFiniteDerivedValue);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dead_state() -> DeadState {
        DeadState {
            temperature_k: 300.0,
            pressure_pa: 101_325.0,
            elevation_m: 0.0,
        }
    }

    #[test]
    fn flow_exergy_is_zero_at_dead_state() {
        let state = IdealGasFlowState {
            temperature_k: 300.0,
            pressure_pa: 101_325.0,
            speed_m_s: 0.0,
            elevation_m: 0.0,
        };
        assert!(physical_flow_exergy_ideal_gas(state, dead_state(), 1005.0, 287.0, 9.80665)
            .unwrap().abs() < 1.0e-12);
    }

    #[test]
    fn flow_exergy_increases_with_kinetic_and_potential_terms() {
        let base = IdealGasFlowState {
            temperature_k: 300.0,
            pressure_pa: 101_325.0,
            speed_m_s: 0.0,
            elevation_m: 0.0,
        };
        let moving = IdealGasFlowState { speed_m_s: 10.0, ..base };
        let elevated = IdealGasFlowState { elevation_m: 2.0, ..base };
        let b0 = physical_flow_exergy_ideal_gas(base, dead_state(), 1005.0, 287.0, 9.80665).unwrap();
        let bv = physical_flow_exergy_ideal_gas(moving, dead_state(), 1005.0, 287.0, 9.80665).unwrap();
        let bz = physical_flow_exergy_ideal_gas(elevated, dead_state(), 1005.0, 287.0, 9.80665).unwrap();
        assert!((bv - b0 - 50.0).abs() < 1.0e-12);
        assert!((bz - b0 - 19.6133).abs() < 1.0e-12);
    }

    #[test]
    fn sub_dead_state_pressure_flow_exergy_remains_signed() {
        let state = IdealGasFlowState {
            temperature_k: 300.0,
            pressure_pa: 0.5 * 101_325.0,
            speed_m_s: 0.0,
            elevation_m: 0.0,
        };
        let value = physical_flow_exergy_ideal_gas(state, dead_state(), 1005.0, 287.0, 9.80665).unwrap();
        let expected = 287.0 * 300.0 * (0.5_f64).ln();
        assert!((value - expected).abs() < 1.0e-10);
        assert!(value < 0.0);
    }

    #[test]
    fn heat_exergy_uses_dead_state_temperature_and_direction() {
        assert_eq!(heat_exergy_transfer_w(1000.0, 600.0, 300.0).unwrap(), 500.0);
        assert_eq!(heat_exergy_transfer_w(1000.0, 200.0, 300.0).unwrap(), -500.0);
    }

    #[test]
    fn reversible_balance_has_zero_destruction() {
        let input = ExergyBalanceInput {
            dead_state_temperature_k: 300.0,
            work_exergy_input_w: 1000.0,
            heat_transfers: Vec::new(),
            mass_flow_exergy_in_w: 0.0,
            mass_flow_exergy_out_w: 1000.0,
            other_exergy_outputs_w: 0.0,
            stored_exergy_accumulation_w: 0.0,
        };
        let report = reconcile_exergy_balance(&input, ExergyBalanceTolerance {
            absolute_w: 1.0e-10, relative: 1.0e-12,
        }).unwrap();
        assert_eq!(report.exergy_destruction_w, 0.0);
        assert_eq!(report.entropy_generation_w_per_k, 0.0);
    }

    #[test]
    fn irreversible_balance_reports_exergy_destruction_and_entropy_generation() {
        let input = ExergyBalanceInput {
            dead_state_temperature_k: 300.0,
            work_exergy_input_w: 1000.0,
            heat_transfers: Vec::new(),
            mass_flow_exergy_in_w: 0.0,
            mass_flow_exergy_out_w: 900.0,
            other_exergy_outputs_w: 0.0,
            stored_exergy_accumulation_w: 0.0,
        };
        let report = reconcile_exergy_balance(&input, ExergyBalanceTolerance {
            absolute_w: 1.0e-10, relative: 1.0e-12,
        }).unwrap();
        assert!((report.exergy_destruction_w - 100.0).abs() < 1.0e-12);
        assert!((report.entropy_generation_w_per_k - (1.0 / 3.0)).abs() < 1.0e-12);
    }

    #[test]
    fn materially_negative_exergy_destruction_is_rejected() {
        let input = ExergyBalanceInput {
            dead_state_temperature_k: 300.0,
            work_exergy_input_w: 1000.0,
            heat_transfers: Vec::new(),
            mass_flow_exergy_in_w: 0.0,
            mass_flow_exergy_out_w: 1100.0,
            other_exergy_outputs_w: 0.0,
            stored_exergy_accumulation_w: 0.0,
        };
        assert!(matches!(
            reconcile_exergy_balance(&input, ExergyBalanceTolerance {
                absolute_w: 1.0e-10, relative: 1.0e-12,
            }),
            Err(ExergyError::SecondLawViolation { .. })
        ));
    }

    #[test]
    fn tiny_negative_residual_is_retained_and_marked_as_roundoff() {
        let input = ExergyBalanceInput {
            dead_state_temperature_k: 300.0,
            work_exergy_input_w: 1000.0,
            heat_transfers: Vec::new(),
            mass_flow_exergy_in_w: 0.0,
            mass_flow_exergy_out_w: 1000.0 + 5.0e-11,
            other_exergy_outputs_w: 0.0,
            stored_exergy_accumulation_w: 0.0,
        };
        let report = reconcile_exergy_balance(&input, ExergyBalanceTolerance {
            absolute_w: 1.0e-10, relative: 1.0e-12,
        }).unwrap();
        assert!(report.negative_residual_within_roundoff_bound);
        assert!(report.balance_residual_w < 0.0);
        assert_eq!(report.exergy_destruction_w, 0.0);
    }
}
