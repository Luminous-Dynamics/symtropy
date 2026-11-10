// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Zero-shot model adequacy across explicitly declared evaluation domains.
//!
//! The same frozen model and calibration-data identity must be evaluated on
//! source-validation and held-out challenge domains. Per-domain results stay
//! separate: there is no pooled transfer score and no target-domain adaptation.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::model_adequacy::{
    AdequacyAssessment, AdequacyError, DomainStatus, PredictiveKind, SplitManifest, SplitRole,
    StatisticRegistry, StatisticStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransferAxis {
    Region,
    TimePeriod,
    Population,
    Institution,
    MeasurementInstrument,
    SourceFamily,
    NetworkTopology,
    InterventionRegime,
}

impl TransferAxis {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Region => "region",
            Self::TimePeriod => "time_period",
            Self::Population => "population",
            Self::Institution => "institution",
            Self::MeasurementInstrument => "measurement_instrument",
            Self::SourceFamily => "source_family",
            Self::NetworkTopology => "network_topology",
            Self::InterventionRegime => "intervention_regime",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferDomainEvidence {
    pub domain_id: String,
    /// Expected, frozen model identity even when a domain has no usable draws.
    pub model_digest: String,
    pub axis: TransferAxis,
    pub registry: StatisticRegistry,
    pub manifest: SplitManifest,
    pub assessment: AdequacyAssessment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatisticStatus {
    DiscrepancyDetected,
    NoDiscrepancyDetected,
    NotAssessed,
}

impl TransferStatisticStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::DiscrepancyDetected => "discrepancy_detected",
            Self::NoDiscrepancyDetected => "no_discrepancy_detected",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferClaimCeiling {
    TransferDiscrepancyDetected,
    NoPrimaryDiscrepancyDetectedAcrossDeclaredTransferDomains,
    NotAssessed,
}

impl TransferClaimCeiling {
    pub const fn name(self) -> &'static str {
        match self {
            Self::TransferDiscrepancyDetected => "transfer_discrepancy_detected",
            Self::NoPrimaryDiscrepancyDetectedAcrossDeclaredTransferDomains => {
                "no_primary_discrepancy_detected_across_declared_transfer_domains"
            }
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferStatisticResult {
    pub statistic_id: String,
    pub construct: String,
    pub measurement_unit: String,
    pub population: String,
    pub context: String,
    pub source_family: String,
    pub source_digest: String,
    pub primary: bool,
    pub required: bool,
    pub expected_units: usize,
    pub assessed_units: usize,
    pub within_reference_units: usize,
    pub outside_reference_units: usize,
    pub not_assessed_units: usize,
    pub status: TransferStatisticStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferDomainResult {
    pub domain_id: String,
    pub model_digest: String,
    pub axis: TransferAxis,
    pub role: SplitRole,
    pub independent_unit_kind: crate::model_adequacy::IndependentUnitKind,
    pub split_manifest_id: String,
    pub split_manifest_digest: String,
    pub required_evidence_complete: bool,
    pub status: DomainStatus,
    pub statistics: Vec<TransferStatisticResult>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferAssessment {
    pub registry_version: String,
    pub model_digest: String,
    pub calibration_data_digest: String,
    pub central_mass: f64,
    pub evidence_complete: bool,
    pub source_validation_domain_count: usize,
    pub held_out_challenge_domain_count: usize,
    pub claim_ceiling: TransferClaimCeiling,
    pub domains: Vec<TransferDomainResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatisticSemanticSignature {
    id: String,
    construct: String,
    measurement_unit: String,
    primary: bool,
    required: bool,
    minimum_draws: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferAdequacyError {
    Adequacy(AdequacyError),
    InvalidDomainId,
    InvalidDomainRole {
        domain_id: String,
    },
    InvalidPredictiveKind {
        domain_id: String,
    },
    EmptyDomainUnits {
        domain_id: String,
    },
    DuplicateDomainId {
        domain_id: String,
    },
    DuplicateManifestId {
        manifest_id: String,
    },
    DuplicateManifestDigest {
        manifest_digest: String,
    },
    CalibrationDataDigestMismatch {
        domain_id: String,
    },
    RegistryVersionMismatch {
        domain_id: String,
    },
    StatisticSchemaMismatch {
        domain_id: String,
        statistic_id: String,
    },
    StatisticRoleMismatch {
        domain_id: String,
        statistic_id: String,
    },
    StatisticUnitKindMismatch {
        domain_id: String,
        statistic_id: String,
    },
    AssessmentManifestMismatch {
        domain_id: String,
    },
    InvalidCentralMass,
    AssessmentCentralMassMismatch {
        domain_id: String,
    },
    AssessmentRegistryMismatch {
        domain_id: String,
    },
    AssessmentRoleMismatch {
        domain_id: String,
    },
    MixedModelIdentityWithinDomain {
        domain_id: String,
    },
    ModelIdentityMismatchAcrossDomains {
        domain_id: String,
    },
    DuplicateAssessmentRow {
        domain_id: String,
        statistic_id: String,
        unit_id: String,
    },
    UnknownAssessmentStatistic {
        domain_id: String,
        statistic_id: String,
    },
    UnknownAssessmentUnit {
        domain_id: String,
        statistic_id: String,
        unit_id: String,
    },
    AssessmentStatisticIdentityMismatch {
        domain_id: String,
        statistic_id: String,
    },
    RequiredEvidenceCompletenessMismatch {
        domain_id: String,
    },
    DomainStatusMismatch {
        domain_id: String,
    },
    MissingSourceValidationDomain,
    MissingHeldOutChallengeDomain,
}

impl fmt::Display for TransferAdequacyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TransferAdequacyError {}

impl From<AdequacyError> for TransferAdequacyError {
    fn from(value: AdequacyError) -> Self {
        Self::Adequacy(value)
    }
}

fn semantic_schema(registry: &StatisticRegistry) -> Vec<StatisticSemanticSignature> {
    let mut schema: Vec<_> = registry
        .statistics
        .iter()
        .map(|spec| StatisticSemanticSignature {
            id: spec.id.clone(),
            construct: spec.construct.clone(),
            measurement_unit: spec.measurement_unit.clone(),
            primary: spec.primary,
            required: spec.required,
            minimum_draws: spec.minimum_draws,
        })
        .collect();
    schema.sort_by(|left, right| left.id.cmp(&right.id));
    schema
}

fn expected_units(manifest: &SplitManifest, role: SplitRole) -> &[String] {
    match role {
        SplitRole::Calibration => &manifest.calibration_units,
        SplitRole::Validation => &manifest.validation_units,
        SplitRole::Challenge => &manifest.challenge_units,
    }
}

fn evaluate_domain(
    domain: &TransferDomainEvidence,
    expected_schema: &[StatisticSemanticSignature],
    expected_registry_version: &str,
    expected_calibration_digest: &str,
    expected_central_mass: f64,
) -> Result<(TransferDomainResult, String), TransferAdequacyError> {
    let domain_id = &domain.domain_id;
    domain.registry.validate()?;
    domain.manifest.validate()?;

    if domain_id.trim().is_empty() || domain.model_digest.trim().is_empty() {
        return Err(TransferAdequacyError::InvalidDomainId);
    }
    if domain.assessment.role != SplitRole::Validation
        && domain.assessment.role != SplitRole::Challenge
    {
        return Err(TransferAdequacyError::InvalidDomainRole {
            domain_id: domain_id.clone(),
        });
    }
    if domain.assessment.predictive_kind != PredictiveKind::Split {
        return Err(TransferAdequacyError::InvalidPredictiveKind {
            domain_id: domain_id.clone(),
        });
    }
    if domain.assessment.central_mass != expected_central_mass {
        return Err(TransferAdequacyError::AssessmentCentralMassMismatch {
            domain_id: domain_id.clone(),
        });
    }
    if domain.assessment.role == SplitRole::Validation
        && domain.manifest.validation_units.is_empty()
        || domain.assessment.role == SplitRole::Challenge
            && domain.manifest.challenge_units.is_empty()
    {
        return Err(TransferAdequacyError::EmptyDomainUnits {
            domain_id: domain_id.clone(),
        });
    }
    if domain.registry.version != expected_registry_version {
        return Err(TransferAdequacyError::RegistryVersionMismatch {
            domain_id: domain_id.clone(),
        });
    }
    if semantic_schema(&domain.registry) != expected_schema {
        let statistic_id = domain
            .registry
            .statistics
            .first()
            .map(|spec| spec.id.clone())
            .unwrap_or_default();
        return Err(TransferAdequacyError::StatisticSchemaMismatch {
            domain_id: domain_id.clone(),
            statistic_id,
        });
    }
    if domain.manifest.calibration_data_digest != expected_calibration_digest {
        return Err(TransferAdequacyError::CalibrationDataDigestMismatch {
            domain_id: domain_id.clone(),
        });
    }
    if domain.assessment.registry_version != domain.registry.version {
        return Err(TransferAdequacyError::AssessmentRegistryMismatch {
            domain_id: domain_id.clone(),
        });
    }
    if domain.assessment.split_manifest_id != domain.manifest.id
        || domain.assessment.split_manifest_digest != domain.manifest.manifest_digest
    {
        return Err(TransferAdequacyError::AssessmentManifestMismatch {
            domain_id: domain_id.clone(),
        });
    }
    if domain.assessment.role
        != domain
            .registry
            .statistics
            .first()
            .map(|spec| spec.role)
            .unwrap_or(SplitRole::Calibration)
    {
        return Err(TransferAdequacyError::AssessmentRoleMismatch {
            domain_id: domain_id.clone(),
        });
    }
    if let Some(spec) = domain
        .registry
        .statistics
        .iter()
        .find(|spec| spec.role != domain.assessment.role)
    {
        return Err(TransferAdequacyError::StatisticRoleMismatch {
            domain_id: domain_id.clone(),
            statistic_id: spec.id.clone(),
        });
    }
    if let Some(spec) = domain
        .registry
        .statistics
        .iter()
        .find(|spec| spec.independent_unit != domain.manifest.independent_unit)
    {
        return Err(TransferAdequacyError::StatisticUnitKindMismatch {
            domain_id: domain_id.clone(),
            statistic_id: spec.id.clone(),
        });
    }

    let units = expected_units(&domain.manifest, domain.assessment.role);
    let unit_set: BTreeSet<&str> = units.iter().map(String::as_str).collect();
    let mut rows_by_key = BTreeMap::new();
    for row in &domain.assessment.statistics {
        let spec = domain.registry.get(&row.statistic_id).ok_or_else(|| {
            TransferAdequacyError::UnknownAssessmentStatistic {
                domain_id: domain_id.clone(),
                statistic_id: row.statistic_id.clone(),
            }
        })?;
        if !unit_set.contains(row.independent_unit_id.as_str()) {
            return Err(TransferAdequacyError::UnknownAssessmentUnit {
                domain_id: domain_id.clone(),
                statistic_id: row.statistic_id.clone(),
                unit_id: row.independent_unit_id.clone(),
            });
        }
        if row.source_family != spec.source_family
            || row.source_digest != spec.source_digest
            || row.measurement_unit != spec.measurement_unit
        {
            return Err(TransferAdequacyError::AssessmentStatisticIdentityMismatch {
                domain_id: domain_id.clone(),
                statistic_id: row.statistic_id.clone(),
            });
        }
        let key = (row.statistic_id.clone(), row.independent_unit_id.clone());
        if rows_by_key.insert(key, row).is_some() {
            return Err(TransferAdequacyError::DuplicateAssessmentRow {
                domain_id: domain_id.clone(),
                statistic_id: row.statistic_id.clone(),
                unit_id: row.independent_unit_id.clone(),
            });
        }
    }

    let model_digest = domain.model_digest.as_str();
    for row in &domain.assessment.statistics {
        if row
            .model_digest
            .as_deref()
            .is_some_and(|row_digest| row_digest != model_digest)
        {
            return Err(TransferAdequacyError::MixedModelIdentityWithinDomain {
                domain_id: domain_id.clone(),
            });
        }
    }

    let required_specs: Vec<_> = domain
        .registry
        .statistics
        .iter()
        .filter(|spec| spec.role == domain.assessment.role && spec.required)
        .collect();
    let mut required_evidence_complete = !required_specs.is_empty() && !units.is_empty();
    let mut statistic_results = Vec::with_capacity(domain.registry.statistics.len());

    for spec in &domain.registry.statistics {
        let spec_units: Vec<&str> = if spec.required {
            units.iter().map(String::as_str).collect()
        } else {
            rows_by_key
                .keys()
                .filter(|(statistic_id, _)| statistic_id == &spec.id)
                .map(|(_, unit_id)| unit_id.as_str())
                .collect()
        };
        let expected_count = spec_units.len();
        let mut assessed = 0;
        let mut within = 0;
        let mut outside = 0;
        let mut not_assessed = 0;

        for unit_id in &spec_units {
            let Some(row) = rows_by_key.get(&(spec.id.clone(), (*unit_id).to_string())) else {
                not_assessed += 1;
                continue;
            };
            if row
                .model_digest
                .as_deref()
                .is_some_and(|digest| digest != model_digest)
            {
                return Err(TransferAdequacyError::MixedModelIdentityWithinDomain {
                    domain_id: domain_id.clone(),
                });
            }
            if row.status != StatisticStatus::NotAssessed {
                let valid_assessed_row = row.model_digest.as_deref() == Some(model_digest)
                    && row
                        .draw_set_digest
                        .as_deref()
                        .is_some_and(|digest| !digest.trim().is_empty())
                    && row
                        .observation_digest
                        .as_deref()
                        .is_some_and(|digest| !digest.trim().is_empty())
                    && row.sample_size > 0
                    && row.observed_value.is_some_and(f64::is_finite)
                    && row.lower_reference.is_some_and(f64::is_finite)
                    && row.upper_reference.is_some_and(f64::is_finite)
                    && row.lower_reference <= row.upper_reference
                    && row
                        .predictive_rank
                        .is_some_and(|rank| rank.is_finite() && (0.0..=1.0).contains(&rank))
                    && row.draw_count >= spec.minimum_draws
                    && row
                        .measurement_standard_error
                        .is_none_or(|value| value.is_finite() && value >= 0.0)
                    && match (
                        row.status,
                        row.observed_value,
                        row.lower_reference,
                        row.upper_reference,
                    ) {
                        (
                            StatisticStatus::WithinReferenceInterval,
                            Some(observed),
                            Some(lower),
                            Some(upper),
                        ) => observed >= lower && observed <= upper,
                        (
                            StatisticStatus::OutsideReferenceInterval,
                            Some(observed),
                            Some(lower),
                            Some(upper),
                        ) => observed < lower || observed > upper,
                        _ => false,
                    };
                if !valid_assessed_row {
                    return Err(TransferAdequacyError::AssessmentStatisticIdentityMismatch {
                        domain_id: domain_id.clone(),
                        statistic_id: spec.id.clone(),
                    });
                }
            }
            match row.status {
                StatisticStatus::WithinReferenceInterval => {
                    assessed += 1;
                    within += 1;
                }
                StatisticStatus::OutsideReferenceInterval => {
                    assessed += 1;
                    outside += 1;
                }
                StatisticStatus::NotAssessed => not_assessed += 1,
            }
        }

        if spec.required && (expected_count != units.len() || not_assessed > 0) {
            required_evidence_complete = false;
        }
        let statistic_status = if outside > 0 {
            TransferStatisticStatus::DiscrepancyDetected
        } else if not_assessed > 0 || assessed == 0 {
            TransferStatisticStatus::NotAssessed
        } else {
            TransferStatisticStatus::NoDiscrepancyDetected
        };

        statistic_results.push(TransferStatisticResult {
            statistic_id: spec.id.clone(),
            construct: spec.construct.clone(),
            measurement_unit: spec.measurement_unit.clone(),
            population: spec.population.clone(),
            context: spec.context.clone(),
            source_family: spec.source_family.clone(),
            source_digest: spec.source_digest.clone(),
            primary: spec.primary,
            required: spec.required,
            expected_units: expected_count,
            assessed_units: assessed,
            within_reference_units: within,
            outside_reference_units: outside,
            not_assessed_units: not_assessed,
            status: statistic_status,
        });
    }

    if domain.assessment.required_evidence_complete != required_evidence_complete {
        return Err(
            TransferAdequacyError::RequiredEvidenceCompletenessMismatch {
                domain_id: domain_id.clone(),
            },
        );
    }

    let primary_specs: BTreeSet<&str> = domain
        .registry
        .statistics
        .iter()
        .filter(|spec| spec.primary)
        .map(|spec| spec.id.as_str())
        .collect();
    let has_primary_discrepancy = statistic_results.iter().any(|result| {
        primary_specs.contains(result.statistic_id.as_str())
            && result.status == TransferStatisticStatus::DiscrepancyDetected
    });
    let primary_fully_assessed = !primary_specs.is_empty()
        && statistic_results
            .iter()
            .filter(|result| primary_specs.contains(result.statistic_id.as_str()))
            .all(|result| result.status != TransferStatisticStatus::NotAssessed);

    let derived_status = if has_primary_discrepancy {
        DomainStatus::PrimaryDiscrepancyDetected
    } else if !required_evidence_complete || !primary_fully_assessed {
        DomainStatus::NotAssessed
    } else {
        DomainStatus::NoPrimaryDiscrepancyDetected
    };
    if domain.assessment.domain_status != derived_status {
        return Err(TransferAdequacyError::DomainStatusMismatch {
            domain_id: domain_id.clone(),
        });
    }

    Ok((
        TransferDomainResult {
            domain_id: domain_id.clone(),
            model_digest: domain.model_digest.clone(),
            axis: domain.axis,
            role: domain.assessment.role,
            independent_unit_kind: domain.manifest.independent_unit,
            split_manifest_id: domain.manifest.id.clone(),
            split_manifest_digest: domain.manifest.manifest_digest.clone(),
            required_evidence_complete,
            status: derived_status,
            statistics: statistic_results,
        },
        domain.model_digest.clone(),
    ))
}

/// Qualify one frozen model across source-validation and target-challenge
/// domains. All target predictions must use the same original calibration-data
/// digest and exact model digest; any target adaptation requires a new model
/// identity and a separately named protocol.
pub fn assess_zero_shot_transfer(
    domains: &[TransferDomainEvidence],
) -> Result<TransferAssessment, TransferAdequacyError> {
    if domains.is_empty() {
        return Err(TransferAdequacyError::MissingSourceValidationDomain);
    }

    let first = &domains[0];
    first.registry.validate()?;
    first.manifest.validate()?;
    if first.registry.version.trim().is_empty() {
        return Err(TransferAdequacyError::RegistryVersionMismatch {
            domain_id: first.domain_id.clone(),
        });
    }

    let expected_schema = semantic_schema(&first.registry);
    let expected_registry_version = first.registry.version.as_str();
    let expected_calibration_digest = first.manifest.calibration_data_digest.as_str();
    let expected_central_mass = first.assessment.central_mass;
    if !expected_central_mass.is_finite()
        || expected_central_mass <= 0.0
        || expected_central_mass >= 1.0
    {
        return Err(TransferAdequacyError::InvalidCentralMass);
    }

    let mut seen_domains = BTreeSet::new();
    let mut seen_manifest_ids = BTreeSet::new();
    let mut seen_manifest_digests = BTreeSet::new();
    let mut results = Vec::with_capacity(domains.len());
    let mut model_digest: Option<String> = None;
    let mut source_validation_domain_count = 0;
    let mut held_out_challenge_domain_count = 0;

    for domain in domains {
        if !seen_domains.insert(domain.domain_id.as_str()) {
            return Err(TransferAdequacyError::DuplicateDomainId {
                domain_id: domain.domain_id.clone(),
            });
        }
        if !seen_manifest_ids.insert(domain.manifest.id.as_str()) {
            return Err(TransferAdequacyError::DuplicateManifestId {
                manifest_id: domain.manifest.id.clone(),
            });
        }
        if !seen_manifest_digests.insert(domain.manifest.manifest_digest.as_str()) {
            return Err(TransferAdequacyError::DuplicateManifestDigest {
                manifest_digest: domain.manifest.manifest_digest.clone(),
            });
        }

        let (result, domain_model_digest) = evaluate_domain(
            domain,
            &expected_schema,
            expected_registry_version,
            expected_calibration_digest,
            expected_central_mass,
        )?;
        if model_digest
            .as_deref()
            .is_some_and(|expected| expected != domain_model_digest)
        {
            return Err(TransferAdequacyError::ModelIdentityMismatchAcrossDomains {
                domain_id: domain.domain_id.clone(),
            });
        }
        model_digest.get_or_insert(domain_model_digest);

        match result.role {
            SplitRole::Validation => source_validation_domain_count += 1,
            SplitRole::Challenge => held_out_challenge_domain_count += 1,
            SplitRole::Calibration => unreachable!("domain roles reject calibration"),
        }
        results.push(result);
    }

    if source_validation_domain_count == 0 {
        return Err(TransferAdequacyError::MissingSourceValidationDomain);
    }
    if held_out_challenge_domain_count == 0 {
        return Err(TransferAdequacyError::MissingHeldOutChallengeDomain);
    }

    let evidence_complete = results.iter().all(|domain| {
        domain.required_evidence_complete && domain.status != DomainStatus::NotAssessed
    });
    let source_domain_not_qualified = results.iter().any(|domain| {
        domain.role == SplitRole::Validation
            && domain.status != DomainStatus::NoPrimaryDiscrepancyDetected
    });
    let has_challenge_discrepancy = results.iter().any(|domain| {
        domain.role == SplitRole::Challenge
            && domain.status == DomainStatus::PrimaryDiscrepancyDetected
    });
    // A known target discrepancy remains visible even when a different
    // declared target is under-assessed. However, target failures cannot be
    // called transfer discrepancies unless the in-domain source baseline
    // itself is qualified.
    let claim_ceiling = if !source_domain_not_qualified && has_challenge_discrepancy {
        TransferClaimCeiling::TransferDiscrepancyDetected
    } else if !evidence_complete || source_domain_not_qualified {
        TransferClaimCeiling::NotAssessed
    } else {
        TransferClaimCeiling::NoPrimaryDiscrepancyDetectedAcrossDeclaredTransferDomains
    };

    Ok(TransferAssessment {
        registry_version: expected_registry_version.to_string(),
        model_digest: model_digest.unwrap_or_default(),
        calibration_data_digest: expected_calibration_digest.to_string(),
        central_mass: expected_central_mass,
        evidence_complete,
        source_validation_domain_count,
        held_out_challenge_domain_count,
        claim_ceiling,
        domains: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_adequacy::{
        IndependentUnitKind, ObservedStatistic, PredictiveDraw, StatisticSpec,
        assess_predictive_partition,
    };

    const MODEL: &str = "sha256-frozen-transfer-model";
    const CALIBRATION: &str = "sha256-frozen-calibration";

    fn domain(
        domain_id: &str,
        axis: TransferAxis,
        role: SplitRole,
        independent_unit: crate::model_adequacy::IndependentUnitKind,
        population: &str,
        context: &str,
        source_family: &str,
        source_digest: &str,
        unit_id: &str,
        observed_value: f64,
        model_digest: &str,
        calibration_digest: &str,
        predictive_kind: PredictiveKind,
        draw_values: &[f64],
    ) -> TransferDomainEvidence {
        let registry = StatisticRegistry {
            version: "transfer-registry-v1".to_string(),
            statistics: vec![StatisticSpec {
                id: "essential-service-coverage".to_string(),
                construct: "essential household service coverage".to_string(),
                measurement_unit: "coverage_fraction".to_string(),
                population: population.to_string(),
                context: context.to_string(),
                source_family: source_family.to_string(),
                source_digest: source_digest.to_string(),
                independent_unit,
                role,
                primary: true,
                required: true,
                minimum_draws: 5,
            }],
        };
        let manifest = SplitManifest {
            id: format!("{domain_id}-split"),
            independent_unit,
            calibration_data_digest: calibration_digest.to_string(),
            manifest_digest: format!("sha256-{domain_id}-manifest"),
            calibration_units: vec![format!("{domain_id}-calibration-unit")],
            validation_units: vec![format!("{domain_id}-reserved-validation-unit")],
            challenge_units: if role == SplitRole::Challenge {
                vec![unit_id.to_string()]
            } else {
                Vec::new()
            },
        };
        let target_units = match role {
            SplitRole::Validation => vec![unit_id.to_string()],
            SplitRole::Challenge => vec![unit_id.to_string()],
            SplitRole::Calibration => Vec::new(),
        };
        let assessment_manifest = if role == SplitRole::Validation {
            SplitManifest {
                validation_units: target_units.clone(),
                ..manifest.clone()
            }
        } else {
            SplitManifest {
                challenge_units: target_units.clone(),
                ..manifest.clone()
            }
        };
        let observation = ObservedStatistic {
            statistic_id: "essential-service-coverage".to_string(),
            independent_unit_id: unit_id.to_string(),
            measurement_unit: "coverage_fraction".to_string(),
            source_digest: source_digest.to_string(),
            sample_size: 100,
            value: observed_value,
            measurement_standard_error: Some(0.01),
            observation_digest: format!("sha256-{domain_id}-observation"),
        };
        let draws: Vec<PredictiveDraw> = draw_values
            .iter()
            .enumerate()
            .map(|(index, value)| PredictiveDraw {
                kind: predictive_kind,
                statistic_id: "essential-service-coverage".to_string(),
                independent_unit_id: unit_id.to_string(),
                sample_size: 100,
                draw_index: index as u64,
                value: *value,
                model_digest: model_digest.to_string(),
                draw_set_digest: format!("sha256-{domain_id}-draws"),
                calibration_data_digest: if predictive_kind == PredictiveKind::Split {
                    Some(calibration_digest.to_string())
                } else {
                    None
                },
            })
            .collect();
        let assessment = assess_predictive_partition(
            &registry,
            &assessment_manifest,
            &[observation],
            &draws,
            role,
            predictive_kind,
            0.9,
        )
        .expect("synthetic transfer domain assessment");
        TransferDomainEvidence {
            domain_id: domain_id.to_string(),
            model_digest: model_digest.to_string(),
            axis,
            registry,
            manifest: assessment_manifest,
            assessment,
        }
    }

    fn standard_domains() -> Vec<TransferDomainEvidence> {
        vec![
            domain(
                "source-region",
                TransferAxis::Region,
                SplitRole::Validation,
                IndependentUnitKind::Region,
                "source-households",
                "source-period",
                "survey-source",
                "sha256-source-region",
                "region-a",
                0.5,
                MODEL,
                CALIBRATION,
                PredictiveKind::Split,
                &[0.45, 0.48, 0.50, 0.52, 0.55],
            ),
            domain(
                "target-site",
                TransferAxis::Institution,
                SplitRole::Challenge,
                IndependentUnitKind::Site,
                "target-households",
                "target-institution-period",
                "administrative-source",
                "sha256-target-site",
                "site-b",
                0.9,
                MODEL,
                CALIBRATION,
                PredictiveKind::Split,
                &[0.45, 0.48, 0.50, 0.52, 0.55],
            ),
            domain(
                "target-time",
                TransferAxis::TimePeriod,
                SplitRole::Challenge,
                IndependentUnitKind::TimeBlock,
                "source-households",
                "future-period",
                "survey-source",
                "sha256-target-time",
                "time-block-c",
                0.5,
                MODEL,
                CALIBRATION,
                PredictiveKind::Split,
                &[0.45, 0.48, 0.50, 0.52, 0.55],
            ),
        ]
    }

    #[test]
    fn reports_transfer_by_domain_without_averaging_away_target_failure() {
        let report = assess_zero_shot_transfer(&standard_domains()).expect("transfer report");
        assert_eq!(report.model_digest, MODEL);
        assert_eq!(report.calibration_data_digest, CALIBRATION);
        assert_eq!(report.source_validation_domain_count, 1);
        assert_eq!(report.held_out_challenge_domain_count, 2);
        assert_eq!(
            report.claim_ceiling,
            TransferClaimCeiling::TransferDiscrepancyDetected
        );
        assert_eq!(
            report.domains[0].status,
            DomainStatus::NoPrimaryDiscrepancyDetected
        );
        assert_eq!(
            report.domains[1].status,
            DomainStatus::PrimaryDiscrepancyDetected
        );
        assert_eq!(
            report.domains[2].status,
            DomainStatus::NoPrimaryDiscrepancyDetected
        );
        assert_eq!(
            report.domains[0].independent_unit_kind,
            IndependentUnitKind::Region
        );
        assert_eq!(
            report.domains[1].independent_unit_kind,
            IndependentUnitKind::Site
        );
        assert_eq!(
            report.domains[2].independent_unit_kind,
            IndependentUnitKind::TimeBlock
        );
    }

    #[test]
    fn model_identity_change_is_rejected_as_target_adaptation() {
        let mut domains = standard_domains();
        domains[1] = domain(
            "target-site",
            TransferAxis::Institution,
            SplitRole::Challenge,
            IndependentUnitKind::Site,
            "target-households",
            "target-institution-period",
            "administrative-source",
            "sha256-target-site",
            "site-b",
            0.9,
            "sha256-adapted-model",
            CALIBRATION,
            PredictiveKind::Split,
            &[0.45, 0.48, 0.50, 0.52, 0.55],
        );
        assert!(matches!(
            assess_zero_shot_transfer(&domains),
            Err(TransferAdequacyError::ModelIdentityMismatchAcrossDomains { .. })
        ));
    }

    #[test]
    fn calibration_dataset_change_is_rejected() {
        let mut domains = standard_domains();
        domains[1].manifest.calibration_data_digest = "sha256-different-calibration".to_string();
        assert!(matches!(
            assess_zero_shot_transfer(&domains),
            Err(TransferAdequacyError::CalibrationDataDigestMismatch { .. })
        ));
    }

    #[test]
    fn prior_predictive_assessment_cannot_qualify_transfer() {
        let mut domains = standard_domains();
        domains[1] = domain(
            "target-site",
            TransferAxis::Institution,
            SplitRole::Challenge,
            IndependentUnitKind::Site,
            "target-households",
            "target-institution-period",
            "administrative-source",
            "sha256-target-site",
            "site-b",
            0.9,
            MODEL,
            CALIBRATION,
            PredictiveKind::Prior,
            &[0.45, 0.48, 0.50, 0.52, 0.55],
        );
        assert!(matches!(
            assess_zero_shot_transfer(&domains),
            Err(TransferAdequacyError::InvalidPredictiveKind { .. })
        ));
    }

    #[test]
    fn differing_predictive_interval_mass_is_rejected() {
        let mut domains = standard_domains();
        domains[1].assessment.central_mass = 0.95;
        assert!(matches!(
            assess_zero_shot_transfer(&domains),
            Err(TransferAdequacyError::AssessmentCentralMassMismatch { .. })
        ));
    }

    #[test]
    fn semantic_statistic_mismatch_is_rejected() {
        let mut domains = standard_domains();
        domains[1].registry.statistics[0].measurement_unit = "percent".to_string();
        assert!(matches!(
            assess_zero_shot_transfer(&domains),
            Err(TransferAdequacyError::StatisticSchemaMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_domain_and_manifest_identities_are_rejected() {
        let mut duplicate_domain = standard_domains();
        duplicate_domain[1].domain_id = duplicate_domain[0].domain_id.clone();
        assert!(matches!(
            assess_zero_shot_transfer(&duplicate_domain),
            Err(TransferAdequacyError::DuplicateDomainId { .. })
        ));

        let mut duplicate_manifest = standard_domains();
        duplicate_manifest[1].manifest.id = duplicate_manifest[0].manifest.id.clone();
        assert!(matches!(
            assess_zero_shot_transfer(&duplicate_manifest),
            Err(TransferAdequacyError::DuplicateManifestId { .. })
        ));
    }

    #[test]
    fn requires_source_validation_and_challenge_target() {
        let mut no_source = standard_domains();
        no_source[0].assessment.role = SplitRole::Challenge;
        assert!(assess_zero_shot_transfer(&no_source).is_err());

        let no_target = vec![domain(
            "only-source",
            TransferAxis::Region,
            SplitRole::Validation,
            IndependentUnitKind::Region,
            "source-households",
            "source-period",
            "survey-source",
            "sha256-source-region",
            "region-a",
            0.5,
            MODEL,
            CALIBRATION,
            PredictiveKind::Split,
            &[0.45, 0.48, 0.50, 0.52, 0.55],
        )];
        assert!(matches!(
            assess_zero_shot_transfer(&no_target),
            Err(TransferAdequacyError::MissingHeldOutChallengeDomain)
        ));
    }

    #[test]
    fn known_target_failure_is_not_hidden_by_another_unassessed_target() {
        let mut domains = standard_domains();
        let spec = domains[2].registry.statistics[0].clone();
        let manifest = domains[2].manifest.clone();
        let observation = ObservedStatistic {
            statistic_id: spec.id.clone(),
            independent_unit_id: "time-block-c".to_string(),
            measurement_unit: spec.measurement_unit.clone(),
            source_digest: spec.source_digest.clone(),
            sample_size: 100,
            value: 0.5,
            measurement_standard_error: None,
            observation_digest: "sha256-time-block-observation".to_string(),
        };
        domains[2].assessment = assess_predictive_partition(
            &domains[2].registry,
            &manifest,
            &[observation],
            &[],
            SplitRole::Challenge,
            PredictiveKind::Split,
            0.9,
        )
        .expect("underpowered target is explicit");

        let report =
            assess_zero_shot_transfer(&domains).expect("partially assessed transfer report");
        assert_eq!(
            report.claim_ceiling,
            TransferClaimCeiling::TransferDiscrepancyDetected
        );
        assert!(!report.evidence_complete);
        assert_eq!(
            report.domains[1].status,
            DomainStatus::PrimaryDiscrepancyDetected
        );
        assert_eq!(report.domains[2].status, DomainStatus::NotAssessed);
    }

    #[test]
    fn source_validation_failure_prevents_a_transfer_claim() {
        let mut domains = standard_domains();
        domains[0] = domain(
            "source-region",
            TransferAxis::Region,
            SplitRole::Validation,
            IndependentUnitKind::Region,
            "source-households",
            "source-period",
            "survey-source",
            "sha256-source-region",
            "region-a",
            0.9,
            MODEL,
            CALIBRATION,
            PredictiveKind::Split,
            &[0.45, 0.48, 0.50, 0.52, 0.55],
        );

        let report = assess_zero_shot_transfer(&domains).expect("source discrepancy report");
        assert_eq!(
            report.domains[0].status,
            DomainStatus::PrimaryDiscrepancyDetected
        );
        assert_eq!(report.claim_ceiling, TransferClaimCeiling::NotAssessed);
    }

    #[test]
    fn incomplete_required_target_evidence_is_not_a_transfer_pass() {
        let mut domains = standard_domains();
        let spec = domains[1].registry.statistics[0].clone();
        let manifest = domains[1].manifest.clone();
        let observation = ObservedStatistic {
            statistic_id: spec.id.clone(),
            independent_unit_id: "site-b".to_string(),
            measurement_unit: spec.measurement_unit.clone(),
            source_digest: spec.source_digest.clone(),
            sample_size: 100,
            value: 0.5,
            measurement_standard_error: None,
            observation_digest: "sha256-site-b-observation".to_string(),
        };
        let assessment = assess_predictive_partition(
            &domains[1].registry,
            &manifest,
            &[observation],
            &[],
            SplitRole::Challenge,
            PredictiveKind::Split,
            0.9,
        )
        .expect("an underpowered assessment is explicit, not an error");
        domains[1].assessment = assessment;

        let report = assess_zero_shot_transfer(&domains).expect("not-assessed transfer report");
        assert_eq!(
            domains[1].assessment.domain_status,
            DomainStatus::NotAssessed
        );
        assert_eq!(report.domains[1].status, DomainStatus::NotAssessed);
        assert_eq!(report.claim_ceiling, TransferClaimCeiling::NotAssessed);
        assert!(!report.evidence_complete);
    }
}
