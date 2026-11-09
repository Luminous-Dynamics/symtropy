// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! First-order standard uncertainty propagation for one hydraulic-to-electric
//! operating point.
//!
//! V0.1 assumes the six supplied standard uncertainties are mutually
//! independent. It does not infer independence from sensor identity, construct
//! a confidence interval, or support covariance/distributional claims. Missing
//! any required standard uncertainty returns None instead of treating it as 0.

use core::fmt;

use crate::{EvidenceReference, HydraulicGenerationResult, STANDARD_GRAVITY_M_PER_S2};

/// A non-negative standard uncertainty expressed in the unit of its paired
/// physical quantity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StandardUncertainty(f64);

impl StandardUncertainty {
    pub fn new(value: f64) -> Result<Self, UncertaintyError> {
        if !value.is_finite() || value < 0.0 {
            return Err(UncertaintyError::InvalidStandardUncertainty { value });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Provenance for one standard-uncertainty value. Strings identify the
/// method, observation/assessment window, instrument or map and calibration
/// revision/validity. They are retained as declared metadata, not authenticated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncertaintyProvenance {
    evidence: EvidenceReference,
    method: String,
    observation_window: String,
    instrument_or_map_id: String,
    revision: String,
    calibration_validity: String,
}

impl UncertaintyProvenance {
    pub fn new(
        evidence: EvidenceReference,
        method: impl Into<String>,
        observation_window: impl Into<String>,
        instrument_or_map_id: impl Into<String>,
        revision: impl Into<String>,
        calibration_validity: impl Into<String>,
    ) -> Result<Self, UncertaintyError> {
        let method = method.into();
        let observation_window = observation_window.into();
        let instrument_or_map_id = instrument_or_map_id.into();
        let revision = revision.into();
        let calibration_validity = calibration_validity.into();

        for (field, value) in [
            ("method", method.as_str()),
            ("observation_window", observation_window.as_str()),
            ("instrument_or_map_id", instrument_or_map_id.as_str()),
            ("revision", revision.as_str()),
            ("calibration_validity", calibration_validity.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(UncertaintyError::MissingProvenance { field });
            }
        }

        Ok(Self {
            evidence,
            method,
            observation_window,
            instrument_or_map_id,
            revision,
            calibration_validity,
        })
    }

    pub fn evidence(&self) -> &EvidenceReference {
        &self.evidence
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn observation_window(&self) -> &str {
        &self.observation_window
    }

    pub fn instrument_or_map_id(&self) -> &str {
        &self.instrument_or_map_id
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }

    pub fn calibration_validity(&self) -> &str {
        &self.calibration_validity
    }
}

/// One standard uncertainty bound to its own provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceBoundStandardUncertainty {
    value: StandardUncertainty,
    provenance: UncertaintyProvenance,
}

impl EvidenceBoundStandardUncertainty {
    pub const fn new(value: StandardUncertainty, provenance: UncertaintyProvenance) -> Self {
        Self { value, provenance }
    }

    pub const fn value(&self) -> StandardUncertainty {
        self.value
    }

    pub fn provenance(&self) -> &UncertaintyProvenance {
        &self.provenance
    }
}

/// Standard uncertainties for the exact component boundary used by the
/// hydraulic generation result.
///
/// Each field is optional by design. None means unknown, not zero. Each
/// populated field has its own source, method, measurement window, instrument
/// or map identity, revision and calibration-validity metadata. The turbine
/// flow is the measured turbine branch, not the upstream/bypass allocation.
/// net_head_m is uncertainty of derived net head, including the caller's
/// declared conveyance/head-loss method.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HydraulicPowerStandardUncertainties {
    /// Standard uncertainty in kg/m³.
    pub water_density_kg_per_m3: Option<EvidenceBoundStandardUncertainty>,
    /// Standard uncertainty in m³/s at the turbine branch.
    pub turbine_flow_m3_per_s: Option<EvidenceBoundStandardUncertainty>,
    /// Standard uncertainty in m of net hydraulic head.
    pub net_head_m: Option<EvidenceBoundStandardUncertainty>,
    /// Dimensionless standard uncertainty in turbine hydraulic efficiency.
    pub turbine_efficiency: Option<EvidenceBoundStandardUncertainty>,
    /// Dimensionless standard uncertainty in generator efficiency.
    pub generator_efficiency: Option<EvidenceBoundStandardUncertainty>,
    /// Standard uncertainty in auxiliary electrical power, W.
    pub auxiliary_power_w: Option<EvidenceBoundStandardUncertainty>,
}

impl HydraulicPowerStandardUncertainties {
    pub fn is_complete(&self) -> bool {
        self.water_density_kg_per_m3.is_some()
            && self.turbine_flow_m3_per_s.is_some()
            && self.net_head_m.is_some()
            && self.turbine_efficiency.is_some()
            && self.generator_efficiency.is_some()
            && self.auxiliary_power_w.is_some()
    }
}

/// The v0.1 calculation explicitly assumes independent input uncertainties.
/// Correlated inputs require a covariance-aware method and are out of scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncertaintyDependence {
    IndependentStandardUncertaintiesAssumed,
}

/// Local Jacobian of net electrical power with respect to the six component
/// inputs. Units are W per unit of the named input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetPowerSensitivity {
    pub water_density_w_per_kg_m3: f64,
    pub turbine_flow_w_per_m3_per_s: f64,
    pub net_head_w_per_m: f64,
    pub turbine_efficiency_w_per_unit: f64,
    pub generator_efficiency_w_per_unit: f64,
    pub auxiliary_power_w_per_w: f64,
}

/// Signed first-order output contributions, in watts, from each input's
/// standard uncertainty. The combined variance uses their squared values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetPowerUncertaintyContributions {
    pub water_density_w: f64,
    pub turbine_flow_w: f64,
    pub net_head_w: f64,
    pub turbine_efficiency_w: f64,
    pub generator_efficiency_w: f64,
    pub auxiliary_power_w: f64,
}

/// First-order standard uncertainty of net electrical power at one operating
/// point. This is not an expanded uncertainty, confidence interval or
/// probability of service compliance.
#[derive(Debug, Clone, PartialEq)]
pub struct NetPowerUncertaintyReport {
    pub standard_uncertainty_w: StandardUncertainty,
    pub sensitivity: NetPowerSensitivity,
    pub contributions: NetPowerUncertaintyContributions,
    pub dependence: UncertaintyDependence,
    pub input_uncertainties: HydraulicPowerStandardUncertainties,
    /// False means the nominal point fails the water-service or source
    /// evidence eligibility gates. Uncertainty cannot lift either gate.
    pub comparison_eligible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UncertaintyError {
    InvalidStandardUncertainty { value: f64 },
    MissingProvenance { field: &'static str },
    NonFiniteSensitivity { field: &'static str, value: f64 },
    NonFiniteContribution { field: &'static str, value: f64 },
    NonFiniteCombinedUncertainty { value: f64 },
}

impl fmt::Display for UncertaintyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStandardUncertainty { value } => write!(
                formatter,
                "standard uncertainty must be finite and non-negative; got {value}"
            ),
            Self::MissingProvenance { field } => {
                write!(formatter, "uncertainty provenance field {field} must not be empty")
            }
            Self::NonFiniteSensitivity { field, value } => {
                write!(formatter, "sensitivity {field} is non-finite: {value}")
            }
            Self::NonFiniteContribution { field, value } => {
                write!(
                    formatter,
                    "uncertainty contribution {field} is non-finite: {value}"
                )
            }
            Self::NonFiniteCombinedUncertainty { value } => {
                write!(
                    formatter,
                    "combined standard uncertainty is non-finite: {value}"
                )
            }
        }
    }
}

impl std::error::Error for UncertaintyError {}

/// Propagate first-order standard uncertainty for:
///
/// P_net = rho * g * Q_turbine * H_net * eta_turbine * eta_generator - P_aux
///
/// If any of the six required standard uncertainties is unknown, returns
/// Ok(None). No missing term is replaced with zero.
///
/// V0.1 assumes mutually independent standard uncertainties:
///
/// u(P_net)^2 = sum_i ((dP_net/dx_i) * u(x_i))^2
///
/// The evidence reference is retained but not authenticated here. The result
/// remains diagnostic if the nominal hydraulic result is not eligible.
pub fn propagate_net_power_standard_uncertainty(
    result: &HydraulicGenerationResult,
    uncertainties: &HydraulicPowerStandardUncertainties,
) -> Result<Option<NetPowerUncertaintyReport>, UncertaintyError> {
    let (
        Some(u_density),
        Some(u_flow),
        Some(u_head),
        Some(u_turbine_efficiency),
        Some(u_generator_efficiency),
        Some(u_auxiliary),
    ) = (
        uncertainties.water_density_kg_per_m3.as_ref(),
        uncertainties.turbine_flow_m3_per_s.as_ref(),
        uncertainties.net_head_m.as_ref(),
        uncertainties.turbine_efficiency.as_ref(),
        uncertainties.generator_efficiency.as_ref(),
        uncertainties.auxiliary_power_w.as_ref(),
    )
    else {
        return Ok(None);
    };

    let density = result.hydraulic_input.water_density().get();
    let flow = result.hydraulic_input.flow_rate().get();
    let net_head = result.hydraulic_input.net_head().get();
    let turbine_efficiency = result.turbine_map.efficiency().get();
    let generator_efficiency = result.generator_model.efficiency().get();

    let sensitivity = NetPowerSensitivity {
        water_density_w_per_kg_m3: STANDARD_GRAVITY_M_PER_S2
            * flow
            * net_head
            * turbine_efficiency
            * generator_efficiency,
        turbine_flow_w_per_m3_per_s: density
            * STANDARD_GRAVITY_M_PER_S2
            * net_head
            * turbine_efficiency
            * generator_efficiency,
        net_head_w_per_m: density
            * STANDARD_GRAVITY_M_PER_S2
            * flow
            * turbine_efficiency
            * generator_efficiency,
        turbine_efficiency_w_per_unit: density
            * STANDARD_GRAVITY_M_PER_S2
            * flow
            * net_head
            * generator_efficiency,
        generator_efficiency_w_per_unit: density
            * STANDARD_GRAVITY_M_PER_S2
            * flow
            * net_head
            * turbine_efficiency,
        auxiliary_power_w_per_w: -1.0,
    };

    for (field, value) in [
        ("water_density", sensitivity.water_density_w_per_kg_m3),
        ("turbine_flow", sensitivity.turbine_flow_w_per_m3_per_s),
        ("net_head", sensitivity.net_head_w_per_m),
        (
            "turbine_efficiency",
            sensitivity.turbine_efficiency_w_per_unit,
        ),
        (
            "generator_efficiency",
            sensitivity.generator_efficiency_w_per_unit,
        ),
        ("auxiliary_power", sensitivity.auxiliary_power_w_per_w),
    ] {
        if !value.is_finite() {
            return Err(UncertaintyError::NonFiniteSensitivity { field, value });
        }
    }

    let contributions = NetPowerUncertaintyContributions {
        water_density_w: sensitivity.water_density_w_per_kg_m3 * u_density.value().get(),
        turbine_flow_w: sensitivity.turbine_flow_w_per_m3_per_s * u_flow.value().get(),
        net_head_w: sensitivity.net_head_w_per_m * u_head.value().get(),
        turbine_efficiency_w: sensitivity.turbine_efficiency_w_per_unit
            * u_turbine_efficiency.value().get(),
        generator_efficiency_w: sensitivity.generator_efficiency_w_per_unit
            * u_generator_efficiency.value().get(),
        auxiliary_power_w: sensitivity.auxiliary_power_w_per_w * u_auxiliary.value().get(),
    };

    for (field, value) in [
        ("water_density", contributions.water_density_w),
        ("turbine_flow", contributions.turbine_flow_w),
        ("net_head", contributions.net_head_w),
        ("turbine_efficiency", contributions.turbine_efficiency_w),
        ("generator_efficiency", contributions.generator_efficiency_w),
        ("auxiliary_power", contributions.auxiliary_power_w),
    ] {
        if !value.is_finite() {
            return Err(UncertaintyError::NonFiniteContribution { field, value });
        }
    }

    let variance_w2 = contributions.water_density_w.powi(2)
        + contributions.turbine_flow_w.powi(2)
        + contributions.net_head_w.powi(2)
        + contributions.turbine_efficiency_w.powi(2)
        + contributions.generator_efficiency_w.powi(2)
        + contributions.auxiliary_power_w.powi(2);
    let standard_uncertainty_w = variance_w2.sqrt();

    if !standard_uncertainty_w.is_finite() {
        return Err(UncertaintyError::NonFiniteCombinedUncertainty {
            value: standard_uncertainty_w,
        });
    }

    Ok(Some(NetPowerUncertaintyReport {
        standard_uncertainty_w: StandardUncertainty(standard_uncertainty_w),
        sensitivity,
        contributions,
        dependence: UncertaintyDependence::IndependentStandardUncertaintiesAssumed,
        input_uncertainties: uncertainties.clone(),
        comparison_eligible: result.service_eligible_net_electrical_power().is_some(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Efficiency, EvidenceClass, FlowRateM3PerSecond, GeneratorModel, HeadMetres, HydraulicInput,
        PressurePascals, TurbineMapCell, WaterDensityKgPerM3, WaterServiceContract,
        WaterServiceState,
    };

    fn provenance(
        id: &str,
        method: &str,
        instrument_or_map: &str,
        revision: &str,
    ) -> UncertaintyProvenance {
        UncertaintyProvenance::new(
            EvidenceReference::new(id, EvidenceClass::LaboratoryModelTest).unwrap(),
            method,
            "2026-11-09T12:00:00Z",
            instrument_or_map,
            revision,
            "calibration valid through 2027-11-09",
        )
        .unwrap()
    }

    fn bound_uncertainty(
        value: f64,
        id: &str,
        method: &str,
        instrument_or_map: &str,
    ) -> EvidenceBoundStandardUncertainty {
        EvidenceBoundStandardUncertainty::new(
            StandardUncertainty::new(value).unwrap(),
            provenance(id, method, instrument_or_map, "revision-a"),
        )
    }

    fn complete_uncertainties() -> HydraulicPowerStandardUncertainties {
        HydraulicPowerStandardUncertainties {
            water_density_kg_per_m3: Some(bound_uncertainty(
                0.5,
                "UNCERTAINTY-DENSITY-001",
                "temperature-to-density propagation",
                "WATER-DENSITY-CAL-01",
            )),
            turbine_flow_m3_per_s: Some(bound_uncertainty(
                0.001,
                "UNCERTAINTY-FLOW-001",
                "meter calibration certificate",
                "TURBINE-FLOW-METER-01",
            )),
            net_head_m: Some(bound_uncertainty(
                0.1,
                "UNCERTAINTY-HEAD-001",
                "paired pressure-transducer calibration and head conversion",
                "NET-HEAD-MEASUREMENT-01",
            )),
            turbine_efficiency: Some(bound_uncertainty(
                0.01,
                "UNCERTAINTY-TURBINE-001",
                "laboratory turbine map repeatability",
                "PAT-MAP-REV-A",
            )),
            generator_efficiency: Some(bound_uncertainty(
                0.005,
                "UNCERTAINTY-GENERATOR-001",
                "generator dynamometer test",
                "GENERATOR-MAP-REV-A",
            )),
            auxiliary_power_w: Some(bound_uncertainty(
                2.0,
                "UNCERTAINTY-AUX-001",
                "auxiliary-load power-meter calibration",
                "AUX-METER-01",
            )),
        }
    }

    fn result() -> HydraulicGenerationResult {
        let evidence = |id: &str, class| EvidenceReference::new(id, class).unwrap();
        let hydraulic = HydraulicInput::new_with_flow_allocation(
            evidence("HYDRAULIC-OBS-001", EvidenceClass::FieldMeasurement),
            WaterDensityKgPerM3::new(1_000.0).unwrap(),
            FlowRateM3PerSecond::new(0.15).unwrap(),
            FlowRateM3PerSecond::new(0.1).unwrap(),
            FlowRateM3PerSecond::new(0.05).unwrap(),
            FlowRateM3PerSecond::new(1e-9).unwrap(),
            HeadMetres::new(12.0).unwrap(),
            HeadMetres::new(2.0).unwrap(),
        )
        .unwrap();
        let turbine = TurbineMapCell::new(
            "PAT-UNCERTAINTY-TEST",
            "rev-a",
            "map-cell-test",
            evidence("PAT-LAB-MAP-001", EvidenceClass::LaboratoryModelTest),
            HeadMetres::new(5.0).unwrap(),
            HeadMetres::new(15.0).unwrap(),
            FlowRateM3PerSecond::new(0.05).unwrap(),
            FlowRateM3PerSecond::new(0.2).unwrap(),
            Efficiency::new(0.8).unwrap(),
        )
        .unwrap();
        let generator = GeneratorModel::new(
            "GEN-UNCERTAINTY-TEST",
            "rev-a",
            evidence("GEN-LAB-MAP-001", EvidenceClass::LaboratoryModelTest),
            Efficiency::new(0.9).unwrap(),
            crate::PowerWatts::new(100.0).unwrap(),
        )
        .unwrap();
        let contract = WaterServiceContract::new(
            "WATER-SERVICE-TEST",
            evidence(
                "WATER-CONTRACT-001",
                EvidenceClass::OperatorApprovedContract,
            ),
            PressurePascals::new(400_000.0).unwrap(),
            FlowRateM3PerSecond::new(0.09).unwrap(),
            FlowRateM3PerSecond::new(0.015).unwrap(),
            PressurePascals::new(50_000.0).unwrap(),
            FlowRateM3PerSecond::new(0.005).unwrap(),
            FlowRateM3PerSecond::new(0.001).unwrap(),
        )
        .unwrap();
        let baseline = WaterServiceState::new(
            evidence("WATER-BASELINE-001", EvidenceClass::OperatorTelemetry),
            PressurePascals::new(500_000.0).unwrap(),
            FlowRateM3PerSecond::new(0.1).unwrap(),
            FlowRateM3PerSecond::new(0.02).unwrap(),
        );
        let candidate = WaterServiceState::new(
            evidence("WATER-CANDIDATE-001", EvidenceClass::OperatorTelemetry),
            PressurePascals::new(480_000.0).unwrap(),
            FlowRateM3PerSecond::new(0.099).unwrap(),
            FlowRateM3PerSecond::new(0.02).unwrap(),
        );

        crate::evaluate_hydraulic_generation(
            hydraulic, turbine, generator, contract, baseline, candidate,
        )
        .unwrap()
    }

    #[test]
    fn analytic_uncertainty_known_answer() {
        let report = propagate_net_power_standard_uncertainty(
            &result(),
            &complete_uncertainties(),
        )
        .unwrap()
        .unwrap();

        assert!((report.standard_uncertainty_w.get() - 138.98178437322835).abs() < 1e-9);
        assert!(report.comparison_eligible);
        assert_eq!(
            report.dependence,
            UncertaintyDependence::IndependentStandardUncertaintiesAssumed
        );
    }

    #[test]
    fn sensitivity_matches_finite_difference_for_each_input() {
        let base = result();
        let report = propagate_net_power_standard_uncertainty(
            &base,
            &complete_uncertainties(),
        )
        .unwrap()
        .unwrap();

        let epsilons = [0.01, 1e-6, 1e-5, 1e-6, 1e-6, 1e-3];
        let labels = [
            "water_density",
            "turbine_flow",
            "net_head",
            "turbine_efficiency",
            "generator_efficiency",
            "auxiliary_power",
        ];
        let sensitivities = [
            report.sensitivity.water_density_w_per_kg_m3,
            report.sensitivity.turbine_flow_w_per_m3_per_s,
            report.sensitivity.net_head_w_per_m,
            report.sensitivity.turbine_efficiency_w_per_unit,
            report.sensitivity.generator_efficiency_w_per_unit,
            report.sensitivity.auxiliary_power_w_per_w,
        ];

        let rho = base.hydraulic_input.water_density().get();
        let flow = base.hydraulic_input.flow_rate().get();
        let head = base.hydraulic_input.net_head().get();
        let eta_t = base.turbine_map.efficiency().get();
        let eta_g = base.generator_model.efficiency().get();
        let aux = base.auxiliary_power.get();
        let power = |rho: f64, flow: f64, head: f64, eta_t: f64, eta_g: f64, aux: f64| {
            rho * STANDARD_GRAVITY_M_PER_S2 * flow * head * eta_t * eta_g - aux
        };
        let params = [rho, flow, head, eta_t, eta_g, aux];

        for index in 0..epsilons.len() {
            let epsilon = epsilons[index];
            let mut plus = params;
            let mut minus = params;
            plus[index] += epsilon;
            minus[index] -= epsilon;
            let finite_difference = (power(plus[0], plus[1], plus[2], plus[3], plus[4], plus[5])
                - power(minus[0], minus[1], minus[2], minus[3], minus[4], minus[5]))
                / (2.0 * epsilon);
            let relative_error = (finite_difference - sensitivities[index]).abs()
                / sensitivities[index].abs().max(1e-12);
            assert!(
                relative_error < 1e-6,
                "{} derivative relative error {relative_error}",
                labels[index]
            );
        }
    }

    #[test]
    fn missing_any_required_uncertainty_returns_unknown() {
        let mut uncertainties = complete_uncertainties();
        uncertainties.net_head_m = None;
        let report = propagate_net_power_standard_uncertainty(&result(), &uncertainties)
            .unwrap();
        assert!(report.is_none());
    }

    #[test]
    fn zero_standard_uncertainty_is_valid_but_not_inferred() {
        let mut uncertainties = complete_uncertainties();
        uncertainties.water_density_kg_per_m3 = Some(bound_uncertainty(
            0.0,
            "UNCERTAINTY-DENSITY-ZERO",
            "explicit zero-uncertainty test fixture",
            "DENSITY-ZERO-TEST",
        ));
        uncertainties.turbine_flow_m3_per_s = Some(bound_uncertainty(
            0.0,
            "UNCERTAINTY-FLOW-ZERO",
            "explicit zero-uncertainty test fixture",
            "FLOW-ZERO-TEST",
        ));
        uncertainties.net_head_m = Some(bound_uncertainty(
            0.0,
            "UNCERTAINTY-HEAD-ZERO",
            "explicit zero-uncertainty test fixture",
            "HEAD-ZERO-TEST",
        ));
        uncertainties.turbine_efficiency = Some(bound_uncertainty(
            0.0,
            "UNCERTAINTY-TURBINE-ZERO",
            "explicit zero-uncertainty test fixture",
            "TURBINE-ZERO-TEST",
        ));
        uncertainties.generator_efficiency = Some(bound_uncertainty(
            0.0,
            "UNCERTAINTY-GENERATOR-ZERO",
            "explicit zero-uncertainty test fixture",
            "GENERATOR-ZERO-TEST",
        ));
        uncertainties.auxiliary_power_w = Some(bound_uncertainty(
            0.0,
            "UNCERTAINTY-AUX-ZERO",
            "explicit zero-uncertainty test fixture",
            "AUX-ZERO-TEST",
        ));
        let report = propagate_net_power_standard_uncertainty(&result(), &uncertainties)
            .unwrap()
            .unwrap();
        assert_eq!(report.standard_uncertainty_w.get(), 0.0);
    }

    #[test]
    fn negative_and_non_finite_standard_uncertainties_are_rejected() {
        assert!(StandardUncertainty::new(-0.1).is_err());
        assert!(StandardUncertainty::new(f64::NAN).is_err());
        assert!(StandardUncertainty::new(f64::INFINITY).is_err());
    }

    #[test]
    fn evidence_is_preserved_per_uncertainty_input() {
        let report = propagate_net_power_standard_uncertainty(
            &result(),
            &complete_uncertainties(),
        )
        .unwrap()
        .unwrap();
        let density = report
            .input_uncertainties
            .water_density_kg_per_m3
            .as_ref()
            .unwrap()
            .provenance();
        let turbine = report
            .input_uncertainties
            .turbine_efficiency
            .as_ref()
            .unwrap()
            .provenance();

        assert_eq!(density.evidence().source_id(), "UNCERTAINTY-DENSITY-001");
        assert_eq!(density.instrument_or_map_id(), "WATER-DENSITY-CAL-01");
        assert_eq!(density.method(), "temperature-to-density propagation");
        assert_eq!(turbine.evidence().source_id(), "UNCERTAINTY-TURBINE-001");
        assert_eq!(turbine.revision(), "revision-a");
    }

    #[test]
    fn uncertainty_provenance_requires_method_window_and_revision() {
        let result = UncertaintyProvenance::new(
            EvidenceReference::new("UNCERTAINTY-PROVENANCE-001", EvidenceClass::LaboratoryModelTest).unwrap(),
            " ",
            "2026-11-09",
            "METER-001",
            "revision-a",
            "valid through 2027-11-09",
        );
        assert!(matches!(
            result,
            Err(UncertaintyError::MissingProvenance { field: "method" })
        ));
    }

    #[test]
    fn uncertainty_does_not_override_service_or_evidence_gate() {
        let mut base = result();
        base.evidence_assessment = crate::EvidenceAssessment {
            violations: vec![crate::EvidenceViolation::HydraulicInputNotMeasured],
        };
        let report = propagate_net_power_standard_uncertainty(
            &base,
            &complete_uncertainties(),
        )
        .unwrap()
        .unwrap();
        assert!(!report.comparison_eligible);
    }
}
