// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Source-bounded geothermal reservoir-to-wellhead state contracts.
//! This is an interface and arithmetic layer, not a reservoir or multiphase solver.

use std::f64::consts::PI;

use crate::EvidenceClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WellheadPhase {
    SinglePhaseLiquid,
    SinglePhaseVapor,
    Supercritical,
    TwoPhase,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluidPropertyProvenance {
    pub fluid_composition_id: String,
    pub model_id: String,
    pub model_version: String,
    pub formulation_or_database_id: String,
    pub reference_state_id: String,
    pub output_digest: String,
    pub state_boundary_id: String,
    pub evidence_class: EvidenceClass,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReservoirBoundaryState {
    pub well_id: String,
    pub boundary_id: String,
    pub pressure_pa: f64,
    pub temperature_k: f64,
    /// Permeability-thickness product in mD·m, not hydraulic transmissivity m²/s.
    pub permeability_thickness_md_m: f64,
    pub evidence_class: EvidenceClass,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WellheadBoundaryState {
    pub well_id: String,
    pub boundary_id: String,
    pub pressure_pa: f64,
    pub temperature_k: f64,
    pub mass_flow_kg_s: f64,
    pub casing_inner_diameter_m: f64,
    pub density_kg_m3: f64,
    pub specific_enthalpy_j_kg: f64,
    pub reference_specific_enthalpy_j_kg: f64,
    pub phase: WellheadPhase,
    pub property_provenance: FluidPropertyProvenance,
    pub evidence_class: EvidenceClass,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReservoirWellheadCase {
    pub case_id: String,
    pub reservoir: ReservoirBoundaryState,
    pub wellhead: WellheadBoundaryState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WellheadFlowMetrics {
    pub casing_cross_section_m2: f64,
    pub volumetric_flow_m3_s: f64,
    pub bulk_velocity_m_s: f64,
    /// Positive means reservoir temperature exceeds wellhead temperature.
    pub reservoir_to_wellhead_temperature_change_k: f64,
    /// rho * (h - h_reference); not exergy or an independent measurement.
    pub reference_relative_volumetric_enthalpy_j_m3: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WellheadValidationError {
    EmptyIdentity(&'static str),
    NonFiniteOrNonPositive(&'static str),
    NonFinite(&'static str),
    WellIdentityMismatch,
    PropertyBoundaryMismatch,
    PropertyProvenanceMissing(&'static str),
    TwoPhaseRequiresQualifiedHydraulics,
    UnknownPhase,
    ArithmeticOverflow,
}

impl std::fmt::Display for WellheadValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for WellheadValidationError {}

fn require_id(name: &'static str, value: &str) -> Result<(), WellheadValidationError> {
    if value.trim().is_empty() {
        return Err(WellheadValidationError::EmptyIdentity(name));
    }
    Ok(())
}

fn require_positive(name: &'static str, value: f64) -> Result<(), WellheadValidationError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(WellheadValidationError::NonFiniteOrNonPositive(name));
    }
    Ok(())
}

impl ReservoirWellheadCase {
    /// Validate exact state/provenance joins and compute single-phase bulk-flow metrics.
    /// Two-phase flow requires a qualified multiphase hydraulics model and is refused here.
    pub fn evaluate(&self) -> Result<WellheadFlowMetrics, WellheadValidationError> {
        require_id("case ID", &self.case_id)?;
        require_id("reservoir well ID", &self.reservoir.well_id)?;
        require_id("wellhead well ID", &self.wellhead.well_id)?;
        require_id("reservoir boundary ID", &self.reservoir.boundary_id)?;
        require_id("wellhead boundary ID", &self.wellhead.boundary_id)?;

        if self.reservoir.well_id != self.wellhead.well_id {
            return Err(WellheadValidationError::WellIdentityMismatch);
        }

        for (name, value) in [
            ("reservoir pressure Pa", self.reservoir.pressure_pa),
            ("reservoir temperature K", self.reservoir.temperature_k),
            ("permeability-thickness mD m", self.reservoir.permeability_thickness_md_m),
            ("wellhead pressure Pa", self.wellhead.pressure_pa),
            ("wellhead temperature K", self.wellhead.temperature_k),
            ("mass flow kg/s", self.wellhead.mass_flow_kg_s),
            ("casing inner diameter m", self.wellhead.casing_inner_diameter_m),
            ("density kg/m3", self.wellhead.density_kg_m3),
        ] {
            require_positive(name, value)?;
        }
        for (name, value) in [
            ("specific enthalpy J/kg", self.wellhead.specific_enthalpy_j_kg),
            ("reference specific enthalpy J/kg", self.wellhead.reference_specific_enthalpy_j_kg),
        ] {
            if !value.is_finite() {
                return Err(WellheadValidationError::NonFinite(name));
            }
        }

        let p = &self.wellhead.property_provenance;
        for (name, value) in [
            ("fluid composition ID", p.fluid_composition_id.as_str()),
            ("property model ID", p.model_id.as_str()),
            ("property model version", p.model_version.as_str()),
            ("property database ID", p.formulation_or_database_id.as_str()),
            ("reference state ID", p.reference_state_id.as_str()),
            ("property output digest", p.output_digest.as_str()),
            ("property state boundary ID", p.state_boundary_id.as_str()),
        ] {
            require_id(name, value)?;
        }
        if p.state_boundary_id != self.wellhead.boundary_id {
            return Err(WellheadValidationError::PropertyBoundaryMismatch);
        }

        match self.wellhead.phase {
            WellheadPhase::SinglePhaseLiquid
            | WellheadPhase::SinglePhaseVapor
            | WellheadPhase::Supercritical => {}
            WellheadPhase::TwoPhase => {
                return Err(WellheadValidationError::TwoPhaseRequiresQualifiedHydraulics);
            }
            WellheadPhase::Unknown => return Err(WellheadValidationError::UnknownPhase),
        }

        let diameter_squared = self.wellhead.casing_inner_diameter_m.powi(2);
        let area = PI * diameter_squared / 4.0;
        let volumetric_flow = self.wellhead.mass_flow_kg_s / self.wellhead.density_kg_m3;
        let velocity = volumetric_flow / area;
        let temperature_change =
            self.reservoir.temperature_k - self.wellhead.temperature_k;
        let volumetric_enthalpy = self.wellhead.density_kg_m3
            * (self.wellhead.specific_enthalpy_j_kg
                - self.wellhead.reference_specific_enthalpy_j_kg);

        if !area.is_finite()
            || area <= 0.0
            || !volumetric_flow.is_finite()
            || volumetric_flow <= 0.0
            || !velocity.is_finite()
            || velocity <= 0.0
            || !temperature_change.is_finite()
            || !volumetric_enthalpy.is_finite()
        {
            return Err(WellheadValidationError::ArithmeticOverflow);
        }

        Ok(WellheadFlowMetrics {
            casing_cross_section_m2: area,
            volumetric_flow_m3_s: volumetric_flow,
            bulk_velocity_m_s: velocity,
            reservoir_to_wellhead_temperature_change_k: temperature_change,
            reference_relative_volumetric_enthalpy_j_m3: volumetric_enthalpy,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VelocityReferenceThreshold {
    pub threshold_id: String,
    pub threshold_m_s: f64,
    pub basis_reference: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityReferenceComparison {
    pub velocity_m_s: f64,
    pub threshold_m_s: f64,
    pub exceeds_declared_reference: bool,
    pub margin_m_s: f64,
}

impl VelocityReferenceThreshold {
    /// Screening comparison only; this is not a universal erosion or safety limit.
    pub fn compare(
        &self,
        metrics: WellheadFlowMetrics,
    ) -> Result<VelocityReferenceComparison, WellheadValidationError> {
        require_id("velocity threshold ID", &self.threshold_id)?;
        require_id("velocity threshold basis", &self.basis_reference)?;
        require_positive("velocity threshold m/s", self.threshold_m_s)?;
        require_positive("bulk velocity m/s", metrics.bulk_velocity_m_s)?;
        let margin = self.threshold_m_s - metrics.bulk_velocity_m_s;
        if !margin.is_finite() {
            return Err(WellheadValidationError::ArithmeticOverflow);
        }
        Ok(VelocityReferenceComparison {
            velocity_m_s: metrics.bulk_velocity_m_s,
            threshold_m_s: self.threshold_m_s,
            exceeds_declared_reference: margin < 0.0,
            margin_m_s: margin,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case() -> ReservoirWellheadCase {
        ReservoirWellheadCase {
            case_id: "synthetic-superhot-01".into(),
            reservoir: ReservoirBoundaryState {
                well_id: "well-01".into(),
                boundary_id: "feedzone-01".into(),
                pressure_pa: 30.0e6,
                temperature_k: 748.15,
                permeability_thickness_md_m: 1000.0,
                evidence_class: EvidenceClass::Synthetic,
            },
            wellhead: WellheadBoundaryState {
                well_id: "well-01".into(),
                boundary_id: "wellhead-01".into(),
                pressure_pa: 10.0e6,
                temperature_k: 648.15,
                mass_flow_kg_s: 100.0,
                casing_inner_diameter_m: 0.15,
                density_kg_m3: 100.0,
                specific_enthalpy_j_kg: 2.8e6,
                reference_specific_enthalpy_j_kg: 0.084e6,
                phase: WellheadPhase::Supercritical,
                property_provenance: FluidPropertyProvenance {
                    fluid_composition_id: "synthetic-water".into(),
                    model_id: "synthetic-properties".into(),
                    model_version: "0.1".into(),
                    formulation_or_database_id: "synthetic-reference".into(),
                    reference_state_id: "synthetic-h0".into(),
                    output_digest: "synthetic-output-digest".into(),
                    state_boundary_id: "wellhead-01".into(),
                    evidence_class: EvidenceClass::Synthetic,
                },
                evidence_class: EvidenceClass::Synthetic,
            },
        }
    }

    #[test]
    fn single_phase_bulk_velocity_and_state_contrast_are_reproducible() {
        let metrics = case().evaluate().unwrap();
        let area = PI * 0.15_f64.powi(2) / 4.0;
        assert!((metrics.casing_cross_section_m2 - area).abs() < 1e-12);
        assert!((metrics.volumetric_flow_m3_s - 1.0).abs() < 1e-12);
        assert!((metrics.bulk_velocity_m_s - 1.0 / area).abs() < 1e-10);
        assert_eq!(metrics.reservoir_to_wellhead_temperature_change_k, 100.0);
        assert!((metrics.reference_relative_volumetric_enthalpy_j_m3 - 271.6e6).abs() < 1.0);
    }

    #[test]
    fn two_phase_flow_requires_specialist_hydraulics() {
        let mut value = case();
        value.wellhead.phase = WellheadPhase::TwoPhase;
        assert_eq!(
            value.evaluate(),
            Err(WellheadValidationError::TwoPhaseRequiresQualifiedHydraulics)
        );
    }

    #[test]
    fn unknown_phase_is_not_ranked() {
        let mut value = case();
        value.wellhead.phase = WellheadPhase::Unknown;
        assert_eq!(value.evaluate(), Err(WellheadValidationError::UnknownPhase));
    }

    #[test]
    fn reservoir_and_wellhead_must_identify_the_same_well() {
        let mut value = case();
        value.wellhead.well_id = "other-well".into();
        assert_eq!(
            value.evaluate(),
            Err(WellheadValidationError::WellIdentityMismatch)
        );
    }

    #[test]
    fn property_output_must_bind_to_the_exact_wellhead_boundary() {
        let mut value = case();
        value.wellhead.property_provenance.state_boundary_id = "other-plane".into();
        assert_eq!(
            value.evaluate(),
            Err(WellheadValidationError::PropertyBoundaryMismatch)
        );
    }

    #[test]
    fn missing_property_model_identity_is_refused() {
        let mut value = case();
        value.wellhead.property_provenance.model_version.clear();
        assert_eq!(
            value.evaluate(),
            Err(WellheadValidationError::PropertyProvenanceMissing(
                "property model version"
            ))
        );
    }

    #[test]
    fn nonphysical_density_or_casing_geometry_is_refused() {
        let mut value = case();
        value.wellhead.density_kg_m3 = 0.0;
        assert_eq!(
            value.evaluate(),
            Err(WellheadValidationError::NonFiniteOrNonPositive("density kg/m3"))
        );
        value.wellhead.density_kg_m3 = 100.0;
        value.wellhead.casing_inner_diameter_m = f64::NAN;
        assert_eq!(
            value.evaluate(),
            Err(WellheadValidationError::NonFiniteOrNonPositive(
                "casing inner diameter m"
            ))
        );
    }

    #[test]
    fn overflow_in_velocity_calculation_is_refused() {
        let mut value = case();
        value.wellhead.mass_flow_kg_s = f64::MAX;
        value.wellhead.density_kg_m3 = f64::MIN_POSITIVE;
        assert_eq!(
            value.evaluate(),
            Err(WellheadValidationError::ArithmeticOverflow)
        );
    }

    #[test]
    fn velocity_reference_requires_explicit_basis_and_is_not_universal() {
        let metrics = case().evaluate().unwrap();
        let threshold = VelocityReferenceThreshold {
            threshold_id: "literature-screening-reference-50mps".into(),
            threshold_m_s: 50.0,
            basis_reference: "Scott 2026 cited reference; screening only".into(),
        };
        let result = threshold.compare(metrics).unwrap();
        assert!(result.exceeds_declared_reference);
        assert!(result.margin_m_s < 0.0);
    }
}
