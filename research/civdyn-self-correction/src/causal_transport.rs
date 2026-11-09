// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Known-truth causal effect transport under measured effect modification.
//!
//! This v0.1 reference transports a binary-outcome risk difference from a
//! source randomized experiment to a target population by standardizing
//! source stratum-specific effects to the target modifier distribution.
//! It never imputes effects for target-only strata. Under full source-target
//! support it reports a point estimand; under support failure it reports
//! worst-case identification bounds only when all declared transport
//! assumptions are supported. It is not a general selection-diagram engine.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssumptionDisposition {
    SupportedByDesign,
    SupportedByEvidence,
    Violated,
    Unknown,
}

impl AssumptionDisposition {
    pub const fn name(self) -> &'static str {
        match self {
            Self::SupportedByDesign => "supported_by_design",
            Self::SupportedByEvidence => "supported_by_evidence",
            Self::Violated => "violated",
            Self::Unknown => "unknown",
        }
    }

    const fn is_supported(self) -> bool {
        matches!(self, Self::SupportedByDesign | Self::SupportedByEvidence)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssumptionEvidence {
    pub disposition: AssumptionDisposition,
    /// Digest of the protocol, evidence packet or explicit assumption review.
    /// It is provenance, not cryptographic proof that the assumption is true.
    pub evidence_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalTransportAssumptions {
    /// Source experiment identifies conditional effects (e.g. treatment randomized).
    pub source_internal_validity: AssumptionEvidence,
    /// Conditional potential-outcome distributions/effects are invariant
    /// between source and target after conditioning on the declared modifier.
    pub selection_exchangeability: AssumptionEvidence,
    /// No relevant cross-unit interference under the declared treatment version.
    pub no_interference: AssumptionEvidence,
    /// Source and target treatment/outcome versions have the same meaning.
    pub treatment_version_invariance: AssumptionEvidence,
}

impl CausalTransportAssumptions {
    fn validate(&self) -> Result<(), CausalTransportError> {
        for evidence in [
            &self.source_internal_validity,
            &self.selection_exchangeability,
            &self.no_interference,
            &self.treatment_version_invariance,
        ] {
            if evidence.evidence_digest.trim().is_empty() {
                return Err(CausalTransportError::MissingAssumptionEvidenceDigest);
            }
        }
        Ok(())
    }

    fn all_supported(&self) -> bool {
        [
            self.source_internal_validity.disposition,
            self.selection_exchangeability.disposition,
            self.no_interference.disposition,
            self.treatment_version_invariance.disposition,
        ]
        .into_iter()
        .all(AssumptionDisposition::is_supported)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEffectStratum {
    /// Stable effect-modifier stratum ID, shared with target population data.
    pub modifier_id: String,
    /// Number of source-population units in this stratum.
    pub source_population_count: u64,
    pub control_sample_size: u64,
    pub control_events: u64,
    pub treatment_sample_size: u64,
    pub treatment_events: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPopulationStratum {
    pub modifier_id: String,
    pub target_population_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalTransportInput {
    pub source_data_digest: String,
    pub target_population_digest: String,
    pub protocol_digest: String,
    pub assumptions: CausalTransportAssumptions,
    pub source_strata: Vec<SourceEffectStratum>,
    pub target_strata: Vec<TargetPopulationStratum>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportIdentificationStatus {
    Identified,
    PartiallyIdentified,
    NotIdentified,
}

impl TransportIdentificationStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Identified => "identified",
            Self::PartiallyIdentified => "partially_identified",
            Self::NotIdentified => "not_identified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportIdentificationReason {
    FullSupportAndAssumptionsSupported,
    TargetSupportViolation,
    AssumptionViolationOrUnknown,
}

impl TransportIdentificationReason {
    pub const fn name(self) -> &'static str {
        match self {
            Self::FullSupportAndAssumptionsSupported => "full_support_assumptions_supported",
            Self::TargetSupportViolation => "target_support_violation",
            Self::AssumptionViolationOrUnknown => "assumption_violation_or_unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransportStratumResult {
    pub modifier_id: String,
    pub source_population_count: Option<u64>,
    pub target_population_count: u64,
    pub source_control_sample_size: Option<u64>,
    pub source_control_events: Option<u64>,
    pub source_treatment_sample_size: Option<u64>,
    pub source_treatment_events: Option<u64>,
    pub control_risk: Option<f64>,
    pub treatment_risk: Option<f64>,
    pub conditional_risk_difference: Option<f64>,
    pub target_population_share: f64,
    pub source_support: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CausalTransportResult {
    pub estimand: &'static str,
    pub identification_status: TransportIdentificationStatus,
    pub identification_reason: TransportIdentificationReason,
    /// Source-population ATE (risk difference), using the randomized source weights.
    pub source_ate: f64,
    /// Target-population ATE only when point-identified under declared assumptions.
    pub target_ate_point: Option<f64>,
    /// Worst-case identification bounds under target support failure only.
    /// These do not include sampling uncertainty.
    pub target_ate_lower: Option<f64>,
    pub target_ate_upper: Option<f64>,
    pub source_population_total: u64,
    pub target_population_total: u64,
    pub supported_target_population_count: u64,
    pub unsupported_target_population_count: u64,
    pub supported_target_mass: f64,
    pub unsupported_target_mass: f64,
    pub assumptions: CausalTransportAssumptions,
    pub source_data_digest: String,
    pub target_population_digest: String,
    pub protocol_digest: String,
    pub strata: Vec<TransportStratumResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CausalTransportError {
    MissingDataDigest,
    MissingAssumptionEvidenceDigest,
    EmptySourceStrata,
    EmptyTargetStrata,
    ZeroSourcePopulation,
    ZeroTargetPopulation,
    CountOverflow,
    UnsupportedTargetMassBelowFloatingPrecision,
    EmptyModifierId,
    DuplicateSourceModifier { modifier_id: String },
    DuplicateTargetModifier { modifier_id: String },
    InvalidSourceStratum { modifier_id: String },
    InvalidTargetStratum { modifier_id: String },
    NonFiniteResult,
}

impl fmt::Display for CausalTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CausalTransportError {}


fn validate_source_strata(
    strata: &[SourceEffectStratum],
) -> Result<(BTreeMap<&str, &SourceEffectStratum>, u64), CausalTransportError> {
    if strata.is_empty() {
        return Err(CausalTransportError::EmptySourceStrata);
    }

    let mut indexed = BTreeMap::new();
    let mut total = 0_u64;
    for stratum in strata {
        if stratum.modifier_id.trim().is_empty() {
            return Err(CausalTransportError::EmptyModifierId);
        }
        if stratum.source_population_count == 0
            || stratum.control_sample_size == 0
            || stratum.treatment_sample_size == 0
            || stratum.control_events > stratum.control_sample_size
            || stratum.treatment_events > stratum.treatment_sample_size
        {
            return Err(CausalTransportError::InvalidSourceStratum {
                modifier_id: stratum.modifier_id.clone(),
            });
        }
        if indexed
            .insert(stratum.modifier_id.as_str(), stratum)
            .is_some()
        {
            return Err(CausalTransportError::DuplicateSourceModifier {
                modifier_id: stratum.modifier_id.clone(),
            });
        }
        total = total
            .checked_add(stratum.source_population_count)
            .ok_or(CausalTransportError::CountOverflow)?;
    }
    if total == 0 {
        return Err(CausalTransportError::ZeroSourcePopulation);
    }
    Ok((indexed, total))
}

fn validate_target_strata(
    strata: &[TargetPopulationStratum],
) -> Result<(BTreeMap<&str, &TargetPopulationStratum>, u64), CausalTransportError> {
    if strata.is_empty() {
        return Err(CausalTransportError::EmptyTargetStrata);
    }

    let mut indexed = BTreeMap::new();
    let mut total = 0_u64;
    for stratum in strata {
        if stratum.modifier_id.trim().is_empty() {
            return Err(CausalTransportError::EmptyModifierId);
        }
        if stratum.target_population_count == 0 {
            return Err(CausalTransportError::InvalidTargetStratum {
                modifier_id: stratum.modifier_id.clone(),
            });
        }
        if indexed
            .insert(stratum.modifier_id.as_str(), stratum)
            .is_some()
        {
            return Err(CausalTransportError::DuplicateTargetModifier {
                modifier_id: stratum.modifier_id.clone(),
            });
        }
        total = total
            .checked_add(stratum.target_population_count)
            .ok_or(CausalTransportError::CountOverflow)?;
    }
    if total == 0 {
        return Err(CausalTransportError::ZeroTargetPopulation);
    }
    Ok((indexed, total))
}

/// Transport a binary-outcome risk difference by standardizing randomized
/// source stratum-specific effects to the target modifier distribution.
///
/// If a target modifier stratum is absent from the source experiment, and all
/// other assumptions are supported, the result is partially identified. The
/// missing stratum's risk difference is bounded by [-1, 1], yielding sharp
/// worst-case bounds conditional on the observed supported-stratum effects.
/// No target-only effect is imputed.
pub fn assess_risk_difference_transport(
    input: &CausalTransportInput,
) -> Result<CausalTransportResult, CausalTransportError> {
    if input.source_data_digest.trim().is_empty()
        || input.target_population_digest.trim().is_empty()
        || input.protocol_digest.trim().is_empty()
    {
        return Err(CausalTransportError::MissingDataDigest);
    }
    input.assumptions.validate()?;

    let (source_by_modifier, source_total) = validate_source_strata(&input.source_strata)?;
    let (target_by_modifier, target_total) = validate_target_strata(&input.target_strata)?;

    let mut source_ate = 0.0;
    for source in &input.source_strata {
        let control_risk = source.control_events as f64 / source.control_sample_size as f64;
        let treatment_risk = source.treatment_events as f64 / source.treatment_sample_size as f64;
        let source_share = source.source_population_count as f64 / source_total as f64;
        source_ate += source_share * (treatment_risk - control_risk);
    }

    let mut supported_target_population_count = 0_u64;
    let mut target_weighted_effect = 0.0;
    let mut strata = Vec::with_capacity(input.target_strata.len());

    for target in &input.target_strata {
        let target_share = target.target_population_count as f64 / target_total as f64;
        if let Some(source) = source_by_modifier.get(target.modifier_id.as_str()) {
            let control_risk = source.control_events as f64 / source.control_sample_size as f64;
            let treatment_risk = source.treatment_events as f64 / source.treatment_sample_size as f64;
            let conditional_effect = treatment_risk - control_risk;
            supported_target_population_count = supported_target_population_count
                .checked_add(target.target_population_count)
                .ok_or(CausalTransportError::CountOverflow)?;
            target_weighted_effect += target_share * conditional_effect;
            strata.push(TransportStratumResult {
                modifier_id: target.modifier_id.clone(),
                source_population_count: Some(source.source_population_count),
                target_population_count: target.target_population_count,
                source_control_sample_size: Some(source.control_sample_size),
                source_control_events: Some(source.control_events),
                source_treatment_sample_size: Some(source.treatment_sample_size),
                source_treatment_events: Some(source.treatment_events),
                control_risk: Some(control_risk),
                treatment_risk: Some(treatment_risk),
                conditional_risk_difference: Some(conditional_effect),
                target_population_share: target_share,
                source_support: true,
            });
        } else {
            strata.push(TransportStratumResult {
                modifier_id: target.modifier_id.clone(),
                source_population_count: None,
                target_population_count: target.target_population_count,
                source_control_sample_size: None,
                source_control_events: None,
                source_treatment_sample_size: None,
                source_treatment_events: None,
                control_risk: None,
                treatment_risk: None,
                conditional_risk_difference: None,
                target_population_share: target_share,
                source_support: false,
            });
        }
    }

    let unsupported_target_population_count = target_total
        .checked_sub(supported_target_population_count)
        .ok_or(CausalTransportError::CountOverflow)?;
    let supported_target_mass =
        supported_target_population_count as f64 / target_total as f64;
    let unsupported_target_mass =
        unsupported_target_population_count as f64 / target_total as f64;
    let target_support_complete = unsupported_target_population_count == 0;
    if unsupported_target_population_count > 0
        && unsupported_target_mass <= f64::EPSILON
    {
        return Err(CausalTransportError::UnsupportedTargetMassBelowFloatingPrecision);
    }
    let assumptions_supported = input.assumptions.all_supported();

    let (identification_status, identification_reason, target_ate_point, target_ate_lower, target_ate_upper) =
        if !assumptions_supported {
            (
                TransportIdentificationStatus::NotIdentified,
                TransportIdentificationReason::AssumptionViolationOrUnknown,
                None,
                None,
                None,
            )
        } else if target_support_complete {
            (
                TransportIdentificationStatus::Identified,
                TransportIdentificationReason::FullSupportAndAssumptionsSupported,
                Some(target_weighted_effect),
                None,
                None,
            )
        } else {
            // For binary outcomes, each unsupported conditional risk difference
            // lies in [-1, 1]. These are identification bounds, not confidence
            // intervals and do not include sampling uncertainty.
            (
                TransportIdentificationStatus::PartiallyIdentified,
                TransportIdentificationReason::TargetSupportViolation,
                None,
                Some(target_weighted_effect - unsupported_target_mass),
                Some(target_weighted_effect + unsupported_target_mass),
            )
        };

    if !source_ate.is_finite()
        || !supported_target_mass.is_finite()
        || !unsupported_target_mass.is_finite()
        || target_ate_point.is_some_and(|value| !value.is_finite())
        || target_ate_lower.is_some_and(|value| !value.is_finite())
        || target_ate_upper.is_some_and(|value| !value.is_finite())
    {
        return Err(CausalTransportError::NonFiniteResult);
    }

    Ok(CausalTransportResult {
        estimand: "target_average_treatment_effect_risk_difference",
        identification_status,
        identification_reason,
        source_ate,
        target_ate_point,
        target_ate_lower,
        target_ate_upper,
        source_population_total: source_total,
        target_population_total: target_total,
        supported_target_population_count,
        unsupported_target_population_count,
        supported_target_mass,
        unsupported_target_mass,
        assumptions: input.assumptions.clone(),
        source_data_digest: input.source_data_digest.clone(),
        target_population_digest: input.target_population_digest.clone(),
        protocol_digest: input.protocol_digest.clone(),
        strata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supported_assumptions() -> CausalTransportAssumptions {
        let evidence = |disposition| AssumptionEvidence {
            disposition,
            evidence_digest: "sha256-assumption-review".to_string(),
        };
        CausalTransportAssumptions {
            source_internal_validity: evidence(AssumptionDisposition::SupportedByDesign),
            selection_exchangeability: evidence(AssumptionDisposition::SupportedByEvidence),
            no_interference: evidence(AssumptionDisposition::SupportedByDesign),
            treatment_version_invariance: evidence(AssumptionDisposition::SupportedByEvidence),
        }
    }

    fn source_strata() -> Vec<SourceEffectStratum> {
        vec![
            SourceEffectStratum {
                modifier_id: "low-infrastructure".to_string(),
                source_population_count: 80,
                control_sample_size: 40,
                control_events: 4,
                treatment_sample_size: 40,
                treatment_events: 12,
            },
            SourceEffectStratum {
                modifier_id: "high-infrastructure".to_string(),
                source_population_count: 20,
                control_sample_size: 10,
                control_events: 5,
                treatment_sample_size: 10,
                treatment_events: 8,
            },
        ]
    }

    fn input(target_strata: Vec<TargetPopulationStratum>) -> CausalTransportInput {
        CausalTransportInput {
            source_data_digest: "sha256-source-rct".to_string(),
            target_population_digest: "sha256-target-population".to_string(),
            protocol_digest: "sha256-transport-protocol".to_string(),
            assumptions: supported_assumptions(),
            source_strata: source_strata(),
            target_strata,
        }
    }

    fn full_support_target() -> Vec<TargetPopulationStratum> {
        vec![
            TargetPopulationStratum {
                modifier_id: "low-infrastructure".to_string(),
                target_population_count: 20,
            },
            TargetPopulationStratum {
                modifier_id: "high-infrastructure".to_string(),
                target_population_count: 80,
            },
        ]
    }

    #[test]
    fn standardizes_source_rct_effects_to_target_modifier_distribution() {
        let result = assess_risk_difference_transport(&input(full_support_target()))
            .expect("identified target transport");
        assert_eq!(result.identification_status, TransportIdentificationStatus::Identified);
        assert_eq!(
            result.identification_reason,
            TransportIdentificationReason::FullSupportAndAssumptionsSupported
        );
        assert!((result.source_ate - 0.22).abs() < 1e-12);
        assert!((result.target_ate_point.expect("identified target ATE") - 0.28).abs() < 1e-12);
        assert_eq!(result.target_ate_lower, None);
        assert_eq!(result.target_ate_upper, None);
        assert!((result.supported_target_mass - 1.0).abs() < 1e-12);
        assert_eq!(result.unsupported_target_mass, 0.0);
    }

    #[test]
    fn target_only_modifier_stratum_returns_partial_identification_bounds() {
        let target = vec![
            TargetPopulationStratum {
                modifier_id: "low-infrastructure".to_string(),
                target_population_count: 20,
            },
            TargetPopulationStratum {
                modifier_id: "high-infrastructure".to_string(),
                target_population_count: 60,
            },
            TargetPopulationStratum {
                modifier_id: "remote-isolated".to_string(),
                target_population_count: 20,
            },
        ];
        let result = assess_risk_difference_transport(&input(target))
            .expect("partial target identification");
        assert_eq!(
            result.identification_status,
            TransportIdentificationStatus::PartiallyIdentified
        );
        assert_eq!(
            result.identification_reason,
            TransportIdentificationReason::TargetSupportViolation
        );
        assert_eq!(result.target_ate_point, None);
        assert!((result.supported_target_mass - 0.8).abs() < 1e-12);
        assert!((result.unsupported_target_mass - 0.2).abs() < 1e-12);
        assert!((result.target_ate_lower.expect("lower bound") - 0.02).abs() < 1e-12);
        assert!((result.target_ate_upper.expect("upper bound") - 0.42).abs() < 1e-12);
        let unsupported = result
            .strata
            .iter()
            .find(|stratum| stratum.modifier_id == "remote-isolated")
            .expect("target-only stratum");
        assert!(!unsupported.source_support);
        assert_eq!(unsupported.conditional_risk_difference, None);
    }

    #[test]
    fn violated_or_unknown_assumptions_block_point_and_partial_transport_claims() {
        let mut violated = input(full_support_target());
        violated.assumptions.selection_exchangeability.disposition =
            AssumptionDisposition::Violated;
        let result = assess_risk_difference_transport(&violated)
            .expect("nonidentified result is reportable");
        assert_eq!(result.identification_status, TransportIdentificationStatus::NotIdentified);
        assert_eq!(
            result.identification_reason,
            TransportIdentificationReason::AssumptionViolationOrUnknown
        );
        assert_eq!(result.target_ate_point, None);
        assert_eq!(result.target_ate_lower, None);
        assert_eq!(result.target_ate_upper, None);

        let mut unknown = input(full_support_target());
        unknown.assumptions.no_interference.disposition = AssumptionDisposition::Unknown;
        let result = assess_risk_difference_transport(&unknown)
            .expect("unknown interference assumption blocks transport");
        assert_eq!(result.identification_status, TransportIdentificationStatus::NotIdentified);
    }

    #[test]
    fn source_internal_validity_failure_blocks_transport() {
        let mut candidate = input(full_support_target());
        candidate.assumptions.source_internal_validity.disposition =
            AssumptionDisposition::Unknown;
        let result = assess_risk_difference_transport(&candidate)
            .expect("invalid source study remains a nonidentified report");
        assert_eq!(result.identification_status, TransportIdentificationStatus::NotIdentified);
        assert_eq!(result.target_ate_point, None);
    }

    #[test]
    fn does_not_round_tiny_target_only_strata_into_full_support() {
        let mut candidate = input(full_support_target());
        candidate.target_strata[0].target_population_count = u64::MAX - 1;
        candidate.target_strata[1].target_population_count = 0;
        candidate.target_strata.push(TargetPopulationStratum {
            modifier_id: "remote-isolated".to_string(),
            target_population_count: 1,
        });
        assert_eq!(
            assess_risk_difference_transport(&candidate).unwrap_err(),
            CausalTransportError::InvalidTargetStratum {
                modifier_id: "high-infrastructure".to_string()
            }
        );

        let mut candidate = input(vec![
            TargetPopulationStratum {
                modifier_id: "low-infrastructure".to_string(),
                target_population_count: u64::MAX - 1,
            },
            TargetPopulationStratum {
                modifier_id: "remote-isolated".to_string(),
                target_population_count: 1,
            },
        ]);
        assert_eq!(
            assess_risk_difference_transport(&candidate).unwrap_err(),
            CausalTransportError::UnsupportedTargetMassBelowFloatingPrecision
        );
        candidate.target_strata[0].target_population_count = u64::MAX / 2;
        candidate.target_strata[1].target_population_count = u64::MAX / 2;
        let result = assess_risk_difference_transport(&candidate)
            .expect("representable unsupported target mass remains partially identified");
        assert_eq!(
            result.identification_status,
            TransportIdentificationStatus::PartiallyIdentified
        );
        assert_eq!(result.unsupported_target_population_count, u64::MAX / 2);
    }

    #[test]
    fn rejects_population_count_overflow() {
        let mut source_overflow = input(full_support_target());
        source_overflow.source_strata[0].source_population_count = u64::MAX;
        source_overflow.source_strata[1].source_population_count = 1;
        assert_eq!(
            assess_risk_difference_transport(&source_overflow).unwrap_err(),
            CausalTransportError::CountOverflow
        );

        let mut target_overflow = input(full_support_target());
        target_overflow.target_strata[0].target_population_count = u64::MAX;
        target_overflow.target_strata[1].target_population_count = 1;
        assert_eq!(
            assess_risk_difference_transport(&target_overflow).unwrap_err(),
            CausalTransportError::CountOverflow
        );
    }

    #[test]
    fn rejects_duplicate_modifier_ids_and_invalid_binary_counts() {
        let mut duplicate = input(full_support_target());
        duplicate.source_strata.push(duplicate.source_strata[0].clone());
        assert!(matches!(
            assess_risk_difference_transport(&duplicate),
            Err(CausalTransportError::DuplicateSourceModifier { .. })
        ));

        let mut invalid = input(full_support_target());
        invalid.source_strata[0].control_events = invalid.source_strata[0].control_sample_size + 1;
        assert!(matches!(
            assess_risk_difference_transport(&invalid),
            Err(CausalTransportError::InvalidSourceStratum { .. })
        ));

        let mut duplicate_target = input(full_support_target());
        duplicate_target.target_strata.push(duplicate_target.target_strata[0].clone());
        assert!(matches!(
            assess_risk_difference_transport(&duplicate_target),
            Err(CausalTransportError::DuplicateTargetModifier { .. })
        ));
    }

    #[test]
    fn rejects_missing_digests_empty_populations_and_missing_assumption_receipts() {
        let mut missing = input(full_support_target());
        missing.target_population_digest.clear();
        assert_eq!(
            assess_risk_difference_transport(&missing).unwrap_err(),
            CausalTransportError::MissingDataDigest
        );

        let mut empty_source = input(full_support_target());
        empty_source.source_strata.clear();
        assert_eq!(
            assess_risk_difference_transport(&empty_source).unwrap_err(),
            CausalTransportError::EmptySourceStrata
        );

        let mut empty_target = input(full_support_target());
        empty_target.target_strata.clear();
        assert_eq!(
            assess_risk_difference_transport(&empty_target).unwrap_err(),
            CausalTransportError::EmptyTargetStrata
        );

        let mut missing_assumption = input(full_support_target());
        missing_assumption.assumptions.selection_exchangeability.evidence_digest.clear();
        assert_eq!(
            assess_risk_difference_transport(&missing_assumption).unwrap_err(),
            CausalTransportError::MissingAssumptionEvidenceDigest
        );
    }
}
