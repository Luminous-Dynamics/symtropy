// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Equation-level reference for root anatomy and mycorrhizal arbuscule energetics.
//!
//! Source: Cao et al. (2025), "Root anatomy governs bi-directional resource
//! transfer in mycorrhizal symbiosis", Nature Communications 16, 8731,
//! https://doi.org/10.1038/s41467-025-64553-1, Box 1 equations 1-10.
//!
//! This is a clean-room transcription of the published analytical model. It
//! preserves the source assumptions and is not a calibrated plant-root or
//! fungal physiology model. Extraradical foraging/transport, enzyme kinetics,
//! detailed active uptake chemistry, and biological heterogeneity are omitted.
//!
//! Units used here:
//! - length: metres;
//! - nutrient amount: milligrams of the declared nutrient element;
//! - carbon amount: milligrams of carbon;
//! - time: hours;
//! - energy: joules.
//!
//! Therefore k and k2 have units mg nutrient / (m * hour), E0 and E0f have
//! units J * m / mg nutrient, and k_ncf has units mg nutrient / mg carbon.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootAnatomyParameters {
    /// Number of tissue-outside-stele (ToS) cell layers; innermost layer is 1.
    pub tos_layer_count: u32,
    /// Thickness of one ToS cell layer, in metres.
    pub layer_thickness_m: f64,
    /// Nutrient arrival rate per metre of root perimeter, mg/(m*h); paper k.
    pub root_surface_nutrient_rate_k_mg_per_m_hour: f64,
    /// Nutrient released per metre of arbuscule-bearing layer perimeter,
    /// mg/(m*h); paper k''.
    pub arbuscule_nutrient_rate_k2_mg_per_m_hour: f64,
    /// Root active-uptake energy cost per nutrient amount per radial length,
    /// J*m/mg; paper E0.
    pub root_transport_energy_e0_j_m_per_mg: f64,
    /// Hyphal transport energy cost on the same basis, J*m/mg; paper E0f.
    pub fungal_transport_energy_e0f_j_m_per_mg: f64,
    /// Nutrient received by the plant per unit root carbon allocation,
    /// mg nutrient/mg carbon; paper k_ncf.
    pub fungal_nutrient_per_carbon_k_ncf_mg_per_mg: f64,
    /// Optional measured root radius, in metres. It is retained separately
    /// from the model's r ~= x*t approximation and is not substituted silently.
    pub observed_root_radius_m: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerEnergyEfficiency {
    /// Innermost ToS layer is assumed to release nutrients directly to the
    /// stele. Eq. 10 has a zero denominator here, so the ratio is undefined.
    DirectToStele,
    /// Finite nutrient benefit per root transport energy, in mg nutrient/J.
    Finite,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootLayerEnergyResult {
    /// ToS layer index with 1 as innermost.
    pub layer_index: u32,
    /// Nutrient rate released by arbuscules in this layer, mg/h (Eq. 4 term).
    pub fungal_nutrient_rate_mg_per_hour: f64,
    /// Root radial transport distance from this layer to the stele, metres.
    pub root_transport_distance_m: f64,
    /// Root energy required to transport this layer's nutrient to the stele,
    /// J/h. Zero for the direct-to-stele case.
    pub root_transport_energy_j_per_hour: f64,
    /// Energy saved by using the fungal pathway instead of root transport
    /// from the epidermis to this arbuscule layer, J/h (Eq. 8).
    /// Negative values are preserved if E0f exceeds E0.
    pub hyphal_transport_energy_saving_j_per_hour: f64,
    /// Eq. 10's layer-specific efficiency, except layer 1 is a typed direct
    /// delivery with undefined ratio rather than infinity.
    pub efficiency_status: LayerEnergyEfficiency,
    pub nutrient_per_energy_mg_per_j: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RootAnatomyEnergyReport {
    pub tos_layer_count: u32,
    pub layer_thickness_m: f64,
    /// Model approximation r = x*t, metres.
    pub model_root_radius_m: f64,
    pub observed_root_radius_m: Option<f64>,
    /// |observed - x*t| / observed, if observed radius is available.
    pub radius_approximation_relative_error: Option<f64>,

    /// Eq. 1: nutrient arrival at root surface, mg/h.
    pub root_surface_nutrient_rate_mg_per_hour: f64,
    /// Eq. 2: energy for root active uptake through the ToS, J/h.
    pub root_transport_energy_j_per_hour: f64,
    /// Eq. 3: non-mycorrhizal root nutrient/energy ratio, mg/J.
    pub non_mycorrhizal_nutrient_per_energy_mg_per_j: f64,

    /// Eq. 4: nutrient received from all arbuscule-bearing layers, mg/h.
    pub fungal_nutrient_rate_mg_per_hour: f64,
    /// Eq. 5: root carbon allocation to support fungal nutrient acquisition,
    /// mg carbon/h.
    pub fungal_carbon_investment_mg_per_hour: f64,
    /// Eq. 6: root energy for transporting arbuscule-derived nutrients to
    /// the stele, J/h.
    pub mycorrhizal_root_transport_energy_j_per_hour: f64,
    /// Eq. 7: total root nutrient benefit / root energy investment, mg/J.
    pub mycorrhizal_root_nutrient_per_energy_mg_per_j: f64,
    /// Eq. 9: total energy saving due to fungal transport, J/h.
    pub total_hyphal_transport_energy_saving_j_per_hour: f64,
    pub layer_results: Vec<RootLayerEnergyResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootAnatomyEnergyError {
    ZeroLayerCount,
    InvalidLayerThickness,
    InvalidRootSurfaceRate,
    InvalidArbusculeRate,
    InvalidRootTransportCost,
    InvalidFungalTransportCost,
    InvalidFungalExchangeRatio,
    InvalidObservedRootRadius,
    AllocationFailure,
    NonFiniteDerivedValue,
}

impl fmt::Display for RootAnatomyEnergyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLayerCount => write!(f, "ToS layer count must be positive"),
            Self::InvalidLayerThickness => {
                write!(f, "ToS layer thickness must be finite and positive")
            }
            Self::InvalidRootSurfaceRate => {
                write!(
                    f,
                    "root surface nutrient rate k must be finite and positive"
                )
            }
            Self::InvalidArbusculeRate => {
                write!(f, "arbuscule nutrient rate k2 must be finite and positive")
            }
            Self::InvalidRootTransportCost => {
                write!(
                    f,
                    "root transport energy cost E0 must be finite and positive"
                )
            }
            Self::InvalidFungalTransportCost => {
                write!(
                    f,
                    "fungal transport energy cost E0f must be finite and non-negative"
                )
            }
            Self::InvalidFungalExchangeRatio => {
                write!(
                    f,
                    "fungal nutrient-per-carbon ratio must be finite and positive"
                )
            }
            Self::InvalidObservedRootRadius => {
                write!(
                    f,
                    "observed root radius must be finite and positive when supplied"
                )
            }
            Self::AllocationFailure => {
                write!(f, "unable to reserve per-layer root anatomy result storage")
            }
            Self::NonFiniteDerivedValue => {
                write!(
                    f,
                    "root anatomy equations produced a non-finite derived value"
                )
            }
        }
    }
}

impl std::error::Error for RootAnatomyEnergyError {}

impl RootAnatomyParameters {
    pub fn validate(&self) -> Result<(), RootAnatomyEnergyError> {
        if self.tos_layer_count == 0 {
            return Err(RootAnatomyEnergyError::ZeroLayerCount);
        }
        if !self.layer_thickness_m.is_finite() || self.layer_thickness_m <= 0.0 {
            return Err(RootAnatomyEnergyError::InvalidLayerThickness);
        }
        if !self.root_surface_nutrient_rate_k_mg_per_m_hour.is_finite()
            || self.root_surface_nutrient_rate_k_mg_per_m_hour <= 0.0
        {
            return Err(RootAnatomyEnergyError::InvalidRootSurfaceRate);
        }
        if !self.arbuscule_nutrient_rate_k2_mg_per_m_hour.is_finite()
            || self.arbuscule_nutrient_rate_k2_mg_per_m_hour <= 0.0
        {
            return Err(RootAnatomyEnergyError::InvalidArbusculeRate);
        }
        if !self.root_transport_energy_e0_j_m_per_mg.is_finite()
            || self.root_transport_energy_e0_j_m_per_mg <= 0.0
        {
            return Err(RootAnatomyEnergyError::InvalidRootTransportCost);
        }
        if !self.fungal_transport_energy_e0f_j_m_per_mg.is_finite()
            || self.fungal_transport_energy_e0f_j_m_per_mg < 0.0
        {
            return Err(RootAnatomyEnergyError::InvalidFungalTransportCost);
        }
        if !self.fungal_nutrient_per_carbon_k_ncf_mg_per_mg.is_finite()
            || self.fungal_nutrient_per_carbon_k_ncf_mg_per_mg <= 0.0
        {
            return Err(RootAnatomyEnergyError::InvalidFungalExchangeRatio);
        }
        if self
            .observed_root_radius_m
            .is_some_and(|radius| !radius.is_finite() || radius <= 0.0)
        {
            return Err(RootAnatomyEnergyError::InvalidObservedRootRadius);
        }
        Ok(())
    }
}

/// Evaluate Box 1 equations 1-10 from Cao et al. (2025).
///
/// The discrete sum is used for Eqs. 4, 6, and 9. It is algebraically equal
/// to the source's closed forms, but retaining the layer loop also returns
/// per-layer outputs and makes the innermost-layer zero-distance case explicit.
pub fn evaluate_root_anatomy_energy(
    parameters: &RootAnatomyParameters,
) -> Result<RootAnatomyEnergyReport, RootAnatomyEnergyError> {
    parameters.validate()?;

    let x = f64::from(parameters.tos_layer_count);
    let t = parameters.layer_thickness_m;
    let k = parameters.root_surface_nutrient_rate_k_mg_per_m_hour;
    let k2 = parameters.arbuscule_nutrient_rate_k2_mg_per_m_hour;
    let e0 = parameters.root_transport_energy_e0_j_m_per_mg;
    let e0f = parameters.fungal_transport_energy_e0f_j_m_per_mg;
    let k_ncf = parameters.fungal_nutrient_per_carbon_k_ncf_mg_per_mg;
    let model_radius = t * x;

    // Eq. 1: N_ru = 2*pi*r*k, with the source's r ~= x*t approximation.
    let root_surface_nutrient = 2.0 * std::f64::consts::PI * model_radius * k;

    // Eq. 2: E_rt = N_ru * E0 * x*t.
    let root_transport_energy = root_surface_nutrient * e0 * model_radius;

    // Eq. 3 is calculated as the explicit ratio as well as the theoretical
    // 1/(E0*r) form; the report exposes the ratio used by the model.
    let non_mycorrhizal_efficiency = root_surface_nutrient / root_transport_energy;

    let mut fungal_nutrient = 0.0;
    let mut fungal_transport_energy = 0.0;
    let mut total_hyphal_saving = 0.0;
    let mut layer_results = Vec::new();
    layer_results
        .try_reserve_exact(parameters.tos_layer_count as usize)
        .map_err(|_| RootAnatomyEnergyError::AllocationFailure)?;

    for layer_index in 1..=parameters.tos_layer_count {
        let i = f64::from(layer_index);

        // Eq. 4: the layer contributes 2*pi*t*i*k2 nutrient mg per hour.
        let layer_nutrient = 2.0 * std::f64::consts::PI * t * i * k2;
        fungal_nutrient += layer_nutrient;

        // Eq. 6: root energy to transport the layer's released nutrient
        // across (i-1)*t to the stele.
        let radial_distance = (i - 1.0) * t;
        let layer_root_energy = layer_nutrient * e0 * radial_distance;
        fungal_transport_energy += layer_root_energy;

        // Eq. 8: energy saving from moving nutrients through fungal hyphae
        // rather than root tissue across the epidermis-to-arbuscule distance.
        let hyphal_distance = (x - i) * t;
        let layer_saving = layer_nutrient * hyphal_distance * (e0 - e0f);
        total_hyphal_saving += layer_saving;

        // Eq. 10: k_ner_i = 1 / (E0*t*(i-1)) for i>1. At i=1 the
        // source assumes direct-to-stele delivery and the denominator is zero.
        let (efficiency_status, nutrient_per_energy) = if layer_index == 1 {
            // Eq. 10's denominator is zero at i=1: direct-to-stele delivery,
            // not an infinite numerical efficiency.
            (LayerEnergyEfficiency::DirectToStele, None)
        } else {
            (
                LayerEnergyEfficiency::Finite,
                Some(layer_nutrient / layer_root_energy),
            )
        };

        layer_results.push(RootLayerEnergyResult {
            layer_index,
            fungal_nutrient_rate_mg_per_hour: layer_nutrient,
            root_transport_distance_m: radial_distance,
            root_transport_energy_j_per_hour: layer_root_energy,
            hyphal_transport_energy_saving_j_per_hour: layer_saving,
            efficiency_status,
            nutrient_per_energy_mg_per_j: nutrient_per_energy,
        });
    }

    // Eq. 5: C_fu = N_fu / k_ncf.
    let fungal_carbon_investment = fungal_nutrient / k_ncf;

    // Eq. 7: combined root-derived and fungal-derived nutrient benefit per
    // root energy investment for root uptake plus transfer to the stele.
    let mycorrhizal_efficiency = (root_surface_nutrient + fungal_nutrient)
        / (root_transport_energy + fungal_transport_energy);

    let report = RootAnatomyEnergyReport {
        tos_layer_count: parameters.tos_layer_count,
        layer_thickness_m: t,
        model_root_radius_m: model_radius,
        observed_root_radius_m: parameters.observed_root_radius_m,
        radius_approximation_relative_error: parameters
            .observed_root_radius_m
            .map(|observed| (observed - model_radius).abs() / observed),
        root_surface_nutrient_rate_mg_per_hour: root_surface_nutrient,
        root_transport_energy_j_per_hour: root_transport_energy,
        non_mycorrhizal_nutrient_per_energy_mg_per_j: non_mycorrhizal_efficiency,
        fungal_nutrient_rate_mg_per_hour: fungal_nutrient,
        fungal_carbon_investment_mg_per_hour: fungal_carbon_investment,
        mycorrhizal_root_transport_energy_j_per_hour: fungal_transport_energy,
        mycorrhizal_root_nutrient_per_energy_mg_per_j: mycorrhizal_efficiency,
        total_hyphal_transport_energy_saving_j_per_hour: total_hyphal_saving,
        layer_results,
    };

    let finite_values = [
        report.model_root_radius_m,
        report.root_surface_nutrient_rate_mg_per_hour,
        report.root_transport_energy_j_per_hour,
        report.non_mycorrhizal_nutrient_per_energy_mg_per_j,
        report.fungal_nutrient_rate_mg_per_hour,
        report.fungal_carbon_investment_mg_per_hour,
        report.mycorrhizal_root_transport_energy_j_per_hour,
        report.mycorrhizal_root_nutrient_per_energy_mg_per_j,
        report.total_hyphal_transport_energy_saving_j_per_hour,
    ];
    if finite_values.iter().any(|value| !value.is_finite())
        || report
            .radius_approximation_relative_error
            .is_some_and(|value| !value.is_finite())
        || report.layer_results.iter().any(|layer| {
            !layer.fungal_nutrient_rate_mg_per_hour.is_finite()
                || !layer.root_transport_distance_m.is_finite()
                || !layer.root_transport_energy_j_per_hour.is_finite()
                || !layer.hyphal_transport_energy_saving_j_per_hour.is_finite()
                || layer
                    .nutrient_per_energy_mg_per_j
                    .is_some_and(|value| !value.is_finite())
        })
    {
        return Err(RootAnatomyEnergyError::NonFiniteDerivedValue);
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameters(layer_count: u32) -> RootAnatomyParameters {
        RootAnatomyParameters {
            tos_layer_count: layer_count,
            layer_thickness_m: 0.001,
            root_surface_nutrient_rate_k_mg_per_m_hour: 2.0,
            arbuscule_nutrient_rate_k2_mg_per_m_hour: 3.0,
            root_transport_energy_e0_j_m_per_mg: 10.0,
            fungal_transport_energy_e0f_j_m_per_mg: 2.0,
            fungal_nutrient_per_carbon_k_ncf_mg_per_mg: 0.5,
            observed_root_radius_m: None,
        }
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "left={left:.16e}, right={right:.16e}"
        );
    }

    #[test]
    fn equations_one_to_three_match_one_layer_known_answer() {
        let report = evaluate_root_anatomy_energy(&parameters(1)).unwrap();
        let radius = 0.001;
        close(
            report.root_surface_nutrient_rate_mg_per_hour,
            2.0 * std::f64::consts::PI * radius * 2.0,
            1e-15,
        );
        close(
            report.root_transport_energy_j_per_hour,
            report.root_surface_nutrient_rate_mg_per_hour * 10.0 * radius,
            1e-15,
        );
        close(
            report.non_mycorrhizal_nutrient_per_energy_mg_per_j,
            1.0 / (10.0 * radius),
            1e-10,
        );
    }

    #[test]
    fn root_transport_and_nonmycorrhizal_efficiency_follow_radius_scaling() {
        let small = evaluate_root_anatomy_energy(&parameters(2)).unwrap();
        let large = evaluate_root_anatomy_energy(&parameters(4)).unwrap();
        assert_eq!(large.model_root_radius_m, small.model_root_radius_m * 2.0);
        assert!(large.root_transport_energy_j_per_hour > small.root_transport_energy_j_per_hour);
        assert!(
            large.non_mycorrhizal_nutrient_per_energy_mg_per_j
                < small.non_mycorrhizal_nutrient_per_energy_mg_per_j
        );
    }

    #[test]
    fn equation_four_discrete_layer_sum_matches_closed_form() {
        for x in 1..=12 {
            let report = evaluate_root_anatomy_energy(&parameters(x)).unwrap();
            let xf = f64::from(x);
            let t = 0.001;
            let k2 = 3.0;
            let closed = std::f64::consts::PI * t * k2 * xf * (xf + 1.0);
            close(report.fungal_nutrient_rate_mg_per_hour, closed, 1e-12);
        }
    }

    #[test]
    fn equation_five_carbon_investment_matches_nutrient_per_carbon_ratio() {
        let report = evaluate_root_anatomy_energy(&parameters(3)).unwrap();
        close(
            report.fungal_carbon_investment_mg_per_hour,
            report.fungal_nutrient_rate_mg_per_hour / 0.5,
            1e-12,
        );
    }

    #[test]
    fn equation_six_layer_transport_energy_matches_closed_form() {
        for x in 1..=12 {
            let report = evaluate_root_anatomy_energy(&parameters(x)).unwrap();
            let xf = f64::from(x);
            let expected =
                2.0 * std::f64::consts::PI * 0.001_f64.powi(2) * 3.0 * 10.0 * (xf.powi(3) - xf)
                    / 3.0;
            close(
                report.mycorrhizal_root_transport_energy_j_per_hour,
                expected,
                1e-12,
            );
        }
    }

    #[test]
    fn equation_seven_matches_direct_benefit_over_energy_ratio() {
        let report = evaluate_root_anatomy_energy(&parameters(5)).unwrap();
        let direct = (report.root_surface_nutrient_rate_mg_per_hour
            + report.fungal_nutrient_rate_mg_per_hour)
            / (report.root_transport_energy_j_per_hour
                + report.mycorrhizal_root_transport_energy_j_per_hour);
        close(
            report.mycorrhizal_root_nutrient_per_energy_mg_per_j,
            direct,
            1e-12,
        );
    }

    #[test]
    fn equation_eight_layer_savings_preserve_inner_middle_outer_pattern() {
        let report = evaluate_root_anatomy_energy(&parameters(3)).unwrap();
        let savings: Vec<_> = report
            .layer_results
            .iter()
            .map(|layer| layer.hyphal_transport_energy_saving_j_per_hour)
            .collect();
        close(savings[0], savings[1], 1e-15);
        assert!(savings[0] > 0.0);
        assert_eq!(savings[2], 0.0);
    }

    #[test]
    fn equation_nine_discrete_sum_matches_closed_form() {
        for x in 1..=16 {
            let report = evaluate_root_anatomy_energy(&parameters(x)).unwrap();
            let xf = f64::from(x);
            let expected = 2.0
                * std::f64::consts::PI
                * 0.001_f64.powi(2)
                * 3.0
                * (10.0 - 2.0)
                * (xf * (xf * xf - 1.0) / 6.0);
            close(
                report.total_hyphal_transport_energy_saving_j_per_hour,
                expected,
                1e-12,
            );
        }
    }

    #[test]
    fn equation_ten_direct_to_stele_is_typed_not_infinite() {
        let report = evaluate_root_anatomy_energy(&parameters(4)).unwrap();
        assert_eq!(
            report.layer_results[0].efficiency_status,
            LayerEnergyEfficiency::DirectToStele
        );
        assert_eq!(report.layer_results[0].nutrient_per_energy_mg_per_j, None);
        for layer in report.layer_results.iter().skip(1) {
            assert_eq!(layer.efficiency_status, LayerEnergyEfficiency::Finite);
            assert!(layer.nutrient_per_energy_mg_per_j.unwrap().is_finite());
        }
    }

    #[test]
    fn equal_root_and_fungal_transport_costs_produce_zero_saving() {
        let mut input = parameters(4);
        input.fungal_transport_energy_e0f_j_m_per_mg = input.root_transport_energy_e0_j_m_per_mg;
        let report = evaluate_root_anatomy_energy(&input).unwrap();
        assert_eq!(report.total_hyphal_transport_energy_saving_j_per_hour, 0.0);
    }

    #[test]
    fn higher_fungal_transport_cost_preserves_negative_energy_saving() {
        let mut input = parameters(4);
        input.fungal_transport_energy_e0f_j_m_per_mg = 12.0;
        let report = evaluate_root_anatomy_energy(&input).unwrap();
        assert!(report.total_hyphal_transport_energy_saving_j_per_hour < 0.0);
    }

    #[test]
    fn observed_root_radius_is_retained_and_compared_with_layer_approximation() {
        let mut input = parameters(4);
        input.observed_root_radius_m = Some(0.005);
        let report = evaluate_root_anatomy_energy(&input).unwrap();
        assert_eq!(report.observed_root_radius_m, Some(0.005));
        close(
            report.radius_approximation_relative_error.unwrap(),
            0.2,
            1e-12,
        );
    }

    #[test]
    fn invalid_parameters_fail_closed() {
        let mut input = parameters(1);
        input.tos_layer_count = 0;
        assert_eq!(
            evaluate_root_anatomy_energy(&input),
            Err(RootAnatomyEnergyError::ZeroLayerCount)
        );

        input = parameters(1);
        input.layer_thickness_m = 0.0;
        assert_eq!(
            evaluate_root_anatomy_energy(&input),
            Err(RootAnatomyEnergyError::InvalidLayerThickness)
        );

        input = parameters(1);
        input.root_transport_energy_e0_j_m_per_mg = 0.0;
        assert_eq!(
            evaluate_root_anatomy_energy(&input),
            Err(RootAnatomyEnergyError::InvalidRootTransportCost)
        );

        input = parameters(1);
        input.observed_root_radius_m = Some(f64::NAN);
        assert_eq!(
            evaluate_root_anatomy_energy(&input),
            Err(RootAnatomyEnergyError::InvalidObservedRootRadius)
        );
    }

    #[test]
    fn scaling_all_nutrient_rates_scales_amounts_but_not_efficiency_ratios() {
        let base = parameters(4);
        let mut doubled = base;
        doubled.root_surface_nutrient_rate_k_mg_per_m_hour *= 2.0;
        doubled.arbuscule_nutrient_rate_k2_mg_per_m_hour *= 2.0;

        let base_report = evaluate_root_anatomy_energy(&base).unwrap();
        let doubled_report = evaluate_root_anatomy_energy(&doubled).unwrap();

        close(
            doubled_report.root_surface_nutrient_rate_mg_per_hour,
            2.0 * base_report.root_surface_nutrient_rate_mg_per_hour,
            1e-12,
        );
        close(
            doubled_report.fungal_nutrient_rate_mg_per_hour,
            2.0 * base_report.fungal_nutrient_rate_mg_per_hour,
            1e-12,
        );
        close(
            doubled_report.non_mycorrhizal_nutrient_per_energy_mg_per_j,
            base_report.non_mycorrhizal_nutrient_per_energy_mg_per_j,
            1e-12,
        );
        close(
            doubled_report.mycorrhizal_root_nutrient_per_energy_mg_per_j,
            base_report.mycorrhizal_root_nutrient_per_energy_mg_per_j,
            1e-12,
        );
    }
}
