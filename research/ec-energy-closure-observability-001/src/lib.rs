// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Reference semantics for the observation-side ecosystem energy-balance
//! closure benchmark. This crate is deliberately not a physical energy ledger.
//!
//! It evaluates a published available-energy component ladder, source-quality
//! dispositions for turbulent fluxes, reduced-major-axis diagnostics, and
//! duration-weighted integrated energy-balance ratios. Missing terms and
//! unresolved residuals remain explicit.

use std::collections::BTreeSet;

/// Measurement state for one source-specific energy term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationStatus {
    Measured,
    Estimated,
    NotApplicable,
    Missing,
    Invalid,
    OutOfDomain,
    Unqualified,
}

/// One energy term normalized to W m^-2 with the declared positive-sink
/// convention used by the available-energy ladder.
///
/// NotApplicable contributes zero only when value_w_m2 is None.
/// Missing, Invalid, OutOfDomain and Unqualified never contribute zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyTerm {
    pub status: ObservationStatus,
    pub value_w_m2: Option<f64>,
}

impl EnergyTerm {
    pub const fn measured(value_w_m2: f64) -> Self {
        Self {
            status: ObservationStatus::Measured,
            value_w_m2: Some(value_w_m2),
        }
    }

    pub const fn estimated(value_w_m2: f64) -> Self {
        Self {
            status: ObservationStatus::Estimated,
            value_w_m2: Some(value_w_m2),
        }
    }

    pub const fn not_applicable() -> Self {
        Self {
            status: ObservationStatus::NotApplicable,
            value_w_m2: None,
        }
    }

    pub const fn missing() -> Self {
        Self {
            status: ObservationStatus::Missing,
            value_w_m2: None,
        }
    }
}

/// Raw unit declared by a source adapter before normalization to W m^-2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyMeasurementUnit {
    WattPerSquareMetre,
    JoulePerSquareMetre,
    Unsupported,
}

/// Whether the source sign matches the term-specific reference direction
/// documented by AvailableEnergyTerms or must be inverted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergySourcePolarity {
    MatchesReferenceDirection,
    InvertedFromReferenceDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyNormalizationError {
    NonFiniteValue,
    UnsupportedUnit,
    MissingDuration,
    ZeroDuration,
    NonFiniteResult,
}

/// Normalize a source energy measurement to W m^-2.
///
/// The caller must map the source's sign convention to the term-specific
/// reference direction before calling this function. Interval energy density
/// (J m^-2) is divided by the exact positive averaging duration in seconds.
/// Flux density (W m^-2) is passed through unchanged. No implicit sign or unit
/// guesses are made.
pub fn normalize_energy_measurement(
    value: f64,
    unit: EnergyMeasurementUnit,
    duration_seconds: Option<u64>,
    polarity: EnergySourcePolarity,
) -> Result<f64, EnergyNormalizationError> {
    if !value.is_finite() {
        return Err(EnergyNormalizationError::NonFiniteValue);
    }
    let normalized = match unit {
        EnergyMeasurementUnit::WattPerSquareMetre => value,
        EnergyMeasurementUnit::JoulePerSquareMetre => {
            let duration = duration_seconds.ok_or(EnergyNormalizationError::MissingDuration)?;
            if duration == 0 {
                return Err(EnergyNormalizationError::ZeroDuration);
            }
            value / duration as f64
        }
        EnergyMeasurementUnit::Unsupported => {
            return Err(EnergyNormalizationError::UnsupportedUnit);
        }
    };
    let normalized = match polarity {
        EnergySourcePolarity::MatchesReferenceDirection => normalized,
        EnergySourcePolarity::InvertedFromReferenceDirection => -normalized,
    };
    if !normalized.is_finite() {
        return Err(EnergyNormalizationError::NonFiniteResult);
    }
    Ok(normalized)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermId {
    NetRadiation,
    SoilHeatFlux,
    SoilHeatStorage,
    AirSensibleStorage,
    AirLatentStorage,
    PhotosyntheticEnergy,
    BiomassHeatStorage,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvailableEnergyTerms {
    /// NETRAD: positive into the declared ecosystem control volume.
    pub net_radiation: EnergyTerm,
    /// G: soil heat flux, normalized to positive energy leaving the available
    /// surface-energy pool and entering the soil.
    pub soil_heat_flux: EnergyTerm,
    /// SG: soil heat-storage change, positive into soil storage.
    pub soil_heat_storage: EnergyTerm,
    /// SH: air-column sensible heat-storage change.
    pub air_sensible_storage: EnergyTerm,
    /// SLE: air-column latent heat-storage change.
    pub air_latent_storage: EnergyTerm,
    /// Spho: biochemical energy absorbed during photosynthesis.
    pub photosynthetic_energy: EnergyTerm,
    /// Sbio: aboveground biomass heat-storage change.
    pub biomass_heat_storage: EnergyTerm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AvailableEnergyFormulation {
    /// NETRAD only; conceptual baseline, not complete available energy.
    Ae1,
    Ae2,
    Ae3,
    Ae4,
    Ae5,
    Ae6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludedTermDisposition {
    Measured,
    Estimated,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncludedEnergyTerm {
    pub term_id: TermId,
    pub disposition: IncludedTermDisposition,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AvailableEnergyResult {
    pub formulation: AvailableEnergyFormulation,
    pub value_w_m2: f64,
    /// Term-level evidence is returned with the numeric result; downstream code
    /// cannot silently forget which components were estimated or not applicable.
    pub included_terms: Vec<IncludedEnergyTerm>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnergyTermError {
    Missing(TermId),
    InvalidStatus(TermId, ObservationStatus),
    ValueMissing(TermId),
    ValueUnexpected(TermId),
    NonFinite(TermId),
    NonFiniteResult,
}

fn term_value(
    term: EnergyTerm,
    id: TermId,
) -> Result<(f64, IncludedEnergyTerm), EnergyTermError> {
    match term.status {
        ObservationStatus::Measured | ObservationStatus::Estimated => {
            let value = term.value_w_m2.ok_or(EnergyTermError::ValueMissing(id))?;
            if !value.is_finite() {
                return Err(EnergyTermError::NonFinite(id));
            }
            let disposition = if term.status == ObservationStatus::Measured {
                IncludedTermDisposition::Measured
            } else {
                IncludedTermDisposition::Estimated
            };
            Ok((
                value,
                IncludedEnergyTerm {
                    term_id: id,
                    disposition,
                },
            ))
        }
        ObservationStatus::NotApplicable => {
            if term.value_w_m2.is_some() {
                return Err(EnergyTermError::ValueUnexpected(id));
            }
            Ok((
                0.0,
                IncludedEnergyTerm {
                    term_id: id,
                    disposition: IncludedTermDisposition::NotApplicable,
                },
            ))
        }
        status @ (ObservationStatus::Missing
        | ObservationStatus::Invalid
        | ObservationStatus::OutOfDomain
        | ObservationStatus::Unqualified) => {
            if status == ObservationStatus::Missing {
                Err(EnergyTermError::Missing(id))
            } else {
                Err(EnergyTermError::InvalidStatus(id, status))
            }
        }
    }
}

/// Evaluate one of the published progressive available-energy formulations.
///
/// Every subtractive term is expressed using the positive-sink convention.
/// Negative signed values are retained: a measured storage release can
/// increase available energy. There is no clamping or residual allocation.
/// The result also records the evidence disposition of every included term.
pub fn available_energy(
    terms: AvailableEnergyTerms,
    formulation: AvailableEnergyFormulation,
) -> Result<AvailableEnergyResult, EnergyTermError> {
    let (net, net_evidence) = term_value(terms.net_radiation, TermId::NetRadiation)?;
    let mut result = net;
    let mut included_terms = vec![net_evidence];

    if formulation >= AvailableEnergyFormulation::Ae2 {
        let (value, evidence) = term_value(terms.soil_heat_flux, TermId::SoilHeatFlux)?;
        result -= value;
        included_terms.push(evidence);
    }
    if formulation >= AvailableEnergyFormulation::Ae3 {
        let (value, evidence) = term_value(terms.soil_heat_storage, TermId::SoilHeatStorage)?;
        result -= value;
        included_terms.push(evidence);
    }
    if formulation >= AvailableEnergyFormulation::Ae4 {
        let (value, evidence) =
            term_value(terms.air_sensible_storage, TermId::AirSensibleStorage)?;
        result -= value;
        included_terms.push(evidence);
        let (value, evidence) = term_value(terms.air_latent_storage, TermId::AirLatentStorage)?;
        result -= value;
        included_terms.push(evidence);
    }
    if formulation >= AvailableEnergyFormulation::Ae5 {
        let (value, evidence) =
            term_value(terms.photosynthetic_energy, TermId::PhotosyntheticEnergy)?;
        result -= value;
        included_terms.push(evidence);
    }
    if formulation >= AvailableEnergyFormulation::Ae6 {
        let (value, evidence) =
            term_value(terms.biomass_heat_storage, TermId::BiomassHeatStorage)?;
        result -= value;
        included_terms.push(evidence);
    }

    if !result.is_finite() {
        return Err(EnergyTermError::NonFiniteResult);
    }

    Ok(AvailableEnergyResult {
        formulation,
        value_w_m2: result,
        included_terms,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurbulentQualityPolicy {
    Unfiltered,
    FullQualityControl,
    FrictionVelocityFiltered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurbulentExclusion {
    FailedFullQualityControl,
    BelowFrictionVelocityThreshold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurbulentNotAssessable {
    MissingQualityFlag,
    MissingFrictionVelocity,
    InvalidFrictionVelocity,
    InvalidThreshold,
    NonFiniteFlux,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TurbulentDisposition {
    Accepted { turbulent_energy_w_m2: f64 },
    Excluded(TurbulentExclusion),
    NotAssessable(TurbulentNotAssessable),
}

/// One paired H + LE record. Quality flags are retained separately from flux
/// values so the same observations can be compared under multiple policies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurbulentFluxSample {
    pub sensible_heat_w_m2: f64,
    pub latent_heat_w_m2: f64,
    pub full_qc_pass: Option<bool>,
    pub friction_velocity_m_s: Option<f64>,
}

pub fn turbulent_energy(
    sample: TurbulentFluxSample,
    policy: TurbulentQualityPolicy,
    minimum_friction_velocity_m_s: Option<f64>,
) -> TurbulentDisposition {
    if !sample.sensible_heat_w_m2.is_finite() || !sample.latent_heat_w_m2.is_finite() {
        return TurbulentDisposition::NotAssessable(TurbulentNotAssessable::NonFiniteFlux);
    }

    match policy {
        TurbulentQualityPolicy::Unfiltered => {}
        TurbulentQualityPolicy::FullQualityControl
        | TurbulentQualityPolicy::FrictionVelocityFiltered => match sample.full_qc_pass {
            Some(true) => {}
            Some(false) => {
                return TurbulentDisposition::Excluded(
                    TurbulentExclusion::FailedFullQualityControl,
                );
            }
            None => {
                return TurbulentDisposition::NotAssessable(
                    TurbulentNotAssessable::MissingQualityFlag,
                );
            }
        },
    }

    if policy == TurbulentQualityPolicy::FrictionVelocityFiltered {
        let threshold = match minimum_friction_velocity_m_s {
            Some(value) if value.is_finite() && value >= 0.0 => value,
            _ => {
                return TurbulentDisposition::NotAssessable(
                    TurbulentNotAssessable::InvalidThreshold,
                );
            }
        };
        let u_star = match sample.friction_velocity_m_s {
            Some(value) if value.is_finite() && value >= 0.0 => value,
            Some(_) => {
                return TurbulentDisposition::NotAssessable(
                    TurbulentNotAssessable::InvalidFrictionVelocity,
                );
            }
            None => {
                return TurbulentDisposition::NotAssessable(
                    TurbulentNotAssessable::MissingFrictionVelocity,
                );
            }
        };
        if u_star < threshold {
            return TurbulentDisposition::Excluded(
                TurbulentExclusion::BelowFrictionVelocityThreshold,
            );
        }
    }

    let energy = sample.sensible_heat_w_m2 + sample.latent_heat_w_m2;
    if !energy.is_finite() {
        return TurbulentDisposition::NotAssessable(TurbulentNotAssessable::NonFiniteFlux);
    }

    TurbulentDisposition::Accepted {
        turbulent_energy_w_m2: energy,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairedEnergyObservation {
    pub available_energy_w_m2: f64,
    pub turbulent_energy_w_m2: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmaResult {
    pub slope: f64,
    pub intercept_w_m2: f64,
    pub correlation: f64,
    pub residual_rms_w_m2: f64,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RmaError {
    TooFewSamples,
    NonFiniteSample,
    ZeroVarianceAvailableEnergy,
    ZeroVarianceTurbulentEnergy,
    ZeroCovariance,
    NonFiniteResult,
}

/// Reduced-major-axis fit of turbulent energy against available energy.
///
/// RMA is used because both axes have measurement error. This is a regression
/// diagnostic, not a conservation proof and not a reason to force slope = 1.
pub fn reduced_major_axis(samples: &[PairedEnergyObservation]) -> Result<RmaResult, RmaError> {
    if samples.len() < 2 {
        return Err(RmaError::TooFewSamples);
    }
    if samples.iter().any(|sample| {
        !sample.available_energy_w_m2.is_finite() || !sample.turbulent_energy_w_m2.is_finite()
    }) {
        return Err(RmaError::NonFiniteSample);
    }

    let n = samples.len() as f64;
    let mean_x = samples
        .iter()
        .map(|sample| sample.available_energy_w_m2)
        .sum::<f64>()
        / n;
    let mean_y = samples
        .iter()
        .map(|sample| sample.turbulent_energy_w_m2)
        .sum::<f64>()
        / n;

    let mut ss_x = 0.0;
    let mut ss_y = 0.0;
    let mut cross = 0.0;
    for sample in samples {
        let dx = sample.available_energy_w_m2 - mean_x;
        let dy = sample.turbulent_energy_w_m2 - mean_y;
        ss_x += dx * dx;
        ss_y += dy * dy;
        cross += dx * dy;
    }

    if ss_x == 0.0 {
        return Err(RmaError::ZeroVarianceAvailableEnergy);
    }
    if ss_y == 0.0 {
        return Err(RmaError::ZeroVarianceTurbulentEnergy);
    }
    if cross == 0.0 {
        return Err(RmaError::ZeroCovariance);
    }

    let slope = cross.signum() * (ss_y / ss_x).sqrt();
    let intercept = mean_y - slope * mean_x;
    let correlation = cross / (ss_x * ss_y).sqrt();
    let residual_sum = samples
        .iter()
        .map(|sample| {
            let residual =
                sample.turbulent_energy_w_m2 - (slope * sample.available_energy_w_m2 + intercept);
            residual * residual
        })
        .sum::<f64>();
    let residual_rms = (residual_sum / n).sqrt();

    if !slope.is_finite()
        || !intercept.is_finite()
        || !correlation.is_finite()
        || !residual_rms.is_finite()
    {
        return Err(RmaError::NonFiniteResult);
    }

    Ok(RmaResult {
        slope,
        intercept_w_m2: intercept,
        correlation,
        residual_rms_w_m2: residual_rms,
        sample_count: samples.len(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapFillPolicy {
    ObservedOnly,
    IncludeGapFilled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapFillState {
    Observed,
    GapFilled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntegratedEnergyInterval {
    pub interval_id: u64,
    /// UTC/Unix interval start; used to prove that the integrated window is contiguous.
    pub interval_start_unix_seconds: i64,
    pub duration_seconds: u64,
    pub available_energy_w_m2: f64,
    pub turbulent_energy_w_m2: f64,
    pub gap_fill_state: GapFillState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntegratedEbrResult {
    pub energy_balance_ratio: f64,
    pub available_energy_j_m2: f64,
    pub turbulent_energy_j_m2: f64,
    /// Start of the retained, continuous integration window (Unix seconds).
    pub window_start_unix_seconds: i64,
    /// End-exclusive boundary of the retained, continuous integration window.
    pub window_end_unix_seconds: i64,
    pub included_intervals: usize,
    pub excluded_gap_filled_intervals: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegratedEbrError {
    EmptyInput,
    DuplicateIntervalId,
    ZeroDuration,
    NonFiniteFlux,
    NoEligibleIntervals,
    NonContiguousIntervals,
    IntervalTimeOverflow,
    ZeroAvailableEnergyIntegral,
    NonFiniteResult,
}

/// Duration-weighted ΣTE / ΣAE. Gap-filled intervals are included only under
/// an explicit policy; the result is never clamped to [0, 1].
pub fn integrated_energy_balance_ratio(
    intervals: &[IntegratedEnergyInterval],
    gap_policy: GapFillPolicy,
) -> Result<IntegratedEbrResult, IntegratedEbrError> {
    if intervals.is_empty() {
        return Err(IntegratedEbrError::EmptyInput);
    }

    let mut ids = BTreeSet::new();
    let mut ordered_intervals = intervals.iter().collect::<Vec<_>>();
    for interval in intervals {
        if interval.duration_seconds == 0 {
            return Err(IntegratedEbrError::ZeroDuration);
        }
        if i64::try_from(interval.duration_seconds).is_err() {
            return Err(IntegratedEbrError::IntervalTimeOverflow);
        }
    }
    ordered_intervals.sort_by_key(|interval| interval.interval_start_unix_seconds);
    for pair in ordered_intervals.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        let duration = i64::try_from(previous.duration_seconds)
            .map_err(|_| IntegratedEbrError::IntervalTimeOverflow)?;
        let expected_next = previous
            .interval_start_unix_seconds
            .checked_add(duration)
            .ok_or(IntegratedEbrError::IntervalTimeOverflow)?;
        if next.interval_start_unix_seconds != expected_next {
            return Err(IntegratedEbrError::NonContiguousIntervals);
        }
    }

    let mut included_intervals = Vec::new();
    let mut excluded_gap_filled = 0;

    for interval in ordered_intervals {
        if !ids.insert(interval.interval_id) {
            return Err(IntegratedEbrError::DuplicateIntervalId);
        }
        if interval.gap_fill_state == GapFillState::GapFilled
            && gap_policy == GapFillPolicy::ObservedOnly
        {
            excluded_gap_filled += 1;
            continue;
        }
        if !interval.available_energy_w_m2.is_finite()
            || !interval.turbulent_energy_w_m2.is_finite()
        {
            return Err(IntegratedEbrError::NonFiniteFlux);
        }
        included_intervals.push(interval);
    }

    // Input intervals may be contiguous while the selected observed-only
    // population is not. Never present a disjoint sum as one continuous EBR.
    for pair in included_intervals.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        let duration = i64::try_from(previous.duration_seconds)
            .map_err(|_| IntegratedEbrError::IntervalTimeOverflow)?;
        let expected_next = previous
            .interval_start_unix_seconds
            .checked_add(duration)
            .ok_or(IntegratedEbrError::IntervalTimeOverflow)?;
        if next.interval_start_unix_seconds != expected_next {
            return Err(IntegratedEbrError::NonContiguousIntervals);
        }
    }

    if included_intervals.is_empty() {
        return Err(IntegratedEbrError::NoEligibleIntervals);
    }

    let mut available_energy_j_m2 = 0.0;
    let mut turbulent_energy_j_m2 = 0.0;
    for interval in &included_intervals {
        let duration = interval.duration_seconds as f64;
        available_energy_j_m2 += interval.available_energy_w_m2 * duration;
        turbulent_energy_j_m2 += interval.turbulent_energy_w_m2 * duration;
    }
    let included = included_intervals.len();

    if available_energy_j_m2 == 0.0 {
        return Err(IntegratedEbrError::ZeroAvailableEnergyIntegral);
    }

    let ratio = turbulent_energy_j_m2 / available_energy_j_m2;
    if !available_energy_j_m2.is_finite()
        || !turbulent_energy_j_m2.is_finite()
        || !ratio.is_finite()
    {
        return Err(IntegratedEbrError::NonFiniteResult);
    }

    let first = included_intervals
        .first()
        .ok_or(IntegratedEbrError::NoEligibleIntervals)?;
    let last = included_intervals
        .last()
        .ok_or(IntegratedEbrError::NoEligibleIntervals)?;
    let last_duration = i64::try_from(last.duration_seconds)
        .map_err(|_| IntegratedEbrError::IntervalTimeOverflow)?;
    let window_end = last
        .interval_start_unix_seconds
        .checked_add(last_duration)
        .ok_or(IntegratedEbrError::IntervalTimeOverflow)?;

    Ok(IntegratedEbrResult {
        energy_balance_ratio: ratio,
        available_energy_j_m2,
        turbulent_energy_j_m2,
        window_start_unix_seconds: first.interval_start_unix_seconds,
        window_end_unix_seconds: window_end,
        included_intervals: included,
        excluded_gap_filled_intervals: excluded_gap_filled,
    })
}

/// Signed residual AE - TE. A positive value indicates more declared available
/// energy than turbulent transfer; negative residuals (overclosure) are kept.
pub fn closure_residual_w_m2(
    available_energy_w_m2: f64,
    turbulent_energy_w_m2: f64,
) -> Result<f64, RmaError> {
    if !available_energy_w_m2.is_finite() || !turbulent_energy_w_m2.is_finite() {
        return Err(RmaError::NonFiniteSample);
    }
    Ok(available_energy_w_m2 - turbulent_energy_w_m2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured(value: f64) -> EnergyTerm {
        EnergyTerm::measured(value)
    }

    fn na() -> EnergyTerm {
        EnergyTerm::not_applicable()
    }

    fn full_terms() -> AvailableEnergyTerms {
        AvailableEnergyTerms {
            net_radiation: measured(500.0),
            soil_heat_flux: measured(40.0),
            soil_heat_storage: measured(10.0),
            air_sensible_storage: measured(5.0),
            air_latent_storage: measured(3.0),
            photosynthetic_energy: measured(20.0),
            biomass_heat_storage: measured(12.0),
        }
    }

    fn sample(ae: f64, te: f64) -> PairedEnergyObservation {
        PairedEnergyObservation {
            available_energy_w_m2: ae,
            turbulent_energy_w_m2: te,
        }
    }

    #[test]
    fn normalize_flux_density_passes_watts_per_square_metre_through() {
        assert_eq!(
            normalize_energy_measurement(
                125.0,
                EnergyMeasurementUnit::WattPerSquareMetre,
                None,
                EnergySourcePolarity::MatchesReferenceDirection
            ),
            Ok(125.0)
        );
    }

    #[test]
    fn normalize_interval_energy_divides_by_exact_duration() {
        assert_eq!(
            normalize_energy_measurement(
                180_000.0,
                EnergyMeasurementUnit::JoulePerSquareMetre,
                Some(1800),
                EnergySourcePolarity::MatchesReferenceDirection
            ),
            Ok(100.0)
        );
    }

    #[test]
    fn normalize_source_polarity_is_explicit_and_reversible() {
        assert_eq!(
            normalize_energy_measurement(
                100.0,
                EnergyMeasurementUnit::WattPerSquareMetre,
                None,
                EnergySourcePolarity::InvertedFromReferenceDirection
            ),
            Ok(-100.0)
        );
    }

    #[test]
    fn normalize_interval_energy_requires_positive_duration() {
        assert_eq!(
            normalize_energy_measurement(
                1.0,
                EnergyMeasurementUnit::JoulePerSquareMetre,
                None,
                EnergySourcePolarity::MatchesReferenceDirection
            ),
            Err(EnergyNormalizationError::MissingDuration)
        );
        assert_eq!(
            normalize_energy_measurement(
                1.0,
                EnergyMeasurementUnit::JoulePerSquareMetre,
                Some(0),
                EnergySourcePolarity::MatchesReferenceDirection
            ),
            Err(EnergyNormalizationError::ZeroDuration)
        );
    }

    #[test]
    fn normalize_unknown_units_and_non_finite_inputs_fail_closed() {
        assert_eq!(
            normalize_energy_measurement(
                1.0,
                EnergyMeasurementUnit::Unsupported,
                None,
                EnergySourcePolarity::MatchesReferenceDirection
            ),
            Err(EnergyNormalizationError::UnsupportedUnit)
        );
        assert_eq!(
            normalize_energy_measurement(
                f64::NAN,
                EnergyMeasurementUnit::WattPerSquareMetre,
                None,
                EnergySourcePolarity::MatchesReferenceDirection
            ),
            Err(EnergyNormalizationError::NonFiniteValue)
        );
        assert_eq!(
            normalize_energy_measurement(
                f64::MAX,
                EnergyMeasurementUnit::JoulePerSquareMetre,
                Some(1),
                EnergySourcePolarity::InvertedFromReferenceDirection
            ),
            Ok(-f64::MAX)
        );
    }

    #[test]
    fn ae1_is_a_net_radiation_only_conceptual_baseline() {
        let mut terms = full_terms();
        terms.soil_heat_flux = EnergyTerm::missing();
        let result = available_energy(terms, AvailableEnergyFormulation::Ae1).unwrap();
        assert_eq!(result.value_w_m2, 500.0);
    }

    #[test]
    fn ae2_subtracts_soil_heat_flux() {
        let result = available_energy(full_terms(), AvailableEnergyFormulation::Ae2).unwrap();
        assert_eq!(result.value_w_m2, 460.0);
    }

    #[test]
    fn ae3_subtracts_soil_storage_without_clamping() {
        let result = available_energy(full_terms(), AvailableEnergyFormulation::Ae3).unwrap();
        assert_eq!(result.value_w_m2, 450.0);
    }

    #[test]
    fn ae4_subtracts_air_column_storage_terms() {
        let result = available_energy(full_terms(), AvailableEnergyFormulation::Ae4).unwrap();
        assert_eq!(result.value_w_m2, 442.0);
    }

    #[test]
    fn ae5_subtracts_photosynthetic_energy() {
        let result = available_energy(full_terms(), AvailableEnergyFormulation::Ae5).unwrap();
        assert_eq!(result.value_w_m2, 422.0);
    }

    #[test]
    fn ae6_includes_biomass_heat_storage() {
        let result = available_energy(full_terms(), AvailableEnergyFormulation::Ae6).unwrap();
        assert_eq!(result.value_w_m2, 410.0);
    }

    #[test]
    fn missing_term_is_not_promoted_to_zero() {
        let mut terms = full_terms();
        terms.soil_heat_storage = EnergyTerm::missing();
        assert_eq!(
            available_energy(terms, AvailableEnergyFormulation::Ae3),
            Err(EnergyTermError::Missing(TermId::SoilHeatStorage))
        );
    }

    #[test]
    fn invalid_and_unqualified_terms_fail_closed() {
        let mut terms = full_terms();
        terms.biomass_heat_storage = EnergyTerm {
            status: ObservationStatus::Unqualified,
            value_w_m2: None,
        };
        assert_eq!(
            available_energy(terms, AvailableEnergyFormulation::Ae6),
            Err(EnergyTermError::InvalidStatus(
                TermId::BiomassHeatStorage,
                ObservationStatus::Unqualified
            ))
        );
    }

    #[test]
    fn out_of_domain_terms_fail_closed() {
        let mut terms = full_terms();
        terms.air_latent_storage = EnergyTerm {
            status: ObservationStatus::OutOfDomain,
            value_w_m2: None,
        };
        assert_eq!(
            available_energy(terms, AvailableEnergyFormulation::Ae4),
            Err(EnergyTermError::InvalidStatus(
                TermId::AirLatentStorage,
                ObservationStatus::OutOfDomain
            ))
        );
    }

    #[test]
    fn not_applicable_is_explicit_and_distinct_from_missing() {
        let mut terms = full_terms();
        terms.biomass_heat_storage = na();
        let result = available_energy(terms, AvailableEnergyFormulation::Ae6).unwrap();
        assert_eq!(result.value_w_m2, 422.0);
    }

    #[test]
    fn not_applicable_with_a_numeric_value_is_rejected() {
        let mut terms = full_terms();
        terms.biomass_heat_storage = EnergyTerm {
            status: ObservationStatus::NotApplicable,
            value_w_m2: Some(12.0),
        };
        assert_eq!(
            available_energy(terms, AvailableEnergyFormulation::Ae6),
            Err(EnergyTermError::ValueUnexpected(TermId::BiomassHeatStorage))
        );
    }

    #[test]
    fn non_finite_measured_term_is_rejected() {
        let mut terms = full_terms();
        terms.net_radiation = measured(f64::NAN);
        assert_eq!(
            available_energy(terms, AvailableEnergyFormulation::Ae1),
            Err(EnergyTermError::NonFinite(TermId::NetRadiation))
        );
    }

    #[test]
    fn ae_result_preserves_disposition_of_every_included_term() {
        let mut terms = full_terms();
        terms.soil_heat_flux = EnergyTerm::estimated(40.0);
        terms.soil_heat_storage = EnergyTerm::not_applicable();
        let result = available_energy(terms, AvailableEnergyFormulation::Ae3).unwrap();
        assert_eq!(result.value_w_m2, 460.0);
        assert_eq!(
            result.included_terms,
            vec![
                IncludedEnergyTerm {
                    term_id: TermId::NetRadiation,
                    disposition: IncludedTermDisposition::Measured,
                },
                IncludedEnergyTerm {
                    term_id: TermId::SoilHeatFlux,
                    disposition: IncludedTermDisposition::Estimated,
                },
                IncludedEnergyTerm {
                    term_id: TermId::SoilHeatStorage,
                    disposition: IncludedTermDisposition::NotApplicable,
                },
            ]
        );
    }

    #[test]
    fn lower_ae_formulation_does_not_claim_unselected_terms() {
        let mut terms = full_terms();
        terms.soil_heat_flux = EnergyTerm::missing();
        terms.biomass_heat_storage = EnergyTerm {
            status: ObservationStatus::Unqualified,
            value_w_m2: None,
        };
        let result = available_energy(terms, AvailableEnergyFormulation::Ae1).unwrap();
        assert_eq!(result.included_terms.len(), 1);
        assert_eq!(result.included_terms[0].term_id, TermId::NetRadiation);
    }

    #[test]
    fn signed_storage_release_can_increase_available_energy() {
        let mut terms = full_terms();
        terms.soil_heat_storage = measured(-5.0);
        let result = available_energy(terms, AvailableEnergyFormulation::Ae3).unwrap();
        assert_eq!(result.value_w_m2, 465.0);
    }

    #[test]
    fn full_qc_turbulent_energy_is_h_plus_le() {
        let sample = TurbulentFluxSample {
            sensible_heat_w_m2: 100.0,
            latent_heat_w_m2: 250.0,
            full_qc_pass: Some(true),
            friction_velocity_m_s: Some(0.4),
        };
        assert_eq!(
            turbulent_energy(sample, TurbulentQualityPolicy::FullQualityControl, None),
            TurbulentDisposition::Accepted {
                turbulent_energy_w_m2: 350.0
            }
        );
    }

    #[test]
    fn failed_full_qc_is_excluded_not_zeroed() {
        let sample = TurbulentFluxSample {
            sensible_heat_w_m2: 100.0,
            latent_heat_w_m2: 250.0,
            full_qc_pass: Some(false),
            friction_velocity_m_s: Some(0.4),
        };
        assert_eq!(
            turbulent_energy(sample, TurbulentQualityPolicy::FullQualityControl, None),
            TurbulentDisposition::Excluded(TurbulentExclusion::FailedFullQualityControl)
        );
    }

    #[test]
    fn missing_qc_flag_is_not_assessable() {
        let sample = TurbulentFluxSample {
            sensible_heat_w_m2: 100.0,
            latent_heat_w_m2: 250.0,
            full_qc_pass: None,
            friction_velocity_m_s: Some(0.4),
        };
        assert_eq!(
            turbulent_energy(sample, TurbulentQualityPolicy::FullQualityControl, None),
            TurbulentDisposition::NotAssessable(TurbulentNotAssessable::MissingQualityFlag)
        );
    }

    #[test]
    fn unfiltered_policy_does_not_claim_that_qc_passed() {
        let sample = TurbulentFluxSample {
            sensible_heat_w_m2: 100.0,
            latent_heat_w_m2: 250.0,
            full_qc_pass: None,
            friction_velocity_m_s: None,
        };
        assert_eq!(
            turbulent_energy(sample, TurbulentQualityPolicy::Unfiltered, None),
            TurbulentDisposition::Accepted {
                turbulent_energy_w_m2: 350.0
            }
        );
    }

    #[test]
    fn ustar_filter_accepts_only_at_or_above_threshold() {
        let sample = TurbulentFluxSample {
            sensible_heat_w_m2: 100.0,
            latent_heat_w_m2: 250.0,
            full_qc_pass: Some(true),
            friction_velocity_m_s: Some(0.3),
        };
        assert_eq!(
            turbulent_energy(
                sample,
                TurbulentQualityPolicy::FrictionVelocityFiltered,
                Some(0.3)
            ),
            TurbulentDisposition::Accepted {
                turbulent_energy_w_m2: 350.0
            }
        );
    }

    #[test]
    fn ustar_below_threshold_is_excluded() {
        let sample = TurbulentFluxSample {
            sensible_heat_w_m2: 100.0,
            latent_heat_w_m2: 250.0,
            full_qc_pass: Some(true),
            friction_velocity_m_s: Some(0.2),
        };
        assert_eq!(
            turbulent_energy(
                sample,
                TurbulentQualityPolicy::FrictionVelocityFiltered,
                Some(0.3)
            ),
            TurbulentDisposition::Excluded(TurbulentExclusion::BelowFrictionVelocityThreshold)
        );
    }

    #[test]
    fn ustar_missing_or_invalid_is_not_assessable() {
        let missing = TurbulentFluxSample {
            sensible_heat_w_m2: 1.0,
            latent_heat_w_m2: 1.0,
            full_qc_pass: Some(true),
            friction_velocity_m_s: None,
        };
        assert_eq!(
            turbulent_energy(
                missing,
                TurbulentQualityPolicy::FrictionVelocityFiltered,
                Some(0.2)
            ),
            TurbulentDisposition::NotAssessable(TurbulentNotAssessable::MissingFrictionVelocity)
        );

        let invalid = TurbulentFluxSample {
            friction_velocity_m_s: Some(-0.1),
            ..missing
        };
        assert_eq!(
            turbulent_energy(
                invalid,
                TurbulentQualityPolicy::FrictionVelocityFiltered,
                Some(0.2)
            ),
            TurbulentDisposition::NotAssessable(TurbulentNotAssessable::InvalidFrictionVelocity)
        );
    }

    #[test]
    fn rma_known_answer_preserves_both_axis_relationship() {
        let observations = [sample(1.0, 2.0), sample(2.0, 4.0), sample(3.0, 6.0)];
        let result = reduced_major_axis(&observations).unwrap();
        assert!((result.slope - 2.0).abs() < 1e-12);
        assert!(result.intercept_w_m2.abs() < 1e-12);
        assert!((result.correlation - 1.0).abs() < 1e-12);
        assert_eq!(result.sample_count, 3);
    }

    #[test]
    fn rma_does_not_clamp_overclosure_slope() {
        let observations = [sample(10.0, 20.0), sample(20.0, 40.0), sample(30.0, 60.0)];
        let result = reduced_major_axis(&observations).unwrap();
        assert!(result.slope > 1.0);
    }

    #[test]
    fn rma_requires_variable_and_finite_samples() {
        assert_eq!(
            reduced_major_axis(&[sample(1.0, 1.0)]),
            Err(RmaError::TooFewSamples)
        );
        assert_eq!(
            reduced_major_axis(&[sample(1.0, 1.0), sample(f64::NAN, 2.0)]),
            Err(RmaError::NonFiniteSample)
        );
        assert_eq!(
            reduced_major_axis(&[sample(1.0, 2.0), sample(1.0, 3.0)]),
            Err(RmaError::ZeroVarianceAvailableEnergy)
        );
    }

    #[test]
    fn integrated_ebr_is_duration_weighted_not_a_mean_of_ratios() {
        // Intentionally reversed: the EBR implementation sorts by the declared timestamps.
        let intervals = [
            IntegratedEnergyInterval {
                interval_id: 2,
                interval_start_unix_seconds: 1800,
                duration_seconds: 3600,
                available_energy_w_m2: 200.0,
                turbulent_energy_w_m2: 180.0,
                gap_fill_state: GapFillState::Observed,
            },
            IntegratedEnergyInterval {
                interval_id: 1,
                interval_start_unix_seconds: 0,
                duration_seconds: 1800,
                available_energy_w_m2: 100.0,
                turbulent_energy_w_m2: 50.0,
                gap_fill_state: GapFillState::Observed,
            },
        ];
        let result =
            integrated_energy_balance_ratio(&intervals, GapFillPolicy::ObservedOnly).unwrap();
        let expected = (50.0 * 1800.0 + 180.0 * 3600.0) / (100.0 * 1800.0 + 200.0 * 3600.0);
        assert!((result.energy_balance_ratio - expected).abs() < 1e-12);
        assert_eq!(result.included_intervals, 2);
    }

    #[test]
    fn observed_only_ebr_excludes_gapfilled_intervals_and_reports_them() {
        let intervals = [
            IntegratedEnergyInterval {
                interval_id: 1,
                interval_start_unix_seconds: 0,
                duration_seconds: 1800,
                available_energy_w_m2: 100.0,
                turbulent_energy_w_m2: 80.0,
                gap_fill_state: GapFillState::Observed,
            },
            IntegratedEnergyInterval {
                interval_id: 2,
                interval_start_unix_seconds: 1800,
                duration_seconds: 1800,
                available_energy_w_m2: 100.0,
                turbulent_energy_w_m2: 100.0,
                gap_fill_state: GapFillState::GapFilled,
            },
        ];
        let result =
            integrated_energy_balance_ratio(&intervals, GapFillPolicy::ObservedOnly).unwrap();
        assert!((result.energy_balance_ratio - 0.8).abs() < 1e-12);
        assert_eq!(result.included_intervals, 1);
        assert_eq!(result.excluded_gap_filled_intervals, 1);
        assert_eq!(result.window_start_unix_seconds, 0);
        assert_eq!(result.window_end_unix_seconds, 1800);
    }

    #[test]
    fn explicitly_including_gapfilled_data_changes_the_declared_population() {
        let intervals = [
            IntegratedEnergyInterval {
                interval_id: 1,
                interval_start_unix_seconds: 0,
                duration_seconds: 1800,
                available_energy_w_m2: 100.0,
                turbulent_energy_w_m2: 80.0,
                gap_fill_state: GapFillState::Observed,
            },
            IntegratedEnergyInterval {
                interval_id: 2,
                interval_start_unix_seconds: 1800,
                duration_seconds: 1800,
                available_energy_w_m2: 100.0,
                turbulent_energy_w_m2: 100.0,
                gap_fill_state: GapFillState::GapFilled,
            },
        ];
        let result =
            integrated_energy_balance_ratio(&intervals, GapFillPolicy::IncludeGapFilled).unwrap();
        assert!((result.energy_balance_ratio - 0.9).abs() < 1e-12);
        assert_eq!(result.included_intervals, 2);
        assert_eq!(result.excluded_gap_filled_intervals, 0);
        assert_eq!(result.window_start_unix_seconds, 0);
        assert_eq!(result.window_end_unix_seconds, 3600);
    }

    #[test]
    fn observed_only_ebr_ignores_unusable_values_in_excluded_gapfilled_intervals() {
        let intervals = [
            IntegratedEnergyInterval {
                interval_id: 1,
                interval_start_unix_seconds: 0,
                duration_seconds: 1800,
                available_energy_w_m2: 100.0,
                turbulent_energy_w_m2: 80.0,
                gap_fill_state: GapFillState::Observed,
            },
            IntegratedEnergyInterval {
                interval_id: 2,
                interval_start_unix_seconds: 1800,
                duration_seconds: 1800,
                available_energy_w_m2: f64::NAN,
                turbulent_energy_w_m2: f64::INFINITY,
                gap_fill_state: GapFillState::GapFilled,
            },
        ];
        let observed_only =
            integrated_energy_balance_ratio(&intervals, GapFillPolicy::ObservedOnly).unwrap();
        assert_eq!(observed_only.energy_balance_ratio, 0.8);
        assert_eq!(observed_only.included_intervals, 1);
        assert_eq!(observed_only.excluded_gap_filled_intervals, 1);

        assert_eq!(
            integrated_energy_balance_ratio(&intervals, GapFillPolicy::IncludeGapFilled),
            Err(IntegratedEbrError::NonFiniteFlux)
        );
    }

    #[test]
    fn integrated_ebr_rejects_duplicate_or_zero_duration_intervals() {
        let base = IntegratedEnergyInterval {
            interval_id: 1,
            interval_start_unix_seconds: 0,
            duration_seconds: 1800,
            available_energy_w_m2: 100.0,
            turbulent_energy_w_m2: 80.0,
            gap_fill_state: GapFillState::Observed,
        };
        assert_eq!(
            integrated_energy_balance_ratio(&[base, base], GapFillPolicy::ObservedOnly),
            Err(IntegratedEbrError::DuplicateIntervalId)
        );
        let zero = IntegratedEnergyInterval {
            duration_seconds: 0,
            ..base
        };
        assert_eq!(
            integrated_energy_balance_ratio(&[zero], GapFillPolicy::ObservedOnly),
            Err(IntegratedEbrError::ZeroDuration)
        );
    }

    #[test]
    fn observed_only_ebr_rejects_an_interior_gapfilled_gap() {
        let intervals = [
            IntegratedEnergyInterval {
                interval_id: 1,
                interval_start_unix_seconds: 0,
                duration_seconds: 1800,
                available_energy_w_m2: 100.0,
                turbulent_energy_w_m2: 80.0,
                gap_fill_state: GapFillState::Observed,
            },
            IntegratedEnergyInterval {
                interval_id: 2,
                interval_start_unix_seconds: 1800,
                duration_seconds: 1800,
                available_energy_w_m2: 100.0,
                turbulent_energy_w_m2: 100.0,
                gap_fill_state: GapFillState::GapFilled,
            },
            IntegratedEnergyInterval {
                interval_id: 3,
                interval_start_unix_seconds: 3600,
                duration_seconds: 1800,
                available_energy_w_m2: 100.0,
                turbulent_energy_w_m2: 75.0,
                gap_fill_state: GapFillState::Observed,
            },
        ];
        assert_eq!(
            integrated_energy_balance_ratio(&intervals, GapFillPolicy::ObservedOnly),
            Err(IntegratedEbrError::NonContiguousIntervals)
        );
        let inclusive =
            integrated_energy_balance_ratio(&intervals, GapFillPolicy::IncludeGapFilled).unwrap();
        assert_eq!(inclusive.included_intervals, 3);
        assert_eq!(inclusive.excluded_gap_filled_intervals, 0);
    }

    #[test]
    fn integrated_ebr_rejects_non_contiguous_windows() {
        let first = IntegratedEnergyInterval {
            interval_id: 1,
            interval_start_unix_seconds: 0,
            duration_seconds: 1800,
            available_energy_w_m2: 100.0,
            turbulent_energy_w_m2: 80.0,
            gap_fill_state: GapFillState::Observed,
        };
        let third = IntegratedEnergyInterval {
            interval_id: 3,
            interval_start_unix_seconds: 3600,
            duration_seconds: 1800,
            available_energy_w_m2: 100.0,
            turbulent_energy_w_m2: 80.0,
            gap_fill_state: GapFillState::Observed,
        };
        assert_eq!(
            integrated_energy_balance_ratio(&[first, third], GapFillPolicy::ObservedOnly),
            Err(IntegratedEbrError::NonContiguousIntervals)
        );
    }

    #[test]
    fn integrated_ebr_rejects_end_timestamp_overflow() {
        let interval = IntegratedEnergyInterval {
            interval_id: 1,
            interval_start_unix_seconds: i64::MAX - 10,
            duration_seconds: 20,
            available_energy_w_m2: 100.0,
            turbulent_energy_w_m2: 80.0,
            gap_fill_state: GapFillState::Observed,
        };
        assert_eq!(
            integrated_energy_balance_ratio(&[interval], GapFillPolicy::ObservedOnly),
            Err(IntegratedEbrError::IntervalTimeOverflow)
        );
    }

    #[test]
    fn residual_is_signed_and_overclosure_is_not_clamped() {
        assert_eq!(closure_residual_w_m2(100.0, 125.0).unwrap(), -25.0);
        assert_eq!(closure_residual_w_m2(100.0, 80.0).unwrap(), 20.0);
    }
}
