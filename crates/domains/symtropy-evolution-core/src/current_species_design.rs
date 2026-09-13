use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    error::validate_text,
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId, BiologicalSpeciesModelDigest,
    EvolutionError, LineageDivergenceHistoryDesignDigest, ReproductiveIsolationDesignDigest,
    SpeciesModelContentDigest, SpeciesModelValidityDomainDigest, ValidatedBiologicalSpeciesModel,
    ValidatedLineageDivergenceHistoryDesign, ValidatedReproductiveIsolationDesign,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const CURRENT_SPECIES_CLASSIFICATION_DESIGN_VERSION: u32 = 1;
const DOMAIN: &[u8] = b"symtropy:evolution:current-species-classification-design:v1\0";
const RULE_DOMAIN: &[u8] = b"symtropy:evolution:strict-biological-species-classification-rule:v1\0";
const RULE_SPEC: &[u8] = b"strict biological species classification v1: target-domain outside has highest applicability precedence; unavailable target-domain applicability is insufficient; current reproductive-isolation contradiction or lineage fusion/persistence loss contradicts distinct current species; missing upstream evidence is insufficient; adequate but unsupported complete-barrier evidence is not-supported-under-model; supported complete isolation plus persistent lineage history or divergence-with-recontact is supported-under-model; no historical speciation event claim";

macro_rules! local_id {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, EvolutionError> {
                let value = value.into();
                validate_text($field, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(<D::Error as serde::de::Error>::custom)
            }
        }
    };
}

local_id!(CurrentSpeciesClassificationId, "CurrentSpeciesClassificationId");

pub fn strict_biological_species_classification_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(RULE_DOMAIN);
    put_u64(&mut digest, RULE_SPEC.len() as u64);
    digest.update(RULE_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("strict-biological-species-classification-v1")
            .expect("static classification rule method ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentSpeciesClassificationDesign {
    design_version: u32,
    pub classification_id: CurrentSpeciesClassificationId,
    pub lineage_a: AnalysisAuthorityRef,
    pub lineage_b: AnalysisAuthorityRef,
    pub lineage_history_design_digest: LineageDivergenceHistoryDesignDigest,
    pub reproductive_isolation_design_digest: ReproductiveIsolationDesignDigest,
    pub species_model_digest: BiologicalSpeciesModelDigest,
    pub species_model_content_digest: SpeciesModelContentDigest,
    pub validity_domain_digest: SpeciesModelValidityDomainDigest,
    pub applicability_protocol: AnalysisAuthorityRef,
    pub classification_rule_authority: AnalysisAuthorityRef,
}

impl CurrentSpeciesClassificationDesign {
    pub fn declare(
        classification_id: CurrentSpeciesClassificationId,
        lineage_history: &ValidatedLineageDivergenceHistoryDesign<'_>,
        reproductive_isolation: &ValidatedReproductiveIsolationDesign<'_>,
        species_model: &ValidatedBiologicalSpeciesModel<'_>,
        applicability_protocol: AnalysisAuthorityRef,
    ) -> Result<Self, CurrentSpeciesClassificationDesignError> {
        let history_design = lineage_history.design();
        let isolation_design = reproductive_isolation.design();
        if history_design.lineage_a != isolation_design.lineage_a
            || history_design.lineage_b != isolation_design.lineage_b
        {
            return Err(CurrentSpeciesClassificationDesignError::LineagePairMismatch);
        }

        let model = species_model.model();
        let design = Self {
            design_version: CURRENT_SPECIES_CLASSIFICATION_DESIGN_VERSION,
            classification_id,
            lineage_a: history_design.lineage_a.clone(),
            lineage_b: history_design.lineage_b.clone(),
            lineage_history_design_digest: lineage_history.design_digest(),
            reproductive_isolation_design_digest: reproductive_isolation.design_digest(),
            species_model_digest: species_model.model_digest(),
            species_model_content_digest: model.model_content_digest,
            validity_domain_digest: model.validity_domain.canonical_digest(),
            applicability_protocol,
            classification_rule_authority: strict_biological_species_classification_rule_v1(),
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<CurrentSpeciesClassificationDesignDigest, CurrentSpeciesClassificationDesignError>
    {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.classification_id.as_str());
        put_authority(&mut digest, &self.lineage_a);
        put_authority(&mut digest, &self.lineage_b);
        digest.update(self.lineage_history_design_digest.as_bytes());
        digest.update(self.reproductive_isolation_design_digest.as_bytes());
        digest.update(self.species_model_digest.as_bytes());
        digest.update(self.species_model_content_digest.as_bytes());
        digest.update(self.validity_domain_digest.as_bytes());
        put_authority(&mut digest, &self.applicability_protocol);
        put_authority(&mut digest, &self.classification_rule_authority);
        Ok(CurrentSpeciesClassificationDesignDigest(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), CurrentSpeciesClassificationDesignError> {
        if self.design_version != CURRENT_SPECIES_CLASSIFICATION_DESIGN_VERSION {
            return Err(CurrentSpeciesClassificationDesignError::UnsupportedVersion(
                self.design_version,
            ));
        }
        if self.lineage_a == self.lineage_b {
            return Err(CurrentSpeciesClassificationDesignError::LineagePairMismatch);
        }
        if self.classification_rule_authority
            != strict_biological_species_classification_rule_v1()
        {
            return Err(CurrentSpeciesClassificationDesignError::ClassificationRuleMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurrentSpeciesClassificationDesignDigest([u8; 32]);

impl CurrentSpeciesClassificationDesignDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for CurrentSpeciesClassificationDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CurrentSpeciesClassificationDesignDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for CurrentSpeciesClassificationDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated classification design should gate current species-status evaluation"]
pub struct ValidatedCurrentSpeciesClassificationDesign<'a> {
    design: &'a CurrentSpeciesClassificationDesign,
    design_digest: CurrentSpeciesClassificationDesignDigest,
}

impl<'a> ValidatedCurrentSpeciesClassificationDesign<'a> {
    pub fn validate_current(
        design: &'a CurrentSpeciesClassificationDesign,
        lineage_history: &ValidatedLineageDivergenceHistoryDesign<'_>,
        reproductive_isolation: &ValidatedReproductiveIsolationDesign<'_>,
        species_model: &ValidatedBiologicalSpeciesModel<'_>,
        applicability_protocol: AnalysisAuthorityRef,
    ) -> Result<Self, CurrentSpeciesClassificationDesignError> {
        design.validate_local()?;
        let recomputed = CurrentSpeciesClassificationDesign::declare(
            design.classification_id.clone(),
            lineage_history,
            reproductive_isolation,
            species_model,
            applicability_protocol,
        )?;
        if recomputed != *design {
            return Err(CurrentSpeciesClassificationDesignError::ReplayMismatch);
        }
        Ok(Self {
            design,
            design_digest: design.canonical_digest()?,
        })
    }

    pub fn design(&self) -> &'a CurrentSpeciesClassificationDesign {
        self.design
    }

    pub fn design_digest(&self) -> CurrentSpeciesClassificationDesignDigest {
        self.design_digest
    }
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum CurrentSpeciesClassificationDesignError {
    UnsupportedVersion(u32),
    LineagePairMismatch,
    ClassificationRuleMismatch,
    ReplayMismatch,
}

impl fmt::Display for CurrentSpeciesClassificationDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => write!(
                f,
                "unsupported current species-classification design version {version}"
            ),
            Self::LineagePairMismatch => write!(
                f,
                "SEL-10A and SEL-09B designs do not bind the same ordered lineage pair"
            ),
            Self::ClassificationRuleMismatch => write!(
                f,
                "persisted design does not bind the built-in strict biological-species V1 classifier"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted current-species classification design does not replay against current upstream designs/model"
            ),
        }
    }
}

impl Error for CurrentSpeciesClassificationDesignError {}
