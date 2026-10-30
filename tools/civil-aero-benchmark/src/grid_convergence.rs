//! Three-grid observed-order, Richardson extrapolation, and GCI diagnostics.
//!
//! Characteristic spacing and the safety factor are caller-supplied. This module does not infer
//! spacing from node count, authenticate solver outputs, or validate a CFD model. It rejects
//! non-monotonic sequences rather than reporting a misleading convergence estimate.

use std::collections::HashSet;

const ORDER_UPPER_LIMIT: f64 = 128.0;
const BISECTION_STEPS: usize = 100;
const ORDER_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq)]
pub struct GridObservation {
    pub grid_id: String,
    /// Caller-supplied characteristic spacing in consistent units.
    pub characteristic_spacing: f64,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GciStudy {
    pub quantity: String,
    pub source_reference: String,
    /// Disclosed derivation, such as a volume-based characteristic length.
    pub spacing_definition: String,
    /// Caller-selected GCI safety factor, finite and >= 1.
    pub safety_factor: f64,
    /// Ordered coarse-to-fine; at least three grids are required.
    pub observations: Vec<GridObservation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GciTriplet {
    pub coarse_grid_id: String,
    pub medium_grid_id: String,
    pub fine_grid_id: String,
    pub h_coarse: f64,
    pub h_medium: f64,
    pub h_fine: f64,
    pub phi_coarse: f64,
    pub phi_medium: f64,
    pub phi_fine: f64,
    pub refinement_ratio_coarse_medium: f64,
    pub refinement_ratio_medium_fine: f64,
    pub observed_order: f64,
    pub extrapolated_value: f64,
    pub medium_gci_absolute: f64,
    pub medium_gci_relative_percent: Option<f64>,
    pub fine_gci_absolute: f64,
    pub fine_gci_relative_percent: Option<f64>,
    /// Absolute-GCI asymptotic ratio; values near 1 are consistent with asymptotic behaviour.
    pub asymptotic_ratio: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GciReport {
    pub quantity: String,
    pub source_reference: String,
    pub spacing_definition: String,
    pub safety_factor: f64,
    pub evidence_level: &'static str,
    pub triplets: Vec<GciTriplet>,
}

pub fn evaluate_grid_convergence(study: &GciStudy) -> Result<GciReport, String> {
    if study.quantity.trim().is_empty()
        || study.source_reference.trim().is_empty()
        || study.spacing_definition.trim().is_empty()
    {
        return Err("quantity, source_reference, and spacing_definition must be non-empty".to_owned());
    }
    if !study.safety_factor.is_finite() || study.safety_factor < 1.0 {
        return Err("safety_factor must be finite and >= 1".to_owned());
    }
    if study.observations.len() < 3 {
        return Err("at least three systematically refined grids are required".to_owned());
    }

    let mut ids = HashSet::new();
    for (index, observation) in study.observations.iter().enumerate() {
        if observation.grid_id.trim().is_empty()
            || observation.grid_id.contains(',')
            || observation.grid_id.contains('\n')
            || observation.grid_id.contains('\r')
        {
            return Err(format!("grid observation {index}: grid_id must be non-empty and CSV-safe"));
        }
        if !ids.insert(observation.grid_id.clone()) {
            return Err(format!("duplicate grid_id {}", observation.grid_id));
        }
        if !observation.characteristic_spacing.is_finite()
            || observation.characteristic_spacing <= 0.0
        {
            return Err(format!(
                "grid {}: characteristic spacing must be finite and > 0",
                observation.grid_id
            ));
        }
        if !observation.value.is_finite() {
            return Err(format!("grid {}: solution value must be finite", observation.grid_id));
        }
        if index > 0
            && study.observations[index - 1].characteristic_spacing
                <= observation.characteristic_spacing
        {
            return Err(format!(
                "grid family must be strictly ordered coarse-to-fine: {} spacing must exceed {} spacing",
                study.observations[index - 1].grid_id,
                observation.grid_id
            ));
        }
    }

    let mut triplets = Vec::with_capacity(study.observations.len() - 2);
    for window in study.observations.windows(3) {
        triplets.push(evaluate_triplet(
            &window[0],
            &window[1],
            &window[2],
            study.safety_factor,
        )?);
    }

    Ok(GciReport {
        quantity: study.quantity.clone(),
        source_reference: study.source_reference.clone(),
        spacing_definition: study.spacing_definition.clone(),
        safety_factor: study.safety_factor,
        evidence_level: "caller_supplied_grid_convergence_estimate_not_independent_validation",
        triplets,
    })
}

fn evaluate_triplet(
    coarse: &GridObservation,
    medium: &GridObservation,
    fine: &GridObservation,
    safety_factor: f64,
) -> Result<GciTriplet, String> {
    let r_cm = coarse.characteristic_spacing / medium.characteristic_spacing;
    let r_mf = medium.characteristic_spacing / fine.characteristic_spacing;
    if !r_cm.is_finite() || !r_mf.is_finite() || r_cm <= 1.0 || r_mf <= 1.0 {
        return Err("refinement ratios must be finite and > 1".to_owned());
    }

    let delta_cm = coarse.value - medium.value;
    let delta_mf = medium.value - fine.value;
    if delta_cm == 0.0 || delta_mf == 0.0 || delta_cm.signum() != delta_mf.signum() {
        return Err(format!(
            "non-monotonic or zero-difference sequence for {}, {}, {}",
            coarse.grid_id, medium.grid_id, fine.grid_id
        ));
    }
    let observed_ratio = (delta_cm / delta_mf).abs();
    if !observed_ratio.is_finite() || observed_ratio <= 0.0 {
        return Err("observed difference ratio is invalid".to_owned());
    }

    let order = solve_observed_order(observed_ratio, r_cm, r_mf)?;
    let r_cm_power = checked_power(r_cm, order)?;
    let r_mf_power = checked_power(r_mf, order)?;
    let denominator_cm = r_cm_power - 1.0;
    let denominator_mf = r_mf_power - 1.0;
    if denominator_cm <= 0.0 || denominator_mf <= 0.0 {
        return Err("observed-order denominator is not positive".to_owned());
    }

    let extrapolated_value = fine.value + (fine.value - medium.value) / denominator_mf;
    let medium_gci_absolute = safety_factor * delta_cm.abs() / denominator_cm;
    let fine_gci_absolute = safety_factor * delta_mf.abs() / denominator_mf;
    let medium_relative = relative_percent(medium_gci_absolute, medium.value);
    let fine_relative = relative_percent(fine_gci_absolute, fine.value);
    let asymptotic_ratio = medium_gci_absolute / (fine_gci_absolute * r_mf_power);

    for (name, value) in [
        ("extrapolated_value", extrapolated_value),
        ("medium_gci_absolute", medium_gci_absolute),
        ("fine_gci_absolute", fine_gci_absolute),
        ("asymptotic_ratio", asymptotic_ratio),
    ] {
        if !value.is_finite() {
            return Err(format!("{name} overflowed to a non-finite value"));
        }
    }
    for value in [medium_relative, fine_relative].into_iter().flatten() {
        if !value.is_finite() {
            return Err("relative GCI overflowed to a non-finite value".to_owned());
        }
    }

    Ok(GciTriplet {
        coarse_grid_id: coarse.grid_id.clone(),
        medium_grid_id: medium.grid_id.clone(),
        fine_grid_id: fine.grid_id.clone(),
        h_coarse: coarse.characteristic_spacing,
        h_medium: medium.characteristic_spacing,
        h_fine: fine.characteristic_spacing,
        phi_coarse: coarse.value,
        phi_medium: medium.value,
        phi_fine: fine.value,
        refinement_ratio_coarse_medium: r_cm,
        refinement_ratio_medium_fine: r_mf,
        observed_order: order,
        extrapolated_value,
        medium_gci_absolute,
        medium_gci_relative_percent: medium_relative,
        fine_gci_absolute,
        fine_gci_relative_percent: fine_relative,
        asymptotic_ratio,
    })
}

fn solve_observed_order(observed_ratio: f64, r_cm: f64, r_mf: f64) -> Result<f64, String> {
    let zero_order_limit = r_cm.ln() / r_mf.ln();
    if !zero_order_limit.is_finite() || observed_ratio <= zero_order_limit {
        return Err(format!(
            "no positive observed order: difference ratio {observed_ratio} must exceed zero-order limit {zero_order_limit}"
        ));
    }

    if (r_cm - r_mf).abs() <= 1.0e-12 * r_cm.max(r_mf) {
        if observed_ratio <= 1.0 {
            return Err("equal-ratio refinement requires observed ratio > 1".to_owned());
        }
        let order = observed_ratio.ln() / r_cm.ln();
        if order.is_finite() && order > 0.0 {
            return Ok(order);
        }
        return Err("equal-ratio observed order is invalid".to_owned());
    }

    let mut low = 0.0;
    let mut high = 1.0;
    while ratio_for_order(high, r_cm, r_mf) < observed_ratio {
        high *= 2.0;
        if high > ORDER_UPPER_LIMIT {
            return Err("observed order could not be bracketed below limit 128".to_owned());
        }
    }

    for _ in 0..BISECTION_STEPS {
        let middle = 0.5 * (low + high);
        if ratio_for_order(middle, r_cm, r_mf) < observed_ratio {
            low = middle;
        } else {
            high = middle;
        }
        if (high - low) <= ORDER_TOLERANCE * high.max(1.0) {
            break;
        }
    }
    let order = 0.5 * (low + high);
    if !order.is_finite() || order <= 0.0 {
        return Err("solved observed order is invalid".to_owned());
    }
    Ok(order)
}

fn ratio_for_order(order: f64, r_cm: f64, r_mf: f64) -> f64 {
    let exponent = order * r_cm.ln();
    if exponent > f64::MAX.ln() {
        return f64::INFINITY;
    }
    (exponent.exp() - 1.0) / (1.0 - (-order * r_mf.ln()).exp())
}

fn checked_power(ratio: f64, order: f64) -> Result<f64, String> {
    let exponent = order * ratio.ln();
    if !exponent.is_finite() || exponent > f64::MAX.ln() {
        return Err("refinement ratio raised to observed order overflowed".to_owned());
    }
    let value = exponent.exp();
    if !value.is_finite() {
        return Err("refinement power is non-finite".to_owned());
    }
    Ok(value)
}

fn relative_percent(absolute: f64, value: f64) -> Option<f64> {
    if value == 0.0 {
        None
    } else {
        Some(100.0 * absolute / value.abs())
    }
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub const GCI_CSV_HEADER: &str = "quantity,source_reference,spacing_definition,safety_factor,coarse_grid_id,medium_grid_id,fine_grid_id,h_coarse,h_medium,h_fine,phi_coarse,phi_medium,phi_fine,r_cm,r_mf,observed_order,extrapolated_value,medium_gci_absolute,medium_gci_relative_percent,fine_gci_absolute,fine_gci_relative_percent,asymptotic_ratio,evidence_level";

pub fn report_to_csv(report: &GciReport) -> String {
    let mut output = format!("{GCI_CSV_HEADER}\n");
    for triplet in &report.triplets {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{},{:.12},{},{:.12},{}\n",
            csv_escape(&report.quantity),
            csv_escape(&report.source_reference),
            csv_escape(&report.spacing_definition),
            report.safety_factor,
            csv_escape(&triplet.coarse_grid_id),
            csv_escape(&triplet.medium_grid_id),
            csv_escape(&triplet.fine_grid_id),
            triplet.h_coarse,
            triplet.h_medium,
            triplet.h_fine,
            triplet.phi_coarse,
            triplet.phi_medium,
            triplet.phi_fine,
            triplet.refinement_ratio_coarse_medium,
            triplet.refinement_ratio_medium_fine,
            triplet.observed_order,
            triplet.extrapolated_value,
            triplet.medium_gci_absolute,
            triplet.medium_gci_relative_percent
                .map(|value| format!("{value:.12}"))
                .unwrap_or_default(),
            triplet.fine_gci_absolute,
            triplet.fine_gci_relative_percent
                .map(|value| format!("{value:.12}"))
                .unwrap_or_default(),
            triplet.asymptotic_ratio,
            report.evidence_level,
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quadratic_six_grid_study() -> GciStudy {
        let spacings = [4.0_f64, 2.0, 1.0, 0.5, 0.25, 0.125];
        GciStudy {
            quantity: "CD".to_owned(),
            source_reference: "synthetic-known-answer-quadratic-v0.1".to_owned(),
            spacing_definition: "synthetic h values, explicitly supplied".to_owned(),
            safety_factor: 1.25,
            observations: spacings
                .iter()
                .enumerate()
                .map(|(index, h)| GridObservation {
                    grid_id: format!("L{}", index + 1),
                    characteristic_spacing: *h,
                    value: 2.0 + 0.8 * h.powi(2),
                })
                .collect(),
        }
    }

    #[test]
    fn six_grid_quadratic_recovers_second_order_exact_limit_and_all_windows() {
        let report = evaluate_grid_convergence(&quadratic_six_grid_study())
            .expect("monotone quadratic refinement");
        assert_eq!(report.triplets.len(), 4);
        for triplet in &report.triplets {
            assert!((triplet.observed_order - 2.0).abs() < 1.0e-10);
            assert!((triplet.extrapolated_value - 2.0).abs() < 1.0e-10);
            assert!((triplet.asymptotic_ratio - 1.0).abs() < 1.0e-10);
            assert!(triplet.fine_gci_absolute > 0.0);
        }
        assert!(report.evidence_level.contains("not_independent_validation"));
    }

    #[test]
    fn nonuniform_refinement_solves_implicit_order() {
        let study = GciStudy {
            quantity: "CL".to_owned(),
            source_reference: "synthetic-nonuniform-v0.1".to_owned(),
            spacing_definition: "explicit nonuniform h".to_owned(),
            safety_factor: 3.0,
            observations: vec![
                GridObservation {
                    grid_id: "coarse".to_owned(),
                    characteristic_spacing: 4.0,
                    value: 49.0,
                },
                GridObservation {
                    grid_id: "medium".to_owned(),
                    characteristic_spacing: 2.0,
                    value: 13.0,
                },
                GridObservation {
                    grid_id: "fine".to_owned(),
                    characteristic_spacing: 0.5,
                    value: 1.75,
                },
            ],
        };
        let report = evaluate_grid_convergence(&study).expect("valid unequal ratios");
        let triplet = &report.triplets[0];
        assert!((triplet.observed_order - 2.0).abs() < 1.0e-10);
        assert!((triplet.extrapolated_value - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn oscillatory_or_zero_difference_sequences_fail_closed() {
        let mut study = quadratic_six_grid_study();
        study.observations[1].value = 20.0;
        assert!(evaluate_grid_convergence(&study)
            .expect_err("oscillatory sequence rejected")
            .contains("non-monotonic"));

        let mut study = quadratic_six_grid_study();
        study.observations[1].value = study.observations[0].value;
        assert!(evaluate_grid_convergence(&study)
            .expect_err("zero delta rejected")
            .contains("zero-difference"));
    }

    #[test]
    fn missing_grids_bad_ordering_duplicate_ids_and_bad_safety_factor_rejected() {
        let mut study = quadratic_six_grid_study();
        study.observations.truncate(2);
        assert!(evaluate_grid_convergence(&study).is_err());

        let mut study = quadratic_six_grid_study();
        study.observations[1].characteristic_spacing = 4.5;
        assert!(evaluate_grid_convergence(&study)
            .expect_err("misordered grids rejected")
            .contains("coarse-to-fine"));

        let mut study = quadratic_six_grid_study();
        study.observations[1].grid_id = study.observations[0].grid_id.clone();
        assert!(evaluate_grid_convergence(&study)
            .expect_err("duplicate grid id rejected")
            .contains("duplicate grid_id"));

        let mut study = quadratic_six_grid_study();
        study.safety_factor = f64::NAN;
        assert!(evaluate_grid_convergence(&study).is_err());
    }

    #[test]
    fn zero_fine_value_has_no_relative_gci_but_keeps_absolute_gci() {
        let study = GciStudy {
            quantity: "residual".to_owned(),
            source_reference: "synthetic-zero-fine-v0.1".to_owned(),
            spacing_definition: "explicit h".to_owned(),
            safety_factor: 1.25,
            observations: vec![
                GridObservation {
                    grid_id: "c".to_owned(),
                    characteristic_spacing: 4.0,
                    value: 3.0,
                },
                GridObservation {
                    grid_id: "m".to_owned(),
                    characteristic_spacing: 2.0,
                    value: 0.75,
                },
                GridObservation {
                    grid_id: "f".to_owned(),
                    characteristic_spacing: 1.0,
                    value: 0.0,
                },
            ],
        };
        let report = evaluate_grid_convergence(&study).expect("valid absolute-error report");
        assert!(report.triplets[0].fine_gci_relative_percent.is_none());
        assert!(report.triplets[0].fine_gci_absolute > 0.0);
    }

    #[test]
    fn csv_report_is_deterministic_and_labels_evidence_ceiling() {
        let report = evaluate_grid_convergence(&quadratic_six_grid_study())
            .expect("valid synthetic study");
        let first = report_to_csv(&report);
        assert_eq!(first, report_to_csv(&report));
        assert!(first.contains("caller_supplied_grid_convergence_estimate_not_independent_validation"));
        assert_eq!(first.lines().count(), 5);
    }
}
