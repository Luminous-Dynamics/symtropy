// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed, authority-free validation of AERO engineering claim specifications.
//!
//! A successful result means only that a claim is formally well-specified
//! under this bounded profile. It does not establish that the proposition is
//! scientifically correct, physically true, safe, accepted, or authorized.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CLAIM_SPECIFICATION_SCHEMA_V0_1: &str = "AeroClaimSpecificationV0_1";
pub const CLAIM_SPECIFICATION_CANONICALIZATION_V0_1: &str = "rust-serde-json-struct-order-v1";

/// Closed unit vocabulary for the first scalar-claim profile.
///
/// No implicit conversions are performed. The predicate unit must exactly
/// match the measurand unit. A future conversion profile must carry an
/// independently verifiable conversion rule/receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum UnitCodeV0_1 {
    #[serde(rename = "1")]
    Dimensionless,
    #[serde(rename = "Pa")]
    Pascal,
    #[serde(rename = "MPa")]
    Megapascal,
    #[serde(rename = "N")]
    Newton,
    #[serde(rename = "m")]
    Meter,
    #[serde(rename = "s")]
    Second,
    #[serde(rename = "kg")]
    Kilogram,
    #[serde(rename = "K")]
    Kelvin,
    #[serde(rename = "m/s")]
    MeterPerSecond,
    #[serde(rename = "m/s^2")]
    MeterPerSecondSquared,
    #[serde(rename = "J")]
    Joule,
    #[serde(rename = "W")]
    Watt,
}

impl UnitCodeV0_1 {
    /// Physical dimension class for the closed v0.1 unit vocabulary.
    pub fn dimension(self) -> &'static str {
        match self {
            Self::Dimensionless => "dimensionless",
            Self::Pascal | Self::Megapascal => "pressure",
            Self::Newton => "force",
            Self::Meter => "length",
            Self::Second => "time",
            Self::Kilogram => "mass",
            Self::Kelvin => "temperature",
            Self::MeterPerSecond => "velocity",
            Self::MeterPerSecondSquared => "acceleration",
            Self::Joule => "energy",
            Self::Watt => "power",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationV0_1 {
    EachUnit,
    Maximum,
    Minimum,
    Mean,
    P95,
    P99,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperatorV0_1 {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// Exact rational value. Floating point is intentionally excluded from v0.1.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RationalV0_1 {
    pub numerator: i64,
    pub denominator: u64,
}

impl RationalV0_1 {
    /// The representation is canonical only when denominator > 0 and the
    /// numerator/denominator pair is reduced to lowest terms.
    pub fn is_canonical(self) -> bool {
        if self.denominator == 0 {
            return false;
        }
        gcd(self.numerator.unsigned_abs(), self.denominator) == 1
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasurandV0_1 {
    pub measurand_id: String,
    pub definition: String,
    pub unit_code: UnitCodeV0_1,
    pub aggregation: AggregationV0_1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredicateV0_1 {
    pub operator: ComparisonOperatorV0_1,
    pub threshold: RationalV0_1,
    pub unit_code: UnitCodeV0_1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PopulationQuantifierV0_1 {
    EveryRequiredUnit,
    WorstCaseOverDeclaredPopulation,
    PopulationConfidenceBound,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredStratumV0_1 {
    pub stratum_id: String,
    pub dimension_ref: String,
    pub level_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationScopeV0_1 {
    pub population_ref: String,
    pub sampling_unit_ref: String,
    pub population_manifest_sha256: String,
    pub quantifier: PopulationQuantifierV0_1,
    /// Parts per million; required only for POPULATION_CONFIDENCE_BOUND.
    pub confidence_level_ppm: Option<u32>,
    /// At least one explicit stratum is required, including an explicit
    /// "unstratified" level when no further partition is intended.
    pub required_strata: Vec<RequiredStratumV0_1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintyTreatmentV0_1 {
    PointEstimate,
    DeterministicUpperBound,
    DeterministicLowerBound,
    StatisticalUpperConfidenceBound,
    StatisticalLowerConfidenceBound,
    IntervalEnclosure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UncertaintyRuleV0_1 {
    pub treatment: UncertaintyTreatmentV0_1,
    pub method_ref: String,
    /// Parts per million; required only for statistical confidence bounds.
    pub confidence_level_ppm: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidityIntervalV0_1 {
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
}

/// A single scalar engineering proposition with explicit semantics.
///
/// v0.1 intentionally supports one scalar predicate. Compound AND/OR claims,
/// derived quantities, unit conversion, probabilistic model evaluation and
/// physical decision rules require separate versioned profiles.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimSpecificationV0_1 {
    pub schema: String,
    pub specification_id: String,
    pub claim_id: String,
    pub subject_ref: String,
    pub configuration_ref: String,
    pub operating_envelope_ref: String,
    pub intended_use_ref: String,
    pub validity: ValidityIntervalV0_1,
    pub measurand: MeasurandV0_1,
    pub predicate: PredicateV0_1,
    pub population: PopulationScopeV0_1,
    pub uncertainty: UncertaintyRuleV0_1,
    pub required_evidence_classes: Vec<String>,
    pub assumptions: Vec<String>,
    pub exclusions: Vec<String>,
    pub rationale_ref: String,
    pub verification_method_ref: String,
    pub acceptance_authority_role_ref: String,
    /// Human-readable limits that must travel with the specification.
    pub limitations: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ClaimSpecificationDispositionV0_1 {
    FormallyWellSpecified,
    Incomplete,
    Rejected,
    UnsupportedProfile,
}

impl ClaimSpecificationDispositionV0_1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FormallyWellSpecified => "FORMALLY_WELL_SPECIFIED",
            Self::Incomplete => "INCOMPLETE",
            Self::Rejected => "REJECTED",
            Self::UnsupportedProfile => "UNSUPPORTED_PROFILE",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClaimSpecificationAssessmentV0_1 {
    pub disposition: ClaimSpecificationDispositionV0_1,
    pub reason_codes: Vec<String>,
    pub physical_truth_established: bool,
    pub engineering_acceptance_granted: bool,
    pub execution_authority_granted: bool,
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value == value.trim()
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_description(value: &str) -> bool {
    !value.trim().is_empty() && value == value.trim()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn unique_identifiers(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .all(|value| valid_identifier(value) && seen.insert(value.as_str()))
}

impl ClaimSpecificationV0_1 {
    /// Validate the specification's formal completeness and internal
    /// consistency. This deliberately does not evaluate physical truth.
    pub fn assess(&self) -> ClaimSpecificationAssessmentV0_1 {
        let mut reasons = BTreeSet::new();

        if self.schema != CLAIM_SPECIFICATION_SCHEMA_V0_1 {
            reasons.insert("CLAIM_SPECIFICATION_SCHEMA_UNSUPPORTED".to_owned());
        }

        for (code, value) in [
            ("SPECIFICATION_ID", self.specification_id.as_str()),
            ("CLAIM_ID", self.claim_id.as_str()),
            ("SUBJECT_REF", self.subject_ref.as_str()),
            ("CONFIGURATION_REF", self.configuration_ref.as_str()),
            (
                "OPERATING_ENVELOPE_REF",
                self.operating_envelope_ref.as_str(),
            ),
            ("INTENDED_USE_REF", self.intended_use_ref.as_str()),
            ("MEASURAND_ID", self.measurand.measurand_id.as_str()),
            ("POPULATION_REF", self.population.population_ref.as_str()),
            (
                "SAMPLING_UNIT_REF",
                self.population.sampling_unit_ref.as_str(),
            ),
            (
                "UNCERTAINTY_METHOD_REF",
                self.uncertainty.method_ref.as_str(),
            ),
            ("RATIONALE_REF", self.rationale_ref.as_str()),
            (
                "VERIFICATION_METHOD_REF",
                self.verification_method_ref.as_str(),
            ),
            (
                "ACCEPTANCE_AUTHORITY_ROLE_REF",
                self.acceptance_authority_role_ref.as_str(),
            ),
        ] {
            if !valid_identifier(value) {
                reasons.insert(format!(
                    "CLAIM_SPECIFICATION_{code}_MISSING_OR_NONCANONICAL"
                ));
            }
        }

        if !valid_description(&self.measurand.definition) {
            reasons.insert("CLAIM_SPECIFICATION_MEASURAND_DEFINITION_MISSING".to_owned());
        }

        if self.validity.effective_from_unix_ms >= self.validity.effective_until_unix_ms {
            reasons.insert("CLAIM_SPECIFICATION_VALIDITY_INTERVAL_INVALID".to_owned());
        }

        if self.measurand.unit_code != self.predicate.unit_code {
            reasons.insert("CLAIM_SPECIFICATION_PREDICATE_UNIT_MISMATCH".to_owned());
        }

        if !self.predicate.threshold.is_canonical() {
            reasons.insert("CLAIM_SPECIFICATION_THRESHOLD_RATIONAL_NONCANONICAL".to_owned());
        }

        if !valid_sha256(&self.population.population_manifest_sha256) {
            reasons.insert("CLAIM_SPECIFICATION_POPULATION_MANIFEST_DIGEST_INVALID".to_owned());
        }

        if self.population.required_strata.is_empty() {
            reasons.insert("CLAIM_SPECIFICATION_REQUIRED_STRATA_EMPTY".to_owned());
        }
        let mut stratum_ids = BTreeSet::new();
        for stratum in &self.population.required_strata {
            if !valid_identifier(&stratum.stratum_id)
                || !valid_identifier(&stratum.dimension_ref)
                || !valid_identifier(&stratum.level_ref)
            {
                reasons.insert(
                    "CLAIM_SPECIFICATION_STRATUM_IDENTITY_MISSING_OR_NONCANONICAL".to_owned(),
                );
            }
            if !stratum_ids.insert(stratum.stratum_id.as_str()) {
                reasons.insert("CLAIM_SPECIFICATION_STRATUM_ID_DUPLICATE".to_owned());
            }
        }

        if self.required_evidence_classes.is_empty() {
            reasons.insert("CLAIM_SPECIFICATION_EVIDENCE_CLASSES_EMPTY".to_owned());
        } else if !unique_identifiers(&self.required_evidence_classes) {
            reasons.insert("CLAIM_SPECIFICATION_EVIDENCE_CLASS_ID_INVALID_OR_DUPLICATE".to_owned());
        }

        if !unique_identifiers(&self.assumptions) {
            reasons.insert("CLAIM_SPECIFICATION_ASSUMPTION_ID_INVALID_OR_DUPLICATE".to_owned());
        }
        if !unique_identifiers(&self.exclusions) {
            reasons.insert("CLAIM_SPECIFICATION_EXCLUSION_ID_INVALID_OR_DUPLICATE".to_owned());
        }
        if self.limitations.is_empty()
            || self.limitations.iter().any(|item| !valid_description(item))
        {
            reasons.insert("CLAIM_SPECIFICATION_LIMITATIONS_MISSING_OR_INVALID".to_owned());
        }

        let population_confidence_required =
            self.population.quantifier == PopulationQuantifierV0_1::PopulationConfidenceBound;
        match (
            population_confidence_required,
            self.population.confidence_level_ppm,
        ) {
            (true, Some(value)) if (1..1_000_000).contains(&value) => {}
            (true, _) => {
                reasons.insert(
                    "CLAIM_SPECIFICATION_POPULATION_CONFIDENCE_LEVEL_MISSING_OR_INVALID".to_owned(),
                );
            }
            (false, None) => {}
            (false, Some(_)) => {
                reasons.insert(
                    "CLAIM_SPECIFICATION_POPULATION_CONFIDENCE_LEVEL_UNEXPECTED".to_owned(),
                );
            }
        }

        let uncertainty_confidence_required = matches!(
            self.uncertainty.treatment,
            UncertaintyTreatmentV0_1::StatisticalUpperConfidenceBound
                | UncertaintyTreatmentV0_1::StatisticalLowerConfidenceBound
        );
        match (
            uncertainty_confidence_required,
            self.uncertainty.confidence_level_ppm,
        ) {
            (true, Some(value)) if (1..1_000_000).contains(&value) => {}
            (true, _) => {
                reasons.insert(
                    "CLAIM_SPECIFICATION_UNCERTAINTY_CONFIDENCE_LEVEL_MISSING_OR_INVALID"
                        .to_owned(),
                );
            }
            (false, None) => {}
            (false, Some(_)) => {
                reasons.insert(
                    "CLAIM_SPECIFICATION_UNCERTAINTY_CONFIDENCE_LEVEL_UNEXPECTED".to_owned(),
                );
            }
        }

        let unsupported = reasons
            .iter()
            .any(|reason| reason == "CLAIM_SPECIFICATION_SCHEMA_UNSUPPORTED");
        let incomplete = reasons.iter().any(|reason| {
            reason.ends_with("_MISSING")
                || reason.ends_with("_EMPTY")
                || reason.contains("_MISSING_OR_")
        });

        let disposition = if unsupported {
            ClaimSpecificationDispositionV0_1::UnsupportedProfile
        } else if incomplete {
            ClaimSpecificationDispositionV0_1::Incomplete
        } else if reasons.is_empty() {
            ClaimSpecificationDispositionV0_1::FormallyWellSpecified
        } else {
            ClaimSpecificationDispositionV0_1::Rejected
        };

        ClaimSpecificationAssessmentV0_1 {
            disposition,
            reason_codes: reasons.into_iter().collect(),
            physical_truth_established: false,
            engineering_acceptance_granted: false,
            execution_authority_granted: false,
        }
    }

    /// Verify the typed semantic commitment against an independently supplied digest.
    ///
    /// The caller must retain the raw-payload digest separately. A matching
    /// semantic digest proves only equality under this typed profile.
    pub fn verify_semantic_sha256(&self, expected: &str) -> Result<(), Vec<String>> {
        if !valid_sha256(expected) {
            return Err(vec![
                "CLAIM_SPECIFICATION_SEMANTIC_DIGEST_INVALID".to_owned(),
            ]);
        }
        let actual = self.compute_semantic_sha256()?;
        if actual != expected {
            return Err(vec![
                "CLAIM_SPECIFICATION_SEMANTIC_DIGEST_MISMATCH".to_owned(),
            ]);
        }
        Ok(())
    }

    /// Compute the typed semantic commitment for a well-specified object.
    ///
    /// This uses the versioned Rust/Serde struct-order profile, not raw source
    /// bytes and not RFC 8785/JCS. The raw payload digest and this normalized
    /// semantic commitment are different objects and must not be substituted
    /// for one another.
    pub fn compute_semantic_sha256(&self) -> Result<String, Vec<String>> {
        let assessment = self.assess();
        if assessment.disposition != ClaimSpecificationDispositionV0_1::FormallyWellSpecified {
            return Err(assessment.reason_codes);
        }
        let envelope = SemanticEnvelopeV0_1 {
            schema: "AeroClaimSpecificationSemanticCommitmentV1",
            canonicalization_profile: CLAIM_SPECIFICATION_CANONICALIZATION_V0_1,
            specification: self,
        };
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|_| vec!["CLAIM_SPECIFICATION_SERIALIZATION_FAILED".to_owned()])?;
        Ok(sha256_hex(&bytes))
    }
}

#[derive(Serialize)]
struct SemanticEnvelopeV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    specification: &'a ClaimSpecificationV0_1,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_specification() -> ClaimSpecificationV0_1 {
        ClaimSpecificationV0_1 {
            schema: CLAIM_SPECIFICATION_SCHEMA_V0_1.to_owned(),
            specification_id: "aero:claim-spec:wing-panel-strength-001".to_owned(),
            claim_id: "aero:claim:wing-panel-strength-001".to_owned(),
            subject_ref: "part:wing-panel-synthetic-001".to_owned(),
            configuration_ref: "configuration:wing-r17-synthetic".to_owned(),
            operating_envelope_ref: "envelope:transport-normal-synthetic".to_owned(),
            intended_use_ref: "intended-use:structural-strength-screening".to_owned(),
            validity: ValidityIntervalV0_1 {
                effective_from_unix_ms: 1_795_514_399_900,
                effective_until_unix_ms: 1_798_668_800_000,
            },
            measurand: MeasurandV0_1 {
                measurand_id: "measurand:maximum-principal-stress".to_owned(),
                definition: "Maximum principal tensile stress over the declared population".to_owned(),
                unit_code: UnitCodeV0_1::Megapascal,
                aggregation: AggregationV0_1::Maximum,
            },
            predicate: PredicateV0_1 {
                operator: ComparisonOperatorV0_1::LessThanOrEqual,
                threshold: RationalV0_1 {
                    numerator: 300,
                    denominator: 1,
                },
                unit_code: UnitCodeV0_1::Megapascal,
            },
            population: PopulationScopeV0_1 {
                population_ref: "population:wing-panel-candidates-synthetic".to_owned(),
                sampling_unit_ref: "sampling-unit:physical-panel".to_owned(),
                population_manifest_sha256: "a".repeat(64),
                quantifier: PopulationQuantifierV0_1::WorstCaseOverDeclaredPopulation,
                confidence_level_ppm: None,
                required_strata: vec![
                    RequiredStratumV0_1 {
                        stratum_id: "stratum:temperature-ambient".to_owned(),
                        dimension_ref: "dimension:temperature".to_owned(),
                        level_ref: "level:ambient".to_owned(),
                    },
                    RequiredStratumV0_1 {
                        stratum_id: "stratum:limit-load".to_owned(),
                        dimension_ref: "dimension:load-case".to_owned(),
                        level_ref: "level:limit-load".to_owned(),
                    },
                ],
            },
            uncertainty: UncertaintyRuleV0_1 {
                treatment: UncertaintyTreatmentV0_1::DeterministicUpperBound,
                method_ref: "method:synthetic-upper-bound-v1".to_owned(),
                confidence_level_ppm: None,
            },
            required_evidence_classes: vec![
                "evidence-class:material-characterization".to_owned(),
                "evidence-class:structural-test".to_owned(),
                "evidence-class:model-validation".to_owned(),
            ],
            assumptions: vec!["assumption:synthetic-boundary-conditions".to_owned()],
            exclusions: vec!["exclusion:damage-growth".to_owned()],
            rationale_ref: "rationale:synthetic-wing-panel-strength".to_owned(),
            verification_method_ref: "method:independent-structural-review".to_owned(),
            acceptance_authority_role_ref: "authority-role:delegated-engineering-authority".to_owned(),
            limitations: vec![
                "Synthetic example only; no physical test or model result is represented.".to_owned(),
                "Formal specification does not establish the selected threshold or assumptions as correct.".to_owned(),
            ],
        }
    }

    #[test]
    fn checked_in_json_fixture_deserializes_and_is_formally_well_specified() {
        let specification: ClaimSpecificationV0_1 = serde_json::from_str(include_str!(
            "../../../../docs/research/aviation/examples/aero_claim_specification_v0_1.json"
        ))
        .expect("synthetic claim specification fixture deserializes");
        assert_eq!(
            specification.assess().disposition,
            ClaimSpecificationDispositionV0_1::FormallyWellSpecified
        );
    }

    #[test]
    fn explicit_scalar_claim_is_formally_well_specified() {
        let result = valid_specification().assess();
        assert_eq!(
            result.disposition,
            ClaimSpecificationDispositionV0_1::FormallyWellSpecified
        );
        assert!(!result.physical_truth_established);
        assert!(!result.engineering_acceptance_granted);
        assert!(!result.execution_authority_granted);
    }

    #[test]
    fn threshold_and_measurand_units_must_match_exactly() {
        let mut specification = valid_specification();
        specification.predicate.unit_code = UnitCodeV0_1::Pascal;
        let result = specification.assess();
        assert_eq!(
            result.disposition,
            ClaimSpecificationDispositionV0_1::Rejected
        );
        assert!(
            result
                .reason_codes
                .contains(&"CLAIM_SPECIFICATION_PREDICATE_UNIT_MISMATCH".to_owned())
        );
    }

    #[test]
    fn rational_threshold_must_be_reduced_and_have_nonzero_denominator() {
        let mut zero_denominator = valid_specification();
        zero_denominator.predicate.threshold.denominator = 0;
        assert!(
            zero_denominator
                .assess()
                .reason_codes
                .contains(&"CLAIM_SPECIFICATION_THRESHOLD_RATIONAL_NONCANONICAL".to_owned())
        );

        let mut unreduced = valid_specification();
        unreduced.predicate.threshold = RationalV0_1 {
            numerator: 600,
            denominator: 2,
        };
        assert!(
            unreduced
                .assess()
                .reason_codes
                .contains(&"CLAIM_SPECIFICATION_THRESHOLD_RATIONAL_NONCANONICAL".to_owned())
        );
    }

    #[test]
    fn empty_strata_or_evidence_classes_are_incomplete() {
        let mut specification = valid_specification();
        specification.population.required_strata.clear();
        specification.required_evidence_classes.clear();
        let result = specification.assess();
        assert_eq!(
            result.disposition,
            ClaimSpecificationDispositionV0_1::Incomplete
        );
        assert!(
            result
                .reason_codes
                .contains(&"CLAIM_SPECIFICATION_REQUIRED_STRATA_EMPTY".to_owned())
        );
        assert!(
            result
                .reason_codes
                .contains(&"CLAIM_SPECIFICATION_EVIDENCE_CLASSES_EMPTY".to_owned())
        );
    }

    #[test]
    fn duplicate_stratum_id_is_rejected() {
        let mut specification = valid_specification();
        specification.population.required_strata[1].stratum_id =
            specification.population.required_strata[0]
                .stratum_id
                .clone();
        let result = specification.assess();
        assert_eq!(
            result.disposition,
            ClaimSpecificationDispositionV0_1::Rejected
        );
        assert!(
            result
                .reason_codes
                .contains(&"CLAIM_SPECIFICATION_STRATUM_ID_DUPLICATE".to_owned())
        );
    }

    #[test]
    fn population_confidence_bound_requires_an_explicit_level() {
        let mut specification = valid_specification();
        specification.population.quantifier = PopulationQuantifierV0_1::PopulationConfidenceBound;
        let result = specification.assess();
        assert_eq!(
            result.disposition,
            ClaimSpecificationDispositionV0_1::Incomplete
        );
        assert!(result.reason_codes.contains(
            &"CLAIM_SPECIFICATION_POPULATION_CONFIDENCE_LEVEL_MISSING_OR_INVALID".to_owned()
        ));
    }

    #[test]
    fn statistical_uncertainty_requires_a_confidence_level() {
        let mut specification = valid_specification();
        specification.uncertainty.treatment =
            UncertaintyTreatmentV0_1::StatisticalUpperConfidenceBound;
        let result = specification.assess();
        assert_eq!(
            result.disposition,
            ClaimSpecificationDispositionV0_1::Incomplete
        );
        assert!(result.reason_codes.contains(
            &"CLAIM_SPECIFICATION_UNCERTAINTY_CONFIDENCE_LEVEL_MISSING_OR_INVALID".to_owned()
        ));
    }

    #[test]
    fn validity_must_be_a_nonempty_half_open_interval() {
        let mut specification = valid_specification();
        specification.validity.effective_until_unix_ms =
            specification.validity.effective_from_unix_ms;
        assert!(
            specification
                .assess()
                .reason_codes
                .contains(&"CLAIM_SPECIFICATION_VALIDITY_INTERVAL_INVALID".to_owned())
        );
    }

    #[test]
    fn unsupported_schema_version_is_never_promoted() {
        let mut specification = valid_specification();
        specification.schema = "AeroClaimSpecificationV9".to_owned();
        assert_eq!(
            specification.assess().disposition,
            ClaimSpecificationDispositionV0_1::UnsupportedProfile
        );
    }

    #[test]
    fn semantic_digest_changes_when_the_proposition_or_scope_changes() {
        let specification = valid_specification();
        let baseline = specification
            .compute_semantic_sha256()
            .expect("valid specification has semantic digest");
        assert!(specification.verify_semantic_sha256(&baseline).is_ok());
        assert_eq!(
            specification.verify_semantic_sha256(&"0".repeat(64)),
            Err(vec![
                "CLAIM_SPECIFICATION_SEMANTIC_DIGEST_MISMATCH".to_owned()
            ])
        );

        let mut changed_threshold = specification.clone();
        changed_threshold.predicate.threshold.numerator += 1;
        assert_ne!(
            changed_threshold.compute_semantic_sha256().unwrap(),
            baseline
        );

        let mut changed_subject = specification;
        changed_subject.subject_ref.push_str("-different");
        assert_ne!(changed_subject.compute_semantic_sha256().unwrap(), baseline);
    }

    #[test]
    fn duplicate_required_evidence_classes_are_rejected() {
        let mut specification = valid_specification();
        specification.required_evidence_classes[1] =
            specification.required_evidence_classes[0].clone();
        let result = specification.assess();
        assert_eq!(
            result.disposition,
            ClaimSpecificationDispositionV0_1::Rejected
        );
        assert!(
            result
                .reason_codes
                .contains(&"CLAIM_SPECIFICATION_EVIDENCE_CLASS_ID_INVALID_OR_DUPLICATE".to_owned())
        );
    }
}
