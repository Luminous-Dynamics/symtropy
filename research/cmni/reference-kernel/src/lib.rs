#![forbid(unsafe_code)]
//! Research-only numerical reference for homogeneous and two-environment
//! screened D-D thick-target yield models.
//!
//! This is not a validated nuclear-data library, a heterogeneous target
//! transport model, or an experimental reproduction. Source-specific S-factor,
//! stopping-power, beam-spread and uncertainty inputs must be supplied by the
//! caller and versioned in an external evidence manifest.

use std::fmt;

const BARN_TO_CM2: f64 = 1.0e-24;

#[derive(Debug, Clone, PartialEq)]
pub enum ModelError {
    InvalidParameter(&'static str),
    NonFiniteValue(&'static str),
    NonPositiveValue(&'static str),
    InvalidIntegrationSteps,
    MismatchedDataLengths,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter(name) => write!(f, "invalid parameter: {name}"),
            Self::NonFiniteValue(name) => write!(f, "non-finite value: {name}"),
            Self::NonPositiveValue(name) => write!(f, "non-positive value: {name}"),
            Self::InvalidIntegrationSteps => write!(f, "Simpson integration requires an even step count >= 2"),
            Self::MismatchedDataLengths => write!(f, "observation, prediction, and uncertainty arrays must have equal non-zero length"),
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
    s_factor_keV_barn: FS,
) -> Result<f64, ModelError>
where
    FS: Fn(f64) -> f64,
{
    let energy = energy_cm.get();
    let screening = checked_nonnegative(screening_kev, "screening_keV")?;
    let s_factor = checked_positive(s_factor_keV_barn(energy), "S_factor_keV_barn")?;
    let shifted_energy = energy + screening;
    let cross_section = s_factor
        / (energy * shifted_energy).sqrt()
        * (-(gamow_energy_cm.0 / shifted_energy).sqrt()).exp();

    if !cross_section.is_finite() || cross_section < 0.0 {
        return Err(ModelError::InvalidParameter("cross_section must be finite and >= 0"));
    }
    Ok(cross_section)
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
/// - stopping_keV_cm_per_cm is **-dEcm/dx** in keV_cm/cm (not automatically
///   the projectile laboratory-frame stopping power);
/// - screening_keV is a non-negative effective potential in keV.
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
    s_factor_keV_barn: FS,
    stopping_keV_cm_per_cm: FStop,
    screening_keV: FU,
) -> Result<f64, ModelError>
where
    FS: Fn(f64) -> f64,
    FStop: Fn(f64) -> f64,
    FU: Fn(f64) -> f64,
{
    if steps < 2 || steps % 2 != 0 {
        return Err(ModelError::InvalidIntegrationSteps);
    }

    // Ecm = t^2 regularizes the integrable Ecm^-1/2 endpoint for Ue > 0.
    let t_max = incident_energy_cm.get().sqrt();
    let h = t_max / steps as f64;
    let mut weighted_sum = 0.0;

    for i in 0..=steps {
        let t = i as f64 * h;
        let energy_cm = t * t;
        let s_factor = checked_positive(s_factor_keV_barn(energy_cm), "S_factor_keV_barn")?;
        let stopping = checked_positive(
            stopping_keV_cm_per_cm(energy_cm),
            "stopping_keV_cm_per_cm",
        )?;
        let screening = checked_nonnegative(screening_keV(energy_cm), "screening_keV")?;

        // This is 2*t*sigma(t^2,Ue), the transformed cross-section integrand.
        let transformed = if t == 0.0 {
            if screening > 0.0 {
                2.0 * s_factor / screening.sqrt()
                    * (-(gamow_energy_cm.0 / screening).sqrt()).exp()
            } else {
                // For Ue=0 the Gamow factor suppresses the endpoint to zero.
                0.0
            }
        } else {
            let shifted_energy = energy_cm + screening;
            let cross_section_barn = s_factor
                / (energy_cm * shifted_energy).sqrt()
                * (-(gamow_energy_cm.0 / shifted_energy).sqrt()).exp();
            2.0 * t * cross_section_barn
        };

        let integrand = target_density.0 * BARN_TO_CM2 * transformed / stopping;
        if !integrand.is_finite() || integrand < 0.0 {
            return Err(ModelError::InvalidParameter("yield integrand must be finite and >= 0"));
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
        return Err(ModelError::InvalidParameter("yield result must be finite and >= 0"));
    }
    Ok(result)
}

/// Relative homogeneous thick-target yield R(E) = Y(E) / Y(E_ref).
///
/// S-factor and stopping closures use Ecm in keV; stopping must be -dEcm/dx
/// in keV_cm/cm. This matches the relative-yield normalization used in the
/// 2026 paper, with the reference energy explicitly supplied by the caller.
pub fn relative_thick_target_yield<FS, FStop, FU>(
    energy_cm: EnergyKev,
    reference_energy_cm: EnergyKev,
    gamow_energy_cm: GamowEnergyKev,
    target_density: TargetDensityPerCm3,
    steps: usize,
    s_factor_keV_barn: FS,
    stopping_keV_cm_per_cm: FStop,
    screening_keV: FU,
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
        &s_factor_keV_barn,
        &stopping_keV_cm_per_cm,
        &screening_keV,
    )?;
    let reference = homogeneous_thick_target_yield(
        reference_energy_cm,
        gamow_energy_cm,
        target_density,
        steps,
        &s_factor_keV_barn,
        &stopping_keV_cm_per_cm,
        &screening_keV,
    )?;
    if reference <= 0.0 {
        return Err(ModelError::NonPositiveValue("reference_yield"));
    }
    Ok(observed / reference)
}

/// Homogeneous ALR enhancement E_scr(E; Ue) = Y_scr(E) / Y_bare(E),
/// using the closed-form expression printed as Eq. (12) in the 2026 paper.
pub fn screening_enhancement(
    energy_cm: EnergyKev,
    gamow_energy_cm: GamowEnergyKev,
    screening_kev: f64,
) -> Result<f64, ModelError> {
    let energy = energy_cm.get();
    let screening = checked_nonnegative(screening_kev, "screening_keV")?;
    if screening == 0.0 {
        return Ok(1.0);
    }

    let shifted_energy = energy + screening;
    let prefactor = (energy / shifted_energy).sqrt();
    let exponent = (gamow_energy_cm.0 / energy).sqrt()
        - (gamow_energy_cm.0 / shifted_energy).sqrt();
    let result = prefactor * exponent.exp();

    if !result.is_finite() || result <= 0.0 {
        return Err(ModelError::InvalidParameter("screening enhancement must be finite and > 0"));
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
        return Err(ModelError::InvalidParameter("surface_amplitude must be in [0, 1]"));
    }
    let surface = screening_enhancement(energy_cm, gamow_energy_cm, surface_screening_kev)?;
    let bulk = screening_enhancement(energy_cm, gamow_energy_cm, bulk_screening_kev)?;
    let result = surface_amplitude * surface + (1.0 - surface_amplitude) * bulk;

    if !result.is_finite() || result <= 0.0 {
        return Err(ModelError::InvalidParameter("mixture enhancement must be finite and > 0"));
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
    let bulk = checked_nonnegative(bulk_screening_kev, "bulk_screening_keV")?;
    let surface = checked_nonnegative(surface_screening_kev, "surface_screening_keV")?;
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
    let bulk = checked_nonnegative(bulk_screening_kev, "bulk_screening_keV")?;
    let surface = checked_nonnegative(surface_screening_kev, "surface_screening_keV")?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_yield(screening: f64, steps: usize) -> Result<f64, ModelError> {
        homogeneous_thick_target_yield(
            EnergyKev::new(6.5)?,
            GamowEnergyKev::new(986.0)?,
            TargetDensityPerCm3::new(5.0e22)?,
            steps,
            |_| 50.0,        // synthetic constant S factor, keV·barn
            |_| 1.0e8,       // synthetic stopping, keV_cm/cm
            |_| screening,   // synthetic effective screening potential, keV
        )
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
        let mixture = two_environment_mixture_enhancement(energy, gamow, 1.0, 2.5, 0.7)
            .expect("mixture");
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
        let printed_high = sigmoid_screening_potential_printed_equation(high, bulk, surface, e0, 0.5)
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
        let value = weighted_log_space_chi_square(
            &[1.0, 2.0, 10.0],
            &[1.0, 2.0, 10.0],
            &[0.1, 0.1, 0.2],
        )
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
        assert_eq!(result, Err(ModelError::NonPositiveValue("stopping_keV_cm_per_cm")));
    }

    #[test]
    fn rejects_odd_or_insufficient_integration_steps() {
        assert_eq!(synthetic_yield(0.0, 1), Err(ModelError::InvalidIntegrationSteps));
        assert_eq!(synthetic_yield(0.0, 3), Err(ModelError::InvalidIntegrationSteps));
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
        assert_eq!(result, Err(ModelError::InvalidParameter("screening_keV")));
    }
}
