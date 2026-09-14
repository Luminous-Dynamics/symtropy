// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Non-Serde current-authority layer for SEL-10E2A.
//!
//! Persisted E2A rows are representations. These wrappers regain current authority
//! only by exact recomputation from live family descriptors/designs and live semantic
//! relation evidence. External fault-domain and evidence-universe authorities remain
//! explicit trust edges supplied at replay time.

use crate::{
    CurrentCrossModelRobustnessDesign, CurrentCrossModelRobustnessDesignDigest,
    FaultDomainIndependencePolicy, MissingModelPolicy, ModelFaultDomainProfile,
    PairwiseFaultDomainAssessment, RobustnessDesignError, SemanticRelationRecord,
    SharedEvidenceSubject, SpeciesModelDesignRecord,
};
use std::{error::Error, fmt};
use symtropy_evolution_core::ValidatedCurrentSpeciesClassificationDesign;
use symtropy_species_concept::ValidatedSpeciesConceptFamilyDescriptor;
use symtropy_species_concept_general_lineage::{
    ValidatedGeneralLineageClassificationDesign, ValidatedGeneralLineageFamilyDescriptor,
};
use symtropy_species_concept_relations::ValidatedSpeciesConceptRelationEvidence;

#[derive(Debug)]
#[must_use = "a persisted model row is not current authority without fresh family replay"]
pub struct CurrentSpeciesModelDesignRecord<'a> {
    record: &'a SpeciesModelDesignRecord,
}

impl<'a> CurrentSpeciesModelDesignRecord<'a> {
    pub fn validate_strict_biological(
        record: &'a SpeciesModelDesignRecord,
        descriptor: &ValidatedSpeciesConceptFamilyDescriptor<'_>,
        classification_design: &ValidatedCurrentSpeciesClassificationDesign<'_>,
        fault_profile: ModelFaultDomainProfile,
    ) -> Result<Self, CurrentRobustnessAuthorityError> {
        let recomputed = SpeciesModelDesignRecord::strict_biological(
            descriptor,
            classification_design,
            fault_profile,
        )?;
        if recomputed != *record {
            return Err(CurrentRobustnessAuthorityError::ModelRecordReplayMismatch);
        }
        Ok(Self { record })
    }

    pub fn validate_general_lineage(
        record: &'a SpeciesModelDesignRecord,
        descriptor: &ValidatedGeneralLineageFamilyDescriptor<'_>,
        classification_design: &ValidatedGeneralLineageClassificationDesign<'_>,
        fault_profile: ModelFaultDomainProfile,
    ) -> Result<Self, CurrentRobustnessAuthorityError> {
        let recomputed = SpeciesModelDesignRecord::general_lineage(
            descriptor,
            classification_design,
            fault_profile,
        )?;
        if recomputed != *record {
            return Err(CurrentRobustnessAuthorityError::ModelRecordReplayMismatch);
        }
        Ok(Self { record })
    }

    pub fn record(&self) -> &'a SpeciesModelDesignRecord {
        self.record
    }
}

#[derive(Debug)]
#[must_use = "a persisted semantic-relation row is not current authority without fresh relation replay"]
pub struct CurrentSemanticRelationRecord<'a> {
    record: &'a SemanticRelationRecord,
}

impl<'a> CurrentSemanticRelationRecord<'a> {
    pub fn validate_current(
        record: &'a SemanticRelationRecord,
        relation: &ValidatedSpeciesConceptRelationEvidence<'_>,
    ) -> Result<Self, CurrentRobustnessAuthorityError> {
        let recomputed = SemanticRelationRecord::from_current(relation)?;
        if recomputed != *record {
            return Err(CurrentRobustnessAuthorityError::RelationRecordReplayMismatch);
        }
        Ok(Self { record })
    }

    pub fn record(&self) -> &'a SemanticRelationRecord {
        self.record
    }
}

#[derive(Debug)]
#[must_use = "current E2A authority should gate SEL-10E2B execution"]
pub struct CurrentCrossModelRobustnessAuthority<'a> {
    design: &'a CurrentCrossModelRobustnessDesign,
    design_digest: CurrentCrossModelRobustnessDesignDigest,
}

impl<'a> CurrentCrossModelRobustnessAuthority<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current<'m, 'r>(
        design: &'a CurrentCrossModelRobustnessDesign,
        subject: SharedEvidenceSubject,
        current_models: impl IntoIterator<Item = CurrentSpeciesModelDesignRecord<'m>>,
        current_relations: impl IntoIterator<Item = CurrentSemanticRelationRecord<'r>>,
        fault_domain_policy: FaultDomainIndependencePolicy,
        fault_assessments: impl IntoIterator<Item = PairwiseFaultDomainAssessment>,
        minimum_conceptual_family_count: u32,
        minimum_independent_model_coverage: u32,
        missing_model_policy: MissingModelPolicy,
    ) -> Result<Self, CurrentRobustnessAuthorityError> {
        let models = current_models
            .into_iter()
            .map(|validated| validated.record().clone())
            .collect::<Vec<_>>();
        let relations = current_relations
            .into_iter()
            .map(|validated| validated.record().clone())
            .collect::<Vec<_>>();
        let recomputed = CurrentCrossModelRobustnessDesign::declare(
            design.design_id.clone(),
            subject,
            models,
            relations,
            fault_domain_policy,
            fault_assessments,
            minimum_conceptual_family_count,
            minimum_independent_model_coverage,
            missing_model_policy,
        )?;
        if recomputed != *design {
            return Err(CurrentRobustnessAuthorityError::DesignReplayMismatch);
        }
        Ok(Self {
            design,
            design_digest: design.canonical_digest()?,
        })
    }

    pub fn design(&self) -> &'a CurrentCrossModelRobustnessDesign {
        self.design
    }

    pub fn design_digest(&self) -> CurrentCrossModelRobustnessDesignDigest {
        self.design_digest
    }
}

#[derive(Debug)]
pub enum CurrentRobustnessAuthorityError {
    ModelRecordReplayMismatch,
    RelationRecordReplayMismatch,
    DesignReplayMismatch,
    Design(RobustnessDesignError),
}

impl From<RobustnessDesignError> for CurrentRobustnessAuthorityError {
    fn from(value: RobustnessDesignError) -> Self {
        Self::Design(value)
    }
}

impl fmt::Display for CurrentRobustnessAuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelRecordReplayMismatch => write!(
                f,
                "persisted E2A model row does not replay from the supplied current family descriptor/design"
            ),
            Self::RelationRecordReplayMismatch => write!(
                f,
                "persisted E2A semantic-relation row does not replay from current relation evidence"
            ),
            Self::DesignReplayMismatch => write!(
                f,
                "persisted E2A design does not replay from the current validated model/relation rows and current trust edges"
            ),
            Self::Design(error) => write!(f, "E2A design error: {error}"),
        }
    }
}

impl Error for CurrentRobustnessAuthorityError {}
