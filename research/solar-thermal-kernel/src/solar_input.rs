// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Direct-beam solar input adapter.
//!
//! Converts a declared DNI sample into interval-integrated energy at the
//! collector aperture plane. It deliberately does not estimate DNI from GHI,
//! model sun position, include diffuse/albedo radiation, or model receiver loss.

use super::{check, finite, nonnegative, positive, EnergyJ, Fraction, ModelError, Seconds};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IrradianceWPerM2(f64);

impl IrradianceWPerM2 {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        nonnegative(value, "direct_normal_irradiance_w_per_m2").map(Self)
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

/// Interval-integrated direct-normal beam irradiation, in Wh/m².
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectNormalIrradiationWhPerM2(f64);

impl DirectNormalIrradiationWhPerM2 {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        nonnegative(value, "direct_normal_irradiation_wh_per_m2").map(Self)
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeanDniReport {
    pub source_interval_irradiation: DirectNormalIrradiationWhPerM2,
    pub duration: Seconds,
    /// Interval-average DNI inferred from integrated irradiation, W/m².
    pub interval_mean_irradiance: IrradianceWPerM2,
}

/// Convert interval-integrated BNI in Wh/m² to interval-average DNI in W/m².
///
/// Formula: DNI_mean = BNI_interval * 3600 / duration_s. This preserves the
/// interval energy when multiplied back by duration. It does not reconstruct
/// within-interval irradiance variation; applying a single geometry factor
/// later is a piecewise-constant/effective-factor approximation.
pub fn mean_dni_from_interval_irradiation(
    irradiation: DirectNormalIrradiationWhPerM2,
    duration: Seconds,
) -> Result<MeanDniReport, ModelError> {
    let mean_w_per_m2 = irradiation.value() * 3_600.0 / duration.value();
    Ok(MeanDniReport {
        source_interval_irradiation: irradiation,
        duration,
        interval_mean_irradiance: IrradianceWPerM2::new(mean_w_per_m2)?,
    })
}

/// Independently recompute the interval-average irradiance conversion.
pub fn verify_mean_dni_report(
    irradiation: DirectNormalIrradiationWhPerM2,
    duration: Seconds,
    claimed: MeanDniReport,
) -> Result<(), ModelError> {
    check(
        "solar_input.mean_dni.source_irradiation",
        claimed.source_interval_irradiation.value(),
        irradiation.value(),
    )?;
    check(
        "solar_input.mean_dni.duration",
        claimed.duration.value(),
        duration.value(),
    )?;
    let expected = finite(irradiation.value(), "direct_normal_irradiation_wh_per_m2")?
        * 3_600.0 / finite(duration.value(), "duration_s")?;
    check(
        "solar_input.mean_dni.irradiance",
        claimed.interval_mean_irradiance.value(),
        expected,
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IncidentPowerW(f64);

impl IncidentPowerW {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        nonnegative(value, "incident_power_w").map(Self)
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApertureAreaM2(f64);

impl ApertureAreaM2 {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        positive(value, "collector_aperture_area_m2").map(Self)
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectBeamIntervalInput {
    /// Direct-normal irradiance, not global-horizontal irradiance.
    pub direct_normal_irradiance: IrradianceWPerM2,
    /// Collector aperture area in square metres.
    pub aperture_area: ApertureAreaM2,
    /// Cosine of the beam incidence angle at the collector aperture, in [0, 1].
    pub incidence_cosine: Fraction,
    /// Unshaded fraction of the projected aperture, in [0, 1].
    pub unshaded_fraction: Fraction,
    pub duration: Seconds,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectBeamIntervalReport {
    /// DNI x aperture area x incidence cosine x unshaded fraction.
    pub incident_power_at_aperture: IncidentPowerW,
    /// Incident direct-beam energy integrated over this interval.
    pub incident_energy: EnergyJ,
}

/// Convert direct-normal irradiance into incident aperture energy.
///
/// Formula: E = DNI * A_aperture * cos(theta_incidence) * f_unshaded * dt.
/// The output is incident radiation, not absorbed or useful thermal energy.
pub fn integrate_direct_beam(
    input: DirectBeamIntervalInput,
) -> Result<DirectBeamIntervalReport, ModelError> {
    let projected_power = input.direct_normal_irradiance.value()
        * input.aperture_area.value()
        * input.incidence_cosine.value()
        * input.unshaded_fraction.value();
    let energy = projected_power * input.duration.value();
    Ok(DirectBeamIntervalReport {
        incident_power_at_aperture: IncidentPowerW::new(projected_power)?,
        incident_energy: EnergyJ::new(energy)?,
    })
}

/// Independent recomputation check; does not call integrate_direct_beam.
pub fn verify_direct_beam_interval(
    input: DirectBeamIntervalInput,
    claimed: DirectBeamIntervalReport,
) -> Result<(), ModelError> {
    let dni = finite(input.direct_normal_irradiance.value(), "direct_normal_irradiance_w_per_m2")?;
    let area = finite(input.aperture_area.value(), "collector_aperture_area_m2")?;
    let incidence = finite(input.incidence_cosine.value(), "incidence_cosine")?;
    let unshaded = finite(input.unshaded_fraction.value(), "unshaded_fraction")?;
    let duration = finite(input.duration.value(), "duration_s")?;

    let expected_power = dni * area * incidence * unshaded;
    let expected_energy = expected_power * duration;
    check(
        "solar_input.incident_power",
        claimed.incident_power_at_aperture.value(),
        expected_power,
    )?;
    check("solar_input.incident_energy", claimed.incident_energy.value(), expected_energy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(duration_s: f64) -> DirectBeamIntervalInput {
        DirectBeamIntervalInput {
            direct_normal_irradiance: IrradianceWPerM2::new(800.0).unwrap(),
            aperture_area: ApertureAreaM2::new(10.0).unwrap(),
            incidence_cosine: Fraction::new(0.5).unwrap(),
            unshaded_fraction: Fraction::new(0.8).unwrap(),
            duration: Seconds::new(duration_s).unwrap(),
        }
    }

    #[test]
    fn aperture_power_and_interval_energy_have_hand_checkable_values() {
        let report = integrate_direct_beam(input(3_600.0)).unwrap();
        assert_eq!(report.incident_power_at_aperture.value(), 3_200.0);
        assert_eq!(report.incident_energy.value(), 11_520_000.0);
        verify_direct_beam_interval(input(3_600.0), report).unwrap();
    }

    #[test]
    fn energy_scales_linearly_with_interval_duration() {
        let short = integrate_direct_beam(input(900.0)).unwrap();
        let long = integrate_direct_beam(input(1_800.0)).unwrap();
        assert_eq!(long.incident_energy.value(), 2.0 * short.incident_energy.value());
        assert_eq!(long.incident_power_at_aperture, short.incident_power_at_aperture);
    }

    #[test]
    fn zero_projection_or_complete_shading_yields_zero_incident_power() {
        let mut no_projection = input(3_600.0);
        no_projection.incidence_cosine = Fraction::new(0.0).unwrap();
        let report = integrate_direct_beam(no_projection).unwrap();
        assert_eq!(report.incident_power_at_aperture.value(), 0.0);
        assert_eq!(report.incident_energy.value(), 0.0);

        let mut shaded = input(3_600.0);
        shaded.unshaded_fraction = Fraction::new(0.0).unwrap();
        let report = integrate_direct_beam(shaded).unwrap();
        assert_eq!(report.incident_power_at_aperture.value(), 0.0);
        assert_eq!(report.incident_energy.value(), 0.0);
    }

    #[test]
    fn verifier_rejects_tampered_incident_energy() {
        let original_input = input(3_600.0);
        let report = integrate_direct_beam(original_input).unwrap();
        let tampered = DirectBeamIntervalReport {
            incident_energy: EnergyJ::new(1.0).unwrap(),
            ..report
        };
        assert!(matches!(
            verify_direct_beam_interval(original_input, tampered),
            Err(ModelError::ReportMismatch("solar_input.incident_energy"))
        ));
    }

    #[test]
    fn interval_irradiation_converts_to_average_dni_without_energy_drift() {
        // 100 Wh/m² over 15 minutes corresponds to 400 W/m² interval-average DNI.
        let irradiation = DirectNormalIrradiationWhPerM2::new(100.0).unwrap();
        let duration = Seconds::new(900.0).unwrap();
        let report = mean_dni_from_interval_irradiation(irradiation, duration).unwrap();
        assert_eq!(report.interval_mean_irradiance.value(), 400.0);
        assert_eq!(report.interval_mean_irradiance.value() * duration.value(),
            irradiation.value() * 3_600.0);
        verify_mean_dni_report(irradiation, duration, report).unwrap();
    }

    #[test]
    fn irradiation_conversion_scales_with_interval_duration() {
        let irradiation = DirectNormalIrradiationWhPerM2::new(50.0).unwrap();
        let short_duration = Seconds::new(900.0).unwrap();
        let long_duration = Seconds::new(1_800.0).unwrap();
        let short = mean_dni_from_interval_irradiation(irradiation, short_duration).unwrap();
        let long = mean_dni_from_interval_irradiation(irradiation, long_duration).unwrap();
        assert_eq!(short.interval_mean_irradiance.value(), 2.0 * long.interval_mean_irradiance.value());
        assert_eq!(short.interval_mean_irradiance.value() * short_duration.value(),
            long.interval_mean_irradiance.value() * long_duration.value());
    }

    #[test]
    fn mean_dni_verifier_rejects_tampered_conversion() {
        let irradiation = DirectNormalIrradiationWhPerM2::new(100.0).unwrap();
        let duration = Seconds::new(900.0).unwrap();
        let report = mean_dni_from_interval_irradiation(irradiation, duration).unwrap();
        let tampered = MeanDniReport {
            interval_mean_irradiance: IrradianceWPerM2::new(401.0).unwrap(),
            ..report
        };
        assert!(matches!(
            verify_mean_dni_report(irradiation, duration, tampered),
            Err(ModelError::ReportMismatch("solar_input.mean_dni.irradiance"))
        ));
    }

    #[test]
    fn direct_normal_irradiation_rejects_invalid_values() {
        assert!(DirectNormalIrradiationWhPerM2::new(-1.0).is_err());
        assert!(DirectNormalIrradiationWhPerM2::new(f64::INFINITY).is_err());
    }

    #[test]
    fn input_types_reject_invalid_physical_values() {
        assert!(IrradianceWPerM2::new(-1.0).is_err());
        assert!(IrradianceWPerM2::new(f64::NAN).is_err());
        assert!(ApertureAreaM2::new(0.0).is_err());
        assert!(ApertureAreaM2::new(f64::INFINITY).is_err());
    }
}
