#![forbid(unsafe_code)]
//! Research-only numerical reference for homogeneous and two-environment
//! screened D-D thick-target yield models.
//!
//! This is not a validated nuclear-data library, a heterogeneous target
//! transport model, or an experimental reproduction. Source-specific S-factor,
//! stopping-power, beam-spread and uncertainty inputs must be supplied by the
//! caller and versioned in an external evidence manifest.

use std::fmt;

mod detector;
pub use detector::DetectorResponseMatrix;

mod measurement_bias;
pub use measurement_bias::apply_feature_conditioned_multiplicative_bias;

mod sparse_bias_inference;
pub use sparse_bias_inference::{fit_sparse_bayesian_poisson_bias, SparseBayesianBiasResult};

mod energy_bias_basis;
pub use energy_bias_basis::{
    aiachne_bias_basis_energy_domain, aiachne_global_scale_index_by_coefficient,
    build_grouped_energy_bias_design, project_grouped_energy_bias_field, GroupedEnergyBiasDesign,
    AIACHNE_BIAS_DOMAIN_LOWER_SUBTRACTION_MEV, AIACHNE_BIAS_DOMAIN_UPPER_ADDITION_MEV,
    ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP, ENERGY_BIAS_CENTRES_PER_SCALE,
    ENERGY_BIAS_LENGTH_SCALES_LOG10,
};

mod correlated_bias_inference;
pub use correlated_bias_inference::{
    fit_sparse_bayesian_correlated_log_bias, CorrelatedLogBiasResult,
};

mod raw_scale_bias_likelihood;
pub use raw_scale_bias_likelihood::{
    raw_scale_gaussian_multiplicative_bias_log_likelihood, RawScaleGaussianBiasLikelihood,
};

mod grouped_horseshoe_hmc;
pub use grouped_horseshoe_hmc::{
    run_grouped_horseshoe_hmc_chain, run_grouped_horseshoe_hmc_chains, GroupedHmcChain,
    GroupedHmcDraw, GroupedHmcInitialization,
};

mod horseshoe_posterior;
pub use horseshoe_posterior::{
    grouped_raw_scale_horseshoe_log_posterior, raw_scale_horseshoe_log_posterior, GlobalScalePrior,
    GroupedHorseshoePosteriorInput, GroupedRawScaleHorseshoePosterior,
    PreparedGroupedHorseshoeEvaluation, PreparedGroupedRawScaleHorseshoeModel,
    RawScaleHorseshoePosterior,
};

mod prepared_horseshoe;
pub use prepared_horseshoe::PreparedRawScaleHorseshoeModel;

pub(crate) mod horseshoe_hmc;
pub use horseshoe_hmc::{
    run_horseshoe_hmc_chain, split_rhat_for_coefficient, HorseshoeHmcChain, HorseshoeHmcConfig,
    HorseshoeHmcDraw,
};

mod sampler_diagnostics;
pub use sampler_diagnostics::{
    diagnose_grouped_hmc_quantity, diagnose_horseshoe_coefficient, GroupedHmcQuantity,
    HmcDiagnosticSummary,
};

mod batch_design;
pub use batch_design::{
    enumerate_batch_portfolios, select_greedy_batch_portfolio, BatchDesignCandidate,
    BatchPortfolioEnumeration, BatchPortfolioScore, GreedyBatchSelection,
};

mod experiment_design;
pub use experiment_design::{
    combine_fisher_information, identifiability_summary, pairwise_poisson_kl_matrix,
    poisson_fisher_information, poisson_kl_divergence, IdentifiabilitySummary,
};

mod shared_reference_normalization;
pub use shared_reference_normalization::{
    aiachne_compatible_overlap_geometry, linear_reference_projection,
    normalize_spectrum_with_shared_reference, shared_reference_cross_covariance,
    trapezoidal_integration_weights, ReferenceOverlapGeometry, SharedReferenceNormalizationInput,
    SharedReferenceNormalizationResult,
};

mod blind_attribution;
pub use blind_attribution::{
    generate_blind_attribution_dataset, AttributionConfounding, BlindAttributionConfig,
    BlindAttributionDataset, BlindAttributionPublicData, BlindAttributionTruth,
};

mod common_cause_covariance;
pub use common_cause_covariance::{
    assemble_joint_experiment_covariance, ExperimentNoiseModel, JointExperimentCovariance,
    SharedUncertaintyContribution,
};

mod nuisance_identifiability;
pub use nuisance_identifiability::profile_nuisance_fisher_information;

mod response_uncertainty;
pub use response_uncertainty::{DetectorResponseEnsemble, PredictiveCountMoments};

mod multifidelity_design;
pub use multifidelity_design::{
    nested_control_variate_estimate, robust_design_scores, select_maximin_design,
    ControlVariateEstimate, RobustDesignScore,
};

mod spectrum_metrics;
pub use spectrum_metrics::{covariance_profiled_spectrum_shape_chi_square, SpectrumShapeScore};

mod measurement;
pub use measurement::{apply_depth_observation_operator, DepthObservedValue, DepthResponseKernel};
mod material_state;
pub use material_state::{diffuse_no_flux_1d, diffusion_stability_limit_s, source_loss_step};

const BARN_TO_CM2: f64 = 1.0e-24;

#[derive(Debug, Clone, PartialEq)]
pub enum ModelError {
    InvalidParameter(&'static str),
    NonFiniteValue(&'static str),
    NonPositiveValue(&'static str),
    InvalidIntegrationSteps,
    MismatchedDataLengths,
    InvalidCovariance(&'static str),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter(name) => write!(f, "invalid parameter: {name}"),
            Self::NonFiniteValue(name) => write!(f, "non-finite value: {name}"),
            Self::NonPositiveValue(name) => write!(f, "non-positive value: {name}"),
            Self::InvalidIntegrationSteps => {
                write!(f, "Simpson integration requires an even step count >= 2")
            }
            Self::MismatchedDataLengths => write!(
                f,
                "observation, prediction, and uncertainty arrays must have equal non-zero length"
            ),
            Self::InvalidCovariance(name) => write!(f, "invalid covariance matrix: {name}"),
        }
    }
}

impl std::error::Error for ModelError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyKev(f64);

impl EnergyKev {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("energy_keV_cm"));
        }
        if value <= 0.0 {
            return Err(ModelError::NonPositiveValue("energy_keV_cm"));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GamowEnergyKev(f64);

impl GamowEnergyKev {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("gamow_energy_keV_cm"));
        }
        if value <= 0.0 {
            return Err(ModelError::NonPositiveValue("gamow_energy_keV_cm"));
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetDensityPerCm3(f64);

impl TargetDensityPerCm3 {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("target_density_cm^-3"));
        }
        if value <= 0.0 {
            return Err(ModelError::NonPositiveValue("target_density_cm^-3"));
        }
        Ok(Self(value))
    }
}

fn checked_nonnegative(value: f64, name: &'static str) -> Result<f64, ModelError> {
    if !value.is_finite() {
        return Err(ModelError::NonFiniteValue(name));
    }
    if value < 0.0 {
        return Err(ModelError::InvalidParameter(name));
    }
    Ok(value)
}

fn checked_positive(value: f64, name: &'static str) -> Result<f64, ModelError> {
    if !value.is_finite() {
        return Err(ModelError::NonFiniteValue(name));
    }
    if value <= 0.0 {
        return Err(ModelError::NonPositiveValue(name));
    }
    Ok(value)
}

/// ALR-style screened cross-section in barns.
///
/// Energy and Gamow energy are in the center-of-mass frame (keV). The
/// astrophysical S factor is supplied in keV·barn and the effective screening
/// potential in keV. The caller is responsible for supplying a validated,
/// versioned S-factor function over the entire integration domain.
pub fn screened_cross_section_barn<FS>(
    energy_cm: EnergyKev,
    gamow_energy_cm: GamowEnergyKev,
    screening_kev: f64,
    s_factor_kev_barn: FS,
) -> Result<f64, ModelError>
where
    FS: Fn(f64) -> f64,
{
    let energy = energy_cm.get();
    let screening = checked_nonnegative(screening_kev, "screening_kev")?;
    let s_factor = checked_positive(s_factor_kev_barn(energy), "S_factor_keV_barn")?;
    let shifted_energy = energy + screening;
    let cross_section = s_factor / (energy * shifted_energy).sqrt()
        * (-(gamow_energy_cm.0 / shifted_energy).sqrt()).exp();

    if !cross_section.is_finite() || cross_section < 0.0 {
        return Err(ModelError::InvalidParameter(
            "cross_section must be finite and >= 0",
        ));
    }
    Ok(cross_section)
}

/// One light-output slice of the source-described EJ-309 neutron PSD gate.
///
/// Values are source-calibrated metadata; this type does not estimate gamma
/// distributions or detector efficiency. For the 2025 paper, slices were
/// 5 keVee wide, the lower PSD boundary was mu_gamma + 5*sigma_gamma, the
/// upper boundary was another 0.2 PSD units above it, and light output below
/// 50 keVee was rejected.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PsdGateSlice {
    pub light_output_min_kev_ee: f64,
    pub light_output_max_kev_ee: f64,
    pub gamma_mean_psd: f64,
    pub gamma_sigma_psd: f64,
    pub upper_offset_psd: f64,
}

/// Apply one calibrated neutron PSD slice to a pulse observation.
///
/// Returns false for observations outside the slice or below the configured
/// light-output threshold. Returns an error for malformed calibration values.
pub fn neutron_psd_gate_accepts(
    light_output_kev_ee: f64,
    psd_value: f64,
    min_light_output_kev_ee: f64,
    slice: PsdGateSlice,
) -> Result<bool, ModelError> {
    for (value, name) in [
        (light_output_kev_ee, "light_output_keVee"),
        (psd_value, "psd_value"),
        (min_light_output_kev_ee, "minimum_light_output_keVee"),
        (
            slice.light_output_min_kev_ee,
            "slice_light_output_min_keVee",
        ),
        (
            slice.light_output_max_kev_ee,
            "slice_light_output_max_keVee",
        ),
        (slice.gamma_mean_psd, "gamma_mean_psd"),
        (slice.gamma_sigma_psd, "gamma_sigma_psd"),
        (slice.upper_offset_psd, "upper_offset_psd"),
    ] {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue(name));
        }
    }
    if slice.light_output_min_kev_ee < 0.0
        || slice.light_output_max_kev_ee <= slice.light_output_min_kev_ee
    {
        return Err(ModelError::InvalidParameter(
            "invalid_light_output_slice_bounds",
        ));
    }
    if slice.gamma_sigma_psd <= 0.0 {
        return Err(ModelError::NonPositiveValue("gamma_sigma_psd"));
    }
    if slice.upper_offset_psd <= 0.0 {
        return Err(ModelError::NonPositiveValue("upper_offset_psd"));
    }
    if min_light_output_kev_ee < 0.0 {
        return Err(ModelError::InvalidParameter(
            "minimum_light_output_must_be_nonnegative",
        ));
    }

    if light_output_kev_ee < min_light_output_kev_ee
        || light_output_kev_ee < slice.light_output_min_kev_ee
        || light_output_kev_ee >= slice.light_output_max_kev_ee
    {
        return Ok(false);
    }

    let lower = slice.gamma_mean_psd + 5.0 * slice.gamma_sigma_psd;
    let upper = lower + slice.upper_offset_psd;
    Ok(psd_value >= lower && psd_value <= upper)
}

/// One per-deuteron component in a prompt-dissociation beam model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeamEnergyComponent {
    /// Center-of-mass energy for D projectile on stationary D target, keV.
    pub energy_cm_kev: f64,
    /// Fraction of the total deuteron flux carried by this component.
    pub deuteron_flux_fraction: f64,
}

/// Prompt-dissociation limit for a singly charged D+ / D2+ beam.
///
/// Fractions are **ion-current fractions**, not deuteron-flux fractions, and
/// must sum to one. A singly charged ion accelerated through V kV receives
/// approximately V keV total kinetic energy. D+ therefore has Ecm ~= V/2.
/// If D2+ promptly dissociates into two D fragments that retain the molecular
/// velocity, each fragment receives approximately V/2 keV lab energy and
/// Ecm ~= V/4. This is a limiting scenario, not an assertion that prompt
/// dissociation occurred in a particular experiment.
///
/// The returned component weights are normalized by deuteron flux:
/// f_D+ = f_ion(D+) / [f_ion(D+) + 2 f_ion(D2+)]
/// f_D2+ = 2 f_ion(D2+) / [f_ion(D+) + 2 f_ion(D2+)].
pub fn prompt_dissociation_beam_components(
    accelerating_potential_kv: f64,
    d_plus_ion_current_fraction: f64,
    d2_plus_ion_current_fraction: f64,
) -> Result<[BeamEnergyComponent; 2], ModelError> {
    let potential = checked_positive(accelerating_potential_kv, "accelerating_potential_kV")?;
    for (fraction, name) in [
        (d_plus_ion_current_fraction, "D_plus_ion_current_fraction"),
        (d2_plus_ion_current_fraction, "D2_plus_ion_current_fraction"),
    ] {
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err(ModelError::InvalidParameter(name));
        }
    }
    if (d_plus_ion_current_fraction + d2_plus_ion_current_fraction - 1.0).abs() > 1.0e-6 {
        return Err(ModelError::InvalidParameter(
            "D_plus_and_D2_plus_ion_current_fractions_must_sum_to_one",
        ));
    }

    let deuterons_per_charge = d_plus_ion_current_fraction + 2.0 * d2_plus_ion_current_fraction;
    if deuterons_per_charge <= 0.0 {
        return Err(ModelError::NonPositiveValue("deuterons_per_charge"));
    }

    let d_plus_weight = d_plus_ion_current_fraction / deuterons_per_charge;
    let d2_plus_weight = (2.0 * d2_plus_ion_current_fraction) / deuterons_per_charge;

    Ok([
        BeamEnergyComponent {
            energy_cm_kev: potential / 2.0,
            deuteron_flux_fraction: d_plus_weight,
        },
        BeamEnergyComponent {
            energy_cm_kev: potential / 4.0,
            deuteron_flux_fraction: d2_plus_weight,
        },
    ])
}

/// Total deuterium-atom arrival rate represented by a singly charged
/// D+ / D2+ ion current. Fractions are ion-current fractions.
pub fn total_deuteron_rate_per_second(
    ion_current_a: f64,
    d_plus_ion_current_fraction: f64,
    d2_plus_ion_current_fraction: f64,
) -> Result<f64, ModelError> {
    let current = checked_nonnegative(ion_current_a, "ion_current_A")?;
    for (fraction, name) in [
        (d_plus_ion_current_fraction, "D_plus_ion_current_fraction"),
        (d2_plus_ion_current_fraction, "D2_plus_ion_current_fraction"),
    ] {
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err(ModelError::InvalidParameter(name));
        }
    }
    if (d_plus_ion_current_fraction + d2_plus_ion_current_fraction - 1.0).abs() > 1.0e-6 {
        return Err(ModelError::InvalidParameter(
            "ion_current_fractions_must_sum_to_one",
        ));
    }
    const ELEMENTARY_CHARGE_C: f64 = 1.602_176_634e-19;
    Ok((current / ELEMENTARY_CHARGE_C)
        * (d_plus_ion_current_fraction + 2.0 * d2_plus_ion_current_fraction))
}

/// One measured/sampled cell in a two-dimensional beam-current-density map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeamProfileCell {
    pub x_cm: f64,
    pub y_cm: f64,
    pub area_cm2: f64,
    /// Measured ion-current density for the combined singly charged D+ / D2+
    /// beam, in A/cm^2. Species fractions are supplied separately.
    pub ion_current_density_a_cm2: f64,
}

/// A spatially resolved beam map reconciled against an independent Faraday-cup
/// total current. Cell order is preserved in all returned arrays.
#[derive(Debug, Clone, PartialEq)]
pub struct ReconciledBeamProfile {
    pub integrated_map_current_a: f64,
    pub faraday_cup_current_a: f64,
    pub relative_current_mismatch: f64,
    /// Total deuteron arrival rate in each cell, atoms/s.
    pub deuteron_rate_by_cell_s: Vec<f64>,
    /// Fraction of total deuteron arrival rate represented by each cell.
    pub deuteron_flux_fraction_by_cell: Vec<f64>,
    /// Average deuteron areal flux density per cell, atoms/(cm^2 s).
    pub deuteron_flux_density_by_cell_cm2_s: Vec<f64>,
}

/// Reconcile an arbitrary 2-D measured beam profile with an independently
/// measured Faraday-cup current and convert it to deuteron flux.
///
/// This function makes no Gaussian, circular-symmetry or uniform-profile
/// assumption. Each cell contributes j_i * area_i to the integrated ion
/// current. The sum must agree with Faraday-cup current within the explicitly
/// supplied relative tolerance. The map is not silently renormalized to hide
/// a disagreement: failure is fail-closed.
///
/// The species fractions are ion-current fractions for singly charged D+ and
/// D2+ and must sum to one. Each D2+ contributes two D nuclei to the flux
/// bookkeeping. This does not model dissociation, implantation or stopping.
pub fn reconcile_measured_beam_profile(
    cells: &[BeamProfileCell],
    faraday_cup_current_a: f64,
    d_plus_ion_current_fraction: f64,
    d2_plus_ion_current_fraction: f64,
    relative_tolerance: f64,
) -> Result<ReconciledBeamProfile, ModelError> {
    if cells.is_empty() {
        return Err(ModelError::InvalidParameter(
            "beam_profile_must_not_be_empty",
        ));
    }
    let cup_current = checked_positive(faraday_cup_current_a, "faraday_cup_current_A")?;
    let tolerance = checked_nonnegative(relative_tolerance, "beam_profile_relative_tolerance")?;
    if !tolerance.is_finite() {
        return Err(ModelError::NonFiniteValue(
            "beam_profile_relative_tolerance",
        ));
    }
    for (fraction, name) in [
        (d_plus_ion_current_fraction, "D_plus_ion_current_fraction"),
        (d2_plus_ion_current_fraction, "D2_plus_ion_current_fraction"),
    ] {
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err(ModelError::InvalidParameter(name));
        }
    }
    if (d_plus_ion_current_fraction + d2_plus_ion_current_fraction - 1.0).abs() > 1.0e-6 {
        return Err(ModelError::InvalidParameter(
            "ion_current_fractions_must_sum_to_one",
        ));
    }

    const ELEMENTARY_CHARGE_C: f64 = 1.602_176_634e-19;
    let deuterons_per_charge = d_plus_ion_current_fraction + 2.0 * d2_plus_ion_current_fraction;
    let mut integrated_current = 0.0;
    let mut cell_currents = Vec::with_capacity(cells.len());

    for cell in cells {
        for (value, name) in [
            (cell.x_cm, "beam_cell_x_cm"),
            (cell.y_cm, "beam_cell_y_cm"),
            (cell.area_cm2, "beam_cell_area_cm2"),
            (
                cell.ion_current_density_a_cm2,
                "beam_cell_current_density_A_cm2",
            ),
        ] {
            if !value.is_finite() {
                return Err(ModelError::NonFiniteValue(name));
            }
        }
        if cell.area_cm2 <= 0.0 || cell.ion_current_density_a_cm2 < 0.0 {
            return Err(ModelError::InvalidParameter(
                "beam_cell_area_must_be_positive_and_current_density_nonnegative",
            ));
        }
        let current = cell.area_cm2 * cell.ion_current_density_a_cm2;
        integrated_current += current;
        cell_currents.push(current);
    }

    if !integrated_current.is_finite() || integrated_current <= 0.0 {
        return Err(ModelError::NonPositiveValue(
            "integrated_beam_map_current_A",
        ));
    }
    let relative_mismatch = (integrated_current - cup_current).abs() / cup_current;
    if relative_mismatch > tolerance {
        return Err(ModelError::InvalidParameter(
            "beam_map_current_does_not_reconcile_with_faraday_cup",
        ));
    }

    let mut rates = Vec::with_capacity(cells.len());
    let mut fractions = Vec::with_capacity(cells.len());
    let mut flux_densities = Vec::with_capacity(cells.len());
    let mut total_deuteron_rate = 0.0;

    for (cell, current) in cells.iter().zip(cell_currents.iter()) {
        let deuteron_rate = current / ELEMENTARY_CHARGE_C * deuterons_per_charge;
        total_deuteron_rate += deuteron_rate;
        rates.push(deuteron_rate);
        flux_densities.push(deuteron_rate / cell.area_cm2);
    }

    if !total_deuteron_rate.is_finite() || total_deuteron_rate <= 0.0 {
        return Err(ModelError::NonPositiveValue("total_deuteron_rate_s"));
    }
    for rate in &rates {
        fractions.push(*rate / total_deuteron_rate);
    }

    Ok(ReconciledBeamProfile {
        integrated_map_current_a: integrated_current,
        faraday_cup_current_a: cup_current,
        relative_current_mismatch: relative_mismatch,
        deuteron_rate_by_cell_s: rates,
        deuteron_flux_fraction_by_cell: fractions,
        deuteron_flux_density_by_cell_cm2_s: flux_densities,
    })
}

/// Axisymmetric Gaussian footprint used only as an explicit baseline model.
///
/// The profile is normalized over the infinite plane, so its spatial integral
/// equals the supplied total deuteron arrival rate. A measured 2-D beam map
/// should replace this baseline when available; a reported approximate spot
/// area does not determine sigma uniquely.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaussianBeamFootprint {
    pub sigma_cm: f64,
}

impl GaussianBeamFootprint {
    pub fn new(sigma_cm: f64) -> Result<Self, ModelError> {
        checked_positive(sigma_cm, "beam_sigma_cm")?;
        Ok(Self { sigma_cm })
    }

    /// Deuteron areal flux density at radial distance r, in D cm^-2 s^-1.
    pub fn deuteron_flux_density_cm2_s(
        self,
        total_deuteron_rate_s: f64,
        radius_cm: f64,
    ) -> Result<f64, ModelError> {
        let total_rate = checked_nonnegative(total_deuteron_rate_s, "total_deuteron_rate_s")?;
        let radius = checked_nonnegative(radius_cm, "beam_radius_cm")?;
        let sigma_sq = self.sigma_cm * self.sigma_cm;
        let density = total_rate / (2.0 * std::f64::consts::PI * sigma_sq)
            * (-(radius * radius) / (2.0 * sigma_sq)).exp();
        if !density.is_finite() || density < 0.0 {
            return Err(ModelError::InvalidParameter(
                "beam_flux_density_must_be_finite_and_nonnegative",
            ));
        }
        Ok(density)
    }
}

/// Homogeneous screened thick-target yield per incident projectile.
///
/// Implements
///
/// n_D * integral_0^Ecm [sigma(Ecm, Ue) / S_stop_cm(Ecm)] dEcm
///
/// where:
/// - Ecm and Gamow energy are in keV in the center-of-mass frame;
/// - S(Ecm) is in keV·barn;
/// - n_D is target deuterium number density in cm^-3;
/// - stopping_kev_cm_per_cm is **-dEcm/dx** in keV_cm/cm (not automatically
///   the projectile laboratory-frame stopping power);
/// - screening_kev is a non-negative effective potential in keV.
///
/// For a D projectile on a stationary D target, Ecm = Elab/2. If a source
/// provides projectile stopping in lab energy per length, convert it
/// consistently: dEcm/dx = (1/2) dElab/dx. The returned reaction yield per
/// incident projectile is dimensionless.
///
/// Inputs are deliberately supplied by the caller. This function does not
/// contain a nuclear-data table, a material stopping table, or uncertainty
/// propagation. The caller must record those sources and uncertainties.
pub fn homogeneous_thick_target_yield<FS, FStop, FU>(
    incident_energy_cm: EnergyKev,
    gamow_energy_cm: GamowEnergyKev,
    target_density: TargetDensityPerCm3,
    steps: usize,
    s_factor_kev_barn: FS,
    stopping_kev_cm_per_cm: FStop,
    screening_kev: FU,
) -> Result<f64, ModelError>
where
    FS: Fn(f64) -> f64,
    FStop: Fn(f64) -> f64,
    FU: Fn(f64) -> f64,
{
    if steps < 2 || !steps.is_multiple_of(2) {
        return Err(ModelError::InvalidIntegrationSteps);
    }

    // Ecm = t^2 regularizes the integrable Ecm^-1/2 endpoint for Ue > 0.
    let t_max = incident_energy_cm.get().sqrt();
    let h = t_max / steps as f64;
    let mut weighted_sum = 0.0;

    for i in 0..=steps {
        let t = i as f64 * h;
        let energy_cm = t * t;
        let s_factor = checked_positive(s_factor_kev_barn(energy_cm), "S_factor_keV_barn")?;
        let stopping =
            checked_positive(stopping_kev_cm_per_cm(energy_cm), "stopping_kev_cm_per_cm")?;
        let screening = checked_nonnegative(screening_kev(energy_cm), "screening_kev")?;

        // This is 2*t*sigma(t^2,Ue), the transformed cross-section integrand.
        let transformed = if t == 0.0 {
            if screening > 0.0 {
                2.0 * s_factor / screening.sqrt() * (-(gamow_energy_cm.0 / screening).sqrt()).exp()
            } else {
                // For Ue=0 the Gamow factor suppresses the endpoint to zero.
                0.0
            }
        } else {
            let shifted_energy = energy_cm + screening;
            let cross_section_barn = s_factor / (energy_cm * shifted_energy).sqrt()
                * (-(gamow_energy_cm.0 / shifted_energy).sqrt()).exp();
            2.0 * t * cross_section_barn
        };

        let integrand = target_density.0 * BARN_TO_CM2 * transformed / stopping;
        if !integrand.is_finite() || integrand < 0.0 {
            return Err(ModelError::InvalidParameter(
                "yield integrand must be finite and >= 0",
            ));
        }

        let coefficient = if i == 0 || i == steps {
            1.0
        } else if i % 2 == 0 {
            2.0
        } else {
            4.0
        };
        weighted_sum += coefficient * integrand;
    }

    let result = weighted_sum * h / 3.0;
    if !result.is_finite() || result < 0.0 {
        return Err(ModelError::InvalidParameter(
            "yield result must be finite and >= 0",
        ));
    }
    Ok(result)
}

/// Relative homogeneous thick-target yield R(E) = Y(E) / Y(E_ref).
///
/// S-factor and stopping closures use Ecm in keV; stopping must be -dEcm/dx
/// in keV_cm/cm. This matches the relative-yield normalization used in the
/// 2026 paper, with the reference energy explicitly supplied by the caller.
// Keep unit-bearing scalar inputs and the three physical closures explicit;
// this is intentionally a high-arity scientific interface rather than an opaque tuple.
#[allow(clippy::too_many_arguments)]
pub fn relative_thick_target_yield<FS, FStop, FU>(
    energy_cm: EnergyKev,
    reference_energy_cm: EnergyKev,
    gamow_energy_cm: GamowEnergyKev,
    target_density: TargetDensityPerCm3,
    steps: usize,
    s_factor_kev_barn: FS,
    stopping_kev_cm_per_cm: FStop,
    screening_kev: FU,
) -> Result<f64, ModelError>
where
    FS: Fn(f64) -> f64,
    FStop: Fn(f64) -> f64,
    FU: Fn(f64) -> f64,
{
    let observed = homogeneous_thick_target_yield(
        energy_cm,
        gamow_energy_cm,
        target_density,
        steps,
        &s_factor_kev_barn,
        &stopping_kev_cm_per_cm,
        &screening_kev,
    )?;
    let reference = homogeneous_thick_target_yield(
        reference_energy_cm,
        gamow_energy_cm,
        target_density,
        steps,
        &s_factor_kev_barn,
        &stopping_kev_cm_per_cm,
        &screening_kev,
    )?;
    if reference <= 0.0 {
        return Err(ModelError::NonPositiveValue("reference_yield"));
    }
    Ok(observed / reference)
}

/// Weighted yield for a discrete measured or hypothesized incident spectrum.
///
/// Each entry is a component or energy bin with a center-of-mass energy and
/// its fraction of total deuteron flux. Fractions must sum to one. This accepts
/// arbitrary measured histograms/quadrature nodes; it does not assume a
/// Gaussian energy distribution.
pub fn weighted_beam_spectrum_yield<FS, FStop, FU>(
    components: &[BeamEnergyComponent],
    gamow_energy_cm: GamowEnergyKev,
    target_density: TargetDensityPerCm3,
    steps: usize,
    s_factor_kev_barn: FS,
    stopping_kev_cm_per_cm: FStop,
    screening_kev: FU,
) -> Result<f64, ModelError>
where
    FS: Fn(f64) -> f64,
    FStop: Fn(f64) -> f64,
    FU: Fn(f64) -> f64,
{
    if components.is_empty() {
        return Err(ModelError::InvalidParameter(
            "beam_spectrum_must_not_be_empty",
        ));
    }
    let mut total = 0.0;
    let mut weight_sum = 0.0;
    for component in components {
        let weight =
            checked_nonnegative(component.deuteron_flux_fraction, "deuteron_flux_fraction")?;
        let energy = EnergyKev::new(component.energy_cm_kev)?;
        let component_yield = homogeneous_thick_target_yield(
            energy,
            gamow_energy_cm,
            target_density,
            steps,
            &s_factor_kev_barn,
            &stopping_kev_cm_per_cm,
            &screening_kev,
        )?;
        total += weight * component_yield;
        weight_sum += weight;
    }
    if (weight_sum - 1.0).abs() > 1.0e-6 {
        return Err(ModelError::InvalidParameter(
            "deuteron_flux_fractions_must_sum_to_one",
        ));
    }
    if !total.is_finite() || total < 0.0 {
        return Err(ModelError::InvalidParameter(
            "weighted_beam_yield_must_be_finite_and_nonnegative",
        ));
    }
    Ok(total)
}

/// First-order thick-target yield for the two prompt-dissociation beam
/// components. Both components use the same S-factor and stopping functions;
/// production work should replace this with species/depth-specific transport
/// and dissociation profiles when those data are available.
pub fn prompt_dissociation_weighted_yield<FS, FStop, FU>(
    components: [BeamEnergyComponent; 2],
    gamow_energy_cm: GamowEnergyKev,
    target_density: TargetDensityPerCm3,
    steps: usize,
    s_factor_kev_barn: FS,
    stopping_kev_cm_per_cm: FStop,
    screening_kev: FU,
) -> Result<f64, ModelError>
where
    FS: Fn(f64) -> f64,
    FStop: Fn(f64) -> f64,
    FU: Fn(f64) -> f64,
{
    weighted_beam_spectrum_yield(
        &components,
        gamow_energy_cm,
        target_density,
        steps,
        s_factor_kev_barn,
        stopping_kev_cm_per_cm,
        screening_kev,
    )
}

/// Homogeneous ALR enhancement E_scr(E; Ue) = Y_scr(E) / Y_bare(E),
/// using the closed-form expression printed as Eq. (12) in the 2026 paper.
pub fn screening_enhancement(
    energy_cm: EnergyKev,
    gamow_energy_cm: GamowEnergyKev,
    screening_kev: f64,
) -> Result<f64, ModelError> {
    let energy = energy_cm.get();
    let screening = checked_nonnegative(screening_kev, "screening_kev")?;
    if screening == 0.0 {
        return Ok(1.0);
    }

    let shifted_energy = energy + screening;
    let prefactor = (energy / shifted_energy).sqrt();
    let exponent =
        (gamow_energy_cm.0 / energy).sqrt() - (gamow_energy_cm.0 / shifted_energy).sqrt();
    let result = prefactor * exponent.exp();

    if !result.is_finite() || result <= 0.0 {
        return Err(ModelError::InvalidParameter(
            "screening enhancement must be finite and > 0",
        ));
    }
    Ok(result)
}

/// Equation 13 two-environment mixture:
/// As * E_scr(E; Ue_surface) + (1-As) * E_scr(E; Ue_bulk).
///
/// This is distinct from the logistic effective-potential model in Equation 16.
pub fn two_environment_mixture_enhancement(
    energy_cm: EnergyKev,
    gamow_energy_cm: GamowEnergyKev,
    surface_amplitude: f64,
    surface_screening_kev: f64,
    bulk_screening_kev: f64,
) -> Result<f64, ModelError> {
    if !surface_amplitude.is_finite() || !(0.0..=1.0).contains(&surface_amplitude) {
        return Err(ModelError::InvalidParameter(
            "surface_amplitude must be in [0, 1]",
        ));
    }
    let surface = screening_enhancement(energy_cm, gamow_energy_cm, surface_screening_kev)?;
    let bulk = screening_enhancement(energy_cm, gamow_energy_cm, bulk_screening_kev)?;
    let result = surface_amplitude * surface + (1.0 - surface_amplitude) * bulk;

    if !result.is_finite() || result <= 0.0 {
        return Err(ModelError::InvalidParameter(
            "mixture enhancement must be finite and > 0",
        ));
    }
    Ok(result)
}

fn logistic(value: f64) -> Result<f64, ModelError> {
    if !value.is_finite() {
        return Err(ModelError::NonFiniteValue("logistic_argument"));
    }
    // Stable evaluation of 1 / (1 + exp(-x)).
    Ok(if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exp_value = value.exp();
        exp_value / (1.0 + exp_value)
    })
}

/// Literal Equation 16 as printed in Karahadian et al. (2026):
///
/// Ue,b + (Ue,s - Ue,b) / (1 + exp[-(Ecm-E0)/Delta]).
///
/// Note: this printed direction gives Ue,b at low energy and Ue,s at high
/// energy, opposite to the adjacent paper prose and Fig. 3c. It is retained
/// only as a source-faithful diagnostic variant, not the default interpretation.
pub fn sigmoid_screening_potential_printed_equation(
    energy_cm: EnergyKev,
    bulk_screening_kev: f64,
    surface_screening_kev: f64,
    crossover_energy_cm: EnergyKev,
    width_kev: f64,
) -> Result<f64, ModelError> {
    let bulk = checked_nonnegative(bulk_screening_kev, "bulk_screening_kev")?;
    let surface = checked_nonnegative(surface_screening_kev, "surface_screening_kev")?;
    let width = checked_positive(width_kev, "sigmoid_width_keV")?;
    let x = (energy_cm.get() - crossover_energy_cm.get()) / width;
    Ok(bulk + (surface - bulk) * logistic(x)?)
}

/// Direction implied by the paper's prose and Fig. 3c:
/// low energy -> surface screening; high energy -> bulk screening.
///
/// This reverses the sign in the printed Eq. 16 exponent. It is an explicitly
/// labelled interpretation pending confirmation against the authors' fit code.
pub fn sigmoid_screening_potential_prose_direction(
    energy_cm: EnergyKev,
    bulk_screening_kev: f64,
    surface_screening_kev: f64,
    crossover_energy_cm: EnergyKev,
    width_kev: f64,
) -> Result<f64, ModelError> {
    let bulk = checked_nonnegative(bulk_screening_kev, "bulk_screening_kev")?;
    let surface = checked_nonnegative(surface_screening_kev, "surface_screening_kev")?;
    let width = checked_positive(width_kev, "sigmoid_width_keV")?;
    let x = (crossover_energy_cm.get() - energy_cm.get()) / width;
    Ok(bulk + (surface - bulk) * logistic(x)?)
}

/// Heterogeneous enhancement under the prose/Fig. 3c direction variant.
pub fn sigmoid_heterogeneous_enhancement_prose_direction(
    energy_cm: EnergyKev,
    gamow_energy_cm: GamowEnergyKev,
    bulk_screening_kev: f64,
    surface_screening_kev: f64,
    crossover_energy_cm: EnergyKev,
    width_kev: f64,
) -> Result<f64, ModelError> {
    let effective = sigmoid_screening_potential_prose_direction(
        energy_cm,
        bulk_screening_kev,
        surface_screening_kev,
        crossover_energy_cm,
        width_kev,
    )?;
    screening_enhancement(energy_cm, gamow_energy_cm, effective)
}

/// Heterogeneous enhancement under the literal printed Eq. 16 direction.
/// Retained separately from the prose/Fig. 3c direction until the fit
/// implementation is source-verified.
pub fn sigmoid_heterogeneous_enhancement_printed_equation(
    energy_cm: EnergyKev,
    gamow_energy_cm: GamowEnergyKev,
    bulk_screening_kev: f64,
    surface_screening_kev: f64,
    crossover_energy_cm: EnergyKev,
    width_kev: f64,
) -> Result<f64, ModelError> {
    let effective = sigmoid_screening_potential_printed_equation(
        energy_cm,
        bulk_screening_kev,
        surface_screening_kev,
        crossover_energy_cm,
        width_kev,
    )?;
    screening_enhancement(energy_cm, gamow_energy_cm, effective)
}

/// Weighted log-space least-squares objective (Eq. 17 convention).
///
/// sigma_ln_ratio must be strictly positive. This is a diagnostic objective,
/// not a full likelihood when uncertainties are correlated or non-Gaussian.
pub fn weighted_log_space_chi_square(
    observed_ratios: &[f64],
    predicted_ratios: &[f64],
    sigma_ln_ratios: &[f64],
) -> Result<f64, ModelError> {
    if observed_ratios.is_empty()
        || observed_ratios.len() != predicted_ratios.len()
        || observed_ratios.len() != sigma_ln_ratios.len()
    {
        return Err(ModelError::MismatchedDataLengths);
    }

    let mut sum = 0.0;
    for ((observed, predicted), sigma) in observed_ratios
        .iter()
        .zip(predicted_ratios.iter())
        .zip(sigma_ln_ratios.iter())
    {
        let observed = checked_positive(*observed, "observed_ratio")?;
        let predicted = checked_positive(*predicted, "predicted_ratio")?;
        let sigma = checked_positive(*sigma, "sigma_ln_ratio")?;
        let residual = (observed.ln() - predicted.ln()) / sigma;
        sum += residual * residual;
    }

    if !sum.is_finite() {
        return Err(ModelError::InvalidParameter("chi_square must be finite"));
    }
    Ok(sum)
}

/// Covariance of log relative-yield observations when each ratio shares
/// one independently measured reference yield.
///
/// For r_i = ln(Y_i) - ln(Y_ref), independent numerator measurements imply
/// Cov(r_i,r_j) = delta_ij Var[ln(Y_i)] + Var[ln(Y_ref)]. This function expects
/// only non-reference numerator points; do not include the reference point
/// itself as an independent ratio observation. Additional correlated
/// systematics must be added to this matrix by the caller.
pub fn shared_reference_log_covariance(
    numerator_log_variances: &[f64],
    reference_log_variance: f64,
) -> Result<Vec<Vec<f64>>, ModelError> {
    if numerator_log_variances.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let reference_variance = checked_positive(reference_log_variance, "reference_log_variance")?;
    let mut covariance = vec![
        vec![reference_variance; numerator_log_variances.len()];
        numerator_log_variances.len()
    ];
    for (i, variance) in numerator_log_variances.iter().enumerate() {
        let variance = checked_positive(*variance, "numerator_log_variance")?;
        covariance[i][i] += variance;
    }
    Ok(covariance)
}

/// Cholesky factorization for a symmetric positive-definite covariance matrix.
/// Returns lower-triangular L such that covariance = L * L^T.
fn cholesky_lower(covariance: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    let n = covariance.len();
    if n == 0 || covariance.iter().any(|row| row.len() != n) {
        return Err(ModelError::InvalidCovariance(
            "matrix must be non-empty and square",
        ));
    }

    for (i, row) in covariance.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            if !value.is_finite() {
                return Err(ModelError::InvalidCovariance("all entries must be finite"));
            }
            let scale = 1.0_f64.max(value.abs()).max(covariance[j][i].abs());
            if (value - covariance[j][i]).abs() > 1.0e-10 * scale {
                return Err(ModelError::InvalidCovariance("matrix must be symmetric"));
            }
        }
        if row[i] <= 0.0 {
            return Err(ModelError::InvalidCovariance(
                "diagonal entries must be positive",
            ));
        }
    }

    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut value = covariance[i][j];
            for (k, lower_value) in lower[i].iter().enumerate().take(j) {
                value -= lower_value * lower[j][k];
            }
            if i == j {
                if !value.is_finite() || value <= 0.0 {
                    return Err(ModelError::InvalidCovariance(
                        "matrix must be positive definite",
                    ));
                }
                lower[i][j] = value.sqrt();
            } else {
                lower[i][j] = value / lower[j][j];
            }
        }
    }
    Ok(lower)
}

/// Generalized least-squares objective for log relative yields:
/// chi-square = (ln R_obs - ln R_pred)^T Sigma^-1 (ln R_obs - ln R_pred).
///
/// The implementation uses a Cholesky solve rather than explicitly inverting
/// Sigma. Covariance should include shared reference counts and correlated
/// systematic effects. This remains a Gaussian likelihood approximation;
/// low-count Poisson data may require a count-space likelihood instead.
pub fn correlated_weighted_log_space_chi_square(
    observed_ratios: &[f64],
    predicted_ratios: &[f64],
    covariance: &[Vec<f64>],
) -> Result<f64, ModelError> {
    if observed_ratios.is_empty() || observed_ratios.len() != predicted_ratios.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    if covariance.len() != observed_ratios.len()
        || covariance
            .iter()
            .any(|row| row.len() != observed_ratios.len())
    {
        return Err(ModelError::InvalidCovariance(
            "dimensions must match the ratio arrays",
        ));
    }

    let residuals = observed_ratios
        .iter()
        .zip(predicted_ratios.iter())
        .map(|(observed, predicted)| {
            let observed = checked_positive(*observed, "observed_ratio")?;
            let predicted = checked_positive(*predicted, "predicted_ratio")?;
            Ok(observed.ln() - predicted.ln())
        })
        .collect::<Result<Vec<f64>, ModelError>>()?;

    let lower = cholesky_lower(covariance)?;
    // Solve L z = residual with forward substitution; chi-square = z^T z.
    let mut whitened = vec![0.0; residuals.len()];
    for i in 0..residuals.len() {
        let mut value = residuals[i];
        for j in 0..i {
            value -= lower[i][j] * whitened[j];
        }
        whitened[i] = value / lower[i][i];
    }
    let score = whitened.iter().map(|value| value * value).sum::<f64>();
    if !score.is_finite() || score < 0.0 {
        return Err(ModelError::InvalidCovariance(
            "generalized chi-square is not finite and nonnegative",
        ));
    }
    Ok(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_yield(screening: f64, steps: usize) -> Result<f64, ModelError> {
        homogeneous_thick_target_yield(
            EnergyKev::new(6.5)?,
            GamowEnergyKev::new(986.0)?,
            TargetDensityPerCm3::new(5.0e22)?,
            steps,
            |_| 50.0,      // synthetic constant S factor, keV·barn
            |_| 1.0e8,     // synthetic stopping, keV_cm/cm
            |_| screening, // synthetic effective screening potential, keV
        )
    }

    fn example_psd_slice() -> PsdGateSlice {
        PsdGateSlice {
            light_output_min_kev_ee: 50.0,
            light_output_max_kev_ee: 55.0,
            gamma_mean_psd: 0.15,
            gamma_sigma_psd: 0.02,
            upper_offset_psd: 0.2,
        }
    }

    #[test]
    fn neutron_psd_gate_accepts_inside_source_defined_window() {
        assert!(
            neutron_psd_gate_accepts(52.5, 0.27, 50.0, example_psd_slice())
                .expect("valid PSD gate")
        );
    }

    #[test]
    fn neutron_psd_gate_rejects_light_output_below_threshold() {
        assert!(
            !neutron_psd_gate_accepts(49.9, 0.27, 50.0, example_psd_slice())
                .expect("valid PSD gate")
        );
    }

    #[test]
    fn neutron_psd_gate_rejects_gamma_side_of_lower_boundary() {
        assert!(
            !neutron_psd_gate_accepts(52.5, 0.249, 50.0, example_psd_slice())
                .expect("valid PSD gate")
        );
    }

    #[test]
    fn neutron_psd_gate_rejects_above_upper_psd_boundary() {
        assert!(
            !neutron_psd_gate_accepts(52.5, 0.451, 50.0, example_psd_slice())
                .expect("valid PSD gate")
        );
    }

    #[test]
    fn neutron_psd_gate_rejects_nonfinite_and_invalid_calibration() {
        assert_eq!(
            neutron_psd_gate_accepts(f64::NAN, 0.27, 50.0, example_psd_slice()),
            Err(ModelError::NonFiniteValue("light_output_keVee"))
        );
        let mut invalid = example_psd_slice();
        invalid.gamma_sigma_psd = 0.0;
        assert_eq!(
            neutron_psd_gate_accepts(52.5, 0.27, 50.0, invalid),
            Err(ModelError::NonPositiveValue("gamma_sigma_psd"))
        );
    }

    #[test]
    fn prompt_dissociation_maps_d_plus_and_d2_plus_to_distinct_cm_energies() {
        let components = prompt_dissociation_beam_components(1.0, 0.85, 0.15)
            .expect("valid ion-current fractions");
        assert!((components[0].energy_cm_kev - 0.5).abs() < 1.0e-12);
        assert!((components[1].energy_cm_kev - 0.25).abs() < 1.0e-12);
        assert!((components[0].deuteron_flux_fraction - 0.85 / 1.15).abs() < 1.0e-12);
        assert!((components[1].deuteron_flux_fraction - 0.30 / 1.15).abs() < 1.0e-12);
    }

    #[test]
    fn prompt_dissociation_deuteron_flux_weights_sum_to_one() {
        let components = prompt_dissociation_beam_components(5.0, 0.85, 0.15)
            .expect("valid ion-current fractions");
        let sum = components
            .iter()
            .map(|component| component.deuteron_flux_fraction)
            .sum::<f64>();
        assert!((sum - 1.0).abs() < 1.0e-12);
        assert!((components[1].deuteron_flux_fraction - 0.2608695652173913).abs() < 1.0e-12);
    }

    #[test]
    fn prompt_dissociation_rejects_invalid_ion_current_fractions() {
        assert!(prompt_dissociation_beam_components(1.0, 0.85, 0.10).is_err());
        assert!(prompt_dissociation_beam_components(1.0, 1.1, -0.1).is_err());
    }

    #[test]
    fn arbitrary_measured_spectrum_is_weighted_by_deuteron_flux() {
        let spectrum = [
            BeamEnergyComponent {
                energy_cm_kev: 0.25,
                deuteron_flux_fraction: 0.25,
            },
            BeamEnergyComponent {
                energy_cm_kev: 0.50,
                deuteron_flux_fraction: 0.50,
            },
            BeamEnergyComponent {
                energy_cm_kev: 0.75,
                deuteron_flux_fraction: 0.25,
            },
        ];
        let actual = weighted_beam_spectrum_yield(
            &spectrum,
            GamowEnergyKev::new(986.0).expect("Gamow"),
            TargetDensityPerCm3::new(5.0e22).expect("density"),
            1024,
            |_| 50.0,
            |_| 1.0e8,
            |_| 0.5,
        )
        .expect("spectrum yield");
        let expected = spectrum
            .iter()
            .map(|component| {
                component.deuteron_flux_fraction
                    * homogeneous_thick_target_yield(
                        EnergyKev::new(component.energy_cm_kev).expect("energy"),
                        GamowEnergyKev::new(986.0).expect("Gamow"),
                        TargetDensityPerCm3::new(5.0e22).expect("density"),
                        1024,
                        |_| 50.0,
                        |_| 1.0e8,
                        |_| 0.5,
                    )
                    .expect("component yield")
            })
            .sum::<f64>();
        assert!((actual - expected).abs() / expected < 1.0e-12);
    }

    #[test]
    fn arbitrary_spectrum_rejects_unnormalized_weights() {
        let spectrum = [
            BeamEnergyComponent {
                energy_cm_kev: 0.25,
                deuteron_flux_fraction: 0.25,
            },
            BeamEnergyComponent {
                energy_cm_kev: 0.50,
                deuteron_flux_fraction: 0.50,
            },
        ];
        let result = weighted_beam_spectrum_yield(
            &spectrum,
            GamowEnergyKev::new(986.0).expect("Gamow"),
            TargetDensityPerCm3::new(5.0e22).expect("density"),
            128,
            |_| 50.0,
            |_| 1.0e8,
            |_| 0.5,
        );
        assert_eq!(
            result,
            Err(ModelError::InvalidParameter(
                "deuteron_flux_fractions_must_sum_to_one"
            ))
        );
    }

    #[test]
    fn prompt_dissociation_weighted_yield_matches_component_weighting() {
        let components = prompt_dissociation_beam_components(1.0, 0.85, 0.15)
            .expect("valid ion-current fractions");
        let weighted = prompt_dissociation_weighted_yield(
            components,
            GamowEnergyKev::new(986.0).expect("Gamow"),
            TargetDensityPerCm3::new(5.0e22).expect("density"),
            2048,
            |_| 50.0,
            |_| 1.0e8,
            |_| 0.5,
        )
        .expect("weighted mixture");
        let d_plus = homogeneous_thick_target_yield(
            EnergyKev::new(0.5).expect("D+ Ecm"),
            GamowEnergyKev::new(986.0).expect("Gamow"),
            TargetDensityPerCm3::new(5.0e22).expect("density"),
            2048,
            |_| 50.0,
            |_| 1.0e8,
            |_| 0.5,
        )
        .expect("D+ component");
        let d2_fragment = homogeneous_thick_target_yield(
            EnergyKev::new(0.25).expect("D2+ fragment Ecm"),
            GamowEnergyKev::new(986.0).expect("Gamow"),
            TargetDensityPerCm3::new(5.0e22).expect("density"),
            2048,
            |_| 50.0,
            |_| 1.0e8,
            |_| 0.5,
        )
        .expect("D2+ component");
        let expected = (0.85 / 1.15) * d_plus + (0.30 / 1.15) * d2_fragment;
        assert!((weighted - expected).abs() / expected < 1.0e-12);
    }

    #[test]
    fn printed_sigmoid_enhancement_uses_literal_equation_direction() {
        let energy = EnergyKev::new(0.01).expect("low energy");
        let enhancement = sigmoid_heterogeneous_enhancement_printed_equation(
            energy,
            GamowEnergyKev::new(986.0).expect("Gamow"),
            0.8,
            3.0,
            EnergyKev::new(0.6).expect("crossover"),
            0.5,
        )
        .expect("printed enhancement");
        let expected_u = sigmoid_screening_potential_printed_equation(
            energy,
            0.8,
            3.0,
            EnergyKev::new(0.6).expect("crossover"),
            0.5,
        )
        .expect("printed potential");
        let expected = screening_enhancement(
            energy,
            GamowEnergyKev::new(986.0).expect("Gamow"),
            expected_u,
        )
        .expect("closed form");
        assert!((enhancement - expected).abs() / expected < 1.0e-12);
    }

    #[test]
    fn shared_reference_covariance_has_positive_off_diagonal_terms() {
        let covariance = shared_reference_log_covariance(&[0.04, 0.09], 0.01)
            .expect("shared-reference covariance");
        assert!((covariance[0][0] - 0.05).abs() < 1.0e-12);
        assert!((covariance[1][1] - 0.10).abs() < 1.0e-12);
        assert!((covariance[0][1] - 0.01).abs() < 1.0e-12);
        assert!((covariance[1][0] - 0.01).abs() < 1.0e-12);
    }

    #[test]
    fn correlated_chi_square_matches_diagonal_objective_for_diagonal_covariance() {
        let observed = [1.1, 2.2, 4.4];
        let predicted = [1.0, 2.0, 4.0];
        let sigmas = [0.1, 0.2, 0.4];
        let diagonal = vec![
            vec![0.01, 0.0, 0.0],
            vec![0.0, 0.04, 0.0],
            vec![0.0, 0.0, 0.16],
        ];
        let old =
            weighted_log_space_chi_square(&observed, &predicted, &sigmas).expect("diagonal score");
        let new = correlated_weighted_log_space_chi_square(&observed, &predicted, &diagonal)
            .expect("GLS score");
        assert!((old - new).abs() < 1.0e-10);
    }

    #[test]
    fn correlated_chi_square_accepts_shared_reference_covariance() {
        let covariance = shared_reference_log_covariance(&[0.04, 0.09], 0.01).expect("covariance");
        let score = correlated_weighted_log_space_chi_square(&[1.1, 2.2], &[1.0, 2.0], &covariance)
            .expect("correlated score");
        assert!(score.is_finite() && score >= 0.0);
    }

    #[test]
    fn correlated_chi_square_rejects_non_positive_definite_covariance() {
        let covariance = vec![vec![1.0, 2.0], vec![2.0, 1.0]];
        let result =
            correlated_weighted_log_space_chi_square(&[1.1, 2.2], &[1.0, 2.0], &covariance);
        assert_eq!(
            result,
            Err(ModelError::InvalidCovariance(
                "matrix must be positive definite"
            ))
        );
    }

    #[test]
    fn correlated_chi_square_rejects_asymmetric_covariance() {
        let covariance = vec![vec![1.0, 0.2], vec![0.1, 1.0]];
        let result =
            correlated_weighted_log_space_chi_square(&[1.1, 2.2], &[1.0, 2.0], &covariance);
        assert_eq!(
            result,
            Err(ModelError::InvalidCovariance("matrix must be symmetric"))
        );
    }

    #[test]
    fn total_deuteron_rate_counts_two_nuclei_per_d2_plus_ion() {
        let rate =
            total_deuteron_rate_per_second(1.0e-6, 0.85, 0.15).expect("valid species fractions");
        const E_CHARGE: f64 = 1.602_176_634e-19;
        let expected = 1.0e-6 / E_CHARGE * 1.15;
        assert!((rate - expected).abs() / expected < 1.0e-12);
    }

    #[test]
    fn measured_beam_profile_reconciles_current_and_conserves_deuteron_flux() {
        let cells = [
            BeamProfileCell {
                x_cm: -0.5,
                y_cm: 0.0,
                area_cm2: 0.25,
                ion_current_density_a_cm2: 2.0e-6,
            },
            BeamProfileCell {
                x_cm: 0.0,
                y_cm: 0.0,
                area_cm2: 0.50,
                ion_current_density_a_cm2: 0.5e-6,
            },
            BeamProfileCell {
                x_cm: 0.5,
                y_cm: 0.0,
                area_cm2: 0.25,
                ion_current_density_a_cm2: 1.0e-6,
            },
        ];
        let profile = reconcile_measured_beam_profile(&cells, 1.0e-6, 0.85, 0.15, 1.0e-12)
            .expect("matching spatial beam profile");
        assert!((profile.integrated_map_current_a - 1.0e-6).abs() < 1.0e-18);
        let fraction_sum = profile.deuteron_flux_fraction_by_cell.iter().sum::<f64>();
        assert!((fraction_sum - 1.0).abs() < 1.0e-12);
        let rate_sum = profile.deuteron_rate_by_cell_s.iter().sum::<f64>();
        let expected =
            total_deuteron_rate_per_second(1.0e-6, 0.85, 0.15).expect("total deuteron rate");
        assert!((rate_sum - expected).abs() / expected < 1.0e-12);
    }

    #[test]
    fn measured_beam_profile_rejects_faraday_current_mismatch() {
        let cells = [BeamProfileCell {
            x_cm: 0.0,
            y_cm: 0.0,
            area_cm2: 1.0,
            ion_current_density_a_cm2: 1.0e-6,
        }];
        let result = reconcile_measured_beam_profile(&cells, 1.2e-6, 1.0, 0.0, 0.01);
        assert_eq!(
            result,
            Err(ModelError::InvalidParameter(
                "beam_map_current_does_not_reconcile_with_faraday_cup"
            ))
        );
    }

    #[test]
    fn measured_beam_profile_rejects_invalid_cell_geometry_or_density() {
        let bad_area = [BeamProfileCell {
            x_cm: 0.0,
            y_cm: 0.0,
            area_cm2: 0.0,
            ion_current_density_a_cm2: 1.0e-6,
        }];
        assert!(reconcile_measured_beam_profile(&bad_area, 1.0e-6, 1.0, 0.0, 0.01).is_err());
        let bad_density = [BeamProfileCell {
            x_cm: 0.0,
            y_cm: 0.0,
            area_cm2: 1.0,
            ion_current_density_a_cm2: -1.0e-6,
        }];
        assert!(reconcile_measured_beam_profile(&bad_density, 1.0e-6, 1.0, 0.0, 0.01).is_err());
    }

    #[test]
    fn gaussian_beam_footprint_is_current_normalized() {
        let footprint = GaussianBeamFootprint::new(0.25).expect("sigma");
        let rate = total_deuteron_rate_per_second(1.0e-6, 0.85, 0.15).expect("deuteron rate");
        let peak = footprint
            .deuteron_flux_density_cm2_s(rate, 0.0)
            .expect("peak density");
        let one_sigma = footprint
            .deuteron_flux_density_cm2_s(rate, 0.25)
            .expect("one-sigma density");
        assert!((one_sigma / peak - (-0.5_f64).exp()).abs() < 1.0e-12);
        let integrated_rate = peak * 2.0 * std::f64::consts::PI * 0.25_f64.powi(2);
        assert!((integrated_rate - rate).abs() / rate < 1.0e-12);
    }

    #[test]
    fn bare_reference_is_finite_and_positive() {
        let value = synthetic_yield(0.0, 4096).expect("valid synthetic reference");
        assert!(value.is_finite());
        assert!(value > 0.0);
    }

    #[test]
    fn increasing_screening_increases_reference_yield() {
        let bare = synthetic_yield(0.0, 4096).expect("bare");
        let screened = synthetic_yield(0.5, 4096).expect("screened");
        assert!(screened > bare);
    }

    #[test]
    fn numerical_integral_converges_under_step_refinement() {
        let coarse = synthetic_yield(0.25, 1024).expect("coarse");
        let fine = synthetic_yield(0.25, 2048).expect("fine");
        let relative_error = (coarse - fine).abs() / fine;
        assert!(relative_error < 1.0e-6, "relative error={relative_error:e}");
    }

    #[test]
    fn relative_yield_is_one_at_reference_energy() {
        let ratio = relative_thick_target_yield(
            EnergyKev::new(6.5).expect("energy"),
            EnergyKev::new(6.5).expect("reference"),
            GamowEnergyKev::new(986.0).expect("Gamow energy"),
            TargetDensityPerCm3::new(5.0e22).expect("density"),
            2048,
            |_| 50.0,
            |_| 1.0e8,
            |_| 0.25,
        )
        .expect("relative yield");
        assert!((ratio - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn screening_enhancement_has_bare_limit_of_one() {
        let enhancement = screening_enhancement(
            EnergyKev::new(2.0).expect("energy"),
            GamowEnergyKev::new(986.0).expect("Gamow energy"),
            0.0,
        )
        .expect("bare enhancement");
        assert_eq!(enhancement, 1.0);
    }

    #[test]
    fn screening_enhancement_increases_for_positive_potential() {
        let bare = screening_enhancement(
            EnergyKev::new(1.0).expect("energy"),
            GamowEnergyKev::new(986.0).expect("Gamow energy"),
            0.0,
        )
        .expect("bare");
        let screened = screening_enhancement(
            EnergyKev::new(1.0).expect("energy"),
            GamowEnergyKev::new(986.0).expect("Gamow energy"),
            1.0,
        )
        .expect("screened");
        assert!(screened > bare);
    }

    #[test]
    fn two_environment_mixture_respects_zero_screening_limit() {
        let value = two_environment_mixture_enhancement(
            EnergyKev::new(0.5).expect("energy"),
            GamowEnergyKev::new(986.0).expect("Gamow"),
            0.3,
            0.0,
            0.0,
        )
        .expect("mixture");
        assert!((value - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn two_environment_mixture_reduces_to_single_environment_at_unit_weight() {
        let energy = EnergyKev::new(0.5).expect("energy");
        let gamow = GamowEnergyKev::new(986.0).expect("Gamow");
        let mixture =
            two_environment_mixture_enhancement(energy, gamow, 1.0, 2.5, 0.7).expect("mixture");
        let surface = screening_enhancement(energy, gamow, 2.5).expect("surface");
        assert!((mixture - surface).abs() < 1.0e-12);
    }

    #[test]
    fn printed_equation_and_prose_direction_have_distinct_asymptotes() {
        let bulk = 0.8;
        let surface = 3.0;
        let e0 = EnergyKev::new(0.6).expect("crossover");
        let low = EnergyKev::new(0.01).expect("low energy");
        let high = EnergyKev::new(6.0).expect("high energy");

        let printed_low = sigmoid_screening_potential_printed_equation(low, bulk, surface, e0, 0.5)
            .expect("printed low");
        let printed_high =
            sigmoid_screening_potential_printed_equation(high, bulk, surface, e0, 0.5)
                .expect("printed high");
        let prose_low = sigmoid_screening_potential_prose_direction(low, bulk, surface, e0, 0.5)
            .expect("prose low");
        let prose_high = sigmoid_screening_potential_prose_direction(high, bulk, surface, e0, 0.5)
            .expect("prose high");

        assert!(printed_low < printed_high);
        assert!(prose_low > prose_high);
        assert!((prose_low - surface).abs() < 0.1);
        assert!((prose_high - bulk).abs() < 0.1);
    }

    #[test]
    fn sigmoid_crosses_midpoint_at_crossover_energy() {
        let value = sigmoid_screening_potential_prose_direction(
            EnergyKev::new(0.6).expect("energy"),
            0.8,
            3.0,
            EnergyKev::new(0.6).expect("crossover"),
            0.5,
        )
        .expect("midpoint");
        assert!((value - 1.9).abs() < 1.0e-12);
    }

    #[test]
    fn weighted_log_space_objective_is_zero_for_exact_prediction() {
        let value =
            weighted_log_space_chi_square(&[1.0, 2.0, 10.0], &[1.0, 2.0, 10.0], &[0.1, 0.1, 0.2])
                .expect("exact fit");
        assert!(value.abs() < 1.0e-24);
    }

    #[test]
    fn rejects_bad_log_space_input_lengths() {
        assert_eq!(
            weighted_log_space_chi_square(&[1.0], &[1.0, 2.0], &[0.1]),
            Err(ModelError::MismatchedDataLengths)
        );
    }

    #[test]
    fn rejects_zero_stopping_power() {
        let result = homogeneous_thick_target_yield(
            EnergyKev::new(6.5).expect("energy"),
            GamowEnergyKev::new(986.0).expect("Gamow energy"),
            TargetDensityPerCm3::new(5.0e22).expect("density"),
            128,
            |_| 50.0,
            |_| 0.0,
            |_| 0.0,
        );
        assert_eq!(
            result,
            Err(ModelError::NonPositiveValue("stopping_kev_cm_per_cm"))
        );
    }

    #[test]
    fn rejects_odd_or_insufficient_integration_steps() {
        assert_eq!(
            synthetic_yield(0.0, 1),
            Err(ModelError::InvalidIntegrationSteps)
        );
        assert_eq!(
            synthetic_yield(0.0, 3),
            Err(ModelError::InvalidIntegrationSteps)
        );
    }

    #[test]
    fn rejects_negative_screening() {
        let result = homogeneous_thick_target_yield(
            EnergyKev::new(6.5).expect("energy"),
            GamowEnergyKev::new(986.0).expect("Gamow energy"),
            TargetDensityPerCm3::new(5.0e22).expect("density"),
            128,
            |_| 50.0,
            |_| 1.0e8,
            |_| -0.1,
        );
        assert_eq!(result, Err(ModelError::InvalidParameter("screening_kev")));
    }
}
