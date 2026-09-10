use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AncestryCopyId, ChromosomeId, ChromosomeMap, ChromosomeMapDigest, EvolutionError,
    HereditarySchema, HereditarySchemaDigest, PhasedHereditaryState, PhasedHereditaryStateDigest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, error::Error, fmt};

pub const PHASED_ANCESTRY_STATE_VERSION: u32 = 1;
const PHASED_ANCESTRY_STATE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:phased-ancestry-state:v1\0";

/// Persistent ancestry identities associated with one complete-haplotype content
/// equivalence class in the exact bound C2 chromosome state.
///
/// `representative_haplotype_slot` is a local content lookup only. Individual
/// `copy_ids` are deliberately not assigned to row slots within an equal-content
/// class because the genetic state contains no evidence for such an assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaplotypeAncestryClass {
    pub representative_haplotype_slot: u8,
    pub copy_ids: Vec<AncestryCopyId>,
}

impl HaplotypeAncestryClass {
    pub fn new(
        representative_haplotype_slot: u8,
        mut copy_ids: Vec<AncestryCopyId>,
    ) -> Result<Self, AncestryAuthorityError> {
        if copy_ids.is_empty() {
            return Err(AncestryAuthorityError::EmptyAncestryClass);
        }
        copy_ids.sort();
        if copy_ids.windows(2).any(|window| window[0] == window[1]) {
            return Err(AncestryAuthorityError::DuplicateAncestryCopyIdentity(
                copy_ids[0].clone(),
            ));
        }
        Ok(Self {
            representative_haplotype_slot,
            copy_ids,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChromosomeAncestryState {
    pub chromosome_id: ChromosomeId,
    pub classes: Vec<HaplotypeAncestryClass>,
}

impl ChromosomeAncestryState {
    pub fn new(
        chromosome_id: ChromosomeId,
        mut classes: Vec<HaplotypeAncestryClass>,
    ) -> Result<Self, AncestryAuthorityError> {
        if classes.is_empty() {
            return Err(AncestryAuthorityError::EmptyChromosomeAncestry {
                chromosome: chromosome_id,
            });
        }
        for class in &mut classes {
            if class.copy_ids.is_empty() {
                return Err(AncestryAuthorityError::EmptyAncestryClass);
            }
            class.copy_ids.sort();
            if class.copy_ids.windows(2).any(|window| window[0] == window[1]) {
                return Err(AncestryAuthorityError::DuplicateAncestryCopyIdentity(
                    class.copy_ids[0].clone(),
                ));
            }
        }
        classes.sort_by_key(|class| class.representative_haplotype_slot);
        if classes.windows(2).any(|window| {
            window[0].representative_haplotype_slot
                == window[1].representative_haplotype_slot
        }) {
            return Err(AncestryAuthorityError::DuplicateAncestryClassRepresentative {
                chromosome: chromosome_id,
            });
        }
        Ok(Self {
            chromosome_id,
            classes,
        })
    }
}

/// Optional persistent ancestry-copy identity sidecar for one exact phased genome.
///
/// Genetic content remains owned by `PhasedHereditaryState`. This object stores
/// only persistent ancestry IDs grouped by the genetic-content equivalence classes
/// derivable from that exact state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhasedAncestryState {
    pub state_version: u32,
    pub schema_digest: HereditarySchemaDigest,
    pub chromosome_map_digest: ChromosomeMapDigest,
    pub phased_state_digest: PhasedHereditaryStateDigest,
    pub chromosomes: BTreeMap<ChromosomeId, ChromosomeAncestryState>,
}

impl PhasedAncestryState {
    pub fn new(
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        phased_state: &PhasedHereditaryState,
        chromosomes: impl IntoIterator<Item = ChromosomeAncestryState>,
    ) -> Result<Self, AncestryAuthorityError> {
        phased_state.validate(schema, chromosome_map)?;
        let mut by_id = BTreeMap::new();
        for mut chromosome in chromosomes {
            for class in &mut chromosome.classes {
                class.copy_ids.sort();
            }
            chromosome
                .classes
                .sort_by_key(|class| class.representative_haplotype_slot);
            let id = chromosome.chromosome_id.clone();
            if by_id.insert(id.clone(), chromosome).is_some() {
                return Err(AncestryAuthorityError::DuplicateChromosomeIdentity(id));
            }
        }
        let value = Self {
            state_version: PHASED_ANCESTRY_STATE_VERSION,
            schema_digest: schema.canonical_digest()?,
            chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
            phased_state_digest: phased_state.canonical_digest(schema, chromosome_map)?,
            chromosomes: by_id,
        };
        value.validate_current(schema, chromosome_map, phased_state)?;
        Ok(value)
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        phased_state: &PhasedHereditaryState,
    ) -> Result<(), AncestryAuthorityError> {
        phased_state.validate(schema, chromosome_map)?;
        if self.state_version != PHASED_ANCESTRY_STATE_VERSION {
            return Err(AncestryAuthorityError::UnsupportedVersion(self.state_version));
        }
        if self.schema_digest != schema.canonical_digest()?
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
            || self.phased_state_digest != phased_state.canonical_digest(schema, chromosome_map)?
        {
            return Err(AncestryAuthorityError::CurrentAuthorityMismatch);
        }
        if self.chromosomes.len() != chromosome_map.chromosomes.len() {
            return Err(AncestryAuthorityError::ChromosomeSetMismatch);
        }

        let mut global_ids = BTreeSet::new();
        for (chromosome_id, definition) in &chromosome_map.chromosomes {
            let ancestry = self
                .chromosomes
                .get(chromosome_id)
                .ok_or(AncestryAuthorityError::ChromosomeSetMismatch)?;
            if ancestry.chromosome_id != *chromosome_id {
                return Err(AncestryAuthorityError::ChromosomeKeyMismatch {
                    key: chromosome_id.clone(),
                    value: ancestry.chromosome_id.clone(),
                });
            }
            let genetic = phased_state
                .chromosomes
                .get(chromosome_id)
                .ok_or(AncestryAuthorityError::ChromosomeSetMismatch)?;
            if genetic.haplotypes.len() != usize::from(schema.ploidy)
                || definition.loci.is_empty()
            {
                return Err(AncestryAuthorityError::ContentClassMismatch {
                    chromosome: chromosome_id.clone(),
                });
            }

            let expected = expected_content_classes(&genetic.haplotypes)?;
            if ancestry.classes.len() != expected.len() {
                return Err(AncestryAuthorityError::ContentClassMismatch {
                    chromosome: chromosome_id.clone(),
                });
            }
            if ancestry.classes.windows(2).any(|window| {
                window[0].representative_haplotype_slot
                    >= window[1].representative_haplotype_slot
            }) {
                return Err(AncestryAuthorityError::NonCanonicalClassOrder {
                    chromosome: chromosome_id.clone(),
                });
            }

            for (class, (expected_slot, expected_multiplicity)) in
                ancestry.classes.iter().zip(expected)
            {
                if usize::from(class.representative_haplotype_slot) != expected_slot {
                    return Err(AncestryAuthorityError::ContentClassMismatch {
                        chromosome: chromosome_id.clone(),
                    });
                }
                if class.copy_ids.len() != expected_multiplicity {
                    return Err(AncestryAuthorityError::ClassMultiplicityMismatch {
                        chromosome: chromosome_id.clone(),
                        representative_haplotype_slot: class.representative_haplotype_slot,
                        expected: expected_multiplicity,
                        observed: class.copy_ids.len(),
                    });
                }
                if class.copy_ids.is_empty() {
                    return Err(AncestryAuthorityError::EmptyAncestryClass);
                }
                if class.copy_ids.windows(2).any(|window| window[0] >= window[1]) {
                    return Err(AncestryAuthorityError::NonCanonicalCopyIdOrder {
                        chromosome: chromosome_id.clone(),
                        representative_haplotype_slot: class.representative_haplotype_slot,
                    });
                }
                for copy_id in &class.copy_ids {
                    if !global_ids.insert(copy_id.clone()) {
                        return Err(AncestryAuthorityError::DuplicateAncestryCopyIdentity(
                            copy_id.clone(),
                        ));
                    }
                }
            }
        }

        if self.chromosomes.keys().any(|id| !chromosome_map.chromosomes.contains_key(id)) {
            return Err(AncestryAuthorityError::ChromosomeSetMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        phased_state: &PhasedHereditaryState,
    ) -> Result<PhasedAncestryStateDigest, AncestryAuthorityError> {
        self.validate_current(schema, chromosome_map, phased_state)?;
        let mut digest = Sha256::new();
        digest.update(PHASED_ANCESTRY_STATE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.state_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.phased_state_digest.as_bytes());
        put_u64(&mut digest, self.chromosomes.len() as u64);
        for (chromosome_id, chromosome) in &self.chromosomes {
            put_text(&mut digest, chromosome_id.as_str());
            put_text(&mut digest, chromosome.chromosome_id.as_str());
            put_u64(&mut digest, chromosome.classes.len() as u64);
            for class in &chromosome.classes {
                digest.update([class.representative_haplotype_slot]);
                put_u64(&mut digest, class.copy_ids.len() as u64);
                for copy_id in &class.copy_ids {
                    put_text(&mut digest, copy_id.as_str());
                }
            }
        }
        Ok(PhasedAncestryStateDigest(digest.finalize().into()))
    }
}

fn expected_content_classes(
    haplotypes: &[crate::ChromosomeHaplotype],
) -> Result<Vec<(usize, usize)>, AncestryAuthorityError> {
    if haplotypes.is_empty() {
        return Err(AncestryAuthorityError::EmptyAncestryClass);
    }
    let mut classes = Vec::new();
    let mut start = 0_usize;
    while start < haplotypes.len() {
        let mut end = start + 1;
        while end < haplotypes.len() && haplotypes[end] == haplotypes[start] {
            end += 1;
        }
        classes.push((start, end - start));
        start = end;
    }
    Ok(classes)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhasedAncestryStateDigest([u8; 32]);

impl PhasedAncestryStateDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for PhasedAncestryStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhasedAncestryStateDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for PhasedAncestryStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AncestryAuthorityError {
    Evolution(EvolutionError),
    UnsupportedVersion(u32),
    CurrentAuthorityMismatch,
    ChromosomeSetMismatch,
    DuplicateChromosomeIdentity(ChromosomeId),
    ChromosomeKeyMismatch { key: ChromosomeId, value: ChromosomeId },
    EmptyChromosomeAncestry { chromosome: ChromosomeId },
    EmptyAncestryClass,
    DuplicateAncestryClassRepresentative { chromosome: ChromosomeId },
    NonCanonicalClassOrder { chromosome: ChromosomeId },
    ContentClassMismatch { chromosome: ChromosomeId },
    ClassMultiplicityMismatch {
        chromosome: ChromosomeId,
        representative_haplotype_slot: u8,
        expected: usize,
        observed: usize,
    },
    NonCanonicalCopyIdOrder {
        chromosome: ChromosomeId,
        representative_haplotype_slot: u8,
    },
    DuplicateAncestryCopyIdentity(AncestryCopyId),
}

impl From<EvolutionError> for AncestryAuthorityError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl fmt::Display for AncestryAuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported phased ancestry-state version {version}"),
            Self::CurrentAuthorityMismatch => write!(f, "phased ancestry sidecar does not match exact current genetic authority"),
            Self::ChromosomeSetMismatch => write!(f, "phased ancestry chromosome set does not match exact chromosome map"),
            Self::DuplicateChromosomeIdentity(chromosome) => write!(f, "duplicate ancestry chromosome {}", chromosome.as_str()),
            Self::ChromosomeKeyMismatch { key, value } => write!(f, "ancestry chromosome key {} does not match embedded chromosome {}", key.as_str(), value.as_str()),
            Self::EmptyChromosomeAncestry { chromosome } => write!(f, "chromosome {} has no ancestry classes", chromosome.as_str()),
            Self::EmptyAncestryClass => write!(f, "haplotype ancestry class must contain at least one persistent copy ID"),
            Self::DuplicateAncestryClassRepresentative { chromosome } => write!(f, "chromosome {} repeats a haplotype ancestry-class representative", chromosome.as_str()),
            Self::NonCanonicalClassOrder { chromosome } => write!(f, "chromosome {} ancestry classes are not in canonical representative-slot order", chromosome.as_str()),
            Self::ContentClassMismatch { chromosome } => write!(f, "chromosome {} ancestry classes do not exactly match current haplotype-content equivalence classes", chromosome.as_str()),
            Self::ClassMultiplicityMismatch { chromosome, representative_haplotype_slot, expected, observed } => write!(f, "chromosome {} ancestry class at representative slot {} expected {} copy IDs, observed {}", chromosome.as_str(), representative_haplotype_slot, expected, observed),
            Self::NonCanonicalCopyIdOrder { chromosome, representative_haplotype_slot } => write!(f, "chromosome {} ancestry copy IDs at representative slot {} are not strictly canonical", chromosome.as_str(), representative_haplotype_slot),
            Self::DuplicateAncestryCopyIdentity(copy_id) => write!(f, "persistent ancestry copy {} appears more than once", copy_id.as_str()),
        }
    }
}

impl Error for AncestryAuthorityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Evolution(error) => Some(error),
            _ => None,
        }
    }
}
