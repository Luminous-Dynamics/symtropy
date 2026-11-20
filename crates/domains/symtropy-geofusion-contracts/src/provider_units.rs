// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explicit SI-to-IAPWS-IF97 adapter boundary.
//!
//! This module does not calculate properties or select an IF97 region. It
//! prevents silent unit and composition mismatches before calling a provider.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaterCompositionClass {
    PureWater,
    SalineBrine,
    WaterWithNonCondensableGas,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct If97AdapterRequest {
    pub fluid_composition_id: String,
    /// Digest of the source artifact supporting the declared composition class.
    pub composition_evidence_digest: String,
    pub state_boundary_id: String,
    /// Absolute pressure in Pa at the exact state boundary.
    pub pressure_pa: f64,
    /// Absolute temperature in K at the exact state boundary.
    pub temperature_k: f64,
    pub composition: WaterCompositionClass,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VleSteamProviderInput {
    pub fluid_composition_id: String,
    pub composition_evidence_digest: String,
    pub state_boundary_id: String,
    /// Absolute pressure in kPa, matching the vle-steam v0.20.1 public API.
    pub pressure_kpa_abs: f64,
    /// Absolute temperature in K.
    pub temperature_k: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeuiF97ProviderInput {
    pub fluid_composition_id: String,
    pub composition_evidence_digest: String,
    pub state_boundary_id: String,
    /// Absolute pressure in MPa, matching the seuif97 2.3.8 API.
    pub pressure_mpa_abs: f64,
    /// Temperature in degrees Celsius, matching the seuif97 2.3.8 API.
    pub temperature_c: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum If97AdapterError {
    EmptyIdentity(&'static str),
    NonFiniteOrNonPositive(&'static str),
    UnsupportedComposition(WaterCompositionClass),
    ConversionUnderflow,
    ArithmeticOverflow,
}

impl std::fmt::Display for If97AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for If97AdapterError {}

/// Convert absolute pressure from Pa to kPa without changing the physical state.
pub fn pascals_to_kilopascals_abs(pa: f64) -> Result<f64, If97AdapterError> {
    if !pa.is_finite() || pa <= 0.0 {
        return Err(If97AdapterError::NonFiniteOrNonPositive(
            "absolute pressure Pa",
        ));
    }
    let kpa = pa / 1_000.0;
    if !kpa.is_finite() {
        return Err(If97AdapterError::ArithmeticOverflow);
    }
    if kpa <= 0.0 {
        return Err(If97AdapterError::ConversionUnderflow);
    }
    Ok(kpa)
}

/// Convert absolute pressure from Pa to MPa without changing the physical state.
pub fn pascals_to_megapascals_abs(pa: f64) -> Result<f64, If97AdapterError> {
    if !pa.is_finite() || pa <= 0.0 {
        return Err(If97AdapterError::NonFiniteOrNonPositive(
            "absolute pressure Pa",
        ));
    }
    let mpa = pa / 1_000_000.0;
    if !mpa.is_finite() {
        return Err(If97AdapterError::ArithmeticOverflow);
    }
    if mpa <= 0.0 {
        return Err(If97AdapterError::ConversionUnderflow);
    }
    Ok(mpa)
}

/// Convert absolute pressure from kPa to Pa without changing the physical state.
pub fn kilopascals_to_pascals_abs(kpa: f64) -> Result<f64, If97AdapterError> {
    if !kpa.is_finite() || kpa <= 0.0 {
        return Err(If97AdapterError::NonFiniteOrNonPositive(
            "absolute pressure kPa",
        ));
    }
    let pa = kpa * 1_000.0;
    if !pa.is_finite() {
        return Err(If97AdapterError::ArithmeticOverflow);
    }
    Ok(pa)
}

impl If97AdapterRequest {
    /// Create vle-steam-specific inputs only for an explicitly identified
    /// pure-water composition. IF97 is not a saline-brine mixture model.
    pub fn to_vle_steam_input(&self) -> Result<VleSteamProviderInput, If97AdapterError> {
        for (name, value) in [
            ("fluid composition ID", self.fluid_composition_id.as_str()),
            (
                "composition evidence artifact digest",
                self.composition_evidence_digest.as_str(),
            ),
            ("state boundary ID", self.state_boundary_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(If97AdapterError::EmptyIdentity(name));
            }
        }
        if self.composition != WaterCompositionClass::PureWater {
            return Err(If97AdapterError::UnsupportedComposition(
                self.composition,
            ));
        }
        if !self.temperature_k.is_finite() || self.temperature_k <= 0.0 {
            return Err(If97AdapterError::NonFiniteOrNonPositive(
                "absolute temperature K",
            ));
        }
        let pressure_kpa_abs = pascals_to_kilopascals_abs(self.pressure_pa)?;
        Ok(VleSteamProviderInput {
            fluid_composition_id: self.fluid_composition_id.clone(),
            composition_evidence_digest: self.composition_evidence_digest.clone(),
            state_boundary_id: self.state_boundary_id.clone(),
            pressure_kpa_abs,
            temperature_k: self.temperature_k,
        })
    }

    /// Create seuif97-specific inputs using MPa absolute and degrees Celsius.
    /// This is a distinct unit contract from the vle-steam K/kPa interface.
    pub fn to_seuif97_input(&self) -> Result<SeuiF97ProviderInput, If97AdapterError> {
        for (name, value) in [
            ("fluid composition ID", self.fluid_composition_id.as_str()),
            (
                "composition evidence artifact digest",
                self.composition_evidence_digest.as_str(),
            ),
            ("state boundary ID", self.state_boundary_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(If97AdapterError::EmptyIdentity(name));
            }
        }
        if self.composition != WaterCompositionClass::PureWater {
            return Err(If97AdapterError::UnsupportedComposition(
                self.composition,
            ));
        }
        if !self.temperature_k.is_finite() || self.temperature_k <= 0.0 {
            return Err(If97AdapterError::NonFiniteOrNonPositive(
                "absolute temperature K",
            ));
        }
        let temperature_c = self.temperature_k - 273.15;
        if !temperature_c.is_finite() {
            return Err(If97AdapterError::ArithmeticOverflow);
        }
        Ok(SeuiF97ProviderInput {
            fluid_composition_id: self.fluid_composition_id.clone(),
            composition_evidence_digest: self.composition_evidence_digest.clone(),
            state_boundary_id: self.state_boundary_id.clone(),
            pressure_mpa_abs: pascals_to_megapascals_abs(self.pressure_pa)?,
            temperature_c,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pure_water_request() -> If97AdapterRequest {
        If97AdapterRequest {
            fluid_composition_id: "pure-water-v1".into(),
            composition_evidence_digest: "synthetic-composition-evidence".into(),
            state_boundary_id: "wellhead-01".into(),
            pressure_pa: 10.0e6,
            temperature_k: 648.15,
            composition: WaterCompositionClass::PureWater,
        }
    }

    #[test]
    fn pa_to_kpa_is_explicit_and_preserves_absolute_pressure() {
        assert_eq!(
            pascals_to_kilopascals_abs(1.0e6).unwrap(),
            1_000.0
        );
        assert_eq!(
            kilopascals_to_pascals_abs(1_000.0).unwrap(),
            1.0e6
        );
    }

    #[test]
    fn provider_input_preserves_fluid_and_boundary_identity() {
        let input = pure_water_request().to_vle_steam_input().unwrap();
        assert_eq!(input.pressure_kpa_abs, 10_000.0);
        assert_eq!(input.temperature_k, 648.15);
        assert_eq!(input.fluid_composition_id, "pure-water-v1");
        assert_eq!(input.composition_evidence_digest, "synthetic-composition-evidence");
        assert_eq!(input.state_boundary_id, "wellhead-01");
    }

    #[test]
    fn saline_brine_is_refused_before_property_call() {
        let mut request = pure_water_request();
        request.composition = WaterCompositionClass::SalineBrine;
        assert_eq!(
            request.to_vle_steam_input(),
            Err(If97AdapterError::UnsupportedComposition(
                WaterCompositionClass::SalineBrine
            ))
        );
    }

    #[test]
    fn non_condensable_gas_and_unknown_composition_are_refused() {
        for composition in [
            WaterCompositionClass::WaterWithNonCondensableGas,
            WaterCompositionClass::Unknown,
        ] {
            let mut request = pure_water_request();
            request.composition = composition;
            assert_eq!(
                request.to_vle_steam_input(),
                Err(If97AdapterError::UnsupportedComposition(composition))
            );
        }
    }

    #[test]
    fn invalid_pressure_and_temperature_are_refused() {
        assert_eq!(
            pascals_to_kilopascals_abs(f64::NAN),
            Err(If97AdapterError::NonFiniteOrNonPositive(
                "absolute pressure Pa"
            ))
        );
        assert_eq!(
            pascals_to_kilopascals_abs(0.0),
            Err(If97AdapterError::NonFiniteOrNonPositive(
                "absolute pressure Pa"
            ))
        );
        let mut request = pure_water_request();
        request.temperature_k = f64::INFINITY;
        assert_eq!(
            request.to_vle_steam_input(),
            Err(If97AdapterError::NonFiniteOrNonPositive(
                "absolute temperature K"
            ))
        );
    }

    #[test]
    fn conversion_underflow_and_overflow_are_not_silently_accepted() {
        assert_eq!(
            pascals_to_kilopascals_abs(f64::from_bits(1)),
            Err(If97AdapterError::ConversionUnderflow)
        );
        assert_eq!(
            kilopascals_to_pascals_abs(f64::MAX),
            Err(If97AdapterError::ArithmeticOverflow)
        );
    }

    #[test]
    fn seuif97_adapter_uses_mpa_and_celsius_not_vle_steam_units() {
        let input = pure_water_request().to_seuif97_input().unwrap();
        assert_eq!(input.pressure_mpa_abs, 10.0);
        assert!((input.temperature_c - 375.0).abs() < 1e-12);
        assert_eq!(input.fluid_composition_id, "pure-water-v1");
        assert_eq!(input.state_boundary_id, "wellhead-01");
    }

    #[test]
    fn pa_to_mpa_is_explicit_and_checked() {
        assert_eq!(pascals_to_megapascals_abs(1.0e6).unwrap(), 1.0);
        assert_eq!(
            pascals_to_megapascals_abs(f64::from_bits(1)),
            Err(If97AdapterError::ConversionUnderflow)
        );
    }

    #[test]
    fn seuif97_adapter_also_refuses_brine_and_non_physical_temperature() {
        let mut request = pure_water_request();
        request.composition = WaterCompositionClass::SalineBrine;
        assert_eq!(
            request.to_seuif97_input(),
            Err(If97AdapterError::UnsupportedComposition(
                WaterCompositionClass::SalineBrine
            ))
        );

        request = pure_water_request();
        request.temperature_k = f64::NAN;
        assert_eq!(
            request.to_seuif97_input(),
            Err(If97AdapterError::NonFiniteOrNonPositive(
                "absolute temperature K"
            ))
        );
    }

    #[test]
    fn provider_input_does_not_confuse_gauge_and_absolute_pressure() {
        let mut request = pure_water_request();
        request.pressure_pa = 101_325.0;
        let input = request.to_vle_steam_input().unwrap();
        assert_eq!(input.pressure_kpa_abs, 101.325);
    }

    #[test]
    fn missing_composition_evidence_digest_is_refused_for_both_providers() {
        let mut request = pure_water_request();
        request.composition_evidence_digest.clear();
        assert_eq!(
            request.to_vle_steam_input(),
            Err(If97AdapterError::EmptyIdentity(
                "composition evidence artifact digest"
            ))
        );
        assert_eq!(
            request.to_seuif97_input(),
            Err(If97AdapterError::EmptyIdentity(
                "composition evidence artifact digest"
            ))
        );
    }

    #[test]
    fn missing_state_identity_is_refused() {
        let mut request = pure_water_request();
        request.state_boundary_id.clear();
        assert_eq!(
            request.to_vle_steam_input(),
            Err(If97AdapterError::EmptyIdentity("state boundary ID"))
        );
    }
}
