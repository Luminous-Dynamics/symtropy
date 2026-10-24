//! Small analytic reference primitives for the Organ Engineering Transport Benchmarks.
//!
//! This crate is a numerical benchmark oracle, not a vascular-network solver,
//! biological model, or clinical tool. Tube flow uses the Hagen–Poiseuille
//! equation only under its stated assumptions: steady laminar Newtonian flow
//! through a rigid circular tube.

use std::f64::consts::PI;

/// Fully specified input for a rigid circular tube under laminar Newtonian flow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TubeSpec {
    /// Tube length in metres. Must be finite and greater than zero.
    pub length_m: f64,
    /// Tube inner radius in metres. Must be finite and greater than zero.
    pub radius_m: f64,
    /// Dynamic viscosity in pascal-seconds. Must be finite and greater than zero.
    pub dynamic_viscosity_pa_s: f64,
    /// Pressure drop in pascals, upstream minus downstream. May be negative.
    pub pressure_drop_pa: f64,
}

/// Reference result for one tube.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TubeResult {
    /// Hydraulic resistance in Pa·s/m³.
    pub hydraulic_resistance_pa_s_per_m3: f64,
    /// Signed volumetric flow in m³/s. Sign follows `pressure_drop_pa`.
    pub volume_flow_m3_per_s: f64,
}

/// Explicit input/numerical errors; invalid values are never silently repaired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowError {
    NonFiniteInput(&'static str),
    NonPositiveInput(&'static str),
    NumericalRange,
}

fn require_positive_finite(value: f64, field: &'static str) -> Result<(), FlowError> {
    if !value.is_finite() {
        return Err(FlowError::NonFiniteInput(field));
    }
    if value <= 0.0 {
        return Err(FlowError::NonPositiveInput(field));
    }
    Ok(())
}

/// Evaluate the analytic Hagen–Poiseuille reference.
///
/// Resistance is `R = 8 μ L / (π r⁴)` and signed flow is `Q = ΔP / R`.
/// Inputs must use SI units. This function does not determine whether the
/// flow is laminar or whether a biological vessel satisfies the model.
pub fn evaluate_tube(spec: TubeSpec) -> Result<TubeResult, FlowError> {
    require_positive_finite(spec.length_m, "length_m")?;
    require_positive_finite(spec.radius_m, "radius_m")?;
    require_positive_finite(spec.dynamic_viscosity_pa_s, "dynamic_viscosity_pa_s")?;

    if !spec.pressure_drop_pa.is_finite() {
        return Err(FlowError::NonFiniteInput("pressure_drop_pa"));
    }

    let radius_fourth = spec.radius_m.powi(4);
    let resistance_numerator =
        8.0 * spec.dynamic_viscosity_pa_s * spec.length_m;
    let resistance_denominator = PI * radius_fourth;

    if !radius_fourth.is_finite()
        || radius_fourth <= 0.0
        || !resistance_numerator.is_finite()
        || resistance_numerator <= 0.0
        || !resistance_denominator.is_finite()
        || resistance_denominator <= 0.0
    {
        return Err(FlowError::NumericalRange);
    }

    let resistance = resistance_numerator / resistance_denominator;
    if !resistance.is_finite() || resistance <= 0.0 {
        return Err(FlowError::NumericalRange);
    }

    let flow = spec.pressure_drop_pa / resistance;
    if !flow.is_finite() {
        return Err(FlowError::NumericalRange);
    }

    Ok(TubeResult {
        hydraulic_resistance_pa_s_per_m3: resistance,
        volume_flow_m3_per_s: flow,
    })
}

/// Return the signed junction residual for edge flows in m³/s.
///
/// Positive values denote flow into the junction; negative values denote flow
/// out. A conserving junction has a residual of zero within a separately
/// declared numerical tolerance. This function intentionally does not pick
/// that tolerance on behalf of a benchmark.
pub fn junction_residual_m3_per_s(
    signed_flows_m3_per_s: &[f64],
) -> Result<f64, FlowError> {
    let mut residual = 0.0;
    for flow in signed_flows_m3_per_s {
        if !flow.is_finite() {
            return Err(FlowError::NonFiniteInput("signed_flows_m3_per_s"));
        }
        residual += *flow;
        if !residual.is_finite() {
            return Err(FlowError::NumericalRange);
        }
    }
    Ok(residual)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> TubeSpec {
        TubeSpec {
            length_m: 0.1,
            radius_m: 0.001,
            dynamic_viscosity_pa_s: 0.001,
            pressure_drop_pa: 1_000.0,
        }
    }

    fn assert_relative_close(actual: f64, expected: f64, tolerance: f64) {
        let scale = expected.abs().max(f64::MIN_POSITIVE);
        assert!(
            (actual - expected).abs() <= tolerance * scale,
            "actual {actual:.16e} differs from expected {expected:.16e}"
        );
    }

    #[test]
    fn tube_matches_pinned_analytic_reference() {
        let result = evaluate_tube(baseline()).unwrap();
        assert_relative_close(
            result.volume_flow_m3_per_s,
            3.926_990_816_987_241e-6,
            1e-12,
        );
        assert_relative_close(
            result.hydraulic_resistance_pa_s_per_m3,
            2.546_479_089_470_325_5e8,
            1e-12,
        );
    }

    #[test]
    fn doubling_radius_multiplies_flow_by_sixteen() {
        let base = evaluate_tube(baseline()).unwrap().volume_flow_m3_per_s;
        let mut spec = baseline();
        spec.radius_m *= 2.0;
        let changed = evaluate_tube(spec).unwrap().volume_flow_m3_per_s;
        assert_relative_close(changed / base, 16.0, 1e-12);
    }

    #[test]
    fn doubling_viscosity_halves_flow() {
        let base = evaluate_tube(baseline()).unwrap().volume_flow_m3_per_s;
        let mut spec = baseline();
        spec.dynamic_viscosity_pa_s *= 2.0;
        let changed = evaluate_tube(spec).unwrap().volume_flow_m3_per_s;
        assert_relative_close(changed / base, 0.5, 1e-12);
    }

    #[test]
    fn doubling_length_halves_flow() {
        let base = evaluate_tube(baseline()).unwrap().volume_flow_m3_per_s;
        let mut spec = baseline();
        spec.length_m *= 2.0;
        let changed = evaluate_tube(spec).unwrap().volume_flow_m3_per_s;
        assert_relative_close(changed / base, 0.5, 1e-12);
    }

    #[test]
    fn flow_sign_follows_pressure_drop_and_zero_is_valid() {
        let positive = evaluate_tube(baseline()).unwrap().volume_flow_m3_per_s;
        let mut reverse = baseline();
        reverse.pressure_drop_pa = -1_000.0;
        let negative = evaluate_tube(reverse).unwrap().volume_flow_m3_per_s;
        assert_eq!(positive, -negative);

        let mut stopped = baseline();
        stopped.pressure_drop_pa = 0.0;
        assert_eq!(evaluate_tube(stopped).unwrap().volume_flow_m3_per_s, 0.0);
    }

    #[test]
    fn invalid_geometry_and_non_finite_pressure_fail_closed() {
        let mut spec = baseline();
        spec.radius_m = 0.0;
        assert_eq!(
            evaluate_tube(spec),
            Err(FlowError::NonPositiveInput("radius_m"))
        );

        let mut spec = baseline();
        spec.length_m = f64::NAN;
        assert_eq!(
            evaluate_tube(spec),
            Err(FlowError::NonFiniteInput("length_m"))
        );

        let mut spec = baseline();
        spec.pressure_drop_pa = f64::INFINITY;
        assert_eq!(
            evaluate_tube(spec),
            Err(FlowError::NonFiniteInput("pressure_drop_pa"))
        );
    }

    #[test]
    fn unrepresentable_intermediate_is_not_reported_as_flow() {
        let mut spec = baseline();
        spec.radius_m = f64::MAX;
        assert_eq!(evaluate_tube(spec), Err(FlowError::NumericalRange));
    }

    #[test]
    fn junction_residual_detects_balance_and_imbalance() {
        let balanced = junction_residual_m3_per_s(&[2e-6, 3e-6, -5e-6]).unwrap();
        assert!(balanced.abs() < 1e-20, "residual {balanced:e}");

        let imbalanced = junction_residual_m3_per_s(&[2e-6, 3e-6, -4e-6]).unwrap();
        assert!((imbalanced - 1e-6).abs() < 1e-20, "residual {imbalanced:e}");
    }

    #[test]
    fn junction_rejects_non_finite_edge_flow() {
        assert_eq!(
            junction_residual_m3_per_s(&[1e-6, f64::NAN]),
            Err(FlowError::NonFiniteInput("signed_flows_m3_per_s"))
        );
    }
}
