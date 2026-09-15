// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! General-lineage species-concept authority for Symtropy.
//!
//! This crate implements a distinct species-concept family whose conceptual target is
//! a separately evolving metapopulation lineage. Operational criteria are explicit,
//! preregistered evidence channels rather than universal defining properties.

mod design;
mod evidence;
mod historical_design;
mod historical_evidence;
mod model;

pub use design::{
    general_lineage_classification_rule_v1, general_lineage_dependency_grouping_rule_v1,
    GeneralLineageClassificationDesign, GeneralLineageClassificationDesignDigest,
    GeneralLineageClassificationId, GeneralLineageDesignError,
    GeneralLineageEvidenceChannelDeclaration, GeneralLineageEvidenceChannelId,
    GeneralLineageEvidenceChannelKind, GeneralLineageEvidenceChannelRole,
    GeneralLineageEvidenceDependencyGroupId, GeneralLineageMissingEvidencePolicy,
    ValidatedGeneralLineageClassificationDesign, GENERAL_LINEAGE_CLASSIFICATION_DESIGN_VERSION,
};
pub use evidence::{
    GeneralLineageChannelDisposition, GeneralLineageChannelEvidenceInput,
    GeneralLineageChannelEvidenceRecord, GeneralLineageChannelEvidenceSource,
    GeneralLineageEvidenceError, GeneralLineageModelApplicabilityDisposition,
    GeneralLineageModelApplicabilityEvidence, GeneralLineageModelApplicabilityInput,
    GeneralLineageSpeciesEvidence, GeneralLineageSpeciesEvidenceDigest,
    GeneralLineageSpeciesStatus, ValidatedGeneralLineageSpeciesEvidence,
    GENERAL_LINEAGE_SPECIES_EVIDENCE_VERSION,
};
pub use historical_design::{
    general_lineage_historical_evidence_design_rule_v1,
    GeneralLineageHistoricalChannelDeclaration, GeneralLineageHistoricalDesignError,
    GeneralLineageHistoricalEvidenceDesign, GeneralLineageHistoricalEvidenceDesignDigest,
    GeneralLineageHistoricalEvidenceDesignId, GeneralLineageHistoricalMissingPolicy,
    HistoricalChannelProjectionInput, ValidatedGeneralLineageHistoricalEvidenceDesign,
    GENERAL_LINEAGE_HISTORICAL_EVIDENCE_DESIGN_VERSION,
};
pub use historical_evidence::{
    ExternalHistoricalChannelEvidence, GeneralLineageHistoricalEvidenceError,
    GeneralLineageHistoricalEvidenceLedger, GeneralLineageHistoricalEvidenceLedgerDigest,
    HistoricalChannelDisposition, HistoricalChannelEvidenceRecord,
    HistoricalChannelEvidenceSource, HistoricalChannelWindow,
    HistoricalExternalChannelEvidenceInput, HistoricalExternalWindowEvidence,
    HistoricalExternalWindowEvidenceInput, HistoricalWindowBounds,
    NativeLineageHistoryTemporalEvidence, NativeLineageHistoryWindow,
    NativeReproductiveIsolationOpportunityRecord, NativeReproductiveIsolationTemporalEvidence,
    NativeReproductiveIsolationWindow, QualifiedHistoricalEvidence,
    ValidatedGeneralLineageHistoricalEvidence, GENERAL_LINEAGE_HISTORICAL_EVIDENCE_LEDGER_VERSION,
};
pub use model::{
    general_lineage_descriptor_adapter_rule_v1, general_lineage_family_descriptor,
    general_lineage_species_model_content_digest_v1, general_lineage_species_model_rule_v1,
    GeneralLineageModelContentDigest, GeneralLineageModelError,
    GeneralLineageReproductiveModePolicy, GeneralLineageSpeciesModel,
    GeneralLineageSpeciesModelDigest, GeneralLineageSpeciesModelId,
    GeneralLineageValidityDomainDigest, GeneralLineageValidityDomainRef,
    ValidatedGeneralLineageFamilyDescriptor, ValidatedGeneralLineageSpeciesModel,
    GENERAL_LINEAGE_SPECIES_FAMILY_VERSION, GENERAL_LINEAGE_SPECIES_MODEL_VERSION,
};
