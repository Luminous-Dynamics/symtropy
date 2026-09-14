// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SEL-10E2B current-species cross-model robustness execution.
//!
//! The matrix preserves every preregistered model row and never treats models as
//! votes. Robust support/contradiction requires an explicit pairwise-independent
//! clique witness under the frozen E2A design. Persisted reports are
//! representations; current report authority requires fresh model-bound status
//! capabilities plus current E2A authority.

use crate::{
    CurrentCrossModelRobustnessAuthority, CurrentCrossModelRobustnessDesign,
    CurrentCrossModelRobustnessDesignDigest, FaultDomainIndependencePolicyDigest,
    MissingModelPolicy, ModelFaultDomainProfileDigest, ModelSpecificClassificationDesignRef,
    PairwiseFaultDomainDisposition, RobustnessDesignError, SpeciesModelDesignRecord,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
use symtropy_evolution_core::{
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId,
    CurrentSpeciesClassificationDesignDigest, CurrentSpeciesStatus,
    CurrentSpeciesStatusEvidenceDigest, LineageDivergenceHistoryDesignDigest,
    SpeciesModelApplicabilityDisposition, ValidatedCurrentSpeciesStatus,
};
use symtropy_species_concept::{
    OpenSpeciesConceptIdentity, SpeciesConceptFamilyDescriptorDigest,
};
use symtropy_species_concept_general_lineage::{
    GeneralLineageClassificationDesignDigest, GeneralLineageModelApplicabilityDisposition,
    GeneralLineageSpeciesEvidenceDigest, GeneralLineageSpeciesStatus,
    ValidatedGeneralLineageSpeciesEvidence,
};
use symtropy_species_concept_relations::{
    SemanticDependencyClass, SpeciesConceptRelationEvidenceDigest,
};

pub const CROSS_MODEL_CURRENT_SPECIES_REPORT_VERSION: u32 = 1;
pub const CROSS_MODEL_CURRENT_SPECIES_MAX_MODELS: usize = 32;
pub const CROSS_MODEL_INDEPENDENCE_SEARCH_MAX_STEPS: u64 = 1_000_000;
const REPORT_DOMAIN: &[u8] = b"symtropy:species-concept:current-robustness-report:v1\0";
const SURFACE_DOMAIN: &[u8] = b"symtropy:species-concept:model-evidence-surface:v1\0";
const RULE_DOMAIN: &[u8] = b"symtropy:species-concept:current-robustness-report-rule:v1\0";
const RULE_SPEC: &[u8] = b"current-species cross-model robustness report v1: execute only against current E2A authority; retain exactly one row per preregistered conceptual family including explicit missing rows; no undeclared family insertion or contradictory-row deletion; preserve family-specific typed current-status semantics; outside-domain and missing evidence are not negative votes; support, contradiction, and concordant non-support each require their own canonical pairwise-eligible clique witness at the frozen minimum independent coverage; non-support is not contradiction; pairwise independence is not transitive; any resolved cross-model conclusion disagreement remains model-dependent, support plus contradiction is mixed, all-outside is explicit; no majority count, scalar taxonomy score, historical robustness, nomenclature, philosophical winner, or universal taxonomy truth";

macro_rules! digest_type {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name([u8; 32]);
        impl $name {
            pub fn new(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }
            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "("))?;
                fmt_hex(&self.0, f)?;
                write!(f, ")")
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt_hex(&self.0, f)
            }
        }
    };
}

digest_type!(ModelSpecificEvidenceSurfaceDigest);
digest_type!(CrossModelCurrentSpeciesReportDigest);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossModelOutcomeDisposition {
    Supports,
    DoesNotSupport,
    Contradicts,
    InsufficientEvidence,
    OutsideValidityDomain,
    MissingCurrentCapability,
}

impl CrossModelOutcomeDisposition {
    fn tag(self) -> u8 {
        match self {
            Self::Supports => 0,
            Self::DoesNotSupport => 1,
            Self::Contradicts => 2,
            Self::InsufficientEvidence => 3,
            Self::OutsideValidityDomain => 4,
            Self::MissingCurrentCapability => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelBoundCurrentStatus {
    StrictBiological {
        design_digest: CurrentSpeciesClassificationDesignDigest,
        evidence_digest: CurrentSpeciesStatusEvidenceDigest,
        applicability: SpeciesModelApplicabilityDisposition,
        status: CurrentSpeciesStatus,
    },
    GeneralLineage {
        design_digest: GeneralLineageClassificationDesignDigest,
        evidence_digest: GeneralLineageSpeciesEvidenceDigest,
        applicability: GeneralLineageModelApplicabilityDisposition,
        status: GeneralLineageSpeciesStatus,
    },
    MissingCurrentCapability {
        reason: AnalysisAuthorityRef,
    },
}

impl ModelBoundCurrentStatus {
    pub fn disposition(&self) -> CrossModelOutcomeDisposition {
        match self {
            Self::StrictBiological { status, .. } => match status {
                CurrentSpeciesStatus::SupportedUnderModel => CrossModelOutcomeDisposition::Supports,
                CurrentSpeciesStatus::NotSupportedUnderModel => {
                    CrossModelOutcomeDisposition::DoesNotSupport
                }
                CurrentSpeciesStatus::ContradictedUnderModel => {
                    CrossModelOutcomeDisposition::Contradicts
                }
                CurrentSpeciesStatus::InsufficientEvidence => {
                    CrossModelOutcomeDisposition::InsufficientEvidence
                }
                CurrentSpeciesStatus::OutsideModelValidityDomain => {
                    CrossModelOutcomeDisposition::OutsideValidityDomain
                }
            },
            Self::GeneralLineage { status, .. } => match status {
                GeneralLineageSpeciesStatus::SupportedUnderGeneralLineageModel => {
                    CrossModelOutcomeDisposition::Supports
                }
                GeneralLineageSpeciesStatus::NotSupportedUnderGeneralLineageModel => {
                    CrossModelOutcomeDisposition::DoesNotSupport
                }
                GeneralLineageSpeciesStatus::ContradictedUnderGeneralLineageModel => {
                    CrossModelOutcomeDisposition::Contradicts
                }
                GeneralLineageSpeciesStatus::InsufficientIndependentEvidence => {
                    CrossModelOutcomeDisposition::InsufficientEvidence
                }
                GeneralLineageSpeciesStatus::OutsideModelValidityDomain => {
                    CrossModelOutcomeDisposition::OutsideValidityDomain
                }
            },
            Self::MissingCurrentCapability { .. } => {
                CrossModelOutcomeDisposition::MissingCurrentCapability
            }
        }
    }

    fn validate_local(&self) -> Result<(), CrossModelRobustnessReportError> {
        match self {
            Self::StrictBiological {
                applicability,
                status,
                ..
            } => match applicability {
                SpeciesModelApplicabilityDisposition::OutsideValidityDomain
                    if *status != CurrentSpeciesStatus::OutsideModelValidityDomain =>
                {
                    Err(CrossModelRobustnessReportError::ApplicabilityStatusMismatch)
                }
                SpeciesModelApplicabilityDisposition::Unavailable
                    if *status != CurrentSpeciesStatus::InsufficientEvidence =>
                {
                    Err(CrossModelRobustnessReportError::ApplicabilityStatusMismatch)
                }
                _ => Ok(()),
            },
            Self::GeneralLineage {
                applicability,
                status,
                ..
            } => match applicability {
                GeneralLineageModelApplicabilityDisposition::OutsideModelValidityDomain
                    if *status != GeneralLineageSpeciesStatus::OutsideModelValidityDomain =>
                {
                    Err(CrossModelRobustnessReportError::ApplicabilityStatusMismatch)
                }
                GeneralLineageModelApplicabilityDisposition::Unavailable
                    if *status
                        != GeneralLineageSpeciesStatus::InsufficientIndependentEvidence =>
                {
                    Err(CrossModelRobustnessReportError::ApplicabilityStatusMismatch)
                }
                _ => Ok(()),
            },
            Self::MissingCurrentCapability { reason } => {
                validate_authority(reason, "missing_current_capability_revision")
            }
        }
    }

    fn put(&self, digest: &mut Sha256) {
        match self {
            Self::StrictBiological {
                design_digest,
                evidence_digest,
                applicability,
                status,
            } => {
                digest.update([0]);
                digest.update(design_digest.as_bytes());
                digest.update(evidence_digest.as_bytes());
                digest.update([match applicability {
                    SpeciesModelApplicabilityDisposition::InsideValidityDomain => 0,
                    SpeciesModelApplicabilityDisposition::OutsideValidityDomain => 1,
                    SpeciesModelApplicabilityDisposition::Unavailable => 2,
                }]);
                digest.update([match status {
                    CurrentSpeciesStatus::SupportedUnderModel => 0,
                    CurrentSpeciesStatus::NotSupportedUnderModel => 1,
                    CurrentSpeciesStatus::ContradictedUnderModel => 2,
                    CurrentSpeciesStatus::InsufficientEvidence => 3,
                    CurrentSpeciesStatus::OutsideModelValidityDomain => 4,
                }]);
            }
            Self::GeneralLineage {
                design_digest,
                evidence_digest,
                applicability,
                status,
            } => {
                digest.update([1]);
                digest.update(design_digest.as_bytes());
                digest.update(evidence_digest.as_bytes());
                digest.update([match applicability {
                    GeneralLineageModelApplicabilityDisposition::InDomain => 0,
                    GeneralLineageModelApplicabilityDisposition::OutsideModelValidityDomain => 1,
                    GeneralLineageModelApplicabilityDisposition::Unavailable => 2,
                }]);
                digest.update([match status {
                    GeneralLineageSpeciesStatus::SupportedUnderGeneralLineageModel => 0,
                    GeneralLineageSpeciesStatus::NotSupportedUnderGeneralLineageModel => 1,
                    GeneralLineageSpeciesStatus::ContradictedUnderGeneralLineageModel => 2,
                    GeneralLineageSpeciesStatus::InsufficientIndependentEvidence => 3,
                    GeneralLineageSpeciesStatus::OutsideModelValidityDomain => 4,
                }]);
            }
            Self::MissingCurrentCapability { reason } => {
                digest.update([2]);
                put_authority(digest, reason);
            }
        }
        digest.update([self.disposition().tag()]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossModelOutcomeRow {
    pub conceptual_identity: OpenSpeciesConceptIdentity,
    pub descriptor_digest: SpeciesConceptFamilyDescriptorDigest,
    pub classification_design: ModelSpecificClassificationDesignRef,
    pub lineage_a: AnalysisAuthorityRef,
    pub lineage_b: AnalysisAuthorityRef,
    pub lineage_history_design_digest: LineageDivergenceHistoryDesignDigest,
    pub fault_profile_digest: ModelFaultDomainProfileDigest,
    pub evidence_surface_digest: ModelSpecificEvidenceSurfaceDigest,
    pub result: ModelBoundCurrentStatus,
}

impl CrossModelOutcomeRow {
    pub fn disposition(&self) -> CrossModelOutcomeDisposition {
        self.result.disposition()
    }

    fn validate_against(
        &self,
        model: &SpeciesModelDesignRecord,
    ) -> Result<(), CrossModelRobustnessReportError> {
        if self.conceptual_identity != *model.conceptual_identity()
            || self.descriptor_digest != model.descriptor_digest
            || self.classification_design != model.classification_design
            || self.lineage_a != model.lineage_a
            || self.lineage_b != model.lineage_b
            || self.lineage_history_design_digest != model.lineage_history_design_digest
            || self.fault_profile_digest != model.fault_profile_digest
        {
            return Err(CrossModelRobustnessReportError::RowModelBindingMismatch);
        }
        if self.evidence_surface_digest
            != model_specific_evidence_surface_digest(&model.classification_design)
        {
            return Err(CrossModelRobustnessReportError::EvidenceSurfaceDigestMismatch);
        }
        self.result.validate_local()?;
        match (&model.classification_design, &self.result) {
            (
                ModelSpecificClassificationDesignRef::StrictBiological { design_digest, .. },
                ModelBoundCurrentStatus::StrictBiological {
                    design_digest: result_design,
                    ..
                },
            ) if design_digest == result_design => Ok(()),
            (
                ModelSpecificClassificationDesignRef::GeneralLineage { design_digest },
                ModelBoundCurrentStatus::GeneralLineage {
                    design_digest: result_design,
                    ..
                },
            ) if design_digest == result_design => Ok(()),
            (_, ModelBoundCurrentStatus::MissingCurrentCapability { .. }) => Ok(()),
            _ => Err(CrossModelRobustnessReportError::ResultFamilyMismatch),
        }
    }

    fn put(&self, digest: &mut Sha256) {
        put_identity(digest, &self.conceptual_identity);
        digest.update(self.descriptor_digest.as_bytes());
        put_classification_design(digest, &self.classification_design);
        put_authority(digest, &self.lineage_a);
        put_authority(digest, &self.lineage_b);
        digest.update(self.lineage_history_design_digest.as_bytes());
        digest.update(self.fault_profile_digest.as_bytes());
        digest.update(self.evidence_surface_digest.as_bytes());
        self.result.put(digest);
    }
}

#[derive(Debug)]
#[must_use = "current model rows are ephemeral capabilities; persist CrossModelOutcomeRow instead"]
pub struct CurrentModelOutcomeRow {
    row: CrossModelOutcomeRow,
}

impl CurrentModelOutcomeRow {
    pub fn strict_biological(
        model: &SpeciesModelDesignRecord,
        evidence: &ValidatedCurrentSpeciesStatus<'_>,
    ) -> Result<Self, CrossModelRobustnessReportError> {
        let expected_design = match &model.classification_design {
            ModelSpecificClassificationDesignRef::StrictBiological { design_digest, .. } => {
                *design_digest
            }
            _ => return Err(CrossModelRobustnessReportError::ResultFamilyMismatch),
        };
        if evidence.design_digest() != expected_design {
            return Err(CrossModelRobustnessReportError::ResultDesignMismatch);
        }
        let raw = evidence.evidence();
        let row = CrossModelOutcomeRow {
            conceptual_identity: model.conceptual_identity().clone(),
            descriptor_digest: model.descriptor_digest,
            classification_design: model.classification_design.clone(),
            lineage_a: model.lineage_a.clone(),
            lineage_b: model.lineage_b.clone(),
            lineage_history_design_digest: model.lineage_history_design_digest,
            fault_profile_digest: model.fault_profile_digest,
            evidence_surface_digest: model_specific_evidence_surface_digest(
                &model.classification_design,
            ),
            result: ModelBoundCurrentStatus::StrictBiological {
                design_digest: evidence.design_digest(),
                evidence_digest: evidence.evidence_digest(),
                applicability: raw.applicability.disposition,
                status: raw.status,
            },
        };
        row.validate_against(model)?;
        Ok(Self { row })
    }

    pub fn general_lineage(
        model: &SpeciesModelDesignRecord,
        evidence: &ValidatedGeneralLineageSpeciesEvidence<'_>,
    ) -> Result<Self, CrossModelRobustnessReportError> {
        let expected_design = match &model.classification_design {
            ModelSpecificClassificationDesignRef::GeneralLineage { design_digest } => {
                *design_digest
            }
            _ => return Err(CrossModelRobustnessReportError::ResultFamilyMismatch),
        };
        let raw = evidence.evidence();
        if raw.design_digest != expected_design {
            return Err(CrossModelRobustnessReportError::ResultDesignMismatch);
        }
        let row = CrossModelOutcomeRow {
            conceptual_identity: model.conceptual_identity().clone(),
            descriptor_digest: model.descriptor_digest,
            classification_design: model.classification_design.clone(),
            lineage_a: model.lineage_a.clone(),
            lineage_b: model.lineage_b.clone(),
            lineage_history_design_digest: model.lineage_history_design_digest,
            fault_profile_digest: model.fault_profile_digest,
            evidence_surface_digest: model_specific_evidence_surface_digest(
                &model.classification_design,
            ),
            result: ModelBoundCurrentStatus::GeneralLineage {
                design_digest: raw.design_digest,
                evidence_digest: evidence.evidence_digest(),
                applicability: raw.applicability.disposition,
                status: raw.status,
            },
        };
        row.validate_against(model)?;
        Ok(Self { row })
    }

    pub fn missing_current_capability(
        model: &SpeciesModelDesignRecord,
        reason: AnalysisAuthorityRef,
    ) -> Result<Self, CrossModelRobustnessReportError> {
        validate_authority(&reason, "missing_current_capability_revision")?;
        let row = CrossModelOutcomeRow {
            conceptual_identity: model.conceptual_identity().clone(),
            descriptor_digest: model.descriptor_digest,
            classification_design: model.classification_design.clone(),
            lineage_a: model.lineage_a.clone(),
            lineage_b: model.lineage_b.clone(),
            lineage_history_design_digest: model.lineage_history_design_digest,
            fault_profile_digest: model.fault_profile_digest,
            evidence_surface_digest: model_specific_evidence_surface_digest(
                &model.classification_design,
            ),
            result: ModelBoundCurrentStatus::MissingCurrentCapability { reason },
        };
        row.validate_against(model)?;
        Ok(Self { row })
    }

    pub fn row(&self) -> &CrossModelOutcomeRow {
        &self.row
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossModelPairContext {
    pub left: OpenSpeciesConceptIdentity,
    pub right: OpenSpeciesConceptIdentity,
    pub relation_evidence_digest: SpeciesConceptRelationEvidenceDigest,
    pub semantic_dependency: Option<SemanticDependencyClass>,
    pub left_profile_digest: ModelFaultDomainProfileDigest,
    pub right_profile_digest: ModelFaultDomainProfileDigest,
    pub policy_digest: FaultDomainIndependencePolicyDigest,
    pub fault_disposition: PairwiseFaultDomainDisposition,
    pub fault_qualification_authority: AnalysisAuthorityRef,
}

impl CrossModelPairContext {
    fn put(&self, digest: &mut Sha256) {
        put_identity(digest, &self.left);
        put_identity(digest, &self.right);
        digest.update(self.relation_evidence_digest.as_bytes());
        digest.update([match self.semantic_dependency {
            None => 0,
            Some(SemanticDependencyClass::NestedOrCriterionDependent) => 1,
            Some(SemanticDependencyClass::Overlapping) => 2,
            Some(SemanticDependencyClass::PotentiallyNonNested) => 3,
        }]);
        digest.update(self.left_profile_digest.as_bytes());
        digest.update(self.right_profile_digest.as_bytes());
        digest.update(self.policy_digest.as_bytes());
        digest.update([match self.fault_disposition {
            PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct => 0,
            PairwiseFaultDomainDisposition::KnownDependent => 1,
            PairwiseFaultDomainDisposition::UnknownOrUnqualified => 2,
        }]);
        put_authority(digest, &self.fault_qualification_authority);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossModelCurrentSpeciesRobustnessStatus {
    RobustSupportAcrossQualifiedIndependentCoverage,
    RobustContradictionAcrossQualifiedIndependentCoverage,
    ConcordantNonSupportAcrossQualifiedIndependentCoverage,
    ModelDependentConclusion,
    MixedSupportAndContradiction,
    InsufficientIndependentModelCoverage,
    AllModelsOutsideValidityDomain,
}

impl CrossModelCurrentSpeciesRobustnessStatus {
    fn tag(self) -> u8 {
        match self {
            Self::RobustSupportAcrossQualifiedIndependentCoverage => 0,
            Self::RobustContradictionAcrossQualifiedIndependentCoverage => 1,
            Self::ConcordantNonSupportAcrossQualifiedIndependentCoverage => 2,
            Self::ModelDependentConclusion => 3,
            Self::MixedSupportAndContradiction => 4,
            Self::InsufficientIndependentModelCoverage => 5,
            Self::AllModelsOutsideValidityDomain => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossModelCurrentSpeciesReport {
    report_version: u32,
    pub design: CurrentCrossModelRobustnessDesign,
    pub design_digest: CurrentCrossModelRobustnessDesignDigest,
    pub rows: Vec<CrossModelOutcomeRow>,
    pub pairwise_context: Vec<CrossModelPairContext>,
    pub support_independence_witness: Vec<OpenSpeciesConceptIdentity>,
    pub non_support_independence_witness: Vec<OpenSpeciesConceptIdentity>,
    pub contradiction_independence_witness: Vec<OpenSpeciesConceptIdentity>,
    pub status: CrossModelCurrentSpeciesRobustnessStatus,
    pub rule_authority: AnalysisAuthorityRef,
}

impl CrossModelCurrentSpeciesReport {
    pub fn evaluate(
        authority: &CurrentCrossModelRobustnessAuthority<'_>,
        current_rows: impl IntoIterator<Item = CurrentModelOutcomeRow>,
    ) -> Result<Self, CrossModelRobustnessReportError> {
        let design = authority.design();
        if design.models.len() > CROSS_MODEL_CURRENT_SPECIES_MAX_MODELS {
            return Err(CrossModelRobustnessReportError::TooManyModels {
                actual: design.models.len(),
                maximum: CROSS_MODEL_CURRENT_SPECIES_MAX_MODELS,
            });
        }
        let mut rows = current_rows
            .into_iter()
            .map(|current| current.row)
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| a.conceptual_identity.cmp(&b.conceptual_identity));
        validate_rows(design, &rows)?;
        let pairwise_context = derive_pairwise_context(design)?;
        let support_independence_witness = find_independence_witness(
            design,
            &rows,
            CrossModelOutcomeDisposition::Supports,
        )?;
        let non_support_independence_witness = find_independence_witness(
            design,
            &rows,
            CrossModelOutcomeDisposition::DoesNotSupport,
        )?;
        let contradiction_independence_witness = find_independence_witness(
            design,
            &rows,
            CrossModelOutcomeDisposition::Contradicts,
        )?;
        let status = derive_report_status(
            &rows,
            &support_independence_witness,
            &non_support_independence_witness,
            &contradiction_independence_witness,
        );
        let report = Self {
            report_version: CROSS_MODEL_CURRENT_SPECIES_REPORT_VERSION,
            design: design.clone(),
            design_digest: authority.design_digest(),
            rows,
            pairwise_context,
            support_independence_witness,
            non_support_independence_witness,
            contradiction_independence_witness,
            status,
            rule_authority: current_cross_model_report_rule_v1(),
        };
        report.validate_local()?;
        Ok(report)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<CrossModelCurrentSpeciesReportDigest, CrossModelRobustnessReportError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(REPORT_DOMAIN);
        put_u32(&mut digest, self.report_version);
        digest.update(self.design_digest.as_bytes());
        put_u64(&mut digest, self.rows.len() as u64);
        for row in &self.rows {
            row.put(&mut digest);
        }
        put_u64(&mut digest, self.pairwise_context.len() as u64);
        for pair in &self.pairwise_context {
            pair.put(&mut digest);
        }
        put_identity_vec(&mut digest, &self.support_independence_witness);
        put_identity_vec(&mut digest, &self.non_support_independence_witness);
        put_identity_vec(&mut digest, &self.contradiction_independence_witness);
        digest.update([self.status.tag()]);
        put_authority(&mut digest, &self.rule_authority);
        Ok(CrossModelCurrentSpeciesReportDigest::new(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), CrossModelRobustnessReportError> {
        if self.report_version != CROSS_MODEL_CURRENT_SPECIES_REPORT_VERSION {
            return Err(CrossModelRobustnessReportError::UnsupportedReportVersion(
                self.report_version,
            ));
        }
        if self.design.models.len() > CROSS_MODEL_CURRENT_SPECIES_MAX_MODELS {
            return Err(CrossModelRobustnessReportError::TooManyModels {
                actual: self.design.models.len(),
                maximum: CROSS_MODEL_CURRENT_SPECIES_MAX_MODELS,
            });
        }
        if self.design.canonical_digest()? != self.design_digest {
            return Err(CrossModelRobustnessReportError::DesignDigestMismatch);
        }
        validate_rows(&self.design, &self.rows)?;
        let expected_pairs = derive_pairwise_context(&self.design)?;
        if expected_pairs != self.pairwise_context {
            return Err(CrossModelRobustnessReportError::PairwiseContextMismatch);
        }
        let expected_support = find_independence_witness(
            &self.design,
            &self.rows,
            CrossModelOutcomeDisposition::Supports,
        )?;
        if expected_support != self.support_independence_witness {
            return Err(CrossModelRobustnessReportError::SupportWitnessMismatch);
        }
        let expected_non_support = find_independence_witness(
            &self.design,
            &self.rows,
            CrossModelOutcomeDisposition::DoesNotSupport,
        )?;
        if expected_non_support != self.non_support_independence_witness {
            return Err(CrossModelRobustnessReportError::NonSupportWitnessMismatch);
        }
        let expected_contradiction = find_independence_witness(
            &self.design,
            &self.rows,
            CrossModelOutcomeDisposition::Contradicts,
        )?;
        if expected_contradiction != self.contradiction_independence_witness {
            return Err(CrossModelRobustnessReportError::ContradictionWitnessMismatch);
        }
        let expected_status = derive_report_status(
            &self.rows,
            &expected_support,
            &expected_non_support,
            &expected_contradiction,
        );
        if expected_status != self.status {
            return Err(CrossModelRobustnessReportError::StatusInvariant);
        }
        if self.rule_authority != current_cross_model_report_rule_v1() {
            return Err(CrossModelRobustnessReportError::RuleAuthorityMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "current report authority should gate downstream robustness claims"]
pub struct ValidatedCrossModelCurrentSpeciesReport<'a> {
    report: &'a CrossModelCurrentSpeciesReport,
    report_digest: CrossModelCurrentSpeciesReportDigest,
    design_digest: CurrentCrossModelRobustnessDesignDigest,
}

impl<'a> ValidatedCrossModelCurrentSpeciesReport<'a> {
    pub fn validate_current(
        report: &'a CrossModelCurrentSpeciesReport,
        authority: &CurrentCrossModelRobustnessAuthority<'_>,
        current_rows: impl IntoIterator<Item = CurrentModelOutcomeRow>,
    ) -> Result<Self, CrossModelRobustnessReportError> {
        report.validate_local()?;
        if report.design_digest != authority.design_digest() {
            return Err(CrossModelRobustnessReportError::CurrentDesignMismatch);
        }
        let recomputed = CrossModelCurrentSpeciesReport::evaluate(authority, current_rows)?;
        if recomputed != *report {
            return Err(CrossModelRobustnessReportError::ReplayMismatch);
        }
        Ok(Self {
            report,
            report_digest: report.canonical_digest()?,
            design_digest: authority.design_digest(),
        })
    }

    pub fn report(&self) -> &'a CrossModelCurrentSpeciesReport {
        self.report
    }

    pub fn report_digest(&self) -> CrossModelCurrentSpeciesReportDigest {
        self.report_digest
    }

    pub fn design_digest(&self) -> CurrentCrossModelRobustnessDesignDigest {
        self.design_digest
    }
}

pub fn current_cross_model_report_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(RULE_DOMAIN);
    digest.update(RULE_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("current-cross-model-species-robustness-report-v1")
            .expect("static report rule ID must be valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

fn validate_rows(
    design: &CurrentCrossModelRobustnessDesign,
    rows: &[CrossModelOutcomeRow],
) -> Result<(), CrossModelRobustnessReportError> {
    if rows.len() != design.models.len() {
        return Err(CrossModelRobustnessReportError::IncompleteModelCoverage);
    }
    for (model, row) in design.models.iter().zip(rows) {
        row.validate_against(model)?;
        if matches!(row.result, ModelBoundCurrentStatus::MissingCurrentCapability { .. })
            && design.missing_model_policy == MissingModelPolicy::FailClosed
        {
            return Err(CrossModelRobustnessReportError::MissingModelFailClosed);
        }
    }
    Ok(())
}

fn derive_pairwise_context(
    design: &CurrentCrossModelRobustnessDesign,
) -> Result<Vec<CrossModelPairContext>, CrossModelRobustnessReportError> {
    let mut pairs = Vec::with_capacity(design.semantic_relations.len());
    for relation in &design.semantic_relations {
        let fault = design
            .fault_assessments
            .iter()
            .find(|assessment| assessment.left == relation.left && assessment.right == relation.right)
            .ok_or(CrossModelRobustnessReportError::IncompletePairwiseContext)?;
        pairs.push(CrossModelPairContext {
            left: relation.left.clone(),
            right: relation.right.clone(),
            relation_evidence_digest: relation.evidence_digest,
            semantic_dependency: relation.dependency_class,
            left_profile_digest: fault.left_profile_digest,
            right_profile_digest: fault.right_profile_digest,
            policy_digest: fault.policy_digest,
            fault_disposition: fault.disposition,
            fault_qualification_authority: fault.qualification_authority.clone(),
        });
    }
    Ok(pairs)
}

fn find_independence_witness(
    design: &CurrentCrossModelRobustnessDesign,
    rows: &[CrossModelOutcomeRow],
    target: CrossModelOutcomeDisposition,
) -> Result<Vec<OpenSpeciesConceptIdentity>, CrossModelRobustnessReportError> {
    let threshold = design.minimum_independent_model_coverage as usize;
    let candidates = rows
        .iter()
        .filter(|row| row.disposition() == target)
        .map(|row| row.conceptual_identity.clone())
        .collect::<Vec<_>>();
    if candidates.len() < threshold {
        return Ok(Vec::new());
    }
    let mut chosen = Vec::with_capacity(threshold);
    let mut steps = 0_u64;
    if witness_search(
        design,
        &candidates,
        threshold,
        0,
        &mut chosen,
        &mut steps,
    )? {
        Ok(chosen)
    } else {
        Ok(Vec::new())
    }
}

fn witness_search(
    design: &CurrentCrossModelRobustnessDesign,
    candidates: &[OpenSpeciesConceptIdentity],
    threshold: usize,
    start: usize,
    chosen: &mut Vec<OpenSpeciesConceptIdentity>,
    steps: &mut u64,
) -> Result<bool, CrossModelRobustnessReportError> {
    if chosen.len() == threshold {
        return Ok(true);
    }
    let needed = threshold - chosen.len();
    if candidates.len().saturating_sub(start) < needed {
        return Ok(false);
    }
    for index in start..candidates.len() {
        *steps = steps.saturating_add(1);
        if *steps > CROSS_MODEL_INDEPENDENCE_SEARCH_MAX_STEPS {
            return Err(CrossModelRobustnessReportError::IndependenceSearchLimitExceeded {
                maximum_steps: CROSS_MODEL_INDEPENDENCE_SEARCH_MAX_STEPS,
            });
        }
        let candidate = &candidates[index];
        if chosen.iter().all(|existing| {
            design.pair_is_eligible_for_independent_coverage(existing, candidate)
        }) {
            chosen.push(candidate.clone());
            if witness_search(
                design,
                candidates,
                threshold,
                index + 1,
                chosen,
                steps,
            )? {
                return Ok(true);
            }
            chosen.pop();
        }
    }
    Ok(false)
}

fn derive_report_status(
    rows: &[CrossModelOutcomeRow],
    support_witness: &[OpenSpeciesConceptIdentity],
    non_support_witness: &[OpenSpeciesConceptIdentity],
    contradiction_witness: &[OpenSpeciesConceptIdentity],
) -> CrossModelCurrentSpeciesRobustnessStatus {
    if !rows.is_empty()
        && rows
            .iter()
            .all(|row| row.disposition() == CrossModelOutcomeDisposition::OutsideValidityDomain)
    {
        return CrossModelCurrentSpeciesRobustnessStatus::AllModelsOutsideValidityDomain;
    }

    let support = rows
        .iter()
        .any(|row| row.disposition() == CrossModelOutcomeDisposition::Supports);
    let does_not_support = rows
        .iter()
        .any(|row| row.disposition() == CrossModelOutcomeDisposition::DoesNotSupport);
    let contradiction = rows
        .iter()
        .any(|row| row.disposition() == CrossModelOutcomeDisposition::Contradicts);

    if support && contradiction {
        return CrossModelCurrentSpeciesRobustnessStatus::MixedSupportAndContradiction;
    }

    let resolved_kinds = [support, does_not_support, contradiction]
        .into_iter()
        .filter(|present| *present)
        .count();
    if resolved_kinds > 1 {
        return CrossModelCurrentSpeciesRobustnessStatus::ModelDependentConclusion;
    }

    if support && !support_witness.is_empty() {
        return CrossModelCurrentSpeciesRobustnessStatus::RobustSupportAcrossQualifiedIndependentCoverage;
    }
    if contradiction && !contradiction_witness.is_empty() {
        return CrossModelCurrentSpeciesRobustnessStatus::RobustContradictionAcrossQualifiedIndependentCoverage;
    }
    if does_not_support && !non_support_witness.is_empty() {
        return CrossModelCurrentSpeciesRobustnessStatus::ConcordantNonSupportAcrossQualifiedIndependentCoverage;
    }

    CrossModelCurrentSpeciesRobustnessStatus::InsufficientIndependentModelCoverage
}

fn model_specific_evidence_surface_digest(
    design: &ModelSpecificClassificationDesignRef,
) -> ModelSpecificEvidenceSurfaceDigest {
    let mut digest = Sha256::new();
    digest.update(SURFACE_DOMAIN);
    put_classification_design(&mut digest, design);
    ModelSpecificEvidenceSurfaceDigest::new(digest.finalize().into())
}

fn put_classification_design(digest: &mut Sha256, design: &ModelSpecificClassificationDesignRef) {
    match design {
        ModelSpecificClassificationDesignRef::StrictBiological {
            design_digest,
            reproductive_isolation_design_digest,
        } => {
            digest.update([0]);
            digest.update(design_digest.as_bytes());
            digest.update(reproductive_isolation_design_digest.as_bytes());
        }
        ModelSpecificClassificationDesignRef::GeneralLineage { design_digest } => {
            digest.update([1]);
            digest.update(design_digest.as_bytes());
        }
    }
}

fn put_identity_vec(digest: &mut Sha256, values: &[OpenSpeciesConceptIdentity]) {
    put_u64(digest, values.len() as u64);
    for value in values {
        put_identity(digest, value);
    }
}

fn put_identity(digest: &mut Sha256, identity: &OpenSpeciesConceptIdentity) {
    put_text(digest, identity.family_id.as_str());
    put_u32(digest, identity.family_version);
    digest.update(identity.content_digest.as_bytes());
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

fn validate_authority(
    authority: &AnalysisAuthorityRef,
    field: &'static str,
) -> Result<(), CrossModelRobustnessReportError> {
    if authority.revision == 0 {
        return Err(CrossModelRobustnessReportError::ZeroAuthorityRevision(field));
    }
    Ok(())
}

fn put_text(digest: &mut Sha256, value: &str) {
    put_u64(digest, value.len() as u64);
    digest.update(value.as_bytes());
}

fn put_u32(digest: &mut Sha256, value: u32) {
    digest.update(value.to_le_bytes());
}

fn put_u64(digest: &mut Sha256, value: u64) {
    digest.update(value.to_le_bytes());
}

fn fmt_hex(bytes: &[u8; 32], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum CrossModelRobustnessReportError {
    Design(RobustnessDesignError),
    UnsupportedReportVersion(u32),
    TooManyModels { actual: usize, maximum: usize },
    ZeroAuthorityRevision(&'static str),
    ResultFamilyMismatch,
    ResultDesignMismatch,
    ApplicabilityStatusMismatch,
    RowModelBindingMismatch,
    EvidenceSurfaceDigestMismatch,
    IncompleteModelCoverage,
    MissingModelFailClosed,
    IncompletePairwiseContext,
    PairwiseContextMismatch,
    SupportWitnessMismatch,
    NonSupportWitnessMismatch,
    ContradictionWitnessMismatch,
    IndependenceSearchLimitExceeded { maximum_steps: u64 },
    DesignDigestMismatch,
    CurrentDesignMismatch,
    RuleAuthorityMismatch,
    StatusInvariant,
    ReplayMismatch,
}

impl From<RobustnessDesignError> for CrossModelRobustnessReportError {
    fn from(value: RobustnessDesignError) -> Self {
        Self::Design(value)
    }
}

impl fmt::Display for CrossModelRobustnessReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design(error) => write!(f, "E2A robustness-design error: {error}"),
            Self::UnsupportedReportVersion(version) => {
                write!(f, "unsupported cross-model report version {version}")
            }
            Self::TooManyModels { actual, maximum } => write!(
                f,
                "cross-model report contains {actual} models but V1 supports at most {maximum}"
            ),
            Self::ZeroAuthorityRevision(field) => write!(f, "{field} must be nonzero"),
            Self::ResultFamilyMismatch => {
                write!(f, "model-bound status family does not match the preregistered E2A row")
            }
            Self::ResultDesignMismatch => {
                write!(f, "model-bound status design does not match the preregistered E2A design")
            }
            Self::ApplicabilityStatusMismatch => {
                write!(f, "persisted applicability and family-specific status are inconsistent")
            }
            Self::RowModelBindingMismatch => {
                write!(f, "cross-model outcome row does not bind the exact preregistered E2A model")
            }
            Self::EvidenceSurfaceDigestMismatch => {
                write!(f, "model-specific evidence-surface digest does not match the frozen classification design")
            }
            Self::IncompleteModelCoverage => {
                write!(f, "cross-model report must retain exactly one row per preregistered model")
            }
            Self::MissingModelFailClosed => {
                write!(f, "current model capability is missing under an E2A fail-closed policy")
            }
            Self::IncompletePairwiseContext => {
                write!(f, "E2A design is missing pairwise fault-domain context for a semantic relation")
            }
            Self::PairwiseContextMismatch => {
                write!(f, "persisted pairwise context does not recompute from the frozen E2A design")
            }
            Self::SupportWitnessMismatch => {
                write!(f, "persisted support independence witness is not the canonical E2A-qualified witness")
            }
            Self::NonSupportWitnessMismatch => {
                write!(f, "persisted non-support independence witness is not the canonical E2A-qualified witness")
            }
            Self::ContradictionWitnessMismatch => {
                write!(f, "persisted contradiction independence witness is not the canonical E2A-qualified witness")
            }
            Self::IndependenceSearchLimitExceeded { maximum_steps } => write!(
                f,
                "exact pairwise-independent coverage search exceeded V1 limit of {maximum_steps} steps"
            ),
            Self::DesignDigestMismatch => {
                write!(f, "persisted E2A design snapshot does not match its digest")
            }
            Self::CurrentDesignMismatch => {
                write!(f, "persisted report binds a different E2A design from current authority")
            }
            Self::RuleAuthorityMismatch => {
                write!(f, "cross-model report rule authority does not match the built-in V1 theorem")
            }
            Self::StatusInvariant => {
                write!(f, "persisted cross-model robustness status does not recompute from the complete matrix")
            }
            Self::ReplayMismatch => write!(
                f,
                "persisted cross-model report does not replay from current E2A authority and current model-bound statuses"
            ),
        }
    }
}

impl Error for CrossModelRobustnessReportError {}
