// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fuel-batch-bound power-system evaluation.
//!
//! This layer ties a calculated steady-state result to its fuel batch and
//! measured mass-flow evidence. It propagates uncertainty only for chemical
//! input power, assuming independent standard uncertainties for mass flow and
//! LHV. It does not claim uncertainty for engine/generator conversion maps;
//! those require their own evidence-bearing uncertainty models.

use serde::Serialize;

use crate::{
    evaluate, EngineModel, FuelBatch, FuelProperty, GeneratorModel, LowerHeatingValueJPerKg,
    MassFlowKgPerSecond, ModelError, PowerSystemResult, Watts,
};

/// A conversion result that retains the fuel-batch and mass-flow evidence
/// used to calculate chemical input power and net electrical output.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FuelBatchEvaluation {
    fuel_batch: FuelBatch,
    fuel_mass_flow: FuelProperty<MassFlowKgPerSecond>,
    engine_model: EngineModel,
    generator_model: GeneratorModel,
    result: PowerSystemResult,
}

impl FuelBatchEvaluation {
    pub fn fuel_batch(&self) -> &FuelBatch {
        &self.fuel_batch
    }

    pub const fn fuel_mass_flow(&self) -> &FuelProperty<MassFlowKgPerSecond> {
        &self.fuel_mass_flow
    }

    pub const fn engine_model(&self) -> EngineModel {
        self.engine_model
    }

    pub const fn generator_model(&self) -> GeneratorModel {
        self.generator_model
    }

    pub const fn result(&self) -> &PowerSystemResult {
        &self.result
    }

    /// First-order standard uncertainty of fuel chemical input power:
    ///
    /// u(P) = sqrt((LHV * u(m_dot))^2 + (m_dot * u(LHV))^2)
    ///
    /// This assumes independent input uncertainties. If either uncertainty
    /// is unknown, the result is Ok(None). Covariance and conversion-map
    /// uncertainty are not represented by this calculation.
    pub fn chemical_input_power_standard_uncertainty(
        &self,
    ) -> Result<Option<Watts>, ModelError> {
        let flow_uncertainty = self.fuel_mass_flow.standard_uncertainty();
        let lhv_uncertainty = self
            .fuel_batch
            .lower_heating_value()
            .standard_uncertainty();

        let (Some(flow_uncertainty), Some(lhv_uncertainty)) =
            (flow_uncertainty, lhv_uncertainty)
        else {
            return Ok(None);
        };

        let mass_flow = self.fuel_mass_flow.value().get();
        let lower_heating_value: LowerHeatingValueJPerKg =
            *self.fuel_batch.lower_heating_value().value();

        let flow_contribution = lower_heating_value.get() * flow_uncertainty;
        let lhv_contribution = mass_flow * lhv_uncertainty;
        let uncertainty_w = flow_contribution.hypot(lhv_contribution);

        Watts::new(uncertainty_w).map(Some)
    }

    /// Net electrical output divided by fuel chemical input. This is a
    /// deterministic point estimate, not an uncertainty-qualified result.
    pub fn net_electrical_efficiency(&self) -> Option<f64> {
        self.result.net_electrical_efficiency()
    }
}

/// Evaluate one explicitly identified fuel batch using an evidence-bearing
/// mass-flow property. The returned result retains both evidence records.
///
/// The caller must ensure the flow measurement applies to the same batch and
/// time window as the supplied engine/generator operating point.
pub fn evaluate_fuel_batch(
    fuel_batch: &FuelBatch,
    fuel_mass_flow: FuelProperty<MassFlowKgPerSecond>,
    engine_model: EngineModel,
    generator_model: GeneratorModel,
) -> Result<FuelBatchEvaluation, ModelError> {
    let fuel = crate::FuelInput::new(
        *fuel_mass_flow.value(),
        *fuel_batch.lower_heating_value().value(),
    );
    let result = evaluate(fuel, engine_model, generator_model)?;

    Ok(FuelBatchEvaluation {
        fuel_batch: fuel_batch.clone(),
        fuel_mass_flow,
        engine_model,
        generator_model,
        result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Efficiency, EvidenceClass, EvidenceReference, FuelFamily, FuelRecordError};

    fn evidence(id: &str) -> EvidenceReference {
        EvidenceReference::new(
            id,
            format!("urn:luminous:test:{id}"),
            "laboratory_measurement",
            "2026-10-01T12:00:00Z",
            EvidenceClass::LaboratoryMeasurement,
        )
        .unwrap()
    }

    fn fuel_batch() -> FuelBatch {
        let lhv = FuelProperty::new(
            LowerHeatingValueJPerKg::new(43_000_000.0).unwrap(),
            Some(100_000.0),
            evidence("LAB-LHV-001"),
        )
        .unwrap();

        FuelBatch::new(
            "DIESEL-BATCH-001",
            FuelFamily::Diesel,
            "reference grade",
            "2026-10-01T12:00:00Z",
            lhv,
            None,
        )
        .unwrap()
    }

    fn flow(uncertainty: Option<f64>) -> FuelProperty<MassFlowKgPerSecond> {
        FuelProperty::new(
            MassFlowKgPerSecond::new(0.01).unwrap(),
            uncertainty,
            evidence("LAB-FLOW-001"),
        )
        .unwrap()
    }

    fn models() -> (EngineModel, GeneratorModel) {
        (
            EngineModel::new(Efficiency::new(0.45).unwrap()),
            GeneratorModel::new(
                Efficiency::new(0.96).unwrap(),
                Watts::new(5_760.0).unwrap(),
            ),
        )
    }

    #[test]
    fn evaluation_retains_batch_and_flow_provenance() {
        let batch = fuel_batch();
        let (engine, generator) = models();
        let evaluation = evaluate_fuel_batch(&batch, flow(Some(0.0001)), engine, generator)
            .unwrap();

        assert_eq!(evaluation.fuel_batch().batch_id(), "DIESEL-BATCH-001");
        assert_eq!(
            evaluation.fuel_batch().lower_heating_value().evidence().source_id(),
            "LAB-LHV-001"
        );
        assert_eq!(
            evaluation.fuel_mass_flow().evidence().source_id(),
            "LAB-FLOW-001"
        );
        assert!(
            (evaluation.result().generator.net_electrical_power.get() - 180_000.0).abs()
                < 1e-9
        );
    }

    #[test]
    fn chemical_input_uncertainty_uses_independent_first_order_propagation() {
        let batch = fuel_batch();
        let (engine, generator) = models();
        let evaluation = evaluate_fuel_batch(&batch, flow(Some(0.0001)), engine, generator)
            .unwrap();

        let uncertainty = evaluation
            .chemical_input_power_standard_uncertainty()
            .unwrap()
            .unwrap()
            .get();
        let expected = (4_300.0_f64.powi(2) + 1_000.0_f64.powi(2)).sqrt();

        assert!((uncertainty - expected).abs() < 1e-9);
    }

    #[test]
    fn missing_mass_flow_uncertainty_is_not_treated_as_zero() {
        let batch = fuel_batch();
        let (engine, generator) = models();
        let evaluation = evaluate_fuel_batch(&batch, flow(None), engine, generator).unwrap();

        assert_eq!(
            evaluation
                .chemical_input_power_standard_uncertainty()
                .unwrap(),
            None
        );
    }

    #[test]
    fn output_uncertainty_is_not_claimed_from_input_uncertainty_alone() {
        let batch = fuel_batch();
        let (engine, generator) = models();
        let evaluation = evaluate_fuel_batch(&batch, flow(Some(0.0001)), engine, generator)
            .unwrap();

        // This API only returns the chemical-input uncertainty. Engine and
        // generator map uncertainties must be represented before output
        // uncertainty can be reported.
        assert!(evaluation.chemical_input_power_standard_uncertainty().is_ok());
    }

    #[test]
    fn rejects_invalid_fuel_flow_property_uncertainty() {
        let invalid: Result<FuelProperty<MassFlowKgPerSecond>, FuelRecordError> =
            FuelProperty::new(
                MassFlowKgPerSecond::new(0.01).unwrap(),
                Some(-0.01),
                evidence("LAB-FLOW-BAD"),
            );

        assert!(invalid.is_err());
    }
}
