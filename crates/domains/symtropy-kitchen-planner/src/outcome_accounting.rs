// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic kitchen outcome accounting for accepted food, water and
//! electrical energy. Quantities use exact integer coefficients and scales.
//!
//! This module calculates submitted observations; it does not establish their
//! truth, qualify food safety, or authorize appliance operation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const KITCHEN_OUTCOME_ACCOUNTING_SCHEMA_VERSION: u32 = 1;
pub const KITCHEN_OUTCOME_ACCOUNTING_CLAIM_CEILING: &str =
    "Deterministic arithmetic and evidence completeness for submitted kitchen flow records only; not proof of measurement truth, food safety, sanitation, sensory quality, energy savings, or product superiority.";

const NANO_PER_MILLIGRAM: u128 = 1_000_000;
const NANO_PER_GRAM: u128 = 1_000_000_000;
const NANO_PER_KILOGRAM: u128 = 1_000_000_000_000;
const NANO_LITRES_PER_MILLILITRE: u128 = 1_000_000;
const NANO_LITRES_PER_LITRE: u128 = 1_000_000_000;
const MICRO_WH_PER_WH: u128 = 1_000_000;
const MICRO_WH_PER_KWH: u128 = 1_000_000_000;
const PPM_SCALE: u128 = 1_000_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenFlowEvidenceClassV1 {
    Measured,
    Allocated,
    Attested,
    Declared,
    Modelled,
    Simulated,
}

impl KitchenFlowEvidenceClassV1 {
    fn permits_claim(self) -> bool {
        matches!(self, Self::Measured | Self::Allocated | Self::Attested)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenFlowEvidenceV1 {
    pub evidence_class: KitchenFlowEvidenceClassV1,
    pub source_ref: String,
    pub method_ref: Option<String>,
    pub calibration_ref: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenMassUnitV1 {
    Milligram,
    Gram,
    Kilogram,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenWaterUnitV1 {
    Millilitre,
    Litre,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenEnergyUnitV1 {
    WattHour,
    KilowattHour,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExactKitchenMassV1 {
    pub value: u64,
    pub decimal_places: u8,
    pub unit: KitchenMassUnitV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExactKitchenWaterV1 {
    pub value: u64,
    pub decimal_places: u8,
    pub unit: KitchenWaterUnitV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExactKitchenEnergyV1 {
    pub value: u64,
    pub decimal_places: u8,
    pub unit: KitchenEnergyUnitV1,
}

/// An absent measurement is not represented as numeric zero.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum KitchenFlowObservationV1<T> {
    Observed {
        quantity: T,
        evidence: KitchenFlowEvidenceV1,
    },
    NotApplicable {
        justification: String,
    },
    Unknown {
        reason: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenMassDirectionV1 {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenMassCategoryV1 {
    Ingredient,
    ProcessWater,
    ProcessAid,
    Packaging,
    OtherInput,
    AcceptedEdible,
    Trim,
    Rejected,
    FoodResidue,
    Drainage,
    Evaporated,
    OtherOutput,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenMassFlowTermV1 {
    pub term_id: String,
    pub direction: KitchenMassDirectionV1,
    pub category: KitchenMassCategoryV1,
    pub observation: KitchenFlowObservationV1<ExactKitchenMassV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenFlowStageV1 {
    Preparation,
    Cooking,
    Holding,
    Cleaning,
    PackagingCleaning,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenWaterFlowTermV1 {
    pub stage: KitchenFlowStageV1,
    pub observation: KitchenFlowObservationV1<ExactKitchenWaterV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenElectricityFlowTermV1 {
    pub stage: KitchenFlowStageV1,
    pub observation: KitchenFlowObservationV1<ExactKitchenEnergyV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenOutcomeAccountingRequestV1 {
    pub schema_version: u32,
    pub run_id: String,
    pub recipe_id: String,
    pub boundary_ref: String,
    pub accepted_servings: u64,
    /// The complete list of mass-flow IDs frozen by the trial protocol.
    pub expected_mass_term_ids: Vec<String>,
    pub mass_terms: Vec<KitchenMassFlowTermV1>,
    pub required_water_stages: Vec<KitchenFlowStageV1>,
    pub water_terms: Vec<KitchenWaterFlowTermV1>,
    pub required_electricity_stages: Vec<KitchenFlowStageV1>,
    pub electricity_terms: Vec<KitchenElectricityFlowTermV1>,
    pub claim_ceiling: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenOutcomeIssueCodeV1 {
    UnsupportedSchemaVersion,
    MissingRequiredValue,
    InvalidClaimCeiling,
    DuplicateExpectedMassTerm,
    DuplicateMassTerm,
    MissingMassTerm,
    UnexpectedMassTerm,
    MassCategoryDirectionMismatch,
    InvalidMassScale,
    InvalidWaterScale,
    InvalidEnergyScale,
    InvalidEvidence,
    InvalidNotApplicable,
    UnknownMeasurement,
    DuplicateWaterStage,
    MissingWaterStage,
    UnexpectedWaterStage,
    DuplicateElectricityStage,
    MissingElectricityStage,
    UnexpectedElectricityStage,
    MissingInputMass,
    MissingOutputMass,
    MissingAcceptedEdibleOutput,
    AcceptedServingsMassMismatch,
    OutputMassExceedsInput,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenOutcomeIssueV1 {
    pub code: KitchenOutcomeIssueCodeV1,
    pub reference: Option<String>,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenOutcomeDispositionV1 {
    CompleteClaimEligible,
    CompleteExploratory,
    Indeterminate,
    Blocked,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExactPerServingRatioV1 {
    pub numerator: u128,
    pub denominator_servings: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenOutcomeAccountingReportV1 {
    pub schema_version: u32,
    pub run_id: String,
    pub recipe_id: String,
    pub boundary_ref: String,
    pub accepted_servings: u64,
    pub disposition: KitchenOutcomeDispositionV1,
    pub complete: bool,
    pub claim_eligible: bool,
    pub input_mass_nanograms: Option<u128>,
    pub output_mass_nanograms: Option<u128>,
    pub accepted_edible_mass_nanograms: Option<u128>,
    pub unaccounted_mass_nanograms: Option<u128>,
    pub unaccounted_mass_fraction_ppm: Option<u64>,
    pub water_nanolitres: Option<u128>,
    pub water_nanolitres_per_accepted_serving: Option<ExactPerServingRatioV1>,
    pub electricity_microwatt_hours: Option<u128>,
    pub electricity_microwatt_hours_per_accepted_serving: Option<ExactPerServingRatioV1>,
    pub not_applicable_refs: Vec<String>,
    pub unknown_refs: Vec<String>,
    pub issues: Vec<KitchenOutcomeIssueV1>,
    pub claim_ceiling: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn add_issue(
    issues: &mut Vec<KitchenOutcomeIssueV1>,
    code: KitchenOutcomeIssueCodeV1,
    reference: Option<&str>,
    detail: impl Into<String>,
) {
    issues.push(KitchenOutcomeIssueV1 {
        code,
        reference: reference.map(str::to_owned),
        detail: detail.into(),
    });
}

fn sort_issues(issues: &mut [KitchenOutcomeIssueV1]) {
    issues.sort_by(|left, right| {
        (left.code, left.reference.as_deref(), left.detail.as_str()).cmp(&(
            right.code,
            right.reference.as_deref(),
            right.detail.as_str(),
        ))
    });
}

fn validate_evidence(
    evidence: &KitchenFlowEvidenceV1,
    reference: &str,
    issues: &mut Vec<KitchenOutcomeIssueV1>,
) -> bool {
    let valid = nonempty(&evidence.source_ref);
    let mut result = valid;
    if !valid {
        add_issue(
            issues,
            KitchenOutcomeIssueCodeV1::InvalidEvidence,
            Some(reference),
            "observed flow requires a non-empty source reference",
        );
    }
    let required_ref = match evidence.evidence_class {
        KitchenFlowEvidenceClassV1::Measured => evidence.calibration_ref.as_deref(),
        KitchenFlowEvidenceClassV1::Allocated | KitchenFlowEvidenceClassV1::Attested => {
            evidence.method_ref.as_deref()
        }
        KitchenFlowEvidenceClassV1::Declared
        | KitchenFlowEvidenceClassV1::Modelled
        | KitchenFlowEvidenceClassV1::Simulated => None,
    };
    if matches!(
        evidence.evidence_class,
        KitchenFlowEvidenceClassV1::Measured
            | KitchenFlowEvidenceClassV1::Allocated
            | KitchenFlowEvidenceClassV1::Attested
    ) && required_ref.is_none_or(|value| !nonempty(value))
    {
        add_issue(
            issues,
            KitchenOutcomeIssueCodeV1::InvalidEvidence,
            Some(reference),
            "measured evidence requires calibration; allocated/attested evidence requires a method reference",
        );
        result = false;
    }
    result
}

fn decimal_divisor(decimal_places: u8) -> Option<u128> {
    if decimal_places > 6 {
        return None;
    }
    10_u128.checked_pow(u32::from(decimal_places))
}

fn exact_units(value: u64, decimal_places: u8, base: u128) -> Result<u128, KitchenOutcomeIssueCodeV1> {
    let divisor = decimal_divisor(decimal_places)
        .ok_or(KitchenOutcomeIssueCodeV1::InvalidMassScale)?;
    if base % divisor != 0 {
        return Err(KitchenOutcomeIssueCodeV1::InvalidMassScale);
    }
    u128::from(value)
        .checked_mul(base / divisor)
        .ok_or(KitchenOutcomeIssueCodeV1::ArithmeticOverflow)
}

fn mass_nanograms(value: ExactKitchenMassV1) -> Result<u128, KitchenOutcomeIssueCodeV1> {
    let base = match value.unit {
        KitchenMassUnitV1::Milligram => NANO_PER_MILLIGRAM,
        KitchenMassUnitV1::Gram => NANO_PER_GRAM,
        KitchenMassUnitV1::Kilogram => NANO_PER_KILOGRAM,
    };
    exact_units(value.value, value.decimal_places, base)
}

fn water_nanolitres(value: ExactKitchenWaterV1) -> Result<u128, KitchenOutcomeIssueCodeV1> {
    let base = match value.unit {
        KitchenWaterUnitV1::Millilitre => NANO_LITRES_PER_MILLILITRE,
        KitchenWaterUnitV1::Litre => NANO_LITRES_PER_LITRE,
    };
    let divisor = decimal_divisor(value.decimal_places)
        .ok_or(KitchenOutcomeIssueCodeV1::InvalidWaterScale)?;
    if base % divisor != 0 {
        return Err(KitchenOutcomeIssueCodeV1::InvalidWaterScale);
    }
    u128::from(value.value)
        .checked_mul(base / divisor)
        .ok_or(KitchenOutcomeIssueCodeV1::ArithmeticOverflow)
}

fn energy_microwatt_hours(value: ExactKitchenEnergyV1) -> Result<u128, KitchenOutcomeIssueCodeV1> {
    let base = match value.unit {
        KitchenEnergyUnitV1::WattHour => MICRO_WH_PER_WH,
        KitchenEnergyUnitV1::KilowattHour => MICRO_WH_PER_KWH,
    };
    let divisor = decimal_divisor(value.decimal_places)
        .ok_or(KitchenOutcomeIssueCodeV1::InvalidEnergyScale)?;
    if base % divisor != 0 {
        return Err(KitchenOutcomeIssueCodeV1::InvalidEnergyScale);
    }
    u128::from(value.value)
        .checked_mul(base / divisor)
        .ok_or(KitchenOutcomeIssueCodeV1::ArithmeticOverflow)
}

fn category_matches(category: KitchenMassCategoryV1, direction: KitchenMassDirectionV1) -> bool {
    match category {
        KitchenMassCategoryV1::Ingredient
        | KitchenMassCategoryV1::ProcessWater
        | KitchenMassCategoryV1::ProcessAid
        | KitchenMassCategoryV1::Packaging
        | KitchenMassCategoryV1::OtherInput => direction == KitchenMassDirectionV1::Input,
        KitchenMassCategoryV1::AcceptedEdible
        | KitchenMassCategoryV1::Trim
        | KitchenMassCategoryV1::Rejected
        | KitchenMassCategoryV1::FoodResidue
        | KitchenMassCategoryV1::Drainage
        | KitchenMassCategoryV1::Evaporated
        | KitchenMassCategoryV1::OtherOutput => direction == KitchenMassDirectionV1::Output,
    }
}

fn observe_mass(
    terms: &[KitchenMassFlowTermV1],
    expected_ids: &[String],
    issues: &mut Vec<KitchenOutcomeIssueV1>,
    not_applicable: &mut Vec<String>,
    unknown: &mut Vec<String>,
    claimable: &mut bool,
) -> (Option<u128>, Option<u128>, Option<u128>, bool) {
    let expected: BTreeSet<&str> = expected_ids.iter().map(String::as_str).collect();
    let mut seen = BTreeSet::new();
    let mut input_total = 0_u128;
    let mut output_total = 0_u128;
    let mut accepted_total = 0_u128;
    let mut input_seen = false;
    let mut output_seen = false;
    let mut accepted_seen = false;
    let mut complete = true;

    for term in terms {
        if !nonempty(&term.term_id) {
            add_issue(
                issues,
                KitchenOutcomeIssueCodeV1::MissingRequiredValue,
                None,
                "mass term identifier must be non-empty",
            );
            complete = false;
            continue;
        }
        if !seen.insert(term.term_id.as_str()) {
            add_issue(
                issues,
                KitchenOutcomeIssueCodeV1::DuplicateMassTerm,
                Some(&term.term_id),
                "duplicate mass term identifier",
            );
            complete = false;
        }
        if !expected.contains(term.term_id.as_str()) {
            add_issue(
                issues,
                KitchenOutcomeIssueCodeV1::UnexpectedMassTerm,
                Some(&term.term_id),
                "mass term is not declared in the expected term set",
            );
            complete = false;
        }
        if !category_matches(term.category, term.direction) {
            add_issue(
                issues,
                KitchenOutcomeIssueCodeV1::MassCategoryDirectionMismatch,
                Some(&term.term_id),
                "mass category and direction do not agree",
            );
            complete = false;
        }

        match &term.observation {
            KitchenFlowObservationV1::Observed { quantity, evidence } => {
                if !validate_evidence(evidence, &term.term_id, issues) {
                    complete = false;
                    continue;
                }
                if !evidence.evidence_class.permits_claim() {
                    *claimable = false;
                }
                match mass_nanograms(*quantity) {
                    Ok(value) => {
                        let total = match term.direction {
                            KitchenMassDirectionV1::Input => {
                                input_seen = true;
                                &mut input_total
                            }
                            KitchenMassDirectionV1::Output => {
                                output_seen = true;
                                &mut output_total
                            }
                        };
                        match total.checked_add(value) {
                            Some(sum) => *total = sum,
                            None => {
                                add_issue(
                                    issues,
                                    KitchenOutcomeIssueCodeV1::ArithmeticOverflow,
                                    Some(&term.term_id),
                                    "mass aggregation exceeded checked u128 arithmetic",
                                );
                                complete = false;
                            }
                        }
                        if term.category == KitchenMassCategoryV1::AcceptedEdible {
                            accepted_seen = true;
                            match accepted_total.checked_add(value) {
                                Some(sum) => accepted_total = sum,
                                None => {
                                    add_issue(
                                        issues,
                                        KitchenOutcomeIssueCodeV1::ArithmeticOverflow,
                                        Some(&term.term_id),
                                        "accepted edible mass aggregation overflowed",
                                    );
                                    complete = false;
                                }
                            }
                        }
                    }
                    Err(code) => {
                        add_issue(
                            issues,
                            code,
                            Some(&term.term_id),
                            "mass quantity cannot be represented exactly",
                        );
                        complete = false;
                    }
                }
            }
            KitchenFlowObservationV1::NotApplicable { justification } => {
                if !nonempty(justification) {
                    add_issue(
                        issues,
                        KitchenOutcomeIssueCodeV1::InvalidNotApplicable,
                        Some(&term.term_id),
                        "not-applicable mass term requires a justification",
                    );
                    complete = false;
                } else {
                    not_applicable.push(format!("mass:{}", term.term_id));
                    if term.category == KitchenMassCategoryV1::AcceptedEdible {
                        add_issue(
                            issues,
                            KitchenOutcomeIssueCodeV1::MissingAcceptedEdibleOutput,
                            Some(&term.term_id),
                            "accepted edible output cannot be not applicable",
                        );
                        complete = false;
                    }
                }
            }
            KitchenFlowObservationV1::Unknown { reason } => {
                unknown.push(format!("mass:{}", term.term_id));
                add_issue(
                    issues,
                    KitchenOutcomeIssueCodeV1::UnknownMeasurement,
                    Some(&term.term_id),
                    if nonempty(reason) { reason.clone() } else { "unknown measurement has no reason".to_owned() },
                );
                complete = false;
            }
        }
    }

    for id in expected_ids {
        if !seen.contains(id.as_str()) {
            add_issue(
                issues,
                KitchenOutcomeIssueCodeV1::MissingMassTerm,
                Some(id),
                "expected mass term is absent",
            );
            complete = false;
        }
    }
    if !input_seen {
        add_issue(
            issues,
            KitchenOutcomeIssueCodeV1::MissingInputMass,
            None,
            "at least one measured/allocated/attested input mass is required",
        );
        complete = false;
    }
    if !output_seen {
        add_issue(
            issues,
            KitchenOutcomeIssueCodeV1::MissingOutputMass,
            None,
            "at least one measured/allocated/attested output mass is required",
        );
        complete = false;
    }
    if !accepted_seen {
        add_issue(
            issues,
            KitchenOutcomeIssueCodeV1::MissingAcceptedEdibleOutput,
            None,
            "accepted edible output requires an explicit observed value, including zero for a rejected batch",
        );
        complete = false;
    }

    if complete {
        (Some(input_total), Some(output_total), Some(accepted_total), true)
    } else {
        (None, None, None, false)
    }
}

fn observe_water(
    terms: &[KitchenWaterFlowTermV1],
    required: &[KitchenFlowStageV1],
    issues: &mut Vec<KitchenOutcomeIssueV1>,
    not_applicable: &mut Vec<String>,
    unknown: &mut Vec<String>,
    claimable: &mut bool,
) -> (Option<u128>, bool) {
    let expected: BTreeSet<KitchenFlowStageV1> = required.iter().copied().collect();
    let mut seen = BTreeSet::new();
    let mut total = 0_u128;
    let mut complete = true;

    for term in terms {
        if !seen.insert(term.stage) {
            add_issue(
                issues,
                KitchenOutcomeIssueCodeV1::DuplicateWaterStage,
                Some(&format!("{:?}", term.stage)),
                "duplicate water stage",
            );
            complete = false;
            continue;
        }
        if !expected.contains(&term.stage) {
            add_issue(
                issues,
                KitchenOutcomeIssueCodeV1::UnexpectedWaterStage,
                Some(&format!("{:?}", term.stage)),
                "water stage is outside the declared boundary",
            );
            complete = false;
        }
        match &term.observation {
            KitchenFlowObservationV1::Observed { quantity, evidence } => {
                let reference = format!("water:{:?}", term.stage);
                if !validate_evidence(evidence, &reference, issues) {
                    complete = false;
                    continue;
                }
                if !evidence.evidence_class.permits_claim() {
                    *claimable = false;
                }
                match water_nanolitres(*quantity) {
                    Ok(value) => match total.checked_add(value) {
                        Some(sum) => total = sum,
                        None => {
                            add_issue(
                                issues,
                                KitchenOutcomeIssueCodeV1::ArithmeticOverflow,
                                Some(&reference),
                                "water aggregation overflowed checked u128 arithmetic",
                            );
                            complete = false;
                        }
                    },
                    Err(code) => {
                        add_issue(
                            issues,
                            code,
                            Some(&reference),
                            "water quantity cannot be represented exactly",
                        );
                        complete = false;
                    }
                }
            }
            KitchenFlowObservationV1::NotApplicable { justification } => {
                if !nonempty(justification) {
                    add_issue(
                        issues,
                        KitchenOutcomeIssueCodeV1::InvalidNotApplicable,
                        Some(&format!("{:?}", term.stage)),
                        "not-applicable water stage requires a justification",
                    );
                    complete = false;
                } else {
                    not_applicable.push(format!("water:{:?}", term.stage));
                }
            }
            KitchenFlowObservationV1::Unknown { reason } => {
                unknown.push(format!("water:{:?}", term.stage));
                add_issue(
                    issues,
                    KitchenOutcomeIssueCodeV1::UnknownMeasurement,
                    Some(&format!("{:?}", term.stage)),
                    if nonempty(reason) { reason.clone() } else { "unknown measurement has no reason".to_owned() },
                );
                complete = false;
            }
        }
    }
    for stage in required {
        if !seen.contains(stage) {
            add_issue(
                issues,
                KitchenOutcomeIssueCodeV1::MissingWaterStage,
                Some(&format!("{:?}", stage)),
                "required water stage is missing",
            );
            complete = false;
        }
    }
    (complete.then_some(total), complete)
}

fn observe_electricity(
    terms: &[KitchenElectricityFlowTermV1],
    required: &[KitchenFlowStageV1],
    issues: &mut Vec<KitchenOutcomeIssueV1>,
    not_applicable: &mut Vec<String>,
    unknown: &mut Vec<String>,
    claimable: &mut bool,
) -> (Option<u128>, bool) {
    let expected: BTreeSet<KitchenFlowStageV1> = required.iter().copied().collect();
    let mut seen = BTreeSet::new();
    let mut total = 0_u128;
    let mut complete = true;

    for term in terms {
        if !seen.insert(term.stage) {
            add_issue(
                issues,
                KitchenOutcomeIssueCodeV1::DuplicateElectricityStage,
                Some(&format!("{:?}", term.stage)),
                "duplicate electricity stage",
            );
            complete = false;
            continue;
        }
        if !expected.contains(&term.stage) {
            add_issue(
                issues,
                KitchenOutcomeIssueCodeV1::UnexpectedElectricityStage,
                Some(&format!("{:?}", term.stage)),
                "electricity stage is outside the declared boundary",
            );
            complete = false;
        }
        match &term.observation {
            KitchenFlowObservationV1::Observed { quantity, evidence } => {
                let reference = format!("electricity:{:?}", term.stage);
                if !validate_evidence(evidence, &reference, issues) {
                    complete = false;
                    continue;
                }
                if !evidence.evidence_class.permits_claim() {
                    *claimable = false;
                }
                match energy_microwatt_hours(*quantity) {
                    Ok(value) => match total.checked_add(value) {
                        Some(sum) => total = sum,
                        None => {
                            add_issue(
                                issues,
                                KitchenOutcomeIssueCodeV1::ArithmeticOverflow,
                                Some(&reference),
                                "electricity aggregation overflowed checked u128 arithmetic",
                            );
                            complete = false;
                        }
                    },
                    Err(code) => {
                        add_issue(
                            issues,
                            code,
                            Some(&reference),
                            "electrical energy cannot be represented exactly",
                        );
                        complete = false;
                    }
                }
            }
            KitchenFlowObservationV1::NotApplicable { justification } => {
                if !nonempty(justification) {
                    add_issue(
                        issues,
                        KitchenOutcomeIssueCodeV1::InvalidNotApplicable,
                        Some(&format!("{:?}", term.stage)),
                        "not-applicable electricity stage requires a justification",
                    );
                    complete = false;
                } else {
                    not_applicable.push(format!("electricity:{:?}", term.stage));
                }
            }
            KitchenFlowObservationV1::Unknown { reason } => {
                unknown.push(format!("electricity:{:?}", term.stage));
                add_issue(
                    issues,
                    KitchenOutcomeIssueCodeV1::UnknownMeasurement,
                    Some(&format!("{:?}", term.stage)),
                    if nonempty(reason) { reason.clone() } else { "unknown measurement has no reason".to_owned() },
                );
                complete = false;
            }
        }
    }
    for stage in required {
        if !seen.contains(stage) {
            add_issue(
                issues,
                KitchenOutcomeIssueCodeV1::MissingElectricityStage,
                Some(&format!("{:?}", stage)),
                "required electricity stage is missing",
            );
            complete = false;
        }
    }
    (complete.then_some(total), complete)
}

fn is_blocking(code: KitchenOutcomeIssueCodeV1) -> bool {
    matches!(
        code,
        KitchenOutcomeIssueCodeV1::UnsupportedSchemaVersion
            | KitchenOutcomeIssueCodeV1::MissingRequiredValue
            | KitchenOutcomeIssueCodeV1::InvalidClaimCeiling
            | KitchenOutcomeIssueCodeV1::DuplicateExpectedMassTerm
            | KitchenOutcomeIssueCodeV1::DuplicateMassTerm
            | KitchenOutcomeIssueCodeV1::UnexpectedMassTerm
            | KitchenOutcomeIssueCodeV1::MassCategoryDirectionMismatch
            | KitchenOutcomeIssueCodeV1::InvalidMassScale
            | KitchenOutcomeIssueCodeV1::InvalidWaterScale
            | KitchenOutcomeIssueCodeV1::InvalidEnergyScale
            | KitchenOutcomeIssueCodeV1::InvalidEvidence
            | KitchenOutcomeIssueCodeV1::InvalidNotApplicable
            | KitchenOutcomeIssueCodeV1::DuplicateWaterStage
            | KitchenOutcomeIssueCodeV1::UnexpectedWaterStage
            | KitchenOutcomeIssueCodeV1::DuplicateElectricityStage
            | KitchenOutcomeIssueCodeV1::UnexpectedElectricityStage
            | KitchenOutcomeIssueCodeV1::OutputMassExceedsInput
            | KitchenOutcomeIssueCodeV1::AcceptedServingsMassMismatch
    )
}

/// Evaluate a frozen accounting boundary. Unknown/missing terms yield no total.
pub fn evaluate_kitchen_outcome_accounting(
    request: &KitchenOutcomeAccountingRequestV1,
) -> KitchenOutcomeAccountingReportV1 {
    let mut issues = Vec::new();
    let mut not_applicable_refs = Vec::new();
    let mut unknown_refs = Vec::new();
    let mut claimable = true;

    if request.schema_version != KITCHEN_OUTCOME_ACCOUNTING_SCHEMA_VERSION {
        add_issue(
            &mut issues,
            KitchenOutcomeIssueCodeV1::UnsupportedSchemaVersion,
            None,
            "unsupported outcome-accounting schema version",
        );
    }
    if !nonempty(&request.run_id)
        || !nonempty(&request.recipe_id)
        || !nonempty(&request.boundary_ref)
    {
        add_issue(
            &mut issues,
            KitchenOutcomeIssueCodeV1::MissingRequiredValue,
            None,
            "run, recipe and boundary identifiers must be non-empty",
        );
    }
    if request.claim_ceiling != KITCHEN_OUTCOME_ACCOUNTING_CLAIM_CEILING {
        add_issue(
            &mut issues,
            KitchenOutcomeIssueCodeV1::InvalidClaimCeiling,
            None,
            "claim ceiling does not match the fixed descriptive-only statement",
        );
    }

    let mut expected_ids = BTreeSet::new();
    for id in &request.expected_mass_term_ids {
        if !nonempty(id) {
            add_issue(
                &mut issues,
                KitchenOutcomeIssueCodeV1::MissingRequiredValue,
                None,
                "expected mass term identifiers must be non-empty",
            );
        }
        if !expected_ids.insert(id.as_str()) {
            add_issue(
                &mut issues,
                KitchenOutcomeIssueCodeV1::DuplicateExpectedMassTerm,
                Some(id),
                "expected mass term identifier is duplicated",
            );
        }
    }
    if request.expected_mass_term_ids.is_empty()
        || request.required_water_stages.is_empty()
        || request.required_electricity_stages.is_empty()
    {
        add_issue(
            &mut issues,
            KitchenOutcomeIssueCodeV1::MissingRequiredValue,
            None,
            "mass, water and electricity boundaries must each declare expected terms/stages",
        );
    }
    if request.required_water_stages.iter().collect::<BTreeSet<_>>().len()
        != request.required_water_stages.len()
    {
        add_issue(
            &mut issues,
            KitchenOutcomeIssueCodeV1::DuplicateWaterStage,
            None,
            "required water stages contain duplicates",
        );
    }
    if request.required_electricity_stages.iter().collect::<BTreeSet<_>>().len()
        != request.required_electricity_stages.len()
    {
        add_issue(
            &mut issues,
            KitchenOutcomeIssueCodeV1::DuplicateElectricityStage,
            None,
            "required electricity stages contain duplicates",
        );
    }

    let (input_mass, output_mass, accepted_mass, mass_complete) = observe_mass(
        &request.mass_terms,
        &request.expected_mass_term_ids,
        &mut issues,
        &mut not_applicable_refs,
        &mut unknown_refs,
        &mut claimable,
    );
    let (water_total, water_complete) = observe_water(
        &request.water_terms,
        &request.required_water_stages,
        &mut issues,
        &mut not_applicable_refs,
        &mut unknown_refs,
        &mut claimable,
    );
    let (electricity_total, electricity_complete) = observe_electricity(
        &request.electricity_terms,
        &request.required_electricity_stages,
        &mut issues,
        &mut not_applicable_refs,
        &mut unknown_refs,
        &mut claimable,
    );

    let mut residual = None;
    let mut residual_ppm = None;
    if let (Some(input), Some(output)) = (input_mass, output_mass) {
        if output > input {
            add_issue(
                &mut issues,
                KitchenOutcomeIssueCodeV1::OutputMassExceedsInput,
                None,
                "accounted output mass exceeds the submitted input mass",
            );
        } else {
            let missing_mass = input - output;
            residual = Some(missing_mass);
            if input > 0 {
                match missing_mass.checked_mul(PPM_SCALE) {
                    Some(scaled) => residual_ppm = Some((scaled / input) as u64),
                    None => add_issue(
                        &mut issues,
                        KitchenOutcomeIssueCodeV1::ArithmeticOverflow,
                        None,
                        "mass-balance fraction overflowed checked arithmetic",
                    ),
                }
            } else {
                add_issue(
                    &mut issues,
                    KitchenOutcomeIssueCodeV1::MissingInputMass,
                    None,
                    "total input mass must be positive",
                );
            }
        }
    }

    match (accepted_mass, request.accepted_servings) {
        (Some(mass), 0) if mass > 0 => add_issue(
            &mut issues,
            KitchenOutcomeIssueCodeV1::AcceptedServingsMassMismatch,
            None,
            "positive accepted edible mass requires at least one accepted serving",
        ),
        (Some(0), servings) if servings > 0 => add_issue(
            &mut issues,
            KitchenOutcomeIssueCodeV1::AcceptedServingsMassMismatch,
            None,
            "positive serving count requires positive accepted edible mass",
        ),
        _ => {}
    }

    sort_issues(&mut issues);
    not_applicable_refs.sort();
    not_applicable_refs.dedup();
    unknown_refs.sort();
    unknown_refs.dedup();

    let complete = mass_complete
        && water_complete
        && electricity_complete
        && input_mass.is_some()
        && output_mass.is_some()
        && accepted_mass.is_some()
        && residual.is_some()
        && !issues.iter().any(|issue| {
            is_blocking(issue.code)
                || issue.code == KitchenOutcomeIssueCodeV1::ArithmeticOverflow
        });
    let claim_eligible = complete
        && claimable
        && request.accepted_servings > 0
        && residual_ppm.is_some();
    let disposition = if issues.iter().any(|issue| is_blocking(issue.code)) {
        KitchenOutcomeDispositionV1::Blocked
    } else if !complete {
        KitchenOutcomeDispositionV1::Indeterminate
    } else if claim_eligible {
        KitchenOutcomeDispositionV1::CompleteClaimEligible
    } else {
        KitchenOutcomeDispositionV1::CompleteExploratory
    };

    let water_per_serving = match (water_total, request.accepted_servings) {
        (Some(total), servings) if servings > 0 => Some(ExactPerServingRatioV1 {
            numerator: total,
            denominator_servings: servings,
        }),
        _ => None,
    };
    let electricity_per_serving = match (electricity_total, request.accepted_servings) {
        (Some(total), servings) if servings > 0 => Some(ExactPerServingRatioV1 {
            numerator: total,
            denominator_servings: servings,
        }),
        _ => None,
    };

    KitchenOutcomeAccountingReportV1 {
        schema_version: KITCHEN_OUTCOME_ACCOUNTING_SCHEMA_VERSION,
        run_id: request.run_id.clone(),
        recipe_id: request.recipe_id.clone(),
        boundary_ref: request.boundary_ref.clone(),
        accepted_servings: request.accepted_servings,
        disposition,
        complete,
        claim_eligible,
        input_mass_nanograms: input_mass,
        output_mass_nanograms: output_mass,
        accepted_edible_mass_nanograms: accepted_mass,
        unaccounted_mass_nanograms: residual,
        unaccounted_mass_fraction_ppm: residual_ppm,
        water_nanolitres: water_total,
        water_nanolitres_per_accepted_serving: water_per_serving,
        electricity_microwatt_hours: electricity_total,
        electricity_microwatt_hours_per_accepted_serving: electricity_per_serving,
        not_applicable_refs,
        unknown_refs,
        issues,
        claim_ceiling: KITCHEN_OUTCOME_ACCOUNTING_CLAIM_CEILING.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(class: KitchenFlowEvidenceClassV1) -> KitchenFlowEvidenceV1 {
        KitchenFlowEvidenceV1 {
            evidence_class: class,
            source_ref: "fixture:meter-v1".to_owned(),
            method_ref: matches!(
                class,
                KitchenFlowEvidenceClassV1::Allocated | KitchenFlowEvidenceClassV1::Attested
            )
            .then(|| "fixture:method-v1".to_owned()),
            calibration_ref: (class == KitchenFlowEvidenceClassV1::Measured)
                .then(|| "fixture:calibration-v1".to_owned()),
        }
    }

    fn mass_term(
        id: &str,
        direction: KitchenMassDirectionV1,
        category: KitchenMassCategoryV1,
        quantity: ExactKitchenMassV1,
    ) -> KitchenMassFlowTermV1 {
        KitchenMassFlowTermV1 {
            term_id: id.to_owned(),
            direction,
            category,
            observation: KitchenFlowObservationV1::Observed {
                quantity,
                evidence: evidence(KitchenFlowEvidenceClassV1::Measured),
            },
        }
    }

    fn water_term(stage: KitchenFlowStageV1, value: u64, unit: KitchenWaterUnitV1) -> KitchenWaterFlowTermV1 {
        KitchenWaterFlowTermV1 {
            stage,
            observation: KitchenFlowObservationV1::Observed {
                quantity: ExactKitchenWaterV1 {
                    value,
                    decimal_places: 0,
                    unit,
                },
                evidence: evidence(KitchenFlowEvidenceClassV1::Measured),
            },
        }
    }

    fn energy_term(stage: KitchenFlowStageV1, value: u64, unit: KitchenEnergyUnitV1) -> KitchenElectricityFlowTermV1 {
        KitchenElectricityFlowTermV1 {
            stage,
            observation: KitchenFlowObservationV1::Observed {
                quantity: ExactKitchenEnergyV1 {
                    value,
                    decimal_places: 0,
                    unit,
                },
                evidence: evidence(KitchenFlowEvidenceClassV1::Measured),
            },
        }
    }

    fn request() -> KitchenOutcomeAccountingRequestV1 {
        KitchenOutcomeAccountingRequestV1 {
            schema_version: KITCHEN_OUTCOME_ACCOUNTING_SCHEMA_VERSION,
            run_id: "fixture:meal-001".to_owned(),
            recipe_id: "fixture:rice-veg-001".to_owned(),
            boundary_ref: "fixture:gate-to-plate-v1".to_owned(),
            accepted_servings: 4,
            expected_mass_term_ids: vec![
                "ingredient".to_owned(),
                "accepted".to_owned(),
                "trim".to_owned(),
                "rejected".to_owned(),
                "evaporated".to_owned(),
            ],
            mass_terms: vec![
                mass_term("ingredient", KitchenMassDirectionV1::Input, KitchenMassCategoryV1::Ingredient, ExactKitchenMassV1 { value: 1_000, decimal_places: 0, unit: KitchenMassUnitV1::Gram }),
                mass_term("accepted", KitchenMassDirectionV1::Output, KitchenMassCategoryV1::AcceptedEdible, ExactKitchenMassV1 { value: 800, decimal_places: 0, unit: KitchenMassUnitV1::Gram }),
                mass_term("trim", KitchenMassDirectionV1::Output, KitchenMassCategoryV1::Trim, ExactKitchenMassV1 { value: 100, decimal_places: 0, unit: KitchenMassUnitV1::Gram }),
                mass_term("rejected", KitchenMassDirectionV1::Output, KitchenMassCategoryV1::Rejected, ExactKitchenMassV1 { value: 50, decimal_places: 0, unit: KitchenMassUnitV1::Gram }),
                mass_term("evaporated", KitchenMassDirectionV1::Output, KitchenMassCategoryV1::Evaporated, ExactKitchenMassV1 { value: 50, decimal_places: 0, unit: KitchenMassUnitV1::Gram }),
            ],
            required_water_stages: vec![KitchenFlowStageV1::Preparation, KitchenFlowStageV1::Cooking, KitchenFlowStageV1::Cleaning],
            water_terms: vec![
                water_term(KitchenFlowStageV1::Preparation, 500, KitchenWaterUnitV1::Millilitre),
                water_term(KitchenFlowStageV1::Cooking, 1, KitchenWaterUnitV1::Litre),
                water_term(KitchenFlowStageV1::Cleaning, 500, KitchenWaterUnitV1::Millilitre),
            ],
            required_electricity_stages: vec![KitchenFlowStageV1::Preparation, KitchenFlowStageV1::Cooking, KitchenFlowStageV1::Cleaning],
            electricity_terms: vec![
                energy_term(KitchenFlowStageV1::Preparation, 100, KitchenEnergyUnitV1::WattHour),
                energy_term(KitchenFlowStageV1::Cooking, 1, KitchenEnergyUnitV1::KilowattHour),
                energy_term(KitchenFlowStageV1::Cleaning, 50, KitchenEnergyUnitV1::WattHour),
            ],
            claim_ceiling: KITCHEN_OUTCOME_ACCOUNTING_CLAIM_CEILING.to_owned(),
        }
    }

    #[test]
    fn complete_meal_accounting_is_exact_and_claim_eligible() {
        let report = evaluate_kitchen_outcome_accounting(&request());
        assert_eq!(report.disposition, KitchenOutcomeDispositionV1::CompleteClaimEligible);
        assert!(report.claim_eligible);
        assert_eq!(report.input_mass_nanograms, Some(1_000 * NANO_PER_GRAM));
        assert_eq!(report.output_mass_nanograms, Some(1_000 * NANO_PER_GRAM));
        assert_eq!(report.unaccounted_mass_nanograms, Some(0));
        assert_eq!(report.water_nanolitres, Some(2_000 * NANO_LITRES_PER_MILLILITRE));
        assert_eq!(report.electricity_microwatt_hours, Some(1_150 * MICRO_WH_PER_WH));
        assert_eq!(
            report.water_nanolitres_per_accepted_serving,
            Some(ExactPerServingRatioV1 {
                numerator: 2_000 * NANO_LITRES_PER_MILLILITRE,
                denominator_servings: 4,
            })
        );
    }

    #[test]
    fn unknown_water_is_not_silently_zero() {
        let mut request = request();
        request.water_terms[1].observation = KitchenFlowObservationV1::Unknown {
            reason: "meter unavailable".to_owned(),
        };
        let report = evaluate_kitchen_outcome_accounting(&request);
        assert_eq!(report.disposition, KitchenOutcomeDispositionV1::Indeterminate);
        assert_eq!(report.water_nanolitres, None);
        assert!(!report.claim_eligible);
        assert!(report.unknown_refs.iter().any(|value| value == "water:Cooking"));
    }

    #[test]
    fn not_applicable_requires_a_reason() {
        let mut request = request();
        request.water_terms[0].observation = KitchenFlowObservationV1::NotApplicable {
            justification: "no preparation water is used in this declared process".to_owned(),
        };
        let report = evaluate_kitchen_outcome_accounting(&request);
        assert!(report.complete);
        assert!(report.not_applicable_refs.iter().any(|value| value == "water:Preparation"));

        let mut request = request();
        request.water_terms[0].observation = KitchenFlowObservationV1::NotApplicable {
            justification: " ".to_owned(),
        };
        assert_eq!(
            evaluate_kitchen_outcome_accounting(&request).disposition,
            KitchenOutcomeDispositionV1::Blocked
        );
    }

    #[test]
    fn simulated_evidence_is_exploratory_not_claim_eligible() {
        let mut request = request();
        if let KitchenFlowObservationV1::Observed { evidence, .. } = &mut request.electricity_terms[0].observation {
            evidence.evidence_class = KitchenFlowEvidenceClassV1::Simulated;
        }
        let report = evaluate_kitchen_outcome_accounting(&request);
        assert_eq!(report.disposition, KitchenOutcomeDispositionV1::CompleteExploratory);
        assert!(!report.claim_eligible);
    }

    #[test]
    fn outputs_above_inputs_are_blocked() {
        let mut request = request();
        if let KitchenFlowObservationV1::Observed { quantity, .. } = &mut request.mass_terms[1].observation {
            quantity.value = 2_000;
        }
        assert_eq!(
            evaluate_kitchen_outcome_accounting(&request).disposition,
            KitchenOutcomeDispositionV1::Blocked
        );
    }

    #[test]
    fn missing_expected_mass_term_is_indeterminate() {
        let mut request = request();
        request.mass_terms.pop();
        let report = evaluate_kitchen_outcome_accounting(&request);
        assert_eq!(report.disposition, KitchenOutcomeDispositionV1::Indeterminate);
        assert_eq!(report.input_mass_nanograms, None);
        assert!(!report.claim_eligible);
    }

    #[test]
    fn duplicate_mass_ids_and_stages_are_blocked() {
        let mut request = request();
        request.mass_terms.push(request.mass_terms[0].clone());
        assert_eq!(
            evaluate_kitchen_outcome_accounting(&request).disposition,
            KitchenOutcomeDispositionV1::Blocked
        );

        let mut request = request();
        request.water_terms.push(request.water_terms[0].clone());
        assert_eq!(
            evaluate_kitchen_outcome_accounting(&request).disposition,
            KitchenOutcomeDispositionV1::Blocked
        );
    }

    #[test]
    fn mass_water_and_energy_units_are_separate_and_exact() {
        assert_eq!(
            mass_nanograms(ExactKitchenMassV1 {
                value: 125,
                decimal_places: 2,
                unit: KitchenMassUnitV1::Gram,
            }).unwrap(),
            1_250_000_000
        );
        assert_eq!(
            water_nanolitres(ExactKitchenWaterV1 {
                value: 125,
                decimal_places: 2,
                unit: KitchenWaterUnitV1::Millilitre,
            }).unwrap(),
            1_250_000
        );
        assert_eq!(
            energy_microwatt_hours(ExactKitchenEnergyV1 {
                value: 125,
                decimal_places: 2,
                unit: KitchenEnergyUnitV1::KilowattHour,
            }).unwrap(),
            1_250_000_000
        );
    }

    #[test]
    fn measured_evidence_requires_calibration() {
        let mut request = request();
        if let KitchenFlowObservationV1::Observed { evidence, .. } = &mut request.mass_terms[0].observation {
            evidence.calibration_ref = None;
        }
        assert_eq!(
            evaluate_kitchen_outcome_accounting(&request).disposition,
            KitchenOutcomeDispositionV1::Blocked
        );
    }

    #[test]
    fn rejected_batch_can_be_complete_without_per_serving_ratios() {
        let mut request = request();
        request.accepted_servings = 0;
        if let KitchenFlowObservationV1::Observed { quantity, .. } = &mut request.mass_terms[1].observation {
            quantity.value = 0;
        }
        let report = evaluate_kitchen_outcome_accounting(&request);
        assert!(report.complete);
        assert_eq!(report.disposition, KitchenOutcomeDispositionV1::CompleteExploratory);
        assert_eq!(report.water_nanolitres_per_accepted_serving, None);
        assert_eq!(report.electricity_microwatt_hours_per_accepted_serving, None);
    }

    #[test]
    fn positive_servings_require_positive_accepted_mass() {
        let mut request = request();
        if let KitchenFlowObservationV1::Observed { quantity, .. } = &mut request.mass_terms[1].observation {
            quantity.value = 0;
        }
        assert_eq!(
            evaluate_kitchen_outcome_accounting(&request).disposition,
            KitchenOutcomeDispositionV1::Blocked
        );
    }

    #[test]
    fn mass_balance_ratio_overflow_is_reported_not_wrapped() {
        let mut request = request();
        request.expected_mass_term_ids.clear();
        request.mass_terms.clear();
        for index in 0..19 {
            let id = format!("input-{index}");
            request.expected_mass_term_ids.push(id.clone());
            request.mass_terms.push(mass_term(
                &id,
                KitchenMassDirectionV1::Input,
                KitchenMassCategoryV1::Ingredient,
                ExactKitchenMassV1 {
                    value: u64::MAX,
                    decimal_places: 0,
                    unit: KitchenMassUnitV1::Kilogram,
                },
            ));
        }
        request.expected_mass_term_ids.push("accepted".to_owned());
        request.mass_terms.push(mass_term(
            "accepted",
            KitchenMassDirectionV1::Output,
            KitchenMassCategoryV1::AcceptedEdible,
            ExactKitchenMassV1 {
                value: 1,
                decimal_places: 0,
                unit: KitchenMassUnitV1::Gram,
            },
        ));
        let report = evaluate_kitchen_outcome_accounting(&request);
        assert_eq!(report.disposition, KitchenOutcomeDispositionV1::Indeterminate);
        assert!(report.issues.iter().any(|issue| {
            issue.code == KitchenOutcomeIssueCodeV1::ArithmeticOverflow
        }));
    }
}
