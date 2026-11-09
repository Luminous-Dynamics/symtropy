// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Analytical, steady-state water-to-electric reference accounting.
//!
//! This crate does not solve CFD, water hammer, transient pipe networks,
//! hydrology, turbine aerodynamics, cavitation, structural integrity or
//! protection/control design. It consumes explicit hydraulic observations and
//! one evidence-bounded turbine operating-map cell. No interpolation or
//! extrapolation is performed.
//!
//! Energy is creditable only when an explicit water-service contract passes.
//! A failed service gate retains the calculated candidate output for
//! diagnostics but exposes no service-eligible candidate electrical energy.

use core::fmt;

pub const STANDARD_GRAVITY_M_PER_S2: f64 = 9.80665;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    FieldMeasurement,
    OperatorTelemetry,
    OperatorApprovedContract,
    LaboratoryModelTest,
    ManufacturerGuaranteedMap,
    ManufacturerPublishedMap,
    LiteratureValue,
    ScenarioAssumption,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReference {
    source_id: String,
    evidence_class: EvidenceClass,
}

impl EvidenceReference {
    pub fn new(
        source_id: impl Into<String>,
        evidence_class: EvidenceClass,
    ) -> Result<Self, HydraulicError> {
        let source_id = source_id.into();
        if source_id.trim().is_empty() {
            return Err(HydraulicError::MissingEvidence("source_id"));
        }
        Ok(Self {
            source_id,
            evidence_class,
        })
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub const fn evidence_class(&self) -> EvidenceClass {
        self.evidence_class
    }
}

macro_rules! non_negative_quantity {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $name(f64);

        impl $name {
            pub fn new(value: f64) -> Result<Self, HydraulicError> {
                if !value.is_finite() || value < 0.0 {
                    return Err(HydraulicError::InvalidQuantity {
                        field: $field,
                        value,
                    });
                }
                Ok(Self(value))
            }

            pub const fn get(self) -> f64 {
                self.0
            }
        }
    };
}

non_negative_quantity!(FlowRateM3PerSecond, "flow_rate_m3_per_s");
non_negative_quantity!(HeadMetres, "head_m");
non_negative_quantity!(PressurePascals, "pressure_pa");
non_negative_quantity!(PowerWatts, "power_w");
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DurationSeconds(f64);

impl DurationSeconds {
    pub fn new(value: f64) -> Result<Self, HydraulicError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(HydraulicError::InvalidQuantity {
                field: "duration_s",
                value,
            });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterDensityKgPerM3(f64);

impl WaterDensityKgPerM3 {
    pub fn new(value: f64) -> Result<Self, HydraulicError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(HydraulicError::InvalidQuantity {
                field: "water_density_kg_per_m3",
                value,
            });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Efficiency(f64);

impl Efficiency {
    pub fn new(value: f64) -> Result<Self, HydraulicError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(HydraulicError::InvalidEfficiency { value });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Measured or declared hydraulic conditions at the generator's water boundary.
///
/// Gross head and conveyance loss are kept separate. The model never infers a
/// friction factor, pipe roughness, static pressure, or transient head.
#[derive(Debug, Clone, PartialEq)]
pub struct HydraulicInput {
    pub evidence: EvidenceReference,
    water_density: WaterDensityKgPerM3,
    upstream_flow: FlowRateM3PerSecond,
    flow_rate: FlowRateM3PerSecond,
    bypass_flow: FlowRateM3PerSecond,
    flow_balance_tolerance: FlowRateM3PerSecond,
    gross_head: HeadMetres,
    conveyance_head_loss: HeadMetres,
    net_head: HeadMetres,
}

impl HydraulicInput {
    pub fn new(
        evidence: EvidenceReference,
        water_density: WaterDensityKgPerM3,
        flow_rate: FlowRateM3PerSecond,
        gross_head: HeadMetres,
        conveyance_head_loss: HeadMetres,
    ) -> Result<Self, HydraulicError> {
        Self::new_with_flow_allocation(
            evidence,
            water_density,
            flow_rate,
            flow_rate,
            FlowRateM3PerSecond(0.0),
            FlowRateM3PerSecond(0.0),
            gross_head,
            conveyance_head_loss,
        )
    }

    /// Construct a turbine/bypass split from a measured upstream conduit flow.
    ///
    /// The source flow must equal turbine flow plus bypass flow within the
    /// declared meter/measurement tolerance. This is an allocation check, not
    /// a hydraulic transient or leak-detection model.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_flow_allocation(
        evidence: EvidenceReference,
        water_density: WaterDensityKgPerM3,
        upstream_flow: FlowRateM3PerSecond,
        turbine_flow: FlowRateM3PerSecond,
        bypass_flow: FlowRateM3PerSecond,
        flow_balance_tolerance: FlowRateM3PerSecond,
        gross_head: HeadMetres,
        conveyance_head_loss: HeadMetres,
    ) -> Result<Self, HydraulicError> {
        if conveyance_head_loss.get() > gross_head.get() {
            return Err(HydraulicError::HeadLossExceedsGrossHead {
                gross_head_m: gross_head.get(),
                head_loss_m: conveyance_head_loss.get(),
            });
        }

        let allocated_flow = turbine_flow.get() + bypass_flow.get();
        if !allocated_flow.is_finite() {
            return Err(HydraulicError::FlowAllocationDoesNotClose {
                upstream_flow_m3_per_s: upstream_flow.get(),
                allocated_flow_m3_per_s: allocated_flow,
                tolerance_m3_per_s: flow_balance_tolerance.get(),
            });
        }
        let flow_residual = upstream_flow.get() - allocated_flow;
        if flow_residual.abs() > flow_balance_tolerance.get() {
            return Err(HydraulicError::FlowAllocationDoesNotClose {
                upstream_flow_m3_per_s: upstream_flow.get(),
                allocated_flow_m3_per_s: allocated_flow,
                tolerance_m3_per_s: flow_balance_tolerance.get(),
            });
        }

        let net_head = HeadMetres(gross_head.get() - conveyance_head_loss.get());

        Ok(Self {
            evidence,
            water_density,
            upstream_flow,
            flow_rate: turbine_flow,
            bypass_flow,
            flow_balance_tolerance,
            gross_head,
            conveyance_head_loss,
            net_head,
        })
    }

    pub const fn water_density(&self) -> WaterDensityKgPerM3 {
        self.water_density
    }

    pub const fn upstream_flow(&self) -> FlowRateM3PerSecond {
        self.upstream_flow
    }

    pub const fn flow_rate(&self) -> FlowRateM3PerSecond {
        self.flow_rate
    }

    pub fn flow_balance_residual(&self) -> f64 {
        self.upstream_flow.get() - self.flow_rate.get() - self.bypass_flow.get()
    }

    pub const fn bypass_flow(&self) -> FlowRateM3PerSecond {
        self.bypass_flow
    }

    pub const fn flow_balance_tolerance(&self) -> FlowRateM3PerSecond {
        self.flow_balance_tolerance
    }

    pub const fn gross_head(&self) -> HeadMetres {
        self.gross_head
    }

    pub const fn conveyance_head_loss(&self) -> HeadMetres {
        self.conveyance_head_loss
    }

    pub const fn net_head(&self) -> HeadMetres {
        self.net_head
    }
}

/// A declared rectangular operating-map cell for one turbine/PAT model.
///
/// Efficiency is valid only for the inclusive head/flow rectangle. This v0.1
/// model does not interpolate between points, smooth maps or extrapolate
/// outside the declared domain.
#[derive(Debug, Clone, PartialEq)]
pub struct TurbineMapCell {
    model_id: String,
    revision: String,
    point_id: String,
    evidence: EvidenceReference,
    minimum_net_head: HeadMetres,
    maximum_net_head: HeadMetres,
    minimum_flow: FlowRateM3PerSecond,
    maximum_flow: FlowRateM3PerSecond,
    efficiency: Efficiency,
}

impl TurbineMapCell {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model_id: impl Into<String>,
        revision: impl Into<String>,
        point_id: impl Into<String>,
        evidence: EvidenceReference,
        minimum_net_head: HeadMetres,
        maximum_net_head: HeadMetres,
        minimum_flow: FlowRateM3PerSecond,
        maximum_flow: FlowRateM3PerSecond,
        efficiency: Efficiency,
    ) -> Result<Self, HydraulicError> {
        let model_id = model_id.into();
        let revision = revision.into();
        let point_id = point_id.into();

        for (field, value) in [
            ("turbine_model_id", model_id.as_str()),
            ("turbine_map_revision", revision.as_str()),
            ("turbine_map_point_id", point_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(HydraulicError::MissingEvidence(field));
            }
        }

        if maximum_net_head.get() < minimum_net_head.get() {
            return Err(HydraulicError::InvalidOperatingMapRange(
                "maximum head is below minimum head",
            ));
        }
        if maximum_flow.get() < minimum_flow.get() {
            return Err(HydraulicError::InvalidOperatingMapRange(
                "maximum flow is below minimum flow",
            ));
        }

        Ok(Self {
            model_id,
            revision,
            point_id,
            evidence,
            minimum_net_head,
            maximum_net_head,
            minimum_flow,
            maximum_flow,
            efficiency,
        })
    }

    fn contains(&self, net_head: HeadMetres, flow: FlowRateM3PerSecond) -> bool {
        (self.minimum_net_head.get()..=self.maximum_net_head.get()).contains(&net_head.get())
            && (self.minimum_flow.get()..=self.maximum_flow.get()).contains(&flow.get())
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }

    pub fn point_id(&self) -> &str {
        &self.point_id
    }

    pub fn evidence(&self) -> &EvidenceReference {
        &self.evidence
    }

    pub const fn efficiency(&self) -> Efficiency {
        self.efficiency
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorModel {
    model_id: String,
    revision: String,
    evidence: EvidenceReference,
    efficiency: Efficiency,
    auxiliary_load: PowerWatts,
}

impl GeneratorModel {
    pub fn new(
        model_id: impl Into<String>,
        revision: impl Into<String>,
        evidence: EvidenceReference,
        efficiency: Efficiency,
        auxiliary_load: PowerWatts,
    ) -> Result<Self, HydraulicError> {
        let model_id = model_id.into();
        let revision = revision.into();

        if model_id.trim().is_empty() {
            return Err(HydraulicError::MissingEvidence("generator_model_id"));
        }
        if revision.trim().is_empty() {
            return Err(HydraulicError::MissingEvidence("generator_model_revision"));
        }

        Ok(Self {
            model_id,
            revision,
            evidence,
            efficiency,
            auxiliary_load,
        })
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }

    pub fn evidence(&self) -> &EvidenceReference {
        &self.evidence
    }

    pub const fn efficiency(&self) -> Efficiency {
        self.efficiency
    }

    pub const fn auxiliary_load(&self) -> PowerWatts {
        self.auxiliary_load
    }
}

/// One observed water-service state, separate from generation output.
#[derive(Debug, Clone, PartialEq)]
pub struct WaterServiceState {
    pub evidence: EvidenceReference,
    downstream_pressure: PressurePascals,
    delivered_flow: FlowRateM3PerSecond,
    ecological_flow: FlowRateM3PerSecond,
}

impl WaterServiceState {
    pub fn new(
        evidence: EvidenceReference,
        downstream_pressure: PressurePascals,
        delivered_flow: FlowRateM3PerSecond,
        ecological_flow: FlowRateM3PerSecond,
    ) -> Self {
        Self {
            evidence,
            downstream_pressure,
            delivered_flow,
            ecological_flow,
        }
    }

    pub const fn downstream_pressure(&self) -> PressurePascals {
        self.downstream_pressure
    }

    pub const fn delivered_flow(&self) -> FlowRateM3PerSecond {
        self.delivered_flow
    }

    pub const fn ecological_flow(&self) -> FlowRateM3PerSecond {
        self.ecological_flow
    }
}

/// Operator/authority-approved service limits for comparing baseline and
/// candidate operation. This is not a legal or environmental authorisation.
#[derive(Debug, Clone, PartialEq)]
pub struct WaterServiceContract {
    contract_id: String,
    evidence: EvidenceReference,
    minimum_downstream_pressure: PressurePascals,
    minimum_delivered_flow: FlowRateM3PerSecond,
    minimum_ecological_flow: FlowRateM3PerSecond,
    maximum_pressure_drop: PressurePascals,
    maximum_delivery_flow_reduction: FlowRateM3PerSecond,
    maximum_ecological_flow_reduction: FlowRateM3PerSecond,
}

impl WaterServiceContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        contract_id: impl Into<String>,
        evidence: EvidenceReference,
        minimum_downstream_pressure: PressurePascals,
        minimum_delivered_flow: FlowRateM3PerSecond,
        minimum_ecological_flow: FlowRateM3PerSecond,
        maximum_pressure_drop: PressurePascals,
        maximum_delivery_flow_reduction: FlowRateM3PerSecond,
        maximum_ecological_flow_reduction: FlowRateM3PerSecond,
    ) -> Result<Self, HydraulicError> {
        let contract_id = contract_id.into();
        if contract_id.trim().is_empty() {
            return Err(HydraulicError::MissingEvidence("water_service_contract_id"));
        }
        if evidence.evidence_class() != EvidenceClass::OperatorApprovedContract {
            return Err(HydraulicError::InvalidServiceContractEvidence);
        }

        Ok(Self {
            contract_id,
            evidence,
            minimum_downstream_pressure,
            minimum_delivered_flow,
            minimum_ecological_flow,
            maximum_pressure_drop,
            maximum_delivery_flow_reduction,
            maximum_ecological_flow_reduction,
        })
    }

    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    pub fn evidence(&self) -> &EvidenceReference {
        &self.evidence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceViolation {
    HydraulicInputNotMeasured,
    TurbineMapNotIndependentlyTestedOrGuaranteed,
    GeneratorMapNotIndependentlyTestedOrGuaranteed,
    BaselineWaterServiceNotMeasured,
    CandidateWaterServiceNotMeasured,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceAssessment {
    violations: Vec<EvidenceViolation>,
}

impl EvidenceAssessment {
    pub fn violations(&self) -> &[EvidenceViolation] {
        &self.violations
    }

    pub fn is_eligible_for_comparison(&self) -> bool {
        self.violations.is_empty()
    }
}

fn measured_or_telemetry(class: EvidenceClass) -> bool {
    matches!(
        class,
        EvidenceClass::FieldMeasurement | EvidenceClass::OperatorTelemetry
    )
}

fn independently_tested_or_guaranteed(class: EvidenceClass) -> bool {
    matches!(
        class,
        EvidenceClass::LaboratoryModelTest | EvidenceClass::ManufacturerGuaranteedMap
    )
}

fn assess_evidence(
    hydraulic: &HydraulicInput,
    turbine: &TurbineMapCell,
    generator: &GeneratorModel,
    baseline: &WaterServiceState,
    candidate: &WaterServiceState,
) -> EvidenceAssessment {
    let mut violations = Vec::new();

    if !measured_or_telemetry(hydraulic.evidence.evidence_class()) {
        violations.push(EvidenceViolation::HydraulicInputNotMeasured);
    }
    if !independently_tested_or_guaranteed(turbine.evidence.evidence_class()) {
        violations.push(EvidenceViolation::TurbineMapNotIndependentlyTestedOrGuaranteed);
    }
    if !independently_tested_or_guaranteed(generator.evidence.evidence_class()) {
        violations.push(EvidenceViolation::GeneratorMapNotIndependentlyTestedOrGuaranteed);
    }
    if !measured_or_telemetry(baseline.evidence.evidence_class()) {
        violations.push(EvidenceViolation::BaselineWaterServiceNotMeasured);
    }
    if !measured_or_telemetry(candidate.evidence.evidence_class()) {
        violations.push(EvidenceViolation::CandidateWaterServiceNotMeasured);
    }

    EvidenceAssessment { violations }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceViolation {
    DownstreamPressureBelowMinimum,
    DeliveryFlowBelowMinimum,
    EcologicalFlowBelowMinimum,
    WaterAllocationExceedsUpstreamFlow,
    PressureRegressionExceeded,
    DeliveryFlowRegressionExceeded,
    EcologicalFlowRegressionExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterServiceAssessment {
    violations: Vec<ServiceViolation>,
}

impl WaterServiceAssessment {
    pub fn violations(&self) -> &[ServiceViolation] {
        &self.violations
    }

    pub fn is_feasible(&self) -> bool {
        self.violations.is_empty()
    }
}

fn assess_water_service(
    contract: &WaterServiceContract,
    upstream_flow: FlowRateM3PerSecond,
    baseline: &WaterServiceState,
    candidate: &WaterServiceState,
) -> WaterServiceAssessment {
    let mut violations = Vec::new();

    if candidate.downstream_pressure.get() < contract.minimum_downstream_pressure.get() {
        violations.push(ServiceViolation::DownstreamPressureBelowMinimum);
    }
    if candidate.delivered_flow.get() < contract.minimum_delivered_flow.get() {
        violations.push(ServiceViolation::DeliveryFlowBelowMinimum);
    }
    if candidate.ecological_flow.get() < contract.minimum_ecological_flow.get() {
        violations.push(ServiceViolation::EcologicalFlowBelowMinimum);
    }
    let candidate_service_flow = candidate.delivered_flow.get() + candidate.ecological_flow.get();
    if !candidate_service_flow.is_finite() || candidate_service_flow > upstream_flow.get() {
        violations.push(ServiceViolation::WaterAllocationExceedsUpstreamFlow);
    }

    let pressure_drop =
        (baseline.downstream_pressure.get() - candidate.downstream_pressure.get()).max(0.0);
    if pressure_drop > contract.maximum_pressure_drop.get() {
        violations.push(ServiceViolation::PressureRegressionExceeded);
    }

    let delivery_flow_reduction =
        (baseline.delivered_flow.get() - candidate.delivered_flow.get()).max(0.0);
    if delivery_flow_reduction > contract.maximum_delivery_flow_reduction.get() {
        violations.push(ServiceViolation::DeliveryFlowRegressionExceeded);
    }

    let ecological_flow_reduction =
        (baseline.ecological_flow.get() - candidate.ecological_flow.get()).max(0.0);
    if ecological_flow_reduction > contract.maximum_ecological_flow_reduction.get() {
        violations.push(ServiceViolation::EcologicalFlowRegressionExceeded);
    }

    WaterServiceAssessment { violations }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HydraulicGenerationResult {
    pub hydraulic_input: HydraulicInput,
    pub turbine_map: TurbineMapCell,
    pub generator_model: GeneratorModel,
    pub water_service_contract_id: String,
    pub baseline_water_service: WaterServiceState,
    pub candidate_water_service: WaterServiceState,
    pub water_service: WaterServiceAssessment,
    pub evidence_assessment: EvidenceAssessment,
    pub hydraulic_input_power: PowerWatts,
    pub turbine_shaft_power: PowerWatts,
    pub turbine_loss_power: PowerWatts,
    pub gross_electrical_power: PowerWatts,
    pub generator_loss_power: PowerWatts,
    pub auxiliary_power: PowerWatts,
    pub net_candidate_electrical_power: PowerWatts,
    pub balance_residual_power: f64,
}

impl HydraulicGenerationResult {
    /// Candidate net output is not service-eligible when the water-service gate fails.
    pub fn service_eligible_net_electrical_power(&self) -> Option<PowerWatts> {
        (self.water_service.is_feasible() && self.evidence_assessment.is_eligible_for_comparison())
            .then_some(self.net_candidate_electrical_power)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HydraulicEnergyResult {
    pub duration: DurationSeconds,
    pub hydraulic_input_energy_j: f64,
    pub turbine_loss_energy_j: f64,
    pub generator_loss_energy_j: f64,
    pub auxiliary_energy_j: f64,
    pub candidate_net_electrical_energy_j: f64,
    pub service_eligible_net_electrical_energy_j: Option<f64>,
}

/// Evaluate one steady hydraulic operating point against a single declared
/// turbine-map cell, generator model and water-service contract.
///
/// The operating map must contain the exact net head and flow. This function
/// does not interpolate/extrapolate maps or validate transient hydraulics.
pub fn evaluate_hydraulic_generation(
    hydraulic: HydraulicInput,
    turbine: TurbineMapCell,
    generator: GeneratorModel,
    service_contract: WaterServiceContract,
    baseline_service: WaterServiceState,
    candidate_service: WaterServiceState,
) -> Result<HydraulicGenerationResult, HydraulicError> {
    let net_head = hydraulic.net_head();
    let flow = hydraulic.flow_rate();

    if net_head.get() > 0.0 && flow.get() > 0.0 && !turbine.contains(net_head, flow) {
        return Err(HydraulicError::TurbineMapOutOfDomain {
            net_head_m: net_head.get(),
            flow_m3_per_s: flow.get(),
        });
    }

    let hydraulic_input_w =
        hydraulic.water_density().get() * STANDARD_GRAVITY_M_PER_S2 * flow.get() * net_head.get();
    let hydraulic_input_power = PowerWatts::new(hydraulic_input_w)?;

    let turbine_shaft_w = hydraulic_input_power.get() * turbine.efficiency().get();
    let turbine_loss_w = hydraulic_input_power.get() - turbine_shaft_w;

    let gross_electrical_w = turbine_shaft_w * generator.efficiency().get();
    let generator_loss_w = turbine_shaft_w - gross_electrical_w;

    if generator.auxiliary_load().get() > gross_electrical_w {
        return Err(HydraulicError::AuxiliaryLoadExceedsGrossOutput {
            auxiliary_w: generator.auxiliary_load().get(),
            gross_electrical_w,
        });
    }

    let net_electrical_w = gross_electrical_w - generator.auxiliary_load().get();
    let net_candidate_electrical_power = PowerWatts::new(net_electrical_w)?;

    let accounted_power_w =
        turbine_loss_w + generator_loss_w + generator.auxiliary_load().get() + net_electrical_w;
    let balance_residual_power = hydraulic_input_w - accounted_power_w;

    let water_service = assess_water_service(
        &service_contract,
        hydraulic.upstream_flow(),
        &baseline_service,
        &candidate_service,
    );
    let evidence_assessment = assess_evidence(
        &hydraulic,
        &turbine,
        &generator,
        &baseline_service,
        &candidate_service,
    );

    Ok(HydraulicGenerationResult {
        hydraulic_input: hydraulic,
        turbine_map: turbine,
        generator_model: generator.clone(),
        water_service_contract_id: service_contract.contract_id().to_owned(),
        baseline_water_service: baseline_service,
        candidate_water_service: candidate_service,
        water_service,
        evidence_assessment,
        hydraulic_input_power,
        turbine_shaft_power: PowerWatts::new(turbine_shaft_w)?,
        turbine_loss_power: PowerWatts::new(turbine_loss_w)?,
        gross_electrical_power: PowerWatts::new(gross_electrical_w)?,
        generator_loss_power: PowerWatts::new(generator_loss_w)?,
        auxiliary_power: generator.auxiliary_load(),
        net_candidate_electrical_power,
        balance_residual_power,
    })
}

/// Integrate one constant operating point over an explicit positive duration.
/// Energy is credited only if the paired water-service assessment passed.
pub fn integrate_constant_interval(
    result: &HydraulicGenerationResult,
    duration: DurationSeconds,
) -> Result<HydraulicEnergyResult, HydraulicError> {
    let seconds = duration.get();
    let hydraulic_input_energy_j = result.hydraulic_input_power.get() * seconds;
    let turbine_loss_energy_j = result.turbine_loss_power.get() * seconds;
    let generator_loss_energy_j = result.generator_loss_power.get() * seconds;
    let auxiliary_energy_j = result.auxiliary_power.get() * seconds;
    let candidate_net_electrical_energy_j = result.net_candidate_electrical_power.get() * seconds;

    for (field, value) in [
        ("hydraulic_input_energy_j", hydraulic_input_energy_j),
        ("turbine_loss_energy_j", turbine_loss_energy_j),
        ("generator_loss_energy_j", generator_loss_energy_j),
        ("auxiliary_energy_j", auxiliary_energy_j),
        (
            "candidate_net_electrical_energy_j",
            candidate_net_electrical_energy_j,
        ),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(HydraulicError::InvalidQuantity { field, value });
        }
    }

    Ok(HydraulicEnergyResult {
        duration,
        hydraulic_input_energy_j,
        turbine_loss_energy_j,
        generator_loss_energy_j,
        auxiliary_energy_j,
        candidate_net_electrical_energy_j,
        service_eligible_net_electrical_energy_j: result
            .water_service
            .is_feasible()
            .then_some(candidate_net_electrical_energy_j),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum HydraulicError {
    InvalidQuantity {
        field: &'static str,
        value: f64,
    },
    InvalidEfficiency {
        value: f64,
    },
    MissingEvidence(&'static str),
    HeadLossExceedsGrossHead {
        gross_head_m: f64,
        head_loss_m: f64,
    },
    FlowAllocationDoesNotClose {
        upstream_flow_m3_per_s: f64,
        allocated_flow_m3_per_s: f64,
        tolerance_m3_per_s: f64,
    },
    InvalidOperatingMapRange(&'static str),
    InvalidServiceContractEvidence,
    TurbineMapOutOfDomain {
        net_head_m: f64,
        flow_m3_per_s: f64,
    },
    AuxiliaryLoadExceedsGrossOutput {
        auxiliary_w: f64,
        gross_electrical_w: f64,
    },
}

impl fmt::Display for HydraulicError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuantity { field, value } => {
                write!(
                    formatter,
                    "{field} must be finite and within its declared physical domain; got {value}"
                )
            }
            Self::InvalidEfficiency { value } => {
                write!(
                    formatter,
                    "efficiency must be finite and in [0, 1]; got {value}"
                )
            }
            Self::MissingEvidence(field) => {
                write!(formatter, "{field} must be present and non-empty")
            }
            Self::HeadLossExceedsGrossHead {
                gross_head_m,
                head_loss_m,
            } => write!(
                formatter,
                "conveyance head loss ({head_loss_m} m) exceeds gross head ({gross_head_m} m)"
            ),
            Self::FlowAllocationDoesNotClose {
                upstream_flow_m3_per_s,
                allocated_flow_m3_per_s,
                tolerance_m3_per_s,
            } => write!(
                formatter,
                "upstream flow ({upstream_flow_m3_per_s} m3/s) does not equal turbine plus bypass allocation ({allocated_flow_m3_per_s} m3/s) within tolerance ({tolerance_m3_per_s} m3/s)"
            ),
            Self::InvalidOperatingMapRange(message) => {
                write!(formatter, "invalid turbine operating-map range: {message}")
            }
            Self::InvalidServiceContractEvidence => {
                write!(
                    formatter,
                    "water-service limits require operator-approved contract evidence"
                )
            }
            Self::TurbineMapOutOfDomain {
                net_head_m,
                flow_m3_per_s,
            } => write!(
                formatter,
                "operating point lies outside turbine map: net head={net_head_m} m, flow={flow_m3_per_s} m3/s"
            ),
            Self::AuxiliaryLoadExceedsGrossOutput {
                auxiliary_w,
                gross_electrical_w,
            } => write!(
                formatter,
                "auxiliary load ({auxiliary_w} W) exceeds gross electrical output ({gross_electrical_w} W)"
            ),
        }
    }
}

impl std::error::Error for HydraulicError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(id: &str, class: EvidenceClass) -> EvidenceReference {
        EvidenceReference::new(id, class).expect("evidence reference")
    }

    fn hydraulic_input(flow: f64, gross_head: f64, head_loss: f64) -> HydraulicInput {
        HydraulicInput::new_with_flow_allocation(
            evidence("FIELD-HYDRO-001", EvidenceClass::FieldMeasurement),
            WaterDensityKgPerM3::new(1_000.0).expect("density"),
            FlowRateM3PerSecond::new(flow + 0.05).expect("upstream flow"),
            FlowRateM3PerSecond::new(flow).expect("turbine flow"),
            FlowRateM3PerSecond::new(0.05).expect("bypass flow"),
            FlowRateM3PerSecond::new(1e-9).expect("flow balance tolerance"),
            HeadMetres::new(gross_head).expect("gross head"),
            HeadMetres::new(head_loss).expect("head loss"),
        )
        .expect("hydraulic input")
    }

    fn turbine() -> TurbineMapCell {
        TurbineMapCell::new(
            "PAT-REFERENCE-001",
            "rev-a",
            "map-cell-10m-0.1m3s",
            evidence("PAT-MAP-001", EvidenceClass::LaboratoryModelTest),
            HeadMetres::new(5.0).expect("minimum head"),
            HeadMetres::new(15.0).expect("maximum head"),
            FlowRateM3PerSecond::new(0.05).expect("minimum flow"),
            FlowRateM3PerSecond::new(0.2).expect("maximum flow"),
            Efficiency::new(0.8).expect("turbine efficiency"),
        )
        .expect("turbine map")
    }

    fn generator(auxiliary_w: f64) -> GeneratorModel {
        GeneratorModel::new(
            "GEN-REFERENCE-001",
            "rev-a",
            evidence("GEN-MAP-001", EvidenceClass::LaboratoryModelTest),
            Efficiency::new(0.9).expect("generator efficiency"),
            PowerWatts::new(auxiliary_w).expect("auxiliary load"),
        )
        .expect("generator model")
    }

    fn service_contract() -> WaterServiceContract {
        WaterServiceContract::new(
            "WATER-SERVICE-001",
            evidence(
                "WATER-SERVICE-CONTRACT-EVIDENCE-001",
                EvidenceClass::OperatorApprovedContract,
            ),
            PressurePascals::new(400_000.0).expect("minimum pressure"),
            FlowRateM3PerSecond::new(0.09).expect("minimum delivery flow"),
            FlowRateM3PerSecond::new(0.015).expect("minimum ecological flow"),
            PressurePascals::new(50_000.0).expect("maximum pressure drop"),
            FlowRateM3PerSecond::new(0.005).expect("maximum delivery flow reduction"),
            FlowRateM3PerSecond::new(0.001).expect("maximum ecological flow reduction"),
        )
        .expect("service contract")
    }

    fn service_state(
        source_id: &str,
        pressure: f64,
        delivery_flow: f64,
        ecological_flow: f64,
    ) -> WaterServiceState {
        WaterServiceState::new(
            evidence(source_id, EvidenceClass::OperatorTelemetry),
            PressurePascals::new(pressure).expect("pressure"),
            FlowRateM3PerSecond::new(delivery_flow).expect("delivery flow"),
            FlowRateM3PerSecond::new(ecological_flow).expect("ecological flow"),
        )
    }

    fn baseline_service() -> WaterServiceState {
        service_state("WATER-BASELINE-001", 500_000.0, 0.1, 0.02)
    }

    fn feasible_candidate_service() -> WaterServiceState {
        service_state("WATER-CANDIDATE-001", 480_000.0, 0.099, 0.02)
    }

    fn evaluate_default() -> HydraulicGenerationResult {
        evaluate_hydraulic_generation(
            hydraulic_input(0.1, 12.0, 2.0),
            turbine(),
            generator(100.0),
            service_contract(),
            baseline_service(),
            feasible_candidate_service(),
        )
        .expect("hydraulic generation")
    }

    #[test]
    fn analytic_hydraulic_to_net_electric_known_answer() {
        let result = evaluate_default();

        assert!((result.hydraulic_input_power.get() - 9_806.65).abs() < 1e-9);
        assert!((result.turbine_shaft_power.get() - 7_845.32).abs() < 1e-9);
        assert!((result.gross_electrical_power.get() - 7_060.788).abs() < 1e-9);
        assert!((result.generator_loss_power.get() - 784.532).abs() < 1e-9);
        assert!((result.net_candidate_electrical_power.get() - 6_960.788).abs() < 1e-9);
        assert!(result.water_service.is_feasible());
        assert!(
            (result
                .service_eligible_net_electrical_power()
                .unwrap()
                .get()
                - 6_960.788)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn hydraulic_energy_balance_closes() {
        let result = evaluate_default();
        assert!(result.balance_residual_power.abs() < 1e-9);

        let accounted = result.turbine_loss_power.get()
            + result.generator_loss_power.get()
            + result.auxiliary_power.get()
            + result.net_candidate_electrical_power.get();
        assert!((accounted - result.hydraulic_input_power.get()).abs() < 1e-9);
    }

    #[test]
    fn zero_turbine_flow_can_preserve_service_through_the_bypass_without_generation() {
        let bypass_only = HydraulicInput::new_with_flow_allocation(
            evidence("FIELD-BYPASS-ONLY-001", EvidenceClass::FieldMeasurement),
            WaterDensityKgPerM3::new(1_000.0).unwrap(),
            FlowRateM3PerSecond::new(0.15).unwrap(),
            FlowRateM3PerSecond::new(0.0).unwrap(),
            FlowRateM3PerSecond::new(0.15).unwrap(),
            FlowRateM3PerSecond::new(1e-9).unwrap(),
            HeadMetres::new(12.0).unwrap(),
            HeadMetres::new(2.0).unwrap(),
        )
        .expect("bypass carries all available water");

        let result = evaluate_hydraulic_generation(
            bypass_only,
            turbine(),
            generator(0.0),
            service_contract(),
            baseline_service(),
            feasible_candidate_service(),
        )
        .expect("bypass-only operation is a valid no-generation state");

        assert_eq!(result.hydraulic_input_power.get(), 0.0);
        assert_eq!(result.net_candidate_electrical_power.get(), 0.0);
        assert!(result.water_service.is_feasible());
        assert_eq!(
            result
                .service_eligible_net_electrical_power()
                .unwrap()
                .get(),
            0.0
        );
    }

    #[test]
    fn zero_net_head_produces_zero_power() {
        let result = evaluate_hydraulic_generation(
            hydraulic_input(0.1, 2.0, 2.0),
            turbine(),
            generator(0.0),
            service_contract(),
            baseline_service(),
            feasible_candidate_service(),
        )
        .expect("zero net head is a valid no-generation state");

        assert_eq!(result.hydraulic_input_power.get(), 0.0);
        assert_eq!(result.net_candidate_electrical_power.get(), 0.0);
    }

    #[test]
    fn head_loss_cannot_exceed_gross_head() {
        let input = HydraulicInput::new(
            evidence("FIELD-HYDRO-001", EvidenceClass::FieldMeasurement),
            WaterDensityKgPerM3::new(1_000.0).unwrap(),
            FlowRateM3PerSecond::new(0.1).unwrap(),
            HeadMetres::new(5.0).unwrap(),
            HeadMetres::new(5.1).unwrap(),
        );
        assert!(matches!(
            input,
            Err(HydraulicError::HeadLossExceedsGrossHead { .. })
        ));
    }

    #[test]
    fn turbine_and_bypass_flow_must_close_to_upstream_measurement() {
        let result = HydraulicInput::new_with_flow_allocation(
            evidence("FLOW-ALLOCATION-001", EvidenceClass::OperatorTelemetry),
            WaterDensityKgPerM3::new(1_000.0).unwrap(),
            FlowRateM3PerSecond::new(0.15).unwrap(),
            FlowRateM3PerSecond::new(0.10).unwrap(),
            FlowRateM3PerSecond::new(0.04).unwrap(),
            FlowRateM3PerSecond::new(0.001).unwrap(),
            HeadMetres::new(12.0).unwrap(),
            HeadMetres::new(2.0).unwrap(),
        );
        assert!(matches!(
            result,
            Err(HydraulicError::FlowAllocationDoesNotClose { .. })
        ));
    }

    #[test]
    fn water_service_flow_cannot_exceed_upstream_source_flow() {
        let candidate = service_state("WATER-CANDIDATE-OVERALLOCATED", 480_000.0, 0.14, 0.02);
        let result = evaluate_hydraulic_generation(
            hydraulic_input(0.1, 12.0, 2.0),
            turbine(),
            generator(100.0),
            service_contract(),
            baseline_service(),
            candidate,
        )
        .expect("candidate remains available for diagnostic assessment");
        assert!(
            result
                .water_service
                .violations()
                .contains(&ServiceViolation::WaterAllocationExceedsUpstreamFlow)
        );
        assert!(result.service_eligible_net_electrical_power().is_none());
    }

    #[test]
    fn turbine_map_refuses_extrapolation() {
        let result = evaluate_hydraulic_generation(
            hydraulic_input(0.3, 12.0, 2.0),
            turbine(),
            generator(0.0),
            service_contract(),
            baseline_service(),
            feasible_candidate_service(),
        );
        assert!(matches!(
            result,
            Err(HydraulicError::TurbineMapOutOfDomain { .. })
        ));
    }

    #[test]
    fn auxiliary_load_cannot_exceed_gross_electrical_output() {
        let result = evaluate_hydraulic_generation(
            hydraulic_input(0.1, 12.0, 2.0),
            turbine(),
            generator(8_000.0),
            service_contract(),
            baseline_service(),
            feasible_candidate_service(),
        );
        assert!(matches!(
            result,
            Err(HydraulicError::AuxiliaryLoadExceedsGrossOutput { .. })
        ));
    }

    #[test]
    fn water_service_floor_failure_prevents_energy_credit() {
        let candidate = service_state("WATER-CANDIDATE-FAIL", 390_000.0, 0.08, 0.01);
        let result = evaluate_hydraulic_generation(
            hydraulic_input(0.1, 12.0, 2.0),
            turbine(),
            generator(100.0),
            service_contract(),
            baseline_service(),
            candidate,
        )
        .expect("candidate output retained for diagnostics");

        assert!(!result.water_service.is_feasible());
        assert!(result.service_eligible_net_electrical_power().is_none());
        assert!(
            result
                .water_service
                .violations()
                .contains(&ServiceViolation::DownstreamPressureBelowMinimum)
        );
        assert!(
            result
                .water_service
                .violations()
                .contains(&ServiceViolation::DeliveryFlowBelowMinimum)
        );
        assert!(
            result
                .water_service
                .violations()
                .contains(&ServiceViolation::EcologicalFlowBelowMinimum)
        );
    }

    #[test]
    fn regression_limit_is_hard_even_when_service_floor_passes() {
        let candidate = service_state("WATER-CANDIDATE-REGRESSION", 430_000.0, 0.094, 0.018);
        let result = evaluate_hydraulic_generation(
            hydraulic_input(0.1, 12.0, 2.0),
            turbine(),
            generator(100.0),
            service_contract(),
            baseline_service(),
            candidate,
        )
        .expect("candidate output retained for diagnostics");

        assert!(!result.water_service.is_feasible());
        assert!(
            result
                .water_service
                .violations()
                .contains(&ServiceViolation::PressureRegressionExceeded)
        );
        assert!(
            result
                .water_service
                .violations()
                .contains(&ServiceViolation::DeliveryFlowRegressionExceeded)
        );
        assert!(
            result
                .water_service
                .violations()
                .contains(&ServiceViolation::EcologicalFlowRegressionExceeded)
        );
        assert!(result.service_eligible_net_electrical_power().is_none());
    }

    #[test]
    fn scenario_hydraulic_input_cannot_receive_comparison_credit() {
        let hydraulic = HydraulicInput::new_with_flow_allocation(
            evidence("SYNTHETIC-HYDRO-001", EvidenceClass::ScenarioAssumption),
            WaterDensityKgPerM3::new(1_000.0).unwrap(),
            FlowRateM3PerSecond::new(0.15).unwrap(),
            FlowRateM3PerSecond::new(0.1).unwrap(),
            FlowRateM3PerSecond::new(0.05).unwrap(),
            FlowRateM3PerSecond::new(1e-9).unwrap(),
            HeadMetres::new(12.0).unwrap(),
            HeadMetres::new(2.0).unwrap(),
        )
        .unwrap();
        let result = evaluate_hydraulic_generation(
            hydraulic,
            turbine(),
            generator(100.0),
            service_contract(),
            baseline_service(),
            feasible_candidate_service(),
        )
        .unwrap();

        assert!(result.water_service.is_feasible());
        assert!(!result.evidence_assessment.is_eligible_for_comparison());
        assert!(
            result
                .evidence_assessment
                .violations()
                .contains(&EvidenceViolation::HydraulicInputNotMeasured)
        );
        assert!(result.service_eligible_net_electrical_power().is_none());
    }

    #[test]
    fn published_vendor_map_alone_does_not_qualify_comparison_credit() {
        let turbine = TurbineMapCell::new(
            "PAT-REFERENCE-001",
            "rev-a",
            "map-cell-10m-0.1m3s",
            evidence(
                "PAT-MAP-PUBLISHED-001",
                EvidenceClass::ManufacturerPublishedMap,
            ),
            HeadMetres::new(5.0).unwrap(),
            HeadMetres::new(15.0).unwrap(),
            FlowRateM3PerSecond::new(0.05).unwrap(),
            FlowRateM3PerSecond::new(0.2).unwrap(),
            Efficiency::new(0.8).unwrap(),
        )
        .unwrap();
        let result = evaluate_hydraulic_generation(
            hydraulic_input(0.1, 12.0, 2.0),
            turbine,
            generator(100.0),
            service_contract(),
            baseline_service(),
            feasible_candidate_service(),
        )
        .unwrap();

        assert!(result.water_service.is_feasible());
        assert!(!result.evidence_assessment.is_eligible_for_comparison());
        assert!(
            result
                .evidence_assessment
                .violations()
                .contains(&EvidenceViolation::TurbineMapNotIndependentlyTestedOrGuaranteed)
        );
        assert!(result.service_eligible_net_electrical_power().is_none());
    }

    #[test]
    fn service_floor_pass_allows_credited_output() {
        let result = evaluate_default();
        assert!(result.water_service.is_feasible());
        assert!(result.evidence_assessment.is_eligible_for_comparison());
        assert!(result.service_eligible_net_electrical_power().is_some());
    }

    #[test]
    fn interval_energy_uses_explicit_duration() {
        let result = evaluate_default();
        let interval =
            integrate_constant_interval(&result, DurationSeconds::new(3_600.0).expect("duration"))
                .expect("interval energy");

        assert_eq!(interval.duration.get(), 3_600.0);
        assert!(
            (interval.service_eligible_net_electrical_energy_j.unwrap() - 6_960.788 * 3_600.0)
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn failed_water_service_returns_no_credited_interval_energy() {
        let candidate = service_state("WATER-CANDIDATE-FAIL", 390_000.0, 0.08, 0.01);
        let result = evaluate_hydraulic_generation(
            hydraulic_input(0.1, 12.0, 2.0),
            turbine(),
            generator(100.0),
            service_contract(),
            baseline_service(),
            candidate,
        )
        .unwrap();
        let interval =
            integrate_constant_interval(&result, DurationSeconds::new(900.0).unwrap()).unwrap();

        assert!(interval.service_eligible_net_electrical_energy_j.is_none());
        assert!(interval.candidate_net_electrical_energy_j > 0.0);
    }

    #[test]
    fn non_finite_and_negative_physical_inputs_are_rejected() {
        assert!(WaterDensityKgPerM3::new(f64::NAN).is_err());
        assert!(WaterDensityKgPerM3::new(0.0).is_err());
        assert!(FlowRateM3PerSecond::new(-0.1).is_err());
        assert!(HeadMetres::new(f64::INFINITY).is_err());
        assert!(PressurePascals::new(-1.0).is_err());
        assert!(PowerWatts::new(f64::NAN).is_err());
        assert!(DurationSeconds::new(0.0).is_err());
    }

    #[test]
    fn efficiencies_are_bounded() {
        assert!(Efficiency::new(-0.01).is_err());
        assert!(Efficiency::new(1.01).is_err());
        assert!(Efficiency::new(f64::NAN).is_err());
    }

    #[test]
    fn evidence_and_model_identifiers_are_required() {
        assert!(EvidenceReference::new(" ", EvidenceClass::FieldMeasurement).is_err());
        assert!(
            TurbineMapCell::new(
                "",
                "rev-a",
                "point",
                evidence("map", EvidenceClass::LaboratoryModelTest),
                HeadMetres::new(1.0).unwrap(),
                HeadMetres::new(2.0).unwrap(),
                FlowRateM3PerSecond::new(0.1).unwrap(),
                FlowRateM3PerSecond::new(0.2).unwrap(),
                Efficiency::new(0.8).unwrap(),
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_turbine_map_ranges_are_rejected() {
        let result = TurbineMapCell::new(
            "PAT-1",
            "rev-a",
            "point-1",
            evidence("map", EvidenceClass::LaboratoryModelTest),
            HeadMetres::new(3.0).unwrap(),
            HeadMetres::new(2.0).unwrap(),
            FlowRateM3PerSecond::new(0.1).unwrap(),
            FlowRateM3PerSecond::new(0.2).unwrap(),
            Efficiency::new(0.8).unwrap(),
        );
        assert!(matches!(
            result,
            Err(HydraulicError::InvalidOperatingMapRange(_))
        ));
    }

    #[test]
    fn water_service_limits_require_operator_approved_evidence() {
        let result = WaterServiceContract::new(
            "WATER-SERVICE-UNAPPROVED",
            evidence("SCENARIO-CONTRACT", EvidenceClass::ScenarioAssumption),
            PressurePascals::new(1.0).unwrap(),
            FlowRateM3PerSecond::new(0.0).unwrap(),
            FlowRateM3PerSecond::new(0.0).unwrap(),
            PressurePascals::new(0.0).unwrap(),
            FlowRateM3PerSecond::new(0.0).unwrap(),
            FlowRateM3PerSecond::new(0.0).unwrap(),
        );
        assert!(matches!(
            result,
            Err(HydraulicError::InvalidServiceContractEvidence)
        ));
    }

    #[test]
    fn water_service_contract_requires_identity() {
        let contract = WaterServiceContract::new(
            " ",
            evidence("CONTRACT-EVIDENCE", EvidenceClass::OperatorApprovedContract),
            PressurePascals::new(1.0).unwrap(),
            FlowRateM3PerSecond::new(0.0).unwrap(),
            FlowRateM3PerSecond::new(0.0).unwrap(),
            PressurePascals::new(0.0).unwrap(),
            FlowRateM3PerSecond::new(0.0).unwrap(),
            FlowRateM3PerSecond::new(0.0).unwrap(),
        );
        assert!(contract.is_err());
    }
}
