// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Claim-specific measurement equivalence for cross-source CIVDYN evidence.
//!
//! Source passports establish rights, acquisition, provenance, units, time and
//! geography. This module adds a separate question: do two operationalizations
//! measure a comparable construct for the specific inference being attempted?
//! It validates typed evidence receipts; it does not fit CFA/IRT models, perform
//! equating, or independently establish the truth of an upstream study.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementFamily {
    SurveyLatentScale,
    DirectPhysicalMeasure,
    AdministrativeClassification,
    DerivedIndicator,
}

impl MeasurementFamily {
    pub const fn name(self) -> &'static str {
        match self {
            Self::SurveyLatentScale => "survey_latent_scale",
            Self::DirectPhysicalMeasure => "direct_physical_measure",
            Self::AdministrativeClassification => "administrative_classification",
            Self::DerivedIndicator => "derived_indicator",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementDefinition {
    pub measurement_id: String,
    pub construct_id: String,
    pub construct_version: String,
    pub operationalization_digest: String,
    pub instrument_id: String,
    pub instrument_digest: String,
    pub family: MeasurementFamily,
    pub measurement_unit: String,
    pub population_id: String,
    pub context_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementClaim {
    StructuralConfiguration,
    AssociationComparison,
    LatentMeanComparison,
    IndividualScoreComparison,
}

impl MeasurementClaim {
    pub const ALL: [Self; 4] = [
        Self::StructuralConfiguration,
        Self::AssociationComparison,
        Self::LatentMeanComparison,
        Self::IndividualScoreComparison,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::StructuralConfiguration => "structural_configuration",
            Self::AssociationComparison => "association_comparison",
            Self::LatentMeanComparison => "latent_mean_comparison",
            Self::IndividualScoreComparison => "individual_score_comparison",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceDisposition {
    Supported,
    NotSupported,
    NotAssessed,
}

impl EvidenceDisposition {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::NotSupported => "not_supported",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquivalenceStatus {
    EquivalentForClaim,
    InsufficientEvidenceForClaim,
    NotEquivalentForClaim,
    NotAssessed,
}

impl EquivalenceStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::EquivalentForClaim => "equivalent_for_claim",
            Self::InsufficientEvidenceForClaim => "insufficient_evidence_for_claim",
            Self::NotEquivalentForClaim => "not_equivalent_for_claim",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquivalenceReason {
    ExactSameOperationalizationAndContext,
    CrossPopulationNeedsMeasurementInvariance,
    ConfiguralInvarianceSupported,
    MetricInvarianceSupported,
    ScalarInvarianceSupported,
    ResidualInvarianceSupported,
    InvarianceNotSupported,
    InvarianceNotAssessed,
    BridgeIntervalsWithinEquivalenceMargins,
    BridgeIntervalsOutsideEquivalenceMargins,
    BridgeIntervalsOverlapEquivalenceMargins,
    PopulationInvarianceNotSupported,
    PopulationInvarianceNotAssessed,
    EvidenceRouteDoesNotSupportClaim,
}

impl EquivalenceReason {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ExactSameOperationalizationAndContext => "exact_same_operationalization_and_context",
            Self::CrossPopulationNeedsMeasurementInvariance => "cross_population_needs_measurement_invariance",
            Self::ConfiguralInvarianceSupported => "configural_invariance_supported",
            Self::MetricInvarianceSupported => "metric_invariance_supported",
            Self::ScalarInvarianceSupported => "scalar_invariance_supported",
            Self::ResidualInvarianceSupported => "residual_invariance_supported",
            Self::InvarianceNotSupported => "invariance_not_supported",
            Self::InvarianceNotAssessed => "invariance_not_assessed",
            Self::BridgeIntervalsWithinEquivalenceMargins => "bridge_intervals_within_equivalence_margins",
            Self::BridgeIntervalsOutsideEquivalenceMargins => "bridge_intervals_outside_equivalence_margins",
            Self::BridgeIntervalsOverlapEquivalenceMargins => "bridge_intervals_overlap_equivalence_margins",
            Self::PopulationInvarianceNotSupported => "population_invariance_not_supported",
            Self::PopulationInvarianceNotAssessed => "population_invariance_not_assessed",
            Self::EvidenceRouteDoesNotSupportClaim => "evidence_route_does_not_support_claim",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeDesign {
    Paired,
    RandomizedSplitSample,
}

impl BridgeDesign {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Paired => "paired",
            Self::RandomizedSplitSample => "randomized_split_sample",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementGroup {
    pub population_id: String,
    pub sample_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SameProtocolEvidence {
    pub protocol_digest: String,
    /// Required for direct physical measurements; absent for other families.
    pub calibration_receipt_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurveyInvarianceEvidence {
    pub study_digest: String,
    pub data_digest: String,
    pub protocol_digest: String,
    pub analysis_digest: String,
    pub minimum_group_sample_size: u64,
    pub groups: Vec<MeasurementGroup>,
    pub configural: EvidenceDisposition,
    pub metric: EvidenceDisposition,
    pub scalar: EvidenceDisposition,
    pub residual: EvidenceDisposition,
    pub anchor_item_ids: Vec<String>,
    pub non_invariant_item_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BridgeStudyEvidence {
    pub bridge_study_digest: String,
    pub data_digest: String,
    pub protocol_digest: String,
    pub analysis_digest: String,
    pub design: BridgeDesign,
    /// Required for paired designs; absent for randomized split-sample designs.
    pub paired_sample_size: Option<u64>,
    pub minimum_group_sample_size: u64,
    pub groups: Vec<MeasurementGroup>,
    pub confidence_level: f64,
    pub mean_bias_ci_lower: f64,
    pub mean_bias_ci_upper: f64,
    pub mean_equivalence_margin: f64,
    pub slope_ci_lower: f64,
    pub slope_ci_upper: f64,
    pub slope_equivalence_margin: f64,
    pub individual_error_lower_bound: Option<f64>,
    pub individual_error_upper_bound: Option<f64>,
    pub individual_score_margin: Option<f64>,
    pub population_invariance: EvidenceDisposition,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MeasurementEquivalenceEvidence {
    SameProtocol(SameProtocolEvidence),
    SurveyInvariance(SurveyInvarianceEvidence),
    BridgeStudy(BridgeStudyEvidence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimEquivalenceResult {
    pub claim: MeasurementClaim,
    pub status: EquivalenceStatus,
    pub reason: EquivalenceReason,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementEquivalenceReport {
    pub left_measurement_id: String,
    pub right_measurement_id: String,
    pub construct_id: String,
    pub construct_version: String,
    pub family: MeasurementFamily,
    pub measurement_unit: String,
    pub left_population_id: String,
    pub right_population_id: String,
    pub left_context_id: String,
    pub right_context_id: String,
    pub left_operationalization_digest: String,
    pub right_operationalization_digest: String,
    pub left_instrument_digest: String,
    pub right_instrument_digest: String,
    pub evidence_digest: String,
    pub data_digest: Option<String>,
    pub protocol_digest: String,
    pub analysis_digest: Option<String>,
    pub calibration_receipt_digest: Option<String>,
    pub evidence_kind: &'static str,
    pub groups: Vec<MeasurementGroup>,
    pub bridge_design: Option<BridgeDesign>,
    pub confidence_level: Option<f64>,
    pub mean_bias_ci: Option<(f64, f64)>,
    pub mean_equivalence_margin: Option<f64>,
    pub slope_ci: Option<(f64, f64)>,
    pub slope_equivalence_margin: Option<f64>,
    pub individual_error_upper_bound: Option<f64>,
    pub individual_score_margin: Option<f64>,
    pub invariance_levels: Option<[EvidenceDisposition; 4]>,
    pub anchor_item_ids: Vec<String>,
    pub non_invariant_item_ids: Vec<String>,
    pub claims: Vec<ClaimEquivalenceResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasurementEquivalenceError {
    MissingMeasurementIdentity,
    DifferentConstruct,
    DifferentConstructVersion,
    DifferentMeasurementFamily,
    DifferentMeasurementUnit,
    MissingEvidenceDigest,
    OperationalizationMismatch,
    MissingPhysicalCalibrationReceipt,
    InvarianceEvidenceRequiresSurveyScale,
    BridgeEvidenceRequiresCompatibleFamilies,
    InvalidGroupSet,
    DuplicatePopulationGroup { population_id: String },
    MissingPopulationGroup { population_id: String },
    InvalidGroupSampleSize { population_id: String },
    InvalidMinimumSampleSize,
    InvalidInvariantHierarchy,
    MissingAnchorItems,
    DuplicateAnchorItem { item_id: String },
    DuplicateNonInvariantItem { item_id: String },
    AnchorItemMarkedNonInvariant { item_id: String },
    MissingBridgeEquivalenceEvidence,
    InvalidBridgeConfidenceInterval,
    InvalidBridgeEquivalenceMargin,
    InvalidBridgeConfidenceLevel,
    InvalidIndividualScoreBound,
}

impl fmt::Display for MeasurementEquivalenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for MeasurementEquivalenceError {}

fn validate_measurement_pair(
    left: &MeasurementDefinition,
    right: &MeasurementDefinition,
) -> Result<(), MeasurementEquivalenceError> {
    for measurement in [left, right] {
        if measurement.measurement_id.trim().is_empty()
            || measurement.construct_id.trim().is_empty()
            || measurement.construct_version.trim().is_empty()
            || measurement.operationalization_digest.trim().is_empty()
            || measurement.instrument_id.trim().is_empty()
            || measurement.instrument_digest.trim().is_empty()
            || measurement.measurement_unit.trim().is_empty()
            || measurement.population_id.trim().is_empty()
            || measurement.context_id.trim().is_empty()
        {
            return Err(MeasurementEquivalenceError::MissingMeasurementIdentity);
        }
    }
    if left.construct_id != right.construct_id {
        return Err(MeasurementEquivalenceError::DifferentConstruct);
    }
    if left.construct_version != right.construct_version {
        return Err(MeasurementEquivalenceError::DifferentConstructVersion);
    }
    if left.family != right.family {
        return Err(MeasurementEquivalenceError::DifferentMeasurementFamily);
    }
    if left.measurement_unit != right.measurement_unit {
        return Err(MeasurementEquivalenceError::DifferentMeasurementUnit);
    }
    Ok(())
}

fn validate_groups(
    groups: &[MeasurementGroup],
    left: &MeasurementDefinition,
    right: &MeasurementDefinition,
    minimum_sample_size: u64,
) -> Result<(), MeasurementEquivalenceError> {
    if minimum_sample_size == 0 || groups.len() < 2 {
        return Err(MeasurementEquivalenceError::InvalidGroupSet);
    }
    let mut seen = BTreeSet::new();
    for group in groups {
        if group.population_id.trim().is_empty() || group.sample_size == 0 {
            return Err(MeasurementEquivalenceError::InvalidGroupSampleSize {
                population_id: group.population_id.clone(),
            });
        }
        if group.sample_size < minimum_sample_size {
            return Err(MeasurementEquivalenceError::InvalidGroupSampleSize {
                population_id: group.population_id.clone(),
            });
        }
        if !seen.insert(group.population_id.as_str()) {
            return Err(MeasurementEquivalenceError::DuplicatePopulationGroup {
                population_id: group.population_id.clone(),
            });
        }
    }
    for population_id in [&left.population_id, &right.population_id] {
        if !seen.contains(population_id.as_str()) {
            return Err(MeasurementEquivalenceError::MissingPopulationGroup {
                population_id: population_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_invariance_hierarchy(
    evidence: &SurveyInvarianceEvidence,
) -> Result<(), MeasurementEquivalenceError> {
    if [
        evidence.study_digest.as_str(),
        evidence.data_digest.as_str(),
        evidence.protocol_digest.as_str(),
        evidence.analysis_digest.as_str(),
    ]
    .iter()
    .any(|digest| digest.trim().is_empty())
    {
        return Err(MeasurementEquivalenceError::MissingEvidenceDigest);
    }
    if evidence.minimum_group_sample_size == 0 {
        return Err(MeasurementEquivalenceError::InvalidMinimumSampleSize);
    }
    if evidence.metric == EvidenceDisposition::Supported
        && evidence.configural != EvidenceDisposition::Supported
        || evidence.scalar == EvidenceDisposition::Supported
            && evidence.metric != EvidenceDisposition::Supported
        || evidence.residual == EvidenceDisposition::Supported
            && evidence.scalar != EvidenceDisposition::Supported
    {
        return Err(MeasurementEquivalenceError::InvalidInvariantHierarchy);
    }
    let mut anchors = BTreeSet::new();
    for item_id in &evidence.anchor_item_ids {
        if item_id.trim().is_empty() {
            return Err(MeasurementEquivalenceError::MissingAnchorItems);
        }
        if !anchors.insert(item_id.as_str()) {
            return Err(MeasurementEquivalenceError::DuplicateAnchorItem {
                item_id: item_id.clone(),
            });
        }
    }
    let mut non_invariant = BTreeSet::new();
    for item_id in &evidence.non_invariant_item_ids {
        if item_id.trim().is_empty() {
            return Err(MeasurementEquivalenceError::MissingAnchorItems);
        }
        if !non_invariant.insert(item_id.as_str()) {
            return Err(MeasurementEquivalenceError::DuplicateNonInvariantItem {
                item_id: item_id.clone(),
            });
        }
        if anchors.contains(item_id.as_str()) {
            return Err(MeasurementEquivalenceError::AnchorItemMarkedNonInvariant {
                item_id: item_id.clone(),
            });
        }
    }
    if matches!(
        evidence.metric,
        EvidenceDisposition::Supported
    ) && evidence.anchor_item_ids.is_empty()
    {
        return Err(MeasurementEquivalenceError::MissingAnchorItems);
    }
    Ok(())
}

fn claim_result_from_invariance(
    claim: MeasurementClaim,
    disposition: EvidenceDisposition,
) -> ClaimEquivalenceResult {
    let (status, reason) = match disposition {
        EvidenceDisposition::Supported => (
            EquivalenceStatus::EquivalentForClaim,
            match claim {
                MeasurementClaim::StructuralConfiguration => {
                    EquivalenceReason::ConfiguralInvarianceSupported
                }
                MeasurementClaim::AssociationComparison => {
                    EquivalenceReason::MetricInvarianceSupported
                }
                MeasurementClaim::LatentMeanComparison => {
                    EquivalenceReason::ScalarInvarianceSupported
                }
                MeasurementClaim::IndividualScoreComparison => {
                    EquivalenceReason::ResidualInvarianceSupported
                }
            },
        ),
        EvidenceDisposition::NotSupported => (
            EquivalenceStatus::NotEquivalentForClaim,
            EquivalenceReason::InvarianceNotSupported,
        ),
        EvidenceDisposition::NotAssessed => (
            EquivalenceStatus::NotAssessed,
            EquivalenceReason::InvarianceNotAssessed,
        ),
    };
    ClaimEquivalenceResult {
        claim,
        status,
        reason,
    }
}

fn interval_equivalence(
    lower: f64,
    upper: f64,
    margin: f64,
) -> Result<EvidenceDisposition, MeasurementEquivalenceError> {
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return Err(MeasurementEquivalenceError::InvalidBridgeConfidenceInterval);
    }
    if !margin.is_finite() || margin <= 0.0 {
        return Err(MeasurementEquivalenceError::InvalidBridgeEquivalenceMargin);
    }
    if lower >= -margin && upper <= margin {
        Ok(EvidenceDisposition::Supported)
    } else if lower > margin || upper < -margin {
        Ok(EvidenceDisposition::NotSupported)
    } else {
        // The interval overlaps the acceptable region but is too wide to
        // establish equivalence. It does not prove nonequivalence either.
        Ok(EvidenceDisposition::NotAssessed)
    }
}

fn bridge_claim(
    claim: MeasurementClaim,
    population_invariance: EvidenceDisposition,
    mean_bias: EvidenceDisposition,
    slope: EvidenceDisposition,
    individual_score: Option<EvidenceDisposition>,
) -> ClaimEquivalenceResult {
    if claim == MeasurementClaim::StructuralConfiguration {
        return ClaimEquivalenceResult {
            claim,
            status: EquivalenceStatus::NotAssessed,
            reason: EquivalenceReason::EvidenceRouteDoesNotSupportClaim,
        };
    }

    let requested = match claim {
        MeasurementClaim::StructuralConfiguration => unreachable!(),
        MeasurementClaim::AssociationComparison => slope,
        MeasurementClaim::LatentMeanComparison => mean_bias,
        MeasurementClaim::IndividualScoreComparison => {
            match individual_score {
                Some(value) => value,
                None => EvidenceDisposition::NotAssessed,
            }
        }
    };

    let (status, reason) = if population_invariance == EvidenceDisposition::NotSupported
        || requested == EvidenceDisposition::NotSupported
    {
        (
            EquivalenceStatus::NotEquivalentForClaim,
            if population_invariance == EvidenceDisposition::NotSupported {
                EquivalenceReason::PopulationInvarianceNotSupported
            } else {
                EquivalenceReason::BridgeIntervalsOutsideEquivalenceMargins
            },
        )
    } else if population_invariance == EvidenceDisposition::NotAssessed
        || requested == EvidenceDisposition::NotAssessed
    {
        (
            EquivalenceStatus::InsufficientEvidenceForClaim,
            if population_invariance == EvidenceDisposition::NotAssessed {
                EquivalenceReason::PopulationInvarianceNotAssessed
            } else {
                EquivalenceReason::BridgeIntervalsOverlapEquivalenceMargins
            },
        )
    } else {
        (
            EquivalenceStatus::EquivalentForClaim,
            EquivalenceReason::BridgeIntervalsWithinEquivalenceMargins,
        )
    };
    ClaimEquivalenceResult {
        claim,
        status,
        reason,
    }
}

/// Evaluate claim-specific equivalence for two registered measurement
/// operationalizations. This function does not perform a data join; source
/// passports and their compatibility gate remain independent prerequisites.
pub fn assess_measurement_equivalence(
    left: &MeasurementDefinition,
    right: &MeasurementDefinition,
    evidence: &MeasurementEquivalenceEvidence,
) -> Result<MeasurementEquivalenceReport, MeasurementEquivalenceError> {
    validate_measurement_pair(left, right)?;

    let (evidence_digest, evidence_kind, invariance_levels, anchor_item_ids, non_invariant_item_ids, claims) =
        match evidence {
            MeasurementEquivalenceEvidence::SameProtocol(receipt) => {
                if receipt.protocol_digest.trim().is_empty() {
                    return Err(MeasurementEquivalenceError::MissingEvidenceDigest);
                }
                if left.operationalization_digest != right.operationalization_digest
                    || left.instrument_digest != right.instrument_digest
                {
                    return Err(MeasurementEquivalenceError::OperationalizationMismatch);
                }
                if left.population_id != right.population_id || left.context_id != right.context_id {
                    let claims = MeasurementClaim::ALL
                        .into_iter()
                        .map(|claim| ClaimEquivalenceResult {
                            claim,
                            status: EquivalenceStatus::InsufficientEvidenceForClaim,
                            reason: EquivalenceReason::CrossPopulationNeedsMeasurementInvariance,
                        })
                        .collect();
                    (
                        receipt.protocol_digest.clone(),
                        "same_protocol",
                        None,
                        Vec::new(),
                        Vec::new(),
                        claims,
                    )
                } else {
                    if left.family == MeasurementFamily::DirectPhysicalMeasure
                        && receipt
                            .calibration_receipt_digest
                            .as_deref()
                            .is_none_or(str::is_empty)
                    {
                        return Err(MeasurementEquivalenceError::MissingPhysicalCalibrationReceipt);
                    }
                    let claims = MeasurementClaim::ALL
                        .into_iter()
                        .map(|claim| ClaimEquivalenceResult {
                            claim,
                            status: EquivalenceStatus::EquivalentForClaim,
                            reason: EquivalenceReason::ExactSameOperationalizationAndContext,
                        })
                        .collect();
                    (
                        receipt.protocol_digest.clone(),
                        "same_protocol",
                        None,
                        Vec::new(),
                        Vec::new(),
                        claims,
                    )
                }
            }
            MeasurementEquivalenceEvidence::SurveyInvariance(receipt) => {
                if left.family != MeasurementFamily::SurveyLatentScale {
                    return Err(MeasurementEquivalenceError::InvarianceEvidenceRequiresSurveyScale);
                }
                validate_invariance_hierarchy(receipt)?;
                validate_groups(
                    &receipt.groups,
                    left,
                    right,
                    receipt.minimum_group_sample_size,
                )?;
                let dispositions = [
                    receipt.configural,
                    receipt.metric,
                    receipt.scalar,
                    receipt.residual,
                ];
                let claims = MeasurementClaim::ALL
                    .into_iter()
                    .zip(dispositions)
                    .map(|(claim, disposition)| claim_result_from_invariance(claim, disposition))
                    .collect();
                (
                    receipt.study_digest.clone(),
                    "survey_invariance",
                    Some([
                        receipt.configural,
                        receipt.metric,
                        receipt.scalar,
                        receipt.residual,
                    ]),
                    receipt.anchor_item_ids.clone(),
                    receipt.non_invariant_item_ids.clone(),
                    claims,
                )
            }
            MeasurementEquivalenceEvidence::BridgeStudy(receipt) => {
                if receipt.bridge_study_digest.trim().is_empty()
                    || receipt.data_digest.trim().is_empty()
                    || receipt.protocol_digest.trim().is_empty()
                    || receipt.analysis_digest.trim().is_empty()
                {
                    return Err(MeasurementEquivalenceError::MissingBridgeEquivalenceEvidence);
                }
                if !receipt.confidence_level.is_finite()
                    || receipt.confidence_level <= 0.0
                    || receipt.confidence_level >= 1.0
                {
                    return Err(MeasurementEquivalenceError::InvalidBridgeConfidenceLevel);
                }
                validate_groups(
                    &receipt.groups,
                    left,
                    right,
                    receipt.minimum_group_sample_size,
                )?;
                match (receipt.design, receipt.paired_sample_size) {
                    (BridgeDesign::Paired, Some(paired_n))
                        if paired_n >= receipt.minimum_group_sample_size
                            && paired_n
                                <= receipt
                                    .groups
                                    .iter()
                                    .map(|group| group.sample_size)
                                    .min()
                                    .unwrap_or(0) => {}
                    (BridgeDesign::RandomizedSplitSample, None) => {}
                    _ => return Err(MeasurementEquivalenceError::InvalidGroupSet),
                }
                if receipt.mean_equivalence_margin <= 0.0
                    || !receipt.mean_equivalence_margin.is_finite()
                    || receipt.slope_equivalence_margin <= 0.0
                    || !receipt.slope_equivalence_margin.is_finite()
                    || receipt.slope_equivalence_margin >= 1.0
                {
                    return Err(MeasurementEquivalenceError::InvalidBridgeEquivalenceMargin);
                }
                let mean_bias = interval_equivalence(
                    receipt.mean_bias_ci_lower,
                    receipt.mean_bias_ci_upper,
                    receipt.mean_equivalence_margin,
                )?;
                let slope = interval_equivalence(
                    receipt.slope_ci_lower - 1.0,
                    receipt.slope_ci_upper - 1.0,
                    receipt.slope_equivalence_margin,
                )?;
                let individual_score = match (
                    receipt.individual_error_lower_bound,
                    receipt.individual_error_upper_bound,
                    receipt.individual_score_margin,
                ) {
                    (Some(lower), Some(upper), Some(margin))
                        if lower.is_finite()
                            && upper.is_finite()
                            && margin.is_finite()
                            && lower >= 0.0
                            && lower <= upper
                            && margin > 0.0 =>
                    {
                        Some(if upper <= margin {
                            EvidenceDisposition::Supported
                        } else if lower > margin {
                            EvidenceDisposition::NotSupported
                        } else {
                            EvidenceDisposition::NotAssessed
                        })
                    }
                    (None, None, None) => None,
                    _ => return Err(MeasurementEquivalenceError::InvalidIndividualScoreBound),
                };
                let claims = MeasurementClaim::ALL
                    .into_iter()
                    .map(|claim| {
                        bridge_claim(
                            claim,
                            receipt.population_invariance,
                            mean_bias,
                            slope,
                            individual_score,
                        )
                    })
                    .collect();
                (
                    receipt.bridge_study_digest.clone(),
                    "bridge_study",
                    None,
                    Vec::new(),
                    Vec::new(),
                    claims,
                )
            }
        };

    let (
        data_digest,
        protocol_digest,
        analysis_digest,
        calibration_receipt_digest,
        groups,
        bridge_design,
        confidence_level,
        mean_bias_ci,
        mean_equivalence_margin,
        slope_ci,
        slope_equivalence_margin,
        individual_error_lower_bound,
        individual_error_upper_bound,
        individual_score_margin,
    ) = match evidence {
        MeasurementEquivalenceEvidence::SameProtocol(receipt) => (
            None,
            receipt.protocol_digest.clone(),
            None,
            receipt.calibration_receipt_digest.clone(),
            Vec::new(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        MeasurementEquivalenceEvidence::SurveyInvariance(receipt) => (
            Some(receipt.data_digest.clone()),
            receipt.protocol_digest.clone(),
            Some(receipt.analysis_digest.clone()),
            None,
            receipt.groups.clone(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        MeasurementEquivalenceEvidence::BridgeStudy(receipt) => (
            Some(receipt.data_digest.clone()),
            receipt.protocol_digest.clone(),
            Some(receipt.analysis_digest.clone()),
            None,
            receipt.groups.clone(),
            Some(receipt.design),
            Some(receipt.confidence_level),
            Some((receipt.mean_bias_ci_lower, receipt.mean_bias_ci_upper)),
            Some(receipt.mean_equivalence_margin),
            Some((receipt.slope_ci_lower, receipt.slope_ci_upper)),
            Some(receipt.slope_equivalence_margin),
            receipt.individual_error_lower_bound,
            receipt.individual_error_upper_bound,
            receipt.individual_score_margin,
        ),
    };

    Ok(MeasurementEquivalenceReport {
        left_measurement_id: left.measurement_id.clone(),
        right_measurement_id: right.measurement_id.clone(),
        construct_id: left.construct_id.clone(),
        construct_version: left.construct_version.clone(),
        family: left.family,
        measurement_unit: left.measurement_unit.clone(),
        left_population_id: left.population_id.clone(),
        right_population_id: right.population_id.clone(),
        left_context_id: left.context_id.clone(),
        right_context_id: right.context_id.clone(),
        left_operationalization_digest: left.operationalization_digest.clone(),
        right_operationalization_digest: right.operationalization_digest.clone(),
        left_instrument_digest: left.instrument_digest.clone(),
        right_instrument_digest: right.instrument_digest.clone(),
        evidence_digest,
        data_digest,
        protocol_digest,
        analysis_digest,
        calibration_receipt_digest,
        evidence_kind,
        groups,
        bridge_design,
        confidence_level,
        mean_bias_ci,
        mean_equivalence_margin,
        slope_ci,
        slope_equivalence_margin,
        individual_error_lower_bound,
        individual_error_upper_bound,
        individual_score_margin,
        invariance_levels,
        anchor_item_ids,
        non_invariant_item_ids,
        claims,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure(
        measurement_id: &str,
        population_id: &str,
        context_id: &str,
        operationalization_digest: &str,
        instrument_digest: &str,
        family: MeasurementFamily,
    ) -> MeasurementDefinition {
        MeasurementDefinition {
            measurement_id: measurement_id.to_string(),
            construct_id: "household-service-security".to_string(),
            construct_version: "v1".to_string(),
            operationalization_digest: operationalization_digest.to_string(),
            instrument_id: "service-security-instrument".to_string(),
            instrument_digest: instrument_digest.to_string(),
            family,
            measurement_unit: "standardized_score".to_string(),
            population_id: population_id.to_string(),
            context_id: context_id.to_string(),
        }
    }

    fn group(population_id: &str, sample_size: u64) -> MeasurementGroup {
        MeasurementGroup {
            population_id: population_id.to_string(),
            sample_size,
        }
    }

    fn invariance(
        configural: EvidenceDisposition,
        metric: EvidenceDisposition,
        scalar: EvidenceDisposition,
        residual: EvidenceDisposition,
    ) -> SurveyInvarianceEvidence {
        SurveyInvarianceEvidence {
            study_digest: "sha256-invariance-study".to_string(),
            data_digest: "sha256-invariance-data".to_string(),
            protocol_digest: "sha256-invariance-protocol".to_string(),
            analysis_digest: "sha256-invariance-analysis".to_string(),
            minimum_group_sample_size: 100,
            groups: vec![group("population-a", 500), group("population-b", 450)],
            configural,
            metric,
            scalar,
            residual,
            anchor_item_ids: vec!["anchor-1".to_string(), "anchor-2".to_string()],
            non_invariant_item_ids: vec!["item-4".to_string()],
        }
    }

    fn bridge(population_invariance: EvidenceDisposition) -> BridgeStudyEvidence {
        BridgeStudyEvidence {
            bridge_study_digest: "sha256-bridge-study".to_string(),
            data_digest: "sha256-bridge-data".to_string(),
            protocol_digest: "sha256-bridge-protocol".to_string(),
            analysis_digest: "sha256-bridge-analysis".to_string(),
            design: BridgeDesign::Paired,
            paired_sample_size: Some(200),
            minimum_group_sample_size: 100,
            groups: vec![group("population-a", 250), group("population-b", 240)],
            confidence_level: 0.95,
            mean_bias_ci_lower: -0.04,
            mean_bias_ci_upper: 0.03,
            mean_equivalence_margin: 0.10,
            slope_ci_lower: 0.94,
            slope_ci_upper: 1.05,
            slope_equivalence_margin: 0.10,
            individual_error_lower_bound: Some(0.04),
            individual_error_upper_bound: Some(0.08),
            individual_score_margin: Some(0.12),
            population_invariance,
        }
    }

    fn survey_pair() -> (MeasurementDefinition, MeasurementDefinition) {
        (
            measure(
                "instrument-a",
                "population-a",
                "baseline",
                "sha256-op-a",
                "sha256-instrument-a",
                MeasurementFamily::SurveyLatentScale,
            ),
            measure(
                "instrument-b",
                "population-b",
                "baseline",
                "sha256-op-b",
                "sha256-instrument-b",
                MeasurementFamily::SurveyLatentScale,
            ),
        )
    }

    #[test]
    fn same_construct_label_with_different_construct_ids_is_rejected() {
        let (left, mut right) = survey_pair();
        right.construct_id = "different-construct".to_string();
        assert_eq!(
            assess_measurement_equivalence(
                &left,
                &right,
                &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
                    EvidenceDisposition::Supported,
                    EvidenceDisposition::Supported,
                    EvidenceDisposition::Supported,
                    EvidenceDisposition::Supported,
                )),
            )
            .unwrap_err(),
            MeasurementEquivalenceError::DifferentConstruct
        );
    }

    #[test]
    fn exact_same_protocol_is_bounded_to_same_population_and_context() {
        let left = measure(
            "measure-a",
            "population-a",
            "same-context",
            "sha256-same-operation",
            "sha256-same-instrument",
            MeasurementFamily::DirectPhysicalMeasure,
        );
        let right = measure(
            "measure-b",
            "population-a",
            "same-context",
            "sha256-same-operation",
            "sha256-same-instrument",
            MeasurementFamily::DirectPhysicalMeasure,
        );
        let report = assess_measurement_equivalence(
            &left,
            &right,
            &MeasurementEquivalenceEvidence::SameProtocol(SameProtocolEvidence {
                protocol_digest: "sha256-common-protocol".to_string(),
                calibration_receipt_digest: Some("sha256-calibration-certificate".to_string()),
            }),
        )
        .expect("same protocol report");
        assert!(
            report
                .claims
                .iter()
                .all(|claim| claim.status == EquivalenceStatus::EquivalentForClaim)
        );

        let mut cross_population = right;
        cross_population.population_id = "population-b".to_string();
        let report = assess_measurement_equivalence(
            &left,
            &cross_population,
            &MeasurementEquivalenceEvidence::SameProtocol(SameProtocolEvidence {
                protocol_digest: "sha256-common-protocol".to_string(),
                calibration_receipt_digest: Some("sha256-calibration-certificate".to_string()),
            }),
        )
        .expect("cross-population same protocol is limited");
        assert!(report.claims.iter().all(|claim| {
            claim.status == EquivalenceStatus::InsufficientEvidenceForClaim
        }));
    }

    #[test]
    fn configural_only_evidence_supports_structure_but_not_associations_or_means() {
        let (left, right) = survey_pair();
        let report = assess_measurement_equivalence(
            &left,
            &right,
            &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
                EvidenceDisposition::Supported,
                EvidenceDisposition::NotAssessed,
                EvidenceDisposition::NotAssessed,
                EvidenceDisposition::NotAssessed,
            )),
        )
        .expect("configural-only report");
        assert_eq!(
            report.claims[0].status,
            EquivalenceStatus::EquivalentForClaim
        );
        assert_eq!(
            report.claims[1].status,
            EquivalenceStatus::NotAssessed
        );
        assert_eq!(
            report.claims[2].status,
            EquivalenceStatus::NotAssessed
        );
    }

    #[test]
    fn metric_but_not_scalar_evidence_supports_associations_not_latent_means() {
        let (left, right) = survey_pair();
        let report = assess_measurement_equivalence(
            &left,
            &right,
            &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
                EvidenceDisposition::Supported,
                EvidenceDisposition::Supported,
                EvidenceDisposition::NotSupported,
                EvidenceDisposition::NotAssessed,
            )),
        )
        .expect("metric-only report");
        assert_eq!(
            report.claims[1].status,
            EquivalenceStatus::EquivalentForClaim
        );
        assert_eq!(
            report.claims[2].status,
            EquivalenceStatus::NotEquivalentForClaim
        );
    }

    #[test]
    fn scalar_but_not_residual_evidence_supports_means_not_individual_scores() {
        let (left, right) = survey_pair();
        let report = assess_measurement_equivalence(
            &left,
            &right,
            &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
                EvidenceDisposition::Supported,
                EvidenceDisposition::Supported,
                EvidenceDisposition::Supported,
                EvidenceDisposition::NotAssessed,
            )),
        )
        .expect("scalar-only report");
        assert_eq!(
            report.claims[2].status,
            EquivalenceStatus::EquivalentForClaim
        );
        assert_eq!(
            report.claims[3].status,
            EquivalenceStatus::NotAssessed
        );
    }

    #[test]
    fn full_measurement_invariance_supports_each_declared_survey_claim() {
        let (left, right) = survey_pair();
        let report = assess_measurement_equivalence(
            &left,
            &right,
            &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
                EvidenceDisposition::Supported,
                EvidenceDisposition::Supported,
                EvidenceDisposition::Supported,
                EvidenceDisposition::Supported,
            )),
        )
        .expect("full invariance report");
        assert_eq!(report.claims.len(), MeasurementClaim::ALL.len());
        assert!(report.claims.iter().all(|claim| {
            claim.status == EquivalenceStatus::EquivalentForClaim
        }));
    }

    #[test]
    fn partial_invariance_preserves_anchor_and_non_invariant_item_identity() {
        let (left, right) = survey_pair();
        let report = assess_measurement_equivalence(
            &left,
            &right,
            &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
                EvidenceDisposition::Supported,
                EvidenceDisposition::Supported,
                EvidenceDisposition::Supported,
                EvidenceDisposition::NotAssessed,
            )),
        )
        .expect("partial invariance report");
        assert_eq!(
            report.anchor_item_ids,
            vec!["anchor-1".to_string(), "anchor-2".to_string()]
        );
        assert_eq!(
            report.non_invariant_item_ids,
            vec!["item-4".to_string()]
        );
    }

    #[test]
    fn bridge_study_supports_claims_only_when_intervals_are_inside_frozen_margins() {
        let (left, right) = survey_pair();
        let report = assess_measurement_equivalence(
            &left,
            &right,
            &MeasurementEquivalenceEvidence::BridgeStudy(bridge(
                EvidenceDisposition::Supported,
            )),
        )
        .expect("bridge study report");
        assert_eq!(
            report.claims[0].status,
            EquivalenceStatus::NotAssessed
        );
        assert_eq!(
            report.claims[1].status,
            EquivalenceStatus::EquivalentForClaim
        );
        assert_eq!(
            report.claims[2].status,
            EquivalenceStatus::EquivalentForClaim
        );
        assert_eq!(
            report.claims[3].status,
            EquivalenceStatus::EquivalentForClaim
        );
    }

    #[test]
    fn bridge_intervals_overlapping_margin_are_insufficient_not_equivalent() {
        let (left, right) = survey_pair();
        let mut evidence = bridge(EvidenceDisposition::Supported);
        evidence.mean_bias_ci_lower = -0.20;
        evidence.mean_bias_ci_upper = 0.05;
        let report = assess_measurement_equivalence(
            &left,
            &right,
            &MeasurementEquivalenceEvidence::BridgeStudy(evidence),
        )
        .expect("wide bridge confidence interval remains reportable");
        assert_eq!(
            report.claims[2].status,
            EquivalenceStatus::InsufficientEvidenceForClaim
        );
        assert_eq!(
            report.claims[1].status,
            EquivalenceStatus::EquivalentForClaim
        );
    }

    #[test]
    fn bridge_individual_score_error_clearly_above_margin_is_not_equivalent() {
        let (left, right) = survey_pair();
        let mut evidence = bridge(EvidenceDisposition::Supported);
        evidence.individual_error_lower_bound = Some(0.20);
        evidence.individual_error_upper_bound = Some(0.25);
        evidence.individual_score_margin = Some(0.12);
        let report = assess_measurement_equivalence(
            &left,
            &right,
            &MeasurementEquivalenceEvidence::BridgeStudy(evidence),
        )
        .expect("clear individual-score mismatch is reportable");
        assert_eq!(
            report.claims[3].status,
            EquivalenceStatus::NotEquivalentForClaim
        );
    }

    #[test]
    fn paired_bridge_requires_a_minimum_paired_sample_count() {
        let (left, right) = survey_pair();
        let mut evidence = bridge(EvidenceDisposition::Supported);
        evidence.paired_sample_size = Some(50);
        assert_eq!(
            assess_measurement_equivalence(
                &left,
                &right,
                &MeasurementEquivalenceEvidence::BridgeStudy(evidence)
            )
            .unwrap_err(),
            MeasurementEquivalenceError::InvalidGroupSet
        );
    }

    #[test]
    fn bridge_population_noninvariance_blocks_comparison_claims() {
        let (left, right) = survey_pair();
        let report = assess_measurement_equivalence(
            &left,
            &right,
            &MeasurementEquivalenceEvidence::BridgeStudy(bridge(
                EvidenceDisposition::NotSupported,
            )),
        )
        .expect("noninvariant bridge remains reportable");
        assert!(report.claims[1..].iter().all(|claim| {
            claim.status == EquivalenceStatus::NotEquivalentForClaim
        }));
    }

    #[test]
    fn bridge_requires_both_populations_and_preregistered_minimum_sample_sizes() {
        let (left, right) = survey_pair();
        let mut missing_group = bridge(EvidenceDisposition::Supported);
        missing_group.groups.pop();
        assert!(matches!(
            assess_measurement_equivalence(
                &left,
                &right,
                &MeasurementEquivalenceEvidence::BridgeStudy(missing_group)
            ),
            Err(MeasurementEquivalenceError::MissingPopulationGroup { .. })
        ));

        let mut underpowered = bridge(EvidenceDisposition::Supported);
        underpowered.groups[1].sample_size = 50;
        assert!(matches!(
            assess_measurement_equivalence(
                &left,
                &right,
                &MeasurementEquivalenceEvidence::BridgeStudy(underpowered)
            ),
            Err(MeasurementEquivalenceError::InvalidGroupSampleSize { .. })
        ));
    }

    #[test]
    fn invalid_or_missing_evidence_receipts_fail_closed() {
        let (left, right) = survey_pair();
        let mut missing_digest = invariance(
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
        );
        missing_digest.analysis_digest.clear();
        assert_eq!(
            assess_measurement_equivalence(
                &left,
                &right,
                &MeasurementEquivalenceEvidence::SurveyInvariance(missing_digest)
            )
            .unwrap_err(),
            MeasurementEquivalenceError::MissingEvidenceDigest
        );

        let mut missing_anchor = invariance(
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::NotAssessed,
        );
        missing_anchor.anchor_item_ids.clear();
        assert_eq!(
            assess_measurement_equivalence(
                &left,
                &right,
                &MeasurementEquivalenceEvidence::SurveyInvariance(missing_anchor)
            )
            .unwrap_err(),
            MeasurementEquivalenceError::MissingAnchorItems
        );

        let mut missing_physical_calibration = measure(
            "measure-a",
            "population-a",
            "same-context",
            "sha256-same-op",
            "sha256-same-instrument",
            MeasurementFamily::DirectPhysicalMeasure,
        );
        let right_physical = missing_physical_calibration.clone();
        assert_eq!(
            assess_measurement_equivalence(
                &missing_physical_calibration,
                &right_physical,
                &MeasurementEquivalenceEvidence::SameProtocol(SameProtocolEvidence {
                    protocol_digest: "sha256-protocol".to_string(),
                    calibration_receipt_digest: None,
                })
            )
            .unwrap_err(),
            MeasurementEquivalenceError::MissingPhysicalCalibrationReceipt
        );
        missing_physical_calibration.measurement_id.clear();
        assert_eq!(
            assess_measurement_equivalence(
                &missing_physical_calibration,
                &right_physical,
                &MeasurementEquivalenceEvidence::SameProtocol(SameProtocolEvidence {
                    protocol_digest: "sha256-protocol".to_string(),
                    calibration_receipt_digest: Some("sha256-cert".to_string()),
                })
            )
            .unwrap_err(),
            MeasurementEquivalenceError::MissingMeasurementIdentity
        );
    }

    #[test]
    fn same_units_do_not_override_different_constructs_or_unit_mismatch() {
        let (left, mut right) = survey_pair();
        right.construct_id = "water-availability".to_string();
        assert!(matches!(
            assess_measurement_equivalence(
                &left,
                &right,
                &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
                    EvidenceDisposition::Supported,
                    EvidenceDisposition::Supported,
                    EvidenceDisposition::Supported,
                    EvidenceDisposition::Supported,
                ))
            ),
            Err(MeasurementEquivalenceError::DifferentConstruct)
        ));

        right.construct_id = left.construct_id.clone();
        right.measurement_unit = "percent".to_string();
        assert_eq!(
            assess_measurement_equivalence(
                &left,
                &right,
                &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
                    EvidenceDisposition::Supported,
                    EvidenceDisposition::Supported,
                    EvidenceDisposition::Supported,
                    EvidenceDisposition::Supported,
                ))
            )
            .unwrap_err(),
            MeasurementEquivalenceError::DifferentMeasurementUnit
        );
    }

    #[test]
    fn unit_conversion_and_invariance_receipt_are_separate_evidence() {
        let (left, right) = survey_pair();
        let mut receipt = invariance(
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
        );
        receipt.groups[1].population_id = "different-population".to_string();
        assert!(matches!(
            assess_measurement_equivalence(
                &left,
                &right,
                &MeasurementEquivalenceEvidence::SurveyInvariance(receipt)
            ),
            Err(MeasurementEquivalenceError::MissingPopulationGroup { .. })
        ));
    }

    #[test]
    fn higher_invariance_levels_cannot_pass_when_lower_levels_fail() {
        let (left, right) = survey_pair();
        let invalid = invariance(
            EvidenceDisposition::NotSupported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::NotAssessed,
            EvidenceDisposition::NotAssessed,
        );
        assert_eq!(
            assess_measurement_equivalence(
                &left,
                &right,
                &MeasurementEquivalenceEvidence::SurveyInvariance(invalid)
            )
            .unwrap_err(),
            MeasurementEquivalenceError::InvalidInvariantHierarchy
        );
    }
}
