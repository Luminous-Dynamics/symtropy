//! Fail-closed accounting primitives for the Plasma Food Benchmark (PFB-1).
//!
//! This crate calculates electricity intensity from explicit process-stage inputs.
//! Unknown stages never become zero, and a complete arithmetic result is not
//! automatically an evidence-qualified performance claim.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Boundary {
    Reactor,
    ProcessorGate,
    Household,
    GateToPlate,
}

impl Boundary {
    pub fn required_stages(self) -> &'static [EnergyStage] {
        use EnergyStage::*;
        match self {
            Boundary::Reactor => &[
                PlasmaGeneration,
                Vacuum,
                GasDelivery,
                Cooling,
                ExhaustTreatment,
                ControlsAndStandby,
            ],
            Boundary::ProcessorGate => &[
                PlasmaGeneration,
                Vacuum,
                GasDelivery,
                Cooling,
                ExhaustTreatment,
                ControlsAndStandby,
                IngredientHandlingAndDrying,
                CleaningAndQualityControl,
                Packaging,
            ],
            Boundary::Household => &[HouseholdPreparation, HouseholdCookingAndHolding],
            Boundary::GateToPlate => &[
                PlasmaGeneration,
                Vacuum,
                GasDelivery,
                Cooling,
                ExhaustTreatment,
                ControlsAndStandby,
                IngredientHandlingAndDrying,
                CleaningAndQualityControl,
                Packaging,
                DistributionAndStorage,
                HouseholdPreparation,
                HouseholdCookingAndHolding,
            ],
        }
    }

    fn allows(self, stage: EnergyStage) -> bool {
        self.required_stages().contains(&stage)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EnergyStage {
    PlasmaGeneration,
    Vacuum,
    GasDelivery,
    Cooling,
    ExhaustTreatment,
    ControlsAndStandby,
    IngredientHandlingAndDrying,
    CleaningAndQualityControl,
    Packaging,
    DistributionAndStorage,
    HouseholdPreparation,
    HouseholdCookingAndHolding,
}

impl EnergyStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            EnergyStage::PlasmaGeneration => "plasma_generation",
            EnergyStage::Vacuum => "vacuum",
            EnergyStage::GasDelivery => "gas_delivery",
            EnergyStage::Cooling => "cooling",
            EnergyStage::ExhaustTreatment => "exhaust_treatment",
            EnergyStage::ControlsAndStandby => "controls_and_standby",
            EnergyStage::IngredientHandlingAndDrying => "ingredient_handling_and_drying",
            EnergyStage::CleaningAndQualityControl => "cleaning_and_quality_control",
            EnergyStage::Packaging => "packaging",
            EnergyStage::DistributionAndStorage => "distribution_and_storage",
            EnergyStage::HouseholdPreparation => "household_preparation",
            EnergyStage::HouseholdCookingAndHolding => "household_cooking_and_holding",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    Measured,
    Allocated,
    Attested,
    Modelled,
    Simulated,
    Declared,
}

impl EvidenceClass {
    fn permits_performance_claim(self) -> bool {
        matches!(
            self,
            EvidenceClass::Measured | EvidenceClass::Allocated | EvidenceClass::Attested
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Applicability {
    Required,
    NotApplicable { justification: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnergyTerm {
    pub stage: EnergyStage,
    pub applicability: Applicability,
    /// Electrical wall-plug energy in kWh for this stage.
    pub wall_plug_kwh: Option<f64>,
    pub evidence_class: Option<EvidenceClass>,
    /// Stable source, meter, allocation or evidence-record reference.
    pub source_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnergyInput {
    pub trial_id: String,
    pub product_key: String,
    pub endpoint_id: String,
    pub comparison_group_id: String,
    pub boundary_ref: String,
    pub boundary: Boundary,
    pub accepted_edible_kg: f64,
    pub serving_mass_kg: f64,
    pub terms: Vec<EnergyTerm>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OracleError {
    EmptyMetadata(&'static str),
    InvalidPositiveNumber(&'static str),
    InvalidNonNegativeNumber(&'static str),
    DuplicateStage(EnergyStage),
    UnexpectedStage { boundary: Boundary, stage: EnergyStage },
    InvalidNotApplicable(EnergyStage),
    IncompatibleComparison(&'static str),
    ZeroControlEnergy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnergyReceipt {
    pub trial_id: String,
    pub product_key: String,
    pub endpoint_id: String,
    pub comparison_group_id: String,
    pub boundary_ref: String,
    pub boundary: Boundary,
    pub accepted_edible_kg: f64,
    pub serving_mass_kg: f64,
    pub wall_plug_kwh: Option<f64>,
    pub wall_plug_kwh_per_accepted_kg: Option<f64>,
    pub wall_plug_kwh_per_serving: Option<f64>,
    pub missing_stages: Vec<EnergyStage>,
    pub evidence_classes: BTreeMap<EnergyStage, EvidenceClass>,
    pub complete: bool,
    /// True only when every included numeric energy value is measured,
    /// explicitly allocated, or attested and every required stage is complete.
    pub claim_eligible: bool,
}

pub fn calculate_energy(input: &EnergyInput) -> Result<EnergyReceipt, OracleError> {
    validate_non_empty("trial_id", &input.trial_id)?;
    validate_non_empty("product_key", &input.product_key)?;
    validate_non_empty("endpoint_id", &input.endpoint_id)?;
    validate_non_empty("comparison_group_id", &input.comparison_group_id)?;
    validate_non_empty("boundary_ref", &input.boundary_ref)?;
    validate_positive("accepted_edible_kg", input.accepted_edible_kg)?;
    validate_positive("serving_mass_kg", input.serving_mass_kg)?;

    let required: BTreeSet<_> = input.boundary.required_stages().iter().copied().collect();
    let mut provided = BTreeMap::<EnergyStage, &EnergyTerm>::new();

    for term in &input.terms {
        if !input.boundary.allows(term.stage) {
            return Err(OracleError::UnexpectedStage {
                boundary: input.boundary,
                stage: term.stage,
            });
        }
        if provided.insert(term.stage, term).is_some() {
            return Err(OracleError::DuplicateStage(term.stage));
        }
        if let Applicability::NotApplicable { justification } = &term.applicability {
            if justification.trim().is_empty()
                || term.wall_plug_kwh.is_some()
                || term.evidence_class.is_some()
                || term.source_ref.is_some()
            {
                return Err(OracleError::InvalidNotApplicable(term.stage));
            }
        }
    }

    let mut missing = Vec::new();
    let mut total_kwh = 0.0_f64;
    let mut evidence_classes = BTreeMap::new();
    let mut all_claimable = true;

    for stage in required {
        let Some(term) = provided.get(&stage).copied() else {
            missing.push(stage);
            continue;
        };

        match &term.applicability {
            Applicability::NotApplicable { .. } => {}
            Applicability::Required => {
                let Some(value) = term.wall_plug_kwh else {
                    missing.push(stage);
                    continue;
                };
                validate_non_negative("wall_plug_kwh", value)?;
                let Some(class) = term.evidence_class else {
                    missing.push(stage);
                    continue;
                };
                let Some(source_ref) = term.source_ref.as_deref() else {
                    missing.push(stage);
                    continue;
                };
                if source_ref.trim().is_empty() {
                    missing.push(stage);
                    continue;
                }
                total_kwh += value;
                evidence_classes.insert(stage, class);
                all_claimable &= class.permits_performance_claim();
            }
        }
    }

    let complete = missing.is_empty();
    let wall_plug_kwh = complete.then_some(total_kwh);
    let wall_plug_kwh_per_accepted_kg =
        wall_plug_kwh.map(|value| value / input.accepted_edible_kg);
    let wall_plug_kwh_per_serving = wall_plug_kwh
        .map(|value| value / input.accepted_edible_kg * input.serving_mass_kg);

    Ok(EnergyReceipt {
        trial_id: input.trial_id.clone(),
        product_key: input.product_key.clone(),
        endpoint_id: input.endpoint_id.clone(),
        comparison_group_id: input.comparison_group_id.clone(),
        boundary_ref: input.boundary_ref.clone(),
        boundary: input.boundary,
        accepted_edible_kg: input.accepted_edible_kg,
        serving_mass_kg: input.serving_mass_kg,
        wall_plug_kwh,
        wall_plug_kwh_per_accepted_kg,
        wall_plug_kwh_per_serving,
        missing_stages: missing,
        evidence_classes,
        complete,
        claim_eligible: complete && all_claimable,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnergyComparison {
    pub control_trial_id: String,
    pub candidate_trial_id: String,
    /// Positive means the candidate uses less electricity per accepted kg.
    /// This is an exploratory arithmetic result, not automatically a claim.
    pub exploratory_reduction_percent: f64,
    /// Present only when both receipts are complete and claim-eligible.
    pub claimable_reduction_percent: Option<f64>,
}

pub fn compare_energy(
    control: &EnergyReceipt,
    candidate: &EnergyReceipt,
) -> Result<EnergyComparison, OracleError> {
    if !control.complete || !candidate.complete {
        return Err(OracleError::IncompatibleComparison(
            "both boundaries must be complete",
        ));
    }
    if control.boundary != candidate.boundary {
        return Err(OracleError::IncompatibleComparison("boundary mismatch"));
    }
    if control.product_key != candidate.product_key {
        return Err(OracleError::IncompatibleComparison("product mismatch"));
    }
    if control.endpoint_id != candidate.endpoint_id {
        return Err(OracleError::IncompatibleComparison("endpoint mismatch"));
    }
    if control.comparison_group_id != candidate.comparison_group_id {
        return Err(OracleError::IncompatibleComparison(
            "comparison group mismatch",
        ));
    }
    if (control.serving_mass_kg - candidate.serving_mass_kg).abs() > 1.0e-9 {
        return Err(OracleError::IncompatibleComparison(
            "serving mass mismatch",
        ));
    }

    let control_intensity = control
        .wall_plug_kwh_per_accepted_kg
        .ok_or(OracleError::IncompatibleComparison("control intensity missing"))?;
    let candidate_intensity = candidate
        .wall_plug_kwh_per_accepted_kg
        .ok_or(OracleError::IncompatibleComparison(
            "candidate intensity missing",
        ))?;
    if control_intensity <= 0.0 {
        return Err(OracleError::ZeroControlEnergy);
    }

    let reduction = (control_intensity - candidate_intensity) / control_intensity * 100.0;
    let claimable_reduction_percent =
        (control.claim_eligible && candidate.claim_eligible).then_some(reduction);

    Ok(EnergyComparison {
        control_trial_id: control.trial_id.clone(),
        candidate_trial_id: candidate.trial_id.clone(),
        exploratory_reduction_percent: reduction,
        claimable_reduction_percent,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeInput {
    pub household_cook_stage_minutes: f64,
    /// Use Some(0.0) when no upstream treatment exists; None means unknown.
    pub upstream_treatment_minutes: Option<f64>,
    pub upstream_parallelized: bool,
    /// Other stages that are genuinely sequential with household cooking.
    /// None means the duration is unknown.
    pub other_sequential_minutes: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeReceipt {
    pub household_cook_stage_minutes: f64,
    pub upstream_treatment_minutes: Option<f64>,
    pub upstream_parallelized: bool,
    /// Unknown when a required duration is missing or upstream work is parallelized.
    /// Parallelized work requires an explicit critical-path schedule, not addition.
    pub total_sequential_minutes: Option<f64>,
}

pub fn calculate_time(input: TimeInput) -> Result<TimeReceipt, OracleError> {
    validate_non_negative(
        "household_cook_stage_minutes",
        input.household_cook_stage_minutes,
    )?;
    if let Some(value) = input.upstream_treatment_minutes {
        validate_non_negative("upstream_treatment_minutes", value)?;
    }
    if let Some(value) = input.other_sequential_minutes {
        validate_non_negative("other_sequential_minutes", value)?;
    }

    let total_sequential_minutes = if input.upstream_parallelized {
        None
    } else {
        input
            .upstream_treatment_minutes
            .zip(input.other_sequential_minutes)
            .map(|(upstream, other)| {
                upstream + input.household_cook_stage_minutes + other
            })
    };

    Ok(TimeReceipt {
        household_cook_stage_minutes: input.household_cook_stage_minutes,
        upstream_treatment_minutes: input.upstream_treatment_minutes,
        upstream_parallelized: input.upstream_parallelized,
        total_sequential_minutes,
    })
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), OracleError> {
    if value.trim().is_empty() {
        return Err(OracleError::EmptyMetadata(field));
    }
    Ok(())
}

fn validate_positive(field: &'static str, value: f64) -> Result<(), OracleError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(OracleError::InvalidPositiveNumber(field));
    }
    Ok(())
}

fn validate_non_negative(field: &'static str, value: f64) -> Result<(), OracleError> {
    if !value.is_finite() || value < 0.0 {
        return Err(OracleError::InvalidNonNegativeNumber(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_input(trial_id: &str, kwh_per_stage: f64) -> EnergyInput {
        let terms = Boundary::GateToPlate
            .required_stages()
            .iter()
            .copied()
            .map(|stage| EnergyTerm {
                stage,
                applicability: Applicability::Required,
                wall_plug_kwh: Some(kwh_per_stage),
                evidence_class: Some(EvidenceClass::Measured),
                source_ref: Some(format!("meter:{:?}", stage)),
            })
            .collect();

        EnergyInput {
            trial_id: trial_id.to_owned(),
            product_key: "brown-rice:cultivar-x".to_owned(),
            endpoint_id: "cooked-texture-v1".to_owned(),
            comparison_group_id: "split-lot-001".to_owned(),
            boundary_ref: "PFB-1:B4".to_owned(),
            boundary: Boundary::GateToPlate,
            accepted_edible_kg: 2.0,
            serving_mass_kg: 0.25,
            terms,
        }
    }

    #[test]
    fn complete_energy_uses_accepted_edible_output_denominator() {
        let input = complete_input("candidate-1", 0.1);
        let receipt = calculate_energy(&input).expect("valid fixture");
        assert!(receipt.complete);
        assert!(receipt.claim_eligible);
        assert_eq!(receipt.wall_plug_kwh, Some(1.2));
        assert_eq!(receipt.wall_plug_kwh_per_accepted_kg, Some(0.6));
        assert_eq!(receipt.wall_plug_kwh_per_serving, Some(0.15));
    }

    #[test]
    fn unknown_required_stage_fails_closed() {
        let mut input = complete_input("candidate-1", 0.1);
        let vacuum = input
            .terms
            .iter_mut()
            .find(|term| term.stage == EnergyStage::Vacuum)
            .expect("vacuum stage exists");
        vacuum.wall_plug_kwh = None;
        vacuum.evidence_class = None;
        vacuum.source_ref = None;

        let receipt = calculate_energy(&input).expect("unknown is not invalid");
        assert!(!receipt.complete);
        assert!(!receipt.claim_eligible);
        assert_eq!(receipt.wall_plug_kwh, None);
        assert_eq!(receipt.missing_stages, vec![EnergyStage::Vacuum]);
    }

    #[test]
    fn not_applicable_requires_a_reason_and_no_numeric_value() {
        let mut input = complete_input("candidate-1", 0.1);
        let vacuum = input
            .terms
            .iter_mut()
            .find(|term| term.stage == EnergyStage::Vacuum)
            .expect("vacuum stage exists");
        vacuum.applicability = Applicability::NotApplicable {
            justification: "Atmospheric-pressure process; no vacuum subsystem".to_owned(),
        };
        vacuum.wall_plug_kwh = None;
        vacuum.evidence_class = None;
        vacuum.source_ref = None;

        let receipt = calculate_energy(&input).expect("valid explicit N/A");
        assert!(receipt.complete);
        assert!(receipt.claim_eligible);
    }

    #[test]
    fn negative_energy_is_rejected() {
        let mut input = complete_input("candidate-1", 0.1);
        input.terms[0].wall_plug_kwh = Some(-0.01);
        assert_eq!(
            calculate_energy(&input),
            Err(OracleError::InvalidNonNegativeNumber("wall_plug_kwh"))
        );
    }

    #[test]
    fn duplicate_stage_is_rejected() {
        let mut input = complete_input("candidate-1", 0.1);
        input.terms.push(input.terms[0].clone());
        assert!(matches!(
            calculate_energy(&input),
            Err(OracleError::DuplicateStage(_))
        ));
    }

    #[test]
    fn mismatched_endpoint_cannot_be_compared() {
        let control =
            calculate_energy(&complete_input("control", 0.2)).expect("control fixture");
        let mut candidate_input = complete_input("candidate", 0.1);
        candidate_input.endpoint_id = "different-texture-v2".to_owned();
        let candidate =
            calculate_energy(&candidate_input).expect("candidate fixture");

        assert_eq!(
            compare_energy(&control, &candidate),
            Err(OracleError::IncompatibleComparison("endpoint mismatch"))
        );
    }

    #[test]
    fn simulated_energy_is_exploratory_not_claimable() {
        let control = calculate_energy(&complete_input("control", 0.2))
            .expect("control fixture");
        let mut candidate_input = complete_input("candidate", 0.1);
        candidate_input.terms[0].evidence_class = Some(EvidenceClass::Simulated);
        let candidate =
            calculate_energy(&candidate_input).expect("candidate fixture");

        let comparison = compare_energy(&control, &candidate).expect("compatible");
        assert_eq!(comparison.exploratory_reduction_percent, 50.0);
        assert_eq!(comparison.claimable_reduction_percent, None);
    }

    #[test]
    fn brown_rice_time_arithmetic_keeps_stage_and_chain_separate() {
        let five_minute_pretreatment = calculate_time(TimeInput {
            household_cook_stage_minutes: 21.1,
            upstream_treatment_minutes: Some(5.0),
            upstream_parallelized: false,
            other_sequential_minutes: Some(0.0),
        })
        .expect("valid time fixture");
        assert_eq!(five_minute_pretreatment.household_cook_stage_minutes, 21.1);
        assert_eq!(five_minute_pretreatment.total_sequential_minutes, Some(26.1));

        let ten_minute_pretreatment = calculate_time(TimeInput {
            household_cook_stage_minutes: 21.1,
            upstream_treatment_minutes: Some(10.0),
            upstream_parallelized: false,
            other_sequential_minutes: Some(0.0),
        })
        .expect("valid time fixture");
        assert_eq!(ten_minute_pretreatment.total_sequential_minutes, Some(31.1));
    }

    #[test]
    fn parallel_upstream_time_is_not_added_as_sequential() {
        let receipt = calculate_time(TimeInput {
            household_cook_stage_minutes: 21.1,
            upstream_treatment_minutes: Some(10.0),
            upstream_parallelized: true,
            other_sequential_minutes: Some(0.0),
        })
        .expect("valid time fixture");
        assert_eq!(receipt.household_cook_stage_minutes, 21.1);
        assert_eq!(receipt.total_sequential_minutes, None);
    }
}
